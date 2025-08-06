use prjcombine_interconnect::grid::DieId;
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{bels::xc2000 as bels, expanded::ExpandedDevice};
use unnamed_entity::{EntityId, EntityVec};

use crate::ExpandedNamedDevice;

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let grid = edev.chip;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);

    let die = DieId::from_idx(0);
    let mut col_x = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut x = 0;
    for col in egrid.cols(die) {
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
        let kind = egrid.db.tile_classes.key(tile.class);
        if kind.starts_with("BIDI") {
            ngrid.name_tile(tcrd, kind, []);
            continue;
        }
        let mut naming = &kind[..];
        if tcrd.col == grid.col_w() && tcrd.row == grid.row_s() + 1 {
            naming = "CLB.B1L";
        }
        if tcrd.col == grid.col_e() && tcrd.row == grid.row_s() + 1 {
            naming = "CLB.B1R";
        }
        let ntile = ngrid.name_tile(
            tcrd,
            naming,
            [(col_x[tcrd.col].clone(), row_y[tcrd.row].clone())],
        );
        if (kind.starts_with("CLB.B") || kind.starts_with("CLB.T")) && !kind.ends_with('R') {
            ntile
                .coords
                .push((col_x[tcrd.col + 1].clone(), row_y[tcrd.row].clone()));
        }

        if kind.ends_with(['L', 'R']) && !kind.starts_with("CLB.B") {
            ntile
                .coords
                .push((col_x[tcrd.col].clone(), row_y[tcrd.row - 1].clone()));
        }
        if !kind.starts_with("CLB.T") {
            ntile
                .coords
                .push((col_x[tcrd.col].clone(), row_y[tcrd.row + 1].clone()));
        }
        let cidx = tcrd.col.to_idx();
        let ridx = grid.rows - tcrd.row.to_idx() - 1;
        let cidx = u32::try_from(cidx).unwrap();
        let ridx = u32::try_from(ridx).unwrap();
        let r = char::from_u32(u32::from('A') + ridx).unwrap();
        let c = char::from_u32(u32::from('A') + cidx).unwrap();
        ntile.add_bel(bels::CLB, vec![format!("{r}{c}")]);
        if kind.starts_with("CLB.B") {
            let p0 = 1 + grid.columns * 2 + grid.rows * 2 - 3
                + (grid.col_e().to_idx() - tcrd.col.to_idx()) * 2;
            let p1 = p0 + 1;
            ntile.add_bel(bels::IO_S0, vec![format!("PAD{p1}")]);
            ntile.add_bel(bels::IO_S1, vec![format!("PAD{p0}")]);
            if kind == "CLB.BL" {
                let p2 = p0 + 2;
                ntile.add_bel(bels::IO_W0, vec![format!("PAD{p2}")]);
            } else if kind == "CLB.BR" {
                let p2 = p0 - 1;
                ntile.add_bel(bels::IO_E0, vec![format!("PAD{p2}")]);
                let cidx = tcrd.col.to_idx() + 1;
                let ridx = grid.rows - tcrd.row.to_idx();
                let cidx = u32::try_from(cidx).unwrap();
                let ridx = u32::try_from(ridx).unwrap();
                let r = char::from_u32(u32::from('A') + ridx).unwrap();
                let c = char::from_u32(u32::from('A') + cidx).unwrap();
                ntile.add_bel(bels::BUFG, vec![format!("CLK.{r}{c}")]);
                ntile.add_bel(bels::OSC, vec![format!("OSC.{r}{c}")]);
            }
        } else if kind.starts_with("CLB.T") {
            let p0 = 1 + tcrd.col.to_idx() * 2;
            let p1 = p0 + 1;
            ntile.add_bel(bels::IO_N0, vec![format!("PAD{p0}")]);
            ntile.add_bel(bels::IO_N1, vec![format!("PAD{p1}")]);
            if kind == "CLB.TL" {
                let p = grid.columns * 4 + grid.rows * 4 - 6;
                ntile.add_bel(bels::IO_W1, vec![format!("PAD{p}")]);
                ntile.add_bel(bels::BUFG, vec!["CLK.AA".into()]);
            } else if kind == "CLB.TR" {
                let p2 = p0 + 2;
                ntile.add_bel(bels::IO_E1, vec![format!("PAD{p2}")]);
            }
        } else if kind == "CLB.ML" {
            let p = 1 + grid.columns * 4 + grid.rows * 2 - 3 + tcrd.row.to_idx() * 2 - 1;
            ntile.add_bel(bels::IO_W1, vec![format!("PAD{p}")]);
        } else if kind == "CLB.MR" {
            let p = 1 + grid.columns * 2 + (grid.row_n().to_idx() - tcrd.row.to_idx()) * 2 - 1;
            ntile.add_bel(bels::IO_E1, vec![format!("PAD{p}")]);
        } else if kind == "CLB.L" {
            let p0 = 1 + grid.columns * 4 + grid.rows * 2 - 3 + tcrd.row.to_idx() * 2
                - 1
                - usize::from(tcrd.row >= grid.row_mid());
            let p1 = p0 + 1;
            ntile.add_bel(bels::IO_W0, vec![format!("PAD{p1}")]);
            ntile.add_bel(bels::IO_W1, vec![format!("PAD{p0}")]);
        } else if kind == "CLB.R" {
            let p0 = 1 + grid.columns * 2 + (grid.row_n().to_idx() - tcrd.row.to_idx()) * 2
                - 1
                - usize::from(tcrd.row < grid.row_mid());
            let p1 = p0 + 1;
            ntile.add_bel(bels::IO_E0, vec![format!("PAD{p0}")]);
            ntile.add_bel(bels::IO_E1, vec![format!("PAD{p1}")]);
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid,
        chip: grid,
        col_x,
        row_y,
        clk_x: None,
        clk_y: None,
    }
}
