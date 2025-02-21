use crate::types::{BitVal, SrcInst, Test, TestGenCtx, TgtInst};

use rand::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Mode {
    Virtex,
    Virtex2,
    Spartan3,
    Virtex4,
}

fn gen_lut_init(sz: u8, ctx: &mut TestGenCtx) -> Vec<BitVal> {
    loop {
        let res = ctx.gen_bits(1 << sz);
        let mut fail = false;
        for i in 0..sz {
            let mut eq = true;
            for k in 0..(1 << sz) {
                if res[k] != res[k ^ (1 << i)] {
                    eq = false;
                }
            }
            if eq {
                fail = true;
            }
        }
        if !fail {
            return res;
        }
    }
}

fn compile_lut(ssz: u8, dsz: u8, init: &[BitVal]) -> u64 {
    let mut val = 0;
    for i in 0..(1 << dsz) {
        if init[i & ((1 << ssz) - 1)] == BitVal::S1 {
            val |= 1 << i;
        }
    }
    val
}

fn make_lut4(test: &mut Test, ctx: &mut TestGenCtx, ti: &mut TgtInst, c: char, out: &str) {
    let mut inst = SrcInst::new(ctx, "LUT4");

    let inp = test.make_ins(ctx, 4);
    for i in 0..4 {
        inst.connect(&format!("I{i}"), &inp[i]);
        ti.pin_in(&format!("{c}{ii}", ii = i + 1), &inp[i]);
    }

    inst.attr_str("BEL", &format!("{c}"));
    inst.connect("O", out);

    let init = gen_lut_init(4, ctx);
    inst.param_bits("INIT", &init);

    ti.bel_lut(&format!("{c}"), &inst.name, 4, compile_lut(4, 4, &init));

    test.src_insts.push(inst);
}

fn make_ffs(
    test: &mut Test,
    ctx: &mut TestGenCtx,
    mode: Mode,
    ti: &mut TgtInst,
    ffs: &[(char, &str)],
    clk: Option<&str>,
    nosr: bool,
    norev: bool,
    uset: Option<(&str, &str)>,
    is_byp: bool,
) {
    if !is_byp && mode == Mode::Virtex2 {
        for &(c, _) in ffs {
            ti.cfg(&format!("{c}USED"), "0");
            let tmp = test.make_wire(ctx);
            ti.pin_out(&format!("{c}"), &tmp);
            ti.pin_in(&format!("D{c}"), &tmp);
        }
    }
    let latch = clk.is_none() && ctx.rng.random();
    let clk_v = match clk {
        None => {
            let (x, clk_x, clk_inv) = test.make_in_inv(ctx);
            if mode == Mode::Virtex {
                ti.cfg("CKINV", if clk_inv ^ latch { "0" } else { "1" });
            } else {
                ti.cfg("CLKINV", if clk_inv ^ latch { "CLK_B" } else { "CLK" });
            }
            ti.pin_in("CLK", &clk_x);
            x
        }
        Some(x) => x.to_string(),
    };
    let ce = if ctx.rng.random() {
        let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
        if mode == Mode::Virtex {
            ti.cfg("CEMUX", if ce_inv { "CE_B" } else { "CE" });
        } else {
            ti.cfg("CEINV", if ce_inv { "CE_B" } else { "CE" });
        }
        ti.pin_in("CE", &ce_x);
        Some(ce_v)
    } else {
        None
    };
    let sr = if ctx.rng.random() && !nosr {
        let (sr_v, sr_x, sr_inv) = test.make_in_inv(ctx);
        if mode == Mode::Virtex {
            ti.cfg("SRMUX", if sr_inv { "SR_B" } else { "SR" });
        } else {
            ti.cfg("SRINV", if sr_inv { "SR_B" } else { "SR" });
        }
        if matches!(mode, Mode::Virtex | Mode::Virtex2) {
            ti.cfg("SRFFMUX", "0");
        } else {
            ti.cond_cfg("SRFFMUX", "0", "SLICEM");
        }
        ti.pin_in("SR", &sr_x);
        Some(sr_v)
    } else {
        None
    };
    let rev = if (mode == Mode::Virtex || sr.is_some()) && !norev && ctx.rng.random() {
        let (rev_v, rev_x, rev_inv) = test.make_in_inv(ctx);
        if mode == Mode::Virtex {
            ti.cfg("BYMUX", if rev_inv { "BY_B" } else { "BY" });
        } else {
            ti.cfg("BYINV", if rev_inv { "BY_B" } else { "BY" });
        }
        ti.pin_in("BY", &rev_x);
        ti.cfg("REVUSED", "0");
        if sr.is_none() {
            ti.cfg("SRMUX", "0");
            ti.cfg("SRFFMUX", "0");
            ti.bel("SR0", "DUMMY", "");
        }
        Some(rev_v)
    } else {
        None
    };
    let async_ = latch || ctx.rng.random();
    ti.cfg("SYNC_ATTR", if async_ { "ASYNC" } else { "SYNC" });
    for &(c, d) in ffs {
        let init = ctx.rng.random();
        let mut rval = ctx.rng.random();
        if mode == Mode::Virtex {
            if rev.is_some() ^ sr.is_some() {
                rval = init ^ rev.is_some();
            }
        } else {
            if rev.is_some() {
                rval = false;
            }
        }
        let (prim, rpin, rpin2) = match (latch, async_, rev.is_some() && sr.is_some(), rval) {
            (false, false, false, false) => ("FDRE", "R", ""),
            (false, false, false, true) => ("FDSE", "S", ""),
            (false, false, true, _) => ("FDRSE", "R", "S"),
            (false, true, false, false) => ("FDCE", "CLR", ""),
            (false, true, false, true) => ("FDPE", "PRE", ""),
            (false, true, true, _) => ("FDCPE", "CLR", "PRE"),
            (true, _, false, false) => ("LDCE", "CLR", ""),
            (true, _, false, true) => ("LDPE", "PRE", ""),
            (true, _, true, _) => ("LDCPE", "CLR", "PRE"),
        };
        let mut inst = SrcInst::new(ctx, prim);
        inst.param_bits("INIT", &[if init { BitVal::S1 } else { BitVal::S0 }]);
        let q = test.make_out(ctx);
        let bel = format!("FF{c}");
        ti.pin_out(&format!("{c}Q"), &q);
        ti.bel(&bel, &inst.name, if latch { "#LATCH" } else { "#FF" });
        if let Some((uset, rloc)) = uset {
            inst.attr_str("RLOC", rloc);
            inst.attr_str("U_SET", uset);
        }
        inst.attr_str("BEL", &bel);
        inst.connect("D", d);
        inst.connect(if latch { "G" } else { "C" }, &clk_v);
        if let Some(ref ce) = ce {
            inst.connect(if latch { "GE" } else { "CE" }, ce);
        }
        if mode == Mode::Virtex {
            if let Some(ref sr) = sr {
                if let Some(ref rev) = rev {
                    if !init {
                        inst.connect(rpin, sr);
                        inst.connect(rpin2, rev);
                    } else {
                        inst.connect(rpin2, sr);
                        inst.connect(rpin, rev);
                    }
                } else {
                    inst.connect(rpin, sr);
                }
            } else if let Some(ref rev) = rev {
                inst.connect(rpin, rev);
            }
        } else {
            if let Some(ref sr) = sr {
                inst.connect(rpin, sr);
            }
            if let Some(ref rev) = rev {
                inst.connect(rpin2, rev);
            }
        }
        inst.connect("Q", &q);
        if mode == Mode::Virtex {
            ti.cfg(&format!("INIT{c}"), if init { "HIGH" } else { "LOW" });
        } else {
            ti.cfg(
                &format!("{bel}_INIT_ATTR"),
                if init { "INIT1" } else { "INIT0" },
            );
            ti.cfg(
                &format!("{bel}_SR_ATTR"),
                if rval && sr.is_some() {
                    "SRHIGH"
                } else {
                    "SRLOW"
                },
            );
        }
        test.src_insts.push(inst);
    }
}

fn slice_kind(mode: Mode) -> &'static [&'static str] {
    match mode {
        Mode::Virtex | Mode::Virtex2 => &["SLICE"],
        Mode::Spartan3 | Mode::Virtex4 => &["SLICEL", "SLICEM"],
    }
}

fn gen_tbuf(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8) {
    let o = test.make_out(ctx);
    for _ in 0..sz {
        let mut inst = SrcInst::new(ctx, "BUFT");
        let mut ti = TgtInst::new(&["TBUF"]);

        let (i_v, i_x, i_inv) = test.make_in_inv(ctx);
        let (t_v, t_x, t_inv) = test.make_in_inv(ctx);
        inst.connect("I", &i_v);
        inst.connect("T", &t_v);
        inst.connect("O", &o);

        ti.bel("TRISTATE", &inst.name, "");
        if mode == Mode::Virtex {
            ti.cfg("IMUX", if i_inv { "I_B" } else { "I" });
            ti.cfg("TMUX", if t_inv { "T_B" } else { "T" });
        } else {
            ti.cfg("IINV", if i_inv { "I_B" } else { "I" });
            ti.cfg("TINV", if t_inv { "T_B" } else { "T" });
        }
        ti.pin_in("I", &i_x);
        ti.pin_in("T", &t_x);
        ti.pin_out("O", &o);

        test.src_insts.push(inst);
        test.tgt_insts.push(ti);
    }
}

