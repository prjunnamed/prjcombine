#![recursion_limit = "1024"]

use prjcombine_interconnect::grid::{CellCoord, DieId, RowId};
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_re_xilinx_naming_virtex4::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};
use prjcombine_virtex4::{bels, tslots};
use unnamed_entity::EntityId;

fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.info.pins.contains_key("WE") {
        "SLICEM"
    } else {
        "SLICEL"
    };
    vrf.verify_bel(
        bel,
        kind,
        &[("CIN", SitePinDir::In), ("COUT", SitePinDir::Out)],
        &[],
    );
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.slot) {
        vrf.claim_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
        vrf.claim_pip(obel.crd(), obel.wire_far("COUT"), obel.wire("COUT"));
    } else {
        vrf.claim_node(&[bel.fwire("CIN")]);
    }
    vrf.claim_node(&[bel.fwire("COUT")]);
    vrf.claim_pip(bel.crd(), bel.wire_far("AMUX"), bel.wire_far("AX"));
    vrf.claim_pip(bel.crd(), bel.wire_far("BMUX"), bel.wire_far("BX"));
    vrf.claim_pip(bel.crd(), bel.wire_far("CMUX"), bel.wire_far("CX"));
    vrf.claim_pip(bel.crd(), bel.wire_far("DMUX"), bel.wire_far("DX"));
}

fn verify_bram(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
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
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bel.slot) {
            vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
        }
    }
}

fn verify_dsp(vrf: &mut Verifier, bel: &BelContext<'_>) {
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
        vrf.claim_node(&[bel.fwire(opin)]);
        if bel.slot == bels::DSP0 {
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bels::DSP1) {
                vrf.claim_node(&[bel.fwire(ipin), obel.fwire_far(opin)]);
                vrf.claim_pip(obel.crd(), obel.wire_far(opin), obel.wire(opin));
            } else {
                vrf.claim_node(&[bel.fwire(ipin)]);
            }
        } else {
            vrf.claim_node(&[bel.fwire(ipin)]);
            let obel = vrf.find_bel_sibling(bel, bels::DSP0);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_bel(bel, "DSP48E", &pins, &[]);
}

fn verify_bufgctrl(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFGCTRL",
        &[
            ("I0", SitePinDir::In),
            ("I1", SitePinDir::In),
            ("O", SitePinDir::Out),
        ],
        &["I0MUX", "I1MUX", "CKINT0", "CKINT1"],
    );
    let idx = bels::BUFGCTRL
        .into_iter()
        .position(|x| x == bel.slot)
        .unwrap();
    let is_b = idx < 16;
    vrf.claim_node(&[bel.fwire("I0")]);
    vrf.claim_node(&[bel.fwire("I1")]);
    vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("I0MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("I1"), bel.wire("I1MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), bel.wire("CKINT0"));
    vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), bel.wire("CKINT1"));
    vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), bel.wire("MUXBUS0"));
    vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), bel.wire("MUXBUS1"));
    for i in 0..16 {
        let obid = if is_b {
            bels::BUFGCTRL[i]
        } else {
            bels::BUFGCTRL[i + 16]
        };
        let obel = vrf.find_bel_sibling(bel, obid);
        vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), obel.wire("GFB"));
        vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), obel.wire("GFB"));
    }
    let obel = vrf.find_bel_sibling(
        bel,
        if is_b {
            bels::BUFG_MGTCLK_S
        } else {
            bels::BUFG_MGTCLK_N
        },
    );
    for i in 0..5 {
        let pin_l = format!("MGT_O_L{i}");
        let pin_r = format!("MGT_O_R{i}");
        vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), obel.wire(&pin_l));
        vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), obel.wire(&pin_l));
        vrf.claim_pip(bel.crd(), bel.wire("I0MUX"), obel.wire(&pin_r));
        vrf.claim_pip(bel.crd(), bel.wire("I1MUX"), obel.wire(&pin_r));
    }
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_node(&[bel.fwire("GCLK")]);
    vrf.claim_node(&[bel.fwire("GFB")]);
    vrf.claim_pip(bel.crd(), bel.wire("GCLK"), bel.wire("O"));
    vrf.claim_pip(bel.crd(), bel.wire("GFB"), bel.wire("O"));
    let srow = if is_b {
        endev.edev.chips[bel.die].row_bufg() - 30
    } else {
        if endev.edev.chips[bel.die].reg_cfg.to_idx() == endev.edev.chips[bel.die].regs - 1 {
            vrf.claim_node(&[bel.fwire("MUXBUS0")]);
            vrf.claim_node(&[bel.fwire("MUXBUS1")]);
            return;
        }
        endev.edev.chips[bel.die].row_bufg() + 20
    };
    let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::CLK_IOB));
    let idx0 = (idx % 16) * 2;
    let idx1 = (idx % 16) * 2 + 1;
    vrf.verify_node(&[bel.fwire("MUXBUS0"), obel.fwire(&format!("MUXBUS_O{idx0}"))]);
    vrf.verify_node(&[bel.fwire("MUXBUS1"), obel.fwire(&format!("MUXBUS_O{idx1}"))]);
}

fn verify_bufg_mgtclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let dy = match bel.slot {
        bels::BUFG_MGTCLK_S => 0,
        bels::BUFG_MGTCLK_N => 20,
        _ => unreachable!(),
    };
    let obel = vrf.find_bel_delta(bel, 0, dy, bels::CLK_HROW).unwrap();
    for i in 0..5 {
        let pin_l_i = format!("MGT_I_L{i}");
        let pin_r_i = format!("MGT_I_R{i}");
        let pin_l_o = format!("MGT_O_L{i}");
        let pin_r_o = format!("MGT_O_R{i}");
        vrf.claim_node(&[bel.fwire(&pin_l_o)]);
        vrf.claim_node(&[bel.fwire(&pin_r_o)]);
        if endev.edev.col_lgt.is_some() {
            vrf.claim_pip(bel.crd(), bel.wire(&pin_l_o), bel.wire(&pin_l_i));
            vrf.verify_node(&[bel.fwire(&pin_l_i), obel.fwire(&pin_l_o)])
        }
        vrf.claim_pip(bel.crd(), bel.wire(&pin_r_o), bel.wire(&pin_r_i));
        vrf.verify_node(&[bel.fwire(&pin_r_i), obel.fwire(&pin_r_o)])
    }
}

