use crate::types::{BitVal, SrcInst, Test, TestGenCtx, TgtInst};

use rand::{seq::SliceRandom, Rng};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Mode {
    Virtex,
    Virtex2,
    Spartan3A,
    Spartan3ADsp,
    Spartan6,
    Virtex4,
    Virtex5,
    Virtex6,
    Series7,
}

const ZERO_INIT: &str = "0000000000000000000000000000000000000000000000000000000000000000";

const PORT_ATTR_V: &[&str] = &["4096X1", "2048X2", "1024X4", "512X8", "256X16"];

const PORT_ATTR_V2: &[&str] = &["16384X1", "8192X2", "4096X4", "2048X9", "1024X18", "512X36"];

const WIDTHS: &[i32] = &[1, 2, 4, 9, 18, 36, 72];

fn init_lowercase(mode: Mode) -> bool {
    matches!(mode, Mode::Virtex | Mode::Virtex2)
}

fn gen_ramb_v(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8, dp: bool, bwe: bool) {
    let mut awlog2;
    let mut bwlog2;
    if sz == 4 {
        awlog2 = ctx.rng.gen_range(0..5);
        bwlog2 = ctx.rng.gen_range(0..5);
    } else {
        if bwe {
            awlog2 = ctx.rng.gen_range(4..6);
            bwlog2 = ctx.rng.gen_range(3..6);
        } else {
            awlog2 = ctx.rng.gen_range(0..6);
            bwlog2 = ctx.rng.gen_range(0..6);
        }
    }
    if dp {
        if bwe {
            if awlog2 < bwlog2 {
                std::mem::swap(&mut awlog2, &mut bwlog2);
            }
        } else {
            if bwlog2 < awlog2 {
                std::mem::swap(&mut awlog2, &mut bwlog2);
            }
        }
    }
    if !dp {
        bwlog2 = 0;
    }
    let aw = 1 << awlog2;
    let bw = 1 << bwlog2;
    let mut awp = aw;
    let mut bwp = bw;
    if sz == 16 && aw >= 8 {
        awp = aw / 8 * 9;
    }
    if sz == 16 && bw >= 8 {
        bwp = bw / 8 * 9;
    }
    let prim = if sz == 4 {
        if dp {
            format!("RAMB4_S{aw}_S{bw}")
        } else {
            format!("RAMB4_S{aw}")
        }
    } else {
        if bwe {
            if dp {
                format!("RAMB16BWE_S{awp}_S{bwp}")
            } else {
                format!("RAMB16BWE_S{awp}")
            }
        } else {
            if dp {
                format!("RAMB16_S{awp}_S{bwp}")
            } else {
                format!("RAMB16_S{awp}")
            }
        }
    };
    let mut inst = SrcInst::new(ctx, &prim);
    let ul = if ctx.rng.gen() { "U" } else { "L" };
    let is_36 = awlog2 == 5 || bwlog2 == 5;
    let uls = if is_36 {
        vec!["L", "U"]
    } else if mode == Mode::Virtex5 {
        vec![ul]
    } else {
        vec![""]
    };

    let hwprim = match mode {
        Mode::Virtex => "BLOCKRAM",
        Mode::Virtex2 => "RAMB16",
        Mode::Spartan3A => "RAMB16BWE",
        Mode::Spartan3ADsp | Mode::Spartan6 => "RAMB16BWER",
        Mode::Virtex4 => "RAMB16",
        Mode::Virtex5 => {
            if is_36 {
                "RAMB36_EXP"
            } else {
                "RAMB18X2"
            }
        }
        Mode::Virtex6 | Mode::Series7 => {
            if is_36 {
                "RAMB36E1"
            } else {
                "RAMB18E1"
            }
        }
    };
    let mut ti = TgtInst::new(&[hwprim]);

    let mut wmode_a = "WRITE_FIRST";
    let mut wmode_b = "WRITE_FIRST";
    if sz == 16 {
        wmode_a = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]
            .choose(&mut ctx.rng)
            .unwrap();
        if dp {
            wmode_b = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]
                .choose(&mut ctx.rng)
                .unwrap();
            inst.param_str("WRITE_MODE_A", wmode_a);
            inst.param_str("WRITE_MODE_B", wmode_b);
        } else {
            inst.param_str("WRITE_MODE", wmode_a);
        }
    }

    if mode == Mode::Virtex {
        ti.bel("BLOCKRAM", &inst.name, "");
        ti.bel("BLOCKRAMA", &format!("{}.A", inst.name), "");
        ti.cfg("PORTA_ATTR", PORT_ATTR_V[awlog2]);
        if dp {
            ti.bel("BLOCKRAMB", &format!("{}.B", inst.name), "");
            ti.cfg("PORTB_ATTR", PORT_ATTR_V[bwlog2]);
        }
    } else if mode == Mode::Virtex2 {
        ti.bel("RAMB16", "DUMMY", "");
        ti.bel("RAMB16A", &format!("{}.A", inst.name), "");
        ti.cfg("PORTA_ATTR", PORT_ATTR_V2[awlog2]);
        ti.cfg("WRITEMODEA", wmode_a);
        if dp {
            ti.bel("RAMB16B", &format!("{}.B", inst.name), "");
            ti.cfg("PORTB_ATTR", PORT_ATTR_V2[bwlog2]);
            ti.cfg("WRITEMODEB", wmode_b);
        }
    } else if mode == Mode::Virtex4 {
        ti.bel("RAMB16", &inst.name, "");
        ti.cfg("EN_ECC_READ", "FALSE");
        ti.cfg("EN_ECC_WRITE", "FALSE");
        ti.cfg("INVERT_CLK_DOA_REG", "FALSE");
        ti.cfg("INVERT_CLK_DOB_REG", "FALSE");
        ti.cfg("RAM_EXTENSION_A", "NONE");
        ti.cfg("RAM_EXTENSION_B", "NONE");
        ti.cfg("SAVEDATA", "FALSE");
        ti.cfg_int("READ_WIDTH_A", WIDTHS[awlog2]);
        ti.cfg_int("WRITE_WIDTH_A", WIDTHS[awlog2]);
        if dp {
            ti.cfg_int("READ_WIDTH_B", WIDTHS[bwlog2]);
            ti.cfg_int("WRITE_WIDTH_B", WIDTHS[bwlog2]);
        } else {
            ti.cfg_int("READ_WIDTH_B", 0);
            ti.cfg_int("WRITE_WIDTH_B", 0);
        }
        ti.cfg("WRITE_MODE_A", wmode_a);
        ti.cfg("WRITE_MODE_B", wmode_b);
        ti.cfg_int("DOA_REG", 0);
        ti.cfg_int("DOB_REG", 0);
        ti.pin_tie_inv("REGCEA", false, false);
        if dp {
            ti.pin_tie_inv("REGCEB", false, false);
        }
    } else if mode == Mode::Virtex5 {
        if is_36 {
            ti.bel("RAMB36_EXP", &inst.name, "");
            ti.cfg("RAM_EXTENSION_A", "NONE");
            ti.cfg("RAM_EXTENSION_B", "NONE");
            ti.cfg("SAVEDATA", "FALSE");
            ti.cfg_int("READ_WIDTH_A", WIDTHS[awlog2]);
            ti.cfg_int("WRITE_WIDTH_A", WIDTHS[awlog2]);
            if dp {
                ti.cfg_int("READ_WIDTH_B", WIDTHS[bwlog2]);
                ti.cfg_int("WRITE_WIDTH_B", WIDTHS[bwlog2]);
            } else {
                ti.cfg_int("READ_WIDTH_B", 0);
                ti.cfg_int("WRITE_WIDTH_B", 0);
            }
            ti.cfg("WRITE_MODE_A", wmode_a);
            ti.cfg("WRITE_MODE_B", wmode_b);
            ti.cfg_int("DOA_REG", 0);
            ti.cfg_int("DOB_REG", 0);
        } else {
            if ul == "L" {
                ti.bel("RAMB18X2_LOWER", &inst.name, "");
                inst.attr_str("BEL", "LOWER");
            } else {
                ti.bel("RAMB18X2_UPPER", &inst.name, "");
                inst.attr_str("BEL", "UPPER");
            }
            ti.cfg(&format!("SAVEDATA_{ul}"), "FALSE");
            ti.cfg_int(&format!("READ_WIDTH_A_{ul}"), WIDTHS[awlog2]);
            ti.cfg_int(&format!("WRITE_WIDTH_A_{ul}"), WIDTHS[awlog2]);
            if dp {
                ti.cfg_int(&format!("READ_WIDTH_B_{ul}"), WIDTHS[bwlog2]);
                ti.cfg_int(&format!("WRITE_WIDTH_B_{ul}"), WIDTHS[bwlog2]);
            } else {
                ti.cfg_int(&format!("READ_WIDTH_B_{ul}"), 0);
                ti.cfg_int(&format!("WRITE_WIDTH_B_{ul}"), 0);
            }
            ti.cfg(&format!("WRITE_MODE_A_{ul}"), wmode_a);
            ti.cfg(&format!("WRITE_MODE_B_{ul}"), wmode_b);
            ti.cfg_int(&format!("DOA_REG_{ul}"), 0);
            ti.cfg_int(&format!("DOB_REG_{ul}"), 0);
        }
        for &ul in &uls {
            ti.pin_tie(&format!("REGCEA{ul}"), false);
            if dp {
                ti.pin_tie(&format!("REGCEB{ul}"), false);
            }
            if is_36 {
                ti.pin_tie_inv(&format!("REGCLKA{ul}"), true, true);
                ti.pin_tie_inv(&format!("REGCLKB{ul}"), true, true);
            }
            if !dp {
                ti.pin_tie_inv(&format!("CLKB{ul}"), true, true);
                ti.pin_tie_inv(&format!("ENB{ul}"), true, true);
                ti.pin_tie_inv(&format!("SSRB{ul}"), true, true);
                if !is_36 {
                    ti.pin_tie_inv(&format!("REGCLKB{ul}"), true, true);
                }
                for i in 0..8 {
                    ti.pin_tie(&format!("WEB{ul}{i}"), false);
                }
                if is_36 {
                    if ul == "L" {
                        ti.pin_tie("ADDRBL14", false);
                        ti.pin_tie("ADDRBL15", true);
                    } else {
                        ti.pin_tie("ADDRBU14", false);
                    }
                } else if ul == "L" {
                    ti.pin_tie("ADDRBL15", true);
                }
            } else {
                for i in 4..8 {
                    ti.pin_tie(&format!("WEB{ul}{i}"), false);
                }
            }
        }
    } else if matches!(mode, Mode::Virtex6 | Mode::Series7) {
        if is_36 {
            ti.bel("RAMB36E1", &inst.name, "");
            ti.cfg("EN_ECC_READ", "FALSE");
            ti.cfg("EN_ECC_WRITE", "FALSE");
            ti.cfg("RAM_EXTENSION_A", "NONE");
            ti.cfg("RAM_EXTENSION_B", "NONE");
        } else {
            ti.bel("RAMB18E1", &inst.name, "");
        }
        ti.cfg("SAVEDATA", "FALSE");
        ti.cfg_int("READ_WIDTH_A", WIDTHS[awlog2]);
        ti.cfg_int("WRITE_WIDTH_A", WIDTHS[awlog2]);
        if dp {
            ti.cfg_int("READ_WIDTH_B", WIDTHS[bwlog2]);
            ti.cfg_int("WRITE_WIDTH_B", WIDTHS[bwlog2]);
        } else {
            ti.cfg_int("READ_WIDTH_B", 0);
            ti.cfg_int("WRITE_WIDTH_B", 0);
        }
        ti.cfg("WRITE_MODE_A", wmode_a);
        ti.cfg("WRITE_MODE_B", wmode_b);
        ti.cfg_int("DOA_REG", 0);
        ti.cfg_int("DOB_REG", 0);
        ti.cfg("RAM_MODE", "TDP");
        ti.cfg("RDADDR_COLLISION_HWCONFIG", "DELAYED_WRITE");
        ti.cfg("RSTREG_PRIORITY_A", "REGCE");
        ti.cfg("RSTREG_PRIORITY_B", "REGCE");
        if mode == Mode::Series7 {
            ti.cfg("EN_PWRGATE", "NONE");
        }
        for &ul in &uls {
            ti.pin_tie(&format!("REGCEAREGCE{ul}"), false);
            ti.pin_tie(&format!("REGCEB{ul}"), false);
            ti.pin_tie_inv(&format!("RSTREGARSTREG{ul}"), true, true);
            ti.pin_tie_inv(&format!("RSTREGB{ul}"), true, true);
            ti.pin_tie_inv(&format!("REGCLKARDRCLK{ul}"), true, true);
            if is_36 {
                ti.pin_tie_inv(&format!("REGCLKB{ul}"), false, false);
            } else {
                ti.pin_tie_inv(&format!("REGCLKB{ul}"), true, true);
            }
            if !dp {
                ti.pin_tie_inv(&format!("CLKBWRCLK{ul}"), true, true);
                ti.pin_tie_inv(&format!("ENBWREN{ul}"), true, true);
                ti.pin_tie_inv(&format!("RSTRAMB{ul}"), true, true);
                for i in 0..14 {
                    ti.pin_tie(&format!("ADDRBWRADDR{ul}{i}"), true);
                }
                for i in 0..8 {
                    ti.pin_tie(&format!("WEBWE{ul}{i}"), false);
                }
                if !is_36 {
                    ti.pin_tie("ADDRBTIEHIGH0", true);
                    ti.pin_tie("ADDRBTIEHIGH1", true);
                } else {
                    if ul == "L" {
                        ti.pin_tie("ADDRBWRADDRL14", false);
                        ti.pin_tie("ADDRBWRADDRL15", true);
                    } else {
                        ti.pin_tie("ADDRBWRADDRU14", false);
                    }
                }
            } else {
                for i in 4..8 {
                    ti.pin_tie(&format!("WEBWE{ul}{i}"), false);
                }
            }
        }
    } else if mode == Mode::Spartan3A {
        ti.bel("RAMB16BWE", &inst.name, "");
        ti.cfg_int("DATA_WIDTH_A", WIDTHS[awlog2]);
        if dp {
            ti.cfg_int("DATA_WIDTH_B", WIDTHS[bwlog2]);
        } else {
            ti.cfg_int("DATA_WIDTH_B", 0);
        }
        ti.cfg("WRITE_MODE_A", wmode_a);
        ti.cfg("WRITE_MODE_B", wmode_b);
    } else {
        ti.bel("RAMB16BWER", &inst.name, "");
        ti.cfg_int("DATA_WIDTH_A", WIDTHS[awlog2]);
        if dp {
            ti.cfg_int("DATA_WIDTH_B", WIDTHS[bwlog2]);
        } else {
            ti.cfg_int("DATA_WIDTH_B", 0);
        }
        ti.cfg_int("DOA_REG", 0);
        ti.cfg_int("DOB_REG", 0);
        ti.cfg("RSTTYPE", "SYNC");
        ti.cfg("WRITE_MODE_A", wmode_a);
        ti.cfg("WRITE_MODE_B", wmode_b);
        if mode == Mode::Spartan6 {
            ti.cfg("EN_RSTRAM_A", "TRUE");
            ti.cfg("EN_RSTRAM_B", "TRUE");
            ti.cfg("RAM_MODE", "TDP");
            ti.cfg("RST_PRIORITY_A", "CE");
            ti.cfg("RST_PRIORITY_B", "CE");
        }
    }

    if sz == 4 {
        for i in 0..16 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            if init_lowercase(mode) {
                ti.cfg_hex(&format!("INIT_{i:02x}"), &init, true);
            } else {
                ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
            }
        }
        if mode != Mode::Virtex {
            for i in 16..64 {
                if init_lowercase(mode) {
                    ti.cfg(&format!("INIT_{i:02x}"), ZERO_INIT);
                } else {
                    ti.cfg(&format!("INIT_{i:02X}"), ZERO_INIT);
                }
            }
            for i in 0..8 {
                ti.cfg(&format!("INITP_{i:02X}"), ZERO_INIT);
            }
        }
    } else {
        for i in 0..64 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            if mode == Mode::Virtex5 && !is_36 {
                ti.cfg_hex(&format!("INIT_{i:02X}_{ul}"), &init, true);
            } else if init_lowercase(mode) {
                ti.cfg_hex(&format!("INIT_{i:02x}"), &init, true);
            } else {
                ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
            }
        }
        if awlog2 >= 3 || (dp && bwlog2 >= 3) {
            for i in 0..8 {
                let init = ctx.gen_bits(256);
                inst.param_bits(&format!("INITP_{i:02X}"), &init);
                if mode == Mode::Virtex5 && !is_36 {
                    ti.cfg_hex(&format!("INITP_{i:02X}_{ul}"), &init, true);
                } else if init_lowercase(mode) {
                    ti.cfg_hex(&format!("INITP_{i:02x}"), &init, true);
                } else {
                    ti.cfg_hex(&format!("INITP_{i:02X}"), &init, true);
                }
            }
        } else if mode != Mode::Virtex2 {
            for i in 0..8 {
                if mode == Mode::Virtex5 && !is_36 {
                    ti.cfg(&format!("INITP_{i:02X}_{ul}"), &ZERO_INIT);
                } else {
                    ti.cfg(&format!("INITP_{i:02X}"), ZERO_INIT);
                }
            }
        }
    }
    if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) && is_36 {
        for i in 64..128 {
            ti.cfg(&format!("INIT_{i:02X}"), ZERO_INIT);
        }
        for i in 8..16 {
            ti.cfg(&format!("INITP_{i:02X}"), ZERO_INIT);
        }
    }

    let tab_sp = [("", "A", awlog2, aw, awp)];
    let tab_dp = [("A", "A", awlog2, aw, awp), ("B", "B", bwlog2, bw, bwp)];

    for &(vl, xl, wlog2, w, wp) in if dp { &tab_dp[..] } else { &tab_sp[..] } {
        let top = if sz == 4 { 12 } else { 14 };
        let addr = test.make_ins(ctx, top - wlog2);
        inst.connect_bus(&format!("ADDR{vl}"), &addr);
        if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            for ul in &uls {
                for i in 0..wlog2 {
                    if xl == "A" {
                        ti.pin_tie(&format!("ADDRARDADDR{ul}{i}"), true);
                    } else {
                        ti.pin_tie(&format!("ADDRBWRADDR{ul}{i}"), true);
                    }
                }
                for i in wlog2..top {
                    if xl == "A" {
                        ti.pin_in(&format!("ADDRARDADDR{ul}{i}"), &addr[i - wlog2]);
                    } else {
                        ti.pin_in(&format!("ADDRBWRADDR{ul}{i}"), &addr[i - wlog2]);
                    }
                }
            }
            if is_36 {
                if xl == "A" {
                    ti.pin_tie("ADDRARDADDRU14", false);
                    ti.pin_tie("ADDRARDADDRL14", false);
                    ti.pin_tie("ADDRARDADDRL15", true);
                } else {
                    ti.pin_tie("ADDRBWRADDRU14", false);
                    ti.pin_tie("ADDRBWRADDRL14", false);
                    ti.pin_tie("ADDRBWRADDRL15", true);
                }
            } else {
                for i in 0..2 {
                    ti.pin_tie(&format!("ADDR{xl}TIEHIGH{i}"), true);
                }
            }
        } else if mode == Mode::Virtex5 {
            for ul in &uls {
                for i in wlog2..top {
                    let mut hwi = i;
                    if !is_36 {
                        hwi = i + 1;
                    }
                    if xl == "A" {
                        ti.pin_in(&format!("ADDRA{ul}{hwi}"), &addr[i - wlog2]);
                    } else {
                        ti.pin_in(&format!("ADDRB{ul}{hwi}"), &addr[i - wlog2]);
                    }
                }
            }
            if is_36 {
                if xl == "A" {
                    ti.pin_tie("ADDRAU14", false);
                    ti.pin_tie("ADDRAL14", false);
                    ti.pin_tie("ADDRAL15", true);
                } else {
                    ti.pin_tie("ADDRBU14", false);
                    ti.pin_tie("ADDRBL14", false);
                    ti.pin_tie("ADDRBL15", true);
                }
            } else if ul == "L" {
                if xl == "A" {
                    ti.pin_tie("ADDRAL15", true);
                } else {
                    ti.pin_tie("ADDRBL15", true);
                }
            }
        } else {
            for i in wlog2..top {
                let hwi;
                if mode == Mode::Spartan6 && (awlog2 == 5 || bwlog2 == 5) && wlog2 != 5 {
                    // TODO: wtf
                    hwi = match i {
                        0..=3 => i,
                        4 => 13,
                        _ => i - 1,
                    };
                } else {
                    hwi = i;
                }
                ti.pin_in(&format!("ADDR{xl}{hwi}"), &addr[i - wlog2]);
            }
            if sz == 4 && mode != Mode::Virtex {
                for i in 12..14 {
                    ti.pin_tie(&format!("ADDR{xl}{i}"), false);
                }
            }
        }

        let do_ = test.make_outs(ctx, w);
        inst.connect_bus(&format!("DO{vl}"), &do_);
        let di = test.make_ins(ctx, w);
        inst.connect_bus(&format!("DI{vl}"), &di);
        for i in 0..w {
            if matches!(mode, Mode::Virtex6 | Mode::Series7) {
                ti.pin_in(&format!("DI{xl}DI{i}"), &di[i]);
                ti.pin_out(&format!("DO{xl}DO{i}"), &do_[i]);
            } else if mode == Mode::Virtex5 && !is_36 {
                ti.pin_in(&format!("DI{xl}{ul}{i}"), &di[i]);
                ti.pin_out(&format!("DO{xl}{ul}{i}"), &do_[i]);
            } else {
                ti.pin_in(&format!("DI{xl}{i}"), &di[i]);
                ti.pin_out(&format!("DO{xl}{i}"), &do_[i]);
            }
        }
        if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7)
            && xl == "A"
            && w == 1
            && is_36
        {
            // ?
            if mode == Mode::Virtex5 {
                ti.pin_in("DIA1", &di[0]);
            } else {
                ti.pin_in("DIADI1", &di[0]);
            }
        }

        if sz == 4 && mode != Mode::Virtex {
            let num = match wlog2 {
                3 => 1,
                4 => 2,
                _ => 0,
            };
            for i in 0..num {
                ti.pin_tie(&format!("DIP{xl}{i}"), true);
            }
        }
        if sz == 16 && wlog2 >= 3 {
            let pw = 1 << (wlog2 - 3);
            let dop = test.make_outs(ctx, pw);
            inst.connect_bus(&format!("DOP{vl}"), &dop);
            let dip = test.make_ins(ctx, pw);
            inst.connect_bus(&format!("DIP{vl}"), &dip);
            for i in 0..pw {
                if matches!(mode, Mode::Virtex6 | Mode::Series7) {
                    ti.pin_in(&format!("DIP{xl}DIP{i}"), &dip[i]);
                    ti.pin_out(&format!("DOP{xl}DOP{i}"), &dop[i]);
                } else if mode == Mode::Virtex5 && !is_36 {
                    ti.pin_in(&format!("DIP{xl}{ul}{i}"), &dip[i]);
                    ti.pin_out(&format!("DOP{xl}{ul}{i}"), &dop[i]);
                } else {
                    ti.pin_in(&format!("DIP{xl}{i}"), &dip[i]);
                    ti.pin_out(&format!("DOP{xl}{i}"), &dop[i]);
                }
            }
            if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7)
                && xl == "A"
                && w == 8
                && is_36
            {
                // ?
                if mode == Mode::Virtex5 {
                    ti.pin_in("DIPA1", &dip[0]);
                } else {
                    ti.pin_in("DIPADIP1", &dip[0]);
                }
            }
        }

        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
        let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
        inst.connect(&format!("CLK{vl}"), &clk_v);
        inst.connect(&format!("EN{vl}"), &en_v);
        if sz == 4 {
            inst.connect(&format!("RST{vl}"), &rst_v);
        } else {
            inst.connect(&format!("SSR{vl}"), &rst_v);
        }
        if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            let we = test.make_in(ctx);
            inst.connect(&format!("WE{vl}"), &we);
            for ul in &uls {
                if xl == "A" {
                    ti.pin_in_inv(&format!("CLKARDCLK{ul}"), &clk_x, clk_inv);
                    ti.pin_in_inv(&format!("ENARDEN{ul}"), &en_x, en_inv);
                    ti.pin_in_inv(&format!("RSTRAMARSTRAM{ul}"), &rst_x, rst_inv);
                    for i in 0..4 {
                        ti.pin_in(&format!("WEA{ul}{i}"), &we);
                    }
                } else {
                    ti.pin_in_inv(&format!("CLKBWRCLK{ul}"), &clk_x, clk_inv);
                    ti.pin_in_inv(&format!("ENBWREN{ul}"), &en_x, en_inv);
                    ti.pin_in_inv(&format!("RSTRAMB{ul}"), &rst_x, rst_inv);
                    for i in 0..4 {
                        ti.pin_in(&format!("WEBWE{ul}{i}"), &we);
                    }
                }
            }
        } else if mode == Mode::Virtex5 {
            let we = test.make_in(ctx);
            inst.connect(&format!("WE{vl}"), &we);
            for ul in &uls {
                ti.pin_in_inv(&format!("CLK{xl}{ul}"), &clk_x, clk_inv);
                ti.pin_in_inv(&format!("EN{xl}{ul}"), &en_x, en_inv);
                ti.pin_in_inv(&format!("SSR{xl}{ul}"), &rst_x, rst_inv);
                if !is_36 {
                    ti.pin_in_inv(&format!("REGCLK{xl}{ul}"), &clk_x, clk_inv);
                }
                for i in 0..4 {
                    ti.pin_in(&format!("WE{xl}{ul}{i}"), &we);
                }
            }
        } else {
            if mode == Mode::Virtex {
                ti.pin_in(&format!("CLK{xl}"), &clk_x);
                ti.pin_in(&format!("EN{xl}"), &en_x);
                ti.pin_in(&format!("RST{xl}"), &rst_x);
                ti.cfg(&format!("CLK{xl}MUX"), if clk_inv { "0" } else { "1" });
                ti.cfg(
                    &format!("EN{xl}MUX"),
                    &if en_inv {
                        format!("EN{xl}_B")
                    } else {
                        format!("EN{xl}")
                    },
                );
                ti.cfg(
                    &format!("RST{xl}MUX"),
                    &if rst_inv {
                        format!("RST{xl}_B")
                    } else {
                        format!("RST{xl}")
                    },
                );
            } else {
                ti.pin_in_inv(&format!("CLK{xl}"), &clk_x, clk_inv);
                ti.pin_in_inv(&format!("EN{xl}"), &en_x, en_inv);
                if matches!(mode, Mode::Spartan3ADsp | Mode::Spartan6) {
                    ti.pin_in_inv(&format!("RST{xl}"), &rst_x, rst_inv);
                } else {
                    ti.pin_in_inv(&format!("SSR{xl}"), &rst_x, rst_inv);
                }
            }
            if !bwe {
                let (we_v, we_x, we_inv) = test.make_in_inv(ctx);
                inst.connect(&format!("WE{vl}"), &we_v);
                if mode == Mode::Virtex {
                    ti.cfg(
                        &format!("WE{xl}MUX"),
                        &if we_inv {
                            format!("WE{xl}_B")
                        } else {
                            format!("WE{xl}")
                        },
                    );
                    ti.pin_in(&format!("WE{xl}"), &we_x);
                } else if mode == Mode::Virtex2 {
                    ti.pin_in_inv(&format!("WE{xl}"), &we_x, we_inv);
                } else {
                    for i in 0..4 {
                        ti.pin_in_inv(&format!("WE{xl}{i}"), &we_x, we_inv);
                    }
                }
            } else {
                let mut we = Vec::new();
                for i in 0..(1 << wlog2 - 3) {
                    let (we_v, we_x, we_inv) = test.make_in_inv(ctx);
                    we.push(we_v);
                    for j in 0..(1 << 5 - wlog2) {
                        let ii = i + j * (1 << wlog2 - 3);
                        ti.pin_in_inv(&format!("WE{xl}{ii}"), &we_x, we_inv);
                    }
                }
                inst.connect_bus(&format!("WE{vl}"), &we);
            }
        }

        if mode != Mode::Virtex {
            if sz == 4 {
                if mode == Mode::Virtex2 {
                    let ival = match wlog2 {
                        0 | 1 | 2 => "0",
                        3 => "000",
                        4 => "00000",
                        5 => "000000000",
                        _ => unreachable!(),
                    };
                    ti.cfg(&format!("INIT_{xl}"), ival);
                    ti.cfg(&format!("SRVAL_{xl}"), ival);
                } else {
                    ti.cfg(&format!("INIT_{xl}"), "000000000");
                    ti.cfg(&format!("SRVAL_{xl}"), "000000000");
                }
            } else {
                let mut init = ctx.gen_bits(wp);
                let mut srval = ctx.gen_bits(wp);
                if dp {
                    inst.param_bits(&format!("INIT_{xl}"), &init);
                    inst.param_bits(&format!("SRVAL_{xl}"), &srval);
                } else {
                    inst.param_bits("INIT", &init);
                    inst.param_bits("SRVAL", &srval);
                }
                if mode != Mode::Virtex2 {
                    for _ in wp..36 {
                        init.push(BitVal::S0);
                        srval.push(BitVal::S0);
                    }
                }
                if mode == Mode::Virtex5 && !is_36 {
                    ti.cfg_hex(&format!("INIT_{xl}_{ul}"), &init, true);
                    ti.cfg_hex(&format!("SRVAL_{xl}_{ul}"), &srval, true);
                } else {
                    ti.cfg_hex(&format!("INIT_{xl}"), &init, true);
                    ti.cfg_hex(&format!("SRVAL_{xl}"), &srval, true);
                }
            }
        }
    }
    if matches!(mode, Mode::Spartan3A | Mode::Spartan3ADsp | Mode::Spartan6) && !dp {
        if mode != Mode::Spartan6 {
            for i in 0..4 {
                ti.pin_tie_inv(&format!("WEB{i}"), true, true);
            }
        }
    }
    if !matches!(mode, Mode::Virtex | Mode::Virtex2) && !dp {
        if mode == Mode::Virtex5 && !is_36 {
            ti.cfg(&format!("INIT_B_{ul}"), "000000000");
            ti.cfg(&format!("SRVAL_B_{ul}"), "000000000");
        } else {
            ti.cfg("INIT_B", "000000000");
            ti.cfg("SRVAL_B", "000000000");
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ramb_bwer(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8, sdp: bool) {
    let prim = format!("RAMB{sz}BWER");
    let mut inst = SrcInst::new(ctx, &prim);
    let mut ti = TgtInst::new(&[&prim]);

    let (use_a, use_b) = if sdp {
        (true, true)
    } else {
        match ctx.rng.gen_range(0..3) {
            0 => (true, true),
            1 => (false, true),
            2 => (true, false),
            _ => unreachable!(),
        }
    };
    let (mut awlog2, mut bwlog2);
    if sdp {
        awlog2 = 5;
        bwlog2 = 5;
    } else if sz == 8 {
        awlog2 = ctx.rng.gen_range(0..5);
        bwlog2 = ctx.rng.gen_range(0..5);
    } else {
        awlog2 = ctx.rng.gen_range(0..6);
        bwlog2 = ctx.rng.gen_range(0..6);
    }
    if !use_a {
        awlog2 = 0;
    }
    if !use_b {
        bwlog2 = 0;
    }
    ti.bel(&prim, &inst.name, "");

    if sz == 8 {
        inst.param_str("RAM_MODE", if sdp { "SDP" } else { "TDP" });
    }
    if mode == Mode::Spartan6 {
        ti.cfg("RAM_MODE", if sdp { "SDP" } else { "TDP" });
    }

    if sz == 16 {
        for i in 0..64 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
        }
        for i in 0..8 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INITP_{i:02X}"), &init);
            ti.cfg_hex(&format!("INITP_{i:02X}"), &init, true);
        }
    } else {
        for i in 0..32 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
        }
        for i in 0..4 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INITP_{i:02X}"), &init);
            ti.cfg_hex(&format!("INITP_{i:02X}"), &init, true);
        }
    }

    let rsttype = if ctx.rng.gen() { "ASYNC" } else { "SYNC" };
    inst.param_str("RSTTYPE", rsttype);
    ti.cfg("RSTTYPE", rsttype);

    for (a, use_, wlog2) in [('A', use_a, awlog2), ('B', use_b, bwlog2)] {
        if use_ {
            inst.param_int(&format!("DATA_WIDTH_{a}"), WIDTHS[wlog2]);
            ti.cfg_int(&format!("DATA_WIDTH_{a}"), WIDTHS[wlog2]);
        } else {
            inst.param_int(&format!("DATA_WIDTH_{a}"), 0);
            ti.cfg_int(&format!("DATA_WIDTH_{a}"), 0);
        }

        let do_reg = ctx.rng.gen_range(0..2);
        inst.param_int(&format!("DO{a}_REG"), do_reg);
        ti.cfg_int(&format!("DO{a}_REG"), do_reg);
        let wrmode = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]
            .choose(&mut ctx.rng)
            .unwrap();
        inst.param_str(&format!("WRITE_MODE_{a}"), wrmode);
        ti.cfg(&format!("WRITE_MODE_{a}"), wrmode);

        let addr = test.make_ins(ctx, if sz == 16 { 14 } else { 13 });
        if sz == 16 {
            inst.connect_bus(&format!("ADDR{a}"), &addr);
        } else if a == 'A' {
            inst.connect_bus("ADDRAWRADDR", &addr);
        } else {
            inst.connect_bus("ADDRBRDADDR", &addr);
        }
        let di = test.make_ins(ctx, if sz == 16 { 32 } else { 16 });
        let dip = test.make_ins(ctx, if sz == 16 { 4 } else { 2 });
        let do_ = test.make_outs(ctx, if sz == 16 { 32 } else { 16 });
        let dop = test.make_outs(ctx, if sz == 16 { 4 } else { 2 });
        if sz == 16 {
            inst.connect_bus(&format!("DI{a}"), &di);
            inst.connect_bus(&format!("DIP{a}"), &dip);
            inst.connect_bus(&format!("DO{a}"), &do_);
            inst.connect_bus(&format!("DOP{a}"), &dop);
        } else {
            inst.connect_bus(&format!("DI{a}DI"), &di);
            inst.connect_bus(&format!("DIP{a}DIP"), &dip);
            inst.connect_bus(&format!("DO{a}DO"), &do_);
            inst.connect_bus(&format!("DOP{a}DOP"), &dop);
        }

        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
        let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
        let (regce_v, regce_x, regce_inv) = test.make_in_inv(ctx);
        if sz == 16 {
            inst.connect(&format!("CLK{a}"), &clk_v);
            inst.connect(&format!("EN{a}"), &en_v);
            inst.connect(&format!("RST{a}"), &rst_v);
            inst.connect(&format!("REGCE{a}"), &regce_v);
        } else if a == 'A' {
            inst.connect("CLKAWRCLK", &clk_v);
            inst.connect("ENAWREN", &en_v);
            inst.connect("RSTA", &rst_v);
            inst.connect("REGCEA", &regce_v);
        } else {
            inst.connect("CLKBRDCLK", &clk_v);
            inst.connect("ENBRDEN", &en_v);
            inst.connect("RSTBRST", &rst_v);
            inst.connect("REGCEBREGCE", &regce_v);
        }

        let init = ctx.gen_bits(if sz == 16 { 36 } else { 18 });
        let srval = ctx.gen_bits(if sz == 16 { 36 } else { 18 });
        inst.param_bits(&format!("INIT_{a}"), &init);
        inst.param_bits(&format!("SRVAL_{a}"), &srval);
        ti.cfg_hex(&format!("INIT_{a}"), &init, true);
        ti.cfg_hex(&format!("SRVAL_{a}"), &srval, true);

        if mode == Mode::Spartan6 {
            let en_rstram = if ctx.rng.gen() { "TRUE" } else { "FALSE" };
            inst.param_str(&format!("EN_RSTRAM_{a}"), en_rstram);
            ti.cfg(&format!("EN_RSTRAM_{a}"), en_rstram);
            let rst_priority = if ctx.rng.gen() { "CE" } else { "SR" };
            inst.param_str(&format!("RST_PRIORITY_{a}"), rst_priority);
            ti.cfg(&format!("RST_PRIORITY_{a}"), rst_priority);
        }

        if use_ || mode == Mode::Spartan6 {
            if mode == Mode::Spartan3ADsp {
                for i in wlog2..14 {
                    ti.pin_in(&format!("ADDR{a}{i}"), &addr[i]);
                }
                for i in 0..(1 << wlog2) {
                    ti.pin_in(&format!("DI{a}{i}"), &di[i]);
                    ti.pin_out(&format!("DO{a}{i}"), &do_[i]);
                }
                if wlog2 >= 3 {
                    for i in 0..(1 << wlog2 - 3) {
                        ti.pin_in(&format!("DIP{a}{i}"), &dip[i]);
                        ti.pin_out(&format!("DOP{a}{i}"), &dop[i]);
                    }
                }
            } else if sz == 16 {
                for i in 0..14 {
                    let hwi;
                    if (awlog2 == 5 || bwlog2 == 5) && wlog2 != 5 {
                        // TODO: wtf
                        hwi = match i {
                            0..=3 => i,
                            4 => 13,
                            _ => i - 1,
                        };
                    } else {
                        hwi = i;
                    }
                    ti.pin_in(&format!("ADDR{a}{hwi}"), &addr[i]);
                }
                for i in 0..32 {
                    ti.pin_in(&format!("DI{a}{i}"), &di[i]);
                    ti.pin_out(&format!("DO{a}{i}"), &do_[i]);
                }
                for i in 0..4 {
                    ti.pin_in(&format!("DIP{a}{i}"), &dip[i]);
                    ti.pin_out(&format!("DOP{a}{i}"), &dop[i]);
                }
            } else {
                for i in 0..13 {
                    if a == 'A' {
                        ti.pin_in(&format!("ADDRAWRADDR{i}"), &addr[i]);
                    } else {
                        ti.pin_in(&format!("ADDRBRDADDR{i}"), &addr[i]);
                    }
                }
                for i in 0..16 {
                    ti.pin_in(&format!("DI{a}DI{i}"), &di[i]);
                    ti.pin_out(&format!("DO{a}DO{i}"), &do_[i]);
                }
                for i in 0..2 {
                    ti.pin_in(&format!("DIP{a}DIP{i}"), &dip[i]);
                    ti.pin_out(&format!("DOP{a}DOP{i}"), &dop[i]);
                }
            }
            if sz == 16 {
                ti.pin_in_inv(&format!("CLK{a}"), &clk_x, clk_inv);
                ti.pin_in_inv(&format!("EN{a}"), &en_x, en_inv);
                ti.pin_in_inv(&format!("RST{a}"), &rst_x, rst_inv);
                ti.pin_in_inv(&format!("REGCE{a}"), &regce_x, regce_inv);
            } else if a == 'A' {
                ti.pin_in_inv("CLKAWRCLK", &clk_x, clk_inv);
                ti.pin_in_inv("ENAWREN", &en_x, en_inv);
                ti.pin_in_inv("RSTA", &rst_x, rst_inv);
                ti.pin_in_inv("REGCEA", &regce_x, regce_inv);
            } else {
                ti.pin_in_inv("CLKBRDCLK", &clk_x, clk_inv);
                ti.pin_in_inv("ENBRDEN", &en_x, en_inv);
                ti.pin_in_inv("RSTBRST", &rst_x, rst_inv);
                ti.pin_in_inv("REGCEBREGCE", &regce_x, regce_inv);
            }
            let mut we = Vec::new();
            for i in 0..(if sz == 16 { 4 } else { 2 }) {
                let (we_v, we_x, we_inv) = test.make_in_inv(ctx);
                we.push(we_v);
                if sz == 16 {
                    ti.pin_in_inv(&format!("WE{a}{i}"), &we_x, we_inv);
                } else if a == 'A' {
                    ti.pin_in_inv(&format!("WEAWEL{i}"), &we_x, we_inv);
                } else {
                    ti.pin_in_inv(&format!("WEBWEU{i}"), &we_x, we_inv);
                }
            }
            if sz == 16 {
                inst.connect_bus(&format!("WE{a}"), &we);
            } else if a == 'A' {
                inst.connect_bus("WEAWEL", &we);
            } else {
                inst.connect_bus("WEBWEU", &we);
            }
        } else {
            for i in 0..4 {
                ti.pin_tie_inv(&format!("WE{a}{i}"), true, true);
            }
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ramb16(test: &mut Test, ctx: &mut TestGenCtx, num: usize) {
    let mut insts = Vec::new();
    let mut tis = Vec::new();
    let clka_x = test.make_bufg(ctx);
    let clkb_x = test.make_bufg(ctx);
    let clka_inv = ctx.rng.gen();
    let clkb_inv = ctx.rng.gen();
    let clka_v;
    let clkb_v;
    if clka_inv {
        clka_v = test.make_inv(ctx, &clka_x);
    } else {
        clka_v = clka_x.clone();
    }
    if clkb_inv {
        clkb_v = test.make_inv(ctx, &clkb_x);
    } else {
        clkb_v = clkb_x.clone();
    }

    for _ in 0..num {
        let (ar_en, br_en) = *[(true, true), (false, true), (true, false)]
            .choose(&mut ctx.rng)
            .unwrap();
        let aw_en = ctx.rng.gen();
        let bw_en = ctx.rng.gen();
        let arwlog2 = ctx.rng.gen_range(0..6);
        let awwlog2 = ctx.rng.gen_range(0..6);
        let brwlog2 = ctx.rng.gen_range(0..6);
        let bwwlog2 = ctx.rng.gen_range(0..6);
        let arw = if ar_en { WIDTHS[arwlog2] } else { 0 };
        let aww = if aw_en { WIDTHS[awwlog2] } else { 0 };
        let brw = if br_en { WIDTHS[brwlog2] } else { 0 };
        let bww = if bw_en { WIDTHS[bwwlog2] } else { 0 };

        let mut inst = SrcInst::new(ctx, "RAMB16");
        let mut ti = TgtInst::new(&["RAMB16"]);
        ti.bel("RAMB16", &inst.name, "");

        for i in 0..64 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
        }
        for i in 0..8 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INITP_{i:02X}"), &init);
            ti.cfg_hex(&format!("INITP_{i:02X}"), &init, true);
        }

        inst.param_int("READ_WIDTH_A", arw);
        inst.param_int("READ_WIDTH_B", brw);
        inst.param_int("WRITE_WIDTH_A", aww);
        inst.param_int("WRITE_WIDTH_B", bww);

        ti.cfg_int("READ_WIDTH_A", arw);
        ti.cfg_int("READ_WIDTH_B", brw);
        ti.cfg_int("WRITE_WIDTH_A", aww);
        ti.cfg_int("WRITE_WIDTH_B", bww);

        inst.connect("CLKA", &clka_v);
        inst.connect("CLKB", &clkb_v);
        ti.pin_in_inv("CLKA", &clka_x, clka_inv);
        ti.pin_in_inv("CLKB", &clkb_x, clkb_inv);

        for (l, r_en, w_en, rwlog2, wwlog2) in [
            ('A', ar_en, aw_en, arwlog2, awwlog2),
            ('B', br_en, bw_en, brwlog2, bwwlog2),
        ] {
            if r_en {
                let do_ = test.make_outs(ctx, 1 << rwlog2);
                inst.connect_bus(&format!("DO{l}"), &do_);
                for (i, w) in do_.iter().enumerate() {
                    ti.pin_out(&format!("DO{l}{i}"), w);
                }
                if rwlog2 >= 3 {
                    let dop = test.make_outs(ctx, 1 << rwlog2 - 3);
                    inst.connect_bus(&format!("DOP{l}"), &dop);
                    for (i, w) in dop.iter().enumerate() {
                        ti.pin_out(&format!("DOP{l}{i}"), w);
                    }
                }
            }
            let di = test.make_ins(ctx, 36);
            let dip = test.make_ins(ctx, 4);
            inst.connect_bus(&format!("DI{l}"), &di);
            inst.connect_bus(&format!("DIP{l}"), &dip);
            if w_en {
                for i in 0..(1 << wwlog2) {
                    ti.pin_in(&format!("DI{l}{i}"), &di[i]);
                }
                if wwlog2 >= 3 {
                    for i in 0..(1 << wwlog2 - 3) {
                        ti.pin_in(&format!("DIP{l}{i}"), &dip[i]);
                    }
                }
            }

            let do_reg = ctx.rng.gen_range(0..2);
            let invert_do_reg = if do_reg == 1 && ctx.rng.gen() {
                "TRUE"
            } else {
                "FALSE"
            };
            inst.param_int(&format!("DO{l}_REG"), do_reg);
            inst.param_str(&format!("INVERT_CLK_DO{l}_REG"), invert_do_reg);
            ti.cfg_int(&format!("DO{l}_REG"), do_reg);
            ti.cfg(&format!("INVERT_CLK_DO{l}_REG"), invert_do_reg);

            let wmode;
            if num == 2 {
                wmode = *["WRITE_FIRST", "READ_FIRST"].choose(&mut ctx.rng).unwrap();
            } else {
                wmode = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]
                    .choose(&mut ctx.rng)
                    .unwrap();
            }
            inst.param_str(&format!("WRITE_MODE_{l}"), wmode);
            ti.cfg(&format!("WRITE_MODE_{l}"), wmode);

            let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
            inst.connect(&format!("EN{l}"), &en_v);
            ti.pin_in_inv(&format!("EN{l}"), &en_x, en_inv);

            if do_reg == 0 {
                let (ssr_v, ssr_x, ssr_inv) = test.make_in_inv(ctx);
                inst.connect(&format!("SSR{l}"), &ssr_v);
                ti.pin_in_inv(&format!("SSR{l}"), &ssr_x, ssr_inv);
            }

            let (regce_v, regce_x, regce_inv) = test.make_in_inv(ctx);
            inst.connect(&format!("REGCE{l}"), &regce_v);
            ti.pin_in_inv(&format!("REGCE{l}"), &regce_x, regce_inv);

            ti.cfg("SAVEDATA", "FALSE");
            ti.cfg("EN_ECC_READ", "FALSE");
            ti.cfg("EN_ECC_WRITE", "FALSE");

            let addr = test.make_ins(ctx, 15);
            inst.connect_bus(&format!("ADDR{l}"), &addr);
            for i in 0..15 {
                if num == 1 && i == 14 {
                    continue;
                }
                if (i < rwlog2 || !r_en) && (i < wwlog2 || !w_en) && i < 14 {
                    continue;
                }
                ti.pin_in(&format!("ADDR{l}{i}"), &addr[i]);
            }

            let init = ctx.gen_bits(36);
            let srval = ctx.gen_bits(36);
            inst.param_bits(&format!("INIT_{l}"), &init);
            inst.param_bits(&format!("SRVAL_{l}"), &srval);
            ti.cfg_hex(&format!("INIT_{l}"), &init, true);
            ti.cfg_hex(&format!("SRVAL_{l}"), &srval, true);

            let mut we = Vec::new();
            match wwlog2 {
                5 => {
                    for i in 0..4 {
                        let (we_v, we_x, we_inv) = test.make_in_inv(ctx);
                        we.push(we_v);
                        ti.pin_in_inv(&format!("WE{l}{i}"), &we_x, we_inv);
                    }
                }
                4 => {
                    for i in 0..2 {
                        let (we_v, we_x, we_inv) = test.make_in_inv(ctx);
                        we.push(we_v);
                        for j in 0..2 {
                            ti.pin_in_inv(&format!("WE{l}{k}", k = i + 2 * j), &we_x, we_inv);
                        }
                    }
                    let we0 = we[0].clone();
                    let we1 = we[1].clone();
                    we.push(we0);
                    we.push(we1);
                }
                _ => {
                    let (we_v, we_x, we_inv) = test.make_in_inv(ctx);
                    for j in 0..4 {
                        ti.pin_in_inv(&format!("WE{l}{j}"), &we_x, we_inv);
                    }
                    we.push(we_v.clone());
                    we.push(we_v.clone());
                    we.push(we_v.clone());
                    we.push(we_v);
                }
            }
            inst.connect_bus(&format!("WE{l}"), &we);
        }

        insts.push(inst);
        tis.push(ti);
    }

    for l in ['A', 'B'] {
        if num == 2 {
            let c = test.make_wire(ctx);
            insts[0].connect(&format!("CASCADEOUT{l}"), &c);
            insts[1].connect(&format!("CASCADEIN{l}"), &c);
            tis[0].pin_out(&format!("CASCADEOUT{l}"), &c);
            tis[1].pin_in(&format!("CASCADEIN{l}"), &c);
            insts[0].param_str(&format!("RAM_EXTENSION_{l}"), "LOWER");
            insts[1].param_str(&format!("RAM_EXTENSION_{l}"), "UPPER");
            tis[0].cfg(&format!("RAM_EXTENSION_{l}"), "LOWER");
            tis[1].cfg(&format!("RAM_EXTENSION_{l}"), "UPPER");
        } else {
            insts[0].param_str(&format!("RAM_EXTENSION_{l}"), "NONE");
            tis[0].cfg(&format!("RAM_EXTENSION_{l}"), "NONE");
        }
    }

    for inst in insts {
        test.src_insts.push(inst);
    }
    for ti in tis {
        test.tgt_insts.push(ti);
    }
}

