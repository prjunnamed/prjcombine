use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::WireSlotIdExt,
    grid::{BelCoord, CellCoord, DieId, RowId},
};
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{RawWireCoord, Verifier};
use prjcombine_virtex4::{
    chip::ColumnKind,
    defs::{
        self, bcls, bslots,
        virtex5::{tcls, wires},
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
    if let Some(obel) = edev.bel_carry_prev(bcrd) {
        bel.claim_net(&[bel.wire("CIN"), bel.bel_wire_far(obel, "COUT")]);
        bel.claim_pip(bel.bel_wire_far(obel, "COUT"), bel.bel_wire(obel, "COUT"));
    } else {
        bel.claim_net(&[bel.wire("CIN")]);
    }
    bel.claim_pip(bel.wire_far("AMUX"), bel.wire_far("AX"));
    bel.claim_pip(bel.wire_far("BMUX"), bel.wire_far("BX"));
    bel.claim_pip(bel.wire_far("CMUX"), bel.wire_far("CX"));
    bel.claim_pip(bel.wire_far("DMUX"), bel.wire_far("DX"));
    bel.commit();
}

fn verify_bram(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("RAMBFIFO36")
        .rename_in(bcls::BRAM_V5::CLKAL, "CLKARDCLKL")
        .rename_in(bcls::BRAM_V5::CLKAU, "CLKARDCLKU")
        .rename_in(bcls::BRAM_V5::CLKBL, "CLKBWRCLKL")
        .rename_in(bcls::BRAM_V5::CLKBU, "CLKBWRCLKU")
        .rename_in(bcls::BRAM_V5::ENAL, "ENARDENL")
        .rename_in(bcls::BRAM_V5::ENBL, "ENBWRENL")
        .rename_in(bcls::BRAM_V5::SSRAL, "SSRARSTL")
        .rename_in(bcls::BRAM_V5::REGCLKAL, "REGCLKARDRCLKL")
        .rename_in(bcls::BRAM_V5::REGCLKAU, "REGCLKARDRCLKU")
        .rename_in(bcls::BRAM_V5::REGCLKBL, "REGCLKBWRRCLKL")
        .rename_in(bcls::BRAM_V5::REGCLKBU, "REGCLKBWRRCLKU");
    for i in 0..16 {
        bel = bel
            .rename_in(bcls::BRAM_V5::DIAL[i], format!("DIADIL{i}"))
            .rename_in(bcls::BRAM_V5::DIAU[i], format!("DIADIU{i}"))
            .rename_in(bcls::BRAM_V5::DIBL[i], format!("DIBDIL{i}"))
            .rename_in(bcls::BRAM_V5::DIBU[i], format!("DIBDIU{i}"))
            .rename_out(bcls::BRAM_V5::DOAL[i], format!("DOADOL{i}"))
            .rename_out(bcls::BRAM_V5::DOAU[i], format!("DOADOU{i}"))
            .rename_out(bcls::BRAM_V5::DOBL[i], format!("DOBDOL{i}"))
            .rename_out(bcls::BRAM_V5::DOBU[i], format!("DOBDOU{i}"));
    }
    for i in 0..2 {
        bel = bel
            .rename_in(bcls::BRAM_V5::DIPAL[i], format!("DIPADIPL{i}"))
            .rename_in(bcls::BRAM_V5::DIPAU[i], format!("DIPADIPU{i}"))
            .rename_in(bcls::BRAM_V5::DIPBL[i], format!("DIPBDIPL{i}"))
            .rename_in(bcls::BRAM_V5::DIPBU[i], format!("DIPBDIPU{i}"))
            .rename_out(bcls::BRAM_V5::DOPAL[i], format!("DOPADOPL{i}"))
            .rename_out(bcls::BRAM_V5::DOPAU[i], format!("DOPADOPU{i}"))
            .rename_out(bcls::BRAM_V5::DOPBL[i], format!("DOPBDOPL{i}"))
            .rename_out(bcls::BRAM_V5::DOPBU[i], format!("DOPBDOPU{i}"));
    }
    for (ipin, opin) in [
        ("CASCADEINLATA", "CASCADEOUTLATA"),
        ("CASCADEINLATB", "CASCADEOUTLATB"),
        ("CASCADEINREGA", "CASCADEOUTREGA"),
        ("CASCADEINREGB", "CASCADEOUTREGB"),
    ] {
        bel = bel.extra_in_claim(ipin).extra_out_claim(opin);
        if let Some(obel) = edev.bel_carry_prev(bcrd) {
            bel.verify_net(&[bel.wire_far(ipin), bel.bel_wire(obel, opin)]);
            bel.claim_pip(bel.wire(ipin), bel.wire_far(ipin));
        }
    }
    bel.commit();
}

fn verify_dsp(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("DSP48E");
    for (name_in, name_out, num) in [
        ("ACIN", "ACOUT", 30),
        ("BCIN", "BCOUT", 18),
        ("PCIN", "PCOUT", 48),
        ("MULTSIGNIN", "MULTSIGNOUT", 0),
        ("CARRYCASCIN", "CARRYCASCOUT", 0),
    ] {
        for i in 0..(num.max(1)) {
            let ipin = &if num == 0 {
                name_in.to_string()
            } else {
                format!("{name_in}{i}")
            };
            let opin = &if num == 0 {
                name_out.to_string()
            } else {
                format!("{name_out}{i}")
            };
            bel = bel.extra_in(ipin).extra_out_claim(opin);
            if bcrd.slot == bslots::DSP[0] {
                if let Some(obel) = edev.bel_carry_prev(bcrd) {
                    bel.claim_net(&[bel.wire(ipin), bel.bel_wire_far(obel, opin)]);
                    bel.claim_pip(bel.bel_wire_far(obel, opin), bel.bel_wire(obel, opin));
                } else {
                    bel.claim_net(&[bel.wire(ipin)]);
                }
            } else {
                bel.claim_net(&[bel.wire(ipin)]);
                let obel = bcrd.bel(bslots::DSP[0]);
                bel.claim_pip(bel.wire(ipin), bel.bel_wire(obel, opin));
            }
        }
    }
    bel.commit();
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
        bel = bel.extra_in(vauxp).extra_in(vauxn);
        let Some((iop, _)) = edev.get_sysmon_vaux(bcrd.cell, i) else {
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

fn verify_ilogic(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::ILOGIC.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("ILOGIC")
        .skip_out(bcls::ILOGIC_V4::CLKPAD)
        .extra_in_claim("TFB")
        .extra_in_claim("OFB")
        .extra_in_claim("D")
        .extra_in_claim("DDLY")
        .extra_in_claim("SHIFTIN1")
        .extra_in_claim("SHIFTIN2")
        .extra_in_claim("OCLK")
        .extra_out_claim("SHIFTOUT1")
        .extra_out_claim("SHIFTOUT2");

    let obel = bcrd.bel(bslots::IODELAY[idx]);
    bel.claim_pip(bel.wire("DDLY"), bel.bel_wire(obel, "DATAOUT"));

    bel.claim_pip(bel.wire("D"), bel.wire("I_IOB"));

    let obel = bcrd.bel(bslots::OLOGIC[idx]);
    bel.claim_pip(bel.wire("OFB"), bel.bel_wire(obel, "OQ"));
    bel.claim_pip(bel.wire("TFB"), bel.bel_wire(obel, "TQ"));
    bel.claim_pip(bel.wire("OCLK"), bel.bel_wire_far(obel, "CLK"));

    if bcrd.slot == bslots::ILOGIC[0] {
        let obel = bcrd.bel(bslots::ILOGIC[1]);
        bel.claim_pip(bel.wire("SHIFTIN1"), bel.bel_wire(obel, "SHIFTOUT1"));
        bel.claim_pip(bel.wire("SHIFTIN2"), bel.bel_wire(obel, "SHIFTOUT2"));
    }

    if idx == 1 {
        let is_bufio = matches!(bcrd.row.to_idx() % 20, 8..12);
        let dy = bcrd.row - edev.chips[bcrd.die].row_bufg();
        let is_giob = matches!(dy, -30..-20 | 20..30);
        if is_giob || is_bufio {
            bel.claim_pip(bel.wire("CLKPAD"), bel.wire("O"));
        }
    }

    bel.commit();
}

fn verify_ologic(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::OLOGIC.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("OLOGIC")
        .extra_out_claim("OQ")
        .extra_in_claim("SHIFTIN1")
        .extra_in_claim("SHIFTIN2")
        .extra_out_claim("SHIFTOUT1")
        .extra_out_claim("SHIFTOUT2");

    let obel = bcrd.bel(bslots::IODELAY[idx]);
    bel.claim_pip(bel.wire("O_IOB"), bel.wire("OQ"));
    bel.claim_pip(bel.wire("O_IOB"), bel.bel_wire(obel, "DATAOUT"));
    bel.claim_pip(bel.wire("T_IOB"), bel.wire("TQ"));

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
        .kind("IODELAY")
        .extra_in_claim("IDATAIN")
        .extra_in_claim("ODATAIN")
        .extra_in_claim("T")
        .extra_in_claim("C")
        .extra_out_claim("DATAOUT");

    let obel = bcrd.bel(bslots::ILOGIC[idx]);
    bel.claim_pip(bel.wire("IDATAIN"), bel.bel_wire(obel, "I_IOB"));
    bel.claim_pip(bel.wire("C"), bel.bel_wire_far(obel, "CLKDIV"));

    let obel = bcrd.bel(bslots::OLOGIC[idx]);
    bel.claim_pip(bel.wire("ODATAIN"), bel.bel_wire(obel, "OQ"));
    bel.claim_pip(bel.wire("T"), bel.bel_wire(obel, "TQ"));

    bel.commit();
}

fn verify_iob(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IOB.index_of(bcrd.slot).unwrap();
    let kind = if bcrd.slot == bslots::IOB[1] {
        "IOBM"
    } else {
        "IOBS"
    };
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(kind)
        .extra_out_claim("I")
        .extra_in_claim("O")
        .extra_in_claim("T")
        .extra_out_claim("PADOUT")
        .extra_in_claim("DIFFI_IN")
        .extra_out_claim("DIFFO_OUT")
        .extra_in_claim("DIFFO_IN");
    let obel = bcrd.bel(bslots::OLOGIC[idx]);
    bel.verify_net(&[bel.wire("O"), bel.bel_wire(obel, "O_IOB")]);
    bel.verify_net(&[bel.wire("T"), bel.bel_wire(obel, "T_IOB")]);
    let obel = bcrd.bel(bslots::ILOGIC[idx]);
    bel.verify_net(&[bel.wire("I"), bel.bel_wire(obel, "I_IOB")]);
    let obel = bcrd.bel(bslots::IOB[idx ^ 1]);
    bel.claim_pip(bel.wire("DIFFI_IN"), bel.bel_wire(obel, "PADOUT"));
    if kind == "IOBS" {
        bel.claim_pip(bel.wire("DIFFO_IN"), bel.bel_wire(obel, "DIFFO_OUT"));
    }
    bel.commit();
}

fn verify_pll(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("PLL_ADV")
        .skip_out(bcls::PLL_V5::TEST_CLKIN);
    bel.claim_pip(bel.wire("TEST_CLKIN"), bel.wire("CLKIN1"));
    bel.commit();
}

fn verify_gt(vrf: &mut Verifier, bcrd: BelCoord) {
    let grefclk = match bcrd.slot {
        bslots::GTP_DUAL => bcls::GTP_DUAL::GREFCLK,
        bslots::GTX_DUAL => bcls::GTX_DUAL::GREFCLK,
        _ => unreachable!(),
    };
    let mut bel = vrf
        .verify_bel(bcrd)
        .ipad("RXP0", 4)
        .ipad("RXN0", 5)
        .ipad("RXP1", 6)
        .ipad("RXN1", 7)
        .opad("TXP0", 8)
        .opad("TXN0", 9)
        .opad("TXP1", 10)
        .opad("TXN1", 11)
        .extra_in_claim("CLKIN")
        .skip_in(grefclk);

    bel.claim_pip(bel.wire("CLKIN"), bel.wire("CLKOUT_NORTH_S"));
    bel.claim_pip(bel.wire("CLKIN"), bel.wire("CLKOUT_SOUTH_N"));
    bel.claim_pip(bel.wire("CLKIN"), bel.wire("GREFCLK"));
    bel.claim_pip(bel.wire("CLKIN"), bel.wire("BUFDS_O"));
    bel.claim_net(&[bel.wire("CLKOUT_NORTH")]);
    bel.claim_pip(bel.wire("CLKOUT_NORTH"), bel.wire("CLKOUT_NORTH_S"));
    bel.claim_pip(bel.wire("CLKOUT_NORTH"), bel.wire("BUFDS_O"));
    bel.claim_net(&[bel.wire("CLKOUT_SOUTH")]);
    bel.claim_pip(bel.wire("CLKOUT_SOUTH"), bel.wire("CLKOUT_SOUTH_N"));
    bel.claim_pip(bel.wire("CLKOUT_SOUTH"), bel.wire("BUFDS_O"));
    if let Some(obel) = bel.vrf.grid.bel_delta(bcrd.cell, 0, -20, bcrd.slot) {
        bel.verify_net(&[
            bel.wire("CLKOUT_NORTH_S"),
            bel.bel_wire(obel, "CLKOUT_NORTH"),
        ]);
    } else {
        bel.claim_net(&[bel.wire("CLKOUT_NORTH_S")]);
    }
    if let Some(obel) = bel.vrf.grid.bel_delta(bcrd.cell, 0, 20, bcrd.slot) {
        bel.verify_net(&[
            bel.wire("CLKOUT_SOUTH_N"),
            bel.bel_wire(obel, "CLKOUT_SOUTH"),
        ]);
    } else {
        bel.claim_net(&[bel.wire("CLKOUT_SOUTH_N")]);
    }
    bel.commit();

    vrf.verify_bel(bcrd)
        .sub(1)
        .kind("BUFDS")
        .skip_auto()
        .ipad_rename("IP", "BUFDS_IP", 2)
        .ipad_rename("IN", "BUFDS_IN", 3)
        .extra_out_claim_rename("O", "BUFDS_O")
        .commit();
}

fn verify_crc32(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::CRC32.index_of(bcrd.slot).unwrap();
    vrf.verify_bel(bcrd).commit();
    if idx.is_multiple_of(2) {
        let mut bel = vrf.verify_bel(bcrd).sub(1).kind("CRC64").skip_auto();
        for pin in [
            "CRCCLK",
            "CRCRESET",
            "CRCDATAVALID",
            "CRCDATAWIDTH0",
            "CRCDATAWIDTH1",
            "CRCDATAWIDTH2",
        ] {
            let pin64 = &format!("CRC64_{pin}");
            bel = bel.extra_in_claim_rename(pin, pin64);
            bel.claim_pip(bel.wire(pin64), bel.wire_far(pin64));
            bel.verify_net(&[bel.wire_far(pin64), bel.wire_far(pin)]);
        }
        let obel = bcrd.bel(bslots::CRC32[idx + 1]);
        for i in 0..32 {
            let pin = &format!("CRCIN{i}");
            let pin64 = &format!("CRC64_CRCIN{i}");
            bel = bel.extra_in_claim_rename(pin, pin64);
            bel.claim_pip(bel.wire(pin64), bel.wire_far(pin64));
            bel.verify_net(&[bel.wire_far(pin64), bel.bel_wire_far(obel, pin)]);
        }
        for i in 0..32 {
            let pin = &format!("CRCIN{i}");
            let pinh = &format!("CRCIN{ii}", ii = i + 32);
            let pin64 = &format!("CRC64_CRCIN{ii}", ii = i + 32);
            bel = bel.extra_in_claim_rename(pinh, pin64);
            bel.claim_pip(bel.wire(pin64), bel.wire_far(pin64));
            bel.verify_net(&[bel.wire_far(pin64), bel.wire_far(pin)]);
        }
        for i in 0..32 {
            let pin = &format!("CRCOUT{i}");
            let pin64 = &format!("CRC64_CRCOUT{i}");
            bel = bel.extra_out_claim_rename(pin, pin64);
            bel.claim_pip(bel.wire_far(pin64), bel.wire(pin64));
            bel.verify_net(&[bel.wire_far(pin64), bel.wire_far(pin)]);
        }
        bel.commit();
    }
}

pub fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
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
        _ if bslots::SLICE.contains(bcrd.slot) => verify_slice(edev, vrf, bcrd),
        _ if bslots::DSP.contains(bcrd.slot) => verify_dsp(edev, vrf, bcrd),
        bslots::BRAM => verify_bram(edev, vrf, bcrd),
        bslots::PMVBRAM => vrf.verify_bel(bcrd).kind("PMVBRAM").commit(),
        bslots::EMAC => vrf.verify_bel(bcrd).kind("TEMAC").commit(),
        bslots::PCIE | bslots::PPC => vrf.verify_bel(bcrd).commit(),

        _ if bslots::BUFGCTRL.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        _ if bslots::BSCAN.contains(bcrd.slot) || bslots::PMV_CFG.contains(bcrd.slot) => {
            vrf.verify_bel(bcrd).commit()
        }
        _ if bslots::ICAP.contains(bcrd.slot) => vrf.verify_bel(bcrd).kind("ICAP").commit(),
        bslots::STARTUP
        | bslots::DCIRESET
        | bslots::CAPTURE
        | bslots::USR_ACCESS
        | bslots::EFUSE_USR
        | bslots::KEY_CLEAR
        | bslots::JTAGPPC => vrf.verify_bel(bcrd).commit(),
        bslots::FRAME_ECC => vrf.verify_bel(bcrd).kind("FRAME_ECC").commit(),
        bslots::DCI => vrf.verify_bel(bcrd).kind("DCI").commit(),
        bslots::GLOBALSIG => vrf.verify_bel(bcrd).commit(),
        bslots::SYSMON => verify_sysmon(edev, vrf, bcrd),

        _ if bslots::ILOGIC.contains(bcrd.slot) => verify_ilogic(edev, vrf, bcrd),
        _ if bslots::OLOGIC.contains(bcrd.slot) => verify_ologic(vrf, bcrd),
        _ if bslots::IODELAY.contains(bcrd.slot) => verify_iodelay(vrf, bcrd),
        _ if bslots::IOB.contains(bcrd.slot) => verify_iob(vrf, bcrd),

        _ if bslots::DCM.contains(bcrd.slot) => vrf.verify_bel(bcrd).kind("DCM_ADV").commit(),
        bslots::PLL => verify_pll(vrf, bcrd),

        bslots::GTX_DUAL | bslots::GTP_DUAL => verify_gt(vrf, bcrd),
        _ if bslots::CRC32.contains(bcrd.slot) => verify_crc32(vrf, bcrd),

        _ if bslots::BUFR.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        _ if bslots::BUFIO.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        bslots::IDELAYCTRL => vrf.verify_bel(bcrd).commit(),

        _ => println!("MEOW {}", bcrd.to_string(edev.db)),
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
            verify_bel(endev.edev, &mut vrf, tcrd.bel(slot));
        }
    }
    verify_extra(endev, &mut vrf);
    vrf.finish();
}
