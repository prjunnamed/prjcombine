use std::collections::BTreeMap;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{CellCoord, DieId, EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPad};

use crate::{
    bond::CfgPad,
    chip::{Chip, ChipKind, ColumnKind},
    iob::IobKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FcCfgPad {
    Init,
    DoneO,
    DoneI,
    PorEnB,
    Dout(u8),
    Din(u8),
    WriteB,
    CsB,
    Cclk,
}

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub io: BTreeMap<EdgeIoCoord, BScanPad>,
    pub cfg: BTreeMap<CfgPad, BScanPad>,
    pub fc_cfg: BTreeMap<FcCfgPad, BScanPad>,
}

impl Chip {
    pub fn get_bscan(&self) -> BScan {
        let mut io = BTreeMap::new();
        let mut cfg = BTreeMap::new();
        let mut fc_cfg = BTreeMap::new();
        let mut builder = BScanBuilder::new();
        if self.kind == ChipKind::FpgaCore {
            for col in self.columns.ids().rev() {
                if self.columns[col].kind == ColumnKind::Clb {
                    for i in 0..4 {
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(4 + i));
                        io.insert(crd, builder.get_o());
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_i());
                    }
                }
            }
            cfg.insert(CfgPad::ProgB, builder.get_i());
            for row in self.rows.ids().rev() {
                if row != self.row_s() && row != self.row_n() {
                    for i in (0..4).rev() {
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(4 + i));
                        io.insert(crd, builder.get_o());
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_i());
                    }
                }
            }
            for pin in [CfgPad::M1, CfgPad::M0, CfgPad::M2] {
                cfg.insert(pin, builder.get_i());
            }
            for col in self.columns.ids() {
                if self.columns[col].kind == ColumnKind::Clb {
                    for i in 0..4 {
                        let crd = EdgeIoCoord::S(col, TileIobId::from_idx(4 + i));
                        io.insert(crd, builder.get_o());
                        let crd = EdgeIoCoord::S(col, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_i());
                    }
                }
            }
            for i in (0..8).rev() {
                fc_cfg.insert(FcCfgPad::Dout(i), builder.get_o());
            }
            fc_cfg.insert(FcCfgPad::Init, builder.get_o());
            fc_cfg.insert(FcCfgPad::DoneO, builder.get_o());
            fc_cfg.insert(FcCfgPad::PorEnB, builder.get_i());
            for i in (0..8).rev() {
                fc_cfg.insert(FcCfgPad::Din(i), builder.get_i());
            }
            fc_cfg.insert(FcCfgPad::WriteB, builder.get_i());
            fc_cfg.insert(FcCfgPad::DoneI, builder.get_i());
            fc_cfg.insert(FcCfgPad::CsB, builder.get_i());
            fc_cfg.insert(FcCfgPad::Cclk, builder.get_i());
            for row in self.rows.ids() {
                if row != self.row_s() && row != self.row_n() {
                    for i in 0..4 {
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(4 + i));
                        io.insert(crd, builder.get_o());
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_i());
                    }
                }
            }
        } else {
            let die = DieId::from_idx(0);
            for col in self.columns.ids().rev() {
                let row = self.row_n();
                if let Some((data, tidx)) = self.get_iob_tile_data(CellCoord::new(die, col, row)) {
                    for &iob in data.iobs.iter().rev() {
                        if iob.cell == tidx {
                            let crd = EdgeIoCoord::N(col, iob.iob_id);
                            if iob.kind == IobKind::Iob {
                                io.insert(crd, builder.get_toi());
                            } else {
                                io.insert(crd, builder.get_i());
                            };
                        }
                    }
                }
            }
            if !self.kind.is_spartan3ea() {
                cfg.insert(CfgPad::HswapEn, builder.get_i());
            }
            cfg.insert(CfgPad::ProgB, builder.get_i());
            for row in self.rows.ids().rev() {
                let col = self.col_w();
                if let Some((data, tidx)) = self.get_iob_tile_data(CellCoord::new(die, col, row)) {
                    for &iob in data.iobs.iter().rev() {
                        if iob.cell == tidx {
                            let crd = EdgeIoCoord::W(row, iob.iob_id);
                            if iob.kind == IobKind::Iob {
                                io.insert(crd, builder.get_toi());
                            } else {
                                io.insert(crd, builder.get_i());
                            };
                        }
                    }
                }
            }
            if !self.kind.is_spartan3ea() {
                for pin in [CfgPad::M1, CfgPad::M0, CfgPad::M2] {
                    cfg.insert(pin, builder.get_i());
                }
            }
            for col in self.columns.ids() {
                let row = self.row_s();
                if let Some((data, tidx)) = self.get_iob_tile_data(CellCoord::new(die, col, row)) {
                    for &iob in data.iobs.iter().rev() {
                        if iob.cell == tidx {
                            let crd = EdgeIoCoord::S(col, iob.iob_id);
                            if iob.kind == IobKind::Iob {
                                io.insert(crd, builder.get_toi());
                            } else {
                                io.insert(crd, builder.get_i());
                            };
                        }
                    }
                }
            }
            cfg.insert(CfgPad::Done, builder.get_toi());
            if self.kind.is_virtex2() {
                cfg.insert(CfgPad::PwrdwnB, builder.get_i());
            }
            if !self.kind.is_spartan3ea() {
                cfg.insert(CfgPad::Cclk, builder.get_toi());
            }
            if self.kind.is_spartan3a() {
                cfg.insert(CfgPad::Suspend, builder.get_i());
            }
            for row in self.rows.ids() {
                let col = self.col_e();
                if let Some((data, tidx)) = self.get_iob_tile_data(CellCoord::new(die, col, row)) {
                    for &iob in data.iobs.iter().rev() {
                        if iob.cell == tidx {
                            let crd = EdgeIoCoord::E(row, iob.iob_id);
                            if iob.kind == IobKind::Iob {
                                io.insert(crd, builder.get_toi());
                            } else {
                                io.insert(crd, builder.get_i());
                            };
                        }
                    }
                }
            }
        }
        BScan {
            bits: builder.bits,
            io,
            cfg,
            fc_cfg,
        }
    }
}
