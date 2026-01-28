use prjcombine_entity::{EntityPartVec, EntityVec};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{BitRectId, TileBit, TileItem, TileItemKind},
};

use crate::{
    collect::Collector,
    diff::{
        Diff, DiffKey, FeatureId, OcdMode, extract_bitvec_val, extract_bitvec_val_part, xlat_bit,
        xlat_bit_bi, xlat_bit_bi_default, xlat_bit_wide, xlat_bit_wide_bi, xlat_bitvec,
        xlat_bitvec_sparse_u32, xlat_enum_raw,
    },
};

impl Diff {
    pub fn discard_bits_legacy(&mut self, item: &TileItem) {
        self.discard_bits(&item.bits);
    }

    pub fn apply_bitvec_diff_legacy(&mut self, item: &TileItem, from: &BitVec, to: &BitVec) {
        self.apply_bitvec_diff(&item.as_bitvec(), from, to);
    }

    pub fn apply_bitvec_diff_int_legacy(&mut self, item: &TileItem, from: u64, to: u64) {
        self.apply_bitvec_diff_int(&item.as_bitvec(), from, to);
    }

    pub fn apply_bit_diff_legacy(&mut self, item: &TileItem, from: bool, to: bool) {
        self.apply_bit_diff(item.as_bit(), from, to);
    }

    pub fn apply_enum_diff_legacy(&mut self, item: &TileItem, from: &str, to: &str) {
        let TileItemKind::Enum { ref values } = item.kind else {
            unreachable!()
        };
        self.apply_enum_diff_raw(&item.bits, &values[from], &values[to]);
    }

    pub fn from_bool_item_legacy(item: &TileItem) -> Self {
        let bit = item.as_bit();
        let mut res = Diff::default();
        res.bits.insert(bit.bit, !bit.inv);
        res
    }
}

