use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, PinDir, SwitchBoxItem, WireSlotIdExt},
    dir::{DirH, DirV},
    grid::{BelCoord, ColId, DieId, DieIdExt, RowId},
};
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_re_xilinx_naming_virtex4::{ExpandedNamedDevice, ExpandedNamedGtz};
use prjcombine_re_xilinx_rawdump::{Part, Source};
use prjcombine_re_xilinx_rdverify::{RawWireCoord, SitePin, SitePinDir, Verifier};
use prjcombine_virtex4::{
    chip::{ColumnKind, DisabledPart},
    defs::{
        bcls, bslots, cslots,
        virtex7::{ccls, tcls, wires},
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

fn verify_hclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let tcrd = endev.edev.bel_tile(bcrd);
    let has_s = endev.edev.has_bel(bcrd.delta(0, -1).bel(bslots::INT));
    let mut bel = vrf.verify_bel(bcrd);
    for i in 0..12 {
        bel.vrf.pin_int_wire(
            bel.wire(&format!("HCLK{i}")),
            endev
                .edev
                .resolve_tile_wire(tcrd, wires::HCLK_ROW[i].cell(1))
                .unwrap(),
        );
        if has_s {
            bel.vrf.pin_int_wire(
                bel.wire(&format!("LCLK{i}_S")),
                endev
                    .edev
                    .resolve_tile_wire(tcrd, wires::LCLK[i].cell(0))
                    .unwrap(),
            );
        }
        bel.vrf.pin_int_wire(
            bel.wire(&format!("LCLK{i}_N")),
            endev
                .edev
                .resolve_tile_wire(tcrd, wires::LCLK[i].cell(1))
                .unwrap(),
        );
    }
    for i in 0..4 {
        bel.vrf.pin_int_wire(
            bel.wire(&format!("RCLK{i}")),
            endev
                .edev
                .resolve_tile_wire(tcrd, wires::RCLK_ROW[i].cell(1))
                .unwrap(),
        );
    }
    for i in 0..6 {
        for sn in ['S', 'N'] {
            if sn == 'S' && !has_s {
                continue;
            }
            for j in 0..8 {
                bel.claim_pip(
                    bel.wire(&format!("LCLK{i}_{sn}")),
                    bel.wire(&format!("HCLK{j}")),
                );
            }
            for j in 8..12 {
                bel.claim_pip(
                    bel.wire(&format!("LCLK{i}_{sn}")),
                    bel.wire(&format!("HCLK{j}_I")),
                );
            }
            for j in 0..4 {
                bel.claim_pip(
                    bel.wire(&format!("LCLK{i}_{sn}")),
                    bel.wire(&format!("RCLK{j}_I")),
                );
            }
        }
    }
    for i in 6..12 {
        for sn in ['S', 'N'] {
            if sn == 'S' && !has_s {
                continue;
            }
            for j in 0..8 {
                bel.claim_pip(
                    bel.wire(&format!("LCLK{i}_{sn}")),
                    bel.wire(&format!("HCLK{j}_I")),
                );
            }
            for j in 8..12 {
                bel.claim_pip(
                    bel.wire(&format!("LCLK{i}_{sn}")),
                    bel.wire(&format!("HCLK{j}")),
                );
            }
            for j in 0..4 {
                bel.claim_pip(
                    bel.wire(&format!("LCLK{i}_{sn}")),
                    bel.wire(&format!("RCLK{j}")),
                );
            }
        }
    }
    for i in 8..12 {
        bel.claim_net(&[
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}_I")),
        ]);
        bel.claim_pip(
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}")),
        );
    }
    for i in 0..4 {
        bel.claim_net(&[
            bel.wire(&format!("RCLK{i}_O")),
            bel.wire(&format!("RCLK{i}_I")),
        ]);
        bel.claim_pip(
            bel.wire(&format!("RCLK{i}_O")),
            bel.wire(&format!("RCLK{i}")),
        );
    }
    for i in 0..8 {
        bel.claim_net(&[
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}_I")),
        ]);
        bel.claim_pip(
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}")),
        );
    }
}

fn verify_clk_rebuf(vrf: &mut Verifier, bcrd: BelCoord) {
    for i in 0..32 {
        let kind = if vrf
            .ngrid
            .get_bel_name_sub(bcrd, i)
            .unwrap()
            .starts_with("BUFG")
        {
            "BUFG_LB"
        } else {
            "GCLK_TEST_BUF"
        };
        let mut bel = vrf.verify_bel(bcrd).sub(i).kind(kind).skip_auto();
        let ii = i / 2;
        if i.is_multiple_of(2) {
            bel = bel
                .extra_in_claim_rename("CLKIN", format!("BUF{ii}_S_CLKIN"))
                .extra_out_rename("CLKOUT", format!("BUF{ii}_S_CLKOUT"));
            bel.claim_pip(
                bel.wire(&format!("BUF{ii}_S_CLKIN")),
                bel.wire(&format!("GCLK{i}_S")),
            );
        } else {
            bel = bel
                .extra_in_claim_rename("CLKIN", format!("BUF{ii}_N_CLKIN"))
                .extra_out_rename("CLKOUT", format!("BUF{ii}_N_CLKOUT"));
            bel.claim_pip(
                bel.wire(&format!("BUF{ii}_N_CLKIN")),
                bel.wire(&format!("GCLK{i}_N")),
            );
        }
        bel.commit();
    }
}

fn verify_clk_hrow(vrf: &mut Verifier, bcrd: BelCoord) {
    for i in 0..32 {
        let clkin = &format!("GCLK{i}_TEST_CLKIN");
        let clkout = &format!("GCLK{i}_TEST_CLKOUT");

        let mut bel = vrf
            .verify_bel(bcrd)
            .sub(i)
            .kind("GCLK_TEST_BUF")
            .skip_auto()
            .extra_in_claim_rename("CLKIN", clkin)
            .extra_out_claim_rename("CLKOUT", clkout);
        bel.claim_pip(bel.wire(clkin), bel.wire(&format!("GCLK{i}_TEST_IN")));
        bel.claim_net(&[bel.wire(&format!("GCLK{i}_TEST_OUT"))]);
        bel.claim_pip(bel.wire(&format!("GCLK{i}_TEST_OUT")), bel.wire(clkout));
        bel.claim_pip(
            bel.wire(&format!("GCLK_TEST{i}")),
            bel.wire(&format!("GCLK{i}_TEST_OUT")),
        );
        bel.claim_pip(
            bel.wire(&format!("GCLK_TEST{ii}", ii = i ^ 1)),
            bel.wire(&format!("GCLK{i}_TEST_OUT")),
        );
        bel.commit();
    }

    let mut bel = vrf
        .verify_bel(bcrd)
        .sub(32)
        .kind("GCLK_TEST_BUF")
        .skip_auto()
        .extra_in_claim_rename("CLKIN", "HCLK_TEST_W_CLKIN")
        .extra_out_claim_rename("CLKOUT", "HCLK_TEST_W_CLKOUT");
    bel.claim_pip(bel.wire("HCLK_TEST_W_CLKIN"), bel.wire("HCLK_TEST_IN_W"));
    bel.claim_pip(bel.wire("HCLK_TEST_OUT_W"), bel.wire("HCLK_TEST_W_CLKOUT"));
    bel.commit();

    let mut bel = vrf
        .verify_bel(bcrd)
        .sub(33)
        .kind("GCLK_TEST_BUF")
        .skip_auto()
        .extra_in_claim_rename("CLKIN", "HCLK_TEST_E_CLKIN")
        .extra_out_claim_rename("CLKOUT", "HCLK_TEST_E_CLKOUT");
    bel.claim_pip(bel.wire("HCLK_TEST_E_CLKIN"), bel.wire("HCLK_TEST_IN_E"));
    bel.claim_pip(bel.wire("HCLK_TEST_OUT_E"), bel.wire("HCLK_TEST_E_CLKOUT"));
    bel.commit();
}

