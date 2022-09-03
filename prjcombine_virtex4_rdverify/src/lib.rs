use prjcombine_entity::EntityId;
use prjcombine_int::db::BelId;
use prjcombine_int::grid::RowId;
use prjcombine_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_virtex4::Grid;

fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if matches!(bel.key, "SLICE0" | "SLICE2") {
        "SLICEM"
    } else {
        "SLICEL"
    };
    let mut pins = vec![
        ("FXINA", SitePinDir::In),
        ("FXINB", SitePinDir::In),
        ("F5", SitePinDir::Out),
        ("FX", SitePinDir::Out),
        ("CIN", SitePinDir::In),
        ("COUT", SitePinDir::Out),
    ];
    if kind == "SLICEM" {
        pins.extend([
            ("SHIFTIN", SitePinDir::In),
            ("SHIFTOUT", SitePinDir::Out),
            ("ALTDIG", SitePinDir::In),
            ("DIG", SitePinDir::Out),
            ("SLICEWE1", SitePinDir::In),
            ("BYOUT", SitePinDir::Out),
            ("BYINVOUT", SitePinDir::Out),
        ]);
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (dbel, dpin, sbel, spin) in [
        ("SLICE0", "FXINA", "SLICE0", "F5"),
        ("SLICE0", "FXINB", "SLICE2", "F5"),
        ("SLICE1", "FXINA", "SLICE1", "F5"),
        ("SLICE1", "FXINB", "SLICE3", "F5"),
        ("SLICE2", "FXINA", "SLICE0", "FX"),
        ("SLICE2", "FXINB", "SLICE1", "FX"),
        ("SLICE3", "FXINA", "SLICE2", "FX"),
        // SLICE3 FXINB <- top's SLICE2 FX

        // SLICE0 CIN <- bot's SLICE2 COUT
        // SLICE1 CIN <- bot's SLICE3 COUT
        ("SLICE2", "CIN", "SLICE0", "COUT"),
        ("SLICE3", "CIN", "SLICE1", "COUT"),
        ("SLICE0", "SHIFTIN", "SLICE2", "SHIFTOUT"),
        // SLICE2 SHIFTIN disconnected?
        ("SLICE0", "ALTDIG", "SLICE2", "DIG"),
        // SLICE2 ALTDIG disconnected?
        ("SLICE0", "SLICEWE1", "SLICE0", "BYOUT"),
        ("SLICE2", "SLICEWE1", "SLICE0", "BYINVOUT"),
    ] {
        if dbel != bel.key {
            continue;
        }
        let obel = vrf.find_bel_sibling(bel, sbel);
        vrf.claim_pip(bel.crd(), bel.wire(dpin), obel.wire(spin));
        vrf.claim_node(&[bel.fwire(dpin)]);
    }
    if bel.key == "SLICE2" {
        vrf.claim_node(&[bel.fwire("SHIFTIN")]);
        vrf.claim_node(&[bel.fwire("ALTDIG")]);
    }
    if bel.key == "SLICE3" {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, "SLICE2") {
            vrf.claim_node(&[bel.fwire("FXINB"), obel.fwire("FX_S")]);
            vrf.claim_pip(obel.crd(), obel.wire("FX_S"), obel.wire("FX"));
        } else {
            vrf.claim_node(&[bel.fwire("FXINB")]);
        }
    }
    for (dbel, sbel) in [("SLICE0", "SLICE2"), ("SLICE1", "SLICE3")] {
        if bel.key != dbel {
            continue;
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, sbel) {
            vrf.claim_node(&[bel.fwire("CIN"), obel.fwire("COUT_N")]);
            vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
        } else {
            vrf.claim_node(&[bel.fwire("CIN")]);
        }
    }
}

fn verify_bram(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "RAMB16",
        &[
            ("CASCADEINA", SitePinDir::In),
            ("CASCADEINB", SitePinDir::In),
            ("CASCADEOUTA", SitePinDir::Out),
            ("CASCADEOUTB", SitePinDir::Out),
        ],
        &[],
    );
    for (ipin, opin) in [("CASCADEINA", "CASCADEOUTA"), ("CASCADEINB", "CASCADEOUTB")] {
        vrf.claim_node(&[bel.fwire(opin)]);
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -4, bel.key) {
            vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
        } else {
            vrf.claim_node(&[bel.fwire(ipin)]);
        }
    }
}

