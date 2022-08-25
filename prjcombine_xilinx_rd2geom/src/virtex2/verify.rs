use crate::verify::{SitePinDir, Verifier};
use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::eint::ExpandedTileNode;
use prjcombine_xilinx_geom::int::{NodeRawTileId, NodeTileId};
use prjcombine_xilinx_geom::virtex2::{ColumnKind, Dcms, Edge, Grid, GridKind, IoDiffKind};
use prjcombine_xilinx_geom::{BelId, ColId, RowId, SlrId};
use prjcombine_xilinx_rawdump::Coord;

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
                    let (onode, _, _, onaming) =
                        vrf.grid.find_bel(slr, (col, srow), "PCI_CE_S").unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[(ocrds[onaming.tile], &onaming.pins["O"].name), (crd, wire)]);
                    return;
                }
            }
        } else {
            for &(srow, _, _) in grid.rows_hclk.iter().rev() {
                if srow <= grid.row_mid() {
                    break;
                }
                if row >= srow {
                    let (onode, _, _, onaming) =
                        vrf.grid.find_bel(slr, (col, srow), "PCI_CE_N").unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[(ocrds[onaming.tile], &onaming.pins["O"].name), (crd, wire)]);
                    return;
                }
            }
        }
        let (onode, _, _, onaming) = vrf
            .grid
            .find_bel(slr, (col, grid.row_mid() - 1), "PCILOGICSE")
            .unwrap();
        let ocrds = vrf.get_node_crds(onode).unwrap();
        let pip = &onaming.pins["PCI_CE"].pips[0];
        vrf.verify_node(&[(ocrds[pip.tile], &pip.wire_to), (crd, wire)]);
    } else {
        if grid.kind == GridKind::Spartan3A {
            if let Some((col_l, col_r)) = grid.cols_clkv {
                if col >= col_l && col < col_r {
                    let (scol, kind) = if col < grid.col_clk {
                        (col_l, "PCI_CE_E")
                    } else {
                        (col_r, "PCI_CE_W")
                    };
                    let (onode, _, _, onaming) = vrf.grid.find_bel(slr, (scol, row), kind).unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[(ocrds[onaming.tile], &onaming.pins["O"].name), (crd, wire)]);
                    return;
                }
            }
        }
        let scol = if col < grid.col_clk {
            grid.col_left()
        } else {
            grid.col_right()
        };
        let (onode, _, _, onaming) = vrf.grid.find_bel(slr, (scol, row), "PCI_CE_CNR").unwrap();
        let ocrds = vrf.get_node_crds(onode).unwrap();
        vrf.verify_node(&[(ocrds[onaming.tile], &onaming.pins["O"].name), (crd, wire)]);
    }
}

