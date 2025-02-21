#![allow(clippy::type_complexity)]

use core::fmt::Debug;
use core::hash::Hash;
use std::collections::{hash_map::Entry, HashMap};

use enum_map::EnumMap;
use itertools::Itertools;
use prjcombine_re_xilinx_cpld::{
    bits::{
        BankBits, Bits, EnumData, FbBits, IBufOut, IPadBits, InvBit, McBits, McOut, PlaAndTerm,
        PtAlloc, PtData,
    },
    device::DeviceKind,
    types::{
        CeMuxVal, ClkMuxVal, ClkPadId, ExportDir, FbGroupId, FclkId, FoeId, FoeMuxVal, IBufMode,
        ImuxId, ImuxInput, OeMode, OeMuxVal, OePadId, PTermId, RegMode, Slew, SrMuxVal, TermMode,
        Ut, Xc9500McPt, XorMuxVal,
    },
    vm6::NodeKind,
};
use prjcombine_types::{FbId, FbMcId, IoId};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::backend::{CpldBackend, FuzzerInfo, Iostd, State};

fn invbit(b: InvBit) -> InvBit {
    (b.0, !b.1)
}

fn invbits(b: HashMap<usize, bool>) -> HashMap<usize, bool> {
    b.into_iter().map(invbit).collect()
}

fn extract_single_invbit(bits: &HashMap<usize, bool>) -> (usize, bool) {
    assert_eq!(bits.len(), 1);
    let (&b, &p) = bits.iter().next().unwrap();
    (b, p)
}

fn extract_enum<K: Clone + Debug + Eq + PartialEq + Hash>(
    items: &[(K, HashMap<usize, bool>)],
) -> EnumData<K> {
    let mut bitvals = HashMap::new();
    for (_, bits) in items {
        for (&k, &v) in bits {
            match bitvals.entry(k) {
                Entry::Occupied(e) => assert_eq!(*e.get(), v),
                Entry::Vacant(e) => {
                    e.insert(v);
                }
            }
        }
    }
    let mut bits: Vec<_> = bitvals.keys().copied().collect();
    bits.sort();
    let default = bits.iter().map(|&bit| !bitvals[&bit]).collect();
    let items = items
        .iter()
        .map(|(k, ibits)| {
            let ibits = bits
                .iter()
                .map(|&bit| !bitvals[&bit] ^ ibits.contains_key(&bit))
                .collect();
            (k.clone(), ibits)
        })
        .collect();
    EnumData {
        bits,
        items,
        default,
    }
}

fn apply_diff(tgt: &mut HashMap<usize, bool>, src: &HashMap<usize, bool>) {
    for (&k, &v) in src {
        match tgt.entry(k) {
            Entry::Occupied(e) => {
                assert_eq!(*e.get(), !v);
                e.remove();
            }
            Entry::Vacant(e) => {
                e.insert(v);
            }
        }
    }
}

impl State {
    fn collect(&mut self, fi: FuzzerInfo) -> HashMap<usize, bool> {
        self.fuzzers
            .remove(&fi)
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
    }

    fn peek(&self, fi: FuzzerInfo) -> &HashMap<usize, bool> {
        &self.fuzzers[&fi][0]
    }

    fn peek_union(&self, fi: &[FuzzerInfo]) -> HashMap<usize, bool> {
        let mut res = self.peek(fi[0]).clone();
        for &fi in &fi[1..] {
            let b = self.peek(fi);
            res.retain(|k, _| b.contains_key(k))
        }
        res
    }

    fn collect_single(&mut self, fi: FuzzerInfo) -> InvBit {
        extract_single_invbit(&self.collect(fi))
    }

    fn collect_many(&mut self, fi: FuzzerInfo) -> Vec<InvBit> {
        self.collect(fi).into_iter().sorted().collect()
    }

    fn extract_single_common_keep(&mut self, fi: &[FuzzerInfo]) -> InvBit {
        let mut bits = self.fuzzers[&fi[0]][0].clone();
        for f in fi {
            let nbits = &self.fuzzers[f][0];
            bits.retain(|k, _| nbits.contains_key(k));
        }
        extract_single_invbit(&bits)
    }

    fn extract_single_common(&mut self, fi: &[FuzzerInfo]) -> InvBit {
        let bit = self.extract_single_common_keep(fi);
        for &f in fi {
            self.kill_fuzzer_bit(f, bit);
        }
        bit
    }

    fn collect_empty(&mut self, fi: FuzzerInfo) {
        let data = self.collect(fi);
        if !data.is_empty() {
            panic!("FUZZER {fi:?} not empty: {data:?}");
        }
    }

    fn collect_enum<K: Clone + Debug + Eq + PartialEq + Hash>(
        &mut self,
        items: &[(FuzzerInfo, K)],
    ) -> EnumData<K> {
        let items: Vec<_> = items
            .iter()
            .map(|&(fi, ref k)| (k.clone(), self.collect(fi)))
            .collect();
        extract_enum(&items)
    }

    fn kill_fuzzer_bit(&mut self, fi: FuzzerInfo, bit: InvBit) {
        assert_eq!(
            self.fuzzers.get_mut(&fi).unwrap()[0].remove(&bit.0),
            Some(bit.1)
        );
    }

    fn kill_fuzzer_bits_from(&mut self, fi: FuzzerInfo, fis: FuzzerInfo) {
        for (k, v) in self.fuzzers[&fis][0].clone() {
            self.kill_fuzzer_bit(fi, (k, v));
        }
    }

    fn kill_fuzzer_bits_enum<K: Clone + Debug + Eq + PartialEq + Hash>(
        &mut self,
        fi: FuzzerInfo,
        enm: &EnumData<K>,
        key: &K,
    ) {
        let val = &enm.items[key];
        for (i, &bit) in enm.bits.iter().enumerate() {
            if val[i] != enm.default[i] {
                self.kill_fuzzer_bit(fi, (bit, val[i]));
            }
        }
    }

    fn kill_fuzzer_bits_enum_diff<K: Clone + Debug + Eq + PartialEq + Hash>(
        &mut self,
        fi: FuzzerInfo,
        enm: &EnumData<K>,
        key_f: &K,
        key_t: &K,
    ) {
        let val_f = &enm.items[key_f];
        let val_t = &enm.items[key_t];
        for (i, &bit) in enm.bits.iter().enumerate() {
            if val_f[i] != val_t[i] {
                self.kill_fuzzer_bit(fi, (bit, val_t[i]));
            }
        }
    }
}

struct Collector<'a> {
    backend: &'a CpldBackend<'a>,
    state: &'a mut State,
    bits: &'a mut Bits,
}

