use prjcombine_entity::EntityId;
use prjcombine_int::grid::RowId;
use prjcombine_rawdump::Part;
use prjcombine_rdverify::{verify, BelContext, SitePinDir, Verifier};
use prjcombine_series7::ExpandedDevice;
use std::collections::HashMap;

fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
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

fn verify_dsp(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    pairs.push(("MULTSIGNIN".to_string(), "MULTSIGNOUT".to_string()));
    pairs.push(("CARRYCASCIN".to_string(), "CARRYCASCOUT".to_string()));
    for i in 0..30 {
        pairs.push((format!("ACIN{i}"), format!("ACOUT{i}")));
    }
    for i in 0..18 {
        pairs.push((format!("BCIN{i}"), format!("BCOUT{i}")));
    }
    for i in 0..48 {
        pairs.push((format!("PCIN{i}"), format!("PCOUT{i}")));
    }
    let mut pins = vec![];
    for (ipin, opin) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        if bel.key == "DSP0" {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "DSP1") {
                vrf.claim_node(&[bel.fwire(ipin), obel.fwire_far(opin)]);
                vrf.claim_pip(obel.crd(), obel.wire_far(opin), obel.wire(opin));
            } else {
                vrf.claim_node(&[bel.fwire(ipin)]);
            }
        } else {
            vrf.claim_node(&[bel.fwire(ipin)]);
            let obel = vrf.find_bel_sibling(bel, "DSP0");
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_bel(bel, "DSP48E1", &pins, &[]);
    let obel = vrf.find_bel_sibling(bel, "TIEOFF");
    for pin in [
        "ALUMODE2",
        "ALUMODE3",
        "CARRYINSEL2",
        "CEAD",
        "CEALUMODE",
        "CED",
        "CEINMODE",
        "INMODE0",
        "INMODE1",
        "INMODE2",
        "INMODE3",
        "INMODE4",
        "OPMODE6",
        "RSTD",
        "D0",
        "D1",
        "D2",
        "D3",
        "D4",
        "D5",
        "D6",
        "D7",
        "D8",
        "D9",
        "D10",
        "D11",
        "D12",
        "D13",
        "D14",
        "D15",
        "D16",
        "D17",
        "D18",
        "D19",
        "D20",
        "D21",
        "D22",
        "D23",
        "D24",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("HARD0"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("HARD1"));
    }
}

fn verify_tieoff(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "TIEOFF",
        &[("HARD0", SitePinDir::Out), ("HARD1", SitePinDir::Out)],
        &[],
    );
    for pin in ["HARD0", "HARD1"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_bram_f(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut addrpins = vec![];
    for ab in ["ARD", "BWR"] {
        for ul in ['U', 'L'] {
            for i in 0..15 {
                addrpins.push(format!("ADDR{ab}ADDR{ul}{i}"));
            }
        }
    }
    let mut pins = vec![
        ("CASCADEINA", SitePinDir::In),
        ("CASCADEINB", SitePinDir::In),
        ("CASCADEOUTA", SitePinDir::Out),
        ("CASCADEOUTB", SitePinDir::Out),
        ("ADDRARDADDRL15", SitePinDir::In),
        ("ADDRBWRADDRL15", SitePinDir::In),
    ];
    for apin in &addrpins {
        pins.push((apin, SitePinDir::In));
    }
    vrf.verify_bel(bel, "RAMBFIFO36E1", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bel.key) {
        for (ipin, opin) in [("CASCADEINA", "CASCADEOUTA"), ("CASCADEINB", "CASCADEOUTB")] {
            vrf.verify_node(&[bel.fwire(ipin), obel.fwire_far(opin)]);
            vrf.claim_pip(obel.crd(), obel.wire_far(opin), obel.wire(opin));
        }
    }
    let obel = vrf.find_bel_sibling(bel, "BRAM_ADDR");
    for apin in &addrpins {
        vrf.claim_pip(bel.crd(), bel.wire(apin), obel.wire(apin));
    }
    for (pin, ipin) in [
        ("ADDRARDADDRL15", "IMUX_ADDRARDADDRL15"),
        ("ADDRBWRADDRL15", "IMUX_ADDRBWRADDRL15"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(ipin));
    }
}

fn verify_bram_h(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut addrpins = vec![];
    for ab in ["ARD", "BWR"] {
        for i in 0..14 {
            addrpins.push(format!("ADDR{ab}ADDR{i}"));
        }
    }
    for ab in ['A', 'B'] {
        for i in 0..2 {
            addrpins.push(format!("ADDR{ab}TIEHIGH{i}"));
        }
    }
    let mut dummy_pins = vec![];
    let kind;
    let ul;
    if bel.key == "BRAM_H1" {
        kind = "RAMB18E1";
        ul = 'U';
        dummy_pins.extend([
            "FULL".to_string(),
            "EMPTY".to_string(),
            "ALMOSTFULL".to_string(),
            "ALMOSTEMPTY".to_string(),
            "WRERR".to_string(),
            "RDERR".to_string(),
        ]);
        for i in 0..12 {
            dummy_pins.push(format!("RDCOUNT{i}"));
            dummy_pins.push(format!("WRCOUNT{i}"));
        }
    } else {
        ul = 'L';
        kind = "FIFO18E1";
    }
    let mut pin_refs: Vec<_> = dummy_pins
        .iter()
        .map(|x| (&x[..], SitePinDir::Out))
        .collect();
    for apin in &addrpins {
        pin_refs.push((apin, SitePinDir::In));
    }
    vrf.verify_bel(bel, kind, &pin_refs, &[]);
    for (pin, _) in pin_refs {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, "BRAM_ADDR");
    for ab in ["ARD", "BWR"] {
        for i in 0..14 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("ADDR{ab}ADDR{i}")),
                obel.wire(&format!("ADDR{ab}ADDR{ul}{ii}", ii = i + 1)),
            );
        }
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("ADDRATIEHIGH0"),
        obel.wire(&format!("ADDRARDADDR{ul}0")),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("ADDRBTIEHIGH0"),
        obel.wire(&format!("ADDRBWRADDR{ul}0")),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("ADDRATIEHIGH1"),
        obel.wire("IMUX_ADDRARDADDRL15"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("ADDRBTIEHIGH1"),
        obel.wire("IMUX_ADDRBWRADDRL15"),
    );
}

