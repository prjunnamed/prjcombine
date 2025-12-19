use prjcombine_entity::EntityId;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_cpld::device::DeviceKind;
use prjcombine_re_xilinx_cpld::types::{
    ClkMuxVal, ClkPadId, ExportDir, FclkId, FoeId, ImuxId, ImuxInput, OeMuxVal, OePadId, SrMuxVal,
    Ut, Xc9500McPt,
};
use prjcombine_re_xilinx_cpld::vm6::{InputNodeKind, NodeKind};
use prjcombine_types::cpld::{
    BlockId, ClusterId, IoCoord, MacrocellCoord, MacrocellId, ProductTermId,
};

use crate::backend::{CpldBackend, FuzzerInfo, Iostd, Key, Value, Voltage};

fn ensure_ibuf<'a>(
    backend: &CpldBackend,
    mut fuzzer: Fuzzer<CpldBackend<'a>>,
    io: IoCoord,
    nk: NodeKind,
) -> Fuzzer<CpldBackend<'a>> {
    fuzzer = fuzzer
        .base(Key::NetworkFlag(17), true)
        .base(Key::IBufPresent(io), true)
        .base(Key::IBufHasOut(io, nk), true)
        .base(Key::IBufOutUseMutex(io, nk), true);
    if backend.device.kind == DeviceKind::Coolrunner2 {
        let bank = backend.device.io[&io].bank;
        fuzzer = fuzzer
            .base(Key::Iostd(io), Iostd::Lvcmos18)
            .base(Key::BankVoltage(bank), Voltage::V18)
            .base(Key::BankMutex(bank), Value::None);
    }
    fuzzer
}

fn ensure_fclk<'a>(
    backend: &CpldBackend,
    mut fuzzer: Fuzzer<CpldBackend<'a>>,
    mc: MacrocellCoord,
    idx: FclkId,
) -> Fuzzer<CpldBackend<'a>> {
    let pad = ClkPadId::from_idx(idx.to_idx());
    fuzzer = ensure_ibuf(
        backend,
        fuzzer,
        backend.device.clk_pads[pad],
        NodeKind::IiFclk,
    );
    match backend.device.kind {
        DeviceKind::Xc9500 => {
            fuzzer = fuzzer.base(
                Key::Fclk(idx),
                Value::ClkPadNode(NodeKind::IiFclk, pad, idx.to_idx() as u8),
            );
        }
        DeviceKind::Xpla3 => {
            fuzzer = fuzzer
                .base(Key::Fclk(idx), true)
                .base(Key::FbClk(mc.block, idx), Value::ClkPad(pad));
        }
        _ => {
            fuzzer = fuzzer.base(Key::Fclk(idx), true);
        }
    }
    fuzzer
}

fn add_imux_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    for ifb in backend.device.fbs() {
        for imid in backend.device.fb_imuxes() {
            for &k in backend.imux[imid].keys() {
                let mut fuzzer = Fuzzer::new(FuzzerInfo::Imux(ifb, imid, k)).fuzz(
                    Key::FbImux(ifb, imid),
                    Value::None,
                    k,
                );
                match k {
                    ImuxInput::Ibuf(io) => {
                        if !backend.pin_map.contains_key(&io) {
                            continue;
                        }
                        let altfb = BlockId::from_idx(ifb.to_idx() ^ 1);
                        fuzzer = fuzzer.base(Key::FbImux(altfb, imid), k);
                        fuzzer = ensure_ibuf(backend, fuzzer, io, NodeKind::IiImux);
                    }
                    ImuxInput::Fbk(omc) => {
                        fuzzer = fuzzer
                            .base(Key::McPresent(MacrocellCoord::simple(ifb, omc)), true)
                            .base(
                                Key::McHasOut(MacrocellCoord::simple(ifb, omc), NodeKind::McFbk),
                                true,
                            )
                            .base(
                                Key::McOutUseMutex(
                                    MacrocellCoord::simple(ifb, omc),
                                    NodeKind::McFbk,
                                ),
                                true,
                            );
                    }
                    ImuxInput::Mc(omc) => {
                        fuzzer = fuzzer
                            .base(Key::McPresent(omc), true)
                            .base(Key::McHasOut(omc, NodeKind::McUim), true)
                            .base(Key::McOutUseMutex(omc, NodeKind::McUim), true);
                    }
                    ImuxInput::Pup => {}
                    ImuxInput::Uim => unreachable!(),
                }
                hammer.add_fuzzer_simple(fuzzer);
            }
            if backend.device.kind == DeviceKind::Xc9500 {
                let fuzzer = Fuzzer::new(FuzzerInfo::Imux(ifb, imid, ImuxInput::Uim))
                    .base(
                        Key::McPresent(MacrocellCoord::simple(ifb, MacrocellId::from_idx(0))),
                        true,
                    )
                    .fuzz(Key::FbImux(ifb, imid), Value::None, ImuxInput::Uim);
                hammer.add_fuzzer_simple(fuzzer);
                for omc in backend.device.mcs() {
                    let fuzzer = Fuzzer::new(FuzzerInfo::ImuxUimMc(ifb, imid, omc))
                        .base(
                            Key::McPresent(MacrocellCoord::simple(ifb, MacrocellId::from_idx(0))),
                            true,
                        )
                        .base(Key::McPresent(omc), true)
                        .base(Key::McHasOut(omc, NodeKind::McUim), true)
                        .base(Key::McOutUseMutex(omc, NodeKind::McUim), true)
                        .base(Key::FbImux(ifb, imid), ImuxInput::Uim)
                        .fuzz(Key::UimPath(ifb, imid, omc), Value::None, true);
                    hammer.add_fuzzer_simple(fuzzer);
                }
            }
        }
    }
}

fn pin_imux_inps<'a>(
    backend: &CpldBackend,
    mut fuzzer: Fuzzer<CpldBackend<'a>>,
    fb: BlockId,
) -> Fuzzer<CpldBackend<'a>> {
    if backend.device.kind == DeviceKind::Xc9500 {
        for imid in backend.device.fb_imuxes() {
            fuzzer = fuzzer
                .base(
                    Key::McPresent(MacrocellCoord::simple(fb, MacrocellId::from_idx(0))),
                    true,
                )
                .base(Key::FbImux(fb, imid), ImuxInput::Uim);
        }
    } else {
        for (imid, &inp) in &backend.imux_pinning {
            fuzzer = fuzzer.base(Key::FbImux(fb, imid), inp);
            match inp {
                ImuxInput::Ibuf(io) => {
                    fuzzer = ensure_ibuf(backend, fuzzer, io, NodeKind::IiImux);
                }
                ImuxInput::Fbk(_) => unreachable!(),
                ImuxInput::Mc(omc) => {
                    fuzzer = fuzzer
                        .base(Key::McPresent(omc), true)
                        .base(Key::McHasOut(omc, NodeKind::McUim), true)
                        .base(Key::McOutUseMutex(omc, NodeKind::McUim), true);
                }
                ImuxInput::Pup => {}
                ImuxInput::Uim => unreachable!(),
            }
        }
    }
    fuzzer
}

const XC9500_SI_KINDS: [(Xc9500McPt, NodeKind); 5] = [
    (Xc9500McPt::Clk, NodeKind::McSiClkf),
    (Xc9500McPt::Oe, NodeKind::McSiTrst),
    (Xc9500McPt::Rst, NodeKind::McSiRstf),
    (Xc9500McPt::Set, NodeKind::McSiSetf),
    (Xc9500McPt::Xor, NodeKind::McSiD1),
];

fn add_pterm_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind.is_xc9500() {
        for mc in backend.device.mcs() {
            for (pt, kind) in XC9500_SI_KINDS {
                for imid in backend.device.fb_imuxes() {
                    for pol in [true, false] {
                        let altimid = ImuxId::from_idx(imid.to_idx() ^ 1);
                        let fuzzer = Fuzzer::new(FuzzerInfo::McPTermImux(mc, pt, imid, pol))
                            .base(Key::McPresent(mc), true)
                            .base(Key::McSiMutex(mc), Value::None)
                            .base(Key::McSiPresent(mc), true)
                            .base(Key::McSiHasOut(mc, kind), true)
                            .base(Key::McSiHasTerm(mc, kind), true)
                            .base(Key::McSiTermImux(mc, kind, altimid), true)
                            .fuzz(
                                Key::McSiTermImux(mc, kind, imid),
                                Value::None,
                                Value::Bool(pol),
                            );
                        let fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                        hammer.add_fuzzer_simple(fuzzer);
                    }
                }
            }
        }
    } else {
        for fb in backend.device.fbs() {
            for pt in backend.device.fb_pterms() {
                for imid in backend.device.fb_imuxes() {
                    for pol in [true, false] {
                        let fuzzer = Fuzzer::new(FuzzerInfo::PlaPTermImux(fb, pt, imid, pol))
                            .base(Key::PlaHasTerm(fb, pt), true)
                            .base(Key::PlaTermMutex(fb), Value::MutexFuzz)
                            .fuzz(
                                Key::PlaTermImux(fb, pt, imid),
                                Value::None,
                                Value::Bool(pol),
                            );
                        let fuzzer = pin_imux_inps(backend, fuzzer, fb);
                        hammer.add_fuzzer_simple(fuzzer);
                    }
                }
                if backend.device.kind == DeviceKind::Xpla3 {
                    for fbnid in backend.device.fb_fbns() {
                        let fuzzer = Fuzzer::new(FuzzerInfo::PlaPTermFbn(fb, pt, fbnid))
                            .base(Key::PlaHasTerm(fb, pt), true)
                            .base(Key::PlaTermMutex(fb), Value::MutexFuzz)
                            .base(Key::FbnPresent(fb, fbnid), true)
                            .fuzz(Key::PlaTermFbn(fb, pt, fbnid), Value::None, true);
                        hammer.add_fuzzer_simple(fuzzer);
                    }
                }
            }
        }
    }
}

fn pin_pterms<'a>(
    backend: &CpldBackend,
    mut fuzzer: Fuzzer<CpldBackend<'a>>,
    fb: BlockId,
) -> Fuzzer<CpldBackend<'a>> {
    fuzzer = pin_imux_inps(backend, fuzzer, fb);
    for pt in backend.device.fb_pterms() {
        let imid = ImuxId::from_idx(pt.to_idx() / 2);
        let val = !pt.to_idx().is_multiple_of(2);
        fuzzer = fuzzer
            .base(Key::PlaTermMutex(fb), Value::MutexPin)
            .base(Key::PlaHasTerm(fb, pt), true)
            .base(Key::PlaTermImux(fb, pt, imid), val);
    }
    fuzzer
}

