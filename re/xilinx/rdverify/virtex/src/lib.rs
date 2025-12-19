use prjcombine_entity::EntityId;
use prjcombine_re_xilinx_naming_virtex::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};
use prjcombine_virtex::{bels, chip::ChipKind};

fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "SLICE",
        &[
            ("CIN", SitePinDir::In),
            ("COUT", SitePinDir::Out),
            ("F5IN", SitePinDir::In),
            ("F5", SitePinDir::Out),
        ],
        &[],
    );
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.slot) {
        vrf.claim_net(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
    } else {
        vrf.claim_net(&[bel.fwire("CIN")]);
    }
    vrf.claim_net(&[bel.fwire("COUT")]);
    vrf.claim_pip(bel.crd(), bel.wire_far("COUT"), bel.wire("COUT"));

    vrf.claim_net(&[bel.fwire("F5")]);
    vrf.claim_net(&[bel.fwire("F5IN")]);
    let oslot = match bel.slot {
        bels::SLICE0 => bels::SLICE1,
        bels::SLICE1 => bels::SLICE0,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("F5IN"), obel.wire("F5"));
}

fn verify_iob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut kind = "IOB";
    let mut pins = Vec::new();
    if bel.name.unwrap().starts_with("EMPTY") {
        kind = "EMPTYIOB";
    }
    if (bel.col == endev.grid.col_w() || bel.col == endev.grid.col_e())
        && ((bel.row == endev.grid.row_mid() && bel.slot == bels::IO3)
            || (bel.row == endev.grid.row_mid() - 1 && bel.slot == bels::IO1))
    {
        kind = "PCIIOB";
        pins.push(("PCI", SitePinDir::Out));
    }
    if endev.grid.kind != ChipKind::Virtex
        && (bel.row == endev.grid.row_s() || bel.row == endev.grid.row_n())
        && ((bel.col == endev.grid.col_clk() && bel.slot == bels::IO2)
            || (bel.col == endev.grid.col_clk() - 1 && bel.slot == bels::IO1))
    {
        kind = "DLLIOB";
        pins.push(("DLLFB", SitePinDir::Out));
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
}

fn verify_tbuf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "TBUF", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_net(&[bel.fwire("O")]);
}

fn verify_tbus(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, bels::TBUF0);
    vrf.claim_pip(bel.crd(), bel.wire("BUS0"), obel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("BUS2"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, bels::TBUF1);
    vrf.claim_pip(bel.crd(), bel.wire("BUS1"), obel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("BUS3"), obel.wire("O"));
    if bel.naming.pins.contains_key("BUS3_E") {
        let col_e = endev.grid.col_e();
        if bel.col.to_idx() < col_e.to_idx() - 5 {
            vrf.claim_net(&[bel.fwire("BUS3_E")]);
        }
        vrf.claim_pip(bel.crd(), bel.wire("BUS3"), bel.wire("BUS3_E"));
        vrf.claim_pip(bel.crd(), bel.wire("BUS3_E"), bel.wire("BUS3"));
        let obel = vrf.find_bel_walk(bel, 1, 0, bels::TBUS).unwrap();
        vrf.verify_net(&[bel.fwire("BUS0"), obel.fwire("BUS1")]);
        vrf.verify_net(&[bel.fwire("BUS1"), obel.fwire("BUS2")]);
        vrf.verify_net(&[bel.fwire("BUS2"), obel.fwire("BUS3")]);
        vrf.verify_net(&[bel.fwire("BUS3_E"), obel.fwire("BUS0")]);
    }
    if bel.naming.pins.contains_key("OUT") {
        vrf.claim_pip(bel.crd(), bel.wire("OUT"), bel.wire("BUS2"));
    }
}

fn verify_bufg(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "GCLK", &[], &["OUT.GLOBAL"]);
    vrf.claim_net(&[bel.fwire("OUT.GLOBAL")]);
    vrf.claim_pip(bel.crd(), bel.wire("OUT.GLOBAL"), bel.wire("OUT"));
}

fn verify_iofb(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = match bel.slot {
        bels::IOFB0 => vrf.find_bel_sibling(bel, bels::IO2),
        bels::IOFB1 => vrf.find_bel_delta(bel, -1, 0, bels::IO1).unwrap(),
        _ => unreachable!(),
    };
    vrf.verify_net(&[bel.fwire("O"), obel.fwire("DLLFB")]);
}

fn verify_pcilogic(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "PCILOGIC",
        &[("IRDY", SitePinDir::In), ("TRDY", SitePinDir::In)],
        &[],
    );
    for pin in ["IRDY", "TRDY"] {
        for pip in &bel.naming.pins[pin].pips {
            vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
        }
        vrf.claim_net(&[bel.fwire(pin)]);
        vrf.claim_net(&[bel.fwire_far(pin)]);
    }
    let obel = vrf.get_bel(bel.cell.with_row(endev.grid.row_mid()).bel(bels::IO3));
    vrf.verify_net(&[bel.fwire_far("IRDY"), obel.fwire("PCI")]);
    let obel = vrf.get_bel(bel.cell.with_row(endev.grid.row_mid() - 1).bel(bels::IO1));
    vrf.verify_net(&[bel.fwire_far("TRDY"), obel.fwire("PCI")]);
}

