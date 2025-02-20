use std::collections::{BTreeMap, HashMap};

use prjcombine_interconnect::db::{Dir, PinDir};
use prjcombine_rawdump::{Part, TkSitePinDir, TkSiteSlot};
use prjcombine_virtex4::gtz::{GtzBel, GtzClkPin, GtzDb, GtzIntColId, GtzIntPin, GtzIntRowId};
use unnamed_entity::EntityId;

pub fn extract_gtz(rd: &Part) -> GtzDb {
    let mut gdb = GtzDb::default();
    for (side, tkn) in [(Dir::N, "GTZ_TOP"), (Dir::S, "GTZ_BOT")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut gtz = GtzBel {
                side,
                pins: BTreeMap::new(),
                clk_pins: BTreeMap::new(),
            };
            let tile = &rd.tiles[&xy];
            let slot_kind = rd.slot_kinds.get("GTZE2_OCTAL").unwrap();
            let tile_kind = &rd.tile_kinds[tile.kind];
            let site = tile_kind
                .sites
                .get(&TkSiteSlot::Xy(slot_kind, 0, 0))
                .unwrap()
                .1;
            let mut bufs_fwd = HashMap::new();
            let mut bufs_bwd = HashMap::new();
            for &(wf, wt) in tile_kind.pips.keys() {
                bufs_bwd.insert(wt, wf);
                bufs_fwd.insert(wf, wt);
            }
            for (pin_name, pin) in &site.pins {
                if pin_name.starts_with("GTREFCLK")
                    || pin_name.starts_with("GTZRXP")
                    || pin_name.starts_with("GTZRXN")
                    || pin_name.starts_with("GTZTXP")
                    || pin_name.starts_with("GTZTXN")
                {
                    continue;
                }
                let dir = match pin.dir {
                    TkSitePinDir::Input => PinDir::Input,
                    TkSitePinDir::Output => PinDir::Output,
                    TkSitePinDir::Bidir => unreachable!(),
                };
                let owire = if dir == PinDir::Input {
                    bufs_bwd[&pin.wire.unwrap()]
                } else {
                    bufs_fwd[&pin.wire.unwrap()]
                };
                let owire = &rd.wires[owire];
                if let Some(idx) = owire.strip_prefix("GTZ_VBRK_INTF_GCLK") {
                    let idx = idx.parse().unwrap();
                    gtz.clk_pins
                        .insert(pin_name.clone(), GtzClkPin { dir, idx });
                } else if let Some(tail) = owire.strip_prefix("GTZ_VBRK_INTF_SLV_") {
                    let (col, row) = tail.split_once('_').unwrap();
                    let col: usize = col.parse().unwrap();
                    let mut row: usize = row.parse().unwrap();
                    if side == Dir::N {
                        row = 48 - row;
                    }
                    gtz.pins.insert(
                        pin_name.clone(),
                        GtzIntPin {
                            dir,
                            col: GtzIntColId::from_idx(col),
                            row: GtzIntRowId::from_idx(row),
                        },
                    );
                } else {
                    panic!("ummm wtf is {owire}");
                }
            }
            gdb.gtz.insert(tkn.into(), gtz);
        }
    }
    gdb
}
