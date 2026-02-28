use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, WireSlotIdExt},
    grid::BelCoord,
};
use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{RawWireCoord, SitePinDir, Verifier};
use prjcombine_virtex4::{
    chip::ColumnKind,
    defs::{
        bcls, bslots,
        virtex6::{tcls, wires},
    },
    expanded::ExpandedDevice,
};

fn verify_slice(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::SLICE.index_of(bcrd.slot).unwrap();
    let kind = if edev.chips[bcrd.die].columns[bcrd.col] == ColumnKind::ClbLM && idx == 0 {
        "SLICEM"
    } else {
        "SLICEL"
    };
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_in("CIN")
        .extra_out_claim("COUT");
    if let Some(cell) = edev.cell_delta(bcrd.cell, 0, -1)
        && let obel = cell.bel(bcrd.slot)
        && edev.has_bel(obel)
    {
        bel.claim_net(&[bel.wire("CIN"), bel.bel_wire_far(obel, "COUT")]);
        bel.claim_pip(bel.bel_wire_far(obel, "COUT"), bel.bel_wire(obel, "COUT"));
    } else {
        bel.claim_net(&[bel.wire("CIN")]);
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
        if bel.slot == bslots::DSP[0] {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bslots::DSP[1]) {
                vrf.claim_net(&[bel.wire(ipin), obel.wire_far(opin)]);
                vrf.claim_pip(obel.wire_far(opin), obel.wire(opin));
            } else {
                vrf.claim_net(&[bel.wire(ipin)]);
            }
        } else {
            vrf.claim_net(&[bel.wire(ipin)]);
            let obel = vrf.find_bel_sibling(bel, bslots::DSP[0]);
            vrf.claim_pip(bel.wire(ipin), obel.wire(opin));
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
    vrf.verify_legacy_bel(bel, "RAMBFIFO36E1", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bel.slot) {
        for (ipin, opin) in [("CASCADEINA", "CASCADEOUTA"), ("CASCADEINB", "CASCADEOUTB")] {
            vrf.verify_net(&[bel.wire(ipin), obel.wire_far(opin)]);
            vrf.claim_pip(obel.wire_far(opin), obel.wire(opin));
        }
    }
}

fn verify_bram_h1(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
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
    vrf.verify_legacy_bel(bel, "RAMB18E1", &pin_refs, &[]);
    for pin in pins {
        vrf.claim_net(&[bel.wire(&pin)]);
    }
}

fn verify_hclk_io_int(vrf: &mut Verifier, bcrd: BelCoord) {
    for i in 0..2 {
        let mut bel = vrf
            .verify_bel(bcrd)
            .sub(i)
            .kind("BUFO")
            .skip_auto()
            .extra_in_rename("I", format!("BUFO{i}_I"))
            .extra_out_claim_rename("O", format!("BUFO{i}_O"));
        bel.claim_pip(
            bel.wire(&format!("BUFO{i}_OCLK")),
            bel.wire(&format!("BUFO{i}_O")),
        );
        bel.commit();
    }
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

fn verify_ilogic(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::ILOGIC.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("ILOGICE1")
        .skip_out(bcls::ILOGIC::CLKPAD)
        .extra_in_claim("OCLK")
        .extra_in_claim("OCLKB")
        .extra_in_claim("D")
        .extra_in_claim("DDLY")
        .extra_in_claim("OFB")
        .extra_in_claim("TFB")
        .extra_in_claim("SHIFTIN1")
        .extra_in_claim("SHIFTIN2")
        .extra_out_claim("SHIFTOUT1")
        .extra_out_claim("SHIFTOUT2")
        .extra_in_claim("REV");

    let obel_ologic = bcrd.bel(bslots::OLOGIC[idx]);
    bel.claim_pip(bel.wire("OCLK"), bel.bel_wire(obel_ologic, "CLK"));
    bel.claim_pip(bel.wire("OCLKB"), bel.bel_wire(obel_ologic, "CLK"));
    bel.claim_pip(bel.wire("OCLKB"), bel.bel_wire(obel_ologic, "CLKB"));
    bel.claim_pip(bel.wire("OFB"), bel.bel_wire(obel_ologic, "OFB"));
    bel.claim_pip(bel.wire("TFB"), bel.bel_wire(obel_ologic, "TFB"));

    let obel_iodelay = bcrd.bel(bslots::IODELAY[idx]);
    bel.claim_pip(bel.wire("DDLY"), bel.bel_wire(obel_iodelay, "DATAOUT"));

    let obel_iob = bcrd.bel(bslots::IOB[idx]);
    bel.claim_pip(bel.wire("D"), bel.wire("IOB_I_BUF"));
    bel.claim_net(&[bel.wire("IOB_I_BUF")]);
    bel.claim_pip(bel.wire("IOB_I_BUF"), bel.wire("IOB_I"));
    bel.verify_net(&[bel.wire("IOB_I"), bel.bel_wire(obel_iob, "I")]);

    if bcrd.slot == bslots::ILOGIC[0] {
        let obel = bcrd.bel(bslots::ILOGIC[1]);
        bel.claim_pip(bel.wire("SHIFTIN1"), bel.bel_wire(obel, "SHIFTOUT1"));
        bel.claim_pip(bel.wire("SHIFTIN2"), bel.bel_wire(obel, "SHIFTOUT2"));
    }

    let is_rclk = matches!(bcrd.row.to_idx() % 40, 16 | 18 | 20 | 22);
    let is_inner = bcrd.col == edev.col_io_iw.unwrap() || bcrd.col == edev.col_io_ie.unwrap();
    let is_gclk = is_inner
        && (bcrd.row == edev.chips[bcrd.die].row_bufg() - 4
            || bcrd.row == edev.chips[bcrd.die].row_bufg() - 2
            || bcrd.row == edev.chips[bcrd.die].row_bufg()
            || bcrd.row == edev.chips[bcrd.die].row_bufg() + 2);
    if (is_rclk || is_gclk) && bcrd.slot == bslots::ILOGIC[1] {
        bel.claim_pip(bel.wire("CLKPAD"), bel.wire("O"));
    }

    bel.commit();
}

fn verify_ologic(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::OLOGIC.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("OLOGICE1")
        .skip_in(bcls::OLOGIC::CLK)
        .skip_in(bcls::OLOGIC::CLKB)
        .skip_out(bcls::OLOGIC::TFB)
        .extra_in_claim_rename("CLK", "CLK_FAKE")
        .extra_in_claim_rename("CLKB", "CLKB_FAKE")
        .extra_in_claim("CLKPERFDELAY")
        .extra_out_claim("OFB")
        .extra_out_claim_rename("TFB", "TFB_FAKE")
        .extra_out_claim("OQ")
        .extra_out_claim("TQ")
        .extra_in_claim("SHIFTIN1")
        .extra_in_claim("SHIFTIN2")
        .extra_out_claim("SHIFTOUT1")
        .extra_out_claim("SHIFTOUT2")
        .extra_in_claim("REV");

    bel.claim_pip(bel.wire("CLK_FAKE"), bel.wire("CLK"));
    bel.claim_pip(bel.wire("CLKB_FAKE"), bel.wire("CLK"));
    bel.claim_pip(bel.wire("CLKB_FAKE"), bel.wire("CLKB"));

    let obel_iodelay = bcrd.bel(bslots::IODELAY[idx]);
    bel.claim_pip(
        bel.wire("CLKPERFDELAY"),
        bel.bel_wire(obel_iodelay, "DATAOUT"),
    );

    bel.claim_pip(bel.wire("TFB"), bel.wire("TFB_FAKE"));

    let obel_iob = bcrd.bel(bslots::IOB[idx]);
    bel.claim_pip(bel.wire("IOB_T"), bel.wire("TQ"));
    bel.claim_pip(bel.wire("IOB_O"), bel.wire("OQ"));
    bel.claim_pip(bel.wire("IOB_O"), bel.bel_wire(obel_iodelay, "DATAOUT"));
    bel.verify_net(&[bel.wire("IOB_O"), bel.bel_wire(obel_iob, "O")]);
    bel.verify_net(&[bel.wire("IOB_T"), bel.bel_wire(obel_iob, "T")]);

    if bcrd.slot == bslots::OLOGIC[1] {
        let obel = bcrd.bel(bslots::OLOGIC[0]);
        bel.claim_pip(bel.wire("SHIFTIN1"), bel.bel_wire(obel, "SHIFTOUT1"));
        bel.claim_pip(bel.wire("SHIFTIN2"), bel.bel_wire(obel, "SHIFTOUT2"));
    }
    bel.commit();
}

fn verify_iodelay(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IODELAY.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("IODELAYE1")
        .extra_in_claim("CLKIN")
        .extra_in_claim("IDATAIN")
        .extra_in_claim("ODATAIN")
        .extra_out_claim("DATAOUT")
        .extra_in_claim("T");

    let obel_ilogic = bcrd.bel(bslots::ILOGIC[idx]);
    bel.claim_pip(bel.wire("IDATAIN"), bel.bel_wire(obel_ilogic, "IOB_I_BUF"));

    let obel_ologic = bcrd.bel(bslots::OLOGIC[idx]);
    bel.claim_pip(bel.wire("CLKIN"), bel.bel_wire(obel_ologic, "CLK"));
    bel.claim_pip(bel.wire("ODATAIN"), bel.bel_wire(obel_ologic, "OFB"));
    bel.claim_pip(bel.wire("T"), bel.bel_wire(obel_ologic, "TFB_FAKE"));

    bel.commit();
}

fn verify_iob(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IOB.index_of(bcrd.slot).unwrap();
    let kind = match idx {
        1 => "IOBM",
        0 => "IOBS",
        _ => unreachable!(),
    };
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_out_claim("I")
        .extra_in_claim("O")
        .extra_in_claim("T")
        .extra_in_claim("O_IN")
        .extra_out_claim("O_OUT")
        .extra_in_claim("DIFFO_IN")
        .extra_out_claim("DIFFO_OUT")
        .extra_in_claim("DIFFI_IN")
        .extra_out_claim("PADOUT");
    if kind == "IOBM" {
        bel = bel.extra_in_claim("DIFF_TERM_INT_EN");
    }
    let oslot = bslots::IOB[idx ^ 1];
    let obel = bcrd.bel(oslot);
    if kind == "IOBS" {
        bel.claim_pip(bel.wire("O_IN"), bel.bel_wire(obel, "O_OUT"));
        bel.claim_pip(bel.wire("DIFFO_IN"), bel.bel_wire(obel, "DIFFO_OUT"));
    }
    bel.claim_pip(bel.wire("DIFFI_IN"), bel.bel_wire(obel, "PADOUT"));
    bel.commit();
}

fn verify_pll(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("MMCM_ADV")
        .skip_in(bcls::PLL_V6::CLKIN_CASC)
        .skip_in(bcls::PLL_V6::CLKFB_CASC);
    bel.claim_net(&[bel.wire("CLKFB")]);
    bel.claim_pip(bel.wire("CLKFB"), bel.wire("CLKFBOUT"));
    bel.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFB"));
    bel.claim_pip(bel.wire("CLKFBIN"), bel.wire("CLKFB_CASC"));
    bel.claim_pip(bel.wire("CLKIN1"), bel.wire("CLKIN_CASC"));
    bel.commit()
}

fn verify_sysmon(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("SYSMON")
        .ipad("VP", 1)
        .ipad("VN", 2);
    for i in 0..16 {
        let vauxp = &format!("VAUXP{i}");
        let vauxn = &format!("VAUXN{i}");
        bel = bel.extra_in_claim(vauxp).extra_in_claim(vauxn);
        let Some((iop, _)) = edev.get_sysmon_vaux(bcrd.cell, i) else {
            continue;
        };
        bel.claim_pip(bel.wire(vauxp), bel.wire_far(vauxp));
        bel.claim_pip(bel.wire(vauxn), bel.wire_far(vauxn));
        let obel = iop.cell.bel(bslots::IOB[1]);
        bel.claim_net(&[bel.wire_far(vauxp), bel.bel_wire(obel, "MONITOR")]);
        bel.claim_pip(bel.bel_wire(obel, "MONITOR"), bel.bel_wire(obel, "PADOUT"));
        let obel = iop.cell.bel(bslots::IOB[0]);
        bel.claim_net(&[bel.wire_far(vauxn), bel.bel_wire(obel, "MONITOR")]);
        bel.claim_pip(bel.bel_wire(obel, "MONITOR"), bel.bel_wire(obel, "PADOUT"));
    }
    bel.commit();
}

fn verify_ipad(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_net(&[bel.wire("O")]);
}

fn verify_opad(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "OPAD", &[("I", SitePinDir::In)], &[]);
    vrf.claim_net(&[bel.wire("I")]);
}

pub fn verify_gtx(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::GTX.index_of(bel.slot).unwrap();
    let pins = [
        ("RXP", SitePinDir::In),
        ("RXN", SitePinDir::In),
        ("TXP", SitePinDir::Out),
        ("TXN", SitePinDir::Out),
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
    vrf.verify_legacy_bel(bel, "GTXE1", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    let rxp = bslots::IPAD_RXP[idx];
    let rxn = bslots::IPAD_RXN[idx];
    let txp = bslots::OPAD_TXP[idx];
    let txn = bslots::OPAD_TXN[idx];
    for (pin, slot) in [("RXP", rxp), ("RXN", rxn)] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.wire(pin), obel.wire("O"));
    }
    for (pin, slot) in [("TXP", txp), ("TXN", txn)] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(obel.wire("I"), bel.wire(pin));
    }

    let obel = vrf.find_bel_sibling(bel, bslots::HCLK_GTX);
    for (orx, otx, pin) in [
        ("MGTREFCLKRX0", "MGTREFCLKTX0", "MGTREFCLKOUT0"),
        ("MGTREFCLKRX1", "MGTREFCLKTX1", "MGTREFCLKOUT1"),
        ("SOUTHREFCLKRX0", "SOUTHREFCLKTX0", "SOUTHREFCLKOUT0"),
        ("SOUTHREFCLKRX1", "SOUTHREFCLKTX1", "SOUTHREFCLKOUT1"),
        ("NORTHREFCLKRX0", "NORTHREFCLKTX0", "NORTHREFCLKIN0"),
        ("NORTHREFCLKRX1", "NORTHREFCLKTX1", "NORTHREFCLKIN1"),
    ] {
        vrf.verify_net(&[bel.wire(pin), obel.wire(pin)]);
        vrf.claim_pip(bel.wire(orx), bel.wire(pin));
        vrf.claim_pip(bel.wire(otx), bel.wire(pin));
    }
}

