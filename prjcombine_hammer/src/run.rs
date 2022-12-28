#![allow(clippy::type_complexity)]
use crate::*;
use bimap::BiHashMap;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::thread_rng;
use std::collections::hash_map::Entry;
use std::sync::Condvar;
use std::sync::Mutex;
use std_semaphore::{Semaphore, SemaphoreGuard};

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
    postproc: HashSet<B::PostProc>,
}

fn prep_batch<B: Backend>(batch: &Batch<B>) -> BatchData<B> {
    let mut postproc = HashSet::new();
    let mut bits = vec![];
    for (fid, f) in &batch.fuzzers {
        postproc.extend(f.postproc.iter().cloned());
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
        if let BatchValue::Base(b) = v {
            let v = b.iter().choose(&mut rng).unwrap();
            base_kv.insert(k.clone(), v.clone());
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
        postproc,
    }
}

fn run_batch_item<B: Backend>(
    backend: &B,
    batch: &Batch<B>,
    bdata: &BatchData<B>,
    idx: Option<usize>,
    g: SemaphoreGuard,
) {
    let mut kv = bdata.base_kv.clone();
    for (k, v) in &batch.kv {
        match v {
            BatchValue::Base(_) => (),
            BatchValue::Fuzzer(fid, a, b) => {
                let v = if let Some(idx) = idx {
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
                    val.push(if let Some(idx) = idx {
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
    bdata.cv.notify_one();
    std::mem::drop(g);
}

impl<'a, B: Backend> Session<'a, B> {
    pub fn run(self) -> Option<B::State> {
        let batches = self.batches.map_values(prep_batch);

        let backend = self.backend;

        let mut state = backend.make_state();
        let sema = Semaphore::new(std::thread::available_parallelism().unwrap().get() as isize);
        std::thread::scope(|s| {
            s.spawn(|| {
                for (bid, batch) in &self.batches {
                    let bd = &batches[bid];
                    let g = sema.access();
                    s.spawn(|| run_batch_item(backend, batch, bd, None, g));
                    for i in 0..bd.width {
                        let g = sema.access();
                        s.spawn(move || run_batch_item(backend, batch, bd, Some(i), g));
                    }
                }
            });
            for (bid, batch) in &self.batches {
                let bd = &batches[bid];
                let mut g = bd.state.lock().unwrap();
                while g.base_bs.is_none() || g.other_bs.iter().any(|x| x.is_none()) {
                    g = bd.cv.wait(g).unwrap();
                }
                let (mut base_bs, kv) = g.base_bs.take().unwrap();
                for pp in &bd.postproc {
                    backend.postproc(&state, &mut base_bs, pp, &kv);
                }
                let mut bits: HashMap<B::BitPos, (bool, u64)> = HashMap::new();
                for (i, x) in g.other_bs.iter_mut().enumerate() {
                    let (mut bs, kv) = x.take().unwrap();
                    for pp in &bd.postproc {
                        backend.postproc(&state, &mut bs, pp, &kv);
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
                        // TODO: diagnostics
                        panic!("oops unknown cw for {bit:?}");
                    }
                }
                for (fid, bits) in fuzzers {
                    let fuzzer = &batch.fuzzers[fid];
                    if let Some(oops) = backend.return_fuzzer(
                        &mut state,
                        &fuzzer.backend,
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

        Some(state)
    }
}
