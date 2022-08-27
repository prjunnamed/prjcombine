use crate::verify::{BelContext, SitePinDir, Verifier};
use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::virtex2::{Dcms, Edge, Grid, GridKind};
use prjcombine_xilinx_geom::BelId;

pub fn verify_bufgmux(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFGMUX",
        &[("I0", SitePinDir::In), ("I1", SitePinDir::In)],
        &["CLK"],
    );
    vrf.claim_node(&[bel.fwire("I0")]);
    vrf.claim_node(&[bel.fwire("I1")]);
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("CLK"));
    let obid = BelId::from_idx(bel.bid.to_idx() ^ 1);
    let obel = vrf.get_bel(bel.slr, bel.node, obid);
    vrf.claim_pip(bel.crd(), bel.wire("I1"), obel.wire("CLK"));
    let edge = if bel.row == grid.row_bot() {
        Edge::Bot
    } else if bel.row == grid.row_top() {
        Edge::Top
    } else if bel.col == grid.col_left() {
        Edge::Left
    } else if bel.col == grid.col_right() {
        Edge::Right
    } else {
        unreachable!()
    };
    if grid.kind.is_virtex2() || grid.kind == GridKind::Spartan3 {
        if let Some((crd, obid)) = grid.get_clk_io(edge, bel.bid.to_idx()) {
            let onode = grid.get_io_node(vrf.grid, crd).unwrap();
            let obel = vrf.get_bel(bel.slr, onode, obid);
            vrf.claim_node(&[bel.fwire("CKI"), obel.fwire("IBUF")]);
            vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
        } else {
            vrf.claim_node(&[bel.fwire("CKI")]);
        }
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKI"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_L"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_R"));
        vrf.claim_node(&[bel.fwire("DCM_OUT_L")]);
        vrf.claim_node(&[bel.fwire("DCM_OUT_R")]);
        if grid.kind.is_virtex2() {
            for pin in ["DCM_PAD_L", "DCM_PAD_R"] {
                vrf.claim_node(&[bel.fwire(pin)]);
                vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKI"));
            }
        } else {
            vrf.claim_node(&[bel.fwire("DCM_PAD")]);
            vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD"), bel.wire("CKI"));
        }
    } else if matches!(edge, Edge::Bot | Edge::Top) {
        let (crd, obid) = grid.get_clk_io(edge, bel.bid.to_idx()).unwrap();
        let onode = grid.get_io_node(vrf.grid, crd).unwrap();
        let obel = vrf.get_bel(bel.slr, onode, obid);
        vrf.claim_node(&[bel.fwire("CKIR"), obel.fwire("IBUF")]);
        vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
        let (crd, obid) = grid.get_clk_io(edge, bel.bid.to_idx() + 4).unwrap();
        let onode = grid.get_io_node(vrf.grid, crd).unwrap();
        let obel = vrf.get_bel(bel.slr, onode, obid);
        vrf.claim_node(&[bel.fwire("CKIL"), obel.fwire("IBUF")]);
        vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKIL"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKIR"));

        let mut has_dcm_l = true;
        let mut has_dcm_r = true;
        if grid.kind == GridKind::Spartan3E {
            if grid.dcms == Some(Dcms::Two) {
                has_dcm_l = false;
            }
        } else {
            if grid.dcms == Some(Dcms::Two) && bel.row == grid.row_bot() {
                has_dcm_l = false;
                has_dcm_r = false;
            }
        }
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_L"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_R"));
        if has_dcm_l {
            vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD_L"), bel.wire("CKIL"));
            let pip = &bel.naming.pins["DCM_OUT_L"].pips[0];
            vrf.claim_node(&[bel.fwire("DCM_OUT_L"), (bel.crds[pip.tile], &pip.wire_to)]);
            vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
            let dy = match edge {
                Edge::Bot => 1,
                Edge::Top => -1,
                _ => unreachable!(),
            };
            let obel = vrf.find_bel_delta(bel, 0, dy, "DCMCONN.S3E").unwrap();
            let (dcm_pad_pin, dcm_out_pin) = match (edge, bel.bid.to_idx()) {
                (Edge::Top, 0) => ("CLKPAD0", "OUT0"),
                (Edge::Top, 1) => ("CLKPAD1", "OUT1"),
                (Edge::Top, 2) => ("CLKPAD2", "OUT2"),
                (Edge::Top, 3) => ("CLKPAD3", "OUT3"),
                (Edge::Bot, 0) => ("CLKPAD3", "OUT0"),
                (Edge::Bot, 1) => ("CLKPAD2", "OUT1"),
                (Edge::Bot, 2) => ("CLKPAD1", "OUT2"),
                (Edge::Bot, 3) => ("CLKPAD0", "OUT3"),
                _ => unreachable!(),
            };
            vrf.verify_node(&[bel.fwire("DCM_PAD_L"), obel.fwire(dcm_pad_pin)]);
            vrf.verify_node(&[
                (bel.crds[pip.tile], &pip.wire_from),
                obel.fwire(dcm_out_pin),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire("DCM_OUT_L")]);
        }
        if has_dcm_r {
            vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD_R"), bel.wire("CKIR"));
            let pip = &bel.naming.pins["DCM_OUT_R"].pips[0];
            vrf.claim_node(&[bel.fwire("DCM_OUT_R"), (bel.crds[pip.tile], &pip.wire_to)]);
            vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
            let dy = match edge {
                Edge::Bot => 1,
                Edge::Top => -1,
                _ => unreachable!(),
            };
            let obel = vrf.find_bel_delta(bel, 1, dy, "DCMCONN.S3E").unwrap();
            let (dcm_pad_pin, dcm_out_pin) = match (edge, bel.bid.to_idx()) {
                (Edge::Top, 0) => ("CLKPAD2", "OUT0"),
                (Edge::Top, 1) => ("CLKPAD3", "OUT1"),
                (Edge::Top, 2) => ("CLKPAD0", "OUT2"),
                (Edge::Top, 3) => ("CLKPAD1", "OUT3"),
                (Edge::Bot, 0) => ("CLKPAD0", "OUT0"),
                (Edge::Bot, 1) => ("CLKPAD1", "OUT1"),
                (Edge::Bot, 2) => ("CLKPAD2", "OUT2"),
                (Edge::Bot, 3) => ("CLKPAD3", "OUT3"),
                _ => unreachable!(),
            };
            vrf.verify_node(&[bel.fwire("DCM_PAD_R"), obel.fwire(dcm_pad_pin)]);
            vrf.verify_node(&[
                (bel.crds[pip.tile], &pip.wire_from),
                obel.fwire(dcm_out_pin),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire("DCM_OUT_R")]);
        }
    } else {
        let (crd, obid) = grid.get_clk_io(edge, bel.bid.to_idx()).unwrap();
        let onode = grid.get_io_node(vrf.grid, crd).unwrap();
        let obel = vrf.get_bel(bel.slr, onode, obid);
        vrf.verify_node(&[bel.fwire("CKI"), obel.fwire("IBUF")]);
        vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKI"));

        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT"));
        if grid.dcms == Some(Dcms::Eight) {
            let pad_pin;
            if grid.kind != GridKind::Spartan3A {
                pad_pin = "CKI";
            } else {
                pad_pin = "DCM_PAD";
                vrf.claim_node(&[bel.fwire("CKI")]);
                vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD"), bel.wire("CKI"));
            }
            let scol = if grid.kind == GridKind::Spartan3E {
                match edge {
                    Edge::Left => grid.col_left() + 9,
                    Edge::Right => grid.col_right() - 9,
                    _ => unreachable!(),
                }
            } else {
                match edge {
                    Edge::Left => grid.col_left() + 3,
                    Edge::Right => grid.col_right() - 6,
                    _ => unreachable!(),
                }
            };
            let srow = if bel.bid.to_idx() < 4 {
                grid.row_mid()
            } else {
                grid.row_mid() - 1
            };
            let obel = vrf.find_bel(bel.slr, (scol, srow), "DCMCONN.S3E").unwrap();
            let (dcm_pad_pin, dcm_out_pin) = match bel.bid.to_idx() {
                0 | 4 => ("CLKPAD0", "OUT0"),
                1 | 5 => ("CLKPAD1", "OUT1"),
                2 | 6 => ("CLKPAD2", "OUT2"),
                3 | 7 => ("CLKPAD3", "OUT3"),
                _ => unreachable!(),
            };
            vrf.verify_node(&[bel.fwire(pad_pin), obel.fwire(dcm_pad_pin)]);
            vrf.verify_node(&[bel.fwire("DCM_OUT"), obel.fwire(dcm_out_pin)]);
        } else {
            vrf.claim_node(&[bel.fwire("CKI")]);
        }
        let obel = vrf.find_bel_sibling(bel, "VCC");
        vrf.claim_pip(bel.crd(), bel.wire_far("CLK"), obel.wire("VCCOUT"));
        vrf.claim_pip(bel.crd(), bel.wire("S"), obel.wire("VCCOUT"));
    }
}

