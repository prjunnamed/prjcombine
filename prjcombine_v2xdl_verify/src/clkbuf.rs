use crate::types::{SrcInst, Test, TestGenCtx, TgtInst};
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Mode {
    Virtex,
    Virtex2,
    Spartan3,
    Spartan6,
    Virtex4,
    Virtex5,
    Virtex6,
    Virtex7,
}

fn make_clk_out(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) -> String {
    let res = test.make_wire(ctx);
    let d = test.make_in(ctx);
    let q = test.make_out(ctx);
    let mut inst = SrcInst::new(ctx, "FD");
    let mut ti = TgtInst::new(&["SLICE", "SLICEL", "SLICEM", "SLICEX"]);
    inst.connect("D", &d);
    inst.connect("Q", &q);
    inst.connect("C", &res);
    match mode {
        Mode::Virtex => {
            inst.attr_str("BEL", "FFX");
            ti.bel("FFX", &inst.name, "#FF");
            ti.pin_in("CLK", &res);
            ti.cfg("CKINV", "1");
            ti.cfg("BXMUX", "BX");
            ti.cfg("INITX", "LOW");
            ti.cfg("DXMUX", "0");
            ti.pin_in("BX", &d);
            ti.pin_out("XQ", &q);
            ti.cfg("SYNC_ATTR", "ASYNC");
        }
        Mode::Virtex2 | Mode::Spartan3 | Mode::Virtex4 => {
            inst.attr_str("BEL", "FFX");
            ti.bel("FFX", &inst.name, "#FF");
            ti.cfg("FFX_INIT_ATTR", "INIT0");
            ti.cfg("FFX_SR_ATTR", "SRLOW");
            ti.pin_in_inv("CLK", &res, false);
            ti.pin_in_inv("BX", &d, false);
            ti.pin_out("XQ", &q);
            ti.cfg("SYNC_ATTR", "ASYNC");
            if mode == Mode::Virtex4 {
                ti.cfg("DXMUX", "BX");
            } else {
                ti.cfg("DXMUX", "0");
            }
        }
        _ => {
            inst.attr_str("BEL", "FFA");
            ti.bel("AFF", &inst.name, "#FF");
            ti.pin_in_inv("CLK", &res, false);
            ti.cfg("AFFMUX", "AX");
            if mode == Mode::Spartan6 {
                ti.cfg("AFFSRINIT", "SRINIT0");
            } else {
                ti.cfg("AFFINIT", "INIT0");
                ti.cfg("AFFSR", "SRLOW");
            }
            ti.pin_in("AX", &d);
            ti.pin_out("AQ", &q);
            ti.cfg("SYNC_ATTR", "ASYNC");
        }
    }
    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
    res
}

