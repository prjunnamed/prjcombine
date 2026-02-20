use prjcombine_entity::{EntityBundleItemIndex, EntityId};
use prjcombine_interconnect::{
    db::{BelInfo, BelInputId, BelKind, SwitchBoxItem, WireSlotIdExt},
    grid::{BelCoord, RowId},
};
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelVerifier, RawWireCoord, Verifier};
use prjcombine_virtex4::{
    defs::{
        bcls, bslots, tslots,
        virtex4::{tcls, wires},
    },
    expanded::ExpandedDevice,
};

fn verify_slice(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::SLICE.index_of(bcrd.slot).unwrap();
    let is_slicem = matches!(idx, 0 | 2);
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind(if is_slicem { "SLICEM" } else { "SLICEL" })
        .extra_in("FXINA")
        .extra_in("FXINB")
        .extra_out("F5")
        .extra_out("FX")
        .extra_in("CIN")
        .extra_out("COUT");
    for pin in ["F5", "FX", "COUT"] {
        bel.claim_net(&[bel.wire(pin)]);
    }
    if is_slicem {
        bel = bel
            .extra_in("SHIFTIN")
            .extra_out("SHIFTOUT")
            .extra_in("ALTDIG")
            .extra_out("DIG")
            .extra_in("SLICEWE1")
            .extra_out("BYOUT")
            .extra_out("BYINVOUT");
        for pin in ["DIG", "BYOUT", "BYINVOUT", "SHIFTOUT"] {
            bel.claim_net(&[bel.wire(pin)]);
        }
    }
    for (dbel, dpin, sbel, spin) in [
        (bslots::SLICE[0], "FXINA", bslots::SLICE[0], "F5"),
        (bslots::SLICE[0], "FXINB", bslots::SLICE[2], "F5"),
        (bslots::SLICE[1], "FXINA", bslots::SLICE[1], "F5"),
        (bslots::SLICE[1], "FXINB", bslots::SLICE[3], "F5"),
        (bslots::SLICE[2], "FXINA", bslots::SLICE[0], "FX"),
        (bslots::SLICE[2], "FXINB", bslots::SLICE[1], "FX"),
        (bslots::SLICE[3], "FXINA", bslots::SLICE[2], "FX"),
        // SLICE3 FXINB <- top's SLICE2 FX

        // SLICE0 CIN <- bot's SLICE2 COUT
        // SLICE1 CIN <- bot's SLICE3 COUT
        (bslots::SLICE[2], "CIN", bslots::SLICE[0], "COUT"),
        (bslots::SLICE[3], "CIN", bslots::SLICE[1], "COUT"),
        (bslots::SLICE[0], "SHIFTIN", bslots::SLICE[2], "SHIFTOUT"),
        // SLICE2 SHIFTIN disconnected?
        (bslots::SLICE[0], "ALTDIG", bslots::SLICE[2], "DIG"),
        // SLICE2 ALTDIG disconnected?
        (bslots::SLICE[0], "SLICEWE1", bslots::SLICE[0], "BYOUT"),
        (bslots::SLICE[2], "SLICEWE1", bslots::SLICE[0], "BYINVOUT"),
    ] {
        if dbel != bcrd.slot {
            continue;
        }
        let obel = bcrd.bel(sbel);
        bel.claim_pip(bel.wire(dpin), bel.bel_wire(obel, spin));
        bel.claim_net(&[bel.wire(dpin)]);
    }
    if bcrd.slot == bslots::SLICE[2] {
        bel.claim_net(&[bel.wire("SHIFTIN")]);
        bel.claim_net(&[bel.wire("ALTDIG")]);
    }
    if bcrd.slot == bslots::SLICE[3] {
        if let Some(cell) = edev.cell_delta(bcrd.cell, 0, 1)
            && let obel = cell.bel(bslots::SLICE[2])
            && edev.has_bel(obel)
        {
            bel.claim_net(&[bel.wire("FXINB"), bel.bel_wire(obel, "FX_S")]);
            bel.claim_pip(bel.bel_wire(obel, "FX_S"), bel.bel_wire(obel, "FX"));
        } else {
            bel.claim_net(&[bel.wire("FXINB")]);
        }
    }
    for (dbel, sbel) in [
        (bslots::SLICE[0], bslots::SLICE[2]),
        (bslots::SLICE[1], bslots::SLICE[3]),
    ] {
        if bcrd.slot != dbel {
            continue;
        }
        if let Some(cell) = edev.cell_delta(bcrd.cell, 0, -1)
            && let obel = cell.bel(sbel)
            && edev.has_bel(obel)
        {
            bel.claim_net(&[bel.wire("CIN"), bel.bel_wire(obel, "COUT_N")]);
            bel.claim_pip(bel.bel_wire(obel, "COUT_N"), bel.bel_wire(obel, "COUT"));
        } else {
            bel.claim_net(&[bel.wire("CIN")]);
        }
    }
    bel.commit();
}