fn verify_dsp(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    for i in 0..18 {
        pairs.push((format!("BCIN{i}"), format!("BCOUT{i}")));
    }
    for i in 0..48 {
        pairs.push((format!("PCIN{i}"), format!("PCOUT{i}")));
    }
    let mut pins = vec![];
    for (ipin, opin) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        if bel.key == "DSP0" {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -4, "DSP1") {
                vrf.claim_node(&[bel.fwire(ipin), obel.fwire_far(opin)]);
                vrf.claim_pip(obel.crd(), obel.wire_far(opin), obel.wire(opin));
            } else {
                vrf.claim_node(&[bel.fwire(ipin)]);
            }
        } else {
            vrf.claim_node(&[bel.fwire(ipin)]);
            let obel = vrf.find_bel_sibling(bel, "DSP0");
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_bel(bel, "DSP48", &pins, &[]);
}

fn verify_ppc(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut dcr_pins = vec![
        ("EMACDCRACK".to_string(), SitePinDir::In),
        ("DCREMACCLK".to_string(), SitePinDir::Out),
        ("DCREMACREAD".to_string(), SitePinDir::Out),
        ("DCREMACWRITE".to_string(), SitePinDir::Out),
    ];
    for i in 0..32 {
        dcr_pins.push((format!("EMACDCRDBUS{i}"), SitePinDir::In));
        dcr_pins.push((format!("DCREMACDBUS{i}"), SitePinDir::Out));
    }
    for i in 8..10 {
        dcr_pins.push((format!("DCREMACABUS{i}"), SitePinDir::Out));
    }
    let pins: Vec<_> = dcr_pins
        .iter()
        .map(|&(ref pin, dir)| (&pin[..], dir))
        .collect();
    vrf.verify_bel(bel, "PPC405_ADV", &pins, &[]);
    let obel = vrf.find_bel_sibling(bel, "EMAC");
    for (pin, dir) in dcr_pins {
        vrf.claim_node(&[bel.fwire(&pin)]);
        match dir {
            SitePinDir::In => vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire(&pin)),
            SitePinDir::Out => vrf.claim_pip(bel.crd(), obel.wire(&pin), bel.wire(&pin)),
            _ => unreachable!(),
        }
    }
}

fn verify_emac(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut dcr_pins = vec![
        ("EMACDCRACK".to_string(), SitePinDir::Out),
        ("DCREMACCLK".to_string(), SitePinDir::In),
        ("DCREMACREAD".to_string(), SitePinDir::In),
        ("DCREMACWRITE".to_string(), SitePinDir::In),
    ];
    for i in 0..32 {
        dcr_pins.push((format!("EMACDCRDBUS{i}"), SitePinDir::Out));
        dcr_pins.push((format!("DCREMACDBUS{i}"), SitePinDir::In));
    }
    for i in 8..10 {
        dcr_pins.push((format!("DCREMACABUS{i}"), SitePinDir::In));
    }
    let pins: Vec<_> = dcr_pins
        .iter()
        .map(|&(ref pin, dir)| (&pin[..], dir))
        .collect();
    vrf.verify_bel(bel, "EMAC", &pins, &[]);
    for (pin, _) in dcr_pins {
        vrf.claim_node(&[bel.fwire(&pin)]);
    }
}

