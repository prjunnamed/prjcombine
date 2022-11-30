use core::cmp::Ordering;
use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::{BelId, IntDb, NodeRawTileId};
use prjcombine_int::grid::{
    ColId, Coord, ExpandedDieRefMut, ExpandedGrid, ExpandedTileNode, Rect, RowId,
};
use prjcombine_virtex_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::collections::BTreeSet;

use crate::expanded::ExpandedDevice;
use crate::grid::{ColumnIoKind, ColumnKind, DisabledPart, Grid, Gts};

struct Expander<'a, 'b> {
    grid: &'b Grid,
    db: &'a IntDb,
    disabled: &'b BTreeSet<DisabledPart>,
    die: ExpandedDieRefMut<'a, 'b>,
    tiexlut: EntityVec<ColId, usize>,
    rxlut: EntityVec<ColId, usize>,
    rylut: EntityVec<RowId, usize>,
    ioxlut: EntityVec<ColId, usize>,
    ioylut: EntityVec<RowId, usize>,
    pad_cnt: usize,
    bonded_ios: Vec<((ColId, RowId), BelId)>,
    holes: Vec<Rect>,
    frame_info: Vec<FrameInfo>,
    iob_frame_len: usize,
}

impl<'a, 'b> Expander<'a, 'b> {
    fn fill_rxlut(&mut self) {
        let mut rx = 2;
        for &cd in self.grid.columns.values() {
            self.rxlut.push(rx);
            match cd.kind {
                ColumnKind::CleXL | ColumnKind::CleXM => rx += 2,
                ColumnKind::CleClk => rx += 4,
                _ => rx += 3,
            }
        }
    }

    fn fill_rylut(&mut self) {
        let mut ry = 2;
        for row in self.grid.rows.ids() {
            if row == self.grid.row_clk() {
                ry += 1;
            }
            if row.to_idx() % 16 == 8 {
                ry += 1;
            }
            self.rylut.push(ry);
            ry += 1;
        }
    }

    fn fill_ioxlut(&mut self) {
        let mut iox = 0;
        for &cd in self.grid.columns.values() {
            self.ioxlut.push(iox);
            if cd.kind == ColumnKind::Io
                || cd.bio != ColumnIoKind::None
                || cd.tio != ColumnIoKind::None
            {
                iox += 1;
            }
        }
    }

    fn fill_ioylut(&mut self) {
        let mut ioy = 0;
        for (row, &rd) in &self.grid.rows {
            self.ioylut.push(ioy);
            if row == self.grid.row_bio_outer()
                || row == self.grid.row_bio_inner()
                || row == self.grid.row_tio_inner()
                || row == self.grid.row_tio_outer()
                || rd.lio
                || rd.rio
            {
                ioy += 1;
            }
        }
    }

    fn fill_tiexlut(&mut self) {
        let mut tie_x = 0;
        for &cd in self.grid.columns.values() {
            self.tiexlut.push(tie_x);
            tie_x += 1;
            if cd.kind == ColumnKind::Io
                || cd.tio != ColumnIoKind::None
                || cd.bio != ColumnIoKind::None
            {
                tie_x += 1;
            }
            if cd.kind == ColumnKind::CleClk {
                tie_x += 1;
            }
        }
    }

