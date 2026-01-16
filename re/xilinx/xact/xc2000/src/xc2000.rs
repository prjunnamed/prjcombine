use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::DieId;
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{expanded::ExpandedDevice, xc2000 as defs, xc2000::xc2000::tcls};

use crate::ExpandedNamedDevice;

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
        if tcrd.slot != defs::tslots::MAIN {
            ngrid.name_tile(tcrd, kind, []);
            continue;
        }
        let mut naming = &kind[..];
        if tcrd.col == chip.col_w() && tcrd.row == chip.row_s() + 1 {
            naming = "CLB_S1W";
        }
        if tcrd.col == chip.col_e() && tcrd.row == chip.row_s() + 1 {
            naming = "CLB_S1E";
        }
        let ntile = ngrid.name_tile(
            tcrd,
            naming,
            [(col_x[tcrd.col].clone(), row_y[tcrd.row].clone())],
        );
        if (tcrd.row == chip.row_s() || tcrd.row == chip.row_n()) && tcrd.col != chip.col_e() {
            ntile
                .coords
                .push((col_x[tcrd.col + 1].clone(), row_y[tcrd.row].clone()));
        }

        if (tcrd.col == chip.col_w() || tcrd.col == chip.col_e()) && tcrd.row != chip.row_s() {
            ntile
                .coords
                .push((col_x[tcrd.col].clone(), row_y[tcrd.row - 1].clone()));
        }
        if tcrd.row != chip.row_n() {
            ntile
                .coords
                .push((col_x[tcrd.col].clone(), row_y[tcrd.row + 1].clone()));
        }
        let cidx = tcrd.col.to_idx();
        let ridx = chip.rows - tcrd.row.to_idx() - 1;
        let cidx = u32::try_from(cidx).unwrap();
        let ridx = u32::try_from(ridx).unwrap();
        let r = char::from_u32(u32::from('A') + ridx).unwrap();
        let c = char::from_u32(u32::from('A') + cidx).unwrap();
        ntile.add_bel(defs::bslots::CLB, vec![format!("{r}{c}")]);
        if tcrd.row == chip.row_s() {
            let p0 = 1 + chip.columns * 2 + chip.rows * 2 - 3
                + (chip.col_e().to_idx() - tcrd.col.to_idx()) * 2;
            let p1 = p0 + 1;
            ntile.add_bel(defs::bslots::IO_S[0], vec![format!("PAD{p1}")]);
            ntile.add_bel(defs::bslots::IO_S[1], vec![format!("PAD{p0}")]);
            if tcrd.col == chip.col_w() {
                let p2 = p0 + 2;
                ntile.add_bel(defs::bslots::IO_W[0], vec![format!("PAD{p2}")]);
            } else if tcrd.col == chip.col_e() {
                let p2 = p0 - 1;
                ntile.add_bel(defs::bslots::IO_E[0], vec![format!("PAD{p2}")]);
                let cidx = tcrd.col.to_idx() + 1;
                let ridx = chip.rows - tcrd.row.to_idx();
                let cidx = u32::try_from(cidx).unwrap();
                let ridx = u32::try_from(ridx).unwrap();
                let r = char::from_u32(u32::from('A') + ridx).unwrap();
                let c = char::from_u32(u32::from('A') + cidx).unwrap();
                ntile.add_bel(defs::bslots::BUFG, vec![format!("CLK.{r}{c}")]);
                ntile.add_bel(defs::bslots::OSC, vec![format!("OSC.{r}{c}")]);
            }
        } else if tcrd.row == chip.row_n() {
            let p0 = 1 + tcrd.col.to_idx() * 2;
            let p1 = p0 + 1;
            ntile.add_bel(defs::bslots::IO_N[0], vec![format!("PAD{p0}")]);
            ntile.add_bel(defs::bslots::IO_N[1], vec![format!("PAD{p1}")]);
            if tcrd.col == chip.col_w() {
                let p = chip.columns * 4 + chip.rows * 4 - 6;
                ntile.add_bel(defs::bslots::IO_W[1], vec![format!("PAD{p}")]);
                ntile.add_bel(defs::bslots::BUFG, vec!["CLK.AA".into()]);
            } else if tcrd.col == chip.col_e() {
                let p2 = p0 + 2;
                ntile.add_bel(defs::bslots::IO_E[1], vec![format!("PAD{p2}")]);
            }
        } else if tile.class == tcls::CLB_MW {
            let p = 1 + chip.columns * 4 + chip.rows * 2 - 3 + tcrd.row.to_idx() * 2 - 1;
            ntile.add_bel(defs::bslots::IO_W[1], vec![format!("PAD{p}")]);
        } else if tile.class == tcls::CLB_ME {
            let p = 1 + chip.columns * 2 + (chip.row_n().to_idx() - tcrd.row.to_idx()) * 2 - 1;
            ntile.add_bel(defs::bslots::IO_E[1], vec![format!("PAD{p}")]);
        } else if tile.class == tcls::CLB_W {
            let p0 = 1 + chip.columns * 4 + chip.rows * 2 - 3 + tcrd.row.to_idx() * 2
                - 1
                - usize::from(tcrd.row >= chip.row_mid());
            let p1 = p0 + 1;
            ntile.add_bel(defs::bslots::IO_W[0], vec![format!("PAD{p1}")]);
            ntile.add_bel(defs::bslots::IO_W[1], vec![format!("PAD{p0}")]);
        } else if tile.class == tcls::CLB_E {
            let p0 = 1 + chip.columns * 2 + (chip.row_n().to_idx() - tcrd.row.to_idx()) * 2
                - 1
                - usize::from(tcrd.row < chip.row_mid());
            let p1 = p0 + 1;
            ntile.add_bel(defs::bslots::IO_E[0], vec![format!("PAD{p0}")]);
            ntile.add_bel(defs::bslots::IO_E[1], vec![format!("PAD{p1}")]);
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
