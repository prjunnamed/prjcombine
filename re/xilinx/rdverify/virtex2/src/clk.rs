use prjcombine_interconnect::dir::Dir;
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_virtex2::{
    chip::{ChipKind, Dcms},
    defs,
};

use crate::get_bel_iob;

pub fn verify_bufgmux(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    let idx = defs::bslots::BUFGMUX.index_of(bel.slot).unwrap();
    if endev.edev.chip.kind == ChipKind::FpgaCore {
        vrf.verify_bel(bel, "BUFG", &[("I", SitePinDir::In)], &["CLK"]);
        vrf.claim_net(&[bel.fwire("I")]);
        vrf.claim_pip(bel.crd(), bel.wire("I"), bel.wire("CLK"));
        let edge = if bel.row == endev.chip.row_s() {
            Dir::S
        } else if bel.row == endev.chip.row_n() {
            Dir::N
        } else {
            unreachable!()
        };
        let crd = endev.chip.get_clk_io(edge, idx).unwrap();
        let obel = vrf.get_bel(endev.chip.get_io_loc(crd));
        vrf.claim_net(&[bel.fwire("CKI"), obel.fwire("IBUF")]);
        vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKI"));
    // let obel = vrf.find_bel_sibling(bel, "VCC");
    // vrf.claim_pip(bel.crd(), bel.wire_far("CLK"), obel.wire("VCCOUT"));
    } else {
        vrf.verify_bel(
            bel,
            "BUFGMUX",
            &[("I0", SitePinDir::In), ("I1", SitePinDir::In)],
            &["CLK"],
        );
        vrf.claim_net(&[bel.fwire("I0")]);
        vrf.claim_net(&[bel.fwire("I1")]);
        vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("CLK"));
        let idx = defs::bslots::BUFGMUX.index_of(bel.slot).unwrap();
        let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFGMUX[idx ^ 1]);
        vrf.claim_pip(bel.crd(), bel.wire("I1"), obel.wire("CLK"));
        let edge = if bel.row == endev.chip.row_s() {
            Dir::S
        } else if bel.row == endev.chip.row_n() {
            Dir::N
        } else if bel.col == endev.chip.col_w() {
            Dir::W
        } else if bel.col == endev.chip.col_e() {
            Dir::E
        } else {
            unreachable!()
        };
        if endev.chip.kind.is_virtex2() || endev.chip.kind == ChipKind::Spartan3 {
            if let Some(crd) = endev.chip.get_clk_io(edge, idx) {
                let obel = get_bel_iob(endev, vrf, crd);
                vrf.claim_net(&[bel.fwire("CKI"), obel.fwire("IBUF")]);
                vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
            } else {
                vrf.claim_net(&[bel.fwire("CKI")]);
            }
            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKI"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_L"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_R"));
            vrf.claim_net(&[bel.fwire("DCM_OUT_L")]);
            vrf.claim_net(&[bel.fwire("DCM_OUT_R")]);
            if endev.chip.kind.is_virtex2() {
                for pin in ["DCM_PAD_L", "DCM_PAD_R"] {
                    vrf.claim_net(&[bel.fwire(pin)]);
                    vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKI"));
                }
            } else {
                vrf.claim_net(&[bel.fwire("DCM_PAD")]);
                vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD"), bel.wire("CKI"));
            }
        } else if matches!(edge, Dir::S | Dir::N) {
            let crd = endev.chip.get_clk_io(edge, idx).unwrap();
            let obel = get_bel_iob(endev, vrf, crd);
            vrf.claim_net(&[bel.fwire("CKIR"), obel.fwire("IBUF")]);
            vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
            let crd = endev.chip.get_clk_io(edge, idx + 4).unwrap();
            let obel = get_bel_iob(endev, vrf, crd);
            vrf.claim_net(&[bel.fwire("CKIL"), obel.fwire("IBUF")]);
            vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKIL"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKIR"));

            let mut has_dcm_l = true;
            let mut has_dcm_r = true;
            if endev.chip.kind == ChipKind::Spartan3E {
                if endev.chip.dcms == Some(Dcms::Two) {
                    has_dcm_l = false;
                }
            } else {
                if endev.chip.dcms == Some(Dcms::Two) && bel.row == endev.chip.row_s() {
                    has_dcm_l = false;
                    has_dcm_r = false;
                }
            }
            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_L"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_R"));
            if has_dcm_l {
                vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD_L"), bel.wire("CKIL"));
                let pip = &bel.naming.pins["DCM_OUT_L"].pips[0];
                vrf.claim_net(&[bel.fwire("DCM_OUT_L"), (bel.crds[pip.tile], &pip.wire_to)]);
                vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
                let dy = match edge {
                    Dir::S => 1,
                    Dir::N => -1,
                    _ => unreachable!(),
                };
                let obel = vrf
                    .find_bel_delta(bel, -1, dy, defs::bslots::DCMCONN_S3E)
                    .unwrap();
                let (dcm_pad_pin, dcm_out_pin) = match (edge, idx) {
                    (Dir::N, 0) => ("CLKPAD0", "OUT0"),
                    (Dir::N, 1) => ("CLKPAD1", "OUT1"),
                    (Dir::N, 2) => ("CLKPAD2", "OUT2"),
                    (Dir::N, 3) => ("CLKPAD3", "OUT3"),
                    (Dir::S, 0) => ("CLKPAD3", "OUT0"),
                    (Dir::S, 1) => ("CLKPAD2", "OUT1"),
                    (Dir::S, 2) => ("CLKPAD1", "OUT2"),
                    (Dir::S, 3) => ("CLKPAD0", "OUT3"),
                    _ => unreachable!(),
                };
                vrf.verify_net(&[bel.fwire("DCM_PAD_L"), obel.fwire(dcm_pad_pin)]);
                vrf.verify_net(&[
                    (bel.crds[pip.tile], &pip.wire_from),
                    obel.fwire(dcm_out_pin),
                ]);
            } else {
                vrf.claim_net(&[bel.fwire("DCM_OUT_L")]);
            }
            if has_dcm_r {
                vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD_R"), bel.wire("CKIR"));
                let pip = &bel.naming.pins["DCM_OUT_R"].pips[0];
                vrf.claim_net(&[bel.fwire("DCM_OUT_R"), (bel.crds[pip.tile], &pip.wire_to)]);
                vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
                let dy = match edge {
                    Dir::S => 1,
                    Dir::N => -1,
                    _ => unreachable!(),
                };
                let obel = vrf
                    .find_bel_delta(bel, 0, dy, defs::bslots::DCMCONN_S3E)
                    .unwrap();
                let (dcm_pad_pin, dcm_out_pin) = match (edge, idx) {
                    (Dir::N, 0) => ("CLKPAD2", "OUT0"),
                    (Dir::N, 1) => ("CLKPAD3", "OUT1"),
                    (Dir::N, 2) => ("CLKPAD0", "OUT2"),
                    (Dir::N, 3) => ("CLKPAD1", "OUT3"),
                    (Dir::S, 0) => ("CLKPAD0", "OUT0"),
                    (Dir::S, 1) => ("CLKPAD1", "OUT1"),
                    (Dir::S, 2) => ("CLKPAD2", "OUT2"),
                    (Dir::S, 3) => ("CLKPAD3", "OUT3"),
                    _ => unreachable!(),
                };
                vrf.verify_net(&[bel.fwire("DCM_PAD_R"), obel.fwire(dcm_pad_pin)]);
                vrf.verify_net(&[
                    (bel.crds[pip.tile], &pip.wire_from),
                    obel.fwire(dcm_out_pin),
                ]);
            } else {
                vrf.claim_net(&[bel.fwire("DCM_OUT_R")]);
            }
        } else {
            let crd = endev.chip.get_clk_io(edge, idx).unwrap();
            let obel = get_bel_iob(endev, vrf, crd);
            vrf.verify_net(&[bel.fwire("CKI"), obel.fwire("IBUF")]);
            vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKI"));

            vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT"));
            if endev.chip.dcms == Some(Dcms::Eight) {
                let pad_pin;
                if endev.chip.kind != ChipKind::Spartan3A {
                    pad_pin = "CKI";
                } else {
                    pad_pin = "DCM_PAD";
                    vrf.claim_net(&[bel.fwire("CKI")]);
                    vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD"), bel.wire("CKI"));
                }
                let scol = if endev.chip.kind == ChipKind::Spartan3E {
                    match edge {
                        Dir::W => endev.chip.col_w() + 9,
                        Dir::E => endev.chip.col_e() - 9,
                        _ => unreachable!(),
                    }
                } else {
                    match edge {
                        Dir::W => endev.chip.col_w() + 3,
                        Dir::E => endev.chip.col_e() - 6,
                        _ => unreachable!(),
                    }
                };
                let srow = if idx < 4 {
                    endev.chip.row_mid()
                } else {
                    endev.chip.row_mid() - 1
                };
                let obel = vrf.get_bel(bel.cell.with_cr(scol, srow).bel(defs::bslots::DCMCONN_S3E));
                let (dcm_pad_pin, dcm_out_pin) = match idx {
                    0 | 4 => ("CLKPAD0", "OUT0"),
                    1 | 5 => ("CLKPAD1", "OUT1"),
                    2 | 6 => ("CLKPAD2", "OUT2"),
                    3 | 7 => ("CLKPAD3", "OUT3"),
                    _ => unreachable!(),
                };
                vrf.verify_net(&[bel.fwire(pad_pin), obel.fwire(dcm_pad_pin)]);
                vrf.verify_net(&[bel.fwire("DCM_OUT"), obel.fwire(dcm_out_pin)]);
            } else {
                vrf.claim_net(&[bel.fwire("CKI")]);
                vrf.claim_net(&[bel.fwire("DCM_OUT")]);
            }
            let obel = vrf.find_bel_sibling(bel, defs::bslots::VCC);
            vrf.claim_pip(bel.crd(), bel.wire_far("CLK"), obel.wire("VCCOUT"));
            vrf.claim_pip(bel.crd(), bel.wire("S"), obel.wire("VCCOUT"));
        }
    }
}