fn verify_dci(vrf: &mut Verifier, bcrd: BelCoord) {
    vrf.verify_bel(bcrd)
        .extra_out_claim("DCIDATA")
        .extra_out_claim("DCIADDRESS0")
        .extra_out_claim("DCIADDRESS1")
        .extra_out_claim("DCIADDRESS2")
        .extra_out_claim("DCIIOUPDATE")
        .extra_out_claim("DCIREFIOUPDATE")
        .extra_out_claim("DCISCLK")
        .commit();
}

fn verify_ilogic(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::ILOGIC.index_of(bcrd.slot).unwrap();
    let tcrd = endev.edev.bel_tile(bcrd);
    let tcid = endev.edev[tcrd].class;
    let is_hp = matches!(tcid, tcls::IO_HP_PAIR | tcls::IO_HP_S | tcls::IO_HP_N);
    let is_single = !matches!(tcid, tcls::IO_HP_PAIR | tcls::IO_HR_PAIR);
    let kind = if is_hp { "ILOGICE2" } else { "ILOGICE3" };
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .skip_out(bcls::ILOGIC::CLKPAD)
        .extra_in_claim("OCLK")
        .extra_in_claim("OCLKB")
        .extra_in_claim("D")
        .extra_in_claim("DDLY")
        .extra_in_claim("OFB")
        .extra_in_claim("TFB")
        .extra_out_claim("SHIFTOUT1")
        .extra_out_claim("SHIFTOUT2")
        .extra_in_dummy("REV");
    if bcrd.slot != bslots::ILOGIC[0] || is_single {
        bel = bel.extra_in_dummy("SHIFTIN1").extra_in_dummy("SHIFTIN2");
    } else {
        bel = bel.extra_in_claim("SHIFTIN1").extra_in_claim("SHIFTIN2");
    }

    let obel_ologic = bcrd.bel(bslots::OLOGIC[idx]);

    bel.claim_pip(bel.wire("OCLK"), bel.bel_wire(obel_ologic, "CLK"));
    bel.claim_pip(bel.wire("OCLKB"), bel.bel_wire(obel_ologic, "CLK"));
    bel.claim_pip(bel.wire("OCLKB"), bel.bel_wire(obel_ologic, "CLKB"));
    bel.claim_pip(bel.wire("OFB"), bel.bel_wire(obel_ologic, "OFB"));
    bel.claim_pip(bel.wire("TFB"), bel.bel_wire(obel_ologic, "TFB"));

    let obel_idelay = bcrd.bel(bslots::IDELAY[idx]);
    bel.claim_pip(bel.wire("DDLY"), bel.bel_wire(obel_idelay, "DATAOUT"));

    let obel_iob = bcrd.bel(bslots::IOB[idx]);
    bel.claim_pip(bel.wire("D"), bel.wire("IOB_I_BUF"));
    bel.claim_net(&[bel.wire("IOB_I_BUF")]);
    bel.claim_pip(bel.wire("IOB_I_BUF"), bel.wire("IOB_I"));
    bel.verify_net(&[bel.wire("IOB_I"), bel.bel_wire(obel_iob, "I")]);

    if bcrd.slot == bslots::ILOGIC[0] && !is_single {
        let obel = bcrd.bel(bslots::ILOGIC[1]);
        bel.claim_pip(bel.wire("SHIFTIN1"), bel.bel_wire(obel, "SHIFTOUT1"));
        bel.claim_pip(bel.wire("SHIFTIN2"), bel.bel_wire(obel, "SHIFTOUT2"));
    }

    if bcrd.slot == bslots::ILOGIC[1] {
        let has_clkout = match bel.vrf.rd.source {
            Source::ISE => matches!(bcrd.row.to_idx() % 50, 7 | 19 | 31 | 43 | 21 | 23 | 25 | 27),
            Source::Vivado => !matches!(bcrd.row.to_idx() % 50, 13 | 37),
        };
        if has_clkout {
            bel.claim_pip(bel.wire("CLKPAD"), bel.wire("O"));
        }
    }

    bel.commit()
}

fn verify_ologic(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::OLOGIC.index_of(bcrd.slot).unwrap();
    let tcrd = endev.edev.bel_tile(bcrd);
    let tcid = endev.edev[tcrd].class;
    let is_hp = matches!(tcid, tcls::IO_HP_PAIR | tcls::IO_HP_S | tcls::IO_HP_N);

    let kind = if is_hp { "OLOGICE2" } else { "OLOGICE3" };
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .skip_in(bcls::OLOGIC::CLK)
        .skip_in(bcls::OLOGIC::CLKB)
        .skip_out(bcls::OLOGIC::TFB)
        .extra_in_claim_rename("CLK", "CLK_FAKE")
        .extra_in_claim_rename("CLKB", "CLKB_FAKE")
        .extra_out_claim("OFB")
        .extra_out_claim_rename("TFB", "TFB_FAKE")
        .extra_out_claim("OQ")
        .extra_out_claim("TQ")
        .extra_out_claim("SHIFTOUT1")
        .extra_out_claim("SHIFTOUT2")
        .extra_in_dummy("REV")
        .extra_in_claim("TBYTEIN")
        .extra_out_claim("TBYTEOUT");
    if bcrd.slot != bslots::OLOGIC[1] {
        bel = bel.extra_in_dummy("SHIFTIN1").extra_in_dummy("SHIFTIN2")
    } else {
        bel = bel.extra_in_claim("SHIFTIN1").extra_in_claim("SHIFTIN2")
    }

    bel.claim_pip(bel.wire("CLK_FAKE"), bel.wire("CLK"));
    bel.claim_pip(bel.wire("CLKB_FAKE"), bel.wire("CLK"));
    bel.claim_pip(bel.wire("CLKB_FAKE"), bel.wire("CLKB"));

    bel.claim_pip(bel.wire("TFB"), bel.wire("TFB_FAKE"));

    let obel_iob = bcrd.bel(bslots::IOB[idx]);
    bel.claim_pip(bel.wire("IOB_T"), bel.wire("TQ"));
    bel.claim_pip(bel.wire("IOB_O"), bel.wire("OQ"));
    if kind == "OLOGICE2" {
        let obel_odelay = bcrd.bel(bslots::ODELAY[idx]);
        bel.claim_pip(bel.wire("IOB_O"), bel.bel_wire(obel_odelay, "DATAOUT"));
    }
    bel.verify_net(&[bel.wire("IOB_O"), bel.bel_wire(obel_iob, "O")]);
    bel.verify_net(&[bel.wire("IOB_T"), bel.bel_wire(obel_iob, "T")]);

    if bcrd.slot == bslots::OLOGIC[1] {
        let obel = bcrd.bel(bslots::OLOGIC[0]);
        bel.claim_pip(bel.wire("SHIFTIN1"), bel.bel_wire(obel, "SHIFTOUT1"));
        bel.claim_pip(bel.wire("SHIFTIN2"), bel.bel_wire(obel, "SHIFTOUT2"));
    }

    bel.claim_pip(bel.wire("TBYTEIN"), bel.wire("TBYTEIN_IOI"));

    let y = bcrd.row.to_idx() % 50 + idx;
    let tbyte_sy = match y {
        0..13 => 7,
        13..25 => 19,
        25..37 => 31,
        37..50 => 43,
        _ => unreachable!(),
    };
    let tbyte_srow = RowId::from_idx(bcrd.row.to_idx() / 50 * 50 + tbyte_sy);
    if tbyte_sy == y {
        bel.claim_net(&[bel.wire("TBYTEIN_IOI")]);
        bel.claim_pip(bel.wire("TBYTEIN_IOI"), bel.wire("TBYTEOUT"));
    } else {
        let obel = bcrd.cell.with_row(tbyte_srow).bel(bslots::OLOGIC[0]);
        bel.verify_net(&[bel.wire("TBYTEIN_IOI"), bel.bel_wire(obel, "TBYTEIN_IOI")]);
    }

    bel.commit();
}

