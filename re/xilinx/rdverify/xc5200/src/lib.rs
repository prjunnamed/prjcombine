use prjcombine_re_xilinx_naming_xc2000::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};

fn verify_lc(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = match bel.key {
        "LC0" | "LC2" => "LC5A",
        "LC1" | "LC3" => "LC5B",
        _ => unreachable!(),
    };
    let mut pins = vec![("CI", SitePinDir::In), ("CO", SitePinDir::Out)];
    if kind == "LC5A" {
        pins.push(("F5I", SitePinDir::In));
        let okey = match bel.key {
            "LC0" => "LC1",
            "LC2" => "LC3",
            _ => unreachable!(),
        };
        vrf.claim_node(&[bel.fwire("F5I")]);
        let obel = vrf.find_bel_sibling(bel, okey);
        vrf.claim_pip(bel.crd(), bel.wire("F5I"), obel.wire("X"));
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
    vrf.claim_node(&[bel.fwire("CI")]);
    vrf.claim_node(&[bel.fwire("CO")]);
    if bel.key == "LC0" {
        vrf.claim_pip(bel.crd(), bel.wire("CI"), bel.wire_far("CI"));
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, "LC3") {
            vrf.claim_node(&[bel.fwire_far("CI"), obel.fwire_far("CO")]);
        } else {
            let obel = vrf.find_bel_delta(bel, 0, -1, "BOT_CIN").unwrap();
            vrf.verify_node(&[bel.fwire_far("CI"), obel.fwire("IN")]);
        }
    } else {
        let okey = match bel.key {
            "LC1" => "LC0",
            "LC2" => "LC1",
            "LC3" => "LC2",
            _ => unreachable!(),
        };
        let obel = vrf.find_bel_sibling(bel, okey);
        vrf.claim_pip(bel.crd(), bel.wire("CI"), obel.wire("CO"));
    }
    if bel.key == "LC3" {
        vrf.claim_pip(bel.crd(), bel.wire_far("CO"), bel.wire("CO"));
    }
}

fn verify_iob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    let kind = if bel.naming.pins.contains_key("CLKIN") {
        pins.push(("CLKIN", SitePinDir::Out));
        let st = if bel.row == endev.edev.grid.row_bio() {
            (endev.edev.grid.col_lio(), endev.edev.grid.row_bio())
        } else if bel.row == endev.edev.grid.row_tio() {
            (endev.edev.grid.col_rio(), endev.edev.grid.row_tio())
        } else if bel.col == endev.edev.grid.col_lio() {
            (endev.edev.grid.col_lio(), endev.edev.grid.row_tio())
        } else if bel.col == endev.edev.grid.col_rio() {
            (endev.edev.grid.col_rio(), endev.edev.grid.row_bio())
        } else {
            unreachable!()
        };
        let obel = vrf.find_bel(bel.die, st, "CLKIOB").unwrap();
        vrf.verify_node(&[bel.fwire("CLKIN"), obel.fwire("OUT")]);
        "CLKIOB"
    } else {
        "IOB"
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
}

fn verify_top_cout(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_delta(bel, 0, -1, "LC3").unwrap();
    vrf.verify_node(&[bel.fwire("OUT"), obel.fwire_far("CO")]);
    // artifact of unbuffered pip representation — disregard
    vrf.claim_pip(bel.crd(), "WIRE_COUT_TOP", "WIRE_M14_TOP");
}

fn verify_bot_cin(vrf: &mut Verifier, bel: &BelContext<'_>) {
    // artifact of unbuffered pip representation — disregard
    vrf.claim_pip(bel.crd(), "WIRE_M14_BOT", "WIRE_COUT_BOT");
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("LC") => verify_lc(vrf, bel),
        _ if bel.key.starts_with("IOB") => verify_iob(endev, vrf, bel),
        _ if bel.key.starts_with("TBUF") => vrf.verify_bel(bel, "TBUF", &[], &[]),
        "BUFG" => vrf.verify_bel(bel, "CLK", &[], &[]),
        "CLKIOB" => (),
        "BUFR" => vrf.claim_pip(bel.crd(), bel.wire("OUT"), bel.wire("IN")),
        "TOP_COUT" => verify_top_cout(vrf, bel),
        "BOT_CIN" => verify_bot_cin(vrf, bel),
        "RDBK" | "STARTUP" | "BSCAN" | "OSC" | "BYPOSC" | "BSUPD" | "VCC_GND" => {
            vrf.verify_bel(bel, bel.key, &[], &[])
        }
        _ => println!("MEOW {} {:?}", bel.key, bel.name),
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