fn gen_lut(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8) {
    let mut inst = SrcInst::new(ctx, &format!("LUT{sz}"));
    let mut ti = TgtInst::new(slice_kind(mode));

    let (f, x) = *[('F', 'X'), ('G', 'Y')].choose(&mut ctx.rng).unwrap();

    inst.attr_str("BEL", &format!("{f}"));
    let inp = test.make_ins(ctx, sz as usize);
    for i in 0..(sz as usize) {
        inst.connect(&format!("I{i}"), &inp[i]);
        ti.pin_in(&format!("{f}{ii}", ii = i + 1), &inp[i]);
    }
    let init = gen_lut_init(sz, ctx);
    inst.param_bits("INIT", &init);

    if ctx.rng.random() && sz != 1 {
        let out = test.make_wire(ctx);
        inst.connect("O", &out);
        if mode == Mode::Virtex4 {
            ti.cfg(&format!("D{x}MUX"), &format!("{x}"));
        } else {
            ti.cfg(&format!("D{x}MUX"), "1");
        }
        make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[(x, &out)],
            None,
            false,
            false,
            None,
            false,
        );
    } else {
        let out = test.make_out(ctx);
        inst.connect("O", &out);
        ti.cfg(&format!("{x}USED"), "0");
        ti.pin_out(&format!("{x}"), &out);
    }
    if mode != Mode::Virtex4 {
        ti.cfg(&format!("{f}{x}MUX"), &format!("{f}"));
    }
    ti.bel_lut(&format!("{f}"), &inst.name, 4, compile_lut(sz, 4, &init));

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_muxf5(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "MUXF5");
    let mut ti = TgtInst::new(slice_kind(mode));

    let i0 = test.make_wire(ctx);
    let i1 = test.make_wire(ctx);
    inst.connect("I0", &i0);
    inst.connect("I1", &i1);
    make_lut4(test, ctx, &mut ti, 'F', &i1);
    make_lut4(test, ctx, &mut ti, 'G', &i0);
    let (s_v, s_x, s_inv) = test.make_in_inv(ctx);
    inst.connect("S", &s_v);
    ti.pin_in("BX", &s_x);
    if mode == Mode::Virtex {
        ti.cfg("BXMUX", if s_inv { "BX_B" } else { "BX" });
    } else {
        ti.cfg("BXINV", if s_inv { "BX_B" } else { "BX" });
    }

    ti.bel("F5MUX", &inst.name, "");
    ti.cfg("FXMUX", "F5");
    if ctx.rng.random() {
        let o = test.make_wire(ctx);
        inst.connect("O", &o);
        if mode == Mode::Virtex4 {
            ti.cfg("DXMUX", "XMUX");
        } else {
            ti.cfg("DXMUX", "1");
        }
        make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('X', &o)],
            None,
            false,
            false,
            None,
            false,
        );
    } else {
        let o = test.make_out(ctx);
        inst.connect("O", &o);
        if mode == Mode::Virtex4 {
            ti.pin_out("XMUX", &o);
            ti.cfg("XMUXUSED", "0");
        } else {
            ti.pin_out("X", &o);
            ti.cfg("XUSED", "0");
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_muxf678(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8) {
    let mut tis = Vec::new();
    let mut outs = Vec::new();
    for i in 0..(1 << (sz - 5)) {
        let mut inst = SrcInst::new(ctx, "MUXF5");
        let mut ti = TgtInst::new(slice_kind(mode));
        let i0 = test.make_wire(ctx);
        let i1 = test.make_wire(ctx);
        inst.connect("I0", &i0);
        inst.connect("I1", &i1);
        make_lut4(test, ctx, &mut ti, 'F', &i1);
        make_lut4(test, ctx, &mut ti, 'G', &i0);

        let (s_v, s_x, s_inv) = test.make_in_inv(ctx);
        inst.connect("S", &s_v);
        ti.pin_in("BX", &s_x);
        if mode == Mode::Virtex {
            ti.cfg("BXMUX", if s_inv { "BX_B" } else { "BX" });
        } else {
            ti.cfg("BXINV", if s_inv { "BX_B" } else { "BX" });
        }
        ti.bel("F5MUX", &inst.name, "");
        let o = test.make_wire(ctx);
        inst.connect("O", &o);
        if i % 2 == 0 || mode != Mode::Virtex {
            ti.cfg("F5USED", "0");
            ti.pin_out("F5", &o);
        }
        outs.push(o);

        tis.push(ti);
        test.src_insts.push(inst);
    }

    for msz in 6..(sz + 1) {
        let mut new_outs = Vec::new();
        for i in 0..(1 << (sz - msz)) {
            let mut inst = SrcInst::new(ctx, &format!("MUXF{msz}"));
            let ti = match msz {
                6 => &mut tis[2 * i + 1],
                7 => &mut tis[4 * i + 2],
                8 => &mut tis[8 * i + 4],
                _ => unreachable!(),
            };

            inst.connect("I0", &outs[2 * i]);
            inst.connect("I1", &outs[2 * i + 1]);

            let (s_v, s_x, s_inv) = test.make_in_inv(ctx);
            inst.connect("S", &s_v);
            ti.pin_in("BY", &s_x);
            if mode == Mode::Virtex {
                ti.cfg("BYMUX", if s_inv { "BY_B" } else { "BY" });
            } else {
                ti.cfg("BYINV", if s_inv { "BY_B" } else { "BY" });
            }
            ti.bel("F6MUX", &inst.name, "");
            if mode == Mode::Virtex {
                ti.pin_in("F5IN", &outs[2 * i]);
            } else {
                ti.pin_in("FXINB", &outs[2 * i]);
                ti.pin_in("FXINA", &outs[2 * i + 1]);
            }
            if msz == sz {
                if mode == Mode::Virtex {
                    ti.cfg("GYMUX", "F6");
                } else {
                    ti.cfg("GYMUX", "FX");
                }
                if ctx.rng.random() {
                    let o = test.make_wire(ctx);
                    inst.connect("O", &o);
                    if mode == Mode::Virtex4 {
                        ti.cfg("DYMUX", "YMUX");
                    } else {
                        ti.cfg("DYMUX", "1");
                    }
                    make_ffs(
                        test,
                        ctx,
                        mode,
                        ti,
                        &[('Y', &o)],
                        None,
                        false,
                        true,
                        None,
                        false,
                    );
                } else {
                    let o = test.make_out(ctx);
                    inst.connect("O", &o);
                    if mode == Mode::Virtex4 {
                        ti.pin_out("YMUX", &o);
                        ti.cfg("YMUXUSED", "0");
                    } else {
                        ti.pin_out("Y", &o);
                        ti.cfg("YUSED", "0");
                    }
                }
            } else {
                let o = test.make_wire(ctx);
                inst.connect("O", &o);
                ti.cfg("FXUSED", "0");
                ti.pin_out("FX", &o);
                new_outs.push(o);
            }

            test.src_insts.push(inst);
        }
        outs = new_outs;
    }

    for ti in tis {
        test.tgt_insts.push(ti);
    }
}

fn gen_cy(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut tis = Vec::new();
    let mut ci = test.make_in(ctx);
    let mut prevcymode = None;

    for i in 0..num {
        let mut inst_l = SrcInst::new(ctx, "LUT4");
        let mut inst_m = SrcInst::new(ctx, "MUXCY");
        let mut inst_x = SrcInst::new(ctx, "XORCY");
        let f = if i % 2 == 1 { "G" } else { "F" };
        let x = if f == "F" { "X" } else { "Y" };
        if f == "F" {
            tis.push(TgtInst::new(slice_kind(mode)));
        }
        let ti = tis.last_mut().unwrap();
        let inp = test.make_ins(ctx, 4);
        for i in 0..4 {
            inst_l.connect(&format!("I{i}"), &inp[i]);
        }
        let mut init = gen_lut_init(4, ctx);
        inst_l.param_bits("INIT", &init);
        let li = test.make_wire(ctx);
        inst_l.connect("O", &li);
        inst_m.connect("S", &li);
        inst_x.connect("LI", &li);
        inst_m.connect("CI", &ci);
        inst_x.connect("CI", &ci);
        let co = test.make_out(ctx);
        inst_m.connect("O", &co);
        let xo = test.make_out(ctx);
        inst_x.connect("O", &xo);

        let cymode;
        if f == "F" {
            cymode = ctx.rng.random_range(0..4);
        } else if mode == Mode::Virtex {
            cymode = prevcymode.unwrap();
        } else {
            cymode = ctx.rng.random_range(0..5);
        }
        prevcymode = Some(cymode);
        let mut do_backflip = false;

        match cymode {
            0 => {
                inst_m.connect("DI", "0");
                ti.cfg(&format!("CY0{f}"), "0");
                ti.bel(&format!("GND{f}"), "DUMMY", "");
            }
            1 => {
                inst_m.connect("DI", "1");
                ti.cfg(&format!("CY0{f}"), "1");
                ti.bel(&format!("C{}VDD", i % 2 + 1), "DUMMY", "");
            }
            2 => {
                let mut inst_a = SrcInst::new(ctx, "MULT_AND");
                let o = test.make_wire(ctx);
                inst_a.connect("LO", &o);
                inst_m.connect("DI", &o);
                ti.cfg(&format!("CY0{f}"), "PROD");
                if mode == Mode::Virtex4 {
                    inst_a.connect("I0", &inp[2]);
                    inst_a.connect("I1", &inp[1]);
                    do_backflip = true;
                } else {
                    inst_a.connect("I0", &inp[0]);
                    inst_a.connect("I1", &inp[1]);
                }
                ti.bel(&format!("{f}AND"), &inst_a.name, "");
                test.src_insts.push(inst_a);
            }
            3 => {
                if mode == Mode::Virtex4 {
                    inst_m.connect("DI", &inp[2]);
                    ti.cfg(&format!("CY0{f}"), &format!("{f}3"));
                } else {
                    inst_m.connect("DI", &inp[0]);
                    ti.cfg(&format!("CY0{f}"), &format!("{f}1"));
                }
            }
            4 => {
                // not usable on F for unknown reason.
                let w = test.make_in(ctx);
                inst_m.connect("DI", &w);
                ti.pin_in(&format!("B{x}"), &w);
                ti.cfg(&format!("B{x}INV"), &format!("B{x}"));
                ti.cfg(&format!("CY0{f}"), &format!("B{x}"));
            }
            _ => unreachable!(),
        }

        if do_backflip {
            let mut new_init = Vec::new();
            for j in 0..16 {
                let mut fj = j & 9;
                if (j & 2) != 0 {
                    fj |= 4;
                }
                if (j & 4) != 0 {
                    fj |= 2;
                }
                new_init.push(init[fj]);
            }
            init = new_init;
            for i in 0..4 {
                ti.pin_in(&format!("{f}{ii}", ii = i + 1), &inp[[0, 2, 1, 3][i]]);
            }
        } else {
            for i in 0..4 {
                ti.pin_in(&format!("{f}{ii}", ii = i + 1), &inp[i]);
            }
        }
        ti.bel(&format!("XOR{f}"), &inst_x.name, "");
        ti.bel(&format!("CYMUX{f}"), &inst_m.name, "");
        ti.bel_lut(f, &inst_l.name, 4, compile_lut(4, 4, &init));

        if mode == Mode::Virtex4 {
            ti.pin_out(&format!("{x}MUX"), &xo);
            ti.cfg(&format!("{x}MUXUSED"), "0");
        } else {
            ti.pin_out(x, &xo);
            ti.cfg(&format!("{x}USED"), "0");
            ti.cfg(&format!("CYSEL{f}"), f);
        }
        ti.cfg(&format!("{f}{x}MUX"), &format!("{f}XOR"));

        if f == "G" && (mode != Mode::Virtex2 || i != num - 1) {
            ti.pin_out("COUT", &co);
            ti.cfg("COUTUSED", "0");
        } else {
            ti.pin_out(&format!("{x}B"), &co);
            if mode == Mode::Virtex2 {
                ti.cfg(&format!("{x}BMUX"), "1");
                if f == "G" {
                    ti.bel("YBUSED", "DUMMY", "0");
                }
            } else {
                ti.cfg(&format!("{x}BUSED"), "0");
            }
        }

        if i == 0 {
            ti.pin_in("BX", &ci);
            if mode == Mode::Virtex {
                ti.cfg("BXMUX", "BX");
            } else {
                ti.cfg("BXINV", "BX");
            }
            ti.cfg("CYINIT", "BX");
        } else if f == "F" {
            ti.pin_in("CIN", &ci);
            ti.cfg("CYINIT", "CIN");
        }

        ci = co;
        test.src_insts.push(inst_l);
        test.src_insts.push(inst_m);
        test.src_insts.push(inst_x);
    }

    for ti in tis {
        test.tgt_insts.push(ti);
    }
}

fn gen_orcy(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut oi: Option<String> = None;

    for _ in 0..num {
        let mut inst_l = SrcInst::new(ctx, "LUT4");
        let mut inst_m = SrcInst::new(ctx, "MUXCY");
        let mut ti = TgtInst::new(slice_kind(mode));
        let inp = test.make_ins(ctx, 4);
        for i in 0..4 {
            inst_l.connect(&format!("I{i}"), &inp[i]);
        }
        let init = gen_lut_init(4, ctx);
        inst_l.param_bits("INIT", &init);
        let li = test.make_wire(ctx);
        let ci = test.make_in(ctx);
        inst_l.connect("O", &li);
        inst_m.connect("S", &li);
        inst_m.connect("CI", &ci);
        let co = test.make_wire(ctx);
        inst_m.connect("O", &co);

        inst_m.connect("DI", &inp[0]);
        ti.cfg("CY0F", "F1");

        for i in 0..4 {
            ti.pin_in(&format!("F{ii}", ii = i + 1), &inp[i]);
        }
        ti.bel("CYMUXF", &inst_m.name, "");
        ti.bel_lut("F", &inst_l.name, 4, compile_lut(4, 4, &init));

        ti.cfg("CYSELF", "F");
        ti.cfg("CYSELG", "1");
        ti.bel("VDDG", "DUMMY", "");
        ti.bel("CYMUXG", "DUMMY", "");

        ti.pin_in("BX", &ci);
        ti.cfg("BXINV", "BX");
        ti.cfg("CYINIT", "BX");

        let mut inst_o = SrcInst::new(ctx, "ORCY");
        ti.bel("ORCY", &inst_o.name, "");
        let oo = test.make_out(ctx);
        if let Some(oi) = oi {
            inst_o.connect("I", &oi);
            ti.cfg("SOPEXTSEL", "SOPIN");
            ti.pin_in("SOPIN", &oi);
        } else {
            inst_o.connect("I", "0");
            ti.cfg("SOPEXTSEL", "0");
            ti.bel("SOPEXTSEL_GND", "DUMMY", "");
        }
        inst_o.connect("CI", &co);
        inst_o.connect("O", &oo);
        ti.cfg("YUSED", "0");
        ti.cfg("GYMUX", "SOPEXT");
        ti.pin_out("Y", &oo);

        oi = Some(oo);

        test.src_insts.push(inst_l);
        test.src_insts.push(inst_o);
        test.src_insts.push(inst_m);
        test.tgt_insts.push(ti);
    }
}

fn gen_rom16x1(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "ROM16X1");
    let mut ti = TgtInst::new(slice_kind(mode));

    let (f, x) = *[('F', 'X'), ('G', 'Y')].choose(&mut ctx.rng).unwrap();

    inst.attr_str("BEL", &format!("{f}"));
    let inp = test.make_ins(ctx, 4);
    for i in 0..4 {
        inst.connect(&format!("A{i}"), &inp[i]);
    }
    if ctx.rng.random() {
        let out = test.make_wire(ctx);
        inst.connect("O", &out);
        if mode == Mode::Virtex4 {
            ti.cfg(&format!("D{x}MUX"), &format!("{x}"));
        } else {
            ti.cfg(&format!("D{x}MUX"), "1");
        }
        make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[(x, &out)],
            None,
            false,
            false,
            None,
            false,
        );
    } else {
        let out = test.make_out(ctx);
        inst.connect("O", &out);
        ti.cfg(&format!("{x}USED"), "0");
        ti.pin_out(&format!("{x}"), &out);
    };
    if mode != Mode::Virtex4 {
        ti.cfg(&format!("{f}{x}MUX"), &format!("{f}"));
    }
    let init = ctx.gen_bits(16);
    inst.param_bits("INIT", &init);
    let mut val: u64 = 0;
    for i in 0..16 {
        if init[i] == BitVal::S1 {
            val |= 1 << i;
        }
    }
    ti.bel_rom(&format!("{f}"), &inst.name, 4, val);
    for i in 0..4 {
        ti.pin_in(&format!("{f}{ii}", ii = i + 1), &inp[i]);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_rom32px1(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8) {
    let mut inst = SrcInst::new(ctx, &format!("ROM{}X1", 32 << sz));

    let inp = test.make_ins(ctx, 5 + (sz as usize));
    for i in 0..(5 + (sz as usize)) {
        inst.connect(&format!("A{i}"), &inp[i]);
    }
    let init = ctx.gen_bits(32 << sz);
    inst.param_bits("INIT", &init);

    let mut tis = Vec::new();
    for i in 0..(1 << sz) {
        let (name_f, name_g, name_f5);
        match sz {
            0 => {
                name_g = format!("{}.G", inst.name);
                name_f = format!("{}.F", inst.name);
                name_f5 = format!("{}.F5", inst.name);
            }
            1 => {
                name_g = format!("{}/G.S{}", inst.name, i ^ 1);
                name_f = format!("{}/F.S{}", inst.name, i ^ 1);
                name_f5 = format!("{}/F5.S{}", inst.name, i ^ 1);
            }
            2 | 3 => {
                if i < 4 {
                    name_g = format!("{}/G.S{}", inst.name, i ^ 3);
                    name_f = format!("{}/F.S{}", inst.name, i ^ 3);
                    name_f5 = format!("{}/F5.S{}", inst.name, i ^ 3);
                } else {
                    name_g = format!("{}/BG.S{}", inst.name, i ^ 7);
                    name_f = format!("{}/BF.S{}", inst.name, i ^ 7);
                    name_f5 = format!("{}/BF5.S{}", inst.name, i ^ 7);
                }
            }
            _ => unreachable!(),
        };
        let mut ti = TgtInst::new(slice_kind(mode));
        let mut val_f = 0;
        let mut val_g = 0;
        for j in 0..16 {
            if init[i * 32 + j] == BitVal::S1 {
                val_g |= 1 << j;
            }
            if init[i * 32 + 16 + j] == BitVal::S1 {
                val_f |= 1 << j;
            }
        }
        ti.bel_rom("F", &name_f, 4, val_f);
        ti.bel_rom("G", &name_g, 4, val_g);
        for i in 0..4 {
            ti.pin_in(&format!("F{ii}", ii = i + 1), &inp[i]);
            ti.pin_in(&format!("G{ii}", ii = i + 1), &inp[i]);
        }
        ti.pin_in("BX", &inp[4]);
        if mode == Mode::Virtex {
            ti.cfg("BXMUX", "BX");
        } else {
            ti.cfg("BXINV", "BX");
        }
        ti.bel("F5MUX", &name_f5, "");
        tis.push(ti);
    }
    for i in 0..(1 << sz) {
        if i % 2 == 1 {
            let name_f6;
            match sz {
                1 => {
                    name_f6 = format!("{}/F6.S{}", inst.name, i ^ 1);
                }
                2 | 3 => {
                    if i < 4 {
                        name_f6 = format!("{}/F6.S{}", inst.name, i ^ 3);
                    } else {
                        name_f6 = format!("{}/BF6.S{}", inst.name, i ^ 7);
                    }
                }
                _ => unreachable!(),
            };
            tis[i].pin_in("BY", &inp[5]);
            tis[i].cfg("BYINV", "BY");
            tis[i].bel("F6MUX", &name_f6, "");
            let w0 = test.make_wire(ctx);
            let w1 = test.make_wire(ctx);
            let i0 = i - 1;
            let i1 = i;
            tis[i0].cfg("F5USED", "0");
            tis[i1].cfg("F5USED", "0");
            tis[i0].pin_out("F5", &w0);
            tis[i1].pin_out("F5", &w1);
            tis[i].pin_in("FXINB", &w0);
            tis[i].pin_in("FXINA", &w1);
        }
        if i % 4 == 2 {
            let name_f7 = if i < 4 {
                format!("{}/F7.S{}", inst.name, i ^ 3)
            } else {
                format!("{}/BF7.S{}", inst.name, i ^ 7)
            };
            tis[i].pin_in("BY", &inp[6]);
            tis[i].cfg("BYINV", "BY");
            tis[i].bel("F6MUX", &name_f7, "");
            let w0 = test.make_wire(ctx);
            let w1 = test.make_wire(ctx);
            let i0 = i - 1;
            let i1 = i + 1;
            tis[i0].cfg("FXUSED", "0");
            tis[i1].cfg("FXUSED", "0");
            tis[i0].pin_out("FX", &w0);
            tis[i1].pin_out("FX", &w1);
            tis[i].pin_in("FXINB", &w0);
            tis[i].pin_in("FXINA", &w1);
        }
        if i % 8 == 4 {
            let name_f8 = format!("{}/F8", inst.name);
            tis[i].pin_in("BY", &inp[7]);
            tis[i].cfg("BYINV", "BY");
            tis[i].bel("F6MUX", &name_f8, "");
            let w0 = test.make_wire(ctx);
            let w1 = test.make_wire(ctx);
            let i0 = i - 2;
            let i1 = i + 2;
            tis[i0].cfg("FXUSED", "0");
            tis[i1].cfg("FXUSED", "0");
            tis[i0].pin_out("FX", &w0);
            tis[i1].pin_out("FX", &w1);
            tis[i].pin_in("FXINB", &w0);
            tis[i].pin_in("FXINA", &w1);
        }
    }

    if sz == 0 {
        tis[0].cfg("FXMUX", "F5");
        if ctx.rng.random() {
            let o = test.make_wire(ctx);
            inst.connect("O", &o);
            if mode == Mode::Virtex4 {
                tis[0].cfg("DXMUX", "XMUX");
            } else {
                tis[0].cfg("DXMUX", "1");
            }
            make_ffs(
                test,
                ctx,
                mode,
                &mut tis[0],
                &[('X', &o)],
                None,
                false,
                false,
                None,
                false,
            );
        } else {
            let o = test.make_out(ctx);
            inst.connect("O", &o);
            if mode == Mode::Virtex4 {
                tis[0].pin_out("XMUX", &o);
                tis[0].cfg("XMUXUSED", "0");
            } else {
                tis[0].pin_out("X", &o);
                tis[0].cfg("XUSED", "0");
            }
        }
    } else {
        let i = 1 << (sz - 1);
        tis[i].cfg("GYMUX", "FX");
        if ctx.rng.random() {
            let o = test.make_wire(ctx);
            inst.connect("O", &o);
            if mode == Mode::Virtex4 {
                tis[i].cfg("DYMUX", "YMUX");
            } else {
                tis[i].cfg("DYMUX", "1");
            }
            make_ffs(
                test,
                ctx,
                mode,
                &mut tis[i],
                &[('Y', &o)],
                None,
                false,
                true,
                None,
                false,
            );
        } else {
            let o = test.make_out(ctx);
            inst.connect("O", &o);
            if mode == Mode::Virtex4 {
                tis[i].pin_out("YMUX", &o);
                tis[i].cfg("YMUXUSED", "0");
            } else {
                tis[i].pin_out("Y", &o);
                tis[i].cfg("YUSED", "0");
            }
        }
    }

    test.src_insts.push(inst);
    for ti in tis {
        test.tgt_insts.push(ti);
    }
}

fn gen_ram16s(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut insts = Vec::new();
    for _ in 0..num {
        let inst = SrcInst::new(ctx, "RAM16X1S");
        insts.push(inst);
    }
    let mut ti = TgtInst::new(slice_kind(mode));
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    let uset = ctx.gen_name();
    let rloc = if mode == Mode::Virtex {
        format!("R0C0.S{}", ctx.rng.random_range(0..2))
    } else {
        "X0Y0".to_string()
    };

    let inp = test.make_ins(ctx, 4);
    for i in 0..num {
        let inst = &mut insts[i];
        let f = ['G', 'F'][i];

        inst.attr_str("BEL", &format!("{f}"));
        inst.attr_str("RLOC", &rloc);
        inst.attr_str("U_SET", &uset);

        for j in 0..4 {
            inst.connect(&format!("A{j}"), &inp[j]);
            ti.pin_in(&format!("{f}{ii}", ii = j + 1), &inp[j]);
            if mode == Mode::Virtex2 {
                ti.pin_in(&format!("W{f}{ii}", ii = j + 1), &inp[j]);
            }
        }

        let init = ctx.gen_bits(16);
        inst.param_bits("INIT", &init);
        let mut val: u64 = 0;
        for j in 0..16 {
            if init[j] == BitVal::S1 {
                val |= 1 << j;
            }
        }
        ti.bel_ram(&format!("{f}"), &inst.name, 4, val);

        inst.connect("WE", &ce_v);
        inst.connect("WCLK", &clk_v);
    }

    if ctx.rng.random() {
        let mut outs = Vec::new();
        for _ in 0..num {
            outs.push(test.make_wire(ctx));
        }
        let mut stuff = Vec::new();
        for i in 0..num {
            let inst = &mut insts[i];
            let x = ['Y', 'X'][i];
            if mode == Mode::Virtex4 {
                ti.cfg(&format!("D{x}MUX"), &format!("{x}"));
            } else {
                ti.cfg(&format!("D{x}MUX"), "1");
            }
            inst.connect("O", &outs[i]);
            stuff.push((x, &outs[i][..]));
        }
        make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &stuff,
            Some(&clk_v),
            true,
            true,
            None,
            false,
        );
    } else {
        for i in 0..num {
            let inst = &mut insts[i];
            let x = ['Y', 'X'][i];
            let out = test.make_out(ctx);
            inst.connect("O", &out);
            ti.cfg(&format!("{x}USED"), "0");
            ti.pin_out(&format!("{x}"), &out);
        }
    };
    if mode != Mode::Virtex4 {
        for i in 0..num {
            let f = ['G', 'F'][i];
            let x = ['Y', 'X'][i];
            ti.cfg(&format!("{f}{x}MUX"), &format!("{f}"));
        }
    }

    ti.pin_in("CLK", &clk_x);
    ti.pin_in("SR", &ce_x);
    if mode == Mode::Virtex {
        for i in 0..num {
            let inst = &mut insts[i];
            let x = ['Y', 'X'][i];
            let (d_v, d_x, d_inv) = test.make_in_inv(ctx);
            inst.connect("D", &d_v);
            ti.pin_in(&format!("B{x}"), &d_x);
            ti.cfg(
                &format!("B{x}MUX"),
                &if d_inv {
                    format!("B{x}_B")
                } else {
                    format!("B{x}")
                },
            );
        }
        ti.cfg("CKINV", if clk_inv { "0" } else { "1" });
        ti.cfg("SRMUX", if ce_inv { "SR_B" } else { "SR" });
        ti.cfg("RAMCONFIG", if num == 2 { "16X2" } else { "16X1" });
        ti.bel("DGEN", &format!("{}.D", insts[num - 1].name), "");
    } else {
        for i in 0..num {
            let inst = &mut insts[i];
            let f = ['G', 'F'][i];
            let x = ['Y', 'X'][i];
            let d = test.make_in(ctx);
            inst.connect("D", &d);
            ti.pin_in(&format!("B{x}"), &d);
            ti.cfg(&format!("B{x}INV"), &format!("B{x}"));
            ti.cfg(&format!("DI{f}_MUX"), &format!("B{x}"));
            for j in 1..5 {
                ti.cfg(&format!("W{f}{j}USED"), "0");
            }
        }
        ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
        ti.cfg("SRINV", if ce_inv { "SR_B" } else { "SR" });
    }
    ti.bel("WSGEN", &format!("{}.WE", insts[num - 1].name), "");

    for inst in insts {
        test.src_insts.push(inst);
    }
    test.tgt_insts.push(ti);
}

fn gen_ram32ps(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8) {
    let mut inst = SrcInst::new(ctx, &format!("RAM{}X1S", 32 << sz));
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &clk_v);
    inst.connect("WE", &ce_v);

    let inp = test.make_ins(ctx, 5 + (sz as usize));
    for i in 0..(5 + (sz as usize)) {
        inst.connect(&format!("A{i}"), &inp[i]);
    }
    let init = ctx.gen_bits(32 << sz);
    inst.param_bits("INIT", &init);

    let mut tis = Vec::new();
    for i in 0..(1 << sz) {
        let (name_f, name_g, name_f5);
        match sz {
            0 => {
                if mode == Mode::Virtex {
                    name_g = format!("{}.G", inst.name);
                    name_f = format!("{}.F", inst.name);
                    name_f5 = format!("{}.F5", inst.name);
                } else {
                    name_g = format!("{}/G", inst.name);
                    name_f = format!("{}/F", inst.name);
                    name_f5 = format!("{}/F5", inst.name);
                }
            }
            1 => {
                name_g = format!("{}/G.S{}", inst.name, i ^ 1);
                name_f = format!("{}/F.S{}", inst.name, i ^ 1);
                name_f5 = format!("{}/F5.S{}", inst.name, i ^ 1);
            }
            2 => {
                name_g = format!("{}/G.S{}", inst.name, i ^ 3);
                name_f = format!("{}/F.S{}", inst.name, i ^ 3);
                name_f5 = format!("{}/F5.S{}", inst.name, i ^ 3);
            }
            _ => unreachable!(),
        };
        let mut ti = TgtInst::new(slice_kind(mode));
        let mut val_f = 0;
        let mut val_g = 0;
        for j in 0..16 {
            if init[i * 32 + j] == BitVal::S1 {
                val_g |= 1 << j;
            }
            if init[i * 32 + 16 + j] == BitVal::S1 {
                val_f |= 1 << j;
            }
        }
        ti.bel_ram("F", &name_f, 4, val_f);
        ti.bel_ram("G", &name_g, 4, val_g);
        for i in 0..4 {
            ti.pin_in(&format!("F{ii}", ii = i + 1), &inp[i]);
            ti.pin_in(&format!("G{ii}", ii = i + 1), &inp[i]);
            if mode == Mode::Virtex2 {
                ti.pin_in(&format!("WF{ii}", ii = i + 1), &inp[i]);
                ti.pin_in(&format!("WG{ii}", ii = i + 1), &inp[i]);
            }
        }
        ti.pin_in("BX", &inp[4]);
        if mode == Mode::Virtex {
            ti.cfg("BXMUX", "BX");
        } else {
            ti.cfg("BXINV", "BX");
        }
        ti.bel("F5MUX", &name_f5, "");
        tis.push(ti);
    }
    for i in 0..(1 << sz) {
        if i % 2 == 1 {
            let name_f6 = match sz {
                1 => {
                    format!("{}/F6.S{}", inst.name, i ^ 1)
                }
                2 => {
                    format!("{}/F6.S{}", inst.name, i ^ 3)
                }
                _ => unreachable!(),
            };
            tis[i].pin_in("BY", &inp[5]);
            tis[i].cfg("BYINV", "BY");
            tis[i].bel("F6MUX", &name_f6, "");
            let w0 = test.make_wire(ctx);
            let w1 = test.make_wire(ctx);
            let i0 = i - 1;
            let i1 = i;
            tis[i0].cfg("F5USED", "0");
            tis[i1].cfg("F5USED", "0");
            tis[i0].pin_out("F5", &w0);
            tis[i1].pin_out("F5", &w1);
            tis[i].pin_in("FXINB", &w0);
            tis[i].pin_in("FXINA", &w1);
        }
        if i % 4 == 2 {
            let name_f7 = format!("{}/F7.S{}", inst.name, i ^ 3);
            tis[i].pin_in("BY", &inp[6]);
            tis[i].cfg("BYINV", "BY");
            tis[i].bel("F6MUX", &name_f7, "");
            let w0 = test.make_wire(ctx);
            let w1 = test.make_wire(ctx);
            let i0 = i - 1;
            let i1 = i + 1;
            tis[i0].cfg("FXUSED", "0");
            tis[i1].cfg("FXUSED", "0");
            tis[i0].pin_out("FX", &w0);
            tis[i1].pin_out("FX", &w1);
            tis[i].pin_in("FXINB", &w0);
            tis[i].pin_in("FXINA", &w1);
        }
    }

    if sz == 0 {
        tis[0].cfg("FXMUX", "F5");
        if ctx.rng.random() {
            let o = test.make_wire(ctx);
            inst.connect("O", &o);
            if mode == Mode::Virtex4 {
                tis[0].cfg("DXMUX", "XMUX");
            } else {
                tis[0].cfg("DXMUX", "1");
            }
            make_ffs(
                test,
                ctx,
                mode,
                &mut tis[0],
                &[('X', &o)],
                Some(&clk_v),
                true,
                true,
                None,
                false,
            );
        } else {
            let o = test.make_out(ctx);
            inst.connect("O", &o);
            if mode == Mode::Virtex4 {
                tis[0].pin_out("XMUX", &o);
                tis[0].cfg("XMUXUSED", "0");
            } else {
                tis[0].pin_out("X", &o);
                tis[0].cfg("XUSED", "0");
            }
        }
    } else {
        let i = 1 << (sz - 1);
        tis[i].cfg("GYMUX", "FX");
        if ctx.rng.random() {
            let o = test.make_wire(ctx);
            inst.connect("O", &o);
            if mode == Mode::Virtex4 {
                tis[i].cfg("DYMUX", "YMUX");
            } else {
                tis[i].cfg("DYMUX", "1");
            }
            make_ffs(
                test,
                ctx,
                mode,
                &mut tis[i],
                &[('Y', &o)],
                Some(&clk_v),
                true,
                true,
                None,
                false,
            );
        } else {
            let o = test.make_out(ctx);
            inst.connect("O", &o);
            if mode == Mode::Virtex4 {
                tis[i].pin_out("YMUX", &o);
                tis[i].cfg("YMUXUSED", "0");
            } else {
                tis[i].pin_out("Y", &o);
                tis[i].cfg("YUSED", "0");
            }
        }
    }

    if mode == Mode::Virtex {
        let (d_v, d_x, d_inv) = test.make_in_inv(ctx);
        inst.connect("D", &d_v);
        for ti in &mut tis {
            ti.pin_in("BY", &d_x);
            ti.cfg("BYMUX", if d_inv { "BY_B" } else { "BY" });
            ti.cfg("CKINV", if clk_inv { "0" } else { "1" });
            ti.cfg("SRMUX", if ce_inv { "SR_B" } else { "SR" });
            ti.cfg("RAMCONFIG", "32X1");
            ti.bel("DGEN", &format!("{}.F.D", inst.name), "");
        }
    } else {
        let d = test.make_in(ctx);
        inst.connect("D", &d);
        for i in 0..(1 << sz) {
            let ti = &mut tis[i];
            ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
            ti.cfg("SRINV", if ce_inv { "SR_B" } else { "SR" });
            ti.cfg("SLICEWE0USED", "0");
            if sz >= 1 {
                ti.cfg("SLICEWE1USED", "0");
            }
            if sz >= 2 {
                ti.cfg("SLICEWE2USED", "0");
            }
            ti.cfg("DIF_MUX", "ALTDIF");
            if i == 0 {
                ti.pin_in("BY", &d);
                ti.cfg("DIG_MUX", "BY");
                ti.cfg("BYINV", "BY");
            } else {
                ti.cfg("DIG_MUX", "ALTDIG");
            }
            for f in ['F', 'G'] {
                for j in 1..5 {
                    ti.cfg(&format!("W{f}{j}USED"), "0");
                }
            }
        }
        if sz == 0 {
            if mode == Mode::Virtex2 {
                let tmp = test.make_wire(ctx);
                tis[0].bel("BXOUTUSED", "DUMMY", "0");
                tis[0].pin_out("BXOUT", &tmp);
                tis[0].pin_in("SLICEWE0", &tmp);
            }
        } else if sz == 1 {
            let dig = test.make_wire(ctx);
            tis[0].pin_out("DIG", &dig);
            tis[0].bel("DIGUSED", "DUMMY", "0");
            tis[1].pin_in("ALTDIG", &dig);

            if mode == Mode::Virtex2 {
                let t0 = test.make_wire(ctx);
                tis[0].bel("BXOUTUSED", "DUMMY", "0");
                tis[0].pin_out("BXOUT", &t0);
                tis[0].pin_in("SLICEWE0", &t0);
                let t1 = test.make_wire(ctx);
                tis[1].bel("BXOUTUSED", "DUMMY", "0");
                tis[1].pin_out("BXOUT", &t1);
                tis[1].pin_in("SLICEWE0", &t1);
            }

            let byout = test.make_wire(ctx);
            let byinvout = test.make_wire(ctx);
            tis[1].bel("BYOUTUSED", "DUMMY", "0");
            tis[1].bel("BYINVOUTUSED", "DUMMY", "0");
            tis[1].pin_out("BYOUT", &byout);
            tis[1].pin_out("BYINVOUT", &byinvout);
            tis[0].pin_in("SLICEWE1", &byinvout);
            tis[1].pin_in("SLICEWE1", &byout);
        } else if sz == 2 {
            let dig0 = test.make_wire(ctx);
            tis[0].pin_out("DIG", &dig0);
            tis[0].bel("DIGUSED", "DUMMY", "0");
            tis[1].pin_in("ALTDIG", &dig0);
            tis[2].pin_in("ALTDIG", &dig0);
            let dig2 = test.make_wire(ctx);
            tis[2].pin_out("DIG", &dig2);
            tis[2].bel("DIGUSED", "DUMMY", "0");
            tis[3].pin_in("ALTDIG", &dig2);

            let t0 = test.make_wire(ctx);
            tis[2].bel("BXOUTUSED", "DUMMY", "0");
            tis[2].pin_out("BXOUT", &t0);
            tis[0].pin_in("SLICEWE0", &t0);
            tis[2].pin_in("SLICEWE0", &t0);
            let t1 = test.make_wire(ctx);
            tis[3].bel("BXOUTUSED", "DUMMY", "0");
            tis[3].pin_out("BXOUT", &t1);
            tis[1].pin_in("SLICEWE0", &t1);
            tis[3].pin_in("SLICEWE0", &t1);

            let byout1 = test.make_wire(ctx);
            let byinvout1 = test.make_wire(ctx);
            tis[3].bel("BYOUTUSED", "DUMMY", "0");
            tis[3].bel("BYINVOUTUSED", "DUMMY", "0");
            tis[3].pin_out("BYOUT", &byout1);
            tis[3].pin_out("BYINVOUT", &byinvout1);
            tis[0].pin_in("SLICEWE1", &byinvout1);
            tis[1].pin_in("SLICEWE1", &byout1);
            tis[2].pin_in("SLICEWE1", &byinvout1);
            tis[3].pin_in("SLICEWE1", &byout1);

            let byout2 = test.make_wire(ctx);
            let byinvout2 = test.make_wire(ctx);
            tis[2].bel("BYOUTUSED", "DUMMY", "0");
            tis[2].bel("BYINVOUTUSED", "DUMMY", "0");
            tis[2].pin_out("BYOUT", &byout2);
            tis[2].pin_out("BYINVOUT", &byinvout2);
            tis[0].pin_in("SLICEWE2", &byinvout2);
            tis[1].pin_in("SLICEWE2", &byinvout2);
            tis[2].pin_in("SLICEWE2", &byout2);
            tis[3].pin_in("SLICEWE2", &byout2);
        }
    }
    for i in 0..(1 << sz) {
        let ti = &mut tis[i];
        ti.pin_in("CLK", &clk_x);
        ti.pin_in("SR", &ce_x);
        if mode == Mode::Virtex {
            ti.bel("WSGEN", &format!("{}.F.WE", inst.name), "");
        } else {
            match sz {
                0 => ti.bel("WSGEN", &format!("{}/F.WE", inst.name), ""),
                1 => ti.bel("WSGEN", &format!("{}/F.S{}.WE", inst.name, i ^ 1), ""),
                2 => ti.bel("WSGEN", &format!("{}/F.S{}.WE", inst.name, i ^ 3), ""),
                _ => unreachable!(),
            }
        }
    }

    test.src_insts.push(inst);
    for ti in tis {
        test.tgt_insts.push(ti);
    }
}

