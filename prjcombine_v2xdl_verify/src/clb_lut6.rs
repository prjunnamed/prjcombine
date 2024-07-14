use crate::types::{BitVal, SrcInst, Test, TestGenCtx, TgtInst};

use rand::{seq::SliceRandom, Rng};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Mode {
    Virtex5,
    Virtex6,
    Virtex7,
    Spartan6,
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
        if init[i >> (dsz - ssz)] == BitVal::S1 {
            val |= 1 << i;
        }
    }
    val
}

fn make_lut6(
    test: &mut Test,
    ctx: &mut TestGenCtx,
    ti: &mut TgtInst,
    c: char,
    out: &str,
    uset: Option<&str>,
) {
    let mut inst = SrcInst::new(ctx, "LUT6");

    let inp = test.make_ins(ctx, 6);
    for i in 0..6 {
        inst.connect(&format!("I{i}"), &inp[i]);
        ti.pin_in(&format!("{c}{ii}", ii = i + 1), &inp[i]);
    }

    inst.attr_str("BEL", &format!("{c}6LUT"));
    inst.connect("O", out);

    let init = gen_lut_init(6, ctx);
    inst.param_bits("INIT", &init);

    ti.bel_lut(&format!("{c}6LUT"), &inst.name, 6, compile_lut(6, 6, &init));

    if let Some(uset) = uset {
        inst.attr_str("RLOC", "X0Y0");
        inst.attr_str("U_SET", uset);
    }

    test.src_insts.push(inst);
}

fn make_lut5_2(
    test: &mut Test,
    ctx: &mut TestGenCtx,
    ti: &mut TgtInst,
    c: char,
    o6: &str,
    o5: &str,
    uset: Option<&str>,
) {
    let mut inst5 = SrcInst::new(ctx, "LUT5");
    let mut inst6 = SrcInst::new(ctx, "LUT5");

    let inp = test.make_ins(ctx, 5);
    for i in 0..5 {
        inst5.connect(&format!("I{i}"), &inp[i]);
        inst6.connect(&format!("I{i}"), &inp[i]);
        ti.pin_in(&format!("{c}{ii}", ii = i + 1), &inp[i]);
    }
    ti.pin_tie(&format!("{c}6"), true);

    inst6.attr_str("BEL", &format!("{c}6LUT"));
    inst5.attr_str("BEL", &format!("{c}5LUT"));
    inst6.connect("O", o6);
    inst5.connect("O", o5);

    let init5 = gen_lut_init(5, ctx);
    let init6 = gen_lut_init(5, ctx);
    inst5.param_bits("INIT", &init5);
    inst6.param_bits("INIT", &init6);

    let mut val6 = 0;
    let mut val5 = 0;
    for i in 0..32 {
        if init5[i] == BitVal::S1 {
            val5 |= 1 << i;
        }
        if init6[i] == BitVal::S1 {
            val6 |= 1 << i;
            val6 |= 1 << (i + 32);
        }
    }
    ti.bel_lut(&format!("{c}6LUT"), &inst6.name, 6, val6);
    ti.bel_lut(&format!("{c}5LUT"), &inst5.name, 5, val5);

    if let Some(uset) = uset {
        inst5.attr_str("RLOC", "X0Y0");
        inst5.attr_str("U_SET", uset);
        inst6.attr_str("RLOC", "X0Y0");
        inst6.attr_str("U_SET", uset);
    }

    test.src_insts.push(inst5);
    test.src_insts.push(inst6);
}

fn make_ffs(
    test: &mut Test,
    ctx: &mut TestGenCtx,
    mode: Mode,
    ti: &mut TgtInst,
    ffs: &[(char, u8, &str)],
    clk: Option<&str>,
    norev: bool,
    uset: Option<&str>,
) -> bool {
    let mut latch = clk.is_none() && ctx.rng.gen();
    for &(_, n, _) in ffs {
        if n == 5 {
            latch = false;
        }
    }
    let clk_v = match clk {
        None => {
            let (x, clk_x, clk_inv) = test.make_in_inv(ctx);
            ti.cfg("CLKINV", if clk_inv ^ latch { "CLK_B" } else { "CLK" });
            ti.pin_in("CLK", &clk_x);
            x
        }
        Some(x) => x.to_string(),
    };
    let ce = if ctx.rng.gen() {
        let ce = test.make_in(ctx);
        ti.pin_in("CE", &ce);
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.cfg("CEUSEDMUX", "IN");
        } else {
            ti.cfg("CEUSED", "0");
        }
        Some(ce)
    } else {
        None
    };
    let sr = if ctx.rng.gen() {
        let sr = test.make_in(ctx);
        ti.pin_in("SR", &sr);
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.cfg("SRUSEDMUX", "IN");
        } else {
            ti.cfg("SRUSED", "0");
        }
        Some(sr)
    } else {
        None
    };
    let rev = if mode == Mode::Virtex5 && !norev && sr.is_some() && clk.is_none() && ctx.rng.gen() {
        let rev = test.make_in(ctx);
        ti.pin_in("DX", &rev);
        ti.cfg("REVUSED", "0");
        Some(rev)
    } else {
        None
    };
    let async_ = latch || ctx.rng.gen();
    ti.cfg("SYNC_ATTR", if async_ { "ASYNC" } else { "SYNC" });
    for &(c, n, d) in ffs {
        let init = ctx.rng.gen();
        let mut rval = if mode == Mode::Spartan6 {
            init
        } else {
            ctx.rng.gen()
        };
        if rev.is_some() {
            rval = false;
        }
        let (prim, rpin, rpin2) = match (latch, async_, rev.is_some(), rval) {
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
        if let Some(uset) = uset {
            inst.attr_str("RLOC", "X0Y0");
            inst.attr_str("U_SET", uset);
        }
        inst.param_bits("INIT", &[if init { BitVal::S1 } else { BitVal::S0 }]);
        let q = test.make_out(ctx);
        let bel = if n == 6 {
            ti.pin_out(&format!("{c}Q"), &q);
            format!("{c}FF")
        } else {
            ti.cfg(&format!("{c}OUTMUX"), &format!("{c}5Q"));
            ti.pin_out(&format!("{c}MUX"), &q);
            format!("{c}5FF")
        };
        ti.bel(
            &bel,
            &inst.name,
            if n == 6 {
                if latch {
                    "#LATCH"
                } else {
                    "#FF"
                }
            } else {
                ""
            },
        );
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            inst.attr_str("BEL", &bel);
        } else {
            if n == 6 {
                inst.attr_str("BEL", &format!("FF{c}"));
            }
        }
        inst.connect("D", d);
        inst.connect(if latch { "G" } else { "C" }, &clk_v);
        if let Some(ref ce) = ce {
            inst.connect(if latch { "GE" } else { "CE" }, ce);
        }
        if let Some(ref sr) = sr {
            inst.connect(rpin, sr);
        }
        if let Some(ref rev) = rev {
            inst.connect(rpin2, rev);
        }
        inst.connect("Q", &q);
        if mode == Mode::Spartan6 {
            ti.cfg(
                &format!("{bel}SRINIT"),
                if init { "SRINIT1" } else { "SRINIT0" },
            );
        } else {
            ti.cfg(&format!("{bel}INIT"), if init { "INIT1" } else { "INIT0" });
            ti.cfg(
                &format!("{bel}SR"),
                if rval && sr.is_some() {
                    "SRHIGH"
                } else {
                    "SRLOW"
                },
            );
        }
        test.src_insts.push(inst);
    }
    ce.is_some()
}