fn add_or_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind.is_xc9500() {
        for mc in backend.device.mcs() {
            for okind in [NodeKind::McSiD2, NodeKind::McSiExport] {
                for (pt, ikind) in XC9500_SI_KINDS {
                    let mut fuzzer = Fuzzer::new(FuzzerInfo::McOrTerm(mc, okind, pt))
                        .base(Key::McPresent(mc), true)
                        .base(Key::McFlag(mc, 0), false)
                        .base(Key::McHasOut(mc, NodeKind::McUim), true)
                        .fuzz(Key::McSiMutex(mc), false, true)
                        .base(Key::McSiPresent(mc), true)
                        .base(Key::McSiHasOut(mc, ikind), false)
                        .base(Key::McSiHasOut(mc, okind), true)
                        .fuzz(Key::McSiHasTerm(mc, okind), false, true)
                        .fuzz(
                            Key::McSiTermImux(mc, okind, ImuxId::from_idx(0)),
                            Value::None,
                            true,
                        );
                    if okind == NodeKind::McSiExport {
                        let dir = ExportDir::Down;
                        let tmcid = backend.device.export_target(mc, dir);
                        fuzzer = fuzzer
                            .base(Key::FbImportMutex(mc.block), Value::Bool(false))
                            .base(Key::McHasOut(mc, NodeKind::McExport), true)
                            .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                            .base(Key::McSiHasTerm(mc, NodeKind::McSiD2), false)
                            .base(Key::McPresent(tmcid), true)
                            .base(Key::McSiPresent(tmcid), true)
                            .base(Key::McSiHasOut(tmcid, NodeKind::McSiD2), true)
                            .base(Key::McSiImport(tmcid, NodeKind::McSiD2, dir), true);
                    } else {
                        fuzzer = fuzzer.base(Key::McSiHasOut(mc, NodeKind::McSiExport), false);
                    }
                    for (_, altikind) in XC9500_SI_KINDS {
                        if altikind == ikind {
                            continue;
                        }
                        fuzzer = fuzzer
                            .base(Key::McSiHasOut(mc, altikind), true)
                            .base(Key::McSiHasTerm(mc, altikind), true)
                            .base(Key::McSiTermImux(mc, altikind, ImuxId::from_idx(1)), true);
                    }
                    let fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                    hammer.add_fuzzer_simple(fuzzer);
                }
                for dir in [ExportDir::Up, ExportDir::Down] {
                    let odir = !dir;
                    let smcid = backend.device.export_source(mc, dir);
                    let mut fuzzer = Fuzzer::new(FuzzerInfo::McOrExp(mc, okind, dir))
                        .base(Key::McPresent(mc), true)
                        .base(Key::McFlag(mc, 0), false)
                        .fuzz(Key::McSiMutex(mc), false, true)
                        .base(Key::McSiPresent(mc), true)
                        .base(Key::McSiHasOut(mc, okind), true)
                        .base(Key::McSiHasOut(mc, XC9500_SI_KINDS[0].1), false)
                        .base(Key::McSiHasTerm(mc, okind), true)
                        .base(Key::McSiTermImux(mc, okind, ImuxId::from_idx(2)), true)
                        .base(Key::McSiImport(mc, okind, odir), false)
                        .base(Key::McPresent(smcid), true)
                        .base(Key::McFlag(smcid, 0), false)
                        .fuzz(Key::McSiMutex(smcid), false, true)
                        .base(Key::McSiPresent(smcid), true)
                        .base(Key::McSiHasOut(smcid, XC9500_SI_KINDS[0].1), false)
                        .base(Key::McSiHasOut(smcid, NodeKind::McSiD2), false)
                        .base(Key::McHasOut(smcid, NodeKind::McUim), true)
                        .fuzz(Key::FbImportMutex(mc.block), false, true)
                        .fuzz(Key::McHasOut(smcid, NodeKind::McExport), false, true)
                        .fuzz(Key::McSiImport(mc, okind, dir), false, true)
                        .fuzz(Key::McSiHasOut(smcid, NodeKind::McSiExport), false, true)
                        .fuzz(Key::McSiHasTerm(smcid, NodeKind::McSiExport), false, true)
                        .fuzz(
                            Key::McSiTermImux(smcid, NodeKind::McSiExport, ImuxId::from_idx(0)),
                            Value::None,
                            true,
                        );
                    if okind == NodeKind::McSiExport {
                        let tmcid = backend.device.export_target(mc, dir);
                        fuzzer = fuzzer
                            .base(Key::McHasOut(mc, NodeKind::McExport), true)
                            .base(Key::McSiHasOut(mc, NodeKind::McSiD2), false)
                            .base(Key::McPresent(tmcid), true)
                            .base(Key::McSiPresent(tmcid), true)
                            .base(Key::McSiHasOut(tmcid, NodeKind::McSiD2), true)
                            .base(Key::McSiImport(tmcid, NodeKind::McSiD2, dir), true);
                    } else {
                        fuzzer = fuzzer.base(Key::McSiHasOut(mc, NodeKind::McSiExport), false);
                    }
                    for &(_, altikind) in &XC9500_SI_KINDS[1..] {
                        fuzzer = fuzzer
                            .base(Key::McSiHasOut(mc, altikind), true)
                            .base(Key::McSiHasTerm(mc, altikind), true)
                            .base(Key::McSiTermImux(mc, altikind, ImuxId::from_idx(1)), true)
                            .base(Key::McSiHasOut(smcid, altikind), true)
                            .base(Key::McSiHasTerm(smcid, altikind), true)
                            .base(
                                Key::McSiTermImux(smcid, altikind, ImuxId::from_idx(1)),
                                true,
                            );
                    }
                    let fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                    hammer.add_fuzzer_simple(fuzzer);
                }
            }
            for (pt, ikind) in XC9500_SI_KINDS {
                let fuzzer = Fuzzer::new(FuzzerInfo::McSiSpec(mc, pt))
                    .base(Key::McPresent(mc), true)
                    .base(Key::McFlag(mc, 0), false)
                    .base(Key::McHasOut(mc, NodeKind::McUim), true)
                    .fuzz(Key::McSiMutex(mc), false, true)
                    .base(Key::McSiPresent(mc), true)
                    .fuzz(Key::McSiHasOut(mc, ikind), false, true)
                    .fuzz(Key::McSiHasTerm(mc, ikind), false, true)
                    .fuzz(
                        Key::McSiTermImux(mc, ikind, ImuxId::from_idx(0)),
                        Value::None,
                        true,
                    );
                let fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                hammer.add_fuzzer_simple(fuzzer);
            }
        }
    } else {
        for mc in backend.device.mcs() {
            for pt in backend.device.fb_pterms() {
                let opt = ProductTermId::from_idx(pt.to_idx() ^ 1);
                let fuzzer = Fuzzer::new(FuzzerInfo::McOrPla(mc, pt))
                    .base(Key::McPresent(mc), true)
                    .base(Key::McSiPresent(mc), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                    .base(Key::McSiPla(mc, NodeKind::McSiD2, opt), true)
                    .fuzz(Key::McSiPla(mc, NodeKind::McSiD2, pt), false, true);
                let fuzzer = pin_pterms(backend, fuzzer, mc.block);
                hammer.add_fuzzer_simple(fuzzer);
            }
        }
    }
}

fn add_ct_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind == DeviceKind::Xpla3 {
        for fb in backend.device.fbs() {
            let mc = MacrocellCoord::simple(fb, MacrocellId::from_idx(0));
            for i in 0..8 {
                let ikind = if i < 6 {
                    InputNodeKind::SrffR
                } else {
                    InputNodeKind::SrffC
                };
                let pt = ProductTermId::from_idx(i);
                let mut fuzzer = Fuzzer::new(FuzzerInfo::CtInvert(fb, pt))
                    .base(Key::CtPresent(fb, pt), true)
                    .base(Key::CtUseMutex(fb, pt), Value::CtUseCt)
                    .base(Key::McPresent(mc), true)
                    .base(Key::McHasOut(mc, NodeKind::McUim), true)
                    .base(Key::McSiMutex(mc), Value::None)
                    .base(Key::McSiPresent(mc), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                    .base(
                        Key::McSiPla(mc, NodeKind::McSiD2, ProductTermId::from_idx(9)),
                        true,
                    )
                    .base(Key::McFfPresent(mc), true)
                    .base(Key::McFfInput(mc, ikind), Value::InputCt(pt))
                    .fuzz(Key::CtInvert(fb, pt), false, true);
                if ikind != InputNodeKind::SrffC {
                    fuzzer = fuzzer
                        .base(
                            Key::McFfInput(mc, InputNodeKind::SrffC),
                            Value::InputCt(ProductTermId::from_idx(7)),
                        )
                        .base(Key::CtPresent(fb, ProductTermId::from_idx(7)), true)
                        .base(
                            Key::CtUseMutex(fb, ProductTermId::from_idx(7)),
                            Value::CtUseCt,
                        );
                }
                let fuzzer = pin_pterms(backend, fuzzer, fb);
                hammer.add_fuzzer_simple(fuzzer);
            }
        }
    }
}

fn pin_ibuf_in<'a>(
    backend: &CpldBackend,
    mut fuzzer: Fuzzer<CpldBackend<'a>>,
    io: IoCoord,
    fb: BlockId,
) -> Fuzzer<CpldBackend<'a>> {
    fuzzer = fuzzer
        .base(
            Key::FbImux(fb, backend.ibuf_test_imux[&io]),
            ImuxInput::Ibuf(io),
        )
        .base(
            Key::McPresent(MacrocellCoord::simple(fb, MacrocellId::from_idx(0))),
            true,
        )
        .base(
            Key::McHasOut(
                MacrocellCoord::simple(fb, MacrocellId::from_idx(0)),
                NodeKind::McUim,
            ),
            true,
        );
    ensure_ibuf(backend, fuzzer, io, NodeKind::IiImux)
}

fn pin_ibuf<'a>(
    backend: &CpldBackend,
    fuzzer: Fuzzer<CpldBackend<'a>>,
    io: IoCoord,
) -> Fuzzer<CpldBackend<'a>> {
    let fbid = match io {
        IoCoord::Ipad(_) => BlockId::from_idx(0),
        IoCoord::Macrocell(mc) => BlockId::from_idx(mc.block.to_idx() ^ 1),
    };
    pin_ibuf_in(backend, fuzzer, io, fbid)
}

