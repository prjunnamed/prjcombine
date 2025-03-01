#![recursion_limit = "1024"]

use prjcombine_interconnect::{
    dir::DirPartMap,
    grid::{ColId, DieId, RowId},
};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{
    bels,
    bond::PsPin,
    chip::{CfgRowKind, ChipKind, GtKind, RegId},
    expanded::{ExpandedDevice, IoCoord},
    gtz::GtzIntColId,
};
use unnamed_entity::{EntityId, EntityVec};

mod virtex4;
mod virtex5;
mod virtex6;
mod virtex7;

#[derive(Debug)]
pub struct SysMon<'a> {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
    pub bank: u32,
    pub pad_vp: &'a str,
    pub pad_vn: &'a str,
    pub vaux: Vec<Option<(IoCoord, IoCoord)>>,
}

#[derive(Debug)]
pub struct Gt<'a> {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
    pub bank: u32,
    pub kind: GtKind,
    pub pads_clk: Vec<(&'a str, &'a str)>,
    pub pads_tx: Vec<(&'a str, &'a str)>,
    pub pads_rx: Vec<(&'a str, &'a str)>,
}

#[derive(Debug)]
pub struct ExpandedNamedGtz {
    pub tile: String,
    pub clk_tile: String,
    pub int_tiles: EntityVec<GtzIntColId, String>,
    pub bel: String,
    pub pads_clk: Vec<(String, String)>,
    pub pads_tx: Vec<(String, String)>,
    pub pads_rx: Vec<(String, String)>,
}

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub gtz: DirPartMap<ExpandedNamedGtz>,
}

