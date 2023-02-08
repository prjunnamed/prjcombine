use bitvec::vec::BitVec;
use core::fmt::Debug;
use core::hash::Hash;
use derivative::Derivative;
use prjcombine_entity::{entity_id, EntityVec};
use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap, HashSet},
};

entity_id! {
    pub id BatchFuzzerId u32;
    id BatchId u32;
}

pub trait Backend: Debug + Sync {
    type Key: Hash + PartialEq + Eq + PartialOrd + Ord + Clone + Debug + Sync + Send;
    type Value: Hash + PartialEq + Eq + PartialOrd + Ord + Clone + Debug + Sync + Send;
    type MultiValue: Hash + PartialEq + Eq + Clone + Debug + Sync + Send;
    type Bitstream: Clone + Debug + Sync + Send;
    type FuzzerInfo: Clone + Debug + Sync + Send;
    type PostProc: Hash + PartialEq + Eq + Clone + Debug + Sync + Send;
    type BitPos: Copy + Clone + Debug + Hash + PartialEq + Eq + Sync + Send;
    type State: Debug + Sync + Send;

    fn make_state(&self) -> Self::State;
    fn assemble_multi(v: &Self::MultiValue, b: &BitVec) -> Self::Value;
    fn bitgen(&self, kv: &HashMap<Self::Key, Self::Value>) -> Self::Bitstream;
    fn diff(bs1: &Self::Bitstream, bs2: &Self::Bitstream) -> HashMap<Self::BitPos, bool>;
    fn return_fuzzer(
        &self,
        s: &mut Self::State,
        f: &Self::FuzzerInfo,
        fi: FuzzerId,
        bits: Vec<HashMap<Self::BitPos, bool>>,
    ) -> Option<Vec<FuzzerId>>;
    fn postproc(
        &self,
        s: &Self::State,
        bs: &mut Self::Bitstream,
        pp: &Self::PostProc,
        kv: &HashMap<Self::Key, Self::Value>,
    ) -> bool;
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Eq(bound = ""),
    PartialEq(bound = "")
)]
pub enum BatchValue<B: Backend> {
    Base(B::Value),
    BaseAny(HashSet<B::Value>),
    Fuzzer(BatchFuzzerId, B::Value, B::Value),
    FuzzerMulti(BatchFuzzerId, B::MultiValue),
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Eq(bound = ""),
    PartialEq(bound = "")
)]
pub enum FuzzerValue<B: Backend> {
    Base(B::Value),
    BaseAny(HashSet<B::Value>),
    Fuzzer(B::Value, B::Value),
    FuzzerMulti(B::MultiValue),
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct FuzzerId {
    batch: BatchId,
    fuzzer: BatchFuzzerId,
}

#[derive(Debug)]
pub struct Fuzzer<B: Backend> {
    pub kv: HashMap<B::Key, FuzzerValue<B>>,
    pub postproc: HashSet<B::PostProc>,
    pub deps: HashSet<FuzzerId>,
    pub bits: usize,
    pub info: B::FuzzerInfo,
}

impl<B: Backend> Clone for Fuzzer<B> {
    fn clone(&self) -> Self {
        Self {
            kv: self.kv.clone(),
            postproc: self.postproc.clone(),
            deps: self.deps.clone(),
            bits: self.bits,
            info: self.info.clone(),
        }
    }
}

impl<B: Backend> Fuzzer<B> {
    pub fn new(info: B::FuzzerInfo) -> Self {
        Self {
            info,
            kv: HashMap::new(),
            postproc: HashSet::new(),
            deps: HashSet::new(),
            bits: 1,
        }
    }

    pub fn bits(mut self, bits: usize) -> Self {
        self.bits = bits;
        self
    }

    pub fn base_any(mut self, key: B::Key, vals: impl IntoIterator<Item = B::Value>) -> Self {
        let val = FuzzerValue::BaseAny(vals.into_iter().collect());
        match self.kv.entry(key) {
            Entry::Occupied(e) => assert_eq!(*e.get(), val),
            Entry::Vacant(e) => {
                e.insert(val);
            }
        }
        self
    }

    pub fn base(mut self, key: B::Key, val: impl Into<B::Value>) -> Self {
        let val = FuzzerValue::Base(val.into());
        match self.kv.entry(key) {
            Entry::Occupied(e) => assert_eq!(*e.get(), val),
            Entry::Vacant(e) => {
                e.insert(val);
            }
        }
        self
    }

