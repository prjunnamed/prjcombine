use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::PinDir,
    dir::{DirH, DirV},
    grid::{BelCoord, CellCoord, ColId, DieId, RowId},
};
use prjcombine_re_xilinx_naming_virtex4::{ExpandedNamedDevice, ExpandedNamedGtz};
use prjcombine_re_xilinx_rawdump::{Part, Source};
use prjcombine_re_xilinx_rdverify::{RawWireCoord, SitePin, SitePinDir, Verifier};
use prjcombine_virtex4::{
    chip::{ColumnKind, DisabledPart},
    defs::{
        bcls, bslots,
        virtex7::{tcls, wires},
    },
    expanded::{ExpandedDevice, ExpandedGtz},
    gtz::{GtzIntColId, GtzIntRowId},
};
use std::collections::{HashMap, HashSet};

fn verify_slice(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::SLICE.index_of(bcrd.slot).unwrap();
    let kind = if endev.edev.chips[bcrd.die].columns[bcrd.col] == ColumnKind::ClbLM && idx == 0 {
        "SLICEM"
    } else {
        "SLICEL"
    };
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_in("CIN")
        .extra_out_claim("COUT");
    if let Some(cell) = endev.edev.cell_delta(bcrd.cell, 0, -1)
        && let obel = cell.bel(bcrd.slot)
        && endev.edev.has_bel(obel)
    {
        bel.claim_net(&[bel.wire("CIN"), bel.bel_wire_far(obel, "COUT")]);
        bel.claim_pip(bel.bel_wire_far(obel, "COUT"), bel.bel_wire(obel, "COUT"));
    } else {
        bel.claim_net(&[bel.wire("CIN")]);
    }
    if bel.vrf.rd.source == Source::Vivado {
        if let Some(cell) = endev.edev.cell_delta(bcrd.cell, 0, 1)
            && let obel = cell.bel(bcrd.slot)
            && endev.edev.has_bel(obel)
        {
            // ok
        } else {
            bel.claim_net(&[bel.wire_far("COUT")]);
            bel.claim_pip(bel.wire_far("COUT"), bel.wire("COUT"));
        }
    }
    bel.commit();
}

fn verify_dsp(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
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
        vrf.claim_net(&[bel.wire(opin)]);
        if bcrd.slot == bslots::DSP[0] {
            if vrf.rd.source == Source::ISE
                && vrf.find_bel_delta(bel, 0, -5, bslots::DSP[1]).is_none()
            {
                vrf.claim_net(&[bel.wire(ipin)]);
            }
        } else {
            vrf.claim_net(&[bel.wire(ipin)]);
            let obel = vrf.find_bel_sibling(bel, bslots::DSP[0]);
            vrf.claim_pip(bel.wire(ipin), obel.wire(opin));

            if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, bslots::DSP[0]) {
                vrf.claim_pip(bel.wire_far(opin), bel.wire(opin));
                vrf.claim_net(&[obel.wire(ipin), bel.wire_far(opin)]);
            } else if vrf.rd.source == Source::Vivado {
                vrf.claim_pip(bel.wire_far(opin), bel.wire(opin));
                vrf.claim_net(&[bel.wire_far(opin)]);
            }
        }
    }
    vrf.verify_legacy_bel(bel, "DSP48E1", &pins, &[]);
    let obel = vrf.find_bel_sibling(bel, bslots::TIEOFF_DSP);
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
        vrf.claim_pip(bel.wire(pin), obel.wire("HARD0"));
        vrf.claim_pip(bel.wire(pin), obel.wire("HARD1"));
    }
}

fn verify_tieoff(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(
        bel,
        "TIEOFF",
        &[("HARD0", SitePinDir::Out), ("HARD1", SitePinDir::Out)],
        &[],
    );
    for pin in ["HARD0", "HARD1"] {
        vrf.claim_net(&[bel.wire(pin)]);
    }
}

fn verify_bram_f(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
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
    vrf.verify_legacy_bel(bel, "RAMBFIFO36E1", &pins, &[]);
    for (pin, _) in pins {
        if !pin.starts_with("CASCADEIN") {
            vrf.claim_net(&[bel.wire(pin)]);
        }
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bcrd.slot) {
        for (ipin, opin) in [("CASCADEINA", "CASCADEOUTA"), ("CASCADEINB", "CASCADEOUTB")] {
            vrf.claim_net(&[bel.wire(ipin), obel.wire_far(opin)]);
            vrf.claim_pip(obel.wire_far(opin), obel.wire(opin));
        }
    } else if vrf.rd.source == Source::ISE {
        for ipin in ["CASCADEINA", "CASCADEINB"] {
            vrf.claim_net(&[bel.wire(ipin)]);
        }
    }
    if vrf.rd.source == Source::Vivado && vrf.find_bel_delta(bel, 0, 5, bcrd.slot).is_none() {
        for opin in ["CASCADEOUTA", "CASCADEOUTB"] {
            vrf.claim_net(&[bel.wire_far(opin)]);
            vrf.claim_pip(bel.wire_far(opin), bel.wire(opin));
        }
    }
    let obel = vrf.find_bel_sibling(bel, bslots::BRAM_ADDR);
    for apin in &addrpins {
        vrf.claim_pip(bel.wire(apin), obel.wire(apin));
    }
    for (pin, ipin) in [
        ("ADDRARDADDRL15", "IMUX_ADDRARDADDRL15"),
        ("ADDRBWRADDRL15", "IMUX_ADDRBWRADDRL15"),
    ] {
        vrf.claim_pip(bel.wire(pin), obel.wire(ipin));
    }
}

fn verify_bram_h(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
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
    if bcrd.slot == bslots::BRAM_H[1] {
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
    vrf.verify_legacy_bel(bel, kind, &pin_refs, &[]);
    for (pin, _) in pin_refs {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, bslots::BRAM_ADDR);
    for ab in ["ARD", "BWR"] {
        for i in 0..14 {
            vrf.claim_pip(
                bel.wire(&format!("ADDR{ab}ADDR{i}")),
                obel.wire(&format!("ADDR{ab}ADDR{ul}{ii}", ii = i + 1)),
            );
        }
    }
    vrf.claim_pip(
        bel.wire("ADDRATIEHIGH0"),
        obel.wire(&format!("ADDRARDADDR{ul}0")),
    );
    vrf.claim_pip(
        bel.wire("ADDRBTIEHIGH0"),
        obel.wire(&format!("ADDRBWRADDR{ul}0")),
    );
    vrf.claim_pip(bel.wire("ADDRATIEHIGH1"), obel.wire("IMUX_ADDRARDADDRL15"));
    vrf.claim_pip(bel.wire("ADDRBTIEHIGH1"), obel.wire("IMUX_ADDRBWRADDRL15"));
}

fn verify_bram_addr(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let mut imux_addr = HashMap::new();
    let obel_t = vrf.find_bel_delta(bel, 0, 5, bcrd.slot);
    let obel_b = vrf.find_bel_delta(bel, 0, -5, bcrd.slot);
    for ab in ["ARD", "BWR"] {
        for ul in ['U', 'L'] {
            for i in 0..15 {
                let apin = format!("ADDR{ab}ADDR{ul}{i}");
                let ipin = format!("IMUX_ADDR{ab}ADDR{ul}{i}");
                let upin = format!("UTURN_ADDR{ab}ADDR{ul}{i}");
                let cibpin = format!("CASCINBOT_ADDR{ab}ADDRU{i}");
                let citpin = format!("CASCINTOP_ADDR{ab}ADDRU{i}");
                vrf.claim_net(&[bel.wire(&apin)]);
                vrf.claim_pip(bel.wire(&apin), bel.wire(&ipin));
                vrf.claim_pip(bel.wire(&apin), bel.wire(&cibpin));
                vrf.claim_pip(bel.wire(&apin), bel.wire(&citpin));
                vrf.claim_net(&[bel.wire(&upin)]);
                vrf.claim_pip(bel.wire(&upin), bel.wire(&apin));
                if ul == 'U' {
                    let copin = format!("CASCOUT_ADDR{ab}ADDRU{i}");
                    vrf.claim_net(&[bel.wire(&copin)]);
                    vrf.claim_pip(bel.wire(&copin), bel.wire(&apin));
                    if let Some(ref obel) = obel_b {
                        vrf.verify_net(&[bel.wire(&cibpin), obel.wire(&copin)]);
                    } else if vrf.rd.source == Source::ISE {
                        vrf.claim_net(&[bel.wire(&cibpin)]);
                    }
                    if let Some(ref obel) = obel_t {
                        vrf.verify_net(&[bel.wire(&citpin), obel.wire(&copin)]);
                    } else if vrf.rd.source == Source::ISE {
                        vrf.claim_net(&[bel.wire(&citpin)]);
                    }
                }
                let iwire = *bel.info.pins[&ipin].wires.iter().next().unwrap();
                imux_addr.insert(iwire, upin);
            }
        }
        let ipin = format!("IMUX_ADDR{ab}ADDRL15");
        let upin = format!("UTURN_ADDR{ab}ADDRL15");
        vrf.claim_net(&[bel.wire(&upin)]);
        vrf.claim_pip(bel.wire(&upin), bel.wire(&ipin));
        let iwire = *bel.info.pins[&ipin].wires.iter().next().unwrap();
        imux_addr.insert(iwire, upin);
    }
    for i in 0..5 {
        for j in 0..48 {
            let ipin = format!("IMUX_{i}_{j}");
            let upin = format!("IMUX_UTURN_{i}_{j}");
            let iwire = *bel.info.pins[&ipin].wires.iter().next().unwrap();
            if let Some(aupin) = imux_addr.get(&iwire) {
                vrf.claim_pip(bel.wire(&upin), bel.wire(aupin));
            } else {
                vrf.claim_pip(bel.wire(&upin), bel.wire(&ipin));
            }
        }
    }
}

fn verify_pmvbram(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("PMVBRAM");
    let tcrd = bel.vrf.grid.bel_tile(bcrd);
    if bel.vrf.grid[tcrd].class == tcls::PMVBRAM_NC {
        bel = bel
            .extra_out_claim("O")
            .extra_out_claim("ODIV2")
            .extra_out_claim("ODIV4");
        if bel.vrf.rd.source == Source::Vivado {
            bel = bel
                .extra_in("SELECT1")
                .extra_in("SELECT2")
                .extra_in("SELECT3")
                .extra_in("SELECT4");
        } else {
            bel = bel
                .extra_in_claim("SELECT1")
                .extra_in_claim("SELECT2")
                .extra_in_claim("SELECT3")
                .extra_in_claim("SELECT4");
        }
    }
    bel.commit();
}

fn verify_int_lclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let (hclk, rng) = match bcrd.slot {
        bslots::INT_LCLK_W => (bslots::HCLK_W, 6..12),
        bslots::INT_LCLK_E => (bslots::HCLK_E, 0..6),
        _ => unreachable!(),
    };
    let srow = endev.edev.chips[bel.die].row_hclk(bel.row);
    let ud = if bel.row.to_idx() % 50 < 25 { 'D' } else { 'U' };
    let obel = vrf.get_legacy_bel(bel.cell.with_row(srow).bel(hclk));
    for i in rng {
        vrf.claim_pip(
            bel.wire(&format!("LCLK{i}_O_L")),
            bel.wire(&format!("LCLK{i}_I")),
        );
        vrf.claim_pip(
            bel.wire(&format!("LCLK{i}_O_R")),
            bel.wire(&format!("LCLK{i}_I")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("LCLK{i}_I")),
            obel.wire(&format!("LCLK{i}_{ud}")),
        ]);
    }
}