fn verify_bram_addr(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut imux_addr = HashMap::new();
    let obel_t = vrf.find_bel_delta(bel, 0, 5, bel.key);
    let obel_b = vrf.find_bel_delta(bel, 0, -5, bel.key);
    for ab in ["ARD", "BWR"] {
        for ul in ['U', 'L'] {
            for i in 0..15 {
                let apin = format!("ADDR{ab}ADDR{ul}{i}");
                let ipin = format!("IMUX_ADDR{ab}ADDR{ul}{i}");
                let upin = format!("UTURN_ADDR{ab}ADDR{ul}{i}");
                let cibpin = format!("CASCINBOT_ADDR{ab}ADDRU{i}");
                let citpin = format!("CASCINTOP_ADDR{ab}ADDRU{i}");
                vrf.claim_node(&[bel.fwire(&apin)]);
                vrf.claim_pip(bel.crd(), bel.wire(&apin), bel.wire(&ipin));
                vrf.claim_pip(bel.crd(), bel.wire(&apin), bel.wire(&cibpin));
                vrf.claim_pip(bel.crd(), bel.wire(&apin), bel.wire(&citpin));
                vrf.claim_node(&[bel.fwire(&upin)]);
                vrf.claim_pip(bel.crd(), bel.wire(&upin), bel.wire(&apin));
                if ul == 'U' {
                    let copin = format!("CASCOUT_ADDR{ab}ADDRU{i}");
                    vrf.claim_node(&[bel.fwire(&copin)]);
                    vrf.claim_pip(bel.crd(), bel.wire(&copin), bel.wire(&apin));
                    if let Some(ref obel) = obel_b {
                        vrf.verify_node(&[bel.fwire(&cibpin), obel.fwire(&copin)]);
                    } else {
                        vrf.claim_node(&[bel.fwire(&cibpin)]);
                    }
                    if let Some(ref obel) = obel_t {
                        vrf.verify_node(&[bel.fwire(&citpin), obel.fwire(&copin)]);
                    } else {
                        vrf.claim_node(&[bel.fwire(&citpin)]);
                    }
                }
                let iwire = *bel.bel.pins[&ipin].wires.iter().next().unwrap();
                imux_addr.insert(iwire, upin);
            }
        }
        let ipin = format!("IMUX_ADDR{ab}ADDRL15");
        let upin = format!("UTURN_ADDR{ab}ADDRL15");
        vrf.claim_node(&[bel.fwire(&upin)]);
        vrf.claim_pip(bel.crd(), bel.wire(&upin), bel.wire(&ipin));
        let iwire = *bel.bel.pins[&ipin].wires.iter().next().unwrap();
        imux_addr.insert(iwire, upin);
    }
    for i in 0..5 {
        for j in 0..48 {
            let ipin = format!("IMUX_{i}_{j}");
            let upin = format!("IMUX_UTURN_{i}_{j}");
            let iwire = *bel.bel.pins[&ipin].wires.iter().next().unwrap();
            if let Some(aupin) = imux_addr.get(&iwire) {
                vrf.claim_pip(bel.crd(), bel.wire(&upin), bel.wire(aupin));
            } else {
                vrf.claim_pip(bel.crd(), bel.wire(&upin), bel.wire(&ipin));
            }
        }
    }
}

fn verify_int_gclk(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let (hclk, rng) = match bel.key {
        "INT_GCLK_L" => ("HCLK_L", 6..12),
        "INT_GCLK_R" => ("HCLK_R", 0..6),
        _ => unreachable!(),
    };
    let srow = edev.grids[bel.die].row_hclk(bel.row);
    let ud = if bel.row.to_idx() % 50 < 25 { 'D' } else { 'U' };
    let obel = vrf.find_bel(bel.die, (bel.col, srow), hclk).unwrap();
    for i in rng {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_O_L")),
            bel.wire(&format!("GCLK{i}_I")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_O_R")),
            bel.wire(&format!("GCLK{i}_I")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("GCLK{i}_I")),
            obel.fwire(&format!("GCLK{i}_O_{ud}")),
        ]);
    }
}

