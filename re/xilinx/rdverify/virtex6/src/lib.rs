#![recursion_limit = "1024"]

use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_virtex4::{
    bels,
    chip::{DisabledPart, GtKind},
};
use unnamed_entity::EntityId;

fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.info.pins.contains_key("WE") {
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
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.slot) {
        vrf.claim_net(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
        vrf.claim_pip(obel.crd(), obel.wire_far("COUT"), obel.wire("COUT"));
    } else {
        vrf.claim_net(&[bel.fwire("CIN")]);
    }
    vrf.claim_net(&[bel.fwire("COUT")]);
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
        vrf.claim_net(&[bel.fwire(opin)]);
        if bel.slot == bels::DSP0 {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bels::DSP1) {
                vrf.claim_net(&[bel.fwire(ipin), obel.fwire_far(opin)]);
                vrf.claim_pip(obel.crd(), obel.wire_far(opin), obel.wire(opin));
            } else {
                vrf.claim_net(&[bel.fwire(ipin)]);
            }
        } else {
            vrf.claim_net(&[bel.fwire(ipin)]);
            let obel = vrf.find_bel_sibling(bel, bels::DSP0);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_bel(bel, "DSP48E1", &pins, &[]);
    let obel = vrf.find_bel_sibling(bel, bels::TIEOFF_DSP);
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
        vrf.claim_net(&[bel.fwire(pin)]);
    }
}

fn verify_bram_f(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CASCADEINA", SitePinDir::In),
        ("CASCADEINB", SitePinDir::In),
        ("CASCADEOUTA", SitePinDir::Out),
        ("CASCADEOUTB", SitePinDir::Out),
        ("TSTOUT1", SitePinDir::Out),
        ("TSTOUT2", SitePinDir::Out),
        ("TSTOUT3", SitePinDir::Out),
        ("TSTOUT4", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "RAMBFIFO36E1", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.fwire(pin)]);
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bel.slot) {
        for (ipin, opin) in [("CASCADEINA", "CASCADEOUTA"), ("CASCADEINB", "CASCADEOUTB")] {
            vrf.verify_net(&[bel.fwire(ipin), obel.fwire_far(opin)]);
            vrf.claim_pip(obel.crd(), obel.wire_far(opin), obel.wire(opin));
        }
    }
}

fn verify_bram_h1(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![
        "FULL".to_string(),
        "EMPTY".to_string(),
        "ALMOSTFULL".to_string(),
        "ALMOSTEMPTY".to_string(),
        "WRERR".to_string(),
        "RDERR".to_string(),
    ];
    for i in 0..12 {
        pins.push(format!("RDCOUNT{i}"));
        pins.push(format!("WRCOUNT{i}"));
    }
    let pin_refs: Vec<_> = pins.iter().map(|x| (&x[..], SitePinDir::Out)).collect();
    vrf.verify_bel(bel, "RAMB18E1", &pin_refs, &[]);
    for pin in pins {
        vrf.claim_net(&[bel.fwire(&pin)]);
    }
}

fn verify_hclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..8 {
        for j in 0..12 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("LCLK{i}_D")),
                bel.wire(&format!("HCLK{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("LCLK{i}_U")),
                bel.wire(&format!("HCLK{j}")),
            );
        }
        for j in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("LCLK{i}_D")),
                bel.wire(&format!("RCLK{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("LCLK{i}_U")),
                bel.wire(&format!("RCLK{j}")),
            );
        }
    }
    let scol = if bel.col <= endev.edev.col_cfg {
        endev.edev.chips[bel.die].cols_qbuf.unwrap().0
    } else {
        endev.edev.chips[bel.die].cols_qbuf.unwrap().1
    };
    let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::HCLK_QBUF));
    for i in 0..12 {
        vrf.verify_net(&[
            bel.fwire(&format!("HCLK{i}")),
            obel.fwire(&format!("HCLK{i}_O")),
        ]);
    }
    // regional clocks can be sourced from both inner and outer IO columns, but we consider inner
    // to be the source for simplicity.
    let scol = if bel.col <= endev.edev.col_cfg {
        endev.edev.col_lcio.unwrap()
    } else {
        endev.edev.col_rcio.unwrap()
    };
    let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::HCLK_IOI));
    for i in 0..6 {
        vrf.verify_net(&[
            bel.fwire(&format!("RCLK{i}")),
            obel.fwire(&format!("RCLK{i}_I")),
        ]);
    }
}

fn verify_hclk_qbuf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.get_bel(bel.cell.with_col(endev.edev.col_cfg).bel(bels::CMT));
    let lr = if bel.col < endev.edev.col_cfg {
        'L'
    } else {
        'R'
    };
    for i in 0..12 {
        vrf.claim_net(&[bel.fwire(&format!("HCLK{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}_I")),
        );
        vrf.verify_net(&[
            bel.fwire(&format!("HCLK{i}_I")),
            obel.fwire(&format!("HCLK{i}_{lr}_O")),
        ]);
    }
}

fn verify_bufo(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFO",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.fwire("I")]);
    vrf.claim_net(&[bel.fwire("O")]);
    vrf.claim_net(&[bel.fwire("VI")]);
    vrf.claim_net(&[bel.fwire("I_PRE")]);
    vrf.claim_net(&[bel.fwire("I_PRE2")]);

    vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire("I_PRE"));
    vrf.claim_pip(bel.crd(), bel.wire("VI"), bel.wire("I_PRE"));

    if let Some(obel) = vrf.find_bel_delta(bel, 0, 40, bel.slot) {
        vrf.verify_net(&[bel.fwire("VI_S"), obel.fwire("VI")]);
        vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire("VI_S"));
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -40, bel.slot) {
        vrf.verify_net(&[bel.fwire("VI_N"), obel.fwire("VI")]);
        vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire("VI_N"));
    }

    vrf.claim_pip(bel.crd(), bel.wire("I_PRE"), bel.wire("I_PRE2"));
    let idx = bels::BUFO.into_iter().position(|x| x == bel.slot).unwrap();
    let obel = vrf.find_bel_sibling(bel, bels::HCLK_IOI);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I_PRE2"),
        obel.wire(&format!("PERF{ii}_BUF", ii = idx * 3)),
    );
}

fn verify_bufiodqs(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFIODQS",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.fwire("I")]);
    vrf.claim_net(&[bel.fwire("O")]);
    let idx = bels::BUFIO.into_iter().position(|x| x == bel.slot).unwrap();
    let obel = vrf.find_bel_sibling(bel, bels::HCLK_IOI);
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
    vrf.claim_net(&[bel.fwire("I")]);
    vrf.claim_net(&[bel.fwire("O")]);

    let obel = vrf.find_bel_sibling(bel, bels::HCLK_IOI);
    for i in 0..2 {
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
    for i in 0..10 {
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire(&format!("MGT{i}")));
    }
}

fn verify_idelayctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IDELAYCTRL", &[("REFCLK", SitePinDir::In)], &[]);
    vrf.claim_net(&[bel.fwire("REFCLK")]);
    let obel = vrf.find_bel_sibling(bel, bels::HCLK_IOI);
    for i in 0..12 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REFCLK"),
            obel.wire(&format!("HCLK{i}_O")),
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
        vrf.claim_net(&[bel.fwire(pin)]);
    }
}

