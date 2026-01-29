use std::collections::HashMap;

use prjcombine_entity::EntityBundleItemIndex;
use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord},
    grid::{BelCoord, EdgeIoCoord},
};
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{LegacyBelContext, RawWireCoord, SitePinDir, Verifier};
use prjcombine_virtex2::{
    chip::ChipKind,
    defs::{self, bcls, bslots},
};

mod clb;
mod clk;
mod io;

fn get_bel_iob<'a>(
    endev: &ExpandedNamedDevice,
    vrf: &Verifier<'a>,
    crd: EdgeIoCoord,
) -> LegacyBelContext<'a> {
    vrf.get_legacy_bel(endev.chip.get_io_loc(crd))
}

fn verify_rll(vrf: &mut Verifier, bcrd: BelCoord) {
    let bel = &vrf.get_legacy_bel(bcrd);
    let mut pins = Vec::new();
    if bel.info.pins.is_empty() {
        for pin in bel.naming.pins.keys() {
            pins.push((&**pin, SitePinDir::In));
            vrf.claim_net(&[bel.wire(pin)]);
        }
    }
    vrf.verify_legacy_bel(bel, "RESERVED_LL", &pins, &[]);
}

fn verify_gt(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .extra_in("RXP")
        .extra_in("RXN")
        .extra_out("TXP")
        .extra_out("TXN");
    if endev.chip.kind == ChipKind::Virtex2PX {
        bel = bel.extra_in("BREFCLKPIN").extra_in("BREFCLKNIN");
        let (slot_p, slot_n) = if bcrd.row == endev.edev.chip.row_s() {
            (bslots::IOI[2], bslots::IOI[3])
        } else {
            (bslots::IOI[0], bslots::IOI[1])
        };
        for (pin, oslot) in [("BREFCLKPIN", slot_p), ("BREFCLKNIN", slot_n)] {
            bel.claim_net(&[bel.wire(pin)]);
            bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
            let obel = bcrd.with_col(endev.chip.col_clk - 1).bel(oslot);
            bel.verify_net(&[bel.wire_far(pin), bel.bel_wire_far(obel, "I")]);
        }
    } else {
        bel = bel
            .extra_in("BREFCLK")
            .extra_in("BREFCLK2")
            .extra_out("TST10B8BICRD0")
            .extra_out("TST10B8BICRD1");
        let obel = bcrd.with_col(endev.chip.col_clk).bel(bslots::BREFCLK);
        for pin in ["BREFCLK", "BREFCLK2"] {
            bel.claim_net(&[bel.wire(pin)]);
            bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
            bel.verify_net(&[bel.wire_far(pin), bel.bel_wire(obel, pin)]);
        }
        bel.claim_net(&[bel.wire("TST10B8BICRD0")]);
        bel.claim_net(&[bel.wire("TST10B8BICRD1")]);
    }
    for pin in ["RXP", "RXN"] {
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_net(&[bel.wire(&format!("I_{pin}"))]);
        bel.claim_pip(bel.wire(pin), bel.wire(&format!("I_{pin}")));
    }
    for pin in ["TXP", "TXN"] {
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_net(&[bel.wire(&format!("O_{pin}"))]);
        bel.claim_pip(bel.wire(&format!("O_{pin}")), bel.wire(pin));
    }
    bel.commit();

    for (pin, sub) in [("I_RXP", 1), ("I_RXN", 2)] {
        vrf.verify_bel(bcrd)
            .sub(sub)
            .kind("GTIPAD")
            .skip_auto()
            .extra_out_rename("I", pin)
            .commit();
    }
    for (pin, sub) in [("O_TXP", 3), ("O_TXN", 4)] {
        vrf.verify_bel(bcrd)
            .sub(sub)
            .kind("GTOPAD")
            .skip_auto()
            .extra_in_rename("O", pin)
            .commit();
    }
}