fn gen_ram16d(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "RAM16X1D");
    let (sf, sg);
    if mode == Mode::Virtex {
        sf = format!("{}.SLICE_F", inst.name);
        sg = format!("{}.SLICE_G", inst.name);
    } else {
        sf = format!("{}.SLICEM_F", inst.name);
        sg = format!("{}.SLICEM_G", inst.name);
    }
    let mut ti = TgtInst::new(slice_kind(mode));
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &clk_v);
    inst.connect("WE", &ce_v);

    let a = test.make_ins(ctx, 4);
    let dpra = test.make_ins(ctx, 4);
    for i in 0..4 {
        inst.connect(&format!("A{i}"), &a[i]);
        inst.connect(&format!("DPRA{i}"), &dpra[i]);
        ti.pin_in(&format!("F{ii}", ii = i + 1), &dpra[i]);
        ti.pin_in(&format!("G{ii}", ii = i + 1), &a[i]);
    }
    let init = ctx.gen_bits(16);
    inst.param_bits("INIT", &init);
    let mut val: u64 = 0;
    for j in 0..16 {
        if init[j] == BitVal::S1 {
            val |= 1 << j;
        }
    }
    ti.bel_ram("F", &sf, 4, val);
    ti.bel_ram("G", &sg, 4, val);

    if ctx.rng.random() {
        let spo = test.make_wire(ctx);
        let dpo = test.make_wire(ctx);
        inst.connect("SPO", &spo);
        inst.connect("DPO", &dpo);
        if mode == Mode::Virtex4 {
            ti.cfg("DXMUX", "X");
            ti.cfg("DYMUX", "Y");
        } else {
            ti.cfg("DXMUX", "1");
            ti.cfg("DYMUX", "1");
        }
        make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('X', &dpo), ('Y', &spo)],
            Some(&clk_v),
            true,
            true,
            None,
            false,
        );
    } else {
        let spo = test.make_out(ctx);
        let dpo = test.make_out(ctx);
        inst.connect("SPO", &spo);
        inst.connect("DPO", &dpo);
        ti.pin_out("X", &dpo);
        ti.pin_out("Y", &spo);
        ti.cfg("XUSED", "0");
        ti.cfg("YUSED", "0");
    }
    if mode != Mode::Virtex4 {
        ti.cfg("FXMUX", "F");
        ti.cfg("GYMUX", "G");
    }

    ti.pin_in("CLK", &clk_x);
    ti.pin_in("SR", &ce_x);
    if mode == Mode::Virtex {
        let (d_v, d_x, d_inv) = test.make_in_inv(ctx);
        inst.connect("D", &d_v);
        ti.pin_in("BY", &d_x);
        ti.cfg("BYMUX", if d_inv { "BY_B" } else { "BY" });
        ti.cfg("CKINV", if clk_inv { "0" } else { "1" });
        ti.cfg("SRMUX", if ce_inv { "SR_B" } else { "SR" });
        ti.cfg("RAMCONFIG", "16X1DP");
        ti.bel("DGEN", &format!("{}.D", inst.name), "");
    } else {
        let d = test.make_in(ctx);
        inst.connect("D", &d);
        ti.pin_in("BY", &d);
        ti.cfg("BYINV", "BY");
        ti.cfg("DIG_MUX", "BY");
        ti.cfg("DIF_MUX", "ALTDIF");
        ti.cfg("F_ATTR", "DUAL_PORT");
        ti.cfg("G_ATTR", "DUAL_PORT");
        ti.cfg("WF1USED", "0");
        ti.cfg("WF2USED", "0");
        ti.cfg("WF3USED", "0");
        ti.cfg("WF4USED", "0");
        ti.cfg("WG1USED", "0");
        ti.cfg("WG2USED", "0");
        ti.cfg("WG3USED", "0");
        ti.cfg("WG4USED", "0");
        ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
        ti.cfg("SRINV", if ce_inv { "SR_B" } else { "SR" });
    }
    ti.bel("WSGEN", &format!("{}.WE", inst.name), "");

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram16d_v2(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut insts = Vec::new();
    for _ in 0..num {
        let inst = SrcInst::new(ctx, "RAM16X1D");
        insts.push(inst);
    }
    let mut tis = TgtInst::new(slice_kind(mode));
    let mut tid = TgtInst::new(slice_kind(mode));
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    let uset = ctx.gen_name();
    let rloc = "X0Y0";

    let inps = test.make_ins(ctx, 4);
    let inpd = test.make_ins(ctx, 4);
    for i in 0..num {
        let inst = &mut insts[i];
        let f = ['G', 'F'][i];

        inst.attr_str("BEL", &format!("{f}"));
        inst.attr_str("RLOC", rloc);
        inst.attr_str("U_SET", &uset);

        for j in 0..4 {
            inst.connect(&format!("A{j}"), &inps[j]);
            inst.connect(&format!("DPRA{j}"), &inpd[j]);
            tis.pin_in(&format!("{f}{ii}", ii = j + 1), &inps[j]);
            tid.pin_in(&format!("{f}{ii}", ii = j + 1), &inpd[j]);
            tis.pin_in(&format!("W{f}{ii}", ii = j + 1), &inps[j]);
            tid.pin_in(&format!("W{f}{ii}", ii = j + 1), &inps[j]);
        }

        let init = ctx.gen_bits(16);
        inst.param_bits("INIT", &init);
        let mut val: u64 = 0;
        for j in 0..16 {
            if init[j] == BitVal::S1 {
                val |= 1 << j;
            }
        }
        tis.bel_ram(&format!("{f}"), &format!("{}/SP", inst.name), 4, val);
        tid.bel_ram(&format!("{f}"), &format!("{}/DP", inst.name), 4, val);

        inst.connect("WE", &ce_v);
        inst.connect("WCLK", &clk_v);
    }

    if ctx.rng.random() {
        let mut spos = Vec::new();
        let mut dpos = Vec::new();
        for _ in 0..num {
            spos.push(test.make_wire(ctx));
            dpos.push(test.make_wire(ctx));
        }
        let mut stuffs = Vec::new();
        let mut stuffd = Vec::new();
        for i in 0..num {
            let inst = &mut insts[i];
            let x = ['Y', 'X'][i];
            tis.cfg(&format!("D{x}MUX"), "1");
            tid.cfg(&format!("D{x}MUX"), "1");
            inst.connect("SPO", &spos[i]);
            inst.connect("DPO", &dpos[i]);
            stuffs.push((x, &spos[i][..]));
            stuffd.push((x, &dpos[i][..]));
        }
        make_ffs(
            test,
            ctx,
            mode,
            &mut tis,
            &stuffs,
            Some(&clk_v),
            true,
            true,
            None,
            false,
        );
        make_ffs(
            test,
            ctx,
            mode,
            &mut tid,
            &stuffd,
            Some(&clk_v),
            true,
            true,
            None,
            false,
        );
    } else {
        for i in 0..num {
            let inst = &mut insts[i];
            let x = ['Y', 'X'][i];
            let spo = test.make_out(ctx);
            let dpo = test.make_out(ctx);
            inst.connect("SPO", &spo);
            inst.connect("DPO", &dpo);
            tis.cfg(&format!("{x}USED"), "0");
            tid.cfg(&format!("{x}USED"), "0");
            tis.pin_out(&format!("{x}"), &spo);
            tid.pin_out(&format!("{x}"), &dpo);
        }
    };
    for i in 0..num {
        let f = ['G', 'F'][i];
        let x = ['Y', 'X'][i];
        tis.cfg(&format!("{f}{x}MUX"), &format!("{f}"));
        tid.cfg(&format!("{f}{x}MUX"), &format!("{f}"));
    }

    for ti in [&mut tis, &mut tid] {
        ti.pin_in("CLK", &clk_x);
        ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
        ti.cfg("SRINV", if ce_inv { "SR_B" } else { "SR" });
        ti.pin_in("SR", &ce_x);
    }
    for i in 0..num {
        let inst = &mut insts[i];
        let f = ['G', 'F'][i];
        let x = ['Y', 'X'][i];
        let d = test.make_in(ctx);
        inst.connect("D", &d);
        tis.pin_in(&format!("B{x}"), &d);
        tid.pin_in(&format!("B{x}"), &d);
        tis.cfg(&format!("{f}_ATTR"), "DUAL_PORT");
        tid.cfg(&format!("{f}_ATTR"), "DUAL_PORT");
        tis.cfg(&format!("B{x}INV"), &format!("B{x}"));
        tid.cfg(&format!("B{x}INV"), &format!("B{x}"));
        tis.cfg(&format!("DI{f}_MUX"), &format!("B{x}"));
        tid.cfg(&format!("DI{f}_MUX"), &format!("B{x}"));
        for j in 1..5 {
            tis.cfg(&format!("W{f}{j}USED"), "0");
            tid.cfg(&format!("W{f}{j}USED"), "0");
        }
    }
    tis.bel("WSGEN", &format!("{}/SP.WE", insts[num - 1].name), "");
    tid.bel("WSGEN", &format!("{}/DP.WE", insts[num - 1].name), "");

    for inst in insts {
        test.src_insts.push(inst);
    }
    test.tgt_insts.push(tis);
    test.tgt_insts.push(tid);
}

