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
    for pin in ["F5", "FX", "COUT"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
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
        for pin in ["DIG", "BYOUT", "BYINVOUT", "SHIFTOUT"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
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
        vrf.claim_node(&[bel.fwire(ipin)]);
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -4, bel.key) {
            vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
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
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_R{i}")),
                bel.wire(&format!("GCLK{j}")),
            );
        }
    }
    for i in 0..32 {
        let orow = grid.row_cfg_below();
        let obel = vrf
            .find_bel(bel.die, (bel.col, orow), &format!("BUFGCTRL{i}"))
            .unwrap();
        vrf.verify_node(&[bel.fwire(&format!("GCLK{i}")), obel.fwire("GCLK")]);
    }
}

fn verify_clk_iob(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
        let obel = vrf.find_bel_delta(bel, 0, i, "ILOGIC0").unwrap();
        vrf.verify_node(&[bel.fwire(&format!("PAD{i}")), obel.fwire("CLKOUT")]);
        // avoid double-claim for IOBs that are also BUFIO inps
        if !matches!(obel.row.to_idx() % 16, 7 | 8) {
            vrf.claim_node(&[obel.fwire("CLKOUT")]);
            vrf.claim_pip(obel.crd(), obel.wire("CLKOUT"), obel.wire("O"));
        }
    }
    let dy = if bel.row < grid.row_cfg_below() {
        -8
    } else {
        16
    };
    let obel = vrf.find_bel_delta(bel, 0, dy, "CLK_DCM").unwrap();
    for i in 0..32 {
        vrf.claim_node(&[bel.fwire(&format!("MUXBUS_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MUXBUS_O{i}")),
            bel.wire(&format!("MUXBUS_I{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("MUXBUS_I{i}")),
            obel.fwire(&format!("MUXBUS_O{i}")),
        ]);
        for j in 0..16 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("MUXBUS_O{i}")),
                bel.wire(&format!("PAD_BUF{j}")),
            );
        }
    }
}

