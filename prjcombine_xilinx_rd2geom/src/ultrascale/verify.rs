use crate::verify::{BelContext, SitePinDir, Verifier};
use prjcombine_entity::EntityVec;
use prjcombine_xilinx_geom::ultrascale::Grid;
use prjcombine_xilinx_geom::SlrId;

pub fn verify_bel(_grids: &EntityVec<SlrId, Grid>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("SLICE") => {
            let kind = if bel.node_kind == "CLEM" {
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
            vrf.claim_pip(bel.crd(), bel.wire("CIN"), bel.wire_far("CIN"));
            vrf.claim_node(&[bel.fwire("CIN")]);
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.key) {
                vrf.verify_node(&[bel.fwire_far("CIN"), obel.fwire("COUT")]);
            }
            vrf.claim_node(&[bel.fwire("COUT")]);
        }
        _ => {
            println!("MEOW {} {:?}", bel.key, bel.name);
        }
    }
}
