use crate::types::{Test, SrcInst, TgtInst, TestGenCtx, BitVal};

use rand::{Rng, seq::SliceRandom};

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

fn fmt_hex(bits: &[BitVal]) -> String {
    let mut res = String::new();
    for i in (0..((bits.len()+3)/4)).rev() {
        let mut v = 0;
        for j in 0..4 {
            if 4*i+j < bits.len() && bits[4*i+j] == BitVal::S1 {
                v |= 1 << j;
            }
        }
        res.push(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'][v]);
    }
    res
}

const ZERO_INIT: &str = "0000000000000000000000000000000000000000000000000000000000000000";

const PORT_ATTR_V: &[&str] = &[
    "4096X1",
    "2048X2",
    "1024X4",
    "512X8",
    "256X16",
];

const PORT_ATTR_V2: &[&str] = &[
    "16384X1",
    "8192X2",
    "4096X4",
    "2048X9",
    "1024X18",
    "512X36",
];

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
    let ul = if ctx.rng.gen() {"U"} else {"L"};
    let is_36 = awlog2 == 5 || bwlog2 == 5;
    let uls = if is_36 {vec!["L", "U"]} else if mode == Mode::Virtex5 {vec![ul]} else {vec![""]};

    let hwprim = match mode {
        Mode::Virtex => "BLOCKRAM",
        Mode::Virtex2 => "RAMB16",
        Mode::Spartan3A => "RAMB16BWE",
        Mode::Spartan3ADsp | Mode::Spartan6 => "RAMB16BWER",
        Mode::Virtex4 => "RAMB16",
        Mode::Virtex5 => if is_36 {"RAMB36_EXP"} else {"RAMB18X2"},
        Mode::Virtex6 | Mode::Series7 => if is_36 {"RAMB36E1"} else {"RAMB18E1"},
    };
    let mut ti = TgtInst::new(&[hwprim]);

    let mut wmode_a = "WRITE_FIRST";
    let mut wmode_b = "WRITE_FIRST";
    if sz == 16 {
        wmode_a = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"].choose(&mut ctx.rng).unwrap();
        if dp {
            wmode_b = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"].choose(&mut ctx.rng).unwrap();
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
        ti.cfg("READ_WIDTH_A", ["1", "2", "4", "9", "18", "36"][awlog2]);
        ti.cfg("WRITE_WIDTH_A", ["1", "2", "4", "9", "18", "36"][awlog2]);
        if dp {
            ti.cfg("READ_WIDTH_B", ["1", "2", "4", "9", "18", "36"][bwlog2]);
            ti.cfg("WRITE_WIDTH_B", ["1", "2", "4", "9", "18", "36"][bwlog2]);
        } else {
            ti.cfg("READ_WIDTH_B", "0");
            ti.cfg("WRITE_WIDTH_B", "0");
        }
        ti.cfg("WRITE_MODE_A", wmode_a);
        ti.cfg("WRITE_MODE_B", wmode_b);
        ti.cfg("DOA_REG", "0");
        ti.cfg("DOB_REG", "0");
        ti.cfg("REGCEAINV", "REGCEA");
        ti.pin_tie("REGCEA", false);
        if dp {
            ti.cfg("REGCEBINV", "REGCEB");
            ti.pin_tie("REGCEB", false);
        }
    } else if mode == Mode::Virtex5 {
        if is_36 {
            ti.bel("RAMB36_EXP", &inst.name, "");
            ti.cfg("RAM_EXTENSION_A", "NONE");
            ti.cfg("RAM_EXTENSION_B", "NONE");
            ti.cfg("SAVEDATA", "FALSE");
            ti.cfg("READ_WIDTH_A", ["1", "2", "4", "9", "18", "36"][awlog2]);
            ti.cfg("WRITE_WIDTH_A", ["1", "2", "4", "9", "18", "36"][awlog2]);
            if dp {
                ti.cfg("READ_WIDTH_B", ["1", "2", "4", "9", "18", "36"][bwlog2]);
                ti.cfg("WRITE_WIDTH_B", ["1", "2", "4", "9", "18", "36"][bwlog2]);
            } else {
                ti.cfg("READ_WIDTH_B", "0");
                ti.cfg("WRITE_WIDTH_B", "0");
            }
            ti.cfg("WRITE_MODE_A", wmode_a);
            ti.cfg("WRITE_MODE_B", wmode_b);
            ti.cfg("DOA_REG", "0");
            ti.cfg("DOB_REG", "0");
        } else {
            if ul == "L" {
                ti.bel("RAMB18X2_LOWER", &inst.name, "");
                inst.attr_str("BEL", "LOWER");
            } else {
                ti.bel("RAMB18X2_UPPER", &inst.name, "");
                inst.attr_str("BEL", "UPPER");
            }
            ti.cfg(&format!("SAVEDATA_{ul}"), "FALSE");
            ti.cfg(&format!("READ_WIDTH_A_{ul}"), ["1", "2", "4", "9", "18", "36"][awlog2]);
            ti.cfg(&format!("WRITE_WIDTH_A_{ul}"), ["1", "2", "4", "9", "18", "36"][awlog2]);
            if dp {
                ti.cfg(&format!("READ_WIDTH_B_{ul}"), ["1", "2", "4", "9", "18", "36"][bwlog2]);
                ti.cfg(&format!("WRITE_WIDTH_B_{ul}"), ["1", "2", "4", "9", "18", "36"][bwlog2]);
            } else {
                ti.cfg(&format!("READ_WIDTH_B_{ul}"), "0");
                ti.cfg(&format!("WRITE_WIDTH_B_{ul}"), "0");
            }
            ti.cfg(&format!("WRITE_MODE_A_{ul}"), wmode_a);
            ti.cfg(&format!("WRITE_MODE_B_{ul}"), wmode_b);
            ti.cfg(&format!("DOA_REG_{ul}"), "0");
            ti.cfg(&format!("DOB_REG_{ul}"), "0");
        }
        for &ul in &uls {
            ti.pin_tie(&format!("REGCEA{ul}"), false);
            if dp {
                ti.pin_tie(&format!("REGCEB{ul}"), false);
            }
            if is_36 {
                ti.pin_tie(&format!("REGCLKA{ul}"), true);
                ti.pin_tie(&format!("REGCLKB{ul}"), true);
                ti.cfg(&format!("REGCLKA{ul}INV"), &format!("REGCLKA{ul}_B"));
                ti.cfg(&format!("REGCLKB{ul}INV"), &format!("REGCLKB{ul}_B"));
            }
            if !dp {
                ti.pin_tie(&format!("CLKB{ul}"), true);
                ti.pin_tie(&format!("ENB{ul}"), true);
                ti.pin_tie(&format!("SSRB{ul}"), true);
                ti.cfg(&format!("CLKB{ul}INV"), &format!("CLKB{ul}_B"));
                ti.cfg(&format!("ENB{ul}INV"), &format!("ENB{ul}_B"));
                ti.cfg(&format!("SSRB{ul}INV"), &format!("SSRB{ul}_B"));
                if !is_36 {
                    ti.pin_tie(&format!("REGCLKB{ul}"), true);
                    ti.cfg(&format!("REGCLKB{ul}INV"), &format!("REGCLKB{ul}_B"));
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
        ti.cfg("READ_WIDTH_A", ["1", "2", "4", "9", "18", "36"][awlog2]);
        ti.cfg("WRITE_WIDTH_A", ["1", "2", "4", "9", "18", "36"][awlog2]);
        if dp {
            ti.cfg("READ_WIDTH_B", ["1", "2", "4", "9", "18", "36"][bwlog2]);
            ti.cfg("WRITE_WIDTH_B", ["1", "2", "4", "9", "18", "36"][bwlog2]);
        } else {
            ti.cfg("READ_WIDTH_B", "0");
            ti.cfg("WRITE_WIDTH_B", "0");
        }
        ti.cfg("WRITE_MODE_A", wmode_a);
        ti.cfg("WRITE_MODE_B", wmode_b);
        ti.cfg("DOA_REG", "0");
        ti.cfg("DOB_REG", "0");
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
            ti.pin_tie(&format!("RSTREGARSTREG{ul}"), true);
            ti.pin_tie(&format!("RSTREGB{ul}"), true);
            ti.pin_tie(&format!("REGCLKARDRCLK{ul}"), true);
            ti.cfg(&format!("RSTREGARSTREG{ul}INV"), &format!("RSTREGARSTREG{ul}_B"));
            ti.cfg(&format!("RSTREGB{ul}INV"), &format!("RSTREGB{ul}_B"));
            ti.cfg(&format!("REGCLKARDRCLK{ul}INV"), &format!("REGCLKARDRCLK{ul}_B"));
            if is_36 {
                ti.pin_tie(&format!("REGCLKB{ul}"), false);
                ti.cfg(&format!("REGCLKB{ul}INV"), &format!("REGCLKB{ul}"));
            } else {
                ti.pin_tie(&format!("REGCLKB{ul}"), true);
                ti.cfg(&format!("REGCLKB{ul}INV"), &format!("REGCLKB{ul}_B"));
            }
            if !dp {
                ti.pin_tie(&format!("CLKBWRCLK{ul}"), true);
                ti.pin_tie(&format!("ENBWREN{ul}"), true);
                ti.pin_tie(&format!("RSTRAMB{ul}"), true);
                ti.cfg(&format!("CLKBWRCLK{ul}INV"), &format!("CLKBWRCLK{ul}_B"));
                ti.cfg(&format!("ENBWREN{ul}INV"), &format!("ENBWREN{ul}_B"));
                ti.cfg(&format!("RSTRAMB{ul}INV"), &format!("RSTRAMB{ul}_B"));
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
        ti.cfg("DATA_WIDTH_A", ["1", "2", "4", "9", "18", "36"][awlog2]);
        if dp {
            ti.cfg("DATA_WIDTH_B", ["1", "2", "4", "9", "18", "36"][bwlog2]);
        } else {
            ti.cfg("DATA_WIDTH_B", "0");
        }
        ti.cfg("WRITE_MODE_A", wmode_a);
        ti.cfg("WRITE_MODE_B", wmode_b);
    } else {
        ti.bel("RAMB16BWER", &inst.name, "");
        ti.cfg("DATA_WIDTH_A", ["1", "2", "4", "9", "18", "36"][awlog2]);
        if dp {
            ti.cfg("DATA_WIDTH_B", ["1", "2", "4", "9", "18", "36"][bwlog2]);
        } else {
            ti.cfg("DATA_WIDTH_B", "0");
        }
        ti.cfg("DOA_REG", "0");
        ti.cfg("DOB_REG", "0");
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
                ti.cfg(&format!("INIT_{i:02x}"), &fmt_hex(&init));
            } else {
                ti.cfg(&format!("INIT_{i:02X}"), &fmt_hex(&init));
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
                ti.cfg(&format!("INIT_{i:02X}_{ul}"), &fmt_hex(&init));
            } else if init_lowercase(mode) {
                ti.cfg(&format!("INIT_{i:02x}"), &fmt_hex(&init));
            } else {
                ti.cfg(&format!("INIT_{i:02X}"), &fmt_hex(&init));
            }
        }
        if awlog2 >= 3 || (dp && bwlog2 >= 3) {
            for i in 0..8 {
                let init = ctx.gen_bits(256);
                inst.param_bits(&format!("INITP_{i:02X}"), &init);
                if mode == Mode::Virtex5 && !is_36 {
                    ti.cfg(&format!("INITP_{i:02X}_{ul}"), &fmt_hex(&init));
                } else if init_lowercase(mode) {
                    ti.cfg(&format!("INITP_{i:02x}"), &fmt_hex(&init));
                } else {
                    ti.cfg(&format!("INITP_{i:02X}"), &fmt_hex(&init));
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

    let tab_sp = [
        ("", "A", awlog2, aw, awp)
    ];
    let tab_dp = [
        ("A", "A", awlog2, aw, awp),
        ("B", "B", bwlog2, bw, bwp)
    ];

    for &(vl, xl, wlog2, w, wp) in if dp {&tab_dp[..]} else {&tab_sp[..]} {
        let top = if sz == 4 {12} else {14};
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
                        ti.pin_in(&format!("ADDRARDADDR{ul}{i}"), &addr[i-wlog2]);
                    } else {
                        ti.pin_in(&format!("ADDRBWRADDR{ul}{i}"), &addr[i-wlog2]);
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
                        ti.pin_in(&format!("ADDRA{ul}{hwi}"), &addr[i-wlog2]);
                    } else {
                        ti.pin_in(&format!("ADDRB{ul}{hwi}"), &addr[i-wlog2]);
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
                    // ISE bug? should also swizzle for wlog2 == 5?
                    hwi = match i {
                        0..=3 => i,
                        4 => 13,
                        _ => i - 1,
                    };
                } else {
                    hwi = i;
                }
                ti.pin_in(&format!("ADDR{xl}{hwi}"), &addr[i-wlog2]);
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
        if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) && xl == "A" && w == 1 && is_36 {
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
            if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) && xl == "A" && w == 8 && is_36 {
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
                    ti.pin_in(&format!("CLKARDCLK{ul}"), &clk_x);
                    ti.pin_in(&format!("ENARDEN{ul}"), &en_x);
                    ti.pin_in(&format!("RSTRAMARSTRAM{ul}"), &rst_x);
                    ti.cfg(&format!("CLKARDCLK{ul}INV"), &if clk_inv {format!("CLKARDCLK{ul}_B")} else {format!("CLKARDCLK{ul}")});
                    ti.cfg(&format!("ENARDEN{ul}INV"), &if en_inv {format!("ENARDEN{ul}_B")} else {format!("ENARDEN{ul}")});
                    ti.cfg(&format!("RSTRAMARSTRAM{ul}INV"), &if rst_inv {format!("RSTRAMARSTRAM{ul}_B")} else {format!("RSTRAMARSTRAM{ul}")});
                    for i in 0..4 {
                        ti.pin_in(&format!("WEA{ul}{i}"), &we);
                    }
                } else {
                    ti.pin_in(&format!("CLKBWRCLK{ul}"), &clk_x);
                    ti.pin_in(&format!("ENBWREN{ul}"), &en_x);
                    ti.pin_in(&format!("RSTRAMB{ul}"), &rst_x);
                    ti.cfg(&format!("CLKBWRCLK{ul}INV"), &if clk_inv {format!("CLKBWRCLK{ul}_B")} else {format!("CLKBWRCLK{ul}")});
                    ti.cfg(&format!("ENBWREN{ul}INV"), &if en_inv {format!("ENBWREN{ul}_B")} else {format!("ENBWREN{ul}")});
                    ti.cfg(&format!("RSTRAMB{ul}INV"), &if rst_inv {format!("RSTRAMB{ul}_B")} else {format!("RSTRAMB{ul}")});
                    for i in 0..4 {
                        ti.pin_in(&format!("WEBWE{ul}{i}"), &we);
                    }
                }
            }
        } else if mode == Mode::Virtex5 {
            let we = test.make_in(ctx);
            inst.connect(&format!("WE{vl}"), &we);
            for ul in &uls {
                ti.pin_in(&format!("CLK{xl}{ul}"), &clk_x);
                ti.pin_in(&format!("EN{xl}{ul}"), &en_x);
                ti.pin_in(&format!("SSR{xl}{ul}"), &rst_x);
                ti.cfg(&format!("CLK{xl}{ul}INV"), &if clk_inv {format!("CLK{xl}{ul}_B")} else {format!("CLK{xl}{ul}")});
                ti.cfg(&format!("EN{xl}{ul}INV"), &if en_inv {format!("EN{xl}{ul}_B")} else {format!("EN{xl}{ul}")});
                ti.cfg(&format!("SSR{xl}{ul}INV"), &if rst_inv {format!("SSR{xl}{ul}_B")} else {format!("SSR{xl}{ul}")});
                if !is_36 {
                    ti.pin_in(&format!("REGCLK{xl}{ul}"), &clk_x);
                    ti.cfg(&format!("REGCLK{xl}{ul}INV"), &if clk_inv {format!("REGCLK{xl}{ul}_B")} else {format!("REGCLK{xl}{ul}")});
                }
                for i in 0..4 {
                    ti.pin_in(&format!("WE{xl}{ul}{i}"), &we);
                }
            }
        } else {
            ti.pin_in(&format!("CLK{xl}"), &clk_x);
            ti.pin_in(&format!("EN{xl}"), &en_x);
            if mode == Mode::Virtex {
                ti.cfg(&format!("CLK{xl}MUX"), if clk_inv {"0"} else {"1"});
                ti.cfg(&format!("EN{xl}MUX"), &if en_inv {format!("EN{xl}_B")} else {format!("EN{xl}")});
                ti.cfg(&format!("RST{xl}MUX"), &if rst_inv {format!("RST{xl}_B")} else {format!("RST{xl}")});
                ti.pin_in(&format!("RST{xl}"), &rst_x);
            } else {
                ti.cfg(&format!("CLK{xl}INV"), &if clk_inv {format!("CLK{xl}_B")} else {format!("CLK{xl}")});
                ti.cfg(&format!("EN{xl}INV"), &if en_inv {format!("EN{xl}_B")} else {format!("EN{xl}")});
                if matches!(mode, Mode::Spartan3ADsp | Mode::Spartan6) {
                    ti.cfg(&format!("RST{xl}INV"), &if rst_inv {format!("RST{xl}_B")} else {format!("RST{xl}")});
                    ti.pin_in(&format!("RST{xl}"), &rst_x);
                } else {
                    ti.cfg(&format!("SSR{xl}INV"), &if rst_inv {format!("SSR{xl}_B")} else {format!("SSR{xl}")});
                    ti.pin_in(&format!("SSR{xl}"), &rst_x);
                }
            }
            if !bwe {
                let (we_v, we_x, we_inv) = test.make_in_inv(ctx);
                inst.connect(&format!("WE{vl}"), &we_v);
                if mode == Mode::Virtex {
                    ti.cfg(&format!("WE{xl}MUX"), &if we_inv {format!("WE{xl}_B")} else {format!("WE{xl}")});
                    ti.pin_in(&format!("WE{xl}"), &we_x);
                } else if mode == Mode::Virtex2 {
                    ti.cfg(&format!("WE{xl}INV"), &if we_inv {format!("WE{xl}_B")} else {format!("WE{xl}")});
                    ti.pin_in(&format!("WE{xl}"), &we_x);
                } else {
                    for i in 0..4 {
                        ti.cfg(&format!("WE{xl}{i}INV"), &if we_inv {format!("WE{xl}{i}_B")} else {format!("WE{xl}{i}")});
                        ti.pin_in(&format!("WE{xl}{i}"), &we_x);
                    }
                }
            } else {
                let mut we = Vec::new();
                for i in 0..(1 << wlog2 - 3) {
                    let (we_v, we_x, we_inv) = test.make_in_inv(ctx);
                    we.push(we_v);
                    for j in 0..(1 << 5 - wlog2) {
                        let ii = i + j * (1 << wlog2 - 3);
                        ti.cfg(&format!("WE{xl}{ii}INV"), &if we_inv {format!("WE{xl}{ii}_B")} else {format!("WE{xl}{ii}")});
                        ti.pin_in(&format!("WE{xl}{ii}"), &we_x);
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
                    ti.cfg(&format!("INIT_{xl}_{ul}"), &fmt_hex(&init));
                    ti.cfg(&format!("SRVAL_{xl}_{ul}"), &fmt_hex(&srval));
                } else {
                    ti.cfg(&format!("INIT_{xl}"), &fmt_hex(&init));
                    ti.cfg(&format!("SRVAL_{xl}"), &fmt_hex(&srval));
                }
            }
        }
    }
    if matches!(mode, Mode::Spartan3A | Mode::Spartan3ADsp | Mode::Spartan6) && !dp {
        if mode != Mode::Spartan6 {
            for i in 0..4 {
                ti.cfg(&format!("WEB{i}INV"), &format!("WEB{i}_B"));
                ti.pin_tie(&format!("WEB{i}"), true);
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

pub fn gen_ramb(ctx: &mut TestGenCtx, mode: Mode, test: &mut Test) {
    for _ in 0..5 {
        if matches!(mode, Mode::Virtex | Mode::Virtex2 | Mode::Spartan3A | Mode::Spartan3ADsp) {
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
            // gen_ramb_bwer(test, ctx, mode, 16);
        }
        if mode == Mode::Spartan6 {
            // gen_ramb_bwer(test, ctx, mode, 8);
        }
        if matches!(mode, Mode::Virtex4 | Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
            // RAMB16
            // RAMB16 cascade pair
            // RAMB32_S64_ECC
            // FIFO16
        }
        if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
            // FIFO18
            // FIFO18_36
            // FIFO36
            // FIFO36_72
            // RAMB18
            // RAMB18SDP
            // RAMB36
            // RAMB36SDP
        }
        if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            // FIFO18E1
            // FIFO36E1
            // RAMB18E1
            // RAMB36E1
        }
    }
}