fn verify_bram(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bcrd: BelCoord) {
    let kind = match endev.chip.kind {
        ChipKind::Spartan3A => "RAMB16BWE",
        ChipKind::Spartan3ADsp => "RAMB16BWER",
        _ => "RAMB16",
    };
    let mut bel = vrf.verify_bel(bcrd).kind(kind);
    if endev.chip.kind != ChipKind::Spartan3ADsp {
        bel = bel
            .rename_in(bcls::BRAM::RSTA, "SSRA")
            .rename_in(bcls::BRAM::RSTB, "SSRB");
    }
    if !endev.chip.kind.is_spartan3a() {
        bel = bel
            .rename_in(bcls::BRAM::WEA[0], "WEA")
            .rename_in(bcls::BRAM::WEB[0], "WEB");
    }
    bel.commit();
}

fn verify_mult(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd);
    if matches!(endev.chip.kind, ChipKind::Spartan3E | ChipKind::Spartan3A) {
        let carry: Vec<_> = (0..18)
            .map(|x| (format!("BCOUT{x}"), format!("BCIN{x}")))
            .collect();
        let mut pins = vec![];
        for (o, i) in &carry {
            pins.push((&**o, SitePinDir::Out));
            pins.push((&**i, SitePinDir::In));
        }
        for i in 0..18 {
            bel = bel
                .extra_in(format!("BCIN{i}"))
                .extra_out(format!("BCOUT{i}"));
            bel.claim_net(&[bel.wire(&format!("BCIN{i}"))]);
            bel.claim_net(&[bel.wire(&format!("BCOUT{i}"))]);
        }
        if let Some(obel) = endev.edev.bel_carry_prev(bcrd) {
            for i in 0..18 {
                let co = &format!("BCOUT{i}");
                let ci = &format!("BCIN{i}");
                bel.verify_net(&[bel.wire(ci), bel.bel_wire_far(obel, co)]);
                bel.claim_pip(bel.bel_wire_far(obel, co), bel.bel_wire(obel, co));
            }
        }
        bel.kind("MULT18X18SIO").commit();
    } else {
        bel.rename_in(bcls::MULT::CEP, "CE")
            .rename_in(bcls::MULT::RSTP, "RST")
            .kind("MULT18X18")
            .commit();
    }
}

fn verify_mult_int(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bcrd: BelCoord) {
    let edev = endev.edev;
    let tcrd = edev.get_tile_by_bel(bcrd);
    let bcrd_bram = bcrd.bel(bslots::BRAM);
    let bel_bram = vrf.verify_bel(bcrd_bram);
    let mut wire_to_name = HashMap::new();
    for pid in bcls::BRAM::DOA
        .into_iter()
        .chain(bcls::BRAM::DOPA)
        .chain(bcls::BRAM::DOB)
        .chain(bcls::BRAM::DOPB)
    {
        let wire = edev.get_bel_output(bcrd_bram, pid)[0];
        let (pname, idx) = edev.db[bcls::BRAM].outputs.key(pid);
        let EntityBundleItemIndex::Array { index, .. } = idx else {
            unreachable!()
        };
        let pname = format!("{pname}{index}");
        let name = bel_bram.wire(&pname);
        wire_to_name.insert(wire, name);
    }
    let crd = bel_bram.crd();
    let BelInfo::SwitchBox(ref sb) = edev.db[edev[tcrd].class].bels[bcrd.slot] else {
        unreachable!()
    };
    let ntile = &endev.ngrid.tiles[&tcrd];
    let ntcls = &endev.ngrid.db.tile_class_namings[ntile.naming];
    for item in &sb.items {
        let SwitchBoxItem::Mux(mux) = item else {
            unreachable!()
        };
        let rw_dst = RawWireCoord {
            crd,
            wire: &ntcls.wires[&mux.dst].name,
        };
        vrf.pin_int_wire(rw_dst, edev.resolve_tile_wire(tcrd, mux.dst).unwrap());
        for src in mux.src.keys() {
            let wire_src = edev.resolve_tile_wire(tcrd, src.tw).unwrap();
            if let Some(&name) = wire_to_name.get(&wire_src) {
                vrf.claim_pip(rw_dst, name);
            } else {
                let rw_src = RawWireCoord {
                    crd,
                    wire: &ntcls.wires[&src.tw].name,
                };
                vrf.pin_int_wire(rw_src, wire_src);
                vrf.claim_pip(rw_dst, rw_src);
            }
        }
    }
}

