#![allow(clippy::type_complexity)]
use crate::*;
use bimap::BiHashMap;
use itertools::Itertools;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::thread_rng;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Condvar;
use std::sync::Mutex;
use unnamed_entity::EntityId;

struct BatchState<B: Backend> {
    base_bs: Option<(B::Bitstream, HashMap<B::Key, B::Value>)>,
    other_bs: Vec<Option<(B::Bitstream, HashMap<B::Key, B::Value>)>>,
}

struct BatchData<B: Backend> {
    state: Mutex<BatchState<B>>,
    cv: Condvar,
    width: usize,
    base_kv: HashMap<B::Key, B::Value>,
    code: BiHashMap<(BatchFuzzerId, usize), u64>,
}

struct TaskQueue<'a, 'b, B: Backend> {
    debug: u8,
    backend: &'b B,
    batches: &'a EntityVec<BatchId, Batch<B>>,
    bdata: EntityVec<BatchId, BatchData<B>>,
    skip: HashSet<BatchFuzzerId>,
    items: Vec<(BatchId, Option<usize>)>,
    next: AtomicUsize,
    abort: AtomicBool,
}

fn prep_batch<B: Backend>(batch: &Batch<B>) -> BatchData<B> {
    let mut bits = vec![];
    for (fid, f) in &batch.fuzzers {
        for i in 0..f.bits {
            bits.push((fid, i));
        }
    }
    let mut rng = thread_rng();
    bits.shuffle(&mut rng);
    let mut width = 1;
    let mut code;
    'cb: loop {
        code = BiHashMap::new();
        let mut cw: u64 = 0;
        let hw = (width + 1) / 2;
        for &x in &bits {
            loop {
                if cw >= 1 << width {
                    width += 1;
                    assert!(width < 64);
                    continue 'cb;
                }
                if cw.count_ones() == hw {
                    break;
                }
                cw += 1;
            }
            code.insert(x, cw);
            cw += 1;
        }
        break;
    }
    let width = width as usize;
    let mut base_kv = HashMap::new();
    for (k, v) in &batch.kv {
        match v {
            BatchValue::Base(b) => {
                base_kv.insert(k.clone(), b.clone());
            }
            BatchValue::BaseAny(b) => {
                let v = b.iter().choose(&mut rng).unwrap();
                base_kv.insert(k.clone(), v.clone());
            }
            _ => (),
        }
    }
    let state = BatchState {
        base_bs: None,
        other_bs: vec![None; width],
    };
    BatchData {
        state: Mutex::new(state),
        cv: Condvar::new(),
        base_kv,
        width,
        code,
    }
}

fn run_batch_item<B: Backend>(
    backend: &B,
    batch: &Batch<B>,
    bdata: &BatchData<B>,
    idx: Option<usize>,
    skip: &HashSet<BatchFuzzerId>,
) {
    let mut kv = bdata.base_kv.clone();
    for (k, v) in &batch.kv {
        match v {
            BatchValue::Base(_) => (),
            BatchValue::BaseAny(_) => (),
            BatchValue::Fuzzer(fid, a, b) => {
                let v = if skip.contains(fid) {
                    a
                } else if let Some(idx) = idx {
                    let cw = bdata.code.get_by_left(&(*fid, 0)).unwrap();
                    let state = cw >> idx & 1;
                    if state != 0 {
                        b
                    } else {
                        a
                    }
                } else {
                    a
                };
                kv.insert(k.clone(), v.clone());
            }
            BatchValue::FuzzerMulti(fid, a) => {
                let mut val: BitVec = BitVec::new();
                for i in 0..batch.fuzzers[*fid].bits {
                    val.push(if skip.contains(fid) {
                        false
                    } else if let Some(idx) = idx {
                        let cw = bdata.code.get_by_left(&(*fid, i)).unwrap();
                        let state = cw >> idx & 1;
                        state != 0
                    } else {
                        false
                    });
                }
                kv.insert(k.clone(), B::assemble_multi(a, &val));
            }
        }
    }
    let bs = backend.bitgen(&kv);
    let mut s = bdata.state.lock().unwrap();
    if let Some(idx) = idx {
        s.other_bs[idx] = Some((bs, kv));
    } else {
        s.base_bs = Some((bs, kv));
    }
    std::mem::drop(s);
    bdata.cv.notify_one();
}

fn work<B: Backend>(queue: &TaskQueue<B>) {
    loop {
        if queue.abort.load(Ordering::Relaxed) {
            return;
        }
        let idx = queue.next.fetch_add(1, Ordering::Relaxed);
        if idx >= queue.items.len() {
            break;
        }
        let (bid, idx) = queue.items[idx];
        if queue.debug >= 2 {
            if let Some(idx) = idx {
                eprintln!("Starting batch {bid} run {idx}", bid = bid.to_idx());
            } else {
                eprintln!("Starting batch {bid} base run", bid = bid.to_idx());
            }
        }
        run_batch_item(
            queue.backend,
            &queue.batches[bid],
            &queue.bdata[bid],
            idx,
            &queue.skip,
        );
    }
}

