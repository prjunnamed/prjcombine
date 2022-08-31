use prjcombine_entity::EntityId;
use prjcombine_entity::EntityVec;
use prjcombine_int::grid::DieId;
use prjcombine_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_ultrascale::Grid;

fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
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

fn verify_dsp(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    pairs.push(("MULTSIGNIN".to_string(), "MULTSIGNOUT".to_string()));
    pairs.push(("CARRYCASCIN".to_string(), "CARRYCASCOUT".to_string()));
    for i in 0..30 {
        pairs.push((format!("ACIN_B{i}"), format!("ACOUT_B{i}")));
    }
    for i in 0..18 {
        pairs.push((format!("BCIN_B{i}"), format!("BCOUT_B{i}")));
    }
    for i in 0..48 {
        pairs.push((format!("PCIN{i}"), format!("PCOUT{i}")));
    }
    let mut pins = vec![];
    for (ipin, opin) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        if bel.key == "DSP0" {
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "DSP1") {
                vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            }
        } else {
            let obel = vrf.find_bel_sibling(bel, "DSP0");
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_bel(bel, "DSP48E2", &pins, &[]);
}

fn verify_bram_f(vrf: &mut Verifier, bel: &BelContext<'_>) {
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum Mode {
        Up,
        DownHalfReg,
        UpBuf,
        DownBuf,
    }
    let mut pairs = vec![
        (
            "ENABLE_BIST".to_string(),
            "START_RSR_NEXT".to_string(),
            Mode::DownHalfReg,
        ),
        (
            "CASINSBITERR".to_string(),
            "CASOUTSBITERR".to_string(),
            Mode::Up,
        ),
        (
            "CASINDBITERR".to_string(),
            "CASOUTDBITERR".to_string(),
            Mode::Up,
        ),
        (
            "CASPRVEMPTY".to_string(),
            "CASNXTEMPTY".to_string(),
            Mode::UpBuf,
        ),
        (
            "CASNXTRDEN".to_string(),
            "CASPRVRDEN".to_string(),
            Mode::DownBuf,
        ),
    ];
    for ab in ['A', 'B'] {
        for ul in ['U', 'L'] {
            for i in 0..16 {
                pairs.push((
                    format!("CASDI{ab}{ul}{i}"),
                    format!("CASDO{ab}{ul}{i}"),
                    Mode::UpBuf,
                ));
            }
            for i in 0..2 {
                pairs.push((
                    format!("CASDIP{ab}{ul}{i}"),
                    format!("CASDOP{ab}{ul}{i}"),
                    Mode::UpBuf,
                ));
            }
        }
    }
    let mut pins = vec![("CASMBIST12OUT", SitePinDir::Out)];
    vrf.claim_node(&[bel.fwire("CASMBIST12OUT")]);
    for (ipin, opin, mode) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
        match mode {
            Mode::UpBuf => {
                vrf.claim_node(&[bel.fwire_far(opin)]);
                vrf.claim_pip(bel.crd(), bel.wire_far(opin), bel.wire(opin));
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire_far(opin)]);
                } else {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::Up => {
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
                } else {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::DownBuf => {
                vrf.claim_node(&[bel.fwire_far(opin)]);
                vrf.claim_pip(bel.crd(), bel.wire_far(opin), bel.wire(opin));
                if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire_far(opin)]);
                } else {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::DownHalfReg => {
                if bel.row.to_idx() % 30 != 25 {
                    if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, "BRAM_F") {
                        vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
                    } else {
                        vrf.claim_node(&[bel.fwire_far(ipin)]);
                    }
                } else {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
        }
    }
    vrf.verify_bel(bel, "RAMBFIFO36", &pins, &[]);
}

fn verify_bram_h(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let (kind, ul) = match bel.key {
        "BRAM_H0" => ("RAMBFIFO18", 'L'),
        "BRAM_H1" => ("RAMB181", 'U'),
        _ => unreachable!(),
    };
    let mut pins = vec![];
    if ul == 'L' {
        pins.extend([
            ("CASPRVEMPTY".to_string(), SitePinDir::In),
            ("CASNXTEMPTY".to_string(), SitePinDir::Out),
            ("CASPRVRDEN".to_string(), SitePinDir::Out),
            ("CASNXTRDEN".to_string(), SitePinDir::In),
        ]);
    }
    for ab in ['A', 'B'] {
        for i in 0..16 {
            pins.push((format!("CASDI{ab}{ul}{i}"), SitePinDir::In));
            pins.push((format!("CASDO{ab}{ul}{i}"), SitePinDir::Out));
        }
        for i in 0..2 {
            pins.push((format!("CASDIP{ab}{ul}{i}"), SitePinDir::In));
            pins.push((format!("CASDOP{ab}{ul}{i}"), SitePinDir::Out));
        }
    }
    let pin_refs: Vec<_> = pins.iter().map(|&(ref pin, dir)| (&pin[..], dir)).collect();
    vrf.verify_bel(bel, kind, &pin_refs, &[]);
    let obel = vrf.find_bel_sibling(bel, "BRAM_F");
    for (pin, dir) in pin_refs {
        vrf.claim_node(&[bel.fwire(pin)]);
        match dir {
            SitePinDir::In => vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire_far(pin)),
            SitePinDir::Out => vrf.claim_pip(bel.crd(), obel.wire_far(pin), bel.wire(pin)),
            _ => unreachable!(),
        }
    }
}

fn verify_uram(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    for ab in ['A', 'B'] {
        for i in 0..23 {
            pairs.push((
                format!("CAS_IN_ADDR_{ab}{i}"),
                format!("CAS_OUT_ADDR_{ab}{i}"),
            ));
        }
        for i in 0..9 {
            pairs.push((
                format!("CAS_IN_BWE_{ab}{i}"),
                format!("CAS_OUT_BWE_{ab}{i}"),
            ));
        }
        for i in 0..72 {
            pairs.push((
                format!("CAS_IN_DIN_{ab}{i}"),
                format!("CAS_OUT_DIN_{ab}{i}"),
            ));
            pairs.push((
                format!("CAS_IN_DOUT_{ab}{i}"),
                format!("CAS_OUT_DOUT_{ab}{i}"),
            ));
        }
        for pin in ["EN", "RDACCESS", "RDB_WR", "DBITERR", "SBITERR"] {
            pairs.push((format!("CAS_IN_{pin}_{ab}"), format!("CAS_OUT_{pin}_{ab}")));
        }
    }
    let mut pins = vec![];
    for (ipin, opin) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        if bel.key == "URAM0" {
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -15, "URAM3") {
                vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            }
        } else {
            let okey = match bel.key {
                "URAM1" => "URAM0",
                "URAM2" => "URAM1",
                "URAM3" => "URAM2",
                _ => unreachable!(),
            };
            let obel = vrf.find_bel_sibling(bel, okey);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_bel(bel, "URAM288", &pins, &[]);
}

pub fn verify_bel(_grids: &EntityVec<DieId, Grid>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        "SLICE_L" | "SLICE_R" => verify_slice(vrf, bel),
        "DSP0" | "DSP1" => verify_dsp(vrf, bel),
        "BRAM_F" => verify_bram_f(vrf, bel),
        "BRAM_H0" | "BRAM_H1" => verify_bram_h(vrf, bel),
        _ if bel.key.starts_with("HARD_SYNC") => vrf.verify_bel(bel, "HARD_SYNC", &[], &[]),
        _ if bel.key.starts_with("URAM") => verify_uram(vrf, bel),
        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}
