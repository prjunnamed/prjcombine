use crate::types::{Test, SrcInst, TgtInst, TestGenCtx};

use rand::{Rng, seq::SliceRandom};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Mode {
    Virtex2,
    Spartan3E,
    Spartan3ADsp,
    Spartan6,
    Virtex4,
    Virtex5,
    Virtex6,
    Series7,
}

fn gen_mult18x18(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let sync = ctx.rng.gen();
    let mut inst = SrcInst::new(ctx, if sync {"MULT18X18S"} else {"MULT18X18"});
    let hwprim = match mode {
        Mode::Virtex2 => "MULT18X18",
        Mode::Spartan3E => "MULT18X18SIO",
        Mode::Spartan3ADsp => "DSP48A",
        Mode::Spartan6 => "DSP48A1",
        Mode::Virtex4 => "DSP48",
        Mode::Virtex5 => "DSP48E",
        Mode::Virtex6 => "DSP48E1",
        Mode::Series7 => "DSP48E1",
    };
    let mut ti = TgtInst::new(&[hwprim]);

    let a = test.make_ins(ctx, 18);
    let b = test.make_ins(ctx, 18);
    let p = test.make_outs(ctx, 36);
    inst.connect_bus("A", &a);
    inst.connect_bus("B", &b);
    inst.connect_bus("P", &p);
    if mode == Mode::Virtex2 {
        ti.bel("BLACKBOX", &inst.name, "");
    } else {
        ti.bel(hwprim, &inst.name, "");
    }
    for i in 0..18 {
        ti.pin_in(&format!("A{i}"), &a[i]);
        ti.pin_in(&format!("B{i}"), &b[i]);
    }
    for i in 0..36 {
        if mode == Mode::Spartan6 {
            ti.pin_out(&format!("M{i}"), &p[i]);
        } else {
            ti.pin_out(&format!("P{i}"), &p[i]);
        }
    }

    let tieval_ce = mode == Mode::Virtex6;
    let tieval_rst = !matches!(mode, Mode::Virtex6 | Mode::Series7);

    if sync {
        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        let (ce_v, ce_x, ce_inv);
        let (rst_v, rst_x, rst_inv);
        if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
            ce_v = test.make_in(ctx);
            ce_x = ce_v.clone();
            ce_inv = false;
            rst_v = test.make_in(ctx);
            rst_x = rst_v.clone();
            rst_inv = false;
        } else {
            (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
            (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
        }
        inst.connect("C", &clk_v);
        inst.connect("CE", &ce_v);
        inst.connect("R", &rst_v);
        if mode == Mode::Virtex2 {
            ti.pin_in_inv("CLK", &clk_x, clk_inv);
            ti.pin_in_inv("CE", &ce_x, ce_inv);
            ti.pin_in_inv("RST", &rst_x, rst_inv);
        } else if mode == Mode::Spartan3E {
            ti.pin_in_inv("CLK", &clk_x, clk_inv);
            ti.pin_in_inv("CEA", &ce_x, ce_inv);
            ti.pin_in_inv("CEB", &ce_x, ce_inv);
            ti.pin_in_inv("CEP", &ce_x, ce_inv);
            ti.pin_in_inv("RSTA", &rst_x, rst_inv);
            ti.pin_in_inv("RSTB", &rst_x, rst_inv);
            ti.pin_in_inv("RSTP", &rst_x, rst_inv);
            ti.cfg_int("AREG", 0);
            ti.cfg_int("BREG", 0);
            ti.cfg_int("PREG", 1);
            ti.cfg("B_INPUT", "DIRECT");
            ti.cfg("PREG_CLKINVERSION", "0");
        } else if matches!(mode, Mode::Spartan3ADsp | Mode::Spartan6) {
            ti.cfg_int("MREG", 1);
            ti.pin_in_inv("CLK", &clk_x, clk_inv);
            ti.pin_in_inv("CEM", &ce_x, ce_inv);
            ti.pin_in_inv("RSTM", &rst_x, rst_inv);
        } else if mode == Mode::Virtex4 {
            ti.cfg("LEGACY_MODE", "MULT18X18S");
            ti.cfg_int("MREG", 1);
            ti.pin_in_inv("CLK", &clk_x, clk_inv);
            ti.pin_in_inv("CEM", &ce_x, ce_inv);
            ti.pin_in_inv("RSTM", &rst_x, rst_inv);
        } else if mode == Mode::Virtex5 {
            ti.cfg("USE_MULT", "MULT_S");
            ti.cfg_int("MREG", 1);
            ti.pin_in_inv("CLK", &clk_x, clk_inv);
            ti.pin_in("CEM", &ce_x);
            ti.pin_in("RSTM", &rst_x);
        } else if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            ti.cfg("USE_MULT", "MULTIPLY");
            ti.cfg_int("MREG", 1);
            ti.pin_in_inv("CLK", &clk_x, clk_inv);
            ti.pin_in("CEM", &ce_x);
            ti.pin_in("RSTM", &rst_x);
        }
    } else {
        if mode == Mode::Spartan3E {
            ti.pin_tie_inv("CLK", false, false);
            ti.pin_tie_inv("CEA", false, false);
            ti.pin_tie_inv("CEB", false, false);
            ti.pin_tie_inv("CEP", false, false);
            ti.pin_tie_inv("RSTA", true, false);
            ti.pin_tie_inv("RSTB", true, false);
            ti.pin_tie_inv("RSTP", true, false);
            ti.cfg_int("AREG", 0);
            ti.cfg_int("BREG", 0);
            ti.cfg_int("PREG", 0);
            ti.cfg("B_INPUT", "DIRECT");
            ti.cfg("PREG_CLKINVERSION", "0");
        } else if matches!(mode, Mode::Spartan3ADsp | Mode::Spartan6) {
            ti.cfg_int("MREG", 0);
            ti.pin_tie_inv("CLK", false, false);
            ti.pin_tie_inv("CEM", false, false);
            ti.pin_tie_inv("RSTM", true, false);
        } else if mode == Mode::Virtex4 {
            ti.cfg("LEGACY_MODE", "MULT18X18");
            ti.cfg_int("MREG", 0);
            ti.pin_tie_inv("CLK", false, false);
            ti.pin_tie_inv("CEM", false, false);
            ti.pin_tie_inv("RSTM", true, false);
        } else if mode == Mode::Virtex5 {
            ti.cfg("USE_MULT", "MULT");
            ti.cfg_int("MREG", 0);
            ti.pin_tie_inv("CLK", false, false);
            ti.pin_tie("CEM", false);
            ti.pin_tie("RSTM", tieval_rst);
        } else if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            ti.cfg("USE_MULT", "MULTIPLY");
            ti.cfg_int("MREG", 0);
            ti.pin_tie_inv("CLK", false, false);
            ti.pin_tie("CEM", tieval_ce);
            ti.pin_tie("RSTM", tieval_rst);
        }
    }

    if matches!(mode, Mode::Spartan3ADsp | Mode::Spartan6) {
        ti.cfg_int("A0REG", 0);
        ti.cfg_int("A1REG", 0);
        ti.cfg_int("B0REG", 0);
        ti.cfg_int("B1REG", 0);
        ti.cfg_int("CREG", 0);
        ti.cfg_int("DREG", 0);
        ti.cfg_int("PREG", 0);
        ti.cfg_int("OPMODEREG", 0);
        ti.cfg_int("CARRYINREG", 0);
        if mode == Mode::Spartan6 {
            ti.cfg_int("CARRYOUTREG", 0);
        }
        ti.cfg("CARRYINSEL", "OPMODE5");
        ti.cfg("B_INPUT", "DIRECT");
        ti.cfg("RSTTYPE", "SYNC");
        ti.pin_tie_inv("RSTA", true, false);
        ti.pin_tie_inv("RSTB", true, false);
        ti.pin_tie_inv("RSTC", true, false);
        ti.pin_tie_inv("RSTD", true, false);
        ti.pin_tie_inv("RSTP", true, false);
        ti.pin_tie_inv("RSTCARRYIN", true, false);
        ti.pin_tie_inv("RSTOPMODE", true, false);
        ti.pin_tie_inv("CEA", false, false);
        ti.pin_tie_inv("CEB", false, false);
        ti.pin_tie_inv("CEC", false, false);
        ti.pin_tie_inv("CED", false, false);
        ti.pin_tie_inv("CEP", false, false);
        ti.pin_tie_inv("CECARRYIN", false, false);
        ti.pin_tie_inv("CEOPMODE", false, false);
        ti.pin_tie("OPMODE0", true);
        ti.pin_tie("OPMODE1", false);
        ti.pin_tie("OPMODE2", false);
        ti.pin_tie("OPMODE3", false);
        ti.pin_tie("OPMODE4", false);
        ti.pin_tie("OPMODE5", false);
        ti.pin_tie("OPMODE6", false);
        ti.pin_tie("OPMODE7", false);
        for i in 0..48 {
            ti.pin_tie(&format!("C{i}"), false);
        }
        for i in 0..18 {
            ti.pin_tie(&format!("D{i}"), false);
        }
    } else if mode == Mode::Virtex4 {
        ti.cfg_int("AREG", 0);
        ti.cfg_int("BREG", 0);
        ti.cfg_int("CREG", 0);
        ti.cfg_int("PREG", 0);
        ti.cfg_int("SUBTRACTREG", 0);
        ti.cfg_int("CARRYINREG", 0);
        ti.cfg_int("CARRYINSELREG", 0);
        ti.cfg_int("OPMODEREG", 0);
        ti.cfg("B_INPUT", "DIRECT");
        ti.pin_tie_inv("RSTA", true, false);
        ti.pin_tie_inv("RSTB", true, false);
        ti.pin_tie_inv("RSTP", true, false);
        ti.pin_tie_inv("RSTCARRYIN", true, false);
        ti.pin_tie_inv("RSTCTRL", true, false);
        ti.pin_tie_inv("CEA", false, false);
        ti.pin_tie_inv("CEB", false, false);
        ti.pin_tie_inv("CEP", false, false);
        ti.pin_tie_inv("CECARRYIN", false, false);
        ti.pin_tie_inv("CECINSUB", false, false);
        ti.pin_tie_inv("CECTRL", false, false);
        ti.pin_tie_inv("OPMODE0", true, false);
        ti.pin_tie_inv("OPMODE1", false, false);
        ti.pin_tie_inv("OPMODE2", true, false);
        ti.pin_tie_inv("OPMODE3", false, false);
        ti.pin_tie_inv("OPMODE4", false, false);
        ti.pin_tie_inv("OPMODE5", false, false);
        ti.pin_tie_inv("OPMODE6", false, false);
        ti.pin_tie_inv("SUBTRACT", false, false);
        ti.pin_tie_inv("CARRYIN", false, false);
        ti.pin_tie_inv("CARRYINSEL0", false, false);
        ti.pin_tie_inv("CARRYINSEL1", false, false);
    } else if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
        ti.cfg_int("AREG", 0);
        ti.cfg_int("BREG", 0);
        ti.cfg_int("ACASCREG", 0);
        ti.cfg_int("BCASCREG", 0);
        ti.cfg_int("CREG", 0);
        ti.cfg_int("PREG", 0);
        ti.cfg_int("CARRYINREG", 0);
        ti.cfg_int("CARRYINSELREG", 0);
        ti.cfg_int("OPMODEREG", 0);
        ti.cfg_int("ALUMODEREG", 0);
        ti.cfg("A_INPUT", "DIRECT");
        ti.cfg("B_INPUT", "DIRECT");
        if mode == Mode::Virtex5 {
            ti.cfg_int("MULTCARRYINREG", 0);
            ti.cfg("AUTORESET_OVER_UNDER_FLOW", "FALSE");
            ti.cfg("AUTORESET_PATTERN_DETECT", "FALSE");
            ti.cfg("AUTORESET_PATTERN_DETECT_OPTINV", "MATCH");
            ti.cfg("SEL_ROUNDING_MASK", "SEL_MASK");
            ti.cfg("ROUNDING_LSB_MASK", "0");
            ti.cfg("CLOCK_INVERT_M", "SAME_EDGE");
            ti.cfg("CLOCK_INVERT_P", "SAME_EDGE");
            ti.cfg("LFSR_EN_SET", "SET");
            ti.cfg("LFSR_EN_SETVAL", "0");
            ti.cfg("SCAN_IN_SETVAL_M", "0");
            ti.cfg("SCAN_IN_SETVAL_P", "0");
            ti.cfg("SCAN_IN_SET_M", "SET");
            ti.cfg("SCAN_IN_SET_P", "SET");
            ti.cfg("TEST_SETVAL_M", "0");
            ti.cfg("TEST_SETVAL_P", "0");
            ti.cfg("TEST_SET_M", "SET");
            ti.cfg("TEST_SET_P", "SET");
            ti.cfg("MASK", "3FFFFFFFFFFF");
        } else {
            ti.cfg_int("ADREG", 0);
            ti.cfg_int("DREG", 0);
            ti.cfg_int("INMODEREG", 0);
            ti.cfg("AUTORESET_PATDET", "NO_RESET");
            ti.cfg("USE_DPORT", "FALSE");
            ti.cfg("MASK", "3fffffffffff");
        }
        ti.cfg("SEL_MASK", "MASK");
        ti.cfg("SEL_PATTERN", "PATTERN");
        ti.cfg("USE_PATTERN_DETECT", "NO_PATDET");
        ti.cfg("USE_SIMD", "ONE48");
        ti.cfg("PATTERN", "000000000000");
        ti.pin_tie("RSTA", tieval_rst);
        ti.pin_tie("RSTB", tieval_rst);
        ti.pin_tie("RSTC", tieval_rst);
        ti.pin_tie("RSTP", tieval_rst);
        ti.pin_tie("RSTCTRL", tieval_rst);
        ti.pin_tie("RSTALUMODE", tieval_rst);
        ti.pin_tie("RSTALLCARRYIN", tieval_rst);
        ti.pin_tie("CEA1", tieval_ce);
        ti.pin_tie("CEA2", tieval_ce);
        ti.pin_tie("CEB1", tieval_ce);
        ti.pin_tie("CEB2", tieval_ce);
        ti.pin_tie("CEC", tieval_ce);
        ti.pin_tie("CEP", tieval_ce);
        ti.pin_tie("CECARRYIN", tieval_ce);
        ti.pin_tie("CECTRL", tieval_ce);
        ti.pin_tie("CEALUMODE", tieval_ce);
        if mode == Mode::Virtex5 {
            ti.pin_tie("CEMULTCARRYIN", tieval_ce);
        } else {
            ti.pin_tie("RSTD", tieval_rst);
            ti.pin_tie("RSTINMODE", tieval_rst);
            ti.pin_tie("CED", tieval_ce);
            ti.pin_tie("CEAD", tieval_ce);
            ti.pin_tie("CEINMODE", tieval_ce);
            ti.pin_tie_inv("INMODE0", false, false);
            ti.pin_tie_inv("INMODE1", false, false);
            ti.pin_tie_inv("INMODE2", false, false);
            ti.pin_tie_inv("INMODE3", false, false);
            ti.pin_tie_inv("INMODE4", false, false);
        }
        ti.pin_tie_inv("OPMODE0", true, false);
        ti.pin_tie_inv("OPMODE1", false, false);
        ti.pin_tie_inv("OPMODE2", true, false);
        ti.pin_tie_inv("OPMODE3", false, false);
        ti.pin_tie_inv("OPMODE4", false, false);
        ti.pin_tie_inv("OPMODE5", false, false);
        ti.pin_tie_inv("OPMODE6", false, false);
        ti.pin_tie_inv("ALUMODE0", false, false);
        ti.pin_tie_inv("ALUMODE1", false, false);
        ti.pin_tie_inv("ALUMODE2", false, false);
        ti.pin_tie_inv("ALUMODE3", false, false);
        ti.pin_tie_inv("CARRYIN", false, false);
        ti.pin_tie("CARRYINSEL0", false);
        ti.pin_tie("CARRYINSEL1", false);
        ti.pin_tie("CARRYINSEL2", false);
        if mode == Mode::Virtex5 {
            for i in 0..48 {
                ti.pin_tie(&format!("C{i}"), false);
            }
            for i in 18..30 {
                ti.pin_in(&format!("A{i}"), &a[17]);
            }
        } else {
            for i in 0..25 {
                ti.pin_tie(&format!("D{i}"), false);
            }
            for i in 18..25 {
                ti.pin_in(&format!("A{i}"), &a[17]);
            }
            if mode == Mode::Series7 {
                for i in 25..30 {
                    ti.pin_tie(&format!("A{i}"), true);
                }
                for i in 0..48 {
                    ti.pin_tie(&format!("C{i}"), true);
                }
            }
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_mult18x18sio(test: &mut Test, ctx: &mut TestGenCtx, num: usize) {
    let mut bcin: Option<Vec<String>> = None;
    for midx in 0..num {
        let mut inst = SrcInst::new(ctx, "MULT18X18SIO");
        let mut ti = TgtInst::new(&["MULT18X18SIO"]);

        let a = test.make_ins(ctx, 18);
        let b = test.make_ins(ctx, 18);
        let p = test.make_outs(ctx, 36);
        inst.connect_bus("A", &a);
        inst.connect_bus("B", &b);
        inst.connect_bus("P", &p);
        ti.bel("MULT18X18SIO", &inst.name, "");
        for i in 0..18 {
            ti.pin_in(&format!("A{i}"), &a[i]);
            ti.pin_in(&format!("B{i}"), &b[i]);
        }
        for i in 0..36 {
            ti.pin_out(&format!("P{i}"), &p[i]);
        }

        let b_input;
        if let Some(bcin) = bcin {
            inst.connect_bus("BCIN", &bcin);
            for i in 0..18 {
                ti.pin_in(&format!("BCIN{i}"), &bcin[i]);
            }
            b_input = *["DIRECT", "CASCADE"].choose(&mut ctx.rng).unwrap();
        } else {
            b_input = "DIRECT";
        }
        if midx != num - 1 {
            let bcout = test.make_bus(ctx, 18);
            inst.connect_bus("BCOUT", &bcout);
            for i in 0..18 {
                ti.pin_out(&format!("BCOUT{i}"), &bcout[i]);
            }
            bcin = Some(bcout);
        } else {
            bcin = None;
        }
        let areg = ctx.rng.gen_range(0..2);
        let breg = ctx.rng.gen_range(0..2);
        let preg = ctx.rng.gen_range(0..2);
        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        let (cea_v, cea_x, cea_inv) = test.make_in_inv(ctx);
        let (ceb_v, ceb_x, ceb_inv) = test.make_in_inv(ctx);
        let (cep_v, cep_x, cep_inv) = test.make_in_inv(ctx);
        let (rsta_v, rsta_x, rsta_inv) = test.make_in_inv(ctx);
        let (rstb_v, rstb_x, rstb_inv) = test.make_in_inv(ctx);
        let (rstp_v, rstp_x, rstp_inv) = test.make_in_inv(ctx);
        inst.connect("CLK", &clk_v);
        inst.connect("CEA", &cea_v);
        inst.connect("CEB", &ceb_v);
        inst.connect("CEP", &cep_v);
        inst.connect("RSTA", &rsta_v);
        inst.connect("RSTB", &rstb_v);
        inst.connect("RSTP", &rstp_v);
        inst.param_int("AREG", areg);
        inst.param_int("BREG", breg);
        inst.param_int("PREG", preg);
        inst.param_str("B_INPUT", b_input);
        ti.pin_in_inv("CLK", &clk_x, clk_inv);
        ti.pin_in_inv("CEA", &cea_x, cea_inv);
        ti.pin_in_inv("CEB", &ceb_x, ceb_inv);
        ti.pin_in_inv("CEP", &cep_x, cep_inv);
        ti.pin_in_inv("RSTA", &rsta_x, rsta_inv);
        ti.pin_in_inv("RSTB", &rstb_x, rstb_inv);
        ti.pin_in_inv("RSTP", &rstp_x, rstp_inv);
        ti.cfg_int("AREG", areg);
        ti.cfg_int("BREG", breg);
        ti.cfg_int("PREG", preg);
        ti.cfg("B_INPUT", b_input);
        ti.cfg("PREG_CLKINVERSION", "0");

        test.src_insts.push(inst);
        test.tgt_insts.push(ti);
    }
}

fn gen_dsp48a(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: u8, num: usize) {
    let mut bcin: Option<Vec<String>> = None;
    let mut pcin: Option<Vec<String>> = None;
    let mut carry: Option<String> = None;
    for midx in 0..num {
        let mut inst = SrcInst::new(ctx, if pk == 6 {"DSP48A1"} else {"DSP48A"});
        let hwprim = if mode == Mode::Spartan6 {"DSP48A1"} else {"DSP48A"};
        let mut ti = TgtInst::new(&[hwprim]);
        ti.bel(hwprim, &inst.name, "");

        let a = test.make_ins(ctx, 18);
        let c = test.make_ins(ctx, 48);
        let d = test.make_ins(ctx, 18);
        inst.connect_bus("A", &a);
        inst.connect_bus("C", &c);
        inst.connect_bus("D", &d);
        for i in 0..18 {
            ti.pin_in(&format!("A{i}"), &a[i]);
            ti.pin_in(&format!("D{i}"), &d[i]);
        }
        for i in 0..48 {
            ti.pin_in(&format!("C{i}"), &c[i]);
        }
        let opmode = test.make_ins(ctx, 8);
        inst.connect_bus("OPMODE", &opmode);
        for i in 0..8 {
            ti.pin_in(&format!("OPMODE{i}"), &opmode[i]);
        }
        // ...cannot both be used?
        if pk == 6 && ctx.rng.gen() && midx != num - 1 {
            let m = test.make_outs(ctx, 36);
            inst.connect_bus("M", &m);
            for i in 0..36 {
                ti.pin_out(&format!("M{i}"), &m[i]);
            }
        } else {
            let p = test.make_outs(ctx, 48);
            inst.connect_bus("P", &p);
            for i in 0..48 {
                ti.pin_out(&format!("P{i}"), &p[i]);
            }
        }
        if pk == 6 {
            let cof = test.make_out(ctx);
            inst.connect("CARRYOUTF", &cof);
            ti.pin_out("CARRYOUTF", &cof);
        }

        if let Some(pcin) = pcin.take() {
            inst.connect_bus("PCIN", &pcin);
            for i in 0..48 {
                ti.pin_in(&format!("PCIN{i}"), &pcin[i]);
            }
        }
        if let Some(carry) = carry.take() {
            inst.connect("CARRYIN", &carry);
            ti.pin_in("CARRYIN", &carry);
        }
        if let Some(bcin) = bcin.take() {
            inst.connect_bus("B", &bcin);
            for i in 0..18 {
                ti.pin_in(&format!("BCIN{i}"), &bcin[i]);
                ti.pin_tie(&format!("B{i}"), false);
            }
            ti.cfg("B_INPUT", "CASCADE");
        } else {
            let b = test.make_ins(ctx, 18);
            inst.connect_bus("B", &b);
            for i in 0..18 {
                ti.pin_in(&format!("B{i}"), &b[i]);
            }
            ti.cfg("B_INPUT", "DIRECT");
        }
        if midx != num - 1 {
            if ctx.rng.gen() {
                let bcout = test.make_bus(ctx, 18);
                inst.connect_bus("BCOUT", &bcout);
                for i in 0..18 {
                    ti.pin_out(&format!("BCOUT{i}"), &bcout[i]);
                }
                bcin = Some(bcout);
            }
            let pcout = test.make_bus(ctx, 48);
            inst.connect_bus("PCOUT", &pcout);
            for i in 0..48 {
                ti.pin_out(&format!("PCOUT{i}"), &pcout[i]);
            }
            pcin = Some(pcout);
            let carryout = test.make_wire(ctx);
            inst.connect("CARRYOUT", &carryout);
            ti.pin_out("CARRYOUT", &carryout);
            carry = Some(carryout);
        }

        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        inst.connect("CLK", &clk_v);
        ti.pin_in_inv("CLK", &clk_x, clk_inv);
        for pin in [
            "CEA", "CEB", "CEC", "CED", "CEM", "CEP", "CECARRYIN", "CEOPMODE",
            "RSTA", "RSTB", "RSTC", "RSTD", "RSTM", "RSTP", "RSTCARRYIN", "RSTOPMODE",
        ] {
            let (pin_v, pin_x, pin_inv) = test.make_in_inv(ctx);
            inst.connect(pin, &pin_v);
            ti.pin_in_inv(pin, &pin_x, pin_inv);
        }
        for p in [
            "A0REG", "A1REG", "B0REG", "B1REG", "CREG", "DREG", "MREG", "PREG", "OPMODEREG",
            "CARRYINREG"
        ] {
            let v = ctx.rng.gen_range(0..2);
            inst.param_int(p, v);
            ti.cfg_int(p, v);
        }
        if pk == 6 {
            let v = ctx.rng.gen_range(0..2);
            inst.param_int("CARRYOUTREG", v);
            ti.cfg_int("CARRYOUTREG", v);
        } else if mode == Mode::Spartan6 {
            ti.cfg_int("CARRYOUTREG", 0);
        }
        let v = if ctx.rng.gen() {"SYNC"} else {"ASYNC"};
        inst.param_str("RSTTYPE", v);
        ti.cfg("RSTTYPE", v);
        let v = if ctx.rng.gen() {"CARRYIN"} else {"OPMODE5"};
        inst.param_str("CARRYINSEL", v);
        ti.cfg("CARRYINSEL", v);

        test.src_insts.push(inst);
        test.tgt_insts.push(ti);
    }
}

fn gen_dsp48(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, pk: u8, num: usize) {
    let mut acin: Option<Vec<String>> = None;
    let mut bcin: Option<Vec<String>> = None;
    let mut pcin: Option<Vec<String>> = None;
    let mut carry: Option<(String, String)> = None;
    let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
    let tieval_ce = mode == Mode::Virtex6;
    let tieval_rst = !matches!(mode, Mode::Virtex6 | Mode::Series7);
    for midx in 0..num {
        let prim = match pk {
            4 => "DSP48",
            5 => "DSP48E",
            6 => "DSP48E1",
            _ => unreachable!(),
        };
        let hwprim = match mode {
            Mode::Virtex4 => "DSP48",
            Mode::Virtex5 => "DSP48E",
            Mode::Virtex6 | Mode::Series7 => "DSP48E1",
            _ => unreachable!(),
        };
        let mut inst = SrcInst::new(ctx, prim);
        let mut ti = TgtInst::new(&[hwprim]);
        if pk == 5 && mode != Mode::Virtex5 {
            ti.bel(hwprim, &format!("{}/DSP48E1", inst.name), "");
        } else {
            ti.bel(hwprim, &inst.name, "");
        }

        // clock
        inst.connect("CLK", &clk_v);
        ti.pin_in_inv("CLK", &clk_x, clk_inv);

        // These have a weird interdependency.
        let areg = ctx.rng.gen_range(0..3);
        inst.param_int("AREG", areg);
        ti.cfg_int("AREG", areg);
        let breg = ctx.rng.gen_range(0..3);
        inst.param_int("BREG", breg);
        ti.cfg_int("BREG", breg);

        // A path
        if pk == 4 {
            let a = test.make_ins(ctx, 18);
            inst.connect_bus("A", &a);
            for i in 0..18 {
                ti.pin_in(&format!("A{i}"), &a[i]);
            }
            if mode != Mode::Virtex4 {
                for i in 18..30 {
                    ti.pin_in(&format!("A{i}"), &a[17]);
                }
                ti.cfg("A_INPUT", "DIRECT");
            }
        } else {
            let a = test.make_ins(ctx, 30);
            inst.connect_bus("A", &a);
            for i in 0..30 {
                ti.pin_in(&format!("A{i}"), &a[i]);
            }
            let a_input;
            if let Some(acin) = acin.take() {
                inst.connect_bus("ACIN", &acin);
                for i in 0..30 {
                    ti.pin_in(&format!("ACIN{i}"), &acin[i]);
                }
                if ctx.rng.gen() {
                    a_input = "CASCADE";
                } else {
                    a_input = "DIRECT";
                }
            } else {
                a_input = "DIRECT";
            }
            inst.param_str("A_INPUT", a_input);
            ti.cfg("A_INPUT", a_input);
            if midx != num - 1 {
                let acout = test.make_bus(ctx, 30);
                inst.connect_bus("ACOUT", &acout);
                for i in 0..30 {
                    ti.pin_out(&format!("ACOUT{i}"), &acout[i]);
                }
                acin = Some(acout);
            }
        }
        if mode != Mode::Virtex4 {
            if pk == 4 {
                ti.cfg_int("ACASCREG", areg);
            } else {
                let acascreg = if areg == 2 {
                    ctx.rng.gen_range(1..3)
                } else {
                    areg
                };
                inst.param_int("ACASCREG", acascreg);
                ti.cfg_int("ACASCREG", acascreg);
            }
        }
        if mode == Mode::Virtex4 {
            let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
            inst.connect("RSTA", &rst_v);
            ti.pin_in_inv("RSTA", &rst_x, rst_inv);
            let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
            inst.connect("CEA", &ce_v);
            ti.pin_in_inv("CEA", &ce_x, ce_inv);
        } else {
            let rst = test.make_in(ctx);
            inst.connect("RSTA", &rst);
            ti.pin_in("RSTA", &rst);
            if pk == 4 {
                let ce = test.make_in(ctx);
                inst.connect("CEA", &ce);
                if areg == 0 && breg == 0 && mode == Mode::Virtex5 {
                    ti.pin_tie("CEA1", false);
                    ti.pin_tie("CEA2", false);
                } else {
                    ti.pin_in("CEA1", &ce);
                    ti.pin_in("CEA2", &ce);
                }
            } else {
                let ce1 = test.make_in(ctx);
                inst.connect("CEA1", &ce1);
                let ce2 = test.make_in(ctx);
                inst.connect("CEA2", &ce2);
                if areg == 0 && breg == 0 && mode == Mode::Virtex5 {
                    ti.pin_tie("CEA1", false);
                    ti.pin_tie("CEA2", false);
                } else {
                    ti.pin_in("CEA1", &ce1);
                    ti.pin_in("CEA2", &ce2);
                }
            }
        }

        // B path
        let b = test.make_ins(ctx, 18);
        inst.connect_bus("B", &b);
        for i in 0..18 {
            ti.pin_in(&format!("B{i}"), &b[i]);
        }
        let b_input;
        if let Some(bcin) = bcin.take() {
            inst.connect_bus("BCIN", &bcin);
            for i in 0..18 {
                ti.pin_in(&format!("BCIN{i}"), &bcin[i]);
            }
            if ctx.rng.gen() {
                b_input = "CASCADE";
            } else {
                b_input = "DIRECT";
            }
        } else {
            b_input = "DIRECT";
        }
        inst.param_str("B_INPUT", b_input);
        ti.cfg("B_INPUT", b_input);
        if midx != num - 1 {
            let bcout = test.make_bus(ctx, 18);
            inst.connect_bus("BCOUT", &bcout);
            for i in 0..18 {
                ti.pin_out(&format!("BCOUT{i}"), &bcout[i]);
            }
            bcin = Some(bcout);
        }
        if mode != Mode::Virtex4 {
            if pk == 4 {
                ti.cfg_int("BCASCREG", breg);
            } else {
                let bcascreg = if breg == 2 {
                    ctx.rng.gen_range(1..3)
                } else {
                    breg
                };
                inst.param_int("BCASCREG", bcascreg);
                ti.cfg_int("BCASCREG", bcascreg);
            }
        }
        if mode == Mode::Virtex4 {
            let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
            inst.connect("RSTB", &rst_v);
            ti.pin_in_inv("RSTB", &rst_x, rst_inv);
            let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
            inst.connect("CEB", &ce_v);
            ti.pin_in_inv("CEB", &ce_x, ce_inv);
        } else {
            let rst = test.make_in(ctx);
            inst.connect("RSTB", &rst);
            ti.pin_in("RSTB", &rst);
            if pk == 4 {
                let ce = test.make_in(ctx);
                inst.connect("CEB", &ce);
                if areg == 0 && breg == 0 && mode == Mode::Virtex5 {
                    ti.pin_tie("CEB1", false);
                    ti.pin_tie("CEB2", false);
                } else {
                    ti.pin_in("CEB1", &ce);
                    ti.pin_in("CEB2", &ce);
                }
            } else {
                let ce1 = test.make_in(ctx);
                inst.connect("CEB1", &ce1);
                let ce2 = test.make_in(ctx);
                inst.connect("CEB2", &ce2);
                if areg == 0 && breg == 0 && mode == Mode::Virtex5 {
                    ti.pin_tie("CEB1", false);
                    ti.pin_tie("CEB2", false);
                } else {
                    ti.pin_in("CEB1", &ce1);
                    ti.pin_in("CEB2", &ce2);
                }
            }
        }

        // C path
        let c = test.make_ins(ctx, 48);
        inst.connect_bus("C", &c);
        for i in 0..48 {
            ti.pin_in(&format!("C{i}"), &c[i]);
        }
        let creg = ctx.rng.gen_range(0..2);
        inst.param_int("CREG", creg);
        ti.cfg_int("CREG", creg);
        if mode == Mode::Virtex4 {
            let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
            inst.connect("RSTC", &rst_v);
            ti.pin_in_inv("RSTC", &rst_x, rst_inv);
            let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
            inst.connect("CEC", &ce_v);
            ti.pin_in_inv("CEC", &ce_x, ce_inv);
        } else {
            let rst = test.make_in(ctx);
            inst.connect("RSTC", &rst);
            ti.pin_in("RSTC", &rst);
            let ce = test.make_in(ctx);
            inst.connect("CEC", &ce);
            ti.pin_in("CEC", &ce);
        }

        // D path
        if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            if pk == 6 {
                let d = test.make_ins(ctx, 25);
                inst.connect_bus("D", &d);
                for i in 0..25 {
                    ti.pin_in(&format!("D{i}"), &d[i]);
                }
                let dreg = ctx.rng.gen_range(0..2);
                inst.param_int("DREG", dreg);
                ti.cfg_int("DREG", dreg);
                let adreg = ctx.rng.gen_range(0..2);
                inst.param_int("ADREG", adreg);
                ti.cfg_int("ADREG", adreg);
                let rst = test.make_in(ctx);
                inst.connect("RSTD", &rst);
                ti.pin_in("RSTD", &rst);
                let ce = test.make_in(ctx);
                inst.connect("CED", &ce);
                ti.pin_in("CED", &ce);
                let ce = test.make_in(ctx);
                inst.connect("CEAD", &ce);
                ti.pin_in("CEAD", &ce);

                let mut inmode = Vec::new();
                for i in 0..5 {
                    let (om_v, om_x, om_inv) = test.make_in_inv(ctx);
                    ti.pin_in_inv(&format!("INMODE{i}"), &om_x, om_inv);
                    inmode.push(om_v);
                }
                inst.connect_bus("INMODE", &inmode);
                let inmodereg = ctx.rng.gen_range(0..2);
                inst.param_int("INMODEREG", inmodereg);
                ti.cfg_int("INMODEREG", inmodereg);
                let rst = test.make_in(ctx);
                inst.connect("RSTINMODE", &rst);
                ti.pin_in("RSTINMODE", &rst);
                let ce = test.make_in(ctx);
                inst.connect("CEINMODE", &ce);
                ti.pin_in("CEINMODE", &ce);

                let use_d = if ctx.rng.gen() {"TRUE"} else {"FALSE"};
                inst.param_str("USE_DPORT", use_d);
                ti.cfg("USE_DPORT", use_d);
            } else {
                for i in 0..25 {
                    ti.pin_tie(&format!("D{i}"), false);
                }
                ti.pin_tie("RSTD", tieval_rst);
                ti.pin_tie("RSTINMODE", tieval_rst);
                ti.pin_tie("CED", tieval_ce);
                ti.pin_tie("CEAD", tieval_ce);
                ti.pin_tie("CEINMODE", tieval_ce);
                ti.pin_tie_inv("INMODE0", false, false);
                ti.pin_tie_inv("INMODE1", false, false);
                ti.pin_tie_inv("INMODE2", false, false);
                ti.pin_tie_inv("INMODE3", false, false);
                ti.pin_tie_inv("INMODE4", false, false);
                ti.cfg("USE_DPORT", "FALSE");
                ti.cfg_int("DREG", 0);
                ti.cfg_int("ADREG", 0);
                ti.cfg_int("INMODEREG", 0);
            }
        }

        // M
        let mreg = ctx.rng.gen_range(0..2);
        inst.param_int("MREG", mreg);
        ti.cfg_int("MREG", mreg);
        if mode == Mode::Virtex4 {
            let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
            inst.connect("RSTM", &rst_v);
            ti.pin_in_inv("RSTM", &rst_x, rst_inv);
            let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
            inst.connect("CEM", &ce_v);
            ti.pin_in_inv("CEM", &ce_x, ce_inv);
        } else {
            let rst = test.make_in(ctx);
            inst.connect("RSTM", &rst);
            ti.pin_in("RSTM", &rst);
            let ce = test.make_in(ctx);
            inst.connect("CEM", &ce);
            ti.pin_in("CEM", &ce);
        }
        if pk == 6 {
            let use_mult = *["NONE", "MULTIPLY", "DYNAMIC"].choose(&mut ctx.rng).unwrap();
            inst.param_str("USE_MULT", use_mult);
            ti.cfg("USE_MULT", use_mult);
        } else {
            let use_mult = ctx.rng.gen();
            if pk == 4 {
                inst.param_str("LEGACY_MODE", if use_mult { if mreg == 1 {"MULT18X18S"} else {"MULT18X18"} } else {"NONE"});
            } else if pk == 5 {
                inst.param_str("USE_MULT", if use_mult { if mreg == 1 {"MULT_S"} else {"MULT"} } else {"NONE"});
            }
            if mode == Mode::Virtex4 {
                ti.cfg("LEGACY_MODE", if use_mult { if mreg == 1 {"MULT18X18S"} else {"MULT18X18"} } else {"NONE"});
            } else if mode == Mode::Virtex5 {
                ti.cfg("USE_MULT", if use_mult { if mreg == 1 {"MULT_S"} else {"MULT"} } else {"NONE"});
            } else {
                ti.cfg("USE_MULT", if use_mult {"MULTIPLY"} else {"NONE"});
            }
        }

        // OPMODE
        let mut opmode = Vec::new();
        for i in 0..7 {
            let (om_v, om_x, om_inv) = test.make_in_inv(ctx);
            ti.pin_in_inv(&format!("OPMODE{i}"), &om_x, om_inv);
            opmode.push(om_v);
        }
        inst.connect_bus("OPMODE", &opmode);
        let opmodereg = ctx.rng.gen_range(0..2);
        inst.param_int("OPMODEREG", opmodereg);
        ti.cfg_int("OPMODEREG", opmodereg);
        if mode == Mode::Virtex4 {
            let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
            inst.connect("RSTCTRL", &rst_v);
            ti.pin_in_inv("RSTCTRL", &rst_x, rst_inv);
            let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
            inst.connect("CECTRL", &ce_v);
            ti.pin_in_inv("CECTRL", &ce_x, ce_inv);
        } else {
            let rst = test.make_in(ctx);
            inst.connect("RSTCTRL", &rst);
            ti.pin_in("RSTCTRL", &rst);
            if pk == 4 {
                ti.pin_in("RSTALUMODE", &rst);
            }
            let ce = test.make_in(ctx);
            inst.connect("CECTRL", &ce);
            ti.pin_in("CECTRL", &ce);
        }

        // ALUMODE
        if pk == 4 {
            let (sub_v, sub_x, sub_inv) = test.make_in_inv(ctx);
            inst.connect("SUBTRACT", &sub_v);
            let subreg = ctx.rng.gen_range(0..2);
            inst.param_int("SUBTRACTREG", subreg);
            if mode == Mode::Virtex4 {
                ti.pin_in_inv("SUBTRACT", &sub_x, sub_inv);
                ti.cfg_int("SUBTRACTREG", subreg);
            } else {
                ti.pin_in_inv("ALUMODE0", &sub_x, sub_inv);
                ti.pin_in_inv("ALUMODE1", &sub_x, sub_inv);
                ti.pin_tie_inv("ALUMODE2", false, false);
                ti.pin_tie_inv("ALUMODE3", false, false);
                ti.cfg_int("ALUMODEREG", subreg);
            }
        } else {
            let mut alumode = Vec::new();
            for i in 0..4 {
                let (om_v, om_x, om_inv) = test.make_in_inv(ctx);
                ti.pin_in_inv(&format!("ALUMODE{i}"), &om_x, om_inv);
                alumode.push(om_v);
            }
            inst.connect_bus("ALUMODE", &alumode);
            let alumodereg = ctx.rng.gen_range(0..2);
            inst.param_int("ALUMODEREG", alumodereg);
            ti.cfg_int("ALUMODEREG", alumodereg);

            let rst = test.make_in(ctx);
            inst.connect("RSTALUMODE", &rst);
            ti.pin_in("RSTALUMODE", &rst);
            let ce = test.make_in(ctx);
            inst.connect("CEALUMODE", &ce);
            ti.pin_in("CEALUMODE", &ce);
        }

        // Carry select
        let (cin_v, cin_x, cin_inv) = test.make_in_inv(ctx);
        inst.connect("CARRYIN", &cin_v);
        ti.pin_in_inv("CARRYIN", &cin_x, cin_inv);
        let carryinreg = ctx.rng.gen_range(0..2);
        inst.param_int("CARRYINREG", carryinreg);
        ti.cfg_int("CARRYINREG", carryinreg);
        if pk == 4 {
            if mode == Mode::Virtex4 {
                let mut cinsel = Vec::new();
                for i in 0..2 {
                    let (s_v, s_x, s_inv) = test.make_in_inv(ctx);
                    ti.pin_in_inv(&format!("CARRYINSEL{i}"), &s_x, s_inv);
                    cinsel.push(s_v);
                }
                inst.connect_bus("CARRYINSEL", &cinsel);
            } else {
                if ctx.rng.gen() {
                    let val = ctx.rng.gen_range(0..4);
                    inst.connect("CARRYINSEL", &format!("{}", val));
                    // well...
                    ti.pin_tie("CARRYINSEL0", (val & 1) != 0);
                    ti.pin_tie("CARRYINSEL1", (val & 2) != 0);
                    ti.pin_tie("CARRYINSEL2", false);
                } else {
                    let cinsel = test.make_ins(ctx, 2);
                    inst.connect_bus("CARRYINSEL", &cinsel);
                    for i in 0..2 {
                        ti.pin_in(&format!("CARRYINSEL{i}"), &cinsel[i]);
                    }
                    ti.pin_tie("CARRYINSEL2", false);
                }
            }
            if mode == Mode::Virtex5 {
                ti.cfg_int("MULTCARRYINREG", 1);
            }
            let carryinselreg = ctx.rng.gen_range(0..2);
            inst.param_int("CARRYINSELREG", carryinselreg);
            ti.cfg_int("CARRYINSELREG", carryinselreg);
            if mode == Mode::Virtex4 {
                let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
                inst.connect("RSTCARRYIN", &rst_v);
                ti.pin_in_inv("RSTCARRYIN", &rst_x, rst_inv);
                let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
                inst.connect("CECARRYIN", &ce_v);
                ti.pin_in_inv("CECARRYIN", &ce_x, ce_inv);
                let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
                inst.connect("CECINSUB", &ce_v);
                ti.pin_in_inv("CECINSUB", &ce_x, ce_inv);
            } else {
                let rst = test.make_in(ctx);
                inst.connect("RSTCARRYIN", &rst);
                ti.pin_in("RSTALLCARRYIN", &rst);
                let ce = test.make_in(ctx);
                inst.connect("CECARRYIN", &ce);
                if mode == Mode::Virtex5 {
                    ti.pin_in("CEMULTCARRYIN", &ce);
                }
                let ce = test.make_in(ctx);
                inst.connect("CECINSUB", &ce);
                ti.pin_in("CEALUMODE", &ce);
                ti.pin_in("CECARRYIN", &ce);
            }
        } else {
            let cinsel = test.make_ins(ctx, 3);
            inst.connect_bus("CARRYINSEL", &cinsel);
            for i in 0..3 {
                ti.pin_in(&format!("CARRYINSEL{i}"), &cinsel[i]);
            }
            let carryinselreg = ctx.rng.gen_range(0..2);
            inst.param_int("CARRYINSELREG", carryinselreg);
            ti.cfg_int("CARRYINSELREG", carryinselreg);
            let rst = test.make_in(ctx);
            inst.connect("RSTALLCARRYIN", &rst);
            ti.pin_in("RSTALLCARRYIN", &rst);
            let ce = test.make_in(ctx);
            inst.connect("CECARRYIN", &ce);
            ti.pin_in("CECARRYIN", &ce);
            if pk == 5 {
                let multcarryinreg = ctx.rng.gen_range(0..2);
                inst.param_int("MULTCARRYINREG", multcarryinreg);
                if mode == Mode::Virtex5 {
                    ti.cfg_int("MULTCARRYINREG", multcarryinreg);
                }
                let ce = test.make_in(ctx);
                inst.connect("CEMULTCARRYIN", &ce);
                if mode == Mode::Virtex5 {
                    ti.pin_in("CEMULTCARRYIN", &ce);
                }
            }
        }
        if let Some((co, mso)) = carry.take() {
            inst.connect("CARRYCASCIN", &co);
            ti.pin_in("CARRYCASCIN", &co);
            inst.connect("MULTSIGNIN", &mso);
            ti.pin_in("MULTSIGNIN", &mso);
        }

        // P out
        let preg = ctx.rng.gen_range(0..2);
        inst.param_int("PREG", preg);
        ti.cfg_int("PREG", preg);
        if mode == Mode::Virtex4 {
            let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
            inst.connect("RSTP", &rst_v);
            ti.pin_in_inv("RSTP", &rst_x, rst_inv);
            let (ce_v, ce_x, ce_inv) = test.make_in_inv(ctx);
            inst.connect("CEP", &ce_v);
            ti.pin_in_inv("CEP", &ce_x, ce_inv);
        } else {
            let rst = test.make_in(ctx);
            inst.connect("RSTP", &rst);
            ti.pin_in("RSTP", &rst);
            let ce = test.make_in(ctx);
            inst.connect("CEP", &ce);
            ti.pin_in("CEP", &ce);
        }
        let p = test.make_outs(ctx, 48);
        inst.connect_bus("P", &p);
        for i in 0..48 {
            ti.pin_out(&format!("P{i}"), &p[i]);
        }
        if let Some(pcin) = pcin.take() {
            inst.connect_bus("PCIN", &pcin);
            for i in 0..48 {
                ti.pin_in(&format!("PCIN{i}"), &pcin[i]);
            }
        }
        if midx != num - 1 {
            let pcout = test.make_bus(ctx, 48);
            inst.connect_bus("PCOUT", &pcout);
            for i in 0..48 {
                ti.pin_out(&format!("PCOUT{i}"), &pcout[i]);
            }
            pcin = Some(pcout);
            if pk != 4 {
                let co = test.make_wire(ctx);
                let mso = test.make_wire(ctx);
                inst.connect("CARRYCASCOUT", &co);
                ti.pin_out("CARRYCASCOUT", &co);
                inst.connect("MULTSIGNOUT", &mso);
                ti.pin_out("MULTSIGNOUT", &mso);
                carry = Some((co, mso));
            }
        }

        if mode != Mode::Virtex4 {
            if pk == 4 {
                ti.cfg("USE_SIMD", "ONE48");
                ti.cfg("SEL_MASK", "MASK");
                ti.cfg("SEL_PATTERN", "PATTERN");
                ti.cfg("USE_PATTERN_DETECT", "NO_PATDET");
                ti.cfg("PATTERN", "000000000000");
                if mode == Mode::Virtex5 {
                    ti.cfg("AUTORESET_PATTERN_DETECT", "FALSE");
                    ti.cfg("AUTORESET_PATTERN_DETECT_OPTINV", "MATCH");
                    ti.cfg("SEL_ROUNDING_MASK", "SEL_MASK");
                    ti.cfg("MASK", "3FFFFFFFFFFF");
                } else {
                    ti.cfg("AUTORESET_PATDET", "NO_RESET");
                    ti.cfg("MASK", "3fffffffffff");
                }
            } else {
                let use_simd = *["ONE48", "TWO24", "FOUR12"].choose(&mut ctx.rng).unwrap();
                inst.param_str("USE_SIMD", use_simd);
                ti.cfg("USE_SIMD", use_simd);

                let co = test.make_outs(ctx, 4);
                inst.connect_bus("CARRYOUT", &co);
                for i in 0..4 {
                    ti.pin_out(&format!("CARRYOUT{i}"), &co[i]);
                }

                for p in [
                    "UNDERFLOW",
                    "OVERFLOW",
                    "PATTERNDETECT",
                    "PATTERNBDETECT",
                ] {
                    let w = test.make_out(ctx);
                    inst.connect(p, &w);
                    ti.pin_out(p, &w);
                }

                let use_pat = if ctx.rng.gen() {"PATDET"} else {"NO_PATDET"};
                inst.param_str("USE_PATTERN_DETECT", use_pat);
                ti.cfg("USE_PATTERN_DETECT", use_pat);

                let mask = ctx.gen_bits(48);
                inst.param_bits("MASK", &mask);
                ti.cfg_hex("MASK", &mask, true);
                let pattern = ctx.gen_bits(48);
                inst.param_bits("PATTERN", &pattern);
                ti.cfg_hex("PATTERN", &pattern, true);

                let sel_pattern = *["C", "PATTERN"].choose(&mut ctx.rng).unwrap();
                inst.param_str("SEL_PATTERN", sel_pattern);
                ti.cfg("SEL_PATTERN", sel_pattern);
                if pk == 5 {
                    let sel_mask = *["C", "MASK"].choose(&mut ctx.rng).unwrap();
                    inst.param_str("SEL_MASK", sel_mask);
                    let rm = ctx.rng.gen_range(0..3);
                    let srm = match rm {
                        0 => "SEL_MASK",
                        1 => "MODE1",
                        2 => "MODE2",
                        _ => unreachable!(),
                    };
                    inst.param_str("SEL_ROUNDING_MASK", srm);
                    if mode == Mode::Virtex5 {
                        ti.cfg("SEL_MASK", sel_mask);
                        ti.cfg("SEL_ROUNDING_MASK", srm);
                    } else {
                        ti.cfg("SEL_MASK", match rm {
                            0 => sel_mask,
                            1 => "ROUNDING_MODE1",
                            2 => "ROUNDING_MODE2",
                            _ => unreachable!(),
                        });
                    }
                } else {
                    let sel_mask = *["C", "MASK", "ROUNDING_MODE1", "ROUNDING_MODE2"].choose(&mut ctx.rng).unwrap();
                    inst.param_str("SEL_MASK", sel_mask);
                    ti.cfg("SEL_MASK", sel_mask);
                }

                if pk == 5 {
                    let arpd = if ctx.rng.gen() {"TRUE"} else {"FALSE"};
                    let arpdi = if ctx.rng.gen() {"NOT_MATCH"} else {"MATCH"};
                    inst.param_str("AUTORESET_PATTERN_DETECT", arpd);
                    inst.param_str("AUTORESET_PATTERN_DETECT_OPTINV", arpdi);
                    if mode == Mode::Virtex5 {
                        ti.cfg("AUTORESET_PATTERN_DETECT", arpd);
                        ti.cfg("AUTORESET_PATTERN_DETECT_OPTINV", arpdi);
                    } else {
                        ti.cfg("AUTORESET_PATDET", match (arpd, arpdi) {
                            ("FALSE", _) => "NO_RESET",
                            ("TRUE", "MATCH") => "RESET_MATCH",
                            ("TRUE", "NOT_MATCH") => "RESET_NOT_MATCH",
                            _ => unreachable!(),
                        });
                    }
                } else {
                    let arp = *["RESET_MATCH", "RESET_NOT_MATCH", "NO_RESET"].choose(&mut ctx.rng).unwrap();
                    inst.param_str("AUTORESET_PATDET", arp);
                    ti.cfg("AUTORESET_PATDET", arp);
                }
            }
        }

        if mode == Mode::Virtex5 {
            ti.cfg("AUTORESET_OVER_UNDER_FLOW", "FALSE");
            ti.cfg("ROUNDING_LSB_MASK", "0");
            ti.cfg("CLOCK_INVERT_M", "SAME_EDGE");
            ti.cfg("CLOCK_INVERT_P", "SAME_EDGE");
            ti.cfg("LFSR_EN_SET", "SET");
            ti.cfg("LFSR_EN_SETVAL", "0");
            ti.cfg("SCAN_IN_SETVAL_M", "0");
            ti.cfg("SCAN_IN_SETVAL_P", "0");
            ti.cfg("SCAN_IN_SET_M", "SET");
            ti.cfg("SCAN_IN_SET_P", "SET");
            ti.cfg("TEST_SETVAL_M", "0");
            ti.cfg("TEST_SETVAL_P", "0");
            ti.cfg("TEST_SET_M", "SET");
            ti.cfg("TEST_SET_P", "SET");
        }

        test.src_insts.push(inst);
        test.tgt_insts.push(ti);
    }
}

pub fn gen_dsp(ctx: &mut TestGenCtx, mode: Mode, test: &mut Test) {
    for num in 1..4 {
        gen_mult18x18(test, ctx, mode);
        if mode == Mode::Spartan3E {
            // This *has* a mapping on S3ADSP and S6, but it's broken and unable to synth.
            gen_mult18x18sio(test, ctx, num);
        }
        if matches!(mode, Mode::Spartan3ADsp | Mode::Spartan6) {
            gen_dsp48a(test, ctx, mode, 3, num);
        }
        if mode == Mode::Spartan6 {
            gen_dsp48a(test, ctx, mode, 6, num);
        }
        if matches!(mode, Mode::Virtex4 | Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
            gen_dsp48(test, ctx, mode, 4, num);
        }
        if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
            gen_dsp48(test, ctx, mode, 5, num);
        }
        if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            gen_dsp48(test, ctx, mode, 6, num);
        }
    }
}
