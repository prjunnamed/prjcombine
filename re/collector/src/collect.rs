use std::collections::{BTreeMap, hash_map};

use prjcombine_interconnect::db::{
    BelAttribute, BelAttributeEnum, BelAttributeId, BelAttributeType, BelInputId, BelKind,
    BelSlotId, ConnectorSlotId, DeviceDataId, EnumValueId, IntDb, PolTileWireCoord, TableFieldId,
    TableId, TableRowId, TableValue, TileClassId, TileWireCoord,
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{EnumData, PolTileBit},
};

use crate::{
    bitdata::CollectorData,
    diff::{
        Diff, DiffKey, OcdMode, SpecialId, xlat_bit, xlat_bit_bi, xlat_bitvec, xlat_bitvec_sparse,
        xlat_enum_attr, xlat_enum_raw,
    },
};

#[derive(Debug)]
pub struct Collector<'a, 'b> {
    pub diffs: &'a mut BTreeMap<DiffKey, Vec<Diff>>,
    pub intdb: &'b IntDb,
    pub dev_name: &'b str,
    pub data: &'a mut CollectorData,
}

impl<'a, 'b> Collector<'a, 'b> {
    pub fn new(
        diffs: &'a mut BTreeMap<DiffKey, Vec<Diff>>,
        data: &'a mut CollectorData,
        dev_name: &'b str,
        intdb: &'b IntDb,
    ) -> Self {
        Self {
            diffs,
            intdb,
            dev_name,
            data,
        }
    }
}

/// Functions to get diffs
impl Collector<'_, '_> {
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

    pub fn peek_diffs_raw(&self, key: &DiffKey) -> &Vec<Diff> {
        self.diffs
            .get(key)
            .unwrap_or_else(|| panic!("NO DIFF: {key:?}"))
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

    pub fn get_diff_attr_bitvec(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: BitVec,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrBitVec(tcid, bslot, attr, val))
    }

    pub fn get_diff_bel_attr_special(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        spec: SpecialId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrSpecial(tcid, bslot, attr, spec))
    }

    pub fn get_diff_attr_bool(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: bool,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrEnumBool(tcid, bslot, attr, val))
    }

    pub fn get_diff_bel_special(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelSpecial(tcid, bslot, spec))
    }

    pub fn get_diff_bel_special_row(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
        row: TableRowId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelSpecialRow(tcid, bslot, spec, row))
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
}

/// Functions to insert bitdata
impl Collector<'_, '_> {
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

    pub fn insert_delay(&mut self, tcid: TileClassId, dst: TileWireCoord, items: EnumData<usize>) {
        match self.data.sb_delay.entry((tcid, dst)) {
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

    pub fn insert_bipass(
        &mut self,
        tcid: TileClassId,
        a: TileWireCoord,
        b: TileWireCoord,
        bit: PolTileBit,
    ) {
        match self.data.sb_bipass.entry((tcid, a, b)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bit);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bit);
            }
        }
    }

    pub fn insert_inv(&mut self, tcid: TileClassId, wire: TileWireCoord, bit: PolTileBit) {
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

    pub fn insert_tmux_group(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        data: EnumData<Option<usize>>,
    ) {
        match self.data.tmux_group.entry((tcid, bslot)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), data);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(data);
            }
        }
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

    pub fn insert_devdata_bitvec(&mut self, ddid: DeviceDataId, val: BitVec) {
        let val = TableValue::BitVec(val);
        let devdata = self
            .data
            .device_data
            .entry(self.dev_name.to_string())
            .or_default();
        if devdata.contains_id(ddid) {
            assert_eq!(devdata[ddid], val);
        } else {
            devdata.insert(ddid, val);
        }
    }


    pub fn insert_devdata_enum(&mut self, ddid: DeviceDataId, val: EnumValueId) {
        let val = TableValue::Enum(val);
        let devdata = self
            .data
            .device_data
            .entry(self.dev_name.to_string())
            .or_default();
        if devdata.contains_id(ddid) {
            assert_eq!(devdata[ddid], val);
        } else {
            devdata.insert(ddid, val);
        }
    }
}

/// Functions that grab bitdata
impl Collector<'_, '_> {
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

    pub fn sb_mux(
        &self,
        tcid: TileClassId,
        dst: TileWireCoord,
    ) -> &EnumData<Option<PolTileWireCoord>> {
        &self.data.sb_mux[&(tcid, dst)]
    }

    pub fn sb_inv(&self, tcid: TileClassId, dst: TileWireCoord) -> PolTileBit {
        self.data.sb_inv[&(tcid, dst)]
    }
}

/// Extract functions (get_diff + xlat)
impl Collector<'_, '_> {
    pub fn extract_bel_special_bitvec(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
        width: usize,
    ) -> Vec<PolTileBit> {
        xlat_bitvec(
            (0..width)
                .map(|idx| self.get_diff_raw(&DiffKey::BelSpecialBit(tcid, bslot, spec, idx)))
                .collect(),
        )
    }
}