fn add_mc_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    for mc in backend.device.mcs() {
        if backend.device.kind.is_xc9500() {
            let fuzzer = Fuzzer::new(FuzzerInfo::McLowPower(mc))
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), false)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), false)
                .base(Key::McSiHasOut(mc, NodeKind::McSiRstf), false)
                .base(Key::McSiHasOut(mc, NodeKind::McSiSetf), false)
                .base(Key::McSiHasOut(mc, NodeKind::McSiClkf), false)
                .base(Key::McSiHasOut(mc, NodeKind::McSiTrst), false)
                .base(Key::McSiHasOut(mc, NodeKind::McSiCe), false)
                .base(Key::McSiHasOut(mc, NodeKind::McSiExport), false)
                .fuzz(Key::McFlag(mc, 0), false, true);
            hammer.add_fuzzer_simple(fuzzer);
        } else {
            let fuzzer = Fuzzer::new(FuzzerInfo::McComb(mc))
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .base(Key::McOutUseMutex(mc, NodeKind::McUim), Value::None)
                .base(
                    Key::IBufOutUseMutex(IoCoord::Macrocell(mc), NodeKind::IiImux),
                    Value::None,
                )
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiClkf), true)
                .base(Key::McFfPresent(mc), true)
                .base(
                    Key::McFfInput(mc, InputNodeKind::SrffC),
                    Value::InputSi(NodeKind::McSiClkf),
                )
                .fuzz(Key::McHasOut(mc, NodeKind::McComb), false, true);
            hammer.add_fuzzer_simple(fuzzer);
        }
        let fuzzer = Fuzzer::new(FuzzerInfo::McUimOut(mc))
            .base(Key::McPresent(mc), true)
            .base(Key::McOutUseMutex(mc, NodeKind::McUim), Value::None)
            .base(
                Key::IBufOutUseMutex(IoCoord::Macrocell(mc), NodeKind::IiImux),
                Value::None,
            )
            .fuzz(Key::McSiMutex(mc), false, true)
            .base(Key::McSiPresent(mc), true)
            .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
            .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
            .base(Key::McSiHasOut(mc, NodeKind::McSiClkf), true)
            .base(Key::McFfPresent(mc), true)
            .base(
                Key::McFfInput(mc, InputNodeKind::SrffC),
                Value::InputSi(NodeKind::McSiClkf),
            )
            .fuzz(Key::McHasOut(mc, NodeKind::McUim), false, true);
        hammer.add_fuzzer_simple(fuzzer);
        if backend.device.kind == DeviceKind::Xc9500 {
            let imid = ImuxId::from_idx(mc.macrocell.to_idx());
            let fuzzer = Fuzzer::new(FuzzerInfo::McUimOutInv(mc))
                .base(Key::McPresent(mc), true)
                .fuzz(Key::McOutUseMutex(mc, NodeKind::McUim), true, false)
                .base(
                    Key::IBufOutUseMutex(IoCoord::Macrocell(mc), NodeKind::IiImux),
                    Value::None,
                )
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiClkf), true)
                .base(Key::McFfPresent(mc), true)
                .base(
                    Key::McFfInput(mc, InputNodeKind::SrffC),
                    Value::InputSi(NodeKind::McSiClkf),
                )
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .base(Key::FbImux(mc.block, imid), ImuxInput::Uim)
                .fuzz(Key::UimPath(mc.block, imid, mc), true, false);
            hammer.add_fuzzer_simple(fuzzer);
        }
        // CLK mux
        let mut srcs = vec![ClkMuxVal::Pt];
        match backend.device.kind {
            DeviceKind::Xc9500 | DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                for i in 0..3 {
                    srcs.push(ClkMuxVal::Fclk(FclkId::from_idx(i)));
                }
            }
            DeviceKind::Xpla3 => {
                for i in 4..8 {
                    srcs.push(ClkMuxVal::Ct(ProductTermId::from_idx(i)));
                }
                for i in 0..2 {
                    srcs.push(ClkMuxVal::Fclk(FclkId::from_idx(i)));
                }
                srcs.push(ClkMuxVal::Ut);
            }
            DeviceKind::Coolrunner2 => {
                srcs.push(ClkMuxVal::Ct(ProductTermId::from_idx(4)));
                for i in 0..3 {
                    srcs.push(ClkMuxVal::Fclk(FclkId::from_idx(i)));
                }
            }
        }
        for inv in [false, true] {
            if backend.device.kind == DeviceKind::Xc9500 && inv {
                continue;
            }
            for &src in &srcs {
                let mut fuzzer = Fuzzer::new(FuzzerInfo::McClk(mc, src, inv))
                    .base(Key::McPresent(mc), true)
                    .base(Key::McFlag(mc, 0), false)
                    .base(Key::McHasOut(mc, NodeKind::McUim), true)
                    .fuzz(Key::McSiMutex(mc), false, true)
                    .base(Key::McSiPresent(mc), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                    .base(Key::McFfPresent(mc), true)
                    .fuzz(Key::McFlag(mc, 28), false, inv);
                let mut do_sibling = None;
                match src {
                    ClkMuxVal::Pt => {
                        fuzzer = fuzzer
                            .fuzz(Key::McSiHasOut(mc, NodeKind::McSiClkf), false, true)
                            .fuzz(
                                Key::McFfInput(mc, InputNodeKind::SrffC),
                                Value::None,
                                Value::InputSi(NodeKind::McSiClkf),
                            );
                        if backend.device.kind.is_xc9500() {
                            fuzzer = fuzzer
                                .fuzz(Key::McSiHasTerm(mc, NodeKind::McSiClkf), false, true)
                                .fuzz(
                                    Key::McSiTermImux(mc, NodeKind::McSiClkf, ImuxId::from_idx(0)),
                                    Value::None,
                                    true,
                                );
                            fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                        } else {
                            let pt = if backend.device.kind == DeviceKind::Xpla3 {
                                ProductTermId::from_idx(9 + mc.macrocell.to_idx() * 2)
                            } else {
                                ProductTermId::from_idx(10 + mc.macrocell.to_idx() * 3)
                            };
                            fuzzer = fuzzer.base(Key::PlaHasTerm(mc.block, pt), true).fuzz(
                                Key::McSiPla(mc, NodeKind::McSiClkf, pt),
                                false,
                                true,
                            );
                            fuzzer = pin_pterms(backend, fuzzer, mc.block);
                        }
                    }
                    ClkMuxVal::Fclk(idx) => {
                        let inp = if backend.device.kind == DeviceKind::Xpla3 {
                            let pad = ClkPadId::from_idx(idx.to_idx());
                            fuzzer = fuzzer
                                .base(Key::Fclk(FclkId::from_idx(pad.to_idx())), true)
                                .base(Key::FbClk(mc.block, idx), Value::ClkPad(pad))
                                .base(
                                    Key::FbClk(mc.block, FclkId::from_idx(idx.to_idx() ^ 1)),
                                    Value::None,
                                );
                            backend.device.clk_pads[pad]
                        } else {
                            let pad = ClkPadId::from_idx(idx.to_idx());
                            if backend.device.kind == DeviceKind::Xc9500 {
                                fuzzer = fuzzer.base(
                                    Key::Fclk(idx),
                                    Value::ClkPadNode(NodeKind::IiFclk, pad, idx.to_idx() as u8),
                                );
                                for i in 0..3 {
                                    let i = FclkId::from_idx(i);
                                    if i != idx {
                                        fuzzer = fuzzer.base(Key::Fclk(i), Value::None);
                                    }
                                }
                            } else {
                                fuzzer = fuzzer.base(Key::Fclk(idx), true);
                            }
                            backend.device.clk_pads[pad]
                        };
                        let nk = if backend.device.kind.is_xc9500x() && inv {
                            NodeKind::IiFclkInv
                        } else {
                            NodeKind::IiFclk
                        };
                        fuzzer = fuzzer.fuzz(
                            Key::McFfInput(mc, InputNodeKind::SrffC),
                            Value::None,
                            Value::InputPad(inp, nk),
                        );
                        fuzzer = ensure_ibuf(backend, fuzzer, inp, nk);
                        do_sibling = Some(Value::InputPad(inp, nk));
                    }
                    ClkMuxVal::Ct(pt) => {
                        fuzzer = fuzzer
                            .base(Key::PlaHasTerm(mc.block, pt), true)
                            .base(Key::CtPresent(mc.block, pt), true)
                            .base(Key::CtUseMutex(mc.block, pt), Value::CtUseCt)
                            .base(Key::Ut(Ut::Clk), Value::None)
                            .fuzz(
                                Key::McFfInput(mc, InputNodeKind::SrffC),
                                Value::None,
                                Value::InputCt(pt),
                            );
                        do_sibling = Some(Value::InputCt(pt));
                    }
                    ClkMuxVal::Ut => {
                        let pt = ProductTermId::from_idx(7);
                        fuzzer = fuzzer
                            .base(Key::PlaHasTerm(mc.block, pt), true)
                            .base(Key::CtPresent(mc.block, pt), true)
                            .base(Key::CtUseMutex(mc.block, pt), Value::CtUseUt(Ut::Clk))
                            .base(Key::Ut(Ut::Clk), Value::Ut(mc.block, pt))
                            .fuzz(
                                Key::McFfInput(mc, InputNodeKind::SrffC),
                                Value::None,
                                Value::InputCt(pt),
                            );
                        do_sibling = Some(Value::InputCt(pt));
                    }
                }
                if let Some(inp) = do_sibling {
                    let omc = MacrocellCoord::simple(
                        mc.block,
                        MacrocellId::from_idx(mc.macrocell.to_idx() ^ 1),
                    );
                    fuzzer = fuzzer
                        .base(Key::McPresent(omc), true)
                        .base(Key::McHasOut(omc, NodeKind::McUim), true)
                        .fuzz(Key::McSiMutex(omc), false, true)
                        .base(Key::McSiPresent(omc), true)
                        .base(Key::McSiHasOut(omc, NodeKind::McSiD1), true)
                        .base(Key::McSiHasOut(omc, NodeKind::McSiD2), true)
                        .base(Key::McFfPresent(omc), true)
                        .base(Key::McFlag(omc, 28), inv)
                        .base(Key::McFfInput(omc, InputNodeKind::SrffC), inp);
                }
                hammer.add_fuzzer_simple(fuzzer);
            }
        }

        // RST, SET muxes
        for is_set in [false, true] {
            let srcs = match backend.device.kind {
                DeviceKind::Xc9500 | DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                    vec![SrMuxVal::Pt, SrMuxVal::Fsr]
                }
                DeviceKind::Xpla3 => vec![
                    SrMuxVal::Ct(ProductTermId::from_idx(0)),
                    SrMuxVal::Ct(ProductTermId::from_idx(1)),
                    SrMuxVal::Ct(ProductTermId::from_idx(2)),
                    SrMuxVal::Ct(ProductTermId::from_idx(3)),
                    SrMuxVal::Ct(ProductTermId::from_idx(4)),
                    SrMuxVal::Ct(ProductTermId::from_idx(5)),
                    SrMuxVal::Ut,
                ],
                DeviceKind::Coolrunner2 => vec![
                    SrMuxVal::Pt,
                    SrMuxVal::Ct(ProductTermId::from_idx(if is_set { 6 } else { 5 })),
                    SrMuxVal::Fsr,
                ],
            };
            for src in srcs {
                let clkpad = ClkPadId::from_idx(0);
                let fclk = FclkId::from_idx(0);
                let clk = backend.device.clk_pads[clkpad];
                let fi = if is_set {
                    FuzzerInfo::McSet(mc, src)
                } else {
                    FuzzerInfo::McRst(mc, src)
                };
                let mut fuzzer = Fuzzer::new(fi)
                    .base(Key::McPresent(mc), true)
                    .base(Key::McFlag(mc, 0), false)
                    .base(Key::McHasOut(mc, NodeKind::McUim), true)
                    .fuzz(Key::McSiMutex(mc), false, true)
                    .base(Key::McSiPresent(mc), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                    .base(Key::McFfPresent(mc), true)
                    .base(
                        Key::McFfInput(mc, InputNodeKind::SrffC),
                        Value::InputPad(clk, NodeKind::IiFclk),
                    );
                fuzzer = ensure_fclk(backend, fuzzer, mc, fclk);

                let mut do_sibling = None;
                let ink = if is_set {
                    InputNodeKind::SrffS
                } else {
                    InputNodeKind::SrffR
                };

                match src {
                    SrMuxVal::Pt => {
                        let nk = if is_set {
                            NodeKind::McSiSetf
                        } else {
                            NodeKind::McSiRstf
                        };
                        fuzzer = fuzzer.fuzz(Key::McSiHasOut(mc, nk), false, true).fuzz(
                            Key::McFfInput(mc, ink),
                            Value::None,
                            Value::InputSi(nk),
                        );
                        if backend.device.kind.is_xc9500() {
                            fuzzer = fuzzer.fuzz(Key::McSiHasTerm(mc, nk), false, true).fuzz(
                                Key::McSiTermImux(mc, nk, ImuxId::from_idx(0)),
                                Value::None,
                                true,
                            );
                            fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                        } else {
                            let pt = ProductTermId::from_idx(8 + mc.macrocell.to_idx() * 3);
                            fuzzer = fuzzer.base(Key::PlaHasTerm(mc.block, pt), true).fuzz(
                                Key::McSiPla(mc, nk, pt),
                                false,
                                true,
                            );
                            fuzzer = pin_pterms(backend, fuzzer, mc.block);
                        }
                    }
                    SrMuxVal::Fsr => {
                        if backend.device.kind == DeviceKind::Xc9500 {
                            fuzzer = fuzzer.base(Key::Fsr, Value::SrPadNode(NodeKind::IiFsr));
                        } else {
                            fuzzer = fuzzer.base(Key::Fsr, true);
                        };
                        let inp = backend.device.sr_pad.unwrap();
                        fuzzer = ensure_ibuf(backend, fuzzer, inp, NodeKind::IiFsr);
                        fuzzer = fuzzer
                            .base(Key::IBufHasOut(inp, NodeKind::IiFsrInv), false)
                            .fuzz(
                                Key::McFfInput(mc, ink),
                                Value::None,
                                Value::InputPad(inp, NodeKind::IiFsr),
                            );

                        do_sibling = Some(Value::InputPad(inp, NodeKind::IiFsr));
                    }
                    SrMuxVal::Ct(pt) => {
                        fuzzer = fuzzer
                            .base(Key::PlaHasTerm(mc.block, pt), true)
                            .base(Key::CtPresent(mc.block, pt), true)
                            .base(Key::CtUseMutex(mc.block, pt), Value::CtUseCt)
                            .base(Key::Ut(Ut::Rst), Value::None)
                            .base(Key::Ut(Ut::Set), Value::None)
                            .fuzz(Key::McFfInput(mc, ink), Value::None, Value::InputCt(pt));
                        do_sibling = Some(Value::InputCt(pt));
                    }
                    SrMuxVal::Ut => {
                        let pt = ProductTermId::from_idx(7);
                        let ut = if is_set { Ut::Set } else { Ut::Rst };
                        fuzzer = fuzzer
                            .base(Key::PlaHasTerm(mc.block, pt), true)
                            .base(Key::CtPresent(mc.block, pt), true)
                            .base(Key::CtUseMutex(mc.block, pt), Value::CtUseUt(ut))
                            .base(Key::Ut(ut), Value::Ut(mc.block, pt))
                            .fuzz(Key::McFfInput(mc, ink), Value::None, Value::InputCt(pt));
                        do_sibling = Some(Value::InputCt(pt));
                    }
                    _ => unreachable!(),
                }
                if let Some(inp) = do_sibling {
                    let omc = MacrocellCoord::simple(
                        mc.block,
                        MacrocellId::from_idx(mc.macrocell.to_idx() ^ 1),
                    );
                    fuzzer = fuzzer
                        .base(Key::McPresent(omc), true)
                        .base(Key::McHasOut(omc, NodeKind::McUim), true)
                        .fuzz(Key::McSiMutex(omc), false, true)
                        .base(Key::McSiPresent(omc), true)
                        .base(Key::McSiHasOut(omc, NodeKind::McSiD1), true)
                        .base(Key::McSiHasOut(omc, NodeKind::McSiD2), true)
                        .base(Key::McFfPresent(omc), true)
                        .base(
                            Key::McFfInput(omc, InputNodeKind::SrffC),
                            Value::InputPad(clk, NodeKind::IiFclk),
                        )
                        .base(Key::McFfInput(omc, ink), inp);
                }
                hammer.add_fuzzer_simple(fuzzer);
            }
        }

        // CE mux
        enum CeIn {
            PtRst,
            PtSet,
            Pt,
            Ct,
        }
        let ceins = match backend.device.kind {
            DeviceKind::Xc9500 => vec![],
            DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => vec![CeIn::PtRst, CeIn::PtSet],
            DeviceKind::Xpla3 => vec![CeIn::Pt, CeIn::Ct],
            DeviceKind::Coolrunner2 => vec![CeIn::Pt],
        };
        for cein in ceins {
            let fi = match cein {
                CeIn::PtRst => FuzzerInfo::McCeRst(mc),
                CeIn::PtSet => FuzzerInfo::McCeSet(mc),
                CeIn::Pt => FuzzerInfo::McCePt(mc),
                CeIn::Ct => FuzzerInfo::McCeCt(mc),
            };
            let clkpad = ClkPadId::from_idx(0);
            let fclk = FclkId::from_idx(0);
            let clk = backend.device.clk_pads[clkpad];
            let mut fuzzer = Fuzzer::new(fi)
                .base(Key::McPresent(mc), true)
                .base(Key::McFlag(mc, 0), false)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McFfPresent(mc), true)
                .base(
                    Key::McFfInput(mc, InputNodeKind::SrffC),
                    Value::InputPad(clk, NodeKind::IiFclk),
                );
            fuzzer = ensure_fclk(backend, fuzzer, mc, fclk);
            match cein {
                CeIn::PtRst | CeIn::PtSet => {
                    let pin_si = if matches!(cein, CeIn::PtRst) {
                        NodeKind::McSiSetf
                    } else {
                        NodeKind::McSiRstf
                    };
                    fuzzer = fuzzer
                        .base(Key::McSiHasOut(mc, pin_si), true)
                        .base(Key::McSiHasTerm(mc, pin_si), true)
                        .base(Key::McSiTermImux(mc, pin_si, ImuxId::from_idx(1)), true)
                        .fuzz(Key::McSiHasOut(mc, NodeKind::McSiCe), false, true)
                        .fuzz(Key::McSiHasTerm(mc, NodeKind::McSiCe), false, true)
                        .fuzz(
                            Key::McSiTermImux(mc, NodeKind::McSiCe, ImuxId::from_idx(0)),
                            Value::None,
                            true,
                        )
                        .fuzz(
                            Key::McFfInput(mc, InputNodeKind::SrffCe),
                            Value::None,
                            Value::InputSi(NodeKind::McSiCe),
                        );
                    fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                }
                CeIn::Pt => {
                    fuzzer = fuzzer
                        .fuzz(Key::McSiHasOut(mc, NodeKind::McSiCe), false, true)
                        .fuzz(
                            Key::McFfInput(mc, InputNodeKind::SrffCe),
                            Value::None,
                            Value::InputSi(NodeKind::McSiCe),
                        );
                    let pt = if backend.device.kind == DeviceKind::Xpla3 {
                        ProductTermId::from_idx(9 + mc.macrocell.to_idx() * 2)
                    } else {
                        ProductTermId::from_idx(10 + mc.macrocell.to_idx() * 3)
                    };
                    fuzzer = fuzzer.base(Key::PlaHasTerm(mc.block, pt), true).fuzz(
                        Key::McSiPla(mc, NodeKind::McSiCe, pt),
                        false,
                        true,
                    );
                    fuzzer = pin_pterms(backend, fuzzer, mc.block);
                }
                CeIn::Ct => {
                    let pt = ProductTermId::from_idx(4);
                    fuzzer = fuzzer
                        .base(Key::PlaHasTerm(mc.block, pt), true)
                        .base(Key::CtPresent(mc.block, pt), true)
                        .base(Key::CtUseMutex(mc.block, pt), Value::CtUseCt)
                        .fuzz(
                            Key::McFfInput(mc, InputNodeKind::SrffCe),
                            Value::None,
                            Value::InputCt(pt),
                        );
                    let omc = MacrocellCoord::simple(
                        mc.block,
                        MacrocellId::from_idx(mc.macrocell.to_idx() ^ 1),
                    );
                    fuzzer = fuzzer
                        .base(Key::McPresent(omc), true)
                        .base(Key::McHasOut(omc, NodeKind::McUim), true)
                        .fuzz(Key::McSiMutex(omc), false, true)
                        .base(Key::McSiPresent(omc), true)
                        .base(Key::McSiHasOut(omc, NodeKind::McSiD1), true)
                        .base(Key::McSiHasOut(omc, NodeKind::McSiD2), true)
                        .base(Key::McFfPresent(omc), true)
                        .base(
                            Key::McFfInput(omc, InputNodeKind::SrffC),
                            Value::InputPad(clk, NodeKind::IiFclk),
                        )
                        .base(
                            Key::McFfInput(omc, InputNodeKind::SrffCe),
                            Value::InputCt(pt),
                        );
                }
            }
            hammer.add_fuzzer_simple(fuzzer);
        }

        // misc FF settings
        enum Flag {
            Tff,
            Init,
            Latch,
            Ddr,
        }
        use Flag::*;
        for flag in [Tff, Init, Latch, Ddr] {
            let (idx, fi) = match flag {
                Tff => (12, FuzzerInfo::McTff(mc)),
                Latch => {
                    if backend.device.kind.is_xc9500() {
                        continue;
                    }
                    (6, FuzzerInfo::McLatch(mc))
                }
                Ddr => {
                    if backend.device.kind != DeviceKind::Coolrunner2 {
                        continue;
                    }
                    (7, FuzzerInfo::McDdr(mc))
                }
                Init => {
                    if backend.device.kind == DeviceKind::Xpla3 {
                        continue;
                    }
                    (9, FuzzerInfo::McInit(mc))
                }
            };
            let clkpad = ClkPadId::from_idx(0);
            let fclk = FclkId::from_idx(0);
            let clk = backend.device.clk_pads[clkpad];
            let mut fuzzer = Fuzzer::new(fi)
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McFfPresent(mc), true)
                .base(
                    Key::McFfInput(mc, InputNodeKind::SrffC),
                    Value::InputPad(clk, NodeKind::IiFclk),
                )
                .fuzz(Key::McFlag(mc, idx), false, true);
            fuzzer = ensure_fclk(backend, fuzzer, mc, fclk);
            if matches!(flag, Init) {
                fuzzer = fuzzer.fuzz(Key::McFlag(mc, 10), true, false);
            }
            hammer.add_fuzzer_simple(fuzzer);
        }

        // D mux
        if !backend.device.kind.is_xc9500() && backend.pin_map.contains_key(&IoCoord::Macrocell(mc))
        {
            let mut fuzzer = Fuzzer::new(FuzzerInfo::McInputIreg(mc))
                .base(
                    Key::IBufHasOut(IoCoord::Macrocell(mc), NodeKind::IiReg),
                    true,
                )
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .fuzz(Key::McFfPresent(mc), false, Value::Ireg);
            fuzzer = pin_ibuf(backend, fuzzer, IoCoord::Macrocell(mc));
            hammer.add_fuzzer_simple(fuzzer);
        }
    }
}

