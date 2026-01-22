use std::collections::BTreeSet;

use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord},
    dir::DirHV,
    grid::BelCoord,
};
use prjcombine_re_xilinx_naming_xc2000::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{RawWireCoord, SitePinDir, Verifier};
use prjcombine_xc2000::{
    chip::ChipKind,
    xc4000::{bslots, wires, xc4000::bcls, xc4000::tcls},
};

fn verify_clb(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).extra_in("CIN").extra_out("COUT");
    bel.claim_net(&[bel.wire("COUT")]);
    if !endev.chip.kind.is_clb_xl() {
        bel.claim_pip(bel.wire("CIN_S"), bel.wire("COUT"));
        bel.claim_pip(bel.wire("CIN_N"), bel.wire("COUT"));
        bel.claim_net(&[bel.wire("CIN")]);
        let obel = bcrd.delta(0, -1).bel(bslots::CLB);
        if endev.edev.has_bel(obel) {
            bel.verify_net(&[bel.wire("CIN_S"), bel.bel_wire(obel, "CIN")]);
        } else {
            let obel = bcrd.delta(1, 0).bel(bslots::CLB);
            if endev.edev.has_bel(obel) {
                bel.verify_net(&[bel.wire("CIN_S"), bel.bel_wire(obel, "CIN")]);
            } else {
                let obel = bcrd.delta(1, -1).bel(bslots::MISC_SE);
                bel.verify_net(&[bel.wire("CIN_S"), bel.bel_wire(obel, "I")]);
            }
        }
        let obel = bcrd.delta(0, 1).bel(bslots::CLB);
        if endev.edev.has_bel(obel) {
            bel.verify_net(&[bel.wire("CIN_N"), bel.bel_wire(obel, "CIN")]);
        } else {
            let obel = bcrd.delta(1, 0).bel(bslots::CLB);
            if endev.edev.has_bel(obel) {
                bel.verify_net(&[bel.wire("CIN_N"), bel.bel_wire(obel, "CIN")]);
            } else {
                let obel = bcrd.delta(1, 1).bel(bslots::MISC_NE);
                bel.verify_net(&[bel.wire("CIN_N"), bel.bel_wire(obel, "I")]);
            }
        }
    } else {
        bel.claim_pip(bel.wire_far("COUT"), bel.wire("COUT"));
        let obel = bcrd.delta(0, -1).bel(bslots::CLB);
        if endev.edev.has_bel(obel) {
            bel.verify_net(&[bel.wire("CIN"), bel.bel_wire_far(obel, "COUT")]);
        } else {
            let obel = bcrd.delta(0, -1).bel(bslots::CIN);
            bel.verify_net(&[bel.wire("CIN"), bel.bel_wire(obel, "I")]);
        }
        let obel = bcrd.delta(0, 1).bel(bslots::COUT);
        if endev.edev.has_bel(obel) {
            bel.verify_net(&[bel.wire_far("COUT"), bel.bel_wire(obel, "O")]);
        } else {
            bel.claim_net(&[bel.wire_far("COUT")]);
        }
    }
    bel.commit();
}

fn verify_iob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let tcrd = endev.edev.get_tile_by_bel(bcrd);
    let tcls = &endev.edev.db[endev.edev[tcrd].class];
    let BelInfo::Bel(ref bel_info) = tcls.bels[bcrd.slot] else {
        unreachable!()
    };
    let mut bel = vrf
        .verify_bel(bcrd)
        .rename_in(bcls::IO::O1, "EC")
        .rename_in(bcls::IO::O2, "O");
    if !bel_info.outputs.contains_id(bcls::IO::I1) {
        bel = bel.kind("FCLKIOB");
    } else if bel_info.outputs.contains_id(bcls::IO::CLKIN) {
        bel = bel.kind("CLKIOB");
    } else if bel.naming.pins.contains_key("CLKIN") {
        bel.claim_net(&[bel.wire("CLKIN")]);
        bel = bel.kind("FCLKIOB").extra_out("CLKIN");
    } else {
        bel = bel.kind("IOB");
    }
    bel.claim_pip(bel.wire("O2"), bel.wire_far("O1"));
    bel.commit();
}

