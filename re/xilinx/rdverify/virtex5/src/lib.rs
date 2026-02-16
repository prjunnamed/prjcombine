use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::WireSlotIdExt,
    grid::{BelCoord, CellCoord, DieId, RowId},
};
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{RawWireCoord, SitePinDir, Verifier};
use prjcombine_virtex4::defs::{
    self, bslots,
    virtex5::{tcls, wires},
};

fn verify_slice(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let kind = if bel.info.pins.contains_key("WE") {
        "SLICEM"
    } else {
        "SLICEL"
    };
    vrf.verify_legacy_bel(
        bel,
        kind,
        &[("CIN", SitePinDir::In), ("COUT", SitePinDir::Out)],
        &[],
    );
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.slot) {
        vrf.claim_net(&[bel.wire("CIN"), obel.wire_far("COUT")]);
        vrf.claim_pip(obel.wire_far("COUT"), obel.wire("COUT"));
    } else {
        vrf.claim_net(&[bel.wire("CIN")]);
    }
    vrf.claim_net(&[bel.wire("COUT")]);
    vrf.claim_pip(bel.wire_far("AMUX"), bel.wire_far("AX"));
    vrf.claim_pip(bel.wire_far("BMUX"), bel.wire_far("BX"));
    vrf.claim_pip(bel.wire_far("CMUX"), bel.wire_far("CX"));
    vrf.claim_pip(bel.wire_far("DMUX"), bel.wire_far("DX"));
}

fn verify_bram(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(
        bel,
        "RAMBFIFO36",
        &[
            ("CASCADEINLATA", SitePinDir::In),
            ("CASCADEINLATB", SitePinDir::In),
            ("CASCADEINREGA", SitePinDir::In),
            ("CASCADEINREGB", SitePinDir::In),
            ("CASCADEOUTLATA", SitePinDir::Out),
            ("CASCADEOUTLATB", SitePinDir::Out),
            ("CASCADEOUTREGA", SitePinDir::Out),
            ("CASCADEOUTREGB", SitePinDir::Out),
        ],
        &[],
    );
    for (ipin, opin) in [
        ("CASCADEINLATA", "CASCADEOUTLATA"),
        ("CASCADEINLATB", "CASCADEOUTLATB"),
        ("CASCADEINREGA", "CASCADEOUTREGA"),
        ("CASCADEINREGB", "CASCADEOUTREGB"),
    ] {
        vrf.claim_net(&[bel.wire(opin)]);
        vrf.claim_net(&[bel.wire(ipin)]);
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bel.slot) {
            vrf.verify_net(&[bel.wire_far(ipin), obel.wire(opin)]);
            vrf.claim_pip(bel.wire(ipin), bel.wire_far(ipin));
        }
    }
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
    vrf.verify_legacy_bel(bel, "DSP48E", &pins, &[]);
}