fn gen_srl(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut insts = Vec::new();
    for _ in 0..num {
        let inst = SrcInst::new(ctx, "SRL16E");
        insts.push(inst);
    }
    let mut ti = TgtInst::new(slice_kind(mode));
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    let uset = ctx.gen_name();
    let rloc = if mode == Mode::Virtex {
        format!("R0C0.S{}", ctx.rng.random_range(0..2))
    } else {
        "X0Y0".to_string()
    };

    for i in 0..num {
        let inst = &mut insts[i];
        let f = ['G', 'F'][i];

        inst.attr_str("BEL", &format!("{f}"));
        inst.attr_str("RLOC", &rloc);
        inst.attr_str("U_SET", &uset);

        let inp = test.make_ins(ctx, 4);
        for j in 0..4 {
            inst.connect(&format!("A{j}"), &inp[j]);
            ti.pin_in(&format!("{f}{ii}", ii = j + 1), &inp[j]);
        }

        let init = ctx.gen_bits(16);
        inst.param_bits("INIT", &init);
        let mut val: u64 = 0;
        for j in 0..16 {
            if init[j] == BitVal::S1 {
                val |= 1 << j;
            }
        }
        ti.bel_ram(&format!("{f}"), &inst.name, 4, val);

        inst.connect("CE", &ce_v);
        inst.connect("CLK", &clk_v);
    }

    if ctx.rng.random() {
        let mut outs = Vec::new();
        for _ in 0..num {
            outs.push(test.make_wire(ctx));
        }
        let mut stuff = Vec::new();
        for i in 0..num {
            let inst = &mut insts[i];
            let x = ['Y', 'X'][i];
            if mode == Mode::Virtex4 {
                ti.cfg(&format!("D{x}MUX"), &format!("{x}"));
            } else {
                ti.cfg(&format!("D{x}MUX"), "1");
            }
            inst.connect("Q", &outs[i]);
            stuff.push((x, &outs[i][..]));
        }
        make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &stuff,
            Some(&clk_v),
            true,
            true,
            None,
            false,
        );
    } else {
        for i in 0..num {
            let inst = &mut insts[i];
            let x = ['Y', 'X'][i];
            let out = test.make_out(ctx);
            inst.connect("Q", &out);
            ti.cfg(&format!("{x}USED"), "0");
            ti.pin_out(&format!("{x}"), &out);
        }
    };
    if mode != Mode::Virtex4 {
        for i in 0..num {
            let f = ['G', 'F'][i];
            let x = ['Y', 'X'][i];
            ti.cfg(&format!("{f}{x}MUX"), &format!("{f}"));
        }
    }

    ti.pin_in("CLK", &clk_x);
    ti.pin_in("SR", &ce_x);
    if mode == Mode::Virtex {
        for i in 0..num {
            let inst = &mut insts[i];
            let x = ['Y', 'X'][i];
            let (d_v, d_x, d_inv) = test.make_in_inv(ctx);
            inst.connect("D", &d_v);
            ti.pin_in(&format!("B{x}"), &d_x);
            ti.cfg(
                &format!("B{x}MUX"),
                &if d_inv {
                    format!("B{x}_B")
                } else {
                    format!("B{x}")
                },
            );
        }
        ti.cfg("CKINV", if clk_inv { "0" } else { "1" });
        ti.cfg("SRMUX", if ce_inv { "SR_B" } else { "SR" });
        ti.cfg("RAMCONFIG", if num == 2 { "2SHIFTS" } else { "1SHIFT" });
        ti.bel("DGEN", &format!("{}.D", insts[num - 1].name), "");
    } else {
        for i in 0..num {
            let inst = &mut insts[i];
            let f = ['G', 'F'][i];
            let x = ['Y', 'X'][i];
            let d = test.make_in(ctx);
            inst.connect("D", &d);
            ti.pin_in(&format!("B{x}"), &d);
            ti.cfg(&format!("B{x}INV"), &format!("B{x}"));
            ti.cfg(&format!("DI{f}_MUX"), &format!("B{x}"));
            ti.cfg(&format!("{f}_ATTR"), "SHIFT_REG");
        }
        ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
        ti.cfg("SRINV", if ce_inv { "SR_B" } else { "SR" });
    }
    ti.bel("WSGEN", &format!("{}.CE", insts[num - 1].name), "");

    for inst in insts {
        test.src_insts.push(inst);
    }
    test.tgt_insts.push(ti);
}

