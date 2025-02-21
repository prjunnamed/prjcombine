use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_virtex2::chip::ColumnKind;

pub fn verify_slice_v2(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    vrf.verify_bel(
        bel,
        "SLICE",
        &[
            ("DX", SitePinDir::In),
            ("DY", SitePinDir::In),
            ("FXINA", SitePinDir::In),
            ("FXINB", SitePinDir::In),
            ("F5", SitePinDir::Out),
            ("FX", SitePinDir::Out),
            ("CIN", SitePinDir::In),
            ("COUT", SitePinDir::Out),
            ("SHIFTIN", SitePinDir::In),
            ("SHIFTOUT", SitePinDir::Out),
            ("ALTDIG", SitePinDir::In),
            ("DIG", SitePinDir::Out),
            ("SLICEWE0", SitePinDir::In),
            ("SLICEWE1", SitePinDir::In),
            ("SLICEWE2", SitePinDir::In),
            ("BXOUT", SitePinDir::Out),
            ("BYOUT", SitePinDir::Out),
            ("BYINVOUT", SitePinDir::Out),
            ("SOPIN", SitePinDir::In),
            ("SOPOUT", SitePinDir::Out),
        ],
        &[],
    );
    vrf.claim_node(&[bel.fwire("DX")]);
    vrf.claim_pip(bel.crd(), bel.wire("DX"), bel.wire("X"));
    vrf.claim_node(&[bel.fwire("DY")]);
    vrf.claim_pip(bel.crd(), bel.wire("DY"), bel.wire("Y"));
    for pin in [
        "F5", "FX", "COUT", "SHIFTOUT", "DIG", "BYOUT", "BXOUT", "BYINVOUT", "SOPOUT",
    ] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    for (dbel, dpin, sbel, spin) in [
        ("SLICE0", "FXINA", "SLICE0", "F5"),
        ("SLICE0", "FXINB", "SLICE1", "F5"),
        ("SLICE1", "FXINA", "SLICE0", "FX"),
        ("SLICE1", "FXINB", "SLICE2", "FX"),
        ("SLICE2", "FXINA", "SLICE2", "F5"),
        ("SLICE2", "FXINB", "SLICE3", "F5"),
        ("SLICE3", "FXINA", "SLICE1", "FX"),
        // SLICE3 FXINB <- top's SLICE1 FX

        // SLICE0 CIN <- bot's SLICE1 COUT
        ("SLICE1", "CIN", "SLICE0", "COUT"),
        // SLICE2 CIN <- bot's SLICE3 COUT
        ("SLICE3", "CIN", "SLICE2", "COUT"),
        ("SLICE0", "SHIFTIN", "SLICE1", "SHIFTOUT"),
        ("SLICE1", "SHIFTIN", "SLICE2", "SHIFTOUT"),
        ("SLICE2", "SHIFTIN", "SLICE3", "SHIFTOUT"),
        // SLICE3 SHIFTIN disconnected? supposed to be top's SLICE0 SHIFTOUT?
        ("SLICE3", "DIG_LOCAL", "SLICE3", "DIG"),
        ("SLICE0", "ALTDIG", "SLICE1", "DIG"),
        ("SLICE1", "ALTDIG", "SLICE3", "DIG_LOCAL"),
        ("SLICE2", "ALTDIG", "SLICE3", "DIG_LOCAL"),
        ("SLICE3", "ALTDIG", "SLICE3", "DIG_S"), // top's SLICE3 DIG
        ("SLICE1", "BYOUT_LOCAL", "SLICE1", "BYOUT"),
        ("SLICE0", "BYINVOUT_LOCAL", "SLICE0", "BYINVOUT"),
        ("SLICE1", "BYINVOUT_LOCAL", "SLICE1", "BYINVOUT"),
        ("SLICE0", "SLICEWE0", "SLICE0", "BXOUT"),
        ("SLICE1", "SLICEWE0", "SLICE1", "BXOUT"),
        ("SLICE2", "SLICEWE0", "SLICE0", "BXOUT"),
        ("SLICE3", "SLICEWE0", "SLICE1", "BXOUT"),
        ("SLICE0", "SLICEWE1", "SLICE0", "BYOUT"),
        ("SLICE1", "SLICEWE1", "SLICE0", "BYINVOUT_LOCAL"),
        ("SLICE2", "SLICEWE1", "SLICE0", "BYOUT"),
        ("SLICE3", "SLICEWE1", "SLICE0", "BYINVOUT_LOCAL"),
        ("SLICE0", "SLICEWE2", "SLICE1", "BYOUT_LOCAL"),
        ("SLICE1", "SLICEWE2", "SLICE1", "BYOUT_LOCAL"),
        ("SLICE2", "SLICEWE2", "SLICE1", "BYINVOUT_LOCAL"),
        ("SLICE3", "SLICEWE2", "SLICE1", "BYINVOUT_LOCAL"),
        // SLICE0 SOPIN <- left's SLICE2 SOPOUT
        // SLICE1 SOPIN <- left's SLICE3 SOPOUT
        ("SLICE2", "SOPIN", "SLICE0", "SOPOUT"),
        ("SLICE3", "SOPIN", "SLICE1", "SOPOUT"),
    ] {
        if dbel != bel.key {
            continue;
        }
        let obel = vrf.find_bel_sibling(bel, sbel);
        vrf.claim_pip(bel.crd(), bel.wire(dpin), obel.wire(spin));
        vrf.claim_node(&[bel.fwire(dpin)]);
    }
    if bel.key == "SLICE3" {
        // supposed to be connected? idk.
        vrf.claim_node(&[bel.fwire("SHIFTIN")]);

        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, "SLICE3") {
            vrf.verify_node(&[bel.fwire("DIG_S"), obel.fwire("DIG_LOCAL")]);
        }

        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, "SLICE1") {
            vrf.claim_node(&[bel.fwire("FXINB"), obel.fwire("FX_S")]);
            vrf.claim_pip(obel.crd(), obel.wire("FX_S"), obel.wire("FX"));
        } else {
            vrf.claim_node(&[bel.fwire("FXINB")]);
        }
    }
    for (dbel, sbel) in [("SLICE0", "SLICE1"), ("SLICE2", "SLICE3")] {
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
    for (dbel, sbel) in [("SLICE0", "SLICE2"), ("SLICE1", "SLICE3")] {
        if bel.key != dbel {
            continue;
        }
        let mut scol = bel.col - 1;
        if endev.chip.columns[scol].kind == ColumnKind::Bram {
            scol -= 1;
        }
        if let Some(obel) = vrf.find_bel(bel.die, (scol, bel.row), sbel) {
            vrf.claim_node(&[bel.fwire("SOPIN"), obel.fwire("SOPOUT_W")]);
            vrf.claim_pip(obel.crd(), obel.wire("SOPOUT_W"), obel.wire("SOPOUT"));
        } else {
            vrf.claim_node(&[bel.fwire("SOPIN")]);
        }
    }
}