fn verify_tbuf(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .bidir_dir(bcls::TBUF::O, SitePinDir::Out);
    bel.claim_net(&[bel.wire("O")]);
    bel.claim_pip(bel.wire_far("O"), bel.wire("O"));
    if endev.chip.kind == ChipKind::Xc4000E {
        let tcrd = endev.edev.get_tile_by_bel(bcrd);
        let tcls_index = &endev.edev.db_index[endev.edev[tcrd].class];
        let naming = &endev.ngrid.db.tile_class_namings[bel.ntile.naming];
        let wire = endev.edev.get_bel_input(bcrd, bcls::TBUF::I).wire;
        let ins = &tcls_index.pips_bwd[&TileWireCoord::new_idx(0, wire.slot)];
        for &inp in ins {
            if inp.wire == wires::TIE_0 {
                continue;
            }
            bel.claim_pip(
                bel.wire_far("O"),
                RawWireCoord {
                    crd: bel.crd(),
                    wire: &naming.wires[&inp.tw].name,
                },
            );
        }
    }
    bel.commit();
}

fn verify_dec(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("DECODER")
        .bidir_dir(bcls::DEC::O1, SitePinDir::Out)
        .bidir_dir(bcls::DEC::O2, SitePinDir::Out)
        .bidir_dir(bcls::DEC::O3, SitePinDir::Out)
        .bidir_dir(bcls::DEC::O4, SitePinDir::Out);
    for pin in ["O1", "O2", "O3", "O4"] {
        bel.claim_pip(bel.wire_far(pin), bel.wire(pin));
        bel.claim_net(&[bel.wire(pin)]);
    }
    bel.commit();
}

fn verify_pullup(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .bidir_dir(bcls::PULLUP::O, SitePinDir::Out);
    bel.claim_net(&[bel.wire("O")]);
    bel.claim_pip(bel.wire_far("O"), bel.wire("O"));
    bel.commit();
}

fn verify_bufg(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    if !endev.chip.kind.is_xl() {
        let name = endev.ngrid.get_bel_name(bcrd).unwrap();
        let kind = if name.starts_with("BUFGP") {
            "PRI-CLK"
        } else if name.starts_with("BUFGS") {
            "SEC-CLK"
        } else {
            "BUFGLS"
        };
        vrf.verify_bel(bcrd).kind(kind).commit();
    } else {
        let mut bel = vrf
            .verify_bel(bcrd)
            .kind("BUFG")
            .skip_out(bcls::BUFG::O)
            .skip_out(bcls::BUFG::O_BUFGE)
            .extra_out_rename("O", "O_BUFG");
        bel.claim_net(&[bel.wire("O_BUFG")]);
        bel.claim_net(&[bel.wire("I_BUFGE")]);
        bel.claim_net(&[bel.wire("I_BUFGLS")]);
        bel.claim_pip(bel.wire("I_BUFGE"), bel.wire("O_BUFG"));
        bel.claim_pip(bel.wire("I_BUFGLS"), bel.wire("O_BUFG"));
        bel.commit();
        vrf.verify_bel(bcrd)
            .sub(1)
            .kind("BUFGE")
            .skip_in(bcls::BUFG::I)
            .skip_out(bcls::BUFG::O)
            .rename_out(bcls::BUFG::O_BUFGE, "O")
            .extra_in_rename("I", "I_BUFGE")
            .commit();
        vrf.verify_bel(bcrd)
            .sub(2)
            .kind("BUFGLS")
            .skip_in(bcls::BUFG::I)
            .skip_out(bcls::BUFG::O_BUFGE)
            .extra_in_rename("I", "I_BUFGLS")
            .commit();
    }
}

fn verify_osc(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .kind("OSCILLATOR")
        .extra_out("F15")
        .extra_out("F490")
        .extra_out("F16K")
        .extra_out("F500K")
        .skip_out(bcls::OSC::OUT0)
        .skip_out(bcls::OSC::OUT1);
    for pin in ["F15", "F490", "F16K", "F500K"] {
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.wire("OUT0"), bel.wire(pin));
        bel.claim_pip(bel.wire("OUT1"), bel.wire(pin));
    }
    bel.commit();
}

fn verify_cout(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    if endev.edev.chip.kind.is_clb_xl() {
        return;
    }
    let mut bel = vrf.verify_bel(bcrd).kind("COUT").extra_in("I");
    bel.claim_net(&[bel.wire("I")]);
    bel.commit();
}

