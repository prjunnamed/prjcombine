use crate::verify::{BelContext, SitePinDir, Verifier};
use prjcombine_xilinx_geom::virtex4::Grid;

pub fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if matches!(bel.key, "SLICE0" | "SLICE2") {
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
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (dbel, dpin, sbel, spin) in [
        ("SLICE0", "FXINA", "SLICE0", "F5"),
        ("SLICE0", "FXINB", "SLICE2", "F5"),
        ("SLICE1", "FXINA", "SLICE1", "F5"),
        ("SLICE1", "FXINB", "SLICE3", "F5"),
        ("SLICE2", "FXINA", "SLICE0", "FX"),
        ("SLICE2", "FXINB", "SLICE1", "FX"),
        ("SLICE3", "FXINA", "SLICE2", "FX"),
        // SLICE3 FXINB <- top's SLICE2 FX

        // SLICE0 CIN <- bot's SLICE2 COUT
        // SLICE1 CIN <- bot's SLICE3 COUT
        ("SLICE2", "CIN", "SLICE0", "COUT"),
        ("SLICE3", "CIN", "SLICE1", "COUT"),
        ("SLICE0", "SHIFTIN", "SLICE2", "SHIFTOUT"),
        // SLICE2 SHIFTIN disconnected?
        ("SLICE0", "ALTDIG", "SLICE2", "DIG"),
        // SLICE2 ALTDIG disconnected?
        ("SLICE0", "SLICEWE1", "SLICE0", "BYOUT"),
        ("SLICE2", "SLICEWE1", "SLICE0", "BYINVOUT"),
    ] {
        if dbel != bel.key {
            continue;
        }
        let obel = vrf.find_bel_sibling(bel, sbel);
        vrf.claim_pip(bel.crd(), bel.wire(dpin), obel.wire(spin));
        vrf.claim_node(&[bel.fwire(dpin)]);
    }
    if bel.key == "SLICE2" {
        vrf.claim_node(&[bel.fwire("SHIFTIN")]);
        vrf.claim_node(&[bel.fwire("ALTDIG")]);
    }
    if bel.key == "SLICE3" {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, "SLICE2") {
            vrf.claim_node(&[bel.fwire("FXINB"), obel.fwire("FX_S")]);
            vrf.claim_pip(obel.crd(), obel.wire("FX_S"), obel.wire("FX"));
        } else {
            vrf.claim_node(&[bel.fwire("FXINB")]);
        }
    }
    for (dbel, sbel) in [("SLICE0", "SLICE2"), ("SLICE1", "SLICE3")] {
        if bel.key != dbel {
            continue;
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, sbel) {
            vrf.claim_node(&[bel.fwire("CIN"), obel.fwire("COUT_N")]);
            vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
        } else {
            vrf.claim_node(&[bel.fwire("CIN")]);
        }
    }
}

pub fn verify_bel(_grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("SLICE") => verify_slice(vrf, bel),
        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}
