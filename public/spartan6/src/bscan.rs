use std::collections::BTreeMap;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPad};

use crate::{
    bond::CfgPad,
    chip::{Chip, ColumnIoKind},
};

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub io: BTreeMap<EdgeIoCoord, BScanPad>,
    pub cfg: BTreeMap<CfgPad, BScanPad>,
}

impl Chip {
    pub fn get_bscan(&self) -> BScan {
        let mut io = BTreeMap::new();
        let mut cfg = BTreeMap::new();
        let mut builder = BScanBuilder::new();
        // TIO
        for (col, &cd) in self.columns.iter().rev() {
            if cd.io_n == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // inner
                (0, cd.io_n == ColumnIoKind::Outer),
                (1, cd.io_n == ColumnIoKind::Outer),
                // outer
                (2, cd.io_n == ColumnIoKind::Inner),
                (3, cd.io_n == ColumnIoKind::Inner),
            ] {
                if !unused {
                    let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                    io.insert(crd, builder.get_toi());
                }
            }
        }
        // LIO
        for (row, &rd) in self.rows.iter().rev() {
            if rd.io_w {
                for iob in [0, 1] {
                    let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                    io.insert(crd, builder.get_toi());
                }
            }
        }
        cfg.insert(CfgPad::ProgB, builder.get_toi());
        // BIO
        for (col, &cd) in &self.columns {
            if cd.io_s == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // inner
                (0, cd.io_s == ColumnIoKind::Outer),
                (1, cd.io_s == ColumnIoKind::Outer),
                // outer
                (2, cd.io_s == ColumnIoKind::Inner),
                (3, cd.io_s == ColumnIoKind::Inner),
            ] {
                if !unused {
                    let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                    io.insert(crd, builder.get_toi());
                }
            }
        }
        cfg.insert(CfgPad::Done, builder.get_toi());
        cfg.insert(CfgPad::CmpCsB, builder.get_toi());
        cfg.insert(CfgPad::Suspend, builder.get_toi());
        // RIO
        for (row, &rd) in &self.rows {
            if rd.io_e {
                for iob in [0, 1] {
                    let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                    io.insert(crd, builder.get_toi());
                }
            }
        }
        BScan {
            bits: builder.bits,
            io,
            cfg,
        }
    }
}
