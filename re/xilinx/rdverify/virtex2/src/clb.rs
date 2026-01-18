use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rdverify::{LegacyBelContext, SitePinDir, Verifier};
use prjcombine_virtex2::{chip::ColumnKind, defs};

pub fn verify_slice_v2(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext) {
    vrf.verify_legacy_bel(
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
    vrf.claim_net(&[bel.fwire("DX")]);
    vrf.claim_pip(bel.crd(), bel.wire("DX"), bel.wire("X"));
    vrf.claim_net(&[bel.fwire("DY")]);
    vrf.claim_pip(bel.crd(), bel.wire("DY"), bel.wire("Y"));
    for pin in [
        "F5", "FX", "COUT", "SHIFTOUT", "DIG", "BYOUT", "BXOUT", "BYINVOUT", "SOPOUT",
    ] {
        vrf.claim_net(&[bel.fwire(pin)]);
    }
    for (dbel, dpin, sbel, spin) in [
        (
            defs::bslots::SLICE[0],
            "FXINA",
            defs::bslots::SLICE[0],
            "F5",
        ),
        (
            defs::bslots::SLICE[0],
            "FXINB",
            defs::bslots::SLICE[1],
            "F5",
        ),
        (
            defs::bslots::SLICE[1],
            "FXINA",
            defs::bslots::SLICE[0],
            "FX",
        ),
        (
            defs::bslots::SLICE[1],
            "FXINB",
            defs::bslots::SLICE[2],
            "FX",
        ),
        (
            defs::bslots::SLICE[2],
            "FXINA",
            defs::bslots::SLICE[2],
            "F5",
        ),
        (
            defs::bslots::SLICE[2],
            "FXINB",
            defs::bslots::SLICE[3],
            "F5",
        ),
        (
            defs::bslots::SLICE[3],
            "FXINA",
            defs::bslots::SLICE[1],
            "FX",
        ),
        // SLICE3 FXINB <- top's SLICE1 FX

        // SLICE0 CIN <- bot's SLICE1 COUT
        (
            defs::bslots::SLICE[1],
            "CIN",
            defs::bslots::SLICE[0],
            "COUT",
        ),
        // SLICE2 CIN <- bot's SLICE3 COUT
        (
            defs::bslots::SLICE[3],
            "CIN",
            defs::bslots::SLICE[2],
            "COUT",
        ),
        (
            defs::bslots::SLICE[0],
            "SHIFTIN",
            defs::bslots::SLICE[1],
            "SHIFTOUT",
        ),
        (
            defs::bslots::SLICE[1],
            "SHIFTIN",
            defs::bslots::SLICE[2],
            "SHIFTOUT",
        ),
        (
            defs::bslots::SLICE[2],
            "SHIFTIN",
            defs::bslots::SLICE[3],
            "SHIFTOUT",
        ),
        // SLICE3 SHIFTIN disconnected? supposed to be top's SLICE0 SHIFTOUT?
        (
            defs::bslots::SLICE[3],
            "DIG_LOCAL",
            defs::bslots::SLICE[3],
            "DIG",
        ),
        (
            defs::bslots::SLICE[0],
            "ALTDIG",
            defs::bslots::SLICE[1],
            "DIG",
        ),
        (
            defs::bslots::SLICE[1],
            "ALTDIG",
            defs::bslots::SLICE[3],
            "DIG_LOCAL",
        ),
        (
            defs::bslots::SLICE[2],
            "ALTDIG",
            defs::bslots::SLICE[3],
            "DIG_LOCAL",
        ),
        (
            defs::bslots::SLICE[3],
            "ALTDIG",
            defs::bslots::SLICE[3],
            "DIG_S",
        ), // top's SLICE3 DIG
        (
            defs::bslots::SLICE[1],
            "BYOUT_LOCAL",
            defs::bslots::SLICE[1],
            "BYOUT",
        ),
        (
            defs::bslots::SLICE[0],
            "BYINVOUT_LOCAL",
            defs::bslots::SLICE[0],
            "BYINVOUT",
        ),
        (
            defs::bslots::SLICE[1],
            "BYINVOUT_LOCAL",
            defs::bslots::SLICE[1],
            "BYINVOUT",
        ),
        (
            defs::bslots::SLICE[0],
            "SLICEWE0",
            defs::bslots::SLICE[0],
            "BXOUT",
        ),
        (
            defs::bslots::SLICE[1],
            "SLICEWE0",
            defs::bslots::SLICE[1],
            "BXOUT",
        ),
        (
            defs::bslots::SLICE[2],
            "SLICEWE0",
            defs::bslots::SLICE[0],
            "BXOUT",
        ),
        (
            defs::bslots::SLICE[3],
            "SLICEWE0",
            defs::bslots::SLICE[1],
            "BXOUT",
        ),
        (
            defs::bslots::SLICE[0],
            "SLICEWE1",
            defs::bslots::SLICE[0],
            "BYOUT",
        ),
        (
            defs::bslots::SLICE[1],
            "SLICEWE1",
            defs::bslots::SLICE[0],
            "BYINVOUT_LOCAL",
        ),
        (
            defs::bslots::SLICE[2],
            "SLICEWE1",
            defs::bslots::SLICE[0],
            "BYOUT",
        ),
        (
            defs::bslots::SLICE[3],
            "SLICEWE1",
            defs::bslots::SLICE[0],
            "BYINVOUT_LOCAL",
        ),
        (
            defs::bslots::SLICE[0],
            "SLICEWE2",
            defs::bslots::SLICE[1],
            "BYOUT_LOCAL",
        ),
        (
            defs::bslots::SLICE[1],
            "SLICEWE2",
            defs::bslots::SLICE[1],
            "BYOUT_LOCAL",
        ),
        (
            defs::bslots::SLICE[2],
            "SLICEWE2",
            defs::bslots::SLICE[1],
            "BYINVOUT_LOCAL",
        ),
        (
            defs::bslots::SLICE[3],
            "SLICEWE2",
            defs::bslots::SLICE[1],
            "BYINVOUT_LOCAL",
        ),
        // SLICE0 SOPIN <- left's SLICE2 SOPOUT
        // SLICE1 SOPIN <- left's SLICE3 SOPOUT
        (
            defs::bslots::SLICE[2],
            "SOPIN",
            defs::bslots::SLICE[0],
            "SOPOUT",
        ),
        (
            defs::bslots::SLICE[3],
            "SOPIN",
            defs::bslots::SLICE[1],
            "SOPOUT",
        ),
    ] {
        if dbel != bel.slot {
            continue;
        }
        let obel = vrf.find_bel_sibling(bel, sbel);
        vrf.claim_pip(bel.crd(), bel.wire(dpin), obel.wire(spin));
        vrf.claim_net(&[bel.fwire(dpin)]);
    }
    if bel.slot == defs::bslots::SLICE[3] {
        // supposed to be connected? idk.
        vrf.claim_net(&[bel.fwire("SHIFTIN")]);

        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, defs::bslots::SLICE[3]) {
            vrf.verify_net(&[bel.fwire("DIG_S"), obel.fwire("DIG_LOCAL")]);
        }

        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, defs::bslots::SLICE[1]) {
            vrf.claim_net(&[bel.fwire("FXINB"), obel.fwire("FX_S")]);
            vrf.claim_pip(obel.crd(), obel.wire("FX_S"), obel.wire("FX"));
        } else {
            vrf.claim_net(&[bel.fwire("FXINB")]);
        }
    }
    for (dbel, sbel) in [
        (defs::bslots::SLICE[0], defs::bslots::SLICE[1]),
        (defs::bslots::SLICE[2], defs::bslots::SLICE[3]),
    ] {
        if bel.slot != dbel {
            continue;
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, sbel) {
            vrf.claim_net(&[bel.fwire("CIN"), obel.fwire("COUT_N")]);
            vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
        } else {
            vrf.claim_net(&[bel.fwire("CIN")]);
        }
    }
    for (dbel, sbel) in [
        (defs::bslots::SLICE[0], defs::bslots::SLICE[2]),
        (defs::bslots::SLICE[1], defs::bslots::SLICE[3]),
    ] {
        if bel.slot != dbel {
            continue;
        }
        let mut scol = bel.col - 1;
        if endev.chip.columns[scol].kind == ColumnKind::Bram {
            scol -= 1;
        }
        if let Some(obel) = vrf.find_bel(bel.cell.with_col(scol).bel(sbel)) {
            vrf.claim_net(&[bel.fwire("SOPIN"), obel.fwire("SOPOUT_W")]);
            vrf.claim_pip(obel.crd(), obel.wire("SOPOUT_W"), obel.wire("SOPOUT"));
        } else {
            vrf.claim_net(&[bel.fwire("SOPIN")]);
        }
    }
}

