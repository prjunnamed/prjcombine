use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::eint::ExpandedTileNode;
use prjcombine_xilinx_geom::int::NodeTileId;
use prjcombine_xilinx_geom::xc5200::Grid;
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
    let naming = &nn.bels[bid];
    let key = &**nk.bels.key(bid);
    let (col, row) = node.tiles[NodeTileId::from_idx(0)];
    match key {
        _ if key.starts_with("LC") => {
            let kind = match key {
                "LC0" | "LC2" => "LC5A",
                "LC1" | "LC3" => "LC5B",
                _ => unreachable!(),
            };
            let mut pins = vec![("CI", SitePinDir::In), ("CO", SitePinDir::Out)];
            if kind == "LC5A" {
                pins.push(("F5I", SitePinDir::In));
                let okey = match key {
                    "LC0" => "LC1",
                    "LC2" => "LC3",
                    _ => unreachable!(),
                };
                vrf.claim_node(&[(crds[naming.tile], &naming.pins["F5I"].name)]);
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), okey).unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["F5I"].name,
                    &onaming.pins["X"].name,
                );
            }
            vrf.verify_bel(slr, node, bid, kind, &node.bels[bid], &pins, &[]);
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["CI"].name)]);
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["CO"].name)]);
            if key == "LC0" {
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CI"].name,
                    &naming.pins["CI"].name_far,
                );
                if let Some((onode, _, _, onaming)) = vrf.grid.find_bel(slr, (col, row - 1), "LC3")
                {
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.claim_node(&[
                        (crds[naming.tile], &naming.pins["CI"].name_far),
                        (ocrds[onaming.tile], &onaming.pins["CO"].name_far),
                    ]);
                } else {
                    let (onode, _, _, onaming) =
                        vrf.grid.find_bel(slr, (col, row - 1), "BOT_CIN").unwrap();
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.verify_node(&[
                        (crds[naming.tile], &naming.pins["CI"].name_far),
                        (ocrds[onaming.tile], &onaming.pins["IN"].name),
                    ]);
                }
            } else {
                let okey = match key {
                    "LC1" => "LC0",
                    "LC2" => "LC1",
                    "LC3" => "LC2",
                    _ => unreachable!(),
                };
                let (_, _, _, onaming) = vrf.grid.find_bel(slr, (col, row), okey).unwrap();
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CI"].name,
                    &onaming.pins["CO"].name,
                );
            }
            if key == "LC3" {
                vrf.claim_pip(
                    crds[naming.tile],
                    &naming.pins["CO"].name_far,
                    &naming.pins["CO"].name,
                );
            }
        }
        _ if key.starts_with("IOB") => {
            let mut pins = vec![];
            let kind = if naming.pins.contains_key("CLKIN") {
                pins.push(("CLKIN", SitePinDir::Out));
                let st = if row == grid.row_bio() {
                    (grid.col_lio(), grid.row_bio())
                } else if row == grid.row_tio() {
                    (grid.col_rio(), grid.row_tio())
                } else if col == grid.col_lio() {
                    (grid.col_lio(), grid.row_tio())
                } else if col == grid.col_rio() {
                    (grid.col_rio(), grid.row_bio())
                } else {
                    unreachable!()
                };
                let (onode, _, _, onaming) = vrf.grid.find_bel(slr, st, "CLKIOB").unwrap();
                let ocrds = vrf.get_node_crds(onode).unwrap();
                vrf.verify_node(&[
                    (crds[naming.tile], &naming.pins["CLKIN"].name),
                    (ocrds[onaming.tile], &onaming.pins["OUT"].name),
                ]);
                "CLKIOB"
            } else {
                "IOB"
            };
            vrf.verify_bel(slr, node, bid, kind, &node.bels[bid], &pins, &[]);
        }
        _ if key.starts_with("TBUF") => {
            vrf.verify_bel(slr, node, bid, "TBUF", &node.bels[bid], &[], &[]);
        }
        "BUFG" => {
            vrf.verify_bel(slr, node, bid, "CLK", &node.bels[bid], &[], &[]);
        }
        "CLKIOB" => (),
        "BUFR" => {
            vrf.claim_pip(
                crds[naming.tile],
                &naming.pins["OUT"].name,
                &naming.pins["IN"].name,
            );
        }
        "TOP_COUT" => {
            let (onode, _, _, onaming) = vrf.grid.find_bel(slr, (col, row - 1), "LC3").unwrap();
            let ocrds = vrf.get_node_crds(onode).unwrap();
            vrf.verify_node(&[
                (crds[naming.tile], &naming.pins["OUT"].name),
                (ocrds[onaming.tile], &onaming.pins["CO"].name_far),
            ]);
        }
        "BOT_CIN" => (),
        "RDBK" | "STARTUP" | "BSCAN" | "OSC" | "BYPOSC" | "BSUPD" | "VCC_GND" => {
            vrf.verify_bel(slr, node, bid, key, &node.bels[bid], &[], &[]);
        }
        _ => println!("MEOW {key}"),
    }
}
