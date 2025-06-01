use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::DirH;
use prjcombine_interconnect::grid::{ColId, DieId, ExpandedGrid, RowId};
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
        for (dieid, chip) in &self.chips {
            let di = &self.die[dieid];
            let mut die = self.egrid.die_mut(dieid);

            let col_l = die.cols().next().unwrap();
            let col_r = die.cols().next_back().unwrap();
            let row_b = die.rows().next().unwrap();
            let row_t = die.rows().next_back().unwrap();
            let ps_width = di.col_cfrm.to_idx();
            for col in chip.columns.ids() {
                if self.disabled.contains(&DisabledPart::Column(dieid, col)) {
                    continue;
                }
                for row in die.rows() {
                    let reg = chip.row_to_reg(row);
                    if self.disabled.contains(&DisabledPart::Region(dieid, reg)) {
                        continue;
                    }
                    if col < di.col_cfrm && row.to_idx() < di.ps_height {
                        continue;
                    }
                    if chip.col_side(col) == DirH::W {
                        die.add_tile((col, row), "INT", &[(col, row), (col + 1, row)]);
                        if row.to_idx() % Chip::ROWS_PER_REG == 0 && chip.is_reg_n(reg) {
                            die.add_tile((col, row), "RCLK", &[(col, row), (col + 1, row)]);
                        }
                    }
                }
            }

            for col in die.cols() {
                for row in die.rows() {
                    if col == chip.columns.last_id().unwrap() {
                        continue;
                    }
                    if chip.in_int_hole(col, row) || chip.in_int_hole(col + 1, row) {
                        continue;
                    }
                    die.fill_conn_pair((col, row), (col + 1, row), "MAIN.E", "MAIN.W");
                    if col == chip.columns.last_id().unwrap() - 1 {
                        continue;
                    }
                    if chip.col_side(col) == DirH::W {
                        die.fill_conn_pair((col, row), (col + 2, row), "MAIN.LE", "MAIN.LW");
                    }
                }
            }

            for col in die.cols() {
                for row in die.rows() {
                    if row == chip.rows().next_back().unwrap() {
                        continue;
                    }
                    if chip.in_int_hole(col, row) || chip.in_int_hole(col, row + 1) {
                        continue;
                    }
                    die.fill_conn_pair((col, row), (col, row + 1), "MAIN.N", "MAIN.S");
                }
            }

            if di.ps_height != chip.regs * Chip::ROWS_PER_REG {
                let row_t = RowId::from_idx(di.ps_height);
                for dx in 0..ps_width {
                    let col = ColId::from_idx(dx);
                    die.fill_conn_term((col, row_t), "TERM.S");
                }
            }
            for dy in 0..di.ps_height {
                let row = RowId::from_idx(dy);
                die.fill_conn_term((di.col_cfrm, row), "TERM.W");
                die.fill_conn_term((di.col_cfrm, row), "TERM.LW");
            }

            for col in die.cols() {
                if col >= di.col_cfrm {
                    die.fill_conn_term((col, row_b), "TERM.S");
                }
                die.fill_conn_term((col, row_t), "TERM.N");
            }
            for row in die.rows() {
                if row.to_idx() >= di.ps_height {
                    die.fill_conn_term((col_l, row), "TERM.W");
                    die.fill_conn_term((col_l, row), "TERM.LW");
                }
                die.fill_conn_term((col_r, row), "TERM.E");
                die.fill_conn_term((col_r - 1, row), "TERM.LE");
            }
        }
    }

    fn fill_cle_bc(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);

            let row_b = die.rows().next().unwrap();
            let row_t = die.rows().next_back().unwrap();
            for (col, &cd) in &chip.columns {
                if matches!(cd.kind, ColumnKind::Cle(_)) && chip.col_side(col) == DirH::E {
                    for row in die.rows() {
                        if chip.in_int_hole(col, row) {
                            continue;
                        }
                        let has_bli = if row < row_b + 4 {
                            cd.has_bli_s
                        } else if row > row_t - 4 {
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
                        die.add_tile((col, row), kind, &[(col, row), (col + 1, row)]);
                        if has_bli {
                            die.fill_conn_term((col, row), "CLE.BLI.E");
                            die.fill_conn_term((col + 1, row), "CLE.BLI.W");
                        } else {
                            die.fill_conn_term((col, row), "CLE.E");
                            die.fill_conn_term((col + 1, row), "CLE.W");
                        }
                        let reg = chip.row_to_reg(row);
                        if row.to_idx() % Chip::ROWS_PER_REG == 0 {
                            if chip.is_reg_half(reg) {
                                die.add_tile((col + 1, row), "RCLK_CLE.HALF", &[(col + 1, row)]);
                            } else if chip.is_reg_n(reg) {
                                die.add_tile(
                                    (col + 1, row),
                                    "RCLK_CLE",
                                    &[(col + 1, row), (col + 1, row - 1)],
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fn fill_intf(&mut self) {
        for (dieid, chip) in &self.chips {
            let di = &self.die[dieid];
            let mut die = self.egrid.die_mut(dieid);

            let row_b = die.rows().next().unwrap();
            let row_t = die.rows().next_back().unwrap();
            for (col, &cd) in &chip.columns {
                for row in die.rows() {
                    if chip.in_int_hole(col, row) {
                        continue;
                    }

                    let side = chip.col_side(col);
                    if !matches!(cd.kind, ColumnKind::Cle(_) | ColumnKind::None) {
                        let kind = match cd.kind {
                            ColumnKind::Gt => format!("INTF.{side}.TERM.GT"),
                            ColumnKind::Cfrm => {
                                if row.to_idx() < di.ps_height {
                                    format!("INTF.{side}.TERM.PSS")
                                } else {
                                    format!("INTF.{side}.PSS")
                                }
                            }
                            ColumnKind::Hard => {
                                let ch = chip.get_col_hard(col).unwrap();
                                match ch.regs[chip.row_to_reg(row)] {
                                    HardRowKind::Hdio => format!("INTF.{side}.HDIO"),
                                    _ => format!("INTF.{side}.HB"),
                                }
                            }
                            ColumnKind::ContHard => {
                                let ch = chip.get_col_hard(col - 1).unwrap();
                                match ch.regs[chip.row_to_reg(row)] {
                                    HardRowKind::Hdio => format!("INTF.{side}.HDIO"),
                                    _ => format!("INTF.{side}.HB"),
                                }
                            }
                            _ => format!("INTF.{side}"),
                        };
                        die.add_tile((col, row), &kind, &[(col, row)]);
                    } else if matches!(cd.kind, ColumnKind::Cle(_))
                        && cd.has_bli_s
                        && row < row_b + 4
                    {
                        let idx = row - row_b;
                        die.add_tile(
                            (col, row),
                            &format!("INTF.BLI_CLE.{side}.S.{idx}"),
                            &[(col, row)],
                        );
                    } else if matches!(cd.kind, ColumnKind::Cle(_))
                        && cd.has_bli_n
                        && row > row_t - 4
                    {
                        let idx = row - (row_t - 3);
                        die.add_tile(
                            (col, row),
                            &format!("INTF.BLI_CLE.{side}.N.{idx}"),
                            &[(col, row)],
                        );
                    }
                    let reg = chip.row_to_reg(row);
                    if row.to_idx() % Chip::ROWS_PER_REG == 0
                        && chip.is_reg_n(reg)
                        && !matches!(cd.kind, ColumnKind::Cle(_) | ColumnKind::None)
                        && !(chip.col_side(col) == DirH::E
                            && matches!(cd.kind, ColumnKind::Gt)
                            && matches!(chip.right, RightKind::Cidb))
                    {
                        if chip.is_reg_half(reg) {
                            die.add_tile(
                                (col, row),
                                &format!("RCLK_INTF.{side}.HALF"),
                                &[(col, row)],
                            );
                        } else {
                            die.add_tile(
                                (col, row),
                                &format!("RCLK_INTF.{side}"),
                                &[(col, row), (col, row - 1)],
                            );
                        }
                        if matches!(
                            cd.kind,
                            ColumnKind::ContDsp | ColumnKind::Bram(_) | ColumnKind::Uram
                        ) {
                            die.add_tile((col, row), &format!("RCLK_DFX.{side}"), &[(col, row)]);
                        }
                        if cd.kind == ColumnKind::Hard {
                            let hc = chip.get_col_hard(col).unwrap();
                            if hc.regs[reg] == HardRowKind::Hdio {
                                die.add_tile((col, row), "RCLK_HDIO", &[]);
                            } else if reg.to_idx() % 2 != 0 && hc.regs[reg - 1] == HardRowKind::Hdio
                            {
                                die.add_tile((col, row), "RCLK_HB_HDIO", &[]);
                            }
                        }
                    }
                }
            }
        }
    }

    fn fill_cle(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &chip.columns {
                if !matches!(cd.kind, ColumnKind::Cle(_)) {
                    continue;
                }
                for row in die.rows() {
                    if cd.has_bli_s && row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_n && row.to_idx() >= die.rows().len() - 4 {
                        continue;
                    }
                    if chip.in_int_hole(col, row) {
                        continue;
                    }

                    if chip.col_side(col) == DirH::W {
                        die.add_tile(
                            (col, row),
                            if chip.is_vr { "CLE_W.VR" } else { "CLE_W" },
                            &[(col, row), (col - 1, row)],
                        );
                    } else {
                        die.add_tile(
                            (col, row),
                            if chip.is_vr { "CLE_E.VR" } else { "CLE_E" },
                            &[(col, row)],
                        );
                    }
                }
            }
        }
    }

    fn fill_dsp(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &chip.columns {
                if cd.kind != ColumnKind::Dsp {
                    continue;
                }
                for row in die.rows() {
                    if row.to_idx() % 2 != 0 {
                        continue;
                    }
                    if cd.has_bli_s && row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_n && row.to_idx() >= die.rows().len() - 4 {
                        continue;
                    }
                    if chip.in_int_hole(col, row) {
                        continue;
                    }
                    die.add_tile(
                        (col, row),
                        "DSP",
                        &[
                            (col, row),
                            (col, row + 1),
                            (col + 1, row),
                            (col + 1, row + 1),
                        ],
                    );
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &chip.columns {
                if !matches!(cd.kind, ColumnKind::Bram(_)) {
                    continue;
                }
                for row in die.rows() {
                    if row.to_idx() % 4 != 0 {
                        continue;
                    }
                    if cd.has_bli_s && row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_n && row.to_idx() >= die.rows().len() - 4 {
                        continue;
                    }
                    if chip.in_int_hole(col, row) {
                        continue;
                    }
                    die.add_tile(
                        (col, row),
                        if chip.col_side(col) == DirH::W {
                            "BRAM_W"
                        } else {
                            "BRAM_E"
                        },
                        &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                    );
                }
            }
        }
    }

    fn fill_uram(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);
            for (col, &cd) in &chip.columns {
                if cd.kind != ColumnKind::Uram {
                    continue;
                }
                for row in die.rows() {
                    if row.to_idx() % 4 != 0 {
                        continue;
                    }
                    if cd.has_bli_s && row.to_idx() < 4 {
                        continue;
                    }
                    if cd.has_bli_n && row.to_idx() >= die.rows().len() - 4 {
                        continue;
                    }
                    if chip.in_int_hole(col, row) {
                        continue;
                    }
                    let reg = chip.row_to_reg(row);
                    die.add_tile(
                        (col, row),
                        if chip.is_reg_n(reg) && row.to_idx() % Chip::ROWS_PER_REG == 44 {
                            "URAM_DELAY"
                        } else {
                            "URAM"
                        },
                        &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                    );
                }
            }
        }
    }

    fn fill_hard(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);
            for hc in &chip.cols_hard {
                for reg in chip.regs() {
                    if self
                        .disabled
                        .contains(&DisabledPart::HardIp(die.die, hc.col, reg))
                    {
                        continue;
                    }
                    if self.disabled.contains(&DisabledPart::Region(die.die, reg)) {
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
                    let row = chip.row_reg_bot(reg);
                    let mut crd = vec![];
                    let height = if is_high {
                        Chip::ROWS_PER_REG * 2
                    } else {
                        Chip::ROWS_PER_REG
                    };
                    for i in 0..height {
                        crd.push((hc.col, row + i));
                    }
                    for i in 0..height {
                        crd.push((hc.col + 1, row + i));
                    }
                    die.add_tile((hc.col, row), nk, &crd);
                }
            }
        }
    }

    fn fill_vnoc(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);
            for (col, cd) in &chip.columns {
                if !matches!(
                    cd.kind,
                    ColumnKind::VNoc | ColumnKind::VNoc2 | ColumnKind::VNoc4
                ) {
                    continue;
                }
                if self.disabled.contains(&DisabledPart::Column(die.die, col)) {
                    continue;
                }
                for reg in chip.regs() {
                    if self.disabled.contains(&DisabledPart::Region(die.die, reg)) {
                        continue;
                    }
                    let row = chip.row_reg_bot(reg);
                    let mut crd = vec![];
                    for i in 0..Chip::ROWS_PER_REG {
                        crd.push((col, row + i));
                    }
                    for i in 0..Chip::ROWS_PER_REG {
                        crd.push((col + 1, row + i));
                    }
                    match cd.kind {
                        ColumnKind::VNoc => {
                            die.add_tile((col, row), "VNOC", &crd);
                        }
                        ColumnKind::VNoc2 => {
                            die.add_tile((col, row), "VNOC2", &crd);
                        }
                        ColumnKind::VNoc4 => {
                            die.add_tile((col, row), "VNOC4", &crd);
                        }
                        _ => unreachable!(),
                    }
                    if chip.is_reg_n(reg) {
                        die.add_tile((col + 1, row), "MISR", &crd);
                    } else {
                        die.add_tile((col, row), "SYSMON_SAT.VNOC", &crd);
                    }
                }
            }
        }
    }

    fn fill_lgt(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);
            let col = die.cols().next().unwrap();
            let ps_height = chip.get_ps_height();
            for reg in chip.regs() {
                let row = chip.row_reg_bot(reg);
                if row.to_idx() < ps_height {
                    continue;
                }
                let crds: [_; Chip::ROWS_PER_REG] = core::array::from_fn(|dy| (col, row + dy));
                die.add_tile(crds[0], "SYSMON_SAT.LGT", &crds);
                die.add_tile(crds[0], "DPLL.LGT", &crds);
                // TODO: actual GT
            }
        }
    }

    fn fill_rgt(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);
            let col = die.cols().next_back().unwrap();
            match chip.right {
                RightKind::Gt(ref regs) => {
                    for (reg, &kind) in regs {
                        let row = chip.row_reg_bot(reg);
                        let crds: [_; Chip::ROWS_PER_REG] =
                            core::array::from_fn(|dy| (col, row + dy));
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
                                die.add_tile(crds[0], "VDU.E", &crds);
                            }
                            GtRowKind::BfrB => {
                                die.add_tile(crds[0], "BFR_B.E", &crds);
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
                let row = chip.row_reg_bot(reg);
                let crds: [_; Chip::ROWS_PER_REG] = core::array::from_fn(|dy| (col, row + dy));
                die.add_tile(crds[0], "SYSMON_SAT.RGT", &crds);
                die.add_tile(crds[0], "DPLL.RGT", &crds);
            }
        }
    }

    fn fill_clkroot(&mut self) {
        for (dieid, chip) in &self.chips {
            let mut die = self.egrid.die_mut(dieid);

            for col in die.cols() {
                for row in die.rows() {
                    let reg = chip.row_to_reg(row);
                    let crow = if chip.is_reg_n(reg) {
                        chip.row_reg_hclk(reg)
                    } else {
                        chip.row_reg_hclk(reg) - 1
                    };
                    die[(col, row)].region_root[REGION_LEAF] = (col, crow);
                }
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