pub fn verify_hclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    for i in 0..8 {
        for bt in ["B", "T"] {
            if bel.tcls.ends_with("_S") && bt == "T" {
                continue;
            }
            if bel.tcls.ends_with("_N") && bt == "B" {
                continue;
            }
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{bt}{i}")),
                bel.wire(&format!("IN{i}")),
            );
        }
        if endev.chip.kind.is_virtex2() {
            let lr = if bel.col < endev.chip.col_clk {
                'L'
            } else {
                'R'
            };
            let obel = vrf.get_bel(
                bel.cell
                    .with_col(endev.chip.col_clk)
                    .bel(defs::bslots::HROW),
            );
            vrf.verify_net(&[
                bel.fwire(&format!("IN{i}")),
                obel.fwire(&format!("OUT_{lr}{i}")),
            ]);
        } else if let Some((col_cl, col_cr)) = endev.chip.cols_clkv {
            let scol = if bel.col < endev.chip.col_clk {
                col_cl
            } else {
                col_cr
            };
            let lr = if bel.col < scol { 'L' } else { 'R' };
            let obel = vrf.get_bel(bel.cell.with_col(scol).bel(defs::bslots::HROW));
            vrf.verify_net(&[
                bel.fwire(&format!("IN{i}")),
                obel.fwire(&format!("OUT_{lr}{i}")),
            ]);
        } else {
            let lr = if bel.col < endev.chip.col_clk {
                'L'
            } else {
                'R'
            };
            let obel = vrf.get_bel(
                bel.cell
                    .with_cr(endev.chip.col_clk, endev.chip.row_mid())
                    .bel(defs::bslots::CLKC_50A),
            );
            vrf.verify_net(&[
                bel.fwire(&format!("IN{i}")),
                obel.fwire(&format!("OUT_{lr}{i}")),
            ]);
        }
    }
}

