use core::{fmt::Debug, hash::Hash};
use std::{
    collections::{BTreeSet, HashMap, btree_map, hash_map},
    error::Error,
    fs::File,
    path::Path,
};

use bincode::{Decode, Encode};
use prjcombine_entity::EntityPartVec;
use prjcombine_interconnect::db::{
    BelAttribute, BelAttributeId, BelInfo, BelInput, BelInputId, BelSlotId, ConnectorSlotId,
    DeviceDataId, IntDb, PolTileWireCoord, SwitchBoxItem, TableFieldId, TableId, TableRowId,
    TableValue, TileClassId, TileWireCoord,
};
use prjcombine_types::bsdata::{BsData, EnumData, PolTileBit};

#[allow(clippy::type_complexity)]
#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct CollectorData {
    pub bel_attrs: HashMap<(TileClassId, BelSlotId, BelAttributeId), BelAttribute>,
    pub bel_input_inv: HashMap<(TileClassId, BelSlotId, BelInputId), PolTileBit>,
    pub sb_inv: HashMap<(TileClassId, TileWireCoord), PolTileBit>,
    pub sb_buf: HashMap<(TileClassId, TileWireCoord, PolTileWireCoord), PolTileBit>,
    pub sb_pass: HashMap<(TileClassId, TileWireCoord, TileWireCoord), PolTileBit>,
    pub sb_bipass: HashMap<(TileClassId, TileWireCoord, TileWireCoord), PolTileBit>,
    pub sb_mux: HashMap<(TileClassId, TileWireCoord), EnumData<Option<PolTileWireCoord>>>,
    pub sb_delay: HashMap<(TileClassId, TileWireCoord), EnumData<usize>>,
    pub sb_bidi: HashMap<(TileClassId, ConnectorSlotId, TileWireCoord), PolTileBit>,
    pub sb_enable: HashMap<(TileClassId, TileWireCoord), Vec<PolTileBit>>,
    pub sb_pairmux:
        HashMap<(TileClassId, [TileWireCoord; 2]), EnumData<[Option<PolTileWireCoord>; 2]>>,
    pub tmux_group: HashMap<(TileClassId, BelSlotId), EnumData<Option<usize>>>,
    pub table_data: HashMap<(TableId, TableRowId, TableFieldId), TableValue>,
    pub device_data: HashMap<String, EntityPartVec<DeviceDataId, TableValue>>,
    pub bsdata: BsData,
}

