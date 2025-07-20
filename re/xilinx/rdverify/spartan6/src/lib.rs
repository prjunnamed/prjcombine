use prjcombine_re_xilinx_naming_spartan6::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};
use prjcombine_spartan6::{
    bels,
    chip::{ColumnKind, DisabledPart, Gts},
};
use std::collections::HashSet;
use unnamed_entity::EntityId;

fn verify_sliceml(vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    if let Some(obel) = vrf.find_bel_walk(bel, 0, -1, bels::SLICE0) {
        vrf.claim_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
        vrf.claim_pip(obel.crd(), obel.wire_far("COUT"), obel.wire("COUT"));
    } else {
        vrf.claim_node(&[bel.fwire("CIN")]);
    }
    vrf.claim_node(&[bel.fwire("COUT")]);
}

fn verify_dsp(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let carry: Vec<_> = (0..18)
        .map(|x| (format!("BCOUT{x}"), format!("BCIN{x}")))
        .chain((0..48).map(|x| (format!("PCOUT{x}"), format!("PCIN{x}"))))
        .chain([("CARRYOUT".to_string(), "CARRYIN".to_string())])
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
    if let Some(obel) = vrf.find_bel_walk(bel, 0, -4, bels::DSP) {
        for (o, i) in &carry {
            vrf.verify_node(&[bel.fwire(i), obel.fwire_far(o)]);
            vrf.claim_pip(obel.crd(), obel.wire_far(o), obel.wire(o));
        }
    }
}

