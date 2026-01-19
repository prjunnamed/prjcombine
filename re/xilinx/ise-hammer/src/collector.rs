use std::ops::{Deref, DerefMut};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::db::{BelInfo, TileClassId};
use prjcombine_re_fpga_hammer::Collector;
use prjcombine_re_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use prjcombine_types::bsdata::{BitRectId, DbValue, TileItem};
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
    pub fn insert(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        item: TileItem,
    ) {
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn item(&self, tile: &str, bel: &str, attr: &str) -> &TileItem {
        self.data.bsdata.item(tile, bel, attr)
    }

    pub fn insert_misc_data(&mut self, key: impl Into<String>, val: impl Into<DbValue>) {
        self.collector.data.bsdata.insert_misc_data(key, val);
    }

    pub fn insert_device_data(&mut self, key: impl Into<String>, val: impl Into<DbValue>) {
        self.collector
            .data
            .bsdata
            .insert_device_data(&self.device.name, key, val);
    }

    pub fn extract_inv(&mut self, tile: &str, bel: &str, pin: &str) -> TileItem {
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        self.extract_enum_bool(tile, bel, &pininv, pin, &pin_b)
    }

    pub fn collect_inv(&mut self, tile: &str, bel: &str, pin: &str) {
        let item = self.extract_inv(tile, bel, pin);
        self.insert(tile, bel, format!("INV.{pin}"), item);
    }

    pub fn has_tile_id(&self, tcid: TileClassId) -> bool {
        !self.edev.tile_index[tcid].is_empty()
    }

    pub fn has_tile(&self, tile: &str) -> bool {
        let tcid = self.edev.db.get_tile_class(tile);
        !self.edev.tile_index[tcid].is_empty()
    }

    fn int_sb(&self, tcname: &str) -> &'a str {
        let intdb = self.edev.db;
        let int_tcls = intdb.tile_classes.get(tcname).unwrap().1;
        let int_sb = int_tcls
            .bels
            .iter()
            .find(|(_, bel)| matches!(bel, BelInfo::SwitchBox(_)))
            .unwrap()
            .0;
        intdb.bel_slots.key(int_sb)
    }

    pub fn insert_int_inv(
        &mut self,
        int_tiles: &[&str],
        tile: &str,
        bel: &str,
        pin: &str,
        mut item: TileItem,
    ) {
        let intdb = self.edev.db;
        let slot = intdb.bel_slots.get(bel).unwrap().0;
        let tcls = intdb.tile_classes.get(tile).unwrap().1;
        let bel = &tcls.bels[slot];
        let BelInfo::Legacy(bel) = bel else {
            unreachable!()
        };
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        assert_eq!(item.bits.len(), 1);
        let bit = &mut item.bits[0];
        assert_eq!(wire.cell.to_idx(), bit.rect.to_idx());
        bit.rect = BitRectId::from_idx(0);
        let wire_name = intdb.wires.key(wire.wire);
        let int_tcname = int_tiles[wire.cell.to_idx()];
        let int_sb = self.int_sb(int_tcname);
        self.insert(int_tcname, int_sb, format!("INV.{wire_name}"), item);
    }

    pub fn item_int_inv(&self, int_tiles: &[&str], tile: &str, bel: &str, pin: &str) -> TileItem {
        let intdb = self.edev.db;
        let slot = intdb.bel_slots.get(bel).unwrap().0;
        let tcls = intdb.tile_classes.get(tile).unwrap().1;
        let bel = &tcls.bels[slot];
        let BelInfo::Legacy(bel) = bel else {
            unreachable!()
        };
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        let wire_name = intdb.wires.key(wire.wire);
        let int_tcname = int_tiles[wire.cell.to_idx()];
        let int_sb = self.int_sb(int_tcname);
        let mut item = self
            .item(int_tcname, int_sb, &format!("INV.{wire_name}"))
            .clone();
        assert_eq!(item.bits.len(), 1);
        let bit = &mut item.bits[0];
        bit.rect = BitRectId::from_idx(wire.cell.to_idx());
        item
    }

    pub fn collect_int_inv(
        &mut self,
        int_tiles: &[&str],
        tile: &str,
        bel: &str,
        pin: &str,
        flip: bool,
    ) {
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        let item = self.extract_enum_bool(
            tile,
            bel,
            &pininv,
            if flip { &pin_b } else { pin },
            if flip { pin } else { &pin_b },
        );
        self.insert_int_inv(int_tiles, tile, bel, pin, item);
    }
}