fn verify_idelay(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IDELAY.index_of(bcrd.slot).unwrap();
    let tcrd = vrf.grid.bel_tile(bcrd);
    let tcid = vrf.grid[tcrd].class;
    let is_hp = matches!(tcid, tcls::IO_HP_PAIR | tcls::IO_HP_S | tcls::IO_HP_N);
    let kind = if is_hp {
        "IDELAYE2_FINEDELAY"
    } else {
        "IDELAYE2"
    };
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_in_claim("IDATAIN")
        .extra_out_claim("DATAOUT");

    let obel_ilogic = bcrd.bel(bslots::ILOGIC[idx]);
    bel.claim_pip(bel.wire("IDATAIN"), bel.bel_wire(obel_ilogic, "IOB_I_BUF"));

    let obel_ologic = bcrd.bel(bslots::OLOGIC[idx]);
    bel.claim_pip(bel.wire("IDATAIN"), bel.bel_wire(obel_ologic, "OFB"));

    bel.commit();
}

fn verify_odelay(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::ODELAY.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("ODELAYE2")
        .extra_in_claim("CLKIN")
        .extra_in_claim("ODATAIN");

    let obel_ologic = bcrd.bel(bslots::OLOGIC[idx]);
    bel.claim_pip(bel.wire("CLKIN"), bel.bel_wire(obel_ologic, "CLK"));
    bel.claim_pip(bel.wire("ODATAIN"), bel.bel_wire(obel_ologic, "OFB"));

    bel.commit();
}

