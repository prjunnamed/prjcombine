#![allow(clippy::bool_to_int_with_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::type_complexity)]
#![allow(clippy::single_match)]
#![allow(clippy::implicit_saturating_sub)]

use bimap::BiHashMap;
use enum_map::{enum_map, EnumMap};
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, RowId};
use std::collections::BTreeSet;

use crate::expanded::{
    ClkSrc, ExpandedDevice, Gt, HdioCoord, HpioCoord, Io, IoCoord, IoDiffKind, IoKind,
};
use crate::grid::{
    BramKind, CleMKind, ColSide, Column, ColumnKindLeft, ColumnKindRight, DeviceNaming,
    DisabledPart, DspKind, Grid, GridKind, HardRowKind, HdioIobId, HpioIobId, IoColumn, IoRowKind,
    PsIntfKind, RegId,
};

use crate::bond::SharedCfgPin;

struct Asx {
    gt: usize,
    io: usize,
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

struct DieExpander<'a, 'b, 'c> {
    grid: &'b Grid,
    db: &'a IntDb,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    ylut: EntityVec<RowId, usize>,
    sylut: EntityVec<RowId, usize>,
    asxlut: EntityVec<ColId, Asx>,
    asylut: EntityVec<RegId, Asy>,
    lylut: EntityVec<RowId, usize>,
    ioxlut: EntityPartVec<ColId, usize>,
    ioylut: EntityVec<RegId, (usize, usize)>,
    brxlut: EntityVec<ColId, (usize, usize)>,
    gtbxlut: EntityVec<ColId, (usize, usize)>,
    gtbylut: EntityVec<RegId, (usize, usize)>,
    gtylut: EntityVec<RegId, usize>,
    vsxlut: EntityVec<ColId, usize>,
    cmtxlut: EntityVec<ColId, usize>,
    gtxlut: EntityVec<ColId, usize>,
    bankxlut: EntityPartVec<ColId, u32>,
    bankylut: EntityVec<RegId, u32>,
    dev_has_hbm: bool,
    hylut: EnumMap<HardRowKind, EntityVec<RegId, usize>>,
    iotxlut: EnumMap<IoRowKind, EntityVec<ColId, usize>>,
    iotylut: EnumMap<IoRowKind, EntityVec<RegId, usize>>,
    naming: &'b DeviceNaming,
    io: &'c mut Vec<Io>,
    cfg_io: BiHashMap<SharedCfgPin, IoCoord>,
    gt: &'c mut Vec<Gt>,
    col_cfg_io: Option<(ColId, ColSide)>,
    reg_cfg_io: Option<RegId>,
    is_cfg_io_hrio: bool,
    has_mcap: bool,
}