fn verify_bram(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("RAMB16")
        .extra_in("CASCADEINA")
        .extra_in("CASCADEINB")
        .extra_out("CASCADEOUTA")
        .extra_out("CASCADEOUTB");
    for (ipin, opin) in [("CASCADEINA", "CASCADEOUTA"), ("CASCADEINB", "CASCADEOUTB")] {
        bel.claim_net(&[bel.wire(opin)]);
        bel.claim_net(&[bel.wire(ipin)]);
        if let Some(obel) = edev.bel_carry_prev(bcrd) {
            bel.verify_net(&[bel.wire_far(ipin), bel.bel_wire(obel, opin)]);
            bel.claim_pip(bel.wire(ipin), bel.wire_far(ipin));
        }
    }
    bel.commit();
    let mut ipins = vec![];
    let mut opins = vec![];
    for i in 0..32 {
        ipins.push((format!("DI{i}"), format!("DIB{i}")));
        opins.push((format!("DO{i}"), format!("DOA{i}")));
    }
    for i in 0..4 {
        ipins.push((format!("DIP{i}"), format!("DIPB{i}")));
        opins.push((format!("DOP{i}"), format!("DOPA{i}")));
    }
    for i in 0..12 {
        let (ridx, widx) = match i {
            0..4 => (i, i + 16),
            4..8 => (i - 4 + 24, i - 4 + 20),
            8..12 => (i - 8 + 12, i - 8 + 28),
            _ => unreachable!(),
        };
        opins.push((format!("RDCOUNT{i}"), format!("DOB{ridx}")));
        opins.push((format!("WRCOUNT{i}"), format!("DOB{widx}")));
    }
    for (idx, pin) in [
        (5, "RDERR"),
        (6, "ALMOSTEMPTY"),
        (7, "EMPTY"),
        (8, "FULL"),
        (9, "ALMOSTFULL"),
        (10, "WRERR"),
    ] {
        opins.push((pin.to_string(), format!("DOB{idx}")));
    }
    for (fpin, bpin) in [
        ("RDEN", "ENA"),
        ("RDCLK", "CLKA"),
        ("WREN", "ENB"),
        ("WRCLK", "CLKB"),
        ("RST", "SSRA"),
    ] {
        ipins.push((fpin.to_string(), bpin.to_string()));
    }
    let mut bel = vrf.verify_bel(bcrd).sub(1).kind("FIFO16").skip_auto();
    for (fpin, bpin) in &ipins {
        bel = bel.extra_in(fpin);
        bel.claim_pip(bel.wire(fpin), bel.wire_far(fpin));
        bel.claim_net(&[bel.wire(fpin)]);
        bel.verify_net(&[bel.wire_far(fpin), bel.wire_far(bpin)]);
    }
    for (fpin, bpin) in &opins {
        bel = bel.extra_out(fpin);
        bel.claim_net(&[bel.wire(fpin)]);
        if fpin.starts_with("DOP") {
            let pnaming = &bel.naming.pins[fpin];
            for pip in pnaming.int_pips.values() {
                bel.claim_pip(
                    RawWireCoord {
                        crd: bel.crd(),
                        wire: &pip.wire_to,
                    },
                    bel.wire(fpin),
                );
            }
        } else {
            bel.claim_pip(bel.wire_far(fpin), bel.wire(fpin));
            bel.verify_net(&[bel.wire_far(fpin), bel.wire_far(bpin)]);
        }
    }
    bel.commit();
}

