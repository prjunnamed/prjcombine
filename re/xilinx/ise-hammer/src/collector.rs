use std::ops::{Deref, DerefMut};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::db::{
    BelInfo, BelInput, BelInputId, BelSlotId, TileClassId, TileWireCoord,
};
use prjcombine_re_collector::{collect::Collector, diff::xlat_bit_bi};
use prjcombine_re_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use prjcombine_types::bsdata::{BitRectId, DbValue, PolTileBit, TileItem};
use prjcombine_xilinx_bitstream::Bitstream;

pub struct CollectorCtx<'a, 'b>
where
    'b: 'a,
{
    pub collector: Collector<'b, 'a>,
    pub device: &'a Device,
    pub db: &'a GeomDb,
    pub edev: &'a ExpandedDevice<'a>,
    pub empty_bs: &'a Bitstream,
}

impl<'a, 'b> Deref for CollectorCtx<'a, 'b> {
    type Target = Collector<'b, 'a>;

    fn deref(&self) -> &Self::Target {
        &self.collector
    }
}

impl DerefMut for CollectorCtx<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.collector
    }
}

impl<'a, 'b: 'a> CollectorCtx<'a, 'b> {
    pub fn insert_legacy(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        item: TileItem,
    ) {
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn item_legacy(&self, tile: &str, bel: &str, attr: &str) -> &TileItem {
        self.data.bsdata.item(tile, bel, attr)
    }

    pub fn insert_misc_data_legacy(&mut self, key: impl Into<String>, val: impl Into<DbValue>) {
        self.collector.data.bsdata.insert_misc_data(key, val);
    }

    pub fn insert_device_data_legacy(&mut self, key: impl Into<String>, val: impl Into<DbValue>) {
        self.collector
            .data
            .bsdata
            .insert_device_data(&self.device.name, key, val);
    }

    pub fn extract_inv_legacy(&mut self, tile: &str, bel: &str, pin: &str) -> TileItem {
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        self.extract_bit_bi_legacy(tile, bel, &pininv, pin, &pin_b)
    }

    pub fn collect_inv_legacy(&mut self, tile: &str, bel: &str, pin: &str) {
        let item = self.extract_inv_legacy(tile, bel, pin);
        self.insert_legacy(tile, bel, format!("INV.{pin}"), item);
    }

    pub fn has_tcls(&self, tcid: TileClassId) -> bool {
        !self.edev.tile_index[tcid].is_empty()
    }

    pub fn has_tile_legacy(&self, tile: &str) -> bool {
        let tcid = self.edev.db.get_tile_class(tile);
        !self.edev.tile_index[tcid].is_empty()
    }

    pub fn insert_int_inv_wire(
        &mut self,
        int_tiles: &[TileClassId],
        wire: TileWireCoord,
        mut bit: PolTileBit,
    ) {
        assert_eq!(wire.cell.to_idx(), bit.bit.rect.to_idx());
        bit.bit.rect = BitRectId::from_idx(0);
        let int_tcid = int_tiles[wire.cell.to_idx()];
        self.insert_inv(int_tcid, TileWireCoord::new_idx(0, wire.wire), bit);
    }

    pub fn insert_int_inv_legacy(
        &mut self,
        int_tiles: &[TileClassId],
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: &str,
        bit: PolTileBit,
    ) {
        let intdb = self.edev.db;
        let tcls = &intdb[tcid];
        let bel = &tcls.bels[bslot];
        let BelInfo::Legacy(bel) = bel else {
            unreachable!()
        };
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        self.insert_int_inv_wire(int_tiles, wire, bit);
    }

    pub fn item_int_inv_legacy(
        &self,
        int_tiles: &[TileClassId],
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: &str,
    ) -> PolTileBit {
        let intdb = self.edev.db;
        let tcls = &intdb[tcid];
        let bel = &tcls.bels[bslot];
        let BelInfo::Legacy(bel) = bel else {
            unreachable!()
        };
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        let int_tcid = int_tiles[wire.cell.to_idx()];
        let mut bit = self.sb_inv(int_tcid, TileWireCoord::new_idx(0, wire.wire));
        bit.bit.rect = BitRectId::from_idx(wire.cell.to_idx());
        bit
    }

    pub fn item_int_inv(
        &self,
        int_tiles: &[TileClassId],
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: BelInputId,
    ) -> PolTileBit {
        let intdb = self.edev.db;
        let tcls = &intdb[tcid];
        let bel = &tcls.bels[bslot];
        let BelInfo::Bel(bel) = bel else {
            unreachable!()
        };
        let BelInput::Fixed(wire) = bel.inputs[pin] else {
            unreachable!()
        };
        let int_tcid = int_tiles[wire.cell.to_idx()];
        let mut bit = self.sb_inv(int_tcid, TileWireCoord::new_idx(0, wire.wire));
        bit.bit.rect = BitRectId::from_idx(wire.cell.to_idx());
        bit
    }

    pub fn collect_int_inv_legacy(
        &mut self,
        int_tiles: &[TileClassId],
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: &str,
        flip: bool,
    ) {
        let intdb = self.edev.db;
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        let item = self.extract_bit_bi_legacy(
            intdb.tile_classes.key(tcid),
            intdb.bel_slots.key(bslot),
            &pininv,
            if flip { &pin_b } else { pin },
            if flip { pin } else { &pin_b },
        );
        self.insert_int_inv_legacy(int_tiles, tcid, bslot, pin, item.as_bit());
    }

    pub fn insert_bel_input_inv_int(
        &mut self,
        int_tiles: &[TileClassId],
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: BelInputId,
        mut bit: PolTileBit,
    ) {
        let intdb = self.edev.db;
        let tcls = &intdb[tcid];
        let bel = &tcls.bels[bslot];
        let BelInfo::Bel(bel) = bel else {
            unreachable!()
        };
        let BelInput::Fixed(wire) = bel.inputs[pin] else {
            unreachable!()
        };
        bit.inv ^= wire.inv;
        self.insert_int_inv_wire(int_tiles, wire.tw, bit);
    }

    pub fn collect_bel_input_inv_int_bi(
        &mut self,
        int_tiles: &[TileClassId],
        tcid: TileClassId,
        bslot: BelSlotId,
        pin: BelInputId,
    ) {
        let diff0 = self.get_diff_bel_input_inv(tcid, bslot, pin, false);
        let diff1 = self.get_diff_bel_input_inv(tcid, bslot, pin, true);
        let bit = xlat_bit_bi(diff0, diff1);
        self.insert_bel_input_inv_int(int_tiles, tcid, bslot, pin, bit);
    }
}
