use prjcombine_entity::EntityId;
use prjcombine_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_virtex6::ExpandedDevice;

fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.bel.pins.contains_key("WE") {
        "SLICEM"
    } else {
        "SLICEL"
    };
    vrf.verify_bel(
        bel,
        kind,
        &[("CIN", SitePinDir::In), ("COUT", SitePinDir::Out)],
        &[],
    );
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.key) {
        vrf.claim_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
        vrf.claim_pip(obel.crd(), obel.wire_far("COUT"), obel.wire("COUT"));
    } else {
        vrf.claim_node(&[bel.fwire("CIN")]);
    }
    vrf.claim_node(&[bel.fwire("COUT")]);
}

fn verify_dsp(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    pairs.push(("MULTSIGNIN".to_string(), "MULTSIGNOUT".to_string()));
    pairs.push(("CARRYCASCIN".to_string(), "CARRYCASCOUT".to_string()));
    for i in 0..30 {
        pairs.push((format!("ACIN{i}"), format!("ACOUT{i}")));
    }
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
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "DSP1") {
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
    vrf.verify_bel(bel, "DSP48E1", &pins, &[]);
    let obel = vrf.find_bel_sibling(bel, "TIEOFF.DSP");
    for pin in [
        "ALUMODE2",
        "ALUMODE3",
        "CARRYINSEL2",
        "CEAD",
        "CEALUMODE",
        "CED",
        "CEINMODE",
        "INMODE0",
        "INMODE1",
        "INMODE2",
        "INMODE3",
        "INMODE4",
        "OPMODE6",
        "RSTD",
        "D0",
        "D1",
        "D2",
        "D3",
        "D4",
        "D5",
        "D6",
        "D7",
        "D8",
        "D9",
        "D10",
        "D11",
        "D12",
        "D13",
        "D14",
        "D15",
        "D16",
        "D17",
        "D18",
        "D19",
        "D20",
        "D21",
        "D22",
        "D23",
        "D24",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("HARD0"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("HARD1"));
    }
}

fn verify_tieoff(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "TIEOFF",
        &[("HARD0", SitePinDir::Out), ("HARD1", SitePinDir::Out)],
        &[],
    );
    for pin in ["HARD0", "HARD1"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_bram_f(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CASCADEINA", SitePinDir::In),
        ("CASCADEINB", SitePinDir::In),
        ("CASCADEOUTA", SitePinDir::Out),
        ("CASCADEOUTB", SitePinDir::Out),
        ("TSTOUT1", SitePinDir::Out),
        ("TSTOUT2", SitePinDir::Out),
        ("TSTOUT3", SitePinDir::Out),
        ("TSTOUT4", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "RAMBFIFO36E1", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bel.key) {
        for (ipin, opin) in [("CASCADEINA", "CASCADEOUTA"), ("CASCADEINB", "CASCADEOUTB")] {
            vrf.verify_node(&[bel.fwire(ipin), obel.fwire_far(opin)]);
            vrf.claim_pip(obel.crd(), obel.wire_far(opin), obel.wire(opin));
        }
    }
}

fn verify_bram_h1(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![
        "FULL".to_string(),
        "EMPTY".to_string(),
        "ALMOSTFULL".to_string(),
        "ALMOSTEMPTY".to_string(),
        "WRERR".to_string(),
        "RDERR".to_string(),
    ];
    for i in 0..12 {
        pins.push(format!("RDCOUNT{i}"));
        pins.push(format!("WRCOUNT{i}"));
    }
    let pin_refs: Vec<_> = pins.iter().map(|x| (&x[..], SitePinDir::Out)).collect();
    vrf.verify_bel(bel, "RAMB18E1", &pin_refs, &[]);
    for pin in pins {
        vrf.claim_node(&[bel.fwire(&pin)]);
    }
}

fn verify_hclk(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        for j in 0..12 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_D{i}")),
                bel.wire(&format!("HCLK{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_U{i}")),
                bel.wire(&format!("HCLK{j}")),
            );
        }
        for j in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_D{i}")),
                bel.wire(&format!("RCLK{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_U{i}")),
                bel.wire(&format!("RCLK{j}")),
            );
        }
    }
    let scol = if bel.col <= edev.grid.col_cfg {
        edev.grid.cols_qbuf.0
    } else {
        edev.grid.cols_qbuf.1
    };
    let obel = vrf.find_bel(bel.die, (scol, bel.row), "HCLK_QBUF").unwrap();
    for i in 0..12 {
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK{i}")),
            obel.fwire(&format!("HCLK{i}_O")),
        ]);
    }
    // regional clocks can be sourced from both inner and outer IO columns, but we consider inner
    // to be the source for simplicity.
    let scol = if bel.col <= edev.grid.col_cfg {
        edev.grid.cols_io[1].unwrap()
    } else {
        edev.grid.cols_io[2].unwrap()
    };
    let obel = vrf.find_bel(bel.die, (scol, bel.row), "HCLK_IOI").unwrap();
    for i in 0..6 {
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK{i}")),
            obel.fwire(&format!("RCLK{i}_I")),
        ]);
    }
}

