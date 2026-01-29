use prjcombine_interconnect::grid::BelCoord;
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rdverify::Verifier;
use prjcombine_virtex2::{chip::ColumnKind, defs::bslots};

pub fn verify_slice_v2(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::SLICE.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .extra_in("DX")
        .extra_in("DY")
        .extra_in("FXINA")
        .extra_in("FXINB")
        .extra_out("F5")
        .extra_out("FX")
        .extra_in("CIN")
        .extra_out("COUT")
        .extra_in("SHIFTIN")
        .extra_out("SHIFTOUT")
        .extra_in("ALTDIG")
        .extra_out("DIG")
        .extra_in("SLICEWE0")
        .extra_in("SLICEWE1")
        .extra_in("SLICEWE2")
        .extra_out("BXOUT")
        .extra_out("BYOUT")
        .extra_out("BYINVOUT")
        .extra_in("SOPIN")
        .extra_out("SOPOUT")
        .extra_in("WF1")
        .extra_in("WF2")
        .extra_in("WF3")
        .extra_in("WF4")
        .extra_in("WG1")
        .extra_in("WG2")
        .extra_in("WG3")
        .extra_in("WG4");
    bel.claim_net(&[bel.wire("DX")]);
    bel.claim_pip(bel.wire("DX"), bel.wire("X"));
    bel.claim_net(&[bel.wire("DY")]);
    bel.claim_pip(bel.wire("DY"), bel.wire("Y"));
    for pin in [
        "F5", "FX", "COUT", "SHIFTOUT", "DIG", "BYOUT", "BXOUT", "BYINVOUT", "SOPOUT",
    ] {
        bel.claim_net(&[bel.wire(pin)]);
    }
    for (dbel, dpin, sbel, spin) in [
        (bslots::SLICE[0], "FXINA", bslots::SLICE[0], "F5"),
        (bslots::SLICE[0], "FXINB", bslots::SLICE[1], "F5"),
        (bslots::SLICE[1], "FXINA", bslots::SLICE[0], "FX"),
        (bslots::SLICE[1], "FXINB", bslots::SLICE[2], "FX"),
        (bslots::SLICE[2], "FXINA", bslots::SLICE[2], "F5"),
        (bslots::SLICE[2], "FXINB", bslots::SLICE[3], "F5"),
        (bslots::SLICE[3], "FXINA", bslots::SLICE[1], "FX"),
        // SLICE3 FXINB <- top's SLICE1 FX

        // SLICE0 CIN <- bot's SLICE1 COUT
        (bslots::SLICE[1], "CIN", bslots::SLICE[0], "COUT"),
        // SLICE2 CIN <- bot's SLICE3 COUT
        (bslots::SLICE[3], "CIN", bslots::SLICE[2], "COUT"),
        (bslots::SLICE[0], "SHIFTIN", bslots::SLICE[1], "SHIFTOUT"),
        (bslots::SLICE[1], "SHIFTIN", bslots::SLICE[2], "SHIFTOUT"),
        (bslots::SLICE[2], "SHIFTIN", bslots::SLICE[3], "SHIFTOUT"),
        // SLICE3 SHIFTIN disconnected? supposed to be top's SLICE0 SHIFTOUT?
        (bslots::SLICE[3], "DIG_LOCAL", bslots::SLICE[3], "DIG"),
        (bslots::SLICE[0], "ALTDIG", bslots::SLICE[1], "DIG"),
        (bslots::SLICE[1], "ALTDIG", bslots::SLICE[3], "DIG_LOCAL"),
        (bslots::SLICE[2], "ALTDIG", bslots::SLICE[3], "DIG_LOCAL"),
        (bslots::SLICE[3], "ALTDIG", bslots::SLICE[3], "DIG_S"), // top's SLICE3 DIG
        (bslots::SLICE[1], "BYOUT_LOCAL", bslots::SLICE[1], "BYOUT"),
        (
            bslots::SLICE[0],
            "BYINVOUT_LOCAL",
            bslots::SLICE[0],
            "BYINVOUT",
        ),
        (
            bslots::SLICE[1],
            "BYINVOUT_LOCAL",
            bslots::SLICE[1],
            "BYINVOUT",
        ),
        (bslots::SLICE[0], "SLICEWE0", bslots::SLICE[0], "BXOUT"),
        (bslots::SLICE[1], "SLICEWE0", bslots::SLICE[1], "BXOUT"),
        (bslots::SLICE[2], "SLICEWE0", bslots::SLICE[0], "BXOUT"),
        (bslots::SLICE[3], "SLICEWE0", bslots::SLICE[1], "BXOUT"),
        (bslots::SLICE[0], "SLICEWE1", bslots::SLICE[0], "BYOUT"),
        (
            bslots::SLICE[1],
            "SLICEWE1",
            bslots::SLICE[0],
            "BYINVOUT_LOCAL",
        ),
        (bslots::SLICE[2], "SLICEWE1", bslots::SLICE[0], "BYOUT"),
        (
            bslots::SLICE[3],
            "SLICEWE1",
            bslots::SLICE[0],
            "BYINVOUT_LOCAL",
        ),
        (
            bslots::SLICE[0],
            "SLICEWE2",
            bslots::SLICE[1],
            "BYOUT_LOCAL",
        ),
        (
            bslots::SLICE[1],
            "SLICEWE2",
            bslots::SLICE[1],
            "BYOUT_LOCAL",
        ),
        (
            bslots::SLICE[2],
            "SLICEWE2",
            bslots::SLICE[1],
            "BYINVOUT_LOCAL",
        ),
        (
            bslots::SLICE[3],
            "SLICEWE2",
            bslots::SLICE[1],
            "BYINVOUT_LOCAL",
        ),
        // SLICE0 SOPIN <- left's SLICE2 SOPOUT
        // SLICE1 SOPIN <- left's SLICE3 SOPOUT
        (bslots::SLICE[2], "SOPIN", bslots::SLICE[0], "SOPOUT"),
        (bslots::SLICE[3], "SOPIN", bslots::SLICE[1], "SOPOUT"),
    ] {
        if dbel != bcrd.slot {
            continue;
        }
        let obel = bcrd.bel(sbel);
        bel.claim_pip(bel.wire(dpin), bel.bel_wire(obel, spin));
        bel.claim_net(&[bel.wire(dpin)]);
    }
    for (pin, spin) in [
        ("WF1", "F1"),
        ("WF2", "F2"),
        ("WF3", "F3"),
        ("WF4", "F4"),
        ("WG1", "G1"),
        ("WG2", "G2"),
        ("WG3", "G3"),
        ("WG4", "G4"),
    ] {
        let obel = bcrd.bel(bslots::SLICE[idx & 1]);
        bel.claim_pip(bel.wire(pin), bel.bel_wire_far(obel, spin));
        bel.claim_net(&[bel.wire(pin)]);
    }
    if bcrd.slot == bslots::SLICE[3] {
        // supposed to be connected? idk.
        bel.claim_net(&[bel.wire("SHIFTIN")]);

        let obel = bcrd.delta(0, 1).bel(bslots::SLICE[3]);
        if endev.edev.has_bel(obel) {
            bel.verify_net(&[bel.wire("DIG_S"), bel.bel_wire(obel, "DIG_LOCAL")]);
        }

        let obel = bcrd.delta(0, 1).bel(bslots::SLICE[1]);
        if endev.edev.has_bel(obel) {
            bel.claim_net(&[bel.wire("FXINB"), bel.bel_wire(obel, "FX_S")]);
            bel.claim_pip(bel.bel_wire(obel, "FX_S"), bel.bel_wire(obel, "FX"));
        } else {
            bel.claim_net(&[bel.wire("FXINB")]);
        }
    }
    for (dbel, sbel) in [
        (bslots::SLICE[0], bslots::SLICE[1]),
        (bslots::SLICE[2], bslots::SLICE[3]),
    ] {
        if bcrd.slot != dbel {
            continue;
        }
        let obel = bcrd.delta(0, -1).bel(sbel);
        if endev.edev.has_bel(obel) {
            bel.claim_net(&[bel.wire("CIN"), bel.bel_wire(obel, "COUT_N")]);
            bel.claim_pip(bel.bel_wire(obel, "COUT_N"), bel.bel_wire(obel, "COUT"));
        } else {
            bel.claim_net(&[bel.wire("CIN")]);
        }
    }
    for (dbel, sbel) in [
        (bslots::SLICE[0], bslots::SLICE[2]),
        (bslots::SLICE[1], bslots::SLICE[3]),
    ] {
        if bcrd.slot != dbel {
            continue;
        }
        let mut scol = bcrd.col - 1;
        if endev.chip.columns[scol].kind == ColumnKind::Bram {
            scol -= 1;
        }
        let obel = bcrd.with_col(scol).bel(sbel);
        if endev.edev.has_bel(obel) {
            bel.claim_net(&[bel.wire("SOPIN"), bel.bel_wire(obel, "SOPOUT_W")]);
            bel.claim_pip(bel.bel_wire(obel, "SOPOUT_W"), bel.bel_wire(obel, "SOPOUT"));
        } else {
            bel.claim_net(&[bel.wire("SOPIN")]);
        }
    }
    bel.commit();
}