fn verify_sysmon(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![];
    for i in 0..16 {
        pins.push(format!("VAUXP{i}"));
        pins.push(format!("VAUXN{i}"));
    }
    pins.push("VP".to_string());
    pins.push("VN".to_string());
    let mut pin_refs = vec![];
    for pin in &pins {
        pin_refs.push((&pin[..], SitePinDir::In));
    }
    vrf.verify_bel(bel, "SYSMON", &pin_refs, &[]);

    vrf.claim_node(&[bel.fwire("VP")]);
    let obel = vrf.find_bel_sibling(bel, bels::IPAD_VP);
    vrf.claim_pip(bel.crd(), bel.wire("VP"), obel.wire("O"));
    vrf.claim_node(&[bel.fwire("VN")]);
    let obel = vrf.find_bel_sibling(bel, bels::IPAD_VN);
    vrf.claim_pip(bel.crd(), bel.wire("VN"), obel.wire("O"));

    for i in 0..16 {
        let Some((iop, _)) = endev.edev.get_sysmon_vaux(bel.cell, i) else {
            continue;
        };
        let vauxp = format!("VAUXP{i}");
        let vauxn = format!("VAUXN{i}");
        vrf.claim_node(&[bel.fwire(&vauxp)]);
        vrf.claim_node(&[bel.fwire(&vauxn)]);
        vrf.claim_pip(bel.crd(), bel.wire(&vauxp), bel.wire_far(&vauxp));
        vrf.claim_pip(bel.crd(), bel.wire(&vauxn), bel.wire_far(&vauxn));
        let obel = vrf.get_bel(iop.cell.bel(bels::IOB1));
        vrf.claim_node(&[bel.fwire_far(&vauxp), obel.fwire("MONITOR")]);
        vrf.claim_pip(obel.crd(), obel.wire("MONITOR"), obel.wire("PADOUT"));
        let obel = vrf.get_bel(iop.cell.bel(bels::IOB0));
        vrf.claim_node(&[bel.fwire_far(&vauxn), obel.fwire("MONITOR")]);
        vrf.claim_pip(obel.crd(), obel.wire("MONITOR"), obel.wire("PADOUT"));
    }
}

fn verify_clk_mux(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.slot {
        bels::CLK_IOB => {
            for i in 0..10 {
                vrf.claim_node(&[bel.fwire(&format!("PAD_BUF{i}"))]);
                vrf.claim_node(&[bel.fwire(&format!("GIOB{i}"))]);
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("PAD_BUF{i}")),
                    bel.wire(&format!("PAD{i}")),
                );
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("GIOB{i}")),
                    bel.wire(&format!("PAD_BUF{i}")),
                );
                let obel = vrf.find_bel_delta(bel, 0, i, bels::ILOGIC1).unwrap();
                vrf.verify_node(&[bel.fwire(&format!("PAD{i}")), obel.fwire("CLKOUT")]);
                // avoid double-claim for IOBs that are also BUFIO inps
                if !matches!(obel.row.to_idx() % 20, 8..=11) {
                    vrf.claim_node(&[obel.fwire("CLKOUT")]);
                    vrf.claim_pip(obel.crd(), obel.wire("CLKOUT"), obel.wire("O"));
                }
            }
        }
        bels::CLK_CMT => {
            let obel = vrf.find_bel_sibling(bel, bels::CMT);
            for i in 0..28 {
                vrf.verify_node(&[
                    bel.fwire(&format!("CMT_CLK{i}")),
                    obel.fwire(&format!("OUT{i}")),
                ]);
            }
        }
        _ => (),
    }

    let is_b = bel.row < endev.edev.chips[bel.die].row_bufg();
    let is_hrow_b = bel.row.to_idx().is_multiple_of(20);

    if is_b != is_hrow_b {
        let dy = if is_hrow_b { 10 } else { 0 };
        let obel = vrf.find_bel_delta(bel, 0, dy, bels::CLK_HROW).unwrap();
        for i in 0..5 {
            if endev.edev.col_lgt.is_some() {
                vrf.verify_node(&[
                    bel.fwire(&format!("MGT_L{i}")),
                    obel.fwire(&format!("MGT_O_L{i}")),
                ]);
            } else if bel.slot != bels::CLK_MGT {
                vrf.claim_node(&[bel.fwire(&format!("MGT_L{i}"))]);
            }
            vrf.verify_node(&[
                bel.fwire(&format!("MGT_R{i}")),
                obel.fwire(&format!("MGT_O_R{i}")),
            ]);
        }
    } else {
        for i in 0..5 {
            vrf.claim_node(&[bel.fwire(&format!("MGT_L{i}"))]);
            vrf.claim_node(&[bel.fwire(&format!("MGT_R{i}"))]);
        }
    }

    let dy = if is_b { -10 } else { 10 };
    let obel = vrf
        .find_bel_delta(bel, 0, dy, bels::CLK_CMT)
        .or_else(|| vrf.find_bel_walk(bel, 0, dy, bels::CLK_MGT));
    for i in 0..32 {
        vrf.claim_node(&[bel.fwire(&format!("MUXBUS_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MUXBUS_O{i}")),
            bel.wire(&format!("MUXBUS_I{i}")),
        );
        if let Some(ref obel) = obel {
            vrf.verify_node(&[
                bel.fwire(&format!("MUXBUS_I{i}")),
                obel.fwire(&format!("MUXBUS_O{i}")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("MUXBUS_I{i}"))]);
        }
        match bel.slot {
            bels::CLK_IOB => {
                for j in 0..10 {
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("MUXBUS_O{i}")),
                        bel.wire(&format!("PAD_BUF{j}")),
                    );
                }
            }
            bels::CLK_CMT => {
                for j in 0..28 {
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("MUXBUS_O{i}")),
                        bel.wire(&format!("CMT_CLK{j}")),
                    );
                }
            }
            _ => (),
        }
        for j in 0..5 {
            if endev.edev.col_lgt.is_some() || bel.slot != bels::CLK_MGT {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("MUXBUS_O{i}")),
                    bel.wire(&format!("MGT_L{j}")),
                );
            }
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("MUXBUS_O{i}")),
                bel.wire(&format!("MGT_R{j}")),
            );
        }
    }
}