fn verify_hclk_qbuf(_edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..12 {
        vrf.claim_node(&[bel.fwire(&format!("HCLK{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}_I")),
        );
        // XXX source
    }
}

fn verify_bufo(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFO",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_node(&[bel.fwire("VI")]);
    vrf.claim_node(&[bel.fwire("I_PRE")]);
    vrf.claim_node(&[bel.fwire("I_PRE2")]);

    vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire("I_PRE"));
    vrf.claim_pip(bel.crd(), bel.wire("VI"), bel.wire("I_PRE"));

    if let Some(obel) = vrf.find_bel_delta(bel, 0, 40, bel.key) {
        vrf.verify_node(&[bel.fwire("VI_S"), obel.fwire("VI")]);
        vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire("VI_S"));
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -40, bel.key) {
        vrf.verify_node(&[bel.fwire("VI_N"), obel.fwire("VI")]);
        vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire("VI_N"));
    }

    vrf.claim_pip(bel.crd(), bel.wire("I_PRE"), bel.wire("I_PRE2"));
    let idx = bel.bid.to_idx() % 2;
    let obel = vrf.find_bel_sibling(bel, "HCLK_IOI");
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I_PRE2"),
        obel.wire(&format!("PERF{ii}_BUF", ii = idx * 3)),
    );
}

fn verify_bufiodqs(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFIODQS",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
    let idx = bel.bid.to_idx() % 4;
    let obel = vrf.find_bel_sibling(bel, "HCLK_IOI");
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I"),
        obel.wire(&format!("IOCLK_IN{idx}")),
    );
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

    let obel = vrf.find_bel_sibling(bel, "HCLK_IOI");
    for i in 0..2 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I"),
            obel.wire(&format!("BUFR_CKINT{i}")),
        );
    }
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I"),
            obel.wire(&format!("IOCLK_IN{i}_BUFR")),
        );
    }
    for i in 0..10 {
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire(&format!("MGT{i}")));
    }
}

fn verify_idelayctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IDELAYCTRL", &[("REFCLK", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("REFCLK")]);
    let obel = vrf.find_bel_sibling(bel, "HCLK_IOI");
    for i in 0..12 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REFCLK"),
            obel.wire(&format!("HCLK{i}_O")),
        );
    }
}