pub fn verify_slice_s3(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::SLICE.index_of(bcrd.slot).unwrap();
    let is_slicem = matches!(idx, 0 | 2);
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(if is_slicem { "SLICEM" } else { "SLICEL" })
        .extra_in("FXINA")
        .extra_in("FXINB")
        .extra_out("F5")
        .extra_out("FX")
        .extra_in("CIN")
        .extra_out("COUT");
    for pin in ["F5", "FX", "COUT"] {
        bel.claim_net(&[bel.wire(pin)]);
    }

    if is_slicem {
        bel = bel
            .extra_in("SHIFTIN")
            .extra_out("SHIFTOUT")
            .extra_in("ALTDIG")
            .extra_out("DIG")
            .extra_in("SLICEWE1")
            .extra_out("BYOUT")
            .extra_out("BYINVOUT");
        for pin in ["SHIFTOUT", "DIG", "BYOUT", "BYINVOUT"] {
            bel.claim_net(&[bel.wire(pin)]);
        }
    }
    for (dbel, dpin, sbel, spin) in [
        (bslots::SLICE[0], "FXINA", bslots::SLICE[0], "F5"),
        (bslots::SLICE[0], "FXINB", bslots::SLICE[2], "F5"),
        (bslots::SLICE[1], "FXINA", bslots::SLICE[1], "F5"),
        (bslots::SLICE[1], "FXINB", bslots::SLICE[3], "F5"),
        (bslots::SLICE[2], "FXINA", bslots::SLICE[0], "FX"),
        (bslots::SLICE[2], "FXINB", bslots::SLICE[1], "FX"),
        (bslots::SLICE[3], "FXINA", bslots::SLICE[2], "FX"),
        // SLICE3 FXINB <- top's SLICE2 FX

        // SLICE0 CIN <- bot's SLICE2 COUT
        // SLICE1 CIN <- bot's SLICE3 COUT
        (bslots::SLICE[2], "CIN", bslots::SLICE[0], "COUT"),
        (bslots::SLICE[3], "CIN", bslots::SLICE[1], "COUT"),
        (bslots::SLICE[0], "SHIFTIN", bslots::SLICE[2], "SHIFTOUT"),
        // SLICE2 SHIFTIN disconnected?
        (bslots::SLICE[0], "ALTDIG", bslots::SLICE[2], "DIG"),
        // SLICE2 ALTDIG disconnected?
        (bslots::SLICE[0], "SLICEWE1", bslots::SLICE[0], "BYOUT"),
        (bslots::SLICE[2], "SLICEWE1", bslots::SLICE[0], "BYINVOUT"),
    ] {
        if dbel != bcrd.slot {
            continue;
        }
        let obel = bcrd.bel(sbel);
        bel.claim_pip(bel.wire(dpin), bel.bel_wire(obel, spin));
        bel.claim_net(&[bel.wire(dpin)]);
    }
    if idx == 2 {
        bel.claim_net(&[bel.wire("SHIFTIN")]);
        bel.claim_net(&[bel.wire("ALTDIG")]);
    }
    if idx == 3 {
        let obel = bcrd.delta(0, 1).bel(bslots::SLICE[2]);
        if endev.edev.has_bel(obel) {
            bel.claim_net(&[bel.wire("FXINB"), bel.bel_wire(obel, "FX_S")]);
            bel.claim_pip(bel.bel_wire(obel, "FX_S"), bel.bel_wire(obel, "FX"));
        } else {
            bel.claim_net(&[bel.wire("FXINB")]);
        }
    }
    for (dbel, sbel) in [
        (bslots::SLICE[0], bslots::SLICE[2]),
        (bslots::SLICE[1], bslots::SLICE[3]),
    ] {
        if bcrd.slot != dbel {
            continue;
        }
        let obel = bcrd.delta(0, -1).bel(sbel);
        if endev.edev.has_bel(obel) {
            bel.claim_net(&[bel.wire("CIN"), bel.bel_wire(obel, "COUT_N")]);
            bel.claim_pip(bel.bel_wire(obel, "COUT_N"), bel.bel_wire(obel, "COUT"));
        } else {
            bel.claim_net(&[bel.wire("CIN")]);
        }
    }
    bel.commit();
}