    pub fn fuzz(mut self, key: B::Key, a: impl Into<B::Value>, b: impl Into<B::Value>) -> Self {
        let val = FuzzerValue::Fuzzer(a.into(), b.into());
        match self.kv.entry(key) {
            Entry::Occupied(e) => assert_eq!(*e.get(), val),
            Entry::Vacant(e) => {
                e.insert(val);
            }
        }
        self
    }
}

struct Batch<B: Backend> {
    kv: HashMap<B::Key, BatchValue<B>>,
    fuzzers: EntityVec<BatchFuzzerId, Fuzzer<B>>,
}

impl<B: Backend> Batch<B> {
    fn add_fuzzer(&mut self, fuzzer: Cow<Fuzzer<B>>) -> Option<BatchFuzzerId> {
        let mut new_kv = HashMap::new();
        let fid = self.fuzzers.next_id();
        for (k, v) in &fuzzer.kv {
            if let Some(cv) = self.kv.get(k) {
                let nv = match (cv, v) {
                    (BatchValue::Base(cb), FuzzerValue::Base(fb)) => {
                        if cb != fb {
                            return None;
                        }
                        continue;
                    }
                    (BatchValue::Base(cb), FuzzerValue::BaseAny(fb)) => {
                        if !fb.contains(cb) {
                            return None;
                        }
                        continue;
                    }
                    (BatchValue::BaseAny(cb), FuzzerValue::Base(fb)) => {
                        if !cb.contains(fb) {
                            continue;
                        }
                        BatchValue::Base(fb.clone())
                    }
                    (BatchValue::BaseAny(cb), FuzzerValue::BaseAny(fb)) => {
                        let nb = cb & fb;
                        if nb.is_empty() {
                            return None;
                        }
                        if nb == *cb {
                            continue;
                        }
                        BatchValue::BaseAny(nb)
                    }
                    (BatchValue::BaseAny(cb), FuzzerValue::Fuzzer(a, b)) => {
                        if !cb.contains(a) || !cb.contains(b) {
                            return None;
                        }
                        BatchValue::Fuzzer(fid, a.clone(), b.clone())
                    }
                    (BatchValue::Fuzzer(_, ca, cb), FuzzerValue::BaseAny(fb)) => {
                        if !fb.contains(ca) || !fb.contains(cb) {
                            return None;
                        }
                        continue;
                    }
                    _ => return None,
                };
                new_kv.insert(k.clone(), nv);
            } else {
                let nv = match v {
                    FuzzerValue::Base(b) => BatchValue::Base(b.clone()),
                    FuzzerValue::BaseAny(b) => BatchValue::BaseAny(b.clone()),
                    FuzzerValue::Fuzzer(a, b) => BatchValue::Fuzzer(fid, a.clone(), b.clone()),
                    FuzzerValue::FuzzerMulti(a) => BatchValue::FuzzerMulti(fid, a.clone()),
                };
                new_kv.insert(k.clone(), nv);
            }
        }
        for (k, v) in new_kv {
            self.kv.insert(k, v);
        }
        Some(self.fuzzers.push(fuzzer.into_owned()))
    }
}

pub struct Session<'a, B: Backend> {
    backend: &'a B,
    pub debug: u8,
    batches: EntityVec<BatchId, Batch<B>>,
}

impl<'a, B: Backend> Session<'a, B> {
    pub fn new(backend: &'a B) -> Self {
        Session {
            backend,
            debug: 0,
            batches: EntityVec::new(),
        }
    }

    pub fn add_fuzzer(
        &mut self,
        mut fgen: impl FnMut(&HashMap<B::Key, BatchValue<B>>) -> Option<Fuzzer<B>>,
    ) -> FuzzerId {
        'batches: for (bid, batch) in &mut self.batches {
            if let Some(fuzzer) = fgen(&batch.kv) {
                for dep in &fuzzer.deps {
                    if bid <= dep.batch {
                        continue 'batches;
                    }
                }
                if let Some(fid) = batch.add_fuzzer(Cow::Owned(fuzzer)) {
                    return FuzzerId {
                        batch: bid,
                        fuzzer: fid,
                    };
                }
            }
        }
        let mut batch = Batch {
            kv: HashMap::new(),
            fuzzers: EntityVec::new(),
        };
        let Some(fuzzer) = fgen(&batch.kv) else {
            panic!("failed to generate fuzzer on empty batch");
        };
        if let Some(fid) = batch.add_fuzzer(Cow::Owned(fuzzer)) {
            let bid = self.batches.push(batch);
            FuzzerId {
                batch: bid,
                fuzzer: fid,
            }
        } else {
            panic!("failed to add fuzzer on empty batch");
        }
    }

    pub fn add_fuzzer_simple(&mut self, fuzzer: Fuzzer<B>) -> FuzzerId {
        'batches: for (bid, batch) in &mut self.batches {
            for dep in &fuzzer.deps {
                if bid <= dep.batch {
                    continue 'batches;
                }
            }
            if let Some(fid) = batch.add_fuzzer(Cow::Borrowed(&fuzzer)) {
                return FuzzerId {
                    batch: bid,
                    fuzzer: fid,
                };
            }
        }
        let mut batch = Batch {
            kv: HashMap::new(),
            fuzzers: EntityVec::new(),
        };
        if let Some(fid) = batch.add_fuzzer(Cow::Borrowed(&fuzzer)) {
            let bid = self.batches.push(batch);
            FuzzerId {
                batch: bid,
                fuzzer: fid,
            }
        } else {
            panic!("failed to add fuzzer on empty batch");
        }
    }

    // TODO:
    // - opportunistic fuzzers
    // - independent fuzzers
}

mod run;
