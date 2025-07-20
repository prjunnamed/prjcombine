use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::DirH;
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, ExpandedGrid, RowId};
use std::collections::{BTreeSet, HashMap};
use unnamed_entity::{EntityBitVec, EntityId, EntityIds, EntityVec};

use crate::chip::{
    Chip, CleKind, ColumnKind, DisabledPart, GtRowKind, HardRowKind, Interposer, RightKind,
};
use crate::expanded::{ExpandedDevice, REGION_LEAF, SllConns, UbumpId};

struct DieInfo {
    col_cfrm: ColId,
    ps_height: usize,
}

struct Expander<'a> {
    chips: EntityVec<DieId, &'a Chip>,
    disabled: BTreeSet<DisabledPart>,
    egrid: ExpandedGrid<'a>,
    die: EntityVec<DieId, DieInfo>,
}

impl Expander<'_> {
    fn in_int_hole(&self, cell: CellCoord) -> bool {
        self.chips[cell.die].in_int_hole(cell.col, cell.row)
    }

    fn fill_die(&mut self) {
        for (_, &chip) in &self.chips {
            self.egrid
                .add_die(chip.columns.len(), chip.regs * Chip::ROWS_PER_REG);
            self.die.push(DieInfo {
                col_cfrm: chip.col_cfrm(),
                ps_height: chip.get_ps_height(),
            });
        }
    }

    fn fill_int(&mut self) {
        for (die, chip) in &self.chips {
            let di = &self.die[die];

            let col_l = self.egrid.cols(die).next().unwrap();
            let col_r = self.egrid.cols(die).next_back().unwrap();
            let row_b = self.egrid.rows(die).next().unwrap();
            let row_t = self.egrid.rows(die).next_back().unwrap();
            let ps_width = di.col_cfrm.to_idx();
            for col in self.egrid.cols(die) {
                if self.disabled.contains(&DisabledPart::Column(die, col)) {
                    continue;
                }
                for cell in self.egrid.column(die, col) {
                    let reg = chip.row_to_reg(cell.row);
                    if self.disabled.contains(&DisabledPart::Region(die, reg)) {
                        continue;
                    }
                    if col < di.col_cfrm && cell.row.to_idx() < di.ps_height {
                        continue;
                    }
                    if chip.col_side(col) == DirH::W {
                        self.egrid.add_tile_e(cell, "INT", 2);
                        if cell.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG)
                            && chip.is_reg_n(reg)
                        {
                            self.egrid.add_tile_e(cell, "RCLK", 2);
                        }
                    }
                }
            }

            for cell in self.egrid.die_cells(die) {
                if cell.col == chip.columns.last_id().unwrap() {
                    continue;
                }
                if self.in_int_hole(cell) || self.in_int_hole(cell.delta(1, 0)) {
                    continue;
                }
                self.egrid
                    .fill_conn_pair(cell, cell.delta(1, 0), "MAIN.E", "MAIN.W");
                if cell.col == chip.columns.last_id().unwrap() - 1 {
                    continue;
                }
                if chip.col_side(cell.col) == DirH::W {
                    self.egrid
                        .fill_conn_pair(cell, cell.delta(2, 0), "MAIN.LE", "MAIN.LW");
                }
            }

            for cell in self.egrid.die_cells(die) {
                if cell.row == chip.rows().next_back().unwrap() {
                    continue;
                }
                if self.in_int_hole(cell) || self.in_int_hole(cell.delta(0, 1)) {
                    continue;
                }
                self.egrid
                    .fill_conn_pair(cell, cell.delta(0, 1), "MAIN.N", "MAIN.S");
            }

            if di.ps_height != chip.regs * Chip::ROWS_PER_REG {
                for dx in 0..ps_width {
                    let cell =
                        CellCoord::new(die, ColId::from_idx(dx), RowId::from_idx(di.ps_height));
                    self.egrid.fill_conn_term(cell, "TERM.S");
                }
            }
            for dy in 0..di.ps_height {
                let cell = CellCoord::new(die, di.col_cfrm, RowId::from_idx(dy));
                self.egrid.fill_conn_term(cell, "TERM.W");
                self.egrid.fill_conn_term(cell, "TERM.LW");
            }

            for cell in self.egrid.row(die, row_b) {
                if cell.col >= di.col_cfrm {
                    self.egrid.fill_conn_term(cell, "TERM.S");
                }
            }
            for cell in self.egrid.row(die, row_t) {
                self.egrid.fill_conn_term(cell, "TERM.N");
            }
            for cell in self.egrid.column(die, col_l) {
                if cell.row.to_idx() >= di.ps_height {
                    self.egrid.fill_conn_term(cell, "TERM.W");
                    self.egrid.fill_conn_term(cell, "TERM.LW");
                }
            }
            for cell in self.egrid.column(die, col_r) {
                self.egrid.fill_conn_term(cell, "TERM.E");
                self.egrid.fill_conn_term(cell.delta(-1, 0), "TERM.LE");
            }
        }
    }

    fn fill_cle_bc(&mut self) {
        for (die, chip) in &self.chips {
            let row_b = self.egrid.rows(die).next().unwrap();
            let row_t = self.egrid.rows(die).next_back().unwrap();
            for (col, &cd) in &chip.columns {
                if matches!(cd.kind, ColumnKind::Cle(_)) && chip.col_side(col) == DirH::E {
                    for cell in self.egrid.column(die, col) {
                        if self.in_int_hole(cell) {
                            continue;
                        }
                        let has_bli = if cell.row < row_b + 4 {
                            cd.has_bli_s
                        } else if cell.row > row_t - 4 {
                            cd.has_bli_n
                        } else {
                            false
                        };
                        let kind = match cd.kind {
                            ColumnKind::Cle(CleKind::Plain) => "CLE_BC",
                            ColumnKind::Cle(CleKind::Sll) => {
                                if has_bli {
                                    "CLE_BC"
                                } else {
                                    "CLE_BC.SLL"
                                }
                            }
                            ColumnKind::Cle(CleKind::Sll2) => {
                                if has_bli {
                                    "CLE_BC"
                                } else {
                                    "CLE_BC.SLL2"
                                }
                            }
                            _ => unreachable!(),
                        };
                        self.egrid.add_tile_e(cell, kind, 2);
                        if has_bli {
                            self.egrid.fill_conn_term(cell, "CLE.BLI.E");
                            self.egrid.fill_conn_term(cell.delta(1, 0), "CLE.BLI.W");
                        } else {
                            self.egrid.fill_conn_term(cell, "CLE.E");
                            self.egrid.fill_conn_term(cell.delta(1, 0), "CLE.W");
                        }
                        let reg = chip.row_to_reg(cell.row);
                        if cell.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG) {
                            let cell = cell.delta(1, 0);
                            if chip.is_reg_half(reg) {
                                self.egrid.add_tile_single(cell, "RCLK_CLE.HALF");
                            } else if chip.is_reg_n(reg) {
                                self.egrid
                                    .add_tile(cell, "RCLK_CLE", &[cell, cell.delta(0, -1)]);
                            }
                        }
                    }
                }
            }
        }
    }

    fn fill_intf(&mut self) {
        for (die, chip) in &self.chips {
            let di = &self.die[die];

            let row_b = self.egrid.rows(die).next().unwrap();
            let row_t = self.egrid.rows(die).next_back().unwrap();
            for (col, &cd) in &chip.columns {
                for cell in self.egrid.column(die, col) {
                    if self.in_int_hole(cell) {
                        continue;
                    }

                    let side = chip.col_side(col);
                    if !matches!(cd.kind, ColumnKind::Cle(_) | ColumnKind::None) {
                        let kind = match cd.kind {
                            ColumnKind::Gt => format!("INTF.{side}.TERM.GT"),
                            ColumnKind::Cfrm => {
                                if cell.row.to_idx() < di.ps_height {
                                    format!("INTF.{side}.TERM.PSS")
                                } else {
                                    format!("INTF.{side}.PSS")
                                }
                            }
                            ColumnKind::Hard => {
                                let ch = chip.get_col_hard(col).unwrap();
                                match ch.regs[chip.row_to_reg(cell.row)] {
                                    HardRowKind::Hdio => format!("INTF.{side}.HDIO"),
                                    _ => format!("INTF.{side}.HB"),
                                }
                            }
                            ColumnKind::ContHard => {
                                let ch = chip.get_col_hard(col - 1).unwrap();
                                match ch.regs[chip.row_to_reg(cell.row)] {
                                    HardRowKind::Hdio => format!("INTF.{side}.HDIO"),
                                    _ => format!("INTF.{side}.HB"),
                                }
                            }
                            _ => format!("INTF.{side}"),
                        };
                        self.egrid.add_tile_single(cell, &kind);
                    } else if matches!(cd.kind, ColumnKind::Cle(_))
                        && cd.has_bli_s
                        && cell.row < row_b + 4
                    {
                        let idx = cell.row - row_b;
                        self.egrid
                            .add_tile_single(cell, &format!("INTF.BLI_CLE.{side}.S.{idx}"));
                    } else if matches!(cd.kind, ColumnKind::Cle(_))
                        && cd.has_bli_n
                        && cell.row > row_t - 4
                    {
                        let idx = cell.row - (row_t - 3);
                        self.egrid
                            .add_tile_single(cell, &format!("INTF.BLI_CLE.{side}.N.{idx}"));
                    }
                    let reg = chip.row_to_reg(cell.row);
                    if cell.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG)
                        && chip.is_reg_n(reg)
                        && !matches!(cd.kind, ColumnKind::Cle(_) | ColumnKind::None)
                        && !(chip.col_side(col) == DirH::E
                            && matches!(cd.kind, ColumnKind::Gt)
                            && matches!(chip.right, RightKind::Cidb))
                    {
                        if chip.is_reg_half(reg) {
                            self.egrid
                                .add_tile_single(cell, &format!("RCLK_INTF.{side}.HALF"));
                        } else {
                            self.egrid.add_tile(
                                cell,
                                &format!("RCLK_INTF.{side}"),
                                &[cell, cell.delta(0, -1)],
                            );
                        }
                        if matches!(
                            cd.kind,
                            ColumnKind::ContDsp | ColumnKind::Bram(_) | ColumnKind::Uram
                        ) {
                            self.egrid
                                .add_tile_single(cell, &format!("RCLK_DFX.{side}"));
                        }
                        if cd.kind == ColumnKind::Hard {
                            let hc = chip.get_col_hard(col).unwrap();
                            if hc.regs[reg] == HardRowKind::Hdio {
                                self.egrid.add_tile(cell, "RCLK_HDIO", &[]);
                            } else if !reg.to_idx().is_multiple_of(2)
                                && hc.regs[reg - 1] == HardRowKind::Hdio
                            {
                                self.egrid.add_tile(cell, "RCLK_HB_HDIO", &[]);
                            }
                        }
                    }
                }
            }
        }
    }

    fn fill_cle(&mut self) {
        for (die, chip) in &self.chips {
            for (col, &cd) in &chip.columns {
                if !matches!(cd.kind, ColumnKind::Cle(_)) {
                    continue;
                }
                for cell in self.egrid.column(die, col) {
                    if cd.has_bli_s && cell.row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_n && cell.row.to_idx() >= self.egrid.rows(die).len() - 4 {
                        continue;
                    }
                    if self.in_int_hole(cell) {
                        continue;
                    }

                    if chip.col_side(col) == DirH::W {
                        self.egrid.add_tile(
                            cell,
                            if chip.is_vr { "CLE_W.VR" } else { "CLE_W" },
                            &[cell, cell.delta(-1, 0)],
                        );
                    } else {
                        self.egrid.add_tile(
                            cell,
                            if chip.is_vr { "CLE_E.VR" } else { "CLE_E" },
                            &[cell],
                        );
                    }
                }
            }
        }
    }

    fn fill_dsp(&mut self) {
        for (die, chip) in &self.chips {
            for (col, &cd) in &chip.columns {
                if cd.kind != ColumnKind::Dsp {
                    continue;
                }
                for cell in self.egrid.column(die, col) {
                    if !cell.row.to_idx().is_multiple_of(2) {
                        continue;
                    }
                    if cd.has_bli_s && cell.row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_n && cell.row.to_idx() >= self.egrid.rows(die).len() - 4 {
                        continue;
                    }
                    if self.in_int_hole(cell) {
                        continue;
                    }
                    self.egrid.add_tile(
                        cell,
                        "DSP",
                        &[
                            cell.delta(0, 0),
                            cell.delta(0, 1),
                            cell.delta(1, 0),
                            cell.delta(1, 1),
                        ],
                    );
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for (die, chip) in &self.chips {
            for (col, &cd) in &chip.columns {
                if !matches!(cd.kind, ColumnKind::Bram(_)) {
                    continue;
                }
                for cell in self.egrid.column(die, col) {
                    if !cell.row.to_idx().is_multiple_of(4) {
                        continue;
                    }
                    if cd.has_bli_s && cell.row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_n && cell.row.to_idx() >= self.egrid.rows(die).len() - 4 {
                        continue;
                    }
                    if self.in_int_hole(cell) {
                        continue;
                    }
                    self.egrid.add_tile_n(
                        cell,
                        if chip.col_side(col) == DirH::W {
                            "BRAM_W"
                        } else {
                            "BRAM_E"
                        },
                        4,
                    );
                }
            }
        }
    }

    fn fill_uram(&mut self) {
        for (die, chip) in &self.chips {
            for (col, &cd) in &chip.columns {
                if cd.kind != ColumnKind::Uram {
                    continue;
                }
                for cell in self.egrid.column(die, col) {
                    if !cell.row.to_idx().is_multiple_of(4) {
                        continue;
                    }
                    if cd.has_bli_s && cell.row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_n && cell.row.to_idx() >= self.egrid.rows(die).len() - 4 {
                        continue;
                    }
                    if self.in_int_hole(cell) {
                        continue;
                    }
                    let reg = chip.row_to_reg(cell.row);
                    self.egrid.add_tile_n(
                        cell,
                        if chip.is_reg_n(reg) && cell.row.to_idx() % Chip::ROWS_PER_REG == 44 {
                            "URAM_DELAY"
                        } else {
                            "URAM"
                        },
                        4,
                    );
                }
            }
        }
    }

    fn fill_hard(&mut self) {
        for (die, chip) in &self.chips {
            for hc in &chip.cols_hard {
                for reg in chip.regs() {
                    if self
                        .disabled
                        .contains(&DisabledPart::HardIp(die, hc.col, reg))
                    {
                        continue;
                    }
                    if self.disabled.contains(&DisabledPart::Region(die, reg)) {
                        continue;
                    }
                    let kind = hc.regs[reg];
                    let (nk, is_high) = match kind {
                        HardRowKind::None => continue,
                        HardRowKind::DcmacT | HardRowKind::IlknT | HardRowKind::HscT => continue,
                        HardRowKind::Hdio => ("HDIO", false),
                        HardRowKind::CpmExt => {
                            // XXX
                            continue;
                        }
                        HardRowKind::Pcie4 => ("PCIE4", false),
                        HardRowKind::Pcie5 => ("PCIE5", false),
                        HardRowKind::Mrmac => ("MRMAC", false),
                        HardRowKind::DcmacB => ("DCMAC", true),
                        HardRowKind::IlknB => ("ILKN", true),
                        HardRowKind::HscB => ("HSC", true),
                        HardRowKind::SdfecA => ("SDFEC", false),
                        HardRowKind::DfeCfcB => ("DFE_CFC_BOT", false),
                        HardRowKind::DfeCfcT => ("DFE_CFC_TOP", false),
                    };
                    let cell = CellCoord::new(die, hc.col, chip.row_reg_bot(reg));
                    let mut tcells = vec![];
                    let height = if is_high {
                        Chip::ROWS_PER_REG * 2
                    } else {
                        Chip::ROWS_PER_REG
                    };
                    tcells.extend(cell.cells_n(height));
                    tcells.extend(cell.delta(1, 0).cells_n(height));
                    self.egrid.add_tile(cell, nk, &tcells);
                }
            }
        }
    }

    fn fill_vnoc(&mut self) {
        for (die, chip) in &self.chips {
            for (col, cd) in &chip.columns {
                if !matches!(
                    cd.kind,
                    ColumnKind::VNoc | ColumnKind::VNoc2 | ColumnKind::VNoc4
                ) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Column(die, col)) {
                    continue;
                }
                for reg in chip.regs() {
                    if self.disabled.contains(&DisabledPart::Region(die, reg)) {
                        continue;
                    }
                    let cell = CellCoord::new(die, col, chip.row_reg_bot(reg));
                    let mut tcells = vec![];
                    tcells.extend(cell.cells_n(Chip::ROWS_PER_REG));
                    tcells.extend(cell.delta(1, 0).cells_n(Chip::ROWS_PER_REG));
                    match cd.kind {
                        ColumnKind::VNoc => {
                            self.egrid.add_tile(cell, "VNOC", &tcells);
                        }
                        ColumnKind::VNoc2 => {
                            self.egrid.add_tile(cell, "VNOC2", &tcells);
                        }
                        ColumnKind::VNoc4 => {
                            self.egrid.add_tile(cell, "VNOC4", &tcells);
                        }
                        _ => unreachable!(),
                    }
                    if chip.is_reg_n(reg) {
                        self.egrid.add_tile(cell.delta(1, 0), "MISR", &tcells);
                    } else {
                        self.egrid.add_tile(cell, "SYSMON_SAT.VNOC", &tcells);
                    }
                }
            }
        }
    }

    fn fill_lgt(&mut self) {
        for (die, chip) in &self.chips {
            let col = self.egrid.cols(die).next().unwrap();
            let ps_height = chip.get_ps_height();
            for reg in chip.regs() {
                let cell = CellCoord::new(die, col, chip.row_reg_bot(reg));
                if cell.row.to_idx() < ps_height {
                    continue;
                }
                self.egrid
                    .add_tile_n(cell, "SYSMON_SAT.LGT", Chip::ROWS_PER_REG);
                self.egrid.add_tile_n(cell, "DPLL.LGT", Chip::ROWS_PER_REG);
                // TODO: actual GT
            }
        }
    }

    fn fill_rgt(&mut self) {
        for (die, chip) in &self.chips {
            let col = self.egrid.cols(die).next_back().unwrap();
            match chip.right {
                RightKind::Gt(ref regs) => {
                    for (reg, &kind) in regs {
                        let cell = CellCoord::new(die, col, chip.row_reg_bot(reg));
                        match kind {
                            GtRowKind::None => (),
                            GtRowKind::Gty => {
                                // TODO
                            }
                            GtRowKind::Gtyp => {
                                // TODO
                            }
                            GtRowKind::Gtm => {
                                // TODO
                            }
                            GtRowKind::RfAdc => {
                                // TODO
                            }
                            GtRowKind::RfDac => {
                                // TODO
                            }
                            GtRowKind::Xram => unreachable!(),
                            GtRowKind::Vdu => {
                                self.egrid.add_tile_n(cell, "VDU.E", Chip::ROWS_PER_REG);
                            }
                            GtRowKind::BfrB => {
                                self.egrid.add_tile_n(cell, "BFR_B.E", Chip::ROWS_PER_REG);
                            }
                            GtRowKind::Isp2 => {
                                // TODO
                            }
                            GtRowKind::Vcu2B => {
                                // TODO
                            }
                            GtRowKind::Vcu2T => {
                                // handled in bottom tile
                            }
                        }
                    }
                }
                RightKind::HNicX => {
                    // TODO
                }
                _ => continue,
            }
            for reg in chip.regs() {
                let cell = CellCoord::new(die, col, chip.row_reg_bot(reg));
                self.egrid
                    .add_tile_n(cell, "SYSMON_SAT.RGT", Chip::ROWS_PER_REG);
                self.egrid.add_tile_n(cell, "DPLL.RGT", Chip::ROWS_PER_REG);
            }
        }
    }

    fn fill_clkroot(&mut self) {
        for (die, chip) in &self.chips {
            for cell in self.egrid.die_cells(die) {
                let reg = chip.row_to_reg(cell.row);
                let crow = if chip.is_reg_n(reg) {
                    chip.row_reg_hclk(reg)
                } else {
                    chip.row_reg_hclk(reg) - 1
                };
                self.egrid[cell].region_root[REGION_LEAF] = cell.with_row(crow);
            }
        }
    }
}

pub fn expand_grid<'a>(
    chips: &EntityVec<DieId, &'a Chip>,
    interposer: &'a Interposer,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedDevice<'a> {
    let mut expander = Expander {
        chips: chips.clone(),
        disabled: disabled.clone(),
        egrid: ExpandedGrid::new(db),
        die: EntityVec::new(),
    };
    expander.fill_die();
    expander.fill_int();
    expander.fill_cle_bc();
    expander.fill_intf();
    expander.fill_cle();
    expander.fill_dsp();
    expander.fill_bram();
    expander.fill_uram();
    expander.fill_hard();
    expander.fill_vnoc();
    expander.fill_lgt();
    expander.fill_rgt();
    expander.fill_clkroot();
    expander.egrid.finish();

    let col_cfrm = expander.die.map_values(|die| die.col_cfrm);

    let mut sll = HashMap::new();
    match interposer.kind {
        crate::chip::InterposerKind::Single => (),
        crate::chip::InterposerKind::Column => {
            fill_sll_column(interposer, chips, &mut sll);
        }
        crate::chip::InterposerKind::MirrorSquare => {
            fill_sll_mirror_square(interposer, chips, &mut sll);
        }
    }

    ExpandedDevice {
        chips: expander.chips,
        interposer,
        egrid: expander.egrid,
        disabled: expander.disabled,
        col_cfrm,
        sll,
    }
}

fn fill_sll_column(
    interposer: &Interposer,
    chips: &EntityVec<DieId, &Chip>,
    sll: &mut HashMap<(DieId, ColId, RowId), SllConns>,
) {
    let mut curse_queue = vec![];
    for (die, cols) in &interposer.sll_columns {
        let chip = chips[die];
        let all_rows = chip.rows();
        let has_link_bot = die != chips.first_id().unwrap();
        let has_link_top = die != chips.last_id().unwrap();
        for (cidx, &col) in cols.iter().enumerate() {
            let start = if chip.columns[col].has_bli_s {
                assert!(!has_link_bot);
                4
            } else {
                0
            };
            let end = if chip.columns[col].has_bli_n {
                assert!(!has_link_top);
                all_rows.len() - 4
            } else {
                all_rows.len()
            };
            let rows: EntityIds<RowId> = EntityIds::new_range(start, end);
            for row in rows {
                let mut conns = SllConns {
                    conns: (0..6).map(|_| None).collect(),
                    cursed: EntityBitVec::repeat(false, 6),
                };
                if has_link_bot && row.to_idx() < start + 75 {
                    let odie = die - 1;
                    let orow = RowId::from_idx(chips[odie].rows().len() - 75 + row.to_idx());
                    let ocol = interposer.sll_columns[odie][cidx];
                    for (bump, obump) in [(0, 0), (1, 1), (3, 2), (4, 4), (5, 5)] {
                        let bump = UbumpId::from_idx(bump);
                        let obump = UbumpId::from_idx(obump);
                        conns.conns[bump] = Some((odie, ocol, orow, obump));
                    }
                    let bump = UbumpId::from_idx(2);
                    let obump = UbumpId::from_idx(3);
                    let orow = row + 75;
                    conns.conns[bump] = Some((die, col, orow, obump));
                } else if has_link_top && row.to_idx() >= end - 75 {
                    let odie = die + 1;
                    let orow = RowId::from_idx(row.to_idx() - (end - 75));
                    let ocol = interposer.sll_columns[odie][cidx];
                    for (bump, obump) in [(0, 0), (1, 1), (2, 3), (4, 4), (5, 5)] {
                        let bump = UbumpId::from_idx(bump);
                        let obump = UbumpId::from_idx(obump);
                        conns.conns[bump] = Some((odie, ocol, orow, obump));
                    }
                    let bump = UbumpId::from_idx(3);
                    let obump = UbumpId::from_idx(2);
                    let orow = row - 75;
                    conns.conns[bump] = Some((die, col, orow, obump));
                } else {
                    if row.to_idx() < start + 75 {
                        let bump = UbumpId::from_idx(3);
                        let triad = (row.to_idx() - start) / 3;
                        let sub = (row.to_idx() - start) % 3;
                        let orow = RowId::from_idx(start + (24 - triad) * 3 + sub);
                        if orow != row {
                            conns.conns[bump] = Some((die, col, orow, bump));
                        }
                    } else {
                        let bump = UbumpId::from_idx(3);
                        let obump = UbumpId::from_idx(2);
                        let orow = row - 75;
                        conns.conns[bump] = Some((die, col, orow, obump));
                    }
                    if row.to_idx() >= end - 75 {
                        let bump = UbumpId::from_idx(2);
                        let triad = (row.to_idx() - (end - 75)) / 3;
                        let sub = (row.to_idx() - (end - 75)) % 3;
                        let orow = RowId::from_idx(end - 75 + (24 - triad) * 3 + sub);
                        if orow != row {
                            conns.conns[bump] = Some((die, col, orow, bump));
                        }
                    } else {
                        let bump = UbumpId::from_idx(2);
                        let obump = UbumpId::from_idx(3);
                        let orow = row + 75;
                        conns.conns[bump] = Some((die, col, orow, obump));
                    }
                    if cidx < 10 {
                        for bump in [1, 5] {
                            let bump = UbumpId::from_idx(bump);
                            let ocol = cols[9 - cidx];
                            conns.conns[bump] = Some((die, ocol, row, bump));
                        }
                    } else {
                        for (bump, obump) in [(1, 0), (5, 4)] {
                            let bump = UbumpId::from_idx(bump);
                            let obump = UbumpId::from_idx(obump);
                            let ocol = cols[cidx - 10];
                            conns.conns[bump] = Some((die, ocol, row, obump));
                        }
                    }
                    if cidx >= cols.len() - 10 {
                        for bump in [0, 4] {
                            let bump = UbumpId::from_idx(bump);
                            let ocol = cols[cols.len() - 10 + (9 - (cidx - (cols.len() - 10)))];
                            conns.conns[bump] = Some((die, ocol, row, bump));
                        }
                    } else {
                        for (bump, obump) in [(0, 1), (4, 5)] {
                            let bump = UbumpId::from_idx(bump);
                            let obump = UbumpId::from_idx(obump);
                            let ocol = cols[cidx + 10];
                            conns.conns[bump] = Some((die, ocol, row, obump));
                        }
                    }
                }
                sll.insert((die, col, row), conns);
            }
            curse_queue.push((die, col, RowId::from_idx(start)));
            if has_link_top {
                curse_queue.push((die, col, RowId::from_idx(end - 75)));
            }
            if has_link_bot {
                curse_queue.push((die, col, RowId::from_idx(start + 75)));
            }
        }
    }
    for (die, col, row) in curse_queue {
        let conns = sll.get_mut(&(die, col, row)).unwrap();
        for mut val in conns.cursed.values_mut() {
            *val = true;
        }
        for (odie, ocol, orow, ubump) in conns.conns.clone().into_values().flatten() {
            let conns = sll.get_mut(&(odie, ocol, orow)).unwrap();
            conns.cursed.set(ubump, true);
        }
    }
}

fn fill_sll_mirror_square(
    interposer: &Interposer,
    chips: &EntityVec<DieId, &Chip>,
    sll: &mut HashMap<(DieId, ColId, RowId), SllConns>,
) {
    let mut curse_queue = vec![];
    for (die, cols) in &interposer.sll_columns {
        let chip = chips[die];
        let all_rows = chip.rows();
        let col_cfrm = chip.col_cfrm();
        let ps_height = chip.get_ps_height();
        let cidx_ps = cols.binary_search(&col_cfrm).unwrap_err();
        for (cidx, &col) in cols.iter().enumerate() {
            let start = if col < col_cfrm {
                ps_height
            } else if chip.columns[col].has_bli_s {
                4
            } else {
                0
            };
            let end = all_rows.len();
            let rows: EntityIds<RowId> = EntityIds::new_range(start, end);
            for row in rows.clone() {
                let cidx_l = if row.to_idx() < ps_height { cidx_ps } else { 0 };
                let mut conns = SllConns {
                    conns: (0..6).map(|_| None).collect(),
                    cursed: EntityBitVec::repeat(false, 6),
                };
                if row == RowId::from_idx(end - 63) {
                    // do nothing
                } else if cidx < cols.len() - 10 {
                    if row < RowId::from_idx(end - 63) {
                        if row.to_idx() < start + 75 {
                            let bump = UbumpId::from_idx(3);
                            let triad = (row.to_idx() - start) / 3;
                            let sub = (row.to_idx() - start) % 3;
                            let orow = RowId::from_idx(start + (24 - triad) * 3 + sub);
                            if orow != row {
                                conns.conns[bump] = Some((die, col, orow, bump));
                            }
                        } else {
                            let bump = UbumpId::from_idx(3);
                            let obump = UbumpId::from_idx(2);
                            let orow = row - 75;
                            conns.conns[bump] = Some((die, col, orow, obump));
                        }
                        if row.to_idx() >= end - 63 - 75 {
                            // nothing
                        } else {
                            let bump = UbumpId::from_idx(2);
                            let obump = UbumpId::from_idx(3);
                            let orow = row + 75;
                            conns.conns[bump] = Some((die, col, orow, obump));
                        }
                        if cidx < cidx_l + 10 {
                            for bump in [1, 5] {
                                let bump = UbumpId::from_idx(bump);
                                let ocol = cols[cidx_l + 9 - (cidx - cidx_l)];
                                conns.conns[bump] = Some((die, ocol, row, bump));
                            }
                        } else {
                            for (bump, obump) in [(1, 0), (5, 4)] {
                                let bump = UbumpId::from_idx(bump);
                                let obump = UbumpId::from_idx(obump);
                                let ocol = cols[cidx - 10];
                                conns.conns[bump] = Some((die, ocol, row, obump));
                            }
                        }
                        for (bump, obump) in [(0, 1), (4, 5)] {
                            let bump = UbumpId::from_idx(bump);
                            let obump = UbumpId::from_idx(obump);
                            let ocol = cols[cidx + 10];
                            conns.conns[bump] = Some((die, ocol, row, obump));
                        }
                    } else {
                        for bump in 0..6 {
                            let bump = UbumpId::from_idx(bump);
                            let odie = DieId::from_idx(die.to_idx() ^ 1);
                            let orow = RowId::from_idx(end - 62 + (end - 1 - row.to_idx()));
                            conns.conns[bump] = Some((odie, col, orow, bump));
                        }
                    }
                } else {
                    if !(46..50).contains(&(row.to_idx() % (Chip::ROWS_PER_REG * 2))) {
                        for bump in [0, 2, 4] {
                            let bump = UbumpId::from_idx(bump);
                            let odie = DieId::from_idx(die.to_idx() ^ 3);
                            let ocol = cols[cols.len() - 10 + (cols.len() - 1 - cidx)];
                            conns.conns[bump] = Some((odie, ocol, row, bump));
                        }
                    }
                    if row.to_idx() % (Chip::ROWS_PER_REG * 2) == 50 {
                        for bump in [0, 2, 4] {
                            let bump = UbumpId::from_idx(bump);
                            curse_queue.push((die, col, row, bump));
                        }
                    }
                    if row < RowId::from_idx(end - 63) {
                        for (bump, obump) in [(1, 0), (5, 4)] {
                            let bump = UbumpId::from_idx(bump);
                            let obump = UbumpId::from_idx(obump);
                            let ocol = cols[cidx - 10];
                            conns.conns[bump] = Some((die, ocol, row, obump));
                        }
                        if row.to_idx() % (Chip::ROWS_PER_REG * 2) == 50 {
                            for bump in [1, 5] {
                                let bump = UbumpId::from_idx(bump);
                                curse_queue.push((die, col, row, bump));
                            }
                        }
                    }
                }
                sll.insert((die, col, row), conns);
            }
            for bump in 0..6 {
                let bump = UbumpId::from_idx(bump);
                if cidx < cols.len() - 10 || bump.to_idx() != 3 {
                    curse_queue.push((die, col, RowId::from_idx(start), bump));
                }
                if cidx < cols.len() - 10 || matches!(bump.to_idx(), 0 | 2 | 4) {
                    curse_queue.push((die, col, RowId::from_idx(end - 63), bump));
                }
            }
            if cidx < 10 {
                let row = RowId::from_idx(ps_height);
                for bump in [1, 5] {
                    let bump = UbumpId::from_idx(bump);
                    curse_queue.push((die, col, row, bump));
                }
            }
        }
    }
    for (die, col, row, bump) in curse_queue {
        let conns = sll.get_mut(&(die, col, row)).unwrap();
        conns.cursed.set(bump, true);
        if let Some((odie, ocol, orow, obump)) = conns.conns[bump] {
            let conns = sll.get_mut(&(odie, ocol, orow)).unwrap();
            conns.cursed.set(obump, true);
        }
    }
}