fn verify_iob(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IOB.index_of(bcrd.slot).unwrap();
    let tcrd = vrf.grid.bel_tile(bcrd);
    let tcid = vrf.grid[tcrd].class;
    let (kind, is_single) = match (idx, tcid) {
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
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_out_claim("I")
        .extra_in_claim("O")
        .extra_in_claim("T")
        .extra_out_claim("O_OUT")
        .extra_out_claim("T_OUT")
        .extra_out_claim("DIFFO_OUT")
        .extra_out_claim("PADOUT");
    if is_single {
        bel = bel
            .extra_in_dummy("DIFFI_IN")
            .extra_in_dummy("DIFFO_IN")
            .extra_in_dummy("O_IN")
            .extra_in_dummy("T_IN")
            .extra_in_dummy("DIFF_TERM_INT_EN");
    } else if idx == 0 {
        bel = bel
            .extra_in_claim("DIFFI_IN")
            .extra_in_claim("DIFFO_IN")
            .extra_in_claim("O_IN")
            .extra_in_claim("T_IN");
    } else {
        bel = bel
            .extra_in_claim("DIFFI_IN")
            .extra_in_dummy("DIFFO_IN")
            .extra_in_dummy("O_IN")
            .extra_in_dummy("T_IN")
            .extra_in_dummy("DIFF_TERM_INT_EN");
    }
    if !is_single {
        let oslot = bslots::IOB[idx ^ 1];
        let obel = bcrd.bel(oslot);
        if idx == 0 {
            bel.claim_pip(bel.wire("O_IN"), bel.bel_wire(obel, "O_OUT"));
            bel.claim_pip(bel.wire("T_IN"), bel.bel_wire(obel, "T_OUT"));
            bel.claim_pip(bel.wire("DIFFO_IN"), bel.bel_wire(obel, "DIFFO_OUT"));
        }
        bel.claim_pip(bel.wire("DIFFI_IN"), bel.bel_wire(obel, "PADOUT"));
    }
    bel.commit();
}

fn verify_phaser_in(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::PHASER_IN.index_of(bcrd.slot).unwrap();
    let abcd = ['A', 'B', 'C', 'D'][idx];
    let mut bel = vrf.verify_bel(bcrd).kind("PHASER_IN_PHY");
    let obel_pc = bcrd.bel(bslots::PHY_CONTROL);
    for (pin, opin) in [
        ("ENCALIBPHY0", "PCENABLECALIB0"),
        ("ENCALIBPHY1", "PCENABLECALIB1"),
        ("RANKSELPHY0", &format!("INRANK{abcd}0")),
        ("RANKSELPHY1", &format!("INRANK{abcd}1")),
        ("BURSTPENDINGPHY", &format!("INBURSTPENDING{idx}")),
    ] {
        bel = bel.extra_in_claim(pin);
        bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
        bel.verify_net(&[bel.wire_far(pin), bel.bel_wire_far(obel_pc, opin)]);
    }
    bel.commit();
}

fn verify_phaser_out(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::PHASER_OUT.index_of(bcrd.slot).unwrap();
    let mut bel = vrf.verify_bel(bcrd).kind("PHASER_OUT_PHY");
    let obel_pc = bcrd.bel(bslots::PHY_CONTROL);
    for (pin, opin) in [
        ("ENCALIBPHY0", "PCENABLECALIB0"),
        ("ENCALIBPHY1", "PCENABLECALIB1"),
        ("BURSTPENDINGPHY", &format!("OUTBURSTPENDING{idx}")),
    ] {
        bel = bel.extra_in_claim(pin);
        bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
        bel.verify_net(&[bel.wire_far(pin), bel.bel_wire_far(obel_pc, opin)]);
    }
    bel.commit();
}

fn verify_phy_control(vrf: &mut Verifier, bcrd: BelCoord) {
    vrf.verify_bel(bcrd)
        .extra_out_claim_far("INRANKA0")
        .extra_out_claim_far("INRANKA1")
        .extra_out_claim_far("INRANKB0")
        .extra_out_claim_far("INRANKB1")
        .extra_out_claim_far("INRANKC0")
        .extra_out_claim_far("INRANKC1")
        .extra_out_claim_far("INRANKD0")
        .extra_out_claim_far("INRANKD1")
        .extra_out_claim_far("PCENABLECALIB0")
        .extra_out_claim_far("PCENABLECALIB1")
        .extra_out_claim_far("INBURSTPENDING0")
        .extra_out_claim_far("INBURSTPENDING1")
        .extra_out_claim_far("INBURSTPENDING2")
        .extra_out_claim_far("INBURSTPENDING3")
        .extra_out_claim_far("OUTBURSTPENDING0")
        .extra_out_claim_far("OUTBURSTPENDING1")
        .extra_out_claim_far("OUTBURSTPENDING2")
        .extra_out_claim_far("OUTBURSTPENDING3")
        .commit();
}

fn verify_mmcm(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("MMCME2_ADV");
    bel.claim_net(&[bel.wire("CLKFB")]);
    bel.claim_pip(bel.wire("CLKFB"), bel.wire("CLKFBOUT"));
    bel.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFB"));
    bel.commit()
}

fn verify_pll(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("PLLE2_ADV");
    bel.claim_net(&[bel.wire("CLKFB")]);
    bel.claim_pip(bel.wire("CLKFB"), bel.wire("CLKFBOUT"));
    bel.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFB"));
    bel.commit()
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

fn verify_xadc(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("XADC")
        .ipad("VP", 1)
        .ipad("VN", 2);

    for i in 0..16 {
        let vauxp = &format!("VAUXP{i}");
        let vauxn = &format!("VAUXN{i}");
        let Some((iop, _)) = endev.edev.get_sysmon_vaux(bcrd.cell, i) else {
            bel = bel.extra_in_dummy(vauxp).extra_in_dummy(vauxn);
            continue;
        };
        bel = bel.extra_in_claim(vauxp).extra_in_claim(vauxn);
        let (ow0p, iw0p) = bel.pip(vauxp, 0);
        let (ow1p, iw1p) = bel.pip(vauxp, 1);
        let (ow0n, iw0n) = bel.pip(vauxn, 0);
        let (ow1n, iw1n) = bel.pip(vauxn, 1);
        bel.claim_pip(ow0p, iw0p);
        bel.claim_pip(ow1p, iw1p);
        bel.claim_pip(ow0n, iw0n);
        bel.claim_pip(ow1n, iw1n);
        bel.claim_net(&[iw0p, ow1p]);
        bel.claim_net(&[iw0n, ow1n]);
        let obel = iop.cell.bel(bslots::IOB[1]);
        bel.claim_net(&[iw1p, bel.bel_wire(obel, "MONITOR")]);
        bel.claim_pip(bel.bel_wire(obel, "MONITOR"), bel.bel_wire(obel, "PADOUT"));
        let obel = iop.cell.bel(bslots::IOB[0]);
        bel.claim_net(&[iw1n, bel.bel_wire(obel, "MONITOR")]);
        bel.claim_pip(bel.bel_wire(obel, "MONITOR"), bel.bel_wire(obel, "PADOUT"));
    }

    if bcrd.die == endev.edev.interposer.unwrap().primary || bel.vrf.rd.source != Source::Vivado {
        bel.commit();
    }
}

fn verify_ps(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("PS7");
    let mut pins = vec![];
    pins.push("DDRWEB".to_string());
    pins.push("DDRVRN".to_string());
    pins.push("DDRVRP".to_string());
    for i in 0..13 {
        pins.push(format!("DDRA{i}"));
    }
    pins.push("DDRA14".to_string());
    pins.push("DDRA13".to_string());
    for i in 0..3 {
        pins.push(format!("DDRBA{i}"));
    }
    pins.push("DDRCASB".to_string());
    pins.push("DDRCKE".to_string());
    pins.push("DDRCKN".to_string());
    pins.push("DDRCKP".to_string());
    pins.push("PSCLK".to_string());
    pins.push("DDRCSB".to_string());
    for i in 0..4 {
        pins.push(format!("DDRDM{i}"));
    }
    for i in 0..32 {
        pins.push(format!("DDRDQ{i}"));
    }
    for i in 0..4 {
        pins.push(format!("DDRDQSN{i}"));
    }
    for i in 0..4 {
        pins.push(format!("DDRDQSP{i}"));
    }
    pins.push("DDRDRSTB".to_string());
    for i in 0..54 {
        pins.push(format!("MIO{i}"));
    }
    pins.push("DDRODT".to_string());
    pins.push("PSPORB".to_string());
    pins.push("DDRRASB".to_string());
    pins.push("PSSRSTB".to_string());

    for (i, pin) in pins.into_iter().enumerate() {
        bel = bel.iopad(pin, i + 1);
    }
    bel.commit();
}

fn verify_ibufds(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::BUFDS.index_of(bcrd.slot).unwrap();
    let pins = [
        ("I", SitePinDir::In),
        ("IB", SitePinDir::In),
        ("O", SitePinDir::Out),
        ("ODIV2", SitePinDir::Out),
    ];
    if !endev.edev.disabled.contains(&DisabledPart::Gtp) || vrf.rd.source == Source::ISE {
        vrf.verify_legacy_bel(bel, "IBUFDS_GTE2", &pins, &["MGTCLKOUT"]);
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
    if let Some((cl, cr)) = endev.edev.col_gt_m {
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
                .with_col(endev.edev.col_gt_m.unwrap().1)
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
                .with_col(endev.edev.col_gt_m.unwrap().0)
                .bel(bslots::GTP_COMMON),
        );
        vrf.verify_net(&[bel.wire("EASTCLK0"), obel.wire("EASTCLK0")]);
        vrf.verify_net(&[bel.wire("EASTCLK1"), obel.wire("EASTCLK1")]);
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
        &[
            "HOUT4", "HOUT5", "HOUT6", "HOUT7", "HOUT8", "HOUT9", "HOUT10", "HOUT11", "HOUT12",
            "HOUT13",
        ],
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
    let tcrd = endev.edev.bel_tile(bcrd);
    let tcid = endev.edev[tcrd].class;
    match bcrd.slot {
        bslots::SPEC_INT if matches!(tcid, tcls::CLK_BUFG_REBUF | tcls::CLK_BALI_REBUF) => {
            verify_clk_rebuf(vrf, bcrd)
        }
        bslots::SPEC_INT if tcid == tcls::CLK_HROW => verify_clk_hrow(vrf, bcrd),
        bslots::INT
        | bslots::INTF_INT
        | bslots::INTF_TESTMUX
        | bslots::SPEC_INT
        | bslots::CMT_FIFO_INT
        | bslots::CLK_INT
        | bslots::HROW_INT
        | bslots::HCLK_IO_INT
        | bslots::MISC_CFG
        | bslots::BANK
        | bslots::GLOBAL
        | bslots::HCLK_DRP_GTP_MID => (),
        _ if bslots::HCLK_DRP.contains(bcrd.slot) => (),
        _ if bslots::SLICE.contains(bcrd.slot) => verify_slice(endev, vrf, bcrd),
        _ if bslots::DSP.contains(bcrd.slot) => verify_dsp(vrf, bcrd),
        bslots::TIEOFF_DSP => verify_tieoff(vrf, bcrd),
        bslots::BRAM_F => verify_bram_f(vrf, bcrd),
        _ if bslots::BRAM_H.contains(bcrd.slot) => verify_bram_h(vrf, bcrd),
        bslots::BRAM_ADDR => verify_bram_addr(vrf, bcrd),
        bslots::PCIE => {
            if !endev.edev.disabled.contains(&DisabledPart::Gtp) || vrf.rd.source == Source::ISE {
                vrf.verify_bel(bcrd).kind("PCIE_2_1").commit();
            }
        }
        bslots::PCIE3 => vrf.verify_bel(bcrd).kind("PCIE_3_0").commit(),
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

        bslots::HCLK => verify_hclk(endev, vrf, bcrd),
        _ if bslots::BUFHCE_W.contains(bcrd.slot) || bslots::BUFHCE_E.contains(bcrd.slot) => {
            vrf.verify_bel(bcrd).commit()
        }
        _ if bslots::BUFGCTRL.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),

        _ if bslots::BUFIO.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        _ if bslots::BUFR.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        bslots::IDELAYCTRL => vrf.verify_bel(bcrd).commit(),
        bslots::DCI => verify_dci(vrf, bcrd),

        _ if bslots::ILOGIC.contains(bcrd.slot) => verify_ilogic(endev, vrf, bcrd),
        _ if bslots::OLOGIC.contains(bcrd.slot) => verify_ologic(endev, vrf, bcrd),
        _ if bslots::IDELAY.contains(bcrd.slot) => verify_idelay(vrf, bcrd),
        _ if bslots::ODELAY.contains(bcrd.slot) => verify_odelay(vrf, bcrd),
        _ if bslots::IOB.contains(bcrd.slot) => verify_iob(vrf, bcrd),

        _ if bslots::PHASER_IN.contains(bcrd.slot) => verify_phaser_in(vrf, bcrd),
        _ if bslots::PHASER_OUT.contains(bcrd.slot) => verify_phaser_out(vrf, bcrd),
        bslots::PHASER_REF => vrf.verify_bel(bcrd).commit(),
        bslots::PHY_CONTROL => verify_phy_control(vrf, bcrd),
        _ if bcrd.slot == bslots::PLL[0] => verify_mmcm(vrf, bcrd),
        _ if bcrd.slot == bslots::PLL[1] => verify_pll(vrf, bcrd),
        _ if bslots::BUFMRCE.contains(bcrd.slot) => vrf.verify_bel(bcrd).kind("BUFMRCE").commit(),
        bslots::IN_FIFO | bslots::OUT_FIFO => vrf.verify_bel(bcrd).commit(),

        bslots::SYSMON => verify_xadc(endev, vrf, bcrd),
        _ if slot_name.starts_with("IPAD") => verify_ipad(endev, vrf, bcrd),
        _ if slot_name.starts_with("OPAD") => verify_opad(endev, vrf, bcrd),
        bslots::PS => verify_ps(vrf, bcrd),

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
        (DieId::from_idx(0), RowId::from_idx(13))
    } else {
        let sdie = endev.edev.chips.last_id().unwrap();
        (sdie, vrf.grid.rows(sdie).last().unwrap() - 12)
    };
    let obel_rebuf = sdie.cell(endev.edev.col_clk, srow).bel(bslots::SPEC_INT);
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
            format!("GCLK{i}_S")
        } else {
            format!("GCLK{i}_N")
        };
        vrf.verify_net(&[vrf.bel_wire(obel_rebuf, &dwire), owire]);
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
                    let rw = egt.die.cell(gcol, egt.rows[row]).wire(sll_wire);
                    if !vrf.try_pin_int_wire(wire_i, rw) {
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

fn verify_pre_hclk(vrf: &mut Verifier) {
    vrf.skip_bslot(bslots::HCLK);
    for tkn in ["INT_L", "INT_L_SLV", "INT_L_SLV_FLY"] {
        for i in 6..12 {
            vrf.mark_merge_pip(tkn, &format!("GCLK_L_B{i}_WEST"), &format!("GCLK_L_B{i}"));
            vrf.mark_merge_pip(tkn, &format!("GCLK_L_B{i}_EAST"), &format!("GCLK_L_B{i}"));
        }
    }
    for tkn in ["INT_R", "INT_R_SLV", "INT_R_SLV_FLY"] {
        for i in 0..6 {
            vrf.mark_merge_pip(tkn, &format!("GCLK_B{i}_WEST"), &format!("GCLK_B{i}"));
            vrf.mark_merge_pip(tkn, &format!("GCLK_B{i}_EAST"), &format!("GCLK_B{i}"));
        }
    }
}

fn verify_pre_clk_bufg(vrf: &mut Verifier) {
    for tkn in ["CLK_BUFG_BOT_R", "CLK_BUFG_TOP_R"] {
        for i in 0..16 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CLK_BUFG_LOGIC_OUTS_B{j}_{k}", j = 4 + i % 4, k = i / 4),
                &format!("CLK_BUFG_R_CK_FB_TEST0_{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("CLK_BUFG_LOGIC_OUTS_B{j}_{k}", j = i % 4, k = i / 4),
                &format!("CLK_BUFG_R_CK_FB_TEST1_{i}"),
            );
        }
    }

    // very likely a case of wrong-direction pip
    for tcid in [tcls::CLK_BUFG_S, tcls::CLK_BUFG_N] {
        if vrf.grid.tile_index[tcid].is_empty() {
            continue;
        }
        let BelInfo::SwitchBox(ref sb) = vrf.grid.db[tcid].bels[bslots::SPEC_INT] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::ProgBuf(buf) = item else {
                continue;
            };
            if !wires::OUT_BEL.contains(buf.dst.wire) {
                continue;
            }
            vrf.skip_tcls_pip(tcid, buf.dst, buf.src.tw);
            vrf.inject_tcls_pip(tcid, buf.src.tw, buf.dst);
        }
    }
}

fn verify_pre_clk_hrow(vrf: &mut Verifier) {
    for &tcrd in &vrf.grid.tile_index[tcls::CLK_HROW] {
        // insert invisible buffers
        let BelInfo::SwitchBox(ref sb) = vrf.grid.db[tcls::CLK_HROW].bels[bslots::SPEC_INT] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::ProgBuf(buf) = item else {
                continue;
            };
            if wires::GCLK_TEST_IN.contains(buf.dst.wire)
                || wires::CKINT_HROW.contains(buf.dst.wire)
            {
                continue;
            }
            vrf.skip_tcls_pip(tcls::CLK_HROW, buf.dst, buf.src.tw);
            let wt = vrf.grid.resolve_tile_wire(tcrd, buf.dst).unwrap();
            let wf = vrf.grid.resolve_tile_wire(tcrd, buf.src.tw).unwrap();
            vrf.alias_wire(wt, wf);
        }
    }
    for i in 0..32 {
        vrf.skip_tcls_pip(
            tcls::CLK_HROW,
            wires::IMUX_BUFG_O[i].cell(1),
            wires::GCLK_TEST[i ^ 1].cell(1),
        );
    }
}