fn verify_ilogic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![
        ("TFB", SitePinDir::In),
        ("OFB", SitePinDir::In),
        ("D", SitePinDir::In),
        ("DDLY", SitePinDir::In),
        ("DDLY2", SitePinDir::In),
        ("CLK0", SitePinDir::In),
        ("CLK1", SitePinDir::In),
        ("IOCE", SitePinDir::In),
        ("SHIFTIN", SitePinDir::In),
        ("SHIFTOUT", SitePinDir::Out),
        ("DFB", SitePinDir::Out),
        ("CFB0", SitePinDir::Out),
        ("CFB1", SitePinDir::Out),
        ("SR", SitePinDir::In),
    ];
    if bel.slot == bels::ILOGIC0 {
        pins.extend([("INCDEC", SitePinDir::Out), ("VALID", SitePinDir::Out)]);
    }
    vrf.verify_bel(bel, "ILOGIC2", &pins, &["SR_INT"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let oslot = match bel.slot {
        bels::ILOGIC0 => bels::OLOGIC0,
        bels::ILOGIC1 => bels::OLOGIC1,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("SR"), bel.wire("SR_INT"));
    vrf.claim_pip(bel.crd(), bel.wire("SR"), obel.wire_far("SR"));
    vrf.claim_pip(bel.crd(), bel.wire("OFB"), obel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("TFB"), obel.wire("TQ"));

    let oslot = match bel.slot {
        bels::ILOGIC0 => bels::IODELAY0,
        bels::ILOGIC1 => bels::IODELAY1,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("DDLY"), obel.wire("DATAOUT"));
    vrf.claim_pip(bel.crd(), bel.wire("DDLY2"), obel.wire("DATAOUT2"));

    let oslot = match bel.slot {
        bels::ILOGIC0 => bels::IOICLK0,
        bels::ILOGIC1 => bels::IOICLK1,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    let obel_tie = vrf.find_bel_sibling(bel, bels::TIEOFF_IOI);
    vrf.claim_pip(bel.crd(), bel.wire("CLK0"), obel.wire("CLK0_ILOGIC"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), obel.wire("CLK1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE"), obel_tie.wire("HARD1"));

    vrf.claim_node(&[bel.fwire("D_MUX")]);
    vrf.claim_pip(bel.crd(), bel.wire("D"), bel.wire("D_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("D_MUX"), bel.wire("IOB_I"));

    let oslot = match bel.slot {
        bels::ILOGIC0 => bels::IOB0,
        bels::ILOGIC1 => bels::IOB1,
        _ => unreachable!(),
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 0, oslot) {
        vrf.verify_node(&[bel.fwire("IOB_I"), obel.fwire_far("I")]);

        vrf.claim_pip(bel.crd(), bel.wire("MCB_FABRICOUT"), bel.wire("FABRICOUT"));
    } else {
        vrf.claim_node(&[bel.fwire("IOB_I")]);
    }
    vrf.claim_node(&[bel.fwire("MCB_FABRICOUT")]);

    let oslot = match bel.slot {
        bels::ILOGIC0 => bels::ILOGIC1,
        bels::ILOGIC1 => bels::ILOGIC0,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN"), obel.wire("SHIFTOUT"));
    if bel.slot == bels::ILOGIC0 {
        vrf.claim_pip(bel.crd(), bel.wire("D_MUX"), obel.wire("IOB_I"));
    }
}

fn verify_ologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLK0", SitePinDir::In),
        ("CLK1", SitePinDir::In),
        ("IOCE", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTIN3", SitePinDir::In),
        ("SHIFTIN4", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
        ("SHIFTOUT3", SitePinDir::Out),
        ("SHIFTOUT4", SitePinDir::Out),
        ("OQ", SitePinDir::Out),
        ("TQ", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "OLOGIC2", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let oslot = match bel.slot {
        bels::OLOGIC0 => bels::IOICLK0,
        bels::OLOGIC1 => bels::IOICLK1,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    let obel_tie = vrf.find_bel_sibling(bel, bels::TIEOFF_IOI);
    vrf.claim_pip(bel.crd(), bel.wire("CLK0"), obel.wire("CLK0_OLOGIC"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), obel.wire("CLK1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE"), obel_tie.wire("HARD1"));

    let obel_ioi = vrf.find_bel_sibling(bel, bels::IOI);
    vrf.claim_pip(bel.crd(), bel.wire("OCE"), obel_ioi.wire("PCI_CE"));
    vrf.claim_pip(bel.crd(), bel.wire("REV"), obel_tie.wire("HARD0"));
    vrf.claim_pip(bel.crd(), bel.wire("SR"), obel_tie.wire("HARD0"));
    vrf.claim_pip(bel.crd(), bel.wire("TRAIN"), obel_tie.wire("HARD0"));

    let oslot = match bel.slot {
        bels::OLOGIC0 => bels::IODELAY0,
        bels::OLOGIC1 => bels::IODELAY1,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), bel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_O"), obel.wire("DOUT"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_T"), bel.wire("TQ"));
    vrf.claim_pip(bel.crd(), bel.wire("IOB_T"), obel.wire("TOUT"));

    let oslot = match bel.slot {
        bels::OLOGIC0 => bels::IOB0,
        bels::OLOGIC1 => bels::IOB1,
        _ => unreachable!(),
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 0, oslot) {
        vrf.verify_node(&[bel.fwire("IOB_O"), obel.fwire_far("O")]);
        vrf.verify_node(&[bel.fwire("IOB_T"), obel.fwire_far("T")]);

        vrf.claim_pip(bel.crd(), bel.wire("D1"), bel.wire("MCB_D1"));
        vrf.claim_pip(bel.crd(), bel.wire("D2"), bel.wire("MCB_D2"));
        vrf.claim_pip(bel.crd(), bel.wire("T1"), obel_ioi.wire("MCB_T1"));
        vrf.claim_pip(bel.crd(), bel.wire("T2"), obel_ioi.wire("MCB_T2"));
        vrf.claim_pip(bel.crd(), bel.wire("TRAIN"), obel_ioi.wire("MCB_DRPTRAIN"));
    } else {
        vrf.claim_node(&[bel.fwire("IOB_T")]);
        vrf.claim_node(&[bel.fwire("IOB_O")]);
    }

    let oslot = match bel.slot {
        bels::OLOGIC0 => bels::OLOGIC1,
        bels::OLOGIC1 => bels::OLOGIC0,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    if bel.slot == bels::OLOGIC0 {
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    } else {
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN3"), obel.wire("SHIFTOUT3"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN4"), obel.wire("SHIFTOUT4"));
    }
}

fn verify_iodelay(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("IOCLK0", SitePinDir::In),
        ("IOCLK1", SitePinDir::In),
        ("IDATAIN", SitePinDir::In),
        ("ODATAIN", SitePinDir::In),
        ("T", SitePinDir::In),
        ("DOUT", SitePinDir::Out),
        ("TOUT", SitePinDir::Out),
        ("DATAOUT", SitePinDir::Out),
        ("DATAOUT2", SitePinDir::Out),
        ("DQSOUTP", SitePinDir::Out),
        ("DQSOUTN", SitePinDir::Out),
        ("AUXSDO", SitePinDir::Out),
        ("AUXSDOIN", SitePinDir::In),
        ("AUXADDR0", SitePinDir::In),
        ("AUXADDR1", SitePinDir::In),
        ("AUXADDR2", SitePinDir::In),
        ("AUXADDR3", SitePinDir::In),
        ("AUXADDR4", SitePinDir::In),
        ("READEN", SitePinDir::In),
        ("MEMUPDATE", SitePinDir::In),
    ];
    vrf.verify_bel(bel, "IODELAY2", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let oslot = match bel.slot {
        bels::IODELAY0 => bels::IOICLK0,
        bels::IODELAY1 => bels::IOICLK1,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK0"), obel.wire("CLK0_ILOGIC"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK0"), obel.wire("CLK0_OLOGIC"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCLK1"), obel.wire("CLK1"));

    let oslot = match bel.slot {
        bels::IODELAY0 => bels::ILOGIC0,
        bels::IODELAY1 => bels::ILOGIC1,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("IDATAIN"), obel.wire("D_MUX"));

    let oslot = match bel.slot {
        bels::IODELAY0 => bels::OLOGIC0,
        bels::IODELAY1 => bels::OLOGIC1,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("ODATAIN"), obel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("T"), obel.wire("TQ"));

    let obel_ioi = vrf.find_bel_sibling(bel, bels::IOI);
    let oslot = match bel.slot {
        bels::IODELAY0 => bels::IOB0,
        bels::IODELAY1 => bels::IOB1,
        _ => unreachable!(),
    };
    vrf.claim_node(&[bel.fwire("MCB_DQSOUTP")]);
    if vrf.find_bel_delta(bel, 0, 0, oslot).is_some() {
        vrf.claim_pip(bel.crd(), bel.wire("MCB_DQSOUTP"), bel.wire("DQSOUTP"));
        vrf.claim_pip(bel.crd(), bel.wire("CAL"), obel_ioi.wire("MCB_DRPADD"));
        vrf.claim_pip(bel.crd(), bel.wire("CE"), obel_ioi.wire("MCB_DRPSDO"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), obel_ioi.wire("MCB_DRPCLK"));
        vrf.claim_pip(bel.crd(), bel.wire("INC"), obel_ioi.wire("MCB_DRPCS"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("RST"),
            obel_ioi.wire("MCB_DRPBROADCAST"),
        );
    }
}

fn verify_ioiclk(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, bels::IOI);
    vrf.claim_node(&[bel.fwire("CLK0INTER")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), obel.wire("IOCLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), obel.wire("IOCLK2"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0INTER"), obel.wire("PLLCLK0"));
    vrf.claim_node(&[bel.fwire("CLK1INTER")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), obel.wire("IOCLK1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), obel.wire("IOCLK3"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1INTER"), obel.wire("PLLCLK1"));
    vrf.claim_node(&[bel.fwire("CLK2INTER")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK2INTER"), obel.wire("PLLCLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK2INTER"), obel.wire("PLLCLK1"));
    vrf.claim_node(&[bel.fwire("CLK0_ILOGIC")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_ILOGIC"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_ILOGIC"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_ILOGIC"), bel.wire("CLK2INTER"));
    vrf.claim_node(&[bel.fwire("CLK0_OLOGIC")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_OLOGIC"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_OLOGIC"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK0_OLOGIC"), bel.wire("CLK2INTER"));
    vrf.claim_node(&[bel.fwire("CLK1")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK1"), bel.wire("CLK2INTER"));
    vrf.claim_node(&[bel.fwire("IOCE0")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("IOCE2"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("IOCE3"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("PLLCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE0"), obel.wire("PLLCE1"));
    vrf.claim_node(&[bel.fwire("IOCE1")]);
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("IOCE2"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("IOCE3"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("PLLCE0"));
    vrf.claim_pip(bel.crd(), bel.wire("IOCE1"), obel.wire("PLLCE1"));
}

fn verify_ioi(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if bel.col == endev.chip.col_w() || bel.col == endev.chip.col_e() {
        verify_pci_ce_v_src(endev, vrf, bel, true, "PCI_CE");
        let srow = endev.chip.row_hclk(bel.row);
        let ud = if bel.row < srow { 'D' } else { 'U' };
        let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::LRIOI_CLK));
        for i in 0..4 {
            vrf.verify_node(&[
                bel.fwire(&format!("IOCLK{i}")),
                obel.fwire(&format!("IOCLK{i}_O_{ud}")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("IOCE{i}")),
                obel.fwire(&format!("IOCE{i}_O_{ud}")),
            ]);
        }
        for i in 0..2 {
            vrf.verify_node(&[
                bel.fwire(&format!("PLLCLK{i}")),
                obel.fwire(&format!("PLLCLK{i}_O_{ud}")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("PLLCE{i}")),
                obel.fwire(&format!("PLLCE{i}_O_{ud}")),
            ]);
        }
    } else {
        let srow = if bel.row < endev.chip.row_clk() {
            endev.chip.row_bio_outer()
        } else {
            endev.chip.row_tio_outer()
        };
        let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::BTIOI_CLK));
        vrf.verify_node(&[bel.fwire("PCI_CE"), obel.fwire("PCI_CE_O")]);
        for i in 0..4 {
            vrf.verify_node(&[
                bel.fwire(&format!("IOCLK{i}")),
                obel.fwire(&format!("IOCLK{i}_O")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("IOCE{i}")),
                obel.fwire(&format!("IOCE{i}_O")),
            ]);
        }
        for i in 0..2 {
            vrf.verify_node(&[
                bel.fwire(&format!("PLLCLK{i}")),
                obel.fwire(&format!("PLLCLK{i}_O")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("PLLCE{i}")),
                obel.fwire(&format!("PLLCE{i}_O")),
            ]);
        }
    }

    vrf.claim_node(&[bel.fwire("MCB_DRPSDI")]);
    if endev.edev.disabled.contains(&DisabledPart::Mcb)
        || endev.chip.columns[bel.col].kind != ColumnKind::Io
    {
        for pin in [
            "MCB_DRPADD",
            "MCB_DRPBROADCAST",
            "MCB_DRPCLK",
            "MCB_DRPCS",
            "MCB_DRPSDO",
            "MCB_DRPTRAIN",
            "MCB_T1",
            "MCB_T2",
        ] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
        for slot in bels::OLOGIC {
            let obel = vrf.find_bel_sibling(bel, slot);
            vrf.claim_node(&[obel.fwire("MCB_D1")]);
            vrf.claim_node(&[obel.fwire("MCB_D2")]);
        }
        // in other cases, nodes will be verified by MCB code
    }
}

fn verify_iob(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![
        ("I", SitePinDir::Out),
        ("O", SitePinDir::In),
        ("T", SitePinDir::In),
        ("PCI_RDY", SitePinDir::Out),
        ("PADOUT", SitePinDir::Out),
        ("DIFFI_IN", SitePinDir::In),
        ("DIFFO_OUT", SitePinDir::Out),
        ("DIFFO_IN", SitePinDir::In),
    ];
    let kind = match bel.slot {
        bels::IOB0 => "IOBS",
        bels::IOB1 => "IOBM",
        _ => unreachable!(),
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    vrf.claim_node(&[bel.fwire_far("I")]);
    vrf.claim_node(&[bel.fwire_far("O")]);
    vrf.claim_node(&[bel.fwire_far("T")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire_far("O"));
    vrf.claim_pip(bel.crd(), bel.wire("T"), bel.wire_far("T"));
    vrf.claim_pip(bel.crd(), bel.wire_far("I"), bel.wire("I"));

    let oslot = match bel.slot {
        bels::IOB0 => bels::IOB1,
        bels::IOB1 => bels::IOB0,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
    if bel.slot == bels::IOB0 {
        vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
    }
}

fn verify_clkpin_buf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (opin, ipin, oslot) in [
        ("CLKPIN0_O", "CLKPIN0_I", bels::ILOGIC1),
        ("CLKPIN1_O", "CLKPIN1_I", bels::ILOGIC0),
    ] {
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
        let obel = vrf.find_bel_sibling(bel, oslot);
        vrf.verify_node(&[bel.fwire(ipin), obel.fwire("IOB_I")]);
    }
    for (opin, ipin, oslot, fopin, fpin) in [
        ("DFB0_O", "DFB0_I", bels::ILOGIC1, "DFB_OUT", "DFB"),
        ("DFB1_O", "DFB1_I", bels::ILOGIC0, "DFB_OUT", "DFB"),
        ("CFB0_0_O", "CFB0_0_I", bels::ILOGIC1, "CFB0_OUT", "CFB0"),
        ("CFB0_1_O", "CFB0_1_I", bels::ILOGIC0, "CFB0_OUT", "CFB0"),
        ("CFB1_0_O", "CFB1_0_I", bels::ILOGIC1, "CFB1_OUT", "CFB1"),
        ("CFB1_1_O", "CFB1_1_I", bels::ILOGIC0, "CFB1_OUT", "CFB1"),
        ("DQSP_O", "DQSP_I", bels::IODELAY1, "DQSOUTP_OUT", "DQSOUTP"),
        ("DQSN_O", "DQSN_I", bels::IODELAY1, "DQSOUTN_OUT", "DQSOUTN"),
    ] {
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
        let obel = vrf.find_bel_sibling(bel, oslot);
        vrf.claim_node(&[bel.fwire(ipin), obel.fwire(fopin)]);
        vrf.claim_pip(obel.crd(), obel.wire(fopin), obel.wire(fpin));
    }
}

fn verify_tieoff(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "TIEOFF",
        &[
            ("HARD0", SitePinDir::Out),
            ("HARD1", SitePinDir::Out),
            ("KEEP1", SitePinDir::Out),
        ],
        &[],
    );
    for pin in ["HARD0", "HARD1", "KEEP1"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_pcilogicse(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "PCILOGICSE",
        &[
            ("PCI_CE", SitePinDir::Out),
            ("IRDY", SitePinDir::In),
            ("TRDY", SitePinDir::In),
        ],
        &[],
    );
    vrf.claim_node(&[bel.fwire("PCI_CE"), bel.pip_iwire("PCI_CE", 0)]);
    vrf.claim_node(&[bel.pip_owire("PCI_CE", 0)]);
    let (pcrd, po, pi) = bel.pip("PCI_CE", 0);
    vrf.claim_pip(pcrd, po, pi);
    let rdy = if bel.col == endev.chip.col_w() {
        [("IRDY", 2, bels::IOB0), ("TRDY", -1, bels::IOB1)]
    } else {
        [("IRDY", 2, bels::IOB1), ("TRDY", -1, bels::IOB0)]
    };
    for (pin, dy, slot) in rdy {
        let (pcrd, po, pi) = bel.pip(pin, 0);
        vrf.claim_node(&[bel.fwire(pin), bel.pip_owire(pin, 0)]);
        vrf.claim_pip(pcrd, po, pi);
        let obel = vrf.find_bel_delta(bel, 0, dy, slot).unwrap();
        vrf.claim_node(&[bel.pip_iwire(pin, 0), obel.fwire_far("PCI_RDY")]);
        vrf.claim_pip(obel.crd(), obel.wire_far("PCI_RDY"), obel.wire("PCI_RDY"));
    }
}

fn verify_mcb(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mcb = endev.chip.get_mcb(bel.row);
    let mut pins_out = vec![
        ("WE".to_string(), mcb.io_we),
        ("CAS".to_string(), mcb.io_cas),
        ("RAS".to_string(), mcb.io_ras),
        ("RST".to_string(), mcb.io_reset),
        ("CKE".to_string(), mcb.io_cke),
        ("ODT".to_string(), mcb.io_odt),
    ];
    for i in 0..3 {
        pins_out.push((format!("BA{i}"), mcb.io_ba[i]));
    }
    for i in 0..15 {
        pins_out.push((format!("ADDR{i}"), mcb.io_addr[i]));
    }
    let mut pins_dq = vec![];
    for i in 0..16 {
        pins_dq.push((
            format!("DQI{i}"),
            format!("DQOP{i}"),
            format!("DQON{i}"),
            mcb.iop_dq[i / 2],
            (i % 2) ^ 1,
        ));
    }
    let pins_dm = [
        ("LDMP", "LDMN", mcb.io_dm[0]),
        ("UDMP", "UDMN", mcb.io_dm[1]),
    ];
    let pins_dqs = [
        ("DQSIOIP", "DQSIOIN", mcb.iop_dqs[0]),
        ("UDQSIOIP", "UDQSIOIN", mcb.iop_dqs[1]),
    ];
    let mut pins_ref = vec![
        ("PLLCE0", SitePinDir::In),
        ("PLLCE1", SitePinDir::In),
        ("PLLCLK0", SitePinDir::In),
        ("PLLCLK1", SitePinDir::In),
        ("IOIDRPCLK", SitePinDir::Out),
        ("IOIDRPCS", SitePinDir::Out),
        ("IOIDRPADD", SitePinDir::Out),
        ("IOIDRPADDR0", SitePinDir::Out),
        ("IOIDRPADDR1", SitePinDir::Out),
        ("IOIDRPADDR2", SitePinDir::Out),
        ("IOIDRPADDR3", SitePinDir::Out),
        ("IOIDRPADDR4", SitePinDir::Out),
        ("IOIDRPBROADCAST", SitePinDir::Out),
        ("IOIDRPUPDATE", SitePinDir::Out),
        ("IOIDRPSDO", SitePinDir::Out),
        ("IOIDRPSDI", SitePinDir::In),
        ("IOIDRPTRAIN", SitePinDir::Out),
        ("DQIOWEN0", SitePinDir::Out),
        ("DQSIOWEN90P", SitePinDir::Out),
        ("DQSIOWEN90N", SitePinDir::Out),
    ];
    for (pin, _) in &pins_out {
        pins_ref.push((pin, SitePinDir::Out));
    }
    for (i, op, on, _, _) in &pins_dq {
        pins_ref.push((i, SitePinDir::In));
        pins_ref.push((op, SitePinDir::Out));
        pins_ref.push((on, SitePinDir::Out));
    }
    for &(op, on, _) in &pins_dm {
        pins_ref.push((op, SitePinDir::Out));
        pins_ref.push((on, SitePinDir::Out));
    }
    for &(pp, pn, _) in &pins_dqs {
        pins_ref.push((pp, SitePinDir::In));
        pins_ref.push((pn, SitePinDir::In));
    }
    vrf.verify_bel(bel, "MCB", &pins_ref, &[]);
    for (pin, dir) in pins_ref {
        if dir == SitePinDir::In {
            vrf.claim_node(&[bel.fwire(pin), bel.pip_owire(pin, 0)]);
        } else {
            vrf.claim_node(&[bel.fwire(pin), bel.pip_iwire(pin, 0)]);
        }
        let (pc, po, pi) = bel.pip(pin, 0);
        vrf.claim_pip(pc, po, pi);
    }

    let obel = vrf.find_bel_sibling(bel, bels::LRIOI_CLK_TERM);
    vrf.verify_node(&[bel.pip_iwire("PLLCE0", 0), obel.fwire("PLLCE0_O")]);
    vrf.verify_node(&[bel.pip_iwire("PLLCE1", 0), obel.fwire("PLLCE1_O")]);
    vrf.verify_node(&[bel.pip_iwire("PLLCLK0", 0), obel.fwire("PLLCLK0_O")]);
    vrf.verify_node(&[bel.pip_iwire("PLLCLK1", 0), obel.fwire("PLLCLK1_O")]);

    let mut rows_handled = HashSet::new();
    {
        let obel = vrf.get_bel(bel.cell.with_row(mcb.iop_clk).bel(bels::IOI));
        vrf.claim_node(&[obel.fwire("MCB_T1")]);
        vrf.claim_node(&[obel.fwire("MCB_T2")]);
        rows_handled.insert(mcb.iop_clk);
    }

    let mut rows_out_handled = HashSet::new();
    for (pin, io) in pins_out {
        let obel = vrf.get_bel(bel.cell.with_row(io.row).bel(bels::OLOGIC[io.iob.to_idx()]));
        vrf.claim_node(&[
            bel.fwire_far(&pin),
            obel.fwire("MCB_D1"),
            obel.fwire("MCB_D2"),
        ]);
        if !rows_out_handled.contains(&io.row) {
            let obel = vrf.get_bel(bel.cell.with_row(io.row).bel(bels::IOI));
            vrf.claim_node(&[obel.fwire("MCB_T1")]);
            vrf.claim_node(&[obel.fwire("MCB_T2")]);
        }
        rows_handled.insert(io.row);
        rows_out_handled.insert(io.row);
    }
    vrf.claim_node(&[bel.fwire_far("DQIOWEN0")]);
    for (i, op, on, row, bi) in pins_dq {
        rows_handled.insert(row);
        let obel = vrf.get_bel(bel.cell.with_row(row).bel(bels::OLOGIC[bi]));
        vrf.claim_node(&[bel.fwire_far(&op), obel.fwire("MCB_D1")]);
        vrf.claim_node(&[bel.fwire_far(&on), obel.fwire("MCB_D2")]);
        let obel = vrf.get_bel(bel.cell.with_row(row).bel(bels::IODELAY[bi]));
        vrf.verify_node(&[bel.fwire_far(&i), obel.fwire("MCB_DQSOUTP")]);
        let obel = vrf.get_bel(bel.cell.with_row(row).bel(bels::IOI));
        vrf.verify_node(&[
            bel.fwire_far("DQIOWEN0"),
            obel.fwire("MCB_T1"),
            obel.fwire("MCB_T2"),
        ]);
    }
    for (op, on, io) in pins_dm {
        rows_handled.insert(io.row);
        let obel = vrf.get_bel(bel.cell.with_row(io.row).bel(bels::OLOGIC[io.iob.to_idx()]));
        vrf.claim_node(&[bel.fwire_far(op), obel.fwire("MCB_D1")]);
        vrf.claim_node(&[bel.fwire_far(on), obel.fwire("MCB_D2")]);
        let obel = vrf.get_bel(bel.cell.with_row(io.row).bel(bels::IOI));
        vrf.verify_node(&[
            bel.fwire_far("DQIOWEN0"),
            obel.fwire("MCB_T1"),
            obel.fwire("MCB_T2"),
        ]);
    }
    vrf.claim_node(&[bel.fwire_far("DQSIOWEN90P")]);
    vrf.claim_node(&[bel.fwire_far("DQSIOWEN90N")]);
    for (pp, pn, row) in pins_dqs {
        rows_handled.insert(row);
        let obel = vrf.get_bel(bel.cell.with_row(row).bel(bels::IOI));
        vrf.verify_node(&[bel.fwire_far("DQSIOWEN90N"), obel.fwire("MCB_T1")]);
        vrf.verify_node(&[bel.fwire_far("DQSIOWEN90P"), obel.fwire("MCB_T2")]);
        let obel = vrf.get_bel(bel.cell.with_row(row).bel(bels::IODELAY1));
        vrf.verify_node(&[bel.fwire_far(pp), obel.fwire("MCB_DQSOUTP")]);
        let obel = vrf.get_bel(bel.cell.with_row(row).bel(bels::IODELAY0));
        vrf.verify_node(&[bel.fwire_far(pn), obel.fwire("MCB_DQSOUTP")]);
    }

    for pin in [
        "IOIDRPCLK",
        "IOIDRPCS",
        "IOIDRPADD",
        "IOIDRPADDR0",
        "IOIDRPADDR1",
        "IOIDRPADDR2",
        "IOIDRPADDR3",
        "IOIDRPADDR4",
        "IOIDRPBROADCAST",
        "IOIDRPUPDATE",
        "IOIDRPSDO",
        "IOIDRPTRAIN",
    ] {
        vrf.claim_node(&[bel.fwire_far(pin)]);
    }

    for row in endev.chip.rows.ids() {
        if let Some(split) = endev.chip.row_mcb_split {
            if bel.row < split && row >= split {
                continue;
            }
            if bel.row >= split && row < split {
                continue;
            }
        }
        if let Some(obel) = vrf.find_bel(bel.cell.with_row(row).bel(bels::IOI)) {
            for (pin, opin) in [
                ("IOIDRPCLK", "MCB_DRPCLK"),
                ("IOIDRPCS", "MCB_DRPCS"),
                ("IOIDRPADD", "MCB_DRPADD"),
                ("IOIDRPBROADCAST", "MCB_DRPBROADCAST"),
                ("IOIDRPSDO", "MCB_DRPSDO"),
                ("IOIDRPTRAIN", "MCB_DRPTRAIN"),
            ] {
                vrf.verify_node(&[bel.fwire_far(pin), obel.fwire(opin)]);
            }
            for slot in bels::IODELAY {
                let oobel = vrf.find_bel_sibling(&obel, slot);
                for (pin, opin, dpin) in [
                    ("IOIDRPADDR0", "MCB_AUXADDR0", "AUXADDR0"),
                    ("IOIDRPADDR1", "MCB_AUXADDR1", "AUXADDR1"),
                    ("IOIDRPADDR2", "MCB_AUXADDR2", "AUXADDR2"),
                    ("IOIDRPADDR3", "MCB_AUXADDR3", "AUXADDR3"),
                    ("IOIDRPADDR4", "MCB_AUXADDR4", "AUXADDR4"),
                    ("IOIDRPUPDATE", "MCB_MEMUPDATE", "MEMUPDATE"),
                ] {
                    vrf.verify_node(&[bel.fwire_far(pin), oobel.fwire(opin)]);
                    vrf.claim_pip(oobel.crd(), oobel.wire(dpin), oobel.wire(opin));
                }
            }
            if !rows_handled.contains(&row) {
                vrf.claim_node(&[obel.fwire("MCB_T1")]);
                vrf.claim_node(&[obel.fwire("MCB_T2")]);
                for slot in bels::OLOGIC {
                    let oobel = vrf.find_bel_sibling(&obel, slot);
                    vrf.claim_node(&[oobel.fwire("MCB_D1")]);
                    vrf.claim_node(&[oobel.fwire("MCB_D2")]);
                }
            }
        }
    }
    let mut last = bel.fwire_far("IOIDRPSDI");
    for row in [
        mcb.io_dm[0].row,
        mcb.iop_dq[2],
        mcb.iop_dq[3],
        mcb.iop_dqs[0],
        mcb.iop_dq[1],
        mcb.iop_dq[0],
        mcb.iop_dq[4],
        mcb.iop_dq[5],
        mcb.iop_dqs[1],
        mcb.iop_dq[6],
        mcb.iop_dq[7],
    ] {
        for slot in [bels::IODELAY1, bels::IODELAY0] {
            let bel = vrf.get_bel(bel.cell.with_row(row).bel(slot));
            vrf.claim_node(&[last, bel.fwire("MCB_AUXSDO")]);
            vrf.claim_pip(bel.crd(), bel.wire("MCB_AUXSDO"), bel.wire("AUXSDO"));
            vrf.claim_pip(bel.crd(), bel.wire("AUXSDOIN"), bel.wire("MCB_AUXSDOIN"));
            last = bel.fwire("MCB_AUXSDOIN");
        }
    }
}

fn verify_mcb_tie(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mcb = endev.chip.get_mcb(bel.row);
    let (oslot, row) = match bel.slot {
        bels::MCB_TIE_CLK => (bels::TIEOFF_CLK, mcb.iop_clk),
        bels::MCB_TIE_DQS0 => (bels::TIEOFF_DQS0, mcb.iop_dqs[0]),
        bels::MCB_TIE_DQS1 => (bels::TIEOFF_DQS1, mcb.iop_dqs[1]),
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.crd(), bel.wire("OUTP0"), obel.wire("HARD0"));
    vrf.claim_pip(bel.crd(), bel.wire("OUTN0"), obel.wire("HARD1"));
    vrf.claim_pip(bel.crd(), bel.wire("OUTP1"), obel.wire("HARD1"));
    vrf.claim_pip(bel.crd(), bel.wire("OUTN1"), obel.wire("HARD0"));
    let o0 = vrf.get_bel(bel.cell.with_row(row).bel(bels::OLOGIC1));
    vrf.claim_node(&[bel.fwire("OUTP0"), o0.fwire("MCB_D1")]);
    vrf.claim_node(&[bel.fwire("OUTN0"), o0.fwire("MCB_D2")]);
    let o1 = vrf.get_bel(bel.cell.with_row(row).bel(bels::OLOGIC0));
    vrf.claim_node(&[bel.fwire("OUTP1"), o1.fwire("MCB_D1")]);
    vrf.claim_node(&[bel.fwire("OUTN1"), o1.fwire("MCB_D2")]);
}

fn verify_pci_ce_trunk_src(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut obel;
    if bel.row <= endev.chip.row_clk() {
        obel = vrf.find_bel_walk(bel, 0, 1, bels::PCI_CE_TRUNK_BUF);
        if let Some(ref ob) = obel
            && ob.row > endev.chip.row_clk()
        {
            obel = None;
        }
    } else {
        obel = vrf.find_bel_walk(bel, 0, -1, bels::PCI_CE_TRUNK_BUF);
        if let Some(ref ob) = obel
            && ob.row <= endev.chip.row_clk()
        {
            obel = None;
        }
    }
    if let Some(obel) = obel {
        vrf.verify_node(&[bel.fwire("PCI_CE_I"), obel.fwire("PCI_CE_O")]);
    } else {
        let obel = vrf.get_bel(
            bel.cell
                .with_row(endev.chip.row_clk())
                .bel(bels::PCILOGICSE),
        );
        vrf.verify_node(&[bel.fwire("PCI_CE_I"), obel.pip_owire("PCI_CE", 0)]);
    }
}

fn verify_pci_ce_trunk_buf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("PCI_CE_O")]);
    vrf.claim_pip(bel.crd(), bel.wire("PCI_CE_O"), bel.wire("PCI_CE_I"));
    verify_pci_ce_trunk_src(endev, vrf, bel);
}

fn verify_pci_ce_split(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("PCI_CE_O")]);
    vrf.claim_pip(bel.crd(), bel.wire("PCI_CE_O"), bel.wire("PCI_CE_I"));
    verify_pci_ce_trunk_src(endev, vrf, bel);
}

fn verify_pci_ce_v_src(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    bel: &BelContext<'_>,
    is_ioi: bool,
    ipin: &str,
) {
    let split_row = if bel.row <= endev.chip.row_clk() {
        endev.chip.rows_pci_ce_split.0
    } else {
        endev.chip.rows_pci_ce_split.1
    };
    let mut obel;
    if bel.row < split_row {
        obel = vrf.find_bel_walk(bel, 0, 1, bels::PCI_CE_V_BUF);
        if let Some(ref ob) = obel
            && ob.row > split_row
        {
            obel = None;
        }
    } else {
        obel = if is_ioi {
            vrf.find_bel_delta(bel, 0, 0, bels::PCI_CE_V_BUF)
        } else {
            None
        };
        if obel.is_none() {
            obel = vrf.find_bel_walk(bel, 0, -1, bels::PCI_CE_V_BUF);
        }
        if let Some(ref ob) = obel
            && ob.row < split_row
        {
            obel = None;
        }
    }
    let obel = obel
        .or_else(|| vrf.find_bel(bel.cell.with_row(split_row).bel(bels::PCI_CE_SPLIT)))
        .unwrap();
    vrf.verify_node(&[bel.fwire(ipin), obel.fwire("PCI_CE_O")]);
}

fn verify_pci_ce_v_buf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("PCI_CE_O")]);
    vrf.claim_pip(bel.crd(), bel.wire("PCI_CE_O"), bel.wire("PCI_CE_I"));
    verify_pci_ce_v_src(endev, vrf, bel, false, "PCI_CE_I");
}

fn verify_pci_ce_h_src(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    bel: &BelContext<'_>,
    ipin: &str,
) {
    let obel = if bel.col <= endev.chip.col_clk {
        vrf.find_bel_walk(bel, -1, 0, bels::PCI_CE_H_BUF).unwrap()
    } else {
        vrf.find_bel_walk(bel, 1, 0, bels::PCI_CE_H_BUF).unwrap()
    };
    vrf.verify_node(&[bel.fwire(ipin), obel.fwire("PCI_CE_O")]);
}

fn verify_pci_ce_h_buf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("PCI_CE_O")]);
    vrf.claim_pip(bel.crd(), bel.wire("PCI_CE_O"), bel.wire("PCI_CE_I"));
    if endev.chip.columns[bel.col].kind == ColumnKind::Io {
        verify_pci_ce_trunk_src(endev, vrf, bel);
    } else {
        verify_pci_ce_h_src(endev, vrf, bel, "PCI_CE_I");
    }
}

fn verify_btioi_clk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("PCI_CE_O")]);
    vrf.claim_pip(bel.crd(), bel.wire("PCI_CE_O"), bel.wire("PCI_CE_I"));
    verify_pci_ce_h_src(endev, vrf, bel, "PCI_CE_I");
    let bi = if bel.col <= endev.chip.col_clk {
        if bel.row == endev.chip.row_bio_outer() {
            4
        } else {
            0
        }
    } else {
        if bel.row == endev.chip.row_bio_outer() {
            0
        } else {
            4
        }
    };
    for i in 0..4 {
        vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}_O"))]);
        vrf.claim_node(&[bel.fwire(&format!("IOCE{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK{i}_O")),
            bel.wire(&format!("IOCLK{i}_I")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCE{i}_O")),
            bel.wire(&format!("IOCE{i}_I")),
        );
        let obel = vrf.get_bel(
            bel.cell
                .with_col(endev.chip.col_clk)
                .bel(bels::BUFIO2[bi + i]),
        );
        vrf.verify_node(&[bel.fwire(&format!("IOCLK{i}_I")), obel.fwire_far("IOCLK")]);
        vrf.verify_node(&[
            bel.fwire(&format!("IOCE{i}_I")),
            obel.fwire_far("SERDESSTROBE"),
        ]);
    }
    let obel = vrf.get_bel(bel.cell.with_col(endev.chip.col_clk).bel(bels::BUFPLL_BUF));
    for i in 0..2 {
        vrf.claim_node(&[bel.fwire(&format!("PLLCLK{i}_O"))]);
        vrf.claim_node(&[bel.fwire(&format!("PLLCE{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PLLCLK{i}_O")),
            bel.wire(&format!("PLLCLK{i}_I")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PLLCE{i}_O")),
            bel.wire(&format!("PLLCE{i}_I")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("PLLCLK{i}_I")),
            obel.fwire(&format!("PLLCLK{i}_O")),
        ]);
        vrf.verify_node(&[
            bel.fwire(&format!("PLLCE{i}_I")),
            obel.fwire(&format!("PLLCE{i}_O")),
        ]);
    }
}

fn verify_lrioi_clk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, bels::LRIOI_CLK_TERM);
    for ud in ['U', 'D'] {
        let by = if ud == 'D' { bel.row - 8 } else { bel.row };
        let mut found = false;
        for i in 0..8 {
            let row = by + i;
            if bel.col == endev.chip.col_w() {
                found |= endev.chip.rows[row].lio;
            } else {
                found |= endev.chip.rows[row].rio;
            }
        }
        if !found {
            continue;
        }
        for i in 0..4 {
            vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}_O_{ud}"))]);
            vrf.claim_node(&[bel.fwire(&format!("IOCE{i}_O_{ud}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("IOCLK{i}_O_{ud}")),
                bel.wire(&format!("IOCLK{i}_I")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("IOCE{i}_O_{ud}")),
                bel.wire(&format!("IOCE{i}_I")),
            );
            vrf.verify_node(&[
                bel.fwire(&format!("IOCLK{i}_I")),
                obel.fwire(&format!("IOCLK{i}_O")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("IOCE{i}_I")),
                obel.fwire(&format!("IOCE{i}_O")),
            ]);
        }
        for i in 0..2 {
            vrf.claim_node(&[bel.fwire(&format!("PLLCLK{i}_O_{ud}"))]);
            vrf.claim_node(&[bel.fwire(&format!("PLLCE{i}_O_{ud}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("PLLCLK{i}_O_{ud}")),
                bel.wire(&format!("PLLCLK{i}_I")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("PLLCE{i}_O_{ud}")),
                bel.wire(&format!("PLLCE{i}_I")),
            );
            vrf.verify_node(&[
                bel.fwire(&format!("PLLCLK{i}_I")),
                obel.fwire(&format!("PLLCLK{i}_O")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("PLLCE{i}_I")),
                obel.fwire(&format!("PLLCE{i}_O")),
            ]);
        }
    }
}

fn verify_lrioi_clk_term(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut found = false;
    for i in 0..16 {
        let row = bel.row - 8 + i;
        if bel.col == endev.chip.col_w() {
            found |= endev.chip.rows[row].lio;
        } else {
            found |= endev.chip.rows[row].rio;
        }
    }
    if !found {
        return;
    }
    let bi = if bel.row <= endev.chip.row_clk() {
        if bel.col == endev.chip.col_w() { 0 } else { 4 }
    } else {
        if bel.col == endev.chip.col_w() { 4 } else { 0 }
    };
    for i in 0..4 {
        vrf.claim_node(&[bel.fwire(&format!("IOCLK{i}_O"))]);
        vrf.claim_node(&[bel.fwire(&format!("IOCE{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCLK{i}_O")),
            bel.wire(&format!("IOCLK{i}_I")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("IOCE{i}_O")),
            bel.wire(&format!("IOCE{i}_I")),
        );
        let obel = vrf.get_bel(
            bel.cell
                .with_row(endev.chip.row_clk())
                .bel(bels::BUFIO2[bi + i]),
        );
        vrf.verify_node(&[bel.fwire(&format!("IOCLK{i}_I")), obel.fwire_far("IOCLK")]);
        vrf.verify_node(&[
            bel.fwire(&format!("IOCE{i}_I")),
            obel.fwire_far("SERDESSTROBE"),
        ]);
    }
    let obel = vrf.get_bel(
        bel.cell
            .with_row(endev.chip.row_clk())
            .bel(bels::BUFPLL_BUF),
    );
    for i in 0..2 {
        vrf.claim_node(&[bel.fwire(&format!("PLLCLK{i}_O"))]);
        vrf.claim_node(&[bel.fwire(&format!("PLLCE{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PLLCLK{i}_O")),
            bel.wire(&format!("PLLCLK{i}_I")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PLLCE{i}_O")),
            bel.wire(&format!("PLLCE{i}_I")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("PLLCLK{i}_I")),
            obel.fwire(&format!("PLLCLK{i}_O")),
        ]);
        vrf.verify_node(&[
            bel.fwire(&format!("PLLCE{i}_I")),
            obel.fwire(&format!("PLLCE{i}_O")),
        ]);
    }
}

fn verify_bufh(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFH",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_node(&[bel.fwire_far("I")]);
    vrf.claim_node(&[bel.fwire_far("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire_far("I"));
    vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
    let idx = bels::BUFH_E
        .into_iter()
        .position(|x| x == bel.slot)
        .unwrap_or_else(|| {
            bels::BUFH_W
                .into_iter()
                .position(|x| x == bel.slot)
                .unwrap()
        });
    let obel = vrf.find_bel_sibling(bel, bels::HCLK_ROW);
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("I"),
        obel.wire(&format!("BUFG{idx}")),
    );
    if bel.row != endev.chip.row_clk() {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("I"),
            obel.wire(&format!("CMT{idx}")),
        );
    }
}

fn verify_hclk_row(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let srow = if bel.row <= endev.chip.row_clk() {
        endev.chip.rows_hclkbuf.0
    } else {
        endev.chip.rows_hclkbuf.1
    };
    let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::HCLK_V_MIDBUF));
    for i in 0..16 {
        vrf.verify_node(&[
            bel.fwire(&format!("BUFG{i}")),
            obel.fwire(&format!("GCLK{i}_O")),
        ]);
    }
    if bel.row != endev.chip.row_clk() {
        let obel = vrf.find_bel_sibling(bel, bels::CMT);
        for i in 0..16 {
            vrf.verify_node(&[
                bel.fwire(&format!("CMT{i}")),
                obel.fwire(&format!("HCLK{i}")),
            ]);
        }
    }
}

fn verify_hclk_v_midbuf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..16 {
        vrf.claim_node(&[bel.fwire(&format!("GCLK{i}_O"))]);
        vrf.claim_node(&[bel.fwire(&format!("GCLK{i}_M"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_O")),
            bel.wire(&format!("GCLK{i}_M")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_M")),
            bel.wire(&format!("GCLK{i}_I")),
        );
        let obel = vrf.get_bel(
            bel.cell
                .with_row(endev.chip.row_clk())
                .bel(bels::BUFGMUX[i]),
        );
        vrf.verify_node(&[bel.fwire(&format!("GCLK{i}_I")), obel.fwire_far("O")]);
    }
}

fn verify_hclk_h_midbuf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let slots = if bel.col < endev.chip.col_clk {
        bels::BUFH_W
    } else {
        bels::BUFH_E
    };
    for i in 0..16 {
        vrf.claim_node(&[bel.fwire(&format!("GCLK{i}_O"))]);
        vrf.claim_node(&[bel.fwire(&format!("GCLK{i}_M"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_O")),
            bel.wire(&format!("GCLK{i}_M")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_M")),
            bel.wire(&format!("GCLK{i}_I")),
        );
        let obel = vrf.get_bel(bel.cell.with_col(endev.chip.col_clk).bel(slots[i]));
        vrf.verify_node(&[bel.fwire(&format!("GCLK{i}_I")), obel.fwire_far("O")]);
    }
}

fn verify_hclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..16 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_O_D")),
            bel.wire(&format!("GCLK{i}_I")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GCLK{i}_O_U")),
            bel.wire(&format!("GCLK{i}_I")),
        );
    }
    if let Some((col_l, col_r)) = endev.chip.cols_clk_fold {
        let col = if bel.col <= endev.chip.col_clk {
            col_l
        } else {
            col_r
        };
        let obel = vrf.get_bel(bel.cell.with_col(col).bel(bels::HCLK_H_MIDBUF));
        for i in 0..16 {
            vrf.verify_node(&[
                bel.fwire(&format!("GCLK{i}_I")),
                obel.fwire(&format!("GCLK{i}_O")),
            ]);
        }
    } else {
        let slots = if bel.col <= endev.chip.col_clk {
            bels::BUFH_W
        } else {
            bels::BUFH_E
        };
        for i in 0..16 {
            let obel = vrf.get_bel(bel.cell.with_col(endev.chip.col_clk).bel(slots[i]));
            vrf.verify_node(&[bel.fwire(&format!("GCLK{i}_I")), obel.fwire_far("O")]);
        }
    }
}

fn verify_bufgmux(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::BUFGMUX
        .into_iter()
        .position(|x| x == bel.slot)
        .unwrap();
    vrf.verify_bel(
        bel,
        "BUFGMUX",
        &[
            ("I0", SitePinDir::In),
            ("I1", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I0")]);
    vrf.claim_node(&[bel.fwire("I1")]);
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_node(&[bel.fwire_far("O")]);
    vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
    let obel = vrf.find_bel_sibling(bel, bels::CLKC);
    let swz = [0, 1, 2, 4, 3, 5, 6, 7, 8, 9, 10, 12, 11, 13, 14, 15];
    let i0 = swz[idx];
    let i1 = swz[idx ^ 1];
    vrf.claim_pip(bel.crd(), bel.wire("I0"), obel.wire(&format!("MUX{i0}")));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), obel.wire(&format!("MUX{i1}")));
}

fn verify_clkc(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_ckpin_l = vrf.get_bel(
        bel.cell
            .with_col(endev.chip.cols_reg_buf.0)
            .bel(bels::CKPIN_H_MIDBUF),
    );
    let obel_ckpin_r = vrf.get_bel(
        bel.cell
            .with_col(endev.chip.cols_reg_buf.1)
            .bel(bels::CKPIN_H_MIDBUF),
    );
    let obel_ckpin_b = vrf.get_bel(
        bel.cell
            .with_row(endev.chip.rows_midbuf.0)
            .bel(bels::CKPIN_V_MIDBUF),
    );
    let obel_ckpin_t = vrf.get_bel(
        bel.cell
            .with_row(endev.chip.rows_midbuf.1)
            .bel(bels::CKPIN_V_MIDBUF),
    );
    let obel_d = vrf.find_bel_walk(bel, 0, -8, bels::CMT).unwrap();
    let obel_u = vrf.find_bel_walk(bel, 0, 8, bels::CMT).unwrap();
    for i in 0..16 {
        vrf.claim_node(&[bel.fwire(&format!("MUX{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MUX{i}")),
            bel.wire(&format!("CKPIN_H{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MUX{i}")),
            bel.wire(&format!("CKPIN_V{i}")),
        );
        if i < 8 {
            vrf.verify_node(&[
                bel.fwire(&format!("CKPIN_H{i}")),
                obel_ckpin_r.fwire(&format!("CKPIN{i}_O")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("CKPIN_V{i}")),
                obel_ckpin_t.fwire(&format!("CKPIN{i}_O")),
            ]);
        } else {
            vrf.verify_node(&[
                bel.fwire(&format!("CKPIN_H{i}")),
                obel_ckpin_l.fwire(&format!("CKPIN{ii}_O", ii = i - 8)),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("CKPIN_V{i}")),
                obel_ckpin_b.fwire(&format!("CKPIN{ii}_O", ii = i - 8)),
            ]);
        }
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MUX{i}")),
            bel.wire(&format!("CMT_U{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MUX{i}")),
            bel.wire(&format!("CMT_D{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("CMT_D{i}")),
            obel_d.fwire(&format!("CASC{i}_O")),
        ]);
        vrf.verify_node(&[
            bel.fwire(&format!("CMT_U{i}")),
            obel_u.fwire(&format!("CASC{i}_O")),
        ]);
    }
}

fn verify_ckpin_v_midbuf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let srow = if bel.row < endev.chip.row_clk() {
        endev.chip.row_bio_outer()
    } else {
        endev.chip.row_tio_outer()
    };
    for i in 0..8 {
        vrf.claim_node(&[bel.fwire(&format!("CKPIN{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CKPIN{i}_O")),
            bel.wire(&format!("CKPIN{i}_I")),
        );
        let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::BUFIO2[i]));
        vrf.verify_node(&[bel.fwire(&format!("CKPIN{i}_I")), obel.fwire("CKPIN")]);
    }
}

fn verify_ckpin_h_midbuf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let scol = if bel.col < endev.chip.col_clk {
        endev.chip.col_w()
    } else {
        endev.chip.col_e()
    };
    for i in 0..8 {
        vrf.claim_node(&[bel.fwire(&format!("CKPIN{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CKPIN{i}_O")),
            bel.wire(&format!("CKPIN{i}_I")),
        );
        let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::BUFIO2[i]));
        vrf.verify_node(&[bel.fwire(&format!("CKPIN{i}_I")), obel.fwire("CKPIN")]);
    }
}

fn verify_bufio2_ins(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let source_iois = if bel.col == endev.chip.col_w() {
        [
            (bel.col, bel.row - 2),
            (bel.col, bel.row - 1),
            (bel.col, bel.row + 2),
            (bel.col, bel.row + 3),
        ]
    } else if bel.col == endev.chip.col_e() {
        [
            (bel.col, bel.row + 3),
            (bel.col, bel.row + 2),
            (bel.col, bel.row - 1),
            (bel.col, bel.row - 2),
        ]
    } else if bel.row == endev.chip.row_bio_outer() {
        [
            (bel.col + 1, bel.row),
            (bel.col + 1, bel.row + 1),
            (bel.col, bel.row),
            (bel.col, bel.row + 1),
        ]
    } else if bel.row == endev.chip.row_tio_outer() {
        [
            (bel.col, bel.row),
            (bel.col, bel.row - 1),
            (bel.col + 1, bel.row),
            (bel.col + 1, bel.row - 1),
        ]
    } else {
        unreachable!()
    };
    for (i, (col, row)) in source_iois.into_iter().enumerate() {
        let obel = vrf.get_bel(bel.cell.with_cr(col, row).bel(bels::CLKPIN_BUF));
        for pin in ["CLKPIN", "DFB", "CFB0_", "CFB1_"] {
            vrf.verify_node(&[
                bel.fwire(&format!("{pin}{ii}", ii = i * 2)),
                obel.fwire(&format!("{pin}0_O")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("{pin}{ii}", ii = i * 2 + 1)),
                obel.fwire(&format!("{pin}1_O")),
            ]);
        }
        vrf.verify_node(&[bel.fwire(&format!("DQSP{i}")), obel.fwire("DQSP_O")]);
        vrf.verify_node(&[bel.fwire(&format!("DQSN{i}")), obel.fwire("DQSN_O")]);
    }
    if bel.row == endev.chip.row_bio_outer() {
        let mut found_l = false;
        let mut found_r = false;
        if let Gts::Quad(cl, cr) = endev.chip.gts {
            if let Some(obel) = vrf.find_bel(bel.cell.with_col(cl).bel(bels::GTP_BUF)) {
                for i in 0..4 {
                    let ii = i + 4;
                    vrf.verify_node(&[
                        bel.fwire(&format!("GTPCLK{ii}")),
                        obel.fwire(&format!("GTPCLK{i}_O")),
                    ]);
                    vrf.verify_node(&[
                        bel.fwire(&format!("GTPFB{ii}")),
                        obel.fwire(&format!("GTPFB{i}_O")),
                    ]);
                }
                found_l = true;
            }
            if let Some(obel) = vrf.find_bel(bel.cell.with_col(cr).bel(bels::GTP_BUF)) {
                for i in 0..4 {
                    vrf.verify_node(&[
                        bel.fwire(&format!("GTPCLK{i}")),
                        obel.fwire(&format!("GTPCLK{i}_O")),
                    ]);
                    vrf.verify_node(&[
                        bel.fwire(&format!("GTPFB{i}")),
                        obel.fwire(&format!("GTPFB{i}_O")),
                    ]);
                }
                found_r = true;
            }
        }
        if !found_l {
            for i in 4..8 {
                vrf.claim_node(&[bel.fwire(&format!("GTPCLK{i}"))]);
                vrf.claim_node(&[bel.fwire(&format!("GTPFB{i}"))]);
            }
        }
        if !found_r {
            for i in 0..4 {
                vrf.claim_node(&[bel.fwire(&format!("GTPCLK{i}"))]);
                vrf.claim_node(&[bel.fwire(&format!("GTPFB{i}"))]);
            }
        }
    } else if bel.row == endev.chip.row_tio_outer() {
        if let Gts::Single(cl) | Gts::Double(cl, _) | Gts::Quad(cl, _) = endev.chip.gts
            && let Some(obel) = vrf.find_bel(bel.cell.with_col(cl).bel(bels::GTP_BUF))
        {
            for i in 0..4 {
                vrf.verify_node(&[
                    bel.fwire(&format!("GTPCLK{i}")),
                    obel.fwire(&format!("GTPCLK{i}_O")),
                ]);
                vrf.verify_node(&[
                    bel.fwire(&format!("GTPFB{i}")),
                    obel.fwire(&format!("GTPFB{i}_O")),
                ]);
            }
        } else {
            for i in 0..4 {
                vrf.claim_node(&[bel.fwire(&format!("GTPCLK{i}"))]);
                vrf.claim_node(&[bel.fwire(&format!("GTPFB{i}"))]);
            }
        }
        if let Gts::Double(_, cr) | Gts::Quad(_, cr) = endev.chip.gts
            && let Some(obel) = vrf.find_bel(bel.cell.with_col(cr).bel(bels::GTP_BUF))
        {
            for i in 0..4 {
                let ii = i + 4;
                vrf.verify_node(&[
                    bel.fwire(&format!("GTPCLK{ii}")),
                    obel.fwire(&format!("GTPCLK{i}_O")),
                ]);
                vrf.verify_node(&[
                    bel.fwire(&format!("GTPFB{ii}")),
                    obel.fwire(&format!("GTPFB{i}_O")),
                ]);
            }
        } else {
            for i in 4..8 {
                vrf.claim_node(&[bel.fwire(&format!("GTPCLK{i}"))]);
                vrf.claim_node(&[bel.fwire(&format!("GTPFB{i}"))]);
            }
        }
    } else {
        for i in 0..8 {
            vrf.claim_node(&[bel.fwire(&format!("GTPCLK{i}"))]);
            vrf.claim_node(&[bel.fwire(&format!("GTPFB{i}"))]);
        }
    }
}

fn verify_bufio2_ckpin(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_ins = vrf.find_bel_sibling(bel, bels::BUFIO2_INS);
    for i in 0..8 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CKPIN{i}")),
            bel.wire(&format!("CLKPIN{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("CLKPIN{i}")),
            obel_ins.fwire(&format!("CLKPIN{i}")),
        ]);
        let obel = vrf.find_bel_sibling(bel, bels::BUFIO2[i]);
        vrf.verify_node(&[bel.fwire(&format!("CKPIN{i}")), obel.fwire("CKPIN")]);
    }
}

fn verify_bufio2(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFIO2",
        &[
            ("I", SitePinDir::In),
            ("IB", SitePinDir::In),
            ("IOCLK", SitePinDir::Out),
            ("DIVCLK", SitePinDir::Out),
            ("SERDESSTROBE", SitePinDir::Out),
        ],
        &[],
    );
    for pin in ["I", "IB", "IOCLK", "DIVCLK", "SERDESSTROBE", "CMT", "CKPIN"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    for pin in ["IOCLK", "DIVCLK", "SERDESSTROBE"] {
        vrf.claim_pip(bel.crd(), bel.wire_far(pin), bel.wire(pin));
        vrf.claim_node(&[bel.fwire_far(pin)]);
    }

    let obel_ins = vrf.find_bel_sibling(bel, bels::BUFIO2_INS);
    let obel_tie = vrf.find_bel_sibling(bel, bels::TIEOFF_REG);
    let idx = bels::BUFIO2
        .into_iter()
        .position(|x| x == bel.slot)
        .unwrap();
    let clkpins = if matches!(idx, 0 | 1 | 4 | 5) {
        ["CLKPIN0", "CLKPIN1", "CLKPIN4", "CLKPIN5"]
    } else {
        ["CLKPIN2", "CLKPIN3", "CLKPIN6", "CLKPIN7"]
    };
    for pin in clkpins {
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire(pin));
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I"),
        obel_ins.wire(&format!("DFB{idx}")),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("IB"),
        obel_ins.wire(&format!("DFB{i}", i = idx ^ 1)),
    );
    let gi = if endev.chip.columns[bel.col].kind == ColumnKind::Io {
        match idx {
            1 => 0,
            3 => 2,
            _ => idx,
        }
    } else {
        idx
    };
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I"),
        obel_ins.wire(&format!("GTPCLK{gi}")),
    );
    match idx {
        0 | 4 => {
            vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire("DQSP0"));
            vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire("DQSP2"));
            vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire("DQSN0"));
            vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire("DQSN2"));
        }
        1 | 5 => {
            vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire("DQSN0"));
            vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire("DQSN2"));
            vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire("DQSP0"));
            vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire("DQSP2"));
        }
        2 | 6 => {
            vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire("DQSP1"));
            vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire("DQSP3"));
            vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire("DQSN1"));
            vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire("DQSN3"));
        }
        3 | 7 => {
            vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire("DQSN1"));
            vrf.claim_pip(bel.crd(), bel.wire("I"), obel_ins.wire("DQSN3"));
            vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire("DQSP1"));
            vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_ins.wire("DQSP3"));
        }
        _ => unreachable!(),
    }
    vrf.claim_pip(bel.crd(), bel.wire("IB"), obel_tie.wire("HARD1"));

    vrf.claim_pip(bel.crd(), bel.wire("CKPIN"), bel.wire_far("DIVCLK"));
    vrf.claim_pip(bel.crd(), bel.wire("CKPIN"), obel_tie.wire("HARD1"));
    vrf.claim_pip(bel.crd(), bel.wire("CMT"), bel.wire_far("DIVCLK"));
    vrf.claim_pip(bel.crd(), bel.wire("CMT"), obel_tie.wire("HARD1"));

    vrf.claim_pip(bel.crd(), bel.wire_far("IOCLK"), obel_tie.wire("HARD0"));
}