fn add_xor_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    for mc in backend.device.mcs() {
        // use D1, use D2, invert
        let mut cases = vec![
            (false, true, false, FuzzerInfo::McInputD2(mc)),
            (false, true, true, FuzzerInfo::McInputD2B(mc)),
            (true, true, false, FuzzerInfo::McInputXor(mc)),
            (true, true, true, FuzzerInfo::McInputXorB(mc)),
        ];
        if backend.device.kind == DeviceKind::Xpla3 {
            cases.extend([
                (true, false, false, FuzzerInfo::McInputD1(mc)),
                (true, false, true, FuzzerInfo::McInputD1B(mc)),
            ]);
        }
        for (has_d1, has_d2, inv, fi) in cases {
            let mut fuzzer = Fuzzer::new(fi)
                .base(Key::McPresent(mc), true)
                .base(Key::McFlag(mc, 0), false)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .fuzz(Key::McSiHasOut(mc, NodeKind::McSiD1), false, true)
                .fuzz(Key::McSiHasOut(mc, NodeKind::McSiD2), false, true)
                .fuzz(Key::McFlag(mc, 8), false, inv)
                .fuzz(Key::McFfPresent(mc), false, true);
            if backend.device.kind.is_xc9500() {
                fuzzer = fuzzer
                    .fuzz(Key::McSiHasTerm(mc, NodeKind::McSiD1), false, has_d1)
                    .fuzz(Key::McSiHasTerm(mc, NodeKind::McSiD2), false, has_d2)
                    .fuzz(
                        Key::McSiTermImux(mc, NodeKind::McSiD1, ImuxId::from_idx(0)),
                        Value::None,
                        if has_d1 {
                            Value::Bool(true)
                        } else {
                            Value::None
                        },
                    )
                    .fuzz(
                        Key::McSiTermImux(mc, NodeKind::McSiD2, ImuxId::from_idx(0)),
                        Value::None,
                        if has_d2 {
                            Value::Bool(true)
                        } else {
                            Value::None
                        },
                    );
                fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
            } else {
                let (ptd1, ptd2) = if backend.device.kind == DeviceKind::Xpla3 {
                    (
                        ProductTermId::from_idx(8 + mc.macrocell.to_idx() * 2),
                        ProductTermId::from_idx(0),
                    )
                } else {
                    (
                        ProductTermId::from_idx(10 + mc.macrocell.to_idx() * 3),
                        ProductTermId::from_idx(0),
                    )
                };
                fuzzer = fuzzer
                    .fuzz(Key::McSiPla(mc, NodeKind::McSiD1, ptd1), false, has_d1)
                    .fuzz(Key::McSiPla(mc, NodeKind::McSiD2, ptd2), false, has_d2);
                fuzzer = pin_pterms(backend, fuzzer, mc.block);
            }
            hammer.add_fuzzer_simple(fuzzer);
        }
    }
}

