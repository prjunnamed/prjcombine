use crate::types::{Test, SrcInst, TgtInst, TestGenCtx, BitVal};

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
            ti.pin_tie("RSTM", true);
        } else if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            ti.cfg("USE_MULT", "MULTIPLY");
            ti.cfg_int("MREG", 0);
            ti.pin_tie_inv("CLK", false, false);
            if mode == Mode::Virtex6 {
                ti.pin_tie("CEM", true);
            } else {
                ti.pin_tie("CEM", false);
            }
            ti.pin_tie("RSTM", false);
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
        if mode == Mode::Virtex5 {
            ti.pin_tie("RSTA", true);
            ti.pin_tie("RSTB", true);
            ti.pin_tie("RSTC", true);
            ti.pin_tie("RSTP", true);
            ti.pin_tie("RSTCTRL", true);
            ti.pin_tie("RSTALUMODE", true);
            ti.pin_tie("RSTALLCARRYIN", true);
            ti.pin_tie("CEA1", false);
            ti.pin_tie("CEA2", false);
            ti.pin_tie("CEB1", false);
            ti.pin_tie("CEB2", false);
            ti.pin_tie("CEC", false);
            ti.pin_tie("CEP", false);
            ti.pin_tie("CECARRYIN", false);
            ti.pin_tie("CECTRL", false);
            ti.pin_tie("CEALUMODE", false);
            ti.pin_tie("CEMULTCARRYIN", false);
        } else {
            ti.pin_tie("RSTA", false);
            ti.pin_tie("RSTB", false);
            ti.pin_tie("RSTC", false);
            ti.pin_tie("RSTP", false);
            ti.pin_tie("RSTCTRL", false);
            ti.pin_tie("RSTALUMODE", false);
            ti.pin_tie("RSTALLCARRYIN", false);
            ti.pin_tie("RSTD", false);
            ti.pin_tie("RSTINMODE", false);
            if mode == Mode::Virtex6 {
                ti.pin_tie("CEA1", true);
                ti.pin_tie("CEA2", true);
                ti.pin_tie("CEB1", true);
                ti.pin_tie("CEB2", true);
                ti.pin_tie("CEC", true);
                ti.pin_tie("CEP", true);
                ti.pin_tie("CECARRYIN", true);
                ti.pin_tie("CECTRL", true);
                ti.pin_tie("CEALUMODE", true);
                ti.pin_tie("CED", true);
                ti.pin_tie("CEAD", true);
                ti.pin_tie("CEINMODE", true);
            } else {
                ti.pin_tie("CEA1", false);
                ti.pin_tie("CEA2", false);
                ti.pin_tie("CEB1", false);
                ti.pin_tie("CEB2", false);
                ti.pin_tie("CEC", false);
                ti.pin_tie("CEP", false);
                ti.pin_tie("CECARRYIN", false);
                ti.pin_tie("CECTRL", false);
                ti.pin_tie("CEALUMODE", false);
                ti.pin_tie("CED", false);
                ti.pin_tie("CEAD", false);
                ti.pin_tie("CEINMODE", false);
            }
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

fn gen_mult18x18sio(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
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
            if mode == Mode::Spartan6 {
                ti.pin_out(&format!("M{i}"), &p[i]);
            } else {
                ti.pin_out(&format!("P{i}"), &p[i]);
            }
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


pub fn gen_dsp(ctx: &mut TestGenCtx, mode: Mode, test: &mut Test) {
    for num in 1..5 {
        gen_mult18x18(test, ctx, mode);
        if mode == Mode::Spartan3E {
            // This *has* a mapping on S3ADSP and S6, but it's broken and unable to synth.
            gen_mult18x18sio(test, ctx, mode, num);
        }
    }
}