fn verify_bufio2fb(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFIO2FB",
        &[
            ("I", SitePinDir::In),
            ("IB", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &[],
    );
    for pin in ["I", "IB", "O", "CMT"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_ins = vrf.find_bel_sibling(bel, bels::BUFIO2_INS);
    let idx = bels::BUFIO2FB
        .into_iter()
        .position(|x| x == bel.slot)
        .unwrap();
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I"),
        obel_ins.wire(&format!("CLKPIN{i}", i = idx ^ 1)),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I"),
        obel_ins.wire(&format!("DFB{idx}")),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I"),
        obel_ins.wire(&format!("CFB0_{idx}")),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("IB"),
        obel_ins.wire(&format!("CFB1_{idx}")),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("I"),
        obel_ins.wire(&format!("GTPFB{idx}")),
    );

    let obel_tie = vrf.find_bel_sibling(bel, bels::TIEOFF_REG);
    vrf.claim_pip(bel.crd(), bel.wire("CMT"), bel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("CMT"), obel_tie.wire("HARD1"));
}

fn verify_bufpll(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFPLL",
        &[
            ("PLLIN", SitePinDir::In),
            ("GCLK", SitePinDir::In),
            ("LOCKED", SitePinDir::In),
            ("IOCLK", SitePinDir::Out),
            ("SERDESSTROBE", SitePinDir::Out),
            ("LOCK", SitePinDir::Out),
        ],
        &[],
    );
    for pin in ["PLLIN", "GCLK", "LOCKED", "IOCLK", "SERDESSTROBE", "LOCK"] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let idx = bels::BUFPLL
        .into_iter()
        .position(|x| x == bel.slot)
        .unwrap();
    if endev.chip.columns[bel.col].kind == ColumnKind::Io {
        let obel = vrf.find_bel_sibling(bel, bels::BUFPLL_INS_LR);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("GCLK"),
            obel.wire(&format!("GCLK{idx}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("PLLIN"),
            obel.wire(&format!("PLLIN{idx}_GCLK")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("PLLIN"),
            obel.wire(&format!("PLLIN{idx}_CMT")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("LOCKED"),
            obel.wire(&format!("LOCKED{idx}")),
        );
    } else {
        let obel = vrf.find_bel_sibling(bel, bels::BUFPLL_INS_BT);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("GCLK"),
            obel.wire(&format!("GCLK{idx}")),
        );
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("PLLIN"),
                obel.wire(&format!("PLLIN{i}")),
            );
        }
        for i in 0..3 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("LOCKED"),
                obel.wire(&format!("LOCKED{i}")),
            );
        }
    }
}

