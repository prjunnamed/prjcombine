use std::collections::{BTreeMap, HashMap, HashSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{SwitchBox, SwitchBoxItem, TileClassId},
    dir::{Dir, DirV},
    grid::{ColId, EdgeIoCoord, RowId, TileIobId},
};
use prjcombine_re_fpga_hammer::{
    Collector, Diff, DiffKey, FeatureData, FeatureId, OcdMode, State, extract_bitvec_val_part,
    xlat_bit, xlat_bitvec, xlat_enum, xlat_enum_ocd,
};
use prjcombine_re_harvester::Harvester;
use prjcombine_siliconblue::{
    chip::{ChipKind, SpecialTileKey},
    defs,
    expanded::{BitOwner, ExpandedDevice},
};
use prjcombine_types::{
    bits,
    bsdata::{BitRectId, BsData, TileBit, TileItem},
};

pub fn collect_iob(
    edev: &ExpandedDevice,
    harvester: &mut Harvester<BitOwner>,
) -> BTreeMap<EdgeIoCoord, EdgeIoCoord> {
    if edev.chip.kind == ChipKind::Ice40P01 {
        for anchor in [
            EdgeIoCoord::W(RowId::from_idx(2), TileIobId::from_idx(0)),
            EdgeIoCoord::W(RowId::from_idx(2), TileIobId::from_idx(1)),
            EdgeIoCoord::E(RowId::from_idx(2), TileIobId::from_idx(0)),
            EdgeIoCoord::E(RowId::from_idx(2), TileIobId::from_idx(1)),
            EdgeIoCoord::S(ColId::from_idx(1), TileIobId::from_idx(0)),
            EdgeIoCoord::S(ColId::from_idx(1), TileIobId::from_idx(1)),
            EdgeIoCoord::N(ColId::from_idx(1), TileIobId::from_idx(0)),
            EdgeIoCoord::N(ColId::from_idx(1), TileIobId::from_idx(1)),
        ] {
            for attrval in ["IBUF_ENABLE:BIT0", "PULLUP:DISABLE"] {
                let bits =
                    &harvester.known_global[&DiffKey::GlobalLegacy(format!("{anchor}:{attrval}"))];
                let owner = bits.keys().next().unwrap().0;
                let bits = BTreeMap::from_iter(bits.iter().map(|(&bit, &val)| {
                    let (bit_owner, frame, bit) = bit;
                    assert_eq!(bit_owner, owner);
                    (
                        TileBit {
                            rect: BitRectId::from_idx(0),
                            frame,
                            bit,
                        },
                        val,
                    )
                }));
                let edge = anchor.edge();
                let iob = anchor.iob();
                let tcid = edev.chip.kind.tile_class_iob(edge).unwrap();
                let tcls = edev.db.tile_classes.key(tcid);
                let (attr, val) = attrval.split_once(':').unwrap();
                harvester.force_tiled(
                    DiffKey::Legacy(FeatureId {
                        tile: tcls.to_string(),
                        bel: format!("IOB{iob:#}"),
                        attr: attr.to_string(),
                        val: val.to_string(),
                    }),
                    bits,
                );
            }
        }
        let mut res = BTreeMap::new();
        for &io in edev.chip.io_iob.keys() {
            let mut iob_loc = None;
            'attrs: for attrval in ["IBUF_ENABLE:BIT0", "PULLUP:DISABLE"] {
                let (attr, val) = attrval.split_once(':').unwrap();
                let bits = &harvester
                    .known_global
                    .remove(&DiffKey::GlobalLegacy(format!("{io}:{attrval}")))
                    .unwrap();
                let owner = bits.keys().next().unwrap().0;
                let bits = BTreeMap::from_iter(bits.iter().map(|(&bit, &val)| {
                    let (bit_owner, frame, bit) = bit;
                    assert_eq!(bit_owner, owner);
                    (
                        TileBit {
                            rect: BitRectId::from_idx(0),
                            frame,
                            bit,
                        },
                        val,
                    )
                }));
                let edge = io.edge();
                let BitOwner::Main(col, row) = owner else {
                    unreachable!()
                };
                for iob in 0..2 {
                    let iob = TileIobId::from_idx(iob);
                    let tcid = edev.chip.kind.tile_class_iob(edge).unwrap();
                    let tcls = edev.db.tile_classes.key(tcid);
                    if harvester.known_tiled[&DiffKey::Legacy(FeatureId {
                        tile: tcls.to_string(),
                        bel: format!("IOB{iob:#}"),
                        attr: attr.to_string(),
                        val: val.to_string(),
                    })] == bits
                    {
                        let loc = match edge {
                            Dir::W => {
                                assert_eq!(col, edev.chip.col_w());
                                EdgeIoCoord::W(row, iob)
                            }
                            Dir::E => {
                                assert_eq!(col, edev.chip.col_e());
                                EdgeIoCoord::E(row, iob)
                            }
                            Dir::S => {
                                assert_eq!(row, edev.chip.row_s());
                                EdgeIoCoord::S(col, iob)
                            }
                            Dir::N => {
                                assert_eq!(row, edev.chip.row_n());
                                EdgeIoCoord::N(col, iob)
                            }
                        };
                        if let Some(iob_loc) = iob_loc {
                            assert_eq!(iob_loc, loc);
                        } else {
                            iob_loc = Some(loc);
                        }
                        continue 'attrs;
                    }
                }
                panic!("can't deal with {io} {attrval}: {owner:?} {bits:?}");
            }
            res.insert(io, iob_loc.unwrap());
        }
        res
    } else {
        edev.chip.io_iob.clone()
    }
}

