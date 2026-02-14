use std::str::FromStr;

use prjcombine_entity::{EntityBundleIndices, EntityBundleMap, EntityId, EntityVec};
use prjcombine_interconnect::db::{BelPinIndexing, IntDb};
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::db::AnnotatedDb;

fn to_ident(s: &str) -> Ident {
    Ident::new(s, Span::call_site())
}

fn keyword(kw: &str) -> TokenTree {
    TokenTree::Ident(Ident::new(kw, Span::call_site()))
}

fn joint(ch: char) -> TokenTree {
    TokenTree::Punct(Punct::new(ch, Spacing::Joint))
}

fn punct(ch: char) -> TokenTree {
    TokenTree::Punct(Punct::new(ch, Spacing::Alone))
}

fn parens(stream: TokenStream) -> TokenTree {
    TokenTree::Group(Group::new(Delimiter::Parenthesis, stream))
}

fn num(n: usize) -> TokenTree {
    TokenTree::Literal(Literal::usize_unsuffixed(n))
}

fn emit_ids<I: EntityId>(cls: &str, idents: &EntityVec<I, Ident>) -> TokenStream {
    let mut res = TokenStream::new();
    emit_use(&mut res, &["prjcombine_interconnect", "db", cls]);

    for (id, name) in idents {
        res.extend([
            keyword("pub"),
            keyword("const"),
            TokenTree::Ident(name.clone()),
            punct(':'),
            keyword(cls),
            punct('='),
            keyword(cls),
            joint(':'),
            punct(':'),
            keyword("from_idx_const"),
            parens(TokenStream::from_iter([num(id.to_idx())])),
            punct(';'),
        ]);
    }

    res
}

fn emit_array_ids<I: EntityId>(
    cls: &str,
    idents: &EntityBundleMap<I, Ident>,
    do_use: bool,
) -> TokenStream {
    let mut res = TokenStream::new();
    if do_use {
        emit_use(&mut res, &["prjcombine_entity", "id", "EntityStaticRange"]);
    }
    emit_use(&mut res, &["prjcombine_interconnect", "db", cls]);

    for (index, _, ident) in idents.bundles() {
        match index {
            EntityBundleIndices::Single(id) => {
                res.extend([
                    keyword("pub"),
                    keyword("const"),
                    TokenTree::Ident(ident.clone()),
                    punct(':'),
                    keyword(cls),
                    punct('='),
                    keyword(cls),
                    joint(':'),
                    punct(':'),
                    keyword("from_idx_const"),
                    parens(TokenStream::from_iter([num(id.to_idx())])),
                    punct(';'),
                ]);
            }
            EntityBundleIndices::Array(range) => {
                res.extend([
                    keyword("pub"),
                    keyword("const"),
                    TokenTree::Ident(ident.clone()),
                    punct(':'),
                    keyword("EntityStaticRange"),
                    punct('<'),
                    keyword(cls),
                    punct(','),
                    num(range.len()),
                    punct('>'),
                    punct('='),
                    keyword("EntityStaticRange"),
                    joint(':'),
                    punct(':'),
                    punct('<'),
                    keyword(cls),
                    punct(','),
                    keyword("_"),
                    punct('>'),
                    joint(':'),
                    punct(':'),
                    keyword("new_const"),
                    parens(TokenStream::from_iter([num(range
                        .first()
                        .unwrap()
                        .to_idx())])),
                    punct(';'),
                ]);
            }
        }
    }

    res
}

fn emit_pin_array_ids<I: EntityId>(
    cls: &str,
    idents: &EntityBundleMap<I, (Ident, BelPinIndexing)>,
    do_use: bool,
) -> TokenStream {
    let mut new_idents: EntityBundleMap<I, _> = EntityBundleMap::new();
    for (idx, name, (ident, _)) in idents.bundles() {
        match idx {
            EntityBundleIndices::Single(_) => {
                new_idents.insert(name.into(), ident.clone());
            }
            EntityBundleIndices::Array(range) => {
                new_idents.insert_array(name.into(), range.len(), ident.clone());
            }
        }
    }
    emit_array_ids(cls, &new_idents, do_use)
}

fn emit_tile_classes(stream: &mut TokenStream, adb: &AnnotatedDb) {
    let mut mod_stream = emit_ids("TileClassId", &adb.tcls_id);
    for (tcid, cell_ids) in &adb.tcls_cell_id {
        let cell_stream = emit_array_ids("CellSlotId", cell_ids, true);
        mod_stream.extend(TokenStream::from_str("#[allow(non_snake_case)]").unwrap());
        emit_mod(&mut mod_stream, adb.tcls_id[tcid].clone(), cell_stream);
    }
    emit_mod(stream, to_ident("tcls"), mod_stream);
}

