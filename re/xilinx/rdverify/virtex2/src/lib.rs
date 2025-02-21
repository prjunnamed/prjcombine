use prjcombine_interconnect::grid::{DieId, EdgeIoCoord};
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};
use prjcombine_virtex2::chip::ChipKind;
use unnamed_entity::EntityId;

mod clb;
mod clk;
mod io;

fn get_bel_iob<'a>(
    endev: &ExpandedNamedDevice,
    vrf: &Verifier<'a>,
    crd: EdgeIoCoord,
) -> BelContext<'a> {
    let (col, row, bel) = endev.grid.get_io_loc(crd);
    vrf.find_bel(
        DieId::from_idx(0),
        (col, row),
        match bel.to_idx() {
            0 => "IOI0",
            1 => "IOI1",
            2 => "IOI2",
            3 => "IOI3",
            _ => unreachable!(),
        },
    )
    .unwrap()
}

fn verify_rll(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = Vec::new();
    if bel.bel.pins.is_empty() {
        for pin in bel.naming.pins.keys() {
            pins.push((&**pin, SitePinDir::In));
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
    vrf.verify_bel(bel, "RESERVED_LL", &pins, &[]);
}

fn verify_gt(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if endev.grid.kind == ChipKind::Virtex2PX {
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
        for (pin, oname) in [("BREFCLKPIN", "CLK_P"), ("BREFCLKNIN", "CLK_N")] {
            vrf.claim_node(&[bel.fwire(pin)]);
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
            let obel = vrf
                .find_bel(bel.die, (endev.grid.col_clk - 1, bel.row), oname)
                .unwrap();
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
        let obel = vrf
            .find_bel(bel.die, (endev.grid.col_clk, bel.row), "BREFCLK")
            .unwrap();
        for pin in ["BREFCLK", "BREFCLK2"] {
            vrf.claim_node(&[bel.fwire(pin)]);
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
            vrf.verify_node(&[bel.fwire_far(pin), obel.fwire(pin)]);
        }
        vrf.claim_node(&[bel.fwire("TST10B8BICRD0")]);
        vrf.claim_node(&[bel.fwire("TST10B8BICRD1")]);
    }
    for (pin, okey) in [("RXP", "IPAD.RXP"), ("RXN", "IPAD.RXN")] {
        vrf.claim_node(&[bel.fwire(pin)]);
        let obel = vrf.find_bel_sibling(bel, okey);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("I"));
    }
    for (pin, okey) in [("TXP", "OPAD.TXP"), ("TXN", "OPAD.TXN")] {
        vrf.claim_node(&[bel.fwire(pin)]);
        let obel = vrf.find_bel_sibling(bel, okey);
        vrf.claim_pip(bel.crd(), obel.wire("O"), bel.wire(pin));
    }
}

fn verify_mult(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    if matches!(endev.grid.kind, ChipKind::Spartan3E | ChipKind::Spartan3A) {
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
        if let Some(obel) = vrf.find_bel_walk(bel, 0, -4, "MULT") {
            for (o, i) in &carry {
                vrf.verify_node(&[bel.fwire(i), obel.fwire_far(o)]);
                vrf.claim_pip(obel.crd(), obel.wire_far(o), obel.wire(o));
            }
        }
        if endev.grid.kind == ChipKind::Spartan3A {
            let obel = vrf.find_bel_sibling(bel, "BRAM");
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
    if let Some(obel) = vrf.find_bel_walk(bel, 0, -4, "DSP") {
        for (o, i) in &carry {
            vrf.verify_node(&[bel.fwire(i), obel.fwire_far(o)]);
            vrf.claim_pip(obel.crd(), obel.wire_far(o), obel.wire(o));
        }
    }
}

fn verify_bel(endev: &ExpandedNamedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        "RLL" => verify_rll(vrf, bel),
        _ if bel.key.starts_with("SLICE") => {
            if endev.grid.kind.is_virtex2() {
                clb::verify_slice_v2(endev, vrf, bel);
            } else {
                clb::verify_slice_s3(vrf, bel);
            }
        }
        _ if bel.key.starts_with("TBUF") => {
            vrf.verify_bel(bel, "TBUF", &[("O", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("O")]);
        }
        "TBUS" => {
            clb::verify_tbus(vrf, bel);
        }
        "RANDOR" => clb::verify_randor(endev, vrf, bel),
        "RANDOR_OUT" => (),

        "BRAM" => {
            let kind = match endev.grid.kind {
                ChipKind::Spartan3A => "RAMB16BWE",
                ChipKind::Spartan3ADsp => "RAMB16BWER",
                _ => "RAMB16",
            };
            vrf.verify_bel(bel, kind, &[], &[]);
        }
        "MULT" => verify_mult(endev, vrf, bel),
        "DSP" => verify_dsp(vrf, bel),

        _ if bel.key.starts_with("IOI") => io::verify_ioi(endev, vrf, bel),
        _ if bel.key.starts_with("IBUF") => vrf.verify_bel(bel, "IBUF", &[], &[]),
        _ if bel.key.starts_with("OBUF") => vrf.verify_bel(bel, "OBUF", &[], &[]),
        "CLK_P" | "CLK_N" => {
            vrf.verify_bel(bel, bel.key, &[("I", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("I")]);
            vrf.claim_node(&[bel.fwire_far("I")]);
            vrf.claim_pip(bel.crd(), bel.wire_far("I"), bel.wire("I"));
        }
        "BREFCLK_INT" => {
            let obel = vrf.find_bel_sibling(bel, "CLK_P");
            vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("I"));
        }
        "PCILOGICSE" => io::verify_pcilogicse(endev, vrf, bel),
        "PCI_CE_N" => io::verify_pci_ce_n(endev, vrf, bel),
        "PCI_CE_S" => io::verify_pci_ce_s(endev, vrf, bel),
        "PCI_CE_E" => io::verify_pci_ce_e(endev, vrf, bel),
        "PCI_CE_W" => io::verify_pci_ce_w(endev, vrf, bel),
        "PCI_CE_CNR" => io::verify_pci_ce_cnr(endev, vrf, bel),

        "BREFCLK" => clk::verify_brefclk(endev, vrf, bel),
        _ if bel.key.starts_with("BUFGMUX") => clk::verify_bufgmux(endev, vrf, bel),
        _ if bel.key.starts_with("BUFG") => clk::verify_bufg(endev, vrf, bel),
        _ if bel.key.starts_with("GCLKH") => clk::verify_gclkh(endev, vrf, bel),
        "GCLKC" => clk::verify_gclkc(endev, vrf, bel),
        "CLKC" => {
            if endev.grid.kind.is_virtex2() {
                clk::verify_clkc_v2(endev, vrf, bel);
            } else {
                clk::verify_clkc_s3(endev, vrf, bel);
            }
        }
        "CLKC_50A" => clk::verify_clkc_50a(endev, vrf, bel),
        "GCLKVM" => clk::verify_gclkvm(endev, vrf, bel),
        "GCLKVC" => clk::verify_gclkvc(endev, vrf, bel),
        "DCMCONN.S3E" => (),
        "DCMCONN" => clk::verify_dcmconn(endev, vrf, bel),

        _ if bel.key.starts_with("GT") => verify_gt(endev, vrf, bel),
        _ if bel.key.starts_with("IPAD") => {
            vrf.verify_bel(bel, "GTIPAD", &[("I", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("I")]);
        }
        _ if bel.key.starts_with("OPAD") => {
            vrf.verify_bel(bel, "GTOPAD", &[("O", SitePinDir::In)], &[]);
            vrf.claim_node(&[bel.fwire("O")]);
        }

        "STARTUP" | "CAPTURE" | "SPI_ACCESS" | "BSCAN" | "JTAGPPC" | "PMV" | "DNA_PORT"
        | "PCILOGIC" | "PPC405" => {
            vrf.verify_bel(bel, bel.key, &[], &[]);
        }
        "DCM" => {
            vrf.verify_bel(bel, bel.key, &[], &[]);
            if endev.grid.kind.is_virtex2p() {
                // just some detritus.
                vrf.claim_node(&[(bel.crd(), "BRAM_IOIS_DATA29")]);
                vrf.claim_pip(bel.crd(), "BRAM_IOIS_DATA29", "BRAM_IOIS_VCC_WIRE");
            }
        }
        "ICAP" => {
            vrf.verify_bel(bel, bel.key, &[], &[]);
            if endev.grid.kind == ChipKind::Spartan3E {
                // eh.
                vrf.claim_node(&[bel.fwire("I2")]);
            }
        }
        _ if bel.key.starts_with("GLOBALSIG") => {
            vrf.verify_bel(bel, "GLOBALSIG", &[], &[]);
        }
        _ if bel.key.starts_with("DCIRESET") => {
            vrf.verify_bel(bel, "DCIRESET", &[], &[]);
        }
        _ if bel.key.starts_with("DCI") => {
            vrf.verify_bel(bel, "DCI", &[], &[]);
        }
        _ if bel.key.starts_with("PTE2OMUX") => {
            let out = bel.wire("OUT");
            for (k, v) in &bel.naming.pins {
                if k == "OUT" {
                    continue;
                }
                vrf.claim_pip(bel.crd(), out, &v.name);
            }
        }
        "VCC" => {
            vrf.verify_bel(bel, "VCC", &[("VCCOUT", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("VCCOUT")]);
        }

        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}

fn verify_extra(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    if endev.grid.kind.is_spartan3ea() {
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