fn add_ibuf_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    for &io in backend.pin_map.keys() {
        #[derive(Eq, PartialEq, Copy, Clone)]
        enum Unused {
            Float,
            Gnd,
            Pullup,
            Keeper,
        }
        let unused = match backend.device.kind {
            DeviceKind::Xc9500 | DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => {
                vec![Unused::Float, Unused::Gnd]
            }
            DeviceKind::Xpla3 => vec![Unused::Float, Unused::Pullup],
            DeviceKind::Coolrunner2 => {
                vec![Unused::Float, Unused::Gnd, Unused::Pullup, Unused::Keeper]
            }
        };
        for un in unused {
            let fi = match un {
                Unused::Float => FuzzerInfo::IBufPresent(io),
                Unused::Gnd => FuzzerInfo::IBufPresentGnd(io),
                Unused::Pullup => FuzzerInfo::IBufPresentPullup(io),
                Unused::Keeper => FuzzerInfo::IBufPresentKeeper(io),
            };
            let mut fuzzer = Fuzzer::new(fi)
                .base(Key::NetworkFlag(17), true)
                .base(Key::NetworkFlag(15), un == Unused::Gnd)
                .base(Key::NetworkFlag(29), un == Unused::Pullup)
                .base(Key::NetworkFlag(30), un == Unused::Keeper)
                .fuzz(Key::IBufPresent(io), false, true)
                .fuzz(Key::IBufHasOut(io, NodeKind::IiImux), false, true)
                .fuzz(
                    Key::FbImux(BlockId::from_idx(0), backend.ibuf_test_imux[&io]),
                    Value::None,
                    ImuxInput::Ibuf(io),
                )
                .base(Key::IBufOutUseMutex(io, NodeKind::IiImux), true);
            if let IoCoord::Macrocell(mc) = io {
                fuzzer = fuzzer
                    .base(Key::OBufPresent(mc), false)
                    .base(Key::McPresent(mc), false);
            }
            if backend.device.kind == DeviceKind::Coolrunner2 {
                let bank = backend.device.io[&io].bank;
                fuzzer = fuzzer
                    .fuzz(Key::Iostd(io), Value::None, Iostd::Lvcmos18)
                    .base(Key::BankVoltage(bank), Voltage::V18)
                    .base(Key::BankMutex(bank), Value::None);
                if matches!(io, IoCoord::Macrocell(_)) {
                    fuzzer = fuzzer.fuzz(Key::IBufFlag(io, 2), false, true);
                }
            }
            hammer.add_fuzzer_simple(fuzzer);
        }
        if backend.device.kind == DeviceKind::Coolrunner2 {
            let bank = backend.device.io[&io].bank;
            let fuzzer =
                Fuzzer::new(FuzzerInfo::IBufSchmitt(io)).fuzz(Key::IBufFlag(io, 2), false, true);
            let fuzzer = pin_ibuf(backend, fuzzer, io);
            hammer.add_fuzzer_simple(fuzzer);

            if backend.device.has_vref {
                let mut pins = backend.pin_map.keys().copied().filter(|&omc| {
                    omc != io
                        && backend.device.io[&omc].bank == bank
                        && backend.pin_map.contains_key(&omc)
                });
                let omc1 = pins.next().unwrap();
                let omc2 = pins.next().unwrap();
                let mut fuzzer = Fuzzer::new(FuzzerInfo::IBufUseVref(io));
                fuzzer = pin_ibuf_in(backend, fuzzer, io, BlockId::from_idx(0));
                fuzzer = pin_ibuf_in(backend, fuzzer, omc1, BlockId::from_idx(1));
                fuzzer = pin_ibuf_in(backend, fuzzer, omc2, BlockId::from_idx(2));
                fuzzer.kv.remove(&Key::Iostd(io));
                fuzzer.kv.remove(&Key::Iostd(omc1));
                fuzzer.kv.remove(&Key::Iostd(omc2));
                fuzzer.kv.remove(&Key::BankVoltage(bank));
                fuzzer = fuzzer
                    .fuzz(Key::Iostd(io), Iostd::Lvcmos25, Iostd::Sstl2I)
                    .base(Key::Iostd(omc1), Iostd::Lvcmos25)
                    .base(Key::Iostd(omc2), Iostd::Sstl2I)
                    .base(Key::BankVoltage(bank), Voltage::V25)
                    .base(Key::VrefMutex, Value::None);
                hammer.add_fuzzer_simple(fuzzer);

                let mut fuzzer = Fuzzer::new(FuzzerInfo::IBufIsVref(io));
                fuzzer = pin_ibuf(backend, fuzzer, omc1);
                fuzzer.kv.remove(&Key::Iostd(omc1));
                fuzzer.kv.remove(&Key::BankVoltage(bank));
                fuzzer = fuzzer
                    .base(Key::NetworkFlag(15), false)
                    .base(Key::NetworkFlag(29), false)
                    .base(Key::NetworkFlag(30), false)
                    .base(Key::IBufPresent(io), false)
                    .fuzz(Key::IsVref(io), false, true)
                    .base(Key::Iostd(omc1), Iostd::Sstl2I)
                    .base(Key::BankVoltage(bank), Voltage::V25)
                    .base(Key::VrefMutex, Value::None);
                if let IoCoord::Macrocell(mc) = io {
                    fuzzer = fuzzer.base(Key::OBufPresent(mc), false);
                }
                hammer.add_fuzzer_simple(fuzzer);
            }
            if let Some(emc) = backend.device.dge_pad {
                let omc = backend
                    .pin_map
                    .keys()
                    .copied()
                    .find(|&omc| omc != emc && omc != io && backend.pin_map.contains_key(&omc))
                    .unwrap();
                let mut fuzzer = Fuzzer::new(FuzzerInfo::IBufDge(io));
                fuzzer = pin_ibuf_in(backend, fuzzer, io, BlockId::from_idx(0));
                if io != emc {
                    fuzzer = pin_ibuf_in(backend, fuzzer, emc, BlockId::from_idx(1));
                }
                fuzzer = pin_ibuf_in(backend, fuzzer, omc, BlockId::from_idx(2));
                fuzzer = fuzzer
                    .base(Key::Dge, true)
                    .base(Key::IBufFlag(emc, 6), true)
                    .base(Key::IBufFlag(omc, 5), true)
                    .fuzz(Key::IBufFlag(io, 5), false, true);
                hammer.add_fuzzer_simple(fuzzer);
            }
        }
    }
    if backend.device.kind.is_xc9500x() {
        let mc = backend.pin_map.keys().copied().next().unwrap();
        let fuzzer = Fuzzer::new(FuzzerInfo::GlobalKeeper).fuzz(Key::IBufFlag(mc, 4), false, true);
        let fuzzer = pin_ibuf(backend, fuzzer, mc);
        hammer.add_fuzzer_simple(fuzzer);
    }

    if backend.device.kind == DeviceKind::Coolrunner2 {
        let fuzzer = Fuzzer::new(FuzzerInfo::GlobalKeeper)
            .base(Key::NetworkFlag(15), false)
            .fuzz(Key::NetworkFlag(29), true, false)
            .fuzz(Key::NetworkFlag(30), false, true);
        hammer.add_fuzzer_simple(fuzzer);

        for (bank, &mc) in &backend.bank_test_iob {
            for iostd in [
                Iostd::Lvttl,
                Iostd::Lvcmos33,
                Iostd::Lvcmos25,
                Iostd::Lvcmos18,
                Iostd::Lvcmos18Any,
                Iostd::Lvcmos15,
                Iostd::Sstl2I,
                Iostd::Sstl3I,
                Iostd::HstlI,
            ] {
                if iostd.is_vref() && !backend.device.has_vref {
                    continue;
                }
                let mut fuzzer = Fuzzer::new(FuzzerInfo::IBufIostd(bank, iostd));
                fuzzer = pin_ibuf(backend, fuzzer, IoCoord::Macrocell(mc));
                fuzzer.kv.remove(&Key::Iostd(IoCoord::Macrocell(mc)));
                fuzzer.kv.remove(&Key::BankVoltage(bank));
                fuzzer.kv.remove(&Key::BankMutex(bank));
                fuzzer = fuzzer
                    .fuzz(Key::BankMutex(bank), false, true)
                    .fuzz(Key::BankVoltage(bank), Voltage::V18, iostd.voltage())
                    .fuzz(Key::Iostd(IoCoord::Macrocell(mc)), Iostd::Lvcmos18, iostd);
                if iostd.is_vref() {
                    fuzzer = fuzzer.fuzz(Key::VrefMutex, false, true);
                }
                hammer.add_fuzzer_simple(fuzzer);
            }
        }

        if let Some(mc) = backend.device.dge_pad {
            let fuzzer = Fuzzer::new(FuzzerInfo::Dge)
                .fuzz(Key::Dge, false, true)
                .fuzz(Key::IBufFlag(mc, 5), false, true)
                .fuzz(Key::IBufFlag(mc, 6), false, true);
            let fuzzer = pin_ibuf(backend, fuzzer, mc);
            hammer.add_fuzzer_simple(fuzzer);
        }
    }
}