fn verify_cin(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    if endev.edev.chip.kind.is_clb_xl() {
        return;
    }
    let mut bel = vrf.verify_bel(bcrd).kind("CIN").extra_out("O");
    bel.claim_net(&[bel.wire("O")]);
    bel.claim_pip(bel.wire_far("O"), bel.wire("O"));
    let obel = if bcrd.row == endev.edev.chip.row_s() {
        bcrd.delta(1, 1).bel(bslots::CLB)
    } else {
        bcrd.delta(1, -1).bel(bslots::CLB)
    };
    bel.verify_net(&[bel.wire_far("O"), bel.bel_wire(obel, "CIN")]);
    bel.commit();
}

fn verify_tbuf_splitter(vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd);
    for (po, pi) in [
        ("W", "E"),
        ("E", "W"),
        ("W_EXCL", "W"),
        ("W", "W_EXCL"),
        ("E_EXCL", "E"),
        ("E", "E_EXCL"),
        ("W_EXCL", "E_EXCL"),
        ("E_EXCL", "W_EXCL"),
    ] {
        bel.claim_pip(bel.wire(po), bel.wire(pi));
    }
    bel.claim_net(&[bel.wire("W_EXCL")]);
    bel.claim_net(&[bel.wire("E_EXCL")]);
}