pub fn verify_bufds(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    if bel.tile.class == tcls::GTX {
        let pins = [
            ("I", SitePinDir::In),
            ("IB", SitePinDir::In),
            ("O", SitePinDir::Out),
            ("ODIV2", SitePinDir::Out),
            ("CLKTESTSIG", SitePinDir::In),
        ];
        vrf.verify_legacy_bel(bel, "IBUFDS_GTXE1", &pins, &["CLKTESTSIG_INT", "HCLK_OUT"]);
        for (pin, _) in pins {
            vrf.claim_net(&[bel.wire(pin)]);
        }
        for (slot, pin, oslot) in [
            (bslots::BUFDS[0], "I", bslots::IPAD_CLKP[0]),
            (bslots::BUFDS[0], "IB", bslots::IPAD_CLKN[0]),
            (bslots::BUFDS[1], "I", bslots::IPAD_CLKP[1]),
            (bslots::BUFDS[1], "IB", bslots::IPAD_CLKN[1]),
        ] {
            if bel.slot != slot {
                continue;
            }
            let obel = vrf.find_bel_sibling(bel, oslot);
            vrf.claim_pip(bel.wire(pin), obel.wire("O"));
        }

        vrf.claim_pip(bel.wire("CLKTESTSIG"), bel.wire("CLKTESTSIG_INT"));

        vrf.claim_pip(bel.wire("HCLK_OUT"), bel.wire("O"));
        vrf.claim_pip(bel.wire("HCLK_OUT"), bel.wire("ODIV2"));
        vrf.claim_pip(bel.wire("HCLK_OUT"), bel.wire("CLKTESTSIG_INT"));
    } else {
        let pins = [
            ("I", SitePinDir::In),
            ("IB", SitePinDir::In),
            ("O", SitePinDir::Out),
        ];
        vrf.verify_legacy_bel(bel, "IBUFDS_GTHE1", &pins, &[]);
        for (pin, _) in pins {
            vrf.claim_net(&[bel.wire(pin)]);
        }
        for (pin, oslot) in [("I", bslots::IPAD_CLKP[0]), ("IB", bslots::IPAD_CLKN[0])] {
            let obel = vrf.find_bel_sibling(bel, oslot);
            vrf.claim_pip(bel.wire(pin), obel.wire("O"));
        }

        vrf.claim_net(&[bel.wire_far("O")]);
        vrf.claim_pip(bel.wire_far("O"), bel.wire("O"));
    }
}