fn gen_lut(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8) {
    let mut inst = SrcInst::new(ctx, &format!("LUT{sz}"));
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL", "SLICEX"]);

    let c = *['A', 'B', 'C', 'D'].choose(&mut ctx.rng).unwrap();
    let l = if sz == 5 {
        // don't use this for sz 1, 2, 3, 4 — map will get smart and pack things
        *[5, 6].choose(&mut ctx.rng).unwrap()
    } else {
        6
    };

    inst.attr_str("BEL", &format!("{c}{l}LUT"));
    let inp = test.make_ins(ctx, sz as usize);
    for i in 0..(sz as usize) {
        inst.connect(&format!("I{i}"), &inp[i]);
    }
    let out;
    if ctx.rng.gen() {
        if l == 6 {
            if ctx.rng.gen() {
                out = test.make_out(ctx);
                ti.cfg(&format!("{c}USED"), "0");
                ti.pin_out(&format!("{c}"), &out);
            } else {
                out = test.make_wire(ctx);
            }
            ti.cfg(&format!("{c}FFMUX"), "O6");
            make_ffs(test, ctx, mode, &mut ti, &[(c, 6, &out)], None, false, None);
        } else if mode == Mode::Virtex5 {
            if ctx.rng.gen() {
                out = test.make_out(ctx);
                ti.cfg(&format!("{c}OUTMUX"), "O5");
                ti.pin_out(&format!("{c}MUX"), &out);
            } else {
                out = test.make_wire(ctx);
            }
            ti.cfg(&format!("{c}FFMUX"), "O5");
            make_ffs(test, ctx, mode, &mut ti, &[(c, 6, &out)], None, false, None);
        } else {
            out = test.make_wire(ctx);
            if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                ti.cfg(&format!("{c}5FFMUX"), "IN_A");
            }
            make_ffs(test, ctx, mode, &mut ti, &[(c, 5, &out)], None, false, None);
        }
    } else {
        out = test.make_out(ctx);
        if l == 6 {
            ti.cfg(&format!("{c}USED"), "0");
            ti.pin_out(&format!("{c}"), &out);
        } else {
            ti.cfg(&format!("{c}OUTMUX"), "O5");
            ti.pin_out(&format!("{c}MUX"), &out);
        }
    };
    inst.connect("O", &out);
    let init = gen_lut_init(sz, ctx);
    inst.param_bits("INIT", &init);

    if l == 6 {
        ti.bel_lut(
            &format!("{c}6LUT"),
            &inst.name,
            6,
            compile_lut(sz, 6, &init),
        );
        for i in 0..(sz as usize) {
            ti.pin_in(
                &format!("{c}{ii}", ii = i + 1 + (6 - (sz as usize))),
                &inp[i],
            );
        }
    } else {
        ti.bel_lut(
            &format!("{c}5LUT"),
            &inst.name,
            5,
            compile_lut(sz, 5, &init),
        );
        for i in 0..(sz as usize) {
            ti.pin_in(
                &format!("{c}{ii}", ii = i + 1 + (5 - (sz as usize))),
                &inp[i],
            );
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_lut6_2(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "LUT6_2");
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL", "SLICEX"]);

    let c = *['A', 'B', 'C', 'D'].choose(&mut ctx.rng).unwrap();

    inst.attr_str("BEL", &format!("{c}6LUT"));
    let inp = test.make_ins(ctx, 6);
    for i in 0..6 {
        inst.connect(&format!("I{i}"), &inp[i]);
        ti.pin_in(&format!("{c}{ii}", ii = i + 1), &inp[i]);
    }
    if mode != Mode::Virtex5 && ctx.rng.gen() {
        let o5 = test.make_wire(ctx);
        let o6 = test.make_wire(ctx);
        inst.connect("O5", &o5);
        inst.connect("O6", &o6);
        ti.cfg(&format!("{c}FFMUX"), "O6");
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.cfg(&format!("{c}5FFMUX"), "IN_A");
        }
        make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[(c, 5, &o5), (c, 6, &o6)],
            None,
            false,
            None,
        );
    } else {
        let o5 = test.make_out(ctx);
        let o6 = test.make_out(ctx);
        inst.connect("O5", &o5);
        inst.connect("O6", &o6);
        ti.cfg(&format!("{c}USED"), "0");
        ti.cfg(&format!("{c}OUTMUX"), "O5");
        ti.pin_out(&format!("{c}MUX"), &o5);
        ti.pin_out(&format!("{c}"), &o6);
    }
    let init = gen_lut_init(6, ctx);
    inst.param_bits("INIT", &init);

    let mut val6 = 0;
    let mut val5 = 0;
    for i in 0..64 {
        if init[i] == BitVal::S1 {
            val6 |= 1 << i;
        }
    }
    for i in 0..32 {
        if init[i] == BitVal::S1 {
            val5 |= 1 << i;
        }
    }
    ti.bel_lut(&format!("{c}6LUT"), &format!("{}/LUT6", inst.name), 6, val6);
    ti.bel_lut(&format!("{c}5LUT"), &format!("{}/LUT5", inst.name), 5, val5);

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_muxf7(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "MUXF7");
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL"]);

    let bel;
    let a;
    let b;
    if ctx.rng.gen() {
        bel = "F7AMUX";
        a = 'A';
        b = 'B';
    } else {
        bel = "F7BMUX";
        a = 'C';
        b = 'D';
    }

    let i0 = test.make_wire(ctx);
    let i1 = test.make_wire(ctx);
    let s = test.make_in(ctx);
    inst.connect("I0", &i0);
    inst.connect("I1", &i1);
    inst.connect("S", &s);
    make_lut6(test, ctx, &mut ti, b, &i0, None);
    make_lut6(test, ctx, &mut ti, a, &i1, None);

    if ctx.rng.gen() {
        let o = test.make_wire(ctx);
        inst.connect("O", &o);
        ti.cfg(&format!("{a}FFMUX"), "F7");
        make_ffs(test, ctx, mode, &mut ti, &[(a, 6, &o)], None, false, None);
    } else {
        let o = test.make_out(ctx);
        inst.connect("O", &o);
        ti.cfg(&format!("{a}OUTMUX"), "F7");
        ti.pin_out(&format!("{a}MUX"), &o);
    }

    ti.bel(bel, &inst.name, "");
    ti.pin_in(&format!("{a}X"), &s);

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_muxf8(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "MUXF8");
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL"]);

    let i0 = test.make_wire(ctx);
    let i1 = test.make_wire(ctx);
    let s = test.make_in(ctx);
    inst.connect("I0", &i0);
    inst.connect("I1", &i1);
    inst.connect("S", &s);

    let i00 = test.make_wire(ctx);
    let i01 = test.make_wire(ctx);
    let s0 = test.make_in(ctx);
    make_lut6(test, ctx, &mut ti, 'D', &i00, None);
    make_lut6(test, ctx, &mut ti, 'C', &i01, None);
    let mut inst_i0 = SrcInst::new(ctx, "MUXF7");
    inst_i0.connect("I0", &i00);
    inst_i0.connect("I1", &i01);
    inst_i0.connect("S", &s0);
    inst_i0.connect("O", &i0);
    ti.bel("F7BMUX", &inst_i0.name, "");
    ti.pin_in("CX", &s0);
    test.src_insts.push(inst_i0);

    let i10 = test.make_wire(ctx);
    let i11 = test.make_wire(ctx);
    let s1 = test.make_in(ctx);
    make_lut6(test, ctx, &mut ti, 'B', &i10, None);
    make_lut6(test, ctx, &mut ti, 'A', &i11, None);
    let mut inst_i1 = SrcInst::new(ctx, "MUXF7");
    inst_i1.connect("I0", &i10);
    inst_i1.connect("I1", &i11);
    inst_i1.connect("S", &s1);
    inst_i1.connect("O", &i1);
    ti.bel("F7AMUX", &inst_i1.name, "");
    ti.pin_in("AX", &s1);
    test.src_insts.push(inst_i1);

    if ctx.rng.gen() {
        let o = test.make_wire(ctx);
        inst.connect("O", &o);
        ti.cfg("BFFMUX", "F8");
        make_ffs(test, ctx, mode, &mut ti, &[('B', 6, &o)], None, false, None);
    } else {
        let o = test.make_out(ctx);
        inst.connect("O", &o);
        ti.cfg("BOUTMUX", "F8");
        ti.pin_out("BMUX", &o);
    }

    ti.bel("F8MUX", &inst.name, "");
    ti.pin_in("BX", &s);

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_carry4(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut ci: Option<String> = None;
    for cidx in 0..num {
        let mut inst = SrcInst::new(ctx, "CARRY4");
        let mut ti = TgtInst::new(&["SLICEM", "SLICEL"]);
        let uset = ctx.gen_name();

        ti.bel("CARRY4", &inst.name, "");
        inst.attr_str("RLOC", "X0Y0");
        inst.attr_str("U_SET", &uset);

        let mut ax_used = false;
        if let Some(ci) = ci {
            inst.connect("CI", &ci);
            ti.pin_in("CIN", &ci);
        } else {
            match ctx.rng.gen_range(0..3) {
                0 => {
                    inst.connect("CYINIT", "0");
                    ti.cfg("PRECYINIT", "0");
                    ti.bel("CYINITGND", "DUMMY", "");
                }
                1 => {
                    inst.connect("CYINIT", "1");
                    ti.cfg("PRECYINIT", "1");
                    ti.bel("CYINITVCC", "DUMMY", "");
                }
                2 => {
                    let cyinit = test.make_in(ctx);
                    inst.connect("CYINIT", &cyinit);
                    ti.cfg("PRECYINIT", "AX");
                    ti.pin_in("AX", &cyinit);
                    ax_used = true;
                }
                _ => unreachable!(),
            }
        }

        let mut di = Vec::new();
        let mut s = Vec::new();
        for (i, l) in [(0, 'A'), (1, 'B'), (2, 'C'), (3, 'D')] {
            let o6 = test.make_wire(ctx);
            if ctx.rng.gen() || (i == 0 && ax_used) {
                let o5 = test.make_wire(ctx);
                make_lut5_2(test, ctx, &mut ti, l, &o6, &o5, Some(&uset));
                ti.cfg(&format!("{l}CY0"), "O5");
                di.push(o5);
            } else {
                let b = test.make_in(ctx);
                make_lut6(test, ctx, &mut ti, l, &o6, Some(&uset));
                ti.pin_in(&format!("{l}X"), &b);
                ti.cfg(&format!("{l}CY0"), &format!("{l}X"));
                di.push(b);
            }
            s.push(o6);
        }
        inst.connect_bus("DI", &di);
        inst.connect_bus("S", &s);

        let mut co;
        match ctx.rng.gen_range(0..5) {
            0 => {
                co = test.make_outs(ctx, 4);
                let xo = test.make_outs(ctx, 4);
                if mode == Mode::Virtex5 {
                    ti.cfg("CLKINV", "CLK");
                    ti.pin_tie("CLK", false);
                } else {
                    ti.cfg("CLKINV", "CLK_B");
                    ti.pin_tie("CLK", true);
                }
                if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                    ti.cfg("CEUSEDMUX", "1");
                    ti.bel("CEUSEDVCC", "DUMMY", "");
                    ti.cfg("SRUSEDMUX", "0");
                    ti.bel("SRUSEDGND", "DUMMY", "");
                } else {
                    if mode == Mode::Spartan6 {
                        ti.cfg("SRUSED", "0");
                        ti.pin_tie("SR", false);
                    }
                    ti.cfg("CEUSED", "0");
                    ti.pin_tie("CE", true);
                }
                ti.cfg("SYNC_ATTR", "ASYNC");
                for (i, l) in [(0, 'A'), (1, 'B'), (2, 'C'), (3, 'D')] {
                    if mode == Mode::Virtex5 {
                        ti.bel(&format!("{l}FF"), "DUMMY", "#LATCH");
                        ti.cfg(&format!("{l}FFINIT"), "INIT0");
                    } else {
                        ti.bel(&format!("{l}FF"), "DUMMY", "#AND2L");
                        if mode == Mode::Spartan6 {
                            ti.cfg(&format!("{l}FFSRINIT"), "SRINIT0");
                        } else {
                            ti.cfg(&format!("{l}FFINIT"), "INIT0");
                            ti.cfg(&format!("{l}FFSR"), "SRLOW");
                        }
                    }
                    ti.cfg(&format!("{l}FFMUX"), "XOR");
                    ti.pin_out(&format!("{l}Q"), &xo[i]);
                    if i == 3 && cidx != num - 1 {
                        ti.pin_out("COUT", &co[i]);
                    } else {
                        ti.cfg(&format!("{l}OUTMUX"), "CY");
                        ti.pin_out(&format!("{l}MUX"), &co[i]);
                    }
                }
                inst.connect_bus("O", &xo);
            }
            1 => {
                co = test.make_outs(ctx, 4);
                let xo = test.make_bus(ctx, 4);
                for (i, l) in [(0, 'A'), (1, 'B'), (2, 'C'), (3, 'D')] {
                    ti.cfg(&format!("{l}FFMUX"), "XOR");
                    if i == 3 && cidx != num - 1 {
                        ti.pin_out("COUT", &co[i]);
                    } else {
                        ti.cfg(&format!("{l}OUTMUX"), "CY");
                        ti.pin_out(&format!("{l}MUX"), &co[i]);
                    }
                }
                make_ffs(
                    test,
                    ctx,
                    mode,
                    &mut ti,
                    &[
                        ('A', 6, &xo[0]),
                        ('B', 6, &xo[1]),
                        ('C', 6, &xo[2]),
                        ('D', 6, &xo[3]),
                    ],
                    None,
                    true,
                    None,
                );
                inst.connect_bus("O", &xo);
            }
            2 => {
                co = test.make_outs(ctx, 4);
                for (i, l) in [(0, 'A'), (1, 'B'), (2, 'C'), (3, 'D')] {
                    if i == 3 && cidx != num - 1 {
                        ti.pin_out("COUT", &co[i]);
                    } else {
                        ti.cfg(&format!("{l}OUTMUX"), "CY");
                        ti.pin_out(&format!("{l}MUX"), &co[i]);
                    }
                }
            }
            3 => {
                co = test.make_bus(ctx, 4);
                for l in ['A', 'B', 'C', 'D'] {
                    ti.cfg(&format!("{l}FFMUX"), "CY");
                }
                make_ffs(
                    test,
                    ctx,
                    mode,
                    &mut ti,
                    &[
                        ('A', 6, &co[0]),
                        ('B', 6, &co[1]),
                        ('C', 6, &co[2]),
                        ('D', 6, &co[3]),
                    ],
                    None,
                    true,
                    None,
                );
            }
            4 => {
                co = test.make_bus(ctx, 4);
                let xo = test.make_outs(ctx, 4);
                for (i, l) in [(0, 'A'), (1, 'B'), (2, 'C'), (3, 'D')] {
                    ti.cfg(&format!("{l}OUTMUX"), "XOR");
                    ti.pin_out(&format!("{l}MUX"), &xo[i]);
                }
                inst.connect_bus("O", &xo);
            }
            _ => unreachable!(),
        }
        inst.connect_bus("CO", &co);

        if cidx != num - 1 {
            ti.pin_out("COUT", &co[3]);
            ti.cfg("COUTUSED", "0");
        }

        test.src_insts.push(inst);
        test.tgt_insts.push(ti);
        ci = Some(co.pop().unwrap());
    }
}

