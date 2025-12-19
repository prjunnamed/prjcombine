use std::collections::BTreeMap;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPad};

use crate::{
    bond::CfgPad,
    chip::{Chip, ChipKind},
};

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub io: BTreeMap<EdgeIoCoord, BScanPad>,
    pub cfg: BTreeMap<CfgPad, BScanPad>,
    pub upd: usize,
}

impl Chip {
    pub fn get_bscan(&self) -> BScan {
        if self.kind.is_xc3000() || self.kind == ChipKind::Xc2000 {
            panic!("no boundary scan on XC2000/XC3000");
        }
        let iobs: &[_] = if self.kind == ChipKind::Xc5200 {
            &[3, 2, 1, 0]
        } else if self.kind == ChipKind::Xc4000H {
            &[0, 1, 2, 3]
        } else {
            &[0, 1]
        };
        let mut io = BTreeMap::new();
        let mut cfg = BTreeMap::new();
        let mut builder = BScanBuilder::new();
        if self.kind.is_xc4000() {
            cfg.insert(CfgPad::Tdo, builder.get_to());
        }
        // top edge, right-to-left
        for col in self.columns() {
            if col == self.col_w() || col == self.col_e() {
                continue;
            }
            for iob in iobs.iter().copied() {
                let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                if self.unbonded_io.contains(&crd) {
                    continue;
                }
                io.insert(crd, builder.get_toi());
            }
        }
        // left edge, top-to-bottom
        for row in self.rows().rev() {
            if row == self.row_s() || row == self.row_n() {
                continue;
            }
            for iob in iobs.iter().copied() {
                let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                if self.unbonded_io.contains(&crd) {
                    continue;
                }
                io.insert(crd, builder.get_toi());
            }
        }
        if self.kind.is_xc4000() {
            cfg.insert(CfgPad::M1, builder.get_toi());
            cfg.insert(CfgPad::M0, builder.get_i());
            if self.kind != ChipKind::SpartanXl {
                cfg.insert(CfgPad::M2, builder.get_i());
            }
        }
        // bottom edge, left-to-right
        for col in self.columns().rev() {
            if col == self.col_w() || col == self.col_e() {
                continue;
            }
            for iob in iobs.iter().copied().rev() {
                let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                if self.unbonded_io.contains(&crd) {
                    continue;
                }
                io.insert(crd, builder.get_toi());
            }
        }
        // right edge, bottom-to-top
        for row in self.rows() {
            if row == self.row_s() || row == self.row_n() {
                continue;
            }
            for iob in iobs.iter().copied().rev() {
                let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                if self.unbonded_io.contains(&crd) {
                    continue;
                }
                io.insert(crd, builder.get_toi());
            }
        }
        let upd = builder.bits;
        builder.bits += 1;
        BScan {
            bits: builder.bits,
            io,
            cfg,
            upd,
        }
    }
}