fn verify_dci(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("DCIDATA", SitePinDir::Out),
        ("DCIADDRESS0", SitePinDir::Out),
        ("DCIADDRESS1", SitePinDir::Out),
        ("DCIADDRESS2", SitePinDir::Out),
        ("DCIIOUPDATE", SitePinDir::Out),
        ("DCIREFIOUPDATE", SitePinDir::Out),
        ("DCISCLK", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "DCI", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_hclk_ioi(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let scol = if bel.col <= edev.grid.col_cfg {
        edev.grid.cols_qbuf.0
    } else {
        edev.grid.cols_qbuf.1
    };
    let obel = vrf.find_bel(bel.die, (scol, bel.row), "HCLK_QBUF").unwrap();
    for i in 0..12 {
        vrf.claim_node(&[bel.fwire(&format!("HCLK{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}_I")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK{i}_I")),
            obel.fwire(&format!("HCLK{i}_O")),
        ]);
    }

    let scol = if bel.col <= edev.grid.col_cfg {
        edev.grid.cols_io[1].unwrap()
    } else {
        edev.grid.cols_io[2].unwrap()
    };
    if bel.col == scol {
        for i in 0..6 {
            vrf.claim_node(&[bel.fwire(&format!("RCLK{i}_I"))]);
        }
    } else {
        let obel = vrf.find_bel(bel.die, (scol, bel.row), "HCLK_IOI").unwrap();
        for i in 0..6 {
            vrf.verify_node(&[
                bel.fwire(&format!("RCLK{i}_I")),
                obel.fwire(&format!("RCLK{i}_I")),
            ]);
        }
    }
    for i in 0..6 {
        vrf.claim_node(&[bel.fwire(&format!("RCLK{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK{i}_O")),
            bel.wire(&format!("RCLK{i}_I")),
        );
        for j in 0..2 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCLK{i}_I")),
                bel.wire(&format!("VRCLK{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCLK{i}_I")),
                bel.wire(&format!("VRCLK{j}_S")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCLK{i}_I")),
                bel.wire(&format!("VRCLK{j}_N")),
            );
        }
    }

    let obel_s = vrf.find_bel_delta(bel, 0, 40, "HCLK_IOI");
    let obel_n = vrf.find_bel_delta(bel, 0, -40, "HCLK_IOI");
    for i in 0..2 {
        vrf.claim_node(&[bel.fwire(&format!("VRCLK{i}"))]);
        let obel = vrf.find_bel_sibling(bel, &format!("BUFR{i}"));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("VRCLK{i}")), obel.wire("O"));
        if let Some(ref obel) = obel_s {
            vrf.verify_node(&[
                bel.fwire(&format!("VRCLK{i}_S")),
                obel.fwire(&format!("VRCLK{i}")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("VRCLK{i}_S"))]);
        }
        if let Some(ref obel) = obel_n {
            vrf.verify_node(&[
                bel.fwire(&format!("VRCLK{i}_N")),
                obel.fwire(&format!("VRCLK{i}")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("VRCLK{i}_N"))]);
        }
    }

    for i in 0..4 {
        vrf.claim_node(&[bel.fwire(&format!("PERF{i}_BUF"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PERF{i}_BUF")),
            bel.wire(&format!("PERF{i}")),
        );

        vrf.claim_node(&[bel.fwire(&format!("IOCLK_IN{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}")),
            bel.wire(&format!("PERF{ii}_BUF", ii = i ^ 1)),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}")),
            bel.wire(&format!("IOCLK_PAD{i}")),
        );

        vrf.claim_node(&[bel.fwire(&format!("IOCLK_IN{i}_BUFR"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}_BUFR")),
            bel.wire(&format!("IOCLK_IN{i}")),
        );

        let dy = match i {
            0 => 0,
            1 => 2,
            2 => -4,
            3 => -2,
            _ => unreachable!(),
        };
        let obel = vrf.find_bel_delta(bel, 0, dy, "ILOGIC0").unwrap();
        vrf.verify_node(&[bel.fwire(&format!("IOCLK_PAD{i}")), obel.fwire("CLKOUT")]);
    }
    // XXX source PERF

    for (i, pin) in [
        (0, "IOCLK0_PRE"),
        (1, "IOCLK1_PRE"),
        (2, "IOCLK2_PRE"),
        (3, "IOCLK3_PRE"),
        (4, "IOCLK0_PRE_S"),
        (5, "IOCLK3_PRE_S"),
        (6, "IOCLK0_PRE_N"),
        (7, "IOCLK3_PRE_N"),
    ] {
        vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}_DLY"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK{i}")),
            bel.wire(&format!("IOCLK{i}_DLY")),
        );
        vrf.claim_pip(bel.crd(), bel.wire(&format!("IOCLK{i}")), bel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("IOCLK{i}_DLY")), bel.wire(pin));
    }
    for i in 0..4 {
        let obel = vrf.find_bel_sibling(bel, &format!("BUFIODQS{i}"));
        vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}_PRE"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK{i}_PRE")),
            obel.wire("O"),
        );
    }
    for i in [0, 3] {
        if let Some(ref obel) = obel_s {
            vrf.verify_node(&[
                bel.fwire(&format!("IOCLK{i}_PRE_S")),
                obel.fwire(&format!("IOCLK{i}_PRE")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}_PRE_S"))]);
        }
        if let Some(ref obel) = obel_n {
            vrf.verify_node(&[
                bel.fwire(&format!("IOCLK{i}_PRE_N")),
                obel.fwire(&format!("IOCLK{i}_PRE")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}_PRE_N"))]);
        }
    }

    for i in 0..2 {
        let obel = vrf.find_bel_sibling(bel, &format!("BUFO{i}"));
        vrf.claim_node(&[bel.fwire(&format!("OCLK{i}"))]);
        vrf.claim_pip(bel.crd(), bel.wire(&format!("OCLK{i}")), obel.wire("O"));
    }

    // XXX source MGT
}

fn verify_ilogic(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLK", SitePinDir::In),
        ("CLKB", SitePinDir::In),
        ("OCLK", SitePinDir::In),
        ("OCLKB", SitePinDir::In),
        ("D", SitePinDir::In),
        ("DDLY", SitePinDir::In),
        ("OFB", SitePinDir::In),
        ("TFB", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
        ("REV", SitePinDir::In),
    ];
    vrf.verify_bel(bel, "ILOGICE1", &pins, &["CKINT0", "CKINT1"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_ioi = vrf.find_bel_sibling(bel, "IOI");
    for pin in ["CLK", "CLKB"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKINT0"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKINT1"));
        for i in 0..12 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
        for i in 0..8 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_ioi.wire(&format!("IOCLK{i}")),
            );
        }
    }

    let obel_ologic = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ILOGIC0" => "OLOGIC0",
            "ILOGIC1" => "OLOGIC1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("OCLK"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("OCLKB"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("OCLKB"), obel_ologic.wire("CLKM"));
    vrf.claim_pip(bel.crd(), bel.wire("OFB"), obel_ologic.wire("OFB"));
    vrf.claim_pip(bel.crd(), bel.wire("TFB"), obel_ologic.wire("TFB_BUF"));

    let obel_iodelay = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ILOGIC0" => "IODELAY0",
            "ILOGIC1" => "IODELAY1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("DDLY"), obel_iodelay.wire("DATAOUT"));

    let obel_iob = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ILOGIC0" => "IOB0",
            "ILOGIC1" => "IOB1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("D"), bel.wire("IOB_I_BUF"));
    vrf.claim_node(&[bel.fwire("IOB_I_BUF")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOB_I_BUF"), bel.wire("IOB_I"));
    vrf.verify_node(&[bel.fwire("IOB_I"), obel_iob.fwire("I")]);

    if bel.key == "ILOGIC1" {
        let obel = vrf.find_bel_sibling(bel, "ILOGIC0");
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }

    let is_rclk = matches!(bel.row.to_idx() % 40, 16 | 18 | 20 | 22);
    let is_inner =
        bel.col == edev.grid.cols_io[1].unwrap() || bel.col == edev.grid.cols_io[2].unwrap();
    let is_gclk = is_inner
        && (bel.row == edev.grid.row_bufg() - 4
            || bel.row == edev.grid.row_bufg() - 2
            || bel.row == edev.grid.row_bufg()
            || bel.row == edev.grid.row_bufg() + 2);
    if (is_rclk || is_gclk) && bel.key == "ILOGIC0" {
        vrf.claim_node(&[bel.fwire("CLKOUT")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLKOUT"), bel.wire("O"));
        if is_inner {
            vrf.claim_node(&[bel.fwire("CLKOUT_GCLK")]);
            vrf.claim_pip(bel.crd(), bel.wire("CLKOUT_GCLK"), bel.wire("CLKOUT"));
        }
    }
}

fn verify_ologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLK", SitePinDir::In),
        ("CLKB", SitePinDir::In),
        ("CLKDIV", SitePinDir::In),
        ("CLKDIVB", SitePinDir::In),
        ("CLKPERF", SitePinDir::In),
        ("CLKPERFDELAY", SitePinDir::In),
        ("OFB", SitePinDir::Out),
        ("TFB", SitePinDir::Out),
        ("OQ", SitePinDir::Out),
        ("TQ", SitePinDir::Out),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
        ("REV", SitePinDir::In),
    ];
    vrf.verify_bel(
        bel,
        "OLOGICE1",
        &pins,
        &["CLK_CKINT", "CLKDIV_CKINT", "CLK_MUX", "TFB_BUF", "CLKDIV"],
    );
    for (pin, _) in pins {
        if pin == "CLKDIV" {
            continue;
        }
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKB"), bel.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKB"), bel.wire("CLKM"));

    let obel_ioi = vrf.find_bel_sibling(bel, "IOI");
    vrf.claim_node(&[bel.fwire("CLKM")]);
    for pin in ["CLK_MUX", "CLKM"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CLK_CKINT"));
        for i in 0..12 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
        for i in 0..8 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_ioi.wire(&format!("IOCLK{i}")),
            );
        }
    }

    for pin in ["CLKDIV", "CLKDIVB"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CLKDIV_CKINT"));
        for i in 0..12 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
    }

    for i in 0..2 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKPERF"),
            obel_ioi.wire(&format!("OCLK{i}")),
        );
    }

    let obel_iodelay = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "OLOGIC0" => "IODELAY0",
            "OLOGIC1" => "IODELAY1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKPERFDELAY"),
        obel_iodelay.wire("DATAOUT"),
    );

    vrf.claim_pip(bel.crd(), bel.wire("TFB_BUF"), bel.wire("TFB"));

    let obel_iob = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "OLOGIC0" => "IOB0",
            "OLOGIC1" => "IOB1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("IOB_T"), bel.wire("TQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), bel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), obel_iodelay.wire("DATAOUT"));
    vrf.verify_node(&[bel.fwire("IOB_O"), obel_iob.fwire("O")]);
    vrf.verify_node(&[bel.fwire("IOB_T"), obel_iob.fwire("T")]);

    if bel.key == "OLOGIC0" {
        let obel = vrf.find_bel_sibling(bel, "OLOGIC1");
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_iodelay(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLKIN", SitePinDir::In),
        ("IDATAIN", SitePinDir::In),
        ("ODATAIN", SitePinDir::In),
        ("DATAOUT", SitePinDir::Out),
        ("T", SitePinDir::In),
    ];
    vrf.verify_bel(bel, "IODELAYE1", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_ilogic = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "IODELAY0" => "ILOGIC0",
            "IODELAY1" => "ILOGIC1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("IDATAIN"),
        obel_ilogic.wire("IOB_I_BUF"),
    );

    let obel_ologic = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "IODELAY0" => "OLOGIC0",
            "IODELAY1" => "OLOGIC1",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("ODATAIN"), obel_ologic.wire("OFB"));
    vrf.claim_pip(bel.crd(), bel.wire("T"), obel_ologic.wire("TFB"));
}