    fn fill_int(&mut self) {
        for (col, &cd) in &self.grid.columns {
            for row in self.die.rows() {
                let x = col.to_idx();
                let y = row.to_idx();
                let tie_y = y * 2;
                let mut is_brk = y % 16 == 0;
                if y == 0
                    && !matches!(
                        cd.kind,
                        ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
                    )
                {
                    is_brk = false;
                }
                if row == self.grid.row_clk() && cd.kind == ColumnKind::Io {
                    is_brk = false;
                }
                let bram = if cd.kind == ColumnKind::Bram {
                    if is_brk {
                        "_BRAM_BRK"
                    } else {
                        "_BRAM"
                    }
                } else {
                    ""
                };
                self.die.fill_tile(
                    (col, row),
                    "INT",
                    if is_brk { "INT.BRK" } else { "INT" },
                    format!("INT{bram}_X{x}Y{y}"),
                );
                let tie_x = self.tiexlut[col];
                self.die[(col, row)].nodes[0].tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                if matches!(
                    cd.kind,
                    ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::DspPlus
                ) {
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("INTF"),
                        &[&format!("INT_INTERFACE_X{x}Y{y}")],
                        self.db.get_node_naming("INTF"),
                        &[(col, row)],
                    );
                }
            }
        }
    }

    fn fill_lio(&mut self) {
        let col = self.grid.col_lio();
        for (row, &rd) in &self.grid.rows {
            let x = col.to_idx();
            let y = row.to_idx();
            let is_brk = y % 16 == 0 && row != self.grid.row_clk();
            let tile = &mut self.die[(col, row)];
            let mut ltt = "IOI_LTERM";
            if rd.lio {
                self.fill_ioi((col, row), if is_brk { "LIOI_BRK" } else { "LIOI" });
                let mut tk = "LIOB";
                if row == self.grid.row_clk() - 1 {
                    tk = "LIOB_RDY";
                }
                if row == self.grid.row_clk() + 2 {
                    tk = "LIOB_PCI";
                }
                self.fill_iob((col, row), tk, tk);
            } else {
                let cnr = if row == self.grid.row_bio_outer() {
                    Some("LL")
                } else if row == self.grid.row_tio_outer() {
                    Some("UL")
                } else {
                    None
                };
                if let Some(cnr) = cnr {
                    ltt = "CNR_TL_LTERM";
                    let name = format!("{cnr}_X{x}Y{y}");
                    tile.add_xnode(
                        self.db.get_node("INTF"),
                        &[&name],
                        self.db.get_node_naming("INTF.CNR"),
                        &[(col, row)],
                    );
                    let node = tile.add_xnode(
                        self.db.get_node(cnr),
                        &[&name],
                        self.db.get_node_naming(cnr),
                        &[(col, row)],
                    );
                    match cnr {
                        "LL" => {
                            node.add_bel(0, "OCT_CAL_X0Y0".to_string());
                            node.add_bel(1, "OCT_CAL_X0Y1".to_string());
                        }
                        "UL" => {
                            node.add_bel(0, "OCT_CAL_X0Y2".to_string());
                            node.add_bel(1, "OCT_CAL_X0Y3".to_string());
                            node.add_bel(2, "PMV".to_string());
                            node.add_bel(3, "DNA_PORT".to_string());
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let carry = if is_brk { "_CARRY" } else { "" };
                    tile.add_xnode(
                        self.db.get_node("INTF"),
                        &[&format!("INT_INTERFACE{carry}_X{x}Y{y}")],
                        self.db.get_node_naming("INTF"),
                        &[(col, row)],
                    );
                }
            }

            let rx = self.rxlut[col];
            let ry = self.rylut[row];
            let (txtra, naming) = if row == self.grid.row_clk() - 2 {
                ("_LOWER_BOT", Some("CLKPIN_BUF.L.BOT"))
            } else if row == self.grid.row_clk() - 1 {
                ("_LOWER_TOP", Some("CLKPIN_BUF.L.TOP"))
            } else if row == self.grid.row_clk() + 2 {
                ("_UPPER_BOT", Some("CLKPIN_BUF.L.BOT"))
            } else if row == self.grid.row_clk() + 3 {
                ("_UPPER_TOP", Some("CLKPIN_BUF.L.TOP"))
            } else {
                ("", None)
            };
            let name = format!("{ltt}{txtra}_X{rx}Y{ry}", rx = rx - 1);
            if let Some(naming) = naming {
                self.die[(col, row)].add_xnode(
                    self.db.get_node("CLKPIN_BUF"),
                    &[&name],
                    self.db.get_node_naming(naming),
                    &[],
                );
            }
            self.die
                .fill_term_tile((col, row), "TERM.W", "TERM.W", name);

            if row.to_idx() % 16 == 8 {
                let kind;
                let split;
                let trunk_naming;
                let v_naming;
                if row <= self.grid.row_clk() {
                    match row.cmp(&self.grid.rows_pci_ce_split.0) {
                        Ordering::Less => {
                            kind = "HCLK_IOIL_BOT_DN";
                            v_naming = "PCI_CE_V_BUF_DN";
                            split = false;
                        }
                        Ordering::Equal => {
                            kind = "HCLK_IOIL_BOT_SPLIT";
                            v_naming = "";
                            split = true;
                        }
                        Ordering::Greater => {
                            kind = "HCLK_IOIL_BOT_UP";
                            v_naming = "PCI_CE_V_BUF_UP";
                            split = false;
                        }
                    }
                    trunk_naming = "PCI_CE_TRUNK_BUF_BOT";
                } else {
                    match row.cmp(&self.grid.rows_pci_ce_split.1) {
                        Ordering::Less => {
                            kind = "HCLK_IOIL_TOP_DN";
                            v_naming = "PCI_CE_V_BUF_DN";
                            split = false;
                        }
                        Ordering::Equal => {
                            kind = "HCLK_IOIL_TOP_SPLIT";
                            v_naming = "";
                            split = true;
                        }
                        Ordering::Greater => {
                            kind = "HCLK_IOIL_TOP_UP";
                            v_naming = "PCI_CE_V_BUF_UP";
                            split = false;
                        }
                    }
                    trunk_naming = "PCI_CE_TRUNK_BUF_TOP";
                }
                let name = format!("{kind}_X{x}Y{y}", y = y - 1);
                let tile = &mut self.die[(col, row)];
                let name_term = if row == self.grid.row_clk() {
                    format!("HCLK_IOI_LTERM_BOT25_X{rx}Y{ry}", rx = rx - 1, ry = ry - 2)
                } else {
                    format!("HCLK_IOI_LTERM_X{rx}Y{ry}", rx = rx - 1, ry = ry - 1)
                };
                tile.add_xnode(
                    self.db.get_node("LRIOI_CLK"),
                    &[&name, &name_term],
                    self.db.get_node_naming("LRIOI_CLK.L"),
                    &[],
                );
                if split {
                    tile.add_xnode(
                        self.db.get_node("PCI_CE_SPLIT"),
                        &[&name],
                        self.db.get_node_naming("PCI_CE_SPLIT"),
                        &[],
                    );
                } else {
                    tile.add_xnode(
                        self.db.get_node("PCI_CE_TRUNK_BUF"),
                        &[&name],
                        self.db.get_node_naming(trunk_naming),
                        &[],
                    );
                    if row != self.grid.row_clk() {
                        tile.add_xnode(
                            self.db.get_node("PCI_CE_V_BUF"),
                            &[&name],
                            self.db.get_node_naming(v_naming),
                            &[],
                        );
                    }
                }
            }

            if row == self.grid.row_bio_outer() {
                let name = format!("IOI_PCI_CE_LEFT_X{rx}Y{ry}", ry = ry - 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("PCI_CE_H_BUF"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_H_BUF_CNR"),
                    &[],
                );
            }
            if row == self.grid.row_tio_outer() {
                let name = format!("IOI_PCI_CE_LEFT_X{rx}Y{ry}", ry = ry + 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("PCI_CE_H_BUF"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_H_BUF_CNR"),
                    &[],
                );
            }
        }
    }

    fn fill_rio(&mut self) {
        let col = self.grid.col_rio();
        for (row, &rd) in self.grid.rows.iter().rev() {
            let x = col.to_idx();
            let y = row.to_idx();
            let is_brk = y % 16 == 0 && row != self.grid.row_clk();
            let tile = &mut self.die[(col, row)];
            let mut rtt = "IOI_RTERM";
            if rd.rio {
                self.fill_ioi((col, row), if is_brk { "RIOI_BRK" } else { "RIOI" });
                let mut tk = "RIOB";
                if row == self.grid.row_clk() - 1 {
                    tk = "RIOB_PCI";
                }
                if row == self.grid.row_clk() + 2 {
                    tk = "RIOB_RDY";
                }
                self.fill_iob((col, row), tk, tk);
            } else {
                let cnr = if row == self.grid.row_bio_outer() {
                    Some("LR_LOWER")
                } else if row == self.grid.row_bio_inner() {
                    Some("LR_UPPER")
                } else if row == self.grid.row_tio_inner() {
                    Some("UR_LOWER")
                } else if row == self.grid.row_tio_outer() {
                    Some("UR_UPPER")
                } else {
                    None
                };
                if let Some(cnr) = cnr {
                    rtt = "CNR_TR_RTERM";
                    let name = format!("{cnr}_X{x}Y{y}");
                    tile.add_xnode(
                        self.db.get_node("INTF"),
                        &[&name],
                        self.db.get_node_naming("INTF.CNR"),
                        &[(col, row)],
                    );
                    let node = tile.add_xnode(
                        self.db.get_node(cnr),
                        &[&name],
                        self.db.get_node_naming(cnr),
                        &[(col, row)],
                    );
                    match cnr {
                        "LR_LOWER" => {
                            node.add_bel(0, "OCT_CAL_X1Y0".to_string());
                            node.add_bel(1, "ICAP_X0Y0".to_string());
                            node.add_bel(2, "SPI_ACCESS".to_string());
                        }
                        "LR_UPPER" => {
                            node.add_bel(0, "SUSPEND_SYNC".to_string());
                            node.add_bel(1, "POST_CRC_INTERNAL".to_string());
                            node.add_bel(2, "STARTUP".to_string());
                            node.add_bel(3, "SLAVE_SPI".to_string());
                        }
                        "UR_LOWER" => {
                            node.add_bel(0, "OCT_CAL_X1Y1".to_string());
                            node.add_bel(1, "BSCAN_X0Y2".to_string());
                            node.add_bel(2, "BSCAN_X0Y3".to_string());
                        }
                        "UR_UPPER" => {
                            node.add_bel(0, "BSCAN_X0Y0".to_string());
                            node.add_bel(1, "BSCAN_X0Y1".to_string());
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let carry = if is_brk { "_CARRY" } else { "" };
                    tile.add_xnode(
                        self.db.get_node("INTF"),
                        &[&format!("INT_INTERFACE{carry}_X{x}Y{y}")],
                        self.db.get_node_naming("INTF"),
                        &[(col, row)],
                    );
                }
            }

            let rx = self.rxlut[col];
            let ry = self.rylut[row];
            let (txtra, naming) = if row == self.grid.row_clk() - 2 {
                ("_LOWER_BOT", Some("CLKPIN_BUF.R.BOT"))
            } else if row == self.grid.row_clk() - 1 {
                ("_LOWER_TOP", Some("CLKPIN_BUF.R.TOP"))
            } else if row == self.grid.row_clk() + 2 {
                ("_UPPER_BOT", Some("CLKPIN_BUF.R.BOT"))
            } else if row == self.grid.row_clk() + 3 {
                ("_UPPER_TOP", Some("CLKPIN_BUF.R.TOP"))
            } else {
                ("", None)
            };
            let name = format!("{rtt}{txtra}_X{rx}Y{ry}", rx = rx + 3);
            if let Some(naming) = naming {
                self.die[(col, row)].add_xnode(
                    self.db.get_node("CLKPIN_BUF"),
                    &[&name],
                    self.db.get_node_naming(naming),
                    &[],
                );
            }
            self.die
                .fill_term_tile((col, row), "TERM.E", "TERM.E", name);

            if row.to_idx() % 16 == 8 {
                let kind;
                let split;
                let trunk_naming;
                let v_naming;
                if row <= self.grid.row_clk() {
                    match row.cmp(&self.grid.rows_pci_ce_split.0) {
                        Ordering::Less => {
                            kind = "HCLK_IOIR_BOT_DN";
                            v_naming = "PCI_CE_V_BUF_DN";
                            split = false;
                        }
                        Ordering::Equal => {
                            kind = "HCLK_IOIR_BOT_SPLIT";
                            v_naming = "";
                            split = true;
                        }
                        Ordering::Greater => {
                            kind = "HCLK_IOIR_BOT_UP";
                            v_naming = "PCI_CE_V_BUF_UP";
                            split = false;
                        }
                    }
                    trunk_naming = "PCI_CE_TRUNK_BUF_BOT";
                } else {
                    match row.cmp(&self.grid.rows_pci_ce_split.1) {
                        Ordering::Less => {
                            kind = "HCLK_IOIR_TOP_DN";
                            v_naming = "PCI_CE_V_BUF_DN";
                            split = false;
                        }
                        Ordering::Equal => {
                            kind = "HCLK_IOIR_TOP_SPLIT";
                            v_naming = "";
                            split = true;
                        }
                        Ordering::Greater => {
                            kind = "HCLK_IOIR_TOP_UP";
                            v_naming = "PCI_CE_V_BUF_UP";
                            split = false;
                        }
                    }
                    trunk_naming = "PCI_CE_TRUNK_BUF_TOP";
                }
                let name = format!("{kind}_X{x}Y{y}", y = y - 1);
                let name_term = if row == self.grid.row_clk() {
                    format!("HCLK_IOI_RTERM_BOT25_X{rx}Y{ry}", rx = rx + 3, ry = ry - 2)
                } else {
                    format!("HCLK_IOI_RTERM_X{rx}Y{ry}", rx = rx + 3, ry = ry - 1)
                };
                let tile = &mut self.die[(col, row)];
                tile.add_xnode(
                    self.db.get_node("LRIOI_CLK"),
                    &[&name, &name_term],
                    self.db.get_node_naming("LRIOI_CLK.R"),
                    &[],
                );
                if split {
                    tile.add_xnode(
                        self.db.get_node("PCI_CE_SPLIT"),
                        &[&name],
                        self.db.get_node_naming("PCI_CE_SPLIT"),
                        &[],
                    );
                } else {
                    tile.add_xnode(
                        self.db.get_node("PCI_CE_TRUNK_BUF"),
                        &[&name],
                        self.db.get_node_naming(trunk_naming),
                        &[],
                    );
                    if row != self.grid.row_clk() && !(self.grid.has_encrypt && row.to_idx() == 8) {
                        tile.add_xnode(
                            self.db.get_node("PCI_CE_V_BUF"),
                            &[&name],
                            self.db.get_node_naming(v_naming),
                            &[],
                        );
                    }
                }
            }

            if row == self.grid.row_bio_outer() {
                let name = format!("IOI_PCI_CE_RIGHT_X{rx}Y{ry}", ry = ry - 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("PCI_CE_H_BUF"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_H_BUF_CNR"),
                    &[],
                );
            }
            if row == self.grid.row_tio_outer() {
                let name = format!("IOI_PCI_CE_RIGHT_X{rx}Y{ry}", ry = ry + 1);
                self.die[(col, row)].add_xnode(
                    self.db.get_node("PCI_CE_H_BUF"),
                    &[&name],
                    self.db.get_node_naming("PCI_CE_H_BUF_CNR"),
                    &[],
                );
            }
        }
    }

    fn fill_tio(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let iob_tk = match cd.tio {
                ColumnIoKind::None => continue,
                ColumnIoKind::Inner => unreachable!(),
                ColumnIoKind::Outer => "TIOB_SINGLE",
                ColumnIoKind::Both => "TIOB",
            };
            for (row, io, unused) in [
                (
                    self.grid.row_tio_outer(),
                    "OUTER",
                    cd.tio == ColumnIoKind::Inner,
                ),
                (
                    self.grid.row_tio_inner(),
                    "INNER",
                    cd.tio == ColumnIoKind::Outer,
                ),
            ] {
                let u = if unused { "_UNUSED" } else { "" };
                let naming = format!("TIOI_{io}{u}");
                self.fill_ioi((col, row), &naming);
                let naming = format!("TIOB_{io}");
                if !unused {
                    self.fill_iob((col, row), iob_tk, &naming);
                }
            }
            let row = self.grid.row_tio_outer();
            let rx = self.rxlut[col] + 1;
            let ry = self.rylut[row] + 1;
            let is_clk = col == self.grid.col_clk || col == self.grid.col_clk + 1;
            let name = if is_clk {
                format!("IOI_TTERM_REGT_X{rx}Y{ry}")
            } else {
                format!("IOI_TTERM_CLB_X{rx}Y{ry}")
            };
            self.die[(col, row)].add_xnode(
                self.db.get_node("BTIOI_CLK"),
                &[&name],
                self.db.get_node_naming("TIOI_CLK"),
                &[],
            );
            if is_clk {
                self.die[(col, row - 1)].add_xnode(
                    self.db.get_node("CLKPIN_BUF"),
                    &[&name],
                    self.db.get_node_naming("CLKPIN_BUF.T.BOT"),
                    &[],
                );
                self.die[(col, row)].add_xnode(
                    self.db.get_node("CLKPIN_BUF"),
                    &[&name],
                    self.db.get_node_naming("CLKPIN_BUF.T.TOP"),
                    &[],
                );
            }
        }
    }

    fn fill_bio(&mut self) {
        for (col, &cd) in self.grid.columns.iter().rev() {
            let iob_tk = match cd.bio {
                ColumnIoKind::None => continue,
                ColumnIoKind::Inner => "BIOB_SINGLE",
                ColumnIoKind::Outer => "BIOB_SINGLE_ALT",
                ColumnIoKind::Both => "BIOB",
            };
            for (row, io, unused) in [
                (
                    self.grid.row_bio_outer(),
                    "OUTER",
                    cd.bio == ColumnIoKind::Inner,
                ),
                (
                    self.grid.row_bio_inner(),
                    "INNER",
                    cd.bio == ColumnIoKind::Outer,
                ),
            ] {
                let u = if unused { "_UNUSED" } else { "" };
                let naming = format!("BIOI_{io}{u}");
                self.fill_ioi((col, row), &naming);
                let naming = format!("BIOB_{io}");
                if !unused {
                    self.fill_iob((col, row), iob_tk, &naming);
                }
            }
            let row = self.grid.row_bio_outer();
            let rx = self.rxlut[col] + 1;
            let ry = self.rylut[row] - 1;
            let is_clk = col == self.grid.col_clk || col == self.grid.col_clk + 1;
            let name = if is_clk {
                format!("IOI_BTERM_REGB_X{rx}Y{ry}")
            } else {
                format!("IOI_BTERM_CLB_X{rx}Y{ry}")
            };
            self.die[(col, row)].add_xnode(
                self.db.get_node("BTIOI_CLK"),
                &[&name],
                self.db.get_node_naming("BIOI_CLK"),
                &[],
            );
            if is_clk {
                self.die[(col, row)].add_xnode(
                    self.db.get_node("CLKPIN_BUF"),
                    &[&name],
                    self.db.get_node_naming("CLKPIN_BUF.B.BOT"),
                    &[],
                );
                self.die[(col, row + 1)].add_xnode(
                    self.db.get_node("CLKPIN_BUF"),
                    &[&name],
                    self.db.get_node_naming("CLKPIN_BUF.B.TOP"),
                    &[],
                );
            }
        }
    }

    fn fill_mcb(&mut self) {
        if self.disabled.contains(&DisabledPart::Mcb) {
            return;
        }
        let mut mx = 0;
        for (col, &cd) in &self.grid.columns {
            if cd.kind != ColumnKind::Io {
                continue;
            }
            let x = col.to_idx();
            let mut my = 1;
            for mcb in &self.grid.mcbs {
                let row = mcb.row_mcb;
                let mut crds = vec![];
                for dy in 0..12 {
                    crds.push((col, row + dy));
                }
                for urow in mcb.row_mui {
                    for dy in 0..2 {
                        crds.push((col, urow + dy));
                    }
                }
                let tk = if self.grid.is_25() {
                    "MCB_L_BOT"
                } else {
                    "MCB_L"
                };
                let name = format!("{tk}_X{x}Y{y}", y = row.to_idx() + 6);
                let name_hclk = format!("MCB_HCLK_X{x}Y{y}", y = row.to_idx() - 1);
                let name_clkpn = format!("MCB_CAP_CLKPN_X{x}Y{y}", y = mcb.iop_clk.to_idx());
                let name_ldqs = format!("MCB_INT_DQI_X{x}Y{y}", y = mcb.iop_dqs[0].to_idx());
                let name_udqs = format!("MCB_INT_DQI_X{x}Y{y}", y = mcb.iop_dqs[1].to_idx());
                let name_mui0r = format!("MCB_MUI0R_X{x}Y{y}", y = mcb.row_mui[0].to_idx());
                let name_mui0w = format!("MCB_MUI0W_X{x}Y{y}", y = mcb.row_mui[1].to_idx());
                let name_mui1r = format!("MCB_MUI1R_X{x}Y{y}", y = mcb.row_mui[2].to_idx());
                let name_mui1w = format!("MCB_MUI1W_X{x}Y{y}", y = mcb.row_mui[3].to_idx());
                let name_mui2 = format!("MCB_MUI2_X{x}Y{y}", y = mcb.row_mui[4].to_idx());
                let name_mui3 = format!("MCB_MUI3_X{x}Y{y}", y = mcb.row_mui[5].to_idx());
                let name_mui4 = format!("MCB_MUI4_X{x}Y{y}", y = mcb.row_mui[6].to_idx());
                let name_mui5 = format!("MCB_MUI5_X{x}Y{y}", y = mcb.row_mui[7].to_idx());
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("MCB"),
                    &[
                        &name,
                        &name_hclk,
                        &name_clkpn,
                        &name_ldqs,
                        &name_udqs,
                        &name_mui0r,
                        &name_mui0w,
                        &name_mui1r,
                        &name_mui1w,
                        &name_mui2,
                        &name_mui3,
                        &name_mui4,
                        &name_mui5,
                    ],
                    self.db.get_node_naming(tk),
                    &crds,
                );
                node.add_bel(0, format!("MCB_X{mx}Y{my}"));
                node.add_bel(
                    1,
                    format!(
                        "TIEOFF_X{x}Y{y}",
                        x = self.tiexlut[col] + 1,
                        y = mcb.iop_clk.to_idx() * 2 + 1
                    ),
                );
                node.add_bel(
                    2,
                    format!(
                        "TIEOFF_X{x}Y{y}",
                        x = self.tiexlut[col] + 1,
                        y = mcb.iop_dqs[0].to_idx() * 2 + 1
                    ),
                );
                node.add_bel(
                    3,
                    format!(
                        "TIEOFF_X{x}Y{y}",
                        x = self.tiexlut[col] + 1,
                        y = mcb.iop_dqs[1].to_idx() * 2 + 1
                    ),
                );
                my += 2;
            }
            mx += 1;
        }
    }

    fn fill_pcilogic(&mut self) {
        let row = self.grid.row_clk();
        let y = row.to_idx();
        let ry = self.rylut[row] - 1;

        let col = self.grid.col_lio();
        let x = col.to_idx();
        let name;
        let name_ioi;
        let rx = self.rxlut[col] - 2;
        if self.grid.is_25() {
            name = format!("REGH_LIOI_INT_BOT25_X{x}Y{y}");
            name_ioi = format!("REGH_IOI_BOT25_X{x}Y{y}");
        } else {
            name = format!("REGH_LIOI_INT_X{x}Y{y}", y = y - 1);
            name_ioi = format!("REGH_IOI_X{x}Y{y}", y = y - 1);
        }
        let name_reg = format!("REG_L_X{rx}Y{ry}");
        let name_int = format!("INT_X{x}Y{y}");
        let node = self.die[(col, row)].add_xnode(
            self.db.get_node("PCILOGICSE"),
            &[&name, &name_reg, &name_ioi, &name_int],
            self.db.get_node_naming("PCILOGICSE_L"),
            &[(col, row)],
        );
        node.add_bel(0, "PCILOGIC_X0Y0".to_string());

        let col = self.grid.col_rio();
        let rx = self.rxlut[col] + 3;
        let x = col.to_idx();
        let name = if self.grid.is_25() {
            format!("REGH_RIOI_BOT25_X{x}Y{y}")
        } else {
            format!("REGH_RIOI_X{x}Y{y}", y = y - 1)
        };
        let name_reg = format!("REG_R_X{rx}Y{ry}");
        let name_int = format!("INT_X{x}Y{y}");
        let node = self.die[(col, row)].add_xnode(
            self.db.get_node("PCILOGICSE"),
            &[&name, &name_reg, &name_int],
            self.db.get_node_naming("PCILOGICSE_R"),
            &[(col, row)],
        );
        node.add_bel(0, "PCILOGIC_X1Y0".to_string());
    }

    fn fill_spine(&mut self) {
        let col = self.grid.col_clk;
        let x = col.to_idx();
        let rx = self.rxlut[col];

        let row = self.grid.row_clk();
        let y = row.to_idx();
        self.die[(col, row)].add_xnode(
            self.db.get_node("INTF"),
            &[&format!("INT_INTERFACE_REGC_X{x}Y{y}")],
            self.db.get_node_naming("INTF.REGC"),
            &[(col, row)],
        );
        let node = self.die[(col, row)].add_xnode(
            self.db.get_node("CLKC"),
            &[&format!(
                "CLKC_X{x}Y{y}",
                y = if self.grid.is_25() { y } else { y - 1 }
            )],
            self.db.get_node_naming("CLKC"),
            &[(col, row)],
        );
        for i in 0..16 {
            node.add_bel(
                i,
                format!(
                    "BUFGMUX_X{x}Y{y}",
                    x = if (i & 4) != 0 { 3 } else { 2 },
                    y = i + 1
                ),
            );
        }

        self.die[(col, row)].add_xnode(
            self.db.get_node("CLKC_BUFPLL"),
            &[&format!(
                "REG_C_CMT_X{x}Y{y}",
                y = if self.grid.is_25() { y } else { y - 1 }
            )],
            self.db.get_node_naming("CLKC_BUFPLL"),
            &[],
        );

        for (row, tk) in [
            (self.grid.rows_hclkbuf.0, "REG_V_HCLKBUF_BOT"),
            (self.grid.rows_hclkbuf.1, "REG_V_HCLKBUF_TOP"),
        ] {
            let y = row.to_idx();
            self.die[(col, row)].add_xnode(
                self.db.get_node("HCLK_V_MIDBUF"),
                &[&format!("{tk}_X{x}Y{y}")],
                self.db.get_node_naming("HCLK_V_MIDBUF"),
                &[],
            );
        }

        for (row, tk) in [
            (self.grid.rows_midbuf.0, "REG_V_MIDBUF_BOT"),
            (self.grid.rows_midbuf.1, "REG_V_MIDBUF_TOP"),
        ] {
            let y = row.to_idx();
            self.die[(col, row)].add_xnode(
                self.db.get_node("CKPIN_V_MIDBUF"),
                &[&format!("{tk}_X{x}Y{y}")],
                self.db.get_node_naming(tk),
                &[],
            );
        }

        {
            let row = self.grid.row_bio_outer();
            let name = format!("REG_B_X{rx}Y{ry}", rx = rx + 1, ry = self.rylut[row] - 2);
            let name_term = format!(
                "REG_B_BTERM_X{rx}Y{ry}",
                rx = rx + 2,
                ry = self.rylut[row] - 1
            );
            let name_bufpll = format!(
                "IOI_BTERM_BUFPLL_X{rx}Y{ry}",
                rx = rx + 4,
                ry = self.rylut[row] - 1
            );
            let name_int = format!(
                "IOI_INT_X{x}Y{y}",
                x = col.to_idx() + 1,
                y = row.to_idx() + 1
            );
            let node = self.die[(col, row)].add_xnode(
                self.db.get_node("REG_B"),
                &[&name, &name_term, &name_bufpll, &name_int],
                self.db.get_node_naming("REG_B"),
                &[(col + 1, row + 1)],
            );
            node.add_bel(0, "BUFIO2_X3Y0".to_string());
            node.add_bel(1, "BUFIO2_X3Y1".to_string());
            node.add_bel(2, "BUFIO2_X3Y6".to_string());
            node.add_bel(3, "BUFIO2_X3Y7".to_string());
            node.add_bel(4, "BUFIO2_X1Y0".to_string());
            node.add_bel(5, "BUFIO2_X1Y1".to_string());
            node.add_bel(6, "BUFIO2_X1Y6".to_string());
            node.add_bel(7, "BUFIO2_X1Y7".to_string());
            node.add_bel(8, "BUFIO2FB_X3Y0".to_string());
            node.add_bel(9, "BUFIO2FB_X3Y1".to_string());
            node.add_bel(10, "BUFIO2FB_X3Y6".to_string());
            node.add_bel(11, "BUFIO2FB_X3Y7".to_string());
            node.add_bel(12, "BUFIO2FB_X1Y0".to_string());
            node.add_bel(13, "BUFIO2FB_X1Y1".to_string());
            node.add_bel(14, "BUFIO2FB_X1Y6".to_string());
            node.add_bel(15, "BUFIO2FB_X1Y7".to_string());
            node.add_bel(16, "BUFPLL_X1Y0".to_string());
            node.add_bel(17, "BUFPLL_X1Y1".to_string());
            node.add_bel(18, "BUFPLL_MCB_X1Y5".to_string());
            node.add_bel(
                19,
                format!(
                    "TIEOFF_X{x}Y{y}",
                    x = self.tiexlut[col] + 4,
                    y = row.to_idx() * 2 + 1
                ),
            );
        }

        {
            let row = self.grid.row_tio_outer();
            let name = format!("REG_T_X{rx}Y{ry}", rx = rx + 1, ry = self.rylut[row] + 2);
            let name_term = format!(
                "REG_T_TTERM_X{rx}Y{ry}",
                rx = rx + 2,
                ry = self.rylut[row] + 1
            );
            let name_bufpll = format!(
                "IOI_TTERM_BUFPLL_X{rx}Y{ry}",
                rx = rx + 4,
                ry = self.rylut[row] + 1
            );
            let name_int = format!("IOI_INT_X{x}Y{y}", x = col.to_idx() + 1, y = row.to_idx());
            let node = self.die[(col, row)].add_xnode(
                self.db.get_node("REG_T"),
                &[&name, &name_term, &name_bufpll, &name_int],
                self.db.get_node_naming("REG_T"),
                &[(col + 1, row)],
            );
            node.add_bel(0, "BUFIO2_X2Y28".to_string());
            node.add_bel(1, "BUFIO2_X2Y29".to_string());
            node.add_bel(2, "BUFIO2_X2Y26".to_string());
            node.add_bel(3, "BUFIO2_X2Y27".to_string());
            node.add_bel(4, "BUFIO2_X4Y28".to_string());
            node.add_bel(5, "BUFIO2_X4Y29".to_string());
            node.add_bel(6, "BUFIO2_X4Y26".to_string());
            node.add_bel(7, "BUFIO2_X4Y27".to_string());
            node.add_bel(8, "BUFIO2FB_X2Y28".to_string());
            node.add_bel(9, "BUFIO2FB_X2Y29".to_string());
            node.add_bel(10, "BUFIO2FB_X2Y26".to_string());
            node.add_bel(11, "BUFIO2FB_X2Y27".to_string());
            node.add_bel(12, "BUFIO2FB_X4Y28".to_string());
            node.add_bel(13, "BUFIO2FB_X4Y29".to_string());
            node.add_bel(14, "BUFIO2FB_X4Y26".to_string());
            node.add_bel(15, "BUFIO2FB_X4Y27".to_string());
            node.add_bel(16, "BUFPLL_X1Y5".to_string());
            node.add_bel(17, "BUFPLL_X1Y4".to_string());
            node.add_bel(18, "BUFPLL_MCB_X1Y9".to_string());
            node.add_bel(
                19,
                format!(
                    "TIEOFF_X{x}Y{y}",
                    x = self.tiexlut[col] + 1,
                    y = row.to_idx() * 2 + 1
                ),
            );
        }

        let mut hy = 0;
        for row in self.die.rows() {
            if row.to_idx() % 16 == 8 {
                let y = row.to_idx();
                let ry = self.rylut[row];
                let name = if row == self.grid.row_clk() {
                    format!("REG_V_HCLK_BOT25_X{x}Y{y}", y = y - 1)
                } else {
                    format!("REG_V_HCLK_X{rx}Y{ry}", rx = rx + 2, ry = ry - 1)
                };
                let node = self.die[(col, row)].add_xnode(
                    self.db.get_node("HCLK_ROW"),
                    &[&name],
                    self.db.get_node_naming("HCLK_ROW"),
                    &[],
                );
                for i in 0..16 {
                    node.add_bel(i, format!("BUFH_X0Y{y}", y = 16 + 32 * hy + i));
                }
                for i in 0..16 {
                    node.add_bel(16 + i, format!("BUFH_X3Y{y}", y = 32 * hy + i));
                }
                hy += 1;
            }
        }

        let row = self.grid.row_clk();
        for (col, lr) in [
            (self.grid.cols_reg_buf.0, 'L'),
            (self.grid.cols_reg_buf.1, 'R'),
        ] {
            let x = col.to_idx();
            let y = row.to_idx();
            let tk = match (lr, self.grid.columns[col].kind) {
                ('L', ColumnKind::Dsp) => "REGH_DSP_L",
                ('R', ColumnKind::Dsp | ColumnKind::DspPlus) => "REGH_DSP_R",
                ('L', ColumnKind::Bram) => "REGH_BRAM_FEEDTHRU_L_GCLK",
                ('R', ColumnKind::Bram) => "REGH_BRAM_FEEDTHRU_R_GCLK",
                ('L', ColumnKind::CleXM) => "REGH_CLEXM_INT_GCLKL",
                ('R', ColumnKind::CleXM | ColumnKind::CleXL) => "REGH_CLEXL_INT_CLK",
                _ => unreachable!(),
            };
            let name = if self.grid.is_25() {
                format!("{tk}_X{x}Y{y}")
            } else {
                format!("{tk}_X{x}Y{y}", y = y - 1)
            };
            self.die[(col, row)].add_xnode(
                self.db.get_node("CKPIN_H_MIDBUF"),
                &[&name],
                self.db.get_node_naming("CKPIN_H_MIDBUF"),
                &[],
            );
        }

        {
            let col = self.grid.col_lio();
            let rx = self.rxlut[col];
            let ry = self.rylut[row];
            let name = format!("REG_L_X{rx}Y{ry}", rx = rx - 2, ry = ry - 1);
            let name_term = format!("REGH_IOI_LTERM_X{rx}Y{ry}", rx = rx - 1, ry = ry - 1);
            let name_int0 = format!("INT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx());
            let name_int1 = format!("INT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() + 1);
            let node = self.die[(col, row)].add_xnode(
                self.db.get_node("REG_L"),
                &[&name, &name_term, &name_int0, &name_int1],
                self.db.get_node_naming("REG_L"),
                &[(col, row), (col, row + 1)],
            );
            node.add_bel(0, "BUFIO2_X1Y8".to_string());
            node.add_bel(1, "BUFIO2_X1Y9".to_string());
            node.add_bel(2, "BUFIO2_X1Y14".to_string());
            node.add_bel(3, "BUFIO2_X1Y15".to_string());
            node.add_bel(4, "BUFIO2_X0Y16".to_string());
            node.add_bel(5, "BUFIO2_X0Y17".to_string());
            node.add_bel(6, "BUFIO2_X0Y22".to_string());
            node.add_bel(7, "BUFIO2_X0Y23".to_string());
            node.add_bel(8, "BUFIO2FB_X1Y8".to_string());
            node.add_bel(9, "BUFIO2FB_X1Y9".to_string());
            node.add_bel(10, "BUFIO2FB_X1Y14".to_string());
            node.add_bel(11, "BUFIO2FB_X1Y15".to_string());
            node.add_bel(12, "BUFIO2FB_X0Y16".to_string());
            node.add_bel(13, "BUFIO2FB_X0Y17".to_string());
            node.add_bel(14, "BUFIO2FB_X0Y22".to_string());
            node.add_bel(15, "BUFIO2FB_X0Y23".to_string());
            node.add_bel(16, "BUFPLL_X0Y3".to_string());
            node.add_bel(17, "BUFPLL_X0Y2".to_string());
            node.add_bel(18, "BUFPLL_MCB_X0Y5".to_string());
            node.add_bel(
                19,
                format!(
                    "TIEOFF_X{x}Y{y}",
                    x = self.tiexlut[col] + 1,
                    y = row.to_idx() * 2 - 1
                ),
            );
        }

        {
            let col = self.grid.col_rio();
            let rx = self.rxlut[col];
            let ry = self.rylut[row];
            let name = format!("REG_R_X{rx}Y{ry}", rx = rx + 3, ry = ry - 1);
            let name_term = format!("REGH_IOI_RTERM_X{rx}Y{ry}", rx = rx + 3, ry = ry - 1);
            let name_int0 = format!("INT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx());
            let name_int1 = format!("INT_X{x}Y{y}", x = col.to_idx(), y = row.to_idx() + 1);
            let node = self.die[(col, row)].add_xnode(
                self.db.get_node("REG_R"),
                &[&name, &name_term, &name_int0, &name_int1],
                self.db.get_node_naming("REG_R"),
                &[(col, row), (col, row + 1)],
            );
            node.add_bel(0, "BUFIO2_X4Y20".to_string());
            node.add_bel(1, "BUFIO2_X4Y21".to_string());
            node.add_bel(2, "BUFIO2_X4Y18".to_string());
            node.add_bel(3, "BUFIO2_X4Y19".to_string());
            node.add_bel(4, "BUFIO2_X3Y12".to_string());
            node.add_bel(5, "BUFIO2_X3Y13".to_string());
            node.add_bel(6, "BUFIO2_X3Y10".to_string());
            node.add_bel(7, "BUFIO2_X3Y11".to_string());
            node.add_bel(8, "BUFIO2FB_X4Y20".to_string());
            node.add_bel(9, "BUFIO2FB_X4Y21".to_string());
            node.add_bel(10, "BUFIO2FB_X4Y18".to_string());
            node.add_bel(11, "BUFIO2FB_X4Y19".to_string());
            node.add_bel(12, "BUFIO2FB_X3Y12".to_string());
            node.add_bel(13, "BUFIO2FB_X3Y13".to_string());
            node.add_bel(14, "BUFIO2FB_X3Y10".to_string());
            node.add_bel(15, "BUFIO2FB_X3Y11".to_string());
            node.add_bel(16, "BUFPLL_X2Y3".to_string());
            node.add_bel(17, "BUFPLL_X2Y2".to_string());
            node.add_bel(18, "BUFPLL_MCB_X2Y5".to_string());
            node.add_bel(
                19,
                format!(
                    "TIEOFF_X{x}Y{y}",
                    x = self.tiexlut[col] + 1,
                    y = row.to_idx() * 2 - 1
                ),
            );
        }
    }

    fn fill_cmts(&mut self) {
        let col = self.grid.col_clk;
        let x = col.to_idx();
        let def_rt = NodeRawTileId::from_idx(0);

        let plls;
        let dcms;
        match self.grid.rows.len() {
            64 | 80 => {
                plls = vec![
                    (
                        self.grid.row_bot() + 24,
                        "CMT_PLL_BOT",
                        Some("PLL_BUFPLL_OUT1"),
                    ),
                    (
                        self.grid.row_top() - 8,
                        "CMT_PLL_TOP",
                        Some("PLL_BUFPLL_OUT1"),
                    ),
                ];
                dcms = vec![
                    (self.grid.row_bot() + 8, "CMT_DCM_BOT", "DCM_BUFPLL_BUF_BOT"),
                    (
                        self.grid.row_top() - 24,
                        "CMT_DCM_TOP",
                        "DCM_BUFPLL_BUF_TOP",
                    ),
                ];
            }
            128 => {
                plls = vec![
                    (
                        self.grid.row_bot() + 24,
                        "CMT_PLL1_BOT",
                        Some("PLL_BUFPLL_OUT1"),
                    ),
                    (
                        self.grid.row_bot() + 56,
                        "CMT_PLL2_BOT",
                        Some("PLL_BUFPLL_OUT0"),
                    ),
                    (
                        self.grid.row_top() - 40,
                        "CMT_PLL2_TOP",
                        Some("PLL_BUFPLL_OUT0"),
                    ),
                    (
                        self.grid.row_top() - 8,
                        "CMT_PLL_TOP",
                        Some("PLL_BUFPLL_OUT1"),
                    ),
                ];
                dcms = vec![
                    (self.grid.row_bot() + 8, "CMT_DCM_BOT", "DCM_BUFPLL_BUF_BOT"),
                    (
                        self.grid.row_bot() + 40,
                        "CMT_DCM2_BOT",
                        "DCM_BUFPLL_BUF_BOT_MID",
                    ),
                    (
                        self.grid.row_top() - 56,
                        "CMT_DCM_TOP",
                        "DCM_BUFPLL_BUF_TOP",
                    ),
                    (
                        self.grid.row_top() - 24,
                        "CMT_DCM2_TOP",
                        "DCM_BUFPLL_BUF_TOP_MID",
                    ),
                ];
            }
            192 => {
                plls = vec![
                    (
                        self.grid.row_bot() + 24,
                        "CMT_PLL1_BOT",
                        Some("PLL_BUFPLL_OUT1"),
                    ),
                    (self.grid.row_bot() + 56, "CMT_PLL3_BOT", None),
                    (
                        self.grid.row_bot() + 88,
                        "CMT_PLL2_BOT",
                        Some("PLL_BUFPLL_OUT0"),
                    ),
                    (
                        self.grid.row_top() - 72,
                        "CMT_PLL2_TOP",
                        Some("PLL_BUFPLL_OUT0"),
                    ),
                    (self.grid.row_top() - 40, "CMT_PLL3_TOP", None),
                    (
                        self.grid.row_top() - 8,
                        "CMT_PLL_TOP",
                        Some("PLL_BUFPLL_OUT1"),
                    ),
                ];
                dcms = vec![
                    (self.grid.row_bot() + 8, "CMT_DCM_BOT", "DCM_BUFPLL_BUF_BOT"),
                    (
                        self.grid.row_bot() + 40,
                        "CMT_DCM2_BOT",
                        "DCM_BUFPLL_BUF_BOT_MID",
                    ),
                    (
                        self.grid.row_bot() + 72,
                        "CMT_DCM2_BOT",
                        "DCM_BUFPLL_BUF_BOT_MID",
                    ),
                    (
                        self.grid.row_top() - 88,
                        "CMT_DCM_TOP",
                        "DCM_BUFPLL_BUF_TOP",
                    ),
                    (
                        self.grid.row_top() - 56,
                        "CMT_DCM2_TOP",
                        "DCM_BUFPLL_BUF_TOP_MID",
                    ),
                    (
                        self.grid.row_top() - 24,
                        "CMT_DCM2_TOP",
                        "DCM_BUFPLL_BUF_TOP_MID",
                    ),
                ];
            }
            _ => unreachable!(),
        }

        for (dy, (br, tk, bk)) in dcms.into_iter().enumerate() {
            for row in [br - 1, br] {
                let y = row.to_idx();
                let tile = &mut self.die[(col, row)];
                let node = &mut tile.nodes[0];
                node.kind = self.db.get_node("INT.IOI");
                node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
                node.naming = self.db.get_node_naming("INT.IOI");
                tile.add_xnode(
                    self.db.get_node("INTF.IOI"),
                    &[&format!("INT_INTERFACE_IOI_X{x}Y{y}")],
                    self.db.get_node_naming("INTF"),
                    &[(col, row)],
                );
            }
            let name = format!("{tk}_X{x}Y{y}", y = br.to_idx());
            let node = self.die[(col, br)].add_xnode(
                self.db.get_node("CMT_DCM"),
                &[&name],
                self.db.get_node_naming(tk),
                &[(col, br - 1), (col, br)],
            );
            node.add_bel(0, format!("DCM_X0Y{y}", y = dy * 2));
            node.add_bel(1, format!("DCM_X0Y{y}", y = dy * 2 + 1));
            self.die[(col, br)].add_xnode(
                self.db.get_node(bk),
                &[&name],
                self.db.get_node_naming(bk),
                &[],
            );
        }

        for (py, (br, tk, out)) in plls.into_iter().enumerate() {
            let row = br - 1;
            let y = row.to_idx();
            let tile = &mut self.die[(col, row)];
            tile.add_xnode(
                self.db.get_node("INTF"),
                &[&format!("INT_INTERFACE_CARRY_X{x}Y{y}")],
                self.db.get_node_naming("INTF"),
                &[(col, row)],
            );
            let row = br;
            let y = row.to_idx();
            let tile = &mut self.die[(col, row)];
            let node = &mut tile.nodes[0];
            node.kind = self.db.get_node("INT.IOI");
            node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
            node.naming = self.db.get_node_naming("INT.IOI");
            tile.add_xnode(
                self.db.get_node("INTF.IOI"),
                &[&format!("INT_INTERFACE_IOI_X{x}Y{y}")],
                self.db.get_node_naming("INTF"),
                &[(col, row)],
            );

            let name = format!("{tk}_X{x}Y{y}", y = br.to_idx());
            let node = self.die[(col, br)].add_xnode(
                self.db.get_node("CMT_PLL"),
                &[&name],
                self.db.get_node_naming(tk),
                &[(col, br - 1), (col, br)],
            );
            node.add_bel(0, format!("PLL_ADV_X0Y{py}"));
            node.add_bel(
                1,
                format!(
                    "TIEOFF_X{x}Y{y}",
                    x = self.tiexlut[col] + 2,
                    y = br.to_idx() * 2 + 1
                ),
            );
            if let Some(out) = out {
                self.die[(col, br)].add_xnode(
                    self.db.get_node("PLL_BUFPLL_OUT"),
                    &[&name],
                    self.db.get_node_naming(out),
                    &[],
                );
            }
        }
    }

    fn fill_gt_bels(node: &mut ExpandedTileNode, gtx: usize, gty: usize) {
        node.add_bel(0, format!("IPAD_X{gtx}Y{y}", y = gty * 8 + 2));
        node.add_bel(1, format!("IPAD_X{gtx}Y{y}", y = gty * 8));
        node.add_bel(2, format!("IPAD_X{gtx}Y{y}", y = gty * 8 + 3));
        node.add_bel(3, format!("IPAD_X{gtx}Y{y}", y = gty * 8 + 1));
        node.add_bel(4, format!("IPAD_X{gtx}Y{y}", y = gty * 8 + 5));
        node.add_bel(5, format!("IPAD_X{gtx}Y{y}", y = gty * 8 + 4));
        node.add_bel(6, format!("IPAD_X{gtx}Y{y}", y = gty * 8 + 7));
        node.add_bel(7, format!("IPAD_X{gtx}Y{y}", y = gty * 8 + 6));
        node.add_bel(8, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 1));
        node.add_bel(9, format!("OPAD_X{gtx}Y{y}", y = gty * 4));
        node.add_bel(10, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 3));
        node.add_bel(11, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 2));
        node.add_bel(12, format!("BUFDS_X{x}Y{y}", x = gtx + 1, y = 2 + gty * 2));
        node.add_bel(
            13,
            format!("BUFDS_X{x}Y{y}", x = gtx + 1, y = 2 + gty * 2 + 1),
        );
        node.add_bel(14, format!("GTPA1_DUAL_X{gtx}Y{gty}"));
    }

    fn fill_gts_holes(&mut self) {
        match self.grid.gts {
            Gts::Single(bc) | Gts::Double(bc, _) | Gts::Quad(bc, _) => {
                let row_gt_mid = self.grid.row_top() - 8;
                let row_gt_bot = row_gt_mid - 8;
                let row_pcie_bot = row_gt_bot - 16;
                self.die.nuke_rect(bc - 6, row_gt_mid, 11, 8);
                self.die.nuke_rect(bc - 4, row_gt_bot, 7, 8);
                self.die.nuke_rect(bc - 1, row_pcie_bot, 3, 16);
                let col_l = bc - 7;
                let col_r = bc + 5;
                let rxl = self.rxlut[col_l] + 6;
                let rxr = self.rxlut[col_r] - 1;
                for dy in 0..8 {
                    let row = row_gt_mid + dy;
                    let ry = self.rylut[row];
                    self.die.fill_term_tile(
                        (col_l, row),
                        "TERM.E",
                        "TERM.E.INTF",
                        format!("INT_RTERM_X{rxl}Y{ry}"),
                    );
                    self.die.fill_term_tile(
                        (col_r, row),
                        "TERM.W",
                        "TERM.W.INTF",
                        format!("INT_LTERM_X{rxr}Y{ry}"),
                    );
                }
                self.holes.push(Rect {
                    col_l: col_l + 1,
                    col_r,
                    row_b: row_gt_mid,
                    row_t: row_gt_mid + 8,
                });
                let col_l = bc - 5;
                let col_r = bc + 3;
                for dy in 0..8 {
                    let row = row_gt_bot + dy;
                    let ry = self.rylut[row];
                    let rxl = self.rxlut[col_l] + 1;
                    let rxr = self.rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    self.fill_intf_rterm((col_l, row), tile_l);
                    self.fill_intf_lterm((col_r, row), tile_r, is_brk);
                }
                self.holes.push(Rect {
                    col_l,
                    col_r: col_r + 1,
                    row_b: row_gt_bot,
                    row_t: row_gt_mid,
                });

                let col_l = bc - 2;
                let col_r = bc + 2;
                for dy in 0..16 {
                    let row = row_pcie_bot + dy;
                    let ry = self.rylut[row];
                    let rxl = self.rxlut[col_l] + 1;
                    let rxr = self.rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    self.fill_intf_rterm((col_l, row), tile_l);
                    self.fill_intf_lterm((col_r, row), tile_r, is_brk);
                }
                self.holes.push(Rect {
                    col_l,
                    col_r: col_r + 1,
                    row_b: row_pcie_bot,
                    row_t: row_gt_bot,
                });
            }
            _ => (),
        }
        match self.grid.gts {
            Gts::Double(_, bc) | Gts::Quad(_, bc) => {
                let row_gt_mid = self.grid.row_top() - 8;
                let row_gt_bot = row_gt_mid - 8;
                self.die.nuke_rect(bc - 4, row_gt_mid, 11, 8);
                self.die.nuke_rect(bc - 2, row_gt_bot, 8, 8);
                let col_l = bc - 5;
                let col_r = bc + 7;
                for dy in 0..8 {
                    let row = row_gt_mid + dy;
                    let ry = self.rylut[row];
                    let rxl = self.rxlut[col_l] + 5;
                    let rxr = self.rxlut[col_r] - 2;
                    self.die.fill_term_tile(
                        (col_l, row),
                        "TERM.E",
                        "TERM.E.INTF",
                        format!("INT_RTERM_X{rxl}Y{ry}"),
                    );
                    self.die.fill_term_tile(
                        (col_r, row),
                        "TERM.W",
                        "TERM.W.INTF",
                        format!("INT_LTERM_X{rxr}Y{ry}"),
                    );
                }
                self.holes.push(Rect {
                    col_l: col_l + 1,
                    col_r,
                    row_b: row_gt_mid,
                    row_t: row_gt_mid + 8,
                });
                let col_l = bc - 3;
                let col_r = bc + 6;
                for dy in 0..8 {
                    let row = row_gt_bot + dy;
                    let ry = self.rylut[row];
                    let rxl = self.rxlut[col_l] + 1;
                    let rxr = self.rxlut[col_r] - 1;
                    let is_brk = dy == 0;
                    let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                    let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                    self.fill_intf_rterm((col_l, row), tile_l);
                    self.fill_intf_lterm((col_r, row), tile_r, is_brk);
                }
                self.holes.push(Rect {
                    col_l,
                    col_r: col_r + 1,
                    row_b: row_gt_bot,
                    row_t: row_gt_mid,
                });
            }
            _ => (),
        }
        if let Gts::Quad(bcl, bcr) = self.grid.gts {
            let row_gt_bot = RowId::from_idx(0);
            let row_gt_mid = RowId::from_idx(8);
            self.die.nuke_rect(bcl - 6, row_gt_bot, 11, 8);
            self.die.nuke_rect(bcl - 4, row_gt_mid, 7, 8);
            self.die.nuke_rect(bcr - 4, row_gt_bot, 11, 8);
            self.die.nuke_rect(bcr - 2, row_gt_mid, 8, 8);
            let col_l = bcl - 7;
            let col_r = bcl + 5;
            for dy in 0..8 {
                let row = row_gt_bot + dy;
                let ry = self.rylut[row];
                let rxl = self.rxlut[col_l] + 6;
                let rxr = self.rxlut[col_r] - 1;
                self.die.fill_term_tile(
                    (col_l, row),
                    "TERM.E",
                    "TERM.E.INTF",
                    format!("INT_RTERM_X{rxl}Y{ry}"),
                );
                self.die.fill_term_tile(
                    (col_r, row),
                    "TERM.W",
                    "TERM.W.INTF",
                    format!("INT_LTERM_X{rxr}Y{ry}"),
                );
            }
            self.holes.push(Rect {
                col_l: col_l + 1,
                col_r,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
            let col_l = bcl - 5;
            let col_r = bcl + 3;
            for dy in 0..8 {
                let row = row_gt_mid + dy;
                let ry = self.rylut[row];
                let rxl = self.rxlut[col_l] + 1;
                let rxr = self.rxlut[col_r] - 1;
                let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                self.fill_intf_rterm((col_l, row), tile_l);
                self.fill_intf_lterm((col_r, row), tile_r, false);
            }
            self.holes.push(Rect {
                col_l,
                col_r: col_r + 1,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });

            let col_l = bcr - 5;
            let col_r = bcr + 7;
            for dy in 0..8 {
                let row = row_gt_bot + dy;
                let ry = self.rylut[row];
                let rxl = self.rxlut[col_l] + 5;
                let rxr = self.rxlut[col_r] - 2;
                self.die.fill_term_tile(
                    (col_l, row),
                    "TERM.E",
                    "TERM.E.INTF",
                    format!("INT_RTERM_X{rxl}Y{ry}"),
                );
                self.die.fill_term_tile(
                    (col_r, row),
                    "TERM.W",
                    "TERM.W.INTF",
                    format!("INT_LTERM_X{rxr}Y{ry}"),
                );
            }
            self.holes.push(Rect {
                col_l: col_l + 1,
                col_r,
                row_b: row_gt_bot,
                row_t: row_gt_mid,
            });
            let col_l = bcr - 3;
            let col_r = bcr + 6;
            for dy in 0..8 {
                let row = row_gt_mid + dy;
                let ry = self.rylut[row];
                let rxl = self.rxlut[col_l] + 1;
                let rxr = self.rxlut[col_r] - 1;
                let tile_l = format!("INT_INTERFACE_RTERM_X{rxl}Y{ry}");
                let tile_r = format!("INT_INTERFACE_LTERM_X{rxr}Y{ry}");
                self.fill_intf_rterm((col_l, row), tile_l);
                self.fill_intf_lterm((col_r, row), tile_r, false);
            }
            self.holes.push(Rect {
                col_l,
                col_r: col_r + 1,
                row_b: row_gt_mid,
                row_t: row_gt_mid + 8,
            });
        }
    }

    fn fill_gts(&mut self) {
        if self.disabled.contains(&DisabledPart::Gtp) {
            return;
        }
        match self.grid.gts {
            Gts::Single(bc) | Gts::Double(bc, _) | Gts::Quad(bc, _) => {
                let row_gt_mid = self.grid.row_top() - 8;
                let row_gt_bot = row_gt_mid - 8;
                let row_pcie_bot = row_gt_bot - 16;

                let col_l = bc - 5;
                let col_r = bc + 3;
                let mut crd = vec![];
                for dy in 0..8 {
                    crd.push((col_l, row_gt_bot + dy));
                }
                for dy in 0..8 {
                    crd.push((col_r, row_gt_bot + dy));
                }
                let x = bc.to_idx();
                let y = row_pcie_bot.to_idx() - 1;
                let name = format!("GTPDUAL_TOP_X{x}Y{y}");
                let name_buf = format!(
                    "BRAM_TOP_TTERM_L_X{x}Y{y}",
                    x = self.rxlut[bc] + 2,
                    y = self.rylut[row_gt_mid + 7] + 1
                );
                let node = self.die[(bc, self.grid.row_tio_outer())].add_xnode(
                    self.db.get_node("GTP"),
                    &[&name, &name_buf],
                    self.db.get_node_naming("GTPDUAL_TOP"),
                    &crd,
                );
                let gty = usize::from(matches!(self.grid.gts, Gts::Quad(_, _)));
                Self::fill_gt_bels(node, 0, gty);

                let col_l = bc - 2;
                let col_r = bc + 2;
                let mut crd = vec![];
                for dy in 0..16 {
                    crd.push((col_l, row_pcie_bot + dy));
                }
                for dy in 0..16 {
                    crd.push((col_r, row_pcie_bot + dy));
                }
                let x = bc.to_idx();
                let y = row_pcie_bot.to_idx() - 1;
                let name = format!("PCIE_TOP_X{x}Y{y}");
                let node = self.die[crd[0]].add_xnode(
                    self.db.get_node("PCIE"),
                    &[&name],
                    self.db.get_node_naming("PCIE"),
                    &crd,
                );
                node.add_bel(0, "PCIE_X0Y0".to_string());
            }
            _ => (),
        }
        match self.grid.gts {
            Gts::Double(_, bc) | Gts::Quad(_, bc) => {
                let row_gt_mid = self.grid.row_top() - 8;
                let row_gt_bot = row_gt_mid - 8;

                let col_l = bc - 3;
                let col_r = bc + 6;
                let mut crd = vec![];
                for dy in 0..8 {
                    crd.push((col_l, row_gt_bot + dy));
                }
                for dy in 0..8 {
                    crd.push((col_r, row_gt_bot + dy));
                }
                let x = bc.to_idx();
                let y = row_gt_bot.to_idx() - 1;
                let name = format!("GTPDUAL_TOP_X{x}Y{y}");
                let name_buf = format!(
                    "BRAM_TOP_TTERM_R_X{x}Y{y}",
                    x = self.rxlut[bc] + 2,
                    y = self.rylut[row_gt_mid + 7] + 1
                );
                let node = self.die[(bc, self.grid.row_tio_outer())].add_xnode(
                    self.db.get_node("GTP"),
                    &[&name, &name_buf],
                    self.db.get_node_naming("GTPDUAL_TOP"),
                    &crd,
                );
                let gty = usize::from(matches!(self.grid.gts, Gts::Quad(_, _)));
                Self::fill_gt_bels(node, 1, gty);
            }
            _ => (),
        }
        if let Gts::Quad(bcl, bcr) = self.grid.gts {
            let row_gt_bot = RowId::from_idx(0);
            let row_gt_mid = RowId::from_idx(8);

            let col_l = bcl - 5;
            let col_r = bcl + 3;
            let mut crd = vec![];
            for dy in 0..8 {
                crd.push((col_l, row_gt_mid + dy));
            }
            for dy in 0..8 {
                crd.push((col_r, row_gt_mid + dy));
            }
            let x = bcl.to_idx();
            let y = row_gt_mid.to_idx() + 8;
            let name = format!("GTPDUAL_BOT_X{x}Y{y}");
            let name_buf = format!(
                "BRAM_BOT_BTERM_L_X{x}Y{y}",
                x = self.rxlut[bcl] + 2,
                y = self.rylut[row_gt_bot] - 1
            );
            let node = self.die[(bcl, self.grid.row_bio_outer())].add_xnode(
                self.db.get_node("GTP"),
                &[&name, &name_buf],
                self.db.get_node_naming("GTPDUAL_BOT"),
                &crd,
            );
            Self::fill_gt_bels(node, 0, 0);

            let col_l = bcr - 3;
            let col_r = bcr + 6;
            let mut crd = vec![];
            for dy in 0..8 {
                crd.push((col_l, row_gt_mid + dy));
            }
            for dy in 0..8 {
                crd.push((col_r, row_gt_mid + dy));
            }
            let x = bcr.to_idx();
            let y = row_gt_mid.to_idx() + 8;
            let name = format!("GTPDUAL_BOT_X{x}Y{y}");
            let name_buf = format!(
                "BRAM_BOT_BTERM_R_X{x}Y{y}",
                x = self.rxlut[bcr] + 2,
                y = self.rylut[row_gt_bot] - 1
            );
            let node = self.die[(bcr, self.grid.row_bio_outer())].add_xnode(
                self.db.get_node("GTP"),
                &[&name, &name_buf],
                self.db.get_node_naming("GTPDUAL_BOT"),
                &crd,
            );
            Self::fill_gt_bels(node, 1, 0);
        }
    }

    fn fill_btterm(&mut self) {
        for (col, &cd) in &self.grid.columns {
            let (btt, ttt) = match cd.kind {
                ColumnKind::Io => ("CNR_BR_BTERM", "CNR_TR_TTERM"),
                ColumnKind::Bram => ("", "RAMB_TOP_TTERM"),
                ColumnKind::Dsp | ColumnKind::DspPlus => ("DSP_INT_BTERM", "DSP_INT_TTERM"),
                _ => {
                    if col == self.grid.col_clk + 1 {
                        ("IOI_BTERM_BUFPLL", "IOI_TTERM_BUFPLL")
                    } else {
                        (
                            if cd.bio == ColumnIoKind::None {
                                "CLB_INT_BTERM"
                            } else {
                                "IOI_BTERM"
                            },
                            "IOI_TTERM",
                        )
                    }
                }
            };
            let rx = self.rxlut[col];
            let ryb = self.rylut[self.grid.row_bio_outer()] - 1;
            let mut row_b = self.grid.row_bio_outer();
            while !self.die[(col, row_b)]
                .nodes
                .iter()
                .any(|x| self.db.nodes.key(x.kind).starts_with("INT"))
            {
                row_b += 1;
            }
            if !btt.is_empty() {
                self.die.fill_term_tile(
                    (col, row_b),
                    "TERM.S",
                    "TERM.S",
                    format!("{btt}_X{rx}Y{ryb}"),
                );
            }

            let ryt = self.rylut[self.grid.row_tio_outer()] + 1;
            let mut row_t = self.grid.row_tio_outer();
            while !self.die[(col, row_t)]
                .nodes
                .iter()
                .any(|x| self.db.nodes.key(x.kind).starts_with("INT"))
            {
                row_t -= 1;
            }
            self.die.fill_term_tile(
                (col, row_t),
                "TERM.N",
                "TERM.N",
                format!("{ttt}_X{rx}Y{ryt}"),
            );
        }
    }

    fn fill_cle(&mut self) {
        let mut sy_base = 2;
        for (col, &cd) in &self.grid.columns {
            if !matches!(
                cd.kind,
                ColumnKind::CleXL | ColumnKind::CleXM | ColumnKind::CleClk
            ) {
                continue;
            }
            if self.disabled.contains(&DisabledPart::ClbColumn(col)) {
                continue;
            }
            let tb = &self.die[(col, RowId::from_idx(0))];
            if !tb.nodes.is_empty() && cd.bio == ColumnIoKind::None {
                sy_base = 0;
                break;
            }
        }
        let mut sx = 0;
        for (col, &cd) in &self.grid.columns {
            if !matches!(
                cd.kind,
                ColumnKind::CleXL | ColumnKind::CleXM | ColumnKind::CleClk
            ) {
                continue;
            }
            if self.disabled.contains(&DisabledPart::ClbColumn(col)) {
                continue;
            }
            'a: for row in self.die.rows() {
                let tile = &mut self.die[(col, row)];
                if (row == self.grid.row_bio_outer() || row == self.grid.row_bio_inner())
                    && cd.bio != ColumnIoKind::None
                {
                    continue;
                }
                if (row == self.grid.row_tio_outer() || row == self.grid.row_tio_inner())
                    && cd.tio != ColumnIoKind::None
                {
                    continue;
                }
                if cd.kind == ColumnKind::CleClk {
                    if row == self.grid.row_clk() {
                        continue;
                    }
                    if matches!(row.to_idx() % 16, 7 | 8) && row != self.grid.row_clk() - 1 {
                        continue;
                    }
                }
                for hole in &self.holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let sy = row.to_idx() - sy_base;
                let kind = if cd.kind == ColumnKind::CleXM {
                    "CLEXM"
                } else {
                    "CLEXL"
                };
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("{kind}_X{x}Y{y}");
                let node = tile.add_xnode(
                    self.db.get_node(kind),
                    &[&name],
                    self.db.get_node_naming(kind),
                    &[(col, row)],
                );
                node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                node.add_bel(1, format!("SLICE_X{sx1}Y{sy}", sx1 = sx + 1));
            }
            sx += 2;
        }
    }

    fn fill_bram(&mut self) {
        let mut bx = 0;
        let mut bby = 0;
        'a: for reg in 0..(self.grid.rows.len() as u32 / 16) {
            for (col, &cd) in &self.grid.columns {
                if cd.kind == ColumnKind::Bram
                    && !self.disabled.contains(&DisabledPart::BramRegion(col, reg))
                {
                    break 'a;
                }
            }
            bby += 8;
        }
        for (col, &cd) in &self.grid.columns {
            if cd.kind != ColumnKind::Bram {
                continue;
            }
            'a: for row in self.die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                let reg = row.to_idx() as u32 / 16;
                if self.disabled.contains(&DisabledPart::BramRegion(col, reg)) {
                    continue;
                }
                let tile = &mut self.die[(col, row)];
                for hole in &self.holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let by = row.to_idx() / 2 - bby;
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("BRAMSITE2_X{x}Y{y}");
                let node = tile.add_xnode(
                    self.db.get_node("BRAM"),
                    &[&name],
                    self.db.get_node_naming("BRAM"),
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
                node.add_bel(0, format!("RAMB16_X{bx}Y{by}"));
                node.add_bel(1, format!("RAMB8_X{bx}Y{by}"));
                node.add_bel(2, format!("RAMB8_X{bx}Y{by}", by = by + 1));
            }
            bx += 1;

            let lr = if col < self.grid.col_clk { 'L' } else { 'R' };
            let rx = self.rxlut[col];

            let row = self.grid.row_bio_outer();
            let ry = self.rylut[row];
            let name = format!("BRAM_BOT_BTERM_{lr}_X{rx}Y{ry}", rx = rx + 2, ry = ry - 1);
            self.die[(col, row)].add_xnode(
                self.db.get_node("PCI_CE_H_BUF"),
                &[&name],
                self.db.get_node_naming("PCI_CE_H_BUF_BRAM"),
                &[],
            );

            let row = self.grid.row_tio_outer();
            let ry = self.rylut[row];
            let name = format!("BRAM_TOP_TTERM_{lr}_X{rx}Y{ry}", rx = rx + 2, ry = ry + 1);
            self.die[(col, row)].add_xnode(
                self.db.get_node("PCI_CE_H_BUF"),
                &[&name],
                self.db.get_node_naming("PCI_CE_H_BUF_BRAM"),
                &[],
            );
        }
    }

    fn fill_dsp(&mut self) {
        let mut dx = 0;
        let mut bdy = 0;
        'a: for reg in 0..(self.grid.rows.len() as u32 / 16) {
            for (col, &cd) in &self.grid.columns {
                if matches!(cd.kind, ColumnKind::Dsp | ColumnKind::DspPlus)
                    && !self.disabled.contains(&DisabledPart::DspRegion(col, reg))
                {
                    break 'a;
                }
            }
            bdy += 4;
        }
        for (col, &cd) in &self.grid.columns {
            if !matches!(cd.kind, ColumnKind::Dsp | ColumnKind::DspPlus) {
                continue;
            }
            'a: for row in self.die.rows() {
                if row.to_idx() % 4 != 0 {
                    continue;
                }
                let reg = row.to_idx() as u32 / 16;
                if self.disabled.contains(&DisabledPart::DspRegion(col, reg)) {
                    continue;
                }
                if cd.kind == ColumnKind::DspPlus {
                    if row.to_idx() >= self.grid.rows.len() - 16 {
                        continue;
                    }
                    if matches!(self.grid.gts, Gts::Quad(_, _)) && row.to_idx() < 16 {
                        continue;
                    }
                }
                let tile = &mut self.die[(col, row)];
                for hole in &self.holes {
                    if hole.contains(col, row) {
                        continue 'a;
                    }
                }
                let dy = row.to_idx() / 4 - bdy;
                let x = col.to_idx();
                let y = row.to_idx();
                let name = format!("MACCSITE2_X{x}Y{y}");
                let node = tile.add_xnode(
                    self.db.get_node("DSP"),
                    &[&name],
                    self.db.get_node_naming("DSP"),
                    &[(col, row), (col, row + 1), (col, row + 2), (col, row + 3)],
                );
                node.add_bel(0, format!("DSP48_X{dx}Y{dy}"));
            }
            dx += 1;

            let lr = if col < self.grid.col_clk { 'L' } else { 'R' };
            let rx = self.rxlut[col];

            let row = self.grid.row_bio_outer();
            let ry = self.rylut[row];
            let name = format!("DSP_BOT_BTERM_{lr}_X{rx}Y{ry}", rx = rx + 2, ry = ry - 1);
            self.die[(col, row)].add_xnode(
                self.db.get_node("PCI_CE_H_BUF"),
                &[&name],
                self.db.get_node_naming("PCI_CE_H_BUF_DSP"),
                &[],
            );

            let row = self.grid.row_tio_outer();
            let ry = self.rylut[row];
            let name = format!("DSP_TOP_TTERM_{lr}_X{rx}Y{ry}", rx = rx + 2, ry = ry + 1);
            self.die[(col, row)].add_xnode(
                self.db.get_node("PCI_CE_H_BUF"),
                &[&name],
                self.db.get_node_naming("PCI_CE_H_BUF_DSP"),
                &[],
            );
        }
    }

    fn fill_hclk_fold(&mut self) {
        if let Some((col_l, col_r)) = self.grid.cols_clk_fold {
            for col in [col_l, col_r] {
                for row in self.die.rows() {
                    if row.to_idx() % 16 != 8 {
                        continue;
                    }
                    let x = col.to_idx();
                    let rx = self.rxlut[col];
                    let y = row.to_idx();
                    let ry = self.rylut[row];
                    let mut name = format!("DSP_HCLK_GCLK_FOLD_X{x}Y{y}", y = y - 1);
                    let mut naming = "DSP_HCLK_GCLK_FOLD";
                    if let Gts::Double(_, cr) | Gts::Quad(_, cr) = self.grid.gts {
                        if col == cr + 6 && row == self.grid.row_top() - 8 {
                            name = format!("GTPDUAL_DSP_FEEDTHRU_X{rx}Y{ry}", rx = rx + 1);
                            naming = "GTPDUAL_DSP_FEEDTHRU";
                        }
                    }
                    if let Gts::Quad(cl, cr) = self.grid.gts {
                        if col == cl - 6 && row == self.grid.row_bio_outer() + 8 {
                            name = format!("DSP_HCLK_GCLK_FOLD_X{x}Y{y}");
                        }
                        if col == cr + 6 && row == self.grid.row_bio_outer() + 8 {
                            name = format!("GTPDUAL_DSP_FEEDTHRU_X{x}Y{y}");
                            naming = "GTPDUAL_DSP_FEEDTHRU";
                        }
                    }
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("HCLK_H_MIDBUF"),
                        &[&name],
                        self.db.get_node_naming(naming),
                        &[],
                    );
                }
            }
        }
    }

    fn fill_hclk(&mut self) {
        let fold = if self.grid.cols_clk_fold.is_some() {
            "_FOLD"
        } else {
            ""
        };
        let naming = if self.grid.cols_clk_fold.is_some() {
            "HCLK_FOLD"
        } else {
            "HCLK"
        };
        for col in self.die.cols() {
            for row in self.die.rows() {
                let crow = if row.to_idx() % 16 < 8 {
                    self.grid.row_hclk(row) - 1
                } else {
                    self.grid.row_hclk(row)
                };
                self.die[(col, row)].clkroot = (col, crow);

                if row.to_idx() % 16 == 8 {
                    let x = col.to_idx();
                    let y = row.to_idx();
                    let mut name = match self.grid.columns[col].kind {
                        ColumnKind::CleXL | ColumnKind::CleClk => {
                            format!("HCLK_CLB_XL_INT{fold}_X{x}Y{y}", y = y - 1)
                        }
                        ColumnKind::CleXM => format!("HCLK_CLB_XM_INT{fold}_X{x}Y{y}", y = y - 1),
                        ColumnKind::Bram => format!("BRAM_HCLK_FEEDTHRU{fold}_X{x}Y{y}", y = y - 1),
                        ColumnKind::Dsp | ColumnKind::DspPlus => {
                            format!("DSP_INT_HCLK_FEEDTHRU{fold}_X{x}Y{y}", y = y - 1)
                        }
                        ColumnKind::Io => {
                            if col == self.grid.col_lio() {
                                format!("HCLK_IOIL_INT{fold}_X{x}Y{y}", y = y - 1)
                            } else {
                                format!("HCLK_IOIR_INT{fold}_X{x}Y{y}", y = y - 1)
                            }
                        }
                    };
                    if self.die[(col, row)].nodes.is_empty()
                        && self.die[(col, row - 1)].nodes.is_empty()
                    {
                        continue;
                    }
                    if let Gts::Single(cl) | Gts::Double(cl, _) | Gts::Quad(cl, _) = self.grid.gts {
                        if col == cl + 2 && row == self.grid.row_top() - 24 {
                            name = format!("HCLK_CLB_XM_INT{fold}_X{x}Y{y}", y = y - 1);
                        }
                        if col == cl + 3 && row == self.grid.row_top() - 8 {
                            name = format!("HCLK_CLB_XL_INT{fold}_X{x}Y{y}", y = y - 1);
                        }
                    }
                    if let Gts::Double(_, cr) | Gts::Quad(_, cr) = self.grid.gts {
                        if col == cr + 6 && row == self.grid.row_top() - 8 {
                            name = format!("HCLK_CLB_XL_INT{fold}_X{x}Y{y}", y = y - 1);
                        }
                    }
                    if let Gts::Quad(cl, cr) = self.grid.gts {
                        if col == cl - 6 && row == self.grid.row_bio_outer() + 8 {
                            name = format!("DSP_INT_HCLK_FEEDTHRU{fold}_X{x}Y{y}");
                        }
                        if (col == cl - 5
                            || col == cl + 3
                            || col == cl + 4
                            || col == cr - 3
                            || col == cr + 6)
                            && row == self.grid.row_bio_outer() + 8
                        {
                            name = format!("HCLK_CLB_XL_INT{fold}_X{x}Y{y}");
                        }
                        if col == cr - 4 && row == self.grid.row_bio_outer() + 8 {
                            name = format!("HCLK_CLB_XM_INT{fold}_X{x}Y{y}");
                        }
                    }
                    self.die[(col, row)].add_xnode(
                        self.db.get_node("HCLK"),
                        &[&name],
                        self.db.get_node_naming(naming),
                        &[(col, row - 1), (col, row)],
                    );
                }
            }
        }
    }

    fn fill_ioi(&mut self, crd: Coord, naming: &str) {
        let x = crd.0.to_idx();
        let y = crd.1.to_idx();
        let tile = &mut self.die[crd];
        let node = &mut tile.nodes[0];
        let def_rt = NodeRawTileId::from_idx(0);
        node.kind = self.db.get_node("INT.IOI");
        let is_brk = y % 16 == 0 && crd.1 != self.grid.row_clk() && y != 0;
        if !is_brk {
            if crd.0 == self.grid.col_lio() {
                node.names[def_rt] = format!("LIOI_INT_X{x}Y{y}");
            } else {
                node.names[def_rt] = format!("IOI_INT_X{x}Y{y}");
            }
        }
        node.naming = self
            .db
            .get_node_naming(if is_brk { "INT.IOI.BRK" } else { "INT.IOI" });
        let name = format!("{naming}_X{x}Y{y}");
        tile.add_xnode(
            self.db.get_node("INTF.IOI"),
            &[&name],
            self.db.get_node_naming("INTF.IOI"),
            &[crd],
        );
        let node = tile.add_xnode(
            self.db.get_node("IOI"),
            &[&name],
            self.db.get_node_naming(naming),
            &[crd],
        );
        let iox = self.ioxlut[crd.0];
        let ioy = self.ioylut[crd.1];
        let tiex = self.tiexlut[crd.0] + 1;
        node.add_bel(0, format!("ILOGIC_X{iox}Y{y}", y = ioy * 2 + 1));
        node.add_bel(1, format!("ILOGIC_X{iox}Y{y}", y = ioy * 2));
        node.add_bel(2, format!("OLOGIC_X{iox}Y{y}", y = ioy * 2 + 1));
        node.add_bel(3, format!("OLOGIC_X{iox}Y{y}", y = ioy * 2));
        node.add_bel(4, format!("IODELAY_X{iox}Y{y}", y = ioy * 2 + 1));
        node.add_bel(5, format!("IODELAY_X{iox}Y{y}", y = ioy * 2));
        node.add_bel(6, format!("TIEOFF_X{tiex}Y{y}", y = y * 2));
    }

    fn fill_iob(&mut self, crd: Coord, tk: &str, naming: &str) {
        let x = crd.0.to_idx();
        let mut y = crd.1.to_idx();
        if tk.starts_with('T') {
            y = self.grid.row_tio_outer().to_idx();
        }
        if tk.starts_with('B') {
            y = 0;
        }
        let tile = &mut self.die[crd];
        let name = format!("{tk}_X{x}Y{y}");
        let node = tile.add_xnode(
            self.db.get_node("IOB"),
            &[&name],
            self.db.get_node_naming(naming),
            &[],
        );
        node.add_bel(0, format!("PAD{i}", i = self.pad_cnt));
        node.add_bel(1, format!("PAD{i}", i = self.pad_cnt + 1));
        self.pad_cnt += 2;
        self.bonded_ios.push((crd, BelId::from_idx(0)));
        self.bonded_ios.push((crd, BelId::from_idx(1)));
    }

    fn fill_intf_rterm(&mut self, crd: Coord, name: String) {
        self.die
            .fill_term_tile(crd, "TERM.E", "TERM.E.INTF", name.clone());
        let tile = &mut self.die[crd];
        tile.nodes.truncate(1);
        tile.add_xnode(
            self.db.get_node("INTF"),
            &[&name],
            self.db.get_node_naming("INTF.RTERM"),
            &[crd],
        );
    }

    fn fill_intf_lterm(&mut self, crd: Coord, name: String, is_brk: bool) {
        self.die
            .fill_term_tile(crd, "TERM.W", "TERM.W.INTF", name.clone());
        let tile = &mut self.die[crd];
        tile.nodes.truncate(1);
        tile.nodes[0].naming =
            self.db
                .get_node_naming(if is_brk { "INT.TERM.BRK" } else { "INT.TERM" });
        tile.add_xnode(
            self.db.get_node("INTF"),
            &[&name],
            self.db.get_node_naming("INTF.LTERM"),
            &[crd],
        );
    }

    fn fill_frame_info(&mut self) {
        let regs = self.grid.rows.len() / 16;
        for reg in 0..regs {
            for (col, cd) in &self.grid.columns {
                // XXX
                //self.col_frame.push(self.frame_info.len());
                let width = match cd.kind {
                    ColumnKind::CleXL => 30,
                    ColumnKind::CleXM => 31,
                    ColumnKind::CleClk => 31,
                    ColumnKind::Bram => 25,
                    ColumnKind::Dsp => 24,
                    ColumnKind::DspPlus => 31,
                    ColumnKind::Io => 30,
                };
                for minor in 0..width {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: reg as i32,
                            major: col.to_idx() as u32,
                            minor,
                        },
                    });
                }
            }
        }
    }
}

