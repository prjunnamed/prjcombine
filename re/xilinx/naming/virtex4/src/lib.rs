#![recursion_limit = "1024"]

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    dir::DirPartMap,
    grid::{CellCoord, DieId},
};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{
    bels,
    bond::PsPad,
    chip::{CfgRowKind, ChipKind, GtKind, RegId},
    expanded::{ExpandedDevice, IoCoord},
    gtz::GtzIntColId,
};

mod virtex4;
mod virtex5;
mod virtex6;
mod virtex7;

#[derive(Debug)]
pub struct SysMon<'a> {
    pub cell: CellCoord,
    pub bank: u32,
    pub pad_vp: &'a str,
    pub pad_vn: &'a str,
    pub vaux: Vec<Option<(IoCoord, IoCoord)>>,
}

#[derive(Debug)]
pub struct Gt<'a> {
    pub cell: CellCoord,
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
            .get_bel_name(io.cell.bel(bels::IOB[io.iob.to_idx()]))
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
                        let cell = CellCoord::new(die, col, row);
                        res.push(SysMon {
                            cell,
                            bank: idx,
                            pad_vp: self.ngrid.get_bel_name(cell.bel(bels::IPAD_VP)).unwrap(),
                            pad_vn: self.ngrid.get_bel_name(cell.bel(bels::IPAD_VN)).unwrap(),
                            vaux: (0..8)
                                .map(|idx| self.edev.get_sysmon_vaux(cell, idx))
                                .collect(),
                        });
                        idx += 1;
                    }
                }
                ChipKind::Virtex5 => {
                    let col = self.edev.col_cfg;
                    let row = chip.row_reg_hclk(chip.reg_cfg - 1);
                    let cell = CellCoord::new(die, col, row);
                    res.push(SysMon {
                        cell,
                        bank: 0,
                        pad_vp: self.ngrid.get_bel_name(cell.bel(bels::IPAD_VP)).unwrap(),
                        pad_vn: self.ngrid.get_bel_name(cell.bel(bels::IPAD_VN)).unwrap(),
                        vaux: (0..16)
                            .map(|idx| self.edev.get_sysmon_vaux(cell, idx))
                            .collect(),
                    });
                }
                ChipKind::Virtex6 => {
                    let col = self.edev.col_cfg;
                    let row = chip.row_reg_bot(chip.reg_cfg);
                    let cell = CellCoord::new(die, col, row);
                    res.push(SysMon {
                        cell,
                        bank: 0,
                        pad_vp: self.ngrid.get_bel_name(cell.bel(bels::IPAD_VP)).unwrap(),
                        pad_vn: self.ngrid.get_bel_name(cell.bel(bels::IPAD_VN)).unwrap(),
                        vaux: (0..16)
                            .map(|idx| self.edev.get_sysmon_vaux(cell, idx))
                            .collect(),
                    });
                }
                ChipKind::Virtex7 => {
                    if chip.regs > 1 {
                        let col = self.edev.col_cfg;
                        let row = chip.row_reg_hclk(chip.reg_cfg);
                        let cell = CellCoord::new(die, col, row);
                        res.push(SysMon {
                            cell,
                            bank: 0,
                            pad_vp: self.ngrid.get_bel_name(cell.bel(bels::IPAD_VP)).unwrap(),
                            pad_vn: self.ngrid.get_bel_name(cell.bel(bels::IPAD_VN)).unwrap(),
                            vaux: (0..16)
                                .map(|idx| self.edev.get_sysmon_vaux(cell, idx))
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
        for &cell in &self.edev.gt {
            let gt_info = self.edev.get_gt_info(cell);
            let gt = match self.edev.kind {
                ChipKind::Virtex4 | ChipKind::Virtex5 => Gt {
                    cell,
                    bank: gt_info.bank,
                    kind: gt_info.kind,
                    pads_clk: vec![(
                        self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKP0)).unwrap(),
                        self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKN0)).unwrap(),
                    )],
                    pads_rx: vec![
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXP0)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXN0)).unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXP1)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXN1)).unwrap(),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXP0)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXN0)).unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXP1)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXN1)).unwrap(),
                        ),
                    ],
                },
                ChipKind::Virtex6 => Gt {
                    cell,
                    bank: gt_info.bank,
                    kind: gt_info.kind,
                    pads_clk: match gt_info.kind {
                        GtKind::Gtx => vec![
                            (
                                self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKP0)).unwrap(),
                                self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKN0)).unwrap(),
                            ),
                            (
                                self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKP1)).unwrap(),
                                self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKN1)).unwrap(),
                            ),
                        ],
                        GtKind::Gth => vec![(
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKP0)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKN0)).unwrap(),
                        )],
                        _ => unreachable!(),
                    },
                    pads_rx: vec![
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXP0)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXN0)).unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXP1)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXN1)).unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXP2)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXN2)).unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXP3)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::IPAD_RXN3)).unwrap(),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXP0)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXN0)).unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXP1)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXN1)).unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXP2)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXN2)).unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXP3)).unwrap(),
                            self.ngrid.get_bel_name(cell.bel(bels::OPAD_TXN3)).unwrap(),
                        ),
                    ],
                },
                ChipKind::Virtex7 => {
                    let channel_cells = [
                        cell.delta(0, -25),
                        cell.delta(0, -25 + 11),
                        cell.delta(0, -25 + 28),
                        cell.delta(0, -25 + 39),
                    ];
                    Gt {
                        cell,
                        bank: gt_info.bank,
                        kind: gt_info.kind,
                        pads_clk: vec![
                            (
                                self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKP0)).unwrap(),
                                self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKN0)).unwrap(),
                            ),
                            (
                                self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKP1)).unwrap(),
                                self.ngrid.get_bel_name(cell.bel(bels::IPAD_CLKN1)).unwrap(),
                            ),
                        ],
                        pads_tx: channel_cells
                            .into_iter()
                            .map(|ccell| {
                                (
                                    self.ngrid.get_bel_name(ccell.bel(bels::OPAD_TXP0)).unwrap(),
                                    self.ngrid.get_bel_name(ccell.bel(bels::OPAD_TXN0)).unwrap(),
                                )
                            })
                            .collect(),
                        pads_rx: channel_cells
                            .into_iter()
                            .map(|ccell| {
                                (
                                    self.ngrid.get_bel_name(ccell.bel(bels::IPAD_RXP0)).unwrap(),
                                    self.ngrid.get_bel_name(ccell.bel(bels::IPAD_RXN0)).unwrap(),
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

    pub fn get_ps_pin_name(&self, io: PsPad) -> &str {
        let slot = match io {
            PsPad::Mio(i) => bels::IOPAD_MIO[i as usize],
            PsPad::Clk => bels::IOPAD_PSCLK,
            PsPad::PorB => bels::IOPAD_PSPORB,
            PsPad::SrstB => bels::IOPAD_PSSRSTB,
            PsPad::DdrDq(i) => bels::IOPAD_DDRDQ[i as usize],
            PsPad::DdrDm(i) => bels::IOPAD_DDRDM[i as usize],
            PsPad::DdrDqsP(i) => bels::IOPAD_DDRDQSP[i as usize],
            PsPad::DdrDqsN(i) => bels::IOPAD_DDRDQSN[i as usize],
            PsPad::DdrA(i) => bels::IOPAD_DDRA[i as usize],
            PsPad::DdrBa(i) => bels::IOPAD_DDRBA[i as usize],
            PsPad::DdrVrP => bels::IOPAD_DDRVRP,
            PsPad::DdrVrN => bels::IOPAD_DDRVRN,
            PsPad::DdrCkP => bels::IOPAD_DDRCKP,
            PsPad::DdrCkN => bels::IOPAD_DDRCKN,
            PsPad::DdrCke => bels::IOPAD_DDRCKE,
            PsPad::DdrOdt => bels::IOPAD_DDRODT,
            PsPad::DdrDrstB => bels::IOPAD_DDRDRSTB,
            PsPad::DdrCsB => bels::IOPAD_DDRCSB,
            PsPad::DdrRasB => bels::IOPAD_DDRRASB,
            PsPad::DdrCasB => bels::IOPAD_DDRCASB,
            PsPad::DdrWeB => bels::IOPAD_DDRWEB,
        };
        let die = DieId::from_idx(0);
        let chip = self.edev.chips[die];
        let col = chip.col_ps();
        let row = chip.row_reg_bot(RegId::from_idx(chip.regs - 1));
        let cell = CellCoord::new(die, col, row);
        self.ngrid.get_bel_name(cell.bel(slot)).unwrap()
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