fn verify_clk_dcm(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..2 {
        let obel = vrf
            .find_bel(bel.die, (bel.col, bel.row + i * 4), "DCM")
            .or_else(|| vrf.find_bel(bel.die, (bel.col, bel.row + i * 4), "CCM"))
            .unwrap();
        for j in 0..12 {
            vrf.claim_node(&[bel.fwire(&format!("DCM{k}", k = j + i * 12))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("DCM{k}", k = j + i * 12)),
                bel.wire(&format!("DCM{i}_{j}")),
            );
            vrf.verify_node(&[
                bel.fwire(&format!("DCM{i}_{j}")),
                obel.fwire(&format!("TO_BUFG{j}")),
            ]);
        }
    }
    let dy = if bel.row < grid.row_cfg_below() {
        -8
    } else {
        8
    };
    let obel = vrf.find_bel_delta(bel, 0, dy, "CLK_DCM");
    for i in 0..32 {
        vrf.claim_node(&[bel.fwire(&format!("MUXBUS_O{i}"))]);
        if let Some(ref obel) = obel {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("MUXBUS_O{i}")),
                bel.wire(&format!("MUXBUS_I{i}")),
            );
            vrf.verify_node(&[
                bel.fwire(&format!("MUXBUS_I{i}")),
                obel.fwire(&format!("MUXBUS_O{i}")),
            ]);
        }
        for j in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("MUXBUS_O{i}")),
                bel.wire(&format!("DCM{j}")),
            );
        }
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
    let dy = match bel.key {
        "BUFIO0" => 0,
        "BUFIO1" => -1,
        _ => unreachable!(),
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, "ILOGIC0") {
        vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire("PAD"));
        vrf.claim_node(&[bel.fwire("PAD"), obel.fwire("CLKOUT")]);
        vrf.claim_pip(obel.crd(), obel.wire("CLKOUT"), obel.wire("O"));
    }
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
    let obel = vrf.find_bel_sibling(bel, "BUFR0");
    vrf.claim_pip(bel.crd(), bel.wire("VRCLK0"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, "BUFR1");
    vrf.claim_pip(bel.crd(), bel.wire("VRCLK1"), obel.wire("O"));

    let obel_s = vrf.find_bel_delta(bel, 0, 16, "RCLK");
    let obel_n = vrf.find_bel_delta(bel, 0, -16, "RCLK");
    if let Some(ref obel) = obel_s {
        vrf.verify_node(&[bel.fwire("VRCLK_S0"), obel.fwire("VRCLK0")]);
        vrf.verify_node(&[bel.fwire("VRCLK_S1"), obel.fwire("VRCLK1")]);
    } else {
        vrf.claim_node(&[bel.fwire("VRCLK_S0")]);
        vrf.claim_node(&[bel.fwire("VRCLK_S1")]);
    }
    if let Some(ref obel) = obel_n {
        vrf.verify_node(&[bel.fwire("VRCLK_N0"), obel.fwire("VRCLK0")]);
        vrf.verify_node(&[bel.fwire("VRCLK_N1"), obel.fwire("VRCLK1")]);
    } else {
        vrf.claim_node(&[bel.fwire("VRCLK_N0")]);
        vrf.claim_node(&[bel.fwire("VRCLK_N1")]);
    }
    for opin in ["RCLK0", "RCLK1"] {
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK0"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK1"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK_S0"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK_S1"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK_N0"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("VRCLK_N1"));
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
    let obel = vrf.find_bel_sibling(bel, "BUFIO0");
    vrf.claim_pip(bel.crd(), bel.wire("VIOCLK0"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, "BUFIO1");
    vrf.claim_pip(bel.crd(), bel.wire("VIOCLK1"), obel.wire("O"));

    vrf.claim_node(&[bel.fwire("IOCLK0")]);
    vrf.claim_node(&[bel.fwire("IOCLK1")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK0"), bel.wire("VIOCLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK1"), bel.wire("VIOCLK1"));

    let mut claim_s = bel.col != grid.cols_io[1];
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 16, "IOCLK") {
        if vrf.find_bel_delta(bel, 0, 0, "STARTUP").is_none() {
            vrf.verify_node(&[bel.fwire("VIOCLK_S0"), obel.fwire("VIOCLK0")]);
            vrf.verify_node(&[bel.fwire("VIOCLK_S1"), obel.fwire("VIOCLK1")]);
            vrf.claim_pip(bel.crd(), bel.wire("IOCLK_S0"), bel.wire("VIOCLK_S0"));
            vrf.claim_pip(bel.crd(), bel.wire("IOCLK_S1"), bel.wire("VIOCLK_S1"));
            claim_s = true;
        }
    }
    if claim_s {
        vrf.claim_node(&[bel.fwire("IOCLK_S0")]);
        vrf.claim_node(&[bel.fwire("IOCLK_S1")]);
    }
    let mut claim_n = bel.col != grid.cols_io[1];
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -16, "IOCLK") {
        if vrf.find_bel_delta(bel, 0, -16, "STARTUP").is_none() {
            vrf.verify_node(&[bel.fwire("VIOCLK_N0"), obel.fwire("VIOCLK0")]);
            vrf.verify_node(&[bel.fwire("VIOCLK_N1"), obel.fwire("VIOCLK1")]);
            vrf.claim_pip(bel.crd(), bel.wire("IOCLK_N0"), bel.wire("VIOCLK_N0"));
            vrf.claim_pip(bel.crd(), bel.wire("IOCLK_N1"), bel.wire("VIOCLK_N1"));
            claim_n = true;
        }
    }
    if claim_n {
        vrf.claim_node(&[bel.fwire("IOCLK_N0")]);
        vrf.claim_node(&[bel.fwire("IOCLK_N1")]);
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

fn verify_dcm(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "DCM_ADV",
        &[("CLKIN", SitePinDir::In), ("CLKFB", SitePinDir::In)],
        &[
            "CLKIN_TEST",
            "CLKFB_TEST",
            "CKINT0",
            "CKINT1",
            "CKINT2",
            "CKINT3",
            "CLK_IN0",
        ],
    );
    vrf.claim_node(&[bel.fwire("CLKIN")]);
    vrf.claim_node(&[bel.fwire("CLKFB")]);
    for pin in ["CLKIN", "CLKIN_TEST", "CLKFB", "CLKFB_TEST"] {
        for ipin in [
            "CKINT0", "CKINT1", "CKINT2", "CKINT3", "BUSOUT0", "BUSOUT1", "GCLK0", "GCLK1",
            "GCLK2", "GCLK3", "GCLK4", "GCLK5", "GCLK6", "GCLK7", "GIOB0", "GIOB1", "GIOB2",
            "GIOB3", "GIOB4", "GIOB5", "GIOB6", "GIOB7", "GIOB8", "GIOB9", "GIOB10", "GIOB11",
            "GIOB12", "GIOB13", "GIOB14", "GIOB15", "MGT0", "MGT1", "MGT2", "MGT3",
        ] {
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire(ipin));
        }
    }
    for i in 0..24 {
        let opin = format!("BUSOUT{i}");
        let ipin = format!("BUSIN{i}");
        vrf.claim_node(&[bel.fwire(&opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(&opin), bel.wire(&ipin));
        for pin in [
            "CLK0_BUF",
            "CLK90_BUF",
            "CLK180_BUF",
            "CLK270_BUF",
            "CLK2X_BUF",
            "CLK2X180_BUF",
            "CLKDV_BUF",
            "CLKFX_BUF",
            "CLKFX180_BUF",
            "CONCUR_BUF",
            "LOCKED_BUF",
            "CLK_IN0",
        ] {
            vrf.claim_pip(bel.crd(), bel.wire(&opin), bel.wire(pin));
        }
    }
    for (pin, bpin, opin) in [
        ("CONCUR", "CONCUR_BUF", "TO_BUFG1"),
        ("CLKFX", "CLKFX_BUF", "TO_BUFG2"),
        ("CLKFX180", "CLKFX180_BUF", "TO_BUFG3"),
        ("CLK0", "CLK0_BUF", "TO_BUFG4"),
        ("CLK180", "CLK180_BUF", "TO_BUFG5"),
        ("CLK90", "CLK90_BUF", "TO_BUFG6"),
        ("CLK270", "CLK270_BUF", "TO_BUFG7"),
        ("CLK2X180", "CLK2X180_BUF", "TO_BUFG8"),
        ("CLK2X", "CLK2X_BUF", "TO_BUFG9"),
        ("CLKDV", "CLKDV_BUF", "TO_BUFG10"),
    ] {
        vrf.claim_node(&[bel.fwire(bpin)]);
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(bpin), bel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(pin));
    }
    vrf.claim_node(&[bel.fwire("TO_BUFG0")]);
    vrf.claim_node(&[bel.fwire("TO_BUFG11")]);
    vrf.claim_node(&[bel.fwire("LOCKED_BUF")]);
    vrf.claim_pip(bel.crd(), bel.wire("LOCKED_BUF"), bel.wire("LOCKED"));
    let dy = if bel.row < grid.row_dcmiob() { -4 } else { 4 };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, "DCM") {
        for i in 0..24 {
            let opin = format!("BUSOUT{i}");
            let ipin = format!("BUSIN{i}");
            vrf.verify_node(&[bel.fwire(&ipin), obel.fwire(&opin)]);
        }
    } else {
        for i in 0..24 {
            let ipin = format!("BUSIN{i}");
            vrf.claim_node(&[bel.fwire(&ipin)]);
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
    for i in 0..8 {
        vrf.verify_node(&[
            bel.fwire(&format!("GCLK{i}")),
            obel.fwire(&format!("GCLK_O_{ud}{i}")),
        ]);
    }
    if grid.has_mgt() {
        for i in 0..4 {
            vrf.verify_node(&[
                bel.fwire(&format!("MGT{i}")),
                obel.fwire(&format!("MGT_O_{ud}{i}")),
            ]);
        }
    }
    // XXX verify MGT
}

fn verify_pmcd(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLKA", SitePinDir::In),
        ("CLKB", SitePinDir::In),
        ("CLKC", SitePinDir::In),
        ("CLKD", SitePinDir::In),
        ("REL", SitePinDir::In),
        ("CLKA1", SitePinDir::Out),
        ("CLKA1D2", SitePinDir::Out),
        ("CLKA1D4", SitePinDir::Out),
        ("CLKA1D8", SitePinDir::Out),
        ("CLKB1", SitePinDir::Out),
        ("CLKC1", SitePinDir::Out),
        ("CLKD1", SitePinDir::Out),
    ];
    vrf.verify_bel(
        bel,
        "PMCD",
        &pins,
        &[
            "CLKA_TEST",
            "CLKB_TEST",
            "CLKC_TEST",
            "CLKD_TEST",
            "REL_TEST",
            "CKINTA0",
            "CKINTA1",
            "CKINTA2",
            "CKINTA3",
            "CKINTB0",
            "CKINTB1",
            "CKINTB2",
            "CKINTB3",
            "CKINTC0",
            "CKINTC1",
            "CKINTC2",
            "CKINTC3",
            "REL_INT",
        ],
    );
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, "CCM");
    let obel_o = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "PMCD0" => "PMCD1",
            "PMCD1" => "PMCD0",
            _ => unreachable!(),
        },
    );
    for (opin, ab) in [
        ("CLKA", 'A'),
        ("CLKA_TEST", 'A'),
        ("CLKB", 'A'),
        ("CLKB_TEST", 'A'),
        ("CLKC", 'B'),
        ("CLKC_TEST", 'B'),
        ("CLKD", 'B'),
        ("CLKD_TEST", 'B'),
        ("REL", 'C'),
        ("REL_TEST", 'C'),
    ] {
        for i in 0..8 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel.wire(&format!("GCLK{i}")));
        }
        for i in 0..16 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel.wire(&format!("GIOB{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel.wire(&format!("MGT{i}")));
        }
        for i in 0..24 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel.wire(&format!("BUSIN{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(opin),
                bel.wire(&format!("CKINT{ab}{i}")),
            );
        }
        if ab != 'C' {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel_o.wire("CLKA1D8"));
        }
    }
    vrf.claim_pip(bel.crd(), bel.wire("REL"), bel.wire("REL_INT"));
}

