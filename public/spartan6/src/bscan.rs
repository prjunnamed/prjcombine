use std::collections::BTreeMap;

use prjcombine_interconnect::grid::{EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPin};
use unnamed_entity::EntityId;

use crate::{
    bond::CfgPin,
    chip::{Chip, ColumnIoKind},
};

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub io: BTreeMap<EdgeIoCoord, BScanPin>,
    pub cfg: BTreeMap<CfgPin, BScanPin>,
}

impl Chip {
    pub fn get_bscan(&self) -> BScan {
        let mut io = BTreeMap::new();
        let mut cfg = BTreeMap::new();
        let mut builder = BScanBuilder::new();
        // TIO
        for (col, &cd) in self.columns.iter().rev() {
            if cd.tio == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // inner
                (0, cd.tio == ColumnIoKind::Outer),
                (1, cd.tio == ColumnIoKind::Outer),
                // outer
                (2, cd.tio == ColumnIoKind::Inner),
                (3, cd.tio == ColumnIoKind::Inner),
            ] {
                if !unused {
                    let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                    io.insert(crd, builder.get_toi());
                }
            }
        }
        // LIO
        for (row, &rd) in self.rows.iter().rev() {
            if rd.lio {
                for iob in [0, 1] {
                    let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                    io.insert(crd, builder.get_toi());
                }
            }
        }
        cfg.insert(CfgPin::ProgB, builder.get_toi());
        // BIO
        for (col, &cd) in &self.columns {
            if cd.bio == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // inner
                (0, cd.bio == ColumnIoKind::Outer),
                (1, cd.bio == ColumnIoKind::Outer),
                // outer
                (2, cd.bio == ColumnIoKind::Inner),
                (3, cd.bio == ColumnIoKind::Inner),
            ] {
                if !unused {
                    let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                    io.insert(crd, builder.get_toi());
                }
            }
        }
        cfg.insert(CfgPin::Done, builder.get_toi());
        cfg.insert(CfgPin::CmpCsB, builder.get_toi());
        cfg.insert(CfgPin::Suspend, builder.get_toi());
        // RIO
        for (row, &rd) in &self.rows {
            if rd.rio {
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