pub fn verify_hrow(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    if endev.chip.kind.is_virtex2() {
        for i in 0..8 {
            for lr in ['L', 'R'] {
                vrf.claim_net(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
                for bt in ['B', 'T'] {
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("OUT_{lr}{i}")),
                        bel.wire(&format!("IN_{bt}{i}")),
                    );
                }
            }
            for bt in ['B', 'T'] {
                let obel = vrf.get_bel(
                    bel.cell
                        .with_cr(endev.chip.col_clk, endev.chip.row_mid())
                        .bel(defs::bslots::CLKC),
                );
                vrf.verify_net(&[
                    bel.fwire(&format!("IN_{bt}{i}")),
                    obel.fwire(&format!("OUT_{bt}{i}")),
                ]);
            }
        }
    } else {
        for i in 0..8 {
            for lr in ['L', 'R'] {
                vrf.claim_net(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("OUT_{lr}{i}")),
                    bel.wire(&format!("IN{i}")),
                );
            }
            let ud = if bel.row < endev.chip.row_mid() {
                "B"
            } else {
                "T"
            };
            let obel = vrf.get_bel(
                bel.cell
                    .with_row(endev.chip.row_mid())
                    .bel(defs::bslots::CLKQC),
            );
            vrf.verify_net(&[
                bel.fwire(&format!("IN{i}")),
                obel.fwire(&format!("OUT_{ud}{i}")),
            ]);
        }
    }
}

