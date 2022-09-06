use prjcombine_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_spartan6::Grid;

fn verify_sliceml(vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    if let Some(obel) = vrf.find_bel_walk(bel, 0, -1, "SLICE0") {
        vrf.claim_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
        vrf.claim_pip(obel.crd(), obel.wire_far("COUT"), obel.wire("COUT"));
    } else {
        vrf.claim_node(&[bel.fwire("CIN")]);
    }
    vrf.claim_node(&[bel.fwire("COUT")]);
}

fn verify_dsp(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let carry: Vec<_> = (0..18)
        .map(|x| (format!("BCOUT{x}"), format!("BCIN{x}")))
        .chain((0..48).map(|x| (format!("PCOUT{x}"), format!("PCIN{x}"))))
        .chain([("CARRYOUT".to_string(), "CARRYIN".to_string())].into_iter())
        .collect();
    let mut pins = vec![];
    for (o, i) in &carry {
        pins.push((&**o, SitePinDir::Out));
        pins.push((&**i, SitePinDir::In));
    }
    vrf.verify_bel(bel, "DSP48A1", &pins, &[]);
    for (o, i) in &carry {
        vrf.claim_node(&[bel.fwire(o)]);
        vrf.claim_node(&[bel.fwire(i)]);
    }
    if let Some(obel) = vrf.find_bel_walk(bel, 0, -4, "DSP") {
        for (o, i) in &carry {
            vrf.verify_node(&[bel.fwire(i), obel.fwire_far(o)]);
            vrf.claim_pip(obel.crd(), obel.wire_far(o), obel.wire(o));
        }
    }
}

