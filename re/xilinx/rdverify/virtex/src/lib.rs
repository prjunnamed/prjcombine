use prjcombine_entity::EntityId;
use prjcombine_interconnect::{db::WireSlotIdExt, grid::BelCoord};
use prjcombine_re_xilinx_naming_virtex::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::Verifier;
use prjcombine_virtex::{
    defs::{
        bcls::{self, BUFGCE, IOI},
        bslots, tcls, wire_to_mux, wires,
    },
    expanded::ExpandedDevice,
};

fn verify_slice(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::SLICE.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .extra_out_claim("COUT")
        .extra_in_claim("CIN")
        .extra_out_claim("F5")
        .extra_in_claim("F5IN");
    if let Some(obel) = bel.vrf.grid.bel_delta(bcrd.cell, 0, -1, bcrd.slot) {
        bel.verify_net(&[bel.wire("CIN"), bel.bel_wire_far(obel, "COUT")]);
    }
    bel.claim_pip(bel.wire_far("COUT"), bel.wire("COUT"));

    let obel = bcrd.bel(bslots::SLICE[idx ^ 1]);
    bel.claim_pip(bel.wire("F5IN"), bel.bel_wire(obel, "F5"));
    bel.commit();
}

fn verify_iob(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IOI.index_of(bcrd.slot).unwrap();
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("IOB")
        .rename_in(IOI::ICLK, "CLK")
        .skip_in(IOI::OCLK)
        .skip_in(IOI::TCLK);
    if idx == 0 || (idx == 3 && (bcrd.row == edev.chip.row_s() || bcrd.row == edev.chip.row_n())) {
        bel = bel.kind("EMPTYIOB");
    }
    if (bcrd.col == edev.chip.col_w() || bcrd.col == edev.chip.col_e())
        && ((bcrd.row == edev.chip.row_mid() && idx == 3)
            || (bcrd.row == edev.chip.row_mid() - 1 && idx == 1))
    {
        bel = bel.kind("PCIIOB").extra_out("PCI");
    }
    if edev.chip.kind.is_virtexe()
        && ((bcrd.col == edev.chip.col_clk() && idx == 2)
            || (bcrd.col == edev.chip.col_clk() - 1 && idx == 1))
    {
        bel = bel.kind("DLLIOB").extra_out("DLLFB");
    }
    bel.commit();
}

fn verify_tbus(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd);
    let obel = bcrd.bel(bslots::TBUF[0]);
    bel.claim_pip(bel.wire("BUS0"), bel.bel_wire(obel, "O"));
    bel.claim_pip(bel.wire("BUS2"), bel.bel_wire(obel, "O"));
    let obel = bcrd.bel(bslots::TBUF[1]);
    bel.claim_pip(bel.wire("BUS1"), bel.bel_wire(obel, "O"));
    bel.claim_pip(bel.wire("BUS3"), bel.bel_wire(obel, "O"));

    bel.claim_pip(bel.wire("OUT"), bel.wire("BUS2"));

    let col_e = edev.chip.col_e();
    if bcrd.col.to_idx() < col_e.to_idx() - 5 {
        bel.claim_net(&[bel.wire("BUS3_E")]);
    }
    bel.claim_pip(bel.wire("BUS3"), bel.wire("BUS3_E"));
    bel.claim_pip(bel.wire("BUS3_E"), bel.wire("BUS3"));
    let scol = if edev.chip.cols_bram.contains(&(bcrd.col + 1)) {
        bcrd.col + 2
    } else {
        bcrd.col + 1
    };
    let obel = bcrd.with_col(scol).bel(if scol == edev.chip.col_e() {
        bslots::TBUS_WE
    } else {
        bslots::TBUS
    });
    bel.verify_net(&[bel.wire("BUS0"), bel.bel_wire(obel, "BUS1")]);
    bel.verify_net(&[bel.wire("BUS1"), bel.bel_wire(obel, "BUS2")]);
    bel.verify_net(&[bel.wire("BUS2"), bel.bel_wire(obel, "BUS3")]);
    bel.verify_net(&[bel.wire("BUS3_E"), bel.bel_wire(obel, "BUS0")]);
}

fn verify_tbus_we(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd);
    let obel = bcrd.bel(bslots::TBUF[0]);
    bel.claim_pip(bel.wire("BUS0"), bel.bel_wire(obel, "O"));
    bel.claim_pip(bel.wire("BUS2"), bel.bel_wire(obel, "O"));
    let obel = bcrd.bel(bslots::TBUF[1]);
    bel.claim_pip(bel.wire("BUS1"), bel.bel_wire(obel, "O"));
    bel.claim_pip(bel.wire("BUS3"), bel.bel_wire(obel, "O"));

    if bcrd.col == edev.chip.col_w() {
        bel.claim_net(&[bel.wire("BUS3_E")]);
        bel.claim_pip(bel.wire("BUS3"), bel.wire("BUS3_E"));
        bel.claim_pip(bel.wire("BUS3_E"), bel.wire("BUS3"));
        let obel = bcrd.delta(2, 0).bel(bslots::TBUS);
        bel.verify_net(&[bel.wire("BUS0"), bel.bel_wire(obel, "BUS1")]);
        bel.verify_net(&[bel.wire("BUS1"), bel.bel_wire(obel, "BUS2")]);
        bel.verify_net(&[bel.wire("BUS2"), bel.bel_wire(obel, "BUS3")]);
        bel.verify_net(&[bel.wire("BUS3_E"), bel.bel_wire(obel, "BUS0")]);
    }
}

