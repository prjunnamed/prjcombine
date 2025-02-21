use prjcombine_spartan6::chip::{ColumnIoKind, ColumnKind, Gts};
use prjcombine_spartan6::expanded::ExpandedDevice;
use unnamed_entity::{EntityId, EntityVec};

use crate::drawer::Drawer;

const W_CLB: f64 = 20.;
const W_SPINE: f64 = 20.;
const W_BRAM: f64 = 40.;
const W_DSP: f64 = 30.;
const W_TERM: f64 = 32.;
const H_CLB: f64 = 8.;
const H_TERM: f64 = 32.;
const H_CLK: f64 = 4.;
const H_HCLK: f64 = 2.;
const H_BRKH: f64 = 2.;

pub fn draw_device(name: &str, edev: ExpandedDevice) -> Drawer {
    let mut x = 0.;
    x += W_TERM;
    let mut col_x = EntityVec::new();
    for (_, cd) in &edev.chip.columns {
        let w = match cd.kind {
            ColumnKind::Bram => W_BRAM,
            ColumnKind::Dsp | ColumnKind::DspPlus => W_DSP,
            _ => W_CLB,
        };
        col_x.push((x, x + w));
        x += w;
        if cd.kind == ColumnKind::CleClk {
            x += W_SPINE;
        }
    }
    x += W_TERM;
    let width = x;

    let mut y = 0.;
    y += H_TERM;
    let mut row_y = EntityVec::new();
    for row in edev.chip.rows.ids() {
        if row.to_idx() % 16 == 0 && row.to_idx() != 0 {
            y += H_BRKH;
        }
        if row.to_idx() % 16 == 8 {
            y += H_HCLK;
        }
        if row == edev.chip.row_clk() {
            y += H_CLK;
        }

        row_y.push((y, y + H_CLB));
        y += H_CLB;
    }
    y += H_TERM;
    let height = y;

    let mut drawer = Drawer::new(name.to_string(), width, height);
    drawer.bel_class("clexl", "#00cc00");
    drawer.bel_class("clexm", "#00ff00");
    drawer.bel_class("bram", "#5555ff");
    drawer.bel_class("dsp", "#00aaaa");
    drawer.bel_class("ioi", "#aa00aa");
    drawer.bel_class("iob", "#ff00ff");
    drawer.bel_class("dcm", "#aaaa00");
    drawer.bel_class("pll", "#ffff00");
    drawer.bel_class("cfg", "#ff8000");
    drawer.bel_class("pcie", "#ff0000");
    drawer.bel_class("mcb", "#ff0000");
    drawer.bel_class("gt", "#8000ff");
    drawer.bel_class("bufg", "#aa5500");

    for (col, cd) in &edev.chip.columns {
        match cd.kind {
            ColumnKind::CleXL | ColumnKind::CleXM | ColumnKind::CleClk => {
                let kind = if cd.kind == ColumnKind::CleXM {
                    "clexm"
                } else {
                    "clexl"
                };
                for row in edev.chip.rows.ids() {
                    if edev.in_site_hole(col, row) {
                        continue;
                    }
                    drawer.bel_rect(col_x[col].0, col_x[col].1, row_y[row].0, row_y[row].1, kind);
                }
                if cd.bio != ColumnIoKind::None {
                    for row in [edev.chip.row_bio_outer(), edev.chip.row_bio_inner()] {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[row].0,
                            row_y[row].1,
                            "ioi",
                        )
                    }
                    let row = edev.chip.row_bio_outer();
                    if cd.bio != ColumnIoKind::Inner {
                        drawer.bel_rect(
                            col_x[col].0 + W_CLB / 2.,
                            col_x[col].1,
                            row_y[row].0 - H_TERM,
                            row_y[row].0,
                            "iob",
                        );
                    }
                    if cd.bio != ColumnIoKind::Outer {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].0 + W_CLB / 2.,
                            row_y[row].0 - H_TERM,
                            row_y[row].0,
                            "iob",
                        );
                    }
                }
                if cd.tio != ColumnIoKind::None {
                    for row in [edev.chip.row_tio_outer(), edev.chip.row_tio_inner()] {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[row].0,
                            row_y[row].1,
                            "ioi",
                        )
                    }
                    let row = edev.chip.row_tio_outer();
                    if cd.bio != ColumnIoKind::Inner {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].0 + W_CLB / 2.,
                            row_y[row].1,
                            row_y[row].1 + H_TERM,
                            "iob",
                        );
                    }
                    if cd.bio != ColumnIoKind::Outer {
                        drawer.bel_rect(
                            col_x[col].0 + W_CLB / 2.,
                            col_x[col].1,
                            row_y[row].1,
                            row_y[row].1 + H_TERM,
                            "iob",
                        );
                    }
                }
            }
            ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus => {
                let kind = if cd.kind == ColumnKind::Bram {
                    "bram"
                } else {
                    "dsp"
                };
                for row in edev.chip.rows.ids().step_by(4) {
                    if edev.in_site_hole(col, row) {
                        continue;
                    }
                    drawer.bel_rect(
                        col_x[col].0,
                        col_x[col].1,
                        row_y[row].0,
                        row_y[row + 3].1,
                        kind,
                    );
                }
            }
            ColumnKind::Io => {
                for (row, rd) in &edev.chip.rows {
                    if (col == edev.chip.col_lio() && rd.lio)
                        || (col == edev.chip.col_rio() && rd.rio)
                    {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[row].0,
                            row_y[row].1,
                            "ioi",
                        );
                        if col == edev.chip.col_lio() {
                            drawer.bel_rect(
                                col_x[col].0 - W_TERM,
                                col_x[col].0,
                                row_y[row].0,
                                row_y[row].1,
                                "iob",
                            );
                        } else {
                            drawer.bel_rect(
                                col_x[col].1,
                                col_x[col].1 + W_TERM,
                                row_y[row].0,
                                row_y[row].1,
                                "iob",
                            );
                        }
                    }
                }
                for mcb in &edev.chip.mcbs {
                    drawer.bel_rect(
                        col_x[col].0,
                        col_x[col].1,
                        row_y[mcb.row_mcb].0,
                        row_y[mcb.row_mcb + 11].1,
                        "mcb",
                    );
                    for row in mcb.row_mui {
                        drawer.bel_rect(
                            col_x[col].0,
                            col_x[col].1,
                            row_y[row].0,
                            row_y[row + 1].1,
                            "mcb",
                        );
                    }
                }
            }
        }
    }
    let col = edev.chip.col_clk;
    drawer.bel_rect(
        col_x[col].0,
        col_x[col].1,
        row_y[edev.chip.row_clk()].0,
        row_y[edev.chip.row_clk()].1,
        "bufg",
    );

    for (row, _) in edev.chip.get_dcms() {
        drawer.bel_poly(
            vec![
                (col_x[col].1, row_y[row - 8].0),
                (col_x[col].1, row_y[row - 1].0),
                (col_x[col].0, row_y[row - 1].0),
                (col_x[col].0, row_y[row].1),
                (col_x[col].1, row_y[row].1),
                (col_x[col].1, row_y[row + 7].1),
                (col_x[col].1 + W_SPINE, row_y[row + 7].1),
                (col_x[col].1 + W_SPINE, row_y[row - 8].0),
            ],
            "dcm",
        );
    }

    for (row, _) in edev.chip.get_plls() {
        drawer.bel_poly(
            vec![
                (col_x[col].1, row_y[row - 8].0),
                (col_x[col].1, row_y[row - 1].0),
                (col_x[col].0, row_y[row - 1].0),
                (col_x[col].0, row_y[row].1),
                (col_x[col].1, row_y[row].1),
                (col_x[col].1, row_y[row + 7].1),
                (col_x[col].1 + W_SPINE, row_y[row + 7].1),
                (col_x[col].1 + W_SPINE, row_y[row - 8].0),
            ],
            "pll",
        );
    }

    drawer.bel_rect(
        col_x[edev.chip.col_lio()].0,
        col_x[edev.chip.col_lio()].1,
        row_y[edev.chip.row_bio_outer()].0,
        row_y[edev.chip.row_bio_outer()].1,
        "cfg",
    );
    drawer.bel_rect(
        col_x[edev.chip.col_lio()].0,
        col_x[edev.chip.col_lio()].1,
        row_y[edev.chip.row_tio_outer()].0,
        row_y[edev.chip.row_tio_outer()].1,
        "cfg",
    );
    drawer.bel_rect(
        col_x[edev.chip.col_rio()].0,
        col_x[edev.chip.col_rio()].1,
        row_y[edev.chip.row_bio_outer()].0,
        row_y[edev.chip.row_bio_inner()].1,
        "cfg",
    );
    drawer.bel_rect(
        col_x[edev.chip.col_rio()].0,
        col_x[edev.chip.col_rio()].1,
        row_y[edev.chip.row_tio_inner()].0,
        row_y[edev.chip.row_tio_outer()].1,
        "cfg",
    );

    if let Gts::Single(cl) | Gts::Double(cl, _) | Gts::Quad(cl, _) = edev.chip.gts {
        let row_b = edev.chip.row_top() - 16;
        let row_m = edev.chip.row_top() - 8;
        let row_t = edev.chip.row_top();
        drawer.bel_poly(
            vec![
                (col_x[cl - 5].0, row_y[row_b].0),
                (col_x[cl - 5].0, row_y[row_m].0),
                (col_x[cl - 6].0, row_y[row_m].0),
                (col_x[cl - 6].0, row_y[row_t - 1].1 + H_TERM),
                (col_x[cl + 4].1, row_y[row_t - 1].1 + H_TERM),
                (col_x[cl + 4].1, row_y[row_m].0),
                (col_x[cl + 3].1, row_y[row_m].0),
                (col_x[cl + 3].1, row_y[row_b].0),
            ],
            "gt",
        );
        drawer.bel_rect(
            col_x[cl - 2].0,
            col_x[cl + 2].1,
            row_y[edev.chip.row_top() - 32].0,
            row_y[edev.chip.row_top() - 17].1,
            "pcie",
        );
    }
    if let Gts::Double(_, cr) | Gts::Quad(_, cr) = edev.chip.gts {
        let row_b = edev.chip.row_top() - 16;
        let row_m = edev.chip.row_top() - 8;
        let row_t = edev.chip.row_top();
        drawer.bel_poly(
            vec![
                (col_x[cr - 3].0, row_y[row_b].0),
                (col_x[cr - 3].0, row_y[row_m].0),
                (col_x[cr - 4].0, row_y[row_m].0),
                (col_x[cr - 4].0, row_y[row_t - 1].1 + H_TERM),
                (col_x[cr + 6].1, row_y[row_t - 1].1 + H_TERM),
                (col_x[cr + 6].1, row_y[row_b].0),
            ],
            "gt",
        );
    }
    if let Gts::Quad(cl, cr) = edev.chip.gts {
        let row_b = edev.chip.row_bot();
        let row_m = edev.chip.row_bot() + 8;
        let row_t = edev.chip.row_bot() + 16;
        drawer.bel_poly(
            vec![
                (col_x[cl - 6].0, row_y[row_b].0 - H_TERM),
                (col_x[cl - 6].0, row_y[row_m - 1].1),
                (col_x[cl - 5].0, row_y[row_m - 1].1),
                (col_x[cl - 5].0, row_y[row_t - 1].1),
                (col_x[cl + 3].1, row_y[row_t - 1].1),
                (col_x[cl + 3].1, row_y[row_m - 1].1),
                (col_x[cl + 4].1, row_y[row_m - 1].1),
                (col_x[cl + 4].1, row_y[row_b].0 - H_TERM),
            ],
            "gt",
        );
        drawer.bel_poly(
            vec![
                (col_x[cr - 4].0, row_y[row_b].0 - H_TERM),
                (col_x[cr - 4].0, row_y[row_m - 1].1),
                (col_x[cr - 3].0, row_y[row_m - 1].1),
                (col_x[cr - 3].0, row_y[row_t - 1].1),
                (col_x[cr + 6].1, row_y[row_t - 1].1),
                (col_x[cr + 6].1, row_y[row_b].0 - H_TERM),
            ],
            "gt",
        );
    }

    drawer
}