impl DieExpander<'_, '_, '_> {
    fn fill_asxlut(&mut self) {
        let mut asx = 0;
        for &cd in self.grid.columns.values() {
            let cfg = asx;
            let gt = asx;
            let mut io = asx;
            let mut hbm = asx;
            match cd.l {
                ColumnKindLeft::Gt(idx) | ColumnKindLeft::Io(idx) => {
                    let regs = &self.grid.cols_io[idx].regs;
                    let has_hpio = regs.values().any(|&x| x == IoRowKind::Hpio);
                    let has_hrio = regs.values().any(|&x| x == IoRowKind::Hrio);
                    let has_gt = regs.values().any(|&x| {
                        !matches!(x, IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio)
                    });
                    if has_gt {
                        asx += 1;
                    }
                    io = asx;
                    if has_hrio {
                        asx += 8;
                    } else if has_hpio {
                        match self.grid.kind {
                            GridKind::Ultrascale => asx += 5,
                            GridKind::UltrascalePlus => asx += 8,
                        }
                    }
                }
                ColumnKindLeft::Hard(idx) => {
                    let regs = &self.grid.cols_hard[idx].regs;
                    let has_hdio = regs
                        .values()
                        .any(|x| matches!(x, HardRowKind::Hdio | HardRowKind::HdioAms));
                    let has_cfg = regs.values().any(|&x| x == HardRowKind::Cfg);
                    if has_cfg {
                        io += 1;
                        hbm += 1;
                        asx += 1;
                        if self.dev_has_hbm {
                            asx += 4;
                        }
                    }
                    if has_hdio {
                        asx += 4;
                    }
                }
                _ => (),
            }
            match cd.r {
                ColumnKindRight::Gt(idx) | ColumnKindRight::Io(idx) => {
                    let regs = &self.grid.cols_io[idx].regs;
                    let has_hpio = regs.values().any(|&x| x == IoRowKind::Hpio);
                    let has_hrio = regs.values().any(|&x| x == IoRowKind::Hrio);
                    let has_gt = regs.values().any(|&x| {
                        !matches!(x, IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio)
                    });
                    if has_hrio {
                        asx += 8;
                    } else if has_hpio {
                        match self.grid.kind {
                            GridKind::Ultrascale => asx += 5,
                            GridKind::UltrascalePlus => asx += 8,
                        }
                    } else if has_gt {
                        asx += 1;
                    }
                }
                _ => (),
            }
            self.asxlut.push(Asx { gt, io, cfg, hbm });
        }
    }

    fn fill_asylut(&mut self, mut asy: usize) -> usize {
        for reg in self.grid.regs() {
            let skip = self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg));
            let has_hdio = self
                .grid
                .cols_hard
                .iter()
                .any(|x| matches!(x.regs[reg], HardRowKind::Hdio | HardRowKind::HdioAms))
                && !skip;
            let has_cfg = self
                .grid
                .cols_hard
                .iter()
                .any(|x| x.regs[reg] == HardRowKind::Cfg)
                && !skip;
            let has_hpio = self
                .grid
                .cols_io
                .iter()
                .any(|x| x.regs[reg] == IoRowKind::Hpio)
                && !skip;
            let has_hrio = self
                .grid
                .cols_io
                .iter()
                .any(|x| x.regs[reg] == IoRowKind::Hrio)
                && !skip;
            let has_gt = self.grid.cols_io.iter().any(|x| {
                !matches!(
                    x.regs[reg],
                    IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio
                )
            }) && !skip;

            let cfg = asy;
            let mut cmt = asy;
            if has_cfg || (self.grid.kind == GridKind::UltrascalePlus && has_hpio) {
                asy += 1;
            }
            let gt = asy;
            if has_gt {
                asy += match self.grid.kind {
                    GridKind::Ultrascale => 4,
                    GridKind::UltrascalePlus => 5,
                };
            }
            if self.grid.kind == GridKind::Ultrascale {
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
            self.asylut.push(Asy {
                gt,
                hdio,
                hpio,
                hrio,
                cmt,
                cfg,
            });
        }
        asy
    }

    fn fill_ylut(&mut self, mut y: usize) -> usize {
        if self.grid.kind == GridKind::Ultrascale
            && self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, RegId::from_idx(0)))
        {
            y += 1;
        }
        for row in self.die.rows() {
            self.ylut.push(y);
            let reg = self.grid.row_to_reg(row);
            if !self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg))
            {
                y += 1;
            }
        }
        y
    }

    fn fill_sylut(&mut self, mut y: usize) -> usize {
        for row in self.die.rows() {
            self.sylut.push(y);
            let reg = self.grid.row_to_reg(row);
            if !self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg))
            {
                y += 1;
            }
        }
        y
    }

    fn fill_lylut(&mut self, mut y: usize) -> usize {
        for row in self.die.rows() {
            self.lylut.push(y);
            let reg = self.grid.row_to_reg(row);
            if self.grid.is_laguna_row(row)
                && !self
                    .disabled
                    .contains(&DisabledPart::Region(self.die.die, reg))
            {
                y += 2;
            }
        }
        y
    }

    fn fill_hylut(&mut self, hy: &mut EnumMap<HardRowKind, usize>) {
        for reg in self.grid.regs() {
            let skip = self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg));
            for (k, v) in hy.iter_mut() {
                self.hylut[k].push(*v);
                let mut found = false;
                if !skip {
                    for hc in &self.grid.cols_hard {
                        if hc.regs[reg] == k
                            || (k == HardRowKind::Hdio && hc.regs[reg] == HardRowKind::HdioAms)
                        {
                            found = true;
                        }
                    }
                }
                if found {
                    *v += 1;
                }
            }
        }
    }

    fn fill_iotylut(&mut self, ioty: &mut EnumMap<IoRowKind, usize>) {
        for reg in self.grid.regs() {
            let skip = self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg));
            for (k, v) in ioty.iter_mut() {
                self.iotylut[k].push(*v);
                let mut found = false;
                if !skip {
                    for ioc in &self.grid.cols_io {
                        if ioc.regs[reg] == k {
                            found = true;
                        }
                    }
                }
                if found {
                    *v += 1;
                }
            }
        }
    }

    fn fill_gtylut(&mut self, mut gty: usize) -> usize {
        for reg in self.grid.regs() {
            let skip = self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg));
            self.gtylut.push(gty);
            let mut found = false;
            if !skip {
                for ioc in &self.grid.cols_io {
                    if !matches!(
                        ioc.regs[reg],
                        IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio
                    ) {
                        found = true;
                    }
                }
            }
            if found {
                gty += 1;
            }
        }
        gty
    }

    fn fill_bankylut(&mut self, mut bank: u32) -> u32 {
        for _ in self.grid.regs() {
            self.bankylut.push(bank);
            bank += 1;
        }
        bank
    }

    fn fill_ioxlut(&mut self) {
        let mut iox = 0;
        for (col, &cd) in &self.grid.columns {
            match cd.l {
                ColumnKindLeft::Io(_) => {
                    self.ioxlut.insert(col, iox);
                    self.col_cfg_io = Some((col, ColSide::Left));
                    iox += 1;
                }
                ColumnKindLeft::Hard(idx) => {
                    let regs = &self.grid.cols_hard[idx].regs;
                    if regs
                        .values()
                        .any(|x| matches!(x, HardRowKind::Hdio | HardRowKind::HdioAms))
                    {
                        self.ioxlut.insert(col, iox);
                        iox += 1;
                    }
                    if let Some((reg, _)) = regs.iter().find(|&(_, &k)| k == HardRowKind::Cfg) {
                        self.reg_cfg_io = Some(reg);
                        self.has_mcap = reg.to_idx() != 0
                            && matches!(regs[reg - 1], HardRowKind::Pcie | HardRowKind::PciePlus);
                    }
                }
                _ => (),
            }
            if matches!(cd.r, ColumnKindRight::Io(_)) {
                if self.col_cfg_io.is_none() {
                    self.col_cfg_io = Some((col, ColSide::Right));
                }
                self.ioxlut.insert(col, iox);
                iox += 1;
            }
        }
        let iox_cfg = self.ioxlut[self.col_cfg_io.unwrap().0];
        for (col, &iox) in &self.ioxlut {
            let mut bank = (60 + iox * 20 - iox_cfg * 20) as u32;
            if col.to_idx() == 0
                && iox != iox_cfg
                && self.grid.kind == GridKind::UltrascalePlus
                && self.grid.cols_hard.len() == 1
            {
                bank -= 20;
            }
            self.bankxlut.insert(col, bank);
        }
        let ioc_cfg = self
            .grid
            .cols_io
            .iter()
            .find(|x| x.col == self.col_cfg_io.unwrap().0)
            .unwrap();
        self.is_cfg_io_hrio = ioc_cfg.regs[self.reg_cfg_io.unwrap()] == IoRowKind::Hrio;
    }

    fn fill_brxlut(&mut self) {
        let mut brx = 0;
        let mut gtbx = 0;
        let mut vsx = 0;
        for (col, &cd) in &self.grid.columns {
            self.vsxlut.push(vsx);
            let lbrx = brx;
            let lgtbx = gtbx;
            match cd.l {
                ColumnKindLeft::CleM(CleMKind::ClkBuf) => (),
                ColumnKindLeft::CleM(CleMKind::Laguna)
                    if self.grid.kind == GridKind::UltrascalePlus =>
                {
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
                ColumnKindLeft::Bram(_) | ColumnKindLeft::Uram => match self.grid.kind {
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
                    if self.grid.kind == GridKind::Ultrascale {
                        brx += 1;
                    }
                    gtbx += 1;
                }
                _ => (),
            }
            let rbrx = brx;
            let rgtbx = gtbx;
            match cd.r {
                ColumnKindRight::CleL(_) if self.grid.kind == GridKind::Ultrascale => {
                    brx += 1;
                    gtbx += 1;
                }
                ColumnKindRight::Dsp(DspKind::ClkBuf) => (),
                ColumnKindRight::Dsp(_) => {
                    brx += 2;
                    gtbx += 2;
                }
                _ => (),
            }
            self.brxlut.push((lbrx, rbrx));
            self.gtbxlut.push((lgtbx, rgtbx));
        }
    }

    fn fill_ioylut(&mut self, mut ioy: usize) -> usize {
        for reg in self.grid.regs() {
            let skip = self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg));
            let has_hdio = self
                .grid
                .cols_hard
                .iter()
                .any(|x| matches!(x.regs[reg], HardRowKind::Hdio | HardRowKind::HdioAms))
                && !skip;
            let has_hprio = self
                .grid
                .cols_io
                .iter()
                .any(|x| matches!(x.regs[reg], IoRowKind::Hpio | IoRowKind::Hrio))
                && !skip;
            if has_hprio {
                self.ioylut.push((ioy, ioy + 26));
                ioy += 52;
            } else if has_hdio {
                self.ioylut.push((ioy, ioy + 12));
                ioy += 24;
            } else {
                self.ioylut.push((0, 0));
            }
        }
        ioy
    }

    fn fill_gtbylut(&mut self, mut gtby: usize) -> usize {
        for reg in self.grid.regs() {
            let skip = self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg));
            let has_hprio = self
                .grid
                .cols_io
                .iter()
                .any(|x| matches!(x.regs[reg], IoRowKind::Hpio | IoRowKind::Hrio))
                && !skip;
            if has_hprio {
                match self.grid.kind {
                    GridKind::Ultrascale => {
                        self.gtbylut.push((gtby, gtby + 24));
                    }
                    GridKind::UltrascalePlus => {
                        self.gtbylut.push((gtby, gtby + 18));
                    }
                }
                gtby += 25;
            } else if !skip {
                self.gtbylut.push((gtby, gtby));
                gtby += 1;
            } else {
                self.gtbylut.push((0, 0));
            }
        }
        gtby
    }

    fn fill_cmtxlut(&mut self) {
        let mut cmtx = 0;
        for &cd in self.grid.columns.values() {
            self.cmtxlut.push(cmtx);
            if matches!(cd.l, ColumnKindLeft::Io(_)) {
                cmtx += 1;
            }
            if matches!(cd.r, ColumnKindRight::Io(_)) {
                cmtx += 1;
            }
        }
    }

    fn fill_gtxlut(&mut self) {
        let mut gtx = 0;
        for &cd in self.grid.columns.values() {
            self.gtxlut.push(gtx);
            if let ColumnKindLeft::Io(idx) | ColumnKindLeft::Gt(idx) = cd.l {
                let ioc = &self.grid.cols_io[idx];
                if ioc
                    .regs
                    .values()
                    .any(|x| !matches!(x, IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio))
                {
                    gtx += 1;
                }
            }
            if let ColumnKindRight::Io(idx) | ColumnKindRight::Gt(idx) = cd.r {
                let ioc = &self.grid.cols_io[idx];
                if ioc
                    .regs
                    .values()
                    .any(|x| !matches!(x, IoRowKind::None | IoRowKind::Hpio | IoRowKind::Hrio))
                {
                    gtx += 1;
                }
            }
        }
    }

    fn fill_iotxlut(&mut self) {
        let mut iotx = EnumMap::default();
        for &cd in self.grid.columns.values() {
            for (k, v) in iotx.iter_mut() {
                self.iotxlut[k].push(*v);
                if let ColumnKindLeft::Io(idx) | ColumnKindLeft::Gt(idx) = cd.l {
                    let ioc = &self.grid.cols_io[idx];
                    if ioc.regs.values().any(|&x| x == k) {
                        *v += 1;
                    }
                }
                if let ColumnKindRight::Io(idx) | ColumnKindRight::Gt(idx) = cd.r {
                    let ioc = &self.grid.cols_io[idx];
                    if ioc.regs.values().any(|&x| x == k) {
                        *v += 1;
                    }
                }
            }
        }
    }

    fn fill_int(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let x = col.to_idx();
            for row in self.die.rows() {
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.grid.row_to_reg(row),
                )) {
                    continue;
                }
                let y = self.ylut[row];
                self.die
                    .fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                if row.to_idx() % 60 == 30 {
                    let lr = if col < self.grid.col_cfg() { 'L' } else { 'R' };
                    let name = format!("RCLK_INT_{lr}_X{x}Y{yy}", yy = y - 1);
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("RCLK_INT"),
                        &[&name],
                        self.db.get_node_naming("RCLK_INT"),
                        &[(col, row), (col, row - 1)],
                    );
                    let sy = self.sylut[row] / 60;
                    match self.grid.kind {
                        GridKind::Ultrascale => {
                            node.add_bel(0, format!("BUFCE_LEAF_X16_X{x}Y{y}", y = sy * 2));
                            node.add_bel(1, format!("BUFCE_LEAF_X16_X{x}Y{y}", y = sy * 2 + 1));
                        }
                        GridKind::UltrascalePlus => {
                            for i in 0..16 {
                                node.add_bel(
                                    i,
                                    format!(
                                        "BUFCE_LEAF_X{x}Y{y}",
                                        x = x * 8 + (i & 7),
                                        y = sy * 4 + i / 8
                                    ),
                                );
                                node.add_bel(
                                    i + 16,
                                    format!(
                                        "BUFCE_LEAF_X{x}Y{y}",
                                        x = x * 8 + (i & 7),
                                        y = sy * 4 + i / 8 + 2
                                    ),
                                );
                            }
                        }
                    }
                }
                match cd.l {
                    ColumnKindLeft::CleL | ColumnKindLeft::CleM(_) => (),
                    ColumnKindLeft::Bram(_) | ColumnKindLeft::Uram => {
                        let kind = if self.grid.kind == GridKind::Ultrascale {
                            "INT_INTERFACE_L"
                        } else {
                            "INT_INTF_L"
                        };
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("INTF.W"),
                            &[&format!("{kind}_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.W"),
                            &[(col, row)],
                        );
                    }
                    ColumnKindLeft::Gt(_) | ColumnKindLeft::Io(_) => {
                        let cio = self
                            .grid
                            .cols_io
                            .iter()
                            .find(|x| x.col == col && x.side == ColSide::Left)
                            .unwrap();
                        let rk = cio.regs[self.grid.row_to_reg(row)];
                        match (self.grid.kind, rk) {
                            (_, IoRowKind::None) => (),
                            (GridKind::Ultrascale, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = "INT_INT_INTERFACE_XIPHY_FT";
                                self.die[(col, row)].add_xnode(
                                    self.db.get_node("INTF.W.DELAY"),
                                    &[&format!("{kind}_X{x}Y{y}")],
                                    self.db.get_node_naming("INTF.W.IO"),
                                    &[(col, row)],
                                );
                            }
                            (GridKind::UltrascalePlus, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = if col.to_idx() == 0 {
                                    "INT_INTF_LEFT_TERM_IO_FT"
                                } else if matches!(row.to_idx() % 15, 0 | 1 | 13 | 14) {
                                    "INT_INTF_L_CMT"
                                } else {
                                    "INT_INTF_L_IO"
                                };
                                self.die[(col, row)].add_xnode(
                                    self.db.get_node("INTF.W.IO"),
                                    &[&format!("{kind}_X{x}Y{y}")],
                                    self.db.get_node_naming("INTF.W.IO"),
                                    &[(col, row)],
                                );
                            }
                            _ => {
                                let kind = if self.grid.kind == GridKind::Ultrascale {
                                    "INT_INT_INTERFACE_GT_LEFT_FT"
                                } else {
                                    "INT_INTF_L_TERM_GT"
                                };
                                self.die[(col, row)].add_xnode(
                                    self.db.get_node("INTF.W.DELAY"),
                                    &[&format!("{kind}_X{x}Y{y}")],
                                    self.db.get_node_naming("INTF.W.GT"),
                                    &[(col, row)],
                                );
                            }
                        }
                    }
                    ColumnKindLeft::Hard(_)
                    | ColumnKindLeft::Sdfec
                    | ColumnKindLeft::DfeC
                    | ColumnKindLeft::DfeDF
                    | ColumnKindLeft::DfeE => {
                        let kind = if self.grid.kind == GridKind::Ultrascale {
                            "INT_INTERFACE_PCIE_L"
                        } else {
                            "INT_INTF_L_PCIE4"
                        };
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("INTF.W.DELAY"),
                            &[&format!("{kind}_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.W.PCIE"),
                            &[(col, row)],
                        );
                    }
                }
                match cd.r {
                    ColumnKindRight::CleL(_) => (),
                    ColumnKindRight::Dsp(_) | ColumnKindRight::Uram => {
                        let kind = if self.grid.kind == GridKind::Ultrascale {
                            "INT_INTERFACE_R"
                        } else {
                            "INT_INTF_R"
                        };
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("INTF.E"),
                            &[&format!("{kind}_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.E"),
                            &[(col, row)],
                        );
                    }
                    ColumnKindRight::Gt(_) | ColumnKindRight::Io(_) => {
                        let cio = self
                            .grid
                            .cols_io
                            .iter()
                            .find(|x| x.col == col && x.side == ColSide::Right)
                            .unwrap();
                        let rk = cio.regs[self.grid.row_to_reg(row)];
                        match (self.grid.kind, rk) {
                            (_, IoRowKind::None) => (),
                            (GridKind::Ultrascale, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                unreachable!()
                            }
                            (GridKind::UltrascalePlus, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = "INT_INTF_RIGHT_TERM_IO";
                                self.die[(col, row)].add_xnode(
                                    self.db.get_node("INTF.E.IO"),
                                    &[&format!("{kind}_X{x}Y{y}")],
                                    self.db.get_node_naming("INTF.E.IO"),
                                    &[(col, row)],
                                );
                            }
                            _ => {
                                let kind = if self.grid.kind == GridKind::Ultrascale {
                                    "INT_INTERFACE_GT_R"
                                } else {
                                    "INT_INTF_R_TERM_GT"
                                };
                                self.die[(col, row)].add_xnode(
                                    self.db.get_node("INTF.E.DELAY"),
                                    &[&format!("{kind}_X{x}Y{y}")],
                                    self.db.get_node_naming("INTF.E.GT"),
                                    &[(col, row)],
                                );
                            }
                        }
                    }
                    ColumnKindRight::Hard(_)
                    | ColumnKindRight::DfeB
                    | ColumnKindRight::DfeC
                    | ColumnKindRight::DfeDF
                    | ColumnKindRight::DfeE => {
                        let kind = if self.grid.kind == GridKind::Ultrascale {
                            "INT_INTERFACE_PCIE_R"
                        } else {
                            "INT_INTF_R_PCIE4"
                        };
                        self.die[(col, row)].add_xnode(
                            self.db.get_node("INTF.E.DELAY"),
                            &[&format!("{kind}_X{x}Y{y}")],
                            self.db.get_node_naming("INTF.E.PCIE"),
                            &[(col, row)],
                        );
                    }
                }
            }
        }
    }

    fn fill_io_pass(&mut self) {
        if self.grid.kind == GridKind::UltrascalePlus {
            for (col, &cd) in &self.grid.columns {
                if matches!(cd.l, ColumnKindLeft::Io(_)) && col.to_idx() != 0 {
                    let term_e = self.db.get_term("IO.E");
                    let term_w = self.db.get_term("IO.W");
                    for row in self.die.rows() {
                        if self.disabled.contains(&DisabledPart::Region(
                            self.die.die,
                            self.grid.row_to_reg(row),
                        )) {
                            continue;
                        }
                        self.die
                            .fill_term_pair_anon((col - 1, row), (col, row), term_e, term_w);
                    }
                }
            }
        }
    }

    fn fill_ps(&mut self) {
        if let Some(ps) = self.grid.ps {
            let height = ps.height();
            let width = ps.col.to_idx();
            self.die
                .nuke_rect(ColId::from_idx(0), RowId::from_idx(0), width, height);
            if height != self.grid.regs * 60 {
                let row_t = RowId::from_idx(height);
                for dx in 0..width {
                    let col = ColId::from_idx(dx);
                    self.die.fill_term_anon((col, row_t), "TERM.S");
                }
            }
            let x = ps.col.to_idx();
            for dy in 0..height {
                let row = RowId::from_idx(dy);
                let y = self.ylut[row];
                self.die.fill_term_anon((ps.col, row), "TERM.W");
                self.die[(ps.col, row)].add_xnode(
                    self.db.get_node("INTF.W.IO"),
                    &[&format!("INT_INTF_LEFT_TERM_PSS_X{x}Y{y}")],
                    self.db.get_node_naming("INTF.PSS"),
                    &[(ps.col, row)],
                );
                if dy % 60 == 30 {
                    let tk = match ps.intf_kind {
                        PsIntfKind::Alto => "RCLK_INTF_LEFT_TERM_ALTO",
                        PsIntfKind::Da6 => "RCLK_RCLK_INTF_LEFT_TERM_DA6_FT",
                        PsIntfKind::Da7 => "RCLK_INTF_LEFT_TERM_DA7",
                        PsIntfKind::Da8 => "RCLK_RCLK_INTF_LEFT_TERM_DA8_FT",
                        PsIntfKind::Dc12 => "RCLK_RCLK_INTF_LEFT_TERM_DC12_FT",
                        PsIntfKind::Mx8 => "RCLK_RCLK_INTF_LEFT_TERM_MX8_FT",
                    };
                    let node = self.die[(ps.col, row)].add_xnode(
                        self.db.get_node("RCLK_PS"),
                        &[&format!("{tk}_X{x}Y{y}", y = y - 1)],
                        self.db.get_node_naming("RCLK_PS"),
                        &[(ps.col, row)],
                    );
                    for i in 0..24 {
                        node.add_bel(i, format!("BUFG_PS_X0Y{y}", y = dy / 60 * 24 + i));
                    }
                }
            }
            let row = RowId::from_idx(if ps.has_vcu { 60 } else { 0 });
            let crds: [_; 180] = core::array::from_fn(|i| (ps.col, row + i));
            let name = format!("PSS_ALTO_X0Y{y}", y = self.ylut[row]);
            let node = self.die[(ps.col, row)].add_xnode(
                self.db.get_node("PS"),
                &[&name],
                self.db.get_node_naming("PS"),
                &crds,
            );
            node.add_bel(0, "PS8_X0Y0".to_string());
            if !ps.has_vcu {
                return;
            }
            let row = RowId::from_idx(0);
            let crds: [_; 60] = core::array::from_fn(|i| (ps.col, row + i));
            let name = format!("VCU_VCU_FT_X0Y{y}", y = self.ylut[row]);
            let node = self.die[(ps.col, row)].add_xnode(
                self.db.get_node("VCU"),
                &[&name],
                self.db.get_node_naming("VCU"),
                &crds,
            );
            node.add_bel(0, "VCU_X0Y0".to_string());
        }
    }

    fn fill_term(&mut self) {
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        for col in self.die.cols() {
            if !self.die[(col, row_b)].nodes.is_empty() {
                self.die.fill_term_anon((col, row_b), "TERM.S");
            }
            if !self.die[(col, row_t)].nodes.is_empty() {
                self.die.fill_term_anon((col, row_t), "TERM.N");
            }
        }
        for row in self.die.rows() {
            if !self.die[(col_l, row)].nodes.is_empty() {
                self.die.fill_term_anon((col_l, row), "TERM.W");
            }
            if !self.die[(col_r, row)].nodes.is_empty() {
                self.die.fill_term_anon((col_r, row), "TERM.E");
            }
        }
    }

    fn fill_clb(&mut self) {
        let mut sx = 0;
        let mut lx = 0;
        let dieid = self.die.die;
        for (col, &cd) in &self.grid.columns {
            let is_l = col < self.grid.col_cfg();
            let mut found = false;
            let mut found_laguna = false;
            let x = col.to_idx();
            if let Some((kind, tk)) = match cd.l {
                ColumnKindLeft::CleL => Some(("CLEL_L", "CLEL_L")),
                ColumnKindLeft::CleM(_) => Some((
                    "CLEM",
                    match (self.grid.kind, is_l) {
                        (GridKind::Ultrascale, true) => "CLE_M",
                        (GridKind::Ultrascale, false) => "CLE_M_R",
                        (GridKind::UltrascalePlus, true) => "CLEM",
                        (GridKind::UltrascalePlus, false) => "CLEM_R",
                    },
                )),
                _ => None,
            } {
                for row in self.die.rows() {
                    let tile = &mut self.die[(col, row)];
                    if let Some(ps) = self.grid.ps {
                        if col == ps.col && row.to_idx() < ps.height() {
                            continue;
                        }
                    }
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let y = self.ylut[row];
                    if cd.l == ColumnKindLeft::CleM(CleMKind::Laguna)
                        && self.grid.is_laguna_row(row)
                    {
                        let (x, tk) = match self.grid.kind {
                            GridKind::Ultrascale => (x, "LAGUNA_TILE"),
                            GridKind::UltrascalePlus => (x - 1, "LAG_LAG"),
                        };
                        let name = format!("{tk}_X{x}Y{y}");
                        let node = tile.add_xnode(
                            self.db.get_node("LAGUNA"),
                            &[&name],
                            self.db.get_node_naming("LAGUNA"),
                            &[(col, row)],
                        );
                        let ly = self.lylut[row];
                        node.add_bel(0, format!("LAGUNA_X{lx}Y{ly}"));
                        node.add_bel(1, format!("LAGUNA_X{x}Y{y}", x = lx, y = ly + 1));
                        node.add_bel(2, format!("LAGUNA_X{x}Y{y}", x = lx + 1, y = ly));
                        node.add_bel(3, format!("LAGUNA_X{x}Y{y}", x = lx + 1, y = ly + 1));
                        found_laguna = true;
                    } else {
                        let name = format!("{tk}_X{x}Y{y}");
                        let node = tile.add_xnode(
                            self.db.get_node(kind),
                            &[&name],
                            self.db.get_node_naming(kind),
                            &[(col, row)],
                        );
                        if row.to_idx() % 60 == 59
                            && self
                                .disabled
                                .contains(&DisabledPart::TopRow(dieid, self.grid.row_to_reg(row)))
                        {
                            continue;
                        }
                        let sy = self.sylut[row];
                        node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                        found = true;
                    }
                }
                for reg in self.grid.regs() {
                    let row = self.grid.row_reg_rclk(reg);
                    let tile = &mut self.die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    if let Some(ps) = self.grid.ps {
                        if col == ps.col && row.to_idx() < ps.height() {
                            continue;
                        }
                    }
                    if matches!(cd.l, ColumnKindLeft::CleM(CleMKind::ClkBuf)) {
                        let name = format!("RCLK_CLEM_CLKBUF_L_X{x}Y{y}", y = self.ylut[row - 1]);
                        tile.add_xnode(
                            self.db.get_node("RCLK_HROUTE_SPLITTER_L"),
                            &[&name],
                            self.db.get_node_naming("RCLK_HROUTE_SPLITTER"),
                            &[],
                        );
                    } else {
                        let tk = match (self.grid.kind, cd.l, is_l, self.grid.is_laguna_row(row)) {
                            (GridKind::Ultrascale, ColumnKindLeft::CleL, true, _) => "RCLK_CLEL_L",
                            (GridKind::Ultrascale, ColumnKindLeft::CleL, false, _) => "RCLK_CLEL_R",
                            (
                                GridKind::Ultrascale,
                                ColumnKindLeft::CleM(CleMKind::Laguna),
                                _,
                                true,
                            ) => continue,
                            (GridKind::Ultrascale, ColumnKindLeft::CleM(_), true, _) => {
                                "RCLK_CLE_M_L"
                            }
                            (GridKind::Ultrascale, ColumnKindLeft::CleM(_), false, _) => {
                                "RCLK_CLE_M_R"
                            }
                            (GridKind::UltrascalePlus, ColumnKindLeft::CleL, true, _) => {
                                "RCLK_CLEL_L_L"
                            }
                            (GridKind::UltrascalePlus, ColumnKindLeft::CleL, false, _) => {
                                "RCLK_CLEL_L_R"
                            }
                            (
                                GridKind::UltrascalePlus,
                                ColumnKindLeft::CleM(CleMKind::Laguna),
                                true,
                                true,
                            ) => {
                                if self.grid.is_dmc {
                                    "RCLK_LAG_DMC_L"
                                } else {
                                    "RCLK_LAG_L"
                                }
                            }
                            (
                                GridKind::UltrascalePlus,
                                ColumnKindLeft::CleM(CleMKind::Laguna),
                                false,
                                true,
                            ) => "RCLK_LAG_R",
                            (GridKind::UltrascalePlus, ColumnKindLeft::CleM(_), true, _) => {
                                if self.grid.is_dmc
                                    && cd.l == ColumnKindLeft::CleM(CleMKind::Laguna)
                                {
                                    "RCLK_CLEM_DMC_L"
                                } else {
                                    "RCLK_CLEM_L"
                                }
                            }
                            (GridKind::UltrascalePlus, ColumnKindLeft::CleM(_), false, _) => {
                                "RCLK_CLEM_R"
                            }
                            _ => unreachable!(),
                        };
                        let is_alt = self.naming.rclk_alt_pins[tk];
                        let x = if tk.starts_with("RCLK_LAG") { x - 1 } else { x };
                        let name = format!("{tk}_X{x}Y{y}", y = self.ylut[row - 1]);
                        let node = tile.add_xnode(
                            self.db.get_node("RCLK_V_SINGLE_L"),
                            &[&name],
                            self.db.get_node_naming(if is_alt {
                                "RCLK_V_SINGLE_L.ALT"
                            } else {
                                "RCLK_V_SINGLE_L"
                            }),
                            &[(col, row)],
                        );
                        let reg = self.grid.row_to_reg(row);
                        let mut brx = self.brxlut[col].0;
                        let mut gtbx = self.gtbxlut[col].0;
                        if self.grid.kind == GridKind::UltrascalePlus
                            && cd.l == ColumnKindLeft::CleM(CleMKind::Laguna)
                            && !self.grid.is_laguna_row(row)
                        {
                            brx += 1;
                            gtbx += 1;
                        }
                        match self.grid.kind {
                            GridKind::Ultrascale => node.add_bel(
                                0,
                                format!("BUFCE_ROW_X{brx}Y{y}", y = self.sylut[row] / 60 * 25 + 24),
                            ),
                            GridKind::UltrascalePlus => node.add_bel(
                                0,
                                format!("BUFCE_ROW_FSR_X{brx}Y{y}", y = self.sylut[row] / 60),
                            ),
                        }
                        let gtby = self.gtbylut[reg].1;
                        node.add_bel(1, format!("GCLK_TEST_BUFE3_X{gtbx}Y{gtby}"));
                    }
                }
            }
            if found {
                sx += 1;
            }
            if found_laguna {
                lx += 2;
            }
            let mut found = false;
            if matches!(cd.r, ColumnKindRight::CleL(_)) {
                for row in self.die.rows() {
                    let tile = &mut self.die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let y = self.ylut[row];
                    let name = format!("CLEL_R_X{x}Y{y}");
                    let node = tile.add_xnode(
                        self.db.get_node("CLEL_R"),
                        &[&name],
                        self.db.get_node_naming("CLEL_R"),
                        &[(col, row)],
                    );
                    if row.to_idx() % 60 == 59
                        && self
                            .disabled
                            .contains(&DisabledPart::TopRow(dieid, self.grid.row_to_reg(row)))
                    {
                        continue;
                    }
                    let sy = self.sylut[row];
                    node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                    found = true;
                }
                for reg in self.grid.regs() {
                    let row = self.grid.row_reg_rclk(reg);
                    let tile = &mut self.die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    if self.grid.kind == GridKind::UltrascalePlus {
                        continue;
                    }
                    let tk = if is_l {
                        "RCLK_CLEL_R_L"
                    } else {
                        "RCLK_CLEL_R_R"
                    };
                    let is_alt = self.naming.rclk_alt_pins[tk];
                    let name = format!("{tk}_X{x}Y{y}", y = self.ylut[row - 1]);
                    let node = tile.add_xnode(
                        self.db.get_node("RCLK_V_SINGLE_R"),
                        &[&name],
                        self.db.get_node_naming(if is_alt {
                            "RCLK_V_SINGLE_R.ALT"
                        } else {
                            "RCLK_V_SINGLE_R"
                        }),
                        &[(col, row)],
                    );
                    let reg = self.grid.row_to_reg(row);
                    let brx = self.brxlut[col].1;
                    node.add_bel(
                        0,
                        format!("BUFCE_ROW_X{brx}Y{y}", y = self.sylut[row] / 60 * 25 + 24),
                    );
                    let gtbx = self.gtbxlut[col].1;
                    let gtby = self.gtbylut[reg].1;
                    node.add_bel(1, format!("GCLK_TEST_BUFE3_X{gtbx}Y{gtby}"));
                }
            }
            if found {
                sx += 1;
            }
        }
    }

    fn fill_bram(&mut self) {
        let has_laguna = self
            .grid
            .columns
            .values()
            .any(|cd| cd.l == ColumnKindLeft::CleM(CleMKind::Laguna));
        let mut bx = 0;
        for (col, &cd) in &self.grid.columns {
            if !matches!(cd.l, ColumnKindLeft::Bram(_)) {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                let tile = &mut self.die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = col.to_idx();
                let y = self.ylut[row];
                let name = format!("BRAM_X{x}Y{y}");
                let node = tile.add_xnode(
                    self.db.get_node("BRAM"),
                    &[&name],
                    self.db.get_node_naming("BRAM"),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                let sy = self.sylut[row];
                node.add_bel(0, format!("RAMB36_X{bx}Y{y}", y = sy / 5));
                node.add_bel(1, format!("RAMB18_X{bx}Y{y}", y = sy / 5 * 2));
                node.add_bel(2, format!("RAMB18_X{bx}Y{y}", y = sy / 5 * 2 + 1));
                if row.to_idx() % 60 == 30 {
                    let in_laguna = has_laguna && self.grid.is_laguna_row(row);
                    let tk = match (self.grid.kind, cd.l, col < self.grid.col_cfg()) {
                        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::Plain), true) => {
                            "RCLK_BRAM_L"
                        }
                        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::Plain), false) => {
                            "RCLK_BRAM_R"
                        }
                        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::BramClmp), true) => {
                            "RCLK_RCLK_BRAM_L_BRAMCLMP_FT"
                        }
                        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::AuxClmp), true) => {
                            "RCLK_RCLK_BRAM_L_AUXCLMP_FT"
                        }
                        (
                            GridKind::Ultrascale,
                            ColumnKindLeft::Bram(BramKind::BramClmpMaybe),
                            true,
                        ) => {
                            if in_laguna {
                                "RCLK_BRAM_L"
                            } else {
                                "RCLK_RCLK_BRAM_L_BRAMCLMP_FT"
                            }
                        }
                        (
                            GridKind::Ultrascale,
                            ColumnKindLeft::Bram(BramKind::AuxClmpMaybe),
                            true,
                        ) => {
                            if in_laguna {
                                "RCLK_BRAM_L"
                            } else {
                                "RCLK_RCLK_BRAM_L_AUXCLMP_FT"
                            }
                        }
                        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Plain), true) => {
                            "RCLK_BRAM_INTF_L"
                        }
                        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Td), true) => {
                            "RCLK_BRAM_INTF_TD_L"
                        }
                        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Td), false) => {
                            "RCLK_BRAM_INTF_TD_R"
                        }
                        _ => unreachable!(),
                    };
                    let name_h = format!("{tk}_X{x}Y{y}", y = y - 1);
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("HARD_SYNC"),
                        &[&name_h],
                        self.db.get_node_naming("HARD_SYNC"),
                        &[(col, row)],
                    );
                    node.add_bel(
                        0,
                        format!("HARD_SYNC_X{sx}Y{sy}", sx = bx * 2, sy = sy / 60 * 2),
                    );
                    node.add_bel(
                        1,
                        format!("HARD_SYNC_X{sx}Y{sy}", sx = bx * 2, sy = sy / 60 * 2 + 1),
                    );
                    node.add_bel(
                        2,
                        format!("HARD_SYNC_X{sx}Y{sy}", sx = bx * 2 + 1, sy = sy / 60 * 2),
                    );
                    node.add_bel(
                        3,
                        format!(
                            "HARD_SYNC_X{sx}Y{sy}",
                            sx = bx * 2 + 1,
                            sy = sy / 60 * 2 + 1
                        ),
                    );

                    let is_alt = self.naming.rclk_alt_pins[tk];
                    if self.grid.kind == GridKind::Ultrascale {
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("RCLK_V_DOUBLE_L"),
                            &[&name_h],
                            self.db.get_node_naming(if is_alt {
                                "RCLK_V_DOUBLE_L.ALT"
                            } else {
                                "RCLK_V_DOUBLE_L"
                            }),
                            &[(col, row)],
                        );
                        let reg = self.grid.row_to_reg(row);
                        let brx = self.brxlut[col].0;
                        for i in 0..2 {
                            node.add_bel(
                                i,
                                format!(
                                    "BUFCE_ROW_X{x}Y{y}",
                                    x = brx + i,
                                    y = self.sylut[row] / 60 * 25 + 24
                                ),
                            );
                        }
                        let gtbx = self.gtbxlut[col].0;
                        let gtby = self.gtbylut[reg].1;
                        for i in 0..2 {
                            node.add_bel(
                                2 + i,
                                format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                            );
                        }
                    } else {
                        let node = self.die[(col, row)].add_xnode(
                            self.db.get_node("RCLK_V_QUAD_L"),
                            &[&name_h],
                            self.db.get_node_naming(if is_alt {
                                "RCLK_V_QUAD_L.ALT"
                            } else {
                                "RCLK_V_QUAD_L"
                            }),
                            &[(col, row)],
                        );
                        let reg = self.grid.row_to_reg(row);
                        let brx = self.brxlut[col].0;
                        for i in 0..4 {
                            node.add_bel(
                                i,
                                format!(
                                    "BUFCE_ROW_FSR_X{x}Y{y}",
                                    x = brx + i,
                                    y = self.sylut[row] / 60
                                ),
                            );
                        }
                        let gtbx = self.gtbxlut[col].0;
                        let gtby = self.gtbylut[reg].1;
                        for i in 0..4 {
                            node.add_bel(
                                4 + i,
                                format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i),
                            );
                        }

                        let vsx = self.vsxlut[col];
                        let vsy = self.sylut[row] / 60 * 2;
                        for i in 0..3 {
                            node.add_bel(
                                8 + i,
                                format!("VBUS_SWITCH_X{x}Y{y}", x = vsx + i / 2, y = vsy + i % 2),
                            );
                        }
                    }
                }
            }
            bx += 1;
        }
    }

    fn fill_dsp(&mut self) {
        let dieid = self.die.die;
        let mut dx = 0;
        for (col, &cd) in &self.grid.columns {
            let x = col.to_idx();
            if !matches!(cd.r, ColumnKindRight::Dsp(_)) {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                if self.grid.has_hbm && row.to_idx() < 15 {
                    if row.to_idx() != 0 {
                        continue;
                    }
                    if dx < 16 && self.disabled.contains(&DisabledPart::HbmLeft) {
                        continue;
                    }
                    let tile = &mut self.die[(col, row)];
                    let y = self.ylut[row];
                    let name = format!("BLI_BLI_FT_X{x}Y{y}");
                    let crds: [_; 15] = core::array::from_fn(|i| (col, row + i));
                    let node = tile.add_xnode(
                        self.db.get_node("BLI"),
                        &[&name],
                        self.db.get_node_naming("BLI"),
                        &crds,
                    );
                    node.add_bel(0, format!("BLI_HBM_APB_INTF_X{dx}Y0"));
                    node.add_bel(1, format!("BLI_HBM_AXI_INTF_X{dx}Y0"));
                } else {
                    let tile = &mut self.die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let y = self.ylut[row];
                    let name = format!("DSP_X{x}Y{y}");
                    let node = tile.add_xnode(
                        self.db.get_node("DSP"),
                        &[&name],
                        self.db.get_node_naming("DSP"),
                        &[
                            (col, row),
                            (col, row + 1),
                            (col, row + 2),
                            (col, row + 3),
                            (col, row + 4),
                        ],
                    );
                    let sy = self.ylut[row];
                    let dy = if self.dev_has_hbm { sy / 5 - 3 } else { sy / 5 };
                    node.add_bel(0, format!("DSP48E2_X{dx}Y{y}", y = dy * 2));
                    if row.to_idx() % 60 == 55
                        && self
                            .disabled
                            .contains(&DisabledPart::TopRow(dieid, self.grid.row_to_reg(row)))
                    {
                        continue;
                    }
                    node.add_bel(1, format!("DSP48E2_X{dx}Y{y}", y = dy * 2 + 1));
                }
            }
            for reg in self.grid.regs() {
                let row = self.grid.row_reg_rclk(reg);
                let tile = &mut self.die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                if matches!(cd.r, ColumnKindRight::Dsp(DspKind::ClkBuf)) {
                    let tk = match self.grid.kind {
                        GridKind::Ultrascale => "RCLK_DSP_CLKBUF_L",
                        GridKind::UltrascalePlus => "RCLK_DSP_INTF_CLKBUF_L",
                    };
                    let name = format!("{tk}_X{x}Y{y}", y = self.ylut[row - 1]);
                    tile.add_xnode(
                        self.db.get_node("RCLK_SPLITTER"),
                        &[&name],
                        self.db.get_node_naming("RCLK_SPLITTER"),
                        &[],
                    );
                } else {
                    let tk = match self.grid.kind {
                        GridKind::Ultrascale => "RCLK_DSP_L",
                        GridKind::UltrascalePlus => {
                            let is_l = col < self.grid.col_cfg();
                            if self.grid.is_dc12() {
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
                    let is_alt = self.naming.rclk_alt_pins[tk];
                    let name = format!("{tk}_X{x}Y{y}", y = self.ylut[row - 1]);
                    let node = tile.add_xnode(
                        self.db.get_node("RCLK_V_DOUBLE_R"),
                        &[&name],
                        self.db.get_node_naming(if is_alt {
                            "RCLK_V_DOUBLE_R.ALT"
                        } else {
                            "RCLK_V_DOUBLE_R"
                        }),
                        &[(col, row)],
                    );
                    let reg = self.grid.row_to_reg(row);
                    let brx = self.brxlut[col].1;
                    for i in 0..2 {
                        match self.grid.kind {
                            GridKind::Ultrascale => node.add_bel(
                                i,
                                format!(
                                    "BUFCE_ROW_X{x}Y{y}",
                                    x = brx + i,
                                    y = self.sylut[row] / 60 * 25 + 24
                                ),
                            ),
                            GridKind::UltrascalePlus => node.add_bel(
                                i,
                                format!(
                                    "BUFCE_ROW_FSR_X{x}Y{y}",
                                    x = brx + i,
                                    y = self.sylut[row] / 60
                                ),
                            ),
                        }
                    }
                    let gtbx = self.gtbxlut[col].1;
                    let gtby = self.gtbylut[reg].1;
                    for i in 0..2 {
                        node.add_bel(2 + i, format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i));
                    }
                }
            }
            dx += 1;
        }
    }

    fn fill_uram(&mut self) {
        let mut uyb = 0;
        if let Some(ps) = self.grid.ps {
            uyb = ps.height();
            for (col, &cd) in &self.grid.columns {
                if cd.r == ColumnKindRight::Uram && col >= ps.col {
                    uyb = 0;
                }
            }
        }
        let mut ux = 0;
        for (col, &cd) in &self.grid.columns {
            if cd.r != ColumnKindRight::Uram {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 15 != 0 {
                    continue;
                }
                let tile = &mut self.die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = col.to_idx();
                let y = self.ylut[row];
                let tk = if row.to_idx() % 60 == 45 {
                    "URAM_URAM_DELAY_FT"
                } else {
                    "URAM_URAM_FT"
                };
                let name = format!("{tk}_X{x}Y{y}");
                let mut crds = vec![];
                for dy in 0..15 {
                    crds.push((col, row + dy));
                }
                for dy in 0..15 {
                    crds.push((col + 1, row + dy));
                }
                let node = tile.add_xnode(
                    self.db.get_node("URAM"),
                    &[&name],
                    self.db.get_node_naming("URAM"),
                    &crds,
                );
                let sy = self.ylut[row] - uyb;
                node.add_bel(0, format!("URAM288_X{ux}Y{y}", y = sy / 15 * 4));
                node.add_bel(1, format!("URAM288_X{ux}Y{y}", y = sy / 15 * 4 + 1));
                node.add_bel(2, format!("URAM288_X{ux}Y{y}", y = sy / 15 * 4 + 2));
                node.add_bel(3, format!("URAM288_X{ux}Y{y}", y = sy / 15 * 4 + 3));
                if row.to_idx() % 60 == 30 {
                    let tk = "RCLK_RCLK_URAM_INTF_L_FT";
                    let name_h = format!("{tk}_X{x}Y{y}", y = y - 1);
                    let is_alt = self.naming.rclk_alt_pins[tk];
                    let node = self.die[(col + 1, row)].add_xnode(
                        self.db.get_node("RCLK_V_QUAD_L"),
                        &[&name_h],
                        self.db.get_node_naming(if is_alt {
                            "RCLK_V_QUAD_L.URAM.ALT"
                        } else {
                            "RCLK_V_QUAD_L.URAM"
                        }),
                        &[(col + 1, row)],
                    );
                    let reg = self.grid.row_to_reg(row);
                    let brx = self.brxlut[col + 1].0;
                    for i in 0..4 {
                        node.add_bel(
                            i,
                            format!(
                                "BUFCE_ROW_FSR_X{x}Y{y}",
                                x = brx + i,
                                y = self.sylut[row] / 60
                            ),
                        );
                    }
                    let gtbx = self.gtbxlut[col + 1].0;
                    let gtby = self.gtbylut[reg].1;
                    for i in 0..4 {
                        node.add_bel(4 + i, format!("GCLK_TEST_BUFE3_X{x}Y{gtby}", x = gtbx + i));
                    }

                    let vsx = self.vsxlut[col + 1];
                    let vsy = self.sylut[row] / 60 * 2;
                    for i in 0..3 {
                        node.add_bel(
                            8 + i,
                            format!("VBUS_SWITCH_X{x}Y{y}", x = vsx + i / 2, y = vsy + i % 2),
                        );
                    }
                }
            }
            ux += 1;
        }
    }

    fn fill_hard_single(
        &mut self,
        col: ColId,
        reg: RegId,
        kind: HardRowKind,
        sx: usize,
        sy: usize,
        hdio_cfg_only: bool,
    ) {
        let row = self.grid.row_reg_bot(reg);
        if self
            .disabled
            .contains(&DisabledPart::Region(self.die.die, reg))
        {
            return;
        }
        let mut x = col.to_idx() - 1;
        if self.grid.kind == GridKind::Ultrascale
            && kind == HardRowKind::Cmac
            && col != self.grid.col_cfg()
        {
            x = col.to_idx();
        }
        if self.grid.kind == GridKind::UltrascalePlus
            && matches!(
                kind,
                HardRowKind::Cfg | HardRowKind::Ams | HardRowKind::Hdio | HardRowKind::HdioAms
            )
            && !hdio_cfg_only
        {
            x = col.to_idx();
        }
        let die = self.die.die;
        let (nk, nn, tk, bk) = match kind {
            HardRowKind::None => return,
            HardRowKind::Hdio | HardRowKind::HdioAms => {
                for (i, (tk, nk)) in [
                    ("HDIO_BOT_RIGHT", "HDIO_BOT"),
                    ("HDIO_TOP_RIGHT", "HDIO_TOP"),
                ]
                .into_iter()
                .enumerate()
                {
                    let row = row + i * 30;
                    let name = format!("{tk}_X{x}Y{y}", y = self.ylut[row]);
                    let crds: [_; 60] = core::array::from_fn(|i| {
                        if i < 30 {
                            (col - 1, row + i)
                        } else {
                            (col, row + (i - 30))
                        }
                    });
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node(nk),
                        &[&name],
                        self.db.get_node_naming(nk),
                        &crds,
                    );
                    let iox = self.ioxlut[col];
                    let ioy = match i {
                        0 => self.ioylut[reg].0,
                        1 => self.ioylut[reg].1,
                        _ => unreachable!(),
                    };
                    for j in 0..12 {
                        let name = format!("IOB_X{iox}Y{y}", y = ioy + j);
                        node.add_bel(j, name.clone());
                        let idx = i * 12 + j;
                        let crd = IoCoord::Hdio(HdioCoord {
                            die,
                            col,
                            reg,
                            iob: HdioIobId::from_idx(idx),
                        });
                        let ocrd = IoCoord::Hdio(HdioCoord {
                            die,
                            col,
                            reg,
                            iob: HdioIobId::from_idx(idx ^ 1),
                        });
                        let bank = self.bankxlut[col] + self.bankylut[reg] - 20;
                        self.io.push(Io {
                            kind: IoKind::Hdio,
                            crd,
                            bank,
                            name,
                            diff: if idx % 2 == 0 {
                                IoDiffKind::P(ocrd)
                            } else {
                                IoDiffKind::N(ocrd)
                            },
                            is_gc: (8..16).contains(&idx),
                            is_dbc: false,
                            is_qbc: false,
                            is_vrp: false,
                            sm_pair: match (kind, idx) {
                                (HardRowKind::HdioAms, 0 | 1) => Some(11),
                                (HardRowKind::HdioAms, 2 | 3) => Some(10),
                                (HardRowKind::HdioAms, 4 | 5) => Some(9),
                                (HardRowKind::HdioAms, 6 | 7) => Some(8),
                                (HardRowKind::HdioAms, 8 | 9) => Some(7),
                                (HardRowKind::HdioAms, 10 | 11) => Some(6),
                                (HardRowKind::HdioAms, 12 | 13) => Some(5),
                                (HardRowKind::HdioAms, 14 | 15) => Some(4),
                                (HardRowKind::HdioAms, 16 | 17) => Some(3),
                                (HardRowKind::HdioAms, 18 | 19) => Some(2),
                                (HardRowKind::HdioAms, 20 | 21) => Some(1),
                                (HardRowKind::HdioAms, 22 | 23) => Some(0),
                                (HardRowKind::Hdio, 0 | 1) => Some(15),
                                (HardRowKind::Hdio, 2 | 3) => Some(14),
                                (HardRowKind::Hdio, 4 | 5) => Some(13),
                                (HardRowKind::Hdio, 6 | 7) => Some(12),
                                (HardRowKind::Hdio, 16 | 17) => Some(11),
                                (HardRowKind::Hdio, 18 | 19) => Some(10),
                                (HardRowKind::Hdio, 20 | 21) => Some(9),
                                (HardRowKind::Hdio, 22 | 23) => Some(8),
                                _ => None,
                            },
                        });
                    }
                    for j in 0..6 {
                        node.add_bel(
                            12 + j,
                            format!("HDIOBDIFFINBUF_X{sx}Y{y}", y = sy * 12 + i * 6 + j),
                        );
                        node.add_bel(
                            18 + 2 * j,
                            format!("HDIOLOGIC_M_X{sx}Y{y}", y = sy * 12 + i * 6 + j),
                        );
                        node.add_bel(
                            18 + 2 * j + 1,
                            format!("HDIOLOGIC_S_X{sx}Y{y}", y = sy * 12 + i * 6 + j),
                        );
                    }
                    node.add_bel(30, format!("HDLOGIC_CSSD_X{sx}Y{y}", y = sy * 2 + i));
                    if i == 0 {
                        node.add_bel(31, format!("HDIO_VREF_X{sx}Y{sy}"));
                    } else {
                        node.add_bel(31, format!("HDIO_BIAS_X{sx}Y{sy}"));
                    }
                }
                let name = format!("RCLK_HDIO_X{x}Y{y}", y = self.ylut[row + 29]);
                let crds: [_; 120] = core::array::from_fn(|i| {
                    if i < 60 {
                        (col - 1, row + i)
                    } else {
                        (col, row + (i - 60))
                    }
                });
                let node = self.die[(col, row + 30)].add_xnode(
                    self.db.get_node("RCLK_HDIO"),
                    &[&name],
                    self.db.get_node_naming("RCLK_HDIO"),
                    &crds,
                );
                node.add_bel(0, format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2));
                node.add_bel(
                    1,
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2, y = sy * 2 + 1),
                );
                node.add_bel(
                    2,
                    format!("BUFGCE_HDIO_X{x}Y{y}", x = sx * 2 + 1, y = sy * 2),
                );
                node.add_bel(
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
                    node.add_bel(
                        4 + i,
                        format!(
                            "ABUS_SWITCH_X{x}Y{y}",
                            x = self.asxlut[col].io + x,
                            y = self.asylut[reg].hdio + y
                        ),
                    );
                }
                return;
            }
            HardRowKind::Cfg => {
                if self.grid.kind == GridKind::Ultrascale {
                    ("CFG", "CFG", "CFG_CFG", "CONFIG_SITE")
                } else {
                    ("CFG", "CFG", "CFG_CONFIG", "CONFIG_SITE")
                }
            }
            HardRowKind::Ams => {
                let tk = if self.grid.kind == GridKind::Ultrascale {
                    "CFGIO_IOB"
                } else {
                    "CFGIO_IOB20"
                };
                let name = format!("{tk}_X{x}Y{y}", y = self.ylut[row]);
                let crds: [_; 60] = core::array::from_fn(|i| {
                    if i < 30 {
                        (col - 1, row + i)
                    } else {
                        (col, row + (i - 30))
                    }
                });
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("CFGIO"),
                    &[&name],
                    self.db.get_node_naming("CFGIO"),
                    &crds,
                );
                node.add_bel(0, format!("PMV_X{sx}Y{sy}"));
                node.add_bel(1, format!("PMV2_X{sx}Y{sy}"));
                node.add_bel(2, format!("PMVIOB_X{sx}Y{sy}"));
                node.add_bel(3, format!("MTBF3_X{sx}Y{sy}"));
                if self.grid.kind == GridKind::UltrascalePlus {
                    node.add_bel(4, format!("CFGIO_SITE_X{sx}Y{sy}"));
                }
                let row = row + 30;
                let name = format!("RCLK_AMS_CFGIO_X{x}Y{y}", y = self.ylut[row - 1]);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("RCLK_HROUTE_SPLITTER_L"),
                    &[&name],
                    self.db.get_node_naming("RCLK_HROUTE_SPLITTER"),
                    &[],
                );
                let name = format!("AMS_X{x}Y{y}", y = self.ylut[row]);
                let crds: [_; 60] = core::array::from_fn(|i| {
                    if i < 30 {
                        (col - 1, row + i)
                    } else {
                        (col, row + (i - 30))
                    }
                });
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("AMS"),
                    &[&name],
                    self.db.get_node_naming("AMS"),
                    &crds,
                );
                let bk = if self.grid.kind == GridKind::Ultrascale {
                    "SYSMONE1"
                } else {
                    "SYSMONE4"
                };
                node.add_bel(0, format!("{bk}_X{sx}Y{sy}"));
                return;
            }
            HardRowKind::Pcie => {
                if self.grid.kind == GridKind::Ultrascale {
                    ("PCIE", "PCIE", "PCIE", "PCIE_3_1")
                } else {
                    (
                        "PCIE4",
                        if self.has_mcap {
                            "PCIE4"
                        } else {
                            "PCIE4.NOCFG"
                        },
                        "PCIE4_PCIE4_FT",
                        "PCIE40E4",
                    )
                }
            }
            HardRowKind::PciePlus => (
                "PCIE4C",
                if self.has_mcap {
                    "PCIE4C"
                } else {
                    "PCIE4C.NOCFG"
                },
                "PCIE4C_PCIE4C_FT",
                "PCIE4CE4",
            ),
            HardRowKind::Cmac => {
                if self.grid.kind == GridKind::Ultrascale {
                    ("CMAC", "CMAC", "CMAC_CMAC_FT", "CMAC_SITE")
                } else {
                    ("CMAC", "CMAC", "CMAC", "CMACE4")
                }
            }
            HardRowKind::Ilkn => {
                if self.grid.kind == GridKind::Ultrascale {
                    ("ILKN", "ILKN", "ILMAC_ILMAC_FT", "ILKN_SITE")
                } else {
                    ("ILKN", "ILKN", "ILKN_ILKN_FT", "ILKNE4")
                }
            }
            HardRowKind::DfeA => ("DFE_A", "DFE_A", "DFE_DFE_TILEA_FT", "DFE_A"),
            HardRowKind::DfeG => ("DFE_G", "DFE_G", "DFE_DFE_TILEG_FT", "DFE_G"),
        };
        let name = format!("{tk}_X{x}Y{y}", y = self.ylut[row]);
        self.die[(col, row + 30)].add_xnode(
            self.db.get_node("RCLK_HROUTE_SPLITTER_L"),
            &[&name],
            self.db.get_node_naming("RCLK_HROUTE_SPLITTER"),
            &[],
        );
        let crds: [_; 120] = core::array::from_fn(|i| {
            if i < 60 {
                (col - 1, row + i)
            } else {
                (col, row + (i - 60))
            }
        });
        if self
            .disabled
            .contains(&DisabledPart::HardIp(self.die.die, col, reg))
        {
            return;
        }
        if nk.starts_with("DFE") && self.disabled.contains(&DisabledPart::Dfe) {
            return;
        }
        let node = self.die[(col, row)].add_xnode(
            self.db.get_node(nk),
            &[&name],
            self.db.get_node_naming(nn),
            &crds,
        );
        node.add_bel(0, format!("{bk}_X{sx}Y{sy}"));
        if kind == HardRowKind::Cfg {
            let asx = self.asxlut[col].cfg;
            let asy = self.asylut[reg].cfg;
            node.add_bel(1, format!("ABUS_SWITCH_X{asx}Y{asy}"));
        }
    }

    fn fill_hard(&mut self, hcx: &EnumMap<HardRowKind, usize>, has_pcie_cfg: &mut bool) {
        for (i, hc) in self.grid.cols_hard.iter().enumerate() {
            let is_cfg = hc.regs.values().any(|&x| x == HardRowKind::Cfg);
            let hdio_cfg_only = hc.regs.values().all(|x| {
                matches!(
                    x,
                    HardRowKind::Cfg
                        | HardRowKind::Ams
                        | HardRowKind::Hdio
                        | HardRowKind::HdioAms
                        | HardRowKind::None
                )
            }) || !is_cfg;
            for reg in self.grid.regs() {
                let kind = hc.regs[reg];
                if kind == HardRowKind::Cfg
                    && reg.to_idx() != 0
                    && matches!(hc.regs[reg - 1], HardRowKind::Pcie | HardRowKind::PciePlus)
                {
                    *has_pcie_cfg = true;
                }
                let adjkind = if kind == HardRowKind::HdioAms {
                    HardRowKind::Hdio
                } else {
                    kind
                };
                let sx = if i == 0 { 0 } else { hcx[adjkind] };
                let sy = self.hylut[adjkind][reg];
                self.fill_hard_single(hc.col, reg, kind, sx, sy, hdio_cfg_only);
            }
            if is_cfg && self.grid.has_hbm {
                let name = format!("CFRM_CFRAME_TERM_H_FT_X{x}Y0", x = hc.col.to_idx());
                let node = self.die[(hc.col, RowId::from_idx(0))].add_xnode(
                    self.db.get_node("HBM_ABUS_SWITCH"),
                    &[&name],
                    self.db.get_node_naming("HBM_ABUS_SWITCH"),
                    &[],
                );
                let asx = self.asxlut[hc.col].hbm;
                for i in 0..8 {
                    node.add_bel(
                        i,
                        format!("ABUS_SWITCH_X{x}Y{y}", x = asx + i / 2, y = i % 2),
                    );
                }
            }
        }
    }

    fn add_hpio_iobs(&mut self, die: DieId, ioc: &IoColumn, reg: RegId) -> Vec<String> {
        let kind = ioc.regs[reg];
        let bank = self.bankxlut[ioc.col] + self.bankylut[reg] - 20;
        let mut res = vec![];
        let iobx = self.ioxlut[ioc.col];
        let ioby = self.ioylut[reg].0;
        for i in 0..2 {
            let bank = if bank == 64 && kind == IoRowKind::Hrio {
                94 - (i as u32) * 10
            } else {
                bank
            };
            for j in 0..26 {
                let idx = i * 26 + j;
                let name = format!("IOB_X{iobx}Y{y}", y = ioby + idx);
                res.push(name.clone());
                let crd = IoCoord::Hpio(HpioCoord {
                    die,
                    col: ioc.col,
                    side: ioc.side,
                    reg,
                    iob: HpioIobId::from_idx(idx),
                });
                self.io.push(Io {
                    kind: match kind {
                        IoRowKind::Hpio => IoKind::Hpio,
                        IoRowKind::Hrio => IoKind::Hrio,
                        _ => unreachable!(),
                    },
                    crd,
                    bank,
                    name,
                    diff: if idx % 13 == 12 {
                        IoDiffKind::None
                    } else if idx % 13 % 2 == 0 {
                        IoDiffKind::P(IoCoord::Hpio(HpioCoord {
                            die,
                            col: ioc.col,
                            side: ioc.side,
                            reg,
                            iob: HpioIobId::from_idx(idx + 1),
                        }))
                    } else {
                        IoDiffKind::N(IoCoord::Hpio(HpioCoord {
                            die,
                            col: ioc.col,
                            side: ioc.side,
                            reg,
                            iob: HpioIobId::from_idx(idx - 1),
                        }))
                    },
                    is_vrp: kind == IoRowKind::Hpio && idx == 12,
                    is_gc: matches!(idx, 21 | 22 | 23 | 24 | 26 | 27 | 28 | 29),
                    is_dbc: matches!(idx, 0 | 1 | 6 | 7 | 39 | 40 | 45 | 46),
                    is_qbc: matches!(idx, 13 | 14 | 19 | 20 | 26 | 27 | 32 | 33),
                    sm_pair: match idx {
                        4 | 5 => Some(15),
                        6 | 7 => Some(7),
                        8 | 9 => Some(14),
                        10 | 11 => Some(6),
                        13 | 14 => Some(13),
                        15 | 16 => Some(5),
                        17 | 18 => Some(12),
                        19 | 20 => Some(4),
                        30 | 31 => Some(11),
                        32 | 33 => Some(3),
                        34 | 35 => Some(10),
                        36 | 37 => Some(2),
                        39 | 40 => Some(9),
                        41 | 42 => Some(1),
                        43 | 44 => Some(8),
                        45 | 46 => Some(0),
                        _ => None,
                    },
                });
                if self.col_cfg_io.unwrap() == (ioc.col, ioc.side)
                    && self.reg_cfg_io.unwrap() == reg
                {
                    if let Some(cfg) = if !self.grid.is_alt_cfg {
                        match idx {
                            0 => Some(SharedCfgPin::Rs(0)),
                            1 => Some(SharedCfgPin::Rs(1)),
                            2 => Some(SharedCfgPin::FoeB),
                            3 => Some(SharedCfgPin::FweB),
                            4 => Some(SharedCfgPin::Addr(26)),
                            5 => Some(SharedCfgPin::Addr(27)),
                            6 => Some(SharedCfgPin::Addr(24)),
                            7 => Some(SharedCfgPin::Addr(25)),
                            8 => Some(SharedCfgPin::Addr(22)),
                            9 => Some(SharedCfgPin::Addr(23)),
                            10 => Some(SharedCfgPin::Addr(20)),
                            11 => Some(SharedCfgPin::Addr(21)),
                            12 => Some(SharedCfgPin::Addr(28)),
                            13 => Some(SharedCfgPin::Addr(18)),
                            14 => Some(SharedCfgPin::Addr(19)),
                            15 => Some(SharedCfgPin::Addr(16)),
                            16 => Some(SharedCfgPin::Addr(17)),
                            17 => Some(SharedCfgPin::Data(30)),
                            18 => Some(SharedCfgPin::Data(31)),
                            19 => Some(SharedCfgPin::Data(28)),
                            20 => Some(SharedCfgPin::Data(29)),
                            21 => Some(SharedCfgPin::Data(26)),
                            22 => Some(SharedCfgPin::Data(27)),
                            23 => Some(SharedCfgPin::Data(24)),
                            24 => Some(SharedCfgPin::Data(25)),
                            25 => Some(if self.grid.kind == GridKind::Ultrascale {
                                SharedCfgPin::PerstN1
                            } else {
                                SharedCfgPin::SmbAlert
                            }),
                            26 => Some(SharedCfgPin::Data(22)),
                            27 => Some(SharedCfgPin::Data(23)),
                            28 => Some(SharedCfgPin::Data(20)),
                            29 => Some(SharedCfgPin::Data(21)),
                            30 => Some(SharedCfgPin::Data(18)),
                            31 => Some(SharedCfgPin::Data(19)),
                            32 => Some(SharedCfgPin::Data(16)),
                            33 => Some(SharedCfgPin::Data(17)),
                            34 => Some(SharedCfgPin::Data(14)),
                            35 => Some(SharedCfgPin::Data(15)),
                            36 => Some(SharedCfgPin::Data(12)),
                            37 => Some(SharedCfgPin::Data(13)),
                            38 => Some(SharedCfgPin::CsiB),
                            39 => Some(SharedCfgPin::Data(10)),
                            40 => Some(SharedCfgPin::Data(11)),
                            41 => Some(SharedCfgPin::Data(8)),
                            42 => Some(SharedCfgPin::Data(9)),
                            43 => Some(SharedCfgPin::Data(6)),
                            44 => Some(SharedCfgPin::Data(7)),
                            45 => Some(SharedCfgPin::Data(4)),
                            46 => Some(SharedCfgPin::Data(5)),
                            47 => Some(SharedCfgPin::I2cSclk),
                            48 => Some(SharedCfgPin::I2cSda),
                            49 => Some(SharedCfgPin::EmCclk),
                            50 => Some(SharedCfgPin::Dout),
                            51 => Some(SharedCfgPin::PerstN0),
                            _ => None,
                        }
                    } else {
                        match idx {
                            0 => Some(SharedCfgPin::Rs(1)),
                            1 => Some(SharedCfgPin::FweB),
                            2 => Some(SharedCfgPin::Rs(0)),
                            3 => Some(SharedCfgPin::FoeB),
                            4 => Some(SharedCfgPin::Addr(28)),
                            5 => Some(SharedCfgPin::Addr(26)),
                            6 => Some(SharedCfgPin::SmbAlert),
                            7 => Some(SharedCfgPin::Addr(27)),
                            8 => Some(SharedCfgPin::Addr(24)),
                            9 => Some(SharedCfgPin::Addr(22)),
                            10 => Some(SharedCfgPin::Addr(25)),
                            11 => Some(SharedCfgPin::Addr(23)),
                            12 => Some(SharedCfgPin::Addr(20)),
                            13 => Some(SharedCfgPin::Addr(18)),
                            14 => Some(SharedCfgPin::Addr(16)),
                            15 => Some(SharedCfgPin::Addr(19)),
                            16 => Some(SharedCfgPin::Addr(17)),
                            17 => Some(SharedCfgPin::Data(30)),
                            18 => Some(SharedCfgPin::Data(28)),
                            19 => Some(SharedCfgPin::Data(31)),
                            20 => Some(SharedCfgPin::Data(29)),
                            21 => Some(SharedCfgPin::Data(26)),
                            22 => Some(SharedCfgPin::Data(24)),
                            23 => Some(SharedCfgPin::Data(27)),
                            24 => Some(SharedCfgPin::Data(25)),
                            25 => Some(SharedCfgPin::Addr(21)),
                            26 => Some(SharedCfgPin::CsiB),
                            27 => Some(SharedCfgPin::Data(22)),
                            28 => Some(SharedCfgPin::EmCclk),
                            29 => Some(SharedCfgPin::Data(23)),
                            30 => Some(SharedCfgPin::Data(20)),
                            31 => Some(SharedCfgPin::Data(18)),
                            32 => Some(SharedCfgPin::Data(21)),
                            33 => Some(SharedCfgPin::Data(19)),
                            34 => Some(SharedCfgPin::Data(16)),
                            35 => Some(SharedCfgPin::Data(14)),
                            36 => Some(SharedCfgPin::Data(17)),
                            37 => Some(SharedCfgPin::Data(15)),
                            38 => Some(SharedCfgPin::Data(12)),
                            39 => Some(SharedCfgPin::Data(10)),
                            40 => Some(SharedCfgPin::Data(8)),
                            41 => Some(SharedCfgPin::Data(11)),
                            42 => Some(SharedCfgPin::Data(9)),
                            43 => Some(SharedCfgPin::Data(6)),
                            44 => Some(SharedCfgPin::Data(4)),
                            45 => Some(SharedCfgPin::Data(7)),
                            46 => Some(SharedCfgPin::Data(5)),
                            47 => Some(SharedCfgPin::I2cSclk),
                            48 => Some(SharedCfgPin::Dout),
                            49 => Some(SharedCfgPin::I2cSda),
                            50 => Some(SharedCfgPin::PerstN0),
                            51 => Some(SharedCfgPin::Data(13)),
                            _ => None,
                        }
                    } {
                        self.cfg_io.insert(cfg, crd);
                    }
                }
            }
        }
        res
    }

    fn fill_io(&mut self) {
        let die = self.die.die;
        for ioc in &self.grid.cols_io {
            for reg in self.grid.regs() {
                if self
                    .disabled
                    .contains(&DisabledPart::Region(self.die.die, reg))
                {
                    continue;
                }
                let kind = ioc.regs[reg];
                match kind {
                    IoRowKind::None => (),
                    IoRowKind::Hpio | IoRowKind::Hrio => {
                        let row = self.grid.row_reg_rclk(reg);
                        let crds: [_; 60] = core::array::from_fn(|i| (ioc.col, row - 30 + i));
                        let iob_names = self.add_hpio_iobs(die, ioc, reg);
                        if self.grid.kind == GridKind::Ultrascale {
                            let name = format!(
                                "XIPHY_L_X{x}Y{y}",
                                x = ioc.col.to_idx(),
                                y = self.ylut[row - 30]
                            );
                            let node = self.die[(ioc.col, row)].add_xnode(
                                self.db.get_node("XIPHY"),
                                &[&name],
                                self.db.get_node_naming("XIPHY"),
                                &crds,
                            );
                            let cmtx = self.cmtxlut[ioc.col];
                            let cmty = self.sylut[row - 30] / 60;
                            for i in 0..24 {
                                node.add_bel(
                                    i,
                                    format!(
                                        "BUFCE_ROW_X{x}Y{y}",
                                        x = self.brxlut[ioc.col].0,
                                        y = cmty * 25 + i
                                    ),
                                );
                                node.add_bel(
                                    24 + i,
                                    format!(
                                        "GCLK_TEST_BUFE3_X{x}Y{y}",
                                        x = self.gtbxlut[ioc.col].0,
                                        y = self.gtbylut[reg].0 + i
                                    ),
                                );
                                node.add_bel(
                                    48 + i,
                                    format!("BUFGCE_X{cmtx}Y{y}", y = cmty * 24 + i),
                                );
                            }
                            for i in 0..8 {
                                node.add_bel(
                                    72 + i,
                                    format!("BUFGCTRL_X{cmtx}Y{y}", y = cmty * 8 + i),
                                );
                            }
                            for i in 0..4 {
                                node.add_bel(
                                    80 + i,
                                    format!("BUFGCE_DIV_X{cmtx}Y{y}", y = cmty * 4 + i),
                                );
                            }
                            for i in 0..2 {
                                node.add_bel(
                                    84 + i,
                                    format!("PLLE3_ADV_X{cmtx}Y{y}", y = cmty * 2 + i),
                                );
                            }
                            node.add_bel(86, format!("MMCME3_ADV_X{cmtx}Y{cmty}"));
                            node.add_bel(
                                87,
                                format!(
                                    "ABUS_SWITCH_X{x}Y{y}",
                                    x = self.asxlut[ioc.col].io,
                                    y = self.asylut[reg].cmt
                                ),
                            );
                            for i in 0..52 {
                                node.add_bel(
                                    88 + i,
                                    format!("BITSLICE_RX_TX_X{cmtx}Y{y}", y = cmty * 52 + i),
                                );
                            }
                            for i in 0..8 {
                                node.add_bel(
                                    140 + i,
                                    format!("BITSLICE_TX_X{cmtx}Y{y}", y = cmty * 8 + i),
                                );
                            }
                            for i in 0..8 {
                                node.add_bel(
                                    148 + i,
                                    format!("BITSLICE_CONTROL_X{cmtx}Y{y}", y = cmty * 8 + i),
                                );
                            }
                            for i in 0..8 {
                                node.add_bel(
                                    156 + i,
                                    format!("PLL_SELECT_SITE_X{cmtx}Y{y}", y = cmty * 8 + (i ^ 1)),
                                );
                            }
                            for i in 0..4 {
                                node.add_bel(
                                    164 + i,
                                    format!("RIU_OR_X{cmtx}Y{y}", y = cmty * 4 + i),
                                );
                            }
                            for i in 0..4 {
                                node.add_bel(
                                    168 + i,
                                    format!("XIPHY_FEEDTHROUGH_X{x}Y{cmty}", x = cmtx * 4 + i),
                                );
                            }
                            let mut iobx = ioc.col.to_idx();
                            if iobx != 0 {
                                iobx -= 1;
                            }
                            if kind == IoRowKind::Hpio {
                                let name =
                                    format!("RCLK_HPIO_L_X{iobx}Y{y}", y = self.ylut[row - 1]);
                                let node = self.die[(ioc.col, row)].add_xnode(
                                    self.db.get_node("RCLK_HPIO"),
                                    &[&name],
                                    self.db.get_node_naming("RCLK_HPIO"),
                                    &crds,
                                );
                                for i in 0..5 {
                                    node.add_bel(
                                        i,
                                        format!(
                                            "ABUS_SWITCH_X{x}Y{y}",
                                            x = self.asxlut[ioc.col].io + i,
                                            y = self.asylut[reg].hpio
                                        ),
                                    );
                                }
                                node.add_bel(5, format!("HPIO_ZMATCH_BLK_HCLK_X{cmtx}Y{cmty}"));
                                for i in 0..2 {
                                    let row = row - 30 + i * 30;
                                    let name = format!("HPIO_L_X{iobx}Y{y}", y = self.ylut[row]);
                                    let node = self.die[(ioc.col, row)].add_xnode(
                                        self.db.get_node("HPIO"),
                                        &[&name],
                                        self.db.get_node_naming(if self.is_cfg_io_hrio {
                                            "HPIO.NOCFG"
                                        } else {
                                            "HPIO"
                                        }),
                                        &crds[i * 30..i * 30 + 30],
                                    );
                                    for j in 0..26 {
                                        node.add_bel(j, iob_names[i * 26 + j].clone());
                                    }
                                    for j in 0..12 {
                                        node.add_bel(
                                            26 + j,
                                            format!(
                                                "HPIOBDIFFINBUF_X{cmtx}Y{y}",
                                                y = cmty * 24 + i * 12 + j
                                            ),
                                        );
                                    }
                                    for j in 0..12 {
                                        node.add_bel(
                                            38 + j,
                                            format!(
                                                "HPIOBDIFFOUTBUF_X{cmtx}Y{y}",
                                                y = cmty * 24 + i * 12 + j
                                            ),
                                        );
                                    }
                                    node.add_bel(
                                        50,
                                        format!("HPIO_VREF_SITE_X{cmtx}Y{y}", y = cmty * 2 + i),
                                    );
                                }
                            } else {
                                let name =
                                    format!("RCLK_HRIO_L_X{iobx}Y{y}", y = self.ylut[row - 1]);
                                let node = self.die[(ioc.col, row)].add_xnode(
                                    self.db.get_node("RCLK_HRIO"),
                                    &[&name],
                                    self.db.get_node_naming("RCLK_HRIO"),
                                    &[],
                                );
                                for i in 0..8 {
                                    node.add_bel(
                                        i,
                                        format!(
                                            "ABUS_SWITCH_X{x}Y{y}",
                                            x = self.asxlut[ioc.col].io + i,
                                            y = self.asylut[reg].hrio
                                        ),
                                    );
                                }
                                for i in 0..2 {
                                    let row = row - 30 + i * 30;
                                    let name = format!("HRIO_L_X{iobx}Y{y}", y = self.ylut[row]);
                                    let node = self.die[(ioc.col, row)].add_xnode(
                                        self.db.get_node("HRIO"),
                                        &[&name],
                                        self.db.get_node_naming(if self.is_cfg_io_hrio {
                                            "HRIO"
                                        } else {
                                            "HRIO.NOCFG"
                                        }),
                                        &crds[i * 30..i * 30 + 30],
                                    );
                                    for j in 0..26 {
                                        node.add_bel(j, iob_names[i * 26 + j].clone());
                                    }
                                    let hrioy = self.iotylut[IoRowKind::Hrio][reg];
                                    for j in 0..12 {
                                        node.add_bel(
                                            26 + j,
                                            format!(
                                                "HRIODIFFINBUF_X0Y{y}",
                                                y = hrioy * 24 + i * 12 + j
                                            ),
                                        );
                                    }
                                    for j in 0..12 {
                                        node.add_bel(
                                            38 + j,
                                            format!(
                                                "HRIODIFFOUTBUF_X0Y{y}",
                                                y = hrioy * 24 + i * 12 + j
                                            ),
                                        );
                                    }
                                }
                            }
                        } else {
                            let is_hbm = self.grid.has_hbm && reg.to_idx() == 0;
                            let (kind, tk) = if ioc.side == ColSide::Right {
                                ("CMT_R", "CMT_RIGHT")
                            } else if is_hbm {
                                ("CMT_L_HBM", "CMT_LEFT_H")
                            } else {
                                ("CMT_L", "CMT_L")
                            };
                            let name = format!(
                                "{tk}_X{x}Y{y}",
                                x = ioc.col.to_idx(),
                                y = self.ylut[row - 30]
                            );
                            let node = self.die[(ioc.col, row)].add_xnode(
                                self.db.get_node(kind),
                                &[&name],
                                self.db.get_node_naming(kind),
                                &crds,
                            );
                            let cmtx = self.cmtxlut[ioc.col];
                            let cmty = self.sylut[row - 30] / 60;
                            let gtbx = if ioc.side == ColSide::Left {
                                self.gtbxlut[ioc.col].0
                            } else {
                                self.gtbxlut[ioc.col].1
                            };
                            for i in 0..24 {
                                node.add_bel(
                                    i,
                                    format!("BUFCE_ROW_X{cmtx}Y{y}", y = cmty * 24 + i),
                                );
                                node.add_bel(
                                    24 + i,
                                    format!(
                                        "GCLK_TEST_BUFE3_X{gtbx}Y{y}",
                                        y = self.gtbylut[reg].0 + if i < 18 { i } else { i + 1 }
                                    ),
                                );
                                node.add_bel(
                                    48 + i,
                                    format!("BUFGCE_X{cmtx}Y{y}", y = cmty * 24 + i),
                                );
                            }
                            for i in 0..8 {
                                node.add_bel(
                                    72 + i,
                                    format!("BUFGCTRL_X{cmtx}Y{y}", y = cmty * 8 + i),
                                );
                            }
                            for i in 0..4 {
                                node.add_bel(
                                    80 + i,
                                    format!("BUFGCE_DIV_X{cmtx}Y{y}", y = cmty * 4 + i),
                                );
                            }
                            for i in 0..2 {
                                node.add_bel(84 + i, format!("PLL_X{cmtx}Y{y}", y = cmty * 2 + i));
                            }
                            node.add_bel(86, format!("MMCM_X{cmtx}Y{cmty}"));
                            let asx = if ioc.side == ColSide::Left {
                                self.asxlut[ioc.col].io + 7
                            } else {
                                self.asxlut[ioc.col].io
                            };
                            node.add_bel(
                                87,
                                format!("ABUS_SWITCH_X{asx}Y{y}", y = self.asylut[reg].cmt),
                            );
                            if is_hbm {
                                node.add_bel(88, "HBM_REF_CLK_X0Y0".to_string());
                                node.add_bel(89, "HBM_REF_CLK_X0Y1".to_string());
                            }

                            let (tk, kind) = if ioc.side == ColSide::Right {
                                ("RCLK_XIPHY_OUTER_RIGHT", "RCLK_XIPHY_R")
                            } else {
                                ("RCLK_RCLK_XIPHY_INNER_FT", "RCLK_XIPHY_L")
                            };
                            let name = format!(
                                "{tk}_X{x}Y{y}",
                                x = ioc.col.to_idx(),
                                y = self.ylut[row - 1]
                            );
                            self.die[(ioc.col, row)].add_xnode(
                                self.db.get_node(kind),
                                &[&name],
                                self.db.get_node_naming(kind),
                                &[],
                            );

                            for i in 0..4 {
                                let (kind, tk) = if ioc.side == ColSide::Right {
                                    ("XIPHY_R", "XIPHY_BYTE_RIGHT")
                                } else {
                                    ("XIPHY_L", "XIPHY_BYTE_L")
                                };
                                let row = self.grid.row_reg_bot(reg) + i * 15;
                                let name = format!(
                                    "{tk}_X{x}Y{y}",
                                    x = ioc.col.to_idx(),
                                    y = self.ylut[row]
                                );
                                let node = self.die[(ioc.col, row)].add_xnode(
                                    self.db.get_node(kind),
                                    &[&name],
                                    self.db.get_node_naming(kind),
                                    &crds[i * 15..i * 15 + 15],
                                );
                                let phyx = self.cmtxlut[ioc.col];
                                let phyy = self.sylut[row] / 15;
                                for i in 0..13 {
                                    node.add_bel(
                                        i,
                                        format!("BITSLICE_RX_TX_X{phyx}Y{y}", y = phyy * 13 + i),
                                    );
                                }
                                for i in 0..2 {
                                    node.add_bel(
                                        13 + i,
                                        format!("BITSLICE_TX_X{phyx}Y{y}", y = phyy * 2 + i),
                                    );
                                }
                                for i in 0..2 {
                                    node.add_bel(
                                        15 + i,
                                        format!("BITSLICE_CONTROL_X{phyx}Y{y}", y = phyy * 2 + i),
                                    );
                                }
                                for i in 0..2 {
                                    node.add_bel(
                                        17 + i,
                                        format!("PLL_SELECT_SITE_X{phyx}Y{y}", y = phyy * 2 + i),
                                    );
                                }
                                node.add_bel(19, format!("RIU_OR_X{phyx}Y{phyy}"));
                                node.add_bel(20, format!("XIPHY_FEEDTHROUGH_X{phyx}Y{phyy}"));
                            }

                            let mut iobx = ioc.col.to_idx();
                            if iobx != 0 && ioc.side == ColSide::Left {
                                iobx -= 1;
                            }
                            for i in 0..2 {
                                let (kind, kind_alt, tk) = if ioc.side == ColSide::Right {
                                    ("HPIO_R", "HPIO_R.ALTCFG", "HPIO_RIGHT")
                                } else {
                                    ("HPIO_L", "HPIO_L.ALTCFG", "HPIO_L")
                                };
                                let row = self.grid.row_reg_bot(reg) + i * 30;
                                let name = format!("{tk}_X{iobx}Y{y}", y = self.ylut[row]);
                                let node = self.die[(ioc.col, row)].add_xnode(
                                    self.db.get_node(kind),
                                    &[&name],
                                    self.db.get_node_naming(
                                        if self.grid.ps.is_some()
                                            && (self.grid.is_alt_cfg
                                                || !self.disabled.contains(&DisabledPart::Ps))
                                        {
                                            kind_alt
                                        } else {
                                            kind
                                        },
                                    ),
                                    &crds[i * 30..i * 30 + 30],
                                );
                                for j in 0..26 {
                                    node.add_bel(j, iob_names[i * 26 + j].clone());
                                }
                                for j in 0..12 {
                                    node.add_bel(
                                        26 + j,
                                        format!(
                                            "HPIOBDIFFINBUF_X{cmtx}Y{y}",
                                            y = cmty * 24 + i * 12 + j
                                        ),
                                    );
                                }
                                for j in 0..12 {
                                    node.add_bel(
                                        38 + j,
                                        format!(
                                            "HPIOBDIFFOUTBUF_X{cmtx}Y{y}",
                                            y = cmty * 24 + i * 12 + j
                                        ),
                                    );
                                }
                                for j in 0..2 {
                                    node.add_bel(
                                        50 + j,
                                        format!(
                                            "HPIOB_DCI_SNGL_X{cmtx}Y{y}",
                                            y = cmty * 4 + i * 2 + j
                                        ),
                                    );
                                }
                                node.add_bel(
                                    52,
                                    format!("HPIO_VREF_SITE_X{cmtx}Y{y}", y = cmty * 2 + i),
                                );
                                node.add_bel(53, format!("BIAS_X{cmtx}Y{y}", y = cmty * 2 + i));
                            }

                            let kind = if ioc.side == ColSide::Left {
                                "RCLK_HPIO_L"
                            } else {
                                "RCLK_HPIO_R"
                            };
                            let name = format!("{kind}_X{iobx}Y{y}", y = self.ylut[row - 1]);
                            let node = self.die[(ioc.col, row)].add_xnode(
                                self.db.get_node(kind),
                                &[&name],
                                self.db.get_node_naming(kind),
                                &crds,
                            );
                            let asx = if ioc.side == ColSide::Left {
                                self.asxlut[ioc.col].io
                            } else {
                                self.asxlut[ioc.col].io + 1
                            };
                            for i in 0..7 {
                                node.add_bel(
                                    i,
                                    format!(
                                        "ABUS_SWITCH_X{x}Y{y}",
                                        x = asx + i,
                                        y = self.asylut[reg].hpio
                                    ),
                                );
                            }
                            node.add_bel(7, format!("HPIO_ZMATCH_BLK_HCLK_X{cmtx}Y{cmty}"));
                            node.add_bel(8, format!("HPIO_RCLK_PRBS_X{cmtx}Y{cmty}"));
                        }
                    }
                    _ => {
                        let bank =
                            self.bankylut[reg] + if ioc.col.to_idx() == 0 { 100 } else { 200 };
                        let mut name_channel = vec![];
                        let name_common;
                        let row = self.grid.row_reg_rclk(reg);
                        let crds: [_; 60] = core::array::from_fn(|i| (ioc.col, row - 30 + i));
                        if self.grid.kind == GridKind::Ultrascale {
                            let (tk, nk, gtk) = match (kind, ioc.side) {
                                (IoRowKind::Gth, ColSide::Left) => {
                                    ("GTH_QUAD_LEFT_FT", "GTH_L", "GTH")
                                }
                                (IoRowKind::Gty, ColSide::Left) => {
                                    ("GTY_QUAD_LEFT_FT", "GTY_L", "GTY")
                                }
                                (IoRowKind::Gth, ColSide::Right) => ("GTH_R", "GTH_R", "GTH"),
                                _ => unreachable!(),
                            };
                            let name = format!(
                                "{tk}_X{x}Y{y}",
                                x = ioc.col.to_idx(),
                                y = self.ylut[row - 30]
                            );
                            let node = self.die[(ioc.col, row)].add_xnode(
                                self.db.get_node(nk),
                                &[&name],
                                self.db.get_node_naming(nk),
                                &crds,
                            );
                            let gtx = self.gtxlut[ioc.col];
                            let gty = self.gtylut[reg];
                            for i in 0..24 {
                                node.add_bel(i, format!("BUFG_GT_X{gtx}Y{y}", y = gty * 24 + i));
                            }
                            for i in 0..11 {
                                node.add_bel(
                                    24 + i,
                                    format!("BUFG_GT_SYNC_X{gtx}Y{y}", y = gty * 11 + i),
                                );
                            }
                            for i in 0..4 {
                                node.add_bel(
                                    35 + i,
                                    format!(
                                        "ABUS_SWITCH_X{x}Y{y}",
                                        x = self.asxlut[ioc.col].gt,
                                        y = self.asylut[reg].gt + i
                                    ),
                                );
                            }
                            let iotx = self.iotxlut[kind][ioc.col];
                            let ioty = self.iotylut[kind][reg];
                            for i in 0..4 {
                                let name = format!("{gtk}E3_CHANNEL_X{iotx}Y{y}", y = ioty * 4 + i);
                                node.add_bel(39 + i, name.clone());
                                name_channel.push(name);
                            }
                            name_common = format!("{gtk}E3_COMMON_X{iotx}Y{ioty}");
                            node.add_bel(43, name_common.clone());
                        } else {
                            let (tk, nk) = match (kind, ioc.side) {
                                (IoRowKind::Gth, ColSide::Left) => ("GTH_QUAD_LEFT", "GTH_L"),
                                (IoRowKind::Gth, ColSide::Right) => ("GTH_QUAD_RIGHT", "GTH_R"),
                                (IoRowKind::Gty, ColSide::Left) => ("GTY_L", "GTY_L"),
                                (IoRowKind::Gty, ColSide::Right) => ("GTY_R", "GTY_R"),
                                (IoRowKind::Gtf, ColSide::Left) => ("GTFY_QUAD_LEFT_FT", "GTF_L"),
                                (IoRowKind::Gtf, ColSide::Right) => ("GTFY_QUAD_RIGHT_FT", "GTF_R"),
                                (IoRowKind::Gtm, ColSide::Left) => ("GTM_DUAL_LEFT_FT", "GTM_L"),
                                (IoRowKind::Gtm, ColSide::Right) => ("GTM_DUAL_RIGHT_FT", "GTM_R"),
                                (IoRowKind::HsAdc, ColSide::Right) => {
                                    ("HSADC_HSADC_RIGHT_FT", "HSADC_R")
                                }
                                (IoRowKind::HsDac, ColSide::Right) => {
                                    ("HSDAC_HSDAC_RIGHT_FT", "HSDAC_R")
                                }
                                (IoRowKind::RfAdc, ColSide::Right) => {
                                    ("RFADC_RFADC_RIGHT_FT", "RFADC_R")
                                }
                                (IoRowKind::RfDac, ColSide::Right) => {
                                    ("RFDAC_RFDAC_RIGHT_FT", "RFDAC_R")
                                }
                                _ => unreachable!(),
                            };
                            let name = format!(
                                "{tk}_X{x}Y{y}",
                                x = ioc.col.to_idx(),
                                y = self.ylut[row - 30]
                            );
                            let node = self.die[(ioc.col, row)].add_xnode(
                                self.db.get_node(nk),
                                &[&name],
                                self.db.get_node_naming(nk),
                                &crds,
                            );
                            let gtx = self.gtxlut[ioc.col];
                            let gty = self.gtylut[reg];
                            for i in 0..24 {
                                node.add_bel(i, format!("BUFG_GT_X{gtx}Y{y}", y = gty * 24 + i));
                            }
                            for i in 0..15 {
                                node.add_bel(
                                    24 + i,
                                    format!("BUFG_GT_SYNC_X{gtx}Y{y}", y = gty * 15 + i),
                                );
                            }
                            for i in 0..5 {
                                node.add_bel(
                                    39 + i,
                                    format!(
                                        "ABUS_SWITCH_X{x}Y{y}",
                                        x = self.asxlut[ioc.col].gt,
                                        y = self.asylut[reg].gt + i
                                    ),
                                );
                            }
                            let iotx = self.iotxlut[kind][ioc.col];
                            let ioty = self.iotylut[kind][reg];
                            if nk.starts_with("GTM") {
                                let name = format!("GTM_DUAL_X{iotx}Y{ioty}");
                                node.add_bel(44, name.clone());
                                name_channel.push(name);
                                name_common = format!("GTM_REFCLK_X{iotx}Y{ioty}");
                                node.add_bel(45, name_common.clone());
                            } else if nk.starts_with("GT") {
                                let gtk = &nk[..3];
                                let pref = if gtk == "GTF" {
                                    "GTF".to_string()
                                } else {
                                    format!("{gtk}E4")
                                };

                                for i in 0..4 {
                                    let name =
                                        format!("{pref}_CHANNEL_X{iotx}Y{y}", y = ioty * 4 + i);
                                    node.add_bel(44 + i, name.clone());
                                    name_channel.push(name);
                                }
                                name_common = format!("{pref}_COMMON_X{iotx}Y{ioty}");
                                node.add_bel(48, name_common.clone());
                            } else {
                                let bk = &nk[..5];
                                name_common = format!("{bk}_X{iotx}Y{ioty}");
                                node.add_bel(44, name_common.clone());
                            }
                        }
                        self.gt.push(Gt {
                            die,
                            col: ioc.col,
                            side: ioc.side,
                            reg,
                            bank,
                            kind,
                            name_common,
                            name_channel,
                        });
                    }
                }
            }
        }
    }

    fn fill_fe(&mut self) {
        if self.disabled.contains(&DisabledPart::Sdfec) {
            return;
        }
        for (col, &cd) in &self.grid.columns {
            if cd.l == ColumnKindLeft::Sdfec {
                for reg in self.grid.regs() {
                    if self
                        .disabled
                        .contains(&DisabledPart::Region(self.die.die, reg))
                    {
                        continue;
                    }
                    let row = self.grid.row_reg_bot(reg);
                    let name = format!(
                        "FE_FE_FT_X{x}Y{y}",
                        x = col.to_idx() - 1,
                        y = self.ylut[row]
                    );
                    let crds: [_; 60] = core::array::from_fn(|i| (col, row + i));
                    let node = self.die[(col, row)].add_xnode(
                        self.db.get_node("FE"),
                        &[&name],
                        self.db.get_node_naming("FE"),
                        &crds,
                    );
                    node.add_bel(0, format!("FE_X0Y{y}", y = self.sylut[row] / 60));
                }
            }
        }
    }

    fn fill_dfe(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let (kind, bi) = match cd.r {
                ColumnKindRight::DfeB => ("DFE_B", false),
                ColumnKindRight::DfeC => ("DFE_C", true),
                ColumnKindRight::DfeDF => ("DFE_D", true),
                ColumnKindRight::DfeE => ("DFE_E", true),
                _ => continue,
            };
            for reg in self.grid.regs() {
                let row = self.grid.row_reg_bot(reg);
                let kind = if kind == "DFE_D" && reg.to_idx() == 2 {
                    "DFE_F"
                } else {
                    kind
                };
                let tk = match kind {
                    "DFE_B" => "DFE_DFE_TILEB_FT",
                    "DFE_C" => "DFE_DFE_TILEC_FT",
                    "DFE_D" => "DFE_DFE_TILED_FT",
                    "DFE_E" => "DFE_DFE_TILEE_FT",
                    "DFE_F" => "DFE_DFE_TILEF_FT",
                    _ => unreachable!(),
                };
                let name = format!("{tk}_X{x}Y{y}", x = col.to_idx(), y = self.ylut[row]);
                if matches!(cd.r, ColumnKindRight::DfeB | ColumnKindRight::DfeE) {
                    self.die[(if bi { col + 1 } else { col }, row + 30)].add_xnode(
                        self.db.get_node(if bi {
                            "RCLK_HROUTE_SPLITTER_L"
                        } else {
                            "RCLK_HROUTE_SPLITTER_R"
                        }),
                        &[&name],
                        self.db.get_node_naming("RCLK_HROUTE_SPLITTER"),
                        &[],
                    );
                }
                if self.disabled.contains(&DisabledPart::Dfe) {
                    continue;
                }
                let crds: [_; 120] = core::array::from_fn(|i| {
                    if i < 60 {
                        (col, row + i)
                    } else {
                        (col + 1, row + (i - 60))
                    }
                });
                let node = self.die[(if bi { col + 1 } else { col }, row)].add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    if bi { &crds } else { &crds[..60] },
                );
                let mut sy = self.sylut[row] / 60;
                if kind == "DFE_F" {
                    sy = 0;
                } else if kind == "DFE_D" && reg.to_idx() > 2 {
                    sy -= 1;
                }
                node.add_bel(0, format!("{kind}_X0Y{sy}"));
            }
        }
    }

    fn fill_clkroot(&mut self) {
        for col in self.die.cols() {
            for row in self.die.rows() {
                let crow = RowId::from_idx(if row.to_idx() % 60 < 30 {
                    row.to_idx() / 60 * 60 + 29
                } else {
                    row.to_idx() / 60 * 60 + 30
                });
                self.die[(col, row)].clkroot = (col, crow);
            }
        }
    }
}

