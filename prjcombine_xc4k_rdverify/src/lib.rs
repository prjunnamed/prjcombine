use prjcombine_rawdump::Part;
use prjcombine_rdverify::{verify, BelContext, SitePinDir, Verifier};
use prjcombine_xc4k::{expanded::ExpandedDevice, grid::GridKind};

fn verify_clb(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "CLB",
        &[("COUT", SitePinDir::Out), ("CIN", SitePinDir::In)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("COUT")]);
    if !edev.grid.kind.is_clb_xl() {
        vrf.claim_pip(bel.crd(), bel.wire("CIN.B"), bel.wire("COUT"));
        vrf.claim_pip(bel.crd(), bel.wire("CIN.T"), bel.wire("COUT"));
        vrf.claim_node(&[bel.fwire("CIN")]);
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, "CLB") {
            vrf.verify_node(&[bel.fwire("CIN.B"), obel.fwire("CIN")]);
        } else if let Some(obel) = vrf.find_bel_delta(bel, 1, 0, "CLB") {
            vrf.verify_node(&[bel.fwire("CIN.B"), obel.fwire("CIN")]);
        } else {
            let obel = vrf.find_bel_delta(bel, 1, -1, "COUT.LR").unwrap();
            vrf.verify_node(&[bel.fwire("CIN.B"), obel.fwire("I")]);
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, "CLB") {
            vrf.verify_node(&[bel.fwire("CIN.T"), obel.fwire("CIN")]);
        } else if let Some(obel) = vrf.find_bel_delta(bel, 1, 0, "CLB") {
            vrf.verify_node(&[bel.fwire("CIN.T"), obel.fwire("CIN")]);
        } else {
            let obel = vrf.find_bel_delta(bel, 1, 1, "COUT.UR").unwrap();
            vrf.verify_node(&[bel.fwire("CIN.T"), obel.fwire("I")]);
        }
    } else {
        vrf.claim_pip(bel.crd(), bel.wire_far("COUT"), bel.wire("COUT"));
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, "CLB") {
            vrf.verify_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
        } else {
            let obel = vrf.find_bel_delta(bel, 0, -1, "BOT_CIN").unwrap();
            vrf.verify_node(&[bel.fwire("CIN"), obel.fwire("CIN")]);
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, "TOP_COUT") {
            vrf.verify_node(&[bel.fwire_far("COUT"), obel.fwire("COUT")]);
        } else {
            vrf.claim_node(&[bel.fwire_far("COUT")]);
        }
    }
}
fn verify_iob(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    let kind = if !bel.bel.pins.contains_key("I1") {
        "FCLKIOB"
    } else if bel.bel.pins.contains_key("CLKIN") {
        "CLKIOB"
    } else if bel.naming.pins.contains_key("CLKIN") {
        pins.push(("CLKIN", SitePinDir::Out));
        vrf.claim_node(&[bel.fwire("CLKIN")]);
        "FCLKIOB"
    } else {
        "IOB"
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
}

fn verify_bufg(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "BUFG", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
}

fn verify_bufge(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_bufg = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "BUFGE.H" => "BUFG.H",
            "BUFGE.V" => "BUFG.V",
            _ => unreachable!(),
        },
    );
    vrf.verify_bel(bel, "BUFGE", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_pip(bel.crd(), bel.wire("I"), obel_bufg.wire("O"));
}

fn verify_bufgls(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if !edev.grid.kind.is_xl() {
        let kind = if bel.name.unwrap().starts_with("BUFGP") {
            "PRI-CLK"
        } else if bel.name.unwrap().starts_with("BUFGS") {
            "SEC-CLK"
        } else {
            "BUFGLS"
        };
        vrf.verify_bel(bel, kind, &[("O", SitePinDir::Out)], &[]);
        vrf.claim_node(&[bel.fwire("O")]);
    } else {
        let obel_bufg = vrf.find_bel_sibling(
            bel,
            match bel.key {
                "BUFGLS.H" => "BUFG.H",
                "BUFGLS.V" => "BUFG.V",
                _ => unreachable!(),
            },
        );
        vrf.verify_bel(
            bel,
            "BUFGLS",
            &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
            &[],
        );
        vrf.claim_node(&[bel.fwire("I")]);
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel_bufg.wire("O"));
        vrf.claim_node(&[bel.fwire("O")]);
        vrf.claim_node(&[bel.fwire_far("O")]);
        vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
    }
}

