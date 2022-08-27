use crate::verify::{BelContext, SitePinDir, Verifier};
use prjcombine_entity::EntityVec;
use prjcombine_xilinx_geom::versal::Grid;
use prjcombine_xilinx_geom::SlrId;

pub fn verify_bel(_grids: &EntityVec<SlrId, Grid>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("SLICE") => {
            let kind = if bel.bel.pins.contains_key("WE") {
                "SLICEM"
            } else {
                "SLICEL"
            };
            let mut pins = vec![
                ("CIN", SitePinDir::In),
                ("COUT", SitePinDir::Out),
                ("LAG_E1", SitePinDir::In),
                ("LAG_E2", SitePinDir::In),
                ("LAG_W1", SitePinDir::In),
                ("LAG_W2", SitePinDir::In),
                ("LAG_S", SitePinDir::In),
                ("LAG_N", SitePinDir::In),
            ];
            if kind == "SLICEM" {
                pins.extend([("SRL_IN_B", SitePinDir::In), ("SRL_OUT_B", SitePinDir::Out)]);
            }
            vrf.verify_bel(bel, kind, &pins, &[]);
            for (pin, dir) in pins {
                vrf.claim_node(&[bel.fwire(pin)]);
                if dir == SitePinDir::In {
                    vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
                } else {
                    vrf.claim_pip(bel.crd(), bel.wire_far(pin), bel.wire(pin));
                }
            }
            vrf.claim_node(&[bel.fwire_far("COUT")]);
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.key) {
                vrf.verify_node(&[bel.fwire_far("CIN"), obel.fwire_far("COUT")]);
            } else {
                vrf.claim_node(&[bel.fwire_far("CIN")]);
            }
            if kind == "SLICEM" {
                vrf.claim_node(&[bel.fwire_far("SRL_OUT_B")]);
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.key) {
                    vrf.verify_node(&[bel.fwire_far("SRL_IN_B"), obel.fwire_far("SRL_OUT_B")]);
                } else {
                    vrf.claim_node(&[bel.fwire_far("SRL_IN_B")]);
                }
            }
            // XXX LAG_*
        }
        _ => {
            println!("MEOW {} {:?}", bel.key, bel.name);
        }
    }
}
