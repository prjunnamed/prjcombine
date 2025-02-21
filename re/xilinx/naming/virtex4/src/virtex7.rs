use enum_map::EnumMap;
use prjcombine_interconnect::{db::Dir, grid::RowId};
use prjcombine_re_xilinx_naming::{
    db::NamingDb,
    grid::{BelMultiGrid, ExpandedGridNaming},
};
use prjcombine_virtex4::{
    chip::{ColumnKind, GtKind, Pcie2Kind, XadcIoLoc},
    expanded::ExpandedDevice,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::{ExpandedNamedDevice, ExpandedNamedGtz};

fn make_int_tie_grid(
    edev: &ExpandedDevice,
    ngrid: &ExpandedGridNaming,
) -> (BelMultiGrid, BelMultiGrid) {
    let mut int_grid = ngrid.bel_multi_grid(|_, node, _| node == "INT");
    let mut tie_grid = int_grid.clone();
    if edev.interposer.unwrap().gtz_bot {
        for die in int_grid.ylut.values_mut() {
            for y in die.values_mut() {
                *y += 1;
            }
        }
    }
    let mut tiexlut = EntityPartVec::new();
    let pchip = edev.chips[edev.interposer.unwrap().primary];
    let mut tiex = 0;
    for col in int_grid.xlut.ids() {
        if pchip.columns[col] == ColumnKind::Dsp && col.to_idx() % 2 == 0 {
            tiex += 1;
        }
        tiexlut.insert(col, tiex);
        tiex += 1;
        if pchip.columns[col] == ColumnKind::Dsp && col.to_idx() % 2 == 1 {
            tiex += 1;
        }
    }
    tie_grid.xlut = tiexlut;
    (int_grid, tie_grid)
}

fn make_raw_grid(edev: &ExpandedDevice) -> BelMultiGrid {
    let pchip = edev.chips[edev.interposer.unwrap().primary];
    let mut xlut = EntityPartVec::new();
    let mut rx = 0;
    for (col, &kind) in &pchip.columns {
        if pchip.has_ps && pchip.regs == 2 && col.to_idx() < 6 {
            continue;
        }
        if pchip.cols_vbrk.contains(&col) && rx != 0 {
            rx += 1;
        }
        if kind == ColumnKind::Bram && col.to_idx() == 0 {
            rx += 1;
        }
        xlut.insert(col, rx);
        match kind {
            ColumnKind::ClbLL | ColumnKind::ClbLM => rx += 2,
            ColumnKind::Bram | ColumnKind::Dsp | ColumnKind::Clk | ColumnKind::Cfg => rx += 3,
            ColumnKind::Io => {
                if col == pchip.columns.first_id().unwrap()
                    || col == pchip.columns.last_id().unwrap()
                {
                    rx += 5;
                } else {
                    rx += 4;
                }
            }
            ColumnKind::Gt | ColumnKind::Cmt => rx += 4,
        }
    }

    let mut ylut: EntityVec<_, _> = edev.egrid.die.ids().map(|_| EntityPartVec::new()).collect();
    let mut ry = 0;
    if edev.interposer.unwrap().gtz_bot {
        ry += 2;
    }
    for (die, dylut) in &mut ylut {
        for row in edev.egrid.die(die).rows() {
            if row.to_idx() % 25 == 0 {
                ry += 1;
            }
            dylut.insert(row, ry);
            ry += 1;
        }
        ry += 1
    }

    BelMultiGrid { xlut, ylut }
}

fn make_ipad_grid(edev: &ExpandedDevice) -> BelMultiGrid {
    let pchip = edev.chips[edev.interposer.unwrap().primary];
    let mut is_7k70t = false;
    if let Some(rgt) = edev.col_rgt {
        let gtcol = pchip.get_col_gt(rgt).unwrap();
        if rgt == pchip.columns.last_id().unwrap() - 6
            && gtcol.regs.values().any(|&y| y == Some(GtKind::Gtx))
            && pchip.regs == 4
            && !pchip.has_ps
        {
            is_7k70t = true;
        }
    }

    let mut xlut = EntityPartVec::new();
    let mut ipx = 0;
    for (col, &kind) in &pchip.columns {
        for gtcol in pchip.cols_gt.iter() {
            if gtcol.col == col {
                xlut.insert(col, ipx);
                ipx += 1;
            }
        }
        if kind == ColumnKind::Cfg && pchip.regs > 1 {
            xlut.insert(col, ipx);
            if !is_7k70t {
                ipx += 1;
            }
        }
        if kind == ColumnKind::Clk
            && (edev.interposer.unwrap().gtz_bot || edev.interposer.unwrap().gtz_top)
        {
            xlut.insert(col, ipx);
            ipx += 1;
        }
    }

    let mut ylut: EntityVec<_, _> = edev.egrid.die.ids().map(|_| EntityPartVec::new()).collect();
    let mut ipy = 0;
    if edev.interposer.unwrap().gtz_bot {
        ipy += 6;
    }
    for (die, dylut) in &mut ylut {
        let chip = edev.chips[die];
        for row in edev.egrid.die[die].rows() {
            if matches!(row.to_idx() % 50, 0 | 11 | 22 | 28 | 39) {
                let reg = chip.row_to_reg(row);
                let mut has_gt = false;
                for gtcol in chip.cols_gt.iter() {
                    if gtcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if has_gt {
                    dylut.insert(row, ipy);
                    ipy += 6;
                }
            }
            if !is_7k70t && row == chip.row_reg_hclk(chip.reg_cfg) {
                dylut.insert(row, ipy);
                ipy += 6;
            }
        }
        if is_7k70t {
            dylut.insert(chip.row_reg_hclk(chip.reg_cfg), ipy + 6);
        }
    }

    BelMultiGrid { xlut, ylut }
}

fn make_opad_grid(edev: &ExpandedDevice) -> BelMultiGrid {
    let pchip = edev.chips[edev.interposer.unwrap().primary];

    let mut xlut = EntityPartVec::new();
    let mut opx = 0;
    for (col, &kind) in &pchip.columns {
        for gtcol in pchip.cols_gt.iter() {
            if gtcol.col == col {
                xlut.insert(col, opx);
                opx += 1;
            }
        }
        if kind == ColumnKind::Clk
            && (edev.interposer.unwrap().gtz_bot || edev.interposer.unwrap().gtz_top)
        {
            xlut.insert(col, opx);
            opx += 1;
        }
    }

    let mut ylut: EntityVec<_, _> = edev.egrid.die.ids().map(|_| EntityPartVec::new()).collect();
    let mut opy = 0;
    if edev.interposer.unwrap().gtz_bot {
        opy += 2;
    }
    for (die, dylut) in &mut ylut {
        let chip = edev.chips[die];
        for row in edev.egrid.die[die].rows() {
            let reg = chip.row_to_reg(row);
            if matches!(row.to_idx() % 50, 0 | 11 | 28 | 39) {
                let mut has_gt = false;
                for gtcol in chip.cols_gt.iter() {
                    if gtcol.regs[reg].is_some() {
                        has_gt = true;
                    }
                }
                if has_gt {
                    dylut.insert(row, opy);
                    opy += 2;
                }
            }
        }
    }

    BelMultiGrid { xlut, ylut }
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let interposer = edev.interposer.unwrap();
    let egrid = &edev.egrid;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);
    ngrid.tie_kind = Some("TIEOFF".to_string());
    ngrid.tie_pin_gnd = Some("HARD0".to_string());
    ngrid.tie_pin_vcc = Some("HARD1".to_string());

    let (int_grid, tie_grid) = make_int_tie_grid(edev, &ngrid);
    let raw_grid = make_raw_grid(edev);
    let clb_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "CLBLL" | "CLBLM"));
    let bram_grid = ngrid.bel_multi_grid(|_, node, _| node == "BRAM");
    let dsp_grid = ngrid.bel_multi_grid(|_, node, _| node == "DSP");
    let cmt_grid = ngrid.bel_multi_grid(|_, node, _| node == "CMT");
    let fifo_grid = ngrid.bel_multi_grid(|_, node, _| node == "CMT_FIFO");
    let pcie_grid = ngrid.bel_multi_grid(|_, node, _| node == "PCIE");
    let pcie3_grid = ngrid.bel_multi_grid(|_, node, _| node == "PCIE3");
    let bufg_grid = ngrid.bel_multi_grid(|_, node, _| node == "CLK_BUFG");
    let pmvbram_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "PMVBRAM" | "PMVBRAM_NC"));
    let io_grid = BelMultiGrid {
        ylut: tie_grid.ylut.clone(),
        ..ngrid.bel_multi_grid(|_, node, _| matches!(node, "IO_HP_BOT" | "IO_HR_BOT"))
    };
    let dci_grid = ngrid.bel_multi_grid(|_, node, _| node == "HCLK_IOI_HP");
    let ipad_grid = make_ipad_grid(edev);
    let opad_grid = make_opad_grid(edev);
    let gt_grid = ngrid.bel_multi_grid(|_, node, _| {
        matches!(
            node,
            "GTP_CHANNEL" | "GTP_CHANNEL_MID" | "GTX_CHANNEL" | "GTH_CHANNEL"
        )
    });
    let gtc_grid = ngrid.bel_multi_grid(|_, node, _| {
        matches!(
            node,
            "GTP_COMMON" | "GTP_COMMON_MID" | "GTX_COMMON" | "GTH_COMMON"
        )
    });
    let mut pmviob_grid = ngrid.bel_multi_grid(|_, node, _| matches!(node, "CFG" | "CLK_PMVIOB"));
    for (die, ylut) in &mut pmviob_grid.ylut {
        let chip = edev.chips[die];
        if chip.reg_cfg == chip.reg_clk {
            for val in ylut.values_mut() {
                *val ^= 1;
            }
        }
    }
    let mut gtz = EnumMap::default();
    for (dir, egt) in &edev.gtz {
        let Some(egt) = egt else { continue };
        let gtzx = 0;
        let (gtzy, ipy, opy) = if dir == Dir::N && edev.gtz[Dir::S].is_some() {
            (1, 20, 16)
        } else {
            (0, 0, 0)
        };
        gtz[dir] = Some(ExpandedNamedGtz {
            int_tiles: egt.cols.map_values(|&col| {
                let lr = if col.to_idx() % 2 == 0 { 'L' } else { 'R' };
                let x = int_grid.xlut[col];
                let y = if dir == Dir::S {
                    int_grid.ylut[egt.die][RowId::from_idx(0)] - 1
                } else {
                    int_grid.ylut[egt.die][RowId::from_idx(edev.chips[egt.die].regs * 50 - 1)] + 1
                };
                let tkn = if dir == Dir::S {
                    format!("GTZ_INT_{lr}B")
                } else {
                    format!("GTZ_INT_{lr}")
                };
                format!("{tkn}_X{x}Y{y}")
            }),
            clk_tile: {
                let tkn = if dir == Dir::S {
                    "GTZ_CLK_B"
                } else {
                    "GTZ_CLK"
                };
                let x = raw_grid.xlut[edev.col_clk] + 2;
                let y = if dir == Dir::S {
                    0
                } else {
                    raw_grid.ylut[egt.die][RowId::from_idx(edev.chips[egt.die].regs * 50 - 1)] + 3
                };
                format!("{tkn}_X{x}Y{y}")
            },
            tile: {
                let tkn = if dir == Dir::S { "GTZ_BOT" } else { "GTZ_TOP" };
                let x = raw_grid.xlut[*edev.gtz[Dir::N].as_ref().unwrap().cols.last().unwrap() + 1];
                let y = if dir == Dir::S {
                    0
                } else {
                    raw_grid.ylut[egt.die][RowId::from_idx(edev.chips[egt.die].regs * 50 - 1)] + 3
                };
                format!("{tkn}_X{x}Y{y}")
            },
            bel: format!("GTZE2_OCTAL_X{gtzx}Y{gtzy}"),
            pads_clk: (0..2)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 1 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 2 * i),
                    )
                })
                .collect(),
            pads_rx: (0..8)
                .map(|i| {
                    (
                        format!("IPAD_X2Y{}", ipy + 5 + 2 * i),
                        format!("IPAD_X2Y{}", ipy + 4 + 2 * i),
                    )
                })
                .collect(),
            pads_tx: (0..8)
                .map(|i| {
                    (
                        format!("OPAD_X1Y{}", opy + 1 + 2 * i),
                        format!("OPAD_X1Y{}", opy + 2 * i),
                    )
                })
                .collect(),
        });
    }
    for die in egrid.dies() {
        let chip = edev.chips[die.die];
        let has_slr_d = die.die != edev.chips.first_id().unwrap();
        let has_slr_u = die.die != edev.chips.last_id().unwrap();
        let has_gtz_d =
            die.die == edev.chips.first_id().unwrap() && edev.interposer.unwrap().gtz_bot;
        let has_gtz_u =
            die.die == edev.chips.last_id().unwrap() && edev.interposer.unwrap().gtz_top;
        for col in die.cols() {
            for row in die.rows() {
                let reg = chip.row_to_reg(row);
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    let x = int_grid.xlut[col];
                    let y = int_grid.ylut[die.die][row];
                    let int_lr = if col.to_idx() % 2 == 0 { 'L' } else { 'R' };
                    match &kind[..] {
                        "INT" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                &format!("INT.{int_lr}"),
                                [format!("INT_{int_lr}_X{x}Y{y}")],
                            );
                            let tie_x = tie_grid.xlut[col];
                            let tie_y = tie_grid.ylut[die.die][row];
                            nnode.tie_name = Some(format!("TIEOFF_X{tie_x}Y{tie_y}"));
                        }
                        "INTF" => match chip.columns[col] {
                            ColumnKind::ClbLL => {
                                ngrid.name_node(
                                    nloc,
                                    "INTF.PSS",
                                    [format!("INT_INTERFACE_PSS_{int_lr}_X{x}Y{y}")],
                                );
                            }
                            ColumnKind::Io => {
                                ngrid.name_node(
                                    nloc,
                                    &format!("INTF.{int_lr}"),
                                    [format!("IO_INT_INTERFACE_{int_lr}_X{x}Y{y}")],
                                );
                            }
                            ColumnKind::Dsp
                            | ColumnKind::Cfg
                            | ColumnKind::Cmt
                            | ColumnKind::Clk => {
                                ngrid.name_node(
                                    nloc,
                                    &format!("INTF.{int_lr}"),
                                    [format!("INT_INTERFACE_{int_lr}_X{x}Y{y}")],
                                );
                            }
                            _ => unreachable!(),
                        },
                        "INTF.BRAM" => {
                            ngrid.name_node(
                                nloc,
                                &format!("INTF.{int_lr}"),
                                [format!("BRAM_INT_INTERFACE_{int_lr}_X{x}Y{y}")],
                            );
                        }
                        "INTF.DELAY" => 'intf: {
                            for gtcol in &chip.cols_gt {
                                if gtcol.col != col {
                                    continue;
                                }
                                if let Some(kind) = gtcol.regs[reg] {
                                    if gtcol.is_middle {
                                        if col < edev.col_clk {
                                            ngrid.name_node(
                                                nloc,
                                                "INTF.GTP_R",
                                                [format!("GTP_INT_INTERFACE_R_X{x}Y{y}")],
                                            );
                                        } else {
                                            ngrid.name_node(
                                                nloc,
                                                "INTF.GTP_L",
                                                [format!("GTP_INT_INTERFACE_L_X{x}Y{y}")],
                                            );
                                        }
                                    } else {
                                        let gkind = match kind {
                                            GtKind::Gtp => "GTP",
                                            GtKind::Gtx => "GTX",
                                            GtKind::Gth => "GTH",
                                        };
                                        if col.to_idx() == 0 {
                                            ngrid.name_node(
                                                nloc,
                                                &format!("INTF.{gkind}_L"),
                                                [format!("{gkind}_INT_INTERFACE_L_X{x}Y{y}")],
                                            );
                                        } else {
                                            ngrid.name_node(
                                                nloc,
                                                &format!("INTF.{gkind}"),
                                                [format!("{gkind}_INT_INTERFACE_X{x}Y{y}")],
                                            );
                                        }
                                    }
                                    break 'intf;
                                }
                            }
                            for pcie2 in &chip.holes_pcie2 {
                                if row < pcie2.row || row > pcie2.row + 25 {
                                    continue;
                                }
                                if col == pcie2.col {
                                    ngrid.name_node(
                                        nloc,
                                        "INTF.PCIE_R",
                                        [format!("PCIE_INT_INTERFACE_R_X{x}Y{y}")],
                                    );
                                    break 'intf;
                                } else if col == pcie2.col + 3 {
                                    match pcie2.kind {
                                        Pcie2Kind::Left => {
                                            ngrid.name_node(
                                                nloc,
                                                "INTF.PCIE_LEFT_L",
                                                [format!("PCIE_INT_INTERFACE_LEFT_L_X{x}Y{y}")],
                                            );
                                        }
                                        Pcie2Kind::Right => {
                                            ngrid.name_node(
                                                nloc,
                                                "INTF.PCIE_L",
                                                [format!("PCIE_INT_INTERFACE_L_X{x}Y{y}")],
                                            );
                                        }
                                    }
                                    break 'intf;
                                }
                            }
                            for &(pcol, prow) in &chip.holes_pcie3 {
                                if row < prow || row > prow + 50 {
                                    continue;
                                }
                                if col == pcol {
                                    ngrid.name_node(
                                        nloc,
                                        "INTF.PCIE3_R",
                                        [format!("PCIE3_INT_INTERFACE_R_X{x}Y{y}")],
                                    );
                                    break 'intf;
                                } else if col == pcol + 5 {
                                    ngrid.name_node(
                                        nloc,
                                        "INTF.PCIE3_L",
                                        [format!("PCIE3_INT_INTERFACE_L_X{x}Y{y}")],
                                    );
                                    break 'intf;
                                }
                            }
                            panic!("wtf is this interface");
                        }
                        "HCLK" => {
                            let mut suf = "";
                            if chip.has_slr && !(col >= edev.col_cfg - 6 && col < edev.col_cfg) {
                                if row.to_idx() < 50 {
                                    if has_slr_d {
                                        suf = "_SLV";
                                    }
                                    if has_gtz_d && col.to_idx() < 162 {
                                        suf = "_SLV";
                                    }
                                }
                                if row.to_idx() >= chip.regs * 50 - 50 {
                                    if has_slr_u {
                                        suf = "_SLV";
                                    }
                                    if has_gtz_u && col.to_idx() < 162 {
                                        suf = "_SLV";
                                    }
                                }
                            }
                            let rx = raw_grid.xlut[col + 1] - 1;
                            let ry = raw_grid.ylut[die.die][row] - 1;
                            let hole_bot = edev.in_int_hole(die.die, col, row - 1);
                            let hole_top = edev.in_int_hole(die.die, col, row);
                            if hole_bot {
                                suf = "_BOT_UTURN";
                            }
                            if hole_top {
                                suf = "_TOP_UTURN";
                            }
                            ngrid.name_node(
                                nloc,
                                "HCLK",
                                [
                                    format!("HCLK_L{suf}_X{rx}Y{ry}"),
                                    format!("HCLK_R{suf}_X{rx}Y{ry}", rx = rx + 1),
                                ],
                            );
                        }
                        "INT_LCLK" => {
                            ngrid.name_node(
                                nloc,
                                "INT_LCLK",
                                [
                                    format!("INT_L_X{x}Y{y}"),
                                    format!("INT_R_X{x}Y{y}", x = x + 1),
                                ],
                            );
                        }
                        "CLBLL" | "CLBLM" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                &format!("{kind}_{int_lr}"),
                                [format!("{kind}_{int_lr}_X{x}Y{y}")],
                            );
                            let sx0 = clb_grid.xlut[col] * 2;
                            let sx1 = clb_grid.xlut[col] * 2 + 1;
                            let sy = clb_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("SLICE_X{sx0}Y{sy}"));
                            nnode.add_bel(1, format!("SLICE_X{sx1}Y{sy}"));
                        }
                        "BRAM" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                &format!("BRAM_{int_lr}"),
                                [format!("BRAM_{int_lr}_X{x}Y{y}")],
                            );
                            let bx = bram_grid.xlut[col];
                            let by = bram_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("RAMB36_X{bx}Y{by}"));
                            nnode.add_bel(1, format!("RAMB18_X{bx}Y{y}", y = by * 2));
                            nnode.add_bel(2, format!("RAMB18_X{bx}Y{y}", y = by * 2 + 1));
                        }
                        "PMVBRAM" => {
                            let hx = if int_lr == 'L' {
                                raw_grid.xlut[col]
                            } else {
                                raw_grid.xlut[col] + 2
                            };
                            let hy = raw_grid.ylut[die.die][row] - 1;
                            let nnode = ngrid.name_node(
                                nloc,
                                "PMVBRAM",
                                [
                                    format!("HCLK_BRAM_X{hx}Y{hy}"),
                                    format!("BRAM_{int_lr}_X{x}Y{y}"),
                                    format!("BRAM_{int_lr}_X{x}Y{y}", y = y + 5),
                                    format!("BRAM_{int_lr}_X{x}Y{y}", y = y + 10),
                                ],
                            );
                            let bx = pmvbram_grid.xlut[col];
                            let by = pmvbram_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PMVBRAM_X{bx}Y{by}"));
                        }
                        "PMVBRAM_NC" => {
                            let hx = if int_lr == 'L' {
                                raw_grid.xlut[col]
                            } else {
                                raw_grid.xlut[col] + 2
                            };
                            let hy = raw_grid.ylut[die.die][row] - 1;
                            let nnode = ngrid.name_node(
                                nloc,
                                "PMVBRAM_NC",
                                [format!("HCLK_BRAM_X{hx}Y{hy}")],
                            );
                            let bx = pmvbram_grid.xlut[col];
                            let by = pmvbram_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PMVBRAM_X{bx}Y{by}"));
                        }
                        "DSP" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                &format!("DSP_{int_lr}"),
                                [format!("DSP_{int_lr}_X{x}Y{y}")],
                            );
                            let dx = dsp_grid.xlut[col];
                            let dy0 = dsp_grid.ylut[die.die][row] * 2;
                            let dy1 = dsp_grid.ylut[die.die][row] * 2 + 1;
                            nnode.add_bel(0, format!("DSP48_X{dx}Y{dy0}"));
                            nnode.add_bel(1, format!("DSP48_X{dx}Y{dy1}"));
                            let tx = if int_lr == 'L' {
                                tie_grid.xlut[col] - 1
                            } else {
                                tie_grid.xlut[col] + 1
                            };
                            let ty = tie_grid.ylut[die.die][row];
                            nnode.add_bel(2, format!("TIEOFF_X{tx}Y{ty}"));
                        }
                        "PCIE" => {
                            let (naming, left, rx) = if col.to_idx() % 2 == 0 {
                                ("PCIE_L", "_LEFT", raw_grid.xlut[col - 3] + 2)
                            } else {
                                ("PCIE_R", "", raw_grid.xlut[col] + 2)
                            };
                            let ry = raw_grid.ylut[die.die][row];
                            let nnode = ngrid.name_node(
                                nloc,
                                naming,
                                [
                                    format!("PCIE_BOT{left}_X{rx}Y{ry}", ry = ry + 10),
                                    format!("PCIE_TOP{left}_X{rx}Y{ry}", ry = ry + 20),
                                ],
                            );
                            let bx = pcie_grid.xlut[col];
                            let by = pcie_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PCIE_X{bx}Y{by}"));
                        }
                        "PCIE3" => {
                            let rx = raw_grid.xlut[col] + 2;
                            let ry = raw_grid.ylut[die.die][row];
                            let nnode = ngrid.name_node(
                                nloc,
                                "PCIE3",
                                [
                                    format!("PCIE3_RIGHT_X{rx}Y{ry}", ry = ry + 26),
                                    format!("PCIE3_BOT_RIGHT_X{rx}Y{ry}", ry = ry + 7),
                                    format!("PCIE3_TOP_RIGHT_X{rx}Y{ry}", ry = ry + 43),
                                ],
                            );
                            let bx = pcie3_grid.xlut[col];
                            let by = pcie3_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PCIE3_X{bx}Y{by}"));
                        }
                        "IO_HP_BOT" | "IO_HP_TOP" | "IO_HP_PAIR" | "IO_HR_BOT" | "IO_HR_TOP"
                        | "IO_HR_PAIR" => {
                            let is_term = col == chip.columns.first_id().unwrap()
                                || col == chip.columns.last_id().unwrap();
                            let is_l = col < edev.col_clk;
                            let is_single = !kind.ends_with("_PAIR");
                            let is_hp = kind.starts_with("IO_HP");
                            let (tk, iob_tk) = if is_hp {
                                if is_l {
                                    ("LIOI", "LIOB18")
                                } else {
                                    ("RIOI", "RIOB18")
                                }
                            } else {
                                if is_l {
                                    ("LIOI3", "LIOB33")
                                } else {
                                    ("RIOI3", "RIOB33")
                                }
                            };
                            let rx = raw_grid.xlut[col]
                                + if is_l {
                                    1
                                } else if is_term {
                                    3
                                } else {
                                    2
                                };
                            let rxiob = if is_l { rx - 1 } else { rx + 1 };
                            let ry = raw_grid.ylut[die.die][row];
                            let (ioi_tk, iob_tk) = if is_single {
                                (format!("{tk}_SING"), format!("{iob_tk}_SING"))
                            } else {
                                let suf = match row.to_idx() % 50 {
                                    7 | 19 | 31 | 43 => "_TBYTESRC",
                                    13 | 37 => "_TBYTETERM",
                                    _ => "",
                                };
                                (format!("{tk}{suf}"), iob_tk.to_string())
                            };
                            let (name, name_iob) = if is_term {
                                (format!("{ioi_tk}_X{x}Y{y}"), format!("{iob_tk}_X{x}Y{y}"))
                            } else {
                                (
                                    format!("{ioi_tk}_X{rx}Y{ry}"),
                                    format!("{iob_tk}_X{rxiob}Y{ry}"),
                                )
                            };
                            let nnode = ngrid.name_node(nloc, &ioi_tk, [name, name_iob]);
                            let iox = io_grid.xlut[col];
                            let ioy0 = io_grid.ylut[die.die][row];
                            let ioy1 = io_grid.ylut[die.die][row] + 1;
                            if !is_single {
                                if is_hp {
                                    nnode.add_bel(0, format!("ILOGIC_X{iox}Y{ioy0}"));
                                    nnode.add_bel(1, format!("ILOGIC_X{iox}Y{ioy1}"));
                                    nnode.add_bel(2, format!("OLOGIC_X{iox}Y{ioy0}"));
                                    nnode.add_bel(3, format!("OLOGIC_X{iox}Y{ioy1}"));
                                    nnode.add_bel(4, format!("IDELAY_X{iox}Y{ioy0}"));
                                    nnode.add_bel(5, format!("IDELAY_X{iox}Y{ioy1}"));
                                    nnode.add_bel(6, format!("ODELAY_X{iox}Y{ioy0}"));
                                    nnode.add_bel(7, format!("ODELAY_X{iox}Y{ioy1}"));
                                    nnode.add_bel(8, format!("IOB_X{iox}Y{ioy0}"));
                                    nnode.add_bel(9, format!("IOB_X{iox}Y{ioy1}"));
                                } else {
                                    nnode.add_bel(0, format!("ILOGIC_X{iox}Y{ioy0}"));
                                    nnode.add_bel(1, format!("ILOGIC_X{iox}Y{ioy1}"));
                                    nnode.add_bel(2, format!("OLOGIC_X{iox}Y{ioy0}"));
                                    nnode.add_bel(3, format!("OLOGIC_X{iox}Y{ioy1}"));
                                    nnode.add_bel(4, format!("IDELAY_X{iox}Y{ioy0}"));
                                    nnode.add_bel(5, format!("IDELAY_X{iox}Y{ioy1}"));
                                    nnode.add_bel(6, format!("IOB_X{iox}Y{ioy0}"));
                                    nnode.add_bel(7, format!("IOB_X{iox}Y{ioy1}"));
                                }
                            } else {
                                if is_hp {
                                    nnode.add_bel(0, format!("ILOGIC_X{iox}Y{ioy0}"));
                                    nnode.add_bel(1, format!("OLOGIC_X{iox}Y{ioy0}"));
                                    nnode.add_bel(2, format!("IDELAY_X{iox}Y{ioy0}"));
                                    nnode.add_bel(3, format!("ODELAY_X{iox}Y{ioy0}"));
                                    nnode.add_bel(4, format!("IOB_X{iox}Y{ioy0}"));
                                } else {
                                    nnode.add_bel(0, format!("ILOGIC_X{iox}Y{ioy0}"));
                                    nnode.add_bel(1, format!("OLOGIC_X{iox}Y{ioy0}"));
                                    nnode.add_bel(2, format!("IDELAY_X{iox}Y{ioy0}"));
                                    nnode.add_bel(3, format!("IOB_X{iox}Y{ioy0}"));
                                }
                            }
                        }
                        "HCLK_IOI_HP" | "HCLK_IOI_HR" => {
                            let is_term = col == chip.columns.first_id().unwrap()
                                || col == chip.columns.last_id().unwrap();
                            let is_l = col < edev.col_clk;
                            let is_hp = kind == "HCLK_IOI_HP";
                            let tk = if is_hp {
                                if is_l { "LIOI" } else { "RIOI" }
                            } else {
                                if is_l { "LIOI3" } else { "RIOI3" }
                            };
                            let htk = if is_hp { "HCLK_IOI" } else { "HCLK_IOI3" };
                            let rx = raw_grid.xlut[col]
                                + if is_l {
                                    1
                                } else if is_term {
                                    3
                                } else {
                                    2
                                };
                            let ry = raw_grid.ylut[die.die][row];
                            let (name_b0, name_b1, name_t0, name_t1) = if is_term {
                                (
                                    format!("{tk}_X{x}Y{y}", y = y - 4),
                                    format!("{tk}_X{x}Y{y}", y = y - 2),
                                    format!("{tk}_X{x}Y{y}"),
                                    format!("{tk}_X{x}Y{y}", y = y + 2),
                                )
                            } else {
                                (
                                    format!("{tk}_X{rx}Y{ry}", ry = ry - 5),
                                    format!("{tk}_X{rx}Y{ry}", ry = ry - 3),
                                    format!("{tk}_X{rx}Y{ry}"),
                                    format!("{tk}_X{rx}Y{ry}", ry = ry + 2),
                                )
                            };

                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    format!("{htk}_X{rx}Y{ry}", ry = ry - 1),
                                    name_b0,
                                    name_b1,
                                    name_t0,
                                    name_t1,
                                ],
                            );
                            let iox = io_grid.xlut[col];
                            let hy = io_grid.ylut[die.die][row] / 50;
                            for i in 0..4 {
                                nnode.add_bel(i, format!("BUFIO_X{iox}Y{y}", y = hy * 4 + (i ^ 2)));
                            }
                            for i in 0..4 {
                                nnode.add_bel(
                                    i + 4,
                                    format!("BUFR_X{iox}Y{y}", y = hy * 4 + (i ^ 2)),
                                );
                            }
                            nnode.add_bel(8, format!("IDELAYCTRL_X{iox}Y{hy}"));
                            if is_hp {
                                let dcix = dci_grid.xlut[col];
                                let dciy = dci_grid.ylut[die.die][row];
                                nnode.add_bel(9, format!("DCI_X{dcix}Y{dciy}"));
                            }
                        }
                        "CMT_FIFO" => {
                            let is_l = col.to_idx() % 2 == 0;
                            let naming = if is_l { "CMT_FIFO_L" } else { "CMT_FIFO_R" };
                            let rx = if is_l {
                                raw_grid.xlut[col] + 1
                            } else {
                                raw_grid.xlut[col] + 2
                            };
                            let ry = raw_grid.ylut[die.die][row];
                            let nnode = ngrid.name_node(
                                nloc,
                                naming,
                                [format!("{naming}_X{rx}Y{ry}", ry = ry + 6)],
                            );
                            let fx = fifo_grid.xlut[col];
                            let fy = fifo_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("IN_FIFO_X{fx}Y{fy}"));
                            nnode.add_bel(1, format!("OUT_FIFO_X{fx}Y{fy}"));
                        }
                        "CMT" => {
                            let is_l = col.to_idx() % 2 == 0;
                            let naming = if is_l { "CMT.L" } else { "CMT.R" };
                            let rx = if is_l {
                                raw_grid.xlut[col]
                            } else {
                                raw_grid.xlut[col] + 3
                            };
                            let nnode = ngrid.name_node(
                                nloc,
                                naming,
                                [
                                    format!(
                                        "CMT_TOP_{int_lr}_LOWER_B_X{rx}Y{y}",
                                        y = raw_grid.ylut[die.die][row - 17]
                                    ),
                                    format!(
                                        "CMT_TOP_{int_lr}_LOWER_T_X{rx}Y{y}",
                                        y = raw_grid.ylut[die.die][row - 8]
                                    ),
                                    format!(
                                        "CMT_TOP_{int_lr}_UPPER_B_X{rx}Y{y}",
                                        y = raw_grid.ylut[die.die][row + 4]
                                    ),
                                    format!(
                                        "CMT_TOP_{int_lr}_UPPER_T_X{rx}Y{y}",
                                        y = raw_grid.ylut[die.die][row + 17]
                                    ),
                                    if is_l {
                                        format!(
                                            "HCLK_CMT_L_X{rx}Y{y}",
                                            y = raw_grid.ylut[die.die][row] - 1
                                        )
                                    } else {
                                        format!(
                                            "HCLK_CMT_X{rx}Y{y}",
                                            y = raw_grid.ylut[die.die][row] - 1
                                        )
                                    },
                                ],
                            );
                            let cx = cmt_grid.xlut[col];
                            let cy = cmt_grid.ylut[die.die][row];
                            for i in 0..4 {
                                nnode
                                    .add_bel(i, format!("PHASER_IN_PHY_X{cx}Y{y}", y = cy * 4 + i));
                            }
                            for i in 0..4 {
                                nnode.add_bel(
                                    4 + i,
                                    format!("PHASER_OUT_PHY_X{cx}Y{y}", y = cy * 4 + i),
                                );
                            }
                            nnode.add_bel(8, format!("PHASER_REF_X{cx}Y{cy}"));
                            nnode.add_bel(9, format!("PHY_CONTROL_X{cx}Y{cy}"));
                            nnode.add_bel(10, format!("MMCME2_ADV_X{cx}Y{cy}"));
                            nnode.add_bel(11, format!("PLLE2_ADV_X{cx}Y{cy}"));
                            for i in 0..2 {
                                nnode.add_bel(12 + i, format!("BUFMRCE_X{cx}Y{y}", y = cy * 2 + i));
                            }
                        }
                        "CLK_BUFG_REBUF" => {
                            let ctb_y = tie_grid.ylut[die.die][row] / 50 * 48
                                + if row.to_idx() % 50 < 25 { 0 } else { 32 };
                            let name = format!(
                                "CLK_BUFG_REBUF_X{x}Y{y}",
                                x = raw_grid.xlut[col] + 2,
                                y = raw_grid.ylut[die.die][row],
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            for i in 0..16 {
                                nnode.add_bel(i, format!("GCLK_TEST_BUF_X0Y{y}", y = ctb_y + i));
                            }
                            for i in 0..16 {
                                nnode.add_bel(
                                    16 + i,
                                    format!("GCLK_TEST_BUF_X1Y{y}", y = ctb_y + i),
                                );
                            }
                        }
                        "CLK_BALI_REBUF" => {
                            let tk = if reg.to_idx() == 0 && has_gtz_d {
                                "CLK_BALI_REBUF_GTZ_BOT"
                            } else if reg.to_idx() != 0 && has_gtz_u {
                                "CLK_BALI_REBUF_GTZ_TOP"
                            } else {
                                "CLK_BALI_REBUF"
                            };
                            let name = format!(
                                "{tk}_X{rx}Y{ry}",
                                rx = raw_grid.xlut[col] + 2,
                                ry = raw_grid.ylut[die.die][row + 8],
                            );
                            let nnode = ngrid.name_node(nloc, kind, [name]);
                            let ctb_y = tie_grid.ylut[die.die][row] / 50 * 48
                                + if row.to_idx() % 50 < 25 { 0 } else { 32 };
                            let bglb_y = if edev.interposer.unwrap().gtz_bot && reg.to_idx() != 0 {
                                16
                            } else {
                                0
                            };
                            for i in 0..16 {
                                let y = (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3;
                                if has_gtz_u && reg.to_idx() != 0 {
                                    nnode.add_bel(i, format!("BUFG_LB_X1Y{y}", y = bglb_y + y));
                                } else {
                                    nnode
                                        .add_bel(i, format!("GCLK_TEST_BUF_X1Y{y}", y = ctb_y + y));
                                }
                                if has_gtz_d && reg.to_idx() == 0 {
                                    nnode
                                        .add_bel(16 + i, format!("BUFG_LB_X3Y{y}", y = bglb_y + y));
                                } else {
                                    nnode.add_bel(
                                        16 + i,
                                        format!("GCLK_TEST_BUF_X3Y{y}", y = ctb_y + y),
                                    );
                                }
                            }
                        }
                        "CLK_HROW" => {
                            let ctb_y = tie_grid.ylut[die.die][row] / 50 * 48;
                            let bufh_y = tie_grid.ylut[die.die][row] / 50 * 12;
                            let naming = if reg < chip.reg_clk {
                                "CLK_HROW_BOT_R"
                            } else {
                                "CLK_HROW_TOP_R"
                            };
                            let name = format!(
                                "{naming}_X{x}Y{y}",
                                x = raw_grid.xlut[col] + 2,
                                y = raw_grid.ylut[die.die][row] - 1,
                            );
                            let nnode = ngrid.name_node(nloc, naming, [name]);
                            for i in 0..32 {
                                nnode.add_bel(
                                    i,
                                    format!(
                                        "GCLK_TEST_BUF_X{x}Y{y}",
                                        x = i >> 4,
                                        y = ctb_y + 16 + (i & 0xf ^ 0xf)
                                    ),
                                );
                            }
                            for i in 0..12 {
                                nnode.add_bel(32 + i, format!("BUFHCE_X0Y{y}", y = bufh_y + i));
                            }
                            for i in 0..12 {
                                nnode.add_bel(44 + i, format!("BUFHCE_X1Y{y}", y = bufh_y + i));
                            }
                            nnode.add_bel(56, format!("GCLK_TEST_BUF_X3Y{y}", y = ctb_y + 17));
                            nnode.add_bel(57, format!("GCLK_TEST_BUF_X3Y{y}", y = ctb_y + 16));
                        }
                        "CLK_BUFG" => {
                            let naming = if reg < chip.reg_clk {
                                "CLK_BUFG_BOT_R"
                            } else {
                                "CLK_BUFG_TOP_R"
                            };
                            let name = format!(
                                "{naming}_X{x}Y{y}",
                                x = raw_grid.xlut[col] + 2,
                                y = raw_grid.ylut[die.die][row]
                            );
                            let nnode = ngrid.name_node(nloc, naming, [name]);
                            let bg_y = bufg_grid.ylut[die.die][row] * 16;
                            for i in 0..16 {
                                nnode.add_bel(i, format!("BUFGCTRL_X0Y{y}", y = bg_y + i));
                            }
                        }
                        "CLK_PMV" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [format!(
                                    "{kind}_X{rx}Y{ry}",
                                    rx = raw_grid.xlut[col] + 2,
                                    ry = raw_grid.ylut[die.die][row - 3]
                                )],
                            );
                            nnode.add_bel(0, format!("PMV_X0Y{y}", y = die.die.to_idx() * 3));
                        }
                        "CLK_PMVIOB" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [format!(
                                    "{kind}_X{rx}Y{ry}",
                                    rx = raw_grid.xlut[col] + 2,
                                    ry = raw_grid.ylut[die.die][row]
                                )],
                            );
                            let pmvx = pmviob_grid.xlut[col];
                            let pmvy = pmviob_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("PMVIOB_X{pmvx}Y{pmvy}"));
                        }
                        "CLK_PMV2_SVT" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [format!(
                                    "{kind}_X{rx}Y{ry}",
                                    rx = raw_grid.xlut[col] + 2,
                                    ry = raw_grid.ylut[die.die][row]
                                )],
                            );
                            nnode.add_bel(0, format!("PMV_X0Y{y}", y = die.die.to_idx() * 3 + 1));
                        }
                        "CLK_PMV2" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [format!(
                                    "{kind}_X{rx}Y{ry}",
                                    rx = raw_grid.xlut[col] + 2,
                                    ry = raw_grid.ylut[die.die][row]
                                )],
                            );
                            nnode.add_bel(0, format!("PMV_X0Y{y}", y = die.die.to_idx() * 3 + 2));
                        }
                        "CLK_MTBF2" => {
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [format!(
                                    "{kind}_X{rx}Y{ry}",
                                    rx = raw_grid.xlut[col] + 2,
                                    ry = raw_grid.ylut[die.die][row]
                                )],
                            );
                            nnode.add_bel(0, format!("MTBF2_X0Y{y}", y = die.die.to_idx()));
                        }
                        "CFG" => {
                            let slv = if die.die == interposer.primary {
                                ""
                            } else {
                                "_SLAVE"
                            };
                            let rx = raw_grid.xlut[col] - 1;
                            let nnode = ngrid.name_node(
                                nloc,
                                kind,
                                [
                                    format!(
                                        "CFG_CENTER_BOT_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row + 10]
                                    ),
                                    format!(
                                        "CFG_CENTER_MID{slv}_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row + 30]
                                    ),
                                    format!(
                                        "CFG_CENTER_TOP{slv}_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row + 40]
                                    ),
                                ],
                            );
                            let di = die.die.to_idx();
                            let pmvx = pmviob_grid.xlut[col];
                            let pmvy = pmviob_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("BSCAN_X0Y{y}", y = di * 4));
                            nnode.add_bel(1, format!("BSCAN_X0Y{y}", y = di * 4 + 1));
                            nnode.add_bel(2, format!("BSCAN_X0Y{y}", y = di * 4 + 2));
                            nnode.add_bel(3, format!("BSCAN_X0Y{y}", y = di * 4 + 3));
                            nnode.add_bel(4, format!("ICAP_X0Y{y}", y = di * 2));
                            nnode.add_bel(5, format!("ICAP_X0Y{y}", y = di * 2 + 1));
                            nnode.add_bel(6, format!("STARTUP_X0Y{di}"));
                            nnode.add_bel(7, format!("CAPTURE_X0Y{di}"));
                            nnode.add_bel(8, format!("FRAME_ECC_X0Y{di}"));
                            nnode.add_bel(9, format!("USR_ACCESS_X0Y{di}"));
                            nnode.add_bel(10, format!("CFG_IO_ACCESS_X0Y{di}"));
                            nnode.add_bel(11, format!("PMVIOB_X{pmvx}Y{pmvy}"));
                            nnode.add_bel(12, format!("DCIRESET_X0Y{di}"));
                            nnode.add_bel(13, format!("DNA_PORT_X0Y{di}"));
                            nnode.add_bel(14, format!("EFUSE_USR_X0Y{di}"));
                        }
                        "XADC" => {
                            let io_loc = chip.get_xadc_io_loc();
                            let naming = match io_loc {
                                XadcIoLoc::Right => "XADC.R",
                                XadcIoLoc::Left => "XADC.L",
                                XadcIoLoc::Both => "XADC.LR",
                            };
                            let suf = match io_loc {
                                XadcIoLoc::Left => "_FUJI2",
                                XadcIoLoc::Right => "_PELE1",
                                XadcIoLoc::Both => "",
                            };
                            let slv = if die.die == interposer.primary {
                                ""
                            } else {
                                "_SLAVE"
                            };
                            let rx = raw_grid.xlut[col] - 1;
                            let mut names = vec![
                                format!(
                                    "MONITOR_BOT{suf}{slv}_X{rx}Y{ry}",
                                    ry = raw_grid.ylut[die.die][row]
                                ),
                                format!(
                                    "MONITOR_MID{suf}_X{rx}Y{ry}",
                                    ry = raw_grid.ylut[die.die][row + 10]
                                ),
                                format!(
                                    "MONITOR_TOP{suf}_X{rx}Y{ry}",
                                    ry = raw_grid.ylut[die.die][row + 20]
                                ),
                            ];
                            if io_loc == XadcIoLoc::Right {
                                names.extend([
                                    format!(
                                        "CFG_SECURITY_BOT_PELE1_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row - 25]
                                    ),
                                    format!(
                                        "CFG_SECURITY_MID_PELE1_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row - 25 + 10]
                                    ),
                                    format!(
                                        "CFG_SECURITY_TOP_PELE1_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row - 25 + 20]
                                    ),
                                ]);
                            }
                            let nnode = ngrid.name_node(nloc, naming, names);
                            let ipx = ipad_grid.xlut[col];
                            let ipy0 = ipad_grid.ylut[die.die][row];
                            let ipy1 = ipy0 + 1;
                            nnode.add_bel(0, format!("IPAD_X{ipx}Y{ipy0}",));
                            nnode.add_bel(1, format!("IPAD_X{ipx}Y{ipy1}",));
                            nnode.add_bel(2, format!("XADC_X0Y{di}", di = die.die.to_idx()));
                        }
                        "PS" => {
                            let rx = raw_grid.xlut[col] - 18;
                            let nnode = ngrid.name_node(
                                nloc,
                                "PS",
                                [
                                    format!(
                                        "PSS0_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row - 40]
                                    ),
                                    format!(
                                        "PSS1_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row - 20]
                                    ),
                                    format!("PSS2_X{rx}Y{ry}", ry = raw_grid.ylut[die.die][row]),
                                    format!(
                                        "PSS3_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row + 20]
                                    ),
                                    format!(
                                        "PSS4_X{rx}Y{ry}",
                                        ry = raw_grid.ylut[die.die][row + 40]
                                    ),
                                ],
                            );
                            nnode.add_bel(0, "PS7_X0Y0".to_string());
                            for i in 1..73 {
                                nnode.add_bel(i, format!("IOPAD_X1Y{i}"));
                            }
                            for i in 77..135 {
                                nnode.add_bel(i - 4, format!("IOPAD_X1Y{i}"));
                            }
                        }
                        "GTP_CHANNEL" | "GTP_CHANNEL_MID" | "GTX_CHANNEL" | "GTH_CHANNEL" => {
                            let gtcol = chip.cols_gt.iter().find(|gtcol| gtcol.col == col).unwrap();
                            let idx = match row.to_idx() % 50 {
                                0 => 0,
                                11 => 1,
                                28 => 2,
                                39 => 3,
                                _ => unreachable!(),
                            };
                            let gkind = match gtcol.regs[reg].unwrap() {
                                GtKind::Gtp => "GTP",
                                GtKind::Gtx => "GTX",
                                GtKind::Gth => "GTH",
                            };
                            let rx = if gtcol.is_middle {
                                if col < edev.col_clk {
                                    raw_grid.xlut[col] + 14
                                } else {
                                    raw_grid.xlut[col] - 18
                                }
                            } else {
                                if col < edev.col_clk {
                                    raw_grid.xlut[col]
                                } else {
                                    raw_grid.xlut[col] + 4
                                }
                            };

                            let naming = if gtcol.is_middle {
                                if col < edev.col_clk {
                                    format!("{gkind}_CHANNEL_{idx}_MID_LEFT")
                                } else {
                                    format!("{gkind}_CHANNEL_{idx}_MID_RIGHT")
                                }
                            } else {
                                format!("{gkind}_CHANNEL_{idx}")
                            };
                            let ry = raw_grid.ylut[die.die][row + 5];
                            let nnode =
                                ngrid.name_node(nloc, &naming, [format!("{naming}_X{rx}Y{ry}")]);
                            let gtx = gt_grid.xlut[col];
                            let gty = gt_grid.ylut[die.die][row];
                            let ipx = ipad_grid.xlut[col];
                            let ipy = ipad_grid.ylut[die.die][row];
                            let opx = opad_grid.xlut[col];
                            let opy = opad_grid.ylut[die.die][row];
                            nnode.add_bel(0, format!("{gkind}E2_CHANNEL_X{gtx}Y{gty}"));
                            nnode.add_bel(1, format!("IPAD_X{ipx}Y{y}", y = ipy + 1));
                            nnode.add_bel(2, format!("IPAD_X{ipx}Y{ipy}"));
                            nnode.add_bel(3, format!("OPAD_X{opx}Y{y}", y = opy + 1));
                            nnode.add_bel(4, format!("OPAD_X{opx}Y{opy}"));
                        }
                        "GTP_COMMON" | "GTP_COMMON_MID" | "GTX_COMMON" | "GTH_COMMON" => {
                            let gtcol = chip.cols_gt.iter().find(|gtcol| gtcol.col == col).unwrap();
                            let gkind = match gtcol.regs[reg].unwrap() {
                                GtKind::Gtp => "GTP",
                                GtKind::Gtx => "GTX",
                                GtKind::Gth => "GTH",
                            };
                            let rx = if gtcol.is_middle {
                                if col < edev.col_clk {
                                    raw_grid.xlut[col] + 14
                                } else {
                                    raw_grid.xlut[col] - 18
                                }
                            } else {
                                if col < edev.col_clk {
                                    raw_grid.xlut[col]
                                } else {
                                    raw_grid.xlut[col] + 4
                                }
                            };

                            let naming = if gtcol.is_middle {
                                if col < edev.col_clk {
                                    format!("{kind}_LEFT")
                                } else {
                                    format!("{kind}_RIGHT")
                                }
                            } else {
                                kind.to_string()
                            };
                            let ry = raw_grid.ylut[die.die][row - 3];
                            let nnode =
                                ngrid.name_node(nloc, &naming, [format!("{naming}_X{rx}Y{ry}")]);
                            let gtx = gtc_grid.xlut[col];
                            let gty = gtc_grid.ylut[die.die][row];
                            let ipx = ipad_grid.xlut[col];
                            let ipy = ipad_grid.ylut[die.die][row - 3];
                            nnode.add_bel(0, format!("{gkind}E2_COMMON_X{gtx}Y{gty}"));
                            nnode.add_bel(1, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2));
                            nnode.add_bel(2, format!("IBUFDS_GTE2_X{gtx}Y{y}", y = gty * 2 + 1));
                            nnode.add_bel(3, format!("IPAD_X{ipx}Y{y}", y = ipy - 4));
                            nnode.add_bel(4, format!("IPAD_X{ipx}Y{y}", y = ipy - 3));
                            nnode.add_bel(5, format!("IPAD_X{ipx}Y{y}", y = ipy - 2));
                            nnode.add_bel(6, format!("IPAD_X{ipx}Y{y}", y = ipy - 1));
                        }
                        "BRKH_GTX" => {
                            let gtcol = chip.cols_gt.iter().find(|gtcol| gtcol.col == col).unwrap();
                            ngrid.name_node(
                                nloc,
                                kind,
                                [if gtcol.regs[reg - 1].is_none() {
                                    format!("BRKH_GTX_X{x}Y{y}", x = x + 1, y = y - 1)
                                } else {
                                    format!(
                                        "BRKH_GTX_X{rx}Y{ry}",
                                        rx = raw_grid.xlut[gtcol.col]
                                            + if col.to_idx() == 0 { 0 } else { 4 },
                                        ry = raw_grid.ylut[die.die][row] - 1
                                    )
                                }],
                            );
                        }
                        _ => panic!("how to {kind}"),
                    }
                }
                for (dir, term) in &die[(col, row)].terms {
                    let Some(term) = term else { continue };
                    let tloc = (die.die, col, row, dir);
                    let kind = egrid.db.terms.key(term.kind);
                    let x = int_grid.xlut[col];
                    let y = int_grid.ylut[die.die][row];

                    match &kind[..] {
                        "BRKH.S" => {
                            let name = format!("BRKH_INT_X{x}Y{y}", y = y - 1);
                            ngrid.name_term_tile(tloc, "BRKH.N", name);
                        }
                        "BRKH.N" => {
                            let name = format!("BRKH_INT_X{x}Y{y}");
                            ngrid.name_term_tile(tloc, "BRKH.S", name);
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    ExpandedNamedDevice { edev, ngrid, gtz }
}
