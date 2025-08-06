use std::fmt::Write;

use prjcombine_interconnect::grid::{CellCoord, ColId, RowId};
use prjcombine_re_xilinx_naming::{
    db::{NamingDb, RawTileId},
    grid::ExpandedGridNaming,
};
use prjcombine_xc2000::{
    bels::xc4000 as bels,
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
};
use unnamed_entity::EntityId;

use crate::ExpandedNamedDevice;

fn get_tile_kind(chip: &Chip, col: ColId, row: RowId) -> &'static str {
    if col == chip.col_w() {
        if row == chip.row_s() {
            "LL"
        } else if row == chip.row_n() {
            "UL"
        } else if row == chip.row_s() + 1 {
            "LEFTSB"
        } else if row == chip.row_n() - 1 {
            "LEFTT"
        } else if chip.kind.is_xl() && row == chip.row_qb() {
            if row.to_idx().is_multiple_of(2) {
                "LEFTF"
            } else {
                "LEFTSF"
            }
        } else if chip.kind.is_xl() && row == chip.row_qt() - 1 {
            if row.to_idx().is_multiple_of(2) {
                "LEFTF1"
            } else {
                "LEFTSF1"
            }
        } else if row.to_idx().is_multiple_of(2) {
            "LEFT"
        } else {
            "LEFTS"
        }
    } else if col == chip.col_e() {
        let row_f = if chip.is_buff_large {
            chip.row_qb() + 1
        } else {
            chip.row_qb()
        };
        let row_f1 = if chip.is_buff_large {
            chip.row_qt() - 2
        } else {
            chip.row_qt() - 1
        };
        if row == chip.row_s() {
            "LR"
        } else if row == chip.row_n() {
            "UR"
        } else if row == chip.row_s() + 1 {
            "RTSB"
        } else if row == chip.row_n() - 1 {
            "RTT"
        } else if chip.kind.is_xl() && row == row_f {
            if row.to_idx().is_multiple_of(2) {
                "RTF"
            } else {
                "RTSF"
            }
        } else if chip.kind.is_xl() && row == row_f1 {
            if row.to_idx().is_multiple_of(2) {
                "RTF1"
            } else {
                "RTSF1"
            }
        } else if row.to_idx().is_multiple_of(2) {
            "RT"
        } else {
            "RTS"
        }
    } else if row == chip.row_s() {
        if col == chip.col_w() + 1 {
            "BOTSL"
        } else if col == chip.col_e() - 1 {
            "BOTRR"
        } else if col.to_idx().is_multiple_of(2) {
            "BOT"
        } else {
            "BOTS"
        }
    } else if row == chip.row_n() {
        if col == chip.col_w() + 1 {
            "TOPSL"
        } else if col == chip.col_e() - 1 {
            "TOPRR"
        } else if col.to_idx().is_multiple_of(2) {
            "TOP"
        } else {
            "TOPS"
        }
    } else {
        "CENTER"
    }
}

