use std::collections::{BTreeMap, BTreeSet, HashMap, hash_map};

use prjcombine_interconnect::db::{
    BelAttribute, BelAttributeEnum, BelAttributeId, BelAttributeType, BelInfo, BelInputId, BelKind,
    BelSlotId, ConnectorSlotId, DeviceDataId, EnumValueId, IntDb, Mux, PolTileWireCoord,
    SwitchBoxItem, TableFieldId, TableId, TableRowId, TableValue, TileClassId, TileWireCoord,
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{EnumData, PolTileBit},
};

use crate::{
    bitdata::CollectorData,
    diff::{
        Diff, DiffKey, OcdMode, SpecialId, xlat_bit, xlat_bit_bi, xlat_bit_bi_default, xlat_bitvec,
        xlat_bitvec_sparse, xlat_enum_attr, xlat_enum_attr_ocd, xlat_enum_raw,
    },
};

#[derive(Debug)]
pub struct Collector<'a, 'b> {
    pub diffs: &'a mut BTreeMap<DiffKey, Vec<Diff>>,
    pub intdb: &'b IntDb,
    pub dev_name: &'b str,
    pub data: &'a mut CollectorData,
    mux_index: HashMap<(TileClassId, TileWireCoord), &'b Mux>,
}

impl<'a, 'b> Collector<'a, 'b> {
    pub fn new(
        diffs: &'a mut BTreeMap<DiffKey, Vec<Diff>>,
        data: &'a mut CollectorData,
        dev_name: &'b str,
        intdb: &'b IntDb,
    ) -> Self {
        let mut mux_index = HashMap::new();
        for (tcid, _, tcls) in &intdb.tile_classes {
            for bel in tcls.bels.values() {
                let BelInfo::SwitchBox(sb) = bel else {
                    continue;
                };
                for item in &sb.items {
                    let SwitchBoxItem::Mux(mux) = item else {
                        continue;
                    };
                    mux_index.insert((tcid, mux.dst), mux);
                }
            }
        }
        Self {
            diffs,
            intdb,
            dev_name,
            data,
            mux_index,
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

    pub fn peek_diff_routing(
        &self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: PolTileWireCoord,
    ) -> &Diff {
        self.peek_diff_raw(&DiffKey::Routing(tcid, dst, src))
    }

    pub fn get_diff_routing(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: PolTileWireCoord,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::Routing(tcid, dst, src))
    }

    pub fn get_diff_routing_special(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        spec: SpecialId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::RoutingSpecial(tcid, dst, spec))
    }