fn verify_iob(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = match bel.key {
        "IOB0" => "IOBM",
        "IOB1" => "IOBS",
        _ => unreachable!(),
    };
    let mut pins = vec![
        ("I", SitePinDir::Out),
        ("O", SitePinDir::In),
        ("T", SitePinDir::In),
        ("O_IN", SitePinDir::In),
        ("O_OUT", SitePinDir::Out),
        ("DIFFO_IN", SitePinDir::In),
        ("DIFFO_OUT", SitePinDir::Out),
        ("DIFFI_IN", SitePinDir::In),
        ("PADOUT", SitePinDir::Out),
    ];
    if kind == "IOBM" {
        pins.push(("DIFF_TERM_INT_EN", SitePinDir::In));
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let okey = match bel.key {
        "IOB0" => "IOB1",
        "IOB1" => "IOB0",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    if kind == "IOBS" {
        vrf.claim_pip(bel.crd(), bel.wire("O_IN"), obel.wire("O_OUT"));
        vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
    }
    vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
}

fn verify_ioi(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let srow = edev.grid.row_hclk(bel.row);
    let obel = vrf.find_bel(bel.die, (bel.col, srow), "HCLK_IOI").unwrap();
    for i in 0..12 {
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK{i}")),
            obel.fwire(&format!("HCLK{i}_O")),
        ]);
    }
    for i in 0..6 {
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK{i}")),
            obel.fwire(&format!("RCLK{i}_O")),
        ]);
    }
    for i in 0..8 {
        vrf.verify_node(&[
            bel.fwire(&format!("IOCLK{i}")),
            obel.fwire(&format!("IOCLK{i}")),
        ]);
    }
    for i in 0..2 {
        vrf.verify_node(&[
            bel.fwire(&format!("OCLK{i}")),
            obel.fwire(&format!("OCLK{i}")),
        ]);
    }
}

