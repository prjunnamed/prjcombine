use std::fmt::Write;

use prjcombine_interconnect::grid::{ColId, RowId};
use prjcombine_re_xilinx_naming::{
    db::{NamingDb, NodeRawTileId},
    grid::ExpandedGridNaming,
};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
};
use unnamed_entity::EntityId;

use crate::ExpandedNamedDevice;

fn get_tile_kind(chip: &Chip, col: ColId, row: RowId) -> &'static str {
    if col == chip.col_lio() {
        if row == chip.row_bio() {
            "LL"
        } else if row == chip.row_tio() {
            "UL"
        } else if row == chip.row_bio() + 1 {
            "LEFTSB"
        } else if row == chip.row_tio() - 1 {
            "LEFTT"
        } else if chip.kind.is_xl() && row == chip.row_qb() {
            if row.to_idx() % 2 == 0 {
                "LEFTF"
            } else {
                "LEFTSF"
            }
        } else if chip.kind.is_xl() && row == chip.row_qt() - 1 {
            if row.to_idx() % 2 == 0 {
                "LEFTF1"
            } else {
                "LEFTSF1"
            }
        } else if row.to_idx() % 2 == 0 {
            "LEFT"
        } else {
            "LEFTS"
        }
    } else if col == chip.col_rio() {
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
        if row == chip.row_bio() {
            "LR"
        } else if row == chip.row_tio() {
            "UR"
        } else if row == chip.row_bio() + 1 {
            "RTSB"
        } else if row == chip.row_tio() - 1 {
            "RTT"
        } else if chip.kind.is_xl() && row == row_f {
            if row.to_idx() % 2 == 0 { "RTF" } else { "RTSF" }
        } else if chip.kind.is_xl() && row == row_f1 {
            if row.to_idx() % 2 == 0 {
                "RTF1"
            } else {
                "RTSF1"
            }
        } else if row.to_idx() % 2 == 0 {
            "RT"
        } else {
            "RTS"
        }
    } else if row == chip.row_bio() {
        if col == chip.col_lio() + 1 {
            "BOTSL"
        } else if col == chip.col_rio() - 1 {
            "BOTRR"
        } else if col.to_idx() % 2 == 0 {
            "BOT"
        } else {
            "BOTS"
        }
    } else if row == chip.row_tio() {
        if col == chip.col_lio() + 1 {
            "TOPSL"
        } else if col == chip.col_rio() - 1 {
            "TOPRR"
        } else if col.to_idx() % 2 == 0 {
            "TOP"
        } else {
            "TOPS"
        }
    } else {
        "CENTER"
    }
}

