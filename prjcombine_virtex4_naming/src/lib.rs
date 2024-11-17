use prjcombine_int::grid::{ColId, DieId, RowId};
use prjcombine_virtex4::{
    bond::PsPin,
    expanded::{ExpandedDevice, IoCoord},
    grid::{CfgRowKind, GridKind, GtKind, RegId},
};
use prjcombine_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use unnamed_entity::EntityId;

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

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
}

impl ExpandedNamedDevice<'_> {
    pub fn get_io_name(&self, io: IoCoord) -> &str {
        match self.edev.kind {
            GridKind::Virtex4 | GridKind::Virtex5 | GridKind::Virtex6 => self
                .ngrid
                .get_bel_name(io.die, io.col, io.row, &format!("IOB{}", io.iob))
                .unwrap(),
            GridKind::Virtex7 => {
                if matches!(io.row.to_idx() % 50, 0 | 49) {
                    self.ngrid
                        .get_bel_name(io.die, io.col, io.row, "IOB")
                        .unwrap()
                } else {
                    self.ngrid
                        .get_bel_name(io.die, io.col, io.row, &format!("IOB{}", io.iob))
                        .unwrap()
                }
            }
        }
    }

    pub fn get_sysmons(&self) -> Vec<SysMon<'_>> {
        let mut res = vec![];
        for (die, grid) in &self.edev.grids {
            match self.edev.kind {
                GridKind::Virtex4 => {
                    let mut idx = 0;
                    for &(row, kind) in &grid.rows_cfg {
                        let col = self.edev.col_cfg;
                        if kind != CfgRowKind::Sysmon {
                            continue;
                        }
                        res.push(SysMon {
                            die,
                            col,
                            row,
                            bank: idx,
                            pad_vp: self.ngrid.get_bel_name(die, col, row, "IPAD.VP").unwrap(),
                            pad_vn: self.ngrid.get_bel_name(die, col, row, "IPAD.VN").unwrap(),
                            vaux: (0..8)
                                .map(|idx| self.edev.get_sysmon_vaux(die, col, row, idx))
                                .collect(),
                        });
                        idx += 1;
                    }
                }
                GridKind::Virtex5 => {
                    let col = self.edev.col_cfg;
                    let row = grid.row_reg_hclk(grid.reg_cfg - 1);
                    res.push(SysMon {
                        die,
                        col,
                        row,
                        bank: 0,
                        pad_vp: self.ngrid.get_bel_name(die, col, row, "IPAD.VP").unwrap(),
                        pad_vn: self.ngrid.get_bel_name(die, col, row, "IPAD.VN").unwrap(),
                        vaux: (0..16)
                            .map(|idx| self.edev.get_sysmon_vaux(die, col, row, idx))
                            .collect(),
                    });
                }
                GridKind::Virtex6 => {
                    let col = self.edev.col_cfg;
                    let row = grid.row_reg_bot(grid.reg_cfg);
                    res.push(SysMon {
                        die,
                        col,
                        row,
                        bank: 0,
                        pad_vp: self.ngrid.get_bel_name(die, col, row, "IPAD.VP").unwrap(),
                        pad_vn: self.ngrid.get_bel_name(die, col, row, "IPAD.VN").unwrap(),
                        vaux: (0..16)
                            .map(|idx| self.edev.get_sysmon_vaux(die, col, row, idx))
                            .collect(),
                    });
                }
                GridKind::Virtex7 => {
                    if grid.regs > 1 {
                        let col = self.edev.col_cfg;
                        let row = grid.row_reg_hclk(grid.reg_cfg);
                        res.push(SysMon {
                            die,
                            col,
                            row,
                            bank: 0,
                            pad_vp: self.ngrid.get_bel_name(die, col, row, "IPAD.VP").unwrap(),
                            pad_vn: self.ngrid.get_bel_name(die, col, row, "IPAD.VN").unwrap(),
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
                GridKind::Virtex4 | GridKind::Virtex5 => Gt {
                    die,
                    col,
                    row,
                    bank: gt_info.bank,
                    kind: gt_info.kind,
                    pads_clk: vec![(
                        self.ngrid.get_bel_name(die, col, row, "IPAD.CLKP").unwrap(),
                        self.ngrid.get_bel_name(die, col, row, "IPAD.CLKN").unwrap(),
                    )],
                    pads_rx: vec![
                        (
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXP0").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXN0").unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXP1").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXN1").unwrap(),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXP0").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXN0").unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXP1").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXN1").unwrap(),
                        ),
                    ],
                },
                GridKind::Virtex6 => Gt {
                    die,
                    col,
                    row,
                    bank: gt_info.bank,
                    kind: gt_info.kind,
                    pads_clk: match gt_info.kind {
                        GtKind::Gtx => vec![
                            (
                                self.ngrid
                                    .get_bel_name(die, col, row, "IPAD.CLKP0")
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name(die, col, row, "IPAD.CLKN0")
                                    .unwrap(),
                            ),
                            (
                                self.ngrid
                                    .get_bel_name(die, col, row, "IPAD.CLKP1")
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name(die, col, row, "IPAD.CLKN1")
                                    .unwrap(),
                            ),
                        ],
                        GtKind::Gth => vec![(
                            self.ngrid.get_bel_name(die, col, row, "IPAD.CLKP").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "IPAD.CLKN").unwrap(),
                        )],
                        _ => unreachable!(),
                    },
                    pads_rx: vec![
                        (
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXP0").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXN0").unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXP1").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXN1").unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXP2").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXN2").unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXP3").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "IPAD.RXN3").unwrap(),
                        ),
                    ],
                    pads_tx: vec![
                        (
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXP0").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXN0").unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXP1").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXN1").unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXP2").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXN2").unwrap(),
                        ),
                        (
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXP3").unwrap(),
                            self.ngrid.get_bel_name(die, col, row, "OPAD.TXN3").unwrap(),
                        ),
                    ],
                },
                GridKind::Virtex7 => {
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
                                    .get_bel_name(die, col, row, "IPAD.CLKP0")
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name(die, col, row, "IPAD.CLKN0")
                                    .unwrap(),
                            ),
                            (
                                self.ngrid
                                    .get_bel_name(die, col, row, "IPAD.CLKP1")
                                    .unwrap(),
                                self.ngrid
                                    .get_bel_name(die, col, row, "IPAD.CLKN1")
                                    .unwrap(),
                            ),
                        ],
                        pads_tx: channel_rows
                            .into_iter()
                            .map(|crow| {
                                (
                                    self.ngrid.get_bel_name(die, col, crow, "OPAD.TXP").unwrap(),
                                    self.ngrid.get_bel_name(die, col, crow, "OPAD.TXN").unwrap(),
                                )
                            })
                            .collect(),
                        pads_rx: channel_rows
                            .into_iter()
                            .map(|crow| {
                                (
                                    self.ngrid.get_bel_name(die, col, crow, "IPAD.RXP").unwrap(),
                                    self.ngrid.get_bel_name(die, col, crow, "IPAD.RXN").unwrap(),
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
        let key = match io {
            PsPin::Mio(i) => format!("IOPAD.MIO{i}"),
            PsPin::Clk => "IOPAD.PSCLK".to_string(),
            PsPin::PorB => "IOPAD.PSPORB".to_string(),
            PsPin::SrstB => "IOPAD.PSSRSTB".to_string(),
            PsPin::DdrDq(i) => format!("IOPAD.DDRDQ{i}"),
            PsPin::DdrDm(i) => format!("IOPAD.DDRDM{i}"),
            PsPin::DdrDqsP(i) => format!("IOPAD.DDRDQSP{i}"),
            PsPin::DdrDqsN(i) => format!("IOPAD.DDRDQSN{i}"),
            PsPin::DdrA(i) => format!("IOPAD.DDRA{i}"),
            PsPin::DdrBa(i) => format!("IOPAD.DDRBA{i}"),
            PsPin::DdrVrP => "IOPAD.DDRVRP".to_string(),
            PsPin::DdrVrN => "IOPAD.DDRVRN".to_string(),
            PsPin::DdrCkP => "IOPAD.DDRCKP".to_string(),
            PsPin::DdrCkN => "IOPAD.DDRCKN".to_string(),
            PsPin::DdrCke => "IOPAD.DDRCKE".to_string(),
            PsPin::DdrOdt => "IOPAD.DDRODT".to_string(),
            PsPin::DdrDrstB => "IOPAD.DDRDRSTB".to_string(),
            PsPin::DdrCsB => "IOPAD.DDRCSB".to_string(),
            PsPin::DdrRasB => "IOPAD.DDRRASB".to_string(),
            PsPin::DdrCasB => "IOPAD.DDRCASB".to_string(),
            PsPin::DdrWeB => "IOPAD.DDRWEB".to_string(),
        };
        let die = DieId::from_idx(0);
        let grid = self.edev.grids[die];
        let col = grid.col_ps();
        let row = grid.row_reg_bot(RegId::from_idx(grid.regs - 1));
        self.ngrid.get_bel_name(die, col, row, &key).unwrap()
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    match edev.kind {
        GridKind::Virtex4 => virtex4::name_device(edev, ndb),
        GridKind::Virtex5 => virtex5::name_device(edev, ndb),
        GridKind::Virtex6 => virtex6::name_device(edev, ndb),
        GridKind::Virtex7 => virtex7::name_device(edev, ndb),
    }
}