pub fn verify_slice_s3(vrf: &mut Verifier, bel: &LegacyBelContext) {
    let idx = defs::bslots::SLICE.index_of(bel.slot).unwrap();
    let kind = if matches!(idx, 0 | 2) {
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
    vrf.verify_legacy_bel(bel, kind, &pins, &[]);
    for pin in ["F5", "FX", "COUT"] {
        vrf.claim_net(&[bel.fwire(pin)]);
    }
    if kind == "SLICEM" {
        for pin in ["SHIFTOUT", "DIG", "BYOUT", "BYINVOUT"] {
            vrf.claim_net(&[bel.fwire(pin)]);
        }
    }
    for (dbel, dpin, sbel, spin) in [
        (
            defs::bslots::SLICE[0],
            "FXINA",
            defs::bslots::SLICE[0],
            "F5",
        ),
        (
            defs::bslots::SLICE[0],
            "FXINB",
            defs::bslots::SLICE[2],
            "F5",
        ),
        (
            defs::bslots::SLICE[1],
            "FXINA",
            defs::bslots::SLICE[1],
            "F5",
        ),
        (
            defs::bslots::SLICE[1],
            "FXINB",
            defs::bslots::SLICE[3],
            "F5",
        ),
        (
            defs::bslots::SLICE[2],
            "FXINA",
            defs::bslots::SLICE[0],
            "FX",
        ),
        (
            defs::bslots::SLICE[2],
            "FXINB",
            defs::bslots::SLICE[1],
            "FX",
        ),
        (
            defs::bslots::SLICE[3],
            "FXINA",
            defs::bslots::SLICE[2],
            "FX",
        ),
        // SLICE3 FXINB <- top's SLICE2 FX

        // SLICE0 CIN <- bot's SLICE2 COUT
        // SLICE1 CIN <- bot's SLICE3 COUT
        (
            defs::bslots::SLICE[2],
            "CIN",
            defs::bslots::SLICE[0],
            "COUT",
        ),
        (
            defs::bslots::SLICE[3],
            "CIN",
            defs::bslots::SLICE[1],
            "COUT",
        ),
        (
            defs::bslots::SLICE[0],
            "SHIFTIN",
            defs::bslots::SLICE[2],
            "SHIFTOUT",
        ),
        // SLICE2 SHIFTIN disconnected?
        (
            defs::bslots::SLICE[0],
            "ALTDIG",
            defs::bslots::SLICE[2],
            "DIG",
        ),
        // SLICE2 ALTDIG disconnected?
        (
            defs::bslots::SLICE[0],
            "SLICEWE1",
            defs::bslots::SLICE[0],
            "BYOUT",
        ),
        (
            defs::bslots::SLICE[2],
            "SLICEWE1",
            defs::bslots::SLICE[0],
            "BYINVOUT",
        ),
    ] {
        if dbel != bel.slot {
            continue;
        }
        let obel = vrf.find_bel_sibling(bel, sbel);
        vrf.claim_pip(bel.crd(), bel.wire(dpin), obel.wire(spin));
        vrf.claim_net(&[bel.fwire(dpin)]);
    }
    if bel.slot == defs::bslots::SLICE[2] {
        vrf.claim_net(&[bel.fwire("SHIFTIN")]);
        vrf.claim_net(&[bel.fwire("ALTDIG")]);
    }
    if bel.slot == defs::bslots::SLICE[3] {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, defs::bslots::SLICE[2]) {
            vrf.claim_net(&[bel.fwire("FXINB"), obel.fwire("FX_S")]);
            vrf.claim_pip(obel.crd(), obel.wire("FX_S"), obel.wire("FX"));
        } else {
            vrf.claim_net(&[bel.fwire("FXINB")]);
        }
    }
    for (dbel, sbel) in [
        (defs::bslots::SLICE[0], defs::bslots::SLICE[2]),
        (defs::bslots::SLICE[1], defs::bslots::SLICE[3]),
    ] {
        if bel.slot != dbel {
            continue;
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, sbel) {
            vrf.claim_net(&[bel.fwire("CIN"), obel.fwire("COUT_N")]);
            vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
        } else {
            vrf.claim_net(&[bel.fwire("CIN")]);
        }
    }
}