fn gen_rom32x1(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "ROM32X1");
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL", "SLICEX"]);
    let c = *['A', 'B', 'C', 'D'].choose(&mut ctx.rng).unwrap();
    let l = *[5, 6].choose(&mut ctx.rng).unwrap();

    inst.attr_str("BEL", &format!("{c}{l}LUT"));
    let inp = test.make_ins(ctx, 5);
    for i in 0..5 {
        inst.connect(&format!("A{i}"), &inp[i]);
    }
    let out;
    if ctx.rng.gen() {
        if l == 6 {
            if ctx.rng.gen() {
                out = test.make_out(ctx);
                ti.cfg(&format!("{c}USED"), "0");
                ti.pin_out(&format!("{c}"), &out);
            } else {
                out = test.make_wire(ctx);
            }
            ti.cfg(&format!("{c}FFMUX"), "O6");
            make_ffs(test, ctx, mode, &mut ti, &[(c, 6, &out)], None, false, None);
        } else if mode == Mode::Virtex5 {
            if ctx.rng.gen() {
                out = test.make_out(ctx);
                ti.cfg(&format!("{c}OUTMUX"), "O5");
                ti.pin_out(&format!("{c}MUX"), &out);
            } else {
                out = test.make_wire(ctx);
            }
            ti.cfg(&format!("{c}FFMUX"), "O5");
            make_ffs(test, ctx, mode, &mut ti, &[(c, 6, &out)], None, false, None);
        } else {
            out = test.make_wire(ctx);
            if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                ti.cfg(&format!("{c}5FFMUX"), "IN_A");
            }
            make_ffs(test, ctx, mode, &mut ti, &[(c, 5, &out)], None, false, None);
        }
    } else {
        out = test.make_out(ctx);
        if l == 6 {
            ti.cfg(&format!("{c}USED"), "0");
            ti.pin_out(&format!("{c}"), &out);
        } else {
            ti.cfg(&format!("{c}OUTMUX"), "O5");
            ti.pin_out(&format!("{c}MUX"), &out);
        }
    };
    inst.connect("O", &out);
    let init = ctx.gen_bits(32);
    inst.param_bits("INIT", &init);

    if l == 6 {
        let mut val: u64 = 0;
        for i in 0..32 {
            if init[i] == BitVal::S1 {
                val |= 1 << (2 * i);
            }
        }
        ti.bel_rom(&format!("{c}6LUT"), &inst.name, 6, val);
        for i in 0..5 {
            ti.pin_in(&format!("{c}{ii}", ii = i + 2), &inp[i]);
        }
        ti.pin_tie(&format!("{c}1"), false);
    } else {
        let mut val: u64 = 0;
        for i in 0..32 {
            if init[i] == BitVal::S1 {
                val |= 1 << i;
            }
        }
        ti.bel_rom(&format!("{c}5LUT"), &inst.name, 5, val);
        for i in 0..5 {
            ti.pin_in(&format!("{c}{ii}", ii = i + 1), &inp[i]);
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_rom64x1(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "ROM64X1");
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL", "SLICEX"]);
    let c = *['A', 'B', 'C', 'D'].choose(&mut ctx.rng).unwrap();

    inst.attr_str("BEL", &format!("{c}6LUT"));
    let inp = test.make_ins(ctx, 6);
    for i in 0..6 {
        inst.connect(&format!("A{i}"), &inp[i]);
    }
    let out;
    if ctx.rng.gen() {
        if ctx.rng.gen() {
            out = test.make_out(ctx);
            ti.cfg(&format!("{c}USED"), "0");
            ti.pin_out(&format!("{c}"), &out);
        } else {
            out = test.make_wire(ctx);
        }
        ti.cfg(&format!("{c}FFMUX"), "O6");
        make_ffs(test, ctx, mode, &mut ti, &[(c, 6, &out)], None, false, None);
    } else {
        out = test.make_out(ctx);
        ti.cfg(&format!("{c}USED"), "0");
        ti.pin_out(&format!("{c}"), &out);
    };
    inst.connect("O", &out);
    let init = ctx.gen_bits(64);
    inst.param_bits("INIT", &init);

    let mut val: u64 = 0;
    for i in 0..64 {
        if init[i] == BitVal::S1 {
            val |= 1 << i;
        }
    }
    ti.bel_rom(&format!("{c}6LUT"), &inst.name, 6, val);
    for i in 0..6 {
        ti.pin_in(&format!("{c}{ii}", ii = i + 1), &inp[i]);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_rom128x1(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "ROM128X1");
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL"]);

    let bel = "F7BMUX";
    let a = 'C';
    let b = 'D';

    inst.attr_str("BEL", bel);
    let inp = test.make_ins(ctx, 7);
    for i in 0..7 {
        inst.connect(&format!("A{i}"), &inp[i]);
    }
    let out;
    if ctx.rng.gen() {
        out = test.make_wire(ctx);
        ti.cfg(&format!("{a}FFMUX"), "F7");
        make_ffs(test, ctx, mode, &mut ti, &[(a, 6, &out)], None, false, None);
    } else {
        out = test.make_out(ctx);
        ti.cfg(&format!("{a}OUTMUX"), "F7");
        ti.pin_out(&format!("{a}MUX"), &out);
    };
    inst.connect("O", &out);
    let init = ctx.gen_bits(128);
    inst.param_bits("INIT", &init);

    let mut val_a: u64 = 0;
    let mut val_b: u64 = 0;
    for i in 0..64 {
        if init[i] == BitVal::S1 {
            val_b |= 1 << i;
        }
        if init[i + 64] == BitVal::S1 {
            val_a |= 1 << i;
        }
    }
    ti.bel_rom(
        &format!("{a}6LUT"),
        &format!("{}/HIGH", inst.name),
        6,
        val_a,
    );
    ti.bel_rom(&format!("{b}6LUT"), &format!("{}/LOW", inst.name), 6, val_b);
    ti.bel(bel, &format!("{}/F7", inst.name), "");
    for i in 0..6 {
        ti.pin_in(&format!("{a}{ii}", ii = i + 1), &inp[i]);
        ti.pin_in(&format!("{b}{ii}", ii = i + 1), &inp[i]);
    }
    ti.pin_in(&format!("{a}X"), &inp[6]);

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_rom256x1(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "ROM256X1");
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL"]);

    let inp = test.make_ins(ctx, 8);
    for i in 0..8 {
        inst.connect(&format!("A{i}"), &inp[i]);
    }
    let out;
    if ctx.rng.gen() {
        out = test.make_wire(ctx);
        ti.cfg("BFFMUX", "F8");
        make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('B', 6, &out)],
            None,
            false,
            None,
        );
    } else {
        out = test.make_out(ctx);
        ti.cfg("BOUTMUX", "F8");
        ti.pin_out("BMUX", &out);
    };
    inst.connect("O", &out);
    let init = ctx.gen_bits(256);
    inst.param_bits("INIT", &init);

    let mut val_a: u64 = 0;
    let mut val_b: u64 = 0;
    let mut val_c: u64 = 0;
    let mut val_d: u64 = 0;
    for i in 0..64 {
        if init[i] == BitVal::S1 {
            val_d |= 1 << i;
        }
        if init[i + 64] == BitVal::S1 {
            val_c |= 1 << i;
        }
        if init[i + 128] == BitVal::S1 {
            val_b |= 1 << i;
        }
        if init[i + 192] == BitVal::S1 {
            val_a |= 1 << i;
        }
    }
    ti.bel_rom("A6LUT", &format!("{}/A", inst.name), 6, val_a);
    ti.bel_rom("B6LUT", &format!("{}/B", inst.name), 6, val_b);
    ti.bel_rom("C6LUT", &format!("{}/C", inst.name), 6, val_c);
    ti.bel_rom("D6LUT", &format!("{}/D", inst.name), 6, val_d);
    ti.bel("F7AMUX", &format!("{}/F7.A", inst.name), "");
    ti.bel("F7BMUX", &format!("{}/F7.B", inst.name), "");
    ti.bel("F8MUX", &format!("{}/F8", inst.name), "");
    for i in 0..6 {
        ti.pin_in(&format!("A{ii}", ii = i + 1), &inp[i]);
        ti.pin_in(&format!("B{ii}", ii = i + 1), &inp[i]);
        ti.pin_in(&format!("C{ii}", ii = i + 1), &inp[i]);
        ti.pin_in(&format!("D{ii}", ii = i + 1), &inp[i]);
    }
    ti.pin_in("AX", &inp[6]);
    ti.pin_in("BX", &inp[7]);
    ti.pin_in("CX", &inp[6]);

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram32m(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut use_abcd: [bool; 4] = ctx.rng.gen();
    if !use_abcd.iter().any(|&x| x) {
        for x in use_abcd.iter_mut() {
            *x = true;
        }
    }
    let mut inst = SrcInst::new(ctx, "RAM32M");
    let mut ti = TgtInst::new(&["SLICEM"]);
    let mut ffs = if ctx.rng.gen() && mode != Mode::Virtex5 {
        Some(Vec::new())
    } else {
        None
    };
    for c in "ABCD"
        .chars()
        .zip(use_abcd.iter().copied())
        .filter(|&(_, u)| u)
        .map(|(c, _)| c)
    {
        let addr = test.make_ins(ctx, 5);
        inst.connect_bus(&format!("ADDR{c}"), &addr);
        for i in 0..5 {
            ti.pin_in(&format!("{c}{ii}", ii = i + 1), &addr[i]);
        }
        ti.pin_tie(&format!("{c}6"), true);
        let di = test.make_ins(ctx, 2);
        inst.connect_bus(&format!("DI{c}"), &di);
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.pin_in(&format!("{c}I"), &di[0]);
            ti.pin_in(&format!("{c}X"), &di[1]);
        } else {
            ti.pin_in(&format!("{c}X"), &di[0]);
            ti.pin_in(&format!("{c}I"), &di[1]);
        }
        if let Some(ref mut ffs) = ffs {
            let do_ = test.make_bus(ctx, 2);
            inst.connect_bus(&format!("DO{c}"), &do_);
            ffs.push((c, do_));
            ti.cfg(&format!("{c}FFMUX"), "O6");
            if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                ti.cfg(&format!("{c}5FFMUX"), "IN_A");
            }
        } else {
            let do_ = test.make_outs(ctx, 2);
            inst.connect_bus(&format!("DO{c}"), &do_);
            ti.pin_out(&format!("{c}MUX"), &do_[0]);
            ti.pin_out(&format!("{c}"), &do_[1]);
            ti.cfg(&format!("{c}USED"), "0");
            ti.cfg(&format!("{c}OUTMUX"), "O5");
        }
        let init = ctx.gen_bits(64);
        inst.param_bits(&format!("INIT_{c}"), &init);

        ti.cfg(&format!("{c}5RAMMODE"), "DPRAM32");
        ti.cfg(&format!("{c}6RAMMODE"), "DPRAM32");
        let mut ram5 = 0;
        let mut ram6 = 0;
        for i in 0..32 {
            if init[2 * i] == BitVal::S1 {
                ram5 |= 1 << i;
            }
            if init[2 * i + 1] == BitVal::S1 {
                ram6 |= 1 << i;
                ram6 |= 1 << (32 + i);
            }
        }
        ti.bel_ram(
            &format!("{c}5LUT"),
            &format!("{iname}_RAM{c}", iname = inst.name),
            5,
            ram5,
        );
        ti.bel_ram(
            &format!("{c}6LUT"),
            &format!("{iname}_RAM{c}_D1", iname = inst.name),
            6,
            ram6,
        );
        if c != 'D' {
            if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                ti.cfg(&format!("{c}DI1MUX"), &format!("{c}I"));
            } else {
                ti.cfg(&format!("{c}DI1MUX"), &format!("{c}X"));
            }
        }
    }
    if !use_abcd[3] {
        let addrd = test.make_ins(ctx, 5);
        inst.connect_bus("ADDRD", &addrd);
        for i in 0..5 {
            ti.pin_in(&format!("D{ii}", ii = i + 1), &addrd[i]);
        }
        ti.pin_tie("D6", true);
        ti.pin_tie("DI", false);
        ti.pin_tie("DX", false);
        ti.cfg("D5RAMMODE", "DPRAM32");
        ti.cfg("D6RAMMODE", "DPRAM32");
        if mode != Mode::Virtex5 {
            ti.cfg("DOUTMUX", "O5");
            ti.pin_dumout("DMUX");
        }
        ti.bel_ram("D5LUT", &format!("{iname}_RAMD", iname = inst.name), 5, 0);
        ti.bel_ram(
            "D6LUT",
            &format!("{iname}_RAMD_D1", iname = inst.name),
            6,
            0,
        );
    }

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &wclk_v);
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);

    let mut ceused = false;
    if let Some(ffs) = ffs {
        let mut stuff = Vec::new();
        for &(c, ref do_) in &ffs {
            stuff.push((c, 5, &do_[0][..]));
            stuff.push((c, 6, &do_[1][..]));
        }
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &stuff[..],
            Some(&wclk_v),
            true,
            None,
        );
    }

    let we = test.make_in(ctx);
    inst.connect("WE", &we);
    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram64m(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut use_abcd: [bool; 4] = ctx.rng.gen();
    if !use_abcd.iter().any(|&x| x) {
        for x in use_abcd.iter_mut() {
            *x = true;
        }
    }
    let mut inst = SrcInst::new(ctx, "RAM64M");
    let mut ti = TgtInst::new(&["SLICEM"]);
    let mut ffs = if ctx.rng.gen() {
        Some(Vec::new())
    } else {
        None
    };
    for c in "ABCD"
        .chars()
        .zip(use_abcd.iter().copied())
        .filter(|&(_, u)| u)
        .map(|(c, _)| c)
    {
        let addr = test.make_ins(ctx, 6);
        inst.connect_bus(&format!("ADDR{c}"), &addr);
        for i in 0..6 {
            ti.pin_in(&format!("{c}{ii}", ii = i + 1), &addr[i]);
        }
        let di = test.make_in(ctx);
        inst.connect(&format!("DI{c}"), &di);
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.pin_in(&format!("{c}I"), &di);
        } else {
            ti.pin_in(&format!("{c}X"), &di);
        }
        if let Some(ref mut ffs) = ffs {
            let do_ = test.make_wire(ctx);
            inst.connect(&format!("DO{c}"), &do_);
            ffs.push((c, do_));
            ti.cfg(&format!("{c}FFMUX"), "O6");
        } else {
            let do_ = test.make_out(ctx);
            inst.connect(&format!("DO{c}"), &do_);
            ti.pin_out(&format!("{c}"), &do_);
            ti.cfg(&format!("{c}USED"), "0");
        }
        let init = ctx.gen_bits(64);
        inst.param_bits(&format!("INIT_{c}"), &init);

        ti.cfg(&format!("{c}6RAMMODE"), "DPRAM64");
        let mut ram6 = 0;
        for i in 0..64 {
            if init[i] == BitVal::S1 {
                ram6 |= 1 << i;
            }
        }
        ti.bel_ram(
            &format!("{c}6LUT"),
            &format!("{iname}_RAM{c}", iname = inst.name),
            6,
            ram6,
        );
        if c != 'D' {
            if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                ti.cfg(&format!("{c}DI1MUX"), &format!("{c}I"));
            } else {
                ti.cfg(&format!("{c}DI1MUX"), &format!("{c}X"));
            }
        }
    }
    if !use_abcd[3] {
        let addrd = test.make_ins(ctx, 6);
        inst.connect_bus("ADDRD", &addrd);
        for i in 0..6 {
            ti.pin_in(&format!("D{ii}", ii = i + 1), &addrd[i]);
        }
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.pin_tie("DI", false);
        } else {
            ti.pin_tie("DX", false);
        }
        ti.cfg("D6RAMMODE", "DPRAM64");
        if mode != Mode::Virtex5 {
            ti.cfg("DUSED", "0");
            ti.pin_dumout("D");
        }
        ti.bel_ram("D6LUT", &format!("{iname}_RAMD", iname = inst.name), 6, 0);
    }

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &wclk_v);
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);

    let mut ceused = false;
    if let Some(ffs) = ffs {
        let mut stuff = Vec::new();
        for &(c, ref do_) in &ffs {
            stuff.push((c, 6, &do_[..]));
        }
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &stuff[..],
            Some(&wclk_v),
            true,
            None,
        );
    }

    let we = test.make_in(ctx);
    inst.connect("WE", &we);
    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram32x1s(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "RAM32X1S");
    let mut ti = TgtInst::new(&["SLICEM"]);

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &wclk_v);
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);
    let mut ceused = false;

    let addr = test.make_ins(ctx, 5);
    for i in 0..5 {
        inst.connect(&format!("A{i}"), &addr[i]);
        ti.pin_in(&format!("D{ii}", ii = i + 2), &addr[i]);
    }
    ti.pin_tie("D1", false);

    let di = test.make_in(ctx);
    inst.connect("D", &di);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in("DI", &di);
    } else {
        ti.pin_in("DX", &di);
    }
    if ctx.rng.gen() {
        let do_ = test.make_wire(ctx);
        inst.connect("O", &do_);
        ti.cfg("DFFMUX", "O6");
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('D', 6, &do_)],
            Some(&wclk_v),
            true,
            None,
        );
    } else {
        let do_ = test.make_out(ctx);
        inst.connect("O", &do_);
        ti.pin_out("D", &do_);
        ti.cfg("DUSED", "0");
    }
    let init = ctx.gen_bits(32);
    inst.param_bits("INIT", &init);

    ti.cfg("D6RAMMODE", "SPRAM64");
    let mut ram6 = 0;
    for i in 0..32 {
        if init[i] == BitVal::S1 {
            ram6 |= 1 << (2 * i);
        }
    }
    ti.bel_ram("D6LUT", &inst.name, 6, ram6);

    let we = test.make_in(ctx);
    inst.connect("WE", &we);
    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram64x1s(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "RAM64X1S");
    let mut ti = TgtInst::new(&["SLICEM"]);

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &wclk_v);
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);
    let mut ceused = false;

    let addr = test.make_ins(ctx, 6);
    for i in 0..6 {
        inst.connect(&format!("A{i}"), &addr[i]);
        ti.pin_in(&format!("D{ii}", ii = i + 1), &addr[i]);
    }

    let di = test.make_in(ctx);
    inst.connect("D", &di);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in("DI", &di);
    } else {
        ti.pin_in("DX", &di);
    }
    if ctx.rng.gen() {
        let do_ = test.make_wire(ctx);
        inst.connect("O", &do_);
        ti.cfg("DFFMUX", "O6");
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('D', 6, &do_)],
            Some(&wclk_v),
            true,
            None,
        );
    } else {
        let do_ = test.make_out(ctx);
        inst.connect("O", &do_);
        ti.pin_out("D", &do_);
        ti.cfg("DUSED", "0");
    }
    let init = ctx.gen_bits(64);
    inst.param_bits("INIT", &init);

    ti.cfg("D6RAMMODE", "SPRAM64");
    let mut ram6 = 0;
    for i in 0..64 {
        if init[i] == BitVal::S1 {
            ram6 |= 1 << i;
        }
    }
    ti.bel_ram("D6LUT", &inst.name, 6, ram6);

    let we = test.make_in(ctx);
    inst.connect("WE", &we);
    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram128x1s(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut insts = Vec::new();
    for _ in 0..num {
        let inst = SrcInst::new(ctx, "RAM128X1S");
        insts.push(inst);
    }
    let mut ti = TgtInst::new(&["SLICEM"]);

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    for inst in &mut insts {
        inst.connect("WCLK", &wclk_v);
    }
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);
    let mut ceused = false;

    let addr = test.make_ins(ctx, 7);
    for i in 0..7 {
        for inst in &mut insts {
            inst.connect(&format!("A{i}"), &addr[i]);
        }
    }
    for i in 0..6 {
        ti.pin_in(&format!("D{ii}", ii = i + 1), &addr[i]);
        ti.pin_in(&format!("C{ii}", ii = i + 1), &addr[i]);
        if num == 2 {
            ti.pin_in(&format!("B{ii}", ii = i + 1), &addr[i]);
            ti.pin_in(&format!("A{ii}", ii = i + 1), &addr[i]);
        }
    }

    ti.pin_in("CX", &addr[6]);
    let di = test.make_in(ctx);
    insts[0].connect("D", &di);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in("DI", &di);
    } else {
        ti.pin_in("DX", &di);
    }
    if num == 2 {
        ti.pin_in("AX", &addr[6]);
        let di = test.make_in(ctx);
        insts[1].connect("D", &di);
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.pin_in("BI", &di);
        } else {
            ti.pin_in("BX", &di);
        }
    }
    if ctx.rng.gen() || num == 2 {
        // Always use FFs for double RAM, to pin down the BELs (using constraints doesn't seem to
        // work).
        if num == 2 {
            let do_ = test.make_bus(ctx, 2);
            insts[0].connect("O", &do_[0]);
            insts[1].connect("O", &do_[1]);
            ti.cfg("CFFMUX", "F7");
            ti.cfg("AFFMUX", "F7");
            ceused = make_ffs(
                test,
                ctx,
                mode,
                &mut ti,
                &[('A', 6, &do_[1]), ('C', 6, &do_[0])],
                Some(&wclk_v),
                true,
                None,
            );
        } else {
            let do_ = test.make_wire(ctx);
            insts[0].connect("O", &do_);
            ti.cfg("CFFMUX", "F7");
            ceused = make_ffs(
                test,
                ctx,
                mode,
                &mut ti,
                &[('C', 6, &do_)],
                Some(&wclk_v),
                true,
                None,
            );
        }
    } else {
        let do_ = test.make_out(ctx);
        insts[0].connect("O", &do_);
        ti.pin_out("CMUX", &do_);
        ti.cfg("COUTMUX", "F7");
        if num == 2 {
            let do_ = test.make_out(ctx);
            insts[1].connect("O", &do_);
            ti.pin_out("AMUX", &do_);
            ti.cfg("AOUTMUX", "F7");
        }
    }

    let we = test.make_in(ctx);
    for i in 0..num {
        let inst = &mut insts[i];
        let init = ctx.gen_bits(128);
        inst.param_bits("INIT", &init);

        let c = ['C', 'A'][i];
        let d = ['D', 'B'][i];

        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            if i == 0 {
                ti.cfg("CDI1MUX", "DI");
            } else {
                ti.cfg("BDI1MUX", "BI");
                ti.cfg("ADI1MUX", "BDI1");
            }
        } else {
            if i == 0 {
                ti.cfg("CDI1MUX", "DX");
            } else {
                ti.cfg("BDI1MUX", "BX");
                ti.cfg("ADI1MUX", "BDI1");
            }
        }
        ti.cfg(&format!("{c}6RAMMODE"), "SPRAM64");
        ti.cfg(&format!("{d}6RAMMODE"), "SPRAM64");
        let mut ram_c = 0;
        let mut ram_d = 0;
        for i in 0..64 {
            if init[i] == BitVal::S1 {
                ram_d |= 1 << i;
            }
            if init[i + 64] == BitVal::S1 {
                ram_c |= 1 << i;
            }
        }
        ti.bel_ram(
            &format!("{c}6LUT"),
            &format!("{}/HIGH", inst.name),
            6,
            ram_c,
        );
        ti.bel_ram(&format!("{d}6LUT"), &format!("{}/LOW", inst.name), 6, ram_d);
        ti.bel(["F7BMUX", "F7AMUX"][i], &format!("{}/F7", inst.name), "");
        inst.connect("WE", &we);
    }
    ti.cfg("WA7USED", "0");

    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    for inst in insts {
        test.src_insts.push(inst);
    }
    test.tgt_insts.push(ti);
}

