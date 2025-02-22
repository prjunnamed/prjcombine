use std::collections::BTreeMap;

use prjcombine_interconnect::grid::{EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPin};
use unnamed_entity::EntityId;

use crate::{
    bond::CfgPin,
    chip::{Chip, ChipKind, ColumnKind},
    iob::IobKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FcCfgPin {
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
    pub io: BTreeMap<EdgeIoCoord, BScanPin>,
    pub cfg: BTreeMap<CfgPin, BScanPin>,
    pub fc_cfg: BTreeMap<FcCfgPin, BScanPin>,
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
            cfg.insert(CfgPin::ProgB, builder.get_i());
            for row in self.rows.ids().rev() {
                if row != self.row_bot() && row != self.row_top() {
                    for i in (0..4).rev() {
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(4 + i));
                        io.insert(crd, builder.get_o());
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_i());
                    }
                }
            }
            for pin in [CfgPin::M1, CfgPin::M0, CfgPin::M2] {
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
                fc_cfg.insert(FcCfgPin::Dout(i), builder.get_o());
            }
            fc_cfg.insert(FcCfgPin::Init, builder.get_o());
            fc_cfg.insert(FcCfgPin::DoneO, builder.get_o());
            fc_cfg.insert(FcCfgPin::PorEnB, builder.get_i());
            for i in (0..8).rev() {
                fc_cfg.insert(FcCfgPin::Din(i), builder.get_i());
            }
            fc_cfg.insert(FcCfgPin::WriteB, builder.get_i());
            fc_cfg.insert(FcCfgPin::DoneI, builder.get_i());
            fc_cfg.insert(FcCfgPin::CsB, builder.get_i());
            fc_cfg.insert(FcCfgPin::Cclk, builder.get_i());
            for row in self.rows.ids() {
                if row != self.row_bot() && row != self.row_top() {
                    for i in 0..4 {
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(4 + i));
                        io.insert(crd, builder.get_o());
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_i());
                    }
                }
            }
        } else {
            for col in self.columns.ids().rev() {
                let row = self.row_top();
                if let Some((data, tidx)) = self.get_iob_data((col, row)) {
                    for &iob in data.iobs.iter().rev() {
                        if iob.tile == tidx {
                            let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob.bel.to_idx()));
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
                cfg.insert(CfgPin::HswapEn, builder.get_i());
            }
            cfg.insert(CfgPin::ProgB, builder.get_i());
            for row in self.rows.ids().rev() {
                let col = self.col_left();
                if let Some((data, tidx)) = self.get_iob_data((col, row)) {
                    for &iob in data.iobs.iter().rev() {
                        if iob.tile == tidx {
                            let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob.bel.to_idx()));
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
                for pin in [CfgPin::M1, CfgPin::M0, CfgPin::M2] {
                    cfg.insert(pin, builder.get_i());
                }
            }
            for col in self.columns.ids() {
                let row = self.row_bot();
                if let Some((data, tidx)) = self.get_iob_data((col, row)) {
                    for &iob in data.iobs.iter().rev() {
                        if iob.tile == tidx {
                            let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob.bel.to_idx()));
                            if iob.kind == IobKind::Iob {
                                io.insert(crd, builder.get_toi());
                            } else {
                                io.insert(crd, builder.get_i());
                            };
                        }
                    }
                }
            }
            cfg.insert(CfgPin::Done, builder.get_toi());
            if self.kind.is_virtex2() {
                cfg.insert(CfgPin::PwrdwnB, builder.get_i());
            }
            if !self.kind.is_spartan3ea() {
                cfg.insert(CfgPin::Cclk, builder.get_toi());
            }
            if self.kind.is_spartan3a() {
                cfg.insert(CfgPin::Suspend, builder.get_i());
            }
            for row in self.rows.ids() {
                let col = self.col_right();
                if let Some((data, tidx)) = self.get_iob_data((col, row)) {
                    for &iob in data.iobs.iter().rev() {
                        if iob.tile == tidx {
                            let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob.bel.to_idx()));
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
