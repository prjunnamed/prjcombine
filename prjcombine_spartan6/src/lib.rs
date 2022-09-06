use core::cmp::Ordering;
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
    pub rows_pci_ce_split: (RowId, RowId),
    pub rows_bank_split: Option<(RowId, RowId)>,
    pub row_mcb_split: Option<RowId>,
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

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: &'a BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub bonded_ios: Vec<((ColId, RowId), BelId)>,
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

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    tiexlut: EntityVec<ColId, usize>,
    rxlut: EntityVec<ColId, usize>,
    rylut: EntityVec<RowId, usize>,
    ioxlut: EntityVec<ColId, usize>,
    ioylut: EntityVec<RowId, usize>,
    pad_cnt: usize,
    bonded_ios: Vec<((ColId, RowId), BelId)>,
}

impl<'a, 'b> Expander<'a, 'b> {
    fn fill_int(&mut self) {
        for (col, &cd) in &self.grid.columns {
            for row in self.die.rows() {
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
                if row == self.grid.row_clk() && cd.kind == ColumnKind::Io {
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
                self.die.fill_tile(
                    (col, row),
                    "INT",
                    if is_brk { "INT.BRK" } else { "INT" },
                    format!("INT{bram}_X{x}Y{y}"),
                );
                let tie_x = self.tiexlut[col];
                self.die[(col, row)].nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                if matches!(
                    cd.kind,
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
                ) {
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("INTF"),
                        &[&format!("INT_INTERFACE_X{x}Y{y}")],
                        self.db.get_node_naming("INTF"),
                        &[(col, row)],
                    );
                }
            }
        }
    }