fn verify_ilogic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("TFB", SitePinDir::In),
        ("OFB", SitePinDir::In),
        ("D", SitePinDir::In),
        ("DDLY", SitePinDir::In),
        ("CLK", SitePinDir::In),
        ("CLKB", SitePinDir::In),
        ("OCLK", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "ILOGIC", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel = vrf.find_bel_sibling(bel, bels::IOI);
    vrf.claim_pip(bel.crd(), bel.wire("CLK"), obel.wire("ICLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK"), obel.wire("ICLK1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKB"), obel.wire("ICLK0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKB"), obel.wire("ICLK1"));

    let obel = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::ILOGIC0 => bels::IODELAY0,
            bels::ILOGIC1 => bels::IODELAY1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("DDLY"), obel.wire("DATAOUT"));

    vrf.claim_pip(bel.crd(), bel.wire("D"), bel.wire("I_IOB"));

    let obel = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::ILOGIC0 => bels::OLOGIC0,
            bels::ILOGIC1 => bels::OLOGIC1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("OCLK"), obel.wire("CLKMUX"));
    vrf.claim_pip(bel.crd(), bel.wire("OFB"), obel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("TFB"), obel.wire("TQ"));

    if bel.slot == bels::ILOGIC0 {
        let obel = vrf.find_bel_sibling(bel, bels::ILOGIC1);
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_ologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("OQ", SitePinDir::Out),
        ("CLK", SitePinDir::In),
        ("CLKDIV", SitePinDir::In),
        ("SHIFTIN1", SitePinDir::In),
        ("SHIFTIN2", SitePinDir::In),
        ("SHIFTOUT1", SitePinDir::Out),
        ("SHIFTOUT2", SitePinDir::Out),
    ];
    vrf.verify_bel(
        bel,
        "OLOGIC",
        &pins,
        &["CKINT", "CKINT_DIV", "CLKMUX", "CLKDIVMUX"],
    );
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CLKMUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKDIV"), bel.wire("CLKDIVMUX"));

    let obel = vrf.find_bel_sibling(bel, bels::IOI);
    vrf.claim_pip(bel.crd(), bel.wire("CLKMUX"), bel.wire("CKINT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKDIVMUX"), bel.wire("CKINT_DIV"));
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKMUX"),
            obel.wire(&format!("IOCLK{i}")),
        );
    }
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKMUX"),
            obel.wire(&format!("RCLK{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKDIVMUX"),
            obel.wire(&format!("RCLK{i}")),
        );
    }
    for i in 0..10 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKMUX"),
            obel.wire(&format!("HCLK{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKDIVMUX"),
            obel.wire(&format!("HCLK{i}")),
        );
    }

    let obel = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::OLOGIC0 => bels::IODELAY0,
            bels::OLOGIC1 => bels::IODELAY1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("O_IOB"), bel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("O_IOB"), obel.wire("DATAOUT"));
    vrf.claim_pip(bel.crd(), bel.wire("T_IOB"), bel.wire("TQ"));

    if bel.slot == bels::OLOGIC1 {
        let obel = vrf.find_bel_sibling(bel, bels::OLOGIC0);
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN1"), obel.wire("SHIFTOUT1"));
        vrf.claim_pip(bel.crd(), bel.wire("SHIFTIN2"), obel.wire("SHIFTOUT2"));
    }
}

fn verify_iodelay(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("IDATAIN", SitePinDir::In),
        ("ODATAIN", SitePinDir::In),
        ("T", SitePinDir::In),
        ("DATAOUT", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "IODELAY", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::IODELAY0 => bels::ILOGIC0,
            bels::IODELAY1 => bels::ILOGIC1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("IDATAIN"), obel.wire("I_IOB"));

    let obel = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::IODELAY0 => bels::OLOGIC0,
            bels::IODELAY1 => bels::OLOGIC1,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("ODATAIN"), obel.wire("OQ"));
    vrf.claim_pip(bel.crd(), bel.wire("T"), obel.wire("TQ"));
}

fn verify_ioi_clk(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("ICLK0")]);
    vrf.claim_node(&[bel.fwire("ICLK1")]);
    for opin in ["ICLK0", "ICLK1"] {
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("CKINT0"));
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire("CKINT1"));
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("IOCLK{i}")));
        }
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("RCLK{i}")));
        }
        for i in 0..10 {
            vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(&format!("HCLK{i}")));
        }
    }

    let srow = RowId::from_idx(bel.row.to_idx() / 20 * 20 + 10);
    let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::IOCLK));
    for i in 0..10 {
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK{i}")),
            obel.fwire(&format!("HCLK_O{i}")),
        ]);
    }
    for i in 0..4 {
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK{i}")),
            obel.fwire(&format!("RCLK_O{i}")),
        ]);
    }
}

fn verify_iob(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.slot == bels::IOB1 {
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
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::IOB0 => bels::OLOGIC0,
            bels::IOB1 => bels::OLOGIC1,
            _ => unreachable!(),
        },
    );
    vrf.verify_node(&[bel.fwire("O"), obel.fwire("O_IOB")]);
    vrf.verify_node(&[bel.fwire("T"), obel.fwire("T_IOB")]);
    let obel = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::IOB0 => bels::ILOGIC0,
            bels::IOB1 => bels::ILOGIC1,
            _ => unreachable!(),
        },
    );
    vrf.verify_node(&[bel.fwire("I"), obel.fwire("I_IOB")]);
    let obel = vrf.find_bel_sibling(
        bel,
        match bel.slot {
            bels::IOB0 => bels::IOB1,
            bels::IOB1 => bels::IOB0,
            _ => unreachable!(),
        },
    );
    vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
    if kind == "IOBS" {
        vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
    }
}