fn verify_dsp(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("DSP48");
    for (name_in, name_out, num) in [("BCIN", "BCOUT", 18), ("PCIN", "PCOUT", 48)] {
        for i in 0..num {
            let ipin = &format!("{name_in}{i}");
            let opin = &format!("{name_out}{i}");
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
    let mut c_pins = vec!["RSTC".to_string(), "CEC".to_string()];
    for i in 0..48 {
        c_pins.push(format!("C{i}"));
    }
    for pin in c_pins {
        bel = bel.extra_in(&pin);
        bel.claim_net(&[bel.wire(&pin)]);
        bel.claim_pip(bel.wire(&pin), bel.wire_far(&pin));
    }
    bel.commit();
}

fn verify_ppc(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("PPC405_ADV");
    let mut dcri = vec!["EMACDCRACK".to_string()];
    let mut dcro = vec![
        "DCREMACCLK".to_string(),
        "DCREMACREAD".to_string(),
        "DCREMACWRITE".to_string(),
    ];
    for i in 0..32 {
        dcri.push(format!("EMACDCRDBUS{i}"));
        dcro.push(format!("DCREMACDBUS{i}"));
    }
    for i in 8..10 {
        dcro.push(format!("DCREMACABUS{i}"));
    }
    let obel = bcrd.bel(bslots::EMAC);
    for pin in &dcri {
        bel = bel.extra_in(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.wire(pin), bel.bel_wire(obel, pin));
    }
    for pin in &dcro {
        bel = bel.extra_out(pin);
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.bel_wire(obel, pin), bel.wire(pin));
    }
    // detritus.
    bel.vrf.claim_pip_tri(
        bel.crds[EntityId::from_idx(0)],
        "PB_OMUX_S0_B5",
        "PB_OMUX15_B5",
    );
    bel.vrf.claim_pip_tri(
        bel.crds[EntityId::from_idx(0)],
        "PB_OMUX_S0_B6",
        "PB_OMUX15_B6",
    );
    bel.vrf.claim_pip_tri(
        bel.crds[EntityId::from_idx(1)],
        "PT_OMUX_N15_T5",
        "PT_OMUX0_T5",
    );
    bel.vrf.claim_pip_tri(
        bel.crds[EntityId::from_idx(1)],
        "PT_OMUX_N15_T6",
        "PT_OMUX0_T6",
    );
    bel.commit();
}

fn verify_emac(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("EMAC");
    let mut dcro = vec!["EMACDCRACK".to_string()];
    let mut dcri = vec![
        "DCREMACCLK".to_string(),
        "DCREMACREAD".to_string(),
        "DCREMACWRITE".to_string(),
    ];
    for i in 0..32 {
        dcro.push(format!("EMACDCRDBUS{i}"));
        dcri.push(format!("DCREMACDBUS{i}"));
    }
    for i in 8..10 {
        dcri.push(format!("DCREMACABUS{i}"));
    }
    for pin in &dcri {
        bel = bel.extra_in(pin);
        bel.claim_net(&[bel.wire(pin)]);
    }
    for pin in &dcro {
        bel = bel.extra_out(pin);
        bel.claim_net(&[bel.wire(pin)]);
    }
    bel.commit();
}

fn verify_bufio(vrf: &mut Verifier, bcrd: BelCoord) {
    vrf.verify_bel(bcrd)
        .skip_auto()
        .extra_in_claim("I")
        .extra_out("O")
        .commit();
}

fn verify_test_in(bel: &mut BelVerifier, pin: BelInputId) {
    let ngrid = bel.vrf.ngrid;
    let egrid = bel.vrf.grid;
    let intdb = egrid.db;
    let BelKind::Class(bcid) = intdb.bel_slots[bel.bcrd.slot].kind else {
        unreachable!()
    };
    let (pname, idx) = intdb[bcid].inputs.key(pin);
    let pname = match idx {
        EntityBundleItemIndex::Single => pname.to_string(),
        EntityBundleItemIndex::Array { index, .. } => format!("{pname}{index}"),
    };
    let tcrd = egrid.bel_tile(bel.bcrd);
    let tcid = egrid[tcrd].class;
    let BelInfo::Bel(ref binfo) = intdb[tcid].bels[bel.bcrd.slot] else {
        unreachable!()
    };
    let wire = binfo.inputs[pin].wire();
    let ntile = &ngrid.tiles[&tcrd];
    let ntcls = &ngrid.db.tile_class_namings[ntile.naming];
    let sbslot = match tcrd.slot {
        tslots::BEL => bslots::SPEC_INT,
        tslots::CFG => bslots::SYSMON_INT,
        _ => unreachable!(),
    };
    let BelInfo::SwitchBox(ref sb) = intdb[tcid].bels[sbslot] else {
        unreachable!()
    };
    for item in &sb.items {
        let SwitchBoxItem::Mux(mux) = item else {
            continue;
        };
        if mux.dst != wire {
            continue;
        }
        for src in mux.src.keys() {
            let wn = &ntcls.wires[&src.tw];
            let rw = RawWireCoord {
                crd: bel.crd(),
                wire: &wn.name,
            };
            bel.claim_pip(bel.wire(&pname), rw);
        }
    }
}

fn verify_dcm(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("DCM_ADV");
    verify_test_in(&mut bel, bcls::DCM_V4::CLKIN);
    verify_test_in(&mut bel, bcls::DCM_V4::CLKFB);

    for (pin, bpin) in [
        ("CONCUR", "CONCUR_BUF"),
        ("CLKFX", "CLKFX_BUF"),
        ("CLKFX180", "CLKFX180_BUF"),
        ("CLK0", "CLK0_BUF"),
        ("CLK180", "CLK180_BUF"),
        ("CLK90", "CLK90_BUF"),
        ("CLK270", "CLK270_BUF"),
        ("CLK2X180", "CLK2X180_BUF"),
        ("CLK2X", "CLK2X_BUF"),
        ("CLKDV", "CLKDV_BUF"),
    ] {
        bel.claim_pip(bel.wire(bpin), bel.wire(pin));
    }

    bel.commit();
}

fn verify_pmcd(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd);
    verify_test_in(&mut bel, bcls::PMCD::CLKA);
    verify_test_in(&mut bel, bcls::PMCD::CLKB);
    verify_test_in(&mut bel, bcls::PMCD::CLKC);
    verify_test_in(&mut bel, bcls::PMCD::CLKD);
    bel.commit();
}

fn verify_dpm(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd);
    verify_test_in(&mut bel, bcls::DPM::REFCLK);
    verify_test_in(&mut bel, bcls::DPM::TESTCLK1);
    verify_test_in(&mut bel, bcls::DPM::TESTCLK2);
    bel.commit();
}