fn gen_ram32pd_v2(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8) {
    let mut inst = SrcInst::new(ctx, &format!("RAM{}X1D", 32 << sz));
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &clk_v);
    inst.connect("WE", &ce_v);

    let inps = test.make_ins(ctx, 5 + (sz as usize));
    let inpd = test.make_ins(ctx, 5 + (sz as usize));
    for i in 0..(5 + (sz as usize)) {
        inst.connect(&format!("A{i}"), &inps[i]);
        inst.connect(&format!("DPRA{i}"), &inpd[i]);
    }
    let init = ctx.gen_bits(32 << sz);
    inst.param_bits("INIT", &init);

    let mut tiss = Vec::new();
    let mut tids = Vec::new();
    for (sd, sdi, tis) in [('S', 0, &mut tiss), ('D', 2, &mut tids)] {
        for i in 0..(1 << sz) {
            let (name_f, name_g, name_f5);
            match sz {
                0 => {
                    name_g = format!("{}/{}P.G", inst.name, sd);
                    name_f = format!("{}/{}P.F", inst.name, sd);
                    name_f5 = format!("{}/F5.{}", inst.name, sd);
                }
                1 => {
                    name_g = format!("{}/G.S{}", inst.name, (sdi + i) ^ 1);
                    name_f = format!("{}/F.S{}", inst.name, (sdi + i) ^ 1);
                    name_f5 = format!("{}/F5.S{}", inst.name, (sdi + i) ^ 1);
                }
                _ => unreachable!(),
            };
            let mut ti = TgtInst::new(slice_kind(mode));
            let mut val_f = 0;
            let mut val_g = 0;
            for j in 0..16 {
                if init[i * 32 + j] == BitVal::S1 {
                    val_g |= 1 << j;
                }
                if init[i * 32 + 16 + j] == BitVal::S1 {
                    val_f |= 1 << j;
                }
            }
            ti.bel_ram("F", &name_f, 4, val_f);
            ti.bel_ram("G", &name_g, 4, val_g);
            ti.cfg("F_ATTR", "DUAL_PORT");
            ti.cfg("G_ATTR", "DUAL_PORT");
            for i in 0..4 {
                if sd == 'S' {
                    ti.pin_in(&format!("F{ii}", ii = i + 1), &inps[i]);
                    ti.pin_in(&format!("G{ii}", ii = i + 1), &inps[i]);
                } else {
                    ti.pin_in(&format!("F{ii}", ii = i + 1), &inpd[i]);
                    ti.pin_in(&format!("G{ii}", ii = i + 1), &inpd[i]);
                }
                ti.pin_in(&format!("WF{ii}", ii = i + 1), &inps[i]);
                ti.pin_in(&format!("WG{ii}", ii = i + 1), &inps[i]);
            }
            if sd == 'S' {
                ti.pin_in("BX", &inps[4]);
            } else {
                ti.pin_in("BX", &inpd[4]);
            }
            ti.cfg("BXINV", "BX");
            ti.bel("F5MUX", &name_f5, "");
            tis.push(ti);
        }
        if sz == 1 {
            let name_f6 = format!("{}/F6.S{}", inst.name, sdi);
            if sd == 'S' {
                tis[1].pin_in("BY", &inps[5]);
            } else {
                tis[1].pin_in("BY", &inpd[5]);
            }
            tis[1].cfg("BYINV", "BY");
            tis[1].bel("F6MUX", &name_f6, "");
            let w0 = test.make_wire(ctx);
            let w1 = test.make_wire(ctx);
            tis[0].cfg("F5USED", "0");
            tis[1].cfg("F5USED", "0");
            tis[0].pin_out("F5", &w0);
            tis[1].pin_out("F5", &w1);
            tis[1].pin_in("FXINB", &w0);
            tis[1].pin_in("FXINA", &w1);
        }

        if sz == 0 {
            tis[0].cfg("FXMUX", "F5");
            if ctx.rng.random() {
                let o = test.make_wire(ctx);
                inst.connect(&format!("{sd}PO"), &o);
                if mode == Mode::Virtex4 {
                    tis[0].cfg("DXMUX", "XMUX");
                } else {
                    tis[0].cfg("DXMUX", "1");
                }
                make_ffs(
                    test,
                    ctx,
                    mode,
                    &mut tis[0],
                    &[('X', &o)],
                    Some(&clk_v),
                    true,
                    true,
                    None,
                    false,
                );
            } else {
                let o = test.make_out(ctx);
                inst.connect(&format!("{sd}PO"), &o);
                if mode == Mode::Virtex4 {
                    tis[0].pin_out("XMUX", &o);
                    tis[0].cfg("XMUXUSED", "0");
                } else {
                    tis[0].pin_out("X", &o);
                    tis[0].cfg("XUSED", "0");
                }
            }
        } else {
            tis[1].cfg("GYMUX", "FX");
            if ctx.rng.random() {
                let o = test.make_wire(ctx);
                inst.connect(&format!("{sd}PO"), &o);
                if mode == Mode::Virtex4 {
                    tis[1].cfg("DYMUX", "YMUX");
                } else {
                    tis[1].cfg("DYMUX", "1");
                }
                make_ffs(
                    test,
                    ctx,
                    mode,
                    &mut tis[1],
                    &[('Y', &o)],
                    Some(&clk_v),
                    true,
                    true,
                    None,
                    false,
                );
            } else {
                let o = test.make_out(ctx);
                inst.connect(&format!("{sd}PO"), &o);
                if mode == Mode::Virtex4 {
                    tis[1].pin_out("YMUX", &o);
                    tis[1].cfg("YMUXUSED", "0");
                } else {
                    tis[1].pin_out("Y", &o);
                    tis[1].cfg("YUSED", "0");
                }
            }
        }
    }

    let d = test.make_in(ctx);
    inst.connect("D", &d);
    for (sd, sdi, tis) in [('S', 0, &mut tiss), ('D', 2, &mut tids)] {
        for i in 0..(1 << sz) {
            let ti = &mut tis[i];
            ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
            ti.cfg("SRINV", if ce_inv { "SR_B" } else { "SR" });
            ti.cfg("SLICEWE0USED", "0");
            if sz >= 1 {
                ti.cfg("SLICEWE1USED", "0");
            }
            if sz >= 2 {
                ti.cfg("SLICEWE2USED", "0");
            }
            ti.cfg("DIF_MUX", "ALTDIF");
            if i == 0 {
                ti.pin_in("BY", &d);
                ti.cfg("DIG_MUX", "BY");
                ti.cfg("BYINV", "BY");
            } else {
                ti.cfg("DIG_MUX", "ALTDIG");
            }
            for f in ['F', 'G'] {
                for j in 1..5 {
                    ti.cfg(&format!("W{f}{j}USED"), "0");
                }
            }
        }
        if sz == 1 {
            let dig = test.make_wire(ctx);
            tis[0].pin_out("DIG", &dig);
            tis[0].bel("DIGUSED", "DUMMY", "0");
            tis[1].pin_in("ALTDIG", &dig);
        }
        for i in 0..(1 << sz) {
            let ti = &mut tis[i];
            ti.pin_in("CLK", &clk_x);
            ti.pin_in("SR", &ce_x);
            match sz {
                0 => ti.bel("WSGEN", &format!("{}/{}P.F.WE", inst.name, sd), ""),
                1 => ti.bel(
                    "WSGEN",
                    &format!("{}/F.S{}.WE", inst.name, (sdi + i) ^ 1),
                    "",
                ),
                _ => unreachable!(),
            }
        }
    }

    for i in 0..(1 << sz) {
        let tmp = test.make_wire(ctx);
        tiss[i].bel("BXOUTUSED", "DUMMY", "0");
        tiss[i].pin_out("BXOUT", &tmp);
        tiss[i].pin_in("SLICEWE0", &tmp);
        tids[i].pin_in("SLICEWE0", &tmp);
    }
    if sz == 1 {
        let byout = test.make_wire(ctx);
        let byinvout = test.make_wire(ctx);
        tiss[1].bel("BYOUTUSED", "DUMMY", "0");
        tiss[1].bel("BYINVOUTUSED", "DUMMY", "0");
        tiss[1].pin_out("BYOUT", &byout);
        tiss[1].pin_out("BYINVOUT", &byinvout);
        tiss[0].pin_in("SLICEWE1", &byinvout);
        tiss[1].pin_in("SLICEWE1", &byout);
        tids[0].pin_in("SLICEWE1", &byinvout);
        tids[1].pin_in("SLICEWE1", &byout);
    }

    test.src_insts.push(inst);
    for ti in tiss {
        test.tgt_insts.push(ti);
    }
    for ti in tids {
        test.tgt_insts.push(ti);
    }
}