fn verify_bufgctrl(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFGCTRL",
        &[
            ("I0", SitePinDir::In),
            ("I1", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &["I0MUX", "I1MUX", "CKINT0", "CKINT1"],
    );
    let is_b = bel.bid.to_idx() < 16;
    vrf.claim_node(&[bel.fwire("I0")]);
    vrf.claim_node(&[bel.fwire("I1")]);
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("I0MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("I1MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), bel.wire("MUXBUS0"));
    vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), bel.wire("MUXBUS1"));
    for i in 0..16 {
        let obid = if is_b {
            BelId::from_idx(i)
        } else {
            BelId::from_idx(i + 16)
        };
        let obel = vrf.get_bel(bel.die, bel.col, bel.row, bel.node, obid);
        vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), obel.wire("GFB"));
        vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), obel.wire("GFB"));
    }
    let obel = vrf.find_bel_sibling(
        bel,
        if is_b {
            "BUFG_MGTCLK_B"
        } else {
            "BUFG_MGTCLK_T"
        },
    );
    for pin in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
        vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), obel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), obel.wire(pin));
    }
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_node(&[bel.fwire("GCLK")]);
    vrf.claim_node(&[bel.fwire("GFB")]);
    vrf.claim_pip(bel.crd(), bel.wire("GCLK"), bel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("GFB"), bel.wire("O"));
    let srow = if is_b {
        grid.row_dcmiob()
    } else {
        grid.row_iobdcm() - 16
    };
    let obel = vrf.find_bel(bel.die, (bel.col, srow), "CLK_IOB").unwrap();
    let idx0 = (bel.bid.to_idx() % 16) * 2;
    let idx1 = (bel.bid.to_idx() % 16) * 2 + 1;
    vrf.verify_node(&[bel.fwire("MUXBUS0"), obel.fwire(&format!("MUXBUS_O{idx0}"))]);
    vrf.verify_node(&[bel.fwire("MUXBUS1"), obel.fwire(&format!("MUXBUS_O{idx1}"))]);
}

fn verify_bufg_mgtclk(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if grid.has_mgt() {
        let obel = vrf.find_bel_sibling(
            bel,
            match bel.key {
                "BUFG_MGTCLK_B" => "BUFG_MGTCLK_B_HROW",
                "BUFG_MGTCLK_T" => "BUFG_MGTCLK_T_HROW",
                _ => unreachable!(),
            },
        );
        for (pin, pin_o) in [
            ("MGT_L0", "MGT_L0_O"),
            ("MGT_L1", "MGT_L1_O"),
            ("MGT_R0", "MGT_R0_O"),
            ("MGT_R1", "MGT_R1_O"),
        ] {
            vrf.verify_node(&[bel.fwire(pin), obel.fwire(pin_o)]);
        }
    } else {
        for pin in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
}

fn verify_bufg_mgtclk_hrow(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if grid.has_mgt() {
        let obel = vrf.find_bel_sibling(
            bel,
            match bel.key {
                "BUFG_MGTCLK_B_HROW" => "BUFG_MGTCLK_B_HCLK",
                "BUFG_MGTCLK_T_HROW" => "BUFG_MGTCLK_T_HCLK",
                _ => unreachable!(),
            },
        );
        for (pin_i, pin_o) in [
            ("MGT_L0_I", "MGT_L0_O"),
            ("MGT_L1_I", "MGT_L1_O"),
            ("MGT_R0_I", "MGT_R0_O"),
            ("MGT_R1_I", "MGT_R1_O"),
        ] {
            vrf.verify_node(&[bel.fwire(pin_i), obel.fwire(pin_o)]);
            vrf.claim_node(&[bel.fwire(pin_o)]);
            vrf.claim_pip(bel.crd(), bel.wire(pin_o), bel.wire(pin_i));
        }
    }
}

fn verify_bufg_mgtclk_hclk(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if grid.has_mgt() {
        for (pin_i, pin_o) in [
            ("MGT_L0_I", "MGT_L0_O"),
            ("MGT_L1_I", "MGT_L1_O"),
            ("MGT_R0_I", "MGT_R0_O"),
            ("MGT_R1_I", "MGT_R1_O"),
        ] {
            vrf.claim_node(&[bel.fwire(pin_o)]);
            vrf.claim_pip(bel.crd(), bel.wire(pin_o), bel.wire(pin_i));
        }
        // XXX source from MGT
    }
}

fn verify_jtagppc(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "JTAGPPC", &[("TDOTSPPC", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("TDOTSPPC")]);
}

fn verify_clk_hrow(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        vrf.claim_node(&[bel.fwire(&format!("OUT_L{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("OUT_R{i}"))]);
        for j in 0..32 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_L{i}")),
                bel.wire(&format!("GCLK{j}")),
            );
        }
    }
    for i in 0..32 {
        let orow = RowId::from_idx(grid.reg_cfg * 16 - 8);
        let obel = vrf
            .find_bel(bel.die, (bel.col, orow), &format!("BUFGCTRL{i}"))
            .unwrap();
        vrf.verify_node(&[bel.fwire(&format!("GCLK{i}")), obel.fwire("GCLK")]);
    }
}

fn verify_clk_iob(_grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..16 {
        vrf.claim_node(&[bel.fwire(&format!("PAD_BUF{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("GIOB{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PAD_BUF{i}")),
            bel.wire(&format!("PAD{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GIOB{i}")),
            bel.wire(&format!("PAD_BUF{i}")),
        );
        // XXX source PAD
    }
    for i in 0..32 {
        vrf.claim_node(&[bel.fwire(&format!("MUXBUS_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MUXBUS_O{i}")),
            bel.wire(&format!("MUXBUS_I{i}")),
        );
        for j in 0..16 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("MUXBUS_O{i}")),
                bel.wire(&format!("PAD_BUF{j}")),
            );
        }
        // XXX source MUXBUS_I
    }
}

fn verify_bufr(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFR",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
    let obel = vrf.find_bel_sibling(bel, "BUFIO0");
    vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, "BUFIO1");
    vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, "RCLK");
    vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire("CKINT1"));
}

fn verify_bufio(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFIO",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
    // XXX source I thru PAD
}

fn verify_idelayctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IDELAYCTRL", &[("REFCLK", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("REFCLK")]);
    let obel = vrf.find_bel_sibling(bel, "IOCLK");
    for i in 0..8 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REFCLK"),
            obel.wire(&format!("GCLK_OUT{i}")),
        );
    }
}

