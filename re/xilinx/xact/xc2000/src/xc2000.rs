use prjcombine_interconnect::grid::DieId;
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::expanded::ExpandedDevice;
use unnamed_entity::{EntityId, EntityVec};

use crate::ExpandedNamedDevice;

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let grid = edev.chip;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);

    let mut col_x = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut x = 0;
    for col in egrid.die(DieId::from_idx(0)).cols() {
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
                    if kind.starts_with("BIDI") {
                        ngrid.name_node(nloc, kind, []);
                        continue;
                    }
                    let mut naming = &kind[..];
                    if col == grid.col_lio() && row == grid.row_bio() + 1 {
                        naming = "CLB.B1L";
                    }
                    if col == grid.col_rio() && row == grid.row_bio() + 1 {
                        naming = "CLB.B1R";
                    }
                    let nnode =
                        ngrid.name_node(nloc, naming, [(col_x[col].clone(), row_y[row].clone())]);
                    if (kind.starts_with("CLB.B") || kind.starts_with("CLB.T"))
                        && !kind.ends_with('R')
                    {
                        nnode
                            .coords
                            .push((col_x[col + 1].clone(), row_y[row].clone()));
                    }

                    if kind.ends_with(['L', 'R']) && !kind.starts_with("CLB.B") {
                        nnode
                            .coords
                            .push((col_x[col].clone(), row_y[row - 1].clone()));
                    }
                    if !kind.starts_with("CLB.T") {
                        nnode
                            .coords
                            .push((col_x[col].clone(), row_y[row + 1].clone()));
                    }
                    let cidx = col.to_idx();
                    let ridx = grid.rows - row.to_idx() - 1;
                    let cidx = u32::try_from(cidx).unwrap();
                    let ridx = u32::try_from(ridx).unwrap();
                    let r = char::from_u32(u32::from('A') + ridx).unwrap();
                    let c = char::from_u32(u32::from('A') + cidx).unwrap();
                    nnode.add_bel(0, vec![format!("{r}{c}")]);
                    if kind.starts_with("CLB.B") {
                        let p0 = 1 + grid.columns * 2 + grid.rows * 2 - 3
                            + (grid.col_rio().to_idx() - col.to_idx()) * 2;
                        let p1 = p0 + 1;
                        nnode.add_bel(1, vec![format!("PAD{p1}")]);
                        nnode.add_bel(2, vec![format!("PAD{p0}")]);
                        if kind == "CLB.BL" {
                            let p2 = p0 + 2;
                            nnode.add_bel(3, vec![format!("PAD{p2}")]);
                        } else if kind == "CLB.BR" {
                            let p2 = p0 - 1;
                            nnode.add_bel(3, vec![format!("PAD{p2}")]);
                            let cidx = col.to_idx() + 1;
                            let ridx = grid.rows - row.to_idx();
                            let cidx = u32::try_from(cidx).unwrap();
                            let ridx = u32::try_from(ridx).unwrap();
                            let r = char::from_u32(u32::from('A') + ridx).unwrap();
                            let c = char::from_u32(u32::from('A') + cidx).unwrap();
                            nnode.add_bel(4, vec![format!("CLK.{r}{c}")]);
                            nnode.add_bel(5, vec![format!("OSC.{r}{c}")]);
                        }
                    } else if kind.starts_with("CLB.T") {
                        let p0 = 1 + col.to_idx() * 2;
                        let p1 = p0 + 1;
                        nnode.add_bel(1, vec![format!("PAD{p0}")]);
                        nnode.add_bel(2, vec![format!("PAD{p1}")]);
                        if kind == "CLB.TL" {
                            let p = grid.columns * 4 + grid.rows * 4 - 6;
                            nnode.add_bel(3, vec![format!("PAD{p}")]);
                            nnode.add_bel(4, vec!["CLK.AA".into()]);
                        } else if kind == "CLB.TR" {
                            let p2 = p0 + 2;
                            nnode.add_bel(3, vec![format!("PAD{p2}")]);
                        }
                    } else if kind == "CLB.ML" {
                        let p = 1 + grid.columns * 4 + grid.rows * 2 - 3 + row.to_idx() * 2 - 1;
                        nnode.add_bel(1, vec![format!("PAD{p}")]);
                    } else if kind == "CLB.MR" {
                        let p =
                            1 + grid.columns * 2 + (grid.row_tio().to_idx() - row.to_idx()) * 2 - 1;
                        nnode.add_bel(1, vec![format!("PAD{p}")]);
                    } else if kind == "CLB.L" {
                        let p0 = 1 + grid.columns * 4 + grid.rows * 2 - 3 + row.to_idx() * 2
                            - 1
                            - usize::from(row >= grid.row_mid());
                        let p1 = p0 + 1;
                        nnode.add_bel(1, vec![format!("PAD{p1}")]);
                        nnode.add_bel(2, vec![format!("PAD{p0}")]);
                    } else if kind == "CLB.R" {
                        let p0 =
                            1 + grid.columns * 2 + (grid.row_tio().to_idx() - row.to_idx()) * 2
                                - 1
                                - usize::from(row < grid.row_mid());
                        let p1 = p0 + 1;
                        nnode.add_bel(1, vec![format!("PAD{p0}")]);
                        nnode.add_bel(2, vec![format!("PAD{p1}")]);
                    }
                }
            }
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
