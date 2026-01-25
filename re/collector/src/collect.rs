use core::ops::Range;
use std::collections::{BTreeMap, hash_map};

use prjcombine_interconnect::db::{
    BelAttribute, BelAttributeEnum, BelAttributeId, BelAttributeType, BelInputId, BelKind,
    BelSlotId, ConnectorSlotId, EnumValueId, IntDb, PolTileWireCoord, TableFieldId, TableId,
    TableRowId, TableValue, TileClassId, TileWireCoord,
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{PolTileBit, TileItem, TileItemKind},
};

use crate::{
    bitdata::CollectorData,
    diff::{
        Diff, DiffKey, EnumData, FeatureId, OcdMode, SpecialId, xlat_bit, xlat_bit_raw,
        xlat_bit_wide, xlat_bitvec, xlat_bitvec_raw, xlat_bool, xlat_bool_default, xlat_bool_raw,
        xlat_enum, xlat_enum_attr, xlat_enum_default, xlat_enum_default_ocd, xlat_enum_int,
        xlat_enum_ocd,
    },
};

#[derive(Debug)]
pub struct Collector<'a, 'b> {
    pub diffs: &'a mut BTreeMap<DiffKey, Vec<Diff>>,
    pub intdb: &'b IntDb,
    pub data: &'a mut CollectorData,
}

impl<'a, 'b> Collector<'a, 'b> {
    pub fn new(
        diffs: &'a mut BTreeMap<DiffKey, Vec<Diff>>,
        data: &'a mut CollectorData,
        intdb: &'b IntDb,
    ) -> Self {
        Self { diffs, intdb, data }
    }

    pub fn get_diffs_raw(&mut self, key: &DiffKey) -> Vec<Diff> {
        self.diffs
            .remove(key)
            .unwrap_or_else(|| panic!("NO DIFF: {key:?}"))
    }

    pub fn get_diff_raw(&mut self, key: &DiffKey) -> Diff {
        let mut res = self.get_diffs_raw(key);
        assert_eq!(res.len(), 1);
        res.pop().unwrap()
    }

    pub fn get_diffs(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Vec<Diff> {
        let tile = tile.into();
        let bel = bel.into();
        let attr = attr.into();
        let val = val.into();
        let key = DiffKey::Legacy(FeatureId {
            tile,
            bel,
            attr,
            val,
        });
        self.get_diffs_raw(&key)
    }

    pub fn get_diff(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Diff {
        let mut res = self.get_diffs(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        res.pop().unwrap()
    }

    pub fn peek_diffs(
        &self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> &Vec<Diff> {
        let tile = tile.into();
        let bel = bel.into();
        let attr = attr.into();
        let val = val.into();
        let key = DiffKey::Legacy(FeatureId {
            tile,
            bel,
            attr,
            val,
        });
        self.diffs
            .get(&key)
            .unwrap_or_else(|| panic!("NO DIFF: {key:?}"))
    }

    pub fn peek_diffs_raw(&self, key: &DiffKey) -> &Vec<Diff> {
        self.diffs
            .get(key)
            .unwrap_or_else(|| panic!("NO DIFF: {key:?}"))
    }

    pub fn peek_diff(
        &self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> &Diff {
        let res = self.peek_diffs(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        &res[0]
    }

    pub fn peek_diff_raw(&self, key: &DiffKey) -> &Diff {
        let res = self.peek_diffs_raw(key);
        assert_eq!(res.len(), 1);
        &res[0]
    }

    pub fn get_diff_attr_special(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        spec: SpecialId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrSpecial(tcid, bslot, attr, spec))
    }

    pub fn get_diff_attr_special_bit(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        spec: SpecialId,
        bit: usize,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrSpecialBit(tcid, bslot, attr, spec, bit))
    }

    pub fn get_diff_attr_val(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: EnumValueId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, attr, val))
    }

    pub fn get_diff_attr_bit(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        bit: usize,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, attr, bit))
    }

