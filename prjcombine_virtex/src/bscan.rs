use std::collections::BTreeMap;

use prjcombine_int::grid::{EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPin};
use unnamed_entity::EntityId;

use crate::{bond::CfgPin, grid::Grid};

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub io: BTreeMap<EdgeIoCoord, BScanPin>,
    pub clk: BTreeMap<u32, BScanPin>,
    pub cfg: BTreeMap<CfgPin, BScanPin>,
}

impl Grid {
    pub fn get_bscan(&self) -> BScan {
        let mut builder = BScanBuilder::new();
        let mut io = BTreeMap::new();
        let mut cfg = BTreeMap::new();
        let mut clk = BTreeMap::new();
        for col in self.columns().rev() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == self.col_lio() || col == self.col_rio() {
                continue;
            }
            for iob in [1, 2] {
                let crd = EdgeIoCoord::T(col, TileIobId::from_idx(iob));
                io.insert(crd, builder.get_toi());
            }
            if col == self.col_clk() {
                clk.insert(1, builder.get_i());
                clk.insert(0, builder.get_i());
            }
        }
        for row in self.rows().rev() {
            if row == self.row_bio() || row == self.row_tio() {
                continue;
            }
            for iob in [1, 2, 3] {
                let crd = EdgeIoCoord::L(row, TileIobId::from_idx(iob));
                io.insert(crd, builder.get_toi());
            }
        }
        for pin in [CfgPin::M1, CfgPin::M0, CfgPin::M2] {
            cfg.insert(pin, builder.get_i());
        }
        for col in self.columns() {
            if self.cols_bram.contains(&col) {
                continue;
            }
            if col == self.col_lio() || col == self.col_rio() {
                continue;
            }
            if col == self.col_clk() {
                clk.insert(5, builder.get_i());
                clk.insert(4, builder.get_i());
            }
            for iob in [2, 1] {
                let crd = EdgeIoCoord::B(col, TileIobId::from_idx(iob));
                io.insert(crd, builder.get_toi());
            }
        }
        cfg.insert(CfgPin::Done, builder.get_toi());
        cfg.insert(CfgPin::ProgB, builder.get_i());
        for row in self.rows() {
            if row == self.row_bio() || row == self.row_tio() {
                continue;
            }
            for iob in [3, 2, 1] {
                let crd = EdgeIoCoord::R(row, TileIobId::from_idx(iob));
                io.insert(crd, builder.get_toi());
            }
        }
        cfg.insert(CfgPin::Cclk, builder.get_toi());
        BScan {
            bits: builder.bits,
            io,
            clk,
            cfg,
        }
    }
}