fn add_obuf_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    for &io in backend.pin_map.keys() {
        let IoCoord::Macrocell(mc) = io else {
            continue;
        };
        let clkpad = ClkPadId::from_idx(0);
        let fclk = FclkId::from_idx(0);
        let clk = backend.device.clk_pads[clkpad];
        for reg in [false, true] {
            let fi = if reg {
                FuzzerInfo::OBufPresentReg(mc)
            } else {
                FuzzerInfo::OBufPresentComb(mc)
            };
            let mut fuzzer = Fuzzer::new(fi)
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McFfPresent(mc), true)
                .fuzz(Key::McHasOut(mc, NodeKind::McQ), false, Value::CopyQ)
                .fuzz(Key::OBufPresent(mc), false, true);
            if reg {
                fuzzer = fuzzer.base(
                    Key::McFfInput(mc, InputNodeKind::SrffC),
                    Value::InputPad(clk, NodeKind::IiFclk),
                );
                fuzzer = ensure_fclk(backend, fuzzer, mc, fclk);
            }
            fuzzer = pin_ibuf(backend, fuzzer, io);
            hammer.add_fuzzer_simple(fuzzer);
        }

        let mut fuzzer = Fuzzer::new(FuzzerInfo::OBufSlew(mc))
            .base(Key::McPresent(mc), true)
            .base(Key::McHasOut(mc, NodeKind::McUim), true)
            .fuzz(Key::McSiMutex(mc), false, true)
            .base(Key::McSiPresent(mc), true)
            .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
            .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
            .base(Key::McFfPresent(mc), true)
            .base(Key::McHasOut(mc, NodeKind::McQ), Value::CopyQ)
            .base(Key::OBufPresent(mc), true)
            .fuzz(Key::OBufFlag(mc, 0), false, true);
        fuzzer = pin_ibuf(backend, fuzzer, io);
        hammer.add_fuzzer_simple(fuzzer);

        if backend.device.kind == DeviceKind::Coolrunner2 {
            let mut fuzzer = Fuzzer::new(FuzzerInfo::OBufOpenDrain(mc))
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McFfPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McQ), Value::CopyQ)
                .base(Key::OBufPresent(mc), true)
                .fuzz(Key::OBufFlag(mc, 4), false, true);
            fuzzer = pin_ibuf(backend, fuzzer, io);
            hammer.add_fuzzer_simple(fuzzer);
        }

        let srcs = match backend.device.kind {
            DeviceKind::Xc9500 | DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => vec![
                OeMuxVal::Pt,
                OeMuxVal::Foe(FoeId::from_idx(0)),
                OeMuxVal::Foe(FoeId::from_idx(1)),
                OeMuxVal::Foe(FoeId::from_idx(2)),
                OeMuxVal::Foe(FoeId::from_idx(3)),
            ],
            DeviceKind::Xpla3 => vec![
                OeMuxVal::Ct(ProductTermId::from_idx(0)),
                OeMuxVal::Ct(ProductTermId::from_idx(1)),
                OeMuxVal::Ct(ProductTermId::from_idx(2)),
                OeMuxVal::Ct(ProductTermId::from_idx(6)),
                OeMuxVal::Ut,
            ],
            DeviceKind::Coolrunner2 => vec![
                OeMuxVal::Pt,
                OeMuxVal::Ct(ProductTermId::from_idx(7)),
                OeMuxVal::Foe(FoeId::from_idx(0)),
                OeMuxVal::Foe(FoeId::from_idx(1)),
                OeMuxVal::Foe(FoeId::from_idx(2)),
                OeMuxVal::Foe(FoeId::from_idx(3)),
            ],
        };
        for src in srcs {
            for inv in [false, true] {
                if inv && !backend.device.kind.is_xc9500x() {
                    continue;
                }
                let mut fuzzer = Fuzzer::new(FuzzerInfo::OBufOe(mc, src, inv))
                    .base(Key::McPresent(mc), true)
                    .base(Key::McFlag(mc, 0), false)
                    .base(Key::McHasOut(mc, NodeKind::McUim), true)
                    .fuzz(Key::McSiMutex(mc), false, true)
                    .base(Key::McSiPresent(mc), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                    .base(Key::McFfPresent(mc), true)
                    .base(Key::McHasOut(mc, NodeKind::McQ), Value::CopyQ)
                    .base(Key::OBufPresent(mc), true)
                    .fuzz(Key::McHasOut(mc, NodeKind::McOe), false, Value::CopyOe)
                    .fuzz(Key::McFlag(mc, 29), false, inv)
                    .fuzz(Key::McFlag(mc, 14), false, true);
                let mut do_sibling = None;
                match src {
                    OeMuxVal::Pt => {
                        fuzzer = fuzzer
                            .fuzz(Key::McSiHasOut(mc, NodeKind::McSiTrst), false, true)
                            .fuzz(
                                Key::McOe(mc),
                                Value::None,
                                Value::InputSi(NodeKind::McSiTrst),
                            );
                        if backend.device.kind.is_xc9500() {
                            fuzzer = fuzzer
                                .fuzz(Key::McSiHasTerm(mc, NodeKind::McSiTrst), false, true)
                                .fuzz(
                                    Key::McSiTermImux(mc, NodeKind::McSiTrst, ImuxId::from_idx(0)),
                                    Value::None,
                                    true,
                                );
                            fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                        } else {
                            let pt = ProductTermId::from_idx(9 + mc.macrocell.to_idx() * 3);
                            fuzzer = fuzzer.base(Key::PlaHasTerm(mc.block, pt), true).fuzz(
                                Key::McSiPla(mc, NodeKind::McSiTrst, pt),
                                false,
                                true,
                            );
                            fuzzer = pin_pterms(backend, fuzzer, mc.block);
                        }
                    }
                    OeMuxVal::Foe(idx) => {
                        if idx.to_idx() >= backend.device.oe_pads.len() {
                            continue;
                        }
                        let pad = OePadId::from_idx(idx.to_idx());
                        let inp = backend.oe_pads_remapped[pad];
                        if backend.device.kind == DeviceKind::Xc9500 {
                            fuzzer = fuzzer.base(
                                Key::Foe(idx),
                                Value::OePadNode(NodeKind::IiFoe, pad, idx.to_idx() as u8),
                            );
                            for i in 0..4 {
                                let i = FoeId::from_idx(i);
                                if i != idx {
                                    fuzzer = fuzzer.base(Key::Foe(i), Value::None);
                                }
                            }
                        } else {
                            fuzzer = fuzzer.base(Key::Foe(idx), true);
                        }
                        let nk = if backend.device.kind.is_xc9500x() && inv {
                            NodeKind::IiFoeInv
                        } else {
                            NodeKind::IiFoe
                        };
                        fuzzer = fuzzer.fuzz(Key::McOe(mc), Value::None, Value::InputPad(inp, nk));
                        fuzzer = ensure_ibuf(backend, fuzzer, inp, nk);
                        do_sibling = Some(Value::InputPad(inp, nk));
                    }
                    OeMuxVal::Ct(pt) => {
                        fuzzer = fuzzer
                            .base(Key::PlaHasTerm(mc.block, pt), true)
                            .base(Key::CtPresent(mc.block, pt), true)
                            .base(Key::CtUseMutex(mc.block, pt), Value::CtUseCt)
                            .base(Key::Ut(Ut::Oe), Value::None)
                            .fuzz(Key::McOe(mc), Value::None, Value::InputCt(pt));
                        do_sibling = Some(Value::InputCt(pt));
                    }
                    OeMuxVal::Ut => {
                        let pt = ProductTermId::from_idx(7);
                        fuzzer = fuzzer
                            .base(Key::PlaHasTerm(mc.block, pt), true)
                            .base(Key::CtPresent(mc.block, pt), true)
                            .base(Key::CtUseMutex(mc.block, pt), Value::CtUseUt(Ut::Clk))
                            .base(Key::Ut(Ut::Oe), Value::Ut(mc.block, pt))
                            .fuzz(Key::McOe(mc), Value::None, Value::InputCt(pt));
                        do_sibling = Some(Value::InputCt(pt));
                    }
                    _ => unreachable!(),
                }
                if let Some(inp) = do_sibling {
                    let omc = backend
                        .pin_map
                        .keys()
                        .copied()
                        .find_map(|oio| match oio {
                            IoCoord::Ipad(_) => None,
                            IoCoord::Macrocell(omc) => {
                                if omc.block == mc.block && omc != mc {
                                    Some(omc)
                                } else {
                                    None
                                }
                            }
                        })
                        .unwrap();
                    fuzzer = fuzzer
                        .base(Key::McPresent(omc), true)
                        .base(Key::McHasOut(omc, NodeKind::McUim), true)
                        .fuzz(Key::McSiMutex(omc), false, true)
                        .base(Key::McSiPresent(omc), true)
                        .base(Key::McSiHasOut(omc, NodeKind::McSiD1), true)
                        .base(Key::McSiHasOut(omc, NodeKind::McSiD2), true)
                        .base(Key::McFfPresent(omc), true)
                        .base(Key::McFlag(omc, 29), inv)
                        .base(Key::McOe(omc), inp);
                }
                fuzzer = pin_ibuf(backend, fuzzer, IoCoord::Macrocell(mc));
                hammer.add_fuzzer_simple(fuzzer);
            }
        }
    }

    if backend.device.kind.is_xc9500() {
        for mc in backend.device.mcs() {
            let srcs = vec![
                OeMuxVal::Pt,
                OeMuxVal::Foe(FoeId::from_idx(0)),
                OeMuxVal::Foe(FoeId::from_idx(1)),
                OeMuxVal::Foe(FoeId::from_idx(2)),
                OeMuxVal::Foe(FoeId::from_idx(3)),
            ];
            for src in srcs {
                for inv in [false, true] {
                    if inv && !backend.device.kind.is_xc9500x() {
                        continue;
                    }
                    let mut fuzzer = Fuzzer::new(FuzzerInfo::McOe(mc, src, inv))
                        .base(Key::McPresent(mc), true)
                        .base(Key::McFlag(mc, 0), false)
                        .base(Key::McHasOut(mc, NodeKind::McUim), true)
                        .fuzz(Key::McSiMutex(mc), false, true)
                        .base(Key::McSiPresent(mc), true)
                        .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                        .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                        .base(Key::McFfPresent(mc), true)
                        .base(Key::OBufPresent(mc), false)
                        .fuzz(Key::McFlag(mc, 29), false, inv);

                    let mut do_sibling = None;
                    match src {
                        OeMuxVal::Pt => {
                            fuzzer = fuzzer
                                .fuzz(Key::McSiHasOut(mc, NodeKind::McSiTrst), false, true)
                                .fuzz(
                                    Key::McOe(mc),
                                    Value::None,
                                    Value::InputSi(NodeKind::McSiTrst),
                                )
                                .fuzz(Key::McSiHasTerm(mc, NodeKind::McSiTrst), false, true)
                                .fuzz(
                                    Key::McSiTermImux(mc, NodeKind::McSiTrst, ImuxId::from_idx(0)),
                                    Value::None,
                                    true,
                                );
                            fuzzer = pin_imux_inps(backend, fuzzer, mc.block);
                        }
                        OeMuxVal::Foe(idx) => {
                            if idx.to_idx() >= backend.device.oe_pads.len() {
                                continue;
                            }
                            let pad = OePadId::from_idx(idx.to_idx());
                            let inp = backend.oe_pads_remapped[pad];
                            if backend.device.kind == DeviceKind::Xc9500 {
                                fuzzer = fuzzer.base(
                                    Key::Foe(idx),
                                    Value::OePadNode(NodeKind::IiFoe, pad, idx.to_idx() as u8),
                                );
                                for i in 0..4 {
                                    let i = FoeId::from_idx(i);
                                    if i != idx {
                                        fuzzer = fuzzer.base(Key::Foe(i), Value::None);
                                    }
                                }
                            } else {
                                fuzzer = fuzzer.base(Key::Foe(idx), true);
                            }
                            let nk = if backend.device.kind.is_xc9500x() && inv {
                                NodeKind::IiFoeInv
                            } else {
                                NodeKind::IiFoe
                            };
                            fuzzer =
                                fuzzer.fuzz(Key::McOe(mc), Value::None, Value::InputPad(inp, nk));
                            fuzzer = ensure_ibuf(backend, fuzzer, inp, nk);
                            do_sibling = Some(Value::InputPad(inp, nk));
                        }
                        _ => unreachable!(),
                    }
                    if let Some(inp) = do_sibling {
                        let omc = MacrocellCoord::simple(
                            mc.block,
                            MacrocellId::from_idx(mc.macrocell.to_idx() ^ 1),
                        );
                        fuzzer = fuzzer
                            .base(Key::McPresent(omc), true)
                            .base(Key::McHasOut(omc, NodeKind::McUim), true)
                            .fuzz(Key::McSiMutex(omc), false, true)
                            .base(Key::McSiPresent(omc), true)
                            .base(Key::McSiHasOut(omc, NodeKind::McSiD1), true)
                            .base(Key::McSiHasOut(omc, NodeKind::McSiD2), true)
                            .base(Key::McFfPresent(omc), true)
                            .base(Key::McFlag(omc, 29), inv)
                            .base(Key::McOe(omc), inp);
                    }
                    hammer.add_fuzzer_simple(fuzzer);
                }
            }
        }
    }

    if backend.device.kind == DeviceKind::Coolrunner2 {
        for (bank, &mc) in &backend.bank_test_iob {
            for iostd in [
                Iostd::Lvttl,
                Iostd::Lvcmos33,
                Iostd::Lvcmos25,
                Iostd::Lvcmos18,
                Iostd::Lvcmos18Any,
                Iostd::Lvcmos15,
                Iostd::Sstl2I,
                Iostd::Sstl3I,
                Iostd::HstlI,
            ] {
                if iostd.is_vref() && !backend.device.has_vref {
                    continue;
                }
                let mut fuzzer = Fuzzer::new(FuzzerInfo::OBufIostd(bank, iostd));
                fuzzer = fuzzer
                    .base(Key::McPresent(mc), true)
                    .base(Key::McHasOut(mc, NodeKind::McUim), true)
                    .fuzz(Key::McSiMutex(mc), false, true)
                    .base(Key::McSiPresent(mc), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                    .base(Key::McFfPresent(mc), true)
                    .base(Key::McHasOut(mc, NodeKind::McQ), Value::CopyQ)
                    .base(Key::IBufPresent(IoCoord::Macrocell(mc)), false)
                    .base(Key::OBufPresent(mc), true)
                    .fuzz(Key::BankMutex(bank), false, true)
                    .fuzz(Key::BankVoltage(bank), Voltage::V18, iostd.voltage())
                    .fuzz(Key::Iostd(IoCoord::Macrocell(mc)), Iostd::Lvcmos18, iostd);
                hammer.add_fuzzer_simple(fuzzer);
            }
        }
    }
}