fn verify_hclk_ioi(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let scol = if bel.col <= endev.edev.col_cfg {
        endev.edev.chips[bel.die].cols_qbuf.unwrap().0
    } else {
        endev.edev.chips[bel.die].cols_qbuf.unwrap().1
    };
    let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::HCLK_QBUF));
    for i in 0..12 {
        vrf.claim_net(&[bel.fwire(&format!("HCLK{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}_O")),
            bel.wire(&format!("HCLK{i}_I")),
        );
        vrf.verify_net(&[
            bel.fwire(&format!("HCLK{i}_I")),
            obel.fwire(&format!("HCLK{i}_O")),
        ]);
    }

    let scol = if bel.col <= endev.edev.col_cfg {
        endev.edev.col_lcio.unwrap()
    } else {
        endev.edev.col_rcio.unwrap()
    };
    if bel.col == scol {
        for i in 0..6 {
            vrf.claim_net(&[bel.fwire(&format!("RCLK{i}_I"))]);
        }
    } else {
        let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::HCLK_IOI));
        for i in 0..6 {
            vrf.verify_net(&[
                bel.fwire(&format!("RCLK{i}_I")),
                obel.fwire(&format!("RCLK{i}_I")),
            ]);
        }
    }
    for i in 0..6 {
        vrf.claim_net(&[bel.fwire(&format!("RCLK{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK{i}_O")),
            bel.wire(&format!("RCLK{i}_I")),
        );
        for j in 0..2 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCLK{i}_I")),
                bel.wire(&format!("VRCLK{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCLK{i}_I")),
                bel.wire(&format!("VRCLK{j}_S")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCLK{i}_I")),
                bel.wire(&format!("VRCLK{j}_N")),
            );
        }
    }

    let obel_s = vrf.find_bel_delta(bel, 0, 40, bels::HCLK_IOI);
    let obel_n = vrf.find_bel_delta(bel, 0, -40, bels::HCLK_IOI);
    for i in 0..2 {
        vrf.claim_net(&[bel.fwire(&format!("VRCLK{i}"))]);
        let obel = vrf.find_bel_sibling(bel, bels::BUFR[i]);
        vrf.claim_pip(bel.crd(), bel.wire(&format!("VRCLK{i}")), obel.wire("O"));
        if let Some(ref obel) = obel_s {
            vrf.verify_net(&[
                bel.fwire(&format!("VRCLK{i}_S")),
                obel.fwire(&format!("VRCLK{i}")),
            ]);
        } else {
            vrf.claim_net(&[bel.fwire(&format!("VRCLK{i}_S"))]);
        }
        if let Some(ref obel) = obel_n {
            vrf.verify_net(&[
                bel.fwire(&format!("VRCLK{i}_N")),
                obel.fwire(&format!("VRCLK{i}")),
            ]);
        } else {
            vrf.claim_net(&[bel.fwire(&format!("VRCLK{i}_N"))]);
        }
    }

    for i in 0..4 {
        vrf.claim_net(&[bel.fwire(&format!("PERF{i}_BUF"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PERF{i}_BUF")),
            bel.wire(&format!("PERF{i}")),
        );

        vrf.claim_net(&[bel.fwire(&format!("IOCLK_IN{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}")),
            bel.wire(&format!("PERF{ii}_BUF", ii = i ^ 1)),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}")),
            bel.wire(&format!("IOCLK_PAD{i}")),
        );

        vrf.claim_net(&[bel.fwire(&format!("IOCLK_IN{i}_BUFR"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK_IN{i}_BUFR")),
            bel.wire(&format!("IOCLK_IN{i}")),
        );

        let dy = match i {
            0 => 0,
            1 => 2,
            2 => -4,
            3 => -2,
            _ => unreachable!(),
        };
        let obel = vrf.find_bel_delta(bel, 0, dy, bels::ILOGIC1).unwrap();
        vrf.verify_net(&[bel.fwire(&format!("IOCLK_PAD{i}")), obel.fwire("CLKOUT")]);
    }
    let obel_cmt = vrf.get_bel(bel.cell.with_col(endev.edev.col_cfg).bel(bels::CMT));
    let which = match [
        endev.edev.col_lio,
        endev.edev.col_lcio,
        endev.edev.col_rcio,
        endev.edev.col_rio,
    ]
    .into_iter()
    .position(|x| x == Some(bel.col))
    .unwrap()
    {
        0 => "OL",
        1 => "IL",
        2 => "IR",
        3 => "OR",
        _ => unreachable!(),
    };
    for i in 0..4 {
        vrf.verify_net(&[
            bel.fwire(&format!("PERF{i}")),
            obel_cmt.fwire(&format!("PERF{i}_{which}_O")),
        ]);
    }

    for (i, pin) in [
        (0, "IOCLK0_PRE"),
        (1, "IOCLK1_PRE"),
        (2, "IOCLK2_PRE"),
        (3, "IOCLK3_PRE"),
        (4, "IOCLK0_PRE_S"),
        (5, "IOCLK3_PRE_S"),
        (6, "IOCLK0_PRE_N"),
        (7, "IOCLK3_PRE_N"),
    ] {
        vrf.claim_net(&[bel.fwire(&format!("IOCLK{i}"))]);
        vrf.claim_net(&[bel.fwire(&format!("IOCLK{i}_DLY"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK{i}")),
            bel.wire(&format!("IOCLK{i}_DLY")),
        );
        vrf.claim_pip(bel.crd(), bel.wire(&format!("IOCLK{i}")), bel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("IOCLK{i}_DLY")), bel.wire(pin));
    }
    for i in 0..4 {
        let obel = vrf.find_bel_sibling(bel, bels::BUFIO[i]);
        vrf.claim_net(&[bel.fwire(&format!("IOCLK{i}_PRE"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK{i}_PRE")),
            obel.wire("O"),
        );
    }
    for i in [0, 3] {
        if let Some(ref obel) = obel_s {
            vrf.verify_net(&[
                bel.fwire(&format!("IOCLK{i}_PRE_S")),
                obel.fwire(&format!("IOCLK{i}_PRE")),
            ]);
        } else {
            vrf.claim_net(&[bel.fwire(&format!("IOCLK{i}_PRE_S"))]);
        }
        if let Some(ref obel) = obel_n {
            vrf.verify_net(&[
                bel.fwire(&format!("IOCLK{i}_PRE_N")),
                obel.fwire(&format!("IOCLK{i}_PRE")),
            ]);
        } else {
            vrf.claim_net(&[bel.fwire(&format!("IOCLK{i}_PRE_N"))]);
        }
    }

    for i in 0..2 {
        let obel = vrf.find_bel_sibling(bel, bels::BUFO[i]);
        vrf.claim_net(&[bel.fwire(&format!("OCLK{i}"))]);
        vrf.claim_pip(bel.crd(), bel.wire(&format!("OCLK{i}")), obel.wire("O"));
    }

    if matches!(which, "OL" | "OR") {
        let scol = if which == "OL" {
            endev.edev.chips[bel.die].columns.first_id().unwrap()
        } else {
            endev.edev.chips[bel.die].columns.last_id().unwrap()
        };
        if let Some(obel) = vrf
            .find_bel(bel.cell.with_col(scol).bel(bels::HCLK_GTX))
            .or_else(|| vrf.find_bel(bel.cell.with_col(scol).bel(bels::HCLK_GTH)))
        {
            for i in 0..10 {
                vrf.verify_net(&[
                    bel.fwire(&format!("MGT{i}")),
                    obel.fwire(&format!("MGT{i}")),
                ]);
            }
        } else {
            for i in 0..10 {
                vrf.claim_net(&[bel.fwire(&format!("MGT{i}"))]);
            }
        }
    } else {
        let reg = endev.edev.chips[bel.die].row_to_reg(bel.row);
        if which == "IR"
            && endev.edev.disabled.contains(&DisabledPart::GtxRow(reg))
            && endev.edev.col_rio.is_none()
        {
            for i in 0..10 {
                vrf.claim_net(&[bel.fwire(&format!("MGT{i}"))]);
            }
        } else {
            let dx = if which == "IL" { -1 } else { 1 };
            if let Some(obel) = vrf.find_bel_walk(bel, dx, 0, bels::MGT_BUF) {
                for i in 0..10 {
                    vrf.verify_net(&[
                        bel.fwire(&format!("MGT{i}")),
                        obel.fwire(&format!("MGT{i}_O")),
                    ]);
                }
            } else {
                let scol = if which == "IL" {
                    endev.edev.col_lio
                } else {
                    endev.edev.col_rio
                }
                .unwrap();
                let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::HCLK_IOI));
                for i in 0..10 {
                    vrf.verify_net(&[
                        bel.fwire(&format!("MGT{i}")),
                        obel.fwire(&format!("MGT{i}")),
                    ]);
                }
            }
        }
    }
}

fn verify_ilogic(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    vrf.verify_bel(bel, "ILOGICE1", &pins, &["CKINT0", "CKINT1"]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.fwire(pin)]);
    }

    let obel_ioi = vrf.find_bel_sibling(bel, bels::IOI);
    for pin in ["CLK", "CLKB"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKINT0"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKINT1"));
        for i in 0..12 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
        for i in 0..8 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_ioi.wire(&format!("IOCLK{i}")),
            );
        }
    }

    let obel_ologic = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::ILOGIC0 => bels::OLOGIC0,
            bels::ILOGIC1 => bels::OLOGIC1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("OCLK"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("OCLKB"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("OCLKB"), obel_ologic.wire("CLKM"));
    vrf.claim_pip(bel.crd(), bel.wire("OFB"), obel_ologic.wire("OFB"));
    vrf.claim_pip(bel.crd(), bel.wire("TFB"), obel_ologic.wire("TFB_BUF"));

    let obel_iodelay = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::ILOGIC0 => bels::IODELAY0,
            bels::ILOGIC1 => bels::IODELAY1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("DDLY"), obel_iodelay.wire("DATAOUT"));

    let obel_iob = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::ILOGIC0 => bels::IOB0,
            bels::ILOGIC1 => bels::IOB1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("D"), bel.wire("IOB_I_BUF"));
    vrf.claim_net(&[bel.fwire("IOB_I_BUF")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOB_I_BUF"), bel.wire("IOB_I"));
    vrf.verify_net(&[bel.fwire("IOB_I"), obel_iob.fwire("I")]);

    if bel.slot == bels::ILOGIC0 {
        let obel = vrf.find_bel_sibling(bel, bels::ILOGIC1);
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }

    let is_rclk = matches!(bel.row.to_idx() % 40, 16 | 18 | 20 | 22);
    let is_inner =
        bel.col == endev.edev.col_lcio.unwrap() || bel.col == endev.edev.col_rcio.unwrap();
    let is_gclk = is_inner
        && (bel.row == endev.edev.chips[bel.die].row_bufg() - 4
            || bel.row == endev.edev.chips[bel.die].row_bufg() - 2
            || bel.row == endev.edev.chips[bel.die].row_bufg()
            || bel.row == endev.edev.chips[bel.die].row_bufg() + 2);
    if (is_rclk || is_gclk) && bel.slot == bels::ILOGIC1 {
        vrf.claim_net(&[bel.fwire("CLKOUT")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLKOUT"), bel.wire("O"));
        if is_inner {
            vrf.claim_net(&[bel.fwire("CLKOUT_CMT")]);
            vrf.claim_pip(bel.crd(), bel.wire("CLKOUT_CMT"), bel.wire("CLKOUT"));
        }
    }
}

fn verify_ologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLK", SitePinDir::In),
        ("CLKB", SitePinDir::In),
        ("CLKDIV", SitePinDir::In),
        ("CLKDIVB", SitePinDir::In),
        ("CLKPERF", SitePinDir::In),
        ("CLKPERFDELAY", SitePinDir::In),
        ("OFB", SitePinDir::Out),
        ("TFB", SitePinDir::Out),
        ("OQ", SitePinDir::Out),
        ("TQ", SitePinDir::Out),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
        ("REV", SitePinDir::In),
    ];
    vrf.verify_bel(
        bel,
        "OLOGICE1",
        &pins,
        &["CLK_CKINT", "CLKDIV_CKINT", "CLK_MUX", "TFB_BUF", "CLKDIV"],
    );
    for (pin, _) in pins {
        if pin == "CLKDIV" {
            continue;
        }
        vrf.claim_net(&[bel.fwire(pin)]);
    }

    vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKB"), bel.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKB"), bel.wire("CLKM"));

    let obel_ioi = vrf.find_bel_sibling(bel, bels::IOI);
    vrf.claim_net(&[bel.fwire("CLKM")]);
    for pin in ["CLK_MUX", "CLKM"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CLK_CKINT"));
        for i in 0..12 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
        for i in 0..8 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_ioi.wire(&format!("IOCLK{i}")),
            );
        }
    }

    for pin in ["CLKDIV", "CLKDIVB"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CLKDIV_CKINT"));
        for i in 0..12 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("HCLK{i}")));
        }
        for i in 0..6 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_ioi.wire(&format!("RCLK{i}")));
        }
    }

    for i in 0..2 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKPERF"),
            obel_ioi.wire(&format!("OCLK{i}")),
        );
    }

    let obel_iodelay = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::OLOGIC0 => bels::IODELAY0,
            bels::OLOGIC1 => bels::IODELAY1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKPERFDELAY"),
        obel_iodelay.wire("DATAOUT"),
    );

    vrf.claim_pip(bel.crd(), bel.wire("TFB_BUF"), bel.wire("TFB"));

    let obel_iob = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::OLOGIC0 => bels::IOB0,
            bels::OLOGIC1 => bels::IOB1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("IOB_T"), bel.wire("TQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), bel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), obel_iodelay.wire("DATAOUT"));
    vrf.verify_net(&[bel.fwire("IOB_O"), obel_iob.fwire("O")]);
    vrf.verify_net(&[bel.fwire("IOB_T"), obel_iob.fwire("T")]);

    if bel.slot == bels::OLOGIC1 {
        let obel = vrf.find_bel_sibling(bel, bels::OLOGIC0);
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_iodelay(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLKIN", SitePinDir::In),
        ("IDATAIN", SitePinDir::In),
        ("ODATAIN", SitePinDir::In),
        ("DATAOUT", SitePinDir::Out),
        ("T", SitePinDir::In),
    ];
    vrf.verify_bel(bel, "IODELAYE1", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.fwire(pin)]);
    }

    let obel_ilogic = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::IODELAY0 => bels::ILOGIC0,
            bels::IODELAY1 => bels::ILOGIC1,
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
        match bel.slot {
            bels::IODELAY0 => bels::OLOGIC0,
            bels::IODELAY1 => bels::OLOGIC1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel_ologic.wire("CLK_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("ODATAIN"), obel_ologic.wire("OFB"));
    vrf.claim_pip(bel.crd(), bel.wire("T"), obel_ologic.wire("TFB"));
}

fn verify_iob(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = match bel.slot {
        bels::IOB1 => "IOBM",
        bels::IOB0 => "IOBS",
        _ => unreachable!(),
    };
    let mut pins = vec![
        ("I", SitePinDir::Out),
        ("O", SitePinDir::In),
        ("T", SitePinDir::In),
        ("O_IN", SitePinDir::In),
        ("O_OUT", SitePinDir::Out),
        ("DIFFO_IN", SitePinDir::In),
        ("DIFFO_OUT", SitePinDir::Out),
        ("DIFFI_IN", SitePinDir::In),
        ("PADOUT", SitePinDir::Out),
    ];
    if kind == "IOBM" {
        pins.push(("DIFF_TERM_INT_EN", SitePinDir::In));
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.fwire(pin)]);
    }
    let oslot = match bel.slot {
        bels::IOB0 => bels::IOB1,
        bels::IOB1 => bels::IOB0,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    if kind == "IOBS" {
        vrf.claim_pip(bel.crd(), bel.wire("O_IN"), obel.wire("O_OUT"));
        vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
    }
    vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
}