/// Functions to get diffs
impl Collector<'_, '_> {
    pub fn get_diffs_legacy(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Vec<Diff> {
        let tile = tile.into();
        let bel = bel.into();
        let attr = attr.into();
        let val = val.into();
        let key = DiffKey::Legacy(FeatureId {
            tile,
            bel,
            attr,
            val,
        });
        self.get_diffs_raw(&key)
    }

    pub fn get_diff_legacy(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Diff {
        let mut res = self.get_diffs_legacy(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        res.pop().unwrap()
    }

    pub fn peek_diffs_legacy(
        &self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> &Vec<Diff> {
        let tile = tile.into();
        let bel = bel.into();
        let attr = attr.into();
        let val = val.into();
        let key = DiffKey::Legacy(FeatureId {
            tile,
            bel,
            attr,
            val,
        });
        self.diffs
            .get(&key)
            .unwrap_or_else(|| panic!("NO DIFF: {key:?}"))
    }

    pub fn peek_diff_legacy(
        &self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> &Diff {
        let res = self.peek_diffs_legacy(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        &res[0]
    }
}

/// Functions that extract (get_diff + xlat) and return the item instead of inserting it to bitdata.
impl Collector<'_, '_> {
    #[must_use]
    pub fn extract_bitvec_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val: &str,
    ) -> TileItem {
        xlat_bitvec_legacy(self.get_diffs_legacy(tile, bel, attr, val))
    }

    #[must_use]
    pub fn extract_bit_legacy(&mut self, tile: &str, bel: &str, attr: &str, val: &str) -> TileItem {
        let diff = self.get_diff_legacy(tile, bel, attr, val);
        xlat_bit(diff).into()
    }

    #[must_use]
    pub fn extract_bit_wide_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val: &str,
    ) -> TileItem {
        let diff = self.get_diff_legacy(tile, bel, attr, val);
        xlat_bit_wide(diff).into()
    }

    #[must_use]
    pub fn extract_bit_bi_default_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> (TileItem, bool) {
        let d0 = self.get_diff_legacy(tile, bel, attr, val0);
        let d1 = self.get_diff_legacy(tile, bel, attr, val1);
        xlat_bit_bi_default_legacy(d0, d1)
    }

    #[must_use]
    pub fn extract_bit_bi_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> TileItem {
        let d0 = self.get_diff_legacy(tile, bel, attr, val0);
        let d1 = self.get_diff_legacy(tile, bel, attr, val1);
        xlat_bit_bi(d0, d1).into()
    }

    #[must_use]
    pub fn extract_bit_wide_bi_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> TileItem {
        let d0 = self.get_diff_legacy(tile, bel, attr, val0);
        let d1 = self.get_diff_legacy(tile, bel, attr, val1);
        xlat_bit_wide_bi(d0, d1).into()
    }

    #[must_use]
    pub fn extract_enum_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.get_diff_legacy(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_legacy(diffs)
    }

    #[must_use]
    pub fn extract_enum_legacy_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        ocd: OcdMode,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.get_diff_legacy(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_legacy_ocd(diffs, ocd)
    }

    #[must_use]
    pub fn extract_enum_legacy_int(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: core::ops::Range<u32>,
        delta: u32,
    ) -> TileItem {
        let diffs = vals
            .map(|val| {
                (
                    val,
                    self.get_diff_legacy(tile, bel, attr, format!("{v}", v = val + delta)),
                )
            })
            .collect();
        xlat_bitvec_sparse_legacy(diffs)
    }

    #[must_use]
    pub fn extract_enum_default_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.get_diff_legacy(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_default_legacy(diffs, default)
    }

    #[must_use]
    pub fn extract_enum_default_legacy_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
        ocd: OcdMode,
    ) -> TileItem {
        let diffs = vals
            .iter()
            .map(|val| {
                (
                    val.as_ref().to_string(),
                    self.get_diff_legacy(tile, bel, attr, val.as_ref()),
                )
            })
            .collect();
        xlat_enum_default_ocd_legacy(diffs, default, ocd)
    }
}

/// Full-service collect functions (get_diff + xlat + insert)
impl Collector<'_, '_> {
    pub fn collect_bitvec_legacy(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        let item = xlat_bitvec_legacy(self.get_diffs_legacy(tile, bel, attr, val));
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_bit_legacy(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        let item = self.extract_bit_legacy(tile, bel, attr, val);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_bit_wide_legacy(&mut self, tile: &str, bel: &str, attr: &str, val: &str) {
        let item = self.extract_bit_wide_legacy(tile, bel, attr, val);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_bit_bi_default_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) -> bool {
        let (item, res) = self.extract_bit_bi_default_legacy(tile, bel, attr, val0, val1);
        self.data.bsdata.insert(tile, bel, attr, item);
        res
    }

    pub fn collect_bit_bi_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) {
        let item = self.extract_bit_bi_legacy(tile, bel, attr, val0, val1);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_bit_wide_bi_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        val0: &str,
        val1: &str,
    ) {
        let item = self.extract_bit_wide_bi_legacy(tile, bel, attr, val0, val1);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
    ) {
        let item = self.extract_enum_legacy(tile, bel, attr, vals);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_legacy_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        ocd: OcdMode,
    ) {
        let item = self.extract_enum_legacy_ocd(tile, bel, attr, vals, ocd);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_legacy_int(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: core::ops::Range<u32>,
        delta: u32,
    ) {
        let item = self.extract_enum_legacy_int(tile, bel, attr, vals, delta);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_default_legacy(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
    ) {
        let item = self.extract_enum_default_legacy(tile, bel, attr, vals, default);
        self.data.bsdata.insert(tile, bel, attr, item);
    }

    pub fn collect_enum_default_legacy_ocd(
        &mut self,
        tile: &str,
        bel: &str,
        attr: &str,
        vals: &[impl AsRef<str>],
        default: &str,
        ocd: OcdMode,
    ) {
        let item = self.extract_enum_default_legacy_ocd(tile, bel, attr, vals, default, ocd);
        self.data.bsdata.insert(tile, bel, attr, item);
    }
}

pub fn enum_ocd_swap_bits_legacy(item: &mut TileItem, a: usize, b: usize) {
    item.bits.swap(a, b);
    let TileItemKind::Enum { ref mut values } = item.kind else {
        unreachable!()
    };
    for val in values.values_mut() {
        val.swap(a, b);
    }
}

pub fn xlat_item_tile_fwd_legacy(
    item: TileItem,
    xlat: &EntityVec<BitRectId, BitRectId>,
) -> TileItem {
    TileItem {
        bits: item
            .bits
            .into_iter()
            .map(|bit| TileBit {
                rect: xlat[bit.rect],
                ..bit
            })
            .collect(),
        kind: item.kind,
    }
}

pub fn xlat_item_tile_legacy(item: TileItem, xlat: &EntityVec<BitRectId, BitRectId>) -> TileItem {
    let mut rxlat = EntityPartVec::new();
    for (dst_rect, &src_rect) in xlat {
        assert!(!rxlat.contains_id(src_rect));
        rxlat.insert(src_rect, dst_rect);
    }
    TileItem {
        bits: item
            .bits
            .into_iter()
            .map(|bit| TileBit {
                rect: rxlat[bit.rect],
                ..bit
            })
            .collect(),
        kind: item.kind,
    }
}

pub fn xlat_bitvec_legacy(diffs: Vec<Diff>) -> TileItem {
    xlat_bitvec(diffs).into()
}

pub fn xlat_bit_legacy(diff: Diff) -> TileItem {
    xlat_bit(diff).into()
}

pub fn xlat_bit_wide_legacy(diff: Diff) -> TileItem {
    xlat_bit_wide(diff).into()
}

pub fn concat_bitvec_legacy(vecs: impl IntoIterator<Item = TileItem>) -> TileItem {
    let mut res = vec![];
    for vec in vecs {
        res.extend(vec.as_bitvec());
    }
    res.into()
}

pub fn extract_bitvec_val_part_legacy(item: &TileItem, base: &BitVec, diff: &mut Diff) -> BitVec {
    extract_bitvec_val_part(&item.as_bitvec(), base, diff)
}

pub fn extract_bitvec_val_legacy(item: &TileItem, base: &BitVec, diff: Diff) -> BitVec {
    extract_bitvec_val(&item.as_bitvec(), base, diff)
}

pub fn xlat_bit_bi_default_legacy(diff0: Diff, diff1: Diff) -> (TileItem, bool) {
    let (bit, default) = xlat_bit_bi_default(diff0, diff1);
    (bit.into(), default)
}

pub fn xlat_bit_bi_legacy(diff0: Diff, diff1: Diff) -> TileItem {
    xlat_bit_bi(diff0, diff1).into()
}

pub fn xlat_enum_legacy_ocd(diffs: Vec<(impl Into<String>, Diff)>, ocd: OcdMode) -> TileItem {
    xlat_enum_raw(
        diffs
            .into_iter()
            .map(|(key, diff)| (key.into(), diff))
            .collect(),
        ocd,
    )
    .into()
}

pub fn xlat_enum_legacy(diffs: Vec<(impl Into<String>, Diff)>) -> TileItem {
    xlat_enum_legacy_ocd(diffs, OcdMode::ValueOrder)
}

pub fn xlat_enum_default_legacy(
    mut diffs: Vec<(String, Diff)>,
    default: impl Into<String>,
) -> TileItem {
    diffs.insert(0, (default.into(), Diff::default()));
    xlat_enum_legacy(diffs)
}

pub fn xlat_enum_default_ocd_legacy(
    mut diffs: Vec<(String, Diff)>,
    default: impl Into<String>,
    ocd: OcdMode,
) -> TileItem {
    diffs.insert(0, (default.into(), Diff::default()));
    xlat_enum_legacy_ocd(diffs, ocd)
}

pub fn xlat_bitvec_sparse_legacy(diffs: Vec<(u32, Diff)>) -> TileItem {
    xlat_bitvec_sparse_u32(diffs).into()
}