fn verify_dpm(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("REFCLK", SitePinDir::In),
        ("TESTCLK1", SitePinDir::In),
        ("TESTCLK2", SitePinDir::In),
        ("REFCLKOUT", SitePinDir::Out),
        ("OSCOUT1", SitePinDir::Out),
        ("OSCOUT2", SitePinDir::Out),
    ];
    vrf.verify_bel(
        bel,
        "DPM",
        &pins,
        &[
            "REFCLK_TEST",
            "TESTCLK1_TEST",
            "TESTCLK2_TEST",
            "CKINTA0",
            "CKINTA1",
            "CKINTA2",
            "CKINTA3",
            "CKINTB0",
            "CKINTB1",
            "CKINTB2",
            "CKINTB3",
        ],
    );
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, "CCM");
    for (opin, ab) in [
        ("REFCLK", 'A'),
        ("REFCLK_TEST", 'A'),
        ("TESTCLK1", 'B'),
        ("TESTCLK1_TEST", 'B'),
        ("TESTCLK2", 'B'),
        ("TESTCLK2_TEST", 'B'),
    ] {
        for i in 0..8 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel.wire(&format!("GCLK{i}")));
        }
        for i in 0..16 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel.wire(&format!("GIOB{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel.wire(&format!("MGT{i}")));
        }
        for i in 0..24 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), obel.wire(&format!("BUSIN{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(opin),
                bel.wire(&format!("CKINT{ab}{i}")),
            );
        }
    }
}