fn add_cdr_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind != DeviceKind::Coolrunner2 {
        return;
    }
    let Some(cdr) = backend.device.cdr_pad else {
        return;
    };
    let IoCoord::Macrocell(cdrmc) = cdr else {
        unreachable!();
    };
    let clkpad = ClkPadId::from_idx(2);
    let fclk = FclkId::from_idx(2);
    let clk = backend.device.clk_pads[clkpad];
    for div in [2, 4, 6, 8, 10, 12, 14, 16] {
        let mut fuzzer = Fuzzer::new(FuzzerInfo::ClkDiv(div));
        fuzzer = ensure_ibuf(backend, fuzzer, clk, NodeKind::IiFclk);
        fuzzer = fuzzer
            .base(Key::NetworkFlag(15), true)
            .base(Key::NetworkFlag(29), false)
            .base(Key::NetworkFlag(30), false)
            .base(Key::IBufPresent(cdr), false)
            .base(Key::OBufPresent(cdrmc), false)
            .base(Key::McPresent(cdrmc), true)
            .base(Key::McHasOut(cdrmc, NodeKind::McUim), true)
            .fuzz(Key::McSiMutex(cdrmc), false, true)
            .base(Key::McSiPresent(cdrmc), true)
            .base(Key::McSiHasOut(cdrmc, NodeKind::McSiD1), true)
            .base(Key::McSiHasOut(cdrmc, NodeKind::McSiD2), true)
            .base(Key::McFfPresent(cdrmc), true)
            .base(
                Key::McFfInput(cdrmc, InputNodeKind::SrffC),
                Value::InputPad(clk, NodeKind::IiFclk),
            )
            .base(Key::Fclk(fclk), true)
            .fuzz(Key::IBufFlag(clk, 7), false, true)
            .fuzz(Key::Cdr, Value::None, Value::Cdr(div, false));
        hammer.add_fuzzer_simple(fuzzer);
    }

    let mut fuzzer = Fuzzer::new(FuzzerInfo::ClkDivDelay);
    fuzzer = ensure_ibuf(backend, fuzzer, clk, NodeKind::IiFclk);
    fuzzer = fuzzer
        .base(Key::NetworkFlag(15), true)
        .base(Key::NetworkFlag(29), false)
        .base(Key::NetworkFlag(30), false)
        .base(Key::IBufPresent(cdr), false)
        .base(Key::OBufPresent(cdrmc), false)
        .base(Key::McPresent(cdrmc), true)
        .base(Key::McHasOut(cdrmc, NodeKind::McUim), true)
        .fuzz(Key::McSiMutex(cdrmc), false, true)
        .base(Key::McSiPresent(cdrmc), true)
        .base(Key::McSiHasOut(cdrmc, NodeKind::McSiD1), true)
        .base(Key::McSiHasOut(cdrmc, NodeKind::McSiD2), true)
        .base(Key::McFfPresent(cdrmc), true)
        .base(
            Key::McFfInput(cdrmc, InputNodeKind::SrffC),
            Value::InputPad(clk, NodeKind::IiFclk),
        )
        .base(Key::Fclk(fclk), true)
        .base(Key::IBufFlag(clk, 7), true)
        .base(Key::Cdr, Value::Cdr(2, false))
        .fuzz(Key::IBufFlag(clk, 9), false, true);
    hammer.add_fuzzer_simple(fuzzer);
}

fn add_ut_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind != DeviceKind::Xpla3 {
        return;
    }
    for ut in [Ut::Clk, Ut::Oe, Ut::Rst, Ut::Set] {
        for fb in backend.device.fbs() {
            for pt in [ProductTermId::from_idx(6), ProductTermId::from_idx(7)] {
                if pt.to_idx() == 6 && backend.device.fbs != 2 {
                    continue;
                }
                let mc = MacrocellCoord {
                    cluster: ClusterId::from_idx(0),
                    block: BlockId::from_idx(fb.to_idx() ^ 1),
                    macrocell: MacrocellId::from_idx(0),
                };
                let mut fuzzer = Fuzzer::new(FuzzerInfo::Ut(ut, fb, pt))
                    .base(Key::PlaHasTerm(fb, pt), true)
                    .fuzz(Key::CtPresent(fb, pt), false, true)
                    .base(Key::CtUseMutex(fb, pt), Value::CtUseUt(ut))
                    .fuzz(Key::Ut(ut), Value::None, Value::Ut(fb, pt))
                    .base(Key::McPresent(mc), true)
                    .base(Key::McHasOut(mc, NodeKind::McUim), true)
                    .fuzz(Key::McSiMutex(mc), false, true)
                    .base(Key::McSiPresent(mc), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                    .base(Key::McFfPresent(mc), true);
                match ut {
                    Ut::Clk => {
                        fuzzer = fuzzer.fuzz(
                            Key::McFfInput(mc, InputNodeKind::SrffC),
                            Value::None,
                            Value::InputUt(fb, pt),
                        );
                    }
                    Ut::Oe => {
                        fuzzer = fuzzer.fuzz(Key::McOe(mc), Value::None, Value::InputUt(fb, pt));
                    }
                    Ut::Set | Ut::Rst => {
                        let clkpad = ClkPadId::from_idx(0);
                        let fclk = FclkId::from_idx(0);
                        let clk = backend.device.clk_pads[clkpad];
                        fuzzer = ensure_fclk(backend, fuzzer, mc, fclk);
                        fuzzer = fuzzer
                            .base(
                                Key::McFfInput(mc, InputNodeKind::SrffC),
                                Value::InputPad(clk, NodeKind::IiFclk),
                            )
                            .fuzz(
                                Key::McFfInput(
                                    mc,
                                    if ut == Ut::Rst {
                                        InputNodeKind::SrffR
                                    } else {
                                        InputNodeKind::SrffS
                                    },
                                ),
                                Value::None,
                                Value::InputUt(fb, pt),
                            );
                    }
                }

                hammer.add_fuzzer_simple(fuzzer);
            }
        }
    }
}

fn add_ipad_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind != DeviceKind::Xpla3 {
        return;
    }
    for ipad in backend.device.ipads() {
        let io = IoCoord::Ipad(ipad);
        for fb in backend.device.fbs() {
            let fuzzer = Fuzzer::new(FuzzerInfo::IpadUimOutFb(ipad, fb))
                .fuzz(Key::IBufPresent(io), false, true)
                .fuzz(Key::IBufHasOut(io, NodeKind::IiImux), false, true)
                .fuzz(
                    Key::FbImux(fb, backend.ibuf_test_imux[&io]),
                    Value::None,
                    ImuxInput::Ibuf(io),
                )
                .base(Key::IBufOutUseMutex(io, NodeKind::IiImux), true);
            hammer.add_fuzzer_simple(fuzzer);
        }
    }
}

fn add_fbclk_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind != DeviceKind::Xpla3 {
        return;
    }
    for fb in backend.device.fbs() {
        for gclks in [
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
        ] {
            let mut fuzzer = Fuzzer::new(FuzzerInfo::FbClk(fb, gclks[0], gclks[1]));
            for (fbclk, gclk) in gclks.into_iter().enumerate() {
                let fbclk = FclkId::from_idx(fbclk);
                let Some(gclk) = gclk else {
                    fuzzer = fuzzer.base(Key::FbClk(fb, fbclk), Value::None);
                    continue;
                };
                let pad = backend.device.clk_pads[gclk];
                let mc = MacrocellCoord::simple(fb, MacrocellId::from_idx(fbclk.to_idx()));
                fuzzer = fuzzer
                    .base(Key::McPresent(mc), true)
                    .base(Key::McHasOut(mc, NodeKind::McUim), true)
                    .fuzz(Key::McSiMutex(mc), false, true)
                    .base(Key::McSiPresent(mc), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                    .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                    .base(Key::McFfPresent(mc), true)
                    .fuzz(
                        Key::McFfInput(mc, InputNodeKind::SrffC),
                        Value::None,
                        Value::InputPad(pad, NodeKind::IiFclk),
                    )
                    .fuzz(Key::FbClk(fb, fbclk), Value::None, Value::ClkPad(gclk))
                    .fuzz(
                        Key::Fclk(FclkId::from_idx(gclk.to_idx())),
                        Value::None,
                        true,
                    )
                    .fuzz(Key::IBufPresent(pad), false, true)
                    .fuzz(Key::IBufHasOut(pad, NodeKind::IiFclk), false, true);
            }
            hammer.add_fuzzer_simple(fuzzer);
        }
    }
}