fn verify_hclk_w(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let has_d = vrf.find_bel_delta(bel, 0, -1, bslots::INT_LCLK_W).is_some();
    for i in 6..12 {
        for ud in ['D', 'U'] {
            if ud == 'D' && !has_d {
                continue;
            }
            vrf.claim_net(&[bel.wire(&format!("LCLK{i}_{ud}"))]);
            for j in 0..8 {
                vrf.claim_pip(
                    bel.wire(&format!("LCLK{i}_{ud}")),
                    bel.wire(&format!("HCLK{j}_I")),
                );
            }
            for j in 8..12 {
                vrf.claim_pip(
                    bel.wire(&format!("LCLK{i}_{ud}")),
                    bel.wire(&format!("HCLK{j}")),
                );
            }
            for j in 0..4 {
                vrf.claim_pip(
                    bel.wire(&format!("LCLK{i}_{ud}")),
                    bel.wire(&format!("RCLK{j}")),
                );
            }
        }
    }
    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_E);
    let grid = endev.edev.chips[bel.die];
    let obel_hrow = vrf.get_legacy_bel(
        bel.cell
            .with_col(endev.edev.col_clk)
            .bel(bslots::CLK_HROW_V7),
    );
    let col_io = if bel.col <= endev.edev.col_clk {
        endev.edev.col_lio
    } else {
        endev.edev.col_rio
    };
    let iocol = col_io.and_then(|col| grid.get_col_io(col));
    let has_rclk = iocol
        .filter(|ioc| ioc.regs[grid.row_to_reg(bel.row)].is_some())
        .is_some();
    let lr = if bel.col <= endev.edev.col_clk {
        'L'
    } else {
        'R'
    };
    for i in 8..12 {
        vrf.claim_net(&[
            bel.wire(&format!("HCLK{i}_O")),
            obel.wire(&format!("HCLK{i}_I")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel_hrow.wire(&format!("HCLK{i}_{lr}")),
        ]);
    }
    for i in 0..4 {
        vrf.claim_net(&[
            bel.wire(&format!("RCLK{i}_O")),
            obel.wire(&format!("RCLK{i}_I")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("RCLK{i}_O")),
            bel.wire(&format!("RCLK{i}")),
        );
        if has_rclk {
            vrf.verify_net(&[
                bel.wire(&format!("RCLK{i}")),
                obel_hrow.wire(&format!("RCLK{i}_{lr}")),
            ]);
        } else {
            vrf.claim_dummy_in(bel.wire(&format!("RCLK{i}")));
        }
    }
}

fn verify_hclk_e(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let has_d = vrf.find_bel_delta(bel, 0, -1, bslots::INT_LCLK_W).is_some();
    for i in 0..6 {
        for ud in ['D', 'U'] {
            if ud == 'D' && !has_d {
                continue;
            }
            vrf.claim_net(&[bel.wire(&format!("LCLK{i}_{ud}"))]);
            for j in 0..8 {
                vrf.claim_pip(
                    bel.wire(&format!("LCLK{i}_{ud}")),
                    bel.wire(&format!("HCLK{j}")),
                );
            }
            for j in 8..12 {
                vrf.claim_pip(
                    bel.wire(&format!("LCLK{i}_{ud}")),
                    bel.wire(&format!("HCLK{j}_I")),
                );
            }
            for j in 0..4 {
                vrf.claim_pip(
                    bel.wire(&format!("LCLK{i}_{ud}")),
                    bel.wire(&format!("RCLK{j}_I")),
                );
            }
        }
    }
    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_W);
    let obel_hrow = vrf.get_legacy_bel(
        bel.cell
            .with_col(endev.edev.col_clk)
            .bel(bslots::CLK_HROW_V7),
    );
    let lr = if bel.col <= endev.edev.col_clk {
        'L'
    } else {
        'R'
    };
    for i in 0..8 {
        vrf.claim_net(&[
            bel.wire(&format!("HCLK{i}_O")),
            obel.wire(&format!("HCLK{i}_I")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel_hrow.wire(&format!("HCLK{i}_{lr}")),
        ]);
    }
}

fn verify_gclk_test_buf(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let kind = if bel.name.unwrap().starts_with("BUFG") {
        "BUFG_LB"
    } else {
        "GCLK_TEST_BUF"
    };
    vrf.verify_legacy_bel(
        bel,
        kind,
        &[("CLKIN", SitePinDir::In), ("CLKOUT", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.wire("CLKIN")]);
    vrf.claim_net(&[bel.wire("CLKOUT")]);
}

fn verify_bufhce(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(
        bel,
        "BUFHCE",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.wire("I")]);
    vrf.claim_net(&[bel.wire("O")]);
}

fn verify_clk_rebuf(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel = vrf
        .find_bel_walk(bel, 0, -1, bslots::CLK_REBUF)
        .or_else(|| {
            if bel.die.to_idx() != 0 {
                let odie = bel.die - 1;
                let srow = vrf.grid.rows(odie).last().unwrap() - 19;
                vrf.find_bel(CellCoord::new(odie, bel.col, srow).bel(bslots::CLK_REBUF))
            } else {
                None
            }
        });
    for i in 0..32 {
        let pin_d = format!("GCLK{i}_D");
        let pin_u = format!("GCLK{i}_U");
        vrf.claim_net(&[bel.wire(&pin_u)]);
        vrf.claim_pip(bel.wire(&pin_d), bel.wire(&pin_u));
        vrf.claim_pip(bel.wire(&pin_u), bel.wire(&pin_d));
        let obel_buf_d = vrf.find_bel_sibling(bel, bslots::GCLK_TEST_BUF_REBUF_S[i / 2]);
        let obel_buf_u = vrf.find_bel_sibling(bel, bslots::GCLK_TEST_BUF_REBUF_N[i / 2]);
        if i.is_multiple_of(2) {
            vrf.claim_pip(obel_buf_d.wire("CLKIN"), bel.wire(&pin_d));
            vrf.claim_pip(bel.wire(&pin_u), obel_buf_u.wire("CLKOUT"));
        } else {
            vrf.claim_pip(bel.wire(&pin_d), obel_buf_d.wire("CLKOUT"));
            vrf.claim_pip(obel_buf_u.wire("CLKIN"), bel.wire(&pin_u));
        }
        if let Some(ref obel) = obel {
            vrf.verify_net(&[bel.wire(&pin_d), obel.wire(&pin_u)]);
        } else {
            vrf.claim_net(&[bel.wire(&pin_d)]);
        }
    }
}

fn verify_clk_hrow(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let grid = endev.edev.chips[bel.die];
    let obel_casc = vrf.find_bel_delta(
        bel,
        0,
        if grid.row_to_reg(bel.row) < grid.reg_clk {
            -50
        } else {
            50
        },
        bslots::CLK_HROW_V7,
    );
    let obel_buf = vrf.find_bel_walk(bel, 0, -1, bslots::CLK_REBUF).unwrap();
    for i in 0..32 {
        vrf.verify_net(&[
            bel.wire(&format!("GCLK{i}")),
            obel_buf.wire(&format!("GCLK{i}_U")),
        ]);
        vrf.claim_net(&[bel.wire(&format!("GCLK{i}_TEST_IN"))]);
        vrf.claim_net(&[bel.wire(&format!("GCLK{i}_TEST_OUT"))]);
        vrf.claim_net(&[bel.wire(&format!("GCLK_TEST{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("GCLK{i}_TEST_IN")),
            bel.wire(&format!("GCLK{i}")),
        );
        let obel = vrf.find_bel_sibling(bel, bslots::GCLK_TEST_BUF_HROW_GCLK[i]);
        vrf.claim_pip(obel.wire("CLKIN"), bel.wire(&format!("GCLK{i}_TEST_IN")));
        vrf.claim_pip(bel.wire(&format!("GCLK{i}_TEST_OUT")), obel.wire("CLKOUT"));
        vrf.claim_pip(
            bel.wire(&format!("GCLK_TEST{i}")),
            bel.wire(&format!("GCLK{i}_TEST_OUT")),
        );
        vrf.claim_pip(
            bel.wire(&format!("GCLK_TEST{ii}", ii = i ^ 1)),
            bel.wire(&format!("GCLK{i}_TEST_OUT")),
        );

        vrf.claim_net(&[bel.wire(&format!("CASCO{i}"))]);
        if let Some(ref obel_casc) = obel_casc {
            vrf.verify_net(&[
                bel.wire(&format!("CASCI{i}")),
                obel_casc.wire(&format!("CASCO{i}")),
            ]);
        } else if vrf.rd.source == Source::ISE {
            vrf.claim_net(&[bel.wire(&format!("CASCI{i}"))]);
        }
        vrf.claim_pip(
            bel.wire(&format!("CASCO{i}")),
            bel.wire(&format!("CASCI{i}")),
        );
        vrf.claim_pip(
            bel.wire(&format!("CASCO{i}")),
            bel.wire(&format!("GCLK_TEST{i}")),
        );
        vrf.claim_pip(bel.wire(&format!("CASCO{i}")), bel.wire("HCLK_TEST_OUT_L"));
        vrf.claim_pip(bel.wire(&format!("CASCO{i}")), bel.wire("HCLK_TEST_OUT_R"));
        for lr in ['L', 'R'] {
            for j in 0..4 {
                vrf.claim_pip(
                    bel.wire(&format!("CASCO{i}")),
                    bel.wire(&format!("RCLK{j}_{lr}")),
                );
            }
            for j in 0..14 {
                vrf.claim_pip(
                    bel.wire(&format!("CASCO{i}")),
                    bel.wire(&format!("HIN{j}_{lr}")),
                );
            }
        }
    }

    for (lr, gclk_test_buf, bufhce) in [
        ('L', bslots::GCLK_TEST_BUF_HROW_BUFH_W, bslots::BUFHCE_W),
        ('R', bslots::GCLK_TEST_BUF_HROW_BUFH_E, bslots::BUFHCE_E),
    ] {
        let col_io = if lr == 'L' {
            endev.edev.col_lio
        } else {
            endev.edev.col_rio
        };
        let iocol = col_io.and_then(|col| grid.get_col_io(col));
        let obel = vrf.find_bel_sibling(bel, gclk_test_buf);
        vrf.claim_net(&[bel.wire(&format!("HCLK_TEST_IN_{lr}"))]);
        vrf.claim_pip(obel.wire("CLKIN"), bel.wire(&format!("HCLK_TEST_IN_{lr}")));
        vrf.claim_net(&[bel.wire(&format!("HCLK_TEST_OUT_{lr}"))]);
        vrf.claim_pip(
            bel.wire(&format!("HCLK_TEST_OUT_{lr}")),
            obel.wire("CLKOUT"),
        );
        for i in 0..14 {
            vrf.claim_pip(
                bel.wire(&format!("HCLK_TEST_IN_{lr}")),
                bel.wire(&format!("HIN{i}_{lr}")),
            );
        }
        for i in 0..12 {
            vrf.claim_net(&[bel.wire(&format!("HCLK{i}_{lr}"))]);
            let obel = vrf.find_bel_sibling(bel, bufhce[i]);
            vrf.claim_pip(bel.wire(&format!("HCLK{i}_{lr}")), obel.wire("O"));

            if (lr == 'R' && i < 6) || (lr == 'L' && i >= 6) {
                vrf.claim_pip(obel.wire("I"), bel.wire("BUFHCE_CKINT0"));
                vrf.claim_pip(obel.wire("I"), bel.wire("BUFHCE_CKINT1"));
            } else {
                vrf.claim_pip(obel.wire("I"), bel.wire("BUFHCE_CKINT2"));
                vrf.claim_pip(obel.wire("I"), bel.wire("BUFHCE_CKINT3"));
            }
            vrf.claim_pip(obel.wire("I"), bel.wire("HCLK_TEST_OUT_L"));
            vrf.claim_pip(obel.wire("I"), bel.wire("HCLK_TEST_OUT_R"));
            for olr in ['L', 'R'] {
                for j in 0..14 {
                    vrf.claim_pip(obel.wire("I"), bel.wire(&format!("HIN{j}_{olr}")));
                }
            }
            for j in 0..32 {
                vrf.claim_pip(obel.wire("I"), bel.wire(&format!("GCLK{j}")));
            }
        }
        let reg = grid.row_to_reg(bel.row);
        let has_rclk = iocol.filter(|ioc| ioc.regs[reg].is_some()).is_some();
        for i in 0..4 {
            if has_rclk {
                vrf.claim_net(&[bel.wire(&format!("RCLK{i}_{lr}"))]);
            } else {
                vrf.claim_dummy_in(bel.wire(&format!("RCLK{i}_{lr}")));
            }
        }

        let mut has_gtp_mid = false;
        if let Some((cl, cr)) = endev.edev.col_mgt {
            let gtcol = grid.get_col_gt(if lr == 'L' { cl } else { cr }).unwrap();
            if gtcol.regs[reg].is_some() {
                has_gtp_mid = true;
                let obel = vrf.get_legacy_bel(bel.cell.with_col(gtcol.col).bel(bslots::GTP_COMMON));
                for i in 0..14 {
                    vrf.verify_net(&[
                        bel.wire(&format!("HIN{i}_{lr}")),
                        obel.wire(&format!("HOUT{i}")),
                    ]);
                }
            }
        }
        if !has_gtp_mid {
            let mut has_io = false;
            if let Some(iocol) = iocol
                && iocol.regs[reg].is_some()
            {
                has_io = true;
                let scol = ColId::from_idx(iocol.col.to_idx() ^ 1);
                let obel = vrf.get_legacy_bel(bel.cell.with_col(scol).bel(bslots::HCLK_CMT));
                for i in 0..14 {
                    vrf.verify_net(&[
                        bel.wire(&format!("HIN{i}_{lr}")),
                        obel.wire(&format!("HOUT{i}")),
                    ]);
                }
            }
            if !has_io {
                let mut has_gt = false;
                if let Some(gc) = if lr == 'L' {
                    endev.edev.col_lgt
                } else {
                    endev.edev.col_rgt
                } {
                    let gtcol = grid.get_col_gt(gc).unwrap();
                    if gtcol.regs[reg].is_some() {
                        has_gt = true;
                        let obel = vrf
                            .find_bel(bel.cell.with_col(gtcol.col).bel(bslots::GTP_COMMON))
                            .or_else(|| {
                                vrf.find_bel(bel.cell.with_col(gtcol.col).bel(bslots::GTX_COMMON))
                            })
                            .or_else(|| {
                                vrf.find_bel(bel.cell.with_col(gtcol.col).bel(bslots::GTH_COMMON))
                            })
                            .unwrap();
                        for i in 0..4 {
                            vrf.claim_dummy_in(bel.wire(&format!("HIN{i}_{lr}")));
                        }
                        for i in 4..14 {
                            vrf.verify_net(&[
                                bel.wire(&format!("HIN{i}_{lr}")),
                                obel.wire(&format!("HOUT{i}")),
                            ]);
                        }
                    }
                }
                if !has_gt {
                    if grid.has_ps && reg == grid.reg_cfg - 1 && lr == 'L' {
                        let obel = vrf.get_legacy_bel(
                            bel.cell
                                .with_cr(grid.col_ps(), bel.row + 25)
                                .bel(bslots::HCLK_PS_S),
                        );
                        for i in 0..4 {
                            vrf.verify_net(&[
                                bel.wire(&format!("HIN{i}_{lr}")),
                                obel.wire(&format!("HOUT{i}")),
                            ]);
                        }
                        for i in 4..14 {
                            vrf.claim_dummy_in(bel.wire(&format!("HIN{i}_{lr}")));
                        }
                    } else if grid.has_ps && reg == grid.reg_cfg && lr == 'L' {
                        let obel = vrf.get_legacy_bel(
                            bel.cell
                                .with_cr(grid.col_ps(), bel.row - 25)
                                .bel(bslots::HCLK_PS_N),
                        );
                        for i in 0..6 {
                            vrf.verify_net(&[
                                bel.wire(&format!("HIN{i}_{lr}")),
                                obel.wire(&format!("HOUT{i}")),
                            ]);
                        }
                        for i in 6..14 {
                            vrf.claim_dummy_in(bel.wire(&format!("HIN{i}_{lr}")));
                        }
                    } else {
                        for i in 0..14 {
                            vrf.claim_dummy_in(bel.wire(&format!("HIN{i}_{lr}")));
                        }
                    }
                }
            }
        }
    }
}

fn verify_bufgctrl(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(
        bel,
        "BUFGCTRL",
        &[
            ("I0", SitePinDir::In),
            ("I1", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &["CKINT0", "CKINT1", "FB_TEST0", "FB_TEST1"],
    );
    vrf.claim_net(&[bel.wire("I0")]);
    vrf.claim_net(&[bel.wire("I1")]);
    vrf.claim_pip(bel.wire("I0"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.wire("I0"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.wire("I1"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.wire("I1"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.wire("I0"), bel.wire("CASCI0"));
    vrf.claim_pip(bel.wire("I1"), bel.wire("CASCI1"));
    // very likely a case of wrong-direction pip
    vrf.claim_pip(bel.wire("I0"), bel.wire("FB_TEST0"));
    vrf.claim_pip(bel.wire("I1"), bel.wire("FB_TEST1"));
    let idx = bslots::BUFGCTRL.index_of(bcrd.slot).unwrap();
    for d in [1, 15] {
        let oidx = (idx + d) % 16;
        let obel = vrf.find_bel_sibling(bel, bslots::BUFGCTRL[oidx]);
        vrf.claim_pip(bel.wire("I0"), obel.wire("FB"));
        vrf.claim_pip(bel.wire("I1"), obel.wire("FB"));
    }

    vrf.claim_net(&[bel.wire("O")]);
    vrf.claim_net(&[bel.wire("FB")]);
    vrf.claim_pip(bel.wire("FB"), bel.wire("O"));
    vrf.claim_pip(bel.wire("GCLK"), bel.wire("O"));

    let is_b = !bel.row.to_idx().is_multiple_of(50);
    let obel_buf = vrf.find_bel_walk(bel, 0, -1, bslots::CLK_REBUF).unwrap();
    if is_b {
        vrf.verify_net(&[bel.wire("GCLK"), obel_buf.wire(&format!("GCLK{idx}_U"))]);
    } else {
        vrf.verify_net(&[
            bel.wire("GCLK"),
            obel_buf.wire(&format!("GCLK{ii}_U", ii = idx + 16)),
        ]);
    }
    let obel_hrow = vrf
        .find_bel_delta(bel, 0, if is_b { -21 } else { 25 }, bslots::CLK_HROW_V7)
        .unwrap();
    vrf.verify_net(&[
        bel.wire("CASCI0"),
        obel_hrow.wire(&format!("CASCO{ii}", ii = idx * 2)),
    ]);
    vrf.verify_net(&[
        bel.wire("CASCI1"),
        obel_hrow.wire(&format!("CASCO{ii}", ii = idx * 2 + 1)),
    ]);
}

fn verify_bufio(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(
        bel,
        "BUFIO",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.wire("I")]);
    vrf.claim_net(&[bel.wire("O")]);
    let idx = bslots::BUFIO.index_of(bcrd.slot).unwrap();
    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_IO);
    vrf.claim_pip(bel.wire("I"), obel.wire(&format!("IOCLK_IN{idx}")));
}

fn verify_bufr(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(
        bel,
        "BUFR",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.wire("I")]);
    vrf.claim_net(&[bel.wire("O")]);

    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_IO);
    for i in 0..4 {
        vrf.claim_pip(bel.wire("I"), obel.wire(&format!("BUFR_CKINT{i}")));
    }
    for i in 0..4 {
        vrf.claim_pip(bel.wire("I"), obel.wire(&format!("IOCLK_IN{i}_BUFR")));
    }
}

fn verify_idelayctrl(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "IDELAYCTRL", &[("REFCLK", SitePinDir::In)], &[]);
    vrf.claim_net(&[bel.wire("REFCLK")]);
    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_IO);
    for i in 0..6 {
        vrf.claim_pip(bel.wire("REFCLK"), obel.wire(&format!("HCLK_IO_D{i}")));
        vrf.claim_pip(bel.wire("REFCLK"), obel.wire(&format!("HCLK_IO_U{i}")));
    }
}

fn verify_dci(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("DCIDATA", SitePinDir::Out),
        ("DCIADDRESS0", SitePinDir::Out),
        ("DCIADDRESS1", SitePinDir::Out),
        ("DCIADDRESS2", SitePinDir::Out),
        ("DCIIOUPDATE", SitePinDir::Out),
        ("DCIREFIOUPDATE", SitePinDir::Out),
        ("DCISCLK", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "DCI", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
}

fn verify_hclk_ioi(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel_hrow = vrf.get_legacy_bel(
        bel.cell
            .with_col(endev.edev.col_clk)
            .bel(bslots::CLK_HROW_V7),
    );
    let lr = if bel.col <= endev.edev.col_clk {
        'L'
    } else {
        'R'
    };
    for i in 0..12 {
        vrf.claim_net(&[bel.wire(&format!("HCLK{i}_BUF"))]);
        vrf.claim_pip(
            bel.wire(&format!("HCLK{i}_BUF")),
            bel.wire(&format!("HCLK{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel_hrow.wire(&format!("HCLK{i}_{lr}")),
        ]);
    }
    for i in 0..6 {
        for ud in ['U', 'D'] {
            vrf.claim_net(&[bel.wire(&format!("HCLK_IO_{ud}{i}"))]);
            for j in 0..12 {
                vrf.claim_pip(
                    bel.wire(&format!("HCLK_IO_{ud}{i}")),
                    bel.wire(&format!("HCLK{j}_BUF")),
                );
            }
        }
    }

    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("RCLK{i}")),
            obel_hrow.wire(&format!("RCLK{i}_{lr}")),
        ]);
        vrf.claim_net(&[bel.wire(&format!("RCLK{i}_IO"))]);
        vrf.claim_pip(
            bel.wire(&format!("RCLK{i}_IO")),
            bel.wire(&format!("RCLK{i}")),
        );
        vrf.claim_net(&[bel.wire(&format!("RCLK{i}_PRE"))]);
        vrf.claim_pip(
            bel.wire(&format!("RCLK{i}")),
            bel.wire(&format!("RCLK{i}_PRE")),
        );
        let obel = vrf.find_bel_sibling(bel, bslots::BUFR[i]);
        vrf.claim_pip(bel.wire(&format!("RCLK{i}_PRE")), obel.wire("O"));
    }

    let obel_hclk_cmt = vrf.get_legacy_bel(
        bel.cell
            .with_col(ColId::from_idx(bel.col.to_idx() ^ 1))
            .bel(bslots::HCLK_CMT),
    );
    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("IOCLK{i}"))]);
        let obel = vrf.find_bel_sibling(bel, bslots::BUFIO[i]);
        vrf.claim_pip(bel.wire(&format!("IOCLK{i}")), obel.wire("O"));
        vrf.claim_net(&[bel.wire(&format!("IOCLK_IN{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("IOCLK_IN{i}")),
            bel.wire(&format!("IOCLK_IN{i}_PERF")),
        );
        vrf.claim_pip(
            bel.wire(&format!("IOCLK_IN{i}")),
            bel.wire(&format!("IOCLK_IN{i}_PAD")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("IOCLK_IN{i}_PERF")),
            obel_hclk_cmt.wire(&format!("PERF{i}")),
        ]);
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
                bslots::ILOGIC[1],
            )
            .unwrap();
        vrf.verify_net(&[bel.wire(&format!("IOCLK_IN{i}_PAD")), obel.wire("CLKOUT")]);
        vrf.claim_net(&[bel.wire(&format!("IOCLK_IN{i}_BUFR"))]);
        vrf.claim_pip(
            bel.wire(&format!("IOCLK_IN{i}_BUFR")),
            bel.wire(&format!("IOCLK_IN{i}")),
        );
    }
}