fn verify_bufpll_mcb(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFPLL_MCB",
        &[
            ("PLLIN0", SitePinDir::In),
            ("PLLIN1", SitePinDir::In),
            ("GCLK", SitePinDir::In),
            ("LOCKED", SitePinDir::In),
            ("IOCLK0", SitePinDir::Out),
            ("IOCLK1", SitePinDir::Out),
            ("SERDESSTROBE0", SitePinDir::Out),
            ("SERDESSTROBE1", SitePinDir::Out),
            ("LOCK", SitePinDir::Out),
        ],
        &[],
    );
    for pin in [
        "PLLIN0",
        "PLLIN1",
        "GCLK",
        "LOCKED",
        "IOCLK0",
        "IOCLK1",
        "SERDESSTROBE0",
        "SERDESSTROBE1",
        "LOCK",
    ] {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    if endev.chip.columns[bel.col].kind == ColumnKind::Io {
        let obel = vrf.find_bel_sibling(bel, bels::BUFPLL_INS_LR);
        vrf.claim_pip(bel.crd(), bel.wire("GCLK"), obel.wire("GCLK0"));
        vrf.claim_pip(bel.crd(), bel.wire("PLLIN0"), obel.wire("PLLIN0_GCLK"));
        vrf.claim_pip(bel.crd(), bel.wire("PLLIN0"), obel.wire("PLLIN0_CMT"));
        vrf.claim_pip(bel.crd(), bel.wire("PLLIN1"), obel.wire("PLLIN1_GCLK"));
        vrf.claim_pip(bel.crd(), bel.wire("PLLIN1"), obel.wire("PLLIN1_CMT"));
        vrf.claim_pip(bel.crd(), bel.wire("LOCKED"), obel.wire("LOCKED0"));
    } else {
        let obel = vrf.find_bel_sibling(bel, bels::BUFPLL_INS_BT);
        vrf.claim_pip(bel.crd(), bel.wire("GCLK"), obel.wire("GCLK0"));
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("PLLIN0"),
                obel.wire(&format!("PLLIN{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("PLLIN1"),
                obel.wire(&format!("PLLIN{i}")),
            );
        }
        for i in 0..3 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("LOCKED"),
                obel.wire(&format!("LOCKED{i}")),
            );
        }
    }
}

