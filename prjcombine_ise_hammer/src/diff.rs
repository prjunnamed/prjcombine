use std::{
    cmp::Ordering,
    collections::{btree_map, hash_map, BTreeMap, BTreeSet, HashMap},
};

use bitvec::vec::BitVec;
use prjcombine_types::{TileItem, TileItemKind};
use prjcombine_xilinx_geom::{Device, ExpandedDevice};
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, State},
    tiledb::TileDb,
};

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Diff {
    pub bits: HashMap<FeatureBit, bool>,
}

impl Diff {
    pub fn assert_empty(self) {
        assert!(self.bits.is_empty());
    }

    pub fn combine(&self, other: &Diff) -> Diff {
        let mut res = self.clone();
        for (k, v) in &other.bits {
            match res.bits.entry(*k) {
                hash_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), !*v);
                    e.remove();
                }
                hash_map::Entry::Vacant(e) => {
                    e.insert(*v);
                }
            }
        }
        res
    }

    pub fn split(mut a: Diff, mut b: Diff) -> (Diff, Diff, Diff) {
        let mut common = Diff::default();
        a.bits.retain(|&k, av| match b.bits.remove(&k) {
            Some(bv) => {
                assert_eq!(*av, bv);
                common.bits.insert(k, bv);
                false
            }
            None => true,
        });
        (a, b, common)
    }

    pub fn discard_bits(&mut self, item: &TileItem<FeatureBit>) {
        for bit in item.bits.iter() {
            self.bits.remove(bit);
        }
    }

    pub fn apply_enum_diff(&mut self, item: &TileItem<FeatureBit>, from: &str, to: &str) {
        let TileItemKind::Enum { ref values } = item.kind else {
            unreachable!()
        };
        let from = &values[from];
        let to = &values[to];
        for (idx, &bit) in item.bits.iter().enumerate() {
            if from[idx] != to[idx] {
                match self.bits.entry(bit) {
                    hash_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), from[idx]);
                        e.remove();
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(to[idx]);
                    }
                }
            }
        }
    }
}

impl core::ops::Not for Diff {
    type Output = Diff;

    fn not(self) -> Self::Output {
        Diff {
            bits: self.bits.into_iter().map(|(k, v)| (k, !v)).collect(),
        }
    }
}

pub struct CollectorCtx<'a, 'b>
where
    'b: 'a,
{
    pub device: &'a Device,
    pub edev: &'a ExpandedDevice<'a>,
    pub state: &'b mut State<'a>,
    pub tiledb: &'b mut TileDb,
}