fn verify_cout(vrf: &mut Verifier, bel: &BelContext) {
    vrf.verify_bel(bel, "COUT", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
}

fn verify_cin(vrf: &mut Verifier, bel: &BelContext) {
    vrf.verify_bel(bel, "CIN", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
    let obel = match bel.key {
        "CIN.LL" => vrf.find_bel_delta(bel, 1, 1, "CLB").unwrap(),
        "CIN.UL" => vrf.find_bel_delta(bel, 1, -1, "CLB").unwrap(),
        _ => unreachable!(),
    };
    vrf.verify_node(&[bel.fwire_far("O"), obel.fwire("CIN")]);
}

fn verify_tbuf_splitter(vrf: &mut Verifier, bel: &BelContext) {
    for (po, pi) in [
        ("L", "R"),
        ("R", "L"),
        ("L.EXCL", "L"),
        ("L", "L.EXCL"),
        ("R.EXCL", "R"),
        ("R", "R.EXCL"),
        ("L.EXCL", "R.EXCL"),
        ("R.EXCL", "L.EXCL"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(po), bel.wire(pi));
    }
    vrf.claim_node(&[bel.fwire("L.EXCL")]);
    vrf.claim_node(&[bel.fwire("R.EXCL")]);
}

fn verify_clkh(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext) {
    if edev.grid.kind == GridKind::SpartanXl {
        for opin in ["O0", "O1", "O2", "O3"] {
            for ipin in [
                "I.LL.H", "I.LL.V", "I.UL.H", "I.UL.V", "I.LR.H", "I.LR.V", "I.UR.H", "I.UR.V",
            ] {
                vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
            }
        }
    } else {
        for (opin, ipin) in [
            ("O0", "I.UL.V"),
            ("O1", "I.LL.H"),
            ("O2", "I.LR.V"),
            ("O3", "I.UR.H"),
        ] {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
            for ipin in ["I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"] {
                vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
            }
        }
    }
    let col_l = edev.grid.col_lio();
    let col_r = edev.grid.col_rio();
    let row_b = edev.grid.row_bio();
    let row_t = edev.grid.row_tio();
    for (pin, col, row, key) in [
        ("I.LL.H", col_l, row_b, "BUFGLS.H"),
        ("I.LL.V", col_l, row_b, "BUFGLS.V"),
        ("I.UL.H", col_l, row_t, "BUFGLS.H"),
        ("I.UL.V", col_l, row_t, "BUFGLS.V"),
        ("I.LR.H", col_r, row_b, "BUFGLS.H"),
        ("I.LR.V", col_r, row_b, "BUFGLS.V"),
        ("I.UR.H", col_r, row_t, "BUFGLS.H"),
        ("I.UR.V", col_r, row_t, "BUFGLS.V"),
    ] {
        let obel = vrf.find_bel(bel.die, (col, row), key).unwrap();
        vrf.verify_node(&[bel.fwire(pin), obel.fwire("O")]);
    }
}

fn verify_buff(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext) {
    vrf.verify_bel(bel, "BUFF", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire_far("I"));
    let (row, key) = match (bel.col < edev.grid.col_mid(), bel.row < edev.grid.row_mid()) {
        (true, true) => (bel.row, "IOB1"),
        (true, false) => (bel.row - 1, "IOB0"),
        (false, true) => (
            if edev.grid.is_buff_large {
                bel.row + 1
            } else {
                bel.row
            },
            "IOB1",
        ),
        (false, false) => (
            if edev.grid.is_buff_large {
                bel.row - 2
            } else {
                bel.row - 1
            },
            "IOB0",
        ),
    };
    let obel = vrf.find_bel(bel.die, (bel.col, row), key).unwrap();
    vrf.verify_node(&[bel.fwire_far("I"), obel.fwire("CLKIN")])
}

fn verify_clkc(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext) {
    let col_l = edev.grid.col_lio();
    let col_r = edev.grid.col_rio();
    let row_b = edev.grid.row_bio();
    let row_t = edev.grid.row_tio();
    for (opin, ipin, col, row) in [
        ("O.LL.V", "I.LL.V", col_l, row_b),
        ("O.UL.V", "I.UL.V", col_l, row_t),
        ("O.LR.V", "I.LR.V", col_r, row_b),
        ("O.UR.V", "I.UR.V", col_r, row_t),
    ] {
        let obel = vrf.find_bel(bel.die, (col, row), "BUFGLS.V").unwrap();
        vrf.verify_node(&[bel.fwire(ipin), obel.fwire_far("O")]);
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
    }
}

fn verify_clkqc(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext) {
    let col_l = edev.grid.col_lio();
    let col_r = edev.grid.col_rio();
    let row_b = edev.grid.row_bio();
    let row_t = edev.grid.row_tio();
    for (opin, ipin, col, row) in [
        ("O.LL.H", "I.LL.H", col_l, row_b),
        ("O.UL.H", "I.UL.H", col_l, row_t),
        ("O.LR.H", "I.LR.H", col_r, row_b),
        ("O.UR.H", "I.UR.H", col_r, row_t),
    ] {
        let obel = vrf.find_bel(bel.die, (col, row), "BUFGLS.H").unwrap();
        vrf.verify_node(&[bel.fwire(ipin), obel.fwire_far("O")]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
    }
    let obel = vrf
        .find_bel(bel.die, (edev.grid.col_mid(), edev.grid.row_mid()), "CLKC")
        .unwrap();
    for (opin, ipin) in [
        ("O.LL.V", "I.LL.V"),
        ("O.UL.V", "I.UL.V"),
        ("O.LR.V", "I.LR.V"),
        ("O.UR.V", "I.UR.V"),
    ] {
        vrf.verify_node(&[bel.fwire(ipin), obel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
    }
}

fn verify_clkq(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext) {
    let col_l = edev.grid.col_lio();
    let col_r = edev.grid.col_rio();
    let row_b = edev.grid.row_bio();
    let row_t = edev.grid.row_tio();
    for (pin, col, row, key) in [
        ("LL.H", col_l, row_b, "BUFGLS.H"),
        ("LL.V", col_l, row_b, "BUFGLS.V"),
        ("UL.H", col_l, row_t, "BUFGLS.H"),
        ("UL.V", col_l, row_t, "BUFGLS.V"),
        ("LR.H", col_r, row_b, "BUFGLS.H"),
        ("LR.V", col_r, row_b, "BUFGLS.V"),
        ("UR.H", col_r, row_t, "BUFGLS.H"),
        ("UR.V", col_r, row_t, "BUFGLS.V"),
    ] {
        let obel = vrf.find_bel(bel.die, (col, row), key).unwrap();
        let ipin = format!("I.{pin}");
        let opin_l = format!("O.{pin}.L");
        let opin_r = format!("O.{pin}.R");
        vrf.verify_node(&[bel.fwire(&ipin), obel.fwire_far("O")]);
        vrf.claim_pip(bel.crd(), bel.wire(&opin_l), bel.wire(&ipin));
        vrf.claim_pip(bel.crd(), bel.wire(&opin_r), bel.wire(&ipin));
    }
}

fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        "CLB" => verify_clb(edev, vrf, bel),
        "IOB0" | "IOB1" => verify_iob(vrf, bel),
        "TBUF0" | "TBUF1" => vrf.verify_bel(bel, "TBUF", &[], &[]),
        "DEC0" | "DEC1" | "DEC2" => vrf.verify_bel(bel, "DECODER", &[], &[]),
        _ if bel.key.starts_with("PULLUP") => vrf.verify_bel(bel, "PULLUP", &[], &[]),

        "BOT_CIN" | "TOP_COUT" => (),
        "BUFG.H" | "BUFG.V" => verify_bufg(vrf, bel),
        "BUFGE.H" | "BUFGE.V" => verify_bufge(vrf, bel),
        "BUFGLS.H" | "BUFGLS.V" => verify_bufgls(edev, vrf, bel),
        "OSC" => vrf.verify_bel(bel, "OSCILLATOR", &[], &[]),
        "TDO" => vrf.verify_bel(bel, "TESTDATA", &[], &[]),
        "MD0" => vrf.verify_bel(bel, "MODE0", &[], &[]),
        "MD1" => vrf.verify_bel(bel, "MODE1", &[], &[]),
        "MD2" => vrf.verify_bel(bel, "MODE2", &[], &[]),
        "RDBK" => vrf.verify_bel(bel, "READBACK", &[], &[]),
        "STARTUP" | "READCLK" | "UPDATE" | "BSCAN" => vrf.verify_bel(bel, bel.key, &[], &[]),
        "COUT.LR" | "COUT.UR" => verify_cout(vrf, bel),
        "CIN.LL" | "CIN.UL" => verify_cin(vrf, bel),
        "TBUF_SPLITTER0" | "TBUF_SPLITTER1" => verify_tbuf_splitter(vrf, bel),
        "CLKH" => verify_clkh(edev, vrf, bel),
        "BUFF" => verify_buff(edev, vrf, bel),
        "CLKC" => verify_clkc(edev, vrf, bel),
        "CLKQC" => verify_clkqc(edev, vrf, bel),
        "CLKQ" => verify_clkq(edev, vrf, bel),
        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}

pub fn verify_device(edev: &ExpandedDevice, rd: &Part) {
    verify(
        rd,
        &edev.egrid,
        |_| (),
        |vrf, bel| verify_bel(edev, vrf, bel),
        |_| (),
    );
}