fn verify_ilogic(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::ILOGIC.index_of(bcrd.slot).unwrap();
    let is_single = !bel.tcls.ends_with("_PAIR");
    let kind = if bel.tcls.contains("HP") {
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
    if bcrd.slot != bslots::ILOGIC[0] || is_single {
        dummies.extend(["SHIFTIN1", "SHIFTIN2"]);
    }
    vrf.verify_bel_dummies(bel, kind, &pins, &["CKINT0", "CKINT1"], &dummies);
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_net(&[bel.wire(pin)]);
        }
    }

    let obel_ologic = vrf.find_bel_sibling(bel, bslots::OLOGIC[idx]);

    let obel_ioi = vrf.find_bel_sibling(bel, bslots::IOI);
    for pin in ["CLK", "CLKB"] {
        vrf.claim_pip(bel.wire(pin), bel.wire("CKINT0"));
        vrf.claim_pip(bel.wire(pin), bel.wire("CKINT1"));
        for i in 0..6 {
            vrf.claim_pip(bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), obel_ioi.wire(&format!("IOCLK{i}")));
        }
        vrf.claim_pip(bel.wire(pin), bel.wire("PHASER_ICLK"));
        vrf.claim_pip(bel.wire(pin), obel_ologic.wire("PHASER_OCLK"));
    }
    vrf.claim_pip(bel.wire("CLKDIVP"), bel.wire("PHASER_ICLKDIV"));

    vrf.claim_pip(bel.wire("OCLK"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.wire("OCLKB"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.wire("OCLKB"), obel_ologic.wire("CLKM"));
    vrf.claim_pip(bel.wire("OFB"), obel_ologic.wire("OFB"));
    vrf.claim_pip(bel.wire("TFB"), obel_ologic.wire("TFB_BUF"));

    let obel_idelay = vrf.find_bel_sibling(bel, bslots::IDELAY[idx]);
    vrf.claim_pip(bel.wire("DDLY"), obel_idelay.wire("DATAOUT"));

    let obel_iob = vrf.find_bel_sibling(bel, bslots::IOB[idx]);
    vrf.claim_pip(bel.wire("D"), bel.wire("IOB_I_BUF"));
    vrf.claim_net(&[bel.wire("IOB_I_BUF")]);
    vrf.claim_pip(bel.wire("IOB_I_BUF"), bel.wire("IOB_I"));
    vrf.verify_net(&[bel.wire("IOB_I"), obel_iob.wire("I")]);

    if bcrd.slot == bslots::ILOGIC[0] && !is_single {
        let obel = vrf.find_bel_sibling(bel, bslots::ILOGIC[1]);
        vrf.claim_pip(bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }

    if bcrd.slot == bslots::ILOGIC[1] {
        let has_clkout = match vrf.rd.source {
            Source::ISE => matches!(bel.row.to_idx() % 50, 7 | 19 | 31 | 43 | 21 | 23 | 25 | 27),
            Source::Vivado => !matches!(bel.row.to_idx() % 50, 13 | 37),
        };
        if has_clkout {
            vrf.claim_net(&[bel.wire("CLKOUT")]);
            vrf.claim_pip(bel.wire("CLKOUT"), bel.wire("O"));
        }
    }

    let y = bel.row.to_idx() % 50 + idx;
    let cmt = match y {
        0..=15 => bslots::CMT_A,
        16..=24 => bslots::CMT_B,
        25..=36 => bslots::CMT_C,
        37..=49 => bslots::CMT_D,
        _ => unreachable!(),
    };
    let obel_cmt = vrf.get_legacy_bel(
        bel.cell
            .with_cr(
                ColId::from_idx(bel.col.to_idx() ^ 1),
                endev.edev.chips[bel.die].row_hclk(bel.row),
            )
            .bel(cmt),
    );
    vrf.verify_net(&[
        bel.wire("PHASER_ICLK"),
        obel_cmt.wire(&format!("IO{y}_ICLK")),
    ]);
    vrf.verify_net(&[
        bel.wire("PHASER_ICLKDIV"),
        obel_cmt.wire(&format!("IO{y}_ICLKDIV")),
    ]);
}

fn verify_ologic(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::OLOGIC.index_of(bcrd.slot).unwrap();
    let kind = if bel.tcls.contains("HP") {
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
    if bcrd.slot != bslots::OLOGIC[1] {
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
        vrf.claim_net(&[bel.wire(pin)]);
    }

    vrf.claim_pip(bel.wire("CLK"), bel.wire("CLK_MUX"));
    vrf.claim_pip(bel.wire("CLKB"), bel.wire("CLK_MUX"));
    vrf.claim_pip(bel.wire("CLKB"), bel.wire("CLKM"));

    let obel_ioi = vrf.find_bel_sibling(bel, bslots::IOI);
    vrf.claim_net(&[bel.wire("CLKM")]);
    for pin in ["CLK_MUX", "CLKM"] {
        vrf.claim_pip(bel.wire(pin), bel.wire("CLK_CKINT"));
        for i in 0..6 {
            vrf.claim_pip(bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), obel_ioi.wire(&format!("IOCLK{i}")));
        }
        vrf.claim_pip(bel.wire(pin), bel.wire("PHASER_OCLK"));
    }
    vrf.claim_pip(bel.wire("CLK_MUX"), bel.wire("PHASER_OCLK90"));

    for pin in ["CLKDIV", "CLKDIVB", "CLKDIVF", "CLKDIVFB"] {
        vrf.claim_pip(bel.wire(pin), bel.wire("CLKDIV_CKINT"));
        for i in 0..6 {
            vrf.claim_pip(bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
    }
    vrf.claim_pip(bel.wire("CLKDIV"), bel.wire("PHASER_OCLKDIV"));
    vrf.claim_pip(bel.wire("CLKDIVB"), bel.wire("PHASER_OCLKDIV"));

    vrf.claim_pip(bel.wire("TFB_BUF"), bel.wire("TFB"));

    let obel_iob = vrf.find_bel_sibling(bel, bslots::IOB[idx]);
    vrf.claim_pip(bel.wire("IOB_T"), bel.wire("TQ"));
    vrf.claim_pip(bel.wire("IOB_O"), bel.wire("OQ"));
    if kind == "OLOGICE2" {
        let obel_odelay = vrf.find_bel_sibling(bel, bslots::ODELAY[idx]);
        vrf.claim_pip(bel.wire("IOB_O"), obel_odelay.wire("DATAOUT"));
    }
    vrf.verify_net(&[bel.wire("IOB_O"), obel_iob.wire("O")]);
    vrf.verify_net(&[bel.wire("IOB_T"), obel_iob.wire("T")]);

    if bcrd.slot == bslots::OLOGIC[1] {
        let obel = vrf.find_bel_sibling(bel, bslots::OLOGIC[0]);
        vrf.claim_pip(bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }

    vrf.claim_pip(bel.wire("TBYTEIN"), obel_ioi.wire("TBYTEIN"));
    if bcrd.slot == bslots::OLOGIC[0] && matches!(bel.row.to_idx() % 50, 7 | 19 | 31 | 43) {
        vrf.claim_pip(obel_ioi.wire("TBYTEIN"), bel.wire("TBYTEOUT"));
    }

    let y = bel.row.to_idx() % 50 + idx;
    let cmt = match y {
        0..=15 => bslots::CMT_A,
        16..=24 => bslots::CMT_B,
        25..=36 => bslots::CMT_C,
        37..=49 => bslots::CMT_D,
        _ => unreachable!(),
    };
    let obel_cmt = vrf.get_legacy_bel(
        bel.cell
            .with_cr(
                ColId::from_idx(bel.col.to_idx() ^ 1),
                endev.edev.chips[bel.die].row_hclk(bel.row),
            )
            .bel(cmt),
    );
    vrf.verify_net(&[
        bel.wire("PHASER_OCLK"),
        obel_cmt.wire(&format!("IO{y}_OCLK")),
    ]);
    vrf.verify_net(&[
        bel.wire("PHASER_OCLKDIV"),
        obel_cmt.wire(&format!("IO{y}_OCLKDIV")),
    ]);
    if matches!(y, 8 | 20 | 32 | 44) {
        vrf.verify_net(&[
            bel.wire("PHASER_OCLK90"),
            obel_cmt.wire(&format!("IO{y}_OCLK90")),
        ]);
    } else {
        vrf.claim_dummy_in(bel.wire("PHASER_OCLK90"));
    }
}

fn verify_idelay(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::IDELAY.index_of(bcrd.slot).unwrap();
    let kind = if bel.tcls.contains("HP") {
        "IDELAYE2_FINEDELAY"
    } else {
        "IDELAYE2"
    };
    let pins = [("IDATAIN", SitePinDir::In), ("DATAOUT", SitePinDir::Out)];
    vrf.verify_legacy_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let obel_ilogic = vrf.find_bel_sibling(bel, bslots::ILOGIC[idx]);
    vrf.claim_pip(bel.wire("IDATAIN"), obel_ilogic.wire("IOB_I_BUF"));

    let obel_ologic = vrf.find_bel_sibling(bel, bslots::OLOGIC[idx]);
    vrf.claim_pip(bel.wire("IDATAIN"), obel_ologic.wire("OFB"));
}

fn verify_odelay(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::ODELAY.index_of(bcrd.slot).unwrap();
    let pins = [("CLKIN", SitePinDir::In), ("ODATAIN", SitePinDir::In)];
    vrf.verify_legacy_bel(bel, "ODELAYE2", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let obel_ologic = vrf.find_bel_sibling(bel, bslots::OLOGIC[idx]);
    vrf.claim_pip(bel.wire("CLKIN"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.wire("ODATAIN"), obel_ologic.wire("OFB"));
}

fn verify_iob(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::IOB.index_of(bcrd.slot).unwrap();
    let (kind, is_single) = match (idx, bel.tile.class) {
        (1, tcls::IO_HP_PAIR) => ("IOB18M", false),
        (0, tcls::IO_HP_PAIR) => ("IOB18S", false),
        (1, tcls::IO_HR_PAIR) => ("IOB33M", false),
        (0, tcls::IO_HR_PAIR) => ("IOB33S", false),
        (0, tcls::IO_HP_S) => ("IOB18", true),
        (0, tcls::IO_HP_N) => ("IOB18", true),
        (0, tcls::IO_HR_S) => ("IOB33", true),
        (0, tcls::IO_HR_N) => ("IOB33", true),
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
    if bcrd.slot != bslots::IOB[0] || is_single {
        dummies.extend(["DIFF_TERM_INT_EN", "DIFFO_IN", "O_IN", "T_IN"]);
        pins.push(("DIFF_TERM_INT_EN", SitePinDir::In));
    }
    if is_single {
        dummies.push("DIFFI_IN");
    }
    vrf.verify_bel_dummies(bel, kind, &pins, &[], &dummies);
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_net(&[bel.wire(pin)]);
        }
    }
    if !is_single {
        let oslot = bslots::IOB[idx ^ 1];
        let obel = vrf.find_bel_sibling(bel, oslot);
        if bcrd.slot == bslots::IOB[0] {
            vrf.claim_pip(bel.wire("O_IN"), obel.wire("O_OUT"));
            vrf.claim_pip(bel.wire("T_IN"), obel.wire("T_OUT"));
            vrf.claim_pip(bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
        }
        vrf.claim_pip(bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
    }
}

fn verify_ioi(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let grid = endev.edev.chips[bel.die];
    let srow = grid.row_hclk(bel.row);
    let obel = vrf.get_legacy_bel(bel.cell.with_row(srow).bel(bslots::HCLK_IO));
    let ud = if bel.row.to_idx() % 50 < 25 { 'D' } else { 'U' };
    for i in 0..6 {
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel.wire(&format!("HCLK_IO_{ud}{i}")),
        ]);
    }
    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("RCLK{i}")),
            obel.wire(&format!("RCLK{i}_IO")),
        ]);
    }
    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("IOCLK{i}")),
            obel.wire(&format!("IOCLK{i}")),
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
        vrf.claim_net(&[bel.wire("TBYTEIN")]);
    } else {
        let obel = vrf.get_legacy_bel(bel.cell.with_row(srow).bel(bslots::IOI));
        vrf.verify_net(&[bel.wire("TBYTEIN"), obel.wire("TBYTEIN")]);
    }
}

fn verify_phaser_in(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::PHASER_IN.index_of(bcrd.slot).unwrap();
    let pins = [
        ("MEMREFCLK", SitePinDir::In),
        ("FREQREFCLK", SitePinDir::In),
        ("PHASEREFCLK", SitePinDir::In),
        ("SYNCIN", SitePinDir::In),
        ("ENCALIBPHY0", SitePinDir::In),
        ("ENCALIBPHY1", SitePinDir::In),
        ("RANKSELPHY0", SitePinDir::In),
        ("RANKSELPHY1", SitePinDir::In),
        ("BURSTPENDINGPHY", SitePinDir::In),
        ("ICLK", SitePinDir::Out),
        ("ICLKDIV", SitePinDir::Out),
        ("RCLK", SitePinDir::Out),
        ("WRENABLE", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "PHASER_IN_PHY", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let obel_pc = vrf.find_bel_sibling(bel, bslots::PHY_CONTROL);
    let (abcd, cmt) = match idx {
        0 => ('A', bslots::CMT_B),
        1 => ('B', bslots::CMT_B),
        2 => ('C', bslots::CMT_C),
        3 => ('D', bslots::CMT_C),
        _ => unreachable!(),
    };
    for (pin, opin) in [
        ("ENCALIBPHY0", "PCENABLECALIB0"),
        ("ENCALIBPHY1", "PCENABLECALIB1"),
        ("RANKSELPHY0", &format!("INRANK{abcd}0")),
        ("RANKSELPHY1", &format!("INRANK{abcd}1")),
        ("BURSTPENDINGPHY", &format!("INBURSTPENDING{idx}")),
    ] {
        vrf.claim_pip(bel.wire(pin), bel.wire_far(pin));
        vrf.verify_net(&[bel.wire_far(pin), obel_pc.wire_far(opin)]);
    }

    vrf.claim_net(&[bel.wire_far("RCLK")]);
    vrf.claim_pip(bel.wire_far("RCLK"), bel.wire("RCLK"));

    let obel_cmt = vrf.find_bel_sibling(bel, cmt);
    for pin in ["MEMREFCLK", "FREQREFCLK", "SYNCIN"] {
        vrf.claim_pip(bel.wire(pin), obel_cmt.wire(pin));
    }
    vrf.claim_net(&[bel.wire_far("PHASEREFCLK")]);
    vrf.claim_pip(bel.wire("PHASEREFCLK"), bel.wire_far("PHASEREFCLK"));
    vrf.claim_pip(bel.wire_far("PHASEREFCLK"), bel.wire("DQS_PAD"));
    for pin in [
        "MRCLK0", "MRCLK1", "MRCLK0_S", "MRCLK1_S", "MRCLK0_N", "MRCLK1_N",
    ] {
        vrf.claim_pip(bel.wire_far("PHASEREFCLK"), obel_cmt.wire(pin));
    }

    let dx = if bel.col.to_idx().is_multiple_of(2) {
        1
    } else {
        -1
    };
    let dy = [-18, -6, 6, 18][idx];
    let obel_ilogic = vrf.find_bel_delta(bel, dx, dy, bslots::ILOGIC[1]).unwrap();
    vrf.verify_net(&[bel.wire("DQS_PAD"), obel_ilogic.wire("CLKOUT")]);

    vrf.claim_net(&[bel.wire("IO_ICLK")]);
    vrf.claim_net(&[bel.wire("IO_ICLKDIV")]);
    vrf.claim_net(&[bel.wire("FIFO_WRCLK")]);
    vrf.claim_net(&[bel.wire("FIFO_WREN")]);
    vrf.claim_pip(bel.wire("FIFO_WRCLK"), bel.wire("ICLKDIV"));
    vrf.claim_pip(bel.wire("FIFO_WREN"), bel.wire("WRENABLE"));
    vrf.claim_pip(bel.wire("IO_ICLK"), bel.wire("ICLK"));
    vrf.claim_pip(bel.wire("IO_ICLKDIV"), bel.wire("FIFO_WRCLK"));
}

fn verify_phaser_out(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::PHASER_OUT.index_of(bcrd.slot).unwrap();
    let pins = [
        ("MEMREFCLK", SitePinDir::In),
        ("FREQREFCLK", SitePinDir::In),
        ("PHASEREFCLK", SitePinDir::In),
        ("SYNCIN", SitePinDir::In),
        ("ENCALIBPHY0", SitePinDir::In),
        ("ENCALIBPHY1", SitePinDir::In),
        ("BURSTPENDINGPHY", SitePinDir::In),
        ("OCLK", SitePinDir::Out),
        ("OCLKDELAYED", SitePinDir::Out),
        ("OCLKDIV", SitePinDir::Out),
        ("RDENABLE", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "PHASER_OUT_PHY", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let obel_pc = vrf.find_bel_sibling(bel, bslots::PHY_CONTROL);
    let cmt = match idx {
        0 => bslots::CMT_B,
        1 => bslots::CMT_B,
        2 => bslots::CMT_C,
        3 => bslots::CMT_C,
        _ => unreachable!(),
    };
    for (pin, opin) in [
        ("ENCALIBPHY0", "PCENABLECALIB0"),
        ("ENCALIBPHY1", "PCENABLECALIB1"),
        ("BURSTPENDINGPHY", &format!("OUTBURSTPENDING{idx}")),
    ] {
        vrf.claim_pip(bel.wire(pin), bel.wire_far(pin));
        vrf.verify_net(&[bel.wire_far(pin), obel_pc.wire_far(opin)]);
    }

    let obel_cmt = vrf.find_bel_sibling(bel, cmt);
    for pin in ["MEMREFCLK", "FREQREFCLK", "SYNCIN"] {
        vrf.claim_pip(bel.wire(pin), obel_cmt.wire(pin));
    }

    vrf.claim_net(&[bel.wire_far("PHASEREFCLK")]);
    vrf.claim_pip(bel.wire("PHASEREFCLK"), bel.wire_far("PHASEREFCLK"));
    for pin in [
        "MRCLK0", "MRCLK1", "MRCLK0_S", "MRCLK1_S", "MRCLK0_N", "MRCLK1_N",
    ] {
        vrf.claim_pip(bel.wire_far("PHASEREFCLK"), obel_cmt.wire(pin));
    }

    vrf.claim_net(&[bel.wire("IO_OCLK")]);
    vrf.claim_net(&[bel.wire("IO_OCLK90")]);
    vrf.claim_net(&[bel.wire("IO_OCLKDIV")]);
    vrf.claim_net(&[bel.wire("FIFO_RDCLK")]);
    vrf.claim_net(&[bel.wire("FIFO_RDEN")]);
    vrf.claim_pip(bel.wire("FIFO_RDCLK"), bel.wire("OCLKDIV"));
    vrf.claim_pip(bel.wire("FIFO_RDEN"), bel.wire("RDENABLE"));
    vrf.claim_pip(bel.wire("IO_OCLK"), bel.wire("OCLK"));
    vrf.claim_pip(bel.wire("IO_OCLK90"), bel.wire("OCLKDELAYED"));
    vrf.claim_pip(bel.wire("IO_OCLKDIV"), bel.wire("FIFO_RDCLK"));
}

fn verify_phaser_ref(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("CLKIN", SitePinDir::In),
        ("CLKOUT", SitePinDir::Out),
        ("TMUXOUT", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "PHASER_REF", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    for pin in ["CLKOUT", "TMUXOUT"] {
        vrf.claim_net(&[bel.wire_far(pin)]);
        vrf.claim_pip(bel.wire_far(pin), bel.wire(pin));
    }
    let obel_cmt = vrf.find_bel_sibling(bel, bslots::CMT_C);
    vrf.claim_pip(bel.wire("CLKIN"), obel_cmt.wire("FREQREFCLK"));
}

fn verify_phy_control(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("MEMREFCLK", SitePinDir::In),
        ("SYNCIN", SitePinDir::In),
        ("INRANKA0", SitePinDir::Out),
        ("INRANKA1", SitePinDir::Out),
        ("INRANKB0", SitePinDir::Out),
        ("INRANKB1", SitePinDir::Out),
        ("INRANKC0", SitePinDir::Out),
        ("INRANKC1", SitePinDir::Out),
        ("INRANKD0", SitePinDir::Out),
        ("INRANKD1", SitePinDir::Out),
        ("PCENABLECALIB0", SitePinDir::Out),
        ("PCENABLECALIB1", SitePinDir::Out),
        ("INBURSTPENDING0", SitePinDir::Out),
        ("INBURSTPENDING1", SitePinDir::Out),
        ("INBURSTPENDING2", SitePinDir::Out),
        ("INBURSTPENDING3", SitePinDir::Out),
        ("OUTBURSTPENDING0", SitePinDir::Out),
        ("OUTBURSTPENDING1", SitePinDir::Out),
        ("OUTBURSTPENDING2", SitePinDir::Out),
        ("OUTBURSTPENDING3", SitePinDir::Out),
        ("PHYCTLMSTREMPTY", SitePinDir::In),
        ("PHYCTLEMPTY", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "PHY_CONTROL", &pins, &[]);
    for (pin, dir) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
        if dir == SitePinDir::Out {
            vrf.claim_net(&[bel.wire_far(pin)]);
            vrf.claim_pip(bel.wire_far(pin), bel.wire(pin));
        }
    }

    vrf.claim_net(&[bel.wire("SYNC_BB")]);
    vrf.claim_net(&[bel.wire_far("PHYCTLMSTREMPTY")]);
    vrf.claim_pip(bel.wire("PHYCTLMSTREMPTY"), bel.wire_far("PHYCTLMSTREMPTY"));
    vrf.claim_pip(bel.wire_far("PHYCTLMSTREMPTY"), bel.wire("SYNC_BB"));
    vrf.claim_pip(bel.wire("SYNC_BB"), bel.wire_far("PHYCTLEMPTY"));

    let obel_cmt = vrf.find_bel_sibling(bel, bslots::CMT_C);
    for pin in ["MEMREFCLK", "SYNCIN"] {
        vrf.claim_pip(bel.wire(pin), obel_cmt.wire(pin));
    }
}

fn verify_mmcm(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("CLKIN1", SitePinDir::In),
        ("CLKIN2", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
        ("CLKFBOUT", SitePinDir::Out),
        ("CLKFBOUTB", SitePinDir::Out),
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT0B", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT1B", SitePinDir::Out),
        ("CLKOUT2", SitePinDir::Out),
        ("CLKOUT2B", SitePinDir::Out),
        ("CLKOUT3", SitePinDir::Out),
        ("CLKOUT3B", SitePinDir::Out),
        ("CLKOUT4", SitePinDir::Out),
        ("CLKOUT5", SitePinDir::Out),
        ("CLKOUT6", SitePinDir::Out),
        ("TMUXOUT", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(
        bel,
        "MMCME2_ADV",
        &pins,
        &["CLKIN1_CKINT", "CLKIN2_CKINT", "CLKFBIN_CKINT"],
    );
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    vrf.claim_net(&[bel.wire("CLKFB")]);
    vrf.claim_pip(bel.wire("CLKFB"), bel.wire("CLKFBOUT"));
    vrf.claim_pip(bel.wire("CLKIN1"), bel.wire("CLKIN1_CKINT"));
    vrf.claim_pip(bel.wire("CLKIN1"), bel.wire("CLKIN1_HCLK"));
    vrf.claim_pip(bel.wire("CLKIN2"), bel.wire("CLKIN2_CKINT"));
    vrf.claim_pip(bel.wire("CLKIN2"), bel.wire("CLKIN2_HCLK"));
    vrf.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFBIN_CKINT"));
    vrf.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFBIN_HCLK"));
    vrf.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFB"));
    for pin in ["CLKIN1", "CLKIN2", "CLKFBIN"] {
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("FREQ_BB{i}_IN")));
        }
    }
    let obel = vrf.find_bel_sibling(bel, bslots::CMT_A);
    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}_IN"))]);
        vrf.claim_pip(
            bel.wire(&format!("FREQ_BB{i}_IN")),
            obel.wire(&format!("FREQ_BB{i}")),
        );
    }
    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_CMT);
    vrf.verify_net(&[bel.wire("CLKIN1_HCLK"), obel.wire("MMCM_CLKIN1")]);
    vrf.verify_net(&[bel.wire("CLKIN2_HCLK"), obel.wire("MMCM_CLKIN2")]);
    vrf.verify_net(&[bel.wire("CLKFBIN_HCLK"), obel.wire("MMCM_CLKFBIN")]);
    for (i, pin) in [
        (0, "CLKOUT0"),
        (1, "CLKOUT0B"),
        (2, "CLKOUT1"),
        (3, "CLKOUT1B"),
        (4, "CLKOUT2"),
        (5, "CLKOUT2B"),
        (6, "CLKOUT3"),
        (7, "CLKOUT3B"),
        (8, "CLKOUT4"),
        (9, "CLKOUT5"),
        (10, "CLKOUT6"),
        (11, "CLKFBOUT"),
        (12, "CLKFBOUTB"),
        (13, "TMUXOUT"),
    ] {
        vrf.claim_net(&[bel.wire(&format!("OUT{i}"))]);
        vrf.claim_pip(bel.wire(&format!("OUT{i}")), bel.wire(pin));
    }
    for (i, pin) in [
        (0, "CLKOUT0"),
        (1, "CLKOUT1"),
        (2, "CLKOUT2"),
        (3, "CLKOUT3"),
    ] {
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB_OUT{i}"))]);
        vrf.claim_pip(bel.wire(&format!("FREQ_BB_OUT{i}")), bel.wire(pin));
    }
    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("PERF{i}"))]);
        for pin in ["CLKFBOUT", "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3"] {
            vrf.claim_pip(bel.wire(&format!("PERF{i}")), bel.wire(pin));
        }
    }
}

