use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{
    expanded::ExpandedDevice,
    grid::{ColumnKind, DisabledPart, Grid},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::ExpandedNamedDevice;

struct Namer<'a> {
    ngrid: ExpandedGridNaming<'a>,
    grid: &'a Grid,
    tiexlut: EntityVec<ColId, usize>,
    rxlut: EntityVec<ColId, usize>,
}

impl Namer<'_> {
    fn fill_rxlut(&mut self) {
        let mut rx = 0;
        for (col, &kind) in &self.grid.columns {
            if self.grid.cols_vbrk.contains(&col) {
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
        for &kind in self.grid.columns.values() {
            self.tiexlut.push(tie_x);
            tie_x += 1;
            if kind == ColumnKind::Dsp {
                tie_x += 1;
            }
        }
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);

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
        grid: edev.grids[DieId::from_idx(0)],
        tiexlut: EntityVec::new(),
        rxlut: EntityVec::new(),
    };

    namer.fill_tiexlut();
    namer.fill_rxlut();

    for die in egrid.dies() {
        let grid = edev.grids[die.die];
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    let x = col.to_idx();
                    let y = row.to_idx();
                    match &kind[..] {
                        "INT" => {
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, "INT", [format!("INT_X{x}Y{y}")]);
                            let tie_x = namer.tiexlut[col];
                            nnode.tie_name = Some(format!("TIEOFF_X{tie_x}Y{y}"));
                        }
                        "INTF" => {
                            if grid.columns[col] == ColumnKind::Io && col < edev.col_cfg {
                                namer.ngrid.name_node(
                                    nloc,
                                    "INTF.IOI_L",
                                    [format!("IOI_L_INT_INTERFACE_X{x}Y{y}")],
                                );
                            } else {
                                namer.ngrid.name_node(
                                    nloc,
                                    "INTF",
                                    [format!("INT_INTERFACE_X{x}Y{y}")],
                                );
                            }
                        }
                        "INTF.DELAY" => {
                            if grid.columns[col] == ColumnKind::Gt {
                                if col.to_idx() == 0 {
                                    namer.ngrid.name_node(
                                        nloc,
                                        "INTF.GT_L",
                                        [format!("GT_L_INT_INTERFACE_X{x}Y{y}")],
                                    );
                                } else {
                                    namer.ngrid.name_node(
                                        nloc,
                                        "INTF.GTX",
                                        [format!("GTX_INT_INTERFACE_X{x}Y{y}")],
                                    );
                                }
                            } else {
                                let hard = grid.col_hard.as_ref().unwrap();
                                if col == hard.col {
                                    namer.ngrid.name_node(
                                        nloc,
                                        "INTF.EMAC",
                                        [format!("EMAC_INT_INTERFACE_X{x}Y{y}")],
                                    );
                                } else if col == hard.col - 3 {
                                    namer.ngrid.name_node(
                                        nloc,
                                        "INTF.PCIE_L",
                                        [format!("PCIE_INT_INTERFACE_L_X{x}Y{y}")],
                                    );
                                } else if col == hard.col - 2 {
                                    namer.ngrid.name_node(
                                        nloc,
                                        "INTF.PCIE_R",
                                        [format!("PCIE_INT_INTERFACE_R_X{x}Y{y}")],
                                    );
                                } else {
                                    unreachable!()
                                }
                            }
                        }
                        "HCLK" => {
                            let mut naming = "HCLK";
                            let mut name = format!("HCLK_X{x}Y{y}", y = y - 1);
                            if col == grid.cols_qbuf.unwrap().0 || col == grid.cols_qbuf.unwrap().1
                            {
                                naming = "HCLK.QBUF";
                                name = format!("HCLK_QBUF_X{x}Y{y}", y = y - 1);
                            }
                            if edev.in_int_hole(die.die, col, row - 1) {
                                name = format!("HCLK_X{x}Y{y}");
                            }
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            nnode.add_bel(0, format!("GLOBALSIG_X{x}Y{y}", y = y / 40));
                        }
                        "HCLK_QBUF" => {
                            namer.ngrid.name_node(
                                nloc,
                                "HCLK_QBUF",
                                [format!("HCLK_QBUF_X{x}Y{y}", y = y - 1)],
                            );
                        }
                        "MGT_BUF" => {
                            if col < edev.col_cfg {
                                namer.ngrid.name_node(
                                    nloc,
                                    "MGT_BUF.L",
                                    [format!("HCLK_CLBLM_MGT_LEFT_X{x}Y{y}", y = y - 1)],
                                );
                            } else {
                                namer.ngrid.name_node(
                                    nloc,
                                    "MGT_BUF.R",
                                    [format!("HCLK_CLB_X{x}Y{y}", y = y - 1)],
                                );
                            }
                        }
                        "CLBLL" | "CLBLM" => {
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, kind, [format!("{kind}_X{x}Y{y}")]);
                            let sx0 = clb_grid.xlut[col] * 2;
                            let sx1 = clb_grid.xlut[col] * 2 + 1;
                            let sy = clb_grid.ylut[row];
                            nnode.add_bel(0, format!("SLICE_X{sx0}Y{sy}"));
                            nnode.add_bel(1, format!("SLICE_X{sx1}Y{sy}"));
                        }
                        "BRAM" => {
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, kind, [format!("BRAM_X{x}Y{y}")]);
                            let bx = bram_grid.xlut[col];
                            let by = bram_grid.ylut[row];
                            nnode.add_bel(0, format!("RAMB36_X{bx}Y{by}"));
                            nnode.add_bel(1, format!("RAMB18_X{bx}Y{y}", y = by * 2));
                            nnode.add_bel(2, format!("RAMB18_X{bx}Y{y}", y = by * 2 + 1));
                        }
                        "PMVBRAM" => {
                            let hy = if edev.in_int_hole(die.die, col, row - 1) {
                                y
                            } else {
                                y - 1
                            };
                            let name = format!("HCLK_BRAM_X{x}Y{hy}");
                            let name_bram0 = format!("BRAM_X{x}Y{y}");
                            let name_bram1 = format!("BRAM_X{x}Y{y}", y = y + 5);
                            let name_bram2 = format!("BRAM_X{x}Y{y}", y = y + 10);
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "PMVBRAM",
                                [name, name_bram0, name_bram1, name_bram2],
                            );
                            let px = pmvbram_grid.xlut[col];
                            let py = pmvbram_grid.ylut[row];
                            nnode.add_bel(0, format!("PMVBRAM_X{px}Y{py}"));
                        }
                        "DSP" => {
                            let nnode =
                                namer.ngrid.name_node(nloc, kind, [format!("DSP_X{x}Y{y}")]);
                            let dx = dsp_grid.xlut[col];
                            let dy0 = dsp_grid.ylut[row] * 2;
                            let dy1 = dsp_grid.ylut[row] * 2 + 1;
                            nnode.add_bel(0, format!("DSP48_X{dx}Y{dy0}"));
                            nnode.add_bel(1, format!("DSP48_X{dx}Y{dy1}"));
                            let tx = namer.tiexlut[col] + 1;
                            nnode.add_bel(2, format!("TIEOFF_X{tx}Y{y}"));
                        }
                        "IO" => {
                            let naming = if col < edev.col_cfg { "LIOI" } else { "RIOI" };
                            let iob_tk = if col < edev.col_cfg {
                                if Some(col) == edev.col_lio || edev.col_lio.is_none() {
                                    "LIOB"
                                } else {
                                    "LIOB_FT"
                                }
                            } else {
                                "RIOB"
                            };
                            let name_ioi = format!("{naming}_X{x}Y{y}");
                            let name_iob = format!("{iob_tk}_X{x}Y{y}");
                            let nnode = namer.ngrid.name_node(nloc, naming, [name_ioi, name_iob]);
                            let iox = io_grid.xlut[col];
                            let ioy0 = io_grid.ylut[row] * 2;
                            let ioy1 = io_grid.ylut[row] * 2 + 1;
                            nnode.add_bel(0, format!("ILOGIC_X{iox}Y{ioy0}"));
                            nnode.add_bel(1, format!("ILOGIC_X{iox}Y{ioy1}"));
                            nnode.add_bel(2, format!("OLOGIC_X{iox}Y{ioy0}"));
                            nnode.add_bel(3, format!("OLOGIC_X{iox}Y{ioy1}"));
                            nnode.add_bel(4, format!("IODELAY_X{iox}Y{ioy0}"));
                            nnode.add_bel(5, format!("IODELAY_X{iox}Y{ioy1}"));
                            nnode.add_bel(6, format!("IOB_X{iox}Y{ioy0}"));
                            nnode.add_bel(7, format!("IOB_X{iox}Y{ioy1}"));
                        }
                        "HCLK_IOI" => {
                            let (naming, hclk_tk, ioi_tk) = if Some(col) == edev.col_lio {
                                ("HCLK_IOI.OL", "HCLK_OUTER_IOI", "LIOI")
                            } else if Some(col) == edev.col_lcio {
                                ("HCLK_IOI.IL", "HCLK_INNER_IOI", "LIOI")
                            } else if Some(col) == edev.col_rcio {
                                ("HCLK_IOI.IR", "HCLK_INNER_IOI", "RIOI")
                            } else if Some(col) == edev.col_rio {
                                ("HCLK_IOI.OR", "HCLK_OUTER_IOI", "RIOI")
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
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, naming, [name, name_ioi_s, name_ioi_n]);
                            let iox = io_grid.xlut[col];
                            let hy = row.to_idx() / 40;
                            nnode.add_bel(0, format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 2));
                            nnode.add_bel(1, format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 3));
                            nnode.add_bel(2, format!("BUFIODQS_X{iox}Y{y}", y = hy * 4));
                            nnode.add_bel(3, format!("BUFIODQS_X{iox}Y{y}", y = hy * 4 + 1));
                            nnode.add_bel(4, format!("BUFR_X{iox}Y{y}", y = hy * 2 + 1));
                            nnode.add_bel(5, format!("BUFR_X{iox}Y{y}", y = hy * 2));
                            nnode.add_bel(6, format!("BUFO_X{iox}Y{y}", y = hy * 2 + 1));
                            nnode.add_bel(7, format!("BUFO_X{iox}Y{y}", y = hy * 2));
                            nnode.add_bel(8, format!("IDELAYCTRL_X{iox}Y{hy}"));
                            nnode.add_bel(9, format!("DCI_X{iox}Y{hy}"));
                        }
                        "CMT" => {
                            let naming = if row < grid.row_bufg() {
                                "CMT.BOT"
                            } else {
                                "CMT.TOP"
                            };
                            let bt = if row < grid.row_bufg() { "BOT" } else { "TOP" };
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                naming,
                                [
                                    format!("CMT_X{x}Y{y}", y = y - 9),
                                    format!("CMT_X{x}Y{y}", y = y + 9),
                                    format!("HCLK_CMT_{bt}_X{x}Y{y}", y = y - 1),
                                ],
                            );
                            let bx = cmt_grid.xlut[col];
                            let by = cmt_grid.ylut[row];
                            for i in 0..2 {
                                for j in 0..12 {
                                    nnode.add_bel(
                                        i * 12 + j,
                                        format!("BUFHCE_X{i}Y{y}", y = by * 12 + j),
                                    );
                                }
                            }
                            nnode.add_bel(24, format!("MMCM_ADV_X{bx}Y{y}", y = by * 2));
                            nnode.add_bel(25, format!("MMCM_ADV_X{bx}Y{y}", y = by * 2 + 1));
                            nnode.add_bel(26, format!("PPR_FRAME_X{bx}Y{by}"));
                        }
                        "PMVIOB" => {
                            let naming = if row < grid.row_bufg() {
                                "CMT_PMVA_BELOW"
                            } else {
                                "CMT_PMVA"
                            };
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, naming, [format!("{naming}_X{x}Y{y}")]);
                            let bx = pmviob_grid.xlut[col];
                            let by = pmviob_grid.ylut[row];
                            nnode.add_bel(0, format!("PMVIOB_X{bx}Y{by}"));
                        }
                        "CMT_BUFG_BOT" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    format!("{kind}_X{x}Y{y}", y = y - 2),
                                    format!("CMT_X{x}Y{y}", y = y - 11),
                                ],
                            );
                            for i in 0..16 {
                                nnode.add_bel(i, format!("BUFGCTRL_X0Y{i}"));
                            }
                        }
                        "CMT_BUFG_TOP" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    format!("{kind}_X{x}Y{y}"),
                                    format!("CMT_X{x}Y{y}", y = y + 11),
                                ],
                            );
                            for i in 0..16 {
                                nnode.add_bel(i, format!("BUFGCTRL_X0Y{y}", y = i + 16));
                            }
                        }
                        "GCLK_BUF" => {
                            let name = if row < grid.row_bufg() {
                                format!("CMT_PMVB_BUF_BELOW_X{x}Y{y}", y = y - 2)
                            } else {
                                format!("CMT_PMVB_BUF_ABOVE_X{x}Y{y}")
                            };
                            namer.ngrid.name_node(nloc, "GCLK_BUF", [name]);
                        }
                        "EMAC" => {
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, kind, [format!("EMAC_X{x}Y{y}")]);
                            let bx = emac_grid.xlut[col];
                            let by = emac_grid.ylut[row];
                            nnode.add_bel(0, format!("TEMAC_X{bx}Y{by}"));
                        }
                        "PCIE" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [format!("PCIE_X{x}Y{y}", x = x + 1, y = y + 10)],
                            );
                            let bx = pcie_grid.xlut[col];
                            let by = pcie_grid.ylut[row];
                            nnode.add_bel(0, format!("PCIE_X{bx}Y{by}"));
                        }
                        "CFG" => {
                            let row_b: RowId = row - 40;
                            let ry = row_b.to_idx() + 11 + row_b.to_idx() / 20;
                            let rx = namer.rxlut[edev.col_cfg] - 2;
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "CFG",
                                [
                                    format!("CFG_CENTER_0_X{rx}Y{ry}"),
                                    format!("CFG_CENTER_1_X{rx}Y{ry}", ry = ry + 21),
                                    format!("CFG_CENTER_2_X{rx}Y{ry}", ry = ry + 42),
                                    format!("CFG_CENTER_3_X{rx}Y{ry}", ry = ry + 63),
                                ],
                            );
                            let ipx = if edev.col_lgt.is_some() { 1 } else { 0 };
                            let mut ipy = 0;
                            if !grid.cols_gt.is_empty() {
                                ipy += 6;
                                for reg in grid.regs() {
                                    if reg < grid.reg_cfg
                                        && !edev.disabled.contains(&DisabledPart::GtxRow(reg))
                                    {
                                        ipy += 24;
                                    }
                                }
                            };
                            nnode.add_bel(0, "BSCAN_X0Y0".to_string());
                            nnode.add_bel(1, "BSCAN_X0Y1".to_string());
                            nnode.add_bel(2, "BSCAN_X0Y2".to_string());
                            nnode.add_bel(3, "BSCAN_X0Y3".to_string());
                            nnode.add_bel(4, "ICAP_X0Y0".to_string());
                            nnode.add_bel(5, "ICAP_X0Y1".to_string());
                            nnode.add_bel(6, "PMV_X0Y0".to_string());
                            nnode.add_bel(7, "PMV_X0Y1".to_string());
                            nnode.add_bel(8, "STARTUP_X0Y0".to_string());
                            nnode.add_bel(9, "CAPTURE_X0Y0".to_string());
                            nnode.add_bel(10, "FRAME_ECC".to_string());
                            nnode.add_bel(11, "EFUSE_USR_X0Y0".to_string());
                            nnode.add_bel(12, "USR_ACCESS_X0Y0".to_string());
                            nnode.add_bel(13, "DNA_PORT_X0Y0".to_string());
                            nnode.add_bel(14, "DCIRESET_X0Y0".to_string());
                            nnode.add_bel(15, "CFG_IO_ACCESS_X0Y0".to_string());
                            nnode.add_bel(16, "SYSMON_X0Y0".to_string());
                            nnode.add_bel(17, format!("IPAD_X{ipx}Y{ipy}"));
                            nnode.add_bel(18, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1));
                        }
                        "GTX" => {
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
                            let nnode = namer.ngrid.name_node(
                                nloc,
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
                                nnode.add_bel(
                                    i * 4,
                                    format!("IPAD_X{ipx}Y{y}", y = gy * 24 + i * 6 + 1),
                                );
                                nnode.add_bel(
                                    i * 4 + 1,
                                    format!("IPAD_X{ipx}Y{y}", y = gy * 24 + i * 6),
                                );
                                nnode.add_bel(
                                    i * 4 + 2,
                                    format!("OPAD_X{gx}Y{y}", y = gy * 8 + i * 2 + 1),
                                );
                                nnode.add_bel(
                                    i * 4 + 3,
                                    format!("OPAD_X{gx}Y{y}", y = gy * 8 + i * 2),
                                );
                                nnode.add_bel(20 + i, format!("GTXE1_X{gx}Y{gy}", gy = gy * 4 + i));
                            }
                            nnode.add_bel(16, format!("IPAD_X{ipx}Y{y}", y = gy * 24 + 10));
                            nnode.add_bel(17, format!("IPAD_X{ipx}Y{y}", y = gy * 24 + 11));
                            nnode.add_bel(18, format!("IPAD_X{ipx}Y{y}", y = gy * 24 + 8));
                            nnode.add_bel(19, format!("IPAD_X{ipx}Y{y}", y = gy * 24 + 9));
                            nnode.add_bel(24, format!("IBUFDS_GTXE1_X{gx}Y{y}", y = gy * 2));
                            nnode.add_bel(25, format!("IBUFDS_GTXE1_X{gx}Y{y}", y = gy * 2 + 1));
                        }
                        "GTH" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                if col.to_idx() == 0 { "GTH.L" } else { "GTH.R" },
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
                                nnode.add_bel(
                                    2 * i,
                                    format!(
                                        "IPAD_X{ipx}Y{y}",
                                        y = gtxy * 24 + gthy * 12 + 6 + (7 - 2 * i)
                                    ),
                                );
                                nnode.add_bel(
                                    2 * i + 1,
                                    format!(
                                        "IPAD_X{ipx}Y{y}",
                                        y = gtxy * 24 + gthy * 12 + 6 + (6 - 2 * i)
                                    ),
                                );
                            }
                            for i in 0..4 {
                                nnode.add_bel(
                                    8 + 2 * i,
                                    format!(
                                        "OPAD_X{gx}Y{y}",
                                        y = (gtxy * 4 + gthy) * 8 + (7 - 2 * i)
                                    ),
                                );
                                nnode.add_bel(
                                    8 + 2 * i + 1,
                                    format!(
                                        "OPAD_X{gx}Y{y}",
                                        y = (gtxy * 4 + gthy) * 8 + (6 - 2 * i)
                                    ),
                                );
                            }
                            nnode.add_bel(
                                16,
                                format!("IPAD_X{ipx}Y{y}", y = gtxy * 24 - 8 + gthy * 12),
                            );
                            nnode.add_bel(
                                17,
                                format!("IPAD_X{ipx}Y{y}", y = gtxy * 24 - 9 + gthy * 12),
                            );
                            nnode.add_bel(18, format!("GTHE1_QUAD_X{gx}Y{gthy}"));
                            nnode.add_bel(19, format!("IBUFDS_GTHE1_X{gx}Y{y}", y = gthy * 2 + 1));
                        }

                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid: namer.ngrid,
        gtz: Default::default(),
    }
}