pub fn collect(
    edev: &ExpandedDevice,
    sbs: &BTreeMap<TileClassId, SwitchBox>,
    harvester: &Harvester<BitOwner>,
) -> BsData {
    let mut tiledb = BsData::new();
    let mut state = State::new();
    let mut bitvec_diffs: BTreeMap<DiffKey, BTreeMap<usize, Diff>> = BTreeMap::new();
    for (key, bits) in &harvester.known_global {
        println!("unhandled global: {key:?}: {bits:?}");
    }
    for (key, bits) in &harvester.known_tiled {
        let diff = Diff {
            bits: HashMap::from_iter(bits.iter().map(|(&k, &v)| (k, v))),
        };
        if let DiffKey::Legacy(id) = key
            && let Some(idx) = id.val.strip_prefix("BIT")
        {
            let key = DiffKey::Legacy(FeatureId {
                val: "".to_string(),
                ..id.clone()
            });
            let idx: usize = idx.parse().unwrap();
            bitvec_diffs.entry(key).or_default().insert(idx, diff);
        } else {
            state.features.insert(
                key.clone(),
                FeatureData {
                    diffs: vec![diff],
                    fuzzers: vec![],
                },
            );
        }
    }
    for (key, mut diffs) in bitvec_diffs {
        let diffs = Vec::from_iter((0..diffs.len()).map(|idx| diffs.remove(&idx).unwrap()));
        state.features.insert(
            key,
            FeatureData {
                diffs,
                fuzzers: vec![],
            },
        );
    }
    let mut collector = Collector {
        state: &mut state,
        tiledb: &mut tiledb,
    };

    for (&tcid, tile_sb) in sbs {
        let tile = edev.db.tile_classes.key(tcid);
        let bel = "INT";
        if !tile.starts_with("IO") {
            collector.collect_bit(tile, bel, "INV.IMUX_CLK_OPTINV", "");
        }
        let mut gout_mux = BTreeMap::new();
        for item in &tile_sb.items {
            if let SwitchBoxItem::Mux(mux) = item
                && defs::wires::GLOBAL_OUT.contains(mux.dst.wire)
            {
                gout_mux.insert(mux.dst, mux);
            }
        }
        for item in &tile_sb.items {
            match item {
                SwitchBoxItem::Mux(mux) => {
                    if gout_mux.contains_key(&mux.dst) {
                        continue;
                    }
                    let wtn = edev.db.wires.key(mux.dst.wire);
                    let mux_name = format!("MUX.{wtn}");
                    let mut values = vec![];
                    let mut diffs = vec![];
                    if tcid == edev.chip.kind.tile_class_plb()
                        && defs::wires::IMUX_LC_I3.contains(mux.dst.wire)
                    {
                        values.push("CI");
                        diffs.push(("CI", collector.state.get_diff(tile, bel, &mux_name, "CI")));
                    }
                    diffs.push(("NONE", Diff::default()));
                    for &wf in mux.src.keys() {
                        if !gout_mux.contains_key(&wf) {
                            let wfn = edev.db.wires.key(wf.wire);
                            values.push(wfn);
                            diffs.push((wfn, collector.state.get_diff(tile, bel, &mux_name, wfn)));
                        }
                    }
                    for &wg in mux.src.keys() {
                        if let Some(gmux) = gout_mux.get(&wg) {
                            let wgn = edev.db.wires.key(gmux.dst.wire);
                            let mut bits_nog2l = HashSet::new();
                            for (_, diff) in &diffs {
                                for &bit in diff.bits.keys() {
                                    bits_nog2l.insert(bit);
                                }
                            }
                            let mut diffs_gout = vec![];
                            for &wf in gmux.src.keys() {
                                let wfn = edev.db.wires.key(wf.wire).as_str();
                                let mut diff_gout =
                                    collector.state.get_diff(tile, bel, &mux_name, wfn);
                                let diff = diff_gout.split_bits(&bits_nog2l);
                                diffs_gout.push((wfn, diff_gout));
                                diffs.push((wgn, diff));
                            }
                            if !diffs_gout.is_empty() {
                                diffs_gout.push(("NONE", Diff::default()));
                                collector.tiledb.insert(
                                    tile,
                                    bel,
                                    format!("MUX.{wgn}"),
                                    xlat_enum_ocd(diffs_gout, OcdMode::Mux),
                                );
                            }
                        }
                    }
                    collector.tiledb.insert(
                        tile,
                        bel,
                        &mux_name,
                        xlat_enum_ocd(diffs, OcdMode::Mux),
                    );
                }
                SwitchBoxItem::ProgBuf(buf) => {
                    let wtn = edev.db.wires.key(buf.dst.wire);
                    let wfn = edev.db.wires.key(buf.src.wire);
                    let mux_name = format!("MUX.{wtn}");
                    let item = collector.extract_bit(tile, bel, &mux_name, wfn);
                    collector
                        .tiledb
                        .insert(tile, bel, format!("BUF.{wtn}.{wfn}"), item);
                }
                SwitchBoxItem::ProgInv(_) => (),
                _ => unreachable!(),
            }
        }
    }
    if let Some(tcid) = edev.chip.kind.tile_class_colbuf() {
        let tile = edev.db.tile_classes.key(tcid);
        for i in 0..8 {
            collector.collect_bit(tile, "COLBUF", &format!("GLOBAL.{i}"), "");
            if edev.chip.kind.has_ioi_we() {
                collector.collect_bit("COLBUF_IO_W", "COLBUF", &format!("GLOBAL.{i}"), "");
                collector.collect_bit("COLBUF_IO_E", "COLBUF", &format!("GLOBAL.{i}"), "");
            }
        }
    }
    for lc in 0..8 {
        let tcid = edev.chip.kind.tile_class_plb();
        let tile = edev.db.tile_classes.key(tcid);
        let bel = &format!("LC{lc}");
        if edev.chip.kind.is_ice40() {
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
        let tcid = edev.chip.kind.tile_class_bram();
        let tile = edev.db.tile_classes.key(tcid);
        let bel = "BRAM";
        let mut item = collector.extract_bitvec("BRAM_DATA", "BRAM", "INIT", "");
        for bit in &mut item.bits {
            assert_eq!(bit.rect.to_idx(), 0);
            bit.rect = BitRectId::from_idx(2);
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
    for edge in Dir::DIRS {
        let Some(tcid) = edev.chip.kind.tile_class_ioi(edge) else {
            continue;
        };
        let tile = edev.db.tile_classes.key(tcid);
        collector.collect_bit(tile, "INT", "INV.IMUX_IO_ICLK_OPTINV", "");
        collector.collect_bit(tile, "INT", "INV.IMUX_IO_OCLK_OPTINV", "");
        for io in 0..2 {
            let bel = &format!("IO{io}");
            collector.collect_bitvec(tile, bel, "PIN_TYPE", "");
            if edev.chip.kind.is_ultra() {
                collector.collect_bit(tile, bel, "OUTPUT_ENABLE", "");
            }
        }
        let Some(tcid) = edev.chip.kind.tile_class_iob(edge) else {
            continue;
        };
        let tile = edev.db.tile_classes.key(tcid);
        for iob in 0..2 {
            let bel = &format!("IOB{iob}");
            if edev.chip.kind.is_ice40() || (edev.chip.kind.has_vref() && edge == Dir::W) {
                collector.collect_bit(tile, bel, "IBUF_ENABLE", "");
            }
            if edev.chip.kind.is_ultra()
                && !(edge == Dir::N && edev.chip.kind == ChipKind::Ice40T01)
            {
                collector.collect_bit(tile, bel, "HARDIP_FABRIC_IN", "");
                collector.collect_bit(tile, bel, "HARDIP_DEDICATED_OUT", "");
                if (edev.chip.kind == ChipKind::Ice40T01 && iob == 0) || edge == Dir::N {
                    collector.collect_bit(tile, bel, "SDA_INPUT_DELAYED", "");
                    collector.collect_bit(tile, bel, "SDA_OUTPUT_DELAYED", "");
                }
            }
            if edge == Dir::W && edev.chip.kind.has_vref() {
                let diff_cmos = collector
                    .state
                    .peek_diff(tile, bel, "IOSTD", "SB_LVCMOS18_10")
                    .clone();
                let item = xlat_bit(diff_cmos.clone());
                collector.tiledb.insert(tile, bel, "CMOS_INPUT", item);
                let diff = collector
                    .state
                    .peek_diff(tile, bel, "IOSTD", "SB_SSTL18_FULL");
                let item = xlat_bit(diff.clone());
                collector.tiledb.insert(tile, bel, "IOSTD_MISC", item);
                let diff0 = collector
                    .state
                    .peek_diff(tile, bel, "IOSTD", "SB_LVCMOS18_8")
                    .combine(&!&diff_cmos);
                let diff1 = collector
                    .state
                    .peek_diff(tile, bel, "IOSTD", "SB_LVCMOS18_4")
                    .combine(&!&diff_cmos);
                let item = xlat_bitvec(vec![diff0, diff1]);
                collector.tiledb.insert(tile, bel, "DRIVE", item);
                for std in [
                    "SB_LVCMOS15_4",
                    "SB_LVCMOS15_2",
                    "SB_LVCMOS18_10",
                    "SB_LVCMOS18_8",
                    "SB_LVCMOS18_4",
                    "SB_LVCMOS18_2",
                    "SB_SSTL18_FULL",
                    "SB_SSTL18_HALF",
                    "SB_MDDR10",
                    "SB_MDDR8",
                    "SB_MDDR4",
                    "SB_MDDR2",
                    "SB_LVCMOS25_16",
                    "SB_LVCMOS25_12",
                    "SB_LVCMOS25_8",
                    "SB_LVCMOS25_4",
                    "SB_SSTL2_CLASS_2",
                    "SB_SSTL2_CLASS_1",
                    "SB_LVCMOS33_8",
                ] {
                    let mut diff = collector.state.get_diff(tile, bel, "IOSTD", std);
                    if !std.starts_with("SB_SSTL") {
                        diff = diff.combine(&!&diff_cmos);
                    }
                    let drive = extract_bitvec_val_part(
                        collector.tiledb.item(tile, bel, "DRIVE"),
                        &bits![0, 0],
                        &mut diff,
                    );
                    collector
                        .tiledb
                        .insert_misc_data(format!("IOSTD:DRIVE:{std}"), drive);
                    let misc = extract_bitvec_val_part(
                        collector.tiledb.item(tile, bel, "IOSTD_MISC"),
                        &bits![0],
                        &mut diff,
                    );
                    collector
                        .tiledb
                        .insert_misc_data(format!("IOSTD:IOSTD_MISC:{std}"), misc);
                    diff.assert_empty();
                }
            } else {
                let diff = collector.state.get_diff(tile, bel, "PULLUP", "DISABLE");
                let item = xlat_bit(!diff);
                collector.tiledb.insert(tile, bel, "PULLUP", item);
                if edev.chip.kind.has_multi_pullup() {
                    let diff = collector
                        .state
                        .get_diff(tile, bel, "WEAK_PULLUP", "DISABLE");
                    let item = xlat_bit(!diff);
                    collector.tiledb.insert(tile, bel, "WEAK_PULLUP", item);
                    for val in ["3P3K", "6P8K", "10K"] {
                        let item = collector.extract_bit(tile, bel, "PULLUP", val);
                        collector
                            .tiledb
                            .insert(tile, bel, format!("PULLUP_{val}"), item);
                    }
                }
            }
        }
        let has_lvds = if edev.chip.kind == ChipKind::Ice65L01 {
            false
        } else if edev.chip.kind.has_iob_we() {
            edge == Dir::W
        } else if edev.chip.kind == ChipKind::Ice40R04 {
            edge == Dir::N
        } else {
            true
        };
        if has_lvds {
            if !edev.chip.kind.is_ice65() {
                collector.collect_bit_wide(tile, "IOB0", "LVDS_INPUT", "");
            } else {
                for std in ["SB_LVDS_INPUT", "SB_SUBLVDS_INPUT"] {
                    let mut diff = collector.state.get_diff(tile, "IOB0", "IOSTD", std);
                    for bel in ["IOB0", "IOB1"] {
                        let misc = extract_bitvec_val_part(
                            collector.tiledb.item(tile, bel, "IOSTD_MISC"),
                            &bits![0],
                            &mut diff,
                        );
                        collector
                            .tiledb
                            .insert_misc_data(format!("IOSTD:IOSTD_MISC:{std}"), misc);
                    }
                    let item = xlat_bit(diff);
                    collector.tiledb.insert(tile, "IOB0", "LVDS_INPUT", item);
                }
            }
        }
        let mut has_latch_global_out = edev.chip.kind.has_latch_global_out();
        if edge == Dir::S
            && (edev
                .chip
                .special_tiles
                .contains_key(&SpecialTileKey::Pll(DirV::S))
                || edev
                    .chip
                    .special_tiles
                    .contains_key(&SpecialTileKey::PllStub(DirV::S)))
            && edev.chip.kind.has_iob_we()
        {
            has_latch_global_out = false;
        }
        if edge == Dir::N
            && (edev
                .chip
                .special_tiles
                .contains_key(&SpecialTileKey::Pll(DirV::N))
                || edev
                    .chip
                    .special_tiles
                    .contains_key(&SpecialTileKey::PllStub(DirV::N)))
        {
            has_latch_global_out = false;
        }
        if edev.chip.kind == ChipKind::Ice40P01 {
            has_latch_global_out = true;
        }
        if has_latch_global_out {
            collector.collect_bit(tile, "IOB", "LATCH_GLOBAL_OUT", "");
        }
        if edev.chip.kind == ChipKind::Ice40R04 {
            collector.collect_bit(tile, "IOB", "HARDIP_FABRIC_IN", "");
            collector.collect_bit(tile, "IOB", "HARDIP_DEDICATED_OUT", "");
        }
    }
    for side in [DirV::S, DirV::N] {
        let key = SpecialTileKey::Pll(side);
        if edev.chip.special_tiles.contains_key(&key) {
            let tcid = key.tile_class(edev.chip.kind);
            let tile = edev.db.tile_classes.key(tcid);
            let bel = "PLL";
            if edev.chip.kind.is_ice65() {
                for (attr, vals, default) in [
                    (
                        "MODE",
                        ["SB_PLL_CORE", "SB_PLL_PAD", "SB_PLL_2_PAD"].as_slice(),
                        Some("NONE"),
                    ),
                    (
                        "FEEDBACK_PATH",
                        ["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"].as_slice(),
                        None,
                    ),
                    (
                        "DELAY_ADJUSTMENT_MODE",
                        ["DYNAMIC", "FIXED"].as_slice(),
                        None,
                    ),
                    (
                        "PLLOUT_PHASE",
                        ["NONE", "0deg", "90deg", "180deg", "270deg"].as_slice(),
                        None,
                    ),
                ] {
                    if let Some(default) = default {
                        collector.collect_enum_default(tile, bel, attr, vals, default);
                    } else {
                        collector.collect_enum(tile, bel, attr, vals);
                    }
                }
                for attr in [
                    "FIXED_DELAY_ADJUSTMENT",
                    "DIVR",
                    "DIVF",
                    "DIVQ",
                    "FILTER_RANGE",
                    "TEST_MODE",
                    "LATCH_GLOBAL_OUT_A",
                    "LATCH_GLOBAL_OUT_B",
                ] {
                    collector.collect_bitvec(tile, bel, attr, "");
                }
            } else {
                for (attr, vals, default) in [
                    (
                        "MODE",
                        [
                            "SB_PLL40_CORE",
                            "SB_PLL40_PAD",
                            "SB_PLL40_2_PAD",
                            "SB_PLL40_2F_CORE",
                            "SB_PLL40_2F_PAD",
                        ]
                        .as_slice(),
                        Some("NONE"),
                    ),
                    (
                        "FEEDBACK_PATH",
                        ["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"].as_slice(),
                        None,
                    ),
                    (
                        "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                        ["DYNAMIC", "FIXED"].as_slice(),
                        None,
                    ),
                    (
                        "DELAY_ADJUSTMENT_MODE_RELATIVE",
                        ["DYNAMIC", "FIXED"].as_slice(),
                        None,
                    ),
                    (
                        "PLLOUT_SELECT_PORTA",
                        ["GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"].as_slice(),
                        Some("GENCLK"),
                    ),
                    (
                        "PLLOUT_SELECT_PORTB",
                        ["GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"].as_slice(),
                        Some("GENCLK"),
                    ),
                ] {
                    if let Some(default) = default {
                        collector.collect_enum_default(tile, bel, attr, vals, default);
                    } else {
                        collector.collect_enum(tile, bel, attr, vals);
                    }
                }
                for attr in [
                    "SHIFTREG_DIV_MODE",
                    "FDA_FEEDBACK",
                    "FDA_RELATIVE",
                    "DIVR",
                    "DIVF",
                    "DIVQ",
                    "FILTER_RANGE",
                    "TEST_MODE",
                    "LATCH_GLOBAL_OUT_A",
                    "LATCH_GLOBAL_OUT_B",
                ] {
                    if attr.starts_with("LATCH_GLOBAL_OUT") && edev.chip.kind == ChipKind::Ice40P01
                    {
                        continue;
                    }
                    collector.collect_bitvec(tile, bel, attr, "");
                }
            }
        }
        let key = SpecialTileKey::PllStub(side);
        if edev.chip.special_tiles.contains_key(&key) {
            let tcid = key.tile_class(edev.chip.kind);
            let tile = edev.db.tile_classes.key(tcid);
            let bel = "PLL";
            for attr in ["LATCH_GLOBAL_OUT_A", "LATCH_GLOBAL_OUT_B"] {
                collector.collect_bitvec(tile, bel, attr, "");
            }
        }
    }

    if edev.chip.kind.is_ultra() {
        let tcid = SpecialTileKey::Trim.tile_class(edev.chip.kind);
        let tile = edev.db.tile_classes.key(tcid);
        let bel = "LFOSC";
        collector.collect_bit(tile, bel, "TRIM_FABRIC", "");
        let bel = "HFOSC";
        collector.collect_bit(tile, bel, "TRIM_FABRIC", "");
        collector.collect_bitvec(tile, bel, "CLKHF_DIV", "");
        let bel = "LED_DRV_CUR";
        collector.collect_bit(tile, bel, "TRIM_FABRIC", "");
        if edev.chip.kind == ChipKind::Ice40T04 {
            let tile = "LED_DRV_CUR_T04";
            let bel = "LED_DRV_CUR";
            collector.collect_bit(tile, bel, "ENABLE", "");
            let tcid = SpecialTileKey::RgbDrv.tile_class(edev.chip.kind);
            let tile = edev.db.tile_classes.key(tcid);
            let bel = "RGB_DRV";
            collector.collect_bit(tile, bel, "ENABLE", "");
            for attr in ["RGB0_CURRENT", "RGB1_CURRENT", "RGB2_CURRENT"] {
                collector.collect_bitvec(tile, bel, attr, "");
            }
            let tile = "IR_DRV";
            let bel = "IR_DRV";
            let mut diffs = collector.state.get_diffs(tile, bel, "IR_CURRENT", "");
            let en = diffs[0].split_bits_by(|bit| bit.frame.to_idx() == 5);
            collector
                .tiledb
                .insert(tile, bel, "IR_CURRENT", xlat_bitvec(diffs));
            collector.tiledb.insert(tile, bel, "ENABLE", xlat_bit(en));
        } else {
            let tcid = SpecialTileKey::RgbDrv.tile_class(edev.chip.kind);
            let tile = edev.db.tile_classes.key(tcid);
            let bel = "RGB_DRV";
            collector.collect_bit(tile, bel, "ENABLE", "");
            collector.collect_bit(tile, bel, "CURRENT_MODE", "");
            for attr in ["RGB0_CURRENT", "RGB1_CURRENT", "RGB2_CURRENT"] {
                collector.collect_bitvec(tile, bel, attr, "");
            }
            if edev.chip.kind == ChipKind::Ice40T01 {
                let tile = "IR500_DRV";
                let bel = "RGB_DRV";
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
    if matches!(edev.chip.kind, ChipKind::Ice40T04 | ChipKind::Ice40T05) {
        for tile in ["MAC16", "MAC16_TRIM"] {
            if tile == "MAC16_TRIM" && edev.chip.kind != ChipKind::Ice40T05 {
                continue;
            }
            let bel = "MAC16";
            for attr in [
                "A_REG",
                "B_REG",
                "C_REG",
                "D_REG",
                "TOP_8x8_MULT_REG",
                "BOT_8x8_MULT_REG",
                "PIPELINE_16x16_MULT_REG1",
                "PIPELINE_16x16_MULT_REG2",
                "TOPOUTPUT_SELECT",
                "BOTOUTPUT_SELECT",
                "TOPADDSUB_LOWERINPUT",
                "BOTADDSUB_LOWERINPUT",
                "TOPADDSUB_UPPERINPUT",
                "BOTADDSUB_UPPERINPUT",
                "TOPADDSUB_CARRYSELECT",
                "BOTADDSUB_CARRYSELECT",
                "MODE_8x8",
                "A_SIGNED",
                "B_SIGNED",
            ] {
                collector.collect_bitvec(tile, bel, attr, "");
            }
        }
    }
    if edev.chip.kind == ChipKind::Ice40T05 {
        let tile = "SPRAM";
        for bel in ["SPRAM0", "SPRAM1"] {
            collector.collect_bit(tile, bel, "ENABLE", "");
        }
        let tile = "I3C";
        for bel in ["FILTER0", "FILTER1"] {
            collector.collect_bit_wide(tile, bel, "ENABLE", "");
        }
    }

    {
        let tcid = edev.chip.kind.tile_class_gb_root();
        let tile = edev.db.tile_classes.key(tcid);
        let bel = "GB_ROOT";
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

    {
        let tile = "SPEED";
        let bel = "SPEED";
        if edev.chip.kind == ChipKind::Ice40T04 {
            collector.tiledb.insert(
                tile,
                bel,
                "SPEED",
                xlat_enum(vec![
                    ("LOW", Diff::default()),
                    (
                        "MEDIUM",
                        Diff {
                            bits: HashMap::from_iter([(TileBit::new(0, 0, 0), true)]),
                        },
                    ),
                    (
                        "HIGH",
                        Diff {
                            bits: HashMap::from_iter([(TileBit::new(0, 0, 1), true)]),
                        },
                    ),
                ]),
            );
        } else {
            collector.collect_enum(tile, bel, "SPEED", &["LOW", "MEDIUM", "HIGH"]);
        }
    }
    {
        let tile = "CREG";
        let bel = "MISC";
        collector.tiledb.insert(
            tile,
            bel,
            "LOW_POWER",
            TileItem::from_bit(TileBit::new(0, 0, 0), true),
        );
        collector.tiledb.insert(
            tile,
            bel,
            "COLD_BOOT",
            TileItem::from_bit(TileBit::new(0, 0, 4), false),
        );
        collector.tiledb.insert(
            tile,
            bel,
            "WARM_BOOT",
            TileItem::from_bit(TileBit::new(0, 0, 5), false),
        );
    }

    for (key, data) in &state.features {
        println!("uncollected: {key:?}: {diffs:?}", diffs = data.diffs);
    }

    tiledb
}
