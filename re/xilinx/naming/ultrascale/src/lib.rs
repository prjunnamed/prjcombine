#![recursion_limit = "1024"]

use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use prjcombine_interconnect::{
    dir::DirH,
    grid::{CellCoord, ColId, DieId, RowId},
};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_ultrascale::{
    bels,
    chip::{
        BramKind, ChipKind, CleMKind, ColumnKind, DisabledPart, DspKind, HardKind, HardRowKind,
        IoRowKind, PsIntfKind, RegId,
    },
    expanded::{ExpandedDevice, GtCoord, IoCoord},
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct DeviceNaming {
    pub rclk_alt_pins: BTreeMap<String, bool>,
}

struct Asx {
    gt: usize,
    io: usize,
    hdio: usize,
    cfg: usize,
    hbm: usize,
}

struct Asy {
    hdio: usize,
    hpio: usize,
    hrio: usize,
    cmt: usize,
    cfg: usize,
    gt: usize,
}

struct ASwitchGrid {
    xlut: EntityVec<ColId, Asx>,
    ylut: EntityVec<DieId, EntityVec<RegId, Asy>>,
}

fn make_aswitch_grid(edev: &ExpandedDevice) -> ASwitchGrid {
    let mut xlut = EntityVec::new();
    let mut asx = 0;
    let dev_has_hbm = edev.chips.first().unwrap().has_hbm;
    let pchip = edev.chips[edev.interposer.primary];
    for (col, &cd) in &pchip.columns {
        let cfg = asx;
        let gt = asx;
        let mut hdio = asx;
        let mut io = asx;
        let mut hbm = asx;
        match cd.kind {
            ColumnKind::Gt(idx) | ColumnKind::Io(idx) => {
                let regs = &pchip.cols_io[idx].regs;
                let has_hpio = regs.values().any(|&x| x == IoRowKind::Hpio);
                let has_hrio = regs.values().any(|&x| x == IoRowKind::Hrio);
                let has_hdio = regs.values().any(|&x| x == IoRowKind::HdioLc);
                let has_gt = regs.values().any(|&x| {
                    !matches!(
                        x,
                        IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio | IoRowKind::HdioLc
                    )
                });
                if has_gt && pchip.col_side(col) == DirH::W {
                    asx += 1;
                }
                hdio = asx;
                if has_hdio {
                    asx += 6;
                }
                io = asx;
                if has_hrio {
                    asx += 8;
                } else if has_hpio {
                    match edev.kind {
                        ChipKind::Ultrascale => asx += 5,
                        ChipKind::UltrascalePlus => asx += 8,
                    }
                } else if has_gt && pchip.col_side(col) == DirH::E {
                    asx += 1;
                }
            }
            ColumnKind::Hard(_, idx) => {
                let regs = &pchip.cols_hard[idx].regs;
                let has_hdio = regs
                    .values()
                    .any(|x| matches!(x, HardRowKind::Hdio | HardRowKind::HdioAms));
                let has_hdiolc = regs.values().any(|&x| x == HardRowKind::HdioLc);
                let has_cfg = regs.values().any(|&x| x == HardRowKind::Cfg);
                if has_cfg {
                    hdio += 1;
                    hbm += 1;
                    asx += 1;
                    if dev_has_hbm {
                        asx += 4;
                    }
                }
                if has_hdiolc {
                    asx += 6;
                } else if has_hdio {
                    asx += 4;
                }
            }
            _ => (),
        }
        xlut.push(Asx {
            gt,
            hdio,
            io,
            cfg,
            hbm,
        });
    }

    let mut ylut: EntityVec<_, _> = edev.chips.ids().map(|_| EntityVec::new()).collect();

    let mut asy = if dev_has_hbm { 2 } else { 0 };
    for (die, &chip) in &edev.chips {
        for reg in chip.regs() {
            let skip = edev.disabled.contains(&DisabledPart::Region(die, reg));
            let has_hdio = chip.cols_hard.iter().any(|x| {
                matches!(
                    x.regs[reg],
                    HardRowKind::Hdio | HardRowKind::HdioAms | HardRowKind::HdioLc
                )
            }) && !skip;
            let has_cfg = chip
                .cols_hard
                .iter()
                .any(|x| x.regs[reg] == HardRowKind::Cfg)
                && !skip;
            let has_hpio = chip.cols_io.iter().any(|x| x.regs[reg] == IoRowKind::Hpio) && !skip;
            let has_hrio = chip.cols_io.iter().any(|x| x.regs[reg] == IoRowKind::Hrio) && !skip;
            let has_hdiolc_l = chip
                .cols_io
                .iter()
                .any(|x| x.regs[reg] == IoRowKind::HdioLc)
                && !skip;
            let has_gt = chip.cols_io.iter().any(|x| {
                !matches!(
                    x.regs[reg],
                    IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio | IoRowKind::HdioLc
                )
            }) && !skip;

            let cfg = asy;
            let mut cmt = asy;
            if has_cfg || (chip.kind == ChipKind::UltrascalePlus && (has_hpio || has_hdiolc_l)) {
                asy += 1;
            }
            let gt = asy;
            if has_gt {
                asy += match chip.kind {
                    ChipKind::Ultrascale => 4,
                    ChipKind::UltrascalePlus => 5,
                };
            }
            if chip.kind == ChipKind::Ultrascale {
                cmt = asy;
                if has_hpio | has_hrio {
                    asy += 1;
                }
            }
            let hrio = asy;
            if has_hrio {
                asy += 1;
            }
            let hdio = asy;
            let mut hpio = asy;
            if has_hdio {
                hpio += 1;
                asy += 2;
            } else if has_hpio {
                asy += 1;
            }
            ylut[die].push(Asy {
                gt,
                hdio,
                hpio,
                hrio,
                cmt,
                cfg,
            });
        }
    }

    ASwitchGrid { xlut, ylut }
}

struct ClkGrid {
    brxlut: EntityVec<ColId, usize>,
    gtbxlut: EntityVec<ColId, usize>,
    gtbylut: EntityVec<DieId, EntityVec<RegId, (usize, usize)>>,
    brylut: EntityVec<DieId, EntityVec<RegId, usize>>,
    vsxlut: EntityVec<ColId, usize>,
}

fn make_clk_grid(edev: &ExpandedDevice) -> ClkGrid {
    let mut brxlut = EntityVec::new();
    let mut gtbxlut = EntityVec::new();
    let mut vsxlut = EntityVec::new();
    let pchip = edev.chips[edev.interposer.primary];

    let mut brx = 0;
    let mut gtbx = 0;
    let mut vsx = 0;
    for (col, &cd) in &pchip.columns {
        vsxlut.push(vsx);
        brxlut.push(brx);
        gtbxlut.push(gtbx);
        match cd.kind {
            ColumnKind::CleM(CleMKind::ClkBuf) => (),
            ColumnKind::CleM(CleMKind::Laguna) if edev.kind == ChipKind::UltrascalePlus => {
                brx += 2;
                gtbx += 2;
            }
            ColumnKind::CleL(_) | ColumnKind::CleM(_) => {
                if edev.kind == ChipKind::UltrascalePlus && pchip.col_side(col) == DirH::E {
                    continue;
                }
                // skip leftmost column on whole-height PS devices
                if col.to_idx() != 0 {
                    brx += 1;
                    gtbx += 1;
                }
            }
            ColumnKind::Bram(_) | ColumnKind::ContUram => match edev.kind {
                ChipKind::Ultrascale => {
                    brx += 2;
                    gtbx += 2;
                }
                ChipKind::UltrascalePlus => {
                    brx += 4;
                    gtbx += 4;
                    vsx += 2;
                }
            },
            ColumnKind::Dsp(DspKind::ClkBuf) => (),
            ColumnKind::Dsp(_) => {
                if (pchip.is_nocfg() && col > pchip.cols_hard.last().unwrap().col)
                    || (matches!(pchip.columns.last().unwrap().kind, ColumnKind::Hard(_, _))
                        && col > pchip.cols_hard.first().unwrap().col)
                {
                    brx += 4;
                    gtbx += 4;
                } else {
                    brx += 2;
                    gtbx += 2;
                }
            }
            ColumnKind::Io(_) => {
                if edev.kind == ChipKind::Ultrascale {
                    brx += 1;
                }
                gtbx += 1;
            }
            _ => (),
        }
    }

    let mut gtbylut: EntityVec<_, _> = edev.chips.ids().map(|_| EntityVec::new()).collect();
    let mut brylut: EntityVec<_, _> = edev.chips.ids().map(|_| EntityVec::new()).collect();
    let mut gtby = 0;
    let mut bry = 0;
    for (die, &chip) in &edev.chips {
        for reg in chip.regs() {
            let skip = edev.disabled.contains(&DisabledPart::Region(die, reg));
            let has_hprio = chip.cols_io.iter().any(|x| {
                matches!(
                    x.regs[reg],
                    IoRowKind::Hpio | IoRowKind::Hrio | IoRowKind::HdioLc
                )
            }) && !skip;
            if has_hprio {
                match edev.kind {
                    ChipKind::Ultrascale => {
                        gtbylut[die].push((gtby, gtby + 24));
                    }
                    ChipKind::UltrascalePlus => {
                        gtbylut[die].push((gtby, gtby + 18));
                    }
                }
                gtby += 25;
            } else if !skip {
                gtbylut[die].push((gtby, gtby));
                gtby += 1;
            } else {
                gtbylut[die].push((0, 0));
            }
            brylut[die].push(bry);
            if !skip {
                bry += 1;
            }
        }
    }

    ClkGrid {
        brxlut,
        gtbxlut,
        gtbylut,
        brylut,
        vsxlut,
    }
}

#[allow(clippy::type_complexity)]
struct IoGrid {
    hpio_xlut: EntityPartVec<ColId, usize>,
    hdio_xlut: EntityPartVec<ColId, usize>,
    hpio_ylut: EntityVec<DieId, EntityPartVec<RegId, (usize, usize, usize, usize)>>,
    hdio_ylut: EntityVec<DieId, EntityPartVec<RegId, (usize, usize)>>,
    is_cfg_io_hrio: bool,
}

fn make_io_grid(edev: &ExpandedDevice) -> IoGrid {
    let pchip = edev.chips[edev.interposer.primary];

    let mut iox = 0;
    let mut hpio_xlut = EntityPartVec::new();
    let mut hdio_xlut = EntityPartVec::new();
    for (col, &cd) in &pchip.columns {
        match cd.kind {
            ColumnKind::Io(idx) => {
                let mut has_hdiolc = false;
                let mut has_hpio = false;
                for chip in edev.chips.values() {
                    let iocol = &chip.cols_io[idx];
                    if iocol
                        .regs
                        .values()
                        .any(|x| matches!(x, IoRowKind::Hpio | IoRowKind::Hrio))
                    {
                        has_hpio = true;
                    }
                    if iocol.regs.values().any(|x| matches!(x, IoRowKind::HdioLc)) {
                        has_hdiolc = true;
                    }
                }
                if has_hdiolc {
                    hdio_xlut.insert(col, iox);
                    iox += 1;
                }
                if has_hpio {
                    hpio_xlut.insert(col, iox);
                    iox += 1;
                }
            }
            ColumnKind::Hard(_, idx) => {
                let regs = &pchip.cols_hard[idx].regs;
                if regs.values().any(|x| {
                    matches!(
                        x,
                        HardRowKind::Hdio | HardRowKind::HdioAms | HardRowKind::HdioLc
                    )
                }) {
                    hdio_xlut.insert(col, iox);
                    iox += 1;
                }
            }
            _ => (),
        }
    }
    let mut is_cfg_io_hrio = false;
    if let Some(ioc_cfg) = pchip.cols_io.iter().find(|x| x.col == edev.col_cfg_io) {
        is_cfg_io_hrio = ioc_cfg.regs[pchip.reg_cfg()] == IoRowKind::Hrio;
    }

    let mut hdio_ylut: EntityVec<_, _> = edev.chips.ids().map(|_| EntityPartVec::new()).collect();
    let mut hpio_ylut: EntityVec<_, _> = edev.chips.ids().map(|_| EntityPartVec::new()).collect();
    let mut ioy = 0;
    for (die, &chip) in &edev.chips {
        for reg in chip.regs() {
            let skip = edev.disabled.contains(&DisabledPart::Region(die, reg));
            let has_hdio = chip
                .cols_hard
                .iter()
                .any(|x| matches!(x.regs[reg], HardRowKind::Hdio | HardRowKind::HdioAms))
                && !skip;
            let has_hdiolc = (chip
                .cols_hard
                .iter()
                .any(|x| matches!(x.regs[reg], HardRowKind::HdioLc))
                || chip
                    .cols_io
                    .iter()
                    .any(|x| matches!(x.regs[reg], IoRowKind::HdioLc)))
                && !skip;
            let has_hprio = chip
                .cols_io
                .iter()
                .any(|x| matches!(x.regs[reg], IoRowKind::Hpio | IoRowKind::Hrio))
                && !skip;
            if has_hprio && has_hdiolc {
                // what in the fuck why am I doing this to myself
                hpio_ylut[die].insert(reg, (ioy, ioy + 30, ioy + 43, ioy + 73));
                hdio_ylut[die].insert(reg, (ioy, ioy + 43));
                ioy += 86;
            } else if has_hdiolc {
                hdio_ylut[die].insert(reg, (ioy, ioy + 42));
                ioy += 84;
            } else if has_hprio {
                hpio_ylut[die].insert(reg, (ioy, ioy + 13, ioy + 26, ioy + 39));
                if has_hdio {
                    hdio_ylut[die].insert(reg, (ioy, ioy + 26));
                }
                ioy += 52;
            } else if has_hdio {
                hdio_ylut[die].insert(reg, (ioy, ioy + 12));
                ioy += 24;
            }
        }
    }

    IoGrid {
        hpio_xlut,
        hdio_xlut,
        hpio_ylut,
        hdio_ylut,
        is_cfg_io_hrio,
    }
}

#[derive(Debug)]
pub struct Gt<'a> {
    pub crd: GtCoord,
    pub bank: u32,
    pub kind: IoRowKind,
    pub name_common: &'a str,
    pub name_channel: Vec<&'a str>,
}

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
}