fn verify_pre_ps(vrf: &mut Verifier) {
    for i in 0..4 {
        vrf.mark_merge_pip(
            "PSS1",
            &format!("PSS_HCLK_CK_IN{i}"),
            &format!("PSS_FCLKCLK{i}"),
        );
    }
    vrf.mark_merge_pip("PSS1", "PSS_LOGIC_OUTS1_19", "PSS1_LOGIC_OUTS1_39");
    vrf.mark_merge_pip("PSS1", "PSS_LOGIC_OUTS2_19", "PSS1_LOGIC_OUTS2_39");
    vrf.mark_merge_pip("PSS3", "PSS_LOGIC_OUTS0_1", "PSS1_LOGIC_OUTS0_1");
    vrf.mark_merge_pip("PSS3", "PSS_LOGIC_OUTS1_1", "PSS1_LOGIC_OUTS1_1");
}

fn verify_pre_cmt(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    for (tkn, lr) in [("CMT_TOP_L_LOWER_B", 'L'), ("CMT_TOP_R_LOWER_B", 'R')] {
        for (i, pin) in [
            "CLKOUT0",
            "CLKOUT0B",
            "CLKOUT1",
            "CLKOUT1B",
            "CLKOUT2",
            "CLKOUT2B",
            "CLKOUT3",
            "CLKOUT3B",
            "CLKOUT4",
            "CLKOUT5",
            "CLKOUT6",
            "CLKFBOUT",
            "CLKFBOUTB",
            "TMUXOUT",
        ]
        .into_iter()
        .enumerate()
        {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_{lr}_LOWER_B_CLK_MMCM{i}"),
                &format!("CMT_LR_LOWER_B_MMCM_{pin}"),
            );
        }
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_{lr}_LOWER_B_CLK_FREQ_BB{i}"),
                &format!("MMCM_CLK_FREQ_BB_NS{ii}", ii = 3 - i),
            );
        }
    }

    for tkn in ["CMT_TOP_L_UPPER_B", "CMT_TOP_R_UPPER_B"] {
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_FREQ_BB_PREF_IN{i}"),
                &format!("PLL_CLK_FREQBB_REBUFOUT{i}"),
            );
        }
    }
    for (tkn, lr) in [("CMT_TOP_L_UPPER_T", 'L'), ("CMT_TOP_R_UPPER_T", 'R')] {
        for (i, pin) in [
            "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKFBOUT", "TMUXOUT",
        ]
        .into_iter()
        .enumerate()
        {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_{lr}_UPPER_T_CLKPLL{i}"),
                &format!("CMT_TOP_R_UPPER_T_PLLE2_{pin}"),
            );
        }
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_{lr}_UPPER_T_FREQ_BB{i}"),
                &format!("PLL_CLK_FREQ_BB{i}_NS"),
            );
        }
    }
    vrf.mark_merge_pip(
        "CMT_TOP_R_LOWER_B",
        "CMT_R_LOWER_B_CLK_IN1_INT",
        "CMT_TOP_CLK0_15",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_R_LOWER_B",
        "CMT_R_LOWER_B_CLK_IN2_INT",
        "CMT_TOP_CLK1_15",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_R_LOWER_B",
        "CMT_R_LOWER_B_CLK_IN3_INT",
        "CMT_TOP_CLK0_14",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_LOWER_B",
        "CMT_L_LOWER_B_CLK_IN1_INT",
        "CMT_TOP_CLK0_15",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_LOWER_B",
        "CMT_L_LOWER_B_CLK_IN2_INT",
        "CMT_TOP_CLK1_15",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_LOWER_B",
        "CMT_L_LOWER_B_CLK_IN3_INT",
        "CMT_TOP_CLK0_14",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_R_UPPER_T",
        "CMT_TOP_R_UPPER_T_PLLE2_CLK_IN1_INT",
        "CMT_TOP_CLK1_0",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_R_UPPER_T",
        "CMT_TOP_R_UPPER_T_PLLE2_CLK_IN2_INT",
        "CMT_TOP_CLK0_0",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_R_UPPER_T",
        "CMT_TOP_R_UPPER_T_PLLE2_CLK_FB_INT",
        "CMT_TOP_CLK0_1",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_UPPER_T",
        "CMT_TOP_L_UPPER_T_PLLE2_CLK_IN1_INT",
        "CMT_TOP_CLK1_0",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_UPPER_T",
        "CMT_TOP_L_UPPER_T_PLLE2_CLK_IN2_INT",
        "CMT_TOP_CLK0_0",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_UPPER_T",
        "CMT_TOP_L_UPPER_T_PLLE2_CLK_FB_INT",
        "CMT_TOP_CLK0_1",
    );

    for tkn in ["CMT_TOP_L_LOWER_B", "CMT_TOP_R_LOWER_B"] {
        for i in 0..13 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_ICLKDIV_{i}"),
                "CMT_PHASER_A_ICLKDIV_TOIOI",
            );
            vrf.mark_merge_pip(tkn, &format!("CMT_TOP_ICLK_{i}"), "CMT_PHASER_A_ICLK_TOIOI");
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_OCLKDIV_{i}"),
                "CMT_PHASER_A_OCLKDIV_TOIOI",
            );
            vrf.mark_merge_pip(tkn, &format!("CMT_TOP_OCLK_{i}"), "CMT_PHASER_A_OCLK_TOIOI");
        }
        for i in 13..16 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_ICLKDIV_{i}"),
                "CMT_MMCM_PHASER_IN_B_ICLKDIV",
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_ICLK_{i}"),
                "CMT_MMCM_PHASER_IN_B_ICLK",
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_OCLKDIV_{i}"),
                "CMT_MMCM_PHASER_OUT_B_OCLKDIV",
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_OCLK_{i}"),
                "CMT_MMCM_PHASER_OUT_B_OCLK",
            );
        }
        vrf.mark_merge_pip(tkn, "CMT_PHASER_A_ICLK_TOIOI", "CMT_MMCM_PHASER_IN_A_ICLK");
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_A_ICLKDIV_TOIOI",
            "CMT_MMCM_PHASER_IN_A_ICLKDIV",
        );
        vrf.mark_merge_pip(tkn, "CMT_PHASER_A_OCLK_TOIOI", "CMT_MMCM_PHASER_OUT_A_OCLK");
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_A_OCLKDIV_TOIOI",
            "CMT_MMCM_PHASER_OUT_A_OCLKDIV",
        );
    }
    for tkn in ["CMT_TOP_L_LOWER_T", "CMT_TOP_R_LOWER_T"] {
        for i in 0..9 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_ICLKDIV_{i}"),
                "CMT_PHASER_B_ICLKDIV_TOIOI",
            );
            vrf.mark_merge_pip(tkn, &format!("CMT_TOP_ICLK_{i}"), "CMT_PHASER_B_ICLK_TOIOI");
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_OCLKDIV_{i}"),
                "CMT_PHASER_B_OCLKDIV_TOIOI",
            );
            vrf.mark_merge_pip(tkn, &format!("CMT_TOP_OCLK_{i}"), "CMT_PHASER_B_OCLK_TOIOI");
        }
        vrf.mark_merge_pip(tkn, "CMT_PHASER_B_ICLK_TOIOI", "CMT_PHASER_IN_B_ICLK");
        vrf.mark_merge_pip(tkn, "CMT_PHASER_B_ICLKDIV_TOIOI", "CMT_PHASER_IN_B_ICLKDIV");
        vrf.mark_merge_pip(tkn, "CMT_PHASER_B_OCLK_TOIOI", "CMT_PHASER_OUT_B_OCLK");
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_B_OCLKDIV_TOIOI",
            "CMT_PHASER_OUT_B_OCLKDIV",
        );
        vrf.mark_merge_pip(tkn, "CMT_PHASER_B_TOMMCM_ICLK", "CMT_PHASER_B_ICLK_TOIOI");
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_B_TOMMCM_ICLKDIV",
            "CMT_PHASER_B_ICLKDIV_TOIOI",
        );
        vrf.mark_merge_pip(tkn, "CMT_PHASER_B_TOMMCM_OCLK", "CMT_PHASER_B_OCLK_TOIOI");
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_B_TOMMCM_OCLKDIV",
            "CMT_PHASER_B_OCLKDIV_TOIOI",
        );
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_IN_A_ICLKDIV",
            "CMT_PHASER_IN_A_WRCLK_TOFIFO",
        );
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_OUT_A_OCLKDIV",
            "CMT_PHASER_OUT_A_RDCLK_TOFIFO",
        );
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_IN_B_ICLKDIV",
            "CMT_PHASER_IN_B_WRCLK_TOFIFO",
        );
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_OUT_B_OCLKDIV",
            "CMT_PHASER_OUT_B_RDCLK_TOFIFO",
        );
    }
    for tkn in ["CMT_TOP_L_UPPER_B", "CMT_TOP_R_UPPER_B"] {
        for i in 0..12 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_ICLKDIV_{i}"),
                "CMT_PHASER_C_ICLKDIV_TOIOI",
            );
            vrf.mark_merge_pip(tkn, &format!("CMT_TOP_ICLK_{i}"), "CMT_PHASER_C_ICLK_TOIOI");
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_OCLKDIV_{i}"),
                "CMT_PHASER_C_OCLKDIV_TOIOI",
            );
            vrf.mark_merge_pip(tkn, &format!("CMT_TOP_OCLK_{i}"), "CMT_PHASER_C_OCLK_TOIOI");
        }
        vrf.mark_merge_pip(tkn, "CMT_PHASER_C_ICLK_TOIOI", "CMT_PHASER_IN_C_ICLK");
        vrf.mark_merge_pip(tkn, "CMT_PHASER_C_ICLKDIV_TOIOI", "CMT_PHASER_IN_C_ICLKDIV");
        vrf.mark_merge_pip(tkn, "CMT_PHASER_C_OCLK_TOIOI", "CMT_PHASER_OUT_C_OCLK");
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_C_OCLKDIV_TOIOI",
            "CMT_PHASER_OUT_C_OCLKDIV",
        );
    }
    vrf.mark_merge_pip(
        "CMT_TOP_R_UPPER_B",
        "CMT_PHASER_IN_C_ICLKDIV",
        "CMT_PHASER_IN_C_WRCLK_TOFIFO",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_R_UPPER_B",
        "CMT_PHASER_OUT_C_OCLKDIV",
        "CMT_PHASER_OUT_C_RDCLK_TOFIFO",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_R_UPPER_B",
        "CMT_PHASER_IN_D_ICLKDIV",
        "CMT_PHASER_IN_D_WRCLK_TOFIFO",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_R_UPPER_B",
        "CMT_PHASER_OUT_D_OCLKDIV",
        "CMT_PHASER_OUT_D_RDCLK_TOFIFO",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_UPPER_B",
        "CMT_PHASER_IN_C_ICLKDIV",
        "CMT_R_PHASER_IN_C_WRCLK_FIFO",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_UPPER_B",
        "CMT_PHASER_OUT_C_OCLKDIV",
        "CMT_R_PHASER_OUT_C_RDCLK_FIFO",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_UPPER_B",
        "CMT_PHASER_IN_D_ICLKDIV",
        "CMT_R_PHASER_IN_D_WRCLK_TOFIFO",
    );
    vrf.mark_merge_pip(
        "CMT_TOP_L_UPPER_B",
        "CMT_PHASER_OUT_D_OCLKDIV",
        "CMT_R_PHASER_OUT_D_RDCLK_TOFIFO",
    );

    for tkn in ["CMT_TOP_L_UPPER_T", "CMT_TOP_R_UPPER_T"] {
        for i in 0..13 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_ICLKDIV_{i}"),
                "CMT_PHASER_D_ICLKDIV_TOIOI",
            );
            vrf.mark_merge_pip(tkn, &format!("CMT_TOP_ICLK_{i}"), "CMT_PHASER_D_ICLK_TOIOI");
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_TOP_OCLKDIV_{i}"),
                "CMT_PHASER_D_OCLKDIV_TOIOI",
            );
            vrf.mark_merge_pip(tkn, &format!("CMT_TOP_OCLK_{i}"), "CMT_PHASER_D_OCLK_TOIOI");
        }
        vrf.mark_merge_pip(tkn, "CMT_PHASER_D_ICLK_TOIOI", "CMT_PLL_PHASER_IN_D_ICLK");
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_D_ICLKDIV_TOIOI",
            "CMT_PLL_PHASER_IN_D_ICLKDIV",
        );
        vrf.mark_merge_pip(tkn, "CMT_PHASER_D_OCLK_TOIOI", "CMT_PLL_PHASER_OUT_D_OCLK");
        vrf.mark_merge_pip(
            tkn,
            "CMT_PHASER_D_OCLKDIV_TOIOI",
            "CMT_PLL_PHASER_OUT_D_OCLKDIV",
        );
    }

    for &tcrd in &endev.edev.tile_index[tcls::CMT] {
        // insert invisible buffers
        let BelInfo::SwitchBox(ref sb) = endev.edev.db[tcls::CMT].bels[bslots::SPEC_INT] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::ProgBuf(buf) = item else {
                continue;
            };
            if !wires::HROW_I_CMT.contains(buf.dst.wire)
                && !wires::CCIO_CMT.contains(buf.dst.wire)
                && !wires::HCLK_CMT.contains(buf.dst.wire)
                && !wires::RCLK_CMT.contains(buf.dst.wire)
                && !wires::PERF_IN_PLL.contains(buf.dst.wire)
                && !wires::PERF_IN_PHASER.contains(buf.dst.wire)
            {
                continue;
            }
            vrf.skip_tcls_pip(tcls::CMT, buf.dst, buf.src.tw);
            let wt = endev.edev.resolve_tile_wire(tcrd, buf.dst).unwrap();
            let wf = endev.edev.resolve_tile_wire(tcrd, buf.src.tw).unwrap();
            vrf.alias_wire(wt, wf);
        }
        // fix up OMUX_CCIO
        for i in 0..4 {
            vrf.skip_tcls_pip(
                tcls::CMT,
                wires::OMUX_CCIO[i].cell(25),
                wires::CCIO_CMT[i].cell(25),
            );
        }
        for i in 0..14 {
            for j in 0..4 {
                vrf.inject_tcls_pip(
                    tcls::CMT,
                    wires::HROW_O[i].cell(25),
                    wires::CCIO_CMT[j].cell(25),
                );
            }
        }
        for wt in [
            wires::IMUX_PLL_CLKIN1_HCLK,
            wires::IMUX_PLL_CLKIN2_HCLK,
            wires::IMUX_PLL_CLKFB_HCLK,
        ]
        .into_iter()
        .flatten()
        {
            for j in 0..4 {
                vrf.inject_tcls_pip(tcls::CMT, wt.cell(25), wires::CCIO_CMT[j].cell(25));
            }
        }
        // fix up unconnected FREQ_BB/SYNC_BB at the edges
        if !endev.edev[tcrd.cell].conns.contains_id(cslots::IO_S) {
            for i in 0..4 {
                vrf.skip_tile_pip(
                    tcrd,
                    wires::CMT_FREQ_BB[i].cell(25),
                    wires::CMT_FREQ_BB_S[i].cell(25),
                );
                vrf.skip_tile_pip(
                    tcrd,
                    wires::CMT_FREQ_BB_S[i].cell(25),
                    wires::CMT_FREQ_BB[i].cell(25),
                );
            }
            vrf.skip_tile_pip(
                tcrd,
                wires::CMT_SYNC_BB.cell(25),
                wires::CMT_SYNC_BB_S.cell(25),
            );
            vrf.skip_tile_pip(
                tcrd,
                wires::CMT_SYNC_BB_S.cell(25),
                wires::CMT_SYNC_BB.cell(25),
            );
        }
        if let Some(conn) = endev.edev[tcrd.cell].conns.get(cslots::IO_S)
            && conn.class == ccls::IO_S_SLR
        {
            vrf.skip_tile_pip(
                tcrd,
                wires::CMT_SYNC_BB.cell(25),
                wires::CMT_SYNC_BB_S.cell(25),
            );
            vrf.skip_tile_pip(
                tcrd,
                wires::CMT_SYNC_BB_S.cell(25),
                wires::CMT_SYNC_BB.cell(25),
            );
        }
        if !endev.edev[tcrd.cell].conns.contains_id(cslots::IO_N) {
            for i in 0..4 {
                vrf.skip_tile_pip(
                    tcrd,
                    wires::CMT_FREQ_BB[i].cell(25),
                    wires::CMT_FREQ_BB_N[i].cell(25),
                );
                vrf.skip_tile_pip(
                    tcrd,
                    wires::CMT_FREQ_BB_N[i].cell(25),
                    wires::CMT_FREQ_BB[i].cell(25),
                );
            }
            vrf.skip_tile_pip(
                tcrd,
                wires::CMT_SYNC_BB.cell(25),
                wires::CMT_SYNC_BB_N.cell(25),
            );
            vrf.skip_tile_pip(
                tcrd,
                wires::CMT_SYNC_BB_N.cell(25),
                wires::CMT_SYNC_BB.cell(25),
            );
        }
        if let Some(conn) = endev.edev[tcrd.cell].conns.get(cslots::IO_N)
            && conn.class == ccls::IO_N_SLR
        {
            vrf.skip_tile_pip(
                tcrd,
                wires::CMT_SYNC_BB.cell(25),
                wires::CMT_SYNC_BB_N.cell(25),
            );
            vrf.skip_tile_pip(
                tcrd,
                wires::CMT_SYNC_BB_N.cell(25),
                wires::CMT_SYNC_BB.cell(25),
            );
        }
    }
}

