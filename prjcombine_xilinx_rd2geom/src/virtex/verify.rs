use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::virtex::{Grid, GridKind};
use prjcombine_xilinx_geom::int::NodeTileId;
use prjcombine_xilinx_geom::eint::ExpandedTileNode;
use prjcombine_xilinx_geom::{BelId, SlrId};

use crate::verify::{SitePinDir, Verifier};

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
        _ if key.starts_with("SLICE") => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "SLICE",
                &node.bels[bid],
                &[
                    ("CIN", SitePinDir::In),
                    ("COUT", SitePinDir::Out),
                    ("F5IN", SitePinDir::In),
                    ("F5", SitePinDir::Out),
                ],
                &[],
            );
            if let Some((onode, _, _, onaming)) = vrf.grid.find_bel(slr, (col, row - 1), key) {
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.claim_node(&[
                    (crds[naming.tile], &naming.pins["CIN"].name),
                    (ocrds[naming.tile], &onaming.pins["COUT"].name_far),
                ]);
            } else {
                vrf.claim_node(&[(crds[naming.tile], &naming.pins["CIN"].name)]);
            }
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["COUT"].name)]);
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["COUT"].name_far,
                &naming.pins["COUT"].name,
            );

            vrf.claim_node(&[(crds[naming.tile], &naming.pins["F5"].name)]);
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["F5IN"].name)]);
            let okey = match key {
                "SLICE0" => "SLICE1",
                "SLICE1" => "SLICE0",
                _ => unreachable!(),
            };
            let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), okey).unwrap();
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["F5IN"].name,
                &onaming.pins["F5"].name,
            );
        }
        _ if key.starts_with("IOB") => {
            let mut kind = "IOB";
            let mut pins = Vec::new();
            if node.bels[bid].starts_with("EMPTY") {
                kind = "EMPTYIOB";
            }
            if (col == grid.col_lio() || col == grid.col_rio())
                && ((row == grid.row_mid() && key == "IOB3")
                    || (row == grid.row_mid() - 1 && key == "IOB1"))
            {
                kind = "PCIIOB";
                pins.push(("PCI", SitePinDir::Out));
            }
            if grid.kind != GridKind::Virtex
                && (row == grid.row_bio() || row == grid.row_tio())
                && ((col == grid.col_clk() && key == "IOB2")
                    || (col == grid.col_clk() - 1 && key == "IOB1"))
            {
                kind = "DLLIOB";
                pins.push(("DLLFB", SitePinDir::Out));
            }
            vrf.verify_bel(slr, node, bid, kind, &node.bels[bid], &pins, &[]);
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
            if bel.pins.contains_key("BUS3_E") {
                let col_r = vrf.grid.slr(slr).cols().next_back().unwrap();
                if col.to_idx() < col_r.to_idx() - 5 {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["BUS3_E"].name)]);
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
                let mut col_r = col + 1;
                loop {
                    if let Some((onode, _, _, onaming)) =
                        vrf.grid.find_bel(slr, (col_r, row), "TBUS")
                    {
                        let ocrds = vrf.get_node_crds(onode).unwrap();
                        vrf.verify_node(&[
                            (crds[naming.tile], &naming.pins["BUS0"].name),
                            (ocrds[onaming.tile], &onaming.pins["BUS1"].name),
                        ]);
                        vrf.verify_node(&[
                            (crds[naming.tile], &naming.pins["BUS1"].name),
                            (ocrds[onaming.tile], &onaming.pins["BUS2"].name),
                        ]);
                        vrf.verify_node(&[
                            (crds[naming.tile], &naming.pins["BUS2"].name),
                            (ocrds[onaming.tile], &onaming.pins["BUS3"].name),
                        ]);
                        vrf.verify_node(&[
                            (crds[naming.tile], &naming.pins["BUS3_E"].name),
                            (ocrds[onaming.tile], &onaming.pins["BUS0"].name),
                        ]);
                        break;
                    } else {
                        col_r += 1;
                    }
                }
            }
            if bel.pins.contains_key("OUT") {
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["OUT"].name,
                    &naming.pins["BUS2"].name,
                );
            }
        }
        "BRAM" => {
            vrf.verify_bel(slr, node, bid, "BLOCKRAM", &node.bels[bid], &[], &[]);
        }
        "STARTUP" | "CAPTURE" | "BSCAN" => {
            vrf.verify_bel(slr, node, bid, key, &node.bels[bid], &[], &[]);
        }
        _ if key.starts_with("GCLKIOB") => {
            vrf.verify_bel(slr, node, bid, "GCLKIOB", &node.bels[bid], &[], &[]);
        }
        _ if key.starts_with("BUFG") => {
            vrf.verify_bel(slr, node, bid, "GCLK", &node.bels[bid], &[], &[]);
        }
        "IOFB0" => {
            let (onode, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), "IOB2").unwrap();
            let ocrds = vrf.get_node_crds(onode).unwrap();
            vrf.verify_node(&[
                (crds[naming.tile], &naming.pins["O"].name),
                (ocrds[onaming.tile], &onaming.pins["DLLFB"].name),
            ]);
        }
        "IOFB1" => {
            let (onode, _, _, onaming) = vrf.grid.find_bel(slr, (col - 1, row), "IOB1").unwrap();
            let ocrds = vrf.get_node_crds(onode).unwrap();
            vrf.verify_node(&[
                (crds[naming.tile], &naming.pins["O"].name),
                (ocrds[onaming.tile], &onaming.pins["DLLFB"].name),
            ]);
        }
        "PCILOGIC" => {
            vrf.verify_bel(
                slr,
                node,
                bid,
                "PCILOGIC",
                &node.bels[bid],
                &[("IRDY", SitePinDir::In), ("TRDY", SitePinDir::In)],
                &[],
            );
            for pin in ["IRDY", "TRDY"] {
                for pip in &naming.pins[pin].pips {
                    vrf.claim_pip(crds[pip.tile], &pip.wire_to, &pip.wire_from);
                }
                vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name)]);
                vrf.claim_node(&[(crds[naming.tile], &naming.pins[pin].name_far)]);
            }
            let (onode, _, _, onaming) = vrf
                .grid
                .find_bel(slr, (col, grid.row_mid()), "IOB3")
                .unwrap();
            let ocrds = vrf.get_node_crds(onode).unwrap();
            vrf.verify_node(&[
                (crds[naming.tile], &naming.pins["IRDY"].name_far),
                (ocrds[onaming.tile], &onaming.pins["PCI"].name),
            ]);
            let (onode, _, _, onaming) = vrf
                .grid
                .find_bel(slr, (col, grid.row_mid() - 1), "IOB1")
                .unwrap();
            let ocrds = vrf.get_node_crds(onode).unwrap();
            vrf.verify_node(&[
                (crds[naming.tile], &naming.pins["TRDY"].name_far),
                (ocrds[onaming.tile], &onaming.pins["PCI"].name),
            ]);
        }
        "DLL" => {
            vrf.verify_bel(slr, node, bid, "DLL", &node.bels[bid], &[], &[]);
        }
        _ => unreachable!(),
    }
}