fn verify_dsp(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).kind("DSP48A");
    let carry: Vec<_> = (0..18)
        .map(|x| (format!("BCOUT{x}"), format!("BCIN{x}")))
        .chain((0..48).map(|x| (format!("PCOUT{x}"), format!("PCIN{x}"))))
        .chain([("CARRYOUT".to_string(), "CARRYIN".to_string())])
        .collect();
    for (o, i) in &carry {
        bel = bel.extra_out(o).extra_in(i);
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

fn verify_bel(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bcrd: BelCoord) {
    let slot_name = endev.edev.db.bel_slots.key(bcrd.slot);
    match bcrd.slot {
        bslots::INT
        | bslots::TERM_W
        | bslots::TERM_E
        | bslots::TERM_S
        | bslots::TERM_N
        | bslots::CLK_INT
        | bslots::PPC_TERM_W
        | bslots::PPC_TERM_E
        | bslots::PPC_TERM_S
        | bslots::PPC_TERM_N
        | bslots::LLH
        | bslots::INTF_TESTMUX
        | bslots::DSP_TESTMUX
        | bslots::LLV => (),
        bslots::RLL => verify_rll(vrf, bcrd),
        _ if bslots::SLICE.contains(bcrd.slot) => {
            if endev.chip.kind.is_virtex2() {
                clb::verify_slice_v2(endev, vrf, bcrd);
            } else {
                clb::verify_slice_s3(endev, vrf, bcrd);
            }
        }
        _ if bslots::TBUF.contains(bcrd.slot) => {
            let mut bel = vrf.verify_bel(bcrd).extra_out("O");
            bel.claim_net(&[bel.wire("O")]);
            bel.commit();
        }
        bslots::TBUS => {
            clb::verify_tbus(endev, vrf, bcrd);
        }
        bslots::RANDOR => clb::verify_randor(endev, vrf, bcrd),
        bslots::RANDOR_OUT | bslots::RANDOR_INIT => (),

        bslots::BRAM => verify_bram(endev, vrf, bcrd),
        bslots::MULT => verify_mult(endev, vrf, bcrd),
        bslots::MULT_INT => verify_mult_int(endev, vrf, bcrd),
        bslots::DSP => verify_dsp(endev, vrf, bcrd),

        _ if slot_name.starts_with("IO") => io::verify_ioi(endev, vrf, bcrd),
        _ if slot_name.starts_with("IBUF") => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "IBUF", &[], &[]);
        }
        _ if slot_name.starts_with("OBUF") => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "OBUF", &[], &[]);
        }
        bslots::BREFCLK_INT => {
            let bel = &vrf.get_legacy_bel(bcrd);
            let slot = if bel.row == endev.edev.chip.row_s() {
                bslots::IOI[2]
            } else {
                bslots::IOI[0]
            };
            let obel = vrf.find_bel_sibling(bel, slot);
            vrf.claim_pip(bel.wire("BREFCLK"), obel.wire_far("I"));
        }
        bslots::PCILOGICSE => io::verify_pcilogicse(endev, vrf, bcrd),
        bslots::PCI_CE_N => io::verify_pci_ce_n(endev, vrf, bcrd),
        bslots::PCI_CE_S => io::verify_pci_ce_s(endev, vrf, bcrd),
        bslots::PCI_CE_E => io::verify_pci_ce_e(endev, vrf, bcrd),
        bslots::PCI_CE_W => io::verify_pci_ce_w(endev, vrf, bcrd),
        bslots::PCI_CE_CNR => io::verify_pci_ce_cnr(endev, vrf, bcrd),

        bslots::BREFCLK => clk::verify_brefclk(endev, vrf, bcrd),
        _ if slot_name.starts_with("BUFGMUX") => clk::verify_bufgmux(endev, vrf, bcrd),
        bslots::HCLK => clk::verify_hclk(endev, vrf, bcrd),
        bslots::HROW => clk::verify_hrow(endev, vrf, bcrd),
        bslots::CLKC => {
            if endev.chip.kind.is_virtex2() {
                clk::verify_clkc_v2(endev, vrf, bcrd);
            } else {
                clk::verify_clkc_s3(endev, vrf, bcrd);
            }
        }
        bslots::CLKC_50A => clk::verify_clkc_50a(endev, vrf, bcrd),
        bslots::CLKQC => clk::verify_gclkvm(endev, vrf, bcrd),
        bslots::DCMCONN_S3E => (),
        bslots::DCMCONN => clk::verify_dcmconn(endev, vrf, bcrd),

        bslots::GT | bslots::GT10 => verify_gt(endev, vrf, bcrd),
        bslots::STARTUP
        | bslots::CAPTURE
        | bslots::SPI_ACCESS
        | bslots::BSCAN
        | bslots::JTAGPPC
        | bslots::PMV
        | bslots::DNA_PORT => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, slot_name, &[], &[]);
        }
        bslots::PCILOGIC => vrf.verify_bel(bcrd).commit(),
        bslots::PPC405 => {
            vrf.verify_bel(bcrd).commit();
        }
        bslots::DCM => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "DCM", &[], &[]);
            if endev.chip.kind.is_virtex2p() {
                // just some detritus.
                let data29 = RawWireCoord {
                    crd: bel.crd(),
                    wire: "BRAM_IOIS_DATA29",
                };
                let vcc = RawWireCoord {
                    crd: bel.crd(),
                    wire: "BRAM_IOIS_VCC_WIRE",
                };
                vrf.claim_net(&[data29]);
                vrf.claim_pip(data29, vcc);
            }
        }
        bslots::ICAP => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "ICAP", &[], &[]);
            if endev.chip.kind == ChipKind::Spartan3E {
                // eh.
                vrf.claim_net(&[bel.wire("I2")]);
            }
        }
        _ if slot_name.starts_with("GLOBALSIG") => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "GLOBALSIG", &[], &[]);
        }
        _ if bslots::DCIRESET.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "DCIRESET", &[], &[]);
        }
        _ if bslots::DCI.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "DCI", &[], &[]);
        }
        _ if slot_name.starts_with("PTE2OMUX") => {
            let bel = &vrf.get_legacy_bel(bcrd);
            let out = bel.wire("OUT");
            for (k, v) in &bel.naming.pins {
                if k == "OUT" {
                    continue;
                }
                vrf.claim_pip(
                    out,
                    RawWireCoord {
                        crd: bel.crd(),
                        wire: &v.name,
                    },
                );
            }
        }
        bslots::VCC => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "VCC", &[("VCCOUT", SitePinDir::Out)], &[]);
            vrf.claim_net(&[bel.wire("VCCOUT")]);
        }
        bslots::MISR => (),

        _ => println!("MEOW {}", bcrd.to_string(endev.edev.db)),
    }
}

