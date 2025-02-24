#![allow(clippy::type_complexity)]

use bimap::BiHashMap;
use prjcombine_interconnect::db::{Dir, IntDb};
use prjcombine_interconnect::grid::{
    ColId, DieId, ExpandedDieRefMut, ExpandedGrid, RowId, TileIobId,
};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{
    Chip, ChipKind, CleMKind, Column, ColumnKind, DisabledPart, DspKind, HardKind, HardRowKind,
    Interposer, IoRowKind, RegId,
};
use crate::expanded::{ClkSrc, ExpandedDevice, GtCoord, HdioCoord, HpioCoord, IoCoord};

use crate::bond::SharedCfgPin;

struct DieExpander<'a, 'b, 'c> {
    chip: &'b Chip,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    io: &'c mut Vec<IoCoord>,
    gt: &'c mut Vec<GtCoord>,
}

impl DieExpander<'_, '_, '_> {
    fn fill_int(&mut self) {
        for (col, &cd) in &self.chip.columns {
            for row in self.die.rows() {
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
                    continue;
                }
                if self.chip.in_int_hole(col, row) {
                    continue;
                }
                if self.chip.col_side(col) == Dir::W {
                    self.die
                        .add_xnode((col, row), "INT", &[(col, row), (col + 1, row)]);
                    if row.to_idx() % 60 == 30 {
                        self.die.add_xnode(
                            (col, row),
                            "RCLK_INT",
                            &[
                                (col, row),
                                (col + 1, row),
                                (col, row - 1),
                                (col + 1, row - 1),
                            ],
                        );
                    }
                }
                match cd.kind {
                    ColumnKind::CleL(_) | ColumnKind::CleM(_) => (),
                    ColumnKind::Bram(_)
                    | ColumnKind::Dsp(_)
                    | ColumnKind::Uram
                    | ColumnKind::ContUram => {
                        self.die.add_xnode((col, row), "INTF", &[(col, row)]);
                    }
                    ColumnKind::Gt(_) | ColumnKind::Io(_) => {
                        let iocol = self.chip.cols_io.iter().find(|x| x.col == col).unwrap();
                        let rk = iocol.regs[self.chip.row_to_reg(row)];
                        match (self.chip.kind, rk) {
                            (_, IoRowKind::None) => (),

                            (
                                ChipKind::UltrascalePlus,
                                IoRowKind::Hpio | IoRowKind::Hrio | IoRowKind::HdioLc,
                            ) => {
                                self.die.add_xnode((col, row), "INTF.IO", &[(col, row)]);
                            }
                            _ => {
                                self.die.add_xnode((col, row), "INTF.DELAY", &[(col, row)]);
                            }
                        }
                    }
                    ColumnKind::Hard(_, _)
                    | ColumnKind::DfeC
                    | ColumnKind::DfeDF
                    | ColumnKind::DfeE
                    | ColumnKind::ContHard
                    | ColumnKind::Sdfec
                    | ColumnKind::DfeB => {
                        self.die.add_xnode((col, row), "INTF.DELAY", &[(col, row)]);
                    }
                }
            }
        }
    }

    fn fill_ps(&mut self) {
        if let Some(ps) = self.chip.ps {
            let height = ps.height();
            let width = ps.col.to_idx();
            if height != self.chip.regs * 60 {
                let row_t = RowId::from_idx(height);
                for dx in 0..width {
                    let col = ColId::from_idx(dx);
                    self.die.fill_term(
                        (col, row_t),
                        &format!("TERM.S{side}", side = col.to_idx() % 2),
                    );
                }
            }
            for dy in 0..height {
                let row = RowId::from_idx(dy);
                self.die.fill_term((ps.col, row), "TERM.W");
                self.die.fill_term((ps.col, row), "TERM.LW");
                self.die
                    .add_xnode((ps.col, row), "INTF.IO", &[(ps.col, row)]);
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
            let side = col.to_idx() % 2;
            if !self.chip.in_int_hole(col, row_b)
                && !self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row_b),
                ))
            {
                self.die.fill_term((col, row_b), &format!("TERM.S{side}"));
            }
            if !self.chip.in_int_hole(col, row_t)
                && !self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row_t),
                ))
            {
                self.die.fill_term((col, row_t), &format!("TERM.N{side}"));
            }
        }
        for row in self.die.rows() {
            if !self.chip.in_int_hole(col_l, row) {
                self.die.fill_term((col_l, row), "TERM.W");
                self.die.fill_term((col_l, row), "TERM.LW");
            }
            if !self.chip.in_int_hole(col_r, row) {
                self.die.fill_term((col_r, row), "TERM.E");
                self.die.fill_term((col_r - 1, row), "TERM.LE");
            }
        }
    }

    fn fill_main_passes(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if col == self.chip.columns.first_id().unwrap() {
                continue;
            }
            let is_io_mid = matches!(cd.kind, ColumnKind::Io(_))
                && col != self.chip.columns.last_id().unwrap()
                && self.chip.kind == ChipKind::UltrascalePlus;
            for row in self.die.rows() {
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
                    continue;
                }
                if self.chip.in_int_hole(col, row) || self.chip.in_int_hole(col - 1, row) {
                    continue;
                }
                if is_io_mid {
                    self.die
                        .fill_term_pair((col - 1, row), (col, row), "IO.E", "IO.W");
                    self.die
                        .fill_term_pair((col - 2, row), (col, row), "IO.LE", "IO.LW");
                } else {
                    self.die
                        .fill_term_pair((col - 1, row), (col, row), "MAIN.E", "MAIN.W");
                    if self.chip.col_side(col) == Dir::W {
                        self.die
                            .fill_term_pair((col - 2, row), (col, row), "MAIN.LE", "MAIN.LW");
                    }
                }
            }
        }
        for col in self.die.cols() {
            for row in self.die.rows() {
                if row == self.chip.rows().next_back().unwrap() {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row + 1),
                )) {
                    continue;
                }
                if self.chip.in_int_hole(col, row) || self.chip.in_int_hole(col, row + 1) {
                    continue;
                }
                self.die
                    .fill_term_pair((col, row), (col, row + 1), "MAIN.N", "MAIN.S");
            }
        }
    }

    fn fill_clb(&mut self) {
        for (col, &cd) in &self.chip.columns {
            let Some(kind) = (match cd.kind {
                ColumnKind::CleL(_) => Some("CLEL"),
                ColumnKind::CleM(_) => Some("CLEM"),
                _ => None,
            }) else {
                continue;
            };
            for row in self.die.rows() {
                if self.chip.in_site_hole(col, row) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
                    continue;
                }
                if cd.kind == ColumnKind::CleM(CleMKind::Laguna) && self.chip.is_laguna_row(row) {
                    self.die.add_xnode((col, row), "LAGUNA", &[(col, row)]);
                } else {
                    self.die.add_xnode((col, row), kind, &[(col, row)]);
                }
            }
            for reg in self.chip.regs() {
                let row = self.chip.row_reg_rclk(reg);
                if self.chip.in_site_hole(col, row) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
                    continue;
                }
                if matches!(cd.kind, ColumnKind::CleM(CleMKind::ClkBuf)) {
                    self.die
                        .add_xnode((col, row), "RCLK_HROUTE_SPLITTER.CLE", &[]);
                } else if cd.kind == ColumnKind::CleM(CleMKind::Laguna)
                    && self.chip.is_laguna_row(row)
                {
                    if self.chip.kind == ChipKind::Ultrascale {
                        continue;
                    }
                    self.die
                        .add_xnode((col, row), "RCLK_V_SINGLE.LAG", &[(col, row)]);
                } else if self.chip.col_side(col) == Dir::W
                    || self.chip.kind != ChipKind::UltrascalePlus
                {
                    self.die
                        .add_xnode((col, row), "RCLK_V_SINGLE.CLE", &[(col, row)]);
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd.kind, ColumnKind::Bram(_)) {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                if self.chip.in_site_hole(col, row) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
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

                    if self.chip.kind == ChipKind::Ultrascale {
                        self.die
                            .add_xnode((col, row), "RCLK_V_DOUBLE.BRAM", &[(col, row)]);
                    } else {
                        self.die
                            .add_xnode((col, row), "RCLK_V_QUAD.BRAM", &[(col, row)]);
                    }
                }
            }
        }
    }

    fn fill_dsp(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd.kind, ColumnKind::Dsp(_)) {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                if self.chip.in_int_hole(col, row) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
                    continue;
                }
                if self.chip.has_hbm && row.to_idx() < 15 {
                    if row.to_idx() != 0 {
                        continue;
                    }
                    if col < self.chip.cols_io[1].col
                        && self.disabled.contains(&DisabledPart::HbmLeft)
                    {
                        continue;
                    }
                    let crds: [_; 15] = core::array::from_fn(|i| (col, row + i));
                    self.die.add_xnode((col, row), "BLI", &crds);
                } else {
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
            for reg in self.chip.regs() {
                let row = self.chip.row_reg_rclk(reg);
                if self.chip.in_int_hole(col, row) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
                    continue;
                }
                if matches!(cd.kind, ColumnKind::Dsp(DspKind::ClkBuf)) {
                    self.die.add_xnode((col, row), "RCLK_SPLITTER", &[]);
                } else {
                    self.die
                        .add_xnode((col, row), "RCLK_V_DOUBLE.DSP", &[(col, row)]);
                }
            }
        }
    }

    fn fill_uram(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd.kind != ColumnKind::Uram {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 15 != 0 {
                    continue;
                }
                if self.chip.in_int_hole(col, row) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die.die,
                    self.chip.row_to_reg(row),
                )) {
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
                        .add_xnode((col + 1, row), "RCLK_V_QUAD.URAM", &[(col + 1, row)]);
                }
            }
        }
    }

    fn fill_hard_single(&mut self, col: ColId, reg: RegId, kind: HardRowKind) {
        let row = self.chip.row_reg_bot(reg);
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
                for (i, nk) in ["HDIO_S", "HDIO_N"].into_iter().enumerate() {
                    let row = row + i * 30;
                    let crds: [_; 30] = core::array::from_fn(|i| (col, row + i));
                    self.die.add_xnode((col, row), nk, &crds);
                    for j in 0..12 {
                        let idx = i * 12 + j;
                        self.io.push(IoCoord::Hdio(HdioCoord {
                            die,
                            col,
                            reg,
                            iob: TileIobId::from_idx(idx),
                        }));
                    }
                }
                let crds: [_; 60] = core::array::from_fn(|i| (col, row + i));
                self.die.add_xnode((col, row + 30), "RCLK_HDIO", &crds);
                return;
            }
            HardRowKind::HdioLc => {
                for (i, nk) in ["HDIOLC_S", "HDIOLC_N"].into_iter().enumerate() {
                    let row = row + i * 30;
                    let crds: [_; 30] = core::array::from_fn(|i| (col, row + i));
                    self.die.add_xnode((col, row), nk, &crds);
                    for j in 0..42 {
                        let idx = i * 42 + j;
                        self.io.push(IoCoord::HdioLc(HdioCoord {
                            die,
                            col,
                            reg,
                            iob: TileIobId::from_idx(idx),
                        }));
                    }
                }
                let crds: [_; 60] = core::array::from_fn(|i| (col, row + i));
                self.die.add_xnode((col, row + 30), "RCLK_HDIOLC", &crds);
                return;
            }
            HardRowKind::Cfg => {
                let kind = if self.chip.has_csec {
                    "CFG_CSEC"
                } else {
                    "CFG"
                };
                let crds: [_; 60] = core::array::from_fn(|i| (col, row + i));
                self.die.add_xnode((col, row), kind, &crds);
                self.die
                    .add_xnode((col, row + 30), "RCLK_HROUTE_SPLITTER.HARD", &[]);
                return;
            }
            HardRowKind::Ams => {
                let crds: [_; 30] = core::array::from_fn(|i| (col, row + i));
                self.die.add_xnode((col, row), "CFGIO", &crds);
                let row = row + 30;
                self.die
                    .add_xnode((col, row), "RCLK_HROUTE_SPLITTER.HARD", &[]);
                let crds: [_; 30] = core::array::from_fn(|i| (col, row + i));
                self.die.add_xnode((col, row), "AMS", &crds);
                return;
            }
            HardRowKind::Pcie => {
                if self.chip.kind == ChipKind::Ultrascale {
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
            .add_xnode((col, row + 30), "RCLK_HROUTE_SPLITTER.HARD", &[]);
        let crds: [_; 120] = core::array::from_fn(|i| {
            if i < 60 {
                (col, row + i)
            } else {
                (col + 1, row + (i - 60))
            }
        });
        self.die.add_xnode((col, row), nk, &crds);
    }

    fn fill_hard(&mut self, has_pcie_cfg: &mut bool) {
        for hc in &self.chip.cols_hard {
            let is_cfg = hc.regs.values().any(|&x| x == HardRowKind::Cfg);
            for reg in self.chip.regs() {
                let kind = hc.regs[reg];
                if kind == HardRowKind::Cfg
                    && reg.to_idx() != 0
                    && matches!(hc.regs[reg - 1], HardRowKind::Pcie | HardRowKind::PciePlus)
                {
                    *has_pcie_cfg = true;
                }
                self.fill_hard_single(hc.col, reg, kind);
            }
            if is_cfg && self.chip.has_hbm {
                self.die
                    .add_xnode((hc.col, RowId::from_idx(0)), "HBM_ABUS_SWITCH", &[]);
            }
        }
    }

    fn fill_io(&mut self) {
        let die = self.die.die;
        for ioc in &self.chip.cols_io {
            for reg in self.chip.regs() {
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
                        let row = self.chip.row_reg_rclk(reg);
                        let crds: [_; 60] = core::array::from_fn(|i| (ioc.col, row - 30 + i));
                        for idx in 0..52 {
                            self.io.push(IoCoord::Hpio(HpioCoord {
                                die,
                                col: ioc.col,
                                reg,
                                iob: TileIobId::from_idx(idx),
                            }));
                        }
                        if self.chip.kind == ChipKind::Ultrascale {
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
                            let is_hbm = self.chip.has_hbm && reg.to_idx() == 0;
                            let kind = if is_hbm { "CMT_HBM" } else { "CMT" };
                            self.die.add_xnode((ioc.col, row), kind, &crds);

                            self.die.add_xnode((ioc.col, row), "RCLK_XIPHY", &[]);

                            for i in 0..4 {
                                let row = self.chip.row_reg_bot(reg) + i * 15;
                                self.die.add_xnode(
                                    (ioc.col, row),
                                    "XIPHY",
                                    &crds[i * 15..i * 15 + 15],
                                );
                            }

                            for i in 0..2 {
                                let row = self.chip.row_reg_bot(reg) + i * 30;
                                self.die.add_xnode(
                                    (ioc.col, row),
                                    "HPIO",
                                    &crds[i * 30..i * 30 + 30],
                                );
                            }
                            self.die.add_xnode((ioc.col, row), "RCLK_HPIO", &crds);
                        }
                    }
                    IoRowKind::HdioLc => {
                        let col = ioc.col;
                        let row = self.chip.row_reg_rclk(reg);
                        for (i, nk) in ["HDIOLC_S", "HDIOLC_N"].into_iter().enumerate() {
                            let row = row - 30 + i * 30;
                            let crds: [_; 30] = core::array::from_fn(|i| (col, row + i));
                            self.die.add_xnode((col, row), nk, &crds);
                            for j in 0..42 {
                                let idx = i * 42 + j;
                                self.io.push(IoCoord::HdioLc(HdioCoord {
                                    die,
                                    col,
                                    reg,
                                    iob: TileIobId::from_idx(idx),
                                }));
                            }
                        }
                        let crds: [_; 60] = core::array::from_fn(|i| (ioc.col, row - 30 + i));
                        self.die.add_xnode((col, row), "CMT", &crds);
                        self.die.add_xnode((col, row), "RCLK_HDIOLC", &crds);
                    }
                    _ => {
                        let row = self.chip.row_reg_rclk(reg);
                        let crds: [_; 60] = core::array::from_fn(|i| (ioc.col, row - 30 + i));
                        let nk = match kind {
                            IoRowKind::Gth => "GTH",
                            IoRowKind::Gty => "GTY",
                            IoRowKind::Gtf => "GTF",
                            IoRowKind::Gtm => "GTM",
                            IoRowKind::HsAdc => "HSADC",
                            IoRowKind::HsDac => "HSDAC",
                            IoRowKind::RfAdc => "RFADC",
                            IoRowKind::RfDac => "RFDAC",
                            _ => unreachable!(),
                        };
                        self.die.add_xnode((ioc.col, row), nk, &crds);
                        self.gt.push(GtCoord {
                            die,
                            col: ioc.col,
                            reg,
                        });
                    }
                }
            }
        }
    }

    fn fill_fe(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd.kind != ColumnKind::Sdfec {
                continue;
            }
            for reg in self.chip.regs() {
                if self
                    .disabled
                    .contains(&DisabledPart::Region(self.die.die, reg))
                {
                    continue;
                }
                let row = self.chip.row_reg_bot(reg);
                let crds: [_; 60] = core::array::from_fn(|i| (col, row + i));
                self.die.add_xnode((col, row), "FE", &crds);
            }
        }
    }

    fn fill_dfe(&mut self) {
        for (col, &cd) in &self.chip.columns {
            let (kind, bi) = match cd.kind {
                ColumnKind::DfeB => ("DFE_B", false),
                ColumnKind::DfeC => ("DFE_C", true),
                ColumnKind::DfeDF => ("DFE_D", true),
                ColumnKind::DfeE => ("DFE_E", true),
                _ => continue,
            };
            for reg in self.chip.regs() {
                let row = self.chip.row_reg_bot(reg);
                let kind = if kind == "DFE_D" && reg.to_idx() == 2 {
                    "DFE_F"
                } else {
                    kind
                };
                if matches!(cd.kind, ColumnKind::DfeB | ColumnKind::DfeE) {
                    self.die
                        .add_xnode((col, row + 30), "RCLK_HROUTE_SPLITTER.HARD", &[]);
                }
                let crds: [_; 120] = core::array::from_fn(|i| {
                    if i < 60 {
                        (col, row + i)
                    } else {
                        (col + 1, row + (i - 60))
                    }
                });
                self.die
                    .add_xnode((col, row), kind, if bi { &crds } else { &crds[..60] });
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
) -> (EntityVec<ColId, ClkSrc>, EntityVec<ColId, ClkSrc>) {
    let mut hroute_src = vec![];
    let mut hdistr_src = vec![];
    let mut hd = ClkSrc::Gt(columns.last_id().unwrap());
    let mut hr = ClkSrc::Gt(columns.last_id().unwrap());
    if matches!(columns.last().unwrap().kind, ColumnKind::Hard(_, _)) {
        hd = ClkSrc::RightHdio(columns.last_id().unwrap());
        hr = ClkSrc::RightHdio(columns.last_id().unwrap());
    }
    for (col, &cd) in columns.iter().rev() {
        hroute_src.push(hr);
        hdistr_src.push(hd);
        match cd.kind {
            ColumnKind::CleM(CleMKind::ClkBuf)
            | ColumnKind::Hard(_, _)
            | ColumnKind::DfeE
            | ColumnKind::DfeB => {
                if col != columns.last_id().unwrap() {
                    hr = ClkSrc::RouteSplitter(col);
                }
            }
            ColumnKind::Dsp(DspKind::ClkBuf) => {
                hd = ClkSrc::DspSplitter(col);
                hr = ClkSrc::DspSplitter(col);
            }
            ColumnKind::Io(_) => {
                if col != columns.last_id().unwrap() {
                    hr = ClkSrc::Cmt(col);
                    hd = ClkSrc::Cmt(col);
                }
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
    chips: &EntityVec<DieId, &'a Chip>,
    interposer: &'a Interposer,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    let pchip = chips[interposer.primary];
    let mut has_pcie_cfg = false;
    let mut io = vec![];
    let mut gt = vec![];
    for (_, chip) in chips {
        let (_, die) = egrid.add_die(chip.columns.len(), chip.regs * 60);

        let mut expander = DieExpander {
            chip,
            disabled,
            die,
            io: &mut io,
            gt: &mut gt,
        };
        expander.fill_int();
        expander.fill_ps();
        expander.fill_term();
        expander.fill_main_passes();
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

    let (hroute_src, hdistr_src) = fill_clk_src(&chips[interposer.primary].columns);
    let is_cut = disabled
        .iter()
        .any(|x| matches!(x, DisabledPart::Region(..)));
    let is_cut_d = disabled.contains(&DisabledPart::Region(
        DieId::from_idx(0),
        RegId::from_idx(0),
    ));

    egrid.finish();

    let mut col_cfg_io = None;
    for (col, &cd) in &pchip.columns {
        if let ColumnKind::Io(_) = cd.kind {
            if col_cfg_io.is_none() || pchip.col_side(col) == Dir::W {
                col_cfg_io = Some(col);
            }
        }
        if let ColumnKind::Hard(HardKind::Term, idx) = cd.kind {
            let mut has_hdiolc = false;
            for chip in chips.values() {
                if chip.cols_hard[idx]
                    .regs
                    .values()
                    .any(|&kind| kind == HardRowKind::HdioLc)
                {
                    has_hdiolc = true;
                }
            }
            if has_hdiolc {
                col_cfg_io = Some(col);
            }
        }
    }
    let col_cfg_io = col_cfg_io.unwrap();

    let mut ioxlut = EntityPartVec::new();
    let mut bankxlut = EntityPartVec::new();
    let mut iox = 0;
    for (col, &cd) in &pchip.columns {
        match cd.kind {
            ColumnKind::Io(_) => {
                ioxlut.insert(col, iox);
                iox += 1;
            }
            ColumnKind::Hard(_, idx) => {
                let regs = &pchip.cols_hard[idx].regs;
                if regs.values().any(|x| {
                    matches!(
                        x,
                        HardRowKind::Hdio | HardRowKind::HdioAms | HardRowKind::HdioLc
                    )
                }) {
                    ioxlut.insert(col, iox);
                    iox += 1;
                }
            }
            _ => (),
        }
    }
    let iox_cfg = ioxlut[col_cfg_io];
    for (col, &iox) in &ioxlut {
        let mut bank = (40 + iox * 20 - iox_cfg * 20) as u32;
        if col.to_idx() == 0
            && iox != iox_cfg
            && pchip.kind == ChipKind::UltrascalePlus
            && pchip.cols_hard.len() == 1
        {
            bank -= 20;
        }
        bankxlut.insert(col, bank);
    }

    let mut bank = 0;
    let mut bankylut = EntityVec::new();
    let mut cfg_bank = None;
    for (die, &chip) in chips {
        let mut ylut = EntityPartVec::new();
        for reg in chip.regs() {
            let mut has_io = false;
            let mut has_hdiolc = false;
            for hcol in &chip.cols_hard {
                match hcol.regs[reg] {
                    HardRowKind::Cfg => {
                        if die == interposer.primary {
                            cfg_bank = Some(bank);
                        }
                    }
                    HardRowKind::Hdio | HardRowKind::HdioAms => {
                        has_io = true;
                    }
                    HardRowKind::HdioLc => {
                        has_hdiolc = true;
                    }
                    _ => (),
                }
            }
            for iocol in &chip.cols_io {
                match iocol.regs[reg] {
                    IoRowKind::Hpio | IoRowKind::Hrio => {
                        has_io = true;
                    }
                    IoRowKind::HdioLc => {
                        has_hdiolc = true;
                    }
                    _ => (),
                }
            }
            if has_hdiolc {
                ylut.insert(reg, bank);
                bank += 2;
            } else if has_io {
                ylut.insert(reg, bank);
                bank += 1;
            }
        }
        bankylut.push(ylut);
    }
    let cfg_bank = cfg_bank.unwrap();
    for ylut in bankylut.values_mut() {
        for bank in ylut.values_mut() {
            *bank += 25;
            *bank -= cfg_bank;
        }
    }

    let mut cfg_io = EntityVec::new();
    for (die, &chip) in chips {
        let mut die_cfg_io = BiHashMap::new();
        if let Some(iocol) = chip.cols_io.iter().find(|iocol| iocol.col == col_cfg_io) {
            if matches!(
                iocol.regs[chip.reg_cfg()],
                IoRowKind::Hpio | IoRowKind::Hrio
            ) {
                for idx in 0..52 {
                    if let Some(cfg) = if !chip.is_alt_cfg {
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
                            25 => Some(if chip.kind == ChipKind::Ultrascale {
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
                                reg: chip.reg_cfg(),
                                iob: TileIobId::from_idx(idx),
                            }),
                        );
                    }
                }
            }
        } else {
            let hcol = chip
                .cols_hard
                .iter()
                .find(|hcol| hcol.col == col_cfg_io)
                .unwrap();
            for idx in 0..84 {
                if let Some(cfg) = match idx {
                    14 => Some(SharedCfgPin::Data(31)),
                    15 => Some(SharedCfgPin::Data(30)),
                    16 => Some(SharedCfgPin::Data(28)),
                    17 => Some(SharedCfgPin::Data(26)),
                    18 => Some(SharedCfgPin::Data(24)),
                    19 => Some(SharedCfgPin::Data(22)),
                    21 => Some(SharedCfgPin::Data(20)),
                    22 => Some(SharedCfgPin::Data(18)),
                    23 => Some(SharedCfgPin::Data(16)),
                    24 => Some(SharedCfgPin::Data(14)),
                    30 => Some(SharedCfgPin::Data(29)),
                    31 => Some(SharedCfgPin::Data(27)),
                    32 => Some(SharedCfgPin::Data(25)),
                    33 => Some(SharedCfgPin::Data(23)),
                    35 => Some(SharedCfgPin::Data(21)),
                    36 => Some(SharedCfgPin::Data(19)),
                    37 => Some(SharedCfgPin::Data(17)),
                    38 => Some(SharedCfgPin::Data(15)),
                    39 => Some(SharedCfgPin::Data(13)),
                    40 => Some(SharedCfgPin::Data(12)),
                    43 => Some(SharedCfgPin::EmCclk),
                    57 => Some(SharedCfgPin::Data(11)),
                    58 => Some(SharedCfgPin::Data(10)),
                    59 => Some(SharedCfgPin::Data(8)),
                    60 => Some(SharedCfgPin::Data(7)),
                    61 => Some(SharedCfgPin::Data(5)),
                    62 => Some(SharedCfgPin::Busy),
                    64 => Some(SharedCfgPin::Fcs1B),
                    65 => Some(SharedCfgPin::CsiB),
                    66 => Some(SharedCfgPin::I2cSda),
                    67 => Some(SharedCfgPin::I2cSclk),
                    68 => Some(SharedCfgPin::PerstN0),
                    69 => Some(SharedCfgPin::SmbAlert),
                    73 => Some(SharedCfgPin::Data(9)),
                    74 => Some(SharedCfgPin::OspiDs),
                    75 => Some(SharedCfgPin::Data(6)),
                    76 => Some(SharedCfgPin::Data(4)),
                    80 => Some(SharedCfgPin::OspiRstB),
                    81 => Some(SharedCfgPin::OspiEccFail),
                    _ => None,
                } {
                    die_cfg_io.insert(
                        cfg,
                        IoCoord::HdioLc(HdioCoord {
                            die,
                            col: hcol.col,
                            reg: chip.reg_cfg(),
                            iob: TileIobId::from_idx(idx),
                        }),
                    );
                }
            }
        }
        cfg_io.push(die_cfg_io);
    }

    ExpandedDevice {
        kind: chips[interposer.primary].kind,
        chips: chips.clone(),
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