pub fn verify_bel(
    grid: &Grid,
    vrf: &mut Verifier,
    slr: SlrId,
    node: &ExpandedTileNode,
    bid: BelId,
) {
    let crds;
    if let Some(c) = vrf.get_node_crds(node) {
        crds = c;
    } else {
        return;
    }
    let nk = &vrf.db.nodes[node.kind];
    let nn = &vrf.db.node_namings[node.naming];
    let bel = &nk.bels[bid];
    let naming = &nn.bels[bid];
    let key = &**nk.bels.key(bid);
    let (col, row) = node.tiles[NodeTileId::from_idx(0)];
    match key {
        "RLL" => {
            if bel.pins.is_empty() {
                let mut pins = Vec::new();
                for (k, v) in &naming.pins {
                    pins.push((&**k, SitePinDir::In));
                    vrf.claim_node(&[(crds[naming.tile], &v.name)]);
                }
                vrf.verify_bel(slr, node, bid, "RESERVED_LL", &node.bels[bid], &pins, &[]);
            } else {
                vrf.verify_bel(slr, node, bid, "RESERVED_LL", &node.bels[bid], &[], &[]);
            }
        }
        _ if key.starts_with("SLICE") => {
            if grid.kind.is_virtex2() {
                vrf.verify_bel(
                    slr,
                    node,
                    bid,
                    "SLICE",
                    &node.bels[bid],
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
                vrf.claim_node(&[(crds[naming.tile], &naming.pins["DX"].name)]);
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["DX"].name,
                    &naming.pins["X"].name,
                );
                vrf.claim_node(&[(crds[naming.tile], &naming.pins["DY"].name)]);
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["DY"].name,
                    &naming.pins["Y"].name,
                );
                for pin in [
                    "F5", "FX", "COUT", "SHIFTOUT", "DIG", "BYOUT", "BXOUT", "BYINVOUT", "SOPOUT",
                ] {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
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
                    if dbel != key {
                        continue;
                    }
                    let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), sbel).unwrap();
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[dpin].name,
                        &onaming.pins[spin].name,
                    );
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[dpin].name)]);
                }
                if key == "SLICE3" {
                    // supposed to be connected? idk.
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["SHIFTIN"].name)]);

                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (col, row + 1), "SLICE3")
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.verify_node(&[
                            (crds[naming.tile], &naming.pins["DIG_S"].name),
                            (ocrds[naming.tile], &onaming.pins["DIG_LOCAL"].name),
                        ]);
                    }

                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (col, row + 1), "SLICE1")
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.claim_node(&[
                            (crds[naming.tile], &naming.pins["FXINB"].name),
                            (ocrds[naming.tile], &onaming.pins["FX_S"].name),
                        ]);
                        vrf.claim_pip(
                            ocrds[naming.tile],
                            &onaming.pins["FX_S"].name,
                            &onaming.pins["FX"].name,
                        );
                    } else {
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins["FXINB"].name)]);
                    }
                }
                for (dbel, sbel) in [("SLICE0", "SLICE1"), ("SLICE2", "SLICE3")] {
                    if key != dbel {
                        continue;
                    }
                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (col, row - 1), sbel)
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.claim_node(&[
                            (crds[naming.tile], &naming.pins["CIN"].name),
                            (ocrds[naming.tile], &onaming.pins["COUT_N"].name),
                        ]);
                        vrf.claim_pip(
                            ocrds[naming.tile],
                            &onaming.pins["COUT_N"].name,
                            &onaming.pins["COUT"].name,
                        );
                    } else {
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins["CIN"].name)]);
                    }
                }
                for (dbel, sbel) in [("SLICE0", "SLICE2"), ("SLICE1", "SLICE3")] {
                    if key != dbel {
                        continue;
                    }
                    let mut scol = col - 1;
                    if grid.columns[scol].kind == ColumnKind::Bram {
                        scol -= 1;
                    }
                    if let Some((onode, _, _, onaming)) = vrf.grid.find_bel(slr, (scol, row), sbel)
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.claim_node(&[
                            (crds[naming.tile], &naming.pins["SOPIN"].name),
                            (ocrds[naming.tile], &onaming.pins["SOPOUT_W"].name),
                        ]);
                        vrf.claim_pip(
                            ocrds[naming.tile],
                            &onaming.pins["SOPOUT_W"].name,
                            &onaming.pins["SOPOUT"].name,
                        );
                    } else {
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins["SOPIN"].name)]);
                    }
                }
            } else {
                let kind = if matches!(key, "SLICE0" | "SLICE2") {
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
                vrf.verify_bel(slr, node, bid, kind, &node.bels[bid], &pins, &[]);
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
                    if dbel != key {
                        continue;
                    }
                    let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), sbel).unwrap();
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[dpin].name,
                        &onaming.pins[spin].name,
                    );
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[dpin].name)]);
                }
                if key == "SLICE2" {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["SHIFTIN"].name)]);
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["ALTDIG"].name)]);
                }
                if key == "SLICE3" {
                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (col, row + 1), "SLICE2")
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.claim_node(&[
                            (crds[naming.tile], &naming.pins["FXINB"].name),
                            (ocrds[naming.tile], &onaming.pins["FX_S"].name),
                        ]);
                        vrf.claim_pip(
                            ocrds[naming.tile],
                            &onaming.pins["FX_S"].name,
                            &onaming.pins["FX"].name,
                        );
                    } else {
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins["FXINB"].name)]);
                    }
                }
                for (dbel, sbel) in [("SLICE0", "SLICE2"), ("SLICE1", "SLICE3")] {
                    if key != dbel {
                        continue;
                    }
                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (col, row - 1), sbel)
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.claim_node(&[
                            (crds[naming.tile], &naming.pins["CIN"].name),
                            (ocrds[naming.tile], &onaming.pins["COUT_N"].name),
                        ]);
                        vrf.claim_pip(
                            ocrds[naming.tile],
                            &onaming.pins["COUT_N"].name,
                            &onaming.pins["COUT"].name,
                        );
                    } else {
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins["CIN"].name)]);
                    }
                }
            }
        }
        _ if key.starts_with("TBUF") => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "TBUF",
                &node.bels[bid],
                &[("O", SitePinDir::Out)],
                &[],
            );
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["O"].name)]);
        }
        "TBUS" => {
            let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "TBUF0").unwrap();
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["BUS0"].name,
                &onaming.pins["O"].name,
            );
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["BUS2"].name,
                &onaming.pins["O"].name,
            );
            let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "TBUF1").unwrap();
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["BUS1"].name,
                &onaming.pins["O"].name,
            );
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["BUS3"].name,
                &onaming.pins["O"].name,
            );
            let mut scol = col - 1;
            loop {
                if scol.to_idx() == 0 {
                    for pin in ["BUS0", "BUS1", "BUS2", "BUS3"] {
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                    }
                    break;
                }
                if let Some((onode, _, _, onaming)) = vrf.grid.find_bel(slr, (scol, row), "TBUS") {
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.claim_node(&[
                        (crds[naming.tile], &naming.pins["BUS0"].name),
                        (ocrds[naming.tile], &onaming.pins["BUS3_E"].name),
                    ]);
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins["BUS1"].name),
                        (ocrds[naming.tile], &onaming.pins["BUS0"].name),
                    ]);
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins["BUS2"].name),
                        (ocrds[naming.tile], &onaming.pins["BUS1"].name),
                    ]);
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins["BUS3"].name),
                        (ocrds[naming.tile], &onaming.pins["BUS2"].name),
                    ]);
                    break;
                }
                scol -= 1;
            }
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["BUS3"].name,
                &naming.pins["BUS3_E"].name,
            );
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["BUS3_E"].name,
                &naming.pins["BUS3"].name,
            );
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["OUT"].name,
                &naming.pins["BUS2"].name,
            );
        }
        _ if key.starts_with("DCIRESET") => {
            vrf.verify_bel(slr, node, bid, "DCIRESET", &node.bels[bid], &[], &[]);
        }
        _ if key.starts_with("DCI") => {
            vrf.verify_bel(slr, node, bid, "DCI", &node.bels[bid], &[], &[]);
        }
        _ if key.starts_with("PTE2OMUX") => {
            let out = &naming.pins["OUT"].name;
            for (k, v) in &naming.pins {
                if k == "OUT" {
                    continue;
                }
                vrf.claim_pip(crds[naming.tile], out, &v.name);
            }
        }
        "STARTUP" | "CAPTURE" | "ICAP" | "SPI_ACCESS" | "BSCAN" | "JTAGPPC" | "PMV"
        | "DNA_PORT" | "PCILOGIC" => {
            vrf.verify_bel(slr, node, bid, key, &node.bels[bid], &[], &[]);
        }
        "BRAM" => {
            let kind = match grid.kind {
                GridKind::Spartan3A => "RAMB16BWE",
                GridKind::Spartan3ADsp => "RAMB16BWER",
                _ => "RAMB16",
            };
            vrf.verify_bel(slr, node, bid, kind, &node.bels[bid], &[], &[]);
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
                vrf.verify_bel(slr, node, bid, "MULT18X18SIO", &node.bels[bid], &pins, &[]);
                for (o, i) in &carry {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[o].name)]);
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[i].name)]);
                }
                let mut srow = row;
                loop {
                    if srow.to_idx() < 4 {
                        break;
                    }
                    srow -= 4;
                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (col, srow), "MULT")
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        for (o, i) in &carry {
                            vrf.verify_node(&[
                                (crds[naming.tile], &naming.pins[i].name),
                                (ocrds[naming.tile], &onaming.pins[o].name_far),
                            ]);
                            vrf.claim_pip(
                                ocrds[naming.tile],
                                &onaming.pins[o].name_far,
                                &onaming.pins[o].name,
                            );
                        }
                        break;
                    }
                }
            } else {
                vrf.verify_bel(slr, node, bid, "MULT18X18", &node.bels[bid], &[], &[]);
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
            vrf.verify_bel(slr, node, bid, "DSP48A", &node.bels[bid], &pins, &[]);
            for (o, i) in &carry {
                vrf.claim_node(&[(crds[naming.tile], &naming.pins[o].name)]);
                vrf.claim_node(&[(crds[naming.tile], &naming.pins[i].name)]);
            }
            let mut srow = row;
            loop {
                if srow.to_idx() < 4 {
                    break;
                }
                srow -= 4;
                if let Some((onode, _, _, onaming)) = vrf.grid.find_bel(slr, (col, srow), "DSP") {
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    for (o, i) in &carry {
                        vrf.verify_node(&[
                            (crds[naming.tile], &naming.pins[i].name),
                            (ocrds[naming.tile], &onaming.pins[o].name_far),
                        ]);
                        vrf.claim_pip(
                            ocrds[naming.tile],
                            &onaming.pins[o].name_far,
                            &onaming.pins[o].name,
                        );
                    }
                    break;
                }
            }
        }
        "RANDOR" => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "RESERVED_ANDOR",
                &node.bels[bid],
                &[
                    ("CIN0", SitePinDir::In),
                    ("CIN1", SitePinDir::In),
                    ("CPREV", SitePinDir::In),
                    ("O", SitePinDir::Out),
                ],
                &[],
            );
            if row == grid.row_bot() {
                for pin in ["CIN0", "CIN1", "CPREV", "O"] {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                }
            } else {
                for pin in ["CPREV", "O"] {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                }
                for (pin, sbel) in [("CIN1", "SLICE2"), ("CIN0", "SLICE3")] {
                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (col, row - 1), sbel)
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.claim_node(&[
                            (crds[naming.tile], &naming.pins[pin].name),
                            (ocrds[onaming.tile], &onaming.pins["COUT_N"].name),
                        ]);
                        vrf.claim_pip(
                            ocrds[onaming.tile],
                            &onaming.pins["COUT_N"].name,
                            &onaming.pins["COUT"].name,
                        );
                    } else {
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                    }
                }
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["O"].name_far,
                    &naming.pins["O"].name,
                );
                let mut ncol = col + 1;
                loop {
                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (ncol, row), "RANDOR")
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.claim_node(&[
                            (crds[naming.tile], &naming.pins["O"].name_far),
                            (ocrds[onaming.tile], &onaming.pins["CPREV"].name_far),
                        ]);
                        vrf.claim_pip(
                            ocrds[onaming.tile],
                            &onaming.pins["CPREV"].name,
                            &onaming.pins["CPREV"].name_far,
                        );
                        break;
                    } else if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (ncol, row), "RANDOR_OUT")
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.verify_node(&[
                            (crds[naming.tile], &naming.pins["O"].name_far),
                            (ocrds[onaming.tile], &onaming.pins["O"].name),
                        ]);
                        break;
                    } else {
                        ncol += 1;
                    }
                }
            }
        }
        "RANDOR_OUT" => (),
        _ if key.starts_with("GT") => {
            if grid.kind == GridKind::Virtex2PX {
                vrf.verify_bel(
                    slr,
                    node,
                    bid,
                    "GT10",
                    &node.bels[bid],
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
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[pin].name,
                        &naming.pins[pin].name_far,
                    );
                    let (onode, _, _, onaming) = vrf
                        .grid
                        .find_bel(slr, (grid.col_clk - 1, row), oname)
                        .unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[pin].name_far),
                        (ocrds[naming.tile], &onaming.pins["I"].name_far),
                    ]);
                }
            } else {
                vrf.verify_bel(
                    slr,
                    node,
                    bid,
                    "GT",
                    &node.bels[bid],
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
                let (onode, _, _, onaming) = vrf
                    .grid
                    .find_bel(slr, (grid.col_clk - 1, row), "BREFCLK")
                    .unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                for pin in ["BREFCLK", "BREFCLK2"] {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[pin].name,
                        &naming.pins[pin].name_far,
                    );
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[pin].name_far),
                        (ocrds[onaming.tile], &onaming.pins[pin].name),
                    ]);
                }
                vrf.claim_node(&[(crds[naming.tile], &naming.pins["TST10B8BICRD0"].name)]);
                vrf.claim_node(&[(crds[naming.tile], &naming.pins["TST10B8BICRD1"].name)]);
            }
            for (pin, okey) in [("RXP", "IPAD.RXP"), ("RXN", "IPAD.RXN")] {
                vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), okey).unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins[pin].name,
                    &onaming.pins["I"].name,
                );
            }
            for (pin, okey) in [("TXP", "OPAD.TXP"), ("TXN", "OPAD.TXN")] {
                vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), okey).unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &onaming.pins["O"].name,
                    &naming.pins[pin].name,
                );
            }
        }
        _ if key.starts_with("IPAD") => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "GTIPAD",
                &node.bels[bid],
                &[("I", SitePinDir::Out)],
                &[],
            );
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["I"].name)]);
        }
        _ if key.starts_with("OPAD") => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "GTOPAD",
                &node.bels[bid],
                &[("O", SitePinDir::In)],
                &[],
            );
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["O"].name)]);
        }
        _ if key.starts_with("IOI") => {
            let attr = grid.get_io_attr(vrf.grid, (col, row), bid);
            let tn = &node.names[NodeRawTileId::from_idx(0)];
            let is_ipad = tn.contains("IBUFS") || (tn.contains("IOIB") && bid.to_idx() == 2);
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
            vrf.verify_bel(slr, node, bid, kind, &node.bels[bid], &pins, &[]);
            // diff pairing
            if !grid.kind.is_virtex2() || attr.diff != IoDiffKind::None {
                for pin in ["PADOUT", "DIFFI_IN", "DIFFO_IN", "DIFFO_OUT"] {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                }
                match attr.diff {
                    IoDiffKind::P(obel) => {
                        let onaming = &nn.bels[obel];
                        vrf.claim_pip(
                            crds[naming.tile],
                            &naming.pins["DIFFI_IN"].name,
                            &onaming.pins["PADOUT"].name,
                        );
                    }
                    IoDiffKind::N(obel) => {
                        let onaming = &nn.bels[obel];
                        vrf.claim_pip(
                            crds[naming.tile],
                            &naming.pins["DIFFI_IN"].name,
                            &onaming.pins["PADOUT"].name,
                        );
                        vrf.claim_pip(
                            crds[naming.tile],
                            &naming.pins["DIFFO_IN"].name,
                            &onaming.pins["DIFFO_OUT"].name,
                        );
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
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                }
                // ODDR, IDDR
                if let IoDiffKind::P(obel) | IoDiffKind::N(obel) = attr.diff {
                    let onaming = &nn.bels[obel];
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins["ODDRIN1"].name,
                        &onaming.pins["ODDROUT2"].name,
                    );
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins["ODDRIN2"].name,
                        &onaming.pins["ODDROUT1"].name,
                    );
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins["IDDRIN1"].name,
                        &onaming.pins["IQ1"].name,
                    );
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins["IDDRIN2"].name,
                        &onaming.pins["IQ2"].name,
                    );
                }
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["PCI_CE"].name,
                    &naming.pins["PCI_CE"].name_far,
                );
                verify_pci_ce(
                    grid,
                    vrf,
                    slr,
                    col,
                    row,
                    crds[naming.tile],
                    &naming.pins["PCI_CE"].name_far,
                );
            }
            if grid.kind == GridKind::Spartan3ADsp {
                for pin in ["OAUX", "TAUX"] {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                }
            }
        }
        _ if key.starts_with("IOBS") => (),
        "BREFCLK" => {
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["BREFCLK"].name)]);
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["BREFCLK2"].name)]);
            if row == grid.row_bot() {
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "BUFGMUX6").unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["BREFCLK"].name,
                    &onaming.pins["CKI"].name_far,
                );
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "BUFGMUX0").unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["BREFCLK2"].name,
                    &onaming.pins["CKI"].name_far,
                );
            } else {
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "BUFGMUX4").unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["BREFCLK"].name,
                    &onaming.pins["CKI"].name_far,
                );
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "BUFGMUX2").unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["BREFCLK2"].name,
                    &onaming.pins["CKI"].name_far,
                );
            }
        }
        "BREFCLK_INT" => {
            let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "CLK_P").unwrap();
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["BREFCLK"].name,
                &onaming.pins["I"].name_far,
            );
        }
        "CLK_P" | "CLK_N" => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                key,
                &node.bels[bid],
                &[("I", SitePinDir::Out)],
                &[],
            );
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["I"].name)]);
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["I"].name_far)]);
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["I"].name_far,
                &naming.pins["I"].name,
            );
        }
        _ if key.starts_with("BUFGMUX") => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "BUFGMUX",
                &node.bels[bid],
                &[("I0", SitePinDir::In), ("I1", SitePinDir::In)],
                &["CLK"],
            );
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["I0"].name)]);
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["I1"].name)]);
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["I0"].name,
                &naming.pins["CLK"].name,
            );
            let obel = BelId::from_idx(bid.to_idx() ^ 1);
            let onaming = &nn.bels[obel];
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["I1"].name,
                &onaming.pins["CLK"].name,
            );
            let edge = if row == grid.row_bot() {
                Edge::Bot
            } else if row == grid.row_top() {
                Edge::Top
            } else if col == grid.col_left() {
                Edge::Left
            } else if col == grid.col_right() {
                Edge::Right
            } else {
                unreachable!()
            };
            if grid.kind.is_virtex2() || grid.kind == GridKind::Spartan3 {
                if let Some((crd, obel)) = grid.get_clk_io(edge, bid.to_idx()) {
                    let (onode, _, onaming, _) = grid.get_io_bel(vrf.grid, crd, obel).unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.claim_node(&[
                        (crds[naming.tile], &naming.pins["CKI"].name),
                        (ocrds[onaming.tile], &onaming.pins["IBUF"].name),
                    ]);
                    vrf.claim_pip(
                        ocrds[onaming.tile],
                        &onaming.pins["IBUF"].name,
                        &onaming.pins["I"].name,
                    );
                } else {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["CKI"].name)]);
                }
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["CKI"].name,
                );
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["DCM_OUT_L"].name,
                );
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["DCM_OUT_R"].name,
                );
                vrf.claim_node(&[(crds[naming.tile], &naming.pins["DCM_OUT_L"].name)]);
                vrf.claim_node(&[(crds[naming.tile], &naming.pins["DCM_OUT_R"].name)]);
                if grid.kind.is_virtex2() {
                    for pin in ["DCM_PAD_L", "DCM_PAD_R"] {
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                        vrf.claim_pip(
                            crds[naming.tile],
                            &naming.pins[pin].name,
                            &naming.pins["CKI"].name,
                        );
                    }
                } else {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["DCM_PAD"].name)]);
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins["DCM_PAD"].name,
                        &naming.pins["CKI"].name,
                    );
                }
            } else if matches!(edge, Edge::Bot | Edge::Top) {
                let (crd, obel) = grid.get_clk_io(edge, bid.to_idx()).unwrap();
                let (onode, _, onaming, _) = grid.get_io_bel(vrf.grid, crd, obel).unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.claim_node(&[
                    (crds[naming.tile], &naming.pins["CKIR"].name),
                    (ocrds[onaming.tile], &onaming.pins["IBUF"].name),
                ]);
                vrf.claim_pip(
                    ocrds[onaming.tile],
                    &onaming.pins["IBUF"].name,
                    &onaming.pins["I"].name,
                );
                let (crd, obel) = grid.get_clk_io(edge, bid.to_idx() + 4).unwrap();
                let (onode, _, onaming, _) = grid.get_io_bel(vrf.grid, crd, obel).unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.claim_node(&[
                    (crds[naming.tile], &naming.pins["CKIL"].name),
                    (ocrds[onaming.tile], &onaming.pins["IBUF"].name),
                ]);
                vrf.claim_pip(
                    ocrds[onaming.tile],
                    &onaming.pins["IBUF"].name,
                    &onaming.pins["I"].name,
                );
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["CKIL"].name,
                );
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["CKIR"].name,
                );

                let mut has_dcm_l = true;
                let mut has_dcm_r = true;
                if grid.kind == GridKind::Spartan3E {
                    if grid.dcms == Some(Dcms::Two) {
                        has_dcm_l = false;
                    }
                } else {
                    if grid.dcms == Some(Dcms::Two) && row == grid.row_bot() {
                        has_dcm_l = false;
                        has_dcm_r = false;
                    }
                }
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["DCM_OUT_L"].name,
                );
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["DCM_OUT_R"].name,
                );
                if has_dcm_l {
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins["DCM_PAD_L"].name,
                        &naming.pins["CKIL"].name,
                    );
                    let pip = &naming.pins["DCM_OUT_L"].pips[0];
                    vrf.claim_node(&[
                        (crds[naming.tile], &naming.pins["DCM_OUT_L"].name),
                        (crds[pip.tile], &pip.wire_to),
                    ]);
                    vrf.claim_pip(crds[pip.tile], &pip.wire_to, &pip.wire_from);
                    let row = match edge {
                        Edge::Bot => row + 1,
                        Edge::Top => row - 1,
                        _ => unreachable!(),
                    };
                    let (onode, _, _, onaming) =
                        vrf.grid.find_bel(slr, (col, row), "DCMCONN.S3E").unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    let (dcm_pad_pin, dcm_out_pin) = match (edge, bid.to_idx()) {
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
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins["DCM_PAD_L"].name),
                        (ocrds[onaming.tile], &onaming.pins[dcm_pad_pin].name),
                    ]);
                    vrf.verify_node(&[
                        (crds[pip.tile], &pip.wire_from),
                        (ocrds[onaming.tile], &onaming.pins[dcm_out_pin].name),
                    ]);
                } else {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["DCM_OUT_L"].name)]);
                }
                if has_dcm_r {
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins["DCM_PAD_R"].name,
                        &naming.pins["CKIR"].name,
                    );
                    let pip = &naming.pins["DCM_OUT_R"].pips[0];
                    vrf.claim_node(&[
                        (crds[naming.tile], &naming.pins["DCM_OUT_R"].name),
                        (crds[pip.tile], &pip.wire_to),
                    ]);
                    vrf.claim_pip(crds[pip.tile], &pip.wire_to, &pip.wire_from);
                    let row = match edge {
                        Edge::Bot => row + 1,
                        Edge::Top => row - 1,
                        _ => unreachable!(),
                    };
                    let (onode, _, _, onaming) = vrf
                        .grid
                        .find_bel(slr, (col + 1, row), "DCMCONN.S3E")
                        .unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    let (dcm_pad_pin, dcm_out_pin) = match (edge, bid.to_idx()) {
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
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins["DCM_PAD_R"].name),
                        (ocrds[onaming.tile], &onaming.pins[dcm_pad_pin].name),
                    ]);
                    vrf.verify_node(&[
                        (crds[pip.tile], &pip.wire_from),
                        (ocrds[onaming.tile], &onaming.pins[dcm_out_pin].name),
                    ]);
                } else {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["DCM_OUT_R"].name)]);
                }
            } else {
                let (crd, obel) = grid.get_clk_io(edge, bid.to_idx()).unwrap();
                let (onode, _, onaming, _) = grid.get_io_bel(vrf.grid, crd, obel).unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.verify_node(&[
                    (crds[naming.tile], &naming.pins["CKI"].name),
                    (ocrds[onaming.tile], &onaming.pins["IBUF"].name),
                ]);
                vrf.claim_pip(
                    ocrds[onaming.tile],
                    &onaming.pins["IBUF"].name,
                    &onaming.pins["I"].name,
                );
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["CKI"].name,
                );

                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name,
                    &naming.pins["DCM_OUT"].name,
                );
                if grid.dcms == Some(Dcms::Eight) {
                    let pad_pin;
                    if grid.kind != GridKind::Spartan3A {
                        pad_pin = "CKI";
                    } else {
                        pad_pin = "DCM_PAD";
                        vrf.claim_node(&[(crds[naming.tile], &naming.pins["CKI"].name)]);
                        vrf.claim_pip(
                            crds[naming.tile],
                            &naming.pins["DCM_PAD"].name,
                            &naming.pins["CKI"].name,
                        );
                    }
                    let col = if grid.kind == GridKind::Spartan3E {
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
                    let row = if bid.to_idx() < 4 {
                        grid.row_mid()
                    } else {
                        grid.row_mid() - 1
                    };
                    let (onode, _, _, onaming) =
                        vrf.grid.find_bel(slr, (col, row), "DCMCONN.S3E").unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    let (dcm_pad_pin, dcm_out_pin) = match bid.to_idx() {
                        0 | 4 => ("CLKPAD0", "OUT0"),
                        1 | 5 => ("CLKPAD1", "OUT1"),
                        2 | 6 => ("CLKPAD2", "OUT2"),
                        3 | 7 => ("CLKPAD3", "OUT3"),
                        _ => unreachable!(),
                    };
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[pad_pin].name),
                        (ocrds[onaming.tile], &onaming.pins[dcm_pad_pin].name),
                    ]);
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins["DCM_OUT"].name),
                        (ocrds[onaming.tile], &onaming.pins[dcm_out_pin].name),
                    ]);
                } else {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["CKI"].name)]);
                }
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "VCC").unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CLK"].name_far,
                    &onaming.pins["VCCOUT"].name,
                );
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["S"].name,
                    &onaming.pins["VCCOUT"].name,
                );
            }
        }
        "PCILOGICSE" => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "PCILOGICSE",
                &node.bels[bid],
                &[
                    ("IRDY", SitePinDir::In),
                    ("TRDY", SitePinDir::In),
                    ("PCI_CE", SitePinDir::Out),
                ],
                &[],
            );
            let edge = if col == grid.col_left() {
                Edge::Left
            } else if col == grid.col_right() {
                Edge::Right
            } else {
                unreachable!()
            };
            let pci_rdy = grid.get_pci_io(edge);
            for (pin, (crd, obel)) in ["IRDY", "TRDY"].into_iter().zip(pci_rdy.into_iter()) {
                vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins[pin].name,
                    &naming.pins[pin].name_far,
                );
                let (onode, _, onaming, _) = grid.get_io_bel(vrf.grid, crd, obel).unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.claim_node(&[
                    (crds[naming.tile], &naming.pins[pin].name_far),
                    (ocrds[onaming.tile], &onaming.pins["PCI_RDY_IN"].name),
                ]);
                vrf.claim_pip(
                    ocrds[onaming.tile],
                    &onaming.pins["PCI_RDY_IN"].name,
                    &onaming.pins["PCI_RDY"].name,
                );
            }
            let pip = &naming.pins["PCI_CE"].pips[0];
            vrf.claim_node(&[
                (crds[naming.tile], &naming.pins["PCI_CE"].name),
                (crds[pip.tile], &pip.wire_from),
            ]);
            vrf.claim_pip(crds[pip.tile], &pip.wire_to, &pip.wire_from);
            vrf.claim_node(&[(crds[pip.tile], &pip.wire_to)]);
        }
        "VCC" => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "VCC",
                &node.bels[bid],
                &[("VCCOUT", SitePinDir::Out)],
                &[],
            );
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["VCCOUT"].name)]);
        }
        "DCM" => {
            vrf.verify_bel(slr, node, bid, "DCM", &node.bels[bid], &[], &[]);
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
                if col < grid.col_clk {
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
            let opin_out = if col < grid.col_clk {
                "DCM_OUT_L"
            } else {
                "DCM_OUT_R"
            };
            for &(pin_o, pin_i, bel) in pins_out {
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins[pin_o].name,
                    &naming.pins[pin_i].name,
                );
                let (onode, _, _, onaming) = vrf
                    .grid
                    .find_bel(slr, (grid.col_clk - 1, row), bel)
                    .unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.verify_node(&[
                    (crds[naming.tile], &naming.pins[pin_o].name),
                    (ocrds[naming.tile], &onaming.pins[opin_out].name),
                ]);
            }
            for &(pin_o, pin_i, bel) in pins_pad {
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins[pin_o].name,
                    &naming.pins[pin_i].name,
                );
                let (onode, _, _, onaming) = vrf
                    .grid
                    .find_bel(slr, (grid.col_clk - 1, row), bel)
                    .unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.verify_node(&[
                    (crds[naming.tile], &naming.pins[pin_i].name),
                    (ocrds[naming.tile], &onaming.pins[opin_pad].name),
                ]);
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
            vrf.verify_bel(slr, node, bid, key, &node.bels[bid], &[], &skip_pins_ref);
            for pin in skip_pins {
                let spin = &pin[..pin.find('.').unwrap()];
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins[&pin].name,
                    &naming.pins[spin].name,
                );
            }
        }
        "PCI_CE_N" => {
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["O"].name)]);
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["O"].name,
                &naming.pins["I"].name,
            );
            verify_pci_ce(
                grid,
                vrf,
                slr,
                col,
                row - 1,
                crds[naming.tile],
                &naming.pins["I"].name,
            );
        }
        "PCI_CE_S" => {
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["O"].name)]);
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["O"].name,
                &naming.pins["I"].name,
            );
            verify_pci_ce(
                grid,
                vrf,
                slr,
                col,
                row,
                crds[naming.tile],
                &naming.pins["I"].name,
            );
        }
        "PCI_CE_E" => {
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["O"].name)]);
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["O"].name,
                &naming.pins["I"].name,
            );
            verify_pci_ce(
                grid,
                vrf,
                slr,
                col - 1,
                row,
                crds[naming.tile],
                &naming.pins["I"].name,
            );
        }
        "PCI_CE_W" => {
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["O"].name)]);
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["O"].name,
                &naming.pins["I"].name,
            );
            verify_pci_ce(
                grid,
                vrf,
                slr,
                col,
                row,
                crds[naming.tile],
                &naming.pins["I"].name,
            );
        }
        "PCI_CE_CNR" => {
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["O"].name)]);
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["O"].name,
                &naming.pins["I"].name,
            );
            verify_pci_ce(
                grid,
                vrf,
                slr,
                col,
                row,
                crds[naming.tile],
                &naming.pins["I"].name,
            );
        }
        _ if key.starts_with("GCLKH") => {
            for i in 0..8 {
                for ud in ["UP", "DN"] {
                    if matches!((key, ud), ("GCLKH.S", "UP") | ("GCLKH.N", "DN")) {
                        continue;
                    }
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{ud}{i}")].name,
                        &naming.pins[&format!("IN{i}")].name,
                    );
                }
                if grid.kind.is_virtex2() {
                    let lr = if col < grid.col_clk { 'L' } else { 'R' };
                    let (onode, _, _, onaming) = vrf
                        .grid
                        .find_bel(slr, (grid.col_clk, row + 1), "GCLKC")
                        .unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[&format!("IN{i}")].name),
                        (
                            ocrds[onaming.tile],
                            &onaming.pins[&format!("OUT_{lr}{i}")].name,
                        ),
                    ]);
                } else if let Some((col_cl, col_cr)) = grid.cols_clkv {
                    let scol = if col < grid.col_clk { col_cl } else { col_cr };
                    let lr = if col < scol { 'L' } else { 'R' };
                    let (onode, _, _, onaming) =
                        vrf.grid.find_bel(slr, (scol, row + 1), "GCLKVC").unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[&format!("IN{i}")].name),
                        (
                            ocrds[onaming.tile],
                            &onaming.pins[&format!("OUT_{lr}{i}")].name,
                        ),
                    ]);
                } else {
                    let lr = if col < grid.col_clk { 'L' } else { 'R' };
                    let (onode, _, _, onaming) = vrf
                        .grid
                        .find_bel(slr, (grid.col_clk, grid.row_mid()), "CLKC_50A")
                        .unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[&format!("IN{i}")].name),
                        (
                            ocrds[onaming.tile],
                            &onaming.pins[&format!("OUT_{lr}{i}")].name,
                        ),
                    ]);
                }
            }
        }
        "GCLKC" => {
            for i in 0..8 {
                for lr in ['L', 'R'] {
                    vrf.claim_node(&[(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{lr}{i}")].name,
                    )]);
                    for bt in ['B', 'T'] {
                        vrf.claim_pip(
                            crds[naming.tile],
                            &naming.pins[&format!("OUT_{lr}{i}")].name,
                            &naming.pins[&format!("IN_{bt}{i}")].name,
                        );
                    }
                }
                for bt in ['B', 'T'] {
                    let (onode, _, _, onaming) = vrf
                        .grid
                        .find_bel(slr, (grid.col_clk, grid.row_mid()), "CLKC")
                        .unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[&format!("IN_{bt}{i}")].name),
                        (
                            ocrds[onaming.tile],
                            &onaming.pins[&format!("OUT_{bt}{i}")].name,
                        ),
                    ]);
                }
            }
        }
        "CLKC" => {
            if grid.kind.is_virtex2() {
                for i in 0..8 {
                    for bt in ['B', 'T'] {
                        vrf.claim_node(&[(
                            crds[naming.tile],
                            &naming.pins[&format!("OUT_{bt}{i}")].name,
                        )]);
                        vrf.claim_pip(
                            crds[naming.tile],
                            &naming.pins[&format!("OUT_{bt}{i}")].name,
                            &naming.pins[&format!("IN_{bt}{i}")].name,
                        );
                        let srow = if bt == 'B' {
                            grid.row_bot()
                        } else {
                            grid.row_top()
                        };
                        let (onode, _, _, onaming) = vrf
                            .grid
                            .find_bel(slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{i}"))
                            .unwrap();
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.verify_node(&[
                            (crds[naming.tile], &naming.pins[&format!("IN_{bt}{i}")].name),
                            (ocrds[onaming.tile], &onaming.pins["O"].name_far),
                        ]);
                    }
                }
            } else {
                for i in 0..8 {
                    let (bt, j) = if i < 4 { ('B', i) } else { ('T', i - 4) };
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins[&format!("OUT{i}")].name)]);
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT{i}")].name,
                        &naming.pins[&format!("IN_{bt}{j}")].name,
                    );
                    let srow = if bt == 'B' {
                        grid.row_bot()
                    } else {
                        grid.row_top()
                    };
                    let (onode, _, _, onaming) = vrf
                        .grid
                        .find_bel(slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{j}"))
                        .unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[&format!("IN_{bt}{j}")].name),
                        (ocrds[onaming.tile], &onaming.pins["O"].name_far),
                    ]);
                }
            }
        }
        "CLKC_50A" => {
            for i in 0..8 {
                let (bt, j) = if i < 4 { ('B', i) } else { ('T', i - 4) };
                for lr in ['L', 'R'] {
                    vrf.claim_node(&[(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{lr}{i}")].name,
                    )]);
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{lr}{i}")].name,
                        &naming.pins[&format!("IN_{bt}{j}")].name,
                    );
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{lr}{i}")].name,
                        &naming.pins[&format!("IN_{lr}{i}")].name,
                    );
                    let scol = if lr == 'L' {
                        grid.col_left()
                    } else {
                        grid.col_right()
                    };
                    let (onode, _, _, onaming) = vrf
                        .grid
                        .find_bel(slr, (scol, grid.row_mid() - 1), &format!("BUFGMUX{i}"))
                        .unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[&format!("IN_{lr}{i}")].name),
                        (ocrds[onaming.tile], &onaming.pins["O"].name_far),
                    ]);
                }
                let srow = if bt == 'B' {
                    grid.row_bot()
                } else {
                    grid.row_top()
                };
                let (onode, _, _, onaming) = vrf
                    .grid
                    .find_bel(slr, (grid.col_clk - 1, srow), &format!("BUFGMUX{j}"))
                    .unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.verify_node(&[
                    (crds[naming.tile], &naming.pins[&format!("IN_{bt}{j}")].name),
                    (ocrds[onaming.tile], &onaming.pins["O"].name_far),
                ]);
            }
        }
        "GCLKVM" => {
            for i in 0..8 {
                for ud in ["UP", "DN"] {
                    vrf.claim_node(&[(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{ud}{i}")].name,
                    )]);
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{ud}{i}")].name,
                        &naming.pins[&format!("IN_CORE{i}")].name,
                    );
                    if grid.kind != GridKind::Spartan3 {
                        vrf.claim_pip(
                            crds[naming.tile],
                            &naming.pins[&format!("OUT_{ud}{i}")].name,
                            &naming.pins[&format!("IN_LR{i}")].name,
                        );
                    }
                }
                let (onode, _, _, onaming) =
                    vrf.grid.find_bel(slr, (grid.col_clk, row), "CLKC").unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.verify_node(&[
                    (crds[naming.tile], &naming.pins[&format!("IN_CORE{i}")].name),
                    (ocrds[onaming.tile], &onaming.pins[&format!("OUT{i}")].name),
                ]);
                if grid.kind != GridKind::Spartan3 {
                    let scol = if col < grid.col_clk {
                        grid.col_left()
                    } else {
                        grid.col_right()
                    };
                    let (onode, _, _, onaming) = vrf
                        .grid
                        .find_bel(slr, (scol, grid.row_mid() - 1), &format!("BUFGMUX{i}"))
                        .unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins[&format!("IN_LR{i}")].name),
                        (ocrds[onaming.tile], &onaming.pins["O"].name_far),
                    ]);
                }
            }
        }
        "GCLKVC" => {
            for i in 0..8 {
                for lr in ['L', 'R'] {
                    vrf.claim_node(&[(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{lr}{i}")].name,
                    )]);
                    vrf.claim_pip(
                        crds[naming.tile],
                        &naming.pins[&format!("OUT_{lr}{i}")].name,
                        &naming.pins[&format!("IN{i}")].name,
                    );
                }
                let ud = if row < grid.row_mid() { "DN" } else { "UP" };
                let (onode, _, _, onaming) = vrf
                    .grid
                    .find_bel(slr, (col, grid.row_mid()), "GCLKVM")
                    .unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.verify_node(&[
                    (crds[naming.tile], &naming.pins[&format!("IN{i}")].name),
                    (
                        ocrds[onaming.tile],
                        &onaming.pins[&format!("OUT_{ud}{i}")].name,
                    ),
                ]);
            }
        }
        _ if key.starts_with("GLOBALSIG") => {
            vrf.verify_bel(slr, node, bid, "GLOBALSIG", &node.bels[bid], &[], &[]);
        }
        _ => {
            println!("MEOW {} {:?}", key, node.bels.get(bid));
        }
    }
}
