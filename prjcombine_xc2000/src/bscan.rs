use std::collections::BTreeMap;

use prjcombine_int::grid::{EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPin};
use unnamed_entity::EntityId;

use crate::{
    bond::CfgPin,
    grid::{Grid, GridKind},
};

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub io: BTreeMap<EdgeIoCoord, BScanPin>,
    pub cfg: BTreeMap<CfgPin, BScanPin>,
    pub upd: usize,
}

impl Grid {
    pub fn get_bscan(&self) -> BScan {
        if self.kind.is_xc3000() || self.kind == GridKind::Xc2000 {
            panic!("no boundary scan on XC2000/XC3000");
        }
        let iobs: &[_] = if self.kind == GridKind::Xc5200 {
            &[3, 2, 1, 0]
        } else if self.kind == GridKind::Xc4000H {
            &[0, 1, 2, 3]
        } else {
            &[0, 1]
        };
        let mut io = BTreeMap::new();
        let mut cfg = BTreeMap::new();
        let mut builder = BScanBuilder::new();
        if self.kind.is_xc4000() {
            cfg.insert(CfgPin::Tdo, builder.get_to());
        }
        // top edge, right-to-left
        for col in self.columns() {
            if col == self.col_lio() || col == self.col_rio() {
                continue;
            }
            for iob in iobs.iter().copied() {
                let crd = EdgeIoCoord::T(col, TileIobId::from_idx(iob));
                if self.unbonded_io.contains(&crd) {
                    continue;
                }
                io.insert(crd, builder.get_toi());
            }
        }
        // left edge, top-to-bottom
        for row in self.rows().rev() {
            if row == self.row_bio() || row == self.row_tio() {
                continue;
            }
            for iob in iobs.iter().copied() {
                let crd = EdgeIoCoord::R(row, TileIobId::from_idx(iob));
                if self.unbonded_io.contains(&crd) {
                    continue;
                }
                io.insert(crd, builder.get_toi());
            }
        }
        if self.kind.is_xc4000() {
            cfg.insert(CfgPin::M1, builder.get_toi());
            cfg.insert(CfgPin::M0, builder.get_i());
            if self.kind != GridKind::SpartanXl {
                cfg.insert(CfgPin::M2, builder.get_i());
            }
        }
        // bottom edge, left-to-right
        for col in self.columns().rev() {
            if col == self.col_lio() || col == self.col_rio() {
                continue;
            }
            for iob in iobs.iter().copied().rev() {
                let crd = EdgeIoCoord::B(col, TileIobId::from_idx(iob));
                if self.unbonded_io.contains(&crd) {
                    continue;
                }
                io.insert(crd, builder.get_toi());
            }
        }
        // right edge, bottom-to-top
        for row in self.rows() {
            if row == self.row_bio() || row == self.row_tio() {
                continue;
            }
            for iob in iobs.iter().copied().rev() {
                let crd = EdgeIoCoord::L(row, TileIobId::from_idx(iob));
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