pub fn verify_tbus(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd);
    let obel = bcrd.bel(bslots::TBUF[0]);
    bel.claim_pip(bel.wire("BUS0"), bel.bel_wire(obel, "O"));
    bel.claim_pip(bel.wire("BUS2"), bel.bel_wire(obel, "O"));
    let obel = bcrd.bel(bslots::TBUF[1]);
    bel.claim_pip(bel.wire("BUS1"), bel.bel_wire(obel, "O"));
    bel.claim_pip(bel.wire("BUS3"), bel.bel_wire(obel, "O"));
    let mut obel = bcrd;
    obel.col -= 1;
    while !endev.edev.has_bel(obel) && obel.col != endev.edev.chip.col_w() {
        obel.col -= 1;
    }
    if endev.edev.has_bel(obel) {
        bel.claim_net(&[bel.wire("BUS0"), bel.bel_wire(obel, "BUS3_E")]);
        bel.verify_net(&[bel.wire("BUS1"), bel.bel_wire(obel, "BUS0")]);
        bel.verify_net(&[bel.wire("BUS2"), bel.bel_wire(obel, "BUS1")]);
        bel.verify_net(&[bel.wire("BUS3"), bel.bel_wire(obel, "BUS2")]);
    } else {
        for pin in ["BUS0", "BUS1", "BUS2", "BUS3"] {
            bel.claim_net(&[bel.wire(pin)]);
        }
    }
    bel.claim_pip(bel.wire("BUS3"), bel.wire("BUS3_E"));
    bel.claim_pip(bel.wire("BUS3_E"), bel.wire("BUS3"));
    bel.claim_pip(bel.wire("OUT"), bel.wire("BUS2"));
}