fn verify_sysmon(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("MONITOR")
        .ipad("VP", 1)
        .ipad("VN", 2);
    for i in 0..8 {
        let Some((iop, _)) = edev.get_sysmon_vaux(bcrd.cell, i) else {
            continue;
        };
        for (j, n) in [(1, "VP"), (0, "VN")] {
            let pin = &format!("{n}{i}");
            bel = bel.extra_in(pin);
            bel.claim_net(&[bel.wire(pin)]);
            bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
            let obel = iop.cell.bel(bslots::IOB[j]);
            bel.claim_net(&[bel.wire_far(pin), bel.bel_wire(obel, "MONITOR")]);
            bel.claim_pip(bel.bel_wire(obel, "MONITOR"), bel.bel_wire(obel, "PADOUT"));
        }
    }

    verify_test_in(&mut bel, bcls::SYSMON_V4::CONVST);

    bel.commit();
}

fn verify_ilogic(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::ILOGIC.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("ISERDES")
        .extra_in_claim("TFB")
        .extra_in_claim("OFB")
        .extra_in_claim("D")
        .extra_in_claim("OCLK")
        .extra_in_claim("SHIFTIN1")
        .extra_in_claim("SHIFTIN2")
        .extra_out_claim("SHIFTOUT1")
        .extra_out_claim("SHIFTOUT2")
        .skip_out(bcls::ILOGIC_V4::CLKPAD);
    let obel = bcrd.bel(bslots::IOB[idx]);
    bel.claim_pip(bel.wire("D"), bel.bel_wire(obel, "I"));
    let obel = bcrd.bel(bslots::OLOGIC[idx]);
    bel.claim_pip(bel.wire("OCLK"), bel.bel_wire_far(obel, "CLK"));
    bel.claim_pip(bel.wire("OFB"), bel.bel_wire(obel, "OQ"));
    bel.claim_pip(bel.wire("TFB"), bel.bel_wire(obel, "TQ"));
    if idx == 0 {
        let obel = bcrd.bel(bslots::ILOGIC[1]);
        bel.claim_pip(bel.wire("SHIFTIN1"), bel.bel_wire(obel, "SHIFTOUT1"));
        bel.claim_pip(bel.wire("SHIFTIN2"), bel.bel_wire(obel, "SHIFTOUT2"));
    }
    if idx == 1 {
        let is_bufio = matches!(bcrd.row.to_idx() % 16, 7 | 8);
        let row_dcmiob = edev.row_dcmiob.unwrap();
        let row_iobdcm = edev.row_iobdcm.unwrap();
        let row_iobdcm_m16: RowId = row_iobdcm - 16;
        let is_giob = bcrd.col == edev.col_cfg
            && (row_dcmiob.range(row_dcmiob + 16).contains(bcrd.row)
                || row_iobdcm_m16.range(row_iobdcm).contains(bcrd.row));
        if is_giob || is_bufio {
            bel.claim_pip(bel.wire("CLKPAD"), bel.wire("O"));
        }
    }
    bel.commit();
}

fn verify_ologic(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("OSERDES")
        .extra_out_claim("OQ")
        .extra_in_claim("SHIFTIN1")
        .extra_in_claim("SHIFTIN2")
        .extra_out_claim("SHIFTOUT1")
        .extra_out_claim("SHIFTOUT2");
    if bcrd.slot == bslots::OLOGIC[1] {
        let obel = bcrd.bel(bslots::OLOGIC[0]);
        bel.claim_pip(bel.wire("SHIFTIN1"), bel.bel_wire(obel, "SHIFTOUT1"));
        bel.claim_pip(bel.wire("SHIFTIN2"), bel.bel_wire(obel, "SHIFTOUT2"));
    }
    bel.commit();
}

