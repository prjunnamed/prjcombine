use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_virtex4::{
    chip::{ColumnKind, GtKind},
    defs,
    defs::virtex5::{ccls, tcls},
    expanded::ExpandedDevice,
};

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
    let mut ngrid = ExpandedGridNaming::new(ndb, edev);

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
                ntile.tie_name = Some(format!("TIEOFF_X{x}Y{y}"));
            }
            tcls::INTF => {
                namer
                    .ngrid
                    .name_tile(tcrd, "INTF", [format!("INT_INTERFACE_X{x}Y{y}")]);
            }
            tcls::INTF_DELAY => {
                if chip.columns[col] == ColumnKind::Gt {
                    if col.to_idx() == 0 {
                        namer.ngrid.name_tile(
                            tcrd,
                            "INTF_GTX_LEFT",
                            [format!("GTX_LEFT_INT_INTERFACE_X{x}Y{y}")],
                        );
                    } else {
                        namer.ngrid.name_tile(
                            tcrd,
                            "INTF_GTP",
                            [format!("GTP_INT_INTERFACE_X{x}Y{y}")],
                        );
                    }
                } else {
                    'intf: {
                        if let Some(ref hard) = chip.col_hard
                            && hard.col == col
                        {
                            for &hrow in &hard.rows_emac {
                                if row >= hrow && row < hrow + 10 {
                                    namer.ngrid.name_tile(
                                        tcrd,
                                        "INTF_EMAC",
                                        [format!("EMAC_INT_INTERFACE_X{x}Y{y}")],
                                    );
                                    break 'intf;
                                }
                            }
                            for &hrow in &hard.rows_pcie {
                                if row >= hrow && row < hrow + 40 {
                                    namer.ngrid.name_tile(
                                        tcrd,
                                        "INTF_PCIE",
                                        [format!("PCIE_INT_INTERFACE_X{x}Y{y}")],
                                    );
                                    break 'intf;
                                }
                            }
                        }
                        for &(pcol, prow) in &chip.holes_ppc {
                            if row >= prow && row < prow + 40 {
                                if col == pcol {
                                    namer.ngrid.name_tile(
                                        tcrd,
                                        "INTF_PPC_L",
                                        [format!("PPC_L_INT_INTERFACE_X{x}Y{y}")],
                                    );
                                    break 'intf;
                                } else if col == pcol + 13 {
                                    namer.ngrid.name_tile(
                                        tcrd,
                                        "INTF_PPC_R",
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
            tcls::HCLK => {
                let reg = chip.row_to_reg(row);
                let kind = match chip.columns[col] {
                    ColumnKind::Gt => {
                        let gtc = chip.cols_gt.iter().find(|gtc| gtc.col == col).unwrap();
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
                let ntile =
                    namer
                        .ngrid
                        .name_tile(tcrd, "HCLK", [format!("{kind}_X{x}Y{y}", y = y - 1)]);
                ntile.add_bel(
                    defs::bslots::GLOBALSIG,
                    format!("GLOBALSIG_X{x}Y{y}", y = y / 20),
                );
            }
            tcls::CLBLL | tcls::CLBLM => {
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [format!("{kind}_X{x}Y{y}")]);
                let sx0 = clb_grid.xlut[col] * 2;
                let sx1 = clb_grid.xlut[col] * 2 + 1;
                let sy = clb_grid.ylut[row];
                ntile.add_bel(defs::bslots::SLICE[0], format!("SLICE_X{sx0}Y{sy}"));
                ntile.add_bel(defs::bslots::SLICE[1], format!("SLICE_X{sx1}Y{sy}"));
            }
            tcls::BRAM => {
                let mut tk = "BRAM";
                if let Some(ref hard) = chip.col_hard
                    && hard.col == col
                {
                    tk = "PCIE_BRAM";
                }
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [format!("{tk}_X{x}Y{y}")]);
                let bx = bram_grid.xlut[col];
                let by = bram_grid.ylut[row];
                ntile.add_bel(defs::bslots::BRAM, format!("RAMB36_X{bx}Y{by}"));
            }
            tcls::DSP => {
                let ntile = namer.ngrid.name_tile(tcrd, kind, [format!("DSP_X{x}Y{y}")]);
                let dx = dsp_grid.xlut[col];
                let dy0 = dsp_grid.ylut[row] * 2;
                let dy1 = dsp_grid.ylut[row] * 2 + 1;
                ntile.add_bel(defs::bslots::DSP[0], format!("DSP48_X{dx}Y{dy0}"));
                ntile.add_bel(defs::bslots::DSP[1], format!("DSP48_X{dx}Y{dy1}"));
            }
            tcls::IO => {
                let iox = io_grid.xlut[col];
                let ioy0 = io_grid.ylut[row] * 2;
                let ioy1 = io_grid.ylut[row] * 2 + 1;
                let naming = match iox {
                    0 => {
                        if col.to_idx() == 0 {
                            "LIOB"
                        } else if row >= chip.row_bufg() + 10 && row < chip.row_bufg() + 20 {
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
                let ntile = namer.ngrid.name_tile(tcrd, naming, [name_ioi, name_iob]);
                ntile.add_bel(defs::bslots::ILOGIC[0], format!("ILOGIC_X{iox}Y{ioy0}"));
                ntile.add_bel(defs::bslots::ILOGIC[1], format!("ILOGIC_X{iox}Y{ioy1}"));
                ntile.add_bel(defs::bslots::OLOGIC[0], format!("OLOGIC_X{iox}Y{ioy0}"));
                ntile.add_bel(defs::bslots::OLOGIC[1], format!("OLOGIC_X{iox}Y{ioy1}"));
                ntile.add_bel(defs::bslots::IODELAY[0], format!("IODELAY_X{iox}Y{ioy0}"));
                ntile.add_bel(defs::bslots::IODELAY[1], format!("IODELAY_X{iox}Y{ioy1}"));
                ntile.add_bel(defs::bslots::IOB[0], format!("IOB_X{iox}Y{ioy0}"));
                ntile.add_bel(defs::bslots::IOB[1], format!("IOB_X{iox}Y{ioy1}"));
            }
            tcls::CMT => {
                let naming = if row.to_idx().is_multiple_of(20) {
                    "CMT_BOT"
                } else {
                    "CMT_TOP"
                };
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, naming, [format!("CMT_X{x}Y{y}")]);
                let bx = cmt_grid.xlut[col];
                let by = cmt_grid.ylut[row];
                ntile.add_bel(
                    defs::bslots::DCM[0],
                    format!("DCM_ADV_X{bx}Y{y}", y = by * 2),
                );
                ntile.add_bel(
                    defs::bslots::DCM[1],
                    format!("DCM_ADV_X{bx}Y{y}", y = by * 2 + 1),
                );
                ntile.add_bel(defs::bslots::PLL, format!("PLL_ADV_X{bx}Y{by}"));
            }
            tcls::EMAC => {
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [format!("EMAC_X{x}Y{y}")]);
                let bx = emac_grid.xlut[col];
                let by = emac_grid.ylut[row];
                ntile.add_bel(defs::bslots::EMAC, format!("TEMAC_X{bx}Y{by}"));
            }
            tcls::PCIE => {
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        format!("PCIE_B_X{x}Y{y}", y = y + 10),
                        format!("PCIE_T_X{x}Y{y}", y = y + 30),
                    ],
                );
                let bx = pcie_grid.xlut[col];
                let by = pcie_grid.ylut[row];
                ntile.add_bel(defs::bslots::PCIE, format!("PCIE_X{bx}Y{by}"));
            }
            tcls::PPC => {
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        format!("PPC_B_X36Y{y}", y = row.to_idx() / 10 * 11 + 11),
                        format!("PPC_T_X36Y{y}", y = row.to_idx() / 10 * 11 + 33),
                    ],
                );
                let bx = ppc_grid.xlut[col];
                let by = ppc_grid.ylut[row];
                ntile.add_bel(defs::bslots::PPC, format!("PPC440_X{bx}Y{by}"));
            }
            tcls::GTP | tcls::GTX => {
                let naming = if tile.class == tcls::GTP {
                    "GT3"
                } else if col.to_idx() == 0 {
                    "GTX_LEFT"
                } else {
                    "GTX"
                };
                let slot = if tile.class == tcls::GTP {
                    defs::bslots::GTP_DUAL
                } else {
                    defs::bslots::GTX_DUAL
                };
                let ntile =
                    namer
                        .ngrid
                        .name_tile(tcrd, naming, [format!("{naming}_X{x}Y{y}", y = y + 9)]);
                let gtx = gt_grid.xlut[col];
                let gty = gt_grid.ylut[row];
                let ipx = if col.to_idx() == 0 { 0 } else { gtx + 1 };
                let ipy = if gty < chip.reg_cfg.to_idx() {
                    gty * 6
                } else {
                    gty * 6 + 6
                };
                ntile.add_bel(slot, format!("{kind}_DUAL_X{gtx}Y{gty}"));
                ntile.add_bel(defs::bslots::BUFDS[0], format!("BUFDS_X{gtx}Y{gty}"));
                ntile.add_bel(
                    defs::bslots::CRC64[0],
                    format!("CRC64_X{gtx}Y{y}", y = gty * 2),
                );
                ntile.add_bel(
                    defs::bslots::CRC64[1],
                    format!("CRC64_X{gtx}Y{y}", y = gty * 2 + 1),
                );
                ntile.add_bel(
                    defs::bslots::CRC32[0],
                    format!("CRC32_X{gtx}Y{y}", y = gty * 4),
                );
                ntile.add_bel(
                    defs::bslots::CRC32[1],
                    format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 1),
                );
                ntile.add_bel(
                    defs::bslots::CRC32[2],
                    format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 2),
                );
                ntile.add_bel(
                    defs::bslots::CRC32[3],
                    format!("CRC32_X{gtx}Y{y}", y = gty * 4 + 3),
                );
                ntile.add_bel(
                    defs::bslots::IPAD_RXP[0],
                    format!("IPAD_X{ipx}Y{y}", y = ipy + 1),
                );
                ntile.add_bel(defs::bslots::IPAD_RXN[0], format!("IPAD_X{ipx}Y{ipy}"));
                ntile.add_bel(
                    defs::bslots::IPAD_RXP[1],
                    format!("IPAD_X{ipx}Y{y}", y = ipy + 3),
                );
                ntile.add_bel(
                    defs::bslots::IPAD_RXN[1],
                    format!("IPAD_X{ipx}Y{y}", y = ipy + 2),
                );
                ntile.add_bel(
                    defs::bslots::IPAD_CLKP[0],
                    format!("IPAD_X{ipx}Y{y}", y = ipy + 5),
                );
                ntile.add_bel(
                    defs::bslots::IPAD_CLKN[0],
                    format!("IPAD_X{ipx}Y{y}", y = ipy + 4),
                );
                ntile.add_bel(
                    defs::bslots::OPAD_TXP[0],
                    format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 1),
                );
                ntile.add_bel(
                    defs::bslots::OPAD_TXN[0],
                    format!("OPAD_X{gtx}Y{y}", y = gty * 4),
                );
                ntile.add_bel(
                    defs::bslots::OPAD_TXP[1],
                    format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 3),
                );
                ntile.add_bel(
                    defs::bslots::OPAD_TXN[1],
                    format!("OPAD_X{gtx}Y{y}", y = gty * 4 + 2),
                );
            }
            tcls::CFG => {
                let rx = namer.rxlut[col] + 3;
                let ry = chip.reg_cfg.to_idx() * 22;
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [format!("CFG_CENTER_X{rx}Y{ry}")]);
                let ipx = if edev.col_lgt.is_some() { 1 } else { 0 };
                let ipy = if !chip.cols_gt.is_empty() {
                    chip.reg_cfg.to_idx() * 6
                } else {
                    0
                };
                for i in 0..4 {
                    ntile.add_bel(defs::bslots::BSCAN[i], format!("BSCAN_X0Y{i}"));
                }
                for i in 0..2 {
                    ntile.add_bel(defs::bslots::ICAP[i], format!("ICAP_X0Y{i}"));
                }
                ntile.add_bel(defs::bslots::PMV_CFG[0], "PMV".to_string());
                ntile.add_bel(defs::bslots::STARTUP, "STARTUP".to_string());
                ntile.add_bel(defs::bslots::JTAGPPC, "JTAGPPC".to_string());
                ntile.add_bel(defs::bslots::FRAME_ECC, "FRAME_ECC".to_string());
                ntile.add_bel(defs::bslots::DCIRESET, "DCIRESET".to_string());
                ntile.add_bel(defs::bslots::CAPTURE, "CAPTURE".to_string());
                ntile.add_bel(defs::bslots::USR_ACCESS, "USR_ACCESS_SITE".to_string());
                ntile.add_bel(defs::bslots::KEY_CLEAR, "KEY_CLEAR".to_string());
                ntile.add_bel(defs::bslots::EFUSE_USR, "EFUSE_USR".to_string());
                ntile.add_bel(defs::bslots::SYSMON, "SYSMON_X0Y0".to_string());
                ntile.add_bel(defs::bslots::IPAD_VP, format!("IPAD_X{ipx}Y{ipy}"));
                ntile.add_bel(
                    defs::bslots::IPAD_VN,
                    format!("IPAD_X{ipx}Y{ipy}", ipy = ipy + 1),
                );
            }
            tcls::CLK_BUFG => {
                let rx = namer.rxlut[col] + 3;
                let ry = chip.reg_cfg.to_idx() * 22;
                let ntile = namer.ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        format!("CFG_CENTER_X{rx}Y{ry}"),
                        format!("CLK_BUFGMUX_X{rx}Y{ry}"),
                    ],
                );
                for i in 0..32 {
                    ntile.add_bel(defs::bslots::BUFGCTRL[i], format!("BUFGCTRL_X0Y{i}"));
                }
            }

            tcls::CLK_HROW => {
                let name_hrow = format!("CLK_HROW{mgt}_X{x}Y{y}", y = y - 1);
                namer.ngrid.name_tile(tcrd, kind, [name_hrow]);
            }
            tcls::CLK_CMT_S | tcls::CLK_CMT_N => {
                let naming = if row < chip.row_bufg() {
                    "CLK_CMT_BOT"
                } else {
                    "CLK_CMT_TOP"
                };
                let rx = namer.rxlut[col] + 4;
                let ry = y / 10 * 11 + 1;
                let name = format!("{naming}{mgt}_X{rx}Y{ry}");
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            tcls::CLK_IOB_S | tcls::CLK_IOB_N => {
                let bt = match tile.class {
                    tcls::CLK_IOB_S => 'B',
                    tcls::CLK_IOB_N => 'T',
                    _ => unreachable!(),
                };
                let name = format!("CLK_IOB_{bt}_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            tcls::CLK_MGT_S | tcls::CLK_MGT_N => {
                let naming = if row < chip.row_bufg() {
                    "CLK_MGT_BOT"
                } else {
                    "CLK_MGT_TOP"
                };
                let name = format!("{naming}{mgt}_X{x}Y{y}");
                namer.ngrid.name_tile(tcrd, kind, [name]);
            }
            tcls::HCLK_IO_CFG_S | tcls::HCLK_IO_CMT_S => {
                let name = if tile.class == tcls::HCLK_IO_CMT_S {
                    format!("HCLK_CMT_IOI_X{x}Y{y}", y = y - 1)
                } else {
                    format!("HCLK_IOI_BOTCEN{mgt}_X{x}Y{y}", y = y - 1)
                };
                let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name, name_i0, name_i1]);
                let iox = io_grid.xlut[col];
                let ioy = io_grid.ylut[row];
                let banky = ioy / 20;
                ntile.add_bel(
                    defs::bslots::BUFIO[2],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 1),
                );
                ntile.add_bel(
                    defs::bslots::BUFIO[3],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4),
                );
                ntile.add_bel(
                    defs::bslots::IDELAYCTRL,
                    format!("IDELAYCTRL_X{iox}Y{banky}"),
                );
                ntile.add_bel(defs::bslots::DCI, format!("DCI_X{iox}Y{banky}"));
            }
            tcls::HCLK_IO_CFG_N | tcls::HCLK_IO_CMT_N => {
                let tkn = match tile.class {
                    tcls::HCLK_IO_CFG_N => "HCLK_IOI_TOPCEN",
                    tcls::HCLK_IO_CMT_N => "HCLK_IOI_CMT",
                    _ => unreachable!(),
                };
                let name = format!("{tkn}{mgt}_X{x}Y{y}", y = y - 1);
                let name_i2 = format!("IOI_X{x}Y{y}");
                let name_i3 = format!("IOI_X{x}Y{y}", y = y + 1);
                let ntile = namer.ngrid.name_tile(tcrd, kind, [name, name_i2, name_i3]);
                let iox = io_grid.xlut[col];
                let ioy = io_grid.ylut[row];
                let banky = ioy / 20;
                ntile.add_bel(
                    defs::bslots::BUFIO[0],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 2),
                );
                ntile.add_bel(
                    defs::bslots::BUFIO[1],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 3),
                );
                ntile.add_bel(
                    defs::bslots::IDELAYCTRL,
                    format!("IDELAYCTRL_X{iox}Y{banky}"),
                );
                ntile.add_bel(defs::bslots::DCI, format!("DCI_X{iox}Y{banky}"));
            }
            tcls::HCLK_IO_CENTER => {
                let name = format!("HCLK_IOI_CENTER_X{x}Y{y}", y = y - 1);
                let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                let name_i2 = format!("IOI_X{x}Y{y}");
                let ntile = namer
                    .ngrid
                    .name_tile(tcrd, kind, [name, name_i0, name_i1, name_i2]);
                let iox = io_grid.xlut[col];
                let ioy = io_grid.ylut[row];
                let banky = ioy / 20;
                ntile.add_bel(
                    defs::bslots::BUFIO[0],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 2),
                );
                ntile.add_bel(
                    defs::bslots::BUFIO[1],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 3),
                );
                ntile.add_bel(
                    defs::bslots::BUFIO[2],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 1),
                );
                ntile.add_bel(
                    defs::bslots::BUFIO[3],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4),
                );
                ntile.add_bel(
                    defs::bslots::IDELAYCTRL,
                    format!("IDELAYCTRL_X{iox}Y{banky}"),
                );
                ntile.add_bel(defs::bslots::DCI, format!("DCI_X{iox}Y{banky}"));
            }
            tcls::HCLK_IO => {
                let name = format!("HCLK_IOI_X{x}Y{y}", y = y - 1);
                let name_i0 = format!("IOI_X{x}Y{y}", y = y - 2);
                let name_i1 = format!("IOI_X{x}Y{y}", y = y - 1);
                let name_i2 = format!("IOI_X{x}Y{y}");
                let name_i3 = format!("IOI_X{x}Y{y}", y = y + 1);
                let ntile =
                    namer
                        .ngrid
                        .name_tile(tcrd, kind, [name, name_i0, name_i1, name_i2, name_i3]);
                let iox = io_grid.xlut[col];
                let ioy = io_grid.ylut[row];
                let banky = ioy / 20;
                ntile.add_bel(
                    defs::bslots::BUFIO[0],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 2),
                );
                ntile.add_bel(
                    defs::bslots::BUFIO[1],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 3),
                );
                ntile.add_bel(
                    defs::bslots::BUFIO[2],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4 + 1),
                );
                ntile.add_bel(
                    defs::bslots::BUFIO[3],
                    format!("BUFIO_X{iox}Y{y}", y = banky * 4),
                );
                ntile.add_bel(
                    defs::bslots::BUFR[0],
                    format!("BUFR_X{x}Y{y}", x = iox / 2, y = banky * 2),
                );
                ntile.add_bel(
                    defs::bslots::BUFR[1],
                    format!("BUFR_X{x}Y{y}", x = iox / 2, y = banky * 2 + 1),
                );
                ntile.add_bel(
                    defs::bslots::IDELAYCTRL,
                    format!("IDELAYCTRL_X{iox}Y{banky}"),
                );
                ntile.add_bel(defs::bslots::DCI, format!("DCI_X{iox}Y{banky}"));
            }
            tcls::HCLK_CMT => {
                let bmt = if row + 30 == chip.row_bufg() {
                    "BOT"
                } else if row == chip.row_bufg() + 30 {
                    "TOP"
                } else {
                    "MID"
                };
                let name = format!("HCLK_IOB_CMT_{bmt}{mgt}_X{x}Y{y}", y = y - 1);
                let name_hrow = format!("CLK_HROW{mgt}_X{x}Y{y}", y = y - 1);
                namer.ngrid.name_tile(tcrd, kind, [name, name_hrow]);
            }
            tcls::PMVBRAM => {
                let mut tkn = "BRAM";
                if let Some(ref hard) = chip.col_hard
                    && hard.col == col
                {
                    tkn = "PCIE_BRAM";
                }
                let name = format!("HCLK_{tkn}_X{x}Y{y}", y = y - 1);
                let name_bram = format!("{tkn}_X{x}Y{y}");
                let ntile = namer.ngrid.name_tile(tcrd, "PMVBRAM", [name, name_bram]);
                let px = pmvbram_grid.xlut[col];
                let py = pmvbram_grid.ylut[row];
                ntile.add_bel(defs::bslots::PMVBRAM, format!("PMVBRAM_X{px}Y{py}"));
            }
            tcls::HCLK_MGT_BUF => {
                let l = if col < edev.col_cfg { "_LEFT" } else { "" };
                let name = format!("HCLK_BRAM_MGT{l}_X{x}Y{y}", y = y - 1);
                namer.ngrid.name_tile(tcrd, "HCLK_BRAM_MGT", [name]);
            }

            _ => unreachable!(),
        }
    }

    for (ccrd, conn) in edev.connectors() {
        let cell = ccrd.cell;
        let CellCoord { col, row, .. } = cell;

        let x = col.to_idx();
        let y = row.to_idx();

        match conn.class {
            ccls::TERM_W => {
                let name = if edev.col_lgt.is_some() {
                    format!("GTX_L_TERM_INT_X{x}Y{y}")
                } else {
                    format!("L_TERM_INT_X{x}Y{y}")
                };
                namer.ngrid.name_conn_tile(ccrd, "TERM_W", name);
            }
            ccls::TERM_E => {
                let name = format!("R_TERM_INT_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM_E", name);
            }
            ccls::TERM_S_PPC => {
                let name = format!("PPC_T_TERM_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM_S_PPC", name);
            }
            ccls::TERM_N_PPC => {
                let name = format!("PPC_B_TERM_X{x}Y{y}");
                namer.ngrid.name_conn_tile(ccrd, "TERM_N_PPC", name);
            }
            ccls::PPC_W => {
                // sigh.
                let ry = y / 10 * 11 + y % 10 + 1;
                let name_l = format!("L_TERM_PPC_X{x}Y{y}", x = x - 13);
                let name_r = format!("R_TERM_PPC_X53Y{ry}");
                namer.ngrid.name_conn_pair(ccrd, "PPC_W", name_r, name_l);
            }
            ccls::PPC_E => {
                let ry = y / 10 * 11 + y % 10 + 1;
                let name_l = format!("L_TERM_PPC_X{x}Y{y}");
                let name_r = format!("R_TERM_PPC_X53Y{ry}");
                namer.ngrid.name_conn_pair(ccrd, "PPC_E", name_l, name_r);
            }
            ccls::INT_BUFS_W => {
                let mon = if edev.col_lgt.is_some() { "_MON" } else { "" };
                let name_l = format!("INT_BUFS_L_X{x}Y{y}", x = x - 1);
                let name_r = format!("INT_BUFS_R{mon}_X{x}Y{y}");
                namer
                    .ngrid
                    .name_conn_pair(ccrd, "INT_BUFS_W", name_r, name_l);
            }
            ccls::INT_BUFS_E => {
                let mon = if edev.col_lgt.is_some() { "_MON" } else { "" };
                let name_l = format!("INT_BUFS_L_X{x}Y{y}");
                let name_r = format!("INT_BUFS_R{mon}_X{x}Y{y}", x = x + 1);
                namer
                    .ngrid
                    .name_conn_pair(ccrd, "INT_BUFS_E", name_l, name_r);
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