pub fn verify_hclk_gtx(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    for i in 0..2 {
        vrf.claim_net(&[bel.wire(&format!("MGTREFCLKOUT{i}"))]);
        let obel = vrf.find_bel_sibling(bel, bslots::BUFDS[i]);
        vrf.claim_pip(bel.wire(&format!("MGTREFCLKOUT{i}")), obel.wire("O"));

        vrf.claim_net(&[bel.wire(&format!("SOUTHREFCLKOUT{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("SOUTHREFCLKOUT{i}")),
            bel.wire("MGTREFCLKIN0"),
        );
        vrf.claim_pip(
            bel.wire(&format!("SOUTHREFCLKOUT{i}")),
            bel.wire("MGTREFCLKIN1"),
        );
        vrf.claim_pip(
            bel.wire(&format!("SOUTHREFCLKOUT{i}")),
            bel.wire(&format!("SOUTHREFCLKIN{i}")),
        );
        vrf.claim_net(&[bel.wire(&format!("NORTHREFCLKOUT{i}"))]);
        vrf.claim_pip(
            bel.wire(&format!("NORTHREFCLKOUT{i}")),
            bel.wire("MGTREFCLKOUT0"),
        );
        vrf.claim_pip(
            bel.wire(&format!("NORTHREFCLKOUT{i}")),
            bel.wire("MGTREFCLKOUT1"),
        );
        vrf.claim_pip(
            bel.wire(&format!("NORTHREFCLKOUT{i}")),
            bel.wire(&format!("NORTHREFCLKIN{i}")),
        );
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -40, bslots::HCLK_GTX) {
            vrf.verify_net(&[
                bel.wire(&format!("NORTHREFCLKIN{i}")),
                obel.wire(&format!("NORTHREFCLKOUT{i}")),
            ]);
        } else {
            vrf.claim_net(&[bel.wire(&format!("NORTHREFCLKIN{i}"))]);
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 40, bslots::HCLK_GTX) {
            vrf.verify_net(&[
                bel.wire(&format!("SOUTHREFCLKIN{i}")),
                obel.wire(&format!("SOUTHREFCLKOUT{i}")),
            ]);
            vrf.verify_net(&[
                bel.wire(&format!("MGTREFCLKIN{i}")),
                obel.wire(&format!("MGTREFCLKOUT{i}")),
            ]);
        } else {
            vrf.claim_net(&[bel.wire(&format!("SOUTHREFCLKIN{i}"))]);
            vrf.claim_net(&[bel.wire(&format!("MGTREFCLKIN{i}"))]);
        }
    }
}

pub fn verify_gth(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let mut pins = vec![];
    for i in 0..4 {
        pins.extend([
            (format!("RXP{i}"), SitePinDir::In),
            (format!("RXN{i}"), SitePinDir::In),
            (format!("TXP{i}"), SitePinDir::Out),
            (format!("TXN{i}"), SitePinDir::Out),
        ]);
    }
    pins.extend([("REFCLK".to_string(), SitePinDir::In)]);
    let pin_refs: Vec<_> = pins.iter().map(|&(ref p, d)| (&p[..], d)).collect();
    vrf.verify_legacy_bel(bel, "GTHE1_QUAD", &pin_refs, &["GREFCLK"]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(&pin)]);
    }
    for i in 0..4 {
        let obel = vrf.find_bel_sibling(bel, bslots::IPAD_RXP[i]);
        vrf.claim_pip(bel.wire(&format!("RXP{i}")), obel.wire("O"));
        let obel = vrf.find_bel_sibling(bel, bslots::IPAD_RXN[i]);
        vrf.claim_pip(bel.wire(&format!("RXN{i}")), obel.wire("O"));
        let obel = vrf.find_bel_sibling(bel, bslots::OPAD_TXP[i]);
        vrf.claim_pip(obel.wire("I"), bel.wire(&format!("TXP{i}")));
        let obel = vrf.find_bel_sibling(bel, bslots::OPAD_TXN[i]);
        vrf.claim_pip(obel.wire("I"), bel.wire(&format!("TXN{i}")));
    }

    vrf.claim_net(&[bel.wire_far("REFCLK")]);
    vrf.claim_pip(bel.wire("REFCLK"), bel.wire_far("REFCLK"));
    vrf.claim_pip(bel.wire_far("REFCLK"), bel.wire("GREFCLK"));
    vrf.claim_pip(bel.wire_far("REFCLK"), bel.wire("REFCLK_IN"));
    vrf.claim_pip(bel.wire_far("REFCLK"), bel.wire("REFCLK_SOUTH"));
    vrf.claim_pip(bel.wire_far("REFCLK"), bel.wire("REFCLK_NORTH"));
    let obel = vrf.find_bel_sibling(bel, bslots::BUFDS[0]);
    vrf.verify_net(&[bel.wire("REFCLK_IN"), obel.wire_far("O")]);
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 40, bslots::GTH_QUAD) {
        vrf.claim_net(&[bel.wire_far("REFCLK_UP")]);
        vrf.claim_pip(bel.wire("REFCLK_UP"), bel.wire_far("REFCLK"));
        vrf.verify_net(&[bel.wire("REFCLK_SOUTH"), obel.wire("REFCLK_DN")]);
    } else {
        vrf.claim_net(&[bel.wire("REFCLK_SOUTH")]);
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -40, bslots::GTH_QUAD) {
        vrf.claim_net(&[bel.wire_far("REFCLK_DN")]);
        vrf.claim_pip(bel.wire("REFCLK_DN"), bel.wire_far("REFCLK"));
        vrf.verify_net(&[bel.wire("REFCLK_NORTH"), obel.wire("REFCLK_UP")]);
    } else {
        vrf.claim_net(&[bel.wire("REFCLK_NORTH")]);
    }
}

fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let slot_name = edev.db.bel_slots.key(bcrd.slot);
    match bcrd.slot {
        bslots::INT
        | bslots::INTF_INT
        | bslots::INTF_TESTMUX
        | bslots::SPEC_INT
        | bslots::CLK_INT
        | bslots::HROW_INT
        | bslots::HCLK
        | bslots::MISC_CFG
        | bslots::BANK
        | bslots::GLOBAL => (),
        _ if bslots::HCLK_DRP.contains(bcrd.slot) => (),
        _ if bslots::SLICE.contains(bcrd.slot) => verify_slice(edev, vrf, bcrd),
        _ if bslots::DSP.contains(bcrd.slot) => verify_dsp(vrf, bcrd),
        bslots::TIEOFF_DSP => verify_tieoff(vrf, bcrd),
        bslots::BRAM_F => verify_bram_f(vrf, bcrd),
        _ if bcrd.slot == bslots::BRAM_H[0] => {
            let bel = &mut vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "FIFO18E1", &[], &[])
        }
        _ if bcrd.slot == bslots::BRAM_H[1] => verify_bram_h1(vrf, bcrd),
        bslots::EMAC => vrf.verify_bel(bcrd).kind("TEMAC_SINGLE").commit(),
        bslots::PCIE => vrf.verify_bel(bcrd).kind("PCIE_2_0").commit(),

        _ if bslots::BUFIO.contains(bcrd.slot) => vrf.verify_bel(bcrd).kind("BUFIODQS").commit(),
        _ if bslots::BUFR.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        bslots::HCLK_IO_INT => verify_hclk_io_int(vrf, bcrd),
        bslots::IDELAYCTRL => vrf.verify_bel(bcrd).commit(),
        bslots::DCI => verify_dci(vrf, bcrd),

        _ if bslots::ILOGIC.contains(bcrd.slot) => verify_ilogic(edev, vrf, bcrd),
        _ if bslots::OLOGIC.contains(bcrd.slot) => verify_ologic(vrf, bcrd),
        _ if bslots::IODELAY.contains(bcrd.slot) => verify_iodelay(vrf, bcrd),
        _ if bslots::IOB.contains(bcrd.slot) => verify_iob(vrf, bcrd),

        _ if bslots::BSCAN.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        _ if bslots::ICAP.contains(bcrd.slot) => vrf.verify_bel(bcrd).kind("ICAP").commit(),
        _ if bslots::PMV_CFG.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        bslots::STARTUP
        | bslots::CAPTURE
        | bslots::EFUSE_USR
        | bslots::USR_ACCESS
        | bslots::DNA_PORT
        | bslots::DCIRESET
        | bslots::PPR_FRAME
        | bslots::PMVIOB_CLK
        | bslots::GLOBALSIG => vrf.verify_bel(bcrd).commit(),
        bslots::CFG_IO_ACCESS => vrf.verify_bel(bcrd).kind("CFG_IO_ACCESS").commit(),
        bslots::PMVBRAM => vrf.verify_bel(bcrd).kind("PMVBRAM").commit(),
        bslots::FRAME_ECC => vrf.verify_bel(bcrd).kind("FRAME_ECC").commit(),
        bslots::SYSMON => verify_sysmon(edev, vrf, bcrd),
        _ if slot_name.starts_with("IPAD") => verify_ipad(vrf, bcrd),
        _ if slot_name.starts_with("OPAD") => verify_opad(vrf, bcrd),

        _ if bslots::BUFHCE_W.contains(bcrd.slot) || bslots::BUFHCE_E.contains(bcrd.slot) => {
            vrf.verify_bel(bcrd).commit()
        }
        _ if bslots::PLL.contains(bcrd.slot) => verify_pll(vrf, bcrd),
        _ if bslots::BUFGCTRL.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),

        _ if bslots::GTX.contains(bcrd.slot) => verify_gtx(vrf, bcrd),
        _ if bslots::BUFDS.contains(bcrd.slot) => verify_bufds(vrf, bcrd),
        bslots::HCLK_GTX => verify_hclk_gtx(vrf, bcrd),
        bslots::GTH_QUAD => verify_gth(vrf, bcrd),

        _ => println!("MEOW {}", bcrd.to_string(edev.db)),
    }
}

