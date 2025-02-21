use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{expanded::ExpandedDevice, grid::Grid};
use unnamed_entity::{EntityId, EntityVec};

use crate::ExpandedNamedDevice;

fn name_a(grid: &Grid, prefix: &str, suffix: &str, col: ColId, row: RowId) -> String {
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

fn name_b(grid: &Grid, prefix: &str, suffix: &str, col: ColId, row: RowId) -> String {
    let cidx = col.to_idx();
    let ridx = grid.rows - row.to_idx() - 1;
    format!("{prefix}R{ridx}C{cidx}{suffix}")
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let grid = edev.grid;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);
    ngrid.tie_pin_gnd = Some("O".to_string());

    let mut col_x = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut clk_x = 0..0;
    let mut clk_y = 0..0;
    let mut x = 0;
    for col in egrid.die(DieId::from_idx(0)).cols() {
        if col == grid.col_mid() {
            let ox = x;
            x += ndb.tile_widths["CLK"];
            clk_x = ox..x;
        }
        let ox = x;
        x += if col == grid.col_lio() {
            ndb.tile_widths["L"]
        } else if col == grid.col_rio() {
            ndb.tile_widths["R"]
        } else {
            ndb.tile_widths["C"]
        };
        col_x.push(ox..x);
    }
    let mut y = 0;
    for row in egrid.die(DieId::from_idx(0)).rows() {
        if row == grid.row_mid() {
            let oy = y;
            y += ndb.tile_heights["CLK"];
            clk_y = oy..y;
        }
        let oy = y;
        y += if row == grid.row_bio() {
            ndb.tile_heights["B"]
        } else if row == grid.row_tio() {
            ndb.tile_heights["T"]
        } else {
            ndb.tile_heights["C"]
        };
        row_y.push(oy..y);
    }
    for die in egrid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    match &kind[..] {
                        "CLB" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.add_bel(
                                0,
                                vec![
                                    name_a(grid, "", "", col, row),
                                    name_b(grid, "CLB_", "", col, row),
                                ],
                            );
                            nnode.add_bel(
                                4,
                                vec![
                                    name_a(grid, "TBUF.", ".0", col, row),
                                    name_b(grid, "TBUF_", ".0", col, row),
                                ],
                            );
                            nnode.add_bel(
                                5,
                                vec![
                                    name_a(grid, "TBUF.", ".1", col, row),
                                    name_b(grid, "TBUF_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                6,
                                vec![
                                    name_a(grid, "TBUF.", ".2", col, row),
                                    name_b(grid, "TBUF_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                7,
                                vec![
                                    name_a(grid, "TBUF.", ".3", col, row),
                                    name_b(grid, "TBUF_", ".3", col, row),
                                ],
                            );
                        }
                        "CNR.BL" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "src0.", ".1", col, row),
                                name_a(grid, "dummy.", ".1", col, row),
                            ];
                            nnode.add_bel(0, vec!["bufgs_bl".to_string()]);
                            nnode.add_bel(1, vec!["i_bufgs_bl".to_string()]);
                            nnode.add_bel(2, vec!["rdbk".to_string()]);
                        }
                        "CNR.BR" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "src0.", ".1", col, row),
                                name_a(grid, "dummy.", ".1", col, row),
                            ];
                            nnode.add_bel(0, vec!["bufgs_br".to_string()]);
                            nnode.add_bel(1, vec!["i_bufgs_br".to_string()]);
                            nnode.add_bel(2, vec!["startup".to_string()]);
                        }
                        "CNR.TL" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "src0.", ".1", col, row),
                                name_a(grid, "dummy.", ".1", col, row),
                            ];
                            nnode.add_bel(0, vec!["bufgs_tl".to_string()]);
                            nnode.add_bel(1, vec!["i_bufgs_tl".to_string()]);
                            nnode.add_bel(2, vec!["bscan".to_string()]);
                        }
                        "CNR.TR" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "src0.", ".1", col, row),
                                name_a(grid, "dummy.", ".1", col, row),
                            ];
                            nnode.add_bel(0, vec!["bufgs_tr".to_string()]);
                            nnode.add_bel(1, vec!["i_bufgs_tr".to_string()]);
                            nnode.add_bel(2, vec!["osc".to_string()]);
                        }
                        "IO.L" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "src0.", ".1", col, row),
                                name_a(grid, "dummy.", ".1", col, row),
                            ];
                            let p = (edev.grid.columns - 2) * 8
                                + (edev.grid.rows - 2) * 4
                                + (row.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(0, vec![format!("PAD{p}")]);
                            nnode.add_bel(1, vec![format!("PAD{}", p + 1)]);
                            nnode.add_bel(2, vec![format!("PAD{}", p + 2)]);
                            nnode.add_bel(3, vec![format!("PAD{}", p + 3)]);
                            nnode.add_bel(
                                4,
                                vec![
                                    name_a(grid, "TBUF.", ".0", col, row),
                                    name_b(grid, "TBUF_", ".0", col, row),
                                ],
                            );
                            nnode.add_bel(
                                5,
                                vec![
                                    name_a(grid, "TBUF.", ".1", col, row),
                                    name_b(grid, "TBUF_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                6,
                                vec![
                                    name_a(grid, "TBUF.", ".2", col, row),
                                    name_b(grid, "TBUF_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                7,
                                vec![
                                    name_a(grid, "TBUF.", ".3", col, row),
                                    name_b(grid, "TBUF_", ".3", col, row),
                                ],
                            );
                        }
                        "IO.R" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "src0.", ".1", col, row),
                                name_a(grid, "dummy.", ".1", col, row),
                            ];
                            let p = (edev.grid.columns - 2) * 4
                                + (edev.grid.row_tio().to_idx() - row.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(0, vec![format!("PAD{}", p + 3)]);
                            nnode.add_bel(1, vec![format!("PAD{}", p + 2)]);
                            nnode.add_bel(2, vec![format!("PAD{}", p + 1)]);
                            nnode.add_bel(3, vec![format!("PAD{p}")]);
                            nnode.add_bel(
                                4,
                                vec![
                                    name_a(grid, "TBUF.", ".0", col, row),
                                    name_b(grid, "TBUF_", ".0", col, row),
                                ],
                            );
                            nnode.add_bel(
                                5,
                                vec![
                                    name_a(grid, "TBUF.", ".1", col, row),
                                    name_b(grid, "TBUF_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                6,
                                vec![
                                    name_a(grid, "TBUF.", ".2", col, row),
                                    name_b(grid, "TBUF_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                7,
                                vec![
                                    name_a(grid, "TBUF.", ".3", col, row),
                                    name_b(grid, "TBUF_", ".3", col, row),
                                ],
                            );
                        }
                        "IO.B" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "src0.", ".1", col, row),
                                name_a(grid, "dummy.", ".1", col, row),
                            ];
                            let p = (edev.grid.columns - 2) * 4
                                + (edev.grid.rows - 2) * 4
                                + (edev.grid.col_rio().to_idx() - col.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(0, vec![format!("PAD{p}")]);
                            nnode.add_bel(1, vec![format!("PAD{}", p + 1)]);
                            nnode.add_bel(2, vec![format!("PAD{}", p + 2)]);
                            nnode.add_bel(3, vec![format!("PAD{}", p + 3)]);
                            nnode.add_bel(
                                4,
                                vec![
                                    name_a(grid, "TBUF.", ".0", col, row),
                                    name_b(grid, "TBUF_", ".0", col, row),
                                ],
                            );
                            nnode.add_bel(
                                5,
                                vec![
                                    name_a(grid, "TBUF.", ".1", col, row),
                                    name_b(grid, "TBUF_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                6,
                                vec![
                                    name_a(grid, "TBUF.", ".2", col, row),
                                    name_b(grid, "TBUF_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                7,
                                vec![
                                    name_a(grid, "TBUF.", ".3", col, row),
                                    name_b(grid, "TBUF_", ".3", col, row),
                                ],
                            );
                            nnode.add_bel(
                                10,
                                vec![
                                    name_a(grid, "SCANTEST.", ".1", col, row),
                                    name_b(grid, "SCANTEST_", ".1", col, row),
                                ],
                            );
                        }
                        "IO.T" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [(col_x[col].clone(), row_y[row].clone())],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "src0.", ".1", col, row),
                                name_a(grid, "dummy.", ".1", col, row),
                            ];
                            let p = (col.to_idx() - 1) * 4 + 1;
                            nnode.add_bel(0, vec![format!("PAD{}", p + 3)]);
                            nnode.add_bel(1, vec![format!("PAD{}", p + 2)]);
                            nnode.add_bel(2, vec![format!("PAD{}", p + 1)]);
                            nnode.add_bel(3, vec![format!("PAD{p}")]);
                            nnode.add_bel(
                                4,
                                vec![
                                    name_a(grid, "TBUF.", ".0", col, row),
                                    name_b(grid, "TBUF_", ".0", col, row),
                                ],
                            );
                            nnode.add_bel(
                                5,
                                vec![
                                    name_a(grid, "TBUF.", ".1", col, row),
                                    name_b(grid, "TBUF_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                6,
                                vec![
                                    name_a(grid, "TBUF.", ".2", col, row),
                                    name_b(grid, "TBUF_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                7,
                                vec![
                                    name_a(grid, "TBUF.", ".3", col, row),
                                    name_b(grid, "TBUF_", ".3", col, row),
                                ],
                            );
                        }
                        "CLKL" | "CLKR" | "CLKH" => {
                            ngrid.name_node(nloc, kind, [(col_x[col].clone(), clk_y.clone())]);
                        }
                        "CLKB" | "CLKT" | "CLKV" => {
                            ngrid.name_node(nloc, kind, [(clk_x.clone(), row_y[row].clone())]);
                        }

                        _ => panic!("umm {kind}"),
                    }
                }
            }
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid,
        grid,
        col_x,
        row_y,
        clk_x: Some(clk_x),
        clk_y: Some(clk_y),
    }
}
