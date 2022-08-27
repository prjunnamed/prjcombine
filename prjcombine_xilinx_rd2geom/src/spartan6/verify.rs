use crate::verify::{SitePinDir, Verifier};
use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::eint::ExpandedTileNode;
use prjcombine_xilinx_geom::int::NodeTileId;
use prjcombine_xilinx_geom::spartan6::Grid;
use prjcombine_xilinx_geom::{BelId, SlrId};

pub fn verify_bel(
    _grid: &Grid,
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
        "SLICE0" => {
            let kind = if bel.pins.contains_key("WE") {
                "SLICEM"
            } else {
                "SLICEL"
            };
            vrf.verify_bel(
                slr,
                node,
                bid,
                kind,
                &node.bels[bid],
                &[("CIN", SitePinDir::In), ("COUT", SitePinDir::Out)],
                &[],
            );
            let mut srow = row;
            loop {
                if srow.to_idx() == 0 {
                    vrf.claim_node(&[(crds[naming.tile], &naming.pins["CIN"].name)]);
                    break;
                }
                srow -= 1;
                if let Some((onode, _, _, onaming)) = vrf.grid.find_bel(slr, (col, srow), "SLICE0")
                {
                    let ocrds = vrf.get_node_crds(onode).unwrap();
                    vrf.claim_node(&[
                        (crds[naming.tile], &naming.pins["CIN"].name),
                        (ocrds[onaming.tile], &onaming.pins["COUT"].name_far),
                    ]);
                    vrf.claim_pip(
                        ocrds[naming.tile],
                        &onaming.pins["COUT"].name_far,
                        &onaming.pins["COUT"].name,
                    );
                    break;
                }
            }
            vrf.claim_node(&[(crds[naming.tile], &naming.pins["COUT"].name)]);
        }
        "SLICE1" => {
            vrf.verify_bel(slr, node, bid, "SLICEX", &node.bels[bid], &[], &[]);
        }
        _ => {
            println!("MEOW {} {:?}", key, node.bels.get(bid));
        }
    }
}
