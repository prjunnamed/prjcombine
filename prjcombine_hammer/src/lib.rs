use bitvec::vec::BitVec;
use core::fmt::Debug;
use core::hash::Hash;
use derivative::Derivative;
use prjcombine_entity::{entity_id, EntityVec};
use std::collections::{hash_map::Entry, HashMap, HashSet};

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
    pub bits: usize,
    pub info: B::FuzzerInfo,
}

impl<B: Backend> Clone for Fuzzer<B> {
    fn clone(&self) -> Self {
        Self {
            kv: self.kv.clone(),
            postproc: self.postproc.clone(),
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

    pub fn fuzz_multi(mut self, key: B::Key, val: impl Into<B::MultiValue>) -> Self {
        let val = FuzzerValue::FuzzerMulti(val.into());
        match self.kv.entry(key) {
            Entry::Occupied(e) => assert_eq!(*e.get(), val),
            Entry::Vacant(e) => {
                e.insert(val);
            }
        }
        self
    }
}

pub trait FuzzerGen<B: Backend>: Debug {
    fn gen(&self, kv: &HashMap<B::Key, BatchValue<B>>) -> Option<Fuzzer<B>>;
}

#[derive(Debug)]
struct SimpleFuzzerGen<B: Backend>(Fuzzer<B>);

impl<B: Backend> FuzzerGen<B> for SimpleFuzzerGen<B> {
    fn gen(&self, kv: &HashMap<<B as Backend>::Key, BatchValue<B>>) -> Option<Fuzzer<B>> {
        for (k, v) in &self.0.kv {
            if let Some(cv) = kv.get(k) {
                match (cv, v) {
                    (BatchValue::Base(cb), FuzzerValue::Base(fb)) => {
                        if cb != fb {
                            return None;
                        }
                    }
                    (BatchValue::Base(cb), FuzzerValue::BaseAny(fb)) => {
                        if !fb.contains(cb) {
                            return None;
                        }
                    }
                    (BatchValue::BaseAny(cb), FuzzerValue::Base(fb)) => {
                        if !cb.contains(fb) {
                            return None;
                        }
                    }
                    (BatchValue::BaseAny(cb), FuzzerValue::BaseAny(fb)) => {
                        let nb = cb & fb;
                        if nb.is_empty() {
                            return None;
                        }
                    }
                    (BatchValue::BaseAny(cb), FuzzerValue::Fuzzer(a, b)) => {
                        if !cb.contains(a) || !cb.contains(b) {
                            return None;
                        }
                    }
                    (BatchValue::Fuzzer(_, ca, cb), FuzzerValue::BaseAny(fb)) => {
                        if !fb.contains(ca) || !fb.contains(cb) {
                            return None;
                        }
                    }
                    _ => return None,
                };
            }
        }
        Some(self.0.clone())
    }
}

#[derive(Debug)]
struct FuzzerGenWrapper<'a, B: Backend> {
    fgen: Box<dyn FuzzerGen<B> + 'a>,
    dup: u32,
}

struct Batch<B: Backend> {
    kv: HashMap<B::Key, BatchValue<B>>,
    fuzzers: EntityVec<BatchFuzzerId, Fuzzer<B>>,
}

pub struct Session<'a, B: Backend> {
    backend: &'a B,
    pub debug: u8,
    pub dup_factor: u32,
    batches: EntityVec<BatchId, Batch<B>>,
    fgens: Vec<FuzzerGenWrapper<'a, B>>,
}

pub struct FuzzerGenHandle<'b, 'a, B: Backend> {
    session: &'b mut Session<'a, B>,
    idx: usize,
}

impl<B: Backend> FuzzerGenHandle<'_, '_, B> {
    pub fn dup(self, val: u32) -> Self {
        assert_ne!(val, 0);
        self.session.fgens[self.idx].dup = val;
        self
    }
}

impl<'a, B: Backend> Session<'a, B> {
    pub fn new(backend: &'a B) -> Self {
        Session {
            backend,
            debug: 0,
            dup_factor: 3,
            batches: EntityVec::new(),
            fgens: vec![],
        }
    }

    pub fn add_fuzzer(&mut self, fgen: Box<dyn FuzzerGen<B> + 'a>) -> FuzzerGenHandle<'_, 'a, B> {
        let i = self.fgens.len();
        self.fgens.push(FuzzerGenWrapper {
            fgen,
            dup: self.dup_factor,
        });
        FuzzerGenHandle {
            session: self,
            idx: i,
        }
    }

    pub fn add_fuzzer_simple(&mut self, fuzzer: Fuzzer<B>) -> FuzzerGenHandle<'_, 'a, B> {
        self.add_fuzzer(Box::new(SimpleFuzzerGen(fuzzer))).dup(1)
    }

    // TODO:
    // - opportunistic fuzzers
    // - independent fuzzers
}

mod run;
