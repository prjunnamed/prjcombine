use prjcombine_interconnect::grid::{ColId, DieId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{
    chip::{ColumnKind, GtKind},
    expanded::ExpandedDevice,
};
use unnamed_entity::{EntityId, EntityVec};

use crate::ExpandedNamedDevice;

struct Namer<'a> {
    edev: &'a ExpandedDevice<'a>,
    ngrid: ExpandedGridNaming<'a>,
    rxlut: EntityVec<ColId, usize>,
}

impl Namer<'_> {
    fn fill_rxlut(&mut self) {
        let grid = &self.edev.chips[DieId::from_idx(0)];
        let mut rx = 0;
        for (col, &kind) in &grid.columns {
            if grid.cols_vbrk.contains(&col) {
                rx += 1;
            }
            self.rxlut.push(rx);
            rx += match kind {
                ColumnKind::ClbLL | ColumnKind::ClbLM => 2,
                ColumnKind::Bram | ColumnKind::Dsp => 3,
                ColumnKind::Io => {
                    if col.to_idx() == 0 {
                        5
                    } else {
                        6
                    }
                }
                ColumnKind::Cfg => 7,
                ColumnKind::Gt => 4,
                _ => unreachable!(),
            };
        }
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);

    ngrid.tie_kind = Some("TIEOFF".to_string());
    ngrid.tie_pin_pullup = Some("KEEP1".to_string());
    ngrid.tie_pin_gnd = Some("HARD0".to_string());
    ngrid.tie_pin_vcc = Some("HARD1".to_string());

    let clb_grid = ngrid.bel_grid(|_, name, _| name == "CLBLL" || name == "CLBLM");
    let bram_grid = ngrid.bel_grid(|_, name, _| name == "BRAM");
    let dsp_grid = ngrid.bel_grid(|_, name, _| name == "DSP");
    let io_grid = ngrid.bel_grid(|_, name, _| name == "IO");
    let cmt_grid = ngrid.bel_grid(|_, name, _| name == "CMT");
    let emac_grid = ngrid.bel_grid(|_, name, _| name == "EMAC");
    let pcie_grid = ngrid.bel_grid(|_, name, _| name == "PCIE");
    let ppc_grid = ngrid.bel_grid(|_, name, _| name == "PPC");
    let gt_grid = ngrid.bel_grid(|_, name, _| name == "GTP" || name == "GTX");
    let pmvbram_grid = ngrid.bel_grid(|_, name, _| name == "PMVBRAM");

    let mut namer = Namer {
        edev,
        ngrid,
        rxlut: EntityVec::new(),
    };

    namer.fill_rxlut();

    let mgt = if edev.col_lgt.is_some() { "_MGT" } else { "" };
    for die in egrid.dies() {
        let chip = edev.chips[die.die];
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
                            nnode.tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
                        }
                        "INTF" => {
                            namer.ngrid.name_node(
                                nloc,
                                "INTF",
                                [format!("INT_INTERFACE_X{x}Y{y}")],
                            );
                        }
                        "INTF.DELAY" => {
                            if chip.columns[col] == ColumnKind::Gt {
                                if col.to_idx() == 0 {
                                    namer.ngrid.name_node(
                                        nloc,
                                        "INTF.GTX_LEFT",
                                        [format!("GTX_LEFT_INT_INTERFACE_X{x}Y{y}")],
                                    );
                                } else {
                                    namer.ngrid.name_node(
                                        nloc,
                                        "INTF.GTP",
                                        [format!("GTP_INT_INTERFACE_X{x}Y{y}")],
                                    );
                                }
                            } else {
                                'intf: {
                                    if let Some(ref hard) = chip.col_hard {
                                        if hard.col == col {
                                            for &hrow in &hard.rows_emac {
                                                if row >= hrow && row < hrow + 10 {
                                                    namer.ngrid.name_node(
                                                        nloc,
                                                        "INTF.EMAC",
                                                        [format!("EMAC_INT_INTERFACE_X{x}Y{y}")],
                                                    );
                                                    break 'intf;
                                                }
                                            }
                                            for &hrow in &hard.rows_pcie {
                                                if row >= hrow && row < hrow + 40 {
                                                    namer.ngrid.name_node(
                                                        nloc,
                                                        "INTF.PCIE",
                                                        [format!("PCIE_INT_INTERFACE_X{x}Y{y}")],
                                                    );
                                                    break 'intf;
                                                }
                                            }
                                        }
                                    }
                                    for &(pcol, prow) in &chip.holes_ppc {
                                        if row >= prow && row < prow + 40 {
                                            if col == pcol {
                                                namer.ngrid.name_node(
                                                    nloc,
                                                    "INTF.PPC_L",
                                                    [format!("PPC_L_INT_INTERFACE_X{x}Y{y}")],
                                                );
                                                break 'intf;
                                            } else if col == pcol + 13 {
                                                namer.ngrid.name_node(
                                                    nloc,
                                                    "INTF.PPC_R",
                                                    [format!("PPC_R_INT_INTERFACE_X{x}Y{y}")],
                                                );
                                                break 'intf;
                                            }
                                        }
                                    }
                                    panic!("umm wtf is this interface");
                                }
                            }
                        }
                        "HCLK" => {
                            let reg = chip.row_to_reg(row);
                            let kind = match chip.columns[col] {
                                ColumnKind::Gt => {
                                    let gtc =
                                        chip.cols_gt.iter().find(|gtc| gtc.col == col).unwrap();
                                    match gtc.regs[reg].unwrap() {
                                        GtKind::Gtp => "HCLK_GT3",
                                        GtKind::Gtx => {
                                            if x == 0 {
                                                "HCLK_GTX_LEFT"
                                            } else {
                                                "HCLK_GTX"
                                            }
                                        }
                                        _ => unreachable!(),
                                    }
                                }
                                _ => "HCLK",
                            };
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "HCLK",
                                [format!("{kind}_X{x}Y{y}", y = y - 1)],
                            );
                            nnode.add_bel(0, format!("GLOBALSIG_X{x}Y{y}", y = y / 20));
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
                            let mut tk = "BRAM";
                            if let Some(ref hard) = chip.col_hard {
                                if hard.col == col {
                                    tk = "PCIE_BRAM";
                                }
                            }
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, kind, [format!("{tk}_X{x}Y{y}")]);
                            let bx = bram_grid.xlut[col];
                            let by = bram_grid.ylut[row];
                            nnode.add_bel(0, format!("RAMB36_X{bx}Y{by}"));
                        }
                        "DSP" => {
                            let nnode =
                                namer.ngrid.name_node(nloc, kind, [format!("DSP_X{x}Y{y}")]);
                            let dx = dsp_grid.xlut[col];
                            let dy0 = dsp_grid.ylut[row] * 2;
                            let dy1 = dsp_grid.ylut[row] * 2 + 1;
                            nnode.add_bel(0, format!("DSP48_X{dx}Y{dy0}"));
                            nnode.add_bel(1, format!("DSP48_X{dx}Y{dy1}"));
                        }
                        "IO" => {
                            let iox = io_grid.xlut[col];
                            let ioy0 = io_grid.ylut[row] * 2;
                            let ioy1 = io_grid.ylut[row] * 2 + 1;
                            let naming = match iox {
                                0 => {
                                    if col.to_idx() == 0 {
                                        "LIOB"
                                    } else if row >= chip.row_bufg() + 10
                                        && row < chip.row_bufg() + 20
                                    {
                                        "RIOB"
                                    } else {
                                        "LIOB_MON"
                                    }
                                }
                                1 => "CIOB",
                                2 => "RIOB",
                                _ => unreachable!(),
                            };
                            let name_ioi = format!("IOI_X{x}Y{y}");
                            let name_iob = format!("{naming}_X{x}Y{y}");
                            let nnode = namer.ngrid.name_node(nloc, naming, [name_ioi, name_iob]);
                            nnode.add_bel(0, format!("ILOGIC_X{iox}Y{ioy0}"));
                            nnode.add_bel(1, format!("ILOGIC_X{iox}Y{ioy1}"));
                            nnode.add_bel(2, format!("OLOGIC_X{iox}Y{ioy0}"));
                            nnode.add_bel(3, format!("OLOGIC_X{iox}Y{ioy1}"));
                            nnode.add_bel(4, format!("IODELAY_X{iox}Y{ioy0}"));
                            nnode.add_bel(5, format!("IODELAY_X{iox}Y{ioy1}"));
                            nnode.add_bel(6, format!("IOB_X{iox}Y{ioy0}"));
                            nnode.add_bel(7, format!("IOB_X{iox}Y{ioy1}"));
                        }
                        "CMT" => {
                            let naming = if row.to_idx() % 20 == 0 {
                                "CMT_BOT"
                            } else {
                                "CMT_TOP"
                            };
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, naming, [format!("CMT_X{x}Y{y}")]);
                            let bx = cmt_grid.xlut[col];
                            let by = cmt_grid.ylut[row];
                            nnode.add_bel(0, format!("DCM_ADV_X{bx}Y{y}", y = by * 2));
                            nnode.add_bel(1, format!("DCM_ADV_X{bx}Y{y}", y = by * 2 + 1));
                            nnode.add_bel(2, format!("PLL_ADV_X{bx}Y{by}"));
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
                                [
                                    format!("PCIE_B_X{x}Y{y}", y = y + 10),
                                    format!("PCIE_T_X{x}Y{y}", y = y + 30),
                                ],
                            );
                            let bx = pcie_grid.xlut[col];
                            let by = pcie_grid.ylut[row];
                            nnode.add_bel(0, format!("PCIE_X{bx}Y{by}"));
                        }
                        "PPC" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    format!("PPC_B_X36Y{y}", y = row.to_idx() / 10 * 11 + 11),
                                    format!("PPC_T_X36Y{y}", y = row.to_idx() / 10 * 11 + 33),
                                ],
                            );
                            let bx = ppc_grid.xlut[col];
                            let by = ppc_grid.ylut[row];
                            nnode.add_bel(0, format!("PPC440_X{bx}Y{by}"));
                        }
                        "GTP" | "GTX" => {
                            let naming = if kind == "GTP" {
                                "GT3"
                            } else if col.to_idx() == 0 {
                                "GTX_LEFT"
                            } else {
                                "GTX"
                            };
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                naming,
                                [format!("{naming}_X{x}Y{y}", y = y + 9)],
                            );
                            let gtx = gt_grid.xlut[col];
                            let gty = gt_grid.ylut[row];
                            let ipx = if col.to_idx() == 0 { 0 } else { gtx + 1 };
                            let ipy = if gty < chip.reg_cfg.to_idx() {
                                gty * 6
                            } else {
                                gty * 6 + 6
                            };
                            nnode.add_bel(0, format!("{kind}_DUAL_X{gtx}Y{gty}"));
                            nnode.add_bel(1, format!("BUFDS_X{gtx}Y{gty}"));
                            nnode.add_bel(2, format!("CRC64_X{gtx}Y{y}", y = gty * 2));
                            nnode.add_bel(3, format!("CRC64_X{gtx}Y{y}", y = gty * 2 + 1));
                            nnode.add_bel(4, format!("CRC32_X{gtx}Y{y}", y = gty * 4));
                            nnode.add_bel(5, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 1));
                            nnode.add_bel(6, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 2));
                            nnode.add_bel(7, format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 3));
                            nnode.add_bel(8, format!("IPAD_X{ipx}Y{y}", y = ipy + 1));
                            nnode.add_bel(9, format!("IPAD_X{ipx}Y{ipy}"));
                            nnode.add_bel(10, format!("IPAD_X{ipx}Y{y}", y = ipy + 3));
                            nnode.add_bel(11, format!("IPAD_X{ipx}Y{y}", y = ipy + 2));
                            nnode.add_bel(12, format!("IPAD_X{ipx}Y{y}", y = ipy + 5));
                            nnode.add_bel(13, format!("IPAD_X{ipx}Y{y}", y = ipy + 4));
                            nnode.add_bel(14, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 1));
                            nnode.add_bel(15, format!("OPAD_X{gtx}Y{y}", y = gty * 4));
                            nnode.add_bel(16, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 3));
                            nnode.add_bel(17, format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 2));
                        }
                        "CFG" => {
                            let rx = namer.rxlut[col] + 3;
                            let ry = chip.reg_cfg.to_idx() * 22;
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    format!("CFG_CENTER_X{rx}Y{ry}"),
                                    format!("CLK_BUFGMUX_X{rx}Y{ry}"),
                                ],
                            );
                            let ipx = if edev.col_lgt.is_some() { 1 } else { 0 };
                            let ipy = if !chip.cols_gt.is_empty() {
                                chip.reg_cfg.to_idx() * 6
                            } else {
                                0
                            };
                            for i in 0..32 {
                                nnode.add_bel(i, format!("BUFGCTRL_X0Y{i}"));
                            }
                            for i in 0..4 {
                                nnode.add_bel(32 + i, format!("BSCAN_X0Y{i}"));
                            }
                            for i in 0..2 {
                                nnode.add_bel(36 + i, format!("ICAP_X0Y{i}"));
                            }
                            nnode.add_bel(38, "PMV".to_string());
                            nnode.add_bel(39, "STARTUP".to_string());
                            nnode.add_bel(40, "JTAGPPC".to_string());
                            nnode.add_bel(41, "FRAME_ECC".to_string());
                            nnode.add_bel(42, "DCIRESET".to_string());
                            nnode.add_bel(43, "CAPTURE".to_string());
                            nnode.add_bel(44, "USR_ACCESS_SITE".to_string());
                            nnode.add_bel(45, "KEY_CLEAR".to_string());
                            nnode.add_bel(46, "EFUSE_USR".to_string());
                            nnode.add_bel(47, "SYSMON_X0Y0".to_string());
                            nnode.add_bel(48, format!("IPAD_X{ipx}Y{ipy}"));
                            nnode.add_bel(49, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1));
                        }

                        "CLK_HROW" => {
                            let name_hrow = format!("CLK_HROW{mgt}_X{x}Y{y}", y = y - 1);
                            namer.ngrid.name_node(nloc, kind, [name_hrow]);
                        }
                        "CLK_CMT_B" | "CLK_CMT_T" => {
                            let naming = if row < chip.row_bufg() {
                                "CLK_CMT_BOT"
                            } else {
                                "CLK_CMT_TOP"
                            };
                            let rx = namer.rxlut[col] + 4;
                            let ry = y / 10 * 11 + 1;
                            let name = format!("{naming}{mgt}_X{rx}Y{ry}");
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        "CLK_IOB_B" | "CLK_IOB_T" => {
                            let name = format!("{kind}_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        "CLK_MGT_B" | "CLK_MGT_T" => {
                            let naming = if row < chip.row_bufg() {
                                "CLK_MGT_BOT"
                            } else {
                                "CLK_MGT_TOP"
                            };
                            let name = format!("{naming}{mgt}_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        "HCLK_IOI_BOTCEN" | "HCLK_CMT_IOI" => {
                            let name = if kind == "HCLK_CMT_IOI" {
                                format!("{kind}_X{x}Y{y}", y = y - 1)
                            } else {
                                format!("{kind}{mgt}_X{x}Y{y}", y = y - 1)
                            };
                            let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                            let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name, name_i0, name_i1]);
                            let iox = io_grid.xlut[col];
                            let ioy = io_grid.ylut[row];
                            let banky = ioy / 20;
                            nnode.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 1));
                            nnode.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = banky * 4));
                            nnode.add_bel(2, format!("IDELAYCTRL_X{iox}Y{banky}"));
                            nnode.add_bel(3, format!("DCI_X{iox}Y{banky}"));
                        }
                        "HCLK_IOI_TOPCEN" | "HCLK_IOI_CMT" => {
                            let name = format!("{kind}{mgt}_X{x}Y{y}", y = y - 1);
                            let name_i2 = format!("IOI_X{x}Y{y}");
                            let name_i3 = format!("IOI_X{x}Y{y}", y = y + 1);
                            let nnode = namer.ngrid.name_node(nloc, kind, [name, name_i2, name_i3]);
                            let iox = io_grid.xlut[col];
                            let ioy = io_grid.ylut[row];
                            let banky = ioy / 20;
                            nnode.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 2));
                            nnode.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 3));
                            nnode.add_bel(2, format!("IDELAYCTRL_X{iox}Y{banky}"));
                            nnode.add_bel(3, format!("DCI_X{iox}Y{banky}"));
                        }
                        "HCLK_IOI_CENTER" => {
                            let name = format!("HCLK_IOI_CENTER_X{x}Y{y}", y = y - 1);
                            let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                            let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                            let name_i2 = format!("IOI_X{x}Y{y}");
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [name, name_i0, name_i1, name_i2],
                            );
                            let iox = io_grid.xlut[col];
                            let ioy = io_grid.ylut[row];
                            let banky = ioy / 20;
                            nnode.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 2));
                            nnode.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 3));
                            nnode.add_bel(2, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 1));
                            nnode.add_bel(3, format!("BUFIO_X{iox}Y{y}", y = banky * 4));
                            nnode.add_bel(4, format!("IDELAYCTRL_X{iox}Y{banky}"));
                            nnode.add_bel(5, format!("DCI_X{iox}Y{banky}"));
                        }
                        "HCLK_IOI" => {
                            let name = format!("HCLK_IOI_X{x}Y{y}", y = y - 1);
                            let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                            let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                            let name_i2 = format!("IOI_X{x}Y{y}");
                            let name_i3 = format!("IOI_X{x}Y{y}", y = y + 1);
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [name, name_i0, name_i1, name_i2, name_i3],
                            );
                            let iox = io_grid.xlut[col];
                            let ioy = io_grid.ylut[row];
                            let banky = ioy / 20;
                            nnode.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 2));
                            nnode.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 3));
                            nnode.add_bel(2, format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 1));
                            nnode.add_bel(3, format!("BUFIO_X{iox}Y{y}", y = banky * 4));
                            nnode.add_bel(4, format!("BUFR_X{x}Y{y}", x = iox / 2, y = banky * 2));
                            nnode.add_bel(
                                5,
                                format!("BUFR_X{x}Y{y}", x = iox / 2, y = banky * 2 + 1),
                            );
                            nnode.add_bel(6, format!("IDELAYCTRL_X{iox}Y{banky}"));
                            nnode.add_bel(7, format!("DCI_X{iox}Y{banky}"));
                        }
                        "HCLK_CMT" => {
                            let bmt = if row + 30 == chip.row_bufg() {
                                "BOT"
                            } else if row == chip.row_bufg() + 30 {
                                "TOP"
                            } else {
                                "MID"
                            };
                            let name = format!("HCLK_IOB_CMT_{bmt}{mgt}_X{x}Y{y}", y = y - 1);
                            let name_hrow = format!("CLK_HROW{mgt}_X{x}Y{y}", y = y - 1);
                            namer.ngrid.name_node(nloc, kind, [name, name_hrow]);
                        }
                        "PMVBRAM" => {
                            let mut kind = "BRAM";
                            if let Some(ref hard) = chip.col_hard {
                                if hard.col == col {
                                    kind = "PCIE_BRAM";
                                }
                            }
                            let name = format!("HCLK_{kind}_X{x}Y{y}", y = y - 1);
                            let name_bram = format!("{kind}_X{x}Y{y}");
                            let nnode = namer.ngrid.name_node(nloc, "PMVBRAM", [name, name_bram]);
                            let px = pmvbram_grid.xlut[col];
                            let py = pmvbram_grid.ylut[row];
                            nnode.add_bel(0, format!("PMVBRAM_X{px}Y{py}"));
                        }
                        "HCLK_BRAM_MGT" => {
                            let l = if col < edev.col_cfg { "_LEFT" } else { "" };
                            let name = format!("HCLK_BRAM_MGT{l}_X{x}Y{y}", y = y - 1);
                            namer.ngrid.name_node(nloc, "HCLK_BRAM_MGT", [name]);
                        }

                        _ => unreachable!(),
                    }
                }
                for (slot, term) in &die[(col, row)].terms {
                    let tloc = (die.die, col, row, slot);
                    let kind = egrid.db.terms.key(term.kind);
                    let x = col.to_idx();
                    let y = row.to_idx();

                    match &kind[..] {
                        "TERM.W" => {
                            let name = if edev.col_lgt.is_some() {
                                format!("GTX_L_TERM_INT_X{x}Y{y}")
                            } else {
                                format!("L_TERM_INT_X{x}Y{y}")
                            };
                            namer.ngrid.name_term_tile(tloc, "TERM.W", name);
                        }
                        "TERM.E" => {
                            let name = format!("R_TERM_INT_X{x}Y{y}");
                            namer.ngrid.name_term_tile(tloc, "TERM.E", name);
                        }
                        "TERM.S.PPC" => {
                            let name = format!("PPC_T_TERM_X{x}Y{y}");
                            namer.ngrid.name_term_tile(tloc, "TERM.S.PPC", name);
                        }
                        "TERM.N.PPC" => {
                            let name = format!("PPC_B_TERM_X{x}Y{y}");
                            namer.ngrid.name_term_tile(tloc, "TERM.N.PPC", name);
                        }
                        "PPC.W" => {
                            // sigh.
                            let ry = y / 10 * 11 + y % 10 + 1;
                            let name_l = format!("L_TERM_PPC_X{x}Y{y}", x = x - 13);
                            let name_r = format!("R_TERM_PPC_X53Y{ry}");
                            namer.ngrid.name_term_pair(tloc, "PPC.W", name_r, name_l);
                        }
                        "PPC.E" => {
                            let ry = y / 10 * 11 + y % 10 + 1;
                            let name_l = format!("L_TERM_PPC_X{x}Y{y}");
                            let name_r = format!("R_TERM_PPC_X53Y{ry}");
                            namer.ngrid.name_term_pair(tloc, "PPC.E", name_l, name_r);
                        }
                        "INT_BUFS.W" => {
                            let mon = if edev.col_lgt.is_some() { "_MON" } else { "" };
                            let name_l = format!("INT_BUFS_L_X{x}Y{y}", x = x - 1);
                            let name_r = format!("INT_BUFS_R{mon}_X{x}Y{y}");
                            namer
                                .ngrid
                                .name_term_pair(tloc, "INT_BUFS.W", name_r, name_l);
                        }
                        "INT_BUFS.E" => {
                            let mon = if edev.col_lgt.is_some() { "_MON" } else { "" };
                            let name_l = format!("INT_BUFS_L_X{x}Y{y}");
                            let name_r = format!("INT_BUFS_R{mon}_X{x}Y{y}", x = x + 1);
                            namer
                                .ngrid
                                .name_term_pair(tloc, "INT_BUFS.E", name_l, name_r);
                        }

                        _ => (),
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
