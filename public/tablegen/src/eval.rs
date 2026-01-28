use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

use prjcombine_entity::{
    EntityBundleIndex, EntityBundleMap, EntityId, EntityPartVec, EntitySet, EntityVec,
};
use prjcombine_interconnect::db::{
    Bel, BelAttribute, BelAttributeEnum, BelAttributeType, BelClass, BelClassAttribute,
    BelClassBidir, BelClassId, BelClassInput, BelClassOutput, BelClassPad, BelInfo, BelInput,
    BelKind, BelPinIndexing, BelSlot, BelSlotId, Bidi, BitRectInfo, CellSlotId, ConnectorClass,
    ConnectorClassId, ConnectorSlot, ConnectorSlotId, ConnectorWire, EnumClass, IntDb, Mux,
    PermaBuf, PolTileWireCoord, ProgBuf, ProgInv, SwitchBox, SwitchBoxItem, Table, TableId,
    TileClass, TileClassId, TileSlotId, TileWireCoord, WireKind,
};
use prjcombine_types::bsdata::{BitRectGeometry, PolTileBit, RectBitId, RectFrameId, TileBit};
use proc_macro::{Ident, Span, TokenStream};

use crate::{
    ast,
    db::AnnotatedDb,
    parse::{error_at, parse_str},
};

type Result<T> = core::result::Result<T, TokenStream>;

struct Context {
    db: AnnotatedDb,
    for_vars: HashMap<String, usize>,
    bitrect_geoms: HashMap<String, BitRectGeometry>,
}

#[derive(Copy, Clone)]
enum IfContext {
    Top,
    TileClass(TileClassId),
    ConnectorClass(ConnectorClassId),
    Bel(TileClassId, BelSlotId),
}

#[derive(Clone)]
enum WildCard {
    Ident(Ident),
    Prefix(Ident),
}

impl Context {
    fn eval_expr(&self, span: Span, expr: &str) -> Result<usize> {
        let expr = expr.trim_ascii();
        if let Some((a, b)) = expr.rsplit_once('+') {
            let a = self.eval_expr(span, a)?;
            let b = self.eval_expr(span, b)?;
            Ok(a + b)
        } else if let Ok(val) = expr.parse() {
            Ok(val)
        } else {
            let Some(&val) = self.for_vars.get(expr) else {
                error_at(span, &format!("unknown variable {expr}"))?
            };
            Ok(val)
        }
    }

    fn eval_templ_wildcard(&self, id: &ast::TemplateId) -> Result<WildCard> {
        Ok(match id {
            ast::TemplateId::Raw(ident) => WildCard::Ident(ident.clone()),
            ast::TemplateId::String(lit) => {
                let mut res = String::new();
                let s = parse_str(lit)?;
                let mut suffix = &s[..];
                while let Some((prefix, rest)) = suffix.split_once('{') {
                    res.push_str(prefix);
                    let Some((expr, rest)) = rest.split_once('}') else {
                        error_at(lit.span(), "unterminated interpolation")?
                    };
                    let val = self.eval_expr(lit.span(), expr)?;
                    write!(res, "{val}").unwrap();
                    suffix = rest;
                }
                res.push_str(suffix);
                if let Some(prefix) = res.strip_suffix('*') {
                    WildCard::Prefix(Ident::new(prefix, lit.span()))
                } else {
                    WildCard::Ident(Ident::new(&res, lit.span()))
                }
            }
        })
    }

    fn eval_templ_id(&self, id: &ast::TemplateId) -> Result<Ident> {
        match self.eval_templ_wildcard(id)? {
            WildCard::Ident(ident) => Ok(ident),
            WildCard::Prefix(_) => error_at(id.span(), "wildcard not accepted here")?,
        }
    }

    fn eval_index(&self, index: &ast::Index) -> Result<usize> {
        match index {
            ast::Index::Ident(ident, offset) => match self.for_vars.get(&ident.to_string()) {
                Some(&n) => Ok(n.strict_add_signed(*offset)),
                None => error_at(ident.span(), "undefined variable")?,
            },
            ast::Index::Literal(n) => Ok(*n),
        }
    }

    fn eval_array_ref_wide<I: EntityId>(
        &self,
        id: &ast::ArrayIdRef,
        map: &EntityBundleMap<I, Ident>,
    ) -> Result<EntityBundleIndex<I>> {
        match id {
            ast::ArrayIdRef::Plain(id) => {
                let id = self.eval_templ_id(id)?;
                let Some((item, _)) = map.get(&id.to_string()) else {
                    error_at(id.span(), "undefined object")?
                };
                Ok(item)
            }
            ast::ArrayIdRef::Indexed(id, index) => {
                let id = self.eval_templ_id(id)?;
                let index = self.eval_index(index)?;
                let Some((item, _)) = map.get(&id.to_string()) else {
                    error_at(id.span(), "undefined object")?
                };
                match item {
                    EntityBundleIndex::Single(_) => error_at(id.span(), "object is not an array")?,
                    EntityBundleIndex::Array(range) => {
                        if index > range.len() {
                            error_at(id.span(), "index out of bounds")?
                        }
                        Ok(EntityBundleIndex::Single(range.index(index)))
                    }
                }
            }
        }
    }

    fn eval_array_ref<I: EntityId>(
        &self,
        id: &ast::ArrayIdRef,
        map: &EntityBundleMap<I, Ident>,
    ) -> Result<I> {
        match self.eval_array_ref_wide(id, map)? {
            EntityBundleIndex::Single(id) => Ok(id),
            EntityBundleIndex::Array(_) => error_at(id.span(), "object is an array")?,
        }
    }