pub fn verify_tbus(vrf: &mut Verifier, bel: &LegacyBelContext) {
    let obel = vrf.find_bel_sibling(bel, defs::bslots::TBUF[0]);
    vrf.claim_pip(bel.crd(), bel.wire("BUS0"), obel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("BUS2"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::TBUF[1]);
    vrf.claim_pip(bel.crd(), bel.wire("BUS1"), obel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("BUS3"), obel.wire("O"));
    if let Some(obel) = vrf.find_bel_walk(bel, -1, 0, defs::bslots::TBUS) {
        vrf.claim_net(&[bel.fwire("BUS0"), obel.fwire("BUS3_E")]);
        vrf.verify_net(&[bel.fwire("BUS1"), obel.fwire("BUS0")]);
        vrf.verify_net(&[bel.fwire("BUS2"), obel.fwire("BUS1")]);
        vrf.verify_net(&[bel.fwire("BUS3"), obel.fwire("BUS2")]);
    } else {
        for pin in ["BUS0", "BUS1", "BUS2", "BUS3"] {
            vrf.claim_net(&[bel.fwire(pin)]);
        }
    }
    vrf.claim_pip(bel.crd(), bel.wire("BUS3"), bel.wire("BUS3_E"));
    vrf.claim_pip(bel.crd(), bel.wire("BUS3_E"), bel.wire("BUS3"));
    vrf.claim_pip(bel.crd(), bel.wire("OUT"), bel.wire("BUS2"));
}

pub fn verify_randor(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext) {
    vrf.verify_legacy_bel(
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
    if bel.row == endev.chip.row_s() {
        for pin in ["CIN0", "CIN1", "CPREV", "O"] {
            vrf.claim_net(&[bel.fwire(pin)]);
        }
    } else {
        for pin in ["CPREV", "O"] {
            vrf.claim_net(&[bel.fwire(pin)]);
        }
        for (pin, sbel) in [
            ("CIN1", defs::bslots::SLICE[2]),
            ("CIN0", defs::bslots::SLICE[3]),
        ] {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, sbel) {
                vrf.claim_net(&[bel.fwire(pin), obel.fwire("COUT_N")]);
                vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
            } else {
                vrf.claim_net(&[bel.fwire(pin)]);
            }
        }
        vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
        if let Some(obel) = vrf.find_bel_walk(bel, 1, 0, defs::bslots::RANDOR) {
            vrf.claim_net(&[bel.fwire_far("O"), obel.fwire_far("CPREV")]);
            vrf.claim_pip(obel.crd(), obel.wire("CPREV"), obel.wire_far("CPREV"));
        } else {
            let obel = vrf
                .find_bel_walk(bel, 1, 0, defs::bslots::RANDOR_OUT)
                .unwrap();
            vrf.verify_net(&[bel.fwire_far("O"), obel.fwire("O")]);
        }
    }
}
