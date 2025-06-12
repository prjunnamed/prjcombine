use prjcombine_re_xilinx_naming_xc2000::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};
use prjcombine_xc2000::{bels::xc4000 as bels, chip::ChipKind};

fn verify_clb(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "CLB",
        &[("COUT", SitePinDir::Out), ("CIN", SitePinDir::In)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("COUT")]);
    if !endev.chip.kind.is_clb_xl() {
        vrf.claim_pip(bel.crd(), bel.wire("CIN.B"), bel.wire("COUT"));
        vrf.claim_pip(bel.crd(), bel.wire("CIN.T"), bel.wire("COUT"));
        vrf.claim_node(&[bel.fwire("CIN")]);
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bels::CLB) {
            vrf.verify_node(&[bel.fwire("CIN.B"), obel.fwire("CIN")]);
        } else if let Some(obel) = vrf.find_bel_delta(bel, 1, 0, bels::CLB) {
            vrf.verify_node(&[bel.fwire("CIN.B"), obel.fwire("CIN")]);
        } else {
            let obel = vrf.find_bel_delta(bel, 1, -1, bels::COUT).unwrap();
            vrf.verify_node(&[bel.fwire("CIN.B"), obel.fwire("I")]);
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, bels::CLB) {
            vrf.verify_node(&[bel.fwire("CIN.T"), obel.fwire("CIN")]);
        } else if let Some(obel) = vrf.find_bel_delta(bel, 1, 0, bels::CLB) {
            vrf.verify_node(&[bel.fwire("CIN.T"), obel.fwire("CIN")]);
        } else {
            let obel = vrf.find_bel_delta(bel, 1, 1, bels::COUT).unwrap();
            vrf.verify_node(&[bel.fwire("CIN.T"), obel.fwire("I")]);
        }
    } else {
        vrf.claim_pip(bel.crd(), bel.wire_far("COUT"), bel.wire("COUT"));
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bels::CLB) {
            vrf.verify_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
        } else {
            let obel = vrf.find_bel_delta(bel, 0, -1, bels::CIN).unwrap();
            vrf.verify_node(&[bel.fwire("CIN"), obel.fwire("CIN")]);
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, bels::COUT) {
            vrf.verify_node(&[bel.fwire_far("COUT"), obel.fwire("COUT")]);
        } else {
            vrf.claim_node(&[bel.fwire_far("COUT")]);
        }
    }
}

fn verify_iob(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    let kind = if !bel.info.pins.contains_key("I1") {
        "FCLKIOB"
    } else if bel.info.pins.contains_key("CLKIN") {
        "CLKIOB"
    } else if bel.naming.pins.contains_key("CLKIN") {
        pins.push(("CLKIN", SitePinDir::Out));
        vrf.claim_node(&[bel.fwire("CLKIN")]);
        "FCLKIOB"
    } else {
        "IOB"
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire_far("EC"));
}

fn verify_tbuf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "TBUF", &[], &[]);
    if endev.chip.kind == ChipKind::Xc4000E {
        let node = &vrf.db.tile_classes[bel.tile.class];
        let naming = &vrf.ndb.tile_class_namings[bel.ntile.naming];
        let i = &bel.info.pins["I"];
        let wire = *i.wires.iter().next().unwrap();
        let mux = &node.muxes[&wire];
        for &inp in &mux.ins {
            vrf.claim_pip(bel.crd(), bel.wire_far("O"), &naming.wires[&inp]);
        }
    }
}

fn verify_bufg(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "BUFG", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
}

fn verify_bufge(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_bufg = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::BUFGE_H => bels::BUFG_H,
            bels::BUFGE_V => bels::BUFG_V,
            _ => unreachable!(),
        },
    );
    vrf.verify_bel(bel, "BUFGE", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_pip(bel.crd(), bel.wire("I"), obel_bufg.wire("O"));
}