fn gen_ram256x1s(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "RAM256X1S");
    let mut ti = TgtInst::new(&["SLICEM"]);

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &wclk_v);
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);
    let mut ceused = false;

    let addr = test.make_ins(ctx, 8);
    inst.connect_bus("A", &addr);
    for i in 0..6 {
        ti.pin_in(&format!("A{ii}", ii = i + 1), &addr[i]);
        ti.pin_in(&format!("B{ii}", ii = i + 1), &addr[i]);
        ti.pin_in(&format!("C{ii}", ii = i + 1), &addr[i]);
        ti.pin_in(&format!("D{ii}", ii = i + 1), &addr[i]);
    }
    ti.pin_in("AX", &addr[6]);
    ti.pin_in("CX", &addr[6]);
    ti.pin_in("BX", &addr[7]);

    let di = test.make_in(ctx);
    inst.connect("D", &di);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in("DI", &di);
    } else {
        ti.pin_in("DX", &di);
    }
    if ctx.rng.gen() {
        let do_ = test.make_wire(ctx);
        inst.connect("O", &do_);
        ti.cfg("BFFMUX", "F8");
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('B', 6, &do_)],
            Some(&wclk_v),
            true,
            None,
        );
    } else {
        let do_ = test.make_out(ctx);
        inst.connect("O", &do_);
        ti.pin_out("BMUX", &do_);
        ti.cfg("BOUTMUX", "F8");
    }
    let init = ctx.gen_bits(256);
    inst.param_bits("INIT", &init);

    ti.cfg("ADI1MUX", "BDI1");
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.cfg("BDI1MUX", "DI");
        ti.cfg("CDI1MUX", "DI");
    } else {
        ti.cfg("BDI1MUX", "DX");
        ti.cfg("CDI1MUX", "DX");
    }
    ti.cfg("A6RAMMODE", "SPRAM64");
    ti.cfg("B6RAMMODE", "SPRAM64");
    ti.cfg("C6RAMMODE", "SPRAM64");
    ti.cfg("D6RAMMODE", "SPRAM64");
    ti.cfg("WA7USED", "0");
    ti.cfg("WA8USED", "0");
    let mut ram_a = 0;
    let mut ram_b = 0;
    let mut ram_c = 0;
    let mut ram_d = 0;
    for i in 0..64 {
        if init[i] == BitVal::S1 {
            ram_d |= 1 << i;
        }
        if init[i + 64] == BitVal::S1 {
            ram_c |= 1 << i;
        }
        if init[i + 128] == BitVal::S1 {
            ram_b |= 1 << i;
        }
        if init[i + 192] == BitVal::S1 {
            ram_a |= 1 << i;
        }
    }
    ti.bel_ram("A6LUT", &format!("{}/A", inst.name), 6, ram_a);
    ti.bel_ram("B6LUT", &format!("{}/B", inst.name), 6, ram_b);
    ti.bel_ram("C6LUT", &format!("{}/C", inst.name), 6, ram_c);
    ti.bel_ram("D6LUT", &format!("{}/D", inst.name), 6, ram_d);
    ti.bel("F7AMUX", &format!("{}/F7.A", inst.name), "");
    ti.bel("F7BMUX", &format!("{}/F7.B", inst.name), "");
    ti.bel("F8MUX", &format!("{}/F8", inst.name), "");

    let we = test.make_in(ctx);
    inst.connect("WE", &we);
    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram32x1d(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "RAM32X1D");
    let mut ti = TgtInst::new(&["SLICEM"]);

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &wclk_v);
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);
    let mut ceused = false;
    let c = if matches!(mode, Mode::Virtex7 | Mode::Spartan6) {
        'B'
    } else {
        'C'
    };

    let addr = test.make_ins(ctx, 5);
    let daddr = test.make_ins(ctx, 5);
    for i in 0..5 {
        inst.connect(&format!("A{i}"), &addr[i]);
        inst.connect(&format!("DPRA{i}"), &daddr[i]);
        ti.pin_in(&format!("{c}{ii}", ii = i + 2), &daddr[i]);
        ti.pin_in(&format!("D{ii}", ii = i + 2), &addr[i]);
    }
    ti.pin_tie(&format!("{c}1"), false);
    ti.pin_tie("D1", false);

    let di = test.make_in(ctx);
    inst.connect("D", &di);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in("DI", &di);
    } else {
        ti.pin_in("DX", &di);
    }
    if ctx.rng.gen() {
        let spo = test.make_wire(ctx);
        let dpo = test.make_wire(ctx);
        inst.connect("SPO", &spo);
        inst.connect("DPO", &dpo);
        ti.cfg("DFFMUX", "O6");
        ti.cfg(&format!("{c}FFMUX"), "O6");
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('D', 6, &spo), (c, 6, &dpo)],
            Some(&wclk_v),
            true,
            None,
        );
    } else {
        let spo = test.make_out(ctx);
        let dpo = test.make_out(ctx);
        inst.connect("SPO", &spo);
        inst.connect("DPO", &dpo);
        ti.pin_out("D", &spo);
        ti.pin_out(&format!("{c}"), &dpo);
        ti.cfg("DUSED", "0");
        ti.cfg(&format!("{c}USED"), "0");
    }
    let init = ctx.gen_bits(32);
    inst.param_bits("INIT", &init);

    ti.cfg(&format!("{c}6RAMMODE"), "DPRAM64");
    ti.cfg("D6RAMMODE", "DPRAM64");
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.cfg(&format!("{c}DI1MUX"), "DI");
    } else {
        ti.cfg(&format!("{c}DI1MUX"), "DX");
    }
    let mut ram6 = 0;
    for i in 0..32 {
        if init[i] == BitVal::S1 {
            ram6 |= 1 << (i * 2);
        }
    }
    ti.bel_ram(&format!("{c}6LUT"), &format!("{}/DP", inst.name), 6, ram6);
    ti.bel_ram("D6LUT", &format!("{}/SP", inst.name), 6, ram6);

    let we = test.make_in(ctx);
    inst.connect("WE", &we);
    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram64x1d(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "RAM64X1D");
    let mut ti = TgtInst::new(&["SLICEM"]);

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &wclk_v);
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);
    let mut ceused = false;
    let c = if matches!(mode, Mode::Virtex7 | Mode::Spartan6) {
        'B'
    } else {
        'C'
    };

    let addr = test.make_ins(ctx, 6);
    let daddr = test.make_ins(ctx, 6);
    for i in 0..6 {
        inst.connect(&format!("A{i}"), &addr[i]);
        inst.connect(&format!("DPRA{i}"), &daddr[i]);
        ti.pin_in(&format!("{c}{ii}", ii = i + 1), &daddr[i]);
        ti.pin_in(&format!("D{ii}", ii = i + 1), &addr[i]);
    }

    let di = test.make_in(ctx);
    inst.connect("D", &di);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in("DI", &di);
    } else {
        ti.pin_in("DX", &di);
    }
    if ctx.rng.gen() {
        let spo = test.make_wire(ctx);
        let dpo = test.make_wire(ctx);
        inst.connect("SPO", &spo);
        inst.connect("DPO", &dpo);
        ti.cfg("DFFMUX", "O6");
        ti.cfg(&format!("{c}FFMUX"), "O6");
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('D', 6, &spo), (c, 6, &dpo)],
            Some(&wclk_v),
            true,
            None,
        );
    } else {
        let spo = test.make_out(ctx);
        let dpo = test.make_out(ctx);
        inst.connect("SPO", &spo);
        inst.connect("DPO", &dpo);
        ti.pin_out("D", &spo);
        ti.pin_out(&format!("{c}"), &dpo);
        ti.cfg("DUSED", "0");
        ti.cfg(&format!("{c}USED"), "0");
    }
    let init = ctx.gen_bits(64);
    inst.param_bits("INIT", &init);

    ti.cfg(&format!("{c}6RAMMODE"), "DPRAM64");
    ti.cfg("D6RAMMODE", "DPRAM64");
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.cfg(&format!("{c}DI1MUX"), "DI");
    } else {
        ti.cfg(&format!("{c}DI1MUX"), "DX");
    }
    let mut ram6 = 0;
    for i in 0..64 {
        if init[i] == BitVal::S1 {
            ram6 |= 1 << i;
        }
    }
    ti.bel_ram(&format!("{c}6LUT"), &format!("{}/DP", inst.name), 6, ram6);
    ti.bel_ram("D6LUT", &format!("{}/SP", inst.name), 6, ram6);

    let we = test.make_in(ctx);
    inst.connect("WE", &we);
    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ram128x1d(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "RAM128X1D");
    let mut ti = TgtInst::new(&["SLICEM"]);

    let (wclk_v, wclk_x, wclk_inv) = test.make_in_inv(ctx);
    inst.connect("WCLK", &wclk_v);
    ti.cfg("CLKINV", if wclk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &wclk_x);
    let mut ceused = false;

    let addr = test.make_ins(ctx, 7);
    let daddr = test.make_ins(ctx, 7);
    inst.connect_bus("A", &addr);
    inst.connect_bus("DPRA", &daddr);
    for i in 0..6 {
        ti.pin_in(&format!("A{ii}", ii = i + 1), &daddr[i]);
        ti.pin_in(&format!("B{ii}", ii = i + 1), &daddr[i]);
        ti.pin_in(&format!("C{ii}", ii = i + 1), &addr[i]);
        ti.pin_in(&format!("D{ii}", ii = i + 1), &addr[i]);
    }
    ti.pin_in("AX", &daddr[6]);
    ti.pin_in("CX", &addr[6]);

    let di = test.make_in(ctx);
    inst.connect("D", &di);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in("DI", &di);
    } else {
        ti.pin_in("DX", &di);
    }
    if ctx.rng.gen() {
        let spo = test.make_wire(ctx);
        let dpo = test.make_wire(ctx);
        inst.connect("SPO", &spo);
        inst.connect("DPO", &dpo);
        ti.cfg("CFFMUX", "F7");
        ti.cfg("AFFMUX", "F7");
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[('C', 6, &spo), ('A', 6, &dpo)],
            Some(&wclk_v),
            true,
            None,
        );
    } else {
        let spo = test.make_out(ctx);
        let dpo = test.make_out(ctx);
        inst.connect("SPO", &spo);
        inst.connect("DPO", &dpo);
        ti.pin_out("CMUX", &spo);
        ti.pin_out("AMUX", &dpo);
        ti.cfg("COUTMUX", "F7");
        ti.cfg("AOUTMUX", "F7");
    }
    let init = ctx.gen_bits(128);
    inst.param_bits("INIT", &init);

    ti.cfg("A6RAMMODE", "DPRAM64");
    ti.cfg("B6RAMMODE", "DPRAM64");
    ti.cfg("C6RAMMODE", "DPRAM64");
    ti.cfg("D6RAMMODE", "DPRAM64");
    ti.cfg("ADI1MUX", "BDI1");
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.cfg("BDI1MUX", "DI");
        ti.cfg("CDI1MUX", "DI");
    } else {
        ti.cfg("BDI1MUX", "DX");
        ti.cfg("CDI1MUX", "DX");
    }
    ti.cfg("WA7USED", "0");
    let mut ram_c = 0;
    let mut ram_d = 0;
    for i in 0..64 {
        if init[i] == BitVal::S1 {
            ram_d |= 1 << i;
        }
        if init[i + 64] == BitVal::S1 {
            ram_c |= 1 << i;
        }
    }
    ti.bel_ram("A6LUT", &format!("{}/DP.HIGH", inst.name), 6, ram_c);
    ti.bel_ram("B6LUT", &format!("{}/DP.LOW", inst.name), 6, ram_d);
    ti.bel_ram("C6LUT", &format!("{}/SP.HIGH", inst.name), 6, ram_c);
    ti.bel_ram("D6LUT", &format!("{}/SP.LOW", inst.name), 6, ram_d);
    ti.bel("F7AMUX", &format!("{}/F7.DP", inst.name), "");
    ti.bel("F7BMUX", &format!("{}/F7.SP", inst.name), "");

    let we = test.make_in(ctx);
    inst.connect("WE", &we);
    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &we);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &we);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_srl16(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut insts = Vec::new();
    for _ in 0..num {
        let inst = SrcInst::new(ctx, "SRL16E");
        insts.push(inst);
    }
    let mut ti = TgtInst::new(&["SLICEM"]);
    let l = *['A', 'B', 'C', 'D'].choose(&mut ctx.rng).unwrap();

    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &clk_x);
    let ce = test.make_in(ctx);
    let mut ceused = false;
    let uset = ctx.gen_name();

    for inst in &mut insts {
        inst.attr_str("RLOC", "X0Y0");
        inst.attr_str("U_SET", &uset);
        inst.connect("CLK", &clk_v);
        inst.connect("CE", &ce);
    }

    let d = test.make_in(ctx);
    insts[0].connect("D", &d);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in(&format!("{l}X"), &d);
    } else {
        ti.pin_in(&format!("{l}I"), &d);
    }
    if num == 2 {
        let d = test.make_in(ctx);
        insts[1].connect("D", &d);
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.pin_in(&format!("{l}I"), &d);
        } else {
            ti.pin_in(&format!("{l}X"), &d);
        }
    }

    insts[0].attr_str("BEL", &format!("{l}6LUT"));
    if num == 2 {
        insts[1].attr_str("BEL", &format!("{l}5LUT"));
    }
    let inp = test.make_ins(ctx, 4);
    for j in 0..4 {
        for inst in &mut insts {
            inst.connect(&format!("A{j}"), &inp[j]);
        }
        ti.pin_in(&format!("{l}{ii}", ii = j + 2), &inp[j]);
    }
    ti.pin_tie(&format!("{l}6"), true);
    ti.pin_tie(&format!("{l}1"), true);

    let init = ctx.gen_bits(16);
    insts[0].param_bits("INIT", &init);
    let mut val6 = 0;
    for i in 0..16 {
        if init[i] == BitVal::S1 {
            val6 |= 1 << i;
        }
    }
    ti.bel_ram(&format!("{l}6LUT"), &insts[0].name, 6, val6);
    ti.cfg(&format!("{l}6RAMMODE"), "SRL16");
    if num == 2 {
        let init = ctx.gen_bits(16);
        insts[1].param_bits("INIT", &init);
        let mut val6 = 0;
        for i in 0..16 {
            if init[i] == BitVal::S1 {
                val6 |= 1 << i;
            }
        }
        ti.bel_ram(&format!("{l}5LUT"), &insts[1].name, 5, val6);
        ti.cfg(&format!("{l}5RAMMODE"), "SRL16");
        if l != 'D' {
            if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                ti.cfg(&format!("{l}DI1MUX"), &format!("{l}I"));
            } else {
                ti.cfg(&format!("{l}DI1MUX"), &format!("{l}X"));
            }
        }
    }

    if ctx.rng.gen() {
        let o6 = test.make_wire(ctx);
        insts[0].connect("Q", &o6);
        ti.cfg(&format!("{l}FFMUX"), "O6");
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[(l, 6, &o6)],
            Some(&clk_v),
            true,
            None,
        );
    } else {
        let o6 = test.make_out(ctx);
        insts[0].connect("Q", &o6);
        ti.cfg(&format!("{l}USED"), "0");
        ti.pin_out(&format!("{l}"), &o6);
    }

    if num == 2 {
        let o5 = test.make_out(ctx);
        insts[1].connect("Q", &o5);
        ti.cfg(&format!("{l}OUTMUX"), "O5");
        ti.pin_out(&format!("{l}MUX"), &o5);
    }

    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &ce);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &ce);
    }

    for inst in insts {
        test.src_insts.push(inst);
    }
    test.tgt_insts.push(ti);
}