fn verify_sysmon(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    for i in 0..16 {
        pins.push(format!("VAUXP{i}"));
        pins.push(format!("VAUXN{i}"));
    }
    pins.push("VP".to_string());
    pins.push("VN".to_string());
    let mut pin_refs = vec![];
    for pin in &pins {
        pin_refs.push((&pin[..], SitePinDir::In));
    }
    vrf.verify_bel(bel, "SYSMON", &pin_refs, &[]);

    vrf.claim_node(&[bel.fwire("VP")]);
    let obel = vrf.find_bel_sibling(bel, "IPAD.VP");
    vrf.claim_pip(bel.crd(), bel.wire("VP"), obel.wire("O"));
    vrf.claim_node(&[bel.fwire("VN")]);
    let obel = vrf.find_bel_sibling(bel, "IPAD.VN");
    vrf.claim_pip(bel.crd(), bel.wire("VN"), obel.wire("O"));

    let cl = edev.grid.cols_io[0].unwrap_or_else(|| edev.grid.cols_io[1].unwrap());
    let cr = edev.grid.cols_io[2].unwrap();

    for (i, (col, dy)) in [
        (cr, 34),
        (cr, 32),
        (cr, 28),
        (cr, 26),
        (cr, 24),
        (cr, 14),
        (cr, 12),
        (cr, 8),
        (cl, 34),
        (cl, 32),
        (cl, 28),
        (cl, 26),
        (cl, 24),
        (cl, 14),
        (cl, 12),
        (cl, 8),
    ]
    .into_iter()
    .enumerate()
    {
        let vauxp = format!("VAUXP{i}");
        let vauxn = format!("VAUXN{i}");
        vrf.claim_node(&[bel.fwire(&vauxp)]);
        vrf.claim_node(&[bel.fwire(&vauxn)]);
        vrf.claim_pip(bel.crd(), bel.wire(&vauxp), bel.wire_far(&vauxp));
        vrf.claim_pip(bel.crd(), bel.wire(&vauxn), bel.wire_far(&vauxn));
        let srow = bel.row + dy;
        let obel = vrf.find_bel(bel.die, (col, srow), "IOB0").unwrap();
        vrf.claim_node(&[bel.fwire_far(&vauxp), obel.fwire("MONITOR")]);
        vrf.claim_pip(obel.crd(), obel.wire("MONITOR"), obel.wire("PADOUT"));
        let obel = vrf.find_bel(bel.die, (col, srow), "IOB1").unwrap();
        vrf.claim_node(&[bel.fwire_far(&vauxn), obel.fwire("MONITOR")]);
        vrf.claim_pip(obel.crd(), obel.wire("MONITOR"), obel.wire("PADOUT"));
    }
}

