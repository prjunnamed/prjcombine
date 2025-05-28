use std::ops::{Deref, DerefMut};

use prjcombine_re_fpga_hammer::Collector;
use prjcombine_re_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use prjcombine_types::bsdata::{DbValue, TileItem};
use prjcombine_xilinx_bitstream::Bitstream;
use unnamed_entity::EntityId;

pub struct CollectorCtx<'a, 'b>
where
    'b: 'a,
{
    pub collector: Collector<'b>,
    pub device: &'a Device,
    pub db: &'a GeomDb,
    pub edev: &'a ExpandedDevice<'a>,
    pub empty_bs: &'a Bitstream,
}

impl<'b> Deref for CollectorCtx<'_, 'b> {
    type Target = Collector<'b>;

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
    pub fn insert_device_data(&mut self, key: impl Into<String>, val: impl Into<DbValue>) {
        self.collector
            .tiledb
            .insert_device_data(&self.device.name, key, val);
    }

    pub fn extract_inv(&mut self, tile: &str, bel: &str, pin: &str) -> TileItem {
        let pininv = format!("{pin}INV");
        let pin_b = format!("{pin}_B");
        self.extract_enum_bool(tile, bel, &pininv, pin, &pin_b)
    }

    pub fn collect_inv(&mut self, tile: &str, bel: &str, pin: &str) {
        let item = self.extract_inv(tile, bel, pin);
        self.tiledb.insert(tile, bel, format!("INV.{pin}"), item);
    }

    pub fn has_tile(&self, tile: &str) -> bool {
        let egrid = self.edev.egrid();
        let node = egrid.db.get_tile_class(tile);
        !egrid.tile_index[node].is_empty()
    }

    pub fn insert_int_inv(
        &mut self,
        int_tiles: &[&str],
        tile: &str,
        bel: &str,
        pin: &str,
        mut item: TileItem,
    ) {
        let intdb = self.edev.egrid().db;
        let slot = intdb.bel_slots.get(bel).unwrap();
        let node = intdb.tile_classes.get(tile).unwrap().1;
        let bel = &node.bels[slot];
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        assert_eq!(item.bits.len(), 1);
        let bit = &mut item.bits[0];
        assert_eq!(wire.0.to_idx(), bit.tile);
        bit.tile = 0;
        let wire_name = intdb.wires.key(wire.1);
        self.tiledb.insert(
            int_tiles[wire.0.to_idx()],
            "INT",
            format!("INV.{wire_name}"),
            item,
        );
    }

    pub fn item_int_inv(&self, int_tiles: &[&str], tile: &str, bel: &str, pin: &str) -> TileItem {
        let intdb = self.edev.egrid().db;
        let slot = intdb.bel_slots.get(bel).unwrap();
        let node = intdb.tile_classes.get(tile).unwrap().1;
        let bel = &node.bels[slot];
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        let wire_name = intdb.wires.key(wire.1);
        let mut item = self
            .tiledb
            .item(
                int_tiles[wire.0.to_idx()],
                "INT",
                &format!("INV.{wire_name}"),
            )
            .clone();
        assert_eq!(item.bits.len(), 1);
        let bit = &mut item.bits[0];
        bit.tile = wire.0.to_idx();
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
