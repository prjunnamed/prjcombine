use std::collections::BTreeMap;

use prjcombine_interconnect::grid::{EdgeIoCoord, TileIobId};
use prjcombine_types::bscan::{BScanBuilder, BScanPad};
use unnamed_entity::EntityId;

use crate::{
    bond::CfgPad,
    chip::{Chip, ChipKind, IoKind, RowKind},
};

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub io: BTreeMap<EdgeIoCoord, BScanPad>,
    pub cfg: BTreeMap<CfgPad, BScanPad>,
}

impl Chip {
    pub fn get_bscan(&self) -> BScan {
        let mut builder = BScanBuilder::new();
        let mut io = BTreeMap::new();
        let mut cfg = BTreeMap::new();
        match self.kind {
            ChipKind::Ecp => {
                for (col, cd) in &self.columns {
                    if col < self.col_clk {
                        continue;
                    }
                    if cd.io_s != IoKind::None {
                        for iob in [0, 1] {
                            let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                            io.insert(crd, builder.get_tb());
                        }
                    }
                }
                let mut got_cfg = false;
                for (row, rd) in &self.rows {
                    if rd.kind == RowKind::Ebr && !got_cfg {
                        cfg.insert(CfgPad::M2, builder.get_i());
                        cfg.insert(CfgPad::M1, builder.get_i());
                        cfg.insert(CfgPad::M0, builder.get_i());
                        cfg.insert(CfgPad::ProgB, builder.get_i());
                        cfg.insert(CfgPad::Cclk, builder.get_tb());
                        cfg.insert(CfgPad::InitB, builder.get_tb());
                        cfg.insert(CfgPad::Done, builder.get_tb());
                        got_cfg = true;
                    }
                    if rd.io_e != IoKind::None {
                        for iob in [1, 0] {
                            let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                            io.insert(crd, builder.get_tb());
                        }
                    }
                }
                for (col, cd) in self.columns.iter().rev() {
                    if cd.io_n != IoKind::None {
                        for iob in [1, 0] {
                            let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                            io.insert(crd, builder.get_tb());
                        }
                    }
                }
                for (row, rd) in self.rows.iter().rev() {
                    if rd.io_w != IoKind::None {
                        for iob in [0, 1] {
                            let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                            io.insert(crd, builder.get_tb());
                        }
                    }
                }
                for (col, cd) in &self.columns {
                    if col >= self.col_clk {
                        continue;
                    }
                    if cd.io_s != IoKind::None {
                        for iob in [0, 1] {
                            let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                            io.insert(crd, builder.get_tb());
                        }
                    }
                }
            }
            ChipKind::Xp => {
                for (col, cd) in &self.columns {
                    if col < self.col_clk {
                        continue;
                    }
                    let iobs = match cd.io_n {
                        IoKind::None => [].as_slice(),
                        IoKind::Double | IoKind::DoubleDqs => [0, 1].as_slice(),
                        IoKind::DoubleA => [0].as_slice(),
                        IoKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                        io.insert(crd, builder.get_tb());
                    }
                }
                for (row, rd) in self.rows.iter().rev() {
                    let iobs = match rd.io_e {
                        IoKind::None => [].as_slice(),
                        IoKind::Double | IoKind::DoubleDqs => [0, 1].as_slice(),
                        IoKind::DoubleA => [0].as_slice(),
                        IoKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                        io.insert(crd, builder.get_tb());
                    }
                }
                for (col, cd) in self.columns.iter().rev() {
                    let iobs = match cd.io_s {
                        IoKind::None => [].as_slice(),
                        IoKind::Double | IoKind::DoubleDqs => [1, 0].as_slice(),
                        IoKind::DoubleA => [0].as_slice(),
                        IoKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                        io.insert(crd, builder.get_tb());
                    }
                }
                cfg.insert(CfgPad::InitB, builder.get_tb());
                cfg.insert(CfgPad::SleepB, builder.get_i());
                for (row, rd) in &self.rows {
                    let iobs = match rd.io_w {
                        IoKind::None => [].as_slice(),
                        IoKind::Double | IoKind::DoubleDqs => [1, 0].as_slice(),
                        IoKind::DoubleA => [0].as_slice(),
                        IoKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                        io.insert(crd, builder.get_tb());
                    }
                }
                cfg.insert(CfgPad::Cclk, builder.get_tb());
                cfg.insert(CfgPad::ProgB, builder.get_i());
                cfg.insert(CfgPad::Done, builder.get_tb());
                cfg.insert(CfgPad::M1, builder.get_i());
                cfg.insert(CfgPad::M0, builder.get_i());
                for (col, cd) in &self.columns {
                    if col >= self.col_clk {
                        continue;
                    }
                    let iobs = match cd.io_n {
                        IoKind::None => [].as_slice(),
                        IoKind::Double | IoKind::DoubleDqs => [0, 1].as_slice(),
                        IoKind::DoubleA => [0].as_slice(),
                        IoKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                        io.insert(crd, builder.get_tb());
                    }
                }
            }
            ChipKind::MachXo => {
                for (row, rd) in &self.rows {
                    let num_io = match rd.io_w {
                        IoKind::None => 0,
                        IoKind::Double => 2,
                        IoKind::Quad => 4,
                        IoKind::QuadReverse => {
                            for i in 0..4 {
                                let crd = EdgeIoCoord::W(row, TileIobId::from_idx(i));
                                io.insert(crd, builder.get_tb());
                            }
                            continue;
                        }
                        _ => unreachable!(),
                    };
                    for i in (0..num_io).rev() {
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_tb());
                    }
                }
                for (col, cd) in &self.columns {
                    let num_io = match cd.io_n {
                        IoKind::None => 0,
                        IoKind::Quad => 4,
                        IoKind::Hex => 6,
                        IoKind::HexReverse => {
                            for i in (0..6).rev() {
                                let crd = EdgeIoCoord::N(col, TileIobId::from_idx(i));
                                io.insert(crd, builder.get_tb());
                            }
                            continue;
                        }
                        _ => unreachable!(),
                    };
                    for i in 0..num_io {
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_tb());
                    }
                }
                for (row, rd) in self.rows.iter().rev() {
                    let num_io = match rd.io_e {
                        IoKind::None => 0,
                        IoKind::Double => 2,
                        IoKind::Quad => 4,
                        _ => unreachable!(),
                    };
                    for i in 0..num_io {
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_tb());
                    }
                }
                for (col, cd) in self.columns.iter().rev() {
                    let num_io = match cd.io_s {
                        IoKind::None => 0,
                        IoKind::Quad => 4,
                        IoKind::Hex => 6,
                        IoKind::HexReverse => {
                            for i in 0..6 {
                                let crd = EdgeIoCoord::S(col, TileIobId::from_idx(i));
                                io.insert(crd, builder.get_tb());
                            }
                            continue;
                        }
                        _ => unreachable!(),
                    };
                    for i in (0..num_io).rev() {
                        let crd = EdgeIoCoord::S(col, TileIobId::from_idx(i));
                        io.insert(crd, builder.get_tb());
                    }
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