fn verify_hclk_l(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let has_d = vrf.find_bel_delta(bel, 0, -1, "INT_GCLK_L").is_some();
    let has_u = vrf.find_bel_delta(bel, 0, 0, "INT_GCLK_L").is_some();
    for i in 6..12 {
        for ud in ['D', 'U'] {
            if ud == 'D' && !has_d {
                continue;
            }
            if ud == 'U' && !has_u {
                continue;
            }
            vrf.claim_node(&[bel.fwire(&format!("GCLK{i}_O_{ud}"))]);
            for j in 0..8 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("GCLK{i}_O_{ud}")),
                    bel.wire(&format!("HCLK{j}_I")),
                );
            }
            for j in 8..12 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("GCLK{i}_O_{ud}")),
                    bel.wire(&format!("HCLK{j}")),
                );
            }
            for j in 0..4 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("GCLK{i}_O_{ud}")),
                    bel.wire(&format!("RCLK{j}")),
                );
            }
        }
    }
    let obel = vrf.find_bel_sibling(bel, "HCLK_R");
    let grid = edev.grids[bel.die];
    let obel_hrow = vrf
        .find_bel(bel.die, (grid.col_clk, bel.row), "CLK_HROW")
        .unwrap();
    let has_rclk = grid.cols_io[if bel.col <= grid.col_clk { 0 } else { 1 }]
        .as_ref()
        .filter(|ioc| ioc.regs[bel.row.to_idx() / 50].is_some())
        .is_some();
    let lr = if bel.col <= grid.col_clk { 'L' } else { 'R' };
    for i in 8..12 {
        vrf.claim_node(&[
            bel.fwire(&format!("HCLK{i}_O")),
            obel.fwire(&format!("HCLK{i}_I")),
        ]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK{i}")),
            obel_hrow.fwire(&format!("HCLK{i}_{lr}")),
        ]);
    }
    for i in 0..4 {
        vrf.claim_node(&[
            bel.fwire(&format!("RCLK{i}_O")),
            obel.fwire(&format!("RCLK{i}_I")),
        ]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK{i}_O")),
            bel.wire(&format!("RCLK{i}")),
        );
        if has_rclk {
            vrf.verify_node(&[
                bel.fwire(&format!("RCLK{i}")),
                obel_hrow.fwire(&format!("RCLK{i}_{lr}")),
            ]);
        } else {
            vrf.claim_dummy_in(bel.fwire(&format!("RCLK{i}")));
        }
    }
}

fn verify_hclk_r(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let has_d = vrf.find_bel_delta(bel, 0, -1, "INT_GCLK_L").is_some();
    let has_u = vrf.find_bel_delta(bel, 0, 0, "INT_GCLK_L").is_some();
    for i in 0..6 {
        for ud in ['D', 'U'] {
            if ud == 'D' && !has_d {
                continue;
            }
            if ud == 'U' && !has_u {
                continue;
            }
            vrf.claim_node(&[bel.fwire(&format!("GCLK{i}_O_{ud}"))]);
            for j in 0..8 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("GCLK{i}_O_{ud}")),
                    bel.wire(&format!("HCLK{j}")),
                );
            }
            for j in 8..12 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("GCLK{i}_O_{ud}")),
                    bel.wire(&format!("HCLK{j}_I")),
                );
            }
            for j in 0..4 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("GCLK{i}_O_{ud}")),
                    bel.wire(&format!("RCLK{j}_I")),
                );
            }
        }
    }
    let obel = vrf.find_bel_sibling(bel, "HCLK_L");
    let obel_hrow = vrf
        .find_bel(bel.die, (edev.grids[bel.die].col_clk, bel.row), "CLK_HROW")
        .unwrap();
    let lr = if bel.col <= edev.grids[bel.die].col_clk {
        'L'
    } else {
        'R'
    };
    for i in 0..8 {
        vrf.claim_node(&[
            bel.fwire(&format!("HCLK{i}_O")),
            obel.fwire(&format!("HCLK{i}_I")),
        ]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK{i}")),
            obel_hrow.fwire(&format!("HCLK{i}_{lr}")),
        ]);
    }
}

fn verify_gclk_test_buf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.name.unwrap().starts_with("BUFG") {
        "BUFG_LB"
    } else {
        "GCLK_TEST_BUF"
    };
    vrf.verify_bel(
        bel,
        kind,
        &[("CLKIN", SitePinDir::In), ("CLKOUT", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("CLKIN")]);
    vrf.claim_node(&[bel.fwire("CLKOUT")]);
}

fn verify_bufhce(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFHCE",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
}

fn verify_clk_rebuf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_walk(bel, 0, -1, "CLK_REBUF").or_else(|| {
        if bel.die.to_idx() != 0 {
            let odie = bel.die - 1;
            let srow = vrf.grid.die(odie).rows().next_back().unwrap() - 11;
            vrf.find_bel(odie, (bel.col, srow), "CLK_REBUF")
        } else {
            None
        }
    });
    for i in 0..32 {
        let pin_d = format!("GCLK{i}_D");
        let pin_u = format!("GCLK{i}_U");
        vrf.claim_node(&[bel.fwire(&pin_u)]);
        vrf.claim_pip(bel.crd(), bel.wire(&pin_d), bel.wire(&pin_u));
        vrf.claim_pip(bel.crd(), bel.wire(&pin_u), bel.wire(&pin_d));
        let obel_buf_d =
            vrf.find_bel_sibling(bel, &format!("GCLK_TEST_BUF.REBUF_D{ii}", ii = i / 2));
        let obel_buf_u =
            vrf.find_bel_sibling(bel, &format!("GCLK_TEST_BUF.REBUF_U{ii}", ii = i / 2));
        if i % 2 == 0 {
            vrf.claim_pip(bel.crd(), obel_buf_d.wire("CLKIN"), bel.wire(&pin_d));
            vrf.claim_pip(bel.crd(), bel.wire(&pin_u), obel_buf_u.wire("CLKOUT"));
        } else {
            vrf.claim_pip(bel.crd(), bel.wire(&pin_d), obel_buf_d.wire("CLKOUT"));
            vrf.claim_pip(bel.crd(), obel_buf_u.wire("CLKIN"), bel.wire(&pin_u));
        }
        if let Some(ref obel) = obel {
            vrf.verify_node(&[bel.fwire(&pin_d), obel.fwire(&pin_u)]);
        } else {
            vrf.claim_node(&[bel.fwire(&pin_d)]);
        }
    }
}

