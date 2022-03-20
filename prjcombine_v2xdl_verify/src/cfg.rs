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
        Mode::Spartan3A | Mode::Spartan3ADsp => "BSCAN_SPARTAN3A",
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

fn gen_capture(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: Mode) {
    let prim = match pk {
        Mode::Virtex => "CAPTURE_VIRTEX",
        Mode::Virtex2 => "CAPTURE_VIRTEX2",
        Mode::Spartan3 => "CAPTURE_SPARTAN3",
        Mode::Spartan3A | Mode::Spartan3ADsp => "CAPTURE_SPARTAN3A",
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
            gen_capture(test, ctx, mode, Mode::Virtex);
        }
        Mode::Virtex2 | Mode::Virtex2P => {
            if ctx.rng.gen() {
                gen_bscan_v(test, ctx, mode, Mode::Virtex);
            } else {
                gen_bscan_v(test, ctx, mode, Mode::Virtex2);
            }
            if ctx.rng.gen() {
                gen_capture(test, ctx, mode, Mode::Virtex);
            } else {
                gen_capture(test, ctx, mode, Mode::Virtex2);
            }
            if mode == Mode::Virtex2P {
                gen_jtagppc(test, ctx, mode);
            }
        }
        Mode::Spartan3 | Mode::Spartan3E => {
            gen_bscan_v(test, ctx, mode, Mode::Spartan3);
            gen_capture(test, ctx, mode, Mode::Spartan3);
        }
        Mode::Spartan3A | Mode::Spartan3ADsp => {
            if ctx.rng.gen() {
                gen_bscan_v(test, ctx, mode, Mode::Spartan3);
            } else {
                gen_bscan_v(test, ctx, mode, Mode::Spartan3A);
            }
            if ctx.rng.gen() {
                gen_capture(test, ctx, mode, Mode::Spartan3);
            } else {
                gen_capture(test, ctx, mode, Mode::Spartan3A);
            }
            if mode == Mode::Spartan3A {
                // ... actually 3an
                gen_spi_access(test, ctx);
            }
            gen_dna_port(test, ctx);
        }
        Mode::Virtex4 => {
            gen_bscan_v4(test, ctx, mode, Mode::Virtex4);
            gen_capture(test, ctx, mode, Mode::Virtex4);
            gen_usr_access(test, ctx, Mode::Virtex4);
            gen_jtagppc(test, ctx, mode);
        }
        Mode::Virtex5 => {
            let pk = *[Mode::Virtex4, Mode::Virtex5].choose(&mut ctx.rng).unwrap();
            gen_bscan_v4(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5].choose(&mut ctx.rng).unwrap();
            gen_capture(test, ctx, mode, pk);
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
            gen_capture(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6].choose(&mut ctx.rng).unwrap();
            gen_usr_access(test, ctx, pk);
            gen_dna_port(test, ctx);
            gen_efuse_usr(test, ctx);
        }
        Mode::Series7 => {
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6, Mode::Series7].choose(&mut ctx.rng).unwrap();
            gen_bscan_v4(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6, Mode::Series7].choose(&mut ctx.rng).unwrap();
            gen_capture(test, ctx, mode, pk);
            let pk = *[Mode::Virtex4, Mode::Virtex5, Mode::Virtex6, Mode::Series7].choose(&mut ctx.rng).unwrap();
            gen_usr_access(test, ctx, pk);
            gen_dna_port(test, ctx);
            gen_efuse_usr(test, ctx);
        }
        Mode::Spartan6 => {
            gen_bscan_v4(test, ctx, mode, Mode::Spartan6);
            gen_dna_port(test, ctx);
            gen_post_crc_internal(test, ctx);
            gen_suspend_sync(test, ctx);
        }
    }
    // XXX JTAGPPC
}