impl Collector<'_> {
    fn clk_mux_srcs(&self) -> Vec<ClkMuxVal> {
        let mut res = vec![ClkMuxVal::Pt];
        let ng = if self.backend.device.kind == DeviceKind::Xpla3 {
            2
        } else {
            3
        };
        for i in 0..ng {
            res.push(ClkMuxVal::Fclk(FclkId::from_idx(i)));
        }
        match self.backend.device.kind {
            DeviceKind::Xpla3 => {
                res.push(ClkMuxVal::Ut);
                for i in 4..8 {
                    let pt = PTermId::from_idx(i);
                    res.push(ClkMuxVal::Ct(pt));
                }
            }
            DeviceKind::Coolrunner2 => {
                let pt = PTermId::from_idx(4);
                res.push(ClkMuxVal::Ct(pt));
            }
            _ => (),
        }
        res
    }

    fn collect_imux(&mut self) {
        for fbid in self.backend.device.fbs() {
            for (imid, imdata) in self.backend.imux {
                let mut items: Vec<_> = imdata
                    .keys()
                    .filter_map(|&inp| {
                        let fi = FuzzerInfo::Imux(fbid, imid, inp);
                        if let ImuxInput::Ibuf(mc) = inp {
                            if !self.backend.pin_map.contains_key(&mc) {
                                return None;
                            }
                        }
                        Some((fi, inp))
                    })
                    .collect();
                if self.backend.device.kind == DeviceKind::Xc9500 {
                    items.push((FuzzerInfo::Imux(fbid, imid, ImuxInput::Uim), ImuxInput::Uim));
                }
                let data = self.state.collect_enum(&items);
                self.bits.fbs[fbid].imux.push(data);
            }
        }
    }

    fn collect_uim_mc(&mut self) {
        for fbid in self.backend.device.fbs() {
            for imid in self.backend.device.fb_imuxes() {
                let data = self
                    .backend
                    .device
                    .fbs()
                    .map(|ifbid| {
                        self.backend
                            .device
                            .fb_mcs()
                            .map(|imcid| {
                                let bit = self.state.collect_single(FuzzerInfo::ImuxUimMc(
                                    fbid,
                                    imid,
                                    (ifbid, imcid),
                                ));
                                self.state.kill_fuzzer_bit(
                                    FuzzerInfo::Imux(fbid, imid, ImuxInput::Uim),
                                    invbit(bit),
                                );
                                bit
                            })
                            .collect()
                    })
                    .collect();
                self.bits.fbs[fbid].uim_mc.push(data);
            }
        }
    }

    fn collect_pt(&mut self) {
        for mc in self.backend.device.mcs() {
            let mcd = EnumMap::from_fn(|pt| {
                let and: EntityVec<ImuxId, (InvBit, InvBit)> = self
                    .backend
                    .device
                    .fb_imuxes()
                    .map(|imid| {
                        (
                            self.state
                                .collect_single(FuzzerInfo::McPTermImux(mc, pt, imid, true)),
                            self.state
                                .collect_single(FuzzerInfo::McPTermImux(mc, pt, imid, false)),
                        )
                    })
                    .collect();
                let f_d2 = FuzzerInfo::McOrTerm(mc, NodeKind::McSiD2, pt);
                let f_exp = FuzzerInfo::McOrTerm(mc, NodeKind::McSiExport, pt);
                let f_spec = FuzzerInfo::McSiSpec(mc, pt);
                let and_bit = and[ImuxId::from_idx(0)].0;
                self.state.kill_fuzzer_bit(f_d2, and_bit);
                self.state.kill_fuzzer_bit(f_exp, and_bit);
                self.state.kill_fuzzer_bit(f_spec, and_bit);
                let hp = self.state.extract_single_common(&[f_d2, f_exp, f_spec]);
                let af = vec![
                    (PtAlloc::OrMain, self.state.collect(f_d2)),
                    (PtAlloc::OrExport, self.state.collect(f_exp)),
                    (
                        PtAlloc::Special,
                        match pt {
                            Xc9500McPt::Clk => {
                                let mut bits = self.state.peek(f_spec).clone();
                                for i in 0..3 {
                                    let obits = self.state.peek(FuzzerInfo::McClk(
                                        mc,
                                        ClkMuxVal::Fclk(FclkId::from_idx(i)),
                                        false,
                                    ));
                                    for k in obits.keys() {
                                        bits.remove(k);
                                    }
                                }
                                bits
                            }
                            Xc9500McPt::Oe if self.backend.device.kind == DeviceKind::Xc9500 => {
                                let mut bits = self.state.peek(f_spec).clone();
                                for k in self.state.peek(FuzzerInfo::McUimOut(mc)).keys() {
                                    bits.remove(k);
                                }
                                bits
                            }
                            _ => self.state.collect(f_spec),
                        },
                    ),
                ];
                let alloc = extract_enum(&af);
                PtData { and, hp, alloc }
            });

            for pt in [Xc9500McPt::Clk, Xc9500McPt::Oe] {
                if pt == Xc9500McPt::Oe && self.backend.device.kind != DeviceKind::Xc9500 {
                    continue;
                }
                let f = FuzzerInfo::McSiSpec(mc, pt);
                self.state
                    .kill_fuzzer_bits_enum(f, &mcd[pt].alloc, &PtAlloc::Special);
            }

            {
                // Clean up CLK.
                let f = FuzzerInfo::McClk(mc, ClkMuxVal::Pt, false);
                let pt = &mcd[Xc9500McPt::Clk];
                self.state.kill_fuzzer_bit(f, pt.and[ImuxId::from_idx(0)].0);
                self.state.kill_fuzzer_bit(f, pt.hp);
                self.state
                    .kill_fuzzer_bits_enum(f, &pt.alloc, &PtAlloc::Special);

                let fs = FuzzerInfo::McSiSpec(mc, Xc9500McPt::Clk);
                self.state.kill_fuzzer_bits_from(fs, f);
                self.state.collect_empty(fs);
            }

            {
                // Clean up RST and SET.
                let f = FuzzerInfo::McRst(mc, SrMuxVal::Pt);
                let pt = &mcd[Xc9500McPt::Rst];
                self.state.kill_fuzzer_bit(f, pt.and[ImuxId::from_idx(0)].0);
                self.state.kill_fuzzer_bit(f, pt.hp);
                self.state
                    .kill_fuzzer_bits_enum(f, &pt.alloc, &PtAlloc::Special);
                if self.backend.device.kind != DeviceKind::Xc9500 {
                    let fe = FuzzerInfo::McCeRst(mc);
                    self.state
                        .kill_fuzzer_bit(fe, pt.and[ImuxId::from_idx(0)].0);
                    self.state.kill_fuzzer_bit(fe, pt.hp);
                    self.state
                        .kill_fuzzer_bits_enum(fe, &pt.alloc, &PtAlloc::Special);
                }

                let f = FuzzerInfo::McSet(mc, SrMuxVal::Pt);
                let pt = &mcd[Xc9500McPt::Set];
                self.state.kill_fuzzer_bit(f, pt.and[ImuxId::from_idx(0)].0);
                self.state.kill_fuzzer_bit(f, pt.hp);
                self.state
                    .kill_fuzzer_bits_enum(f, &pt.alloc, &PtAlloc::Special);
                if self.backend.device.kind != DeviceKind::Xc9500 {
                    let fe = FuzzerInfo::McCeSet(mc);
                    self.state
                        .kill_fuzzer_bit(fe, pt.and[ImuxId::from_idx(0)].0);
                    self.state.kill_fuzzer_bit(fe, pt.hp);
                    self.state
                        .kill_fuzzer_bits_enum(fe, &pt.alloc, &PtAlloc::Special);
                }
            }

            {
                // Clean up OE.
                let f = FuzzerInfo::McOe(mc, OeMuxVal::Pt, false);
                let pt = &mcd[Xc9500McPt::Oe];
                self.state.kill_fuzzer_bit(f, pt.and[ImuxId::from_idx(0)].0);
                self.state.kill_fuzzer_bit(f, pt.hp);
                self.state
                    .kill_fuzzer_bits_enum(f, &pt.alloc, &PtAlloc::Special);
                if self.backend.device.kind == DeviceKind::Xc9500
                    && self.backend.pin_map.contains_key(&IoId::Mc(mc))
                {
                    let fo = FuzzerInfo::OBufOe(mc, OeMuxVal::Pt, false);
                    self.state
                        .kill_fuzzer_bit(fo, pt.and[ImuxId::from_idx(0)].0);
                    self.state.kill_fuzzer_bit(fo, pt.hp);
                    self.state
                        .kill_fuzzer_bits_enum(fo, &pt.alloc, &PtAlloc::Special);
                }
            }

            {
                // Clean up XOR.
                let f1 = FuzzerInfo::McInputD2(mc);
                let f2 = FuzzerInfo::McInputD2B(mc);
                let f3 = FuzzerInfo::McInputXor(mc);
                let f4 = FuzzerInfo::McInputXorB(mc);
                self.state.kill_fuzzer_bits_from(f2, f1);
                self.state.kill_fuzzer_bits_from(f4, f3);
                self.state.kill_fuzzer_bits_from(f3, f1);
                self.state.kill_fuzzer_bits_from(f4, f2);
                self.state.collect_empty(f4);
                self.state.kill_fuzzer_bit(f3, mcd[Xc9500McPt::Xor].hp);
                self.state
                    .kill_fuzzer_bit(f3, mcd[Xc9500McPt::Xor].and[ImuxId::from_idx(0)].0);
                self.state.kill_fuzzer_bits_enum(
                    f3,
                    &mcd[Xc9500McPt::Xor].alloc,
                    &PtAlloc::Special,
                );
                self.state.collect_empty(f3);
                self.state
                    .kill_fuzzer_bit(f1, mcd[Xc9500McPt::Clk].and[ImuxId::from_idx(0)].0);
                self.state.kill_fuzzer_bit(f1, mcd[Xc9500McPt::Clk].hp);
                self.state
                    .kill_fuzzer_bits_enum(f1, &mcd[Xc9500McPt::Clk].alloc, &PtAlloc::OrMain);
                self.state.collect_empty(f1);
            }

            self.bits.fbs[mc.0].mcs[mc.1].pt = Some(mcd);
        }
    }

    fn collect_exp_en(&mut self) {
        for fb in self.backend.device.fbs() {
            self.bits.fbs[fb].exp_en = Some(self.state.extract_single_common_keep(&[
                FuzzerInfo::McOrExp((fb, FbMcId::from_idx(0)), NodeKind::McSiD2, ExportDir::Up),
                FuzzerInfo::McOrExp((fb, FbMcId::from_idx(0)), NodeKind::McSiD2, ExportDir::Down),
            ]));
        }
    }

    fn collect_exp_down(&mut self) {
        for mc in self.backend.device.mcs() {
            let smc = self.backend.device.export_target(mc, ExportDir::Down);
            let fmu = FuzzerInfo::McOrExp(smc, NodeKind::McSiD2, ExportDir::Up);
            let fmd = FuzzerInfo::McOrExp(smc, NodeKind::McSiD2, ExportDir::Down);
            let feu = FuzzerInfo::McOrExp(smc, NodeKind::McSiExport, ExportDir::Up);
            let fed = FuzzerInfo::McOrExp(smc, NodeKind::McSiExport, ExportDir::Down);
            let mcd = self.backend.device.export_target(smc, ExportDir::Down);
            let mcu = self.backend.device.export_target(smc, ExportDir::Up);
            for f in [fmu, feu] {
                let ptd = &self.bits.fbs[mcd.0].mcs[mcd.1].pt.as_ref().unwrap()[Xc9500McPt::Clk];
                self.state
                    .kill_fuzzer_bit(f, ptd.and[ImuxId::from_idx(0)].0);
                self.state.kill_fuzzer_bit(f, ptd.hp);
                self.state
                    .kill_fuzzer_bits_enum(f, &ptd.alloc, &PtAlloc::OrExport);
            }
            for f in [fmd, fed] {
                let ptu = &self.bits.fbs[mcu.0].mcs[mcu.1].pt.as_ref().unwrap()[Xc9500McPt::Clk];
                self.state
                    .kill_fuzzer_bit(f, ptu.and[ImuxId::from_idx(0)].0);
                self.state.kill_fuzzer_bit(f, ptu.hp);
                self.state
                    .kill_fuzzer_bits_enum(f, &ptu.alloc, &PtAlloc::OrExport);
            }
            let dir = self
                .state
                .collect_enum(&[(feu, ExportDir::Up), (fed, ExportDir::Down)]);
            self.state.kill_fuzzer_bits_enum(fmu, &dir, &ExportDir::Up);
            self.state
                .kill_fuzzer_bits_enum(fmd, &dir, &ExportDir::Down);
            self.state
                .kill_fuzzer_bit(fmu, self.bits.fbs[mc.0].exp_en.unwrap());
            self.state
                .kill_fuzzer_bit(fmd, self.bits.fbs[mc.0].exp_en.unwrap());
            self.bits.fbs[mc.0].mcs[mc.1].exp_dir = Some(dir);
        }
    }

    fn collect_import(&mut self) {
        for mc in self.backend.device.mcs() {
            self.bits.fbs[mc.0].mcs[mc.1].import = Some(EnumMap::from_fn(|k| {
                self.state
                    .collect_single(FuzzerInfo::McOrExp(mc, NodeKind::McSiD2, k))
            }));
        }
    }

    fn collect_mc_inv(&mut self) {
        for mc in self.backend.device.mcs() {
            self.bits.fbs[mc.0].mcs[mc.1].inv =
                Some(self.state.collect_single(FuzzerInfo::McInputD2B(mc)));
        }
    }

    fn collect_mc_lp(&mut self) {
        for mc in self.backend.device.mcs() {
            self.bits.fbs[mc.0].mcs[mc.1].hp = Some(invbit(
                self.state.collect_single(FuzzerInfo::McLowPower(mc)),
            ));
        }
    }

    fn collect_ff_en(&mut self) {
        for mc in self.backend.device.mcs() {
            let f: Vec<_> = self
                .clk_mux_srcs()
                .into_iter()
                .map(|src| FuzzerInfo::McClk(mc, src, false))
                .collect();
            let bit = self.state.extract_single_common(&f);
            self.bits.fbs[mc.0].mcs[mc.1].ff_en = Some(bit);
        }
    }

    fn collect_usercode(&mut self) {
        self.bits.usercode = Some(core::array::from_fn(|i| {
            self.state.collect_single(FuzzerInfo::Usercode(i as u8))
        }))
    }

    fn collect_pla_and(&mut self) {
        for fbid in self.backend.device.fbs() {
            for ptid in self.backend.device.fb_pterms() {
                let mut term = PlaAndTerm {
                    imux: EntityVec::new(),
                    fbn: EntityVec::new(),
                };
                for imid in self.backend.device.fb_imuxes() {
                    term.imux.push((
                        self.state
                            .collect_single(FuzzerInfo::PlaPTermImux(fbid, ptid, imid, true)),
                        self.state
                            .collect_single(FuzzerInfo::PlaPTermImux(fbid, ptid, imid, false)),
                    ));
                }
                if self.backend.device.kind == DeviceKind::Xpla3 {
                    for fbnid in self.backend.device.fb_fbns() {
                        term.fbn.push(
                            self.state
                                .collect_single(FuzzerInfo::PlaPTermFbn(fbid, ptid, fbnid)),
                        );
                    }
                }
                self.bits.fbs[fbid].pla_and.push(term);
            }
        }
    }

    fn collect_pla_or(&mut self) {
        for mc in self.backend.device.mcs() {
            for pt in self.backend.device.fb_pterms() {
                let bit = self.state.collect_single(FuzzerInfo::McOrPla(mc, pt));
                self.bits.fbs[mc.0].mcs[mc.1].pla_or.push(bit);
            }
        }
    }

    fn collect_ct_invert(&mut self) {
        for fb in self.backend.device.fbs() {
            for idx in 0..8 {
                let ptid = PTermId::from_idx(idx);
                let bit = self.state.collect_single(FuzzerInfo::CtInvert(fb, ptid));
                self.bits.fbs[fb].ct_invert.insert(ptid, bit);
            }
        }
    }

    fn collect_mc_lut(&mut self) {
        for mc in self.backend.device.mcs() {
            let f1 = FuzzerInfo::McInputD1(mc);
            let f1b = FuzzerInfo::McInputD1B(mc);
            let f2 = FuzzerInfo::McInputD2(mc);
            let f2b = FuzzerInfo::McInputD2B(mc);
            let fx = FuzzerInfo::McInputXor(mc);
            let fxb = FuzzerInfo::McInputXorB(mc);
            for f in [f2, f2b, fx, fxb] {
                self.state.kill_fuzzer_bit(
                    f,
                    self.bits.fbs[mc.0].mcs[mc.1].pla_or[PTermId::from_idx(0)],
                );
            }
            let bits = [
                self.state.extract_single_common(&[f1b, f2b, fxb]),
                self.state.extract_single_common(&[f1b, f2, fx]),
                self.state.extract_single_common(&[f1, f2b, fx]),
                self.state.extract_single_common(&[f1, f2, fxb]),
            ];
            for f in [f1, f1b, f2, f2b, fx, fxb] {
                self.state.collect_empty(f);
            }
            self.bits.fbs[mc.0].mcs[mc.1].lut = Some(bits);
        }
    }

    fn collect_mc_xor_mux(&mut self) {
        for mc in self.backend.device.mcs() {
            let f = [
                (FuzzerInfo::McInputD2(mc), XorMuxVal::Gnd),
                (FuzzerInfo::McInputD2B(mc), XorMuxVal::Vcc),
                (FuzzerInfo::McInputXor(mc), XorMuxVal::Pt),
                (FuzzerInfo::McInputXorB(mc), XorMuxVal::PtInv),
            ];
            for &(fi, _) in &f {
                let bit = self.bits.fbs[mc.0].mcs[mc.1].pla_or[PTermId::from_idx(0)];

                self.state.kill_fuzzer_bit(fi, bit);
            }
            self.bits.fbs[mc.0].mcs[mc.1].xor_mux = Some(self.state.collect_enum(&f));
        }
    }

    fn collect_mc_uim_out(&mut self) {
        for mc in self.backend.device.mcs() {
            let b_reg = self.state.collect(FuzzerInfo::McUimOut(mc));
            let mut b_comb_d = self.state.peek(FuzzerInfo::McComb(mc)).clone();
            let b_clk = self.state.peek(FuzzerInfo::McClk(mc, ClkMuxVal::Pt, false));
            b_comb_d.retain(|k, _| b_clk.contains_key(k));
            let mut b_comb = b_reg.clone();
            apply_diff(&mut b_comb, &b_comb_d);
            let f = [(McOut::Reg, b_reg), (McOut::Comb, b_comb)];
            let data = extract_enum(&f);
            self.state.kill_fuzzer_bits_enum_diff(
                FuzzerInfo::McComb(mc),
                &data,
                &McOut::Reg,
                &McOut::Comb,
            );
            let cmf: Vec<_> = self
                .clk_mux_srcs()
                .into_iter()
                .map(|src| FuzzerInfo::McClk(mc, src, false))
                .collect();
            for &f in &cmf {
                self.state
                    .kill_fuzzer_bits_enum_diff(f, &data, &McOut::Comb, &McOut::Reg);
            }
            if self.backend.device.kind == DeviceKind::Xpla3 {
                let bits = self.state.peek_union(&cmf);
                assert_eq!(
                    !bits.is_empty(),
                    self.backend.pin_map.contains_key(&IoId::Mc(mc))
                );
                if !bits.is_empty() {
                    let odata = extract_enum(&[(McOut::Reg, bits), (McOut::Comb, HashMap::new())]);
                    for &f in &cmf {
                        self.state
                            .kill_fuzzer_bits_enum_diff(f, &odata, &McOut::Comb, &McOut::Reg);
                    }
                    self.bits.fbs[mc.0].mcs[mc.1].mc_obuf_out = Some(odata);
                }
            }
            self.bits.fbs[mc.0].mcs[mc.1].mc_uim_out = Some(data);
        }
    }

    fn collect_no_isp(&mut self) {
        self.bits.no_isp = Some(self.state.collect_single(FuzzerInfo::NoIsp));
    }

    fn collect_mc_clk_inv(&mut self) {
        for mc in self.backend.device.mcs() {
            for src in self.clk_mux_srcs() {
                let f = FuzzerInfo::McClk(mc, src, false);
                let fb = FuzzerInfo::McClk(mc, src, true);
                self.state.kill_fuzzer_bits_from(fb, f);
                if let Some(bit) = self.bits.fbs[mc.0].mcs[mc.1].clk_inv {
                    self.state.kill_fuzzer_bit(fb, bit);
                    self.state.collect_empty(fb);
                } else {
                    self.bits.fbs[mc.0].mcs[mc.1].clk_inv = Some(self.state.collect_single(fb));
                }
            }
        }
    }

    fn collect_mc_oe_inv(&mut self) {
        let mut srcs = vec![OeMuxVal::Pt];
        for i in 0..self.backend.device.oe_pads.len() {
            srcs.push(OeMuxVal::Foe(FoeId::from_idx(i)));
        }
        for mc in self.backend.device.mcs() {
            for &src in &srcs {
                let f = FuzzerInfo::McOe(mc, src, false);
                let fb = FuzzerInfo::McOe(mc, src, true);
                self.state.kill_fuzzer_bits_from(fb, f);
                if let Some(bit) = self.bits.fbs[mc.0].mcs[mc.1].oe_inv {
                    self.state.kill_fuzzer_bit(fb, bit);
                    self.state.collect_empty(fb);
                } else {
                    self.bits.fbs[mc.0].mcs[mc.1].oe_inv = Some(self.state.collect_single(fb));
                }

                if self.backend.pin_map.contains_key(&IoId::Mc(mc)) {
                    let fo = FuzzerInfo::OBufOe(mc, src, false);
                    let fob = FuzzerInfo::OBufOe(mc, src, true);
                    let bit = self.bits.fbs[mc.0].mcs[mc.1].oe_inv.unwrap();
                    self.state.kill_fuzzer_bit(fo, invbit(bit));
                    self.state.kill_fuzzer_bits_from(fo, f);
                    self.state.kill_fuzzer_bits_from(fob, f);
                    self.state.collect_empty(fo);
                    self.state.collect_empty(fob);
                }
            }
        }
    }

    fn collect_mc_clk_mux(&mut self) {
        for mc in self.backend.device.mcs() {
            let fs: Vec<_> = self
                .clk_mux_srcs()
                .into_iter()
                .map(|src| (FuzzerInfo::McClk(mc, src, false), src))
                .collect();
            self.bits.fbs[mc.0].mcs[mc.1].clk_mux = self.state.collect_enum(&fs);
        }
    }

    fn collect_mc_sr_mux(&mut self) {
        for mc in self.backend.device.mcs() {
            for is_set in [false, true] {
                let srcs = match self.backend.device.kind {
                    DeviceKind::Xc9500 | DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                        vec![SrMuxVal::Pt, SrMuxVal::Fsr]
                    }
                    DeviceKind::Xpla3 => vec![
                        SrMuxVal::Ct(PTermId::from_idx(0)),
                        SrMuxVal::Ct(PTermId::from_idx(1)),
                        SrMuxVal::Ct(PTermId::from_idx(2)),
                        SrMuxVal::Ct(PTermId::from_idx(3)),
                        SrMuxVal::Ct(PTermId::from_idx(4)),
                        SrMuxVal::Ct(PTermId::from_idx(5)),
                        SrMuxVal::Ut,
                    ],
                    DeviceKind::Coolrunner2 => vec![
                        SrMuxVal::Pt,
                        SrMuxVal::Ct(PTermId::from_idx(if is_set { 6 } else { 5 })),
                        SrMuxVal::Fsr,
                    ],
                };
                let mut fs: Vec<_> = srcs
                    .into_iter()
                    .map(|k| {
                        (
                            k,
                            self.state.collect(if is_set {
                                FuzzerInfo::McSet(mc, k)
                            } else {
                                FuzzerInfo::McRst(mc, k)
                            }),
                        )
                    })
                    .collect();
                if !self.backend.device.kind.is_xc9500() {
                    fs.push((SrMuxVal::Gnd, HashMap::new()));
                }
                let data = extract_enum(&fs);
                if is_set {
                    self.bits.fbs[mc.0].mcs[mc.1].set_mux = data;
                } else {
                    self.bits.fbs[mc.0].mcs[mc.1].rst_mux = data;
                }
            }
        }
    }

    fn collect_mc_init(&mut self) {
        for mc in self.backend.device.mcs() {
            self.bits.fbs[mc.0].mcs[mc.1].init =
                Some(self.state.collect_single(FuzzerInfo::McInit(mc)));
        }
    }

    fn collect_mc_ddr(&mut self) {
        for mc in self.backend.device.mcs() {
            self.bits.fbs[mc.0].mcs[mc.1].ddr =
                Some(self.state.collect_single(FuzzerInfo::McDdr(mc)));
        }
    }

    fn collect_reg_mode(&mut self) {
        for mc in self.backend.device.mcs() {
            let mut f = vec![
                (RegMode::Dff, HashMap::new()),
                (RegMode::Tff, self.state.collect(FuzzerInfo::McTff(mc))),
            ];
            if !self.backend.device.kind.is_xc9500() {
                f.push((RegMode::Latch, self.state.collect(FuzzerInfo::McLatch(mc))));
                if self.backend.device.kind == DeviceKind::Xpla3 {
                    let mut bits = self.state.peek(FuzzerInfo::McCePt(mc)).clone();
                    let bits2 = self.state.peek(FuzzerInfo::McCeCt(mc));
                    bits.retain(|k, _| bits2.contains_key(k));
                    f.push((RegMode::DffCe, bits));
                } else {
                    f.push((RegMode::DffCe, self.state.collect(FuzzerInfo::McCePt(mc))));
                }
            }
            self.bits.fbs[mc.0].mcs[mc.1].reg_mode = extract_enum(&f);
        }
    }

    fn collect_ce_mux_xc9500x(&mut self) {
        for mc in self.backend.device.mcs() {
            let f = vec![
                (FuzzerInfo::McCeRst(mc), CeMuxVal::PtRst),
                (FuzzerInfo::McCeSet(mc), CeMuxVal::PtSet),
            ];
            self.bits.fbs[mc.0].mcs[mc.1].ce_mux = Some(self.state.collect_enum(&f));
        }
    }

    fn collect_ce_mux_xpla3(&mut self) {
        for mc in self.backend.device.mcs() {
            let f = vec![
                (FuzzerInfo::McCePt(mc), CeMuxVal::Pt),
                (FuzzerInfo::McCeCt(mc), CeMuxVal::Ct(PTermId::from_idx(4))),
            ];
            for &(fi, _) in &f {
                self.state.kill_fuzzer_bits_enum(
                    fi,
                    &self.bits.fbs[mc.0].mcs[mc.1].reg_mode,
                    &RegMode::DffCe,
                );
            }
            self.bits.fbs[mc.0].mcs[mc.1].ce_mux = Some(self.state.collect_enum(&f));
        }
    }

    fn collect_use_ireg(&mut self) {
        for mc in self.backend.device.mcs() {
            if self.backend.pin_map.contains_key(&IoId::Mc(mc)) {
                self.bits.fbs[mc.0].mcs[mc.1].use_ireg =
                    Some(self.state.collect_single(FuzzerInfo::McInputIreg(mc)));
            }
        }
    }

    fn collect_ut(&mut self) {
        let ut = EnumMap::from_fn(|ut| {
            let mut f = vec![];
            for fb in self.backend.device.fbs() {
                for pt in [PTermId::from_idx(6), PTermId::from_idx(7)] {
                    if pt.to_idx() == 6 && self.backend.device.fbs != 2 {
                        continue;
                    }
                    let fi = FuzzerInfo::Ut(ut, fb, pt);
                    let bit = self.bits.fbs[fb].ct_invert[pt];
                    self.state.kill_fuzzer_bit(fi, invbit(bit));
                    let mc = (FbId::from_idx(fb.to_idx() ^ 1), FbMcId::from_idx(0));
                    match ut {
                        Ut::Clk => {
                            self.state.kill_fuzzer_bits_enum(
                                fi,
                                &self.bits.fbs[mc.0].mcs[mc.1].clk_mux,
                                &ClkMuxVal::Ut,
                            );
                            self.state.kill_fuzzer_bits_enum_diff(
                                fi,
                                self.bits.fbs[mc.0].mcs[mc.1].mc_uim_out.as_ref().unwrap(),
                                &McOut::Comb,
                                &McOut::Reg,
                            );
                            if self.backend.pin_map.contains_key(&IoId::Mc(mc)) {
                                self.state.kill_fuzzer_bits_enum_diff(
                                    fi,
                                    self.bits.fbs[mc.0].mcs[mc.1].mc_obuf_out.as_ref().unwrap(),
                                    &McOut::Comb,
                                    &McOut::Reg,
                                );
                            }
                        }
                        Ut::Oe => (),
                        Ut::Set => {
                            self.state.kill_fuzzer_bits_enum(
                                fi,
                                &self.bits.fbs[mc.0].mcs[mc.1].set_mux,
                                &SrMuxVal::Ut,
                            );
                        }
                        Ut::Rst => {
                            self.state.kill_fuzzer_bits_enum(
                                fi,
                                &self.bits.fbs[mc.0].mcs[mc.1].rst_mux,
                                &SrMuxVal::Ut,
                            );
                        }
                    }
                    f.push((fi, (fb, pt)));
                }
            }
            self.state.collect_enum(&f)
        });
        self.bits.ut = Some(ut);
    }

    fn collect_fbclk(&mut self) {
        for fb in self.backend.device.fbs() {
            let f = [
                [Some(ClkPadId::from_idx(0)), None],
                [None, Some(ClkPadId::from_idx(1))],
                [Some(ClkPadId::from_idx(2)), None],
                [None, Some(ClkPadId::from_idx(3))],
                [Some(ClkPadId::from_idx(0)), Some(ClkPadId::from_idx(1))],
                [Some(ClkPadId::from_idx(0)), Some(ClkPadId::from_idx(2))],
                [Some(ClkPadId::from_idx(0)), Some(ClkPadId::from_idx(3))],
                [Some(ClkPadId::from_idx(1)), Some(ClkPadId::from_idx(2))],
                [Some(ClkPadId::from_idx(1)), Some(ClkPadId::from_idx(3))],
                [Some(ClkPadId::from_idx(2)), Some(ClkPadId::from_idx(3))],
            ]
            .map(|[a, b]| (FuzzerInfo::FbClk(fb, a, b), (a, b)));
            for (fi, (a, b)) in f {
                for (mc, gclk, fclk) in [
                    (FbMcId::from_idx(0), a, FclkId::from_idx(0)),
                    (FbMcId::from_idx(1), b, FclkId::from_idx(1)),
                ] {
                    if gclk.is_some() {
                        self.state.kill_fuzzer_bits_enum_diff(
                            fi,
                            self.bits.fbs[fb].mcs[mc].mc_uim_out.as_ref().unwrap(),
                            &McOut::Comb,
                            &McOut::Reg,
                        );
                        if self.backend.pin_map.contains_key(&IoId::Mc((fb, mc))) {
                            self.state.kill_fuzzer_bits_enum_diff(
                                fi,
                                self.bits.fbs[fb].mcs[mc].mc_obuf_out.as_ref().unwrap(),
                                &McOut::Comb,
                                &McOut::Reg,
                            );
                        }
                        self.state.kill_fuzzer_bits_enum(
                            fi,
                            &self.bits.fbs[fb].mcs[mc].clk_mux,
                            &ClkMuxVal::Fclk(fclk),
                        );
                    }
                }
            }
            self.bits.fbs[fb].fbclk = Some(self.state.collect_enum(&f));
        }
    }

    fn collect_ipad_uim_out(&mut self) {
        for ipad in self.backend.device.ipads() {
            let io = IoId::Ipad(ipad);
            let mut bits = EntityPartVec::new();
            for fb in self.backend.device.fbs() {
                let f = FuzzerInfo::IpadUimOutFb(ipad, fb);
                self.state.kill_fuzzer_bits_enum(
                    f,
                    &self.bits.fbs[fb].imux[self.backend.ibuf_test_imux[&io]],
                    &ImuxInput::Ibuf(io),
                );
                let fbg = self.backend.device.fb_group[fb];
                if let Some(&bit) = bits.get(fbg) {
                    self.state.kill_fuzzer_bit(f, bit);
                    self.state.collect_empty(f);
                } else {
                    bits.insert(fbg, self.state.collect_single(f));
                }
            }
            self.bits.ipads[ipad].uim_out_en = bits.into_full();
        }
    }

    fn collect_ibuf(&mut self) {
        for &io in self.backend.pin_map.keys() {
            let fp = FuzzerInfo::IBufPresent(io);
            let fpg = FuzzerInfo::IBufPresentGnd(io);
            let fpp = FuzzerInfo::IBufPresentPullup(io);
            let fpk = FuzzerInfo::IBufPresentKeeper(io);

            self.state.kill_fuzzer_bits_enum(
                fp,
                &self.bits.fbs[FbId::from_idx(0)].imux[self.backend.ibuf_test_imux[&io]],
                &ImuxInput::Ibuf(io),
            );

            if self.backend.device.kind != DeviceKind::Xpla3 {
                self.state.kill_fuzzer_bits_enum(
                    fpg,
                    &self.bits.fbs[FbId::from_idx(0)].imux[self.backend.ibuf_test_imux[&io]],
                    &ImuxInput::Ibuf(io),
                );
            }
            if !self.backend.device.kind.is_xc9500() {
                self.state.kill_fuzzer_bits_enum(
                    fpp,
                    &self.bits.fbs[FbId::from_idx(0)].imux[self.backend.ibuf_test_imux[&io]],
                    &ImuxInput::Ibuf(io),
                );
            }
            if self.backend.device.kind == DeviceKind::Coolrunner2 {
                self.state.kill_fuzzer_bits_enum(
                    fpk,
                    &self.bits.fbs[FbId::from_idx(0)].imux[self.backend.ibuf_test_imux[&io]],
                    &ImuxInput::Ibuf(io),
                );
            }

            match io {
                IoId::Ipad(ip) => {
                    if self.backend.device.kind == DeviceKind::Coolrunner2 {
                        self.state.collect_empty(fp);
                        self.state.collect_empty(fpg);
                        let term_n = self.state.collect_single(fpp);
                        self.state.kill_fuzzer_bit(fpk, term_n);
                        self.state.collect_empty(fpk);
                        let term = invbit(term_n);
                        self.bits.ipads[ip].term = Some(term);

                        let ds = self.state.collect(FuzzerInfo::IBufSchmitt(io));
                        let f = vec![
                            (IBufMode::Plain, HashMap::new()),
                            (IBufMode::Schmitt, ds.clone()),
                        ];
                        let data = extract_enum(&f);
                        self.bits.ipads[ip].ibuf_mode = Some(data);
                    } else {
                        let en = self.bits.ipads[ip].uim_out_en[FbGroupId::from_idx(0)];
                        self.state.kill_fuzzer_bit(fp, en);
                        self.state.collect_empty(fp);
                        self.state.kill_fuzzer_bit(fpp, en);
                        self.state.collect_empty(fpp);
                    }
                }
                IoId::Mc(mc) => {
                    let mcd = &mut self.bits.fbs[mc.0].mcs[mc.1];
                    if self.backend.device.kind.is_xc9500() {
                        self.state.kill_fuzzer_bits_from(fpg, fp);
                        if self.backend.device.kind == DeviceKind::Xc9500
                            && self.backend.device.fbs == 16
                        {
                            mcd.ibuf_uim_en = self.state.collect_many(fp);
                        } else {
                            self.state.collect_empty(fp);
                        }
                        mcd.is_gnd = Some(invbit(
                            self.state.collect_single(FuzzerInfo::IBufPresentGnd(io)),
                        ));
                    } else {
                        let fc = FuzzerInfo::McComb(mc);
                        let f = [(fc, IBufOut::Reg), (fp, IBufOut::Pad)];
                        let data = self.state.collect_enum(&f);
                        self.state.kill_fuzzer_bits_enum(fpp, &data, &IBufOut::Pad);
                        if self.backend.device.kind == DeviceKind::Coolrunner2 {
                            self.state.kill_fuzzer_bits_enum(fpg, &data, &IBufOut::Pad);
                            self.state.kill_fuzzer_bits_enum(fpk, &data, &IBufOut::Pad);
                            let term_n = self.state.collect_single(fpp);
                            self.state.kill_fuzzer_bit(fpk, term_n);
                            self.state.collect_empty(fpk);
                            let term = invbit(term_n);
                            mcd.term = Some(term);
                        }
                        mcd.ibuf_uim_out = Some(data);
                    }

                    if self.backend.device.kind == DeviceKind::Coolrunner2 {
                        let ds = self.state.collect(FuzzerInfo::IBufSchmitt(io));
                        let mut f = vec![
                            (IBufMode::Plain, HashMap::new()),
                            (IBufMode::Schmitt, ds.clone()),
                        ];
                        if self.backend.device.has_vref {
                            let mut ivr = ds;
                            apply_diff(&mut ivr, &self.state.collect(FuzzerInfo::IBufIsVref(io)));
                            f.extend([
                                (
                                    IBufMode::UseVref,
                                    self.state.collect(FuzzerInfo::IBufUseVref(io)),
                                ),
                                (IBufMode::IsVref, ivr),
                            ]);
                        }
                        let data = extract_enum(&f);
                        mcd.ibuf_mode = Some(data);
                        if self.backend.device.dge_pad.is_some() {
                            let bit = self.state.collect_single(FuzzerInfo::IBufDge(io));
                            mcd.dge_en = Some(bit);
                        }
                    }
                }
            }
        }
        for mc in self.backend.device.mcs() {
            if self.backend.pin_map.contains_key(&IoId::Mc(mc)) {
                continue;
            }
            if !self.backend.device.kind.is_xc9500() {
                let fc = FuzzerInfo::McComb(mc);
                if self.backend.device.io.contains_key(&IoId::Mc(mc)) {
                    let f = [(fc, IBufOut::Reg)];
                    self.bits.fbs[mc.0].mcs[mc.1].ibuf_uim_out = Some(self.state.collect_enum(&f));
                } else {
                    self.state.collect_empty(fc);
                }
            }
        }
        if self.backend.device.kind.is_xc9500x() {
            let f = [(
                TermMode::Keeper,
                self.state.collect(FuzzerInfo::GlobalKeeper),
            )];
            self.bits.term_mode = Some(extract_enum(&f));
        }

        if self.backend.device.kind == DeviceKind::Coolrunner2 {
            let f = [
                (TermMode::Pullup, HashMap::new()),
                (
                    TermMode::Keeper,
                    self.state.collect(FuzzerInfo::GlobalKeeper),
                ),
            ];
            self.bits.term_mode = Some(extract_enum(&f));

            for (bank, &mc) in &self.backend.bank_test_iob {
                for iostd in [Iostd::Lvcmos15, Iostd::Lvcmos18, Iostd::Lvcmos18Any] {
                    self.state.collect_empty(FuzzerInfo::IBufIostd(bank, iostd));
                    self.state.collect_empty(FuzzerInfo::OBufIostd(bank, iostd));
                }
                let ibuf_hv = self
                    .state
                    .collect_single(FuzzerInfo::IBufIostd(bank, Iostd::Lvcmos25));
                let obuf_hv = self
                    .state
                    .collect_single(FuzzerInfo::OBufIostd(bank, Iostd::Lvcmos25));
                for iostd in [Iostd::Lvcmos33, Iostd::Lvttl] {
                    self.state
                        .kill_fuzzer_bit(FuzzerInfo::IBufIostd(bank, iostd), ibuf_hv);
                    self.state.collect_empty(FuzzerInfo::IBufIostd(bank, iostd));
                    self.state
                        .kill_fuzzer_bit(FuzzerInfo::OBufIostd(bank, iostd), obuf_hv);
                    self.state.collect_empty(FuzzerInfo::OBufIostd(bank, iostd));
                }
                self.bits.banks.push(BankBits { ibuf_hv, obuf_hv });
                if self.backend.device.has_vref {
                    for iostd in [Iostd::Sstl2I, Iostd::Sstl3I] {
                        self.state
                            .kill_fuzzer_bit(FuzzerInfo::OBufIostd(bank, iostd), obuf_hv);
                    }
                    for iostd in [Iostd::Sstl2I, Iostd::Sstl3I, Iostd::HstlI] {
                        let f = FuzzerInfo::IBufIostd(bank, iostd);
                        self.state.kill_fuzzer_bits_enum(
                            f,
                            self.bits.fbs[mc.0].mcs[mc.1].ibuf_mode.as_ref().unwrap(),
                            &IBufMode::UseVref,
                        );
                        if let Some(bit) = self.bits.vref_en {
                            self.state.kill_fuzzer_bit(f, bit);
                            self.state.collect_empty(f);
                        } else {
                            self.bits.vref_en = Some(self.state.collect_single(f));
                        }
                        self.state.collect_empty(FuzzerInfo::OBufIostd(bank, iostd));
                    }
                }
            }
            if let Some(io) = self.backend.device.dge_pad {
                let IoId::Mc(mc) = io else {
                    unreachable!();
                };
                let f = FuzzerInfo::Dge;
                self.state
                    .kill_fuzzer_bit(f, self.bits.fbs[mc.0].mcs[mc.1].dge_en.unwrap());
                self.bits.dge_en = Some(self.state.collect_single(f));
            }
        }
    }

    fn collect_oe(&mut self) {
        let mut srcs = vec![OeMuxVal::Pt];
        for i in 0..self.backend.device.oe_pads.len() {
            srcs.push(OeMuxVal::Foe(FoeId::from_idx(i)));
        }
        for mc in self.backend.device.mcs() {
            if self.backend.device.kind == DeviceKind::Xc9500 {
                let vcc = self.state.collect(FuzzerInfo::McUimOut(mc));
                let mut oe = vcc.clone();
                apply_diff(
                    &mut oe,
                    &self.state.collect(FuzzerInfo::McSiSpec(mc, Xc9500McPt::Oe)),
                );
                let f = [
                    (OeMode::Gnd, HashMap::new()),
                    (OeMode::Vcc, vcc),
                    (OeMode::McOe, oe),
                ];
                let uim_oe_mode = extract_enum(&f);
                for &src in &srcs {
                    self.state.kill_fuzzer_bits_enum_diff(
                        FuzzerInfo::McOe(mc, src, false),
                        &uim_oe_mode,
                        &OeMode::Vcc,
                        &OeMode::McOe,
                    );
                }
                if self.backend.pin_map.contains_key(&IoId::Mc(mc)) {
                    for &src in &srcs {
                        self.state.kill_fuzzer_bits_from(
                            FuzzerInfo::OBufOe(mc, src, false),
                            FuzzerInfo::McOe(mc, src, false),
                        );
                    }
                }
                self.bits.fbs[mc.0].mcs[mc.1].uim_oe_mode = Some(uim_oe_mode);
                self.bits.fbs[mc.0].mcs[mc.1].uim_out_inv =
                    Some(self.state.collect_single(FuzzerInfo::McUimOutInv(mc)));
            } else {
                self.state.collect_empty(FuzzerInfo::McUimOut(mc));
            }
            let f: Vec<_> = srcs
                .iter()
                .map(|&src| (FuzzerInfo::McOe(mc, src, false), src))
                .collect();
            self.bits.fbs[mc.0].mcs[mc.1].oe_mux = Some(self.state.collect_enum(&f));
        }
    }

    fn collect_obuf(&mut self) {
        for &io in self.backend.pin_map.keys() {
            let IoId::Mc(mc) = io else {
                continue;
            };
            let slew = extract_enum(&[
                (Slew::Slow, HashMap::new()),
                (Slew::Fast, self.state.collect(FuzzerInfo::OBufSlew(mc))),
            ]);

            let fpc = FuzzerInfo::OBufPresentComb(mc);
            let fpr = FuzzerInfo::OBufPresentReg(mc);
            self.state.kill_fuzzer_bits_from(fpr, fpc);
            if !self.backend.device.kind.is_xc9500() {
                self.state
                    .kill_fuzzer_bits_enum_diff(fpc, &slew, &Slew::Fast, &Slew::Slow);
            }
            if self.backend.device.kind == DeviceKind::Coolrunner2 {
                self.bits.fbs[mc.0].mcs[mc.1].mc_obuf_out = Some(extract_enum(&[
                    (McOut::Comb, HashMap::new()),
                    (McOut::Reg, self.state.collect(fpr)),
                ]));
            } else {
                self.state.collect_empty(fpr);
            }
            self.bits.fbs[mc.0].mcs[mc.1].slew = Some(slew);

            match self.backend.device.kind {
                DeviceKind::Xc9500 => {
                    let vcc = self.state.collect(fpc);
                    let mut oe = vcc.clone();
                    apply_diff(
                        &mut oe,
                        &self
                            .state
                            .collect(FuzzerInfo::OBufOe(mc, OeMuxVal::Pt, false)),
                    );
                    let f = [
                        (OeMode::Gnd, HashMap::new()),
                        (OeMode::Vcc, vcc),
                        (OeMode::McOe, oe),
                    ];
                    let obuf_oe_mode = extract_enum(&f);
                    for i in 0..self.backend.device.oe_pads.len() {
                        let fi = FuzzerInfo::OBufOe(mc, OeMuxVal::Foe(FoeId::from_idx(i)), false);
                        self.state.kill_fuzzer_bits_enum_diff(
                            fi,
                            &obuf_oe_mode,
                            &OeMode::Vcc,
                            &OeMode::McOe,
                        );
                        self.state.collect_empty(fi);
                    }
                    self.bits.fbs[mc.0].mcs[mc.1].obuf_oe_mode = Some(obuf_oe_mode);
                }
                DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                    let bit = self.bits.fbs[mc.0].mcs[mc.1].oe_inv.unwrap();
                    self.state.kill_fuzzer_bit(fpc, bit);
                    self.state.collect_empty(fpc);
                }
                DeviceKind::Xpla3 => {
                    let bpc = self.state.collect(fpc);
                    let mut f = vec![
                        (OeMuxVal::Gnd, HashMap::new()),
                        (
                            OeMuxVal::Pullup,
                            invbits(
                                self.state
                                    .collect(FuzzerInfo::IBufPresentPullup(IoId::Mc(mc))),
                            ),
                        ),
                        (OeMuxVal::Vcc, bpc.clone()),
                    ];
                    for v in [
                        OeMuxVal::Ut,
                        OeMuxVal::Ct(PTermId::from_idx(0)),
                        OeMuxVal::Ct(PTermId::from_idx(1)),
                        OeMuxVal::Ct(PTermId::from_idx(2)),
                        OeMuxVal::Ct(PTermId::from_idx(6)),
                    ] {
                        let mut b = bpc.clone();
                        apply_diff(
                            &mut b,
                            &self.state.collect(FuzzerInfo::OBufOe(mc, v, false)),
                        );
                        f.push((v, b));
                    }
                    self.bits.fbs[mc.0].mcs[mc.1].oe_mux = Some(extract_enum(&f));
                }
                DeviceKind::Coolrunner2 => {
                    let bpc = self.state.collect(fpc);
                    let mut od = bpc.clone();
                    apply_diff(&mut od, &self.state.collect(FuzzerInfo::OBufOpenDrain(mc)));
                    let mut f = vec![
                        (OeMuxVal::Gnd, HashMap::new()),
                        (
                            OeMuxVal::IsGround,
                            invbits(self.state.collect(FuzzerInfo::IBufPresentGnd(IoId::Mc(mc)))),
                        ),
                        (OeMuxVal::OpenDrain, od),
                        (OeMuxVal::Vcc, bpc.clone()),
                    ];
                    for v in [
                        OeMuxVal::Pt,
                        OeMuxVal::Ct(PTermId::from_idx(7)),
                        OeMuxVal::Foe(FoeId::from_idx(0)),
                        OeMuxVal::Foe(FoeId::from_idx(1)),
                        OeMuxVal::Foe(FoeId::from_idx(2)),
                        OeMuxVal::Foe(FoeId::from_idx(3)),
                    ] {
                        let mut b = bpc.clone();
                        apply_diff(
                            &mut b,
                            &self.state.collect(FuzzerInfo::OBufOe(mc, v, false)),
                        );
                        f.push((v, b));
                    }
                    self.bits.fbs[mc.0].mcs[mc.1].oe_mux = Some(extract_enum(&f));
                }
            }
        }
    }

    fn collect_clkdiv(&mut self) {
        if self.backend.device.cdr_pad.is_none() {
            return;
        }
        let f = [2, 4, 6, 8, 10, 12, 14, 16].map(|d| (FuzzerInfo::ClkDiv(d), d));
        self.bits.clkdiv_en = Some(self.state.extract_single_common(&f.map(|(f, _)| f)));
        self.bits.clkdiv_div = Some(self.state.collect_enum(&f));
        self.bits.clkdiv_dly_en = Some(self.state.collect_single(FuzzerInfo::ClkDivDelay));
    }

    fn collect_fclk(&mut self) {
        let mc = (FbId::from_idx(0), FbMcId::from_idx(0));
        let mc = &self.bits.fbs[mc.0].mcs[mc.1];
        if self.backend.device.kind == DeviceKind::Xc9500 {
            for (tgt, srcs) in [[0, 1], [1, 2], [2, 0]].into_iter().enumerate() {
                let tgt = FclkId::from_idx(tgt);
                let srcs = srcs.map(ClkPadId::from_idx);
                for src in srcs {
                    let ff = FuzzerInfo::Fclk(tgt, src, false);
                    let ft = FuzzerInfo::Fclk(tgt, src, true);
                    self.state.kill_fuzzer_bits_from(ft, ff);
                    self.state.kill_fuzzer_bit(ff, mc.ff_en.unwrap());
                    self.state
                        .kill_fuzzer_bits_enum(ff, &mc.clk_mux, &ClkMuxVal::Fclk(tgt));
                }
                let inv = self
                    .state
                    .collect_single(FuzzerInfo::Fclk(tgt, srcs[0], true));
                self.state
                    .kill_fuzzer_bit(FuzzerInfo::Fclk(tgt, srcs[1], true), inv);
                self.state
                    .collect_empty(FuzzerInfo::Fclk(tgt, srcs[1], true));
                self.bits.fclk_inv.push(inv);
                let f = srcs.map(|src| (FuzzerInfo::Fclk(tgt, src, false), src));
                self.bits.fclk_mux.push(self.state.collect_enum(&f));
            }
        } else {
            for i in 0..3 {
                let tgt = FclkId::from_idx(i);
                let src = ClkPadId::from_idx(i);
                let f = FuzzerInfo::Fclk(tgt, src, false);
                if self.backend.device.kind.is_xc9500() {
                    self.state.kill_fuzzer_bit(f, mc.ff_en.unwrap());
                } else {
                    self.state.kill_fuzzer_bits_enum_diff(
                        f,
                        mc.mc_uim_out.as_ref().unwrap(),
                        &McOut::Comb,
                        &McOut::Reg,
                    );
                }
                self.state
                    .kill_fuzzer_bits_enum(f, &mc.clk_mux, &ClkMuxVal::Fclk(tgt));
                self.bits.fclk_en.push(self.state.collect_single(f));
            }
        }
    }

    fn collect_fsr(&mut self) {
        let mc = (FbId::from_idx(0), FbMcId::from_idx(0));
        for inv in [false, true] {
            self.state.kill_fuzzer_bits_enum(
                FuzzerInfo::Fsr(inv),
                &self.bits.fbs[mc.0].mcs[mc.1].rst_mux,
                &SrMuxVal::Fsr,
            );
        }
        let ff = FuzzerInfo::Fsr(false);
        let ft = FuzzerInfo::Fsr(true);
        if self.backend.device.kind == DeviceKind::Coolrunner2 {
            let bit = self.state.collect_single(ft);
            self.state.kill_fuzzer_bit(ff, bit);
            self.bits.fsr_en = Some(bit);
            self.bits.fsr_inv = Some(invbit(self.state.collect_single(ff)));
        } else {
            self.state.collect_empty(ff);
            self.bits.fsr_inv = Some(self.state.collect_single(ft));
        }
    }

    fn collect_foe(&mut self) {
        let io = *self.backend.device.clk_pads.first().unwrap();
        let IoId::Mc(mc) = io else {
            unreachable!();
        };
        let mc = &self.bits.fbs[mc.0].mcs[mc.1];
        let num = self.backend.device.oe_pads.len();
        for tgt in 0..num {
            match self.backend.device.kind {
                DeviceKind::Xc9500 => {
                    let srcs = [tgt, (tgt + 1) % num].map(OePadId::from_idx);
                    let tgt = FoeId::from_idx(tgt);
                    for src in srcs {
                        let ff = FuzzerInfo::Foe(tgt, src, false);
                        let ft = FuzzerInfo::Foe(tgt, src, true);
                        self.state.kill_fuzzer_bits_from(ft, ff);
                        self.state.kill_fuzzer_bits_enum(
                            ff,
                            mc.oe_mux.as_ref().unwrap(),
                            &OeMuxVal::Foe(tgt),
                        );
                        self.state.kill_fuzzer_bits_enum_diff(
                            ff,
                            mc.obuf_oe_mode.as_ref().unwrap(),
                            &OeMode::Vcc,
                            &OeMode::McOe,
                        );
                    }
                    let inv = self
                        .state
                        .collect_single(FuzzerInfo::Foe(tgt, srcs[0], true));
                    self.state
                        .kill_fuzzer_bit(FuzzerInfo::Foe(tgt, srcs[1], true), inv);
                    self.state
                        .collect_empty(FuzzerInfo::Foe(tgt, srcs[1], true));

                    self.bits.foe_inv.push(inv);
                    let f = srcs.map(|src| (FuzzerInfo::Foe(tgt, src, false), src));
                    self.bits.foe_mux.push(self.state.collect_enum(&f));
                }
                DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                    let src = OePadId::from_idx(tgt);
                    let tgt = FoeId::from_idx(tgt);
                    let ff = FuzzerInfo::Foe(tgt, src, false);
                    self.state.kill_fuzzer_bits_enum(
                        ff,
                        mc.oe_mux.as_ref().unwrap(),
                        &OeMuxVal::Foe(tgt),
                    );
                    self.state.kill_fuzzer_bit(ff, invbit(mc.oe_inv.unwrap()));
                    self.bits.foe_en.push(self.state.collect_single(ff));
                }
                DeviceKind::Xpla3 => unreachable!(),
                DeviceKind::Coolrunner2 => {
                    let src = OePadId::from_idx(tgt);
                    let tgt = FoeId::from_idx(tgt);
                    let ff = FuzzerInfo::Foe(tgt, src, false);
                    let ft = FuzzerInfo::Foe(tgt, src, true);
                    let fm = FuzzerInfo::FoeMc(tgt);
                    for f in [ff, ft, fm] {
                        self.state.kill_fuzzer_bits_enum_diff(
                            f,
                            mc.oe_mux.as_ref().unwrap(),
                            &OeMuxVal::Vcc,
                            &OeMuxVal::Foe(tgt),
                        );
                    }
                    let f = [
                        (ff, FoeMuxVal::Ibuf),
                        (ft, FoeMuxVal::IbufInv),
                        (fm, FoeMuxVal::Mc),
                    ];
                    self.bits.foe_mux_xbr.push(self.state.collect_enum(&f));
                }
            }
        }
    }
    fn collect_fb(&mut self) {
        for fb in self.backend.device.fbs() {
            let f = FuzzerInfo::FbPresent(fb);
            if self.backend.device.kind == DeviceKind::Xc9500 {
                for mc in self.bits.fbs[fb].mcs.values() {
                    for pt in mc.pt.as_ref().unwrap().values() {
                        for &(a, b) in pt.and.values() {
                            self.state.kill_fuzzer_bit(f, invbit(a));
                            self.state.kill_fuzzer_bit(f, invbit(b));
                        }
                    }
                }
            }
            self.bits.fbs[fb].en = Some(self.state.collect_single(f));
        }
    }
}

