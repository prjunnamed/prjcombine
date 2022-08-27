use crate::pkg::{GtPin, SysMonPin};
use crate::{eint, int, ColId, RowId};
use prjcombine_entity::{EntityId, EntityVec};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_io: [ColId; 3],
    pub regs: usize,
    pub has_bot_sysmon: bool,
    pub has_top_sysmon: bool,
    pub regs_cfg_io: usize,
    pub ccm: usize,
    pub reg_cfg: usize,
    pub holes_ppc: Vec<(ColId, RowId)>,
    pub has_bram_fx: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Clb,
    Bram,
    Dsp,
    Io,
    Gt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    Data(u8), // ×32
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: ColId,
    pub row: RowId,
    pub bel: u32,
    pub ioc: u32,
    pub bank: u32,
    pub bbel: u32,
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.ioc;
        let y = self.row.to_idx() * 2 + self.bel as usize;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_cc(&self) -> bool {
        matches!(self.row.to_idx() % 16, 7 | 8)
    }
    pub fn is_lc(&self) -> bool {
        matches!(self.row.to_idx() % 16, 7 | 8) || self.ioc == 1
    }
    pub fn is_gc(&self) -> bool {
        matches!(self.bank, 3 | 4) || (matches!(self.bank, 1 | 2) && matches!(self.bbel, 18..=33))
    }
    pub fn is_vref(&self) -> bool {
        self.row.to_idx() % 8 == 4 && self.bel == 0
    }
    pub fn is_vr(&self) -> bool {
        match self.bank {
            1 => self.bbel / 2 == 18,
            2 => self.bbel / 2 == 23,
            3 => self.bbel / 2 == 2,
            4 => self.bbel / 2 == 7,
            _ => self.row.to_idx() % 32 == 9,
        }
    }
    pub fn get_cfg(&self) -> Option<SharedCfgPin> {
        if !matches!(self.bank, 1 | 2) {
            return None;
        }
        if self.bbel > 17 {
            return None;
        }
        if self.bank == 2 {
            Some(SharedCfgPin::Data(
                (self.row.to_idx() % 8 * 2 + self.bel as usize) as u8,
            ))
        } else {
            Some(SharedCfgPin::Data(
                (self.row.to_idx() % 8 * 2 + self.bel as usize + 16) as u8,
            ))
        }
    }
    pub fn sm_pair(&self, grid: &Grid) -> Option<(u32, u32)> {
        if grid.has_bot_sysmon {
            match (self.bank, self.row.to_idx() % 32) {
                (7, 0) => return Some((0, 1)),
                (7, 1) => return Some((0, 2)),
                (7, 2) => return Some((0, 3)),
                (7, 3) => return Some((0, 4)),
                (7, 5) => return Some((0, 5)),
                (7, 6) => return Some((0, 6)),
                (7, 7) => return Some((0, 7)),
                _ => (),
            }
        }
        if grid.has_top_sysmon {
            match (self.bank, self.row.to_idx() % 32) {
                (5, 24) => return Some((1, 1)),
                (5, 25) => return Some((1, 2)),
                (5, 26) => return Some((1, 3)),
                (5, 27) => return Some((1, 4)),
                (5, 29) => return Some((1, 5)),
                (5, 30) => return Some((1, 6)),
                (5, 31) => return Some((1, 7)),
                _ => (),
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub row: RowId,
    pub gtc: u32,
    pub bank: u32,
}

impl Gt {
    pub fn get_pads(&self, grid: &Grid) -> Vec<(String, String, GtPin, u32)> {
        let reg = self.row.to_idx() / 32;
        let (ipx, ipy);
        if grid.has_bot_sysmon {
            ipy = 2 + reg * 6;
            ipx = self.gtc * 2;
        } else {
            ipy = reg * 6;
            ipx = self.gtc;
        }
        let opy = reg * 4;
        let opx = self.gtc;
        vec![
            (
                format!("IPAD_X{}Y{}", ipx, ipy),
                format!("RXPPADB_{}", self.bank),
                GtPin::RxP,
                0,
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 1),
                format!("RXNPADB_{}", self.bank),
                GtPin::RxN,
                0,
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 2),
                format!("MGTCLK_N_{}", self.bank),
                GtPin::ClkN,
                0,
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 3),
                format!("MGTCLK_P_{}", self.bank),
                GtPin::ClkP,
                0,
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 4),
                format!("RXPPADA_{}", self.bank),
                GtPin::RxP,
                1,
            ),
            (
                format!("IPAD_X{}Y{}", ipx, ipy + 5),
                format!("RXNPADA_{}", self.bank),
                GtPin::RxN,
                1,
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy),
                format!("TXPPADB_{}", self.bank),
                GtPin::TxP,
                0,
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 1),
                format!("TXNPADB_{}", self.bank),
                GtPin::TxN,
                0,
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 2),
                format!("TXPPADA_{}", self.bank),
                GtPin::TxP,
                1,
            ),
            (
                format!("OPAD_X{}Y{}", opx, opy + 3),
                format!("TXNPADA_{}", self.bank),
                GtPin::TxN,
                1,
            ),
        ]
    }
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let lbanks: &[u32] = match self.regs {
            4 => &[7, 5],
            6 => &[7, 9, 5],
            8 => &[7, 11, 9, 5],
            10 => &[7, 11, 13, 9, 5],
            12 => &[7, 11, 15, 13, 9, 5],
            _ => unreachable!(),
        };
        let rbanks: &[u32] = match self.regs {
            4 => &[8, 6],
            6 => &[8, 10, 6],
            8 => &[8, 12, 10, 6],
            10 => &[8, 12, 14, 10, 6],
            12 => &[8, 12, 16, 14, 10, 6],
            _ => unreachable!(),
        };
        let mut res = Vec::new();
        // left column
        for (i, b) in lbanks.iter().copied().enumerate() {
            for j in 0..32 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[0],
                        row: RowId::from_idx(i * 32 + j),
                        ioc: 0,
                        bel: k,
                        bank: b,
                        bbel: (32 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // center column
        // bank 4
        let base = (self.reg_cfg - self.regs_cfg_io) * 16 - 8;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.cols_io[1],
                    row: RowId::from_idx(base + j),
                    ioc: 1,
                    bel: k,
                    bank: 4,
                    bbel: (8 - j as u32) * 2 + k,
                });
            }
        }
        // bank 2
        if self.regs_cfg_io > 1 {
            let base = (self.reg_cfg - self.regs_cfg_io) * 16;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1],
                        row: RowId::from_idx(base + j),
                        ioc: 1,
                        bel: k,
                        bank: 2,
                        bbel: (8 + 16 - (j as u32 ^ 8)) * 2 + k,
                    });
                }
            }
        }
        if self.regs_cfg_io > 2 {
            let base = self.reg_cfg * 16 - 32;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1],
                        row: RowId::from_idx(base + j),
                        ioc: 1,
                        bel: k,
                        bank: 2,
                        bbel: (24 + 16 - (j as u32 ^ 8)) * 2 + k,
                    });
                }
            }
        }
        let base = self.reg_cfg * 16 - 16;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.cols_io[1],
                    row: RowId::from_idx(base + j),
                    ioc: 1,
                    bel: k,
                    bank: 2,
                    bbel: (8 - j as u32) * 2 + k,
                });
            }
        }
        // bank 1
        let base = self.reg_cfg * 16 + 8;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.cols_io[1],
                    row: RowId::from_idx(base + j),
                    ioc: 1,
                    bel: k,
                    bank: 1,
                    bbel: (8 - j as u32) * 2 + k,
                });
            }
        }
        if self.regs_cfg_io > 2 {
            let base = self.reg_cfg * 16 + 16;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1],
                        row: RowId::from_idx(base + j),
                        ioc: 1,
                        bel: k,
                        bank: 1,
                        bbel: (24 + 16 - j as u32) * 2 + k,
                    });
                }
            }
        }
        if self.regs_cfg_io > 1 {
            let base = (self.reg_cfg + self.regs_cfg_io) * 16 - 16;
            for j in 0..16 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[1],
                        row: RowId::from_idx(base + j),
                        ioc: 1,
                        bel: k,
                        bank: 1,
                        bbel: (8 + 16 - j as u32) * 2 + k,
                    });
                }
            }
        }
        // bank 3
        let base = (self.reg_cfg + self.regs_cfg_io) * 16;
        for j in 0..8 {
            for k in 0..2 {
                res.push(Io {
                    col: self.cols_io[1],
                    row: RowId::from_idx(base + j),
                    ioc: 1,
                    bel: k,
                    bank: 3,
                    bbel: (8 - j as u32) * 2 + k,
                });
            }
        }
        // right column
        for (i, b) in rbanks.iter().copied().enumerate() {
            for j in 0..32 {
                for k in 0..2 {
                    res.push(Io {
                        col: self.cols_io[2],
                        row: RowId::from_idx(i * 32 + j),
                        ioc: 2,
                        bel: k,
                        bank: b,
                        bbel: (32 - j as u32) * 2 + k,
                    });
                }
            }
        }
        res
    }

    pub fn get_gt(&self) -> Vec<Gt> {
        let mut res = Vec::new();
        if *self.columns.first().unwrap() == ColumnKind::Gt {
            let lbanks: &[u32] = match self.regs {
                4 => &[105, 102],
                6 => &[105, 103, 102],
                8 => &[106, 105, 103, 102],
                10 => &[106, 105, 103, 102, 101],
                12 => &[106, 105, 104, 103, 102, 101],
                _ => unreachable!(),
            };
            for (i, b) in lbanks.iter().copied().enumerate() {
                res.push(Gt {
                    col: self.columns.first_id().unwrap(),
                    row: RowId::from_idx(i * 32),
                    gtc: 0,
                    bank: b,
                });
            }
        }
        if *self.columns.last().unwrap() == ColumnKind::Gt {
            let rbanks: &[u32] = match self.regs {
                4 => &[110, 113],
                6 => &[110, 112, 113],
                8 => &[109, 110, 112, 113],
                10 => &[109, 110, 112, 113, 114],
                12 => &[109, 110, 111, 112, 113, 114],
                _ => unreachable!(),
            };
            for (i, b) in rbanks.iter().copied().enumerate() {
                res.push(Gt {
                    col: self.columns.last_id().unwrap(),
                    row: RowId::from_idx(i * 32),
                    gtc: 1,
                    bank: b,
                });
            }
        }
        res
    }

    pub fn get_sysmon_pads(&self) -> Vec<(String, u32, SysMonPin)> {
        let mut res = Vec::new();
        let has_gt = *self.columns.first().unwrap() == ColumnKind::Gt;
        if has_gt {
            if self.has_bot_sysmon {
                res.push(("IPAD_X1Y0".to_string(), 0, SysMonPin::VP));
                res.push(("IPAD_X1Y1".to_string(), 0, SysMonPin::VN));
            }
            if self.has_top_sysmon {
                let ipy = self.regs * 3;
                res.push((format!("IPAD_X1Y{}", ipy), 1, SysMonPin::VP));
                res.push((format!("IPAD_X1Y{}", ipy + 1), 1, SysMonPin::VN));
            }
        } else {
            if self.has_bot_sysmon {
                res.push(("IPAD_X0Y0".to_string(), 0, SysMonPin::VP));
                res.push(("IPAD_X0Y1".to_string(), 0, SysMonPin::VN));
            }
            if self.has_top_sysmon {
                res.push(("IPAD_X0Y2".to_string(), 1, SysMonPin::VP));
                res.push(("IPAD_X0Y3".to_string(), 1, SysMonPin::VN));
            }
        }
        res
    }

    pub fn expand_grid<'a>(&self, db: &'a int::IntDb) -> eint::ExpandedGrid<'a> {
        let mut egrid = eint::ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, mut grid) = egrid.add_slr(self.columns.len(), self.regs * 16);

        for (col, &kind) in &self.columns {
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                grid.fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                let tile = &mut grid[(col, row)];
                tile.nodes[0].tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
                match kind {
                    ColumnKind::Bram => {
                        let yy = y % 4;
                        let dy = y - yy;
                        tile.add_intf(
                            db.get_intf("INTF"),
                            format!("BRAM_X{x}Y{dy}"),
                            db.get_intf_naming(&format!("BRAM.{yy}")),
                        );
                    }
                    ColumnKind::Dsp => {
                        let yy = y % 4;
                        let dy = y - yy;
                        tile.add_intf(
                            db.get_intf("INTF"),
                            format!("DSP_X{x}Y{dy}"),
                            db.get_intf_naming(&format!("DSP.{yy}")),
                        );
                    }
                    _ => (),
                }
            }
        }

        for col in [self.cols_io[0], self.cols_io[2]] {
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let c = match y % 16 {
                    7 | 8 => "LC",
                    _ => "NC",
                };
                let l = if col.to_idx() == 0 { "_L" } else { "" };
                grid[(col, row)].add_intf(
                    db.get_intf("INTF"),
                    format!("IOIS_{c}{l}_X{x}Y{y}"),
                    db.get_intf_naming("IOIS"),
                );
            }
        }

        let mut row_b = grid.rows().next().unwrap();
        let mut row_t = grid.rows().next_back().unwrap() + 1;
        let mut sysmons = vec![];
        if self.has_bot_sysmon {
            sysmons.push(row_b);
            row_b += 8;
        }
        if self.has_top_sysmon {
            sysmons.push(row_t - 8);
            row_t -= 8;
        }
        for row in sysmons {
            for dy in 0..8 {
                let x = self.cols_io[1].to_idx();
                let y = row.to_idx();
                grid[(self.cols_io[1], row + dy)].add_intf(
                    db.get_intf("INTF"),
                    format!("SYS_MON_X{x}Y{y}"),
                    db.get_intf_naming(&format!("SYSMON.{dy}")),
                );
            }
        }
        for dy in 0..16 {
            let x = self.cols_io[1].to_idx();
            let y = self.reg_cfg * 16 - 1;
            let row = RowId::from_idx(self.reg_cfg * 16 - 8 + dy);
            grid[(self.cols_io[1], row)].add_intf(
                db.get_intf("INTF"),
                format!("CFG_CENTER_X{x}Y{y}"),
                db.get_intf_naming(&format!("CFG_CENTER.{dy}")),
            );
        }
        for dy in 0..(self.regs_cfg_io * 16) {
            let row = RowId::from_idx(self.reg_cfg * 16 + 8 + dy);
            let x = self.cols_io[1].to_idx();
            let y = row.to_idx();
            grid[(self.cols_io[1], row)].add_intf(
                db.get_intf("INTF"),
                format!("IOIS_LC_X{x}Y{y}"),
                db.get_intf_naming("IOIS"),
            );
        }
        for dy in 0..(self.regs_cfg_io * 16) {
            let row = RowId::from_idx(self.reg_cfg * 16 - 8 - self.regs_cfg_io * 16 + dy);
            let x = self.cols_io[1].to_idx();
            let y = row.to_idx();
            grid[(self.cols_io[1], row)].add_intf(
                db.get_intf("INTF"),
                format!("IOIS_LC_X{x}Y{y}"),
                db.get_intf_naming("IOIS"),
            );
        }
        let mut row = RowId::from_idx(self.reg_cfg * 16 + 8 + self.regs_cfg_io * 16);
        let mut ccms = self.ccm;
        while row != row_t {
            let t = if ccms != 0 {
                ccms -= 1;
                "CCM"
            } else {
                grid[(self.cols_io[1], row)].nodes[0].naming = db.get_node_naming("INT.DCM0");
                "DCM"
            };
            let x = self.cols_io[1].to_idx();
            let y = row.to_idx();
            for dy in 0..4 {
                grid[(self.cols_io[1], row + dy)].add_intf(
                    db.get_intf("INTF"),
                    format!("{t}_X{x}Y{y}"),
                    db.get_intf_naming(&format!("{t}.{dy}")),
                );
            }
            row += 4;
        }
        let mut row = RowId::from_idx(self.reg_cfg * 16 - 8 - self.regs_cfg_io * 16);
        let mut ccms = self.ccm;
        while row != row_b {
            row -= 4;
            let x = self.cols_io[1].to_idx();
            let y = row.to_idx();
            let (t, tt) = if ccms != 0 {
                ccms -= 1;
                ("CCM", "CCM")
            } else {
                grid[(self.cols_io[1], row)].nodes[0].naming = db.get_node_naming("INT.DCM0");
                ("DCM", "DCM_BOT")
            };
            for dy in 0..4 {
                grid[(self.cols_io[1], row + dy)].add_intf(
                    db.get_intf("INTF"),
                    format!("{tt}_X{x}Y{y}"),
                    db.get_intf_naming(&format!("{t}.{dy}")),
                );
            }
        }

        for (py, &(bc, br)) in self.holes_ppc.iter().enumerate() {
            grid.nuke_rect(bc + 1, br + 1, 7, 22);
            let x = bc.to_idx();
            let yb = br.to_idx() + 3;
            let yt = br.to_idx() + 19;
            let tile_pb = format!("PB_X{x}Y{yb}");
            let tile_pt = format!("PT_X{x}Y{yt}");
            let col_l = bc;
            let col_r = bc + 8;
            for dy in 0..22 {
                let row = br + 1 + dy;
                let tile = if dy < 11 { &tile_pb } else { &tile_pt };
                grid.fill_term_pair_buf(
                    (col_l, row),
                    (col_r, row),
                    db.get_term("PPC.E"),
                    db.get_term("PPC.W"),
                    tile.clone(),
                    db.get_term_naming(&format!("TERM.PPC.E{dy}")),
                    db.get_term_naming(&format!("TERM.PPC.W{dy}")),
                );
            }
            let row_b = br;
            let row_t = br + 23;
            for dx in 0..7 {
                let col = bc + 1 + dx;
                grid.fill_term_pair_dbuf(
                    (col, row_b),
                    (col, row_t),
                    db.get_term(if dx < 5 { "PPCA.N" } else { "PPCB.N" }),
                    db.get_term(if dx < 5 { "PPCA.S" } else { "PPCB.S" }),
                    tile_pb.clone(),
                    tile_pt.clone(),
                    db.get_term_naming(&format!("TERM.PPC.N{dx}")),
                    db.get_term_naming(&format!("TERM.PPC.S{dx}")),
                );
            }
            for dy in 0..24 {
                let row = br + dy;
                let tile = if dy < 12 { &tile_pb } else { &tile_pt };
                let tile_l = &mut grid[(col_l, row)];
                tile_l.intfs.clear();
                tile_l.add_intf(
                    db.get_intf("INTF"),
                    tile.clone(),
                    db.get_intf_naming(&format!("PPC.L{dy}")),
                );
                let tile_r = &mut grid[(col_r, row)];
                tile_r.intfs.clear();
                tile_r.add_intf(
                    db.get_intf("INTF"),
                    tile.clone(),
                    db.get_intf_naming(&format!("PPC.R{dy}")),
                );
            }
            for dx in 0..7 {
                let col = bc + dx + 1;
                let tile_b = &mut grid[(col, row_b)];
                tile_b.intfs.clear();
                tile_b.add_intf(
                    db.get_intf("INTF"),
                    tile_pb.clone(),
                    db.get_intf_naming(&format!("PPC.B{dx}")),
                );
                let tile_t = &mut grid[(col, row_t)];
                tile_t.intfs.clear();
                tile_t.add_intf(
                    db.get_intf("INTF"),
                    tile_pt.clone(),
                    db.get_intf_naming(&format!("PPC.T{dx}")),
                );
            }
            let mut crds = vec![];
            for dy in 0..24 {
                crds.push((col_l, br + dy));
            }
            for dy in 0..24 {
                crds.push((col_r, br + dy));
            }
            for dx in 1..8 {
                crds.push((bc + dx, row_b));
            }
            for dx in 1..8 {
                crds.push((bc + dx, row_t));
            }
            let node = grid[(bc, br)].add_xnode(
                db.get_node("PPC"),
                &[&tile_pb, &tile_pt],
                db.get_node_naming("PPC"),
                &crds,
            );
            node.add_bel(0, format!("PPC405_ADV_X0Y{py}"));
            node.add_bel(1, format!("EMAC_X0Y{py}"));
        }

        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();
        let yb = row_b.to_idx();
        let yt = row_t.to_idx();
        for col in grid.cols() {
            let x = col.to_idx();
            grid.fill_term_tile(
                (col, row_b),
                "TERM.S",
                "TERM.S",
                format!("B_TERM_INT_X{x}Y{yb}"),
            );
            grid.fill_term_tile(
                (col, row_t),
                "TERM.N",
                "TERM.N",
                format!("T_TERM_INT_X{x}Y{yt}"),
            );
        }
        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let xl = col_l.to_idx();
        let xr = col_r.to_idx();
        for row in grid.rows() {
            let y = row.to_idx();
            if self.columns[col_l] == ColumnKind::Gt {
                let dy = y % 16;
                let yy = y - dy + 8;
                let ab = if y % 32 >= 16 { "A" } else { "B" };
                let tile = format!("MGT_{ab}L_X{xl}Y{yy}");
                grid.fill_term_tile(
                    (col_l, row),
                    "TERM.W",
                    &format!("TERM.W.MGT{dy}"),
                    tile.clone(),
                );
                grid[(col_l, row)].add_intf(
                    db.get_intf("INTF"),
                    tile,
                    db.get_intf_naming(&format!("MGT.{dy}")),
                );
            } else {
                grid.fill_term_tile(
                    (col_l, row),
                    "TERM.W",
                    "TERM.W",
                    format!("L_TERM_INT_X{xl}Y{y}"),
                );
            }
            if self.columns[col_r] == ColumnKind::Gt {
                let dy = y % 16;
                let yy = y - dy + 8;
                let ab = if y % 32 >= 16 { "A" } else { "B" };
                let tile = format!("MGT_{ab}R_X{xr}Y{yy}");
                grid.fill_term_tile(
                    (col_r, row),
                    "TERM.E",
                    &format!("TERM.E.MGT{dy}"),
                    tile.clone(),
                );
                grid[(col_r, row)].add_intf(
                    db.get_intf("INTF"),
                    tile,
                    db.get_intf_naming(&format!("MGT.{dy}")),
                );
            } else {
                grid.fill_term_tile(
                    (col_r, row),
                    "TERM.E",
                    "TERM.E",
                    format!("R_TERM_INT_X{xr}Y{y}"),
                );
            }
        }

        let term_s = db.get_term("BRKH.S");
        let term_n = db.get_term("BRKH.N");
        for col in grid.cols() {
            for row in grid.rows() {
                if row.to_idx() % 8 != 0 || row.to_idx() == 0 {
                    continue;
                }
                if !grid[(col, row)].nodes.is_empty() {
                    grid.fill_term_pair_anon((col, row - 1), (col, row), term_n, term_s);
                }
            }
        }

        let term_w = db.get_term("CLB_BUFFER.W");
        let term_e = db.get_term("CLB_BUFFER.E");
        let naming_w = db.get_term_naming("PASS.CLB_BUFFER.W");
        let naming_e = db.get_term_naming("PASS.CLB_BUFFER.E");
        for (col, &cd) in &self.columns {
            if cd != ColumnKind::Io || col == col_l || col == col_r {
                continue;
            }
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let tile = format!("CLB_BUFFER_X{x}Y{y}");
                grid.fill_term_pair_buf(
                    (col, row),
                    (col + 1, row),
                    term_e,
                    term_w,
                    tile,
                    naming_w,
                    naming_e,
                );
            }
        }

        grid.fill_main_passes();

        let mut sx = 0;
        for (col, &cd) in &self.columns {
            if cd != ColumnKind::Clb {
                continue;
            }
            for row in grid.rows() {
                let tile = &mut grid[(col, row)];
                if tile.nodes.is_empty() || !tile.intfs.is_empty() {
                    continue;
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("CLB_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node("CLB"),
                    &[&name],
                    db.get_node_naming("CLB"),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}", sy = 2 * y));
                node.add_bel(1, format!("SLICE_X{sx}Y{sy}", sx = sx + 1, sy = 2 * y));
                node.add_bel(2, format!("SLICE_X{sx}Y{sy}", sy = 2 * y + 1));
                node.add_bel(3, format!("SLICE_X{sx}Y{sy}", sx = sx + 1, sy = 2 * y + 1));
            }
            sx += 2;
        }

        let mut bx = 0;
        let mut dx = 0;
        for (col, &cd) in &self.columns {
            let kind = match cd {
                ColumnKind::Bram => "BRAM",
                ColumnKind::Dsp => "DSP",
                _ => continue,
            };
            'a: for row in grid.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                for &(bc, br) in &self.holes_ppc {
                    if col >= bc && col < bc + 9 && row >= br && row < br + 24 {
                        continue 'a;
                    }
                }
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
                let node = grid[(col, row)].add_xnode(
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(kind),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                    ],
                );
                if cd == ColumnKind::Bram {
                    node.add_bel(0, format!("RAMB16_X{bx}Y{sy}", sy = y / 4));
                    node.add_bel(1, format!("FIFO16_X{bx}Y{sy}", sy = y / 4));
                } else {
                    node.add_bel(0, format!("DSP48_X{dx}Y{sy}", sy = y / 4 * 2));
                    node.add_bel(1, format!("DSP48_X{dx}Y{sy}", sy = y / 4 * 2 + 1));
                }
            }
            if cd == ColumnKind::Bram {
                bx += 1;
            } else {
                dx += 1;
            }
        }

        for col in grid.cols() {
            for row in grid.rows() {
                let crow = RowId::from_idx(row.to_idx() / 16 * 16 + 8);
                grid[(col, row)].clkroot = (col, crow);
            }
        }

        egrid
    }
}