    pub fn get_diff_bel_special(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelSpecial(tcid, bslot, spec))
    }

    pub fn get_diff_bel_input_inv(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: BelInputId,
        val: bool,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelInputInv(tcid, bslot, pin, val))
    }

    pub fn insert_bel_attr_raw(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        attr: BelAttribute,
    ) {
        match self.data.bel_attrs.entry((tcid, bslot, aid)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), attr);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(attr);
            }
        }
    }

    pub fn insert_bel_attr_bool(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        bit: PolTileBit,
    ) {
        self.insert_bel_attr_raw(tcid, bslot, aid, BelAttribute::BitVec(vec![bit]));
    }

    pub fn insert_bel_attr_bitvec(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        bits: Vec<PolTileBit>,
    ) {
        self.insert_bel_attr_raw(tcid, bslot, aid, BelAttribute::BitVec(bits));
    }

    pub fn collect_bel_attr(&mut self, tcid: TileClassId, bslot: BelSlotId, aid: BelAttributeId) {
        let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
            unreachable!()
        };
        let bcattr = &self.intdb.bel_classes[bcid].attributes[aid];
        let attr = match bcattr.typ {
            BelAttributeType::Enum(ecid) => {
                let ecls = &self.intdb.enum_classes[ecid];
                let mut diffs = vec![];
                for vid in ecls.values.ids() {
                    diffs.push((
                        vid,
                        self.get_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, aid, vid)),
                    ));
                }
                xlat_enum_attr(diffs)
            }
            BelAttributeType::Bool => BelAttribute::BitVec(vec![xlat_bit_raw(
                self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, aid, 0)),
            )]),
            BelAttributeType::Bitvec(width) => BelAttribute::BitVec(xlat_bitvec_raw(
                (0..width)
                    .map(|idx| self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, aid, idx)))
                    .collect(),
            )),
            BelAttributeType::BitvecArray(_, _) => todo!(),
        };
        self.insert_bel_attr_raw(tcid, bslot, aid, attr);
    }

    pub fn collect_bel_attr_subset(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        vals: &[EnumValueId],
    ) {
        let mut diffs = vec![];
        for &vid in vals {
            diffs.push((
                vid,
                self.get_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, aid, vid)),
            ));
        }

        let attr = xlat_enum_attr(diffs);
        self.insert_bel_attr_raw(tcid, bslot, aid, attr);
    }

    pub fn collect_bel_attr_default(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        default: EnumValueId,
    ) {
        let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
            unreachable!()
        };
        let bcattr = &self.intdb.bel_classes[bcid].attributes[aid];
        let BelAttributeType::Enum(ecid) = bcattr.typ else {
            unreachable!()
        };

        let ecls = &self.intdb.enum_classes[ecid];
        let mut diffs = vec![(default, Diff::default())];
        for vid in ecls.values.ids() {
            if vid == default {
                continue;
            }
            diffs.push((
                vid,
                self.get_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, aid, vid)),
            ));
        }

        let attr = xlat_enum_attr(diffs);
        self.insert_bel_attr_raw(tcid, bslot, aid, attr);
    }

    pub fn collect_bel_attr_enum_bool(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
    ) {
        let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
            unreachable!()
        };
        let bcattr = &self.intdb.bel_classes[bcid].attributes[aid];
        let diff0 = self.get_diff_raw(&DiffKey::BelAttrEnumBool(tcid, bslot, aid, false));
        let diff1 = self.get_diff_raw(&DiffKey::BelAttrEnumBool(tcid, bslot, aid, true));
        let bit = xlat_bool_raw(diff0, diff1);
        assert!(matches!(
            bcattr.typ,
            BelAttributeType::Bool | BelAttributeType::Bitvec(1)
        ));
        let attr = BelAttribute::BitVec(vec![bit]);
        self.insert_bel_attr_raw(tcid, bslot, aid, attr);
    }

    pub fn insert_bel_input_inv(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: BelInputId,
        bit: PolTileBit,
    ) {
        match self.data.bel_input_inv.entry((tcid, bslot, pin)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn collect_bel_input_inv(&mut self, tcid: TileClassId, bslot: BelSlotId, pin: BelInputId) {
        let diff = self.get_diff_bel_input_inv(tcid, bslot, pin, true);
        let bit = xlat_bit_raw(diff);
        self.insert_bel_input_inv(tcid, bslot, pin, bit);
    }

    pub fn collect_bel_input_inv_bi(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: BelInputId,
    ) {
        let diff0 = self.get_diff_bel_input_inv(tcid, bslot, pin, false);
        let diff1 = self.get_diff_bel_input_inv(tcid, bslot, pin, true);
        let bit = xlat_bool_raw(diff0, diff1);
        self.insert_bel_input_inv(tcid, bslot, pin, bit);
    }

    pub fn bel_attr_raw(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
    ) -> &BelAttribute {
        &self.data.bel_attrs[&(tcid, bslot, aid)]
    }

    pub fn bel_attr_bitvec(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
    ) -> &[PolTileBit] {
        let BelAttribute::BitVec(ref bits) = self.data.bel_attrs[&(tcid, bslot, aid)] else {
            unreachable!()
        };
        bits
    }

    pub fn bel_attr_bit(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
    ) -> PolTileBit {
        let bits = self.bel_attr_bitvec(tcid, bslot, aid);
        assert_eq!(bits.len(), 1);
        bits[0]
    }

    pub fn bel_attr_enum(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
    ) -> &BelAttributeEnum {
        let BelAttribute::Enum(ref data) = self.data.bel_attrs[&(tcid, bslot, aid)] else {
            unreachable!()
        };
        data
    }

    pub fn bel_input_inv(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: BelInputId,
    ) -> PolTileBit {
        self.data.bel_input_inv[&(tcid, bslot, pin)]
    }

    pub fn insert_mux(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        items: EnumData<Option<PolTileWireCoord>>,
    ) {
        match self.data.sb_mux.entry((tcid, dst)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), items);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(items);
            }
        }
    }

    pub fn insert_progbuf(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: PolTileWireCoord,
        bit: PolTileBit,
    ) {
        match self.data.sb_buf.entry((tcid, dst, src)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn collect_progbuf(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: PolTileWireCoord,
    ) {
        let diff = self.get_diff_raw(&DiffKey::Routing(tcid, dst, src));
        let bit = xlat_bit_raw(diff);
        self.insert_progbuf(tcid, dst, src, bit);
    }

    pub fn insert_pass(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: TileWireCoord,
        bit: PolTileBit,
    ) {
        match self.data.sb_pass.entry((tcid, dst, src)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn collect_pass(&mut self, tcid: TileClassId, dst: TileWireCoord, src: TileWireCoord) {
        let diff = self.get_diff_raw(&DiffKey::Routing(tcid, dst, src.pos()));
        let bit = xlat_bit_raw(diff);
        self.insert_pass(tcid, dst, src, bit);
    }

    pub fn collect_bipass(&mut self, tcid: TileClassId, a: TileWireCoord, b: TileWireCoord) {
        let diff_a = self.get_diff_raw(&DiffKey::Routing(tcid, a, b.pos()));
        let diff_b = self.get_diff_raw(&DiffKey::Routing(tcid, b, a.pos()));
        assert_eq!(diff_a, diff_b);
        let bit = xlat_bit_raw(diff_a);
        match self.data.sb_bipass.entry((tcid, a, b)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn collect_inv(&mut self, tcid: TileClassId, wire: TileWireCoord) {
        let diff = self.get_diff_raw(&DiffKey::RoutingInv(tcid, wire, true));
        let bit = xlat_bit_raw(diff);
        match self.data.sb_inv.entry((tcid, wire)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn collect_inv_pair(&mut self, tcid: TileClassId, wire: TileWireCoord) {
        let diff0 = self.get_diff_raw(&DiffKey::RoutingInv(tcid, wire, false));
        let diff1 = self.get_diff_raw(&DiffKey::RoutingInv(tcid, wire, true));
        let bit = xlat_bool_raw(diff0, diff1);
        match self.data.sb_inv.entry((tcid, wire)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn insert_bidi(
        &mut self,
        tcid: TileClassId,
        conn: ConnectorSlotId,
        wire: TileWireCoord,
        bit: PolTileBit,
    ) {
        match self.data.sb_bidi.entry((tcid, conn, wire)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn collect_bidi(&mut self, tcid: TileClassId, conn: ConnectorSlotId, wire: TileWireCoord) {
        let diff0 = self.get_diff_raw(&DiffKey::RoutingBidi(tcid, conn, wire, false));
        let diff1 = self.get_diff_raw(&DiffKey::RoutingBidi(tcid, conn, wire, true));
        let bit = xlat_bool_raw(diff0, diff1);
        self.insert_bidi(tcid, conn, wire, bit);
    }

    pub fn insert_table_bitvec(
        &mut self,
        tid: TableId,
        rid: TableRowId,
        fid: TableFieldId,
        val: BitVec,
    ) {
        let val = TableValue::BitVec(val);
        match self.data.table_data.entry((tid, rid, fid)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), val);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(val);
            }
        }
    }

    #[must_use]
    pub fn extract_bitvec(&mut self, tile: &str, bel: &str, attr: &str, val: &str) -> TileItem {
        xlat_bitvec(self.get_diffs(tile, bel, attr, val))
    }

    pub fn collect_bitvec(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        let item = xlat_bitvec(self.get_diffs(tile, bel, attr, val));
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_enum(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.get_diff(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum(diffs)
    }

    #[must_use]
    pub fn extract_enum_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        ocd: OcdMode,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.get_diff(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_ocd(diffs, ocd)
    }

    #[must_use]
    pub fn extract_enum_int(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: Range<u32>,
        delta: u32,
    ) -> TileItem {
        let diffs = vals
            .map(|val| {
                (
                    val,
                    self.get_diff(tile, bel, attr, format!("{v}", v = val + delta)),
                )
            })
            .collect();
        xlat_enum_int(diffs)
    }

    pub fn collect_enum(&mut self, tile: &str, bel: &str, attr: &str, vals: &[impl AsRef<str>]) {
        let item = self.extract_enum(tile, bel, attr, vals);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        ocd: OcdMode,
    ) {
        let item = self.extract_enum_ocd(tile, bel, attr, vals, ocd);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_int(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: Range<u32>,
        delta: u32,
    ) {
        let item = self.extract_enum_int(tile, bel, attr, vals, delta);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_bit(&mut self, tile: &str, bel: &str, attr: &str, val: &str) -> TileItem {
        let diff = self.get_diff(tile, bel, attr, val);
        xlat_bit(diff)
    }

    #[must_use]
    pub fn extract_bit_wide(&mut self, tile: &str, bel: &str, attr: &str, val: &str) -> TileItem {
        let diff = self.get_diff(tile, bel, attr, val);
        xlat_bit_wide(diff)
    }

    pub fn collect_bit(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        let item = self.extract_bit(tile, bel, attr, val);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_bit_wide(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        let item = self.extract_bit_wide(tile, bel, attr, val);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_enum_default(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.get_diff(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_default(diffs, default)
    }

    #[must_use]
    pub fn extract_enum_default_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
        ocd: OcdMode,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.get_diff(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_default_ocd(diffs, default, ocd)
    }

    pub fn collect_enum_default(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
    ) {
        let item = self.extract_enum_default(tile, bel, attr, vals, default);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_default_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
        ocd: OcdMode,
    ) {
        let item = self.extract_enum_default_ocd(tile, bel, attr, vals, default, ocd);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_enum_bool_default(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> (TileItem, bool) {
        let d0 = self.get_diff(tile, bel, attr, val0);
        let d1 = self.get_diff(tile, bel, attr, val1);
        xlat_bool_default(d0, d1)
    }

    #[must_use]
    pub fn extract_enum_bool(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> TileItem {
        let d0 = self.get_diff(tile, bel, attr, val0);
        let d1 = self.get_diff(tile, bel, attr, val1);
        xlat_bool(d0, d1)
    }

    pub fn collect_enum_bool_default(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> bool {
        let (item, res) = self.extract_enum_bool_default(tile, bel, attr, val0, val1);
        self.data.bsdata.insert(tile, bel, attr, item);
        res
    }

    pub fn collect_enum_bool(&mut self, tile: &str, bel: &str, attr: &str, val0: &str, val1: &str) {
        let item = self.extract_enum_bool(tile, bel, attr, val0, val1);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    #[must_use]
    pub fn extract_enum_bool_wide(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> TileItem {
        let d0 = self.get_diff(tile, bel, attr, val0);
        let d1 = self.get_diff(tile, bel, attr, val1);
        let item = xlat_enum(vec![("0", d0), ("1", d1)]);
        let TileItemKind::Enum { values } = item.kind else {
            unreachable!()
        };
        let v0 = &values["0"];
        let v1 = &values["1"];
        let invert = if v1.all() && !v0.any() {
            false
        } else if v0.all() && !v1.any() {
            true
        } else {
            panic!("not a bool: {tile} {bel} {attr} {values:?}");
        };
        let invert = BitVec::from_iter(vec![invert; item.bits.len()]);
        TileItem {
            bits: item.bits,
            kind: TileItemKind::BitVec { invert },
        }
    }

    #[must_use]
    pub fn extract_enum_bool_wide_mixed(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> TileItem {
        let d0 = self.get_diff(tile, bel, attr, val0);
        let d1 = self.get_diff(tile, bel, attr, val1);
        let item = xlat_enum(vec![("0", d0), ("1", d1)]);
        let TileItemKind::Enum { values } = item.kind else {
            unreachable!()
        };
        let v0 = &values["0"];
        let v1 = &values["1"];
        for (b0, b1) in v0.iter().zip(v1) {
            assert_eq!(b0, !b1);
        }
        let invert = v0.clone();
        TileItem {
            bits: item.bits,
            kind: TileItemKind::BitVec { invert },
        }
    }

    pub fn collect_enum_bool_wide(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) {
        let item = self.extract_enum_bool_wide(tile, bel, attr, val0, val1);

        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_bool_wide_mixed(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) {
        let item = self.extract_enum_bool_wide_mixed(tile, bel, attr, val0, val1);

        self.data.bsdata.insert(tile, bel, attr, item);
    }
}