pub fn verify_gclkh(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        for ud in ["UP", "DN"] {
            if matches!((bel.key, ud), ("GCLKH.S", "UP") | ("GCLKH.N", "DN")) {
                continue;
            }
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{ud}{i}")),
                bel.wire(&format!("IN{i}")),
            );
        }
        if grid.kind.is_virtex2() {
            let lr = if bel.col < grid.col_clk { 'L' } else { 'R' };
            let obel = vrf
                .find_bel(bel.slr, (grid.col_clk, bel.row + 1), "GCLKC")
                .unwrap();
            vrf.verify_node(&[
                bel.fwire(&format!("IN{i}")),
                obel.fwire(&format!("OUT_{lr}{i}")),
            ]);
        } else if let Some((col_cl, col_cr)) = grid.cols_clkv {
            let scol = if bel.col < grid.col_clk {
                col_cl
            } else {
                col_cr
            };
            let lr = if bel.col < scol { 'L' } else { 'R' };
            let obel = vrf
                .find_bel(bel.slr, (scol, bel.row + 1), "GCLKVC")
                .unwrap();
            vrf.verify_node(&[
                bel.fwire(&format!("IN{i}")),
                obel.fwire(&format!("OUT_{lr}{i}")),
            ]);
        } else {
            let lr = if bel.col < grid.col_clk { 'L' } else { 'R' };
            let obel = vrf
                .find_bel(bel.slr, (grid.col_clk, grid.row_mid()), "CLKC_50A")
                .unwrap();
            vrf.verify_node(&[
                bel.fwire(&format!("IN{i}")),
                obel.fwire(&format!("OUT_{lr}{i}")),
            ]);
        }
    }
}