pub fn verify_slice_s3(vrf: &mut Verifier, bel: &BelContext) {
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
    for pin in ["F5", "FX", "COUT"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    if kind == "SLICEM" {
        for pin in ["SHIFTOUT", "DIG", "BYOUT", "BYINVOUT"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
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

pub fn verify_tbus(vrf: &mut Verifier, bel: &BelContext) {
    let obel = vrf.find_bel_sibling(bel, "TBUF0");
    vrf.claim_pip(bel.crd(), bel.wire("BUS0"), obel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("BUS2"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, "TBUF1");
    vrf.claim_pip(bel.crd(), bel.wire("BUS1"), obel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("BUS3"), obel.wire("O"));
    if let Some(obel) = vrf.find_bel_walk(bel, -1, 0, "TBUS") {
        vrf.claim_node(&[bel.fwire("BUS0"), obel.fwire("BUS3_E")]);
        vrf.verify_node(&[bel.fwire("BUS1"), obel.fwire("BUS0")]);
        vrf.verify_node(&[bel.fwire("BUS2"), obel.fwire("BUS1")]);
        vrf.verify_node(&[bel.fwire("BUS3"), obel.fwire("BUS2")]);
    } else {
        for pin in ["BUS0", "BUS1", "BUS2", "BUS3"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
    vrf.claim_pip(bel.crd(), bel.wire("BUS3"), bel.wire("BUS3_E"));
    vrf.claim_pip(bel.crd(), bel.wire("BUS3_E"), bel.wire("BUS3"));
    vrf.claim_pip(bel.crd(), bel.wire("OUT"), bel.wire("BUS2"));
}

pub fn verify_randor(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    vrf.verify_bel(
        bel,
        "RESERVED_ANDOR",
        &[
            ("CIN0", SitePinDir::In),
            ("CIN1", SitePinDir::In),
            ("CPREV", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &[],
    );
    if bel.row == endev.chip.row_bot() {
        for pin in ["CIN0", "CIN1", "CPREV", "O"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    } else {
        for pin in ["CPREV", "O"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
        for (pin, sbel) in [("CIN1", "SLICE2"), ("CIN0", "SLICE3")] {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, sbel) {
                vrf.claim_node(&[bel.fwire(pin), obel.fwire("COUT_N")]);
                vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
            } else {
                vrf.claim_node(&[bel.fwire(pin)]);
            }
        }
        vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
        if let Some(obel) = vrf.find_bel_walk(bel, 1, 0, "RANDOR") {
            vrf.claim_node(&[bel.fwire_far("O"), obel.fwire_far("CPREV")]);
            vrf.claim_pip(obel.crd(), obel.wire("CPREV"), obel.wire_far("CPREV"));
        } else {
            let obel = vrf.find_bel_walk(bel, 1, 0, "RANDOR_OUT").unwrap();
            vrf.verify_node(&[bel.fwire_far("O"), obel.fwire("O")]);
        }
    }
}
