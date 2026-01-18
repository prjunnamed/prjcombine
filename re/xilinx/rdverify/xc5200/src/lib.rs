use prjcombine_interconnect::grid::BelCoord;
use prjcombine_re_xilinx_naming_xc2000::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{SitePin, SitePinDir, Verifier};
use prjcombine_xc2000::xc5200::bslots;

fn verify_lc(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::LC.index_of(bcrd.slot).unwrap();
    let kind = match idx {
        0 | 2 => "LC5A",
        1 | 3 => "LC5B",
        _ => unreachable!(),
    };
    let mut bvrf = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_in("CI")
        .extra_out("CO");
    if kind == "LC5A" {
        bvrf = bvrf.extra_in("F5I");
        let oslot = match idx {
            0 => bslots::LC[1],
            2 => bslots::LC[3],
            _ => unreachable!(),
        };
        bvrf.claim_net(&[bvrf.fwire("F5I")]);
        let obel = bcrd.bel(oslot);
        bvrf.claim_pip(bvrf.crd(), bvrf.wire("F5I"), bvrf.bel_wire(obel, "X"));
    }
    bvrf.claim_net(&[bvrf.fwire("CI")]);
    bvrf.claim_net(&[bvrf.fwire("CO")]);
    if bcrd.slot == bslots::LC[0] {
        bvrf.claim_pip(bvrf.crd(), bvrf.wire("CI"), bvrf.wire_far("CI"));
        let obel = bcrd.delta(0, -1).bel(bslots::LC[3]);
        if endev.edev.has_bel(obel) {
            bvrf.claim_net(&[bvrf.fwire_far("CI"), bvrf.bel_fwire_far(obel, "CO")]);
        } else {
            let obel = bcrd.delta(0, -1).bel(bslots::CIN);
            bvrf.verify_net(&[bvrf.fwire_far("CI"), bvrf.bel_fwire(obel, "IN")]);
        }
    } else {
        let okey = match idx {
            1 => bslots::LC[0],
            2 => bslots::LC[1],
            3 => bslots::LC[2],
            _ => unreachable!(),
        };
        let obel = bcrd.bel(okey);
        bvrf.claim_pip(bvrf.crd(), bvrf.wire("CI"), bvrf.bel_wire(obel, "CO"));
    }
    if bcrd.slot == bslots::LC[3] {
        bvrf.claim_pip(bvrf.crd(), bvrf.wire_far("CO"), bvrf.wire("CO"));
    }
    bvrf.commit();
}

fn verify_iob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bvrf = vrf.verify_bel(bcrd);
    let kind = if bvrf.naming.pins.contains_key("CLKIN") {
        bvrf = bvrf.extra_out("CLKIN");
        let (col, row) = if bcrd.row == endev.edev.chip.row_s() {
            (endev.edev.chip.col_w(), endev.edev.chip.row_s())
        } else if bcrd.row == endev.edev.chip.row_n() {
            (endev.edev.chip.col_e(), endev.edev.chip.row_n())
        } else if bcrd.col == endev.edev.chip.col_w() {
            (endev.edev.chip.col_w(), endev.edev.chip.row_n())
        } else if bcrd.col == endev.edev.chip.col_e() {
            (endev.edev.chip.col_e(), endev.edev.chip.row_s())
        } else {
            unreachable!()
        };
        let obel = bcrd.cell.with_cr(col, row).bel(bslots::CLKIOB);
        bvrf.verify_net(&[bvrf.fwire("CLKIN"), bvrf.bel_fwire(obel, "OUT")]);
        "CLKIOB"
    } else {
        "IOB"
    };
    bvrf.kind(kind).commit();
}

fn verify_bufg(vrf: &mut Verifier, bcrd: BelCoord) {
    let name = vrf.ngrid.get_bel_name(bcrd).unwrap();
    vrf.claim_pip(
        vrf.bel_rcrd(bcrd),
        vrf.bel_wire_far(bcrd, "O"),
        vrf.bel_wire(bcrd, "O"),
    );
    vrf.claim_net(&[vrf.bel_fwire(bcrd, "O")]);
    vrf.claim_site(
        vrf.bel_rcrd(bcrd),
        name,
        "CLK",
        &[
            SitePin {
                dir: SitePinDir::In,
                pin: "I".into(),
                wire: Some(vrf.bel_wire(bcrd, "I")),
            },
            SitePin {
                dir: SitePinDir::Out,
                pin: "O".into(),
                wire: Some(vrf.bel_wire(bcrd, "O")),
            },
        ],
    );
}

fn verify_top_cout(vrf: &mut Verifier, bcrd: BelCoord) {
    let obel = bcrd.delta(0, -1).bel(bslots::LC[3]);
    vrf.verify_net(&[vrf.bel_fwire(bcrd, "OUT"), vrf.bel_fwire_far(obel, "CO")]);
    // artifact of unbuffered pip representation — disregard
    vrf.claim_pip(vrf.bel_rcrd(bcrd), "WIRE_COUT_TOP", "WIRE_M14_TOP");
}

fn verify_bot_cin(vrf: &mut Verifier, bcrd: BelCoord) {
    // artifact of unbuffered pip representation — disregard
    vrf.claim_pip(vrf.bel_rcrd(bcrd), "WIRE_M14_BOT", "WIRE_COUT_BOT");
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    match bcrd.slot {
        _ if bslots::LC.contains(bcrd.slot) => verify_lc(endev, vrf, bcrd),
        _ if bslots::IO.contains(bcrd.slot) => verify_iob(endev, vrf, bcrd),
        _ if bslots::TBUF.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        bslots::BUFG => verify_bufg(vrf, bcrd),
        bslots::CLKIOB => (),
        bslots::COUT => verify_top_cout(vrf, bcrd),
        bslots::CIN => verify_bot_cin(vrf, bcrd),
        bslots::SCANTEST => (),
        bslots::RDBK
        | bslots::STARTUP
        | bslots::BSCAN
        | bslots::OSC
        | bslots::BYPOSC
        | bslots::BSUPD
        | bslots::VCC_GND => vrf.verify_bel(bcrd).commit(),
        _ => println!("MEOW {}", bcrd.to_string(endev.edev.db)),
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);
    vrf.skip_sb(bslots::BUFG);
    vrf.prep_int_wires();
    vrf.handle_int();
    for (tcrd, tile) in endev.edev.tiles() {
        let tcls = &endev.edev.db[tile.class];
        for slot in tcls.bels.ids() {
            if matches!(slot, bslots::INT | bslots::LLV | bslots::LLH | bslots::BUFR) {
                continue;
            }
            verify_bel(endev, &mut vrf, tcrd.bel(slot));
        }
    }
    vrf.finish();
}
