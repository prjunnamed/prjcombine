use std::collections::{BTreeMap, HashMap, HashSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem},
    dir::{Dir, DirV},
    grid::{BelCoord, CellCoord, DieId},
};
use prjcombine_re_fpga_hammer::{
    Collector, CollectorData, Diff, DiffKey, FeatureData, FeatureId, OcdMode, State,
    extract_bitvec_val_part_raw, xlat_bit_raw, xlat_bitvec_raw, xlat_enum_raw,
};
use prjcombine_re_harvester::Harvester;
use prjcombine_siliconblue::{
    chip::{ChipKind, SpecialTileKey},
    defs,
    expanded::{BitOwner, ExpandedDevice},
};
use prjcombine_types::{
    bimap::BiMap,
    bits,
    bsdata::{BitRectId, BsData, PolTileBit, RectBitId, TileBit},
};

use crate::specials;

pub fn collect_iob(
    edev: &ExpandedDevice,
    harvester: &mut Harvester<BitOwner>,
) -> BiMap<BelCoord, BelCoord> {
    #[derive(Debug)]
    enum Key {
        IbufEnable,
        PullupDisable,
    }

    if edev.chip.kind == ChipKind::Ice40P01 {
        let die = DieId::from_idx(0);
        for (col, row, idx) in [
            (edev.chip.col_w(), edev.chip.row_s() + 2, 0),
            (edev.chip.col_w(), edev.chip.row_s() + 2, 1),
            (edev.chip.col_e(), edev.chip.row_s() + 2, 0),
            (edev.chip.col_e(), edev.chip.row_s() + 2, 1),
            (edev.chip.col_w() + 1, edev.chip.row_s(), 0),
            (edev.chip.col_w() + 1, edev.chip.row_s(), 1),
            (edev.chip.col_w() + 1, edev.chip.row_n(), 0),
            (edev.chip.col_w() + 1, edev.chip.row_n(), 1),
        ] {
            let anchor = CellCoord::new(die, col, row).bel(defs::bslots::IOB[idx]);
            for attrval in [Key::IbufEnable, Key::PullupDisable] {
                let key = match attrval {
                    Key::IbufEnable => {
                        DiffKey::GlobalBelAttrBit(anchor, defs::bcls::IOB::IBUF_ENABLE, 0)
                    }
                    Key::PullupDisable => DiffKey::GlobalBelAttrSpecial(
                        anchor,
                        defs::bcls::IOB::PULLUP,
                        specials::DISABLE,
                    ),
                };
                let bits = &harvester.known_global[&key];
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
                let tcid = edev
                    .chip
                    .kind
                    .tile_class_iob(edev.chip.get_io_edge(anchor))
                    .unwrap();
                let key = match attrval {
                    Key::IbufEnable => {
                        DiffKey::BelAttrBit(tcid, anchor.slot, defs::bcls::IOB::IBUF_ENABLE, 0)
                    }
                    Key::PullupDisable => DiffKey::BelAttrSpecial(
                        tcid,
                        anchor.slot,
                        defs::bcls::IOB::PULLUP,
                        specials::DISABLE,
                    ),
                };
                harvester.force_tiled(key, bits);
            }
        }
        let mut res = BiMap::new();
        for (&ioi, &fake_iob) in &edev.chip.ioi_iob {
            let mut iob_loc = None;
            'attrs: for attrval in [Key::IbufEnable, Key::PullupDisable] {
                let key = match attrval {
                    Key::IbufEnable => {
                        DiffKey::GlobalBelAttrBit(fake_iob, defs::bcls::IOB::IBUF_ENABLE, 0)
                    }
                    Key::PullupDisable => DiffKey::GlobalBelAttrSpecial(
                        fake_iob,
                        defs::bcls::IOB::PULLUP,
                        specials::DISABLE,
                    ),
                };
                let bits = &harvester.known_global.remove(&key).unwrap();
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
                let edge = edev.chip.get_io_edge(ioi);
                let BitOwner::Main(col, row) = owner else {
                    unreachable!()
                };
                for slot in defs::bslots::IOB {
                    let tcid = edev.chip.kind.tile_class_iob(edge).unwrap();
                    let key = match attrval {
                        Key::IbufEnable => {
                            DiffKey::BelAttrBit(tcid, slot, defs::bcls::IOB::IBUF_ENABLE, 0)
                        }
                        Key::PullupDisable => DiffKey::BelAttrSpecial(
                            tcid,
                            slot,
                            defs::bcls::IOB::PULLUP,
                            specials::DISABLE,
                        ),
                    };
                    if harvester.known_tiled[&key] == bits {
                        let new_iob = CellCoord::new(DieId::from_idx(0), col, row).bel(slot);
                        if let Some(cur_iob) = iob_loc {
                            assert_eq!(cur_iob, new_iob);
                        } else {
                            iob_loc = Some(new_iob);
                        }
                        continue 'attrs;
                    }
                }
                panic!(
                    "can't deal with {ioi} {attrval:?}: {owner:?} {bits:?}",
                    ioi = ioi.to_string(edev.db)
                );
            }
            res.insert(ioi, iob_loc.unwrap());
        }
        res
    } else {
        edev.chip.ioi_iob.clone()
    }
}