fn verify_pll(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("CLKIN1", SitePinDir::In),
        ("CLKIN2", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
        ("CLKFBOUT", SitePinDir::Out),
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT2", SitePinDir::Out),
        ("CLKOUT3", SitePinDir::Out),
        ("CLKOUT4", SitePinDir::Out),
        ("CLKOUT5", SitePinDir::Out),
        ("TMUXOUT", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(
        bel,
        "PLLE2_ADV",
        &pins,
        &["CLKIN1_CKINT", "CLKIN2_CKINT", "CLKFBIN_CKINT"],
    );
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    vrf.claim_net(&[bel.wire("CLKFB")]);
    vrf.claim_pip(bel.wire("CLKFB"), bel.wire("CLKFBOUT"));
    vrf.claim_pip(bel.wire("CLKIN1"), bel.wire("CLKIN1_CKINT"));
    vrf.claim_pip(bel.wire("CLKIN1"), bel.wire("CLKIN1_HCLK"));
    vrf.claim_pip(bel.wire("CLKIN2"), bel.wire("CLKIN2_CKINT"));
    vrf.claim_pip(bel.wire("CLKIN2"), bel.wire("CLKIN2_HCLK"));
    vrf.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFBIN_CKINT"));
    vrf.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFBIN_HCLK"));
    vrf.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFB"));
    for pin in ["CLKIN1", "CLKIN2", "CLKFBIN"] {
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("FREQ_BB{i}_IN")));
        }
    }
    let obel = vrf.find_bel_sibling(bel, bslots::CMT_D);
    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}_IN"))]);
        vrf.claim_pip(
            bel.wire(&format!("FREQ_BB{i}_IN")),
            obel.wire(&format!("FREQ_BB{i}")),
        );
    }
    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_CMT);
    vrf.verify_net(&[bel.wire("CLKIN1_HCLK"), obel.wire("PLL_CLKIN1")]);
    vrf.verify_net(&[bel.wire("CLKIN2_HCLK"), obel.wire("PLL_CLKIN2")]);
    vrf.verify_net(&[bel.wire("CLKFBIN_HCLK"), obel.wire("PLL_CLKFBIN")]);
    for (i, pin) in [
        (0, "CLKOUT0"),
        (1, "CLKOUT1"),
        (2, "CLKOUT2"),
        (3, "CLKOUT3"),
        (4, "CLKOUT4"),
        (5, "CLKOUT5"),
        (6, "CLKFBOUT"),
        (7, "TMUXOUT"),
    ] {
        vrf.claim_net(&[bel.wire(&format!("OUT{i}"))]);
        vrf.claim_pip(bel.wire(&format!("OUT{i}")), bel.wire(pin));
    }
    for (i, pin) in [
        (0, "CLKOUT0"),
        (1, "CLKOUT1"),
        (2, "CLKOUT2"),
        (3, "CLKOUT3"),
    ] {
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB_OUT{i}"))]);
        vrf.claim_pip(bel.wire(&format!("FREQ_BB_OUT{i}")), bel.wire(pin));
    }
}

fn verify_bufmrce(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(
        bel,
        "BUFMRCE",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.wire("I")]);
    vrf.claim_net(&[bel.wire("O")]);

    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_CMT);
    for i in 4..14 {
        vrf.claim_pip(bel.wire("I"), obel.wire(&format!("HIN{i}")));
    }
    vrf.claim_pip(bel.wire("I"), obel.wire("CKINT0"));
    vrf.claim_pip(bel.wire("I"), obel.wire("CKINT1"));
    if bcrd.slot == bslots::BUFMRCE[0] {
        vrf.claim_pip(bel.wire("I"), obel.wire("CCIO0"));
    } else {
        vrf.claim_pip(bel.wire("I"), obel.wire("CCIO3"));
    }
}

fn verify_hclk_cmt(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let grid = endev.edev.chips[bel.die];
    let obel_hrow = vrf.get_legacy_bel(
        bel.cell
            .with_col(endev.edev.col_clk)
            .bel(bslots::CLK_HROW_V7),
    );
    let lr = if bel.col <= endev.edev.col_clk {
        'L'
    } else {
        'R'
    };
    for i in 0..12 {
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel_hrow.wire(&format!("HCLK{i}_{lr}")),
        ]);
    }
    for i in 0..2 {
        for ud in ['U', 'D'] {
            vrf.claim_net(&[bel.wire(&format!("LCLK{i}_CMT_{ud}"))]);
            for j in 0..12 {
                vrf.claim_pip(
                    bel.wire(&format!("LCLK{i}_CMT_{ud}")),
                    bel.wire(&format!("HCLK{j}")),
                );
            }
            for j in 0..4 {
                vrf.claim_pip(
                    bel.wire(&format!("LCLK{i}_CMT_{ud}")),
                    bel.wire(&format!("RCLK{j}")),
                );
            }
        }
    }

    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("RCLK{i}")),
            obel_hrow.wire(&format!("RCLK{i}_{lr}")),
        ]);
    }

    let obel_hclk_ioi = vrf.get_legacy_bel(
        bel.cell
            .with_col(ColId::from_idx(bel.col.to_idx() ^ 1))
            .bel(bslots::HCLK_IO),
    );
    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("CCIO{i}")),
            obel_hclk_ioi.wire(&format!("IOCLK_IN{i}_PAD")),
        ]);
    }

    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}"))]);
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}_MUX"))]);
        vrf.claim_pip(
            bel.wire(&format!("FREQ_BB{i}")),
            bel.wire(&format!("FREQ_BB{i}_MUX")),
        );
        vrf.claim_pip(
            bel.wire(&format!("FREQ_BB{i}_MUX")),
            bel.wire(&format!("CCIO{i}")),
        );
        vrf.claim_pip(
            bel.wire(&format!("FREQ_BB{i}_MUX")),
            bel.wire(&format!("CKINT{i}")),
        );
    }

    for pin in [
        "MMCM_CLKIN1",
        "MMCM_CLKIN2",
        "MMCM_CLKFBIN",
        "PLL_CLKIN1",
        "PLL_CLKIN2",
        "PLL_CLKFBIN",
    ] {
        vrf.claim_net(&[bel.wire(pin)]);
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("CCIO{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("PHASER_REF_BOUNCE{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("RCLK{i}")));
        }
        for i in 0..12 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("HCLK{i}")));
        }
        for i in 4..14 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("HIN{i}")));
        }
    }

    for i in 0..14 {
        vrf.claim_net(&[bel.wire(&format!("HOUT{i}"))]);
        for j in 0..4 {
            vrf.claim_pip(bel.wire(&format!("HOUT{i}")), bel.wire(&format!("CCIO{j}")));
        }
        for j in 0..12 {
            vrf.claim_pip(bel.wire(&format!("HOUT{i}")), bel.wire(&format!("HCLK{j}")));
        }
        for j in 4..14 {
            vrf.claim_pip(bel.wire(&format!("HOUT{i}")), bel.wire(&format!("HIN{j}")));
        }
        for j in 0..14 {
            vrf.claim_pip(
                bel.wire(&format!("HOUT{i}")),
                bel.wire(&format!("MMCM_OUT{j}")),
            );
        }
        for j in 0..8 {
            vrf.claim_pip(
                bel.wire(&format!("HOUT{i}")),
                bel.wire(&format!("PLL_OUT{j}")),
            );
        }
        for j in 0..4 {
            vrf.claim_pip(
                bel.wire(&format!("HOUT{i}")),
                bel.wire(&format!("PHASER_REF_BOUNCE{j}")),
            );
        }
    }

    let mut has_gt = false;
    if let Some(gc) = if lr == 'L' {
        endev.edev.col_lgt
    } else {
        endev.edev.col_rgt
    } {
        let gtcol = grid.get_col_gt(gc).unwrap();
        if gtcol.regs[grid.row_to_reg(bel.row)].is_some() {
            has_gt = true;
            let obel = vrf
                .find_bel(bel.cell.with_col(gtcol.col).bel(bslots::GTP_COMMON))
                .or_else(|| vrf.find_bel(bel.cell.with_col(gtcol.col).bel(bslots::GTX_COMMON)))
                .or_else(|| vrf.find_bel(bel.cell.with_col(gtcol.col).bel(bslots::GTH_COMMON)))
                .unwrap();
            for i in 4..14 {
                vrf.verify_net(&[bel.wire(&format!("HIN{i}")), obel.wire(&format!("HOUT{i}"))]);
            }
        }
    }
    if !has_gt {
        for i in 4..14 {
            vrf.claim_dummy_in(bel.wire(&format!("HIN{i}")));
        }
    }

    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("PERF{i}"))]);
        for j in 0..4 {
            vrf.claim_pip(
                bel.wire(&format!("PERF{i}")),
                bel.wire(&format!("PHASER_IN_RCLK{j}")),
            );
        }
        if i < 2 {
            vrf.claim_pip(bel.wire(&format!("PERF{i}")), bel.wire("MMCM_PERF0"));
            vrf.claim_pip(bel.wire(&format!("PERF{i}")), bel.wire("MMCM_PERF1"));
        } else {
            vrf.claim_pip(bel.wire(&format!("PERF{i}")), bel.wire("MMCM_PERF2"));
            vrf.claim_pip(bel.wire(&format!("PERF{i}")), bel.wire("MMCM_PERF3"));
        }

        let obel_pi = vrf.find_bel_sibling(bel, bslots::PHASER_IN[i]);
        vrf.verify_net(&[
            bel.wire(&format!("PHASER_IN_RCLK{i}")),
            obel_pi.wire_far("RCLK"),
        ]);
    }

    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("PHASER_REF_BOUNCE{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("PHASER_REF_BOUNCE{i}")),
            bel.wire("PHASER_REF_CLKOUT"),
        );
        vrf.claim_pip(
            bel.wire(&format!("PHASER_REF_BOUNCE{i}")),
            bel.wire("PHASER_REF_TMUXOUT"),
        );
    }
    let obel_pref = vrf.find_bel_sibling(bel, bslots::PHASER_REF);
    vrf.verify_net(&[bel.wire("PHASER_REF_CLKOUT"), obel_pref.wire_far("CLKOUT")]);
    vrf.verify_net(&[
        bel.wire("PHASER_REF_TMUXOUT"),
        obel_pref.wire_far("TMUXOUT"),
    ]);

    for i in 0..2 {
        vrf.claim_net(&[bel.wire(&format!("MRCLK{i}"))]);
        let obel = vrf.find_bel_sibling(bel, bslots::BUFMRCE[i]);
        vrf.claim_pip(bel.wire(&format!("MRCLK{i}")), obel.wire("O"));
    }

    let obel_mmcm = vrf.find_bel_sibling(bel, bslots::MMCM[0]);
    for i in 0..14 {
        vrf.verify_net(&[
            bel.wire(&format!("MMCM_OUT{i}")),
            obel_mmcm.wire(&format!("OUT{i}")),
        ]);
    }
    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("MMCM_PERF{i}")),
            obel_mmcm.wire(&format!("PERF{i}")),
        ]);
    }
    let obel_pll = vrf.find_bel_sibling(bel, bslots::PLL);
    for i in 0..8 {
        vrf.verify_net(&[
            bel.wire(&format!("PLL_OUT{i}")),
            obel_pll.wire(&format!("OUT{i}")),
        ]);
    }
}