pub fn xlat_bitvec(diffs: Vec<Diff>) -> TileItem<FeatureBit> {
    let mut invert = None;
    let mut bits = vec![];
    for diff in diffs {
        assert_eq!(diff.bits.len(), 1);
        for (k, v) in diff.bits {
            bits.push(k);
            if invert.is_none() {
                invert = Some(!v);
            } else {
                assert_eq!(invert, Some(!v));
            }
        }
    }
    TileItem {
        bits,
        kind: TileItemKind::BitVec {
            invert: invert.unwrap(),
        },
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OcdMode {
    BitOrder,
    ValueOrder,
    Mux,
}

pub fn xlat_enum_inner(diffs: Vec<(String, Diff)>, ocd: OcdMode) -> TileItem<FeatureBit> {
    let mut bits = BTreeMap::new();
    for (_, diff) in &diffs {
        for (&bit, &pol) in &diff.bits {
            match bits.entry(bit) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(pol);
                }
                btree_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), pol);
                }
            }
        }
    }
    let mut bits_vec: Vec<_> = bits.iter().map(|(&f, _)| f).collect();
    if ocd != OcdMode::BitOrder {
        bits_vec.sort_by(|a, b| {
            for (_, diff) in &diffs {
                let va = bits[a] ^ !diff.bits.contains_key(a);
                let vb = bits[b] ^ !diff.bits.contains_key(b);
                if va != vb {
                    return vb.cmp(&va);
                }
            }
            Ordering::Equal
        });
    }
    if ocd == OcdMode::Mux {
        let tmp_values = Vec::from_iter(diffs.iter().map(|(_, v)| {
            Vec::from_iter(
                bits_vec
                    .iter()
                    .map(|&bit| bits[&bit] ^ !v.bits.contains_key(&bit)),
            )
        }));

        let mut en_bits = vec![];
        let mut onehot_groups = vec![];
        let mut taken = BTreeSet::new();

        for sbit in 0..bits_vec.len() {
            if taken.contains(&sbit) {
                continue;
            }
            let mut onehot_group = BTreeSet::new();
            onehot_group.insert(sbit);
            for nbit in (sbit + 1)..bits_vec.len() {
                if taken.contains(&nbit) {
                    continue;
                }
                let mut disjoint = true;
                for &cbit in &onehot_group {
                    for val in &tmp_values {
                        if val[nbit] && val[cbit] {
                            disjoint = false;
                            break;
                        }
                    }
                }
                if disjoint {
                    onehot_group.insert(nbit);
                }
            }
            let mut full = true;
            for val in &tmp_values {
                let mut cnt = 0;
                for &bit in &onehot_group {
                    if val[bit] {
                        cnt += 1;
                    }
                }
                assert!(cnt < 2);
                if cnt == 0 && val.iter().any(|&x| x) {
                    full = false;
                    break;
                }
            }
            if !full {
                continue;
            }
            for &bit in &onehot_group {
                taken.insert(bit);
            }
            if onehot_group.len() == 1 {
                // enable, not onehot_group.
                let bit = onehot_group.pop_first().unwrap();
                en_bits.push(bit);
            } else {
                onehot_groups.push(onehot_group);
            }
        }
        onehot_groups.sort_by_key(|group| std::cmp::Reverse(group.len()));
        let mut new_bits_vec = vec![];
        for bit in en_bits {
            new_bits_vec.push(bits_vec[bit]);
        }
        for group in onehot_groups {
            for bit in group {
                new_bits_vec.push(bits_vec[bit]);
            }
        }
        for bit in 0..bits_vec.len() {
            if !taken.contains(&bit) {
                new_bits_vec.push(bits_vec[bit]);
            }
        }
        bits_vec = new_bits_vec;
    }
    let values = diffs
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                bits_vec
                    .iter()
                    .map(|&bit| bits[&bit] ^ !v.bits.contains_key(&bit))
                    .collect(),
            )
        })
        .collect();
    TileItem {
        bits: bits_vec,
        kind: TileItemKind::Enum { values },
    }
}

pub fn xlat_enum(diffs: Vec<(String, Diff)>) -> TileItem<FeatureBit> {
    xlat_enum_inner(diffs, OcdMode::ValueOrder)
}

pub fn xlat_enum_default(
    mut diffs: Vec<(String, Diff)>,
    default: impl Into<String>,
) -> TileItem<FeatureBit> {
    diffs.insert(0, (default.into(), Diff::default()));
    xlat_enum(diffs)
}

pub fn xlat_bool(diff0: Diff, diff1: Diff) -> TileItem<FeatureBit> {
    let diff = if diff0.bits.is_empty() {
        diff0.assert_empty();
        diff1
    } else {
        diff1.assert_empty();
        !diff0
    };
    xlat_bitvec(vec![diff])
}