fn get_tile_name(chip: &Chip, col: ColId, row: RowId) -> String {
    let r = chip.row_tio().to_idx() - row.to_idx();
    let c = col.to_idx();
    if col == chip.col_lio() {
        if row == chip.row_bio() {
            "BL".into()
        } else if row == chip.row_tio() {
            "TL".into()
        } else {
            format!("LR{r}")
        }
    } else if col == chip.col_rio() {
        if row == chip.row_bio() {
            "BR".into()
        } else if row == chip.row_tio() {
            "TR".into()
        } else {
            format!("RR{r}")
        }
    } else {
        if row == chip.row_bio() {
            format!("BC{c}")
        } else if row == chip.row_tio() {
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
    for die in egrid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    let c = col.to_idx();
                    let r = chip.row_tio() - row;
                    match &kind[..] {
                        "CNR.BL" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                "CNR.BL",
                                [
                                    get_tile_name(chip, col, row),
                                    get_tile_name(chip, col + 1, row),
                                ],
                            );
                            if chip.kind == ChipKind::SpartanXl {
                                nnode.add_bel(0, "BUFGLS_SSW".to_string());
                                nnode.add_bel(1, "BUFGLS_WSW".to_string());
                                nnode.add_bel(2, "RDBK".to_string());
                            } else if chip.kind.is_xl() {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.8"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.7"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.6"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.5"));
                                nnode.add_bel(4, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(5, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(6, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(7, format!("PULLUP_R{r}C{c}.1"));
                                nnode.add_bel(8, "BUFG_SSW".to_string());
                                nnode.add_bel(9, "BUFG_WSW".to_string());
                                nnode.add_bel(10, "BUFGE_SSW".to_string());
                                nnode.add_bel(11, "BUFGE_WSW".to_string());
                                nnode.add_bel(12, "BUFGLS_SSW".to_string());
                                nnode.add_bel(13, "BUFGLS_WSW".to_string());
                                nnode.add_bel(14, "MD0".to_string());
                                nnode.add_bel(15, "MD1".to_string());
                                nnode.add_bel(16, "MD2".to_string());
                                nnode.add_bel(17, "RDBK".to_string());
                            } else {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.8"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.7"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.6"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.5"));
                                nnode.add_bel(4, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(5, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(6, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(7, format!("PULLUP_R{r}C{c}.1"));
                                nnode.add_bel(8, "BUFGP_BL".to_string());
                                nnode.add_bel(9, "BUFGS_BL".to_string());
                                nnode.add_bel(10, "CI_BL".to_string());
                                nnode.add_bel(11, "MD0".to_string());
                                nnode.add_bel(12, "MD1".to_string());
                                nnode.add_bel(13, "MD2".to_string());
                                nnode.add_bel(14, "RDBK".to_string());
                            }
                        }
                        "CNR.TL" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                "CNR.TL",
                                [
                                    get_tile_name(chip, col, row),
                                    get_tile_name(chip, col + 1, row),
                                    get_tile_name(chip, col, row - 1),
                                    get_tile_name(chip, col + 1, row - 1),
                                ],
                            );
                            if chip.kind == ChipKind::SpartanXl {
                                nnode.add_bel(0, "BUFGLS_NNW".to_string());
                                nnode.add_bel(1, "BUFGLS_WNW".to_string());
                                nnode.add_bel(2, "BSCAN".to_string());
                            } else if chip.kind.is_xl() {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(4, format!("PULLUP_R{r}C{c}.5"));
                                nnode.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                                nnode.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                                nnode.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                                nnode.add_bel(8, "BUFG_NNW".to_string());
                                nnode.add_bel(9, "BUFG_WNW".to_string());
                                nnode.add_bel(10, "BUFGE_NNW".to_string());
                                nnode.add_bel(11, "BUFGE_WNW".to_string());
                                nnode.add_bel(12, "BUFGLS_NNW".to_string());
                                nnode.add_bel(13, "BUFGLS_WNW".to_string());
                                nnode.add_bel(14, "BSCAN".to_string());
                            } else {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(4, format!("PULLUP_R{r}C{c}.5"));
                                nnode.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                                nnode.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                                nnode.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                                nnode.add_bel(8, "BUFGS_TL".to_string());
                                nnode.add_bel(9, "BUFGP_TL".to_string());
                                nnode.add_bel(10, "CI_TL".to_string());
                                nnode.add_bel(11, "BSCAN".to_string());
                            }
                        }
                        "CNR.BR" => {
                            let nnode =
                                ngrid.name_node(nloc, "CNR.BR", [get_tile_name(chip, col, row)]);
                            if chip.kind == ChipKind::SpartanXl {
                                nnode.add_bel(0, "BUFGLS_SSE".to_string());
                                nnode.add_bel(1, "BUFGLS_ESE".to_string());
                                nnode.add_bel(2, "STARTUP".to_string());
                                nnode.add_bel(3, "RDCLK".to_string());
                            } else if chip.kind.is_xl() {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.8"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.7"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.6"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.5"));
                                nnode.add_bel(4, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(5, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(6, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(7, format!("PULLUP_R{r}C{c}.1"));
                                nnode.add_bel(8, "BUFG_SSE".to_string());
                                nnode.add_bel(9, "BUFG_ESE".to_string());
                                nnode.add_bel(10, "BUFGE_SSE".to_string());
                                nnode.add_bel(11, "BUFGE_ESE".to_string());
                                nnode.add_bel(12, "BUFGLS_SSE".to_string());
                                nnode.add_bel(13, "BUFGLS_ESE".to_string());
                                nnode.add_bel(14, "STARTUP".to_string());
                                nnode.add_bel(15, "RDCLK".to_string());
                            } else {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.8"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.7"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.6"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.5"));
                                nnode.add_bel(4, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(5, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(6, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(7, format!("PULLUP_R{r}C{c}.1"));
                                nnode.add_bel(8, "BUFGS_BR".to_string());
                                nnode.add_bel(9, "BUFGP_BR".to_string());
                                nnode.add_bel(10, "CO_BR".to_string());
                                nnode.add_bel(11, "STARTUP".to_string());
                                nnode.add_bel(12, "RDCLK".to_string());
                            }
                            nnode.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                        }
                        "CNR.TR" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                "CNR.TR",
                                [
                                    get_tile_name(chip, col, row),
                                    get_tile_name(chip, col, row - 1),
                                ],
                            );
                            if chip.kind == ChipKind::SpartanXl {
                                nnode.add_bel(0, "BUFGLS_NNE".to_string());
                                nnode.add_bel(1, "BUFGLS_ENE".to_string());
                                nnode.add_bel(2, "UPDATE".to_string());
                                nnode.add_bel(3, "OSC".to_string());
                                nnode.add_bel(4, "TDO".to_string());
                            } else if chip.kind.is_xl() {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(4, format!("PULLUP_R{r}C{c}.5"));
                                nnode.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                                nnode.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                                nnode.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                                nnode.add_bel(8, "BUFG_NNE".to_string());
                                nnode.add_bel(9, "BUFG_ENE".to_string());
                                nnode.add_bel(10, "BUFGE_NNE".to_string());
                                nnode.add_bel(11, "BUFGE_ENE".to_string());
                                nnode.add_bel(12, "BUFGLS_NNE".to_string());
                                nnode.add_bel(13, "BUFGLS_ENE".to_string());
                                nnode.add_bel(14, "UPDATE".to_string());
                                nnode.add_bel(15, "OSC".to_string());
                                nnode.add_bel(16, "TDO".to_string());
                            } else {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(4, format!("PULLUP_R{r}C{c}.5"));
                                nnode.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                                nnode.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                                nnode.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                                nnode.add_bel(8, "BUFGP_TR".to_string());
                                nnode.add_bel(9, "BUFGS_TR".to_string());
                                nnode.add_bel(10, "CO_TR".to_string());
                                nnode.add_bel(11, "UPDATE".to_string());
                                nnode.add_bel(12, "OSC".to_string());
                                nnode.add_bel(13, "TDO".to_string());
                            }
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
                            let nnode = ngrid.name_node(nloc, &format!("{kind}.{kind_s}"), names);
                            let p = (chip.columns - 2) * 4
                                + (chip.rows - 2) * 2
                                + (row.to_idx() - 1) * 2
                                + 1;
                            nnode.add_bel(0, format!("PAD{}", p + 1));
                            nnode.add_bel(1, format!("PAD{p}"));
                            nnode.add_bel(2, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(3, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(4, format!("PULLUP_R{r}C{c}.2"));
                            nnode.add_bel(5, format!("PULLUP_R{r}C{c}.1"));
                            if chip.kind != ChipKind::SpartanXl {
                                nnode.add_bel(6, format!("DEC_R{r}C{c}.1"));
                                nnode.add_bel(7, format!("DEC_R{r}C{c}.2"));
                                nnode.add_bel(8, format!("DEC_R{r}C{c}.3"));
                            }
                            if chip.kind == ChipKind::Xc4000Xv {
                                nnode.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                                nnode.tie_rt = NodeRawTileId::from_idx(4);
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
                            let nnode = ngrid.name_node(nloc, &format!("{kind}.{kind_s}"), names);
                            let p = (chip.columns - 2) * 2
                                + (chip.row_tio().to_idx() - row.to_idx() - 1) * 2
                                + 1;
                            nnode.add_bel(0, format!("PAD{p}"));
                            nnode.add_bel(1, format!("PAD{}", p + 1));
                            nnode.add_bel(2, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(3, format!("TBUF_R{r}C{c}.1"));
                            nnode.add_bel(4, format!("PULLUP_R{r}C{c}.2"));
                            nnode.add_bel(5, format!("PULLUP_R{r}C{c}.1"));
                            if chip.kind != ChipKind::SpartanXl {
                                nnode.add_bel(6, format!("DEC_R{r}C{c}.1"));
                                nnode.add_bel(7, format!("DEC_R{r}C{c}.2"));
                                nnode.add_bel(8, format!("DEC_R{r}C{c}.3"));
                            }

                            nnode.tie_name = Some(format!("TIE_R{r}C{c}.1"));
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
                            let nnode = ngrid.name_node(nloc, &format!("{kind}.{kind_e}"), names);
                            let p = (chip.columns - 2) * 2
                                + (chip.rows - 2) * 2
                                + (chip.col_rio().to_idx() - col.to_idx() - 1) * 2
                                + 1;

                            nnode.add_bel(0, format!("PAD{}", p + 1));
                            nnode.add_bel(1, format!("PAD{p}"));
                            if chip.kind != ChipKind::SpartanXl {
                                nnode.add_bel(2, format!("DEC_R{r}C{c}.1"));
                                nnode.add_bel(3, format!("DEC_R{r}C{c}.2"));
                                nnode.add_bel(4, format!("DEC_R{r}C{c}.3"));
                            }
                            nnode.tie_name = Some(format!("TIE_R{r}C{c}.1"));
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
                            let nnode = ngrid.name_node(nloc, &format!("{kind}.{kind_e}"), names);
                            let p = (col.to_idx() - 1) * 2 + 1;
                            nnode.add_bel(0, format!("PAD{p}"));
                            nnode.add_bel(1, format!("PAD{}", p + 1));
                            if chip.kind != ChipKind::SpartanXl {
                                nnode.add_bel(2, format!("DEC_R{r}C{c}.1"));
                                nnode.add_bel(3, format!("DEC_R{r}C{c}.2"));
                                nnode.add_bel(4, format!("DEC_R{r}C{c}.3"));
                            }
                            if chip.kind == ChipKind::Xc4000Xv {
                                nnode.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                                nnode.tie_rt = NodeRawTileId::from_idx(3);
                            }
                        }
                        _ if kind.starts_with("CLB") => {
                            let mut naming = "CLB".to_string();
                            if row == chip.row_tio() - 1 {
                                let kind_n = get_tile_kind(chip, col, row + 1);
                                write!(naming, ".{kind_n}").unwrap();
                            }
                            if col == chip.col_rio() - 1 {
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
                            let nnode = ngrid.name_node(nloc, &naming, names);
                            nnode.add_bel(0, format!("CLB_R{r}C{c}"));
                            nnode.add_bel(1, format!("TBUF_R{r}C{c}.2"));
                            nnode.add_bel(2, format!("TBUF_R{r}C{c}.1"));
                            nnode.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                        }

                        "LLH.IO.B" => {
                            ngrid.name_node(nloc, "LLH.IO.B", ["BM".into()]);
                        }
                        "LLH.IO.T" => {
                            ngrid.name_node(nloc, "LLH.IO.T", ["TM".into()]);
                        }
                        _ if kind.starts_with("LLH.CLB") => {
                            let naming = if row < chip.row_mid() {
                                "LLH.CLB.B"
                            } else {
                                "LLH.CLB.T"
                            };
                            ngrid.name_node(nloc, naming, [format!("VMR{r}")]);
                        }
                        "LLV.IO.L" => {
                            ngrid.name_node(nloc, "LLV.IO.L", ["LM".into()]);
                        }
                        "LLV.IO.R" => {
                            ngrid.name_node(nloc, "LLV.IO.R", ["RM".into()]);
                        }
                        "LLV.CLB" => {
                            ngrid.name_node(nloc, "LLV.CLB", [format!("HMC{c}")]);
                        }

                        "LLHQ.IO.B" => {
                            let lr = if col == chip.col_ql() {
                                'L'
                            } else if col == chip.col_qr() {
                                'R'
                            } else {
                                unreachable!()
                            };
                            ngrid.name_node(nloc, "LLHQ.IO.B", [format!("BQ{lr}")]);
                        }
                        "LLHQ.IO.T" => {
                            let lr = if col == chip.col_ql() {
                                'L'
                            } else if col == chip.col_qr() {
                                'R'
                            } else {
                                unreachable!()
                            };
                            ngrid.name_node(nloc, "LLHQ.IO.T", [format!("TQ{lr}")]);
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
                            let nnode = ngrid.name_node(nloc, naming, [format!("VQ{lr}R{r}")]);
                            if chip.kind == ChipKind::Xc4000Xla {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{cc}.4", cc = c - 1));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{cc}.3", cc = c - 1));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.1"));
                            } else {
                                nnode.add_bel(0, format!("PULLUP_R{r}C{c}.4"));
                                nnode.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                                nnode.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                                nnode.add_bel(3, format!("PULLUP_R{r}C{c}.1"));
                            }
                        }
                        "LLHC.IO.B" => {
                            let nnode = ngrid.name_node(nloc, "LLHC.IO.B", ["BM".into()]);
                            nnode.add_bel(0, format!("PULLUP_R{r}C{c}.4"));
                            nnode.add_bel(1, format!("PULLUP_R{r}C{c}.5"));
                            nnode.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                            nnode.add_bel(3, format!("PULLUP_R{r}C{c}.6"));
                            nnode.add_bel(4, format!("PULLUP_R{r}C{c}.2"));
                            nnode.add_bel(5, format!("PULLUP_R{r}C{c}.7"));
                            nnode.add_bel(6, format!("PULLUP_R{r}C{c}.1"));
                            nnode.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                        }
                        "LLHC.IO.T" => {
                            let nnode = ngrid.name_node(nloc, "LLHC.IO.T", ["TM".into()]);
                            nnode.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                            nnode.add_bel(1, format!("PULLUP_R{r}C{c}.8"));
                            nnode.add_bel(2, format!("PULLUP_R{r}C{c}.2"));
                            nnode.add_bel(3, format!("PULLUP_R{r}C{c}.7"));
                            nnode.add_bel(4, format!("PULLUP_R{r}C{c}.3"));
                            nnode.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                            nnode.add_bel(6, format!("PULLUP_R{r}C{c}.4"));
                            nnode.add_bel(7, format!("PULLUP_R{r}C{c}.5"));
                        }
                        _ if kind.starts_with("LLHC.CLB") => {
                            let naming = if row >= chip.row_qb() && row < chip.row_qt() {
                                "LLHC.CLB.I"
                            } else {
                                "LLHC.CLB.O"
                            };
                            let nnode = ngrid.name_node(nloc, naming, [format!("VMR{r}")]);
                            nnode.add_bel(0, format!("PULLUP_R{r}C{c}.2"));
                            nnode.add_bel(1, format!("PULLUP_R{r}C{c}.4"));
                            nnode.add_bel(2, format!("PULLUP_R{r}C{c}.1"));
                            nnode.add_bel(3, format!("PULLUP_R{r}C{c}.3"));
                        }
                        _ if kind.starts_with("LLVQ.IO.L") => {
                            let bt = if row == chip.row_qb() {
                                'B'
                            } else if row == chip.row_qt() {
                                'T'
                            } else {
                                unreachable!()
                            };
                            let nnode = ngrid.name_node(nloc, kind, [format!("LQ{bt}")]);
                            let sn = if bt == 'B' { 'S' } else { 'N' };
                            nnode.add_bel(0, format!("BUFF_{sn}W"));
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
                            let nnode = ngrid.name_node(nloc, naming, [format!("RQ{bt}")]);
                            let sn = if bt == 'B' { 'S' } else { 'N' };
                            nnode.add_bel(0, format!("BUFF_{sn}E"));
                        }
                        "LLVQ.CLB" => {
                            let bt = if row == chip.row_qb() {
                                'B'
                            } else if row == chip.row_qt() {
                                'T'
                            } else {
                                unreachable!()
                            };
                            ngrid.name_node(nloc, "LLVQ.CLB", [format!("HQ{bt}C{c}")]);
                        }
                        "LLVC.IO.L" => {
                            let nnode = ngrid.name_node(nloc, "LLVC.IO.L", ["LM".into()]);
                            let r = r + 1;
                            nnode.add_bel(0, format!("PULLUP_R{r}C{c}.10"));
                            nnode.add_bel(1, format!("PULLUP_R{r}C{c}.3"));
                            nnode.add_bel(2, format!("PULLUP_R{r}C{c}.9"));
                            nnode.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                            nnode.add_bel(4, format!("PULLUP_R{r}C{c}.8"));
                            nnode.add_bel(5, format!("PULLUP_R{r}C{c}.5"));
                            nnode.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                            nnode.add_bel(7, format!("PULLUP_R{r}C{c}.6"));
                        }
                        "LLVC.IO.R" => {
                            let nnode = ngrid.name_node(nloc, "LLVC.IO.R", ["RM".into()]);
                            let r = r + 1;
                            nnode.add_bel(0, format!("PULLUP_R{r}C{c}.10"));
                            nnode.add_bel(1, format!("PULLUP_R{r}C{c}.3"));
                            nnode.add_bel(2, format!("PULLUP_R{r}C{c}.9"));
                            nnode.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                            nnode.add_bel(4, format!("PULLUP_R{r}C{c}.8"));
                            nnode.add_bel(5, format!("PULLUP_R{r}C{c}.5"));
                            nnode.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                            nnode.add_bel(7, format!("PULLUP_R{r}C{c}.6"));
                        }
                        "LLVC.CLB" => {
                            ngrid.name_node(nloc, "LLVC.CLB", [format!("HMC{c}")]);
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

                            ngrid.name_node(nloc, &format!("CLKQ.{bt}"), [format!("Q{bt}{lr}")]);
                        }
                        "CLKC" => {
                            ngrid.name_node(nloc, "CLKC", ["M".into()]);
                        }
                        "CLKQC" => {
                            let bt = if row == chip.row_qb() {
                                'B'
                            } else if row == chip.row_qt() {
                                'T'
                            } else {
                                unreachable!()
                            };
                            ngrid.name_node(nloc, &format!("CLKQC.{bt}"), [format!("VMQ{bt}")]);
                        }

                        _ => unreachable!(),
                    }
                }
            }
        }
    }
    ExpandedNamedDevice { edev, ngrid, chip }
}
