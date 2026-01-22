use std::fmt::Write;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    dir::{DirH, DirV},
    grid::{CellCoord, ColId, RowId},
};
use prjcombine_re_xilinx_naming::{
    db::{NamingDb, RawTileId},
    grid::ExpandedGridNaming,
};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
    xc4000::{bslots, xc4000::tcls},
};

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
        } else if chip.kind.is_xl() && row == chip.row_q(DirV::S) {
            if row.to_idx().is_multiple_of(2) {
                "LEFTF"
            } else {
                "LEFTSF"
            }
        } else if chip.kind.is_xl() && row == chip.row_q(DirV::N) - 1 {
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
            chip.row_q(DirV::S) + 1
        } else {
            chip.row_q(DirV::S)
        };
        let row_f1 = if chip.is_buff_large {
            chip.row_q(DirV::N) - 2
        } else {
            chip.row_q(DirV::N) - 1
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
    let chip = edev.chip;
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);
    ngrid.tie_kind = Some("TIE".to_string());
    ngrid.tie_pin_gnd = Some("O".to_string());
    for (tcrd, tile) in edev.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;
        let kind = edev.db.tile_classes.key(tile.class);
        let c = col.to_idx();
        let r = chip.row_n() - row;
        match tile.class {
            tcls::CNR_SW => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    "CNR_SW",
                    [
                        get_tile_name(chip, col, row),
                        get_tile_name(chip, col + 1, row),
                        "M".into(),
                    ],
                );
                if chip.kind == ChipKind::SpartanXl {
                    ntile.add_bel(bslots::BUFG_H, "BUFGLS_SSW".to_string());
                    ntile.add_bel(bslots::BUFG_V, "BUFGLS_WSW".to_string());
                } else if chip.kind.is_xl() {
                    ntile.add_bel(bslots::PULLUP_DEC_H[0], format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[1], format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[2], format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[3], format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[0], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[1], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[2], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[3], format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel_multi(
                        bslots::BUFG_H,
                        vec![
                            "BUFG_SSW".to_string(),
                            "BUFGE_SSW".to_string(),
                            "BUFGLS_SSW".to_string(),
                        ],
                    );
                    ntile.add_bel_multi(
                        bslots::BUFG_V,
                        vec![
                            "BUFG_WSW".to_string(),
                            "BUFGE_WSW".to_string(),
                            "BUFGLS_WSW".to_string(),
                        ],
                    );
                    ntile.add_bel(bslots::MD0, "MD0".to_string());
                    ntile.add_bel(bslots::MD1, "MD1".to_string());
                    ntile.add_bel(bslots::MD2, "MD2".to_string());
                } else {
                    ntile.add_bel(bslots::PULLUP_DEC_H[0], format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[1], format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[2], format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[3], format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[0], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[1], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[2], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[3], format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bslots::BUFG_H, "BUFGP_BL".to_string());
                    ntile.add_bel(bslots::BUFG_V, "BUFGS_BL".to_string());
                    ntile.add_bel(bslots::MISC_SW, "CI_BL".to_string());
                    ntile.add_bel(bslots::MD0, "MD0".to_string());
                    ntile.add_bel(bslots::MD1, "MD1".to_string());
                    ntile.add_bel(bslots::MD2, "MD2".to_string());
                }
                ntile.add_bel(bslots::RDBK, "RDBK".to_string());
            }
            tcls::CNR_NW => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    "CNR_NW",
                    [
                        get_tile_name(chip, col, row),
                        get_tile_name(chip, col + 1, row),
                        get_tile_name(chip, col, row - 1),
                        get_tile_name(chip, col + 1, row - 1),
                        "M".into(),
                    ],
                );
                if chip.kind == ChipKind::SpartanXl {
                    ntile.add_bel(bslots::BUFG_H, "BUFGLS_NNW".to_string());
                    ntile.add_bel(bslots::BUFG_V, "BUFGLS_WNW".to_string());
                } else if chip.kind.is_xl() {
                    ntile.add_bel(bslots::PULLUP_DEC_H[0], format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[1], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[2], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[3], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[0], format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[1], format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[2], format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[3], format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel_multi(
                        bslots::BUFG_H,
                        vec![
                            "BUFG_NNW".to_string(),
                            "BUFGE_NNW".to_string(),
                            "BUFGLS_NNW".to_string(),
                        ],
                    );
                    ntile.add_bel_multi(
                        bslots::BUFG_V,
                        vec![
                            "BUFG_WNW".to_string(),
                            "BUFGE_WNW".to_string(),
                            "BUFGLS_WNW".to_string(),
                        ],
                    );
                } else {
                    ntile.add_bel(bslots::PULLUP_DEC_H[0], format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[1], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[2], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[3], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[0], format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[1], format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[2], format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[3], format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bslots::BUFG_H, "BUFGS_TL".to_string());
                    ntile.add_bel(bslots::BUFG_V, "BUFGP_TL".to_string());
                    ntile.add_bel(bslots::MISC_NW, "CI_TL".to_string());
                }
                ntile.add_bel(bslots::BSCAN, "BSCAN".to_string());
            }
            tcls::CNR_SE => {
                let ntile =
                    ngrid.name_tile(tcrd, "CNR_SE", [get_tile_name(chip, col, row), "M".into()]);
                if chip.kind == ChipKind::SpartanXl {
                    ntile.add_bel(bslots::BUFG_H, "BUFGLS_SSE".to_string());
                    ntile.add_bel(bslots::BUFG_V, "BUFGLS_ESE".to_string());
                } else if chip.kind.is_xl() {
                    ntile.add_bel(bslots::PULLUP_DEC_H[0], format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[1], format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[2], format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[3], format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[0], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[1], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[2], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[3], format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel_multi(
                        bslots::BUFG_H,
                        vec![
                            "BUFG_SSE".to_string(),
                            "BUFGE_SSE".to_string(),
                            "BUFGLS_SSE".to_string(),
                        ],
                    );
                    ntile.add_bel_multi(
                        bslots::BUFG_V,
                        vec![
                            "BUFG_ESE".to_string(),
                            "BUFGE_ESE".to_string(),
                            "BUFGLS_ESE".to_string(),
                        ],
                    );
                } else {
                    ntile.add_bel(bslots::PULLUP_DEC_H[0], format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[1], format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[2], format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[3], format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[0], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[1], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[2], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[3], format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bslots::BUFG_H, "BUFGS_BR".to_string());
                    ntile.add_bel(bslots::BUFG_V, "BUFGP_BR".to_string());
                    ntile.add_bel(bslots::MISC_SE, "CO_BR".to_string());
                }
                ntile.add_bel(bslots::STARTUP, "STARTUP".to_string());
                ntile.add_bel(bslots::READCLK, "RDCLK".to_string());
                ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
            }
            tcls::CNR_NE => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    "CNR_NE",
                    [
                        get_tile_name(chip, col, row),
                        get_tile_name(chip, col, row - 1),
                        "M".into(),
                    ],
                );
                if chip.kind == ChipKind::SpartanXl {
                    ntile.add_bel(bslots::BUFG_H, "BUFGLS_NNE".to_string());
                    ntile.add_bel(bslots::BUFG_V, "BUFGLS_ENE".to_string());
                } else if chip.kind.is_xl() {
                    ntile.add_bel(bslots::PULLUP_DEC_H[0], format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[1], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[2], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[3], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[0], format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[1], format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[2], format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[3], format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel_multi(
                        bslots::BUFG_H,
                        vec![
                            "BUFG_NNE".to_string(),
                            "BUFGE_NNE".to_string(),
                            "BUFGLS_NNE".to_string(),
                        ],
                    );
                    ntile.add_bel_multi(
                        bslots::BUFG_V,
                        vec![
                            "BUFG_ENE".to_string(),
                            "BUFGE_ENE".to_string(),
                            "BUFGLS_ENE".to_string(),
                        ],
                    );
                } else {
                    ntile.add_bel(bslots::PULLUP_DEC_H[0], format!("PULLUP_R{r}C{c}.1"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[1], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[2], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_DEC_H[3], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[0], format!("PULLUP_R{r}C{c}.5"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[1], format!("PULLUP_R{r}C{c}.6"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[2], format!("PULLUP_R{r}C{c}.7"));
                    ntile.add_bel(bslots::PULLUP_DEC_V[3], format!("PULLUP_R{r}C{c}.8"));
                    ntile.add_bel(bslots::BUFG_H, "BUFGP_TR".to_string());
                    ntile.add_bel(bslots::BUFG_V, "BUFGS_TR".to_string());
                    ntile.add_bel(bslots::MISC_NE, "CO_TR".to_string());
                }
                ntile.add_bel(bslots::UPDATE, "UPDATE".to_string());
                ntile.add_bel(bslots::OSC, "OSC".to_string());
                ntile.add_bel(bslots::TDO, "TDO".to_string());
            }
            _ if kind.starts_with("IO_W") => {
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
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}_{kind_s}"), names);
                let p = (chip.columns - 2) * 4 + (chip.rows - 2) * 2 + (row.to_idx() - 1) * 2 + 1;
                ntile.add_bel(bslots::IO[0], format!("PAD{}", p + 1));
                ntile.add_bel(bslots::IO[1], format!("PAD{p}"));
                ntile.add_bel(bslots::TBUF[0], format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bslots::TBUF[1], format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bslots::PULLUP_TBUF[0], format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bslots::PULLUP_TBUF[1], format!("PULLUP_R{r}C{c}.1"));
                if chip.kind != ChipKind::SpartanXl {
                    ntile.add_bel(bslots::DEC[0], format!("DEC_R{r}C{c}.1"));
                    ntile.add_bel(bslots::DEC[1], format!("DEC_R{r}C{c}.2"));
                    ntile.add_bel(bslots::DEC[2], format!("DEC_R{r}C{c}.3"));
                }
                if chip.kind == ChipKind::Xc4000Xv {
                    ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                    ntile.tie_rt = RawTileId::from_idx(4);
                }
            }
            _ if kind.starts_with("IO_E") => {
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
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}_{kind_s}"), names);
                let p = (chip.columns - 2) * 2 + (chip.row_n().to_idx() - row.to_idx() - 1) * 2 + 1;
                ntile.add_bel(bslots::IO[0], format!("PAD{p}"));
                ntile.add_bel(bslots::IO[1], format!("PAD{}", p + 1));
                ntile.add_bel(bslots::TBUF[0], format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bslots::TBUF[1], format!("TBUF_R{r}C{c}.1"));
                ntile.add_bel(bslots::PULLUP_TBUF[0], format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bslots::PULLUP_TBUF[1], format!("PULLUP_R{r}C{c}.1"));
                if chip.kind != ChipKind::SpartanXl {
                    ntile.add_bel(bslots::DEC[0], format!("DEC_R{r}C{c}.1"));
                    ntile.add_bel(bslots::DEC[1], format!("DEC_R{r}C{c}.2"));
                    ntile.add_bel(bslots::DEC[2], format!("DEC_R{r}C{c}.3"));
                }

                ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
            }
            _ if kind.starts_with("IO_S") => {
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
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}_{kind_e}"), names);
                let p = (chip.columns - 2) * 2
                    + (chip.rows - 2) * 2
                    + (chip.col_e().to_idx() - col.to_idx() - 1) * 2
                    + 1;

                ntile.add_bel(bslots::IO[0], format!("PAD{}", p + 1));
                ntile.add_bel(bslots::IO[1], format!("PAD{p}"));
                if chip.kind != ChipKind::SpartanXl {
                    ntile.add_bel(bslots::DEC[0], format!("DEC_R{r}C{c}.1"));
                    ntile.add_bel(bslots::DEC[1], format!("DEC_R{r}C{c}.2"));
                    ntile.add_bel(bslots::DEC[2], format!("DEC_R{r}C{c}.3"));
                }
                ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
            }
            _ if kind.starts_with("IO_N") => {
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
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}_{kind_e}"), names);
                let p = (col.to_idx() - 1) * 2 + 1;
                ntile.add_bel(bslots::IO[0], format!("PAD{p}"));
                ntile.add_bel(bslots::IO[1], format!("PAD{}", p + 1));
                if chip.kind != ChipKind::SpartanXl {
                    ntile.add_bel(bslots::DEC[0], format!("DEC_R{r}C{c}.1"));
                    ntile.add_bel(bslots::DEC[1], format!("DEC_R{r}C{c}.2"));
                    ntile.add_bel(bslots::DEC[2], format!("DEC_R{r}C{c}.3"));
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
                    write!(naming, "_{kind_n}").unwrap();
                }
                if col == chip.col_e() - 1 {
                    let kind_e = get_tile_kind(chip, col + 1, row);
                    write!(naming, "_{kind_e}").unwrap();
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
                ntile.add_bel(bslots::CLB, format!("CLB_R{r}C{c}"));
                ntile.add_bel(bslots::TBUF[0], format!("TBUF_R{r}C{c}.2"));
                ntile.add_bel(bslots::TBUF[1], format!("TBUF_R{r}C{c}.1"));
                ntile.tie_name = Some(format!("TIE_R{r}C{c}.1"));
            }

            tcls::LLH_IO_S => {
                ngrid.name_tile(tcrd, "LLH_IO_S", ["BM".into()]);
            }
            tcls::LLH_IO_N => {
                ngrid.name_tile(tcrd, "LLH_IO_N", ["TM".into()]);
            }
            _ if kind.starts_with("LLH_CLB") => {
                let naming = if row < chip.row_mid() {
                    "LLH_CLB_S"
                } else {
                    "LLH_CLB_N"
                };
                ngrid.name_tile(tcrd, naming, [format!("VMR{r}")]);
            }
            tcls::LLV_IO_W => {
                ngrid.name_tile(tcrd, "LLV_IO_W", ["LM".into()]);
            }
            tcls::LLV_IO_E => {
                ngrid.name_tile(tcrd, "LLV_IO_E", ["RM".into()]);
            }
            tcls::LLV_CLB => {
                ngrid.name_tile(tcrd, "LLV_CLB", [format!("HMC{c}")]);
            }

            tcls::LLHQ_IO_S => {
                let lr = if col == chip.col_q(DirH::W) {
                    'L'
                } else if col == chip.col_q(DirH::E) {
                    'R'
                } else {
                    unreachable!()
                };
                ngrid.name_tile(tcrd, "LLHQ_IO_S", [format!("BQ{lr}")]);
            }
            tcls::LLHQ_IO_N => {
                let lr = if col == chip.col_q(DirH::W) {
                    'L'
                } else if col == chip.col_q(DirH::E) {
                    'R'
                } else {
                    unreachable!()
                };
                ngrid.name_tile(tcrd, "LLHQ_IO_N", [format!("TQ{lr}")]);
            }
            _ if kind.starts_with("LLHQ_CLB") => {
                let lr = if col == chip.col_q(DirH::W) {
                    'L'
                } else if col == chip.col_q(DirH::E) {
                    'R'
                } else {
                    unreachable!()
                };
                let naming = if chip.kind != ChipKind::Xc4000Xv {
                    "LLHQ_CLB"
                } else if row >= chip.row_q(DirV::S) && row < chip.row_q(DirV::N) {
                    "LLHQ_CLB_I"
                } else {
                    "LLHQ_CLB_O"
                };
                let ntile = ngrid.name_tile(tcrd, naming, [format!("VQ{lr}R{r}")]);
                if chip.kind == ChipKind::Xc4000Xla {
                    ntile.add_bel(
                        bslots::PULLUP_TBUF_W[0],
                        format!("PULLUP_R{r}C{cc}.4", cc = c - 1),
                    );
                    ntile.add_bel(bslots::PULLUP_TBUF_E[0], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(
                        bslots::PULLUP_TBUF_W[1],
                        format!("PULLUP_R{r}C{cc}.3", cc = c - 1),
                    );
                    ntile.add_bel(bslots::PULLUP_TBUF_E[1], format!("PULLUP_R{r}C{c}.1"));
                } else {
                    ntile.add_bel(bslots::PULLUP_TBUF_W[0], format!("PULLUP_R{r}C{c}.4"));
                    ntile.add_bel(bslots::PULLUP_TBUF_E[0], format!("PULLUP_R{r}C{c}.2"));
                    ntile.add_bel(bslots::PULLUP_TBUF_W[1], format!("PULLUP_R{r}C{c}.3"));
                    ntile.add_bel(bslots::PULLUP_TBUF_E[1], format!("PULLUP_R{r}C{c}.1"));
                }
            }
            tcls::LLHC_IO_S => {
                let ntile = ngrid.name_tile(tcrd, "LLHC_IO_S", ["BM".into()]);
                ntile.add_bel(bslots::PULLUP_DEC_W[0], format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bslots::PULLUP_DEC_E[0], format!("PULLUP_R{r}C{c}.5"));
                ntile.add_bel(bslots::PULLUP_DEC_W[1], format!("PULLUP_R{r}C{c}.3"));
                ntile.add_bel(bslots::PULLUP_DEC_E[1], format!("PULLUP_R{r}C{c}.6"));
                ntile.add_bel(bslots::PULLUP_DEC_W[2], format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bslots::PULLUP_DEC_E[2], format!("PULLUP_R{r}C{c}.7"));
                ntile.add_bel(bslots::PULLUP_DEC_W[3], format!("PULLUP_R{r}C{c}.1"));
                ntile.add_bel(bslots::PULLUP_DEC_E[3], format!("PULLUP_R{r}C{c}.8"));
            }
            tcls::LLHC_IO_N => {
                let ntile = ngrid.name_tile(tcrd, "LLHC_IO_N", ["TM".into()]);
                ntile.add_bel(bslots::PULLUP_DEC_W[0], format!("PULLUP_R{r}C{c}.1"));
                ntile.add_bel(bslots::PULLUP_DEC_E[0], format!("PULLUP_R{r}C{c}.8"));
                ntile.add_bel(bslots::PULLUP_DEC_W[1], format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bslots::PULLUP_DEC_E[1], format!("PULLUP_R{r}C{c}.7"));
                ntile.add_bel(bslots::PULLUP_DEC_W[2], format!("PULLUP_R{r}C{c}.3"));
                ntile.add_bel(bslots::PULLUP_DEC_E[2], format!("PULLUP_R{r}C{c}.6"));
                ntile.add_bel(bslots::PULLUP_DEC_W[3], format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bslots::PULLUP_DEC_E[3], format!("PULLUP_R{r}C{c}.5"));
            }
            _ if kind.starts_with("LLHC_CLB") => {
                let naming = if row >= chip.row_q(DirV::S) && row < chip.row_q(DirV::N) {
                    "LLHC_CLB_I"
                } else {
                    "LLHC_CLB_O"
                };
                let ntile = ngrid.name_tile(tcrd, naming, [format!("VMR{r}")]);
                ntile.add_bel(bslots::PULLUP_TBUF_W[0], format!("PULLUP_R{r}C{c}.2"));
                ntile.add_bel(bslots::PULLUP_TBUF_E[0], format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bslots::PULLUP_TBUF_W[1], format!("PULLUP_R{r}C{c}.1"));
                ntile.add_bel(bslots::PULLUP_TBUF_E[1], format!("PULLUP_R{r}C{c}.3"));
            }
            tcls::LLVQ_IO_SW | tcls::LLVQ_IO_NW => {
                let bt = if row == chip.row_q(DirV::S) {
                    'B'
                } else if row == chip.row_q(DirV::N) {
                    'T'
                } else {
                    unreachable!()
                };
                let ntile = ngrid.name_tile(tcrd, kind, [format!("LQ{bt}")]);
                let sn = if bt == 'B' { 'S' } else { 'N' };
                ntile.add_bel(bslots::BUFF, format!("BUFF_{sn}W"));
            }
            tcls::LLVQ_IO_SE | tcls::LLVQ_IO_NE => {
                let bt = if row == chip.row_q(DirV::S) {
                    'B'
                } else if row == chip.row_q(DirV::N) {
                    'T'
                } else {
                    unreachable!()
                };
                let naming = if chip.is_buff_large {
                    if bt == 'B' {
                        "LLVQ_IO_SE_L"
                    } else {
                        "LLVQ_IO_NE_L"
                    }
                } else {
                    if bt == 'B' {
                        "LLVQ_IO_SE_S"
                    } else {
                        "LLVQ_IO_NE_S"
                    }
                };
                let ntile = ngrid.name_tile(tcrd, naming, [format!("RQ{bt}")]);
                let sn = if bt == 'B' { 'S' } else { 'N' };
                ntile.add_bel(bslots::BUFF, format!("BUFF_{sn}E"));
            }
            tcls::LLVQ_CLB => {
                let bt = if row == chip.row_q(DirV::S) {
                    'B'
                } else if row == chip.row_q(DirV::N) {
                    'T'
                } else {
                    unreachable!()
                };
                ngrid.name_tile(tcrd, "LLVQ_CLB", [format!("HQ{bt}C{c}")]);
            }
            tcls::LLVC_IO_W => {
                let ntile = ngrid.name_tile(tcrd, "LLVC_IO_W", ["LM".into()]);
                let r = r + 1;
                ntile.add_bel(bslots::PULLUP_DEC_S[0], format!("PULLUP_R{r}C{c}.10"));
                ntile.add_bel(bslots::PULLUP_DEC_N[0], format!("PULLUP_R{r}C{c}.3"));
                ntile.add_bel(bslots::PULLUP_DEC_S[1], format!("PULLUP_R{r}C{c}.9"));
                ntile.add_bel(bslots::PULLUP_DEC_N[1], format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bslots::PULLUP_DEC_S[2], format!("PULLUP_R{r}C{c}.8"));
                ntile.add_bel(bslots::PULLUP_DEC_N[2], format!("PULLUP_R{r}C{c}.5"));
                ntile.add_bel(bslots::PULLUP_DEC_S[3], format!("PULLUP_R{r}C{c}.7"));
                ntile.add_bel(bslots::PULLUP_DEC_N[3], format!("PULLUP_R{r}C{c}.6"));
            }
            tcls::LLVC_IO_E => {
                let ntile = ngrid.name_tile(tcrd, "LLVC_IO_E", ["RM".into()]);
                let r = r + 1;
                ntile.add_bel(bslots::PULLUP_DEC_S[0], format!("PULLUP_R{r}C{c}.10"));
                ntile.add_bel(bslots::PULLUP_DEC_N[0], format!("PULLUP_R{r}C{c}.3"));
                ntile.add_bel(bslots::PULLUP_DEC_S[1], format!("PULLUP_R{r}C{c}.9"));
                ntile.add_bel(bslots::PULLUP_DEC_N[1], format!("PULLUP_R{r}C{c}.4"));
                ntile.add_bel(bslots::PULLUP_DEC_S[2], format!("PULLUP_R{r}C{c}.8"));
                ntile.add_bel(bslots::PULLUP_DEC_N[2], format!("PULLUP_R{r}C{c}.5"));
                ntile.add_bel(bslots::PULLUP_DEC_S[3], format!("PULLUP_R{r}C{c}.7"));
                ntile.add_bel(bslots::PULLUP_DEC_N[3], format!("PULLUP_R{r}C{c}.6"));
            }
            tcls::LLVC_CLB => {
                ngrid.name_tile(tcrd, "LLVC_CLB", [format!("HMC{c}")]);
            }

            tcls::CLKQ => {
                let (bt, sn) = if row == chip.row_q(DirV::S) {
                    ('B', DirV::S)
                } else if row == chip.row_q(DirV::N) {
                    ('T', DirV::N)
                } else {
                    unreachable!()
                };
                let lr = if col == chip.col_q(DirH::W) {
                    'L'
                } else if col == chip.col_q(DirH::E) {
                    'R'
                } else {
                    unreachable!()
                };

                ngrid.name_tile(tcrd, &format!("CLKQ_{sn}"), [format!("Q{bt}{lr}")]);
            }
            tcls::CLKQC => {
                let (bt, sn) = if row == chip.row_q(DirV::S) {
                    ('B', DirV::S)
                } else if row == chip.row_q(DirV::N) {
                    ('T', DirV::N)
                } else {
                    unreachable!()
                };
                ngrid.name_tile(tcrd, &format!("CLKQC_{sn}"), [format!("VMQ{bt}")]);
            }

            _ => unreachable!(),
        }
    }
    ExpandedNamedDevice { edev, ngrid, chip }
}
