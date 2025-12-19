#![allow(clippy::type_complexity)]

use bimap::BiHashMap;
use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::DirH;
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, ExpandedGrid, RowId, TileIobId};
use std::collections::BTreeSet;
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{
    Chip, ChipKind, CleMKind, Column, ColumnKind, ConfigKind, DisabledPart, DspKind, HardKind,
    HardRowKind, Interposer, IoRowKind, RegId,
};
use crate::expanded::{ClkSrc, ExpandedDevice, HdioCoord, HpioCoord, IoCoord, Xp5ioCoord};

use crate::bond::SharedCfgPad;
use crate::regions;

struct DieExpander<'a, 'b, 'c> {
    chip: &'b Chip,
    disabled: &'a BTreeSet<DisabledPart>,
    egrid: &'a mut ExpandedGrid<'b>,
    die: DieId,
    io: &'c mut Vec<IoCoord>,
    gt: &'c mut Vec<CellCoord>,
}

impl DieExpander<'_, '_, '_> {
    fn in_int_hole(&self, cell: CellCoord) -> bool {
        self.chip.in_int_hole(cell.col, cell.row)
    }

    fn in_site_hole(&self, cell: CellCoord) -> bool {
        self.chip.in_site_hole(cell.col, cell.row)
    }

    fn fill_int(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            if self.disabled.contains(&DisabledPart::Region(
                self.die,
                self.chip.row_to_reg(cell.row),
            )) {
                continue;
            }
            if self.in_int_hole(cell) {
                continue;
            }
            if self.chip.col_side(cell.col) == DirH::W {
                self.egrid.add_tile_e(cell, "INT", 2);
                if cell.row.to_idx() % 60 == 30 {
                    self.egrid.add_tile(
                        cell,
                        "RCLK_INT",
                        &[
                            cell.delta(0, 0),
                            cell.delta(1, 0),
                            cell.delta(0, -1),
                            cell.delta(1, -1),
                        ],
                    );
                }
            }
            match self.chip.columns[cell.col].kind {
                ColumnKind::CleL(_) | ColumnKind::CleM(_) => (),
                ColumnKind::Bram(_)
                | ColumnKind::Dsp(_)
                | ColumnKind::Uram
                | ColumnKind::ContUram => {
                    self.egrid.add_tile_single(cell, "INTF");
                }
                ColumnKind::Gt(idx) | ColumnKind::Io(idx) => {
                    let iocol = &self.chip.cols_io[idx];
                    let rk = iocol.regs[self.chip.row_to_reg(cell.row)];
                    match (self.chip.kind, rk) {
                        (_, IoRowKind::None) => (),

                        (
                            ChipKind::UltrascalePlus,
                            IoRowKind::Hpio | IoRowKind::Hrio | IoRowKind::HdioL | IoRowKind::Xp5io,
                        ) => {
                            self.egrid.add_tile_single(cell, "INTF.IO");
                        }
                        _ => {
                            self.egrid.add_tile_single(cell, "INTF.DELAY");
                        }
                    }
                }
                ColumnKind::Hard(_, _)
                | ColumnKind::DfeC
                | ColumnKind::DfeDF
                | ColumnKind::DfeE
                | ColumnKind::HdioS
                | ColumnKind::ContHard
                | ColumnKind::Sdfec
                | ColumnKind::DfeB => {
                    self.egrid.add_tile_single(cell, "INTF.DELAY");
                }
            }
        }
    }

    fn fill_ps(&mut self) {
        if let Some(ps) = self.chip.ps {
            let cell = CellCoord::new(self.die, ps.col, RowId::from_idx(0));
            let height = ps.height();
            let width = ps.col.to_idx();
            if height != self.chip.regs * 60 {
                let row_t = RowId::from_idx(height);
                for dx in 0..width {
                    let col = ColId::from_idx(dx);
                    self.egrid.fill_conn_term(
                        CellCoord::new(self.die, col, row_t),
                        &format!("TERM.S{side}", side = col.to_idx() % 2),
                    );
                }
            }
            for dy in 0..height {
                let dy = dy as i32;
                self.egrid.fill_conn_term(cell.delta(0, dy), "TERM.W");
                self.egrid.fill_conn_term(cell.delta(0, dy), "TERM.LW");
                self.egrid.add_tile_single(cell.delta(0, dy), "INTF.IO");
                if dy % 60 == 30 {
                    self.egrid.add_tile_single(cell.delta(0, dy), "RCLK_PS");
                }
            }
            let dy = if ps.has_vcu { 60 } else { 0 };
            self.egrid.add_tile_n(cell.delta(0, dy), "PS", 180);
            if ps.has_vcu {
                self.egrid.add_tile_n(cell, "VCU", 60);
            }
        }
    }

    fn fill_term(&mut self) {
        for cell in self
            .egrid
            .row(self.die, self.egrid.rows(self.die).next().unwrap())
        {
            let side = cell.col.to_idx() % 2;
            if !self.in_int_hole(cell)
                && !self.disabled.contains(&DisabledPart::Region(
                    self.die,
                    self.chip.row_to_reg(cell.row),
                ))
            {
                self.egrid.fill_conn_term(cell, &format!("TERM.S{side}"));
            }
        }
        for cell in self
            .egrid
            .row(self.die, self.egrid.rows(self.die).next_back().unwrap())
        {
            let side = cell.col.to_idx() % 2;
            if !self.in_int_hole(cell)
                && !self.disabled.contains(&DisabledPart::Region(
                    self.die,
                    self.chip.row_to_reg(cell.row),
                ))
            {
                self.egrid.fill_conn_term(cell, &format!("TERM.N{side}"));
            }
        }
        for cell in self
            .egrid
            .column(self.die, self.egrid.cols(self.die).next().unwrap())
        {
            if !self.in_int_hole(cell) {
                self.egrid.fill_conn_term(cell, "TERM.W");
                self.egrid.fill_conn_term(cell, "TERM.LW");
            }
        }
        for cell in self
            .egrid
            .column(self.die, self.egrid.cols(self.die).next_back().unwrap())
        {
            if !self.in_int_hole(cell) {
                self.egrid.fill_conn_term(cell, "TERM.E");
                self.egrid.fill_conn_term(cell.delta(-1, 0), "TERM.LE");
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
            for cell in self.egrid.column(self.die, col) {
                if self.disabled.contains(&DisabledPart::Region(
                    self.die,
                    self.chip.row_to_reg(cell.row),
                )) {
                    continue;
                }
                if self.in_int_hole(cell) || self.in_int_hole(cell.delta(-1, 0)) {
                    continue;
                }
                if is_io_mid {
                    self.egrid
                        .fill_conn_pair(cell.delta(-1, 0), cell, "IO.E", "IO.W");
                    self.egrid
                        .fill_conn_pair(cell.delta(-2, 0), cell, "IO.LE", "IO.LW");
                } else {
                    self.egrid
                        .fill_conn_pair(cell.delta(-1, 0), cell, "MAIN.E", "MAIN.W");
                    if self.chip.col_side(col) == DirH::W {
                        self.egrid
                            .fill_conn_pair(cell.delta(-2, 0), cell, "MAIN.LE", "MAIN.LW");
                    }
                }
            }
        }
        for row in self.egrid.rows(self.die) {
            if row == self.chip.rows().next_back().unwrap() {
                continue;
            }
            if self
                .disabled
                .contains(&DisabledPart::Region(self.die, self.chip.row_to_reg(row)))
            {
                continue;
            }
            if self.disabled.contains(&DisabledPart::Region(
                self.die,
                self.chip.row_to_reg(row + 1),
            )) {
                continue;
            }
            for cell in self.egrid.row(self.die, row) {
                if self.in_int_hole(cell) || self.in_int_hole(cell.delta(0, 1)) {
                    continue;
                }
                self.egrid
                    .fill_conn_pair(cell, cell.delta(0, 1), "MAIN.N", "MAIN.S");
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
            for cell in self.egrid.column(self.die, col) {
                if self.in_site_hole(cell) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die,
                    self.chip.row_to_reg(cell.row),
                )) {
                    continue;
                }
                if cd.kind == ColumnKind::CleM(CleMKind::Laguna)
                    && self.chip.is_laguna_row(cell.row)
                {
                    self.egrid.add_tile_single(cell, "LAGUNA");
                } else {
                    self.egrid.add_tile_single(cell, kind);
                }
                if cell.row == self.chip.row_rclk(cell.row) {
                    if matches!(cd.kind, ColumnKind::CleM(CleMKind::ClkBuf)) {
                        self.egrid.add_tile(cell, "RCLK_HROUTE_SPLITTER.CLE", &[]);
                    } else if cd.kind == ColumnKind::CleM(CleMKind::Laguna)
                        && self.chip.is_laguna_row(cell.row)
                    {
                        if self.chip.kind == ChipKind::Ultrascale {
                            continue;
                        }
                        self.egrid.add_tile_single(cell, "RCLK_V_SINGLE.LAG");
                    } else if self.chip.col_side(col) == DirH::W
                        || self.chip.kind != ChipKind::UltrascalePlus
                    {
                        self.egrid.add_tile_single(cell, "RCLK_V_SINGLE.CLE");
                    }
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if !matches!(cd.kind, ColumnKind::Bram(_)) {
                continue;
            }
            for cell in self.egrid.column(self.die, col) {
                if !cell.row.to_idx().is_multiple_of(5) {
                    continue;
                }
                if self.in_site_hole(cell) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die,
                    self.chip.row_to_reg(cell.row),
                )) {
                    continue;
                }
                self.egrid.add_tile_n(cell, "BRAM", 5);
                if cell.row == self.chip.row_rclk(cell.row) {
                    self.egrid.add_tile_single(cell, "HARD_SYNC");
                    if self.chip.kind == ChipKind::Ultrascale {
                        self.egrid.add_tile_single(cell, "RCLK_V_DOUBLE.BRAM");
                    } else {
                        self.egrid.add_tile_single(cell, "RCLK_V_QUAD.BRAM");
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
            for cell in self.egrid.column(self.die, col) {
                if !cell.row.to_idx().is_multiple_of(5) {
                    continue;
                }
                if self.in_int_hole(cell) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die,
                    self.chip.row_to_reg(cell.row),
                )) {
                    continue;
                }
                if self.chip.has_hbm && cell.row.to_idx() < 15 {
                    if cell.row.to_idx() != 0 {
                        continue;
                    }
                    if cell.col < self.chip.cols_io[1].col
                        && self.disabled.contains(&DisabledPart::HbmLeft)
                    {
                        continue;
                    }
                    self.egrid.add_tile_n(cell, "BLI", 15);
                } else {
                    self.egrid.add_tile_n(cell, "DSP", 5);
                }
                if cell.row == self.chip.row_rclk(cell.row) {
                    if matches!(cd.kind, ColumnKind::Dsp(DspKind::ClkBuf)) {
                        self.egrid.add_tile(cell, "RCLK_SPLITTER", &[]);
                    } else {
                        self.egrid.add_tile_single(cell, "RCLK_V_DOUBLE.DSP");
                    }
                }
            }
        }
    }

    fn fill_uram(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd.kind != ColumnKind::Uram {
                continue;
            }
            for cell in self.egrid.column(self.die, col) {
                if !cell.row.to_idx().is_multiple_of(15) {
                    continue;
                }
                if self.in_int_hole(cell) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Region(
                    self.die,
                    self.chip.row_to_reg(cell.row),
                )) {
                    continue;
                }
                let mut tcells = vec![];
                tcells.extend(cell.cells_n_const::<15>());
                tcells.extend(cell.delta(1, 0).cells_n_const::<15>());
                self.egrid.add_tile(cell, "URAM", &tcells);
                if cell.row == self.chip.row_rclk(cell.row) {
                    self.egrid
                        .add_tile_single(cell.delta(1, 0), "RCLK_V_QUAD.URAM");
                }
            }
        }
    }

    fn fill_hard_single(&mut self, cell: CellCoord, kind: HardRowKind) {
        let reg = self.chip.row_to_reg(cell.row);
        if self.disabled.contains(&DisabledPart::Region(self.die, reg)) {
            return;
        }

        let nk = match kind {
            HardRowKind::None => return,
            HardRowKind::Hdio | HardRowKind::HdioAms => {
                for (i, nk) in ["HDIO_S", "HDIO_N"].into_iter().enumerate() {
                    let cell = cell.delta(0, (i * 30) as i32);
                    self.egrid.add_tile_n(cell, nk, 30);
                }
                for idx in 0..24 {
                    self.io.push(IoCoord::Hdio(HdioCoord {
                        cell: cell.delta(0, 30),
                        iob: TileIobId::from_idx(idx),
                    }));
                }
                self.egrid
                    .add_tile_sn(cell.delta(0, 30), "RCLK_HDIO", 30, 60);
                return;
            }
            HardRowKind::HdioL => {
                for (i, nk) in ["HDIOL_S", "HDIOL_N"].into_iter().enumerate() {
                    let cell = cell.delta(0, (i * 30) as i32);
                    self.egrid.add_tile_n(cell, nk, 30);
                    for j in 0..42 {
                        self.io.push(IoCoord::HdioLc(HdioCoord {
                            cell,
                            iob: TileIobId::from_idx(j),
                        }));
                    }
                }
                self.egrid
                    .add_tile_sn(cell.delta(0, 30), "RCLK_HDIOL", 30, 60);
                return;
            }
            HardRowKind::Cfg => {
                let kind = match self.chip.config_kind {
                    ConfigKind::Config => "CFG",
                    ConfigKind::Csec => "CFG_CSEC",
                    ConfigKind::CsecV2 => "CFG_CSEC_V2",
                };
                self.egrid.add_tile_n(cell, kind, 60);
                self.egrid
                    .add_tile(cell.delta(0, 30), "RCLK_HROUTE_SPLITTER.HARD", &[]);
                return;
            }
            HardRowKind::Ams => {
                self.egrid.add_tile_n(cell, "CFGIO", 30);
                self.egrid
                    .add_tile(cell.delta(0, 30), "RCLK_HROUTE_SPLITTER.HARD", &[]);
                self.egrid.add_tile_n(cell.delta(0, 30), "AMS", 30);
                return;
            }
            HardRowKind::Pcie => {
                if self.chip.kind == ChipKind::Ultrascale {
                    "PCIE"
                } else {
                    "PCIE4"
                }
            }
            HardRowKind::Pcie4C => "PCIE4C",
            HardRowKind::Pcie4CE => "PCIE4CE",
            HardRowKind::Cmac => "CMAC",
            HardRowKind::Ilkn => "ILKN",
            HardRowKind::DfeA => "DFE_A",
            HardRowKind::DfeG => "DFE_G",
        };
        self.egrid
            .add_tile(cell.delta(0, 30), "RCLK_HROUTE_SPLITTER.HARD", &[]);
        let mut tcells = vec![];
        tcells.extend(cell.cells_n_const::<60>());
        tcells.extend(cell.delta(1, 0).cells_n_const::<60>());
        self.egrid.add_tile(cell, nk, &tcells);
    }

    fn fill_hard(&mut self, has_pcie_cfg: &mut bool) {
        for hc in &self.chip.cols_hard {
            let is_cfg = hc.regs.values().any(|&x| x == HardRowKind::Cfg);
            for reg in self.chip.regs() {
                let kind = hc.regs[reg];
                if kind == HardRowKind::Cfg
                    && reg.to_idx() != 0
                    && matches!(hc.regs[reg - 1], HardRowKind::Pcie | HardRowKind::Pcie4C)
                {
                    *has_pcie_cfg = true;
                }
                self.fill_hard_single(
                    CellCoord::new(self.die, hc.col, self.chip.row_reg_bot(reg)),
                    kind,
                );
            }
            if is_cfg && self.chip.has_hbm {
                self.egrid.add_tile(
                    CellCoord::new(self.die, hc.col, RowId::from_idx(0)),
                    "HBM_ABUS_SWITCH",
                    &[],
                );
            }
        }
    }

    fn fill_io(&mut self) {
        for ioc in &self.chip.cols_io {
            for reg in self.chip.regs() {
                if self.disabled.contains(&DisabledPart::Region(self.die, reg)) {
                    continue;
                }
                let kind = ioc.regs[reg];
                let cell = CellCoord::new(self.die, ioc.col, self.chip.row_reg_rclk(reg));
                match kind {
                    IoRowKind::None => (),
                    IoRowKind::Hpio | IoRowKind::Hrio => {
                        for idx in 0..52 {
                            self.io.push(IoCoord::Hpio(HpioCoord {
                                cell,
                                iob: TileIobId::from_idx(idx),
                            }));
                        }
                        if self.chip.kind == ChipKind::Ultrascale {
                            self.egrid.add_tile_sn(cell, "XIPHY", 30, 60);
                            if kind == IoRowKind::Hpio {
                                self.egrid.add_tile_sn(cell, "RCLK_HPIO", 30, 60);
                                for i in 0..2 {
                                    self.egrid
                                        .add_tile_n(cell.delta(0, -30 + i * 30), "HPIO", 30);
                                }
                            } else {
                                self.egrid.add_tile(cell, "RCLK_HRIO", &[]);
                                for i in 0..2 {
                                    self.egrid
                                        .add_tile_n(cell.delta(0, -30 + i * 30), "HRIO", 30);
                                }
                            }
                        } else {
                            let is_hbm = self.chip.has_hbm && reg.to_idx() == 0;
                            let kind = if is_hbm { "CMT_HBM" } else { "CMT" };
                            self.egrid.add_tile_sn(cell, kind, 30, 60);
                            self.egrid.add_tile(cell, "RCLK_XIPHY", &[]);

                            for i in 0..4 {
                                self.egrid
                                    .add_tile_n(cell.delta(0, -30 + i * 15), "XIPHY", 15);
                            }

                            for i in 0..2 {
                                self.egrid
                                    .add_tile_n(cell.delta(0, -30 + i * 30), "HPIO", 30);
                            }
                            self.egrid.add_tile_sn(cell, "RCLK_HPIO", 30, 60);
                        }
                    }
                    IoRowKind::HdioL => {
                        for (i, nk) in ["HDIOL_S", "HDIOL_N"].into_iter().enumerate() {
                            let cell = cell.delta(0, -30 + i as i32 * 30);
                            self.egrid.add_tile_n(cell, nk, 30);
                            for idx in 0..42 {
                                self.io.push(IoCoord::HdioLc(HdioCoord {
                                    cell,
                                    iob: TileIobId::from_idx(idx),
                                }));
                            }
                        }
                        self.egrid.add_tile_sn(cell, "CMT", 30, 60);
                        self.egrid.add_tile_sn(cell, "RCLK_HDIOL", 30, 60);
                    }
                    IoRowKind::Xp5io => {
                        self.egrid.add_tile_sn(cell, "CMTXP", 30, 60);
                        self.egrid.add_tile_sn(cell, "XP5IO", 30, 60);
                        for idx in 0..66 {
                            self.io.push(IoCoord::Xp5io(Xp5ioCoord {
                                cell,
                                iob: TileIobId::from_idx(idx),
                            }));
                        }
                    }
                    _ => {
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
                        self.egrid.add_tile_sn(cell, nk, 30, 60);
                        self.gt.push(cell);
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
                if self.disabled.contains(&DisabledPart::Region(self.die, reg)) {
                    continue;
                }
                let cell = CellCoord::new(self.die, col, self.chip.row_reg_bot(reg));
                self.egrid.add_tile_n(cell, "FE", 60);
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
                let cell = CellCoord::new(self.die, col, self.chip.row_reg_bot(reg));
                let kind = if kind == "DFE_D" && reg.to_idx() == 2 {
                    "DFE_F"
                } else {
                    kind
                };
                if matches!(cd.kind, ColumnKind::DfeB | ColumnKind::DfeE) {
                    self.egrid
                        .add_tile(cell.delta(0, 30), "RCLK_HROUTE_SPLITTER.HARD", &[]);
                }
                if bi {
                    let mut tcells = vec![];
                    tcells.extend(cell.cells_n_const::<60>());
                    tcells.extend(cell.delta(1, 0).cells_n_const::<60>());
                    self.egrid.add_tile(cell, kind, &tcells);
                } else {
                    self.egrid.add_tile_n(cell, kind, 60);
                }
            }
        }
    }

    fn fill_hdios(&mut self) {
        for (col, &cd) in &self.chip.columns {
            if cd.kind != ColumnKind::HdioS {
                continue;
            }
            for reg in self.chip.regs() {
                let row = self.chip.row_reg_bot(reg);
                let cell = CellCoord::new(self.die, col, row);
                let mut tcells = vec![];
                tcells.extend(cell.cells_n_const::<60>());
                tcells.extend(cell.delta(1, 0).cells_n_const::<60>());
                self.egrid
                    .add_tile_sn(cell.delta(0, 30), "RCLK_HDIOS", 30, 60);
                self.egrid.add_tile(cell, "HDIOS", &tcells);
                for i in 0..42 {
                    self.io.push(IoCoord::HdioLc(HdioCoord {
                        cell,
                        iob: TileIobId::from_idx(i),
                    }));
                }
            }
        }
    }

    fn fill_clkroot(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            let row_rclk = self.chip.row_rclk(cell.row);
            let cell_leaf = if cell.row < row_rclk {
                cell.with_row(row_rclk - 1)
            } else {
                cell.with_row(row_rclk)
            };
            self.egrid[cell].region_root[regions::LEAF] = cell_leaf;
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
            | ColumnKind::DfeB
            | ColumnKind::HdioS => {
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
        let die = egrid.add_die(chip.columns.len(), chip.regs * 60);

        let mut expander = DieExpander {
            chip,
            disabled,
            egrid: &mut egrid,
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
        expander.fill_hdios();
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
        if let ColumnKind::Io(_) = cd.kind
            && (col_cfg_io.is_none() || pchip.col_side(col) == DirH::W)
        {
            col_cfg_io = Some(col);
        }
        if cd.kind == ColumnKind::HdioS {
            col_cfg_io = Some(col);
        }
        if let ColumnKind::Hard(HardKind::Term, idx) = cd.kind {
            let mut has_hdiolc = false;
            for chip in chips.values() {
                if chip.cols_hard[idx]
                    .regs
                    .values()
                    .any(|&kind| kind == HardRowKind::HdioL)
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
            ColumnKind::HdioS => {
                ioxlut.insert(col, iox);
                iox += 1;
            }
            ColumnKind::Hard(_, idx) => {
                let regs = &pchip.cols_hard[idx].regs;
                if regs.values().any(|x| {
                    matches!(
                        x,
                        HardRowKind::Hdio | HardRowKind::HdioAms | HardRowKind::HdioL
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
            && !pchip.config_kind.is_csec()
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
                    HardRowKind::HdioL => {
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
                    IoRowKind::HdioL => {
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
                let cell = CellCoord::new(die, iocol.col, chip.row_reg_rclk(chip.reg_cfg()));
                for idx in 0..52 {
                    if let Some(cfg) = if !chip.is_alt_cfg {
                        match idx {
                            0 => Some(SharedCfgPad::Rs(0)),
                            1 => Some(SharedCfgPad::Rs(1)),
                            2 => Some(SharedCfgPad::FoeB),
                            3 => Some(SharedCfgPad::FweB),
                            4 => Some(SharedCfgPad::Addr(26)),
                            5 => Some(SharedCfgPad::Addr(27)),
                            6 => Some(SharedCfgPad::Addr(24)),
                            7 => Some(SharedCfgPad::Addr(25)),
                            8 => Some(SharedCfgPad::Addr(22)),
                            9 => Some(SharedCfgPad::Addr(23)),
                            10 => Some(SharedCfgPad::Addr(20)),
                            11 => Some(SharedCfgPad::Addr(21)),
                            12 => Some(SharedCfgPad::Addr(28)),
                            13 => Some(SharedCfgPad::Addr(18)),
                            14 => Some(SharedCfgPad::Addr(19)),
                            15 => Some(SharedCfgPad::Addr(16)),
                            16 => Some(SharedCfgPad::Addr(17)),
                            17 => Some(SharedCfgPad::Data(30)),
                            18 => Some(SharedCfgPad::Data(31)),
                            19 => Some(SharedCfgPad::Data(28)),
                            20 => Some(SharedCfgPad::Data(29)),
                            21 => Some(SharedCfgPad::Data(26)),
                            22 => Some(SharedCfgPad::Data(27)),
                            23 => Some(SharedCfgPad::Data(24)),
                            24 => Some(SharedCfgPad::Data(25)),
                            25 => Some(if chip.kind == ChipKind::Ultrascale {
                                SharedCfgPad::PerstN1
                            } else {
                                SharedCfgPad::SmbAlert
                            }),
                            26 => Some(SharedCfgPad::Data(22)),
                            27 => Some(SharedCfgPad::Data(23)),
                            28 => Some(SharedCfgPad::Data(20)),
                            29 => Some(SharedCfgPad::Data(21)),
                            30 => Some(SharedCfgPad::Data(18)),
                            31 => Some(SharedCfgPad::Data(19)),
                            32 => Some(SharedCfgPad::Data(16)),
                            33 => Some(SharedCfgPad::Data(17)),
                            34 => Some(SharedCfgPad::Data(14)),
                            35 => Some(SharedCfgPad::Data(15)),
                            36 => Some(SharedCfgPad::Data(12)),
                            37 => Some(SharedCfgPad::Data(13)),
                            38 => Some(SharedCfgPad::CsiB),
                            39 => Some(SharedCfgPad::Data(10)),
                            40 => Some(SharedCfgPad::Data(11)),
                            41 => Some(SharedCfgPad::Data(8)),
                            42 => Some(SharedCfgPad::Data(9)),
                            43 => Some(SharedCfgPad::Data(6)),
                            44 => Some(SharedCfgPad::Data(7)),
                            45 => Some(SharedCfgPad::Data(4)),
                            46 => Some(SharedCfgPad::Data(5)),
                            47 => Some(SharedCfgPad::I2cSclk),
                            48 => Some(SharedCfgPad::I2cSda),
                            49 => Some(SharedCfgPad::EmCclk),
                            50 => Some(SharedCfgPad::Dout),
                            51 => Some(SharedCfgPad::PerstN0),
                            _ => None,
                        }
                    } else {
                        match idx {
                            0 => Some(SharedCfgPad::Rs(1)),
                            1 => Some(SharedCfgPad::FweB),
                            2 => Some(SharedCfgPad::Rs(0)),
                            3 => Some(SharedCfgPad::FoeB),
                            4 => Some(SharedCfgPad::Addr(28)),
                            5 => Some(SharedCfgPad::Addr(26)),
                            6 => Some(SharedCfgPad::SmbAlert),
                            7 => Some(SharedCfgPad::Addr(27)),
                            8 => Some(SharedCfgPad::Addr(24)),
                            9 => Some(SharedCfgPad::Addr(22)),
                            10 => Some(SharedCfgPad::Addr(25)),
                            11 => Some(SharedCfgPad::Addr(23)),
                            12 => Some(SharedCfgPad::Addr(20)),
                            13 => Some(SharedCfgPad::Addr(18)),
                            14 => Some(SharedCfgPad::Addr(16)),
                            15 => Some(SharedCfgPad::Addr(19)),
                            16 => Some(SharedCfgPad::Addr(17)),
                            17 => Some(SharedCfgPad::Data(30)),
                            18 => Some(SharedCfgPad::Data(28)),
                            19 => Some(SharedCfgPad::Data(31)),
                            20 => Some(SharedCfgPad::Data(29)),
                            21 => Some(SharedCfgPad::Data(26)),
                            22 => Some(SharedCfgPad::Data(24)),
                            23 => Some(SharedCfgPad::Data(27)),
                            24 => Some(SharedCfgPad::Data(25)),
                            25 => Some(SharedCfgPad::Addr(21)),
                            26 => Some(SharedCfgPad::CsiB),
                            27 => Some(SharedCfgPad::Data(22)),
                            28 => Some(SharedCfgPad::EmCclk),
                            29 => Some(SharedCfgPad::Data(23)),
                            30 => Some(SharedCfgPad::Data(20)),
                            31 => Some(SharedCfgPad::Data(18)),
                            32 => Some(SharedCfgPad::Data(21)),
                            33 => Some(SharedCfgPad::Data(19)),
                            34 => Some(SharedCfgPad::Data(16)),
                            35 => Some(SharedCfgPad::Data(14)),
                            36 => Some(SharedCfgPad::Data(17)),
                            37 => Some(SharedCfgPad::Data(15)),
                            38 => Some(SharedCfgPad::Data(12)),
                            39 => Some(SharedCfgPad::Data(10)),
                            40 => Some(SharedCfgPad::Data(8)),
                            41 => Some(SharedCfgPad::Data(11)),
                            42 => Some(SharedCfgPad::Data(9)),
                            43 => Some(SharedCfgPad::Data(6)),
                            44 => Some(SharedCfgPad::Data(4)),
                            45 => Some(SharedCfgPad::Data(7)),
                            46 => Some(SharedCfgPad::Data(5)),
                            47 => Some(SharedCfgPad::I2cSclk),
                            48 => Some(SharedCfgPad::Dout),
                            49 => Some(SharedCfgPad::I2cSda),
                            50 => Some(SharedCfgPad::PerstN0),
                            51 => Some(SharedCfgPad::Data(13)),
                            _ => None,
                        }
                    } {
                        die_cfg_io.insert(
                            cfg,
                            IoCoord::Hpio(HpioCoord {
                                cell,
                                iob: TileIobId::from_idx(idx),
                            }),
                        );
                    }
                }
            }
        } else if let Some(hcol) = chip.cols_hard.iter().find(|hcol| hcol.col == col_cfg_io) {
            let cell = CellCoord::new(die, hcol.col, chip.row_reg_bot(chip.reg_cfg()));
            for idx in 0..84 {
                if let Some(cfg) = match idx {
                    14 => Some(SharedCfgPad::Data(31)),
                    15 => Some(SharedCfgPad::Data(30)),
                    16 => Some(SharedCfgPad::Data(28)),
                    17 => Some(SharedCfgPad::Data(26)),
                    18 => Some(SharedCfgPad::Data(24)),
                    19 => Some(SharedCfgPad::Data(22)),
                    21 => Some(SharedCfgPad::Data(20)),
                    22 => Some(SharedCfgPad::Data(18)),
                    23 => Some(SharedCfgPad::Data(16)),
                    24 => Some(SharedCfgPad::Data(14)),
                    30 => Some(SharedCfgPad::Data(29)),
                    31 => Some(SharedCfgPad::Data(27)),
                    32 => Some(SharedCfgPad::Data(25)),
                    33 => Some(SharedCfgPad::Data(23)),
                    35 => Some(SharedCfgPad::Data(21)),
                    36 => Some(SharedCfgPad::Data(19)),
                    37 => Some(SharedCfgPad::Data(17)),
                    38 => Some(SharedCfgPad::Data(15)),
                    39 => Some(SharedCfgPad::Data(13)),
                    40 => Some(SharedCfgPad::Data(12)),
                    43 => Some(SharedCfgPad::EmCclk),
                    57 => Some(SharedCfgPad::Data(11)),
                    58 => Some(SharedCfgPad::Data(10)),
                    59 => Some(SharedCfgPad::Data(8)),
                    60 => Some(SharedCfgPad::Data(7)),
                    61 => Some(SharedCfgPad::Data(5)),
                    62 => Some(SharedCfgPad::Busy),
                    64 => Some(SharedCfgPad::Fcs1B),
                    65 => Some(SharedCfgPad::CsiB),
                    66 => Some(SharedCfgPad::I2cSda),
                    67 => Some(SharedCfgPad::I2cSclk),
                    68 => Some(SharedCfgPad::PerstN0),
                    69 => Some(SharedCfgPad::SmbAlert),
                    73 => Some(SharedCfgPad::Data(9)),
                    74 => Some(SharedCfgPad::OspiDs),
                    75 => Some(SharedCfgPad::Data(6)),
                    76 => Some(SharedCfgPad::Data(4)),
                    80 => Some(SharedCfgPad::OspiRstB),
                    81 => Some(SharedCfgPad::OspiEccFail),
                    _ => None,
                } {
                    die_cfg_io.insert(
                        cfg,
                        if idx < 42 {
                            IoCoord::HdioLc(HdioCoord {
                                cell,
                                iob: TileIobId::from_idx(idx),
                            })
                        } else {
                            IoCoord::HdioLc(HdioCoord {
                                cell: cell.delta(0, 30),
                                iob: TileIobId::from_idx(idx - 42),
                            })
                        },
                    );
                }
            }
        } else {
            let cell = CellCoord::new(die, col_cfg_io, chip.row_reg_bot(chip.reg_cfg()));
            for idx in 0..42 {
                if let Some(cfg) = match idx {
                    0 => Some(SharedCfgPad::Data(31)),
                    1 => Some(SharedCfgPad::Data(30)),
                    2 => Some(SharedCfgPad::Data(28)),
                    3 => Some(SharedCfgPad::Data(26)),
                    4 => Some(SharedCfgPad::Data(24)),
                    5 => Some(SharedCfgPad::Data(22)),
                    6 => Some(SharedCfgPad::Data(20)),
                    7 => Some(SharedCfgPad::Data(18)),
                    8 => Some(SharedCfgPad::Data(16)),
                    9 => Some(SharedCfgPad::Data(14)),
                    12 => Some(SharedCfgPad::Data(29)),
                    13 => Some(SharedCfgPad::Data(27)),
                    14 => Some(SharedCfgPad::Data(25)),
                    15 => Some(SharedCfgPad::Data(23)),
                    16 => Some(SharedCfgPad::Data(21)),
                    17 => Some(SharedCfgPad::Data(19)),
                    18 => Some(SharedCfgPad::Data(17)),
                    19 => Some(SharedCfgPad::Data(15)),
                    20 => Some(SharedCfgPad::Data(13)),
                    21 => Some(SharedCfgPad::Data(12)),
                    22 => Some(SharedCfgPad::EmCclk),
                    24 => Some(SharedCfgPad::Data(8)),
                    25 => Some(SharedCfgPad::Data(7)),
                    26 => Some(SharedCfgPad::Data(5)),
                    27 => Some(SharedCfgPad::Busy),
                    28 => Some(SharedCfgPad::Fcs1B),
                    29 => Some(SharedCfgPad::CsiB),
                    30 => Some(SharedCfgPad::I2cSda),
                    31 => Some(SharedCfgPad::I2cSclk),
                    32 => Some(SharedCfgPad::PerstN0),
                    33 => Some(SharedCfgPad::SmbAlert),
                    34 => Some(SharedCfgPad::Data(11)),
                    35 => Some(SharedCfgPad::Data(10)),
                    36 => Some(SharedCfgPad::Data(9)),
                    37 => Some(SharedCfgPad::OspiDs),
                    38 => Some(SharedCfgPad::Data(6)),
                    39 => Some(SharedCfgPad::Data(4)),
                    40 => Some(SharedCfgPad::OspiRstB),
                    41 => Some(SharedCfgPad::OspiEccFail),
                    _ => None,
                } {
                    die_cfg_io.insert(
                        cfg,
                        IoCoord::HdioLc(HdioCoord {
                            cell,
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
