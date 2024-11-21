#![allow(clippy::type_complexity)]

use bimap::BiHashMap;
use enum_map::{enum_map, EnumMap};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, RowId};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::expanded::{ClkSrc, ExpandedDevice, GtCoord, HdioCoord, HpioCoord, IoCoord};
use crate::grid::{
    CleMKind, ColSide, Column, ColumnKindLeft, ColumnKindRight, DisabledPart, DspKind, Grid,
    GridKind, HardRowKind, HdioIobId, HpioIobId, Interposer, IoRowKind, RegId,
};

use crate::bond::SharedCfgPin;

struct DieExpander<'a, 'b, 'c> {
    grid: &'b Grid,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    io: &'c mut Vec<IoCoord>,
    gt: &'c mut Vec<GtCoord>,
}

impl DieExpander<'_, '_, '_> {
    fn fill_int(&mut self) {
        for (col, &cd) in &self.grid.columns {
            for row in self.die.rows() {
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.grid.row_to_reg(row),
                )) {
                    continue;
                }
                if let Some(ps) = self.grid.ps {
                    if col < ps.col && row.to_idx() < ps.height() {
                        continue;
                    }
                }
                self.die.fill_tile((col, row), "INT");
                if row.to_idx() % 60 == 30 {
                    self.die
                        .add_xnode((col, row), "RCLK_INT", &[(col, row), (col, row - 1)]);
                }
                match cd.l {
                    ColumnKindLeft::CleL | ColumnKindLeft::CleM(_) => (),
                    ColumnKindLeft::Bram(_) | ColumnKindLeft::Uram => {
                        self.die.add_xnode((col, row), "INTF.W", &[(col, row)]);
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
                            (GridKind::UltrascalePlus, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                self.die.add_xnode((col, row), "INTF.W.IO", &[(col, row)]);
                            }
                            _ => {
                                self.die
                                    .add_xnode((col, row), "INTF.W.DELAY", &[(col, row)]);
                            }
                        }
                    }
                    ColumnKindLeft::Hard(_, _)
                    | ColumnKindLeft::Sdfec
                    | ColumnKindLeft::DfeC
                    | ColumnKindLeft::DfeDF
                    | ColumnKindLeft::DfeE => {
                        self.die
                            .add_xnode((col, row), "INTF.W.DELAY", &[(col, row)]);
                    }
                }
                match cd.r {
                    ColumnKindRight::CleL(_) => (),
                    ColumnKindRight::Dsp(_) | ColumnKindRight::Uram => {
                        self.die.add_xnode((col, row), "INTF.E", &[(col, row)]);
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
                                self.die.add_xnode((col, row), "INTF.E.IO", &[(col, row)]);
                            }
                            _ => {
                                self.die
                                    .add_xnode((col, row), "INTF.E.DELAY", &[(col, row)]);
                            }
                        }
                    }
                    ColumnKindRight::Hard(_, _)
                    | ColumnKindRight::DfeB
                    | ColumnKindRight::DfeC
                    | ColumnKindRight::DfeDF
                    | ColumnKindRight::DfeE => {
                        self.die
                            .add_xnode((col, row), "INTF.E.DELAY", &[(col, row)]);
                    }
                }
            }
        }
    }

    fn fill_io_pass(&mut self) {
        if self.grid.kind == GridKind::UltrascalePlus {
            for (col, &cd) in &self.grid.columns {
                if matches!(cd.l, ColumnKindLeft::Io(_)) && col.to_idx() != 0 {
                    for row in self.die.rows() {
                        if self.disabled.contains(&DisabledPart::Region(
                            self.die.die,
                            self.grid.row_to_reg(row),
                        )) {
                            continue;
                        }
                        self.die
                            .fill_term_pair((col - 1, row), (col, row), "IO.E", "IO.W");
                    }
                }
            }
        }
    }

    fn fill_ps(&mut self) {
        if let Some(ps) = self.grid.ps {
            let height = ps.height();
            let width = ps.col.to_idx();
            if height != self.grid.regs * 60 {
                let row_t = RowId::from_idx(height);
                for dx in 0..width {
                    let col = ColId::from_idx(dx);
                    self.die.fill_term((col, row_t), "TERM.S");
                }
            }
            for dy in 0..height {
                let row = RowId::from_idx(dy);
                self.die.fill_term((ps.col, row), "TERM.W");
                self.die
                    .add_xnode((ps.col, row), "INTF.W.IO", &[(ps.col, row)]);
                if dy % 60 == 30 {
                    self.die
                        .add_xnode((ps.col, row), "RCLK_PS", &[(ps.col, row)]);
                }
            }
            let row = RowId::from_idx(if ps.has_vcu { 60 } else { 0 });
            let crds: [_; 180] = core::array::from_fn(|i| (ps.col, row + i));
            self.die.add_xnode((ps.col, row), "PS", &crds);
            if ps.has_vcu {
                let row = RowId::from_idx(0);
                let crds: [_; 60] = core::array::from_fn(|i| (ps.col, row + i));
                self.die.add_xnode((ps.col, row), "VCU", &crds);
            }
        }
    }

    fn fill_term(&mut self) {
        let col_l = self.die.cols().next().unwrap();
        let col_r = self.die.cols().next_back().unwrap();
        let row_b = self.die.rows().next().unwrap();
        let row_t = self.die.rows().next_back().unwrap();
        for col in self.die.cols() {
            if !self.die[(col, row_b)].nodes.is_empty() {
                self.die.fill_term((col, row_b), "TERM.S");
            }
            if !self.die[(col, row_t)].nodes.is_empty() {
                self.die.fill_term((col, row_t), "TERM.N");
            }
        }
        for row in self.die.rows() {
            if !self.die[(col_l, row)].nodes.is_empty() {
                self.die.fill_term((col_l, row), "TERM.W");
            }
            if !self.die[(col_r, row)].nodes.is_empty() {
                self.die.fill_term((col_r, row), "TERM.E");
            }
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in &self.grid.columns {
            if let Some(kind) = match cd.l {
                ColumnKindLeft::CleL => Some("CLEL_L"),
                ColumnKindLeft::CleM(_) => Some("CLEM"),
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
                    if cd.l == ColumnKindLeft::CleM(CleMKind::Laguna)
                        && self.grid.is_laguna_row(row)
                    {
                        self.die.add_xnode((col, row), "LAGUNA", &[(col, row)]);
                    } else {
                        self.die.add_xnode((col, row), kind, &[(col, row)]);
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
                        self.die
                            .add_xnode((col, row), "RCLK_HROUTE_SPLITTER_L.CLE", &[]);
                    } else if cd.l == ColumnKindLeft::CleM(CleMKind::Laguna)
                        && self.grid.is_laguna_row(row)
                    {
                        if self.grid.kind == GridKind::Ultrascale {
                            continue;
                        }
                        self.die
                            .add_xnode((col, row), "RCLK_V_SINGLE_L.LAG", &[(col, row)]);
                    } else {
                        self.die
                            .add_xnode((col, row), "RCLK_V_SINGLE_L.CLE", &[(col, row)]);
                    }
                }
            }
            if matches!(cd.r, ColumnKindRight::CleL(_)) {
                for row in self.die.rows() {
                    let tile = &mut self.die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    self.die.add_xnode((col, row), "CLEL_R", &[(col, row)]);
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
                    self.die
                        .add_xnode((col, row), "RCLK_V_SINGLE_R.CLE", &[(col, row)]);
                }
            }
        }
    }

    fn fill_bram(&mut self) {
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
                self.die.add_xnode(
                    (col, row),
                    "BRAM",
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                if row.to_idx() % 60 == 30 {
                    self.die.add_xnode((col, row), "HARD_SYNC", &[(col, row)]);

                    if self.grid.kind == GridKind::Ultrascale {
                        self.die
                            .add_xnode((col, row), "RCLK_V_DOUBLE_L", &[(col, row)]);
                    } else {
                        self.die
                            .add_xnode((col, row), "RCLK_V_QUAD_L.BRAM", &[(col, row)]);
                    }
                }
            }
        }
    }

    fn fill_dsp(&mut self) {
        for (col, &cd) in &self.grid.columns {
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
                    if col < self.grid.cols_io[1].col
                        && self.disabled.contains(&DisabledPart::HbmLeft)
                    {
                        continue;
                    }
                    let crds: [_; 15] = core::array::from_fn(|i| (col, row + i));
                    self.die.add_xnode((col, row), "BLI", &crds);
                } else {
                    let tile = &mut self.die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    self.die.add_xnode(
                        (col, row),
                        "DSP",
                        &[
                            (col, row),
                            (col, row + 1),
                            (col, row + 2),
                            (col, row + 3),
                            (col, row + 4),
                        ],
                    );
                }
            }
            for reg in self.grid.regs() {
                let row = self.grid.row_reg_rclk(reg);
                let tile = &mut self.die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                if matches!(cd.r, ColumnKindRight::Dsp(DspKind::ClkBuf)) {
                    self.die.add_xnode((col, row), "RCLK_SPLITTER", &[]);
                } else {
                    self.die
                        .add_xnode((col, row), "RCLK_V_DOUBLE_R", &[(col, row)]);
                }
            }
        }
    }

    fn fill_uram(&mut self) {
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
                let mut crds = vec![];
                for dy in 0..15 {
                    crds.push((col, row + dy));
                }
                for dy in 0..15 {
                    crds.push((col + 1, row + dy));
                }
                self.die.add_xnode((col, row), "URAM", &crds);
                if row.to_idx() % 60 == 30 {
                    self.die
                        .add_xnode((col + 1, row), "RCLK_V_QUAD_L.URAM", &[(col + 1, row)]);
                }
            }
        }
    }

    fn fill_hard_single(&mut self, col: ColId, reg: RegId, kind: HardRowKind) {
        let row = self.grid.row_reg_bot(reg);
        if self
            .disabled
            .contains(&DisabledPart::Region(self.die.die, reg))
        {
            return;
        }

        let die = self.die.die;
        let nk = match kind {
            HardRowKind::None => return,
            HardRowKind::Hdio | HardRowKind::HdioAms => {
                let col = col - 1;
                for (i, nk) in ["HDIO_BOT", "HDIO_TOP"].into_iter().enumerate() {
                    let row = row + i * 30;
                    let crds: [_; 30] = core::array::from_fn(|i| (col, row + i));
                    self.die.add_xnode((col, row), nk, &crds);
                    for j in 0..12 {
                        let idx = i * 12 + j;
                        self.io.push(IoCoord::Hdio(HdioCoord {
                            die,
                            col,
                            reg,
                            iob: HdioIobId::from_idx(idx),
                        }));
                    }
                }
                let crds: [_; 60] = core::array::from_fn(|i| (col, row + i));
                self.die.add_xnode((col, row + 30), "RCLK_HDIO", &crds);
                return;
            }
            HardRowKind::Cfg => "CFG",
            HardRowKind::Ams => {
                let crds: [_; 60] = core::array::from_fn(|i| {
                    if i < 30 {
                        (col - 1, row + i)
                    } else {
                        (col, row + (i - 30))
                    }
                });
                self.die.add_xnode((col, row), "CFGIO", &crds);
                let row = row + 30;
                self.die
                    .add_xnode((col, row), "RCLK_HROUTE_SPLITTER_L.HARD", &[]);
                let crds: [_; 60] = core::array::from_fn(|i| {
                    if i < 30 {
                        (col - 1, row + i)
                    } else {
                        (col, row + (i - 30))
                    }
                });
                self.die.add_xnode((col, row), "AMS", &crds);
                return;
            }
            HardRowKind::Pcie => {
                if self.grid.kind == GridKind::Ultrascale {
                    "PCIE"
                } else {
                    "PCIE4"
                }
            }
            HardRowKind::PciePlus => "PCIE4C",
            HardRowKind::Cmac => "CMAC",
            HardRowKind::Ilkn => "ILKN",
            HardRowKind::DfeA => "DFE_A",
            HardRowKind::DfeG => "DFE_G",
        };
        self.die
            .add_xnode((col, row + 30), "RCLK_HROUTE_SPLITTER_L.HARD", &[]);
        let crds: [_; 120] = core::array::from_fn(|i| {
            if i < 60 {
                (col - 1, row + i)
            } else {
                (col, row + (i - 60))
            }
        });
        self.die.add_xnode((col, row), nk, &crds);
    }

    fn fill_hard(&mut self, has_pcie_cfg: &mut bool) {
        for hc in &self.grid.cols_hard {
            let is_cfg = hc.regs.values().any(|&x| x == HardRowKind::Cfg);
            for reg in self.grid.regs() {
                let kind = hc.regs[reg];
                if kind == HardRowKind::Cfg
                    && reg.to_idx() != 0
                    && matches!(hc.regs[reg - 1], HardRowKind::Pcie | HardRowKind::PciePlus)
                {
                    *has_pcie_cfg = true;
                }
                self.fill_hard_single(hc.col, reg, kind);
            }
            if is_cfg && self.grid.has_hbm {
                self.die
                    .add_xnode((hc.col, RowId::from_idx(0)), "HBM_ABUS_SWITCH", &[]);
            }
        }
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
                        for idx in 0..52 {
                            self.io.push(IoCoord::Hpio(HpioCoord {
                                die,
                                col: ioc.col,
                                side: ioc.side,
                                reg,
                                iob: HpioIobId::from_idx(idx),
                            }));
                        }
                        if self.grid.kind == GridKind::Ultrascale {
                            self.die.add_xnode((ioc.col, row), "XIPHY", &crds);
                            if kind == IoRowKind::Hpio {
                                self.die.add_xnode((ioc.col, row), "RCLK_HPIO", &crds);
                                for i in 0..2 {
                                    let row = row - 30 + i * 30;
                                    self.die.add_xnode(
                                        (ioc.col, row),
                                        "HPIO",
                                        &crds[i * 30..i * 30 + 30],
                                    );
                                }
                            } else {
                                self.die.add_xnode((ioc.col, row), "RCLK_HRIO", &[]);
                                for i in 0..2 {
                                    let row = row - 30 + i * 30;
                                    self.die.add_xnode(
                                        (ioc.col, row),
                                        "HRIO",
                                        &crds[i * 30..i * 30 + 30],
                                    );
                                }
                            }
                        } else {
                            let is_hbm = self.grid.has_hbm && reg.to_idx() == 0;
                            let kind = if ioc.side == ColSide::Right {
                                "CMT_R"
                            } else if is_hbm {
                                "CMT_L_HBM"
                            } else {
                                "CMT_L"
                            };
                            self.die.add_xnode((ioc.col, row), kind, &crds);

                            let kind = if ioc.side == ColSide::Right {
                                "RCLK_XIPHY_R"
                            } else {
                                "RCLK_XIPHY_L"
                            };
                            self.die.add_xnode((ioc.col, row), kind, &[]);

                            for i in 0..4 {
                                let kind = if ioc.side == ColSide::Right {
                                    "XIPHY_R"
                                } else {
                                    "XIPHY_L"
                                };
                                let row = self.grid.row_reg_bot(reg) + i * 15;
                                self.die.add_xnode(
                                    (ioc.col, row),
                                    kind,
                                    &crds[i * 15..i * 15 + 15],
                                );
                            }

                            for i in 0..2 {
                                let kind = if ioc.side == ColSide::Right {
                                    "HPIO_R"
                                } else {
                                    "HPIO_L"
                                };
                                let row = self.grid.row_reg_bot(reg) + i * 30;
                                self.die.add_xnode(
                                    (ioc.col, row),
                                    kind,
                                    &crds[i * 30..i * 30 + 30],
                                );
                            }

                            let kind = if ioc.side == ColSide::Left {
                                "RCLK_HPIO_L"
                            } else {
                                "RCLK_HPIO_R"
                            };
                            self.die.add_xnode((ioc.col, row), kind, &crds);
                        }
                    }
                    _ => {
                        let row = self.grid.row_reg_rclk(reg);
                        let crds: [_; 60] = core::array::from_fn(|i| (ioc.col, row - 30 + i));
                        let nk = match (kind, ioc.side) {
                            (IoRowKind::Gth, ColSide::Left) => "GTH_L",
                            (IoRowKind::Gth, ColSide::Right) => "GTH_R",
                            (IoRowKind::Gty, ColSide::Left) => "GTY_L",
                            (IoRowKind::Gty, ColSide::Right) => "GTY_R",
                            (IoRowKind::Gtf, ColSide::Left) => "GTF_L",
                            (IoRowKind::Gtf, ColSide::Right) => "GTF_R",
                            (IoRowKind::Gtm, ColSide::Left) => "GTM_L",
                            (IoRowKind::Gtm, ColSide::Right) => "GTM_R",
                            (IoRowKind::HsAdc, ColSide::Right) => "HSADC_R",
                            (IoRowKind::HsDac, ColSide::Right) => "HSDAC_R",
                            (IoRowKind::RfAdc, ColSide::Right) => "RFADC_R",
                            (IoRowKind::RfDac, ColSide::Right) => "RFDAC_R",
                            _ => unreachable!(),
                        };
                        self.die.add_xnode((ioc.col, row), nk, &crds);
                        self.gt.push(GtCoord {
                            die,
                            col: ioc.col,
                            side: ioc.side,
                            reg,
                        });
                    }
                }
            }
        }
    }

    fn fill_fe(&mut self) {
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
                    let crds: [_; 60] = core::array::from_fn(|i| (col, row + i));
                    self.die.add_xnode((col, row), "FE", &crds);
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
                if matches!(cd.r, ColumnKindRight::DfeB | ColumnKindRight::DfeE) {
                    self.die.add_xnode(
                        (if bi { col + 1 } else { col }, row + 30),
                        if bi {
                            "RCLK_HROUTE_SPLITTER_L.HARD"
                        } else {
                            "RCLK_HROUTE_SPLITTER_R.HARD"
                        },
                        &[],
                    );
                }
                let crds: [_; 120] = core::array::from_fn(|i| {
                    if i < 60 {
                        (col, row + i)
                    } else {
                        (col + 1, row + (i - 60))
                    }
                });
                self.die.add_xnode(
                    (if bi { col + 1 } else { col }, row),
                    kind,
                    if bi { &crds } else { &crds[..60] },
                );
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
    if matches!(columns.last().unwrap().r, ColumnKindRight::Hard(_, _)) {
        hd = ClkSrc::RightHdio(columns.last_id().unwrap());
        hr = ClkSrc::RightHdio(columns.last_id().unwrap());
    }
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
            | ColumnKindLeft::Hard(_, _)
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
    interposer: &'a Interposer,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    let mgrid = grids[interposer.primary];
    let mut has_pcie_cfg = false;
    let mut io = vec![];
    let mut gt = vec![];
    for (_, grid) in grids {
        let (_, die) = egrid.add_die(grid.columns.len(), grid.regs * 60);

        let mut expander = DieExpander {
            grid,
            disabled,
            die,
            io: &mut io,
            gt: &mut gt,
        };
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
        expander.fill_hard(&mut has_pcie_cfg);
        expander.fill_io();
        expander.fill_clkroot();
    }

    let (hroute_src, hdistr_src) = fill_clk_src(&grids[interposer.primary].columns);
    let is_cut = disabled
        .iter()
        .any(|x| matches!(x, DisabledPart::Region(..)));
    let is_cut_d = disabled.contains(&DisabledPart::Region(
        DieId::from_idx(0),
        RegId::from_idx(0),
    ));

    egrid.finish();

    let mut col_cfg_io = None;
    for (col, &cd) in &mgrid.columns {
        if let ColumnKindLeft::Io(_) = cd.l {
            col_cfg_io = Some((col, ColSide::Left));
        }
        if let ColumnKindRight::Io(_) = cd.r {
            if col_cfg_io.is_none() {
                col_cfg_io = Some((col, ColSide::Right));
            }
        }
    }
    let col_cfg_io = col_cfg_io.unwrap();

    let mut ioxlut = EntityPartVec::new();
    let mut bankxlut = EntityPartVec::new();
    let mut iox = 0;
    for (col, &cd) in &mgrid.columns {
        if let ColumnKindLeft::Io(_) = cd.l {
            ioxlut.insert(col, iox);
            iox += 1;
        }
        match cd.r {
            ColumnKindRight::Io(_) => {
                ioxlut.insert(col, iox);
                iox += 1;
            }
            ColumnKindRight::Hard(_, idx) => {
                let regs = &mgrid.cols_hard[idx].regs;
                if regs
                    .values()
                    .any(|x| matches!(x, HardRowKind::Hdio | HardRowKind::HdioAms))
                {
                    ioxlut.insert(col, iox);
                    iox += 1;
                }
            }
            _ => (),
        }
    }
    let iox_cfg = ioxlut[col_cfg_io.0];
    for (col, &iox) in &ioxlut {
        let mut bank = (40 + iox * 20 - iox_cfg * 20) as u32;
        if col.to_idx() == 0
            && iox != iox_cfg
            && mgrid.kind == GridKind::UltrascalePlus
            && mgrid.cols_hard.len() == 1
        {
            bank -= 20;
        }
        bankxlut.insert(col, bank);
    }

    let mut bank = (25
        - mgrid.reg_cfg().to_idx()
        - grids
            .iter()
            .filter_map(|(die, grid)| {
                if die < interposer.primary {
                    Some(grid.regs)
                } else {
                    None
                }
            })
            .sum::<usize>()) as u32;
    let mut bankylut = EntityVec::new();
    for &grid in grids.values() {
        bankylut.push(bank);
        bank += grid.regs as u32;
    }

    let mut cfg_io = EntityVec::new();
    for (die, &grid) in grids {
        let mut die_cfg_io = BiHashMap::new();
        let iocol = grid
            .cols_io
            .iter()
            .find(|iocol| (iocol.col, iocol.side) == col_cfg_io)
            .unwrap();
        if matches!(
            iocol.regs[grid.reg_cfg()],
            IoRowKind::Hpio | IoRowKind::Hrio
        ) {
            for idx in 0..52 {
                if let Some(cfg) = if !grid.is_alt_cfg {
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
                        25 => Some(if grid.kind == GridKind::Ultrascale {
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
                    die_cfg_io.insert(
                        cfg,
                        IoCoord::Hpio(HpioCoord {
                            die,
                            col: iocol.col,
                            side: iocol.side,
                            reg: grid.reg_cfg(),
                            iob: HpioIobId::from_idx(idx),
                        }),
                    );
                }
            }
        }
        cfg_io.push(die_cfg_io);
    }

    ExpandedDevice {
        kind: grids[interposer.primary].kind,
        grids: grids.clone(),
        interposer,
        egrid,
        disabled: disabled.clone(),
        hroute_src,
        hdistr_src,
        has_pcie_cfg,
        is_cut,
        is_cut_d,
        io,
        cfg_io,
        gt,
        col_cfg_io,
        bankxlut,
        bankylut,
    }
}