fn gen_srl32(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "SRLC32E");
    let mut ti = TgtInst::new(&["SLICEM"]);
    let l = *['A', 'B', 'C', 'D'].choose(&mut ctx.rng).unwrap();

    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &clk_x);
    let ce = test.make_in(ctx);
    let mut ceused = false;
    inst.connect("CLK", &clk_v);
    inst.connect("CE", &ce);

    let d = test.make_in(ctx);
    inst.connect("D", &d);
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in(&format!("{l}I"), &d);
    } else {
        ti.pin_in(&format!("{l}X"), &d);
    }

    inst.attr_str("BEL", &format!("{l}6LUT"));
    let inp = test.make_ins(ctx, 5);
    inst.connect_bus("A", &inp);
    for j in 0..5 {
        ti.pin_in(&format!("{l}{ii}", ii = j + 2), &inp[j]);
    }
    ti.pin_tie(&format!("{l}1"), true);
    let init = ctx.gen_bits(32);
    inst.param_bits("INIT", &init);
    let mut val6 = 0;
    for i in 0..32 {
        if init[i] == BitVal::S1 {
            val6 |= 1 << i;
        }
    }
    ti.bel_ram(&format!("{l}6LUT"), &inst.name, 6, val6);
    ti.cfg(&format!("{l}6RAMMODE"), "SRL32");
    if l != 'D' {
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
            ti.cfg(&format!("{l}DI1MUX"), &format!("{l}I"));
        } else {
            ti.cfg(&format!("{l}DI1MUX"), &format!("{l}X"));
        }
    }

    if ctx.rng.gen() {
        let o6 = test.make_wire(ctx);
        inst.connect("Q", &o6);
        ti.cfg(&format!("{l}FFMUX"), "O6");
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &[(l, 6, &o6)],
            Some(&clk_v),
            true,
            None,
        );
    } else {
        let o6 = test.make_out(ctx);
        inst.connect("Q", &o6);
        ti.cfg(&format!("{l}USED"), "0");
        ti.pin_out(&format!("{l}"), &o6);
    }

    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &ce);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &ce);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_srlc(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize, sz: u8) {
    let mut insts = Vec::new();
    for _ in 0..num {
        let inst = SrcInst::new(ctx, if sz == 16 { "SRLC16E" } else { "SRLC32E" });
        insts.push(inst);
    }
    let mut ti = TgtInst::new(&["SLICEM"]);
    let lets = ['A', 'B', 'C', 'D'];
    let uset = ctx.gen_name();

    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &clk_x);
    let ce = test.make_in(ctx);
    let mut ceused = false;
    for inst in &mut insts {
        inst.attr_str("RLOC", "X0Y0");
        inst.attr_str("U_SET", &uset);
        inst.connect("CLK", &clk_v);
        inst.connect("CE", &ce);
    }
    let mut cd = Vec::new();
    for _ in 0..(num - 1) {
        cd.push(test.make_wire(ctx));
    }
    cd.push(test.make_in(ctx));
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        if sz == 16 {
            ti.pin_in(&format!("{l}X", l = lets[num - 1]), &cd[num - 1]);
        } else {
            ti.pin_in(&format!("{l}I", l = lets[num - 1]), &cd[num - 1]);
        }
    } else {
        if sz == 16 {
            ti.pin_in(&format!("{l}I", l = lets[num - 1]), &cd[num - 1]);
        } else {
            ti.pin_in(&format!("{l}X", l = lets[num - 1]), &cd[num - 1]);
        }
    }

    for i in 0..num {
        let l = lets[i];
        let inst = &mut insts[i];
        inst.attr_str("BEL", &format!("{l}6LUT"));
        if sz == 16 {
            let inp = test.make_ins(ctx, 4);
            for j in 0..4 {
                inst.connect(&format!("A{j}"), &inp[j]);
                ti.pin_in(&format!("{l}{ii}", ii = j + 2), &inp[j]);
            }
            ti.pin_tie(&format!("{l}6"), true);
        } else {
            let inp = test.make_ins(ctx, 5);
            inst.connect_bus("A", &inp);
            for j in 0..5 {
                ti.pin_in(&format!("{l}{ii}", ii = j + 2), &inp[j]);
            }
        }
        ti.pin_tie(&format!("{l}1"), true);
        if sz == 16 {
            let init = ctx.gen_bits(16);
            inst.param_bits("INIT", &init);
            let mut val6 = 0;
            for i in 0..16 {
                if init[i] == BitVal::S1 {
                    val6 |= 1 << i;
                }
            }
            ti.bel_ram(&format!("{l}6LUT"), &inst.name, 6, val6);
        } else {
            let init = ctx.gen_bits(32);
            inst.param_bits("INIT", &init);
            let mut val6 = 0;
            for i in 0..32 {
                if init[i] == BitVal::S1 {
                    val6 |= 1 << i;
                }
            }
            ti.bel_ram(&format!("{l}6LUT"), &inst.name, 6, val6);
        }
        ti.cfg(
            &format!("{l}6RAMMODE"),
            if sz == 16 { "SRL16" } else { "SRL32" },
        );
        if i < 3 && sz == 32 {
            if i == num - 1 {
                if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                    ti.cfg(&format!("{l}DI1MUX"), &format!("{l}I"));
                } else {
                    ti.cfg(&format!("{l}DI1MUX"), &format!("{l}X"));
                }
            } else {
                let nl = lets[i + 1];
                ti.cfg(&format!("{l}DI1MUX"), &format!("{nl}MC31"));
            }
        }
    }

    let q31 = if sz == 16 { "Q15" } else { "Q31" };

    if ctx.rng.gen() {
        let mut o6 = Vec::new();
        for i in 0..num {
            let l = lets[i];
            o6.push(test.make_wire(ctx));
            insts[i].connect("Q", &o6[i]);
            ti.cfg(&format!("{l}FFMUX"), "O6");
        }
        let mut stuff = Vec::new();
        for i in 0..num {
            stuff.push((lets[i], 6, &o6[i][..]));
        }
        let mc31 = test.make_wire(ctx);
        if num != 4 {
            insts[0].connect(q31, &mc31);
            ti.cfg("DFFMUX", "MC31");
            stuff.push(('D', 6, &mc31[..]));
        } else {
            let mc31 = test.make_out(ctx);
            insts[0].connect(q31, &mc31);
            ti.pin_out("DMUX", &mc31);
            ti.cfg("DOUTMUX", "MC31");
        }
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &stuff[..],
            Some(&clk_v),
            true,
            Some(&uset),
        );
    } else {
        for i in 0..num {
            let l = lets[i];
            let o6 = test.make_out(ctx);
            insts[i].connect("Q", &o6);
            ti.cfg(&format!("{l}USED"), "0");
            ti.pin_out(&format!("{l}"), &o6);
        }
        let mc31 = test.make_out(ctx);
        insts[0].connect(q31, &mc31);
        ti.pin_out("DMUX", &mc31);
        ti.cfg("DOUTMUX", "MC31");
    }

    for i in 0..num {
        insts[i].connect("D", &cd[i]);
        if i > 0 {
            insts[i].connect(q31, &cd[i - 1]);
        }
    }

    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &ce);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &ce);
    }

    for inst in insts {
        test.src_insts.push(inst);
    }
    test.tgt_insts.push(ti);
}

