use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord},
    dir::{DirH, DirHV, DirV},
    grid::BelCoord,
};
use prjcombine_re_xilinx_naming_spartan6::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{LegacyBelContext, RawWireCoord, SitePinDir, Verifier};
use prjcombine_spartan6::{
    chip::{ColumnKind, DisabledPart},
    defs::{bcls, bslots, tcls, wires},
};
use std::collections::HashSet;

fn verify_sliceml(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let kind = if endev.chip.columns[bcrd.col].kind == ColumnKind::CleXM {
        "SLICEM"
    } else {
        "SLICEL"
    };
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_in("CIN")
        .extra_out("COUT");
    if let Some(obel) = endev.edev.bel_carry_prev(bcrd) {
        bel.claim_net(&[bel.wire("CIN"), bel.bel_wire_far(obel, "COUT")]);
        bel.claim_pip(bel.bel_wire_far(obel, "COUT"), bel.bel_wire(obel, "COUT"));
    } else {
        bel.claim_net(&[bel.wire("CIN")]);
    }
    bel.claim_net(&[bel.wire("COUT")]);
    bel.commit();
}

fn verify_dsp(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let carry: Vec<_> = (0..18)
        .map(|x| (format!("BCOUT{x}"), format!("BCIN{x}")))
        .chain((0..48).map(|x| (format!("PCOUT{x}"), format!("PCIN{x}"))))
        .chain([("CARRYOUT".to_string(), "CARRYIN".to_string())])
        .collect();
    let mut bel = vrf.verify_bel(bcrd).kind("DSP48A1");
    for (o, i) in &carry {
        bel = bel.extra_in(i).extra_out(o);
        bel.claim_net(&[bel.wire(o)]);
        bel.claim_net(&[bel.wire(i)]);
    }
    if let Some(obel) = endev.edev.bel_carry_prev(bcrd) {
        for (o, i) in &carry {
            bel.verify_net(&[bel.wire(i), bel.bel_wire_far(obel, o)]);
            bel.claim_pip(bel.bel_wire_far(obel, o), bel.bel_wire(obel, o));
        }
    }
    bel.commit();
}

fn verify_ilogic(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
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
        ("SR", SitePinDir::In),
    ];
    if bel.slot == bslots::ILOGIC[0] {
        pins.extend([("INCDEC", SitePinDir::Out), ("VALID", SitePinDir::Out)]);
    }
    vrf.verify_legacy_bel(bel, "ILOGIC2", &pins, &["SR_INT"]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let idx = bslots::ILOGIC.index_of(bel.slot).unwrap();
    let oslot = bslots::OLOGIC[idx];
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.wire("SR"), bel.wire("SR_INT"));
    vrf.claim_pip(bel.wire("SR"), obel.wire_far("SR"));
    vrf.claim_pip(bel.wire("OFB"), obel.wire("OQ"));
    vrf.claim_pip(bel.wire("TFB"), obel.wire("TQ"));

    let oslot = bslots::IODELAY[idx];
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.wire("DDLY"), obel.wire("DATAOUT"));
    vrf.claim_pip(bel.wire("DDLY2"), obel.wire("DATAOUT2"));

    let oslot = bslots::IOICLK[idx];
    let obel = vrf.find_bel_sibling(bel, oslot);
    let obel_tie = vrf.find_bel_sibling(bel, bslots::TIEOFF_IOI);
    vrf.claim_pip(bel.wire("CLK0"), obel.wire("CLK0_ILOGIC"));
    vrf.claim_pip(bel.wire("CLK1"), obel.wire("CLK1"));
    vrf.claim_pip(bel.wire("IOCE"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.wire("IOCE"), obel_tie.wire("HARD1"));

    vrf.claim_net(&[bel.wire("D_MUX")]);
    vrf.claim_pip(bel.wire("D"), bel.wire("D_MUX"));
    vrf.claim_pip(bel.wire("D_MUX"), bel.wire("IOB_I"));

    let obel = bel.cell.bel(bslots::IOB[idx]);
    if vrf.grid.has_bel(obel) {
        vrf.verify_net(&[bel.wire("IOB_I"), vrf.bel_wire_far(obel, "I")]);

        vrf.claim_pip(bel.wire("MCB_FABRICOUT"), bel.wire("FABRICOUT"));
    } else {
        vrf.claim_net(&[bel.wire("IOB_I")]);
    }
    vrf.claim_net(&[bel.wire("MCB_FABRICOUT")]);

    let oslot = bslots::ILOGIC[idx ^ 1];
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.wire("SHIFTIN"), obel.wire("SHIFTOUT"));
    if bel.slot == bslots::ILOGIC[0] {
        vrf.claim_pip(bel.wire("D_MUX"), obel.wire("IOB_I"));
    }
}

