use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::BelCoord;
use prjcombine_re_xilinx_naming_virtex::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{SitePinDir, Verifier};
use prjcombine_virtex::{chip::ChipKind, defs::bslots, expanded::ExpandedDevice};

fn verify_slice(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::SLICE.index_of(bel.slot).unwrap();
    vrf.verify_legacy_bel(
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
        vrf.claim_net(&[bel.wire("CIN"), obel.wire_far("COUT")]);
    } else {
        vrf.claim_net(&[bel.wire("CIN")]);
    }
    vrf.claim_net(&[bel.wire("COUT")]);
    vrf.claim_pip(bel.wire_far("COUT"), bel.wire("COUT"));

    vrf.claim_net(&[bel.wire("F5")]);
    vrf.claim_net(&[bel.wire("F5IN")]);
    let oslot = bslots::SLICE[idx ^ 1];
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.wire("F5IN"), obel.wire("F5"));
}

fn verify_iob(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let mut kind = "IOB";
    let mut pins = Vec::new();
    if bel.name.unwrap().starts_with("EMPTY") {
        kind = "EMPTYIOB";
    }
    if (bel.col == edev.chip.col_w() || bel.col == edev.chip.col_e())
        && ((bel.row == edev.chip.row_mid() && bel.slot == bslots::IO[3])
            || (bel.row == edev.chip.row_mid() - 1 && bel.slot == bslots::IO[1]))
    {
        kind = "PCIIOB";
        pins.push(("PCI", SitePinDir::Out));
    }
    if edev.chip.kind != ChipKind::Virtex
        && (bel.row == edev.chip.row_s() || bel.row == edev.chip.row_n())
        && ((bel.col == edev.chip.col_clk() && bel.slot == bslots::IO[2])
            || (bel.col == edev.chip.col_clk() - 1 && bel.slot == bslots::IO[1]))
    {
        kind = "DLLIOB";
        pins.push(("DLLFB", SitePinDir::Out));
    }
    vrf.verify_legacy_bel(bel, kind, &pins, &[]);
}

fn verify_tbuf(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "TBUF", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_net(&[bel.wire("O")]);
}

fn verify_tbus(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel = vrf.find_bel_sibling(bel, bslots::TBUF[0]);
    vrf.claim_pip(bel.wire("BUS0"), obel.wire("O"));
    vrf.claim_pip(bel.wire("BUS2"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, bslots::TBUF[1]);
    vrf.claim_pip(bel.wire("BUS1"), obel.wire("O"));
    vrf.claim_pip(bel.wire("BUS3"), obel.wire("O"));
    if bel.naming.pins.contains_key("BUS3_E") {
        let col_e = edev.chip.col_e();
        if bel.col.to_idx() < col_e.to_idx() - 5 {
            vrf.claim_net(&[bel.wire("BUS3_E")]);
        }
        vrf.claim_pip(bel.wire("BUS3"), bel.wire("BUS3_E"));
        vrf.claim_pip(bel.wire("BUS3_E"), bel.wire("BUS3"));
        let obel = vrf.find_bel_walk(bel, 1, 0, bslots::TBUS).unwrap();
        vrf.verify_net(&[bel.wire("BUS0"), obel.wire("BUS1")]);
        vrf.verify_net(&[bel.wire("BUS1"), obel.wire("BUS2")]);
        vrf.verify_net(&[bel.wire("BUS2"), obel.wire("BUS3")]);
        vrf.verify_net(&[bel.wire("BUS3_E"), obel.wire("BUS0")]);
    }
    if bel.naming.pins.contains_key("OUT") {
        vrf.claim_pip(bel.wire("OUT"), bel.wire("BUS2"));
    }
}

fn verify_bufg(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "GCLK", &[], &["OUT.GLOBAL"]);
    vrf.claim_net(&[bel.wire("OUT.GLOBAL")]);
    vrf.claim_pip(bel.wire("OUT.GLOBAL"), bel.wire("OUT"));
}

fn verify_iofb(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::IOFB.index_of(bel.slot).unwrap();
    let obel = match idx {
        0 => vrf.find_bel_sibling(bel, bslots::IO[2]),
        1 => vrf.find_bel_delta(bel, -1, 0, bslots::IO[1]).unwrap(),
        _ => unreachable!(),
    };
    vrf.verify_net(&[bel.wire("O"), obel.wire("DLLFB")]);
}

