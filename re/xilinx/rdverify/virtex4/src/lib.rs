use prjcombine_entity::EntityId;
use prjcombine_interconnect::{db::BelInfo, grid::RowId};
use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{LegacyBelContext, SitePinDir, Verifier};
use prjcombine_virtex4::{defs, defs::virtex4::wires};

fn verify_slice(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let idx = defs::bslots::SLICE.index_of(bel.slot).unwrap();
    let kind = if matches!(idx, 0 | 2) {
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
    for pin in ["F5", "FX", "COUT"] {
        vrf.claim_net(&[bel.wire(pin)]);
    }
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
        for pin in ["DIG", "BYOUT", "BYINVOUT", "SHIFTOUT"] {
            vrf.claim_net(&[bel.wire(pin)]);
        }
    }
    vrf.verify_legacy_bel(bel, kind, &pins, &[]);
    for (dbel, dpin, sbel, spin) in [
        (
            defs::bslots::SLICE[0],
            "FXINA",
            defs::bslots::SLICE[0],
            "F5",
        ),
        (
            defs::bslots::SLICE[0],
            "FXINB",
            defs::bslots::SLICE[2],
            "F5",
        ),
        (
            defs::bslots::SLICE[1],
            "FXINA",
            defs::bslots::SLICE[1],
            "F5",
        ),
        (
            defs::bslots::SLICE[1],
            "FXINB",
            defs::bslots::SLICE[3],
            "F5",
        ),
        (
            defs::bslots::SLICE[2],
            "FXINA",
            defs::bslots::SLICE[0],
            "FX",
        ),
        (
            defs::bslots::SLICE[2],
            "FXINB",
            defs::bslots::SLICE[1],
            "FX",
        ),
        (
            defs::bslots::SLICE[3],
            "FXINA",
            defs::bslots::SLICE[2],
            "FX",
        ),
        // SLICE3 FXINB <- top's SLICE2 FX

        // SLICE0 CIN <- bot's SLICE2 COUT
        // SLICE1 CIN <- bot's SLICE3 COUT
        (
            defs::bslots::SLICE[2],
            "CIN",
            defs::bslots::SLICE[0],
            "COUT",
        ),
        (
            defs::bslots::SLICE[3],
            "CIN",
            defs::bslots::SLICE[1],
            "COUT",
        ),
        (
            defs::bslots::SLICE[0],
            "SHIFTIN",
            defs::bslots::SLICE[2],
            "SHIFTOUT",
        ),
        // SLICE2 SHIFTIN disconnected?
        (
            defs::bslots::SLICE[0],
            "ALTDIG",
            defs::bslots::SLICE[2],
            "DIG",
        ),
        // SLICE2 ALTDIG disconnected?
        (
            defs::bslots::SLICE[0],
            "SLICEWE1",
            defs::bslots::SLICE[0],
            "BYOUT",
        ),
        (
            defs::bslots::SLICE[2],
            "SLICEWE1",
            defs::bslots::SLICE[0],
            "BYINVOUT",
        ),
    ] {
        if dbel != bel.slot {
            continue;
        }
        let obel = vrf.find_bel_sibling(bel, sbel);
        vrf.claim_pip(bel.wire(dpin), obel.wire(spin));
        vrf.claim_net(&[bel.wire(dpin)]);
    }
    if bel.slot == defs::bslots::SLICE[2] {
        vrf.claim_net(&[bel.wire("SHIFTIN")]);
        vrf.claim_net(&[bel.wire("ALTDIG")]);
    }
    if bel.slot == defs::bslots::SLICE[3] {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 1, defs::bslots::SLICE[2]) {
            vrf.claim_net(&[bel.wire("FXINB"), obel.wire("FX_S")]);
            vrf.claim_pip(obel.wire("FX_S"), obel.wire("FX"));
        } else {
            vrf.claim_net(&[bel.wire("FXINB")]);
        }
    }
    for (dbel, sbel) in [
        (defs::bslots::SLICE[0], defs::bslots::SLICE[2]),
        (defs::bslots::SLICE[1], defs::bslots::SLICE[3]),
    ] {
        if bel.slot != dbel {
            continue;
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, sbel) {
            vrf.claim_net(&[bel.wire("CIN"), obel.wire("COUT_N")]);
            vrf.claim_pip(obel.wire("COUT_N"), obel.wire("COUT"));
        } else {
            vrf.claim_net(&[bel.wire("CIN")]);
        }
    }
}

fn verify_bram(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(
        bel,
        "RAMB16",
        &[
            ("CASCADEINA", SitePinDir::In),
            ("CASCADEINB", SitePinDir::In),
            ("CASCADEOUTA", SitePinDir::Out),
            ("CASCADEOUTB", SitePinDir::Out),
        ],
        &[],
    );
    for (ipin, opin) in [("CASCADEINA", "CASCADEOUTA"), ("CASCADEINB", "CASCADEOUTB")] {
        vrf.claim_net(&[bel.wire(opin)]);
        vrf.claim_net(&[bel.wire(ipin)]);
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -4, bel.slot) {
            vrf.verify_net(&[bel.wire_far(ipin), obel.wire(opin)]);
            vrf.claim_pip(bel.wire(ipin), bel.wire_far(ipin));
        }
    }
}