fn verify_dcm(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLK0", SitePinDir::Out),
        ("CLK90", SitePinDir::Out),
        ("CLK180", SitePinDir::Out),
        ("CLK270", SitePinDir::Out),
        ("CLK2X", SitePinDir::Out),
        ("CLK2X180", SitePinDir::Out),
        ("CLKFX", SitePinDir::Out),
        ("CLKFX180", SitePinDir::Out),
        ("CLKDV", SitePinDir::Out),
        ("CONCUR", SitePinDir::Out),
        ("CLKIN", SitePinDir::In),
        ("CLKFB", SitePinDir::In),
        ("SKEWCLKIN1", SitePinDir::In),
        ("SKEWCLKIN2", SitePinDir::In),
    ];
    vrf.verify_bel(bel, "DCM_ADV", &pins, &["CKINT0", "CKINT1", "CKINT2"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, bels::CMT);
    let obel_pll = vrf.find_bel_sibling(bel, bels::PLL);
    let pllpin = if bel.slot == bels::DCM0 {
        "CLK_TO_DCM0"
    } else {
        "CLK_TO_DCM1"
    };
    for i in 0..10 {
        vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel.wire(&format!("HCLK{i}")));
        vrf.claim_pip(bel.crd(), bel.wire("CLKFB"), obel.wire(&format!("HCLK{i}")));
        vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel.wire(&format!("GIOB{i}")));
        vrf.claim_pip(bel.crd(), bel.wire("CLKFB"), obel.wire(&format!("GIOB{i}")));
    }
    for i in 0..3 {
        vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), bel.wire(&format!("CKINT{i}")));
        vrf.claim_pip(bel.crd(), bel.wire("CLKFB"), bel.wire(&format!("CKINT{i}")));
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel_pll.wire(pllpin));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFB"), obel_pll.wire(pllpin));

    vrf.claim_node(&[bel.fwire("CLKIN_TEST")]);
    vrf.claim_node(&[bel.fwire("CLKFB_TEST")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN_TEST"), bel.wire("CLKIN"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFB_TEST"), bel.wire("CLKFB"));

    let base = if bel.slot == bels::DCM0 { 0 } else { 18 };
    vrf.claim_node(&[bel.fwire("MUXED_CLK")]);
    for i in 0..10 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("MUXED_CLK"),
            obel.wire(&format!("OUT{ii}", ii = base + i)),
        );
    }
    vrf.claim_pip(bel.crd(), bel.wire("SKEWCLKIN1"), bel.wire("MUXED_CLK"));
    for i in 0..10 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("SKEWCLKIN2"),
            obel.wire(&format!("OUT{ii}_TEST", ii = base + i)),
        );
    }
}

fn verify_pll(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("CLKIN1", SitePinDir::In),
        ("CLKIN2", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
        ("SKEWCLKIN1", SitePinDir::In),
        ("SKEWCLKIN2", SitePinDir::In),
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT2", SitePinDir::Out),
        ("CLKOUT3", SitePinDir::Out),
        ("CLKOUT4", SitePinDir::Out),
        ("CLKOUT5", SitePinDir::Out),
        ("CLKFBOUT", SitePinDir::Out),
        ("CLKOUTDCM0", SitePinDir::Out),
        ("CLKOUTDCM1", SitePinDir::Out),
        ("CLKOUTDCM2", SitePinDir::Out),
        ("CLKOUTDCM3", SitePinDir::Out),
        ("CLKOUTDCM4", SitePinDir::Out),
        ("CLKOUTDCM5", SitePinDir::Out),
        ("CLKFBDCM", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "PLL_ADV", &pins, &["CKINT0", "CKINT1"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    vrf.claim_node(&[bel.fwire("CLKIN1_TEST")]);
    vrf.claim_node(&[bel.fwire("CLKINFB_TEST")]);
    vrf.claim_node(&[bel.fwire("CLKFBDCM_TEST")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1_TEST"), bel.wire("CLKIN1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKINFB_TEST"), bel.wire("CLKFBIN"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBDCM_TEST"), bel.wire("CLKFBDCM"));

    let obel = vrf.find_bel_sibling(bel, bels::CMT);

    for i in 0..10 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKIN1"),
            obel.wire(&format!("HCLK{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKFBIN"),
            obel.wire(&format!("HCLK{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKIN1"),
            obel.wire(&format!("GIOB{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKFBIN"),
            obel.wire(&format!("GIOB{i}")),
        );
        if i >= 5 {
            let pin2 = format!("HCLK{i}_TO_CLKIN2");
            if obel.naming.pins.contains_key(&pin2) {
                vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), obel.wire(&pin2));
            } else {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLKIN2"),
                    obel.wire(&format!("HCLK{i}")),
                );
            }
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLKIN2"),
                obel.wire(&format!("GIOB{i}")),
            );
        }
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKFBDCM"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLK_DCM_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire("CLKFBDCM"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire("CLK_FB_FROM_DCM"));

    let obel_dcm0 = vrf.find_bel_sibling(bel, bels::DCM0);
    let obel_dcm1 = vrf.find_bel_sibling(bel, bels::DCM1);
    vrf.claim_node(&[bel.fwire("CLK_DCM_MUX")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK_DCM_MUX"), bel.wire("CKINT0"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_DCM_MUX"),
        obel_dcm0.wire("MUXED_CLK"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_DCM_MUX"),
        obel_dcm1.wire("MUXED_CLK"),
    );

    vrf.claim_node(&[bel.fwire("CLK_FB_FROM_DCM")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK_FB_FROM_DCM"), obel.wire("OUT11"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_FB_FROM_DCM"), bel.wire("CKINT1"));

    vrf.claim_node(&[bel.fwire("CLK_TO_DCM0")]);
    vrf.claim_node(&[bel.fwire("CLK_TO_DCM1")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM0"), bel.wire("CLKOUTDCM0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM0"), bel.wire("CLKOUTDCM1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM0"), bel.wire("CLKOUTDCM2"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM0"), bel.wire("CLKOUTDCM3"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM0"), bel.wire("CLKOUTDCM4"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM0"), bel.wire("CLKOUTDCM5"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM1"), bel.wire("CLKOUTDCM0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM1"), bel.wire("CLKOUTDCM1"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM1"), bel.wire("CLKOUTDCM2"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM1"), bel.wire("CLKOUTDCM3"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM1"), bel.wire("CLKOUTDCM4"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_TO_DCM1"), bel.wire("CLKOUTDCM5"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_TO_DCM1"),
        bel.wire("CLKFBDCM_TEST"),
    );

    vrf.claim_pip(bel.crd(), bel.wire("SKEWCLKIN1"), bel.wire("CLK_TO_DCM1"));
    vrf.claim_pip(bel.crd(), bel.wire("SKEWCLKIN2"), bel.wire("CLK_TO_DCM0"));
}

fn verify_cmt(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_dcm0 = vrf.find_bel_sibling(bel, bels::DCM0);
    let obel_dcm1 = vrf.find_bel_sibling(bel, bels::DCM1);
    let obel_pll = vrf.find_bel_sibling(bel, bels::PLL);
    for (i, obel, ipin) in [
        (0, &obel_dcm0, "CLK0"),
        (1, &obel_dcm0, "CLK90"),
        (2, &obel_dcm0, "CLK180"),
        (3, &obel_dcm0, "CLK270"),
        (4, &obel_dcm0, "CLK2X"),
        (5, &obel_dcm0, "CLK2X180"),
        (6, &obel_dcm0, "CLKDV"),
        (7, &obel_dcm0, "CLKFX"),
        (8, &obel_dcm0, "CLKFX180"),
        (9, &obel_dcm0, "CONCUR"),
        (11, &obel_pll, "CLKFBOUT"),
        (12, &obel_pll, "CLKOUT0"),
        (13, &obel_pll, "CLKOUT1"),
        (14, &obel_pll, "CLKOUT2"),
        (15, &obel_pll, "CLKOUT3"),
        (16, &obel_pll, "CLKOUT4"),
        (17, &obel_pll, "CLKOUT5"),
        (18, &obel_dcm1, "CLK0"),
        (19, &obel_dcm1, "CLK90"),
        (20, &obel_dcm1, "CLK180"),
        (21, &obel_dcm1, "CLK270"),
        (22, &obel_dcm1, "CLK2X"),
        (23, &obel_dcm1, "CLK2X180"),
        (24, &obel_dcm1, "CLKDV"),
        (25, &obel_dcm1, "CLKFX"),
        (26, &obel_dcm1, "CLKFX180"),
        (27, &obel_dcm1, "CONCUR"),
    ] {
        let pin = format!("OUT{i}");
        vrf.claim_node(&[bel.fwire(&pin)]);
        vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire(ipin));
        if !(10..18).contains(&i) {
            let pin_test = format!("OUT{i}_TEST");
            vrf.claim_node(&[bel.fwire(&pin_test)]);
            vrf.claim_pip(bel.crd(), bel.wire(&pin_test), bel.wire(&pin));
        }
    }
    vrf.claim_pip(bel.crd(), bel.wire("OUT10"), obel_dcm0.wire("CLKFB_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("OUT10"), obel_dcm0.wire("CLKIN_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("OUT10"), obel_dcm1.wire("CLKFB_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("OUT10"), obel_dcm1.wire("CLKIN_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("OUT10"), obel_pll.wire("CLKIN1_TEST"));
    vrf.claim_pip(bel.crd(), bel.wire("OUT10"), obel_pll.wire("CLKINFB_TEST"));
    let srow = RowId::from_idx(bel.row.to_idx() / 20 * 20 + 10);
    let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::HCLK_CMT_HCLK));
    for i in 0..10 {
        let pin = format!("HCLK{i}");
        vrf.verify_node(&[bel.fwire(&pin), obel.fwire(&format!("HCLK_O{i}"))]);
        let pin2 = format!("HCLK{i}_TO_CLKIN2");
        if bel.naming.pins.contains_key(&pin2) {
            vrf.claim_node(&[bel.fwire(&pin2)]);
            vrf.claim_pip(bel.crd(), bel.wire(&pin2), bel.wire(&pin));
        }
    }
    let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::HCLK_CMT_GIOB));
    for i in 0..10 {
        let pin = format!("GIOB{i}");
        vrf.verify_node(&[bel.fwire(&pin), obel.fwire(&format!("GIOB_O{i}"))]);
    }
}