fn verify_rclk(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("VRCLK0")]);
    vrf.claim_node(&[bel.fwire("VRCLK1")]);
    // beware they are swapped [!]
    let obel = vrf.find_bel_sibling(bel, "BUFR1");
    vrf.claim_pip(bel.crd(), bel.wire("VRCLK0"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, "BUFR0");
    vrf.claim_pip(bel.crd(), bel.wire("VRCLK1"), obel.wire("O"));

    let obel_s = vrf.find_bel_delta(bel, 0, 16, "RCLK");
    let obel_n = vrf.find_bel_delta(bel, 0, -16, "RCLK");
    if let Some(ref obel) = obel_s {
        vrf.verify_node(&[bel.fwire("VRCLK_S0"), obel.fwire("VRCLK0")]);
        vrf.verify_node(&[bel.fwire("VRCLK_S1"), obel.fwire("VRCLK1")]);
    }
    if let Some(ref obel) = obel_n {
        vrf.verify_node(&[bel.fwire("VRCLK_N0"), obel.fwire("VRCLK0")]);
        vrf.verify_node(&[bel.fwire("VRCLK_N1"), obel.fwire("VRCLK1")]);
    }
    for opin in ["RCLK0", "RCLK1"] {
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK0"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK1"));
        if obel_s.is_some() {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK_S0"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK_S1"));
        }
        if obel_n.is_some() {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK_N0"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK_N1"));
        }
    }
}

