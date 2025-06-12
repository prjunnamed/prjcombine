use prjcombine_interconnect::grid::EdgeIoCoord;
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};
use prjcombine_virtex2::{bels, chip::ChipKind};

mod clb;
mod clk;
mod io;

fn get_bel_iob<'a>(
    endev: &ExpandedNamedDevice,
    vrf: &Verifier<'a>,
    crd: EdgeIoCoord,
) -> BelContext<'a> {
    vrf.get_bel(endev.chip.get_io_loc(crd))
}

fn verify_rll(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = Vec::new();
    if bel.info.pins.is_empty() {
        for pin in bel.naming.pins.keys() {
            pins.push((&**pin, SitePinDir::In));
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
    vrf.verify_bel(bel, "RESERVED_LL", &pins, &[]);
}

fn verify_gt(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if endev.chip.kind == ChipKind::Virtex2PX {
        vrf.verify_bel(
            bel,
            "GT10",
            &[
                ("RXP", SitePinDir::In),
                ("RXN", SitePinDir::In),
                ("TXP", SitePinDir::Out),
                ("TXN", SitePinDir::Out),
                ("BREFCLKPIN", SitePinDir::In),
                ("BREFCLKNIN", SitePinDir::In),
            ],
            &[],
        );
        let (slot_p, slot_n) = if bel.row == endev.edev.chip.row_s() {
            (bels::IO[2], bels::IO[3])
        } else {
            (bels::IO[0], bels::IO[1])
        };
        for (pin, oslot) in [("BREFCLKPIN", slot_p), ("BREFCLKNIN", slot_n)] {
            vrf.claim_node(&[bel.fwire(pin)]);
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
            let obel = vrf.get_bel(bel.cell.with_col(endev.chip.col_clk - 1).bel(oslot));
            vrf.verify_node(&[bel.fwire_far(pin), obel.fwire_far("I")]);
        }
    } else {
        vrf.verify_bel(
            bel,
            "GT",
            &[
                ("RXP", SitePinDir::In),
                ("RXN", SitePinDir::In),
                ("TXP", SitePinDir::Out),
                ("TXN", SitePinDir::Out),
                ("BREFCLK", SitePinDir::In),
                ("BREFCLK2", SitePinDir::In),
                ("TST10B8BICRD0", SitePinDir::Out),
                ("TST10B8BICRD1", SitePinDir::Out),
            ],
            &[],
        );
        let obel = vrf.get_bel(bel.cell.with_col(endev.chip.col_clk).bel(bels::BREFCLK));
        for pin in ["BREFCLK", "BREFCLK2"] {
            vrf.claim_node(&[bel.fwire(pin)]);
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
            vrf.verify_node(&[bel.fwire_far(pin), obel.fwire(pin)]);
        }
        vrf.claim_node(&[bel.fwire("TST10B8BICRD0")]);
        vrf.claim_node(&[bel.fwire("TST10B8BICRD1")]);
    }
    for (pin, oslot) in [("RXP", bels::IPAD_RXP), ("RXN", bels::IPAD_RXN)] {
        vrf.claim_node(&[bel.fwire(pin)]);
        let obel = vrf.find_bel_sibling(bel, oslot);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("I"));
    }
    for (pin, oslot) in [("TXP", bels::OPAD_TXP), ("TXN", bels::OPAD_TXN)] {
        vrf.claim_node(&[bel.fwire(pin)]);
        let obel = vrf.find_bel_sibling(bel, oslot);
        vrf.claim_pip(bel.crd(), obel.wire("O"), bel.wire(pin));
    }
}

fn verify_mult(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if matches!(endev.chip.kind, ChipKind::Spartan3E | ChipKind::Spartan3A) {
        let carry: Vec<_> = (0..18)
            .map(|x| (format!("BCOUT{x}"), format!("BCIN{x}")))
            .collect();
        let mut pins = vec![];
        for (o, i) in &carry {
            pins.push((&**o, SitePinDir::Out));
            pins.push((&**i, SitePinDir::In));
        }
        vrf.verify_bel(bel, "MULT18X18SIO", &pins, &[]);
        for (o, i) in &carry {
            vrf.claim_node(&[bel.fwire(o)]);
            vrf.claim_node(&[bel.fwire(i)]);
        }
        if let Some(obel) = vrf.find_bel_walk(bel, 0, -4, bels::MULT) {
            for (o, i) in &carry {
                vrf.verify_node(&[bel.fwire(i), obel.fwire_far(o)]);
                vrf.claim_pip(obel.crd(), obel.wire_far(o), obel.wire(o));
            }
        }
        if endev.chip.kind == ChipKind::Spartan3A {
            let obel = vrf.find_bel_sibling(bel, bels::BRAM);
            for ab in ['A', 'B'] {
                for i in 0..16 {
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("{ab}{i}")),
                        obel.wire(&format!("DO{ab}{i}")),
                    );
                }
                for i in 0..2 {
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("{ab}{ii}", ii = i + 16)),
                        obel.wire(&format!("DOP{ab}{i}")),
                    );
                }
            }
        }
    } else {
        vrf.verify_bel(bel, "MULT18X18", &[], &[]);
    }
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
    vrf.verify_bel(bel, "DSP48A", &pins, &[]);
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