fn verify_clk_hrow(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let obel_casc = vrf.find_bel_delta(
        bel,
        0,
        if bel.row.to_idx() / 50 < grid.reg_clk {
            -50
        } else {
            50
        },
        "CLK_HROW",
    );
    let obel_buf = vrf.find_bel_walk(bel, 0, -1, "CLK_REBUF").unwrap();
    for i in 0..32 {
        vrf.verify_node(&[
            bel.fwire(&format!("GCLK{i}")),
            obel_buf.fwire(&format!("GCLK{i}_U")),
        ]);
        vrf.claim_node(&[bel.fwire(&format!("GCLK{i}_TEST_IN"))]);
        vrf.claim_node(&[bel.fwire(&format!("GCLK{i}_TEST_OUT"))]);
        vrf.claim_node(&[bel.fwire(&format!("GCLK_TEST{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_TEST_IN")),
            bel.wire(&format!("GCLK{i}")),
        );
        let obel = vrf.find_bel_sibling(bel, &format!("GCLK_TEST_BUF.HROW_GCLK{i}"));
        vrf.claim_pip(
            bel.crd(),
            obel.wire("CLKIN"),
            bel.wire(&format!("GCLK{i}_TEST_IN")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_TEST_OUT")),
            obel.wire("CLKOUT"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK_TEST{i}")),
            bel.wire(&format!("GCLK{i}_TEST_OUT")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK_TEST{ii}", ii = i ^ 1)),
            bel.wire(&format!("GCLK{i}_TEST_OUT")),
        );

        vrf.claim_node(&[bel.fwire(&format!("CASCO{i}"))]);
        if let Some(ref obel_casc) = obel_casc {
            vrf.verify_node(&[
                bel.fwire(&format!("CASCI{i}")),
                obel_casc.fwire(&format!("CASCO{i}")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("CASCI{i}"))]);
        }
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASCO{i}")),
            bel.wire(&format!("CASCI{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASCO{i}")),
            bel.wire(&format!("GCLK_TEST{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASCO{i}")),
            bel.wire("HCLK_TEST_OUT_L"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASCO{i}")),
            bel.wire("HCLK_TEST_OUT_R"),
        );
        for lr in ['L', 'R'] {
            for j in 0..4 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("CASCO{i}")),
                    bel.wire(&format!("RCLK{j}_{lr}")),
                );
            }
            for j in 0..14 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("CASCO{i}")),
                    bel.wire(&format!("IN{j}_{lr}")),
                );
            }
        }
    }

    for lr in ['L', 'R'] {
        let obel = vrf.find_bel_sibling(bel, &format!("GCLK_TEST_BUF.HROW_BUFH_{lr}"));
        vrf.claim_node(&[bel.fwire(&format!("HCLK_TEST_IN_{lr}"))]);
        vrf.claim_pip(
            bel.crd(),
            obel.wire("CLKIN"),
            bel.wire(&format!("HCLK_TEST_IN_{lr}")),
        );
        vrf.claim_node(&[bel.fwire(&format!("HCLK_TEST_OUT_{lr}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK_TEST_OUT_{lr}")),
            obel.wire("CLKOUT"),
        );
        for i in 0..14 {
            vrf.claim_node(&[bel.fwire(&format!("IN{i}_{lr}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HCLK_TEST_IN_{lr}")),
                bel.wire(&format!("IN{i}_{lr}")),
            );
        }
        for i in 0..12 {
            vrf.claim_node(&[bel.fwire(&format!("HCLK{i}_{lr}"))]);
            let obel = vrf.find_bel_sibling(bel, &format!("BUFHCE_{lr}{i}"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HCLK{i}_{lr}")),
                obel.wire("O"),
            );

            if (lr == 'R' && i < 6) || (lr == 'L' && i >= 6) {
                vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire("BUFHCE_CKINT0"));
                vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire("BUFHCE_CKINT1"));
            } else {
                vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire("BUFHCE_CKINT2"));
                vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire("BUFHCE_CKINT3"));
            }
            vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire("HCLK_TEST_OUT_L"));
            vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire("HCLK_TEST_OUT_R"));
            for olr in ['L', 'R'] {
                for j in 0..14 {
                    vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire(&format!("IN{j}_{olr}")));
                }
            }
            for j in 0..32 {
                vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire(&format!("GCLK{j}")));
            }
        }
        let has_rclk = grid.cols_io[if lr == 'L' { 0 } else { 1 }]
            .as_ref()
            .filter(|ioc| ioc.regs[bel.row.to_idx() / 50].is_some())
            .is_some();
        for i in 0..4 {
            if has_rclk {
                vrf.claim_node(&[bel.fwire(&format!("RCLK{i}_{lr}"))]);
            } else {
                vrf.claim_dummy_in(bel.fwire(&format!("RCLK{i}_{lr}")));
            }
        }
    }
}

