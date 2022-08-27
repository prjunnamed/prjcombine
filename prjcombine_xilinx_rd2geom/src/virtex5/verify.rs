use crate::verify::{BelContext, SitePinDir, Verifier};
use prjcombine_xilinx_geom::virtex5::Grid;

pub fn verify_bel(_grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("SLICE") => {
            let kind = if bel.bel.pins.contains_key("WE") {
                "SLICEM"
            } else {
                "SLICEL"
            };
            vrf.verify_bel(
                bel,
                kind,
                &[("CIN", SitePinDir::In), ("COUT", SitePinDir::Out)],
                &[],
            );
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.key) {
                vrf.claim_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
                vrf.claim_pip(obel.crd(), obel.wire_far("COUT"), obel.wire("COUT"));
            } else {
                vrf.claim_node(&[bel.fwire("CIN")]);
            }
            vrf.claim_node(&[bel.fwire("COUT")]);
        }
        _ => {
            println!("MEOW {} {:?}", bel.key, bel.name);
        }
    }
}
