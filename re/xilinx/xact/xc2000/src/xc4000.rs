use prjcombine_interconnect::{
    db::BelId,
    grid::{ColId, DieId, RowId},
};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
};
use unnamed_entity::{EntityId, EntityVec};

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
    if grid.columns <= 22 && grid.rows <= 22 {
        let cidx = u32::try_from(cidx).unwrap();
        let ridx = u32::try_from(ridx).unwrap();
        let r = char::from_u32(u32::from('A') + ridx).unwrap();
        let c = char::from_u32(u32::from('A') + cidx).unwrap();
        format!("{prefix}{r}{c}{suffix}")
    } else {
        format!("{prefix}R{ridx}C{cidx}{suffix}")
    }
}

fn name_b(grid: &Chip, prefix: &str, suffix: &str, col: ColId, row: RowId) -> String {
    let cidx = col.to_idx();
    let ridx = if row < grid.row_mid() && prefix == "TIE_" && grid.kind == ChipKind::Xc4000H {
        grid.rows - row.to_idx()
    } else {
        grid.rows - row.to_idx() - 1
    };
    format!("{prefix}R{ridx}C{cidx}{suffix}")
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let grid = edev.chip;
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
                        "CLB.LB" | "CLB.B" | "CLB.RB" | "CLB.L" | "CLB" | "CLB.R" | "CLB.LT"
                        | "CLB.T" | "CLB.RT" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col].clone(), row_y[row - 1].clone()),
                                    (col_x[col].clone(), row_y[row + 1].clone()),
                                ],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "TIE.", ".1", col, row),
                                name_b(grid, "TIE_", ".1", col, row),
                            ];
                            if kind == "CLB.LB" {
                                nnode
                                    .coords
                                    .push((col_x[col - 1].clone(), row_y[row - 1].clone()));
                            } else if kind == "CLB.B"
                                || kind == "CLB.RB"
                                || kind == "CLB.T"
                                || kind == "CLB.RT"
                            {
                                nnode
                                    .coords
                                    .push((col_x[col - 1].clone(), row_y[row].clone()));
                            } else if kind == "CLB.LT" {
                                nnode
                                    .coords
                                    .push((col_x[col - 1].clone(), row_y[row + 1].clone()));
                            }

                            nnode.add_bel(
                                0,
                                vec![
                                    name_a(grid, "", "", col, row),
                                    name_b(grid, "CLB_", "", col, row),
                                ],
                            );
                            nnode.add_bel(
                                1,
                                vec![
                                    name_a(grid, "TBUF.", ".2", col, row),
                                    name_b(grid, "TBUF_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                2,
                                vec![
                                    name_a(grid, "TBUF.", ".1", col, row),
                                    name_b(grid, "TBUF_", ".1", col, row),
                                ],
                            );
                        }
                        "IO.B" | "IO.B.R" | "IO.BS" | "IO.BS.L" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col].clone(), row_y[row + 1].clone()),
                                    (col_x[col + 1].clone(), row_y[row].clone()),
                                ],
                            );
                            nnode.tie_names = vec![
                                name_a(grid, "TIE.", ".1", col, row),
                                name_b(grid, "TIE_", ".1", col, row),
                            ];
                            let bidx = if grid.kind == ChipKind::Xc4000H {
                                let p = (grid.columns - 2) * 4
                                    + (grid.rows - 2) * 4
                                    + (grid.col_rio().to_idx() - col.to_idx() - 1) * 4
                                    + 1;
                                nnode.add_bel(0, vec![format!("PAD{}", p + 3)]);
                                nnode.add_bel(1, vec![format!("PAD{}", p + 2)]);
                                nnode.add_bel(2, vec![format!("PAD{}", p + 1)]);
                                nnode.add_bel(3, vec![format!("PAD{p}")]);
                                4
                            } else {
                                let p = (grid.columns - 2) * 2
                                    + (grid.rows - 2) * 2
                                    + (grid.col_rio().to_idx() - col.to_idx() - 1) * 2
                                    + 1;
                                nnode.add_bel(0, vec![format!("PAD{}", p + 1)]);
                                nnode.add_bel(1, vec![format!("PAD{p}")]);
                                2
                            };
                            if kind == "IO.B.R" {
                                nnode.bels[BelId::from_idx(bidx - 1)].push("i_bufgs_br".into());
                            }
                            if kind == "IO.BS.L" {
                                nnode.bels[BelId::from_idx(0)].push("i_bufgp_bl".into());
                            }
                            nnode.add_bel(
                                bidx,
                                vec![
                                    name_a(grid, "DEC.", ".1", col, row),
                                    name_b(grid, "DEC_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 1,
                                vec![
                                    name_a(grid, "DEC.", ".2", col, row),
                                    name_b(grid, "DEC_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 2,
                                vec![
                                    name_a(grid, "DEC.", ".3", col, row),
                                    name_b(grid, "DEC_", ".3", col, row),
                                ],
                            );
                        }
                        "IO.T" | "IO.T.R" | "IO.TS" | "IO.TS.L" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col + 1].clone(), row_y[row].clone()),
                                ],
                            );
                            let bidx = if grid.kind == ChipKind::Xc4000H {
                                let p = (col.to_idx() - 1) * 4 + 1;
                                nnode.add_bel(0, vec![format!("PAD{p}")]);
                                nnode.add_bel(1, vec![format!("PAD{}", p + 1)]);
                                nnode.add_bel(2, vec![format!("PAD{}", p + 2)]);
                                nnode.add_bel(3, vec![format!("PAD{}", p + 3)]);
                                4
                            } else {
                                let p = (col.to_idx() - 1) * 2 + 1;
                                nnode.add_bel(0, vec![format!("PAD{p}")]);
                                nnode.add_bel(1, vec![format!("PAD{}", p + 1)]);
                                2
                            };
                            if kind == "IO.T.R" {
                                nnode.bels[BelId::from_idx(bidx - 2)].push("i_bufgp_tr".into());
                            }
                            if kind == "IO.TS.L" {
                                nnode.bels[BelId::from_idx(0)].push("i_bufgs_tl".into());
                            }
                            nnode.add_bel(
                                bidx,
                                vec![
                                    name_a(grid, "DEC.", ".1", col, row),
                                    name_b(grid, "DEC_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 1,
                                vec![
                                    name_a(grid, "DEC.", ".2", col, row),
                                    name_b(grid, "DEC_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 2,
                                vec![
                                    name_a(grid, "DEC.", ".3", col, row),
                                    name_b(grid, "DEC_", ".3", col, row),
                                ],
                            );
                        }
                        "IO.L" | "IO.L.T" | "IO.LS" | "IO.LS.B" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col].clone(), row_y[row - 1].clone()),
                                ],
                            );
                            let bidx = if grid.kind == ChipKind::Xc4000H {
                                let p = (grid.columns - 2) * 8
                                    + (grid.rows - 2) * 4
                                    + (row.to_idx() - 1) * 4
                                    + 1;
                                nnode.add_bel(0, vec![format!("PAD{}", p + 3)]);
                                nnode.add_bel(1, vec![format!("PAD{}", p + 2)]);
                                nnode.add_bel(2, vec![format!("PAD{}", p + 1)]);
                                nnode.add_bel(3, vec![format!("PAD{p}")]);
                                4
                            } else {
                                let p = (grid.columns - 2) * 4
                                    + (grid.rows - 2) * 2
                                    + (row.to_idx() - 1) * 2
                                    + 1;
                                nnode.add_bel(0, vec![format!("PAD{}", p + 1)]);
                                nnode.add_bel(1, vec![format!("PAD{p}")]);
                                2
                            };
                            if kind == "IO.L.T" {
                                nnode.bels[BelId::from_idx(0)].push("i_bufgp_tl".into());
                            }
                            if kind == "IO.LS.B" {
                                nnode.bels[BelId::from_idx(bidx - 1)].push("i_bufgs_bl".into());
                            }
                            nnode.add_bel(
                                bidx,
                                vec![
                                    name_a(grid, "TBUF.", ".2", col, row),
                                    name_b(grid, "TBUF_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 1,
                                vec![
                                    name_a(grid, "TBUF.", ".1", col, row),
                                    name_b(grid, "TBUF_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 2,
                                vec![
                                    name_a(grid, "PULLUP.", ".2", col, row),
                                    name_b(grid, "PULLUP_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 3,
                                vec![
                                    name_a(grid, "PULLUP.", ".1", col, row),
                                    name_b(grid, "PULLUP_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 4,
                                vec![
                                    name_a(grid, "DEC.", ".1", col, row),
                                    name_b(grid, "DEC_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 5,
                                vec![
                                    name_a(grid, "DEC.", ".2", col, row),
                                    name_b(grid, "DEC_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 6,
                                vec![
                                    name_a(grid, "DEC.", ".3", col, row),
                                    name_b(grid, "DEC_", ".3", col, row),
                                ],
                            );
                        }
                        "IO.R" | "IO.R.T" | "IO.RS" | "IO.RS.B" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col].clone(), row_y[row - 1].clone()),
                                ],
                            );
                            if grid.kind != ChipKind::Xc4000A {
                                nnode.tie_names = vec![
                                    name_a(grid, "TIE.", ".1", col, row),
                                    name_b(grid, "TIE_", ".1", col, row),
                                ];
                            }

                            let bidx = if grid.kind == ChipKind::Xc4000H {
                                let p = (grid.columns - 2) * 4
                                    + (grid.row_tio().to_idx() - row.to_idx() - 1) * 4
                                    + 1;
                                nnode.add_bel(0, vec![format!("PAD{p}")]);
                                nnode.add_bel(1, vec![format!("PAD{}", p + 1)]);
                                nnode.add_bel(2, vec![format!("PAD{}", p + 2)]);
                                nnode.add_bel(3, vec![format!("PAD{}", p + 3)]);
                                4
                            } else {
                                let p = (grid.columns - 2) * 2
                                    + (grid.row_tio().to_idx() - row.to_idx() - 1) * 2
                                    + 1;
                                nnode.add_bel(0, vec![format!("PAD{p}")]);
                                nnode.add_bel(1, vec![format!("PAD{}", p + 1)]);
                                2
                            };
                            if kind == "IO.R.T" {
                                nnode.bels[BelId::from_idx(0)].push("i_bufgs_tr".into());
                            }
                            if kind == "IO.RS.B" {
                                nnode.bels[BelId::from_idx(bidx - 2)].push("i_bufgp_br".into());
                            }
                            nnode.add_bel(
                                bidx,
                                vec![
                                    name_a(grid, "TBUF.", ".2", col, row),
                                    name_b(grid, "TBUF_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 1,
                                vec![
                                    name_a(grid, "TBUF.", ".1", col, row),
                                    name_b(grid, "TBUF_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 2,
                                vec![
                                    name_a(grid, "PULLUP.", ".2", col, row),
                                    name_b(grid, "PULLUP_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 3,
                                vec![
                                    name_a(grid, "PULLUP.", ".1", col, row),
                                    name_b(grid, "PULLUP_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 4,
                                vec![
                                    name_a(grid, "DEC.", ".1", col, row),
                                    name_b(grid, "DEC_", ".1", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 5,
                                vec![
                                    name_a(grid, "DEC.", ".2", col, row),
                                    name_b(grid, "DEC_", ".2", col, row),
                                ],
                            );
                            nnode.add_bel(
                                bidx + 6,
                                vec![
                                    name_a(grid, "DEC.", ".3", col, row),
                                    name_b(grid, "DEC_", ".3", col, row),
                                ],
                            );
                        }

                        "CNR.BL" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col + 1].clone(), row_y[row].clone()),
                                ],
                            );

                            let bidx = if grid.kind == ChipKind::Xc4000A {
                                nnode.add_bel(
                                    0,
                                    vec![
                                        name_a(grid, "PULLUP.", ".1", col, row),
                                        name_b(grid, "PULLUP_", ".1", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    1,
                                    vec![
                                        name_a(grid, "PULLUP.", ".0", col, row),
                                        name_b(grid, "PULLUP_", ".0", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    2,
                                    vec![
                                        name_a(grid, "PULLUP.", ".2", col, row),
                                        name_b(grid, "PULLUP_", ".2", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    3,
                                    vec![
                                        name_a(grid, "PULLUP.", ".3", col, row),
                                        name_b(grid, "PULLUP_", ".3", col, row),
                                    ],
                                );
                                4
                            } else {
                                nnode.add_bel(
                                    0,
                                    vec![
                                        name_a(grid, "PULLUP.", ".8", col, row),
                                        name_b(grid, "PULLUP_", ".8", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    1,
                                    vec![
                                        name_a(grid, "PULLUP.", ".7", col, row),
                                        name_b(grid, "PULLUP_", ".7", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    2,
                                    vec![
                                        name_a(grid, "PULLUP.", ".6", col, row),
                                        name_b(grid, "PULLUP_", ".6", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    3,
                                    vec![
                                        name_a(grid, "PULLUP.", ".5", col, row),
                                        name_b(grid, "PULLUP_", ".5", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    4,
                                    vec![
                                        name_a(grid, "PULLUP.", ".4", col, row),
                                        name_b(grid, "PULLUP_", ".4", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    5,
                                    vec![
                                        name_a(grid, "PULLUP.", ".3", col, row),
                                        name_b(grid, "PULLUP_", ".3", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    6,
                                    vec![
                                        name_a(grid, "PULLUP.", ".2", col, row),
                                        name_b(grid, "PULLUP_", ".2", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    7,
                                    vec![
                                        name_a(grid, "PULLUP.", ".1", col, row),
                                        name_b(grid, "PULLUP_", ".1", col, row),
                                    ],
                                );
                                8
                            };
                            nnode.add_bel(bidx, vec!["bufgp_bl".to_string()]);
                            nnode.add_bel(bidx + 1, vec!["bufgs_bl".to_string()]);
                            nnode.add_bel(bidx + 2, vec!["ci_bl".to_string()]);
                            nnode.add_bel(bidx + 3, vec!["md0".to_string()]);
                            nnode.add_bel(bidx + 4, vec!["md1".to_string()]);
                            nnode.add_bel(bidx + 5, vec!["md2".to_string()]);
                            nnode.add_bel(bidx + 6, vec!["rdbk".to_string()]);
                        }
                        "CNR.TL" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col + 1].clone(), row_y[row].clone()),
                                    (col_x[col].clone(), row_y[row - 1].clone()),
                                ],
                            );

                            let bidx = if grid.kind == ChipKind::Xc4000A {
                                nnode.add_bel(
                                    0,
                                    vec![
                                        name_a(grid, "PULLUP.", ".1", col, row),
                                        name_b(grid, "PULLUP_", ".1", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    1,
                                    vec![
                                        name_a(grid, "PULLUP.", ".0", col, row),
                                        name_b(grid, "PULLUP_", ".0", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    2,
                                    vec![
                                        name_a(grid, "PULLUP.", ".2", col, row),
                                        name_b(grid, "PULLUP_", ".2", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    3,
                                    vec![
                                        name_a(grid, "PULLUP.", ".3", col, row),
                                        name_b(grid, "PULLUP_", ".3", col, row),
                                    ],
                                );
                                4
                            } else {
                                nnode.add_bel(
                                    0,
                                    vec![
                                        name_a(grid, "PULLUP.", ".1", col, row),
                                        name_b(grid, "PULLUP_", ".1", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    1,
                                    vec![
                                        name_a(grid, "PULLUP.", ".2", col, row),
                                        name_b(grid, "PULLUP_", ".2", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    2,
                                    vec![
                                        name_a(grid, "PULLUP.", ".3", col, row),
                                        name_b(grid, "PULLUP_", ".3", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    3,
                                    vec![
                                        name_a(grid, "PULLUP.", ".4", col, row),
                                        name_b(grid, "PULLUP_", ".4", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    4,
                                    vec![
                                        name_a(grid, "PULLUP.", ".5", col, row),
                                        name_b(grid, "PULLUP_", ".5", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    5,
                                    vec![
                                        name_a(grid, "PULLUP.", ".6", col, row),
                                        name_b(grid, "PULLUP_", ".6", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    6,
                                    vec![
                                        name_a(grid, "PULLUP.", ".7", col, row),
                                        name_b(grid, "PULLUP_", ".7", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    7,
                                    vec![
                                        name_a(grid, "PULLUP.", ".8", col, row),
                                        name_b(grid, "PULLUP_", ".8", col, row),
                                    ],
                                );
                                8
                            };
                            nnode.add_bel(bidx, vec!["bufgs_tl".to_string()]);
                            nnode.add_bel(bidx + 1, vec!["bufgp_tl".to_string()]);
                            nnode.add_bel(bidx + 2, vec!["ci_tl".to_string()]);
                            nnode.add_bel(bidx + 3, vec!["bscan".to_string()]);
                        }
                        "CNR.BR" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col - 1].clone(), row_y[row + 1].clone()),
                                ],
                            );

                            if grid.kind != ChipKind::Xc4000A {
                                nnode.tie_names = vec![
                                    name_a(grid, "TIE.", ".1", col, row),
                                    name_b(grid, "TIE_", ".1", col, row),
                                ];
                            }

                            let bidx = if grid.kind == ChipKind::Xc4000A {
                                nnode.add_bel(
                                    0,
                                    vec![
                                        name_a(grid, "PULLUP.", ".1", col, row),
                                        name_b(grid, "PULLUP_", ".1", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    1,
                                    vec![
                                        name_a(grid, "PULLUP.", ".0", col, row),
                                        name_b(grid, "PULLUP_", ".0", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    2,
                                    vec![
                                        name_a(grid, "PULLUP.", ".2", col, row),
                                        name_b(grid, "PULLUP_", ".2", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    3,
                                    vec![
                                        name_a(grid, "PULLUP.", ".3", col, row),
                                        name_b(grid, "PULLUP_", ".3", col, row),
                                    ],
                                );
                                4
                            } else {
                                nnode.add_bel(
                                    0,
                                    vec![
                                        name_a(grid, "PULLUP.", ".8", col, row),
                                        name_b(grid, "PULLUP_", ".8", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    1,
                                    vec![
                                        name_a(grid, "PULLUP.", ".7", col, row),
                                        name_b(grid, "PULLUP_", ".7", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    2,
                                    vec![
                                        name_a(grid, "PULLUP.", ".6", col, row),
                                        name_b(grid, "PULLUP_", ".6", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    3,
                                    vec![
                                        name_a(grid, "PULLUP.", ".5", col, row),
                                        name_b(grid, "PULLUP_", ".5", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    4,
                                    vec![
                                        name_a(grid, "PULLUP.", ".4", col, row),
                                        name_b(grid, "PULLUP_", ".4", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    5,
                                    vec![
                                        name_a(grid, "PULLUP.", ".3", col, row),
                                        name_b(grid, "PULLUP_", ".3", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    6,
                                    vec![
                                        name_a(grid, "PULLUP.", ".2", col, row),
                                        name_b(grid, "PULLUP_", ".2", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    7,
                                    vec![
                                        name_a(grid, "PULLUP.", ".1", col, row),
                                        name_b(grid, "PULLUP_", ".1", col, row),
                                    ],
                                );
                                8
                            };
                            nnode.add_bel(bidx, vec!["bufgs_br".to_string()]);
                            nnode.add_bel(bidx + 1, vec!["bufgp_br".to_string()]);
                            nnode.add_bel(bidx + 2, vec!["co_br".to_string()]);
                            nnode.add_bel(bidx + 3, vec!["startup".to_string()]);
                            nnode.add_bel(bidx + 4, vec!["rdclk".to_string()]);
                        }
                        "CNR.TR" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    (col_x[col].clone(), row_y[row].clone()),
                                    (col_x[col].clone(), row_y[row - 1].clone()),
                                    (col_x[col - 1].clone(), row_y[row - 1].clone()),
                                ],
                            );

                            let bidx = if grid.kind == ChipKind::Xc4000A {
                                nnode.add_bel(
                                    0,
                                    vec![
                                        name_a(grid, "PULLUP.", ".1", col, row),
                                        name_b(grid, "PULLUP_", ".1", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    1,
                                    vec![
                                        name_a(grid, "PULLUP.", ".0", col, row),
                                        name_b(grid, "PULLUP_", ".0", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    2,
                                    vec![
                                        name_a(grid, "PULLUP.", ".2", col, row),
                                        name_b(grid, "PULLUP_", ".2", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    3,
                                    vec![
                                        name_a(grid, "PULLUP.", ".3", col, row),
                                        name_b(grid, "PULLUP_", ".3", col, row),
                                    ],
                                );
                                4
                            } else {
                                nnode.add_bel(
                                    0,
                                    vec![
                                        name_a(grid, "PULLUP.", ".1", col, row),
                                        name_b(grid, "PULLUP_", ".1", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    1,
                                    vec![
                                        name_a(grid, "PULLUP.", ".2", col, row),
                                        name_b(grid, "PULLUP_", ".2", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    2,
                                    vec![
                                        name_a(grid, "PULLUP.", ".3", col, row),
                                        name_b(grid, "PULLUP_", ".3", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    3,
                                    vec![
                                        name_a(grid, "PULLUP.", ".4", col, row),
                                        name_b(grid, "PULLUP_", ".4", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    4,
                                    vec![
                                        name_a(grid, "PULLUP.", ".5", col, row),
                                        name_b(grid, "PULLUP_", ".5", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    5,
                                    vec![
                                        name_a(grid, "PULLUP.", ".6", col, row),
                                        name_b(grid, "PULLUP_", ".6", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    6,
                                    vec![
                                        name_a(grid, "PULLUP.", ".7", col, row),
                                        name_b(grid, "PULLUP_", ".7", col, row),
                                    ],
                                );
                                nnode.add_bel(
                                    7,
                                    vec![
                                        name_a(grid, "PULLUP.", ".8", col, row),
                                        name_b(grid, "PULLUP_", ".8", col, row),
                                    ],
                                );
                                8
                            };
                            nnode.add_bel(bidx, vec!["bufgp_tr".to_string()]);
                            nnode.add_bel(bidx + 1, vec!["bufgs_tr".to_string()]);
                            nnode.add_bel(bidx + 2, vec!["co_tr".to_string()]);
                            nnode.add_bel(bidx + 3, vec!["update".to_string()]);
                            nnode.add_bel(bidx + 4, vec!["osc".to_string()]);
                            nnode.add_bel(bidx + 5, vec!["tdo".to_string()]);
                        }

                        "LLV.IO.L" | "LLV.IO.R" | "LLV.CLB" => {
                            let nnode =
                                ngrid.name_node(nloc, kind, [(col_x[col].clone(), clk_y.clone())]);
                            if grid.kind == ChipKind::Xc4000H {
                                let cidx = if col < grid.col_mid() {
                                    col.to_idx()
                                } else {
                                    col.to_idx() + 1
                                };
                                let ridx = grid.rows - row.to_idx();
                                let cidx = u32::try_from(cidx).unwrap();
                                let ridx = u32::try_from(ridx).unwrap();
                                let r = char::from_u32(u32::from('A') + ridx).unwrap();
                                let c = char::from_u32(u32::from('A') + cidx).unwrap();
                                nnode.add_bel(0, vec![format!("SRC0.{r}{c}.1")]);
                            }
                        }
                        "LLH.IO.B" | "LLH.IO.T" | "LLH.CLB" | "LLH.CLB.B" => {
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