fn gen_srlc(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut insts = Vec::new();
    let mut tis = Vec::new();
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
    let mut cd = test.make_in(ctx);
    for i in 0..num {
        let mut inst = SrcInst::new(ctx, "SRLC16E");
        let inp = test.make_ins(ctx, 4);
        for j in 0..4 {
            inst.connect(&format!("A{j}"), &inp[j]);
        }
        let init = ctx.gen_bits(16);
        inst.param_bits("INIT", &init);
        let mut val = 0;
        for i in 0..16 {
            if init[i] == BitVal::S1 {
                val |= 1 << i;
            }
        }
        let ncd = test.make_out(ctx);
        if i % 2 == 0 {
            inst.attr_str("BEL", "G");
            let mut ti = TgtInst::new(slice_kind(mode));
            ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
            ti.cfg("SRINV", if ce_inv { "SR_B" } else { "SR" });
            ti.pin_in("CLK", &clk_x);
            ti.pin_in("SR", &ce_x);
            for j in 0..4 {
                ti.pin_in(&format!("G{ii}", ii = j + 1), &inp[j]);
            }
            ti.bel_ram("G", &inst.name, 4, val);
            ti.cfg("G_ATTR", "SHIFT_REG");
            ti.bel("GMC15_BLACKBOX", "DUMMY", "");
            if i == 0 || (mode != Mode::Virtex2 && i % 4 == 0) {
                ti.cfg("DIG_MUX", "BY");
                ti.cfg("BYINV", "BY");
                ti.pin_in("BY", &cd);
            } else {
                ti.cfg("DIG_MUX", "SHIFTIN");
                ti.pin_in("SHIFTIN", &cd);
            }
            ti.cfg("YBMUX", "0");
            ti.bel("YBUSED", "DUMMY", "0");
            ti.pin_out("YB", &ncd);
            if i == num - 1 {
                ti.bel("WSGEN", &format!("{}.CE", inst.name), "");
            }
            tis.push(ti);
        } else {
            let ti = &mut tis[i / 2];
            for j in 0..4 {
                ti.pin_in(&format!("F{ii}", ii = j + 1), &inp[j]);
            }
            ti.bel_ram("F", &inst.name, 4, val);
            ti.cfg("F_ATTR", "SHIFT_REG");
            ti.bel("FMC15_BLACKBOX", "DUMMY", "");
            ti.cfg("DIF_MUX", "SHIFTIN");
            if i == num - 1 || (mode != Mode::Virtex2 && i % 4 == 3) {
                ti.pin_out("XB", &ncd);
                ti.cfg("XBMUX", "0");
                if mode == Mode::Virtex4 {
                    ti.cfg("XBUSED", "0");
                }
            } else {
                ti.pin_out("SHIFTOUT", &ncd);
                ti.cfg("SHIFTOUTUSED", "0");
            }
            ti.bel("WSGEN", &format!("{}.CE", inst.name), "");
        }
        inst.connect("CLK", &clk_v);
        inst.connect("D", &cd);
        inst.connect("Q15", &ncd);
        inst.connect("CE", &ce_v);
        insts.push(inst);
        cd = ncd;
    }

    for i in 0..(num / 2) {
        let ti = &mut tis[i];
        if ctx.rng.random() {
            let of = test.make_wire(ctx);
            let og = test.make_wire(ctx);
            insts[2 * i].connect("Q", &og);
            insts[2 * i + 1].connect("Q", &of);
            if mode == Mode::Virtex4 {
                ti.cfg("DXMUX", "X");
                ti.cfg("DYMUX", "Y");
            } else {
                ti.cfg("DXMUX", "1");
                ti.cfg("DYMUX", "1");
            }
            make_ffs(
                test,
                ctx,
                mode,
                ti,
                &[('Y', &og), ('X', &of)],
                Some(&clk_v),
                true,
                true,
                None,
                false,
            );
        } else {
            let of = test.make_out(ctx);
            let og = test.make_out(ctx);
            insts[2 * i].connect("Q", &og);
            insts[2 * i + 1].connect("Q", &of);
            ti.cfg("XUSED", "0");
            ti.cfg("YUSED", "0");
            ti.pin_out("X", &of);
            ti.pin_out("Y", &og);
        }
        if mode != Mode::Virtex4 {
            ti.cfg("FXMUX", "F");
            ti.cfg("GYMUX", "G");
        }
    }
    if num % 2 == 1 {
        let ti = &mut tis[num / 2];
        if ctx.rng.random() {
            let og = test.make_wire(ctx);
            insts[num - 1].connect("Q", &og);
            if mode == Mode::Virtex4 {
                ti.cfg("DYMUX", "Y");
            } else {
                ti.cfg("DYMUX", "1");
            }
            make_ffs(
                test,
                ctx,
                mode,
                ti,
                &[('Y', &og)],
                Some(&clk_v),
                true,
                true,
                None,
                false,
            );
        } else {
            let og = test.make_out(ctx);
            insts[num - 1].connect("Q", &og);
            ti.cfg("YUSED", "0");
            ti.pin_out("Y", &og);
        }
        if mode != Mode::Virtex4 {
            ti.cfg("GYMUX", "G");
        }
    }

    for inst in insts {
        test.src_insts.push(inst);
    }
    for ti in tis {
        test.tgt_insts.push(ti);
    }
}

