use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    dir::DirPartMap,
    grid::{CellCoord, DieId},
};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{
    bond::PsPad,
    chip::{CfgRowKind, ChipKind, GtKind, RegId},
    defs::bslots,
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
            .get_bel_name(io.cell.bel(bslots::IOB[io.iob.to_idx()]))
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
                            pad_vp: self
                                .ngrid
                                .get_bel_name_sub(cell.bel(bslots::SYSMON), 1)
                                .unwrap(),
                            pad_vn: self
                                .ngrid
                                .get_bel_name_sub(cell.bel(bslots::SYSMON), 2)
                                .unwrap(),
                            vaux: (0..8)
                                .map(|idx| self.edev.get_sysmon_vaux(cell, idx))
                                .collect(),
                        });
                        idx += 1;
                    }
                }
                ChipKind::Virtex5 => {
                    let tcrd = self.edev.tile_cfg(die);
                    res.push(SysMon {
                        cell: tcrd.cell,
                        bank: 0,
                        pad_vp: self
                            .ngrid
                            .get_bel_name_sub(tcrd.cell.bel(bslots::SYSMON), 1)
                            .unwrap(),
                        pad_vn: self
                            .ngrid
                            .get_bel_name_sub(tcrd.cell.bel(bslots::SYSMON), 2)
                            .unwrap(),
                        vaux: (0..16)
                            .map(|idx| self.edev.get_sysmon_vaux(tcrd.cell, idx))
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
                        pad_vp: self.ngrid.get_bel_name(cell.bel(bslots::IPAD_VP)).unwrap(),
                        pad_vn: self.ngrid.get_bel_name(cell.bel(bslots::IPAD_VN)).unwrap(),
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
                            pad_vp: self.ngrid.get_bel_name(cell.bel(bslots::IPAD_VP)).unwrap(),
                            pad_vn: self.ngrid.get_bel_name(cell.bel(bslots::IPAD_VN)).unwrap(),
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
                ChipKind::Virtex4 => Gt {
                    cell,
                    bank: gt_info.bank,
                    kind: gt_info.kind,
                    pads_clk: vec![(
                        self.ngrid
                            .get_bel_name_sub(cell.bel(bslots::GT11CLK), 1)
                            .unwrap(),
                        self.ngrid
                            .get_bel_name_sub(cell.bel(bslots::GT11CLK), 2)
                            .unwrap(),
                    )],
                    pads_rx: vec![
                        (
                            self.ngrid
                                .get_bel_name_sub(cell.bel(bslots::GT11[0]), 1)
                                .unwrap(),
                            self.ngrid
                                .get_bel_name_sub(cell.bel(bslots::GT11[0]), 2)
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name_sub(cell.bel(bslots::GT11[1]), 1)
                                .unwrap(),
                            self.ngrid
                                .get_bel_name_sub(cell.bel(bslots::GT11[1]), 2)
                                .unwrap(),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            self.ngrid
                                .get_bel_name_sub(cell.bel(bslots::GT11[0]), 3)
                                .unwrap(),
                            self.ngrid
                                .get_bel_name_sub(cell.bel(bslots::GT11[0]), 4)
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name_sub(cell.bel(bslots::GT11[1]), 3)
                                .unwrap(),
                            self.ngrid
                                .get_bel_name_sub(cell.bel(bslots::GT11[1]), 4)
                                .unwrap(),
                        ),
                    ],
                },
                ChipKind::Virtex5 => {
                    let bslot = match gt_info.kind {
                        GtKind::Gtp => bslots::GTP_DUAL,
                        GtKind::Gtx => bslots::GTX_DUAL,
                        _ => unreachable!(),
                    };
                    Gt {
                        cell,
                        bank: gt_info.bank,
                        kind: gt_info.kind,
                        pads_clk: vec![(
                            self.ngrid.get_bel_name_sub(cell.bel(bslot), 2).unwrap(),
                            self.ngrid.get_bel_name_sub(cell.bel(bslot), 3).unwrap(),
                        )],
                        pads_rx: vec![
                            (
                                self.ngrid.get_bel_name_sub(cell.bel(bslot), 4).unwrap(),
                                self.ngrid.get_bel_name_sub(cell.bel(bslot), 5).unwrap(),
                            ),
                            (
                                self.ngrid.get_bel_name_sub(cell.bel(bslot), 6).unwrap(),
                                self.ngrid.get_bel_name_sub(cell.bel(bslot), 7).unwrap(),
                            ),
                        ],
                        pads_tx: vec![
                            (
                                self.ngrid.get_bel_name_sub(cell.bel(bslot), 8).unwrap(),
                                self.ngrid.get_bel_name_sub(cell.bel(bslot), 9).unwrap(),
                            ),
                            (
                                self.ngrid.get_bel_name_sub(cell.bel(bslot), 10).unwrap(),
                                self.ngrid.get_bel_name_sub(cell.bel(bslot), 11).unwrap(),
                            ),
                        ],
                    }
                }
                ChipKind::Virtex6 => Gt {
                    cell,
                    bank: gt_info.bank,
                    kind: gt_info.kind,
                    pads_clk: match gt_info.kind {
                        GtKind::Gtx => vec![
                            (
                                self.ngrid
                                    .get_bel_name(cell.bel(bslots::IPAD_CLKP[0]))
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name(cell.bel(bslots::IPAD_CLKN[0]))
                                    .unwrap(),
                            ),
                            (
                                self.ngrid
                                    .get_bel_name(cell.bel(bslots::IPAD_CLKP[1]))
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name(cell.bel(bslots::IPAD_CLKN[1]))
                                    .unwrap(),
                            ),
                        ],
                        GtKind::Gth => vec![(
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_CLKP[0]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_CLKN[0]))
                                .unwrap(),
                        )],
                        _ => unreachable!(),
                    },
                    pads_rx: vec![
                        (
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_RXP[0]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_RXN[0]))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_RXP[1]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_RXN[1]))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_RXP[2]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_RXN[2]))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_RXP[3]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::IPAD_RXN[3]))
                                .unwrap(),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::OPAD_TXP[0]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::OPAD_TXN[0]))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::OPAD_TXP[1]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::OPAD_TXN[1]))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::OPAD_TXP[2]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::OPAD_TXN[2]))
                                .unwrap(),
                        ),
                        (
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::OPAD_TXP[3]))
                                .unwrap(),
                            self.ngrid
                                .get_bel_name(cell.bel(bslots::OPAD_TXN[3]))
                                .unwrap(),
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
                                self.ngrid
                                    .get_bel_name(cell.bel(bslots::IPAD_CLKP[0]))
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name(cell.bel(bslots::IPAD_CLKN[0]))
                                    .unwrap(),
                            ),
                            (
                                self.ngrid
                                    .get_bel_name(cell.bel(bslots::IPAD_CLKP[1]))
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name(cell.bel(bslots::IPAD_CLKN[1]))
                                    .unwrap(),
                            ),
                        ],
                        pads_tx: channel_cells
                            .into_iter()
                            .map(|ccell| {
                                (
                                    self.ngrid
                                        .get_bel_name(ccell.bel(bslots::OPAD_TXP[0]))
                                        .unwrap(),
                                    self.ngrid
                                        .get_bel_name(ccell.bel(bslots::OPAD_TXN[0]))
                                        .unwrap(),
                                )
                            })
                            .collect(),
                        pads_rx: channel_cells
                            .into_iter()
                            .map(|ccell| {
                                (
                                    self.ngrid
                                        .get_bel_name(ccell.bel(bslots::IPAD_RXP[0]))
                                        .unwrap(),
                                    self.ngrid
                                        .get_bel_name(ccell.bel(bslots::IPAD_RXN[0]))
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

    pub fn get_ps_pin_name(&self, io: PsPad) -> &str {
        let slot = match io {
            PsPad::Mio(i) => bslots::IOPAD_MIO[i as usize],
            PsPad::Clk => bslots::IOPAD_PSCLK,
            PsPad::PorB => bslots::IOPAD_PSPORB,
            PsPad::SrstB => bslots::IOPAD_PSSRSTB,
            PsPad::DdrDq(i) => bslots::IOPAD_DDRDQ[i as usize],
            PsPad::DdrDm(i) => bslots::IOPAD_DDRDM[i as usize],
            PsPad::DdrDqsP(i) => bslots::IOPAD_DDRDQSP[i as usize],
            PsPad::DdrDqsN(i) => bslots::IOPAD_DDRDQSN[i as usize],
            PsPad::DdrA(i) => bslots::IOPAD_DDRA[i as usize],
            PsPad::DdrBa(i) => bslots::IOPAD_DDRBA[i as usize],
            PsPad::DdrVrP => bslots::IOPAD_DDRVRP,
            PsPad::DdrVrN => bslots::IOPAD_DDRVRN,
            PsPad::DdrCkP => bslots::IOPAD_DDRCKP,
            PsPad::DdrCkN => bslots::IOPAD_DDRCKN,
            PsPad::DdrCke => bslots::IOPAD_DDRCKE,
            PsPad::DdrOdt => bslots::IOPAD_DDRODT,
            PsPad::DdrDrstB => bslots::IOPAD_DDRDRSTB,
            PsPad::DdrCsB => bslots::IOPAD_DDRCSB,
            PsPad::DdrRasB => bslots::IOPAD_DDRRASB,
            PsPad::DdrCasB => bslots::IOPAD_DDRCASB,
            PsPad::DdrWeB => bslots::IOPAD_DDRWEB,
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