fn emit_bel_classes(stream: &mut TokenStream, adb: &AnnotatedDb) {
    let mut mod_stream = emit_ids("BelClassId", &adb.bcls_id);
    for (bcid, bcls) in &adb.bcls {
        let mut inner = TokenStream::new();
        inner.extend(emit_pin_array_ids("BelInputId", &bcls.input_id, true));
        inner.extend(emit_pin_array_ids("BelOutputId", &bcls.output_id, false));
        inner.extend(emit_pin_array_ids("BelBidirId", &bcls.bidir_id, false));
        inner.extend(emit_array_ids("BelPadId", &bcls.pad_id, false));
        inner.extend(emit_ids("BelAttributeId", &bcls.attr_id));
        mod_stream.extend(TokenStream::from_str("#[allow(non_snake_case)]").unwrap());
        emit_mod(&mut mod_stream, adb.bcls_id[bcid].clone(), inner);
    }
    emit_mod(stream, to_ident("bcls"), mod_stream);
}

fn emit_enums(stream: &mut TokenStream, adb: &AnnotatedDb) {
    let mut mod_stream = emit_ids("EnumClassId", &adb.enum_id);
    for (ecid, vals) in &adb.eval_id {
        let val_stream = emit_ids("EnumValueId", vals);
        mod_stream.extend(TokenStream::from_str("#[allow(non_snake_case)]").unwrap());
        emit_mod(&mut mod_stream, adb.enum_id[ecid].clone(), val_stream);
    }
    emit_mod(stream, to_ident("enums"), mod_stream);
}

fn emit_tables(stream: &mut TokenStream, adb: &AnnotatedDb) {
    let mut mod_stream = emit_ids("TableId", &adb.table_id);
    for (tid, tdata) in &adb.table {
        let mut inner = TokenStream::new();
        inner.extend(emit_ids("TableFieldId", &tdata.field_id));
        inner.extend(emit_ids("TableRowId", &tdata.row_id));
        mod_stream.extend(TokenStream::from_str("#[allow(non_snake_case)]").unwrap());
        emit_mod(&mut mod_stream, adb.table_id[tid].clone(), inner);
    }
    emit_mod(stream, to_ident("tables"), mod_stream);
}

fn emit_use(stream: &mut TokenStream, name: &[&str]) {
    stream.extend([keyword("use")]);
    for part in name {
        stream.extend([
            TokenTree::Punct(Punct::new(':', Spacing::Joint)),
            TokenTree::Punct(Punct::new(':', Spacing::Alone)),
            keyword(part),
        ]);
    }
    stream.extend([Punct::new(';', Spacing::Alone)]);
}

fn emit_mod(stream: &mut TokenStream, name: Ident, block: TokenStream) {
    let mut group = TokenTree::Group(Group::new(Delimiter::Brace, block));
    group.set_span(name.span());
    stream.extend([
        keyword("pub"),
        keyword("mod"),
        TokenTree::Ident(name),
        group,
    ]);
}

fn emit_init(stream: &mut TokenStream, db: &IntDb) {
    let data = bincode::encode_to_vec(db, bincode::config::standard()).unwrap();
    stream.extend(TokenStream::from_str("pub const INIT: &[u8] = ").unwrap());
    stream.extend([TokenTree::Literal(Literal::byte_string(&data)), punct(';')]);
}