fn verify_gt(vrf: &mut Verifier, bel: &BelContext<'_>) {
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
        bels::GTP_DUAL => "GTP_DUAL",
        bels::GTX_DUAL => "GTX_DUAL",
        _ => unreachable!(),
    };
    vrf.verify_bel(bel, kind, &pins, &["GREFCLK"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    for (pin, slot) in [
        ("RXP0", bels::IPAD_RXP0),
        ("RXN0", bels::IPAD_RXN0),
        ("RXP1", bels::IPAD_RXP1),
        ("RXN1", bels::IPAD_RXN1),
    ] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("O"));
    }
    for (pin, slot) in [
        ("TXP0", bels::OPAD_TXP0),
        ("TXN0", bels::OPAD_TXN0),
        ("TXP1", bels::OPAD_TXP1),
        ("TXN1", bels::OPAD_TXN1),
    ] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), obel.wire("I"), bel.wire(pin));
    }

    let obel = vrf.find_bel_sibling(bel, bels::BUFDS0);
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), bel.wire("CLKOUT_NORTH_S"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), bel.wire("CLKOUT_SOUTH_N"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), bel.wire("GREFCLK"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel.wire("O"));
    vrf.claim_node(&[bel.fwire("CLKOUT_NORTH")]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_NORTH"),
        bel.wire("CLKOUT_NORTH_S"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKOUT_NORTH"), obel.wire("O"));
    vrf.claim_node(&[bel.fwire("CLKOUT_SOUTH")]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_SOUTH"),
        bel.wire("CLKOUT_SOUTH_N"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKOUT_SOUTH"), obel.wire("O"));
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -20, bel.slot) {
        vrf.verify_node(&[bel.fwire("CLKOUT_NORTH_S"), obel.fwire("CLKOUT_NORTH")]);
    } else {
        vrf.claim_node(&[bel.fwire("CLKOUT_NORTH_S")]);
    }
    if let Some(obel) = vrf.find_bel_delta(bel, 0, 20, bel.slot) {
        vrf.verify_node(&[bel.fwire("CLKOUT_SOUTH_N"), obel.fwire("CLKOUT_SOUTH")]);
    } else {
        vrf.claim_node(&[bel.fwire("CLKOUT_SOUTH_N")]);
    }

    for (opin, pin) in [
        ("MGT0", "RXRECCLK0"),
        ("MGT1", "RXRECCLK1"),
        ("MGT2", "REFCLKOUT"),
        ("MGT3", "TXOUTCLK0"),
        ("MGT4", "TXOUTCLK1"),
    ] {
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(pin));
    }
}

fn verify_bufds(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("IP", SitePinDir::In),
        ("IN", SitePinDir::In),
        ("O", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "BUFDS", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    for (pin, slot) in [("IP", bels::IPAD_CLKP0), ("IN", bels::IPAD_CLKN0)] {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("O"));
    }
}

fn verify_ipad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IPAD", &[("O", SitePinDir::Out)], &[]);
    vrf.claim_node(&[bel.fwire("O")]);
}