fn verify_extra(_edev: &ExpandedDevice, vrf: &mut Verifier) {
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
                    vrf.claim_net(&[(RawWireCoord {
                        crd,
                        wire: &vrf.rd.wires[w],
                    })]);
                }
            }
        }
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);

    for i in 0..12 {
        vrf.skip_tcls_pip(
            tcls::HCLK,
            wires::HCLK_BUF[i].cell(1),
            wires::HCLK_ROW[i].cell(1),
        );
    }
    for i in 0..6 {
        vrf.skip_tcls_pip(
            tcls::HCLK,
            wires::RCLK_BUF[i].cell(1),
            wires::RCLK_ROW[i].cell(1),
        );
    }
    for co in 0..2 {
        for o in 0..8 {
            for i in 0..12 {
                vrf.skip_tcls_pip(
                    tcls::HCLK,
                    wires::LCLK[o].cell(co),
                    wires::HCLK_BUF[i].cell(1),
                );
                vrf.inject_tcls_pip(
                    tcls::HCLK,
                    wires::LCLK[o].cell(co),
                    wires::HCLK_ROW[i].cell(1),
                );
            }
            for i in 0..6 {
                vrf.skip_tcls_pip(
                    tcls::HCLK,
                    wires::LCLK[o].cell(co),
                    wires::RCLK_BUF[i].cell(1),
                );
                vrf.inject_tcls_pip(
                    tcls::HCLK,
                    wires::LCLK[o].cell(co),
                    wires::RCLK_ROW[i].cell(1),
                );
            }
        }
    }
    for i in 0..6 {
        vrf.skip_tcls_pip(
            tcls::HCLK_IO,
            wires::RCLK_ROW[i].cell(4),
            wires::PULLUP.cell(4),
        );
    }
    for i in 0..2 {
        vrf.alias_wire_slot(wires::VIOCLK_S_BUF[i], wires::VIOCLK_S[i]);
        vrf.alias_wire_slot(wires::VIOCLK_N_BUF[i], wires::VIOCLK_N[i]);
    }

    for tkn in ["HCLK_CMT_BOT", "HCLK_CMT_TOP"] {
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_CMT_CK_PERF_OUTER_L{i}_LEFT"),
                &format!("HCLK_CMT_CK_PERF_OUTER_L{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_CMT_CK_PERF_INNER_L{i}_LEFT"),
                &format!("HCLK_CMT_CK_PERF_INNER_L{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_CMT_CK_PERF_OUTER_R{i}_RIGHT"),
                &format!("HCLK_CMT_CK_PERF_OUTER_R{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_CMT_CK_PERF_INNER_R{i}_RIGHT"),
                &format!("HCLK_CMT_CK_PERF_INNER_R{i}"),
            );
        }
    }
    for i in 0..48 {
        vrf.mark_merge_pip(
            "CMT_BOT",
            &format!("CMT_BOT_IMUX_B_2_BUFG{i}"),
            &format!("CMT_BOT_IMUX_B{i}_0"),
        );
        vrf.mark_merge_pip(
            "CMT_TOP",
            &format!("CMT_TOP_IMUX_B_2_BUFG{i}"),
            &format!("CMT_TOP_IMUX_B{i}_17"),
        );
    }
    for tkn in ["CMT_BOT", "CMT_TOP"] {
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_0", "CMT_MMCM_CLKOUT0");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_1", "CMT_MMCM_CLKOUT0B");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_2", "CMT_MMCM_CLKOUT1");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_3", "CMT_MMCM_CLKOUT1B");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_4", "CMT_MMCM_CLKOUT2");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_5", "CMT_MMCM_CLKOUT2B");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_6", "CMT_MMCM_CLKOUT3");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_7", "CMT_MMCM_CLKOUT3B");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_8", "CMT_MMCM_CLKOUT4");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_9", "CMT_MMCM_CLKOUT5");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_10", "CMT_MMCM_CLKOUT6");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_11", "CMT_MMCM_CLKFBOUT");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_12", "CMT_MMCM_CLKFBOUTB");
        vrf.mark_merge_pip(tkn, "CMT_CK_MMCM_13", "CMT_MMCM_TMUXOUT");
    }
    vrf.mark_merge_pip("CMT_BOT", "CMT_MMCM_IMUX_CLKIN1", "CMT_BOT_CLK_B0_15");
    vrf.mark_merge_pip("CMT_BOT", "CMT_MMCM_IMUX_CLKIN2", "CMT_BOT_CLK_B1_15");
    vrf.mark_merge_pip("CMT_BOT", "CMT_MMCM_IMUX_CLKFB", "CMT_BOT_CLK_B1_16");
    vrf.mark_merge_pip("CMT_TOP", "CMT_MMCM_IMUX_CLKIN1", "CMT_TOP_CLK_B1_2");
    vrf.mark_merge_pip("CMT_TOP", "CMT_MMCM_IMUX_CLKIN2", "CMT_TOP_CLK_B0_2");
    vrf.mark_merge_pip("CMT_TOP", "CMT_MMCM_IMUX_CLKFB", "CMT_TOP_CLK_B0_1");
    for tkn in ["LIOI", "RIOI"] {
        vrf.mark_merge_pip(tkn, "IOI_INT_RCLKMUX_B_N", "IOI_IMUX_B4_0");
        vrf.mark_merge_pip(tkn, "IOI_INT_RCLKMUX_B_S", "IOI_IMUX_B4_1");
    }
    vrf.mark_merge_pip("LIOI", "LIOI_I_2IOCLK_BOT1_I2GCLK", "LIOI_I_2IOCLK_BOT1");
    vrf.mark_merge_pip("RIOI", "RIOI_I_2IOCLK_BOT1_I2GCLK", "RIOI_I_2IOCLK_BOT1");
    for tkn in ["HCLK_INNER_IOI", "HCLK_OUTER_IOI"] {
        vrf.mark_merge_pip(tkn, "HCLK_IOI_VBUFOCLK0", "HCLK_IOI_BUFOCLK0");
        vrf.mark_merge_pip(tkn, "HCLK_IOI_VBUFOCLK1", "HCLK_IOI_BUFOCLK1");
        for &crd in rd.tiles_by_kind_name(tkn) {
            vrf.merge_node(
                RawWireCoord {
                    crd,
                    wire: "HCLK_IOI_OCLK0",
                },
                RawWireCoord {
                    crd,
                    wire: "HCLK_IOI_BUFO_IN0",
                },
            );
            vrf.merge_node(
                RawWireCoord {
                    crd,
                    wire: "HCLK_IOI_OCLK1",
                },
                RawWireCoord {
                    crd,
                    wire: "HCLK_IOI_BUFO_IN1",
                },
            );
        }
    }
    for i in 0..10 {
        vrf.mark_merge_pip(
            "HCLK_CLBLM_MGT_LEFT",
            &format!("HCLK_CLB_MGT_CK_IN_MGT{i}"),
            &format!("HCLK_CLB_MGT_CK_OUT_MGT{i}"),
        );
        vrf.mark_merge_pip(
            "HCLK_CLBLM_MGT",
            &format!("HCLK_CLB_MGT_CK_OUT_MGT{i}"),
            &format!("HCLK_CLB_MGT_CK_IN_MGT{i}"),
        );
    }
    for tkn in ["CMT_PMVB_BUF_BELOW", "CMT_PMVB_BUF_ABOVE"] {
        for i in 0..32 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_PMVB_CK_GCLK{i}_OUT"),
                &format!("CMT_PMVB_CK_GCLK{i}_IN"),
            );
        }
        for i in 0..8 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CMT_PMVB_CK_IO_TO_CMT{i}_OUT"),
                &format!("CMT_PMVB_CK_IO_TO_CMT{i}_IN"),
            );
        }
    }

    vrf.prep_int_wires();

    for i in 0..32 {
        vrf.skip_tcls_pip(
            tcls::CMT,
            wires::GCLK_TEST[i].cell(20),
            wires::GCLK_TEST_IN[i].cell(20),
        );
        vrf.skip_tcls_pip(
            tcls::CMT,
            wires::GCLK_TEST_IN[i].cell(20),
            wires::GCLK_CMT[i].cell(20),
        );
    }
    for &tcrd in &endev.edev.tile_index[tcls::CMT] {
        let bcrd = tcrd.bel(bslots::SPEC_INT);
        let mut bel = vrf.verify_bel(bcrd);
        for lr in ['L', 'R'] {
            bel.claim_pip(
                bel.wire(&format!("BUFH_TEST_{lr}")),
                bel.wire(&format!("BUFH_TEST_{lr}_INV")),
            );
            bel.claim_pip(
                bel.wire(&format!("BUFH_TEST_{lr}")),
                bel.wire(&format!("BUFH_TEST_{lr}_NOINV")),
            );
            bel.claim_net(&[bel.wire(&format!("BUFH_TEST_{lr}_INV"))]);
            bel.claim_pip(
                bel.wire(&format!("BUFH_TEST_{lr}_INV")),
                bel.wire(&format!("BUFH_TEST_{lr}_PRE")),
            );
            bel.claim_net(&[bel.wire(&format!("BUFH_TEST_{lr}_NOINV"))]);
            bel.claim_pip(
                bel.wire(&format!("BUFH_TEST_{lr}_NOINV")),
                bel.wire(&format!("BUFH_TEST_{lr}_PRE")),
            );
        }
        for i in 0..32 {
            bel.claim_pip(
                bel.wire(&format!("GCLK{i}_TEST")),
                bel.wire(&format!("GCLK{i}_INV")),
            );
            bel.claim_pip(
                bel.wire(&format!("GCLK{i}_TEST")),
                bel.wire(&format!("GCLK{i}_NOINV")),
            );
            bel.claim_net(&[bel.wire(&format!("GCLK{i}_INV"))]);
            bel.claim_pip(
                bel.wire(&format!("GCLK{i}_INV")),
                bel.wire(&format!("GCLK{i}")),
            );
            bel.claim_net(&[bel.wire(&format!("GCLK{i}_NOINV"))]);
            bel.claim_pip(
                bel.wire(&format!("GCLK{i}_NOINV")),
                bel.wire(&format!("GCLK{i}")),
            );
        }
        let BelInfo::SwitchBox(ref sb) = endev.edev.db[tcls::CMT].bels[bslots::SPEC_INT] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::ProgBuf(buf) = item else {
                continue;
            };
            if !wires::GCLK_CMT.contains(buf.dst.wire)
                && !wires::GIOB_CMT.contains(buf.dst.wire)
                && !wires::CCIO_CMT_W.contains(buf.dst.wire)
                && !wires::CCIO_CMT_E.contains(buf.dst.wire)
                && !wires::HCLK_CMT_W.contains(buf.dst.wire)
                && !wires::HCLK_CMT_E.contains(buf.dst.wire)
                && !wires::RCLK_CMT_W.contains(buf.dst.wire)
                && !wires::RCLK_CMT_E.contains(buf.dst.wire)
                && !wires::MGT_CMT_W.contains(buf.dst.wire)
                && !wires::MGT_CMT_E.contains(buf.dst.wire)
            {
                continue;
            }
            vrf.skip_tcls_pip(tcls::CMT, buf.dst, buf.src.tw);
            let wt = endev.edev.resolve_tile_wire(tcrd, buf.dst).unwrap();
            let wf = endev.edev.resolve_tile_wire(tcrd, buf.src.tw).unwrap();
            vrf.alias_wire(wt, wf);
        }
    }

    for (tcid, range) in [(tcls::CLK_BUFG_S, 0..4), (tcls::CLK_BUFG_N, 4..8)] {
        for &tcrd in &endev.edev.tile_index[tcid] {
            let bcrd = tcrd.bel(bslots::SPEC_INT);
            for i in range.clone() {
                vrf.claim_pip(
                    vrf.bel_wire(bcrd, &format!("GIO{i}_CMT")),
                    vrf.bel_wire(bcrd, &format!("GIO{i}")),
                );
                let wt = vrf.bel_wire(bcrd, &format!("GIO{i}_CMT"));
                let wf = vrf.bel_wire(bcrd, &format!("GIO{i}_BUFG"));
                vrf.merge_node(wt, wf);
            }
            let BelInfo::SwitchBox(ref sb) = endev.edev.db[tcid].bels[bslots::SPEC_INT] else {
                unreachable!()
            };
            for item in &sb.items {
                let SwitchBoxItem::ProgBuf(buf) = item else {
                    continue;
                };
                if wires::OUT_BUFG_GFB.contains(buf.dst.wire) {
                    continue;
                }
                vrf.skip_tcls_pip(tcid, buf.dst, buf.src.tw);
                vrf.inject_tcls_pip(tcid, buf.src.tw, buf.dst);
            }
        }
    }

    vrf.handle_int();

    for (tcrd, tile) in endev.edev.tiles() {
        let tcls = &endev.edev.db[tile.class];
        for slot in tcls.bels.ids() {
            verify_bel(endev.edev, &mut vrf, tcrd.bel(slot));
        }
    }
    verify_extra(endev.edev, &mut vrf);
    vrf.finish();
}
