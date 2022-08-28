use crate::verify::{BelContext, SitePinDir, Verifier};
use prjcombine_xilinx_geom::spartan6::Grid;

pub fn verify_bel(_grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        "SLICE0" => {
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
            if let Some(obel) = vrf.find_bel_walk(bel, 0, -1, "SLICE0") {
                vrf.claim_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
                vrf.claim_pip(obel.crd(), obel.wire_far("COUT"), obel.wire("COUT"));
            } else {
                vrf.claim_node(&[bel.fwire("CIN")]);
            }
            vrf.claim_node(&[bel.fwire("COUT")]);
        }
        "SLICE1" => {
            vrf.verify_bel(bel, "SLICEX", &[], &[]);
        }
        "BRAM_F" => vrf.verify_bel(bel, "RAMB16BWER", &[], &[]),
        _ if bel.key.starts_with("BRAM_H") => vrf.verify_bel(bel, "RAMB8BWER", &[], &[]),
        "DSP" => {
            let carry: Vec<_> = (0..18)
                .map(|x| (format!("BCOUT{x}"), format!("BCIN{x}")))
                .chain((0..48).map(|x| (format!("PCOUT{x}"), format!("PCIN{x}"))))
                .chain([("CARRYOUT".to_string(), "CARRYIN".to_string())].into_iter())
                .collect();
            let mut pins = vec![];
            for (o, i) in &carry {
                pins.push((&**o, SitePinDir::Out));
                pins.push((&**i, SitePinDir::In));
            }
            vrf.verify_bel(bel, "DSP48A1", &pins, &[]);
            for (o, i) in &carry {
                vrf.claim_node(&[bel.fwire(o)]);
                vrf.claim_node(&[bel.fwire(i)]);
            }
            if let Some(obel) = vrf.find_bel_walk(bel, 0, -4, "DSP") {
                for (o, i) in &carry {
                    vrf.verify_node(&[bel.fwire(i), obel.fwire_far(o)]);
                    vrf.claim_pip(obel.crd(), obel.wire_far(o), obel.wire(o));
                }
            }
        }
        "PCIE" => vrf.verify_bel(bel, "PCIE_A1", &[], &[]),
        _ => {
            println!("MEOW {} {:?}", bel.key, bel.name);
        }
    }
}