fn verify_extra(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    if endev.chip.kind.is_spartan3ea() {
        vrf.kill_stub_out("IOIS_STUB_F1_B3");
        vrf.kill_stub_out("IOIS_STUB_F2_B3");
        vrf.kill_stub_out("IOIS_STUB_F3_B3");
        vrf.kill_stub_out("IOIS_STUB_F4_B3");
        vrf.kill_stub_out("IOIS_STUB_G1_B3");
        vrf.kill_stub_out("IOIS_STUB_G2_B3");
        vrf.kill_stub_out("IOIS_STUB_G3_B3");
        vrf.kill_stub_out("IOIS_STUB_G4_B3");
        vrf.kill_stub_out("IOIS_STUB_F4_B0");
        vrf.kill_stub_out("IOIS_STUB_F4_B1");
        vrf.kill_stub_out("IOIS_STUB_F4_B2");
        vrf.kill_stub_in("STUB_IOIS_X3");
        vrf.kill_stub_in("STUB_IOIS_Y3");
        vrf.kill_stub_in("STUB_IOIS_XQ3");
        vrf.kill_stub_in("STUB_IOIS_YQ3");
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);
    if endev.chip.kind.is_virtex2() {
        for (wt, wf) in [
            (
                defs::virtex2::wires::IMUX_CLK_OPTINV.as_slice(),
                defs::virtex2::wires::IMUX_CLK.as_slice(),
            ),
            (
                defs::virtex2::wires::IMUX_DCM_CLK_OPTINV.as_slice(),
                defs::virtex2::wires::IMUX_DCM_CLK.as_slice(),
            ),
            (
                defs::virtex2::wires::IMUX_SR_OPTINV.as_slice(),
                defs::virtex2::wires::IMUX_SR.as_slice(),
            ),
            (
                defs::virtex2::wires::IMUX_CE_OPTINV.as_slice(),
                defs::virtex2::wires::IMUX_CE.as_slice(),
            ),
            (
                defs::virtex2::wires::IMUX_TS_OPTINV.as_slice(),
                defs::virtex2::wires::IMUX_TS.as_slice(),
            ),
            (
                defs::virtex2::wires::IMUX_TI_OPTINV.as_slice(),
                defs::virtex2::wires::IMUX_TI.as_slice(),
            ),
        ] {
            for (&wt, &wf) in wt.iter().zip(wf) {
                vrf.alias_wire_slot(wt, wf);
            }
        }
        for wt in [
            defs::virtex2::wires::IMUX_CE[0],
            defs::virtex2::wires::IMUX_CE[1],
            defs::virtex2::wires::IMUX_TS[0],
            defs::virtex2::wires::IMUX_TS[1],
        ] {
            vrf.inject_tcls_pip(
                defs::virtex2::tcls::INT_GT_CLKPAD,
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, defs::virtex2::wires::PULLUP),
            );
        }
    } else {
        for (wt, wf) in [
            (
                defs::spartan3::wires::IMUX_CLK_OPTINV.as_slice(),
                defs::spartan3::wires::IMUX_CLK.as_slice(),
            ),
            (
                defs::spartan3::wires::IMUX_SR_OPTINV.as_slice(),
                defs::spartan3::wires::IMUX_SR.as_slice(),
            ),
            (
                defs::spartan3::wires::IMUX_CE_OPTINV.as_slice(),
                defs::spartan3::wires::IMUX_CE.as_slice(),
            ),
        ] {
            for (&wt, &wf) in wt.iter().zip(wf) {
                vrf.alias_wire_slot(wt, wf);
            }
        }
        if endev.chip.kind == ChipKind::Spartan3A {
            vrf.kill_stub_out_cond("BRAM_CE_B0");
            vrf.kill_stub_out_cond("BRAM_CE_B1");
            vrf.kill_stub_out_cond("BRAM_CE_B2");
            vrf.kill_stub_out_cond("BRAM_CE_B3");
            vrf.kill_stub_out_cond("BRAM_CLK0");
            vrf.kill_stub_out_cond("BRAM_CLK1");
            vrf.kill_stub_out_cond("BRAM_CLK2");
            vrf.kill_stub_out_cond("BRAM_CLK3");
        }
    }
    vrf.skip_sb(bslots::MULT_INT);
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