pub fn fill_clk_src(
    columns: &EntityVec<ColId, Column>,
) -> (
    EntityVec<ColId, EnumMap<ColSide, ClkSrc>>,
    EntityVec<ColId, EnumMap<ColSide, ClkSrc>>,
) {
    let mut hroute_src = vec![];
    let mut hdistr_src = vec![];
    let mut hd = ClkSrc::Gt(columns.last_id().unwrap());
    let mut hr = ClkSrc::Gt(columns.last_id().unwrap());
    for (col, &cd) in columns.iter().rev() {
        let rhd = hd;
        let rhr = hr;
        match cd.r {
            ColumnKindRight::Dsp(DspKind::ClkBuf) => {
                hd = ClkSrc::DspSplitter(col);
                hr = ClkSrc::DspSplitter(col);
            }
            ColumnKindRight::DfeB => {
                hr = ClkSrc::RouteSplitter(col);
            }
            _ => (),
        }
        hroute_src.push(enum_map! {
            ColSide::Left => hr,
            ColSide::Right => rhr,
        });
        hdistr_src.push(enum_map! {
            ColSide::Left => hd,
            ColSide::Right => rhd,
        });
        match cd.l {
            ColumnKindLeft::CleM(CleMKind::ClkBuf)
            | ColumnKindLeft::Hard(_)
            | ColumnKindLeft::DfeE => {
                hr = ClkSrc::RouteSplitter(col);
            }
            ColumnKindLeft::Io(_) => {
                hr = ClkSrc::Cmt(col);
                hd = ClkSrc::Cmt(col);
            }
            _ => (),
        }
    }
    (
        hroute_src.into_iter().rev().collect(),
        hdistr_src.into_iter().rev().collect(),
    )
}

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &'a Grid>,
    grid_master: DieId,
    disabled: &BTreeSet<DisabledPart>,
    naming: &'a DeviceNaming,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    let mut y = 0;
    let mut sy = 0;
    let mut ly = 0;
    let mut ioy = 0;
    let mut gty = 0;
    let mut hy = EnumMap::default();
    let mut ioty = EnumMap::default();
    let dev_has_hbm = grids.first().unwrap().has_hbm;
    let mut asy = if dev_has_hbm { 2 } else { 0 };
    let mut gtby = 0;
    let hcx = enum_map! {
        k => if grids.values().any(|grid| {
            grid.cols_hard[0].regs.values().any(|&x| x == k || (k == HardRowKind::Hdio && x == HardRowKind::HdioAms))
        }) {
            1
        } else {
            0
        }
    };
    let mgrid = grids[grid_master];
    let mrcfg = mgrid
        .cols_hard
        .iter()
        .find_map(|hc| hc.regs.iter().find(|&(_, &hcr)| hcr == HardRowKind::Cfg))
        .unwrap()
        .0;
    let mut bank = (25
        - mrcfg.to_idx()
        - grids
            .iter()
            .filter_map(|(die, grid)| {
                if die < grid_master {
                    Some(grid.regs)
                } else {
                    None
                }
            })
            .sum::<usize>()) as u32;
    let mut has_pcie_cfg = false;
    let mut io = vec![];
    let mut cfg_io = EntityVec::new();
    let mut gt = vec![];
    for (_, grid) in grids {
        let (_, die) = egrid.add_die(grid.columns.len(), grid.regs * 60);

        let mut expander = DieExpander {
            grid,
            disabled,
            die,
            db,
            ylut: EntityVec::new(),
            sylut: EntityVec::new(),
            lylut: EntityVec::new(),
            asxlut: EntityVec::new(),
            asylut: EntityVec::new(),
            ioxlut: EntityPartVec::new(),
            ioylut: EntityVec::new(),
            brxlut: EntityVec::new(),
            gtbxlut: EntityVec::new(),
            gtbylut: EntityVec::new(),
            vsxlut: EntityVec::new(),
            cmtxlut: EntityVec::new(),
            gtylut: EntityVec::new(),
            gtxlut: EntityVec::new(),
            bankxlut: EntityPartVec::new(),
            bankylut: EntityVec::new(),
            dev_has_hbm,
            hylut: EnumMap::default(),
            iotxlut: EnumMap::default(),
            iotylut: EnumMap::default(),
            naming,
            io: &mut io,
            cfg_io: BiHashMap::new(),
            gt: &mut gt,
            col_cfg_io: None,
            reg_cfg_io: None,
            is_cfg_io_hrio: false,
            has_mcap: false,
        };

        y = expander.fill_ylut(y);
        sy = expander.fill_sylut(sy);
        ly = expander.fill_lylut(ly);
        expander.fill_hylut(&mut hy);
        expander.fill_iotylut(&mut ioty);
        expander.fill_asxlut();
        asy = expander.fill_asylut(asy);
        expander.fill_ioxlut();
        ioy = expander.fill_ioylut(ioy);
        expander.fill_brxlut();
        gtby = expander.fill_gtbylut(gtby);
        expander.fill_cmtxlut();
        expander.fill_gtxlut();
        expander.fill_iotxlut();
        gty = expander.fill_gtylut(gty);
        bank = expander.fill_bankylut(bank);

        expander.fill_int();
        expander.fill_io_pass();
        expander.fill_ps();
        expander.fill_term();
        expander.die.fill_main_passes();
        expander.fill_clb();
        expander.fill_bram();
        expander.fill_dsp();
        expander.fill_uram();
        expander.fill_fe();
        expander.fill_dfe();
        expander.fill_hard(&hcx, &mut has_pcie_cfg);
        expander.fill_io();
        expander.fill_clkroot();

        cfg_io.push(expander.cfg_io);
    }

    let (hroute_src, hdistr_src) = fill_clk_src(&grids[grid_master].columns);
    let is_cut = disabled
        .iter()
        .any(|x| matches!(x, DisabledPart::Region(..)));
    let is_cut_d = disabled.contains(&DisabledPart::Region(
        DieId::from_idx(0),
        RegId::from_idx(0),
    ));

    ExpandedDevice {
        kind: grids[grid_master].kind,
        grids: grids.clone(),
        grid_master,
        egrid,
        disabled: disabled.clone(),
        naming,
        hroute_src,
        hdistr_src,
        has_pcie_cfg,
        is_cut,
        is_cut_d,
        io,
        cfg_io,
        gt,
    }
}