fn verify_opad(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "OPAD", &[("I", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("I")]);
}

fn verify_clk_hrow(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..10 {
        vrf.claim_node(&[bel.fwire(&format!("HCLK_L{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("HCLK_R{i}"))]);
        for j in 0..32 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HCLK_L{i}")),
                bel.wire(&format!("GCLK{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HCLK_R{i}")),
                bel.wire(&format!("GCLK{j}")),
            );
        }
    }
    for i in 0..32 {
        let orow = endev.edev.chips[bel.die].row_bufg() - 10;
        let obel = vrf.get_bel(bel.cell.with_row(orow).bel(bels::BUFGCTRL[i]));
        vrf.verify_node(&[bel.fwire(&format!("GCLK{i}")), obel.fwire("GCLK")]);
    }
    if endev.edev.col_lgt.is_some() {
        verify_mgt_conn(endev, vrf, bel, "MGT_I_L", true);
        for i in 0..5 {
            vrf.claim_node(&[bel.fwire(&format!("MGT_O_L{i}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("MGT_O_L{i}")),
                bel.wire(&format!("MGT_I_L{i}")),
            );
        }
    }
    verify_mgt_conn(endev, vrf, bel, "MGT_I_R", false);
    for i in 0..5 {
        vrf.claim_node(&[bel.fwire(&format!("MGT_O_R{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MGT_O_R{i}")),
            bel.wire(&format!("MGT_I_R{i}")),
        );
    }
}

fn verify_hclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.get_bel(bel.cell.with_col(endev.edev.col_cfg).bel(bels::CLK_HROW));
    let lr = if bel.col <= endev.edev.col_cfg {
        'L'
    } else {
        'R'
    };
    for i in 0..10 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK_O{i}")),
            bel.wire(&format!("HCLK_I{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK_I{i}")),
            obel.fwire(&format!("HCLK_{lr}{i}")),
        ]);
    }
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK_O{i}")),
            bel.wire(&format!("RCLK_I{i}")),
        );
    }
    // actually sourced from HCLK_IOI, but instead pretend it's sourced from the edge because the
    // HCLK_IOI may be missing.
    let scol = if lr == 'L' {
        endev.edev.chips[bel.die].columns.first_id().unwrap()
    } else {
        endev.edev.chips[bel.die].columns.last_id().unwrap()
    };
    if bel.col == scol {
        for i in 0..4 {
            vrf.claim_node(&[bel.fwire(&format!("RCLK_I{i}"))]);
        }
    } else {
        let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::HCLK));
        for i in 0..4 {
            vrf.verify_node(&[
                bel.fwire(&format!("RCLK_I{i}")),
                obel.fwire(&format!("RCLK_I{i}")),
            ]);
        }
    }
}

fn verify_hclk_cmt_hclk(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.find_bel_sibling(bel, bels::CLK_HROW);
    for i in 0..10 {
        vrf.claim_node(&[bel.fwire(&format!("HCLK_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK_O{i}")),
            bel.wire(&format!("HCLK_I{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK_I{i}")),
            obel.fwire(&format!("HCLK_L{i}")),
        ]);
    }
}