fn verify_ioi(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let srow = endev.edev.chips[bel.die].row_hclk(bel.row);
    let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::HCLK_IOI));
    for i in 0..12 {
        vrf.verify_net(&[
            bel.fwire(&format!("HCLK{i}")),
            obel.fwire(&format!("HCLK{i}_O")),
        ]);
    }
    for i in 0..6 {
        vrf.verify_net(&[
            bel.fwire(&format!("RCLK{i}")),
            obel.fwire(&format!("RCLK{i}_O")),
        ]);
    }
    for i in 0..8 {
        vrf.verify_net(&[
            bel.fwire(&format!("IOCLK{i}")),
            obel.fwire(&format!("IOCLK{i}")),
        ]);
    }
    for i in 0..2 {
        vrf.verify_net(&[
            bel.fwire(&format!("OCLK{i}")),
            obel.fwire(&format!("OCLK{i}")),
        ]);
    }
}

fn verify_sysmon(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    for i in 0..16 {
        pins.push(format!("VAUXP{i}"));
        pins.push(format!("VAUXN{i}"));
    }
    pins.push("VP".to_string());
    pins.push("VN".to_string());
    let mut pin_refs = vec![];
    for pin in &pins {
        pin_refs.push((&pin[..], SitePinDir::In));
    }
    vrf.verify_bel(bel, "SYSMON", &pin_refs, &[]);

    vrf.claim_net(&[bel.fwire("VP")]);
    let obel = vrf.find_bel_sibling(bel, bels::IPAD_VP);
    vrf.claim_pip(bel.crd(), bel.wire("VP"), obel.wire("O"));
    vrf.claim_net(&[bel.fwire("VN")]);
    let obel = vrf.find_bel_sibling(bel, bels::IPAD_VN);
    vrf.claim_pip(bel.crd(), bel.wire("VN"), obel.wire("O"));

    for i in 0..16 {
        let Some((iop, _)) = endev.edev.get_sysmon_vaux(bel.cell, i) else {
            continue;
        };
        let vauxp = format!("VAUXP{i}");
        let vauxn = format!("VAUXN{i}");
        vrf.claim_net(&[bel.fwire(&vauxp)]);
        vrf.claim_net(&[bel.fwire(&vauxn)]);
        vrf.claim_pip(bel.crd(), bel.wire(&vauxp), bel.wire_far(&vauxp));
        vrf.claim_pip(bel.crd(), bel.wire(&vauxn), bel.wire_far(&vauxn));
        let obel = vrf.get_bel(iop.cell.bel(bels::IOB1));
        vrf.claim_net(&[bel.fwire_far(&vauxp), obel.fwire("MONITOR")]);
        vrf.claim_pip(obel.crd(), obel.wire("MONITOR"), obel.wire("PADOUT"));
        let obel = vrf.get_bel(iop.cell.bel(bels::IOB0));
        vrf.claim_net(&[bel.fwire_far(&vauxn), obel.fwire("MONITOR")]);
        vrf.claim_pip(obel.crd(), obel.wire("MONITOR"), obel.wire("PADOUT"));
    }
}

fn verify_ipad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_net(&[bel.fwire("O")]);
}

fn verify_opad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "OPAD", &[("I", SitePinDir::In)], &[]);
    vrf.claim_net(&[bel.fwire("I")]);
}