    fn eval_wire_ref(&self, tcls: TileClassId, wref: &ast::WireRef) -> Result<TileWireCoord> {
        Ok(match wref {
            ast::WireRef::Simple(wref) => {
                if self.db.db.tile_classes[tcls].cells.len() != 1 {
                    error_at(
                        wref.span(),
                        "non-qualified wire references are only valid in single-cell tiles",
                    )?
                }
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: self.eval_array_ref(wref, &self.db.wire_id)?,
                }
            }
            ast::WireRef::Qualified(cref, wref) => TileWireCoord {
                cell: self.eval_array_ref(cref, &self.db.tcls_cell_id[tcls])?,
                wire: self.eval_array_ref(wref, &self.db.wire_id)?,
            },
        })
    }

    fn eval_pol_wire_ref(
        &self,
        tcls: TileClassId,
        wref: &ast::PolWireRef,
    ) -> Result<PolTileWireCoord> {
        Ok(match wref {
            ast::PolWireRef::Pos(wire_ref) => self.eval_wire_ref(tcls, wire_ref)?.pos(),
            ast::PolWireRef::Neg(wire_ref) => self.eval_wire_ref(tcls, wire_ref)?.neg(),
        })
    }

    fn eval_pol_tile_bit(&self, tcls: TileClassId, tbit: &ast::TileBit) -> Result<PolTileBit> {
        let name = self.eval_templ_id(&tbit.name)?;
        let mut index = vec![];
        for idx in &tbit.index {
            index.push(self.eval_index(idx)?);
        }
        let Some((rect, _)) = self.db.tcls_bitrect_id[tcls].get(&name.to_string()) else {
            error_at(name.span(), "undefined bitrect")?
        };
        let rect = match rect {
            EntityBundleIndex::Single(r) => r,
            EntityBundleIndex::Array(range) => {
                if index.is_empty() {
                    error_at(name.span(), "missing bitrect index")?
                }
                let idx = index.remove(0);
                if idx >= range.len() {
                    error_at(name.span(), "bitrect out of bounds")?
                }
                range.index(idx)
            }
        };
        let geom = self.db.db.tile_classes[tcls].bitrects[rect].geometry;
        let (frame, bit) = match *index.as_slice() {
            [] => error_at(name.span(), "missing bit coordinates")?,
            [bit] => {
                if geom.frames != 1 {
                    error_at(
                        name.span(),
                        "single index can only be used for single-frame bitrects",
                    )?;
                }
                if bit >= geom.bits {
                    error_at(name.span(), "bit out of range")?;
                }
                (RectFrameId::from_idx(0), RectBitId::from_idx(bit))
            }
            [frame, bit] => {
                if frame >= geom.frames {
                    error_at(name.span(), "frame out of range")?;
                }
                if bit >= geom.bits {
                    error_at(name.span(), "bit out of range")?;
                }
                (RectFrameId::from_idx(frame), RectBitId::from_idx(bit))
            }
            _ => error_at(name.span(), "too many coordinates")?,
        };
        Ok(PolTileBit {
            bit: TileBit { rect, frame, bit },
            inv: tbit.inv,
        })
    }

    fn eval_tile_bit(&self, tcls: TileClassId, tbit: &ast::TileBit) -> Result<TileBit> {
        let bit = self.eval_pol_tile_bit(tcls, tbit)?;
        if bit.inv {
            error_at(tbit.name.span(), "inversion not accepted here")?;
        }
        Ok(bit.bit)
    }

    fn eval_for<T>(
        &mut self,
        loop_: &ast::ForLoop<T>,
        mut eval_item: impl FnMut(&mut Context, &T) -> Result<()>,
    ) -> Result<()> {
        let var = &loop_.var;
        let var_name = var.to_string();
        if self.for_vars.contains_key(&var_name) {
            error_at(var.span(), "loop variable {var} redefined")?
        }
        match loop_.iterator {
            ast::ForIterator::Range(ref range) => {
                for i in range.clone() {
                    self.for_vars.insert(var_name.clone(), i);
                    for item in &loop_.items {
                        eval_item(self, item)?;
                    }
                }
            }
            ast::ForIterator::RangeInclusive(ref range) => {
                for i in range.clone() {
                    self.for_vars.insert(var_name.clone(), i);
                    for item in &loop_.items {
                        eval_item(self, item)?;
                    }
                }
            }
        }
        self.for_vars.remove(&var_name);
        Ok(())
    }

    fn eval_cond(&mut self, ictx: IfContext, span: Span, cond: &ast::IfCond) -> Result<bool> {
        match cond {
            ast::IfCond::Variant(idents) => {
                let Some(ref our_name) = self.db.name else {
                    error_at(span, "no variants defined")?
                };
                for id in idents {
                    if id.to_string() == our_name.to_string() {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ast::IfCond::TileClass(ids) => {
                let tcls = match ictx {
                    IfContext::TileClass(tcls) => tcls,
                    IfContext::Bel(tcls, _) => tcls,
                    _ => error_at(span, "not in tile class")?,
                };
                let name = self.db.db.tile_classes.key(tcls);
                for id in ids {
                    let ok = match self.eval_templ_wildcard(id)? {
                        WildCard::Ident(ident) => name == &ident.to_string(),
                        WildCard::Prefix(ident) => name.starts_with(&ident.to_string()),
                    };
                    if ok {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ast::IfCond::ConnectorClass(ids) => {
                let ccls = match ictx {
                    IfContext::ConnectorClass(ccls) => ccls,
                    _ => error_at(span, "not in connector class")?,
                };
                let name = self.db.db.conn_classes.key(ccls);
                for id in ids {
                    let ok = match self.eval_templ_wildcard(id)? {
                        WildCard::Ident(ident) => name == &ident.to_string(),
                        WildCard::Prefix(ident) => name.starts_with(&ident.to_string()),
                    };
                    if ok {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ast::IfCond::BelSlot(ids) => {
                let our_bslot = match ictx {
                    IfContext::Bel(_, bslot) => bslot,
                    _ => error_at(span, "not in bel")?,
                };
                for id in ids {
                    let bslot = self.eval_array_ref(id, &self.db.bslot_id)?;
                    if bslot == our_bslot {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    fn eval_if<T>(
        &mut self,
        ictx: IfContext,
        if_: &ast::If<T>,
        mut eval_item: impl FnMut(&mut Context, &T) -> Result<()>,
    ) -> Result<()> {
        for branch in &if_.branches {
            if self.eval_cond(ictx, if_.span, &branch.0)? {
                for item in &branch.1 {
                    eval_item(self, item)?;
                }
                return Ok(());
            }
        }
        for item in &if_.else_items {
            eval_item(self, item)?;
        }
        Ok(())
    }

    fn eval_pin_indexing(
        &self,
        name: &ast::PinArrayIdDef,
    ) -> Result<(Ident, Option<usize>, BelPinIndexing)> {
        Ok(match *name {
            ast::PinArrayIdDef::Plain(ref name) => {
                let name = self.eval_templ_id(name)?;
                (name, None, Default::default())
            }
            ast::PinArrayIdDef::Array(ref name, width) => {
                let name = self.eval_templ_id(name)?;
                (name, Some(width), Default::default())
            }
            ast::PinArrayIdDef::ArrayRange(ref name, msb, lsb) => {
                let name = self.eval_templ_id(name)?;
                if msb < lsb {
                    (
                        name,
                        Some(lsb - msb + 1),
                        BelPinIndexing {
                            lsb_index: lsb,
                            wrong_endian: true,
                        },
                    )
                } else {
                    (
                        name,
                        Some(lsb - msb + 1),
                        BelPinIndexing {
                            lsb_index: lsb,
                            wrong_endian: false,
                        },
                    )
                }
            }
        })
    }

    fn eval_bel_class(&mut self, bcls: BelClassId, item: &ast::BelClassItem) -> Result<()> {
        match item {
            ast::BelClassItem::Input(pin) => {
                for name in &pin.names {
                    let (name, width, indexing) = self.eval_pin_indexing(name)?;
                    let inp = BelClassInput {
                        nonroutable: pin.nonroutable,
                        indexing,
                    };
                    match width {
                        None => {
                            if self.db.db.bel_classes[bcls]
                                .inputs
                                .insert(name.to_string(), inp)
                                .is_none()
                            {
                                error_at(name.span(), "bel input redefined")?
                            }
                            self.db.bcls[bcls]
                                .input_id
                                .insert(name.to_string(), (name, indexing));
                        }
                        Some(width) => {
                            if self.db.db.bel_classes[bcls]
                                .inputs
                                .insert_array(name.to_string(), width, inp)
                                .is_none()
                            {
                                error_at(name.span(), "bel input redefined")?
                            }
                            self.db.bcls[bcls].input_id.insert_array(
                                name.to_string(),
                                width,
                                (name, indexing),
                            );
                        }
                    }
                }
            }
            ast::BelClassItem::Output(pin) => {
                for name in &pin.names {
                    let (name, width, indexing) = self.eval_pin_indexing(name)?;
                    let outp = BelClassOutput {
                        nonroutable: pin.nonroutable,
                        indexing,
                    };
                    match width {
                        None => {
                            if self.db.db.bel_classes[bcls]
                                .outputs
                                .insert(name.to_string(), outp)
                                .is_none()
                            {
                                error_at(name.span(), "bel output redefined")?
                            }
                            self.db.bcls[bcls]
                                .output_id
                                .insert(name.to_string(), (name, indexing));
                        }
                        Some(width) => {
                            if self.db.db.bel_classes[bcls]
                                .outputs
                                .insert_array(name.to_string(), width, outp)
                                .is_none()
                            {
                                error_at(name.span(), "bel output redefined")?
                            }
                            self.db.bcls[bcls].output_id.insert_array(
                                name.to_string(),
                                width,
                                (name, indexing),
                            );
                        }
                    }
                }
            }
            ast::BelClassItem::Bidir(pin) => {
                for name in &pin.names {
                    let (name, width, indexing) = self.eval_pin_indexing(name)?;
                    let bidi = BelClassBidir {
                        nonroutable: pin.nonroutable,
                        indexing,
                    };
                    match width {
                        None => {
                            if self.db.db.bel_classes[bcls]
                                .bidirs
                                .insert(name.to_string(), bidi)
                                .is_none()
                            {
                                error_at(name.span(), "bel bidir redefined")?
                            }
                            self.db.bcls[bcls]
                                .bidir_id
                                .insert(name.to_string(), (name, indexing));
                        }
                        Some(width) => {
                            if self.db.db.bel_classes[bcls]
                                .bidirs
                                .insert_array(name.to_string(), width, bidi)
                                .is_none()
                            {
                                error_at(name.span(), "bel bidir redefined")?
                            }
                            self.db.bcls[bcls].bidir_id.insert_array(
                                name.to_string(),
                                width,
                                (name, indexing),
                            );
                        }
                    }
                }
            }
            ast::BelClassItem::Pad(pad) => {
                for name in &pad.names {
                    let pad = BelClassPad { kind: pad.kind };
                    match *name {
                        ast::ArrayIdDef::Plain(ref name) => {
                            let name = self.eval_templ_id(name)?;
                            if self.db.db.bel_classes[bcls]
                                .pads
                                .insert(name.to_string(), pad)
                                .is_none()
                            {
                                error_at(name.span(), "bel pad redefined")?
                            }
                            self.db.bcls[bcls]
                                .pad_id
                                .insert(name.to_string(), name.clone());
                        }
                        ast::ArrayIdDef::Array(ref name, num) => {
                            let name = self.eval_templ_id(name)?;
                            if self.db.db.bel_classes[bcls]
                                .pads
                                .insert_array(name.to_string(), num, pad)
                                .is_none()
                            {
                                error_at(name.span(), "bel pad redefined")?
                            }
                            self.db.bcls[bcls].pad_id.insert_array(
                                name.to_string(),
                                num,
                                name.clone(),
                            );
                        }
                    }
                }
            }
            ast::BelClassItem::Attribute(attr) => {
                let typ = match attr.typ {
                    ast::AttributeType::Bool => BelAttributeType::Bool,
                    ast::AttributeType::BitVec(width) => BelAttributeType::BitVec(width),
                    ast::AttributeType::BitVecArray(width, depth) => {
                        BelAttributeType::BitVecArray(width, depth)
                    }
                    ast::AttributeType::Enum(ref ident) => {
                        let Some((eid, _)) = self.db.db.enum_classes.get(&ident.to_string()) else {
                            error_at(ident.span(), "undefined enum")?
                        };
                        BelAttributeType::Enum(eid)
                    }
                };
                let attribute = BelClassAttribute { typ };
                for name in &attr.names {
                    let ident = self.eval_templ_id(name)?;
                    let (_, prev) = self.db.db.bel_classes[bcls]
                        .attributes
                        .insert(ident.to_string(), attribute.clone());
                    if prev.is_some() {
                        error_at(ident.span(), "bel attribute redefined")?
                    }
                    self.db.bcls[bcls].attr_id.push(ident.clone());
                }
            }
            ast::BelClassItem::ForLoop(for_loop) => {
                self.eval_for(for_loop, |ctx, subitem| ctx.eval_bel_class(bcls, subitem))?
            }
            ast::BelClassItem::If(if_) => self.eval_if(IfContext::Top, if_, |ctx, subitem| {
                ctx.eval_bel_class(bcls, subitem)
            })?,
        }
        Ok(())
    }

    fn eval_tile_slot(&mut self, tslot: TileSlotId, item: &ast::TileSlotItem) -> Result<()> {
        match item {
            ast::TileSlotItem::BelSlot(bslot) => {
                let (name, num) = match &bslot.name {
                    ast::ArrayIdDef::Plain(id) => (id, None),
                    ast::ArrayIdDef::Array(id, n) => (id, Some(*n)),
                };
                let name = self.eval_templ_id(name)?;
                let kind = match &bslot.kind {
                    ast::BelKind::Routing => BelKind::Routing,
                    ast::BelKind::Class(id) => {
                        let Some((bcls, _)) = self.db.db.bel_classes.get(&id.to_string()) else {
                            error_at(id.span(), "unknown bel class")?
                        };
                        BelKind::Class(bcls)
                    }
                    ast::BelKind::Legacy => BelKind::Legacy,
                };
                if let Some(num) = num {
                    let Some(range) =
                        self.db
                            .bslot_id
                            .insert_array(name.to_string(), num, name.clone())
                    else {
                        error_at(name.span(), "bel slot redefined")?
                    };
                    assert_eq!(range.first().unwrap(), self.db.db.bel_slots.next_id());
                    for i in 0..num {
                        self.db.db.bel_slots.insert(
                            format!("{name}[{i}]"),
                            BelSlot {
                                tile_slot: tslot,
                                kind,
                            },
                        );
                    }
                } else {
                    let Some(id) = self.db.bslot_id.insert(name.to_string(), name.clone()) else {
                        error_at(name.span(), "bel slot redefined")?
                    };
                    assert_eq!(id, self.db.db.bel_slots.next_id());
                    self.db.db.bel_slots.insert(
                        name.to_string(),
                        BelSlot {
                            tile_slot: tslot,
                            kind,
                        },
                    );
                }
            }
            ast::TileSlotItem::TileClass(tcls) => {
                for name in &tcls.names {
                    let name = self.eval_templ_id(name)?;
                    let (ccid, prev) = self.db.db.tile_classes.insert(
                        name.to_string(),
                        TileClass {
                            slot: tslot,
                            cells: Default::default(),
                            bitrects: Default::default(),
                            bels: Default::default(),
                        },
                    );
                    if prev.is_some() {
                        error_at(name.span(), "tile class redefined")?;
                    }
                    self.db.tcls_id.push(name);
                    self.db.tcls_cell_id.push(Default::default());
                    self.db.tcls_bitrect_id.push(Default::default());
                    for item in &tcls.items {
                        self.eval_tile_class(ccid, item)?;
                    }
                }
            }
            ast::TileSlotItem::ForLoop(for_loop) => {
                self.eval_for(for_loop, |ctx, subitem| ctx.eval_tile_slot(tslot, subitem))?
            }
            ast::TileSlotItem::If(if_) => self.eval_if(IfContext::Top, if_, |ctx, subitem| {
                ctx.eval_tile_slot(tslot, subitem)
            })?,
        }
        Ok(())
    }

    fn eval_tile_class(&mut self, tcls: TileClassId, item: &ast::TileClassItem) -> Result<()> {
        match item {
            ast::TileClassItem::Cell(names) => {
                for name in names {
                    let (name, num) = match name {
                        ast::ArrayIdDef::Plain(id) => (id, None),
                        ast::ArrayIdDef::Array(id, n) => (id, Some(*n)),
                    };
                    let name = self.eval_templ_id(name)?;
                    if let Some(num) = num {
                        let Some(range) = self.db.tcls_cell_id[tcls].insert_array(
                            name.to_string(),
                            num,
                            name.clone(),
                        ) else {
                            error_at(name.span(), "cell slot redefined")?
                        };
                        assert_eq!(
                            range.first().unwrap(),
                            self.db.db.tile_classes[tcls].cells.next_id()
                        );
                        for i in 0..num {
                            self.db.db.tile_classes[tcls]
                                .cells
                                .push(format!("{name}[{i}]"));
                        }
                    } else {
                        let Some(id) =
                            self.db.tcls_cell_id[tcls].insert(name.to_string(), name.clone())
                        else {
                            error_at(name.span(), "cell slot redefined")?
                        };
                        assert_eq!(id, self.db.db.tile_classes[tcls].cells.next_id());
                        self.db.db.tile_classes[tcls].cells.push(name.to_string());
                    }
                }
            }
            ast::TileClassItem::BitRect(name, class) => {
                let (name, num) = match name {
                    ast::ArrayIdDef::Plain(id) => (id, None),
                    ast::ArrayIdDef::Array(id, n) => (id, Some(*n)),
                };
                let name = self.eval_templ_id(name)?;
                let Some(&geometry) = self.bitrect_geoms.get(&class.to_string()) else {
                    error_at(class.span(), "undefined bitrect class")?
                };
                if let Some(num) = num {
                    let Some(range) = self.db.tcls_bitrect_id[tcls].insert_array(
                        name.to_string(),
                        num,
                        name.clone(),
                    ) else {
                        error_at(name.span(), "bitrect redefined")?
                    };
                    assert_eq!(
                        range.first().unwrap(),
                        self.db.db.tile_classes[tcls].bitrects.next_id()
                    );
                    for i in 0..num {
                        self.db.db.tile_classes[tcls].bitrects.push(BitRectInfo {
                            name: format!("{name}[{i}]"),
                            geometry,
                        });
                    }
                } else {
                    let Some(id) =
                        self.db.tcls_bitrect_id[tcls].insert(name.to_string(), name.clone())
                    else {
                        error_at(name.span(), "bitrect redefined")?
                    };
                    assert_eq!(id, self.db.db.tile_classes[tcls].bitrects.next_id());
                    self.db.db.tile_classes[tcls].bitrects.push(BitRectInfo {
                        name: name.to_string(),
                        geometry,
                    });
                }
            }
            ast::TileClassItem::SwitchBox(sbox) => {
                let slot = self.eval_array_ref(&sbox.slot, &self.db.bslot_id)?;
                if self.db.db.bel_slots[slot].kind != BelKind::Routing {
                    error_at(sbox.slot.span(), "switchbox must be on routing bel")?;
                }
                let mut switchbox = SwitchBox::default();
                for subitem in &sbox.items {
                    self.eval_switchbox(tcls, slot, &mut switchbox, subitem)?;
                }
                self.db.db.tile_classes[tcls]
                    .bels
                    .insert(slot, BelInfo::SwitchBox(switchbox));
            }
            ast::TileClassItem::Bel(sbel) => {
                let slot = self.eval_array_ref(&sbel.slot, &self.db.bslot_id)?;
                match self.db.db.bel_slots[slot].kind {
                    BelKind::Routing => {
                        error_at(sbel.slot.span(), "bel kind must not be switchbox")?
                    }
                    BelKind::Class(bcls) => {
                        let mut bel = Bel::default();
                        for subitem in &sbel.items {
                            self.eval_bel(tcls, slot, bcls, &mut bel, subitem)?;
                        }
                        self.db.db.tile_classes[tcls]
                            .bels
                            .insert(slot, BelInfo::Bel(bel));
                    }
                    BelKind::Legacy => {
                        if !sbel.items.is_empty() {
                            error_at(sbel.slot.span(), "legacy bel must not have items")?;
                        }
                        self.db.db.tile_classes[tcls]
                            .bels
                            .insert(slot, BelInfo::Legacy(Default::default()));
                    }
                }
            }
            ast::TileClassItem::ForLoop(for_loop) => {
                self.eval_for(for_loop, |ctx, subitem| ctx.eval_tile_class(tcls, subitem))?
            }
            ast::TileClassItem::If(if_) => {
                self.eval_if(IfContext::TileClass(tcls), if_, |ctx, subitem| {
                    ctx.eval_tile_class(tcls, subitem)
                })?
            }
        }
        Ok(())
    }

    fn eval_switchbox(
        &mut self,
        tcls: TileClassId,
        bslot: BelSlotId,
        switchbox: &mut SwitchBox,
        item: &ast::SwitchBoxItem,
    ) -> Result<()> {
        match item {
            ast::SwitchBoxItem::PermaBuf(dst, src) => {
                let dst = self.eval_wire_ref(tcls, dst)?;
                let src = self.eval_pol_wire_ref(tcls, src)?;
                switchbox
                    .items
                    .push(SwitchBoxItem::PermaBuf(PermaBuf { dst, src }));
            }
            ast::SwitchBoxItem::ProgBuf(dst, src) => {
                let dst = self.eval_wire_ref(tcls, dst)?;
                let src = self.eval_pol_wire_ref(tcls, src)?;
                switchbox.items.push(SwitchBoxItem::ProgBuf(ProgBuf {
                    dst,
                    src,
                    bit: PolTileBit::DUMMY,
                }));
            }
            ast::SwitchBoxItem::ProgInv(dst, src) => {
                let dst = self.eval_wire_ref(tcls, dst)?;
                let src = self.eval_wire_ref(tcls, src)?;
                switchbox.items.push(SwitchBoxItem::ProgInv(ProgInv {
                    dst,
                    src,
                    bit: PolTileBit::DUMMY,
                }));
            }
            ast::SwitchBoxItem::Mux(dst, srcs) => {
                let mut mux = Mux {
                    dst: self.eval_wire_ref(tcls, dst)?,
                    bits: Default::default(),
                    src: Default::default(),
                    bits_off: None,
                };
                for src in srcs {
                    let src = self.eval_pol_wire_ref(tcls, src)?;
                    mux.src.insert(src, Default::default());
                }
                switchbox.items.push(SwitchBoxItem::Mux(mux));
            }
            ast::SwitchBoxItem::Bidi(conn, wire) => {
                let id = self.eval_templ_id(conn)?;
                let Some((csid, _)) = self.db.db.conn_slots.get(&id.to_string()) else {
                    error_at(id.span(), &format!("undefined connector slot: {id}"))?
                };
                let bidi = Bidi {
                    conn: csid,
                    wire: self.eval_wire_ref(tcls, wire)?,
                    bit_upstream: PolTileBit::DUMMY,
                };
                switchbox.items.push(SwitchBoxItem::Bidi(bidi));
            }
            ast::SwitchBoxItem::ForLoop(for_loop) => self.eval_for(for_loop, |ctx, subitem| {
                ctx.eval_switchbox(tcls, bslot, switchbox, subitem)
            })?,
            ast::SwitchBoxItem::If(if_) => {
                self.eval_if(IfContext::Bel(tcls, bslot), if_, |ctx, subitem| {
                    ctx.eval_switchbox(tcls, bslot, switchbox, subitem)
                })?
            }
        }
        Ok(())
    }

    fn eval_pin_array_ref<T: EntityId>(
        &self,
        id: &ast::ArrayIdRef,
        map: &EntityBundleMap<T, (Ident, BelPinIndexing)>,
    ) -> Result<T> {
        match id {
            ast::ArrayIdRef::Plain(id) => {
                let id = self.eval_templ_id(id)?;
                let Some((item, _)) = map.get(&id.to_string()) else {
                    error_at(id.span(), "undefined object")?
                };
                match item {
                    EntityBundleIndex::Single(id) => Ok(id),
                    EntityBundleIndex::Array(_) => error_at(id.span(), "object is an array")?,
                }
            }
            ast::ArrayIdRef::Indexed(id, index) => {
                let id = self.eval_templ_id(id)?;
                let index = self.eval_index(index)?;
                let Some((item, &(_, indexing))) = map.get(&id.to_string()) else {
                    error_at(id.span(), "undefined object")?
                };
                match item {
                    EntityBundleIndex::Single(_) => error_at(id.span(), "object is not an array")?,
                    EntityBundleIndex::Array(range) => {
                        let Some(index) = indexing.try_virt_to_phys(index) else {
                            error_at(id.span(), "index out of bounds")?
                        };
                        if index > range.len() {
                            error_at(id.span(), "index out of bounds")?
                        }
                        Ok(range.index(index))
                    }
                }
            }
        }
    }

    fn eval_bel(
        &mut self,
        tcls: TileClassId,
        bslot: BelSlotId,
        bcls: BelClassId,
        bel: &mut Bel,
        item: &ast::BelItem,
    ) -> Result<()> {
        match item {
            ast::BelItem::Input(pin, wire_ref) => {
                let pin = self.eval_pin_array_ref(pin, &self.db.bcls[bcls].input_id)?;
                let wire = self.eval_pol_wire_ref(tcls, wire_ref)?;
                bel.inputs.insert(pin, BelInput::Fixed(wire));
            }
            ast::BelItem::Output(pin, wire_refs) => {
                let pin = self.eval_pin_array_ref(pin, &self.db.bcls[bcls].output_id)?;
                let mut wires = BTreeSet::new();
                for wref in wire_refs {
                    let wire = self.eval_wire_ref(tcls, wref)?;
                    wires.insert(wire);
                }
                bel.outputs.insert(pin, wires);
            }
            ast::BelItem::Bidir(pin, wire_ref) => {
                let pin = self.eval_pin_array_ref(pin, &self.db.bcls[bcls].bidir_id)?;
                let wire = self.eval_wire_ref(tcls, wire_ref)?;
                bel.bidirs.insert(pin, wire);
            }
            ast::BelItem::Attribute(attr) => {
                let name = self.eval_templ_id(&attr.name)?;
                let Some((aid, cattr)) = self.db.db.bel_classes[bcls]
                    .attributes
                    .get(&name.to_string())
                else {
                    error_at(name.span(), "unknown attribute")?
                };
                if let Some(ref avalues) = attr.values {
                    let mut bits = vec![];
                    for bit in &attr.bits {
                        bits.push(self.eval_tile_bit(tcls, bit)?);
                    }
                    let BelAttributeType::Enum(ecid) = cattr.typ else {
                        unreachable!()
                    };
                    let mut values = EntityPartVec::new();
                    for (vname, val) in avalues {
                        let vname = self.eval_templ_id(vname)?;
                        let Some(vid) =
                            self.db.db.enum_classes[ecid].values.get(&vname.to_string())
                        else {
                            error_at(name.span(), "unknown enum value")?
                        };
                        values.insert(vid, val.clone());
                    }
                    bel.attributes
                        .insert(aid, BelAttribute::Enum(BelAttributeEnum { bits, values }));
                } else {
                    let mut bits = vec![];
                    for bit in &attr.bits {
                        bits.push(self.eval_pol_tile_bit(tcls, bit)?);
                    }
                    match cattr.typ {
                        BelAttributeType::Enum(_) => error_at(name.span(), "missing enum values")?,
                        BelAttributeType::Bool => {
                            if bits.len() != 1 {
                                error_at(name.span(), "expected single bit")?;
                            }
                        }
                        BelAttributeType::BitVec(width) => {
                            if bits.len() != width {
                                error_at(
                                    name.span(),
                                    &format!("expected {width} bits, got {n}", n = bits.len()),
                                )?;
                            }
                        }
                        BelAttributeType::BitVecArray(_, _) => todo!("bitvec array"),
                    }
                    bel.attributes.insert(aid, BelAttribute::BitVec(bits));
                }
            }
            ast::BelItem::ForLoop(for_loop) => self.eval_for(for_loop, |ctx, subitem| {
                ctx.eval_bel(tcls, bslot, bcls, bel, subitem)
            })?,
            ast::BelItem::If(if_) => {
                self.eval_if(IfContext::Bel(tcls, bslot), if_, |ctx, subitem| {
                    ctx.eval_bel(tcls, bslot, bcls, bel, subitem)
                })?
            }
        }
        Ok(())
    }

    fn eval_connector_slot(
        &mut self,
        cslot: ConnectorSlotId,
        got_opposite: &mut bool,
        item: &ast::ConnectorSlotItem,
    ) -> Result<()> {
        match item {
            ast::ConnectorSlotItem::Opposite(ident) => {
                if *got_opposite {
                    error_at(ident.span(), "opposite slot redefined")?;
                }
                *got_opposite = true;
                let ident = self.eval_templ_id(ident)?;
                let Some((opposite, _)) = self.db.db.conn_slots.get(&ident.to_string()) else {
                    error_at(ident.span(), &format!("undefined connector slot: {ident}"))?
                };
                self.db.db.conn_slots[cslot].opposite = opposite;
            }
            ast::ConnectorSlotItem::ConnectorClass(ccls) => {
                for name in &ccls.names {
                    let name = self.eval_templ_id(name)?;
                    let (ccid, prev) = self.db.db.conn_classes.insert(
                        name.to_string(),
                        ConnectorClass {
                            slot: cslot,
                            wires: Default::default(),
                        },
                    );
                    if prev.is_some() {
                        error_at(name.span(), "connector class redefined")?;
                    }
                    self.db.ccls_id.push(name);
                    for item in &ccls.items {
                        self.eval_connector_class(ccid, item)?;
                    }
                }
            }
            ast::ConnectorSlotItem::ForLoop(for_loop) => self
                .eval_for(for_loop, |ctx, subitem| {
                    ctx.eval_connector_slot(cslot, got_opposite, subitem)
                })?,
            ast::ConnectorSlotItem::If(if_) => {
                self.eval_if(IfContext::Top, if_, |ctx, subitem| {
                    ctx.eval_connector_slot(cslot, got_opposite, subitem)
                })?
            }
        }
        Ok(())
    }

    fn eval_connector_class(
        &mut self,
        ccls: ConnectorClassId,
        item: &ast::ConnectorClassItem,
    ) -> Result<()> {
        match item {
            ast::ConnectorClassItem::Pass(dst, src) => {
                let span = dst.span();
                let dst = self.eval_array_ref_wide(dst, &self.db.wire_id)?;
                let src = self.eval_array_ref_wide(src, &self.db.wire_id)?;
                let ccls = &mut self.db.db.conn_classes[ccls];
                match (dst, src) {
                    (EntityBundleIndex::Single(dst), EntityBundleIndex::Single(src)) => {
                        ccls.wires.insert(dst, ConnectorWire::Pass(src));
                    }
                    (EntityBundleIndex::Array(dsts), EntityBundleIndex::Array(srcs))
                        if dsts.len() == srcs.len() =>
                    {
                        for (dst, src) in dsts.into_iter().zip(srcs) {
                            ccls.wires.insert(dst, ConnectorWire::Pass(src));
                        }
                    }
                    _ => error_at(span, "assignment width mismatch")?,
                }
            }
            ast::ConnectorClassItem::Reflect(dst, src) => {
                let span = dst.span();
                let dst = self.eval_array_ref_wide(dst, &self.db.wire_id)?;
                let src = self.eval_array_ref_wide(src, &self.db.wire_id)?;
                let ccls = &mut self.db.db.conn_classes[ccls];
                match (dst, src) {
                    (EntityBundleIndex::Single(dst), EntityBundleIndex::Single(src)) => {
                        ccls.wires.insert(dst, ConnectorWire::Reflect(src));
                    }
                    (EntityBundleIndex::Array(dsts), EntityBundleIndex::Array(srcs))
                        if dsts.len() == srcs.len() =>
                    {
                        for (dst, src) in dsts.into_iter().zip(srcs) {
                            ccls.wires.insert(dst, ConnectorWire::Reflect(src));
                        }
                    }
                    _ => error_at(span, "assignment width mismatch")?,
                }
            }
            ast::ConnectorClassItem::Blackhole(dst) => {
                let dst = self.eval_array_ref_wide(dst, &self.db.wire_id)?;
                let ccls = &mut self.db.db.conn_classes[ccls];
                match dst {
                    EntityBundleIndex::Single(dst) => {
                        ccls.wires.insert(dst, ConnectorWire::BlackHole);
                    }
                    EntityBundleIndex::Array(dsts) => {
                        for dst in dsts {
                            ccls.wires.insert(dst, ConnectorWire::BlackHole);
                        }
                    }
                }
            }
            ast::ConnectorClassItem::ForLoop(for_loop) => self
                .eval_for(for_loop, |ctx, subitem| {
                    ctx.eval_connector_class(ccls, subitem)
                })?,
            ast::ConnectorClassItem::If(if_) => {
                self.eval_if(IfContext::ConnectorClass(ccls), if_, |ctx, subitem| {
                    ctx.eval_connector_class(ccls, subitem)
                })?
            }
        }
        Ok(())
    }

    fn eval_table(&mut self, tid: TableId, item: &ast::TableItem) -> Result<()> {
        match item {
            ast::TableItem::Field(field) => {
                let typ = match field.typ {
                    ast::AttributeType::Bool => BelAttributeType::Bool,
                    ast::AttributeType::BitVec(width) => BelAttributeType::BitVec(width),
                    ast::AttributeType::BitVecArray(width, depth) => {
                        BelAttributeType::BitVecArray(width, depth)
                    }
                    ast::AttributeType::Enum(ref ident) => {
                        let Some((eid, _)) = self.db.db.enum_classes.get(&ident.to_string()) else {
                            error_at(ident.span(), "undefined enum")?
                        };
                        BelAttributeType::Enum(eid)
                    }
                };
                for name in &field.names {
                    let ident = self.eval_templ_id(name)?;
                    let (_, prev) = self.db.db.tables[tid].fields.insert(ident.to_string(), typ);
                    if prev.is_some() {
                        error_at(ident.span(), "table field redefined")?
                    }
                    self.db.table[tid].field_id.push(ident.clone());
                }
            }
            ast::TableItem::Row(names) => {
                for name in names {
                    let ident = self.eval_templ_id(name)?;
                    let (_, prev) = self.db.db.tables[tid]
                        .rows
                        .insert(ident.to_string(), EntityPartVec::new());
                    if prev.is_some() {
                        error_at(ident.span(), "table row redefined")?
                    }
                    self.db.table[tid].row_id.push(ident.clone());
                }
            }
            ast::TableItem::ForLoop(for_loop) => {
                self.eval_for(for_loop, |ctx, subitem| ctx.eval_table(tid, subitem))?
            }
            ast::TableItem::If(if_) => self.eval_if(IfContext::Top, if_, |ctx, subitem| {
                ctx.eval_table(tid, subitem)
            })?,
        }
        Ok(())
    }

    fn eval_wire_kind(&self, kind: &ast::WireKind) -> Result<WireKind> {
        match kind {
            ast::WireKind::Tie0 => Ok(WireKind::Tie0),
            ast::WireKind::Tie1 => Ok(WireKind::Tie1),
            ast::WireKind::TiePullup => Ok(WireKind::TiePullup),
            ast::WireKind::Regional(id) => {
                let id = self.eval_templ_id(id)?;
                let Some(rslot) = self.db.db.region_slots.get(&id.to_string()) else {
                    error_at(id.span(), "unknown region slot")?
                };
                Ok(WireKind::Regional(rslot))
            }
            ast::WireKind::Mux => Ok(WireKind::MuxOut),
            ast::WireKind::Bel => Ok(WireKind::BelOut),
            ast::WireKind::Test => Ok(WireKind::TestOut),
            ast::WireKind::MultiRoot => Ok(WireKind::MultiRoot),
            ast::WireKind::MultiBranch(id) => {
                let Some((csid, _)) = self.db.db.conn_slots.get(&id.to_string()) else {
                    error_at(id.span(), &format!("undefined connector slot: {id}"))?
                };
                Ok(WireKind::MultiBranch(csid))
            }
            ast::WireKind::Branch(id) => {
                let Some((csid, _)) = self.db.db.conn_slots.get(&id.to_string()) else {
                    error_at(id.span(), &format!("undefined connector slot: {id}"))?
                };
                Ok(WireKind::Branch(csid))
            }
            ast::WireKind::Special => Ok(WireKind::Special),
        }
    }

    fn eval_wire_kinds(
        &self,
        span: Span,
        kind: &ast::WireKinds,
        idx: Option<usize>,
    ) -> Result<WireKind> {
        match kind {
            ast::WireKinds::Single(kind) => self.eval_wire_kind(kind),
            ast::WireKinds::Multi(kinds) => {
                let idx = idx.unwrap();
                for (range, kind) in kinds {
                    if range.contains(&idx) {
                        return self.eval_wire_kind(kind);
                    }
                }
                error_at(span, "index not in match")?
            }
        }
    }

    fn eval_top_ph1(&mut self, item: &ast::TopItem) -> Result<()> {
        match item {
            ast::TopItem::Variant(ident) => {
                error_at(ident.span(), "variants must be at the top level")?;
            }
            ast::TopItem::BitRectClass(brc) => {
                if self
                    .bitrect_geoms
                    .insert(brc.name.to_string(), brc.geometry)
                    .is_some()
                {
                    error_at(brc.name.span(), "bitrect class redefined")?;
                }
            }
            ast::TopItem::EnumClass(ecls) => {
                let mut values = EntitySet::new();
                let mut vids = EntityVec::new();
                for val in &ecls.values {
                    let (_vid, new) = values.insert(val.to_string());
                    if !new {
                        error_at(val.span(), "enum value redefined")?;
                    }
                    vids.push(val.clone());
                }
                let (_eclsid, prev) = self
                    .db
                    .db
                    .enum_classes
                    .insert(ecls.name.to_string(), EnumClass { values });
                if prev.is_some() {
                    error_at(ecls.name.span(), "enum class redefined")?;
                }
                self.db.enum_id.push(ecls.name.clone());
                self.db.eval_id.push(vids);
            }
            ast::TopItem::BelClass(bcls) => {
                let (_bcid, prev) = self
                    .db
                    .db
                    .bel_classes
                    .insert(bcls.name.to_string(), BelClass::default());
                if prev.is_some() {
                    error_at(bcls.name.span(), "bel class redefined")?;
                }
                self.db.bcls_id.push(bcls.name.clone());
                self.db.bcls.push(Default::default());
            }
            ast::TopItem::TileSlot(tslot) => {
                let name = self.eval_templ_id(&tslot.name)?;
                let (_tsid, new) = self.db.db.tile_slots.insert(name.to_string());
                if !new {
                    error_at(tslot.name.span(), "tile slot redefined")?;
                }
                self.db.tslot_id.push(name);
            }
            ast::TopItem::RegionSlot(rslot) => {
                let name = self.eval_templ_id(&rslot.name)?;
                let (_, new) = self.db.db.region_slots.insert(name.to_string());
                if !new {
                    error_at(rslot.name.span(), "region slot redefined")?;
                }
                self.db.rslot_id.push(name);
            }
            ast::TopItem::ConnectorSlot(cslot) => {
                let name = self.eval_templ_id(&cslot.name)?;
                let (_csid, prev) = self.db.db.conn_slots.insert(
                    name.to_string(),
                    ConnectorSlot {
                        opposite: ConnectorSlotId::from_idx(0),
                    },
                );
                if prev.is_some() {
                    error_at(cslot.name.span(), "connector slot redefined")?;
                }
                self.db.cslot_id.push(name);
            }
            ast::TopItem::Wire(_) => (),
            ast::TopItem::Table(_) => (),
            ast::TopItem::ForLoop(for_loop) => {
                self.eval_for(for_loop, |ctx, subitem| ctx.eval_top_ph1(subitem))?
            }
            ast::TopItem::If(if_) => self.eval_if(IfContext::Top, if_, |ctx, subitem| {
                ctx.eval_top_ph1(subitem)
            })?,
        }
        Ok(())
    }

    fn eval_top(&mut self, item: &ast::TopItem) -> Result<()> {
        match item {
            ast::TopItem::Variant(ident) => {
                error_at(ident.span(), "variants must be at the top level")?;
            }
            ast::TopItem::BitRectClass(_) => (),
            ast::TopItem::EnumClass(_) => (),
            ast::TopItem::BelClass(bcls) => {
                let bcid = self
                    .db
                    .db
                    .bel_classes
                    .get(&bcls.name.to_string())
                    .unwrap()
                    .0;
                for item in &bcls.items {
                    self.eval_bel_class(bcid, item)?;
                }
            }
            ast::TopItem::TileSlot(tslot) => {
                let name = self.eval_templ_id(&tslot.name)?;
                let tsid = self.db.db.tile_slots.get(&name.to_string()).unwrap();
                for item in &tslot.items {
                    self.eval_tile_slot(tsid, item)?;
                }
            }
            ast::TopItem::RegionSlot(_) => (),
            ast::TopItem::ConnectorSlot(cslot) => {
                let name = self.eval_templ_id(&cslot.name)?;
                let csid = self.db.db.conn_slots.get(&name.to_string()).unwrap().0;
                let mut got_opposite = false;
                for item in &cslot.items {
                    self.eval_connector_slot(csid, &mut got_opposite, item)?;
                }
                if !got_opposite {
                    error_at(cslot.name.span(), "missing opposite connector slot")?;
                }
            }
            ast::TopItem::Wire(wire) => {
                let (name, num) = match &wire.name {
                    ast::ArrayIdDef::Plain(id) => (id, None),
                    ast::ArrayIdDef::Array(id, n) => (id, Some(*n)),
                };
                let name = self.eval_templ_id(name)?;
                if let Some(num) = num {
                    let Some(range) =
                        self.db
                            .wire_id
                            .insert_array(name.to_string(), num, name.clone())
                    else {
                        error_at(name.span(), "wire redefined")?
                    };
                    assert_eq!(range.first().unwrap(), self.db.db.wires.next_id());
                    for i in 0..num {
                        let kind = self.eval_wire_kinds(name.span(), &wire.kinds, Some(i))?;
                        self.db.db.wires.insert(format!("{name}[{i}]"), kind);
                    }
                } else {
                    let Some(id) = self.db.wire_id.insert(name.to_string(), name.clone()) else {
                        error_at(name.span(), "wire redefined")?
                    };
                    assert_eq!(id, self.db.db.wires.next_id());
                    let kind = self.eval_wire_kinds(name.span(), &wire.kinds, None)?;
                    self.db.db.wires.insert(name.to_string(), kind);
                }
            }
            ast::TopItem::Table(table) => {
                let name = self.eval_templ_id(&table.name)?;
                let (tid, prev) = self.db.db.tables.insert(
                    name.to_string(),
                    Table {
                        fields: Default::default(),
                        rows: Default::default(),
                    },
                );
                if prev.is_some() {
                    error_at(table.name.span(), "table redefined")?;
                }
                self.db.table_id.push(name);
                self.db.table.push(Default::default());
                for item in &table.items {
                    self.eval_table(tid, item)?;
                }
            }
            ast::TopItem::ForLoop(for_loop) => {
                self.eval_for(for_loop, |ctx, subitem| ctx.eval_top(subitem))?
            }
            ast::TopItem::If(if_) => {
                self.eval_if(IfContext::Top, if_, |ctx, subitem| ctx.eval_top(subitem))?
            }
        }
        Ok(())
    }
}

fn eval_variant(variant: Option<Ident>, items: &[ast::TopItem]) -> Result<AnnotatedDb> {
    let mut ctx = Context {
        db: AnnotatedDb {
            name: variant,
            db: IntDb::default(),
            enum_id: EntityVec::new(),
            eval_id: EntityVec::new(),
            tslot_id: EntityVec::new(),
            cslot_id: EntityVec::new(),
            bslot_id: Default::default(),
            rslot_id: EntityVec::new(),
            tcls_id: EntityVec::new(),
            ccls_id: EntityVec::new(),
            bcls_id: EntityVec::new(),
            bcls: EntityVec::new(),
            wire_id: Default::default(),
            tcls_cell_id: Default::default(),
            tcls_bitrect_id: Default::default(),
            table_id: Default::default(),
            table: Default::default(),
        },
        for_vars: Default::default(),
        bitrect_geoms: Default::default(),
    };
    for item in items {
        if let ast::TopItem::Variant(_) = item {
            continue;
        }
        ctx.eval_top_ph1(item)?;
    }
    for item in items {
        if let ast::TopItem::Variant(_) = item {
            continue;
        }
        ctx.eval_top(item)?;
    }
    Ok(ctx.db)
}

pub fn eval(items: Vec<ast::TopItem>) -> Result<Vec<AnnotatedDb>> {
    let mut variants = vec![];
    for item in &items {
        if let ast::TopItem::Variant(variant) = item {
            variants.push(variant.clone());
        }
    }
    if variants.is_empty() {
        Ok(vec![eval_variant(None, &items)?])
    } else {
        let mut res = vec![];
        for variant in variants {
            res.push(eval_variant(Some(variant), &items)?);
        }
        Ok(res)
    }
}