pub fn emit(dbs: Vec<AnnotatedDb>) -> TokenStream {
    let mut enum_uniform = true;
    let mut bcls_uniform = true;
    let mut bslot_uniform = true;
    let mut tslot_uniform = true;
    let mut tcls_uniform = true;
    let mut rslot_uniform = true;
    let mut cslot_uniform = true;
    let mut ccls_uniform = true;
    let mut wire_uniform = true;
    let mut table_uniform = true;
    let mut devdata_uniform = true;
    for db in &dbs[1..] {
        if db.db.enum_classes != dbs[0].db.enum_classes {
            enum_uniform = false;
        }
        if db.db.bel_classes != dbs[0].db.bel_classes {
            bcls_uniform = false;
        }
        if Vec::from_iter(db.db.bel_slots.keys()) != Vec::from_iter(dbs[0].db.bel_slots.keys()) {
            bslot_uniform = false;
        }
        if db.db.tile_slots != dbs[0].db.tile_slots {
            tslot_uniform = false;
        }
        if db.db.tile_classes != dbs[0].db.tile_classes {
            tcls_uniform = false;
        }
        if db.db.region_slots != dbs[0].db.region_slots {
            rslot_uniform = false;
        }
        if db.db.conn_classes != dbs[0].db.conn_classes {
            ccls_uniform = false;
        }
        if db.db.conn_slots != dbs[0].db.conn_slots {
            cslot_uniform = false;
        }
        if db.db.wires != dbs[0].db.wires {
            wire_uniform = false;
        }
        if db.db.tables != dbs[0].db.tables {
            table_uniform = false;
        }
        if db.db.devdata != dbs[0].db.devdata {
            devdata_uniform = false;
        }
    }
    let mut res = TokenStream::new();
    let mut variant_outs = Vec::from_iter(dbs.iter().map(|_| TokenStream::new()));

    if enum_uniform {
        emit_enums(&mut res, &dbs[0]);
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::enums;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_enums(stream, adb);
        }
    }

    if tslot_uniform {
        emit_mod(
            &mut res,
            to_ident("tslots"),
            emit_ids("TileSlotId", &dbs[0].tslot_id),
        );
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::tslots;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_mod(
                stream,
                to_ident("tslots"),
                emit_ids("TileSlotId", &adb.tslot_id),
            );
        }
    }

    if rslot_uniform {
        emit_mod(
            &mut res,
            to_ident("rslots"),
            emit_ids("RegionSlotId", &dbs[0].rslot_id),
        );
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::rslots;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_mod(
                stream,
                to_ident("rslots"),
                emit_ids("RegionSlotId", &adb.rslot_id),
            );
        }
    }

    if cslot_uniform {
        emit_mod(
            &mut res,
            to_ident("cslots"),
            emit_ids("ConnectorSlotId", &dbs[0].cslot_id),
        );
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::cslots;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_mod(
                stream,
                to_ident("cslots"),
                emit_ids("ConnectorSlotId", &adb.cslot_id),
            );
        }
    }

    if tcls_uniform {
        emit_tile_classes(&mut res, &dbs[0]);
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::tcls;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_tile_classes(stream, adb);
        }
    }

    if bcls_uniform {
        emit_bel_classes(&mut res, &dbs[0]);
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::bcls;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_bel_classes(stream, adb);
        }
    }

    if ccls_uniform {
        emit_mod(
            &mut res,
            to_ident("ccls"),
            emit_ids("ConnectorClassId", &dbs[0].ccls_id),
        );
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::ccls;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_mod(
                stream,
                to_ident("ccls"),
                emit_ids("ConnectorClassId", &adb.ccls_id),
            );
        }
    }

    if bslot_uniform {
        emit_mod(
            &mut res,
            to_ident("bslots"),
            emit_array_ids("BelSlotId", &dbs[0].bslot_id, true),
        );
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::bslots;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_mod(
                stream,
                to_ident("bslots"),
                emit_array_ids("BelSlotId", &adb.bslot_id, true),
            );
        }
    }

    if wire_uniform {
        emit_mod(
            &mut res,
            to_ident("wires"),
            emit_array_ids("WireSlotId", &dbs[0].wire_id, true),
        );
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::wires;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_mod(
                stream,
                to_ident("wires"),
                emit_array_ids("WireSlotId", &adb.wire_id, true),
            );
        }
    }

    if table_uniform {
        emit_tables(&mut res, &dbs[0]);
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("use super::tables;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_tables(stream, adb);
        }
    }

    if dbs.len() == 1 {
        emit_init(&mut res, &dbs[0].db);
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_init(stream, &adb.db);
        }
    }

    if devdata_uniform {
        emit_mod(
            &mut res,
            to_ident("devdata"),
            emit_ids("DeviceDataId", &dbs[0].devdata_id),
        );
        if dbs.len() != 1 {
            for stream in &mut variant_outs {
                stream.extend(TokenStream::from_str("pub use super::devdata;").unwrap());
            }
        }
    } else {
        for (adb, stream) in dbs.iter().zip(variant_outs.iter_mut()) {
            emit_mod(
                stream,
                to_ident("devdata"),
                emit_ids("DeviceDataId", &adb.devdata_id),
            );
        }
    }

    for (adb, stream) in dbs.iter().zip(variant_outs) {
        if !stream.is_empty() {
            emit_mod(&mut res, adb.name.clone().unwrap(), stream);
        }
    }
    res
}
