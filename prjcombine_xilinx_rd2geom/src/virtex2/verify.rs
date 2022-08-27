use crate::verify::{BelContext, SitePinDir, Verifier};
use prjcombine_entity::EntityId;
use prjcombine_rawdump::Coord;
use prjcombine_xilinx_geom::int::NodeRawTileId;
use prjcombine_xilinx_geom::virtex2::{ColumnKind, Dcms, Edge, Grid, GridKind, IoDiffKind};
use prjcombine_xilinx_geom::{BelId, ColId, RowId, SlrId};

fn verify_pci_ce(
    grid: &Grid,
    vrf: &mut Verifier,
    slr: SlrId,
    col: ColId,
    row: RowId,
    crd: Coord,
    wire: &str,
) {
    if col == grid.col_left() || col == grid.col_right() {
        if row < grid.row_mid() {
            for &(srow, _, _) in &grid.rows_hclk {
                if srow > grid.row_mid() {
                    break;
                }
                if row < srow {
                    let obel = vrf.find_bel(slr, (col, srow), "PCI_CE_S").unwrap();
                    vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
                    return;
                }
            }
        } else {
            for &(srow, _, _) in grid.rows_hclk.iter().rev() {
                if srow <= grid.row_mid() {
                    break;
                }
                if row >= srow {
                    let obel = vrf.find_bel(slr, (col, srow), "PCI_CE_N").unwrap();
                    vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
                    return;
                }
            }
        }
        let obel = vrf
            .find_bel(slr, (col, grid.row_mid() - 1), "PCILOGICSE")
            .unwrap();
        let pip = &obel.naming.pins["PCI_CE"].pips[0];
        vrf.verify_node(&[(obel.crds[pip.tile], &pip.wire_to), (crd, wire)]);
    } else {
        if grid.kind == GridKind::Spartan3A {
            if let Some((col_l, col_r)) = grid.cols_clkv {
                if col >= col_l && col < col_r {
                    let (scol, kind) = if col < grid.col_clk {
                        (col_l, "PCI_CE_E")
                    } else {
                        (col_r, "PCI_CE_W")
                    };
                    let obel = vrf.find_bel(slr, (scol, row), kind).unwrap();
                    vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
                    return;
                }
            }
        }
        let scol = if col < grid.col_clk {
            grid.col_left()
        } else {
            grid.col_right()
        };
        let obel = vrf.find_bel(slr, (scol, row), "PCI_CE_CNR").unwrap();
        vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
    }
}