pub fn verify_gclkc(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        for lr in ['L', 'R'] {
            vrf.claim_node(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
            for bt in ['B', 'T'] {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("OUT_{lr}{i}")),
                    bel.wire(&format!("IN_{bt}{i}")),
                );
            }
        }
        for bt in ['B', 'T'] {
            let obel = vrf
                .find_bel(bel.slr, (grid.col_clk, grid.row_mid()), "CLKC")
                .unwrap();
            vrf.verify_node(&[
                bel.fwire(&format!("IN_{bt}{i}")),
                obel.fwire(&format!("OUT_{bt}{i}")),
            ]);
        }
    }
}

pub fn verify_clkc_v2(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        for bt in ['B', 'T'] {
            vrf.claim_node(&[(bel.crd(), bel.wire(&format!("OUT_{bt}{i}")))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{bt}{i}")),
                bel.wire(&format!("IN_{bt}{i}")),
            );
            let srow = if bt == 'B' {
                grid.row_bot()
            } else {
                grid.row_top()
            };
            let obel = vrf
                .find_bel(bel.slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{i}"))
                .unwrap();
            vrf.verify_node(&[bel.fwire(&format!("IN_{bt}{i}")), obel.fwire_far("O")]);
        }
    }
}

pub fn verify_clkc_s3(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        let (bt, j) = if i < 4 { ('B', i) } else { ('T', i - 4) };
        vrf.claim_node(&[bel.fwire(&format!("OUT{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("OUT{i}")),
            bel.wire(&format!("IN_{bt}{j}")),
        );
        let srow = if bt == 'B' {
            grid.row_bot()
        } else {
            grid.row_top()
        };
        let obel = vrf
            .find_bel(bel.slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{j}"))
            .unwrap();
        vrf.verify_node(&[bel.fwire(&format!("IN_{bt}{j}")), obel.fwire_far("O")]);
    }
}

pub fn verify_clkc_50a(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        let (bt, j) = if i < 4 { ('B', i) } else { ('T', i - 4) };
        for lr in ['L', 'R'] {
            vrf.claim_node(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{lr}{i}")),
                bel.wire(&format!("IN_{bt}{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{lr}{i}")),
                bel.wire(&format!("IN_{lr}{i}")),
            );
            let scol = if lr == 'L' {
                grid.col_left()
            } else {
                grid.col_right()
            };
            let obel = vrf
                .find_bel(bel.slr, (scol, grid.row_mid() - 1), &format!("BUFGMUX{i}"))
                .unwrap();
            vrf.verify_node(&[bel.fwire(&format!("IN_{lr}{i}")), obel.fwire_far("O")]);
        }
        let srow = if bt == 'B' {
            grid.row_bot()
        } else {
            grid.row_top()
        };
        let obel = vrf
            .find_bel(bel.slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{j}"))
            .unwrap();
        vrf.verify_node(&[bel.fwire(&format!("IN_{bt}{j}")), obel.fwire_far("O")]);
    }
}

pub fn verify_gclkvm(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        for ud in ["UP", "DN"] {
            vrf.claim_node(&[bel.fwire(&format!("OUT_{ud}{i}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{ud}{i}")),
                bel.wire(&format!("IN_CORE{i}")),
            );
            if grid.kind != GridKind::Spartan3 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("OUT_{ud}{i}")),
                    bel.wire(&format!("IN_LR{i}")),
                );
            }
        }
        let obel = vrf
            .find_bel(bel.slr, (grid.col_clk, bel.row), "CLKC")
            .unwrap();
        vrf.verify_node(&[
            bel.fwire(&format!("IN_CORE{i}")),
            obel.fwire(&format!("OUT{i}")),
        ]);
        if grid.kind != GridKind::Spartan3 {
            let scol = if bel.col < grid.col_clk {
                grid.col_left()
            } else {
                grid.col_right()
            };
            let obel = vrf
                .find_bel(bel.slr, (scol, grid.row_mid() - 1), &format!("BUFGMUX{i}"))
                .unwrap();
            vrf.verify_node(&[bel.fwire(&format!("IN_LR{i}")), obel.fwire_far("O")]);
        }
    }
}

pub fn verify_gclkvc(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        for lr in ['L', 'R'] {
            vrf.claim_node(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{lr}{i}")),
                bel.wire(&format!("IN{i}")),
            );
        }
        let ud = if bel.row < grid.row_mid() { "DN" } else { "UP" };
        let obel = vrf
            .find_bel(bel.slr, (bel.col, grid.row_mid()), "GCLKVM")
            .unwrap();
        vrf.verify_node(&[
            bel.fwire(&format!("IN{i}")),
            obel.fwire(&format!("OUT_{ud}{i}")),
        ]);
    }
}