impl ExpandedNamedDevice<'_> {
    pub fn get_io_name(&self, io: IoCoord) -> &str {
        self.ngrid
            .get_bel_name((io.die, (io.col, io.row), bels::IOB[io.iob.to_idx()]))
            .unwrap()
    }

    pub fn get_sysmons(&self) -> Vec<SysMon<'_>> {
        let mut res = vec![];
        for (die, chip) in &self.edev.chips {
            match self.edev.kind {
                ChipKind::Virtex4 => {
                    let mut idx = 0;
                    for &(row, kind) in &chip.rows_cfg {
                        let col = self.edev.col_cfg;
                        if kind != CfgRowKind::Sysmon {
                            continue;
                        }
                        res.push(SysMon {
                            die,
                            col,
                            row,
                            bank: idx,
                            pad_vp: self
                                .ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_VP))
                                .unwrap(),
                            pad_vn: self
                                .ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_VN))
                                .unwrap(),
                            vaux: (0..8)
                                .map(|idx| self.edev.get_sysmon_vaux(die, col, row, idx))
                                .collect(),
                        });
                        idx += 1;
                    }
                }
                ChipKind::Virtex5 => {
                    let col = self.edev.col_cfg;
                    let row = chip.row_reg_hclk(chip.reg_cfg - 1);
                    res.push(SysMon {
                        die,
                        col,
                        row,
                        bank: 0,
                        pad_vp: self
                            .ngrid
                            .get_bel_name((die, (col, row), bels::IPAD_VP))
                            .unwrap(),
                        pad_vn: self
                            .ngrid
                            .get_bel_name((die, (col, row), bels::IPAD_VN))
                            .unwrap(),
                        vaux: (0..16)
                            .map(|idx| self.edev.get_sysmon_vaux(die, col, row, idx))
                            .collect(),
                    });
                }
                ChipKind::Virtex6 => {
                    let col = self.edev.col_cfg;
                    let row = chip.row_reg_bot(chip.reg_cfg);
                    res.push(SysMon {
                        die,
                        col,
                        row,
                        bank: 0,
                        pad_vp: self
                            .ngrid
                            .get_bel_name((die, (col, row), bels::IPAD_VP))
                            .unwrap(),
                        pad_vn: self
                            .ngrid
                            .get_bel_name((die, (col, row), bels::IPAD_VN))
                            .unwrap(),
                        vaux: (0..16)
                            .map(|idx| self.edev.get_sysmon_vaux(die, col, row, idx))
                            .collect(),
                    });
                }
                ChipKind::Virtex7 => {
                    if chip.regs > 1 {
                        let col = self.edev.col_cfg;
                        let row = chip.row_reg_hclk(chip.reg_cfg);
                        res.push(SysMon {
                            die,
                            col,
                            row,
                            bank: 0,
                            pad_vp: self
                                .ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_VP))
                                .unwrap(),
                            pad_vn: self
                                .ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_VN))
                                .unwrap(),
                            vaux: (0..16)
                                .map(|idx| self.edev.get_sysmon_vaux(die, col, row, idx))
                                .collect(),
                        });
                    }
                }
            }
        }
        res
    }

    pub fn get_gts(&self) -> Vec<Gt<'_>> {
        let mut res = vec![];
        for &(die, col, row) in &self.edev.gt {
            let gt_info = self.edev.get_gt_info(die, col, row);
            let gt = match self.edev.kind {
                ChipKind::Virtex4 | ChipKind::Virtex5 => Gt {
                    die,
                    col,
                    row,
                    bank: gt_info.bank,
                    kind: gt_info.kind,
                    pads_clk: vec![(
                        self.ngrid
                            .get_bel_name((die, (col, row), bels::IPAD_CLKP0))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name((die, (col, row), bels::IPAD_CLKN0))
                            .unwrap(),
                    )],
                    pads_rx: vec![
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXP0))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXN0))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXP1))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXN1))
                                .unwrap(),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXP0))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXN0))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXP1))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXN1))
                                .unwrap(),
                        ),
                    ],
                },
                ChipKind::Virtex6 => Gt {
                    die,
                    col,
                    row,
                    bank: gt_info.bank,
                    kind: gt_info.kind,
                    pads_clk: match gt_info.kind {
                        GtKind::Gtx => vec![
                            (
                                self.ngrid
                                    .get_bel_name((die, (col, row), bels::IPAD_CLKP0))
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name((die, (col, row), bels::IPAD_CLKN0))
                                    .unwrap(),
                            ),
                            (
                                self.ngrid
                                    .get_bel_name((die, (col, row), bels::IPAD_CLKP1))
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name((die, (col, row), bels::IPAD_CLKN1))
                                    .unwrap(),
                            ),
                        ],
                        GtKind::Gth => vec![(
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_CLKP0))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_CLKN0))
                                .unwrap(),
                        )],
                        _ => unreachable!(),
                    },
                    pads_rx: vec![
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXP0))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXN0))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXP1))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXN1))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXP2))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXN2))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXP3))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::IPAD_RXN3))
                                .unwrap(),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXP0))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXN0))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXP1))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXN1))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXP2))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXN2))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXP3))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name((die, (col, row), bels::OPAD_TXN3))
                                .unwrap(),
                        ),
                    ],
                },
                ChipKind::Virtex7 => {
                    let channel_rows = [row - 25, row - 25 + 11, row - 25 + 28, row - 25 + 39];
                    Gt {
                        die,
                        col,
                        row,
                        bank: gt_info.bank,
                        kind: gt_info.kind,
                        pads_clk: vec![
                            (
                                self.ngrid
                                    .get_bel_name((die, (col, row), bels::IPAD_CLKP0))
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name((die, (col, row), bels::IPAD_CLKN0))
                                    .unwrap(),
                            ),
                            (
                                self.ngrid
                                    .get_bel_name((die, (col, row), bels::IPAD_CLKP1))
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name((die, (col, row), bels::IPAD_CLKN1))
                                    .unwrap(),
                            ),
                        ],
                        pads_tx: channel_rows
                            .into_iter()
                            .map(|crow| {
                                (
                                    self.ngrid
                                        .get_bel_name((die, (col, crow), bels::OPAD_TXP0))
                                        .unwrap(),
                                    self.ngrid
                                        .get_bel_name((die, (col, crow), bels::OPAD_TXN0))
                                        .unwrap(),
                                )
                            })
                            .collect(),
                        pads_rx: channel_rows
                            .into_iter()
                            .map(|crow| {
                                (
                                    self.ngrid
                                        .get_bel_name((die, (col, crow), bels::IPAD_RXP0))
                                        .unwrap(),
                                    self.ngrid
                                        .get_bel_name((die, (col, crow), bels::IPAD_RXN0))
                                        .unwrap(),
                                )
                            })
                            .collect(),
                    }
                }
            };
            res.push(gt);
        }
        res
    }

    pub fn get_ps_pin_name(&self, io: PsPin) -> &str {
        let slot = match io {
            PsPin::Mio(i) => bels::IOPAD_MIO[i as usize],
            PsPin::Clk => bels::IOPAD_PSCLK,
            PsPin::PorB => bels::IOPAD_PSPORB,
            PsPin::SrstB => bels::IOPAD_PSSRSTB,
            PsPin::DdrDq(i) => bels::IOPAD_DDRDQ[i as usize],
            PsPin::DdrDm(i) => bels::IOPAD_DDRDM[i as usize],
            PsPin::DdrDqsP(i) => bels::IOPAD_DDRDQSP[i as usize],
            PsPin::DdrDqsN(i) => bels::IOPAD_DDRDQSN[i as usize],
            PsPin::DdrA(i) => bels::IOPAD_DDRA[i as usize],
            PsPin::DdrBa(i) => bels::IOPAD_DDRBA[i as usize],
            PsPin::DdrVrP => bels::IOPAD_DDRVRP,
            PsPin::DdrVrN => bels::IOPAD_DDRVRN,
            PsPin::DdrCkP => bels::IOPAD_DDRCKP,
            PsPin::DdrCkN => bels::IOPAD_DDRCKN,
            PsPin::DdrCke => bels::IOPAD_DDRCKE,
            PsPin::DdrOdt => bels::IOPAD_DDRODT,
            PsPin::DdrDrstB => bels::IOPAD_DDRDRSTB,
            PsPin::DdrCsB => bels::IOPAD_DDRCSB,
            PsPin::DdrRasB => bels::IOPAD_DDRRASB,
            PsPin::DdrCasB => bels::IOPAD_DDRCASB,
            PsPin::DdrWeB => bels::IOPAD_DDRWEB,
        };
        let die = DieId::from_idx(0);
        let chip = self.edev.chips[die];
        let col = chip.col_ps();
        let row = chip.row_reg_bot(RegId::from_idx(chip.regs - 1));
        self.ngrid.get_bel_name((die, (col, row), slot)).unwrap()
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    match edev.kind {
        ChipKind::Virtex4 => virtex4::name_device(edev, ndb),
        ChipKind::Virtex5 => virtex5::name_device(edev, ndb),
        ChipKind::Virtex6 => virtex6::name_device(edev, ndb),
        ChipKind::Virtex7 => virtex7::name_device(edev, ndb),
    }
}