pub fn collect_fuzzers(backend: &CpldBackend, mut state: State) -> Bits {
    let mut res = Bits {
        fbs: backend
            .device
            .fbs()
            .map(|_| FbBits {
                imux: EntityVec::new(),
                uim_mc: EntityVec::new(),
                en: None,
                exp_en: None,
                pla_and: EntityVec::new(),
                ct_invert: EntityPartVec::new(),
                fbclk: None,
                mcs: backend
                    .device
                    .fb_mcs()
                    .map(|_| McBits {
                        pt: None,
                        exp_dir: None,
                        import: None,
                        inv: None,
                        hp: None,
                        ff_en: None,
                        pla_or: EntityVec::new(),
                        lut: None,
                        xor_mux: None,
                        mc_uim_out: None,
                        mc_obuf_out: None,
                        ibuf_uim_out: None,
                        clk_mux: EnumData::empty(),
                        rst_mux: EnumData::empty(),
                        set_mux: EnumData::empty(),
                        clk_inv: None,
                        init: None,
                        ddr: None,
                        ce_mux: None,
                        use_ireg: None,
                        reg_mode: EnumData::empty(),
                        term: None,
                        ibuf_mode: None,
                        dge_en: None,
                        slew: None,
                        oe_mux: None,
                        oe_inv: None,
                        is_gnd: None,
                        uim_oe_mode: None,
                        uim_out_inv: None,
                        obuf_oe_mode: None,
                        ibuf_uim_en: vec![],
                    })
                    .collect(),
            })
            .collect(),
        ipads: backend
            .device
            .ipads()
            .map(|_| IPadBits {
                uim_out_en: EntityVec::new(),
                term: None,
                ibuf_mode: None,
            })
            .collect(),
        fclk_mux: EntityVec::new(),
        fclk_en: EntityVec::new(),
        fclk_inv: EntityVec::new(),
        fsr_en: None,
        fsr_inv: None,
        foe_mux: EntityVec::new(),
        foe_en: EntityVec::new(),
        foe_inv: EntityVec::new(),
        foe_mux_xbr: EntityVec::new(),
        banks: EntityVec::new(),
        term_mode: None,
        vref_en: None,
        dge_en: None,
        clkdiv_en: None,
        clkdiv_div: None,
        clkdiv_dly_en: None,
        ut: None,
        no_isp: None,
        usercode: None,
    };

    if backend.debug >= 3 {
        for (f, bits) in state.fuzzers.iter().sorted_by_key(|x| x.0) {
            println!(
                "FUZZER {f:?} {l} {bits:?}",
                l = bits[0].len(),
                bits = bits
                    .iter()
                    .map(|x| x.iter().sorted().collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            );
        }
    }

    let mut collector = Collector {
        backend,
        state: &mut state,
        bits: &mut res,
    };

    if backend.device.kind == DeviceKind::Xc9500 {
        collector.collect_uim_mc();
    }
    collector.collect_imux();
    if backend.device.kind != DeviceKind::Xc9500 {
        collector.collect_mc_clk_inv();
    }
    if backend.device.kind.is_xc9500x() {
        collector.collect_mc_oe_inv();
    }
    if backend.device.kind.is_xc9500() {
        collector.collect_pt();
        collector.collect_exp_en();
        collector.collect_exp_down();
        collector.collect_import();
        collector.collect_mc_inv();
        collector.collect_mc_lp();
        collector.collect_ff_en();
        collector.collect_usercode();
    } else {
        collector.collect_pla_and();
        collector.collect_pla_or();
        if backend.device.kind == DeviceKind::Xpla3 {
            collector.collect_ct_invert();
            collector.collect_mc_lut();
            collector.collect_no_isp();
        } else {
            collector.collect_mc_xor_mux();
        }
        collector.collect_use_ireg();
        collector.collect_mc_uim_out();
    }
    collector.collect_mc_clk_mux();
    collector.collect_mc_sr_mux();
    if backend.device.kind != DeviceKind::Xpla3 {
        collector.collect_mc_init();
    }
    if backend.device.kind == DeviceKind::Coolrunner2 {
        collector.collect_mc_ddr();
    }
    collector.collect_reg_mode();
    if backend.device.kind.is_xc9500x() {
        collector.collect_ce_mux_xc9500x();
    }
    if backend.device.kind == DeviceKind::Xpla3 {
        collector.collect_ce_mux_xpla3();
        collector.collect_ut();
        collector.collect_fbclk();
        collector.collect_ipad_uim_out();
    }
    collector.collect_ibuf();
    if backend.device.kind.is_xc9500() {
        collector.collect_oe();
    }
    collector.collect_obuf();
    if backend.device.kind == DeviceKind::Coolrunner2 {
        collector.collect_clkdiv();
    }
    if backend.device.kind != DeviceKind::Xpla3 {
        collector.collect_fclk();
        collector.collect_fsr();
        collector.collect_foe();
    }
    if backend.device.kind.is_xc9500() {
        collector.collect_fb();
    }

    for (f, bits) in state.fuzzers.iter().sorted_by_key(|x| x.0) {
        println!(
            "FUZZER {f:?} {l} {bits:?}",
            l = bits[0].len(),
            bits = bits
                .iter()
                .map(|x| x.iter().sorted().collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
    }

    res
}