impl CollectorData {
    pub fn insert_into(mut self, intdb: &mut IntDb, missing_ok: bool) {
        for ((tcid, bslot, aid), attr) in self.bel_attrs {
            let BelInfo::Bel(ref mut bel) = intdb.tile_classes[tcid].bels[bslot] else {
                unreachable!()
            };
            if bel.attributes.contains_id(aid) {
                assert_eq!(bel.attributes[aid], attr);
            } else {
                bel.attributes.insert(aid, attr);
            }
        }

        for ((tcid, bslot, pin), bit) in self.bel_input_inv {
            let BelInfo::Bel(ref mut bel) = intdb.tile_classes[tcid].bels[bslot] else {
                unreachable!()
            };
            match bel.inputs[pin] {
                BelInput::Fixed(ptwc) => {
                    bel.inputs[pin] = BelInput::Invertible(ptwc.tw, bit);
                }
                BelInput::Invertible(_, ref mut inp_bit) => {
                    *inp_bit = bit;
                }
            }
        }

        for (tcid, _, tcls) in &mut intdb.tile_classes {
            for (bslot, bel) in &mut tcls.bels {
                match bel {
                    BelInfo::SwitchBox(sbox) => {
                        for item in &mut sbox.items {
                            match item {
                                SwitchBoxItem::Mux(mux) => {
                                    let Some(edata) = self.sb_mux.remove(&(tcid, mux.dst)) else {
                                        if missing_ok {
                                            continue;
                                        }
                                        let dst = mux.dst;
                                        panic!(
                                            "can't find collect enum mux {tcname} {dst}",
                                            tcname = intdb.tile_classes.key(tcid),
                                            dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                                        )
                                    };
                                    mux.bits = edata.bits;
                                    let mut handled = BTreeSet::new();
                                    for (src, val) in edata.values {
                                        if let Some(src) = src {
                                            *mux.src.get_mut(&src).unwrap() = val;
                                            handled.insert(src);
                                        } else {
                                            mux.bits_off = Some(val);
                                        }
                                    }
                                    for src in mux.src.keys() {
                                        let src = *src;
                                        if !handled.contains(&src) {
                                            let dst = mux.dst;
                                            panic!(
                                                "can't find mux input {tcname} {dst} {src}",
                                                tcname = intdb.tile_classes.key(tcid),
                                                dst =
                                                    dst.to_string(intdb, &intdb.tile_classes[tcid]),
                                                src =
                                                    src.to_string(intdb, &intdb.tile_classes[tcid]),
                                            );
                                        }
                                    }
                                }
                                SwitchBoxItem::ProgBuf(buf) => {
                                    let Some(bit) = self.sb_buf.remove(&(tcid, buf.dst, buf.src))
                                    else {
                                        if missing_ok {
                                            continue;
                                        }
                                        let dst = buf.dst;
                                        let src = buf.src;
                                        panic!(
                                            "can't find collect bit progbuf {tcname} {dst} {src}",
                                            tcname = intdb.tile_classes.key(tcid),
                                            dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                                            src = src.to_string(intdb, &intdb.tile_classes[tcid])
                                        )
                                    };
                                    buf.bit = bit;
                                }
                                SwitchBoxItem::PermaBuf(_) => (),
                                SwitchBoxItem::Pass(pass) => {
                                    let Some(bit) =
                                        self.sb_pass.remove(&(tcid, pass.dst, pass.src))
                                    else {
                                        if missing_ok {
                                            continue;
                                        }
                                        let dst = pass.dst;
                                        let src = pass.src;
                                        panic!(
                                            "can't find collect bit pass {tcname} {dst} {src}",
                                            tcname = intdb.tile_classes.key(tcid),
                                            dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                                            src = src.to_string(intdb, &intdb.tile_classes[tcid])
                                        )
                                    };
                                    pass.bit = bit;
                                }
                                SwitchBoxItem::BiPass(pass) => {
                                    let Some(bit) = self.sb_bipass.remove(&(tcid, pass.a, pass.b))
                                    else {
                                        if missing_ok {
                                            continue;
                                        }
                                        let dst = pass.a;
                                        let src = pass.b;
                                        panic!(
                                            "can't find collect bit bipass {tcname} {dst} {src}",
                                            tcname = intdb.tile_classes.key(tcid),
                                            dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                                            src = src.to_string(intdb, &intdb.tile_classes[tcid])
                                        )
                                    };
                                    pass.bit = bit;
                                }
                                SwitchBoxItem::ProgInv(inv) => {
                                    let Some(bit) = self.sb_inv.remove(&(tcid, inv.dst)) else {
                                        if missing_ok {
                                            continue;
                                        }
                                        let twc = inv.dst;
                                        panic!(
                                            "can't find collect bit proginv {tcname} {wire}",
                                            tcname = intdb.tile_classes.key(tcid),
                                            wire = twc.to_string(intdb, &intdb.tile_classes[tcid])
                                        )
                                    };
                                    inv.bit = bit;
                                }
                                SwitchBoxItem::ProgDelay(delay) => {
                                    let Some(mut data) = self.sb_delay.remove(&(tcid, delay.dst))
                                    else {
                                        if missing_ok {
                                            continue;
                                        }
                                        let twc = delay.dst;
                                        panic!(
                                            "can't find collect progdelay {tcname} {wire}",
                                            tcname = intdb.tile_classes.key(tcid),
                                            wire = twc.to_string(intdb, &intdb.tile_classes[tcid])
                                        )
                                    };
                                    delay.bits = data.bits;
                                    for (i, val) in delay.steps.iter_mut().enumerate() {
                                        *val = data.values.remove(&i).unwrap();
                                    }
                                    assert!(data.values.is_empty());
                                }
                                SwitchBoxItem::Bidi(bidi) => {
                                    let Some(bit) =
                                        self.sb_bidi.remove(&(tcid, bidi.conn, bidi.wire))
                                    else {
                                        if missing_ok {
                                            continue;
                                        }
                                        let conn = bidi.conn;
                                        let twc = bidi.wire;
                                        panic!(
                                            "can't find collect bit bidi {tcname} {conn} {wire}",
                                            tcname = intdb.tile_classes.key(tcid),
                                            conn = intdb.conn_slots.key(conn),
                                            wire = twc.to_string(intdb, &intdb.tile_classes[tcid])
                                        )
                                    };
                                    bidi.bit_upstream = bit;
                                }
                                SwitchBoxItem::PairMux(mux) => {
                                    let Some(edata) = self.sb_pairmux.remove(&(tcid, mux.dst))
                                    else {
                                        if missing_ok {
                                            continue;
                                        }
                                        let dst = mux.dst;
                                        panic!(
                                            "can't find collect enum pair mux {tcname} {dst0} {dst1}",
                                            tcname = intdb.tile_classes.key(tcid),
                                            dst0 =
                                                dst[0].to_string(intdb, &intdb.tile_classes[tcid]),
                                            dst1 =
                                                dst[1].to_string(intdb, &intdb.tile_classes[tcid]),
                                        )
                                    };
                                    mux.bits = edata.bits;
                                    let mut handled = BTreeSet::new();
                                    for (src, val) in edata.values {
                                        *mux.src.get_mut(&src).unwrap() = val;
                                        handled.insert(src);
                                    }
                                    for src in mux.src.keys() {
                                        let src = *src;
                                        if !handled.contains(&src) {
                                            let dst = mux.dst;
                                            panic!(
                                                "can't find mux input {tcname} ({dst0}, {dst1}) ({src0}, {src1})",
                                                tcname = intdb.tile_classes.key(tcid),
                                                dst0 = dst[0]
                                                    .to_string(intdb, &intdb.tile_classes[tcid]),
                                                dst1 = dst[1]
                                                    .to_string(intdb, &intdb.tile_classes[tcid]),
                                                src0 = if let Some(src) = src[0] {
                                                    src.to_string(intdb, &intdb.tile_classes[tcid])
                                                } else {
                                                    "_".to_string()
                                                },
                                                src1 = if let Some(src) = src[0] {
                                                    src.to_string(intdb, &intdb.tile_classes[tcid])
                                                } else {
                                                    "_".to_string()
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    BelInfo::TestMux(tmux) => {
                        let Some(mut data) = self.tmux_group.remove(&(tcid, bslot)) else {
                            if missing_ok {
                                continue;
                            }
                            panic!(
                                "can't find collect group testmux {tcname} {bel}",
                                tcname = intdb.tile_classes.key(tcid),
                                bel = intdb.bel_slots.key(bslot),
                            )
                        };
                        tmux.bits = data.bits;
                        for (i, val) in tmux.groups.iter_mut().enumerate() {
                            *val = data.values.remove(&Some(i)).unwrap();
                        }
                        tmux.bits_primary = data.values.remove(&None).unwrap();
                        assert!(data.values.is_empty());
                    }
                    _ => (),
                }
            }
        }

        for ((tcid, dst), data) in self.sb_mux {
            println!(
                "uncollected enum: mux {tcls} {dst}: {data:?}",
                tcls = intdb.tile_classes.key(tcid),
                dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
            );
        }

        for ((tcid, dst, src), bit) in self.sb_buf {
            println!(
                "uncollected bit: progbuf {tcls} {dst} {src}: {bit}",
                tcls = intdb.tile_classes.key(tcid),
                dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                src = src.to_string(intdb, &intdb.tile_classes[tcid]),
                bit = intdb.tile_classes[tcid].dump_polbit(bit),
            );
        }

        for ((tcid, dst, src), bit) in self.sb_pass {
            println!(
                "uncollected bit: pass {tcls} {dst} {src}: {bit}",
                tcls = intdb.tile_classes.key(tcid),
                dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                src = src.to_string(intdb, &intdb.tile_classes[tcid]),
                bit = intdb.tile_classes[tcid].dump_polbit(bit),
            );
        }

        for ((tcid, dst, src), bit) in self.sb_bipass {
            println!(
                "uncollected bit: bipass {tcls} {dst} {src}: {bit}",
                tcls = intdb.tile_classes.key(tcid),
                dst = dst.to_string(intdb, &intdb.tile_classes[tcid]),
                src = src.to_string(intdb, &intdb.tile_classes[tcid]),
                bit = intdb.tile_classes[tcid].dump_polbit(bit),
            );
        }

        for ((tcid, twc), bit) in self.sb_inv {
            println!(
                "uncollected bit: proginv {tcls} {wire}: {bit}",
                tcls = intdb.tile_classes.key(tcid),
                wire = twc.to_string(intdb, &intdb.tile_classes[tcid]),
                bit = intdb.tile_classes[tcid].dump_polbit(bit),
            );
        }

        for ((tcid, twc), data) in self.sb_delay {
            println!(
                "uncollected enum: delay {tcls} {wire}: {data:?}",
                tcls = intdb.tile_classes.key(tcid),
                wire = twc.to_string(intdb, &intdb.tile_classes[tcid]),
            );
        }

        for ((tcid, bslot), data) in self.tmux_group {
            println!(
                "uncollected enum: delay {tcls} {bel}: {data:?}",
                tcls = intdb.tile_classes.key(tcid),
                bel = intdb.bel_slots.key(bslot),
            );
        }

        for ((tcid, conn, twc), bit) in self.sb_bidi {
            println!(
                "uncollected bit: bidi {tcls} {conn} {wire}: {bit}",
                tcls = intdb.tile_classes.key(tcid),
                conn = intdb.conn_slots.key(conn),
                wire = twc.to_string(intdb, &intdb.tile_classes[tcid]),
                bit = intdb.tile_classes[tcid].dump_polbit(bit),
            );
        }

        for ((tid, rid, fid), value) in self.table_data {
            let row = &mut intdb.tables[tid].rows[rid];
            if row.contains_id(fid) {
                assert_eq!(row[fid], value);
            } else {
                row.insert(fid, value);
            }
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let mut cf = zstd::stream::Decoder::new(f)?;
        let config = bincode::config::standard();
        Ok(bincode::decode_from_std_read(&mut cf, config)?)
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        let config = bincode::config::standard();
        bincode::encode_into_std_write(self, &mut cf, config)?;
        cf.finish()?;
        Ok(())
    }

    pub fn merge(&mut self, other: CollectorData) {
        fn merge_hashmap<K: Eq + Hash, V: Eq + Debug>(a: &mut HashMap<K, V>, b: HashMap<K, V>) {
            for (k, v) in b {
                match a.entry(k) {
                    hash_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), v);
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(v);
                    }
                }
            }
        }
        merge_hashmap(&mut self.bel_attrs, other.bel_attrs);
        merge_hashmap(&mut self.bel_input_inv, other.bel_input_inv);
        merge_hashmap(&mut self.sb_inv, other.sb_inv);
        merge_hashmap(&mut self.sb_buf, other.sb_buf);
        merge_hashmap(&mut self.sb_pass, other.sb_pass);
        merge_hashmap(&mut self.sb_bipass, other.sb_bipass);
        merge_hashmap(&mut self.sb_mux, other.sb_mux);
        merge_hashmap(&mut self.sb_delay, other.sb_delay);
        merge_hashmap(&mut self.sb_bidi, other.sb_bidi);
        merge_hashmap(&mut self.sb_enable, other.sb_enable);
        merge_hashmap(&mut self.sb_pairmux, other.sb_pairmux);
        merge_hashmap(&mut self.tmux_group, other.tmux_group);
        merge_hashmap(&mut self.table_data, other.table_data);
        merge_hashmap(&mut self.device_data, other.device_data);
        for (tile, tile_data) in other.bsdata.tiles {
            let tile_dst = self.bsdata.tiles.entry(tile).or_default();
            for (key, item) in tile_data.items {
                match tile_dst.items.entry(key) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(item);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        // could make a little smarter?
                        assert_eq!(item, *entry.get());
                    }
                }
            }
        }
        for (device, data) in other.bsdata.device_data {
            for (key, val) in data {
                self.bsdata.insert_device_data(&device, key, val);
            }
        }
        for (key, val) in other.bsdata.misc_data {
            self.bsdata.insert_misc_data(key, val);
        }
    }
}
