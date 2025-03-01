use prjcombine_interconnect::grid::RowId;
use prjcombine_virtex2::chip::{ChipKind, ColumnIoKind, ColumnKind, DcmPairKind, RowIoKind};
use prjcombine_virtex2::expanded::ExpandedDevice;
use unnamed_entity::EntityVec;

use crate::drawer::Drawer;

const W_CLB_V2: f64 = 8.;
const W_CLB_S3: f64 = 10.;
const W_CLK: f64 = 4.;
const W_CLKV: f64 = 2.;
const W_TERM: f64 = 24.;
const H_CLB: f64 = 8.;
const H_TERM: f64 = 24.;
const H_CLK: f64 = 4.;
const H_PCI: f64 = 4.;
const H_GCLKH: f64 = 2.;
const H_BRKH: f64 = 2.;
pub fn draw_device(name: &str, edev: ExpandedDevice) -> Drawer {
    let mut col_x = EntityVec::new();
    let mut x = 0.;
    let cw = if edev.chip.kind.is_virtex2() {
        W_CLB_V2
    } else {
        W_CLB_S3
    };
    // left term
    let term_x_l = 0.;
    x += W_TERM;
    for (col, cd) in &edev.chip.columns {
        if edev.chip.col_clk == col {
            x += W_CLK;
        }
        if let Some((cl, cr)) = edev.chip.cols_clkv {
            if col == cl || col == cr {
                x += W_CLKV;
            }
        }
        let l = x;
        if cd.kind == prjcombine_virtex2::chip::ColumnKind::Bram && !edev.chip.kind.is_spartan3ea()
        {
            x += cw * 4.;
        } else {
            x += cw;
        }
        col_x.push((l, x));
    }
    x += W_TERM;
    let term_x_r = x;
    let width = x;
    let mut row_y = EntityVec::new();
    let mut y = 0.;
    let term_y_b = 0.;
    y += H_TERM;
    for (row, _) in &edev.chip.rows {
        if edev.chip.kind.is_virtex2() && edev.chip.row_pci == Some(row) {
            y += H_PCI;
        } else if edev.chip.kind.is_spartan3ea() && edev.chip.row_mid() == row {
            y += H_CLK;
        } else if edev.chip.rows_hclk.iter().any(|&(_, b, _)| b == row) && row != edev.chip.row_s()
        {
            y += H_BRKH;
        }
        if edev.chip.rows_hclk.iter().any(|&(m, _, _)| m == row) {
            y += H_GCLKH;
        }
        let b = y;
        y += H_CLB;
        row_y.push((b, y));
    }
    y += H_TERM;
    let height = y;
    let term_y_t = y;
    let mut drawer = Drawer::new(name.to_string(), width, height);
    drawer.bel_class("clb", "#00ff00");
    drawer.bel_class("bram", "#5555ff");
    drawer.bel_class("dsp", "#00aaaa");
    drawer.bel_class("ioi", "#aa00aa");
    drawer.bel_class("iob", "#ff00ff");
    drawer.bel_class("dcm", "#aaaa00");
    drawer.bel_class("ppc", "#ff0000");
    drawer.bel_class("cfg", "#ff8000");
    drawer.bel_class("pci", "#ff0000");
    drawer.bel_class("gt", "#8000ff");
    drawer.bel_class("bufg", "#aa5500");

    for (col, cd) in &edev.chip.columns {
        if cd.kind == prjcombine_virtex2::chip::ColumnKind::Io {
            continue;
        }
        if cd.kind != prjcombine_virtex2::chip::ColumnKind::Clb
            && edev.chip.kind != ChipKind::Spartan3E
        {
            continue;
        }
        for (row, &rd) in &edev.chip.rows {
            if rd == RowIoKind::None {
                continue;
            }
            if edev.is_in_hole(col, row) {
                continue;
            }
            drawer.bel_rect(
                col_x[col].0,
                col_x[col + 1 - 1].1,
                row_y[row].0,
                row_y[row + 1 - 1].1,
                "clb",
            );
        }
    }
    for (col, cd) in &edev.chip.columns {
        if cd.kind != prjcombine_virtex2::chip::ColumnKind::Bram {
            continue;
        }
        let (row_b, row_t): (RowId, RowId) = if let Some((row_b, row_t)) = edev.chip.rows_ram {
            (row_b + 1, row_t)
        } else {
            (edev.chip.row_s() + 1, edev.chip.row_n())
        };
        for row in row_b.range(row_t).step_by(4) {
            if edev.chip.kind != ChipKind::Spartan3E && edev.is_in_hole(col, row) {
                continue;
            }
            let width = if edev.chip.kind == ChipKind::Spartan3ADsp {
                3
            } else if edev.chip.kind.is_spartan3ea() {
                4
            } else {
                1
            };
            drawer.bel_rect(
                col_x[col].0,
                col_x[col + width - 1].1,
                row_y[row].0,
                row_y[row + 4 - 1].1,
                "bram",
            );
            if edev.chip.kind == ChipKind::Spartan3ADsp {
                let col = col + 3;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 1 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dsp",
                );
            }
        }
    }
    for (col, cd) in &edev.chip.columns {
        if cd.kind == ColumnKind::Io {
            continue;
        }
        if !edev.chip.kind.is_spartan3ea() && cd.kind == ColumnKind::Bram {
            if !edev.chip.kind.is_virtex2()
                && col != edev.chip.col_w() + 3
                && col != edev.chip.col_e() - 3
            {
                continue;
            }
            if edev.is_in_hole(col, edev.chip.row_s()) {
                continue;
            }
            drawer.bel_rect(
                col_x[col].0,
                col_x[col].1,
                term_y_b,
                row_y[edev.chip.row_s()].1,
                "dcm",
            );
            drawer.bel_rect(
                col_x[col].0,
                col_x[col].1,
                row_y[edev.chip.row_n()].0,
                term_y_t,
                "dcm",
            );
        } else {
            for row in [edev.chip.row_s(), edev.chip.row_n()] {
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 1 - 1].1,
                    row_y[row].0,
                    row_y[row + 1 - 1].1,
                    "ioi",
                );
            }
            let cr = match cd.io {
                ColumnIoKind::Single
                | ColumnIoKind::SingleLeft
                | ColumnIoKind::SingleLeftAlt
                | ColumnIoKind::SingleRight
                | ColumnIoKind::SingleRightAlt => col,
                ColumnIoKind::Double(0)
                | ColumnIoKind::DoubleLeft(0)
                | ColumnIoKind::DoubleRight(0) => col + 1,
                ColumnIoKind::Triple(0) => col + 2,
                ColumnIoKind::Quad(0) => col + 3,
                _ => continue,
            };
            drawer.bel_rect(
                col_x[col].0,
                col_x[cr].1,
                term_y_b,
                row_y[edev.chip.row_s()].0,
                "iob",
            );
            drawer.bel_rect(
                col_x[col].0,
                col_x[cr].1,
                row_y[edev.chip.row_n()].1,
                term_y_t,
                "iob",
            );
        }
    }
    for (row, &rd) in &edev.chip.rows {
        if rd == RowIoKind::None {
            continue;
        }
        for col in [edev.chip.col_w(), edev.chip.col_e()] {
            drawer.bel_rect(
                col_x[col].0,
                col_x[col + 1 - 1].1,
                row_y[row].0,
                row_y[row + 1 - 1].1,
                "ioi",
            );
        }
        let rt = match rd {
            RowIoKind::Single => row,
            RowIoKind::Double(0) | RowIoKind::DoubleBot(0) | RowIoKind::DoubleTop(0) => row + 1,
            RowIoKind::Triple(0) => row + 2,
            RowIoKind::Quad(0) => row + 3,
            _ => continue,
        };
        drawer.bel_rect(
            term_x_l,
            col_x[edev.chip.col_w()].0,
            row_y[row].0,
            row_y[rt].1,
            "iob",
        );
        drawer.bel_rect(
            col_x[edev.chip.col_e()].1,
            term_x_r,
            row_y[row].0,
            row_y[rt].1,
            "iob",
        );
    }
    for pair in edev.chip.get_dcm_pairs() {
        match pair.kind {
            DcmPairKind::Bot => {
                let col = pair.col - 4;
                let row = pair.row;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
                let col = pair.col;
                let row = pair.row;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
            }
            DcmPairKind::BotSingle => {
                let col = pair.col - 1;
                let row = pair.row;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 1 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
                let col = pair.col;
                let row = pair.row;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
            }
            DcmPairKind::Top => {
                let col = pair.col - 4;
                let row = pair.row - 3;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
                let col = pair.col;
                let row = pair.row - 3;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
            }
            DcmPairKind::TopSingle => {
                let col = pair.col - 1;
                let row = pair.row - 3;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 1 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
                let col = pair.col;
                let row = pair.row - 3;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
            }
            DcmPairKind::Left | DcmPairKind::Bram => {
                let col = pair.col;
                let row = pair.row - 4;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
                let col = pair.col;
                let row = pair.row;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
            }
            DcmPairKind::Right => {
                let col = pair.col - 3;
                let row = pair.row - 4;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
                let col = pair.col - 3;
                let row = pair.row;
                drawer.bel_rect(
                    col_x[col].0,
                    col_x[col + 4 - 1].1,
                    row_y[row].0,
                    row_y[row + 4 - 1].1,
                    "dcm",
                );
            }
        }
    }
    for &col in edev.chip.cols_gt.keys() {
        let row = edev.chip.row_s();
        let sz = if edev.chip.kind == ChipKind::Virtex2PX {
            8
        } else {
            4
        };
        drawer.bel_poly(
            vec![
                (col_x[col - 1].0, term_y_b),
                (col_x[col - 1].0, row_y[row].0),
                (col_x[col].0, row_y[row].0),
                (col_x[col].0, row_y[row + sz].1),
                (col_x[col].1, row_y[row + sz].1),
                (col_x[col].1, row_y[row].0),
                (col_x[col + 1].1, row_y[row].0),
                (col_x[col + 1].1, term_y_b),
            ],
            "gt",
        );
        let row = edev.chip.row_n();
        drawer.bel_poly(
            vec![
                (col_x[col].0, row_y[row - sz].0),
                (col_x[col].1, row_y[row - sz].0),
                (col_x[col].1, row_y[row].1),
                (col_x[col + 1].1, row_y[row].1),
                (col_x[col + 1].1, term_y_t),
                (col_x[col - 1].0, term_y_t),
                (col_x[col - 1].0, row_y[row].1),
                (col_x[col].0, row_y[row].1),
            ],
            "gt",
        );
    }
    for &(col, row) in &edev.chip.holes_ppc {
        drawer.bel_rect(
            col_x[col].0,
            col_x[col + 10 - 1].1,
            row_y[row].0,
            row_y[row + 16 - 1].1,
            "ppc",
        );
    }
    drawer.bel_rect(
        term_x_l,
        col_x[edev.chip.col_w()].1,
        term_y_b,
        row_y[edev.chip.row_s()].1,
        "cfg",
    );
    drawer.bel_rect(
        term_x_l,
        col_x[edev.chip.col_w()].1,
        row_y[edev.chip.row_n()].0,
        term_y_t,
        "cfg",
    );
    drawer.bel_rect(
        col_x[edev.chip.col_e()].0,
        term_x_r,
        term_y_b,
        row_y[edev.chip.row_s()].1,
        "cfg",
    );
    drawer.bel_rect(
        col_x[edev.chip.col_e()].0,
        term_x_r,
        row_y[edev.chip.row_n()].0,
        term_y_t,
        "cfg",
    );

    let col = edev.chip.col_clk;
    drawer.bel_rect(
        col_x[col - 1].1,
        col_x[col].0,
        term_y_b,
        row_y[edev.chip.row_s()].1,
        "bufg",
    );
    drawer.bel_rect(
        col_x[col - 1].1,
        col_x[col].0,
        row_y[edev.chip.row_n()].0,
        term_y_t,
        "bufg",
    );

    if edev.chip.kind.is_virtex2() {
        let row = edev.chip.row_pci.unwrap();
        drawer.bel_rect(
            term_x_l,
            col_x[edev.chip.col_w()].1,
            row_y[row - 1].1,
            row_y[row].0,
            "pci",
        );
        drawer.bel_rect(
            col_x[edev.chip.col_e()].0,
            term_x_r,
            row_y[row - 1].1,
            row_y[row].0,
            "pci",
        );
    }
    if edev.chip.kind.is_spartan3ea() {
        let row = edev.chip.row_mid();
        drawer.bel_rect(
            term_x_l,
            col_x[edev.chip.col_w()].1,
            row_y[row - 1].1,
            row_y[row].0,
            "bufg",
        );
        drawer.bel_rect(
            col_x[edev.chip.col_e()].0,
            term_x_r,
            row_y[row - 1].1,
            row_y[row].0,
            "bufg",
        );
    }

    drawer
}