fn verify_bufgctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFGCTRL",
        &[
            ("I0", SitePinDir::In),
            ("I1", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &["CKINT0", "CKINT1", "FB_TEST0", "FB_TEST1"],
    );
    vrf.claim_node(&[bel.fwire("I0")]);
    vrf.claim_node(&[bel.fwire("I1")]);
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("CASCI0"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("CASCI1"));
    // very likely a case of wrong-direction pip
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("FB_TEST0"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("FB_TEST1"));
    let idx = bel.bid.to_idx();
    for d in [1, 15] {
        let oidx = (idx + d) % 16;
        let obel = vrf.find_bel_sibling(bel, &format!("BUFGCTRL{oidx}"));
        vrf.claim_pip(bel.crd(), bel.wire("I0"), obel.wire("FB"));
        vrf.claim_pip(bel.crd(), bel.wire("I1"), obel.wire("FB"));
    }

    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_node(&[bel.fwire("FB")]);
    vrf.claim_pip(bel.crd(), bel.wire("FB"), bel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("GCLK"), bel.wire("O"));

    let is_b = bel.row.to_idx() % 50 != 0;
    let obel_buf = vrf.find_bel_walk(bel, 0, -1, "CLK_REBUF").unwrap();
    if is_b {
        vrf.verify_node(&[bel.fwire("GCLK"), obel_buf.fwire(&format!("GCLK{idx}_U"))]);
    } else {
        vrf.verify_node(&[
            bel.fwire("GCLK"),
            obel_buf.fwire(&format!("GCLK{ii}_U", ii = idx + 16)),
        ]);
    }
    let obel_hrow = vrf
        .find_bel_delta(bel, 0, if is_b { -21 } else { 25 }, "CLK_HROW")
        .unwrap();
    vrf.verify_node(&[
        bel.fwire("CASCI0"),
        obel_hrow.fwire(&format!("CASCO{ii}", ii = idx * 2)),
    ]);
    vrf.verify_node(&[
        bel.fwire("CASCI1"),
        obel_hrow.fwire(&format!("CASCO{ii}", ii = idx * 2 + 1)),
    ]);
}

fn verify_bufio(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFIO",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
    let idx = bel.bid.to_idx() % 4;
    let obel = vrf.find_bel_sibling(bel, "HCLK_IOI");
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I"),
        obel.wire(&format!("IOCLK_IN{idx}")),
    );
}

fn verify_bufr(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFR",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);

    let obel = vrf.find_bel_sibling(bel, "HCLK_IOI");
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I"),
            obel.wire(&format!("BUFR_CKINT{i}")),
        );
    }
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I"),
            obel.wire(&format!("IOCLK_IN{i}_BUFR")),
        );
    }
}

fn verify_idelayctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IDELAYCTRL", &[("REFCLK", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("REFCLK")]);
    let obel = vrf.find_bel_sibling(bel, "HCLK_IOI");
    for i in 0..6 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REFCLK"),
            obel.wire(&format!("HCLK_IO_D{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REFCLK"),
            obel.wire(&format!("HCLK_IO_U{i}")),
        );
    }
}