fn verify_hclk_cmt_giob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let srow = if bel.row < endev.edev.chips[bel.die].row_bufg() {
        endev.edev.chips[bel.die].row_bufg() - 30
    } else {
        endev.edev.chips[bel.die].row_bufg() + 20
    };
    let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::CLK_IOB));
    for i in 0..10 {
        vrf.claim_node(&[bel.fwire(&format!("GIOB_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("GIOB_O{i}")),
            bel.wire(&format!("GIOB_I{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("GIOB_I{i}")),
            obel.fwire(&format!("GIOB{i}")),
        ]);
    }
}

fn verify_mgt_conn(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    bel: &BelContext<'_>,
    pref: &str,
    is_l: bool,
) {
    let dx = if is_l { -1 } else { 1 };
    let scol = if is_l {
        endev.edev.chips[bel.die].columns.first_id().unwrap()
    } else {
        endev.edev.chips[bel.die].columns.last_id().unwrap()
    };
    if let Some(obel) = vrf.find_bel_walk(bel, dx, 0, bels::HCLK_BRAM_MGT) {
        for i in 0..5 {
            vrf.verify_node(&[
                bel.fwire(&format!("{pref}{i}")),
                obel.fwire(&format!("MGT_O{i}")),
            ]);
        }
    } else if let Some(obel) = vrf
        .find_bel(bel.cell.with_cr(scol, bel.row - 10).bel(bels::GTP_DUAL))
        .or_else(|| vrf.find_bel(bel.cell.with_cr(scol, bel.row - 10).bel(bels::GTX_DUAL)))
    {
        for i in 0..5 {
            vrf.verify_node(&[
                bel.fwire(&format!("{pref}{i}")),
                obel.fwire(&format!("MGT{i}")),
            ]);
        }
    } else if !is_l && endev.edev.col_rio.is_some() {
        let obel = vrf.get_bel(
            bel.cell
                .with_col(endev.edev.col_rio.unwrap())
                .bel(bels::RCLK),
        );
        for i in 0..5 {
            vrf.verify_node(&[
                bel.fwire(&format!("{pref}{i}")),
                obel.fwire(&format!("MGT{i}")),
            ]);
        }
    } else {
        for i in 0..5 {
            vrf.claim_node(&[bel.fwire(&format!("{pref}{i}"))]);
        }
    }
}

fn verify_hclk_bram_mgt(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    for i in 0..5 {
        vrf.claim_node(&[bel.fwire(&format!("MGT_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("MGT_O{i}")),
            bel.wire(&format!("MGT_I{i}")),
        );
    }
    let is_l = bel.col < endev.edev.col_cfg;
    verify_mgt_conn(endev, vrf, bel, "MGT_I", is_l);
}

fn verify_idelayctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "IDELAYCTRL", &[("REFCLK", SitePinDir::In)], &[]);
    vrf.claim_node(&[bel.fwire("REFCLK")]);
    let obel = vrf.find_bel_sibling(bel, bels::IOCLK);
    for i in 0..10 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REFCLK"),
            obel.wire(&format!("HCLK_O{i}")),
        );
    }
}

fn verify_bufr(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFR",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
    for slot in bels::BUFIO {
        let obel = vrf.find_bel_sibling(bel, slot);
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire_far("I"));
    }
    let obel = vrf.find_bel_sibling(bel, bels::RCLK);
    for pin in ["MGT0", "MGT1", "MGT2", "MGT3", "MGT4", "CKINT0", "CKINT1"] {
        vrf.claim_pip(bel.crd(), bel.wire("I"), obel.wire(pin));
    }
    let obel = vrf.find_bel_sibling(bel, bels::IOCLK);
    let pin = match bel.slot {
        bels::BUFR0 => "VRCLK0",
        bels::BUFR1 => "VRCLK1",
        _ => unreachable!(),
    };
    vrf.claim_pip(bel.crd(), obel.wire(pin), bel.wire("O"));
}

fn verify_bufio(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "BUFIO",
        &[("I", SitePinDir::In), ("O", SitePinDir::Out)],
        &[],
    );
    vrf.claim_node(&[bel.fwire("I")]);
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire_far("I"));

    let dy = match bel.slot {
        bels::BUFIO0 => 0,
        bels::BUFIO1 => 1,
        bels::BUFIO2 => -2,
        bels::BUFIO3 => -1,
        _ => unreachable!(),
    };
    if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, bels::ILOGIC1) {
        vrf.claim_node(&[bel.fwire_far("I"), obel.fwire("CLKOUT")]);
        vrf.claim_pip(obel.crd(), obel.wire("CLKOUT"), obel.wire("O"));
    }

    let obel = vrf.find_bel_sibling(bel, bels::IOCLK);
    let pin = match bel.slot {
        bels::BUFIO0 => "IOCLK0",
        bels::BUFIO1 => "IOCLK1",
        bels::BUFIO2 => "IOCLK2",
        bels::BUFIO3 => "IOCLK3",
        _ => unreachable!(),
    };
    vrf.claim_pip(bel.crd(), obel.wire(pin), bel.wire("O"));
}

fn verify_rclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = bel.col < endev.edev.col_cfg;
    let scol = if is_l {
        endev.edev.chips[bel.die].columns.first_id().unwrap()
    } else {
        endev.edev.chips[bel.die].columns.last_id().unwrap()
    };
    if let Some(obel) = vrf
        .find_bel(bel.cell.with_cr(scol, bel.row - 10).bel(bels::GTP_DUAL))
        .or_else(|| vrf.find_bel(bel.cell.with_cr(scol, bel.row - 10).bel(bels::GTX_DUAL)))
    {
        for i in 0..5 {
            vrf.verify_node(&[
                bel.fwire(&format!("MGT{i}")),
                obel.fwire(&format!("MGT{i}")),
            ]);
        }
    } else {
        for i in 0..5 {
            vrf.claim_node(&[bel.fwire(&format!("MGT{i}"))]);
        }
    }
}