impl Grid {
    pub fn expand_grid<'a>(
        &'a self,
        db: &'a IntDb,
        disabled: &'a BTreeSet<DisabledPart>,
    ) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        egrid.tie_kind = Some("TIEOFF".to_string());
        egrid.tie_pin_pullup = Some("KEEP1".to_string());
        egrid.tie_pin_gnd = Some("HARD0".to_string());
        egrid.tie_pin_vcc = Some("HARD1".to_string());
        let (_, die) = egrid.add_die(self.columns.len(), self.rows.len());

        let mut expander = Expander {
            grid: self,
            db,
            disabled,
            die,
            rxlut: EntityVec::new(),
            rylut: EntityVec::new(),
            tiexlut: EntityVec::new(),
            ioxlut: EntityVec::new(),
            ioylut: EntityVec::new(),
            pad_cnt: 1,
            bonded_ios: vec![],
            holes: vec![],
            frame_info: vec![],
            iob_frame_len: 0,
        };

        expander.fill_rxlut();
        expander.fill_rylut();
        expander.fill_ioxlut();
        expander.fill_ioylut();
        expander.fill_tiexlut();

        expander.fill_int();
        expander.fill_tio();
        expander.fill_rio();
        expander.fill_bio();
        expander.fill_lio();
        expander.fill_mcb();
        expander.fill_pcilogic();
        expander.fill_spine();
        expander.fill_cmts();
        expander.fill_gts_holes();
        expander.fill_btterm();
        expander.die.fill_main_passes();
        expander.fill_gts();
        expander.fill_bram();
        expander.fill_dsp();
        expander.fill_cle();
        expander.fill_hclk_fold();
        expander.fill_hclk();
        expander.fill_frame_info();
        // XXX compute iob frame data

        let bonded_ios = expander.bonded_ios;

        // XXX fill frame_info
        let die_bs_geom = DieBitstreamGeom {
            frame_len: 1040,
            frame_info: expander.frame_info,
            bram_cols: self
                .columns
                .values()
                .filter(|x| x.kind == ColumnKind::Bram)
                .count(),
            bram_regs: self.rows.len() / 16,
            iob_frame_len: expander.iob_frame_len,
        };
        let bs_geom = BitstreamGeom {
            kind: DeviceKind::Spartan6,
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![expander.die.die],
        };

        ExpandedDevice {
            grid: self,
            disabled,
            egrid,
            bonded_ios,
            bs_geom,
        }
    }
}
