#![allow(clippy::bool_to_int_with_if)]

use enum_map::{enum_map, EnumMap};
use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedDieRefMut, ExpandedGrid, RowId};
use std::collections::BTreeSet;

use crate::{
    BramKind, CleMKind, ColSide, ColumnKindLeft, ColumnKindRight, DisabledPart, ExpandedDevice,
    Grid, GridKind, HardRowKind, IoRowKind, RegId,
};

struct DieExpander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    ylut: EntityVec<RowId, usize>,
    sylut: EntityVec<RowId, usize>,
    dev_has_hbm: bool,
    hylut: EnumMap<HardRowKind, EntityVec<RegId, usize>>,
}

impl<'a, 'b> DieExpander<'a, 'b> {
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

    fn fill_hylut(&mut self, hy: &mut EnumMap<HardRowKind, usize>) {
        for reg in self.grid.regs() {
            let skip = self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg));
            for (k, v) in hy.iter_mut() {
                self.hylut[k].push(*v);
                let mut found = false;
                if !skip {
                    if self.grid.col_cfg.regs[reg] == k {
                        found = true;
                    }
                    if let Some(ref hard) = self.grid.col_hard {
                        if hard.regs[reg] == k {
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

    fn fill_int(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let x = col.to_idx();
            for row in self.die.rows() {
                let y = self.ylut[row];
                self.die
                    .fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                if row.to_idx() % 60 == 30 && y != 0 {
                    let lr = if col < self.grid.col_cfg.col {
                        'L'
                    } else {
                        'R'
                    };
                    let name = format!("RCLK_INT_{lr}_X{x}Y{yy}", yy = y - 1);
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("RCLK"),
                        &[&name],
                        self.db.get_node_naming("RCLK"),
                        &[(col, row)],
                    );
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
                    ColumnKindLeft::Gt | ColumnKindLeft::Io => {
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
                    ColumnKindLeft::Hard
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
                    ColumnKindRight::Gt | ColumnKindRight::Io => {
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
                    ColumnKindRight::Hard
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
                if cd.l == ColumnKindLeft::Io && col.to_idx() != 0 {
                    let term_e = self.db.get_term("IO.E");
                    let term_w = self.db.get_term("IO.W");
                    for row in self.die.rows() {
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
            }
        }
    }

    fn cut_regions(&mut self) {
        for reg in self.grid.regs() {
            if self
                .disabled
                .contains(&DisabledPart::Region(self.die.die, reg))
            {
                self.die.nuke_rect(
                    ColId::from_idx(0),
                    self.grid.row_reg_bot(reg),
                    self.grid.columns.len(),
                    60,
                );
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
        let dieid = self.die.die;
        for (col, &cd) in &self.grid.columns {
            let mut found = false;
            if let Some((kind, tk)) = match cd.l {
                ColumnKindLeft::CleL => Some(("CLEL_L", "CLEL_L")),
                ColumnKindLeft::CleM(_) => Some((
                    "CLEM",
                    match (self.grid.kind, col < self.grid.col_cfg.col) {
                        (GridKind::Ultrascale, true) => "CLE_M",
                        (GridKind::Ultrascale, false) => "CLE_M_R",
                        (GridKind::UltrascalePlus, true) => "CLEM",
                        (GridKind::UltrascalePlus, false) => "CLEM_R",
                    },
                )),
                _ => None,
            } {
                for row in self.die.rows() {
                    if cd.l == ColumnKindLeft::CleM(CleMKind::Laguna)
                        && self.grid.is_laguna_row(row)
                    {
                        continue;
                    }
                    let tile = &mut self.die[(col, row)];
                    if let Some(ps) = self.grid.ps {
                        if col == ps.col && row.to_idx() < ps.height() {
                            continue;
                        }
                    }
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let x = col.to_idx();
                    let y = self.ylut[row];
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
            if found {
                sx += 1;
            }
            let mut found = false;
            if matches!(cd.r, ColumnKindRight::CleL(_)) {
                for row in self.die.rows() {
                    let tile = &mut self.die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let x = col.to_idx();
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
                    let tk = match (self.grid.kind, cd.l, col < self.grid.col_cfg.col) {
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
                }
            }
            bx += 1;
        }
    }

    fn fill_dsp(&mut self) {
        let dieid = self.die.die;
        let mut dx = 0;
        for (col, &cd) in &self.grid.columns {
            if !matches!(cd.r, ColumnKindRight::Dsp(_)) {
                continue;
            }
            for row in self.die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                if self.grid.has_hbm && row.to_idx() < 15 {
                    continue;
                }
                let tile = &mut self.die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = col.to_idx();
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
            }
            ux += 1;
        }
    }

    fn fill_hard_single(&mut self, col: ColId, reg: RegId, kind: HardRowKind, sx: usize) {
        let sy = self.hylut[kind][reg];
        let row = self.grid.row_reg_bot(reg);
        if self
            .disabled
            .contains(&DisabledPart::Region(self.die.die, reg))
        {
            return;
        }
        let (nk, tk, bk) = match kind {
            HardRowKind::None => return,
            HardRowKind::Hdio | HardRowKind::HdioAms => {
                // XXX HDIO
                return;
            }
            HardRowKind::Cfg => {
                // XXX CFG
                return;
            }
            HardRowKind::Ams => {
                // XXX CFG
                return;
            }
            HardRowKind::Pcie => {
                if self.grid.kind == GridKind::Ultrascale {
                    ("PCIE", "PCIE", "PCIE_3_1")
                } else {
                    ("PCIE4", "PCIE4_PCIE4_FT", "PCIE40E4")
                }
            }
            HardRowKind::PciePlus => ("PCIE4C", "PCIE4C_PCIE4C_FT", "PCIE4CE4"),
            HardRowKind::Cmac => {
                if self.grid.kind == GridKind::Ultrascale {
                    ("CMAC", "CMAC_CMAC_FT", "CMAC_SITE")
                } else {
                    ("CMAC", "CMAC", "CMACE4")
                }
            }
            HardRowKind::Ilkn => {
                if self.grid.kind == GridKind::Ultrascale {
                    ("ILKN", "ILMAC_ILMAC_FT", "ILKN_SITE")
                } else {
                    ("ILKN", "ILKN_ILKN_FT", "ILKNE4")
                }
            }
            HardRowKind::DfeA => ("DFE_A", "DFE_DFE_TILEA_FT", "DFE_A"),
            HardRowKind::DfeG => ("DFE_G", "DFE_DFE_TILEG_FT", "DFE_G"),
        };
        let mut x = col.to_idx() - 1;
        if self.grid.kind == GridKind::Ultrascale
            && kind == HardRowKind::Cmac
            && col != self.grid.col_cfg.col
        {
            x = col.to_idx();
        }
        let name = format!("{tk}_X{x}Y{y}", y = self.ylut[row]);
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
            self.db.get_node_naming(nk),
            &crds,
        );
        node.add_bel(0, format!("{bk}_X{sx}Y{sy}"));
    }

    fn fill_hard(&mut self, hcx: &EnumMap<HardRowKind, usize>) {
        if let Some(ref hard) = self.grid.col_hard {
            for reg in self.grid.regs() {
                self.fill_hard_single(hard.col, reg, hard.regs[reg], 0);
            }
        }
        for reg in self.grid.regs() {
            let kind = self.grid.col_cfg.regs[reg];
            self.fill_hard_single(self.grid.col_cfg.col, reg, kind, hcx[kind]);
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
                let node = self.die[(col, row)].add_xnode(
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

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &'a Grid>,
    grid_master: DieId,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut egrid = ExpandedGrid::new(db);
    let mut y = 0;
    let mut sy = 0;
    let mut hy = EnumMap::default();
    let hcx = enum_map! {
        k => if grids.values().any(|grid| {
             if let Some(ref hard) = grid.col_hard {
                 hard.regs.values().any(|&x| x == k)
             } else {
                 false
             }
        }) {
            1
        } else {
            0
        }
    };
    for (_, grid) in grids {
        let (_, die) = egrid.add_die(grid.columns.len(), grid.regs * 60);

        let mut expander = DieExpander {
            grid,
            disabled,
            die,
            db,
            ylut: EntityVec::new(),
            sylut: EntityVec::new(),
            dev_has_hbm: grids.first().unwrap().has_hbm,
            hylut: EnumMap::default(),
        };

        y = expander.fill_ylut(y);
        sy = expander.fill_sylut(sy);
        expander.fill_hylut(&mut hy);

        expander.fill_int();
        expander.fill_io_pass();
        expander.fill_ps();
        expander.cut_regions();
        expander.fill_term();
        expander.die.fill_main_passes();
        expander.fill_clb();
        expander.fill_bram();
        expander.fill_dsp();
        expander.fill_uram();
        expander.fill_fe();
        expander.fill_dfe();
        expander.fill_hard(&hcx);
        expander.fill_clkroot();
    }

    ExpandedDevice {
        grids: grids.clone(),
        grid_master,
        egrid,
        disabled: disabled.clone(),
    }
}