fn verify_cmt_a(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel_hclk = vrf.find_bel_sibling(bel, bslots::HCLK_CMT);
    let obel_s = vrf.find_bel_walk(bel, 0, -50, bslots::CMT_D).or_else(|| {
        if bel.die.to_idx() != 0 {
            let odie = bel.die - 1;
            let srow = vrf.grid.rows(odie).last().unwrap() - 24;
            vrf.find_bel(CellCoord::new(odie, bel.col, srow).bel(bslots::CMT_D))
        } else {
            None
        }
    });
    let obel_pc = vrf.find_bel_sibling(bel, bslots::PHY_CONTROL);
    let mut has_conn = false;
    if let Some(ref obel_s) = obel_s
        && obel_s.die == bel.die
    {
        vrf.verify_net(&[bel.wire("SYNC_BB"), obel_pc.wire("SYNC_BB")]);
        vrf.claim_pip(bel.wire("SYNC_BB_S"), bel.wire("SYNC_BB"));
        vrf.claim_pip(bel.wire("SYNC_BB"), bel.wire("SYNC_BB_S"));
        vrf.claim_net(&[bel.wire("SYNC_BB_S"), obel_s.wire("SYNC_BB_N")]);
        has_conn = true;
    }
    if !has_conn && vrf.rd.source == Source::Vivado {
        vrf.verify_net(&[bel.wire("SYNC_BB"), obel_pc.wire("SYNC_BB")]);
        vrf.claim_pip(bel.wire("SYNC_BB_S"), bel.wire("SYNC_BB"));
        vrf.claim_pip(bel.wire("SYNC_BB"), bel.wire("SYNC_BB_S"));
        vrf.claim_net(&[bel.wire("SYNC_BB_S")]);
    }
    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("FREQ_BB{i}")),
            obel_hclk.wire(&format!("FREQ_BB{i}")),
        ]);
        if let Some(ref obel_s) = obel_s {
            vrf.claim_net(&[
                bel.wire(&format!("FREQ_BB{i}_S")),
                obel_s.wire(&format!("FREQ_BB{i}_N")),
            ]);
            vrf.claim_pip(
                bel.wire(&format!("FREQ_BB{i}")),
                bel.wire(&format!("FREQ_BB{i}_S")),
            );
            vrf.claim_pip(
                bel.wire(&format!("FREQ_BB{i}_S")),
                bel.wire(&format!("FREQ_BB{i}")),
            );
        } else if vrf.rd.source == Source::Vivado {
            vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}_S"))]);
            vrf.claim_pip(
                bel.wire(&format!("FREQ_BB{i}")),
                bel.wire(&format!("FREQ_BB{i}_S")),
            );
            vrf.claim_pip(
                bel.wire(&format!("FREQ_BB{i}_S")),
                bel.wire(&format!("FREQ_BB{i}")),
            );
        }
    }

    for i in 0..13 {
        for pin in ["ICLK", "ICLKDIV", "OCLK", "OCLKDIV"] {
            vrf.claim_net(&[bel.wire(&format!("IO{i}_{pin}"))]);
            vrf.claim_pip(
                bel.wire(&format!("IO{i}_{pin}")),
                bel.wire(&format!("PHASER_A_{pin}_BUF")),
            );
        }
    }
    for i in 13..16 {
        for pin in ["ICLK", "ICLKDIV", "OCLK", "OCLKDIV"] {
            vrf.claim_net(&[bel.wire(&format!("IO{i}_{pin}"))]);
            vrf.claim_pip(
                bel.wire(&format!("IO{i}_{pin}")),
                bel.wire(&format!("PHASER_B_{pin}")),
            );
        }
    }
    vrf.claim_net(&[bel.wire("IO8_OCLK90")]);
    vrf.claim_pip(bel.wire("IO8_OCLK90"), bel.wire("PHASER_A_OCLK90_BUF"));
    for pin in ["ICLK", "ICLKDIV", "OCLK", "OCLKDIV", "OCLK90"] {
        vrf.claim_net(&[bel.wire(&format!("PHASER_A_{pin}_BUF"))]);
        vrf.claim_pip(
            bel.wire(&format!("PHASER_A_{pin}_BUF")),
            bel.wire(&format!("PHASER_A_{pin}")),
        );
    }

    let obel_pi_a = vrf.find_bel_sibling(bel, bslots::PHASER_IN[0]);
    vrf.verify_net(&[bel.wire("PHASER_A_ICLK"), obel_pi_a.wire("IO_ICLK")]);
    vrf.verify_net(&[bel.wire("PHASER_A_ICLKDIV"), obel_pi_a.wire("IO_ICLKDIV")]);
    let obel_po_a = vrf.find_bel_sibling(bel, bslots::PHASER_OUT[0]);
    vrf.verify_net(&[bel.wire("PHASER_A_OCLK"), obel_po_a.wire("IO_OCLK")]);
    vrf.verify_net(&[bel.wire("PHASER_A_OCLKDIV"), obel_po_a.wire("IO_OCLKDIV")]);
    vrf.verify_net(&[bel.wire("PHASER_A_OCLK90"), obel_po_a.wire("IO_OCLK90")]);

    let obel_b = vrf.find_bel_sibling(bel, bslots::CMT_B);
    vrf.verify_net(&[bel.wire("PHASER_B_ICLK"), obel_b.wire("PHASER_B_ICLK_A")]);
    vrf.verify_net(&[
        bel.wire("PHASER_B_ICLKDIV"),
        obel_b.wire("PHASER_B_ICLKDIV_A"),
    ]);
    vrf.verify_net(&[bel.wire("PHASER_B_OCLK"), obel_b.wire("PHASER_B_OCLK_A")]);
    vrf.verify_net(&[
        bel.wire("PHASER_B_OCLKDIV"),
        obel_b.wire("PHASER_B_OCLKDIV_A"),
    ]);
}

fn verify_cmt_b(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel_hclk = vrf.find_bel_sibling(bel, bslots::HCLK_CMT);
    let obel_mmcm = vrf.find_bel_sibling(bel, bslots::MMCM[0]);
    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("FREQ_BB{i}")),
            obel_hclk.wire(&format!("FREQ_BB{i}")),
        ]);
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}_MUX"))]);
        vrf.verify_net(&[
            bel.wire(&format!("MMCM_FREQ_BB{i}")),
            obel_mmcm.wire(&format!("FREQ_BB_OUT{i}")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("FREQ_BB{i}")),
            bel.wire(&format!("FREQ_BB{i}_MUX")),
        );
        for j in 0..4 {
            vrf.claim_pip(
                bel.wire(&format!("FREQ_BB{i}_MUX")),
                bel.wire(&format!("MMCM_FREQ_BB{j}")),
            );
        }
    }
    let obel_c = vrf.find_bel_sibling(bel, bslots::CMT_C);
    for pin in ["FREQREFCLK", "MEMREFCLK", "SYNCIN"] {
        vrf.verify_net(&[bel.wire(pin), obel_c.wire(pin)]);
    }

    for i in 0..2 {
        vrf.verify_net(&[
            bel.wire(&format!("MRCLK{i}")),
            obel_hclk.wire(&format!("MRCLK{i}")),
        ]);
    }
    if let Some(obel_hclk_n) = vrf.find_bel_delta(bel, 0, 50, bslots::HCLK_CMT) {
        for i in 0..2 {
            vrf.verify_net(&[
                bel.wire(&format!("MRCLK{i}_S")),
                obel_hclk_n.wire(&format!("MRCLK{i}")),
            ]);
        }
    } else {
        for i in 0..2 {
            vrf.claim_dummy_in(bel.wire(&format!("MRCLK{i}_S")));
        }
    }
    if let Some(obel_hclk_s) = vrf.find_bel_delta(bel, 0, -50, bslots::HCLK_CMT) {
        for i in 0..2 {
            vrf.verify_net(&[
                bel.wire(&format!("MRCLK{i}_N")),
                obel_hclk_s.wire(&format!("MRCLK{i}")),
            ]);
        }
    } else {
        for i in 0..2 {
            vrf.claim_dummy_in(bel.wire(&format!("MRCLK{i}_N")));
        }
    }

    for i in 16..25 {
        for pin in ["ICLK", "ICLKDIV", "OCLK", "OCLKDIV"] {
            vrf.claim_net(&[bel.wire(&format!("IO{i}_{pin}"))]);
            vrf.claim_pip(
                bel.wire(&format!("IO{i}_{pin}")),
                bel.wire(&format!("PHASER_B_{pin}_BUF")),
            );
        }
    }
    vrf.claim_net(&[bel.wire("IO20_OCLK90")]);
    vrf.claim_pip(bel.wire("IO20_OCLK90"), bel.wire("PHASER_B_OCLK90_BUF"));

    vrf.claim_net(&[bel.wire("PHASER_B_ICLK_BUF")]);
    vrf.claim_net(&[bel.wire("PHASER_B_ICLKDIV_BUF")]);
    vrf.claim_net(&[bel.wire("PHASER_B_OCLK_BUF")]);
    vrf.claim_net(&[bel.wire("PHASER_B_OCLKDIV_BUF")]);
    vrf.claim_net(&[bel.wire("PHASER_B_OCLK90_BUF")]);
    let obel_pi_b = vrf.find_bel_sibling(bel, bslots::PHASER_IN[1]);
    vrf.claim_pip(bel.wire("PHASER_B_ICLK_BUF"), obel_pi_b.wire("IO_ICLK"));
    vrf.claim_pip(
        bel.wire("PHASER_B_ICLKDIV_BUF"),
        obel_pi_b.wire("IO_ICLKDIV"),
    );
    let obel_po_b = vrf.find_bel_sibling(bel, bslots::PHASER_OUT[1]);
    vrf.claim_pip(bel.wire("PHASER_B_OCLK_BUF"), obel_po_b.wire("IO_OCLK"));
    vrf.claim_pip(
        bel.wire("PHASER_B_OCLKDIV_BUF"),
        obel_po_b.wire("IO_OCLKDIV"),
    );
    vrf.claim_pip(bel.wire("PHASER_B_OCLK90_BUF"), obel_po_b.wire("IO_OCLK90"));

    vrf.claim_net(&[bel.wire("PHASER_B_ICLK_A")]);
    vrf.claim_net(&[bel.wire("PHASER_B_ICLKDIV_A")]);
    vrf.claim_net(&[bel.wire("PHASER_B_OCLK_A")]);
    vrf.claim_net(&[bel.wire("PHASER_B_OCLKDIV_A")]);
    vrf.claim_pip(bel.wire("PHASER_B_ICLK_A"), bel.wire("PHASER_B_ICLK_BUF"));
    vrf.claim_pip(
        bel.wire("PHASER_B_ICLKDIV_A"),
        bel.wire("PHASER_B_ICLKDIV_BUF"),
    );
    vrf.claim_pip(bel.wire("PHASER_B_OCLK_A"), bel.wire("PHASER_B_OCLK_BUF"));
    vrf.claim_pip(
        bel.wire("PHASER_B_OCLKDIV_A"),
        bel.wire("PHASER_B_OCLKDIV_BUF"),
    );
}

fn verify_cmt_c(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel_hclk = vrf.find_bel_sibling(bel, bslots::HCLK_CMT);
    let obel_pll = vrf.find_bel_sibling(bel, bslots::PLL);
    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("FREQ_BB{i}")),
            obel_hclk.wire(&format!("FREQ_BB{i}")),
        ]);
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}_MUX"))]);
        vrf.verify_net(&[
            bel.wire(&format!("PLL_FREQ_BB{i}")),
            obel_pll.wire(&format!("FREQ_BB_OUT{i}")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("FREQ_BB{i}")),
            bel.wire(&format!("FREQ_BB{i}_MUX")),
        );
        for j in 0..4 {
            vrf.claim_pip(
                bel.wire(&format!("FREQ_BB{i}_MUX")),
                bel.wire(&format!("PLL_FREQ_BB{j}")),
            );
        }
        vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}_REF"))]);
        vrf.claim_pip(
            bel.wire(&format!("FREQ_BB{i}_REF")),
            bel.wire(&format!("FREQ_BB{i}")),
        );
    }
    for pin in ["FREQREFCLK", "MEMREFCLK", "SYNCIN"] {
        vrf.claim_net(&[bel.wire(pin)]);
        for i in 0..4 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("FREQ_BB{i}_REF")));
        }
    }
    vrf.claim_pip(bel.wire("FREQREFCLK"), bel.wire("PLL_FREQ_BB0"));
    vrf.claim_pip(bel.wire("MEMREFCLK"), bel.wire("PLL_FREQ_BB1"));
    vrf.claim_pip(bel.wire("SYNCIN"), bel.wire("PLL_FREQ_BB2"));

    for i in 0..2 {
        vrf.verify_net(&[
            bel.wire(&format!("MRCLK{i}")),
            obel_hclk.wire(&format!("MRCLK{i}")),
        ]);
    }
    if let Some(obel_hclk_n) = vrf.find_bel_delta(bel, 0, 50, bslots::HCLK_CMT) {
        for i in 0..2 {
            vrf.verify_net(&[
                bel.wire(&format!("MRCLK{i}_S")),
                obel_hclk_n.wire(&format!("MRCLK{i}")),
            ]);
        }
    } else {
        for i in 0..2 {
            vrf.claim_dummy_in(bel.wire(&format!("MRCLK{i}_S")));
        }
    }
    if let Some(obel_hclk_s) = vrf.find_bel_delta(bel, 0, -50, bslots::HCLK_CMT) {
        for i in 0..2 {
            vrf.verify_net(&[
                bel.wire(&format!("MRCLK{i}_N")),
                obel_hclk_s.wire(&format!("MRCLK{i}")),
            ]);
        }
    } else {
        for i in 0..2 {
            vrf.claim_dummy_in(bel.wire(&format!("MRCLK{i}_N")));
        }
    }

    for i in 25..37 {
        for pin in ["ICLK", "ICLKDIV", "OCLK", "OCLKDIV"] {
            vrf.claim_net(&[bel.wire(&format!("IO{i}_{pin}"))]);
            vrf.claim_pip(
                bel.wire(&format!("IO{i}_{pin}")),
                bel.wire(&format!("PHASER_C_{pin}_BUF")),
            );
        }
    }
    vrf.claim_net(&[bel.wire("IO32_OCLK90")]);
    vrf.claim_pip(bel.wire("IO32_OCLK90"), bel.wire("PHASER_C_OCLK90_BUF"));

    vrf.claim_net(&[bel.wire("PHASER_C_ICLK_BUF")]);
    vrf.claim_net(&[bel.wire("PHASER_C_ICLKDIV_BUF")]);
    vrf.claim_net(&[bel.wire("PHASER_C_OCLK_BUF")]);
    vrf.claim_net(&[bel.wire("PHASER_C_OCLKDIV_BUF")]);
    vrf.claim_net(&[bel.wire("PHASER_C_OCLK90_BUF")]);
    let obel_pi_c = vrf.find_bel_sibling(bel, bslots::PHASER_IN[2]);
    vrf.claim_pip(bel.wire("PHASER_C_ICLK_BUF"), obel_pi_c.wire("IO_ICLK"));
    vrf.claim_pip(
        bel.wire("PHASER_C_ICLKDIV_BUF"),
        obel_pi_c.wire("IO_ICLKDIV"),
    );
    let obel_po_c = vrf.find_bel_sibling(bel, bslots::PHASER_OUT[2]);
    vrf.claim_pip(bel.wire("PHASER_C_OCLK_BUF"), obel_po_c.wire("IO_OCLK"));
    vrf.claim_pip(
        bel.wire("PHASER_C_OCLKDIV_BUF"),
        obel_po_c.wire("IO_OCLKDIV"),
    );
    vrf.claim_pip(bel.wire("PHASER_C_OCLK90_BUF"), obel_po_c.wire("IO_OCLK90"));
}

fn verify_cmt_d(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel_hclk = vrf.find_bel_sibling(bel, bslots::HCLK_CMT);
    let obel_pc = vrf.find_bel_sibling(bel, bslots::PHY_CONTROL);
    if vrf.find_bel_walk(bel, 0, 50, bslots::CMT_A).is_some() {
        vrf.verify_net(&[bel.wire("SYNC_BB"), obel_pc.wire("SYNC_BB")]);
        vrf.claim_pip(bel.wire("SYNC_BB_N"), bel.wire("SYNC_BB"));
        vrf.claim_pip(bel.wire("SYNC_BB"), bel.wire("SYNC_BB_N"));
    } else if vrf.rd.source == Source::Vivado {
        vrf.verify_net(&[bel.wire("SYNC_BB"), obel_pc.wire("SYNC_BB")]);
        vrf.claim_pip(bel.wire("SYNC_BB_N"), bel.wire("SYNC_BB"));
        vrf.claim_pip(bel.wire("SYNC_BB"), bel.wire("SYNC_BB_N"));
        vrf.claim_net(&[bel.wire("SYNC_BB_N")]);
    }
    let obel_n = vrf.find_bel_walk(bel, 0, 50, bslots::CMT_A).or_else(|| {
        if bel.die.to_idx() != vrf.grid.die.len() - 1 {
            let odie = bel.die + 1;
            let srow = vrf.grid.rows(odie).first().unwrap() + 25;
            vrf.find_bel(CellCoord::new(odie, bel.col, srow).bel(bslots::CMT_A))
        } else {
            None
        }
    });
    for i in 0..4 {
        vrf.verify_net(&[
            bel.wire(&format!("FREQ_BB{i}")),
            obel_hclk.wire(&format!("FREQ_BB{i}")),
        ]);
        if obel_n.is_some() || vrf.rd.source == Source::Vivado {
            vrf.claim_pip(
                bel.wire(&format!("FREQ_BB{i}")),
                bel.wire(&format!("FREQ_BB{i}_N")),
            );
            vrf.claim_pip(
                bel.wire(&format!("FREQ_BB{i}_N")),
                bel.wire(&format!("FREQ_BB{i}")),
            );
            if obel_n.is_none() {
                vrf.claim_net(&[bel.wire(&format!("FREQ_BB{i}_N"))]);
            }
        }
    }

    for i in 37..50 {
        for pin in ["ICLK", "ICLKDIV", "OCLK", "OCLKDIV"] {
            vrf.claim_net(&[bel.wire(&format!("IO{i}_{pin}"))]);
            vrf.claim_pip(
                bel.wire(&format!("IO{i}_{pin}")),
                bel.wire(&format!("PHASER_D_{pin}_BUF")),
            );
        }
    }
    vrf.claim_net(&[bel.wire("IO44_OCLK90")]);
    vrf.claim_pip(bel.wire("IO44_OCLK90"), bel.wire("PHASER_D_OCLK90_BUF"));

    for pin in ["ICLK", "ICLKDIV", "OCLK", "OCLKDIV", "OCLK90"] {
        vrf.claim_net(&[bel.wire(&format!("PHASER_D_{pin}_BUF"))]);
        vrf.claim_pip(
            bel.wire(&format!("PHASER_D_{pin}_BUF")),
            bel.wire(&format!("PHASER_D_{pin}")),
        );
    }

    let obel_pi_d = vrf.find_bel_sibling(bel, bslots::PHASER_IN[3]);
    vrf.verify_net(&[bel.wire("PHASER_D_ICLK"), obel_pi_d.wire("IO_ICLK")]);
    vrf.verify_net(&[bel.wire("PHASER_D_ICLKDIV"), obel_pi_d.wire("IO_ICLKDIV")]);
    let obel_po_d = vrf.find_bel_sibling(bel, bslots::PHASER_OUT[3]);
    vrf.verify_net(&[bel.wire("PHASER_D_OCLK"), obel_po_d.wire("IO_OCLK")]);
    vrf.verify_net(&[bel.wire("PHASER_D_OCLKDIV"), obel_po_d.wire("IO_OCLKDIV")]);
    vrf.verify_net(&[bel.wire("PHASER_D_OCLK90"), obel_po_d.wire("IO_OCLK90")]);
}