fn verify_clkc(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (opin, ipin, srow, sslot) in [
        ("OUT0", "IN0", endev.grid.row_s(), bels::BUFG0),
        ("OUT1", "IN1", endev.grid.row_s(), bels::BUFG1),
        ("OUT2", "IN2", endev.grid.row_n(), bels::BUFG0),
        ("OUT3", "IN3", endev.grid.row_n(), bels::BUFG1),
    ] {
        vrf.claim_net(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
        let obel = vrf.get_bel(bel.cell.with_cr(endev.grid.col_clk(), srow).bel(sslot));
        vrf.verify_net(&[bel.fwire(ipin), obel.fwire("OUT.GLOBAL")]);
    }
}

fn verify_gclkc(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (opin, ipin) in [
        ("OUT0", "IN0"),
        ("OUT1", "IN1"),
        ("OUT2", "IN2"),
        ("OUT3", "IN3"),
    ] {
        vrf.claim_net(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
        let obel = vrf.get_bel(bel.cell.with_col(endev.grid.col_clk()).bel(bels::CLKC));
        vrf.verify_net(&[bel.fwire(ipin), obel.fwire(opin)]);
    }
}

fn verify_bram_clkh(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (opin, ipin) in [
        ("OUT0", "IN0"),
        ("OUT1", "IN1"),
        ("OUT2", "IN2"),
        ("OUT3", "IN3"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
        let obel = vrf.get_bel(bel.cell.with_col(endev.grid.col_clk()).bel(bels::CLKC));
        vrf.verify_net(&[bel.fwire(ipin), obel.fwire(opin)]);
    }
}

fn verify_clkv(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (opinl, opinr, ipin, opin) in [
        ("OUT_L0", "OUT_R0", "IN0", "OUT0"),
        ("OUT_L1", "OUT_R1", "IN1", "OUT1"),
        ("OUT_L2", "OUT_R2", "IN2", "OUT2"),
        ("OUT_L3", "OUT_R3", "IN3", "OUT3"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(opinl), bel.wire(ipin));
        vrf.claim_pip(bel.crd(), bel.wire(opinr), bel.wire(ipin));
        let obel = vrf.get_bel(bel.cell.with_row(endev.grid.row_clk()).bel(bels::GCLKC));
        vrf.verify_net(&[bel.fwire(ipin), obel.fwire(opin)]);
    }
}

fn verify_clkv_bram_sn(vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (opinl, opinr, ipin) in [
        ("OUT_L0", "OUT_R0", "IN0"),
        ("OUT_L1", "OUT_R1", "IN1"),
        ("OUT_L2", "OUT_R2", "IN2"),
        ("OUT_L3", "OUT_R3", "IN3"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(opinl), bel.wire(ipin));
        vrf.claim_pip(bel.crd(), bel.wire(opinr), bel.wire(ipin));
    }
}

fn verify_clkv_bram(vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..4 {
        let ipin = format!("IN{i}");
        for j in 0..4 {
            let opinl = format!("OUT_L{j}_{i}");
            let opinr = format!("OUT_R{j}_{i}");
            vrf.claim_pip(bel.crd(), bel.wire(&opinl), bel.wire(&ipin));
            vrf.claim_pip(bel.crd(), bel.wire(&opinr), bel.wire(&ipin));
        }
    }
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.slot {
        bels::SLICE0 | bels::SLICE1 => verify_slice(vrf, bel),
        bels::IO0 | bels::IO1 | bels::IO2 | bels::IO3 => verify_iob(endev, vrf, bel),
        bels::TBUF0 | bels::TBUF1 => verify_tbuf(vrf, bel),
        bels::TBUS => verify_tbus(endev, vrf, bel),
        bels::BRAM => vrf.verify_bel(bel, "BLOCKRAM", &[], &[]),
        bels::STARTUP | bels::CAPTURE | bels::BSCAN => {
            vrf.verify_bel(bel, endev.edev.db.bel_slots.key(bel.slot), &[], &[])
        }
        bels::GCLK_IO0 | bels::GCLK_IO1 => vrf.verify_bel(bel, "GCLKIOB", &[], &[]),
        bels::BUFG0 | bels::BUFG1 => verify_bufg(vrf, bel),
        bels::IOFB0 | bels::IOFB1 => verify_iofb(vrf, bel),
        bels::PCILOGIC => verify_pcilogic(endev, vrf, bel),
        bels::DLL => vrf.verify_bel(bel, "DLL", &[], &[]),
        bels::CLKC => verify_clkc(endev, vrf, bel),
        bels::GCLKC => verify_gclkc(endev, vrf, bel),
        bels::BRAM_CLKH => verify_bram_clkh(endev, vrf, bel),
        bels::CLKV => verify_clkv(endev, vrf, bel),
        bels::CLKV_BRAM_S | bels::CLKV_BRAM_N => verify_clkv_bram_sn(vrf, bel),
        bels::CLKV_BRAM => verify_clkv_bram(vrf, bel),
        _ => unreachable!(),
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
