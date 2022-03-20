use crate::types::{Test, SrcInst, TgtInst, TestGenCtx};
use rand::Rng;
use rand::seq::SliceRandom;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Mode {
    Virtex,
    Virtex2,
    Virtex2P,
    Spartan3,
    Spartan3E,
    Spartan3A,
    Spartan3ADsp,
    Spartan6,
    Virtex4,
    Virtex5,
    Virtex6,
    Series7,
}

fn make_in(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str) {
    let w = test.make_in(ctx);
    inst.connect(name, &w);
    ti.pin_in(name, &w);
}

fn make_in_inv(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str) {
    let (w_v, w_x, w_inv) = test.make_in_inv(ctx);
    inst.connect(name, &w_v);
    ti.pin_in_inv(name, &w_x, w_inv);
}

fn make_ins(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str, msb: usize, lsb: usize) {
    if msb < lsb {
        let w = test.make_ins(ctx, lsb - msb + 1);
        inst.connect_bus(name, &w);
        for (i, w) in w.iter().enumerate() {
            ti.pin_in(&format!("{name}{ii}", ii = lsb - i), w);
        }
    } else {
        let w = test.make_ins(ctx, msb - lsb + 1);
        inst.connect_bus(name, &w);
        for (i, w) in w.iter().enumerate() {
            ti.pin_in(&format!("{name}{ii}", ii = lsb + i), w);
        }
    }
}

fn make_out(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str) {
    let w = test.make_out(ctx);
    inst.connect(name, &w);
    ti.pin_out(name, &w);
}

fn make_outs(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str, msb: usize, lsb: usize) {
    if msb < lsb {
        let w = test.make_outs(ctx, lsb - msb + 1);
        inst.connect_bus(name, &w);
        for (i, w) in w.iter().enumerate() {
            ti.pin_out(&format!("{name}{ii}", ii = lsb - i), w);
        }
    } else {
        let w = test.make_outs(ctx, msb - lsb + 1);
        inst.connect_bus(name, &w);
        for (i, w) in w.iter().enumerate() {
            ti.pin_out(&format!("{name}{ii}", ii = lsb + i), w);
        }
    }
}