fn verify_iob(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IOB.index_of(bcrd.slot).unwrap();
    let kind = if bcrd.col == edev.col_cfg || matches!(bcrd.row.to_idx() % 16, 7 | 8) {
        "LOWCAPIOB"
    } else if bcrd.slot == bslots::IOB[1] {
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
    bel.claim_pip(bel.wire("O"), bel.bel_wire(obel, "OQ"));
    bel.claim_pip(bel.wire("T"), bel.bel_wire(obel, "TQ"));
    let obel = bcrd.bel(bslots::IOB[idx ^ 1]);
    bel.claim_pip(bel.wire("DIFFI_IN"), bel.bel_wire(obel, "PADOUT"));
    if kind == "IOBS" {
        bel.claim_pip(bel.wire("DIFFO_IN"), bel.bel_wire(obel, "DIFFO_OUT"));
    }
    bel.commit();
}

fn verify_gt11(vrf: &mut Verifier, bcrd: BelCoord) {
    let gtidx = bslots::GT11.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .extra_out_claim("RXMCLK")
        .ipad("RX1P", 1)
        .ipad("RX1N", 2)
        .opad("TX1P", 3)
        .opad("TX1N", 4);
    for i in 0..16 {
        bel = bel
            .extra_in_claim(format!("COMBUSIN{i}"))
            .extra_out_claim(format!("COMBUSOUT{i}"));
    }
    if gtidx == 0 {
        bel.claim_pip(bel.wire_far("RXMCLK"), bel.wire("RXMCLK"));
    }
    bel.commit();
}

fn verify_gt11clk(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .skip_out(bcls::GT11CLK::SYNCLK1)
        .skip_out(bcls::GT11CLK::SYNCLK2)
        .extra_in_claim("RXBCLK")
        .extra_in_claim("SYNCLK1IN")
        .extra_in_claim("SYNCLK2IN")
        .extra_out_claim("SYNCLK1OUT")
        .extra_out_claim("SYNCLK2OUT")
        .ipad("MGTCLKP", 1)
        .ipad("MGTCLKN", 2);

    let obel_a = bcrd.bel(bslots::GT11[1]);
    let obel_b = bcrd.bel(bslots::GT11[0]);

    bel.verify_net(&[bel.wire("RXBCLK"), bel.bel_wire_far(obel_b, "RXMCLK")]);

    for i in 0..16 {
        bel.verify_net(&[
            bel.wire(&format!("COMBUSIN_A{i}")),
            bel.bel_wire(obel_a, &format!("COMBUSIN{i}")),
        ]);
        bel.verify_net(&[
            bel.wire(&format!("COMBUSIN_B{i}")),
            bel.bel_wire(obel_b, &format!("COMBUSIN{i}")),
        ]);
        bel.verify_net(&[
            bel.wire(&format!("COMBUSOUT_A{i}")),
            bel.bel_wire(obel_a, &format!("COMBUSOUT{i}")),
        ]);
        bel.verify_net(&[
            bel.wire(&format!("COMBUSOUT_B{i}")),
            bel.bel_wire(obel_b, &format!("COMBUSOUT{i}")),
        ]);
        bel.claim_pip(
            bel.wire(&format!("COMBUSIN_A{i}")),
            bel.wire(&format!("COMBUSOUT_B{i}")),
        );
        bel.claim_pip(
            bel.wire(&format!("COMBUSIN_B{i}")),
            bel.wire(&format!("COMBUSOUT_A{i}")),
        );
    }

    if let Some(cell) = bel.vrf.grid.cell_delta(bcrd.cell, 0, -32) {
        let obel = cell.bel(bslots::GT11CLK);
        bel.verify_net(&[bel.wire("SYNCLK1_S"), bel.bel_wire(obel, "SYNCLK1")]);
        bel.verify_net(&[bel.wire("SYNCLK2_S"), bel.bel_wire(obel, "SYNCLK2")]);
    } else {
        bel.claim_net(&[bel.wire("SYNCLK1_S")]);
        bel.claim_net(&[bel.wire("SYNCLK2_S")]);
    }
    bel.claim_pip(bel.wire("SYNCLK1_S"), bel.wire("SYNCLK1"));
    bel.claim_pip(bel.wire("SYNCLK2_S"), bel.wire("SYNCLK2"));
    bel.claim_pip(bel.wire("SYNCLK1_S"), bel.wire("SYNCLK1OUT"));
    bel.claim_pip(bel.wire("SYNCLK2_S"), bel.wire("SYNCLK2OUT"));
    bel.claim_pip(bel.wire("SYNCLK1"), bel.wire("SYNCLK1_S"));
    bel.claim_pip(bel.wire("SYNCLK2"), bel.wire("SYNCLK2_S"));
    bel.claim_pip(bel.wire("SYNCLK1"), bel.wire("SYNCLK1OUT"));
    bel.claim_pip(bel.wire("SYNCLK2"), bel.wire("SYNCLK2OUT"));
    bel.claim_pip(bel.wire("SYNCLK1IN"), bel.wire("SYNCLK1"));
    bel.claim_pip(bel.wire("SYNCLK2IN"), bel.wire("SYNCLK2"));

    bel.commit();
}

fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    match bcrd.slot {
        bslots::INT
        | bslots::INTF_INT
        | bslots::INTF_TESTMUX
        | bslots::HCLK
        | bslots::HROW_INT
        | bslots::SPEC_INT
        | bslots::CLK_INT
        | bslots::SYSMON_INT
        | bslots::HCLK_IO_INT
        | bslots::MISC_CFG
        | bslots::GLOBAL => (),
        _ if bslots::SLICE.contains(bcrd.slot) => verify_slice(edev, vrf, bcrd),
        _ if bslots::DSP.contains(bcrd.slot) => verify_dsp(edev, vrf, bcrd),
        bslots::DSP_C => (),
        bslots::BRAM => verify_bram(edev, vrf, bcrd),
        bslots::PPC => verify_ppc(vrf, bcrd),
        bslots::EMAC => verify_emac(vrf, bcrd),

        _ if bslots::BUFGCTRL.contains(bcrd.slot) || bslots::BSCAN.contains(bcrd.slot) => {
            vrf.verify_bel(bcrd).commit()
        }
        _ if bslots::ICAP.contains(bcrd.slot) => vrf.verify_bel(bcrd).kind("ICAP").commit(),
        _ if bslots::PMV_CFG.contains(bcrd.slot) => vrf.verify_bel(bcrd).kind("PMV").commit(),
        bslots::STARTUP
        | bslots::DCIRESET
        | bslots::CAPTURE
        | bslots::USR_ACCESS
        | bslots::GLOBALSIG => vrf.verify_bel(bcrd).commit(),
        bslots::FRAME_ECC => vrf.verify_bel(bcrd).kind("FRAME_ECC").commit(),
        bslots::DCI => vrf.verify_bel(bcrd).kind("DCI").commit(),
        bslots::LVDS => (),
        bslots::JTAGPPC => vrf.verify_bel(bcrd).extra_in_claim("TDOTSPPC").commit(),

        _ if bslots::BUFR.contains(bcrd.slot) => vrf.verify_bel(bcrd).commit(),
        _ if bslots::BUFIO.contains(bcrd.slot) => verify_bufio(vrf, bcrd),
        bslots::IDELAYCTRL => vrf.verify_bel(bcrd).commit(),

        _ if bslots::ILOGIC.contains(bcrd.slot) => verify_ilogic(edev, vrf, bcrd),
        _ if bslots::OLOGIC.contains(bcrd.slot) => verify_ologic(vrf, bcrd),
        _ if bslots::IOB.contains(bcrd.slot) => verify_iob(edev, vrf, bcrd),

        _ if bslots::DCM.contains(bcrd.slot) => verify_dcm(vrf, bcrd),
        _ if bslots::PMCD.contains(bcrd.slot) => verify_pmcd(vrf, bcrd),
        bslots::DPM => verify_dpm(vrf, bcrd),
        bslots::CCM => (),
        bslots::SYSMON => verify_sysmon(edev, vrf, bcrd),
        _ if bslots::GT11.contains(bcrd.slot) => verify_gt11(vrf, bcrd),
        bslots::GT11CLK => verify_gt11clk(vrf, bcrd),
        _ => println!("MEOW {}", bcrd.to_string(edev.db)),
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
    let mut vrf = Verifier::new(rd, &endev.ngrid);

    for (tkn, prefix, do_s, do_n) in [
        ("HCLK_IOIS_DCI", "HCLK_IOIS", true, true),
        ("HCLK_IOIS_LVDS", "HCLK_IOIS", true, true),
        (
            "HCLK_CENTER",
            "HCLK_IOIS",
            true,
            endev.edev.tile_index[tcls::HCLK_IO_CENTER].len() > 1,
        ),
        ("HCLK_CENTER_ABOVE_CFG", "HCLK_IOIS", false, true),
        ("HCLK_DCMIOB", "HCLK_DCMIOB", false, true),
        ("HCLK_IOBDCM", "HCLK_IOBDCM", true, false),
    ] {
        for i in 0..2 {
            vrf.mark_merge_pip(
                tkn,
                &format!("{prefix}_IOCLKP{i}"),
                &format!("{prefix}_VIOCLKP{i}"),
            );
        }
        for &crd in rd.tiles_by_kind_name(tkn) {
            if do_n {
                vrf.try_merge_node(
                    RawWireCoord {
                        crd,
                        wire: "HCLK_IOIS_BUFIO_OUT0",
                    },
                    RawWireCoord {
                        crd,
                        wire: "HCLK_IOIS_I2IOCLK_TOP_P",
                    },
                );
                vrf.claim_pip_tri(crd, "HCLK_IOIS_BUFIO_IN0", "HCLK_IOIS_I2IOCLK_TOP_P");
            }
            if do_s {
                vrf.merge_node(
                    RawWireCoord {
                        crd,
                        wire: "HCLK_IOIS_BUFIO_OUT1",
                    },
                    RawWireCoord {
                        crd,
                        wire: "HCLK_IOIS_I2IOCLK_BOT_P",
                    },
                );
                vrf.claim_pip_tri(crd, "HCLK_IOIS_BUFIO_IN1", "HCLK_IOIS_I2IOCLK_BOT_P");
            }
        }
    }
    for tcid in [tcls::HCLK_IO_DCI, tcls::HCLK_IO_LVDS] {
        for &tcrd in &endev.edev.tile_index[tcid] {
            let ntile = &endev.ngrid.tiles[&tcrd];
            for i in [2, 3] {
                let crd = vrf.xlat_tile(&ntile.names[RawTileId::from_idx(i)]).unwrap();
                vrf.mark_merge_single_pip(crd, "IOIS_BYP_INT_B4", "BYP_INT_B4_INT");
            }
        }
    }
    for tcid in [
        tcls::HCLK_IO_DCI,
        tcls::HCLK_IO_LVDS,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_N,
        tcls::HCLK_IO_DCM_S,
        tcls::HCLK_IO_DCM_N,
    ] {
        vrf.inject_tcls_pip(tcid, wires::IOCLK[0].cell(2), wires::OUT_CLKPAD.cell(2));
        vrf.inject_tcls_pip(tcid, wires::IOCLK[1].cell(2), wires::OUT_CLKPAD.cell(1));
    }

    for tkn in ["CLK_IOB_B", "CLK_IOB_T"] {
        for i in 0..16 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CLK_IOB_IOB_BUFCLKP{i}"),
                &format!("CLK_IOB_PAD_CLKP{i}"),
            );
        }
    }

    for tkn in ["CLK_BUFGCTRL_B", "CLK_BUFGCTRL_T"] {
        for i in 0..16 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CLK_BUFGCTRL_GFB_P{i}"),
                &format!("CLK_BUFGCTRL_POSTMUX_GCLKP{i}"),
            );
        }
    }

    for i in 0..16 {
        vrf.mark_merge_pip(
            "CLK_HROW",
            &format!("CLK_HROW_IOB_H_BUFCLKP{i}"),
            &format!("CLK_HROW_IOB_BUFCLKP{i}"),
        );
    }
    vrf.mark_merge_pip("CLK_HROW", "CLK_HROW_V_MGT_L0", "CLK_HROW_H_MGT_L0");
    vrf.mark_merge_pip("CLK_HROW", "CLK_HROW_V_MGT_L1", "CLK_HROW_H_MGT_L1");
    vrf.mark_merge_pip("CLK_HROW", "CLK_HROW_V_MGT_R0", "CLK_HROW_H_MGT_R0");
    vrf.mark_merge_pip("CLK_HROW", "CLK_HROW_V_MGT_R1", "CLK_HROW_H_MGT_R1");
    for tkn in ["HCLK_CENTER", "HCLK_CENTER_ABOVE_CFG"] {
        vrf.mark_merge_pip(tkn, "HCLK_CENTER_MGT0", "HCLK_MGT_CLKL0");
        vrf.mark_merge_pip(tkn, "HCLK_CENTER_MGT1", "HCLK_MGT_CLKL1");
        vrf.mark_merge_pip(tkn, "HCLK_CENTER_MGT2", "HCLK_MGT_CLKR0");
        vrf.mark_merge_pip(tkn, "HCLK_CENTER_MGT3", "HCLK_MGT_CLKR1");
    }

    for i in 0..16 {
        let ii0 = if i < 8 { 2 * i } else { 31 - 2 * (i - 8) };
        let ii1 = ii0 ^ 1;
        vrf.mark_merge_pip(
            "CFG_CENTER",
            &format!("LOGIC_CREATED_INPUT_B0_INT{i}"),
            &format!("CFG_CENTER_PREMUX1_CLKP{ii0}"),
        );
        vrf.mark_merge_pip(
            "CFG_CENTER",
            &format!("LOGIC_CREATED_INPUT_B1_INT{i}"),
            &format!("CFG_CENTER_PREMUX0_CLKP{ii0}"),
        );
        vrf.mark_merge_pip(
            "CFG_CENTER",
            &format!("LOGIC_CREATED_INPUT_B2_INT{i}"),
            &format!("CFG_CENTER_PREMUX1_CLKP{ii1}"),
        );
        vrf.mark_merge_pip(
            "CFG_CENTER",
            &format!("LOGIC_CREATED_INPUT_B3_INT{i}"),
            &format!("CFG_CENTER_PREMUX0_CLKP{ii1}"),
        );
    }
    for i in 0..32 {
        let int = if i < 16 { 1 + i / 4 } else { 11 + (i - 16) / 4 };
        vrf.mark_merge_pip(
            "CFG_CENTER",
            &format!("CFG_CENTER_CKINT0_{i}"),
            &format!("IMUX_B{ii}_INT{int}", ii = [3, 7, 19, 23][i % 4]),
        );
        vrf.mark_merge_pip(
            "CFG_CENTER",
            &format!("CFG_CENTER_CKINT1_{i}"),
            &format!("IMUX_B{ii}_INT{int}", ii = [11, 15, 27, 31][i % 4]),
        );
    }

    for tkn in ["CLKV_DCM_B", "CLKV_DCM_T"] {
        for i in 0..24 {
            vrf.mark_merge_pip(
                tkn,
                &format!("CLKV_DCM_DCM_OUTCLKP{i}"),
                &format!("CLKV_DCM_DCM{j}_CLKP{k}", j = i / 12, k = i % 12),
            );
        }
    }

    if endev.edev.col_lgt.is_none() {
        for (wt, wf) in [
            (wires::MGT_DCM[0].cell(2), wires::MGT_ROW[0].cell(2)),
            (wires::MGT_DCM[1].cell(2), wires::MGT_ROW[1].cell(2)),
            (wires::MGT_DCM[2].cell(2), wires::MGT_ROW[0].cell(4)),
            (wires::MGT_DCM[3].cell(2), wires::MGT_ROW[1].cell(4)),
            (wires::MGT_DCM[0].cell(1), wires::MGT_ROW[0].cell(2)),
            (wires::MGT_DCM[1].cell(1), wires::MGT_ROW[1].cell(2)),
            (wires::MGT_DCM[2].cell(1), wires::MGT_ROW[0].cell(4)),
            (wires::MGT_DCM[3].cell(1), wires::MGT_ROW[1].cell(4)),
        ] {
            vrf.skip_tcls_pip(tcls::HCLK_DCM, wt, wf);
        }
        vrf.kill_stub_in_cond("HCLK_DCM_MGT0");
        vrf.kill_stub_in_cond("HCLK_DCM_MGT1");
        vrf.kill_stub_in_cond("HCLK_DCM_MGT2");
        vrf.kill_stub_in_cond("HCLK_DCM_MGT3");
    }

    for &crd in rd.tiles_by_kind_name("SYS_MON") {
        vrf.try_merge_node(
            RawWireCoord {
                crd,
                wire: "MONITOR_CONVST",
            },
            RawWireCoord {
                crd,
                wire: "MONITOR_CONVST_TEST",
            },
        );
    }

    for &crd in rd.tiles_by_kind_name("CCM") {
        for wire in [
            "PMCD_0_CLKA",
            "PMCD_0_CLKB",
            "PMCD_0_CLKC",
            "PMCD_0_CLKD",
            "PMCD_1_CLKA",
            "PMCD_1_CLKB",
            "PMCD_1_CLKC",
            "PMCD_1_CLKD",
            "DPM_REFCLK",
            "DPM_TESTCLK1",
            "DPM_TESTCLK2",
        ] {
            vrf.try_merge_node(
                RawWireCoord { crd, wire },
                RawWireCoord {
                    crd,
                    wire: &format!("{wire}_TEST"),
                },
            );
        }
    }
    for tkn in ["DCM", "DCM_BOT"] {
        for &crd in rd.tiles_by_kind_name(tkn) {
            for wire in ["DCM_ADV_CLKIN", "DCM_ADV_CLKFB"] {
                vrf.try_merge_node(
                    RawWireCoord { crd, wire },
                    RawWireCoord {
                        crd,
                        wire: &format!("{wire}_TEST"),
                    },
                );
            }
            for (i, wire) in [
                (1, "DCM_CONCUR"),
                (2, "DCM_CLKFX"),
                (3, "DCM_CLKFX180"),
                (4, "DCM_CLK0"),
                (5, "DCM_CLK180"),
                (6, "DCM_CLK90"),
                (7, "DCM_CLK270"),
                (8, "DCM_CLK2X180"),
                (9, "DCM_CLK2X"),
                (10, "DCM_CLKDV"),
            ] {
                vrf.try_merge_node(
                    RawWireCoord { crd, wire },
                    RawWireCoord {
                        crd,
                        wire: &format!("DCM_TO_BUFG{i}"),
                    },
                );
            }
        }
    }

    for tkn in [
        "MGT_AL",
        "MGT_AL_BOT",
        "MGT_AL_MID",
        "MGT_AR",
        "MGT_AR_BOT",
        "MGT_AR_MID",
        "MGT_BL",
        "MGT_BR",
    ] {
        for &crd in rd.tiles_by_kind_name(tkn) {
            for (wa, wb) in [
                ("MGT_FWDCLK1_T", "MGT_FWDCLK1_B"),
                ("MGT_FWDCLK2_T", "MGT_FWDCLK2_B"),
                ("MGT_FWDCLK3_T", "MGT_FWDCLK3_B"),
                ("MGT_FWDCLK4_T", "MGT_FWDCLK4_B"),
            ] {
                vrf.claim_pip_tri(crd, wa, wb);
                vrf.claim_pip_tri(crd, wb, wa);
                vrf.try_merge_node(
                    RawWireCoord { crd, wire: wa },
                    RawWireCoord { crd, wire: wb },
                );
            }
        }
    }

    vrf.skip_bslot(bslots::BUFIO[0]);
    vrf.skip_bslot(bslots::BUFIO[1]);

    for i in 0..2 {
        vrf.skip_tcls_pip(
            tcls::HCLK_MGT,
            wires::MGT_ROW[i].cell(0),
            wires::MGT_CLK_OUT[i].cell(0),
        );
    }
    for i in 0..8 {
        vrf.skip_tcls_pip(
            tcls::HCLK_MGT,
            wires::HCLK_MGT[i].cell(0),
            wires::HCLK_ROW[i].cell(0),
        );
    }

    for &tcrd in &endev.edev.tile_index[tcls::HCLK_MGT] {
        for i in 0..8 {
            let dst = wires::HCLK_MGT[i].cell(0);
            let src = wires::HCLK_ROW[i].cell(0);
            vrf.alias_wire(
                endev.edev.resolve_tile_wire(tcrd, dst).unwrap(),
                endev.edev.resolve_tile_wire(tcrd, src).unwrap(),
            );
        }
        for i in 0..2 {
            let dst = wires::MGT_ROW[i].cell(0);
            let src = wires::MGT_CLK_OUT[i].cell(0);
            vrf.alias_wire(
                endev.edev.resolve_tile_wire(tcrd, dst).unwrap(),
                endev.edev.resolve_tile_wire(tcrd, src).unwrap(),
            );
        }
    }

    for co in 0..2 {
        for o in 0..8 {
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

    if !endev.edev.tile_index[tcls::CCM].is_empty() {
        let muxes = &endev.edev.db_index.tile_classes[tcls::CCM].muxes;
        for (rel, rel_test) in [
            (wires::IMUX_CCM_REL[0].cell(0), wires::IMUX_SPEC[3].cell(0)),
            (wires::IMUX_CCM_REL[1].cell(0), wires::IMUX_SPEC[2].cell(0)),
        ] {
            vrf.skip_tcls_pip(tcls::CCM, rel, rel_test);
            for &src in muxes[&rel_test].src.keys() {
                vrf.inject_tcls_pip(tcls::CCM, rel, src.tw);
            }
        }
    }

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
        for slot in tcls.bels.ids() {
            verify_bel(endev.edev, &mut vrf, tcrd.bel(slot));
        }
    }
    verify_extra(endev, &mut vrf);
    vrf.finish();
}