fn verify_bufgls(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if !endev.chip.kind.is_xl() {
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
            match bel.slot {
                bels::BUFGLS_H => bels::BUFG_H,
                bels::BUFGLS_V => bels::BUFG_V,
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

fn verify_osc(vrf: &mut Verifier, bel: &BelContext) {
    vrf.verify_bel(
        bel,
        "OSCILLATOR",
        &[
            ("F15", SitePinDir::Out),
            ("F490", SitePinDir::Out),
            ("F16K", SitePinDir::Out),
            ("F500K", SitePinDir::Out),
        ],
        &["OUT0", "OUT1"],
    );
    for pin in ["F15", "F490", "F16K", "F500K"] {
        vrf.claim_node(&[bel.fwire(pin)]);
        vrf.claim_pip(bel.crd(), bel.wire("OUT0"), bel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire("OUT1"), bel.wire(pin));
    }
}

fn verify_cout(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    if endev.edev.chip.kind.is_clb_xl() {
        return;
    }
    vrf.verify_bel(bel, "COUT", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
}

fn verify_cin(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    if endev.edev.chip.kind.is_clb_xl() {
        return;
    }
    vrf.verify_bel(bel, "CIN", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
    let obel = if bel.row == endev.edev.chip.row_s() {
        vrf.find_bel_delta(bel, 1, 1, bels::CLB).unwrap()
    } else {
        vrf.find_bel_delta(bel, 1, -1, bels::CLB).unwrap()
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

fn verify_clkh(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    if endev.chip.kind == ChipKind::SpartanXl {
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
    let col_w = endev.chip.col_w();
    let col_e = endev.chip.col_e();
    let row_s = endev.chip.row_s();
    let row_n = endev.chip.row_n();
    for (pin, col, row, slot) in [
        ("I.LL.H", col_w, row_s, bels::BUFGLS_H),
        ("I.LL.V", col_w, row_s, bels::BUFGLS_V),
        ("I.UL.H", col_w, row_n, bels::BUFGLS_H),
        ("I.UL.V", col_w, row_n, bels::BUFGLS_V),
        ("I.LR.H", col_e, row_s, bels::BUFGLS_H),
        ("I.LR.V", col_e, row_s, bels::BUFGLS_V),
        ("I.UR.H", col_e, row_n, bels::BUFGLS_H),
        ("I.UR.V", col_e, row_n, bels::BUFGLS_V),
    ] {
        let obel = vrf.get_bel(bel.cell.with_cr(col, row).bel(slot));
        vrf.verify_node(&[bel.fwire(pin), obel.fwire("O")]);
    }
}

fn verify_buff(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    vrf.verify_bel(bel, "BUFF", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire_far("I"));
    let (row, slot) = match (
        bel.col < endev.chip.col_mid(),
        bel.row < endev.chip.row_mid(),
    ) {
        (true, true) => (bel.row, bels::IO1),
        (true, false) => (bel.row - 1, bels::IO0),
        (false, true) => (
            if endev.chip.is_buff_large {
                bel.row + 1
            } else {
                bel.row
            },
            bels::IO1,
        ),
        (false, false) => (
            if endev.chip.is_buff_large {
                bel.row - 2
            } else {
                bel.row - 1
            },
            bels::IO0,
        ),
    };
    let obel = vrf.get_bel(bel.cell.with_row(row).bel(slot));
    vrf.verify_node(&[bel.fwire_far("I"), obel.fwire("CLKIN")])
}

fn verify_clkc(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    let col_w = endev.chip.col_w();
    let col_e = endev.chip.col_e();
    let row_s = endev.chip.row_s();
    let row_n = endev.chip.row_n();
    for (opin, ipin, col, row) in [
        ("O.LL.V", "I.LL.V", col_w, row_s),
        ("O.UL.V", "I.UL.V", col_w, row_n),
        ("O.LR.V", "I.LR.V", col_e, row_s),
        ("O.UR.V", "I.UR.V", col_e, row_n),
    ] {
        let obel = vrf.get_bel(bel.cell.with_cr(col, row).bel(bels::BUFGLS_V));
        vrf.verify_node(&[bel.fwire(ipin), obel.fwire_far("O")]);
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
    }
}

fn verify_clkqc(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    let col_w = endev.chip.col_w();
    let col_e = endev.chip.col_e();
    let row_s = endev.chip.row_s();
    let row_n = endev.chip.row_n();
    for (opin, ipin, col, row) in [
        ("O.LL.H", "I.LL.H", col_w, row_s),
        ("O.UL.H", "I.UL.H", col_w, row_n),
        ("O.LR.H", "I.LR.H", col_e, row_s),
        ("O.UR.H", "I.UR.H", col_e, row_n),
    ] {
        let obel = vrf.get_bel(bel.cell.with_cr(col, row).bel(bels::BUFGLS_H));
        vrf.verify_node(&[bel.fwire(ipin), obel.fwire_far("O")]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
    }
    let obel = vrf.get_bel(
        bel.cell
            .with_cr(endev.chip.col_mid(), endev.chip.row_mid())
            .bel(bels::CLKC),
    );
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

fn verify_clkq(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    let col_w = endev.chip.col_w();
    let col_e = endev.chip.col_e();
    let row_s = endev.chip.row_s();
    let row_n = endev.chip.row_n();
    for (pin, col, row, slot) in [
        ("LL.H", col_w, row_s, bels::BUFGLS_H),
        ("LL.V", col_w, row_s, bels::BUFGLS_V),
        ("UL.H", col_w, row_n, bels::BUFGLS_H),
        ("UL.V", col_w, row_n, bels::BUFGLS_V),
        ("LR.H", col_e, row_s, bels::BUFGLS_H),
        ("LR.V", col_e, row_s, bels::BUFGLS_V),
        ("UR.H", col_e, row_n, bels::BUFGLS_H),
        ("UR.V", col_e, row_n, bels::BUFGLS_V),
    ] {
        let obel = vrf.get_bel(bel.cell.with_cr(col, row).bel(slot));
        let ipin = format!("I.{pin}");
        let opin_l = format!("O.{pin}.L");
        let opin_r = format!("O.{pin}.R");
        vrf.verify_node(&[bel.fwire(&ipin), obel.fwire_far("O")]);
        vrf.claim_pip(bel.crd(), bel.wire(&opin_l), bel.wire(&ipin));
        vrf.claim_pip(bel.crd(), bel.wire(&opin_r), bel.wire(&ipin));
    }
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let slot_name = endev.edev.egrid.db.bel_slots.key(bel.slot);
    match bel.slot {
        bels::CLB => verify_clb(endev, vrf, bel),
        bels::IO0 | bels::IO1 => verify_iob(vrf, bel),
        bels::TBUF0 | bels::TBUF1 => verify_tbuf(endev, vrf, bel),
        bels::DEC0 | bels::DEC1 | bels::DEC2 => vrf.verify_bel(bel, "DECODER", &[], &[]),
        _ if slot_name.starts_with("PULLUP") => vrf.verify_bel(bel, "PULLUP", &[], &[]),
        bels::BUFG_H | bels::BUFG_V => verify_bufg(vrf, bel),
        bels::BUFGE_H | bels::BUFGE_V => verify_bufge(vrf, bel),
        bels::BUFGLS_H | bels::BUFGLS_V => verify_bufgls(endev, vrf, bel),
        bels::OSC => verify_osc(vrf, bel),
        bels::TDO => vrf.verify_bel(bel, "TESTDATA", &[], &[]),
        bels::MD0 => vrf.verify_bel(bel, "MODE0", &[], &[]),
        bels::MD1 => vrf.verify_bel(bel, "MODE1", &[], &[]),
        bels::MD2 => vrf.verify_bel(bel, "MODE2", &[], &[]),
        bels::RDBK => vrf.verify_bel(bel, "READBACK", &[], &[]),
        bels::STARTUP | bels::READCLK | bels::UPDATE | bels::BSCAN => {
            vrf.verify_bel(bel, slot_name, &[], &[])
        }
        bels::COUT => verify_cout(endev, vrf, bel),
        bels::CIN => verify_cin(endev, vrf, bel),
        bels::TBUF_SPLITTER0 | bels::TBUF_SPLITTER1 => verify_tbuf_splitter(vrf, bel),
        bels::CLKH => verify_clkh(endev, vrf, bel),
        bels::BUFF => verify_buff(endev, vrf, bel),
        bels::CLKC => verify_clkc(endev, vrf, bel),
        bels::CLKQC => verify_clkqc(endev, vrf, bel),
        bels::CLKQ => verify_clkq(endev, vrf, bel),
        _ => println!("MEOW {} {:?}", slot_name, bel.name),
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    verify(
        rd,
        &endev.ngrid,
        |_| (),
        |vrf, bel| verify_bel(endev, vrf, bel),
        |_| (),
    );
}