impl ExpandedNamedDevice<'_> {
    pub fn get_io_name(&self, io: IoCoord) -> &str {
        match io {
            IoCoord::Hpio(hpio) => {
                let chip = self.edev.chips[hpio.die];
                let iocol = chip
                    .cols_io
                    .iter()
                    .find(|iocol| iocol.col == hpio.col)
                    .unwrap();
                let kind = iocol.regs[hpio.reg];
                let (row, idx) = if hpio.iob.to_idx() < 26 {
                    (chip.row_reg_bot(hpio.reg), hpio.iob.to_idx())
                } else {
                    (chip.row_reg_bot(hpio.reg) + 30, hpio.iob.to_idx() - 26)
                };
                self.ngrid
                    .get_bel_name(CellCoord::new(hpio.die, hpio.col, row).bel(
                        if kind == IoRowKind::Hpio {
                            bels::HPIOB[idx]
                        } else {
                            bels::HRIOB[idx]
                        },
                    ))
                    .unwrap()
            }
            IoCoord::Hdio(hdio) => {
                let chip = self.edev.chips[hdio.die];
                let (row, idx) = if hdio.iob.to_idx() < 12 {
                    (chip.row_reg_bot(hdio.reg), hdio.iob.to_idx())
                } else {
                    (chip.row_reg_bot(hdio.reg) + 30, hdio.iob.to_idx() - 12)
                };
                self.ngrid
                    .get_bel_name(CellCoord::new(hdio.die, hdio.col, row).bel(bels::HDIOB[idx]))
                    .unwrap()
            }
            IoCoord::HdioLc(hdio) => {
                let chip = self.edev.chips[hdio.die];
                let (row, idx) = if hdio.iob.to_idx() < 42 {
                    (chip.row_reg_bot(hdio.reg), hdio.iob.to_idx())
                } else {
                    (chip.row_reg_bot(hdio.reg) + 30, hdio.iob.to_idx() - 42)
                };
                self.ngrid
                    .get_bel_name(CellCoord::new(hdio.die, hdio.col, row).bel(bels::HDIOB[idx]))
                    .unwrap()
            }
        }
    }

    pub fn get_gts(&self) -> Vec<Gt<'_>> {
        let mut res = vec![];
        for &crd in &self.edev.gt {
            let chip = self.edev.chips[crd.die];
            let gt_info = self.edev.get_gt_info(crd);
            let row = chip.row_reg_rclk(crd.reg);
            let cell = CellCoord::new(crd.die, crd.col, row);
            let (name_common, name_channel) = match gt_info.kind {
                IoRowKind::Gth => (
                    self.ngrid.get_bel_name(cell.bel(bels::GTH_COMMON)).unwrap(),
                    vec![
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTH_CHANNEL0))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTH_CHANNEL1))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTH_CHANNEL2))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTH_CHANNEL3))
                            .unwrap(),
                    ],
                ),
                IoRowKind::Gty => (
                    self.ngrid.get_bel_name(cell.bel(bels::GTY_COMMON)).unwrap(),
                    vec![
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTY_CHANNEL0))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTY_CHANNEL1))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTY_CHANNEL2))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTY_CHANNEL3))
                            .unwrap(),
                    ],
                ),
                IoRowKind::Gtm => (
                    self.ngrid.get_bel_name(cell.bel(bels::GTM_REFCLK)).unwrap(),
                    vec![self.ngrid.get_bel_name(cell.bel(bels::GTM_DUAL)).unwrap()],
                ),
                IoRowKind::Gtf => (
                    self.ngrid.get_bel_name(cell.bel(bels::GTF_COMMON)).unwrap(),
                    vec![
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTF_CHANNEL0))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTF_CHANNEL1))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTF_CHANNEL2))
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(cell.bel(bels::GTF_CHANNEL3))
                            .unwrap(),
                    ],
                ),
                IoRowKind::HsAdc => (
                    self.ngrid.get_bel_name(cell.bel(bels::HSADC)).unwrap(),
                    vec![],
                ),
                IoRowKind::HsDac => (
                    self.ngrid.get_bel_name(cell.bel(bels::HSDAC)).unwrap(),
                    vec![],
                ),
                IoRowKind::RfAdc => (
                    self.ngrid.get_bel_name(cell.bel(bels::RFADC)).unwrap(),
                    vec![],
                ),
                IoRowKind::RfDac => (
                    self.ngrid.get_bel_name(cell.bel(bels::RFDAC)).unwrap(),
                    vec![],
                ),
                _ => unreachable!(),
            };
            res.push(Gt {
                crd,
                bank: gt_info.bank,
                kind: gt_info.kind,
                name_common,
                name_channel,
            })
        }
        res
    }
}

fn get_bram_tk(
    edev: &ExpandedDevice,
    has_laguna: bool,
    die: DieId,
    col: ColId,
    row: RowId,
) -> &'static str {
    let chip = edev.chips[die];
    let in_laguna = has_laguna && chip.is_laguna_row(row);
    let cd = chip.columns[col];
    match (chip.kind, cd.kind, col < chip.col_cfg()) {
        (ChipKind::Ultrascale, ColumnKind::Bram(BramKind::Plain), true) => "RCLK_BRAM_L",
        (ChipKind::Ultrascale, ColumnKind::Bram(BramKind::Plain), false) => "RCLK_BRAM_R",
        (ChipKind::Ultrascale, ColumnKind::Bram(BramKind::BramClmp), true) => {
            "RCLK_RCLK_BRAM_L_BRAMCLMP_FT"
        }
        (ChipKind::Ultrascale, ColumnKind::Bram(BramKind::AuxClmp), true) => {
            "RCLK_RCLK_BRAM_L_AUXCLMP_FT"
        }
        (ChipKind::Ultrascale, ColumnKind::Bram(BramKind::BramClmpMaybe), true) => {
            if in_laguna {
                "RCLK_BRAM_L"
            } else {
                "RCLK_RCLK_BRAM_L_BRAMCLMP_FT"
            }
        }
        (ChipKind::Ultrascale, ColumnKind::Bram(BramKind::AuxClmpMaybe), true) => {
            if in_laguna {
                "RCLK_BRAM_L"
            } else {
                "RCLK_RCLK_BRAM_L_AUXCLMP_FT"
            }
        }
        (ChipKind::UltrascalePlus, ColumnKind::Bram(BramKind::Plain), true) => "RCLK_BRAM_INTF_L",
        (ChipKind::UltrascalePlus, ColumnKind::Bram(BramKind::Plain), false) => "RCLK_BRAM_INTF_R",
        (ChipKind::UltrascalePlus, ColumnKind::Bram(BramKind::Td), true) => "RCLK_BRAM_INTF_TD_L",
        (ChipKind::UltrascalePlus, ColumnKind::Bram(BramKind::Td), false) => "RCLK_BRAM_INTF_TD_R",
        _ => unreachable!(),
    }
}

