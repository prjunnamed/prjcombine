use std::collections::HashMap;

use prjcombine_entity::{EntityBundleItemIndex, EntityId};
use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord},
    grid::{BelCoord, EdgeIoCoord},
};
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{LegacyBelContext, RawWireCoord, SitePinDir, Verifier};
use prjcombine_virtex2::{
    chip::{ChipKind, ColumnKind},
    defs::{
        self, bcls, bslots,
        spartan3::{tcls as tcls_s3, wires as wires_s3},
    },
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
        for (idx, pin) in ["BREFCLK", "BREFCLK2"].into_iter().enumerate() {
            bel.claim_net(&[bel.wire(pin)]);
            bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
            let obel = bcrd
                .with_col(endev.chip.col_clk)
                .bel(bslots::GLOBALSIG_BUFG[idx]);
            bel.verify_net(&[bel.wire_far(pin), bel.bel_wire(obel, "BREFCLK_O")]);
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
        bslots::INT => {
            let crd = vrf.get_tile_crds(endev.edev.get_tile_by_bel(bcrd)).unwrap()
                [RawTileId::from_idx(0)];
            let name = endev.ngrid.get_bel_name(bcrd).unwrap();
            vrf.claim_site_dummy(crd, name);
        }
        bslots::TERM_W
        | bslots::TERM_E
        | bslots::TERM_S
        | bslots::TERM_N
        | bslots::CLK_INT
        | bslots::DCM_INT
        | bslots::PPC_TERM_W
        | bslots::PPC_TERM_E
        | bslots::PPC_TERM_S
        | bslots::PPC_TERM_N
        | bslots::LLH
        | bslots::LLV
        | bslots::PTE2OMUX
        | bslots::INTF_TESTMUX
        | bslots::DSP_TESTMUX
        | bslots::HROW
        | bslots::HCLK
        | bslots::DCMCONN => (),
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
        _ if bslots::IBUF.contains(bcrd.slot) => {
            let idx = bslots::IBUF.index_of(bcrd.slot).unwrap();
            let bel = &vrf.get_legacy_bel(bcrd);
            if (bcrd.col == endev.chip.col_clk - 1 || bcrd.col == endev.chip.col_clk) && idx < 2 {
                vrf.claim_pip(bel.wire("CLKPAD"), bel.wire("I"));
            }
            vrf.verify_legacy_bel(bel, "IBUF", &[], &["CLKPAD"]);
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

        _ if slot_name.starts_with("BUFGMUX") => clk::verify_bufgmux(endev, vrf, bcrd),
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
        bslots::GLOBALSIG_HCLK => {
            vrf.verify_bel(bcrd).kind("GLOBALSIG").commit();
            if endev.chip.columns[bcrd.col].kind == ColumnKind::Dsp {
                vrf.verify_bel(bcrd).sub(1).kind("GLOBALSIG").commit();
            }
        }
        _ if bslots::GLOBALSIG_BUFG.contains(bcrd.slot) => {
            clk::verify_globalsig_bufg(endev, vrf, bcrd);
        }
        _ if bslots::DCIRESET.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "DCIRESET", &[], &[]);
        }
        _ if bslots::DCI.contains(bcrd.slot) => {
            let bel = &vrf.get_legacy_bel(bcrd);
            vrf.verify_legacy_bel(bel, "DCI", &[], &[]);
        }
        bslots::MISR => (),

        _ => println!("MEOW {}", bcrd.to_string(endev.edev.db)),
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);
    vrf.kill_stub_out("LH0_TESTWIRE");
    vrf.kill_stub_out("LH6_TESTWIRE");
    vrf.kill_stub_out("LH12_TESTWIRE");
    vrf.kill_stub_out("LH18_TESTWIRE");
    vrf.kill_stub_out("LV0_TESTWIRE");
    vrf.kill_stub_out("LV6_TESTWIRE");
    vrf.kill_stub_out("LV12_TESTWIRE");
    vrf.kill_stub_out("LV18_TESTWIRE");
    if endev.chip.kind.is_virtex2() {
        for i in 0..8 {
            for tkn in ["BBTERM", "BGIGABIT_INT_TERM", "BGIGABIT10_INT_TERM"] {
                vrf.mark_merge_pip(
                    tkn,
                    &format!("BBTERM_CLKPAD{i}"),
                    &format!("BBTERM_DLL_CLKPAD{i}"),
                );
            }
            for tkn in ["BTTERM", "TGIGABIT_INT_TERM", "TGIGABIT10_INT_TERM"] {
                vrf.mark_merge_pip(
                    tkn,
                    &format!("BTTERM_CLKPAD{i}"),
                    &format!("BTTERM_DLL_CLKPAD{i}"),
                );
            }
        }
        for i in 0..8 {
            vrf.mark_merge_pip(
                "CLKC",
                &format!("CLKC_GCLKB{i}"),
                &format!("CLKC_GCLKB_IN{i}"),
            );
            vrf.mark_merge_pip(
                "CLKC",
                &format!("CLKC_GCLKT{i}"),
                &format!("CLKC_GCLKT_IN{i}"),
            );
        }
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
        for i in 0..8 {
            for tkn in ["CLKC", "CLKC_LL"] {
                vrf.mark_merge_pip(
                    tkn,
                    &format!("CLKC_GCLK{i}"),
                    &format!("CLKC_GCLK_MAIN_B{i}"),
                );
                vrf.mark_merge_pip(
                    tkn,
                    &format!("CLKC_GCLK{ii}", ii = i + 4),
                    &format!("CLKC_GCLK_MAIN_T{i}"),
                );
            }
            vrf.mark_merge_pip(
                "GCLKVC",
                &format!("GCLKC_GCLK_OUT_L{i}"),
                &format!("GCLKC_GCLK{i}"),
            );
            vrf.mark_merge_pip(
                "GCLKVC",
                &format!("GCLKC_GCLK_OUT_R{i}"),
                &format!("GCLKC_GCLK{i}"),
            );
        }
        for tkn in ["GCLKH_PCI_CE_S", "GCLKH_PCI_CE_S_50A"] {
            vrf.mark_merge_pip(tkn, "GCLKH_PCI_CE_IN", "GCLKH_PCI_CE_OUT");
        }
        vrf.mark_merge_pip("GCLKH_PCI_CE_N", "GCLKH_PCI_CE_OUT", "GCLKH_PCI_CE_IN");
        for tkn in ["LL", "LR", "UL", "UR"] {
            vrf.mark_merge_pip(tkn, "PCI_CE_EW", "PCI_CE_NS");
        }
        vrf.mark_merge_pip("GCLKV_IOISL", "CLKV_PCI_CE_E", "CLKV_PCI_CE_W");
        vrf.mark_merge_pip("GCLKV_IOISR", "CLKV_PCI_CE_W", "CLKV_PCI_CE_E");
        if endev.chip.kind == ChipKind::Spartan3 {
            for i in 0..4 {
                vrf.mark_merge_pip(
                    "BBTERM",
                    &format!("BBTERM_CLKPAD{i}"),
                    &format!("BBTERM_DLL_CLKPAD{i}"),
                );
                vrf.mark_merge_pip(
                    "BTTERM",
                    &format!("BTTERM_CLKPAD{i}"),
                    &format!("BTTERM_DLL_CLKPAD{i}"),
                );
            }
        } else {
            for tkn in ["CLKV", "CLKV_LL"] {
                vrf.mark_merge_pip(tkn, "CLKV_OUTR0", "CLKV_OMUX10_OUTR0");
                vrf.mark_merge_pip(tkn, "CLKV_OUTR1", "CLKV_OMUX11_OUTR1");
                vrf.mark_merge_pip(tkn, "CLKV_OUTR2", "CLKV_OMUX12_OUTR2");
                vrf.mark_merge_pip(tkn, "CLKV_OUTR3", "CLKV_OMUX15_OUTR3");
                vrf.mark_merge_pip(tkn, "CLKV_OUTL0", "CLKV_OMUX10_OUTL0");
                vrf.mark_merge_pip(tkn, "CLKV_OUTL1", "CLKV_OMUX11_OUTL1");
                vrf.mark_merge_pip(tkn, "CLKV_OUTL2", "CLKV_OMUX12_OUTL2");
                vrf.mark_merge_pip(tkn, "CLKV_OUTL3", "CLKV_OMUX15_OUTL3");
            }
        }
        if endev.chip.kind.is_spartan3a() {
            for i in 0..8 {
                for tkn in ["CLKL", "CLKR"] {
                    vrf.mark_merge_pip(tkn, &format!("{tkn}_CKI{i}_END"), &format!("{tkn}_CKI{i}"));
                }
            }
            for i in 0..8 {
                for tkn in ["CLKL_IOIS", "CLKL_IOIS_LL", "CLKL_IOIS_50A"] {
                    vrf.mark_merge_pip(
                        tkn,
                        &format!("CLKLH_GCLKH_MAIN_V{i}"),
                        &format!("CLKLH_GCLKH_MAIN{i}"),
                    );
                }
                for tkn in ["CLKR_IOIS", "CLKR_IOIS_LL", "CLKR_IOIS_50A"] {
                    vrf.mark_merge_pip(
                        tkn,
                        &format!("CLKRH_GCLKH_MAIN_V{i}"),
                        &format!("CLKRH_GCLKH_MAIN{i}"),
                    );
                }
            }
        }
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
        for tcid in [
            tcls_s3::CLK_W_S3E,
            tcls_s3::CLK_W_S3A,
            tcls_s3::CLK_E_S3E,
            tcls_s3::CLK_E_S3A,
        ] {
            for i in 0..4 {
                for c in [3, 4] {
                    vrf.inject_tcls_pip(
                        tcid,
                        TileWireCoord::new_idx(c, wires_s3::IMUX_DATA[3 + i * 4]),
                        TileWireCoord::new_idx(4, wires_s3::PULLUP),
                    );
                }
            }
            for &tcrd in &endev.edev.tile_index[tcid] {
                let BelInfo::SwitchBox(sb) = &endev.edev.db[tcid].bels[bslots::CLK_INT] else {
                    unreachable!()
                };
                for item in &sb.items {
                    let SwitchBoxItem::PermaBuf(buf) = item else {
                        continue;
                    };
                    let dst = endev.edev.tile_wire(tcrd, buf.dst);
                    let src = endev.edev.tile_wire(tcrd, buf.src.tw);
                    vrf.skip_tcls_pip(tcid, buf.dst, buf.src.tw);
                    vrf.alias_wire(dst, src);
                }
                for i in 0..8 {
                    let BelInfo::Bel(bel) = &endev.edev.db[tcid].bels[bslots::BUFGMUX[i]] else {
                        unreachable!()
                    };
                    let wires = Vec::from_iter(
                        bel.outputs[bcls::BUFGMUX::O]
                            .iter()
                            .map(|&wire| endev.edev.resolve_tile_wire(tcrd, wire).unwrap()),
                    );
                    assert_eq!(wires.len(), 2);
                    vrf.alias_wire(wires[0], wires[1]);
                }
            }
        }
        for &tcrd in &endev.edev.tile_index[tcls_s3::HCLK_UNI] {
            let ntile = &endev.ngrid.tiles[&tcrd];
            if endev.ngrid.db.tile_class_namings.key(ntile.naming) == "HCLK_BRAM" {
                let naming = &endev.ngrid.db.tile_class_namings[ntile.naming];
                let crd = vrf.get_tile_crds(tcrd).unwrap()[RawTileId::from_idx(0)];
                for i in 0..8 {
                    let wire_s = RawWireCoord {
                        crd,
                        wire: &naming.wires[&TileWireCoord::new_idx(0, wires_s3::GCLK[i])].name,
                    };
                    let wire_n = RawWireCoord {
                        crd,
                        wire: &naming.wires[&TileWireCoord::new_idx(1, wires_s3::GCLK[i])].name,
                    };
                    let wire_row = RawWireCoord {
                        crd,
                        wire: &naming.wires[&TileWireCoord::new_idx(1, wires_s3::GCLK_QUAD[i])]
                            .name,
                    };
                    vrf.claim_pip(wire_s, wire_row);
                    vrf.merge_node(wire_s, wire_n);
                }
            }
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
    vrf.finish();
}