fn verify_pre_io(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    for tcid in [tcls::HCLK_IO_HP, tcls::HCLK_IO_HR] {
        for &tcrd in &endev.edev.tile_index[tcid] {
            // insert invisible buffers
            for (wt, wf) in wires::PERF_IO.into_iter().zip(wires::PERF) {
                let dst = wt.cell(4);
                let src = wf.cell(4);
                vrf.skip_tcls_pip(tcid, dst, src);
                let wt = endev.edev.resolve_tile_wire(tcrd, dst).unwrap();
                let wf = endev.edev.resolve_tile_wire(tcrd, src).unwrap();
                vrf.alias_wire(wt, wf);
            }
            let crds = vrf.get_tile_crds(tcrd).unwrap();
            let crd = crds[RawTileId::from_idx(3)];
            vrf.mark_merge_single_pip(crd, "IOI_IMUX_RC0", "IOI_BYP3_0");
            vrf.mark_merge_single_pip(crd, "IOI_IMUX_RC1", "IOI_BYP4_0");
            let crd = crds[RawTileId::from_idx(2)];
            vrf.mark_merge_single_pip(crd, "IOI_IMUX_RC2", "IOI_BYP4_1");
            vrf.mark_merge_single_pip(crd, "IOI_IMUX_RC3", "IOI_BYP3_1");
        }
    }

    for (tcid, num_cells) in [
        (tcls::IO_HP_S, 1),
        (tcls::IO_HR_S, 1),
        (tcls::IO_HP_N, 1),
        (tcls::IO_HR_N, 1),
        (tcls::IO_HP_PAIR, 2),
        (tcls::IO_HR_PAIR, 2),
    ] {
        for c in 0..num_cells {
            vrf.skip_tcls_pip(
                tcid,
                wires::IMUX_IOI_OCLK[1].cell(c),
                wires::PHASER_OCLK90.cell(c),
            );
            for i in 0..2 {
                vrf.skip_tcls_pip(
                    tcid,
                    wires::IMUX_IOI_OCLKDIV[i].cell(c),
                    wires::IMUX_IOI_OCLKDIVF[i].cell(c),
                );
                vrf.inject_tcls_pip(
                    tcid,
                    wires::IMUX_IOI_OCLKDIV[i].cell(c),
                    wires::IMUX_IMUX[8].cell(c),
                );
                for j in 0..4 {
                    vrf.inject_tcls_pip(
                        tcid,
                        wires::IMUX_IOI_OCLKDIV[i].cell(c),
                        wires::RCLK_IO[j].cell(0),
                    );
                }
                for j in 0..6 {
                    vrf.inject_tcls_pip(
                        tcid,
                        wires::IMUX_IOI_OCLKDIV[i].cell(c),
                        wires::LCLK_IO[j].cell(0),
                    );
                }
            }
        }
    }
}

fn verify_pre_gtp(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    let tcid = tcls::GTP_COMMON_MID;
    for &tcrd in &endev.edev.tile_index[tcid] {
        // insert invisible buffers
        for i in 0..14 {
            let dst = wires::HROW_I_GTP[i].cell(3);
            let src = wires::HROW_I[i].cell(3);
            vrf.skip_tcls_pip(tcid, dst, src);
            let wt = endev.edev.resolve_tile_wire(tcrd, dst).unwrap();
            let wf = endev.edev.resolve_tile_wire(tcrd, src).unwrap();
            vrf.alias_wire(wt, wf);
        }
    }
}

fn verify_pre(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    verify_pre_hclk(vrf);
    verify_pre_clk_bufg(vrf);
    verify_pre_clk_hrow(vrf);
    verify_pre_ps(vrf);
    verify_pre_cmt(endev, vrf);
    verify_pre_io(endev, vrf);
    verify_pre_gtp(endev, vrf);
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