fn verify_pcilogic(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(
        bel,
        "PCILOGIC",
        &[("IRDY", SitePinDir::In), ("TRDY", SitePinDir::In)],
        &[],
    );
    for pin in ["IRDY", "TRDY"] {
        for pip in &bel.naming.pins[pin].pips {
            vrf.claim_pip_tri(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
        }
        vrf.claim_net(&[bel.wire(pin)]);
        vrf.claim_net(&[bel.wire_far(pin)]);
    }
    let obel = vrf.get_legacy_bel(bel.cell.with_row(edev.chip.row_mid()).bel(bslots::IO[3]));
    vrf.verify_net(&[bel.wire_far("IRDY"), obel.wire("PCI")]);
    let obel = vrf.get_legacy_bel(
        bel.cell
            .with_row(edev.chip.row_mid() - 1)
            .bel(bslots::IO[1]),
    );
    vrf.verify_net(&[bel.wire_far("TRDY"), obel.wire("PCI")]);
}

fn verify_clkc(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    for (opin, ipin, srow, sslot) in [
        ("OUT0", "IN0", edev.chip.row_s(), bslots::BUFG[0]),
        ("OUT1", "IN1", edev.chip.row_s(), bslots::BUFG[1]),
        ("OUT2", "IN2", edev.chip.row_n(), bslots::BUFG[0]),
        ("OUT3", "IN3", edev.chip.row_n(), bslots::BUFG[1]),
    ] {
        vrf.claim_net(&[bel.wire(opin)]);
        vrf.claim_pip(bel.wire(opin), bel.wire(ipin));
        let obel = vrf.get_legacy_bel(bel.cell.with_cr(edev.chip.col_clk(), srow).bel(sslot));
        vrf.verify_net(&[bel.wire(ipin), obel.wire("OUT.GLOBAL")]);
    }
}

fn verify_gclkc(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    for (opin, ipin) in [
        ("OUT0", "IN0"),
        ("OUT1", "IN1"),
        ("OUT2", "IN2"),
        ("OUT3", "IN3"),
    ] {
        vrf.claim_net(&[bel.wire(opin)]);
        vrf.claim_pip(bel.wire(opin), bel.wire(ipin));
        let obel = vrf.get_legacy_bel(bel.cell.with_col(edev.chip.col_clk()).bel(bslots::CLKC));
        vrf.verify_net(&[bel.wire(ipin), obel.wire(opin)]);
    }
}

fn verify_bram_clkh(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    for (opin, ipin) in [
        ("OUT0", "IN0"),
        ("OUT1", "IN1"),
        ("OUT2", "IN2"),
        ("OUT3", "IN3"),
    ] {
        vrf.claim_pip(bel.wire(opin), bel.wire(ipin));
        let obel = vrf.get_legacy_bel(bel.cell.with_col(edev.chip.col_clk()).bel(bslots::CLKC));
        vrf.verify_net(&[bel.wire(ipin), obel.wire(opin)]);
    }
}

fn verify_clkv(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    for (opinl, opinr, ipin, opin) in [
        ("OUT_L0", "OUT_R0", "IN0", "OUT0"),
        ("OUT_L1", "OUT_R1", "IN1", "OUT1"),
        ("OUT_L2", "OUT_R2", "IN2", "OUT2"),
        ("OUT_L3", "OUT_R3", "IN3", "OUT3"),
    ] {
        vrf.claim_pip(bel.wire(opinl), bel.wire(ipin));
        vrf.claim_pip(bel.wire(opinr), bel.wire(ipin));
        let obel = vrf.get_legacy_bel(bel.cell.with_row(edev.chip.row_clk()).bel(bslots::GCLKC));
        vrf.verify_net(&[bel.wire(ipin), obel.wire(opin)]);
    }
}

fn verify_clkv_bram_sn(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    for (opinl, opinr, ipin) in [
        ("OUT_L0", "OUT_R0", "IN0"),
        ("OUT_L1", "OUT_R1", "IN1"),
        ("OUT_L2", "OUT_R2", "IN2"),
        ("OUT_L3", "OUT_R3", "IN3"),
    ] {
        vrf.claim_pip(bel.wire(opinl), bel.wire(ipin));
        vrf.claim_pip(bel.wire(opinr), bel.wire(ipin));
    }
}

fn verify_clkv_bram(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    for i in 0..4 {
        let ipin = format!("IN{i}");
        for j in 0..4 {
            let opinl = format!("OUT_L{j}_{i}");
            let opinr = format!("OUT_R{j}_{i}");
            vrf.claim_pip(bel.wire(&opinl), bel.wire(&ipin));
            vrf.claim_pip(bel.wire(&opinr), bel.wire(&ipin));
        }
    }
}

fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    match bcrd.slot {
        bslots::INT | bslots::GCLK_INT | bslots::DLL_INT | bslots::PCI_INT | bslots::GLOBAL => (),
        _ if bslots::SLICE.contains(bcrd.slot) => verify_slice(vrf, bcrd),
        _ if bslots::IO.contains(bcrd.slot) => verify_iob(edev, vrf, bcrd),
        _ if bslots::TBUF.contains(bcrd.slot) => verify_tbuf(vrf, bcrd),
        bslots::TBUS => verify_tbus(edev, vrf, bcrd),
        bslots::BRAM => {
            let bel = &mut vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "BLOCKRAM", &[], &[])
        }
        bslots::STARTUP | bslots::CAPTURE | bslots::BSCAN => {
            let bel = &mut vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, edev.db.bel_slots.key(bcrd.slot), &[], &[])
        }
        _ if bslots::GCLK_IO.contains(bcrd.slot) => {
            let bel = &mut vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "GCLKIOB", &[], &[])
        }
        _ if bslots::BUFG.contains(bcrd.slot) => verify_bufg(vrf, bcrd),
        _ if bslots::IOFB.contains(bcrd.slot) => verify_iofb(vrf, bcrd),
        bslots::PCILOGIC => verify_pcilogic(edev, vrf, bcrd),
        bslots::DLL => {
            let bel = &mut vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "DLL", &[], &[])
        }
        bslots::CLKC => verify_clkc(edev, vrf, bcrd),
        bslots::GCLKC => verify_gclkc(edev, vrf, bcrd),
        bslots::BRAM_CLKH => verify_bram_clkh(edev, vrf, bcrd),
        bslots::CLKV => verify_clkv(edev, vrf, bcrd),
        bslots::CLKV_BRAM_S | bslots::CLKV_BRAM_N => verify_clkv_bram_sn(vrf, bcrd),
        bslots::CLKV_BRAM => verify_clkv_bram(vrf, bcrd),
        _ => unreachable!(),
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);
    vrf.prep_int_wires();
    vrf.handle_int();
    for (tcrd, tile) in endev.edev.tiles() {
        let tcls = &endev.edev.db[tile.class];
        for slot in tcls.bels.ids() {
            verify_bel(endev.edev, &mut vrf, tcrd.bel(slot));
        }
    }
    vrf.finish();
}
