use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
    xc4000::{bslots, xc4000::tcls},
};

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
    let chip = edev.chip;
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);
    ngrid.tie_pin_gnd = Some("O".to_string());

    let die = DieId::from_idx(0);
    let mut col_x = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut clk_x = 0..0;
    let mut clk_y = 0..0;
    let mut x = 0;
    for col in edev.cols(die) {
        if col == chip.col_mid() {
            let ox = x;
            x += ndb.tile_widths["CLK"];
            clk_x = ox..x;
        }
        let ox = x;
        x += if col == chip.col_w() {
            ndb.tile_widths["L"]
        } else if col == chip.col_e() {
            ndb.tile_widths["R"]
        } else {
            ndb.tile_widths["C"]
        };
        col_x.push(ox..x);
    }
    let mut y = 0;
    for row in edev.rows(die) {
        if row == chip.row_mid() {
            let oy = y;
            y += ndb.tile_heights["CLK"];
            clk_y = oy..y;
        }
        let oy = y;
        y += if row == chip.row_s() {
            ndb.tile_heights["B"]
        } else if row == chip.row_n() {
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
        match tile.class {
            tcls::CLB
            | tcls::CLB_W
            | tcls::CLB_E
            | tcls::CLB_S
            | tcls::CLB_SW
            | tcls::CLB_SE
            | tcls::CLB_N
            | tcls::CLB_NW
            | tcls::CLB_NE => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row - 1].clone()),
                        (col_x[col].clone(), row_y[row + 1].clone()),
                    ],
                );
                ntile.tie_names = vec![
                    name_a(chip, "TIE.", ".1", col, row),
                    name_b(chip, "TIE_", ".1", col, row),
                ];
                if tile.class == tcls::CLB_SW {
                    ntile
                        .coords
                        .push((col_x[col - 1].clone(), row_y[row - 1].clone()));
                } else if tile.class == tcls::CLB_S
                    || tile.class == tcls::CLB_SE
                    || tile.class == tcls::CLB_N
                    || tile.class == tcls::CLB_NE
                {
                    ntile
                        .coords
                        .push((col_x[col - 1].clone(), row_y[row].clone()));
                } else if tile.class == tcls::CLB_NW {
                    ntile
                        .coords
                        .push((col_x[col - 1].clone(), row_y[row + 1].clone()));
                }

                let mut clb_names = vec![
                    name_a(chip, "", "", col, row),
                    name_b(chip, "CLB_", "", col, row),
                ];
                if col == chip.col_w() + 1 && row == chip.row_s() + 1 {
                    clb_names.push("ci_bl".to_string());
                }
                if col == chip.col_w() + 1 && row == chip.row_n() - 1 {
                    clb_names.push("ci_tl".to_string());
                }
                if col == chip.col_e() - 1 && row == chip.row_s() + 1 {
                    clb_names.push("co_br".to_string());
                }
                if col == chip.col_e() - 1 && row == chip.row_n() - 1 {
                    clb_names.push("co_tr".to_string());
                }
                ntile.add_bel(bslots::CLB, clb_names);
                ntile.add_bel(
                    bslots::TBUF[0],
                    vec![
                        name_a(chip, "TBUF.", ".2", col, row),
                        name_b(chip, "TBUF_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::TBUF[1],
                    vec![
                        name_a(chip, "TBUF.", ".1", col, row),
                        name_b(chip, "TBUF_", ".1", col, row),
                    ],
                );
            }
            tcls::IO_S0 | tcls::IO_S1 | tcls::IO_S0_E | tcls::IO_S1_W => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row + 1].clone()),
                        (col_x[col + 1].clone(), row_y[row].clone()),
                    ],
                );
                ntile.tie_names = vec![
                    name_a(chip, "TIE.", ".1", col, row),
                    name_b(chip, "TIE_", ".1", col, row),
                ];
                let (slot0, slot1) = if chip.kind == ChipKind::Xc4000H {
                    let p = (chip.columns - 2) * 4
                        + (chip.rows - 2) * 4
                        + (chip.col_e().to_idx() - col.to_idx() - 1) * 4
                        + 1;
                    ntile.add_bel(bslots::HIO[0], vec![format!("PAD{}", p + 3)]);
                    ntile.add_bel(bslots::HIO[1], vec![format!("PAD{}", p + 2)]);
                    ntile.add_bel(bslots::HIO[2], vec![format!("PAD{}", p + 1)]);
                    ntile.add_bel(bslots::HIO[3], vec![format!("PAD{p}")]);
                    (bslots::HIO[0], bslots::HIO[3])
                } else {
                    let p = (chip.columns - 2) * 2
                        + (chip.rows - 2) * 2
                        + (chip.col_e().to_idx() - col.to_idx() - 1) * 2
                        + 1;
                    ntile.add_bel(bslots::IO[0], vec![format!("PAD{}", p + 1)]);
                    ntile.add_bel(bslots::IO[1], vec![format!("PAD{p}")]);
                    (bslots::IO[0], bslots::IO[1])
                };
                if tile.class == tcls::IO_S0_E {
                    ntile.bels[slot1].push("i_bufgs_br".into());
                }
                if tile.class == tcls::IO_S1_W {
                    ntile.bels[slot0].push("i_bufgp_bl".into());
                }
                ntile.add_bel(
                    bslots::DEC[0],
                    vec![
                        name_a(chip, "DEC.", ".1", col, row),
                        name_b(chip, "DEC_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[1],
                    vec![
                        name_a(chip, "DEC.", ".2", col, row),
                        name_b(chip, "DEC_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[2],
                    vec![
                        name_a(chip, "DEC.", ".3", col, row),
                        name_b(chip, "DEC_", ".3", col, row),
                    ],
                );
            }
            tcls::IO_N0 | tcls::IO_N1 | tcls::IO_N0_E | tcls::IO_N1_W => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col + 1].clone(), row_y[row].clone()),
                    ],
                );
                let (slot0, slot1) = if chip.kind == ChipKind::Xc4000H {
                    let p = (col.to_idx() - 1) * 4 + 1;
                    ntile.add_bel(bslots::HIO[0], vec![format!("PAD{p}")]);
                    ntile.add_bel(bslots::HIO[1], vec![format!("PAD{}", p + 1)]);
                    ntile.add_bel(bslots::HIO[2], vec![format!("PAD{}", p + 2)]);
                    ntile.add_bel(bslots::HIO[3], vec![format!("PAD{}", p + 3)]);
                    (bslots::HIO[0], bslots::HIO[2])
                } else {
                    let p = (col.to_idx() - 1) * 2 + 1;
                    ntile.add_bel(bslots::IO[0], vec![format!("PAD{p}")]);
                    ntile.add_bel(bslots::IO[1], vec![format!("PAD{}", p + 1)]);
                    (bslots::IO[0], bslots::IO[0])
                };
                if tile.class == tcls::IO_N0_E {
                    ntile.bels[slot1].push("i_bufgp_tr".into());
                }
                if tile.class == tcls::IO_N1_W {
                    ntile.bels[slot0].push("i_bufgs_tl".into());
                }
                ntile.add_bel(
                    bslots::DEC[0],
                    vec![
                        name_a(chip, "DEC.", ".1", col, row),
                        name_b(chip, "DEC_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[1],
                    vec![
                        name_a(chip, "DEC.", ".2", col, row),
                        name_b(chip, "DEC_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[2],
                    vec![
                        name_a(chip, "DEC.", ".3", col, row),
                        name_b(chip, "DEC_", ".3", col, row),
                    ],
                );
            }
            tcls::IO_W0
            | tcls::IO_W1
            | tcls::IO_W0_N
            | tcls::IO_W1_S
            | tcls::IO_W0_F1
            | tcls::IO_W1_F1
            | tcls::IO_W0_F0
            | tcls::IO_W1_F0 => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row - 1].clone()),
                    ],
                );
                let (slot0, slot1) = if chip.kind == ChipKind::Xc4000H {
                    let p =
                        (chip.columns - 2) * 8 + (chip.rows - 2) * 4 + (row.to_idx() - 1) * 4 + 1;
                    ntile.add_bel(bslots::HIO[0], vec![format!("PAD{}", p + 3)]);
                    ntile.add_bel(bslots::HIO[1], vec![format!("PAD{}", p + 2)]);
                    ntile.add_bel(bslots::HIO[2], vec![format!("PAD{}", p + 1)]);
                    ntile.add_bel(bslots::HIO[3], vec![format!("PAD{p}")]);
                    (bslots::HIO[0], bslots::HIO[3])
                } else {
                    let p =
                        (chip.columns - 2) * 4 + (chip.rows - 2) * 2 + (row.to_idx() - 1) * 2 + 1;
                    ntile.add_bel(bslots::IO[0], vec![format!("PAD{}", p + 1)]);
                    ntile.add_bel(bslots::IO[1], vec![format!("PAD{p}")]);
                    (bslots::IO[0], bslots::IO[1])
                };
                if tile.class == tcls::IO_W0_N {
                    ntile.bels[slot0].push("i_bufgp_tl".into());
                }
                if tile.class == tcls::IO_W1_S {
                    ntile.bels[slot1].push("i_bufgs_bl".into());
                }
                ntile.add_bel(
                    bslots::TBUF[0],
                    vec![
                        name_a(chip, "TBUF.", ".2", col, row),
                        name_b(chip, "TBUF_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::TBUF[1],
                    vec![
                        name_a(chip, "TBUF.", ".1", col, row),
                        name_b(chip, "TBUF_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::PULLUP_TBUF[0],
                    vec![
                        name_a(chip, "PULLUP.", ".2", col, row),
                        name_b(chip, "PULLUP_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::PULLUP_TBUF[1],
                    vec![
                        name_a(chip, "PULLUP.", ".1", col, row),
                        name_b(chip, "PULLUP_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[0],
                    vec![
                        name_a(chip, "DEC.", ".1", col, row),
                        name_b(chip, "DEC_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[1],
                    vec![
                        name_a(chip, "DEC.", ".2", col, row),
                        name_b(chip, "DEC_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[2],
                    vec![
                        name_a(chip, "DEC.", ".3", col, row),
                        name_b(chip, "DEC_", ".3", col, row),
                    ],
                );
            }
            tcls::IO_E0
            | tcls::IO_E1
            | tcls::IO_E0_N
            | tcls::IO_E1_S
            | tcls::IO_E0_F1
            | tcls::IO_E1_F1
            | tcls::IO_E0_F0
            | tcls::IO_E1_F0 => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row - 1].clone()),
                    ],
                );
                if chip.kind != ChipKind::Xc4000A {
                    ntile.tie_names = vec![
                        name_a(chip, "TIE.", ".1", col, row),
                        name_b(chip, "TIE_", ".1", col, row),
                    ];
                }

                let (slot0, slot1) = if chip.kind == ChipKind::Xc4000H {
                    let p =
                        (chip.columns - 2) * 4 + (chip.row_n().to_idx() - row.to_idx() - 1) * 4 + 1;
                    ntile.add_bel(bslots::HIO[0], vec![format!("PAD{p}")]);
                    ntile.add_bel(bslots::HIO[1], vec![format!("PAD{}", p + 1)]);
                    ntile.add_bel(bslots::HIO[2], vec![format!("PAD{}", p + 2)]);
                    ntile.add_bel(bslots::HIO[3], vec![format!("PAD{}", p + 3)]);
                    (bslots::HIO[0], bslots::HIO[2])
                } else {
                    let p =
                        (chip.columns - 2) * 2 + (chip.row_n().to_idx() - row.to_idx() - 1) * 2 + 1;
                    ntile.add_bel(bslots::IO[0], vec![format!("PAD{p}")]);
                    ntile.add_bel(bslots::IO[1], vec![format!("PAD{}", p + 1)]);
                    (bslots::IO[0], bslots::IO[0])
                };
                if tile.class == tcls::IO_E0_N {
                    ntile.bels[slot0].push("i_bufgs_tr".into());
                }
                if tile.class == tcls::IO_E1_S {
                    ntile.bels[slot1].push("i_bufgp_br".into());
                }
                ntile.add_bel(
                    bslots::TBUF[0],
                    vec![
                        name_a(chip, "TBUF.", ".2", col, row),
                        name_b(chip, "TBUF_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::TBUF[1],
                    vec![
                        name_a(chip, "TBUF.", ".1", col, row),
                        name_b(chip, "TBUF_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::PULLUP_TBUF[0],
                    vec![
                        name_a(chip, "PULLUP.", ".2", col, row),
                        name_b(chip, "PULLUP_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::PULLUP_TBUF[1],
                    vec![
                        name_a(chip, "PULLUP.", ".1", col, row),
                        name_b(chip, "PULLUP_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[0],
                    vec![
                        name_a(chip, "DEC.", ".1", col, row),
                        name_b(chip, "DEC_", ".1", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[1],
                    vec![
                        name_a(chip, "DEC.", ".2", col, row),
                        name_b(chip, "DEC_", ".2", col, row),
                    ],
                );
                ntile.add_bel(
                    bslots::DEC[2],
                    vec![
                        name_a(chip, "DEC.", ".3", col, row),
                        name_b(chip, "DEC_", ".3", col, row),
                    ],
                );
            }

            tcls::CNR_SW => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col + 1].clone(), row_y[row].clone()),
                    ],
                );

                if chip.kind == ChipKind::Xc4000A {
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[0],
                        vec![
                            name_a(chip, "PULLUP.", ".1", col, row),
                            name_b(chip, "PULLUP_", ".1", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[1],
                        vec![
                            name_a(chip, "PULLUP.", ".0", col, row),
                            name_b(chip, "PULLUP_", ".0", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[0],
                        vec![
                            name_a(chip, "PULLUP.", ".2", col, row),
                            name_b(chip, "PULLUP_", ".2", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[1],
                        vec![
                            name_a(chip, "PULLUP.", ".3", col, row),
                            name_b(chip, "PULLUP_", ".3", col, row),
                        ],
                    );
                } else {
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[0],
                        vec![
                            name_a(chip, "PULLUP.", ".8", col, row),
                            name_b(chip, "PULLUP_", ".8", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[1],
                        vec![
                            name_a(chip, "PULLUP.", ".7", col, row),
                            name_b(chip, "PULLUP_", ".7", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[2],
                        vec![
                            name_a(chip, "PULLUP.", ".6", col, row),
                            name_b(chip, "PULLUP_", ".6", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[3],
                        vec![
                            name_a(chip, "PULLUP.", ".5", col, row),
                            name_b(chip, "PULLUP_", ".5", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[0],
                        vec![
                            name_a(chip, "PULLUP.", ".4", col, row),
                            name_b(chip, "PULLUP_", ".4", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[1],
                        vec![
                            name_a(chip, "PULLUP.", ".3", col, row),
                            name_b(chip, "PULLUP_", ".3", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[2],
                        vec![
                            name_a(chip, "PULLUP.", ".2", col, row),
                            name_b(chip, "PULLUP_", ".2", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[3],
                        vec![
                            name_a(chip, "PULLUP.", ".1", col, row),
                            name_b(chip, "PULLUP_", ".1", col, row),
                        ],
                    );
                }
                ntile.add_bel(bslots::BUFG_H, vec!["bufgp_bl".to_string()]);
                ntile.add_bel(bslots::BUFG_V, vec!["bufgs_bl".to_string()]);
                ntile.add_bel(bslots::MD0, vec!["md0".to_string()]);
                ntile.add_bel(bslots::MD1, vec!["md1".to_string()]);
                ntile.add_bel(bslots::MD2, vec!["md2".to_string()]);
                ntile.add_bel(bslots::RDBK, vec!["rdbk".to_string()]);
            }
            tcls::CNR_NW => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col + 1].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row - 1].clone()),
                    ],
                );

                if chip.kind == ChipKind::Xc4000A {
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[0],
                        vec![
                            name_a(chip, "PULLUP.", ".1", col, row),
                            name_b(chip, "PULLUP_", ".1", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[1],
                        vec![
                            name_a(chip, "PULLUP.", ".0", col, row),
                            name_b(chip, "PULLUP_", ".0", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[0],
                        vec![
                            name_a(chip, "PULLUP.", ".2", col, row),
                            name_b(chip, "PULLUP_", ".2", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[1],
                        vec![
                            name_a(chip, "PULLUP.", ".3", col, row),
                            name_b(chip, "PULLUP_", ".3", col, row),
                        ],
                    );
                } else {
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[0],
                        vec![
                            name_a(chip, "PULLUP.", ".1", col, row),
                            name_b(chip, "PULLUP_", ".1", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[1],
                        vec![
                            name_a(chip, "PULLUP.", ".2", col, row),
                            name_b(chip, "PULLUP_", ".2", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[2],
                        vec![
                            name_a(chip, "PULLUP.", ".3", col, row),
                            name_b(chip, "PULLUP_", ".3", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[3],
                        vec![
                            name_a(chip, "PULLUP.", ".4", col, row),
                            name_b(chip, "PULLUP_", ".4", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[0],
                        vec![
                            name_a(chip, "PULLUP.", ".5", col, row),
                            name_b(chip, "PULLUP_", ".5", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[1],
                        vec![
                            name_a(chip, "PULLUP.", ".6", col, row),
                            name_b(chip, "PULLUP_", ".6", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[2],
                        vec![
                            name_a(chip, "PULLUP.", ".7", col, row),
                            name_b(chip, "PULLUP_", ".7", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[3],
                        vec![
                            name_a(chip, "PULLUP.", ".8", col, row),
                            name_b(chip, "PULLUP_", ".8", col, row),
                        ],
                    );
                }
                ntile.add_bel(bslots::BUFG_H, vec!["bufgs_tl".to_string()]);
                ntile.add_bel(bslots::BUFG_V, vec!["bufgp_tl".to_string()]);
                ntile.add_bel(bslots::BSCAN, vec!["bscan".to_string()]);
            }
            tcls::CNR_SE => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col - 1].clone(), row_y[row + 1].clone()),
                    ],
                );

                if chip.kind != ChipKind::Xc4000A {
                    ntile.tie_names = vec![
                        name_a(chip, "TIE.", ".1", col, row),
                        name_b(chip, "TIE_", ".1", col, row),
                    ];
                }

                if chip.kind == ChipKind::Xc4000A {
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[0],
                        vec![
                            name_a(chip, "PULLUP.", ".1", col, row),
                            name_b(chip, "PULLUP_", ".1", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[1],
                        vec![
                            name_a(chip, "PULLUP.", ".0", col, row),
                            name_b(chip, "PULLUP_", ".0", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[0],
                        vec![
                            name_a(chip, "PULLUP.", ".2", col, row),
                            name_b(chip, "PULLUP_", ".2", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[1],
                        vec![
                            name_a(chip, "PULLUP.", ".3", col, row),
                            name_b(chip, "PULLUP_", ".3", col, row),
                        ],
                    );
                } else {
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[0],
                        vec![
                            name_a(chip, "PULLUP.", ".8", col, row),
                            name_b(chip, "PULLUP_", ".8", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[1],
                        vec![
                            name_a(chip, "PULLUP.", ".7", col, row),
                            name_b(chip, "PULLUP_", ".7", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[2],
                        vec![
                            name_a(chip, "PULLUP.", ".6", col, row),
                            name_b(chip, "PULLUP_", ".6", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[3],
                        vec![
                            name_a(chip, "PULLUP.", ".5", col, row),
                            name_b(chip, "PULLUP_", ".5", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[0],
                        vec![
                            name_a(chip, "PULLUP.", ".4", col, row),
                            name_b(chip, "PULLUP_", ".4", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[1],
                        vec![
                            name_a(chip, "PULLUP.", ".3", col, row),
                            name_b(chip, "PULLUP_", ".3", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[2],
                        vec![
                            name_a(chip, "PULLUP.", ".2", col, row),
                            name_b(chip, "PULLUP_", ".2", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[3],
                        vec![
                            name_a(chip, "PULLUP.", ".1", col, row),
                            name_b(chip, "PULLUP_", ".1", col, row),
                        ],
                    );
                }
                ntile.add_bel(bslots::BUFG_H, vec!["bufgs_br".to_string()]);
                ntile.add_bel(bslots::BUFG_V, vec!["bufgp_br".to_string()]);
                ntile.add_bel(bslots::STARTUP, vec!["startup".to_string()]);
                ntile.add_bel(bslots::READCLK, vec!["rdclk".to_string()]);
            }
            tcls::CNR_NE => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        (col_x[col].clone(), row_y[row].clone()),
                        (col_x[col].clone(), row_y[row - 1].clone()),
                        (col_x[col - 1].clone(), row_y[row - 1].clone()),
                    ],
                );

                if chip.kind == ChipKind::Xc4000A {
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[0],
                        vec![
                            name_a(chip, "PULLUP.", ".1", col, row),
                            name_b(chip, "PULLUP_", ".1", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[1],
                        vec![
                            name_a(chip, "PULLUP.", ".0", col, row),
                            name_b(chip, "PULLUP_", ".0", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[0],
                        vec![
                            name_a(chip, "PULLUP.", ".2", col, row),
                            name_b(chip, "PULLUP_", ".2", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[1],
                        vec![
                            name_a(chip, "PULLUP.", ".3", col, row),
                            name_b(chip, "PULLUP_", ".3", col, row),
                        ],
                    );
                } else {
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[0],
                        vec![
                            name_a(chip, "PULLUP.", ".1", col, row),
                            name_b(chip, "PULLUP_", ".1", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[1],
                        vec![
                            name_a(chip, "PULLUP.", ".2", col, row),
                            name_b(chip, "PULLUP_", ".2", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[2],
                        vec![
                            name_a(chip, "PULLUP.", ".3", col, row),
                            name_b(chip, "PULLUP_", ".3", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_H[3],
                        vec![
                            name_a(chip, "PULLUP.", ".4", col, row),
                            name_b(chip, "PULLUP_", ".4", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[0],
                        vec![
                            name_a(chip, "PULLUP.", ".5", col, row),
                            name_b(chip, "PULLUP_", ".5", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[1],
                        vec![
                            name_a(chip, "PULLUP.", ".6", col, row),
                            name_b(chip, "PULLUP_", ".6", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[2],
                        vec![
                            name_a(chip, "PULLUP.", ".7", col, row),
                            name_b(chip, "PULLUP_", ".7", col, row),
                        ],
                    );
                    ntile.add_bel(
                        bslots::PULLUP_DEC_V[3],
                        vec![
                            name_a(chip, "PULLUP.", ".8", col, row),
                            name_b(chip, "PULLUP_", ".8", col, row),
                        ],
                    );
                }
                ntile.add_bel(bslots::BUFG_H, vec!["bufgp_tr".to_string()]);
                ntile.add_bel(bslots::BUFG_V, vec!["bufgs_tr".to_string()]);
                ntile.add_bel(bslots::UPDATE, vec!["update".to_string()]);
                ntile.add_bel(bslots::OSC, vec!["osc".to_string()]);
                ntile.add_bel(bslots::TDO, vec!["tdo".to_string()]);
            }

            tcls::LLV_IO_W | tcls::LLV_IO_E | tcls::LLV_CLB => {
                let ntile = ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), clk_y.clone())]);
                if chip.kind == ChipKind::Xc4000H {
                    let cidx = if col < chip.col_mid() {
                        col.to_idx()
                    } else {
                        col.to_idx() + 1
                    };
                    let ridx = chip.rows - row.to_idx();
                    let cidx = u32::try_from(cidx).unwrap();
                    let ridx = u32::try_from(ridx).unwrap();
                    let r = char::from_u32(u32::from('A') + ridx).unwrap();
                    let c = char::from_u32(u32::from('A') + cidx).unwrap();
                    ntile.tie_names = vec![format!("SRC0.{r}{c}.1")];
                }
            }
            tcls::LLH_IO_S | tcls::LLH_IO_N | tcls::LLH_CLB | tcls::LLH_CLB_S => {
                ngrid.name_tile(tcrd, kind, [(clk_x.clone(), row_y[row].clone())]);
            }

            _ => panic!("umm {kind}"),
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid,
        chip,
        col_x,
        row_y,
        clk_x: Some(clk_x),
        clk_y: Some(clk_y),
    }
}