fn verify_buff(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd).extra_in("I");
    bel.claim_net(&[bel.wire("I")]);
    bel.claim_pip(bel.wire("I"), bel.wire_far("I"));
    let obel = endev.edev.chip.bel_buff_io(DirHV {
        h: endev.edev.chip.col_side_of_mid(bcrd.col),
        v: endev.edev.chip.row_side_of_mid(bcrd.row),
    });
    bel.verify_net(&[bel.wire_far("I"), bel.bel_wire(obel, "CLKIN")]);
    bel.commit();
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: BelCoord) {
    let slot_name = endev.edev.db.bel_slots.key(bel.slot);
    match bel.slot {
        bslots::INT
        | bslots::LLH
        | bslots::LLV
        | bslots::MISC_W
        | bslots::MISC_E
        | bslots::COUT
        | bslots::CIN
        | bslots::CLKQ
        | bslots::CLKQC => (),
        bslots::CLB => verify_clb(endev, vrf, bel),
        _ if bslots::IO.contains(bel.slot) => verify_iob(endev, vrf, bel),
        _ if bslots::TBUF.contains(bel.slot) => verify_tbuf(endev, vrf, bel),
        _ if bslots::DEC.contains(bel.slot) => verify_dec(vrf, bel),
        _ if slot_name.starts_with("PULLUP") => verify_pullup(vrf, bel),
        bslots::BUFG_H | bslots::BUFG_V => verify_bufg(endev, vrf, bel),
        bslots::OSC => verify_osc(vrf, bel),
        bslots::TDO => vrf.verify_bel(bel).kind("TESTDATA").commit(),
        bslots::MD0 => {
            if endev.edev.chip.kind != ChipKind::SpartanXl {
                vrf.verify_bel(bel).kind("MODE0").commit();
            }
        }
        bslots::MD1 => {
            if endev.edev.chip.kind != ChipKind::SpartanXl {
                vrf.verify_bel(bel).kind("MODE1").commit();
            }
        }
        bslots::MD2 => {
            if endev.edev.chip.kind != ChipKind::SpartanXl {
                vrf.verify_bel(bel).kind("MODE2").commit();
            }
        }
        bslots::RDBK => vrf.verify_bel(bel).kind("READBACK").commit(),
        bslots::STARTUP | bslots::READCLK | bslots::UPDATE | bslots::BSCAN => {
            vrf.verify_bel(bel).commit();
        }
        bslots::MISC_SE | bslots::MISC_NE => verify_cout(endev, vrf, bel),
        bslots::MISC_SW | bslots::MISC_NW => verify_cin(endev, vrf, bel),
        _ if bslots::TBUF_SPLITTER.contains(bel.slot) => verify_tbuf_splitter(vrf, bel),
        bslots::BUFF => verify_buff(endev, vrf, bel),
        _ => println!("MEOW {bcrd}", bcrd = bel.to_string(endev.edev.db)),
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    {
        let mut vrf = Verifier::new(rd, &endev.ngrid);
        if endev.edev.chip.kind == ChipKind::SpartanXl {
            for tcid in [tcls::LLV_CLB, tcls::LLV_IO_W, tcls::LLV_IO_E] {
                let BelInfo::SwitchBox(ref sb) = endev.edev.db[tcid].bels[bslots::LLV] else {
                    unreachable!()
                };
                for item in &sb.items {
                    let SwitchBoxItem::Mux(mux) = item else {
                        continue;
                    };
                    for src in mux.src.keys() {
                        let idx = wires::BUFGLS_H.index_of(src.wire).unwrap();
                        vrf.skip_tcls_pip(tcid, mux.dst, src.tw);
                        vrf.inject_tcls_pip(
                            tcid,
                            mux.dst,
                            TileWireCoord::new_idx(0, wires::BUFGLS[idx]),
                        );
                    }
                }
                for idx in 0..8 {
                    vrf.skip_tcls_pip(
                        tcid,
                        TileWireCoord::new_idx(0, wires::BUFGLS_H[idx]),
                        TileWireCoord::new_idx(0, wires::BUFGLS[idx]),
                    );
                }
            }
        }
        if endev.edev.chip.kind == ChipKind::Xc4000Xv {
            for (tcid, tcname, tcls) in &endev.edev.db.tile_classes {
                let Some(BelInfo::SwitchBox(sb)) = tcls.bels.get(bslots::INT) else {
                    continue;
                };
                if !tcls.bels.contains_id(bslots::IO[0]) {
                    continue;
                }
                let mut obuf_outs = BTreeSet::new();
                let mut obuf_ins = BTreeSet::new();
                for item in &sb.items {
                    let SwitchBoxItem::ProgBuf(buf) = item else {
                        continue;
                    };
                    if buf.src.wire == wires::OBUF {
                        obuf_outs.insert(buf.dst);
                        vrf.skip_tcls_pip(tcid, buf.dst, buf.src.tw);
                    }
                    if buf.dst.wire == wires::OBUF {
                        obuf_ins.insert(buf.src);
                        vrf.skip_tcls_pip(tcid, buf.dst, buf.src.tw);
                    }
                }
                for &dst in &obuf_outs {
                    for &src in &obuf_ins {
                        if src.tw != dst {
                            vrf.inject_tcls_pip(tcid, dst, src.tw);
                        }
                    }
                }
                if tcname.starts_with("IO_W") {
                    vrf.inject_tcls_pip(
                        tcid,
                        TileWireCoord::new_idx(0, wires::OCTAL_H[7]),
                        TileWireCoord::new_idx(0, wires::TIE_0),
                    );
                }
                if tcname.starts_with("IO_N") {
                    vrf.inject_tcls_pip(
                        tcid,
                        TileWireCoord::new_idx(0, wires::OCTAL_V[7]),
                        TileWireCoord::new_idx(0, wires::TIE_0),
                    );
                }
            }
        }
        for (tcid, _, tcls) in &endev.edev.db.tile_classes {
            let Some(BelInfo::SwitchBox(sb)) = tcls.bels.get(bslots::INT) else {
                continue;
            };
            for item in &sb.items {
                let SwitchBoxItem::Mux(mux) = item else {
                    continue;
                };
                if wires::IMUX_TBUF_I.contains(mux.dst.wire)
                    || wires::IMUX_TBUF_T.contains(mux.dst.wire)
                    || wires::IMUX_IO_T.contains(mux.dst.wire)
                    || wires::IMUX_IO_O1.contains(mux.dst.wire)
                {
                    for &src in mux.src.keys() {
                        if matches!(src.wire, wires::TIE_0 | wires::TIE_1) {
                            vrf.skip_tcls_pip(tcid, mux.dst, src.tw);
                        }
                    }
                }
            }
        }
        vrf.prep_int_wires();
        vrf.handle_int();
        for (tcrd, tile) in endev.edev.tiles() {
            let tcls = &endev.edev.db[tile.class];
            for (slot, _) in &tcls.bels {
                verify_bel(endev, &mut vrf, tcrd.bel(slot))
            }
        }
        vrf.finish();
    };
}