fn get_tile_name(chip: &Chip, col: ColId, row: RowId) -> String {
    let r = chip.row_n().to_idx() - row.to_idx();
    let c = col.to_idx();
    if col == chip.col_w() {
        if row == chip.row_s() {
            "BL".into()
        } else if row == chip.row_n() {
            "TL".into()
        } else {
            format!("LR{r}")
        }
    } else if col == chip.col_e() {
        if row == chip.row_s() {
            "BR".into()
        } else if row == chip.row_n() {
            "TR".into()
        } else {
            format!("RR{r}")
        }
    } else {
        if row == chip.row_s() {
            format!("BC{c}")
        } else if row == chip.row_n() {
            format!("TC{c}")
        } else {
            format!("R{r}C{c}")
        }
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let chip = edev.chip;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);
    ngrid.tie_kind = Some("TIE".to_string());
    ngrid.tie_pin_gnd = Some("O".to_string());
    for (tcrd, tile) in egrid.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;
        let kind = egrid.db.tile_classes.key(tile.class);
        let c = col.to_idx();
        let r = chip.row_n() - row;
        match &kind[..] {
            "CNR.BL" => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    "CNR.BL",
                    [
                        get_tile_name(chip, col, row),
                        get_tile_name(chip, col + 1, row),
                    ],
                );
                if chip.kind == ChipKind::SpartanXl {
                    ntile.add_bel(bels::BUFGLS_H, "BUFGLS_SSW".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGLS_WSW".to_string());
                } else if chip.kind.is_xl() {
                    ntile.add_bel(bels::PULLUP_DEC0_H, format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bels::PULLUP_DEC1_H, format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bels::PULLUP_DEC2_H, format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bels::PULLUP_DEC3_H, format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bels::PULLUP_DEC0_V, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_DEC1_V, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_DEC2_V, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_DEC3_V, format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bels::BUFG_H, "BUFG_SSW".to_string());
                    ntile.add_bel(bels::BUFG_V, "BUFG_WSW".to_string());
                    ntile.add_bel(bels::BUFGE_H, "BUFGE_SSW".to_string());
                    ntile.add_bel(bels::BUFGE_V, "BUFGE_WSW".to_string());
                    ntile.add_bel(bels::BUFGLS_H, "BUFGLS_SSW".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGLS_WSW".to_string());
                    ntile.add_bel(bels::MD0, "MD0".to_string());
                    ntile.add_bel(bels::MD1, "MD1".to_string());
                    ntile.add_bel(bels::MD2, "MD2".to_string());
                } else {
                    ntile.add_bel(bels::PULLUP_DEC0_H, format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bels::PULLUP_DEC1_H, format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bels::PULLUP_DEC2_H, format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bels::PULLUP_DEC3_H, format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bels::PULLUP_DEC0_V, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_DEC1_V, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_DEC2_V, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_DEC3_V, format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bels::BUFGLS_H, "BUFGP_BL".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGS_BL".to_string());
                    ntile.add_bel(bels::CIN, "CI_BL".to_string());
                    ntile.add_bel(bels::MD0, "MD0".to_string());
                    ntile.add_bel(bels::MD1, "MD1".to_string());
                    ntile.add_bel(bels::MD2, "MD2".to_string());
                }
                ntile.add_bel(bels::RDBK, "RDBK".to_string());
            }
            "CNR.TL" => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    "CNR.TL",
                    [
                        get_tile_name(chip, col, row),
                        get_tile_name(chip, col + 1, row),
                        get_tile_name(chip, col, row - 1),
                        get_tile_name(chip, col + 1, row - 1),
                    ],
                );
                if chip.kind == ChipKind::SpartanXl {
                    ntile.add_bel(bels::BUFGLS_H, "BUFGLS_NNW".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGLS_WNW".to_string());
                } else if chip.kind.is_xl() {
                    ntile.add_bel(bels::PULLUP_DEC0_H, format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bels::PULLUP_DEC1_H, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_DEC2_H, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_DEC3_H, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_DEC0_V, format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bels::PULLUP_DEC1_V, format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bels::PULLUP_DEC2_V, format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bels::PULLUP_DEC3_V, format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bels::BUFG_H, "BUFG_NNW".to_string());
                    ntile.add_bel(bels::BUFG_V, "BUFG_WNW".to_string());
                    ntile.add_bel(bels::BUFGE_H, "BUFGE_NNW".to_string());
                    ntile.add_bel(bels::BUFGE_V, "BUFGE_WNW".to_string());
                    ntile.add_bel(bels::BUFGLS_H, "BUFGLS_NNW".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGLS_WNW".to_string());
                } else {
                    ntile.add_bel(bels::PULLUP_DEC0_H, format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bels::PULLUP_DEC1_H, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_DEC2_H, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_DEC3_H, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_DEC0_V, format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bels::PULLUP_DEC1_V, format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bels::PULLUP_DEC2_V, format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bels::PULLUP_DEC3_V, format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bels::BUFGLS_H, "BUFGS_TL".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGP_TL".to_string());
                    ntile.add_bel(bels::CIN, "CI_TL".to_string());
                }
                ntile.add_bel(bels::BSCAN, "BSCAN".to_string());
            }
            "CNR.BR" => {
                let ntile = ngrid.name_tile(tcrd, "CNR.BR", [get_tile_name(chip, col, row)]);
                if chip.kind == ChipKind::SpartanXl {
                    ntile.add_bel(bels::BUFGLS_H, "BUFGLS_SSE".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGLS_ESE".to_string());
                } else if chip.kind.is_xl() {
                    ntile.add_bel(bels::PULLUP_DEC0_H, format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bels::PULLUP_DEC1_H, format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bels::PULLUP_DEC2_H, format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bels::PULLUP_DEC3_H, format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bels::PULLUP_DEC0_V, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_DEC1_V, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_DEC2_V, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_DEC3_V, format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bels::BUFG_H, "BUFG_SSE".to_string());
                    ntile.add_bel(bels::BUFG_V, "BUFG_ESE".to_string());
                    ntile.add_bel(bels::BUFGE_H, "BUFGE_SSE".to_string());
                    ntile.add_bel(bels::BUFGE_V, "BUFGE_ESE".to_string());
                    ntile.add_bel(bels::BUFGLS_H, "BUFGLS_SSE".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGLS_ESE".to_string());
                } else {
                    ntile.add_bel(bels::PULLUP_DEC0_H, format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bels::PULLUP_DEC1_H, format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bels::PULLUP_DEC2_H, format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bels::PULLUP_DEC3_H, format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bels::PULLUP_DEC0_V, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_DEC1_V, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_DEC2_V, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_DEC3_V, format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bels::BUFGLS_H, "BUFGS_BR".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGP_BR".to_string());
                    ntile.add_bel(bels::COUT, "CO_BR".to_string());
                }
                ntile.add_bel(bels::STARTUP, "STARTUP".to_string());
                ntile.add_bel(bels::READCLK, "RDCLK".to_string());
                ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
            }
            "CNR.TR" => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    "CNR.TR",
                    [
                        get_tile_name(chip, col, row),
                        get_tile_name(chip, col, row - 1),
                    ],
                );
                if chip.kind == ChipKind::SpartanXl {
                    ntile.add_bel(bels::BUFGLS_H, "BUFGLS_NNE".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGLS_ENE".to_string());
                } else if chip.kind.is_xl() {
                    ntile.add_bel(bels::PULLUP_DEC0_H, format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bels::PULLUP_DEC1_H, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_DEC2_H, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_DEC3_H, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_DEC0_V, format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bels::PULLUP_DEC1_V, format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bels::PULLUP_DEC2_V, format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bels::PULLUP_DEC3_V, format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bels::BUFG_H, "BUFG_NNE".to_string());
                    ntile.add_bel(bels::BUFG_V, "BUFG_ENE".to_string());
                    ntile.add_bel(bels::BUFGE_H, "BUFGE_NNE".to_string());
                    ntile.add_bel(bels::BUFGE_V, "BUFGE_ENE".to_string());
                    ntile.add_bel(bels::BUFGLS_H, "BUFGLS_NNE".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGLS_ENE".to_string());
                } else {
                    ntile.add_bel(bels::PULLUP_DEC0_H, format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bels::PULLUP_DEC1_H, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_DEC2_H, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_DEC3_H, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_DEC0_V, format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bels::PULLUP_DEC1_V, format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bels::PULLUP_DEC2_V, format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bels::PULLUP_DEC3_V, format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bels::BUFGLS_H, "BUFGP_TR".to_string());
                    ntile.add_bel(bels::BUFGLS_V, "BUFGS_TR".to_string());
                    ntile.add_bel(bels::COUT, "CO_TR".to_string());
                }
                ntile.add_bel(bels::UPDATE, "UPDATE".to_string());
                ntile.add_bel(bels::OSC, "OSC".to_string());
                ntile.add_bel(bels::TDO, "TDO".to_string());
            }
            _ if kind.starts_with("IO.L") => {
                let kind = get_tile_kind(chip, col, row);
                let kind_s = get_tile_kind(chip, col, row - 1);
                let mut names = vec![
                    get_tile_name(chip, col, row),
                    get_tile_name(chip, col, row - 1),
                    get_tile_name(chip, col + 1, row),
                    get_tile_name(chip, col, row + 1),
                ];
                if chip.kind == ChipKind::Xc4000Xv {
                    names.push(format!("LHIR{r}"));
                }
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}.{kind_s}"), names);
                let p = (chip.columns - 2) * 4 + (chip.rows - 2) * 2 + (row.to_idx() - 1) * 2 + 1;
                ntile.add_bel(bels::IO0, format!("PAD{}", p + 1));
                ntile.add_bel(bels::IO1, format!("PAD{p}"));
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bels::PULLUP_TBUF0, format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bels::PULLUP_TBUF1, format!("PULLUP_R{r}C{c}.1"));
                if chip.kind != ChipKind::SpartanXl {
                    ntile.add_bel(bels::DEC0, format!("DEC_R{r}C{c}.1"));
                    ntile.add_bel(bels::DEC1, format!("DEC_R{r}C{c}.2"));
                    ntile.add_bel(bels::DEC2, format!("DEC_R{r}C{c}.3"));
                }
                if chip.kind == ChipKind::Xc4000Xv {
                    ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                    ntile.tie_rt = RawTileId::from_idx(4);
                }
            }
            _ if kind.starts_with("IO.R") => {
                let kind = get_tile_kind(chip, col, row);
                let kind_s = get_tile_kind(chip, col, row - 1);
                let mut names = vec![
                    get_tile_name(chip, col, row),
                    get_tile_name(chip, col, row - 1),
                    get_tile_name(chip, col, row + 1),
                ];
                if chip.kind == ChipKind::Xc4000Xv {
                    names.push(format!("RHIR{r}"));
                }
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}.{kind_s}"), names);
                let p = (chip.columns - 2) * 2 + (chip.row_n().to_idx() - row.to_idx() - 1) * 2 + 1;
                ntile.add_bel(bels::IO0, format!("PAD{p}"));
                ntile.add_bel(bels::IO1, format!("PAD{}", p + 1));
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bels::PULLUP_TBUF0, format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bels::PULLUP_TBUF1, format!("PULLUP_R{r}C{c}.1"));
                if chip.kind != ChipKind::SpartanXl {
                    ntile.add_bel(bels::DEC0, format!("DEC_R{r}C{c}.1"));
                    ntile.add_bel(bels::DEC1, format!("DEC_R{r}C{c}.2"));
                    ntile.add_bel(bels::DEC2, format!("DEC_R{r}C{c}.3"));
                }

                ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
            }
            _ if kind.starts_with("IO.B") => {
                let kind = get_tile_kind(chip, col, row);
                let kind_e = get_tile_kind(chip, col + 1, row);
                let mut names = vec![
                    get_tile_name(chip, col, row),
                    get_tile_name(chip, col, row + 1),
                    get_tile_name(chip, col + 1, row),
                    get_tile_name(chip, col - 1, row),
                ];
                if chip.kind == ChipKind::Xc4000Xv {
                    names.push(format!("BVIC{c}"));
                }
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}.{kind_e}"), names);
                let p = (chip.columns - 2) * 2
                    + (chip.rows - 2) * 2
                    + (chip.col_e().to_idx() - col.to_idx() - 1) * 2
                    + 1;

                ntile.add_bel(bels::IO0, format!("PAD{}", p + 1));
                ntile.add_bel(bels::IO1, format!("PAD{p}"));
                if chip.kind != ChipKind::SpartanXl {
                    ntile.add_bel(bels::DEC0, format!("DEC_R{r}C{c}.1"));
                    ntile.add_bel(bels::DEC1, format!("DEC_R{r}C{c}.2"));
                    ntile.add_bel(bels::DEC2, format!("DEC_R{r}C{c}.3"));
                }
                ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
            }
            _ if kind.starts_with("IO.T") => {
                let kind = get_tile_kind(chip, col, row);
                let kind_e = get_tile_kind(chip, col + 1, row);
                let mut names = vec![
                    get_tile_name(chip, col, row),
                    get_tile_name(chip, col + 1, row),
                    get_tile_name(chip, col - 1, row),
                ];
                if chip.kind == ChipKind::Xc4000Xv {
                    names.push(format!("TVIC{c}"));
                }
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}.{kind_e}"), names);
                let p = (col.to_idx() - 1) * 2 + 1;
                ntile.add_bel(bels::IO0, format!("PAD{p}"));
                ntile.add_bel(bels::IO1, format!("PAD{}", p + 1));
                if chip.kind != ChipKind::SpartanXl {
                    ntile.add_bel(bels::DEC0, format!("DEC_R{r}C{c}.1"));
                    ntile.add_bel(bels::DEC1, format!("DEC_R{r}C{c}.2"));
                    ntile.add_bel(bels::DEC2, format!("DEC_R{r}C{c}.3"));
                }
                if chip.kind == ChipKind::Xc4000Xv {
                    ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                    ntile.tie_rt = RawTileId::from_idx(3);
                }
            }
            _ if kind.starts_with("CLB") => {
                let mut naming = "CLB".to_string();
                if row == chip.row_n() - 1 {
                    let kind_n = get_tile_kind(chip, col, row + 1);
                    write!(naming, ".{kind_n}").unwrap();
                }
                if col == chip.col_e() - 1 {
                    let kind_e = get_tile_kind(chip, col + 1, row);
                    write!(naming, ".{kind_e}").unwrap();
                }
                let mut names = vec![
                    get_tile_name(chip, col, row),
                    get_tile_name(chip, col, row + 1),
                    get_tile_name(chip, col + 1, row),
                ];
                if chip.kind == ChipKind::Xc4000Xv {
                    names.extend([
                        format!("VIR{r}C{c}"),
                        format!("HIR{r}C{c}"),
                        format!("VHIR{r}C{c}"),
                    ]);
                }
                let ntile = ngrid.name_tile(tcrd, &naming, names);
                ntile.add_bel(bels::CLB, format!("CLB_R{r}C{c}"));
                ntile.add_bel(bels::TBUF0, format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bels::TBUF1, format!("TBUF_R{r}C{c}.1"));
                ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
            }

            "LLH.IO.B" => {
                ngrid.name_tile(tcrd, "LLH.IO.B", ["BM".into()]);
            }
            "LLH.IO.T" => {
                ngrid.name_tile(tcrd, "LLH.IO.T", ["TM".into()]);
            }
            _ if kind.starts_with("LLH.CLB") => {
                let naming = if row < chip.row_mid() {
                    "LLH.CLB.B"
                } else {
                    "LLH.CLB.T"
                };
                ngrid.name_tile(tcrd, naming, [format!("VMR{r}")]);
            }
            "LLV.IO.L" => {
                ngrid.name_tile(tcrd, "LLV.IO.L", ["LM".into()]);
            }
            "LLV.IO.R" => {
                ngrid.name_tile(tcrd, "LLV.IO.R", ["RM".into()]);
            }
            "LLV.CLB" => {
                ngrid.name_tile(tcrd, "LLV.CLB", [format!("HMC{c}")]);
            }

            "LLHQ.IO.B" => {
                let lr = if col == chip.col_ql() {
                    'L'
                } else if col == chip.col_qr() {
                    'R'
                } else {
                    unreachable!()
                };
                ngrid.name_tile(tcrd, "LLHQ.IO.B", [format!("BQ{lr}")]);
            }
            "LLHQ.IO.T" => {
                let lr = if col == chip.col_ql() {
                    'L'
                } else if col == chip.col_qr() {
                    'R'
                } else {
                    unreachable!()
                };
                ngrid.name_tile(tcrd, "LLHQ.IO.T", [format!("TQ{lr}")]);
            }
            _ if kind.starts_with("LLHQ.CLB") => {
                let lr = if col == chip.col_ql() {
                    'L'
                } else if col == chip.col_qr() {
                    'R'
                } else {
                    unreachable!()
                };
                let naming = if chip.kind != ChipKind::Xc4000Xv {
                    "LLHQ.CLB"
                } else if row >= chip.row_qb() && row < chip.row_qt() {
                    "LLHQ.CLB.I"
                } else {
                    "LLHQ.CLB.O"
                };
                let ntile = ngrid.name_tile(tcrd, naming, [format!("VQ{lr}R{r}")]);
                if chip.kind == ChipKind::Xc4000Xla {
                    ntile.add_bel(
                        bels::PULLUP_TBUF0_W,
                        format!("PULLUP_R{r}C{cc}.4", cc = c - 1),
                    );
                    ntile.add_bel(bels::PULLUP_TBUF0_E, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(
                        bels::PULLUP_TBUF1_W,
                        format!("PULLUP_R{r}C{cc}.3", cc = c - 1),
                    );
                    ntile.add_bel(bels::PULLUP_TBUF1_E, format!("PULLUP_R{r}C{c}.1"));
                } else {
                    ntile.add_bel(bels::PULLUP_TBUF0_W, format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bels::PULLUP_TBUF0_E, format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bels::PULLUP_TBUF1_W, format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bels::PULLUP_TBUF1_E, format!("PULLUP_R{r}C{c}.1"));
                }
            }
            "LLHC.IO.B" => {
                let ntile = ngrid.name_tile(tcrd, "LLHC.IO.B", ["BM".into()]);
                ntile.add_bel(bels::PULLUP_DEC0_W, format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bels::PULLUP_DEC0_E, format!("PULLUP_R{r}C{c}.5"));
                ntile.add_bel(bels::PULLUP_DEC1_W, format!("PULLUP_R{r}C{c}.3"));
                ntile.add_bel(bels::PULLUP_DEC1_E, format!("PULLUP_R{r}C{c}.6"));
                ntile.add_bel(bels::PULLUP_DEC2_W, format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bels::PULLUP_DEC2_E, format!("PULLUP_R{r}C{c}.7"));
                ntile.add_bel(bels::PULLUP_DEC3_W, format!("PULLUP_R{r}C{c}.1"));
                ntile.add_bel(bels::PULLUP_DEC3_E, format!("PULLUP_R{r}C{c}.8"));
            }
            "LLHC.IO.T" => {
                let ntile = ngrid.name_tile(tcrd, "LLHC.IO.T", ["TM".into()]);
                ntile.add_bel(bels::PULLUP_DEC0_W, format!("PULLUP_R{r}C{c}.1"));
                ntile.add_bel(bels::PULLUP_DEC0_E, format!("PULLUP_R{r}C{c}.8"));
                ntile.add_bel(bels::PULLUP_DEC1_W, format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bels::PULLUP_DEC1_E, format!("PULLUP_R{r}C{c}.7"));
                ntile.add_bel(bels::PULLUP_DEC2_W, format!("PULLUP_R{r}C{c}.3"));
                ntile.add_bel(bels::PULLUP_DEC2_E, format!("PULLUP_R{r}C{c}.6"));
                ntile.add_bel(bels::PULLUP_DEC3_W, format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bels::PULLUP_DEC3_E, format!("PULLUP_R{r}C{c}.5"));
            }
            _ if kind.starts_with("LLHC.CLB") => {
                let naming = if row >= chip.row_qb() && row < chip.row_qt() {
                    "LLHC.CLB.I"
                } else {
                    "LLHC.CLB.O"
                };
                let ntile = ngrid.name_tile(tcrd, naming, [format!("VMR{r}")]);
                ntile.add_bel(bels::PULLUP_TBUF0_W, format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bels::PULLUP_TBUF0_E, format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bels::PULLUP_TBUF1_W, format!("PULLUP_R{r}C{c}.1"));
                ntile.add_bel(bels::PULLUP_TBUF1_E, format!("PULLUP_R{r}C{c}.3"));
            }
            _ if kind.starts_with("LLVQ.IO.L") => {
                let bt = if row == chip.row_qb() {
                    'B'
                } else if row == chip.row_qt() {
                    'T'
                } else {
                    unreachable!()
                };
                let ntile = ngrid.name_tile(tcrd, kind, [format!("LQ{bt}")]);
                let sn = if bt == 'B' { 'S' } else { 'N' };
                ntile.add_bel(bels::BUFF, format!("BUFF_{sn}W"));
            }
            _ if kind.starts_with("LLVQ.IO.R") => {
                let bt = if row == chip.row_qb() {
                    'B'
                } else if row == chip.row_qt() {
                    'T'
                } else {
                    unreachable!()
                };
                let naming = if chip.is_buff_large {
                    if bt == 'B' {
                        "LLVQ.IO.R.B"
                    } else {
                        "LLVQ.IO.R.T"
                    }
                } else {
                    if bt == 'B' {
                        "LLVQ.IO.R.BS"
                    } else {
                        "LLVQ.IO.R.TS"
                    }
                };
                let ntile = ngrid.name_tile(tcrd, naming, [format!("RQ{bt}")]);
                let sn = if bt == 'B' { 'S' } else { 'N' };
                ntile.add_bel(bels::BUFF, format!("BUFF_{sn}E"));
            }
            "LLVQ.CLB" => {
                let bt = if row == chip.row_qb() {
                    'B'
                } else if row == chip.row_qt() {
                    'T'
                } else {
                    unreachable!()
                };
                ngrid.name_tile(tcrd, "LLVQ.CLB", [format!("HQ{bt}C{c}")]);
            }
            "LLVC.IO.L" => {
                let ntile = ngrid.name_tile(tcrd, "LLVC.IO.L", ["LM".into()]);
                let r = r + 1;
                ntile.add_bel(bels::PULLUP_DEC0_S, format!("PULLUP_R{r}C{c}.10"));
                ntile.add_bel(bels::PULLUP_DEC0_N, format!("PULLUP_R{r}C{c}.3"));
                ntile.add_bel(bels::PULLUP_DEC1_S, format!("PULLUP_R{r}C{c}.9"));
                ntile.add_bel(bels::PULLUP_DEC1_N, format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bels::PULLUP_DEC2_S, format!("PULLUP_R{r}C{c}.8"));
                ntile.add_bel(bels::PULLUP_DEC2_N, format!("PULLUP_R{r}C{c}.5"));
                ntile.add_bel(bels::PULLUP_DEC3_S, format!("PULLUP_R{r}C{c}.7"));
                ntile.add_bel(bels::PULLUP_DEC3_N, format!("PULLUP_R{r}C{c}.6"));
            }
            "LLVC.IO.R" => {
                let ntile = ngrid.name_tile(tcrd, "LLVC.IO.R", ["RM".into()]);
                let r = r + 1;
                ntile.add_bel(bels::PULLUP_DEC0_S, format!("PULLUP_R{r}C{c}.10"));
                ntile.add_bel(bels::PULLUP_DEC0_N, format!("PULLUP_R{r}C{c}.3"));
                ntile.add_bel(bels::PULLUP_DEC1_S, format!("PULLUP_R{r}C{c}.9"));
                ntile.add_bel(bels::PULLUP_DEC1_N, format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bels::PULLUP_DEC2_S, format!("PULLUP_R{r}C{c}.8"));
                ntile.add_bel(bels::PULLUP_DEC2_N, format!("PULLUP_R{r}C{c}.5"));
                ntile.add_bel(bels::PULLUP_DEC3_S, format!("PULLUP_R{r}C{c}.7"));
                ntile.add_bel(bels::PULLUP_DEC3_N, format!("PULLUP_R{r}C{c}.6"));
            }
            "LLVC.CLB" => {
                ngrid.name_tile(tcrd, "LLVC.CLB", [format!("HMC{c}")]);
            }

            "CLKQ" => {
                let bt = if row == chip.row_qb() {
                    'B'
                } else if row == chip.row_qt() {
                    'T'
                } else {
                    unreachable!()
                };
                let lr = if col == chip.col_ql() {
                    'L'
                } else if col == chip.col_qr() {
                    'R'
                } else {
                    unreachable!()
                };

                ngrid.name_tile(tcrd, &format!("CLKQ.{bt}"), [format!("Q{bt}{lr}")]);
            }
            "CLKC" => {
                ngrid.name_tile(tcrd, "CLKC", ["M".into()]);
            }
            "CLKQC" => {
                let bt = if row == chip.row_qb() {
                    'B'
                } else if row == chip.row_qt() {
                    'T'
                } else {
                    unreachable!()
                };
                ngrid.name_tile(tcrd, &format!("CLKQC.{bt}"), [format!("VMQ{bt}")]);
            }

            _ => unreachable!(),
        }
    }
    ExpandedNamedDevice { edev, ngrid, chip }
}