fn verify_bufhce(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFHCE",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_net(&[bel.fwire("I")]);
    vrf.claim_net(&[bel.fwire("O")]);

    let obel = vrf.find_bel_sibling(bel, bels::CMT);
    vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire("BUFH_TEST_L"));
    vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire("BUFH_TEST_R"));
    for i in 0..4 {
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire(&format!("CCIO{i}_L")));
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire(&format!("CCIO{i}_R")));
    }
    for i in 0..14 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I"),
            obel.wire(&format!("MMCM0_OUT{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I"),
            obel.wire(&format!("MMCM1_OUT{i}")),
        );
    }
    for i in 0..32 {
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire(&format!("GCLK{i}")));
    }
    let lr = if bels::BUFHCE_W.contains(&bel.slot) {
        'L'
    } else {
        'R'
    };
    for i in 0..2 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I"),
            obel.wire(&format!("BUFHCE_{lr}_CKINT{i}")),
        );
    }
}

fn verify_mmcm(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::MMCM.into_iter().position(|x| x == bel.slot).unwrap();
    let pins = [
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT2", SitePinDir::Out),
        ("CLKOUT3", SitePinDir::Out),
        ("CLKOUT4", SitePinDir::Out),
        ("CLKOUT5", SitePinDir::Out),
        ("CLKOUT6", SitePinDir::Out),
        ("CLKFBOUT", SitePinDir::Out),
        ("CLKOUT0B", SitePinDir::Out),
        ("CLKOUT1B", SitePinDir::Out),
        ("CLKOUT2B", SitePinDir::Out),
        ("CLKOUT3B", SitePinDir::Out),
        ("CLKFBOUTB", SitePinDir::Out),
        ("TMUXOUT", SitePinDir::Out),
        ("CLKIN1", SitePinDir::In),
        ("CLKIN2", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
    ];
    vrf.verify_bel(
        bel,
        "MMCM_ADV",
        &pins,
        &["CLKIN1_CKINT", "CLKIN2_CKINT", "CLKFBIN_CKINT"],
    );
    for (pin, _) in pins {
        vrf.claim_net(&[bel.fwire(pin)]);
    }
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
        vrf.claim_net(&[bel.fwire(&format!("CMT_OUT{i}"))]);
        vrf.claim_pip(bel.crd(), bel.wire(&format!("CMT_OUT{i}")), bel.wire(pin));
    }

    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_HCLK"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_IO"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_MGT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CASC_IN"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_CKINT"));

    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_HCLK"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_IO"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_MGT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_CKINT"));

    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire("CLKFBIN_HCLK"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire("CLKFBIN_IO"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire("CASC_OUT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire("CLKFB"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire("CLKFBIN_CKINT"));

    vrf.claim_net(&[bel.fwire("CLKFB")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLKFB"), bel.wire("CLKFBOUT"));

    let obel_cmt = vrf.find_bel_sibling(bel, bels::CMT);
    for pin in [
        "CLKFBIN_HCLK",
        "CLKFBIN_IO",
        "CLKIN1_HCLK",
        "CLKIN1_IO",
        "CLKIN1_MGT",
        "CLKIN2_HCLK",
        "CLKIN2_IO",
        "CLKIN2_MGT",
    ] {
        vrf.verify_net(&[bel.fwire(pin), obel_cmt.fwire(&format!("MMCM{idx}_{pin}"))]);
    }

    vrf.claim_net(&[bel.fwire("CASC_OUT")]);
    for pin in [
        "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKOUT6", "CLKFBOUT",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire("CASC_OUT"), bel.wire(pin));
    }
    let obel_mmcm = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::MMCM0 => bels::MMCM1,
            bels::MMCM1 => bels::MMCM0,
            _ => unreachable!(),
        },
    );
    vrf.verify_net(&[bel.fwire("CASC_IN"), obel_mmcm.fwire("CASC_OUT")]);

    for i in 0..4 {
        vrf.claim_net(&[bel.fwire(&format!("PERF{i}"))]);
        for pin in ["CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3"] {
            vrf.claim_pip(bel.crd(), bel.wire(&format!("PERF{i}")), bel.wire(pin));
        }
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PERF{i}_IL")),
            bel.wire(&format!("PERF{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PERF{i}_IR")),
            bel.wire(&format!("PERF{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PERF{ii}_OL", ii = i ^ 1)),
            bel.wire(&format!("PERF{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PERF{ii}_OR", ii = i ^ 1)),
            bel.wire(&format!("PERF{i}")),
        );
    }
}