    fn fill_lio(&mut self) {
        let col = self.grid.col_lio();
        for (row, &rd) in &self.grid.rows {
            let x = col.to_idx();
            let y = row.to_idx();
            let is_brk = y % 16 == 0 && row != self.grid.row_clk();
            let tile = &mut self.die[(col, row)];
            let mut ltt = "IOI_LTERM";
            if rd.lio {
                self.fill_ioi((col, row), if is_brk { "LIOI_BRK" } else { "LIOI" });
                let mut tk = "LIOB";
                if row == self.grid.row_clk() - 1 {
                    tk = "LIOB_RDY";
                }
                if row == self.grid.row_clk() + 2 {
                    tk = "LIOB_PCI";
                }
                self.fill_iob((col, row), tk, tk);
            } else {
                let cnr = if row == self.grid.row_bio_outer() {
                    Some("LL")
                } else if row == self.grid.row_tio_outer() {
                    Some("UL")
                } else {
                    None
                };
                if let Some(cnr) = cnr {
                    ltt = "CNR_TL_LTERM";
                    let name = format!("{cnr}_X{x}Y{y}");
                    tile.add_xnode(
                        self.db.get_node("INTF"),
                        &[&name],
                        self.db.get_node_naming("INTF.CNR"),
                        &[(col, row)],
                    );
                    let node = tile.add_xnode(
                        self.db.get_node(cnr),
                        &[&name],
                        self.db.get_node_naming(cnr),
                        &[(col, row)],
                    );
                    match cnr {
                        "LL" => {
                            node.add_bel(0, "OCT_CAL_X0Y0".to_string());
                            node.add_bel(1, "OCT_CAL_X0Y1".to_string());
                        }
                        "UL" => {
                            node.add_bel(0, "OCT_CAL_X0Y2".to_string());
                            node.add_bel(1, "OCT_CAL_X0Y3".to_string());
                            node.add_bel(2, "PMV".to_string());
                            node.add_bel(3, "DNA_PORT".to_string());
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let carry = if is_brk { "_CARRY" } else { "" };
                    tile.add_xnode(
                        self.db.get_node("INTF"),
                        &[&format!("INT_INTERFACE{carry}_X{x}Y{y}")],
                        self.db.get_node_naming("INTF"),
                        &[(col, row)],
                    );
                }
            }

            let rx = self.rxlut[col];
            let ry = self.rylut[row];
            let txtra = if row == self.grid.row_clk() - 2 {
                "_LOWER_BOT"
            } else if row == self.grid.row_clk() - 1 {
                "_LOWER_TOP"
            } else if row == self.grid.row_clk() + 2 {
                "_UPPER_BOT"
            } else if row == self.grid.row_clk() + 3 {
                "_UPPER_TOP"
            } else {
                ""
            };
            self.die.fill_term_tile(
                (col, row),
                "TERM.W",
                "TERM.W",
                format!("{ltt}{txtra}_X{rx}Y{ry}", rx = rx - 1),
            );

            if row.to_idx() % 16 == 8 {
                let kind;
                let split;
                let trunk_naming;
                let v_naming;
                if row <= self.grid.row_clk() {
                    match row.cmp(&self.grid.rows_pci_ce_split.0) {
                        Ordering::Less => {
                            kind = "HCLK_IOIL_BOT_DN";
                            v_naming = "PCI_CE_V_BUF_DN";
                            split = false;
                        }
                        Ordering::Equal => {
                            kind = "HCLK_IOIL_BOT_SPLIT";
                            v_naming = "";
                            split = true;
                        }
                        Ordering::Greater => {
                            kind = "HCLK_IOIL_BOT_UP";
                            v_naming = "PCI_CE_V_BUF_UP";
                            split = false;
                        }
                    }
                    trunk_naming = "PCI_CE_TRUNK_BUF_BOT";
                } else {
                    match row.cmp(&self.grid.rows_pci_ce_split.1) {
                        Ordering::Less => {
                            kind = "HCLK_IOIL_TOP_DN";
                            v_naming = "PCI_CE_V_BUF_DN";
                            split = false;
                        }
                        Ordering::Equal => {
                            kind = "HCLK_IOIL_TOP_SPLIT";
                            v_naming = "";
                            split = true;
                        }
                        Ordering::Greater => {
                            kind = "HCLK_IOIL_TOP_UP";
                            v_naming = "PCI_CE_V_BUF_UP";
                            split = false;
                        }
                    }
                    trunk_naming = "PCI_CE_TRUNK_BUF_TOP";
                }
                let name = format!("{kind}_X{x}Y{y}", y = y - 1);
                let tile = &mut self.die[(col, row)];
                let name_term = if row == self.grid.row_clk() {
                    format!("HCLK_IOI_LTERM_BOT25_X{rx}Y{ry}", rx = rx - 1, ry = ry - 2)
                } else {
                    format!("HCLK_IOI_LTERM_X{rx}Y{ry}", rx = rx - 1, ry = ry - 1)
                };
                tile.add_xnode(
                    self.db.get_node("LRIOI_CLK"),
                    &[&name, &name_term],
                    self.db.get_node_naming("LRIOI_CLK.L"),
                    &[],
                );
                if split {
                    tile.add_xnode(
                        self.db.get_node("PCI_CE_SPLIT"),
                        &[&name],
                        self.db.get_node_naming("PCI_CE_SPLIT"),
                        &[],
                    );
                } else {
                    tile.add_xnode(
                        self.db.get_node("PCI_CE_TRUNK_BUF"),
                        &[&name],
                        self.db.get_node_naming(trunk_naming),
                        &[],
                    );
                    if row != self.grid.row_clk() {
                        tile.add_xnode(
                            self.db.get_node("PCI_CE_V_BUF"),
                            &[&name],
                            self.db.get_node_naming(v_naming),
                            &[],
                        );
                    }
                }
            }

            if row == self.grid.row_bio_outer() {
                let name = format!("IOI_PCI_CE_LEFT_X{rx}Y{ry}", ry = ry - 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("PCI_CE_H_BUF"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_H_BUF_CNR"),
                    &[],
                );
            }
            if row == self.grid.row_tio_outer() {
                let name = format!("IOI_PCI_CE_LEFT_X{rx}Y{ry}", ry = ry + 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("PCI_CE_H_BUF"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_H_BUF_CNR"),
                    &[],
                );
            }
        }
    }

    fn fill_rio(&mut self) {
        let col = self.grid.col_rio();
        for (row, &rd) in self.grid.rows.iter().rev() {
            let x = col.to_idx();
            let y = row.to_idx();
            let is_brk = y % 16 == 0 && row != self.grid.row_clk();
            let tile = &mut self.die[(col, row)];
            let mut rtt = "IOI_RTERM";
            if rd.rio {
                self.fill_ioi((col, row), if is_brk { "RIOI_BRK" } else { "RIOI" });
                let mut tk = "RIOB";
                if row == self.grid.row_clk() - 1 {
                    tk = "RIOB_PCI";
                }
                if row == self.grid.row_clk() + 2 {
                    tk = "RIOB_RDY";
                }
                self.fill_iob((col, row), tk, tk);
            } else {
                let cnr = if row == self.grid.row_bio_outer() {
                    Some("LR_LOWER")
                } else if row == self.grid.row_bio_inner() {
                    Some("LR_UPPER")
                } else if row == self.grid.row_tio_inner() {
                    Some("UR_LOWER")
                } else if row == self.grid.row_tio_outer() {
                    Some("UR_UPPER")
                } else {
                    None
                };
                if let Some(cnr) = cnr {
                    rtt = "CNR_TR_RTERM";
                    let name = format!("{cnr}_X{x}Y{y}");
                    tile.add_xnode(
                        self.db.get_node("INTF"),
                        &[&name],
                        self.db.get_node_naming("INTF.CNR"),
                        &[(col, row)],
                    );
                    let node = tile.add_xnode(
                        self.db.get_node(cnr),
                        &[&name],
                        self.db.get_node_naming(cnr),
                        &[(col, row)],
                    );
                    match cnr {
                        "LR_LOWER" => {
                            node.add_bel(0, "OCT_CAL_X1Y0".to_string());
                            node.add_bel(1, "ICAP_X0Y0".to_string());
                            node.add_bel(2, "SPI_ACCESS".to_string());
                        }
                        "LR_UPPER" => {
                            node.add_bel(0, "SUSPEND_SYNC".to_string());
                            node.add_bel(1, "POST_CRC_INTERNAL".to_string());
                            node.add_bel(2, "STARTUP".to_string());
                            node.add_bel(3, "SLAVE_SPI".to_string());
                        }
                        "UR_LOWER" => {
                            node.add_bel(0, "OCT_CAL_X1Y1".to_string());
                            node.add_bel(1, "BSCAN_X0Y2".to_string());
                            node.add_bel(2, "BSCAN_X0Y3".to_string());
                        }
                        "UR_UPPER" => {
                            node.add_bel(0, "BSCAN_X0Y0".to_string());
                            node.add_bel(1, "BSCAN_X0Y1".to_string());
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let carry = if is_brk { "_CARRY" } else { "" };
                    tile.add_xnode(
                        self.db.get_node("INTF"),
                        &[&format!("INT_INTERFACE{carry}_X{x}Y{y}")],
                        self.db.get_node_naming("INTF"),
                        &[(col, row)],
                    );
                }
            }

            let rx = self.rxlut[col];
            let ry = self.rylut[row];
            let txtra = if row == self.grid.row_clk() - 2 {
                "_LOWER_BOT"
            } else if row == self.grid.row_clk() - 1 {
                "_LOWER_TOP"
            } else if row == self.grid.row_clk() + 2 {
                "_UPPER_BOT"
            } else if row == self.grid.row_clk() + 3 {
                "_UPPER_TOP"
            } else {
                ""
            };
            self.die.fill_term_tile(
                (col, row),
                "TERM.E",
                "TERM.E",
                format!("{rtt}{txtra}_X{rx}Y{ry}", rx = rx + 3),
            );

            if row.to_idx() % 16 == 8 {
                let kind;
                let split;
                let trunk_naming;
                let v_naming;
                if row <= self.grid.row_clk() {
                    match row.cmp(&self.grid.rows_pci_ce_split.0) {
                        Ordering::Less => {
                            kind = "HCLK_IOIR_BOT_DN";
                            v_naming = "PCI_CE_V_BUF_DN";
                            split = false;
                        }
                        Ordering::Equal => {
                            kind = "HCLK_IOIR_BOT_SPLIT";
                            v_naming = "";
                            split = true;
                        }
                        Ordering::Greater => {
                            kind = "HCLK_IOIR_BOT_UP";
                            v_naming = "PCI_CE_V_BUF_UP";
                            split = false;
                        }
                    }
                    trunk_naming = "PCI_CE_TRUNK_BUF_BOT";
                } else {
                    match row.cmp(&self.grid.rows_pci_ce_split.1) {
                        Ordering::Less => {
                            kind = "HCLK_IOIR_TOP_DN";
                            v_naming = "PCI_CE_V_BUF_DN";
                            split = false;
                        }
                        Ordering::Equal => {
                            kind = "HCLK_IOIR_TOP_SPLIT";
                            v_naming = "";
                            split = true;
                        }
                        Ordering::Greater => {
                            kind = "HCLK_IOIR_TOP_UP";
                            v_naming = "PCI_CE_V_BUF_UP";
                            split = false;
                        }
                    }
                    trunk_naming = "PCI_CE_TRUNK_BUF_TOP";
                }
                let name = format!("{kind}_X{x}Y{y}", y = y - 1);
                let name_term = if row == self.grid.row_clk() {
                    format!("HCLK_IOI_RTERM_BOT25_X{rx}Y{ry}", rx = rx + 3, ry = ry - 2)
                } else {
                    format!("HCLK_IOI_RTERM_X{rx}Y{ry}", rx = rx + 3, ry = ry - 1)
                };
                let tile = &mut self.die[(col, row)];
                tile.add_xnode(
                    self.db.get_node("LRIOI_CLK"),
                    &[&name, &name_term],
                    self.db.get_node_naming("LRIOI_CLK.R"),
                    &[],
                );
                if split {
                    tile.add_xnode(
                        self.db.get_node("PCI_CE_SPLIT"),
                        &[&name],
                        self.db.get_node_naming("PCI_CE_SPLIT"),
                        &[],
                    );
                } else {
                    tile.add_xnode(
                        self.db.get_node("PCI_CE_TRUNK_BUF"),
                        &[&name],
                        self.db.get_node_naming(trunk_naming),
                        &[],
                    );
                    if row != self.grid.row_clk() && !(self.grid.has_encrypt && row.to_idx() == 8) {
                        tile.add_xnode(
                            self.db.get_node("PCI_CE_V_BUF"),
                            &[&name],
                            self.db.get_node_naming(v_naming),
                            &[],
                        );
                    }
                }
            }

            if row == self.grid.row_bio_outer() {
                let name = format!("IOI_PCI_CE_RIGHT_X{rx}Y{ry}", ry = ry - 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("PCI_CE_H_BUF"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_H_BUF_CNR"),
                    &[],
                );
            }
            if row == self.grid.row_tio_outer() {
                let name = format!("IOI_PCI_CE_RIGHT_X{rx}Y{ry}", ry = ry + 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("PCI_CE_H_BUF"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_H_BUF_CNR"),
                    &[],
                );
            }
        }
    }

    fn fill_tio(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let iob_tk = match cd.tio {
                ColumnIoKind::None => continue,
                ColumnIoKind::Inner => unreachable!(),
                ColumnIoKind::Outer => "TIOB_SINGLE",
                ColumnIoKind::Both => "TIOB",
            };
            for (row, io, unused) in [
                (
                    self.grid.row_tio_outer(),
                    "OUTER",
                    cd.tio == ColumnIoKind::Inner,
                ),
                (
                    self.grid.row_tio_inner(),
                    "INNER",
                    cd.tio == ColumnIoKind::Outer,
                ),
            ] {
                let u = if unused { "_UNUSED" } else { "" };
                let naming = format!("TIOI_{io}{u}");
                self.fill_ioi((col, row), &naming);
                let naming = format!("TIOB_{io}");
                if !unused {
                    self.fill_iob((col, row), iob_tk, &naming);
                }
            }
            let row = self.grid.row_tio_outer();
            let rx = self.rxlut[col] + 1;
            let ry = self.rylut[row] + 1;
            let name = if col == self.grid.col_clk || col == self.grid.col_clk + 1 {
                format!("IOI_TTERM_REGT_X{rx}Y{ry}")
            } else {
                format!("IOI_TTERM_CLB_X{rx}Y{ry}")
            };
            self.die[(col, row)].add_xnode(
                self.db.get_node("BTIOI_CLK"),
                &[&name],
                self.db.get_node_naming("TIOI_CLK"),
                &[],
            );
        }
    }

    fn fill_bio(&mut self) {
        for (col, &cd) in self.grid.columns.iter().rev() {
            let iob_tk = match cd.bio {
                ColumnIoKind::None => continue,
                ColumnIoKind::Inner => "BIOB_SINGLE",
                ColumnIoKind::Outer => "BIOB_SINGLE_ALT",
                ColumnIoKind::Both => "BIOB",
            };
            for (row, io, unused) in [
                (
                    self.grid.row_bio_outer(),
                    "OUTER",
                    cd.bio == ColumnIoKind::Inner,
                ),
                (
                    self.grid.row_bio_inner(),
                    "INNER",
                    cd.bio == ColumnIoKind::Outer,
                ),
            ] {
                let u = if unused { "_UNUSED" } else { "" };
                let naming = format!("BIOI_{io}{u}");
                self.fill_ioi((col, row), &naming);
                let naming = format!("BIOB_{io}");
                if !unused {
                    self.fill_iob((col, row), iob_tk, &naming);
                }
            }
            let row = self.grid.row_bio_outer();
            let rx = self.rxlut[col] + 1;
            let ry = self.rylut[row] - 1;
            let name = if col == self.grid.col_clk || col == self.grid.col_clk + 1 {
                format!("IOI_BTERM_REGB_X{rx}Y{ry}")
            } else {
                format!("IOI_BTERM_CLB_X{rx}Y{ry}")
            };
            self.die[(col, row)].add_xnode(
                self.db.get_node("BTIOI_CLK"),
                &[&name],
                self.db.get_node_naming("BIOI_CLK"),
                &[],
            );
        }
    }

    fn fill_mcb(&mut self) {
        if self.disabled.contains(&DisabledPart::Mcb) {
            return;
        }
        let mut mx = 0;
        for (col, &cd) in &self.grid.columns {
            if cd.kind != ColumnKind::Io {
                continue;
            }
            let x = col.to_idx();
            let mut my = 1;
            for mcb in &self.grid.mcbs {
                let row = mcb.row_mcb;
                let mut crds = vec![];
                for dy in 0..12 {
                    crds.push((col, row + dy));
                }
                for urow in mcb.row_mui {
                    for dy in 0..2 {
                        crds.push((col, urow + dy));
                    }
                }
                let tk = if self.grid.rows.len() % 32 == 16 {
                    "MCB_L_BOT"
                } else {
                    "MCB_L"
                };
                let name = format!("{tk}_X{x}Y{y}", y = row.to_idx() + 6);
                let name_hclk = format!("MCB_HCLK_X{x}Y{y}", y = row.to_idx() - 1);
                let name_clkpn = format!("MCB_CAP_CLKPN_X{x}Y{y}", y = mcb.iop_clk.to_idx());
                let name_ldqs = format!("MCB_INT_DQI_X{x}Y{y}", y = mcb.iop_dqs[0].to_idx());
                let name_udqs = format!("MCB_INT_DQI_X{x}Y{y}", y = mcb.iop_dqs[1].to_idx());
                let name_mui0r = format!("MCB_MUI0R_X{x}Y{y}", y = mcb.row_mui[0].to_idx());
                let name_mui0w = format!("MCB_MUI0W_X{x}Y{y}", y = mcb.row_mui[1].to_idx());
                let name_mui1r = format!("MCB_MUI1R_X{x}Y{y}", y = mcb.row_mui[2].to_idx());
                let name_mui1w = format!("MCB_MUI1W_X{x}Y{y}", y = mcb.row_mui[3].to_idx());
                let name_mui2 = format!("MCB_MUI2_X{x}Y{y}", y = mcb.row_mui[4].to_idx());
                let name_mui3 = format!("MCB_MUI3_X{x}Y{y}", y = mcb.row_mui[5].to_idx());
                let name_mui4 = format!("MCB_MUI4_X{x}Y{y}", y = mcb.row_mui[6].to_idx());
                let name_mui5 = format!("MCB_MUI5_X{x}Y{y}", y = mcb.row_mui[7].to_idx());
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("MCB"),
                    &[
                        &name,
                        &name_hclk,
                        &name_clkpn,
                        &name_ldqs,
                        &name_udqs,
                        &name_mui0r,
                        &name_mui0w,
                        &name_mui1r,
                        &name_mui1w,
                        &name_mui2,
                        &name_mui3,
                        &name_mui4,
                        &name_mui5,
                    ],
                    self.db.get_node_naming(tk),
                    &crds,
                );
                node.add_bel(0, format!("MCB_X{mx}Y{my}"));
                node.add_bel(
                    1,
                    format!(
                        "TIEOFF_X{x}Y{y}",
                        x = self.tiexlut[col] + 1,
                        y = mcb.iop_clk.to_idx() * 2 + 1
                    ),
                );
                node.add_bel(
                    2,
                    format!(
                        "TIEOFF_X{x}Y{y}",
                        x = self.tiexlut[col] + 1,
                        y = mcb.iop_dqs[0].to_idx() * 2 + 1
                    ),
                );
                node.add_bel(
                    3,
                    format!(
                        "TIEOFF_X{x}Y{y}",
                        x = self.tiexlut[col] + 1,
                        y = mcb.iop_dqs[1].to_idx() * 2 + 1
                    ),
                );
                my += 2;
            }
            mx += 1;
        }
    }

    fn fill_pcilogic(&mut self) {
        let row = self.grid.row_clk();
        let y = row.to_idx();
        let ry = self.rylut[row] - 1;

        let col = self.grid.col_lio();
        let x = col.to_idx();
        let name;
        let name_ioi;
        let rx = self.rxlut[col] - 2;
        if self.grid.rows.len() % 32 == 16 {
            name = format!("REGH_LIOI_INT_BOT25_X{x}Y{y}");
            name_ioi = format!("REGH_IOI_BOT25_X{x}Y{y}");
        } else {
            name = format!("REGH_LIOI_INT_X{x}Y{y}", y = y - 1);
            name_ioi = format!("REGH_IOI_X{x}Y{y}", y = y - 1);
        }
        let name_reg = format!("REG_L_X{rx}Y{ry}");
        let name_int = format!("INT_X{x}Y{y}");
        let node = self.die[(col, row)].add_xnode(
            self.db.get_node("PCILOGICSE"),
            &[&name, &name_reg, &name_ioi, &name_int],
            self.db.get_node_naming("PCILOGICSE_L"),
            &[(col, row)],
        );
        node.add_bel(0, "PCILOGIC_X0Y0".to_string());

        let col = self.grid.col_rio();
        let rx = self.rxlut[col] + 3;
        let x = col.to_idx();
        let name = if self.grid.rows.len() % 32 == 16 {
            format!("REGH_RIOI_BOT25_X{x}Y{y}")
        } else {
            format!("REGH_RIOI_X{x}Y{y}", y = y - 1)
        };
        let name_reg = format!("REG_R_X{rx}Y{ry}");
        let name_int = format!("INT_X{x}Y{y}");
        let node = self.die[(col, row)].add_xnode(
            self.db.get_node("PCILOGICSE"),
            &[&name, &name_reg, &name_int],
            self.db.get_node_naming("PCILOGICSE_R"),
            &[(col, row)],
        );
        node.add_bel(0, "PCILOGIC_X1Y0".to_string());
    }

    fn fill_clkc(&mut self) {
        let col = self.grid.col_clk;
        let row = self.grid.row_clk();
        let x = col.to_idx();
        let y = row.to_idx();
        self.die[(col, row)].add_xnode(
            self.db.get_node("INTF"),
            &[&format!("INT_INTERFACE_REGC_X{x}Y{y}")],
            self.db.get_node_naming("INTF.REGC"),
            &[(col, row)],
        );
    }

    fn fill_dcms(&mut self) {
        let col = self.grid.col_clk;
        let x = col.to_idx();
        let def_rt = NodeRawTileId::from_idx(0);
        for br in self.grid.get_dcms() {
            for row in [br + 7, br + 8] {
                let y = row.to_idx();
                let tile = &mut self.die[(col, row)];
                let node = &mut tile.nodes[0];
                node.kind = self.db.get_node("INT.IOI");
                node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
                node.naming = self.db.get_node_naming("INT.IOI");
                tile.add_xnode(
                    self.db.get_node("INTF.IOI"),
                    &[&format!("INT_INTERFACE_IOI_X{x}Y{y}")],
                    self.db.get_node_naming("INTF"),
                    &[(col, row)],
                );
            }
        }
    }

    fn fill_plls(&mut self) {
        let col = self.grid.col_clk;
        let x = col.to_idx();
        let def_rt = NodeRawTileId::from_idx(0);
        for br in self.grid.get_plls() {
            let row = br + 7;
            let y = row.to_idx();
            let tile = &mut self.die[(col, row)];
            tile.add_xnode(
                self.db.get_node("INTF"),
                &[&format!("INT_INTERFACE_CARRY_X{x}Y{y}")],
                self.db.get_node_naming("INTF"),
                &[(col, row)],
            );
            let row = br + 8;
            let y = row.to_idx();
            let tile = &mut self.die[(col, row)];
            let node = &mut tile.nodes[0];
            node.kind = self.db.get_node("INT.IOI");
            node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
            node.naming = self.db.get_node_naming("INT.IOI");
            tile.add_xnode(
                self.db.get_node("INTF.IOI"),
                &[&format!("INT_INTERFACE_IOI_X{x}Y{y}")],
                self.db.get_node_naming("INTF"),
                &[(col, row)],
            );
        }
    }

    fn fill_gts(&mut self) {
        match self.grid.gts {
            Gts::Single(bc) | Gts::Double(bc, _) | Gts::Quad(bc, _) => {
                let row_gt_mid = RowId::from_idx(self.grid.rows.len() - 8);
                let row_gt_bot = row_gt_mid - 8;
                let row_pcie_bot = row_gt_bot - 16;
                self.die.nuke_rect(bc - 6, row_gt_mid, 11, 8);
                self.die.nuke_rect(bc - 4, row_gt_bot, 7, 8);
                self.die.nuke_rect(bc - 1, row_pcie_bot, 3, 16);
                let col_l = bc - 7;
                let col_r = bc + 5;
                let rxl = self.rxlut[col_l] + 6;
                let rxr = self.rxlut[col_r] - 1;
                for dy in 0..8 {
                    let row = self.grid.row_tio_outer() - 7 + dy;
                    let ry = self.rylut[row];
                    self.die.fill_term_tile(
                        (col_l, row),
                        "TERM.E",
                        "TERM.E.INTF",
                        format!("INT_RTERM_X{rxl}Y{ry}"),
                    );
                    self.die.fill_term_tile(
                        (col_r, row),
                        "TERM.W",
                        "TERM.W.INTF",
                        format!("INT_LTERM_X{rxr}Y{ry}"),
                    );
                }
                let col_l = bc - 5;
                let col_r = bc + 3;
                for dy in 0..8 {
                    let row = row_gt_bot + dy;
                    let ry = self.rylut[row];
                    let rxl = self.rxlut[col_l] + 1;
                    let rxr = self.rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    self.fill_intf_rterm((col_l, row), tile_l);
                    self.fill_intf_lterm((col_r, row), tile_r, is_brk);
                }
                let col_l = bc - 2;
                let col_r = bc + 2;
                for dy in 0..16 {
                    let row = row_pcie_bot + dy;
                    let ry = self.rylut[row];
                    let rxl = self.rxlut[col_l] + 1;
                    let rxr = self.rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    self.fill_intf_rterm((col_l, row), tile_l);
                    self.fill_intf_lterm((col_r, row), tile_r, is_brk);
                }
                if !self.disabled.contains(&DisabledPart::Gtp) {
                    let mut crd = vec![];
                    for dy in 0..16 {
                        crd.push((col_l, row_pcie_bot + dy));
                    }
                    for dy in 0..16 {
                        crd.push((col_r, row_pcie_bot + dy));
                    }
                    let x = bc.to_idx();
                    let y = row_pcie_bot.to_idx() - 1;
                    let name = format!("PCIE_TOP_X{x}Y{y}");
                    let node = self.die[crd[0]].add_xnode(
                        self.db.get_node("PCIE"),
                        &[&name],
                        self.db.get_node_naming("PCIE"),
                        &crd,
                    );
                    node.add_bel(0, "PCIE_X0Y0".to_string());
                }
            }
            _ => (),
        }
        match self.grid.gts {
            Gts::Double(_, bc) | Gts::Quad(_, bc) => {
                let row_gt_mid = RowId::from_idx(self.grid.rows.len() - 8);
                let row_gt_bot = row_gt_mid - 8;
                self.die.nuke_rect(bc - 4, row_gt_mid, 11, 8);
                self.die.nuke_rect(bc - 2, row_gt_bot, 8, 8);
                let col_l = bc - 5;
                let col_r = bc + 7;
                for dy in 0..8 {
                    let row = row_gt_mid + dy;
                    let ry = self.rylut[row];
                    let rxl = self.rxlut[col_l] + 5;
                    let rxr = self.rxlut[col_r] - 2;
                    self.die.fill_term_tile(
                        (col_l, row),
                        "TERM.E",
                        "TERM.E.INTF",
                        format!("INT_RTERM_X{rxl}Y{ry}"),
                    );
                    self.die.fill_term_tile(
                        (col_r, row),
                        "TERM.W",
                        "TERM.W.INTF",
                        format!("INT_LTERM_X{rxr}Y{ry}"),
                    );
                }
                let col_l = bc - 3;
                let col_r = bc + 6;
                for dy in 0..8 {
                    let row = row_gt_bot + dy;
                    let ry = self.rylut[row];
                    let rxl = self.rxlut[col_l] + 1;
                    let rxr = self.rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    self.fill_intf_rterm((col_l, row), tile_l);
                    self.fill_intf_lterm((col_r, row), tile_r, is_brk);
                }
            }
            _ => (),
        }
        if let Gts::Quad(bcl, bcr) = self.grid.gts {
            let row_gt_bot = RowId::from_idx(0);
            let row_gt_mid = RowId::from_idx(8);
            self.die.nuke_rect(bcl - 6, row_gt_bot, 11, 8);
            self.die.nuke_rect(bcl - 4, row_gt_mid, 7, 8);
            self.die.nuke_rect(bcr - 4, row_gt_bot, 11, 8);
            self.die.nuke_rect(bcr - 2, row_gt_mid, 8, 8);
            let col_l = bcl - 7;
            let col_r = bcl + 5;
            for dy in 0..8 {
                let row = row_gt_bot + dy;
                let ry = self.rylut[row];
                let rxl = self.rxlut[col_l] + 6;
                let rxr = self.rxlut[col_r] - 1;
                self.die.fill_term_tile(
                    (col_l, row),
                    "TERM.E",
                    "TERM.E.INTF",
                    format!("INT_RTERM_X{rxl}Y{ry}"),
                );
                self.die.fill_term_tile(
                    (col_r, row),
                    "TERM.W",
                    "TERM.W.INTF",
                    format!("INT_LTERM_X{rxr}Y{ry}"),
                );
            }
            let col_l = bcl - 5;
            let col_r = bcl + 3;
            for dy in 0..8 {
                let row = row_gt_mid + dy;
                let ry = self.rylut[row];
                let rxl = self.rxlut[col_l] + 1;
                let rxr = self.rxlut[col_r] - 1;
                let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                self.fill_intf_rterm((col_l, row), tile_l);
                self.fill_intf_lterm((col_r, row), tile_r, false);
            }
            let col_l = bcr - 5;
            let col_r = bcr + 7;
            for dy in 0..8 {
                let row = row_gt_bot + dy;
                let ry = self.rylut[row];
                let rxl = self.rxlut[col_l] + 5;
                let rxr = self.rxlut[col_r] - 2;
                self.die.fill_term_tile(
                    (col_l, row),
                    "TERM.E",
                    "TERM.E.INTF",
                    format!("INT_RTERM_X{rxl}Y{ry}"),
                );
                self.die.fill_term_tile(
                    (col_r, row),
                    "TERM.W",
                    "TERM.W.INTF",
                    format!("INT_LTERM_X{rxr}Y{ry}"),
                );
            }
            let col_l = bcr - 3;
            let col_r = bcr + 6;
            for dy in 0..8 {
                let row = row_gt_mid + dy;
                let ry = self.rylut[row];
                let rxl = self.rxlut[col_l] + 1;
                let rxr = self.rxlut[col_r] - 1;
                let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                self.fill_intf_rterm((col_l, row), tile_l);
                self.fill_intf_lterm((col_r, row), tile_r, false);
            }
        }
    }

    fn fill_btterm(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let (btt, ttt) = match cd.kind {
                ColumnKind::Io => ("CNR_BR_BTERM", "CNR_TR_TTERM"),
                ColumnKind::Bram => ("", "RAMB_TOP_TTERM"),
                ColumnKind::Dsp | ColumnKind::DspPlus => ("DSP_INT_BTERM", "DSP_INT_TTERM"),
                _ => {
                    if col == self.grid.col_clk + 1 {
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
            let rx = self.rxlut[col];
            let ryb = self.rylut[self.grid.row_bio_outer()] - 1;
            let mut row_b = self.grid.row_bio_outer();
            while self.die[(col, row_b)].nodes.is_empty() {
                row_b += 1;
            }
            if !btt.is_empty() {
                self.die.fill_term_tile(
                    (col, row_b),
                    "TERM.S",
                    "TERM.S",
                    format!("{btt}_X{rx}Y{ryb}"),
                );
            }

            let ryt = self.rylut[self.grid.row_tio_outer()] + 1;
            let mut row_t = self.grid.row_tio_outer();
            while self.die[(col, row_t)].nodes.is_empty() {
                row_t -= 1;
            }
            self.die.fill_term_tile(
                (col, row_t),
                "TERM.N",
                "TERM.N",
                format!("{ttt}_X{rx}Y{ryt}"),
            );
        }
    }

    fn fill_cle(&mut self) {
        let mut sy_base = 2;
        for (col, &cd) in &self.grid.columns {
            if !matches!(
                cd.kind,
                ColumnKind::CleXL | ColumnKind::CleXM | ColumnKind::CleClk
            ) {
                continue;
            }
            if self.disabled.contains(&DisabledPart::ClbColumn(col)) {
                continue;
            }
            let tb = &self.die[(col, RowId::from_idx(0))];
            if !tb.nodes.is_empty() && cd.bio == ColumnIoKind::None {
                sy_base = 0;
                break;
            }
        }
        let mut sx = 0;
        for (col, &cd) in &self.grid.columns {
            if !matches!(
                cd.kind,
                ColumnKind::CleXL | ColumnKind::CleXM | ColumnKind::CleClk
            ) {
                continue;
            }
            if self.disabled.contains(&DisabledPart::ClbColumn(col)) {
                continue;
            }
            for row in self.die.rows() {
                let tile = &mut self.die[(col, row)];
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
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                node.add_bel(1, format!("SLICE_X{sx1}Y{sy}", sx1 = sx + 1));
            }
            sx += 2;
        }
    }

    fn fill_bram(&mut self) {
        let mut bx = 0;
        let mut bby = 0;
        'a: for reg in 0..(self.grid.rows.len() as u32 / 16) {
            for (col, &cd) in &self.grid.columns {
                if cd.kind == ColumnKind::Bram
                    && !self.disabled.contains(&DisabledPart::BramRegion(col, reg))
                {
                    break 'a;
                }
            }
            bby += 8;
        }
        for (col, &cd) in &self.grid.columns {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                let reg = row.to_idx() as u32 / 16;
                if self.disabled.contains(&DisabledPart::BramRegion(col, reg)) {
                    continue;
                }
                let tile = &mut self.die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let by = row.to_idx() / 2 - bby;
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("BRAMSITE2_X{x}Y{y}");
                let node = tile.add_xnode(
                    self.db.get_node("BRAM"),
                    &[&name],
                    self.db.get_node_naming("BRAM"),
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
                node.add_bel(0, format!("RAMB16_X{bx}Y{by}"));
                node.add_bel(1, format!("RAMB8_X{bx}Y{by}"));
                node.add_bel(2, format!("RAMB8_X{bx}Y{by}", by = by + 1));
            }
            bx += 1;

            let lr = if col < self.grid.col_clk { 'L' } else { 'R' };
            let rx = self.rxlut[col];

            let row = self.grid.row_bio_outer();
            let ry = self.rylut[row];
            let name = format!("BRAM_BOT_BTERM_{lr}_X{rx}Y{ry}", rx = rx + 2, ry = ry - 1);
            self.die[(col, row)].add_xnode(
                self.db.get_node("PCI_CE_H_BUF"),
                &[&name],
                self.db.get_node_naming("PCI_CE_H_BUF_BRAM"),
                &[],
            );

            let row = self.grid.row_tio_outer();
            let ry = self.rylut[row];
            let name = format!("BRAM_TOP_TTERM_{lr}_X{rx}Y{ry}", rx = rx + 2, ry = ry + 1);
            self.die[(col, row)].add_xnode(
                self.db.get_node("PCI_CE_H_BUF"),
                &[&name],
                self.db.get_node_naming("PCI_CE_H_BUF_BRAM"),
                &[],
            );
        }
    }

    fn fill_dsp(&mut self) {
        let mut dx = 0;
        let mut bdy = 0;
        'a: for reg in 0..(self.grid.rows.len() as u32 / 16) {
            for (col, &cd) in &self.grid.columns {
                if matches!(cd.kind, ColumnKind::Dsp | ColumnKind::DspPlus)
                    && !self.disabled.contains(&DisabledPart::DspRegion(col, reg))
                {
                    break 'a;
                }
            }
            bdy += 4;
        }
        for (col, &cd) in &self.grid.columns {
            if !matches!(cd.kind, ColumnKind::Dsp | ColumnKind::DspPlus) {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                let reg = row.to_idx() as u32 / 16;
                if self.disabled.contains(&DisabledPart::DspRegion(col, reg)) {
                    continue;
                }
                if cd.kind == ColumnKind::DspPlus {
                    if row.to_idx() >= self.grid.rows.len() - 16 {
                        continue;
                    }
                    if matches!(self.grid.gts, Gts::Quad(_, _)) && row.to_idx() < 16 {
                        continue;
                    }
                }
                let tile = &mut self.die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let dy = row.to_idx() / 4 - bdy;
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("MACCSITE2_X{x}Y{y}");
                let node = tile.add_xnode(
                    self.db.get_node("DSP"),
                    &[&name],
                    self.db.get_node_naming("DSP"),
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
                node.add_bel(0, format!("DSP48_X{dx}Y{dy}"));
            }
            dx += 1;

            let lr = if col < self.grid.col_clk { 'L' } else { 'R' };
            let rx = self.rxlut[col];

            let row = self.grid.row_bio_outer();
            let ry = self.rylut[row];
            let name = format!("DSP_BOT_BTERM_{lr}_X{rx}Y{ry}", rx = rx + 2, ry = ry - 1);
            self.die[(col, row)].add_xnode(
                self.db.get_node("PCI_CE_H_BUF"),
                &[&name],
                self.db.get_node_naming("PCI_CE_H_BUF_DSP"),
                &[],
            );

            let row = self.grid.row_tio_outer();
            let ry = self.rylut[row];
            let name = format!("DSP_TOP_TTERM_{lr}_X{rx}Y{ry}", rx = rx + 2, ry = ry + 1);
            self.die[(col, row)].add_xnode(
                self.db.get_node("PCI_CE_H_BUF"),
                &[&name],
                self.db.get_node_naming("PCI_CE_H_BUF_DSP"),
                &[],
            );
        }
    }

    fn fill_hclk(&mut self) {
        for col in self.die.cols() {
            for row in self.die.rows() {
                let crow = RowId::from_idx(if row.to_idx() % 16 < 8 {
                    row.to_idx() / 16 * 16 + 7
                } else {
                    row.to_idx() / 16 * 16 + 8
                });
                self.die[(col, row)].clkroot = (col, crow);
            }
        }
    }

    fn fill_ioi(&mut self, crd: Coord, naming: &str) {
        let x = crd.0.to_idx();
        let y = crd.1.to_idx();
        let tile = &mut self.die[crd];
        let node = &mut tile.nodes[0];
        let def_rt = NodeRawTileId::from_idx(0);
        node.kind = self.db.get_node("INT.IOI");
        let is_brk = y % 16 == 0 && crd.1 != self.grid.row_clk() && y != 0;
        if !is_brk {
            if crd.0 == self.grid.col_lio() {
                node.names[def_rt] = format!("LIOI_INT_X{x}Y{y}");
            } else {
                node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
            }
        }
        node.naming = self
            .db
            .get_node_naming(if is_brk { "INT.IOI.BRK" } else { "INT.IOI" });
        let name = format!("{naming}_X{x}Y{y}");
        tile.add_xnode(
            self.db.get_node("INTF.IOI"),
            &[&name],
            self.db.get_node_naming("INTF.IOI"),
            &[crd],
        );
        let node = tile.add_xnode(
            self.db.get_node("IOI"),
            &[&name],
            self.db.get_node_naming(naming),
            &[crd],
        );
        let iox = self.ioxlut[crd.0];
        let ioy = self.ioylut[crd.1];
        let tiex = self.tiexlut[crd.0] + 1;
        node.add_bel(0, format!("ILOGIC_X{iox}Y{y}", y = ioy * 2 + 1));
        node.add_bel(1, format!("ILOGIC_X{iox}Y{y}", y = ioy * 2));
        node.add_bel(2, format!("OLOGIC_X{iox}Y{y}", y = ioy * 2 + 1));
        node.add_bel(3, format!("OLOGIC_X{iox}Y{y}", y = ioy * 2));
        node.add_bel(4, format!("IODELAY_X{iox}Y{y}", y = ioy * 2 + 1));
        node.add_bel(5, format!("IODELAY_X{iox}Y{y}", y = ioy * 2));
        node.add_bel(6, format!("TIEOFF_X{tiex}Y{y}", y = y * 2));
    }

    fn fill_iob(&mut self, crd: Coord, tk: &str, naming: &str) {
        let x = crd.0.to_idx();
        let mut y = crd.1.to_idx();
        if tk.starts_with('T') {
            y = self.grid.row_tio_outer().to_idx();
        }
        if tk.starts_with('B') {
            y = 0;
        }
        let tile = &mut self.die[crd];
        let name = format!("{tk}_X{x}Y{y}");
        let node = tile.add_xnode(
            self.db.get_node("IOB"),
            &[&name],
            self.db.get_node_naming(naming),
            &[],
        );
        node.add_bel(0, format!("PAD{i}", i = self.pad_cnt));
        node.add_bel(1, format!("PAD{i}", i = self.pad_cnt + 1));
        self.pad_cnt += 2;
        self.bonded_ios.push((crd, BelId::from_idx(0)));
        self.bonded_ios.push((crd, BelId::from_idx(1)));
    }

    fn fill_intf_rterm(&mut self, crd: Coord, name: String) {
        self.die
            .fill_term_tile(crd, "TERM.E", "TERM.E.INTF", name.clone());
        let tile = &mut self.die[crd];
        tile.nodes.truncate(1);
        tile.add_xnode(
            self.db.get_node("INTF"),
            &[&name],
            self.db.get_node_naming("INTF.RTERM"),
            &[crd],
        );
    }

    fn fill_intf_lterm(&mut self, crd: Coord, name: String, is_brk: bool) {
        self.die
            .fill_term_tile(crd, "TERM.W", "TERM.W.INTF", name.clone());
        let tile = &mut self.die[crd];
        tile.nodes.truncate(1);
        tile.nodes[0].naming =
            self.db
                .get_node_naming(if is_brk { "INT.TERM.BRK" } else { "INT.TERM" });
        tile.add_xnode(
            self.db.get_node("INTF"),
            &[&name],
            self.db.get_node_naming("INTF.LTERM"),
            &[crd],
        );
    }
}

impl Grid {
    pub fn col_lio(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_rio(&self) -> ColId {
        ColId::from_idx(self.columns.len() - 1)
    }

    pub fn row_bio_outer(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_bio_inner(&self) -> RowId {
        RowId::from_idx(1)
    }

    pub fn row_tio_outer(&self) -> RowId {
        RowId::from_idx(self.rows.len() - 1)
    }

    pub fn row_tio_inner(&self) -> RowId {
        RowId::from_idx(self.rows.len() - 2)
    }

    pub fn get_mcb(&self, row: RowId) -> &Mcb {
        for mcb in &self.mcbs {
            if mcb.row_mcb == row {
                return mcb;
            }
        }
        unreachable!()
    }

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

    fn make_ioxlut(&self) -> EntityVec<ColId, usize> {
        let mut res = EntityVec::new();
        let mut iox = 0;
        for &cd in self.columns.values() {
            res.push(iox);
            if cd.kind == ColumnKind::Io
                || cd.bio != ColumnIoKind::None
                || cd.tio != ColumnIoKind::None
            {
                iox += 1;
            }
        }
        res
    }

    fn make_ioylut(&self) -> EntityVec<RowId, usize> {
        let mut res = EntityVec::new();
        let mut ioy = 0;
        for (row, &rd) in &self.rows {
            res.push(ioy);
            if row == self.row_bio_outer()
                || row == self.row_bio_inner()
                || row == self.row_tio_inner()
                || row == self.row_tio_outer()
                || rd.lio
                || rd.rio
            {
                ioy += 1;
            }
        }
        res
    }

    fn make_tiexlut(&self) -> EntityVec<ColId, usize> {
        let mut res = EntityVec::new();
        let mut tie_x = 0;
        for &cd in self.columns.values() {
            res.push(tie_x);
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
        }
        res
    }

    fn make_rxlut(&self) -> EntityVec<ColId, usize> {
        let mut res = EntityVec::new();
        let mut rx = 2;
        for &cd in self.columns.values() {
            res.push(rx);
            match cd.kind {
                ColumnKind::CleXL | ColumnKind::CleXM => rx += 2,
                ColumnKind::CleClk => rx += 4,
                _ => rx += 3,
            }
        }
        res
    }

    fn make_rylut(&self) -> EntityVec<RowId, usize> {
        let mut res = EntityVec::new();
        let mut ry = 2;
        for row in self.rows.ids() {
            if row == self.row_clk() {
                ry += 1;
            }
            if row.to_idx() % 16 == 8 {
                ry += 1;
            }
            res.push(ry);
            ry += 1;
        }
        res
    }

    pub fn expand_grid<'a>(
        &'a self,
        db: &'a IntDb,
        disabled: &'a BTreeSet<DisabledPart>,
    ) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, die) = egrid.add_die(self.columns.len(), self.rows.len());

        let mut expander = Expander {
            grid: self,
            db,
            disabled,
            die,
            rxlut: self.make_rxlut(),
            rylut: self.make_rylut(),
            tiexlut: self.make_tiexlut(),
            ioxlut: self.make_ioxlut(),
            ioylut: self.make_ioylut(),
            pad_cnt: 1,
            bonded_ios: vec![],
        };

        expander.fill_int();
        expander.fill_tio();
        expander.fill_rio();
        expander.fill_bio();
        expander.fill_lio();
        expander.fill_mcb();
        expander.fill_pcilogic();
        expander.fill_clkc();
        expander.fill_dcms();
        expander.fill_plls();
        expander.fill_gts();
        expander.fill_btterm();
        expander.die.fill_main_passes();
        expander.fill_cle();
        expander.fill_bram();
        expander.fill_dsp();
        expander.fill_hclk();

        let bonded_ios = expander.bonded_ios;

        ExpandedDevice {
            grid: self,
            disabled,
            egrid,
            bonded_ios,
        }
    }
}