fn gen_cfglut5(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut insts = Vec::new();
    for _ in 0..num {
        let inst = SrcInst::new(ctx, "CFGLUT5");
        insts.push(inst);
    }
    let mut ti = TgtInst::new(&["SLICEM"]);
    let lets = ['A', 'B', 'C', 'D'];

    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    ti.cfg("CLKINV", if clk_inv { "CLK_B" } else { "CLK" });
    ti.pin_in("CLK", &clk_x);
    let ce = test.make_in(ctx);
    let mut ceused = false;
    for inst in &mut insts {
        inst.connect("CLK", &clk_v);
        inst.connect("CE", &ce);
    }
    let mut cd = Vec::new();
    for _ in 0..(num - 1) {
        cd.push(test.make_wire(ctx));
    }
    cd.push(test.make_in(ctx));
    if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
        ti.pin_in(&format!("{l}I", l = lets[num - 1]), &cd[num - 1]);
    } else {
        ti.pin_in(&format!("{l}X", l = lets[num - 1]), &cd[num - 1]);
    }

    for i in 0..num {
        let inst = &mut insts[i];
        let inp = test.make_ins(ctx, 5);
        let l = lets[i];
        for j in 0..5 {
            inst.connect(&format!("I{j}"), &inp[j]);
            ti.pin_in(&format!("{l}{ii}", ii = j + 2), &inp[j]);
        }
        ti.pin_tie(&format!("{l}1"), true);
        let init = ctx.gen_bits(32);
        inst.param_bits("INIT", &init);
        let mut val5 = 0;
        let mut val6 = 0;
        for i in 0..16 {
            if init[i] == BitVal::S1 {
                val5 |= 1 << i;
            }
        }
        for i in 0..32 {
            if init[i] == BitVal::S1 {
                val6 |= 1 << i;
            }
        }
        ti.bel_ram(&format!("{l}5LUT"), &format!("{}/O5", inst.name), 5, val5);
        ti.bel_ram(&format!("{l}6LUT"), &format!("{}/O6", inst.name), 6, val6);
        ti.cfg(&format!("{l}5RAMMODE"), "SRL32");
        ti.cfg(&format!("{l}6RAMMODE"), "SRL32");
        if i < 3 {
            if i == num - 1 {
                if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                    ti.cfg(&format!("{l}DI1MUX"), &format!("{l}I"));
                } else {
                    ti.cfg(&format!("{l}DI1MUX"), &format!("{l}X"));
                }
            } else {
                let nl = lets[i + 1];
                ti.cfg(&format!("{l}DI1MUX"), &format!("{nl}MC31"));
            }
        }
    }

    if ctx.rng.gen() {
        let mut o6 = Vec::new();
        for i in 0..num {
            let l = lets[i];
            o6.push(test.make_wire(ctx));
            insts[i].connect("O6", &o6[i]);
            ti.cfg(&format!("{l}FFMUX"), "O6");
        }
        let mut stuff = Vec::new();
        for i in 0..num {
            stuff.push((lets[i], 6, &o6[i][..]));
        }
        let mc31 = test.make_wire(ctx);
        if num != 4 {
            insts[0].connect("CDO", &mc31);
            ti.cfg("DFFMUX", "MC31");
            stuff.push(('D', 6, &mc31[..]));
        }
        ceused = make_ffs(
            test,
            ctx,
            mode,
            &mut ti,
            &stuff[..],
            Some(&clk_v),
            true,
            None,
        );
    } else {
        for i in 0..num {
            let l = lets[i];
            let o6 = test.make_out(ctx);
            insts[i].connect("O6", &o6);
            ti.cfg(&format!("{l}USED"), "0");
            ti.pin_out(&format!("{l}"), &o6);
        }
        if num != 4 {
            let mc31 = test.make_out(ctx);
            insts[0].connect("CDO", &mc31);
            ti.pin_out("DMUX", &mc31);
            ti.cfg("DOUTMUX", "MC31");
        }
    }

    for i in 0..num {
        let l = lets[i];
        let o5 = test.make_out(ctx);
        insts[i].connect("O5", &o5);
        ti.cfg(&format!("{l}OUTMUX"), "O5");
        ti.pin_out(&format!("{l}MUX"), &o5);
    }

    for i in 0..num {
        insts[i].connect("CDI", &cd[i]);
        if i > 0 {
            insts[i].connect("CDO", &cd[i - 1]);
        }
    }

    if ceused {
        ti.cfg("WEMUX", "WE");
        ti.pin_in("WE", &ce);
    } else {
        ti.cfg("WEMUX", "CE");
        ti.pin_in("CE", &ce);
    }

    for inst in insts {
        test.src_insts.push(inst);
    }
    test.tgt_insts.push(ti);
}