fn verify_in_fifo(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "IN_FIFO", &[], &[]);
    vrf.claim_pip(bel.wire("WRCLK"), bel.wire("PHASER_WRCLK"));
    vrf.claim_pip(bel.wire("WREN"), bel.wire("PHASER_WREN"));

    let pidx = (bel.row.to_idx() % 50 - 1) / 12;
    let srow = endev.edev.chips[bel.die].row_hclk(bel.row);
    let obel = vrf.get_legacy_bel(bel.cell.with_row(srow).bel(bslots::PHASER_IN[pidx]));
    vrf.verify_net(&[bel.wire("PHASER_WRCLK"), obel.wire("FIFO_WRCLK")]);
    vrf.verify_net(&[bel.wire("PHASER_WREN"), obel.wire("FIFO_WREN")]);
}

fn verify_out_fifo(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "OUT_FIFO", &[], &[]);
    vrf.claim_pip(bel.wire("RDCLK"), bel.wire("PHASER_RDCLK"));
    vrf.claim_pip(bel.wire("RDEN"), bel.wire("PHASER_RDEN"));

    let pidx = (bel.row.to_idx() % 50 - 1) / 12;
    let srow = endev.edev.chips[bel.die].row_hclk(bel.row);
    let obel = vrf.get_legacy_bel(bel.cell.with_row(srow).bel(bslots::PHASER_OUT[pidx]));
    vrf.verify_net(&[bel.wire("PHASER_RDCLK"), obel.wire("FIFO_RDCLK")]);
    vrf.verify_net(&[bel.wire("PHASER_RDEN"), obel.wire("FIFO_RDEN")]);
}

fn verify_ipad(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    if !bel.tcls.starts_with("GTP")
        || !endev.edev.disabled.contains(&DisabledPart::Gtp)
        || vrf.rd.source == Source::ISE
    {
        vrf.verify_legacy_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    }
    vrf.claim_net(&[bel.wire("O")]);
}

fn verify_opad(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    if !bel.tcls.starts_with("GTP")
        || !endev.edev.disabled.contains(&DisabledPart::Gtp)
        || vrf.rd.source == Source::ISE
    {
        vrf.verify_legacy_bel(bel, "OPAD", &[("I", SitePinDir::In)], &[]);
    }
    vrf.claim_net(&[bel.wire("I")]);
}

fn verify_iopad(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "IOPAD", &[("IO", SitePinDir::Inout)], &[]);
    vrf.claim_net(&[bel.wire("IO")]);
}

fn verify_xadc(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let mut pins = vec![];
    for i in 0..16 {
        pins.push(format!("VAUXP{i}"));
        pins.push(format!("VAUXN{i}"));
    }
    pins.push("VP".to_string());
    pins.push("VN".to_string());
    let mut dummies = vec![];
    for i in 0..16 {
        if endev.edev.get_sysmon_vaux(bel.cell, i).is_none() {
            dummies.push(&*pins[2 * i]);
            dummies.push(&*pins[2 * i + 1]);
        }
    }
    let mut pin_refs = vec![];
    for pin in &pins {
        pin_refs.push((&pin[..], SitePinDir::In));
    }
    if bel.die == endev.edev.interposer.unwrap().primary || vrf.rd.source != Source::Vivado {
        vrf.verify_bel_dummies(bel, "XADC", &pin_refs, &[], &dummies);
    }

    vrf.claim_net(&[bel.wire("VP")]);
    let obel = vrf.find_bel_sibling(bel, bslots::IPAD_VP);
    vrf.claim_pip(bel.wire("VP"), obel.wire("O"));
    vrf.claim_net(&[bel.wire("VN")]);
    let obel = vrf.find_bel_sibling(bel, bslots::IPAD_VN);
    vrf.claim_pip(bel.wire("VN"), obel.wire("O"));

    for i in 0..16 {
        let Some((iop, _)) = endev.edev.get_sysmon_vaux(bel.cell, i) else {
            continue;
        };
        let vauxp = format!("VAUXP{i}");
        let vauxn = format!("VAUXN{i}");
        vrf.claim_net(&[bel.wire(&vauxp)]);
        vrf.claim_net(&[bel.wire(&vauxn)]);
        let (ow0p, iw0p) = bel.pip(&vauxp, 0);
        let (ow1p, iw1p) = bel.pip(&vauxp, 1);
        let (ow0n, iw0n) = bel.pip(&vauxn, 0);
        let (ow1n, iw1n) = bel.pip(&vauxn, 1);
        vrf.claim_pip(ow0p, iw0p);
        vrf.claim_pip(ow1p, iw1p);
        vrf.claim_pip(ow0n, iw0n);
        vrf.claim_pip(ow1n, iw1n);
        vrf.claim_net(&[iw0p, ow1p]);
        vrf.claim_net(&[iw0n, ow1n]);
        let obel = vrf.get_legacy_bel(iop.cell.bel(bslots::IOB[1]));
        vrf.claim_net(&[iw1p, obel.wire("MONITOR")]);
        vrf.claim_pip(obel.wire("MONITOR"), obel.wire("PADOUT"));
        let obel = vrf.get_legacy_bel(iop.cell.bel(bslots::IOB[0]));
        vrf.claim_net(&[iw1n, obel.wire("MONITOR")]);
        vrf.claim_pip(obel.wire("MONITOR"), obel.wire("PADOUT"));
    }
}

fn verify_ps(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let mut iopins = vec![];
    iopins.push((bslots::IOPAD_DDRWEB, "DDRWEB".to_string()));
    iopins.push((bslots::IOPAD_DDRVRN, "DDRVRN".to_string()));
    iopins.push((bslots::IOPAD_DDRVRP, "DDRVRP".to_string()));
    for i in 0..15 {
        iopins.push((bslots::IOPAD_DDRA[i], format!("DDRA{i}")));
    }
    for i in 0..3 {
        iopins.push((bslots::IOPAD_DDRBA[i], format!("DDRBA{i}")));
    }
    iopins.push((bslots::IOPAD_DDRCASB, "DDRCASB".to_string()));
    iopins.push((bslots::IOPAD_DDRCKE, "DDRCKE".to_string()));
    iopins.push((bslots::IOPAD_DDRCKN, "DDRCKN".to_string()));
    iopins.push((bslots::IOPAD_DDRCKP, "DDRCKP".to_string()));
    iopins.push((bslots::IOPAD_PSCLK, "PSCLK".to_string()));
    iopins.push((bslots::IOPAD_DDRCSB, "DDRCSB".to_string()));
    for i in 0..4 {
        iopins.push((bslots::IOPAD_DDRDM[i], format!("DDRDM{i}")));
    }
    for i in 0..32 {
        iopins.push((bslots::IOPAD_DDRDQ[i], format!("DDRDQ{i}")));
    }
    for i in 0..4 {
        iopins.push((bslots::IOPAD_DDRDQSN[i], format!("DDRDQSN{i}")));
    }
    for i in 0..4 {
        iopins.push((bslots::IOPAD_DDRDQSP[i], format!("DDRDQSP{i}")));
    }
    iopins.push((bslots::IOPAD_DDRDRSTB, "DDRDRSTB".to_string()));
    for i in 0..54 {
        iopins.push((bslots::IOPAD_MIO[i], format!("MIO{i}")));
    }
    iopins.push((bslots::IOPAD_DDRODT, "DDRODT".to_string()));
    iopins.push((bslots::IOPAD_PSPORB, "PSPORB".to_string()));
    iopins.push((bslots::IOPAD_DDRRASB, "DDRRASB".to_string()));
    iopins.push((bslots::IOPAD_PSSRSTB, "PSSRSTB".to_string()));
    let mut pin_refs: Vec<_> = iopins
        .iter()
        .map(|(_, x)| (&x[..], SitePinDir::Inout))
        .collect();
    pin_refs.extend([
        ("FCLKCLK0", SitePinDir::Out),
        ("FCLKCLK1", SitePinDir::Out),
        ("FCLKCLK2", SitePinDir::Out),
        ("FCLKCLK3", SitePinDir::Out),
        ("TESTPLLNEWCLK0", SitePinDir::Out),
        ("TESTPLLNEWCLK1", SitePinDir::Out),
        ("TESTPLLNEWCLK2", SitePinDir::Out),
        ("TESTPLLCLKOUT0", SitePinDir::Out),
        ("TESTPLLCLKOUT1", SitePinDir::Out),
        ("TESTPLLCLKOUT2", SitePinDir::Out),
    ]);
    vrf.verify_legacy_bel(
        bel,
        "PS7",
        &pin_refs,
        &[
            "FCLKCLK0_INT",
            "FCLKCLK1_INT",
            "FCLKCLK2_INT",
            "FCLKCLK3_INT",
        ],
    );

    for pin in [
        "TESTPLLNEWCLK0",
        "TESTPLLNEWCLK1",
        "TESTPLLNEWCLK2",
        "TESTPLLCLKOUT0",
        "TESTPLLCLKOUT1",
        "TESTPLLCLKOUT2",
    ] {
        vrf.claim_net(&[bel.wire(pin)]);
        vrf.claim_net(&[bel.wire_far(pin)]);
        vrf.claim_pip(bel.wire_far(pin), bel.wire(pin));
    }

    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("FCLKCLK{i}"))]);
        vrf.claim_net(&[bel.wire(&format!("FCLKCLK{i}_HOUT"))]);
        vrf.claim_pip(
            bel.wire(&format!("FCLKCLK{i}_HOUT")),
            bel.wire(&format!("FCLKCLK{i}")),
        );
        vrf.claim_pip(
            bel.wire(&format!("FCLKCLK{i}_INT")),
            bel.wire(&format!("FCLKCLK{i}")),
        );
    }

    for (bslot, pin) in &iopins {
        vrf.claim_net(&[bel.wire(pin)]);
        let obel = vrf.find_bel_sibling(bel, *bslot);
        vrf.claim_pip(bel.wire(pin), obel.wire("IO"));
        vrf.claim_pip(obel.wire("IO"), bel.wire(pin));
    }
}

fn verify_hclk_ps_lo(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel = vrf.find_bel_sibling(bel, bslots::PS);
    for i in 0..4 {
        vrf.claim_net(&[bel.wire(&format!("HOUT{i}"))]);
        vrf.verify_net(&[
            bel.wire(&format!("FCLKCLK{i}")),
            obel.wire(&format!("FCLKCLK{i}_HOUT")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("HOUT{i}")),
            bel.wire(&format!("FCLKCLK{i}")),
        );
    }
}

fn verify_hclk_ps_hi(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let obel = vrf.find_bel_sibling(bel, bslots::PS);
    for i in 0..3 {
        vrf.claim_net(&[bel.wire(&format!("HOUT{i}"))]);
        vrf.verify_net(&[
            bel.wire(&format!("TESTPLLNEWCLK{i}")),
            obel.wire_far(&format!("TESTPLLNEWCLK{i}")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("HOUT{i}")),
            bel.wire(&format!("TESTPLLNEWCLK{i}")),
        );
        vrf.claim_net(&[bel.wire(&format!("HOUT{ii}", ii = i + 3))]);
        vrf.verify_net(&[
            bel.wire(&format!("TESTPLLCLKOUT{i}")),
            obel.wire_far(&format!("TESTPLLCLKOUT{i}")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("HOUT{ii}", ii = i + 3)),
            bel.wire(&format!("TESTPLLCLKOUT{i}")),
        );
    }
}

pub fn verify_ibufds(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::BUFDS.index_of(bcrd.slot).unwrap();
    let pins = [
        ("I", SitePinDir::In),
        ("IB", SitePinDir::In),
        ("O", SitePinDir::Out),
        ("ODIV2", SitePinDir::Out),
    ];
    if !endev.edev.disabled.contains(&DisabledPart::Gtp) || vrf.rd.source == Source::ISE {
        vrf.verify_legacy_bel(bel, "IBUFDS_GTE2", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    for (pin, oslot) in [
        ("I", bslots::IPAD_CLKP[idx]),
        ("IB", bslots::IPAD_CLKN[idx]),
    ] {
        let obel = vrf.find_bel_sibling(bel, oslot);
        vrf.claim_pip(bel.wire(pin), obel.wire("O"));
    }
    vrf.claim_net(&[bel.wire("MGTCLKOUT")]);
    vrf.claim_pip(bel.wire("MGTCLKOUT"), bel.wire("O"));
    vrf.claim_pip(bel.wire("MGTCLKOUT"), bel.wire("ODIV2"));
}

fn verify_gtp_channel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("GTPRXP", SitePinDir::In),
        ("GTPRXN", SitePinDir::In),
        ("GTPTXP", SitePinDir::Out),
        ("GTPTXN", SitePinDir::Out),
        ("PLL0CLK", SitePinDir::In),
        ("PLL1CLK", SitePinDir::In),
        ("PLL0REFCLK", SitePinDir::In),
        ("PLL1REFCLK", SitePinDir::In),
        ("RXOUTCLK", SitePinDir::Out),
        ("TXOUTCLK", SitePinDir::Out),
    ];
    if !endev.edev.disabled.contains(&DisabledPart::Gtp) || vrf.rd.source == Source::ISE {
        vrf.verify_legacy_bel(bel, "GTPE2_CHANNEL", &pins, &[]);
    }
    for (pin, _) in &pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    for (pin, slot) in [
        ("GTPRXP", bslots::IPAD_RXP[0]),
        ("GTPRXN", bslots::IPAD_RXN[0]),
    ] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.wire(pin), obel.wire("O"));
    }
    for (pin, slot) in [
        ("GTPTXP", bslots::OPAD_TXP[0]),
        ("GTPTXN", bslots::OPAD_TXN[0]),
    ] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(obel.wire("I"), bel.wire(pin));
    }

    let obel = vrf.get_legacy_bel(
        bel.cell
            .with_row(endev.edev.chips[bel.die].row_hclk(bel.row))
            .bel(bslots::GTP_COMMON),
    );
    for (pin, opin) in [
        ("PLL0CLK", "PLL0OUTCLK"),
        ("PLL1CLK", "PLL1OUTCLK"),
        ("PLL0REFCLK", "PLL0OUTREFCLK"),
        ("PLL1REFCLK", "PLL1OUTREFCLK"),
    ] {
        vrf.verify_net(&[bel.wire_far(pin), obel.wire_far(opin)]);
        vrf.claim_pip(bel.wire(pin), bel.wire_far(pin));
    }
    for pin in ["RXOUTCLK", "TXOUTCLK"] {
        vrf.claim_net(&[bel.wire_far(pin)]);
        vrf.claim_pip(bel.wire_far(pin), bel.wire(pin));
    }
}

fn verify_gtxh_channel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let is_gth = bcrd.slot == bslots::GTH_CHANNEL;
    let rxp = if is_gth { "GTHRXP" } else { "GTXRXP" };
    let rxn = if is_gth { "GTHRXN" } else { "GTXRXN" };
    let txp = if is_gth { "GTHTXP" } else { "GTXTXP" };
    let txn = if is_gth { "GTHTXN" } else { "GTXTXN" };
    let pins = [
        ("GTREFCLK0", SitePinDir::In),
        ("GTREFCLK1", SitePinDir::In),
        ("GTNORTHREFCLK0", SitePinDir::In),
        ("GTNORTHREFCLK1", SitePinDir::In),
        ("GTSOUTHREFCLK0", SitePinDir::In),
        ("GTSOUTHREFCLK1", SitePinDir::In),
        ("QPLLCLK", SitePinDir::In),
        ("QPLLREFCLK", SitePinDir::In),
        ("RXOUTCLK", SitePinDir::Out),
        ("TXOUTCLK", SitePinDir::Out),
        (rxp, SitePinDir::In),
        (rxn, SitePinDir::In),
        (txp, SitePinDir::Out),
        (txn, SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(
        bel,
        if is_gth {
            "GTHE2_CHANNEL"
        } else {
            "GTXE2_CHANNEL"
        },
        &pins,
        &[],
    );
    for (pin, _) in &pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    for (pin, slot) in [(rxp, bslots::IPAD_RXP[0]), (rxn, bslots::IPAD_RXN[0])] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.wire(pin), obel.wire("O"));
    }
    for (pin, slot) in [(txp, bslots::OPAD_TXP[0]), (txn, bslots::OPAD_TXN[0])] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(obel.wire("I"), bel.wire(pin));
    }

    let obel = vrf.get_legacy_bel(
        bel.cell
            .with_row(endev.edev.chips[bel.die].row_hclk(bel.row))
            .bel(if is_gth {
                bslots::GTH_COMMON
            } else {
                bslots::GTX_COMMON
            }),
    );
    let has_s = vrf
        .find_bel_delta(&obel, 0, -25, bslots::BRKH_GTX)
        .is_some();
    let has_n = vrf.find_bel_delta(&obel, 0, 25, bslots::BRKH_GTX).is_some();
    for (pin, opin, present) in [
        ("QPLLCLK", "QPLLOUTCLK", true),
        ("QPLLREFCLK", "QPLLOUTREFCLK", true),
        ("GTREFCLK0", "GTREFCLK0", true),
        ("GTREFCLK1", "GTREFCLK1", true),
        ("GTNORTHREFCLK0", "GTNORTHREFCLK0", has_s),
        ("GTNORTHREFCLK1", "GTNORTHREFCLK1", has_s),
        ("GTSOUTHREFCLK0", "GTSOUTHREFCLK0", has_n),
        ("GTSOUTHREFCLK1", "GTSOUTHREFCLK1", has_n),
    ] {
        if present {
            vrf.verify_net(&[bel.wire_far(pin), obel.wire_far(opin)]);
        } else {
            vrf.claim_dummy_in(bel.wire_far(pin));
        }
        vrf.claim_pip(bel.wire(pin), bel.wire_far(pin));
    }
    for pin in ["RXOUTCLK", "TXOUTCLK"] {
        vrf.claim_net(&[bel.wire_far(pin)]);
        vrf.claim_pip(bel.wire_far(pin), bel.wire(pin));
    }
}