fn verify_dci(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("DCIDATA", SitePinDir::Out),
        ("DCIADDRESS0", SitePinDir::Out),
        ("DCIADDRESS1", SitePinDir::Out),
        ("DCIADDRESS2", SitePinDir::Out),
        ("DCIIOUPDATE", SitePinDir::Out),
        ("DCIREFIOUPDATE", SitePinDir::Out),
        ("DCISCLK", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "DCI", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_hclk_ioi(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_hrow = vrf
        .find_bel(bel.die, (edev.grids[bel.die].col_clk, bel.row), "CLK_HROW")
        .unwrap();
    let lr = if bel.col <= edev.grids[bel.die].col_clk {
        'L'
    } else {
        'R'
    };
    for i in 0..12 {
        vrf.claim_node(&[bel.fwire(&format!("HCLK{i}_BUF"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}_BUF")),
            bel.wire(&format!("HCLK{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK{i}")),
            obel_hrow.fwire(&format!("HCLK{i}_{lr}")),
        ]);
    }
    for i in 0..6 {
        for ud in ['U', 'D'] {
            vrf.claim_node(&[bel.fwire(&format!("HCLK_IO_{ud}{i}"))]);
            for j in 0..12 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("HCLK_IO_{ud}{i}")),
                    bel.wire(&format!("HCLK{j}_BUF")),
                );
            }
        }
    }

    for i in 0..4 {
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK{i}")),
            obel_hrow.fwire(&format!("RCLK{i}_{lr}")),
        ]);
        vrf.claim_node(&[bel.fwire(&format!("RCLK{i}_IO"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK{i}_IO")),
            bel.wire(&format!("RCLK{i}")),
        );
        vrf.claim_node(&[bel.fwire(&format!("RCLK{i}_PRE"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK{i}")),
            bel.wire(&format!("RCLK{i}_PRE")),
        );
        let obel = vrf.find_bel_sibling(bel, &format!("BUFR{i}"));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("RCLK{i}_PRE")), obel.wire("O"));
    }

    for i in 0..4 {
        vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}"))]);
        let obel = vrf.find_bel_sibling(bel, &format!("BUFIO{i}"));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("IOCLK{i}")), obel.wire("O"));
        vrf.claim_node(&[bel.fwire(&format!("IOCLK_IN{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}")),
            bel.wire(&format!("IOCLK_IN{i}_PLL")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}")),
            bel.wire(&format!("IOCLK_IN{i}_PAD")),
        );
        // XXX source PLL
        let obel = vrf
            .find_bel_delta(
                bel,
                0,
                match i {
                    0 => 0,
                    1 => 2,
                    2 => -4,
                    3 => -2,
                    _ => unreachable!(),
                },
                "ILOGIC0",
            )
            .unwrap();
        vrf.verify_node(&[bel.fwire(&format!("IOCLK_IN{i}_PAD")), obel.fwire("CLKOUT")]);
        vrf.claim_node(&[bel.fwire(&format!("IOCLK_IN{i}_BUFR"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}_BUFR")),
            bel.wire(&format!("IOCLK_IN{i}")),
        );
    }
}

fn verify_ilogic(_edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.node_kind.contains("HP") {
        "ILOGICE2"
    } else {
        "ILOGICE3"
    };
    let pins = [
        ("CLK", SitePinDir::In),
        ("CLKB", SitePinDir::In),
        ("OCLK", SitePinDir::In),
        ("OCLKB", SitePinDir::In),
        ("D", SitePinDir::In),
        ("DDLY", SitePinDir::In),
        ("OFB", SitePinDir::In),
        ("TFB", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
        ("REV", SitePinDir::In),
    ];
    let mut dummies = vec!["REV"];
    if bel.key != "ILOGIC1" {
        dummies.extend(["SHIFTIN1", "SHIFTIN2"]);
    }
    vrf.verify_bel_dummies(bel, kind, &pins, &["CKINT0", "CKINT1"], &dummies);
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }

    let obel_ologic = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ILOGIC0" => "OLOGIC0",
            "ILOGIC1" => "OLOGIC1",
            "ILOGIC" => "OLOGIC",
            _ => unreachable!(),
        },
    );

    let obel_ioi = vrf.find_bel_sibling(bel, "IOI");
    for pin in ["CLK", "CLKB"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKINT0"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKINT1"));
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_ioi.wire(&format!("IOCLK{i}")),
            );
        }
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("PHASER_TO_ICLK"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ologic.wire("PHASER_TO_OCLK"));
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKDIVP"),
        bel.wire("PHASER_TO_ICLKDIV"),
    );

    vrf.claim_pip(bel.crd(), bel.wire("OCLK"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("OCLKB"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("OCLKB"), obel_ologic.wire("CLKM"));
    vrf.claim_pip(bel.crd(), bel.wire("OFB"), obel_ologic.wire("OFB"));
    vrf.claim_pip(bel.crd(), bel.wire("TFB"), obel_ologic.wire("TFB_BUF"));

    let obel_idelay = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ILOGIC0" => "IDELAY0",
            "ILOGIC1" => "IDELAY1",
            "ILOGIC" => "IDELAY",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("DDLY"), obel_idelay.wire("DATAOUT"));

    let obel_iob = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ILOGIC0" => "IOB0",
            "ILOGIC1" => "IOB1",
            "ILOGIC" => "IOB",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("D"), bel.wire("IOB_I_BUF"));
    vrf.claim_node(&[bel.fwire("IOB_I_BUF")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOB_I_BUF"), bel.wire("IOB_I"));
    vrf.verify_node(&[bel.fwire("IOB_I"), obel_iob.fwire("I")]);

    if bel.key == "ILOGIC1" {
        let obel = vrf.find_bel_sibling(bel, "ILOGIC0");
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }

    if bel.key == "ILOGIC0" && matches!(bel.row.to_idx() % 50, 7 | 19 | 31 | 43 | 21 | 23 | 25 | 27)
    {
        vrf.claim_node(&[bel.fwire("CLKOUT")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLKOUT"), bel.wire("O"));
    }

    // XXX source phaser stuff
}

fn verify_ologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.node_kind.contains("HP") {
        "OLOGICE2"
    } else {
        "OLOGICE3"
    };
    let pins = [
        ("CLK", SitePinDir::In),
        ("CLKB", SitePinDir::In),
        ("CLKDIV", SitePinDir::In),
        ("CLKDIVB", SitePinDir::In),
        ("CLKDIVF", SitePinDir::In),
        ("CLKDIVFB", SitePinDir::In),
        ("OFB", SitePinDir::Out),
        ("TFB", SitePinDir::Out),
        ("OQ", SitePinDir::Out),
        ("TQ", SitePinDir::Out),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
        ("REV", SitePinDir::In),
        ("TBYTEIN", SitePinDir::In),
        ("TBYTEOUT", SitePinDir::Out),
    ];
    let mut dummies = vec!["REV"];
    if bel.key != "OLOGIC0" {
        dummies.extend(["SHIFTIN1", "SHIFTIN2"]);
    }
    vrf.verify_bel_dummies(
        bel,
        kind,
        &pins,
        &["CLK_CKINT", "CLKDIV_CKINT", "CLK_MUX", "TFB_BUF", "CLKDIV"],
        &dummies,
    );
    for (pin, _) in pins {
        if pin == "CLKDIV" || dummies.contains(&pin) {
            continue;
        }
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKB"), bel.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKB"), bel.wire("CLKM"));

    let obel_ioi = vrf.find_bel_sibling(bel, "IOI");
    vrf.claim_node(&[bel.fwire("CLKM")]);
    for pin in ["CLK_MUX", "CLKM"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CLK_CKINT"));
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_ioi.wire(&format!("IOCLK{i}")),
            );
        }
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("PHASER_TO_OCLK"));
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_MUX"),
        bel.wire("PHASER_TO_OCLK1X_90"),
    );

    for pin in ["CLKDIV", "CLKDIVB", "CLKDIVF", "CLKDIVFB"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CLKDIV_CKINT"));
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLKDIV"), bel.wire("PHASER_TO_OCLKDIV"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKDIVB"),
        bel.wire("PHASER_TO_OCLKDIV"),
    );

    vrf.claim_pip(bel.crd(), bel.wire("TFB_BUF"), bel.wire("TFB"));

    let obel_iob = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "OLOGIC0" => "IOB0",
            "OLOGIC1" => "IOB1",
            "OLOGIC" => "IOB",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("IOB_T"), bel.wire("TQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), bel.wire("OQ"));
    if kind == "OLOGICE2" {
        let obel_odelay = vrf.find_bel_sibling(
            bel,
            match bel.key {
                "OLOGIC0" => "ODELAY0",
                "OLOGIC1" => "ODELAY1",
                "OLOGIC" => "ODELAY",
                _ => unreachable!(),
            },
        );
        vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), obel_odelay.wire("DATAOUT"));
    }
    vrf.verify_node(&[bel.fwire("IOB_O"), obel_iob.fwire("O")]);
    vrf.verify_node(&[bel.fwire("IOB_T"), obel_iob.fwire("T")]);

    if bel.key == "OLOGIC0" {
        let obel = vrf.find_bel_sibling(bel, "OLOGIC1");
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }

    vrf.claim_pip(bel.crd(), bel.wire("TBYTEIN"), obel_ioi.wire("TBYTEIN"));
    if bel.key == "OLOGIC1" && matches!(bel.row.to_idx() % 50, 7 | 19 | 31 | 43) {
        vrf.claim_pip(bel.crd(), obel_ioi.wire("TBYTEIN"), bel.wire("TBYTEOUT"));
    }

    // XXX source phaser stuff
}