fn postproc_batch<B: Backend>(
    backend: &B,
    state: &B::State,
    batch: &Batch<B>,
    bd: &BatchData<B>,
    skip: &HashSet<BatchFuzzerId>,
) -> Result<EntityVec<BatchFuzzerId, Vec<HashMap<B::BitPos, bool>>>, B::BitPos> {
    let mut g = bd.state.lock().unwrap();
    while g.base_bs.is_none() || g.other_bs.iter().any(|x| x.is_none()) {
        g = bd.cv.wait(g).unwrap();
    }
    let (mut base_bs, kv) = g.base_bs.take().unwrap();
    let mut postproc = HashSet::new();
    for (fid, f) in &batch.fuzzers {
        if !skip.contains(&fid) {
            postproc.extend(f.postproc.iter().cloned());
        }
    }
    for pp in &postproc {
        backend.postproc(state, &mut base_bs, pp, &kv);
    }
    let mut bits: HashMap<B::BitPos, (bool, u64)> = HashMap::new();
    for (i, x) in g.other_bs.iter_mut().enumerate() {
        let (mut bs, kv) = x.take().unwrap();
        for pp in &postproc {
            backend.postproc(state, &mut bs, pp, &kv);
        }
        let diff = B::diff(&base_bs, &bs);
        for (bit, dir) in diff {
            match bits.entry(bit) {
                Entry::Vacant(e) => {
                    e.insert((dir, 1 << i));
                }
                Entry::Occupied(mut v) => {
                    assert_eq!(v.get().0, dir);
                    v.get_mut().1 |= 1 << i;
                }
            }
        }
    }
    let mut fuzzers = batch.fuzzers.map_values(|f| vec![HashMap::new(); f.bits]);
    for (bit, (dir, cw)) in bits {
        if let Some(&(fid, bidx)) = bd.code.get_by_right(&cw) {
            fuzzers[fid][bidx].insert(bit, dir);
        } else {
            return Err(bit);
        }
    }
    Ok(fuzzers)
}

fn try_cw_fail<B: Backend>(
    backend: &B,
    state: &B::State,
    batch: &Batch<B>,
    bd: &BatchData<B>,
    skip: &HashSet<BatchFuzzerId>,
) -> Result<EntityVec<BatchFuzzerId, Vec<HashMap<B::BitPos, bool>>>, B::BitPos> {
    let bstate = BatchState {
        base_bs: None,
        other_bs: vec![None; bd.width],
    };
    let nbd = BatchData {
        state: Mutex::new(bstate),
        cv: Condvar::new(),
        width: bd.width,
        base_kv: bd.base_kv.clone(),
        code: bd.code.clone(),
    };
    let bd = &nbd;
    std::thread::scope(|s| {
        s.spawn(|| run_batch_item(backend, batch, bd, None, skip));
        for i in 0..bd.width {
            s.spawn(move || run_batch_item(backend, batch, bd, Some(i), skip));
        }
    });
    postproc_batch(backend, state, batch, bd, skip)
}

fn diagnose_cw_fail<B: Backend>(
    backend: &B,
    state: &B::State,
    batch: &Batch<B>,
    bd: &BatchData<B>,
    bitpos: B::BitPos,
) {
    eprintln!(
        "CW FAIL at {bitpos:?}; DIAGNOSING [{}]",
        batch.fuzzers.len()
    );
    let mut rng = thread_rng();
    let mut skip: HashSet<BatchFuzzerId> = HashSet::new();
    'big: loop {
        let left: Vec<_> = batch.fuzzers.ids().filter(|f| !skip.contains(f)).collect();
        if left.len() >= 9 {
            for _ in 0..4 {
                let mut nskip = skip.clone();
                let n = left.len() / 3;
                nskip.extend(left.choose_multiple(&mut rng, n));
                if try_cw_fail(backend, state, batch, bd, &nskip).is_err() {
                    skip = nskip;
                    println!("REDUCE FAST {}", batch.fuzzers.len() - skip.len());
                    continue 'big;
                }
            }
        }
        for &f in &left {
            let mut nskip = skip.clone();
            nskip.insert(f);
            if try_cw_fail(backend, state, batch, bd, &nskip).is_err() {
                skip.insert(f);
                println!("REDUCE SLOW {}", batch.fuzzers.len() - skip.len());
                continue 'big;
            }
        }
        eprintln!("INTERFERENCE FUZZERS:");
        for &fid in &left {
            let tskip: HashSet<_> = batch.fuzzers.ids().filter(|of| *of != fid).collect();
            let fuzzers = try_cw_fail(backend, state, batch, bd, &tskip).unwrap();
            eprintln!(
                "FUZZER {f:?}: {fd:?}",
                f = batch.fuzzers[fid].info,
                fd = fuzzers[fid]
            );
            for (k, v) in batch.fuzzers[fid].kv.iter().sorted_by_key(|x| x.0) {
                match v {
                    FuzzerValue::Base(_) => eprintln!("BASE {k:?} {v:?}", v = bd.base_kv[k]),
                    FuzzerValue::BaseAny(_) => eprintln!("BASE ANY {k:?} {v:?}", v = bd.base_kv[k]),
                    FuzzerValue::Fuzzer(v1, v2) => eprintln!("FUZZ {k:?} {v1:?} {v2:?}"),
                    FuzzerValue::FuzzerMulti(mv) => eprintln!("MULTIFUZZ {k:?} {mv:?}"),
                }
            }
        }
        break;
    }
}

