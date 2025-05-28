use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{bels::xc5200 as bels, expanded::ExpandedDevice};
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
                for (layer, node) in &die[(col, row)].tiles {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.tile_classes.key(node.class);
                    let c = col.to_idx();
                    let r = edev.chip.row_n() - row;
                    match &kind[..] {
                        "CNR.BL" => {
                            let nnode = ngrid.name_tile(nloc, "CNR.BL", ["BL".into()]);
                            nnode.add_bel(bels::BUFG, "BUFG_BL".to_string());
                            nnode.add_bel(bels::RDBK, "RDBK".to_string());
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "CNR.TL" => {
                            let nnode = ngrid.name_tile(nloc, "CNR.TL", ["TL".into()]);
                            nnode.add_bel(bels::BUFG, "BUFG_TL".to_string());
                            nnode.add_bel(bels::BSCAN, "BSCAN".to_string());
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "CNR.BR" => {
                            let nnode = ngrid.name_tile(nloc, "CNR.BR", ["BR".into()]);
                            nnode.add_bel(bels::BUFG, "BUFG_BR".to_string());
                            nnode.add_bel(bels::STARTUP, "STARTUP".to_string());
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "CNR.TR" => {
                            let nnode = ngrid.name_tile(nloc, "CNR.TR", ["TR".into()]);
                            nnode.add_bel(bels::BUFG, "BUFG_TR".to_string());
                            nnode.add_bel(bels::OSC, "OSC".to_string());
                            nnode.add_bel(bels::BYPOSC, "BYPOSC".to_string());
                            nnode.add_bel(bels::BSUPD, "BSUPD".to_string());
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "IO.L" => {
                            let nnode = if row == edev.chip.row_n() - 1 {
                                ngrid.name_tile(nloc, "IO.L.CLK", ["LCLK".into()])
                            } else {
                                ngrid.name_tile(nloc, "IO.L", [format!("LR{r}")])
                            };
                            let p = (edev.chip.columns - 2) * 8
                                + (edev.chip.rows - 2) * 4
                                + (row.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(bels::IO0, format!("PAD{p}"));
                            nnode.add_bel(bels::IO1, format!("PAD{}", p + 1));
                            nnode.add_bel(bels::IO2, format!("PAD{}", p + 2));
                            nnode.add_bel(bels::IO3, format!("PAD{}", p + 3));
                            nnode.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "IO.R" => {
                            let nnode = if row == edev.chip.row_s() + 1 {
                                ngrid.name_tile(nloc, "IO.R.CLK", ["RCLK".into()])
                            } else {
                                ngrid.name_tile(nloc, "IO.R", [format!("RR{r}")])
                            };
                            let p = (edev.chip.columns - 2) * 4
                                + (edev.chip.row_n().to_idx() - row.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(bels::IO0, format!("PAD{}", p + 3));
                            nnode.add_bel(bels::IO1, format!("PAD{}", p + 2));
                            nnode.add_bel(bels::IO2, format!("PAD{}", p + 1));
                            nnode.add_bel(bels::IO3, format!("PAD{p}"));
                            nnode.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "IO.B" => {
                            let nnode = if col == edev.chip.col_w() + 1 {
                                ngrid.name_tile(nloc, "IO.B.CLK", ["BCLK".into()])
                            } else {
                                ngrid.name_tile(nloc, "IO.B", [format!("BC{c}")])
                            };
                            let p = (edev.chip.columns - 2) * 4
                                + (edev.chip.rows - 2) * 4
                                + (edev.chip.col_e().to_idx() - col.to_idx() - 1) * 4
                                + 1;
                            nnode.add_bel(bels::IO0, format!("PAD{p}"));
                            nnode.add_bel(bels::IO1, format!("PAD{}", p + 1));
                            nnode.add_bel(bels::IO2, format!("PAD{}", p + 2));
                            nnode.add_bel(bels::IO3, format!("PAD{}", p + 3));
                            nnode.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "IO.T" => {
                            let nnode = if col == edev.chip.col_e() - 2 {
                                ngrid.name_tile(nloc, "IO.T.CLK", ["TCLK".into()])
                            } else {
                                ngrid.name_tile(nloc, "IO.T", [format!("TC{c}")])
                            };
                            let p = (col.to_idx() - 1) * 4 + 1;
                            nnode.add_bel(bels::IO0, format!("PAD{}", p + 3));
                            nnode.add_bel(bels::IO1, format!("PAD{}", p + 2));
                            nnode.add_bel(bels::IO2, format!("PAD{}", p + 1));
                            nnode.add_bel(bels::IO3, format!("PAD{p}"));
                            nnode.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                            nnode.tie_name = Some(format!("GND_R{r}C{c}"));
                        }
                        "CLB" => {
                            let nnode = ngrid.name_tile(nloc, "CLB", [format!("R{r}C{c}")]);
                            nnode.add_bel(bels::LC0, format!("CLB_R{r}C{c}.LC0"));
                            nnode.add_bel(bels::LC1, format!("CLB_R{r}C{c}.LC1"));
                            nnode.add_bel(bels::LC2, format!("CLB_R{r}C{c}.LC2"));
                            nnode.add_bel(bels::LC3, format!("CLB_R{r}C{c}.LC3"));
                            nnode.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                            nnode.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                            nnode.add_bel(bels::VCC_GND, format!("VCC_GND_R{r}C{c}"));
                        }
                        "CLKL" => {
                            ngrid.name_tile(nloc, "CLKL", ["LM".into()]);
                        }
                        "CLKR" => {
                            ngrid.name_tile(nloc, "CLKR", ["RM".into()]);
                        }
                        "CLKH" => {
                            ngrid.name_tile(nloc, "CLKH", [format!("HMC{c}")]);
                        }
                        "CLKB" => {
                            ngrid.name_tile(nloc, "CLKB", ["BM".into()]);
                        }
                        "CLKT" => {
                            ngrid.name_tile(nloc, "CLKT", ["TM".into()]);
                        }
                        "CLKV" => {
                            ngrid.name_tile(nloc, "CLKV", [format!("VMR{r}")]);
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
        chip: edev.chip,
    }
}
