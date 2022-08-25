use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize};
use crate::{CfgPin, BelCoord, GtPin, DisabledPart, ColId, RowId, BelId, int, eint::{self, ExpandedSlrRefMut, Coord}};
use prjcombine_entity::{EntityVec, EntityId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: EntityVec<ColId, Column>,
    pub col_clk: ColId,
    pub cols_clk_fold: Option<(ColId, ColId)>,
    pub cols_reg_buf: (ColId, ColId),
    pub rows: EntityVec<RowId, Row>,
    pub rows_midbuf: (RowId, RowId),
    pub rows_hclkbuf: (RowId, RowId),
    pub rows_bufio_split: (RowId, RowId),
    pub rows_bank_split: Option<(RowId, RowId)>,
    pub gts: Gts,
    pub mcbs: Vec<Mcb>,
    pub vref: BTreeSet<BelCoord>,
    pub cfg_io: BTreeMap<CfgPin, BelCoord>,
    pub has_encrypt: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub kind: ColumnKind,
    pub bio: ColumnIoKind,
    pub tio: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Io,
    CleXL,
    CleXM,
    CleClk,
    Bram,
    Dsp,
    DspPlus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnIoKind {
    None,
    Both,
    Inner,
    Outer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub lio: bool,
    pub rio: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Gts {
    None,
    Single(ColId),
    Double(ColId, ColId),
    Quad(ColId, ColId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McbIo {
    pub row: RowId,
    pub bel: BelId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Mcb {
    pub row_mcb: RowId,
    pub row_mui: [RowId; 8],
    pub iop_dq: [RowId; 8],
    pub iop_dqs: [RowId; 2],
    pub io_dm: [McbIo; 2],
    pub iop_clk: RowId,
    pub io_addr: [McbIo; 15],
    pub io_ba: [McbIo; 3],
    pub io_ras: McbIo,
    pub io_cas: McbIo,
    pub io_we: McbIo,
    pub io_odt: McbIo,
    pub io_cke: McbIo,
    pub io_reset: McbIo,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub bank: u32,
    pub coord: BelCoord,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub gx: u32,
    pub gy: u32,
    pub top: bool,
    pub bank: u32,
}

impl Gt {
    pub fn get_pads(&self) -> Vec<(String, String, GtPin, u32)> {
        let mut res = Vec::new();
        for b in 0..2 {
            res.push((format!("OPAD_X{}Y{}", self.gx, self.gy * 4 + 1 - b), format!("MGTTXP{}_{}", b, self.bank), GtPin::TxP, b));
            res.push((format!("OPAD_X{}Y{}", self.gx, self.gy * 4 + 3 - b), format!("MGTTXN{}_{}", b, self.bank), GtPin::TxN, b));
            res.push((format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + b), format!("MGTRXN{}_{}", b, self.bank), GtPin::RxN, b));
            res.push((format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 2 + b), format!("MGTRXP{}_{}", b, self.bank), GtPin::RxP, b));
        }
        for b in 0..2 {
            res.push((format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 4 + 2 * b), format!("MGTREFCLK{}N_{}", b, self.bank), GtPin::ClkN, b));
            res.push((format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 5 + 2 * b), format!("MGTREFCLK{}P_{}", b, self.bank), GtPin::ClkP, b));
        }
        res
    }
}

fn fill_intf_rterm(db: &int::IntDb, slr: &mut ExpandedSlrRefMut, crd: Coord, name: String) {
    slr.fill_term_tile(crd, "TERM.E", "TERM.E.INTF", name.clone());
    let tile = &mut slr[crd];
    tile.intfs.clear();
    tile.add_intf(
        db.get_intf("INTF"),
        name,
        db.get_intf_naming("INTF.RTERM"),
    );
}

fn fill_intf_lterm(db: &int::IntDb, slr: &mut ExpandedSlrRefMut, crd: Coord, name: String, is_brk: bool) {
    slr.fill_term_tile(crd, "TERM.W", "TERM.W.INTF", name.clone());
    let tile = &mut slr[crd];
    tile.intfs.clear();
    tile.add_intf(
        db.get_intf("INTF"),
        name,
        db.get_intf_naming("INTF.LTERM"),
    );
    tile.nodes[0].naming = db.get_node_naming(if is_brk {"INT.TERM.BRK"} else {"INT.TERM"});
}

impl Grid {
    pub fn get_io(&self) -> Vec<Io> {
        let mut res = Vec::new();
        let mut ctr = 1;
        // top
        for (col, &cd) in self.columns.iter() {
            let row_o = self.rows.last_id().unwrap();
            let row_i = row_o - 1;
            if matches!(cd.tio, ColumnIoKind::Outer | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 0,
                        coord: BelCoord {
                            col,
                            row: row_o,
                            bel: BelId::from_idx(bel),
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
            if matches!(cd.tio, ColumnIoKind::Inner | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 0,
                        coord: BelCoord {
                            col,
                            row: row_i,
                            bel: BelId::from_idx(bel),
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
        }
        // right
        for (row, rd) in self.rows.iter().rev() {
            if !rd.rio {
                continue;
            }
            let col = self.columns.last_id().unwrap();
            let bank = if let Some((_, sr)) = self.rows_bank_split {
                if row >= sr {
                    5
                } else {
                    1
                }
            } else {
                1
            };
            for bel in [0, 1] {
                res.push(Io {
                    bank,
                    coord: BelCoord {
                        col,
                        row,
                        bel: BelId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        // bot
        for (col, &cd) in self.columns.iter().rev() {
            let row_o = self.rows.first_id().unwrap();
            let row_i = row_o + 1;
            if matches!(cd.bio, ColumnIoKind::Outer | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 2,
                        coord: BelCoord {
                            col,
                            row: row_o,
                            bel: BelId::from_idx(bel),
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
            if matches!(cd.bio, ColumnIoKind::Inner | ColumnIoKind::Both) {
                for bel in [0, 1] {
                    res.push(Io {
                        bank: 2,
                        coord: BelCoord {
                            col,
                            row: row_i,
                            bel: BelId::from_idx(bel),
                        },
                        name: format!("PAD{ctr}"),
                    });
                    ctr += 1;
                }
            }
        }
        // left
        for (row, rd) in self.rows.iter() {
            if !rd.lio {
                continue;
            }
            let col = self.columns.first_id().unwrap();
            let bank = if let Some((sl, _)) = self.rows_bank_split {
                if row >= sl {
                    4
                } else {
                    3
                }
            } else {
                3
            };
            for bel in [0, 1] {
                res.push(Io {
                    bank,
                    coord: BelCoord {
                        col,
                        row,
                        bel: BelId::from_idx(bel),
                    },
                    name: format!("PAD{ctr}"),
                });
                ctr += 1;
            }
        }
        res
    }

    pub fn get_gt(&self, disabled: &BTreeSet<DisabledPart>) -> Vec<Gt> {
        let mut res = Vec::new();
        if !disabled.contains(&DisabledPart::Spartan6Gtp) {
            match self.gts {
                Gts::Single(_) => {
                    res.push(Gt {
                        gx: 0,
                        gy: 0,
                        top: true,
                        bank: 101,
                    });
                }
                Gts::Double(_, _) => {
                    res.push(Gt {
                        gx: 0,
                        gy: 0,
                        top: true,
                        bank: 101,
                    });
                    res.push(Gt {
                        gx: 1,
                        gy: 0,
                        top: true,
                        bank: 123,
                    });
                }
                Gts::Quad(_, _) => {
                    res.push(Gt {
                        gx: 0,
                        gy: 1,
                        top: true,
                        bank: 101,
                    });
                    res.push(Gt {
                        gx: 1,
                        gy: 1,
                        top: true,
                        bank: 123,
                    });
                    res.push(Gt {
                        gx: 0,
                        gy: 0,
                        top: false,
                        bank: 245,
                    });
                    res.push(Gt {
                        gx: 1,
                        gy: 0,
                        top: false,
                        bank: 267,
                    });
                }
                Gts::None => (),
            }
        }
        res
    }

    pub fn row_clk(&self) -> RowId {
        RowId::from_idx(self.rows.len() / 2)
    }

    pub fn get_plls(&self) -> Vec<RowId> {
        let mut res = Vec::new();
        let mut row = self.rows.first_id().unwrap();
        while row + 32 <= self.row_clk() {
            res.push(row + 16);
            row += 32;
        }
        if self.row_clk().to_idx() % 16 == 8 {
            row += 16;
        }
        while row + 32 <= self.rows.next_id() {
            res.push(row + 16);
            row += 32;
        }
        res
    }

    pub fn get_dcms(&self) -> Vec<RowId> {
        let mut res = Vec::new();
        let mut row = self.rows.first_id().unwrap();
        while row + 32 <= self.row_clk() {
            res.push(row);
            row += 32;
        }
        if self.row_clk().to_idx() % 16 == 8 {
            row += 16;
        }
        while row + 32 <= self.rows.next_id() {
            res.push(row);
            row += 32;
        }
        res
    }

    pub fn expand_grid<'a>(&self, db: &'a int::IntDb) -> eint::ExpandedGrid<'a> {
        let mut egrid = eint::ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, mut grid) = egrid.add_slr(self.columns.len(), self.rows.len());
        let def_rt = int::NodeRawTileId::from_idx(0);

        let mut tie_x = 0;
        let mut rxlut = EntityVec::new();
        let mut rx = 2;
        for (col, &cd) in &self.columns {
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let tie_y = y * 2;
                let mut is_brk = y % 16 == 0;
                if y == 0 && !matches!(cd.kind, ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus) {
                    is_brk = false;
                }
                if row == self.row_clk() && cd.kind == ColumnKind::Io {
                    is_brk = false;
                }
                let bram = if cd.kind == ColumnKind::Bram {if is_brk {"_BRAM_BRK"} else {"_BRAM"}} else {""};
                grid.fill_tile((col, row), "INT", if is_brk {"INT.BRK"} else {"INT"}, format!("INT{bram}_X{x}Y{y}"));
                grid[(col, row)].nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                if matches!(cd.kind, ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus) {
                    grid[(col, row)].add_intf(
                        db.get_intf("INTF"),
                        format!("INT_INTERFACE_X{x}Y{y}"),
                        db.get_intf_naming("INTF"),
                    );
                }
            }
            tie_x += 1;
            if cd.kind == ColumnKind::Io || cd.tio != ColumnIoKind::None || cd.bio != ColumnIoKind::None {
                tie_x += 1;
            }
            if cd.kind == ColumnKind::CleClk {
                tie_x += 1;
            }
            rxlut.push(rx);
            match cd.kind {
                ColumnKind::CleXL | ColumnKind::CleXM => rx += 2,
                ColumnKind::CleClk => rx += 4,
                _ => rx += 3,
            }
        }

        let col_l = self.columns.first_id().unwrap();
        let col_r = self.columns.last_id().unwrap();
        let xl = col_l.to_idx();
        let xc = self.col_clk.to_idx();
        let xr = col_r.to_idx();
        let row_bo = self.rows.first_id().unwrap();
        let row_bi = row_bo + 1;
        let row_to = self.rows.last_id().unwrap();
        let row_ti = row_to - 1;

        let mut rylut = EntityVec::new();
        let mut ry = 2;
        for row in self.rows.ids() {
            if row == self.row_clk() {
                ry += 1;
            }
            if row.to_idx() % 16 == 8 {
                ry += 1;
            }
            rylut.push(ry);
            ry += 1;
        }

        for (row, &rd) in &self.rows {
            let y = row.to_idx();
            let ry = rylut[row];
            let is_brk = y % 16 == 0 && row != self.row_clk();
            let brk = if is_brk {"_BRK"} else {""};
            let txtra = if row == self.row_clk() - 2 {
                "_LOWER_BOT"
            } else if row == self.row_clk() - 1 {
                "_LOWER_TOP"
            } else if row == self.row_clk() + 2 {
                "_UPPER_BOT"
            } else if row == self.row_clk() + 3 {
                "_UPPER_TOP"
            } else {
                ""
            };
            let tile = &mut grid[(col_l, row)];
            let mut ltt = "IOI_LTERM";
            if rd.lio {
                let node = &mut tile.nodes[0];
                node.kind = db.get_node("IOI");
                if !is_brk {
                    node.names[def_rt] = format!("LIOI_INT_X{xl}Y{y}");
                }
                node.naming = db.get_node_naming(if is_brk {"IOI.BRK"} else {"IOI"});
                tile.add_intf(
                    db.get_intf("INTF.IOI"),
                    format!("LIOI{brk}_X{xl}Y{y}"),
                    db.get_intf_naming("INTF.IOI"),
                );
            } else {
                let cnr = if row == row_bo {
                    Some("LL")
                } else if row == row_to {
                    Some("UL")
                } else {
                    None
                };
                if let Some(cnr) = cnr {
                    ltt = "CNR_TL_LTERM";
                    tile.add_intf(
                        db.get_intf("INTF"),
                        format!("{cnr}_X{xl}Y{y}"),
                        db.get_intf_naming("INTF.CNR"),
                    );
                } else {
                    let carry = if is_brk {"_CARRY"} else {""};
                    tile.add_intf(
                        db.get_intf("INTF"),
                        format!("INT_INTERFACE{carry}_X{xl}Y{y}"),
                        db.get_intf_naming("INTF"),
                    );
                }
            }
            let rxl = rxlut[col_l] - 1;
            grid.fill_term_tile((col_l, row), "TERM.W", "TERM.W", format!("{ltt}{txtra}_X{rxl}Y{ry}"));
            let tile = &mut grid[(col_r, row)];
            let mut rtt = "IOI_RTERM";
            if rd.rio {
                let node = &mut tile.nodes[0];
                node.kind = db.get_node("IOI");
                if !is_brk {
                    node.names[def_rt] = format!("IOI_INT_X{xr}Y{y}");
                }
                node.naming = db.get_node_naming(if is_brk {"IOI.BRK"} else {"IOI"});
                tile.add_intf(
                    db.get_intf("INTF.IOI"),
                    format!("RIOI{brk}_X{xr}Y{y}"),
                    db.get_intf_naming("INTF.IOI"),
                );
            } else {
                let cnr = if row == row_bo {
                    Some("LR_LOWER")
                } else if row == row_bi {
                    Some("LR_UPPER")
                } else if row == row_ti {
                    Some("UR_LOWER")
                } else if row == row_to {
                    Some("UR_UPPER")
                } else {
                    None
                };
                if let Some(cnr) = cnr {
                    rtt = "CNR_TR_RTERM";
                    tile.add_intf(
                        db.get_intf("INTF"),
                        format!("{cnr}_X{xr}Y{y}"),
                        db.get_intf_naming("INTF.CNR"),
                    );
                } else {
                    let carry = if is_brk {"_CARRY"} else {""};
                    tile.add_intf(
                        db.get_intf("INTF"),
                        format!("INT_INTERFACE{carry}_X{xr}Y{y}"),
                        db.get_intf_naming("INTF"),
                    );
                }
            }
            let rxr = rxlut[col_r] + 3;
            grid.fill_term_tile((col_r, row), "TERM.E", "TERM.E", format!("{rtt}{txtra}_X{rxr}Y{ry}"));
        }

        for (col, &cd) in &self.columns {
            let x = col.to_idx();
            if cd.bio != ColumnIoKind::None {
                for (row, io, unused) in [
                    (row_bo, "OUTER", cd.bio == ColumnIoKind::Inner),
                    (row_bi, "INNER", cd.bio == ColumnIoKind::Outer),
                ] {
                    let y = row.to_idx();
                    let tile = &mut grid[(col, row)];
                    let node = &mut tile.nodes[0];
                    let unused = if unused {"_UNUSED"} else {""};
                    node.kind = db.get_node("IOI");
                    node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
                    node.naming = db.get_node_naming("IOI");
                    tile.add_intf(
                        db.get_intf("INTF.IOI"),
                        format!("BIOI_{io}{unused}_X{x}Y{y}"),
                        db.get_intf_naming("INTF.IOI"),
                    );
                }
            }
            if cd.tio != ColumnIoKind::None {
                for (row, io, unused) in [
                    (row_to, "OUTER", cd.tio == ColumnIoKind::Inner),
                    (row_ti, "INNER", cd.tio == ColumnIoKind::Outer),
                ] {
                    let y = row.to_idx();
                    let tile = &mut grid[(col, row)];
                    let node = &mut tile.nodes[0];
                    let unused = if unused {"_UNUSED"} else {""};
                    node.kind = db.get_node("IOI");
                    node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
                    node.naming = db.get_node_naming("IOI");
                    tile.add_intf(
                        db.get_intf("INTF.IOI"),
                        format!("TIOI_{io}{unused}_X{x}Y{y}"),
                        db.get_intf_naming("INTF.IOI"),
                    );
                }
            }
        }

        let yc = self.row_clk().to_idx();
        grid[(self.col_clk, self.row_clk())].add_intf(
            db.get_intf("INTF"),
            format!("INT_INTERFACE_REGC_X{xc}Y{yc}"),
            db.get_intf_naming("INTF.REGC"),
        );

        for br in self.get_dcms() {
            for row in [br + 7, br + 8] {
                let y = row.to_idx();
                let tile = &mut grid[(self.col_clk, row)];
                let node = &mut tile.nodes[0];
                node.kind = db.get_node("IOI");
                node.names[def_rt] = format!("IOI_INT_X{xc}Y{y}");
                node.naming = db.get_node_naming("IOI");
                tile.add_intf(
                    db.get_intf("INTF.IOI"),
                    format!("INT_INTERFACE_IOI_X{xc}Y{y}"),
                    db.get_intf_naming("INTF"),
                );
            }
        }

        for br in self.get_plls() {
            let row = br + 7;
            let y = row.to_idx();
            let tile = &mut grid[(self.col_clk, row)];
            tile.add_intf(
                db.get_intf("INTF"),
                format!("INT_INTERFACE_CARRY_X{xc}Y{y}"),
                db.get_intf_naming("INTF"),
            );
            let row = br + 8;
            let y = row.to_idx();
            let tile = &mut grid[(self.col_clk, row)];
            let node = &mut tile.nodes[0];
            node.kind = db.get_node("IOI");
            node.names[def_rt] = format!("IOI_INT_X{xc}Y{y}");
            node.naming = db.get_node_naming("IOI");
            tile.add_intf(
                db.get_intf("INTF.IOI"),
                format!("INT_INTERFACE_IOI_X{xc}Y{y}"),
                db.get_intf_naming("INTF"),
            );
        }

        match self.gts {
            Gts::Single(bc) | Gts::Double(bc, _) | Gts::Quad(bc, _) => {
                grid.nuke_rect(bc - 6, row_to - 7, 11, 8);
                grid.nuke_rect(bc - 4, row_to - 15, 7, 8);
                grid.nuke_rect(bc - 1, row_to - 31, 3, 16);
                let col_l = bc - 7;
                let col_r = bc + 5;
                let rxl = rxlut[col_l] + 6;
                let rxr = rxlut[col_r] - 1;
                for dy in 0..8 {
                    let row = row_to - 7 + dy;
                    let ry = rylut[row];
                    grid.fill_term_tile((col_l, row), "TERM.E", "TERM.E.INTF", format!("INT_RTERM_X{rxl}Y{ry}"));
                    grid.fill_term_tile((col_r, row), "TERM.W", "TERM.W.INTF", format!("INT_LTERM_X{rxr}Y{ry}"));
                }
                let col_l = bc - 5;
                let col_r = bc + 3;
                for dy in 0..8 {
                    let row = row_to - 15 + dy;
                    let ry = rylut[row];
                    let rxl = rxlut[col_l] + 1;
                    let rxr = rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    fill_intf_rterm(db, &mut grid, (col_l, row), tile_l);
                    fill_intf_lterm(db, &mut grid, (col_r, row), tile_r, is_brk);
                }
                let col_l = bc - 2;
                let col_r = bc + 2;
                for dy in 0..16 {
                    let row = row_to - 31 + dy;
                    let ry = rylut[row];
                    let rxl = rxlut[col_l] + 1;
                    let rxr = rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    fill_intf_rterm(db, &mut grid, (col_l, row), tile_l);
                    fill_intf_lterm(db, &mut grid, (col_r, row), tile_r, is_brk);
                }
            }
            _ => (),
        }
        match self.gts {
            Gts::Double(_, bc) | Gts::Quad(_, bc) => {
                grid.nuke_rect(bc - 4, row_to - 7, 11, 8);
                grid.nuke_rect(bc - 2, row_to - 15, 8, 8);
                let col_l = bc - 5;
                let col_r = bc + 7;
                for dy in 0..8 {
                    let row = row_to - 7 + dy;
                    let ry = rylut[row];
                    let rxl = rxlut[col_l] + 5;
                    let rxr = rxlut[col_r] - 2;
                    grid.fill_term_tile((col_l, row), "TERM.E", "TERM.E.INTF", format!("INT_RTERM_X{rxl}Y{ry}"));
                    grid.fill_term_tile((col_r, row), "TERM.W", "TERM.W.INTF", format!("INT_LTERM_X{rxr}Y{ry}"));
                }
                let col_l = bc - 3;
                let col_r = bc + 6;
                for dy in 0..8 {
                    let row = row_to - 15 + dy;
                    let ry = rylut[row];
                    let rxl = rxlut[col_l] + 1;
                    let rxr = rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    fill_intf_rterm(db, &mut grid, (col_l, row), tile_l);
                    fill_intf_lterm(db, &mut grid, (col_r, row), tile_r, is_brk);
                }
            }
            _ => (),
        }
        if let Gts::Quad(bcl, bcr) = self.gts {
            grid.nuke_rect(bcl - 6, row_bo, 11, 8);
            grid.nuke_rect(bcl - 4, row_bo + 8, 7, 8);
            grid.nuke_rect(bcr - 4, row_bo, 11, 8);
            grid.nuke_rect(bcr - 2, row_bo + 8, 8, 8);
            let col_l = bcl - 7;
            let col_r = bcl + 5;
            for dy in 0..8 {
                let row = row_bo + dy;
                let ry = rylut[row];
                let rxl = rxlut[col_l] + 6;
                let rxr = rxlut[col_r] - 1;
                grid.fill_term_tile((col_l, row), "TERM.E", "TERM.E.INTF", format!("INT_RTERM_X{rxl}Y{ry}"));
                grid.fill_term_tile((col_r, row), "TERM.W", "TERM.W.INTF", format!("INT_LTERM_X{rxr}Y{ry}"));
            }
            let col_l = bcl - 5;
            let col_r = bcl + 3;
            for dy in 0..8 {
                let row = row_bo + 8 + dy;
                let ry = rylut[row];
                let rxl = rxlut[col_l] + 1;
                let rxr = rxlut[col_r] - 1;
                let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                fill_intf_rterm(db, &mut grid, (col_l, row), tile_l);
                fill_intf_lterm(db, &mut grid, (col_r, row), tile_r, false);
            }
            let col_l = bcr - 5;
            let col_r = bcr + 7;
            for dy in 0..8 {
                let row = row_bo + dy;
                let ry = rylut[row];
                let rxl = rxlut[col_l] + 5;
                let rxr = rxlut[col_r] - 2;
                grid.fill_term_tile((col_l, row), "TERM.E", "TERM.E.INTF", format!("INT_RTERM_X{rxl}Y{ry}"));
                grid.fill_term_tile((col_r, row), "TERM.W", "TERM.W.INTF", format!("INT_LTERM_X{rxr}Y{ry}"));
            }
            let col_l = bcr - 3;
            let col_r = bcr + 6;
            for dy in 0..8 {
                let row = row_bo + 8 + dy;
                let ry = rylut[row];
                let rxl = rxlut[col_l] + 1;
                let rxr = rxlut[col_r] - 1;
                let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                fill_intf_rterm(db, &mut grid, (col_l, row), tile_l);
                fill_intf_lterm(db, &mut grid, (col_r, row), tile_r, false);
            }
        }

        for (col, &cd) in &self.columns {
            let (btt, ttt) = match cd.kind {
                ColumnKind::Io => ("CNR_BR_BTERM", "CNR_TR_TTERM"),
                ColumnKind::Bram => ("", "RAMB_TOP_TTERM"),
                ColumnKind::Dsp | ColumnKind::DspPlus => ("DSP_INT_BTERM", "DSP_INT_TTERM"),
                _ => if col == self.col_clk + 1 {
                    ("IOI_BTERM_BUFPLL", "IOI_TTERM_BUFPLL")
                } else {
                    (if cd.bio == ColumnIoKind::None {"CLB_INT_BTERM"} else {"IOI_BTERM"}, "IOI_TTERM")
                }
            };
            let rx = rxlut[col];
            let ryb = rylut[row_bo] - 1;
            let ryt = rylut[row_to] + 1;
            let mut row_b = row_bo;
            let mut row_t = row_to;
            while grid[(col, row_b)].nodes.is_empty() {
                row_b += 1;
            }
            while grid[(col, row_t)].nodes.is_empty() {
                row_t -= 1;
            }
            if !btt.is_empty() {
                grid.fill_term_tile((col, row_b), "TERM.S", "TERM.S", format!("{btt}_X{rx}Y{ryb}"));
            }
            grid.fill_term_tile((col, row_t), "TERM.N", "TERM.N", format!("{ttt}_X{rx}Y{ryt}"));
        }

        grid.fill_main_passes();

        for col in grid.cols() {
            for row in grid.rows() {
                let crow = RowId::from_idx(if row.to_idx() % 16 < 8 {
                    row.to_idx() / 16 * 16 + 7
                } else {
                    row.to_idx() / 16 * 16 + 8
                });
                grid[(col, row)].clkroot = (col, crow);
            }
        }

        egrid
    }
}
