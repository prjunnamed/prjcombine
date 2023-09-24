use prjcombine_int::grid::{ColId, DieId, RowId};
use prjcombine_ultrascale::expanded::ExpandedDevice;
use prjcombine_ultrascale::grid::{
    CleMKind, ColSide, ColumnKindLeft, ColumnKindRight, HardRowKind, IoRowKind,
};
use unnamed_entity::{EntityId, EntityVec};

use crate::drawer::Drawer;

const W_CLB: f64 = 10.;
const W_DSP: f64 = 16.;
const W_BRAM: f64 = 40.;
const W_URAM: f64 = 120.;
const W_HARD: f64 = 120.;
const W_IO: f64 = 240.;
const W_TERM: f64 = 4.;
const W_BRK: f64 = 2.;
const H_TERM: f64 = 4.;
const H_CLB: f64 = 8.;
const H_HCLK: f64 = 2.;
const H_BRKH: f64 = 2.;
const H_HBM: f64 = 40.;

pub fn draw_device(name: &str, edev: ExpandedDevice) -> Drawer {
    let mut x = 0.;
    let mut col_x = EntityVec::new();
    let mgrid = edev.grids[edev.grid_master];
    x += W_TERM;
    for (col, &cd) in &mgrid.columns {
        if mgrid.cols_vbrk.contains(&col) {
            x += W_BRK;
        }
        let xl = x;
        let w = match cd.l {
            ColumnKindLeft::CleL | ColumnKindLeft::CleM(_) => W_CLB,
            ColumnKindLeft::Bram(_) => W_BRAM,
            ColumnKindLeft::Uram => W_URAM,
            ColumnKindLeft::Io(_) | ColumnKindLeft::Gt(_) => W_IO,
            ColumnKindLeft::Hard(_, _)
            | ColumnKindLeft::Sdfec
            | ColumnKindLeft::DfeC
            | ColumnKindLeft::DfeDF
            | ColumnKindLeft::DfeE => W_HARD,
        };
        x += w;
        let xm = x;
        let w = match cd.r {
            ColumnKindRight::CleL(_) => W_CLB,
            ColumnKindRight::Dsp(_) => W_DSP,
            ColumnKindRight::Io(_) | ColumnKindRight::Gt(_) => W_IO,
            ColumnKindRight::DfeB => W_HARD,
            ColumnKindRight::Uram
            | ColumnKindRight::Hard(_, _)
            | ColumnKindRight::DfeC
            | ColumnKindRight::DfeDF
            | ColumnKindRight::DfeE => 0.,
        };
        x += w;
        col_x.push((xl, xm, x));
    }
    x += W_TERM;
    let width = x;

    let mut y = 0.;
    let mut die_y: EntityVec<DieId, _> = EntityVec::new();
    let mut row_y = EntityVec::new();
    for (_, grid) in &edev.grids {
        let term_y_b = y;
        let mut die_row_y = EntityVec::new();
        y += H_TERM;
        if grid.has_hbm {
            y += H_HBM;
        }
        for row in grid.rows() {
            if row.to_idx() % 60 == 0 {
                y += H_BRKH;
            }
            if row.to_idx() % 60 == 60 / 2 {
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
    drawer.bel_class("clel", "#00cc00");
    drawer.bel_class("clem", "#00ff00");
    drawer.bel_class("laguna", "#ff80ff");
    drawer.bel_class("bram", "#5555ff");
    drawer.bel_class("uram", "#0000ff");
    drawer.bel_class("dsp", "#00aaaa");
    drawer.bel_class("hrio", "#ff33ff");
    drawer.bel_class("hdio", "#ff66ff");
    drawer.bel_class("hpio", "#ff00ff");
    drawer.bel_class("gth", "#c000ff");
    drawer.bel_class("gty", "#8000ff");
    drawer.bel_class("gtm", "#4000ff");
    drawer.bel_class("gtf", "#4000c0");
    drawer.bel_class("sysmon", "#aa00aa");
    drawer.bel_class("hsdac", "#4040c0");
    drawer.bel_class("hsadc", "#8040c0");
    drawer.bel_class("rfdac", "#2020c0");
    drawer.bel_class("rfadc", "#4020c0");
    drawer.bel_class("ps", "#ff0000");
    drawer.bel_class("vcu", "#aa0000");
    drawer.bel_class("hbm", "#aa0000");
    drawer.bel_class("cfg", "#ff8000");
    drawer.bel_class("pcie", "#ff0000");
    drawer.bel_class("ilkn", "#aa0000");
    drawer.bel_class("cmac", "#ff3333");
    drawer.bel_class("dfea", "#aa0055");
    drawer.bel_class("dfeb", "#aa3300");
    drawer.bel_class("dfec", "#aa3355");
    drawer.bel_class("dfed", "#ff0055");
    drawer.bel_class("dfee", "#ff3300");
    drawer.bel_class("dfef", "#ff3355");
    drawer.bel_class("dfeg", "#cc0033");
    drawer.bel_class("sdfec", "#cc3333");

    for (die, grid) in &edev.grids {
        for (col, &cd) in &grid.columns {
            match cd.l {
                ColumnKindLeft::CleL | ColumnKindLeft::CleM(_) => {
                    for row in grid.rows() {
                        let kind = match cd.l {
                            ColumnKindLeft::CleL => "clel",
                            ColumnKindLeft::CleM(CleMKind::Laguna) if grid.is_laguna_row(row) => {
                                "laguna"
                            }
                            _ => "clem",
                        };
                        if edev.in_site_hole(die, col, row, ColSide::Left) {
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
                ColumnKindLeft::Bram(_) => {
                    for row in grid.rows().step_by(5) {
                        if edev.in_site_hole(die, col, row, ColSide::Left) {
                            continue;
                        }
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][row].0,
                            row_y[die][row + 4].1,
                            "bram",
                        )
                    }
                }
                ColumnKindLeft::Uram => {
                    for row in grid.rows().step_by(15) {
                        if edev.in_site_hole(die, col, row, ColSide::Left) {
                            continue;
                        }
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][row].0,
                            row_y[die][row + 14].1,
                            "uram",
                        )
                    }
                }

                ColumnKindLeft::Io(idx) | ColumnKindLeft::Gt(idx) => {
                    for (reg, kind) in &grid.cols_io[idx].regs {
                        let kind = match kind {
                            IoRowKind::None => continue,
                            IoRowKind::Hpio => "hpio",
                            IoRowKind::Hrio => "hrio",
                            IoRowKind::Gth => "gth",
                            IoRowKind::Gty => "gty",
                            IoRowKind::Gtm => "gtm",
                            IoRowKind::Gtf => "gtf",
                            IoRowKind::HsAdc => "hsadc",
                            IoRowKind::HsDac => "hsdac",
                            IoRowKind::RfAdc => "rfadc",
                            IoRowKind::RfDac => "rfdac",
                        };
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][grid.row_reg_bot(reg)].0,
                            row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                            kind,
                        )
                    }
                }
                ColumnKindLeft::Hard(_, idx) => {
                    for (reg, kind) in &grid.cols_hard[idx].regs {
                        let kind = match kind {
                            HardRowKind::Cfg => "cfg",
                            HardRowKind::Ams => "sysmon",
                            HardRowKind::None => continue,
                            HardRowKind::Hdio | HardRowKind::HdioAms => "hdio",
                            HardRowKind::Pcie | HardRowKind::PciePlus => "pcie",
                            HardRowKind::Cmac => "cmac",
                            HardRowKind::Ilkn => "ilkn",
                            HardRowKind::DfeA => "dfea",
                            HardRowKind::DfeG => "dfeg",
                        };
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][grid.row_reg_bot(reg)].0,
                            row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                            kind,
                        )
                    }
                }
                ColumnKindLeft::Sdfec
                | ColumnKindLeft::DfeC
                | ColumnKindLeft::DfeDF
                | ColumnKindLeft::DfeE => {
                    for reg in grid.regs() {
                        let kind = match cd.l {
                            ColumnKindLeft::Sdfec => "sdfec",
                            ColumnKindLeft::DfeC => "dfec",
                            ColumnKindLeft::DfeDF => {
                                if reg.to_idx() == 2 {
                                    "dfef"
                                } else {
                                    "dfed"
                                }
                            }
                            ColumnKindLeft::DfeE => "dfee",
                            _ => unreachable!(),
                        };
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[die][grid.row_reg_bot(reg)].0,
                            row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                            kind,
                        )
                    }
                }
            }
            match cd.r {
                ColumnKindRight::CleL(_) => {
                    for row in grid.rows() {
                        if edev.in_site_hole(die, col, row, ColSide::Right) {
                            continue;
                        }
                        drawer.bel_rect(
                            col_x[col].1,
                            col_x[col].2,
                            row_y[die][row].0,
                            row_y[die][row].1,
                            "clel",
                        )
                    }
                }
                ColumnKindRight::Dsp(_) => {
                    for row in grid.rows().step_by(5) {
                        if edev.in_site_hole(die, col, row, ColSide::Right) {
                            continue;
                        }
                        drawer.bel_rect(
                            col_x[col].1,
                            col_x[col].2,
                            row_y[die][row].0,
                            row_y[die][row + 4].1,
                            "dsp",
                        )
                    }
                }
                ColumnKindRight::Io(idx) | ColumnKindRight::Gt(idx) => {
                    for (reg, kind) in &grid.cols_io[idx].regs {
                        let kind = match kind {
                            IoRowKind::None => continue,
                            IoRowKind::Hpio => "hpio",
                            IoRowKind::Hrio => "hrio",
                            IoRowKind::Gth => "gth",
                            IoRowKind::Gty => "gty",
                            IoRowKind::Gtm => "gtm",
                            IoRowKind::Gtf => "gtf",
                            IoRowKind::HsAdc => "hsadc",
                            IoRowKind::HsDac => "hsdac",
                            IoRowKind::RfAdc => "rfadc",
                            IoRowKind::RfDac => "rfdac",
                        };
                        drawer.bel_rect(
                            col_x[col].1,
                            col_x[col].2,
                            row_y[die][grid.row_reg_bot(reg)].0,
                            row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                            kind,
                        )
                    }
                }
                ColumnKindRight::DfeB => {
                    for reg in grid.regs() {
                        drawer.bel_rect(
                            col_x[col].1,
                            col_x[col].2,
                            row_y[die][grid.row_reg_bot(reg)].0,
                            row_y[die][grid.row_reg_bot(reg + 1) - 1].1,
                            "dfeb",
                        )
                    }
                }
                _ => (),
            }
        }
        if let Some(ps) = grid.ps {
            let col_l = ColId::from_idx(0);
            let row_b = if ps.has_vcu {
                let row_t = RowId::from_idx(60);
                drawer.bel_rect(
                    col_x[col_l].0,
                    col_x[ps.col].1,
                    row_y[die][RowId::from_idx(0)].0,
                    row_y[die][row_t - 1].1,
                    "vcu",
                );
                row_t
            } else {
                RowId::from_idx(0)
            };
            drawer.bel_rect(
                col_x[col_l].0,
                col_x[ps.col].1,
                row_y[die][row_b].0,
                row_y[die][row_b + 3 * 60 - 1].1,
                "ps",
            )
        }
        if grid.has_hbm {
            let col_l = grid.columns.first_id().unwrap();
            let col_r = grid.columns.last_id().unwrap();
            let row_b = RowId::from_idx(0);
            let mut points = vec![
                (col_x[col_l].0, row_y[die][row_b].0 - H_HBM),
                (col_x[col_l].0, row_y[die][row_b].0),
            ];
            for (col, cd) in &grid.columns {
                if matches!(cd.r, ColumnKindRight::Dsp(_)) {
                    points.extend([
                        (col_x[col].1, row_y[die][row_b].0),
                        (col_x[col].1, row_y[die][row_b + 14].1),
                        (col_x[col].2, row_y[die][row_b + 14].1),
                        (col_x[col].2, row_y[die][row_b].0),
                    ]);
                }
            }
            points.extend([
                (col_x[col_r].2, row_y[die][row_b].0),
                (col_x[col_r].2, row_y[die][row_b].0 - H_HBM),
            ]);
            drawer.bel_poly(points, "hbm");
        }
    }
    drawer
}
