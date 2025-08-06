use prjcombine_re_xilinx_naming_xc2000::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};
use prjcombine_xc2000::bels::xc5200 as bels;

fn verify_lc(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = match bel.slot {
        bels::LC0 | bels::LC2 => "LC5A",
        bels::LC1 | bels::LC3 => "LC5B",
        _ => unreachable!(),
    };
    let mut pins = vec![("CI", SitePinDir::In), ("CO", SitePinDir::Out)];
    if kind == "LC5A" {
        pins.push(("F5I", SitePinDir::In));
        let oslot = match bel.slot {
            bels::LC0 => bels::LC1,
            bels::LC2 => bels::LC3,
            _ => unreachable!(),
        };
        vrf.claim_net(&[bel.fwire("F5I")]);
        let obel = vrf.find_bel_sibling(bel, oslot);
        vrf.claim_pip(bel.crd(), bel.wire("F5I"), obel.wire("X"));
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
    vrf.claim_net(&[bel.fwire("CI")]);
    vrf.claim_net(&[bel.fwire("CO")]);
    if bel.slot == bels::LC0 {
        vrf.claim_pip(bel.crd(), bel.wire("CI"), bel.wire_far("CI"));
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bels::LC3) {
            vrf.claim_net(&[bel.fwire_far("CI"), obel.fwire_far("CO")]);
        } else {
            let obel = vrf.find_bel_delta(bel, 0, -1, bels::CIN).unwrap();
            vrf.verify_net(&[bel.fwire_far("CI"), obel.fwire("IN")]);
        }
    } else {
        let okey = match bel.slot {
            bels::LC1 => bels::LC0,
            bels::LC2 => bels::LC1,
            bels::LC3 => bels::LC2,
            _ => unreachable!(),
        };
        let obel = vrf.find_bel_sibling(bel, okey);
        vrf.claim_pip(bel.crd(), bel.wire("CI"), obel.wire("CO"));
    }
    if bel.slot == bels::LC3 {
        vrf.claim_pip(bel.crd(), bel.wire_far("CO"), bel.wire("CO"));
    }
}

fn verify_iob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    let kind = if bel.naming.pins.contains_key("CLKIN") {
        pins.push(("CLKIN", SitePinDir::Out));
        let (col, row) = if bel.row == endev.edev.chip.row_s() {
            (endev.edev.chip.col_w(), endev.edev.chip.row_s())
        } else if bel.row == endev.edev.chip.row_n() {
            (endev.edev.chip.col_e(), endev.edev.chip.row_n())
        } else if bel.col == endev.edev.chip.col_w() {
            (endev.edev.chip.col_w(), endev.edev.chip.row_n())
        } else if bel.col == endev.edev.chip.col_e() {
            (endev.edev.chip.col_e(), endev.edev.chip.row_s())
        } else {
            unreachable!()
        };
        let obel = vrf.get_bel(bel.cell.with_cr(col, row).bel(bels::CLKIOB));
        vrf.verify_net(&[bel.fwire("CLKIN"), obel.fwire("OUT")]);
        "CLKIOB"
    } else {
        "IOB"
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
}

fn verify_top_cout(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_delta(bel, 0, -1, bels::LC3).unwrap();
    vrf.verify_net(&[bel.fwire("OUT"), obel.fwire_far("CO")]);
    // artifact of unbuffered pip representation — disregard
    vrf.claim_pip(bel.crd(), "WIRE_COUT_TOP", "WIRE_M14_TOP");
}

fn verify_bot_cin(vrf: &mut Verifier, bel: &BelContext<'_>) {
    // artifact of unbuffered pip representation — disregard
    vrf.claim_pip(bel.crd(), "WIRE_M14_BOT", "WIRE_COUT_BOT");
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let slot_name = endev.edev.egrid.db.bel_slots.key(bel.slot);
    match bel.slot {
        _ if slot_name.starts_with("LC") => verify_lc(vrf, bel),
        _ if slot_name.starts_with("IO") => verify_iob(endev, vrf, bel),
        _ if slot_name.starts_with("TBUF") => vrf.verify_bel(bel, "TBUF", &[], &[]),
        bels::BUFG => vrf.verify_bel(bel, "CLK", &[], &[]),
        bels::CLKIOB => (),
        bels::BUFR => vrf.claim_pip(bel.crd(), bel.wire("OUT"), bel.wire("IN")),
        bels::COUT => verify_top_cout(vrf, bel),
        bels::CIN => verify_bot_cin(vrf, bel),
        bels::RDBK
        | bels::STARTUP
        | bels::BSCAN
        | bels::OSC
        | bels::BYPOSC
        | bels::BSUPD
        | bels::VCC_GND => vrf.verify_bel(bel, slot_name, &[], &[]),
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
