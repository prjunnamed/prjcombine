use std::collections::{BTreeMap, HashMap, HashSet};

use prjcombine_interconnect::db::{MuxInfo, NodeKindId, NodeWireId};
use prjcombine_re_fpga_hammer::{
    Collector, Diff, FeatureData, FeatureId, OcdMode, State, xlat_bit, xlat_bitvec, xlat_enum_ocd,
};
use prjcombine_re_harvester::Harvester;
use prjcombine_siliconblue::{
    chip::ChipKind,
    expanded::{BitOwner, ExpandedDevice},
};
use prjcombine_types::tiledb::TileDb;

pub fn collect(
    edev: &ExpandedDevice,
    muxes: &BTreeMap<NodeKindId, BTreeMap<NodeWireId, MuxInfo>>,
    harvester: &Harvester<BitOwner>,
) -> TileDb {
    let mut tiledb = TileDb::new();
    let mut state = State::new();
    let mut bitvec_diffs: BTreeMap<FeatureId, BTreeMap<usize, Diff>> = BTreeMap::new();
    for (key, bits) in &harvester.known_global {
        println!("unhandled global: {key}: {bits:?}");
    }
    for (key, bits) in &harvester.known_tiled {
        let &[tile, bel, attr, val] = Vec::from_iter(key.split(':')).as_slice() else {
            unreachable!()
        };
        let diff = Diff {
            bits: HashMap::from_iter(bits.iter().map(|(&k, &v)| (k, v))),
        };
        if let Some(idx) = val.strip_prefix("BIT") {
            let fid = FeatureId {
                tile: tile.to_string(),
                bel: bel.to_string(),
                attr: attr.to_string(),
                val: "".to_string(),
            };
            let idx: usize = idx.parse().unwrap();
            bitvec_diffs.entry(fid).or_default().insert(idx, diff);
        } else {
            let fid = FeatureId {
                tile: tile.to_string(),
                bel: bel.to_string(),
                attr: attr.to_string(),
                val: val.to_string(),
            };
            state.features.insert(
                fid,
                FeatureData {
                    diffs: vec![diff],
                    fuzzers: vec![],
                },
            );
        }
    }
    for (fid, mut diffs) in bitvec_diffs {
        let diffs = Vec::from_iter((0..diffs.len()).map(|idx| diffs.remove(&idx).unwrap()));
        state.features.insert(
            fid,
            FeatureData {
                diffs,
                fuzzers: vec![],
            },
        );
    }
    // TODO: deal with globals I guess.
    let mut collector = Collector {
        state: &mut state,
        tiledb: &mut tiledb,
    };

    for (&node_kind, tile_muxes) in muxes {
        let tile = edev.egrid.db.nodes.key(node_kind);
        let bel = "INT";
        if !tile.starts_with("IO") {
            collector.collect_bit(tile, bel, "INV.IMUX.CLK", "");
        }
        for (&(_, wt), mux) in tile_muxes {
            let wtn = edev.egrid.db.wires.key(wt);
            let mux_name = format!("MUX.{wtn}");
            let mut values = vec![];
            if tile == "PLB" && wtn.starts_with("IMUX.LC") && wtn.ends_with(".I3") {
                values.push("CI");
            }
            for &(_, wf) in &mux.ins {
                let wfn = edev.egrid.db.wires.key(wf);
                if (wfn.starts_with("OUT") && (wtn.starts_with("QUAD") || wtn.starts_with("LONG")))
                    || (wfn.starts_with("LONG") && wtn.starts_with("QUAD"))
                {
                    let item = collector.extract_bit(tile, bel, &mux_name, wfn);
                    collector
                        .tiledb
                        .insert(tile, bel, format!("BUF.{wtn}.{wfn}"), item);
                } else {
                    values.push(wfn);
                }
            }
            let mut diffs = vec![];
            if values.is_empty() {
                continue;
            }
            diffs.push(("NONE", Diff::default()));
            for val in values {
                diffs.push((val, collector.state.get_diff(tile, bel, &mux_name, val)));
            }
            if let Some(idx) = wtn.strip_prefix("LOCAL.") {
                let (a, b) = idx.split_once('.').unwrap();
                let a: usize = a.parse().unwrap();
                let b: usize = b.parse().unwrap();
                if a == 0 && b >= 4 {
                    let g2l_wire = edev.egrid.db.get_wire(&format!("GOUT.{}", b - 4));
                    let g2l_name = edev.egrid.db.wires.key(g2l_wire);

                    let mut bits_nog2l = HashSet::new();
                    for (wfn, diff) in &diffs {
                        if !wfn.starts_with("GLOBAL") {
                            for &bit in diff.bits.keys() {
                                bits_nog2l.insert(bit);
                            }
                        }
                    }
                    let mut diffs_g2l = vec![];
                    for (wfn, diff) in &mut diffs {
                        if wfn.starts_with("GLOBAL") {
                            let mut diff_g2l = std::mem::take(diff);
                            *diff = diff_g2l.split_bits(&bits_nog2l);
                            diffs_g2l.push((*wfn, diff_g2l));
                            *wfn = g2l_name;
                        }
                    }
                    if !diffs_g2l.is_empty() {
                        diffs_g2l.push(("NONE", Diff::default()));
                        collector.tiledb.insert(
                            tile,
                            bel,
                            format!("MUX.{g2l_name}"),
                            xlat_enum_ocd(diffs_g2l, OcdMode::Mux),
                        );
                    }
                }
            }
            collector
                .tiledb
                .insert(tile, bel, &mux_name, xlat_enum_ocd(diffs, OcdMode::Mux));
        }
    }
    if edev.chip.kind.has_colbuf() {
        for i in 0..8 {
            collector.collect_bit("PLB", "COLBUF", &format!("GLOBAL.{i}"), "");
            collector.collect_bit("INT.BRAM", "COLBUF", &format!("GLOBAL.{i}"), "");
            if edev.chip.kind.has_io_we() {
                collector.collect_bit("IO.W", "COLBUF", &format!("GLOBAL.{i}"), "");
                collector.collect_bit("IO.E", "COLBUF", &format!("GLOBAL.{i}"), "");
            }
        }
    }
    for lc in 0..8 {
        let tile = "PLB";
        let bel = &format!("LC{lc}");
        if lc != 0 && edev.chip.kind.is_ice40() {
            collector.collect_enum_default(tile, bel, "MUX.I2", &["LTIN"], "INT");
        }
        collector.collect_bitvec(tile, bel, "LUT_INIT", "");
        collector.collect_bit(tile, bel, "CARRY_ENABLE", "");
        collector.collect_bit(tile, bel, "FF_ENABLE", "");
        collector.collect_bit(tile, bel, "FF_SR_VALUE", "");
        collector.collect_bit(tile, bel, "FF_SR_ASYNC", "");
        if lc == 0 {
            collector.collect_enum(tile, bel, "MUX.CI", &["0", "1", "CHAIN"]);
        }
    }
    if !edev.chip.cols_bram.is_empty() {
        let tile = "BRAM";
        let bel = "BRAM";
        let mut item = collector.extract_bitvec("BRAM_DATA", "BRAM", "INIT", "");
        for bit in &mut item.bits {
            assert_eq!(bit.tile, 0);
            bit.tile = 2;
        }
        collector.tiledb.insert(tile, bel, "INIT", item);
        if edev.chip.kind.is_ice40() {
            collector.collect_bit(tile, bel, "ENABLE", "");
            collector.collect_bit(tile, bel, "CASCADE_IN_WADDR", "");
            collector.collect_bit(tile, bel, "CASCADE_IN_RADDR", "");
            collector.collect_bit(tile, bel, "CASCADE_OUT_WADDR", "");
            collector.collect_bit(tile, bel, "CASCADE_OUT_RADDR", "");
            collector.collect_enum(tile, bel, "READ_MODE", &["0", "1", "2", "3"]);
            collector.collect_enum(tile, bel, "WRITE_MODE", &["0", "1", "2", "3"]);
        }
    }
    for tile in ["IO.W", "IO.E", "IO.S", "IO.N"] {
        if matches!(tile, "IO.W" | "IO.E") && !edev.chip.kind.has_io_we() {
            continue;
        }
        collector.collect_bit(tile, "INT", "INV.IMUX.IO.ICLK", "");
        collector.collect_bit(tile, "INT", "INV.IMUX.IO.OCLK", "");
        for io in 0..2 {
            let bel = &format!("IO{io}");
            collector.collect_bitvec(tile, bel, "PIN_TYPE", "");
            if edev.chip.kind.is_ultra() {
                collector.collect_bit(tile, bel, "OUTPUT_ENABLE", "");
            }
        }
        let has_lvds = if edev.chip.kind == ChipKind::Ice65L01 {
            false
        } else if edev.chip.kind.has_actual_io_we() {
            tile == "IO.W"
        } else if edev.chip.kind == ChipKind::Ice40R04 {
            tile == "IO.N"
        } else {
            true
        };
        if has_lvds && !edev.chip.kind.is_ice65() {
            collector.collect_bit_wide(tile, "IO", "LVDS_INPUT", "");
        }
    }

    if edev.chip.kind.is_ultra() {
        let tile = "TRIM";
        let bel = "LFOSC";
        collector.collect_bit(tile, bel, "TRIM_FABRIC", "");
        let bel = "HFOSC";
        collector.collect_bit(tile, bel, "TRIM_FABRIC", "");
        collector.collect_bitvec(tile, bel, "CLKHF_DIV", "");
        let bel = "LED_DRV_CUR";
        collector.collect_bit(tile, bel, "TRIM_FABRIC", "");
        if edev.chip.kind == ChipKind::Ice40T04 {
            let tile = "LED_DRV_CUR";
            let bel = "LED_DRV_CUR";
            collector.collect_bit(tile, bel, "ENABLE", "");
            let tile = "RGB_DRV";
            let bel = "RGB_DRV";
            collector.collect_bit(tile, bel, "ENABLE", "");
            for attr in ["RGB0_CURRENT", "RGB1_CURRENT", "RGB2_CURRENT"] {
                collector.collect_bitvec(tile, bel, attr, "");
            }
            let tile = "IR_DRV";
            let bel = "IR_DRV";
            let mut diffs = collector.state.get_diffs(tile, bel, "IR_CURRENT", "");
            let en = diffs[0].split_bits_by(|bit| bit.frame == 5);
            collector
                .tiledb
                .insert(tile, bel, "IR_CURRENT", xlat_bitvec(diffs));
            collector.tiledb.insert(tile, bel, "ENABLE", xlat_bit(en));
        } else {
            let tile = "RGBA_DRV";
            let bel = "RGBA_DRV";
            collector.collect_bit(tile, bel, "ENABLE", "");
            collector.collect_bit(tile, bel, "CURRENT_MODE", "");
            for attr in ["RGB0_CURRENT", "RGB1_CURRENT", "RGB2_CURRENT"] {
                collector.collect_bitvec(tile, bel, attr, "");
            }
            if edev.chip.kind == ChipKind::Ice40T01 {
                let tile = "IR500_DRV";
                let bel = "RGBA_DRV";
                collector.collect_bit(tile, bel, "ENABLE", "");
                let bel = "IR500_DRV";
                collector.collect_bit(tile, bel, "ENABLE", "");
                collector.collect_bit(tile, bel, "CURRENT_MODE", "");
                let bel = "IR400_DRV";
                collector.collect_bit(tile, bel, "ENABLE", "");
                collector.collect_bitvec(tile, bel, "IR400_CURRENT", "");
                let bel = "BARCODE_DRV";
                collector.collect_bit(tile, bel, "ENABLE", "");
                collector.collect_bitvec(tile, bel, "BARCODE_CURRENT", "");
            }
        }
    }
    if edev.chip.kind == ChipKind::Ice40T05 {
        let tile = "SPRAM";
        for bel in ["SPRAM0", "SPRAM1"] {
            collector.collect_bit(tile, bel, "ENABLE", "");
        }
    }

    {
        let tile = "GBOUT";
        let bel = "GBOUT";
        for i in 0..8 {
            collector.collect_enum_default(
                tile,
                bel,
                &format!("MUX.GLOBAL.{i}"),
                &["IO"],
                "FABRIC",
            );
        }
    }

    if edev.chip.kind != ChipKind::Ice40T04 {
        let tile = "SPEED";
        let bel = "SPEED";
        collector.collect_enum(tile, bel, "SPEED", &["LOW", "MEDIUM", "HIGH"]);
    }

    for (feat, data) in &state.features {
        println!(
            "uncollected: {} {} {} {}: {:?}",
            feat.tile, feat.bel, feat.attr, feat.val, data.diffs
        );
    }

    tiledb
}
