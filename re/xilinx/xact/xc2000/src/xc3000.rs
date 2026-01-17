use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{chip::Chip, expanded::ExpandedDevice, xc3000 as defs};

use crate::ExpandedNamedDevice;

fn name_a(
    grid: &Chip,
    prefix: &str,
    suffix: &str,
    col: ColId,
    row: RowId,
    dx: i32,
    dy: i32,
) -> String {
    let cidx = col.to_idx();
    let ridx = grid.rows - row.to_idx() - 1;
    let cidx = u32::try_from(cidx).unwrap().checked_add_signed(dx).unwrap();
    let ridx = u32::try_from(ridx).unwrap().checked_add_signed(dy).unwrap();
    let r = char::from_u32(u32::from('A') + ridx).unwrap();
    let c = char::from_u32(u32::from('A') + cidx).unwrap();
    format!("{prefix}{r}{c}{suffix}")
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let chip = edev.chip;
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);

    let die = DieId::from_idx(0);
    let mut col_x = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut x = 0;
    for col in edev.cols(die) {
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
        let kind = edev.db.tile_classes.key(tile.class);
        let col = tcrd.col;
        let row = tcrd.row;
        let mut naming = kind.to_string();
        if col == chip.col_w() + 1 {
            naming += "_W1";
        }
        if row == chip.row_s() + 1 {
            naming += "_S1";
        }
        if tcrd.slot == defs::tslots::MAIN {
            let mut raw_cells = vec![(col_x[col].clone(), row_y[row].clone())];
            match tile.class {
                defs::tcls::CLB_SW2_S => {
                    raw_cells.push((col_x[col].clone(), row_y[chip.row_n()].clone()));
                    raw_cells.push((col_x[chip.col_e()].clone(), row_y[row].clone()));
                }
                defs::tcls::CLB_SE0_S => {
                    raw_cells.push((col_x[chip.col_w()].clone(), row_y[row].clone()));
                }
                defs::tcls::CLB_NW0_S => {
                    raw_cells.push((col_x[col].clone(), row_y[chip.row_s()].clone()));
                }
                _ => (),
            }
            let ntile = ngrid.name_tile(tcrd, &naming, raw_cells);

            if col != chip.col_w() {
                ntile
                    .coords
                    .push((col_x[col - 1].clone(), row_y[row].clone()));
            }
            if col != chip.col_e() {
                ntile
                    .coords
                    .push((col_x[col + 1].clone(), row_y[row].clone()));
            }
            if row != chip.row_s() {
                ntile
                    .coords
                    .push((col_x[col].clone(), row_y[row - 1].clone()));
            }
            if row != chip.row_n() {
                ntile
                    .coords
                    .push((col_x[col].clone(), row_y[row + 1].clone()));
            }

            ntile.add_bel(
                defs::bslots::CLB,
                vec![name_a(chip, "", "", col, row, 0, 0)],
            );

            if tcrd.row == chip.row_s() {
                let p0 = 1
                    + chip.columns * 2
                    + chip.rows * 2
                    + (chip.col_e().to_idx() - col.to_idx()) * 2;
                let p1 = p0 + 1;
                ntile.add_bel(defs::bslots::IO_S[0], vec![format!("PAD{p1}")]);
                ntile.add_bel(defs::bslots::IO_S[1], vec![format!("PAD{p0}")]);
                if tcrd.col == chip.col_w() {
                    let p2 = p0 + 2;
                    let p3 = p0 + 3;
                    ntile.add_bel(defs::bslots::IO_W[0], vec![format!("PAD{p3}")]);
                    ntile.add_bel(defs::bslots::IO_W[1], vec![format!("PAD{p2}")]);
                } else if tcrd.col == chip.col_e() {
                    let p2 = p0 - 2;
                    let p3 = p0 - 1;
                    ntile.add_bel(defs::bslots::IO_E[0], vec![format!("PAD{p2}")]);
                    ntile.add_bel(defs::bslots::IO_E[1], vec![format!("PAD{p3}")]);
                }
            } else if tcrd.row == chip.row_n() {
                let p0 = 1 + col.to_idx() * 2;
                let p1 = p0 + 1;
                ntile.add_bel(defs::bslots::IO_N[0], vec![format!("PAD{p0}")]);
                ntile.add_bel(defs::bslots::IO_N[1], vec![format!("PAD{p1}")]);
                if tcrd.col == chip.col_w() {
                    let p0 = chip.columns * 4 + chip.rows * 4 - 1;
                    let p1 = p0 + 1;
                    ntile.add_bel(defs::bslots::IO_W[0], vec![format!("PAD{p1}")]);
                    ntile.add_bel(defs::bslots::IO_W[1], vec![format!("PAD{p0}")]);
                } else if tcrd.col == chip.col_e() {
                    let p2 = p0 + 2;
                    let p3 = p0 + 3;
                    ntile.add_bel(defs::bslots::IO_E[0], vec![format!("PAD{p2}")]);
                    ntile.add_bel(defs::bslots::IO_E[1], vec![format!("PAD{p3}")]);
                }
            } else if tcrd.col == chip.col_w() {
                let p0 = 1 + chip.columns * 4 + chip.rows * 2 + row.to_idx() * 2;
                let p1 = p0 + 1;
                ntile.add_bel(defs::bslots::IO_W[0], vec![format!("PAD{p1}")]);
                ntile.add_bel(defs::bslots::IO_W[1], vec![format!("PAD{p0}")]);
            } else if tcrd.col == chip.col_e() {
                let p0 = 1 + chip.columns * 2 + (chip.row_n().to_idx() - row.to_idx()) * 2;
                let p1 = p0 + 1;
                ntile.add_bel(defs::bslots::IO_E[0], vec![format!("PAD{p0}")]);
                ntile.add_bel(defs::bslots::IO_E[1], vec![format!("PAD{p1}")]);
            }

            let suf2 = if row == chip.row_n() { ".1" } else { ".2" };
            ntile.add_bel(
                defs::bslots::TBUF[0],
                vec![name_a(chip, "TBUF.", ".1", col, row, 0, 1)],
            );
            ntile.add_bel(
                defs::bslots::TBUF[1],
                vec![name_a(chip, "TBUF.", suf2, col, row, 0, 0)],
            );
            if col == chip.col_e() {
                ntile.add_bel(
                    defs::bslots::TBUF_E[0],
                    vec![name_a(chip, "TBUF.", ".1", col, row, 1, 1)],
                );
                ntile.add_bel(
                    defs::bslots::TBUF_E[1],
                    vec![name_a(chip, "TBUF.", suf2, col, row, 1, 0)],
                );
                ntile.add_bel(
                    defs::bslots::PULLUP_TBUF[0],
                    vec![name_a(chip, "PU.", ".1", col, row, 1, 1)],
                );
                ntile.add_bel(
                    defs::bslots::PULLUP_TBUF[1],
                    vec![name_a(chip, "PU.", suf2, col, row, 1, 0)],
                );
            } else if col == chip.col_w() {
                ntile.add_bel(
                    defs::bslots::PULLUP_TBUF[0],
                    vec![name_a(chip, "PU.", ".1", col, row, 0, 1)],
                );
                ntile.add_bel(
                    defs::bslots::PULLUP_TBUF[1],
                    vec![name_a(chip, "PU.", suf2, col, row, 0, 0)],
                );
            }

            if tcrd.col == chip.col_w() && tcrd.row == chip.row_n() {
                ntile.add_bel(defs::bslots::CLKIOB, vec!["TCLKIN".into()]);
                ntile.add_bel(defs::bslots::BUFG, vec!["GCLK".into()]);
            }
            if tcrd.col == chip.col_e() && tcrd.row == chip.row_s() {
                ntile.add_bel(defs::bslots::CLKIOB, vec!["BCLKIN".into()]);
                ntile.add_bel(defs::bslots::BUFG, vec!["ACLK".into()]);
                ntile.add_bel(defs::bslots::OSC, vec!["OSC".into()]);
            }
        } else if tcrd.slot == defs::tslots::LLH {
            ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
        } else if tcrd.slot == defs::tslots::LLV {
            ngrid.name_tile(tcrd, kind, [(col_x[col].clone(), row_y[row - 1].clone())]);
        } else if tcrd.slot == defs::tslots::MISC_E {
            ngrid.name_tile(tcrd, kind, []);
        } else {
            panic!("ummmm {kind}?");
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid,
        chip,
        col_x,
        row_y,
        clk_x: None,
        clk_y: None,
    }
}