fn verify_ioclk(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf
        .find_bel(bel.die, (grid.cols_io[1], bel.row), "CLK_HROW")
        .unwrap();
    let lr = if bel.col <= grid.cols_io[1] { 'L' } else { 'R' };
    for i in 0..8 {
        vrf.claim_node(&[bel.fwire(&format!("GCLK_OUT{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK_OUT{i}")),
            bel.wire(&format!("GCLK_IN{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("GCLK_IN{i}")),
            obel.fwire(&format!("OUT_{lr}{i}")),
        ]);
    }

    let scol = if bel.col <= grid.cols_io[1] {
        grid.cols_io[0]
    } else {
        grid.cols_io[2]
    };
    let obel = vrf.find_bel(bel.die, (scol, bel.row), "RCLK").unwrap();
    for i in 0..2 {
        vrf.claim_node(&[bel.fwire(&format!("RCLK_OUT{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK_OUT{i}")),
            bel.wire(&format!("RCLK_IN{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK_IN{i}")),
            obel.fwire(&format!("RCLK{i}")),
        ]);
    }

    vrf.claim_node(&[bel.fwire("VIOCLK0")]);
    vrf.claim_node(&[bel.fwire("VIOCLK1")]);
    // beware they are swapped [!]
    let obel = vrf.find_bel_sibling(bel, "BUFIO1");
    vrf.claim_pip(bel.crd(), bel.wire("VIOCLK0"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, "BUFIO0");
    vrf.claim_pip(bel.crd(), bel.wire("VIOCLK1"), obel.wire("O"));

    vrf.claim_node(&[bel.fwire("IOCLK0")]);
    vrf.claim_node(&[bel.fwire("IOCLK1")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK0"), bel.wire("VIOCLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK1"), bel.wire("VIOCLK1"));

    if let Some(obel) = vrf.find_bel_delta(bel, 0, 16, "IOCLK") {
        if vrf.find_bel_delta(bel, 0, 0, "STARTUP").is_none() {
            vrf.verify_node(&[bel.fwire("VIOCLK_S0"), obel.fwire("VIOCLK0")]);
            vrf.verify_node(&[bel.fwire("VIOCLK_S1"), obel.fwire("VIOCLK1")]);
            vrf.claim_node(&[bel.fwire("IOCLK_S0")]);
            vrf.claim_node(&[bel.fwire("IOCLK_S1")]);
            vrf.claim_pip(bel.crd(), bel.wire("IOCLK_S0"), bel.wire("VIOCLK_S0"));
            vrf.claim_pip(bel.crd(), bel.wire("IOCLK_S1"), bel.wire("VIOCLK_S1"));
        }
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -16, "IOCLK") {
        if vrf.find_bel_delta(bel, 0, -16, "STARTUP").is_none() {
            vrf.verify_node(&[bel.fwire("VIOCLK_N0"), obel.fwire("VIOCLK0")]);
            vrf.verify_node(&[bel.fwire("VIOCLK_N1"), obel.fwire("VIOCLK1")]);
            vrf.claim_node(&[bel.fwire("IOCLK_N0")]);
            vrf.claim_node(&[bel.fwire("IOCLK_N1")]);
            vrf.claim_pip(bel.crd(), bel.wire("IOCLK_N0"), bel.wire("VIOCLK_N0"));
            vrf.claim_pip(bel.crd(), bel.wire("IOCLK_N1"), bel.wire("VIOCLK_N1"));
        }
    }
}

fn verify_hclk_dcm(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, "HCLK_DCM_HROW");
    for i in 0..16 {
        vrf.verify_node(&[
            bel.fwire(&format!("GIOB_I{i}")),
            obel.fwire(&format!("GIOB_O{i}")),
        ]);
        if bel.key != "HCLK_DCM_S" {
            vrf.claim_node(&[bel.fwire(&format!("GIOB_O_U{i}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("GIOB_O_U{i}")),
                bel.wire(&format!("GIOB_I{i}")),
            );
        }
        if bel.key != "HCLK_DCM_N" {
            vrf.claim_node(&[bel.fwire(&format!("GIOB_O_D{i}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("GIOB_O_D{i}")),
                bel.wire(&format!("GIOB_I{i}")),
            );
        }
    }
    let has_sysmon_s = vrf.find_bel_delta(bel, 0, -8, "SYSMON").is_some();
    let has_sysmon_n = vrf.find_bel_delta(bel, 0, 0, "SYSMON").is_some();
    let obel = vrf.find_bel_sibling(bel, "CLK_HROW");
    for i in 0..8 {
        vrf.verify_node(&[
            bel.fwire(&format!("GCLK_I{i}")),
            obel.fwire(&format!("OUT_L{i}")),
        ]);
        if bel.key != "HCLK_DCM_S" && !has_sysmon_n {
            vrf.claim_node(&[bel.fwire(&format!("GCLK_O_U{i}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("GCLK_O_U{i}")),
                bel.wire(&format!("GCLK_I{i}")),
            );
        }
        if bel.key != "HCLK_DCM_N" && !has_sysmon_s {
            vrf.claim_node(&[bel.fwire(&format!("GCLK_O_D{i}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("GCLK_O_D{i}")),
                bel.wire(&format!("GCLK_I{i}")),
            );
        }
    }
    match bel.key {
        "HCLK_DCM" => {
            for i in 0..4 {
                if grid.has_mgt() || !has_sysmon_s {
                    let skip = !grid.has_mgt() && bel.row.to_idx() == grid.regs * 16 - 8;
                    if !skip {
                        vrf.claim_node(&[bel.fwire(&format!("MGT{i}"))]);
                    }
                    if grid.has_mgt() {
                        vrf.claim_pip(
                            bel.crd(),
                            bel.wire(&format!("MGT{i}")),
                            bel.wire(&format!("MGT_I{i}")),
                        );
                    }
                    if !has_sysmon_s {
                        vrf.claim_node(&[bel.fwire(&format!("MGT_O_D{i}"))]);
                        if !skip {
                            vrf.claim_pip(
                                bel.crd(),
                                bel.wire(&format!("MGT_O_D{i}")),
                                bel.wire(&format!("MGT{i}")),
                            );
                        }
                    }
                    if !has_sysmon_n {
                        vrf.claim_node(&[bel.fwire(&format!("MGT_O_U{i}"))]);
                        if !skip {
                            vrf.claim_pip(
                                bel.crd(),
                                bel.wire(&format!("MGT_O_U{i}")),
                                bel.wire(&format!("MGT{i}")),
                            );
                        }
                    }
                }
            }
        }
        "HCLK_DCM_S" => {
            if grid.has_mgt() {
                for i in 0..4 {
                    vrf.claim_node(&[bel.fwire(&format!("MGT_O_D{i}"))]);
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("MGT_O_D{i}")),
                        bel.wire(&format!("MGT_I{i}")),
                    );
                }
            }
        }
        "HCLK_DCM_N" => {
            if grid.has_mgt() {
                for i in 0..4 {
                    vrf.claim_node(&[bel.fwire(&format!("MGT_O_U{i}"))]);
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("MGT_O_U{i}")),
                        bel.wire(&format!("MGT_I{i}")),
                    );
                }
            }
        }
        _ => unreachable!(),
    }
    if grid.has_mgt() {
        // XXX source MGT_I
    }
}

fn verify_hclk_dcm_hrow(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let srow = if bel.row <= grid.row_dcmiob() {
        grid.row_dcmiob()
    } else {
        grid.row_iobdcm() - 16
    };
    let obel = vrf.find_bel(bel.die, (bel.col, srow), "CLK_IOB").unwrap();
    for i in 0..16 {
        vrf.claim_node(&[bel.fwire(&format!("GIOB_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GIOB_O{i}")),
            bel.wire(&format!("GIOB_I{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("GIOB_I{i}")),
            obel.fwire(&format!("GIOB{i}")),
        ]);
    }
}

fn verify_hclk(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf
        .find_bel(bel.die, (grid.cols_io[1], bel.row), "CLK_HROW")
        .unwrap();
    let lr = if bel.col <= grid.cols_io[1] { 'L' } else { 'R' };
    for i in 0..8 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK_O{i}")),
            bel.wire(&format!("GCLK_I{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("GCLK_I{i}")),
            obel.fwire(&format!("OUT_{lr}{i}")),
        ]);
    }
    let scol = if bel.col <= grid.cols_io[1] {
        grid.cols_io[0]
    } else {
        grid.cols_io[2]
    };
    let obel = vrf.find_bel(bel.die, (scol, bel.row), "RCLK").unwrap();
    for i in 0..2 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK_O{i}")),
            bel.wire(&format!("RCLK_I{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK_I{i}")),
            obel.fwire(&format!("RCLK{i}")),
        ]);
    }
}

fn verify_sysmon(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "MONITOR",
        &[
            ("CONVST", SitePinDir::In),
            ("VP", SitePinDir::In),
            ("VP1", SitePinDir::In),
            ("VP2", SitePinDir::In),
            ("VP3", SitePinDir::In),
            ("VP4", SitePinDir::In),
            ("VP5", SitePinDir::In),
            ("VP6", SitePinDir::In),
            ("VP7", SitePinDir::In),
            ("VN", SitePinDir::In),
            ("VN1", SitePinDir::In),
            ("VN2", SitePinDir::In),
            ("VN3", SitePinDir::In),
            ("VN4", SitePinDir::In),
            ("VN5", SitePinDir::In),
            ("VN6", SitePinDir::In),
            ("VN7", SitePinDir::In),
        ],
        &["CONVST_INT_IMUX", "CONVST_INT_CLK", "CONVST_TEST"],
    );
    vrf.claim_node(&[bel.fwire("CONVST")]);
    for pin in ["CONVST", "CONVST_TEST"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CONVST_INT_IMUX"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CONVST_INT_CLK"));
        for i in 0..16 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire(&format!("GIOB{i}")));
        }
    }
    let srow = RowId::from_idx(bel.row.to_idx() / 16 * 16 + 8);
    let obel = vrf
        .find_bel(bel.die, (bel.col, srow), "HCLK_DCM")
        .or_else(|| vrf.find_bel(bel.die, (bel.col, srow), "HCLK_DCM_S"))
        .or_else(|| vrf.find_bel(bel.die, (bel.col, srow), "HCLK_DCM_N"))
        .unwrap();
    let ud = if bel.row.to_idx() % 16 < 8 { 'D' } else { 'U' };
    for i in 0..16 {
        vrf.verify_node(&[
            bel.fwire(&format!("GIOB{i}")),
            obel.fwire(&format!("GIOB_O_{ud}{i}")),
        ]);
    }
    vrf.claim_node(&[bel.fwire("VP")]);
    let obel = vrf.find_bel_sibling(bel, "IPAD0");
    vrf.claim_pip(bel.crd(), bel.wire("VP"), obel.wire("O"));
    vrf.claim_node(&[bel.fwire("VN")]);
    let obel = vrf.find_bel_sibling(bel, "IPAD1");
    vrf.claim_pip(bel.crd(), bel.wire("VN"), obel.wire("O"));
    for i in 1..8 {
        vrf.claim_node(&[bel.fwire(&format!("VP{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("VN{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VP{i}")),
            bel.wire_far(&format!("VP{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VN{i}")),
            bel.wire_far(&format!("VN{i}")),
        );
        // XXX source
    }
}

fn verify_ipad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
}

pub fn verify_bel(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("SLICE") => verify_slice(vrf, bel),
        _ if bel.key.starts_with("DSP") => verify_dsp(vrf, bel),
        "BRAM" => verify_bram(vrf, bel),
        "FIFO" => vrf.verify_bel(bel, "FIFO16", &[], &[]),
        "PPC" => verify_ppc(vrf, bel),
        "EMAC" => verify_emac(vrf, bel),

        _ if bel.key.starts_with("BUFGCTRL") => verify_bufgctrl(grid, vrf, bel),
        _ if bel.key.starts_with("BSCAN") => vrf.verify_bel(bel, "BSCAN", &[], &[]),
        _ if bel.key.starts_with("ICAP") => vrf.verify_bel(bel, "ICAP", &[], &[]),
        "PMV" | "STARTUP" | "FRAME_ECC" | "DCIRESET" | "CAPTURE" | "USR_ACCESS" | "DCI" => {
            vrf.verify_bel(bel, bel.key, &[], &[])
        }
        "JTAGPPC" => verify_jtagppc(vrf, bel),
        "BUFG_MGTCLK_B" | "BUFG_MGTCLK_T" => verify_bufg_mgtclk(grid, vrf, bel),
        "BUFG_MGTCLK_B_HROW" | "BUFG_MGTCLK_T_HROW" => verify_bufg_mgtclk_hrow(grid, vrf, bel),
        "BUFG_MGTCLK_B_HCLK" | "BUFG_MGTCLK_T_HCLK" => verify_bufg_mgtclk_hclk(grid, vrf, bel),

        "CLK_HROW" => verify_clk_hrow(grid, vrf, bel),
        "CLK_IOB" => verify_clk_iob(grid, vrf, bel),

        _ if bel.key.starts_with("BUFR") => verify_bufr(vrf, bel),
        _ if bel.key.starts_with("BUFIO") => verify_bufio(vrf, bel),
        "IDELAYCTRL" => verify_idelayctrl(vrf, bel),
        "RCLK" => verify_rclk(vrf, bel),
        "IOCLK" => verify_ioclk(grid, vrf, bel),
        "HCLK_DCM" | "HCLK_DCM_S" | "HCLK_DCM_N" => verify_hclk_dcm(grid, vrf, bel),
        "HCLK_DCM_HROW" => verify_hclk_dcm_hrow(grid, vrf, bel),
        "HCLK" => verify_hclk(grid, vrf, bel),

        "SYSMON" => verify_sysmon(vrf, bel),
        _ if bel.key.starts_with("IPAD") => verify_ipad(vrf, bel),

        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}