fn gen_bscan_v(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: Mode) {
    let prim = match pk {
        Mode::Virtex => "BSCAN_VIRTEX",
        Mode::Virtex2 => "BSCAN_VIRTEX2",
        Mode::Spartan3 => "BSCAN_SPARTAN3",
        Mode::Spartan3A => "BSCAN_SPARTAN3A",
        _ => unimplemented!(),
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["BSCAN"]);

    make_out(test, ctx, &mut inst, &mut ti, "UPDATE");
    make_out(test, ctx, &mut inst, &mut ti, "SHIFT");
    make_out(test, ctx, &mut inst, &mut ti, "RESET");
    make_out(test, ctx, &mut inst, &mut ti, "TDI");
    make_out(test, ctx, &mut inst, &mut ti, "SEL1");
    make_out(test, ctx, &mut inst, &mut ti, "DRCK1");
    make_out(test, ctx, &mut inst, &mut ti, "SEL2");
    make_out(test, ctx, &mut inst, &mut ti, "DRCK2");
    if pk != Mode::Virtex {
        make_out(test, ctx, &mut inst, &mut ti, "CAPTURE");
    }
    if matches!(pk, Mode::Spartan3A | Mode::Spartan3ADsp) {
        make_out(test, ctx, &mut inst, &mut ti, "TCK");
        make_out(test, ctx, &mut inst, &mut ti, "TMS");
    }

    if mode == Mode::Virtex {
        let (tdo1_v, tdo1_x, tdo1_inv) = test.make_in_inv(ctx);
        inst.connect("TDO1", &tdo1_v);
        ti.pin_in("TDO1", &tdo1_x);
        ti.cfg("TDO1MUX", if tdo1_inv {"TDO1_B"} else {"TDO1"});
        let (tdo2_v, tdo2_x, tdo2_inv) = test.make_in_inv(ctx);
        inst.connect("TDO2", &tdo2_v);
        ti.pin_in("TDO2", &tdo2_x);
        ti.cfg("TDO2MUX", if tdo2_inv {"TDO2_B"} else {"TDO2"});
    } else {
        make_in(test, ctx, &mut inst, &mut ti, "TDO1");
        make_in(test, ctx, &mut inst, &mut ti, "TDO2");
    }

    match mode {
        Mode::Virtex => {
            ti.bel("BSCAN_VIRTEX", &inst.name, "");
        }
        _ => {
            if pk == Mode::Virtex {
                ti.bel("BSCAN_BLACKBOX", &format!("{}/BSCAN_VIRTEX2", inst.name), "");
            } else {
                ti.bel("BSCAN_BLACKBOX", &inst.name, "");
            }
        }
    }

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_bscan_v4(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: Mode) {
    let prim = match pk {
        Mode::Virtex4 => "BSCAN_VIRTEX4",
        Mode::Virtex5 => "BSCAN_VIRTEX5",
        Mode::Virtex6 => "BSCAN_VIRTEX6",
        Mode::Spartan6 => "BSCAN_SPARTAN6",
        Mode::Series7 => "BSCANE2",
        _ => unimplemented!(),
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["BSCAN"]);

    make_out(test, ctx, &mut inst, &mut ti, "UPDATE");
    make_out(test, ctx, &mut inst, &mut ti, "CAPTURE");
    make_out(test, ctx, &mut inst, &mut ti, "SHIFT");
    make_out(test, ctx, &mut inst, &mut ti, "RESET");
    make_out(test, ctx, &mut inst, &mut ti, "TDI");
    make_out(test, ctx, &mut inst, &mut ti, "SEL");
    make_out(test, ctx, &mut inst, &mut ti, "DRCK");
    if matches!(pk, Mode::Virtex6 | Mode::Series7 | Mode::Spartan6) {
        make_out(test, ctx, &mut inst, &mut ti, "TCK");
        make_out(test, ctx, &mut inst, &mut ti, "TMS");
        make_out(test, ctx, &mut inst, &mut ti, "RUNTEST");
    }

    make_in(test, ctx, &mut inst, &mut ti, "TDO");
    let chain = ctx.rng.gen_range(1..5);
    inst.param_int("JTAG_CHAIN", chain);
    ti.cfg_int("JTAG_CHAIN", chain);
    if matches!(pk, Mode::Virtex6 | Mode::Series7) {
        let dis = ctx.rng.gen();
        inst.param_bool("DISABLE_JTAG", dis);
        ti.cfg_bool("DISABLE_JTAG", dis);
    } else if matches!(mode, Mode::Virtex6 | Mode::Series7) {
        ti.cfg_bool("DISABLE_JTAG", false);
    }

    if mode == Mode::Spartan6 {
        ti.cfg("JTAG_TEST", "0");
    }

    ti.bel("BSCAN", &inst.name, "");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_startup_v(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: Mode) {
    let prim = match pk {
        Mode::Virtex => "STARTUP_VIRTEX",
        Mode::Virtex2 => "STARTUP_VIRTEX2",
        Mode::Spartan3 => "STARTUP_SPARTAN3",
        Mode::Spartan3E => "STARTUP_SPARTAN3E",
        Mode::Spartan3A => "STARTUP_SPARTAN3A",
        _ => unimplemented!(),
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["STARTUP"]);

    let (gts_v, gts_x, gts_inv) = test.make_in_inv(ctx);
    let (gsr_v, gsr_x, gsr_inv) = test.make_in_inv(ctx);
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    inst.connect("GTS", &gts_v);
    inst.connect("GSR", &gsr_v);
    inst.connect("CLK", &clk_v);
    if mode == Mode::Virtex {
        ti.pin_in("GTS", &gts_x);
        ti.cfg("GTSMUX", if gts_inv {"GTS_B"} else {"GTS"});
        ti.pin_in("GSR", &gsr_x);
        ti.cfg("GSRMUX", if gsr_inv {"GSR_B"} else {"GSR"});
        ti.pin_in("CLK", &clk_x);
        ti.cfg("CLKINV", if clk_inv {"0"} else {"1"});
    } else {
        ti.pin_in_inv("GTS", &gts_x, gts_inv);
        ti.pin_in_inv("GSR", &gsr_x, gsr_inv);
        ti.pin_in_inv("CLK", &clk_x, clk_inv);
    }

    if pk == Mode::Spartan3E && mode == Mode::Spartan3E {
        make_in(test, ctx, &mut inst, &mut ti, "MBT");
    }

    match mode {
        Mode::Virtex => {
            ti.bel("STARTUP", &inst.name, "");
        }
        _ => {
            if pk == Mode::Virtex {
                ti.bel("STARTUP", &format!("{}/STARTUP_VIRTEX2", inst.name), "");
            } else {
                ti.bel("STARTUP", &inst.name, "");
            }
        }
    }

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_startup_s6(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "STARTUP_SPARTAN6");
    let mut ti = TgtInst::new(&["STARTUP"]);

    ti.bel("STARTUP", &inst.name, "");
    make_in(test, ctx, &mut inst, &mut ti, "CLK");
    make_in(test, ctx, &mut inst, &mut ti, "GTS");
    make_in(test, ctx, &mut inst, &mut ti, "GSR");
    make_in(test, ctx, &mut inst, &mut ti, "KEYCLEARB");
    make_out(test, ctx, &mut inst, &mut ti, "EOS");
    make_out(test, ctx, &mut inst, &mut ti, "CFGMCLK");
    make_out(test, ctx, &mut inst, &mut ti, "CFGCLK");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_startup_v4(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: Mode) {
    let prim = match pk {
        Mode::Virtex4 => "STARTUP_VIRTEX4",
        Mode::Virtex5 => "STARTUP_VIRTEX5",
        Mode::Virtex6 => "STARTUP_VIRTEX6",
        Mode::Series7 => "STARTUPE2",
        _ => unimplemented!(),
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["STARTUP"]);

    ti.bel("STARTUP", &inst.name, "");
    if mode == Mode::Virtex4 {
        make_in_inv(test, ctx, &mut inst, &mut ti, "CLK");
        make_in_inv(test, ctx, &mut inst, &mut ti, "GTS");
        make_in_inv(test, ctx, &mut inst, &mut ti, "GSR");
        make_in_inv(test, ctx, &mut inst, &mut ti, "USRCCLKO");
        make_in_inv(test, ctx, &mut inst, &mut ti, "USRCCLKTS");
        make_in_inv(test, ctx, &mut inst, &mut ti, "USRDONEO");
        make_in_inv(test, ctx, &mut inst, &mut ti, "USRDONETS");
    } else {
        make_in(test, ctx, &mut inst, &mut ti, "CLK");
        make_in(test, ctx, &mut inst, &mut ti, "GTS");
        make_in(test, ctx, &mut inst, &mut ti, "GSR");
        make_in(test, ctx, &mut inst, &mut ti, "USRCCLKO");
        make_in(test, ctx, &mut inst, &mut ti, "USRCCLKTS");
        make_in(test, ctx, &mut inst, &mut ti, "USRDONEO");
        make_in(test, ctx, &mut inst, &mut ti, "USRDONETS");
    }
    make_out(test, ctx, &mut inst, &mut ti, "EOS");
    if mode == Mode::Virtex6 {
        // special hack for unused GTX
        ti.pin_out("CFGMCLK", "STARTUP_CFGMCLK");
    }
    if pk != Mode::Virtex4 {
        make_out(test, ctx, &mut inst, &mut ti, "CFGCLK");
        if mode != Mode::Virtex6 {
            make_out(test, ctx, &mut inst, &mut ti, "CFGMCLK");
        }
    }
    if matches!(pk, Mode::Virtex5 | Mode::Virtex6) && mode != Mode::Series7 {
        make_out(test, ctx, &mut inst, &mut ti, "TCKSPI");
        make_out(test, ctx, &mut inst, &mut ti, "DINSPI");
    }
    if matches!(pk, Mode::Virtex6 | Mode::Series7) {
        make_in(test, ctx, &mut inst, &mut ti, "KEYCLEARB");
        make_in(test, ctx, &mut inst, &mut ti, "PACK");
        make_out(test, ctx, &mut inst, &mut ti, "PREQ");
        let prog_usr = ctx.rng.gen();
        inst.param_bool("PROG_USR", prog_usr);
        ti.cfg_bool("PROG_USR", prog_usr);
    } else if matches!(mode, Mode::Virtex6 | Mode::Series7) {
        ti.cfg_bool("PROG_USR", false);
        ti.pin_tie("KEYCLEARB", true);
        ti.pin_tie("PACK", false);
    }

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_capture(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: Mode) {
    let prim = match pk {
        Mode::Virtex => "CAPTURE_VIRTEX",
        Mode::Virtex2 => "CAPTURE_VIRTEX2",
        Mode::Spartan3 => "CAPTURE_SPARTAN3",
        Mode::Spartan3A => "CAPTURE_SPARTAN3A",
        Mode::Virtex4 => "CAPTURE_VIRTEX4",
        Mode::Virtex5 => "CAPTURE_VIRTEX5",
        Mode::Virtex6 => "CAPTURE_VIRTEX6",
        Mode::Series7 => "CAPTUREE2",
        _ => unimplemented!(),
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["CAPTURE"]);

    let oneshot = ctx.rng.gen();
    inst.param_bool("ONESHOT", oneshot);

    if matches!(mode, Mode::Virtex | Mode::Virtex2 | Mode::Virtex2P | Mode::Spartan3 | Mode::Spartan3E) {
        if oneshot {
            ti.cfg("ONESHOT_ATTR", "ONE_SHOT");
        }
    } else {
        ti.cfg_bool("ONESHOT", oneshot);
    }

    match mode {
        Mode::Virtex => {
            ti.bel("CAPTURE_BLACKBOX", &inst.name, "");
            let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
            inst.connect("CLK", &clk_v);
            ti.pin_in("CLK", &clk_x);
            ti.cfg("CLKINV", if clk_inv {"0"} else {"1"});
            let (cap_v, cap_x, cap_inv) = test.make_in_inv(ctx);
            inst.connect("CAP", &cap_v);
            ti.pin_in("CAP", &cap_x);
            ti.cfg("CAPMUX", if cap_inv {"CAP_B"} else {"CAP"});
        }
        Mode::Virtex2 | Mode::Virtex2P | Mode::Spartan3 | Mode::Spartan3E | Mode::Spartan3A | Mode::Spartan3ADsp | Mode::Virtex4 => {
            if mode == Mode::Virtex4 {
                ti.bel("CAPTURE", &inst.name, "");
            } else if pk == Mode::Virtex {
                ti.bel("CAPTURE_BLACKBOX", &format!("{}/CAPTURE_VIRTEX2", inst.name), "");
            } else {
                ti.bel("CAPTURE_BLACKBOX", &inst.name, "");
            }
            let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
            inst.connect("CLK", &clk_v);
            ti.pin_in_inv("CLK", &clk_x, clk_inv);
            let (cap_v, cap_x, cap_inv) = test.make_in_inv(ctx);
            inst.connect("CAP", &cap_v);
            ti.pin_in_inv("CAP", &cap_x, cap_inv);
        }
        _ => {
            ti.bel("CAPTURE", &inst.name, "");
            make_in(test, ctx, &mut inst, &mut ti, "CLK");
            make_in(test, ctx, &mut inst, &mut ti, "CAP");
        }
    }

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_icap(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: Mode) {
    let (prim, w) = match pk {
        Mode::Virtex2 => ("ICAP_VIRTEX2", 8),
        Mode::Spartan3A => ("ICAP_SPARTAN3A", 8),
        Mode::Spartan6 => ("ICAP_SPARTAN6", 16),
        Mode::Virtex4 => ("ICAP_VIRTEX4", 32),
        Mode::Virtex5 => ("ICAP_VIRTEX5", 32),
        Mode::Virtex6 => ("ICAP_VIRTEX6", 32),
        Mode::Series7 => ("ICAPE2", 32),
        _ => unimplemented!(),
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["ICAP"]);

    if matches!(pk, Mode::Virtex4 | Mode::Virtex5) && mode == Mode::Series7 {
        ti.bel("ICAP", &format!("{}/ICAP_VIRTEX6", inst.name), "");
    } else {
        ti.bel("ICAP", &inst.name, "");
    }
    if matches!(mode, Mode::Virtex2 | Mode::Virtex2P | Mode::Spartan3A | Mode::Spartan3ADsp | Mode::Virtex4) {
        make_in_inv(test, ctx, &mut inst, &mut ti, "CLK");
        make_in_inv(test, ctx, &mut inst, &mut ti, "CE");
        make_in_inv(test, ctx, &mut inst, &mut ti, "WRITE");
    } else {
        make_in(test, ctx, &mut inst, &mut ti, "CLK");
        let ce = test.make_in(ctx);
        let write = test.make_in(ctx);
        if pk == Mode::Series7 {
            inst.connect("CSIB", &ce);
            inst.connect("RDWRB", &write);
        } else if pk == Mode::Virtex6 {
            inst.connect("CSB", &ce);
            inst.connect("RDWRB", &write);
        } else {
            inst.connect("CE", &ce);
            inst.connect("WRITE", &write);
        }
        if mode == Mode::Series7 {
            ti.pin_in("CSIB", &ce);
            ti.pin_in("RDWRB", &write);
        } else if mode == Mode::Virtex6 {
            ti.pin_in("CSB", &ce);
            ti.pin_in("RDWRB", &write);
        } else {
            ti.pin_in("CE", &ce);
            ti.pin_in("WRITE", &write);
        }
    }
    make_ins(test, ctx, &mut inst, &mut ti, "I", w-1, 0);
    make_outs(test, ctx, &mut inst, &mut ti, "O", w-1, 0);
    if mode != Mode::Series7 {
        make_out(test, ctx, &mut inst, &mut ti, "BUSY");
    }
    if matches!(mode, Mode::Virtex4 | Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
        let width;
        if mode == Mode::Virtex4 {
            width = *["X8", "X32"].choose(&mut ctx.rng).unwrap();
        } else {
            width = *["X8", "X16", "X32"].choose(&mut ctx.rng).unwrap();
        }
        inst.param_str("ICAP_WIDTH", width);
        ti.cfg("ICAP_WIDTH", width);
    }
    if matches!(mode, Mode::Virtex6 | Mode::Series7) {
        ti.cfg("ICAP_AUTO_SWITCH", "DISABLE");
    }

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_usr_access(test: &mut Test, ctx: &mut TestGenCtx, pk: Mode) {
    let prim = match pk {
        Mode::Virtex4 => "USR_ACCESS_VIRTEX4",
        Mode::Virtex5 => "USR_ACCESS_VIRTEX5",
        Mode::Virtex6 => "USR_ACCESS_VIRTEX6",
        Mode::Series7 => "USR_ACCESSE2",
        _ => unimplemented!(),
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["USR_ACCESS"]);

    ti.bel("USR_ACCESS", &inst.name, "");
    make_out(test, ctx, &mut inst, &mut ti, "DATAVALID");
    make_outs(test, ctx, &mut inst, &mut ti, "DATA", 31, 0);
    if pk != Mode::Virtex4 {
        make_out(test, ctx, &mut inst, &mut ti, "CFGCLK");
    }

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_frame_ecc(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: Mode) {
    let prim = match pk {
        Mode::Virtex4 => "FRAME_ECC_VIRTEX4",
        Mode::Virtex5 => "FRAME_ECC_VIRTEX5",
        Mode::Virtex6 => "FRAME_ECC_VIRTEX6",
        Mode::Series7 => "FRAME_ECCE2",
        _ => unimplemented!(),
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["FRAME_ECC"]);

    ti.bel("FRAME_ECC", &inst.name, "");
    let error = test.make_out(ctx);
    if pk == Mode::Virtex4 {
        inst.connect("ERROR", &error);
    } else {
        inst.connect("ECCERROR", &error);
    }
    if mode == Mode::Virtex4 {
        ti.pin_out("ERROR", &error);
    } else {
        ti.pin_out("ECCERROR", &error);
    }
    if pk != Mode::Virtex4 {
        make_out(test, ctx, &mut inst, &mut ti, "CRCERROR");
    }
    if matches!(pk, Mode::Virtex6 | Mode::Series7) {
        make_out(test, ctx, &mut inst, &mut ti, "ECCERRORSINGLE");
        make_outs(test, ctx, &mut inst, &mut ti, "SYNDROME", 12, 0);
        make_outs(test, ctx, &mut inst, &mut ti, "SYNBIT", 4, 0);
        make_outs(test, ctx, &mut inst, &mut ti, "SYNWORD", 6, 0);
        if pk == Mode::Virtex6 {
            make_outs(test, ctx, &mut inst, &mut ti, "FAR", 23, 0);
        } else {
            make_outs(test, ctx, &mut inst, &mut ti, "FAR", 25, 0);
        }
        let far = if ctx.rng.gen() {"FAR"} else {"EFAR"};
        inst.param_str("FARSRC", &far);
        ti.cfg("FARSRC", &far);
    } else {
        make_outs(test, ctx, &mut inst, &mut ti, "SYNDROME", 11, 0);
    }
    make_out(test, ctx, &mut inst, &mut ti, "SYNDROMEVALID");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_jtagppc(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "JTAGPPC");
    let mut ti = TgtInst::new(&["JTAGPPC"]);

    if mode == Mode::Virtex2P {
        ti.bel("JTAGPPC_BLACKBOX", &inst.name, "");
    } else {
        ti.bel("JTAGPPC", &inst.name, "");
    }
    make_out(test, ctx, &mut inst, &mut ti, "TCK");
    make_out(test, ctx, &mut inst, &mut ti, "TMS");
    make_out(test, ctx, &mut inst, &mut ti, "TDIPPC");
    make_in(test, ctx, &mut inst, &mut ti, "TDOPPC");
    let tdots = test.make_in(ctx);
    inst.connect("TDOTSPPC", &tdots);
    if mode == Mode::Virtex2P {
        ti.pin_in("TDOTSPPC", &tdots);
    }

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_jtagppc440(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "JTAGPPC440");
    let mut ti = TgtInst::new(&["JTAGPPC"]);

    ti.bel("JTAGPPC", &inst.name, "");
    make_out(test, ctx, &mut inst, &mut ti, "TCK");
    make_out(test, ctx, &mut inst, &mut ti, "TMS");
    make_out(test, ctx, &mut inst, &mut ti, "TDIPPC");
    make_in(test, ctx, &mut inst, &mut ti, "TDOPPC");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_dna_port(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "DNA_PORT");
    let mut ti = TgtInst::new(&["DNA_PORT"]);

    ti.bel("DNA_PORT", &inst.name, "");
    make_out(test, ctx, &mut inst, &mut ti, "DOUT");
    make_in(test, ctx, &mut inst, &mut ti, "DIN");
    make_in(test, ctx, &mut inst, &mut ti, "READ");
    make_in(test, ctx, &mut inst, &mut ti, "SHIFT");
    make_in(test, ctx, &mut inst, &mut ti, "CLK");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_spi_access(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "SPI_ACCESS");
    let mut ti = TgtInst::new(&["SPI_ACCESS"]);

    ti.bel("SPI_ACCESS", &inst.name, "");
    make_out(test, ctx, &mut inst, &mut ti, "MISO");
    make_in(test, ctx, &mut inst, &mut ti, "MOSI");
    make_in(test, ctx, &mut inst, &mut ti, "CSB");
    make_in(test, ctx, &mut inst, &mut ti, "CLK");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_suspend_sync(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "SUSPEND_SYNC");
    let mut ti = TgtInst::new(&["SUSPEND_SYNC"]);

    ti.bel("SUSPEND_SYNC", &inst.name, "");
    make_out(test, ctx, &mut inst, &mut ti, "SREQ");
    make_in(test, ctx, &mut inst, &mut ti, "SACK");
    make_in(test, ctx, &mut inst, &mut ti, "CLK");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_post_crc_internal(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "POST_CRC_INTERNAL");
    let mut ti = TgtInst::new(&["POST_CRC_INTERNAL"]);

    ti.bel("POST_CRC_INTERNAL", &inst.name, "");
    make_out(test, ctx, &mut inst, &mut ti, "CRCERROR");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_key_clear(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "KEY_CLEAR");
    let mut ti = TgtInst::new(&["KEY_CLEAR"]);

    ti.bel("KEY_CLEAR", &inst.name, "");
    make_in(test, ctx, &mut inst, &mut ti, "KEYCLEARB");

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

fn gen_efuse_usr(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "EFUSE_USR");
    let mut ti = TgtInst::new(&["EFUSE_USR"]);

    ti.bel("EFUSE_USR", &inst.name, "");
    make_outs(test, ctx, &mut inst, &mut ti, "EFUSEUSR", 31, 0);

    test.tgt_insts.push(ti);
    test.src_insts.push(inst);
}

pub fn gen_cfg(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    match mode {
        Mode::Virtex => {
            gen_bscan_v(test, ctx, mode, Mode::Virtex);
            gen_startup_v(test, ctx, mode, Mode::Virtex);
            gen_capture(test, ctx, mode, Mode::Virtex);
        }
        Mode::Virtex2 | Mode::Virtex2P => {
            if ctx.rng.gen() {
                gen_bscan_v(test, ctx, mode, Mode::Virtex);
            } else {
                gen_bscan_v(test, ctx, mode, Mode::Virtex2);
            }
            if ctx.rng.gen() {
                gen_startup_v(test, ctx, mode, Mode::Virtex);
            } else {
                gen_startup_v(test, ctx, mode, Mode::Virtex2);
            }
            if ctx.rng.gen() {
                gen_capture(test, ctx, mode, Mode::Virtex);
            } else {
                gen_capture(test, ctx, mode, Mode::Virtex2);
            }
            gen_icap(test, ctx, mode, Mode::Virtex2);
            if mode == Mode::Virtex2P {
                gen_jtagppc(test, ctx, mode);
            }
        }
        Mode::Spartan3 | Mode::Spartan3E => {
            gen_bscan_v(test, ctx, mode, Mode::Spartan3);
            if mode == Mode::Spartan3E && ctx.rng.gen() {
                gen_startup_v(test, ctx, mode, Mode::Spartan3E);
            } else {
                gen_startup_v(test, ctx, mode, Mode::Spartan3);
            }
            gen_capture(test, ctx, mode, Mode::Spartan3);
        }
        Mode::Spartan3A | Mode::Spartan3ADsp => {
            if ctx.rng.gen() {
                gen_bscan_v(test, ctx, mode, Mode::Spartan3);
            } else {
                gen_bscan_v(test, ctx, mode, Mode::Spartan3A);
            }
            let pk = *[Mode::Spartan3, Mode::Spartan3E, Mode::Spartan3A].choose(&mut ctx.rng).unwrap();
            gen_startup_v(test, ctx, mode, pk);
            if ctx.rng.gen() {
                gen_capture(test, ctx, mode, Mode::Spartan3);
            } else {
                gen_capture(test, ctx, mode, Mode::Spartan3A);
            }
            gen_icap(test, ctx, mode, Mode::Spartan3A);
            if mode == Mode::Spartan3A {
                // ... actually 3an
                gen_spi_access(test, ctx);
            }
            gen_dna_port(test, ctx);
        }
        Mode::Virtex4 => {
            gen_bscan_v4(test, ctx, mode, Mode::Virtex4);
            gen_startup_v4(test, ctx, mode, Mode::Virtex4);
            if ctx.rng.gen() {
                gen_capture(test, ctx, mode, Mode::Virtex4);
            } else {
                gen_frame_ecc(test, ctx, mode, Mode::Virtex4);
            }
            gen_icap(test, ctx, mode, Mode::Virtex4);
            gen_usr_access(test, ctx, Mode::Virtex4);
            gen_jtagppc(test, ctx, mode);
        }
        Mode::Virtex5 => {
            let pk = *[Mode::Virtex4, Mode::Virtex5].choose(&mut ctx.rng).unwrap();
            gen_bscan_v4(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5].choose(&mut ctx.rng).unwrap();
            gen_startup_v4(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5].choose(&mut ctx.rng).unwrap();
            if ctx.rng.gen() {
                gen_capture(test, ctx, mode, pk);
            } else {
                gen_frame_ecc(test, ctx, mode, pk);
            }
            let pk = *[Mode::Virtex4, Mode::Virtex5].choose(&mut ctx.rng).unwrap();
            gen_icap(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5].choose(&mut ctx.rng).unwrap();
            gen_usr_access(test, ctx, pk);
            gen_jtagppc440(test, ctx);
            gen_key_clear(test, ctx);
            gen_efuse_usr(test, ctx);
        }
        Mode::Virtex6 => {
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6].choose(&mut ctx.rng).unwrap();
            gen_bscan_v4(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6].choose(&mut ctx.rng).unwrap();
            gen_startup_v4(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6].choose(&mut ctx.rng).unwrap();
            if ctx.rng.gen() {
                gen_capture(test, ctx, mode, pk);
            } else {
                gen_frame_ecc(test, ctx, mode, pk);
            }
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6].choose(&mut ctx.rng).unwrap();
            gen_icap(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6].choose(&mut ctx.rng).unwrap();
            gen_usr_access(test, ctx, pk);
            gen_dna_port(test, ctx);
            gen_efuse_usr(test, ctx);
        }
        Mode::Series7 => {
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6, Mode::Series7].choose(&mut ctx.rng).unwrap();
            gen_bscan_v4(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6, Mode::Series7].choose(&mut ctx.rng).unwrap();
            gen_startup_v4(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6, Mode::Series7].choose(&mut ctx.rng).unwrap();
            if ctx.rng.gen() {
                gen_capture(test, ctx, mode, pk);
            } else {
                gen_frame_ecc(test, ctx, mode, pk);
            }
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6, Mode::Series7].choose(&mut ctx.rng).unwrap();
            gen_icap(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6, Mode::Series7].choose(&mut ctx.rng).unwrap();
            gen_usr_access(test, ctx, pk);
            gen_dna_port(test, ctx);
            gen_efuse_usr(test, ctx);
        }
        Mode::Spartan6 => {
            gen_bscan_v4(test, ctx, mode, Mode::Spartan6);
            gen_startup_s6(test, ctx);
            gen_icap(test, ctx, mode, Mode::Spartan6);
            gen_dna_port(test, ctx);
            gen_post_crc_internal(test, ctx);
            gen_suspend_sync(test, ctx);
        }
    }
    // XXX JTAGPPC
}
