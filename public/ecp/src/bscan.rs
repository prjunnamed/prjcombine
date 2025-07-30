use std::collections::{BTreeMap, BTreeSet};

use prjcombine_interconnect::{
    dir::DirV,
    grid::{EdgeIoCoord, TileIobId},
};
use prjcombine_types::bscan::{BScanBuilder, BScanPad};
use unnamed_entity::EntityId;

use crate::{
    bond::{BondPad, CfgPad, SerdesPad},
    chip::{Chip, ChipKind, IoGroupKind, RowKind, SpecialIoKey, SpecialLocKey},
};

#[derive(Debug)]
pub struct BScan {
    pub bits: usize,
    pub pads: BTreeMap<BondPad, BScanPad>,
}

impl Chip {
    pub fn get_bscan(&self) -> BScan {
        let mut builder = BScanBuilder::new();
        let mut pads = BTreeMap::new();
        match self.kind {
            ChipKind::Ecp | ChipKind::Ecp2 | ChipKind::Ecp2M => {
                for (col, cd) in &self.columns {
                    if col < self.col_clk {
                        continue;
                    }
                    if cd.io_s == IoGroupKind::Serdes {
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(3)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(3)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(2)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(2)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::ClkP),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(1)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(1)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(0)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(0)),
                            builder.get_i(),
                        );
                    } else if cd.io_s != IoGroupKind::None {
                        for iob in [0, 1] {
                            let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                let mut got_cfg = false;
                if matches!(self.kind, ChipKind::Ecp2 | ChipKind::Ecp2M) {
                    pads.insert(BondPad::Cfg(CfgPad::M2), builder.get_i());
                    pads.insert(BondPad::Cfg(CfgPad::M1), builder.get_i());
                    pads.insert(BondPad::Cfg(CfgPad::M0), builder.get_i());
                    pads.insert(BondPad::Cfg(CfgPad::ProgB), builder.get_i());
                    pads.insert(BondPad::Cfg(CfgPad::Cclk), builder.get_tb());
                    pads.insert(BondPad::Cfg(CfgPad::InitB), builder.get_tb());
                    pads.insert(BondPad::Cfg(CfgPad::Done), builder.get_tb());
                    got_cfg = true;
                    if self.rows[self.row_s() + 2].io_e == IoGroupKind::None {
                        for pad in [
                            CfgPad::WriteN,
                            CfgPad::Cs1N,
                            CfgPad::CsN,
                            CfgPad::D(0),
                            CfgPad::D(1),
                            CfgPad::D(2),
                            CfgPad::D(3),
                            CfgPad::D(4),
                            CfgPad::D(5),
                            CfgPad::D(6),
                            CfgPad::D(7),
                            CfgPad::Di,
                            CfgPad::Dout,
                            CfgPad::Busy,
                        ] {
                            pads.insert(BondPad::Cfg(pad), builder.get_tb());
                        }
                    }
                }
                for (row, rd) in &self.rows {
                    if rd.kind == RowKind::Ebr && !got_cfg {
                        pads.insert(BondPad::Cfg(CfgPad::M2), builder.get_i());
                        pads.insert(BondPad::Cfg(CfgPad::M1), builder.get_i());
                        pads.insert(BondPad::Cfg(CfgPad::M0), builder.get_i());
                        pads.insert(BondPad::Cfg(CfgPad::ProgB), builder.get_i());
                        pads.insert(BondPad::Cfg(CfgPad::Cclk), builder.get_tb());
                        pads.insert(BondPad::Cfg(CfgPad::InitB), builder.get_tb());
                        pads.insert(BondPad::Cfg(CfgPad::Done), builder.get_tb());
                        got_cfg = true;
                    }
                    if rd.io_e != IoGroupKind::None {
                        for iob in [1, 0] {
                            let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (col, cd) in self.columns.iter().rev() {
                    if cd.io_n == IoGroupKind::Serdes {
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::InP(0)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::OutP(0)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::OutP(1)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::InP(1)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::ClkP),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::InP(2)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::OutP(2)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::OutP(3)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::N, col, SerdesPad::InP(3)),
                            builder.get_i(),
                        );
                    } else if cd.io_n != IoGroupKind::None {
                        for iob in [1, 0] {
                            let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (row, rd) in self.rows.iter().rev() {
                    if rd.io_w != IoGroupKind::None {
                        for iob in [0, 1] {
                            let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                if matches!(self.kind, ChipKind::Ecp2 | ChipKind::Ecp2M) {
                    pads.insert(BondPad::Cfg(CfgPad::Hfp), builder.get_i());
                }
                for (col, cd) in &self.columns {
                    if col >= self.col_clk {
                        continue;
                    }
                    if cd.io_s == IoGroupKind::Serdes {
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(3)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(3)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(2)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(2)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::ClkP),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(1)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(1)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(0)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(0)),
                            builder.get_i(),
                        );
                    } else if cd.io_s != IoGroupKind::None {
                        for iob in [0, 1] {
                            let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
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
                        IoGroupKind::None => [].as_slice(),
                        IoGroupKind::Double | IoGroupKind::DoubleDqs => [0, 1].as_slice(),
                        IoGroupKind::DoubleA => [0].as_slice(),
                        IoGroupKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                for (row, rd) in self.rows.iter().rev() {
                    let iobs = match rd.io_e {
                        IoGroupKind::None => [].as_slice(),
                        IoGroupKind::Double | IoGroupKind::DoubleDqs => [0, 1].as_slice(),
                        IoGroupKind::DoubleA => [0].as_slice(),
                        IoGroupKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                for (col, cd) in self.columns.iter().rev() {
                    let iobs = match cd.io_s {
                        IoGroupKind::None => [].as_slice(),
                        IoGroupKind::Double | IoGroupKind::DoubleDqs => [1, 0].as_slice(),
                        IoGroupKind::DoubleA => [0].as_slice(),
                        IoGroupKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                pads.insert(BondPad::Cfg(CfgPad::InitB), builder.get_tb());
                pads.insert(BondPad::Cfg(CfgPad::SleepB), builder.get_i());
                for (row, rd) in &self.rows {
                    let iobs = match rd.io_w {
                        IoGroupKind::None => [].as_slice(),
                        IoGroupKind::Double | IoGroupKind::DoubleDqs => [1, 0].as_slice(),
                        IoGroupKind::DoubleA => [0].as_slice(),
                        IoGroupKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                pads.insert(BondPad::Cfg(CfgPad::Cclk), builder.get_tb());
                pads.insert(BondPad::Cfg(CfgPad::ProgB), builder.get_i());
                pads.insert(BondPad::Cfg(CfgPad::Done), builder.get_tb());
                pads.insert(BondPad::Cfg(CfgPad::M1), builder.get_i());
                pads.insert(BondPad::Cfg(CfgPad::M0), builder.get_i());
                for (col, cd) in &self.columns {
                    if col >= self.col_clk {
                        continue;
                    }
                    let iobs = match cd.io_n {
                        IoGroupKind::None => [].as_slice(),
                        IoGroupKind::Double | IoGroupKind::DoubleDqs => [0, 1].as_slice(),
                        IoGroupKind::DoubleA => [0].as_slice(),
                        IoGroupKind::DoubleB => [1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &iob in iobs {
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
            }
            ChipKind::MachXo => {
                for (row, rd) in &self.rows {
                    let num_io = match rd.io_w {
                        IoGroupKind::None => 0,
                        IoGroupKind::Double => 2,
                        IoGroupKind::Quad => 4,
                        IoGroupKind::QuadReverse => {
                            for i in 0..4 {
                                let crd = EdgeIoCoord::W(row, TileIobId::from_idx(i));
                                pads.insert(BondPad::Io(crd), builder.get_tb());
                            }
                            continue;
                        }
                        _ => unreachable!(),
                    };
                    for i in (0..num_io).rev() {
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(i));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                for (col, cd) in &self.columns {
                    let num_io = match cd.io_n {
                        IoGroupKind::None => 0,
                        IoGroupKind::Quad => 4,
                        IoGroupKind::Hex => 6,
                        IoGroupKind::HexReverse => {
                            for i in (0..6).rev() {
                                let crd = EdgeIoCoord::N(col, TileIobId::from_idx(i));
                                pads.insert(BondPad::Io(crd), builder.get_tb());
                            }
                            continue;
                        }
                        _ => unreachable!(),
                    };
                    for i in 0..num_io {
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(i));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                for (row, rd) in self.rows.iter().rev() {
                    let num_io = match rd.io_e {
                        IoGroupKind::None => 0,
                        IoGroupKind::Double => 2,
                        IoGroupKind::Quad => 4,
                        _ => unreachable!(),
                    };
                    for i in 0..num_io {
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(i));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                for (col, cd) in self.columns.iter().rev() {
                    let num_io = match cd.io_s {
                        IoGroupKind::None => 0,
                        IoGroupKind::Quad => 4,
                        IoGroupKind::Hex => 6,
                        IoGroupKind::HexReverse => {
                            for i in 0..6 {
                                let crd = EdgeIoCoord::S(col, TileIobId::from_idx(i));
                                pads.insert(BondPad::Io(crd), builder.get_tb());
                            }
                            continue;
                        }
                        _ => unreachable!(),
                    };
                    for i in (0..num_io).rev() {
                        let crd = EdgeIoCoord::S(col, TileIobId::from_idx(i));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
            }
            ChipKind::Xp2 => {
                for (row, rd) in &self.rows {
                    if row < self.row_clk {
                        continue;
                    }
                    if rd.io_e != IoGroupKind::None {
                        for iob in [1, 0] {
                            let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (col, cd) in self.columns.iter().rev() {
                    if cd.io_n != IoGroupKind::None {
                        for iob in [1, 0] {
                            let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (row, rd) in self.rows.iter().rev() {
                    if row == self.row_clk {
                        pads.insert(BondPad::Cfg(CfgPad::Toe), builder.get_i());
                        pads.insert(BondPad::Cfg(CfgPad::M0), builder.get_i());
                    }
                    if rd.io_w != IoGroupKind::None {
                        for iob in [0, 1] {
                            let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (col, cd) in &self.columns {
                    if cd.io_s != IoGroupKind::None {
                        for iob in [0, 1] {
                            let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (row, rd) in &self.rows {
                    if row >= self.row_clk {
                        continue;
                    }
                    if rd.io_e != IoGroupKind::None {
                        for iob in [1, 0] {
                            let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                let row_cfg = self.special_loc[&SpecialLocKey::Config].row;
                let mut pll_rows = BTreeSet::new();
                for (&loc, &cell) in &self.special_loc {
                    if matches!(loc, SpecialLocKey::Pll(..)) {
                        pll_rows.insert(cell.row);
                    }
                }
                for (row, rd) in &self.rows {
                    if row < row_cfg {
                        continue;
                    }
                    if rd.io_e != IoGroupKind::None {
                        for iob in [3, 2, 1, 0] {
                            let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                pads.insert(BondPad::Cfg(CfgPad::M2), builder.get_i());
                pads.insert(BondPad::Cfg(CfgPad::M1), builder.get_i());
                pads.insert(BondPad::Cfg(CfgPad::M0), builder.get_i());
                pads.insert(BondPad::Cfg(CfgPad::ProgB), builder.get_i());
                pads.insert(BondPad::Cfg(CfgPad::Cclk), builder.get_tb());
                pads.insert(BondPad::Cfg(CfgPad::InitB), builder.get_tb());
                pads.insert(BondPad::Cfg(CfgPad::Done), builder.get_tb());
                for (col, cd) in self.columns.iter().rev() {
                    if cd.io_n != IoGroupKind::None {
                        for iob in [3, 2, 1, 0] {
                            let crd = EdgeIoCoord::N(col, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (row, rd) in self.rows.iter().rev() {
                    if rd.io_w != IoGroupKind::None {
                        for iob in [0, 1, 2, 3] {
                            let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                    if pll_rows.contains(&row) {
                        for iob in [4, 5, 6, 7] {
                            let crd = EdgeIoCoord::W(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (col, cd) in &self.columns {
                    if cd.io_s == IoGroupKind::Serdes {
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(3)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(3)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(2)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(2)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::ClkP),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(1)),
                            builder.get_i(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(1)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::OutP(0)),
                            builder.get_o(),
                        );
                        pads.insert(
                            BondPad::Serdes(DirV::S, col, SerdesPad::InP(0)),
                            builder.get_i(),
                        );
                    } else if cd.io_s != IoGroupKind::None {
                        for iob in [0, 1, 2, 3] {
                            let crd = EdgeIoCoord::S(col, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
                for (row, rd) in &self.rows {
                    if row >= row_cfg {
                        continue;
                    }
                    if pll_rows.contains(&row) {
                        for iob in [7, 6, 5, 4] {
                            let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                    if rd.io_e != IoGroupKind::None {
                        for iob in [3, 2, 1, 0] {
                            let crd = EdgeIoCoord::E(row, TileIobId::from_idx(iob));
                            pads.insert(BondPad::Io(crd), builder.get_tb());
                        }
                    }
                }
            }
            ChipKind::MachXo2(_) => {
                let skip_io = [
                    self.special_io[&SpecialIoKey::Tck],
                    self.special_io[&SpecialIoKey::Tms],
                    self.special_io[&SpecialIoKey::Tdi],
                    self.special_io[&SpecialIoKey::Tdo],
                ];
                for (row, rd) in self.rows.iter().rev() {
                    let num_io = match rd.io_w {
                        IoGroupKind::None => 0,
                        IoGroupKind::Double => 2,
                        IoGroupKind::Quad | IoGroupKind::QuadI3c => 4,
                        _ => unreachable!(),
                    };
                    for i in 0..num_io {
                        let crd = EdgeIoCoord::W(row, TileIobId::from_idx(i));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                for (col, cd) in &self.columns {
                    let iobs = match cd.io_s {
                        IoGroupKind::None => [].as_slice(),
                        IoGroupKind::Double => [0, 1].as_slice(),
                        IoGroupKind::Quad => [0, 1, 2, 3].as_slice(),
                        IoGroupKind::QuadReverse => [2, 3, 0, 1].as_slice(),
                        _ => unreachable!(),
                    };
                    for &i in iobs {
                        let crd = EdgeIoCoord::S(col, TileIobId::from_idx(i));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                for (row, rd) in &self.rows {
                    let num_io = match rd.io_e {
                        IoGroupKind::None => 0,
                        IoGroupKind::Double => 2,
                        IoGroupKind::Quad => 4,
                        _ => unreachable!(),
                    };
                    for i in (0..num_io).rev() {
                        let crd = EdgeIoCoord::E(row, TileIobId::from_idx(i));
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
                for (col, cd) in self.columns.iter().rev() {
                    let iobs = match cd.io_n {
                        IoGroupKind::None | IoGroupKind::Ebr => [].as_slice(),
                        IoGroupKind::Double => [1, 0].as_slice(),
                        IoGroupKind::Quad => [3, 2, 1, 0].as_slice(),
                        IoGroupKind::QuadReverse => [1, 0, 3, 2].as_slice(),
                        _ => unreachable!(),
                    };
                    for &i in iobs {
                        let crd = EdgeIoCoord::N(col, TileIobId::from_idx(i));
                        if skip_io.contains(&crd) {
                            continue;
                        }
                        pads.insert(BondPad::Io(crd), builder.get_tb());
                    }
                }
            }
        }
        BScan {
            bits: builder.bits,
            pads,
        }
    }
}
