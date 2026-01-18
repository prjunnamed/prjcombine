use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::CellCoord;
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{
    expanded::ExpandedDevice,
    xc5200::{bslots, tcls},
};

use crate::ExpandedNamedDevice;

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);
    ngrid.tie_kind = Some("GND".to_string());
    ngrid.tie_pin_gnd = Some("O".to_string());

    for (tcrd, tile) in edev.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;

        let c = col.to_idx();
        let r = edev.chip.row_n() - row;
        match tile.class {
            tcls::CNR_SW => {
                let ntile = ngrid.name_tile(tcrd, "CNR_SW", ["BL".into()]);
                ntile.add_bel(bslots::BUFG, "BUFG_BL".to_string());
                ntile.add_bel(bslots::RDBK, "RDBK".to_string());
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            tcls::CNR_NW => {
                let ntile = ngrid.name_tile(tcrd, "CNR_NW", ["TL".into()]);
                ntile.add_bel(bslots::BUFG, "BUFG_TL".to_string());
                ntile.add_bel(bslots::BSCAN, "BSCAN".to_string());
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            tcls::CNR_SE => {
                let ntile = ngrid.name_tile(tcrd, "CNR_SE", ["BR".into()]);
                ntile.add_bel(bslots::BUFG, "BUFG_BR".to_string());
                ntile.add_bel(bslots::STARTUP, "STARTUP".to_string());
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            tcls::CNR_NE => {
                let ntile = ngrid.name_tile(tcrd, "CNR_NE", ["TR".into()]);
                ntile.add_bel(bslots::BUFG, "BUFG_TR".to_string());
                ntile.add_bel(bslots::OSC, "OSC".to_string());
                ntile.add_bel(bslots::BYPOSC, "BYPOSC".to_string());
                ntile.add_bel(bslots::BSUPD, "BSUPD".to_string());
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            tcls::IO_W => {
                let ntile = if row == edev.chip.row_n() - 1 {
                    ngrid.name_tile(tcrd, "IO_W_CLK", ["LCLK".into()])
                } else {
                    ngrid.name_tile(tcrd, "IO_W", [format!("LR{r}")])
                };
                let p = (edev.chip.columns - 2) * 8
                    + (edev.chip.rows - 2) * 4
                    + (row.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bslots::IO[0], format!("PAD{p}"));
                ntile.add_bel(bslots::IO[1], format!("PAD{}", p + 1));
                ntile.add_bel(bslots::IO[2], format!("PAD{}", p + 2));
                ntile.add_bel(bslots::IO[3], format!("PAD{}", p + 3));
                ntile.add_bel(bslots::TBUF[0], format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bslots::TBUF[1], format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bslots::TBUF[2], format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bslots::TBUF[3], format!("TBUF_R{r}C{c}.3"));
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            tcls::IO_E => {
                let ntile = if row == edev.chip.row_s() + 1 {
                    ngrid.name_tile(tcrd, "IO_E_CLK", ["RCLK".into()])
                } else {
                    ngrid.name_tile(tcrd, "IO_E", [format!("RR{r}")])
                };
                let p = (edev.chip.columns - 2) * 4
                    + (edev.chip.row_n().to_idx() - row.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bslots::IO[0], format!("PAD{}", p + 3));
                ntile.add_bel(bslots::IO[1], format!("PAD{}", p + 2));
                ntile.add_bel(bslots::IO[2], format!("PAD{}", p + 1));
                ntile.add_bel(bslots::IO[3], format!("PAD{p}"));
                ntile.add_bel(bslots::TBUF[0], format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bslots::TBUF[1], format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bslots::TBUF[2], format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bslots::TBUF[3], format!("TBUF_R{r}C{c}.3"));
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            tcls::IO_S => {
                let ntile = if col == edev.chip.col_w() + 1 {
                    ngrid.name_tile(tcrd, "IO_S_CLK", ["BCLK".into()])
                } else {
                    ngrid.name_tile(tcrd, "IO_S", [format!("BC{c}")])
                };
                let p = (edev.chip.columns - 2) * 4
                    + (edev.chip.rows - 2) * 4
                    + (edev.chip.col_e().to_idx() - col.to_idx() - 1) * 4
                    + 1;
                ntile.add_bel(bslots::IO[0], format!("PAD{p}"));
                ntile.add_bel(bslots::IO[1], format!("PAD{}", p + 1));
                ntile.add_bel(bslots::IO[2], format!("PAD{}", p + 2));
                ntile.add_bel(bslots::IO[3], format!("PAD{}", p + 3));
                ntile.add_bel(bslots::TBUF[0], format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bslots::TBUF[1], format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bslots::TBUF[2], format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bslots::TBUF[3], format!("TBUF_R{r}C{c}.3"));
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            tcls::IO_N => {
                let ntile = if col == edev.chip.col_e() - 2 {
                    ngrid.name_tile(tcrd, "IO_N_CLK", ["TCLK".into()])
                } else {
                    ngrid.name_tile(tcrd, "IO_N", [format!("TC{c}")])
                };
                let p = (col.to_idx() - 1) * 4 + 1;
                ntile.add_bel(bslots::IO[0], format!("PAD{}", p + 3));
                ntile.add_bel(bslots::IO[1], format!("PAD{}", p + 2));
                ntile.add_bel(bslots::IO[2], format!("PAD{}", p + 1));
                ntile.add_bel(bslots::IO[3], format!("PAD{p}"));
                ntile.add_bel(bslots::TBUF[0], format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bslots::TBUF[1], format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bslots::TBUF[2], format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bslots::TBUF[3], format!("TBUF_R{r}C{c}.3"));
                ntile.tie_name = Some(format!("GND_R{r}C{c}"));
            }
            tcls::CLB => {
                let ntile = ngrid.name_tile(tcrd, "CLB", [format!("R{r}C{c}")]);
                ntile.add_bel(bslots::LC[0], format!("CLB_R{r}C{c}.LC0"));
                ntile.add_bel(bslots::LC[1], format!("CLB_R{r}C{c}.LC1"));
                ntile.add_bel(bslots::LC[2], format!("CLB_R{r}C{c}.LC2"));
                ntile.add_bel(bslots::LC[3], format!("CLB_R{r}C{c}.LC3"));
                ntile.add_bel(bslots::TBUF[0], format!("TBUF_R{r}C{c}.0"));
                ntile.add_bel(bslots::TBUF[1], format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bslots::TBUF[2], format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bslots::TBUF[3], format!("TBUF_R{r}C{c}.3"));
                ntile.add_bel(bslots::VCC_GND, format!("VCC_GND_R{r}C{c}"));
            }
            tcls::LLV_W => {
                ngrid.name_tile(tcrd, "LLV_W", ["LM".into()]);
            }
            tcls::LLV_E => {
                ngrid.name_tile(tcrd, "LLV_E", ["RM".into()]);
            }
            tcls::LLV => {
                ngrid.name_tile(tcrd, "LLV", [format!("HMC{c}")]);
            }
            tcls::LLH_S => {
                ngrid.name_tile(tcrd, "LLH_S", ["BM".into()]);
            }
            tcls::LLH_N => {
                ngrid.name_tile(tcrd, "LLH_N", ["TM".into()]);
            }
            tcls::LLH => {
                ngrid.name_tile(tcrd, "LLH", [format!("VMR{r}")]);
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