fn verify_ipad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
}

fn verify_opad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "OPAD", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
}

pub fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("SLICE") => verify_slice(vrf, bel),
        _ if bel.key.starts_with("DSP") => verify_dsp(vrf, bel),
        _ if bel.key.starts_with("TIEOFF") => verify_tieoff(vrf, bel),
        "BRAM_F" => verify_bram_f(vrf, bel),
        "BRAM_H0" => vrf.verify_bel(bel, "FIFO18E1", &[], &[]),
        "BRAM_H1" => verify_bram_h1(vrf, bel),
        "PMVBRAM" => vrf.verify_bel(bel, "PMVBRAM", &[], &[]),
        "EMAC" => vrf.verify_bel(bel, "TEMAC_SINGLE", &[], &[]),
        "PCIE" => vrf.verify_bel(bel, "PCIE_2_0", &[], &[]),

        "HCLK" => verify_hclk(edev, vrf, bel),
        "HCLK_QBUF" => verify_hclk_qbuf(edev, vrf, bel),

        _ if bel.key.starts_with("BUFIODQS") => verify_bufiodqs(vrf, bel),
        _ if bel.key.starts_with("BUFO") => verify_bufo(vrf, bel),
        _ if bel.key.starts_with("BUFR") => verify_bufr(vrf, bel),
        "IDELAYCTRL" => verify_idelayctrl(vrf, bel),
        "DCI" => verify_dci(vrf, bel),
        "HCLK_IOI" => verify_hclk_ioi(edev, vrf, bel),

        _ if bel.key.starts_with("ILOGIC") => verify_ilogic(edev, vrf, bel),
        _ if bel.key.starts_with("OLOGIC") => verify_ologic(vrf, bel),
        _ if bel.key.starts_with("IODELAY") => verify_iodelay(vrf, bel),
        _ if bel.key.starts_with("IOB") => verify_iob(vrf, bel),
        "IOI" => verify_ioi(edev, vrf, bel),

        _ if bel.key.starts_with("BSCAN") => vrf.verify_bel(bel, "BSCAN", &[], &[]),
        _ if bel.key.starts_with("ICAP") => vrf.verify_bel(bel, "ICAP", &[], &[]),
        _ if bel.key.starts_with("PMV") => vrf.verify_bel(bel, "PMV", &[], &[]),
        "STARTUP" | "CAPTURE" | "FRAME_ECC" | "EFUSE_USR" | "USR_ACCESS" | "DNA_PORT"
        | "DCIRESET" | "CFG_IO_ACCESS" => vrf.verify_bel(bel, bel.key, &[], &[]),
        "SYSMON" => verify_sysmon(edev, vrf, bel),
        _ if bel.key.starts_with("IPAD") => verify_ipad(vrf, bel),
        _ if bel.key.starts_with("OPAD") => verify_opad(vrf, bel),

        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}
