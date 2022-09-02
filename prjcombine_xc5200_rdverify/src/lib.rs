use prjcombine_xc5200::ExpandedDevice;

use prjcombine_rdverify::{BelContext, SitePinDir, Verifier};

pub fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("LC") => {
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
        _ if bel.key.starts_with("IOB") => {
            let mut pins = vec![];
            let kind = if bel.naming.pins.contains_key("CLKIN") {
                pins.push(("CLKIN", SitePinDir::Out));
                let st = if bel.row == edev.grid.row_bio() {
                    (edev.grid.col_lio(), edev.grid.row_bio())
                } else if bel.row == edev.grid.row_tio() {
                    (edev.grid.col_rio(), edev.grid.row_tio())
                } else if bel.col == edev.grid.col_lio() {
                    (edev.grid.col_lio(), edev.grid.row_tio())
                } else if bel.col == edev.grid.col_rio() {
                    (edev.grid.col_rio(), edev.grid.row_bio())
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
        _ if bel.key.starts_with("TBUF") => {
            vrf.verify_bel(bel, "TBUF", &[], &[]);
        }
        "BUFG" => {
            vrf.verify_bel(bel, "CLK", &[], &[]);
        }
        "CLKIOB" => (),
        "BUFR" => {
            vrf.claim_pip(bel.crd(), bel.wire("OUT"), bel.wire("IN"));
        }
        "TOP_COUT" => {
            let obel = vrf.find_bel_delta(bel, 0, -1, "LC3").unwrap();
            vrf.verify_node(&[bel.fwire("OUT"), obel.fwire_far("CO")]);
            // artifact of unbuffered pip representation — disregard
            vrf.claim_pip(bel.crd(), "WIRE_COUT_TOP", "WIRE_M14_TOP");
        }
        "BOT_CIN" => {
            // artifact of unbuffered pip representation — disregard
            vrf.claim_pip(bel.crd(), "WIRE_M14_BOT", "WIRE_COUT_BOT");
        }
        "RDBK" | "STARTUP" | "BSCAN" | "OSC" | "BYPOSC" | "BSUPD" | "VCC_GND" => {
            vrf.verify_bel(bel, bel.key, &[], &[]);
        }
        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}
