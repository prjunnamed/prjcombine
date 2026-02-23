use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, RowId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{
    chip::{Chip, ColumnKind, DisabledPart},
    defs::{bslots, virtex6::tcls},
    expanded::ExpandedDevice,
};

use crate::ExpandedNamedDevice;

struct Namer<'a> {
    ngrid: ExpandedGridNaming<'a>,
    chip: &'a Chip,
    tiexlut: EntityVec<ColId, usize>,
    rxlut: EntityVec<ColId, usize>,
}

impl Namer<'_> {
    fn fill_rxlut(&mut self) {
        let mut rx = 0;
        for (col, &kind) in &self.chip.columns {
            if self.chip.cols_vbrk.contains(&col) {
                rx += 1;
            }
            self.rxlut.push(rx);
            match kind {
                ColumnKind::ClbLL | ColumnKind::ClbLM => rx += 2,
                ColumnKind::Bram | ColumnKind::Dsp => rx += 3,
                ColumnKind::Io => {
                    if col.to_idx() == 0 {
                        rx += 5;
                    } else {
                        rx += 4;
                    }
                }
                ColumnKind::Gt => rx += 4,
                ColumnKind::Cfg => rx += 4,
                _ => unreachable!(),
            }
        }
    }

    fn fill_tiexlut(&mut self) {
        let mut tie_x = 0;
        for &kind in self.chip.columns.values() {
            self.tiexlut.push(tie_x);
            tie_x += 1;
            if kind == ColumnKind::Dsp {
                tie_x += 1;
            }
        }
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);

    ngrid.tie_kind = Some("TIEOFF".to_string());
    ngrid.tie_pin_gnd = Some("HARD0".to_string());
    ngrid.tie_pin_vcc = Some("HARD1".to_string());

    let clb_grid = ngrid.bel_grid(|_, name, _| name == "CLBLL" || name == "CLBLM");
    let bram_grid = ngrid.bel_grid(|_, name, _| name == "BRAM");
    let dsp_grid = ngrid.bel_grid(|_, name, _| name == "DSP");
    let io_grid = ngrid.bel_grid(|_, name, _| name == "IO");
    let cmt_grid = ngrid.bel_grid(|_, name, _| name == "CMT");
    let emac_grid = ngrid.bel_grid(|_, name, _| name == "EMAC");
    let pcie_grid = ngrid.bel_grid(|_, name, _| name == "PCIE");
    let gt_grid = ngrid.bel_grid(|_, name, _| name == "GTX" || name == "GTH");
    let gth_grid = ngrid.bel_grid(|_, name, _| name == "GTH");
    let pmvbram_grid = ngrid.bel_grid(|_, name, _| name == "PMVBRAM");
    let pmviob_grid = ngrid.bel_grid(|_, name, _| name == "PMVIOB");

    let mut namer = Namer {
        ngrid,
        chip: edev.chips[DieId::from_idx(0)],
        tiexlut: EntityVec::new(),
        rxlut: EntityVec::new(),
    };

    namer.fill_tiexlut();
    namer.fill_rxlut();

    for (tcrd, tile) in edev.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;

        let chip = edev.chips[cell.die];
        let kind = edev.db.tile_classes.key(tile.class);
        let x = col.to_idx();
        let y = row.to_idx();
        match tile.class {
            tcls::INT => {
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "INT", [format!("INT_X{x}Y{y}")]);
                let tie_x = namer.tiexlut[col];
                ntile.tie_name = Some(format!("TIEOFF_X{tie_x}Y{y}"));
            }
            tcls::INTF => {
                if chip.columns[col] == ColumnKind::Io && col < edev.col_cfg {
                    namer.ngrid.name_tile(
                        tcrd,
                        "INTF_IOI_L",
                        [format!("IOI_L_INT_INTERFACE_X{x}Y{y}")],
                    );
                } else {
                    namer
                        .ngrid
                        .name_tile(tcrd, "INTF", [format!("INT_INTERFACE_X{x}Y{y}")]);
                }
            }
            tcls::INTF_DELAY => {
                if chip.columns[col] == ColumnKind::Gt {
                    if col.to_idx() == 0 {
                        namer.ngrid.name_tile(
                            tcrd,
                            "INTF_GT_L",
                            [format!("GT_L_INT_INTERFACE_X{x}Y{y}")],
                        );
                    } else {
                        namer.ngrid.name_tile(
                            tcrd,
                            "INTF_GTX",
                            [format!("GTX_INT_INTERFACE_X{x}Y{y}")],
                        );
                    }
                } else {
                    let hard = chip.col_hard.as_ref().unwrap();
                    if col == hard.col {
                        namer.ngrid.name_tile(
                            tcrd,
                            "INTF_EMAC",
                            [format!("EMAC_INT_INTERFACE_X{x}Y{y}")],
                        );
                    } else if col == hard.col - 3 {
                        namer.ngrid.name_tile(
                            tcrd,
                            "INTF_PCIE_L",
                            [format!("PCIE_INT_INTERFACE_L_X{x}Y{y}")],
                        );
                    } else if col == hard.col - 2 {
                        namer.ngrid.name_tile(
                            tcrd,
                            "INTF_PCIE_R",
                            [format!("PCIE_INT_INTERFACE_R_X{x}Y{y}")],
                        );
                    } else {
                        unreachable!()
                    }
                }
            }
            tcls::HCLK => {
                let mut naming = "HCLK";
                let mut name = format!("HCLK_X{x}Y{y}", y = y - 1);
                if col == chip.cols_qbuf.unwrap().0 || col == chip.cols_qbuf.unwrap().1 {
                    naming = "HCLK_QBUF";
                    name = format!("HCLK_QBUF_X{x}Y{y}", y = y - 1);
                }
                if edev.in_int_hole(cell.delta(0, -1)) {
                    name = format!("HCLK_X{x}Y{y}");
                }
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                ntile.add_bel(bslots::GLOBALSIG, format!("GLOBALSIG_X{x}Y{y}", y = y / 40));
            }
            tcls::CLBLL | tcls::CLBLM => {
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [format!("{kind}_X{x}Y{y}")]);
                let sx0 = clb_grid.xlut[col] * 2;
                let sx1 = clb_grid.xlut[col] * 2 + 1;
                let sy = clb_grid.ylut[row];
                ntile.add_bel(bslots::SLICE[0], format!("SLICE_X{sx0}Y{sy}"));
                ntile.add_bel(bslots::SLICE[1], format!("SLICE_X{sx1}Y{sy}"));
            }
            tcls::BRAM => {
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [format!("BRAM_X{x}Y{y}")]);
                let bx = bram_grid.xlut[col];
                let by = bram_grid.ylut[row];
                ntile.add_bel(bslots::BRAM_F, format!("RAMB36_X{bx}Y{by}"));
                ntile.add_bel(bslots::BRAM_H[0], format!("RAMB18_X{bx}Y{y}", y = by * 2));
                ntile.add_bel(
                    bslots::BRAM_H[1],
                    format!("RAMB18_X{bx}Y{y}", y = by * 2 + 1),
                );
            }
            tcls::PMVBRAM => {
                let hy = if edev.in_int_hole(cell.delta(0, -1)) {
                    y
                } else {
                    y - 1
                };
                let name = format!("HCLK_BRAM_X{x}Y{hy}");
                let name_bram0 = format!("BRAM_X{x}Y{y}");
                let name_bram1 = format!("BRAM_X{x}Y{y}", y = y + 5);
                let name_bram2 = format!("BRAM_X{x}Y{y}", y = y + 10);
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    "PMVBRAM",
                    [name, name_bram0, name_bram1, name_bram2],
                );
                let px = pmvbram_grid.xlut[col];
                let py = pmvbram_grid.ylut[row];
                ntile.add_bel(bslots::PMVBRAM, format!("PMVBRAM_X{px}Y{py}"));
            }
            tcls::DSP => {
                let ntile = namer.ngrid.name_tile(tcrd, kind, [format!("DSP_X{x}Y{y}")]);
                let dx = dsp_grid.xlut[col];
                let dy0 = dsp_grid.ylut[row] * 2;
                let dy1 = dsp_grid.ylut[row] * 2 + 1;
                ntile.add_bel(bslots::DSP[0], format!("DSP48_X{dx}Y{dy0}"));
                ntile.add_bel(bslots::DSP[1], format!("DSP48_X{dx}Y{dy1}"));
                let tx = namer.tiexlut[col] + 1;
                ntile.add_bel(bslots::TIEOFF_DSP, format!("TIEOFF_X{tx}Y{y}"));
            }
            tcls::IO => {
                let naming = if col < edev.col_cfg { "LIOI" } else { "RIOI" };
                let iob_tk = if col < edev.col_cfg {
                    if Some(col) == edev.col_io_w || edev.col_io_w.is_none() {
                        "LIOB"
                    } else {
                        "LIOB_FT"
                    }
                } else {
                    "RIOB"
                };
                let name_ioi = format!("{naming}_X{x}Y{y}");
                let name_iob = format!("{iob_tk}_X{x}Y{y}");
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name_ioi, name_iob]);
                let iox = io_grid.xlut[col];
                let ioy0 = io_grid.ylut[row] * 2;
                let ioy1 = io_grid.ylut[row] * 2 + 1;
                ntile.add_bel(bslots::ILOGIC[0], format!("ILOGIC_X{iox}Y{ioy0}"));
                ntile.add_bel(bslots::ILOGIC[1], format!("ILOGIC_X{iox}Y{ioy1}"));
                ntile.add_bel(bslots::OLOGIC[0], format!("OLOGIC_X{iox}Y{ioy0}"));
                ntile.add_bel(bslots::OLOGIC[1], format!("OLOGIC_X{iox}Y{ioy1}"));
                ntile.add_bel(bslots::IODELAY[0], format!("IODELAY_X{iox}Y{ioy0}"));
                ntile.add_bel(bslots::IODELAY[1], format!("IODELAY_X{iox}Y{ioy1}"));
                ntile.add_bel(bslots::IOB[0], format!("IOB_X{iox}Y{ioy0}"));
                ntile.add_bel(bslots::IOB[1], format!("IOB_X{iox}Y{ioy1}"));
            }
            tcls::HCLK_IO => {
                let (naming, hclk_tk, ioi_tk) = if Some(col) == edev.col_io_w {
                    ("HCLK_IO_OL", "HCLK_OUTER_IOI", "LIOI")
                } else if Some(col) == edev.col_io_iw {
                    ("HCLK_IO_IL", "HCLK_INNER_IOI", "LIOI")
                } else if Some(col) == edev.col_io_ie {
                    ("HCLK_IO_IR", "HCLK_INNER_IOI", "RIOI")
                } else if Some(col) == edev.col_io_e {
                    ("HCLK_IO_OR", "HCLK_OUTER_IOI", "RIOI")
                } else {
                    unreachable!()
                };
                let hx = if col < edev.col_cfg && x != 0 {
                    x - 1
                } else {
                    x
                };
                let name = format!("{hclk_tk}_X{hx}Y{y}", y = y - 1);
                let name_ioi_s = format!("{ioi_tk}_X{x}Y{y}", y = y - 2);
                let name_ioi_n = format!("{ioi_tk}_X{x}Y{y}");
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, naming, [name, name_ioi_s, name_ioi_n]);
                let iox = io_grid.xlut[col];
                let hy = row.to_idx() / 40;
                ntile.add_bel(
                    bslots::BUFIO[0],
                    format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 2),
                );
                ntile.add_bel(
                    bslots::BUFIO[1],
                    format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 3),
                );
                ntile.add_bel(bslots::BUFIO[2], format!("BUFIODQS_X{iox}Y{y}", y = hy * 4));
                ntile.add_bel(
                    bslots::BUFIO[3],
                    format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 1),
                );
                ntile.add_bel(bslots::BUFR[0], format!("BUFR_X{iox}Y{y}", y = hy * 2 + 1));
                ntile.add_bel(bslots::BUFR[1], format!("BUFR_X{iox}Y{y}", y = hy * 2));
                ntile.add_bel_multi(
                    bslots::HCLK_IO_INT,
                    [
                        format!("BUFO_X{iox}Y{y}", y = hy * 2 + 1),
                        format!("BUFO_X{iox}Y{y}", y = hy * 2),
                    ],
                );
                ntile.add_bel(bslots::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{hy}"));
                ntile.add_bel(bslots::DCI, format!("DCI_X{iox}Y{hy}"));
            }
            tcls::CMT => {
                let naming = if row < chip.row_bufg() {
                    "CMT.BOT"
                } else {
                    "CMT.TOP"
                };
                let bt = if row < chip.row_bufg() { "BOT" } else { "TOP" };
                let (xw, xe) = chip.cols_qbuf.unwrap();
                let xw = xw.to_idx();
                let xe = xe.to_idx();
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    naming,
                    [
                        format!("CMT_X{x}Y{y}", y = y - 9),
                        format!("CMT_X{x}Y{y}", y = y + 9),
                        format!("HCLK_CMT_{bt}_X{x}Y{y}", y = y - 1),
                        format!("HCLK_QBUF_X{xw}Y{y}", y = y - 1),
                        format!("HCLK_QBUF_X{xe}Y{y}", y = y - 1),
                    ],
                );
                let bx = cmt_grid.xlut[col];
                let by = cmt_grid.ylut[row];
                for (i, slots) in [bslots::BUFHCE_W, bslots::BUFHCE_E].into_iter().enumerate() {
                    for j in 0..12 {
                        ntile.add_bel(slots[j], format!("BUFHCE_X{i}Y{y}", y = by * 12 + j));
                    }
                }
                ntile.add_bel(bslots::PLL[0], format!("MMCM_ADV_X{bx}Y{y}", y = by * 2));
                ntile.add_bel(
                    bslots::PLL[1],
                    format!("MMCM_ADV_X{bx}Y{y}", y = by * 2 + 1),
                );
                ntile.add_bel(bslots::PPR_FRAME, format!("PPR_FRAME_X{bx}Y{by}"));
            }
            tcls::PMVIOB => {
                let naming = if row < chip.row_bufg() {
                    "CMT_PMVA_BELOW"
                } else {
                    "CMT_PMVA"
                };
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, naming, [format!("{naming}_X{x}Y{y}")]);
                let bx = pmviob_grid.xlut[col];
                let by = pmviob_grid.ylut[row];
                ntile.add_bel(bslots::PMVIOB_CLK, format!("PMVIOB_X{bx}Y{by}"));
            }
            tcls::CMT_BUFG_S => {
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        format!("CMT_BUFG_BOT_X{x}Y{y}"),
                        format!("CMT_X{x}Y{y}", y = y - 9),
                    ],
                );
                for i in 0..16 {
                    ntile.add_bel(bslots::BUFGCTRL[i], format!("BUFGCTRL_X0Y{i}"));
                }
            }
            tcls::CMT_BUFG_N => {
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        format!("CMT_BUFG_TOP_X{x}Y{y}"),
                        format!("CMT_X{x}Y{y}", y = y + 11),
                    ],
                );
                for i in 0..16 {
                    ntile.add_bel(
                        bslots::BUFGCTRL[16 + i],
                        format!("BUFGCTRL_X0Y{y}", y = i + 16),
                    );
                }
            }
            tcls::EMAC => {
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [format!("EMAC_X{x}Y{y}")]);
                let bx = emac_grid.xlut[col];
                let by = emac_grid.ylut[row];
                ntile.add_bel(bslots::EMAC, format!("TEMAC_X{bx}Y{by}"));
            }
            tcls::PCIE => {
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    kind,
                    [format!("PCIE_X{x}Y{y}", x = x + 1, y = y + 10)],
                );
                let bx = pcie_grid.xlut[col];
                let by = pcie_grid.ylut[row];
                ntile.add_bel(bslots::PCIE, format!("PCIE_X{bx}Y{by}"));
            }
            tcls::CFG => {
                let row_b: RowId = row - 40;
                let ry = row_b.to_idx() + 11 + row_b.to_idx() / 20;
                let rx = namer.rxlut[edev.col_cfg] - 2;
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    "CFG",
                    [
                        format!("CFG_CENTER_0_X{rx}Y{ry}"),
                        format!("CFG_CENTER_1_X{rx}Y{ry}", ry = ry + 21),
                        format!("CFG_CENTER_2_X{rx}Y{ry}", ry = ry + 42),
                        format!("CFG_CENTER_3_X{rx}Y{ry}", ry = ry + 63),
                    ],
                );
                let ipx = if edev.col_gt_w.is_some() { 1 } else { 0 };
                let mut ipy = 0;
                if !chip.cols_gt.is_empty() {
                    ipy += 6;
                    for reg in chip.regs() {
                        if reg < chip.reg_cfg && !edev.disabled.contains(&DisabledPart::GtxRow(reg))
                        {
                            ipy += 24;
                        }
                    }
                };
                ntile.add_bel(bslots::BSCAN[0], "BSCAN_X0Y0".to_string());
                ntile.add_bel(bslots::BSCAN[1], "BSCAN_X0Y1".to_string());
                ntile.add_bel(bslots::BSCAN[2], "BSCAN_X0Y2".to_string());
                ntile.add_bel(bslots::BSCAN[3], "BSCAN_X0Y3".to_string());
                ntile.add_bel(bslots::ICAP[0], "ICAP_X0Y0".to_string());
                ntile.add_bel(bslots::ICAP[1], "ICAP_X0Y1".to_string());
                ntile.add_bel(bslots::PMV_CFG[0], "PMV_X0Y0".to_string());
                ntile.add_bel(bslots::PMV_CFG[1], "PMV_X0Y1".to_string());
                ntile.add_bel(bslots::STARTUP, "STARTUP_X0Y0".to_string());
                ntile.add_bel(bslots::CAPTURE, "CAPTURE_X0Y0".to_string());
                ntile.add_bel(bslots::FRAME_ECC, "FRAME_ECC".to_string());
                ntile.add_bel(bslots::EFUSE_USR, "EFUSE_USR_X0Y0".to_string());
                ntile.add_bel(bslots::USR_ACCESS, "USR_ACCESS_X0Y0".to_string());
                ntile.add_bel(bslots::DNA_PORT, "DNA_PORT_X0Y0".to_string());
                ntile.add_bel(bslots::DCIRESET, "DCIRESET_X0Y0".to_string());
                ntile.add_bel(bslots::CFG_IO_ACCESS, "CFG_IO_ACCESS_X0Y0".to_string());
                ntile.add_bel_multi(
                    bslots::SYSMON,
                    [
                        "SYSMON_X0Y0".to_string(),
                        format!("IPAD_X{ipx}Y{ipy}"),
                        format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1),
                    ],
                );
            }
            tcls::GTX => {
                let naming = if col.to_idx() == 0 { "GTX_LEFT" } else { "GTX" };
                let name_hclk = if col.to_idx() == 0 {
                    format!("HCLK_{naming}_X{x}Y{y}", y = y - 1)
                } else {
                    format!(
                        "HCLK_{naming}_X{x}Y{y}",
                        x = namer.rxlut[col] + 2,
                        y = y + y / 20
                    )
                };
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    naming,
                    [
                        name_hclk,
                        format!("{naming}_X{x}Y{y}", y = y - 20),
                        format!("{naming}_X{x}Y{y}", y = y - 10),
                        format!("{naming}_X{x}Y{y}",),
                        format!("{naming}_X{x}Y{y}", y = y + 10),
                    ],
                );
                let gx = gt_grid.xlut[col];
                let gy = gt_grid.ylut[row];
                let ipx = if col.to_idx() == 0 { 0 } else { 1 + gx };
                for i in 0..4 {
                    ntile.add_bel(
                        bslots::IPAD_RXP[i],
                        format!("IPAD_X{ipx}Y{y}", y = gy * 24 + i * 6 + 1),
                    );
                    ntile.add_bel(
                        bslots::IPAD_RXN[i],
                        format!("IPAD_X{ipx}Y{y}", y = gy * 24 + i * 6),
                    );
                    ntile.add_bel(
                        bslots::OPAD_TXP[i],
                        format!("OPAD_X{gx}Y{y}", y = gy * 8 + i * 2 + 1),
                    );
                    ntile.add_bel(
                        bslots::OPAD_TXN[i],
                        format!("OPAD_X{gx}Y{y}", y = gy * 8 + i * 2),
                    );
                    ntile.add_bel(bslots::GTX[i], format!("GTXE1_X{gx}Y{gy}", gy = gy * 4 + i));
                }
                ntile.add_bel(
                    bslots::IPAD_CLKP[0],
                    format!("IPAD_X{ipx}Y{y}", y = gy * 24 + 10),
                );
                ntile.add_bel(
                    bslots::IPAD_CLKN[0],
                    format!("IPAD_X{ipx}Y{y}", y = gy * 24 + 11),
                );
                ntile.add_bel(
                    bslots::IPAD_CLKP[1],
                    format!("IPAD_X{ipx}Y{y}", y = gy * 24 + 8),
                );
                ntile.add_bel(
                    bslots::IPAD_CLKN[1],
                    format!("IPAD_X{ipx}Y{y}", y = gy * 24 + 9),
                );
                ntile.add_bel(
                    bslots::BUFDS[0],
                    format!("IBUFDS_GTXE1_X{gx}Y{y}", y = gy * 2),
                );
                ntile.add_bel(
                    bslots::BUFDS[1],
                    format!("IBUFDS_GTXE1_X{gx}Y{y}", y = gy * 2 + 1),
                );
            }
            tcls::GTH => {
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    if col.to_idx() == 0 { "GTH_W" } else { "GTH_E" },
                    if col.to_idx() == 0 {
                        [
                            format!("GTH_L_BOT_X{x}Y{y}", y = y - 10),
                            format!("GTH_L_TOP_X{x}Y{y}", y = y + 10),
                            format!("HCLK_GTH_LEFT_X{x}Y{y}", y = y - 1),
                        ]
                    } else {
                        [
                            format!("GTH_BOT_X{x}Y{y}", y = y - 10),
                            format!("GTH_TOP_X{x}Y{y}", y = y + 10),
                            format!(
                                "HCLK_GTH_X{x}Y{y}",
                                x = namer.rxlut[col] + 2,
                                y = y + y / 20
                            ),
                        ]
                    },
                );
                let gx = gt_grid.xlut[col];
                let gy = gt_grid.ylut[row];
                let gthy = gth_grid.ylut[row];
                let gtxy = gy - gthy;
                let ipx = if col.to_idx() == 0 { 0 } else { 1 + gx };
                for i in 0..4 {
                    ntile.add_bel(
                        bslots::IPAD_RXP[i],
                        format!(
                            "IPAD_X{ipx}Y{y}",
                            y = gtxy * 24 + gthy * 12 + 6 + (7 - 2 * i)
                        ),
                    );
                    ntile.add_bel(
                        bslots::IPAD_RXN[i],
                        format!(
                            "IPAD_X{ipx}Y{y}",
                            y = gtxy * 24 + gthy * 12 + 6 + (6 - 2 * i)
                        ),
                    );
                }
                for i in 0..4 {
                    ntile.add_bel(
                        bslots::OPAD_TXP[i],
                        format!("OPAD_X{gx}Y{y}", y = (gtxy * 4 + gthy) * 8 + (7 - 2 * i)),
                    );
                    ntile.add_bel(
                        bslots::OPAD_TXN[i],
                        format!("OPAD_X{gx}Y{y}", y = (gtxy * 4 + gthy) * 8 + (6 - 2 * i)),
                    );
                }
                ntile.add_bel(
                    bslots::IPAD_CLKP[0],
                    format!("IPAD_X{ipx}Y{y}", y = gtxy * 24 - 8 + gthy * 12),
                );
                ntile.add_bel(
                    bslots::IPAD_CLKN[0],
                    format!("IPAD_X{ipx}Y{y}", y = gtxy * 24 - 9 + gthy * 12),
                );
                ntile.add_bel(bslots::GTH_QUAD, format!("GTHE1_QUAD_X{gx}Y{gthy}"));
                ntile.add_bel(
                    bslots::BUFDS[0],
                    format!("IBUFDS_GTHE1_X{gx}Y{y}", y = gthy * 2 + 1),
                );
            }
            tcls::GLOBAL => (),

            _ => unreachable!(),
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid: namer.ngrid,
        gtz: Default::default(),
    }
}