pub fn verify_clkc_v2(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    for i in 0..8 {
        for bt in ['B', 'T'] {
            vrf.claim_net(&[(bel.crd(), bel.wire(&format!("OUT_{bt}{i}")))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{bt}{i}")),
                bel.wire(&format!("IN_{bt}{i}")),
            );
            let srow = if bt == 'B' {
                endev.chip.row_s()
            } else {
                endev.chip.row_n()
            };
            let obel = vrf.get_bel(
                bel.cell
                    .with_cr(endev.chip.col_clk, srow)
                    .bel(defs::bslots::BUFGMUX[i]),
            );
            vrf.verify_net(&[bel.fwire(&format!("IN_{bt}{i}")), obel.fwire_far("O")]);
        }
    }
}

pub fn verify_clkc_s3(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    for i in 0..8 {
        let (bt, j) = if i < 4 { ('B', i) } else { ('T', i - 4) };
        vrf.claim_net(&[bel.fwire(&format!("OUT{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("OUT{i}")),
            bel.wire(&format!("IN_{bt}{j}")),
        );
        let srow = if bt == 'B' {
            endev.chip.row_s()
        } else {
            endev.chip.row_n()
        };
        let obel = vrf.get_bel(
            bel.cell
                .with_cr(endev.chip.col_clk, srow)
                .bel(defs::bslots::BUFGMUX[j]),
        );
        vrf.verify_net(&[bel.fwire(&format!("IN_{bt}{j}")), obel.fwire_far("O")]);
    }
}

pub fn verify_clkc_50a(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    for i in 0..8 {
        let (bt, j) = if i < 4 { ('B', i) } else { ('T', i - 4) };
        for lr in ['L', 'R'] {
            vrf.claim_net(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{lr}{i}")),
                bel.wire(&format!("IN_{bt}{j}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{lr}{i}")),
                bel.wire(&format!("IN_{lr}{i}")),
            );
            let scol = if lr == 'L' {
                endev.chip.col_w()
            } else {
                endev.chip.col_e()
            };
            let obel = vrf.get_bel(
                bel.cell
                    .with_cr(scol, endev.chip.row_mid())
                    .bel(defs::bslots::BUFGMUX[i]),
            );
            vrf.verify_net(&[bel.fwire(&format!("IN_{lr}{i}")), obel.fwire_far("O")]);
        }
        let srow = if bt == 'B' {
            endev.chip.row_s()
        } else {
            endev.chip.row_n()
        };
        let obel = vrf.get_bel(
            bel.cell
                .with_cr(endev.chip.col_clk, srow)
                .bel(defs::bslots::BUFGMUX[j]),
        );
        vrf.verify_net(&[bel.fwire(&format!("IN_{bt}{j}")), obel.fwire_far("O")]);
    }
}

pub fn verify_gclkvm(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    for i in 0..8 {
        for bt in ["B", "T"] {
            vrf.claim_net(&[bel.fwire(&format!("OUT_{bt}{i}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("OUT_{bt}{i}")),
                bel.wire(&format!("IN_CORE{i}")),
            );
            if endev.chip.kind.is_spartan3ea() {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&format!("OUT_{bt}{i}")),
                    bel.wire(&format!("IN_LR{i}")),
                );
            }
        }
        let obel = vrf.get_bel(
            bel.cell
                .with_col(endev.chip.col_clk)
                .bel(defs::bslots::CLKC),
        );
        vrf.verify_net(&[
            bel.fwire(&format!("IN_CORE{i}")),
            obel.fwire(&format!("OUT{i}")),
        ]);
        if endev.chip.kind.is_spartan3ea() {
            let scol = if bel.col < endev.chip.col_clk {
                endev.chip.col_w()
            } else {
                endev.chip.col_e()
            };
            let obel = vrf.get_bel(
                bel.cell
                    .with_cr(scol, endev.chip.row_mid())
                    .bel(defs::bslots::BUFGMUX[i]),
            );
            vrf.verify_net(&[bel.fwire(&format!("IN_LR{i}")), obel.fwire_far("O")]);
        }
    }
}

pub fn verify_dcmconn(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    let opin_pad;
    let pins_out;
    let pins_pad;
    if endev.chip.kind.is_virtex2() {
        pins_out = &[
            ("OUTBUS0", "OUT0", 0),
            ("OUTBUS1", "OUT1", 1),
            ("OUTBUS2", "OUT2", 2),
            ("OUTBUS3", "OUT3", 3),
            ("OUTBUS4", "OUT0", 4),
            ("OUTBUS5", "OUT1", 5),
            ("OUTBUS6", "OUT2", 6),
            ("OUTBUS7", "OUT3", 7),
        ][..];
        if bel.col < endev.chip.col_clk {
            opin_pad = "DCM_PAD_L";
            pins_pad = &[
                ("CLKPAD0", "CLKPADBUS0", 4),
                ("CLKPAD1", "CLKPADBUS1", 5),
                ("CLKPAD2", "CLKPADBUS2", 6),
                ("CLKPAD3", "CLKPADBUS3", 7),
                ("CLKPAD4", "CLKPADBUS4", 0),
                ("CLKPAD5", "CLKPADBUS5", 1),
                ("CLKPAD6", "CLKPADBUS6", 2),
                ("CLKPAD7", "CLKPADBUS7", 3),
            ][..];
        } else {
            opin_pad = "DCM_PAD_R";
            pins_pad = &[
                ("CLKPAD0", "CLKPADBUS0", 0),
                ("CLKPAD1", "CLKPADBUS1", 1),
                ("CLKPAD2", "CLKPADBUS2", 2),
                ("CLKPAD3", "CLKPADBUS3", 3),
                ("CLKPAD4", "CLKPADBUS4", 4),
                ("CLKPAD5", "CLKPADBUS5", 5),
                ("CLKPAD6", "CLKPADBUS6", 6),
                ("CLKPAD7", "CLKPADBUS7", 7),
            ][..];
        }
    } else {
        pins_out = &[
            ("OUTBUS0", "OUT0", 0),
            ("OUTBUS1", "OUT1", 1),
            ("OUTBUS2", "OUT2", 2),
            ("OUTBUS3", "OUT3", 3),
        ][..];
        opin_pad = "DCM_PAD";
        pins_pad = &[
            ("CLKPAD0", "CLKPADBUS0", 0),
            ("CLKPAD1", "CLKPADBUS1", 1),
            ("CLKPAD2", "CLKPADBUS2", 2),
            ("CLKPAD3", "CLKPADBUS3", 3),
        ][..];
    }
    let opin_out = if bel.col < endev.chip.col_clk {
        "DCM_OUT_L"
    } else {
        "DCM_OUT_R"
    };
    for &(pin_o, pin_i, obk) in pins_out {
        vrf.claim_pip(bel.crd(), bel.wire(pin_o), bel.wire(pin_i));
        let obel = vrf.get_bel(
            bel.cell
                .with_col(endev.chip.col_clk)
                .bel(defs::bslots::BUFGMUX[obk]),
        );
        vrf.verify_net(&[bel.fwire(pin_o), obel.fwire(opin_out)]);
    }
    for &(pin_o, pin_i, obk) in pins_pad {
        vrf.claim_pip(bel.crd(), bel.wire(pin_o), bel.wire(pin_i));
        let obel = vrf.get_bel(
            bel.cell
                .with_col(endev.chip.col_clk)
                .bel(defs::bslots::BUFGMUX[obk]),
        );
        vrf.verify_net(&[bel.fwire(pin_i), obel.fwire(opin_pad)]);
    }
}

pub fn verify_brefclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext) {
    vrf.claim_net(&[bel.fwire("BREFCLK")]);
    vrf.claim_net(&[bel.fwire("BREFCLK2")]);
    if bel.row == endev.chip.row_s() {
        let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFGMUX[6]);
        vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("CKI"));
        let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFGMUX[0]);
        vrf.claim_pip(bel.crd(), bel.wire("BREFCLK2"), obel.wire_far("CKI"));
    } else {
        let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFGMUX[4]);
        vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("CKI"));
        let obel = vrf.find_bel_sibling(bel, defs::bslots::BUFGMUX[2]);
        vrf.claim_pip(bel.crd(), bel.wire("BREFCLK2"), obel.wire_far("CKI"));
    }
}