pub fn name_device<'a>(
    edev: &'a ExpandedDevice<'a>,
    ndb: &'a NamingDb,
    dev_naming: &DeviceNaming,
) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);

    let mut int_grid = ngrid.bel_multi_grid(|_, node, _| node == "INT");
    for col in edev.chips[edev.interposer.primary].columns.ids() {
        int_grid.xlut.insert(col, col.to_idx() / 2);
    }
    if edev.kind == ChipKind::Ultrascale
        && edev.disabled.contains(&DisabledPart::Region(
            DieId::from_idx(0),
            RegId::from_idx(0),
        ))
    {
        for dylut in int_grid.ylut.values_mut() {
            for y in dylut.values_mut() {
                *y += 1;
            }
        }
    }

    let rclk_int_grid = ngrid.bel_multi_grid(|_, node, _| node == "RCLK_INT");
    let rclk_ps_grid = ngrid.bel_multi_grid(|_, node, _| node == "RCLK_PS");
    let cle_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "CLEL" | "CLEM"));
    let laguna_grid = ngrid.bel_multi_grid(|_, node, _| node == "LAGUNA");
    let bram_grid = ngrid.bel_multi_grid(|_, node, _| node == "BRAM");
    let hard_sync_grid = ngrid.bel_multi_grid(|_, node, _| node == "HARD_SYNC");
    let dsp_grid = ngrid.bel_multi_grid(|_, node, _| node == "DSP");
    let uram_grid = ngrid.bel_multi_grid(|_, node, _| node == "URAM");
    let cfg_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "CFG" | "CFG_CSEC"));
    let cfgio_grid = ngrid.bel_multi_grid(|_, node, _| node == "CFGIO");
    let ams_grid = ngrid.bel_multi_grid(|_, node, _| node == "AMS");
    let cmac_grid = ngrid.bel_multi_grid(|_, node, _| node == "CMAC");
    let pcie_grid = ngrid.bel_multi_grid(|_, node, _| node == "PCIE");
    let pcie4_grid = ngrid.bel_multi_grid(|_, node, _| node == "PCIE4");
    let pcie4c_grid = ngrid.bel_multi_grid(|_, node, _| node == "PCIE4C");
    let ilkn_grid = ngrid.bel_multi_grid(|_, node, _| node == "ILKN");
    let fe_grid = ngrid.bel_multi_grid(|_, node, _| node == "FE");
    let dfe_a_grid = ngrid.bel_multi_grid(|_, node, _| node == "DFE_A");
    let dfe_b_grid = ngrid.bel_multi_grid(|_, node, _| node == "DFE_B");
    let dfe_c_grid = ngrid.bel_multi_grid(|_, node, _| node == "DFE_C");
    let dfe_d_grid = ngrid.bel_multi_grid(|_, node, _| node == "DFE_D");
    let dfe_e_grid = ngrid.bel_multi_grid(|_, node, _| node == "DFE_E");
    let dfe_f_grid = ngrid.bel_multi_grid(|_, node, _| node == "DFE_F");
    let dfe_g_grid = ngrid.bel_multi_grid(|_, node, _| node == "DFE_G");
    let hdio_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "HDIO_S" | "HDIO_N"));
    let hdiolc_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "HDIOLC_S" | "HDIOLC_N"));
    let hpio_grid = ngrid.bel_multi_grid(|_, node, _| node == "HPIO");
    let rclk_hpio_grid = ngrid.bel_multi_grid(|_, node, _| node == "RCLK_HPIO");
    let hrio_grid = ngrid.bel_multi_grid(|_, node, _| node == "HRIO");
    let rclk_hdio_grid =
        ngrid.bel_multi_grid(|_, node, _| matches!(node, "RCLK_HDIO" | "RCLK_HDIOLC"));
    let aswitch_grid = make_aswitch_grid(edev);
    let io_grid = make_io_grid(edev);
    let clk_grid = make_clk_grid(edev);
    let xiphy_grid = ngrid.bel_multi_grid(|_, node, _| node == "XIPHY");
    let cmt_grid = ngrid.bel_multi_grid(|_, node, _| {
        matches!(node, "CMT" | "CMT_HBM") || (edev.kind == ChipKind::Ultrascale && node == "XIPHY")
    });
    let gt_grid = ngrid.bel_multi_grid(|_, node, _| {
        matches!(
            node,
            "GTH" | "GTY" | "GTM" | "GTF" | "HSADC" | "HSDAC" | "RFADC" | "RFDAC"
        )
    });
    let gth_grid = ngrid.bel_multi_grid(|_, node, _| node == "GTH");
    let gty_grid = ngrid.bel_multi_grid(|_, node, _| node == "GTY");
    let gtm_grid = ngrid.bel_multi_grid(|_, node, _| node == "GTM");
    let gtf_grid = ngrid.bel_multi_grid(|_, node, _| node == "GTF");
    let hsadc_grid = ngrid.bel_multi_grid(|_, node, _| node == "HSADC");
    let hsdac_grid = ngrid.bel_multi_grid(|_, node, _| node == "HSDAC");
    let rfadc_grid = ngrid.bel_multi_grid(|_, node, _| node == "RFADC");
    let rfdac_grid = ngrid.bel_multi_grid(|_, node, _| node == "RFDAC");

    let has_laguna = edev.chips.values().any(|chip| {
        chip.columns
            .values()
            .any(|cd| cd.kind == ColumnKind::CleM(CleMKind::Laguna))
    });

    let hdio_cfg_only = edev.chips.map(|_, chip| {
        Vec::from_iter(chip.cols_hard.iter().map(|hcol| {
            hcol.regs.values().all(|&x| {
                matches!(
                    x,
                    HardRowKind::Cfg
                        | HardRowKind::Ams
                        | HardRowKind::Hdio
                        | HardRowKind::HdioAms
                        | HardRowKind::None
                )
            }) || !hcol.regs.values().any(|&x| x == HardRowKind::Cfg)
        }))
    });

    let has_mcap = edev.chips.map(|_, chip| {
        chip.cols_hard.iter().any(|hcol| {
            hcol.regs.iter().any(|(reg, &kind)| {
                kind == HardRowKind::Cfg
                    && reg.to_idx() != 0
                    && matches!(
                        hcol.regs[reg - 1],
                        HardRowKind::Pcie | HardRowKind::PciePlus
                    )
            })
        }) && !chip.is_nocfg()
    });

    for (tcrd, tile) in egrid.tiles() {
        let cell = tcrd.cell;
        let CellCoord { die, col, row } = cell;
        let chip = edev.chips[die];
        let reg = chip.row_to_reg(row);
        let kind = egrid.db.tile_classes.key(tile.class);
        let x = int_grid.xlut[col];
        let y = int_grid.ylut[die][row];
        match &kind[..] {
            "INT" => {
                ngrid.name_tile(tcrd, "INT", [format!("INT_X{x}Y{y}")]);
            }
            "INTF" => match chip.kind {
                ChipKind::Ultrascale => {
                    if chip.col_side(col) == DirH::W {
                        ngrid.name_tile(tcrd, "INTF.W", [format!("INT_INTERFACE_L_X{x}Y{y}")]);
                    } else {
                        ngrid.name_tile(tcrd, "INTF.E", [format!("INT_INTERFACE_R_X{x}Y{y}")]);
                    }
                }
                ChipKind::UltrascalePlus => {
                    if chip.col_side(col) == DirH::W {
                        ngrid.name_tile(tcrd, "INTF.W", [format!("INT_INTF_L_X{x}Y{y}")]);
                    } else {
                        ngrid.name_tile(tcrd, "INTF.E", [format!("INT_INTF_R_X{x}Y{y}")]);
                    }
                }
            },
            "INTF.DELAY" => {
                if chip.col_side(col) == DirH::W {
                    match chip.columns[col].kind {
                        ColumnKind::Io(_) | ColumnKind::Gt(_) => {
                            let cio = chip.cols_io.iter().find(|x| x.col == col).unwrap();
                            match cio.regs[reg] {
                                IoRowKind::Hpio | IoRowKind::Hrio => {
                                    ngrid.name_tile(
                                        tcrd,
                                        "INTF.W.IO",
                                        [format!("INT_INT_INTERFACE_XIPHY_FT_X{x}Y{y}")],
                                    );
                                }
                                _ => {
                                    let kind = if chip.kind == ChipKind::Ultrascale {
                                        "INT_INT_INTERFACE_GT_LEFT_FT"
                                    } else {
                                        "INT_INTF_L_TERM_GT"
                                    };
                                    ngrid.name_tile(
                                        tcrd,
                                        "INTF.W.GT",
                                        [format!("{kind}_X{x}Y{y}")],
                                    );
                                }
                            }
                        }
                        ColumnKind::ContHard | ColumnKind::Sdfec => {
                            let kind = if chip.kind == ChipKind::Ultrascale {
                                "INT_INTERFACE_PCIE_L"
                            } else {
                                "INT_INTF_L_PCIE4"
                            };
                            ngrid.name_tile(tcrd, "INTF.W.PCIE", [format!("{kind}_X{x}Y{y}")]);
                        }
                        _ => unreachable!(),
                    }
                } else {
                    match chip.columns[col].kind {
                        ColumnKind::Gt(_) | ColumnKind::Io(_) => {
                            let kind = if chip.kind == ChipKind::Ultrascale {
                                "INT_INTERFACE_GT_R"
                            } else {
                                "INT_INTF_R_TERM_GT"
                            };
                            ngrid.name_tile(tcrd, "INTF.E.GT", [format!("{kind}_X{x}Y{y}")]);
                        }
                        ColumnKind::Hard(HardKind::Term, _) => {
                            ngrid.name_tile(
                                tcrd,
                                "INTF.E.GT",
                                [format!("INT_INTF_RIGHT_TERM_HDIO_FT_X{x}Y{y}")],
                            );
                        }
                        ColumnKind::Hard(_, _)
                        | ColumnKind::DfeB
                        | ColumnKind::DfeC
                        | ColumnKind::DfeDF
                        | ColumnKind::DfeE => {
                            let kind = if chip.kind == ChipKind::Ultrascale {
                                "INT_INTERFACE_PCIE_R"
                            } else {
                                "INT_INTF_R_PCIE4"
                            };
                            ngrid.name_tile(tcrd, "INTF.E.PCIE", [format!("{kind}_X{x}Y{y}")]);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            "INTF.IO" => {
                if chip.col_side(col) == DirH::W {
                    match chip.columns[col].kind {
                        ColumnKind::Io(_) | ColumnKind::Gt(_) => {
                            let kind = if col.to_idx() == 0 {
                                "INT_INTF_LEFT_TERM_IO_FT"
                            } else if matches!(row.to_idx() % 15, 0 | 1 | 13 | 14) {
                                "INT_INTF_L_CMT"
                            } else {
                                "INT_INTF_L_IO"
                            };
                            ngrid.name_tile(tcrd, "INTF.W.IO", [format!("{kind}_X{x}Y{y}")]);
                        }
                        _ => {
                            ngrid.name_tile(
                                tcrd,
                                "INTF.PSS",
                                [format!("INT_INTF_LEFT_TERM_PSS_X{x}Y{y}")],
                            );
                        }
                    }
                } else {
                    ngrid.name_tile(
                        tcrd,
                        "INTF.E.IO",
                        [format!("INT_INTF_RIGHT_TERM_IO_X{x}Y{y}")],
                    );
                }
            }
            "RCLK_INT" => {
                let lr = if col < chip.col_cfg() { 'L' } else { 'R' };
                let name = format!("RCLK_INT_{lr}_X{x}Y{yy}", yy = y - 1);
                let nnode = ngrid.name_tile(tcrd, "RCLK_INT", [name]);
                let rx = rclk_int_grid.xlut[col];
                let ry = rclk_int_grid.ylut[die][row];
                match chip.kind {
                    ChipKind::Ultrascale => {
                        nnode.add_bel(
                            bels::BUFCE_LEAF_X16_S,
                            format!("BUFCE_LEAF_X16_X{rx}Y{y}", y = ry * 2),
                        );
                        nnode.add_bel(
                            bels::BUFCE_LEAF_X16_N,
                            format!("BUFCE_LEAF_X16_X{rx}Y{y}", y = ry * 2 + 1),
                        );
                    }
                    ChipKind::UltrascalePlus => {
                        for i in 0..16 {
                            nnode.add_bel(
                                bels::BUFCE_LEAF_S[i],
                                format!(
                                    "BUFCE_LEAF_X{x}Y{y}",
                                    x = rx * 8 + (i & 7),
                                    y = ry * 4 + i / 8
                                ),
                            );
                            nnode.add_bel(
                                bels::BUFCE_LEAF_N[i],
                                format!(
                                    "BUFCE_LEAF_X{x}Y{y}",
                                    x = rx * 8 + (i & 7),
                                    y = ry * 4 + i / 8 + 2
                                ),
                            );
                        }
                    }
                }
            }

            "CLEL" => {
                let tkn = if chip.col_side(col) == DirH::W {
                    "CLEL_L"
                } else {
                    "CLEL_R"
                };
                let nnode = ngrid.name_tile(tcrd, "CLEL", [format!("{tkn}_X{x}Y{y}")]);
                if !(row.to_idx() % 60 == 59
                    && edev
                        .disabled
                        .contains(&DisabledPart::TopRow(die, chip.row_to_reg(row))))
                {
                    let sx = cle_grid.xlut[col];
                    let sy = cle_grid.ylut[die][row];
                    nnode.add_bel(bels::SLICE, format!("SLICE_X{sx}Y{sy}"));
                }
            }
            "CLEM" => {
                let tk = match (chip.kind, col < chip.col_cfg()) {
                    (ChipKind::Ultrascale, true) => "CLE_M",
                    (ChipKind::Ultrascale, false) => "CLE_M_R",
                    (ChipKind::UltrascalePlus, true) => "CLEM",
                    (ChipKind::UltrascalePlus, false) => "CLEM_R",
                };
                let nnode = ngrid.name_tile(tcrd, "CLEM", [format!("{tk}_X{x}Y{y}")]);
                if !(row.to_idx() % 60 == 59
                    && edev
                        .disabled
                        .contains(&DisabledPart::TopRow(die, chip.row_to_reg(row))))
                {
                    let sx = cle_grid.xlut[col];
                    let sy = cle_grid.ylut[die][row];
                    nnode.add_bel(bels::SLICE, format!("SLICE_X{sx}Y{sy}"));
                }
            }
            "LAGUNA" => {
                let (x, tk) = match chip.kind {
                    ChipKind::Ultrascale => (x, "LAGUNA_TILE"),
                    ChipKind::UltrascalePlus => (x - 1, "LAG_LAG"),
                };
                let nnode = ngrid.name_tile(tcrd, "LAGUNA", [format!("{tk}_X{x}Y{y}")]);
                let lx0 = laguna_grid.xlut[col] * 2;
                let lx1 = lx0 + 1;
                let ly0 = laguna_grid.ylut[die][row] * 2;
                let ly1 = ly0 + 1;
                nnode.add_bel(bels::LAGUNA0, format!("LAGUNA_X{lx0}Y{ly0}"));
                nnode.add_bel(bels::LAGUNA1, format!("LAGUNA_X{lx0}Y{ly1}"));
                nnode.add_bel(bels::LAGUNA2, format!("LAGUNA_X{lx1}Y{ly0}"));
                nnode.add_bel(bels::LAGUNA3, format!("LAGUNA_X{lx1}Y{ly1}"));
            }
            "BRAM" => {
                let nnode = ngrid.name_tile(tcrd, "BRAM", [format!("BRAM_X{x}Y{y}")]);
                let bx = bram_grid.xlut[col];
                let by = bram_grid.ylut[die][row];
                nnode.add_bel(bels::BRAM_F, format!("RAMB36_X{bx}Y{by}"));
                nnode.add_bel(bels::BRAM_H0, format!("RAMB18_X{bx}Y{y}", y = by * 2));
                nnode.add_bel(bels::BRAM_H1, format!("RAMB18_X{bx}Y{y}", y = by * 2 + 1));
            }
            "HARD_SYNC" => {
                let tk = get_bram_tk(edev, has_laguna, die, col, row);
                let nnode =
                    ngrid.name_tile(tcrd, "HARD_SYNC", [format!("{tk}_X{x}Y{y}", y = y - 1)]);
                let hx0 = hard_sync_grid.xlut[col] * 2;
                let hx1 = hx0 + 1;
                let hy0 = hard_sync_grid.ylut[die][row] * 2;
                let hy1 = hy0 + 1;
                nnode.add_bel(bels::HARD_SYNC0, format!("HARD_SYNC_X{hx0}Y{hy0}"));
                nnode.add_bel(bels::HARD_SYNC1, format!("HARD_SYNC_X{hx0}Y{hy1}"));
                nnode.add_bel(bels::HARD_SYNC2, format!("HARD_SYNC_X{hx1}Y{hy0}"));
                nnode.add_bel(bels::HARD_SYNC3, format!("HARD_SYNC_X{hx1}Y{hy1}"));
            }
            "DSP" => {
                let nnode = ngrid.name_tile(tcrd, "DSP", [format!("DSP_X{x}Y{y}")]);
                let dx = dsp_grid.xlut[col];
                let dy0 = dsp_grid.ylut[die][row] * 2;
                let dy1 = dy0 + 1;
                nnode.add_bel(bels::DSP0, format!("DSP48E2_X{dx}Y{dy0}"));
                if !(row.to_idx() % 60 == 55
                    && edev
                        .disabled
                        .contains(&DisabledPart::TopRow(die, chip.row_to_reg(row))))
                {
                    nnode.add_bel(bels::DSP1, format!("DSP48E2_X{dx}Y{dy1}"));
                }
            }
            "URAM" => {
                let tk = if row.to_idx() % 60 == 45 {
                    "URAM_URAM_DELAY_FT"
                } else {
                    "URAM_URAM_FT"
                };
                let nnode = ngrid.name_tile(tcrd, "URAM", [format!("{tk}_X{x}Y{y}")]);
                let ux = uram_grid.xlut[col];
                let uy0 = uram_grid.ylut[die][row] * 4;
                let uy1 = uy0 + 1;
                let uy2 = uy0 + 2;
                let uy3 = uy0 + 3;
                nnode.add_bel(bels::URAM0, format!("URAM288_X{ux}Y{uy0}"));
                nnode.add_bel(bels::URAM1, format!("URAM288_X{ux}Y{uy1}"));
                nnode.add_bel(bels::URAM2, format!("URAM288_X{ux}Y{uy2}"));
                nnode.add_bel(bels::URAM3, format!("URAM288_X{ux}Y{uy3}"));
            }
            "CFG" | "CFG_CSEC" => {
                let ColumnKind::Hard(_, idx) = chip.columns[col].kind else {
                    unreachable!()
                };
                let x = if chip.kind == ChipKind::UltrascalePlus && !hdio_cfg_only[die][idx] {
                    x + 1
                } else {
                    x
                };
                let tk = if chip.kind == ChipKind::Ultrascale {
                    "CFG_CFG"
                } else if !chip.has_csec {
                    "CFG_CONFIG"
                } else {
                    "CSEC_CONFIG_FT"
                };
                let name = format!("{tk}_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = cfg_grid.xlut[col];
                let sy = cfg_grid.ylut[die][row];
                if chip.has_csec {
                    nnode.add_bel(bels::CFG, format!("CSEC_SITE_X{sx}Y{sy}"));
                } else {
                    nnode.add_bel(bels::CFG, format!("CONFIG_SITE_X{sx}Y{sy}"));
                }
                let asx = aswitch_grid.xlut[col].cfg;
                let asy = aswitch_grid.ylut[die][reg].cfg;
                nnode.add_bel(bels::ABUS_SWITCH_CFG, format!("ABUS_SWITCH_X{asx}Y{asy}"));
            }
            "CFGIO" => {
                let ColumnKind::Hard(_, idx) = chip.columns[col].kind else {
                    unreachable!()
                };
                let x = if chip.kind == ChipKind::UltrascalePlus
                    && (!hdio_cfg_only[die][idx] || chip.has_csec)
                {
                    x + 1
                } else {
                    x
                };
                let tk = if chip.kind == ChipKind::Ultrascale {
                    "CFGIO_IOB"
                } else if !chip.has_csec {
                    "CFGIO_IOB20"
                } else {
                    "CFGIOLC_IOB20_FT"
                };
                let name = format!("{tk}_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = cfgio_grid.xlut[col];
                let sy = cfgio_grid.ylut[die][row];
                nnode.add_bel(bels::PMV, format!("PMV_X{sx}Y{sy}"));
                nnode.add_bel(bels::PMV2, format!("PMV2_X{sx}Y{sy}"));
                nnode.add_bel(bels::PMVIOB, format!("PMVIOB_X{sx}Y{sy}"));
                nnode.add_bel(bels::MTBF3, format!("MTBF3_X{sx}Y{sy}"));
                if chip.kind == ChipKind::UltrascalePlus {
                    nnode.add_bel(bels::CFGIO, format!("CFGIO_SITE_X{sx}Y{sy}"));
                }
            }
            "AMS" => {
                let ColumnKind::Hard(_, idx) = chip.columns[col].kind else {
                    unreachable!()
                };
                let x = if chip.kind == ChipKind::UltrascalePlus
                    && (!hdio_cfg_only[die][idx] || chip.has_csec)
                {
                    x + 1
                } else {
                    x
                };
                let name = format!("AMS_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = ams_grid.xlut[col];
                let sy = ams_grid.ylut[die][row];
                let bk = if chip.kind == ChipKind::Ultrascale {
                    "SYSMONE1"
                } else {
                    "SYSMONE4"
                };
                nnode.add_bel(bels::SYSMON, format!("{bk}_X{sx}Y{sy}"));
            }
            "PCIE" => {
                let name = format!("PCIE_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = pcie_grid.xlut[col];
                let sy = pcie_grid.ylut[die][row];
                nnode.add_bel(bels::PCIE3, format!("PCIE_3_1_X{sx}Y{sy}"));
            }
            "PCIE4" => {
                let name = format!("PCIE4_PCIE4_FT_X{x}Y{y}");
                let naming = if has_mcap[die] {
                    "PCIE4"
                } else {
                    "PCIE4.NOCFG"
                };
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let sx = pcie4_grid.xlut[col];
                let sy = pcie4_grid.ylut[die][row];
                nnode.add_bel(bels::PCIE4, format!("PCIE40E4_X{sx}Y{sy}"));
            }
            "PCIE4C" => {
                let name = format!("PCIE4C_PCIE4C_FT_X{x}Y{y}");
                let naming = if has_mcap[die] {
                    "PCIE4C"
                } else {
                    "PCIE4C.NOCFG"
                };
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let sx = pcie4c_grid.xlut[col];
                let sy = pcie4c_grid.ylut[die][row];
                nnode.add_bel(bels::PCIE4C, format!("PCIE4CE4_X{sx}Y{sy}"));
            }
            "CMAC" => {
                let name = if chip.kind == ChipKind::Ultrascale {
                    let x = if col == chip.col_cfg() { x } else { x + 1 };
                    format!("CMAC_CMAC_FT_X{x}Y{y}")
                } else {
                    format!("CMAC_X{x}Y{y}")
                };
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = cmac_grid.xlut[col];
                let sy = cmac_grid.ylut[die][row];
                if chip.kind == ChipKind::Ultrascale {
                    nnode.add_bel(bels::CMAC, format!("CMAC_SITE_X{sx}Y{sy}"));
                } else {
                    nnode.add_bel(bels::CMAC, format!("CMACE4_X{sx}Y{sy}"));
                }
            }
            "ILKN" => {
                let name = if chip.kind == ChipKind::Ultrascale {
                    format!("ILMAC_ILMAC_FT_X{x}Y{y}")
                } else {
                    format!("ILKN_ILKN_FT_X{x}Y{y}")
                };
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = ilkn_grid.xlut[col];
                let sy = ilkn_grid.ylut[die][row];
                if chip.kind == ChipKind::Ultrascale {
                    nnode.add_bel(bels::ILKN, format!("ILKN_SITE_X{sx}Y{sy}"));
                } else {
                    nnode.add_bel(bels::ILKN, format!("ILKNE4_X{sx}Y{sy}"));
                }
            }
            "FE" => {
                let name = format!("FE_FE_FT_X{x}Y{y}", x = x - 1);
                let nnode = ngrid.name_tile(tcrd, "FE", [name]);
                let sx = fe_grid.xlut[col];
                let sy = fe_grid.ylut[die][row];
                nnode.add_bel(bels::FE, format!("FE_X{sx}Y{sy}"));
            }
            "DFE_A" => {
                let name = format!("DFE_DFE_TILEA_FT_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = dfe_a_grid.xlut[col];
                let sy = dfe_a_grid.ylut[die][row];
                nnode.add_bel(bels::DFE_A, format!("DFE_A_X{sx}Y{sy}"));
            }
            "DFE_B" => {
                let name = format!("DFE_DFE_TILEB_FT_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = dfe_b_grid.xlut[col];
                let sy = dfe_b_grid.ylut[die][row];
                nnode.add_bel(bels::DFE_B, format!("DFE_B_X{sx}Y{sy}"));
            }
            "DFE_C" => {
                let name = format!("DFE_DFE_TILEC_FT_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = dfe_c_grid.xlut[col];
                let sy = dfe_c_grid.ylut[die][row];
                nnode.add_bel(bels::DFE_C, format!("DFE_C_X{sx}Y{sy}"));
            }
            "DFE_D" => {
                let name = format!("DFE_DFE_TILED_FT_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = dfe_d_grid.xlut[col];
                let sy = dfe_d_grid.ylut[die][row];
                nnode.add_bel(bels::DFE_D, format!("DFE_D_X{sx}Y{sy}"));
            }
            "DFE_E" => {
                let name = format!("DFE_DFE_TILEE_FT_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = dfe_e_grid.xlut[col];
                let sy = dfe_e_grid.ylut[die][row];
                nnode.add_bel(bels::DFE_E, format!("DFE_E_X{sx}Y{sy}"));
            }
            "DFE_F" => {
                let name = format!("DFE_DFE_TILEF_FT_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = dfe_f_grid.xlut[col];
                let sy = dfe_f_grid.ylut[die][row];
                nnode.add_bel(bels::DFE_F, format!("DFE_F_X{sx}Y{sy}"));
            }
            "DFE_G" => {
                let name = format!("DFE_DFE_TILEG_FT_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = dfe_g_grid.xlut[col];
                let sy = dfe_g_grid.ylut[die][row];
                nnode.add_bel(bels::DFE_G, format!("DFE_G_X{sx}Y{sy}"));
            }
            "HDIO_S" | "HDIO_N" => {
                let ColumnKind::Hard(_, idx) = chip.columns[col].kind else {
                    unreachable!()
                };
                let tkn = match kind.as_str() {
                    "HDIO_S" => "HDIO_BOT",
                    "HDIO_N" => "HDIO_TOP",
                    _ => unreachable!(),
                };
                let x = if hdio_cfg_only[die][idx] { x } else { x + 1 };
                let name = format!("{tkn}_RIGHT_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let iox = io_grid.hdio_xlut[col];
                let ioy = match &kind[..] {
                    "HDIO_S" => io_grid.hdio_ylut[die][reg].0,
                    "HDIO_N" => io_grid.hdio_ylut[die][reg].1,
                    _ => unreachable!(),
                };
                let sx = hdio_grid.xlut[col];
                let sy = hdio_grid.ylut[die][row];
                for j in 0..12 {
                    nnode.add_bel(bels::HDIOB[j], format!("IOB_X{iox}Y{y}", y = ioy + j));
                }
                for j in 0..6 {
                    nnode.add_bel(
                        bels::HDIOB_DIFF_IN[j],
                        format!("HDIOBDIFFINBUF_X{sx}Y{y}", y = sy * 6 + j),
                    );
                    nnode.add_bel(
                        bels::HDIOLOGIC[2 * j],
                        format!("HDIOLOGIC_M_X{sx}Y{y}", y = sy * 6 + j),
                    );
                    nnode.add_bel(
                        bels::HDIOLOGIC[2 * j + 1],
                        format!("HDIOLOGIC_S_X{sx}Y{y}", y = sy * 6 + j),
                    );
                }
                nnode.add_bel(bels::HDLOGIC_CSSD[0], format!("HDLOGIC_CSSD_X{sx}Y{sy}"));
                if kind == "HDIO_S" {
                    nnode.add_bel(bels::HDIO_VREF0, format!("HDIO_VREF_X{sx}Y{y}", y = sy / 2));
                } else {
                    nnode.add_bel(bels::HDIO_BIAS, format!("HDIO_BIAS_X{sx}Y{y}", y = sy / 2));
                }
            }
            "HDIOLC_S" | "HDIOLC_N" => {
                let naming = match &kind[..] {
                    "HDIOLC_S" => {
                        if chip.col_side(col) == DirH::W {
                            "HDIOLC_HDIOL_BOT_LEFT_FT"
                        } else if reg == chip.reg_cfg() {
                            "HDIOLC_HDIOL_BOT_RIGHT_CFG_FT"
                        } else {
                            "HDIOLC_HDIOL_BOT_RIGHT_AUX_FT"
                        }
                    }
                    "HDIOLC_N" => {
                        if chip.col_side(col) == DirH::W {
                            "HDIOLC_HDIOL_TOP_LEFT_FT"
                        } else if reg == chip.reg_cfg() {
                            "HDIOLC_HDIOL_TOP_RIGHT_CFG_FT"
                        } else {
                            "HDIOLC_HDIOL_TOP_RIGHT_AUX_FT"
                        }
                    }
                    _ => unreachable!(),
                };
                let name = format!("{naming}_X{x}Y{y}");
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let iox = io_grid.hdio_xlut[col];
                let ioy = match &kind[..] {
                    "HDIOLC_S" => io_grid.hdio_ylut[die][reg].0,
                    "HDIOLC_N" => io_grid.hdio_ylut[die][reg].1,
                    _ => unreachable!(),
                };
                let sx = hdiolc_grid.xlut[col];
                let sy = hdiolc_grid.ylut[die][row];
                for j in 0..42 {
                    nnode.add_bel(bels::HDIOB[j], format!("IOB_X{iox}Y{y}", y = ioy + j));
                }
                for j in 0..21 {
                    nnode.add_bel(
                        bels::HDIOB_DIFF_IN[j],
                        format!("HDIOBDIFFINBUF_X{sx}Y{y}", y = sy * 21 + j),
                    );
                    nnode.add_bel(
                        bels::HDIOLOGIC[2 * j],
                        format!("HDIOLOGIC_M_X{sx}Y{y}", y = sy * 21 + j),
                    );
                    nnode.add_bel(
                        bels::HDIOLOGIC[2 * j + 1],
                        format!("HDIOLOGIC_S_X{sx}Y{y}", y = sy * 21 + j),
                    );
                }
                for j in 0..3 {
                    nnode.add_bel(
                        bels::HDLOGIC_CSSD[j],
                        format!("HDLOGIC_CSSD_X{sx}Y{y}", y = sy * 3 + j),
                    );
                }
                for j in 0..2 {
                    nnode.add_bel(
                        bels::HDIO_VREF[j],
                        format!("HDIO_VREF_X{sx}Y{y}", y = sy * 2 + j),
                    );
                }
                nnode.add_bel(bels::HDIO_BIAS, format!("HDIO_BIAS_X{sx}Y{sy}"));
            }
            "RCLK_HDIO" => {
                let ColumnKind::Hard(hk, idx) = chip.columns[col].kind else {
                    unreachable!()
                };
                let x = if hdio_cfg_only[die][idx] { x } else { x + 1 };
                let tkn = match hk {
                    HardKind::Clk => "RCLK_HDIO",
                    HardKind::NonClk => "RCLK_RCLK_HDIO_R_FT",
                    HardKind::Term => "RCLK_RCLK_HDIO_LAST_R_FT",
                };
                let name = format!("{tkn}_X{x}Y{y}", y = y - 1);
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = rclk_hdio_grid.xlut[col];
                let sy = rclk_hdio_grid.ylut[die][row];
                nnode.add_bel(
                    bels::BUFGCE_HDIO[0],
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2),
                );
                nnode.add_bel(
                    bels::BUFGCE_HDIO[1],
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2 + 1),
                );
                nnode.add_bel(
                    bels::BUFGCE_HDIO[2],
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2 + 1, y = sy * 2),
                );
                nnode.add_bel(
                    bels::BUFGCE_HDIO[3],
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2 + 1, y = sy * 2 + 1),
                );
                for (i, x, y) in [
                    (0, 0, 0),
                    (1, 0, 1),
                    (2, 1, 0),
                    (3, 1, 1),
                    (4, 2, 0),
                    (5, 2, 1),
                    (6, 3, 0),
                ] {
                    nnode.add_bel(
                        bels::ABUS_SWITCH_HDIO[i],
                        format!(
                            "ABUS_SWITCH_X{x}Y{y}",
                            x = aswitch_grid.xlut[col].hdio + x,
                            y = aswitch_grid.ylut[die][reg].hdio + y
                        ),
                    );
                }
            }
            "RCLK_HDIOLC" => {
                let (naming, tkn) = if chip.col_side(col) == DirH::W {
                    ("RCLK_HDIOLC_L", "RCLK_RCLK_HDIOL_L_FT")
                } else {
                    ("RCLK_HDIOLC_R", "RCLK_RCLK_HDIOL_R_FT")
                };
                let name = format!("{tkn}_X{x}Y{y}", y = y - 1);
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let sx = rclk_hdio_grid.xlut[col];
                let sy = rclk_hdio_grid.ylut[die][row];
                nnode.add_bel(
                    bels::BUFGCE_HDIO[0],
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2),
                );
                nnode.add_bel(
                    bels::BUFGCE_HDIO[1],
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2 + 1),
                );
                nnode.add_bel(
                    bels::BUFGCE_HDIO[2],
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2 + 1, y = sy * 2),
                );
                nnode.add_bel(
                    bels::BUFGCE_HDIO[3],
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2 + 1, y = sy * 2 + 1),
                );
                for (i, x, y) in [
                    (0, 0, 0),
                    (1, 0, 1),
                    (2, 1, 0),
                    (3, 1, 1),
                    (4, 2, 0),
                    (5, 2, 1),
                    (6, 3, 0),
                    (7, 3, 1),
                    (8, 4, 0),
                    (9, 4, 1),
                    (10, 5, 0),
                    (11, 5, 1),
                ] {
                    nnode.add_bel(
                        bels::ABUS_SWITCH_HDIO[i],
                        format!(
                            "ABUS_SWITCH_X{x}Y{y}",
                            x = aswitch_grid.xlut[col].hdio + x,
                            y = aswitch_grid.ylut[die][reg].hdio + y
                        ),
                    );
                }
            }

            "CMT" | "CMT_HBM" => {
                let iocol = chip.cols_io.iter().find(|iocol| iocol.col == col).unwrap();
                let tk = if chip.col_side(col) == DirH::W {
                    if kind == "CMT_HBM" {
                        "CMT_LEFT_H"
                    } else if iocol.regs[reg] == IoRowKind::HdioLc {
                        "CMT_CMT_LEFT_DL3_FT"
                    } else {
                        "CMT_L"
                    }
                } else {
                    "CMT_RIGHT"
                };
                let naming = if kind == "CMT_HBM" {
                    "CMT_L_HBM"
                } else if chip.col_side(col) == DirH::W {
                    "CMT_L"
                } else {
                    "CMT_R"
                };
                let name = format!("{tk}_X{x}Y{y}", y = y - 30);
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let cmtx = cmt_grid.xlut[col];
                let cmty = cmt_grid.ylut[die][row];
                let gtbx = clk_grid.gtbxlut[col];
                for i in 0..24 {
                    nnode.add_bel(
                        bels::BUFCE_ROW_CMT[i],
                        format!("BUFCE_ROW_X{cmtx}Y{y}", y = cmty * 24 + i),
                    );
                    nnode.add_bel(
                        bels::GCLK_TEST_BUF_CMT[i],
                        format!(
                            "GCLK_TEST_BUFE3_X{gtbx}Y{y}",
                            y = clk_grid.gtbylut[die][reg].0 + if i < 18 { i } else { i + 1 }
                        ),
                    );
                    nnode.add_bel(
                        bels::BUFGCE[i],
                        format!("BUFGCE_X{cmtx}Y{y}", y = cmty * 24 + i),
                    );
                }
                for i in 0..8 {
                    nnode.add_bel(
                        bels::BUFGCTRL[i],
                        format!("BUFGCTRL_X{cmtx}Y{y}", y = cmty * 8 + i),
                    );
                }
                for i in 0..4 {
                    nnode.add_bel(
                        bels::BUFGCE_DIV[i],
                        format!("BUFGCE_DIV_X{cmtx}Y{y}", y = cmty * 4 + i),
                    );
                }
                for i in 0..2 {
                    nnode.add_bel(bels::PLL[i], format!("PLL_X{cmtx}Y{y}", y = cmty * 2 + i));
                }
                nnode.add_bel(bels::MMCM, format!("MMCM_X{cmtx}Y{cmty}"));
                let asx = if chip.col_side(col) == DirH::W {
                    aswitch_grid.xlut[col].io + 7
                } else {
                    aswitch_grid.xlut[col].io
                };
                nnode.add_bel(
                    bels::ABUS_SWITCH_CMT,
                    format!(
                        "ABUS_SWITCH_X{asx}Y{y}",
                        y = aswitch_grid.ylut[die][reg].cmt
                    ),
                );
                if kind == "CMT_HBM" {
                    nnode.add_bel(bels::HBM_REF_CLK0, "HBM_REF_CLK_X0Y0".to_string());
                    nnode.add_bel(bels::HBM_REF_CLK1, "HBM_REF_CLK_X0Y1".to_string());
                }
            }
            "XIPHY" if edev.kind == ChipKind::Ultrascale => {
                let nnode =
                    ngrid.name_tile(tcrd, "XIPHY", [format!("XIPHY_L_X{x}Y{y}", y = y - 30)]);
                let cmtx = cmt_grid.xlut[col];
                let cmty = cmt_grid.ylut[die][row];
                for i in 0..24 {
                    nnode.add_bel(
                        bels::BUFCE_ROW_CMT[i],
                        format!(
                            "BUFCE_ROW_X{x}Y{y}",
                            x = clk_grid.brxlut[col],
                            y = cmty * 25 + i
                        ),
                    );
                    nnode.add_bel(
                        bels::GCLK_TEST_BUF_CMT[i],
                        format!(
                            "GCLK_TEST_BUFE3_X{x}Y{y}",
                            x = clk_grid.gtbxlut[col],
                            y = clk_grid.gtbylut[die][reg].0 + i
                        ),
                    );
                    nnode.add_bel(
                        bels::BUFGCE[i],
                        format!("BUFGCE_X{cmtx}Y{y}", y = cmty * 24 + i),
                    );
                }
                for i in 0..8 {
                    nnode.add_bel(
                        bels::BUFGCTRL[i],
                        format!("BUFGCTRL_X{cmtx}Y{y}", y = cmty * 8 + i),
                    );
                }
                for i in 0..4 {
                    nnode.add_bel(
                        bels::BUFGCE_DIV[i],
                        format!("BUFGCE_DIV_X{cmtx}Y{y}", y = cmty * 4 + i),
                    );
                }
                for i in 0..2 {
                    nnode.add_bel(
                        bels::PLL[i],
                        format!("PLLE3_ADV_X{cmtx}Y{y}", y = cmty * 2 + i),
                    );
                }
                nnode.add_bel(bels::MMCM, format!("MMCME3_ADV_X{cmtx}Y{cmty}"));
                nnode.add_bel(
                    bels::ABUS_SWITCH_CMT,
                    format!(
                        "ABUS_SWITCH_X{x}Y{y}",
                        x = aswitch_grid.xlut[col].io,
                        y = aswitch_grid.ylut[die][reg].cmt
                    ),
                );
                for i in 0..52 {
                    nnode.add_bel(
                        bels::BITSLICE[i],
                        format!("BITSLICE_RX_TX_X{cmtx}Y{y}", y = cmty * 52 + i),
                    );
                }
                for i in 0..8 {
                    nnode.add_bel(
                        bels::BITSLICE_T[i],
                        format!("BITSLICE_TX_X{cmtx}Y{y}", y = cmty * 8 + i),
                    );
                }
                for i in 0..8 {
                    nnode.add_bel(
                        bels::BITSLICE_CONTROL[i],
                        format!("BITSLICE_CONTROL_X{cmtx}Y{y}", y = cmty * 8 + i),
                    );
                }
                for i in 0..8 {
                    nnode.add_bel(
                        bels::PLL_SELECT[i],
                        format!("PLL_SELECT_SITE_X{cmtx}Y{y}", y = cmty * 8 + (i ^ 1)),
                    );
                }
                for i in 0..4 {
                    nnode.add_bel(
                        bels::RIU_OR[i],
                        format!("RIU_OR_X{cmtx}Y{y}", y = cmty * 4 + i),
                    );
                }
                for i in 0..4 {
                    nnode.add_bel(
                        bels::XIPHY_FEEDTHROUGH[i],
                        format!("XIPHY_FEEDTHROUGH_X{x}Y{cmty}", x = cmtx * 4 + i),
                    );
                }
            }
            "XIPHY" => {
                let tk = if chip.col_side(col) == DirH::W {
                    "XIPHY_BYTE_L"
                } else {
                    "XIPHY_BYTE_RIGHT"
                };
                let nnode = ngrid.name_tile(tcrd, kind, [format!("{tk}_X{x}Y{y}")]);
                let phyx = xiphy_grid.xlut[col];
                let phyy = xiphy_grid.ylut[die][row];
                for i in 0..13 {
                    nnode.add_bel(
                        bels::BITSLICE[i],
                        format!("BITSLICE_RX_TX_X{phyx}Y{y}", y = phyy * 13 + i),
                    );
                }
                for i in 0..2 {
                    nnode.add_bel(
                        bels::BITSLICE_T[i],
                        format!("BITSLICE_TX_X{phyx}Y{y}", y = phyy * 2 + i),
                    );
                }
                for i in 0..2 {
                    nnode.add_bel(
                        bels::BITSLICE_CONTROL[i],
                        format!("BITSLICE_CONTROL_X{phyx}Y{y}", y = phyy * 2 + i),
                    );
                }
                for i in 0..2 {
                    nnode.add_bel(
                        bels::PLL_SELECT[i],
                        format!("PLL_SELECT_SITE_X{phyx}Y{y}", y = phyy * 2 + i),
                    );
                }
                nnode.add_bel(bels::RIU_OR0, format!("RIU_OR_X{phyx}Y{phyy}"));
                nnode.add_bel(
                    bels::XIPHY_FEEDTHROUGH0,
                    format!("XIPHY_FEEDTHROUGH_X{phyx}Y{phyy}"),
                );
            }
            "RCLK_XIPHY" => {
                let (naming, tk) = match chip.col_side(col) {
                    DirH::W => ("RCLK_XIPHY_L", "RCLK_RCLK_XIPHY_INNER_FT"),
                    DirH::E => ("RCLK_XIPHY_R", "RCLK_XIPHY_OUTER_RIGHT"),
                };
                let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                ngrid.name_tile(tcrd, naming, [name]);
            }

            "HPIO" if chip.kind == ChipKind::Ultrascale => {
                let name = format!("HPIO_L_X{x}Y{y}", x = if x > 0 { x - 1 } else { x });
                let naming = if io_grid.is_cfg_io_hrio {
                    "HPIO.NOCFG"
                } else {
                    "HPIO"
                };
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let iox = io_grid.hpio_xlut[col];
                let (ioy_b, ioy_t) = match row.to_idx() % 60 {
                    0 => (io_grid.hpio_ylut[die][reg].0, io_grid.hpio_ylut[die][reg].1),
                    30 => (io_grid.hpio_ylut[die][reg].2, io_grid.hpio_ylut[die][reg].3),
                    _ => unreachable!(),
                };
                let sx = hpio_grid.xlut[col];
                let sy = hpio_grid.ylut[die][row];
                for j in 0..13 {
                    nnode.add_bel(bels::HPIOB[j], format!("IOB_X{iox}Y{y}", y = ioy_b + j));
                }
                for j in 0..13 {
                    nnode.add_bel(
                        bels::HPIOB[13 + j],
                        format!("IOB_X{iox}Y{y}", y = ioy_t + j),
                    );
                }
                for j in 0..12 {
                    nnode.add_bel(
                        bels::HPIOB_DIFF_IN[j],
                        format!("HPIOBDIFFINBUF_X{sx}Y{y}", y = sy * 12 + j),
                    );
                }
                for j in 0..12 {
                    nnode.add_bel(
                        bels::HPIOB_DIFF_OUT[j],
                        format!("HPIOBDIFFOUTBUF_X{sx}Y{y}", y = sy * 12 + j),
                    );
                }
                nnode.add_bel(bels::HPIO_VREF, format!("HPIO_VREF_SITE_X{sx}Y{sy}"));
            }
            "RCLK_HPIO" if chip.kind == ChipKind::Ultrascale => {
                let name = format!(
                    "RCLK_HPIO_L_X{x}Y{y}",
                    x = if x > 0 { x - 1 } else { x },
                    y = y - 1
                );
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = rclk_hpio_grid.xlut[col];
                let sy = rclk_hpio_grid.ylut[die][row];
                for i in 0..5 {
                    nnode.add_bel(
                        bels::ABUS_SWITCH_HPIO[i],
                        format!(
                            "ABUS_SWITCH_X{x}Y{y}",
                            x = aswitch_grid.xlut[col].io + i,
                            y = aswitch_grid.ylut[die][reg].hpio
                        ),
                    );
                }
                nnode.add_bel(
                    bels::HPIO_ZMATCH,
                    format!("HPIO_ZMATCH_BLK_HCLK_X{sx}Y{sy}"),
                );
            }
            "HRIO" => {
                let name = format!("HRIO_L_X{x}Y{y}", x = if x > 0 { x - 1 } else { x });
                let naming = if io_grid.is_cfg_io_hrio {
                    "HRIO"
                } else {
                    "HRIO.NOCFG"
                };
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let iox = io_grid.hpio_xlut[col];
                let (ioy_b, ioy_t) = match row.to_idx() % 60 {
                    0 => (io_grid.hpio_ylut[die][reg].0, io_grid.hpio_ylut[die][reg].1),
                    30 => (io_grid.hpio_ylut[die][reg].2, io_grid.hpio_ylut[die][reg].3),
                    _ => unreachable!(),
                };
                let sx = hrio_grid.xlut[col];
                let sy = hrio_grid.ylut[die][row];
                for j in 0..13 {
                    nnode.add_bel(bels::HRIOB[j], format!("IOB_X{iox}Y{y}", y = ioy_b + j));
                }
                for j in 0..13 {
                    nnode.add_bel(
                        bels::HRIOB[13 + j],
                        format!("IOB_X{iox}Y{y}", y = ioy_t + j),
                    );
                }
                for j in 0..12 {
                    nnode.add_bel(
                        bels::HRIOB_DIFF_IN[j],
                        format!("HRIODIFFINBUF_X{sx}Y{y}", y = sy * 12 + j),
                    );
                }
                for j in 0..12 {
                    nnode.add_bel(
                        bels::HRIOB_DIFF_OUT[j],
                        format!("HRIODIFFOUTBUF_X{sx}Y{y}", y = sy * 12 + j),
                    );
                }
            }
            "RCLK_HRIO" => {
                let name = format!(
                    "RCLK_HRIO_L_X{x}Y{y}",
                    x = if x > 0 { x - 1 } else { x },
                    y = y - 1
                );
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                for i in 0..8 {
                    nnode.add_bel(
                        bels::ABUS_SWITCH_HRIO[i],
                        format!(
                            "ABUS_SWITCH_X{x}Y{y}",
                            x = aswitch_grid.xlut[col].io + i,
                            y = aswitch_grid.ylut[die][reg].hrio
                        ),
                    );
                }
            }
            "HPIO" => {
                let tk = if chip.col_side(col) == DirH::W {
                    "HPIO_L"
                } else {
                    "HPIO_RIGHT"
                };
                let naming = if chip.has_csec {
                    "HPIO.NOAMS"
                } else if chip.is_nocfg() {
                    "HPIO.NOCFG"
                } else if chip.ps.is_some()
                    && (chip.is_alt_cfg || !edev.disabled.contains(&DisabledPart::Ps))
                {
                    "HPIO.ALTCFG"
                } else {
                    "HPIO"
                };
                let name = format!(
                    "{tk}_X{x}Y{y}",
                    x = if x > 0 && chip.col_side(col) == DirH::W {
                        x - 1
                    } else {
                        x
                    }
                );
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let iox = io_grid.hpio_xlut[col];
                let (ioy_b, ioy_t) = match row.to_idx() % 60 {
                    0 => (io_grid.hpio_ylut[die][reg].0, io_grid.hpio_ylut[die][reg].1),
                    30 => (io_grid.hpio_ylut[die][reg].2, io_grid.hpio_ylut[die][reg].3),
                    _ => unreachable!(),
                };
                let sx = hpio_grid.xlut[col];
                let sy = hpio_grid.ylut[die][row];
                for j in 0..13 {
                    nnode.add_bel(bels::HPIOB[j], format!("IOB_X{iox}Y{y}", y = ioy_b + j));
                }
                for j in 0..13 {
                    nnode.add_bel(
                        bels::HPIOB[13 + j],
                        format!("IOB_X{iox}Y{y}", y = ioy_t + j),
                    );
                }
                for j in 0..12 {
                    nnode.add_bel(
                        bels::HPIOB_DIFF_IN[j],
                        format!("HPIOBDIFFINBUF_X{sx}Y{y}", y = sy * 12 + j),
                    );
                }
                for j in 0..12 {
                    nnode.add_bel(
                        bels::HPIOB_DIFF_OUT[j],
                        format!("HPIOBDIFFOUTBUF_X{sx}Y{y}", y = sy * 12 + j),
                    );
                }
                for j in 0..2 {
                    nnode.add_bel(
                        bels::HPIOB_DCI[j],
                        format!("HPIOB_DCI_SNGL_X{sx}Y{y}", y = sy * 2 + j),
                    );
                }
                nnode.add_bel(bels::HPIO_VREF, format!("HPIO_VREF_SITE_X{sx}Y{sy}"));
                nnode.add_bel(bels::HPIO_BIAS, format!("BIAS_X{sx}Y{sy}"));
            }
            "RCLK_HPIO" => {
                let name = format!(
                    "{kind}_{lr}_X{x}Y{y}",
                    lr = if chip.col_side(col) == DirH::W {
                        'L'
                    } else {
                        'R'
                    },
                    x = if x > 0 && chip.col_side(col) == DirH::W {
                        x - 1
                    } else {
                        x
                    },
                    y = y - 1
                );
                let nnode = ngrid.name_tile(tcrd, kind, [name]);
                let sx = rclk_hpio_grid.xlut[col];
                let sy = rclk_hpio_grid.ylut[die][row];
                let asx = if chip.col_side(col) == DirH::W {
                    aswitch_grid.xlut[col].io
                } else {
                    aswitch_grid.xlut[col].io + 1
                };
                for i in 0..7 {
                    let idx = if chip.col_side(col) == DirH::W {
                        i
                    } else {
                        [0, 6, 1, 3, 2, 4, 5][i]
                    };
                    nnode.add_bel(
                        bels::ABUS_SWITCH_HPIO[i],
                        format!(
                            "ABUS_SWITCH_X{x}Y{y}",
                            x = asx + idx,
                            y = aswitch_grid.ylut[die][reg].hpio
                        ),
                    );
                }
                nnode.add_bel(
                    bels::HPIO_ZMATCH,
                    format!("HPIO_ZMATCH_BLK_HCLK_X{sx}Y{sy}"),
                );
                nnode.add_bel(bels::HPIO_PRBS, format!("HPIO_RCLK_PRBS_X{sx}Y{sy}"));
            }

            "GTH" | "GTY" | "GTF" | "GTM" | "HSADC" | "HSDAC" | "RFADC" | "RFDAC" => {
                let (tk, naming, gtk_grid) = match (chip.kind, &kind[..], chip.col_side(col)) {
                    (ChipKind::Ultrascale, "GTH", DirH::W) => {
                        ("GTH_QUAD_LEFT_FT", "GTH_L", &gth_grid)
                    }
                    (ChipKind::Ultrascale, "GTY", DirH::W) => {
                        ("GTY_QUAD_LEFT_FT", "GTY_L", &gty_grid)
                    }
                    (ChipKind::Ultrascale, "GTH", DirH::E) => ("GTH_R", "GTH_R", &gth_grid),
                    (ChipKind::UltrascalePlus, "GTH", DirH::W) => {
                        ("GTH_QUAD_LEFT", "GTH_L", &gth_grid)
                    }
                    (ChipKind::UltrascalePlus, "GTH", DirH::E) => {
                        ("GTH_QUAD_RIGHT", "GTH_R", &gth_grid)
                    }
                    (ChipKind::UltrascalePlus, "GTY", DirH::W) => ("GTY_L", "GTY_L", &gty_grid),
                    (ChipKind::UltrascalePlus, "GTY", DirH::E) => ("GTY_R", "GTY_R", &gty_grid),
                    (ChipKind::UltrascalePlus, "GTF", DirH::W) => {
                        ("GTFY_QUAD_LEFT_FT", "GTF_L", &gtf_grid)
                    }
                    (ChipKind::UltrascalePlus, "GTF", DirH::E) => {
                        ("GTFY_QUAD_RIGHT_FT", "GTF_R", &gtf_grid)
                    }
                    (ChipKind::UltrascalePlus, "GTM", DirH::W) => {
                        ("GTM_DUAL_LEFT_FT", "GTM_L", &gtm_grid)
                    }
                    (ChipKind::UltrascalePlus, "GTM", DirH::E) => {
                        ("GTM_DUAL_RIGHT_FT", "GTM_R", &gtm_grid)
                    }
                    (ChipKind::UltrascalePlus, "HSADC", DirH::E) => {
                        ("HSADC_HSADC_RIGHT_FT", "HSADC_R", &hsadc_grid)
                    }
                    (ChipKind::UltrascalePlus, "HSDAC", DirH::E) => {
                        ("HSDAC_HSDAC_RIGHT_FT", "HSDAC_R", &hsdac_grid)
                    }
                    (ChipKind::UltrascalePlus, "RFADC", DirH::E) => {
                        ("RFADC_RFADC_RIGHT_FT", "RFADC_R", &rfadc_grid)
                    }
                    (ChipKind::UltrascalePlus, "RFDAC", DirH::E) => {
                        ("RFDAC_RFDAC_RIGHT_FT", "RFDAC_R", &rfdac_grid)
                    }
                    _ => unreachable!(),
                };
                let name = format!("{tk}_X{x}Y{y}", y = y - 30);
                let nnode = ngrid.name_tile(tcrd, naming, [name]);
                let gtx = gt_grid.xlut[col];
                let gty = gt_grid.ylut[die][row];
                for i in 0..24 {
                    nnode.add_bel(
                        bels::BUFG_GT[i],
                        format!("BUFG_GT_X{gtx}Y{y}", y = gty * 24 + i),
                    );
                }
                if chip.kind == ChipKind::Ultrascale {
                    for i in 0..11 {
                        nnode.add_bel(
                            bels::BUFG_GT_SYNC[i],
                            format!("BUFG_GT_SYNC_X{gtx}Y{y}", y = gty * 11 + i),
                        );
                    }
                    for i in 0..4 {
                        nnode.add_bel(
                            bels::ABUS_SWITCH_GT[i],
                            format!(
                                "ABUS_SWITCH_X{x}Y{y}",
                                x = aswitch_grid.xlut[col].gt,
                                y = aswitch_grid.ylut[die][reg].gt + i
                            ),
                        );
                    }
                    let gtkx = gtk_grid.xlut[col];
                    let gtky = gtk_grid.ylut[die][row];
                    let (common, channel) = match kind.as_str() {
                        "GTH" => (bels::GTH_COMMON, bels::GTH_CHANNEL),
                        "GTY" => (bels::GTY_COMMON, bels::GTY_CHANNEL),
                        _ => unreachable!(),
                    };
                    for i in 0..4 {
                        nnode.add_bel(
                            channel[i],
                            format!("{kind}E3_CHANNEL_X{gtkx}Y{y}", y = gtky * 4 + i),
                        );
                    }
                    nnode.add_bel(common, format!("{kind}E3_COMMON_X{gtkx}Y{gtky}"));
                } else {
                    for i in 0..15 {
                        nnode.add_bel(
                            bels::BUFG_GT_SYNC[i],
                            format!("BUFG_GT_SYNC_X{gtx}Y{y}", y = gty * 15 + i),
                        );
                    }
                    for i in 0..5 {
                        nnode.add_bel(
                            bels::ABUS_SWITCH_GT[i],
                            format!(
                                "ABUS_SWITCH_X{x}Y{y}",
                                x = aswitch_grid.xlut[col].gt,
                                y = aswitch_grid.ylut[die][reg].gt + i
                            ),
                        );
                    }
                    let gtkx = gtk_grid.xlut[col];
                    let gtky = gtk_grid.ylut[die][row];
                    if kind == "GTM" {
                        nnode.add_bel(bels::GTM_DUAL, format!("GTM_DUAL_X{gtkx}Y{gtky}"));
                        nnode.add_bel(bels::GTM_REFCLK, format!("GTM_REFCLK_X{gtkx}Y{gtky}"));
                    } else if kind.starts_with("GT") {
                        let (common, channel) = match kind.as_str() {
                            "GTH" => (bels::GTH_COMMON, bels::GTH_CHANNEL),
                            "GTY" => (bels::GTY_COMMON, bels::GTY_CHANNEL),
                            "GTF" => (bels::GTF_COMMON, bels::GTF_CHANNEL),
                            _ => unreachable!(),
                        };
                        let pref = if kind == "GTF" {
                            "GTF".to_string()
                        } else {
                            format!("{kind}E4")
                        };
                        for i in 0..4 {
                            nnode.add_bel(
                                channel[i],
                                format!("{pref}_CHANNEL_X{gtkx}Y{y}", y = gtky * 4 + i),
                            );
                        }
                        nnode.add_bel(common, format!("{pref}_COMMON_X{gtkx}Y{gtky}"));
                    } else {
                        let slot = match kind.as_str() {
                            "HSDAC" => bels::HSDAC,
                            "HSADC" => bels::HSADC,
                            "RFDAC" => bels::RFDAC,
                            "RFADC" => bels::RFADC,
                            _ => unreachable!(),
                        };
                        nnode.add_bel(slot, format!("{kind}_X{gtkx}Y{gtky}"));
                    }
                }
            }

            "PS" => {
                let nnode = ngrid.name_tile(tcrd, "PS", [format!("PSS_ALTO_X0Y{y}")]);
                nnode.add_bel(bels::PS, "PS8_X0Y0".to_string());
            }
            "VCU" => {
                let nnode = ngrid.name_tile(tcrd, "VCU", [format!("VCU_VCU_FT_X0Y{y}")]);
                nnode.add_bel(bels::VCU, "VCU_X0Y0".to_string());
            }
            "RCLK_PS" => {
                let tk = match chip.ps.as_ref().unwrap().intf_kind {
                    PsIntfKind::Alto => "RCLK_INTF_LEFT_TERM_ALTO",
                    PsIntfKind::Da6 => "RCLK_RCLK_INTF_LEFT_TERM_DA6_FT",
                    PsIntfKind::Da7 => "RCLK_INTF_LEFT_TERM_DA7",
                    PsIntfKind::Da8 => "RCLK_RCLK_INTF_LEFT_TERM_DA8_FT",
                    PsIntfKind::Dc12 => "RCLK_RCLK_INTF_LEFT_TERM_DC12_FT",
                    PsIntfKind::Mx8 => "RCLK_RCLK_INTF_LEFT_TERM_MX8_FT",
                };
                let nnode = ngrid.name_tile(tcrd, "RCLK_PS", [format!("{tk}_X{x}Y{y}", y = y - 1)]);
                let by = rclk_ps_grid.ylut[die][row];
                for i in 0..24 {
                    nnode.add_bel(bels::BUFG_PS[i], format!("BUFG_PS_X0Y{y}", y = by * 24 + i));
                }
            }
            "BLI" => {
                let nnode = ngrid.name_tile(tcrd, "BLI", [format!("BLI_BLI_FT_X{x}Y{y}")]);
                let dx = dsp_grid.xlut[col];
                nnode.add_bel(bels::BLI_HBM_APB_INTF, format!("BLI_HBM_APB_INTF_X{dx}Y0"));
                nnode.add_bel(bels::BLI_HBM_AXI_INTF, format!("BLI_HBM_AXI_INTF_X{dx}Y0"));
            }

            "RCLK_V_SINGLE.CLE" => {
                let is_l = col < chip.col_cfg();
                if chip.col_side(col) == DirH::W {
                    let tk = match (chip.kind, chip.columns[col].kind, is_l) {
                        (ChipKind::Ultrascale, ColumnKind::CleL(_), true) => "RCLK_CLEL_L",
                        (ChipKind::Ultrascale, ColumnKind::CleL(_), false) => "RCLK_CLEL_R",
                        (ChipKind::Ultrascale, ColumnKind::CleM(_), true) => "RCLK_CLE_M_L",
                        (ChipKind::Ultrascale, ColumnKind::CleM(_), false) => "RCLK_CLE_M_R",
                        (ChipKind::UltrascalePlus, ColumnKind::CleL(_), true) => "RCLK_CLEL_L_L",
                        (ChipKind::UltrascalePlus, ColumnKind::CleL(_), false) => "RCLK_CLEL_L_R",
                        (ChipKind::UltrascalePlus, ColumnKind::CleM(subkind), true) => {
                            if chip.is_dmc && subkind == CleMKind::Laguna {
                                "RCLK_CLEM_DMC_L"
                            } else {
                                "RCLK_CLEM_L"
                            }
                        }
                        (ChipKind::UltrascalePlus, ColumnKind::CleM(_), false) => "RCLK_CLEM_R",
                        _ => unreachable!(),
                    };
                    let is_alt = dev_naming.rclk_alt_pins[tk];
                    let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                    let nnode = ngrid.name_tile(
                        tcrd,
                        if is_alt {
                            "RCLK_V_SINGLE.ALT"
                        } else {
                            "RCLK_V_SINGLE"
                        },
                        [name],
                    );
                    let reg = chip.row_to_reg(row);
                    let mut brx = clk_grid.brxlut[col];
                    let bry = clk_grid.brylut[die][reg];
                    let mut gtbx = clk_grid.gtbxlut[col];
                    let gtby = clk_grid.gtbylut[die][reg].1;
                    if chip.kind == ChipKind::UltrascalePlus
                        && chip.columns[col].kind == ColumnKind::CleM(CleMKind::Laguna)
                    {
                        brx += 1;
                        gtbx += 1;
                    }
                    match chip.kind {
                        ChipKind::Ultrascale => nnode.add_bel(
                            bels::BUFCE_ROW_RCLK[0],
                            format!("BUFCE_ROW_X{brx}Y{y}", y = bry * 25 + 24),
                        ),
                        ChipKind::UltrascalePlus => nnode.add_bel(
                            bels::BUFCE_ROW_RCLK[0],
                            format!("BUFCE_ROW_FSR_X{brx}Y{bry}"),
                        ),
                    }
                    nnode.add_bel(
                        bels::GCLK_TEST_BUF_RCLK[0],
                        format!("GCLK_TEST_BUFE3_X{gtbx}Y{gtby}"),
                    );
                } else {
                    let tk = if is_l {
                        "RCLK_CLEL_R_L"
                    } else {
                        "RCLK_CLEL_R_R"
                    };
                    let is_alt = dev_naming.rclk_alt_pins[tk];
                    let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                    let nnode = ngrid.name_tile(
                        tcrd,
                        if is_alt {
                            "RCLK_V_SINGLE.ALT"
                        } else {
                            "RCLK_V_SINGLE"
                        },
                        [name],
                    );
                    let brx = clk_grid.brxlut[col];
                    let bry = clk_grid.brylut[die][reg];
                    nnode.add_bel(
                        bels::BUFCE_ROW_RCLK[0],
                        format!("BUFCE_ROW_X{brx}Y{y}", y = bry * 25 + 24),
                    );
                    let gtbx = clk_grid.gtbxlut[col];
                    let gtby = clk_grid.gtbylut[die][reg].1;
                    nnode.add_bel(
                        bels::GCLK_TEST_BUF_RCLK[0],
                        format!("GCLK_TEST_BUFE3_X{gtbx}Y{gtby}"),
                    );
                }
            }
            "RCLK_V_SINGLE.LAG" => {
                let is_l = col < chip.col_cfg();
                let tk = if is_l {
                    if chip.is_dmc {
                        "RCLK_LAG_DMC_L"
                    } else {
                        "RCLK_LAG_L"
                    }
                } else {
                    "RCLK_LAG_R"
                };
                let is_alt = dev_naming.rclk_alt_pins[tk];
                let name = format!("{tk}_X{x}Y{y}", x = x - 1, y = y - 1);
                let nnode = ngrid.name_tile(
                    tcrd,
                    if is_alt {
                        "RCLK_V_SINGLE.ALT"
                    } else {
                        "RCLK_V_SINGLE"
                    },
                    [name],
                );
                let brx = clk_grid.brxlut[col];
                let bry = clk_grid.brylut[die][reg];
                nnode.add_bel(
                    bels::BUFCE_ROW_RCLK[0],
                    format!("BUFCE_ROW_FSR_X{brx}Y{bry}"),
                );
                let gtbx = clk_grid.gtbxlut[col];
                let gtby = clk_grid.gtbylut[die][reg].1;
                nnode.add_bel(
                    bels::GCLK_TEST_BUF_RCLK[0],
                    format!("GCLK_TEST_BUFE3_X{gtbx}Y{gtby}"),
                );
            }

            "RCLK_V_DOUBLE.BRAM" => {
                let tk = get_bram_tk(edev, has_laguna, die, col, row);
                let is_alt = dev_naming.rclk_alt_pins[tk];
                let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                let nnode = ngrid.name_tile(
                    tcrd,
                    if is_alt {
                        "RCLK_V_DOUBLE.ALT"
                    } else {
                        "RCLK_V_DOUBLE"
                    },
                    [name],
                );
                let brx = clk_grid.brxlut[col];
                let bry = clk_grid.brylut[die][reg];
                for i in 0..2 {
                    nnode.add_bel(
                        bels::BUFCE_ROW_RCLK[i],
                        format!("BUFCE_ROW_X{x}Y{y}", x = brx + i, y = bry * 25 + 24),
                    );
                }
                let gtbx = clk_grid.gtbxlut[col];
                let gtby = clk_grid.gtbylut[die][reg].1;
                for i in 0..2 {
                    nnode.add_bel(
                        bels::GCLK_TEST_BUF_RCLK[i],
                        format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                    );
                }
            }
            "RCLK_V_DOUBLE.DSP" => {
                let mut brx = clk_grid.brxlut[col];
                let mut gtbx = clk_grid.gtbxlut[col];
                let tk = match chip.kind {
                    ChipKind::Ultrascale => "RCLK_DSP_L",
                    ChipKind::UltrascalePlus => {
                        let is_l = col < chip.col_cfg();
                        let mut is_dc12 = chip.is_dc12();
                        if chip.is_nocfg() && !chip.has_csec {
                            if col < chip.cols_hard.first().unwrap().col {
                                is_dc12 = true;
                            }
                            if col > chip.cols_hard.last().unwrap().col {
                                if reg == chip.reg_cfg() {
                                    is_dc12 = true;
                                } else {
                                    brx += 2;
                                    gtbx += 2;
                                }
                            }
                        }
                        if matches!(chip.columns.last().unwrap().kind, ColumnKind::Hard(_, _))
                            && col > chip.cols_hard.first().unwrap().col
                        {
                            if reg != chip.reg_cfg() {
                                is_dc12 = true;
                            } else {
                                brx += 2;
                                gtbx += 2;
                            }
                        }
                        if is_dc12 {
                            if is_l {
                                "RCLK_RCLK_DSP_INTF_DC12_L_FT"
                            } else {
                                "RCLK_RCLK_DSP_INTF_DC12_R_FT"
                            }
                        } else {
                            if is_l {
                                "RCLK_DSP_INTF_L"
                            } else {
                                "RCLK_DSP_INTF_R"
                            }
                        }
                    }
                };
                let is_alt = dev_naming.rclk_alt_pins[tk];
                let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                let nnode = ngrid.name_tile(
                    tcrd,
                    if is_alt {
                        "RCLK_V_DOUBLE.ALT"
                    } else {
                        "RCLK_V_DOUBLE"
                    },
                    [name],
                );
                let bry = clk_grid.brylut[die][reg];
                for i in 0..2 {
                    match chip.kind {
                        ChipKind::Ultrascale => nnode.add_bel(
                            bels::BUFCE_ROW_RCLK[i],
                            format!("BUFCE_ROW_X{x}Y{y}", x = brx + i, y = bry * 25 + 24),
                        ),
                        ChipKind::UltrascalePlus => nnode.add_bel(
                            bels::BUFCE_ROW_RCLK[i],
                            format!("BUFCE_ROW_FSR_X{x}Y{bry}", x = brx + i,),
                        ),
                    }
                }
                let gtby = clk_grid.gtbylut[die][reg].1;
                for i in 0..2 {
                    nnode.add_bel(
                        bels::GCLK_TEST_BUF_RCLK[i],
                        format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                    );
                }
            }
            "RCLK_V_QUAD.BRAM" => {
                let tk = get_bram_tk(edev, has_laguna, die, col, row);
                let is_alt = dev_naming.rclk_alt_pins[tk];
                let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                let nnode = ngrid.name_tile(
                    tcrd,
                    if is_alt {
                        "RCLK_V_QUAD.BRAM.ALT"
                    } else {
                        "RCLK_V_QUAD.BRAM"
                    },
                    [name],
                );
                let brx = clk_grid.brxlut[col];
                let bry = clk_grid.brylut[die][reg];
                for i in 0..4 {
                    nnode.add_bel(
                        bels::BUFCE_ROW_RCLK[i],
                        format!("BUFCE_ROW_FSR_X{x}Y{bry}", x = brx + i,),
                    );
                }
                let gtbx = clk_grid.gtbxlut[col];
                let gtby = clk_grid.gtbylut[die][reg].1;
                for i in 0..4 {
                    nnode.add_bel(
                        bels::GCLK_TEST_BUF_RCLK[i],
                        format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                    );
                }

                let vsx = clk_grid.vsxlut[col];
                let vsy = clk_grid.brylut[die][reg] * 2;
                for i in 0..3 {
                    nnode.add_bel(
                        bels::VBUS_SWITCH[i],
                        format!("VBUS_SWITCH_X{x}Y{y}", x = vsx + i / 2, y = vsy + i % 2),
                    );
                }
            }
            "RCLK_V_QUAD.URAM" => {
                let tk = "RCLK_RCLK_URAM_INTF_L_FT";
                let is_alt = dev_naming.rclk_alt_pins[tk];
                let name = format!("{tk}_X{x}Y{y}", x = x - 1, y = y - 1);
                let nnode = ngrid.name_tile(
                    tcrd,
                    if is_alt {
                        "RCLK_V_QUAD.URAM.ALT"
                    } else {
                        "RCLK_V_QUAD.URAM"
                    },
                    [name],
                );
                let brx = clk_grid.brxlut[col];
                let bry = clk_grid.brylut[die][reg];
                for i in 0..4 {
                    nnode.add_bel(
                        bels::BUFCE_ROW_RCLK[i],
                        format!("BUFCE_ROW_FSR_X{x}Y{bry}", x = brx + i,),
                    );
                }
                let gtbx = clk_grid.gtbxlut[col];
                let gtby = clk_grid.gtbylut[die][reg].1;
                for i in 0..4 {
                    nnode.add_bel(
                        bels::GCLK_TEST_BUF_RCLK[i],
                        format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                    );
                }

                let vsx = clk_grid.vsxlut[col];
                let vsy = clk_grid.brylut[die][reg] * 2;
                for i in 0..3 {
                    nnode.add_bel(
                        bels::VBUS_SWITCH[i],
                        format!("VBUS_SWITCH_X{x}Y{y}", x = vsx + i / 2, y = vsy + i % 2),
                    );
                }
            }
            "RCLK_SPLITTER" => {
                let tk = match chip.kind {
                    ChipKind::Ultrascale => "RCLK_DSP_CLKBUF_L",
                    ChipKind::UltrascalePlus => "RCLK_DSP_INTF_CLKBUF_L",
                };
                ngrid.name_tile(tcrd, "RCLK_SPLITTER", [format!("{tk}_X{x}Y{y}", y = y - 1)]);
            }
            "RCLK_HROUTE_SPLITTER.CLE" => {
                ngrid.name_tile(
                    tcrd,
                    "RCLK_HROUTE_SPLITTER",
                    [format!("RCLK_CLEM_CLKBUF_L_X{x}Y{y}", y = y - 1)],
                );
            }
            "RCLK_HROUTE_SPLITTER.HARD" => {
                let name = match chip.columns[col].kind {
                    ColumnKind::Hard(_, idx) => {
                        let col_hard = &chip.cols_hard[idx];
                        match (chip.kind, col_hard.regs[reg]) {
                            (ChipKind::Ultrascale, HardRowKind::Cfg) => {
                                format!("CFG_CFG_X{x}Y{y}", y = y - 30)
                            }
                            (_, HardRowKind::Ams) => {
                                let x = if chip.kind == ChipKind::UltrascalePlus
                                    && (!hdio_cfg_only[die][idx] || chip.has_csec)
                                {
                                    x + 1
                                } else {
                                    x
                                };
                                format!("RCLK_AMS_CFGIO_X{x}Y{y}", y = y - 1)
                            }
                            (ChipKind::Ultrascale, HardRowKind::Pcie) => {
                                format!("PCIE_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::Ultrascale, HardRowKind::Cmac) => {
                                let x = if col == chip.col_cfg() { x } else { x + 1 };
                                format!("CMAC_CMAC_FT_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::Ultrascale, HardRowKind::Ilkn) => {
                                format!("ILMAC_ILMAC_FT_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::UltrascalePlus, HardRowKind::Cfg) => {
                                let x = if hdio_cfg_only[die][idx] { x } else { x + 1 };
                                let tkn = if chip.has_csec {
                                    "CSEC_CONFIG_FT"
                                } else {
                                    "CFG_CONFIG"
                                };
                                format!("{tkn}_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::UltrascalePlus, HardRowKind::Pcie) => {
                                format!("PCIE4_PCIE4_FT_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::UltrascalePlus, HardRowKind::PciePlus) => {
                                format!("PCIE4C_PCIE4C_FT_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::UltrascalePlus, HardRowKind::Cmac) => {
                                format!("CMAC_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::UltrascalePlus, HardRowKind::Ilkn) => {
                                format!("ILKN_ILKN_FT_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::UltrascalePlus, HardRowKind::DfeA) => {
                                format!("DFE_DFE_TILEA_FT_X{x}Y{y}", y = y - 30)
                            }
                            (ChipKind::UltrascalePlus, HardRowKind::DfeG) => {
                                format!("DFE_DFE_TILEG_FT_X{x}Y{y}", y = y - 30)
                            }
                            _ => unreachable!(),
                        }
                    }
                    ColumnKind::DfeE => {
                        format!("DFE_DFE_TILEE_FT_X{x}Y{y}", y = y - 30)
                    }
                    ColumnKind::DfeB => {
                        format!("DFE_DFE_TILEB_FT_X{x}Y{y}", y = y - 30)
                    }
                    _ => unreachable!(),
                };
                ngrid.name_tile(tcrd, "RCLK_HROUTE_SPLITTER", [name]);
            }
            "HBM_ABUS_SWITCH" => {
                let nnode = ngrid.name_tile(
                    tcrd,
                    kind,
                    [format!("CFRM_CFRAME_TERM_H_FT_X{x}Y{y}", x = x + 1)],
                );
                let asx = aswitch_grid.xlut[col].hbm;
                for i in 0..8 {
                    nnode.add_bel(
                        bels::ABUS_SWITCH_HBM[i],
                        format!("ABUS_SWITCH_X{x}Y{y}", x = asx + i / 2, y = i % 2),
                    );
                }
            }

            _ => panic!("how to {kind}"),
        }
    }

    ExpandedNamedDevice { edev, ngrid }
}
