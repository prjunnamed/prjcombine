use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{CellCoord, RowId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{
    chip::CfgRowKind,
    defs::{
        bslots, tslots,
        virtex4::{ccls, tcls},
    },
    expanded::ExpandedDevice,
};

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
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);

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
    let dci_grid = ngrid.bel_grid(|_, _, tcls| tcls.bels.contains_id(bslots::DCI));

    let mut namer = Namer { ngrid };

    let term_slot_w = edev.db.get_conn_slot("W");
    let term_slot_e = edev.db.get_conn_slot("E");
    let term_slot_s = edev.db.get_conn_slot("S");
    let term_slot_n = edev.db.get_conn_slot("N");

    for (tcrd, tile) in edev.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;
        let chip = edev.chips[cell.die];
        let kind = edev.db.tile_classes.key(tile.class);
        let x = col.to_idx();
        let y = row.to_idx();
        match tile.class {
            tcls::INT => {
                let mut naming = "INT";
                if edev.has_bel(cell.bel(bslots::DCM[0])) {
                    naming = "INT_DCM0"
                }
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, naming, [format!("INT_X{x}Y{y}")]);
                ntile.tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
            }
            tcls::INTF => {
                // handled together with bel
            }
            tcls::HCLK => {
                let ntile =
                    namer
                        .ngrid
                        .name_tile(tcrd, "HCLK", [format!("HCLK_X{x}Y{y}", y = y - 1)]);
                ntile.add_bel(bslots::GLOBALSIG, format!("GLOBALSIG_X{x}Y{y}", y = y / 16));
            }
            tcls::CLB => {
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, "CLB", [format!("CLB_X{x}Y{y}")]);
                let sx0 = clb_grid.xlut[col] * 2;
                let sx1 = clb_grid.xlut[col] * 2 + 1;
                let sy0 = clb_grid.ylut[row] * 2;
                let sy1 = clb_grid.ylut[row] * 2 + 1;
                ntile.add_bel(bslots::SLICE[0], format!("SLICE_X{sx0}Y{sy0}"));
                ntile.add_bel(bslots::SLICE[1], format!("SLICE_X{sx1}Y{sy0}"));
                ntile.add_bel(bslots::SLICE[2], format!("SLICE_X{sx0}Y{sy1}"));
                ntile.add_bel(bslots::SLICE[3], format!("SLICE_X{sx1}Y{sy1}"));
            }
            tcls::BRAM => {
                let name = format!("BRAM_X{x}Y{y}");
                for i in 0..4 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF_BRAM_{i}"), &name);
                }
                let ntile = namer.ngrid.name_tile(tcrd, "BRAM", [name]);
                let bx = bram_grid.xlut[col];
                let by = bram_grid.ylut[row];
                ntile.add_bel_multi(
                    bslots::BRAM,
                    [format!("RAMB16_X{bx}Y{by}"), format!("FIFO16_X{bx}Y{by}")],
                );
            }
            tcls::DSP => {
                let name = format!("DSP_X{x}Y{y}");
                for i in 0..4 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF_DSP_{i}"), &name);
                }
                let ntile = namer.ngrid.name_tile(tcrd, "DSP", [name]);
                let dx = dsp_grid.xlut[col];
                let dy0 = dsp_grid.ylut[row] * 2;
                let dy1 = dsp_grid.ylut[row] * 2 + 1;
                ntile.add_bel(bslots::DSP[0], format!("DSP48_X{dx}Y{dy0}"));
                ntile.add_bel(bslots::DSP[1], format!("DSP48_X{dx}Y{dy1}"));
            }
            tcls::IO => {
                let naming = if col == edev.col_cfg || matches!(y % 16, 7 | 8) {
                    "IOIS_LC"
                } else {
                    "IOIS_NC"
                };
                let l = if col.to_idx() == 0 { "_L" } else { "" };
                let name = format!("{naming}{l}_X{x}Y{y}");
                namer.name_intf(cell, "INTF_IOIS", &name);
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let iox = io_grid.xlut[col];
                let ioy0 = io_grid.ylut[row] * 2;
                let ioy1 = io_grid.ylut[row] * 2 + 1;
                ntile.add_bel(bslots::ILOGIC[0], format!("ILOGIC_X{iox}Y{ioy0}"));
                ntile.add_bel(bslots::ILOGIC[1], format!("ILOGIC_X{iox}Y{ioy1}"));
                ntile.add_bel(bslots::OLOGIC[0], format!("OLOGIC_X{iox}Y{ioy0}"));
                ntile.add_bel(bslots::OLOGIC[1], format!("OLOGIC_X{iox}Y{ioy1}"));
                ntile.add_bel(bslots::IOB[0], format!("IOB_X{iox}Y{ioy0}"));
                ntile.add_bel(bslots::IOB[1], format!("IOB_X{iox}Y{ioy1}"));
            }
            tcls::DCM => {
                let naming = if row < chip.row_reg_bot(chip.reg_cfg) {
                    "DCM_BOT"
                } else {
                    "DCM"
                };
                let name = format!("{naming}_X{x}Y{y}");
                for i in 0..4 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF_DCM_{i}"), &name);
                }
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name]);
                let dx = dcm_grid.xlut[col];
                let dy = dcm_grid.ylut[row];
                ntile.add_bel(bslots::DCM[0], format!("DCM_ADV_X{dx}Y{dy}"));
            }
            tcls::CCM => {
                let name = format!("CCM_X{x}Y{y}");
                for i in 0..4 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF_CCM_{i}"), &name);
                }
                let ntile = namer.ngrid.name_tile(tcrd, "CCM", [name]);
                let cx = ccm_grid.xlut[col];
                let cy = ccm_grid.ylut[row];
                ntile.add_bel(bslots::PMCD[0], format!("PMCD_X{cx}Y{y}", y = cy * 2));
                ntile.add_bel(bslots::PMCD[1], format!("PMCD_X{cx}Y{y}", y = cy * 2 + 1));
                ntile.add_bel(bslots::DPM, format!("DPM_X{cx}Y{cy}"));
            }
            tcls::SYSMON => {
                let name = format!("SYS_MON_X{x}Y{y}");
                for i in 0..8 {
                    namer.name_intf(cell.delta(0, i), &format!("INTF_SYSMON_{i}"), &name);
                }
                let ntile = namer.ngrid.name_tile(tcrd, "SYSMON", [name]);
                let sx = sysmon_grid.xlut[col];
                let sy = sysmon_grid.ylut[row];
                let ipx = if edev.col_gt_w.is_some() { 1 } else { 0 };
                let ipy0 = if row.to_idx() == 0 {
                    0
                } else if edev.col_gt_w.is_some() {
                    chip.regs * 3
                } else {
                    2
                };
                let ipy1 = ipy0 + 1;
                ntile.add_bel_multi(
                    bslots::SYSMON,
                    [
                        format!("MONITOR_X{sx}Y{sy}"),
                        format!("IPAD_X{ipx}Y{ipy0}"),
                        format!("IPAD_X{ipx}Y{ipy1}"),
                    ],
                );
            }
            tcls::CFG => {
                let name = format!("CFG_CENTER_X{x}Y{y}", y = y - 1);
                for i in 0..16 {
                    namer.name_intf(cell.delta(0, -8 + i), &format!("INTF_CFG_{i}"), &name);
                }
                let ntile = namer.ngrid.name_tile(tcrd, "CFG", [name]);
                for i in 0..4 {
                    ntile.add_bel(bslots::BSCAN[i], format!("BSCAN_X0Y{i}"));
                }
                for i in 0..2 {
                    ntile.add_bel(bslots::ICAP[i], format!("ICAP_X0Y{i}"));
                }
                ntile.add_bel(bslots::PMV_CFG[0], "PMV".to_string());
                ntile.add_bel(bslots::STARTUP, "STARTUP".to_string());
                ntile.add_bel(bslots::JTAGPPC, "JTAGPPC".to_string());
                ntile.add_bel(bslots::FRAME_ECC, "FRAME_ECC".to_string());
                ntile.add_bel(bslots::DCIRESET, "DCIRESET".to_string());
                ntile.add_bel(bslots::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(bslots::USR_ACCESS, "USR_ACCESS_SITE".to_string());
            }
            tcls::CLK_BUFG => {
                let name = format!("CFG_CENTER_X{x}Y{y}", y = y - 1);
                let name_bufg_b = format!("CLK_BUFGCTRL_B_X{x}Y{y}", y = y - 8);
                let name_bufg_t = format!("CLK_BUFGCTRL_T_X{x}Y{y}");
                let name_hrow_b = format!("CLK_HROW_X{x}Y{y}", y = y - 9);
                let name_hrow_t = format!("CLK_HROW_X{x}Y{y}", y = y + 7);
                let name_hclk_b = format!("HCLK_CENTER_X{x}Y{y}", y = y - 9);
                let name_hclk_t = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y + 7);
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    "CLK_BUFG",
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
                    ntile.add_bel(bslots::BUFGCTRL[i], format!("BUFGCTRL_X0Y{i}"));
                }
            }
            tcls::PPC => {
                let name_b = format!("PB_X{x}Y{y}", y = y + 3);
                let name_t = format!("PT_X{x}Y{y}", y = y + 19);
                for dy in 0..24 {
                    let name = if dy < 12 { &name_b } else { &name_t };
                    namer.name_intf(cell.delta(0, dy), &format!("INTF_PPC_W{dy}"), name);
                    namer.name_intf(cell.delta(8, dy), &format!("INTF_PPC_E{dy}"), name);
                }
                for dy in 0..22 {
                    let name = if dy < 11 { &name_b } else { &name_t };
                    namer.ngrid.name_conn_tile(
                        cell.delta(0, 1 + dy).connector(term_slot_e),
                        &format!("TERM_PPC_E{dy}"),
                        name.into(),
                    );
                    namer.ngrid.name_conn_tile(
                        cell.delta(8, 1 + dy).connector(term_slot_w),
                        &format!("TERM_PPC_W{dy}"),
                        name.into(),
                    );
                }
                for dx in 0..7 {
                    namer.name_intf(cell.delta(dx + 1, 0), &format!("INTF_PPC_S{dx}"), &name_b);
                    namer.name_intf(cell.delta(dx + 1, 23), &format!("INTF_PPC_N{dx}"), &name_t);
                    namer.ngrid.name_conn_pair(
                        cell.delta(dx + 1, 0).connector(term_slot_n),
                        &format!("TERM_PPC_N{dx}"),
                        name_b.clone(),
                        name_t.clone(),
                    );
                    namer.ngrid.name_conn_pair(
                        cell.delta(dx + 1, 23).connector(term_slot_s),
                        &format!("TERM_PPC_S{dx}"),
                        name_t.clone(),
                        name_b.clone(),
                    );
                }
                let ntile = namer.ngrid.name_tile(tcrd, "PPC", [name_b, name_t]);
                let px = ppc_grid.xlut[col];
                let py = ppc_grid.ylut[row];
                ntile.add_bel(bslots::PPC, format!("PPC405_ADV_X{px}Y{py}"));
                ntile.add_bel(bslots::EMAC, format!("EMAC_X{px}Y{py}"));
            }
            tcls::MGT => {
                let lr = if Some(col) == edev.col_gt_w { 'L' } else { 'R' };
                let name0 = format!("MGT_B{lr}_X{x}Y{y}", y = y + 8);
                let name1 = format!("MGT_A{lr}_X{x}Y{y}", y = y + 24);
                let name_clk = format!("BRKH_MGT11CLK_{lr}_X{x}Y{y}", y = y + 15);
                let naming = if lr == 'L' { "MGT_W" } else { "MGT_E" };
                for (br, name) in [(0, &name0), (16, &name1)] {
                    for i in 0..16 {
                        namer.name_intf(cell.delta(0, br + i), &format!("INTF_MGT_{i}"), name);
                        if lr == 'L' {
                            namer.ngrid.name_conn_tile(
                                cell.delta(0, br + i).connector(term_slot_w),
                                &format!("TERM_W_MGT{i}"),
                                name.into(),
                            );
                        } else {
                            namer.ngrid.name_conn_tile(
                                cell.delta(0, br + i).connector(term_slot_e),
                                &format!("TERM_E_MGT{i}"),
                                name.into(),
                            );
                        }
                    }
                }
                let ntile = namer
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
                ntile.add_bel_multi(
                    bslots::GT11[0],
                    [
                        format!("GT11_X{gtx}Y{gty}", gty = gty * 2),
                        format!("IPAD_X{ipx}Y{ipy}"),
                        format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1),
                        format!("OPAD_X{gtx}Y{opy}", opy = gty * 4),
                        format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 1),
                    ],
                );
                ntile.add_bel_multi(
                    bslots::GT11[1],
                    [
                        format!("GT11_X{gtx}Y{gty}", gty = gty * 2 + 1),
                        format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 4),
                        format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 5),
                        format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 2),
                        format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 3),
                    ],
                );
                ntile.add_bel_multi(
                    bslots::GT11CLK,
                    [
                        format!("GT11CLK_X{gtx}Y{gty}"),
                        format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 3),
                        format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 2),
                    ],
                );
            }

            tcls::HCLK_IO_DCI | tcls::HCLK_IO_LVDS => {
                let tkn = match tile.class {
                    tcls::HCLK_IO_DCI => "HCLK_IOIS_DCI",
                    tcls::HCLK_IO_LVDS => "HCLK_IOIS_LVDS",
                    _ => unreachable!(),
                };
                let l = if col.to_idx() == 0 { "_L" } else { "" };
                let name = format!("{tkn}_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_NC{l}_X{x}Y{y}", y = y - 2);
                let name_io1 = format!("IOIS_LC{l}_X{x}Y{y}", y = y - 1);
                let name_io2 = format!("IOIS_LC{l}_X{x}Y{y}");
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_io0, name_io1, name_io2]);
                let iox = io_grid.xlut[col];
                let reg = chip.row_to_reg(row).to_idx();
                let brx = if Some(col) == edev.col_io_w { 0 } else { 1 };
                ntile.add_bel(bslots::BUFR[0], format!("BUFR_X{brx}Y{y}", y = reg * 2 + 1));
                ntile.add_bel(bslots::BUFR[1], format!("BUFR_X{brx}Y{y}", y = reg * 2));
                ntile.add_bel(
                    bslots::BUFIO[0],
                    format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1),
                );
                ntile.add_bel(bslots::BUFIO[1], format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                ntile.add_bel(bslots::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                if tile.class == tcls::HCLK_IO_DCI {
                    let dciy = dci_grid.ylut[row];
                    ntile.add_bel(bslots::DCI, format!("DCI_X{iox}Y{dciy}"));
                }
            }
            tcls::HCLK_IO_CENTER => {
                let name = format!("HCLK_CENTER_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_io0, name_io1]);
                let iox = io_grid.xlut[col];
                let dciy = dci_grid.ylut[row];
                let reg = chip.row_to_reg(row).to_idx();
                ntile.add_bel(
                    bslots::BUFIO[0],
                    format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1),
                );
                ntile.add_bel(bslots::BUFIO[1], format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                ntile.add_bel(bslots::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                ntile.add_bel(bslots::DCI, format!("DCI_X{iox}Y{dciy}"));
            }
            tcls::HCLK_IO_CFG_N => {
                let name = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_io0, name_io1]);
                let iox = io_grid.xlut[col];
                let dciy = dci_grid.ylut[row];
                let reg = chip.row_to_reg(row).to_idx();
                ntile.add_bel(
                    bslots::BUFIO[0],
                    format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1),
                );
                ntile.add_bel(bslots::BUFIO[1], format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                ntile.add_bel(bslots::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                ntile.add_bel(bslots::DCI, format!("DCI_X{iox}Y{dciy}"));
            }
            tcls::HCLK_IO_DCM_N => {
                let name = format!("HCLK_DCMIOB_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_io0, name_io1]);
                let iox = io_grid.xlut[col];
                let dciy = dci_grid.ylut[row];
                let reg = chip.row_to_reg(row).to_idx();
                ntile.add_bel(
                    bslots::BUFIO[0],
                    format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1),
                );
                ntile.add_bel(bslots::BUFIO[1], format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                ntile.add_bel(bslots::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                ntile.add_bel(bslots::DCI, format!("DCI_X{iox}Y{dciy}"));
            }
            tcls::HCLK_IO_DCM_S => {
                let name = format!("HCLK_IOBDCM_X{x}Y{y}", y = y - 1);
                let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_io0, name_io1]);
                let iox = io_grid.xlut[col];
                let dciy = dci_grid.ylut[row];
                let reg = chip.row_to_reg(row).to_idx();
                ntile.add_bel(
                    bslots::BUFIO[0],
                    format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1),
                );
                ntile.add_bel(bslots::BUFIO[1], format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                ntile.add_bel(bslots::IDELAYCTRL, format!("IDELAYCTRL_X{iox}Y{reg}"));
                ntile.add_bel(bslots::DCI, format!("DCI_X{iox}Y{dciy}"));
            }
            tcls::HCLK_DCM => {
                let name = format!("HCLK_DCM_X{x}Y{y}", y = y - 1);
                namer.ngrid.name_tile(tcrd, "HCLK_DCM", [name]);
            }

            tcls::CLK_HROW => {
                let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                namer.ngrid.name_tile(tcrd, "CLK_HROW", [name_hrow]);
            }
            tcls::CLK_DCM_S | tcls::CLK_DCM_N => {
                let bt = if row < chip.row_reg_bot(chip.reg_cfg) {
                    'B'
                } else {
                    'T'
                };
                let name = format!("CLKV_DCM_{bt}_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            tcls::CLK_IOB_S | tcls::CLK_IOB_N => {
                let bt = if row < chip.row_reg_bot(chip.reg_cfg) {
                    'B'
                } else {
                    'T'
                };
                let name = format!("CLK_IOB_{bt}_X{x}Y{y}", y = y + 7);
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            tcls::CLK_TERM
            | tcls::HCLK_TERM
            | tcls::HCLK_MGT
            | tcls::HCLK_MGT_BUF
            | tcls::GLOBAL => (),
            _ => unreachable!(),
        }
    }
    for (ccrd, conn) in edev.connectors() {
        let CellCoord { col, row, .. } = ccrd.cell;
        let x = col.to_idx();
        let y = row.to_idx();

        match conn.class {
            ccls::TERM_W => {
                if edev.col_gt_w.is_none() {
                    let name = format!("L_TERM_INT_X{x}Y{y}");
                    namer.ngrid.name_conn_tile(ccrd, "TERM_W", name);
                }
            }
            ccls::TERM_E => {
                if edev.col_gt_e.is_none() {
                    let name = format!("R_TERM_INT_X{x}Y{y}");
                    namer.ngrid.name_conn_tile(ccrd, "TERM_E", name);
                }
            }
            ccls::TERM_S => {
                let name = format!("B_TERM_INT_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM_S", name);
            }
            ccls::TERM_N => {
                let name = format!("T_TERM_INT_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM_N", name);
            }
            ccls::CLB_BUFFER_W => {
                let name = format!("CLB_BUFFER_X{x}Y{y}", x = x - 1);
                namer.ngrid.name_conn_tile(ccrd, "CLB_BUFFER_W", name);
            }
            ccls::CLB_BUFFER_E => {
                let name = format!("CLB_BUFFER_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "CLB_BUFFER_E", name);
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
