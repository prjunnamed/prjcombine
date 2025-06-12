use prjcombine_interconnect::grid::{CellCoord, RowId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{bels, chip::CfgRowKind, expanded::ExpandedDevice, tslots};
use unnamed_entity::EntityId;

use crate::ExpandedNamedDevice;

struct Namer<'a> {
    ngrid: ExpandedGridNaming<'a>,
}

impl Namer<'_> {
    fn name_intf(&mut self, cell: CellCoord, naming: &str, name: &str) {
        self.ngrid
            .name_tile(cell.tile(tslots::INTF), naming, [name.into()]);
    }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);

    ngrid.tie_kind = Some("TIEOFF".to_string());
    ngrid.tie_pin_pullup = Some("KEEP1".to_string());
    ngrid.tie_pin_gnd = Some("HARD0".to_string());
    ngrid.tie_pin_vcc = Some("HARD1".to_string());

    let clb_grid = ngrid.bel_grid(|_, name, _| name == "CLB");
    let bram_grid = ngrid.bel_grid(|_, name, _| name == "BRAM");
    let dsp_grid = ngrid.bel_grid(|_, name, _| name == "DSP");
    let io_grid = ngrid.bel_grid(|_, name, _| name == "IO");
    let dcm_grid = ngrid.bel_grid(|_, name, _| name == "DCM");
    let ccm_grid = ngrid.bel_grid(|_, name, _| name == "CCM");
    let sysmon_grid = ngrid.bel_grid(|_, name, _| name == "SYSMON");
    let ppc_grid = ngrid.bel_grid(|_, name, _| name == "PPC");
    let mgt_grid = ngrid.bel_grid(|_, name, _| name == "MGT");
    let dci_grid = ngrid.bel_grid(|_, name, _| {
        matches!(
            name,
            "HCLK_IOIS_DCI"
                | "HCLK_DCMIOB"
                | "HCLK_IOBDCM"
                | "HCLK_CENTER"
                | "HCLK_CENTER_ABOVE_CFG"
        )
    });

    let mut namer = Namer { ngrid };

    let term_slot_w = egrid.db.get_conn_slot("W");
    let term_slot_e = egrid.db.get_conn_slot("E");
    let term_slot_s = egrid.db.get_conn_slot("S");
    let term_slot_n = egrid.db.get_conn_slot("N");

    for (tcrd, tile) in egrid.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;
        let chip = edev.chips[cell.die];
        let kind = egrid.db.tile_classes.key(tile.class);
        let x = col.to_idx();
        let y = row.to_idx();
        match &kind[..] {
            "INT" => {
                let mut naming = "INT";
                if egrid.has_bel(cell.bel(bels::DCM0)) {
                    naming = "INT.DCM0"
                }
                let nnode = namer
                    .ngrid
                    .name_tile(tcrd, naming, [format!("INT_X{x}Y{y}")]);
                nnode.tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
            }
            "INTF" => {
                // handled together with bel
            }
            "HCLK" => {
                let nnode =
                    namer
                        .ngrid
                        .name_tile(tcrd, "HCLK", [format!("HCLK_X{x}Y{y}", y = y - 1)]);
                nnode.add_bel(bels::GLOBALSIG, format!("GLOBALSIG_X{x}Y{y}", y = y / 16));
            }
            "CLB" => {
                let nnode = namer
                    .ngrid
                    .name_tile(tcrd, "CLB", [format!("CLB_X{x}Y{y}")]);
                let sx0 = clb_grid.xlut[col] * 2;
                let sx1 = clb_grid.xlut[col] * 2 + 1;
                let sy0 = clb_grid.ylut[row] * 2;
                let sy1 = clb_grid.ylut[row] * 2 + 1;
                nnode.add_bel(bels::SLICE0, format!("SLICE_X{sx0}Y{sy0}"));
                nnode.add_bel(bels::SLICE1, format!("SLICE_X{sx1}Y{sy0}"));
                nnode.add_bel(bels::SLICE2, format!("SLICE_X{sx0}Y{sy1}"));
                nnode.add_bel(bels::SLICE3, format!("SLICE_X{sx1}Y{sy1}"));
            }
            "BRAM" => {
                let name = format!("BRAM_X{x}Y{y}");
                for i in 0..4 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF.BRAM.{i}"), &name);
                }
                let nnode = namer.ngrid.name_tile(tcrd, "BRAM", [name]);
                let bx = bram_grid.xlut[col];
                let by = bram_grid.ylut[row];
                nnode.add_bel(bels::BRAM, format!("RAMB16_X{bx}Y{by}"));
                nnode.add_bel(bels::FIFO, format!("FIFO16_X{bx}Y{by}"));
            }
            "DSP" => {
                let name = format!("DSP_X{x}Y{y}");
                for i in 0..4 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF.DSP.{i}"), &name);
                }
                let nnode = namer.ngrid.name_tile(tcrd, "DSP", [name]);
                let dx = dsp_grid.xlut[col];
                let dy0 = dsp_grid.ylut[row] * 2;
                let dy1 = dsp_grid.ylut[row] * 2 + 1;
                nnode.add_bel(bels::DSP0, format!("DSP48_X{dx}Y{dy0}"));
                nnode.add_bel(bels::DSP1, format!("DSP48_X{dx}Y{dy1}"));
            }
            "IO" => {
                let naming = if col == edev.col_cfg || matches!(y % 16, 7 | 8) {
                    "IOIS_LC"
                } else {
                    "IOIS_NC"
                };
                let l = if col.to_idx() == 0 { "_L" } else { "" };
                let name = format!("{naming}{l}_X{x}Y{y}");
                namer.name_intf(cell, "INTF.IOIS", &name);
                let nnode = namer.ngrid.name_tile(tcrd, naming, [name]);
                let iox = io_grid.xlut[col];
                let ioy0 = io_grid.ylut[row] * 2;
                let ioy1 = io_grid.ylut[row] * 2 + 1;
                nnode.add_bel(bels::ILOGIC0, format!("ILOGIC_X{iox}Y{ioy0}"));
                nnode.add_bel(bels::ILOGIC1, format!("ILOGIC_X{iox}Y{ioy1}"));
                nnode.add_bel(bels::OLOGIC0, format!("OLOGIC_X{iox}Y{ioy0}"));
                nnode.add_bel(bels::OLOGIC1, format!("OLOGIC_X{iox}Y{ioy1}"));
                nnode.add_bel(bels::IOB0, format!("IOB_X{iox}Y{ioy0}"));
                nnode.add_bel(bels::IOB1, format!("IOB_X{iox}Y{ioy1}"));
            }
            "DCM" => {
                let naming = if row < chip.row_reg_bot(chip.reg_cfg) {
                    "DCM_BOT"
                } else {
                    "DCM"
                };
                let name = format!("{naming}_X{x}Y{y}");
                for i in 0..4 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF.DCM.{i}"), &name);
                }
                let nnode = namer.ngrid.name_tile(tcrd, naming, [name]);
                let dx = dcm_grid.xlut[col];
                let dy = dcm_grid.ylut[row];
                nnode.add_bel(bels::DCM0, format!("DCM_ADV_X{dx}Y{dy}"));
            }
            "CCM" => {
                let name = format!("CCM_X{x}Y{y}");
                for i in 0..4 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF.CCM.{i}"), &name);
                }
                let nnode = namer.ngrid.name_tile(tcrd, "CCM", [name]);
                let cx = ccm_grid.xlut[col];
                let cy = ccm_grid.ylut[row];
                nnode.add_bel(bels::PMCD0, format!("PMCD_X{cx}Y{y}", y = cy * 2));
                nnode.add_bel(bels::PMCD1, format!("PMCD_X{cx}Y{y}", y = cy * 2 + 1));
                nnode.add_bel(bels::DPM, format!("DPM_X{cx}Y{cy}"));
            }
            "SYSMON" => {
                let name = format!("SYS_MON_X{x}Y{y}");
                for i in 0..8 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF.SYSMON.{i}"), &name);
                }
                let nnode = namer.ngrid.name_tile(tcrd, "SYSMON", [name]);
                let sx = sysmon_grid.xlut[col];
                let sy = sysmon_grid.ylut[row];
                let ipx = if edev.col_lgt.is_some() { 1 } else { 0 };
                let ipy0 = if row.to_idx() == 0 {
                    0
                } else if edev.col_lgt.is_some() {
                    chip.regs * 3
                } else {
                    2
                };
                let ipy1 = ipy0 + 1;
                nnode.add_bel(bels::SYSMON, format!("MONITOR_X{sx}Y{sy}"));
                nnode.add_bel(bels::IPAD_VP, format!("IPAD_X{ipx}Y{ipy0}"));
                nnode.add_bel(bels::IPAD_VN, format!("IPAD_X{ipx}Y{ipy1}"));
            }
            "CFG" => {
                let name = format!("CFG_CENTER_X{x}Y{y}", y = y + 7);
                for i in 0..16 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF.CFG.{i}"), &name);
                }
                let name_bufg_b = format!("CLK_BUFGCTRL_B_X{x}Y{y}");
                let name_bufg_t = format!("CLK_BUFGCTRL_T_X{x}Y{y}", y = y + 8);
                let name_hrow_b = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                let name_hrow_t = format!("CLK_HROW_X{x}Y{y}", y = y + 15);
                let name_hclk_b = format!("HCLK_CENTER_X{x}Y{y}", y = y - 1);
                let name_hclk_t = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y + 15);
                let nnode = namer.ngrid.name_tile(
                    tcrd,
                    "CFG",
                    [
                        name,
                        name_bufg_b,
                        name_bufg_t,
                        name_hrow_b,
                        name_hrow_t,
                        name_hclk_b,
                        name_hclk_t,
                    ],
                );
                for i in 0..32 {
                    nnode.add_bel(bels::BUFGCTRL[i], format!("BUFGCTRL_X0Y{i}"));
                }
                for i in 0..4 {
                    nnode.add_bel(bels::BSCAN[i], format!("BSCAN_X0Y{i}"));
                }
                for i in 0..2 {
                    nnode.add_bel(bels::ICAP[i], format!("ICAP_X0Y{i}"));
                }
                nnode.add_bel(bels::PMV0, "PMV".to_string());
                nnode.add_bel(bels::STARTUP, "STARTUP".to_string());
                nnode.add_bel(bels::JTAGPPC, "JTAGPPC".to_string());
                nnode.add_bel(bels::FRAME_ECC, "FRAME_ECC".to_string());
                nnode.add_bel(bels::DCIRESET, "DCIRESET".to_string());
                nnode.add_bel(bels::CAPTURE, "CAPTURE".to_string());
                nnode.add_bel(bels::USR_ACCESS, "USR_ACCESS_SITE".to_string());
            }
            "PPC" => {
                let name_b = format!("PB_X{x}Y{y}", y = y + 3);
                let name_t = format!("PT_X{x}Y{y}", y = y + 19);
                for dy in 0..24 {
                    let name = if dy < 12 { &name_b } else { &name_t };
                    namer.name_intf(cell.delta(0, dy), &format!("INTF.PPC.L{dy}"), name);
                    namer.name_intf(cell.delta(8, dy), &format!("INTF.PPC.R{dy}"), name);
                }
                for dy in 0..22 {
                    let name = if dy < 11 { &name_b } else { &name_t };
                    namer.ngrid.name_conn_tile(
                        cell.delta(0, 1 + dy).connector(term_slot_e),
                        &format!("TERM.PPC.E{dy}"),
                        name.into(),
                    );
                    namer.ngrid.name_conn_tile(
                        cell.delta(8, 1 + dy).connector(term_slot_w),
                        &format!("TERM.PPC.W{dy}"),
                        name.into(),
                    );
                }
                for dx in 0..7 {
                    namer.name_intf(cell.delta(dx + 1, 0), &format!("INTF.PPC.B{dx}"), &name_b);
                    namer.name_intf(cell.delta(dx + 1, 23), &format!("INTF.PPC.T{dx}"), &name_t);
                    namer.ngrid.name_conn_pair(
                        cell.delta(dx + 1, 0).connector(term_slot_n),
                        &format!("TERM.PPC.N{dx}"),
                        name_b.clone(),
                        name_t.clone(),
                    );
                    namer.ngrid.name_conn_pair(
                        cell.delta(dx + 1, 23).connector(term_slot_s),
                        &format!("TERM.PPC.S{dx}"),
                        name_t.clone(),
                        name_b.clone(),
                    );
                }
                let nnode = namer.ngrid.name_tile(tcrd, "PPC", [name_b, name_t]);
                let px = ppc_grid.xlut[col];
                let py = ppc_grid.ylut[row];
                nnode.add_bel(bels::PPC, format!("PPC405_ADV_X{px}Y{py}"));
                nnode.add_bel(bels::EMAC, format!("EMAC_X{px}Y{py}"));
            }
            "MGT" => {
                let lr = if Some(col) == edev.col_lgt { 'L' } else { 'R' };
                let name0 = format!("MGT_B{lr}_X{x}Y{y}", y = y + 8);
                let name1 = format!("MGT_A{lr}_X{x}Y{y}", y = y + 24);
                let name_clk = format!("BRKH_MGT11CLK_{lr}_X{x}Y{y}", y = y + 15);
                let naming = if lr == 'L' { "MGT.L" } else { "MGT.R" };
                for (br, name) in [(0, &name0), (16, &name1)] {
                    for i in 0..16 {
                        namer.name_intf(cell.delta(0, br + i), &format!("INTF.MGT.{i}"), name);
                        if lr == 'L' {
                            namer.ngrid.name_conn_tile(
                                cell.delta(0, br + i).connector(term_slot_w),
                                &format!("TERM.W.MGT{i}"),
                                name.into(),
                            );
                        } else {
                            namer.ngrid.name_conn_tile(
                                cell.delta(0, br + i).connector(term_slot_e),
                                &format!("TERM.E.MGT{i}"),
                                name.into(),
                            );
                        }
                    }
                }
                let nnode = namer
                    .ngrid
                    .name_tile(tcrd, naming, [name0, name1, name_clk]);
                let gtx = mgt_grid.xlut[col];
                let gty = mgt_grid.ylut[row];
                let has_bot_sysmon = chip
                    .rows_cfg
                    .contains(&(RowId::from_idx(0), CfgRowKind::Sysmon));
                let ipx = if col.to_idx() == 0 {
                    0
                } else if has_bot_sysmon {
                    2
                } else {
                    1
                };
                let ipy = 6 * gty + if has_bot_sysmon { 2 } else { 0 };
                nnode.add_bel(bels::GT11_0, format!("GT11_X{gtx}Y{gty}", gty = gty * 2));
                nnode.add_bel(
                    bels::GT11_1,
                    format!("GT11_X{gtx}Y{gty}", gty = gty * 2 + 1),
                );
                nnode.add_bel(bels::IPAD_RXP0, format!("IPAD_X{ipx}Y{ipy}"));
                nnode.add_bel(bels::IPAD_RXN0, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1));
                nnode.add_bel(bels::OPAD_TXP0, format!("OPAD_X{gtx}Y{opy}", opy = gty * 4));
                nnode.add_bel(
                    bels::OPAD_TXN0,
                    format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 1),
                );
                nnode.add_bel(bels::IPAD_RXP1, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 4));
                nnode.add_bel(bels::IPAD_RXN1, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 5));
                nnode.add_bel(
                    bels::OPAD_TXP1,
                    format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 2),
                );
                nnode.add_bel(
                    bels::OPAD_TXN1,
                    format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 3),
                );
                nnode.add_bel(bels::GT11CLK, format!("GT11CLK_X{gtx}Y{gty}"));
                nnode.add_bel(
                    bels::IPAD_CLKP0,
                    format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 3),
                );
                nnode.add_bel(
                    bels::IPAD_CLKN0,
                    format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 2),
                );
            }

            "HCLK_IOIS_DCI" | "HCLK_IOIS_LVDS" => {
                let l = if col.to_idx() == 0 { "_L" } else { "" };
                let name = format!("{kind}_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_NC{l}_X{x}Y{y}", y = y - 2);
                let name_io1 = format!("IOIS_LC{l}_X{x}Y{y}", y = y - 1);
                let name_io2 = format!("IOIS_LC{l}_X{x}Y{y}");
                let nnode = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_io0, name_io1, name_io2]);
                let iox = io_grid.xlut[col];
                let reg = chip.row_to_reg(row).to_idx();
                let brx = if Some(col) == edev.col_lio { 0 } else { 1 };
                nnode.add_bel(bels::BUFR0, format!("BUFR_X{brx}Y{y}", y = reg * 2 + 1));
                nnode.add_bel(bels::BUFR1, format!("BUFR_X{brx}Y{y}", y = reg * 2));
                nnode.add_bel(bels::BUFIO0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                nnode.add_bel(bels::BUFIO1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                nnode.add_bel(bels::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                if kind == "HCLK_IOIS_DCI" {
                    let dciy = dci_grid.ylut[row];
                    nnode.add_bel(bels::DCI, format!("DCI_X{iox}Y{dciy}"));
                }
            }
            "HCLK_CENTER" => {
                let name = format!("HCLK_CENTER_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                let nnode = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_io0, name_io1]);
                let iox = io_grid.xlut[col];
                let dciy = dci_grid.ylut[row];
                let reg = chip.row_to_reg(row).to_idx();
                nnode.add_bel(bels::BUFIO0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                nnode.add_bel(bels::BUFIO1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                nnode.add_bel(bels::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                nnode.add_bel(bels::DCI, format!("DCI_X{iox}Y{dciy}"));
            }
            "HCLK_CENTER_ABOVE_CFG" => {
                let name = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                let nnode = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_io0, name_io1]);
                let iox = io_grid.xlut[col];
                let dciy = dci_grid.ylut[row];
                let reg = chip.row_to_reg(row).to_idx();
                nnode.add_bel(bels::BUFIO0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                nnode.add_bel(bels::BUFIO1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                nnode.add_bel(bels::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                nnode.add_bel(bels::DCI, format!("DCI_X{iox}Y{dciy}"));
            }
            "HCLK_DCMIOB" => {
                let name = format!("HCLK_DCMIOB_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                let nnode =
                    namer
                        .ngrid
                        .name_tile(tcrd, kind, [name, name_io0, name_io1, name_hrow]);
                let iox = io_grid.xlut[col];
                let dciy = dci_grid.ylut[row];
                let reg = chip.row_to_reg(row).to_idx();
                nnode.add_bel(bels::BUFIO0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                nnode.add_bel(bels::BUFIO1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                nnode.add_bel(bels::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                nnode.add_bel(bels::DCI, format!("DCI_X{iox}Y{dciy}"));
            }
            "HCLK_IOBDCM" => {
                let name = format!("HCLK_IOBDCM_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                let nnode =
                    namer
                        .ngrid
                        .name_tile(tcrd, kind, [name, name_io0, name_io1, name_hrow]);
                let iox = io_grid.xlut[col];
                let dciy = dci_grid.ylut[row];
                let reg = chip.row_to_reg(row).to_idx();
                nnode.add_bel(bels::BUFIO0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                nnode.add_bel(bels::BUFIO1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                nnode.add_bel(bels::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                nnode.add_bel(bels::DCI, format!("DCI_X{iox}Y{dciy}"));
            }
            "HCLK_DCM" => {
                let name = format!("HCLK_DCM_X{x}Y{y}", y = y - 1);
                let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                namer.ngrid.name_tile(tcrd, "HCLK_DCM", [name, name_hrow]);
            }

            "CLK_HROW" => {
                let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                namer.ngrid.name_tile(tcrd, "CLK_HROW", [name_hrow]);
            }
            "CLK_DCM_B" | "CLK_DCM_T" => {
                let bt = if row < chip.row_reg_bot(chip.reg_cfg) {
                    'B'
                } else {
                    'T'
                };
                let name = format!("CLKV_DCM_{bt}_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            "CLK_IOB_B" | "CLK_IOB_T" => {
                let bt = if row < chip.row_reg_bot(chip.reg_cfg) {
                    'B'
                } else {
                    'T'
                };
                let name = format!("CLK_IOB_{bt}_X{x}Y{y}", y = y + 7);
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            "CLK_TERM" | "HCLK_TERM" | "HCLK_MGT" | "HCLK_MGT_REPEATER" => (),
            _ => unreachable!(),
        }
    }
    for (ccrd, conn) in egrid.connectors() {
        let CellCoord { col, row, .. } = ccrd.cell;
        let kind = egrid.db.conn_classes.key(conn.class);
        let x = col.to_idx();
        let y = row.to_idx();

        match &kind[..] {
            "TERM.W" => {
                if edev.col_lgt.is_none() {
                    let name = format!("L_TERM_INT_X{x}Y{y}");
                    namer.ngrid.name_conn_tile(ccrd, "TERM.W", name);
                }
            }
            "TERM.E" => {
                if edev.col_rgt.is_none() {
                    let name = format!("R_TERM_INT_X{x}Y{y}");
                    namer.ngrid.name_conn_tile(ccrd, "TERM.E", name);
                }
            }
            "TERM.S" => {
                let name = format!("B_TERM_INT_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM.S", name);
            }
            "TERM.N" => {
                let name = format!("T_TERM_INT_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM.N", name);
            }
            "CLB_BUFFER.W" => {
                let name = format!("CLB_BUFFER_X{x}Y{y}", x = x - 1);
                namer.ngrid.name_conn_tile(ccrd, "PASS.CLB_BUFFER.E", name);
            }
            "CLB_BUFFER.E" => {
                let name = format!("CLB_BUFFER_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "PASS.CLB_BUFFER.W", name);
            }
            _ => (),
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid: namer.ngrid,
        gtz: Default::default(),
    }
}
