use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

use prjcombine_types::bsdata::TileBit;
use unnamed_entity::{EntityVec, entity_id};

#[derive(Debug, Clone)]
pub struct Sample<BitTile: Copy + Eq + Ord + Debug> {
    pub diff: BTreeMap<(BitTile, usize, usize), bool>,
    pub patterns: BTreeSet<SamplePattern<BitTile>>,
}

impl<BitTile: Copy + Eq + Ord + Debug> Default for Sample<BitTile> {
    fn default() -> Self {
        Sample::new()
    }
}

impl<BitTile: Copy + Eq + Ord + Debug> Sample<BitTile> {
    pub fn new() -> Self {
        Self {
            diff: Default::default(),
            patterns: BTreeSet::new(),
        }
    }

    pub fn add_tiled_pattern(&mut self, bittiles: &[BitTile], name: impl Into<String>) {
        self.patterns.insert(SamplePattern {
            tiles: Some(bittiles.to_vec()),
            name: name.into(),
            single: false,
        });
    }

    pub fn add_tiled_pattern_single(&mut self, bittiles: &[BitTile], name: impl Into<String>) {
        self.patterns.insert(SamplePattern {
            tiles: Some(bittiles.to_vec()),
            name: name.into(),
            single: true,
        });
    }

    pub fn add_global_pattern(&mut self, name: impl Into<String>) {
        self.patterns.insert(SamplePattern {
            tiles: None,
            name: name.into(),
            single: false,
        });
    }