impl<B: Backend> Batch<B> {
    fn install_fuzzer(&mut self, fuzzer: Fuzzer<B>) -> Option<BatchFuzzerId> {
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
                            return None;
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
        Some(self.fuzzers.push(fuzzer))
    }
}

impl<'a, B: Backend> Session<'a, B> {
    fn install_fuzzer(
        &mut self,
        state: &mut B::State,
        fgen: &(dyn FuzzerGen<B> + 'a),
    ) -> (FuzzerId, Option<Box<dyn FuzzerGen<B> + 'a>>) {
        for (bid, batch) in &mut self.batches {
            if let Some((fuzzer, chain)) = fgen.gen(self.backend, state, &batch.kv) {
                if let Some(fid) = batch.install_fuzzer(fuzzer) {
                    return (
                        FuzzerId {
                            batch: bid,
                            fuzzer: fid,
                        },
                        chain,
                    );
                }
            }
        }
        let mut batch = Batch {
            kv: HashMap::new(),
            fuzzers: EntityVec::new(),
        };
        let Some((fuzzer, chain)) = fgen.gen(self.backend, state, &batch.kv) else {
            panic!("failed to generate fuzzer on empty batch");
        };
        if let Some(fid) = batch.install_fuzzer(fuzzer) {
            let bid = self.batches.push(batch);
            (
                FuzzerId {
                    batch: bid,
                    fuzzer: fid,
                },
                chain,
            )
        } else {
            panic!("failed to add fuzzer on empty batch");
        }
    }

    fn prep_batches(&mut self, state: &mut B::State) {
        let mut gens = vec![];
        for (i, g) in self.fgens.iter().enumerate() {
            assert_ne!(g.dup, 0);
            for _ in 0..g.dup {
                gens.push(i);
            }
        }
        let mut rng = thread_rng();
        gens.shuffle(&mut rng);
        let fgens = core::mem::take(&mut self.fgens);
        for i in gens {
            let mut chain = self.install_fuzzer(state, &*fgens[i].fgen).1;
            while let Some(gen) = chain {
                chain = self.install_fuzzer(state, &*gen).1;
            }
        }
        self.fgens = fgens;
    }

    pub fn run(mut self) -> Option<B::State> {
        let backend = self.backend;
        let mut state = backend.make_state();
        self.prep_batches(&mut state);
        let batches = self.batches.map_values(prep_batch);
        let mut items = vec![];
        for (bid, b) in &batches {
            items.push((bid, None));
            for i in 0..b.width {
                items.push((bid, Some(i)));
            }
        }
        let queue = TaskQueue {
            debug: self.debug,
            backend: self.backend,
            batches: &self.batches,
            bdata: batches,
            skip: HashSet::new(),
            items,
            next: 0.into(),
            abort: false.into(),
        };
        if self.debug >= 1 {
            let nf: usize = self.batches.values().map(|x| x.fuzzers.len()).sum();
            let nr: usize = queue.bdata.values().map(|x| x.width + 1).sum();
            let nb = self.batches.len();
            eprintln!("Starting hammer run with {nf} fuzzers and {nr} runs in {nb} batches");
        }
        if self.debug >= 3 {
            for (bid, batch) in &self.batches {
                eprintln!(
                    "Batch {bid} [{nr} runs]:",
                    bid = bid.to_idx(),
                    nr = queue.bdata[bid].width
                );
                for f in batch.fuzzers.values() {
                    eprintln!("{f:?}", f = f.info);
                }
            }
        }
        let nt = std::thread::available_parallelism().unwrap().get();
        std::thread::scope(|s| {
            for _ in 0..nt {
                s.spawn(|| work(&queue));
            }
            for (bid, batch) in &self.batches {
                let bd = &queue.bdata[bid];
                let fuzzers = match postproc_batch(backend, &state, batch, bd, &queue.skip) {
                    Ok(f) => f,
                    Err(bitpos) => {
                        queue.abort.store(true, Ordering::Relaxed);
                        diagnose_cw_fail(backend, &state, batch, bd, bitpos);
                        panic!("oops weird cw for {bitpos:?}")
                    }
                };
                for (fid, bits) in fuzzers {
                    let fuzzer = &batch.fuzzers[fid];
                    if let Some(oops) = backend.return_fuzzer(
                        &mut state,
                        &fuzzer.info,
                        FuzzerId {
                            batch: bid,
                            fuzzer: fid,
                        },
                        bits,
                    ) {
                        // TODO: diagnostics
                        panic!("oopsed fuzzer {oops:?}");
                    }
                }
            }
        });
        if self.debug >= 1 {
            eprintln!("Hammer done");
        }

        Some(state)
    }
}