fn verify_bufg(vrf: &mut Verifier, bcrd: BelCoord) {
    vrf.verify_bel(bcrd)
        .kind("GCLK")
        .rename_in(BUFGCE::I, "IN")
        .rename_out(BUFGCE::O, "OUT")
        .commit();
}

fn verify_iofb(vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IOFB.index_of(bcrd.slot).unwrap();
    let mut bel = vrf.verify_bel(bcrd);
    let obel = match idx {
        0 => bcrd.bel(bslots::IOI[2]),
        1 => bcrd.delta(-1, 0).bel(bslots::IOI[1]),
        _ => unreachable!(),
    };
    bel.verify_net(&[bel.wire("I"), bel.bel_wire(obel, "DLLFB")]);
}

fn verify_pcilogic(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .extra_in_claim("IRDY")
        .extra_in_claim("TRDY");
    for pin in ["IRDY", "TRDY"] {
        bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
        bel.claim_net(&[bel.wire_far(pin)]);
    }
    let obel = bcrd.with_row(edev.chip.row_mid()).bel(bslots::IOI[3]);
    bel.verify_net(&[bel.wire_far("IRDY"), bel.bel_wire(obel, "PCI")]);
    let obel = bcrd.with_row(edev.chip.row_mid() - 1).bel(bslots::IOI[1]);
    bel.verify_net(&[bel.wire_far("TRDY"), bel.bel_wire(obel, "PCI")]);
    bel.commit();
}

fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    match bcrd.slot {
        bslots::INT
        | bslots::CLK_INT
        | bslots::DLL_INT
        | bslots::PCI_INT
        | bslots::MISC_SW
        | bslots::MISC_SE
        | bslots::MISC_NW
        | bslots::MISC_NE
        | bslots::GLOBAL => (),
        _ if bslots::SLICE.contains(bcrd.slot) => verify_slice(vrf, bcrd),
        _ if bslots::IOI.contains(bcrd.slot) => verify_iob(edev, vrf, bcrd),
        _ if bslots::IOB.contains(bcrd.slot) => (),
        _ if bslots::TBUF.contains(bcrd.slot) => {
            vrf.verify_bel(bcrd).extra_out_claim("O").commit();
        }
        bslots::TBUS => verify_tbus(edev, vrf, bcrd),
        bslots::TBUS_WE => verify_tbus_we(edev, vrf, bcrd),
        bslots::BRAM => vrf.verify_bel(bcrd).kind("BLOCKRAM").commit(),
        bslots::STARTUP | bslots::CAPTURE | bslots::BSCAN | bslots::DLL => {
            vrf.verify_bel(bcrd).commit()
        }
        _ if bslots::GCLK_IOB.contains(bcrd.slot) => vrf
            .verify_bel(bcrd)
            .kind("GCLKIOB")
            .rename_out(bcls::GCLK_IOB::I, "GCLKOUT")
            .commit(),
        _ if bslots::BUFGCE.contains(bcrd.slot) => verify_bufg(vrf, bcrd),
        _ if bslots::IOFB.contains(bcrd.slot) => verify_iofb(vrf, bcrd),
        bslots::PCILOGIC => verify_pcilogic(edev, vrf, bcrd),
        _ => unreachable!(),
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    let mut vrf = Verifier::new(rd, &endev.ngrid);

    for w in endev.edev.db.wires.ids() {
        if let Some(wm) = wire_to_mux(w) {
            vrf.alias_wire_slot(wm, w);
        }
    }

    for i in 0..4 {
        vrf.mark_merge_pip("CLKC", &format!("CLKC_HGCLK{i}"), &format!("CLKC_GCLK{i}"));
        vrf.mark_merge_pip("CLKC", &format!("CLKC_VGCLK{i}"), &format!("CLKC_HGCLK{i}"));
        vrf.mark_merge_pip(
            "GCLKC",
            &format!("GCLKC_VGCLK{i}"),
            &format!("GCLKC_HGCLK{i}"),
        );
        vrf.mark_merge_pip(
            "BRAM_CLKH",
            &format!("BRAM_CLKH_VGCLK{i}"),
            &format!("BRAM_CLKH_GCLK{i}"),
        );
    }

    for tcid in [tcls::IO_W, tcls::IO_E, tcls::IO_S, tcls::IO_N] {
        vrf.skip_bel_output(tcid, bslots::IOI[0], bcls::IOI::I);
        vrf.skip_bel_output(tcid, bslots::IOI[0], bcls::IOI::IQ);
    }
    for tcid in [tcls::IO_S, tcls::IO_N] {
        vrf.skip_bel_output(tcid, bslots::IOI[3], bcls::IOI::I);
        vrf.skip_bel_output(tcid, bslots::IOI[3], bcls::IOI::IQ);
    }

    for w in [
        wires::IMUX_CLB_CLK,
        wires::IMUX_CLB_CE,
        wires::IMUX_CLB_SR,
        wires::IMUX_CLB_BX,
        wires::IMUX_CLB_BY,
        wires::IMUX_TBUF_I,
        wires::IMUX_TBUF_T,
    ] {
        for w in w {
            vrf.skip_tcls_pip(tcls::CLB, w.cell(0), wires::PULLUP.cell(0));
        }
    }
    for w in [wires::IMUX_TBUF_I, wires::IMUX_TBUF_T] {
        for w in w {
            vrf.skip_tcls_pip(tcls::IO_W, w.cell(0), wires::PULLUP.cell(0));
            vrf.skip_tcls_pip(tcls::IO_E, w.cell(0), wires::PULLUP.cell(0));
        }
    }
    for w in [
        wires::IMUX_IO_CLK,
        wires::IMUX_IO_SR,
        wires::IMUX_IO_ICE,
        wires::IMUX_IO_OCE,
        wires::IMUX_IO_TCE,
        wires::IMUX_IO_O,
        wires::IMUX_IO_T,
    ] {
        for w in w {
            vrf.skip_tcls_pip(tcls::IO_W, w.cell(0), wires::PULLUP.cell(0));
            vrf.skip_tcls_pip(tcls::IO_E, w.cell(0), wires::PULLUP.cell(0));
            vrf.skip_tcls_pip(tcls::IO_S, w.cell(0), wires::PULLUP.cell(0));
            vrf.skip_tcls_pip(tcls::IO_N, w.cell(0), wires::PULLUP.cell(0));
        }
    }
    for w in [wires::IMUX_CAP_CLK, wires::IMUX_CAP_CAP] {
        vrf.skip_tcls_pip(tcls::CNR_SW, w.cell(0), wires::PULLUP.cell(0));
        vrf.skip_tcls_pip(tcls::CNR_SW_S2, w.cell(0), wires::PULLUP.cell(0));
    }
    for w in [
        wires::IMUX_STARTUP_CLK,
        wires::IMUX_STARTUP_GTS,
        wires::IMUX_STARTUP_GSR,
        wires::IMUX_STARTUP_GWE,
        wires::IMUX_BSCAN_TDO1,
        wires::IMUX_BSCAN_TDO2,
    ] {
        vrf.skip_tcls_pip(tcls::CNR_NW, w.cell(0), wires::PULLUP.cell(0));
        vrf.skip_tcls_pip(tcls::CNR_NW_S2, w.cell(0), wires::PULLUP.cell(0));
    }
    for tcid in [
        tcls::BRAM_W,
        tcls::BRAM_E,
        tcls::BRAM_W_S2,
        tcls::BRAM_E_S2,
        tcls::BRAM_M,
    ] {
        for w in [
            wires::IMUX_BRAM_WEA,
            wires::IMUX_BRAM_WEB,
            wires::IMUX_BRAM_RSTA,
            wires::IMUX_BRAM_RSTB,
            wires::IMUX_BRAM_SELA,
            wires::IMUX_BRAM_SELB,
        ] {
            vrf.skip_tcls_pip(tcid, w.cell(0), wires::PULLUP.cell(0));
        }
    }
    for tcid in [
        tcls::DLL_S,
        tcls::DLL_N,
        tcls::DLLP_S,
        tcls::DLLP_N,
        tcls::DLLS_S,
        tcls::DLLS_N,
    ] {
        vrf.skip_tcls_pip(tcid, wires::IMUX_DLL_RST.cell(0), wires::PULLUP.cell(0));
    }
    for tcid in [
        tcls::CLK_S_V,
        tcls::CLK_N_V,
        tcls::CLK_S_VE_2DLL,
        tcls::CLK_N_VE_2DLL,
        tcls::CLK_S_VE_4DLL,
        tcls::CLK_N_VE_4DLL,
    ] {
        for w in wires::IMUX_BUFGCE_CE {
            vrf.skip_tcls_pip(tcid, w.cell(1), wires::PULLUP.cell(1));
        }
    }

    vrf.prep_int_wires();
    vrf.handle_int();
    for (tcrd, tile) in endev.edev.tiles() {
        let tcls = &endev.edev.db[tile.class];
        for slot in tcls.bels.ids() {
            verify_bel(endev.edev, &mut vrf, tcrd.bel(slot));
        }
    }

    vrf.kill_stub_in("LEFT_I0");
    vrf.kill_stub_in("LEFT_IQ0");
    vrf.kill_stub_in("RIGHT_I0");
    vrf.kill_stub_in("RIGHT_IQ0");
    vrf.kill_stub_in("BOT_I0");
    vrf.kill_stub_in("BOT_IQ0");
    vrf.kill_stub_in("BOT_I3");
    vrf.kill_stub_in("BOT_IQ3");
    vrf.kill_stub_in("TOP_I0");
    vrf.kill_stub_in("TOP_IQ0");
    vrf.kill_stub_in("TOP_I3");
    vrf.kill_stub_in("TOP_IQ3");

    vrf.finish();
}