fn verify_gtp_common(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("GTREFCLK0", SitePinDir::In),
        ("GTREFCLK1", SitePinDir::In),
        ("GTEASTREFCLK0", SitePinDir::In),
        ("GTEASTREFCLK1", SitePinDir::In),
        ("GTWESTREFCLK0", SitePinDir::In),
        ("GTWESTREFCLK1", SitePinDir::In),
        ("PLL0OUTCLK", SitePinDir::Out),
        ("PLL1OUTCLK", SitePinDir::Out),
        ("PLL0OUTREFCLK", SitePinDir::Out),
        ("PLL1OUTREFCLK", SitePinDir::Out),
    ];
    let mut dummies = vec![];
    let mut is_mid_l = false;
    let mut is_mid_r = false;
    if let Some((cl, cr)) = endev.edev.col_mgt {
        if cl == bel.col {
            dummies.extend(["GTEASTREFCLK0", "GTEASTREFCLK1"]);
            is_mid_l = true;
        } else {
            assert_eq!(bel.col, cr);
            dummies.extend(["GTWESTREFCLK0", "GTWESTREFCLK1"]);
            is_mid_r = true;
        }
    } else {
        dummies.extend([
            "GTEASTREFCLK0",
            "GTEASTREFCLK1",
            "GTWESTREFCLK0",
            "GTWESTREFCLK1",
        ]);
    }
    if !endev.edev.disabled.contains(&DisabledPart::Gtp) || vrf.rd.source == Source::ISE {
        vrf.verify_bel_dummies(bel, "GTPE2_COMMON", &pins, &[], &dummies);
    }
    for (pin, _) in &pins {
        if !dummies.contains(pin) {
            vrf.claim_net(&[bel.wire(pin)]);
        }
    }

    for i in 0..2 {
        vrf.claim_net(&[bel.wire(&format!("REFCLK{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("GTREFCLK{i}")),
            bel.wire(&format!("REFCLK{i}")),
        );
        let obel = vrf.find_bel_sibling(bel, bslots::BUFDS[i]);
        vrf.claim_pip(bel.wire(&format!("REFCLK{i}")), obel.wire("O"));
    }

    for pin in ["PLL0OUTCLK", "PLL1OUTCLK", "PLL0OUTREFCLK", "PLL1OUTREFCLK"] {
        vrf.claim_net(&[bel.wire_far(pin)]);
        vrf.claim_pip(bel.wire_far(pin), bel.wire(pin));
    }

    if is_mid_l {
        vrf.claim_net(&[bel.wire("EASTCLK0")]);
        vrf.claim_net(&[bel.wire("EASTCLK1")]);
        vrf.claim_pip(bel.wire("EASTCLK0"), bel.wire("REFCLK0"));
        vrf.claim_pip(bel.wire("EASTCLK0"), bel.wire("REFCLK1"));
        vrf.claim_pip(bel.wire("EASTCLK1"), bel.wire("REFCLK0"));
        vrf.claim_pip(bel.wire("EASTCLK1"), bel.wire("REFCLK1"));

        vrf.claim_pip(bel.wire("GTWESTREFCLK0"), bel.wire("WESTCLK0"));
        vrf.claim_pip(bel.wire("GTWESTREFCLK1"), bel.wire("WESTCLK1"));
        let obel = vrf.get_legacy_bel(
            bel.cell
                .with_col(endev.edev.col_mgt.unwrap().1)
                .bel(bslots::GTP_COMMON),
        );
        vrf.verify_net(&[bel.wire("WESTCLK0"), obel.wire("WESTCLK0")]);
        vrf.verify_net(&[bel.wire("WESTCLK1"), obel.wire("WESTCLK1")]);
    }
    if is_mid_r {
        vrf.claim_net(&[bel.wire("WESTCLK0")]);
        vrf.claim_net(&[bel.wire("WESTCLK1")]);
        vrf.claim_pip(bel.wire("WESTCLK0"), bel.wire("REFCLK0"));
        vrf.claim_pip(bel.wire("WESTCLK0"), bel.wire("REFCLK1"));
        vrf.claim_pip(bel.wire("WESTCLK1"), bel.wire("REFCLK0"));
        vrf.claim_pip(bel.wire("WESTCLK1"), bel.wire("REFCLK1"));

        vrf.claim_pip(bel.wire("GTEASTREFCLK0"), bel.wire("EASTCLK0"));
        vrf.claim_pip(bel.wire("GTEASTREFCLK1"), bel.wire("EASTCLK1"));
        let obel = vrf.get_legacy_bel(
            bel.cell
                .with_col(endev.edev.col_mgt.unwrap().0)
                .bel(bslots::GTP_COMMON),
        );
        vrf.verify_net(&[bel.wire("EASTCLK0"), obel.wire("EASTCLK0")]);
        vrf.verify_net(&[bel.wire("EASTCLK1"), obel.wire("EASTCLK1")]);
    }

    for (i, dy) in [(0, -25), (1, -14), (2, 3), (3, 14)] {
        let obel = vrf.find_bel_delta(bel, 0, dy, bslots::GTP_CHANNEL).unwrap();
        vrf.verify_net(&[bel.wire(&format!("TXOUTCLK{i}")), obel.wire_far("TXOUTCLK")]);
        vrf.verify_net(&[bel.wire(&format!("RXOUTCLK{i}")), obel.wire_far("RXOUTCLK")]);
    }
    if is_mid_l || is_mid_r {
        for pin in [
            "RXOUTCLK0",
            "RXOUTCLK1",
            "RXOUTCLK2",
            "RXOUTCLK3",
            "TXOUTCLK0",
            "TXOUTCLK1",
            "TXOUTCLK2",
            "TXOUTCLK3",
        ] {
            vrf.claim_net(&[bel.wire(&format!("{pin}_BUF"))]);
            vrf.claim_pip(bel.wire(&format!("{pin}_BUF")), bel.wire(pin));
        }
        for (bpin, slot) in [
            ("MGTCLKOUT0_BUF", bslots::BUFDS[0]),
            ("MGTCLKOUT1_BUF", bslots::BUFDS[1]),
        ] {
            let obel = vrf.find_bel_sibling(bel, slot);
            vrf.claim_net(&[bel.wire(bpin)]);
            vrf.claim_pip(bel.wire(bpin), obel.wire("MGTCLKOUT"));
        }
        let scol_io = if is_mid_l {
            endev.edev.col_lio
        } else {
            endev.edev.col_rio
        }
        .unwrap();
        let scol = ColId::from_idx(scol_io.to_idx() ^ 1);
        let obel = vrf.get_legacy_bel(bel.cell.with_col(scol).bel(bslots::HCLK_CMT));
        for i in 0..14 {
            let opin = format!("HOUT{i}");
            vrf.claim_net(&[bel.wire(&opin)]);
            for spin in [
                "RXOUTCLK0_BUF",
                "RXOUTCLK1_BUF",
                "RXOUTCLK2_BUF",
                "RXOUTCLK3_BUF",
                "TXOUTCLK0_BUF",
                "TXOUTCLK1_BUF",
                "TXOUTCLK2_BUF",
                "TXOUTCLK3_BUF",
                "MGTCLKOUT0_BUF",
                "MGTCLKOUT1_BUF",
            ] {
                vrf.claim_pip(bel.wire(&opin), bel.wire(spin));
            }
            vrf.claim_pip(bel.wire(&opin), bel.wire(&format!("HIN{i}")));
            vrf.claim_pip(bel.wire(&opin), bel.wire(&format!("HIN{ii}", ii = i ^ 1)));
            vrf.verify_net(&[bel.wire(&format!("HIN{i}")), obel.wire(&opin)]);
        }
    } else {
        for (i, pin) in [
            (4, "RXOUTCLK0"),
            (5, "RXOUTCLK1"),
            (6, "TXOUTCLK0"),
            (7, "TXOUTCLK1"),
            (10, "RXOUTCLK2"),
            (11, "RXOUTCLK3"),
            (12, "TXOUTCLK2"),
            (13, "TXOUTCLK3"),
        ] {
            vrf.claim_net(&[bel.wire(&format!("HOUT{i}"))]);
            vrf.claim_pip(bel.wire(&format!("HOUT{i}")), bel.wire(pin));
        }
        for (i, slot) in [(8, bslots::BUFDS[0]), (9, bslots::BUFDS[1])] {
            let obel = vrf.find_bel_sibling(bel, slot);
            vrf.claim_net(&[bel.wire(&format!("HOUT{i}"))]);
            vrf.claim_pip(bel.wire(&format!("HOUT{i}")), obel.wire("MGTCLKOUT"));
        }
    }
}

fn verify_gtxh_common(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("GTREFCLK0", SitePinDir::In),
        ("GTREFCLK1", SitePinDir::In),
        ("GTNORTHREFCLK0", SitePinDir::In),
        ("GTNORTHREFCLK1", SitePinDir::In),
        ("GTSOUTHREFCLK0", SitePinDir::In),
        ("GTSOUTHREFCLK1", SitePinDir::In),
        ("QPLLOUTCLK", SitePinDir::Out),
        ("QPLLOUTREFCLK", SitePinDir::Out),
    ];
    let is_gth = bcrd.slot == bslots::GTH_COMMON;
    vrf.verify_legacy_bel(
        bel,
        if is_gth {
            "GTHE2_COMMON"
        } else {
            "GTXE2_COMMON"
        },
        &pins,
        &[],
    );
    for (pin, _) in &pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    for i in 0..2 {
        vrf.claim_net(&[bel.wire_far(&format!("GTREFCLK{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("GTREFCLK{i}")),
            bel.wire_far(&format!("GTREFCLK{i}")),
        );
        let obel = vrf.find_bel_sibling(bel, bslots::BUFDS[i]);
        vrf.claim_pip(bel.wire_far(&format!("GTREFCLK{i}")), obel.wire("O"));
        vrf.claim_pip(
            bel.wire(&format!("GTSOUTHREFCLK{i}")),
            bel.wire_far(&format!("GTSOUTHREFCLK{i}")),
        );
        vrf.claim_pip(
            bel.wire(&format!("GTNORTHREFCLK{i}")),
            bel.wire_far(&format!("GTNORTHREFCLK{i}")),
        );
    }
    if let Some(obel_n) = vrf.find_bel_delta(bel, 0, 25, bslots::BRKH_GTX) {
        for i in 0..2 {
            vrf.verify_net(&[
                bel.wire_far(&format!("GTSOUTHREFCLK{i}")),
                obel_n.wire(&format!("SOUTHREFCLK{i}_D")),
            ]);
        }
    } else {
        for i in 0..2 {
            vrf.claim_dummy_in(bel.wire_far(&format!("GTSOUTHREFCLK{i}")));
        }
    }
    if let Some(obel_s) = vrf.find_bel_delta(bel, 0, -25, bslots::BRKH_GTX) {
        for i in 0..2 {
            vrf.verify_net(&[
                bel.wire_far(&format!("GTNORTHREFCLK{i}")),
                obel_s.wire(&format!("NORTHREFCLK{i}_U")),
            ]);
        }
    } else {
        for i in 0..2 {
            vrf.claim_dummy_in(bel.wire_far(&format!("GTNORTHREFCLK{i}")));
        }
    }

    for pin in ["QPLLOUTCLK", "QPLLOUTREFCLK"] {
        vrf.claim_net(&[bel.wire_far(pin)]);
        vrf.claim_pip(bel.wire_far(pin), bel.wire(pin));
    }

    for (i, dy) in [(0, -25), (1, -14), (2, 3), (3, 14)] {
        let obel = vrf
            .find_bel_delta(
                bel,
                0,
                dy,
                if is_gth {
                    bslots::GTH_CHANNEL
                } else {
                    bslots::GTX_CHANNEL
                },
            )
            .unwrap();
        vrf.verify_net(&[bel.wire(&format!("TXOUTCLK{i}")), obel.wire_far("TXOUTCLK")]);
        vrf.verify_net(&[bel.wire(&format!("RXOUTCLK{i}")), obel.wire_far("RXOUTCLK")]);
    }
    for (i, pin) in [
        (4, "RXOUTCLK0"),
        (5, "RXOUTCLK1"),
        (6, "TXOUTCLK0"),
        (7, "TXOUTCLK1"),
        (10, "RXOUTCLK2"),
        (11, "RXOUTCLK3"),
        (12, "TXOUTCLK2"),
        (13, "TXOUTCLK3"),
    ] {
        vrf.claim_net(&[bel.wire(&format!("HOUT{i}"))]);
        vrf.claim_pip(bel.wire(&format!("HOUT{i}")), bel.wire(pin));
    }
    for (i, slot) in [(8, bslots::BUFDS[0]), (9, bslots::BUFDS[1])] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_net(&[bel.wire(&format!("HOUT{i}"))]);
        vrf.claim_pip(bel.wire(&format!("HOUT{i}")), obel.wire("MGTCLKOUT"));
    }
}

fn verify_brkh_gtx(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.claim_net(&[bel.wire("NORTHREFCLK0_U")]);
    vrf.claim_net(&[bel.wire("NORTHREFCLK1_U")]);
    vrf.claim_net(&[bel.wire("SOUTHREFCLK0_D")]);
    vrf.claim_net(&[bel.wire("SOUTHREFCLK1_D")]);
    vrf.claim_pip(bel.wire("NORTHREFCLK0_U"), bel.wire("NORTHREFCLK0_D"));
    vrf.claim_pip(bel.wire("NORTHREFCLK0_U"), bel.wire("REFCLK0_D"));
    vrf.claim_pip(bel.wire("NORTHREFCLK0_U"), bel.wire("REFCLK1_D"));
    vrf.claim_pip(bel.wire("NORTHREFCLK1_U"), bel.wire("NORTHREFCLK1_D"));
    vrf.claim_pip(bel.wire("NORTHREFCLK1_U"), bel.wire("REFCLK0_D"));
    vrf.claim_pip(bel.wire("NORTHREFCLK1_U"), bel.wire("REFCLK1_D"));
    vrf.claim_pip(bel.wire("SOUTHREFCLK0_D"), bel.wire("SOUTHREFCLK0_U"));
    vrf.claim_pip(bel.wire("SOUTHREFCLK0_D"), bel.wire("REFCLK0_U"));
    vrf.claim_pip(bel.wire("SOUTHREFCLK0_D"), bel.wire("REFCLK1_U"));
    vrf.claim_pip(bel.wire("SOUTHREFCLK1_D"), bel.wire("SOUTHREFCLK1_U"));
    vrf.claim_pip(bel.wire("SOUTHREFCLK1_D"), bel.wire("REFCLK0_U"));
    vrf.claim_pip(bel.wire("SOUTHREFCLK1_D"), bel.wire("REFCLK1_U"));
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -50, bslots::BRKH_GTX) {
        vrf.verify_net(&[bel.wire("NORTHREFCLK0_D"), obel.wire("NORTHREFCLK0_U")]);
        vrf.verify_net(&[bel.wire("NORTHREFCLK1_D"), obel.wire("NORTHREFCLK1_U")]);
    } else {
        vrf.claim_dummy_in(bel.wire("NORTHREFCLK0_D"));
        vrf.claim_dummy_in(bel.wire("NORTHREFCLK1_D"));
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 50, bslots::BRKH_GTX) {
        vrf.verify_net(&[bel.wire("SOUTHREFCLK0_U"), obel.wire("SOUTHREFCLK0_D")]);
        vrf.verify_net(&[bel.wire("SOUTHREFCLK1_U"), obel.wire("SOUTHREFCLK1_D")]);
    } else {
        vrf.claim_dummy_in(bel.wire("SOUTHREFCLK0_U"));
        vrf.claim_dummy_in(bel.wire("SOUTHREFCLK1_U"));
    }
    if let Some(obel) = vrf
        .find_bel_delta(bel, 0, -25, bslots::GTX_COMMON)
        .or_else(|| vrf.find_bel_delta(bel, 0, -25, bslots::GTH_COMMON))
    {
        vrf.verify_net(&[bel.wire("REFCLK0_D"), obel.wire_far("GTREFCLK0")]);
        vrf.verify_net(&[bel.wire("REFCLK1_D"), obel.wire_far("GTREFCLK1")]);
    } else {
        vrf.claim_dummy_in(bel.wire("REFCLK0_D"));
        vrf.claim_dummy_in(bel.wire("REFCLK1_D"));
    }
    if let Some(obel) = vrf
        .find_bel_delta(bel, 0, 25, bslots::GTX_COMMON)
        .or_else(|| vrf.find_bel_delta(bel, 0, 25, bslots::GTH_COMMON))
    {
        vrf.verify_net(&[bel.wire("REFCLK0_U"), obel.wire_far("GTREFCLK0")]);
        vrf.verify_net(&[bel.wire("REFCLK1_U"), obel.wire_far("GTREFCLK1")]);
    } else {
        vrf.claim_dummy_in(bel.wire("REFCLK0_U"));
        vrf.claim_dummy_in(bel.wire("REFCLK1_U"));
    }
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let slot_name = endev.edev.db.bel_slots.key(bcrd.slot);
    match bcrd.slot {
        bslots::INT
        | bslots::INTF_INT
        | bslots::INTF_TESTMUX
        | bslots::SPEC_INT
        | bslots::CLK_INT
        | bslots::HROW_INT
        | bslots::HCLK_IO_INT
        | bslots::MISC_CFG
        | bslots::BANK
        | bslots::GLOBAL => (),
        _ if bslots::SLICE.contains(bcrd.slot) => verify_slice(endev, vrf, bcrd),
        _ if bslots::DSP.contains(bcrd.slot) => verify_dsp(vrf, bcrd),
        bslots::TIEOFF_DSP => verify_tieoff(vrf, bcrd),
        bslots::BRAM_F => verify_bram_f(vrf, bcrd),
        _ if bslots::BRAM_H.contains(bcrd.slot) => verify_bram_h(vrf, bcrd),
        bslots::BRAM_ADDR => verify_bram_addr(vrf, bcrd),
        bslots::PCIE => {
            if !endev.edev.disabled.contains(&DisabledPart::Gtp) || vrf.rd.source == Source::ISE {
                let bel = &mut vrf.get_legacy_bel(bcrd);
                vrf.verify_legacy_bel(bel, "PCIE_2_1", &[], &[])
            }
        }
        bslots::PCIE3 => {
            let bel = &mut vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "PCIE_3_0", &[], &[])
        }
        bslots::PMVBRAM => verify_pmvbram(vrf, bcrd),
        _ if bslots::PMV_CFG.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        bslots::PMV_CLK => vrf.verify_bel(bcrd).commit(),
        bslots::PMV2 | bslots::MTBF2 | bslots::STARTUP | bslots::CAPTURE | bslots::USR_ACCESS => {
            vrf.verify_bel(bcrd).commit()
        }
        bslots::PMV2_SVT => vrf.verify_bel(bcrd).kind("PMV2_SVT").commit(),
        bslots::CFG_IO_ACCESS => vrf.verify_bel(bcrd).kind("CFG_IO_ACCESS").commit(),
        bslots::FRAME_ECC => vrf.verify_bel(bcrd).kind("FRAME_ECC").commit(),
        bslots::PMVIOB_CFG | bslots::PMVIOB_CLK => vrf.verify_bel(bcrd).commit(),
        bslots::DCIRESET | bslots::DNA_PORT | bslots::EFUSE_USR => {
            if bcrd.die == endev.edev.interposer.unwrap().primary || vrf.rd.source != Source::Vivado
            {
                vrf.verify_bel(bcrd).commit()
            }
        }
        _ if bslots::BSCAN.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        _ if bslots::ICAP.contains(bcrd.slot) => vrf
            .verify_bel(bcrd)
            .kind("ICAP")
            .rename_in(bcls::ICAP_V6::CSB, "CSIB")
            .commit(),

        bslots::INT_LCLK_W | bslots::INT_LCLK_E => verify_int_lclk(endev, vrf, bcrd),
        bslots::HCLK_W => verify_hclk_w(endev, vrf, bcrd),
        bslots::HCLK_E => verify_hclk_e(endev, vrf, bcrd),
        _ if slot_name.starts_with("GCLK_TEST_BUF") => verify_gclk_test_buf(vrf, bcrd),
        _ if slot_name.starts_with("BUFHCE") => verify_bufhce(vrf, bcrd),
        bslots::CLK_REBUF => verify_clk_rebuf(vrf, bcrd),
        bslots::CLK_HROW_V7 => verify_clk_hrow(endev, vrf, bcrd),
        _ if slot_name.starts_with("BUFGCTRL") => verify_bufgctrl(vrf, bcrd),

        _ if slot_name.starts_with("BUFIO") => verify_bufio(vrf, bcrd),
        _ if slot_name.starts_with("BUFR") => verify_bufr(vrf, bcrd),
        bslots::IDELAYCTRL => verify_idelayctrl(vrf, bcrd),
        bslots::DCI => verify_dci(vrf, bcrd),
        bslots::HCLK_IO => verify_hclk_ioi(endev, vrf, bcrd),

        _ if bslots::ILOGIC.contains(bcrd.slot) => verify_ilogic(endev, vrf, bcrd),
        _ if bslots::OLOGIC.contains(bcrd.slot) => verify_ologic(endev, vrf, bcrd),
        _ if bslots::IDELAY.contains(bcrd.slot) => verify_idelay(vrf, bcrd),
        _ if bslots::ODELAY.contains(bcrd.slot) => verify_odelay(vrf, bcrd),
        _ if bslots::IOB.contains(bcrd.slot) => verify_iob(vrf, bcrd),
        bslots::IOI => verify_ioi(endev, vrf, bcrd),

        _ if slot_name.starts_with("PHASER_IN") => verify_phaser_in(vrf, bcrd),
        _ if slot_name.starts_with("PHASER_OUT") => verify_phaser_out(vrf, bcrd),
        bslots::PHASER_REF => verify_phaser_ref(vrf, bcrd),
        bslots::PHY_CONTROL => verify_phy_control(vrf, bcrd),
        _ if bslots::MMCM.contains(bcrd.slot) => verify_mmcm(vrf, bcrd),
        bslots::PLL => verify_pll(vrf, bcrd),
        _ if slot_name.starts_with("BUFMRCE") => verify_bufmrce(vrf, bcrd),
        bslots::HCLK_CMT => verify_hclk_cmt(endev, vrf, bcrd),
        bslots::CMT_A => verify_cmt_a(vrf, bcrd),
        bslots::CMT_B => verify_cmt_b(vrf, bcrd),
        bslots::CMT_C => verify_cmt_c(vrf, bcrd),
        bslots::CMT_D => verify_cmt_d(vrf, bcrd),
        bslots::IN_FIFO => verify_in_fifo(endev, vrf, bcrd),
        bslots::OUT_FIFO => verify_out_fifo(endev, vrf, bcrd),

        bslots::SYSMON => verify_xadc(endev, vrf, bcrd),
        _ if slot_name.starts_with("IPAD") => verify_ipad(endev, vrf, bcrd),
        _ if slot_name.starts_with("OPAD") => verify_opad(endev, vrf, bcrd),
        _ if slot_name.starts_with("IOPAD") => verify_iopad(vrf, bcrd),
        bslots::PS => verify_ps(vrf, bcrd),
        bslots::HCLK_PS_S => verify_hclk_ps_lo(vrf, bcrd),
        bslots::HCLK_PS_N => verify_hclk_ps_hi(vrf, bcrd),

        bslots::GTP_CHANNEL => verify_gtp_channel(endev, vrf, bcrd),
        bslots::GTP_COMMON => verify_gtp_common(endev, vrf, bcrd),
        bslots::GTX_CHANNEL | bslots::GTH_CHANNEL => verify_gtxh_channel(endev, vrf, bcrd),
        bslots::GTX_COMMON | bslots::GTH_COMMON => verify_gtxh_common(vrf, bcrd),
        _ if bslots::BUFDS.contains(bcrd.slot) => verify_ibufds(endev, vrf, bcrd),
        bslots::BRKH_GTX => verify_brkh_gtx(vrf, bcrd),

        _ => println!("MEOW {}", bcrd.to_string(endev.edev.db)),
    }
}

fn verify_gtz(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    egt: &ExpandedGtz,
    ngt: &ExpandedNamedGtz,
) {
    fn int_wire_name_gtz(side: DirV, col: GtzIntColId, row: GtzIntRowId) -> String {
        let x = col.to_idx();
        let y = match side {
            DirV::N => 48 - row.to_idx(),
            DirV::S => row.to_idx(),
        };
        format!("GTZ_VBRK_INTF_SLV_{x}_{y}")
    }
    fn int_wire_name_int_l(
        side: DirV,
        icol: GtzIntColId,
        col: GtzIntColId,
        row: GtzIntRowId,
    ) -> String {
        let x = (86 + icol.to_idx() - col.to_idx() - 1) % 86;
        let y = match side {
            DirV::S => 48 - row.to_idx(),
            DirV::N => row.to_idx(),
        };
        format!("GTZ_INT_L_SLV_{x}_{y}")
    }
    fn int_wire_name_int_r(
        side: DirV,
        icol: GtzIntColId,
        col: GtzIntColId,
        row: GtzIntRowId,
    ) -> String {
        let x = (86 + icol.to_idx() - col.to_idx()) % 86;
        let y = match side {
            DirV::S => 48 - row.to_idx(),
            DirV::N => row.to_idx(),
        };
        format!("GTZ_INT_R_SLV_{x}_{y}")
    }
    fn int_wire_name_int_i(
        edev: &ExpandedDevice,
        side: DirV,
        gcol: ColId,
        row: GtzIntRowId,
    ) -> String {
        let lr = match edev.col_side(gcol) {
            DirH::W => 'L',
            DirH::E => 'R',
        };
        let bt = if side == DirV::S { 'T' } else { 'B' };
        let y = row.to_idx();
        format!("GTZ_INT_{lr}{bt}_SLV_{y}")
    }
    let mut pin_wires = HashMap::new();
    let mut out_gclk = HashSet::new();
    let gtz = &endev.edev.gdb.gtz[egt.kind];
    let crd_gtz = vrf.xlat_tile(&ngt.tile).unwrap();
    for (pname, pin) in &gtz.pins {
        let wire = format!("GTZE2_OCTAL_{pname}");
        let iwire = int_wire_name_gtz(gtz.side, pin.col, pin.row);
        if pin.dir == PinDir::Output {
            vrf.claim_pip_tri(crd_gtz, &iwire, &wire);
        } else {
            vrf.claim_pip_tri(crd_gtz, &wire, &iwire);
        }
        pin_wires.insert(pname.clone(), (pin.dir, wire));
    }
    for (pname, pin) in &gtz.clk_pins {
        let wire = format!("GTZE2_OCTAL_{pname}");
        let cwire = format!("GTZ_VBRK_INTF_GCLK{idx}", idx = pin.idx);
        if pin.dir == PinDir::Output {
            out_gclk.insert(pin.idx);
            vrf.claim_pip_tri(crd_gtz, &cwire, &wire);
        } else {
            vrf.claim_pip_tri(crd_gtz, &wire, &cwire);
        }
        pin_wires.insert(pname.clone(), (pin.dir, wire));
    }
    let mut pads = vec![];
    for i in 0..2 {
        pads.push((format!("GTREFCLK{i}P"), PinDir::Input, &ngt.pads_clk[i].0));
        pads.push((format!("GTREFCLK{i}N"), PinDir::Input, &ngt.pads_clk[i].1));
    }
    for i in 0..8 {
        pads.push((format!("GTZRXP{i}"), PinDir::Input, &ngt.pads_rx[i].0));
        pads.push((format!("GTZRXN{i}"), PinDir::Input, &ngt.pads_rx[i].1));
        pads.push((format!("GTZTXP{i}"), PinDir::Output, &ngt.pads_tx[i].0));
        pads.push((format!("GTZTXN{i}"), PinDir::Output, &ngt.pads_tx[i].1));
    }
    for &(ref pin, dir, _) in &pads {
        pin_wires.insert(pin.clone(), (dir, format!("GTZE2_OCTAL_{pin}")));
    }
    let mut pins = vec![];
    for (pin, (dir, wire)) in &pin_wires {
        pins.push(SitePin {
            dir: match dir {
                PinDir::Input => SitePinDir::In,
                PinDir::Output => SitePinDir::Out,
                PinDir::Inout => unreachable!(),
            },
            pin: pin.into(),
            wire: Some(wire),
        });
        vrf.claim_net(&[RawWireCoord { crd: crd_gtz, wire }]);
    }
    vrf.claim_site(crd_gtz, &ngt.bel, "GTZE2_OCTAL", &pins);
    for &(ref pin, dir, bel) in &pads {
        vrf.claim_net(&[RawWireCoord {
            crd: crd_gtz,
            wire: &format!("GTZE2_OCTAL_{pin}_PAD"),
        }]);
        match dir {
            PinDir::Input => {
                vrf.claim_site(
                    crd_gtz,
                    bel,
                    "IPAD",
                    &[SitePin {
                        dir: SitePinDir::Out,
                        pin: "O".into(),
                        wire: Some(&format!("GTZE2_OCTAL_{pin}_PAD")),
                    }],
                );
                vrf.claim_pip_tri(
                    crd_gtz,
                    &format!("GTZE2_OCTAL_{pin}"),
                    &format!("GTZE2_OCTAL_{pin}_PAD"),
                );
            }
            PinDir::Output => {
                vrf.claim_site(
                    crd_gtz,
                    bel,
                    "OPAD",
                    &[SitePin {
                        dir: SitePinDir::In,
                        pin: "I".into(),
                        wire: Some(&format!("GTZE2_OCTAL_{pin}_PAD")),
                    }],
                );
                vrf.claim_pip_tri(
                    crd_gtz,
                    &format!("GTZE2_OCTAL_{pin}_PAD"),
                    &format!("GTZE2_OCTAL_{pin}"),
                );
            }
            PinDir::Inout => unreachable!(),
        }
    }
    let crd_clk = vrf.xlat_tile(&ngt.clk_tile).unwrap();
    let (sdie, srow) = if gtz.side == DirV::S {
        (DieId::from_idx(0), RowId::from_idx(4))
    } else {
        let sdie = endev.edev.chips.last_id().unwrap();
        (sdie, vrf.grid.rows(sdie).last().unwrap() - 19)
    };
    let obel_rebuf =
        vrf.get_legacy_bel(CellCoord::new(sdie, endev.edev.col_clk, srow).bel(bslots::CLK_REBUF));
    for i in 0..32 {
        let wire = format!("GTZ_CLK_GCLK{i}");
        let wire = RawWireCoord {
            crd: crd_clk,
            wire: &wire,
        };
        vrf.claim_net(&[
            wire,
            RawWireCoord {
                crd: crd_gtz,
                wire: &format!("GTZ_VBRK_INTF_GCLK{i}"),
            },
        ]);
        let owire = if gtz.side == DirV::S {
            format!("GTZ_CLK_TOP_IN_GCLK{i}")
        } else {
            format!("GTZ_CLK_BOT_IN_GCLK{i}")
        };
        let owire = RawWireCoord {
            crd: crd_clk,
            wire: &owire,
        };
        if out_gclk.contains(&i) {
            vrf.claim_pip(owire, wire);
        } else {
            vrf.claim_pip(wire, owire);
        }
        let dwire = if gtz.side == DirV::S {
            format!("GCLK{i}_D")
        } else {
            format!("GCLK{i}_U")
        };
        vrf.verify_net(&[obel_rebuf.wire(&dwire), owire]);
    }
    let sll_wire = wires::LVB[6];
    for icol in egt.cols.ids() {
        let crd = vrf.xlat_tile(&ngt.int_tiles[icol]).unwrap();
        let is_last = icol == egt.cols.last_id().unwrap();
        let is_first = icol == egt.cols.first_id().unwrap();
        let crd_next = if is_last {
            crd_gtz
        } else {
            vrf.xlat_tile(&ngt.int_tiles[icol + 1]).unwrap()
        };
        let gcol = egt.cols[icol];
        for col in egt.cols.ids() {
            for row in egt.rows.ids() {
                let wire_l = int_wire_name_int_l(gtz.side, icol, col, row);
                let wire_r = int_wire_name_int_r(gtz.side, icol, col, row);
                let wire_l = RawWireCoord { crd, wire: &wire_l };
                let wire_r = RawWireCoord { crd, wire: &wire_r };
                if col == icol {
                    let wire_i = int_wire_name_int_i(endev.edev, gtz.side, gcol, row);
                    let wire_i = RawWireCoord { crd, wire: &wire_i };
                    vrf.claim_pip(wire_i, wire_r);
                    vrf.claim_pip(wire_r, wire_i);
                    let rw = CellCoord::new(egt.die, gcol, egt.rows[row]).wire(sll_wire);
                    if !vrf.pin_int_wire(wire_i, rw) {
                        println!("FAIL TO PIN GTZ {col} {row}");
                    }
                } else {
                    vrf.claim_pip(wire_l, wire_r);
                    vrf.claim_pip(wire_r, wire_l);
                }
                if is_last {
                    let wire_gtz = int_wire_name_gtz(gtz.side, col, row);
                    let wire_gtz = RawWireCoord {
                        crd: crd_gtz,
                        wire: &wire_gtz,
                    };
                    vrf.claim_net(&[wire_r, wire_gtz]);
                } else {
                    let wire_l_next = int_wire_name_int_l(gtz.side, icol + 1, col, row);
                    let wire_l_next = RawWireCoord {
                        crd: crd_next,
                        wire: &wire_l_next,
                    };
                    vrf.claim_net(&[wire_r, wire_l_next]);
                }
                if is_first {
                    vrf.claim_net(&[wire_l]);
                }
            }
        }
    }
}

fn verify_extra(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    for (dir, egt) in &endev.edev.gtz {
        let ngt = &endev.gtz[dir];
        verify_gtz(endev, vrf, egt, ngt);
    }
    let mut stub_out_cond = vec![
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
        "LIOB_MONITOR_P",
        "LIOB_MONITOR_N",
        "RIOB_MONITOR_P",
        "RIOB_MONITOR_N",
    ];
    if vrf.rd.source == Source::Vivado {
        stub_out_cond.extend([
            "BRAM_PMVBRAM_SELECT1",
            "BRAM_PMVBRAM_SELECT2",
            "BRAM_PMVBRAM_SELECT3",
            "BRAM_PMVBRAM_SELECT4",
            // hmmmmm.
            "IOI_INT_DCI_EN",
            "IOI_DCI_TSTRST",
            "IOI_DCI_TSTHLP",
            "IOI_DCI_TSTHLN",
            "IOI_DCI_TSTCLK",
            "IOI_DCI_TSTRST0",
        ]);
    }
    for w in stub_out_cond {
        vrf.kill_stub_out_cond(w);
    }
    for prefix in ["PSS0", "PSS1", "PSS2"] {
        for i in 0..40 {
            for j in 0..2 {
                vrf.kill_stub_out_cond(&format!("{prefix}_CLK_B{j}_{i}"));
            }
            for j in 0..48 {
                vrf.kill_stub_out_cond(&format!("{prefix}_IMUX_B{j}_{i}"));
            }
            for j in 0..24 {
                vrf.kill_stub_in_cond(&format!("{prefix}_LOGIC_OUTS{j}_{i}"));
            }
        }
    }
    for i in 0..15 {
        for tb in ["BOT", "TOP"] {
            for lr in ['L', 'R'] {
                for j in 0..2 {
                    vrf.kill_stub_out_cond(&format!("PCIE3_{tb}_CLK{j}_{lr}_{i}"));
                    vrf.kill_stub_out_cond(&format!("PCIE3_{tb}_CTRL{j}_{lr}_{i}"));
                }
                for j in 0..48 {
                    vrf.kill_stub_out_cond(&format!("PCIE3_{tb}_IMUX{j}_{lr}_{i}"));
                }
            }
        }
    }
    if vrf.rd.source == Source::Vivado {
        for &crd in vrf.rd.tiles_by_kind_name("BRKH_INT") {
            if crd.y == vrf.rd.height - 1 {
                for w in [
                    "BRKH_INT_SL1END0",
                    "BRKH_INT_SL1END1",
                    "BRKH_INT_SL1END2",
                    "BRKH_INT_SL1END3",
                    "BRKH_INT_SR1END1",
                    "BRKH_INT_SR1END2",
                    "BRKH_INT_SR1END3",
                    "BRKH_INT_NL1BEG0_SLOW",
                    "BRKH_INT_NL1BEG1_SLOW",
                    "BRKH_INT_NL1BEG2_SLOW",
                    "BRKH_INT_NR1BEG0_SLOW",
                    "BRKH_INT_NR1BEG1_SLOW",
                    "BRKH_INT_NR1BEG2_SLOW",
                    "BRKH_INT_NR1BEG3_SLOW",
                ] {
                    vrf.claim_net(&[RawWireCoord { crd, wire: w }]);
                }
            }
        }
    }
}

fn verify_pre(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    if vrf.rd.source == Source::Vivado {
        for (tcrd, tile) in endev.edev.tiles() {
            if endev.edev.db.tile_classes.key(tile.class) == "CLK_BUFG" {
                for bel in endev.edev.db[tile.class].bels.ids() {
                    vrf.skip_bel_pin(tcrd.bel(bel), "FB_TEST0");
                    vrf.skip_bel_pin(tcrd.bel(bel), "FB_TEST1");
                }
            }
        }
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);
    verify_pre(endev, &mut vrf);
    vrf.prep_int_wires();
    vrf.handle_int();
    for (tcrd, tile) in endev.edev.tiles() {
        let tcls = &endev.edev.db[tile.class];
        for slot in tcls.bels.ids() {
            verify_bel(endev, &mut vrf, tcrd.bel(slot));
        }
    }
    verify_extra(endev, &mut vrf);
    vrf.finish();
}
