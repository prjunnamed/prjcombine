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
                    // TODO: wtf
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
                ti.cfg(&format!("CLK{xl}MUX"), if clk_inv {"0"} else {"1"});
                ti.cfg(&format!("EN{xl}MUX"), &if en_inv {format!("EN{xl}_B")} else {format!("EN{xl}")});
                ti.cfg(&format!("RST{xl}MUX"), &if rst_inv {format!("RST{xl}_B")} else {format!("RST{xl}")});
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
                    ti.cfg(&format!("WE{xl}MUX"), &if we_inv {format!("WE{xl}_B")} else {format!("WE{xl}")});
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
        inst.param_str("RAM_MODE", if sdp {"SDP"} else {"TDP"});
    }
    if mode == Mode::Spartan6 {
        ti.cfg("RAM_MODE", if sdp {"SDP"} else {"TDP"});
    }

    if sz == 16 {
        for i in 0..64 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            ti.cfg(&format!("INIT_{i:02X}"), &fmt_hex(&init));
        }
        for i in 0..8 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INITP_{i:02X}"), &init);
            ti.cfg(&format!("INITP_{i:02X}"), &fmt_hex(&init));
        }
    } else {
        for i in 0..32 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INIT_{i:02X}"), &init);
            ti.cfg(&format!("INIT_{i:02X}"), &fmt_hex(&init));
        }
        for i in 0..4 {
            let init = ctx.gen_bits(256);
            inst.param_bits(&format!("INITP_{i:02X}"), &init);
            ti.cfg(&format!("INITP_{i:02X}"), &fmt_hex(&init));
        }
    }

    let rsttype = if ctx.rng.gen() {"ASYNC"} else {"SYNC"};
    inst.param_str("RSTTYPE", rsttype);
    ti.cfg("RSTTYPE", rsttype);

    for (a, use_, wlog2) in [('A', use_a, awlog2), ('B', use_b, bwlog2)] {
        if use_ {
            inst.param_int(&format!("DATA_WIDTH_{a}"), [1, 2, 4, 9, 18, 36][wlog2]);
            ti.cfg(&format!("DATA_WIDTH_{a}"), ["1", "2", "4", "9", "18", "36"][wlog2]);
        } else {
            inst.param_int(&format!("DATA_WIDTH_{a}"), 0);
            ti.cfg(&format!("DATA_WIDTH_{a}"), "0");
        }

        let do_reg = ctx.rng.gen_range(0..2);
        inst.param_int(&format!("DO{a}_REG"), do_reg);
        ti.cfg_int(&format!("DO{a}_REG"), do_reg);
        let wrmode = *["WRITE_FIRST", "READ_FIRST", "NO_CHANGE"].choose(&mut ctx.rng).unwrap();
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

        let init = ctx.gen_bits(if sz == 16 {36} else {18});
        let srval = ctx.gen_bits(if sz == 16 {36} else {18});
        inst.param_bits(&format!("INIT_{a}"), &init);
        inst.param_bits(&format!("SRVAL_{a}"), &srval);
        ti.cfg(&format!("INIT_{a}"), &fmt_hex(&init));
        ti.cfg(&format!("SRVAL_{a}"), &fmt_hex(&srval));

        if mode == Mode::Spartan6 {
            let en_rstram = if ctx.rng.gen() {"TRUE"} else {"FALSE"};
            inst.param_str(&format!("EN_RSTRAM_{a}"), en_rstram);
            ti.cfg(&format!("EN_RSTRAM_{a}"), en_rstram);
            let rst_priority = if ctx.rng.gen() {"CE"} else {"SR"};
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
            for i in 0..(if sz == 16 {4} else {2}) {
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
        let mut tis = [
            TgtInst::new(&["RAMB16"]),
            TgtInst::new(&["RAMB16"]),
        ];
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
                ti.pin_in(&format!("ADDRA{}", i+5), &rdaddr[i]);
                ti.pin_in(&format!("ADDRB{}", i+5), &wraddr[i]);
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
        let mut ti = TgtInst::new(&[if mode == Mode::Virtex5 {"RAMB36SDP_EXP"} else {"RAMB36E1"}]);
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
                ti.pin_in(&format!("RDADDRL{}", i+6), &rdaddr[i]);
                ti.pin_in(&format!("RDADDRU{}", i+6), &rdaddr[i]);
                ti.pin_in(&format!("WRADDRL{}", i+6), &wraddr[i]);
                ti.pin_in(&format!("WRADDRU{}", i+6), &wraddr[i]);
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
                ti.pin_in(&format!("ADDRARDADDRL{}", i+6), &rdaddr[i]);
                ti.pin_in(&format!("ADDRARDADDRU{}", i+6), &rdaddr[i]);
                ti.pin_in(&format!("ADDRBWRADDRL{}", i+6), &wraddr[i]);
                ti.pin_in(&format!("ADDRBWRADDRU{}", i+6), &wraddr[i]);
            }
            ti.pin_tie("ADDRARDADDRL15", true);
            ti.pin_tie("ADDRBWRADDRL15", true);
            for i in 0..32 {
                ti.pin_in(&format!("DIADI{i}"), &di[i]);
                ti.pin_in(&format!("DIBDI{i}"), &di[i+32]);
                ti.pin_out(&format!("DOADO{i}"), &do_[i]);
                ti.pin_out(&format!("DOBDO{i}"), &do_[i+32]);
            }
        }
        test.tgt_insts.push(ti);
    }

    test.src_insts.push(inst);
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
        inst.param_int("DATA_WIDTH", [1, 2, 4, 9, 18, 36, 72][wlog2]);
    }
    if pk == 6 {
        if sz == 16 {
            inst.param_str("FIFO_MODE", if is_sdp {"FIFO18_36"} else {"FIFO18"})
        } else {
            inst.param_str("FIFO_MODE", if is_sdp {"FIFO36_72"} else {"FIFO36"})
        }
    }

    let en_ecc_read;
    let en_ecc_write;
    let en_syn;
    let do_reg;
    if pk != 4 && is_sdp && sz == 32 {
        en_ecc_read = if ctx.rng.gen() {"TRUE"} else {"FALSE"};
        en_ecc_write = if ctx.rng.gen() {"TRUE"} else {"FALSE"};
        inst.param_str("EN_ECC_READ", en_ecc_read);
        inst.param_str("EN_ECC_WRITE", en_ecc_write);
    } else {
        en_ecc_read = "FALSE";
        en_ecc_write = "FALSE";
    }
    if pk != 4 {
        en_syn = if ctx.rng.gen() {"TRUE"} else {"FALSE"};
        do_reg = if en_syn == "FALSE" || ctx.rng.gen() {1} else {0};
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
        ti.cfg("DATA_WIDTH", ["1", "2", "4", "9", "18", "36", "72"][wlog2]);
        ti.cfg("EN_ECC_READ", en_ecc_read);
        ti.cfg("EN_ECC_WRITE", en_ecc_write);
    } else if mode == Mode::Virtex5 {
        if !is_sdp {
            ti.cfg("DATA_WIDTH", ["1", "2", "4", "9", "18", "36", "72"][wlog2]);
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
                ti.cfg("INIT", &fmt_hex(&init));
                ti.cfg("SRVAL", &fmt_hex(&srval));
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
                ti.cfg("INIT", &fmt_hex(&init));
                ti.cfg("SRVAL", &fmt_hex(&srval));
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
        ti.cfg("DATA_WIDTH", ["1", "2", "4", "9", "18", "36", "72"][wlog2]);
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
        if sz == 16  {
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
                    ti.pin_in(&format!("DIBDI{i}"), &di[i+16]);
                }
                for i in 0..2 {
                    ti.pin_in(&format!("DIPADIP{i}"), &dip[i]);
                    ti.pin_in(&format!("DIPBDIP{i}"), &dip[i+2]);
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
                    ti.pin_in(&format!("DIBDI{i}"), &di[i+32]);
                }
                for i in 0..4 {
                    ti.pin_in(&format!("DIPADIP{i}"), &dip[i]);
                    ti.pin_in(&format!("DIPBDIP{i}"), &dip[i+4]);
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
        // XXX
    }

    for p in ["EMPTY", "FULL", "ALMOSTEMPTY", "ALMOSTFULL", "RDERR", "WRERR"] {
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

    let fwft = if do_reg == 1 && en_syn == "FALSE" && ctx.rng.gen() {"TRUE"} else {"FALSE"};
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
            gen_ramb_bwer(test, ctx, mode, 16, false);
        }
        if mode == Mode::Spartan6 {
            gen_ramb_bwer(test, ctx, mode, 8, false);
            gen_ramb_bwer(test, ctx, mode, 8, true);
        }
        if matches!(mode, Mode::Virtex4 | Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
            // RAMB16
            // RAMB16 cascade pair
            gen_ramb32_ecc(test, ctx, mode);
            gen_fifo(test, ctx, mode, 16, 4);
        }
        if matches!(mode, Mode::Virtex5 | Mode::Virtex6 | Mode::Series7) {
            gen_fifo(test, ctx, mode, 16, 5);
            gen_fifo(test, ctx, mode, 32, 5);
            // RAMB18
            // RAMB18SDP
            // RAMB36
            // RAMB36SDP
        }
        if matches!(mode, Mode::Virtex6 | Mode::Series7) {
            gen_fifo(test, ctx, mode, 16, 6);
            gen_fifo(test, ctx, mode, 32, 6);
            // RAMB18E1
            // RAMB36E1
        }
    }
}
