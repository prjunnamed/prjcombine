use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::{BelId, IntDb, NodeRawTileId};
use prjcombine_int::grid::{ColId, Coord, ExpandedDieRefMut, ExpandedGrid, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
    pub vref: BTreeSet<IoCoord>,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
    pub has_encrypt: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×16
    // 0 doubles as DIN, MISO, MISO1
    // 1 doubles as MISO2
    // 2 doubles as MISO3
    Data(u8),
    CsoB,
    InitB,
    RdWrB,
    FcsB,
    FoeB,
    FweB,
    Ldc,
    Hdc,
    Addr(u8),
    Dout, // doubles as BUSY
    Mosi, // doubles as CSI_B, MISO0
    M0,   // doubles as CMPMISO
    M1,
    Cclk,
    UserCclk,
    HswapEn,
    CmpClk,
    CmpMosi,
    Awake,
    Scp(u8), // ×8
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Gtp,
    Mcb,
    ClbColumn(ColId),
    BramRegion(ColId, u32),
    DspRegion(ColId, u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    CmpCsB,
    Done,
    ProgB,
    Suspend,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    TxP(u8),
    TxN(u8),
    RxP(u8),
    RxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVcc,
    AVccPll(u8),
    VtTx,
    VtRx,
    RRef,
    AVttRCal,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Io(IoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Vfs,
    RFuse,
    Cfg(CfgPin),
    Gt(u32, GtPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub bel: BelId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub bank: u32,
    pub coord: IoCoord,
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
    pub fn get_pads(&self) -> Vec<(String, String, GtPin)> {
        let mut res = Vec::new();
        for b in 0..2 {
            res.push((
                format!("OPAD_X{}Y{}", self.gx, self.gy * 4 + 1 - b),
                format!("MGTTXP{}_{}", b, self.bank),
                GtPin::TxP(b as u8),
            ));
            res.push((
                format!("OPAD_X{}Y{}", self.gx, self.gy * 4 + 3 - b),
                format!("MGTTXN{}_{}", b, self.bank),
                GtPin::TxN(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + b),
                format!("MGTRXN{}_{}", b, self.bank),
                GtPin::RxN(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 2 + b),
                format!("MGTRXP{}_{}", b, self.bank),
                GtPin::RxP(b as u8),
            ));
        }
        for b in 0..2 {
            res.push((
                format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 4 + 2 * b),
                format!("MGTREFCLK{}N_{}", b, self.bank),
                GtPin::ClkN(b as u8),
            ));
            res.push((
                format!("IPAD_X{}Y{}", self.gx, self.gy * 8 + 5 + 2 * b),
                format!("MGTREFCLK{}P_{}", b, self.bank),
                GtPin::ClkP(b as u8),
            ));
        }
        res
    }
}

fn fill_intf_rterm(db: &IntDb, die: &mut ExpandedDieRefMut, crd: Coord, name: String) {
    die.fill_term_tile(crd, "TERM.E", "TERM.E.INTF", name.clone());
    let tile = &mut die[crd];
    tile.nodes.truncate(1);
    tile.add_xnode(
        db.get_node("INTF"),
        &[&name],
        db.get_node_naming("INTF.RTERM"),
        &[crd],
    );
}

fn fill_intf_lterm(
    db: &IntDb,
    die: &mut ExpandedDieRefMut,
    crd: Coord,
    name: String,
    is_brk: bool,
) {
    die.fill_term_tile(crd, "TERM.W", "TERM.W.INTF", name.clone());
    let tile = &mut die[crd];
    tile.nodes.truncate(1);
    tile.nodes[0].naming = db.get_node_naming(if is_brk { "INT.TERM.BRK" } else { "INT.TERM" });
    tile.add_xnode(
        db.get_node("INTF"),
        &[&name],
        db.get_node_naming("INTF.LTERM"),
        &[crd],
    );
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
                        coord: IoCoord {
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
                        coord: IoCoord {
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
                    coord: IoCoord {
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
                        coord: IoCoord {
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
                        coord: IoCoord {
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
                    coord: IoCoord {
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
        if !disabled.contains(&DisabledPart::Gtp) {
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

    pub fn expand_grid<'a>(
        &self,
        db: &'a IntDb,
        disabled: &BTreeSet<DisabledPart>,
    ) -> ExpandedGrid<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, mut grid) = egrid.add_die(self.columns.len(), self.rows.len());
        let def_rt = NodeRawTileId::from_idx(0);

        let mut tie_x = 0;
        let mut rxlut = EntityVec::new();
        let mut rx = 2;
        for (col, &cd) in &self.columns {
            for row in grid.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let tie_y = y * 2;
                let mut is_brk = y % 16 == 0;
                if y == 0
                    && !matches!(
                        cd.kind,
                        ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
                    )
                {
                    is_brk = false;
                }
                if row == self.row_clk() && cd.kind == ColumnKind::Io {
                    is_brk = false;
                }
                let bram = if cd.kind == ColumnKind::Bram {
                    if is_brk {
                        "_BRAM_BRK"
                    } else {
                        "_BRAM"
                    }
                } else {
                    ""
                };
                grid.fill_tile(
                    (col, row),
                    "INT",
                    if is_brk { "INT.BRK" } else { "INT" },
                    format!("INT{bram}_X{x}Y{y}"),
                );
                grid[(col, row)].nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                if matches!(
                    cd.kind,
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
                ) {
                    grid[(col, row)].add_xnode(
                        db.get_node("INTF"),
                        &[&format!("INT_INTERFACE_X{x}Y{y}")],
                        db.get_node_naming("INTF"),
                        &[(col, row)],
                    );
                }
            }
            tie_x += 1;
            if cd.kind == ColumnKind::Io
                || cd.tio != ColumnIoKind::None
                || cd.bio != ColumnIoKind::None
            {
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
            let brk = if is_brk { "_BRK" } else { "" };
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
                node.naming = db.get_node_naming(if is_brk { "IOI.BRK" } else { "IOI" });
                tile.add_xnode(
                    db.get_node("INTF.IOI"),
                    &[&format!("LIOI{brk}_X{xl}Y{y}")],
                    db.get_node_naming("INTF.IOI"),
                    &[(col_l, row)],
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
                    tile.add_xnode(
                        db.get_node("INTF"),
                        &[&format!("{cnr}_X{xl}Y{y}")],
                        db.get_node_naming("INTF.CNR"),
                        &[(col_l, row)],
                    );
                } else {
                    let carry = if is_brk { "_CARRY" } else { "" };
                    tile.add_xnode(
                        db.get_node("INTF"),
                        &[&format!("INT_INTERFACE{carry}_X{xl}Y{y}")],
                        db.get_node_naming("INTF"),
                        &[(col_l, row)],
                    );
                }
            }
            let rxl = rxlut[col_l] - 1;
            grid.fill_term_tile(
                (col_l, row),
                "TERM.W",
                "TERM.W",
                format!("{ltt}{txtra}_X{rxl}Y{ry}"),
            );
            let tile = &mut grid[(col_r, row)];
            let mut rtt = "IOI_RTERM";
            if rd.rio {
                let node = &mut tile.nodes[0];
                node.kind = db.get_node("IOI");
                if !is_brk {
                    node.names[def_rt] = format!("IOI_INT_X{xr}Y{y}");
                }
                node.naming = db.get_node_naming(if is_brk { "IOI.BRK" } else { "IOI" });
                tile.add_xnode(
                    db.get_node("INTF.IOI"),
                    &[&format!("RIOI{brk}_X{xr}Y{y}")],
                    db.get_node_naming("INTF.IOI"),
                    &[(col_r, row)],
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
                    tile.add_xnode(
                        db.get_node("INTF"),
                        &[&format!("{cnr}_X{xr}Y{y}")],
                        db.get_node_naming("INTF.CNR"),
                        &[(col_r, row)],
                    );
                } else {
                    let carry = if is_brk { "_CARRY" } else { "" };
                    tile.add_xnode(
                        db.get_node("INTF"),
                        &[&format!("INT_INTERFACE{carry}_X{xr}Y{y}")],
                        db.get_node_naming("INTF"),
                        &[(col_r, row)],
                    );
                }
            }
            let rxr = rxlut[col_r] + 3;
            grid.fill_term_tile(
                (col_r, row),
                "TERM.E",
                "TERM.E",
                format!("{rtt}{txtra}_X{rxr}Y{ry}"),
            );
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
                    let unused = if unused { "_UNUSED" } else { "" };
                    node.kind = db.get_node("IOI");
                    node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
                    node.naming = db.get_node_naming("IOI");
                    tile.add_xnode(
                        db.get_node("INTF.IOI"),
                        &[&format!("BIOI_{io}{unused}_X{x}Y{y}")],
                        db.get_node_naming("INTF.IOI"),
                        &[(col, row)],
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
                    let unused = if unused { "_UNUSED" } else { "" };
                    node.kind = db.get_node("IOI");
                    node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
                    node.naming = db.get_node_naming("IOI");
                    tile.add_xnode(
                        db.get_node("INTF.IOI"),
                        &[&format!("TIOI_{io}{unused}_X{x}Y{y}")],
                        db.get_node_naming("INTF.IOI"),
                        &[(col, row)],
                    );
                }
            }
        }

        let yc = self.row_clk().to_idx();
        grid[(self.col_clk, self.row_clk())].add_xnode(
            db.get_node("INTF"),
            &[&format!("INT_INTERFACE_REGC_X{xc}Y{yc}")],
            db.get_node_naming("INTF.REGC"),
            &[(self.col_clk, self.row_clk())],
        );

        for br in self.get_dcms() {
            for row in [br + 7, br + 8] {
                let y = row.to_idx();
                let tile = &mut grid[(self.col_clk, row)];
                let node = &mut tile.nodes[0];
                node.kind = db.get_node("IOI");
                node.names[def_rt] = format!("IOI_INT_X{xc}Y{y}");
                node.naming = db.get_node_naming("IOI");
                tile.add_xnode(
                    db.get_node("INTF.IOI"),
                    &[&format!("INT_INTERFACE_IOI_X{xc}Y{y}")],
                    db.get_node_naming("INTF"),
                    &[(self.col_clk, row)],
                );
            }
        }

        for br in self.get_plls() {
            let row = br + 7;
            let y = row.to_idx();
            let tile = &mut grid[(self.col_clk, row)];
            tile.add_xnode(
                db.get_node("INTF"),
                &[&format!("INT_INTERFACE_CARRY_X{xc}Y{y}")],
                db.get_node_naming("INTF"),
                &[(self.col_clk, row)],
            );
            let row = br + 8;
            let y = row.to_idx();
            let tile = &mut grid[(self.col_clk, row)];
            let node = &mut tile.nodes[0];
            node.kind = db.get_node("IOI");
            node.names[def_rt] = format!("IOI_INT_X{xc}Y{y}");
            node.naming = db.get_node_naming("IOI");
            tile.add_xnode(
                db.get_node("INTF.IOI"),
                &[&format!("INT_INTERFACE_IOI_X{xc}Y{y}")],
                db.get_node_naming("INTF"),
                &[(self.col_clk, row)],
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
                    grid.fill_term_tile(
                        (col_l, row),
                        "TERM.E",
                        "TERM.E.INTF",
                        format!("INT_RTERM_X{rxl}Y{ry}"),
                    );
                    grid.fill_term_tile(
                        (col_r, row),
                        "TERM.W",
                        "TERM.W.INTF",
                        format!("INT_LTERM_X{rxr}Y{ry}"),
                    );
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
                if !disabled.contains(&DisabledPart::Gtp) {
                    let mut crd = vec![];
                    for dy in 0..16 {
                        crd.push((col_l, row_to - 31 + dy));
                    }
                    for dy in 0..16 {
                        crd.push((col_r, row_to - 31 + dy));
                    }
                    let x = bc.to_idx();
                    let y = row_to.to_idx() - 32;
                    let name = format!("PCIE_TOP_X{x}Y{y}");
                    let node = grid[crd[0]].add_xnode(
                        db.get_node("PCIE"),
                        &[&name],
                        db.get_node_naming("PCIE"),
                        &crd,
                    );
                    node.add_bel(0, "PCIE_X0Y0".to_string());
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
                    grid.fill_term_tile(
                        (col_l, row),
                        "TERM.E",
                        "TERM.E.INTF",
                        format!("INT_RTERM_X{rxl}Y{ry}"),
                    );
                    grid.fill_term_tile(
                        (col_r, row),
                        "TERM.W",
                        "TERM.W.INTF",
                        format!("INT_LTERM_X{rxr}Y{ry}"),
                    );
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
                grid.fill_term_tile(
                    (col_l, row),
                    "TERM.E",
                    "TERM.E.INTF",
                    format!("INT_RTERM_X{rxl}Y{ry}"),
                );
                grid.fill_term_tile(
                    (col_r, row),
                    "TERM.W",
                    "TERM.W.INTF",
                    format!("INT_LTERM_X{rxr}Y{ry}"),
                );
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
                grid.fill_term_tile(
                    (col_l, row),
                    "TERM.E",
                    "TERM.E.INTF",
                    format!("INT_RTERM_X{rxl}Y{ry}"),
                );
                grid.fill_term_tile(
                    (col_r, row),
                    "TERM.W",
                    "TERM.W.INTF",
                    format!("INT_LTERM_X{rxr}Y{ry}"),
                );
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
                _ => {
                    if col == self.col_clk + 1 {
                        ("IOI_BTERM_BUFPLL", "IOI_TTERM_BUFPLL")
                    } else {
                        (
                            if cd.bio == ColumnIoKind::None {
                                "CLB_INT_BTERM"
                            } else {
                                "IOI_BTERM"
                            },
                            "IOI_TTERM",
                        )
                    }
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
                grid.fill_term_tile(
                    (col, row_b),
                    "TERM.S",
                    "TERM.S",
                    format!("{btt}_X{rx}Y{ryb}"),
                );
            }
            grid.fill_term_tile(
                (col, row_t),
                "TERM.N",
                "TERM.N",
                format!("{ttt}_X{rx}Y{ryt}"),
            );
        }

        grid.fill_main_passes();

        let mut sy_base = 2;
        for (col, &cd) in &self.columns {
            if !matches!(
                cd.kind,
                ColumnKind::CleXL | ColumnKind::CleXM | ColumnKind::CleClk
            ) {
                continue;
            }
            if disabled.contains(&DisabledPart::ClbColumn(col)) {
                continue;
            }
            let tb = &grid[(col, RowId::from_idx(0))];
            if !tb.nodes.is_empty() && cd.bio == ColumnIoKind::None {
                sy_base = 0;
                break;
            }
        }
        let mut sx = 0;
        for (col, &cd) in &self.columns {
            if !matches!(
                cd.kind,
                ColumnKind::CleXL | ColumnKind::CleXM | ColumnKind::CleClk
            ) {
                continue;
            }
            if disabled.contains(&DisabledPart::ClbColumn(col)) {
                continue;
            }
            for row in grid.rows() {
                let tile = &mut grid[(col, row)];
                if tile.nodes.len() != 1 {
                    continue;
                }
                let sy = row.to_idx() - sy_base;
                let kind = if cd.kind == ColumnKind::CleXM {
                    "CLEXM"
                } else {
                    "CLEXL"
                };
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(kind),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                node.add_bel(1, format!("SLICE_X{sx1}Y{sy}", sx1 = sx + 1));
            }
            sx += 2;
        }

        let mut bx = 0;
        let mut bby = 0;
        'a: for reg in 0..(self.rows.len() as u32 / 16) {
            for (col, &cd) in &self.columns {
                if cd.kind == ColumnKind::Bram
                    && !disabled.contains(&DisabledPart::BramRegion(col, reg))
                {
                    break 'a;
                }
            }
            bby += 8;
        }
        for (col, &cd) in &self.columns {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            for row in grid.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                let reg = row.to_idx() as u32 / 16;
                if disabled.contains(&DisabledPart::BramRegion(col, reg)) {
                    continue;
                }
                let tile = &mut grid[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let by = row.to_idx() / 2 - bby;
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("BRAMSITE2_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node("BRAM"),
                    &[&name],
                    db.get_node_naming("BRAM"),
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
                node.add_bel(0, format!("RAMB16_X{bx}Y{by}"));
                node.add_bel(1, format!("RAMB8_X{bx}Y{by}"));
                node.add_bel(2, format!("RAMB8_X{bx}Y{by}", by = by + 1));
            }
            bx += 1;
        }

        let mut dx = 0;
        let mut bdy = 0;
        'a: for reg in 0..(self.rows.len() as u32 / 16) {
            for (col, &cd) in &self.columns {
                if matches!(cd.kind, ColumnKind::Dsp | ColumnKind::DspPlus)
                    && !disabled.contains(&DisabledPart::DspRegion(col, reg))
                {
                    break 'a;
                }
            }
            bdy += 4;
        }
        for (col, &cd) in &self.columns {
            if !matches!(cd.kind, ColumnKind::Dsp | ColumnKind::DspPlus) {
                continue;
            }
            for row in grid.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                let reg = row.to_idx() as u32 / 16;
                if disabled.contains(&DisabledPart::DspRegion(col, reg)) {
                    continue;
                }
                if cd.kind == ColumnKind::DspPlus {
                    if row.to_idx() >= self.rows.len() - 16 {
                        continue;
                    }
                    if matches!(self.gts, Gts::Quad(_, _)) && row.to_idx() < 16 {
                        continue;
                    }
                }
                let tile = &mut grid[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let dy = row.to_idx() / 4 - bdy;
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("MACCSITE2_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node("DSP"),
                    &[&name],
                    db.get_node_naming("DSP"),
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
                node.add_bel(0, format!("DSP48_X{dx}Y{dy}"));
            }
            dx += 1;
        }

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