pub fn verify_dcmconn(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let opin_pad;
    let pins_out;
    let pins_pad;
    if grid.kind.is_virtex2() {
        pins_out = &[
            ("OUTBUS0", "OUT0", "BUFGMUX0"),
            ("OUTBUS1", "OUT1", "BUFGMUX1"),
            ("OUTBUS2", "OUT2", "BUFGMUX2"),
            ("OUTBUS3", "OUT3", "BUFGMUX3"),
            ("OUTBUS4", "OUT0", "BUFGMUX4"),
            ("OUTBUS5", "OUT1", "BUFGMUX5"),
            ("OUTBUS6", "OUT2", "BUFGMUX6"),
            ("OUTBUS7", "OUT3", "BUFGMUX7"),
        ][..];
        if bel.col < grid.col_clk {
            opin_pad = "DCM_PAD_L";
            pins_pad = &[
                ("CLKPAD0", "CLKPADBUS0", "BUFGMUX4"),
                ("CLKPAD1", "CLKPADBUS1", "BUFGMUX5"),
                ("CLKPAD2", "CLKPADBUS2", "BUFGMUX6"),
                ("CLKPAD3", "CLKPADBUS3", "BUFGMUX7"),
                ("CLKPAD4", "CLKPADBUS4", "BUFGMUX0"),
                ("CLKPAD5", "CLKPADBUS5", "BUFGMUX1"),
                ("CLKPAD6", "CLKPADBUS6", "BUFGMUX2"),
                ("CLKPAD7", "CLKPADBUS7", "BUFGMUX3"),
            ][..];
        } else {
            opin_pad = "DCM_PAD_R";
            pins_pad = &[
                ("CLKPAD0", "CLKPADBUS0", "BUFGMUX0"),
                ("CLKPAD1", "CLKPADBUS1", "BUFGMUX1"),
                ("CLKPAD2", "CLKPADBUS2", "BUFGMUX2"),
                ("CLKPAD3", "CLKPADBUS3", "BUFGMUX3"),
                ("CLKPAD4", "CLKPADBUS4", "BUFGMUX4"),
                ("CLKPAD5", "CLKPADBUS5", "BUFGMUX5"),
                ("CLKPAD6", "CLKPADBUS6", "BUFGMUX6"),
                ("CLKPAD7", "CLKPADBUS7", "BUFGMUX7"),
            ][..];
        }
    } else {
        pins_out = &[
            ("OUTBUS0", "OUT0", "BUFGMUX0"),
            ("OUTBUS1", "OUT1", "BUFGMUX1"),
            ("OUTBUS2", "OUT2", "BUFGMUX2"),
            ("OUTBUS3", "OUT3", "BUFGMUX3"),
        ][..];
        opin_pad = "DCM_PAD";
        pins_pad = &[
            ("CLKPAD0", "CLKPADBUS0", "BUFGMUX0"),
            ("CLKPAD1", "CLKPADBUS1", "BUFGMUX1"),
            ("CLKPAD2", "CLKPADBUS2", "BUFGMUX2"),
            ("CLKPAD3", "CLKPADBUS3", "BUFGMUX3"),
        ][..];
    }
    let opin_out = if bel.col < grid.col_clk {
        "DCM_OUT_L"
    } else {
        "DCM_OUT_R"
    };
    for &(pin_o, pin_i, obk) in pins_out {
        vrf.claim_pip(bel.crd(), bel.wire(pin_o), bel.wire(pin_i));
        let obel = vrf
            .find_bel(bel.slr, (grid.col_clk - 1, bel.row), obk)
            .unwrap();
        vrf.verify_node(&[bel.fwire(pin_o), obel.fwire(opin_out)]);
    }
    for &(pin_o, pin_i, obk) in pins_pad {
        vrf.claim_pip(bel.crd(), bel.wire(pin_o), bel.wire(pin_i));
        let obel = vrf
            .find_bel(bel.slr, (grid.col_clk - 1, bel.row), obk)
            .unwrap();
        vrf.verify_node(&[bel.fwire(pin_i), obel.fwire(opin_pad)]);
    }
}

pub fn verify_brefclk(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("BREFCLK")]);
    vrf.claim_node(&[bel.fwire("BREFCLK2")]);
    if bel.row == grid.row_bot() {
        let obel = vrf.find_bel_sibling(bel, "BUFGMUX6");
        vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("CKI"));
        let obel = vrf.find_bel_sibling(bel, "BUFGMUX0");
        vrf.claim_pip(bel.crd(), bel.wire("BREFCLK2"), obel.wire_far("CKI"));
    } else {
        let obel = vrf.find_bel_sibling(bel, "BUFGMUX4");
        vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("CKI"));
        let obel = vrf.find_bel_sibling(bel, "BUFGMUX2");
        vrf.claim_pip(bel.crd(), bel.wire("BREFCLK2"), obel.wire_far("CKI"));
    }
}
