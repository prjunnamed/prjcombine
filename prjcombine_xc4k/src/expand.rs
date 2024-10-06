use std::fmt::Write;

use prjcombine_int::{
    db::{BelId, IntDb, NodeRawTileId},
    grid::{ColId, DieId, ExpandedGrid, RowId},
};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use unnamed_entity::{EntityId, EntityVec};

use crate::{
    expanded::{ExpandedDevice, Io},
    grid::{Grid, GridKind, IoCoord, TileIobId},
};

impl Grid {
    fn get_bio_kind(&self, col: ColId) -> &'static str {
        if col == self.col_lio() + 1 {
            "BOTSL"
        } else if col == self.col_rio() - 1 {
            "BOTRR"
        } else if col == self.col_rio() {
            "LR"
        } else if col.to_idx() % 2 == 0 {
            "BOT"
        } else {
            "BOTS"
        }
    }

    fn get_tio_kind(&self, col: ColId) -> &'static str {
        if col == self.col_lio() + 1 {
            "TOPSL"
        } else if col == self.col_rio() - 1 {
            "TOPRR"
        } else if col == self.col_rio() {
            "UR"
        } else if col.to_idx() % 2 == 0 {
            "TOP"
        } else {
            "TOPS"
        }
    }

    fn get_lio_kind(&self, row: RowId) -> &'static str {
        if row == self.row_bio() + 1 {
            "LEFTSB"
        } else if row == self.row_bio() {
            "LL"
        } else if row == self.row_tio() - 1 {
            "LEFTT"
        } else if self.kind.is_xl() && row == self.row_qb() {
            if row.to_idx() % 2 == 0 {
                "LEFTF"
            } else {
                "LEFTSF"
            }
        } else if self.kind.is_xl() && row == self.row_qt() - 1 {
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
    }

    fn get_rio_kind(&self, row: RowId) -> &'static str {
        let row_f = if self.is_buff_large {
            self.row_qb() + 1
        } else {
            self.row_qb()
        };
        let row_f1 = if self.is_buff_large {
            self.row_qt() - 2
        } else {
            self.row_qt() - 1
        };
        if row == self.row_bio() + 1 {
            "RTSB"
        } else if row == self.row_bio() {
            "LR"
        } else if row == self.row_tio() - 1 {
            "RTT"
        } else if self.kind.is_xl() && row == row_f {
            if row.to_idx() % 2 == 0 {
                "RTF"
            } else {
                "RTSF"
            }
        } else if self.kind.is_xl() && row == row_f1 {
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
    }

    fn get_tile_name(&self, col: ColId, row: RowId) -> String {
        let r = self.row_tio().to_idx() - row.to_idx();
        let c = col.to_idx();
        if col == self.col_lio() {
            if row == self.row_bio() {
                "BL".into()
            } else if row == self.row_tio() {
                "TL".into()
            } else {
                format!("LR{r}")
            }
        } else if col == self.col_rio() {
            if row == self.row_bio() {
                "BR".into()
            } else if row == self.row_tio() {
                "TR".into()
            } else {
                format!("RR{r}")
            }
        } else {
            if row == self.row_bio() {
                format!("BC{c}")
            } else if row == self.row_tio() {
                format!("TC{c}")
            } else {
                format!("R{r}C{c}")
            }
        }
    }

    pub fn expand_grid<'a>(&'a self, db: &'a IntDb) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIE".to_string());
        egrid.tie_pin_gnd = Some("O".to_string());
        let (_, mut grid) = egrid.add_die(self.columns, self.rows);

        let col_l = grid.cols().next().unwrap();
        let col_r = grid.cols().next_back().unwrap();
        let row_b = grid.rows().next().unwrap();
        let row_t = grid.rows().next_back().unwrap();

        for col in grid.cols() {
            let c = col.to_idx();
            if col == self.col_lio() {
                for row in grid.rows() {
                    let name = self.get_tile_name(col, row);
                    let r = row_t.to_idx() - row.to_idx();
                    if row == self.row_bio() {
                        let node = grid.add_xnode(
                            (col, row),
                            db.get_node("LL"),
                            &[&name, &self.get_tile_name(col + 1, row)],
                            db.get_node_naming("LL"),
                            &[(col, row), (col + 1, row)],
                        );
                        if self.kind == GridKind::SpartanXl {
                            node.add_bel(0, "BUFGLS_SSW".to_string());
                            node.add_bel(1, "BUFGLS_WSW".to_string());
                            node.add_bel(2, "RDBK".to_string());
                        } else if self.kind.is_xl() {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.8"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.7"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.6"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.5"));
                            node.add_bel(4, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(5, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(6, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(7, format!("PULLUP_R{r}C{c}.1"));
                            node.add_bel(8, "BUFG_SSW".to_string());
                            node.add_bel(9, "BUFG_WSW".to_string());
                            node.add_bel(10, "BUFGE_SSW".to_string());
                            node.add_bel(11, "BUFGE_WSW".to_string());
                            node.add_bel(12, "BUFGLS_SSW".to_string());
                            node.add_bel(13, "BUFGLS_WSW".to_string());
                            node.add_bel(14, "MD0".to_string());
                            node.add_bel(15, "MD1".to_string());
                            node.add_bel(16, "MD2".to_string());
                            node.add_bel(17, "RDBK".to_string());
                        } else {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.8"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.7"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.6"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.5"));
                            node.add_bel(4, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(5, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(6, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(7, format!("PULLUP_R{r}C{c}.1"));
                            node.add_bel(8, "BUFGP_BL".to_string());
                            node.add_bel(9, "BUFGS_BL".to_string());
                            node.add_bel(10, "CI_BL".to_string());
                            node.add_bel(11, "MD0".to_string());
                            node.add_bel(12, "MD1".to_string());
                            node.add_bel(13, "MD2".to_string());
                            node.add_bel(14, "RDBK".to_string());
                        }
                    } else if row == self.row_tio() {
                        let node = grid.add_xnode(
                            (col, row),
                            db.get_node("UL"),
                            &[
                                &name,
                                &self.get_tile_name(col + 1, row),
                                &self.get_tile_name(col, row - 1),
                                &self.get_tile_name(col + 1, row - 1),
                            ],
                            db.get_node_naming("UL"),
                            &[
                                (col, row),
                                (col + 1, row),
                                (col, row - 1),
                                (col + 1, row - 1),
                            ],
                        );
                        if self.kind == GridKind::SpartanXl {
                            node.add_bel(0, "BUFGLS_NNW".to_string());
                            node.add_bel(1, "BUFGLS_WNW".to_string());
                            node.add_bel(2, "BSCAN".to_string());
                        } else if self.kind.is_xl() {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(4, format!("PULLUP_R{r}C{c}.5"));
                            node.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                            node.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                            node.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                            node.add_bel(8, "BUFG_NNW".to_string());
                            node.add_bel(9, "BUFG_WNW".to_string());
                            node.add_bel(10, "BUFGE_NNW".to_string());
                            node.add_bel(11, "BUFGE_WNW".to_string());
                            node.add_bel(12, "BUFGLS_NNW".to_string());
                            node.add_bel(13, "BUFGLS_WNW".to_string());
                            node.add_bel(14, "BSCAN".to_string());
                        } else {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(4, format!("PULLUP_R{r}C{c}.5"));
                            node.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                            node.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                            node.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                            node.add_bel(8, "BUFGS_TL".to_string());
                            node.add_bel(9, "BUFGP_TL".to_string());
                            node.add_bel(10, "CI_TL".to_string());
                            node.add_bel(11, "BSCAN".to_string());
                        }
                    } else {
                        let kind = self.get_lio_kind(row);
                        let kind_s = self.get_lio_kind(row - 1);
                        let mut names = vec![
                            self.get_tile_name(col, row),
                            self.get_tile_name(col, row - 1),
                            self.get_tile_name(col + 1, row),
                        ];
                        if self.kind == GridKind::Xc4000Xv {
                            names.push(format!("LHIR{r}"));
                        }
                        let names_ref: Vec<&str> = names.iter().map(|x| &**x).collect();
                        let node = grid.add_xnode(
                            (col, row),
                            db.get_node(kind),
                            &names_ref,
                            db.get_node_naming(&format!("{kind}.{kind_s}")),
                            &[(col, row), (col, row - 1), (col + 1, row)],
                        );
                        let p = (self.columns - 2) * 4
                            + (self.rows - 2) * 2
                            + (row.to_idx() - 1) * 2
                            + 1;
                        node.add_bel(0, format!("PAD{}", p + 1));
                        node.add_bel(1, format!("PAD{p}"));
                        node.add_bel(2, format!("TBUF_R{r}C{c}.2"));
                        node.add_bel(3, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(4, format!("PULLUP_R{r}C{c}.2"));
                        node.add_bel(5, format!("PULLUP_R{r}C{c}.1"));
                        if self.kind != GridKind::SpartanXl {
                            node.add_bel(6, format!("DEC_R{r}C{c}.1"));
                            node.add_bel(7, format!("DEC_R{r}C{c}.2"));
                            node.add_bel(8, format!("DEC_R{r}C{c}.3"));
                        }
                        if self.kind == GridKind::Xc4000Xv {
                            node.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                            node.tie_rt = NodeRawTileId::from_idx(3);
                        }
                    }
                }
            } else if col == self.col_rio() {
                for row in grid.rows() {
                    let name = self.get_tile_name(col, row);

                    let r = row_t.to_idx() - row.to_idx();
                    if row == self.row_bio() {
                        let node = grid.fill_tile((col, row), "LR", "LR", name);
                        if self.kind == GridKind::SpartanXl {
                            node.add_bel(0, "BUFGLS_SSE".to_string());
                            node.add_bel(1, "BUFGLS_ESE".to_string());
                            node.add_bel(2, "STARTUP".to_string());
                            node.add_bel(3, "RDCLK".to_string());
                        } else if self.kind.is_xl() {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.8"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.7"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.6"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.5"));
                            node.add_bel(4, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(5, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(6, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(7, format!("PULLUP_R{r}C{c}.1"));
                            node.add_bel(8, "BUFG_SSE".to_string());
                            node.add_bel(9, "BUFG_ESE".to_string());
                            node.add_bel(10, "BUFGE_SSE".to_string());
                            node.add_bel(11, "BUFGE_ESE".to_string());
                            node.add_bel(12, "BUFGLS_SSE".to_string());
                            node.add_bel(13, "BUFGLS_ESE".to_string());
                            node.add_bel(14, "STARTUP".to_string());
                            node.add_bel(15, "RDCLK".to_string());
                        } else {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.8"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.7"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.6"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.5"));
                            node.add_bel(4, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(5, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(6, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(7, format!("PULLUP_R{r}C{c}.1"));
                            node.add_bel(8, "BUFGS_BR".to_string());
                            node.add_bel(9, "BUFGP_BR".to_string());
                            node.add_bel(10, "CO_BR".to_string());
                            node.add_bel(11, "STARTUP".to_string());
                            node.add_bel(12, "RDCLK".to_string());
                        }
                        node.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                    } else if row == self.row_tio() {
                        let node = grid.add_xnode(
                            (col, row),
                            db.get_node("UR"),
                            &[&name, &self.get_tile_name(col, row - 1)],
                            db.get_node_naming("UR"),
                            &[(col, row), (col, row - 1)],
                        );
                        if self.kind == GridKind::SpartanXl {
                            node.add_bel(0, "BUFGLS_NNE".to_string());
                            node.add_bel(1, "BUFGLS_ENE".to_string());
                            node.add_bel(2, "UPDATE".to_string());
                            node.add_bel(3, "OSC".to_string());
                            node.add_bel(4, "TDO".to_string());
                        } else if self.kind.is_xl() {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(4, format!("PULLUP_R{r}C{c}.5"));
                            node.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                            node.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                            node.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                            node.add_bel(8, "BUFG_NNE".to_string());
                            node.add_bel(9, "BUFG_ENE".to_string());
                            node.add_bel(10, "BUFGE_NNE".to_string());
                            node.add_bel(11, "BUFGE_ENE".to_string());
                            node.add_bel(12, "BUFGLS_NNE".to_string());
                            node.add_bel(13, "BUFGLS_ENE".to_string());
                            node.add_bel(14, "UPDATE".to_string());
                            node.add_bel(15, "OSC".to_string());
                            node.add_bel(16, "TDO".to_string());
                        } else {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(4, format!("PULLUP_R{r}C{c}.5"));
                            node.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                            node.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                            node.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                            node.add_bel(8, "BUFGP_TR".to_string());
                            node.add_bel(9, "BUFGS_TR".to_string());
                            node.add_bel(10, "CO_TR".to_string());
                            node.add_bel(11, "UPDATE".to_string());
                            node.add_bel(12, "OSC".to_string());
                            node.add_bel(13, "TDO".to_string());
                        }
                    } else {
                        let kind = self.get_rio_kind(row);
                        let kind_s = self.get_rio_kind(row - 1);
                        let mut names = vec![
                            self.get_tile_name(col, row),
                            self.get_tile_name(col, row - 1),
                        ];
                        if self.kind == GridKind::Xc4000Xv {
                            names.push(format!("RHIR{r}"));
                        }
                        let names_ref: Vec<&str> = names.iter().map(|x| &**x).collect();
                        let node = grid.add_xnode(
                            (col, row),
                            db.get_node(kind),
                            &names_ref,
                            db.get_node_naming(&format!("{kind}.{kind_s}")),
                            &[(col, row), (col, row - 1)],
                        );
                        let p =
                            (self.columns - 2) * 2 + (row_t.to_idx() - row.to_idx() - 1) * 2 + 1;
                        node.add_bel(0, format!("PAD{p}"));
                        node.add_bel(1, format!("PAD{}", p + 1));
                        node.add_bel(2, format!("TBUF_R{r}C{c}.2"));
                        node.add_bel(3, format!("TBUF_R{r}C{c}.1"));
                        node.add_bel(4, format!("PULLUP_R{r}C{c}.2"));
                        node.add_bel(5, format!("PULLUP_R{r}C{c}.1"));
                        if self.kind != GridKind::SpartanXl {
                            node.add_bel(6, format!("DEC_R{r}C{c}.1"));
                            node.add_bel(7, format!("DEC_R{r}C{c}.2"));
                            node.add_bel(8, format!("DEC_R{r}C{c}.3"));
                        }

                        node.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                    }
                }
            } else {
                for row in grid.rows() {
                    let name = self.get_tile_name(col, row);

                    let r = row_t.to_idx() - row.to_idx();
                    if row == self.row_bio() {
                        let kind = self.get_bio_kind(col);
                        let kind_e = self.get_bio_kind(col + 1);
                        let mut names = vec![
                            name,
                            self.get_tile_name(col + 1, row),
                            self.get_tile_name(col, row + 1),
                        ];
                        if self.kind == GridKind::Xc4000Xv {
                            names.push(format!("BVIC{c}"));
                        }
                        let names_ref: Vec<&str> = names.iter().map(|x| &**x).collect();
                        let node = grid.add_xnode(
                            (col, row),
                            db.get_node(kind),
                            &names_ref,
                            db.get_node_naming(&format!("{kind}.{kind_e}")),
                            &[(col, row), (col + 1, row), (col, row + 1)],
                        );
                        let p = (self.columns - 2) * 2
                            + (self.rows - 2) * 2
                            + (col_r.to_idx() - col.to_idx() - 1) * 2
                            + 1;

                        node.add_bel(0, format!("PAD{}", p + 1));
                        node.add_bel(1, format!("PAD{p}"));
                        if self.kind != GridKind::SpartanXl {
                            node.add_bel(2, format!("DEC_R{r}C{c}.1"));
                            node.add_bel(3, format!("DEC_R{r}C{c}.2"));
                            node.add_bel(4, format!("DEC_R{r}C{c}.3"));
                        }
                        node.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                    } else if row == self.row_tio() {
                        let kind = self.get_tio_kind(col);
                        let kind_e = self.get_tio_kind(col + 1);
                        let mut names = vec![
                            name,
                            self.get_tile_name(col + 1, row),
                            self.get_tile_name(col, row - 1),
                            self.get_tile_name(col + 1, row - 1),
                        ];
                        if self.kind == GridKind::Xc4000Xv {
                            names.push(format!("TVIC{c}"));
                        }
                        let names_ref: Vec<&str> = names.iter().map(|x| &**x).collect();

                        let node = grid.add_xnode(
                            (col, row),
                            db.get_node(kind),
                            &names_ref,
                            db.get_node_naming(&format!("{kind}.{kind_e}")),
                            &[
                                (col, row),
                                (col + 1, row),
                                (col, row - 1),
                                (col + 1, row - 1),
                            ],
                        );
                        let p = (col.to_idx() - 1) * 2 + 1;
                        node.add_bel(0, format!("PAD{p}"));
                        node.add_bel(1, format!("PAD{}", p + 1));
                        if self.kind != GridKind::SpartanXl {
                            node.add_bel(2, format!("DEC_R{r}C{c}.1"));
                            node.add_bel(3, format!("DEC_R{r}C{c}.2"));
                            node.add_bel(4, format!("DEC_R{r}C{c}.3"));
                        }
                        if self.kind == GridKind::Xc4000Xv {
                            node.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                            node.tie_rt = NodeRawTileId::from_idx(4);
                        }
                    } else {
                        let kind = if row == self.row_bio() + 1 {
                            "CLB.B"
                        } else if row == self.row_tio() - 1 {
                            "CLB.T"
                        } else {
                            "CLB"
                        };
                        let mut naming = "CLB".to_string();
                        if row == self.row_bio() + 1 {
                            let kind_s = self.get_bio_kind(col);
                            let kind_se = self.get_bio_kind(col + 1);
                            write!(naming, ".{kind_s}.{kind_se}").unwrap();
                        }
                        if col == self.col_rio() - 1 {
                            if row != self.row_bio() + 1 {
                                let kind_se = self.get_rio_kind(row - 1);
                                write!(naming, ".{kind_se}").unwrap();
                            }
                            let kind_e = self.get_rio_kind(row);
                            write!(naming, ".{kind_e}").unwrap();
                        }
                        if row == self.row_tio() - 1 {
                            let kind_n = self.get_tio_kind(col);
                            write!(naming, ".{kind_n}").unwrap();
                        }
                        let mut names = vec![
                            name,
                            self.get_tile_name(col, row - 1),
                            self.get_tile_name(col + 1, row - 1),
                            self.get_tile_name(col + 1, row),
                            self.get_tile_name(col, row + 1),
                        ];
                        if self.kind == GridKind::Xc4000Xv {
                            names.extend([
                                format!("VIR{r}C{c}"),
                                format!("HIR{r}C{c}"),
                                format!("VHIR{r}C{c}"),
                            ]);
                        }
                        let names_ref: Vec<&str> = names.iter().map(|x| &**x).collect();
                        let node = grid.add_xnode(
                            (col, row),
                            db.get_node(kind),
                            &names_ref,
                            db.get_node_naming(&naming),
                            &[
                                (col, row),
                                (col, row - 1),
                                (col + 1, row - 1),
                                (col + 1, row),
                                (col, row + 1),
                            ],
                        );
                        node.add_bel(0, format!("CLB_R{r}C{c}"));
                        node.add_bel(1, format!("TBUF_R{r}C{c}.2"));
                        node.add_bel(2, format!("TBUF_R{r}C{c}.1"));
                        node.tie_name = Some(format!("TIE_R{r}C{c}.1"));
                    }
                }
            }
        }

        if self.kind.is_xl() {
            let llhq_e = db.get_term("LLHQ.E");
            let llhq_w = db.get_term("LLHQ.W");
            let llhq_io_e = db.get_term("LLHQ.IO.E");
            let llhq_io_w = db.get_term("LLHQ.IO.W");
            let llhc_e = db.get_term("LLHC.E");
            let llhc_w = db.get_term("LLHC.W");

            for row in grid.rows() {
                let r = row_t.to_idx() - row.to_idx();
                for (lr, col) in [('L', self.col_ql()), ('R', self.col_qr())] {
                    let c = col.to_idx();
                    if row == self.row_bio() || row == self.row_tio() {
                        grid.fill_term_pair_anon((col - 1, row), (col, row), llhq_io_e, llhq_io_w);
                    } else {
                        grid.fill_term_pair_anon((col - 1, row), (col, row), llhq_e, llhq_w);
                    }
                    let (kind, naming, name) = if row == self.row_bio() {
                        ("LLHQ.B", "LLHQ.B", format!("BQ{lr}"))
                    } else if row == self.row_tio() {
                        ("LLHQ.T", "LLHQ.T", format!("TQ{lr}"))
                    } else {
                        (
                            "LLHQ",
                            if self.kind != GridKind::Xc4000Xv {
                                "LLHQ"
                            } else if row >= self.row_qb() && row < self.row_qt() {
                                "LLHQ.I"
                            } else {
                                "LLHQ.O"
                            },
                            format!("VQ{lr}R{r}"),
                        )
                    };
                    let node = grid.add_xnode(
                        (col, row),
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(naming),
                        &[(col - 1, row), (col, row)],
                    );
                    if kind == "LLHQ" {
                        if self.kind == GridKind::Xc4000Xla {
                            node.add_bel(0, format!("PULLUP_R{r}C{cc}.4", cc = c - 1));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(2, format!("PULLUP_R{r}C{cc}.3", cc = c - 1));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.1"));
                        } else {
                            node.add_bel(0, format!("PULLUP_R{r}C{c}.4"));
                            node.add_bel(1, format!("PULLUP_R{r}C{c}.2"));
                            node.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                            node.add_bel(3, format!("PULLUP_R{r}C{c}.1"));
                        }
                    }
                }
                let col = self.col_mid();
                let c = col.to_idx();
                grid.fill_term_pair_anon((col - 1, row), (col, row), llhc_e, llhc_w);
                let (kind, naming, name) = if row == self.row_bio() {
                    ("LLHC.B", "LLHC.B", "BM".to_string())
                } else if row == self.row_tio() {
                    ("LLHC.T", "LLHC.T", "TM".to_string())
                } else {
                    (
                        "LLHC",
                        if row >= self.row_qb() && row < self.row_qt() {
                            "LLHC.I"
                        } else {
                            "LLHC.O"
                        },
                        format!("VMR{r}"),
                    )
                };
                let node = grid.add_xnode(
                    (col, row),
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(naming),
                    &[(col - 1, row), (col, row)],
                );
                if row == self.row_bio() {
                    node.add_bel(0, format!("PULLUP_R{r}C{c}.4"));
                    node.add_bel(1, format!("PULLUP_R{r}C{c}.5"));
                    node.add_bel(2, format!("PULLUP_R{r}C{c}.3"));
                    node.add_bel(3, format!("PULLUP_R{r}C{c}.6"));
                    node.add_bel(4, format!("PULLUP_R{r}C{c}.2"));
                    node.add_bel(5, format!("PULLUP_R{r}C{c}.7"));
                    node.add_bel(6, format!("PULLUP_R{r}C{c}.1"));
                    node.add_bel(7, format!("PULLUP_R{r}C{c}.8"));
                } else if row == self.row_tio() {
                    node.add_bel(0, format!("PULLUP_R{r}C{c}.1"));
                    node.add_bel(1, format!("PULLUP_R{r}C{c}.8"));
                    node.add_bel(2, format!("PULLUP_R{r}C{c}.2"));
                    node.add_bel(3, format!("PULLUP_R{r}C{c}.7"));
                    node.add_bel(4, format!("PULLUP_R{r}C{c}.3"));
                    node.add_bel(5, format!("PULLUP_R{r}C{c}.6"));
                    node.add_bel(6, format!("PULLUP_R{r}C{c}.4"));
                    node.add_bel(7, format!("PULLUP_R{r}C{c}.5"));
                } else {
                    node.add_bel(0, format!("PULLUP_R{r}C{c}.2"));
                    node.add_bel(1, format!("PULLUP_R{r}C{c}.4"));
                    node.add_bel(2, format!("PULLUP_R{r}C{c}.1"));
                    node.add_bel(3, format!("PULLUP_R{r}C{c}.3"));
                }
            }

            let llvq_n = db.get_term("LLVQ.N");
            let llvq_s = db.get_term("LLVQ.S");
            let llvc_n = db.get_term("LLVC.N");
            let llvc_s = db.get_term("LLVC.S");

            for col in grid.cols() {
                let c = col.to_idx();
                for (bt, row) in [('B', self.row_qb()), ('T', self.row_qt())] {
                    grid.fill_term_pair_anon((col, row - 1), (col, row), llvq_n, llvq_s);
                    let (kind, naming, name) = if col == self.col_lio() {
                        (
                            if bt == 'B' { "LLVQ.L.B" } else { "LLVQ.L.T" },
                            if bt == 'B' { "LLVQ.L.B" } else { "LLVQ.L.T" },
                            format!("LQ{bt}"),
                        )
                    } else if col == self.col_rio() {
                        (
                            if bt == 'B' { "LLVQ.R.B" } else { "LLVQ.R.T" },
                            if self.is_buff_large {
                                if bt == 'B' {
                                    "LLVQ.R.B"
                                } else {
                                    "LLVQ.R.T"
                                }
                            } else {
                                if bt == 'B' {
                                    "LLVQ.R.BS"
                                } else {
                                    "LLVQ.R.TS"
                                }
                            },
                            format!("RQ{bt}"),
                        )
                    } else {
                        ("LLVQ", "LLVQ", format!("HQ{bt}C{c}"))
                    };
                    let node = grid.add_xnode(
                        (col, row),
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(naming),
                        &[(col, row - 1), (col, row)],
                    );
                    let sn = if bt == 'B' { 'S' } else { 'N' };
                    let we = if col == self.col_lio() { 'W' } else { 'E' };
                    if kind != "LLVQ" {
                        node.add_bel(0, format!("BUFF_{sn}{we}"));
                    }
                }
                let row = self.row_mid();
                let r = row_t.to_idx() - row.to_idx() + 1;
                grid.fill_term_pair_anon((col, row - 1), (col, row), llvc_n, llvc_s);
                let (kind, name) = if col == self.col_lio() {
                    ("LLVC.L", "LM".to_string())
                } else if col == self.col_rio() {
                    ("LLVC.R", "RM".to_string())
                } else {
                    ("LLVC", format!("HMC{c}"))
                };
                let node = grid.add_xnode(
                    (col, row),
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(kind),
                    &[(col, row - 1), (col, row)],
                );
                if col == self.col_lio() || col == self.col_rio() {
                    node.add_bel(0, format!("PULLUP_R{r}C{c}.10"));
                    node.add_bel(1, format!("PULLUP_R{r}C{c}.3"));
                    node.add_bel(2, format!("PULLUP_R{r}C{c}.9"));
                    node.add_bel(3, format!("PULLUP_R{r}C{c}.4"));
                    node.add_bel(4, format!("PULLUP_R{r}C{c}.8"));
                    node.add_bel(5, format!("PULLUP_R{r}C{c}.5"));
                    node.add_bel(6, format!("PULLUP_R{r}C{c}.7"));
                    node.add_bel(7, format!("PULLUP_R{r}C{c}.6"));
                }
            }

            if self.kind == GridKind::Xc4000Xv {
                for (bt, row) in [('B', self.row_qb()), ('T', self.row_qt())] {
                    for (lr, col) in [('L', self.col_ql()), ('R', self.col_qr())] {
                        grid.add_xnode(
                            (col, row),
                            db.get_node("CLKQ"),
                            &[&format!("Q{bt}{lr}")],
                            db.get_node_naming(&format!("CLKQ.{bt}")),
                            &[(col - 1, row), (col, row)],
                        );
                    }
                }
                // XXX
            } else {
                grid.add_xnode(
                    (self.col_mid(), self.row_mid()),
                    db.get_node("CLKC"),
                    &["M"],
                    db.get_node_naming("CLKC"),
                    &[],
                );
                grid.add_xnode(
                    (self.col_mid(), self.row_qb()),
                    db.get_node("CLKQC"),
                    &["VMQB"],
                    db.get_node_naming("CLKQC.B"),
                    &[(self.col_mid(), self.row_qb())],
                );
                grid.add_xnode(
                    (self.col_mid(), self.row_qt()),
                    db.get_node("CLKQC"),
                    &["VMQT"],
                    db.get_node_naming("CLKQC.T"),
                    &[(self.col_mid(), self.row_qt())],
                );
            }
        } else {
            let llhc_e = db.get_term("LLHC.E");
            let llhc_w = db.get_term("LLHC.W");

            for row in grid.rows() {
                let col = self.col_mid();
                let r = row_t.to_idx() - row.to_idx();
                grid.fill_term_pair_anon((col - 1, row), (col, row), llhc_e, llhc_w);
                let (kind, naming, name) = if row == self.row_bio() {
                    ("LLH.B", "LLH.B", "BM".to_string())
                } else if row == self.row_tio() {
                    ("LLH.T", "LLH.T", "TM".to_string())
                } else {
                    (
                        "LLH",
                        if row < self.row_mid() {
                            "LLH.CB"
                        } else {
                            "LLH.CT"
                        },
                        format!("VMR{r}"),
                    )
                };
                grid.add_xnode(
                    (col, row),
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(naming),
                    &[(col - 1, row), (col, row)],
                );
            }

            let llvc_n = db.get_term("LLVC.N");
            let llvc_s = db.get_term("LLVC.S");

            for col in grid.cols() {
                let row = self.row_mid();
                grid.fill_term_pair_anon((col, row - 1), (col, row), llvc_n, llvc_s);
                let c = col.to_idx();
                let (kind, name) = if col == self.col_lio() {
                    ("LLV.L", "LM".to_string())
                } else if col == self.col_rio() {
                    ("LLV.R", "RM".to_string())
                } else {
                    ("LLV", format!("HMC{c}"))
                };
                grid.add_xnode(
                    (col, row),
                    db.get_node(kind),
                    &[&name],
                    db.get_node_naming(kind),
                    &[(col, row - 1), (col, row)],
                );
            }
        }

        let tclb_n = db.get_term("TCLB.N");
        let main_s = db.get_term("MAIN.S");
        for col in grid.cols() {
            if col != self.col_lio() && col != self.col_rio() {
                grid.fill_term_pair_anon(
                    (col, self.row_tio() - 1),
                    (col, self.row_tio()),
                    tclb_n,
                    main_s,
                );
            }
        }

        let lclb_w = db.get_term("LCLB.W");
        let main_e = db.get_term("MAIN.E");
        for row in grid.rows() {
            if row != self.row_bio() && row != self.row_tio() {
                grid.fill_term_pair_anon(
                    (self.col_lio(), row),
                    (self.col_lio() + 1, row),
                    main_e,
                    lclb_w,
                );
            }
        }

        grid.fill_main_passes();
        grid.fill_term_anon((col_l, row_b), "CNR.LL.W");
        grid.fill_term_anon((col_r, row_b), "CNR.LR.S");
        grid.fill_term_anon((col_l, row_t), "CNR.UL.N");
        grid.fill_term_anon((col_r, row_t), "CNR.UR.E");
        grid.fill_term_anon((col_r, row_t), "CNR.UR.N");

        let mut io = vec![];
        for col in grid.cols() {
            if col == self.col_lio() || col == self.col_rio() {
                continue;
            }
            let node = grid[(col, self.row_tio())].nodes.first().unwrap();
            for iob in [0, 1] {
                io.extend([Io {
                    name: node.bels[BelId::from_idx(iob)].clone(),
                    crd: IoCoord {
                        col,
                        row: self.row_tio(),
                        iob: TileIobId::from_idx(iob),
                    },
                }]);
            }
        }
        for row in grid.rows().rev() {
            if row == self.row_bio() || row == self.row_tio() {
                continue;
            }
            let node = grid[(self.col_rio(), row)].nodes.first().unwrap();
            for iob in [0, 1] {
                io.extend([Io {
                    name: node.bels[BelId::from_idx(iob)].clone(),
                    crd: IoCoord {
                        col: self.col_rio(),
                        row,
                        iob: TileIobId::from_idx(iob),
                    },
                }]);
            }
        }
        for col in grid.cols().rev() {
            if col == self.col_lio() || col == self.col_rio() {
                continue;
            }
            let node = grid[(col, self.row_bio())].nodes.first().unwrap();
            for iob in [1, 0] {
                io.extend([Io {
                    name: node.bels[BelId::from_idx(iob)].clone(),
                    crd: IoCoord {
                        col,
                        row: self.row_bio(),
                        iob: TileIobId::from_idx(iob),
                    },
                }]);
            }
        }
        for row in grid.rows() {
            if row == self.row_bio() || row == self.row_tio() {
                continue;
            }
            let node = grid[(self.col_lio(), row)].nodes.first().unwrap();
            for iob in [1, 0] {
                io.extend([Io {
                    name: node.bels[BelId::from_idx(iob)].clone(),
                    crd: IoCoord {
                        col: self.col_lio(),
                        row,
                        iob: TileIobId::from_idx(iob),
                    },
                }]);
            }
        }

        let mut spine_framebit = None;
        let mut qb_framebit = None;
        let mut qt_framebit = None;
        let mut row_framebit = EntityVec::new();
        let mut frame_len = 0;
        for row in grid.rows() {
            if self.kind.is_xl() && row == self.row_qb() {
                qb_framebit = Some(frame_len);
                frame_len += 2;
            }
            if self.kind.is_xl() && row == self.row_qt() {
                qt_framebit = Some(frame_len);
                frame_len += 2;
            }
            if row == self.row_mid() {
                spine_framebit = Some(frame_len);
                frame_len += match self.kind {
                    GridKind::Xc4000
                    | GridKind::Xc4000A
                    | GridKind::Xc4000H
                    | GridKind::Xc4000E => 1,
                    GridKind::Xc4000Ex
                    | GridKind::Xc4000Xla
                    | GridKind::Xc4000Xv
                    | GridKind::SpartanXl => 2,
                };
            }
            row_framebit.push(frame_len);
            let height = if row == self.row_bio() {
                match self.kind {
                    GridKind::Xc4000 | GridKind::Xc4000E => 13,
                    GridKind::Xc4000A => todo!(),
                    GridKind::Xc4000H => todo!(),
                    GridKind::Xc4000Ex | GridKind::Xc4000Xla => 16,
                    GridKind::Xc4000Xv => 17,
                    GridKind::SpartanXl => 13,
                }
            } else if row == self.row_tio() {
                match self.kind {
                    GridKind::Xc4000 | GridKind::Xc4000E => 7,
                    GridKind::Xc4000A => todo!(),
                    GridKind::Xc4000H => todo!(),
                    GridKind::Xc4000Ex | GridKind::Xc4000Xla => 8,
                    GridKind::Xc4000Xv => 9,
                    GridKind::SpartanXl => 7,
                }
            } else {
                match self.kind {
                    GridKind::Xc4000 | GridKind::Xc4000E => 10,
                    GridKind::Xc4000A => todo!(),
                    GridKind::Xc4000H => todo!(),
                    GridKind::Xc4000Ex | GridKind::Xc4000Xla => 12,
                    GridKind::Xc4000Xv => 13,
                    GridKind::SpartanXl => 10,
                }
            };
            frame_len += height;
        }
        let spine_framebit = spine_framebit.unwrap();
        let quarter_framebit = qb_framebit.zip(qt_framebit);

        let mut frame_info = vec![];
        let mut spine_frame = None;
        let mut ql_frame = None;
        let mut qr_frame = None;
        let mut col_frame: EntityVec<_, _> = grid.cols().map(|_| 0).collect();
        for col in grid.cols().rev() {
            // TODO
            let width = if col == self.col_lio() {
                match self.kind {
                    GridKind::Xc4000 | GridKind::Xc4000E | GridKind::SpartanXl => 26,
                    GridKind::Xc4000A => todo!(),
                    GridKind::Xc4000H => todo!(),
                    GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 27,
                }
            } else if col == self.col_rio() {
                match self.kind {
                    GridKind::Xc4000 | GridKind::Xc4000E | GridKind::SpartanXl => 41,
                    GridKind::Xc4000A => todo!(),
                    GridKind::Xc4000H => todo!(),
                    GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 52,
                }
            } else {
                match self.kind {
                    GridKind::Xc4000 | GridKind::Xc4000E | GridKind::SpartanXl => 36,
                    GridKind::Xc4000A => todo!(),
                    GridKind::Xc4000H => todo!(),
                    GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv => 47,
                }
            };
            col_frame[col] = frame_info.len();
            for _ in 0..width {
                let minor = frame_info.len();
                frame_info.push(FrameInfo {
                    addr: FrameAddr {
                        typ: 0,
                        region: 0,
                        major: 0,
                        minor: minor as u32,
                    },
                });
            }
            if col == self.col_mid() {
                let width = match self.kind {
                    GridKind::Xc4000 | GridKind::Xc4000E => 1,
                    GridKind::Xc4000A => todo!(),
                    GridKind::Xc4000H => todo!(),
                    GridKind::Xc4000Ex
                    | GridKind::Xc4000Xla
                    | GridKind::Xc4000Xv
                    | GridKind::SpartanXl => 2,
                };
                spine_frame = Some(frame_info.len());
                for _ in 0..width {
                    let minor = frame_info.len();
                    frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: 0,
                            major: 0,
                            minor: minor as u32,
                        },
                    });
                }
            }
            if self.kind.is_xl() && col == self.col_ql() {
                let minor = frame_info.len();
                ql_frame = Some(minor);
                frame_info.push(FrameInfo {
                    addr: FrameAddr {
                        typ: 0,
                        region: 0,
                        major: 0,
                        minor: minor as u32,
                    },
                });
            }
            if self.kind.is_xl() && col == self.col_qr() {
                let minor = frame_info.len();
                qr_frame = Some(minor);
                frame_info.push(FrameInfo {
                    addr: FrameAddr {
                        typ: 0,
                        region: 0,
                        major: 0,
                        minor: minor as u32,
                    },
                });
            }
        }
        let spine_frame = spine_frame.unwrap();
        let quarter_frame = ql_frame.zip(qr_frame);

        let die_bs_geom = DieBitstreamGeom {
            frame_len,
            frame_info,
            bram_frame_len: 0,
            bram_frame_info: vec![],
            iob_frame_len: 0,
        };
        let bs_geom = BitstreamGeom {
            kind: DeviceKind::Xc4000,
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![DieId::from_idx(0)],
        };

        egrid.finish();

        ExpandedDevice {
            grid: self,
            egrid,
            io,
            bs_geom,
            spine_frame,
            quarter_frame,
            col_frame,
            spine_framebit,
            quarter_framebit,
            row_framebit,
        }
    }
}
