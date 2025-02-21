use bitvec::vec::BitVec;
use prjcombine_re_hammer::{Backend, Fuzzer, Session};
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_cpld::{
    bits::{BitPos, BitstreamMap},
    bitstream::{parse_svf, Bitstream},
    device::DeviceKind,
    impact::run_impact,
};
use std::collections::HashMap;

#[derive(Debug)]
struct BitstreamBackend<'a> {
    tc: &'a Toolchain,
    dev: &'a str,
    pkg: &'a str,
    kind: DeviceKind,
    len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum Never {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum Key {
    Main(usize),
    Usercode(usize),
    Ues(usize),
    ReadProt,
    WriteProt,
    Lockout,
    ProtLock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum FuzzerInfo {
    Main(usize),
    Usercode(usize),
    Ues(usize),
    ReadProt,
    WriteProt,
}

#[derive(Debug, Default)]
struct State {
    main: HashMap<usize, BitPos>,
    usercode: HashMap<usize, BitPos>,
    ues: HashMap<usize, BitPos>,
    rprot: Vec<BitPos>,
    wprot: Vec<BitPos>,
}

impl Backend for BitstreamBackend<'_> {
    type Key = Key;
    type Value = bool;
    type MultiValue = Never;
    type Bitstream = Bitstream;
    type FuzzerInfo = FuzzerInfo;
    type PostProc = Never;
    type BitPos = (BitPos, bool);
    type State = State;

    fn make_state(&self) -> State {
        State::default()
    }

    fn assemble_multi(v: &Self::MultiValue, _b: &bitvec::vec::BitVec) -> Self::Value {
        match *v {}
    }

    fn bitgen(&self, kv: &std::collections::HashMap<Self::Key, Self::Value>) -> Self::Bitstream {
        let mut bits = BitVec::repeat(!self.kind.is_xc9500x(), self.len);
        let mut usercode: u32 = 0;
        let mut ues = vec![];
        let mut rprot = false;
        let mut wprot = false;
        for (&k, &v) in kv {
            match k {
                Key::Main(idx) => {
                    bits.set(idx, v);
                }
                Key::Usercode(idx) => {
                    if v {
                        usercode |= 1 << idx;
                    }
                }
                Key::Ues(idx) => {
                    let byte = idx >> 3;
                    let bit = idx & 7;
                    while byte >= ues.len() {
                        ues.push(0xff);
                    }
                    if v {
                        ues[byte] |= 1 << (bit ^ 7);
                    } else {
                        ues[byte] &= !(1 << (bit ^ 7));
                    }
                }
                Key::ReadProt => rprot = v,
                Key::WriteProt => wprot = v,
                Key::Lockout | Key::ProtLock => (),
            }
        }
        let usercode = if self.kind == DeviceKind::Coolrunner2 {
            Some(usercode)
        } else {
            None
        };
        let ues = if self.kind == DeviceKind::Xpla3 {
            while ues.last() == Some(&0xff) {
                ues.pop();
            }
            if !ues.is_empty() {
                Some(&*ues)
            } else {
                None
            }
        } else {
            None
        };
        let svf = run_impact(
            self.tc, self.dev, self.pkg, &bits, usercode, ues, rprot, wprot,
        )
        .unwrap();
        parse_svf(self.kind, &svf)
    }

    fn diff(
        bs1: &Self::Bitstream,
        bs2: &Self::Bitstream,
    ) -> std::collections::HashMap<Self::BitPos, bool> {
        assert_eq!(bs1.words.len(), bs2.words.len());
        let mut res = HashMap::new();
        for (&(a1, ref w1), &(a2, ref w2)) in bs1.words.iter().zip(bs2.words.iter()) {
            assert_eq!(a1, a2);
            if w1 != w2 {
                for (i, v) in w2.iter().enumerate() {
                    if w1[i] != v {
                        res.insert(((a1, i), false), *v);
                    }
                }
            }
        }
        for &k in &bs1.fixups {
            if !bs2.fixups.contains(&k) {
                res.insert((k, true), false);
            }
        }
        for &k in &bs2.fixups {
            if !bs1.fixups.contains(&k) {
                res.insert((k, true), true);
            }
        }
        res
    }

    fn return_fuzzer(
        &self,
        s: &mut Self::State,
        f: &FuzzerInfo,
        _fi: prjcombine_re_hammer::FuzzerId,
        mut bits: Vec<std::collections::HashMap<Self::BitPos, bool>>,
    ) -> Option<Vec<prjcombine_re_hammer::FuzzerId>> {
        match *f {
            FuzzerInfo::Main(idx) => {
                if self.kind == DeviceKind::Xc9500Xv {
                    bits[0].retain(|k, _| !k.1);
                }
                assert_eq!(bits[0].len(), 1);
                let (&(bit, is_f), &val) = bits[0].iter().next().unwrap();
                assert!(val);
                assert!(!is_f);
                assert!(s.main.insert(idx, bit).is_none());
            }
            FuzzerInfo::Usercode(idx) => {
                assert_eq!(bits[0].len(), 1);
                let (&(bit, is_f), &val) = bits[0].iter().next().unwrap();
                assert!(!val);
                assert!(is_f);
                assert!(s.usercode.insert(idx, bit).is_none());
            }
            FuzzerInfo::Ues(idx) => {
                if bits[0].is_empty() {
                    return None;
                }
                assert_eq!(bits[0].len(), 1);
                let (&(bit, is_f), &val) = bits[0].iter().next().unwrap();
                assert!(!val);
                assert!(is_f);
                assert!(s.ues.insert(idx, bit).is_none());
            }
            FuzzerInfo::ReadProt => {
                assert!(bits[0].values().all(|&x| x));
                assert!(bits[0].keys().all(|&(_, is_f)| is_f));
                let mut bits: Vec<_> = bits[0].keys().map(|&(b, _)| b).collect();
                bits.sort();
                s.rprot = bits;
            }
            FuzzerInfo::WriteProt => {
                assert!(bits[0].values().all(|&x| x));
                assert!(bits[0].keys().all(|&(_, is_f)| is_f));
                let mut bits: Vec<_> = bits[0].keys().map(|&(b, _)| b).collect();
                bits.sort();
                s.wprot = bits;
            }
        }
        None
    }

    fn postproc(
        &self,
        _s: &Self::State,
        _bs: &mut Self::Bitstream,
        pp: &Self::PostProc,
        _kv: &std::collections::HashMap<Self::Key, Self::Value>,
    ) -> bool {
        match *pp {}
    }
}

// Sigh. There's a typo in impact data files.  Force these to the right values.
const XCR3512XL_UES_FIXUP: [(usize, BitPos); 4] = [
    (141, (209, 425)),
    (161, (209, 405)),
    (524, (465, 425)),
    (544, (465, 405)),
];

pub fn reverse_bitstream(
    tc: &Toolchain,
    kind: DeviceKind,
    dev: &str,
    pkg: &str,
    len: usize,
) -> BitstreamMap {
    let backend = BitstreamBackend {
        tc,
        dev,
        pkg,
        kind,
        len,
    };
    let mut hammer = Session::new(&backend);
    let need_lockout = kind == DeviceKind::Coolrunner2 && dev.starts_with("xa");
    for i in 0..len {
        let mut f = Fuzzer::new(FuzzerInfo::Main(i))
            .base(Key::ProtLock, false)
            .fuzz(Key::Main(i), false, true);
        if need_lockout {
            f = f.base(Key::Lockout, false);
        }
        hammer.add_fuzzer_simple(f);
    }
    let is_xcr3512xl = dev == "xcr3512xl";
    if !kind.is_xc9500() {
        if kind == DeviceKind::Coolrunner2 {
            for i in 0..32 {
                let mut f =
                    Fuzzer::new(FuzzerInfo::Usercode(i)).fuzz(Key::Usercode(i), false, true);
                if need_lockout {
                    f = f.base(Key::Lockout, true);
                }
                hammer.add_fuzzer_simple(f);
            }
        } else {
            for i in 0..(100 * 8) {
                if is_xcr3512xl && XCR3512XL_UES_FIXUP.iter().any(|&(pos, _)| pos == i) {
                    continue;
                }
                let byte = i >> 3;
                let bit = i & 7;
                let mut f = Fuzzer::new(FuzzerInfo::Ues(i)).fuzz(Key::Ues(i), false, true);
                for j in 0..byte {
                    f = f.base(Key::Ues(j * 8), false);
                }
                match bit {
                    0 => {
                        f = f.base(Key::Ues((byte + 1) * 8), true);
                        for j in 1..8 {
                            f = f.base(Key::Ues(byte * 8 + j), true);
                        }
                    }
                    1 => {
                        f = f
                            .base(Key::Ues(byte * 8), false)
                            .base(Key::Ues(byte * 8 + 2), true)
                            .base(Key::Ues(byte * 8 + 3), true);
                    }
                    _ => {
                        f = f
                            .base(Key::Ues(byte * 8), false)
                            .base(Key::Ues(byte * 8 + 1), true);
                    }
                }
                hammer.add_fuzzer_simple(f);
            }
        }
    }
    let mut fuzzer = Fuzzer::new(FuzzerInfo::ReadProt).fuzz(Key::ReadProt, false, true);
    if kind.is_xc9500() {
        fuzzer = fuzzer.base(Key::ProtLock, true);
    }
    hammer.add_fuzzer_simple(fuzzer);
    if kind.is_xc9500() {
        let fuzzer = Fuzzer::new(FuzzerInfo::WriteProt)
            .base(Key::ProtLock, true)
            .fuzz(Key::WriteProt, false, true);
        hammer.add_fuzzer_simple(fuzzer);
    }

    let mut map = hammer.run().unwrap();
    if is_xcr3512xl {
        for (k, v) in XCR3512XL_UES_FIXUP {
            map.ues.insert(k, v);
        }
    }
    let main = (0..len).map(|i| map.main[&i]).collect();
    let usercode = if map.usercode.is_empty() {
        None
    } else {
        Some(core::array::from_fn(|i| map.usercode[&i]))
    };
    let ues = if map.ues.is_empty() {
        None
    } else {
        Some((0..map.ues.len()).map(|i| map.ues[&i]).collect())
    };

    let mut dims = None;
    let mut done = None;
    let mut transfer = vec![];
    let svf = run_impact(
        tc,
        dev,
        pkg,
        &BitVec::repeat(!kind.is_xc9500x(), len),
        None,
        None,
        false,
        false,
    )
    .unwrap();
    let bs = parse_svf(kind, &svf);

    if !kind.is_xc9500() {
        let cols = bs.words[0].1.len();
        let rows = bs.words.len();
        dims = Some((cols, rows, bs.abits.unwrap()));
        let mut tw = bs.words[0].1.clone();
        for (_, w) in &bs.words {
            tw |= w;
        }
        for i in tw.iter_zeros() {
            transfer.push(i);
        }
        for &(a, ref w) in &bs.words {
            for i in w.iter_zeros() {
                if tw[i] {
                    assert!(done.is_none());
                    done = Some((a, i));
                }
            }
        }
    }
    if matches!(kind, DeviceKind::Xc9500Xv | DeviceKind::Coolrunner2) {
        assert_eq!(bs.fixups.len(), 1);
        done = bs.fixups.into_iter().next();
    } else {
        assert!(bs.fixups.is_empty());
    }
    BitstreamMap {
        main,
        usercode,
        ues,
        rprot: map.rprot,
        wprot: map.wprot,
        dims,
        transfer,
        done,
    }
}