/// Full-service collect functions (get_diff + xlat + insert)
impl Collector<'_, '_> {
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
            BelAttributeType::Bool => BelAttribute::BitVec(vec![xlat_bit(
                self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, aid, 0)),
            )]),
            BelAttributeType::BitVec(width) => BelAttribute::BitVec(xlat_bitvec(
                (0..width)
                    .map(|idx| self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, aid, idx)))
                    .collect(),
            )),
            BelAttributeType::BitVecArray(_, _) => todo!(),
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

    pub fn collect_bel_attr_bool_bi(
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
        let bit = xlat_bit_bi(diff0, diff1);
        assert!(matches!(
            bcattr.typ,
            BelAttributeType::Bool | BelAttributeType::BitVec(1)
        ));
        self.insert_bel_attr_bool(tcid, bslot, aid, bit);
    }

    pub fn collect_bel_attr_sparse(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        vals: impl IntoIterator<Item = u32>,
    ) {
        let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
            unreachable!()
        };
        let bcattr = &self.intdb.bel_classes[bcid].attributes[aid];
        let BelAttributeType::BitVec(width) = bcattr.typ else {
            unreachable!()
        };
        let mut diffs = vec![];
        for val in vals {
            let mut bv = BitVec::repeat(false, width);
            for i in 0..width {
                bv.set(i, (val & 1 << i) != 0);
            }
            diffs.push((bv.clone(), self.get_diff_attr_bitvec(tcid, bslot, aid, bv)));
        }
        let bits = xlat_bitvec_sparse(diffs);
        assert_eq!(bits.len(), width);
        self.insert_bel_attr_bitvec(tcid, bslot, aid, bits);
    }

    pub fn collect_bel_input_inv(&mut self, tcid: TileClassId, bslot: BelSlotId, pin: BelInputId) {
        let diff = self.get_diff_bel_input_inv(tcid, bslot, pin, true);
        let bit = xlat_bit(diff);
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
        let bit = xlat_bit_bi(diff0, diff1);
        self.insert_bel_input_inv(tcid, bslot, pin, bit);
    }

    pub fn collect_progbuf(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: PolTileWireCoord,
    ) {
        let diff = self.get_diff_raw(&DiffKey::Routing(tcid, dst, src));
        let bit = xlat_bit(diff);
        self.insert_progbuf(tcid, dst, src, bit);
    }

    pub fn collect_pass(&mut self, tcid: TileClassId, dst: TileWireCoord, src: TileWireCoord) {
        let diff = self.get_diff_raw(&DiffKey::Routing(tcid, dst, src.pos()));
        let bit = xlat_bit(diff);
        self.insert_pass(tcid, dst, src, bit);
    }

    pub fn collect_bipass(&mut self, tcid: TileClassId, a: TileWireCoord, b: TileWireCoord) {
        let diff_a = self.get_diff_raw(&DiffKey::Routing(tcid, a, b.pos()));
        let diff_b = self.get_diff_raw(&DiffKey::Routing(tcid, b, a.pos()));
        assert_eq!(diff_a, diff_b);
        let bit = xlat_bit(diff_a);
        self.insert_bipass(tcid, a, b, bit);
    }

    pub fn collect_inv(&mut self, tcid: TileClassId, wire: TileWireCoord) {
        let diff = self.get_diff_raw(&DiffKey::RoutingInv(tcid, wire, true));
        let bit = xlat_bit(diff);
        self.insert_inv(tcid, wire, bit);
    }

    pub fn collect_inv_bi(&mut self, tcid: TileClassId, wire: TileWireCoord) {
        let diff0 = self.get_diff_raw(&DiffKey::RoutingInv(tcid, wire, false));
        let diff1 = self.get_diff_raw(&DiffKey::RoutingInv(tcid, wire, true));
        let bit = xlat_bit_bi(diff0, diff1);
        self.insert_inv(tcid, wire, bit);
    }

    pub fn collect_bidi(&mut self, tcid: TileClassId, conn: ConnectorSlotId, wire: TileWireCoord) {
        let diff0 = self.get_diff_raw(&DiffKey::RoutingBidi(tcid, conn, wire, false));
        let diff1 = self.get_diff_raw(&DiffKey::RoutingBidi(tcid, conn, wire, true));
        let bit = xlat_bit_bi(diff0, diff1);
        self.insert_bidi(tcid, conn, wire, bit);
    }

    pub fn collect_delay(&mut self, tcid: TileClassId, wire: TileWireCoord, num: usize) {
        let mut diffs = vec![];
        for i in 0..num {
            let diff = self.get_diff_raw(&DiffKey::ProgDelay(tcid, wire, i));
            diffs.push((i, diff));
        }
        self.insert_delay(tcid, wire, xlat_enum_raw(diffs, OcdMode::ValueOrder));
    }
}