fn verify_ccm(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_pmcd0 = vrf.find_bel_sibling(bel, "PMCD0");
    let obel_pmcd1 = vrf.find_bel_sibling(bel, "PMCD1");
    let obel_dpm = vrf.find_bel_sibling(bel, "DPM");
    for i in 0..12 {
        let opin = format!("TO_BUFG{i}");
        for (ibel, ipin) in [
            (&obel_pmcd0, "CLKA1"),
            (&obel_pmcd0, "CLKA1D2"),
            (&obel_pmcd0, "CLKA1D4"),
            (&obel_pmcd0, "CLKA1D8"),
            (&obel_pmcd0, "CLKB1"),
            (&obel_pmcd0, "CLKC1"),
            (&obel_pmcd0, "CLKD1"),
            (&obel_pmcd1, "CLKA1"),
            (&obel_pmcd1, "CLKA1D2"),
            (&obel_pmcd1, "CLKA1D4"),
            (&obel_pmcd1, "CLKA1D8"),
            (&obel_pmcd1, "CLKB1"),
            (&obel_pmcd1, "CLKC1"),
            (&obel_pmcd1, "CLKD1"),
            (&obel_dpm, "REFCLKOUT"),
            (&obel_dpm, "OSCOUT1"),
            (&obel_dpm, "OSCOUT2"),
            (bel, "CKINT"),
        ] {
            vrf.claim_pip(bel.crd(), bel.wire(&opin), ibel.wire(ipin));
        }
    }
    let dy = if bel.row < grid.row_dcmiob() { -4 } else { 4 };
    let obel = vrf.find_bel_walk(bel, 0, dy, "DCM").unwrap();
    for i in 0..24 {
        let opin = format!("BUSOUT{i}");
        let ipin = format!("BUSIN{i}");
        vrf.verify_node(&[bel.fwire(&ipin), obel.fwire(&opin)]);
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
    for i in 0..8 {
        vrf.verify_node(&[
            bel.fwire(&format!("GCLK{i}")),
            obel.fwire(&format!("GCLK_O_{ud}{i}")),
        ]);
    }
    if grid.has_mgt() {
        for i in 0..4 {
            vrf.verify_node(&[
                bel.fwire(&format!("MGT{i}")),
                obel.fwire(&format!("MGT_O_{ud}{i}")),
            ]);
        }
    }
    // XXX verify MGT
}

fn verify_sysmon(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    for (i, dy) in [(1, 0), (2, 1), (3, 2), (4, 3), (5, 5), (6, 6), (7, 7)] {
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
        let obel = vrf
            .find_bel(bel.die, (grid.cols_io[0], bel.row + dy), "IOB0")
            .unwrap();
        vrf.claim_node(&[bel.fwire_far(&format!("VP{i}")), obel.fwire("MONITOR")]);
        vrf.claim_pip(obel.crd(), obel.wire("MONITOR"), obel.wire("PADOUT"));
        let obel = vrf
            .find_bel(bel.die, (grid.cols_io[0], bel.row + dy), "IOB1")
            .unwrap();
        vrf.claim_node(&[bel.fwire_far(&format!("VN{i}")), obel.fwire("MONITOR")]);
        vrf.claim_pip(obel.crd(), obel.wire("MONITOR"), obel.wire("PADOUT"));
    }
}

fn verify_ipad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
}