    pub fn add_global_pattern_single(&mut self, name: impl Into<String>) {
        self.patterns.insert(SamplePattern {
            tiles: None,
            name: name.into(),
            single: true,
        });
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct SamplePattern<BitTile: Copy + Eq + Ord + Debug> {
    pub tiles: Option<Vec<BitTile>>,
    pub name: String,
    pub single: bool,
}

#[derive(Debug)]
pub struct Harvester<BitTile: Copy + Eq + Ord + Debug> {
    pub samples: EntityVec<WorkSampleId, WorkSample<BitTile>>,
    pub pending_samples: BTreeSet<WorkSampleId>,
    pub work_tiled: BTreeMap<String, WorkPatternTiled>,
    pub work_global: BTreeMap<String, WorkPatternGlobal<BitTile>>,
    pub known_tiled: BTreeMap<String, BTreeMap<TileBit, bool>>,
    pub known_global: BTreeMap<String, BTreeMap<(BitTile, usize, usize), bool>>,
    pub debug: u8,
}

entity_id! {
    pub id WorkSampleId u32;
    pub id WorkSamplePatternId u32;
}

#[derive(Debug)]
pub struct WorkSample<BitTile: Copy + Eq + Ord + Debug> {
    pub orig_diff: BTreeMap<(BitTile, usize, usize), bool>,
    pub diff: BTreeMap<BitTile, BTreeMap<(usize, usize), bool>>,
    pub patterns: EntityVec<WorkSamplePatternId, SamplePattern<BitTile>>,
    pub global_patterns: BTreeSet<WorkSamplePatternId>,
    pub tiled_patterns: BTreeMap<BitTile, BTreeSet<(WorkSamplePatternId, usize)>>,
}

impl<BitTile: Copy + Eq + Ord + Debug> WorkSample<BitTile> {
    fn contains_bit(&self, bit: (BitTile, usize, usize), val: bool) -> bool {
        let Some(tile_bits) = self.diff.get(&bit.0) else {
            return false;
        };
        tile_bits.get(&(bit.1, bit.2)) == Some(&val)
    }

    fn remove_bit(&mut self, bit: (BitTile, usize, usize), val: bool) -> bool {
        let Some(tile_bits) = self.diff.get_mut(&bit.0) else {
            return false;
        };
        if tile_bits.remove(&(bit.1, bit.2)) != Some(val) {
            return false;
        }
        if tile_bits.is_empty() {
            self.diff.remove(&bit.0);
        }
        true
    }
}

#[derive(Debug, Default)]
pub struct WorkPatternTiled {
    pub locs: BTreeSet<(WorkSampleId, WorkSamplePatternId)>,
    pub known_bits: BTreeMap<TileBit, bool>,
    pub single: bool,
}

#[derive(Debug)]
pub struct WorkPatternGlobal<BitTile: Copy + Eq + Ord + Debug> {
    pub locs: BTreeSet<(WorkSampleId, WorkSamplePatternId)>,
    pub known_bits: BTreeMap<(BitTile, usize, usize), bool>,
    pub single: bool,
}

impl<BitTile: Copy + Eq + Ord + Debug> Default for WorkPatternGlobal<BitTile> {
    fn default() -> Self {
        Self {
            locs: Default::default(),
            known_bits: Default::default(),
            single: false,
        }
    }
}

impl<BitTile: Copy + Eq + Ord + Debug> Harvester<BitTile> {
    pub fn new() -> Self {
        Self {
            samples: Default::default(),
            pending_samples: Default::default(),
            work_tiled: Default::default(),
            work_global: Default::default(),
            known_tiled: Default::default(),
            known_global: Default::default(),
            debug: 0,
        }
    }

    pub fn want_tiled(&mut self, key: impl Into<String>) {
        let key = key.into();
        if self.known_tiled.contains_key(&key) {
            return;
        }
        self.work_tiled.entry(key).or_default();
    }

    pub fn want_global(&mut self, key: impl Into<String>) {
        let key = key.into();
        if self.known_global.contains_key(&key) {
            return;
        }
        self.work_global.entry(key).or_default();
    }

    pub fn force_tiled(&mut self, key: impl Into<String>, bits: BTreeMap<TileBit, bool>) {
        let key = key.into();
        if let Some(thing) = self.known_tiled.get(&key) {
            assert_eq!(*thing, bits);
        } else {
            for (&bit, &val) in &bits {
                self.add_known_bit_tiled(&key, bit, val);
            }
            let work = self.work_tiled.entry(key.clone()).or_default();
            assert_eq!(work.known_bits, bits);
            self.finish_tiled(key);
        }
    }

    pub fn force_global(
        &mut self,
        key: impl Into<String>,
        bits: BTreeMap<(BitTile, usize, usize), bool>,
    ) {
        let key = key.into();
        if let Some(thing) = self.known_global.get(&key) {
            assert_eq!(*thing, bits);
        } else {
            for (&bit, &val) in &bits {
                self.add_known_bit_global(&key, bit, val);
            }
            let work = self.work_global.entry(key.clone()).or_default();
            assert_eq!(work.known_bits, bits);
            self.finish_global(key);
        }
    }

    pub fn add_sample(&mut self, sample: Sample<BitTile>) -> Option<WorkSampleId> {
        let sample_id = self.samples.next_id();
        let mut diff: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        for (&(tile, frame, bit), &value) in &sample.diff {
            diff.entry(tile).or_default().insert((frame, bit), value);
        }
        let mut remove_bit = |harvester: &Harvester<BitTile>, (tile, frame, bit), val| {
            let Some(tile_bits) = diff.get_mut(&tile) else {
                harvester.fail(sample_id, (tile, frame, bit));
            };
            if tile_bits.remove(&(frame, bit)) != Some(val) {
                harvester.fail(sample_id, (tile, frame, bit));
            }
            if tile_bits.is_empty() {
                diff.remove(&tile);
            }
        };
        let patterns = EntityVec::from_iter(sample.patterns);
        let mut global_patterns = BTreeSet::new();
        let mut tiled_patterns: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for (pattern_id, pattern) in &patterns {
            match pattern.tiles {
                None => {
                    if let Some(bits) = self.known_global.get(&pattern.name) {
                        for (&bit, &val) in bits {
                            remove_bit(self, bit, val);
                        }
                    } else {
                        let work = self.work_global.entry(pattern.name.clone()).or_default();
                        if pattern.single {
                            work.single = true;
                        }
                        work.locs.insert((sample_id, pattern_id));
                        for (bit, val) in work.known_bits.clone() {
                            remove_bit(self, bit, val);
                        }
                        global_patterns.insert(pattern_id);
                    }
                }
                Some(ref tiles) => {
                    if let Some(bits) = self.known_tiled.get(&pattern.name) {
                        for (&bit, &val) in bits {
                            let xbit = (tiles[bit.tile], bit.frame, bit.bit);
                            remove_bit(self, xbit, val);
                        }
                    } else {
                        let work = self.work_tiled.entry(pattern.name.clone()).or_default();
                        if pattern.single {
                            work.single = true;
                        }
                        work.locs.insert((sample_id, pattern_id));
                        for (bit, val) in work.known_bits.clone() {
                            let xbit = (tiles[bit.tile], bit.frame, bit.bit);
                            remove_bit(self, xbit, val);
                        }

                        for (tile_idx, &tile) in tiles.iter().enumerate() {
                            tiled_patterns
                                .entry(tile)
                                .or_default()
                                .insert((pattern_id, tile_idx));
                        }
                    }
                }
            }
        }
        if global_patterns.is_empty() && tiled_patterns.is_empty() {
            if !diff.is_empty() {
                for (&tile, tile_bits) in &diff {
                    if let Some(&(frame, bit)) = tile_bits.keys().next() {
                        self.fail(sample_id, (tile, frame, bit));
                    }
                }
            }
            assert!(diff.is_empty());
            return None;
        }
        let sample = WorkSample {
            orig_diff: sample.diff,
            diff,
            patterns,
            global_patterns,
            tiled_patterns,
        };
        self.samples.push(sample);
        self.pending_samples.insert(sample_id);
        Some(sample_id)
    }

    fn add_known_bit_tiled(&mut self, name: &str, bit: TileBit, val: bool) {
        if let Some(bits) = self.known_tiled.get(name) {
            assert_eq!(bits.get(&bit), Some(&val));
            return;
        }
        let work = self.work_tiled.entry(name.to_string()).or_default();
        if let Some(&cur_val) = work.known_bits.get(&bit) {
            assert_eq!(cur_val, val);
            return;
        }
        if self.debug >= 3 {
            println!("KNOWN BIT TILED {name} {bit:?}:{val}");
        }
        work.known_bits.insert(bit, val);
        for &(sample_id, pattern_id) in &work.locs {
            let sample = &mut self.samples[sample_id];
            let tiles = sample.patterns[pattern_id].tiles.as_ref().unwrap();
            let xbit = (tiles[bit.tile], bit.frame, bit.bit);
            if !sample.remove_bit(xbit, val) {
                self.fail(sample_id, xbit);
            }
        }
    }

    fn add_known_bit_global(&mut self, name: &str, bit: (BitTile, usize, usize), val: bool) {
        if let Some(bits) = self.known_global.get(name) {
            assert_eq!(bits.get(&bit), Some(&val));
            return;
        }
        let work = self.work_global.entry(name.to_string()).or_default();
        if let Some(&cur_val) = work.known_bits.get(&bit) {
            assert_eq!(cur_val, val);
            return;
        }
        if self.debug >= 3 {
            println!("KNOWN BIT GLOBAL {name} {bit:?}:{val}");
        }
        work.known_bits.insert(bit, val);
        for &(sample_id, _) in &work.locs {
            let sample = &mut self.samples[sample_id];
            if !sample.remove_bit(bit, val) {
                self.fail(sample_id, bit);
            }
        }
    }

    fn tiled_bit_possible(&self, name: &str, bit: TileBit, val: bool) -> bool {
        let work = &self.work_tiled[name];
        for &(sample_id, pattern_id) in &work.locs {
            let sample = &self.samples[sample_id];
            let tiles = sample.patterns[pattern_id].tiles.as_ref().unwrap();
            let xbit = (tiles[bit.tile], bit.frame, bit.bit);
            if !sample.contains_bit(xbit, val) {
                return false;
            }
        }
        true
    }

    fn global_bit_possible(&self, name: &str, bit: (BitTile, usize, usize), val: bool) -> bool {
        let work = &self.work_global[name];
        for &(sample_id, _) in &work.locs {
            let sample = &self.samples[sample_id];
            if !sample.contains_bit(bit, val) {
                return false;
            }
        }
        true
    }

    fn finish_tiled(&mut self, name: String) {
        let work = self.work_tiled.remove(&name).unwrap();
        if self.debug >= 2 {
            println!("FINISH TILED {name} {bits:?}", bits = work.known_bits);
        }
        if work.single && work.known_bits.len() != 1 {
            panic!(
                "TILED NOT ACTUALLY SINGLE: {name} {bits:?}",
                bits = work.known_bits
            );
        }
        self.known_tiled.insert(name, work.known_bits);
        for (sample_id, pattern_id) in work.locs {
            let sample = &mut self.samples[sample_id];
            for (tile_idx, &tile) in sample.patterns[pattern_id]
                .tiles
                .as_ref()
                .unwrap()
                .iter()
                .enumerate()
            {
                let patterns = sample.tiled_patterns.get_mut(&tile).unwrap();
                assert!(patterns.remove(&(pattern_id, tile_idx)));
                if patterns.is_empty() {
                    sample.tiled_patterns.remove(&tile);
                }
            }
        }
    }

    fn finish_global(&mut self, name: String) {
        if self.known_global.contains_key(&name) {
            return;
        }
        let work = self.work_global.remove(&name).unwrap();
        if self.debug >= 2 {
            println!("FINISH GLOBAL {name} {bits:?}", bits = work.known_bits);
        }
        if work.single && work.known_bits.len() != 1 {
            panic!(
                "UMMM {name} not actually single: {bits:?}",
                bits = work.known_bits
            );
        }
        self.known_global.insert(name, work.known_bits);
        for (sample_id, pattern_id) in work.locs {
            let sample = &mut self.samples[sample_id];
            assert!(sample.global_patterns.remove(&pattern_id));
        }
    }

    pub fn rename_global(&mut self, name_from: impl Into<String>, name_to: impl Into<String>) {
        let name_from = name_from.into();
        let name_to = name_to.into();
        assert_ne!(name_from, name_to);
        let mut known_bits = BTreeMap::new();
        let mut known_all = false;
        for name in [&name_from, &name_to] {
            if let Some(bits) = self.known_global.get(name) {
                for (&bit, &val) in bits {
                    known_bits.insert(bit, val);
                }
                known_all = true;
            } else if let Some(work) = self.work_global.get(name) {
                for (&bit, &val) in &work.known_bits {
                    known_bits.insert(bit, val);
                }
            }
        }
        for (bit, val) in known_bits {
            for name in [&name_from, &name_to] {
                self.add_known_bit_global(name, bit, val);
            }
        }
        if known_all {
            self.finish_global(name_from.clone());
            self.finish_global(name_to.clone());
        }
        if let Some(bits) = self.known_global.remove(&name_from) {
            assert_eq!(self.known_global.get(&name_to), Some(&bits));
        } else if let Some(work_from) = self.work_global.remove(&name_from) {
            let work_to = self.work_global.entry(name_to.clone()).or_default();
            assert_eq!(work_to.known_bits, work_from.known_bits);
            work_to.single |= work_from.single;
            for (sample_id, pattern_id) in work_from.locs {
                self.samples[sample_id].patterns[pattern_id].name = name_to.clone();
                work_to.locs.insert((sample_id, pattern_id));
            }
        }
    }

    pub fn process(&mut self) {
        loop {
            if self.debug >= 2 {
                println!("JUDGEMENT ROUND STARTS");
            }
            let mut changed = false;
            let mut pairs = BTreeSet::new();
            let mut triples = BTreeSet::new();
            for sample_id in self.pending_samples.clone() {
                let diff = self.samples[sample_id].diff.clone();
                for (tile, tile_bits) in diff {
                    for ((frame, bit), value) in tile_bits {
                        let bit = (tile, frame, bit);
                        let sample = &self.samples[sample_id];
                        if !sample.contains_bit(bit, value) {
                            // removed in the meantime
                            continue;
                        }
                        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
                        enum CandidateOwner {
                            Global(WorkSamplePatternId),
                            Tiled(WorkSamplePatternId, usize),
                        }
                        let mut cands = vec![];
                        for &pattern_id in &sample.global_patterns {
                            if self.global_bit_possible(
                                &sample.patterns[pattern_id].name,
                                bit,
                                value,
                            ) {
                                cands.push(CandidateOwner::Global(pattern_id));
                            }
                        }
                        if let Some(tiled_patterns) = sample.tiled_patterns.get(&bit.0) {
                            for &(pattern_id, tile_idx) in tiled_patterns {
                                let tbit = TileBit {
                                    tile: tile_idx,
                                    frame: bit.1,
                                    bit: bit.2,
                                };
                                if self.tiled_bit_possible(
                                    &sample.patterns[pattern_id].name,
                                    tbit,
                                    value,
                                ) {
                                    cands.push(CandidateOwner::Tiled(pattern_id, tile_idx));
                                }
                            }
                        }
                        if cands.is_empty() {
                            println!("EMPTY CANDS for {bit:?}");
                            self.fail(sample_id, bit);
                        }
                        if cands.len() > 1 {
                            cands.sort();
                            match cands[..] {
                                [
                                    CandidateOwner::Tiled(pattern_id_a, tile_idx_a),
                                    CandidateOwner::Tiled(pattern_id_b, tile_idx_b),
                                ] if tile_idx_a == tile_idx_b => {
                                    let tbit = TileBit {
                                        tile: tile_idx_a,
                                        frame: bit.1,
                                        bit: bit.2,
                                    };
                                    pairs.insert((
                                        sample.patterns[pattern_id_a].name.clone(),
                                        sample.patterns[pattern_id_b].name.clone(),
                                        tbit,
                                        value,
                                    ));
                                }
                                [
                                    CandidateOwner::Tiled(pattern_id_a, tile_idx_a),
                                    CandidateOwner::Tiled(pattern_id_b, tile_idx_b),
                                    CandidateOwner::Tiled(pattern_id_c, tile_idx_c),
                                ] if tile_idx_a == tile_idx_b && tile_idx_a == tile_idx_c => {
                                    let tbit = TileBit {
                                        tile: tile_idx_a,
                                        frame: bit.1,
                                        bit: bit.2,
                                    };
                                    triples.insert((
                                        sample.patterns[pattern_id_a].name.clone(),
                                        sample.patterns[pattern_id_b].name.clone(),
                                        sample.patterns[pattern_id_c].name.clone(),
                                        tbit,
                                        value,
                                    ));
                                }
                                _ => (),
                            }
                            continue;
                        }
                        match cands[0] {
                            CandidateOwner::Global(pattern_id) => {
                                let pattern = &sample.patterns[pattern_id];
                                self.add_known_bit_global(&pattern.name.clone(), bit, value);
                            }
                            CandidateOwner::Tiled(pattern_id, tile_idx) => {
                                let pattern = &sample.patterns[pattern_id];
                                let tbit = TileBit {
                                    tile: tile_idx,
                                    frame: bit.1,
                                    bit: bit.2,
                                };
                                self.add_known_bit_tiled(&pattern.name.clone(), tbit, value);
                            }
                        }
                        changed = true;
                    }
                }
                let sample = &self.samples[sample_id];
                if sample.diff.is_empty() {
                    let mut finish_tiled_queue = BTreeSet::new();
                    let mut finish_global_queue = BTreeSet::new();
                    for &pattern_id in &sample.global_patterns {
                        finish_global_queue.insert(sample.patterns[pattern_id].name.clone());
                    }
                    for tiled_patterns in sample.tiled_patterns.values() {
                        for &(pattern_id, _) in tiled_patterns {
                            finish_tiled_queue.insert(sample.patterns[pattern_id].name.clone());
                        }
                    }
                    for name in finish_global_queue {
                        self.finish_global(name);
                    }
                    for name in finish_tiled_queue {
                        self.finish_tiled(name);
                    }
                    self.pending_samples.remove(&sample_id);
                    changed = true;
                    continue;
                }
                let mut finish_tiled_queue = BTreeSet::new();
                for tiled_patterns in sample.tiled_patterns.values() {
                    for &(pattern_id, _) in tiled_patterns {
                        let mut is_empty = true;
                        for &tile in sample.patterns[pattern_id].tiles.as_ref().unwrap() {
                            if sample.diff.contains_key(&tile) {
                                is_empty = false;
                                break;
                            }
                        }
                        if is_empty {
                            finish_tiled_queue.insert(sample.patterns[pattern_id].name.clone());
                        }
                    }
                }
                for name in finish_tiled_queue {
                    self.finish_tiled(name);
                    changed = true;
                }
                let sample = &self.samples[sample_id];
                let mut patterns_empty = sample.global_patterns.is_empty();
                for tiled_patterns in sample.tiled_patterns.values() {
                    if !tiled_patterns.is_empty() {
                        patterns_empty = false;
                    }
                }
                if patterns_empty {
                    for (&tile, tile_bits) in &sample.diff {
                        if let Some(&(frame, bit)) = tile_bits.keys().next() {
                            self.fail(sample_id, (tile, frame, bit));
                        }
                    }
                }
            }
            for (pattern_a, pattern_b, pattern_c, tbit, val) in triples {
                for (pattern, pair0, pair1) in [
                    (
                        &pattern_a,
                        (pattern_a.clone(), pattern_b.clone(), tbit, val),
                        (pattern_a.clone(), pattern_c.clone(), tbit, val),
                    ),
                    (
                        &pattern_b,
                        (pattern_a.clone(), pattern_b.clone(), tbit, val),
                        (pattern_b.clone(), pattern_c.clone(), tbit, val),
                    ),
                    (
                        &pattern_c,
                        (pattern_a.clone(), pattern_c.clone(), tbit, val),
                        (pattern_b.clone(), pattern_c.clone(), tbit, val),
                    ),
                ] {
                    if pairs.contains(&pair0) && pairs.contains(&pair1) {
                        self.add_known_bit_tiled(pattern, tbit, val);
                    }
                }
            }
            let mut finish_tiled_queue = BTreeSet::new();
            for (key, work) in &self.work_tiled {
                if work.known_bits.len() == 1 && work.single {
                    finish_tiled_queue.insert(key.clone());
                }
            }
            for key in finish_tiled_queue {
                self.finish_tiled(key);
                changed = true;
            }
            let mut finish_global_queue = BTreeSet::new();
            for (key, work) in &self.work_global {
                if work.known_bits.len() == 1 && work.single {
                    finish_global_queue.insert(key.clone());
                }
            }
            for key in finish_global_queue {
                self.finish_global(key);
                changed = true;
            }
            if !changed {
                break;
            }
        }
        if self.debug >= 1 {
            println!(
                "GLOBAL {kg}:{wg} TILED {kt}:{wt}",
                kg = self.known_global.len(),
                wg = self.work_global.len(),
                kt = self.known_tiled.len(),
                wt = self.work_tiled.len()
            );
        }
        if self.debug >= 4 {
            for (sample_id, sample) in &self.samples {
                println!("SAMPLE {sample_id}");
                for (tile, bits) in &sample.diff {
                    println!("    BITTILE {tile:?}");
                    for (bit, val) in bits {
                        println!("    BIT {bit:?} {val}");
                    }
                }
            }
            for (key, val) in &self.known_global {
                println!("KNOWN GLOBAL: {key} {val:?}");
            }
            for (key, val) in &self.known_tiled {
                println!("KNOWN TILED: {key} {val:?}");
            }
        }
        if self.debug >= 2 {
            for (key, val) in &self.work_global {
                println!("WORK GLOBAL: {key} {known:?}", known = val.known_bits);
            }
            for (key, val) in &self.work_tiled {
                println!("WORK TILED: {key} {known:?}", known = val.known_bits);
            }
        }
    }

    pub fn has_unresolved(&self) -> bool {
        !self.pending_samples.is_empty()
            || !self.work_global.is_empty()
            || !self.work_tiled.is_empty()
    }

    fn fail(&self, sample_id: WorkSampleId, bit: (BitTile, usize, usize)) -> ! {
        println!("FAIL AT SAMPLE {sample_id} {bit:?}");
        for (sample_id, sample) in &self.samples {
            println!("SAMPLE {sample_id}:");
            if self.debug >= 5 {
                for (&(tile, frame, bit), &value) in &sample.orig_diff {
                    println!("    {tile:?} {frame}.{bit}: {value}");
                }
                for (pattern_id, pattern) in &sample.patterns {
                    if let Some(ref tiles) = pattern.tiles {
                        println!(
                            "    PATTERN {pattern_id}: {name} at {tiles:?}",
                            name = pattern.name
                        );
                    } else {
                        println!(
                            "    PATTERN {pattern_id}: {name} [global]",
                            name = pattern.name
                        );
                    }
                }
            }
            println!("    DIFF:");
            for (&tile, tile_bits) in &sample.diff {
                println!("    TILE {tile:?}:");
                for (bit, value) in tile_bits {
                    println!("        {bit:?}: {value}");
                }
                if let Some(patterns) = sample.tiled_patterns.get(&tile) {
                    for &(pattern_id, tile_idx) in patterns {
                        let name = &sample.patterns[pattern_id].name;
                        println!("        PATTERN {name} [{tile_idx}]");
                    }
                }
            }
            for (&tile, patterns) in &sample.tiled_patterns {
                if sample.diff.contains_key(&tile) {
                    continue;
                }
                println!("    TILE {tile:?}:");
                for &(pattern_id, tile_idx) in patterns {
                    let name = &sample.patterns[pattern_id].name;
                    println!("        PATTERN {name} [{tile_idx}]");
                }
            }
            for &pattern_id in &sample.global_patterns {
                let name = &sample.patterns[pattern_id].name;
                println!("    GLOBAL {name}");
            }
        }
        println!("FAIL SAMPLE:");
        let sample = &self.samples[sample_id];
        for (&bit, &val) in &sample.orig_diff {
            println!("    BIT {bit:?} {val}");
        }
        for (pattern_id, pattern) in &sample.patterns {
            if let Some(ref tiles) = pattern.tiles {
                println!(
                    "    PATTERN {pattern_id}: {name} at {tiles:?}",
                    name = pattern.name
                );
            } else {
                println!(
                    "    PATTERN {pattern_id}: {name} [global]",
                    name = pattern.name
                );
            }
        }
        panic!("FAIL AT {bit:?}");
    }
}

impl<BitTile: Copy + Eq + Ord + Debug> Default for Harvester<BitTile> {
    fn default() -> Self {
        Self::new()
    }
}