fn gen_ramb32_ecc(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let mut inst = SrcInst::new(ctx, "RAMB32_S64_ECC");

    let do_reg = if mode == Mode::Virtex4 {
        ctx.rng.gen_range(0..2)
    } else {
        0
    };
    inst.param_int("DO_REG", do_reg);

    let di = test.make_ins(ctx, 64);
    inst.connect_bus("DI", &di);
    let do_ = test.make_outs(ctx, 64);
    inst.connect_bus("DO", &do_);
    let status = test.make_outs(ctx, 2);
    inst.connect_bus("STATUS", &status);
    let rdaddr = test.make_ins(ctx, 9);
    inst.connect_bus("RDADDR", &rdaddr);
    let wraddr = test.make_ins(ctx, 9);
    inst.connect_bus("WRADDR", &wraddr);
    let (rdclk_v, rdclk_x, rdclk_inv) = test.make_in_inv(ctx);
    inst.connect("RDCLK", &rdclk_v);
    let (wrclk_v, wrclk_x, wrclk_inv) = test.make_in_inv(ctx);
    inst.connect("WRCLK", &wrclk_v);
    let (rden_v, rden_x, rden_inv) = test.make_in_inv(ctx);
    inst.connect("RDEN", &rden_v);
    let (wren_v, wren_x, wren_inv) = test.make_in_inv(ctx);
    inst.connect("WREN", &wren_v);

    if mode == Mode::Virtex4 {
        inst.connect("SSR", "0");
        let mut tis = [TgtInst::new(&["RAMB16"]), TgtInst::new(&["RAMB16"])];
        tis[0].bel("RAMB16", &format!("{}/RAMB16_LOWER", inst.name), "");
        tis[1].bel("RAMB16", &format!("{}/RAMB16_UPPER", inst.name), "");
        for ti in &mut tis {
            ti.cfg("INIT_A", "000000000");
            ti.cfg("INIT_B", "000000000");
            ti.cfg("SRVAL_A", "000000000");
            ti.cfg("SRVAL_B", "000000000");
            ti.cfg_int("DOA_REG", do_reg);
            ti.cfg_int("DOB_REG", 0);
            ti.cfg("INVERT_CLK_DOA_REG", "FALSE");
            ti.cfg("INVERT_CLK_DOB_REG", "FALSE");
            ti.cfg("SAVEDATA", "FALSE");
            ti.cfg("EN_ECC_READ", "TRUE");
            ti.cfg("EN_ECC_WRITE", "TRUE");
            ti.cfg("RAM_EXTENSION_A", "NONE");
            ti.cfg("RAM_EXTENSION_B", "NONE");
            ti.cfg("READ_WIDTH_A", "36");
            ti.cfg("READ_WIDTH_B", "36");
            ti.cfg("WRITE_WIDTH_A", "36");
            ti.cfg("WRITE_WIDTH_B", "36");
            ti.cfg("WRITE_MODE_A", "READ_FIRST");
            ti.cfg("WRITE_MODE_B", "READ_FIRST");
            for i in 0..64 {
                ti.cfg(&format!("INIT_{i:02X}"), ZERO_INIT);
            }
            for i in 0..8 {
                ti.cfg(&format!("INITP_{i:02X}"), ZERO_INIT);
            }
            ti.pin_in_inv("CLKA", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("CLKB", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("ENA", &rden_x, rden_inv);
            ti.pin_in_inv("ENB", &wren_x, wren_inv);
            ti.pin_tie_inv("WEA0", false, false);
            ti.pin_tie_inv("WEA1", false, false);
            ti.pin_tie_inv("WEA2", false, false);
            ti.pin_tie_inv("WEA3", false, false);
            ti.pin_tie_inv("WEB0", true, false);
            ti.pin_tie_inv("WEB1", true, false);
            ti.pin_tie_inv("WEB2", true, false);
            ti.pin_tie_inv("WEB3", true, false);
            ti.pin_tie_inv("REGCEA", true, false);
            ti.pin_tie_inv("REGCEB", false, false);
            ti.pin_tie_inv("SSRA", false, false);
            ti.pin_tie_inv("SSRB", false, false);
            for i in 0..9 {
                ti.pin_in(&format!("ADDRA{}", i + 5), &rdaddr[i]);
                ti.pin_in(&format!("ADDRB{}", i + 5), &wraddr[i]);
            }
            for i in 0..32 {
                ti.pin_tie(&format!("DIA{}", i), false);
            }
            for i in 0..4 {
                ti.pin_tie(&format!("DIPA{}", i), false);
            }
        }

        for i in 0..32 {
            match i {
                14 => {
                    tis[0].pin_in("DIPB1", &di[i]);
                    tis[0].pin_out("DOPA1", &do_[i]);
                }
                15 => {
                    tis[0].pin_in("DIPB3", &di[i]);
                    tis[0].pin_out("DOPA3", &do_[i]);
                }
                30 => {
                    tis[0].pin_in("DIPB0", &di[i]);
                    tis[0].pin_out("DOPA0", &do_[i]);
                }
                31 => {
                    tis[0].pin_in("DIPB2", &di[i]);
                    tis[0].pin_out("DOPA2", &do_[i]);
                }
                _ => {
                    tis[0].pin_in(&format!("DIB{i}"), &di[i]);
                    tis[0].pin_out(&format!("DOA{i}"), &do_[i]);
                }
            }
        }
        for i in 0..32 {
            match i {
                0 => {
                    tis[1].pin_in("DIPB0", &di[i + 32]);
                    tis[1].pin_out("DOPA0", &do_[i + 32]);
                }
                1 => {
                    tis[1].pin_in("DIPB2", &di[i + 32]);
                    tis[1].pin_out("DOPA2", &do_[i + 32]);
                }
                16 => {
                    tis[1].pin_in("DIPB1", &di[i + 32]);
                    tis[1].pin_out("DOPA1", &do_[i + 32]);
                }
                17 => {
                    tis[1].pin_in("DIPB3", &di[i + 32]);
                    tis[1].pin_out("DOPA3", &do_[i + 32]);
                }
                _ => {
                    tis[1].pin_in(&format!("DIB{i}"), &di[i + 32]);
                    tis[1].pin_out(&format!("DOA{i}"), &do_[i + 32]);
                }
            }
        }
        tis[0].pin_tie("DIB14", false);
        tis[0].pin_tie("DIB15", false);
        tis[0].pin_tie("DIB30", false);
        tis[0].pin_tie("DIB31", false);
        tis[1].pin_tie("DIB0", false);
        tis[1].pin_tie("DIB1", false);
        tis[1].pin_tie("DIB16", false);
        tis[1].pin_tie("DIB17", false);
        tis[0].pin_out("DOA31", &status[0]);
        tis[1].pin_out("DOA0", &status[1]);

        for ti in tis {
            test.tgt_insts.push(ti);
        }
    } else {
        let (ssr_v, ssr_x, ssr_inv) = test.make_in_inv(ctx);
        inst.connect("SSR", &ssr_v);
        let mut ti = TgtInst::new(&[if mode == Mode::Virtex5 {
            "RAMB36SDP_EXP"
        } else {
            "RAMB36E1"
        }]);
        for i in 0..128 {
            ti.cfg(&format!("INIT_{i:02X}"), ZERO_INIT);
        }
        for i in 0..16 {
            ti.cfg(&format!("INITP_{i:02X}"), ZERO_INIT);
        }
        if mode == Mode::Virtex5 {
            ti.bel("RAMB36SDP_EXP", &inst.name, "");
            ti.cfg_int("DO_REG", 1);
            // what.
            ti.cfg("EN_ECC_READ", "FALSE");
            ti.cfg("EN_ECC_WRITE", "FALSE");
            ti.cfg("EN_ECC_SCRUB", "FALSE");
            ti.cfg("INIT", "000000000000000000");
            ti.cfg("SRVAL", "000000000000000000");
            ti.cfg("SAVEDATA", "FALSE");
            ti.pin_in_inv("RDCLKL", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("RDCLKU", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("RDRCLKL", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("RDRCLKU", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("WRCLKL", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("WRCLKU", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("RDENL", &rden_x, rden_inv);
            ti.pin_in_inv("RDENU", &rden_x, rden_inv);
            ti.pin_in_inv("WRENL", &wren_x, wren_inv);
            ti.pin_in_inv("WRENU", &wren_x, wren_inv);
            ti.pin_in_inv("SSRL", &ssr_x, ssr_inv);
            ti.pin_in_inv("SSRU", &ssr_x, ssr_inv);
            ti.pin_tie("TIEOFFSSRBL", false);
            ti.pin_tie("TIEOFFSSRBU", false);
            for i in 0..4 {
                ti.pin_tie(&format!("TIEOFFWEAL{i}"), false);
                ti.pin_tie(&format!("TIEOFFWEAU{i}"), false);
            }
            for i in 0..8 {
                ti.pin_tie(&format!("WEL{i}"), true);
                ti.pin_tie(&format!("WEU{i}"), true);
            }
            for i in 0..9 {
                ti.pin_in(&format!("RDADDRL{}", i + 6), &rdaddr[i]);
                ti.pin_in(&format!("RDADDRU{}", i + 6), &rdaddr[i]);
                ti.pin_in(&format!("WRADDRL{}", i + 6), &wraddr[i]);
                ti.pin_in(&format!("WRADDRU{}", i + 6), &wraddr[i]);
            }
            ti.pin_tie("RDADDRL15", true);
            ti.pin_tie("WRADDRL15", true);
            ti.pin_tie("REGCEL", true);
            ti.pin_tie("REGCEU", true);
            for i in 0..64 {
                ti.pin_in(&format!("DI{i}"), &di[i]);
                ti.pin_out(&format!("DO{i}"), &do_[i]);
            }
            ti.pin_out("SBITERR", &status[0]);
            ti.pin_out("DBITERR", &status[1]);
        } else {
            ti.bel("RAMB36E1", &inst.name, "");
            ti.cfg_int("DOA_REG", 1);
            ti.cfg_int("DOB_REG", 1);
            ti.cfg("EN_ECC_READ", "FALSE");
            ti.cfg("EN_ECC_WRITE", "FALSE");
            ti.cfg("INIT_A", "000000000000000000");
            ti.cfg("INIT_B", "000000000000000000");
            ti.cfg("SRVAL_A", "000000000000000000");
            ti.cfg("SRVAL_B", "000000000000000000");
            ti.cfg("WRITE_MODE_A", "READ_FIRST");
            ti.cfg("WRITE_MODE_B", "READ_FIRST");
            ti.cfg("WRITE_WIDTH_A", "0");
            ti.cfg("WRITE_WIDTH_B", "72");
            ti.cfg("READ_WIDTH_A", "72");
            ti.cfg("READ_WIDTH_B", "0");
            ti.cfg("SAVEDATA", "FALSE");
            ti.cfg("RAM_MODE", "SDP");
            ti.cfg("RSTREG_PRIORITY_A", "REGCE");
            ti.cfg("RSTREG_PRIORITY_B", "REGCE");
            ti.cfg("RAM_EXTENSION_A", "NONE");
            ti.cfg("RAM_EXTENSION_B", "NONE");
            ti.cfg("RDADDR_COLLISION_HWCONFIG", "DELAYED_WRITE");
            if mode == Mode::Series7 {
                ti.cfg("EN_PWRGATE", "NONE");
            }
            ti.pin_tie("REGCEAREGCEL", true);
            ti.pin_tie("REGCEAREGCEU", true);
            ti.pin_tie("REGCEBL", true);
            ti.pin_tie("REGCEBU", true);
            ti.pin_in_inv("CLKARDCLKL", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("CLKARDCLKU", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("CLKBWRCLKL", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("CLKBWRCLKU", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("ENARDENL", &rden_x, rden_inv);
            ti.pin_in_inv("ENARDENU", &rden_x, rden_inv);
            ti.pin_in_inv("ENBWRENL", &wren_x, wren_inv);
            ti.pin_in_inv("ENBWRENU", &wren_x, wren_inv);
            ti.pin_in_inv("RSTREGARSTREGL", &ssr_x, ssr_inv);
            ti.pin_in_inv("RSTREGARSTREGU", &ssr_x, ssr_inv);
            ti.pin_tie_inv("RSTRAMARSTRAML", true, true);
            ti.pin_tie_inv("RSTRAMARSTRAMU", true, true);
            ti.pin_tie_inv("RSTRAMBL", true, true);
            ti.pin_tie_inv("RSTRAMBU", true, true);
            ti.pin_tie_inv("RSTREGBL", false, false);
            ti.pin_tie_inv("RSTREGBU", false, false);
            ti.pin_tie_inv("REGCLKARDRCLKL", true, true);
            ti.pin_tie_inv("REGCLKARDRCLKU", true, true);
            ti.pin_tie_inv("REGCLKBL", false, false);
            ti.pin_tie_inv("REGCLKBU", false, false);
            for i in 0..4 {
                ti.pin_tie(&format!("WEAL{i}"), false);
                ti.pin_tie(&format!("WEAU{i}"), false);
            }
            for i in 0..8 {
                ti.pin_tie(&format!("WEBWEL{i}"), true);
                ti.pin_tie(&format!("WEBWEU{i}"), true);
            }
            for i in 0..6 {
                ti.pin_tie(&format!("ADDRARDADDRL{i}"), true);
                ti.pin_tie(&format!("ADDRARDADDRU{i}"), true);
                ti.pin_tie(&format!("ADDRBWRADDRL{i}"), true);
                ti.pin_tie(&format!("ADDRBWRADDRU{i}"), true);
            }
            for i in 0..9 {
                ti.pin_in(&format!("ADDRARDADDRL{}", i + 6), &rdaddr[i]);
                ti.pin_in(&format!("ADDRARDADDRU{}", i + 6), &rdaddr[i]);
                ti.pin_in(&format!("ADDRBWRADDRL{}", i + 6), &wraddr[i]);
                ti.pin_in(&format!("ADDRBWRADDRU{}", i + 6), &wraddr[i]);
            }
            ti.pin_tie("ADDRARDADDRL15", true);
            ti.pin_tie("ADDRBWRADDRL15", true);
            for i in 0..32 {
                ti.pin_in(&format!("DIADI{i}"), &di[i]);
                ti.pin_in(&format!("DIBDI{i}"), &di[i + 32]);
                ti.pin_out(&format!("DOADO{i}"), &do_[i]);
                ti.pin_out(&format!("DOBDO{i}"), &do_[i + 32]);
            }
        }
        test.tgt_insts.push(ti);
    }

    test.src_insts.push(inst);
}

fn gen_ramb18(test: &mut Test, ctx: &mut TestGenCtx) {
    let (ar_en, br_en) = *[(true, true), (false, true), (true, false)]
        .choose(&mut ctx.rng)
        .unwrap();
    let aw_en = ctx.rng.gen();
    let bw_en = ctx.rng.gen();
    let arwlog2 = ctx.rng.gen_range(0..5);
    let awwlog2 = ctx.rng.gen_range(0..5);
    let brwlog2 = ctx.rng.gen_range(0..5);
    let bwwlog2 = ctx.rng.gen_range(0..5);
    let arw = if ar_en { WIDTHS[arwlog2] } else { 0 };
    let aww = if aw_en { WIDTHS[awwlog2] } else { 0 };
    let brw = if br_en { WIDTHS[brwlog2] } else { 0 };
    let bww = if bw_en { WIDTHS[bwwlog2] } else { 0 };

    let is_18 = ctx.rng.gen();

    let mut inst = SrcInst::new(ctx, if is_18 { "RAMB18" } else { "RAMB16" });
    let mut ti = TgtInst::new(&["RAMB18X2"]);
    let ul = if ctx.rng.gen() { "U" } else { "L" };
    if ul == "U" {
        ti.bel("RAMB18X2_UPPER", &inst.name, "");
        inst.attr_str("BEL", "UPPER");
    } else {
        ti.bel("RAMB18X2_LOWER", &inst.name, "");
        inst.attr_str("BEL", "LOWER");
    }

    for i in 0..64 {
        let init = ctx.gen_bits(256);
        inst.param_bits(&format!("INIT_{i:02X}"), &init);
        ti.cfg_hex(&format!("INIT_{i:02X}_{ul}"), &init, true);
    }
    for i in 0..8 {
        let init = ctx.gen_bits(256);
        inst.param_bits(&format!("INITP_{i:02X}"), &init);
        ti.cfg_hex(&format!("INITP_{i:02X}_{ul}"), &init, true);
    }

    inst.param_int("READ_WIDTH_A", arw);
    inst.param_int("READ_WIDTH_B", brw);
    inst.param_int("WRITE_WIDTH_A", aww);
    inst.param_int("WRITE_WIDTH_B", bww);

    ti.cfg_int(&format!("READ_WIDTH_A_{ul}"), arw);
    ti.cfg_int(&format!("READ_WIDTH_B_{ul}"), brw);
    ti.cfg_int(&format!("WRITE_WIDTH_A_{ul}"), aww);
    ti.cfg_int(&format!("WRITE_WIDTH_B_{ul}"), bww);

    for (l, r_en, w_en, rwlog2, wwlog2) in [
        ('A', ar_en, aw_en, arwlog2, awwlog2),
        ('B', br_en, bw_en, brwlog2, bwwlog2),
    ] {
        if r_en {
            let do_ = test.make_outs(ctx, 1 << rwlog2);
            inst.connect_bus(&format!("DO{l}"), &do_);
            for (i, w) in do_.iter().enumerate() {
                ti.pin_out(&format!("DO{l}{ul}{i}"), w);
            }
            if rwlog2 >= 3 {
                let dop = test.make_outs(ctx, 1 << rwlog2 - 3);
                inst.connect_bus(&format!("DOP{l}"), &dop);
                for (i, w) in dop.iter().enumerate() {
                    ti.pin_out(&format!("DOP{l}{ul}{i}"), w);
                }
            }
        }
        let di = test.make_ins(ctx, 18);
        let dip = test.make_ins(ctx, 2);
        inst.connect_bus(&format!("DI{l}"), &di);
        inst.connect_bus(&format!("DIP{l}"), &dip);
        if is_18 {
            if w_en {
                for i in 0..(1 << wwlog2) {
                    ti.pin_in(&format!("DI{l}{ul}{i}"), &di[i]);
                }
                if wwlog2 >= 3 {
                    for i in 0..(1 << wwlog2 - 3) {
                        ti.pin_in(&format!("DIP{l}{ul}{i}"), &dip[i]);
                    }
                }
            }
        } else {
            for i in 0..16 {
                ti.pin_in(&format!("DI{l}{ul}{i}"), &di[i]);
            }
            for i in 0..2 {
                ti.pin_in(&format!("DIP{l}{ul}{i}"), &dip[i]);
            }
        }

        let do_reg = ctx.rng.gen_range(0..2);
        inst.param_int(&format!("DO{l}_REG"), do_reg);
        ti.cfg_int(&format!("DO{l}_REG_{ul}"), do_reg);

        let invert_do_reg = !is_18 && do_reg == 1 && ctx.rng.gen();
        if !is_18 {
            inst.param_str(
                &format!("INVERT_CLK_DO{l}_REG"),
                if invert_do_reg { "TRUE" } else { "FALSE" },
            );
        }

        let wmode = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]
            .choose(&mut ctx.rng)
            .unwrap();
        inst.param_str(&format!("WRITE_MODE_{l}"), wmode);
        ti.cfg(&format!("WRITE_MODE_{l}_{ul}"), wmode);

        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        inst.connect(&format!("CLK{l}"), &clk_v);
        ti.pin_in_inv(&format!("CLK{l}{ul}"), &clk_x, clk_inv);
        ti.pin_in_inv(&format!("REGCLK{l}{ul}"), &clk_x, clk_inv);

        let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
        inst.connect(&format!("EN{l}"), &en_v);
        ti.pin_in_inv(&format!("EN{l}{ul}"), &en_x, en_inv);

        let (ssr_v, ssr_x, ssr_inv) = test.make_in_inv(ctx);
        inst.connect(&format!("SSR{l}"), &ssr_v);
        ti.pin_in_inv(&format!("SSR{l}{ul}"), &ssr_x, ssr_inv);

        let regce = test.make_in(ctx);
        inst.connect(&format!("REGCE{l}"), &regce);
        ti.pin_in(&format!("REGCE{l}{ul}"), &regce);

        let addr = test.make_ins(ctx, 14);
        inst.connect_bus(&format!("ADDR{l}"), &addr);
        for i in 0..14 {
            if (i < rwlog2 || !r_en) && (i < wwlog2 || !w_en) && is_18 {
                continue;
            }
            ti.pin_in(&format!("ADDR{l}{ul}{k}", k = i + 1), &addr[i]);
        }
        if ul == "L" {
            ti.pin_tie(&format!("ADDR{l}L15"), true);
        }

        let mut init = ctx.gen_bits(18);
        let mut srval = ctx.gen_bits(18);
        inst.param_bits(&format!("INIT_{l}"), &init);
        inst.param_bits(&format!("SRVAL_{l}"), &srval);
        if !is_18 {
            for _ in 18..36 {
                init.push(BitVal::S0);
                srval.push(BitVal::S0);
            }
        }
        ti.cfg_hex(&format!("INIT_{l}_{ul}"), &init, true);
        ti.cfg_hex(&format!("SRVAL_{l}_{ul}"), &srval, true);

        let mut we = Vec::new();
        match wwlog2 {
            4 => {
                for i in 0..2 {
                    let w = test.make_in(ctx);
                    ti.pin_in(&format!("WE{l}{ul}{k}", k = i * 2), &w);
                    ti.pin_in(&format!("WE{l}{ul}{k}", k = i * 2 + 1), &w);
                    we.push(w);
                }
            }
            _ => {
                let w = test.make_in(ctx);
                for j in 0..4 {
                    ti.pin_in(&format!("WE{l}{ul}{j}"), &w);
                }
                we.push(w.clone());
                we.push(w);
            }
        }
        inst.connect_bus(&format!("WE{l}"), &we);
    }

    ti.cfg(&format!("SAVEDATA_{ul}"), "FALSE");

    for i in 4..8 {
        ti.pin_tie(&format!("WEB{ul}{i}"), false);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ramb36(test: &mut Test, ctx: &mut TestGenCtx, num: usize) {
    let mut insts = Vec::new();
    let mut tis = Vec::new();
    for _ in 0..num {
        let (ar_en, br_en) = *[(true, true), (false, true), (true, false)]
            .choose(&mut ctx.rng)
            .unwrap();
        let aw_en = ctx.rng.gen();
        let bw_en = ctx.rng.gen();
        let arwlog2 = ctx.rng.gen_range(0..6);
        let awwlog2 = ctx.rng.gen_range(0..6);
        let brwlog2 = ctx.rng.gen_range(0..6);
        let bwwlog2 = ctx.rng.gen_range(0..6);
        let arw = if ar_en { WIDTHS[arwlog2] } else { 0 };
        let aww = if aw_en { WIDTHS[awwlog2] } else { 0 };
        let brw = if br_en { WIDTHS[brwlog2] } else { 0 };
        let bww = if bw_en { WIDTHS[bwwlog2] } else { 0 };

        let mut inst = SrcInst::new(ctx, "RAMB36");
        let mut ti = TgtInst::new(&["RAMB36_EXP"]);
        ti.bel("RAMB36_EXP", &inst.name, "");

        for i in 0..128 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
        }
        for i in 0..16 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INITP_{i:02X}"), &init);
            ti.cfg_hex(&format!("INITP_{i:02X}"), &init, true);
        }

        inst.param_int("READ_WIDTH_A", arw);
        inst.param_int("READ_WIDTH_B", brw);
        inst.param_int("WRITE_WIDTH_A", aww);
        inst.param_int("WRITE_WIDTH_B", bww);

        ti.cfg_int("READ_WIDTH_A", arw);
        ti.cfg_int("READ_WIDTH_B", brw);
        ti.cfg_int("WRITE_WIDTH_A", aww);
        ti.cfg_int("WRITE_WIDTH_B", bww);

        for (l, r_en, w_en, rwlog2, wwlog2) in [
            ('A', ar_en, aw_en, arwlog2, awwlog2),
            ('B', br_en, bw_en, brwlog2, bwwlog2),
        ] {
            if r_en {
                let do_ = test.make_outs(ctx, 1 << rwlog2);
                inst.connect_bus(&format!("DO{l}"), &do_);
                for (i, w) in do_.iter().enumerate() {
                    ti.pin_out(&format!("DO{l}{i}"), w);
                }
                if rwlog2 >= 3 {
                    let dop = test.make_outs(ctx, 1 << rwlog2 - 3);
                    inst.connect_bus(&format!("DOP{l}"), &dop);
                    for (i, w) in dop.iter().enumerate() {
                        ti.pin_out(&format!("DOP{l}{i}"), w);
                    }
                }
            }
            let di = test.make_ins(ctx, 36);
            let dip = test.make_ins(ctx, 4);
            inst.connect_bus(&format!("DI{l}"), &di);
            inst.connect_bus(&format!("DIP{l}"), &dip);
            if w_en {
                for i in 0..(1 << wwlog2) {
                    ti.pin_in(&format!("DI{l}{i}"), &di[i]);
                }
                if wwlog2 == 0 {
                    ti.pin_in(&format!("DI{l}1"), &di[0]);
                }
                if wwlog2 >= 3 {
                    for i in 0..(1 << wwlog2 - 3) {
                        ti.pin_in(&format!("DIP{l}{i}"), &dip[i]);
                    }
                }
                if wwlog2 == 3 {
                    ti.pin_in(&format!("DIP{l}1"), &dip[0]);
                }
            }

            let do_reg = ctx.rng.gen_range(0..2);
            inst.param_int(&format!("DO{l}_REG"), do_reg);
            ti.cfg_int(&format!("DO{l}_REG"), do_reg);

            let wmode = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]
                .choose(&mut ctx.rng)
                .unwrap();
            inst.param_str(&format!("WRITE_MODE_{l}"), wmode);
            ti.cfg(&format!("WRITE_MODE_{l}"), wmode);

            let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
            inst.connect(&format!("CLK{l}"), &clk_v);
            for ul in ['U', 'L'] {
                ti.pin_in_inv(&format!("CLK{l}{ul}"), &clk_x, clk_inv);
                if do_reg == 1 {
                    ti.pin_in_inv(&format!("REGCLK{l}{ul}"), &clk_x, clk_inv);
                } else {
                    ti.pin_tie_inv(&format!("REGCLK{l}{ul}"), true, true);
                }
            }

            let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
            inst.connect(&format!("EN{l}"), &en_v);
            for ul in ['U', 'L'] {
                ti.pin_in_inv(&format!("EN{l}{ul}"), &en_x, en_inv);
            }

            let (ssr_v, ssr_x, ssr_inv) = test.make_in_inv(ctx);
            inst.connect(&format!("SSR{l}"), &ssr_v);
            for ul in ['U', 'L'] {
                ti.pin_in_inv(&format!("SSR{l}{ul}"), &ssr_x, ssr_inv);
            }

            let regce = test.make_in(ctx);
            inst.connect(&format!("REGCE{l}"), &regce);
            for ul in ['U', 'L'] {
                ti.pin_in(&format!("REGCE{l}{ul}"), &regce);
            }

            let addr = test.make_ins(ctx, 16);
            inst.connect_bus(&format!("ADDR{l}"), &addr);
            for ul in ['U', 'L'] {
                for i in 0..(if num == 1 || ul == 'U' { 15 } else { 16 }) {
                    if (i < rwlog2 || !r_en) && (i < wwlog2 || !w_en) && i != 15 {
                        continue;
                    }
                    ti.pin_in(&format!("ADDR{l}{ul}{i}"), &addr[i]);
                }
            }
            if num == 1 {
                ti.pin_tie(&format!("ADDR{l}L15"), true);
            }

            let init = ctx.gen_bits(36);
            let srval = ctx.gen_bits(36);
            inst.param_bits(&format!("INIT_{l}"), &init);
            inst.param_bits(&format!("SRVAL_{l}"), &srval);
            ti.cfg_hex(&format!("INIT_{l}"), &init, true);
            ti.cfg_hex(&format!("SRVAL_{l}"), &srval, true);

            let mut we = Vec::new();
            match wwlog2 {
                5 => {
                    for i in 0..4 {
                        let w = test.make_in(ctx);
                        for ul in ['U', 'L'] {
                            ti.pin_in(&format!("WE{l}{ul}{i}"), &w);
                        }
                        we.push(w);
                    }
                }
                4 => {
                    for i in 0..2 {
                        let w = test.make_in(ctx);
                        for ul in ['U', 'L'] {
                            ti.pin_in(&format!("WE{l}{ul}{k}", k = i), &w);
                            ti.pin_in(&format!("WE{l}{ul}{k}", k = i + 2), &w);
                        }
                        we.push(w);
                    }
                    let we0 = we[0].clone();
                    let we1 = we[1].clone();
                    we.push(we0);
                    we.push(we1);
                }
                _ => {
                    let w = test.make_in(ctx);
                    for j in 0..4 {
                        for ul in ['U', 'L'] {
                            ti.pin_in(&format!("WE{l}{ul}{j}"), &w);
                        }
                    }
                    we.push(w.clone());
                    we.push(w.clone());
                    we.push(w.clone());
                    we.push(w);
                }
            }
            inst.connect_bus(&format!("WE{l}"), &we);
        }

        ti.cfg("SAVEDATA", "FALSE");

        for ul in ['U', 'L'] {
            for i in 4..8 {
                ti.pin_tie(&format!("WEB{ul}{i}"), false);
                ti.pin_tie(&format!("WEB{ul}{i}"), false);
            }
        }
        insts.push(inst);
        tis.push(ti);
    }

    for l in ['A', 'B'] {
        if num == 2 {
            let cl = test.make_wire(ctx);
            let cr = test.make_wire(ctx);
            insts[0].connect(&format!("CASCADEOUTLAT{l}"), &cl);
            insts[1].connect(&format!("CASCADEINLAT{l}"), &cl);
            tis[0].pin_out(&format!("CASCADEOUTLAT{l}"), &cl);
            tis[1].pin_in(&format!("CASCADEINLAT{l}"), &cl);
            insts[0].connect(&format!("CASCADEOUTREG{l}"), &cr);
            insts[1].connect(&format!("CASCADEINREG{l}"), &cr);
            tis[0].pin_out(&format!("CASCADEOUTREG{l}"), &cr);
            tis[1].pin_in(&format!("CASCADEINREG{l}"), &cr);
            insts[0].param_str(&format!("RAM_EXTENSION_{l}"), "LOWER");
            insts[1].param_str(&format!("RAM_EXTENSION_{l}"), "UPPER");
            tis[0].cfg(&format!("RAM_EXTENSION_{l}"), "LOWER");
            tis[1].cfg(&format!("RAM_EXTENSION_{l}"), "UPPER");
        } else {
            insts[0].param_str(&format!("RAM_EXTENSION_{l}"), "NONE");
            tis[0].cfg(&format!("RAM_EXTENSION_{l}"), "NONE");
        }
    }

    for inst in insts {
        test.src_insts.push(inst);
    }
    for ti in tis {
        test.tgt_insts.push(ti);
    }
}

fn gen_ramb18sdp(test: &mut Test, ctx: &mut TestGenCtx) {
    let is_18 = ctx.rng.gen();

    let mut inst = SrcInst::new(ctx, if is_18 { "RAMB18SDP" } else { "RAMB16" });
    let mut ti = TgtInst::new(&["RAMB18X2SDP", "RAMBFIFO18_36"]);
    let ul = if ctx.rng.gen() { "U" } else { "L" };
    if ul == "U" {
        ti.cond_bel("RAMB18X2SDP_UPPER", &inst.name, "", "RAMB18X2SDP");
        ti.cond_bel("RAMBFIFO18_36_UPPER", &inst.name, "", "RAMBFIFO18_36");
        inst.attr_str("BEL", "UPPER");
    } else {
        ti.cond_bel("RAMB18X2SDP_LOWER", &inst.name, "", "RAMB18X2SDP");
        inst.attr_str("BEL", "LOWER");
    }

    for i in 0..64 {
        let init = ctx.gen_bits(256);
        inst.param_bits(&format!("INIT_{i:02X}"), &init);
        ti.cond_cfg_hex(&format!("INIT_{i:02X}_{ul}"), &init, true, "RAMB18X2SDP");
        ti.cond_cfg_hex(&format!("INIT_{i:02X}"), &init, true, "RAMBFIFO18_36");
    }
    for i in 0..8 {
        let init = ctx.gen_bits(256);
        inst.param_bits(&format!("INITP_{i:02X}"), &init);
        ti.cond_cfg_hex(&format!("INITP_{i:02X}_{ul}"), &init, true, "RAMB18X2SDP");
        ti.cond_cfg_hex(&format!("INITP_{i:02X}"), &init, true, "RAMBFIFO18_36");
    }

    if !is_18 {
        inst.param_int("READ_WIDTH_A", 36);
        inst.param_int("READ_WIDTH_B", 0);
        inst.param_int("WRITE_WIDTH_A", 0);
        inst.param_int("WRITE_WIDTH_B", 36);
    }

    let do_ = test.make_outs(ctx, 32);
    let dop = test.make_outs(ctx, 4);
    let di = test.make_ins(ctx, 32);
    let dip = test.make_ins(ctx, 4);
    if is_18 {
        inst.connect_bus("DO", &do_);
        inst.connect_bus("DOP", &dop);
        inst.connect_bus("DI", &di);
        inst.connect_bus("DIP", &dip);
    } else {
        inst.connect_bus("DOA", &do_);
        inst.connect_bus("DOPA", &dop);
        inst.connect_bus("DIB", &di);
        inst.connect_bus("DIPB", &dip);
    }
    for (i, w) in do_.iter().enumerate() {
        ti.pin_out(&format!("DO{ul}{i}"), w);
    }
    for (i, w) in dop.iter().enumerate() {
        ti.pin_out(&format!("DOP{ul}{i}"), w);
    }
    for i in 0..32 {
        ti.pin_in(&format!("DI{ul}{i}"), &di[i]);
    }
    for i in 0..4 {
        ti.pin_in(&format!("DIP{ul}{i}"), &dip[i]);
    }

    let do_reg = ctx.rng.gen_range(0..2);
    if !is_18 {
        inst.param_int("DOA_REG", do_reg);
    } else {
        inst.param_int("DO_REG", do_reg);
    }
    ti.cfg_int(&format!("DO_REG_{ul}"), do_reg);

    let invert_do_reg = !is_18 && do_reg == 1 && ctx.rng.gen();
    if !is_18 {
        inst.param_str(
            &format!("INVERT_CLK_DOB_REG"),
            if invert_do_reg { "TRUE" } else { "FALSE" },
        );
    }

    for (l, rw) in [("A", "RD"), ("B", "WR")] {
        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        if !is_18 {
            inst.connect(&format!("CLK{l}"), &clk_v);
        } else {
            inst.connect(&format!("{rw}CLK"), &clk_v);
        }
        ti.pin_in_inv(&format!("{rw}CLK{ul}"), &clk_x, clk_inv);
        if rw == "RD" {
            ti.pin_in_inv(&format!("RDRCLK{ul}"), &clk_x, clk_inv);
        }

        let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
        if !is_18 {
            inst.connect(&format!("EN{l}"), &en_v);
        } else {
            inst.connect(&format!("{rw}EN"), &en_v);
        }
        ti.pin_in_inv(&format!("{rw}EN{ul}"), &en_x, en_inv);

        if !is_18 {
            let addr = test.make_ins(ctx, 14);
            inst.connect_bus(&format!("ADDR{l}"), &addr);
            for i in 5..14 {
                ti.pin_in(&format!("{rw}ADDR{ul}{k}", k = i + 1), &addr[i]);
            }
        } else {
            let addr = test.make_ins(ctx, 9);
            inst.connect_bus(&format!("{rw}ADDR"), &addr);
            for i in 5..14 {
                ti.pin_in(&format!("{rw}ADDR{ul}{k}", k = i + 1), &addr[i - 5]);
            }
        }
        if ul == "L" {
            ti.pin_tie(&format!("{rw}ADDRL15"), true);
        }
    }

    let init = ctx.gen_bits(36);
    let srval = ctx.gen_bits(36);
    if !is_18 {
        inst.param_bits("INIT_A", &init);
        inst.param_bits("SRVAL_A", &srval);
    } else {
        inst.param_bits("INIT", &init);
        inst.param_bits("SRVAL", &srval);
    }
    ti.cond_cfg_hex(&format!("INIT_{ul}"), &init, true, "RAMB18X2SDP");
    ti.cond_cfg_hex(&format!("SRVAL_{ul}"), &srval, true, "RAMB18X2SDP");
    ti.cond_cfg_hex("INIT", &init, true, "RAMBFIFO18_36");
    ti.cond_cfg_hex("SRVAL", &srval, true, "RAMBFIFO18_36");

    ti.cond_cfg(&format!("SAVEDATA_{ul}"), "FALSE", "RAMB18X2SDP");
    ti.cond_cfg("SAVEDATA", "FALSE", "RAMBFIFO18_36");

    let (ssr_v, ssr_x, ssr_inv) = test.make_in_inv(ctx);
    if !is_18 {
        inst.connect("SSRA", &ssr_v);
    } else {
        inst.connect("SSR", &ssr_v);
    }
    ti.pin_in_inv(&format!("SSR{ul}"), &ssr_x, ssr_inv);
    ti.pin_tie(&format!("TIEOFFSSRB{ul}"), false);

    let regce = test.make_in(ctx);
    if !is_18 {
        inst.connect("REGCEA", &regce);
    } else {
        inst.connect("REGCE", &regce);
    }
    ti.pin_in(&format!("REGCE{ul}"), &regce);

    let we = test.make_ins(ctx, 4);
    if !is_18 {
        inst.connect_bus("WEB", &we);
    } else {
        inst.connect_bus("WE", &we);
    }
    for i in 0..8 {
        ti.pin_in(&format!("WE{ul}{i}"), &we[i / 2]);
    }
    for i in 0..4 {
        ti.pin_tie(&format!("TIEOFFWEA{ul}{i}"), false);
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ramb36sdp(test: &mut Test, ctx: &mut TestGenCtx) {
    let mut inst = SrcInst::new(ctx, "RAMB36SDP");
    let mut ti = TgtInst::new(&["RAMB36SDP_EXP"]);
    ti.bel("RAMB36SDP_EXP", &inst.name, "");

    for i in 0..128 {
        let init = ctx.gen_bits(256);
        inst.param_bits(&format!("INIT_{i:02X}"), &init);
        ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
    }
    for i in 0..16 {
        let init = ctx.gen_bits(256);
        inst.param_bits(&format!("INITP_{i:02X}"), &init);
        ti.cfg_hex(&format!("INITP_{i:02X}"), &init, true);
    }

    let do_ = test.make_outs(ctx, 64);
    let dop = test.make_outs(ctx, 8);
    let di = test.make_ins(ctx, 64);
    let dip = test.make_ins(ctx, 8);
    inst.connect_bus("DO", &do_);
    inst.connect_bus("DOP", &dop);
    inst.connect_bus("DI", &di);
    inst.connect_bus("DIP", &dip);
    for (i, w) in do_.iter().enumerate() {
        ti.pin_out(&format!("DO{i}"), w);
    }
    for (i, w) in dop.iter().enumerate() {
        ti.pin_out(&format!("DOP{i}"), w);
    }
    for i in 0..64 {
        ti.pin_in(&format!("DI{i}"), &di[i]);
    }
    for i in 0..8 {
        ti.pin_in(&format!("DIP{i}"), &dip[i]);
    }

    let do_reg = ctx.rng.gen_range(0..2);
    inst.param_int("DO_REG", do_reg);
    ti.cfg_int("DO_REG", do_reg);

    for p in ["EN_ECC_READ", "EN_ECC_WRITE"] {
        let v = if ctx.rng.gen() { "TRUE" } else { "FALSE" };
        inst.param_str(p, v);
        ti.cfg(p, v);
    }
    ti.cfg("EN_ECC_SCRUB", "FALSE");

    for o in ["SBITERR", "DBITERR"] {
        let w = test.make_out(ctx);
        inst.connect(o, &w);
        ti.pin_out(o, &w);
    }
    let eccparity = test.make_outs(ctx, 8);
    inst.connect_bus("ECCPARITY", &eccparity);
    for i in 0..8 {
        ti.pin_out(&format!("ECCPARITY{i}"), &eccparity[i]);
    }

    for rw in ["RD", "WR"] {
        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        inst.connect(&format!("{rw}CLK"), &clk_v);
        for ul in ['U', 'L'] {
            ti.pin_in_inv(&format!("{rw}CLK{ul}"), &clk_x, clk_inv);
            if rw == "RD" {
                if do_reg == 1 {
                    ti.pin_in_inv(&format!("RDRCLK{ul}"), &clk_x, clk_inv);
                } else {
                    ti.pin_tie_inv(&format!("RDRCLK{ul}"), true, true);
                }
            }
        }

        let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
        inst.connect(&format!("{rw}EN"), &en_v);
        for ul in ['U', 'L'] {
            ti.pin_in_inv(&format!("{rw}EN{ul}"), &en_x, en_inv);
        }

        let addr = test.make_ins(ctx, 9);
        inst.connect_bus(&format!("{rw}ADDR"), &addr);
        for i in 6..15 {
            for ul in ['U', 'L'] {
                ti.pin_in(&format!("{rw}ADDR{ul}{i}"), &addr[i - 6]);
            }
        }
        ti.pin_tie(&format!("{rw}ADDRL15"), true);
    }

    let init = ctx.gen_bits(72);
    let srval = ctx.gen_bits(72);
    inst.param_bits("INIT", &init);
    inst.param_bits("SRVAL", &srval);
    ti.cfg_hex("INIT", &init, true);
    ti.cfg_hex("SRVAL", &srval, true);

    ti.cfg("SAVEDATA", "FALSE");

    let (ssr_v, ssr_x, ssr_inv) = test.make_in_inv(ctx);
    inst.connect("SSR", &ssr_v);
    for ul in ['U', 'L'] {
        ti.pin_in_inv(&format!("SSR{ul}"), &ssr_x, ssr_inv);
        ti.pin_tie(&format!("TIEOFFSSRB{ul}"), false);
    }

    let regce = test.make_in(ctx);
    inst.connect("REGCE", &regce);
    for ul in ['U', 'L'] {
        ti.pin_in(&format!("REGCE{ul}"), &regce);
    }

    let we = test.make_ins(ctx, 8);
    inst.connect_bus("WE", &we);
    for ul in ['U', 'L'] {
        for i in 0..8 {
            ti.pin_in(&format!("WE{ul}{i}"), &we[i]);
        }
        for i in 0..4 {
            ti.pin_tie(&format!("TIEOFFWEA{ul}{i}"), false);
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ramb18e1(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode) {
    let is_sdp = ctx.rng.gen();
    let (ar_en, br_en) = *[(true, true), (false, true), (true, false)]
        .choose(&mut ctx.rng)
        .unwrap();
    let aw_en = ctx.rng.gen();
    let bw_en = ctx.rng.gen();
    let arwlog2 = ctx.rng.gen_range(0..6);
    let awwlog2 = ctx.rng.gen_range(0..5);
    let brwlog2 = ctx.rng.gen_range(0..5);
    let bwwlog2 = ctx.rng.gen_range(0..6);
    let arw = if ar_en { WIDTHS[arwlog2] } else { 0 };
    let aww = if aw_en { WIDTHS[awwlog2] } else { 0 };
    let brw = if br_en { WIDTHS[brwlog2] } else { 0 };
    let bww = if bw_en { WIDTHS[bwwlog2] } else { 0 };

    let mut inst = SrcInst::new(ctx, "RAMB18E1");
    let mut ti = TgtInst::new(&["RAMB18E1"]);
    ti.bel("RAMB18E1", &inst.name, "");

    for i in 0..64 {
        let init = ctx.gen_bits(256);
        inst.param_bits(&format!("INIT_{i:02X}"), &init);
        ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
    }
    for i in 0..8 {
        let init = ctx.gen_bits(256);
        inst.param_bits(&format!("INITP_{i:02X}"), &init);
        ti.cfg_hex(&format!("INITP_{i:02X}"), &init, true);
    }

    inst.param_int("READ_WIDTH_A", arw);
    inst.param_int("READ_WIDTH_B", brw);
    inst.param_int("WRITE_WIDTH_A", aww);
    inst.param_int("WRITE_WIDTH_B", bww);

    ti.cfg_int("READ_WIDTH_A", arw);
    ti.cfg_int("READ_WIDTH_B", brw);
    ti.cfg_int("WRITE_WIDTH_A", aww);
    ti.cfg_int("WRITE_WIDTH_B", bww);

    inst.param_str("RAM_MODE", if is_sdp { "SDP" } else { "TDP" });
    ti.cfg("RAM_MODE", if is_sdp { "SDP" } else { "TDP" });

    let col = if ctx.rng.gen() {
        "DELAYED_WRITE"
    } else {
        "PERFORMANCE"
    };
    inst.param_str("RDADDR_COLLISION_HWCONFIG", col);
    ti.cfg("RDADDR_COLLISION_HWCONFIG", col);
    let do_reg_sdp = ctx.rng.gen_range(0..2);
    let rst_prio_sdp = if ctx.rng.gen() { "RSTREG" } else { "REGCE" };

    for (rw, l) in [("RD", 'A'), ("WR", 'B')] {
        let init = ctx.gen_bits(18);
        let srval = ctx.gen_bits(18);
        inst.param_bits(&format!("INIT_{l}"), &init);
        inst.param_bits(&format!("SRVAL_{l}"), &srval);
        ti.cfg_hex(&format!("INIT_{l}"), &init, true);
        ti.cfg_hex(&format!("SRVAL_{l}"), &srval, true);

        let do_reg = if is_sdp {
            do_reg_sdp
        } else {
            ctx.rng.gen_range(0..2)
        };
        inst.param_int(&format!("DO{l}_REG"), do_reg);
        ti.cfg_int(&format!("DO{l}_REG"), do_reg);

        let wmode;
        if is_sdp {
            wmode = *["WRITE_FIRST", "READ_FIRST"].choose(&mut ctx.rng).unwrap();
        } else {
            wmode = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]
                .choose(&mut ctx.rng)
                .unwrap();
        }
        inst.param_str(&format!("WRITE_MODE_{l}"), wmode);
        ti.cfg(&format!("WRITE_MODE_{l}"), wmode);

        let rst_prio = if is_sdp {
            rst_prio_sdp
        } else if ctx.rng.gen() {
            "RSTREG"
        } else {
            "REGCE"
        };
        inst.param_str(&format!("RSTREG_PRIORITY_{l}"), rst_prio);
        ti.cfg(&format!("RSTREG_PRIORITY_{l}"), rst_prio);

        let di = test.make_ins(ctx, 16);
        let do_ = test.make_outs(ctx, 16);
        let dip = test.make_ins(ctx, 2);
        let dop = test.make_outs(ctx, 2);
        inst.connect_bus(&format!("DI{l}DI"), &di);
        inst.connect_bus(&format!("DO{l}DO"), &do_);
        inst.connect_bus(&format!("DIP{l}DIP"), &dip);
        inst.connect_bus(&format!("DOP{l}DOP"), &dop);
        for i in 0..16 {
            ti.pin_in(&format!("DI{l}DI{i}"), &di[i]);
            ti.pin_out(&format!("DO{l}DO{i}"), &do_[i]);
        }
        for i in 0..2 {
            ti.pin_in(&format!("DIP{l}DIP{i}"), &dip[i]);
            ti.pin_out(&format!("DOP{l}DOP{i}"), &dop[i]);
        }

        let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
        inst.connect(&format!("CLK{l}{rw}CLK"), &clk_v);
        ti.pin_in_inv(&format!("CLK{l}{rw}CLK"), &clk_x, clk_inv);
        if l == 'A' {
            if do_reg == 1 {
                ti.pin_in_inv("REGCLKARDRCLK", &clk_x, clk_inv);
            } else {
                ti.pin_tie_inv("REGCLKARDRCLK", true, true);
            }
        } else {
            if is_sdp {
                ti.pin_tie_inv("REGCLKB", false, false);
            } else if do_reg == 1 {
                ti.pin_in_inv("REGCLKB", &clk_x, clk_inv);
            } else {
                ti.pin_tie_inv("REGCLKB", true, true);
            }
        }
        let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
        inst.connect(&format!("EN{l}{rw}EN"), &en_v);
        ti.pin_in_inv(&format!("EN{l}{rw}EN"), &en_x, en_inv);
        let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
        if l == 'A' {
            inst.connect("RSTRAMARSTRAM", &rst_v);
            ti.pin_in_inv("RSTRAMARSTRAM", &rst_x, rst_inv);
        } else {
            inst.connect("RSTRAMB", &rst_v);
            ti.pin_in_inv("RSTRAMB", &rst_x, rst_inv);
        }

        let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
        if l == 'A' {
            inst.connect("RSTREGARSTREG", &rst_v);
            ti.pin_in_inv("RSTREGARSTREG", &rst_x, rst_inv);
        } else {
            inst.connect("RSTREGB", &rst_v);
            if is_sdp {
                ti.pin_tie_inv("RSTREGB", false, false);
            } else {
                ti.pin_in_inv("RSTREGB", &rst_x, rst_inv);
            }
        }
        let regce = test.make_in(ctx);
        if l == 'A' {
            inst.connect("REGCEAREGCE", &regce);
            if do_reg == 1 {
                ti.pin_in("REGCEAREGCE", &regce);
            } else {
                ti.pin_tie("REGCEAREGCE", false);
            }
        } else {
            inst.connect("REGCEB", &regce);
            if is_sdp {
                ti.pin_tie("REGCEB", true);
            } else if do_reg == 1 {
                ti.pin_in("REGCEB", &regce);
            } else {
                ti.pin_tie("REGCEB", false);
            }
        }

        let addr = test.make_ins(ctx, 14);
        inst.connect_bus(&format!("ADDR{l}{rw}ADDR"), &addr);
        for i in 0..14 {
            ti.pin_in(&format!("ADDR{l}{rw}ADDR{i}"), &addr[i]);
        }
        for i in 0..2 {
            ti.pin_tie(&format!("ADDR{l}TIEHIGH{i}"), true);
        }
    }

    let wea = test.make_ins(ctx, 2);
    let web = test.make_ins(ctx, 4);
    inst.connect_bus("WEA", &wea);
    inst.connect_bus("WEBWE", &web);
    for i in 0..4 {
        if !is_sdp {
            let ri = match aww {
                18 => i >> 1 & 1,
                _ => 0,
            };
            ti.pin_in(&format!("WEA{i}"), &wea[ri]);
        } else {
            ti.pin_tie(&format!("WEA{i}"), false);
        }
    }
    for i in 0..8 {
        if (is_sdp && bww == 36) || i < 4 {
            let ri = match bww {
                36 => i >> 1 & 3,
                18 => i >> 1 & 1,
                _ => 0,
            };
            ti.pin_in(&format!("WEBWE{i}"), &web[ri]);
        } else {
            ti.pin_tie(&format!("WEBWE{i}"), false);
        }
    }

    ti.cfg("SAVEDATA", "FALSE");
    if mode == Mode::Series7 {
        ti.cfg("EN_PWRGATE", "NONE");
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

fn gen_ramb36e1(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, num: usize) {
    let mut insts = Vec::new();
    let mut tis = Vec::new();
    for _ in 0..num {
        let is_sdp = num == 1 && ctx.rng.gen();
        let (mut ar_en, mut br_en) = *[(true, true), (false, true), (true, false)]
            .choose(&mut ctx.rng)
            .unwrap();
        let mut aw_en = ctx.rng.gen();
        let mut bw_en = ctx.rng.gen();
        let mut arwlog2 = ctx.rng.gen_range(0..7);
        let mut awwlog2 = ctx.rng.gen_range(0..6);
        let mut brwlog2 = ctx.rng.gen_range(0..6);
        let mut bwwlog2 = ctx.rng.gen_range(0..7);
        if mode == Mode::Series7 && num == 2 {
            arwlog2 = 0;
            awwlog2 = 0;
            brwlog2 = 0;
            bwwlog2 = 0;
            ar_en = true;
            aw_en = true;
            br_en = true;
            bw_en = true;
        }
        let arw = if ar_en { WIDTHS[arwlog2] } else { 0 };
        let aww = if aw_en { WIDTHS[awwlog2] } else { 0 };
        let brw = if br_en { WIDTHS[brwlog2] } else { 0 };
        let bww = if bw_en { WIDTHS[bwwlog2] } else { 0 };

        let mut inst = SrcInst::new(ctx, "RAMB36E1");
        let mut ti = TgtInst::new(&["RAMB36E1"]);
        ti.bel("RAMB36E1", &inst.name, "");

        for i in 0..128 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            ti.cfg_hex(&format!("INIT_{i:02X}"), &init, true);
        }
        for i in 0..16 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INITP_{i:02X}"), &init);
            ti.cfg_hex(&format!("INITP_{i:02X}"), &init, true);
        }

        inst.param_int("READ_WIDTH_A", arw);
        inst.param_int("READ_WIDTH_B", brw);
        inst.param_int("WRITE_WIDTH_A", aww);
        inst.param_int("WRITE_WIDTH_B", bww);

        ti.cfg_int("READ_WIDTH_A", arw);
        ti.cfg_int("READ_WIDTH_B", brw);
        ti.cfg_int("WRITE_WIDTH_A", aww);
        ti.cfg_int("WRITE_WIDTH_B", bww);

        inst.param_str("RAM_MODE", if is_sdp { "SDP" } else { "TDP" });
        ti.cfg("RAM_MODE", if is_sdp { "SDP" } else { "TDP" });

        let col = if ctx.rng.gen() {
            "DELAYED_WRITE"
        } else {
            "PERFORMANCE"
        };
        inst.param_str("RDADDR_COLLISION_HWCONFIG", col);
        ti.cfg("RDADDR_COLLISION_HWCONFIG", col);
        let do_reg_sdp = ctx.rng.gen_range(0..2);
        let rst_prio_sdp = if ctx.rng.gen() { "RSTREG" } else { "REGCE" };

        let en_ecc_read = is_sdp && ctx.rng.gen();
        inst.param_bool("EN_ECC_READ", en_ecc_read);
        ti.cfg_bool("EN_ECC_READ", en_ecc_read);
        let en_ecc_write = is_sdp && bww == 72 && ctx.rng.gen();
        inst.param_bool("EN_ECC_WRITE", en_ecc_write);
        ti.cfg_bool("EN_ECC_WRITE", en_ecc_write);
        for o in ["SBITERR", "DBITERR"] {
            let w = test.make_out(ctx);
            inst.connect(o, &w);
            ti.pin_out(o, &w);
        }
        for o in ["INJECTSBITERR", "INJECTDBITERR"] {
            let w = test.make_in(ctx);
            inst.connect(o, &w);
            ti.pin_in(o, &w);
        }
        if !(en_ecc_read && en_ecc_write) {
            let eccparity = test.make_outs(ctx, 8);
            inst.connect_bus("ECCPARITY", &eccparity);
            for i in 0..8 {
                ti.pin_out(&format!("ECCPARITY{i}"), &eccparity[i]);
            }
        }
        let rdaddrecc = test.make_outs(ctx, 9);
        inst.connect_bus("RDADDRECC", &rdaddrecc);
        for i in 0..9 {
            ti.pin_out(&format!("RDADDRECC{i}"), &rdaddrecc[i]);
        }

        for (rw, l, ww) in [("RD", 'A', aww), ("WR", 'B', bww)] {
            let init = ctx.gen_bits(36);
            let srval = ctx.gen_bits(36);
            inst.param_bits(&format!("INIT_{l}"), &init);
            inst.param_bits(&format!("SRVAL_{l}"), &srval);
            ti.cfg_hex(&format!("INIT_{l}"), &init, true);
            ti.cfg_hex(&format!("SRVAL_{l}"), &srval, true);

            let do_reg = if is_sdp {
                do_reg_sdp
            } else {
                ctx.rng.gen_range(0..2)
            };
            inst.param_int(&format!("DO{l}_REG"), do_reg);
            ti.cfg_int(&format!("DO{l}_REG"), do_reg);

            let wmode;
            if is_sdp {
                wmode = *["WRITE_FIRST", "READ_FIRST"].choose(&mut ctx.rng).unwrap();
            } else {
                wmode = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"]
                    .choose(&mut ctx.rng)
                    .unwrap();
            }
            inst.param_str(&format!("WRITE_MODE_{l}"), wmode);
            ti.cfg(&format!("WRITE_MODE_{l}"), wmode);

            let rst_prio = if is_sdp {
                rst_prio_sdp
            } else if ctx.rng.gen() {
                "RSTREG"
            } else {
                "REGCE"
            };
            inst.param_str(&format!("RSTREG_PRIORITY_{l}"), rst_prio);
            ti.cfg(&format!("RSTREG_PRIORITY_{l}"), rst_prio);

            let di = test.make_ins(ctx, 32);
            let do_ = test.make_outs(ctx, 32);
            let dip = test.make_ins(ctx, 4);
            let dop = test.make_outs(ctx, 4);
            inst.connect_bus(&format!("DI{l}DI"), &di);
            inst.connect_bus(&format!("DO{l}DO"), &do_);
            inst.connect_bus(&format!("DIP{l}DIP"), &dip);
            inst.connect_bus(&format!("DOP{l}DOP"), &dop);
            for i in 0..32 {
                if i == 1 && ww == 1 {
                    ti.pin_in(&format!("DI{l}DI{i}"), &di[0]);
                } else {
                    ti.pin_in(&format!("DI{l}DI{i}"), &di[i]);
                }
                ti.pin_out(&format!("DO{l}DO{i}"), &do_[i]);
            }
            for i in 0..4 {
                if i == 1 && ww == 9 {
                    ti.pin_in(&format!("DIP{l}DIP{i}"), &dip[0]);
                } else {
                    ti.pin_in(&format!("DIP{l}DIP{i}"), &dip[i]);
                }
                ti.pin_out(&format!("DOP{l}DOP{i}"), &dop[i]);
            }

            let (clk_v, clk_x, clk_inv) = test.make_in_inv(ctx);
            inst.connect(&format!("CLK{l}{rw}CLK"), &clk_v);
            for ul in ['U', 'L'] {
                ti.pin_in_inv(&format!("CLK{l}{rw}CLK{ul}"), &clk_x, clk_inv);
                if l == 'A' {
                    if do_reg == 1 {
                        ti.pin_in_inv(&format!("REGCLKARDRCLK{ul}"), &clk_x, clk_inv);
                    } else {
                        ti.pin_tie_inv(&format!("REGCLKARDRCLK{ul}"), true, true);
                    }
                } else {
                    if is_sdp {
                        ti.pin_tie_inv(&format!("REGCLKB{ul}"), false, false);
                    } else if do_reg == 1 {
                        ti.pin_in_inv(&format!("REGCLKB{ul}"), &clk_x, clk_inv);
                    } else {
                        ti.pin_tie_inv(&format!("REGCLKB{ul}"), false, false);
                    }
                }
            }
            let (en_v, en_x, en_inv) = test.make_in_inv(ctx);
            inst.connect(&format!("EN{l}{rw}EN"), &en_v);
            for ul in ['U', 'L'] {
                ti.pin_in_inv(&format!("EN{l}{rw}EN{ul}"), &en_x, en_inv);
            }
            let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
            if l == 'A' {
                inst.connect("RSTRAMARSTRAM", &rst_v);
                for ul in ['U', 'L'] {
                    ti.pin_in_inv(&format!("RSTRAMARSTRAM{ul}"), &rst_x, rst_inv);
                }
            } else {
                inst.connect("RSTRAMB", &rst_v);
                for ul in ['U', 'L'] {
                    ti.pin_in_inv(&format!("RSTRAMB{ul}"), &rst_x, rst_inv);
                }
            }

            let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
            if l == 'A' {
                inst.connect("RSTREGARSTREG", &rst_v);
                for ul in ['U', 'L'] {
                    ti.pin_in_inv(&format!("RSTREGARSTREG{ul}"), &rst_x, rst_inv);
                }
            } else {
                inst.connect("RSTREGB", &rst_v);
                for ul in ['U', 'L'] {
                    if is_sdp {
                        ti.pin_tie_inv(&format!("RSTREGB{ul}"), false, false);
                    } else {
                        ti.pin_in_inv(&format!("RSTREGB{ul}"), &rst_x, rst_inv);
                    }
                }
            }
            let regce = test.make_in(ctx);
            if l == 'A' {
                inst.connect("REGCEAREGCE", &regce);
                for ul in ['U', 'L'] {
                    if do_reg == 1 {
                        ti.pin_in(&format!("REGCEAREGCE{ul}"), &regce);
                    } else {
                        ti.pin_tie(&format!("REGCEAREGCE{ul}"), false);
                    }
                }
            } else {
                inst.connect("REGCEB", &regce);
                for ul in ['U', 'L'] {
                    if is_sdp {
                        ti.pin_tie(&format!("REGCEB{ul}"), true);
                    } else if do_reg == 1 {
                        ti.pin_in(&format!("REGCEB{ul}"), &regce);
                    } else {
                        ti.pin_tie(&format!("REGCEB{ul}"), false);
                    }
                }
            }

            let mut addr = test.make_ins(ctx, 16);
            if mode == Mode::Series7 && num == 1 {
                addr[15] = "1'b1".to_string();
            }
            inst.connect_bus(&format!("ADDR{l}{rw}ADDR"), &addr);
            for i in 0..16 {
                for ul in ['U', 'L'] {
                    if ul == 'U' && i == 15 {
                        continue;
                    }
                    if mode == Mode::Series7 && num == 1 && i == 15 {
                        ti.pin_tie(&format!("ADDR{l}{rw}ADDR{ul}{i}"), true);
                    } else {
                        ti.pin_in(&format!("ADDR{l}{rw}ADDR{ul}{i}"), &addr[i]);
                    }
                }
            }
        }

        let wea = test.make_ins(ctx, 4);
        let mut web;
        if en_ecc_write {
            let we = test.make_in(ctx);
            web = Vec::new();
            for _ in 0..8 {
                web.push(we.clone());
            }
        } else {
            web = test.make_ins(ctx, 8);
        }
        inst.connect_bus("WEA", &wea);
        inst.connect_bus("WEBWE", &web);
        for ul in ['U', 'L'] {
            for i in 0..4 {
                if !is_sdp {
                    let ri = match aww {
                        36 => i & 3,
                        18 => i & 1,
                        _ => 0,
                    };
                    ti.pin_in(&format!("WEA{ul}{i}"), &wea[ri]);
                } else {
                    ti.pin_tie(&format!("WEA{ul}{i}"), false);
                }
            }
            for i in 0..8 {
                if (is_sdp && bww == 72) || i < 4 {
                    let ri = match bww {
                        72 => i & 7,
                        36 => i & 3,
                        18 => i & 1,
                        _ => 0,
                    };
                    ti.pin_in(&format!("WEBWE{ul}{i}"), &web[ri]);
                } else {
                    ti.pin_tie(&format!("WEBWE{ul}{i}"), false);
                }
            }
        }

        ti.cfg("SAVEDATA", "FALSE");
        if mode == Mode::Series7 {
            ti.cfg("EN_PWRGATE", "NONE");
        }
        insts.push(inst);
        tis.push(ti);
    }

    for l in ['A', 'B'] {
        if num == 2 {
            let c = test.make_wire(ctx);
            insts[0].connect(&format!("CASCADEOUT{l}"), &c);
            insts[1].connect(&format!("CASCADEIN{l}"), &c);
            tis[0].pin_out(&format!("CASCADEOUT{l}"), &c);
            tis[1].pin_in(&format!("CASCADEIN{l}"), &c);
            insts[0].param_str(&format!("RAM_EXTENSION_{l}"), "LOWER");
            insts[1].param_str(&format!("RAM_EXTENSION_{l}"), "UPPER");
            tis[0].cfg(&format!("RAM_EXTENSION_{l}"), "LOWER");
            tis[1].cfg(&format!("RAM_EXTENSION_{l}"), "UPPER");
        } else {
            insts[0].param_str(&format!("RAM_EXTENSION_{l}"), "NONE");
            tis[0].cfg(&format!("RAM_EXTENSION_{l}"), "NONE");
        }
    }

    for inst in insts {
        test.src_insts.push(inst);
    }
    for ti in tis {
        test.tgt_insts.push(ti);
    }
}

fn gen_fifo(test: &mut Test, ctx: &mut TestGenCtx, mode: Mode, sz: u8, pk: u8) {
    let wlog2;
    let is_sdp = ctx.rng.gen();
    if sz == 16 {
        if is_sdp {
            wlog2 = 5;
        } else {
            wlog2 = ctx.rng.gen_range(2..5);
        }
    } else {
        if is_sdp {
            wlog2 = 6;
        } else {
            wlog2 = ctx.rng.gen_range(2..6);
        }
    }
    let prim = match (pk, sz, is_sdp) {
        (4, 16, _) => "FIFO16",
        (5, 16, false) => "FIFO18",
        (5, 16, true) => "FIFO18_36",
        (5, 32, false) => "FIFO36",
        (5, 32, true) => "FIFO36_72",
        (6, 16, _) => "FIFO18E1",
        (6, 32, _) => "FIFO36E1",
        _ => unreachable!(),
    };
    let hwprim = match (mode, sz, is_sdp) {
        (Mode::Virtex4, 16, _) => "FIFO16",
        (Mode::Virtex5, 16, false) => "RAMBFIFO18",
        (Mode::Virtex5, 16, true) => "RAMBFIFO18_36",
        (Mode::Virtex5, 32, false) => "FIFO36_EXP",
        (Mode::Virtex5, 32, true) => "FIFO36_72_EXP",
        (Mode::Virtex6 | Mode::Series7, 16, _) => "FIFO18E1",
        (Mode::Virtex6 | Mode::Series7, 32, _) => "FIFO36E1",
        _ => unreachable!(),
    };
    let mut inst = SrcInst::new(ctx, &prim);
    let mut ti = TgtInst::new(&[hwprim]);

    if pk != 5 || !is_sdp {
        inst.param_int("DATA_WIDTH", WIDTHS[wlog2]);
    }
    if pk == 6 {
        if sz == 16 {
            inst.param_str("FIFO_MODE", if is_sdp { "FIFO18_36" } else { "FIFO18" })
        } else {
            inst.param_str("FIFO_MODE", if is_sdp { "FIFO36_72" } else { "FIFO36" })
        }
    }

    let en_ecc_read;
    let en_ecc_write;
    let en_syn;
    let do_reg;
    if pk != 4 && is_sdp && sz == 32 {
        en_ecc_read = if ctx.rng.gen() { "TRUE" } else { "FALSE" };
        en_ecc_write = if ctx.rng.gen() { "TRUE" } else { "FALSE" };
        inst.param_str("EN_ECC_READ", en_ecc_read);
        inst.param_str("EN_ECC_WRITE", en_ecc_write);
    } else {
        en_ecc_read = "FALSE";
        en_ecc_write = "FALSE";
    }
    if pk != 4 {
        en_syn = if ctx.rng.gen() { "TRUE" } else { "FALSE" };
        do_reg = if en_syn == "FALSE" || ctx.rng.gen() {
            1
        } else {
            0
        };
        inst.param_str("EN_SYN", en_syn);
        inst.param_int("DO_REG", do_reg);
    } else {
        en_syn = "FALSE";
        do_reg = 1;
    }
    if mode != Mode::Virtex4 {
        ti.cfg("EN_SYN", en_syn);
    }

    if mode == Mode::Virtex4 {
        ti.bel("FIFO16", &inst.name, "");
        ti.cfg_int("DATA_WIDTH", WIDTHS[wlog2]);
        ti.cfg("EN_ECC_READ", en_ecc_read);
        ti.cfg("EN_ECC_WRITE", en_ecc_write);
    } else if mode == Mode::Virtex5 {
        if !is_sdp {
            ti.cfg_int("DATA_WIDTH", WIDTHS[wlog2]);
        }
        if sz == 16 {
            if !is_sdp {
                ti.bel("RAMBFIFO18_LOWER", &inst.name, "");
                ti.cfg_int("DO_REG", do_reg);
            } else {
                ti.bel("RAMBFIFO18_36_LOWER", &inst.name, "");
                ti.cfg_int("DO_REG_L", do_reg);
                ti.pin_tie("TIEOFFWEAL0", false);
                ti.pin_tie("TIEOFFWEAL1", false);
                ti.pin_tie("TIEOFFWEAL2", false);
                ti.pin_tie("TIEOFFWEAL3", false);
                ti.pin_tie("TIEOFFSSRBL", false);
            }
            ti.pin_tie("TIEOFFREGCEAL", true);
        } else {
            if !is_sdp {
                ti.bel("FIFO36_EXP", &inst.name, "");
            } else {
                ti.bel("FIFO36_72_EXP", &inst.name, "");
                ti.cfg("EN_ECC_READ", en_ecc_read);
                ti.cfg("EN_ECC_WRITE", en_ecc_write);
            }
            ti.cfg_int("DO_REG", do_reg);
            ti.pin_tie("TIEOFFREGCEAL", true);
            ti.pin_tie("TIEOFFREGCEAU", true);
        }
    } else {
        if sz == 16 {
            if pk == 5 {
                ti.bel("FIFO18E1", &format!("{}/FIFO18E1", inst.name), "");
            } else {
                ti.bel("FIFO18E1", &inst.name, "");
            }
            if pk == 6 {
                let init = ctx.gen_bits(36);
                let srval = ctx.gen_bits(36);
                inst.param_bits("INIT", &init);
                inst.param_bits("SRVAL", &srval);
                ti.cfg_hex("INIT", &init, true);
                ti.cfg_hex("SRVAL", &srval, true);
            } else {
                ti.cfg("INIT", "000000000");
                ti.cfg("SRVAL", "000000000");
            }
            if is_sdp {
                ti.cfg("FIFO_MODE", "FIFO18_36");
            } else {
                ti.cfg("FIFO_MODE", "FIFO18");
            }
        } else {
            if pk == 5 {
                ti.bel("FIFO36E1", &format!("{}/FIFO36E1", inst.name), "");
            } else {
                ti.bel("FIFO36E1", &inst.name, "");
            }
            ti.cfg("EN_ECC_READ", en_ecc_read);
            ti.cfg("EN_ECC_WRITE", en_ecc_write);
            if pk == 6 {
                let init = ctx.gen_bits(72);
                let srval = ctx.gen_bits(72);
                inst.param_bits("INIT", &init);
                inst.param_bits("SRVAL", &srval);
                ti.cfg_hex("INIT", &init, true);
                ti.cfg_hex("SRVAL", &srval, true);
            } else {
                ti.cfg("INIT", "000000000000000000");
                ti.cfg("SRVAL", "000000000000000000");
            }
            if is_sdp {
                ti.cfg("FIFO_MODE", "FIFO36_72");
            } else {
                ti.cfg("FIFO_MODE", "FIFO36");
            }
            if pk == 6 {
                for p in ["INJECTSBITERR", "INJECTDBITERR"] {
                    let w = test.make_in(ctx);
                    inst.connect(p, &w);
                    ti.pin_in(p, &w);
                }
            } else if is_sdp && (en_ecc_write == "TRUE" || en_ecc_read == "TRUE") {
                for p in ["INJECTSBITERR", "INJECTDBITERR"] {
                    ti.pin_tie(p, false);
                }
            }
        }
        ti.cfg_int("DATA_WIDTH", WIDTHS[wlog2]);
        ti.cfg_int("DO_REG", do_reg);
        if mode == Mode::Series7 {
            ti.cfg("EN_PWRGATE", "NONE");
        } else {
            ti.cfg("RSTREG_PRIORITY", "RSTREG");
        }
    }

    let (rdclk_v, rdclk_x, rdclk_inv) = test.make_in_inv(ctx);
    inst.connect("RDCLK", &rdclk_v);
    let (wrclk_v, wrclk_x, wrclk_inv);
    if en_syn == "TRUE" {
        (wrclk_v, wrclk_x, wrclk_inv) = (rdclk_v.clone(), rdclk_x.clone(), rdclk_inv);
    } else {
        (wrclk_v, wrclk_x, wrclk_inv) = test.make_in_inv(ctx);
    }
    inst.connect("WRCLK", &wrclk_v);
    let (rden_v, rden_x, rden_inv) = test.make_in_inv(ctx);
    inst.connect("RDEN", &rden_v);
    let (wren_v, wren_x, wren_inv) = test.make_in_inv(ctx);
    inst.connect("WREN", &wren_v);
    let (rst_v, rst_x, rst_inv) = test.make_in_inv(ctx);
    inst.connect("RST", &rst_v);

    let (di, dip, do_, dop);
    if pk == 5 && !is_sdp {
        if sz == 16 {
            di = test.make_ins(ctx, 16);
            dip = test.make_ins(ctx, 2);
            do_ = test.make_outs(ctx, 16);
            dop = test.make_outs(ctx, 2);
        } else {
            di = test.make_ins(ctx, 32);
            dip = test.make_ins(ctx, 4);
            do_ = test.make_outs(ctx, 32);
            dop = test.make_outs(ctx, 4);
        }
    } else {
        if sz == 16 {
            di = test.make_ins(ctx, 32);
            dip = test.make_ins(ctx, 4);
            do_ = test.make_outs(ctx, 32);
            dop = test.make_outs(ctx, 4);
        } else {
            di = test.make_ins(ctx, 64);
            dip = test.make_ins(ctx, 8);
            do_ = test.make_outs(ctx, 64);
            dop = test.make_outs(ctx, 8);
        }
    }
    inst.connect_bus("DI", &di);
    inst.connect_bus("DIP", &dip);
    inst.connect_bus("DO", &do_);
    inst.connect_bus("DOP", &dop);
    if mode == Mode::Virtex4 {
        for i in 0..32 {
            ti.pin_in(&format!("DI{i}"), &di[i]);
            ti.pin_out(&format!("DO{i}"), &do_[i]);
        }
        for i in 0..4 {
            ti.pin_in(&format!("DIP{i}"), &dip[i]);
            ti.pin_out(&format!("DOP{i}"), &dop[i]);
        }
        ti.pin_in_inv("RDCLK", &rdclk_x, rdclk_inv);
        ti.pin_in_inv("WRCLK", &wrclk_x, wrclk_inv);
        ti.pin_in_inv("RDEN", &rden_x, rden_inv);
        ti.pin_in_inv("WREN", &wren_x, wren_inv);
        ti.pin_in_inv("RST", &rst_x, rst_inv);
    } else if mode == Mode::Virtex5 {
        if sz == 16 {
            if !is_sdp {
                for i in 0..16 {
                    ti.pin_in(&format!("DI{i}"), &di[i]);
                    ti.pin_out(&format!("DO{i}"), &do_[i]);
                }
                for i in 0..2 {
                    ti.pin_in(&format!("DIP{i}"), &dip[i]);
                    ti.pin_out(&format!("DOP{i}"), &dop[i]);
                }
            } else {
                for i in 0..32 {
                    ti.pin_in(&format!("DI{i}"), &di[i]);
                    ti.pin_out(&format!("DO{i}"), &do_[i]);
                }
                for i in 0..4 {
                    ti.pin_in(&format!("DIP{i}"), &dip[i]);
                    ti.pin_out(&format!("DOP{i}"), &dop[i]);
                }
            }
            ti.pin_in_inv("RDCLK", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("RDRCLK", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("WRCLK", &wrclk_x, wrclk_inv);
        } else {
            if !is_sdp {
                for i in 0..32 {
                    ti.pin_in(&format!("DI{i}"), &di[i]);
                    ti.pin_out(&format!("DO{i}"), &do_[i]);
                }
                for i in 0..4 {
                    if wlog2 == 3 && i == 1 {
                        ti.pin_in(&format!("DIP{i}"), &dip[0]);
                    } else {
                        ti.pin_in(&format!("DIP{i}"), &dip[i]);
                    }
                    ti.pin_out(&format!("DOP{i}"), &dop[i]);
                }
            } else {
                for i in 0..64 {
                    ti.pin_in(&format!("DI{i}"), &di[i]);
                    ti.pin_out(&format!("DO{i}"), &do_[i]);
                }
                for i in 0..8 {
                    ti.pin_in(&format!("DIP{i}"), &dip[i]);
                    ti.pin_out(&format!("DOP{i}"), &dop[i]);
                }
            }
            ti.pin_in_inv("RDCLKL", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("RDCLKU", &rdclk_x, rdclk_inv);
            if do_reg == 1 {
                ti.pin_in_inv("RDRCLKL", &rdclk_x, rdclk_inv);
                ti.pin_in_inv("RDRCLKU", &rdclk_x, rdclk_inv);
            } else {
                ti.pin_tie_inv("RDRCLKL", false, false);
                ti.pin_tie_inv("RDRCLKU", false, false);
            }
            ti.pin_in_inv("WRCLKL", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("WRCLKU", &wrclk_x, wrclk_inv);
        }
        ti.pin_in_inv("RDEN", &rden_x, rden_inv);
        ti.pin_in_inv("WREN", &wren_x, wren_inv);
        ti.pin_in_inv("RST", &rst_x, rst_inv);
    } else {
        if sz == 16 {
            if !is_sdp {
                for i in 0..16 {
                    ti.pin_in(&format!("DIBDI{i}"), &di[i]);
                }
                for i in 0..2 {
                    ti.pin_in(&format!("DIPBDIP{i}"), &dip[i]);
                }
            } else {
                for i in 0..16 {
                    ti.pin_in(&format!("DIADI{i}"), &di[i]);
                    ti.pin_in(&format!("DIBDI{i}"), &di[i + 16]);
                }
                for i in 0..2 {
                    ti.pin_in(&format!("DIPADIP{i}"), &dip[i]);
                    ti.pin_in(&format!("DIPBDIP{i}"), &dip[i + 2]);
                }
            }
            if pk == 4 && !is_sdp {
                for i in 0..16 {
                    ti.pin_out(&format!("DO{i}"), &do_[i]);
                }
                for i in 0..2 {
                    ti.pin_out(&format!("DOP{i}"), &dop[i]);
                }
            } else {
                for i in 0..do_.len() {
                    ti.pin_out(&format!("DO{i}"), &do_[i]);
                }
                for i in 0..dop.len() {
                    ti.pin_out(&format!("DOP{i}"), &dop[i]);
                }
            }
            ti.pin_in_inv("RDCLK", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("WRCLK", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("RDEN", &rden_x, rden_inv);
            ti.pin_in_inv("WREN", &wren_x, wren_inv);
            ti.pin_in_inv("RST", &rst_x, rst_inv);
            if pk == 4 {
                ti.pin_tie_inv("RSTREG", false, false);
            } else if pk == 6 {
                let regce = test.make_in(ctx);
                let (rr_v, rr_x, rr_inv) = test.make_in_inv(ctx);
                inst.connect("REGCE", &regce);
                inst.connect("RSTREG", &rr_v);
                if en_syn == "TRUE" {
                    ti.pin_in("REGCE", &regce);
                } else if do_reg == 1 {
                    ti.pin_tie("REGCE", true);
                } else {
                    ti.pin_tie("REGCE", false);
                }
                if do_reg == 1 && en_syn == "TRUE" {
                    ti.pin_in_inv("RSTREG", &rr_x, rr_inv);
                } else {
                    ti.pin_tie_inv("RSTREG", false, false);
                }
            } else {
                if do_reg == 1 {
                    ti.pin_tie("REGCE", true);
                    if en_syn == "TRUE" {
                        ti.pin_tie_inv("RSTREG", true, true);
                    } else {
                        ti.pin_tie_inv("RSTREG", false, false);
                    }
                } else {
                    ti.pin_tie("REGCE", false);
                    ti.pin_tie_inv("RSTREG", false, false);
                }
            }
            if do_reg == 1 && pk != 4 {
                ti.pin_in_inv("RDRCLK", &rdclk_x, rdclk_inv);
            } else {
                ti.pin_tie_inv("RDRCLK", true, true);
            }
        } else {
            if !is_sdp {
                for i in 0..32 {
                    ti.pin_in(&format!("DIBDI{i}"), &di[i]);
                }
                for i in 0..4 {
                    if wlog2 == 3 && i == 1 {
                        ti.pin_in(&format!("DIPBDIP{i}"), &dip[0]);
                    } else {
                        ti.pin_in(&format!("DIPBDIP{i}"), &dip[i]);
                    }
                }
            } else {
                for i in 0..32 {
                    ti.pin_in(&format!("DIADI{i}"), &di[i]);
                    ti.pin_in(&format!("DIBDI{i}"), &di[i + 32]);
                }
                for i in 0..4 {
                    ti.pin_in(&format!("DIPADIP{i}"), &dip[i]);
                    ti.pin_in(&format!("DIPBDIP{i}"), &dip[i + 4]);
                }
            }
            for i in 0..do_.len() {
                ti.pin_out(&format!("DO{i}"), &do_[i]);
            }
            for i in 0..dop.len() {
                ti.pin_out(&format!("DOP{i}"), &dop[i]);
            }
            ti.pin_in_inv("RDCLKL", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("RDCLKU", &rdclk_x, rdclk_inv);
            ti.pin_in_inv("WRCLKL", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("WRCLKU", &wrclk_x, wrclk_inv);
            ti.pin_in_inv("RDENL", &rden_x, rden_inv);
            ti.pin_tie_inv("RDENU", true, true);
            ti.pin_in_inv("WRENL", &wren_x, wren_inv);
            ti.pin_tie_inv("WRENU", true, true);
            ti.pin_in_inv("RST", &rst_x, rst_inv);
            if pk == 6 {
                let regce = test.make_in(ctx);
                let (rr_v, rr_x, rr_inv) = test.make_in_inv(ctx);
                inst.connect("REGCE", &regce);
                inst.connect("RSTREG", &rr_v);
                if do_reg == 1 && en_syn == "TRUE" {
                    ti.pin_in("REGCEL", &regce);
                    ti.pin_in("REGCEU", &regce);
                } else if do_reg == 1 {
                    ti.pin_tie("REGCEL", true);
                    ti.pin_tie("REGCEU", true);
                } else {
                    ti.pin_tie("REGCEL", false);
                    ti.pin_tie("REGCEU", false);
                }
                if do_reg == 1 || en_syn == "TRUE" {
                    ti.pin_in_inv("RSTREGL", &rr_x, rr_inv);
                    ti.pin_in_inv("RSTREGU", &rr_x, rr_inv);
                } else {
                    ti.pin_tie_inv("RSTREGL", false, false);
                    ti.pin_tie_inv("RSTREGU", false, false);
                }
            } else {
                if do_reg == 1 {
                    ti.pin_tie("REGCEL", true);
                    ti.pin_tie("REGCEU", true);
                } else {
                    ti.pin_tie("REGCEL", false);
                    ti.pin_tie("REGCEU", false);
                }
                ti.pin_tie_inv("RSTREGL", true, true);
                ti.pin_tie_inv("RSTREGU", true, true);
            }
            if do_reg == 1 {
                ti.pin_in_inv("RDRCLKL", &rdclk_x, rdclk_inv);
                ti.pin_in_inv("RDRCLKU", &rdclk_x, rdclk_inv);
            } else {
                ti.pin_tie_inv("RDRCLKL", true, true);
                ti.pin_tie_inv("RDRCLKU", true, true);
            }
        }
    }

    for p in [
        "EMPTY",
        "FULL",
        "ALMOSTEMPTY",
        "ALMOSTFULL",
        "RDERR",
        "WRERR",
    ] {
        let w = test.make_out(ctx);
        inst.connect(p, &w);
        ti.pin_out(p, &w);
    }

    let cntbits;
    if pk == 4 {
        cntbits = 12;
    } else if pk == 5 {
        if is_sdp {
            cntbits = 9;
        } else if sz == 16 {
            cntbits = 12;
        } else {
            cntbits = 13;
        }
    } else {
        if sz == 16 {
            cntbits = 12;
        } else {
            cntbits = 13;
        }
    }

    let fwft = if do_reg == 1 && en_syn == "FALSE" && ctx.rng.gen() {
        "TRUE"
    } else {
        "FALSE"
    };
    inst.param_str("FIRST_WORD_FALL_THROUGH", fwft);
    ti.cfg("FIRST_WORD_FALL_THROUGH", fwft);

    let num_e;
    if sz == 16 {
        num_e = 1 << 14 - wlog2;
    } else {
        num_e = 1 << 15 - wlog2;
    }
    let (ae_off, af_off);
    if en_syn == "FALSE" {
        match mode {
            Mode::Virtex4 => {
                if fwft == "TRUE" {
                    ae_off = ctx.rng.gen_range(6..(num_e - 2));
                } else {
                    ae_off = ctx.rng.gen_range(5..(num_e - 3));
                }
                af_off = ctx.rng.gen_range(4..(num_e - 4));
            }
            Mode::Virtex5 | Mode::Virtex6 => {
                if fwft == "TRUE" {
                    ae_off = ctx.rng.gen_range(6..(num_e - 3));
                } else {
                    ae_off = ctx.rng.gen_range(5..(num_e - 4));
                }
                af_off = ctx.rng.gen_range(4..(num_e - 4));
            }
            Mode::Series7 => {
                if fwft == "TRUE" {
                    ae_off = ctx.rng.gen_range(6..(num_e - 4));
                } else {
                    ae_off = ctx.rng.gen_range(5..(num_e - 5));
                }
                af_off = ctx.rng.gen_range(4..(num_e - 6));
            }
            _ => unreachable!(),
        }
    } else {
        ae_off = ctx.rng.gen_range(1..(num_e - 1));
        af_off = ctx.rng.gen_range(1..(num_e - 1));
    }
    inst.param_int("ALMOST_EMPTY_OFFSET", ae_off);
    inst.param_int("ALMOST_FULL_OFFSET", af_off);
    if mode == Mode::Virtex6 || (sz == 32 && !is_sdp) || (mode == Mode::Series7 && pk != 4) {
        ti.cfg("ALMOST_EMPTY_OFFSET", &format!("{ae_off:04X}"));
        ti.cfg("ALMOST_FULL_OFFSET", &format!("{af_off:04X}"));
    } else {
        ti.cfg("ALMOST_EMPTY_OFFSET", &format!("{ae_off:03X}"));
        ti.cfg("ALMOST_FULL_OFFSET", &format!("{af_off:03X}"));
    }

    for p in ["RDCOUNT", "WRCOUNT"] {
        let w = test.make_outs(ctx, cntbits);
        inst.connect_bus(p, &w);
        let mut num = cntbits;
        if pk == 4 && mode != Mode::Virtex4 && is_sdp {
            num = 9;
        }
        for i in 0..num {
            ti.pin_out(&format!("{p}{i}"), &w[i]);
        }
    }

    if wlog2 == 6 || (sz == 32 && pk == 6) {
        for p in ["SBITERR", "DBITERR"] {
            let w = test.make_out(ctx);
            inst.connect(p, &w);
            ti.pin_out(p, &w);
        }
        let w = test.make_outs(ctx, 8);
        inst.connect_bus("ECCPARITY", &w);
        for i in 0..8 {
            ti.pin_out(&format!("ECCPARITY{i}"), &w[i]);
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

pub fn gen_io_fifo(test: &mut Test, ctx: &mut TestGenCtx, is_out: bool) {
    let prim = if is_out { "OUT_FIFO" } else { "IN_FIFO" };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&[prim]);

    ti.bel(prim, &inst.name, "");

    let ar_mode = *[
        if is_out {
            "ARRAY_MODE_8_X_4"
        } else {
            "ARRAY_MODE_4_X_8"
        },
        "ARRAY_MODE_4_X_4",
    ]
    .choose(&mut ctx.rng)
    .unwrap();
    inst.param_str("ARRAY_MODE", ar_mode);
    ti.cfg("ARRAY_MODE", ar_mode);

    // This is a primitive parameter, but a TRUE is rejected anyway.
    ti.cfg("SYNCHRONOUS_MODE", "FALSE");

    let aev = ctx.rng.gen_range(1..3);
    inst.param_int("ALMOST_EMPTY_VALUE", aev);
    ti.cfg_int("ALMOST_EMPTY_VALUE", aev);
    let afv = ctx.rng.gen_range(1..3);
    inst.param_int("ALMOST_FULL_VALUE", afv);
    ti.cfg_int("ALMOST_FULL_VALUE", afv);

    if is_out {
        let od = if ctx.rng.gen() { "TRUE" } else { "FALSE" };
        inst.param_str("OUTPUT_DISABLE", od);
        ti.cfg("OUTPUT_DISABLE", od);
    }

    for pin in ["RDEN", "WREN", "RESET", "RDCLK", "WRCLK"] {
        let w = test.make_in(ctx);
        inst.connect(pin, &w);
        ti.pin_in(pin, &w);
    }

    for pin in ["FULL", "EMPTY", "ALMOSTFULL", "ALMOSTEMPTY"] {
        let w = test.make_out(ctx);
        inst.connect(pin, &w);
        ti.pin_out(pin, &w);
    }

    for i in 0..10 {
        let qsz = if !is_out || matches!(i, 5 | 6) { 8 } else { 4 };
        let q = test.make_outs(ctx, qsz);
        inst.connect_bus(&format!("Q{i}"), &q);
        for j in 0..qsz {
            ti.pin_out(&format!("Q{i}{j}"), &q[j]);
        }
        let dsz = if is_out || matches!(i, 5 | 6) { 8 } else { 4 };
        let d = test.make_ins(ctx, dsz);
        inst.connect_bus(&format!("D{i}"), &d);
        for j in 0..dsz {
            ti.pin_in(&format!("D{i}{j}"), &d[j]);
        }
    }

    ti.cfg("SLOW_RD_CLK", "FALSE");
    ti.cfg("SLOW_WR_CLK", "FALSE");
    ti.cfg("SPARE", "0000");

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

pub fn gen_ramb(ctx: &mut TestGenCtx, mode: Mode, test: &mut Test) {
    for _ in 0..3 {
        if matches!(
            mode,
            Mode::Virtex | Mode::Virtex2 | Mode::Spartan3A | Mode::Spartan3ADsp
        ) {
            gen_ramb_v(test, ctx, mode, 4, false, false);
            gen_ramb_v(test, ctx, mode, 4, true, false);
        }
        if mode != Mode::Virtex {
            gen_ramb_v(test, ctx, mode, 16, false, false);
            gen_ramb_v(test, ctx, mode, 16, true, false);
            if matches!(mode, Mode::Spartan3A | Mode::Spartan3ADsp | Mode::Spartan6) {
                gen_ramb_v(test, ctx, mode, 16, false, true);
                gen_ramb_v(test, ctx, mode, 16, true, true);
            }
        }
        if matches!(mode, Mode::Spartan3ADsp | Mode::Spartan6) {
            gen_ramb_bwer(test, ctx, mode, 16, false);
        }
        if mode == Mode::Spartan6 {
            gen_ramb_bwer(test, ctx, mode, 8, false);
            gen_ramb_bwer(test, ctx, mode, 8, true);
        }
        if matches!(
            mode,
            Mode::Virtex4 | Mode::Virtex5 | Mode::Virtex6 | Mode::Series7
        ) {
            if mode == Mode::Virtex4 {
                // these have retarget rules but they're more trouble than they're worth
                gen_ramb16(test, ctx, 1);
                gen_ramb16(test, ctx, 2);
            }
            gen_ramb32_ecc(test, ctx, mode);
            gen_fifo(test, ctx, mode, 16, 4);
        }
        if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
            if mode == Mode::Virtex5 {
                gen_ramb18(test, ctx);
                gen_ramb18sdp(test, ctx);
                gen_ramb36(test, ctx, 1);
                gen_ramb36(test, ctx, 2);
                gen_ramb36sdp(test, ctx);
            }
            gen_fifo(test, ctx, mode, 16, 5);
            gen_fifo(test, ctx, mode, 32, 5);
        }
        if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            gen_fifo(test, ctx, mode, 16, 6);
            gen_fifo(test, ctx, mode, 32, 6);
            gen_ramb18e1(test, ctx, mode);
            gen_ramb36e1(test, ctx, mode, 1);
            gen_ramb36e1(test, ctx, mode, 2);
        }
        if mode == Mode::Series7 {
            gen_io_fifo(test, ctx, false);
            gen_io_fifo(test, ctx, true);
        }
    }
}
