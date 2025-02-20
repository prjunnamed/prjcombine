use std::collections::{BTreeMap, BTreeSet};

use prjcombine_interconnect::{
    db::{NodeKind, NodeKindId},
    grid::{ColId, DieId, ExpandedGrid, RowId},
};
use prjcombine_ultrascale::{
    expanded::{ExpandedDevice, GtCoord, IoCoord},
    grid::{
        BramKind, CleMKind, ColSide, ColumnKindLeft, ColumnKindRight, DisabledPart, DspKind,
        GridKind, HardKind, HardRowKind, IoRowKind, PsIntfKind, RegId,
    },
};
use prjcombine_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeviceNaming {
    pub rclk_alt_pins: BTreeMap<String, bool>,
}

#[derive(Clone, Debug)]
struct BelMultiGridBi {
    pub xlut_l: EntityPartVec<ColId, usize>,
    pub xlut_r: EntityPartVec<ColId, usize>,
    pub ylut: EntityVec<DieId, EntityPartVec<RowId, usize>>,
}

impl BelMultiGridBi {
    pub fn new(
        egrid: &ExpandedGrid,
        fl: impl Fn(NodeKindId, &str, &NodeKind) -> bool,
        fr: impl Fn(NodeKindId, &str, &NodeKind) -> bool,
    ) -> Self {
        let mut cols = BTreeSet::new();
        let mut rows = BTreeSet::new();
        for (kind, name, node) in &egrid.db.nodes {
            if fl(kind, name, node) {
                for &nloc in &egrid.node_index[kind] {
                    cols.insert((nloc.1, ColSide::Left));
                    rows.insert((nloc.0, nloc.2));
                }
            }
            if fr(kind, name, node) {
                for &nloc in &egrid.node_index[kind] {
                    cols.insert((nloc.1, ColSide::Right));
                    rows.insert((nloc.0, nloc.2));
                }
            }
        }
        let mut xlut_l = EntityPartVec::new();
        let mut xlut_r = EntityPartVec::new();
        let mut ylut: EntityVec<_, _> = egrid.die.ids().map(|_| EntityPartVec::new()).collect();
        for (i, (col, side)) in cols.into_iter().enumerate() {
            match side {
                ColSide::Left => {
                    xlut_l.insert(col, i);
                }
                ColSide::Right => {
                    xlut_r.insert(col, i);
                }
            }
        }
        for (i, (die, row)) in rows.into_iter().enumerate() {
            ylut[die].insert(row, i);
        }
        BelMultiGridBi {
            xlut_l,
            xlut_r,
            ylut,
        }
    }
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
    let dev_has_hbm = edev.grids.first().unwrap().has_hbm;
    let pgrid = edev.grids[edev.interposer.primary];
    for &cd in pgrid.columns.values() {
        let cfg = asx;
        let gt = asx;
        let mut hdio = asx;
        let mut io = asx;
        let mut hbm = asx;
        match cd.l {
            ColumnKindLeft::Gt(idx) | ColumnKindLeft::Io(idx) => {
                let regs = &pgrid.cols_io[idx].regs;
                let has_hpio = regs.values().any(|&x| x == IoRowKind::Hpio);
                let has_hrio = regs.values().any(|&x| x == IoRowKind::Hrio);
                let has_hdio = regs.values().any(|&x| x == IoRowKind::HdioLc);
                let has_gt = regs.values().any(|&x| {
                    !matches!(
                        x,
                        IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio | IoRowKind::HdioLc
                    )
                });
                if has_gt {
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
                        GridKind::Ultrascale => asx += 5,
                        GridKind::UltrascalePlus => asx += 8,
                    }
                }
            }
            _ => (),
        }
        match cd.r {
            ColumnKindRight::Gt(idx) | ColumnKindRight::Io(idx) => {
                let regs = &pgrid.cols_io[idx].regs;
                let has_hpio = regs.values().any(|&x| x == IoRowKind::Hpio);
                let has_hrio = regs.values().any(|&x| x == IoRowKind::Hrio);
                let has_gt = regs
                    .values()
                    .any(|&x| !matches!(x, IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio));
                if has_hrio {
                    asx += 8;
                } else if has_hpio {
                    match edev.kind {
                        GridKind::Ultrascale => asx += 5,
                        GridKind::UltrascalePlus => asx += 8,
                    }
                } else if has_gt {
                    asx += 1;
                }
            }
            ColumnKindRight::Hard(_, idx) => {
                let regs = &pgrid.cols_hard[idx].regs;
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

    let mut ylut: EntityVec<_, _> = edev.grids.ids().map(|_| EntityVec::new()).collect();

    let mut asy = if dev_has_hbm { 2 } else { 0 };
    for (die, &grid) in &edev.grids {
        for reg in grid.regs() {
            let skip = edev.disabled.contains(&DisabledPart::Region(die, reg));
            let has_hdio = grid.cols_hard.iter().any(|x| {
                matches!(
                    x.regs[reg],
                    HardRowKind::Hdio | HardRowKind::HdioAms | HardRowKind::HdioLc
                )
            }) && !skip;
            let has_cfg = grid
                .cols_hard
                .iter()
                .any(|x| x.regs[reg] == HardRowKind::Cfg)
                && !skip;
            let has_hpio = grid.cols_io.iter().any(|x| x.regs[reg] == IoRowKind::Hpio) && !skip;
            let has_hrio = grid.cols_io.iter().any(|x| x.regs[reg] == IoRowKind::Hrio) && !skip;
            let has_hdiolc_l = grid
                .cols_io
                .iter()
                .any(|x| x.regs[reg] == IoRowKind::HdioLc)
                && !skip;
            let has_gt = grid.cols_io.iter().any(|x| {
                !matches!(
                    x.regs[reg],
                    IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio | IoRowKind::HdioLc
                )
            }) && !skip;

            let cfg = asy;
            let mut cmt = asy;
            if has_cfg || (grid.kind == GridKind::UltrascalePlus && (has_hpio || has_hdiolc_l)) {
                asy += 1;
            }
            let gt = asy;
            if has_gt {
                asy += match grid.kind {
                    GridKind::Ultrascale => 4,
                    GridKind::UltrascalePlus => 5,
                };
            }
            if grid.kind == GridKind::Ultrascale {
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
    brxlut: EntityVec<ColId, (usize, usize)>,
    gtbxlut: EntityVec<ColId, (usize, usize)>,
    gtbylut: EntityVec<DieId, EntityVec<RegId, (usize, usize)>>,
    brylut: EntityVec<DieId, EntityVec<RegId, usize>>,
    vsxlut: EntityVec<ColId, usize>,
}

fn make_clk_grid(edev: &ExpandedDevice) -> ClkGrid {
    let mut brxlut = EntityVec::new();
    let mut gtbxlut = EntityVec::new();
    let mut vsxlut = EntityVec::new();
    let pgrid = edev.grids[edev.interposer.primary];

    let mut brx = 0;
    let mut gtbx = 0;
    let mut vsx = 0;
    for (col, &cd) in &pgrid.columns {
        vsxlut.push(vsx);
        let lbrx = brx;
        let lgtbx = gtbx;
        match cd.l {
            ColumnKindLeft::CleM(CleMKind::ClkBuf) => (),
            ColumnKindLeft::CleM(CleMKind::Laguna) if edev.kind == GridKind::UltrascalePlus => {
                brx += 2;
                gtbx += 2;
            }
            ColumnKindLeft::CleL | ColumnKindLeft::CleM(_) => {
                // skip leftmost column on whole-height PS devices
                if col.to_idx() != 0 {
                    brx += 1;
                    gtbx += 1;
                }
            }
            ColumnKindLeft::Bram(_) | ColumnKindLeft::Uram => match edev.kind {
                GridKind::Ultrascale => {
                    brx += 2;
                    gtbx += 2;
                }
                GridKind::UltrascalePlus => {
                    brx += 4;
                    gtbx += 4;
                    vsx += 2;
                }
            },
            ColumnKindLeft::Io(_) => {
                if edev.kind == GridKind::Ultrascale {
                    brx += 1;
                }
                gtbx += 1;
            }
            _ => (),
        }
        let rbrx = brx;
        let rgtbx = gtbx;
        match cd.r {
            ColumnKindRight::CleL(_) if edev.kind == GridKind::Ultrascale => {
                brx += 1;
                gtbx += 1;
            }
            ColumnKindRight::Dsp(DspKind::ClkBuf) => (),
            ColumnKindRight::Dsp(_) => {
                if (pgrid.is_nocfg() && col > pgrid.cols_hard.last().unwrap().col)
                    || (matches!(pgrid.columns.last().unwrap().r, ColumnKindRight::Hard(_, _))
                        && col > pgrid.cols_hard.first().unwrap().col)
                {
                    brx += 4;
                    gtbx += 4;
                } else {
                    brx += 2;
                    gtbx += 2;
                }
            }
            _ => (),
        }
        brxlut.push((lbrx, rbrx));
        gtbxlut.push((lgtbx, rgtbx));
    }

    let mut gtbylut: EntityVec<_, _> = edev.grids.ids().map(|_| EntityVec::new()).collect();
    let mut brylut: EntityVec<_, _> = edev.grids.ids().map(|_| EntityVec::new()).collect();
    let mut gtby = 0;
    let mut bry = 0;
    for (die, &grid) in &edev.grids {
        for reg in grid.regs() {
            let skip = edev.disabled.contains(&DisabledPart::Region(die, reg));
            let has_hprio = grid.cols_io.iter().any(|x| {
                matches!(
                    x.regs[reg],
                    IoRowKind::Hpio | IoRowKind::Hrio | IoRowKind::HdioLc
                )
            }) && !skip;
            if has_hprio {
                match edev.kind {
                    GridKind::Ultrascale => {
                        gtbylut[die].push((gtby, gtby + 24));
                    }
                    GridKind::UltrascalePlus => {
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
    let pgrid = edev.grids[edev.interposer.primary];

    let mut iox = 0;
    let mut hpio_xlut = EntityPartVec::new();
    let mut hdio_xlut = EntityPartVec::new();
    for (col, &cd) in &pgrid.columns {
        if let ColumnKindLeft::Io(idx) = cd.l {
            let mut has_hdiolc = false;
            let mut has_hpio = false;
            for grid in edev.grids.values() {
                let iocol = &grid.cols_io[idx];
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
        match cd.r {
            ColumnKindRight::Io(_) => {
                hpio_xlut.insert(col, iox);
                iox += 1;
            }
            ColumnKindRight::Hard(_, idx) => {
                let regs = &pgrid.cols_hard[idx].regs;
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
    if let Some(ioc_cfg) = pgrid.cols_io.iter().find(|x| x.col == edev.col_cfg_io.0) {
        is_cfg_io_hrio = ioc_cfg.regs[pgrid.reg_cfg()] == IoRowKind::Hrio;
    }

    let mut hdio_ylut: EntityVec<_, _> = edev.grids.ids().map(|_| EntityPartVec::new()).collect();
    let mut hpio_ylut: EntityVec<_, _> = edev.grids.ids().map(|_| EntityPartVec::new()).collect();
    let mut ioy = 0;
    for (die, &grid) in &edev.grids {
        for reg in grid.regs() {
            let skip = edev.disabled.contains(&DisabledPart::Region(die, reg));
            let has_hdio = grid
                .cols_hard
                .iter()
                .any(|x| matches!(x.regs[reg], HardRowKind::Hdio | HardRowKind::HdioAms))
                && !skip;
            let has_hdiolc = (grid
                .cols_hard
                .iter()
                .any(|x| matches!(x.regs[reg], HardRowKind::HdioLc))
                || grid
                    .cols_io
                    .iter()
                    .any(|x| matches!(x.regs[reg], IoRowKind::HdioLc)))
                && !skip;
            let has_hprio = grid
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
                let grid = self.edev.grids[hpio.die];
                let iocol = grid
                    .cols_io
                    .iter()
                    .find(|iocol| iocol.col == hpio.col)
                    .unwrap();
                let kind = iocol.regs[hpio.reg];
                let (row, idx) = if hpio.iob.to_idx() < 26 {
                    (grid.row_reg_bot(hpio.reg), hpio.iob.to_idx())
                } else {
                    (grid.row_reg_bot(hpio.reg) + 30, hpio.iob.to_idx() - 26)
                };
                self.ngrid
                    .get_bel_name(
                        hpio.die,
                        hpio.col,
                        row,
                        &if kind == IoRowKind::Hpio {
                            format!("HPIOB{idx}")
                        } else {
                            format!("HRIOB{idx}")
                        },
                    )
                    .unwrap()
            }
            IoCoord::Hdio(hdio) => {
                let grid = self.edev.grids[hdio.die];
                let (row, idx) = if hdio.iob.to_idx() < 12 {
                    (grid.row_reg_bot(hdio.reg), hdio.iob.to_idx())
                } else {
                    (grid.row_reg_bot(hdio.reg) + 30, hdio.iob.to_idx() - 12)
                };
                let bel = if idx % 2 == 0 {
                    format!("HDIOB_M{}", idx / 2)
                } else {
                    format!("HDIOB_S{}", idx / 2)
                };
                self.ngrid
                    .get_bel_name(hdio.die, hdio.col, row, &bel)
                    .unwrap()
            }
            IoCoord::HdioLc(hdio) => {
                let grid = self.edev.grids[hdio.die];
                let (row, idx) = if hdio.iob.to_idx() < 42 {
                    (grid.row_reg_bot(hdio.reg), hdio.iob.to_idx())
                } else {
                    (grid.row_reg_bot(hdio.reg) + 30, hdio.iob.to_idx() - 42)
                };
                let bel = if idx % 2 == 0 {
                    format!("HDIOB_M{}", idx / 2)
                } else {
                    format!("HDIOB_S{}", idx / 2)
                };
                self.ngrid
                    .get_bel_name(hdio.die, hdio.col, row, &bel)
                    .unwrap()
            }
        }
    }

    pub fn get_gts(&self) -> Vec<Gt<'_>> {
        let mut res = vec![];
        for &crd in &self.edev.gt {
            let grid = self.edev.grids[crd.die];
            let gt_info = self.edev.get_gt_info(crd);
            let row = grid.row_reg_rclk(crd.reg);
            let (name_common, name_channel) = match gt_info.kind {
                IoRowKind::Gth => (
                    self.ngrid
                        .get_bel_name(crd.die, crd.col, row, "GTH_COMMON")
                        .unwrap(),
                    vec![
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTH_CHANNEL0")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTH_CHANNEL1")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTH_CHANNEL2")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTH_CHANNEL3")
                            .unwrap(),
                    ],
                ),
                IoRowKind::Gty => (
                    self.ngrid
                        .get_bel_name(crd.die, crd.col, row, "GTY_COMMON")
                        .unwrap(),
                    vec![
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTY_CHANNEL0")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTY_CHANNEL1")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTY_CHANNEL2")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTY_CHANNEL3")
                            .unwrap(),
                    ],
                ),
                IoRowKind::Gtm => (
                    self.ngrid
                        .get_bel_name(crd.die, crd.col, row, "GTM_REFCLK")
                        .unwrap(),
                    vec![self
                        .ngrid
                        .get_bel_name(crd.die, crd.col, row, "GTM_DUAL")
                        .unwrap()],
                ),
                IoRowKind::Gtf => (
                    self.ngrid
                        .get_bel_name(crd.die, crd.col, row, "GTF_COMMON")
                        .unwrap(),
                    vec![
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTF_CHANNEL0")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTF_CHANNEL1")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTF_CHANNEL2")
                            .unwrap(),
                        self.ngrid
                            .get_bel_name(crd.die, crd.col, row, "GTF_CHANNEL3")
                            .unwrap(),
                    ],
                ),
                IoRowKind::HsAdc => (
                    self.ngrid
                        .get_bel_name(crd.die, crd.col, row, "HSADC")
                        .unwrap(),
                    vec![],
                ),
                IoRowKind::HsDac => (
                    self.ngrid
                        .get_bel_name(crd.die, crd.col, row, "HSDAC")
                        .unwrap(),
                    vec![],
                ),
                IoRowKind::RfAdc => (
                    self.ngrid
                        .get_bel_name(crd.die, crd.col, row, "RFADC")
                        .unwrap(),
                    vec![],
                ),
                IoRowKind::RfDac => (
                    self.ngrid
                        .get_bel_name(crd.die, crd.col, row, "RFDAC")
                        .unwrap(),
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
    let grid = edev.grids[die];
    let in_laguna = has_laguna && grid.is_laguna_row(row);
    let cd = grid.columns[col];
    match (grid.kind, cd.l, col < grid.col_cfg()) {
        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::Plain), true) => "RCLK_BRAM_L",
        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::Plain), false) => "RCLK_BRAM_R",
        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::BramClmp), true) => {
            "RCLK_RCLK_BRAM_L_BRAMCLMP_FT"
        }
        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::AuxClmp), true) => {
            "RCLK_RCLK_BRAM_L_AUXCLMP_FT"
        }
        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::BramClmpMaybe), true) => {
            if in_laguna {
                "RCLK_BRAM_L"
            } else {
                "RCLK_RCLK_BRAM_L_BRAMCLMP_FT"
            }
        }
        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::AuxClmpMaybe), true) => {
            if in_laguna {
                "RCLK_BRAM_L"
            } else {
                "RCLK_RCLK_BRAM_L_AUXCLMP_FT"
            }
        }
        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Plain), true) => {
            "RCLK_BRAM_INTF_L"
        }
        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Plain), false) => {
            "RCLK_BRAM_INTF_R"
        }
        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Td), true) => {
            "RCLK_BRAM_INTF_TD_L"
        }
        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Td), false) => {
            "RCLK_BRAM_INTF_TD_R"
        }
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
    if edev.kind == GridKind::Ultrascale
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
    let cle_grid = BelMultiGridBi::new(
        egrid,
        |_, node, _| matches!(node, "CLEL_L" | "CLEM"),
        |_, node, _| matches!(node, "CLEL_R"),
    );
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
    let hdio_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "HDIO_BOT" | "HDIO_TOP"));
    let hdiolc_grid = ngrid.bel_multi_grid(|_, node, _| {
        matches!(
            node,
            "HDIOLC_L_BOT" | "HDIOLC_L_TOP" | "HDIOLC_R_BOT" | "HDIOLC_R_TOP"
        )
    });
    let hpio_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "HPIO" | "HPIO_L" | "HPIO_R"));
    let rclk_hpio_grid = ngrid
        .bel_multi_grid(|_, node, _| matches!(node, "RCLK_HPIO" | "RCLK_HPIO_L" | "RCLK_HPIO_R"));
    let hrio_grid = ngrid.bel_multi_grid(|_, node, _| node == "HRIO");
    let rclk_hdio_grid = ngrid.bel_multi_grid(|_, node, _| {
        matches!(node, "RCLK_HDIO" | "RCLK_HDIOLC_L" | "RCLK_HDIOLC_R")
    });
    let aswitch_grid = make_aswitch_grid(edev);
    let io_grid = make_io_grid(edev);
    let clk_grid = make_clk_grid(edev);
    let xiphy_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "XIPHY_L" | "XIPHY_R"));
    let cmt_grid = ngrid
        .bel_multi_grid(|_, node, _| matches!(node, "XIPHY" | "CMT_L" | "CMT_L_HBM" | "CMT_R"));
    let gt_grid = ngrid.bel_multi_grid(|_, node, _| {
        matches!(
            node,
            "GTH_L"
                | "GTH_R"
                | "GTY_L"
                | "GTY_R"
                | "GTM_L"
                | "GTM_R"
                | "GTF_L"
                | "GTF_R"
                | "HSADC_R"
                | "HSDAC_R"
                | "RFADC_R"
                | "RFDAC_R"
        )
    });
    let gth_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "GTH_L" | "GTH_R"));
    let gty_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "GTY_L" | "GTY_R"));
    let gtm_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "GTM_L" | "GTM_R"));
    let gtf_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "GTF_L" | "GTF_R"));
    let hsadc_grid = ngrid.bel_multi_grid(|_, node, _| node == "HSADC_R");
    let hsdac_grid = ngrid.bel_multi_grid(|_, node, _| node == "HSDAC_R");
    let rfadc_grid = ngrid.bel_multi_grid(|_, node, _| node == "RFADC_R");
    let rfdac_grid = ngrid.bel_multi_grid(|_, node, _| node == "RFDAC_R");

    for die in egrid.dies() {
        let grid = edev.grids[die.die];
        let has_laguna = grid
            .columns
            .values()
            .any(|cd| cd.l == ColumnKindLeft::CleM(CleMKind::Laguna));
        let hdio_cfg_only: Vec<_> = grid
            .cols_hard
            .iter()
            .map(|hcol| {
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
            })
            .collect();
        let has_mcap = grid.cols_hard.iter().any(|hcol| {
            hcol.regs.iter().any(|(reg, &kind)| {
                kind == HardRowKind::Cfg
                    && reg.to_idx() != 0
                    && matches!(
                        hcol.regs[reg - 1],
                        HardRowKind::Pcie | HardRowKind::PciePlus
                    )
            })
        }) && !grid.is_nocfg();
        for col in die.cols() {
            for row in die.rows() {
                let reg = grid.row_to_reg(row);
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    let x = int_grid.xlut[col];
                    let y = int_grid.ylut[die.die][row];
                    match &kind[..] {
                        "INT" => {
                            ngrid.name_node(nloc, "INT", [format!("INT_X{x}Y{y}")]);
                        }
                        "INTF.W" => match grid.kind {
                            GridKind::Ultrascale => {
                                ngrid.name_node(
                                    nloc,
                                    "INTF.W",
                                    [format!("INT_INTERFACE_L_X{x}Y{y}")],
                                );
                            }
                            GridKind::UltrascalePlus => {
                                ngrid.name_node(nloc, "INTF.W", [format!("INT_INTF_L_X{x}Y{y}")]);
                            }
                        },
                        "INTF.E" => match grid.kind {
                            GridKind::Ultrascale => {
                                ngrid.name_node(
                                    nloc,
                                    "INTF.E",
                                    [format!("INT_INTERFACE_R_X{x}Y{y}")],
                                );
                            }
                            GridKind::UltrascalePlus => {
                                ngrid.name_node(nloc, "INTF.E", [format!("INT_INTF_R_X{x}Y{y}")]);
                            }
                        },
                        "INTF.W.DELAY" => match grid.columns[col].l {
                            ColumnKindLeft::Io(_) | ColumnKindLeft::Gt(_) => {
                                let cio = grid
                                    .cols_io
                                    .iter()
                                    .find(|x| x.col == col && x.side == ColSide::Left)
                                    .unwrap();
                                match cio.regs[reg] {
                                    IoRowKind::Hpio | IoRowKind::Hrio => {
                                        ngrid.name_node(
                                            nloc,
                                            "INTF.W.IO",
                                            [format!("INT_INT_INTERFACE_XIPHY_FT_X{x}Y{y}")],
                                        );
                                    }
                                    _ => {
                                        let kind = if grid.kind == GridKind::Ultrascale {
                                            "INT_INT_INTERFACE_GT_LEFT_FT"
                                        } else {
                                            "INT_INTF_L_TERM_GT"
                                        };
                                        ngrid.name_node(
                                            nloc,
                                            "INTF.W.GT",
                                            [format!("{kind}_X{x}Y{y}")],
                                        );
                                    }
                                }
                            }
                            ColumnKindLeft::Hard(_, _)
                            | ColumnKindLeft::Sdfec
                            | ColumnKindLeft::DfeC
                            | ColumnKindLeft::DfeDF
                            | ColumnKindLeft::DfeE => {
                                let kind = if grid.kind == GridKind::Ultrascale {
                                    "INT_INTERFACE_PCIE_L"
                                } else {
                                    "INT_INTF_L_PCIE4"
                                };
                                ngrid.name_node(nloc, "INTF.W.PCIE", [format!("{kind}_X{x}Y{y}")]);
                            }
                            _ => unreachable!(),
                        },
                        "INTF.E.DELAY" => match grid.columns[col].r {
                            ColumnKindRight::Gt(_) | ColumnKindRight::Io(_) => {
                                let kind = if grid.kind == GridKind::Ultrascale {
                                    "INT_INTERFACE_GT_R"
                                } else {
                                    "INT_INTF_R_TERM_GT"
                                };
                                ngrid.name_node(nloc, "INTF.E.GT", [format!("{kind}_X{x}Y{y}")]);
                            }
                            ColumnKindRight::Hard(HardKind::Term, _) => {
                                ngrid.name_node(
                                    nloc,
                                    "INTF.E.GT",
                                    [format!("INT_INTF_RIGHT_TERM_HDIO_FT_X{x}Y{y}")],
                                );
                            }
                            ColumnKindRight::Hard(_, _)
                            | ColumnKindRight::DfeB
                            | ColumnKindRight::DfeC
                            | ColumnKindRight::DfeDF
                            | ColumnKindRight::DfeE => {
                                let kind = if grid.kind == GridKind::Ultrascale {
                                    "INT_INTERFACE_PCIE_R"
                                } else {
                                    "INT_INTF_R_PCIE4"
                                };
                                ngrid.name_node(nloc, "INTF.E.PCIE", [format!("{kind}_X{x}Y{y}")]);
                            }
                            _ => unreachable!(),
                        },
                        "INTF.W.IO" => match grid.columns[col].l {
                            ColumnKindLeft::Io(_) | ColumnKindLeft::Gt(_) => {
                                let kind = if col.to_idx() == 0 {
                                    "INT_INTF_LEFT_TERM_IO_FT"
                                } else if matches!(row.to_idx() % 15, 0 | 1 | 13 | 14) {
                                    "INT_INTF_L_CMT"
                                } else {
                                    "INT_INTF_L_IO"
                                };
                                ngrid.name_node(nloc, "INTF.W.IO", [format!("{kind}_X{x}Y{y}")]);
                            }
                            _ => {
                                ngrid.name_node(
                                    nloc,
                                    "INTF.PSS",
                                    [format!("INT_INTF_LEFT_TERM_PSS_X{x}Y{y}")],
                                );
                            }
                        },
                        "INTF.E.IO" => {
                            ngrid.name_node(
                                nloc,
                                "INTF.E.IO",
                                [format!("INT_INTF_RIGHT_TERM_IO_X{x}Y{y}")],
                            );
                        }
                        "RCLK_INT" => {
                            let lr = if col < grid.col_cfg() { 'L' } else { 'R' };
                            let name = format!("RCLK_INT_{lr}_X{x}Y{yy}", yy = y - 1);
                            let nnode = ngrid.name_node(nloc, "RCLK_INT", [name]);
                            let rx = rclk_int_grid.xlut[col];
                            let ry = rclk_int_grid.ylut[die.die][row];
                            match grid.kind {
                                GridKind::Ultrascale => {
                                    nnode.add_bel(
                                        0,
                                        format!("BUFCE_LEAF_X16_X{rx}Y{y}", y = ry * 2),
                                    );
                                    nnode.add_bel(
                                        1,
                                        format!("BUFCE_LEAF_X16_X{rx}Y{y}", y = ry * 2 + 1),
                                    );
                                }
                                GridKind::UltrascalePlus => {
                                    for i in 0..16 {
                                        nnode.add_bel(
                                            i,
                                            format!(
                                                "BUFCE_LEAF_X{x}Y{y}",
                                                x = rx * 8 + (i & 7),
                                                y = ry * 4 + i / 8
                                            ),
                                        );
                                        nnode.add_bel(
                                            i + 16,
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

                        "CLEL_L" => {
                            let nnode =
                                ngrid.name_node(nloc, "CLEL_L", [format!("CLEL_L_X{x}Y{y}")]);
                            if !(row.to_idx() % 60 == 59
                                && edev
                                    .disabled
                                    .contains(&DisabledPart::TopRow(die.die, grid.row_to_reg(row))))
                            {
                                let sx = cle_grid.xlut_l[col];
                                let sy = cle_grid.ylut[die.die][row];
                                nnode.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                            }
                        }
                        "CLEM" => {
                            let tk = match (grid.kind, col < grid.col_cfg()) {
                                (GridKind::Ultrascale, true) => "CLE_M",
                                (GridKind::Ultrascale, false) => "CLE_M_R",
                                (GridKind::UltrascalePlus, true) => "CLEM",
                                (GridKind::UltrascalePlus, false) => "CLEM_R",
                            };
                            let nnode = ngrid.name_node(nloc, "CLEM", [format!("{tk}_X{x}Y{y}")]);
                            if !(row.to_idx() % 60 == 59
                                && edev
                                    .disabled
                                    .contains(&DisabledPart::TopRow(die.die, grid.row_to_reg(row))))
                            {
                                let sx = cle_grid.xlut_l[col];
                                let sy = cle_grid.ylut[die.die][row];
                                nnode.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                            }
                        }
                        "CLEL_R" => {
                            let nnode =
                                ngrid.name_node(nloc, "CLEL_R", [format!("CLEL_R_X{x}Y{y}")]);
                            if !(row.to_idx() % 60 == 59
                                && edev
                                    .disabled
                                    .contains(&DisabledPart::TopRow(die.die, grid.row_to_reg(row))))
                            {
                                let sx = cle_grid.xlut_r[col];
                                let sy = cle_grid.ylut[die.die][row];
                                nnode.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                            }
                        }
                        "LAGUNA" => {
                            let (x, tk) = match grid.kind {
                                GridKind::Ultrascale => (x, "LAGUNA_TILE"),
                                GridKind::UltrascalePlus => (x - 1, "LAG_LAG"),
                            };
                            let nnode = ngrid.name_node(nloc, "LAGUNA", [format!("{tk}_X{x}Y{y}")]);
                            let lx0 = laguna_grid.xlut[col] * 2;
                            let lx1 = lx0 + 1;
                            let ly0 = laguna_grid.ylut[die.die][row] * 2;
                            let ly1 = ly0 + 1;
                            nnode.add_bel(0, format!("LAGUNA_X{lx0}Y{ly0}"));
                            nnode.add_bel(1, format!("LAGUNA_X{lx0}Y{ly1}"));
                            nnode.add_bel(2, format!("LAGUNA_X{lx1}Y{ly0}"));
                            nnode.add_bel(3, format!("LAGUNA_X{lx1}Y{ly1}"));
                        }
                        "BRAM" => {
                            let nnode = ngrid.name_node(nloc, "BRAM", [format!("BRAM_X{x}Y{y}")]);
                            let bx = bram_grid.xlut[col];
                            let by = bram_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("RAMB36_X{bx}Y{by}"));
                            nnode.add_bel(1, format!("RAMB18_X{bx}Y{y}", y = by * 2));
                            nnode.add_bel(2, format!("RAMB18_X{bx}Y{y}", y = by * 2 + 1));
                        }
                        "HARD_SYNC" => {
                            let tk = get_bram_tk(edev, has_laguna, die.die, col, row);
                            let nnode = ngrid.name_node(
                                nloc,
                                "HARD_SYNC",
                                [format!("{tk}_X{x}Y{y}", y = y - 1)],
                            );
                            let hx0 = hard_sync_grid.xlut[col] * 2;
                            let hx1 = hx0 + 1;
                            let hy0 = hard_sync_grid.ylut[die.die][row] * 2;
                            let hy1 = hy0 + 1;
                            nnode.add_bel(0, format!("HARD_SYNC_X{hx0}Y{hy0}"));
                            nnode.add_bel(1, format!("HARD_SYNC_X{hx0}Y{hy1}"));
                            nnode.add_bel(2, format!("HARD_SYNC_X{hx1}Y{hy0}"));
                            nnode.add_bel(3, format!("HARD_SYNC_X{hx1}Y{hy1}"));
                        }
                        "DSP" => {
                            let nnode = ngrid.name_node(nloc, "DSP", [format!("DSP_X{x}Y{y}")]);
                            let dx = dsp_grid.xlut[col];
                            let dy0 = dsp_grid.ylut[die.die][row] * 2;
                            let dy1 = dy0 + 1;
                            nnode.add_bel(0, format!("DSP48E2_X{dx}Y{dy0}"));
                            if !(row.to_idx() % 60 == 55
                                && edev
                                    .disabled
                                    .contains(&DisabledPart::TopRow(die.die, grid.row_to_reg(row))))
                            {
                                nnode.add_bel(1, format!("DSP48E2_X{dx}Y{dy1}"));
                            }
                        }
                        "URAM" => {
                            let tk = if row.to_idx() % 60 == 45 {
                                "URAM_URAM_DELAY_FT"
                            } else {
                                "URAM_URAM_FT"
                            };
                            let nnode = ngrid.name_node(nloc, "URAM", [format!("{tk}_X{x}Y{y}")]);
                            let ux = uram_grid.xlut[col];
                            let uy0 = uram_grid.ylut[die.die][row] * 4;
                            let uy1 = uy0 + 1;
                            let uy2 = uy0 + 2;
                            let uy3 = uy0 + 3;
                            nnode.add_bel(0, format!("URAM288_X{ux}Y{uy0}"));
                            nnode.add_bel(1, format!("URAM288_X{ux}Y{uy1}"));
                            nnode.add_bel(2, format!("URAM288_X{ux}Y{uy2}"));
                            nnode.add_bel(3, format!("URAM288_X{ux}Y{uy3}"));
                        }
                        "CFG" | "CFG_CSEC" => {
                            let ColumnKindLeft::Hard(_, idx) = grid.columns[col].l else {
                                unreachable!()
                            };
                            let x = if grid.kind == GridKind::UltrascalePlus && !hdio_cfg_only[idx]
                            {
                                x
                            } else {
                                x - 1
                            };
                            let tk = if grid.kind == GridKind::Ultrascale {
                                "CFG_CFG"
                            } else if !grid.has_csec {
                                "CFG_CONFIG"
                            } else {
                                "CSEC_CONFIG_FT"
                            };
                            let name = format!("{tk}_X{x}Y{y}");
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = cfg_grid.xlut[col];
                            let sy = cfg_grid.ylut[die.die][row];
                            if grid.has_csec {
                                nnode.add_bel(0, format!("CSEC_SITE_X{sx}Y{sy}"));
                            } else {
                                nnode.add_bel(0, format!("CONFIG_SITE_X{sx}Y{sy}"));
                            }
                            let asx = aswitch_grid.xlut[col - 1].cfg;
                            let asy = aswitch_grid.ylut[die.die][reg].cfg;
                            nnode.add_bel(1, format!("ABUS_SWITCH_X{asx}Y{asy}"));
                        }
                        "CFGIO" => {
                            let ColumnKindLeft::Hard(_, idx) = grid.columns[col].l else {
                                unreachable!()
                            };
                            let x = if grid.kind == GridKind::UltrascalePlus
                                && (!hdio_cfg_only[idx] || grid.has_csec)
                            {
                                x
                            } else {
                                x - 1
                            };
                            let tk = if grid.kind == GridKind::Ultrascale {
                                "CFGIO_IOB"
                            } else if !grid.has_csec {
                                "CFGIO_IOB20"
                            } else {
                                "CFGIOLC_IOB20_FT"
                            };
                            let name = format!("{tk}_X{x}Y{y}");
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = cfgio_grid.xlut[col];
                            let sy = cfgio_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PMV_X{sx}Y{sy}"));
                            nnode.add_bel(1, format!("PMV2_X{sx}Y{sy}"));
                            nnode.add_bel(2, format!("PMVIOB_X{sx}Y{sy}"));
                            nnode.add_bel(3, format!("MTBF3_X{sx}Y{sy}"));
                            if grid.kind == GridKind::UltrascalePlus {
                                nnode.add_bel(4, format!("CFGIO_SITE_X{sx}Y{sy}"));
                            }
                        }
                        "AMS" => {
                            let ColumnKindLeft::Hard(_, idx) = grid.columns[col].l else {
                                unreachable!()
                            };
                            let x = if grid.kind == GridKind::UltrascalePlus
                                && (!hdio_cfg_only[idx] || grid.has_csec)
                            {
                                x
                            } else {
                                x - 1
                            };
                            let name = format!("AMS_X{x}Y{y}");
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = ams_grid.xlut[col];
                            let sy = ams_grid.ylut[die.die][row];
                            let bk = if grid.kind == GridKind::Ultrascale {
                                "SYSMONE1"
                            } else {
                                "SYSMONE4"
                            };
                            nnode.add_bel(0, format!("{bk}_X{sx}Y{sy}"));
                        }
                        "PCIE" => {
                            let name = format!("PCIE_X{x}Y{y}", x = x - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = pcie_grid.xlut[col];
                            let sy = pcie_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PCIE_3_1_X{sx}Y{sy}"));
                        }
                        "PCIE4" => {
                            let name = format!("PCIE4_PCIE4_FT_X{x}Y{y}", x = x - 1);
                            let naming = if has_mcap { "PCIE4" } else { "PCIE4.NOCFG" };
                            let nnode = ngrid.name_node(nloc, naming, [name]);
                            let sx = pcie4_grid.xlut[col];
                            let sy = pcie4_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PCIE40E4_X{sx}Y{sy}"));
                        }
                        "PCIE4C" => {
                            let name = format!("PCIE4C_PCIE4C_FT_X{x}Y{y}", x = x - 1);
                            let naming = if has_mcap { "PCIE4C" } else { "PCIE4C.NOCFG" };
                            let nnode = ngrid.name_node(nloc, naming, [name]);
                            let sx = pcie4c_grid.xlut[col];
                            let sy = pcie4c_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PCIE4CE4_X{sx}Y{sy}"));
                        }
                        "CMAC" => {
                            let name = if grid.kind == GridKind::Ultrascale {
                                let x = if col == grid.col_cfg() { x - 1 } else { x };
                                format!("CMAC_CMAC_FT_X{x}Y{y}")
                            } else {
                                format!("CMAC_X{x}Y{y}", x = x - 1)
                            };
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = cmac_grid.xlut[col];
                            let sy = cmac_grid.ylut[die.die][row];
                            if grid.kind == GridKind::Ultrascale {
                                nnode.add_bel(0, format!("CMAC_SITE_X{sx}Y{sy}"));
                            } else {
                                nnode.add_bel(0, format!("CMACE4_X{sx}Y{sy}"));
                            }
                        }
                        "ILKN" => {
                            let name = if grid.kind == GridKind::Ultrascale {
                                format!("ILMAC_ILMAC_FT_X{x}Y{y}", x = x - 1)
                            } else {
                                format!("ILKN_ILKN_FT_X{x}Y{y}", x = x - 1)
                            };
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = ilkn_grid.xlut[col];
                            let sy = ilkn_grid.ylut[die.die][row];
                            if grid.kind == GridKind::Ultrascale {
                                nnode.add_bel(0, format!("ILKN_SITE_X{sx}Y{sy}"));
                            } else {
                                nnode.add_bel(0, format!("ILKNE4_X{sx}Y{sy}"));
                            }
                        }
                        "FE" => {
                            let name = format!("FE_FE_FT_X{x}Y{y}", x = x - 1);
                            let nnode = ngrid.name_node(nloc, "FE", [name]);
                            let sx = fe_grid.xlut[col];
                            let sy = fe_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("FE_X{sx}Y{sy}"));
                        }
                        "DFE_A" => {
                            let name = format!("DFE_DFE_TILEA_FT_X{x}Y{y}", x = x - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = dfe_a_grid.xlut[col];
                            let sy = dfe_a_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("DFE_A_X{sx}Y{sy}"));
                        }
                        "DFE_B" => {
                            let name = format!("DFE_DFE_TILEB_FT_X{x}Y{y}");
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = dfe_b_grid.xlut[col];
                            let sy = dfe_b_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("DFE_B_X{sx}Y{sy}"));
                        }
                        "DFE_C" => {
                            let name = format!("DFE_DFE_TILEC_FT_X{x}Y{y}", x = x - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = dfe_c_grid.xlut[col];
                            let sy = dfe_c_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("DFE_C_X{sx}Y{sy}"));
                        }
                        "DFE_D" => {
                            let name = format!("DFE_DFE_TILED_FT_X{x}Y{y}", x = x - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = dfe_d_grid.xlut[col];
                            let sy = dfe_d_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("DFE_D_X{sx}Y{sy}"));
                        }
                        "DFE_E" => {
                            let name = format!("DFE_DFE_TILEE_FT_X{x}Y{y}", x = x - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = dfe_e_grid.xlut[col];
                            let sy = dfe_e_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("DFE_E_X{sx}Y{sy}"));
                        }
                        "DFE_F" => {
                            let name = format!("DFE_DFE_TILEF_FT_X{x}Y{y}", x = x - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = dfe_f_grid.xlut[col];
                            let sy = dfe_f_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("DFE_F_X{sx}Y{sy}"));
                        }
                        "DFE_G" => {
                            let name = format!("DFE_DFE_TILEG_FT_X{x}Y{y}", x = x - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = dfe_g_grid.xlut[col];
                            let sy = dfe_g_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("DFE_G_X{sx}Y{sy}"));
                        }
                        "HDIO_BOT" | "HDIO_TOP" => {
                            let ColumnKindRight::Hard(_, idx) = grid.columns[col].r else {
                                unreachable!()
                            };
                            let x = if hdio_cfg_only[idx] { x } else { x + 1 };
                            let name = format!("{kind}_RIGHT_X{x}Y{y}");
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let iox = io_grid.hdio_xlut[col];
                            let ioy = match &kind[..] {
                                "HDIO_BOT" => io_grid.hdio_ylut[die.die][reg].0,
                                "HDIO_TOP" => io_grid.hdio_ylut[die.die][reg].1,
                                _ => unreachable!(),
                            };
                            let sx = hdio_grid.xlut[col];
                            let sy = hdio_grid.ylut[die.die][row];
                            for j in 0..12 {
                                nnode.add_bel(j, format!("IOB_X{iox}Y{y}", y = ioy + j));
                            }
                            for j in 0..6 {
                                nnode.add_bel(
                                    12 + j,
                                    format!("HDIOBDIFFINBUF_X{sx}Y{y}", y = sy * 6 + j),
                                );
                                nnode.add_bel(
                                    18 + 2 * j,
                                    format!("HDIOLOGIC_M_X{sx}Y{y}", y = sy * 6 + j),
                                );
                                nnode.add_bel(
                                    18 + 2 * j + 1,
                                    format!("HDIOLOGIC_S_X{sx}Y{y}", y = sy * 6 + j),
                                );
                            }
                            nnode.add_bel(30, format!("HDLOGIC_CSSD_X{sx}Y{sy}"));
                            if kind == "HDIO_BOT" {
                                nnode.add_bel(31, format!("HDIO_VREF_X{sx}Y{y}", y = sy / 2));
                            } else {
                                nnode.add_bel(31, format!("HDIO_BIAS_X{sx}Y{y}", y = sy / 2));
                            }
                        }
                        "HDIOLC_L_BOT" | "HDIOLC_L_TOP" | "HDIOLC_R_BOT" | "HDIOLC_R_TOP" => {
                            let naming = match &kind[..] {
                                "HDIOLC_L_BOT" => "HDIOLC_HDIOL_BOT_LEFT_FT",
                                "HDIOLC_L_TOP" => "HDIOLC_HDIOL_TOP_LEFT_FT",
                                "HDIOLC_R_BOT" => {
                                    if reg == grid.reg_cfg() {
                                        "HDIOLC_HDIOL_BOT_RIGHT_CFG_FT"
                                    } else {
                                        "HDIOLC_HDIOL_BOT_RIGHT_AUX_FT"
                                    }
                                }
                                "HDIOLC_R_TOP" => {
                                    if reg == grid.reg_cfg() {
                                        "HDIOLC_HDIOL_TOP_RIGHT_CFG_FT"
                                    } else {
                                        "HDIOLC_HDIOL_TOP_RIGHT_AUX_FT"
                                    }
                                }
                                _ => unreachable!(),
                            };
                            let name = format!("{naming}_X{x}Y{y}");
                            let nnode = ngrid.name_node(nloc, naming, [name]);
                            let iox = io_grid.hdio_xlut[col];
                            let ioy = match &kind[..] {
                                "HDIOLC_L_BOT" | "HDIOLC_R_BOT" => {
                                    io_grid.hdio_ylut[die.die][reg].0
                                }
                                "HDIOLC_L_TOP" | "HDIOLC_R_TOP" => {
                                    io_grid.hdio_ylut[die.die][reg].1
                                }
                                _ => unreachable!(),
                            };
                            let sx = hdiolc_grid.xlut[col];
                            let sy = hdiolc_grid.ylut[die.die][row];
                            for j in 0..42 {
                                nnode.add_bel(j, format!("IOB_X{iox}Y{y}", y = ioy + j));
                            }
                            for j in 0..21 {
                                nnode.add_bel(
                                    42 + j,
                                    format!("HDIOBDIFFINBUF_X{sx}Y{y}", y = sy * 21 + j),
                                );
                                nnode.add_bel(
                                    63 + 2 * j,
                                    format!("HDIOLOGIC_M_X{sx}Y{y}", y = sy * 21 + j),
                                );
                                nnode.add_bel(
                                    63 + 2 * j + 1,
                                    format!("HDIOLOGIC_S_X{sx}Y{y}", y = sy * 21 + j),
                                );
                            }
                            for j in 0..3 {
                                nnode.add_bel(
                                    105 + j,
                                    format!("HDLOGIC_CSSD_X{sx}Y{y}", y = sy * 3 + j),
                                );
                            }
                            for j in 0..2 {
                                nnode.add_bel(
                                    108 + j,
                                    format!("HDIO_VREF_X{sx}Y{y}", y = sy * 2 + j),
                                );
                            }
                            nnode.add_bel(110, format!("HDIO_BIAS_X{sx}Y{sy}"));
                        }
                        "RCLK_HDIO" => {
                            let ColumnKindRight::Hard(hk, idx) = grid.columns[col].r else {
                                unreachable!()
                            };
                            let x = if hdio_cfg_only[idx] { x } else { x + 1 };
                            let tkn = match hk {
                                HardKind::Clk => "RCLK_HDIO",
                                HardKind::NonClk => "RCLK_RCLK_HDIO_R_FT",
                                HardKind::Term => "RCLK_RCLK_HDIO_LAST_R_FT",
                            };
                            let name = format!("{tkn}_X{x}Y{y}", y = y - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = rclk_hdio_grid.xlut[col];
                            let sy = rclk_hdio_grid.ylut[die.die][row];
                            nnode.add_bel(
                                0,
                                format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2),
                            );
                            nnode.add_bel(
                                1,
                                format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2 + 1),
                            );
                            nnode.add_bel(
                                2,
                                format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2 + 1, y = sy * 2),
                            );
                            nnode.add_bel(
                                3,
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
                                    4 + i,
                                    format!(
                                        "ABUS_SWITCH_X{x}Y{y}",
                                        x = aswitch_grid.xlut[col].hdio + x,
                                        y = aswitch_grid.ylut[die.die][reg].hdio + y
                                    ),
                                );
                            }
                        }
                        "RCLK_HDIOLC_L" | "RCLK_HDIOLC_R" => {
                            let tkn = if kind == "RCLK_HDIOLC_L" {
                                "RCLK_RCLK_HDIOL_L_FT"
                            } else {
                                "RCLK_RCLK_HDIOL_R_FT"
                            };
                            let name = format!("{tkn}_X{x}Y{y}", y = y - 1);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = rclk_hdio_grid.xlut[col];
                            let sy = rclk_hdio_grid.ylut[die.die][row];
                            nnode.add_bel(
                                0,
                                format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2),
                            );
                            nnode.add_bel(
                                1,
                                format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2 + 1),
                            );
                            nnode.add_bel(
                                2,
                                format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2 + 1, y = sy * 2),
                            );
                            nnode.add_bel(
                                3,
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
                                    4 + i,
                                    format!(
                                        "ABUS_SWITCH_X{x}Y{y}",
                                        x = aswitch_grid.xlut[col].hdio + x,
                                        y = aswitch_grid.ylut[die.die][reg].hdio + y
                                    ),
                                );
                            }
                        }

                        "CMT_L" | "CMT_L_HBM" | "CMT_R" => {
                            let iocol = grid.cols_io.iter().find(|iocol| iocol.col == col).unwrap();
                            let tk = match &kind[..] {
                                "CMT_L" => {
                                    if iocol.regs[reg] == IoRowKind::HdioLc {
                                        "CMT_CMT_LEFT_DL3_FT"
                                    } else {
                                        "CMT_L"
                                    }
                                }
                                "CMT_L_HBM" => "CMT_LEFT_H",
                                "CMT_R" => "CMT_RIGHT",
                                _ => unreachable!(),
                            };
                            let name = format!("{tk}_X{x}Y{y}", y = y - 30);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let cmtx = cmt_grid.xlut[col];
                            let cmty = cmt_grid.ylut[die.die][row];
                            let gtbx = if kind != "CMT_R" {
                                clk_grid.gtbxlut[col].0
                            } else {
                                clk_grid.gtbxlut[col].1
                            };
                            for i in 0..24 {
                                nnode.add_bel(
                                    i,
                                    format!("BUFCE_ROW_X{cmtx}Y{y}", y = cmty * 24 + i),
                                );
                                nnode.add_bel(
                                    24 + i,
                                    format!(
                                        "GCLK_TEST_BUFE3_X{gtbx}Y{y}",
                                        y = clk_grid.gtbylut[die.die][reg].0
                                            + if i < 18 { i } else { i + 1 }
                                    ),
                                );
                                nnode.add_bel(
                                    48 + i,
                                    format!("BUFGCE_X{cmtx}Y{y}", y = cmty * 24 + i),
                                );
                            }
                            for i in 0..8 {
                                nnode.add_bel(
                                    72 + i,
                                    format!("BUFGCTRL_X{cmtx}Y{y}", y = cmty * 8 + i),
                                );
                            }
                            for i in 0..4 {
                                nnode.add_bel(
                                    80 + i,
                                    format!("BUFGCE_DIV_X{cmtx}Y{y}", y = cmty * 4 + i),
                                );
                            }
                            for i in 0..2 {
                                nnode.add_bel(84 + i, format!("PLL_X{cmtx}Y{y}", y = cmty * 2 + i));
                            }
                            nnode.add_bel(86, format!("MMCM_X{cmtx}Y{cmty}"));
                            let asx = if kind != "CMT_R" {
                                aswitch_grid.xlut[col].io + 7
                            } else {
                                aswitch_grid.xlut[col].io
                            };
                            nnode.add_bel(
                                87,
                                format!(
                                    "ABUS_SWITCH_X{asx}Y{y}",
                                    y = aswitch_grid.ylut[die.die][reg].cmt
                                ),
                            );
                            if kind == "CMT_L_HBM" {
                                nnode.add_bel(88, "HBM_REF_CLK_X0Y0".to_string());
                                nnode.add_bel(89, "HBM_REF_CLK_X0Y1".to_string());
                            }
                        }
                        "XIPHY" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                "XIPHY",
                                [format!("XIPHY_L_X{x}Y{y}", y = y - 30)],
                            );
                            let cmtx = cmt_grid.xlut[col];
                            let cmty = cmt_grid.ylut[die.die][row];
                            for i in 0..24 {
                                nnode.add_bel(
                                    i,
                                    format!(
                                        "BUFCE_ROW_X{x}Y{y}",
                                        x = clk_grid.brxlut[col].0,
                                        y = cmty * 25 + i
                                    ),
                                );
                                nnode.add_bel(
                                    24 + i,
                                    format!(
                                        "GCLK_TEST_BUFE3_X{x}Y{y}",
                                        x = clk_grid.gtbxlut[col].0,
                                        y = clk_grid.gtbylut[die.die][reg].0 + i
                                    ),
                                );
                                nnode.add_bel(
                                    48 + i,
                                    format!("BUFGCE_X{cmtx}Y{y}", y = cmty * 24 + i),
                                );
                            }
                            for i in 0..8 {
                                nnode.add_bel(
                                    72 + i,
                                    format!("BUFGCTRL_X{cmtx}Y{y}", y = cmty * 8 + i),
                                );
                            }
                            for i in 0..4 {
                                nnode.add_bel(
                                    80 + i,
                                    format!("BUFGCE_DIV_X{cmtx}Y{y}", y = cmty * 4 + i),
                                );
                            }
                            for i in 0..2 {
                                nnode.add_bel(
                                    84 + i,
                                    format!("PLLE3_ADV_X{cmtx}Y{y}", y = cmty * 2 + i),
                                );
                            }
                            nnode.add_bel(86, format!("MMCME3_ADV_X{cmtx}Y{cmty}"));
                            nnode.add_bel(
                                87,
                                format!(
                                    "ABUS_SWITCH_X{x}Y{y}",
                                    x = aswitch_grid.xlut[col].io,
                                    y = aswitch_grid.ylut[die.die][reg].cmt
                                ),
                            );
                            for i in 0..52 {
                                nnode.add_bel(
                                    88 + i,
                                    format!("BITSLICE_RX_TX_X{cmtx}Y{y}", y = cmty * 52 + i),
                                );
                            }
                            for i in 0..8 {
                                nnode.add_bel(
                                    140 + i,
                                    format!("BITSLICE_TX_X{cmtx}Y{y}", y = cmty * 8 + i),
                                );
                            }
                            for i in 0..8 {
                                nnode.add_bel(
                                    148 + i,
                                    format!("BITSLICE_CONTROL_X{cmtx}Y{y}", y = cmty * 8 + i),
                                );
                            }
                            for i in 0..8 {
                                nnode.add_bel(
                                    156 + i,
                                    format!("PLL_SELECT_SITE_X{cmtx}Y{y}", y = cmty * 8 + (i ^ 1)),
                                );
                            }
                            for i in 0..4 {
                                nnode.add_bel(
                                    164 + i,
                                    format!("RIU_OR_X{cmtx}Y{y}", y = cmty * 4 + i),
                                );
                            }
                            for i in 0..4 {
                                nnode.add_bel(
                                    168 + i,
                                    format!("XIPHY_FEEDTHROUGH_X{x}Y{cmty}", x = cmtx * 4 + i),
                                );
                            }
                        }
                        "XIPHY_L" | "XIPHY_R" => {
                            let tk = if kind == "XIPHY_L" {
                                "XIPHY_BYTE_L"
                            } else {
                                "XIPHY_BYTE_RIGHT"
                            };
                            let nnode = ngrid.name_node(nloc, kind, [format!("{tk}_X{x}Y{y}")]);
                            let phyx = xiphy_grid.xlut[col];
                            let phyy = xiphy_grid.ylut[die.die][row];
                            for i in 0..13 {
                                nnode.add_bel(
                                    i,
                                    format!("BITSLICE_RX_TX_X{phyx}Y{y}", y = phyy * 13 + i),
                                );
                            }
                            for i in 0..2 {
                                nnode.add_bel(
                                    13 + i,
                                    format!("BITSLICE_TX_X{phyx}Y{y}", y = phyy * 2 + i),
                                );
                            }
                            for i in 0..2 {
                                nnode.add_bel(
                                    15 + i,
                                    format!("BITSLICE_CONTROL_X{phyx}Y{y}", y = phyy * 2 + i),
                                );
                            }
                            for i in 0..2 {
                                nnode.add_bel(
                                    17 + i,
                                    format!("PLL_SELECT_SITE_X{phyx}Y{y}", y = phyy * 2 + i),
                                );
                            }
                            nnode.add_bel(19, format!("RIU_OR_X{phyx}Y{phyy}"));
                            nnode.add_bel(20, format!("XIPHY_FEEDTHROUGH_X{phyx}Y{phyy}"));
                        }
                        "RCLK_XIPHY_L" | "RCLK_XIPHY_R" => {
                            let tk = match &kind[..] {
                                "RCLK_XIPHY_L" => "RCLK_RCLK_XIPHY_INNER_FT",
                                "RCLK_XIPHY_R" => "RCLK_XIPHY_OUTER_RIGHT",
                                _ => unreachable!(),
                            };
                            let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                            ngrid.name_node(nloc, kind, [name]);
                        }

                        "HPIO" => {
                            let name =
                                format!("HPIO_L_X{x}Y{y}", x = if x > 0 { x - 1 } else { x });
                            let naming = if io_grid.is_cfg_io_hrio {
                                "HPIO.NOCFG"
                            } else {
                                "HPIO"
                            };
                            let nnode = ngrid.name_node(nloc, naming, [name]);
                            let iox = io_grid.hpio_xlut[col];
                            let (ioy_b, ioy_t) = match row.to_idx() % 60 {
                                0 => (
                                    io_grid.hpio_ylut[die.die][reg].0,
                                    io_grid.hpio_ylut[die.die][reg].1,
                                ),
                                30 => (
                                    io_grid.hpio_ylut[die.die][reg].2,
                                    io_grid.hpio_ylut[die.die][reg].3,
                                ),
                                _ => unreachable!(),
                            };
                            let sx = hpio_grid.xlut[col];
                            let sy = hpio_grid.ylut[die.die][row];
                            for j in 0..13 {
                                nnode.add_bel(j, format!("IOB_X{iox}Y{y}", y = ioy_b + j));
                            }
                            for j in 0..13 {
                                nnode.add_bel(13 + j, format!("IOB_X{iox}Y{y}", y = ioy_t + j));
                            }
                            for j in 0..12 {
                                nnode.add_bel(
                                    26 + j,
                                    format!("HPIOBDIFFINBUF_X{sx}Y{y}", y = sy * 12 + j),
                                );
                            }
                            for j in 0..12 {
                                nnode.add_bel(
                                    38 + j,
                                    format!("HPIOBDIFFOUTBUF_X{sx}Y{y}", y = sy * 12 + j),
                                );
                            }
                            nnode.add_bel(50, format!("HPIO_VREF_SITE_X{sx}Y{sy}"));
                        }
                        "RCLK_HPIO" => {
                            let name = format!(
                                "RCLK_HPIO_L_X{x}Y{y}",
                                x = if x > 0 { x - 1 } else { x },
                                y = y - 1
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = rclk_hpio_grid.xlut[col];
                            let sy = rclk_hpio_grid.ylut[die.die][row];
                            for i in 0..5 {
                                nnode.add_bel(
                                    i,
                                    format!(
                                        "ABUS_SWITCH_X{x}Y{y}",
                                        x = aswitch_grid.xlut[col].io + i,
                                        y = aswitch_grid.ylut[die.die][reg].hpio
                                    ),
                                );
                            }
                            nnode.add_bel(5, format!("HPIO_ZMATCH_BLK_HCLK_X{sx}Y{sy}"));
                        }
                        "HRIO" => {
                            let name =
                                format!("HRIO_L_X{x}Y{y}", x = if x > 0 { x - 1 } else { x });
                            let naming = if io_grid.is_cfg_io_hrio {
                                "HRIO"
                            } else {
                                "HRIO.NOCFG"
                            };
                            let nnode = ngrid.name_node(nloc, naming, [name]);
                            let iox = io_grid.hpio_xlut[col];
                            let (ioy_b, ioy_t) = match row.to_idx() % 60 {
                                0 => (
                                    io_grid.hpio_ylut[die.die][reg].0,
                                    io_grid.hpio_ylut[die.die][reg].1,
                                ),
                                30 => (
                                    io_grid.hpio_ylut[die.die][reg].2,
                                    io_grid.hpio_ylut[die.die][reg].3,
                                ),
                                _ => unreachable!(),
                            };
                            let sx = hrio_grid.xlut[col];
                            let sy = hrio_grid.ylut[die.die][row];
                            for j in 0..13 {
                                nnode.add_bel(j, format!("IOB_X{iox}Y{y}", y = ioy_b + j));
                            }
                            for j in 0..13 {
                                nnode.add_bel(13 + j, format!("IOB_X{iox}Y{y}", y = ioy_t + j));
                            }
                            for j in 0..12 {
                                nnode.add_bel(
                                    26 + j,
                                    format!("HRIODIFFINBUF_X{sx}Y{y}", y = sy * 12 + j),
                                );
                            }
                            for j in 0..12 {
                                nnode.add_bel(
                                    38 + j,
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
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            for i in 0..8 {
                                nnode.add_bel(
                                    i,
                                    format!(
                                        "ABUS_SWITCH_X{x}Y{y}",
                                        x = aswitch_grid.xlut[col].io + i,
                                        y = aswitch_grid.ylut[die.die][reg].hrio
                                    ),
                                );
                            }
                        }
                        "HPIO_L" | "HPIO_R" => {
                            let (naming, naming_alt, naming_nocfg, naming_noams, tk) =
                                if kind == "HPIO_R" {
                                    (
                                        "HPIO_R",
                                        "HPIO_R.ALTCFG",
                                        "HPIO_R.NOCFG",
                                        "HPIO_R.NOAMS",
                                        "HPIO_RIGHT",
                                    )
                                } else {
                                    (
                                        "HPIO_L",
                                        "HPIO_L.ALTCFG",
                                        "HPIO_L.NOCFG",
                                        "HPIO_L.NOAMS",
                                        "HPIO_L",
                                    )
                                };
                            let naming = if grid.has_csec {
                                naming_noams
                            } else if grid.is_nocfg() {
                                naming_nocfg
                            } else if grid.ps.is_some()
                                && (grid.is_alt_cfg || !edev.disabled.contains(&DisabledPart::Ps))
                            {
                                naming_alt
                            } else {
                                naming
                            };
                            let name = format!(
                                "{tk}_X{x}Y{y}",
                                x = if x > 0 && kind == "HPIO_L" { x - 1 } else { x }
                            );
                            let nnode = ngrid.name_node(nloc, naming, [name]);
                            let iox = io_grid.hpio_xlut[col];
                            let (ioy_b, ioy_t) = match row.to_idx() % 60 {
                                0 => (
                                    io_grid.hpio_ylut[die.die][reg].0,
                                    io_grid.hpio_ylut[die.die][reg].1,
                                ),
                                30 => (
                                    io_grid.hpio_ylut[die.die][reg].2,
                                    io_grid.hpio_ylut[die.die][reg].3,
                                ),
                                _ => unreachable!(),
                            };
                            let sx = hpio_grid.xlut[col];
                            let sy = hpio_grid.ylut[die.die][row];
                            for j in 0..13 {
                                nnode.add_bel(j, format!("IOB_X{iox}Y{y}", y = ioy_b + j));
                            }
                            for j in 0..13 {
                                nnode.add_bel(13 + j, format!("IOB_X{iox}Y{y}", y = ioy_t + j));
                            }
                            for j in 0..12 {
                                nnode.add_bel(
                                    26 + j,
                                    format!("HPIOBDIFFINBUF_X{sx}Y{y}", y = sy * 12 + j),
                                );
                            }
                            for j in 0..12 {
                                nnode.add_bel(
                                    38 + j,
                                    format!("HPIOBDIFFOUTBUF_X{sx}Y{y}", y = sy * 12 + j),
                                );
                            }
                            for j in 0..2 {
                                nnode.add_bel(
                                    50 + j,
                                    format!("HPIOB_DCI_SNGL_X{sx}Y{y}", y = sy * 2 + j),
                                );
                            }
                            nnode.add_bel(52, format!("HPIO_VREF_SITE_X{sx}Y{sy}"));
                            nnode.add_bel(53, format!("BIAS_X{sx}Y{sy}"));
                        }
                        "RCLK_HPIO_L" | "RCLK_HPIO_R" => {
                            let name = format!(
                                "{kind}_X{x}Y{y}",
                                x = if x > 0 && kind == "RCLK_HPIO_L" {
                                    x - 1
                                } else {
                                    x
                                },
                                y = y - 1
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let sx = rclk_hpio_grid.xlut[col];
                            let sy = rclk_hpio_grid.ylut[die.die][row];
                            let asx = if kind == "RCLK_HPIO_L" {
                                aswitch_grid.xlut[col].io
                            } else {
                                aswitch_grid.xlut[col].io + 1
                            };
                            for i in 0..7 {
                                nnode.add_bel(
                                    i,
                                    format!(
                                        "ABUS_SWITCH_X{x}Y{y}",
                                        x = asx + i,
                                        y = aswitch_grid.ylut[die.die][reg].hpio
                                    ),
                                );
                            }
                            nnode.add_bel(7, format!("HPIO_ZMATCH_BLK_HCLK_X{sx}Y{sy}"));
                            nnode.add_bel(8, format!("HPIO_RCLK_PRBS_X{sx}Y{sy}"));
                        }

                        "GTH_L" | "GTH_R" | "GTY_L" | "GTY_R" | "GTF_L" | "GTF_R" | "GTM_L"
                        | "GTM_R" | "HSADC_R" | "HSDAC_R" | "RFADC_R" | "RFDAC_R" => {
                            let (tk, gtk, gtk_grid) = match (grid.kind, &kind[..]) {
                                (GridKind::Ultrascale, "GTH_L") => {
                                    ("GTH_QUAD_LEFT_FT", "GTH", &gth_grid)
                                }
                                (GridKind::Ultrascale, "GTY_L") => {
                                    ("GTY_QUAD_LEFT_FT", "GTY", &gty_grid)
                                }
                                (GridKind::Ultrascale, "GTH_R") => ("GTH_R", "GTH", &gth_grid),
                                (GridKind::UltrascalePlus, "GTH_L") => {
                                    ("GTH_QUAD_LEFT", "GTH", &gth_grid)
                                }
                                (GridKind::UltrascalePlus, "GTH_R") => {
                                    ("GTH_QUAD_RIGHT", "GTH", &gth_grid)
                                }
                                (GridKind::UltrascalePlus, "GTY_L") => ("GTY_L", "GTY", &gty_grid),
                                (GridKind::UltrascalePlus, "GTY_R") => ("GTY_R", "GTY", &gty_grid),
                                (GridKind::UltrascalePlus, "GTF_L") => {
                                    ("GTFY_QUAD_LEFT_FT", "GTF", &gtf_grid)
                                }
                                (GridKind::UltrascalePlus, "GTF_R") => {
                                    ("GTFY_QUAD_RIGHT_FT", "GTF", &gtf_grid)
                                }
                                (GridKind::UltrascalePlus, "GTM_L") => {
                                    ("GTM_DUAL_LEFT_FT", "GTM", &gtm_grid)
                                }
                                (GridKind::UltrascalePlus, "GTM_R") => {
                                    ("GTM_DUAL_RIGHT_FT", "GTM", &gtm_grid)
                                }
                                (GridKind::UltrascalePlus, "HSADC_R") => {
                                    ("HSADC_HSADC_RIGHT_FT", "HSADC", &hsadc_grid)
                                }
                                (GridKind::UltrascalePlus, "HSDAC_R") => {
                                    ("HSDAC_HSDAC_RIGHT_FT", "HSDAC", &hsdac_grid)
                                }
                                (GridKind::UltrascalePlus, "RFADC_R") => {
                                    ("RFADC_RFADC_RIGHT_FT", "RFADC", &rfadc_grid)
                                }
                                (GridKind::UltrascalePlus, "RFDAC_R") => {
                                    ("RFDAC_RFDAC_RIGHT_FT", "RFDAC", &rfdac_grid)
                                }
                                _ => unreachable!(),
                            };
                            let name = format!("{tk}_X{x}Y{y}", y = y - 30);
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let gtx = gt_grid.xlut[col];
                            let gty = gt_grid.ylut[die.die][row];
                            for i in 0..24 {
                                nnode.add_bel(i, format!("BUFG_GT_X{gtx}Y{y}", y = gty * 24 + i));
                            }
                            if grid.kind == GridKind::Ultrascale {
                                for i in 0..11 {
                                    nnode.add_bel(
                                        24 + i,
                                        format!("BUFG_GT_SYNC_X{gtx}Y{y}", y = gty * 11 + i),
                                    );
                                }
                                for i in 0..4 {
                                    nnode.add_bel(
                                        35 + i,
                                        format!(
                                            "ABUS_SWITCH_X{x}Y{y}",
                                            x = aswitch_grid.xlut[col].gt,
                                            y = aswitch_grid.ylut[die.die][reg].gt + i
                                        ),
                                    );
                                }
                                let gtkx = gtk_grid.xlut[col];
                                let gtky = gtk_grid.ylut[die.die][row];
                                for i in 0..4 {
                                    nnode.add_bel(
                                        39 + i,
                                        format!("{gtk}E3_CHANNEL_X{gtkx}Y{y}", y = gtky * 4 + i),
                                    );
                                }
                                nnode.add_bel(43, format!("{gtk}E3_COMMON_X{gtkx}Y{gtky}"));
                            } else {
                                for i in 0..15 {
                                    nnode.add_bel(
                                        24 + i,
                                        format!("BUFG_GT_SYNC_X{gtx}Y{y}", y = gty * 15 + i),
                                    );
                                }
                                for i in 0..5 {
                                    nnode.add_bel(
                                        39 + i,
                                        format!(
                                            "ABUS_SWITCH_X{x}Y{y}",
                                            x = aswitch_grid.xlut[col].gt,
                                            y = aswitch_grid.ylut[die.die][reg].gt + i
                                        ),
                                    );
                                }
                                let gtkx = gtk_grid.xlut[col];
                                let gtky = gtk_grid.ylut[die.die][row];
                                if gtk == "GTM" {
                                    nnode.add_bel(44, format!("GTM_DUAL_X{gtkx}Y{gtky}"));
                                    nnode.add_bel(45, format!("GTM_REFCLK_X{gtkx}Y{gtky}"));
                                } else if gtk.starts_with("GT") {
                                    let pref = if gtk == "GTF" {
                                        "GTF".to_string()
                                    } else {
                                        format!("{gtk}E4")
                                    };

                                    for i in 0..4 {
                                        nnode.add_bel(
                                            44 + i,
                                            format!("{pref}_CHANNEL_X{gtkx}Y{y}", y = gtky * 4 + i),
                                        );
                                    }
                                    nnode.add_bel(48, format!("{pref}_COMMON_X{gtkx}Y{gtky}"));
                                } else {
                                    nnode.add_bel(44, format!("{gtk}_X{gtkx}Y{gtky}"));
                                }
                            }
                        }

                        "PS" => {
                            let nnode = ngrid.name_node(nloc, "PS", [format!("PSS_ALTO_X0Y{y}")]);
                            nnode.add_bel(0, "PS8_X0Y0".to_string());
                        }
                        "VCU" => {
                            let nnode =
                                ngrid.name_node(nloc, "VCU", [format!("VCU_VCU_FT_X0Y{y}")]);
                            nnode.add_bel(0, "VCU_X0Y0".to_string());
                        }
                        "RCLK_PS" => {
                            let tk = match grid.ps.as_ref().unwrap().intf_kind {
                                PsIntfKind::Alto => "RCLK_INTF_LEFT_TERM_ALTO",
                                PsIntfKind::Da6 => "RCLK_RCLK_INTF_LEFT_TERM_DA6_FT",
                                PsIntfKind::Da7 => "RCLK_INTF_LEFT_TERM_DA7",
                                PsIntfKind::Da8 => "RCLK_RCLK_INTF_LEFT_TERM_DA8_FT",
                                PsIntfKind::Dc12 => "RCLK_RCLK_INTF_LEFT_TERM_DC12_FT",
                                PsIntfKind::Mx8 => "RCLK_RCLK_INTF_LEFT_TERM_MX8_FT",
                            };
                            let nnode = ngrid.name_node(
                                nloc,
                                "RCLK_PS",
                                [format!("{tk}_X{x}Y{y}", y = y - 1)],
                            );
                            let by = rclk_ps_grid.ylut[die.die][row];
                            for i in 0..24 {
                                nnode.add_bel(i, format!("BUFG_PS_X0Y{y}", y = by * 24 + i));
                            }
                        }
                        "BLI" => {
                            let nnode =
                                ngrid.name_node(nloc, "BLI", [format!("BLI_BLI_FT_X{x}Y{y}")]);
                            let dx = dsp_grid.xlut[col];
                            nnode.add_bel(0, format!("BLI_HBM_APB_INTF_X{dx}Y0"));
                            nnode.add_bel(1, format!("BLI_HBM_AXI_INTF_X{dx}Y0"));
                        }

                        "RCLK_V_SINGLE_L.CLE" => {
                            let is_l = col < grid.col_cfg();
                            let tk = match (grid.kind, grid.columns[col].l, is_l) {
                                (GridKind::Ultrascale, ColumnKindLeft::CleL, true) => "RCLK_CLEL_L",
                                (GridKind::Ultrascale, ColumnKindLeft::CleL, false) => {
                                    "RCLK_CLEL_R"
                                }
                                (GridKind::Ultrascale, ColumnKindLeft::CleM(_), true) => {
                                    "RCLK_CLE_M_L"
                                }
                                (GridKind::Ultrascale, ColumnKindLeft::CleM(_), false) => {
                                    "RCLK_CLE_M_R"
                                }
                                (GridKind::UltrascalePlus, ColumnKindLeft::CleL, true) => {
                                    "RCLK_CLEL_L_L"
                                }
                                (GridKind::UltrascalePlus, ColumnKindLeft::CleL, false) => {
                                    "RCLK_CLEL_L_R"
                                }
                                (GridKind::UltrascalePlus, ColumnKindLeft::CleM(subkind), true) => {
                                    if grid.is_dmc && subkind == CleMKind::Laguna {
                                        "RCLK_CLEM_DMC_L"
                                    } else {
                                        "RCLK_CLEM_L"
                                    }
                                }
                                (GridKind::UltrascalePlus, ColumnKindLeft::CleM(_), false) => {
                                    "RCLK_CLEM_R"
                                }
                                _ => unreachable!(),
                            };
                            let is_alt = dev_naming.rclk_alt_pins[tk];
                            let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                            let nnode = ngrid.name_node(
                                nloc,
                                if is_alt {
                                    "RCLK_V_SINGLE_L.ALT"
                                } else {
                                    "RCLK_V_SINGLE_L"
                                },
                                [name],
                            );
                            let reg = grid.row_to_reg(row);
                            let mut brx = clk_grid.brxlut[col].0;
                            let bry = clk_grid.brylut[die.die][reg];
                            let mut gtbx = clk_grid.gtbxlut[col].0;
                            let gtby = clk_grid.gtbylut[die.die][reg].1;
                            if grid.kind == GridKind::UltrascalePlus
                                && grid.columns[col].l == ColumnKindLeft::CleM(CleMKind::Laguna)
                            {
                                brx += 1;
                                gtbx += 1;
                            }
                            match grid.kind {
                                GridKind::Ultrascale => nnode
                                    .add_bel(0, format!("BUFCE_ROW_X{brx}Y{y}", y = bry * 25 + 24)),
                                GridKind::UltrascalePlus => {
                                    nnode.add_bel(0, format!("BUFCE_ROW_FSR_X{brx}Y{bry}"))
                                }
                            }
                            nnode.add_bel(1, format!("GCLK_TEST_BUFE3_X{gtbx}Y{gtby}"));
                        }
                        "RCLK_V_SINGLE_R.CLE" => {
                            let is_l = col < grid.col_cfg();
                            let tk = if is_l {
                                "RCLK_CLEL_R_L"
                            } else {
                                "RCLK_CLEL_R_R"
                            };
                            let is_alt = dev_naming.rclk_alt_pins[tk];
                            let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                            let nnode = ngrid.name_node(
                                nloc,
                                if is_alt {
                                    "RCLK_V_SINGLE_R.ALT"
                                } else {
                                    "RCLK_V_SINGLE_R"
                                },
                                [name],
                            );
                            let brx = clk_grid.brxlut[col].1;
                            let bry = clk_grid.brylut[die.die][reg];
                            nnode.add_bel(0, format!("BUFCE_ROW_X{brx}Y{y}", y = bry * 25 + 24));
                            let gtbx = clk_grid.gtbxlut[col].1;
                            let gtby = clk_grid.gtbylut[die.die][reg].1;
                            nnode.add_bel(1, format!("GCLK_TEST_BUFE3_X{gtbx}Y{gtby}"));
                        }
                        "RCLK_V_SINGLE_L.LAG" => {
                            let is_l = col < grid.col_cfg();
                            let tk = if is_l {
                                if grid.is_dmc {
                                    "RCLK_LAG_DMC_L"
                                } else {
                                    "RCLK_LAG_L"
                                }
                            } else {
                                "RCLK_LAG_R"
                            };
                            let is_alt = dev_naming.rclk_alt_pins[tk];
                            let name = format!("{tk}_X{x}Y{y}", x = x - 1, y = y - 1);
                            let nnode = ngrid.name_node(
                                nloc,
                                if is_alt {
                                    "RCLK_V_SINGLE_L.ALT"
                                } else {
                                    "RCLK_V_SINGLE_L"
                                },
                                [name],
                            );
                            let brx = clk_grid.brxlut[col].0;
                            let bry = clk_grid.brylut[die.die][reg];
                            nnode.add_bel(0, format!("BUFCE_ROW_FSR_X{brx}Y{bry}"));
                            let gtbx = clk_grid.gtbxlut[col].0;
                            let gtby = clk_grid.gtbylut[die.die][reg].1;
                            nnode.add_bel(1, format!("GCLK_TEST_BUFE3_X{gtbx}Y{gtby}"));
                        }

                        "RCLK_V_DOUBLE_L" => {
                            let tk = get_bram_tk(edev, has_laguna, die.die, col, row);
                            let is_alt = dev_naming.rclk_alt_pins[tk];
                            let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                            let nnode = ngrid.name_node(
                                nloc,
                                if is_alt {
                                    "RCLK_V_DOUBLE_L.ALT"
                                } else {
                                    "RCLK_V_DOUBLE_L"
                                },
                                [name],
                            );
                            let brx = clk_grid.brxlut[col].0;
                            let bry = clk_grid.brylut[die.die][reg];
                            for i in 0..2 {
                                nnode.add_bel(
                                    i,
                                    format!("BUFCE_ROW_X{x}Y{y}", x = brx + i, y = bry * 25 + 24),
                                );
                            }
                            let gtbx = clk_grid.gtbxlut[col].0;
                            let gtby = clk_grid.gtbylut[die.die][reg].1;
                            for i in 0..2 {
                                nnode.add_bel(
                                    2 + i,
                                    format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                                );
                            }
                        }
                        "RCLK_V_DOUBLE_R" => {
                            let mut brx = clk_grid.brxlut[col].1;
                            let mut gtbx = clk_grid.gtbxlut[col].1;
                            let tk = match grid.kind {
                                GridKind::Ultrascale => "RCLK_DSP_L",
                                GridKind::UltrascalePlus => {
                                    let is_l = col < grid.col_cfg();
                                    let mut is_dc12 = grid.is_dc12();
                                    if grid.is_nocfg() && !grid.has_csec {
                                        if col < grid.cols_hard.first().unwrap().col {
                                            is_dc12 = true;
                                        }
                                        if col > grid.cols_hard.last().unwrap().col {
                                            if reg == grid.reg_cfg() {
                                                is_dc12 = true;
                                            } else {
                                                brx += 2;
                                                gtbx += 2;
                                            }
                                        }
                                    }
                                    if matches!(
                                        grid.columns.last().unwrap().r,
                                        ColumnKindRight::Hard(_, _)
                                    ) && col > grid.cols_hard.first().unwrap().col
                                    {
                                        if reg != grid.reg_cfg() {
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
                            let nnode = ngrid.name_node(
                                nloc,
                                if is_alt {
                                    "RCLK_V_DOUBLE_R.ALT"
                                } else {
                                    "RCLK_V_DOUBLE_R"
                                },
                                [name],
                            );
                            let bry = clk_grid.brylut[die.die][reg];
                            for i in 0..2 {
                                match grid.kind {
                                    GridKind::Ultrascale => nnode.add_bel(
                                        i,
                                        format!(
                                            "BUFCE_ROW_X{x}Y{y}",
                                            x = brx + i,
                                            y = bry * 25 + 24
                                        ),
                                    ),
                                    GridKind::UltrascalePlus => nnode.add_bel(
                                        i,
                                        format!("BUFCE_ROW_FSR_X{x}Y{bry}", x = brx + i,),
                                    ),
                                }
                            }
                            let gtby = clk_grid.gtbylut[die.die][reg].1;
                            for i in 0..2 {
                                nnode.add_bel(
                                    2 + i,
                                    format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                                );
                            }
                        }
                        "RCLK_V_QUAD_L.BRAM" => {
                            let tk = get_bram_tk(edev, has_laguna, die.die, col, row);
                            let is_alt = dev_naming.rclk_alt_pins[tk];
                            let name = format!("{tk}_X{x}Y{y}", y = y - 1);
                            let nnode = ngrid.name_node(
                                nloc,
                                if is_alt {
                                    "RCLK_V_QUAD_L.BRAM.ALT"
                                } else {
                                    "RCLK_V_QUAD_L.BRAM"
                                },
                                [name],
                            );
                            let brx = clk_grid.brxlut[col].0;
                            let bry = clk_grid.brylut[die.die][reg];
                            for i in 0..4 {
                                nnode.add_bel(i, format!("BUFCE_ROW_FSR_X{x}Y{bry}", x = brx + i,));
                            }
                            let gtbx = clk_grid.gtbxlut[col].0;
                            let gtby = clk_grid.gtbylut[die.die][reg].1;
                            for i in 0..4 {
                                nnode.add_bel(
                                    4 + i,
                                    format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                                );
                            }

                            let vsx = clk_grid.vsxlut[col];
                            let vsy = clk_grid.brylut[die.die][reg] * 2;
                            for i in 0..3 {
                                nnode.add_bel(
                                    8 + i,
                                    format!(
                                        "VBUS_SWITCH_X{x}Y{y}",
                                        x = vsx + i / 2,
                                        y = vsy + i % 2
                                    ),
                                );
                            }
                        }
                        "RCLK_V_QUAD_L.URAM" => {
                            let tk = "RCLK_RCLK_URAM_INTF_L_FT";
                            let is_alt = dev_naming.rclk_alt_pins[tk];
                            let name = format!("{tk}_X{x}Y{y}", x = x - 1, y = y - 1);
                            let nnode = ngrid.name_node(
                                nloc,
                                if is_alt {
                                    "RCLK_V_QUAD_L.URAM.ALT"
                                } else {
                                    "RCLK_V_QUAD_L.URAM"
                                },
                                [name],
                            );
                            let brx = clk_grid.brxlut[col].0;
                            let bry = clk_grid.brylut[die.die][reg];
                            for i in 0..4 {
                                nnode.add_bel(i, format!("BUFCE_ROW_FSR_X{x}Y{bry}", x = brx + i,));
                            }
                            let gtbx = clk_grid.gtbxlut[col].0;
                            let gtby = clk_grid.gtbylut[die.die][reg].1;
                            for i in 0..4 {
                                nnode.add_bel(
                                    4 + i,
                                    format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                                );
                            }

                            let vsx = clk_grid.vsxlut[col];
                            let vsy = clk_grid.brylut[die.die][reg] * 2;
                            for i in 0..3 {
                                nnode.add_bel(
                                    8 + i,
                                    format!(
                                        "VBUS_SWITCH_X{x}Y{y}",
                                        x = vsx + i / 2,
                                        y = vsy + i % 2
                                    ),
                                );
                            }
                        }
                        "RCLK_SPLITTER" => {
                            let tk = match grid.kind {
                                GridKind::Ultrascale => "RCLK_DSP_CLKBUF_L",
                                GridKind::UltrascalePlus => "RCLK_DSP_INTF_CLKBUF_L",
                            };
                            ngrid.name_node(
                                nloc,
                                "RCLK_SPLITTER",
                                [format!("{tk}_X{x}Y{y}", y = y - 1)],
                            );
                        }
                        "RCLK_HROUTE_SPLITTER_L.CLE" => {
                            ngrid.name_node(
                                nloc,
                                "RCLK_HROUTE_SPLITTER",
                                [format!("RCLK_CLEM_CLKBUF_L_X{x}Y{y}", y = y - 1)],
                            );
                        }
                        "RCLK_HROUTE_SPLITTER_L.HARD" => {
                            let name = match grid.columns[col].l {
                                ColumnKindLeft::Hard(_, idx) => {
                                    let col_hard = &grid.cols_hard[idx];
                                    match (grid.kind, col_hard.regs[reg]) {
                                        (GridKind::Ultrascale, HardRowKind::Cfg) => {
                                            format!("CFG_CFG_X{x}Y{y}", x = x - 1, y = y - 30)
                                        }
                                        (_, HardRowKind::Ams) => {
                                            let x = if grid.kind == GridKind::UltrascalePlus
                                                && (!hdio_cfg_only[idx] || grid.has_csec)
                                            {
                                                x
                                            } else {
                                                x - 1
                                            };
                                            format!("RCLK_AMS_CFGIO_X{x}Y{y}", y = y - 1)
                                        }
                                        (GridKind::Ultrascale, HardRowKind::Pcie) => {
                                            format!("PCIE_X{x}Y{y}", x = x - 1, y = y - 30)
                                        }
                                        (GridKind::Ultrascale, HardRowKind::Cmac) => {
                                            let x = if col == grid.col_cfg() { x - 1 } else { x };
                                            format!("CMAC_CMAC_FT_X{x}Y{y}", y = y - 30)
                                        }
                                        (GridKind::Ultrascale, HardRowKind::Ilkn) => {
                                            format!(
                                                "ILMAC_ILMAC_FT_X{x}Y{y}",
                                                x = x - 1,
                                                y = y - 30
                                            )
                                        }
                                        (GridKind::UltrascalePlus, HardRowKind::Cfg) => {
                                            let x = if hdio_cfg_only[idx] { x - 1 } else { x };
                                            let tkn = if grid.has_csec {
                                                "CSEC_CONFIG_FT"
                                            } else {
                                                "CFG_CONFIG"
                                            };
                                            format!("{tkn}_X{x}Y{y}", y = y - 30)
                                        }
                                        (GridKind::UltrascalePlus, HardRowKind::Pcie) => {
                                            format!(
                                                "PCIE4_PCIE4_FT_X{x}Y{y}",
                                                x = x - 1,
                                                y = y - 30
                                            )
                                        }
                                        (GridKind::UltrascalePlus, HardRowKind::PciePlus) => {
                                            format!(
                                                "PCIE4C_PCIE4C_FT_X{x}Y{y}",
                                                x = x - 1,
                                                y = y - 30
                                            )
                                        }
                                        (GridKind::UltrascalePlus, HardRowKind::Cmac) => {
                                            format!("CMAC_X{x}Y{y}", x = x - 1, y = y - 30)
                                        }
                                        (GridKind::UltrascalePlus, HardRowKind::Ilkn) => {
                                            format!("ILKN_ILKN_FT_X{x}Y{y}", x = x - 1, y = y - 30)
                                        }
                                        (GridKind::UltrascalePlus, HardRowKind::DfeA) => {
                                            format!(
                                                "DFE_DFE_TILEA_FT_X{x}Y{y}",
                                                x = x - 1,
                                                y = y - 30
                                            )
                                        }
                                        (GridKind::UltrascalePlus, HardRowKind::DfeG) => {
                                            format!(
                                                "DFE_DFE_TILEG_FT_X{x}Y{y}",
                                                x = x - 1,
                                                y = y - 30
                                            )
                                        }
                                        _ => unreachable!(),
                                    }
                                }
                                ColumnKindLeft::DfeE => {
                                    format!("DFE_DFE_TILEE_FT_X{x}Y{y}", x = x - 1, y = y - 30)
                                }
                                _ => unreachable!(),
                            };
                            ngrid.name_node(nloc, "RCLK_HROUTE_SPLITTER", [name]);
                        }
                        "RCLK_HROUTE_SPLITTER_R.HARD" => {
                            let name = format!("DFE_DFE_TILEB_FT_X{x}Y{y}", y = y - 30);
                            ngrid.name_node(nloc, "RCLK_HROUTE_SPLITTER", [name]);
                        }

                        "HBM_ABUS_SWITCH" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [format!("CFRM_CFRAME_TERM_H_FT_X{x}Y{y}")],
                            );
                            let asx = aswitch_grid.xlut[col - 1].hbm;
                            for i in 0..8 {
                                nnode.add_bel(
                                    i,
                                    format!("ABUS_SWITCH_X{x}Y{y}", x = asx + i / 2, y = i % 2),
                                );
                            }
                        }

                        _ => panic!("how to {kind}"),
                    }
                }
            }
        }
    }

    ExpandedNamedDevice { edev, ngrid }
}