fn verify_dsp(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let mut pairs = vec![];
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
        if bel.slot == defs::bslots::DSP[0] {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -4, defs::bslots::DSP[1]) {
                vrf.claim_net(&[bel.wire(ipin), obel.wire_far(opin)]);
                vrf.claim_pip(obel.wire_far(opin), obel.wire(opin));
            } else {
                vrf.claim_net(&[bel.wire(ipin)]);
            }
        } else {
            vrf.claim_net(&[bel.wire(ipin)]);
            let obel = vrf.find_bel_sibling(bel, defs::bslots::DSP[0]);
            vrf.claim_pip(bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_legacy_bel(bel, "DSP48", &pins, &[]);
}

fn verify_ppc(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let mut dcr_pins = vec![
        ("EMACDCRACK".to_string(), SitePinDir::In),
        ("DCREMACCLK".to_string(), SitePinDir::Out),
        ("DCREMACREAD".to_string(), SitePinDir::Out),
        ("DCREMACWRITE".to_string(), SitePinDir::Out),
    ];
    for i in 0..32 {
        dcr_pins.push((format!("EMACDCRDBUS{i}"), SitePinDir::In));
        dcr_pins.push((format!("DCREMACDBUS{i}"), SitePinDir::Out));
    }
    for i in 8..10 {
        dcr_pins.push((format!("DCREMACABUS{i}"), SitePinDir::Out));
    }
    let pins: Vec<_> = dcr_pins
        .iter()
        .map(|&(ref pin, dir)| (&pin[..], dir))
        .collect();
    vrf.verify_legacy_bel(bel, "PPC405_ADV", &pins, &[]);
    let obel = vrf.find_bel_sibling(bel, defs::bslots::EMAC);
    for (pin, dir) in dcr_pins {
        vrf.claim_net(&[bel.wire(&pin)]);
        match dir {
            SitePinDir::In => vrf.claim_pip(bel.wire(&pin), obel.wire(&pin)),
            SitePinDir::Out => vrf.claim_pip(obel.wire(&pin), bel.wire(&pin)),
            _ => unreachable!(),
        }
    }
    // detritus.
    vrf.claim_pip_tri(
        bel.crds[EntityId::from_idx(0)],
        "PB_OMUX_S0_B5",
        "PB_OMUX15_B5",
    );
    vrf.claim_pip_tri(
        bel.crds[EntityId::from_idx(0)],
        "PB_OMUX_S0_B6",
        "PB_OMUX15_B6",
    );
    vrf.claim_pip_tri(
        bel.crds[EntityId::from_idx(1)],
        "PT_OMUX_N15_T5",
        "PT_OMUX0_T5",
    );
    vrf.claim_pip_tri(
        bel.crds[EntityId::from_idx(1)],
        "PT_OMUX_N15_T6",
        "PT_OMUX0_T6",
    );
}

fn verify_emac(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let mut dcr_pins = vec![
        ("EMACDCRACK".to_string(), SitePinDir::Out),
        ("DCREMACCLK".to_string(), SitePinDir::In),
        ("DCREMACREAD".to_string(), SitePinDir::In),
        ("DCREMACWRITE".to_string(), SitePinDir::In),
    ];
    for i in 0..32 {
        dcr_pins.push((format!("EMACDCRDBUS{i}"), SitePinDir::Out));
        dcr_pins.push((format!("DCREMACDBUS{i}"), SitePinDir::In));
    }
    for i in 8..10 {
        dcr_pins.push((format!("DCREMACABUS{i}"), SitePinDir::In));
    }
    let pins: Vec<_> = dcr_pins
        .iter()
        .map(|&(ref pin, dir)| (&pin[..], dir))
        .collect();
    vrf.verify_legacy_bel(bel, "EMAC", &pins, &[]);
    for (pin, _) in dcr_pins {
        vrf.claim_net(&[bel.wire(&pin)]);
    }
}

fn verify_bufgctrl(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(
        bel,
        "BUFGCTRL",
        &[
            ("I0", SitePinDir::In),
            ("I1", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &["I0MUX", "I1MUX", "CKINT0", "CKINT1"],
    );
    let idx = defs::bslots::BUFGCTRL.index_of(bel.slot).unwrap();
    let is_b = idx < 16;
    vrf.claim_net(&[bel.wire("I0")]);
    vrf.claim_net(&[bel.wire("I1")]);
    vrf.claim_pip(bel.wire("I0"), bel.wire("I0MUX"));
    vrf.claim_pip(bel.wire("I1"), bel.wire("I1MUX"));
    vrf.claim_pip(bel.wire("I0MUX"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.wire("I0MUX"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.wire("I1MUX"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.wire("I1MUX"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.wire("I0MUX"), bel.wire("MUXBUS0"));
    vrf.claim_pip(bel.wire("I1MUX"), bel.wire("MUXBUS1"));
    for i in 0..16 {
        let obid = if is_b {
            defs::bslots::BUFGCTRL[i]
        } else {
            defs::bslots::BUFGCTRL[i + 16]
        };
        let obel = vrf.find_bel_sibling(bel, obid);
        vrf.claim_pip(bel.wire("I0MUX"), obel.wire("GFB"));
        vrf.claim_pip(bel.wire("I1MUX"), obel.wire("GFB"));
    }
    let obel = vrf.find_bel_sibling(
        bel,
        if is_b {
            defs::bslots::BUFG_MGTCLK_S
        } else {
            defs::bslots::BUFG_MGTCLK_N
        },
    );
    for pin in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
        vrf.claim_pip(bel.wire("I0MUX"), obel.wire(pin));
        vrf.claim_pip(bel.wire("I1MUX"), obel.wire(pin));
    }
    vrf.claim_net(&[bel.wire("O")]);
    vrf.claim_net(&[bel.wire("GCLK")]);
    vrf.claim_net(&[bel.wire("GFB")]);
    vrf.claim_pip(bel.wire("GCLK"), bel.wire("O"));
    vrf.claim_pip(bel.wire("GFB"), bel.wire("O"));
    let srow = if is_b {
        endev.edev.row_dcmiob.unwrap()
    } else {
        endev.edev.row_iobdcm.unwrap() - 16
    };
    let obel = vrf.get_legacy_bel(bel.cell.with_row(srow).bel(defs::bslots::CLK_IOB));
    let idx0 = (idx % 16) * 2;
    let idx1 = (idx % 16) * 2 + 1;
    vrf.verify_net(&[bel.wire("MUXBUS0"), obel.wire(&format!("MUXBUS_O{idx0}"))]);
    vrf.verify_net(&[bel.wire("MUXBUS1"), obel.wire(&format!("MUXBUS_O{idx1}"))]);
}

fn verify_bufg_mgtclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    if endev.edev.col_lgt.is_some() {
        let obel = vrf.find_bel_sibling(
            bel,
            match bel.slot {
                defs::bslots::BUFG_MGTCLK_S => defs::bslots::BUFG_MGTCLK_S_HROW,
                defs::bslots::BUFG_MGTCLK_N => defs::bslots::BUFG_MGTCLK_N_HROW,
                _ => unreachable!(),
            },
        );
        for (pin, pin_o) in [
            ("MGT_L0", "MGT_L0_O"),
            ("MGT_L1", "MGT_L1_O"),
            ("MGT_R0", "MGT_R0_O"),
            ("MGT_R1", "MGT_R1_O"),
        ] {
            vrf.verify_net(&[bel.wire(pin), obel.wire(pin_o)]);
        }
    } else {
        for pin in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
            vrf.claim_net(&[bel.wire(pin)]);
        }
    }
}

fn verify_bufg_mgtclk_hrow(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    bel: &LegacyBelContext<'_>,
) {
    if endev.edev.col_lgt.is_some() {
        let obel = vrf.find_bel_sibling(
            bel,
            match bel.slot {
                defs::bslots::BUFG_MGTCLK_S_HROW => defs::bslots::BUFG_MGTCLK_S_HCLK,
                defs::bslots::BUFG_MGTCLK_N_HROW => defs::bslots::BUFG_MGTCLK_N_HCLK,
                _ => unreachable!(),
            },
        );
        for (pin_i, pin_o) in [
            ("MGT_L0_I", "MGT_L0_O"),
            ("MGT_L1_I", "MGT_L1_O"),
            ("MGT_R0_I", "MGT_R0_O"),
            ("MGT_R1_I", "MGT_R1_O"),
        ] {
            vrf.verify_net(&[bel.wire(pin_i), obel.wire(pin_o)]);
            vrf.claim_net(&[bel.wire(pin_o)]);
            vrf.claim_pip(bel.wire(pin_o), bel.wire(pin_i));
        }
    }
}

fn verify_bufg_mgtclk_hclk(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    bel: &LegacyBelContext<'_>,
) {
    if let Some(col_lgt) = endev.edev.col_lgt {
        for (pin_i, pin_o) in [
            ("MGT_L0_I", "MGT_L0_O"),
            ("MGT_L1_I", "MGT_L1_O"),
            ("MGT_R0_I", "MGT_R0_O"),
            ("MGT_R1_I", "MGT_R1_O"),
        ] {
            vrf.claim_net(&[bel.wire(pin_o)]);
            vrf.claim_pip(bel.wire(pin_o), bel.wire(pin_i));
        }
        let srow: RowId = match bel.slot {
            defs::bslots::BUFG_MGTCLK_S_HCLK => bel.row - 8,
            defs::bslots::BUFG_MGTCLK_N_HCLK => bel.row + 8,
            _ => unreachable!(),
        };
        let (srow, oslot) = match srow.to_idx() % 32 {
            0 => (srow, defs::bslots::GT11[0]),
            16 => (srow - 16, defs::bslots::GT11[1]),
            _ => unreachable!(),
        };
        let obel = vrf.get_legacy_bel(bel.cell.with_cr(col_lgt, srow).bel(oslot));
        vrf.verify_net(&[bel.wire("MGT_L0_I"), obel.wire("MGT0")]);
        vrf.verify_net(&[bel.wire("MGT_L1_I"), obel.wire("MGT1")]);
        let obel = vrf.get_legacy_bel(
            bel.cell
                .with_cr(endev.edev.col_rgt.unwrap(), srow)
                .bel(oslot),
        );
        vrf.verify_net(&[bel.wire("MGT_R0_I"), obel.wire("MGT0")]);
        vrf.verify_net(&[bel.wire("MGT_R1_I"), obel.wire("MGT1")]);
    }
}

fn verify_jtagppc(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(bel, "JTAGPPC", &[("TDOTSPPC", SitePinDir::In)], &[]);
    vrf.claim_net(&[bel.wire("TDOTSPPC")]);
}

fn verify_clk_hrow(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    for i in 0..8 {
        vrf.claim_net(&[bel.wire(&format!("HCLK_L{i}"))]);
        vrf.claim_net(&[bel.wire(&format!("HCLK_R{i}"))]);
        for j in 0..32 {
            vrf.claim_pip(
                bel.wire(&format!("HCLK_L{i}")),
                bel.wire(&format!("GCLK{j}")),
            );
            vrf.claim_pip(
                bel.wire(&format!("HCLK_R{i}")),
                bel.wire(&format!("GCLK{j}")),
            );
        }
    }
    for i in 0..32 {
        let orow = endev.edev.chips[bel.die].row_bufg() - 8;
        let obel = vrf.get_legacy_bel(bel.cell.with_row(orow).bel(defs::bslots::BUFGCTRL[i]));
        vrf.verify_net(&[bel.wire(&format!("GCLK{i}")), obel.wire("GCLK")]);
    }
}

fn verify_clk_iob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    for i in 0..16 {
        vrf.claim_net(&[bel.wire(&format!("PAD_BUF{i}"))]);
        vrf.claim_net(&[bel.wire(&format!("GIOB{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("PAD_BUF{i}")),
            bel.wire(&format!("PAD{i}")),
        );
        vrf.claim_pip(
            bel.wire(&format!("GIOB{i}")),
            bel.wire(&format!("PAD_BUF{i}")),
        );
        let obel = vrf
            .find_bel_delta(bel, 0, i, defs::bslots::ILOGIC[1])
            .unwrap();
        vrf.verify_net(&[bel.wire(&format!("PAD{i}")), obel.wire("CLKOUT")]);
        // avoid double-claim for IOBs that are also BUFIO inps
        if !matches!(obel.row.to_idx() % 16, 7 | 8) {
            vrf.claim_net(&[obel.wire("CLKOUT")]);
            vrf.claim_pip(obel.wire("CLKOUT"), obel.wire("O"));
        }
    }
    let dy = if bel.row < endev.edev.chips[bel.die].row_bufg() {
        -8
    } else {
        16
    };
    let obel = vrf
        .find_bel_delta(bel, 0, dy, defs::bslots::CLK_DCM)
        .unwrap();
    for i in 0..32 {
        vrf.claim_net(&[bel.wire(&format!("MUXBUS_O{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("MUXBUS_O{i}")),
            bel.wire(&format!("MUXBUS_I{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("MUXBUS_I{i}")),
            obel.wire(&format!("MUXBUS_O{i}")),
        ]);
        for j in 0..16 {
            vrf.claim_pip(
                bel.wire(&format!("MUXBUS_O{i}")),
                bel.wire(&format!("PAD_BUF{j}")),
            );
        }
    }
}

fn verify_clk_dcm(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    for i in 0..2 {
        let obel = vrf
            .find_bel(bel.cell.with_row(bel.row + i * 4).bel(defs::bslots::DCM[0]))
            .or_else(|| vrf.find_bel(bel.cell.with_row(bel.row + i * 4).bel(defs::bslots::CCM)))
            .unwrap();
        for j in 0..12 {
            vrf.claim_net(&[bel.wire(&format!("DCM{k}", k = j + i * 12))]);
            vrf.claim_pip(
                bel.wire(&format!("DCM{k}", k = j + i * 12)),
                bel.wire(&format!("DCM{i}_{j}")),
            );
            vrf.verify_net(&[
                bel.wire(&format!("DCM{i}_{j}")),
                obel.wire(&format!("TO_BUFG{j}")),
            ]);
        }
    }
    let dy = if bel.row < endev.edev.chips[bel.die].row_bufg() {
        -8
    } else {
        8
    };
    let obel = vrf.find_bel_delta(bel, 0, dy, defs::bslots::CLK_DCM);
    for i in 0..32 {
        vrf.claim_net(&[bel.wire(&format!("MUXBUS_O{i}"))]);
        if let Some(ref obel) = obel {
            vrf.claim_pip(
                bel.wire(&format!("MUXBUS_O{i}")),
                bel.wire(&format!("MUXBUS_I{i}")),
            );
            vrf.verify_net(&[
                bel.wire(&format!("MUXBUS_I{i}")),
                obel.wire(&format!("MUXBUS_O{i}")),
            ]);
        }
        for j in 0..24 {
            vrf.claim_pip(
                bel.wire(&format!("MUXBUS_O{i}")),
                bel.wire(&format!("DCM{j}")),
            );
        }
    }
}

fn verify_bufr(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(
        bel,
        "BUFR",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.wire("I")]);
    vrf.claim_net(&[bel.wire("O")]);
    let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFIO[0]);
    vrf.claim_pip(bel.wire("I"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFIO[1]);
    vrf.claim_pip(bel.wire("I"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::RCLK);
    vrf.claim_pip(bel.wire("I"), obel.wire("CKINT0"));
    vrf.claim_pip(bel.wire("I"), obel.wire("CKINT1"));
}

fn verify_bufio(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(
        bel,
        "BUFIO",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.wire("I")]);
    vrf.claim_net(&[bel.wire("O")]);
    let dy = match defs::bslots::BUFIO.index_of(bel.slot).unwrap() {
        0 => 0,
        1 => -1,
        _ => unreachable!(),
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, defs::bslots::ILOGIC[1]) {
        vrf.claim_pip(bel.wire("I"), bel.wire("PAD"));
        vrf.claim_net(&[bel.wire("PAD"), obel.wire("CLKOUT")]);
        vrf.claim_pip(obel.wire("CLKOUT"), obel.wire("O"));
    }
}

fn verify_idelayctrl(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(bel, "IDELAYCTRL", &[("REFCLK", SitePinDir::In)], &[]);
    vrf.claim_net(&[bel.wire("REFCLK")]);
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IOCLK);
    for i in 0..8 {
        vrf.claim_pip(bel.wire("REFCLK"), obel.wire(&format!("HCLK_O{i}")));
    }
}

fn verify_rclk(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.claim_net(&[bel.wire("VRCLK0")]);
    vrf.claim_net(&[bel.wire("VRCLK1")]);
    let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFR[0]);
    vrf.claim_pip(bel.wire("VRCLK0"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFR[1]);
    vrf.claim_pip(bel.wire("VRCLK1"), obel.wire("O"));

    let obel_s = vrf.find_bel_delta(bel, 0, 16, defs::bslots::RCLK);
    let obel_n = vrf.find_bel_delta(bel, 0, -16, defs::bslots::RCLK);
    if let Some(ref obel) = obel_s {
        vrf.verify_net(&[bel.wire("VRCLK_S0"), obel.wire("VRCLK0")]);
        vrf.verify_net(&[bel.wire("VRCLK_S1"), obel.wire("VRCLK1")]);
    } else {
        vrf.claim_net(&[bel.wire("VRCLK_S0")]);
        vrf.claim_net(&[bel.wire("VRCLK_S1")]);
    }
    if let Some(ref obel) = obel_n {
        vrf.verify_net(&[bel.wire("VRCLK_N0"), obel.wire("VRCLK0")]);
        vrf.verify_net(&[bel.wire("VRCLK_N1"), obel.wire("VRCLK1")]);
    } else {
        vrf.claim_net(&[bel.wire("VRCLK_N0")]);
        vrf.claim_net(&[bel.wire("VRCLK_N1")]);
    }
    for opin in ["RCLK0", "RCLK1"] {
        vrf.claim_net(&[bel.wire(opin)]);
        vrf.claim_pip(bel.wire(opin), bel.wire("VRCLK0"));
        vrf.claim_pip(bel.wire(opin), bel.wire("VRCLK1"));
        vrf.claim_pip(bel.wire(opin), bel.wire("VRCLK_S0"));
        vrf.claim_pip(bel.wire(opin), bel.wire("VRCLK_S1"));
        vrf.claim_pip(bel.wire(opin), bel.wire("VRCLK_N0"));
        vrf.claim_pip(bel.wire(opin), bel.wire("VRCLK_N1"));
    }
}

fn verify_ioclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let obel = vrf.get_legacy_bel(
        bel.cell
            .with_col(endev.edev.col_cfg)
            .bel(defs::bslots::CLK_HROW),
    );
    let lr = if bel.col <= endev.edev.col_cfg {
        'L'
    } else {
        'R'
    };
    for i in 0..8 {
        vrf.claim_net(&[bel.wire(&format!("HCLK_O{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("HCLK_O{i}")),
            bel.wire(&format!("HCLK_I{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("HCLK_I{i}")),
            obel.wire(&format!("HCLK_{lr}{i}")),
        ]);
    }

    let scol = if bel.col <= endev.edev.col_cfg {
        endev.edev.col_lio.unwrap()
    } else {
        endev.edev.col_rio.unwrap()
    };
    let obel = vrf.get_legacy_bel(bel.cell.with_col(scol).bel(defs::bslots::RCLK));
    for i in 0..2 {
        vrf.claim_net(&[bel.wire(&format!("RCLK_O{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("RCLK_O{i}")),
            bel.wire(&format!("RCLK_I{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("RCLK_I{i}")),
            obel.wire(&format!("RCLK{i}")),
        ]);
    }

    vrf.claim_net(&[bel.wire("VIOCLK0")]);
    vrf.claim_net(&[bel.wire("VIOCLK1")]);
    let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFIO[0]);
    vrf.claim_pip(bel.wire("VIOCLK0"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFIO[1]);
    vrf.claim_pip(bel.wire("VIOCLK1"), obel.wire("O"));

    vrf.claim_pip(bel.wire("IOCLK0"), bel.wire("VIOCLK0"));
    vrf.claim_pip(bel.wire("IOCLK1"), bel.wire("VIOCLK1"));

    let mut claim_s = bel.col != endev.edev.col_cfg;
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 16, defs::bslots::IOCLK)
        && vrf
            .find_bel_delta(bel, 0, 0, defs::bslots::STARTUP)
            .is_none()
    {
        vrf.verify_net(&[bel.wire("VIOCLK_S0"), obel.wire("VIOCLK0")]);
        vrf.verify_net(&[bel.wire("VIOCLK_S1"), obel.wire("VIOCLK1")]);
        vrf.claim_pip(bel.wire("IOCLK_S0"), bel.wire("VIOCLK_S0"));
        vrf.claim_pip(bel.wire("IOCLK_S1"), bel.wire("VIOCLK_S1"));
        claim_s = true;
    }
    let mut claim_n = bel.col != endev.edev.col_cfg;
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -16, defs::bslots::IOCLK)
        && vrf
            .find_bel_delta(bel, 0, -16, defs::bslots::STARTUP)
            .is_none()
    {
        vrf.verify_net(&[bel.wire("VIOCLK_N0"), obel.wire("VIOCLK0")]);
        vrf.verify_net(&[bel.wire("VIOCLK_N1"), obel.wire("VIOCLK1")]);
        vrf.claim_pip(bel.wire("IOCLK_N0"), bel.wire("VIOCLK_N0"));
        vrf.claim_pip(bel.wire("IOCLK_N1"), bel.wire("VIOCLK_N1"));
        claim_n = true;
    }
    let mut wires0 = vec![bel.wire("IOCLK0")];
    let mut wires1 = vec![bel.wire("IOCLK1")];
    let mut wires_s0 = vec![];
    let mut wires_s1 = vec![];
    let mut wires_n0 = vec![];
    let mut wires_n1 = vec![];
    if claim_s {
        wires_s0.push(bel.wire("IOCLK_S0"));
        wires_s1.push(bel.wire("IOCLK_S1"));
    }
    if claim_n {
        wires_n0.push(bel.wire("IOCLK_N0"));
        wires_n1.push(bel.wire("IOCLK_N1"));
    }
    for i in 0..16 {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, i - 8, defs::bslots::IOI) {
            wires0.push(obel.wire("IOCLK0"));
            wires1.push(obel.wire("IOCLK1"));
            wires_s0.push(obel.wire("IOCLK_S0"));
            wires_s1.push(obel.wire("IOCLK_S1"));
            wires_n0.push(obel.wire("IOCLK_N0"));
            wires_n1.push(obel.wire("IOCLK_N1"));
        }
    }
    vrf.claim_net(&wires0);
    vrf.claim_net(&wires1);
    vrf.claim_net(&wires_s0);
    vrf.claim_net(&wires_s1);
    vrf.claim_net(&wires_n0);
    vrf.claim_net(&wires_n1);
}

fn verify_hclk_dcm(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, defs::bslots::HCLK_DCM_HROW);
    for i in 0..16 {
        vrf.verify_net(&[
            bel.wire(&format!("GIOB_I{i}")),
            obel.wire(&format!("GIOB_O{i}")),
        ]);
        if bel.slot != defs::bslots::HCLK_DCM_S {
            vrf.claim_net(&[bel.wire(&format!("GIOB_O_U{i}"))]);
            vrf.claim_pip(
                bel.wire(&format!("GIOB_O_U{i}")),
                bel.wire(&format!("GIOB_I{i}")),
            );
        }
        if bel.slot != defs::bslots::HCLK_DCM_N {
            vrf.claim_net(&[bel.wire(&format!("GIOB_O_D{i}"))]);
            vrf.claim_pip(
                bel.wire(&format!("GIOB_O_D{i}")),
                bel.wire(&format!("GIOB_I{i}")),
            );
        }
    }
    let has_sysmon_s = vrf
        .find_bel_delta(bel, 0, -8, defs::bslots::SYSMON)
        .is_some();
    let has_sysmon_n = vrf
        .find_bel_delta(bel, 0, 0, defs::bslots::SYSMON)
        .is_some();
    let obel = vrf.find_bel_sibling(bel, defs::bslots::CLK_HROW);
    for i in 0..8 {
        vrf.verify_net(&[
            bel.wire(&format!("HCLK_I{i}")),
            obel.wire(&format!("HCLK_L{i}")),
        ]);
        if bel.slot != defs::bslots::HCLK_DCM_S && !has_sysmon_n {
            vrf.claim_net(&[bel.wire(&format!("HCLK_O_U{i}"))]);
            vrf.claim_pip(
                bel.wire(&format!("HCLK_O_U{i}")),
                bel.wire(&format!("HCLK_I{i}")),
            );
        }
        if bel.slot != defs::bslots::HCLK_DCM_N && !has_sysmon_s {
            vrf.claim_net(&[bel.wire(&format!("HCLK_O_D{i}"))]);
            vrf.claim_pip(
                bel.wire(&format!("HCLK_O_D{i}")),
                bel.wire(&format!("HCLK_I{i}")),
            );
        }
    }
    let mut wires_s = [vec![], vec![], vec![], vec![]];
    let mut wires_n = [vec![], vec![], vec![], vec![]];
    for dy in [-8, -4] {
        if let Some(obel) = vrf
            .find_bel_delta(bel, 0, dy, defs::bslots::DCM[0])
            .or_else(|| vrf.find_bel_delta(bel, 0, dy, defs::bslots::CCM))
        {
            for i in 0..4 {
                wires_s[i].push(obel.wire(&format!("MGT{i}")));
            }
        }
    }
    for dy in [0, 4] {
        if let Some(obel) = vrf
            .find_bel_delta(bel, 0, dy, defs::bslots::DCM[0])
            .or_else(|| vrf.find_bel_delta(bel, 0, dy, defs::bslots::CCM))
        {
            for i in 0..4 {
                wires_n[i].push(obel.wire(&format!("MGT{i}")));
            }
        }
    }
    match bel.slot {
        defs::bslots::HCLK_DCM => {
            for i in 0..4 {
                if endev.edev.col_lgt.is_some() || !has_sysmon_s {
                    let skip = endev.edev.col_lgt.is_none()
                        && bel.row.to_idx() == endev.edev.chips[bel.die].regs * 16 - 8;
                    if !skip {
                        vrf.claim_net(&[bel.wire(&format!("MGT{i}"))]);
                    }
                    if endev.edev.col_lgt.is_some() {
                        vrf.claim_pip(bel.wire(&format!("MGT{i}")), bel.wire(&format!("MGT_I{i}")));
                    }
                    if !has_sysmon_s {
                        wires_s[i].push(bel.wire(&format!("MGT_O_D{i}")));
                        if !skip {
                            vrf.claim_pip(
                                bel.wire(&format!("MGT_O_D{i}")),
                                bel.wire(&format!("MGT{i}")),
                            );
                        }
                    }
                    if !has_sysmon_n {
                        wires_n[i].push(bel.wire(&format!("MGT_O_U{i}")));
                        if !skip {
                            vrf.claim_pip(
                                bel.wire(&format!("MGT_O_U{i}")),
                                bel.wire(&format!("MGT{i}")),
                            );
                        }
                    }
                }
            }
        }
        defs::bslots::HCLK_DCM_S => {
            if endev.edev.col_lgt.is_some() {
                for i in 0..4 {
                    wires_s[i].push(bel.wire(&format!("MGT_O_D{i}")));
                    vrf.claim_pip(
                        bel.wire(&format!("MGT_O_D{i}")),
                        bel.wire(&format!("MGT_I{i}")),
                    );
                }
            }
        }
        defs::bslots::HCLK_DCM_N => {
            if endev.edev.col_lgt.is_some() {
                for i in 0..4 {
                    wires_n[i].push(bel.wire(&format!("MGT_O_U{i}")));
                    vrf.claim_pip(
                        bel.wire(&format!("MGT_O_U{i}")),
                        bel.wire(&format!("MGT_I{i}")),
                    );
                }
            }
        }
        _ => unreachable!(),
    }
    for i in 0..4 {
        vrf.claim_net(&wires_s[i]);
        vrf.claim_net(&wires_n[i]);
    }
    if let Some(col_lgt) = endev.edev.col_lgt {
        let (srow, oslot) = match bel.row.to_idx() % 32 {
            8 => (bel.row - 8, defs::bslots::GT11[0]),
            24 => (bel.row - 24, defs::bslots::GT11[1]),
            _ => unreachable!(),
        };
        let obel = vrf.get_legacy_bel(bel.cell.with_cr(col_lgt, srow).bel(oslot));
        vrf.verify_net(&[bel.wire("MGT_I0"), obel.wire("MGT0")]);
        vrf.verify_net(&[bel.wire("MGT_I1"), obel.wire("MGT1")]);
        let obel = vrf.get_legacy_bel(
            bel.cell
                .with_cr(endev.edev.col_rgt.unwrap(), srow)
                .bel(oslot),
        );
        vrf.verify_net(&[bel.wire("MGT_I2"), obel.wire("MGT0")]);
        vrf.verify_net(&[bel.wire("MGT_I3"), obel.wire("MGT1")]);
    }
}

fn verify_hclk_dcm_hrow(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    bel: &LegacyBelContext<'_>,
) {
    let srow = if bel.row <= endev.edev.chips[bel.die].row_bufg() {
        endev.edev.row_dcmiob.unwrap()
    } else {
        endev.edev.row_iobdcm.unwrap() - 16
    };
    let obel = vrf.get_legacy_bel(bel.cell.with_row(srow).bel(defs::bslots::CLK_IOB));
    for i in 0..16 {
        vrf.claim_net(&[bel.wire(&format!("GIOB_O{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("GIOB_O{i}")),
            bel.wire(&format!("GIOB_I{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("GIOB_I{i}")),
            obel.wire(&format!("GIOB{i}")),
        ]);
    }
}

fn verify_hclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let obel = vrf.get_legacy_bel(
        bel.cell
            .with_col(endev.edev.col_cfg)
            .bel(defs::bslots::CLK_HROW),
    );
    let lr = if bel.col <= endev.edev.col_cfg {
        'L'
    } else {
        'R'
    };
    for i in 0..8 {
        vrf.claim_pip(
            bel.wire(&format!("HCLK_O{i}")),
            bel.wire(&format!("HCLK_I{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("HCLK_I{i}")),
            obel.wire(&format!("HCLK_{lr}{i}")),
        ]);
    }
    let scol = if bel.col <= endev.edev.col_cfg {
        endev.edev.col_lio.unwrap()
    } else {
        endev.edev.col_rio.unwrap()
    };
    let obel = vrf.get_legacy_bel(bel.cell.with_col(scol).bel(defs::bslots::RCLK));
    for i in 0..2 {
        vrf.claim_pip(
            bel.wire(&format!("RCLK_O{i}")),
            bel.wire(&format!("RCLK_I{i}")),
        );
        vrf.verify_net(&[
            bel.wire(&format!("RCLK_I{i}")),
            obel.wire(&format!("RCLK{i}")),
        ]);
    }
}

fn verify_dcm(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(
        bel,
        "DCM_ADV",
        &[("CLKIN", SitePinDir::In), ("CLKFB", SitePinDir::In)],
        &[
            "CLKIN_TEST",
            "CLKFB_TEST",
            "CKINT0",
            "CKINT1",
            "CKINT2",
            "CKINT3",
            "CLK_IN0",
        ],
    );
    vrf.claim_net(&[bel.wire("CLKIN")]);
    vrf.claim_net(&[bel.wire("CLKFB")]);
    for pin in ["CLKIN", "CLKIN_TEST", "CLKFB", "CLKFB_TEST"] {
        for ipin in [
            "CKINT0", "CKINT1", "CKINT2", "CKINT3", "BUSOUT0", "BUSOUT1", "HCLK0", "HCLK1",
            "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "GIOB0", "GIOB1", "GIOB2",
            "GIOB3", "GIOB4", "GIOB5", "GIOB6", "GIOB7", "GIOB8", "GIOB9", "GIOB10", "GIOB11",
            "GIOB12", "GIOB13", "GIOB14", "GIOB15", "MGT0", "MGT1", "MGT2", "MGT3",
        ] {
            vrf.claim_pip(bel.wire(pin), bel.wire(ipin));
        }
    }
    for i in 0..24 {
        let opin = format!("BUSOUT{i}");
        let ipin = format!("BUSIN{i}");
        vrf.claim_net(&[bel.wire(&opin)]);
        vrf.claim_pip(bel.wire(&opin), bel.wire(&ipin));
        for pin in [
            "CLK0_BUF",
            "CLK90_BUF",
            "CLK180_BUF",
            "CLK270_BUF",
            "CLK2X_BUF",
            "CLK2X180_BUF",
            "CLKDV_BUF",
            "CLKFX_BUF",
            "CLKFX180_BUF",
            "CONCUR_BUF",
            "LOCKED_BUF",
            "CLK_IN0",
        ] {
            vrf.claim_pip(bel.wire(&opin), bel.wire(pin));
        }
    }
    for (pin, bpin, opin) in [
        ("CONCUR", "CONCUR_BUF", "TO_BUFG1"),
        ("CLKFX", "CLKFX_BUF", "TO_BUFG2"),
        ("CLKFX180", "CLKFX180_BUF", "TO_BUFG3"),
        ("CLK0", "CLK0_BUF", "TO_BUFG4"),
        ("CLK180", "CLK180_BUF", "TO_BUFG5"),
        ("CLK90", "CLK90_BUF", "TO_BUFG6"),
        ("CLK270", "CLK270_BUF", "TO_BUFG7"),
        ("CLK2X180", "CLK2X180_BUF", "TO_BUFG8"),
        ("CLK2X", "CLK2X_BUF", "TO_BUFG9"),
        ("CLKDV", "CLKDV_BUF", "TO_BUFG10"),
    ] {
        vrf.claim_net(&[bel.wire(bpin)]);
        vrf.claim_net(&[bel.wire(opin)]);
        vrf.claim_pip(bel.wire(bpin), bel.wire(pin));
        vrf.claim_pip(bel.wire(opin), bel.wire(pin));
    }
    vrf.claim_net(&[bel.wire("TO_BUFG0")]);
    vrf.claim_net(&[bel.wire("TO_BUFG11")]);
    vrf.claim_net(&[bel.wire("LOCKED_BUF")]);
    vrf.claim_pip(bel.wire("LOCKED_BUF"), bel.wire("LOCKED"));
    let dy = if bel.row < endev.edev.chips[bel.die].row_bufg() {
        -4
    } else {
        4
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, defs::bslots::DCM[0]) {
        for i in 0..24 {
            let opin = format!("BUSOUT{i}");
            let ipin = format!("BUSIN{i}");
            vrf.verify_net(&[bel.wire(&ipin), obel.wire(&opin)]);
        }
    } else {
        for i in 0..24 {
            let ipin = format!("BUSIN{i}");
            vrf.claim_net(&[bel.wire(&ipin)]);
        }
    }
    let srow = RowId::from_idx(bel.row.to_idx() / 16 * 16 + 8);
    let obel = vrf
        .find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM))
        .or_else(|| vrf.find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM_S)))
        .or_else(|| vrf.find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM_N)))
        .unwrap();
    let ud = if bel.row.to_idx() % 16 < 8 { 'D' } else { 'U' };
    for i in 0..16 {
        vrf.verify_net(&[
            bel.wire(&format!("GIOB{i}")),
            obel.wire(&format!("GIOB_O_{ud}{i}")),
        ]);
    }
    for i in 0..8 {
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel.wire(&format!("HCLK_O_{ud}{i}")),
        ]);
    }
    // MGT verified in hclk
}

fn verify_pmcd(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let pins = [
        ("CLKA", SitePinDir::In),
        ("CLKB", SitePinDir::In),
        ("CLKC", SitePinDir::In),
        ("CLKD", SitePinDir::In),
        ("REL", SitePinDir::In),
        ("CLKA1", SitePinDir::Out),
        ("CLKA1D2", SitePinDir::Out),
        ("CLKA1D4", SitePinDir::Out),
        ("CLKA1D8", SitePinDir::Out),
        ("CLKB1", SitePinDir::Out),
        ("CLKC1", SitePinDir::Out),
        ("CLKD1", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(
        bel,
        "PMCD",
        &pins,
        &[
            "CLKA_TEST",
            "CLKB_TEST",
            "CLKC_TEST",
            "CLKD_TEST",
            "REL_TEST",
            "CKINTA0",
            "CKINTA1",
            "CKINTA2",
            "CKINTA3",
            "CKINTB0",
            "CKINTB1",
            "CKINTB2",
            "CKINTB3",
            "CKINTC0",
            "CKINTC1",
            "CKINTC2",
            "CKINTC3",
            "REL_INT",
        ],
    );
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, defs::bslots::CCM);
    let obel_o = vrf.find_bel_sibling(
        bel,
        defs::bslots::PMCD[defs::bslots::PMCD.index_of(bel.slot).unwrap() ^ 1],
    );
    for (opin, ab) in [
        ("CLKA", 'A'),
        ("CLKA_TEST", 'A'),
        ("CLKB", 'A'),
        ("CLKB_TEST", 'A'),
        ("CLKC", 'B'),
        ("CLKC_TEST", 'B'),
        ("CLKD", 'B'),
        ("CLKD_TEST", 'B'),
        ("REL", 'C'),
        ("REL_TEST", 'C'),
    ] {
        for i in 0..8 {
            vrf.claim_pip(bel.wire(opin), obel.wire(&format!("HCLK{i}")));
        }
        for i in 0..16 {
            vrf.claim_pip(bel.wire(opin), obel.wire(&format!("GIOB{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(opin), obel.wire(&format!("MGT{i}")));
        }
        for i in 0..24 {
            vrf.claim_pip(bel.wire(opin), obel.wire(&format!("BUSIN{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(opin), bel.wire(&format!("CKINT{ab}{i}")));
        }
        if ab != 'C' {
            vrf.claim_pip(bel.wire(opin), obel_o.wire("CLKA1D8"));
        }
    }
    vrf.claim_pip(bel.wire("REL"), bel.wire("REL_INT"));
}

fn verify_dpm(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let pins = [
        ("REFCLK", SitePinDir::In),
        ("TESTCLK1", SitePinDir::In),
        ("TESTCLK2", SitePinDir::In),
        ("REFCLKOUT", SitePinDir::Out),
        ("OSCOUT1", SitePinDir::Out),
        ("OSCOUT2", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(
        bel,
        "DPM",
        &pins,
        &[
            "REFCLK_TEST",
            "TESTCLK1_TEST",
            "TESTCLK2_TEST",
            "CKINTA0",
            "CKINTA1",
            "CKINTA2",
            "CKINTA3",
            "CKINTB0",
            "CKINTB1",
            "CKINTB2",
            "CKINTB3",
        ],
    );
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, defs::bslots::CCM);
    for (opin, ab) in [
        ("REFCLK", 'A'),
        ("REFCLK_TEST", 'A'),
        ("TESTCLK1", 'B'),
        ("TESTCLK1_TEST", 'B'),
        ("TESTCLK2", 'B'),
        ("TESTCLK2_TEST", 'B'),
    ] {
        for i in 0..8 {
            vrf.claim_pip(bel.wire(opin), obel.wire(&format!("HCLK{i}")));
        }
        for i in 0..16 {
            vrf.claim_pip(bel.wire(opin), obel.wire(&format!("GIOB{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(opin), obel.wire(&format!("MGT{i}")));
        }
        for i in 0..24 {
            vrf.claim_pip(bel.wire(opin), obel.wire(&format!("BUSIN{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.wire(opin), bel.wire(&format!("CKINT{ab}{i}")));
        }
    }
}

fn verify_ccm(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let obel_pmcd0 = vrf.find_bel_sibling(bel, defs::bslots::PMCD[0]);
    let obel_pmcd1 = vrf.find_bel_sibling(bel, defs::bslots::PMCD[1]);
    let obel_dpm = vrf.find_bel_sibling(bel, defs::bslots::DPM);
    for i in 0..12 {
        let opin = format!("TO_BUFG{i}");
        for (ibel, ipin) in [
            (&obel_pmcd0, "CLKA1"),
            (&obel_pmcd0, "CLKA1D2"),
            (&obel_pmcd0, "CLKA1D4"),
            (&obel_pmcd0, "CLKA1D8"),
            (&obel_pmcd0, "CLKB1"),
            (&obel_pmcd0, "CLKC1"),
            (&obel_pmcd0, "CLKD1"),
            (&obel_pmcd1, "CLKA1"),
            (&obel_pmcd1, "CLKA1D2"),
            (&obel_pmcd1, "CLKA1D4"),
            (&obel_pmcd1, "CLKA1D8"),
            (&obel_pmcd1, "CLKB1"),
            (&obel_pmcd1, "CLKC1"),
            (&obel_pmcd1, "CLKD1"),
            (&obel_dpm, "REFCLKOUT"),
            (&obel_dpm, "OSCOUT1"),
            (&obel_dpm, "OSCOUT2"),
            (bel, "CKINT"),
        ] {
            vrf.claim_pip(bel.wire(&opin), ibel.wire(ipin));
        }
    }
    let dy = if bel.row < endev.edev.chips[bel.die].row_bufg() {
        -4
    } else {
        4
    };
    let obel = vrf.find_bel_walk(bel, 0, dy, defs::bslots::DCM[0]).unwrap();
    for i in 0..24 {
        let opin = format!("BUSOUT{i}");
        let ipin = format!("BUSIN{i}");
        vrf.verify_net(&[bel.wire(&ipin), obel.wire(&opin)]);
    }
    let srow = RowId::from_idx(bel.row.to_idx() / 16 * 16 + 8);
    let obel = vrf
        .find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM))
        .or_else(|| vrf.find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM_S)))
        .or_else(|| vrf.find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM_N)))
        .unwrap();
    let ud = if bel.row.to_idx() % 16 < 8 { 'D' } else { 'U' };
    for i in 0..16 {
        vrf.verify_net(&[
            bel.wire(&format!("GIOB{i}")),
            obel.wire(&format!("GIOB_O_{ud}{i}")),
        ]);
    }
    for i in 0..8 {
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel.wire(&format!("HCLK_O_{ud}{i}")),
        ]);
    }
    // MGT verified in hclk
}

fn verify_sysmon(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(
        bel,
        "MONITOR",
        &[
            ("CONVST", SitePinDir::In),
            ("VP", SitePinDir::In),
            ("VP1", SitePinDir::In),
            ("VP2", SitePinDir::In),
            ("VP3", SitePinDir::In),
            ("VP4", SitePinDir::In),
            ("VP5", SitePinDir::In),
            ("VP6", SitePinDir::In),
            ("VP7", SitePinDir::In),
            ("VN", SitePinDir::In),
            ("VN1", SitePinDir::In),
            ("VN2", SitePinDir::In),
            ("VN3", SitePinDir::In),
            ("VN4", SitePinDir::In),
            ("VN5", SitePinDir::In),
            ("VN6", SitePinDir::In),
            ("VN7", SitePinDir::In),
        ],
        &["CONVST_INT_IMUX", "CONVST_INT_CLK", "CONVST_TEST"],
    );
    vrf.claim_net(&[bel.wire("CONVST")]);
    for pin in ["CONVST", "CONVST_TEST"] {
        vrf.claim_pip(bel.wire(pin), bel.wire("CONVST_INT_IMUX"));
        vrf.claim_pip(bel.wire(pin), bel.wire("CONVST_INT_CLK"));
        for i in 0..16 {
            vrf.claim_pip(bel.wire(pin), bel.wire(&format!("GIOB{i}")));
        }
    }
    let srow = RowId::from_idx(bel.row.to_idx() / 16 * 16 + 8);
    let obel = vrf
        .find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM))
        .or_else(|| vrf.find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM_S)))
        .or_else(|| vrf.find_bel(bel.cell.with_row(srow).bel(defs::bslots::HCLK_DCM_N)))
        .unwrap();
    let ud = if bel.row.to_idx() % 16 < 8 { 'D' } else { 'U' };
    for i in 0..16 {
        vrf.verify_net(&[
            bel.wire(&format!("GIOB{i}")),
            obel.wire(&format!("GIOB_O_{ud}{i}")),
        ]);
    }
    vrf.claim_net(&[bel.wire("VP")]);
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IPAD_VP);
    vrf.claim_pip(bel.wire("VP"), obel.wire("O"));
    vrf.claim_net(&[bel.wire("VN")]);
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IPAD_VN);
    vrf.claim_pip(bel.wire("VN"), obel.wire("O"));
    for i in 0..8 {
        let Some((iop, _)) = endev.edev.get_sysmon_vaux(bel.cell, i) else {
            continue;
        };
        vrf.claim_net(&[bel.wire(&format!("VP{i}"))]);
        vrf.claim_net(&[bel.wire(&format!("VN{i}"))]);
        vrf.claim_pip(bel.wire(&format!("VP{i}")), bel.wire_far(&format!("VP{i}")));
        vrf.claim_pip(bel.wire(&format!("VN{i}")), bel.wire_far(&format!("VN{i}")));
        let obel = vrf.get_legacy_bel(iop.cell.bel(defs::bslots::IOB[1]));
        vrf.claim_net(&[bel.wire_far(&format!("VP{i}")), obel.wire("MONITOR")]);
        vrf.claim_pip(obel.wire("MONITOR"), obel.wire("PADOUT"));
        let obel = vrf.get_legacy_bel(iop.cell.bel(defs::bslots::IOB[0]));
        vrf.claim_net(&[bel.wire_far(&format!("VN{i}")), obel.wire("MONITOR")]);
        vrf.claim_pip(obel.wire("MONITOR"), obel.wire("PADOUT"));
    }
}

fn verify_ipad(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_net(&[bel.wire("O")]);
}

fn verify_opad(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(bel, "OPAD", &[("I", SitePinDir::In)], &[]);
    vrf.claim_net(&[bel.wire("I")]);
}

fn verify_ilogic(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let idx = defs::bslots::ILOGIC.index_of(bel.slot).unwrap();
    let pins = [
        ("TFB", SitePinDir::In),
        ("OFB", SitePinDir::In),
        ("D", SitePinDir::In),
        ("CLK", SitePinDir::In),
        ("OCLK", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "ISERDES", &pins, &["CLKMUX", "CLKMUX_INT"]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    vrf.claim_pip(bel.wire("CLK"), bel.wire("CLKMUX"));
    vrf.claim_pip(bel.wire("CLKMUX"), bel.wire("CLKMUX_INT"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IOI);
    for pin in [
        "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "RCLK0", "RCLK1",
        "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0", "IOCLK_N1",
    ] {
        vrf.claim_pip(bel.wire("CLKMUX"), obel.wire(pin));
    }
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IOB[idx]);
    vrf.claim_pip(bel.wire("D"), obel.wire("I"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::OLOGIC[idx]);
    vrf.claim_pip(bel.wire("OCLK"), obel.wire("CLKMUX"));
    vrf.claim_pip(bel.wire("OFB"), obel.wire("OQ"));
    vrf.claim_pip(bel.wire("TFB"), obel.wire("TQ"));
    if bel.slot == defs::bslots::ILOGIC[0] {
        let obel = vrf.find_bel_sibling(bel, defs::bslots::ILOGIC[1]);
        vrf.claim_pip(bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_ologic(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let pins = [
        ("OQ", SitePinDir::Out),
        ("CLK", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "OSERDES", &pins, &["CLKMUX", "CLKMUX_INT"]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    vrf.claim_pip(bel.wire("CLK"), bel.wire("CLKMUX"));
    vrf.claim_pip(bel.wire("CLKMUX"), bel.wire("CLKMUX_INT"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IOI);
    for pin in [
        "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "RCLK0", "RCLK1",
        "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0", "IOCLK_N1",
    ] {
        vrf.claim_pip(bel.wire("CLKMUX"), obel.wire(pin));
    }
    if bel.slot == defs::bslots::OLOGIC[1] {
        let obel = vrf.find_bel_sibling(bel, defs::bslots::OLOGIC[0]);
        vrf.claim_pip(bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_iob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let idx = defs::bslots::IOB.index_of(bel.slot).unwrap();
    let kind = if bel.col == endev.edev.col_cfg || matches!(bel.row.to_idx() % 16, 7 | 8) {
        "LOWCAPIOB"
    } else if bel.slot == defs::bslots::IOB[1] {
        "IOBM"
    } else {
        "IOBS"
    };
    let pins = [
        ("I", SitePinDir::Out),
        ("O", SitePinDir::In),
        ("T", SitePinDir::In),
        ("PADOUT", SitePinDir::Out),
        ("DIFFI_IN", SitePinDir::In),
        ("DIFFO_OUT", SitePinDir::Out),
        ("DIFFO_IN", SitePinDir::In),
    ];
    vrf.verify_legacy_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, defs::bslots::OLOGIC[idx]);
    vrf.claim_pip(bel.wire("O"), obel.wire("OQ"));
    vrf.claim_pip(bel.wire("T"), obel.wire("TQ"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IOB[idx ^ 1]);
    vrf.claim_pip(bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
    if kind == "IOBS" {
        vrf.claim_pip(bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
    }
}

fn verify_ioi(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let srow = RowId::from_idx(bel.row.to_idx() / 16 * 16 + 8);
    let obel = vrf.get_legacy_bel(bel.cell.with_row(srow).bel(defs::bslots::IOCLK));
    for i in 0..8 {
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel.wire(&format!("HCLK_O{i}")),
        ]);
    }
    for i in 0..2 {
        vrf.verify_net(&[
            bel.wire(&format!("RCLK{i}")),
            obel.wire(&format!("RCLK_O{i}")),
        ]);
    }
    // IOCLK verfied by hclk
}

fn verify_gt11(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let mut pins = vec![
        ("TX1P", SitePinDir::Out),
        ("TX1N", SitePinDir::Out),
        ("RX1P", SitePinDir::In),
        ("RX1N", SitePinDir::In),
        ("REFCLK1", SitePinDir::In),
        ("REFCLK2", SitePinDir::In),
        ("GREFCLK", SitePinDir::In),
        ("RXMCLK", SitePinDir::Out),
        ("TXPCSHCLKOUT", SitePinDir::Out),
        ("RXPCSHCLKOUT", SitePinDir::Out),
    ];
    let combusin: [_; 16] = core::array::from_fn(|i| format!("COMBUSIN{i}"));
    let combusout: [_; 16] = core::array::from_fn(|i| format!("COMBUSOUT{i}"));
    for pin in &combusin {
        pins.push((pin, SitePinDir::In));
    }
    for pin in &combusout {
        pins.push((pin, SitePinDir::Out));
    }
    vrf.verify_legacy_bel(bel, "GT11", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let gtidx = defs::bslots::GT11.index_of(bel.slot).unwrap();
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IPAD_RXP[gtidx]);
    vrf.claim_pip(bel.wire("RX1P"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IPAD_RXN[gtidx]);
    vrf.claim_pip(bel.wire("RX1N"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::OPAD_TXP[gtidx]);
    vrf.claim_pip(obel.wire("I"), bel.wire("TX1P"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::OPAD_TXN[gtidx]);
    vrf.claim_pip(obel.wire("I"), bel.wire("TX1N"));

    if gtidx == 0 {
        vrf.claim_pip(bel.wire_far("RXMCLK"), bel.wire("RXMCLK"));
    }

    for opin in ["REFCLK", "PMACLK"] {
        vrf.claim_net(&[bel.wire(opin)]);
        for i in 0..8 {
            vrf.claim_pip(bel.wire(opin), bel.wire(&format!("HCLK{i}")));
        }
    }
    let obel = vrf.get_legacy_bel(
        bel.cell
            .with_cr(endev.edev.col_cfg, bel.row + 8 + gtidx * 16)
            .bel(defs::bslots::CLK_HROW),
    );
    let lr = if bel.col <= endev.edev.col_cfg {
        'L'
    } else {
        'R'
    };
    for i in 0..8 {
        vrf.verify_net(&[
            bel.wire(&format!("HCLK{i}")),
            obel.wire(&format!("HCLK_{lr}{i}")),
        ]);
    }

    let obel_clk = vrf.find_bel_sibling(bel, defs::bslots::GT11CLK);

    vrf.claim_pip(bel.wire("GREFCLK"), bel.wire_far("GREFCLK"));
    vrf.verify_net(&[bel.wire_far("GREFCLK"), obel_clk.wire("PMACLK")]);

    vrf.claim_pip(bel.wire("REFCLK1"), bel.wire_far("REFCLK1"));
    vrf.claim_pip(bel.wire("REFCLK2"), bel.wire_far("REFCLK2"));
    vrf.verify_net(&[bel.wire_far("REFCLK1"), obel_clk.wire("SYNCLK1_N")]);
    vrf.verify_net(&[bel.wire_far("REFCLK2"), obel_clk.wire("SYNCLK2_N")]);

    for pin in ["TXPCSHCLKOUT", "RXPCSHCLKOUT"] {
        vrf.claim_pip(bel.wire_far(pin), bel.wire(pin));
        vrf.claim_net(&[bel.wire_far(pin)]);
    }

    vrf.claim_net(&[bel.wire("MGT0")]);
    vrf.claim_net(&[bel.wire("MGT1")]);
    vrf.claim_net(&[bel.wire("SYNCLK_OUT")]);
    vrf.claim_pip(bel.wire("MGT0"), bel.wire("SYNCLK_OUT"));
    vrf.claim_pip(bel.wire("MGT0"), bel.wire("FWDCLK0_OUT"));
    vrf.claim_pip(bel.wire("MGT0"), bel.wire("FWDCLK1_OUT"));
    vrf.claim_pip(bel.wire("MGT1"), bel.wire("SYNCLK_OUT"));
    vrf.claim_pip(bel.wire("MGT1"), bel.wire("FWDCLK0_OUT"));
    vrf.claim_pip(bel.wire("MGT1"), bel.wire("FWDCLK1_OUT"));
    vrf.claim_pip(bel.wire("SYNCLK_OUT"), bel.wire("SYNCLK1_OUT"));
    vrf.claim_pip(bel.wire("SYNCLK_OUT"), bel.wire("SYNCLK2_OUT"));
    vrf.verify_net(&[bel.wire("SYNCLK1_OUT"), obel_clk.wire("SYNCLK1_N")]);
    vrf.verify_net(&[bel.wire("SYNCLK2_OUT"), obel_clk.wire("SYNCLK2_N")]);
    if gtidx == 0 {
        vrf.verify_net(&[bel.wire("FWDCLK0_OUT"), obel_clk.wire("FWDCLK0B_OUT")]);
        vrf.verify_net(&[bel.wire("FWDCLK1_OUT"), obel_clk.wire("FWDCLK1B_OUT")]);
    } else {
        vrf.verify_net(&[bel.wire("FWDCLK0_OUT"), obel_clk.wire("FWDCLK0A_OUT")]);
        vrf.verify_net(&[bel.wire("FWDCLK1_OUT"), obel_clk.wire("FWDCLK1A_OUT")]);
    }

    for i in 1..=4 {
        vrf.claim_pip(
            bel.wire(&format!("FWDCLK{i}_B")),
            bel.wire(&format!("FWDCLK{i}_T")),
        );
        vrf.claim_pip(
            bel.wire(&format!("FWDCLK{i}_T")),
            bel.wire(&format!("FWDCLK{i}_B")),
        );
        if gtidx == 0 {
            vrf.verify_net(&[
                bel.wire(&format!("FWDCLK{i}_T")),
                obel_clk.wire(&format!("SFWDCLK{i}")),
            ]);
        } else {
            vrf.verify_net(&[
                bel.wire(&format!("FWDCLK{i}_B")),
                obel_clk.wire(&format!("NFWDCLK{i}")),
            ]);
            vrf.claim_net(&[bel.wire(&format!("FWDCLK{i}_T"))]);
        }
    }
    if gtidx == 0 {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -32, defs::bslots::GT11[1]) {
            for i in 1..=4 {
                vrf.verify_net(&[
                    bel.wire(&format!("FWDCLK{i}_B")),
                    obel.wire(&format!("FWDCLK{i}_T")),
                ]);
            }
        } else {
            for i in 1..=4 {
                vrf.claim_net(&[bel.wire(&format!("FWDCLK{i}_B"))]);
            }
        }
    }
}

fn verify_gt11clk(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let pins = [
        ("SYNCLK1IN", SitePinDir::In),
        ("SYNCLK2IN", SitePinDir::In),
        ("SYNCLK1OUT", SitePinDir::Out),
        ("SYNCLK2OUT", SitePinDir::Out),
        ("REFCLK", SitePinDir::In),
        ("RXBCLK", SitePinDir::In),
        ("MGTCLKP", SitePinDir::In),
        ("MGTCLKN", SitePinDir::In),
    ];
    vrf.verify_legacy_bel(bel, "GT11CLK", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IPAD_CLKP[0]);
    vrf.claim_pip(bel.wire("MGTCLKP"), obel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, defs::bslots::IPAD_CLKN[0]);
    vrf.claim_pip(bel.wire("MGTCLKN"), obel.wire("O"));
    let obel_a = vrf.find_bel_sibling(bel, defs::bslots::GT11[1]);
    let obel_b = vrf.find_bel_sibling(bel, defs::bslots::GT11[0]);

    vrf.verify_net(&[bel.wire("RXBCLK"), obel_b.wire_far("RXMCLK")]);

    vrf.verify_net(&[bel.wire("REFCLKA"), obel_a.wire("REFCLK")]);
    vrf.verify_net(&[bel.wire("REFCLKB"), obel_b.wire("REFCLK")]);
    vrf.claim_pip(bel.wire("REFCLK"), bel.wire("REFCLKA"));
    vrf.claim_pip(bel.wire("REFCLK"), bel.wire("REFCLKB"));

    vrf.verify_net(&[bel.wire("PMACLKA"), obel_a.wire("PMACLK")]);
    vrf.verify_net(&[bel.wire("PMACLKB"), obel_b.wire("PMACLK")]);
    vrf.claim_net(&[bel.wire("PMACLK")]);
    vrf.claim_pip(bel.wire("PMACLK"), bel.wire("PMACLKA"));
    vrf.claim_pip(bel.wire("PMACLK"), bel.wire("PMACLKB"));

    for i in 0..16 {
        vrf.verify_net(&[
            bel.wire(&format!("COMBUSIN_A{i}")),
            obel_a.wire(&format!("COMBUSIN{i}")),
        ]);
        vrf.verify_net(&[
            bel.wire(&format!("COMBUSIN_B{i}")),
            obel_b.wire(&format!("COMBUSIN{i}")),
        ]);
        vrf.verify_net(&[
            bel.wire(&format!("COMBUSOUT_A{i}")),
            obel_a.wire(&format!("COMBUSOUT{i}")),
        ]);
        vrf.verify_net(&[
            bel.wire(&format!("COMBUSOUT_B{i}")),
            obel_b.wire(&format!("COMBUSOUT{i}")),
        ]);
        vrf.claim_pip(
            bel.wire(&format!("COMBUSIN_A{i}")),
            bel.wire(&format!("COMBUSOUT_B{i}")),
        );
        vrf.claim_pip(
            bel.wire(&format!("COMBUSIN_B{i}")),
            bel.wire(&format!("COMBUSOUT_A{i}")),
        );
    }

    vrf.claim_net(&[bel.wire("SYNCLK1_N")]);
    vrf.claim_net(&[bel.wire("SYNCLK2_N")]);
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -32, defs::bslots::GT11CLK) {
        vrf.verify_net(&[bel.wire("SYNCLK1_S"), obel.wire("SYNCLK1_N")]);
        vrf.verify_net(&[bel.wire("SYNCLK2_S"), obel.wire("SYNCLK2_N")]);
    } else {
        vrf.claim_net(&[bel.wire("SYNCLK1_S")]);
        vrf.claim_net(&[bel.wire("SYNCLK2_S")]);
    }
    vrf.claim_pip(bel.wire("SYNCLK1_S"), bel.wire("SYNCLK1_N"));
    vrf.claim_pip(bel.wire("SYNCLK2_S"), bel.wire("SYNCLK2_N"));
    vrf.claim_pip(bel.wire("SYNCLK1_S"), bel.wire("SYNCLK1OUT"));
    vrf.claim_pip(bel.wire("SYNCLK2_S"), bel.wire("SYNCLK2OUT"));
    vrf.claim_pip(bel.wire("SYNCLK1_N"), bel.wire("SYNCLK1_S"));
    vrf.claim_pip(bel.wire("SYNCLK2_N"), bel.wire("SYNCLK2_S"));
    vrf.claim_pip(bel.wire("SYNCLK1_N"), bel.wire("SYNCLK1OUT"));
    vrf.claim_pip(bel.wire("SYNCLK2_N"), bel.wire("SYNCLK2OUT"));
    vrf.claim_pip(bel.wire("SYNCLK1IN"), bel.wire("SYNCLK1_N"));
    vrf.claim_pip(bel.wire("SYNCLK2IN"), bel.wire("SYNCLK2_N"));

    vrf.claim_net(&[bel.wire("FWDCLK0B_OUT")]);
    vrf.claim_net(&[bel.wire("FWDCLK1B_OUT")]);
    vrf.claim_net(&[bel.wire("FWDCLK0A_OUT")]);
    vrf.claim_net(&[bel.wire("FWDCLK1A_OUT")]);
    vrf.claim_pip(bel.wire("FWDCLK0B_OUT"), bel.wire("SFWDCLK1"));
    vrf.claim_pip(bel.wire("FWDCLK0B_OUT"), bel.wire("SFWDCLK2"));
    vrf.claim_pip(bel.wire("FWDCLK0B_OUT"), bel.wire("SFWDCLK3"));
    vrf.claim_pip(bel.wire("FWDCLK0B_OUT"), bel.wire("SFWDCLK4"));
    vrf.claim_pip(bel.wire("FWDCLK1B_OUT"), bel.wire("SFWDCLK1"));
    vrf.claim_pip(bel.wire("FWDCLK1B_OUT"), bel.wire("SFWDCLK2"));
    vrf.claim_pip(bel.wire("FWDCLK1B_OUT"), bel.wire("SFWDCLK3"));
    vrf.claim_pip(bel.wire("FWDCLK1B_OUT"), bel.wire("SFWDCLK4"));
    vrf.claim_pip(bel.wire("FWDCLK0A_OUT"), bel.wire("NFWDCLK1"));
    vrf.claim_pip(bel.wire("FWDCLK0A_OUT"), bel.wire("NFWDCLK2"));
    vrf.claim_pip(bel.wire("FWDCLK0A_OUT"), bel.wire("NFWDCLK3"));
    vrf.claim_pip(bel.wire("FWDCLK0A_OUT"), bel.wire("NFWDCLK4"));
    vrf.claim_pip(bel.wire("FWDCLK1A_OUT"), bel.wire("NFWDCLK1"));
    vrf.claim_pip(bel.wire("FWDCLK1A_OUT"), bel.wire("NFWDCLK2"));
    vrf.claim_pip(bel.wire("FWDCLK1A_OUT"), bel.wire("NFWDCLK3"));
    vrf.claim_pip(bel.wire("FWDCLK1A_OUT"), bel.wire("NFWDCLK4"));

    for i in 1..=4 {
        vrf.claim_net(&[bel.wire(&format!("NFWDCLK{i}"))]);
        vrf.claim_net(&[bel.wire(&format!("SFWDCLK{i}"))]);
        for pin in [
            "RXPCSHCLKOUTA",
            "RXPCSHCLKOUTB",
            "TXPCSHCLKOUTA",
            "TXPCSHCLKOUTB",
        ] {
            vrf.claim_pip(bel.wire(&format!("NFWDCLK{i}")), bel.wire(pin));
            vrf.claim_pip(bel.wire(&format!("SFWDCLK{i}")), bel.wire(pin));
        }
        vrf.claim_pip(
            bel.wire(&format!("NFWDCLK{i}")),
            bel.wire(&format!("SFWDCLK{i}")),
        );
        vrf.claim_pip(
            bel.wire(&format!("SFWDCLK{i}")),
            bel.wire(&format!("NFWDCLK{i}")),
        );
    }

    vrf.verify_net(&[bel.wire("RXPCSHCLKOUTA"), obel_a.wire_far("RXPCSHCLKOUT")]);
    vrf.verify_net(&[bel.wire("RXPCSHCLKOUTB"), obel_b.wire_far("RXPCSHCLKOUT")]);
    vrf.verify_net(&[bel.wire("TXPCSHCLKOUTA"), obel_a.wire_far("TXPCSHCLKOUT")]);
    vrf.verify_net(&[bel.wire("TXPCSHCLKOUTB"), obel_b.wire_far("TXPCSHCLKOUT")]);
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let slot_name = endev.edev.db.bel_slots.key(bel.slot);
    match bel.slot {
        _ if defs::bslots::SLICE.contains(bel.slot) => verify_slice(vrf, bel),
        _ if defs::bslots::DSP.contains(bel.slot) => verify_dsp(vrf, bel),
        defs::bslots::BRAM => verify_bram(vrf, bel),
        defs::bslots::FIFO => vrf.verify_legacy_bel(bel, "FIFO16", &[], &[]),
        defs::bslots::PPC => verify_ppc(vrf, bel),
        defs::bslots::EMAC => verify_emac(vrf, bel),

        _ if slot_name.starts_with("BUFGCTRL") => verify_bufgctrl(endev, vrf, bel),
        _ if defs::bslots::BSCAN.contains(bel.slot) => {
            vrf.verify_legacy_bel(bel, "BSCAN", &[], &[])
        }
        _ if defs::bslots::ICAP.contains(bel.slot) => vrf.verify_legacy_bel(bel, "ICAP", &[], &[]),
        _ if defs::bslots::PMV_CFG.contains(bel.slot) => {
            vrf.verify_legacy_bel(bel, "PMV", &[], &[])
        }
        defs::bslots::STARTUP
        | defs::bslots::FRAME_ECC
        | defs::bslots::DCIRESET
        | defs::bslots::CAPTURE
        | defs::bslots::USR_ACCESS
        | defs::bslots::DCI
        | defs::bslots::GLOBALSIG => vrf.verify_legacy_bel(bel, slot_name, &[], &[]),
        defs::bslots::JTAGPPC => verify_jtagppc(vrf, bel),
        defs::bslots::BUFG_MGTCLK_S | defs::bslots::BUFG_MGTCLK_N => {
            verify_bufg_mgtclk(endev, vrf, bel)
        }
        defs::bslots::BUFG_MGTCLK_S_HROW | defs::bslots::BUFG_MGTCLK_N_HROW => {
            verify_bufg_mgtclk_hrow(endev, vrf, bel)
        }
        defs::bslots::BUFG_MGTCLK_S_HCLK | defs::bslots::BUFG_MGTCLK_N_HCLK => {
            verify_bufg_mgtclk_hclk(endev, vrf, bel)
        }

        defs::bslots::CLK_HROW => verify_clk_hrow(endev, vrf, bel),
        defs::bslots::CLK_IOB => verify_clk_iob(endev, vrf, bel),
        defs::bslots::CLK_DCM => verify_clk_dcm(endev, vrf, bel),

        _ if defs::bslots::BUFR.contains(bel.slot) => verify_bufr(vrf, bel),
        _ if defs::bslots::BUFIO.contains(bel.slot) => verify_bufio(vrf, bel),
        defs::bslots::IDELAYCTRL => verify_idelayctrl(vrf, bel),
        defs::bslots::RCLK => verify_rclk(vrf, bel),
        defs::bslots::IOCLK => verify_ioclk(endev, vrf, bel),
        defs::bslots::HCLK_DCM | defs::bslots::HCLK_DCM_S | defs::bslots::HCLK_DCM_N => {
            verify_hclk_dcm(endev, vrf, bel)
        }
        defs::bslots::HCLK_DCM_HROW => verify_hclk_dcm_hrow(endev, vrf, bel),
        defs::bslots::HCLK => verify_hclk(endev, vrf, bel),

        _ if defs::bslots::ILOGIC.contains(bel.slot) => verify_ilogic(vrf, bel),
        _ if defs::bslots::OLOGIC.contains(bel.slot) => verify_ologic(vrf, bel),
        _ if defs::bslots::IOB.contains(bel.slot) => verify_iob(endev, vrf, bel),
        defs::bslots::IOI => verify_ioi(vrf, bel),

        _ if defs::bslots::DCM.contains(bel.slot) => verify_dcm(endev, vrf, bel),
        _ if defs::bslots::PMCD.contains(bel.slot) => verify_pmcd(vrf, bel),
        defs::bslots::DPM => verify_dpm(vrf, bel),
        defs::bslots::CCM => verify_ccm(endev, vrf, bel),
        defs::bslots::SYSMON => verify_sysmon(endev, vrf, bel),
        _ if defs::bslots::GT11.contains(bel.slot) => verify_gt11(endev, vrf, bel),
        defs::bslots::GT11CLK => verify_gt11clk(vrf, bel),
        _ if slot_name.starts_with("IPAD") => verify_ipad(vrf, bel),
        _ if slot_name.starts_with("OPAD") => verify_opad(vrf, bel),

        _ => println!("MEOW {} {:?}", slot_name, bel.name),
    }
}

fn verify_extra(_endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    vrf.kill_stub_in("PT_OMUX3_T5");
    vrf.kill_stub_in("PT_OMUX3_T6");
    vrf.kill_stub_in("PT_OMUX5_T5");
    vrf.kill_stub_in("PT_OMUX5_T6");
    vrf.kill_stub_in("PT_OMUX_E7_T5");
    vrf.kill_stub_in("PT_OMUX_E7_T6");
    vrf.kill_stub_in("PT_OMUX_W1_T5");
    vrf.kill_stub_in("PT_OMUX_W1_T6");
    vrf.kill_stub_out("PT_OMUX_EN8_T5");
    vrf.kill_stub_out("PT_OMUX_EN8_T6");
    vrf.kill_stub_out("PT_OMUX_N10_T5");
    vrf.kill_stub_out("PT_OMUX_N10_T6");
    vrf.kill_stub_out("PT_OMUX_N11_T5");
    vrf.kill_stub_out("PT_OMUX_N11_T6");
    vrf.kill_stub_out("PT_OMUX_N12_T5");
    vrf.kill_stub_out("PT_OMUX_N12_T6");

    vrf.kill_stub_in("PB_OMUX10_B5");
    vrf.kill_stub_in("PB_OMUX10_B6");
    vrf.kill_stub_in("PB_OMUX11_B5");
    vrf.kill_stub_in("PB_OMUX11_B6");
    vrf.kill_stub_in("PB_OMUX12_B5");
    vrf.kill_stub_in("PB_OMUX12_B6");
    vrf.kill_stub_in("PB_OMUX_E8_B5");
    vrf.kill_stub_in("PB_OMUX_E8_B6");
    vrf.kill_stub_out("PB_OMUX_ES7_B5");
    vrf.kill_stub_out("PB_OMUX_ES7_B6");
    vrf.kill_stub_out("PB_OMUX_S3_B5");
    vrf.kill_stub_out("PB_OMUX_S3_B6");
    vrf.kill_stub_out("PB_OMUX_S5_B5");
    vrf.kill_stub_out("PB_OMUX_S5_B6");
    vrf.kill_stub_out("PB_OMUX_WS1_B5");
    vrf.kill_stub_out("PB_OMUX_WS1_B6");

    vrf.kill_stub_out_cond("IOIS_BYP_INT_B0");
    vrf.kill_stub_out_cond("IOIS_BYP_INT_B2");
    vrf.kill_stub_out_cond("IOIS_BYP_INT_B4");
    vrf.kill_stub_out_cond("IOIS_BYP_INT_B7");
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    {
        let mut vrf = Verifier::new(rd, &endev.ngrid);
        for (wt, wf) in [
            (
                wires::IMUX_CLK_OPTINV.as_slice(),
                wires::IMUX_CLK.as_slice(),
            ),
            (wires::IMUX_SR_OPTINV.as_slice(), wires::IMUX_SR.as_slice()),
            (wires::IMUX_CE_OPTINV.as_slice(), wires::IMUX_CE.as_slice()),
        ] {
            for (&wt, &wf) in wt.iter().zip(wf) {
                vrf.alias_wire_slot(wt, wf);
            }
        }
        vrf.prep_int_wires();
        vrf.handle_int();
        for (tcrd, tile) in endev.ngrid.egrid.tiles() {
            let tcls = &endev.ngrid.egrid.db[tile.class];
            for (slot, bel) in &tcls.bels {
                if let BelInfo::Legacy(_) = bel {
                    let ctx = vrf.get_legacy_bel(tcrd.bel(slot));
                    verify_bel(endev, &mut vrf, &ctx);
                }
            }
        }
        verify_extra(endev, &mut vrf);
        vrf.finish();
    };
}