fn gen_bufg(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "BUFG");
    let prim = match mode {
        Mode::Virtex => "GCLK",
        Mode::Virtex2 | Mode::Spartan3 => "BUFGMUX",
        _ => "BUFG",
    };
    let mut ti = TgtInst::new(&[prim]);

    let i = test.make_in(ctx);
    let o = make_clk_out(test, ctx, mode);
    inst.connect("I", &i);
    inst.connect("O", &o);

    match mode {
        Mode::Virtex => {
            ti.bel("GCLK_BUFFER", &inst.name, "");
            ti.bel("CE_POWER", "DUMMY", "");
            ti.pin_in("IN", &i);
            ti.pin_out("OUT", &o);
            ti.cfg("CEMUX", "1");
            ti.cfg("DISABLE_ATTR", "LOW");
        }
        Mode::Virtex2 | Mode::Spartan3 => {
            ti.bel("GCLK_BUFFER", &inst.name, "");
            ti.bel("GCLKMUX", &format!("{}.GCLKMUX", inst.name), "");
            ti.pin_in("I0", &i);
            ti.pin_tie_inv("S", true, true);
            ti.pin_out("O", &o);
            ti.cfg("DISABLE_ATTR", "LOW");
            ti.cfg("I0_USED", "0");
        }
        Mode::Virtex4 => {
            ti.bel("GCLK_BUFFER", &inst.name, "");
            ti.pin_in("I0", &i);
            ti.pin_out("O", &o);
        }
        _ => {
            ti.bel("BUFG", &inst.name, "");
            ti.pin_in("I0", &i);
            ti.pin_out("O", &o);
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_bufgce(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let dis_attr = ctx.rng.gen();
    let mut inst = SrcInst::new(ctx, if dis_attr { "BUFGCE_1" } else { "BUFGCE" });
    let prim = match mode {
        Mode::Virtex2 | Mode::Spartan3 | Mode::Spartan6 => "BUFGMUX",
        _ => "BUFGCTRL",
    };
    let mut ti = TgtInst::new(&[prim]);

    let i = test.make_in(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    let o = make_clk_out(test, ctx, mode);
    inst.connect("I", &i);
    inst.connect("CE", &ce_v);
    inst.connect("O", &o);

    match mode {
        Mode::Virtex2 | Mode::Spartan3 => {
            if dis_attr {
                ti.bel("GCLK_BUFFER", &format!("{}/BUFGMUX_1", inst.name), "");
                ti.bel("GCLKMUX", &format!("{}/BUFGMUX_1.GCLKMUX", inst.name), "");
            } else {
                ti.bel("GCLK_BUFFER", &format!("{}/BUFGMUX", inst.name), "");
                ti.bel("GCLKMUX", &format!("{}/BUFGMUX.GCLKMUX", inst.name), "");
            }
            ti.pin_in("I0", &i);
            ti.pin_tie("I1", dis_attr);
            ti.pin_in_inv("S", &ce_x, !ce_inv);
            ti.pin_out("O", &o);
            ti.cfg("DISABLE_ATTR", if dis_attr { "HIGH" } else { "LOW" });
            ti.cfg("I0_USED", "0");
            ti.cfg("I1_USED", "0");
        }
        Mode::Spartan6 => {
            ti.bel("BUFGMUX", &inst.name, "");
            ti.pin_in("I0", &i);
            ti.pin_tie("I1", dis_attr);
            ti.pin_in_inv("S", &ce_x, !ce_inv);
            ti.pin_out("O", &o);
            ti.cfg("DISABLE_ATTR", if dis_attr { "HIGH" } else { "LOW" });
            ti.cfg("CLK_SEL_TYPE", "SYNC");
        }
        _ => {
            ti.bel("BUFGCTRL", &inst.name, "");
            ti.pin_in("I0", &i);
            ti.pin_in_inv("CE0", &ce_x, ce_inv);
            ti.pin_tie_inv("CE1", false, false);
            ti.pin_tie_inv("IGNORE0", false, false);
            ti.pin_tie_inv("IGNORE1", true, false);
            ti.pin_tie_inv("S0", true, false);
            ti.pin_tie_inv("S1", false, false);
            ti.pin_out("O", &o);
            ti.cfg("INIT_OUT", if dis_attr { "1" } else { "0" });
            ti.cfg("PRESELECT_I0", "TRUE");
            ti.cfg("PRESELECT_I1", "FALSE");
            ti.cfg("CREATE_EDGE", "TRUE");
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_bufgmux(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let dis_attr = ctx.rng.gen();
    let st = if mode != Mode::Spartan6 || ctx.rng.gen() {
        "SYNC"
    } else {
        "ASYNC"
    };
    let mut inst = SrcInst::new(ctx, if dis_attr { "BUFGMUX_1" } else { "BUFGMUX" });
    let prim = match mode {
        Mode::Virtex2 | Mode::Spartan3 | Mode::Spartan6 => "BUFGMUX",
        _ => "BUFGCTRL",
    };
    let mut ti = TgtInst::new(&[prim]);

    let i0 = test.make_in(ctx);
    let i1 = test.make_in(ctx);
    let (s_v, s_x, s_inv) = test.make_in_inv(ctx);
    let o = make_clk_out(test, ctx, mode);
    inst.connect("I0", &i0);
    inst.connect("I1", &i1);
    inst.connect("S", &s_v);
    inst.connect("O", &o);
    if mode == Mode::Spartan6 {
        inst.param_str("CLK_SEL_TYPE", st);
    }

    match mode {
        Mode::Virtex2 | Mode::Spartan3 => {
            ti.bel("GCLK_BUFFER", &inst.name, "");
            ti.bel("GCLKMUX", &format!("{}.GCLKMUX", inst.name), "");
            ti.pin_in("I0", &i0);
            ti.pin_in("I1", &i1);
            ti.pin_in_inv("S", &s_x, s_inv);
            ti.pin_out("O", &o);
            ti.cfg("DISABLE_ATTR", if dis_attr { "HIGH" } else { "LOW" });
            ti.cfg("I0_USED", "0");
            ti.cfg("I1_USED", "0");
        }
        Mode::Spartan6 => {
            ti.bel("BUFGMUX", &inst.name, "");
            ti.pin_in("I0", &i0);
            ti.pin_in("I1", &i1);
            ti.pin_in_inv("S", &s_x, s_inv);
            ti.pin_out("O", &o);
            ti.cfg("DISABLE_ATTR", if dis_attr { "HIGH" } else { "LOW" });
            ti.cfg("CLK_SEL_TYPE", st);
        }
        _ => {
            ti.bel("BUFGCTRL", &inst.name, "");
            ti.pin_in("I0", &i0);
            ti.pin_in("I1", &i1);
            ti.pin_in_inv("CE0", &s_x, !s_inv);
            ti.pin_in_inv("CE1", &s_x, s_inv);
            ti.pin_tie_inv("IGNORE0", false, false);
            ti.pin_tie_inv("IGNORE1", false, false);
            ti.pin_tie_inv("S0", true, false);
            ti.pin_tie_inv("S1", true, false);
            ti.pin_out("O", &o);
            ti.cfg("INIT_OUT", if dis_attr { "1" } else { "0" });
            ti.cfg("PRESELECT_I0", "TRUE");
            ti.cfg("PRESELECT_I1", "FALSE");
            ti.cfg("CREATE_EDGE", "TRUE");
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_bufgmux_ctrl(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let prim = if ctx.rng.gen() || mode == Mode::Virtex4 {
        "BUFGMUX_VIRTEX4"
    } else {
        "BUFGMUX_CTRL"
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&["BUFGCTRL"]);

    let i0 = test.make_in(ctx);
    let i1 = test.make_in(ctx);
    let (s_v, s_x, s_inv) = test.make_in_inv(ctx);
    let o = make_clk_out(test, ctx, mode);
    inst.connect("I0", &i0);
    inst.connect("I1", &i1);
    inst.connect("S", &s_v);
    inst.connect("O", &o);

    ti.bel("BUFGCTRL", &inst.name, "");
    ti.pin_in("I0", &i0);
    ti.pin_in("I1", &i1);
    ti.pin_in_inv("S0", &s_x, !s_inv);
    ti.pin_in_inv("S1", &s_x, s_inv);
    ti.pin_tie_inv("IGNORE0", false, false);
    ti.pin_tie_inv("IGNORE1", false, false);
    ti.pin_tie_inv("CE0", true, false);
    ti.pin_tie_inv("CE1", true, false);
    ti.pin_out("O", &o);
    ti.cfg("INIT_OUT", "0");
    ti.cfg("PRESELECT_I0", "TRUE");
    ti.cfg("PRESELECT_I1", "FALSE");
    ti.cfg("CREATE_EDGE", "TRUE");

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_bufgctrl(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "BUFGCTRL");
    let mut ti = TgtInst::new(&["BUFGCTRL"]);

    let i0 = test.make_in(ctx);
    let i1 = test.make_in(ctx);
    let (s0_v, s0_x, s0_inv) = test.make_in_inv(ctx);
    let (s1_v, s1_x, s1_inv) = test.make_in_inv(ctx);
    let (ignore0_v, ignore0_x, ignore0_inv) = test.make_in_inv(ctx);
    let (ignore1_v, ignore1_x, ignore1_inv) = test.make_in_inv(ctx);
    let (ce0_v, ce0_x, ce0_inv) = test.make_in_inv(ctx);
    let (ce1_v, ce1_x, ce1_inv) = test.make_in_inv(ctx);
    let o = make_clk_out(test, ctx, mode);
    inst.connect("I0", &i0);
    inst.connect("I1", &i1);
    inst.connect("S0", &s0_v);
    inst.connect("S1", &s1_v);
    inst.connect("CE0", &ce0_v);
    inst.connect("CE1", &ce1_v);
    inst.connect("IGNORE0", &ignore0_v);
    inst.connect("IGNORE1", &ignore1_v);
    inst.connect("O", &o);
    let init_out = ctx.gen_bits(1);
    let (pre0, pre1) = *[(false, false), (false, true), (true, false)]
        .choose(&mut ctx.rng)
        .unwrap();
    inst.param_bits("INIT_OUT", &init_out);
    inst.param_bool("PRESELECT_I0", pre0);
    inst.param_bool("PRESELECT_I1", pre1);

    ti.bel("BUFGCTRL", &inst.name, "");
    ti.pin_in("I0", &i0);
    ti.pin_in("I1", &i1);
    ti.pin_in_inv("CE0", &ce0_x, ce0_inv);
    ti.pin_in_inv("CE1", &ce1_x, ce1_inv);
    ti.pin_in_inv("IGNORE0", &ignore0_x, ignore0_inv);
    ti.pin_in_inv("IGNORE1", &ignore1_x, ignore1_inv);
    ti.pin_in_inv("S0", &s0_x, s0_inv);
    ti.pin_in_inv("S1", &s1_x, s1_inv);
    ti.pin_out("O", &o);
    ti.cfg_hex("INIT_OUT", &init_out, true);
    ti.cfg_bool("PRESELECT_I0", pre0);
    ti.cfg_bool("PRESELECT_I1", pre1);
    ti.cfg("CREATE_EDGE", "TRUE");

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn make_bufr(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, i: &str) {
    let mut inst = SrcInst::new(ctx, "BUFR");
    let mut ti = TgtInst::new(&["BUFR"]);

    let o = make_clk_out(test, ctx, mode);
    let ce = test.make_in(ctx);
    let clr = test.make_in(ctx);
    inst.connect("I", i);
    inst.connect("CE", &ce);
    inst.connect("CLR", &clr);
    inst.connect("O", &o);
    let div = *["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"]
        .choose(&mut ctx.rng)
        .unwrap();
    inst.param_str("BUFR_DIVIDE", div);

    ti.bel("BUFR", &inst.name, "");
    ti.pin_in("I", i);
    ti.pin_in("CE", &ce);
    ti.pin_in("CLR", &clr);
    ti.pin_out("O", &o);
    ti.cfg("BUFR_DIVIDE", div);

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_bufr(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let i = test.make_in(ctx);
    make_bufr(test, ctx, mode, &i);
}

fn gen_bufh(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "BUFH");

    let i = test.make_in(ctx);
    let o = make_clk_out(test, ctx, mode);
    inst.connect("I", &i);
    inst.connect("O", &o);

    match mode {
        Mode::Spartan6 => {
            let mut ti = TgtInst::new(&["BUFH"]);
            ti.bel("BUFH", &inst.name, "");
            ti.pin_in("I", &i);
            ti.pin_out("O", &o);
            test.tgt_insts.push(ti);
        }
        _ => {
            let mut ti = TgtInst::new(&["BUFHCE"]);
            ti.bel("BUFHCE", &inst.name, "");
            ti.pin_in("I", &i);
            ti.pin_out("O", &o);
            ti.pin_tie_inv("CE", true, false);
            if mode == Mode::Virtex7 {
                ti.cfg("CE_TYPE", "SYNC");
            }
            ti.cfg("INIT_OUT", "0");
            test.tgt_insts.push(ti);
        }
    }

    test.src_insts.push(inst);
}

fn gen_bufhce(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "BUFHCE");
    let mut ti = TgtInst::new(&["BUFHCE"]);

    let i = test.make_in(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    let o = make_clk_out(test, ctx, mode);
    inst.connect("I", &i);
    inst.connect("CE", &ce_v);
    inst.connect("O", &o);
    let init = ctx.gen_bits(1);
    inst.param_bits("INIT_OUT", &init);

    ti.bel("BUFHCE", &inst.name, "");
    ti.pin_in("I", &i);
    ti.pin_in_inv("CE", &ce_x, ce_inv);
    ti.pin_out("O", &o);
    ti.cfg_hex("INIT_OUT", &init, true);

    if mode == Mode::Virtex7 {
        let cet = *["SYNC", "ASYNC"].choose(&mut ctx.rng).unwrap();
        inst.param_str("CE_TYPE", cet);
        ti.cfg("CE_TYPE", cet);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_bufmr(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "BUFMR");

    let i = test.make_in(ctx);
    let o = test.make_wire(ctx);
    make_bufr(test, ctx, mode, &o);
    inst.connect("I", &i);
    inst.connect("O", &o);

    let mut ti = TgtInst::new(&["BUFMRCE"]);
    ti.bel("BUFMRCE", &inst.name, "");
    ti.pin_in("I", &i);
    ti.pin_out("O", &o);
    ti.pin_tie_inv("CE", true, false);
    ti.cfg("CE_TYPE", "SYNC");
    ti.cfg("INIT_OUT", "0");
    test.tgt_insts.push(ti);

    test.src_insts.push(inst);
}

fn gen_bufmrce(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "BUFMRCE");
    let mut ti = TgtInst::new(&["BUFMRCE"]);

    let i = test.make_in(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    let o = test.make_wire(ctx);
    make_bufr(test, ctx, mode, &o);
    inst.connect("I", &i);
    inst.connect("CE", &ce_v);
    inst.connect("O", &o);
    let init = ctx.gen_bits(1);
    inst.param_bits("INIT_OUT", &init);
    let cet = *["SYNC", "ASYNC"].choose(&mut ctx.rng).unwrap();
    inst.param_str("CE_TYPE", cet);

    ti.bel("BUFMRCE", &inst.name, "");
    ti.pin_in("I", &i);
    ti.pin_in_inv("CE", &ce_x, ce_inv);
    ti.pin_out("O", &o);
    ti.cfg_hex("INIT_OUT", &init, true);
    ti.cfg("CE_TYPE", cet);

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

pub fn gen_clkbuf(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    gen_bufg(test, ctx, mode);
    if mode != Mode::Virtex {
        gen_bufgce(test, ctx, mode);
        gen_bufgmux(test, ctx, mode);
    }
    if matches!(
        mode,
        Mode::Virtex4 | Mode::Virtex5 | Mode::Virtex6 | Mode::Virtex7
    ) {
        gen_bufgmux_ctrl(test, ctx, mode);
        gen_bufgctrl(test, ctx, mode);
        gen_bufr(test, ctx, mode);
    }
    if matches!(mode, Mode::Spartan6 | Mode::Virtex6 | Mode::Virtex7) {
        gen_bufh(test, ctx, mode);
    }
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        gen_bufhce(test, ctx, mode);
    }
    if mode == Mode::Virtex7 {
        gen_bufmr(test, ctx, mode);
        gen_bufmrce(test, ctx, mode);
    }
}