fn verify_sysmon(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("SYSMON")
        .ipad("VP", 1)
        .ipad("VN", 2);
    for i in 0..16 {
        let vauxp = &format!("VAUXP{i}");
        let vauxn = &format!("VAUXN{i}");
        bel = bel.extra_in(vauxp).extra_in(vauxn);
        let Some((iop, _)) = endev.edev.get_sysmon_vaux(bcrd.cell, i) else {
            continue;
        };
        bel.claim_net(&[bel.wire(vauxp)]);
        bel.claim_net(&[bel.wire(vauxn)]);
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

fn verify_ilogic(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::ILOGIC.index_of(bel.slot).unwrap();
    let pins = [
        ("TFB", SitePinDir::In),
        ("OFB", SitePinDir::In),
        ("D", SitePinDir::In),
        ("DDLY", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "ILOGIC", &pins, &["CLKPAD"]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let obel = vrf.find_bel_sibling(bel, bslots::IODELAY[idx]);
    vrf.claim_pip(bel.wire("DDLY"), obel.wire("DATAOUT"));

    vrf.claim_pip(bel.wire("D"), bel.wire("I_IOB"));

    let obel = vrf.find_bel_sibling(bel, bslots::OLOGIC[idx]);
    vrf.claim_pip(bel.wire("OFB"), obel.wire("OQ"));
    vrf.claim_pip(bel.wire("TFB"), obel.wire("TQ"));

    if bel.slot == bslots::ILOGIC[0] {
        let obel = vrf.find_bel_sibling(bel, bslots::ILOGIC[1]);
        vrf.claim_pip(bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }

    if idx == 1 {
        let is_bufio = matches!(bcrd.row.to_idx() % 20, 8..12);
        let dy = bcrd.row - endev.edev.chips[bcrd.die].row_bufg();
        let is_giob = matches!(dy, -30..-20 | 20..30);
        if is_giob || is_bufio {
            vrf.claim_pip(bel.wire("CLKPAD"), bel.wire("O"));
        }
    }
}

fn verify_ologic(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::OLOGIC.index_of(bel.slot).unwrap();
    let pins = [
        ("OQ", SitePinDir::Out),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "OLOGIC", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let obel = vrf.find_bel_sibling(bel, bslots::IODELAY[idx]);
    vrf.claim_pip(bel.wire("O_IOB"), bel.wire("OQ"));
    vrf.claim_pip(bel.wire("O_IOB"), obel.wire("DATAOUT"));
    vrf.claim_pip(bel.wire("T_IOB"), bel.wire("TQ"));

    if bel.slot == bslots::OLOGIC[1] {
        let obel = vrf.find_bel_sibling(bel, bslots::OLOGIC[0]);
        vrf.claim_pip(bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_iodelay(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::IODELAY.index_of(bel.slot).unwrap();
    let pins = [
        ("IDATAIN", SitePinDir::In),
        ("ODATAIN", SitePinDir::In),
        ("T", SitePinDir::In),
        ("DATAOUT", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "IODELAY", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    let obel = vrf.find_bel_sibling(bel, bslots::ILOGIC[idx]);
    vrf.claim_pip(bel.wire("IDATAIN"), obel.wire("I_IOB"));

    let obel = vrf.find_bel_sibling(bel, bslots::OLOGIC[idx]);
    vrf.claim_pip(bel.wire("ODATAIN"), obel.wire("OQ"));
    vrf.claim_pip(bel.wire("T"), obel.wire("TQ"));
}

fn verify_iob(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let idx = bslots::IOB.index_of(bel.slot).unwrap();
    let kind = if bel.slot == bslots::IOB[1] {
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
    let obel = vrf.find_bel_sibling(bel, bslots::OLOGIC[idx]);
    vrf.verify_net(&[bel.wire("O"), obel.wire("O_IOB")]);
    vrf.verify_net(&[bel.wire("T"), obel.wire("T_IOB")]);
    let obel = vrf.find_bel_sibling(bel, bslots::ILOGIC[idx]);
    vrf.verify_net(&[bel.wire("I"), obel.wire("I_IOB")]);
    let obel = vrf.find_bel_sibling(bel, bslots::IOB[idx ^ 1]);
    vrf.claim_pip(bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
    if kind == "IOBS" {
        vrf.claim_pip(bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
    }
}

fn verify_pll(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    vrf.verify_legacy_bel(bel, "PLL_ADV", &[], &["TEST_CLKIN"]);
    vrf.claim_pip(bel.wire("TEST_CLKIN"), bel.wire("CLKIN1"));
}

fn verify_gt(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("RXP0", SitePinDir::In),
        ("RXN0", SitePinDir::In),
        ("RXP1", SitePinDir::In),
        ("RXN1", SitePinDir::In),
        ("TXP0", SitePinDir::Out),
        ("TXN0", SitePinDir::Out),
        ("TXP1", SitePinDir::Out),
        ("TXN1", SitePinDir::Out),
        ("CLKIN", SitePinDir::In),
    ];
    let kind = match bel.slot {
        bslots::GTP_DUAL => "GTP_DUAL",
        bslots::GTX_DUAL => "GTX_DUAL",
        _ => unreachable!(),
    };
    vrf.verify_legacy_bel(bel, kind, &pins, &["GREFCLK"]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }

    for (pin, slot) in [
        ("RXP0", bslots::IPAD_RXP[0]),
        ("RXN0", bslots::IPAD_RXN[0]),
        ("RXP1", bslots::IPAD_RXP[1]),
        ("RXN1", bslots::IPAD_RXN[1]),
    ] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.wire(pin), obel.wire("O"));
    }
    for (pin, slot) in [
        ("TXP0", bslots::OPAD_TXP[0]),
        ("TXN0", bslots::OPAD_TXN[0]),
        ("TXP1", bslots::OPAD_TXP[1]),
        ("TXN1", bslots::OPAD_TXN[1]),
    ] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(obel.wire("I"), bel.wire(pin));
    }

    let obel = vrf.find_bel_sibling(bel, bslots::BUFDS[0]);
    vrf.claim_pip(bel.wire("CLKIN"), bel.wire("CLKOUT_NORTH_S"));
    vrf.claim_pip(bel.wire("CLKIN"), bel.wire("CLKOUT_SOUTH_N"));
    vrf.claim_pip(bel.wire("CLKIN"), bel.wire("GREFCLK"));
    vrf.claim_pip(bel.wire("CLKIN"), obel.wire("O"));
    vrf.claim_net(&[bel.wire("CLKOUT_NORTH")]);
    vrf.claim_pip(bel.wire("CLKOUT_NORTH"), bel.wire("CLKOUT_NORTH_S"));
    vrf.claim_pip(bel.wire("CLKOUT_NORTH"), obel.wire("O"));
    vrf.claim_net(&[bel.wire("CLKOUT_SOUTH")]);
    vrf.claim_pip(bel.wire("CLKOUT_SOUTH"), bel.wire("CLKOUT_SOUTH_N"));
    vrf.claim_pip(bel.wire("CLKOUT_SOUTH"), obel.wire("O"));
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -20, bel.slot) {
        vrf.verify_net(&[bel.wire("CLKOUT_NORTH_S"), obel.wire("CLKOUT_NORTH")]);
    } else {
        vrf.claim_net(&[bel.wire("CLKOUT_NORTH_S")]);
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 20, bel.slot) {
        vrf.verify_net(&[bel.wire("CLKOUT_SOUTH_N"), obel.wire("CLKOUT_SOUTH")]);
    } else {
        vrf.claim_net(&[bel.wire("CLKOUT_SOUTH_N")]);
    }
}

fn verify_bufds(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &mut vrf.get_legacy_bel(bcrd);
    let pins = [
        ("IP", SitePinDir::In),
        ("IN", SitePinDir::In),
        ("O", SitePinDir::Out),
    ];
    vrf.verify_legacy_bel(bel, "BUFDS", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_net(&[bel.wire(pin)]);
    }
    for (pin, slot) in [("IP", bslots::IPAD_CLKP[0]), ("IN", bslots::IPAD_CLKN[0])] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.wire(pin), obel.wire("O"));
    }
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

pub fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let slot_name = endev.edev.db.bel_slots.key(bcrd.slot);
    match bcrd.slot {
        bslots::INT
        | bslots::INTF_INT
        | bslots::INTF_TESTMUX
        | bslots::SPEC_INT
        | bslots::CLK_INT
        | bslots::HROW_INT
        | bslots::HCLK
        | bslots::HCLK_IO_INT
        | bslots::HCLK_CMT_DRP
        | bslots::MISC_CFG
        | bslots::BANK
        | bslots::GLOBAL => (),
        _ if bslots::SLICE.contains(bcrd.slot) => verify_slice(vrf, bcrd),
        _ if bslots::DSP.contains(bcrd.slot) => verify_dsp(vrf, bcrd),
        bslots::BRAM => verify_bram(vrf, bcrd),
        bslots::PMVBRAM => vrf.verify_bel(bcrd).commit(),
        bslots::EMAC => vrf.verify_bel(bcrd).kind("TEMAC").commit(),
        bslots::PCIE => {
            let bel = vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(&bel, "PCIE", &[], &[])
        }
        bslots::PPC => {
            let bel = vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(&bel, "PPC440", &[], &[])
        }

        _ if bslots::BUFGCTRL.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        _ if bslots::BSCAN.contains(bcrd.slot)
            || bslots::ICAP.contains(bcrd.slot)
            || bslots::PMV_CFG.contains(bcrd.slot) =>
        {
            vrf.verify_bel(bcrd).commit()
        }
        bslots::STARTUP
        | bslots::FRAME_ECC
        | bslots::DCIRESET
        | bslots::CAPTURE
        | bslots::USR_ACCESS
        | bslots::EFUSE_USR
        | bslots::KEY_CLEAR
        | bslots::JTAGPPC => vrf.verify_bel(bcrd).commit(),
        bslots::DCI => vrf.verify_bel(bcrd).kind("DCI").commit(),
        bslots::GLOBALSIG => vrf.verify_bel(bcrd).commit(),
        bslots::SYSMON => verify_sysmon(endev, vrf, bcrd),

        _ if bslots::ILOGIC.contains(bcrd.slot) => verify_ilogic(endev, vrf, bcrd),
        _ if bslots::OLOGIC.contains(bcrd.slot) => verify_ologic(vrf, bcrd),
        _ if bslots::IODELAY.contains(bcrd.slot) => verify_iodelay(vrf, bcrd),
        _ if bslots::IOB.contains(bcrd.slot) => verify_iob(vrf, bcrd),

        _ if bslots::DCM.contains(bcrd.slot) => {
            let bel = &mut vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "DCM_ADV", &[], &[]);
        }
        bslots::PLL => verify_pll(vrf, bcrd),

        bslots::GTX_DUAL | bslots::GTP_DUAL => verify_gt(vrf, bcrd),
        _ if bslots::BUFDS.contains(bcrd.slot) => verify_bufds(vrf, bcrd),
        _ if bslots::CRC32.contains(bcrd.slot) => {
            let bel = vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(&bel, "CRC32", &[], &[])
        }
        _ if bslots::CRC64.contains(bcrd.slot) => {
            let bel = vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(&bel, "CRC64", &[], &[])
        }
        _ if slot_name.starts_with("IPAD") => verify_ipad(vrf, bcrd),
        _ if slot_name.starts_with("OPAD") => verify_opad(vrf, bcrd),

        _ if bslots::BUFR.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        _ if bslots::BUFIO.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        bslots::IDELAYCTRL => vrf.verify_bel(bcrd).commit(),

        _ => println!("MEOW {}", bcrd.to_string(endev.edev.db)),
    }
}

pub fn verify_extra(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_BYP0");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_BYP1");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_BYP2");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_BYP3");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_BYP4");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_BYP5");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_BYP6");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_BYP7");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_CTRL0");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_CTRL1");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_CTRL2");
    vrf.kill_stub_out("CFG_PPC_DL_BUFS_CTRL3");
    if endev.edev.col_rgt.is_none() {
        let nnode = &endev.ngrid.tiles[&CellCoord::new(
            DieId::from_idx(0),
            endev.edev.chips.first().unwrap().columns.last_id().unwrap(),
            RowId::from_idx(0),
        )
        .tile(defs::tslots::INT)];
        let crd = vrf.xlat_tile(&nnode.names[RawTileId::from_idx(0)]).unwrap();
        vrf.claim_net(&[(RawWireCoord {
            crd,
            wire: "ER2BEG0",
        })]);
    }
    vrf.kill_stub_out_cond("IOI_BYP_INT_B0");
    vrf.kill_stub_out_cond("IOI_BYP_INT_B2");
    vrf.kill_stub_out_cond("IOI_BYP_INT_B3");
    vrf.kill_stub_out_cond("IOI_BYP_INT_B6");
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);

    vrf.mark_merge_pip("IOI", "IOI_BLOCK_OUTS_B0", "IOI_OCLKP_1");
    vrf.mark_merge_pip("IOI", "IOI_BLOCK_OUTS_B1", "IOI_OCLKDIV1");
    vrf.mark_merge_pip("IOI", "IOI_BLOCK_OUTS_B2", "IOI_OCLKP_0");
    vrf.mark_merge_pip("IOI", "IOI_BLOCK_OUTS_B3", "IOI_OCLKDIV0");

    for tkn in ["CLK_HROW", "CLK_HROW_MGT"] {
        for i in 0..5 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CLK_HROW_MGT_CLKV{i}"),
                &format!("CLK_HROW_MGT_CLK_P{i}"),
            );
            vrf.mark_merge_pip(
                tkn,
                &format!("CLK_HROW_MGT_CLKV{i}_LEFT"),
                &format!("CLK_HROW_MGT_CLK_P{i}_LEFT"),
            );
        }
    }
    for (i, (c, i0, i1)) in [
        (2, 3, 15),
        (2, 9, 21),
        (2, 27, 39),
        (2, 33, 45),
        (3, 3, 15),
        (3, 9, 21),
        (3, 27, 39),
        (3, 33, 45),
        (4, 3, 15),
        (4, 9, 21),
        (4, 27, 39),
        (4, 33, 45),
        (3, 11, 35),
        (3, 23, 47),
        (4, 11, 35),
        (4, 23, 47),
        (15, 3, 15),
        (15, 9, 21),
        (15, 27, 39),
        (15, 33, 45),
        (16, 3, 15),
        (16, 9, 21),
        (16, 27, 39),
        (16, 33, 45),
        (17, 3, 15),
        (17, 9, 21),
        (17, 27, 39),
        (17, 33, 45),
        (16, 11, 35),
        (16, 23, 47),
        (17, 11, 35),
        (17, 23, 47),
    ]
    .into_iter()
    .enumerate()
    {
        vrf.mark_merge_pip(
            "CFG_CENTER",
            &format!("CFG_CENTER_CKINT0_{i}"),
            &format!("CFG_CENTER_IMUX_B{i0}_{c}"),
        );
        vrf.mark_merge_pip(
            "CFG_CENTER",
            &format!("CFG_CENTER_CKINT1_{i}"),
            &format!("CFG_CENTER_IMUX_B{i1}_{c}"),
        );
    }

    for tkn in ["CLK_IOB_B", "CLK_IOB_T"] {
        for i in 0..10 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CLK_IOB_CLK_BUF{i}"),
                &format!("CLK_IOB_PAD_CLK{i}"),
            );
        }
    }

    for tkn in ["CMT_BOT", "CMT_TOP"] {
        vrf.mark_merge_pip(tkn, "CMT_DCM_0_SE_CLK_IN0", "CMT_CLK_B0_0");
        vrf.mark_merge_pip(tkn, "CMT_DCM_0_SE_CLK_IN1", "CMT_CLK_B1_0");
        vrf.mark_merge_pip(tkn, "CMT_DCM_0_SE_CLK_IN2", "CMT_IMUX_B6_0");
        vrf.mark_merge_pip(tkn, "CMT_PLL_SE_CLK_IN0", "CMT_CLK_B0_3");
        vrf.mark_merge_pip(tkn, "CMT_PLL_SE_CLK_IN1", "CMT_CLK_B0_4");
        vrf.mark_merge_pip(tkn, "CMT_DCM_1_SE_CLK_IN0", "CMT_CLK_B0_7");
        vrf.mark_merge_pip(tkn, "CMT_DCM_1_SE_CLK_IN1", "CMT_CLK_B1_7");
        vrf.mark_merge_pip(tkn, "CMT_DCM_1_SE_CLK_IN2", "CMT_IMUX_B12_7");
        vrf.mark_merge_pip(tkn, "CMT_PLL_CLKINFB_TEST", "CMT_PLL_CLKFBIN");
    }

    for tkn in ["HCLK_IOI_CENTER", "HCLK_CMT_IOI"] {
        vrf.kill_stub_in_cond_tk(tkn, "HCLK_IOI_VRCLK0");
        vrf.kill_stub_in_cond_tk(tkn, "HCLK_IOI_VRCLK1");
        vrf.kill_stub_in_cond_tk(tkn, "HCLK_IOI_VRCLK_S0");
        vrf.kill_stub_in_cond_tk(tkn, "HCLK_IOI_VRCLK_S1");
        vrf.kill_stub_in_cond_tk(tkn, "HCLK_IOI_VRCLK_N0");
        vrf.kill_stub_in_cond_tk(tkn, "HCLK_IOI_VRCLK_N1");
    }

    for co in 0..2 {
        for o in 0..10 {
            for i in 0..32 {
                vrf.skip_tcls_pip(
                    tcls::CLK_HROW,
                    wires::HCLK_ROW[o].cell(co),
                    wires::GCLK_BUF[i].cell(0),
                );
                vrf.inject_tcls_pip(
                    tcls::CLK_HROW,
                    wires::HCLK_ROW[o].cell(co),
                    wires::GCLK[i].cell(0),
                );
            }
        }
    }
    for i in 0..32 {
        vrf.skip_tcls_pip(
            tcls::CLK_HROW,
            wires::GCLK_BUF[i].cell(0),
            wires::GCLK[i].cell(0),
        );
    }

    for tcid in [
        tcls::CLK_IOB_S,
        tcls::CLK_IOB_N,
        tcls::CLK_CMT_S,
        tcls::CLK_CMT_N,
        tcls::CLK_MGT_S,
        tcls::CLK_MGT_N,
    ] {
        for i in 0..32 {
            for j in 0..10 {
                vrf.skip_tcls_pip(
                    tcid,
                    wires::IMUX_BUFG_O[i].cell(0),
                    wires::MGT_BUF[j].cell(0),
                );
                vrf.inject_tcls_pip(
                    tcid,
                    wires::IMUX_BUFG_O[i].cell(0),
                    wires::MGT_ROW_I[j % 5].cell((j / 5) * 10),
                );
            }
        }
        for i in 0..5 {
            vrf.skip_tcls_pip(tcid, wires::MGT_BUF[i].cell(0), wires::MGT_ROW_I[i].cell(0));
            vrf.skip_tcls_pip(
                tcid,
                wires::MGT_BUF[i + 5].cell(0),
                wires::MGT_ROW_I[i].cell(10),
            );
        }
    }

    // fake pips (actually going through CLKIN2)
    for i in 5..10 {
        vrf.inject_tcls_pip(
            tcls::CMT,
            wires::IMUX_PLL_CLKIN1.cell(0),
            wires::HCLK_CMT[i].cell(0),
        );
        vrf.inject_tcls_pip(
            tcls::CMT,
            wires::IMUX_PLL_CLKIN1.cell(0),
            wires::GIOB_CMT[i].cell(0),
        );
    }
    vrf.inject_tcls_pip(
        tcls::CMT,
        wires::IMUX_PLL_CLKIN1.cell(0),
        wires::OUT_PLL_CLKFBDCM.cell(0),
    );
    vrf.skip_tcls_pip(
        tcls::CMT,
        wires::IMUX_PLL_CLKIN2.cell(0),
        wires::OUT_PLL_CLKFBDCM.cell(0),
    );

    for i in 0..2 {
        vrf.alias_wire_slot(wires::IMUX_IO_ICLK_OPTINV[i], wires::IMUX_IO_ICLK[i]);
    }

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
