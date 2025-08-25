use prjcombine_interconnect::grid::CellCoord;
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{bels::xc5200 as bels, expanded::ExpandedDevice};
use unnamed_entity::EntityId;

use crate::ExpandedNamedDevice;

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);
    ngrid.tie_kind = Some("GND".to_string());
    ngrid.tie_pin_gnd = Some("O".to_string());

    for (tcrd, tile) in edev.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;

        let kind = edev.db.tile_classes.key(tile.class);
        let c = col.to_idx();
        let r = edev.chip.row_n() - row;
        match &kind[..] {
            "CNR.BL" => {
                let ntile = ngrid.name_tile(tcrd, "CNR.BL", ["BL".into()]);
                ntile.add_bel(bels::BUFG, "BUFG_BL".to_string());
                ntile.add_bel(bels::RDBK, "RDBK".to_string());
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            "CNR.TL" => {
                let ntile = ngrid.name_tile(tcrd, "CNR.TL", ["TL".into()]);
                ntile.add_bel(bels::BUFG, "BUFG_TL".to_string());
                ntile.add_bel(bels::BSCAN, "BSCAN".to_string());
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            "CNR.BR" => {
                let ntile = ngrid.name_tile(tcrd, "CNR.BR", ["BR".into()]);
                ntile.add_bel(bels::BUFG, "BUFG_BR".to_string());
                ntile.add_bel(bels::STARTUP, "STARTUP".to_string());
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            "CNR.TR" => {
                let ntile = ngrid.name_tile(tcrd, "CNR.TR", ["TR".into()]);
                ntile.add_bel(bels::BUFG, "BUFG_TR".to_string());
                ntile.add_bel(bels::OSC, "OSC".to_string());
                ntile.add_bel(bels::BYPOSC, "BYPOSC".to_string());
                ntile.add_bel(bels::BSUPD, "BSUPD".to_string());
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            "IO.L" => {
                let ntile = if row == edev.chip.row_n() - 1 {
                    ngrid.name_tile(tcrd, "IO.L.CLK", ["LCLK".into()])
                } else {
                    ngrid.name_tile(tcrd, "IO.L", [format!("LR{r}")])
                };
                let p = (edev.chip.columns - 2) * 8
                    + (edev.chip.rows - 2) * 4
                    + (row.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bels::IO0, format!("PAD{p}"));
                ntile.add_bel(bels::IO1, format!("PAD{}", p + 1));
                ntile.add_bel(bels::IO2, format!("PAD{}", p + 2));
                ntile.add_bel(bels::IO3, format!("PAD{}", p + 3));
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            "IO.R" => {
                let ntile = if row == edev.chip.row_s() + 1 {
                    ngrid.name_tile(tcrd, "IO.R.CLK", ["RCLK".into()])
                } else {
                    ngrid.name_tile(tcrd, "IO.R", [format!("RR{r}")])
                };
                let p = (edev.chip.columns - 2) * 4
                    + (edev.chip.row_n().to_idx() - row.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bels::IO0, format!("PAD{}", p + 3));
                ntile.add_bel(bels::IO1, format!("PAD{}", p + 2));
                ntile.add_bel(bels::IO2, format!("PAD{}", p + 1));
                ntile.add_bel(bels::IO3, format!("PAD{p}"));
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            "IO.B" => {
                let ntile = if col == edev.chip.col_w() + 1 {
                    ngrid.name_tile(tcrd, "IO.B.CLK", ["BCLK".into()])
                } else {
                    ngrid.name_tile(tcrd, "IO.B", [format!("BC{c}")])
                };
                let p = (edev.chip.columns - 2) * 4
                    + (edev.chip.rows - 2) * 4
                    + (edev.chip.col_e().to_idx() - col.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bels::IO0, format!("PAD{p}"));
                ntile.add_bel(bels::IO1, format!("PAD{}", p + 1));
                ntile.add_bel(bels::IO2, format!("PAD{}", p + 2));
                ntile.add_bel(bels::IO3, format!("PAD{}", p + 3));
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            "IO.T" => {
                let ntile = if col == edev.chip.col_e() - 2 {
                    ngrid.name_tile(tcrd, "IO.T.CLK", ["TCLK".into()])
                } else {
                    ngrid.name_tile(tcrd, "IO.T", [format!("TC{c}")])
                };
                let p = (col.to_idx() - 1) * 4 + 1;
                ntile.add_bel(bels::IO0, format!("PAD{}", p + 3));
                ntile.add_bel(bels::IO1, format!("PAD{}", p + 2));
                ntile.add_bel(bels::IO2, format!("PAD{}", p + 1));
                ntile.add_bel(bels::IO3, format!("PAD{p}"));
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            "CLB" => {
                let ntile = ngrid.name_tile(tcrd, "CLB", [format!("R{r}C{c}")]);
                ntile.add_bel(bels::LC0, format!("CLB_R{r}C{c}.LC0"));
                ntile.add_bel(bels::LC1, format!("CLB_R{r}C{c}.LC1"));
                ntile.add_bel(bels::LC2, format!("CLB_R{r}C{c}.LC2"));
                ntile.add_bel(bels::LC3, format!("CLB_R{r}C{c}.LC3"));
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bels::TBUF2, format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bels::TBUF3, format!("TBUF_R{r}C{c}.3"));
                ntile.add_bel(bels::VCC_GND, format!("VCC_GND_R{r}C{c}"));
            }
            "CLKL" => {
                ngrid.name_tile(tcrd, "CLKL", ["LM".into()]);
            }
            "CLKR" => {
                ngrid.name_tile(tcrd, "CLKR", ["RM".into()]);
            }
            "CLKH" => {
                ngrid.name_tile(tcrd, "CLKH", [format!("HMC{c}")]);
            }
            "CLKB" => {
                ngrid.name_tile(tcrd, "CLKB", ["BM".into()]);
            }
            "CLKT" => {
                ngrid.name_tile(tcrd, "CLKT", ["TM".into()]);
            }
            "CLKV" => {
                ngrid.name_tile(tcrd, "CLKV", [format!("VMR{r}")]);
            }
            _ => unreachable!(),
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid,
        chip: edev.chip,
    }
}