pub fn verify_randor(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("RESERVED_ANDOR")
        .extra_in("CIN0")
        .extra_in("CIN1")
        .extra_in("CPREV")
        .extra_out("O");
    if bcrd.row == endev.chip.row_s() {
        for pin in ["CIN0", "CIN1", "CPREV", "O"] {
            bel.claim_net(&[bel.wire(pin)]);
        }
    } else {
        for pin in ["CPREV", "O"] {
            bel.claim_net(&[bel.wire(pin)]);
        }
        for (pin, sbel) in [("CIN1", bslots::SLICE[2]), ("CIN0", bslots::SLICE[3])] {
            let obel = bcrd.delta(0, -1).bel(sbel);
            if endev.edev.has_bel(obel) {
                bel.claim_net(&[bel.wire(pin), bel.bel_wire(obel, "COUT_N")]);
                bel.claim_pip(bel.bel_wire(obel, "COUT_N"), bel.bel_wire(obel, "COUT"));
            } else {
                bel.claim_net(&[bel.wire(pin)]);
            }
        }
        bel.claim_pip(bel.wire_far("O"), bel.wire("O"));
        let mut obel = bcrd.delta(1, 0).bel(bslots::RANDOR);
        while obel.col != endev.edev.chip.col_e() && !endev.edev.has_bel(obel) {
            obel.col += 1;
        }
        if endev.edev.has_bel(obel) {
            bel.claim_net(&[bel.wire_far("O"), bel.bel_wire_far(obel, "CPREV")]);
            bel.claim_pip(bel.bel_wire(obel, "CPREV"), bel.bel_wire_far(obel, "CPREV"));
        } else {
            let obel = obel.bel(bslots::RANDOR_OUT);
            bel.verify_net(&[bel.wire_far("O"), bel.bel_wire(obel, "O")]);
        }
    }
    bel.commit();
}