pub fn collect(edev: &ExpandedDevice, harvester: &Harvester<BitOwner>) -> CollectorData {
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
    let mut collector = Collector::new(&mut state, &mut tiledb, edev.db);

    for (tcid, _, tcls) in &edev.db.tile_classes {
        if edev.tile_index[tcid].is_empty() {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let mut global_out_mux = BTreeMap::new();
            for item in &sb.items {
                if let SwitchBoxItem::Mux(mux) = item
                    && defs::wires::GLOBAL_OUT.contains(mux.dst.wire)
                {
                    global_out_mux.insert(mux.dst, mux);
                }
            }
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        if global_out_mux.contains_key(&mux.dst) {
                            continue;
                        }
                        let mut diffs = vec![];
                        let mut got_tie = false;
                        for &wf in mux.src.keys() {
                            if global_out_mux.contains_key(&wf) {
                                continue;
                            }
                            if matches!(wf.wire, defs::wires::TIE_0 | defs::wires::TIE_1) {
                                diffs.push((Some(wf), Diff::default()));
                                got_tie = true;
                            } else {
                                diffs.push((
                                    Some(wf),
                                    collector
                                        .state
                                        .get_diff_raw(&DiffKey::Routing(tcid, mux.dst, wf)),
                                ));
                            }
                        }
                        for &wg in mux.src.keys() {
                            if let Some(gmux) = global_out_mux.get(&wg) {
                                let mut bits_nog2l = HashSet::new();
                                for (_, diff) in &diffs {
                                    for &bit in diff.bits.keys() {
                                        bits_nog2l.insert(bit);
                                    }
                                }
                                let mut diffs_global_out = vec![];
                                for &wf in gmux.src.keys() {
                                    if wf.wire == defs::wires::TIE_0 {
                                        diffs_global_out.push((Some(wf), Diff::default()));
                                        continue;
                                    }
                                    let mut diff_global_out = collector
                                        .state
                                        .get_diff_raw(&DiffKey::Routing(tcid, mux.dst, wf));
                                    let diff = diff_global_out.split_bits(&bits_nog2l);
                                    diffs_global_out.push((Some(wf), diff_global_out));
                                    diffs.push((Some(wg), diff));
                                }
                                if !diffs_global_out.is_empty() {
                                    collector.data.mux.insert(
                                        (tcid, gmux.dst),
                                        xlat_enum_raw(diffs_global_out, OcdMode::Mux),
                                    );
                                }
                            }
                        }
                        if !got_tie && bslot == defs::bslots::INT {
                            diffs.push((None, Diff::default()));
                        }
                        collector
                            .data
                            .mux
                            .insert((tcid, mux.dst), xlat_enum_raw(diffs, OcdMode::Mux));
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        collector.collect_progbuf(tcid, buf.dst, buf.src);
                    }
                    SwitchBoxItem::ProgInv(inv) => {
                        collector.collect_inv(tcid, inv.dst);
                    }
                    SwitchBoxItem::PermaBuf(_) => (),
                    _ => unreachable!(),
                }
            }
        }
    }
    for lc in 0..8 {
        let tcid = edev.chip.kind.tile_class_plb();
        let bel = defs::bslots::LC[lc];
        if edev.chip.kind.is_ice40() {
            collector.collect_bel_attr(tcid, bel, defs::bcls::LC::LTIN_ENABLE);
        }
        for attr in [
            defs::bcls::LC::LUT_INIT,
            defs::bcls::LC::CARRY_ENABLE,
            defs::bcls::LC::FF_ENABLE,
            defs::bcls::LC::FF_SR_VALUE,
            defs::bcls::LC::FF_SR_ASYNC,
        ] {
            collector.collect_bel_attr(tcid, bel, attr);
        }
        if lc == 0 {
            collector.collect_bel_attr(tcid, bel, defs::bcls::LC::MUX_CI);
        }
    }
    if !edev.chip.cols_bram.is_empty() {
        let tcid = edev.chip.kind.tile_class_bram();
        let bel = defs::bslots::BRAM;
        collector.collect_bel_attr(tcid, bel, defs::bcls::BRAM::INIT);
        if edev.chip.kind.is_ice40() {
            for attr in [
                defs::bcls::BRAM::ENABLE,
                defs::bcls::BRAM::CASCADE_IN_WADDR,
                defs::bcls::BRAM::CASCADE_IN_RADDR,
                defs::bcls::BRAM::CASCADE_OUT_WADDR,
                defs::bcls::BRAM::CASCADE_OUT_RADDR,
                defs::bcls::BRAM::READ_MODE,
                defs::bcls::BRAM::WRITE_MODE,
            ] {
                collector.collect_bel_attr(tcid, bel, attr);
            }
        }
    }
    for edge in Dir::DIRS {
        let Some(tcid) = edev.chip.kind.tile_class_ioi(edge) else {
            continue;
        };
        for io in 0..2 {
            let bel = defs::bslots::IOI[io];
            collector.collect_bel_attr(tcid, bel, defs::bcls::IOI::PIN_TYPE);
            if edev.chip.kind.is_ultra() {
                collector.collect_bel_attr(tcid, bel, defs::bcls::IOI::OUTPUT_ENABLE);
            }
        }
        let Some(tcid) = edev.chip.kind.tile_class_iob(edge) else {
            continue;
        };
        for iob in 0..2 {
            let bel = defs::bslots::IOB[iob];
            if edev.chip.kind.is_ice40() || (edev.chip.kind.has_vref() && edge == Dir::W) {
                collector.collect_bel_attr(tcid, bel, defs::bcls::IOB::IBUF_ENABLE);
            }
            if edev.chip.kind.is_ultra()
                && !(edge == Dir::N && edev.chip.kind == ChipKind::Ice40T01)
            {
                collector.collect_bel_attr(tcid, bel, defs::bcls::IOB::HARDIP_FABRIC_IN);
                collector.collect_bel_attr(tcid, bel, defs::bcls::IOB::HARDIP_DEDICATED_OUT);
            }
            if edge == Dir::W && edev.chip.kind.has_vref() {
                let diff_cmos = collector
                    .state
                    .peek_diff_raw(&DiffKey::BelSpecialString(
                        tcid,
                        bel,
                        specials::IOSTD,
                        "SB_LVCMOS18_10".to_string(),
                    ))
                    .clone();
                let bit = xlat_bit_raw(diff_cmos.clone());
                collector.insert_bel_attr_bool(tcid, bel, defs::bcls::IOB::CMOS_INPUT, bit);
                let diff = collector.state.peek_diff_raw(&DiffKey::BelSpecialString(
                    tcid,
                    bel,
                    specials::IOSTD,
                    "SB_SSTL18_FULL".to_string(),
                ));
                let bit = xlat_bit_raw(diff.clone());
                collector.insert_bel_attr_bool(tcid, bel, defs::bcls::IOB::IOSTD_MISC, bit);
                let diff0 = collector
                    .state
                    .peek_diff_raw(&DiffKey::BelSpecialString(
                        tcid,
                        bel,
                        specials::IOSTD,
                        "SB_LVCMOS18_8".to_string(),
                    ))
                    .combine(&!&diff_cmos);
                let diff1 = collector
                    .state
                    .peek_diff_raw(&DiffKey::BelSpecialString(
                        tcid,
                        bel,
                        specials::IOSTD,
                        "SB_LVCMOS18_4".to_string(),
                    ))
                    .combine(&!&diff_cmos);
                let bits = xlat_bitvec_raw(vec![diff0, diff1]);
                collector.insert_bel_attr_bitvec(tcid, bel, defs::bcls::IOB::DRIVE, bits);
                let table = &edev.db.tables[defs::tables::IOSTD];
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
                    let mut diff = collector.state.get_diff_raw(&DiffKey::BelSpecialString(
                        tcid,
                        bel,
                        specials::IOSTD,
                        std.to_string(),
                    ));
                    if !std.starts_with("SB_SSTL") {
                        diff = diff.combine(&!&diff_cmos);
                    }
                    let drive = extract_bitvec_val_part_raw(
                        collector.bel_attr_bitvec(tcid, bel, defs::bcls::IOB::DRIVE),
                        &bits![0, 0],
                        &mut diff,
                    );
                    let rid = table.rows.get(std).unwrap().0;
                    collector.insert_table_bitvec(
                        defs::tables::IOSTD,
                        rid,
                        defs::tables::IOSTD::DRIVE,
                        drive,
                    );
                    let misc = extract_bitvec_val_part_raw(
                        collector.bel_attr_bitvec(tcid, bel, defs::bcls::IOB::IOSTD_MISC),
                        &bits![0],
                        &mut diff,
                    );
                    collector.insert_table_bitvec(
                        defs::tables::IOSTD,
                        rid,
                        defs::tables::IOSTD::IOSTD_MISC,
                        misc,
                    );
                    diff.assert_empty();
                }
            } else {
                let diff = collector.state.get_diff_raw(&DiffKey::BelAttrSpecial(
                    tcid,
                    bel,
                    defs::bcls::IOB::PULLUP,
                    specials::DISABLE,
                ));
                let bit = xlat_bit_raw(!diff);
                collector.insert_bel_attr_bool(tcid, bel, defs::bcls::IOB::PULLUP, bit);
                if edev.chip.kind.has_multi_pullup() {
                    let diff = collector.state.get_diff_raw(&DiffKey::BelAttrSpecial(
                        tcid,
                        bel,
                        defs::bcls::IOB::WEAK_PULLUP,
                        specials::DISABLE,
                    ));
                    let bit = xlat_bit_raw(!diff);
                    collector.insert_bel_attr_bool(tcid, bel, defs::bcls::IOB::WEAK_PULLUP, bit);
                    for attr in [
                        defs::bcls::IOB::PULLUP_3P3K,
                        defs::bcls::IOB::PULLUP_6P8K,
                        defs::bcls::IOB::PULLUP_10K,
                    ] {
                        collector.collect_bel_attr(tcid, bel, attr);
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
                collector.collect_bel_attr(
                    tcid,
                    defs::bslots::IOB_PAIR,
                    defs::bcls::IOB_PAIR::LVDS_INPUT,
                );
            } else {
                let table = &edev.db.tables[defs::tables::IOSTD];
                for std in ["SB_LVDS_INPUT", "SB_SUBLVDS_INPUT"] {
                    let mut diff = collector.state.get_diff_raw(&DiffKey::BelSpecialString(
                        tcid,
                        defs::bslots::IOB[0],
                        specials::IOSTD,
                        std.to_string(),
                    ));
                    for bel in defs::bslots::IOB {
                        let misc = extract_bitvec_val_part_raw(
                            collector.bel_attr_bitvec(tcid, bel, defs::bcls::IOB::IOSTD_MISC),
                            &bits![0],
                            &mut diff,
                        );
                        let rid = table.rows.get(std).unwrap().0;
                        collector.insert_table_bitvec(
                            defs::tables::IOSTD,
                            rid,
                            defs::tables::IOSTD::IOSTD_MISC,
                            misc,
                        );
                    }
                    let bit = xlat_bit_raw(diff);
                    collector.insert_bel_attr_bool(
                        tcid,
                        defs::bslots::IOB_PAIR,
                        defs::bcls::IOB_PAIR::LVDS_INPUT,
                        bit,
                    );
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
            collector.collect_bel_attr(
                tcid,
                defs::bslots::IOB_PAIR,
                defs::bcls::IOB_PAIR::LATCH_GLOBAL_OUT,
            );
        }
        if edev.chip.kind == ChipKind::Ice40R04 {
            for attr in [
                defs::bcls::IOB_PAIR::HARDIP_FABRIC_IN,
                defs::bcls::IOB_PAIR::HARDIP_DEDICATED_OUT,
            ] {
                collector.collect_bel_attr(tcid, defs::bslots::IOB_PAIR, attr);
            }
        }
        if edev.chip.kind.is_ultra() {
            let i2c_edge = if edev.chip.kind == ChipKind::Ice40T01 {
                Dir::S
            } else {
                Dir::N
            };
            if edge == i2c_edge {
                for attr in [
                    defs::bcls::IOB_PAIR::SDA_INPUT_DELAYED,
                    defs::bcls::IOB_PAIR::SDA_OUTPUT_DELAYED,
                ] {
                    collector.collect_bel_attr(tcid, defs::bslots::IOB_PAIR, attr);
                }
            }
        }
    }
    for side in [DirV::S, DirV::N] {
        let key = SpecialTileKey::Pll(side);
        if edev.chip.special_tiles.contains_key(&key) {
            let tcid = key.tile_class(edev.chip.kind);
            if edev.chip.kind.is_ice65() {
                let bslot = defs::bslots::PLL65;
                collector.collect_bel_attr_default(
                    tcid,
                    bslot,
                    defs::bcls::PLL65::MODE,
                    defs::enums::PLL65_MODE::NONE,
                );
                for attr in [
                    defs::bcls::PLL65::FIXED_DELAY_ADJUSTMENT,
                    defs::bcls::PLL65::DIVR,
                    defs::bcls::PLL65::DIVF,
                    defs::bcls::PLL65::DIVQ,
                    defs::bcls::PLL65::FILTER_RANGE,
                    defs::bcls::PLL65::TEST_MODE,
                    defs::bcls::PLL65::LATCH_GLOBAL_OUT_A,
                    defs::bcls::PLL65::LATCH_GLOBAL_OUT_B,
                    defs::bcls::PLL65::FEEDBACK_PATH,
                    defs::bcls::PLL65::PLLOUT_PHASE,
                ] {
                    collector.collect_bel_attr(tcid, bslot, attr);
                }
                let attr = defs::bcls::PLL65::DELAY_ADJUSTMENT_MODE_DYNAMIC;
                let mut diff = collector
                    .state
                    .get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, attr, 0));
                let fda = collector.bel_attr_bitvec(
                    tcid,
                    bslot,
                    defs::bcls::PLL65::FIXED_DELAY_ADJUSTMENT,
                );
                let damd = Vec::from_iter(fda.iter().map(|&bit| PolTileBit {
                    bit: TileBit {
                        bit: RectBitId::from_idx(bit.bit.bit.to_idx() ^ 1),
                        ..bit.bit
                    },
                    inv: bit.inv,
                }));
                diff.apply_bitvec_diff_raw(&damd, &bits![1; 4], &bits![0; 4]);
                diff.assert_empty();
                collector.insert_bel_attr_bitvec(tcid, bslot, attr, damd);
            } else {
                let bslot = defs::bslots::PLL40;
                collector.collect_bel_attr_default(
                    tcid,
                    bslot,
                    defs::bcls::PLL40::MODE,
                    defs::enums::PLL40_MODE::NONE,
                );
                for attr in [
                    defs::bcls::PLL40::PLLOUT_SELECT_PORTA,
                    defs::bcls::PLL40::PLLOUT_SELECT_PORTB,
                ] {
                    collector.collect_bel_attr_default(
                        tcid,
                        bslot,
                        attr,
                        defs::enums::PLL40_PLLOUT_SELECT::GENCLK,
                    );
                }
                for attr in [
                    defs::bcls::PLL40::SHIFTREG_DIV_MODE,
                    defs::bcls::PLL40::FDA_FEEDBACK,
                    defs::bcls::PLL40::FDA_RELATIVE,
                    defs::bcls::PLL40::DIVR,
                    defs::bcls::PLL40::DIVF,
                    defs::bcls::PLL40::DIVQ,
                    defs::bcls::PLL40::FILTER_RANGE,
                    defs::bcls::PLL40::TEST_MODE,
                    defs::bcls::PLL40::FEEDBACK_PATH,
                    defs::bcls::PLL40::DELAY_ADJUSTMENT_MODE_FEEDBACK,
                    defs::bcls::PLL40::DELAY_ADJUSTMENT_MODE_RELATIVE,
                ] {
                    collector.collect_bel_attr(tcid, bslot, attr);
                }
                if edev.chip.kind != ChipKind::Ice40P01 {
                    for attr in [
                        defs::bcls::PLL40::LATCH_GLOBAL_OUT_A,
                        defs::bcls::PLL40::LATCH_GLOBAL_OUT_B,
                    ] {
                        collector.collect_bel_attr(tcid, bslot, attr);
                    }
                }
            }
        }
        let key = SpecialTileKey::PllStub(side);
        if edev.chip.special_tiles.contains_key(&key) {
            let tcid = key.tile_class(edev.chip.kind);
            let bslot = defs::bslots::PLL40;
            for attr in [
                defs::bcls::PLL40::LATCH_GLOBAL_OUT_A,
                defs::bcls::PLL40::LATCH_GLOBAL_OUT_B,
            ] {
                collector.collect_bel_attr(tcid, bslot, attr);
            }
        }
    }

    if edev.chip.kind.is_ultra() {
        let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
        collector.collect_bel_attr(tcid, defs::bslots::LFOSC, defs::bcls::LFOSC::TRIM_FABRIC);
        collector.collect_bel_attr(tcid, defs::bslots::HFOSC, defs::bcls::HFOSC::TRIM_FABRIC);
        collector.collect_bel_attr(tcid, defs::bslots::HFOSC, defs::bcls::HFOSC::CLKHF_DIV);
        collector.collect_bel_attr(
            tcid,
            defs::bslots::LED_DRV_CUR,
            defs::bcls::LED_DRV_CUR::TRIM_FABRIC,
        );
        if edev.chip.kind == ChipKind::Ice40T04 {
            collector.collect_bel_attr(
                tcid,
                defs::bslots::LED_DRV_CUR,
                defs::bcls::LED_DRV_CUR::ENABLE,
            );
            for attr in [
                defs::bcls::RGB_DRV::ENABLE,
                defs::bcls::RGB_DRV::RGB0_CURRENT,
                defs::bcls::RGB_DRV::RGB1_CURRENT,
                defs::bcls::RGB_DRV::RGB2_CURRENT,
            ] {
                collector.collect_bel_attr(tcid, defs::bslots::RGB_DRV, attr);
            }
            let mut diffs = Vec::from_iter((0..10).map(|i| {
                collector.state.get_diff_raw(&DiffKey::BelAttrBit(
                    tcid,
                    defs::bslots::IR_DRV,
                    defs::bcls::IR_DRV::IR_CURRENT,
                    i,
                ))
            }));
            let en = diffs[0].split_bits_by(|bit| bit.frame.to_idx() == 5);
            collector.insert_bel_attr_bitvec(
                tcid,
                defs::bslots::IR_DRV,
                defs::bcls::IR_DRV::IR_CURRENT,
                xlat_bitvec_raw(diffs),
            );
            collector.insert_bel_attr_bool(
                tcid,
                defs::bslots::IR_DRV,
                defs::bcls::IR_DRV::ENABLE,
                xlat_bit_raw(en),
            );
        } else {
            for attr in [
                defs::bcls::RGB_DRV::ENABLE,
                defs::bcls::RGB_DRV::RGB0_CURRENT,
                defs::bcls::RGB_DRV::RGB1_CURRENT,
                defs::bcls::RGB_DRV::RGB2_CURRENT,
                defs::bcls::RGB_DRV::CURRENT_MODE,
            ] {
                if attr == defs::bcls::RGB_DRV::ENABLE && edev.chip.kind == ChipKind::Ice40T01 {
                    let mut diff = collector.state.get_diff_raw(&DiffKey::BelAttrBit(
                        tcid,
                        defs::bslots::RGB_DRV,
                        attr,
                        0,
                    ));
                    let led_drv_cur_en = diff.split_bits_by(|bit| bit.rect.to_idx() >= 3);
                    collector.insert_bel_attr_bool(
                        tcid,
                        defs::bslots::RGB_DRV,
                        attr,
                        xlat_bit_raw(diff),
                    );
                    collector.insert_bel_attr_bool(
                        tcid,
                        defs::bslots::LED_DRV_CUR,
                        defs::bcls::LED_DRV_CUR::RGB_ENABLE,
                        xlat_bit_raw(led_drv_cur_en),
                    );
                } else {
                    collector.collect_bel_attr(tcid, defs::bslots::RGB_DRV, attr);
                }
            }
            if edev.chip.kind == ChipKind::Ice40T01 {
                for attr in [
                    defs::bcls::IR500_DRV::BARCODE_ENABLE,
                    defs::bcls::IR500_DRV::BARCODE_CURRENT,
                    defs::bcls::IR500_DRV::IR400_ENABLE,
                    defs::bcls::IR500_DRV::IR400_CURRENT,
                    defs::bcls::IR500_DRV::IR500_ENABLE,
                    defs::bcls::IR500_DRV::CURRENT_MODE,
                ] {
                    collector.collect_bel_attr(tcid, defs::bslots::IR500_DRV, attr);
                }
            }
        }
    }
    if matches!(edev.chip.kind, ChipKind::Ice40T04 | ChipKind::Ice40T05) {
        for tcid in [defs::tcls::MAC16, defs::tcls::MAC16_TRIM] {
            if tcid == defs::tcls::MAC16_TRIM && edev.chip.kind != ChipKind::Ice40T05 {
                continue;
            }
            for attr in edev.db.bel_classes[defs::bcls::MAC16].attributes.ids() {
                collector.collect_bel_attr(tcid, defs::bslots::MAC16, attr);
            }
        }
    }
    if edev.chip.kind == ChipKind::Ice40T05 {
        for bslot in defs::bslots::SPRAM {
            collector.collect_bel_attr(defs::tcls::SPRAM, bslot, defs::bcls::SPRAM::ENABLE);
        }
        for bslot in defs::bslots::FILTER {
            let tcid = defs::tcls::MISC_T05;
            let aid = defs::bcls::FILTER::ENABLE;
            let diff = collector
                .state
                .get_diff_raw(&DiffKey::BelAttrBit(tcid, bslot, aid, 0));
            let mut bits = Vec::from_iter(
                diff.bits
                    .into_iter()
                    .map(|(bit, val)| PolTileBit { bit, inv: !val }),
            );
            bits.sort_by_key(|bit| bit.bit.frame.to_idx() ^ 1);
            collector.insert_bel_attr_bitvec(tcid, bslot, aid, bits);
        }
    }

    if edev.chip.kind != ChipKind::Ice40T04 {
        collector.collect_bel_attr(
            defs::tcls::GLOBALS,
            defs::bslots::GLOBAL_OPTIONS,
            defs::bcls::GLOBAL_OPTIONS::SPEED,
        );
    }

    let data = collector.data;

    for (key, data) in &state.features {
        println!("uncollected: {key:?}: {diffs:?}", diffs = data.diffs);
    }

    assert_eq!(tiledb, BsData::new());

    data
}