pub fn verify_bel(grid: &Grid, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        "RLL" => {
            let mut pins = Vec::new();
            if bel.bel.pins.is_empty() {
                for pin in bel.naming.pins.keys() {
                    pins.push((&**pin, SitePinDir::In));
                    vrf.claim_node(&[bel.fwire(pin)]);
                }
            }
            vrf.verify_bel(bel, "RESERVED_LL", &pins, &[]);
        }
        _ if bel.key.starts_with("SLICE") => {
            if grid.kind.is_virtex2() {
                vrf.verify_bel(
                    bel,
                    "SLICE",
                    &[
                        ("DX", SitePinDir::In),
                        ("DY", SitePinDir::In),
                        ("FXINA", SitePinDir::In),
                        ("FXINB", SitePinDir::In),
                        ("F5", SitePinDir::Out),
                        ("FX", SitePinDir::Out),
                        ("CIN", SitePinDir::In),
                        ("COUT", SitePinDir::Out),
                        ("SHIFTIN", SitePinDir::In),
                        ("SHIFTOUT", SitePinDir::Out),
                        ("ALTDIG", SitePinDir::In),
                        ("DIG", SitePinDir::Out),
                        ("SLICEWE0", SitePinDir::In),
                        ("SLICEWE1", SitePinDir::In),
                        ("SLICEWE2", SitePinDir::In),
                        ("BXOUT", SitePinDir::Out),
                        ("BYOUT", SitePinDir::Out),
                        ("BYINVOUT", SitePinDir::Out),
                        ("SOPIN", SitePinDir::In),
                        ("SOPOUT", SitePinDir::Out),
                    ],
                    &[],
                );
                vrf.claim_node(&[bel.fwire("DX")]);
                vrf.claim_pip(bel.crd(), bel.wire("DX"), bel.wire("X"));
                vrf.claim_node(&[bel.fwire("DY")]);
                vrf.claim_pip(bel.crd(), bel.wire("DY"), bel.wire("Y"));
                for pin in [
                    "F5", "FX", "COUT", "SHIFTOUT", "DIG", "BYOUT", "BXOUT", "BYINVOUT", "SOPOUT",
                ] {
                    vrf.claim_node(&[bel.fwire(pin)]);
                }
                for (dbel, dpin, sbel, spin) in [
                    ("SLICE0", "FXINA", "SLICE0", "F5"),
                    ("SLICE0", "FXINB", "SLICE1", "F5"),
                    ("SLICE1", "FXINA", "SLICE0", "FX"),
                    ("SLICE1", "FXINB", "SLICE2", "FX"),
                    ("SLICE2", "FXINA", "SLICE2", "F5"),
                    ("SLICE2", "FXINB", "SLICE3", "F5"),
                    ("SLICE3", "FXINA", "SLICE1", "FX"),
                    // SLICE3 FXINB <- top's SLICE1 FX

                    // SLICE0 CIN <- bot's SLICE1 COUT
                    ("SLICE1", "CIN", "SLICE0", "COUT"),
                    // SLICE2 CIN <- bot's SLICE3 COUT
                    ("SLICE3", "CIN", "SLICE2", "COUT"),
                    ("SLICE0", "SHIFTIN", "SLICE1", "SHIFTOUT"),
                    ("SLICE1", "SHIFTIN", "SLICE2", "SHIFTOUT"),
                    ("SLICE2", "SHIFTIN", "SLICE3", "SHIFTOUT"),
                    // SLICE3 SHIFTIN disconnected? supposed to be top's SLICE0 SHIFTOUT?
                    ("SLICE3", "DIG_LOCAL", "SLICE3", "DIG"),
                    ("SLICE0", "ALTDIG", "SLICE1", "DIG"),
                    ("SLICE1", "ALTDIG", "SLICE3", "DIG_LOCAL"),
                    ("SLICE2", "ALTDIG", "SLICE3", "DIG_LOCAL"),
                    ("SLICE3", "ALTDIG", "SLICE3", "DIG_S"), // top's SLICE3 DIG
                    ("SLICE1", "BYOUT_LOCAL", "SLICE1", "BYOUT"),
                    ("SLICE0", "BYINVOUT_LOCAL", "SLICE0", "BYINVOUT"),
                    ("SLICE1", "BYINVOUT_LOCAL", "SLICE1", "BYINVOUT"),
                    ("SLICE0", "SLICEWE0", "SLICE0", "BXOUT"),
                    ("SLICE1", "SLICEWE0", "SLICE1", "BXOUT"),
                    ("SLICE2", "SLICEWE0", "SLICE0", "BXOUT"),
                    ("SLICE3", "SLICEWE0", "SLICE1", "BXOUT"),
                    ("SLICE0", "SLICEWE1", "SLICE0", "BYOUT"),
                    ("SLICE1", "SLICEWE1", "SLICE0", "BYINVOUT_LOCAL"),
                    ("SLICE2", "SLICEWE1", "SLICE0", "BYOUT"),
                    ("SLICE3", "SLICEWE1", "SLICE0", "BYINVOUT_LOCAL"),
                    ("SLICE0", "SLICEWE2", "SLICE1", "BYOUT_LOCAL"),
                    ("SLICE1", "SLICEWE2", "SLICE1", "BYOUT_LOCAL"),
                    ("SLICE2", "SLICEWE2", "SLICE1", "BYINVOUT_LOCAL"),
                    ("SLICE3", "SLICEWE2", "SLICE1", "BYINVOUT_LOCAL"),
                    // SLICE0 SOPIN <- left's SLICE2 SOPOUT
                    // SLICE1 SOPIN <- left's SLICE3 SOPOUT
                    ("SLICE2", "SOPIN", "SLICE0", "SOPOUT"),
                    ("SLICE3", "SOPIN", "SLICE1", "SOPOUT"),
                ] {
                    if dbel != bel.key {
                        continue;
                    }
                    let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), sbel).unwrap();
                    vrf.claim_pip(bel.crd(), bel.wire(dpin), obel.wire(spin));
                    vrf.claim_node(&[bel.fwire(dpin)]);
                }
                if bel.key == "SLICE3" {
                    // supposed to be connected? idk.
                    vrf.claim_node(&[bel.fwire("SHIFTIN")]);

                    if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, bel.row + 1), "SLICE3") {
                        vrf.verify_node(&[bel.fwire("DIG_S"), obel.fwire("DIG_LOCAL")]);
                    }

                    if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, bel.row + 1), "SLICE1") {
                        vrf.claim_node(&[bel.fwire("FXINB"), obel.fwire("FX_S")]);
                        vrf.claim_pip(obel.crd(), obel.wire("FX_S"), obel.wire("FX"));
                    } else {
                        vrf.claim_node(&[bel.fwire("FXINB")]);
                    }
                }
                for (dbel, sbel) in [("SLICE0", "SLICE1"), ("SLICE2", "SLICE3")] {
                    if bel.key != dbel {
                        continue;
                    }
                    if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, bel.row - 1), sbel) {
                        vrf.claim_node(&[bel.fwire("CIN"), obel.fwire("COUT_N")]);
                        vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
                    } else {
                        vrf.claim_node(&[bel.fwire("CIN")]);
                    }
                }
                for (dbel, sbel) in [("SLICE0", "SLICE2"), ("SLICE1", "SLICE3")] {
                    if bel.key != dbel {
                        continue;
                    }
                    let mut scol = bel.col - 1;
                    if grid.columns[scol].kind == ColumnKind::Bram {
                        scol -= 1;
                    }
                    if let Some(obel) = vrf.find_bel(bel.slr, (scol, bel.row), sbel) {
                        vrf.claim_node(&[bel.fwire("SOPIN"), obel.fwire("SOPOUT_W")]);
                        vrf.claim_pip(obel.crd(), obel.wire("SOPOUT_W"), obel.wire("SOPOUT"));
                    } else {
                        vrf.claim_node(&[bel.fwire("SOPIN")]);
                    }
                }
            } else {
                let kind = if matches!(bel.key, "SLICE0" | "SLICE2") {
                    "SLICEM"
                } else {
                    "SLICEL"
                };
                let mut pins = vec![
                    ("FXINA", SitePinDir::In),
                    ("FXINB", SitePinDir::In),
                    ("F5", SitePinDir::Out),
                    ("FX", SitePinDir::Out),
                    ("CIN", SitePinDir::In),
                    ("COUT", SitePinDir::Out),
                ];
                if kind == "SLICEM" {
                    pins.extend([
                        ("SHIFTIN", SitePinDir::In),
                        ("SHIFTOUT", SitePinDir::Out),
                        ("ALTDIG", SitePinDir::In),
                        ("DIG", SitePinDir::Out),
                        ("SLICEWE1", SitePinDir::In),
                        ("BYOUT", SitePinDir::Out),
                        ("BYINVOUT", SitePinDir::Out),
                    ]);
                }
                vrf.verify_bel(bel, kind, &pins, &[]);
                for (dbel, dpin, sbel, spin) in [
                    ("SLICE0", "FXINA", "SLICE0", "F5"),
                    ("SLICE0", "FXINB", "SLICE2", "F5"),
                    ("SLICE1", "FXINA", "SLICE1", "F5"),
                    ("SLICE1", "FXINB", "SLICE3", "F5"),
                    ("SLICE2", "FXINA", "SLICE0", "FX"),
                    ("SLICE2", "FXINB", "SLICE1", "FX"),
                    ("SLICE3", "FXINA", "SLICE2", "FX"),
                    // SLICE3 FXINB <- top's SLICE2 FX

                    // SLICE0 CIN <- bot's SLICE2 COUT
                    // SLICE1 CIN <- bot's SLICE3 COUT
                    ("SLICE2", "CIN", "SLICE0", "COUT"),
                    ("SLICE3", "CIN", "SLICE1", "COUT"),
                    ("SLICE0", "SHIFTIN", "SLICE2", "SHIFTOUT"),
                    // SLICE2 SHIFTIN disconnected?
                    ("SLICE0", "ALTDIG", "SLICE2", "DIG"),
                    // SLICE2 ALTDIG disconnected?
                    ("SLICE0", "SLICEWE1", "SLICE0", "BYOUT"),
                    ("SLICE2", "SLICEWE1", "SLICE0", "BYINVOUT"),
                ] {
                    if dbel != bel.key {
                        continue;
                    }
                    let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), sbel).unwrap();
                    vrf.claim_pip(bel.crd(), bel.wire(dpin), obel.wire(spin));
                    vrf.claim_node(&[bel.fwire(dpin)]);
                }
                if bel.key == "SLICE2" {
                    vrf.claim_node(&[bel.fwire("SHIFTIN")]);
                    vrf.claim_node(&[bel.fwire("ALTDIG")]);
                }
                if bel.key == "SLICE3" {
                    if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, bel.row + 1), "SLICE2") {
                        vrf.claim_node(&[bel.fwire("FXINB"), obel.fwire("FX_S")]);
                        vrf.claim_pip(obel.crd(), obel.wire("FX_S"), obel.wire("FX"));
                    } else {
                        vrf.claim_node(&[bel.fwire("FXINB")]);
                    }
                }
                for (dbel, sbel) in [("SLICE0", "SLICE2"), ("SLICE1", "SLICE3")] {
                    if bel.key != dbel {
                        continue;
                    }
                    if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, bel.row - 1), sbel) {
                        vrf.claim_node(&[bel.fwire("CIN"), obel.fwire("COUT_N")]);
                        vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
                    } else {
                        vrf.claim_node(&[bel.fwire("CIN")]);
                    }
                }
            }
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
            let mut scol = bel.col - 1;
            loop {
                if scol.to_idx() == 0 {
                    for pin in ["BUS0", "BUS1", "BUS2", "BUS3"] {
                        vrf.claim_node(&[bel.fwire(pin)]);
                    }
                    break;
                }
                if let Some(obel) = vrf.find_bel(bel.slr, (scol, bel.row), "TBUS") {
                    vrf.claim_node(&[bel.fwire("BUS0"), obel.fwire("BUS3_E")]);
                    vrf.verify_node(&[bel.fwire("BUS1"), obel.fwire("BUS0")]);
                    vrf.verify_node(&[bel.fwire("BUS2"), obel.fwire("BUS1")]);
                    vrf.verify_node(&[bel.fwire("BUS3"), obel.fwire("BUS2")]);
                    break;
                }
                scol -= 1;
            }
            vrf.claim_pip(bel.crd(), bel.wire("BUS3"), bel.wire("BUS3_E"));
            vrf.claim_pip(bel.crd(), bel.wire("BUS3_E"), bel.wire("BUS3"));
            vrf.claim_pip(bel.crd(), bel.wire("OUT"), bel.wire("BUS2"));
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
        "STARTUP" | "CAPTURE" | "ICAP" | "SPI_ACCESS" | "BSCAN" | "JTAGPPC" | "PMV"
        | "DNA_PORT" | "PCILOGIC" => {
            vrf.verify_bel(bel, bel.key, &[], &[]);
        }
        "BRAM" => {
            let kind = match grid.kind {
                GridKind::Spartan3A => "RAMB16BWE",
                GridKind::Spartan3ADsp => "RAMB16BWER",
                _ => "RAMB16",
            };
            vrf.verify_bel(bel, kind, &[], &[]);
        }
        "MULT" => {
            if matches!(grid.kind, GridKind::Spartan3E | GridKind::Spartan3A) {
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
                let mut srow = bel.row;
                loop {
                    if srow.to_idx() < 4 {
                        break;
                    }
                    srow -= 4;
                    if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, srow), "MULT") {
                        for (o, i) in &carry {
                            vrf.verify_node(&[bel.fwire(i), obel.fwire_far(o)]);
                            vrf.claim_pip(obel.crd(), obel.wire_far(o), obel.wire(o));
                        }
                        break;
                    }
                }
            } else {
                vrf.verify_bel(bel, "MULT18X18", &[], &[]);
            }
        }
        "DSP" => {
            let carry: Vec<_> = (0..18)
                .map(|x| (format!("BCOUT{x}"), format!("BCIN{x}")))
                .chain((0..48).map(|x| (format!("PCOUT{x}"), format!("PCIN{x}"))))
                .chain([("CARRYOUT".to_string(), "CARRYIN".to_string())].into_iter())
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
            let mut srow = bel.row;
            loop {
                if srow.to_idx() < 4 {
                    break;
                }
                srow -= 4;
                if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, srow), "DSP") {
                    for (o, i) in &carry {
                        vrf.verify_node(&[bel.fwire(i), obel.fwire_far(o)]);
                        vrf.claim_pip(obel.crd(), obel.wire_far(o), obel.wire(o));
                    }
                    break;
                }
            }
        }
        "RANDOR" => {
            vrf.verify_bel(
                bel,
                "RESERVED_ANDOR",
                &[
                    ("CIN0", SitePinDir::In),
                    ("CIN1", SitePinDir::In),
                    ("CPREV", SitePinDir::In),
                    ("O", SitePinDir::Out),
                ],
                &[],
            );
            if bel.row == grid.row_bot() {
                for pin in ["CIN0", "CIN1", "CPREV", "O"] {
                    vrf.claim_node(&[bel.fwire(pin)]);
                }
            } else {
                for pin in ["CPREV", "O"] {
                    vrf.claim_node(&[bel.fwire(pin)]);
                }
                for (pin, sbel) in [("CIN1", "SLICE2"), ("CIN0", "SLICE3")] {
                    if let Some(obel) = vrf.find_bel(bel.slr, (bel.col, bel.row - 1), sbel) {
                        vrf.claim_node(&[bel.fwire(pin), obel.fwire("COUT_N")]);
                        vrf.claim_pip(obel.crd(), obel.wire("COUT_N"), obel.wire("COUT"));
                    } else {
                        vrf.claim_node(&[bel.fwire(pin)]);
                    }
                }
                vrf.claim_pip(bel.crd(), bel.wire_far("O"), bel.wire("O"));
                let mut ncol = bel.col + 1;
                loop {
                    if let Some(obel) = vrf.find_bel(bel.slr, (ncol, bel.row), "RANDOR") {
                        vrf.claim_node(&[bel.fwire_far("O"), obel.fwire_far("CPREV")]);
                        vrf.claim_pip(obel.crd(), obel.wire("CPREV"), obel.wire_far("CPREV"));
                        break;
                    } else if let Some(obel) = vrf.find_bel(bel.slr, (ncol, bel.row), "RANDOR_OUT")
                    {
                        vrf.verify_node(&[bel.fwire_far("O"), obel.fwire("O")]);
                        break;
                    } else {
                        ncol += 1;
                    }
                }
            }
        }
        "RANDOR_OUT" => (),
        _ if bel.key.starts_with("GT") => {
            if grid.kind == GridKind::Virtex2PX {
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
                        .find_bel(bel.slr, (grid.col_clk - 1, bel.row), oname)
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
                    .find_bel(bel.slr, (grid.col_clk - 1, bel.row), "BREFCLK")
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
                let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), okey).unwrap();
                vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("I"));
            }
            for (pin, okey) in [("TXP", "OPAD.TXP"), ("TXN", "OPAD.TXN")] {
                vrf.claim_node(&[bel.fwire(pin)]);
                let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), okey).unwrap();
                vrf.claim_pip(bel.crd(), obel.wire("O"), bel.wire(pin));
            }
        }
        _ if bel.key.starts_with("IPAD") => {
            vrf.verify_bel(bel, "GTIPAD", &[("I", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("I")]);
        }
        _ if bel.key.starts_with("OPAD") => {
            vrf.verify_bel(bel, "GTOPAD", &[("O", SitePinDir::In)], &[]);
            vrf.claim_node(&[bel.fwire("O")]);
        }
        _ if bel.key.starts_with("IOI") => {
            let attr = grid.get_io_attr(vrf.grid, (bel.col, bel.row), bel.bid);
            let tn = &bel.node.names[NodeRawTileId::from_idx(0)];
            let is_ipad = tn.contains("IBUFS") || (tn.contains("IOIB") && bel.bid.to_idx() == 2);
            let kind = if matches!(grid.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp) {
                let is_tb = matches!(attr.bank, 0 | 2);
                match (attr.diff, is_ipad) {
                    (IoDiffKind::P(_), false) => {
                        if is_tb {
                            "DIFFMTB"
                        } else {
                            "DIFFMLR"
                        }
                    }
                    (IoDiffKind::P(_), true) => "DIFFMI_NDT",
                    (IoDiffKind::N(_), false) => {
                        if is_tb {
                            "DIFFSTB"
                        } else {
                            "DIFFSLR"
                        }
                    }
                    (IoDiffKind::N(_), true) => "DIFFSI_NDT",
                    (IoDiffKind::None, false) => "IOB",
                    (IoDiffKind::None, true) => "IBUF",
                }
            } else {
                match (attr.diff, is_ipad) {
                    (IoDiffKind::P(_), false) => "DIFFM",
                    (IoDiffKind::P(_), true) => "DIFFMI",
                    (IoDiffKind::N(_), false) => "DIFFS",
                    (IoDiffKind::N(_), true) => "DIFFSI",
                    (IoDiffKind::None, false) => "IOB",
                    (IoDiffKind::None, true) => "IBUF",
                }
            };
            let mut pins = vec![
                ("PADOUT", SitePinDir::Out),
                ("DIFFI_IN", SitePinDir::In),
                ("DIFFO_OUT", SitePinDir::Out),
                ("DIFFO_IN", SitePinDir::In),
            ];
            if matches!(
                grid.kind,
                GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
            ) {
                pins.extend([
                    ("PCI_RDY", SitePinDir::Out),
                    ("PCI_CE", SitePinDir::In),
                    ("ODDROUT1", SitePinDir::Out),
                    ("ODDROUT2", SitePinDir::Out),
                    ("ODDRIN1", SitePinDir::In),
                    ("ODDRIN2", SitePinDir::In),
                    ("IDDRIN1", SitePinDir::In),
                    ("IDDRIN2", SitePinDir::In),
                ]);
            }
            if grid.kind == GridKind::Spartan3ADsp {
                pins.extend([("OAUX", SitePinDir::In), ("TAUX", SitePinDir::In)]);
            }
            vrf.verify_bel(bel, kind, &pins, &[]);
            // diff pairing
            if !grid.kind.is_virtex2() || attr.diff != IoDiffKind::None {
                for pin in ["PADOUT", "DIFFI_IN", "DIFFO_IN", "DIFFO_OUT"] {
                    vrf.claim_node(&[bel.fwire(pin)]);
                }
                match attr.diff {
                    IoDiffKind::P(obid) => {
                        let obel = vrf.get_bel(bel.slr, bel.node, obid);
                        vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
                    }
                    IoDiffKind::N(obid) => {
                        let obel = vrf.get_bel(bel.slr, bel.node, obid);
                        vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
                        vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
                    }
                    IoDiffKind::None => (),
                }
            }
            if matches!(
                grid.kind,
                GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
            ) {
                for pin in [
                    "ODDRIN1", "ODDRIN2", "ODDROUT1", "ODDROUT2", "IDDRIN1", "IDDRIN2", "PCI_CE",
                    "PCI_RDY",
                ] {
                    vrf.claim_node(&[bel.fwire(pin)]);
                }
                // ODDR, IDDR
                if let IoDiffKind::P(obid) | IoDiffKind::N(obid) = attr.diff {
                    let obel = vrf.get_bel(bel.slr, bel.node, obid);
                    vrf.claim_pip(bel.crd(), bel.wire("ODDRIN1"), obel.wire("ODDROUT2"));
                    vrf.claim_pip(bel.crd(), bel.wire("ODDRIN2"), obel.wire("ODDROUT1"));
                    vrf.claim_pip(bel.crd(), bel.wire("IDDRIN1"), obel.wire("IQ1"));
                    vrf.claim_pip(bel.crd(), bel.wire("IDDRIN2"), obel.wire("IQ2"));
                }
                vrf.claim_pip(bel.crd(), bel.wire("PCI_CE"), bel.wire_far("PCI_CE"));
                verify_pci_ce(
                    grid,
                    vrf,
                    bel.slr,
                    bel.col,
                    bel.row,
                    bel.crd(),
                    bel.wire_far("PCI_CE"),
                );
            }
            if grid.kind == GridKind::Spartan3ADsp {
                for pin in ["OAUX", "TAUX"] {
                    vrf.claim_node(&[bel.fwire(pin)]);
                }
            }
        }
        _ if bel.key.starts_with("IOBS") => (),
        "BREFCLK" => {
            vrf.claim_node(&[bel.fwire("BREFCLK")]);
            vrf.claim_node(&[bel.fwire("BREFCLK2")]);
            if bel.row == grid.row_bot() {
                let obel = vrf
                    .find_bel(bel.slr, (bel.col, bel.row), "BUFGMUX6")
                    .unwrap();
                vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("CKI"));
                let obel = vrf
                    .find_bel(bel.slr, (bel.col, bel.row), "BUFGMUX0")
                    .unwrap();
                vrf.claim_pip(bel.crd(), bel.wire("BREFCLK2"), obel.wire_far("CKI"));
            } else {
                let obel = vrf
                    .find_bel(bel.slr, (bel.col, bel.row), "BUFGMUX4")
                    .unwrap();
                vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("CKI"));
                let obel = vrf
                    .find_bel(bel.slr, (bel.col, bel.row), "BUFGMUX2")
                    .unwrap();
                vrf.claim_pip(bel.crd(), bel.wire("BREFCLK2"), obel.wire_far("CKI"));
            }
        }
        "BREFCLK_INT" => {
            let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), "CLK_P").unwrap();
            vrf.claim_pip(bel.crd(), bel.wire("BREFCLK"), obel.wire_far("I"));
        }
        "CLK_P" | "CLK_N" => {
            vrf.verify_bel(bel, bel.key, &[("I", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("I")]);
            vrf.claim_node(&[bel.fwire_far("I")]);
            vrf.claim_pip(bel.crd(), bel.wire_far("I"), bel.wire("I"));
        }
        _ if bel.key.starts_with("BUFGMUX") => {
            vrf.verify_bel(
                bel,
                "BUFGMUX",
                &[("I0", SitePinDir::In), ("I1", SitePinDir::In)],
                &["CLK"],
            );
            vrf.claim_node(&[bel.fwire("I0")]);
            vrf.claim_node(&[bel.fwire("I1")]);
            vrf.claim_pip(bel.crd(), bel.wire("I0"), bel.wire("CLK"));
            let obid = BelId::from_idx(bel.bid.to_idx() ^ 1);
            let obel = vrf.get_bel(bel.slr, bel.node, obid);
            vrf.claim_pip(bel.crd(), bel.wire("I1"), obel.wire("CLK"));
            let edge = if bel.row == grid.row_bot() {
                Edge::Bot
            } else if bel.row == grid.row_top() {
                Edge::Top
            } else if bel.col == grid.col_left() {
                Edge::Left
            } else if bel.col == grid.col_right() {
                Edge::Right
            } else {
                unreachable!()
            };
            if grid.kind.is_virtex2() || grid.kind == GridKind::Spartan3 {
                if let Some((crd, obid)) = grid.get_clk_io(edge, bel.bid.to_idx()) {
                    let onode = grid.get_io_node(vrf.grid, crd).unwrap();
                    let obel = vrf.get_bel(bel.slr, onode, obid);
                    vrf.claim_node(&[bel.fwire("CKI"), obel.fwire("IBUF")]);
                    vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
                } else {
                    vrf.claim_node(&[bel.fwire("CKI")]);
                }
                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKI"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_L"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_R"));
                vrf.claim_node(&[bel.fwire("DCM_OUT_L")]);
                vrf.claim_node(&[bel.fwire("DCM_OUT_R")]);
                if grid.kind.is_virtex2() {
                    for pin in ["DCM_PAD_L", "DCM_PAD_R"] {
                        vrf.claim_node(&[bel.fwire(pin)]);
                        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("CKI"));
                    }
                } else {
                    vrf.claim_node(&[bel.fwire("DCM_PAD")]);
                    vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD"), bel.wire("CKI"));
                }
            } else if matches!(edge, Edge::Bot | Edge::Top) {
                let (crd, obid) = grid.get_clk_io(edge, bel.bid.to_idx()).unwrap();
                let onode = grid.get_io_node(vrf.grid, crd).unwrap();
                let obel = vrf.get_bel(bel.slr, onode, obid);
                vrf.claim_node(&[bel.fwire("CKIR"), obel.fwire("IBUF")]);
                vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
                let (crd, obid) = grid.get_clk_io(edge, bel.bid.to_idx() + 4).unwrap();
                let onode = grid.get_io_node(vrf.grid, crd).unwrap();
                let obel = vrf.get_bel(bel.slr, onode, obid);
                vrf.claim_node(&[bel.fwire("CKIL"), obel.fwire("IBUF")]);
                vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKIL"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKIR"));

                let mut has_dcm_l = true;
                let mut has_dcm_r = true;
                if grid.kind == GridKind::Spartan3E {
                    if grid.dcms == Some(Dcms::Two) {
                        has_dcm_l = false;
                    }
                } else {
                    if grid.dcms == Some(Dcms::Two) && bel.row == grid.row_bot() {
                        has_dcm_l = false;
                        has_dcm_r = false;
                    }
                }
                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_L"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT_R"));
                if has_dcm_l {
                    vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD_L"), bel.wire("CKIL"));
                    let pip = &bel.naming.pins["DCM_OUT_L"].pips[0];
                    vrf.claim_node(&[bel.fwire("DCM_OUT_L"), (bel.crds[pip.tile], &pip.wire_to)]);
                    vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
                    let srow = match edge {
                        Edge::Bot => bel.row + 1,
                        Edge::Top => bel.row - 1,
                        _ => unreachable!(),
                    };
                    let obel = vrf
                        .find_bel(bel.slr, (bel.col, srow), "DCMCONN.S3E")
                        .unwrap();
                    let (dcm_pad_pin, dcm_out_pin) = match (edge, bel.bid.to_idx()) {
                        (Edge::Top, 0) => ("CLKPAD0", "OUT0"),
                        (Edge::Top, 1) => ("CLKPAD1", "OUT1"),
                        (Edge::Top, 2) => ("CLKPAD2", "OUT2"),
                        (Edge::Top, 3) => ("CLKPAD3", "OUT3"),
                        (Edge::Bot, 0) => ("CLKPAD3", "OUT0"),
                        (Edge::Bot, 1) => ("CLKPAD2", "OUT1"),
                        (Edge::Bot, 2) => ("CLKPAD1", "OUT2"),
                        (Edge::Bot, 3) => ("CLKPAD0", "OUT3"),
                        _ => unreachable!(),
                    };
                    vrf.verify_node(&[bel.fwire("DCM_PAD_L"), obel.fwire(dcm_pad_pin)]);
                    vrf.verify_node(&[
                        (bel.crds[pip.tile], &pip.wire_from),
                        obel.fwire(dcm_out_pin),
                    ]);
                } else {
                    vrf.claim_node(&[bel.fwire("DCM_OUT_L")]);
                }
                if has_dcm_r {
                    vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD_R"), bel.wire("CKIR"));
                    let pip = &bel.naming.pins["DCM_OUT_R"].pips[0];
                    vrf.claim_node(&[bel.fwire("DCM_OUT_R"), (bel.crds[pip.tile], &pip.wire_to)]);
                    vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
                    let srow = match edge {
                        Edge::Bot => bel.row + 1,
                        Edge::Top => bel.row - 1,
                        _ => unreachable!(),
                    };
                    let obel = vrf
                        .find_bel(bel.slr, (bel.col + 1, srow), "DCMCONN.S3E")
                        .unwrap();
                    let (dcm_pad_pin, dcm_out_pin) = match (edge, bel.bid.to_idx()) {
                        (Edge::Top, 0) => ("CLKPAD2", "OUT0"),
                        (Edge::Top, 1) => ("CLKPAD3", "OUT1"),
                        (Edge::Top, 2) => ("CLKPAD0", "OUT2"),
                        (Edge::Top, 3) => ("CLKPAD1", "OUT3"),
                        (Edge::Bot, 0) => ("CLKPAD0", "OUT0"),
                        (Edge::Bot, 1) => ("CLKPAD1", "OUT1"),
                        (Edge::Bot, 2) => ("CLKPAD2", "OUT2"),
                        (Edge::Bot, 3) => ("CLKPAD3", "OUT3"),
                        _ => unreachable!(),
                    };
                    vrf.verify_node(&[bel.fwire("DCM_PAD_R"), obel.fwire(dcm_pad_pin)]);
                    vrf.verify_node(&[
                        (bel.crds[pip.tile], &pip.wire_from),
                        obel.fwire(dcm_out_pin),
                    ]);
                } else {
                    vrf.claim_node(&[bel.fwire("DCM_OUT_R")]);
                }
            } else {
                let (crd, obid) = grid.get_clk_io(edge, bel.bid.to_idx()).unwrap();
                let onode = grid.get_io_node(vrf.grid, crd).unwrap();
                let obel = vrf.get_bel(bel.slr, onode, obid);
                vrf.verify_node(&[bel.fwire("CKI"), obel.fwire("IBUF")]);
                vrf.claim_pip(obel.crd(), obel.wire("IBUF"), obel.wire("I"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("CKI"));

                vrf.claim_pip(bel.crd(), bel.wire("CLK"), bel.wire("DCM_OUT"));
                if grid.dcms == Some(Dcms::Eight) {
                    let pad_pin;
                    if grid.kind != GridKind::Spartan3A {
                        pad_pin = "CKI";
                    } else {
                        pad_pin = "DCM_PAD";
                        vrf.claim_node(&[bel.fwire("CKI")]);
                        vrf.claim_pip(bel.crd(), bel.wire("DCM_PAD"), bel.wire("CKI"));
                    }
                    let scol = if grid.kind == GridKind::Spartan3E {
                        match edge {
                            Edge::Left => grid.col_left() + 9,
                            Edge::Right => grid.col_right() - 9,
                            _ => unreachable!(),
                        }
                    } else {
                        match edge {
                            Edge::Left => grid.col_left() + 3,
                            Edge::Right => grid.col_right() - 6,
                            _ => unreachable!(),
                        }
                    };
                    let srow = if bel.bid.to_idx() < 4 {
                        grid.row_mid()
                    } else {
                        grid.row_mid() - 1
                    };
                    let obel = vrf.find_bel(bel.slr, (scol, srow), "DCMCONN.S3E").unwrap();
                    let (dcm_pad_pin, dcm_out_pin) = match bel.bid.to_idx() {
                        0 | 4 => ("CLKPAD0", "OUT0"),
                        1 | 5 => ("CLKPAD1", "OUT1"),
                        2 | 6 => ("CLKPAD2", "OUT2"),
                        3 | 7 => ("CLKPAD3", "OUT3"),
                        _ => unreachable!(),
                    };
                    vrf.verify_node(&[bel.fwire(pad_pin), obel.fwire(dcm_pad_pin)]);
                    vrf.verify_node(&[bel.fwire("DCM_OUT"), obel.fwire(dcm_out_pin)]);
                } else {
                    vrf.claim_node(&[bel.fwire("CKI")]);
                }
                let obel = vrf.find_bel(bel.slr, (bel.col, bel.row), "VCC").unwrap();
                vrf.claim_pip(bel.crd(), bel.wire_far("CLK"), obel.wire("VCCOUT"));
                vrf.claim_pip(bel.crd(), bel.wire("S"), obel.wire("VCCOUT"));
            }
        }
        "PCILOGICSE" => {
            vrf.verify_bel(
                bel,
                "PCILOGICSE",
                &[
                    ("IRDY", SitePinDir::In),
                    ("TRDY", SitePinDir::In),
                    ("PCI_CE", SitePinDir::Out),
                ],
                &[],
            );
            let edge = if bel.col == grid.col_left() {
                Edge::Left
            } else if bel.col == grid.col_right() {
                Edge::Right
            } else {
                unreachable!()
            };
            let pci_rdy = grid.get_pci_io(edge);
            for (pin, (crd, obid)) in ["IRDY", "TRDY"].into_iter().zip(pci_rdy.into_iter()) {
                vrf.claim_node(&[bel.fwire(pin)]);
                vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
                let onode = grid.get_io_node(vrf.grid, crd).unwrap();
                let obel = vrf.get_bel(bel.slr, onode, obid);
                vrf.claim_node(&[bel.fwire_far(pin), obel.fwire("PCI_RDY_IN")]);
                vrf.claim_pip(obel.crd(), obel.wire("PCI_RDY_IN"), obel.wire("PCI_RDY"));
            }
            let pip = &bel.naming.pins["PCI_CE"].pips[0];
            vrf.claim_node(&[bel.fwire("PCI_CE"), (bel.crds[pip.tile], &pip.wire_from)]);
            vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
            vrf.claim_node(&[(bel.crds[pip.tile], &pip.wire_to)]);
        }
        "VCC" => {
            vrf.verify_bel(bel, "VCC", &[("VCCOUT", SitePinDir::Out)], &[]);
            vrf.claim_node(&[bel.fwire("VCCOUT")]);
        }
        "DCM" => {
            vrf.verify_bel(bel, "DCM", &[], &[]);
        }
        "DCMCONN.S3E" => (),
        "DCMCONN" => {
            let opin_pad;
            let pins_out;
            let pins_pad;
            if grid.kind.is_virtex2() {
                pins_out = &[
                    ("OUTBUS0", "OUT0", "BUFGMUX0"),
                    ("OUTBUS1", "OUT1", "BUFGMUX1"),
                    ("OUTBUS2", "OUT2", "BUFGMUX2"),
                    ("OUTBUS3", "OUT3", "BUFGMUX3"),
                    ("OUTBUS4", "OUT0", "BUFGMUX4"),
                    ("OUTBUS5", "OUT1", "BUFGMUX5"),
                    ("OUTBUS6", "OUT2", "BUFGMUX6"),
                    ("OUTBUS7", "OUT3", "BUFGMUX7"),
                ][..];
                if bel.col < grid.col_clk {
                    opin_pad = "DCM_PAD_L";
                    pins_pad = &[
                        ("CLKPAD0", "CLKPADBUS0", "BUFGMUX4"),
                        ("CLKPAD1", "CLKPADBUS1", "BUFGMUX5"),
                        ("CLKPAD2", "CLKPADBUS2", "BUFGMUX6"),
                        ("CLKPAD3", "CLKPADBUS3", "BUFGMUX7"),
                        ("CLKPAD4", "CLKPADBUS4", "BUFGMUX0"),
                        ("CLKPAD5", "CLKPADBUS5", "BUFGMUX1"),
                        ("CLKPAD6", "CLKPADBUS6", "BUFGMUX2"),
                        ("CLKPAD7", "CLKPADBUS7", "BUFGMUX3"),
                    ][..];
                } else {
                    opin_pad = "DCM_PAD_R";
                    pins_pad = &[
                        ("CLKPAD0", "CLKPADBUS0", "BUFGMUX0"),
                        ("CLKPAD1", "CLKPADBUS1", "BUFGMUX1"),
                        ("CLKPAD2", "CLKPADBUS2", "BUFGMUX2"),
                        ("CLKPAD3", "CLKPADBUS3", "BUFGMUX3"),
                        ("CLKPAD4", "CLKPADBUS4", "BUFGMUX4"),
                        ("CLKPAD5", "CLKPADBUS5", "BUFGMUX5"),
                        ("CLKPAD6", "CLKPADBUS6", "BUFGMUX6"),
                        ("CLKPAD7", "CLKPADBUS7", "BUFGMUX7"),
                    ][..];
                }
            } else {
                pins_out = &[
                    ("OUTBUS0", "OUT0", "BUFGMUX0"),
                    ("OUTBUS1", "OUT1", "BUFGMUX1"),
                    ("OUTBUS2", "OUT2", "BUFGMUX2"),
                    ("OUTBUS3", "OUT3", "BUFGMUX3"),
                ][..];
                opin_pad = "DCM_PAD";
                pins_pad = &[
                    ("CLKPAD0", "CLKPADBUS0", "BUFGMUX0"),
                    ("CLKPAD1", "CLKPADBUS1", "BUFGMUX1"),
                    ("CLKPAD2", "CLKPADBUS2", "BUFGMUX2"),
                    ("CLKPAD3", "CLKPADBUS3", "BUFGMUX3"),
                ][..];
            }
            let opin_out = if bel.col < grid.col_clk {
                "DCM_OUT_L"
            } else {
                "DCM_OUT_R"
            };
            for &(pin_o, pin_i, obk) in pins_out {
                vrf.claim_pip(bel.crd(), bel.wire(pin_o), bel.wire(pin_i));
                let obel = vrf
                    .find_bel(bel.slr, (grid.col_clk - 1, bel.row), obk)
                    .unwrap();
                vrf.verify_node(&[bel.fwire(pin_o), obel.fwire(opin_out)]);
            }
            for &(pin_o, pin_i, obk) in pins_pad {
                vrf.claim_pip(bel.crd(), bel.wire(pin_o), bel.wire(pin_i));
                let obel = vrf
                    .find_bel(bel.slr, (grid.col_clk - 1, bel.row), obk)
                    .unwrap();
                vrf.verify_node(&[bel.fwire(pin_i), obel.fwire(opin_pad)]);
            }
        }
        "PPC405" => {
            let mut skip_pins = vec![];
            for i in 15..29 {
                skip_pins.push(format!("ISOCMBRAMWRABUS{i}.BL"));
                skip_pins.push(format!("ISOCMBRAMWRABUS{i}.BR"));
                skip_pins.push(format!("ISOCMBRAMRDABUS{i}.BL"));
                skip_pins.push(format!("ISOCMBRAMRDABUS{i}.BR"));
            }
            for i in 16..30 {
                skip_pins.push(format!("DSOCMBRAMABUS{i}.TL"));
                skip_pins.push(format!("DSOCMBRAMABUS{i}.TR"));
            }
            let skip_pins_ref: Vec<_> = skip_pins.iter().map(|x| &**x).collect();
            vrf.verify_bel(bel, bel.key, &[], &skip_pins_ref);
            for pin in skip_pins {
                let spin = &pin[..pin.find('.').unwrap()];
                vrf.claim_pip(bel.crd(), bel.wire(&pin), bel.wire(spin));
            }
        }
        "PCI_CE_N" => {
            vrf.claim_node(&[bel.fwire("O")]);
            vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
            verify_pci_ce(
                grid,
                vrf,
                bel.slr,
                bel.col,
                bel.row - 1,
                bel.crd(),
                bel.wire("I"),
            );
        }
        "PCI_CE_S" => {
            vrf.claim_node(&[bel.fwire("O")]);
            vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
            verify_pci_ce(
                grid,
                vrf,
                bel.slr,
                bel.col,
                bel.row,
                bel.crd(),
                bel.wire("I"),
            );
        }
        "PCI_CE_E" => {
            vrf.claim_node(&[bel.fwire("O")]);
            vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
            verify_pci_ce(
                grid,
                vrf,
                bel.slr,
                bel.col - 1,
                bel.row,
                bel.crd(),
                bel.wire("I"),
            );
        }
        "PCI_CE_W" => {
            vrf.claim_node(&[bel.fwire("O")]);
            vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
            verify_pci_ce(
                grid,
                vrf,
                bel.slr,
                bel.col,
                bel.row,
                bel.crd(),
                bel.wire("I"),
            );
        }
        "PCI_CE_CNR" => {
            vrf.claim_node(&[bel.fwire("O")]);
            vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
            verify_pci_ce(
                grid,
                vrf,
                bel.slr,
                bel.col,
                bel.row,
                bel.crd(),
                bel.wire("I"),
            );
        }
        _ if bel.key.starts_with("GCLKH") => {
            for i in 0..8 {
                for ud in ["UP", "DN"] {
                    if matches!((bel.key, ud), ("GCLKH.S", "UP") | ("GCLKH.N", "DN")) {
                        continue;
                    }
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("OUT_{ud}{i}")),
                        bel.wire(&format!("IN{i}")),
                    );
                }
                if grid.kind.is_virtex2() {
                    let lr = if bel.col < grid.col_clk { 'L' } else { 'R' };
                    let obel = vrf
                        .find_bel(bel.slr, (grid.col_clk, bel.row + 1), "GCLKC")
                        .unwrap();
                    vrf.verify_node(&[
                        bel.fwire(&format!("IN{i}")),
                        obel.fwire(&format!("OUT_{lr}{i}")),
                    ]);
                } else if let Some((col_cl, col_cr)) = grid.cols_clkv {
                    let scol = if bel.col < grid.col_clk {
                        col_cl
                    } else {
                        col_cr
                    };
                    let lr = if bel.col < scol { 'L' } else { 'R' };
                    let obel = vrf
                        .find_bel(bel.slr, (scol, bel.row + 1), "GCLKVC")
                        .unwrap();
                    vrf.verify_node(&[
                        bel.fwire(&format!("IN{i}")),
                        obel.fwire(&format!("OUT_{lr}{i}")),
                    ]);
                } else {
                    let lr = if bel.col < grid.col_clk { 'L' } else { 'R' };
                    let obel = vrf
                        .find_bel(bel.slr, (grid.col_clk, grid.row_mid()), "CLKC_50A")
                        .unwrap();
                    vrf.verify_node(&[
                        bel.fwire(&format!("IN{i}")),
                        obel.fwire(&format!("OUT_{lr}{i}")),
                    ]);
                }
            }
        }
        "GCLKC" => {
            for i in 0..8 {
                for lr in ['L', 'R'] {
                    vrf.claim_node(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
                    for bt in ['B', 'T'] {
                        vrf.claim_pip(
                            bel.crd(),
                            bel.wire(&format!("OUT_{lr}{i}")),
                            bel.wire(&format!("IN_{bt}{i}")),
                        );
                    }
                }
                for bt in ['B', 'T'] {
                    let obel = vrf
                        .find_bel(bel.slr, (grid.col_clk, grid.row_mid()), "CLKC")
                        .unwrap();
                    vrf.verify_node(&[
                        bel.fwire(&format!("IN_{bt}{i}")),
                        obel.fwire(&format!("OUT_{bt}{i}")),
                    ]);
                }
            }
        }
        "CLKC" => {
            if grid.kind.is_virtex2() {
                for i in 0..8 {
                    for bt in ['B', 'T'] {
                        vrf.claim_node(&[(bel.crd(), bel.wire(&format!("OUT_{bt}{i}")))]);
                        vrf.claim_pip(
                            bel.crd(),
                            bel.wire(&format!("OUT_{bt}{i}")),
                            bel.wire(&format!("IN_{bt}{i}")),
                        );
                        let srow = if bt == 'B' {
                            grid.row_bot()
                        } else {
                            grid.row_top()
                        };
                        let obel = vrf
                            .find_bel(bel.slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{i}"))
                            .unwrap();
                        vrf.verify_node(&[bel.fwire(&format!("IN_{bt}{i}")), obel.fwire_far("O")]);
                    }
                }
            } else {
                for i in 0..8 {
                    let (bt, j) = if i < 4 { ('B', i) } else { ('T', i - 4) };
                    vrf.claim_node(&[bel.fwire(&format!("OUT{i}"))]);
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("OUT{i}")),
                        bel.wire(&format!("IN_{bt}{j}")),
                    );
                    let srow = if bt == 'B' {
                        grid.row_bot()
                    } else {
                        grid.row_top()
                    };
                    let obel = vrf
                        .find_bel(bel.slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{j}"))
                        .unwrap();
                    vrf.verify_node(&[bel.fwire(&format!("IN_{bt}{j}")), obel.fwire_far("O")]);
                }
            }
        }
        "CLKC_50A" => {
            for i in 0..8 {
                let (bt, j) = if i < 4 { ('B', i) } else { ('T', i - 4) };
                for lr in ['L', 'R'] {
                    vrf.claim_node(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
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
                        grid.col_left()
                    } else {
                        grid.col_right()
                    };
                    let obel = vrf
                        .find_bel(bel.slr, (scol, grid.row_mid() - 1), &format!("BUFGMUX{i}"))
                        .unwrap();
                    vrf.verify_node(&[bel.fwire(&format!("IN_{lr}{i}")), obel.fwire_far("O")]);
                }
                let srow = if bt == 'B' {
                    grid.row_bot()
                } else {
                    grid.row_top()
                };
                let obel = vrf
                    .find_bel(bel.slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{j}"))
                    .unwrap();
                vrf.verify_node(&[bel.fwire(&format!("IN_{bt}{j}")), obel.fwire_far("O")]);
            }
        }
        "GCLKVM" => {
            for i in 0..8 {
                for ud in ["UP", "DN"] {
                    vrf.claim_node(&[bel.fwire(&format!("OUT_{ud}{i}"))]);
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("OUT_{ud}{i}")),
                        bel.wire(&format!("IN_CORE{i}")),
                    );
                    if grid.kind != GridKind::Spartan3 {
                        vrf.claim_pip(
                            bel.crd(),
                            bel.wire(&format!("OUT_{ud}{i}")),
                            bel.wire(&format!("IN_LR{i}")),
                        );
                    }
                }
                let obel = vrf
                    .find_bel(bel.slr, (grid.col_clk, bel.row), "CLKC")
                    .unwrap();
                vrf.verify_node(&[
                    bel.fwire(&format!("IN_CORE{i}")),
                    obel.fwire(&format!("OUT{i}")),
                ]);
                if grid.kind != GridKind::Spartan3 {
                    let scol = if bel.col < grid.col_clk {
                        grid.col_left()
                    } else {
                        grid.col_right()
                    };
                    let obel = vrf
                        .find_bel(bel.slr, (scol, grid.row_mid() - 1), &format!("BUFGMUX{i}"))
                        .unwrap();
                    vrf.verify_node(&[bel.fwire(&format!("IN_LR{i}")), obel.fwire_far("O")]);
                }
            }
        }
        "GCLKVC" => {
            for i in 0..8 {
                for lr in ['L', 'R'] {
                    vrf.claim_node(&[(bel.crd(), bel.wire(&format!("OUT_{lr}{i}")))]);
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(&format!("OUT_{lr}{i}")),
                        bel.wire(&format!("IN{i}")),
                    );
                }
                let ud = if bel.row < grid.row_mid() { "DN" } else { "UP" };
                let obel = vrf
                    .find_bel(bel.slr, (bel.col, grid.row_mid()), "GCLKVM")
                    .unwrap();
                vrf.verify_node(&[
                    bel.fwire(&format!("IN{i}")),
                    obel.fwire(&format!("OUT_{ud}{i}")),
                ]);
            }
        }
        _ if bel.key.starts_with("GLOBALSIG") => {
            vrf.verify_bel(bel, "GLOBALSIG", &[], &[]);
        }
        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}