fn gen_ff(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut ti = TgtInst::new(slice_kind(mode));
    let num = if ctx.rng.random() { 1 } else { 2 };
    let mut stuff = Vec::new();
    let uset = ctx.gen_name();
    let rloc = if mode == Mode::Virtex {
        format!("R0C0.S{}", ctx.rng.random_range(0..2))
    } else {
        "X0Y0".to_string()
    };
    let mut inps = Vec::new();
    for i in 0..num {
        let x = ['X', 'Y'][i];
        if mode == Mode::Virtex {
            let (d_v, d_x, d_inv) = test.make_in_inv(ctx);
            ti.pin_in(&format!("B{x}"), &d_x);
            ti.cfg(
                &format!("B{x}MUX"),
                &if d_inv {
                    format!("B{x}_B")
                } else {
                    format!("B{x}")
                },
            );
            inps.push(d_v);
        } else {
            let inp = test.make_in(ctx);
            ti.pin_in(&format!("B{x}"), &inp);
            ti.cfg(&format!("B{x}INV"), &format!("B{x}"));
            inps.push(inp);
        }
        if mode == Mode::Virtex4 {
            ti.cfg(&format!("D{x}MUX"), &format!("B{x}"));
        } else {
            ti.cfg(&format!("D{x}MUX"), "0");
        }
    }
    for i in 0..num {
        let x = ['X', 'Y'][i];
        stuff.push((x, &inps[i][..]));
    }
    make_ffs(
        test,
        ctx,
        mode,
        &mut ti,
        &stuff,
        None,
        false,
        num == 2,
        Some((&uset, &rloc)),
        true,
    );
    test.tgt_insts.push(ti);
}