fn verify_bel(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let slot_name = endev.edev.egrid.db.bel_slots.key(bel.slot);
    match bel.slot {
        bels::RLL => verify_rll(vrf, bel),
        bels::SLICE0 | bels::SLICE1 | bels::SLICE2 | bels::SLICE3 => {
            if endev.chip.kind.is_virtex2() {
                clb::verify_slice_v2(endev, vrf, bel);
            } else {
                clb::verify_slice_s3(vrf, bel);
            }
        }
        bels::TBUF0 | bels::TBUF1 => {
            vrf.verify_bel(bel, "TBUF", &[("O", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("O")]);
        }
        bels::TBUS => {
            clb::verify_tbus(vrf, bel);
        }
        bels::RANDOR => clb::verify_randor(endev, vrf, bel),
        bels::RANDOR_OUT => (),

        bels::BRAM => {
            let kind = match endev.chip.kind {
                ChipKind::Spartan3A => "RAMB16BWE",
                ChipKind::Spartan3ADsp => "RAMB16BWER",
                _ => "RAMB16",
            };
            vrf.verify_bel(bel, kind, &[], &[]);
        }
        bels::MULT => verify_mult(endev, vrf, bel),
        bels::DSP => verify_dsp(vrf, bel),

        _ if slot_name.starts_with("IO") => io::verify_ioi(endev, vrf, bel),
        _ if slot_name.starts_with("IBUF") => vrf.verify_bel(bel, "IBUF", &[], &[]),
        _ if slot_name.starts_with("OBUF") => vrf.verify_bel(bel, "OBUF", &[], &[]),
        bels::BREFCLK_INT => {
            let slot = if bel.row == endev.edev.chip.row_s() {
                bels::IO[2]
            } else {
                bels::IO[0]
            };
            let obel = vrf.find_bel_sibling(bel, slot);
            vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("I"));
        }
        bels::PCILOGICSE => io::verify_pcilogicse(endev, vrf, bel),
        bels::PCI_CE_N => io::verify_pci_ce_n(endev, vrf, bel),
        bels::PCI_CE_S => io::verify_pci_ce_s(endev, vrf, bel),
        bels::PCI_CE_E => io::verify_pci_ce_e(endev, vrf, bel),
        bels::PCI_CE_W => io::verify_pci_ce_w(endev, vrf, bel),
        bels::PCI_CE_CNR => io::verify_pci_ce_cnr(endev, vrf, bel),

        bels::BREFCLK => clk::verify_brefclk(endev, vrf, bel),
        _ if slot_name.starts_with("BUFGMUX") => clk::verify_bufgmux(endev, vrf, bel),
        _ if slot_name.starts_with("GCLKH") => clk::verify_gclkh(endev, vrf, bel),
        bels::GCLKC => clk::verify_gclkc(endev, vrf, bel),
        bels::CLKC => {
            if endev.chip.kind.is_virtex2() {
                clk::verify_clkc_v2(endev, vrf, bel);
            } else {
                clk::verify_clkc_s3(endev, vrf, bel);
            }
        }
        bels::CLKC_50A => clk::verify_clkc_50a(endev, vrf, bel),
        bels::GCLKVM => clk::verify_gclkvm(endev, vrf, bel),
        bels::GCLKVC => clk::verify_gclkvc(endev, vrf, bel),
        bels::DCMCONN_S3E => (),
        bels::DCMCONN => clk::verify_dcmconn(endev, vrf, bel),

        bels::GT | bels::GT10 => verify_gt(endev, vrf, bel),
        bels::IPAD_RXP | bels::IPAD_RXN => {
            vrf.verify_bel(bel, "GTIPAD", &[("I", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("I")]);
        }
        bels::OPAD_TXP | bels::OPAD_TXN => {
            vrf.verify_bel(bel, "GTOPAD", &[("O", SitePinDir::In)], &[]);
            vrf.claim_node(&[bel.fwire("O")]);
        }

        bels::STARTUP
        | bels::CAPTURE
        | bels::SPI_ACCESS
        | bels::BSCAN
        | bels::JTAGPPC
        | bels::PMV
        | bels::DNA_PORT
        | bels::PCILOGIC
        | bels::PPC405 => {
            vrf.verify_bel(bel, slot_name, &[], &[]);
        }
        bels::DCM => {
            vrf.verify_bel(bel, "DCM", &[], &[]);
            if endev.chip.kind.is_virtex2p() {
                // just some detritus.
                vrf.claim_node(&[(bel.crd(), "BRAM_IOIS_DATA29")]);
                vrf.claim_pip(bel.crd(), "BRAM_IOIS_DATA29", "BRAM_IOIS_VCC_WIRE");
            }
        }
        bels::ICAP => {
            vrf.verify_bel(bel, "ICAP", &[], &[]);
            if endev.chip.kind == ChipKind::Spartan3E {
                // eh.
                vrf.claim_node(&[bel.fwire("I2")]);
            }
        }
        _ if slot_name.starts_with("GLOBALSIG") => {
            vrf.verify_bel(bel, "GLOBALSIG", &[], &[]);
        }
        bels::DCIRESET0 | bels::DCIRESET1 => {
            vrf.verify_bel(bel, "DCIRESET", &[], &[]);
        }
        bels::DCI0 | bels::DCI1 => {
            vrf.verify_bel(bel, "DCI", &[], &[]);
        }
        _ if slot_name.starts_with("PTE2OMUX") => {
            let out = bel.wire("OUT");
            for (k, v) in &bel.naming.pins {
                if k == "OUT" {
                    continue;
                }
                vrf.claim_pip(bel.crd(), out, &v.name);
            }
        }
        bels::VCC => {
            vrf.verify_bel(bel, "VCC", &[("VCCOUT", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("VCCOUT")]);
        }
        bels::MISR => (),

        _ => println!("MEOW {} {:?}", slot_name, bel.name),
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
    verify(
        rd,
        &endev.ngrid,
        |_| (),
        |vrf, bel| verify_bel(endev, vrf, bel),
        |vrf| verify_extra(endev, vrf),
    );
}
