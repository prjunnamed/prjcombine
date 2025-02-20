use prjcombine_interconnect::{
    db::Dir,
    grid::{ColId, DieId, ExpandedDieRef, RowId},
};
use prjcombine_virtex4::{expanded::ExpandedDevice, grid::CfgRowKind};
use prjcombine_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use unnamed_entity::EntityId;

use crate::ExpandedNamedDevice;

struct Namer<'a> {
    edev: &'a ExpandedDevice<'a>,
    die: ExpandedDieRef<'a, 'a>,
    ngrid: ExpandedGridNaming<'a>,
}

impl Namer<'_> {
    fn name_intf(&mut self, col: ColId, row: RowId, naming: &str, name: &str) {
        let ilayer = self
            .edev
            .egrid
            .find_node_layer(self.die.die, (col, row), |node| node == "INTF")
            .unwrap();
        self.ngrid
            .name_node((self.die.die, col, row, ilayer), naming, [name.into()]);
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

    let mut namer = Namer {
        edev,
        die: egrid.die(DieId::from_idx(0)),
        ngrid,
    };

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
                            let mut naming = "INT";
                            if egrid
                                .find_node_layer(die.die, (col, row), |node| node == "DCM")
                                .is_some()
                            {
                                naming = "INT.DCM0"
                            }
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, naming, [format!("INT_X{x}Y{y}")]);
                            nnode.tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
                        }
                        "INTF" => {
                            // handled together with bel
                        }
                        "HCLK" => {
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                "HCLK",
                                [format!("HCLK_X{x}Y{y}", y = y - 1)],
                            );
                            nnode.add_bel(0, format!("GLOBALSIG_X{x}Y{y}", y = y / 16));
                        }
                        "CLB" => {
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, "CLB", [format!("CLB_X{x}Y{y}")]);
                            let sx0 = clb_grid.xlut[col] * 2;
                            let sx1 = clb_grid.xlut[col] * 2 + 1;
                            let sy0 = clb_grid.ylut[row] * 2;
                            let sy1 = clb_grid.ylut[row] * 2 + 1;
                            nnode.add_bel(0, format!("SLICE_X{sx0}Y{sy0}"));
                            nnode.add_bel(1, format!("SLICE_X{sx1}Y{sy0}"));
                            nnode.add_bel(2, format!("SLICE_X{sx0}Y{sy1}"));
                            nnode.add_bel(3, format!("SLICE_X{sx1}Y{sy1}"));
                        }
                        "BRAM" => {
                            let name = format!("BRAM_X{x}Y{y}");
                            for tile in 0..4 {
                                namer.name_intf(
                                    col,
                                    row + tile,
                                    &format!("INTF.BRAM.{tile}"),
                                    &name,
                                );
                            }
                            let nnode = namer.ngrid.name_node(nloc, "BRAM", [name]);
                            let bx = bram_grid.xlut[col];
                            let by = bram_grid.ylut[row];
                            nnode.add_bel(0, format!("RAMB16_X{bx}Y{by}"));
                            nnode.add_bel(1, format!("FIFO16_X{bx}Y{by}"));
                        }
                        "DSP" => {
                            let name = format!("DSP_X{x}Y{y}");
                            for tile in 0..4 {
                                namer.name_intf(
                                    col,
                                    row + tile,
                                    &format!("INTF.DSP.{tile}"),
                                    &name,
                                );
                            }
                            let nnode = namer.ngrid.name_node(nloc, "DSP", [name]);
                            let dx = dsp_grid.xlut[col];
                            let dy0 = dsp_grid.ylut[row] * 2;
                            let dy1 = dsp_grid.ylut[row] * 2 + 1;
                            nnode.add_bel(0, format!("DSP48_X{dx}Y{dy0}"));
                            nnode.add_bel(1, format!("DSP48_X{dx}Y{dy1}"));
                        }
                        "IO" => {
                            let naming = if col == edev.col_cfg || matches!(y % 16, 7 | 8) {
                                "IOIS_LC"
                            } else {
                                "IOIS_NC"
                            };
                            let l = if col.to_idx() == 0 { "_L" } else { "" };
                            let name = format!("{naming}{l}_X{x}Y{y}");
                            namer.name_intf(col, row, "INTF.IOIS", &name);
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let iox = io_grid.xlut[col];
                            let ioy0 = io_grid.ylut[row] * 2;
                            let ioy1 = io_grid.ylut[row] * 2 + 1;
                            nnode.add_bel(0, format!("ILOGIC_X{iox}Y{ioy0}"));
                            nnode.add_bel(1, format!("ILOGIC_X{iox}Y{ioy1}"));
                            nnode.add_bel(2, format!("OLOGIC_X{iox}Y{ioy0}"));
                            nnode.add_bel(3, format!("OLOGIC_X{iox}Y{ioy1}"));
                            nnode.add_bel(4, format!("IOB_X{iox}Y{ioy0}"));
                            nnode.add_bel(5, format!("IOB_X{iox}Y{ioy1}"));
                        }
                        "DCM" => {
                            let naming = if row < grid.row_reg_bot(grid.reg_cfg) {
                                "DCM_BOT"
                            } else {
                                "DCM"
                            };
                            let name = format!("{naming}_X{x}Y{y}");
                            for tile in 0..4 {
                                namer.name_intf(
                                    col,
                                    row + tile,
                                    &format!("INTF.DCM.{tile}"),
                                    &name,
                                );
                            }
                            let nnode = namer.ngrid.name_node(nloc, naming, [name]);
                            let dx = dcm_grid.xlut[col];
                            let dy = dcm_grid.ylut[row];
                            nnode.add_bel(0, format!("DCM_ADV_X{dx}Y{dy}"));
                        }
                        "CCM" => {
                            let name = format!("CCM_X{x}Y{y}");
                            for tile in 0..4 {
                                namer.name_intf(
                                    col,
                                    row + tile,
                                    &format!("INTF.CCM.{tile}"),
                                    &name,
                                );
                            }
                            let nnode = namer.ngrid.name_node(nloc, "CCM", [name]);
                            let cx = ccm_grid.xlut[col];
                            let cy = ccm_grid.ylut[row];
                            nnode.add_bel(0, format!("PMCD_X{cx}Y{y}", y = cy * 2));
                            nnode.add_bel(1, format!("PMCD_X{cx}Y{y}", y = cy * 2 + 1));
                            nnode.add_bel(2, format!("DPM_X{cx}Y{cy}"));
                        }
                        "SYSMON" => {
                            let name = format!("SYS_MON_X{x}Y{y}");
                            for tile in 0..8 {
                                namer.name_intf(
                                    col,
                                    row + tile,
                                    &format!("INTF.SYSMON.{tile}"),
                                    &name,
                                );
                            }
                            let nnode = namer.ngrid.name_node(nloc, "SYSMON", [name]);
                            let sx = sysmon_grid.xlut[col];
                            let sy = sysmon_grid.ylut[row];
                            let ipx = if edev.col_lgt.is_some() { 1 } else { 0 };
                            let ipy0 = if row.to_idx() == 0 {
                                0
                            } else if edev.col_lgt.is_some() {
                                grid.regs * 3
                            } else {
                                2
                            };
                            let ipy1 = ipy0 + 1;
                            nnode.add_bel(0, format!("MONITOR_X{sx}Y{sy}"));
                            nnode.add_bel(1, format!("IPAD_X{ipx}Y{ipy0}"));
                            nnode.add_bel(2, format!("IPAD_X{ipx}Y{ipy1}"));
                        }
                        "CFG" => {
                            let name = format!("CFG_CENTER_X{x}Y{y}", y = y + 7);
                            for tile in 0..16 {
                                namer.name_intf(
                                    col,
                                    row + tile,
                                    &format!("INTF.CFG.{tile}"),
                                    &name,
                                );
                            }
                            let name_bufg_b = format!("CLK_BUFGCTRL_B_X{x}Y{y}");
                            let name_bufg_t = format!("CLK_BUFGCTRL_T_X{x}Y{y}", y = y + 8);
                            let name_hrow_b = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                            let name_hrow_t = format!("CLK_HROW_X{x}Y{y}", y = y + 15);
                            let name_hclk_b = format!("HCLK_CENTER_X{x}Y{y}", y = y - 1);
                            let name_hclk_t = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y + 15);
                            let nnode = namer.ngrid.name_node(
                                nloc,
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
                        }
                        "PPC" => {
                            let name_b = format!("PB_X{x}Y{y}", y = y + 3);
                            let name_t = format!("PT_X{x}Y{y}", y = y + 19);
                            for dy in 0..24 {
                                let name = if dy < 12 { &name_b } else { &name_t };
                                namer.name_intf(col, row + dy, &format!("INTF.PPC.L{dy}"), name);
                                namer.name_intf(
                                    col + 8,
                                    row + dy,
                                    &format!("INTF.PPC.R{dy}"),
                                    name,
                                );
                            }
                            for dy in 0..22 {
                                let name = if dy < 11 { &name_b } else { &name_t };
                                namer.ngrid.name_term_tile(
                                    (die.die, col, row + 1 + dy, Dir::E),
                                    &format!("TERM.PPC.E{dy}"),
                                    name.into(),
                                );
                                namer.ngrid.name_term_tile(
                                    (die.die, col + 8, row + 1 + dy, Dir::W),
                                    &format!("TERM.PPC.W{dy}"),
                                    name.into(),
                                );
                            }
                            for dx in 0..7 {
                                namer.name_intf(
                                    col + dx + 1,
                                    row,
                                    &format!("INTF.PPC.B{dx}"),
                                    &name_b,
                                );
                                namer.name_intf(
                                    col + dx + 1,
                                    row + 23,
                                    &format!("INTF.PPC.T{dx}"),
                                    &name_t,
                                );
                                namer.ngrid.name_term_pair(
                                    (die.die, col + dx + 1, row, Dir::N),
                                    &format!("TERM.PPC.N{dx}"),
                                    name_b.clone(),
                                    name_t.clone(),
                                );
                                namer.ngrid.name_term_pair(
                                    (die.die, col + dx + 1, row + 23, Dir::S),
                                    &format!("TERM.PPC.S{dx}"),
                                    name_t.clone(),
                                    name_b.clone(),
                                );
                            }
                            let nnode = namer.ngrid.name_node(nloc, "PPC", [name_b, name_t]);
                            let px = ppc_grid.xlut[col];
                            let py = ppc_grid.ylut[row];
                            nnode.add_bel(0, format!("PPC405_ADV_X{px}Y{py}"));
                            nnode.add_bel(1, format!("EMAC_X{px}Y{py}"));
                        }
                        "MGT" => {
                            let lr = if Some(col) == edev.col_lgt { 'L' } else { 'R' };
                            let name0 = format!("MGT_B{lr}_X{x}Y{y}", y = y + 8);
                            let name1 = format!("MGT_A{lr}_X{x}Y{y}", y = y + 24);
                            let name_clk = format!("BRKH_MGT11CLK_{lr}_X{x}Y{y}", y = y + 15);
                            let naming = if lr == 'L' { "MGT.L" } else { "MGT.R" };
                            for (br, name) in [(row, &name0), (row + 16, &name1)] {
                                for tile in 0..16 {
                                    namer.name_intf(
                                        col,
                                        br + tile,
                                        &format!("INTF.MGT.{tile}"),
                                        name,
                                    );
                                    if lr == 'L' {
                                        namer.ngrid.name_term_tile(
                                            (die.die, col, br + tile, Dir::W),
                                            &format!("TERM.W.MGT{tile}"),
                                            name.into(),
                                        );
                                    } else {
                                        namer.ngrid.name_term_tile(
                                            (die.die, col, br + tile, Dir::E),
                                            &format!("TERM.E.MGT{tile}"),
                                            name.into(),
                                        );
                                    }
                                }
                            }
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, naming, [name0, name1, name_clk]);
                            let gtx = mgt_grid.xlut[col];
                            let gty = mgt_grid.ylut[row];
                            let has_bot_sysmon = grid
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
                            nnode.add_bel(0, format!("GT11_X{gtx}Y{gty}", gty = gty * 2));
                            nnode.add_bel(1, format!("GT11_X{gtx}Y{gty}", gty = gty * 2 + 1));
                            nnode.add_bel(2, format!("IPAD_X{ipx}Y{ipy}"));
                            nnode.add_bel(3, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1));
                            nnode.add_bel(4, format!("OPAD_X{gtx}Y{opy}", opy = gty * 4));
                            nnode.add_bel(5, format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 1));
                            nnode.add_bel(6, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 4));
                            nnode.add_bel(7, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 5));
                            nnode.add_bel(8, format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 2));
                            nnode.add_bel(9, format!("OPAD_X{gtx}Y{opy}", opy = gty * 4 + 3));
                            nnode.add_bel(10, format!("GT11CLK_X{gtx}Y{gty}"));
                            nnode.add_bel(11, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 3));
                            nnode.add_bel(12, format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 2));
                        }

                        "HCLK_IOIS_DCI" | "HCLK_IOIS_LVDS" => {
                            let l = if col.to_idx() == 0 { "_L" } else { "" };
                            let name = format!("{kind}_X{x}Y{y}", y = y - 1);
                            let name_io0 = format!("IOIS_NC{l}_X{x}Y{y}", y = y - 2);
                            let name_io1 = format!("IOIS_LC{l}_X{x}Y{y}", y = y - 1);
                            let name_io2 = format!("IOIS_LC{l}_X{x}Y{y}");
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [name, name_io0, name_io1, name_io2],
                            );
                            let iox = io_grid.xlut[col];
                            let reg = grid.row_to_reg(row).to_idx();
                            let brx = if Some(col) == edev.col_lio { 0 } else { 1 };
                            nnode.add_bel(0, format!("BUFR_X{brx}Y{y}", y = reg * 2 + 1));
                            nnode.add_bel(1, format!("BUFR_X{brx}Y{y}", y = reg * 2));
                            nnode.add_bel(2, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                            nnode.add_bel(3, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                            nnode.add_bel(4, format!("IDELAYCTRL_X{iox}Y{reg}"));
                            if kind == "HCLK_IOIS_DCI" {
                                let dciy = dci_grid.ylut[row];
                                nnode.add_bel(5, format!("DCI_X{iox}Y{dciy}"));
                            }
                        }
                        "HCLK_CENTER" => {
                            let name = format!("HCLK_CENTER_X{x}Y{y}", y = y - 1);
                            let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                            let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, kind, [name, name_io0, name_io1]);
                            let iox = io_grid.xlut[col];
                            let dciy = dci_grid.ylut[row];
                            let reg = grid.row_to_reg(row).to_idx();
                            nnode.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                            nnode.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                            nnode.add_bel(2, format!("IDELAYCTRL_X{iox}Y{reg}"));
                            nnode.add_bel(3, format!("DCI_X{iox}Y{dciy}"));
                        }
                        "HCLK_CENTER_ABOVE_CFG" => {
                            let name = format!("HCLK_CENTER_ABOVE_CFG_X{x}Y{y}", y = y - 1);
                            let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                            let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                            let nnode =
                                namer
                                    .ngrid
                                    .name_node(nloc, kind, [name, name_io0, name_io1]);
                            let iox = io_grid.xlut[col];
                            let dciy = dci_grid.ylut[row];
                            let reg = grid.row_to_reg(row).to_idx();
                            nnode.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                            nnode.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                            nnode.add_bel(2, format!("IDELAYCTRL_X{iox}Y{reg}"));
                            nnode.add_bel(3, format!("DCI_X{iox}Y{dciy}"));
                        }
                        "HCLK_DCMIOB" => {
                            let name = format!("HCLK_DCMIOB_X{x}Y{y}", y = y - 1);
                            let name_io0 = format!("IOIS_LC_X{x}Y{y}");
                            let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y + 1);
                            let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [name, name_io0, name_io1, name_hrow],
                            );
                            let iox = io_grid.xlut[col];
                            let dciy = dci_grid.ylut[row];
                            let reg = grid.row_to_reg(row).to_idx();
                            nnode.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                            nnode.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                            nnode.add_bel(2, format!("IDELAYCTRL_X{iox}Y{reg}"));
                            nnode.add_bel(3, format!("DCI_X{iox}Y{dciy}"));
                        }
                        "HCLK_IOBDCM" => {
                            let name = format!("HCLK_IOBDCM_X{x}Y{y}", y = y - 1);
                            let name_io0 = format!("IOIS_LC_X{x}Y{y}", y = y - 2);
                            let name_io1 = format!("IOIS_LC_X{x}Y{y}", y = y - 1);
                            let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                            let nnode = namer.ngrid.name_node(
                                nloc,
                                kind,
                                [name, name_io0, name_io1, name_hrow],
                            );
                            let iox = io_grid.xlut[col];
                            let dciy = dci_grid.ylut[row];
                            let reg = grid.row_to_reg(row).to_idx();
                            nnode.add_bel(0, format!("BUFIO_X{iox}Y{y}", y = reg * 2 + 1));
                            nnode.add_bel(1, format!("BUFIO_X{iox}Y{y}", y = reg * 2));
                            nnode.add_bel(2, format!("IDELAYCTRL_X{iox}Y{reg}"));
                            nnode.add_bel(3, format!("DCI_X{iox}Y{dciy}"));
                        }
                        "HCLK_DCM" => {
                            let name = format!("HCLK_DCM_X{x}Y{y}", y = y - 1);
                            let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                            namer.ngrid.name_node(nloc, "HCLK_DCM", [name, name_hrow]);
                        }

                        "CLK_HROW" => {
                            let name_hrow = format!("CLK_HROW_X{x}Y{y}", y = y - 1);
                            namer.ngrid.name_node(nloc, "CLK_HROW", [name_hrow]);
                        }
                        "CLK_DCM_B" | "CLK_DCM_T" => {
                            let bt = if row < grid.row_reg_bot(grid.reg_cfg) {
                                'B'
                            } else {
                                'T'
                            };
                            let name = format!("CLKV_DCM_{bt}_X{x}Y{y}");
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        "CLK_IOB_B" | "CLK_IOB_T" => {
                            let bt = if row < grid.row_reg_bot(grid.reg_cfg) {
                                'B'
                            } else {
                                'T'
                            };
                            let name = format!("CLK_IOB_{bt}_X{x}Y{y}", y = y + 7);
                            namer.ngrid.name_node(nloc, kind, [name]);
                        }
                        _ => unreachable!(),
                    }
                }
                for (dir, term) in &die[(col, row)].terms {
                    let Some(term) = term else { continue };
                    let tloc = (die.die, col, row, dir);
                    let kind = egrid.db.terms.key(term.kind);
                    let x = col.to_idx();
                    let y = row.to_idx();

                    match &kind[..] {
                        "TERM.W" => {
                            if edev.col_lgt.is_none() {
                                let name = format!("L_TERM_INT_X{x}Y{y}");
                                namer.ngrid.name_term_tile(tloc, "TERM.W", name);
                            }
                        }
                        "TERM.E" => {
                            if edev.col_rgt.is_none() {
                                let name = format!("R_TERM_INT_X{x}Y{y}");
                                namer.ngrid.name_term_tile(tloc, "TERM.E", name);
                            }
                        }
                        "TERM.S" => {
                            let name = format!("B_TERM_INT_X{x}Y{y}");
                            namer.ngrid.name_term_tile(tloc, "TERM.S", name);
                        }
                        "TERM.N" => {
                            let name = format!("T_TERM_INT_X{x}Y{y}");
                            namer.ngrid.name_term_tile(tloc, "TERM.N", name);
                        }
                        "CLB_BUFFER.W" => {
                            let name = format!("CLB_BUFFER_X{x}Y{y}", x = x - 1);
                            namer.ngrid.name_term_tile(tloc, "PASS.CLB_BUFFER.E", name);
                        }
                        "CLB_BUFFER.E" => {
                            let name = format!("CLB_BUFFER_X{x}Y{y}");
                            namer.ngrid.name_term_tile(tloc, "PASS.CLB_BUFFER.W", name);
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