fn verify_ioclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = vrf.get_bel(bel.cell.with_col(endev.edev.col_cfg).bel(bels::CLK_HROW));
    let lr = if bel.col <= endev.edev.col_cfg {
        'L'
    } else {
        'R'
    };
    for i in 0..10 {
        vrf.claim_node(&[bel.fwire(&format!("HCLK_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HCLK_O{i}")),
            bel.wire(&format!("HCLK_I{i}")),
        );
        vrf.verify_node(&[
            bel.fwire(&format!("HCLK_I{i}")),
            obel.fwire(&format!("HCLK_{lr}{i}")),
        ]);
    }
    for i in 0..4 {
        vrf.claim_node(&[bel.fwire(&format!("RCLK_O{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RCLK_O{i}")),
            bel.wire(&format!("RCLK_I{i}")),
        );
    }
    // actually sourced from HCLK_IOI, but instead pretend it's sourced from the edge because the
    // HCLK_IOI may be missing.
    let scol = if lr == 'L' {
        endev.edev.chips[bel.die].columns.first_id().unwrap()
    } else {
        endev.edev.chips[bel.die].columns.last_id().unwrap()
    };
    let obel = vrf.get_bel(bel.cell.with_col(scol).bel(bels::HCLK));
    for i in 0..4 {
        vrf.verify_node(&[
            bel.fwire(&format!("RCLK_I{i}")),
            obel.fwire(&format!("RCLK_I{i}")),
        ]);
    }

    let mut wires = [vec![], vec![], vec![], vec![]];
    for dy in -10..10 {
        if let Some(obel) = vrf.find_bel_delta(bel, 0, dy, bels::IOI) {
            for i in 0..4 {
                wires[i].push(obel.fwire(&format!("IOCLK{i}")));
            }
        }
    }
    for i in 0..4 {
        let pin = format!("IOCLK{i}");
        if bel.naming.pins.contains_key(&pin) {
            wires[i].push(bel.fwire(&pin));
        }
        vrf.claim_node(&wires[i]);
    }

    if bel.naming.pins.contains_key("VRCLK0") {
        vrf.claim_node(&[bel.fwire("VRCLK0")]);
        vrf.claim_node(&[bel.fwire("VRCLK1")]);
        for i in 0..4 {
            let opin = format!("RCLK_I{i}");
            vrf.claim_pip(bel.crd(), bel.wire(&opin), bel.wire("VRCLK0"));
            vrf.claim_pip(bel.crd(), bel.wire(&opin), bel.wire("VRCLK1"));
            vrf.claim_pip(bel.crd(), bel.wire(&opin), bel.wire("VRCLK_S0"));
            vrf.claim_pip(bel.crd(), bel.wire(&opin), bel.wire("VRCLK_S1"));
            vrf.claim_pip(bel.crd(), bel.wire(&opin), bel.wire("VRCLK_N0"));
            vrf.claim_pip(bel.crd(), bel.wire(&opin), bel.wire("VRCLK_N1"));
        }
        let obel_s = vrf.find_bel_delta(bel, 0, 20, bels::IOCLK);
        let mut got_s = false;
        if let Some(ref obel) = obel_s
            && obel.naming.pins.contains_key("VRCLK0")
        {
            vrf.verify_node(&[bel.fwire("VRCLK_S0"), obel.fwire("VRCLK0")]);
            vrf.verify_node(&[bel.fwire("VRCLK_S1"), obel.fwire("VRCLK1")]);
            got_s = true;
        }
        if !got_s {
            vrf.claim_node(&[bel.fwire("VRCLK_S0")]);
            vrf.claim_node(&[bel.fwire("VRCLK_S1")]);
        }
        let obel_n = vrf.find_bel_delta(bel, 0, -20, bels::IOCLK);
        let mut got_n = false;
        if let Some(ref obel) = obel_n
            && obel.naming.pins.contains_key("VRCLK0")
        {
            vrf.verify_node(&[bel.fwire("VRCLK_N0"), obel.fwire("VRCLK0")]);
            vrf.verify_node(&[bel.fwire("VRCLK_N1"), obel.fwire("VRCLK1")]);
            got_n = true;
        }
        if !got_n {
            vrf.claim_node(&[bel.fwire("VRCLK_N0")]);
            vrf.claim_node(&[bel.fwire("VRCLK_N1")]);
        }
    }
}

pub fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let slot_name = endev.edev.egrid.db.bel_slots.key(bel.slot);
    match bel.slot {
        bels::SLICE0 | bels::SLICE1 => verify_slice(vrf, bel),
        bels::DSP0 | bels::DSP1 => verify_dsp(vrf, bel),
        bels::BRAM => verify_bram(vrf, bel),
        bels::PMVBRAM => vrf.verify_bel(bel, "PMVBRAM", &[], &[]),
        bels::EMAC => vrf.verify_bel(bel, "TEMAC", &[], &[]),
        bels::PCIE => vrf.verify_bel(bel, "PCIE", &[], &[]),
        bels::PPC => vrf.verify_bel(bel, "PPC440", &[], &[]),

        _ if slot_name.starts_with("BUFGCTRL") => verify_bufgctrl(endev, vrf, bel),
        bels::BSCAN0 | bels::BSCAN1 | bels::BSCAN2 | bels::BSCAN3 => {
            vrf.verify_bel(bel, "BSCAN", &[], &[])
        }
        bels::ICAP0 | bels::ICAP1 => vrf.verify_bel(bel, "ICAP", &[], &[]),
        bels::PMV0 => vrf.verify_bel(bel, "PMV", &[], &[]),
        bels::STARTUP
        | bels::FRAME_ECC
        | bels::DCIRESET
        | bels::CAPTURE
        | bels::USR_ACCESS
        | bels::EFUSE_USR
        | bels::KEY_CLEAR
        | bels::JTAGPPC
        | bels::DCI
        | bels::GLOBALSIG => vrf.verify_bel(bel, slot_name, &[], &[]),
        bels::BUFG_MGTCLK_S | bels::BUFG_MGTCLK_N => verify_bufg_mgtclk(endev, vrf, bel),
        bels::SYSMON => verify_sysmon(endev, vrf, bel),

        bels::CLK_IOB | bels::CLK_CMT | bels::CLK_MGT => verify_clk_mux(endev, vrf, bel),

        bels::ILOGIC0 | bels::ILOGIC1 => verify_ilogic(vrf, bel),
        bels::OLOGIC0 | bels::OLOGIC1 => verify_ologic(vrf, bel),
        bels::IODELAY0 | bels::IODELAY1 => verify_iodelay(vrf, bel),
        bels::IOB0 | bels::IOB1 => verify_iob(vrf, bel),
        bels::IOI => verify_ioi_clk(vrf, bel),

        bels::DCM0 | bels::DCM1 => verify_dcm(vrf, bel),
        bels::PLL => verify_pll(vrf, bel),
        bels::CMT => verify_cmt(vrf, bel),

        bels::GTX_DUAL | bels::GTP_DUAL => verify_gt(vrf, bel),
        bels::BUFDS0 => verify_bufds(vrf, bel),
        _ if slot_name.starts_with("CRC32") => vrf.verify_bel(bel, "CRC32", &[], &[]),
        _ if slot_name.starts_with("CRC64") => vrf.verify_bel(bel, "CRC64", &[], &[]),
        _ if slot_name.starts_with("IPAD") => verify_ipad(vrf, bel),
        _ if slot_name.starts_with("OPAD") => verify_opad(vrf, bel),

        bels::CLK_HROW => verify_clk_hrow(endev, vrf, bel),
        bels::HCLK => verify_hclk(endev, vrf, bel),
        bels::HCLK_CMT_HCLK => verify_hclk_cmt_hclk(vrf, bel),
        bels::HCLK_CMT_GIOB => verify_hclk_cmt_giob(endev, vrf, bel),
        bels::HCLK_BRAM_MGT => verify_hclk_bram_mgt(endev, vrf, bel),
        _ if slot_name.starts_with("BUFR") => verify_bufr(vrf, bel),
        _ if slot_name.starts_with("BUFIO") => verify_bufio(vrf, bel),
        bels::IDELAYCTRL => verify_idelayctrl(vrf, bel),
        bels::RCLK => verify_rclk(endev, vrf, bel),
        bels::IOCLK => verify_ioclk(endev, vrf, bel),

        _ => println!("MEOW {} {:?}", slot_name, bel.name),
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
        .tile(tslots::INT)];
        let crd = vrf.xlat_tile(&nnode.names[RawTileId::from_idx(0)]).unwrap();
        vrf.claim_node(&[(crd, "ER2BEG0")]);
    }
    vrf.kill_stub_out_cond("IOI_BYP_INT_B0");
    vrf.kill_stub_out_cond("IOI_BYP_INT_B2");
    vrf.kill_stub_out_cond("IOI_BYP_INT_B3");
    vrf.kill_stub_out_cond("IOI_BYP_INT_B6");
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