fn verify_bufpll_out(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel0 = vrf.find_bel_sibling(bel, bels::BUFPLL0);
    let obel1 = vrf.find_bel_sibling(bel, bels::BUFPLL1);
    let obel_mcb = vrf.find_bel_sibling(bel, bels::BUFPLL_MCB);
    let obel_tie = vrf.find_bel_sibling(bel, bels::TIEOFF_REG);
    vrf.claim_node(&[bel.fwire("PLLCLK0")]);
    vrf.claim_node(&[bel.fwire("PLLCLK1")]);
    vrf.claim_node(&[bel.fwire("PLLCE0")]);
    vrf.claim_node(&[bel.fwire("PLLCE1")]);
    vrf.claim_pip(bel.crd(), bel.wire("PLLCLK0"), obel0.wire("IOCLK"));
    vrf.claim_pip(bel.crd(), bel.wire("PLLCLK0"), obel_mcb.wire("IOCLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("PLLCLK0"), obel_tie.wire("HARD1"));
    vrf.claim_pip(bel.crd(), bel.wire("PLLCLK1"), obel1.wire("IOCLK"));
    vrf.claim_pip(bel.crd(), bel.wire("PLLCLK1"), obel_mcb.wire("IOCLK1"));
    vrf.claim_pip(bel.crd(), bel.wire("PLLCLK1"), obel_tie.wire("HARD1"));
    vrf.claim_pip(bel.crd(), bel.wire("PLLCE0"), obel0.wire("SERDESSTROBE"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("PLLCE0"),
        obel_mcb.wire("SERDESSTROBE0"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("PLLCE1"), obel1.wire("SERDESSTROBE"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("PLLCE1"),
        obel_mcb.wire("SERDESSTROBE1"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("LOCK0"), obel0.wire("LOCK"));
    vrf.claim_pip(bel.crd(), bel.wire("LOCK0"), obel_mcb.wire("LOCK"));
    vrf.claim_pip(bel.crd(), bel.wire("LOCK1"), obel1.wire("LOCK"));
}

fn verify_bufpll_buf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, bels::BUFPLL_OUT);
    for i in 0..2 {
        vrf.claim_node(&[bel.fwire(&format!("PLLCLK{i}_O"))]);
        vrf.claim_node(&[bel.fwire(&format!("PLLCE{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PLLCLK{i}_O")),
            bel.wire(&format!("PLLCLK{i}_I")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("PLLCE{i}_O")),
            bel.wire(&format!("PLLCE{i}_I")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("PLLCLK{i}_I")),
            obel.fwire(&format!("PLLCLK{i}")),
        ]);
        vrf.verify_node(&[
            bel.fwire(&format!("PLLCE{i}_I")),
            obel.fwire(&format!("PLLCE{i}")),
        ]);
    }
}

fn verify_bufpll_ins_lr(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.get_bel(bel.cell.with_col(endev.chip.col_clk).bel(bels::CLKC_BUFPLL));
    let lr = if bel.col < endev.chip.col_clk {
        'L'
    } else {
        'R'
    };
    for (pin, opin) in [
        ("PLLIN0_CMT", "CLKOUT0"),
        ("PLLIN1_CMT", "CLKOUT1"),
        ("LOCKED0", "LOCKED0"),
        ("LOCKED1", "LOCKED1"),
    ] {
        vrf.verify_node(&[bel.fwire(pin), obel.fwire(&format!("OUT{lr}_{opin}"))]);
    }
}

fn verify_bufpll_ins_bt(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if bel.row == endev.chip.row_bio_outer() {
        let obel = vrf
            .find_bel_delta(bel, 0, 8, bels::DCM_BUFPLL_BUF_S)
            .unwrap();
        for (pin, opin) in [
            ("PLLIN0", "PLL0_CLKOUT0_O"),
            ("PLLIN1", "PLL0_CLKOUT1_O"),
            ("PLLIN2", "PLL1_CLKOUT0_O"),
            ("PLLIN3", "PLL1_CLKOUT1_O"),
            ("PLLIN4", "CLKC_CLKOUT0_O"),
            ("PLLIN5", "CLKC_CLKOUT1_O"),
            ("LOCKED0", "PLL0_LOCKED_O"),
            ("LOCKED1", "PLL1_LOCKED_O"),
            ("LOCKED2", "CLKC_LOCKED_O"),
        ] {
            vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
        }
    } else {
        let obel = vrf.find_bel_delta(bel, 0, -7, bels::PLL_BUFPLL).unwrap();
        for (pin, opin) in [
            ("PLLIN2", "CLKOUT0_U"),
            ("PLLIN3", "CLKOUT1_U"),
            ("LOCKED1", "LOCKED_U"),
        ] {
            vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
        }
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -23, bels::DCM_BUFPLL_BUF_N_MID) {
            for (pin, opin) in [
                ("PLLIN0", "PLL0_CLKOUT0_O"),
                ("PLLIN1", "PLL0_CLKOUT1_O"),
                ("PLLIN4", "CLKC_CLKOUT0_O"),
                ("PLLIN5", "CLKC_CLKOUT1_O"),
                ("LOCKED0", "PLL0_LOCKED_O"),
                ("LOCKED2", "CLKC_LOCKED_O"),
            ] {
                vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
            }
        } else {
            let obel = vrf
                .find_bel_delta(bel, 0, -23, bels::DCM_BUFPLL_BUF_N)
                .unwrap();
            for (pin, opin) in [
                ("PLLIN0", "PLL0_CLKOUT0_I"),
                ("PLLIN1", "PLL0_CLKOUT1_I"),
                ("PLLIN4", "CLKC_CLKOUT0_O"),
                ("PLLIN5", "CLKC_CLKOUT1_O"),
                ("LOCKED0", "PLL0_LOCKED_I"),
                ("LOCKED2", "CLKC_LOCKED_O"),
            ] {
                vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
            }
        }
    }
}

fn verify_dcm(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = match bel.slot {
        bels::DCM0 => 0,
        bels::DCM1 => 1,
        _ => unreachable!(),
    };
    let pins = [
        ("CLK0", SitePinDir::Out),
        ("CLK90", SitePinDir::Out),
        ("CLK180", SitePinDir::Out),
        ("CLK270", SitePinDir::Out),
        ("CLK2X", SitePinDir::Out),
        ("CLK2X180", SitePinDir::Out),
        ("CLKFX", SitePinDir::Out),
        ("CLKFX180", SitePinDir::Out),
        ("CLKDV", SitePinDir::Out),
        ("CONCUR", SitePinDir::Out),
        ("CLKIN", SitePinDir::In),
        ("CLKFB", SitePinDir::In),
        ("SKEWCLKIN1", SitePinDir::In),
        ("SKEWCLKIN2", SitePinDir::In),
    ];
    vrf.verify_bel(
        bel,
        "DCM",
        &pins,
        &[
            "CLKIN_CKINT0",
            "CLKIN_CKINT1",
            "CLKFB_CKINT0",
            "CLKFB_CKINT1",
        ],
    );
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let out_pins = [
        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
        "CONCUR",
    ];

    for &pin in &out_pins {
        vrf.claim_node(&[bel.fwire(&format!("{pin}_OUT"))]);
        vrf.claim_node(&[bel.fwire(&format!("{pin}_TEST"))]);
        vrf.claim_pip(bel.crd(), bel.wire(&format!("{pin}_OUT")), bel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("{pin}_TEST")), bel.wire(pin));
    }

    let obel = vrf.find_bel_sibling(bel, bels::CMT);

    vrf.claim_node(&[bel.fwire("CLKIN_TEST")]);
    vrf.claim_node(&[bel.fwire("CLKFB_TEST")]);
    for opin in ["CLKIN", "CLKIN_TEST"] {
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("CLKIN_CKINT0"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("CLKIN_CKINT1"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("CLK_FROM_PLL"));
        for i in 0..8 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(opin),
                obel.wire(&format!("BUFIO2_BT{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(opin),
                obel.wire(&format!("BUFIO2_LR{i}")),
            );
        }
    }
    for opin in ["CLKFB", "CLKFB_TEST"] {
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("CLKFB_CKINT0"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("CLKFB_CKINT1"));
        for i in 0..8 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(opin),
                obel.wire(&format!("BUFIO2FB_BT{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(opin),
                obel.wire(&format!("BUFIO2FB_LR{i}")),
            );
        }
    }

    vrf.claim_node(&[bel.fwire("CLK_TO_PLL")]);
    for &pin in &out_pins {
        vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_PLL"), bel.wire(pin));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("SKEWCLKIN2"),
            bel.wire(&format!("{pin}_TEST")),
        );
    }
    vrf.claim_pip(bel.crd(), bel.wire("SKEWCLKIN1"), bel.wire("CLK_TO_PLL"));

    let obel_pll = vrf.find_bel_delta(bel, 0, 16, bels::PLL).unwrap();
    vrf.verify_node(&[
        bel.fwire("CLK_FROM_PLL"),
        obel_pll.fwire(&format!("CLK_TO_DCM{idx}")),
    ]);
}

fn verify_cmt(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_delta(
        bel,
        0,
        if bel.row < endev.chip.row_clk() {
            -16
        } else {
            16
        },
        bels::CMT,
    );
    for i in 0..16 {
        vrf.claim_node(&[bel.fwire(&format!("HCLK{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}")),
            bel.wire(&format!("HCLK{i}_CKINT")),
        );

        vrf.claim_node(&[bel.fwire(&format!("HCLK{i}_BUF"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK{i}_BUF")),
            bel.wire(&format!("HCLK{i}")),
        );

        vrf.claim_node(&[bel.fwire(&format!("CASC{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASC{i}_O")),
            bel.wire(&format!("HCLK{i}_BUF")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("CASC{i}_O")),
            bel.wire(&format!("CASC{i}_I")),
        );
        if let Some(ref obel) = obel {
            vrf.verify_node(&[
                bel.fwire(&format!("CASC{i}_I")),
                obel.fwire(&format!("CASC{i}_O")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("CASC{i}_I"))]);
        }
    }
    if let Some(obel_pll) = vrf.find_bel_delta(bel, 0, 0, bels::PLL) {
        for i in 0..16 {
            for pin in [
                "CLKOUT0",
                "CLKOUT1",
                "CLKOUT2",
                "CLKOUT3",
                "CLKOUT4",
                "CLKOUT5",
                "TEST_CLK_OUT",
            ] {
                vrf.claim_pip(bel.crd(), bel.wire(&format!("HCLK{i}")), obel_pll.wire(pin));
            }
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HCLK{i}")),
                obel_pll.wire_far("CLKFBOUT"),
            );
        }
    } else {
        let obel_dcm0 = vrf.find_bel_sibling(bel, bels::DCM0);
        let obel_dcm1 = vrf.find_bel_sibling(bel, bels::DCM1);
        for i in 0..16 {
            for pin in [
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ] {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("HCLK{i}")),
                    obel_dcm0.wire(&format!("{pin}_OUT")),
                );
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("HCLK{i}")),
                    obel_dcm1.wire(&format!("{pin}_OUT")),
                );
            }
        }
    }

    for i in 0..8 {
        if bel.row < endev.chip.row_clk() {
            let obel = vrf.get_bel(
                bel.cell
                    .with_row(endev.chip.row_bio_outer())
                    .bel(bels::BUFIO2[i]),
            );
            vrf.verify_node(&[bel.fwire(&format!("BUFIO2_BT{i}")), obel.fwire("CMT")]);
            let obel = vrf.get_bel(
                bel.cell
                    .with_row(endev.chip.row_bio_outer())
                    .bel(bels::BUFIO2FB[i]),
            );
            vrf.verify_node(&[bel.fwire(&format!("BUFIO2FB_BT{i}")), obel.fwire("CMT")]);
            let scol = if i < 4 {
                endev.chip.col_e()
            } else {
                endev.chip.col_w()
            };
            let obel = vrf.get_bel(
                bel.cell
                    .with_cr(scol, endev.chip.row_clk())
                    .bel(bels::BUFIO2[i ^ 4]),
            );
            vrf.verify_node(&[bel.fwire(&format!("BUFIO2_LR{i}")), obel.fwire("CMT")]);
            let obel = vrf.get_bel(
                bel.cell
                    .with_cr(scol, endev.chip.row_clk())
                    .bel(bels::BUFIO2FB[i ^ 4]),
            );
            vrf.verify_node(&[bel.fwire(&format!("BUFIO2FB_LR{i}")), obel.fwire("CMT")]);
        } else {
            let scol = if i < 4 {
                endev.chip.col_e()
            } else {
                endev.chip.col_w()
            };
            let obel = vrf.get_bel(
                bel.cell
                    .with_cr(scol, endev.chip.row_clk())
                    .bel(bels::BUFIO2[i]),
            );
            vrf.verify_node(&[bel.fwire(&format!("BUFIO2_LR{i}")), obel.fwire("CMT")]);
            let obel = vrf.get_bel(
                bel.cell
                    .with_cr(scol, endev.chip.row_clk())
                    .bel(bels::BUFIO2FB[i]),
            );
            vrf.verify_node(&[bel.fwire(&format!("BUFIO2FB_LR{i}")), obel.fwire("CMT")]);
            let obel = vrf.get_bel(
                bel.cell
                    .with_row(endev.chip.row_tio_outer())
                    .bel(bels::BUFIO2[i]),
            );
            vrf.verify_node(&[bel.fwire(&format!("BUFIO2_BT{i}")), obel.fwire("CMT")]);
            let obel = vrf.get_bel(
                bel.cell
                    .with_row(endev.chip.row_tio_outer())
                    .bel(bels::BUFIO2FB[i]),
            );
            vrf.verify_node(&[bel.fwire(&format!("BUFIO2FB_BT{i}")), obel.fwire("CMT")]);
        }
    }
}

fn verify_pll(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLKIN1", SitePinDir::In),
        ("CLKIN2", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
        ("SKEWCLKIN1", SitePinDir::In),
        ("SKEWCLKIN2", SitePinDir::In),
        ("REL", SitePinDir::In),
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT2", SitePinDir::Out),
        ("CLKOUT3", SitePinDir::Out),
        ("CLKOUT4", SitePinDir::Out),
        ("CLKOUT5", SitePinDir::Out),
        ("CLKFBOUT", SitePinDir::Out),
        ("CLKOUTDCM0", SitePinDir::Out),
        ("CLKOUTDCM1", SitePinDir::Out),
        ("CLKOUTDCM2", SitePinDir::Out),
        ("CLKOUTDCM3", SitePinDir::Out),
        ("CLKOUTDCM4", SitePinDir::Out),
        ("CLKOUTDCM5", SitePinDir::Out),
        ("CLKFBDCM", SitePinDir::Out),
    ];
    vrf.verify_bel(
        bel,
        "PLL_ADV",
        &pins,
        &[
            "CLKIN1_CKINT0",
            "CLKIN2_CKINT0",
            "CLKIN2_CKINT1",
            "CLKFBIN_CKINT0",
            "CLKFBIN_CKINT1",
            "TEST_CLK",
        ],
    );
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    vrf.claim_node(&[bel.fwire_far("CLKFBOUT")]);
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKFBOUT"), bel.wire("CLKFBOUT"));

    let obel = vrf.find_bel_sibling(bel, bels::CMT);

    vrf.claim_node(&[bel.fwire_far("CLKIN1")]);
    vrf.claim_node(&[bel.fwire_far("CLKIN2")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire_far("CLKIN1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire_far("CLKIN2"));
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKIN1"), bel.wire("CLKIN1_CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKIN2"), bel.wire("CLKIN2_CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKIN2"), bel.wire("CLKIN2_CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKIN2"), bel.wire("CLK_FROM_DCM0"));
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKIN2"), bel.wire("CLK_FROM_DCM1"));
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("CLKIN1"),
            obel.wire(&format!("BUFIO2_BT{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("CLKIN1"),
            obel.wire(&format!("BUFIO2_LR{i}")),
        );
    }
    for i in 4..8 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("CLKIN2"),
            obel.wire(&format!("BUFIO2_BT{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("CLKIN2"),
            obel.wire(&format!("BUFIO2_LR{i}")),
        );
    }

    vrf.claim_node(&[bel.fwire_far("CLKFBIN")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire_far("CLKFBIN"));
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKFBIN"), bel.wire_far("CLKFBOUT"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("CLKFBIN"),
        bel.wire("CLKFBIN_CKINT0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("CLKFBIN"),
        bel.wire("CLKFBIN_CKINT1"),
    );
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKFBIN"), bel.wire("CLKOUT0"));
    vrf.claim_pip(bel.crd(), bel.wire_far("CLKFBIN"), bel.wire("CLKFBDCM"));
    for i in 0..8 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("CLKFBIN"),
            obel.wire(&format!("BUFIO2FB_BT{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("CLKFBIN"),
            obel.wire(&format!("BUFIO2FB_LR{i}")),
        );
    }

    vrf.claim_node(&[bel.fwire_far("CLKIN1_TEST")]);
    vrf.claim_node(&[bel.fwire_far("CLKFBIN_TEST")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1_TEST"), bel.wire_far("CLKIN1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN_TEST"), bel.wire_far("CLKFBIN"));

    vrf.claim_node(&[bel.fwire_far("CLKFBDCM_TEST")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBDCM_TEST"), bel.wire("CLKFBDCM"));

    let obel = vrf.find_bel_sibling(bel, bels::TIEOFF_PLL);
    vrf.claim_pip(bel.crd(), bel.wire("REL"), obel.wire("HARD1"));

    vrf.claim_node(&[bel.fwire("CLK_TO_DCM0")]);
    vrf.claim_node(&[bel.fwire("CLK_TO_DCM1")]);
    for pin in [
        "CLKOUTDCM0",
        "CLKOUTDCM1",
        "CLKOUTDCM2",
        "CLKOUTDCM3",
        "CLKOUTDCM4",
        "CLKOUTDCM5",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM0"), bel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM1"), bel.wire(pin));
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_TO_DCM1"),
        bel.wire("CLKFBDCM_TEST"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("SKEWCLKIN1"), bel.wire("CLK_TO_DCM1"));
    vrf.claim_pip(bel.crd(), bel.wire("SKEWCLKIN2"), bel.wire("CLK_TO_DCM0"));

    let obel_dcm0 = vrf.find_bel_delta(bel, 0, -16, bels::DCM0).unwrap();
    let obel_dcm1 = vrf.find_bel_delta(bel, 0, -16, bels::DCM1).unwrap();
    vrf.verify_node(&[bel.fwire("CLK_FROM_DCM0"), obel_dcm0.fwire("CLK_TO_PLL")]);
    vrf.verify_node(&[bel.fwire("CLK_FROM_DCM1"), obel_dcm1.fwire("CLK_TO_PLL")]);
    vrf.verify_node(&[bel.fwire("DCM0_CLKIN_TEST"), obel_dcm0.fwire("CLKIN_TEST")]);
    vrf.verify_node(&[bel.fwire("DCM1_CLKIN_TEST"), obel_dcm1.fwire("CLKIN_TEST")]);
    vrf.verify_node(&[bel.fwire("DCM0_CLKFB_TEST"), obel_dcm0.fwire("CLKFB_TEST")]);
    vrf.verify_node(&[bel.fwire("DCM1_CLKFB_TEST"), obel_dcm1.fwire("CLKFB_TEST")]);

    vrf.claim_pip(bel.crd(), bel.wire("TEST_CLK"), bel.wire("CLKIN1_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("TEST_CLK"), bel.wire("CLKFBIN_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("TEST_CLK"), bel.wire("DCM0_CLKIN_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("TEST_CLK"), bel.wire("DCM1_CLKIN_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("TEST_CLK"), bel.wire("DCM0_CLKFB_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("TEST_CLK"), bel.wire("DCM1_CLKFB_TEST"));
    vrf.claim_node(&[bel.fwire("TEST_CLK_OUT")]);
    vrf.claim_pip(bel.crd(), bel.wire("TEST_CLK_OUT"), bel.wire("TEST_CLK"));
}

fn verify_pll_bufpll(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, bels::PLL);
    for pin in ["CLKOUT0", "CLKOUT1", "LOCKED"] {
        let pin_d = format!("{pin}_D");
        let pin_u = format!("{pin}_U");
        vrf.claim_node(&[bel.fwire(pin)]);
        vrf.claim_node(&[bel.fwire(&pin_d)]);
        vrf.claim_node(&[bel.fwire(&pin_u)]);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire(&pin_d), bel.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire(&pin_u), bel.wire(pin));
    }
}

fn verify_dcm_bufpll_buf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    const PINS: [&str; 3] = ["LOCKED", "CLKOUT0", "CLKOUT1"];
    for src in ["PLL0", "PLL1", "CLKC"] {
        for pin in PINS {
            vrf.claim_node(&[bel.fwire(&format!("{src}_{pin}_O"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("{src}_{pin}_O")),
                bel.wire(&format!("{src}_{pin}_I")),
            );
        }
    }
    if matches!(
        bel.slot,
        bels::DCM_BUFPLL_BUF_S | bels::DCM_BUFPLL_BUF_S_MID
    ) {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 32, bels::DCM_BUFPLL_BUF_S_MID) {
            for pin in PINS {
                vrf.verify_node(&[
                    bel.fwire(&format!("PLL0_{pin}_I")),
                    obel.fwire(&format!("PLL0_{pin}_O")),
                ]);
                vrf.verify_node(&[
                    bel.fwire(&format!("CLKC_{pin}_I")),
                    obel.fwire(&format!("CLKC_{pin}_O")),
                ]);
            }
        } else {
            if bel.slot == bels::DCM_BUFPLL_BUF_S {
                // no PLL0 in this case
                for pin in PINS {
                    vrf.claim_node(&[bel.fwire(&format!("PLL0_{pin}_I"))]);
                }
            } else {
                let obel = vrf.find_bel_delta(bel, 0, 16, bels::PLL_BUFPLL).unwrap();
                for pin in PINS {
                    vrf.verify_node(&[
                        bel.fwire(&format!("PLL0_{pin}_I")),
                        obel.fwire(&format!("{pin}_D")),
                    ]);
                }
            }
            let obel = vrf.get_bel(
                bel.cell
                    .with_row(endev.chip.row_clk())
                    .bel(bels::CLKC_BUFPLL),
            );
            for pin in PINS {
                vrf.verify_node(&[
                    bel.fwire(&format!("CLKC_{pin}_I")),
                    obel.fwire(&format!("OUTD_{pin}")),
                ]);
            }
        }
        if bel.slot == bels::DCM_BUFPLL_BUF_S {
            let obel = vrf.find_bel_delta(bel, 0, 16, bels::PLL_BUFPLL).unwrap();
            for pin in PINS {
                vrf.verify_node(&[
                    bel.fwire(&format!("PLL1_{pin}_I")),
                    obel.fwire(&format!("{pin}_D")),
                ]);
            }
        } else if let Some(obel) = vrf.find_bel_delta(bel, 0, -32, bels::DCM_BUFPLL_BUF_S_MID) {
            for pin in PINS {
                vrf.verify_node(&[
                    bel.fwire(&format!("PLL1_{pin}_I")),
                    obel.fwire(&format!("PLL1_{pin}_O")),
                ]);
            }
        } else {
            let obel = vrf.find_bel_delta(bel, 0, -16, bels::PLL_BUFPLL).unwrap();
            for pin in PINS {
                vrf.verify_node(&[
                    bel.fwire(&format!("PLL1_{pin}_I")),
                    obel.fwire(&format!("{pin}_U")),
                ]);
            }
        }
    } else {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 32, bels::DCM_BUFPLL_BUF_N_MID) {
            for pin in PINS {
                vrf.verify_node(&[
                    bel.fwire(&format!("PLL1_{pin}_I")),
                    obel.fwire(&format!("PLL1_{pin}_O")),
                ]);
            }
            if bel.slot == bels::DCM_BUFPLL_BUF_N {
                let obel = vrf.find_bel_delta(bel, 0, 16, bels::PLL_BUFPLL).unwrap();
                for pin in PINS {
                    vrf.verify_node(&[
                        bel.fwire(&format!("PLL0_{pin}_I")),
                        obel.fwire(&format!("{pin}_D")),
                    ]);
                }
            }
        } else {
            let obel = vrf.find_bel_delta(bel, 0, 16, bels::PLL_BUFPLL).unwrap();
            for pin in PINS {
                vrf.verify_node(&[
                    bel.fwire(&format!("PLL1_{pin}_I")),
                    obel.fwire(&format!("{pin}_D")),
                ]);
            }
            if bel.slot == bels::DCM_BUFPLL_BUF_N {
                // no PLL0 in this case
                for pin in PINS {
                    vrf.claim_node(&[bel.fwire(&format!("PLL0_{pin}_I"))]);
                }
            }
        }
        if bel.slot == bels::DCM_BUFPLL_BUF_N {
            let obel = vrf.get_bel(
                bel.cell
                    .with_row(endev.chip.row_clk())
                    .bel(bels::CLKC_BUFPLL),
            );
            for pin in PINS {
                vrf.verify_node(&[
                    bel.fwire(&format!("CLKC_{pin}_I")),
                    obel.fwire(&format!("OUTU_{pin}")),
                ]);
            }
        } else {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -32, bels::DCM_BUFPLL_BUF_N_MID) {
                for pin in PINS {
                    vrf.verify_node(&[
                        bel.fwire(&format!("PLL0_{pin}_I")),
                        obel.fwire(&format!("PLL0_{pin}_O")),
                    ]);
                    vrf.verify_node(&[
                        bel.fwire(&format!("CLKC_{pin}_I")),
                        obel.fwire(&format!("CLKC_{pin}_O")),
                    ]);
                }
            } else {
                let obel = vrf.find_bel_delta(bel, 0, -16, bels::PLL_BUFPLL).unwrap();
                for pin in PINS {
                    vrf.verify_node(&[
                        bel.fwire(&format!("PLL0_{pin}_I")),
                        obel.fwire(&format!("{pin}_U")),
                    ]);
                }
                let obel = vrf
                    .find_bel_delta(bel, 0, -32, bels::DCM_BUFPLL_BUF_N)
                    .unwrap();
                for pin in PINS {
                    vrf.verify_node(&[
                        bel.fwire(&format!("CLKC_{pin}_I")),
                        obel.fwire(&format!("CLKC_{pin}_O")),
                    ]);
                }
            }
        }
    }
}

fn verify_clkc_bufpll(vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (opin, skip) in [
        ("OUTD_CLKOUT0", 'D'),
        ("OUTD_CLKOUT1", 'D'),
        ("OUTU_CLKOUT0", 'U'),
        ("OUTU_CLKOUT1", 'U'),
        ("OUTL_CLKOUT0", '_'),
        ("OUTL_CLKOUT1", '_'),
        ("OUTR_CLKOUT0", '_'),
        ("OUTR_CLKOUT1", '_'),
    ] {
        vrf.claim_node(&[bel.fwire(opin)]);
        if skip != 'D' {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL0D_CLKOUT0"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL0D_CLKOUT1"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL1D_CLKOUT0"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL1D_CLKOUT1"));
        }
        if skip != 'U' {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL0U_CLKOUT0"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL0U_CLKOUT1"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL1U_CLKOUT0"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL1U_CLKOUT1"));
        }
    }
    for (opin, skip) in [
        ("OUTD_LOCKED", 'D'),
        ("OUTU_LOCKED", 'U'),
        ("OUTL_LOCKED0", '_'),
        ("OUTL_LOCKED1", '_'),
        ("OUTR_LOCKED0", '_'),
        ("OUTR_LOCKED1", '_'),
    ] {
        vrf.claim_node(&[bel.fwire(opin)]);
        if skip != 'D' {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL0D_LOCKED"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL1D_LOCKED"));
        }
        if skip != 'U' {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL0U_LOCKED"));
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("PLL1U_LOCKED"));
        }
    }

    if let Some(obel) = vrf.find_bel_walk(bel, 0, -8, bels::DCM_BUFPLL_BUF_S_MID) {
        for (pin, opin) in [
            ("PLL1D_CLKOUT0", "PLL1_CLKOUT0_O"),
            ("PLL1D_CLKOUT1", "PLL1_CLKOUT1_O"),
            ("PLL1D_LOCKED", "PLL1_LOCKED_O"),
        ] {
            vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
        }
        let obel = vrf.find_bel_walk(bel, 0, -8, bels::PLL_BUFPLL).unwrap();
        for (pin, opin) in [
            ("PLL0D_CLKOUT0", "CLKOUT0_U"),
            ("PLL0D_CLKOUT1", "CLKOUT1_U"),
            ("PLL0D_LOCKED", "LOCKED_U"),
        ] {
            vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
        }
    } else {
        let obel = vrf
            .find_bel_walk(bel, 0, -8, bels::DCM_BUFPLL_BUF_S)
            .unwrap();
        for (pin, opin) in [
            ("PLL0D_CLKOUT0", "PLL0_CLKOUT0_I"),
            ("PLL0D_CLKOUT1", "PLL0_CLKOUT1_I"),
            ("PLL0D_LOCKED", "PLL0_LOCKED_I"),
        ] {
            vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
        }
        let obel = vrf.find_bel_walk(bel, 0, -8, bels::PLL_BUFPLL).unwrap();
        for (pin, opin) in [
            ("PLL1D_CLKOUT0", "CLKOUT0_U"),
            ("PLL1D_CLKOUT1", "CLKOUT1_U"),
            ("PLL1D_LOCKED", "LOCKED_U"),
        ] {
            vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
        }
    }

    let obel = vrf
        .find_bel_walk(bel, 0, 8, bels::DCM_BUFPLL_BUF_N)
        .unwrap();
    for (pin, opin) in [
        ("PLL0U_CLKOUT0", "PLL0_CLKOUT0_O"),
        ("PLL0U_CLKOUT1", "PLL0_CLKOUT1_O"),
        ("PLL0U_LOCKED", "PLL0_LOCKED_O"),
        ("PLL1U_CLKOUT0", "PLL1_CLKOUT0_O"),
        ("PLL1U_CLKOUT1", "PLL1_CLKOUT1_O"),
        ("PLL1U_LOCKED", "PLL1_LOCKED_O"),
    ] {
        vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
    }
}

fn verify_gtp(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    for (pin, slot) in [
        ("RXP0", bels::IPAD_RXP0),
        ("RXN0", bels::IPAD_RXN0),
        ("RXP1", bels::IPAD_RXP1),
        ("RXN1", bels::IPAD_RXN1),
    ] {
        pins.push((pin, SitePinDir::In));
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("O"));
    }
    for (pin, slot) in [
        ("TXP0", bels::OPAD_TXP0),
        ("TXN0", bels::OPAD_TXN0),
        ("TXP1", bels::OPAD_TXP1),
        ("TXN1", bels::OPAD_TXN1),
    ] {
        pins.push((pin, SitePinDir::Out));
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire(pin));
    }
    for (pin, slot) in [
        ("CLK00", bels::BUFDS0),
        ("CLK01", bels::BUFDS0),
        ("CLK10", bels::BUFDS1),
        ("CLK11", bels::BUFDS1),
    ] {
        pins.push((pin, SitePinDir::In));
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("O"));
    }
    for (pin, opin) in [
        ("PLLCLK00", "PLLCLK0"),
        ("PLLCLK01", "PLLCLK0"),
        ("PLLCLK10", "PLLCLK1"),
        ("PLLCLK11", "PLLCLK1"),
        ("CLKINEAST0", "CLKINEAST"),
        ("CLKINEAST1", "CLKINEAST"),
        ("CLKINWEST0", "CLKINWEST"),
        ("CLKINWEST1", "CLKINWEST"),
    ] {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire(opin));
    }
    for pin in ["RXCHBONDI0", "RXCHBONDI1", "RXCHBONDI2"] {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
    }
    for pin in [
        "RXCHBONDO0",
        "RXCHBONDO1",
        "RXCHBONDO2",
        "GTPCLKOUT00",
        "GTPCLKOUT01",
        "GTPCLKOUT10",
        "GTPCLKOUT11",
        "GTPCLKFBEAST0",
        "GTPCLKFBEAST1",
        "GTPCLKFBWEST0",
        "GTPCLKFBWEST1",
    ] {
        pins.push((pin, SitePinDir::Out));
        vrf.claim_pip(bel.crd(), bel.wire_far(pin), bel.wire(pin));
        vrf.claim_node(&[bel.fwire_far(pin)]);
    }
    for pin in [
        "RCALINEAST0",
        "RCALINEAST1",
        "RCALINEAST2",
        "RCALINEAST3",
        "RCALINEAST4",
        "RCALINWEST0",
        "RCALINWEST1",
        "RCALINWEST2",
        "RCALINWEST3",
        "RCALINWEST4",
    ] {
        pins.push((pin, SitePinDir::In));
    }
    for pin in [
        "RCALOUTEAST0",
        "RCALOUTEAST1",
        "RCALOUTEAST2",
        "RCALOUTEAST3",
        "RCALOUTEAST4",
        "RCALOUTWEST0",
        "RCALOUTWEST1",
        "RCALOUTWEST2",
        "RCALOUTWEST3",
        "RCALOUTWEST4",
    ] {
        pins.push((pin, SitePinDir::Out));
    }
    vrf.claim_node(&[bel.fwire("CLKOUT_EW")]);
    for pin in ["REFCLKPLL0", "REFCLKPLL1"] {
        pins.push((pin, SitePinDir::Out));
        vrf.claim_pip(bel.crd(), bel.wire("CLKOUT_EW"), bel.wire(pin));
    }
    vrf.verify_bel(bel, "GTPA1_DUAL", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel = vrf.find_bel_sibling(bel, bels::GTP_BUF);
    for (pin, opin) in [
        ("PLLCLK0", "PLLCLK0_O"),
        ("PLLCLK1", "PLLCLK1_O"),
        ("CLKINEAST", "CLKINEAST_O"),
        ("CLKINWEST", "CLKINWEST_O"),
    ] {
        vrf.verify_node(&[bel.fwire(pin), obel.fwire(opin)]);
    }
    for (pin, opin) in [
        ("RXCHBONDI0", "RXCHBONDI0_O"),
        ("RXCHBONDI1", "RXCHBONDI1_O"),
        ("RXCHBONDI2", "RXCHBONDI2_O"),
    ] {
        vrf.verify_node(&[bel.fwire_far(pin), obel.fwire(opin)]);
    }

    if bel.col < endev.chip.col_clk {
        for i in 0..5 {
            vrf.claim_node(&[bel.fwire(&format!("RCALOUTEAST{i}_BUF"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCALOUTEAST{i}_BUF")),
                bel.wire(&format!("RCALOUTEAST{i}")),
            );
        }
    } else {
        for i in 0..5 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCALINEAST{i}")),
                bel.wire(&format!("RCALINEAST{i}_BUF")),
            );
            vrf.verify_node(&[
                bel.fwire(&format!("RCALINEAST{i}_BUF")),
                obel.fwire(&format!("RCALINEAST{i}_O")),
            ]);
        }
    }
}

fn verify_gtp_buf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for (opin, ipin) in [
        ("PLLCLK0_O", "PLLCLK0_I"),
        ("PLLCLK1_O", "PLLCLK1_I"),
        ("CLKINEAST_O", "CLKINEAST_I"),
        ("CLKINWEST_O", "CLKINWEST_I"),
        ("CLKOUT_EW_O", "CLKOUT_EW_I"),
    ] {
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
    }
    let obel = vrf.find_bel_sibling(bel, bels::GTP);
    vrf.verify_node(&[bel.fwire("CLKOUT_EW_I"), obel.fwire("CLKOUT_EW")]);
    let srow = if bel.row < endev.chip.row_clk() {
        endev.chip.row_bio_outer()
    } else {
        endev.chip.row_tio_outer()
    };
    let obel_bufpll = vrf.get_bel(
        bel.cell
            .with_cr(endev.chip.col_clk, srow)
            .bel(bels::BUFPLL_BUF),
    );
    vrf.verify_node(&[bel.fwire("PLLCLK0_I"), obel_bufpll.fwire("PLLCLK0_O")]);
    vrf.verify_node(&[bel.fwire("PLLCLK1_I"), obel_bufpll.fwire("PLLCLK1_O")]);

    for i in 0..4 {
        vrf.claim_node(&[bel.fwire(&format!("GTPCLK{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GTPCLK{i}_O")),
            bel.wire(&format!("GTPCLK{i}_I")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("GTPCLK{i}_I")),
            obel.fwire_far(["GTPCLKOUT00", "GTPCLKOUT01", "GTPCLKOUT10", "GTPCLKOUT11"][i]),
        ]);
        vrf.claim_node(&[bel.fwire(&format!("GTPFB{i}_O"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GTPFB{i}_O")),
            bel.wire(&format!("GTPFB{i}_I")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("GTPFB{i}_I")),
            obel.fwire_far(
                [
                    "GTPCLKFBWEST0",
                    "GTPCLKFBWEST1",
                    "GTPCLKFBEAST0",
                    "GTPCLKFBEAST1",
                ][i],
            ),
        ]);
    }

    let obel_h = vrf.get_bel(bel.cell.with_col(endev.chip.col_clk).bel(bels::GTP_H_BUF));
    let lr = if bel.col < endev.chip.col_clk {
        'L'
    } else {
        'R'
    };
    for i in 0..3 {
        vrf.verify_node(&[
            bel.fwire(&format!("RXCHBONDO{i}_I")),
            obel.fwire_far(&format!("RXCHBONDO{i}")),
        ]);
        vrf.claim_node(&[bel.fwire(&format!("RXCHBONDI{i}_O"))]);
    }
    if !matches!(endev.chip.gts, Gts::Single(_)) {
        for i in 0..3 {
            vrf.claim_node(&[bel.fwire(&format!("RXCHBONDO{i}_O"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RXCHBONDO{i}_O")),
                bel.wire(&format!("RXCHBONDO{i}_I")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RXCHBONDI{i}_O")),
                bel.wire(&format!("RXCHBONDI{i}_I")),
            );
            vrf.verify_node(&[
                bel.fwire(&format!("RXCHBONDI{i}_I")),
                obel_h.fwire_far(&format!("RXCHBONDI{i}_{lr}")),
            ]);
        }
        if bel.col < endev.chip.col_clk {
            for i in 0..5 {
                vrf.claim_node(&[bel.fwire(&format!("RCALOUTEAST{i}_O"))]);
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("RCALOUTEAST{i}_O")),
                    bel.wire(&format!("RCALOUTEAST{i}_I")),
                );
                vrf.verify_node(&[
                    bel.fwire(&format!("RCALOUTEAST{i}_I")),
                    obel.fwire(&format!("RCALOUTEAST{i}_BUF")),
                ]);
            }
        } else {
            for i in 0..5 {
                vrf.claim_node(&[bel.fwire(&format!("RCALINEAST{i}_O"))]);
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("RCALINEAST{i}_O")),
                    bel.wire(&format!("RCALINEAST{i}_I")),
                );
                vrf.verify_node(&[
                    bel.fwire(&format!("RCALINEAST{i}_I")),
                    obel_h.fwire(&format!("RCALINEAST{i}_{lr}")),
                ]);
            }
        }
    }
    vrf.verify_node(&[
        bel.fwire("CLKINEAST_I"),
        obel_h.fwire_far(&format!("CLKINEAST_{lr}")),
    ]);
    vrf.verify_node(&[
        bel.fwire("CLKINWEST_I"),
        obel_h.fwire_far(&format!("CLKINWEST_{lr}")),
    ]);
}

fn verify_gtp_h_buf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut l = None;
    let mut r = None;
    let mut have_both = None;
    if !endev.edev.disabled.contains(&DisabledPart::Gtp) {
        if bel.row == endev.chip.row_bio_outer() {
            if let Gts::Quad(cl, cr) = endev.chip.gts {
                have_both = Some((cl, cr));
                l = Some(cl);
                r = Some(cr);
            }
        } else {
            match endev.chip.gts {
                Gts::Double(cl, cr) | Gts::Quad(cl, cr) => {
                    have_both = Some((cl, cr));
                    l = Some(cl);
                    r = Some(cr);
                }
                Gts::Single(cl) => {
                    l = Some(cl);
                }
                _ => (),
            }
        }
    }
    if let Some(cl) = l {
        let obel_l = vrf.get_bel(bel.cell.with_col(cl).bel(bels::GTP_BUF));
        vrf.verify_node(&[bel.fwire("CLKOUT_EW_L"), obel_l.fwire("CLKOUT_EW_O")]);
    } else {
        vrf.claim_node(&[bel.fwire("CLKOUT_EW_L")]);
    }
    if let Some(cr) = r {
        let obel_r = vrf.get_bel(bel.cell.with_col(cr).bel(bels::GTP_BUF));
        vrf.verify_node(&[bel.fwire("CLKOUT_EW_R"), obel_r.fwire("CLKOUT_EW_O")]);
    } else {
        vrf.claim_node(&[bel.fwire("CLKOUT_EW_R")]);
    }
    vrf.claim_node(&[bel.fwire("CLKINEAST_L")]);
    vrf.claim_node(&[bel.fwire("CLKINWEST_L")]);
    vrf.claim_node(&[bel.fwire("CLKINEAST_R")]);
    vrf.claim_node(&[bel.fwire("CLKINWEST_R")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLKINWEST_L"), bel.wire("CLKOUT_EW_R"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKINEAST_R"), bel.wire("CLKOUT_EW_L"));
    if let Some((cl, cr)) = have_both {
        let obel_l = vrf.get_bel(bel.cell.with_col(cl).bel(bels::GTP_BUF));
        let obel_r = vrf.get_bel(bel.cell.with_col(cr).bel(bels::GTP_BUF));
        for i in 0..3 {
            vrf.claim_node(&[bel.fwire(&format!("RXCHBONDI{i}_L"))]);
            vrf.claim_node(&[bel.fwire(&format!("RXCHBONDI{i}_R"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RXCHBONDI{i}_L")),
                bel.wire(&format!("RXCHBONDO{i}_R")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RXCHBONDI{i}_R")),
                bel.wire(&format!("RXCHBONDO{i}_L")),
            );
            vrf.verify_node(&[
                bel.fwire(&format!("RXCHBONDO{i}_L")),
                obel_l.fwire(&format!("RXCHBONDO{i}_O")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("RXCHBONDO{i}_R")),
                obel_r.fwire(&format!("RXCHBONDO{i}_O")),
            ]);
        }
        for i in 0..5 {
            vrf.claim_node(&[bel.fwire(&format!("RCALINEAST{i}_R"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("RCALINEAST{i}_R")),
                bel.wire(&format!("RCALOUTEAST{i}_L")),
            );
            vrf.verify_node(&[
                bel.fwire(&format!("RCALOUTEAST{i}_L")),
                obel_l.fwire(&format!("RCALOUTEAST{i}_O")),
            ]);
        }
    }
}

fn verify_bufds(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("I", SitePinDir::In),
        ("IB", SitePinDir::In),
        ("O", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "BUFDS", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let idx = match bel.slot {
        bels::BUFDS0 => 0,
        bels::BUFDS1 => 1,
        _ => unreachable!(),
    };
    for (pin, slot) in [
        ("I", [bels::IPAD_CLKP0, bels::IPAD_CLKP1][idx]),
        ("IB", [bels::IPAD_CLKN0, bels::IPAD_CLKN1][idx]),
    ] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("O"));
    }
}

fn verify_ipad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
}

fn verify_opad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "OPAD", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let slot_name = endev.edev.egrid.db.bel_slots.key(bel.slot);
    match bel.slot {
        bels::SLICE0 => verify_sliceml(vrf, bel),
        bels::SLICE1 => vrf.verify_bel(bel, "SLICEX", &[], &[]),
        bels::BRAM_F => vrf.verify_bel(bel, "RAMB16BWER", &[], &[]),
        bels::BRAM_H0 | bels::BRAM_H1 => vrf.verify_bel(bel, "RAMB8BWER", &[], &[]),
        bels::DSP => verify_dsp(vrf, bel),
        bels::PCIE => vrf.verify_bel(bel, "PCIE_A1", &[], &[]),

        _ if slot_name.starts_with("OCT_CAL") => vrf.verify_bel(bel, "OCT_CALIBRATE", &[], &[]),
        _ if slot_name.starts_with("BSCAN") => vrf.verify_bel(bel, "BSCAN", &[], &[]),
        bels::PMV
        | bels::DNA_PORT
        | bels::ICAP
        | bels::SPI_ACCESS
        | bels::SUSPEND_SYNC
        | bels::POST_CRC_INTERNAL
        | bels::STARTUP
        | bels::SLAVE_SPI => vrf.verify_bel(bel, slot_name, &[], &[]),

        bels::ILOGIC0 | bels::ILOGIC1 => verify_ilogic(vrf, bel),
        bels::OLOGIC0 | bels::OLOGIC1 => verify_ologic(vrf, bel),
        bels::IODELAY0 | bels::IODELAY1 => verify_iodelay(vrf, bel),
        bels::IOICLK0 | bels::IOICLK1 => verify_ioiclk(vrf, bel),
        bels::IOI => verify_ioi(endev, vrf, bel),
        bels::IOB0 | bels::IOB1 => verify_iob(vrf, bel),
        _ if slot_name.starts_with("TIEOFF") => verify_tieoff(vrf, bel),
        bels::CLKPIN_BUF => verify_clkpin_buf(vrf, bel),

        bels::PCILOGICSE => verify_pcilogicse(endev, vrf, bel),
        bels::MCB => verify_mcb(endev, vrf, bel),
        _ if slot_name.starts_with("MCB_TIE") => verify_mcb_tie(endev, vrf, bel),

        bels::BTIOI_CLK => verify_btioi_clk(endev, vrf, bel),
        bels::LRIOI_CLK => verify_lrioi_clk(endev, vrf, bel),
        bels::LRIOI_CLK_TERM => verify_lrioi_clk_term(endev, vrf, bel),
        bels::PCI_CE_TRUNK_BUF => verify_pci_ce_trunk_buf(endev, vrf, bel),
        bels::PCI_CE_SPLIT => verify_pci_ce_split(endev, vrf, bel),
        bels::PCI_CE_V_BUF => verify_pci_ce_v_buf(endev, vrf, bel),
        bels::PCI_CE_H_BUF => verify_pci_ce_h_buf(endev, vrf, bel),

        _ if slot_name.starts_with("BUFH") => verify_bufh(endev, vrf, bel),
        bels::HCLK_V_MIDBUF => verify_hclk_v_midbuf(endev, vrf, bel),
        bels::HCLK_ROW => verify_hclk_row(endev, vrf, bel),
        bels::HCLK_H_MIDBUF => verify_hclk_h_midbuf(endev, vrf, bel),
        bels::HCLK => verify_hclk(endev, vrf, bel),
        _ if slot_name.starts_with("BUFGMUX") => verify_bufgmux(vrf, bel),
        bels::CLKC => verify_clkc(endev, vrf, bel),

        bels::CKPIN_V_MIDBUF => verify_ckpin_v_midbuf(endev, vrf, bel),
        bels::CKPIN_H_MIDBUF => verify_ckpin_h_midbuf(endev, vrf, bel),

        bels::BUFIO2_INS => verify_bufio2_ins(endev, vrf, bel),
        bels::BUFIO2_CKPIN => verify_bufio2_ckpin(vrf, bel),
        _ if slot_name.starts_with("BUFIO2_") => verify_bufio2(endev, vrf, bel),
        _ if slot_name.starts_with("BUFIO2FB_") => verify_bufio2fb(vrf, bel),

        bels::BUFPLL0 | bels::BUFPLL1 => verify_bufpll(endev, vrf, bel),
        bels::BUFPLL_MCB => verify_bufpll_mcb(endev, vrf, bel),
        bels::BUFPLL_OUT => verify_bufpll_out(vrf, bel),
        bels::BUFPLL_BUF => verify_bufpll_buf(vrf, bel),
        bels::BUFPLL_INS_LR => verify_bufpll_ins_lr(endev, vrf, bel),
        bels::BUFPLL_INS_BT => verify_bufpll_ins_bt(endev, vrf, bel),

        bels::DCM0 | bels::DCM1 => verify_dcm(vrf, bel),
        bels::CMT => verify_cmt(endev, vrf, bel),
        bels::PLL => verify_pll(vrf, bel),
        bels::PLL_BUFPLL => verify_pll_bufpll(vrf, bel),
        _ if slot_name.starts_with("DCM_BUFPLL_BUF") => verify_dcm_bufpll_buf(endev, vrf, bel),
        bels::CLKC_BUFPLL => verify_clkc_bufpll(vrf, bel),

        bels::GTP => verify_gtp(endev, vrf, bel),
        _ if slot_name.starts_with("BUFDS") => verify_bufds(vrf, bel),
        _ if slot_name.starts_with("IPAD") => verify_ipad(vrf, bel),
        _ if slot_name.starts_with("OPAD") => verify_opad(vrf, bel),
        bels::GTP_BUF => verify_gtp_buf(endev, vrf, bel),
        bels::GTP_H_BUF => verify_gtp_h_buf(endev, vrf, bel),

        _ => println!("MEOW {} {:?}", bel.slot, bel.name),
    }
}

fn verify_extra(_endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    vrf.kill_stub_out("INT_IOI_LOGICIN_B4");
    vrf.kill_stub_out("INT_IOI_LOGICIN_B10");
    vrf.kill_stub_out("FAN");
    let mut dummy_terms = HashSet::new();
    for &crd in vrf.rd.tiles_by_kind_name("INT_LTERM") {
        let tile = &vrf.rd.tiles[&crd];
        let otile = &vrf.rd.tiles[&crd.delta(2, 0)];
        if tile.kind == otile.kind {
            let tk = &vrf.rd.tile_kinds[tile.kind];
            for &(wi, wo) in tk.pips.keys() {
                vrf.claim_node(&[(crd, &vrf.rd.wires[wo])]);
                vrf.claim_node(&[(crd, &vrf.rd.wires[wi])]);
                vrf.claim_pip(crd, &vrf.rd.wires[wo], &vrf.rd.wires[wi]);
            }
            dummy_terms.insert(crd);
        }
    }
    let wrong_term_pips: Vec<_> = [
        ("LTERM_NW4M0", "LTERM_NE4C0"),
        ("LTERM_NW4M1", "LTERM_NE4C1"),
        ("LTERM_NW4M2", "LTERM_NE4C2"),
        ("LTERM_NW4M3", "LTERM_NE4C3"),
        ("INT_INTERFACE_LTERM_NW4M0", "INT_INTERFACE_LTERM_NE4C0"),
        ("INT_INTERFACE_LTERM_NW4M1", "INT_INTERFACE_LTERM_NE4C1"),
        ("INT_INTERFACE_LTERM_NW4M2", "INT_INTERFACE_LTERM_NE4C2"),
        ("INT_INTERFACE_LTERM_NW4M3", "INT_INTERFACE_LTERM_NE4C3"),
        ("IOI_BTERM_SL1B0", "IOI_BTERM_NL1E0"),
        ("IOI_BTERM_SL1B1", "IOI_BTERM_NL1E1"),
        ("IOI_BTERM_SL1B2", "IOI_BTERM_NL1E2"),
        ("IOI_BTERM_SL1B3", "IOI_BTERM_NL1E3"),
    ]
    .into_iter()
    .filter_map(|(a, b)| {
        if let (Some(a), Some(b)) = (vrf.rd.wires.get(a), vrf.rd.wires.get(b)) {
            Some((b, a))
        } else {
            None
        }
    })
    .collect();
    for (&crd, tile) in &vrf.rd.tiles {
        if dummy_terms.contains(&crd) {
            continue;
        }
        let tk = &vrf.rd.tile_kinds[tile.kind];
        for &key in &wrong_term_pips {
            if tk.pips.contains_key(&key) {
                vrf.claim_pip(crd, &vrf.rd.wires[key.1], &vrf.rd.wires[key.0]);
            }
        }
    }
    for (tkn, base) in [("GTPDUAL_BOT", 9), ("GTPDUAL_TOP", 1)] {
        for &crd in vrf.rd.tiles_by_kind_name(tkn) {
            for i in 0..8 {
                let idx = base + i;
                for j in 0..63 {
                    vrf.claim_node(&[(crd, &format!("GTPDUAL_LEFT_LOGICIN_B{j}_{idx}"))]);
                    vrf.claim_node(&[(crd, &format!("GTPDUAL_RIGHT_LOGICIN_B{j}_{idx}"))]);
                }
                for j in 0..2 {
                    vrf.claim_node(&[(crd, &format!("GTPDUAL_LEFT_CLK{j}_{idx}"))]);
                    vrf.claim_node(&[(crd, &format!("GTPDUAL_RIGHT_CLK{j}_{idx}"))]);
                    vrf.claim_node(&[(crd, &format!("GTPDUAL_LEFT_SR{j}_{idx}"))]);
                    vrf.claim_node(&[(crd, &format!("GTPDUAL_RIGHT_SR{j}_{idx}"))]);
                }
                for j in 0..24 {
                    vrf.claim_node(&[(crd, &format!("GTPDUAL_LEFT_LOGICOUT{j}_{idx}"))]);
                    vrf.claim_node(&[(crd, &format!("GTPDUAL_RIGHT_LOGICOUT{j}_{idx}"))]);
                }
            }
        }
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    verify(
        rd,
        &endev.ngrid,
        |_| (),
        |vrf, bel| verify_bel(endev, vrf, bel),
        |vrf| verify_extra(endev, vrf),
    );
}
