use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::virtex::{Grid, GridKind};

use crate::verify::{BelContext, SitePinDir, Verifier};

pub fn verify_bel(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        _ if bel.key.starts_with("SLICE") => {
            vrf.verify_bel(
                bel,
                "SLICE",
                &[
                    ("CIN", SitePinDir::In),
                    ("COUT", SitePinDir::Out),
                    ("F5IN", SitePinDir::In),
                    ("F5", SitePinDir::Out),
                ],
                &[],
            );
            if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, bel.row - 1), bel.key) {
                vrf.claim_node(&[bel.fwire("CIN"), obel.fwire_far("COUT")]);
            } else {
                vrf.claim_node(&[bel.fwire("CIN")]);
            }
            vrf.claim_node(&[bel.fwire("COUT")]);
            vrf.claim_pip(bel.crd(), bel.wire_far("COUT"), bel.wire("COUT"));

            vrf.claim_node(&[bel.fwire("F5")]);
            vrf.claim_node(&[bel.fwire("F5IN")]);
            let okey = match bel.key {
                "SLICE0" => "SLICE1",
                "SLICE1" => "SLICE0",
                _ => unreachable!(),
            };
            let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), okey).unwrap();
            vrf.claim_pip(bel.crd(), bel.wire("F5IN"), obel.wire("F5"));
        }
        _ if bel.key.starts_with("IOB") => {
            let mut kind = "IOB";
            let mut pins = Vec::new();
            if bel.name.unwrap().starts_with("EMPTY") {
                kind = "EMPTYIOB";
            }
            if (bel.col == grid.col_lio() || bel.col == grid.col_rio())
                && ((bel.row == grid.row_mid() && bel.key == "IOB3")
                    || (bel.row == grid.row_mid() - 1 && bel.key == "IOB1"))
            {
                kind = "PCIIOB";
                pins.push(("PCI", SitePinDir::Out));
            }
            if grid.kind != GridKind::Virtex
                && (bel.row == grid.row_bio() || bel.row == grid.row_tio())
                && ((bel.col == grid.col_clk() && bel.key == "IOB2")
                    || (bel.col == grid.col_clk() - 1 && bel.key == "IOB1"))
            {
                kind = "DLLIOB";
                pins.push(("DLLFB", SitePinDir::Out));
            }
            vrf.verify_bel(bel, kind, &pins, &[]);
        }
        _ if bel.key.starts_with("TBUF") => {
            vrf.verify_bel(bel, "TBUF", &[("O", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("O")]);
        }
        "TBUS" => {
            let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), "TBUF0").unwrap();
            vrf.claim_pip(bel.crd(), bel.wire("BUS0"), obel.wire("O"));
            vrf.claim_pip(bel.crd(), bel.wire("BUS2"), obel.wire("O"));
            let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), "TBUF1").unwrap();
            vrf.claim_pip(bel.crd(), bel.wire("BUS1"), obel.wire("O"));
            vrf.claim_pip(bel.crd(), bel.wire("BUS3"), obel.wire("O"));
            if bel.naming.pins.contains_key("BUS3_E") {
                let col_r = grid.col_rio();
                if bel.col.to_idx() < col_r.to_idx() - 5 {
                    vrf.claim_node(&[bel.fwire("BUS3_E")]);
                }
                vrf.claim_pip(bel.crd(), bel.wire("BUS3"), bel.wire("BUS3_E"));
                vrf.claim_pip(bel.crd(), bel.wire("BUS3_E"), bel.wire("BUS3"));
                let mut col_r = bel.col + 1;
                loop {
                    if let Some(obel) = vrf.find_bel(bel.slr, (col_r, bel.row), "TBUS") {
                        vrf.verify_node(&[bel.fwire("BUS0"), obel.fwire("BUS1")]);
                        vrf.verify_node(&[bel.fwire("BUS1"), obel.fwire("BUS2")]);
                        vrf.verify_node(&[bel.fwire("BUS2"), obel.fwire("BUS3")]);
                        vrf.verify_node(&[bel.fwire("BUS3_E"), obel.fwire("BUS0")]);
                        break;
                    } else {
                        col_r += 1;
                    }
                }
            }
            if bel.naming.pins.contains_key("OUT") {
                vrf.claim_pip(bel.crd(), bel.wire("OUT"), bel.wire("BUS2"));
            }
        }
        "BRAM" => {
            vrf.verify_bel(bel, "BLOCKRAM", &[], &[]);
        }
        "STARTUP" | "CAPTURE" | "BSCAN" => {
            vrf.verify_bel(bel, bel.key, &[], &[]);
        }
        _ if bel.key.starts_with("GCLKIOB") => {
            vrf.verify_bel(bel, "GCLKIOB", &[], &[]);
        }
        _ if bel.key.starts_with("BUFG") => {
            vrf.verify_bel(bel, "GCLK", &[], &["OUT.GLOBAL"]);
            vrf.claim_node(&[bel.fwire("OUT.GLOBAL")]);
            vrf.claim_pip(bel.crd(), bel.wire("OUT.GLOBAL"), bel.wire("OUT"));
        }
        "IOFB0" => {
            let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), "IOB2").unwrap();
            vrf.verify_node(&[bel.fwire("O"), obel.fwire("DLLFB")]);
        }
        "IOFB1" => {
            let obel = vrf
                .find_bel(bel.slr, (bel.col - 1, bel.row), "IOB1")
                .unwrap();
            vrf.verify_node(&[bel.fwire("O"), obel.fwire("DLLFB")]);
        }
        "PCILOGIC" => {
            vrf.verify_bel(
                bel,
                "PCILOGIC",
                &[("IRDY", SitePinDir::In), ("TRDY", SitePinDir::In)],
                &[],
            );
            for pin in ["IRDY", "TRDY"] {
                for pip in &bel.naming.pins[pin].pips {
                    vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
                }
                vrf.claim_node(&[bel.fwire(pin)]);
                vrf.claim_node(&[bel.fwire_far(pin)]);
            }
            let obel = vrf
                .find_bel(bel.slr, (bel.col, grid.row_mid()), "IOB3")
                .unwrap();
            vrf.verify_node(&[bel.fwire_far("IRDY"), obel.fwire("PCI")]);
            let obel = vrf
                .find_bel(bel.slr, (bel.col, grid.row_mid() - 1), "IOB1")
                .unwrap();
            vrf.verify_node(&[bel.fwire_far("TRDY"), obel.fwire("PCI")]);
        }
        "DLL" => {
            vrf.verify_bel(bel, "DLL", &[], &[]);
        }
        "CLKC" => {
            for (opin, ipin, srow, sbel) in [
                ("OUT0", "IN0", grid.row_bio(), "BUFG0"),
                ("OUT1", "IN1", grid.row_bio(), "BUFG1"),
                ("OUT2", "IN2", grid.row_tio(), "BUFG0"),
                ("OUT3", "IN3", grid.row_tio(), "BUFG1"),
            ] {
                vrf.claim_node(&[bel.fwire(opin)]);
                vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
                let obel = vrf.find_bel(bel.slr, (grid.col_clk(), srow), sbel).unwrap();
                vrf.verify_node(&[bel.fwire(ipin), obel.fwire("OUT.GLOBAL")]);
            }
        }
        "GCLKC" => {
            for (opin, ipin) in [
                ("OUT0", "IN0"),
                ("OUT1", "IN1"),
                ("OUT2", "IN2"),
                ("OUT3", "IN3"),
            ] {
                vrf.claim_node(&[bel.fwire(opin)]);
                vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
                let obel = vrf
                    .find_bel(bel.slr, (grid.col_clk(), bel.row), "CLKC")
                    .unwrap();
                vrf.verify_node(&[bel.fwire(ipin), obel.fwire(opin)]);
            }
        }
        "BRAM_CLKH" => {
            for (opin, ipin) in [
                ("OUT0", "IN0"),
                ("OUT1", "IN1"),
                ("OUT2", "IN2"),
                ("OUT3", "IN3"),
            ] {
                vrf.claim_pip(bel.crd(), bel.wire(opin), bel.wire(ipin));
                let obel = vrf
                    .find_bel(bel.slr, (grid.col_clk(), bel.row), "CLKC")
                    .unwrap();
                vrf.verify_node(&[bel.fwire(ipin), obel.fwire(opin)]);
            }
        }
        "CLKV" => {
            for (opinl, opinr, ipin, opin) in [
                ("OUT_L0", "OUT_R0", "IN0", "OUT0"),
                ("OUT_L1", "OUT_R1", "IN1", "OUT1"),
                ("OUT_L2", "OUT_R2", "IN2", "OUT2"),
                ("OUT_L3", "OUT_R3", "IN3", "OUT3"),
            ] {
                vrf.claim_pip(bel.crd(), bel.wire(opinl), bel.wire(ipin));
                vrf.claim_pip(bel.crd(), bel.wire(opinr), bel.wire(ipin));
                let obel = vrf
                    .find_bel(bel.slr, (bel.col + 1, grid.row_clk()), "GCLKC")
                    .unwrap();
                vrf.verify_node(&[bel.fwire(ipin), obel.fwire(opin)]);
            }
        }
        "CLKV_BRAM_BOT" | "CLKV_BRAM_TOP" => {
            for (opinl, opinr, ipin) in [
                ("OUT_L0", "OUT_R0", "IN0"),
                ("OUT_L1", "OUT_R1", "IN1"),
                ("OUT_L2", "OUT_R2", "IN2"),
                ("OUT_L3", "OUT_R3", "IN3"),
            ] {
                vrf.claim_pip(bel.crd(), bel.wire(opinl), bel.wire(ipin));
                vrf.claim_pip(bel.crd(), bel.wire(opinr), bel.wire(ipin));
            }
        }
        "CLKV_BRAM" => {
            for i in 0..4 {
                let ipin = format!("IN{i}");
                for j in 0..4 {
                    let opinl = format!("OUT_L{j}_{i}");
                    let opinr = format!("OUT_R{j}_{i}");
                    vrf.claim_pip(bel.crd(), bel.wire(&opinl), bel.wire(&ipin));
                    vrf.claim_pip(bel.crd(), bel.wire(&opinr), bel.wire(&ipin));
                }
            }
        }
        _ => unreachable!(),
    }
}