fn verify_ologic(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
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
    vrf.verify_legacy_bel(bel, "OLOGIC2", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let idx = bslots::OLOGIC.index_of(bel.slot).unwrap();
    let oslot = bslots::IOICLK[idx];
    let obel = vrf.find_bel_sibling(bel, oslot);
    let obel_tie = vrf.find_bel_sibling(bel, bslots::TIEOFF_IOI);
    vrf.claim_pip(bel.wire("CLK0"), obel.wire("CLK0_OLOGIC"));
    vrf.claim_pip(bel.wire("CLK1"), obel.wire("CLK1"));
    vrf.claim_pip(bel.wire("IOCE"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.wire("IOCE"), obel_tie.wire("HARD1"));

    let obel_ioi = vrf.find_bel_sibling(bel, bslots::IOI);
    vrf.claim_pip(bel.wire("OCE"), obel_ioi.wire("PCI_CE"));
    vrf.claim_pip(bel.wire("REV"), obel_tie.wire("HARD0"));
    vrf.claim_pip(bel.wire("SR"), obel_tie.wire("HARD0"));
    vrf.claim_pip(bel.wire("TRAIN"), obel_tie.wire("HARD0"));

    let oslot = bslots::IODELAY[idx];
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.wire("IOB_O"), bel.wire("OQ"));
    vrf.claim_pip(bel.wire("IOB_O"), obel.wire("DOUT"));
    vrf.claim_pip(bel.wire("IOB_T"), bel.wire("TQ"));
    vrf.claim_pip(bel.wire("IOB_T"), obel.wire("TOUT"));

    let obel = bel.cell.bel(bslots::IOB[idx]);
    if vrf.grid.has_bel(obel) {
        vrf.verify_net(&[bel.wire("IOB_O"), vrf.bel_wire_far(obel, "O")]);
        vrf.verify_net(&[bel.wire("IOB_T"), vrf.bel_wire_far(obel, "T")]);

        vrf.claim_pip(bel.wire("D1"), bel.wire("MCB_D1"));
        vrf.claim_pip(bel.wire("D2"), bel.wire("MCB_D2"));
        vrf.claim_pip(bel.wire("T1"), obel_ioi.wire("MCB_T1"));
        vrf.claim_pip(bel.wire("T2"), obel_ioi.wire("MCB_T2"));
        vrf.claim_pip(bel.wire("TRAIN"), obel_ioi.wire("MCB_DRPTRAIN"));
    } else {
        vrf.claim_net(&[bel.wire("IOB_T")]);
        vrf.claim_net(&[bel.wire("IOB_O")]);
    }

    let oslot = bslots::OLOGIC[idx ^ 1];
    let obel = vrf.find_bel_sibling(bel, oslot);
    if bel.slot == bslots::OLOGIC[0] {
        vrf.claim_pip(bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    } else {
        vrf.claim_pip(bel.wire("SHIFTIN3"), obel.wire("SHIFTOUT3"));
        vrf.claim_pip(bel.wire("SHIFTIN4"), obel.wire("SHIFTOUT4"));
    }
}

fn verify_iodelay(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let idx = bslots::IODELAY.index_of(bel.slot).unwrap();
    let mut pins = vec![
        ("IOCLK0", SitePinDir::In),
        ("IOCLK1", SitePinDir::In),
        ("IDATAIN", SitePinDir::In),
        ("ODATAIN", SitePinDir::In),
        ("T", SitePinDir::In),
        ("DOUT", SitePinDir::Out),
        ("TOUT", SitePinDir::Out),
        ("DATAOUT", SitePinDir::Out),
        ("DATAOUT2", SitePinDir::Out),
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
    if idx == 0 {
        pins.extend([("DQSOUTP", SitePinDir::Out), ("DQSOUTN", SitePinDir::Out)]);
    }
    vrf.verify_legacy_bel(bel, "IODELAY2", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let oslot = bslots::IOICLK[idx];
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.wire("IOCLK0"), obel.wire("CLK0_ILOGIC"));
    vrf.claim_pip(bel.wire("IOCLK0"), obel.wire("CLK0_OLOGIC"));
    vrf.claim_pip(bel.wire("IOCLK1"), obel.wire("CLK1"));

    let oslot = bslots::ILOGIC[idx];
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.wire("IDATAIN"), obel.wire("D_MUX"));

    let oslot = bslots::OLOGIC[idx];
    let obel = vrf.find_bel_sibling(bel, oslot);
    vrf.claim_pip(bel.wire("ODATAIN"), obel.wire("OQ"));
    vrf.claim_pip(bel.wire("T"), obel.wire("TQ"));

    let obel_ioi = vrf.find_bel_sibling(bel, bslots::IOI);
    vrf.claim_net(&[bel.wire("MCB_DQSOUTP")]);
    let obel = bel.cell.bel(bslots::IOB[idx]);
    if vrf.grid.has_bel(obel) {
        vrf.claim_pip(bel.wire("MCB_DQSOUTP"), bel.wire("DQSOUTP"));
        vrf.claim_pip(bel.wire("CAL"), obel_ioi.wire("MCB_DRPADD"));
        vrf.claim_pip(bel.wire("CE"), obel_ioi.wire("MCB_DRPSDO"));
        vrf.claim_pip(bel.wire("CLK"), obel_ioi.wire("MCB_DRPCLK"));
        vrf.claim_pip(bel.wire("INC"), obel_ioi.wire("MCB_DRPCS"));
        vrf.claim_pip(bel.wire("RST"), obel_ioi.wire("MCB_DRPBROADCAST"));
    }
}

fn verify_ioiclk(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, bslots::IOI);
    vrf.claim_net(&[bel.wire("CLK0INTER")]);
    vrf.claim_pip(bel.wire("CLK0INTER"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.wire("CLK0INTER"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.wire("CLK0INTER"), obel.wire("IOCLK0"));
    vrf.claim_pip(bel.wire("CLK0INTER"), obel.wire("IOCLK2"));
    vrf.claim_pip(bel.wire("CLK0INTER"), obel.wire("PLLCLK0"));
    vrf.claim_net(&[bel.wire("CLK1INTER")]);
    vrf.claim_pip(bel.wire("CLK1INTER"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.wire("CLK1INTER"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.wire("CLK1INTER"), obel.wire("IOCLK1"));
    vrf.claim_pip(bel.wire("CLK1INTER"), obel.wire("IOCLK3"));
    vrf.claim_pip(bel.wire("CLK1INTER"), obel.wire("PLLCLK1"));
    vrf.claim_net(&[bel.wire("CLK2INTER")]);
    vrf.claim_pip(bel.wire("CLK2INTER"), obel.wire("PLLCLK0"));
    vrf.claim_pip(bel.wire("CLK2INTER"), obel.wire("PLLCLK1"));
    vrf.claim_net(&[bel.wire("CLK0_ILOGIC")]);
    vrf.claim_pip(bel.wire("CLK0_ILOGIC"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.wire("CLK0_ILOGIC"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.wire("CLK0_ILOGIC"), bel.wire("CLK2INTER"));
    vrf.claim_net(&[bel.wire("CLK0_OLOGIC")]);
    vrf.claim_pip(bel.wire("CLK0_OLOGIC"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.wire("CLK0_OLOGIC"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.wire("CLK0_OLOGIC"), bel.wire("CLK2INTER"));
    vrf.claim_net(&[bel.wire("CLK1")]);
    vrf.claim_pip(bel.wire("CLK1"), bel.wire("CLK0INTER"));
    vrf.claim_pip(bel.wire("CLK1"), bel.wire("CLK1INTER"));
    vrf.claim_pip(bel.wire("CLK1"), bel.wire("CLK2INTER"));
    vrf.claim_net(&[bel.wire("IOCE0")]);
    vrf.claim_pip(bel.wire("IOCE0"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.wire("IOCE0"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.wire("IOCE0"), obel.wire("IOCE2"));
    vrf.claim_pip(bel.wire("IOCE0"), obel.wire("IOCE3"));
    vrf.claim_pip(bel.wire("IOCE0"), obel.wire("PLLCE0"));
    vrf.claim_pip(bel.wire("IOCE0"), obel.wire("PLLCE1"));
    vrf.claim_net(&[bel.wire("IOCE1")]);
    vrf.claim_pip(bel.wire("IOCE1"), obel.wire("IOCE0"));
    vrf.claim_pip(bel.wire("IOCE1"), obel.wire("IOCE1"));
    vrf.claim_pip(bel.wire("IOCE1"), obel.wire("IOCE2"));
    vrf.claim_pip(bel.wire("IOCE1"), obel.wire("IOCE3"));
    vrf.claim_pip(bel.wire("IOCE1"), obel.wire("PLLCE0"));
    vrf.claim_pip(bel.wire("IOCE1"), obel.wire("PLLCE1"));
}

fn verify_ioi(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    let obel = endev.chip.bel_pcilogicse(if bel.col <= endev.chip.col_clk {
        DirH::W
    } else {
        DirH::E
    });
    vrf.verify_net(&[bel.wire("PCI_CE"), vrf.bel_pip_owire(obel, "PCI_CE", 0)]);

    vrf.claim_net(&[bel.wire("MCB_DRPSDI")]);
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
            vrf.claim_net(&[bel.wire(pin)]);
        }
        for slot in bslots::OLOGIC {
            let obel = vrf.find_bel_sibling(bel, slot);
            vrf.claim_net(&[obel.wire("MCB_D1")]);
            vrf.claim_net(&[obel.wire("MCB_D2")]);
        }
        // in other cases, nodes will be verified by MCB code
    }
}

fn verify_iob(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IOB.index_of(bcrd.slot).unwrap();
    let kind = ["IOBS", "IOBM"][idx];
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_in("O")
        .extra_in("T")
        .extra_out("PCI_RDY")
        .extra_out("PADOUT")
        .extra_in("DIFFI_IN")
        .extra_out("DIFFO_OUT")
        .extra_in("DIFFO_IN");
    for pin in [
        "O",
        "T",
        "PCI_RDY",
        "PADOUT",
        "DIFFI_IN",
        "DIFFO_OUT",
        "DIFFO_IN",
    ] {
        bel.claim_net(&[bel.wire(pin)]);
    }
    bel.claim_net(&[bel.wire_far("O")]);
    bel.claim_net(&[bel.wire_far("T")]);
    bel.claim_pip(bel.wire("O"), bel.wire_far("O"));
    bel.claim_pip(bel.wire("T"), bel.wire_far("T"));

    let obel = bcrd.bel(bslots::IOB[idx ^ 1]);
    bel.claim_pip(bel.wire("DIFFI_IN"), bel.bel_wire(obel, "PADOUT"));
    if idx == 0 {
        bel.claim_pip(bel.wire("DIFFO_IN"), bel.bel_wire(obel, "DIFFO_OUT"));
    }
    bel.commit();
}

fn verify_tieoff(vrf: &mut Verifier, bel: &LegacyBelContext<'_>) {
    vrf.verify_legacy_bel(
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
        vrf.claim_net(&[bel.wire(pin)]);
    }
}

fn verify_pcilogicse(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .extra_in("IRDY")
        .extra_in("TRDY")
        .extra_out("PCI_CE");
    let (po, pi) = bel.pip("PCI_CE", 0);
    bel.claim_net(&[bel.wire("PCI_CE"), pi]);
    bel.claim_net(&[po]);
    bel.claim_pip(po, pi);
    let rdy = if bcrd.col == endev.chip.col_w() {
        [("IRDY", 2, bslots::IOB[0]), ("TRDY", -1, bslots::IOB[1])]
    } else {
        [("IRDY", 2, bslots::IOB[1]), ("TRDY", -1, bslots::IOB[0])]
    };
    for (pin, dy, slot) in rdy {
        let (po, pi) = bel.pip(pin, 0);
        bel.claim_net(&[bel.wire(pin), po]);
        bel.claim_pip(po, pi);
        let obel = bcrd.delta(0, dy).bel(slot);
        bel.claim_net(&[pi, bel.bel_wire_far(obel, "PCI_RDY")]);
        bel.claim_pip(
            bel.bel_wire_far(obel, "PCI_RDY"),
            bel.bel_wire(obel, "PCI_RDY"),
        );
    }
    bel.commit();
}

fn verify_mcb(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mcb = endev.chip.get_mcb(bcrd.row);
    let mut bel = vrf.verify_bel(bcrd);
    let mut rows_handled = HashSet::new();
    let mut rows_out_handled = HashSet::new();

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
    for (pin, io) in pins_out {
        bel = bel.extra_out(&pin);
        let (po, pi) = bel.pip(&pin, 0);
        bel.claim_pip(po, pi);
        bel.claim_net(&[bel.wire(&pin), pi]);
        let obel = bcrd
            .cell
            .with_row(io.row)
            .bel(bslots::OLOGIC[io.iob.to_idx()]);
        bel.claim_net(&[
            bel.wire_far(&pin),
            bel.bel_wire(obel, "MCB_D1"),
            bel.bel_wire(obel, "MCB_D2"),
        ]);
        if !rows_out_handled.contains(&io.row) {
            let obel = bcrd.with_row(io.row).bel(bslots::IOI);
            bel.claim_net(&[bel.bel_wire(obel, "MCB_T1")]);
            bel.claim_net(&[bel.bel_wire(obel, "MCB_T2")]);
        }
        rows_handled.insert(io.row);
        rows_out_handled.insert(io.row);
    }

    bel.claim_net(&[bel.wire_far("DQIOWEN0")]);
    for i in 0..16 {
        let dqi = &format!("DQI{i}");
        let dqop = &format!("DQOP{i}");
        let dqon = &format!("DQON{i}");
        let row = mcb.iop_dq[i / 2];
        let bi = (i % 2) ^ 1;
        bel = bel.extra_in(dqi).extra_out(dqop).extra_out(dqon);
        for pin in [dqop, dqon] {
            let (po, pi) = bel.pip(pin, 0);
            bel.claim_pip(po, pi);
            bel.claim_net(&[bel.wire(pin), pi]);
        }
        let (po, pi) = bel.pip(dqi, 0);
        bel.claim_pip(po, pi);
        bel.claim_net(&[bel.wire(dqi), po]);
        rows_handled.insert(row);

        let obel = bcrd.with_row(row).bel(bslots::OLOGIC[bi]);
        bel.claim_net(&[bel.wire_far(dqop), bel.bel_wire(obel, "MCB_D1")]);
        bel.claim_net(&[bel.wire_far(dqon), bel.bel_wire(obel, "MCB_D2")]);
        let obel = bcrd.with_row(row).bel(bslots::IODELAY[bi]);
        bel.verify_net(&[bel.wire_far(dqi), bel.bel_wire(obel, "MCB_DQSOUTP")]);
        let obel = bcrd.with_row(row).bel(bslots::IOI);
        bel.verify_net(&[
            bel.wire_far("DQIOWEN0"),
            bel.bel_wire(obel, "MCB_T1"),
            bel.bel_wire(obel, "MCB_T2"),
        ]);
    }

    for (op, on, io) in [
        ("LDMP", "LDMN", mcb.io_dm[0]),
        ("UDMP", "UDMN", mcb.io_dm[1]),
    ] {
        bel = bel.extra_out(op).extra_out(on);
        for pin in [op, on] {
            let (po, pi) = bel.pip(pin, 0);
            bel.claim_pip(po, pi);
            bel.claim_net(&[bel.wire(pin), pi]);
        }
        rows_handled.insert(io.row);
        let obel = bcrd.with_row(io.row).bel(bslots::OLOGIC[io.iob.to_idx()]);
        bel.claim_net(&[bel.wire_far(op), bel.bel_wire(obel, "MCB_D1")]);
        bel.claim_net(&[bel.wire_far(on), bel.bel_wire(obel, "MCB_D2")]);
        let obel = bcrd.with_row(io.row).bel(bslots::IOI);
        bel.verify_net(&[
            bel.wire_far("DQIOWEN0"),
            bel.bel_wire(obel, "MCB_T1"),
            bel.bel_wire(obel, "MCB_T2"),
        ]);
    }

    bel.claim_net(&[bel.wire_far("DQSIOWEN90P")]);
    bel.claim_net(&[bel.wire_far("DQSIOWEN90N")]);
    for (pp, pn, row) in [
        ("DQSIOIP", "DQSIOIN", mcb.iop_dqs[0]),
        ("UDQSIOIP", "UDQSIOIN", mcb.iop_dqs[1]),
    ] {
        bel = bel.extra_in(pp).extra_in(pn);
        for pin in [pp, pn] {
            let (po, pi) = bel.pip(pin, 0);
            bel.claim_pip(po, pi);
            bel.claim_net(&[bel.wire(pin), po]);
        }
        rows_handled.insert(row);
        let obel = bcrd.with_row(row).bel(bslots::IOI);
        bel.verify_net(&[bel.wire_far("DQSIOWEN90N"), bel.bel_wire(obel, "MCB_T1")]);
        bel.verify_net(&[bel.wire_far("DQSIOWEN90P"), bel.bel_wire(obel, "MCB_T2")]);
        let obel = bcrd.with_row(row).bel(bslots::IODELAY[1]);
        bel.verify_net(&[bel.wire_far(pp), bel.bel_wire(obel, "MCB_DQSOUTP")]);
        let obel = bcrd.with_row(row).bel(bslots::IODELAY[0]);
        bel.verify_net(&[bel.wire_far(pn), bel.bel_wire(obel, "MCB_DQSOUTP")]);
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
        bel = bel.extra_out(pin);
        let (po, pi) = bel.pip(pin, 0);
        bel.claim_pip(po, pi);
        bel.claim_net(&[bel.wire(pin), pi]);
        bel.claim_net(&[bel.wire_far(pin)]);
    }
    for pin in ["DQIOWEN0", "DQSIOWEN90P", "DQSIOWEN90N"] {
        bel = bel.extra_out(pin);
        let (po, pi) = bel.pip(pin, 0);
        bel.claim_pip(po, pi);
        bel.claim_net(&[bel.wire(pin), pi]);
    }

    {
        bel = bel.extra_in("IOIDRPSDI");
        let (po, pi) = bel.pip("IOIDRPSDI", 0);
        bel.claim_pip(po, pi);
        bel.claim_net(&[bel.wire("IOIDRPSDI"), po]);
    }

    {
        let obel = bcrd.with_row(mcb.iop_clk).bel(bslots::IOI);
        bel.claim_net(&[bel.bel_wire(obel, "MCB_T1")]);
        bel.claim_net(&[bel.bel_wire(obel, "MCB_T2")]);
        rows_handled.insert(mcb.iop_clk);
    }

    for row in endev.chip.rows.ids() {
        if let Some(split) = endev.chip.row_mcb_split {
            if bcrd.row < split && row >= split {
                continue;
            }
            if bcrd.row >= split && row < split {
                continue;
            }
        }
        let obel = bcrd.with_row(row).bel(bslots::IOI);
        if endev.edev.has_bel(obel) {
            for (pin, opin) in [
                ("IOIDRPCLK", "MCB_DRPCLK"),
                ("IOIDRPCS", "MCB_DRPCS"),
                ("IOIDRPADD", "MCB_DRPADD"),
                ("IOIDRPBROADCAST", "MCB_DRPBROADCAST"),
                ("IOIDRPSDO", "MCB_DRPSDO"),
                ("IOIDRPTRAIN", "MCB_DRPTRAIN"),
            ] {
                bel.verify_net(&[bel.wire_far(pin), bel.bel_wire(obel, opin)]);
            }
            for slot in bslots::IODELAY {
                let oobel = obel.bel(slot);
                for (pin, opin, dpin) in [
                    ("IOIDRPADDR0", "MCB_AUXADDR0", "AUXADDR0"),
                    ("IOIDRPADDR1", "MCB_AUXADDR1", "AUXADDR1"),
                    ("IOIDRPADDR2", "MCB_AUXADDR2", "AUXADDR2"),
                    ("IOIDRPADDR3", "MCB_AUXADDR3", "AUXADDR3"),
                    ("IOIDRPADDR4", "MCB_AUXADDR4", "AUXADDR4"),
                    ("IOIDRPUPDATE", "MCB_MEMUPDATE", "MEMUPDATE"),
                ] {
                    bel.verify_net(&[bel.wire_far(pin), bel.bel_wire(oobel, opin)]);
                    bel.claim_pip(bel.bel_wire(oobel, dpin), bel.bel_wire(oobel, opin));
                }
            }
            if !rows_handled.contains(&row) {
                bel.claim_net(&[bel.bel_wire(obel, "MCB_T1")]);
                bel.claim_net(&[bel.bel_wire(obel, "MCB_T2")]);
                for slot in bslots::OLOGIC {
                    let oobel = obel.bel(slot);
                    bel.claim_net(&[bel.bel_wire(oobel, "MCB_D1")]);
                    bel.claim_net(&[bel.bel_wire(oobel, "MCB_D2")]);
                }
            }
        }
    }
    let mut last = bel.wire_far("IOIDRPSDI");
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
        for slot in [bslots::IODELAY[1], bslots::IODELAY[0]] {
            let obel = bcrd.with_row(row).bel(slot);
            bel.claim_net(&[last, bel.bel_wire(obel, "MCB_AUXSDO")]);
            bel.claim_pip(
                bel.bel_wire(obel, "MCB_AUXSDO"),
                bel.bel_wire(obel, "AUXSDO"),
            );
            bel.claim_pip(
                bel.bel_wire(obel, "AUXSDOIN"),
                bel.bel_wire(obel, "MCB_AUXSDOIN"),
            );
            last = bel.bel_wire(obel, "MCB_AUXSDOIN");
        }
    }

    bel.commit();

    for (sub, prefix, row) in [
        (1, "TIE_CLK", mcb.iop_clk),
        (2, "TIE_DQS0", mcb.iop_dqs[0]),
        (3, "TIE_DQS1", mcb.iop_dqs[1]),
    ] {
        let mut bel = vrf
            .verify_bel(bcrd)
            .sub(sub)
            .kind("TIEOFF")
            .skip_auto()
            .extra_out_rename("HARD0", format!("{prefix}_HARD0"))
            .extra_out_rename("HARD1", format!("{prefix}_HARD1"))
            .extra_out_rename("KEEP1", format!("{prefix}_KEEP1"));
        bel.claim_net(&[bel.wire(&format!("{prefix}_HARD0"))]);
        bel.claim_net(&[bel.wire(&format!("{prefix}_HARD1"))]);
        bel.claim_net(&[bel.wire(&format!("{prefix}_KEEP1"))]);
        bel.claim_pip(
            bel.wire(&format!("{prefix}_OUTP0")),
            bel.wire(&format!("{prefix}_HARD0")),
        );
        bel.claim_pip(
            bel.wire(&format!("{prefix}_OUTN0")),
            bel.wire(&format!("{prefix}_HARD1")),
        );
        bel.claim_pip(
            bel.wire(&format!("{prefix}_OUTP1")),
            bel.wire(&format!("{prefix}_HARD1")),
        );
        bel.claim_pip(
            bel.wire(&format!("{prefix}_OUTN1")),
            bel.wire(&format!("{prefix}_HARD0")),
        );
        let o0 = bcrd.cell.with_row(row).bel(bslots::OLOGIC[1]);
        bel.claim_net(&[
            bel.wire(&format!("{prefix}_OUTP0")),
            bel.bel_wire(o0, "MCB_D1"),
        ]);
        bel.claim_net(&[
            bel.wire(&format!("{prefix}_OUTN0")),
            bel.bel_wire(o0, "MCB_D2"),
        ]);
        let o1 = bcrd.cell.with_row(row).bel(bslots::OLOGIC[0]);
        bel.claim_net(&[
            bel.wire(&format!("{prefix}_OUTP1")),
            bel.bel_wire(o1, "MCB_D1"),
        ]);
        bel.claim_net(&[
            bel.wire(&format!("{prefix}_OUTN1")),
            bel.bel_wire(o1, "MCB_D2"),
        ]);

        bel.commit();
    }
}

fn verify_hclk_row(vrf: &mut Verifier, bcrd: BelCoord) {
    for (i, we) in ['W', 'E'].into_iter().enumerate() {
        for j in 0..16 {
            let iname = &format!("I_{we}{j}");
            let oname = &format!("O_{we}{j}");
            let mut bel = vrf
                .verify_bel(bcrd)
                .kind("BUFH")
                .skip_auto()
                .sub(i * 16 + j)
                .extra_in_rename("I", iname)
                .extra_out_rename("O", oname);
            bel.claim_net(&[bel.wire(iname)]);
            bel.claim_net(&[bel.wire(oname)]);
            bel.claim_pip(bel.wire(iname), bel.wire_far(iname));
            bel.claim_pip(bel.wire_far(oname), bel.wire(oname));
            bel.commit();
        }
    }
}

fn verify_bufio2(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).skip_out(bcls::BUFIO2::DIVCLK_CMT);

    bel.claim_pip(bel.wire("IB"), bel.wire("TIE_1"));

    bel.claim_pip(bel.wire("DIVCLK_CMT"), bel.wire_far("DIVCLK"));
    bel.claim_pip(bel.wire("DIVCLK_CMT"), bel.wire("TIE_1"));

    bel.claim_pip(bel.wire_far("IOCLK"), bel.wire("TIE_0"));
    bel.commit();
}

fn verify_bufio2fb(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).extra_in("IB");
    bel.claim_pip(bel.wire_far("O"), bel.wire("TIE_1"));
    bel.commit();
}

fn verify_bufpll(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    vrf.verify_bel(bcrd)
        .kind("BUFPLL_MCB")
        .skip_auto()
        .extra_in_rename("PLLIN0", "BUFPLL_MCB_PLLIN0")
        .extra_in_rename("PLLIN1", "BUFPLL_MCB_PLLIN1")
        .extra_in_rename("LOCKED", "BUFPLL_MCB_LOCKED")
        .extra_in_rename("GCLK", "BUFPLL_MCB_GCLK")
        .extra_out_rename("IOCLK0", "BUFPLL_MCB_IOCLK0")
        .extra_out_rename("IOCLK1", "BUFPLL_MCB_IOCLK1")
        .extra_out_rename("SERDESSTROBE0", "BUFPLL_MCB_SERDESSTROBE0")
        .extra_out_rename("SERDESSTROBE1", "BUFPLL_MCB_SERDESSTROBE1")
        .extra_out_rename("LOCK", "BUFPLL_MCB_LOCK")
        .commit();

    for i in 0..2 {
        vrf.verify_bel(bcrd)
            .sub(1 + i)
            .skip_auto()
            .extra_in_rename("PLLIN", format!("BUFPLL{i}_PLLIN"))
            .extra_in_rename("GCLK", format!("BUFPLL{i}_GCLK"))
            .extra_in_rename("LOCKED", format!("BUFPLL{i}_LOCKED"))
            .extra_out_rename("IOCLK", format!("BUFPLL{i}_IOCLK"))
            .extra_out_rename("SERDESSTROBE", format!("BUFPLL{i}_SERDESSTROBE"))
            .extra_out_rename("LOCK", format!("BUFPLL{i}_LOCK"))
            .commit();
    }

    let mut bel = vrf.verify_bel(bcrd);
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
        bel.claim_net(&[bel.wire(&format!("BUFPLL_MCB_{pin}"))]);
    }
    for i in 0..2 {
        for pin in ["PLLIN", "GCLK", "LOCKED", "IOCLK", "SERDESSTROBE", "LOCK"] {
            bel.claim_net(&[bel.wire(&format!("BUFPLL{i}_{pin}"))]);
        }
    }
    if endev.chip.columns[bcrd.col].kind == ColumnKind::Io {
        bel.claim_pip(bel.wire("BUFPLL_MCB_GCLK"), bel.wire("GCLK0"));
        bel.claim_pip(bel.wire("BUFPLL_MCB_PLLIN0"), bel.wire("PLLIN_GCLK0"));
        bel.claim_pip(bel.wire("BUFPLL_MCB_PLLIN0"), bel.wire("PLLIN_CMT0"));
        bel.claim_pip(bel.wire("BUFPLL_MCB_PLLIN1"), bel.wire("PLLIN_GCLK1"));
        bel.claim_pip(bel.wire("BUFPLL_MCB_PLLIN1"), bel.wire("PLLIN_CMT1"));
        bel.claim_pip(bel.wire("BUFPLL_MCB_LOCKED"), bel.wire("LOCKED0"));
        for i in 0..2 {
            bel.claim_pip(
                bel.wire(&format!("BUFPLL{i}_GCLK")),
                bel.wire(&format!("GCLK{i}")),
            );
            bel.claim_pip(
                bel.wire(&format!("BUFPLL{i}_PLLIN")),
                bel.wire(&format!("PLLIN_GCLK{i}")),
            );
            bel.claim_pip(
                bel.wire(&format!("BUFPLL{i}_PLLIN")),
                bel.wire(&format!("PLLIN_CMT{i}")),
            );
            bel.claim_pip(
                bel.wire(&format!("BUFPLL{i}_LOCKED")),
                bel.wire(&format!("LOCKED{i}")),
            );
        }
    } else {
        bel.claim_pip(bel.wire("BUFPLL_MCB_GCLK"), bel.wire("GCLK0"));
        for j in 0..6 {
            bel.claim_pip(
                bel.wire("BUFPLL_MCB_PLLIN0"),
                bel.wire(&format!("PLLIN_SN{j}")),
            );
            bel.claim_pip(
                bel.wire("BUFPLL_MCB_PLLIN1"),
                bel.wire(&format!("PLLIN_SN{j}")),
            );
        }
        for j in 0..3 {
            bel.claim_pip(
                bel.wire("BUFPLL_MCB_LOCKED"),
                bel.wire(&format!("LOCKED_SN{j}")),
            );
        }
        for i in 0..2 {
            bel.claim_pip(
                bel.wire(&format!("BUFPLL{i}_GCLK")),
                bel.wire(&format!("GCLK{i}")),
            );
            for j in 0..6 {
                bel.claim_pip(
                    bel.wire(&format!("BUFPLL{i}_PLLIN")),
                    bel.wire(&format!("PLLIN_SN{j}")),
                );
            }
            for j in 0..3 {
                bel.claim_pip(
                    bel.wire(&format!("BUFPLL{i}_LOCKED")),
                    bel.wire(&format!("LOCKED_SN{j}")),
                );
            }
        }
    }

    bel.claim_pip(bel.wire("PLLCLK0"), bel.wire("BUFPLL0_IOCLK"));
    bel.claim_pip(bel.wire("PLLCLK0"), bel.wire("BUFPLL_MCB_IOCLK0"));
    bel.claim_pip(bel.wire("PLLCLK0"), bel.wire("TIE_1"));
    bel.claim_pip(bel.wire("PLLCLK1"), bel.wire("BUFPLL1_IOCLK"));
    bel.claim_pip(bel.wire("PLLCLK1"), bel.wire("BUFPLL_MCB_IOCLK1"));
    bel.claim_pip(bel.wire("PLLCLK1"), bel.wire("TIE_1"));
    bel.claim_pip(bel.wire("PLLCE0"), bel.wire("BUFPLL0_SERDESSTROBE"));
    bel.claim_pip(bel.wire("PLLCE0"), bel.wire("BUFPLL_MCB_SERDESSTROBE0"));
    bel.claim_pip(bel.wire("PLLCE1"), bel.wire("BUFPLL1_SERDESSTROBE"));
    bel.claim_pip(bel.wire("PLLCE1"), bel.wire("BUFPLL_MCB_SERDESSTROBE1"));
    bel.claim_pip(bel.wire("LOCK0"), bel.wire("BUFPLL0_LOCK"));
    bel.claim_pip(bel.wire("LOCK0"), bel.wire("BUFPLL_MCB_LOCK"));
    bel.claim_pip(bel.wire("LOCK1"), bel.wire("BUFPLL1_LOCK"));
}

fn verify_dcm(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::DCM.index_of(bcrd.slot).unwrap();
    let mut bel = vrf.verify_bel(bcrd);

    let BelInfo::SwitchBox(ref sb) = endev.edev.db[tcls::CMT_DCM].bels[bslots::CMT_INT] else {
        unreachable!()
    };
    let tcrd = endev.edev.get_tile_by_bel(bcrd);
    let ntile = &endev.ngrid.tiles[&tcrd];
    let ntcls = &endev.ngrid.db.tile_class_namings[ntile.naming];
    for (wt, pin) in [
        (wires::IMUX_DCM_CLKIN[idx], "CLKIN_TEST"),
        (wires::IMUX_DCM_CLKFB[idx], "CLKFB_TEST"),
    ] {
        for item in &sb.items {
            let SwitchBoxItem::Mux(mux) = item else {
                continue;
            };
            if mux.dst.wire != wt {
                continue;
            }
            for src in mux.src.keys() {
                let wn = &ntcls.wires[&src.tw];
                let rw = RawWireCoord {
                    crd: bel.crd(),
                    wire: wn.alt_name.as_ref().unwrap_or(&wn.name),
                };
                bel.claim_pip(bel.wire(pin), rw);
            }
        }
    }

    bel.commit();
}

fn verify_pll(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("PLL_ADV")
        .skip_out(bcls::PLL::TEST_CLKIN)
        .extra_in("REL");
    bel.claim_net(&[bel.wire("REL")]);
    bel.claim_pip(bel.wire("TEST_CLKIN"), bel.wire_far("CLKIN1"));
    bel.claim_pip(bel.wire("REL"), bel.wire("TIE_PLL_HARD1"));
    bel.commit();

    let mut bel = vrf
        .verify_bel(bcrd)
        .sub(1)
        .kind("TIEOFF")
        .skip_auto()
        .extra_out_rename("HARD0", "TIE_PLL_HARD0")
        .extra_out_rename("HARD1", "TIE_PLL_HARD1")
        .extra_out_rename("KEEP1", "TIE_PLL_KEEP1");
    bel.claim_net(&[bel.wire("TIE_PLL_HARD0")]);
    bel.claim_net(&[bel.wire("TIE_PLL_HARD1")]);
    bel.claim_net(&[bel.wire("TIE_PLL_KEEP1")]);
    bel.commit();
}

fn verify_gtp(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let side = DirHV {
        h: if bcrd.col <= endev.chip.col_clk {
            DirH::W
        } else {
            DirH::E
        },
        v: if bcrd.row < endev.chip.row_clk() {
            DirV::S
        } else {
            DirV::N
        },
    };
    let mut bel = vrf.verify_bel(bcrd).kind("GTPA1_DUAL");
    for (idx, pin) in [(7, "RXP0"), (8, "RXN0"), (9, "RXP1"), (10, "RXN1")] {
        let ipad_pin = &format!("IPAD_{pin}_O");
        bel = bel.extra_in(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_net(&[bel.wire(ipad_pin)]);
        bel.claim_pip(bel.wire(pin), bel.wire(ipad_pin));
        bel.vrf
            .verify_bel(bcrd)
            .sub(idx)
            .kind("IPAD")
            .skip_auto()
            .extra_out_rename("O", ipad_pin)
            .commit();
    }
    for (idx, pin) in [(3, "TXP0"), (4, "TXN0"), (5, "TXP1"), (6, "TXN1")] {
        let opad_pin = &format!("OPAD_{pin}_I");
        bel = bel.extra_out(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_net(&[bel.wire(opad_pin)]);
        bel.claim_pip(bel.wire(opad_pin), bel.wire(pin));
        bel.vrf
            .verify_bel(bcrd)
            .sub(idx)
            .kind("OPAD")
            .skip_auto()
            .extra_in_rename("I", opad_pin)
            .commit();
    }

    for (pin, pin_bufds) in [
        ("CLK00", "BUFDS0_O"),
        ("CLK01", "BUFDS0_O"),
        ("CLK10", "BUFDS1_O"),
        ("CLK11", "BUFDS1_O"),
    ] {
        bel = bel.extra_in(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.wire(pin), bel.wire(pin_bufds));
    }
    for (pin, opin) in [
        ("CLKINEAST0", "CLKINEAST"),
        ("CLKINEAST1", "CLKINEAST"),
        ("CLKINWEST0", "CLKINWEST"),
        ("CLKINWEST1", "CLKINWEST"),
    ] {
        bel = bel.extra_in(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.wire(pin), bel.wire(opin));
    }
    for pin in ["RXCHBONDI0", "RXCHBONDI1", "RXCHBONDI2"] {
        bel = bel.extra_in(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
    }
    for pin in ["RXCHBONDO0", "RXCHBONDO1", "RXCHBONDO2"] {
        bel = bel.extra_out(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.wire_far(pin), bel.wire(pin));
        bel.claim_net(&[bel.wire_far(pin)]);
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
        bel = bel.extra_in(pin);
        bel.claim_net(&[bel.wire(pin)]);
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
        bel = bel.extra_out(pin);
        bel.claim_net(&[bel.wire(pin)]);
    }
    bel.claim_net(&[bel.wire("CLKOUT_EW")]);
    for pin in ["REFCLKPLL0", "REFCLKPLL1"] {
        bel = bel.extra_out(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.wire("CLKOUT_EW"), bel.wire(pin));
    }

    if let Some(obel) = endev.chip.bel_gtp(side.with_h(!side.h)) {
        for (pin, opin) in [
            ("RXCHBONDI0", "RXCHBONDO0"),
            ("RXCHBONDI1", "RXCHBONDO1"),
            ("RXCHBONDI2", "RXCHBONDO2"),
        ] {
            bel.verify_net(&[bel.wire_far(pin), bel.bel_wire_far(obel, opin)]);
        }
    } else {
        for pin in ["RXCHBONDI0", "RXCHBONDI1", "RXCHBONDI2"] {
            bel.claim_net(&[bel.wire_far(pin)]);
        }
    }

    if side.h == DirH::W {
        for i in 0..5 {
            bel.claim_net(&[bel.wire(&format!("RCALOUTEAST{i}_BUF"))]);
            bel.claim_pip(
                bel.wire(&format!("RCALOUTEAST{i}_BUF")),
                bel.wire(&format!("RCALOUTEAST{i}")),
            );
        }
        if let Some(obel) = endev.chip.bel_gtp(side.with_h(DirH::E)) {
            bel.verify_net(&[bel.wire("CLKINWEST"), bel.bel_wire(obel, "CLKOUT_EW")]);
        } else {
            bel.claim_net(&[bel.wire("CLKINWEST")]);
        }
        bel.claim_net(&[bel.wire("CLKINEAST")]);
    } else {
        let obel = endev.chip.bel_gtp(side.with_h(DirH::W)).unwrap();
        for i in 0..5 {
            bel.claim_pip(
                bel.wire(&format!("RCALINEAST{i}")),
                bel.wire(&format!("RCALINEAST{i}_BUF")),
            );
            bel.verify_net(&[
                bel.wire(&format!("RCALINEAST{i}_BUF")),
                bel.bel_wire(obel, &format!("RCALOUTEAST{i}_BUF")),
            ]);
        }
        bel.verify_net(&[bel.wire("CLKINEAST"), bel.bel_wire(obel, "CLKOUT_EW")]);
        bel.claim_net(&[bel.wire("CLKINWEST")]);
    }

    bel.commit();

    for i in 0..2 {
        let mut bel = vrf
            .verify_bel(bcrd)
            .sub(1 + i)
            .kind("BUFDS")
            .skip_auto()
            .extra_out_rename("O", format!("BUFDS{i}_O"));
        bel.claim_net(&[bel.wire(&format!("BUFDS{i}_O"))]);

        for (idx, name, spin) in [(11 + i * 2, "CLKP", "I"), (12 + i * 2, "CLKN", "IB")] {
            let ipad_pin = &format!("IPAD_{name}{i}_O");
            let pin = &format!("BUFDS{i}_{spin}");
            bel = bel.extra_in_rename(spin, pin);
            bel.claim_net(&[bel.wire(pin)]);
            bel.claim_net(&[bel.wire(ipad_pin)]);
            bel.claim_pip(bel.wire(pin), bel.wire(ipad_pin));
            bel.vrf
                .verify_bel(bcrd)
                .sub(idx)
                .kind("IPAD")
                .skip_auto()
                .extra_out_rename("O", ipad_pin)
                .commit();
        }

        bel.commit();
    }
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let slot_name = endev.edev.db.bel_slots.key(bcrd.slot);
    match bcrd.slot {
        bslots::INT
        | bslots::INTF_INT
        | bslots::INTF_TESTMUX
        | bslots::CLK_INT
        | bslots::HCLK
        | bslots::CLKC_INT
        | bslots::CMT_INT
        | bslots::CMT_VREG
        | bslots::CMT_BUF
        | bslots::MISR_CLK
        | bslots::MISR_CNR_H
        | bslots::MISR_CNR_V
        | bslots::MISC_SW
        | bslots::MISC_SE
        | bslots::MISC_NW
        | bslots::MISC_NE
        | bslots::GLUTMASK_HCLK => {}
        _ if bcrd.slot == bslots::SLICE[0] => verify_sliceml(endev, vrf, bcrd),
        _ if bcrd.slot == bslots::SLICE[1] => vrf.verify_bel(bcrd).kind("SLICEX").commit(),
        bslots::BRAM_F => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "RAMB16BWER", &[], &[]);
        }
        _ if bslots::BRAM_H.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "RAMB8BWER", &[], &[]);
        }
        bslots::DSP => verify_dsp(endev, vrf, bcrd),
        bslots::PCIE => vrf.verify_bel(bcrd).kind("PCIE_A1").commit(),

        _ if bslots::OCT_CAL.contains(bcrd.slot) => {
            vrf.verify_bel(bcrd).kind("OCT_CALIBRATE").commit()
        }
        _ if bslots::BANK.contains(bcrd.slot) => (),
        _ if bslots::BSCAN.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        bslots::PMV
        | bslots::DNA_PORT
        | bslots::ICAP
        | bslots::SPI_ACCESS
        | bslots::SUSPEND_SYNC
        | bslots::POST_CRC_INTERNAL
        | bslots::STARTUP
        | bslots::SLAVE_SPI => vrf.verify_bel(bcrd).commit(),

        _ if bslots::ILOGIC.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            verify_ilogic(vrf, bel);
        }
        _ if bslots::OLOGIC.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            verify_ologic(vrf, bel);
        }
        _ if bslots::IODELAY.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            verify_iodelay(vrf, bel);
        }
        _ if bslots::IOICLK.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            verify_ioiclk(vrf, bel);
        }
        bslots::IOI => {
            let bel = &vrf.get_legacy_bel(bcrd);
            verify_ioi(endev, vrf, bel);
        }
        _ if bslots::IOB.contains(bcrd.slot) => verify_iob(vrf, bcrd),
        _ if slot_name.starts_with("TIEOFF") => {
            let bel = &vrf.get_legacy_bel(bcrd);
            verify_tieoff(vrf, bel)
        }

        bslots::PCILOGICSE => verify_pcilogicse(endev, vrf, bcrd),
        bslots::MCB => verify_mcb(endev, vrf, bcrd),

        bslots::HCLK_ROW => verify_hclk_row(vrf, bcrd),
        _ if bslots::BUFGMUX.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),

        _ if bslots::BUFIO2.contains(bcrd.slot) => verify_bufio2(vrf, bcrd),
        _ if bslots::BUFIO2FB.contains(bcrd.slot) => verify_bufio2fb(vrf, bcrd),
        bslots::BUFPLL => verify_bufpll(endev, vrf, bcrd),

        _ if bslots::DCM.contains(bcrd.slot) => verify_dcm(endev, vrf, bcrd),
        bslots::PLL => verify_pll(vrf, bcrd),

        bslots::GTP => verify_gtp(endev, vrf, bcrd),

        _ => println!("MEOW {}", bcrd.to_string(endev.edev.db)),
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
                let wo = RawWireCoord {
                    crd,
                    wire: &vrf.rd.wires[wo],
                };
                let wi = RawWireCoord {
                    crd,
                    wire: &vrf.rd.wires[wi],
                };
                vrf.claim_net(&[wo]);
                vrf.claim_net(&[wi]);
                vrf.claim_pip(wo, wi);
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
                let wt = RawWireCoord {
                    crd,
                    wire: &vrf.rd.wires[key.1],
                };
                let wf = RawWireCoord {
                    crd,
                    wire: &vrf.rd.wires[key.0],
                };
                vrf.claim_pip(wt, wf);
            }
        }
    }
    for (tkn, base) in [("GTPDUAL_BOT", 9), ("GTPDUAL_TOP", 1)] {
        for &crd in vrf.rd.tiles_by_kind_name(tkn) {
            let mut junk = vec![];
            for i in 0..8 {
                let idx = base + i;
                for j in 0..63 {
                    junk.push(format!("GTPDUAL_LEFT_LOGICIN_B{j}_{idx}"));
                    junk.push(format!("GTPDUAL_RIGHT_LOGICIN_B{j}_{idx}"));
                }
                for j in 0..2 {
                    junk.push(format!("GTPDUAL_LEFT_CLK{j}_{idx}"));
                    junk.push(format!("GTPDUAL_RIGHT_CLK{j}_{idx}"));
                    junk.push(format!("GTPDUAL_LEFT_SR{j}_{idx}"));
                    junk.push(format!("GTPDUAL_RIGHT_SR{j}_{idx}"));
                }
                for j in 0..24 {
                    junk.push(format!("GTPDUAL_LEFT_LOGICOUT{j}_{idx}"));
                    junk.push(format!("GTPDUAL_RIGHT_LOGICOUT{j}_{idx}"));
                }
            }
            for wire in junk {
                vrf.claim_net(&[RawWireCoord { crd, wire: &wire }]);
            }
        }
    }
    vrf.kill_stub_out_cond("REGB_BTERM_ALTGTP_CLKINWEST0");
    vrf.kill_stub_out_cond("REGB_BTERM_ALTGTP_CLKOUTEW0");
    vrf.kill_stub_out_cond("REGB_BTERM_GTP_CLKINEAST0");
    vrf.kill_stub_out_cond("REGB_BTERM_GTP_CLKOUTEW0");
    vrf.kill_stub_out_cond("REGT_TTERM_ALTGTP_CLKINWEST0");
    vrf.kill_stub_out_cond("REGT_TTERM_ALTGTP_CLKOUTEW0");
    vrf.kill_stub_out_cond("REGT_TTERM_GTP_CLKINEAST0");
    vrf.kill_stub_out_cond("REGT_TTERM_GTP_CLKOUTEW0");
    vrf.kill_stub_out_cond("REGL_GTPCLK1");
    vrf.kill_stub_out_cond("REGL_GTPCLK3");
    vrf.kill_stub_out_cond("REGR_GTPCLK1");
    vrf.kill_stub_out_cond("REGR_GTPCLK3");
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);
    for (tkn, is_trunk_s, is_v_s) in [
        ("HCLK_IOIL_BOT_DN", true, true),
        ("HCLK_IOIL_BOT_UP", true, false),
        ("HCLK_IOIL_TOP_DN", false, true),
        ("HCLK_IOIL_TOP_UP", false, false),
        ("HCLK_IOIR_BOT_DN", true, true),
        ("HCLK_IOIR_BOT_UP", true, false),
        ("HCLK_IOIR_TOP_DN", false, true),
        ("HCLK_IOIR_TOP_UP", false, false),
    ] {
        if is_trunk_s {
            vrf.mark_merge_pip(tkn, "HCLK_PCI_CE_TRUNK_IN", "HCLK_PCI_CE_TRUNK_OUT");
        } else {
            vrf.mark_merge_pip(tkn, "HCLK_PCI_CE_TRUNK_OUT", "HCLK_PCI_CE_TRUNK_IN");
        }
        if is_v_s {
            vrf.mark_merge_pip(tkn, "HCLK_PCI_CE_IN", "HCLK_PCI_CE_OUT");
        } else {
            vrf.mark_merge_pip(tkn, "HCLK_PCI_CE_OUT", "HCLK_PCI_CE_IN");
        }
    }
    for tkn in [
        "HCLK_IOIL_BOT_SPLIT",
        "HCLK_IOIL_TOP_SPLIT",
        "HCLK_IOIR_BOT_SPLIT",
        "HCLK_IOIR_TOP_SPLIT",
    ] {
        vrf.mark_merge_pip(tkn, "HCLK_PCI_CE_INOUT", "HCLK_PCI_CE_SPLIT");
    }
    for tkn in ["IOI_PCI_CE_LEFT", "IOI_PCI_CE_RIGHT"] {
        vrf.mark_merge_pip(tkn, "IOI_PCICE_EW", "IOI_PCICE_TB");
    }
    for tkn in [
        "BRAM_BOT_BTERM_L",
        "BRAM_BOT_BTERM_R",
        "BRAM_TOP_TTERM_L",
        "BRAM_TOP_TTERM_R",
    ] {
        vrf.mark_merge_pip(tkn, "BRAM_TTERM_PCICE_OUT", "BRAM_TTERM_PCICE_IN");
    }
    for tkn in [
        "DSP_BOT_BTERM_L",
        "DSP_BOT_BTERM_R",
        "DSP_TOP_TTERM_L",
        "DSP_TOP_TTERM_R",
    ] {
        vrf.mark_merge_pip(tkn, "MACCSITE2_TTERM_PCICE_OUT", "MACCSITE2_TTERM_PCICE_IN");
    }
    for tkn in ["REG_V_HCLKBUF_BOT", "REG_V_HCLKBUF_TOP"] {
        for i in 0..16 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CLKV_GCLK_MAIN{i}_BUF"),
                &format!("CLKV_MIDBUF_GCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("CLKV_MIDBUF_GCLK{i}"),
                &format!("CLKV_GCLK_MAIN{i}"),
            );
        }
    }
    for i in 0..8 {
        vrf.mark_merge_pip(
            "REG_V_MIDBUF_BOT",
            &format!("CLKV_CKPIN_BOT_BUF{i}"),
            &format!("CLKV_MIDBUF_BOT_CKPIN{i}"),
        );
        vrf.mark_merge_pip(
            "REG_V_MIDBUF_TOP",
            &format!("CLKV_MIDBUF_TOP_CKPIN{i}"),
            &format!("CLKV_CKPIN_BUF{i}"),
        );
    }
    for tkn in [
        "REGH_DSP_L",
        "REGH_DSP_R",
        "REGH_CLEXL_INT_CLK",
        "REGH_CLEXM_INT_GCLKL",
        "REGH_BRAM_FEEDTHRU_L_GCLK",
        "REGH_BRAM_FEEDTHRU_R_GCLK",
    ] {
        for i in 0..8 {
            vrf.mark_merge_pip(
                tkn,
                &format!("REGH_DSP_OUT_CKPIN{i}"),
                &format!("REGH_DSP_IN_CKPIN{i}"),
            );
        }
    }
    for i in 0..16 {
        vrf.mark_merge_pip(
            "DSP_HCLK_GCLK_FOLD",
            &format!("HCLK_GCLK{i}_DSP_FOLD"),
            &format!("HCLK_MIDBUF_GCLK{i}"),
        );
        vrf.mark_merge_pip(
            "DSP_HCLK_GCLK_FOLD",
            &format!("HCLK_MIDBUF_GCLK{i}"),
            &format!("HCLK_GCLK{i}_DSP_NOFOLD"),
        );
        vrf.mark_merge_pip(
            "GTPDUAL_DSP_FEEDTHRU",
            &format!("HCLK_GCLK{i}_GTPDSP_FOLD"),
            &format!("GTP_MIDBUF_GCLK{i}"),
        );
        vrf.mark_merge_pip(
            "GTPDUAL_DSP_FEEDTHRU",
            &format!("GTP_MIDBUF_GCLK{i}"),
            &format!("GTP_DSP_FEEDTHRU_HCLK_GCLK{i}"),
        );
    }
    for tkn in [
        "HCLK_IOIL_BOT_DN",
        "HCLK_IOIL_BOT_SPLIT",
        "HCLK_IOIL_BOT_UP",
        "HCLK_IOIL_TOP_DN",
        "HCLK_IOIL_TOP_SPLIT",
        "HCLK_IOIL_TOP_UP",
        "HCLK_IOIR_BOT_DN",
        "HCLK_IOIR_BOT_SPLIT",
        "HCLK_IOIR_BOT_UP",
        "HCLK_IOIR_TOP_DN",
        "HCLK_IOIR_TOP_SPLIT",
        "HCLK_IOIR_TOP_UP",
    ] {
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOIL_IOCLK{i}_DOWN"),
                &format!("HCLK_IOIL_IOCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOIL_IOCLK{i}_UP"),
                &format!("HCLK_IOIL_IOCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOIL_IOCE{i}_DOWN"),
                &format!("HCLK_IOIL_IOCE{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOIL_IOCE{i}_UP"),
                &format!("HCLK_IOIL_IOCE{i}"),
            );
        }
        for i in 0..2 {
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOIL_PLLCLK{i}_DOWN"),
                &format!("HCLK_IOIL_PLLCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOIL_PLLCLK{i}_UP"),
                &format!("HCLK_IOIL_PLLCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOIL_PLLCE{i}_DOWN"),
                &format!("HCLK_IOIL_PLLCE{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOIL_PLLCE{i}_UP"),
                &format!("HCLK_IOIL_PLLCE{i}"),
            );
        }
    }
    for tkn in ["HCLK_IOI_LTERM", "HCLK_IOI_LTERM_BOT25"] {
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOI_LTERM_IOCLK{i}_E"),
                &format!("HCLK_IOI_LTERM_IOCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOI_LTERM_IOCE{i}_E"),
                &format!("HCLK_IOI_LTERM_IOCE{i}"),
            );
        }
        for i in 0..2 {
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOI_LTERM_PLLCLK{i}_E"),
                &format!("HCLK_IOI_LTERM_PLLCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOI_LTERM_PLLCE{i}_E"),
                &format!("HCLK_IOI_LTERM_PLLCE{i}"),
            );
        }
    }
    for tkn in ["HCLK_IOI_RTERM", "HCLK_IOI_RTERM_BOT25"] {
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOI_RTERM_IOCLK{i}_W"),
                &format!("HCLK_IOI_RTERM_IOCLK{ii}", ii = i ^ 3),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOI_RTERM_IOCE{i}_W"),
                &format!("HCLK_IOI_RTERM_IOCE{ii}", ii = i ^ 3),
            );
        }
        for i in 0..2 {
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOI_RTERM_PLLCLKOUT{i}_W"),
                &format!("HCLK_IOI_RTERM_PLLCLKOUT{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("HCLK_IOI_RTERM_PLLCEOUT{i}_W"),
                &format!("HCLK_IOI_RTERM_PLLCEOUT{i}"),
            );
        }
    }
    for tkn in ["IOI_BTERM_CLB", "IOI_BTERM_REGB"] {
        vrf.mark_merge_pip(tkn, "BTERM_CLB_PCICE_N", "BTERM_CLB_PCICE");
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("BTERM_CLB_CLKOUT{i}_N"),
                &format!("BTERM_CLB_CLKOUT{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("BTERM_CLB_CEOUT{i}_N"),
                &format!("BTERM_CLB_CEOUT{i}"),
            );
        }
        for i in 0..2 {
            vrf.mark_merge_pip(
                tkn,
                &format!("BTERM_CLB_PLLCLKOUT{i}_N"),
                &format!("BTERM_CLB_PLLCLKOUT{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("BTERM_CLB_PLLCEOUT{i}_N"),
                &format!("BTERM_CLB_PLLCEOUT{i}"),
            );
        }
    }
    for tkn in ["IOI_TTERM_CLB", "IOI_TTERM_REGT"] {
        vrf.mark_merge_pip(tkn, "TTERM_CLB_PCICE_S", "TTERM_CLB_PCICE");
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("TTERM_CLB_IOCLK{i}_S"),
                &format!("TTERM_CLB_IOCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("TTERM_CLB_IOCE{i}_S"),
                &format!("TTERM_CLB_IOCE{i}"),
            );
        }
        for i in 0..2 {
            vrf.mark_merge_pip(
                tkn,
                &format!("TTERM_CLB_PLLCLK{i}_S"),
                &format!("TTERM_CLB_PLLCLK{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("TTERM_CLB_PLLCE{i}_S"),
                &format!("TTERM_CLB_PLLCE{i}"),
            );
        }
    }
    for tkn in ["BRAM_TOP_TTERM_L", "BRAM_TOP_TTERM_R"] {
        for i in 0..2 {
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_TTERM_PLLCLK{i}_N"),
                &format!("BRAM_TTERM_PLLCLK{i}"),
            );
        }
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_TTERM_GTPCLK{ii}", ii = i + 4),
                &format!("BRAM_TTERM_GTPCLK{i}_N"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_TTERM_GTPFB{ii}", ii = i + 4),
                &format!("BRAM_TTERM_GTPCLKFB{i}_N"),
            );
        }
        for i in 0..5 {
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_TTERM_RCALINEAST{i}_N"),
                &format!("BRAM_TTERM_RCALINEAST{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_TTERM_RCALOUTEAST{i}"),
                &format!("BRAM_TTERM_RCALOUTEAST{i}_N"),
            );
        }
        for i in 0..3 {
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_TTERM_RXCHBONDO{i}_N"),
                &format!("BRAM_TTERM_RXCHBONDO{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_TTERM_RXCHBONDI{i}"),
                &format!("BRAM_TTERM_RXCHBONDI{i}_N"),
            );
        }
        vrf.mark_merge_pip(tkn, "BRAM_TTERM_CLKOUTEAST0_N", "BRAM_TTERM_CLKOUTEAST0");
        vrf.mark_merge_pip(tkn, "BRAM_TTERM_CLKOUTWEST0_N", "BRAM_TTERM_CLKOUTWEST0");
        vrf.mark_merge_pip(tkn, "BRAM_TTERM_CLKOUT_EW0", "BRAM_TTERM_CLKOUT_EW0_N");
    }
    for tkn in ["BRAM_BOT_BTERM_L", "BRAM_BOT_BTERM_R"] {
        for i in 0..2 {
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_BTERM_PLLCLK{i}_S"),
                &format!("IOI_BTERM_PLLCLKOUT{i}"),
            );
        }
        for i in 0..4 {
            vrf.mark_merge_pip(
                tkn,
                &format!("IOI_BTERM_GTPCLK{i}"),
                &format!("BRAM_BTERM_GTPCLK{i}_S"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("IOI_BTERM_GTPFB{i}"),
                &format!("BRAM_BTERM_GTPFB{i}_S"),
            );
        }
        for i in 0..5 {
            vrf.mark_merge_pip(
                tkn,
                &format!("BRAM_BTERM_RCALINEAST{i}_S"),
                &format!("IOI_BTERM_RCALINEAST{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("IOI_BTERM_RCALOUTEAST{i}"),
                &format!("BRAM_BTERM_RCALOUTEAST{i}_S"),
            );
        }
        for i in 0..3 {
            vrf.mark_merge_pip(
                tkn,
                // I FUCKING HATE SPARTAN 6 IT IS A PIECE OF SHIT
                &if i == 0 {
                    format!("BRAM_BTERM_RXCHBONDO{i}_S")
                } else {
                    format!("BRAM_BTERM_RXCHBOND0{i}_S")
                },
                &format!("IOI_BTERM_RXCHBONDO{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("IOI_BTERM_RXCHBONDI{i}"),
                &format!("BRAM_BTERM_RXCHBONDI{i}_S"),
            );
        }
        vrf.mark_merge_pip(tkn, "BRAM_BTERM_CLKOUTEAST0_S", "IOI_BTERM_CLKOUTEAST0");
        vrf.mark_merge_pip(tkn, "BRAM_BTERM_CLKOUTWEST0_S", "IOI_BTERM_CLKOUTWEST0");
        vrf.mark_merge_pip(tkn, "IOI_BTERM_CLKOUT_EW0", "BRAM_BTERM_CLKOUT_EW0_S");
    }
    for bt in ['B', 'T'] {
        let tkn = &format!("REG_{bt}_{bt}TERM");
        for i in 0..5 {
            vrf.mark_merge_pip(
                tkn,
                &format!("REG{bt}_{bt}TERM_ALTGTP_RCALINEAST{i}"),
                &format!("REG{bt}_{bt}TERM_GTP_RCALOUTEAST{i}"),
            );
        }
        for i in 0..5 {
            vrf.mark_merge_pip(
                tkn,
                &format!("REG{bt}_{bt}TERM_GTP_RXCHBONDO{i}"),
                &format!("REG{bt}_{bt}TERM_ALTGTP_RXCHBONDI{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("REG{bt}_{bt}TERM_ALTGTP_RXCHBONDO{i}"),
                &format!("REG{bt}_{bt}TERM_GTP_RXCHBONDI{i}"),
            );
        }
        vrf.mark_merge_pip(
            tkn,
            &format!("REG{bt}_{bt}TERM_GTP_CLKINWEST0"),
            &format!("REG{bt}_{bt}TERM_ALTGTP_CLKOUTEW0"),
        );
        vrf.mark_merge_pip(
            tkn,
            &format!("REG{bt}_{bt}TERM_ALTGTP_CLKINEAST0"),
            &format!("REG{bt}_{bt}TERM_GTP_CLKOUTEW0"),
        );
    }
    for (tkn, suffix) in [
        ("LIOI", "_ILOGIC"),
        ("RIOI", "_ILOGIC"),
        ("BIOI_INNER", ""),
        ("BIOI_OUTER", ""),
        ("TIOI_INNER", ""),
        ("TIOI_OUTER", ""),
    ] {
        for (pin, opin) in [("CFB0", "CFB"), ("CFB1", "CFB1"), ("DFB", "DFB")] {
            vrf.mark_merge_pip(
                tkn,
                &format!("{tkn}_{opin}_S{suffix}"),
                &format!("{pin}_ILOGIC_SITE_S"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("{tkn}_{opin}_M{suffix}"),
                &format!("{pin}_ILOGIC_SITE"),
            );
        }
        let tknalt = if tkn == "TIOI_OUTER" {
            "TIOI_UPPER"
        } else {
            tkn
        };
        vrf.mark_merge_pip(tkn, &format!("{tknalt}_OUTP"), "OUTP_IODELAY_SITE");
        vrf.mark_merge_pip(tkn, &format!("{tknalt}_OUTN"), "OUTN_IODELAY_SITE");
    }
    for (tkn, pref, suf, base, spref, iobpref) in [
        (
            "IOI_LTERM_LOWER_BOT",
            "IOI_LTERM_BOT",
            "_E",
            0,
            "IOI_LTERM",
            "LTERM_IOB_IBUF",
        ),
        (
            "IOI_LTERM_LOWER_TOP",
            "IOI_LTERM_TOP",
            "_E",
            1,
            "IOI_LTERM",
            "LTERM_IOB_IBUF",
        ),
        (
            "IOI_LTERM_UPPER_BOT",
            "IOI_LTERM_BOT",
            "_E",
            0,
            "IOI_LTERM",
            "LTERM_IOB_IBUF",
        ),
        (
            "IOI_LTERM_UPPER_TOP",
            "IOI_LTERM_TOP",
            "_E",
            1,
            "IOI_LTERM",
            "LTERM_IOB_IBUF",
        ),
        (
            "IOI_RTERM_LOWER_BOT",
            "IOI_RTERM_BOT",
            "_W",
            1,
            "IOI_RTERM",
            "RTERM_IOB_IBUF",
        ),
        (
            "IOI_RTERM_LOWER_TOP",
            "IOI_RTERM_TOP",
            "_W",
            0,
            "IOI_RTERM",
            "RTERM_IOB_IBUF",
        ),
        (
            "IOI_RTERM_UPPER_BOT",
            "IOI_RTERM_BOT",
            "_W",
            1,
            "IOI_RTERM",
            "RTERM_IOB_IBUF",
        ),
        (
            "IOI_RTERM_UPPER_TOP",
            "IOI_RTERM_TOP",
            "_W",
            0,
            "IOI_RTERM",
            "RTERM_IOB_IBUF",
        ),
        (
            "IOI_BTERM_REGB",
            "BTERM_CLB",
            "_N",
            2,
            "BTERM_CLB",
            "BTERM_IOIBOT_IBUF",
        ),
        (
            "IOI_BTERM_REGB",
            "BTERM_CLB",
            "_N",
            3,
            "BTERM_CLB",
            "BTERM_IOIUP_IBUF",
        ),
    ] {
        for i in 0..2 {
            let ii = base * 2 + i;
            for pin in ["CFB", "CFB1_", "DFB"] {
                vrf.mark_merge_pip(
                    tkn,
                    &format!("{pref}_{pin}{ii}"),
                    &format!("{pref}_{pin}{ii}{suf}"),
                );
            }
            vrf.mark_merge_pip(
                tkn,
                &format!("{spref}_CLKPIN{ii}"),
                &format!("{iobpref}{i}"),
            );
        }
        for pin in ["DQSP", "DQSN"] {
            vrf.mark_merge_pip(
                tkn,
                &format!("{pref}_{pin}{base}"),
                &format!("{pref}_{pin}{base}{suf}"),
            );
        }
    }
    for pin in ["CFB", "CFB1", "DFB"] {
        for i in [1, 2] {
            for ms in ['M', 'S'] {
                vrf.mark_merge_pip(
                    "IOI_TTERM_REGT",
                    &format!("IOI_REGT_{pin}_{ms}{i}"),
                    &format!("IOI_REGT_{pin}_{ms}{i}_S"),
                );
            }
        }
    }
    for pin in ["DQSP", "DQSN"] {
        for i in 0..2 {
            vrf.mark_merge_pip(
                "IOI_TTERM_REGT",
                &format!("IOI_REGT_{pin}{i}"),
                &format!("IOI_REGT_{pin}{i}_S"),
            );
        }
    }
    for i in 0..2 {
        vrf.mark_merge_pip(
            "IOI_TTERM_REGT",
            &format!("IOI_REGT_CLKPIN{i}"),
            &format!("TTERM_IOIUP_IBUF{i}"),
        );
        vrf.mark_merge_pip(
            "IOI_TTERM_REGT",
            &format!("IOI_REGT_CLKPIN{ii}", ii = i + 2),
            &format!("TTERM_IOIBOT_IBUF{i}"),
        );
    }

    for tkn in ["CMT_DCM_BOT", "CMT_DCM2_BOT", "CMT_DCM_TOP", "CMT_DCM2_TOP"] {
        for (i, pin) in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
            "CONCUR",
        ]
        .into_iter()
        .enumerate()
        {
            for j in [1, 2] {
                vrf.mark_merge_pip(tkn, &format!("DCM{j}_{pin}_TEST"), &format!("DCM{j}_{pin}"));
                vrf.mark_merge_pip(tkn, &format!("DCM{j}_CLKOUT{i}"), &format!("DCM{j}_{pin}"));
            }
        }
        for &crd in rd.tiles_by_kind_name(tkn) {
            for j in [1, 2] {
                vrf.merge_node(
                    RawWireCoord {
                        crd,
                        wire: &format!("DCM{j}_CLKIN_TOPLL"),
                    },
                    RawWireCoord {
                        crd,
                        wire: &format!("DCM{j}_CLKIN"),
                    },
                );
                vrf.merge_node(
                    RawWireCoord {
                        crd,
                        wire: &format!("DCM{j}_CLKFB_TOPLL"),
                    },
                    RawWireCoord {
                        crd,
                        wire: &format!("DCM{j}_CLKFB"),
                    },
                );
            }
        }
    }

    for tkn in [
        "CMT_PLL_BOT",
        "CMT_PLL1_BOT",
        "CMT_PLL2_BOT",
        "CMT_PLL3_BOT",
        "CMT_PLL_TOP",
        "CMT_PLL2_TOP",
        "CMT_PLL3_TOP",
    ] {
        for (wt, wf) in [
            ("PLLCASC_CLKOUT0", "CMT_PLL_CLKOUT0"),
            ("PLLCASC_CLKOUT1", "CMT_PLL_CLKOUT1"),
            ("CMT_PLL_LOCKED", "PLL_LOCKED"),
        ] {
            vrf.mark_merge_pip(tkn, wt, wf);
        }
    }
    vrf.inject_tcls_pip(
        tcls::CMT_PLL,
        TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKFB),
        TileWireCoord::new_idx(1, wires::OUT_PLL_CLKOUT[0]),
    );

    for tcid in [
        tcls::PLL_BUFPLL_OUT0_S,
        tcls::PLL_BUFPLL_OUT0_N,
        tcls::PLL_BUFPLL_OUT1_S,
        tcls::PLL_BUFPLL_OUT1_N,
        tcls::PLL_BUFPLL_S,
        tcls::PLL_BUFPLL_N,
    ] {
        let Some(BelInfo::SwitchBox(sb)) = endev.edev.db[tcid].bels.get(bslots::CMT_BUF) else {
            continue;
        };
        let mut skip_bufs = vec![];
        for item in &sb.items {
            match item {
                SwitchBoxItem::ProgBuf(buf) => {
                    if wires::OUT_PLL_CLKOUT.contains(buf.src.wire) {
                        continue;
                    }
                    skip_bufs.push((buf.dst, buf.src));
                    vrf.skip_tcls_pip(tcid, buf.dst, buf.src.tw);
                }
                SwitchBoxItem::PermaBuf(buf) => {
                    if buf.src.wire == wires::OUT_PLL_LOCKED {
                        continue;
                    }
                    skip_bufs.push((buf.dst, buf.src));
                    vrf.skip_tcls_pip(tcid, buf.dst, buf.src.tw);
                }
                _ => unreachable!(),
            }
        }
        for &tcrd in &endev.edev.tile_index[tcid] {
            for &(dst, src) in &skip_bufs {
                vrf.alias_wire(
                    endev.edev.resolve_tile_wire(tcrd, dst).unwrap(),
                    endev.edev.resolve_tile_wire(tcrd, src.tw).unwrap(),
                );
            }
        }
    }

    for &tcrd in &endev.edev.tile_index[tcls::CMT_DCM] {
        vrf.alias_wire(
            tcrd.wire(wires::OMUX_PLL_SKEWCLKIN1_BUF),
            tcrd.delta(0, 16).wire(wires::OMUX_PLL_SKEWCLKIN1),
        );
        vrf.alias_wire(
            tcrd.wire(wires::OMUX_PLL_SKEWCLKIN2_BUF),
            tcrd.delta(0, 16).wire(wires::OMUX_PLL_SKEWCLKIN2),
        );
    }
    vrf.skip_tcls_pip(
        tcls::CMT_DCM,
        TileWireCoord::new_idx(1, wires::OMUX_PLL_SKEWCLKIN1_BUF),
        TileWireCoord::new_idx(2, wires::OMUX_PLL_SKEWCLKIN1),
    );
    vrf.skip_tcls_pip(
        tcls::CMT_DCM,
        TileWireCoord::new_idx(1, wires::OMUX_PLL_SKEWCLKIN2_BUF),
        TileWireCoord::new_idx(2, wires::OMUX_PLL_SKEWCLKIN2),
    );

    for (tcid, ci) in [
        (tcls::CLK_W, 1),
        (tcls::CLK_W, 2),
        (tcls::CLK_E, 1),
        (tcls::CLK_E, 2),
        (tcls::CLK_S, 1),
        (tcls::CLK_S, 3),
        (tcls::CLK_N, 0),
        (tcls::CLK_N, 2),
    ] {
        for i in 0..4 {
            vrf.skip_alt_pip(tcid, TileWireCoord::new_idx(ci, wires::IMUX_BUFIO2FB[i]));
        }
    }

    // do you have ANY IDEA just how much I hate spartan6?
    for (tcid, ci) in [(tcls::CLK_W, 1), (tcls::CLK_E, 2)] {
        for i in [1, 3] {
            let dst = TileWireCoord::new_idx(ci, wires::IMUX_BUFIO2_I[i]);
            let src_actual = TileWireCoord::new_idx(ci, wires::GTPCLK[i]);
            let src_fake = TileWireCoord::new_idx(ci, wires::GTPCLK[i ^ 1]);
            vrf.skip_tcls_pip(tcid, dst, src_actual);
            vrf.inject_tcls_pip(tcid, dst, src_fake);
        }
    }

    for &tcrd in &endev.edev.tile_index[tcls::HCLK_ROW] {
        let bcrd = tcrd.bel(bslots::HCLK_ROW);
        for we in ['W', 'E'] {
            for i in 0..16 {
                vrf.merge_node(
                    vrf.bel_wire_far(bcrd, &format!("I_{we}{i}")),
                    vrf.bel_wire_far(bcrd, &format!("O_{we}{i}")),
                );
            }
        }
    }

    for tcid in [tcls::CLK_W, tcls::CLK_E] {
        vrf.force_bel_output(
            tcid,
            bslots::BUFPLL,
            bcls::BUFPLL::LOCK[0],
            TileWireCoord::new_idx(2, wires::OUT[0]),
        );
        vrf.force_bel_output(
            tcid,
            bslots::BUFPLL,
            bcls::BUFPLL::LOCK[1],
            TileWireCoord::new_idx(2, wires::OUT[1]),
        );
    }
    for (tcid, ci) in [(tcls::CLK_S, 1), (tcls::CLK_N, 0)] {
        vrf.force_bel_output(
            tcid,
            bslots::BUFPLL,
            bcls::BUFPLL::LOCK[0],
            TileWireCoord::new_idx(2, wires::OUT[18]),
        );
        vrf.force_bel_output(
            tcid,
            bslots::BUFPLL,
            bcls::BUFPLL::LOCK[1],
            TileWireCoord::new_idx(2, wires::OUT[19]),
        );
        for i in 0..2 {
            vrf.skip_bel_input(tcid, bslots::BUFPLL, bcls::BUFPLL::PLLIN_CMT[i]);
            vrf.skip_bel_input(tcid, bslots::BUFPLL, bcls::BUFPLL::LOCKED[i]);
        }
        for i in 0..2 {
            for j in 0..6 {
                vrf.skip_tcls_pip(
                    tcid,
                    TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_PLLIN[i]),
                    TileWireCoord::new_idx(
                        ci,
                        if tcid == tcls::CLK_N {
                            wires::CMT_BUFPLL_V_CLKOUT_S[j]
                        } else {
                            wires::CMT_BUFPLL_V_CLKOUT_N[j]
                        },
                    ),
                );
            }
            for j in 0..3 {
                vrf.skip_tcls_pip(
                    tcid,
                    TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_LOCKED[i]),
                    TileWireCoord::new_idx(
                        ci,
                        if tcid == tcls::CLK_N {
                            wires::CMT_BUFPLL_V_LOCKED_S[j]
                        } else {
                            wires::CMT_BUFPLL_V_LOCKED_N[j]
                        },
                    ),
                );
            }
        }
    }
    vrf.prep_int_wires();
    vrf.handle_int();
    for (tcrd, tile) in endev.ngrid.egrid.tiles() {
        let tcls = &endev.ngrid.egrid.db[tile.class];
        for slot in tcls.bels.ids() {
            verify_bel(endev, &mut vrf, tcrd.bel(slot));
        }
    }
    verify_extra(endev, &mut vrf);
    vrf.finish();
}