pub fn verify_cmt(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (scol_h, scol_r, lr, bufhce) in [
        (
            endev.edev.chips[bel.die].cols_qbuf.unwrap().0,
            endev.edev.col_lcio.unwrap(),
            'L',
            bels::BUFHCE_W,
        ),
        (
            endev.edev.chips[bel.die].cols_qbuf.unwrap().1,
            endev.edev.col_rcio.unwrap(),
            'R',
            bels::BUFHCE_E,
        ),
    ] {
        vrf.claim_net(&[bel.fwire(&format!("BUFH_TEST_{lr}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("BUFH_TEST_{lr}")),
            bel.wire(&format!("BUFH_TEST_{lr}_INV")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("BUFH_TEST_{lr}")),
            bel.wire(&format!("BUFH_TEST_{lr}_NOINV")),
        );
        vrf.claim_net(&[bel.fwire(&format!("BUFH_TEST_{lr}_INV"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("BUFH_TEST_{lr}_INV")),
            bel.wire(&format!("BUFH_TEST_{lr}_PRE")),
        );
        vrf.claim_net(&[bel.fwire(&format!("BUFH_TEST_{lr}_NOINV"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("BUFH_TEST_{lr}_NOINV")),
            bel.wire(&format!("BUFH_TEST_{lr}_PRE")),
        );
        vrf.claim_net(&[bel.fwire(&format!("BUFH_TEST_{lr}_PRE"))]);

        let obel_qbuf = vrf.get_bel(bel.cell.with_col(scol_h).bel(bels::HCLK_QBUF));
        for i in 0..12 {
            vrf.claim_net(&[bel.fwire(&format!("HCLK{i}_{lr}_O"))]);
            let obel = vrf.find_bel_sibling(bel, bufhce[i]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HCLK{i}_{lr}_O")),
                obel.wire("O"),
            );
            vrf.verify_net(&[
                bel.fwire(&format!("HCLK{i}_{lr}_I")),
                obel_qbuf.fwire(&format!("HCLK{i}_O")),
            ]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("BUFH_TEST_{lr}_PRE")),
                bel.wire(&format!("HCLK{i}_{lr}_I")),
            );
        }
        let obel_io = vrf.get_bel(bel.cell.with_col(scol_r).bel(bels::HCLK_IOI));
        for i in 0..6 {
            vrf.verify_net(&[
                bel.fwire(&format!("RCLK{i}_{lr}_I")),
                obel_io.fwire(&format!("RCLK{i}_I")),
            ]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("BUFH_TEST_{lr}_PRE")),
                bel.wire(&format!("RCLK{i}_{lr}_I")),
            );
        }
    }

    let obel_mmcm0 = vrf.find_bel_sibling(bel, bels::MMCM0);
    let obel_mmcm1 = vrf.find_bel_sibling(bel, bels::MMCM1);

    for i in 0..4 {
        for which in ["OL", "IL", "IR", "OR"] {
            vrf.claim_net(&[
                bel.fwire(&format!("PERF{i}_{which}_I")),
                obel_mmcm0.fwire(&format!("PERF{i}_{which}")),
                obel_mmcm1.fwire(&format!("PERF{i}_{which}")),
            ]);
            let reg = endev.edev.chips[bel.die].row_to_reg(bel.row);
            if which == "OL"
                && endev.edev.col_lio.is_none()
                && endev.edev.col_lgt.is_none_or(|col| {
                    endev.edev.chips[bel.die].get_col_gt(col).unwrap().regs[reg]
                        == Some(GtKind::Gth)
                })
            {
                continue;
            }
            if which == "OR"
                && endev.edev.col_rio.is_none()
                && (endev.edev.col_rgt.is_none_or(|col| {
                    endev.edev.chips[bel.die].get_col_gt(col).unwrap().regs[reg]
                        == Some(GtKind::Gth)
                }) || endev.edev.disabled.contains(&DisabledPart::GtxRow(reg)))
            {
                continue;
            }
            vrf.claim_net(&[bel.fwire(&format!("PERF{i}_{which}_O"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("PERF{i}_{which}_O")),
                bel.wire(&format!("PERF{i}_{which}_I")),
            );
        }
    }

    for i in 0..14 {
        vrf.verify_net(&[
            bel.fwire(&format!("MMCM0_OUT{i}")),
            obel_mmcm0.fwire(&format!("CMT_OUT{i}")),
        ]);
        vrf.verify_net(&[
            bel.fwire(&format!("MMCM1_OUT{i}")),
            obel_mmcm1.fwire(&format!("CMT_OUT{i}")),
        ]);
    }

    for opin in [
        "MMCM0_CLKIN1_MGT",
        "MMCM0_CLKIN2_MGT",
        "MMCM1_CLKIN1_MGT",
        "MMCM1_CLKIN2_MGT",
    ] {
        vrf.claim_net(&[bel.fwire(opin)]);
        for i in 0..10 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("MGT{i}_L")));
        }
        for i in 0..10 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("MGT{i}_R")));
        }
    }

    for opin in [
        "MMCM0_CLKIN1_IO",
        "MMCM0_CLKIN2_IO",
        "MMCM0_CLKFBIN_IO",
        "MMCM1_CLKIN1_IO",
        "MMCM1_CLKIN2_IO",
        "MMCM1_CLKFBIN_IO",
    ] {
        vrf.claim_net(&[bel.fwire(opin)]);
        for i in 0..8 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("GIO{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("CCIO{i}_L")));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("CCIO{i}_R")));
        }
    }

    for opin in [
        "MMCM0_CLKIN1_HCLK",
        "MMCM0_CLKIN2_HCLK",
        "MMCM0_CLKFBIN_HCLK",
        "MMCM1_CLKIN1_HCLK",
        "MMCM1_CLKIN2_HCLK",
        "MMCM1_CLKFBIN_HCLK",
    ] {
        vrf.claim_net(&[bel.fwire(opin)]);
        for lr in ['L', 'R'] {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("{opin}_{lr}")));
            vrf.claim_net(&[bel.fwire(&format!("{opin}_{lr}"))]);
            for i in 0..12 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("{opin}_{lr}")),
                    bel.wire(&format!("HCLK{i}_{lr}_I")),
                );
            }
            for i in 0..6 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("{opin}_{lr}")),
                    bel.wire(&format!("RCLK{i}_{lr}_I")),
                );
            }
        }
    }

    for i in 0..32 {
        vrf.claim_net(&[bel.fwire(&format!("GCLK{i}_TEST"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_TEST")),
            bel.wire(&format!("GCLK{i}_INV")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_TEST")),
            bel.wire(&format!("GCLK{i}_NOINV")),
        );
        vrf.claim_net(&[bel.fwire(&format!("GCLK{i}_INV"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_INV")),
            bel.wire(&format!("GCLK{i}")),
        );
        vrf.claim_net(&[bel.fwire(&format!("GCLK{i}_NOINV"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_NOINV")),
            bel.wire(&format!("GCLK{i}")),
        );

        vrf.claim_net(&[bel.fwire(&format!("CASCO{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASCO{i}")),
            bel.wire(&format!("CASCI{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASCO{i}")),
            bel.wire(&format!("GCLK{i}_TEST")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASCO{i}")),
            bel.wire("BUFH_TEST_L"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASCO{i}")),
            bel.wire("BUFH_TEST_R"),
        );
        for j in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("CASCO{i}")),
                bel.wire(&format!("CCIO{j}_L")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("CASCO{i}")),
                bel.wire(&format!("CCIO{j}_R")),
            );
        }
        for j in 0..10 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("CASCO{i}")),
                bel.wire(&format!("MGT{j}_L")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("CASCO{i}")),
                bel.wire(&format!("MGT{j}_R")),
            );
        }
        for j in 0..14 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("CASCO{i}")),
                bel.wire(&format!("MMCM0_OUT{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("CASCO{i}")),
                bel.wire(&format!("MMCM1_OUT{j}")),
            );
        }
        for j in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("CASCO{i}")),
                bel.wire(&format!("RCLK{j}_L_I")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("CASCO{i}")),
                bel.wire(&format!("RCLK{j}_R_I")),
            );
        }
    }
    let dy = if bel.row < endev.edev.chips[bel.die].row_bufg() {
        -40
    } else {
        40
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, bels::CMT) {
        for i in 0..32 {
            vrf.verify_net(&[
                bel.fwire(&format!("CASCI{i}")),
                obel.fwire(&format!("CASCO{i}")),
            ]);
        }
    } else {
        for i in 0..32 {
            vrf.claim_net(&[bel.fwire(&format!("CASCI{i}"))]);
        }
    }

    for (col, lr) in [
        (endev.edev.col_lcio.unwrap(), 'L'),
        (endev.edev.col_rcio.unwrap(), 'R'),
    ] {
        for (i, dy) in [(0, 0), (1, 2), (2, 4), (3, 6)] {
            let obel = vrf.get_bel(bel.cell.with_cr(col, bel.row - 4 + dy).bel(bels::ILOGIC1));
            vrf.verify_net(&[
                bel.fwire(&format!("CCIO{i}_{lr}")),
                obel.fwire("CLKOUT_CMT"),
            ]);
        }

        // HCLK_IOI is not the true source, but it already did the job for us.
        let obel = vrf.get_bel(bel.cell.with_cr(col, bel.row).bel(bels::HCLK_IOI));
        for i in 0..4 {
            vrf.verify_net(&[
                bel.fwire(&format!("MGT{i}_{lr}")),
                obel.fwire(&format!("MGT{i}")),
            ]);
        }
    }

    let dy = if bel.row < endev.edev.chips[bel.die].row_bufg() {
        18
    } else {
        -20
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, bels::GCLK_BUF) {
        for i in 0..32 {
            vrf.verify_net(&[
                bel.fwire(&format!("GCLK{i}")),
                obel.fwire(&format!("GCLK{i}_O")),
            ]);
        }
        for i in 0..8 {
            vrf.verify_net(&[
                bel.fwire(&format!("GIO{i}")),
                obel.fwire(&format!("GIO{i}_O")),
            ]);
        }
    } else {
        let dy_n = if bel.row < endev.edev.chips[bel.die].row_bufg() {
            20
        } else {
            -20
        };
        let dy_s = dy_n - 2;
        for i in 0..32 {
            let obel = vrf
                .find_bel_delta(bel, 0, if i < 16 { dy_s } else { dy_n }, bels::BUFGCTRL[i])
                .unwrap();
            vrf.verify_net(&[bel.fwire(&format!("GCLK{i}")), obel.fwire("GCLK")]);
        }
        let obel = vrf.find_bel_delta(bel, 0, dy_s, bels::GIO_S).unwrap();
        for i in 0..4 {
            vrf.verify_net(&[
                bel.fwire(&format!("GIO{i}")),
                obel.fwire(&format!("GIO{i}_CMT")),
            ]);
        }
        let obel = vrf.find_bel_delta(bel, 0, dy_n, bels::GIO_N).unwrap();
        for i in 4..8 {
            vrf.verify_net(&[
                bel.fwire(&format!("GIO{i}")),
                obel.fwire(&format!("GIO{i}_CMT")),
            ]);
        }
    }
}

pub fn verify_gclk_buf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..32 {
        vrf.claim_net(&[bel.fwire(&format!("GCLK{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_O")),
            bel.wire(&format!("GCLK{i}_I")),
        );
    }
    for i in 0..8 {
        vrf.claim_net(&[bel.fwire(&format!("GIO{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GIO{i}_O")),
            bel.wire(&format!("GIO{i}_I")),
        );
    }
    let dy = if bel.row < endev.edev.chips[bel.die].row_bufg() {
        40
    } else {
        -40
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, bels::GCLK_BUF) {
        for i in 0..32 {
            vrf.verify_net(&[
                bel.fwire(&format!("GCLK{i}_I")),
                obel.fwire(&format!("GCLK{i}_O")),
            ]);
        }
        for i in 0..8 {
            vrf.verify_net(&[
                bel.fwire(&format!("GIO{i}_I")),
                obel.fwire(&format!("GIO{i}_O")),
            ]);
        }
    } else {
        let dy_n = if bel.row < endev.edev.chips[bel.die].row_bufg() {
            42
        } else {
            -40
        };
        let dy_s = dy_n - 2;
        for i in 0..32 {
            let obel = vrf
                .find_bel_delta(bel, 0, if i < 16 { dy_s } else { dy_n }, bels::BUFGCTRL[i])
                .unwrap();
            vrf.verify_net(&[bel.fwire(&format!("GCLK{i}_I")), obel.fwire("GCLK")]);
        }
        let obel = vrf.find_bel_delta(bel, 0, dy_s, bels::GIO_S).unwrap();
        for i in 0..4 {
            vrf.verify_net(&[
                bel.fwire(&format!("GIO{i}_I")),
                obel.fwire(&format!("GIO{i}_CMT")),
            ]);
        }
        let obel = vrf.find_bel_delta(bel, 0, dy_n, bels::GIO_N).unwrap();
        for i in 4..8 {
            vrf.verify_net(&[
                bel.fwire(&format!("GIO{i}_I")),
                obel.fwire(&format!("GIO{i}_CMT")),
            ]);
        }
    }
}

pub fn verify_bufgctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFGCTRL",
        &[
            ("I0", SitePinDir::In),
            ("I1", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &["I0_CKINT", "I1_CKINT", "I0_FB_TEST", "I1_FB_TEST"],
    );

    let is_b = bel.tcls == "CMT_BUFG_BOT";
    let obel_gio = vrf.find_bel_sibling(bel, if is_b { bels::GIO_S } else { bels::GIO_N });
    vrf.claim_net(&[bel.fwire("I0")]);
    vrf.claim_net(&[bel.fwire("I1")]);
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("I0_CKINT"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("I1_CKINT"));
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("I0_CASCI"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("I1_CASCI"));
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("I0_FB_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("I1_FB_TEST"));
    for i in 0..8 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I0"),
            obel_gio.wire(&format!("GIO{i}_BUFG")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I1"),
            obel_gio.wire(&format!("GIO{i}_BUFG")),
        );
    }
    let idx = bels::BUFGCTRL
        .into_iter()
        .position(|x| x == bel.slot)
        .unwrap();

    for oi in [(idx + 1) % 16, (idx + 15) % 16] {
        let obi = if is_b { oi } else { oi + 16 };
        let obel = vrf.find_bel_sibling(bel, bels::BUFGCTRL[obi]);
        vrf.claim_pip(bel.crd(), bel.wire("I0"), obel.wire("FB"));
        vrf.claim_pip(bel.crd(), bel.wire("I1"), obel.wire("FB"));
    }

    vrf.claim_net(&[bel.fwire("O")]);
    vrf.claim_net(&[bel.fwire("FB")]);
    vrf.claim_net(&[bel.fwire("GCLK")]);
    vrf.claim_pip(bel.crd(), bel.wire("FB"), bel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("GCLK"), bel.wire("O"));
}

pub fn verify_gio_s(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_delta(bel, 0, 2, bels::GIO_N).unwrap();
    for (i, col, row) in [
        (0, endev.edev.col_lcio.unwrap(), bel.row - 2),
        (1, endev.edev.col_rcio.unwrap(), bel.row - 2),
        (2, endev.edev.col_lcio.unwrap(), bel.row),
        (3, endev.edev.col_rcio.unwrap(), bel.row),
    ] {
        vrf.claim_net(&[
            bel.fwire(&format!("GIO{i}_BUFG")),
            obel.fwire(&format!("GIO{i}_BUFG")),
        ]);
        vrf.claim_net(&[bel.fwire(&format!("GIO{i}_CMT"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GIO{i}_BUFG")),
            bel.wire(&format!("GIO{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GIO{i}_CMT")),
            bel.wire(&format!("GIO{i}")),
        );
        let obel_io = vrf.get_bel(bel.cell.with_cr(col, row).bel(bels::ILOGIC1));
        vrf.verify_net(&[bel.fwire(&format!("GIO{i}")), obel_io.fwire("CLKOUT_CMT")]);
    }
}

pub fn verify_gio_n(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_delta(bel, 0, -2, bels::GIO_S).unwrap();
    for (i, col, row) in [
        (4, endev.edev.col_lcio.unwrap(), bel.row),
        (5, endev.edev.col_rcio.unwrap(), bel.row),
        (6, endev.edev.col_lcio.unwrap(), bel.row + 2),
        (7, endev.edev.col_rcio.unwrap(), bel.row + 2),
    ] {
        vrf.claim_net(&[
            bel.fwire(&format!("GIO{i}_BUFG")),
            obel.fwire(&format!("GIO{i}_BUFG")),
        ]);
        vrf.claim_net(&[bel.fwire(&format!("GIO{i}_CMT"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GIO{i}_BUFG")),
            bel.wire(&format!("GIO{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GIO{i}_CMT")),
            bel.wire(&format!("GIO{i}")),
        );
        let obel_io = vrf.get_bel(bel.cell.with_cr(col, row).bel(bels::ILOGIC1));
        vrf.verify_net(&[bel.fwire(&format!("GIO{i}")), obel_io.fwire("CLKOUT_CMT")]);
    }
}

pub fn verify_gtx(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("RXP", SitePinDir::In),
        ("RXN", SitePinDir::In),
        ("TXP", SitePinDir::Out),
        ("TXN", SitePinDir::Out),
        ("PERFCLKRX", SitePinDir::In),
        ("PERFCLKTX", SitePinDir::In),
        ("RXRECCLK", SitePinDir::Out),
        ("TXOUTCLK", SitePinDir::Out),
        ("MGTREFCLKRX0", SitePinDir::In),
        ("MGTREFCLKRX1", SitePinDir::In),
        ("MGTREFCLKTX0", SitePinDir::In),
        ("MGTREFCLKTX1", SitePinDir::In),
        ("NORTHREFCLKRX0", SitePinDir::In),
        ("NORTHREFCLKRX1", SitePinDir::In),
        ("NORTHREFCLKTX0", SitePinDir::In),
        ("NORTHREFCLKTX1", SitePinDir::In),
        ("SOUTHREFCLKRX0", SitePinDir::In),
        ("SOUTHREFCLKRX1", SitePinDir::In),
        ("SOUTHREFCLKTX0", SitePinDir::In),
        ("SOUTHREFCLKTX1", SitePinDir::In),
    ];
    vrf.verify_bel(bel, "GTXE1", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.fwire(pin)]);
    }
    let (rxp, rxn, txp, txn) = match bel.slot {
        bels::GTX0 => (
            bels::IPAD_RXP0,
            bels::IPAD_RXN0,
            bels::OPAD_TXP0,
            bels::OPAD_TXN0,
        ),
        bels::GTX1 => (
            bels::IPAD_RXP1,
            bels::IPAD_RXN1,
            bels::OPAD_TXP1,
            bels::OPAD_TXN1,
        ),
        bels::GTX2 => (
            bels::IPAD_RXP2,
            bels::IPAD_RXN2,
            bels::OPAD_TXP2,
            bels::OPAD_TXN2,
        ),
        bels::GTX3 => (
            bels::IPAD_RXP3,
            bels::IPAD_RXN3,
            bels::OPAD_TXP3,
            bels::OPAD_TXN3,
        ),
        _ => unreachable!(),
    };
    for (pin, slot) in [("RXP", rxp), ("RXN", rxn)] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("O"));
    }
    for (pin, slot) in [("TXP", txp), ("TXN", txn)] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire(pin));
    }

    for pin in ["RXRECCLK", "TXOUTCLK"] {
        vrf.claim_net(&[bel.fwire_far(pin)]);
        vrf.claim_pip(bel.crd(), bel.wire_far(pin), bel.wire(pin));
    }

    let obel = vrf.find_bel_sibling(bel, bels::HCLK_GTX);
    for (orx, otx, pin) in [
        ("PERFCLKRX", "PERFCLKTX", "PERFCLK"),
        ("MGTREFCLKRX0", "MGTREFCLKTX0", "MGTREFCLKOUT0"),
        ("MGTREFCLKRX1", "MGTREFCLKTX1", "MGTREFCLKOUT1"),
        ("SOUTHREFCLKRX0", "SOUTHREFCLKTX0", "SOUTHREFCLKOUT0"),
        ("SOUTHREFCLKRX1", "SOUTHREFCLKTX1", "SOUTHREFCLKOUT1"),
        ("NORTHREFCLKRX0", "NORTHREFCLKTX0", "NORTHREFCLKIN0"),
        ("NORTHREFCLKRX1", "NORTHREFCLKTX1", "NORTHREFCLKIN1"),
    ] {
        vrf.verify_net(&[bel.fwire(pin), obel.fwire(pin)]);
        vrf.claim_pip(bel.crd(), bel.wire(orx), bel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire(otx), bel.wire(pin));
    }
}

pub fn verify_bufds(vrf: &mut Verifier, bel: &BelContext<'_>) {
    if bel.tcls.starts_with("GTX") {
        let pins = [
            ("I", SitePinDir::In),
            ("IB", SitePinDir::In),
            ("O", SitePinDir::Out),
            ("ODIV2", SitePinDir::Out),
            ("CLKTESTSIG", SitePinDir::In),
        ];
        vrf.verify_bel(bel, "IBUFDS_GTXE1", &pins, &["CLKTESTSIG_INT"]);
        for (pin, _) in pins {
            vrf.claim_net(&[bel.fwire(pin)]);
        }
        for (slot, pin, oslot) in [
            (bels::BUFDS0, "I", bels::IPAD_CLKP0),
            (bels::BUFDS0, "IB", bels::IPAD_CLKN0),
            (bels::BUFDS1, "I", bels::IPAD_CLKP1),
            (bels::BUFDS1, "IB", bels::IPAD_CLKN1),
        ] {
            if bel.slot != slot {
                continue;
            }
            let obel = vrf.find_bel_sibling(bel, oslot);
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("O"));
        }

        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKTESTSIG"),
            bel.wire("CLKTESTSIG_INT"),
        );

        vrf.claim_net(&[bel.fwire("HCLK_OUT")]);
        vrf.claim_pip(bel.crd(), bel.wire("HCLK_OUT"), bel.wire("O"));
        vrf.claim_pip(bel.crd(), bel.wire("HCLK_OUT"), bel.wire("ODIV2"));
        vrf.claim_pip(bel.crd(), bel.wire("HCLK_OUT"), bel.wire("CLKTESTSIG_INT"));
    } else {
        let pins = [
            ("I", SitePinDir::In),
            ("IB", SitePinDir::In),
            ("O", SitePinDir::Out),
        ];
        vrf.verify_bel(bel, "IBUFDS_GTHE1", &pins, &[]);
        for (pin, _) in pins {
            vrf.claim_net(&[bel.fwire(pin)]);
        }
        for (pin, oslot) in [("I", bels::IPAD_CLKP0), ("IB", bels::IPAD_CLKN0)] {
            let obel = vrf.find_bel_sibling(bel, oslot);
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("O"));
        }

        vrf.claim_net(&[bel.fwire_far("O")]);
        vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
    }
}

