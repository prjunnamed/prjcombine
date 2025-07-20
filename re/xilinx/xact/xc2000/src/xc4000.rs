use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{
    bels::xc4000 as bels,
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

    let die = DieId::from_idx(0);
    let mut col_x = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut clk_x = 0..0;
    let mut clk_y = 0..0;
    let mut x = 0;
    for col in egrid.cols(die) {
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
    for row in egrid.rows(die) {
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
    for (tcrd, tile) in egrid.tiles() {
        let col = tcrd.col;
        let row = tcrd.row;
        let kind = egrid.db.tile_classes.key(tile.class);
        match &kind[..] {
            "CLB.LB" | "CLB.B" | "CLB.RB" | "CLB.L" | "CLB" | "CLB.R" | "CLB.LT" | "CLB.T"
            | "CLB.RT" => {
                let nnode = ngrid.name_node(
                    tcrd,
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
                } else if kind == "CLB.B" || kind == "CLB.RB" || kind == "CLB.T" || kind == "CLB.RT"
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
                    bels::CLB,
                    vec![
                        name_a(grid, "", "", col, row),
                        name_b(grid, "CLB_", "", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::TBUF0,
                    vec![
                        name_a(grid, "TBUF.", ".2", col, row),
                        name_b(grid, "TBUF_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::TBUF1,
                    vec![
                        name_a(grid, "TBUF.", ".1", col, row),
                        name_b(grid, "TBUF_", ".1", col, row),
                    ],
                );
            }
            "IO.B" | "IO.B.R" | "IO.BS" | "IO.BS.L" => {
                let nnode = ngrid.name_node(
                    tcrd,
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
                let (slot0, slot1) = if grid.kind == ChipKind::Xc4000H {
                    let p = (grid.columns - 2) * 4
                        + (grid.rows - 2) * 4
                        + (grid.col_e().to_idx() - col.to_idx() - 1) * 4
                        + 1;
                    nnode.add_bel(bels::HIO0, vec![format!("PAD{}", p + 3)]);
                    nnode.add_bel(bels::HIO1, vec![format!("PAD{}", p + 2)]);
                    nnode.add_bel(bels::HIO2, vec![format!("PAD{}", p + 1)]);
                    nnode.add_bel(bels::HIO3, vec![format!("PAD{p}")]);
                    (bels::HIO0, bels::HIO3)
                } else {
                    let p = (grid.columns - 2) * 2
                        + (grid.rows - 2) * 2
                        + (grid.col_e().to_idx() - col.to_idx() - 1) * 2
                        + 1;
                    nnode.add_bel(bels::IO0, vec![format!("PAD{}", p + 1)]);
                    nnode.add_bel(bels::IO1, vec![format!("PAD{p}")]);
                    (bels::IO0, bels::IO1)
                };
                if kind == "IO.B.R" {
                    nnode.bels[slot1].push("i_bufgs_br".into());
                }
                if kind == "IO.BS.L" {
                    nnode.bels[slot0].push("i_bufgp_bl".into());
                }
                nnode.add_bel(
                    bels::DEC0,
                    vec![
                        name_a(grid, "DEC.", ".1", col, row),
                        name_b(grid, "DEC_", ".1", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC1,
                    vec![
                        name_a(grid, "DEC.", ".2", col, row),
                        name_b(grid, "DEC_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC2,
                    vec![
                        name_a(grid, "DEC.", ".3", col, row),
                        name_b(grid, "DEC_", ".3", col, row),
                    ],
                );
            }
            "IO.T" | "IO.T.R" | "IO.TS" | "IO.TS.L" => {
                let nnode = ngrid.name_node(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col + 1].clone(), row_y[row].clone()),
                    ],
                );
                let (slot0, slot1) = if grid.kind == ChipKind::Xc4000H {
                    let p = (col.to_idx() - 1) * 4 + 1;
                    nnode.add_bel(bels::HIO0, vec![format!("PAD{p}")]);
                    nnode.add_bel(bels::HIO1, vec![format!("PAD{}", p + 1)]);
                    nnode.add_bel(bels::HIO2, vec![format!("PAD{}", p + 2)]);
                    nnode.add_bel(bels::HIO3, vec![format!("PAD{}", p + 3)]);
                    (bels::HIO0, bels::HIO2)
                } else {
                    let p = (col.to_idx() - 1) * 2 + 1;
                    nnode.add_bel(bels::IO0, vec![format!("PAD{p}")]);
                    nnode.add_bel(bels::IO1, vec![format!("PAD{}", p + 1)]);
                    (bels::IO0, bels::IO0)
                };
                if kind == "IO.T.R" {
                    nnode.bels[slot1].push("i_bufgp_tr".into());
                }
                if kind == "IO.TS.L" {
                    nnode.bels[slot0].push("i_bufgs_tl".into());
                }
                nnode.add_bel(
                    bels::DEC0,
                    vec![
                        name_a(grid, "DEC.", ".1", col, row),
                        name_b(grid, "DEC_", ".1", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC1,
                    vec![
                        name_a(grid, "DEC.", ".2", col, row),
                        name_b(grid, "DEC_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC2,
                    vec![
                        name_a(grid, "DEC.", ".3", col, row),
                        name_b(grid, "DEC_", ".3", col, row),
                    ],
                );
            }
            "IO.L" | "IO.L.T" | "IO.LS" | "IO.LS.B" => {
                let nnode = ngrid.name_node(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row - 1].clone()),
                    ],
                );
                let (slot0, slot1) = if grid.kind == ChipKind::Xc4000H {
                    let p =
                        (grid.columns - 2) * 8 + (grid.rows - 2) * 4 + (row.to_idx() - 1) * 4 + 1;
                    nnode.add_bel(bels::HIO0, vec![format!("PAD{}", p + 3)]);
                    nnode.add_bel(bels::HIO1, vec![format!("PAD{}", p + 2)]);
                    nnode.add_bel(bels::HIO2, vec![format!("PAD{}", p + 1)]);
                    nnode.add_bel(bels::HIO3, vec![format!("PAD{p}")]);
                    (bels::HIO0, bels::HIO3)
                } else {
                    let p =
                        (grid.columns - 2) * 4 + (grid.rows - 2) * 2 + (row.to_idx() - 1) * 2 + 1;
                    nnode.add_bel(bels::IO0, vec![format!("PAD{}", p + 1)]);
                    nnode.add_bel(bels::IO1, vec![format!("PAD{p}")]);
                    (bels::IO0, bels::IO1)
                };
                if kind == "IO.L.T" {
                    nnode.bels[slot0].push("i_bufgp_tl".into());
                }
                if kind == "IO.LS.B" {
                    nnode.bels[slot1].push("i_bufgs_bl".into());
                }
                nnode.add_bel(
                    bels::TBUF0,
                    vec![
                        name_a(grid, "TBUF.", ".2", col, row),
                        name_b(grid, "TBUF_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::TBUF1,
                    vec![
                        name_a(grid, "TBUF.", ".1", col, row),
                        name_b(grid, "TBUF_", ".1", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::PULLUP_TBUF0,
                    vec![
                        name_a(grid, "PULLUP.", ".2", col, row),
                        name_b(grid, "PULLUP_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::PULLUP_TBUF1,
                    vec![
                        name_a(grid, "PULLUP.", ".1", col, row),
                        name_b(grid, "PULLUP_", ".1", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC0,
                    vec![
                        name_a(grid, "DEC.", ".1", col, row),
                        name_b(grid, "DEC_", ".1", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC1,
                    vec![
                        name_a(grid, "DEC.", ".2", col, row),
                        name_b(grid, "DEC_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC2,
                    vec![
                        name_a(grid, "DEC.", ".3", col, row),
                        name_b(grid, "DEC_", ".3", col, row),
                    ],
                );
            }
            "IO.R" | "IO.R.T" | "IO.RS" | "IO.RS.B" => {
                let nnode = ngrid.name_node(
                    tcrd,
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

                let (slot0, slot1) = if grid.kind == ChipKind::Xc4000H {
                    let p =
                        (grid.columns - 2) * 4 + (grid.row_n().to_idx() - row.to_idx() - 1) * 4 + 1;
                    nnode.add_bel(bels::HIO0, vec![format!("PAD{p}")]);
                    nnode.add_bel(bels::HIO1, vec![format!("PAD{}", p + 1)]);
                    nnode.add_bel(bels::HIO2, vec![format!("PAD{}", p + 2)]);
                    nnode.add_bel(bels::HIO3, vec![format!("PAD{}", p + 3)]);
                    (bels::HIO0, bels::HIO2)
                } else {
                    let p =
                        (grid.columns - 2) * 2 + (grid.row_n().to_idx() - row.to_idx() - 1) * 2 + 1;
                    nnode.add_bel(bels::IO0, vec![format!("PAD{p}")]);
                    nnode.add_bel(bels::IO1, vec![format!("PAD{}", p + 1)]);
                    (bels::IO0, bels::IO0)
                };
                if kind == "IO.R.T" {
                    nnode.bels[slot0].push("i_bufgs_tr".into());
                }
                if kind == "IO.RS.B" {
                    nnode.bels[slot1].push("i_bufgp_br".into());
                }
                nnode.add_bel(
                    bels::TBUF0,
                    vec![
                        name_a(grid, "TBUF.", ".2", col, row),
                        name_b(grid, "TBUF_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::TBUF1,
                    vec![
                        name_a(grid, "TBUF.", ".1", col, row),
                        name_b(grid, "TBUF_", ".1", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::PULLUP_TBUF0,
                    vec![
                        name_a(grid, "PULLUP.", ".2", col, row),
                        name_b(grid, "PULLUP_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::PULLUP_TBUF1,
                    vec![
                        name_a(grid, "PULLUP.", ".1", col, row),
                        name_b(grid, "PULLUP_", ".1", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC0,
                    vec![
                        name_a(grid, "DEC.", ".1", col, row),
                        name_b(grid, "DEC_", ".1", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC1,
                    vec![
                        name_a(grid, "DEC.", ".2", col, row),
                        name_b(grid, "DEC_", ".2", col, row),
                    ],
                );
                nnode.add_bel(
                    bels::DEC2,
                    vec![
                        name_a(grid, "DEC.", ".3", col, row),
                        name_b(grid, "DEC_", ".3", col, row),
                    ],
                );
            }

            "CNR.BL" => {
                let nnode = ngrid.name_node(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col + 1].clone(), row_y[row].clone()),
                    ],
                );

                if grid.kind == ChipKind::Xc4000A {
                    nnode.add_bel(
                        bels::PULLUP_DEC0_H,
                        vec![
                            name_a(grid, "PULLUP.", ".1", col, row),
                            name_b(grid, "PULLUP_", ".1", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_H,
                        vec![
                            name_a(grid, "PULLUP.", ".0", col, row),
                            name_b(grid, "PULLUP_", ".0", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC0_V,
                        vec![
                            name_a(grid, "PULLUP.", ".2", col, row),
                            name_b(grid, "PULLUP_", ".2", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_V,
                        vec![
                            name_a(grid, "PULLUP.", ".3", col, row),
                            name_b(grid, "PULLUP_", ".3", col, row),
                        ],
                    );
                } else {
                    nnode.add_bel(
                        bels::PULLUP_DEC0_H,
                        vec![
                            name_a(grid, "PULLUP.", ".8", col, row),
                            name_b(grid, "PULLUP_", ".8", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_H,
                        vec![
                            name_a(grid, "PULLUP.", ".7", col, row),
                            name_b(grid, "PULLUP_", ".7", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC2_H,
                        vec![
                            name_a(grid, "PULLUP.", ".6", col, row),
                            name_b(grid, "PULLUP_", ".6", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC3_H,
                        vec![
                            name_a(grid, "PULLUP.", ".5", col, row),
                            name_b(grid, "PULLUP_", ".5", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC0_V,
                        vec![
                            name_a(grid, "PULLUP.", ".4", col, row),
                            name_b(grid, "PULLUP_", ".4", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_V,
                        vec![
                            name_a(grid, "PULLUP.", ".3", col, row),
                            name_b(grid, "PULLUP_", ".3", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC2_V,
                        vec![
                            name_a(grid, "PULLUP.", ".2", col, row),
                            name_b(grid, "PULLUP_", ".2", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC3_V,
                        vec![
                            name_a(grid, "PULLUP.", ".1", col, row),
                            name_b(grid, "PULLUP_", ".1", col, row),
                        ],
                    );
                }
                nnode.add_bel(bels::BUFGLS_H, vec!["bufgp_bl".to_string()]);
                nnode.add_bel(bels::BUFGLS_V, vec!["bufgs_bl".to_string()]);
                nnode.add_bel(bels::CIN, vec!["ci_bl".to_string()]);
                nnode.add_bel(bels::MD0, vec!["md0".to_string()]);
                nnode.add_bel(bels::MD1, vec!["md1".to_string()]);
                nnode.add_bel(bels::MD2, vec!["md2".to_string()]);
                nnode.add_bel(bels::RDBK, vec!["rdbk".to_string()]);
            }
            "CNR.TL" => {
                let nnode = ngrid.name_node(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col + 1].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row - 1].clone()),
                    ],
                );

                if grid.kind == ChipKind::Xc4000A {
                    nnode.add_bel(
                        bels::PULLUP_DEC0_H,
                        vec![
                            name_a(grid, "PULLUP.", ".1", col, row),
                            name_b(grid, "PULLUP_", ".1", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_H,
                        vec![
                            name_a(grid, "PULLUP.", ".0", col, row),
                            name_b(grid, "PULLUP_", ".0", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC0_V,
                        vec![
                            name_a(grid, "PULLUP.", ".2", col, row),
                            name_b(grid, "PULLUP_", ".2", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_V,
                        vec![
                            name_a(grid, "PULLUP.", ".3", col, row),
                            name_b(grid, "PULLUP_", ".3", col, row),
                        ],
                    );
                } else {
                    nnode.add_bel(
                        bels::PULLUP_DEC0_H,
                        vec![
                            name_a(grid, "PULLUP.", ".1", col, row),
                            name_b(grid, "PULLUP_", ".1", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_H,
                        vec![
                            name_a(grid, "PULLUP.", ".2", col, row),
                            name_b(grid, "PULLUP_", ".2", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC2_H,
                        vec![
                            name_a(grid, "PULLUP.", ".3", col, row),
                            name_b(grid, "PULLUP_", ".3", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC3_H,
                        vec![
                            name_a(grid, "PULLUP.", ".4", col, row),
                            name_b(grid, "PULLUP_", ".4", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC0_V,
                        vec![
                            name_a(grid, "PULLUP.", ".5", col, row),
                            name_b(grid, "PULLUP_", ".5", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_V,
                        vec![
                            name_a(grid, "PULLUP.", ".6", col, row),
                            name_b(grid, "PULLUP_", ".6", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC2_V,
                        vec![
                            name_a(grid, "PULLUP.", ".7", col, row),
                            name_b(grid, "PULLUP_", ".7", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC3_V,
                        vec![
                            name_a(grid, "PULLUP.", ".8", col, row),
                            name_b(grid, "PULLUP_", ".8", col, row),
                        ],
                    );
                }
                nnode.add_bel(bels::BUFGLS_H, vec!["bufgs_tl".to_string()]);
                nnode.add_bel(bels::BUFGLS_V, vec!["bufgp_tl".to_string()]);
                nnode.add_bel(bels::CIN, vec!["ci_tl".to_string()]);
                nnode.add_bel(bels::BSCAN, vec!["bscan".to_string()]);
            }
            "CNR.BR" => {
                let nnode = ngrid.name_node(
                    tcrd,
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

                if grid.kind == ChipKind::Xc4000A {
                    nnode.add_bel(
                        bels::PULLUP_DEC0_H,
                        vec![
                            name_a(grid, "PULLUP.", ".1", col, row),
                            name_b(grid, "PULLUP_", ".1", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_H,
                        vec![
                            name_a(grid, "PULLUP.", ".0", col, row),
                            name_b(grid, "PULLUP_", ".0", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC0_V,
                        vec![
                            name_a(grid, "PULLUP.", ".2", col, row),
                            name_b(grid, "PULLUP_", ".2", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_V,
                        vec![
                            name_a(grid, "PULLUP.", ".3", col, row),
                            name_b(grid, "PULLUP_", ".3", col, row),
                        ],
                    );
                } else {
                    nnode.add_bel(
                        bels::PULLUP_DEC0_H,
                        vec![
                            name_a(grid, "PULLUP.", ".8", col, row),
                            name_b(grid, "PULLUP_", ".8", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_H,
                        vec![
                            name_a(grid, "PULLUP.", ".7", col, row),
                            name_b(grid, "PULLUP_", ".7", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC2_H,
                        vec![
                            name_a(grid, "PULLUP.", ".6", col, row),
                            name_b(grid, "PULLUP_", ".6", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC3_H,
                        vec![
                            name_a(grid, "PULLUP.", ".5", col, row),
                            name_b(grid, "PULLUP_", ".5", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC0_V,
                        vec![
                            name_a(grid, "PULLUP.", ".4", col, row),
                            name_b(grid, "PULLUP_", ".4", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_V,
                        vec![
                            name_a(grid, "PULLUP.", ".3", col, row),
                            name_b(grid, "PULLUP_", ".3", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC2_V,
                        vec![
                            name_a(grid, "PULLUP.", ".2", col, row),
                            name_b(grid, "PULLUP_", ".2", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC3_V,
                        vec![
                            name_a(grid, "PULLUP.", ".1", col, row),
                            name_b(grid, "PULLUP_", ".1", col, row),
                        ],
                    );
                }
                nnode.add_bel(bels::BUFGLS_H, vec!["bufgs_br".to_string()]);
                nnode.add_bel(bels::BUFGLS_V, vec!["bufgp_br".to_string()]);
                nnode.add_bel(bels::COUT, vec!["co_br".to_string()]);
                nnode.add_bel(bels::STARTUP, vec!["startup".to_string()]);
                nnode.add_bel(bels::READCLK, vec!["rdclk".to_string()]);
            }
            "CNR.TR" => {
                let nnode = ngrid.name_node(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row - 1].clone()),
                        (col_x[col - 1].clone(), row_y[row - 1].clone()),
                    ],
                );

                if grid.kind == ChipKind::Xc4000A {
                    nnode.add_bel(
                        bels::PULLUP_DEC0_H,
                        vec![
                            name_a(grid, "PULLUP.", ".1", col, row),
                            name_b(grid, "PULLUP_", ".1", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_H,
                        vec![
                            name_a(grid, "PULLUP.", ".0", col, row),
                            name_b(grid, "PULLUP_", ".0", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC0_V,
                        vec![
                            name_a(grid, "PULLUP.", ".2", col, row),
                            name_b(grid, "PULLUP_", ".2", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_V,
                        vec![
                            name_a(grid, "PULLUP.", ".3", col, row),
                            name_b(grid, "PULLUP_", ".3", col, row),
                        ],
                    );
                } else {
                    nnode.add_bel(
                        bels::PULLUP_DEC0_H,
                        vec![
                            name_a(grid, "PULLUP.", ".1", col, row),
                            name_b(grid, "PULLUP_", ".1", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_H,
                        vec![
                            name_a(grid, "PULLUP.", ".2", col, row),
                            name_b(grid, "PULLUP_", ".2", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC2_H,
                        vec![
                            name_a(grid, "PULLUP.", ".3", col, row),
                            name_b(grid, "PULLUP_", ".3", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC3_H,
                        vec![
                            name_a(grid, "PULLUP.", ".4", col, row),
                            name_b(grid, "PULLUP_", ".4", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC0_V,
                        vec![
                            name_a(grid, "PULLUP.", ".5", col, row),
                            name_b(grid, "PULLUP_", ".5", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC1_V,
                        vec![
                            name_a(grid, "PULLUP.", ".6", col, row),
                            name_b(grid, "PULLUP_", ".6", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC2_V,
                        vec![
                            name_a(grid, "PULLUP.", ".7", col, row),
                            name_b(grid, "PULLUP_", ".7", col, row),
                        ],
                    );
                    nnode.add_bel(
                        bels::PULLUP_DEC3_V,
                        vec![
                            name_a(grid, "PULLUP.", ".8", col, row),
                            name_b(grid, "PULLUP_", ".8", col, row),
                        ],
                    );
                }
                nnode.add_bel(bels::BUFGLS_H, vec!["bufgp_tr".to_string()]);
                nnode.add_bel(bels::BUFGLS_V, vec!["bufgs_tr".to_string()]);
                nnode.add_bel(bels::COUT, vec!["co_tr".to_string()]);
                nnode.add_bel(bels::UPDATE, vec!["update".to_string()]);
                nnode.add_bel(bels::OSC, vec!["osc".to_string()]);
                nnode.add_bel(bels::TDO, vec!["tdo".to_string()]);
            }

            "LLV.IO.L" | "LLV.IO.R" | "LLV.CLB" => {
                let nnode = ngrid.name_node(tcrd, kind, [(col_x[col].clone(), clk_y.clone())]);
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
                    nnode.add_bel(bels::CLKH, vec![format!("SRC0.{r}{c}.1")]);
                }
            }
            "LLH.IO.B" | "LLH.IO.T" | "LLH.CLB" | "LLH.CLB.B" => {
                ngrid.name_node(tcrd, kind, [(clk_x.clone(), row_y[row].clone())]);
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
