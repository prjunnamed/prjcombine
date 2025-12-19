use std::collections::BTreeMap;

use prjcombine_interconnect::grid::{EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPad};
use prjcombine_entity::EntityId;

use crate::{bond::CfgPad, chip::Chip};

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub io: BTreeMap<EdgeIoCoord, BScanPad>,
    pub clk: BTreeMap<u32, BScanPad>,
    pub cfg: BTreeMap<CfgPad, BScanPad>,
}

impl Chip {
    pub fn get_bscan(&self) -> BScan {
        let mut builder = BScanBuilder::new();
        let mut io = BTreeMap::new();
        let mut cfg = BTreeMap::new();
        let mut clk = BTreeMap::new();
        for col in self.columns().rev() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == self.col_w() || col == self.col_e() {
                continue;
            }
            for iob in [1, 2] {
                let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                io.insert(crd, builder.get_toi());
            }
            if col == self.col_clk() {
                clk.insert(1, builder.get_i());
                clk.insert(0, builder.get_i());
            }
        }
        for row in self.rows().rev() {
            if row == self.row_s() || row == self.row_n() {
                continue;
            }
            for iob in [1, 2, 3] {
                let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                io.insert(crd, builder.get_toi());
            }
        }
        for pin in [CfgPad::M1, CfgPad::M0, CfgPad::M2] {
            cfg.insert(pin, builder.get_i());
        }
        for col in self.columns() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == self.col_w() || col == self.col_e() {
                continue;
            }
            if col == self.col_clk() {
                clk.insert(5, builder.get_i());
                clk.insert(4, builder.get_i());
            }
            for iob in [2, 1] {
                let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                io.insert(crd, builder.get_toi());
            }
        }
        cfg.insert(CfgPad::Done, builder.get_toi());
        cfg.insert(CfgPad::ProgB, builder.get_i());
        for row in self.rows() {
            if row == self.row_s() || row == self.row_n() {
                continue;
            }
            for iob in [3, 2, 1] {
                let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                io.insert(crd, builder.get_toi());
            }
        }
        cfg.insert(CfgPad::Cclk, builder.get_toi());
        BScan {
            bits: builder.bits,
            io,
            clk,
            cfg,
        }
    }
}