pub fn gen_clb(ctx: &mut TestGenCtx, mode: Mode, test: &mut Test) {
    if matches!(mode, Mode::Virtex | Mode::Virtex2) {
        for sz in 1..5 {
            gen_tbuf(test, ctx, mode, sz);
        }
    }

    for sz in [1, 2, 3, 4] {
        for _ in 0..5 {
            gen_lut(test, ctx, mode, sz);
        }
    }

    gen_muxf5(test, ctx, mode);
    gen_muxf678(test, ctx, mode, 6);
    if mode != Mode::Virtex {
        gen_muxf678(test, ctx, mode, 7);
        gen_muxf678(test, ctx, mode, 8);
    }

    for num in 1..9 {
        gen_cy(test, ctx, mode, num);
    }
    if mode == Mode::Virtex2 {
        for num in 2..5 {
            gen_orcy(test, ctx, mode, num);
        }
    }

    gen_rom16x1(test, ctx, mode);
    gen_rom32px1(test, ctx, mode, 0);
    if mode != Mode::Virtex {
        gen_rom32px1(test, ctx, mode, 1);
        gen_rom32px1(test, ctx, mode, 2);
        gen_rom32px1(test, ctx, mode, 3);
    }

    gen_ram16s(test, ctx, mode, 1);
    gen_ram16s(test, ctx, mode, 2);
    gen_ram32ps(test, ctx, mode, 0);
    if mode != Mode::Virtex {
        gen_ram32ps(test, ctx, mode, 1);
    }
    if mode == Mode::Virtex2 {
        gen_ram32ps(test, ctx, mode, 2);
    }

    if mode == Mode::Virtex2 {
        gen_ram16d_v2(test, ctx, mode, 1);
        gen_ram16d_v2(test, ctx, mode, 2);
        gen_ram32pd_v2(test, ctx, mode, 0);
        gen_ram32pd_v2(test, ctx, mode, 1);
    } else {
        gen_ram16d(test, ctx, mode);
    }

    gen_srl(test, ctx, mode, 1);
    gen_srl(test, ctx, mode, 2);
    if mode != Mode::Virtex {
        for sz in 1..9 {
            gen_srlc(test, ctx, mode, sz);
        }
    }

    gen_ff(test, ctx, mode);
}