fn verify_ilogic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![
        ("TFB", SitePinDir::In),
        ("OFB", SitePinDir::In),
        ("D", SitePinDir::In),
        ("DDLY", SitePinDir::In),
        ("DDLY2", SitePinDir::In),
        ("CLK0", SitePinDir::In),
        ("CLK1", SitePinDir::In),
        ("IOCE", SitePinDir::In),
        ("SHIFTIN", SitePinDir::In),
        ("SHIFTOUT", SitePinDir::Out),
        ("DFB", SitePinDir::Out),
        ("CFB0", SitePinDir::Out),
        ("CFB1", SitePinDir::Out),
        ("SR", SitePinDir::In),
    ];
    if bel.key == "ILOGIC1" {
        pins.extend([("INCDEC", SitePinDir::Out), ("VALID", SitePinDir::Out)]);
    }
    vrf.verify_bel(bel, "ILOGIC2", &pins, &["SR_INT"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let okey = match bel.key {
        "ILOGIC0" => "OLOGIC0",
        "ILOGIC1" => "OLOGIC1",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    vrf.claim_pip(bel.crd(), bel.wire("SR"), bel.wire("SR_INT"));
    vrf.claim_pip(bel.crd(), bel.wire("SR"), obel.wire_far("SR"));
    vrf.claim_pip(bel.crd(), bel.wire("OFB"), obel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("TFB"), obel.wire("TQ"));

    let okey = match bel.key {
        "ILOGIC0" => "IODELAY0",
        "ILOGIC1" => "IODELAY1",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    vrf.claim_pip(bel.crd(), bel.wire("DDLY"), obel.wire("DATAOUT"));
    vrf.claim_pip(bel.crd(), bel.wire("DDLY2"), obel.wire("DATAOUT2"));

    let okey = match bel.key {
        "ILOGIC0" => "IOICLK0",
        "ILOGIC1" => "IOICLK1",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    let obel_tie = vrf.find_bel_sibling(bel, "TIEOFF");
    vrf.claim_pip(bel.crd(), bel.wire("CLK0"), obel.wire("CLK0_ILOGIC"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), obel.wire("CLK1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE"), obel_tie.wire("HARD1"));

    vrf.claim_node(&[bel.fwire("D_MUX")]);
    vrf.claim_pip(bel.crd(), bel.wire("D"), bel.wire("D_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("D_MUX"), bel.wire("IOB_I"));

    let okey = match bel.key {
        "ILOGIC0" => "IOB0",
        "ILOGIC1" => "IOB1",
        _ => unreachable!(),
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 0, okey) {
        vrf.verify_node(&[bel.fwire("IOB_I"), obel.fwire_far("I")]);

        vrf.claim_pip(bel.crd(), bel.wire("MCB_FABRICOUT"), bel.wire("FABRICOUT"));
    } else {
        vrf.claim_node(&[bel.fwire("IOB_I")]);
    }

    let okey = match bel.key {
        "ILOGIC0" => "ILOGIC1",
        "ILOGIC1" => "ILOGIC0",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN"), obel.wire("SHIFTOUT"));
    if bel.key == "ILOGIC1" {
        vrf.claim_pip(bel.crd(), bel.wire("D_MUX"), obel.wire("IOB_I"));
    }
}

fn verify_ologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLK0", SitePinDir::In),
        ("CLK1", SitePinDir::In),
        ("IOCE", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTIN3", SitePinDir::In),
        ("SHIFTIN4", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
        ("SHIFTOUT3", SitePinDir::Out),
        ("SHIFTOUT4", SitePinDir::Out),
        ("OQ", SitePinDir::Out),
        ("TQ", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "OLOGIC2", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let okey = match bel.key {
        "OLOGIC0" => "IOICLK0",
        "OLOGIC1" => "IOICLK1",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    let obel_tie = vrf.find_bel_sibling(bel, "TIEOFF");
    vrf.claim_pip(bel.crd(), bel.wire("CLK0"), obel.wire("CLK0_OLOGIC"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), obel.wire("CLK1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE"), obel_tie.wire("HARD1"));

    let obel_ioi = vrf.find_bel_sibling(bel, "IOI");
    vrf.claim_pip(bel.crd(), bel.wire("OCE"), obel_ioi.wire("PCI_CE"));
    vrf.claim_pip(bel.crd(), bel.wire("REV"), obel_tie.wire("HARD0"));
    vrf.claim_pip(bel.crd(), bel.wire("SR"), obel_tie.wire("HARD0"));
    vrf.claim_pip(bel.crd(), bel.wire("TRAIN"), obel_tie.wire("HARD0"));

    let okey = match bel.key {
        "OLOGIC0" => "IODELAY0",
        "OLOGIC1" => "IODELAY1",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), bel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), obel.wire("DOUT"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_T"), bel.wire("TQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_T"), obel.wire("TOUT"));

    let okey = match bel.key {
        "OLOGIC0" => "IOB0",
        "OLOGIC1" => "IOB1",
        _ => unreachable!(),
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 0, okey) {
        vrf.verify_node(&[bel.fwire("IOB_O"), obel.fwire_far("O")]);
        vrf.verify_node(&[bel.fwire("IOB_T"), obel.fwire_far("T")]);

        vrf.claim_pip(bel.crd(), bel.wire("D1"), bel.wire("MCB_D1"));
        vrf.claim_pip(bel.crd(), bel.wire("D2"), bel.wire("MCB_D2"));
        if bel.key == "OLOGIC0" {
            vrf.claim_pip(bel.crd(), bel.wire("T2"), bel.wire("MCB_T"));
        } else {
            vrf.claim_pip(bel.crd(), bel.wire("T1"), bel.wire("MCB_T"));
        }
        vrf.claim_pip(bel.crd(), bel.wire("TRAIN"), obel_ioi.wire("MCB_DRPTRAIN"));
    } else {
        vrf.claim_node(&[bel.fwire("IOB_T")]);
        vrf.claim_node(&[bel.fwire("IOB_O")]);
    }

    let okey = match bel.key {
        "OLOGIC0" => "OLOGIC1",
        "OLOGIC1" => "OLOGIC0",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    if bel.key == "OLOGIC0" {
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN3"), obel.wire("SHIFTOUT3"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN4"), obel.wire("SHIFTOUT4"));
    } else {
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_iodelay(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("IOCLK0", SitePinDir::In),
        ("IOCLK1", SitePinDir::In),
        ("IDATAIN", SitePinDir::In),
        ("ODATAIN", SitePinDir::In),
        ("T", SitePinDir::In),
        ("DOUT", SitePinDir::Out),
        ("TOUT", SitePinDir::Out),
        ("DATAOUT", SitePinDir::Out),
        ("DATAOUT2", SitePinDir::Out),
        ("DQSOUTP", SitePinDir::Out),
        ("DQSOUTN", SitePinDir::Out),
        ("AUXSDO", SitePinDir::Out),
        ("AUXSDOIN", SitePinDir::In),
        ("AUXADDR0", SitePinDir::In),
        ("AUXADDR1", SitePinDir::In),
        ("AUXADDR2", SitePinDir::In),
        ("AUXADDR3", SitePinDir::In),
        ("AUXADDR4", SitePinDir::In),
        ("READEN", SitePinDir::In),
        ("MEMUPDATE", SitePinDir::In),
    ];
    vrf.verify_bel(bel, "IODELAY2", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let okey = match bel.key {
        "IODELAY0" => "IOICLK0",
        "IODELAY1" => "IOICLK1",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK0"), obel.wire("CLK0_ILOGIC"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK0"), obel.wire("CLK0_OLOGIC"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK1"), obel.wire("CLK1"));

    let okey = match bel.key {
        "IODELAY0" => "ILOGIC0",
        "IODELAY1" => "ILOGIC1",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    vrf.claim_pip(bel.crd(), bel.wire("IDATAIN"), obel.wire("D_MUX"));

    let okey = match bel.key {
        "IODELAY0" => "OLOGIC0",
        "IODELAY1" => "OLOGIC1",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    vrf.claim_pip(bel.crd(), bel.wire("ODATAIN"), obel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("T"), obel.wire("TQ"));

    let obel_ioi = vrf.find_bel_sibling(bel, "IOI");
    let okey = match bel.key {
        "IODELAY0" => "IOB0",
        "IODELAY1" => "IOB1",
        _ => unreachable!(),
    };
    if vrf.find_bel_delta(bel, 0, 0, okey).is_some() {
        vrf.claim_pip(bel.crd(), bel.wire("MCB_DQSOUTP"), bel.wire("DQSOUTP"));
        vrf.claim_pip(bel.crd(), bel.wire("CAL"), obel_ioi.wire("MCB_DRPADD"));
        vrf.claim_pip(bel.crd(), bel.wire("CE"), obel_ioi.wire("MCB_DRPSDO"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), obel_ioi.wire("MCB_DRPCLK"));
        vrf.claim_pip(bel.crd(), bel.wire("INC"), obel_ioi.wire("MCB_DRPCS"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("RST"),
            obel_ioi.wire("MCB_DRPBROADCAST"),
        );
    }

    // XXX AUX*, MEMUPDATE [LR only!]
}

fn verify_ioiclk(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, "IOI");
    vrf.claim_node(&[bel.fwire("CLK0INTER")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), obel.wire("IOCLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), obel.wire("IOCLK2"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), obel.wire("PLLCLK0"));
    vrf.claim_node(&[bel.fwire("CLK1INTER")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), obel.wire("IOCLK1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), obel.wire("IOCLK3"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), obel.wire("PLLCLK1"));
    vrf.claim_node(&[bel.fwire("CLK2INTER")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK2INTER"), obel.wire("PLLCLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK2INTER"), obel.wire("PLLCLK1"));
    vrf.claim_node(&[bel.fwire("CLK0_ILOGIC")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_ILOGIC"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_ILOGIC"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_ILOGIC"), bel.wire("CLK2INTER"));
    vrf.claim_node(&[bel.fwire("CLK0_OLOGIC")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_OLOGIC"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_OLOGIC"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_OLOGIC"), bel.wire("CLK2INTER"));
    vrf.claim_node(&[bel.fwire("CLK1")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), bel.wire("CLK2INTER"));
    vrf.claim_node(&[bel.fwire("IOCE0")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("IOCE2"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("IOCE3"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("PLLCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("PLLCE1"));
    vrf.claim_node(&[bel.fwire("IOCE1")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("IOCE2"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("IOCE3"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("PLLCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("PLLCE1"));
}

fn verify_ioi(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    // XXX source MCB stuff, incl. I/O LOGIC
    // XXX source PCI_CE
    // XXX source IOCLK/IOCE/PLLCLK/PLLCE
}

fn verify_iob(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![
        ("I", SitePinDir::Out),
        ("O", SitePinDir::In),
        ("T", SitePinDir::In),
        ("PCI_RDY", SitePinDir::Out),
        ("PADOUT", SitePinDir::Out),
        ("DIFFI_IN", SitePinDir::In),
        ("DIFFO_OUT", SitePinDir::Out),
        ("DIFFO_IN", SitePinDir::In),
    ];
    let kind = match bel.key {
        "IOB0" => "IOBM",
        "IOB1" => "IOBS",
        _ => unreachable!(),
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    vrf.claim_node(&[bel.fwire_far("I")]);
    vrf.claim_node(&[bel.fwire_far("O")]);
    vrf.claim_node(&[bel.fwire_far("T")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire_far("O"));
    vrf.claim_pip(bel.crd(), bel.wire("T"), bel.wire_far("T"));
    vrf.claim_pip(bel.crd(), bel.wire_far("I"), bel.wire("I"));

    let okey = match bel.key {
        "IOB0" => "IOB1",
        "IOB1" => "IOB0",
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, okey);
    vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
    if bel.key == "IOB1" {
        vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
    }
}

fn verify_tieoff(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "TIEOFF",
        &[
            ("HARD0", SitePinDir::Out),
            ("HARD1", SitePinDir::Out),
            ("KEEP1", SitePinDir::Out),
        ],
        &[],
    );
    for pin in ["HARD0", "HARD1", "KEEP1"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_pcilogicse(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "PCILOGICSE",
        &[
            ("PCI_CE", SitePinDir::Out),
            ("IRDY", SitePinDir::In),
            ("TRDY", SitePinDir::In),
        ],
        &[],
    );
    let pip = &bel.naming.pins["PCI_CE"].pips[0];
    vrf.claim_node(&[bel.fwire("PCI_CE"), (bel.crds[pip.tile], &pip.wire_from)]);
    vrf.claim_node(&[(bel.crds[pip.tile], &pip.wire_to)]);
    vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
    let rdy = if bel.col == grid.col_lio() {
        [
            ("IRDY", 2, "IOB1"),
            ("TRDY", -1, "IOB0"),
        ]
    } else {
        [
            ("IRDY", 2, "IOB0"),
            ("TRDY", -1, "IOB1"),
        ]
    };
    for (pin, dy, key) in rdy {
        let pip = &bel.naming.pins[pin].pips[0];
        vrf.claim_node(&[
            bel.fwire(pin),
            (bel.crds[pip.tile], &pip.wire_to),
        ]);
        vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
        let obel = vrf.find_bel_delta(bel, 0, dy, key).unwrap();
        vrf.claim_node(&[
            (bel.crds[pip.tile], &pip.wire_from),
            obel.fwire_far("PCI_RDY"),
        ]);
        vrf.claim_pip(obel.crd(), obel.wire_far("PCI_RDY"), obel.wire("PCI_RDY"));
    }
}

pub fn verify_bel(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        "SLICE0" => verify_sliceml(vrf, bel),
        "SLICE1" => vrf.verify_bel(bel, "SLICEX", &[], &[]),
        "BRAM_F" => vrf.verify_bel(bel, "RAMB16BWER", &[], &[]),
        _ if bel.key.starts_with("BRAM_H") => vrf.verify_bel(bel, "RAMB8BWER", &[], &[]),
        "DSP" => verify_dsp(vrf, bel),
        "PCIE" => vrf.verify_bel(bel, "PCIE_A1", &[], &[]),

        _ if bel.key.starts_with("OCT_CAL") => vrf.verify_bel(bel, "OCT_CALIBRATE", &[], &[]),
        _ if bel.key.starts_with("BSCAN") => vrf.verify_bel(bel, "BSCAN", &[], &[]),
        "PMV" | "DNA_PORT" | "ICAP" | "SPI_ACCESS" | "SUSPEND_SYNC" | "POST_CRC_INTERNAL"
        | "STARTUP" | "SLAVE_SPI" => vrf.verify_bel(bel, bel.key, &[], &[]),

        "ILOGIC0" | "ILOGIC1" => verify_ilogic(vrf, bel),
        "OLOGIC0" | "OLOGIC1" => verify_ologic(vrf, bel),
        "IODELAY0" | "IODELAY1" => verify_iodelay(grid, vrf, bel),
        "IOICLK0" | "IOICLK1" => verify_ioiclk(vrf, bel),
        "IOI" => verify_ioi(grid, vrf, bel),
        "IOB0" | "IOB1" => verify_iob(vrf, bel),
        "TIEOFF" => verify_tieoff(vrf, bel),
        "PCILOGICSE" => verify_pcilogicse(grid, vrf, bel),

        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}
