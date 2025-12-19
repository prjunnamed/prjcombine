use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{bels::xc5200 as bels, chip::Chip, expanded::ExpandedDevice};
use prjcombine_entity::{EntityId, EntityVec};

use crate::ExpandedNamedDevice;

fn name_a(grid: &Chip, prefix: &str, suffix: &str, col: ColId, row: RowId) -> String {
    let cidx = if col < grid.col_mid() {
        col.to_idx()
    } else {
        col.to_idx() + 1
    };
    let ridx = if row < grid.row_mid() {
        grid.rows - row.to_idx()
    } else {
        grid.rows - row.to_idx() - 1
    };
    let cidx = u32::try_from(cidx).unwrap();
    let ridx = u32::try_from(ridx).unwrap();
    let r = char::from_u32(u32::from('A') + ridx).unwrap();
    let c = char::from_u32(u32::from('A') + cidx).unwrap();
    format!("{prefix}{r}{c}{suffix}")
}

fn name_b(grid: &Chip, prefix: &str, suffix: &str, col: ColId, row: RowId) -> String {
    let cidx = col.to_idx();
    let ridx = grid.rows - row.to_idx() - 1;
    format!("{prefix}R{ridx}C{cidx}{suffix}")
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let grid = edev.chip;
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);
    ngrid.tie_pin_gnd = Some("O".to_string());

    let die = DieId::from_idx(0);
    let mut col_x = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut clk_x = 0..0;
    let mut clk_y = 0..0;
    let mut x = 0;
    for col in edev.cols(die) {
        if col == grid.col_mid() {
            let ox = x;
            x += ndb.tile_widths["CLK"];
            clk_x = ox..x;
        }
        let ox = x;
        x += if col == grid.col_w() {
            ndb.tile_widths["L"]
        } else if col == grid.col_e() {
            ndb.tile_widths["R"]
        } else {
            ndb.tile_widths["C"]
        };
        col_x.push(ox..x);
    }
    let mut y = 0;
    for row in edev.rows(die) {
        if row == grid.row_mid() {
            let oy = y;
            y += ndb.tile_heights["CLK"];
            clk_y = oy..y;
        }
        let oy = y;
        y += if row == grid.row_s() {
            ndb.tile_heights["B"]
        } else if row == grid.row_n() {
            ndb.tile_heights["T"]
        } else {
            ndb.tile_heights["C"]
        };
        row_y.push(oy..y);
    }
    for (tcrd, tile) in edev.tiles() {
        let col = tcrd.col;
        let row = tcrd.row;
        let kind = edev.db.tile_classes.key(tile.class);
        match &kind[..] {
            "CLB" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.add_bel(
                    bels::LC0,
                    vec![
                        name_a(grid, "", "", col, row),
                        name_b(grid, "CLB_", "", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF0,
                    vec![
                        name_a(grid, "TBUF.", ".0", col, row),
                        name_b(grid, "TBUF_", ".0", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF1,
                    vec![
                        name_a(grid, "TBUF.", ".1", col, row),
                        name_b(grid, "TBUF_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF2,
                    vec![
                        name_a(grid, "TBUF.", ".2", col, row),
                        name_b(grid, "TBUF_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF3,
                    vec![
                        name_a(grid, "TBUF.", ".3", col, row),
                        name_b(grid, "TBUF_", ".3", col, row),
                    ],
                );
            }
            "CNR.BL" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.tie_names = vec![
                    name_a(grid, "src0.", ".1", col, row),
                    name_a(grid, "dummy.", ".1", col, row),
                ];
                ntile.add_bel(bels::BUFG, vec!["bufgs_bl".to_string()]);
                ntile.add_bel(bels::CLKIOB, vec!["i_bufgs_bl".to_string()]);
                ntile.add_bel(bels::RDBK, vec!["rdbk".to_string()]);
            }
            "CNR.BR" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.tie_names = vec![
                    name_a(grid, "src0.", ".1", col, row),
                    name_a(grid, "dummy.", ".1", col, row),
                ];
                ntile.add_bel(bels::BUFG, vec!["bufgs_br".to_string()]);
                ntile.add_bel(bels::CLKIOB, vec!["i_bufgs_br".to_string()]);
                ntile.add_bel(bels::STARTUP, vec!["startup".to_string()]);
            }
            "CNR.TL" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.tie_names = vec![
                    name_a(grid, "src0.", ".1", col, row),
                    name_a(grid, "dummy.", ".1", col, row),
                ];
                ntile.add_bel(bels::BUFG, vec!["bufgs_tl".to_string()]);
                ntile.add_bel(bels::CLKIOB, vec!["i_bufgs_tl".to_string()]);
                ntile.add_bel(bels::BSCAN, vec!["bscan".to_string()]);
            }
            "CNR.TR" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.tie_names = vec![
                    name_a(grid, "src0.", ".1", col, row),
                    name_a(grid, "dummy.", ".1", col, row),
                ];
                ntile.add_bel(bels::BUFG, vec!["bufgs_tr".to_string()]);
                ntile.add_bel(bels::CLKIOB, vec!["i_bufgs_tr".to_string()]);
                ntile.add_bel(bels::OSC, vec!["osc".to_string()]);
            }
            "IO.L" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.tie_names = vec![
                    name_a(grid, "src0.", ".1", col, row),
                    name_a(grid, "dummy.", ".1", col, row),
                ];
                let p = (edev.chip.columns - 2) * 8
                    + (edev.chip.rows - 2) * 4
                    + (row.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bels::IO0, vec![format!("PAD{p}")]);
                ntile.add_bel(bels::IO1, vec![format!("PAD{}", p + 1)]);
                ntile.add_bel(bels::IO2, vec![format!("PAD{}", p + 2)]);
                ntile.add_bel(bels::IO3, vec![format!("PAD{}", p + 3)]);
                ntile.add_bel(
                    bels::TBUF0,
                    vec![
                        name_a(grid, "TBUF.", ".0", col, row),
                        name_b(grid, "TBUF_", ".0", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF1,
                    vec![
                        name_a(grid, "TBUF.", ".1", col, row),
                        name_b(grid, "TBUF_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF2,
                    vec![
                        name_a(grid, "TBUF.", ".2", col, row),
                        name_b(grid, "TBUF_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF3,
                    vec![
                        name_a(grid, "TBUF.", ".3", col, row),
                        name_b(grid, "TBUF_", ".3", col, row),
                    ],
                );
            }
            "IO.R" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.tie_names = vec![
                    name_a(grid, "src0.", ".1", col, row),
                    name_a(grid, "dummy.", ".1", col, row),
                ];
                let p = (edev.chip.columns - 2) * 4
                    + (edev.chip.row_n().to_idx() - row.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bels::IO0, vec![format!("PAD{}", p + 3)]);
                ntile.add_bel(bels::IO1, vec![format!("PAD{}", p + 2)]);
                ntile.add_bel(bels::IO2, vec![format!("PAD{}", p + 1)]);
                ntile.add_bel(bels::IO3, vec![format!("PAD{p}")]);
                ntile.add_bel(
                    bels::TBUF0,
                    vec![
                        name_a(grid, "TBUF.", ".0", col, row),
                        name_b(grid, "TBUF_", ".0", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF1,
                    vec![
                        name_a(grid, "TBUF.", ".1", col, row),
                        name_b(grid, "TBUF_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF2,
                    vec![
                        name_a(grid, "TBUF.", ".2", col, row),
                        name_b(grid, "TBUF_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF3,
                    vec![
                        name_a(grid, "TBUF.", ".3", col, row),
                        name_b(grid, "TBUF_", ".3", col, row),
                    ],
                );
            }
            "IO.B" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.tie_names = vec![
                    name_a(grid, "src0.", ".1", col, row),
                    name_a(grid, "dummy.", ".1", col, row),
                ];
                let p = (edev.chip.columns - 2) * 4
                    + (edev.chip.rows - 2) * 4
                    + (edev.chip.col_e().to_idx() - col.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bels::IO0, vec![format!("PAD{p}")]);
                ntile.add_bel(bels::IO1, vec![format!("PAD{}", p + 1)]);
                ntile.add_bel(bels::IO2, vec![format!("PAD{}", p + 2)]);
                ntile.add_bel(bels::IO3, vec![format!("PAD{}", p + 3)]);
                ntile.add_bel(
                    bels::TBUF0,
                    vec![
                        name_a(grid, "TBUF.", ".0", col, row),
                        name_b(grid, "TBUF_", ".0", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF1,
                    vec![
                        name_a(grid, "TBUF.", ".1", col, row),
                        name_b(grid, "TBUF_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF2,
                    vec![
                        name_a(grid, "TBUF.", ".2", col, row),
                        name_b(grid, "TBUF_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF3,
                    vec![
                        name_a(grid, "TBUF.", ".3", col, row),
                        name_b(grid, "TBUF_", ".3", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::SCANTEST,
                    vec![
                        name_a(grid, "SCANTEST.", ".1", col, row),
                        name_b(grid, "SCANTEST_", ".1", col, row),
                    ],
                );
            }
            "IO.T" => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                ntile.tie_names = vec![
                    name_a(grid, "src0.", ".1", col, row),
                    name_a(grid, "dummy.", ".1", col, row),
                ];
                let p = (col.to_idx() - 1) * 4 + 1;
                ntile.add_bel(bels::IO0, vec![format!("PAD{}", p + 3)]);
                ntile.add_bel(bels::IO1, vec![format!("PAD{}", p + 2)]);
                ntile.add_bel(bels::IO2, vec![format!("PAD{}", p + 1)]);
                ntile.add_bel(bels::IO3, vec![format!("PAD{p}")]);
                ntile.add_bel(
                    bels::TBUF0,
                    vec![
                        name_a(grid, "TBUF.", ".0", col, row),
                        name_b(grid, "TBUF_", ".0", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF1,
                    vec![
                        name_a(grid, "TBUF.", ".1", col, row),
                        name_b(grid, "TBUF_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF2,
                    vec![
                        name_a(grid, "TBUF.", ".2", col, row),
                        name_b(grid, "TBUF_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bels::TBUF3,
                    vec![
                        name_a(grid, "TBUF.", ".3", col, row),
                        name_b(grid, "TBUF_", ".3", col, row),
                    ],
                );
            }
            "CLKL" | "CLKR" | "CLKH" => {
                ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), clk_y.clone())]);
            }
            "CLKB" | "CLKT" | "CLKV" => {
                ngrid.name_tile(tcrd, kind, [(clk_x.clone(), row_y[row].clone())]);
            }

            _ => panic!("umm {kind}"),
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid,
        chip: grid,
        col_x,
        row_y,
        clk_x: Some(clk_x),
        clk_y: Some(clk_y),
    }
}