fn gen_ff(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut ti = TgtInst::new(&["SLICEM", "SLICEL", "SLICEX"]);
    let num = if mode == Mode::Virtex5 && ctx.rng.gen() {
        3
    } else {
        4
    };
    let inps = test.make_ins(ctx, num);
    let mut stuff = Vec::new();
    let uset = ctx.gen_name();
    for i in 0..num {
        let l = ['A', 'B', 'C', 'D'][i];
        if matches!(mode, Mode::Virtex6 | Mode::Virtex7) && ctx.rng.gen() {
            ti.cfg(&format!("{l}5FFMUX"), "IN_B");
            ti.pin_in(&format!("{l}X"), &inps[i]);
            stuff.push((l, 5, &inps[i][..]));
        } else {
            ti.cfg(&format!("{l}FFMUX"), &format!("{l}X"));
            ti.pin_in(&format!("{l}X"), &inps[i]);
            stuff.push((l, 6, &inps[i][..]));
        }
    }
    make_ffs(
        test,
        ctx,
        mode,
        &mut ti,
        &stuff,
        None,
        num == 4,
        Some(&uset),
    );
    test.tgt_insts.push(ti);
}

pub fn gen_clb(ctx: &mut TestGenCtx, mode: Mode, test: &mut Test) {
    for sz in [1, 2, 3, 4, 5, 6] {
        for _ in 0..5 {
            gen_lut(test, ctx, mode, sz);
        }
    }
    for _ in 0..5 {
        gen_lut6_2(test, ctx, mode);
    }

    gen_muxf7(test, ctx, mode);
    gen_muxf8(test, ctx, mode);

    gen_carry4(test, ctx, mode, 1);
    gen_carry4(test, ctx, mode, 2);
    gen_carry4(test, ctx, mode, 3);

    gen_rom32x1(test, ctx, mode);
    gen_rom64x1(test, ctx, mode);
    gen_rom128x1(test, ctx, mode);
    gen_rom256x1(test, ctx, mode);

    for _ in 0..10 {
        gen_ram32m(test, ctx, mode);
        gen_ram64m(test, ctx, mode);
    }

    gen_ram32x1s(test, ctx, mode);
    gen_ram64x1s(test, ctx, mode);
    gen_ram128x1s(test, ctx, mode, 1);
    gen_ram128x1s(test, ctx, mode, 2);
    gen_ram256x1s(test, ctx, mode);

    gen_ram32x1d(test, ctx, mode);
    gen_ram64x1d(test, ctx, mode);
    gen_ram128x1d(test, ctx, mode);

    gen_srl16(test, ctx, mode, 1);
    gen_srl16(test, ctx, mode, 2);
    gen_srl32(test, ctx, mode);
    gen_srlc(test, ctx, mode, 1, 16);
    gen_srlc(test, ctx, mode, 1, 32);
    gen_srlc(test, ctx, mode, 2, 32);
    gen_srlc(test, ctx, mode, 3, 32);
    gen_srlc(test, ctx, mode, 4, 32);
    gen_cfglut5(test, ctx, mode, 1);
    gen_cfglut5(test, ctx, mode, 2);
    gen_cfglut5(test, ctx, mode, 3);
    gen_cfglut5(test, ctx, mode, 4);

    gen_ff(test, ctx, mode);
}