pub fn verify_hclk_gtx(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_net(&[bel.fwire("PERFCLK")]);
    let obel_cmt = vrf.get_bel(bel.cell.with_col(endev.edev.col_cfg).bel(bels::CMT));
    let which = if bel.col.to_idx() == 0 { "OL" } else { "OR" };
    for i in 0..4 {
        vrf.verify_net(&[
            bel.fwire(&format!("PERF{i}")),
            obel_cmt.fwire(&format!("PERF{i}_{which}_O")),
        ]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("PERFCLK"),
            bel.wire(&format!("PERF{i}")),
        );
    }

    for (i, slot, pin, lpin) in [
        (0, bels::GTX0, "RXRECCLK", Some("RXRECCLK0")),
        (1, bels::GTX1, "RXRECCLK", Some("RXRECCLK1")),
        (2, bels::GTX0, "TXOUTCLK", Some("TXOUTCLK0")),
        (3, bels::GTX1, "TXOUTCLK", Some("TXOUTCLK1")),
        (4, bels::BUFDS0, "HCLK_OUT", None),
        (5, bels::BUFDS1, "HCLK_OUT", None),
        (6, bels::GTX2, "RXRECCLK", Some("RXRECCLK2")),
        (7, bels::GTX3, "RXRECCLK", Some("RXRECCLK3")),
        (8, bels::GTX2, "TXOUTCLK", Some("TXOUTCLK2")),
        (9, bels::GTX3, "TXOUTCLK", Some("TXOUTCLK3")),
    ] {
        let mpin = format!("MGT{i}");
        vrf.claim_net(&[bel.fwire(&mpin)]);
        let obel = vrf.find_bel_sibling(bel, slot);
        if let Some(lpin) = lpin {
            vrf.verify_net(&[bel.fwire(lpin), obel.fwire_far(pin)]);
            vrf.claim_pip(bel.crd(), bel.wire(&mpin), bel.wire(lpin));
        } else {
            vrf.claim_pip(bel.crd(), bel.wire(&mpin), obel.wire(pin));
        }
    }

    for i in 0..2 {
        vrf.claim_net(&[bel.fwire(&format!("MGTREFCLKOUT{i}"))]);
        let obel = vrf.find_bel_sibling(bel, bels::BUFDS[i]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MGTREFCLKOUT{i}")),
            obel.wire("O"),
        );

        vrf.claim_net(&[bel.fwire(&format!("SOUTHREFCLKOUT{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("SOUTHREFCLKOUT{i}")),
            bel.wire("MGTREFCLKIN0"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("SOUTHREFCLKOUT{i}")),
            bel.wire("MGTREFCLKIN1"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("SOUTHREFCLKOUT{i}")),
            bel.wire(&format!("SOUTHREFCLKIN{i}")),
        );
        vrf.claim_net(&[bel.fwire(&format!("NORTHREFCLKOUT{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("NORTHREFCLKOUT{i}")),
            bel.wire("MGTREFCLKOUT0"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("NORTHREFCLKOUT{i}")),
            bel.wire("MGTREFCLKOUT1"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("NORTHREFCLKOUT{i}")),
            bel.wire(&format!("NORTHREFCLKIN{i}")),
        );
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -40, bels::HCLK_GTX) {
            vrf.verify_net(&[
                bel.fwire(&format!("NORTHREFCLKIN{i}")),
                obel.fwire(&format!("NORTHREFCLKOUT{i}")),
            ]);
        } else {
            vrf.claim_net(&[bel.fwire(&format!("NORTHREFCLKIN{i}"))]);
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 40, bels::HCLK_GTX) {
            vrf.verify_net(&[
                bel.fwire(&format!("SOUTHREFCLKIN{i}")),
                obel.fwire(&format!("SOUTHREFCLKOUT{i}")),
            ]);
            vrf.verify_net(&[
                bel.fwire(&format!("MGTREFCLKIN{i}")),
                obel.fwire(&format!("MGTREFCLKOUT{i}")),
            ]);
        } else {
            vrf.claim_net(&[bel.fwire(&format!("SOUTHREFCLKIN{i}"))]);
            vrf.claim_net(&[bel.fwire(&format!("MGTREFCLKIN{i}"))]);
        }
    }
}

pub fn verify_gth(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    for i in 0..4 {
        pins.extend([
            (format!("RXP{i}"), SitePinDir::In),
            (format!("RXN{i}"), SitePinDir::In),
            (format!("TXP{i}"), SitePinDir::Out),
            (format!("TXN{i}"), SitePinDir::Out),
            (format!("RXUSERCLKOUT{i}"), SitePinDir::Out),
            (format!("TXUSERCLKOUT{i}"), SitePinDir::Out),
        ]);
    }
    pins.extend([
        ("REFCLK".to_string(), SitePinDir::In),
        ("TSTPATH".to_string(), SitePinDir::Out),
        ("TSTREFCLKOUT".to_string(), SitePinDir::Out),
    ]);
    let pin_refs: Vec<_> = pins.iter().map(|&(ref p, d)| (&p[..], d)).collect();
    vrf.verify_bel(bel, "GTHE1_QUAD", &pin_refs, &["GREFCLK"]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.fwire(&pin)]);
    }
    for i in 0..4 {
        let obel = vrf.find_bel_sibling(bel, bels::IPAD_RXP[i]);
        vrf.claim_pip(bel.crd(), bel.wire(&format!("RXP{i}")), obel.wire("O"));
        let obel = vrf.find_bel_sibling(bel, bels::IPAD_RXN[i]);
        vrf.claim_pip(bel.crd(), bel.wire(&format!("RXN{i}")), obel.wire("O"));
        let obel = vrf.find_bel_sibling(bel, bels::OPAD_TXP[i]);
        vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire(&format!("TXP{i}")));
        let obel = vrf.find_bel_sibling(bel, bels::OPAD_TXN[i]);
        vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire(&format!("TXN{i}")));
    }

    vrf.claim_net(&[bel.fwire_far("REFCLK")]);
    vrf.claim_pip(bel.crd(), bel.wire("REFCLK"), bel.wire_far("REFCLK"));
    vrf.claim_pip(bel.crd(), bel.wire_far("REFCLK"), bel.wire("GREFCLK"));
    vrf.claim_pip(bel.crd(), bel.wire_far("REFCLK"), bel.wire("REFCLK_IN"));
    vrf.claim_pip(bel.crd(), bel.wire_far("REFCLK"), bel.wire("REFCLK_SOUTH"));
    vrf.claim_pip(bel.crd(), bel.wire_far("REFCLK"), bel.wire("REFCLK_NORTH"));
    let obel = vrf.find_bel_sibling(bel, bels::BUFDS0);
    vrf.verify_net(&[bel.fwire("REFCLK_IN"), obel.fwire_far("O")]);
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 40, bels::GTH_QUAD) {
        vrf.claim_net(&[bel.fwire_far("REFCLK_UP")]);
        vrf.claim_pip(bel.crd(), bel.wire("REFCLK_UP"), bel.wire_far("REFCLK"));
        vrf.verify_net(&[bel.fwire("REFCLK_SOUTH"), obel.fwire("REFCLK_DN")]);
    } else {
        vrf.claim_net(&[bel.fwire("REFCLK_SOUTH")]);
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -40, bels::GTH_QUAD) {
        vrf.claim_net(&[bel.fwire_far("REFCLK_DN")]);
        vrf.claim_pip(bel.crd(), bel.wire("REFCLK_DN"), bel.wire_far("REFCLK"));
        vrf.verify_net(&[bel.fwire("REFCLK_NORTH"), obel.fwire("REFCLK_UP")]);
    } else {
        vrf.claim_net(&[bel.fwire("REFCLK_NORTH")]);
    }
    for (opin, ipin) in [
        ("MGT0", "RXUSERCLKOUT0"),
        ("MGT1", "RXUSERCLKOUT1"),
        ("MGT2", "TXUSERCLKOUT0"),
        ("MGT3", "TXUSERCLKOUT1"),
        ("MGT4", "TSTPATH"),
        ("MGT5", "TSTREFCLKOUT"),
        ("MGT6", "RXUSERCLKOUT2"),
        ("MGT7", "RXUSERCLKOUT3"),
        ("MGT8", "TXUSERCLKOUT2"),
        ("MGT9", "TXUSERCLKOUT3"),
    ] {
        vrf.claim_net(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
    }
}

pub fn verify_hclk_gth(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, bels::GTH_QUAD);
    for i in 0..10 {
        vrf.claim_net(&[bel.fwire(&format!("MGT{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MGT{i}")),
            bel.wire(&format!("MGT{i}_I")),
        );
        vrf.verify_net(&[
            bel.fwire(&format!("MGT{i}_I")),
            obel.fwire(&format!("MGT{i}")),
        ]);
    }
}

pub fn verify_mgt_buf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let reg = endev.edev.chips[bel.die].row_to_reg(bel.row);
    if endev.edev.disabled.contains(&DisabledPart::GtxRow(reg)) && endev.edev.col_rio.is_none() {
        return;
    }
    for i in 0..10 {
        vrf.claim_net(&[bel.fwire(&format!("MGT{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MGT{i}_O")),
            bel.wire(&format!("MGT{i}_I")),
        );
    }
    let is_l = bel.col < endev.edev.col_cfg;
    let dx = if is_l { -1 } else { 1 };
    let gtcol = if is_l {
        endev.edev.chips[bel.die].columns.first_id().unwrap()
    } else {
        endev.edev.chips[bel.die].columns.last_id().unwrap()
    };
    if let Some(obel) = vrf.find_bel_walk(bel, dx, 0, bels::MGT_BUF) {
        for i in 0..10 {
            vrf.verify_net(&[
                bel.fwire(&format!("MGT{i}_I")),
                obel.fwire(&format!("MGT{i}_O")),
            ]);
        }
    } else if let Some(obel) = vrf
        .find_bel(bel.cell.with_col(gtcol).bel(bels::HCLK_GTX))
        .or_else(|| vrf.find_bel(bel.cell.with_col(gtcol).bel(bels::HCLK_GTH)))
    {
        for i in 0..10 {
            vrf.verify_net(&[
                bel.fwire(&format!("MGT{i}_I")),
                obel.fwire(&format!("MGT{i}")),
            ]);
        }
    } else {
        let scol = if is_l {
            endev.edev.col_lio
        } else {
            endev.edev.col_rio
        }
        .unwrap();
        let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::HCLK_IOI));
        for i in 0..10 {
            vrf.verify_net(&[
                bel.fwire(&format!("MGT{i}_I")),
                obel.fwire(&format!("MGT{i}")),
            ]);
        }
    }
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let slot_name = endev.edev.db.bel_slots.key(bel.slot);
    match bel.slot {
        bels::SLICE0 | bels::SLICE1 => verify_slice(vrf, bel),
        bels::DSP0 | bels::DSP1 => verify_dsp(vrf, bel),
        bels::TIEOFF_DSP => verify_tieoff(vrf, bel),
        bels::BRAM_F => verify_bram_f(vrf, bel),
        bels::BRAM_H0 => vrf.verify_bel(bel, "FIFO18E1", &[], &[]),
        bels::BRAM_H1 => verify_bram_h1(vrf, bel),
        bels::PMVBRAM => vrf.verify_bel(bel, "PMVBRAM", &[], &[]),
        bels::EMAC => vrf.verify_bel(bel, "TEMAC_SINGLE", &[], &[]),
        bels::PCIE => vrf.verify_bel(bel, "PCIE_2_0", &[], &[]),

        bels::HCLK => verify_hclk(endev, vrf, bel),
        bels::HCLK_QBUF => verify_hclk_qbuf(endev, vrf, bel),

        _ if slot_name.starts_with("BUFIO") => verify_bufiodqs(vrf, bel),
        _ if slot_name.starts_with("BUFO") => verify_bufo(vrf, bel),
        _ if slot_name.starts_with("BUFR") => verify_bufr(vrf, bel),
        bels::IDELAYCTRL => verify_idelayctrl(vrf, bel),
        bels::DCI => verify_dci(vrf, bel),
        bels::HCLK_IOI => verify_hclk_ioi(endev, vrf, bel),

        bels::ILOGIC0 | bels::ILOGIC1 => verify_ilogic(endev, vrf, bel),
        bels::OLOGIC0 | bels::OLOGIC1 => verify_ologic(vrf, bel),
        bels::IODELAY0 | bels::IODELAY1 => verify_iodelay(vrf, bel),
        bels::IOB0 | bels::IOB1 => verify_iob(vrf, bel),
        bels::IOI => verify_ioi(endev, vrf, bel),

        _ if slot_name.starts_with("BSCAN") => vrf.verify_bel(bel, "BSCAN", &[], &[]),
        _ if slot_name.starts_with("ICAP") => vrf.verify_bel(bel, "ICAP", &[], &[]),
        bels::PMV0 | bels::PMV1 => vrf.verify_bel(bel, "PMV", &[], &[]),
        bels::STARTUP
        | bels::CAPTURE
        | bels::FRAME_ECC
        | bels::EFUSE_USR
        | bels::USR_ACCESS
        | bels::DNA_PORT
        | bels::DCIRESET
        | bels::CFG_IO_ACCESS
        | bels::PMVIOB
        | bels::PPR_FRAME
        | bels::GLOBALSIG => vrf.verify_bel(bel, slot_name, &[], &[]),
        bels::SYSMON => verify_sysmon(endev, vrf, bel),
        _ if slot_name.starts_with("IPAD") => verify_ipad(vrf, bel),
        _ if slot_name.starts_with("OPAD") => verify_opad(vrf, bel),

        _ if slot_name.starts_with("BUFHCE") => verify_bufhce(vrf, bel),
        bels::MMCM0 | bels::MMCM1 => verify_mmcm(vrf, bel),
        bels::CMT => verify_cmt(endev, vrf, bel),
        bels::GCLK_BUF => verify_gclk_buf(endev, vrf, bel),
        _ if slot_name.starts_with("BUFGCTRL") => verify_bufgctrl(vrf, bel),
        bels::GIO_S => verify_gio_s(endev, vrf, bel),
        bels::GIO_N => verify_gio_n(endev, vrf, bel),

        bels::GTX0 | bels::GTX1 | bels::GTX2 | bels::GTX3 => verify_gtx(vrf, bel),
        bels::BUFDS0 | bels::BUFDS1 => verify_bufds(vrf, bel),
        bels::HCLK_GTX => verify_hclk_gtx(endev, vrf, bel),
        bels::GTH_QUAD => verify_gth(vrf, bel),
        bels::HCLK_GTH => verify_hclk_gth(vrf, bel),
        bels::MGT_BUF => verify_mgt_buf(endev, vrf, bel),

        _ => println!("MEOW {} {:?}", bel.slot, bel.name),
    }
}