fn verify_ilogic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("TFB", SitePinDir::In),
        ("OFB", SitePinDir::In),
        ("D", SitePinDir::In),
        ("CLK", SitePinDir::In),
        ("OCLK", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "ISERDES", &pins, &["CLKMUX", "CLKMUX_INT"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CLKMUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKMUX"), bel.wire("CLKMUX_INT"));
    let obel = vrf.find_bel_sibling(bel, "IOIS_CLK");
    for pin in [
        "GCLK0", "GCLK1", "GCLK2", "GCLK3", "GCLK4", "GCLK5", "GCLK6", "GCLK7", "RCLK0", "RCLK1",
        "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0", "IOCLK_N1",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire("CLKMUX"), obel.wire(pin));
    }
    let obel = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ILOGIC0" => "IOB0",
            "ILOGIC1" => "IOB1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("D"), obel.wire("I"));
    let obel = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ILOGIC0" => "OLOGIC0",
            "ILOGIC1" => "OLOGIC1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("OCLK"), obel.wire("CLKMUX"));
    vrf.claim_pip(bel.crd(), bel.wire("OFB"), obel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("TFB"), obel.wire("TQ"));
    if bel.key == "ILOGIC1" {
        let obel = vrf.find_bel_sibling(bel, "ILOGIC0");
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_ologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("OQ", SitePinDir::Out),
        ("CLK", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "OSERDES", &pins, &["CLKMUX", "CLKMUX_INT"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CLKMUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKMUX"), bel.wire("CLKMUX_INT"));
    let obel = vrf.find_bel_sibling(bel, "IOIS_CLK");
    for pin in [
        "GCLK0", "GCLK1", "GCLK2", "GCLK3", "GCLK4", "GCLK5", "GCLK6", "GCLK7", "RCLK0", "RCLK1",
        "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0", "IOCLK_N1",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire("CLKMUX"), obel.wire(pin));
    }
    if bel.key == "OLOGIC0" {
        let obel = vrf.find_bel_sibling(bel, "OLOGIC1");
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_iob(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.col == grid.cols_io[1] || matches!(bel.row.to_idx() % 16, 7 | 8) {
        "LOWCAPIOB"
    } else if bel.key == "IOB0" {
        "IOBM"
    } else {
        "IOBS"
    };
    let pins = [
        ("I", SitePinDir::Out),
        ("O", SitePinDir::In),
        ("T", SitePinDir::In),
        ("PADOUT", SitePinDir::Out),
        ("DIFFI_IN", SitePinDir::In),
        ("DIFFO_OUT", SitePinDir::Out),
        ("DIFFO_IN", SitePinDir::In),
    ];
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "IOB0" => "OLOGIC0",
            "IOB1" => "OLOGIC1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("O"), obel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("T"), obel.wire("TQ"));
    let obel = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "IOB0" => "IOB1",
            "IOB1" => "IOB0",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
    if kind == "IOBS" {
        vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
    }
}

fn verify_iois_clk(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let srow = RowId::from_idx(bel.row.to_idx() / 16 * 16 + 8);
    let obel = vrf.find_bel(bel.die, (bel.col, srow), "IOCLK").unwrap();
    for i in 0..8 {
        vrf.verify_node(&[
            bel.fwire(&format!("GCLK{i}")),
            obel.fwire(&format!("GCLK_OUT{i}")),
        ]);
    }
    for i in 0..2 {
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK{i}")),
            obel.fwire(&format!("RCLK_OUT{i}")),
        ]);
        vrf.verify_node(&[
            bel.fwire(&format!("IOCLK{i}")),
            obel.fwire(&format!("IOCLK{i}")),
        ]);
    }
    let mut do_s = true;
    let mut do_n = true;
    if bel.col == grid.cols_io[1] {
        if srow == grid.row_dcmiob() || srow == grid.row_cfg_above() {
            do_n = false;
        }
        if srow == grid.row_iobdcm() || srow == grid.row_cfg_below() {
            do_s = false;
        }
    }
    if do_s {
        for i in 0..2 {
            vrf.verify_node(&[
                bel.fwire(&format!("IOCLK_S{i}")),
                obel.fwire(&format!("IOCLK_S{i}")),
            ]);
        }
    }
    if do_n {
        for i in 0..2 {
            vrf.verify_node(&[
                bel.fwire(&format!("IOCLK_N{i}")),
                obel.fwire(&format!("IOCLK_N{i}")),
            ]);
        }
    }
    // XXX
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
        "PMV" | "STARTUP" | "FRAME_ECC" | "DCIRESET" | "CAPTURE" | "USR_ACCESS" | "DCI"
        | "GLOBALSIG" => vrf.verify_bel(bel, bel.key, &[], &[]),
        "JTAGPPC" => verify_jtagppc(vrf, bel),
        "BUFG_MGTCLK_B" | "BUFG_MGTCLK_T" => verify_bufg_mgtclk(grid, vrf, bel),
        "BUFG_MGTCLK_B_HROW" | "BUFG_MGTCLK_T_HROW" => verify_bufg_mgtclk_hrow(grid, vrf, bel),
        "BUFG_MGTCLK_B_HCLK" | "BUFG_MGTCLK_T_HCLK" => verify_bufg_mgtclk_hclk(grid, vrf, bel),

        "CLK_HROW" => verify_clk_hrow(grid, vrf, bel),
        "CLK_IOB" => verify_clk_iob(grid, vrf, bel),
        "CLK_DCM" => verify_clk_dcm(grid, vrf, bel),

        _ if bel.key.starts_with("BUFR") => verify_bufr(vrf, bel),
        _ if bel.key.starts_with("BUFIO") => verify_bufio(vrf, bel),
        "IDELAYCTRL" => verify_idelayctrl(vrf, bel),
        "RCLK" => verify_rclk(vrf, bel),
        "IOCLK" => verify_ioclk(grid, vrf, bel),
        "HCLK_DCM" | "HCLK_DCM_S" | "HCLK_DCM_N" => verify_hclk_dcm(grid, vrf, bel),
        "HCLK_DCM_HROW" => verify_hclk_dcm_hrow(grid, vrf, bel),
        "HCLK" => verify_hclk(grid, vrf, bel),

        _ if bel.key.starts_with("ILOGIC") => verify_ilogic(vrf, bel),
        _ if bel.key.starts_with("OLOGIC") => verify_ologic(vrf, bel),
        _ if bel.key.starts_with("IOB") => verify_iob(grid, vrf, bel),
        "IOIS_CLK" => verify_iois_clk(grid, vrf, bel),

        "DCM" => verify_dcm(grid, vrf, bel),
        "PMCD0" | "PMCD1" => verify_pmcd(vrf, bel),
        "DPM" => verify_dpm(vrf, bel),
        "CCM" => verify_ccm(grid, vrf, bel),
        "SYSMON" => verify_sysmon(grid, vrf, bel),
        _ if bel.key.starts_with("IPAD") => verify_ipad(vrf, bel),

        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}