impl<'a, 'b: 'a> CollectorCtx<'a, 'b> {
    pub fn collect_bitvec(&mut self, tile: &'a str, bel: &'a str, attr: &'a str, val: &'a str) {
        let diffs = self.state.get_diffs(tile, bel, attr, val);
        let ti = xlat_bitvec(diffs);
        self.tiledb.insert(tile, bel, attr, ti);
    }

    pub fn collect_enum(&mut self, tile: &'a str, bel: &'a str, attr: &'a str, vals: &[&'a str]) {
        let diffs = vals
            .iter()
            .map(|val| (val.to_string(), self.state.get_diff(tile, bel, attr, val)))
            .collect();
        let ti = xlat_enum(diffs);
        self.tiledb.insert(tile, bel, attr, ti);
    }

    pub fn collect_enum_default(
        &mut self,
        tile: &'b str,
        bel: &'b str,
        attr: &'b str,
        vals: &[&'b str],
        default: &'b str,
    ) {
        let diffs = vals
            .iter()
            .map(|val| (val.to_string(), self.state.get_diff(tile, bel, attr, val)))
            .collect();
        let ti = xlat_enum_default(diffs, default);
        self.tiledb.insert(tile, bel, attr, ti);
    }

    pub fn extract_enum_bool(
        &mut self,
        tile: &'b str,
        bel: &'b str,
        attr: &'b str,
        val0: &'b str,
        val1: &'b str,
    ) -> TileItem<FeatureBit> {
        let d0 = self.state.get_diff(tile, bel, attr, val0);
        let d1 = self.state.get_diff(tile, bel, attr, val1);
        xlat_bool(d0, d1)
    }

    pub fn collect_enum_bool(
        &mut self,
        tile: &'b str,
        bel: &'b str,
        attr: &'b str,
        val0: &'b str,
        val1: &'b str,
    ) {
        let item = self.extract_enum_bool(tile, bel, attr, val0, val1);
        self.tiledb.insert(tile, bel, attr, item);
    }

    pub fn collect_inv(&mut self, tile: &'b str, bel: &'b str, pin: &'b str) {
        let pininv = format!("{pin}INV").leak();
        let pin_b = format!("{pin}_B").leak();
        self.collect_enum_bool(tile, bel, pininv, pin, pin_b);
    }

    pub fn insert_int_inv(
        &mut self,
        int_tiles: &[&str],
        tile: &str,
        bel: &str,
        pin: &str,
        mut item: TileItem<FeatureBit>,
    ) {
        let intdb = self.edev.egrid().db;
        let node = intdb.nodes.get(tile).unwrap().1;
        let bel = node.bels.get(bel).unwrap().1;
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        assert_eq!(item.bits.len(), 1);
        let bit = &mut item.bits[0];
        assert_eq!(wire.0.to_idx(), bit.tile);
        bit.tile = 0;
        let wire_name = intdb.wires.key(wire.1);
        self.tiledb.insert(
            int_tiles[wire.0.to_idx()],
            "INT",
            format!("INV.{wire_name}"),
            item,
        );
    }

    pub fn item_int_inv(
        &self,
        int_tiles: &[&'b str],
        tile: &'b str,
        bel: &'b str,
        pin: &'b str,
    ) -> TileItem<FeatureBit> {
        let intdb = self.edev.egrid().db;
        let node = intdb.nodes.get(tile).unwrap().1;
        let bel = node.bels.get(bel).unwrap().1;
        let pin = &bel.pins[pin];
        assert_eq!(pin.wires.len(), 1);
        let wire = *pin.wires.first().unwrap();
        let wire_name = intdb.wires.key(wire.1);
        let mut item = self
            .tiledb
            .item(
                int_tiles[wire.0.to_idx()],
                "INT",
                &format!("INV.{wire_name}"),
            )
            .clone();
        assert_eq!(item.bits.len(), 1);
        let bit = &mut item.bits[0];
        bit.tile = wire.0.to_idx();
        item
    }

    pub fn collect_int_inv(
        &mut self,
        int_tiles: &[&'b str],
        tile: &'b str,
        bel: &'b str,
        pin: &'b str,
        flip: bool,
    ) {
        let pininv = format!("{pin}INV").leak();
        let pin_b = format!("{pin}_B").leak();
        let item = self.extract_enum_bool(
            tile,
            bel,
            pininv,
            if flip { pin_b } else { pin },
            if flip { pin } else { pin_b },
        );
        self.insert_int_inv(int_tiles, tile, bel, pin, item);
    }
}

pub fn extract_bitvec_val(item: &TileItem<FeatureBit>, base: &BitVec, diff: Diff) -> BitVec {
    let TileItemKind::BitVec { invert } = item.kind else {
        unreachable!()
    };
    assert_eq!(item.bits.len(), base.len());
    let mut res = base.clone();
    let rev: HashMap<_, _> = item
        .bits
        .iter()
        .copied()
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();
    for (&bit, &val) in diff.bits.iter() {
        let bitidx = rev[&bit];
        assert_eq!(res[bitidx], !(val ^ invert));
        res.set(bitidx, val ^ invert);
    }
    res
}