fn verify_idelay(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.node_kind.contains("HP") {
        "IDELAYE2_FINEDELAY"
    } else {
        "IDELAYE2"
    };
    let pins = [("IDATAIN", SitePinDir::In), ("DATAOUT", SitePinDir::Out)];
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_ilogic = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "IDELAY0" => "ILOGIC0",
            "IDELAY1" => "ILOGIC1",
            "IDELAY" => "ILOGIC",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("IDATAIN"),
        obel_ilogic.wire("IOB_I_BUF"),
    );

    let obel_ologic = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "IDELAY0" => "OLOGIC0",
            "IDELAY1" => "OLOGIC1",
            "IDELAY" => "OLOGIC",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("IDATAIN"), obel_ologic.wire("OFB"));
}

fn verify_odelay(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [("CLKIN", SitePinDir::In), ("ODATAIN", SitePinDir::In)];
    vrf.verify_bel(bel, "ODELAYE2", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_ologic = vrf.find_bel_sibling(
        bel,
        match bel.key {
            "ODELAY0" => "OLOGIC0",
            "ODELAY1" => "OLOGIC1",
            "ODELAY" => "OLOGIC",
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("ODATAIN"), obel_ologic.wire("OFB"));
}

fn verify_iob(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = match (bel.key, bel.node_kind) {
        ("IOB0", "IOP_HP") => "IOB18M",
        ("IOB1", "IOP_HP") => "IOB18S",
        ("IOB0", "IOP_HR") => "IOB33M",
        ("IOB1", "IOP_HR") => "IOB33S",
        ("IOB", "IOS_HP") => "IOB18",
        ("IOB", "IOS_HR") => "IOB33",
        _ => unreachable!(),
    };
    let mut pins = vec![
        ("I", SitePinDir::Out),
        ("O", SitePinDir::In),
        ("T", SitePinDir::In),
        ("O_IN", SitePinDir::In),
        ("O_OUT", SitePinDir::Out),
        ("T_IN", SitePinDir::In),
        ("T_OUT", SitePinDir::Out),
        ("DIFFO_IN", SitePinDir::In),
        ("DIFFO_OUT", SitePinDir::Out),
        ("DIFFI_IN", SitePinDir::In),
        ("PADOUT", SitePinDir::Out),
    ];
    let mut dummies = vec![];
    if bel.key != "IOB1" {
        dummies.extend(["DIFF_TERM_INT_EN", "DIFFO_IN", "O_IN", "T_IN"]);
        pins.push(("DIFF_TERM_INT_EN", SitePinDir::In));
    }
    if bel.key == "IOB" {
        dummies.push("DIFFI_IN");
    }
    vrf.verify_bel_dummies(bel, kind, &pins, &[], &dummies);
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
    if bel.key != "IOB" {
        let okey = match bel.key {
            "IOB0" => "IOB1",
            "IOB1" => "IOB0",
            _ => unreachable!(),
        };
        let obel = vrf.find_bel_sibling(bel, okey);
        if bel.key == "IOB1" {
            vrf.claim_pip(bel.crd(), bel.wire("O_IN"), obel.wire("O_OUT"));
            vrf.claim_pip(bel.crd(), bel.wire("T_IN"), obel.wire("T_OUT"));
            vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
        }
        vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
    }
}

fn verify_ioi(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let srow = grid.row_hclk(bel.row);
    let obel = vrf.find_bel(bel.die, (bel.col, srow), "HCLK_IOI").unwrap();
    let ud = if bel.row.to_idx() % 50 < 25 { 'D' } else { 'U' };
    for i in 0..6 {
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK{i}")),
            obel.fwire(&format!("HCLK_IO_{ud}{i}")),
        ]);
    }
    for i in 0..4 {
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK{i}")),
            obel.fwire(&format!("RCLK{i}_IO")),
        ]);
    }
    for i in 0..4 {
        vrf.verify_node(&[
            bel.fwire(&format!("IOCLK{i}")),
            obel.fwire(&format!("IOCLK{i}")),
        ]);
    }

    let rm = bel.row.to_idx() % 50;
    let srow = RowId::from_idx(
        bel.row.to_idx() / 50 * 50
            + match rm {
                0..=12 => 7,
                13..=24 => 19,
                25..=36 => 31,
                37..=49 => 43,
                _ => unreachable!(),
            },
    );
    if srow == bel.row {
        vrf.claim_node(&[bel.fwire("TBYTEIN")]);
    } else {
        let obel = vrf.find_bel(bel.die, (bel.col, srow), "IOI").unwrap();
        vrf.verify_node(&[bel.fwire("TBYTEIN"), obel.fwire("TBYTEIN")]);
    }
}

fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("SLICE") => verify_slice(vrf, bel),
        _ if bel.key.starts_with("DSP") => verify_dsp(vrf, bel),
        "TIEOFF" => verify_tieoff(vrf, bel),
        "BRAM_F" => verify_bram_f(vrf, bel),
        "BRAM_H0" | "BRAM_H1" => verify_bram_h(vrf, bel),
        "BRAM_ADDR" => verify_bram_addr(vrf, bel),
        "PCIE" => vrf.verify_bel(bel, "PCIE_2_1", &[], &[]),
        "PCIE3" => vrf.verify_bel(bel, "PCIE_3_0", &[], &[]),
        "PMVBRAM" | "PMV" | "PMV2" | "PMV2_SVT" | "PMVIOB" | "MTBF2" => {
            vrf.verify_bel(bel, bel.key, &[], &[])
        }

        "INT_GCLK_L" | "INT_GCLK_R" => verify_int_gclk(edev, vrf, bel),
        "HCLK_L" => verify_hclk_l(edev, vrf, bel),
        "HCLK_R" => verify_hclk_r(edev, vrf, bel),
        _ if bel.key.starts_with("GCLK_TEST_BUF") => verify_gclk_test_buf(vrf, bel),
        _ if bel.key.starts_with("BUFHCE") => verify_bufhce(vrf, bel),
        "CLK_REBUF" => verify_clk_rebuf(vrf, bel),
        "CLK_HROW" => verify_clk_hrow(edev, vrf, bel),
        _ if bel.key.starts_with("BUFGCTRL") => verify_bufgctrl(vrf, bel),

        _ if bel.key.starts_with("BUFIO") => verify_bufio(vrf, bel),
        _ if bel.key.starts_with("BUFR") => verify_bufr(vrf, bel),
        "IDELAYCTRL" => verify_idelayctrl(vrf, bel),
        "DCI" => verify_dci(vrf, bel),
        "HCLK_IOI" => verify_hclk_ioi(edev, vrf, bel),

        _ if bel.key.starts_with("ILOGIC") => verify_ilogic(edev, vrf, bel),
        _ if bel.key.starts_with("OLOGIC") => verify_ologic(vrf, bel),
        _ if bel.key.starts_with("IDELAY") => verify_idelay(vrf, bel),
        _ if bel.key.starts_with("ODELAY") => verify_odelay(vrf, bel),
        _ if bel.key.starts_with("IOB") => verify_iob(vrf, bel),
        "IOI" => verify_ioi(edev, vrf, bel),

        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}

fn verify_extra(_edev: &ExpandedDevice, vrf: &mut Verifier) {
    for w in [
        "IOI_IMUX_RC0",
        "IOI_IMUX_RC1",
        "IOI_IMUX_RC2",
        "IOI_IMUX_RC3",
        "IOI_RCLK_DIV_CE0",
        "IOI_RCLK_DIV_CE1",
        "IOI_RCLK_DIV_CE2_1",
        "IOI_RCLK_DIV_CE3_1",
        "IOI_RCLK_DIV_CLR0_1",
        "IOI_RCLK_DIV_CLR1_1",
        "IOI_RCLK_DIV_CLR2",
        "IOI_RCLK_DIV_CLR3",
        "IOI_IDELAYCTRL_RST",
        "IOI_IDELAYCTRL_DNPULSEOUT",
        "IOI_IDELAYCTRL_UPPULSEOUT",
        "IOI_IDELAYCTRL_RDY",
        "IOI_IDELAYCTRL_OUTN1",
        "IOI_IDELAYCTRL_OUTN65",
    ] {
        vrf.kill_stub_out_cond(w);
    }
    // XXX
    vrf.skip_residual();
}

pub fn verify_device(edev: &ExpandedDevice, rd: &Part) {
    verify(
        rd,
        &edev.egrid,
        |vrf, bel| verify_bel(edev, vrf, bel),
        |vrf| verify_extra(edev, vrf),
    );
}