fn add_fclk_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind == DeviceKind::Xpla3 {
        return;
    }
    let gclks = if backend.device.kind == DeviceKind::Xc9500 {
        vec![
            (0, 0, 0),
            (0, 1, 3),
            (1, 1, 1),
            (1, 2, 4),
            (2, 2, 2),
            (2, 0, 5),
        ]
    } else {
        vec![(0, 0, 0), (1, 1, 1), (2, 2, 2)]
    };
    for (tgt, src, path) in gclks {
        let tgt = FclkId::from_idx(tgt);
        let src = ClkPadId::from_idx(src);
        for inv in [false, true] {
            if inv && backend.device.kind != DeviceKind::Xc9500 {
                continue;
            }
            let mut fuzzer = Fuzzer::new(FuzzerInfo::Fclk(tgt, src, inv));
            let pad = backend.device.clk_pads[src];
            let mc = MacrocellCoord::simple(BlockId::from_idx(0), MacrocellId::from_idx(0));
            let nk = if inv {
                NodeKind::IiFclkInv
            } else {
                NodeKind::IiFclk
            };
            fuzzer = fuzzer
                .base(Key::NetworkFlag(15), false)
                .base(Key::NetworkFlag(29), false)
                .base(Key::NetworkFlag(30), false)
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McFfPresent(mc), true)
                .fuzz(
                    Key::McFfInput(mc, InputNodeKind::SrffC),
                    Value::None,
                    Value::InputPad(pad, nk),
                )
                .fuzz(Key::IBufPresent(pad), false, true)
                .fuzz(Key::IBufHasOut(pad, nk), false, true);

            if backend.device.kind == DeviceKind::Xc9500 {
                fuzzer = fuzzer.fuzz(
                    Key::Fclk(tgt),
                    Value::None,
                    Value::ClkPadNode(nk, src, path),
                )
            } else {
                fuzzer = fuzzer.fuzz(Key::Fclk(tgt), Value::None, true)
            }
            if backend.device.kind == DeviceKind::Coolrunner2 {
                let bank = backend.device.io[&pad].bank;
                fuzzer = fuzzer
                    .fuzz(Key::IBufFlag(pad, 2), false, true)
                    .fuzz(Key::Iostd(pad), Value::None, Iostd::Lvcmos18)
                    .base(Key::BankVoltage(bank), Voltage::V18)
                    .base(Key::BankMutex(bank), Value::None);
            }
            hammer.add_fuzzer_simple(fuzzer);
        }
    }
}

fn add_fsr_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind == DeviceKind::Xpla3 {
        return;
    }
    for inv in [false, true] {
        let mut fuzzer = Fuzzer::new(FuzzerInfo::Fsr(inv));
        let clkpad = ClkPadId::from_idx(0);
        let fclk = FclkId::from_idx(0);
        let clk = backend.device.clk_pads[clkpad];
        let pad = backend.device.sr_pad.unwrap();
        let mc = MacrocellCoord {
            cluster: ClusterId::from_idx(0),
            block: BlockId::from_idx(0),
            macrocell: MacrocellId::from_idx(0),
        };
        let nk = if inv {
            NodeKind::IiFsrInv
        } else {
            NodeKind::IiFsr
        };
        fuzzer = fuzzer
            .base(Key::NetworkFlag(15), false)
            .base(Key::NetworkFlag(29), false)
            .base(Key::NetworkFlag(30), false)
            .base(Key::McPresent(mc), true)
            .base(Key::McHasOut(mc, NodeKind::McUim), true)
            .fuzz(Key::McSiMutex(mc), false, true)
            .base(Key::McSiPresent(mc), true)
            .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
            .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
            .base(Key::McFfPresent(mc), true)
            .base(
                Key::McFfInput(mc, InputNodeKind::SrffC),
                Value::InputPad(clk, NodeKind::IiFclk),
            )
            .fuzz(
                Key::McFfInput(mc, InputNodeKind::SrffR),
                Value::None,
                Value::InputPad(pad, nk),
            )
            .fuzz(Key::IBufPresent(pad), false, true)
            .fuzz(Key::IBufHasOut(pad, nk), false, true);
        fuzzer = ensure_fclk(backend, fuzzer, mc, fclk);

        if backend.device.kind == DeviceKind::Xc9500 {
            fuzzer = fuzzer.fuzz(Key::Fsr, Value::None, Value::SrPadNode(nk))
        } else {
            fuzzer = fuzzer.fuzz(Key::Fsr, Value::None, true)
        }
        if backend.device.kind == DeviceKind::Coolrunner2 {
            let bank = backend.device.io[&pad].bank;
            fuzzer = fuzzer
                .fuzz(Key::IBufFlag(pad, 2), false, true)
                .fuzz(Key::Iostd(pad), Value::None, Iostd::Lvcmos18)
                .base(Key::BankVoltage(bank), Voltage::V18)
                .base(Key::BankMutex(bank), Value::None);
        }
        hammer.add_fuzzer_simple(fuzzer);
    }
}

fn add_foe_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind == DeviceKind::Xpla3 {
        return;
    }
    let oes = match (backend.device.kind, backend.device.oe_pads.len()) {
        (DeviceKind::Xc9500, 2) => vec![(0, 0, 0), (0, 1, 2), (1, 1, 1), (1, 0, 3)],
        (DeviceKind::Xc9500, 4) => vec![
            (0, 0, 0),
            (0, 1, 4),
            (1, 1, 1),
            (1, 2, 5),
            (2, 2, 2),
            (2, 3, 6),
            (3, 3, 3),
            (3, 0, 7),
        ],
        (_, l) => (0..(l)).map(|i| (i, i, i)).collect(),
    };
    for (tgt, src, path) in oes {
        let tgt = FoeId::from_idx(tgt);
        let src = OePadId::from_idx(src);
        for inv in [false, true] {
            if inv && backend.device.kind.is_xc9500x() {
                continue;
            }
            let mut fuzzer = Fuzzer::new(FuzzerInfo::Foe(tgt, src, inv));
            let pad = backend.oe_pads_remapped[src];
            let IoCoord::Macrocell(mc) = backend.device.clk_pads[ClkPadId::from_idx(0)] else {
                unreachable!();
            };
            let nk = if inv {
                NodeKind::IiFoeInv
            } else {
                NodeKind::IiFoe
            };
            fuzzer = fuzzer
                .base(Key::NetworkFlag(15), false)
                .base(Key::NetworkFlag(29), false)
                .base(Key::NetworkFlag(30), false)
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .base(Key::McHasOut(mc, NodeKind::McQ), Value::CopyQ)
                .fuzz(Key::McHasOut(mc, NodeKind::McOe), false, Value::CopyOe)
                .fuzz(Key::McFlag(mc, 14), false, true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McFfPresent(mc), true)
                .fuzz(Key::McOe(mc), Value::None, Value::InputPad(pad, nk))
                .fuzz(Key::IBufPresent(pad), false, true)
                .fuzz(Key::IBufHasOut(pad, nk), false, true)
                .base(Key::OBufPresent(mc), true);

            fuzzer = pin_ibuf(backend, fuzzer, IoCoord::Macrocell(mc));

            if backend.device.kind == DeviceKind::Xc9500 {
                fuzzer = fuzzer.fuzz(
                    Key::Foe(tgt),
                    Value::None,
                    Value::OePadNode(nk, src, path as u8),
                )
            } else {
                fuzzer = fuzzer.fuzz(Key::Foe(tgt), Value::None, true)
            }
            if backend.device.kind == DeviceKind::Coolrunner2 {
                let bank = backend.device.io[&pad].bank;
                fuzzer = fuzzer
                    .fuzz(Key::IBufFlag(pad, 2), false, true)
                    .fuzz(Key::Iostd(pad), Value::None, Iostd::Lvcmos18)
                    .base(Key::BankVoltage(bank), Voltage::V18)
                    .base(Key::BankMutex(bank), Value::None);
            }
            hammer.add_fuzzer_simple(fuzzer);
        }
        if backend.device.kind == DeviceKind::Coolrunner2 {
            let mut fuzzer = Fuzzer::new(FuzzerInfo::FoeMc(tgt));
            let IoCoord::Macrocell(oemc) = backend.device.oe_pads[src] else {
                unreachable!();
            };
            let IoCoord::Macrocell(mc) = backend.device.clk_pads[ClkPadId::from_idx(0)] else {
                unreachable!();
            };
            fuzzer = fuzzer
                .base(Key::McPresent(mc), true)
                .base(Key::McHasOut(mc, NodeKind::McUim), true)
                .base(Key::McHasOut(mc, NodeKind::McQ), Value::CopyQ)
                .fuzz(Key::McHasOut(mc, NodeKind::McOe), false, Value::CopyOe)
                .fuzz(Key::McFlag(mc, 14), false, true)
                .fuzz(Key::McSiMutex(mc), false, true)
                .base(Key::McSiPresent(mc), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD1), true)
                .base(Key::McSiHasOut(mc, NodeKind::McSiD2), true)
                .base(Key::McFfPresent(mc), true)
                .fuzz(
                    Key::McOe(mc),
                    Value::None,
                    Value::InputMc(oemc, NodeKind::McGlb),
                )
                .base(Key::McPresent(oemc), true)
                .base(Key::McFfInput(oemc, InputNodeKind::SrffC), Value::None)
                .base(Key::McHasOut(oemc, NodeKind::McUim), true)
                .fuzz(Key::McHasOut(oemc, NodeKind::McGlb), false, Value::CopyQ)
                .fuzz(Key::Foe(tgt), Value::None, Value::McGlb)
                .base(Key::OBufPresent(mc), true);

            fuzzer = pin_ibuf(backend, fuzzer, IoCoord::Macrocell(mc));
            hammer.add_fuzzer_simple(fuzzer);
        }
    }
}

fn add_fb_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if !backend.device.kind.is_xc9500() {
        return;
    }
    for fb in backend.device.fbs() {
        let tmc = MacrocellId::from_idx(0);
        let mut fuzzer = Fuzzer::new(FuzzerInfo::FbPresent(fb)).fuzz(
            Key::McPresent(MacrocellCoord::simple(fb, tmc)),
            false,
            true,
        );
        for mc in backend.device.fb_mcs() {
            if mc != tmc {
                fuzzer = fuzzer.base(Key::McPresent(MacrocellCoord::simple(fb, mc)), false);
            }
        }
        hammer.add_fuzzer_simple(fuzzer);
    }
}

fn add_usercode_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind.is_xc9500() {
        for i in 0..32 {
            let mut f = Fuzzer::new(FuzzerInfo::Usercode(i));
            if i == 31 {
                for j in 0..31 {
                    f = f.base(Key::Usercode(j), true);
                }
                f = f.fuzz(Key::UsercodePresent, true, false)
            } else {
                f = f
                    .base(Key::UsercodePresent, true)
                    .fuzz(Key::Usercode(i), false, true);
            }
            hammer.add_fuzzer_simple(f);
        }
    }
}

fn add_isp_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    if backend.device.kind == DeviceKind::Xpla3 {
        let fuzzer = Fuzzer::new(FuzzerInfo::NoIsp).fuzz(Key::NetworkFlag(17), false, true);
        hammer.add_fuzzer_simple(fuzzer);
    }
}

pub fn add_fuzzers(backend: &CpldBackend, hammer: &mut Session<CpldBackend>) {
    add_imux_fuzzers(backend, hammer);
    add_pterm_fuzzers(backend, hammer);
    add_or_fuzzers(backend, hammer);
    add_ct_fuzzers(backend, hammer);
    add_mc_fuzzers(backend, hammer);
    add_xor_fuzzers(backend, hammer);
    add_ibuf_fuzzers(backend, hammer);
    add_obuf_fuzzers(backend, hammer);
    add_cdr_fuzzers(backend, hammer);
    add_ut_fuzzers(backend, hammer);
    add_ipad_fuzzers(backend, hammer);
    add_fbclk_fuzzers(backend, hammer);
    add_fclk_fuzzers(backend, hammer);
    add_fsr_fuzzers(backend, hammer);
    add_foe_fuzzers(backend, hammer);
    add_fb_fuzzers(backend, hammer);
    add_usercode_fuzzers(backend, hammer);
    add_isp_fuzzers(backend, hammer);
}