    pub fn get_diff_routing_pair_special(
        &mut self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: PolTileWireCoord,
        spec: SpecialId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::RoutingPairSpecial(tcid, dst, src, spec))
    }

    pub fn get_diffs_attr_bits(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        bits: usize,
    ) -> Vec<Diff> {
        (0..bits)
            .map(|idx| self.get_diff_attr_bit(tcid, bslot, attr, idx))
            .collect()
    }

    pub fn get_diffs_bel_special_bits(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
        bits: usize,
    ) -> Vec<Diff> {
        (0..bits)
            .map(|idx| self.get_diff_bel_special_bit(tcid, bslot, spec, idx))
            .collect()
    }

    pub fn get_diffs_attr_special_bits(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        spec: SpecialId,
        bits: usize,
    ) -> Vec<Diff> {
        (0..bits)
            .map(|idx| self.get_diff_attr_special_bit(tcid, bslot, attr, spec, idx))
            .collect()
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

    pub fn get_diff_attr_special_bit_bi(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        spec: SpecialId,
        bit: usize,
        val: bool,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrSpecialBit(
            tcid, bslot, attr, spec, bit, val,
        ))
    }

    pub fn get_diff_attr_special_bit(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        spec: SpecialId,
        bit: usize,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrSpecialBit(
            tcid, bslot, attr, spec, bit, true,
        ))
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
        self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, attr, bit, true))
    }

    pub fn get_diff_attr_bit_bi(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        bit: usize,
        val: bool,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, attr, bit, val))
    }

    pub fn get_diff_attr_bool(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, attr, 0, true))
    }

    pub fn get_diff_attr_bool_bi(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: bool,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, attr, 0, val))
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

    pub fn get_diff_attr_u32(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: u32,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrU32(tcid, bslot, attr, val))
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

    pub fn get_diff_attr_special_val(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        spec: SpecialId,
        val: EnumValueId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrSpecialValue(tcid, bslot, attr, spec, val))
    }

    pub fn get_diff_bel_attr_row(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        row: TableRowId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelAttrRow(tcid, bslot, attr, row))
    }

    pub fn get_diff_bel_special(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelSpecial(tcid, bslot, spec))
    }

    pub fn get_diff_bel_special_special(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec1: SpecialId,
        spec2: SpecialId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelSpecialSpecial(tcid, bslot, spec1, spec2))
    }

    pub fn get_diff_bel_special_bit(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
        bidx: usize,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelSpecialBit(tcid, bslot, spec, bidx))
    }

    pub fn get_diff_bel_special_u32(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
        val: u32,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelSpecialU32(tcid, bslot, spec, val))
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

    pub fn get_diff_bel_sss_row(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec0: SpecialId,
        spec1: SpecialId,
        spec2: SpecialId,
        row: TableRowId,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelSpecialSpecialSpecialRow(
            tcid, bslot, spec0, spec1, spec2, row,
        ))
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

    pub fn get_diff_bel_input_inv_special(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: BelInputId,
        spec: SpecialId,
        val: bool,
    ) -> Diff {
        self.get_diff_raw(&DiffKey::BelInputInvSpecial(tcid, bslot, pin, spec, val))
    }

    pub fn peek_diff_attr_bit(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        bit: usize,
    ) -> &Diff {
        self.peek_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, attr, bit, true))
    }

    pub fn peek_diff_attr_val(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        attr: BelAttributeId,
        val: EnumValueId,
    ) -> &Diff {
        self.peek_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, attr, val))
    }

    pub fn peek_diff_bel_special(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
    ) -> &Diff {
        self.peek_diff_raw(&DiffKey::BelSpecial(tcid, bslot, spec))
    }

    pub fn peek_diff_bel_special_row(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec: SpecialId,
        row: TableRowId,
    ) -> &Diff {
        self.peek_diff_raw(&DiffKey::BelSpecialRow(tcid, bslot, spec, row))
    }

    pub fn peek_diff_bel_sss_row(
        &self,
        tcid: TileClassId,
        bslot: BelSlotId,
        spec0: SpecialId,
        spec1: SpecialId,
        spec2: SpecialId,
        row: TableRowId,
    ) -> &Diff {
        self.peek_diff_raw(&DiffKey::BelSpecialSpecialSpecialRow(
            tcid, bslot, spec0, spec1, spec2, row,
        ))
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

    pub fn insert_bel_attr_enum(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        attr: BelAttributeEnum,
    ) {
        self.insert_bel_attr_raw(tcid, bslot, aid, BelAttribute::Enum(attr));
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

    pub fn insert_pairmux(
        &mut self,
        tcid: TileClassId,
        dst: [TileWireCoord; 2],
        items: EnumData<[Option<PolTileWireCoord>; 2]>,
    ) {
        match self.data.sb_pairmux.entry((tcid, dst)) {
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

    pub fn insert_support(
        &mut self,
        tcid: TileClassId,
        wires: BTreeSet<TileWireCoord>,
        bits: Vec<PolTileBit>,
    ) {
        match self.data.sb_support.entry((tcid, wires)) {
            hash_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bits);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(bits);
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

    pub fn insert_devdata_bool(&mut self, ddid: DeviceDataId, val: bool) {
        let val = TableValue::BitVec(BitVec::from_iter([val]));
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

    pub fn insert_devdata_u32(&mut self, ddid: DeviceDataId, val: u32) {
        let val = TableValue::U32(val);
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

    pub fn sb_progbuf(
        &self,
        tcid: TileClassId,
        dst: TileWireCoord,
        src: PolTileWireCoord,
    ) -> PolTileBit {
        self.data.sb_buf[&(tcid, dst, src)]
    }

    pub fn sb_mux(
        &self,
        tcid: TileClassId,
        dst: TileWireCoord,
    ) -> &EnumData<Option<PolTileWireCoord>> {
        &self.data.sb_mux[&(tcid, dst)]
    }

    pub fn sb_pairmux(
        &self,
        tcid: TileClassId,
        dst: [TileWireCoord; 2],
    ) -> &EnumData<[Option<PolTileWireCoord>; 2]> {
        &self.data.sb_pairmux[&(tcid, dst)]
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
                BelAttribute::Enum(xlat_enum_attr(diffs))
            }
            BelAttributeType::Bool => {
                BelAttribute::BitVec(vec![xlat_bit(self.get_diff_attr_bool(tcid, bslot, aid))])
            }
            BelAttributeType::BitVec(width) => BelAttribute::BitVec(xlat_bitvec(
                (0..width)
                    .map(|idx| self.get_diff_attr_bit(tcid, bslot, aid, idx))
                    .collect(),
            )),
            BelAttributeType::BitVecArray(width, height) => BelAttribute::BitVec(xlat_bitvec(
                (0..(width * height))
                    .map(|idx| self.get_diff_attr_bit(tcid, bslot, aid, idx))
                    .collect(),
            )),
            BelAttributeType::U32 => unreachable!(),
        };
        self.insert_bel_attr_raw(tcid, bslot, aid, attr);
    }

    pub fn collect_bel_attr_ocd(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        ocd: OcdMode,
    ) {
        let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
            unreachable!()
        };
        let bcattr = &self.intdb.bel_classes[bcid].attributes[aid];
        let BelAttributeType::Enum(ecid) = bcattr.typ else {
            unreachable!()
        };

        let ecls = &self.intdb.enum_classes[ecid];
        let mut diffs = vec![];
        for vid in ecls.values.ids() {
            diffs.push((
                vid,
                self.get_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, aid, vid)),
            ));
        }

        let attr = xlat_enum_attr_ocd(diffs, ocd);
        self.insert_bel_attr_enum(tcid, bslot, aid, attr);
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
        self.insert_bel_attr_enum(tcid, bslot, aid, attr);
    }

    pub fn collect_bel_attr_subset_default_ocd(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        vals: &[EnumValueId],
        default: EnumValueId,
        ocd: OcdMode,
    ) {
        let mut diffs = vec![];
        for &vid in vals {
            diffs.push((
                vid,
                self.get_diff_raw(&DiffKey::BelAttrValue(tcid, bslot, aid, vid)),
            ));
        }
        diffs.push((default, Diff::default()));

        let attr = xlat_enum_attr_ocd(diffs, ocd);
        self.insert_bel_attr_enum(tcid, bslot, aid, attr);
    }

    pub fn collect_bel_attr_default_ocd(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        default: EnumValueId,
        ocd: OcdMode,
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

        let attr = xlat_enum_attr_ocd(diffs, ocd);
        self.insert_bel_attr_enum(tcid, bslot, aid, attr);
    }

    pub fn collect_bel_attr_default(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
        default: EnumValueId,
    ) {
        self.collect_bel_attr_default_ocd(tcid, bslot, aid, default, OcdMode::ValueOrder);
    }

    pub fn collect_bel_attr_bi(
        &mut self,
        tcid: TileClassId,
        bslot: BelSlotId,
        aid: BelAttributeId,
    ) -> BitVec {
        let BelKind::Class(bcid) = self.intdb.bel_slots[bslot].kind else {
            unreachable!()
        };
        let bcattr = &self.intdb.bel_classes[bcid].attributes[aid];
        let width = match bcattr.typ {
            BelAttributeType::Bool => 1,
            BelAttributeType::BitVec(width) => width,
            _ => unreachable!(),
        };
        let mut res = vec![];
        let mut default = BitVec::new();
        for i in 0..width {
            let diff0 = self.get_diff_attr_bit_bi(tcid, bslot, aid, i, false);
            let diff1 = self.get_diff_attr_bit_bi(tcid, bslot, aid, i, true);
            let (bit, def) = xlat_bit_bi_default(diff0, diff1);
            res.push(bit);
            default.push(def);
        }
        self.insert_bel_attr_bitvec(tcid, bslot, aid, res);
        default
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

    pub fn collect_mux_ocd(&mut self, tcid: TileClassId, dst: TileWireCoord, ocd: OcdMode) {
        let mux = self.mux_index[&(tcid, dst)];
        let mut diffs = vec![];
        let mut got_empty = false;
        for &src in mux.src.keys() {
            let diff = self.get_diff_routing(tcid, dst, src);
            if diff.bits.is_empty() {
                got_empty = true;
            }
            diffs.push((Some(src), diff));
        }
        if !got_empty {
            diffs.push((None, Diff::default()));
        }
        self.insert_mux(tcid, dst, xlat_enum_raw(diffs, ocd));
    }

    pub fn collect_mux(&mut self, tcid: TileClassId, dst: TileWireCoord) {
        self.collect_mux_ocd(tcid, dst, OcdMode::Mux);
    }
}