fn verify_extra(_endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    vrf.kill_stub_out_cond("IOI_PREAMBLE_DGLITCH0");
    vrf.kill_stub_out_cond("IOI_PREAMBLE_DGLITCH1");
    vrf.kill_stub_out_cond("IOI_PREAMBLE_DGLITCH2");
    vrf.kill_stub_out_cond("IOI_PREAMBLE_DGLITCH3");
    vrf.kill_stub_out_cond("IOI_INT_BUFR_CLR_B_S");
    vrf.kill_stub_out_cond("IOI_INT_BUFR_CLR_B_N");
    vrf.kill_stub_out_cond("IOI_INT_BUFR_CE_B_S");
    vrf.kill_stub_out_cond("IOI_INT_BUFR_CE_B_N");
    vrf.kill_stub_out_cond("IOI_INT_RCLKMUX_B_S");
    vrf.kill_stub_out_cond("IOI_INT_RCLKMUX_B_N");
    for i in 0..40 {
        vrf.kill_stub_out_cond(&format!("CMT_TOP_IMUX_B_2_BUFG{i}"));
        vrf.kill_stub_out_cond(&format!("CMT_BOT_IMUX_B_2_BUFG{i}"));
    }
    vrf.kill_stub_out_cond("GTX_IBUFDSMGTCEB0");
    vrf.kill_stub_out_cond("GTX_IBUFDSMGTCEB1");
    vrf.kill_stub_out_cond("GTX_CLKTESTSIG2");
    vrf.kill_stub_out_cond("GTX_CLKTESTSIG3");
    vrf.kill_stub_out_cond("GTX_LEFT_IBUFDSMGTCEB0");
    vrf.kill_stub_out_cond("GTX_LEFT_IBUFDSMGTCEB1");
    vrf.kill_stub_out_cond("GTX_LEFT_CLKTESTSIG2");
    vrf.kill_stub_out_cond("GTX_LEFT_CLKTESTSIG3");
    for &crd in vrf.rd.tiles_by_kind_name("T_TERM_INT") {
        let tile = &vrf.rd.tiles[&crd];
        let otile = &vrf.rd.tiles[&crd.delta(0, -1)];
        if vrf.rd.tile_kinds.key(otile.kind) == "CENTER_SPACE2" {
            let tk = &vrf.rd.tile_kinds[tile.kind];
            for &w in tk.wires.keys() {
                if vrf.rd.lookup_wire_raw(crd, w).is_some() {
                    vrf.claim_net(&[(crd, &vrf.rd.wires[w])]);
                }
            }
        }
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    prjcombine_re_xilinx_rdverify::verify(
        rd,
        &endev.ngrid,
        |_| (),
        |vrf, bel| verify_bel(endev, vrf, bel),
        |vrf| verify_extra(endev, vrf),
    );
}
