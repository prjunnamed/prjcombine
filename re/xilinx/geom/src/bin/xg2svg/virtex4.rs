use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId};
use prjcombine_virtex4::chip::{CfgRowKind, ChipKind, ColumnKind, GtKind, RegId};
use prjcombine_virtex4::expanded::ExpandedDevice;

use crate::drawer::Drawer;

const W_CLB: f64 = 10.;
const W_BRAM: f64 = 40.;
const W_IO: f64 = 80.;
const W_GT: f64 = 160.;
const W_CMT: f64 = 40.;
const W_FIFO: f64 = 20.;
const W_SPINE: f64 = 10.;
const W_TERM: f64 = 4.;
const W_BRK: f64 = 2.;
const H_TERM: f64 = 4.;
const H_CLB: f64 = 8.;
const H_HCLK: f64 = 2.;
const H_BRKH: f64 = 2.;

pub fn draw_device(name: &str, edev: ExpandedDevice) -> Drawer {
    let fgrid = edev.chips.first().unwrap();
    let mut x = 0.;
    let mut col_x = EntityVec::new();
    x += W_TERM;
    for (col, &cd) in &fgrid.columns {
        if fgrid.cols_vbrk.contains(&col) {
            x += W_BRK;
        }
        let w = match cd {
            ColumnKind::ClbLL | ColumnKind::ClbLM | ColumnKind::Clk | ColumnKind::Dsp => W_CLB,
            ColumnKind::Bram => W_BRAM,
            ColumnKind::Io => W_IO,
            ColumnKind::Cfg => match edev.kind {
                ChipKind::Virtex7 => W_CLB,
                ChipKind::Virtex6 => W_CMT,
                _ => W_IO,
            },
            ColumnKind::Cmt => W_CMT,
            ColumnKind::Gt => W_GT,
        };
        col_x.push((x, x + w));
        x += w;
        if cd == ColumnKind::Cfg && matches!(edev.kind, ChipKind::Virtex4 | ChipKind::Virtex5) {
            x += W_SPINE;
        }
    }
    x += W_TERM;
    let width = x;

    let mut die_y: EntityVec<DieId, _> = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut y = 0.;
    for (_, grid) in &edev.chips {
        let term_y_b = y;
        let mut die_row_y = EntityVec::new();
        y += H_TERM;
        for row in grid.rows() {
            if row.to_idx().is_multiple_of(fgrid.rows_per_reg()) {
                y += H_BRKH;
            }
            if row.to_idx() % fgrid.rows_per_reg() == fgrid.rows_per_reg() / 2 {
                y += H_HCLK;
            }
            die_row_y.push((y, y + H_CLB));
            y += H_CLB;
        }
        y += H_TERM;
        row_y.push(die_row_y);
        die_y.push((term_y_b, y));
    }
    let height = y;

    let mut drawer = Drawer::new(name.to_string(), width, height);
    drawer.bel_class("clbll", "#00cc00");
    drawer.bel_class("clblm", "#00ff00");
    drawer.bel_class("bram", "#5555ff");
    drawer.bel_class("dsp", "#00aaaa");
    drawer.bel_class("fifo", "#8080ff");
    drawer.bel_class("io", "#ff00ff");
    drawer.bel_class("ppc", "#ff0000");
    drawer.bel_class("emac", "#ff0000");
    drawer.bel_class("pcie2", "#ff0000");
    drawer.bel_class("pcie3", "#cc0000");
    drawer.bel_class("cfg", "#ff8000");
    drawer.bel_class("sysmon", "#aa00aa");
    drawer.bel_class("pmv", "#ff8000");
    drawer.bel_class("dcm", "#aaaa00");
    drawer.bel_class("ccm", "#ffff00");
    drawer.bel_class("pll", "#ffff00");
    drawer.bel_class("mmcm", "#aaaa00");
    drawer.bel_class("phaser", "#ff8080");
    drawer.bel_class("gtp", "#c000ff");
    drawer.bel_class("gtx", "#8000ff");
    drawer.bel_class("gth", "#4000ff");
    drawer.bel_class("gtz", "#4000c0");
    drawer.bel_class("bufg", "#aa5500");
    drawer.bel_class("bufh", "#ffaa00");

    let bram_rows = if edev.kind == ChipKind::Virtex4 { 4 } else { 5 };
    for (die, grid) in &edev.chips {
        for (col, &cd) in &grid.columns {
            match cd {
                ColumnKind::ClbLL | ColumnKind::ClbLM => {
                    let kind = if cd == ColumnKind::ClbLL {
                        "clbll"
                    } else {
                        "clblm"
                    };
                    for row in grid.rows() {
                        if edev.in_site_hole(CellCoord::new(die, col, row)) {
                            continue;
                        }
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][row].0,
                            row_y[die][row].1,
                            kind,
                        )
                    }
                }
                ColumnKind::Bram | ColumnKind::Dsp => {
                    let kind = if cd == ColumnKind::Bram {
                        "bram"
                    } else {
                        "dsp"
                    };
                    for row in grid.rows().step_by(bram_rows) {
                        if edev.in_site_hole(CellCoord::new(die, col, row)) {
                            continue;
                        }
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][row].0,
                            row_y[die][row + bram_rows - 1].1,
                            kind,
                        )
                    }
                }
                ColumnKind::Cfg if edev.kind == ChipKind::Virtex7 => {
                    for reg in grid.regs() {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][grid.row_reg_bot(reg)].0,
                            row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                            "cfg",
                        );
                    }
                }
                ColumnKind::Cfg if edev.kind == ChipKind::Virtex6 => {
                    for reg in grid.regs() {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][grid.row_reg_bot(reg) + 2].0,
                            row_y[die][grid.row_reg_hclk(reg) - 1].1,
                            "mmcm",
                        );
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][grid.row_reg_hclk(reg)].0,
                            row_y[die][grid.row_reg_bot(reg + 1) - 3].1,
                            "mmcm",
                        );
                        if reg == grid.reg_cfg {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][grid.row_reg_bot(reg)].0,
                                row_y[die][grid.row_reg_bot(reg) + 1].1,
                                "bufg",
                            );
                        }
                        if reg == grid.reg_cfg - 1 {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][grid.row_reg_bot(reg + 1) - 2].0,
                                row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                                "bufg",
                            );
                        }
                        if reg >= grid.reg_cfg {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][grid.row_reg_bot(reg + 1) - 2].0,
                                row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                                "pmv",
                            );
                        }
                        if reg < grid.reg_cfg - 1 {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][grid.row_reg_bot(reg)].0,
                                row_y[die][grid.row_reg_bot(reg) + 1].1,
                                "pmv",
                            );
                        }
                    }
                    // XXX
                }
                ColumnKind::Cmt => {
                    for reg in grid.regs() {
                        let row = grid.row_reg_bot(reg);
                        if edev.in_site_hole(CellCoord::new(die, col, row)) {
                            continue;
                        }
                        let (fl, fr, cl, cr) = if col.to_idx() % 2 == 1 {
                            (
                                col_x[col].0,
                                col_x[col].0 + W_FIFO,
                                col_x[col].0 + W_FIFO,
                                col_x[col].1,
                            )
                        } else {
                            (
                                col_x[col].1 - W_FIFO,
                                col_x[col].1,
                                col_x[col].0,
                                col_x[col].1 - W_FIFO,
                            )
                        };
                        for i in 0..4 {
                            drawer.bel_rect(
                                fl,
                                fr,
                                row_y[die][row + 1 + i * 12].0,
                                row_y[die][row + 1 + i * 12 + 11].1,
                                "fifo",
                            );
                        }
                        for (dy, h, kind) in [
                            (0, 16, "mmcm"),
                            (16, 9, "phaser"),
                            (25, 12, "phaser"),
                            (37, 13, "pll"),
                        ] {
                            drawer.bel_rect(
                                cl,
                                cr,
                                row_y[die][row + dy].0,
                                row_y[die][row + dy + h - 1].1,
                                kind,
                            )
                        }
                    }
                }
                ColumnKind::Io | ColumnKind::Cfg => {
                    for row in grid.rows() {
                        if edev.in_site_hole(CellCoord::new(die, col, row)) {
                            continue;
                        }
                        let h = match edev.kind {
                            ChipKind::Virtex6 => {
                                if row.to_idx() % 2 == 1 {
                                    continue;
                                }
                                2
                            }
                            ChipKind::Virtex7 => {
                                if matches!(row.to_idx() % 50, 0 | 49) {
                                    1
                                } else if row.to_idx().is_multiple_of(2) {
                                    continue;
                                } else {
                                    2
                                }
                            }
                            _ => 1,
                        };
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][row].0,
                            row_y[die][row + h - 1].1,
                            "io",
                        )
                    }
                }
                ColumnKind::Clk => {
                    for reg in grid.regs() {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][grid.row_reg_hclk(reg) - 4].0,
                            row_y[die][grid.row_reg_hclk(reg) + 3].1,
                            "bufh",
                        );

                        if reg == grid.reg_clk {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][grid.row_reg_bot(reg)].0,
                                row_y[die][grid.row_reg_bot(reg) + 3].1,
                                "bufg",
                            );
                        }
                        if reg == grid.reg_clk - 1 {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][grid.row_reg_bot(reg + 1) - 4].0,
                                row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                                "bufg",
                            );
                            for (dy, h) in [
                                (if grid.regs == 1 { 0 } else { 1 }, 7), // PMV
                                (17, 1),                                 // PMVIOB
                                (32, 1),                                 // PMV2_SVT
                                (41, 1),                                 // PMV2
                                (45, 1),                                 // MTBF2
                            ] {
                                drawer.bel_rect(
                                    col_x[col].0,
                                    col_x[col].1,
                                    row_y[die][grid.row_reg_bot(reg) + dy].0,
                                    row_y[die][grid.row_reg_bot(reg) + dy + h - 1].1,
                                    "pmv",
                                );
                            }
                        }
                    }
                }
                _ => (),
            }
        }
        for &(col, row) in &grid.holes_ppc {
            let (w, h) = match edev.kind {
                ChipKind::Virtex4 => (9, 24),
                ChipKind::Virtex5 => (14, 40),
                _ => unreachable!(),
            };
            drawer.bel_rect(
                col_x[col].0,
                col_x[col + w - 1].1,
                row_y[die][row].0,
                row_y[die][row + h - 1].1,
                "ppc",
            );
        }
        for pcie2 in &grid.holes_pcie2 {
            drawer.bel_rect(
                col_x[pcie2.col].0,
                col_x[pcie2.col + 3].1,
                row_y[die][pcie2.row].0,
                row_y[die][pcie2.row + 24].1,
                "pcie2",
            );
        }
        for &(col, row) in &grid.holes_pcie3 {
            drawer.bel_rect(
                col_x[col].0,
                col_x[col + 5].1,
                row_y[die][row].0,
                row_y[die][row + 49].1,
                "pcie3",
            );
        }
        if let Some(ref hc) = grid.col_hard {
            for &row in &hc.rows_emac {
                let height = match edev.kind {
                    ChipKind::Virtex5 => 10,
                    ChipKind::Virtex6 => 10,
                    _ => unreachable!(),
                };
                drawer.bel_rect(
                    col_x[hc.col].0,
                    col_x[hc.col].1,
                    row_y[die][row].0,
                    row_y[die][row + height - 1].1,
                    "emac",
                );
            }
            for &row in &hc.rows_pcie {
                let (w, h) = match edev.kind {
                    ChipKind::Virtex5 => (1, 40),
                    ChipKind::Virtex6 => (4, 20),
                    _ => unreachable!(),
                };
                drawer.bel_rect(
                    col_x[hc.col + 1 - w].0,
                    col_x[hc.col].1,
                    row_y[die][row].0,
                    row_y[die][row + h - 1].1,
                    "pcie2",
                );
            }
        }
        let col = edev.col_cfg;
        match grid.kind {
            ChipKind::Virtex4 => {
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col].1,
                    row_y[die][grid.row_reg_hclk(grid.reg_cfg - 1)].0,
                    row_y[die][grid.row_reg_hclk(grid.reg_cfg) - 1].1,
                    "cfg",
                );
                for &(row, kind) in &grid.rows_cfg {
                    match kind {
                        CfgRowKind::Dcm => {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][row].0,
                                row_y[die][row + 3].1,
                                "dcm",
                            );
                        }
                        CfgRowKind::Ccm => {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][row].0,
                                row_y[die][row + 3].1,
                                "ccm",
                            );
                        }
                        CfgRowKind::Sysmon => {
                            drawer.bel_rect(
                                col_x[col].0,
                                col_x[col].1,
                                row_y[die][row].0,
                                row_y[die][row + 7].1,
                                "sysmon",
                            );
                        }
                    }
                }
            }
            ChipKind::Virtex5 => {
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col].1,
                    row_y[die][grid.row_reg_hclk(grid.reg_cfg - 1)].0,
                    row_y[die][grid.row_reg_hclk(grid.reg_cfg) - 1].1,
                    "cfg",
                );
                for row in grid.get_cmt_rows() {
                    drawer.bel_rect(
                        col_x[col].0,
                        col_x[col].1,
                        row_y[die][row].0,
                        row_y[die][row + 9].1,
                        "dcm",
                    );
                }
            }
            ChipKind::Virtex6 => {
                drawer.bel_rect(
                    col_x[col - 6].0,
                    col_x[col - 1].1,
                    row_y[die][grid.row_reg_bot(grid.reg_cfg - 1)].0,
                    row_y[die][grid.row_reg_bot(grid.reg_cfg + 1) - 1].1,
                    "cfg",
                );
            }
            ChipKind::Virtex7 => {
                drawer.bel_rect(
                    col_x[col - 6].0,
                    col_x[col - 1].1,
                    row_y[die][grid.row_reg_bot(grid.reg_cfg - 1)].0,
                    row_y[die][grid.row_reg_bot(grid.reg_cfg) - 1].1,
                    "cfg",
                );
                if grid.regs != 1 {
                    drawer.bel_rect(
                        col_x[col - 6].0,
                        col_x[col - 1].1,
                        row_y[die][grid.row_reg_bot(grid.reg_cfg)].0,
                        row_y[die][grid.row_reg_hclk(grid.reg_cfg) - 1].1,
                        "cfg",
                    );
                    drawer.bel_rect(
                        col_x[col - 6].0,
                        col_x[col - 1].1,
                        row_y[die][grid.row_reg_hclk(grid.reg_cfg)].0,
                        row_y[die][grid.row_reg_bot(grid.reg_cfg + 1) - 1].1,
                        "sysmon",
                    );
                }
            }
        }

        if grid.has_ps {
            drawer.bel_rect(
                col_x[ColId::from_idx(0)].0,
                col_x[ColId::from_idx(18)].1,
                row_y[die][grid.row_reg_bot(RegId::from_idx(grid.regs - 2))].0,
                row_y[die][grid.row_reg_bot(RegId::from_idx(grid.regs)) - 1].1,
                "ppc",
            );
        }

        for gtc in &grid.cols_gt {
            let col = gtc.col;
            let (xl, xr) = if grid.columns[gtc.col] == ColumnKind::Gt {
                (col_x[col].0, col_x[col].1)
            } else if gtc.col == grid.columns.last_id().unwrap() - 6 {
                (col_x[col].0, col_x[col + 6].1)
            } else if gtc.col.to_idx() % 2 == 1 {
                (col_x[col].0, col_x[col + 18].1)
            } else {
                (col_x[col - 18].0, col_x[col].1)
            };
            for (reg, &kind) in &gtc.regs {
                if let Some(kind) = kind {
                    let kind = match kind {
                        GtKind::Gtp => "gtp",
                        GtKind::Gtx => "gtx",
                        GtKind::Gth => "gth",
                    };
                    drawer.bel_rect(
                        xl,
                        xr,
                        row_y[die][grid.row_reg_bot(reg)].0,
                        row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                        kind,
                    );
                }
            }
        }
    }
    // TODO:
    // - GT
    // - GTZ
    // - HPIO/HRIO split

    drawer
}
