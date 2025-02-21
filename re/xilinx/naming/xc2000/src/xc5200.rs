use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::expanded::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::ExpandedNamedDevice;

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);
    ngrid.tie_kind = Some("GND".to_string());
    ngrid.tie_pin_gnd = Some("O".to_string());

    for die in egrid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    let c = col.to_idx();
                    let r = edev.grid.row_tio() - row;
                    match &kind[..] {
                        "CNR.BL" => {
                            let nnode = ngrid.name_node(nloc, "CNR.BL", ["BL".into()]);
                            nnode.add_bel(0, "BUFG_BL".to_string());
                            nnode.add_bel(2, "RDBK".to_string());
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "CNR.TL" => {
                            let nnode = ngrid.name_node(nloc, "CNR.TL", ["TL".into()]);
                            nnode.add_bel(0, "BUFG_TL".to_string());
                            nnode.add_bel(2, "BSCAN".to_string());
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "CNR.BR" => {
                            let nnode = ngrid.name_node(nloc, "CNR.BR", ["BR".into()]);
                            nnode.add_bel(0, "BUFG_BR".to_string());
                            nnode.add_bel(2, "STARTUP".to_string());
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "CNR.TR" => {
                            let nnode = ngrid.name_node(nloc, "CNR.TR", ["TR".into()]);
                            nnode.add_bel(0, "BUFG_TR".to_string());
                            nnode.add_bel(2, "OSC".to_string());
                            nnode.add_bel(3, "BYPOSC".to_string());
                            nnode.add_bel(4, "BSUPD".to_string());
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "IO.L" => {
                            let nnode = if row == edev.grid.row_tio() - 1 {
                                ngrid.name_node(nloc, "IO.L.CLK", ["LCLK".into()])
                            } else {
                                ngrid.name_node(nloc, "IO.L", [format!("LR{r}")])
                            };
                            let p = (edev.grid.columns - 2) * 8
                                + (edev.grid.rows - 2) * 4
                                + (row.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(0, format!("PAD{p}"));
                            nnode.add_bel(1, format!("PAD{}", p + 1));
                            nnode.add_bel(2, format!("PAD{}", p + 2));
                            nnode.add_bel(3, format!("PAD{}", p + 3));
                            nnode.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "IO.R" => {
                            let nnode = if row == edev.grid.row_bio() + 1 {
                                ngrid.name_node(nloc, "IO.R.CLK", ["RCLK".into()])
                            } else {
                                ngrid.name_node(nloc, "IO.R", [format!("RR{r}")])
                            };
                            let p = (edev.grid.columns - 2) * 4
                                + (edev.grid.row_tio().to_idx() - row.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(0, format!("PAD{}", p + 3));
                            nnode.add_bel(1, format!("PAD{}", p + 2));
                            nnode.add_bel(2, format!("PAD{}", p + 1));
                            nnode.add_bel(3, format!("PAD{p}"));
                            nnode.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "IO.B" => {
                            let nnode = if col == edev.grid.col_lio() + 1 {
                                ngrid.name_node(nloc, "IO.B.CLK", ["BCLK".into()])
                            } else {
                                ngrid.name_node(nloc, "IO.B", [format!("BC{c}")])
                            };
                            let p = (edev.grid.columns - 2) * 4
                                + (edev.grid.rows - 2) * 4
                                + (edev.grid.col_rio().to_idx() - col.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(0, format!("PAD{p}"));
                            nnode.add_bel(1, format!("PAD{}", p + 1));
                            nnode.add_bel(2, format!("PAD{}", p + 2));
                            nnode.add_bel(3, format!("PAD{}", p + 3));
                            nnode.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "IO.T" => {
                            let nnode = if col == edev.grid.col_rio() - 2 {
                                ngrid.name_node(nloc, "IO.T.CLK", ["TCLK".into()])
                            } else {
                                ngrid.name_node(nloc, "IO.T", [format!("TC{c}")])
                            };
                            let p = (col.to_idx() - 1) * 4 + 1;
                            nnode.add_bel(0, format!("PAD{}", p + 3));
                            nnode.add_bel(1, format!("PAD{}", p + 2));
                            nnode.add_bel(2, format!("PAD{}", p + 1));
                            nnode.add_bel(3, format!("PAD{p}"));
                            nnode.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "CLB" => {
                            let nnode = ngrid.name_node(nloc, "CLB", [format!("R{r}C{c}")]);
                            nnode.add_bel(0, format!("CLB_R{r}C{c}.LC0"));
                            nnode.add_bel(1, format!("CLB_R{r}C{c}.LC1"));
                            nnode.add_bel(2, format!("CLB_R{r}C{c}.LC2"));
                            nnode.add_bel(3, format!("CLB_R{r}C{c}.LC3"));
                            nnode.add_bel(4, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(5, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(6, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(7, format!("TBUF_R{r}C{c}.3"));
                            nnode.add_bel(8, format!("VCC_GND_R{r}C{c}"));
                        }
                        "CLKL" => {
                            ngrid.name_node(nloc, "CLKL", ["LM".into()]);
                        }
                        "CLKR" => {
                            ngrid.name_node(nloc, "CLKR", ["RM".into()]);
                        }
                        "CLKH" => {
                            ngrid.name_node(nloc, "CLKH", [format!("HMC{c}")]);
                        }
                        "CLKB" => {
                            ngrid.name_node(nloc, "CLKB", ["BM".into()]);
                        }
                        "CLKT" => {
                            ngrid.name_node(nloc, "CLKT", ["TM".into()]);
                        }
                        "CLKV" => {
                            ngrid.name_node(nloc, "CLKV", [format!("VMR{r}")]);
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid,
        grid: edev.grid,
    }
}
