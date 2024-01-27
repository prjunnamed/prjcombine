use std::{collections::BTreeMap, error::Error, path::PathBuf};

use bitvec::vec::BitVec;
use clap::Parser;
use prjcombine_types::{FbId, FbMcId, IoId, Tile, TileItem, TileItemKind};
use prjcombine_xc2c as xc2c;
use prjcombine_xilinx_cpld::{
    bits::{extract_bool, extract_bool_to_enum, extract_enum, IBufOut, McOut},
    device::{Device, JtagPin, PkgPin},
    types::{
        BankId, ClkMuxVal, FoeMuxVal, IBufMode, ImuxId, ImuxInput, OeMuxVal, PTermId, RegMode,
        Slew, SrMuxVal, TermMode, XorMuxVal,
    },
};
use prjcombine_xilinx_recpld::{
    db::Database,
    fuzzdb::{FuzzDb, FuzzDbPart},
    speeddb::SpeedDb,
};
use serde_json::json;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};
use xc2c::{BitCoord, BsLayout};

const JED_MC_BITS_SMALL: &[(&str, usize)] = &[
    ("CLK_MUX", 0),
    ("CLK_INV", 0),
    ("CLK_MUX", 1),
    ("CLK_MUX", 2),
    ("CLK_DDR", 0),
    ("RST_MUX", 0),
    ("RST_MUX", 1),
    ("SET_MUX", 0),
    ("SET_MUX", 1),
    ("REG_MODE", 0),
    ("REG_MODE", 1),
    ("IOB_ZIA_MUX", 0),
    ("IOB_ZIA_MUX", 1),
    ("MC_ZIA_MUX", 0),
    ("MC_ZIA_MUX", 1),
    ("REG_D_MUX", 0),
    ("IBUF_MODE", 0),
    ("XOR_MUX", 0),
    ("XOR_MUX", 1),
    ("MC_IOB_MUX", 0),
    ("OE_MUX", 0),
    ("OE_MUX", 1),
    ("OE_MUX", 2),
    ("OE_MUX", 3),
    ("IOB_TERM_ENABLE", 0),
    ("IOB_SLEW", 0),
    ("REG_INIT", 0),
];

const JED_MC_BITS_LARGE_IOB: &[(&str, usize)] = &[
    ("CLK_MUX", 0),
    ("CLK_MUX", 1),
    ("CLK_MUX", 2),
    ("CLK_DDR", 0),
    ("CLK_INV", 0),
    ("DGE_ENABLE", 0),
    ("MC_ZIA_MUX", 0),
    ("MC_ZIA_MUX", 1),
    ("IBUF_MODE", 0),
    ("IBUF_MODE", 1),
    ("REG_D_MUX", 0),
    ("IOB_ZIA_MUX", 0),
    ("IOB_ZIA_MUX", 1),
    ("OE_MUX", 0),
    ("OE_MUX", 1),
    ("OE_MUX", 2),
    ("OE_MUX", 3),
    ("SET_MUX", 0),
    ("SET_MUX", 1),
    ("REG_INIT", 0),
    ("MC_IOB_MUX", 0),
    ("REG_MODE", 0),
    ("REG_MODE", 1),
    ("RST_MUX", 0),
    ("RST_MUX", 1),
    ("IOB_SLEW", 0),
    ("IOB_TERM_ENABLE", 0),
    ("XOR_MUX", 0),
    ("XOR_MUX", 1),
];

const JED_MC_BITS_LARGE_BURIED: &[(&str, usize)] = &[
    ("CLK_MUX", 0),
    ("CLK_MUX", 1),
    ("CLK_MUX", 2),
    ("CLK_DDR", 0),
    ("CLK_INV", 0),
    ("MC_ZIA_MUX", 0),
    ("MC_ZIA_MUX", 1),
    ("SET_MUX", 0),
    ("SET_MUX", 1),
    ("REG_INIT", 0),
    ("REG_MODE", 0),
    ("REG_MODE", 1),
    ("RST_MUX", 0),
    ("RST_MUX", 1),
    ("XOR_MUX", 0),
    ("XOR_MUX", 1),
];

fn extract_mc_bits(device: &Device, fpart: &FuzzDbPart, dd: &mut DevData) {
    let neutral = |_| true;
    for (fb, mc) in device.mcs() {
        let mcbits = &fpart.bits.fbs[fb].mcs[mc];
        let fbc = fb.to_idx() as u32 / (dd.fb_rows * 2);
        let fbr = fb.to_idx() as u32 / 2 % dd.fb_rows;
        let xlat_bit = |bit: usize| {
            let (row, column) = fpart.map.main[bit];
            let row = match dd.bs_layout {
                xc2c::BsLayout::Narrow => {
                    row - fbr * 40 - mc.to_idx() as u32 / 2 * 5 - mc.to_idx() as u32 % 2 * 3
                }
                xc2c::BsLayout::Wide => row - fbr * 48 - mc.to_idx() as u32 * 3,
            };
            let column = dd.bs_cols - 1 - column as u32;
            let column = if fb.to_idx() % 2 == 0 {
                column - dd.fb_cols[fbc as usize]
            } else {
                dd.imux_width * 2
                    + dd.mc_width * 2
                    + match dd.bs_layout {
                        xc2c::BsLayout::Narrow => 112 * 2 + 32 * 2,
                        xc2c::BsLayout::Wide => 112 * 2,
                    }
                    - 1
                    - (column - dd.fb_cols[fbc as usize])
            };
            BitCoord { row, column }
        };
        dd.mc_bits.insert(
            "CLK_MUX",
            extract_enum(
                &mcbits.clk_mux,
                |val| match val {
                    ClkMuxVal::Pt => "PT".to_string(),
                    ClkMuxVal::Fclk(fclk) => format!("FCLK{fclk}"),
                    ClkMuxVal::Ct(ct) => format!("CT{ct}"),
                    _ => unreachable!(),
                },
                xlat_bit,
                "FCLK0",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "CLK_INV",
            extract_bool(mcbits.clk_inv.unwrap(), xlat_bit),
            neutral,
        );
        dd.mc_bits.insert(
            "CLK_DDR",
            extract_bool(mcbits.ddr.unwrap(), xlat_bit),
            neutral,
        );
        if let Some(bit) = mcbits.dge_en {
            dd.mc_bits
                .insert("DGE_ENABLE", extract_bool(bit, xlat_bit), neutral);
        }
        dd.mc_bits.insert(
            "MC_ZIA_MUX",
            extract_enum(
                mcbits.mc_uim_out.as_ref().unwrap(),
                |val| {
                    match val {
                        McOut::Comb => "XOR",
                        McOut::Reg => "REG",
                    }
                    .to_string()
                },
                xlat_bit,
                "NONE",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "RST_MUX",
            extract_enum(
                &mcbits.rst_mux,
                |val| match val {
                    SrMuxVal::Pt => "PT".to_string(),
                    SrMuxVal::Fsr => "FSR".to_string(),
                    SrMuxVal::Ct(ct) => format!("CT{ct}"),
                    SrMuxVal::Gnd => "GND".to_string(),
                    _ => unreachable!(),
                },
                xlat_bit,
                "GND",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "SET_MUX",
            extract_enum(
                &mcbits.set_mux,
                |val| match val {
                    SrMuxVal::Pt => "PT".to_string(),
                    SrMuxVal::Fsr => "FSR".to_string(),
                    SrMuxVal::Ct(ct) => format!("CT{ct}"),
                    SrMuxVal::Gnd => "GND".to_string(),
                    _ => unreachable!(),
                },
                xlat_bit,
                "GND",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "REG_INIT",
            extract_bool(mcbits.init.unwrap(), xlat_bit),
            neutral,
        );
        dd.mc_bits.insert(
            "REG_MODE",
            extract_enum(
                &mcbits.reg_mode,
                |val| {
                    match val {
                        RegMode::Dff => "DFF",
                        RegMode::Tff => "TFF",
                        RegMode::Latch => "LATCH",
                        RegMode::DffCe => "DFFCE",
                    }
                    .to_string()
                },
                xlat_bit,
                "DFF",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "XOR_MUX",
            extract_enum(
                mcbits.xor_mux.as_ref().unwrap(),
                |val| {
                    match val {
                        XorMuxVal::Gnd => "GND",
                        XorMuxVal::Vcc => "VCC",
                        XorMuxVal::Pt => "PT",
                        XorMuxVal::PtInv => "PT_INV",
                    }
                    .to_string()
                },
                xlat_bit,
                "GND",
            ),
            neutral,
        );

        // IOB comes here
        if mcbits.slew.is_none() {
            continue;
        }
        dd.mc_bits.insert(
            "IBUF_MODE",
            extract_enum(
                mcbits.ibuf_mode.as_ref().unwrap(),
                |val| {
                    match val {
                        IBufMode::Plain => "PLAIN",
                        IBufMode::Schmitt => "SCHMITT",
                        IBufMode::UseVref => "USE_VREF",
                        IBufMode::IsVref => "IS_VREF",
                    }
                    .to_string()
                },
                xlat_bit,
                "PLAIN",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "REG_D_MUX",
            extract_bool_to_enum(mcbits.use_ireg.unwrap(), xlat_bit, "IBUF", "XOR"),
            neutral,
        );
        dd.mc_bits.insert(
            "IOB_ZIA_MUX",
            extract_enum(
                mcbits.ibuf_uim_out.as_ref().unwrap(),
                |val| {
                    match val {
                        IBufOut::Pad => "IBUF",
                        IBufOut::Reg => "REG",
                    }
                    .to_string()
                },
                xlat_bit,
                "NONE",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "OE_MUX",
            extract_enum(
                mcbits.oe_mux.as_ref().unwrap(),
                |val| {
                    match val {
                        OeMuxVal::Gnd => "GND".to_string(),
                        OeMuxVal::Vcc => "VCC".to_string(),
                        OeMuxVal::Pt => "PT".to_string(),
                        OeMuxVal::Foe(foe) => format!("FOE{foe}"),
                        OeMuxVal::Ct(ct) => format!("CT{ct}"),
                        OeMuxVal::OpenDrain => "OPEN_DRAIN".to_string(),
                        OeMuxVal::IsGround => "IS_GND".to_string(),
                        _ => unreachable!(),
                    }
                    .to_string()
                },
                xlat_bit,
                "GND",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "MC_IOB_MUX",
            extract_enum(
                mcbits.mc_obuf_out.as_ref().unwrap(),
                |val| {
                    match val {
                        McOut::Comb => "XOR",
                        McOut::Reg => "REG",
                    }
                    .to_string()
                },
                xlat_bit,
                "XOR",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "IOB_SLEW",
            extract_enum(
                mcbits.slew.as_ref().unwrap(),
                |val| {
                    match val {
                        Slew::Slow => "SLOW",
                        Slew::Fast => "FAST",
                    }
                    .to_string()
                },
                xlat_bit,
                "SLOW",
            ),
            neutral,
        );
        dd.mc_bits.insert(
            "IOB_TERM_ENABLE",
            extract_bool(mcbits.term.unwrap(), xlat_bit),
            neutral,
        );
    }
}

fn extract_global_bits(device: &Device, fpart: &FuzzDbPart, dd: &mut DevData) {
    let neutral = |_| true;
    let xlat_bit = |bit| {
        let (row, column) = fpart.map.main[bit];
        let column = dd.bs_cols - 1 - column as u32;
        BitCoord { row, column }
    };
    for (fclk, bit) in &fpart.bits.fclk_en {
        dd.jed_global_bits.push((format!("FCLK{fclk}_ENABLE"), 0));
        dd.global_bits.insert(
            format!("FCLK{fclk}_ENABLE"),
            extract_bool(*bit, xlat_bit),
            neutral,
        );
    }
    if device.cdr_pad.is_some() {
        dd.jed_global_bits.push(("CLKDIV_ENABLE".to_string(), 0));
        dd.global_bits.insert(
            "CLKDIV_ENABLE",
            extract_bool(fpart.bits.clkdiv_en.unwrap(), xlat_bit),
            neutral,
        );
        let item = extract_enum(
            fpart.bits.clkdiv_div.as_ref().unwrap(),
            |x| format!("{x}"),
            xlat_bit,
            "16",
        );
        for i in 0..item.bits.len() {
            dd.jed_global_bits.push(("CLKDIV_DIV".to_string(), i));
        }
        dd.global_bits.insert("CLKDIV_DIV", item, neutral);
        dd.jed_global_bits
            .push(("CLKDIV_DELAY_ENABLE".to_string(), 0));
        dd.global_bits.insert(
            "CLKDIV_DELAY_ENABLE",
            extract_bool(fpart.bits.clkdiv_dly_en.unwrap(), xlat_bit),
            neutral,
        );
    }
    dd.jed_global_bits.push(("FSR_INV".to_string(), 0));
    dd.global_bits.insert(
        "FSR_INV",
        extract_bool(fpart.bits.fsr_inv.unwrap(), xlat_bit),
        neutral,
    );
    dd.jed_global_bits.push(("FSR_ENABLE".to_string(), 0));
    dd.global_bits.insert(
        "FSR_ENABLE",
        extract_bool(fpart.bits.fsr_en.unwrap(), xlat_bit),
        neutral,
    );
    for (foe, enum_) in &fpart.bits.foe_mux_xbr {
        let item = extract_enum(
            enum_,
            |val| {
                match val {
                    FoeMuxVal::Ibuf => "IBUF",
                    FoeMuxVal::IbufInv => "IBUF_INV",
                    FoeMuxVal::Mc => "MC",
                }
                .to_string()
            },
            xlat_bit,
            "NONE",
        );
        for i in 0..item.bits.len() {
            dd.jed_global_bits.push((format!("FOE{foe}_MUX"), i));
        }
        dd.global_bits
            .insert(format!("FOE{foe}_MUX"), item, neutral)
    }
    let item = extract_enum(
        fpart.bits.term_mode.as_ref().unwrap(),
        |val| {
            match val {
                TermMode::Pullup => "PULLUP",
                TermMode::Keeper => "KEEPER",
            }
            .to_string()
        },
        xlat_bit,
        "PULLUP",
    );
    for i in 0..item.bits.len() {
        dd.jed_global_bits.push(("TERM_MODE".to_string(), i));
    }
    dd.global_bits.insert("TERM_MODE", item, neutral);
    if device.dge_pad.is_some() {
        dd.jed_global_bits.push(("DGE_ENABLE".to_string(), 0));
        dd.global_bits.insert(
            "DGE_ENABLE",
            extract_bool(fpart.bits.dge_en.unwrap(), xlat_bit),
            neutral,
        );
    }
    if !device.has_vref {
        if device.banks == 0 {
            dd.global_bits.insert(
                "IBUF_VOLT",
                extract_bool_to_enum(
                    fpart.bits.banks[BankId::from_idx(0)].ibuf_hv,
                    xlat_bit,
                    "HIGH",
                    "LOW",
                ),
                neutral,
            );
            dd.global_bits.insert(
                "OBUF_VOLT",
                extract_bool_to_enum(
                    fpart.bits.banks[BankId::from_idx(0)].obuf_hv,
                    xlat_bit,
                    "HIGH",
                    "LOW",
                ),
                neutral,
            );
        } else {
            let (ibuf_bit, obuf_bit) = if device.fbs == 2 {
                (
                    fpart.bits.term_mode.as_ref().unwrap().bits[0] + 2,
                    fpart.bits.term_mode.as_ref().unwrap().bits[0] + 1,
                )
            } else {
                (
                    fpart.bits.term_mode.as_ref().unwrap().bits[0] + 1,
                    fpart.bits.term_mode.as_ref().unwrap().bits[0] + 2,
                )
            };
            dd.global_bits.insert(
                "IBUF_VOLT",
                extract_bool_to_enum((ibuf_bit, false), xlat_bit, "HIGH", "LOW"),
                neutral,
            );
            dd.global_bits.insert(
                "OBUF_VOLT",
                extract_bool_to_enum((obuf_bit, false), xlat_bit, "HIGH", "LOW"),
                neutral,
            );
            if device.fbs == 2 {
                dd.jed_global_bits.push(("OBUF_VOLT".to_string(), 0));
                dd.jed_global_bits.push(("IBUF_VOLT".to_string(), 0));
            } else {
                dd.jed_global_bits.push(("IBUF_VOLT".to_string(), 0));
                dd.jed_global_bits.push(("OBUF_VOLT".to_string(), 0));
            }
        }
    }
    for ipad in device.ipads() {
        dd.global_bits.insert(
            format!("IPAD{ipad}_IBUF_MODE"),
            extract_enum(
                fpart.bits.ipads[ipad].ibuf_mode.as_ref().unwrap(),
                |val| {
                    match val {
                        IBufMode::Plain => "PLAIN",
                        IBufMode::Schmitt => "SCHMITT",
                        IBufMode::UseVref => "USE_VREF",
                        IBufMode::IsVref => "IS_VREF",
                    }
                    .to_string()
                },
                xlat_bit,
                "PLAIN",
            ),
            neutral,
        );
        dd.global_bits.insert(
            format!("IPAD{ipad}_TERM_ENABLE"),
            extract_bool(fpart.bits.ipads[ipad].term.unwrap(), xlat_bit),
            neutral,
        );
        dd.jed_global_bits
            .push((format!("IPAD{ipad}_IBUF_MODE"), 0));
        dd.jed_global_bits
            .push((format!("IPAD{ipad}_TERM_ENABLE"), 0));
    }
    if device.banks != 1 {
        for bank in device.banks() {
            dd.global_bits.insert(
                format!("BANK{bank}_IBUF_VOLT"),
                extract_bool_to_enum(fpart.bits.banks[bank].ibuf_hv, xlat_bit, "HIGH", "LOW"),
                neutral,
            );
            dd.global_bits.insert(
                format!("BANK{bank}_OBUF_VOLT"),
                extract_bool_to_enum(fpart.bits.banks[bank].obuf_hv, xlat_bit, "HIGH", "LOW"),
                neutral,
            );
        }
        if device.fbs <= 4 {
            for bank in device.banks() {
                dd.jed_global_bits
                    .push((format!("BANK{bank}_IBUF_VOLT"), 0));
                dd.jed_global_bits
                    .push((format!("BANK{bank}_OBUF_VOLT"), 0));
            }
        } else {
            for bank in device.banks().rev() {
                dd.jed_global_bits
                    .push((format!("BANK{bank}_IBUF_VOLT"), 0));
            }
            for bank in device.banks().rev() {
                dd.jed_global_bits
                    .push((format!("BANK{bank}_OBUF_VOLT"), 0));
            }
        }
    }
    if device.has_vref {
        dd.jed_global_bits.push(("VREF_ENABLE".to_string(), 0));
        dd.global_bits.insert(
            "VREF_ENABLE",
            extract_bool(fpart.bits.vref_en.unwrap(), xlat_bit),
            neutral,
        );
    }
}

fn extract_imux_bits(
    data: &mut [BTreeMap<String, BitVec>; 40],
    device: &Device,
    fpart: &FuzzDbPart,
    dd: &DevData,
) {
    for fb in device.fbs() {
        let fbc = fb.to_idx() / 2 / dd.fb_rows as usize;
        let imux_col = dd.fb_cols[fbc]
            + dd.mc_width
            + 112
            + if dd.bs_layout == xc2c::BsLayout::Narrow {
                32
            } else {
                0
            };
        for i in 0..40 {
            let imux = ImuxId::from_idx(i);
            let enum_ = &fpart.bits.fbs[fb].imux[imux];
            let xlat: Vec<_> = enum_
                .bits
                .iter()
                .map(|&x| {
                    let col = dd.bs_cols - 1 - fpart.map.main[x].1 as u32;
                    let col = (col - imux_col - (fb.to_idx() as u32 % 2)) / 2;
                    let col = dd.imux_width - 1 - col;
                    assert!(col < dd.imux_width);
                    col as usize
                })
                .collect();
            for (&val, bits) in &enum_.items {
                let vname = match val {
                    ImuxInput::Ibuf(IoId::Ipad(ipad)) => format!("IPAD{ipad}"),
                    ImuxInput::Ibuf(IoId::Mc((fb, mc))) => format!("IOB_{fb}_{mc}"),
                    ImuxInput::Mc((fb, mc)) => format!("MC_{fb}_{mc}"),
                    _ => unreachable!(),
                };
                let mut vbits = BitVec::repeat(true, dd.imux_width as usize);
                for (i, bit) in bits.iter().enumerate() {
                    vbits.set(xlat[i], *bit);
                }
                data[i].insert(vname, vbits);
            }
        }
    }
}

fn prep_imux_bits(imux_bits: &[BTreeMap<String, BitVec>; 40], dd: &DevData) -> Tile<BitCoord> {
    let mut tile = Tile::new();
    for i in 0..40 {
        let mut values = imux_bits[i].clone();
        values.insert(
            "VCC".to_string(),
            BitVec::repeat(true, dd.imux_width as usize),
        );
        let item = TileItem {
            bits: (0..dd.imux_width)
                .map(|j| xc2c::BitCoord {
                    row: if i < 20 || dd.bs_layout == xc2c::BsLayout::Narrow {
                        i as u32
                    } else {
                        8 + i as u32
                    },
                    column: (dd.imux_width - 1 - j) * 2,
                })
                .collect(),
            kind: TileItemKind::Enum { values },
        };
        tile.insert(format!("IM[{i}].MUX"), item, |_| true);
    }
    tile
}

fn verify_jed(device: &Device, fpart: &FuzzDbPart, dd: &DevData) {
    let check_bit = |pos: &mut usize, col: u32, row: u32| {
        let exp_coord = (row, (dd.bs_cols - 1 - col) as usize);
        assert_eq!(fpart.map.main[*pos], exp_coord);
        *pos += 1;
    };

    let mut pos = 0;
    for fb in device.fbs() {
        let fbc = fb.to_idx() as u32 / (dd.fb_rows * 2);
        let fbr = fb.to_idx() as u32 / 2 % dd.fb_rows;
        let fb_odd = fb.to_idx() % 2 == 1;
        let mc_a_col = dd.fb_cols[fbc as usize];
        let pla_or_a_col = mc_a_col + dd.mc_width;
        let pla_and_a_col = if dd.bs_layout == BsLayout::Narrow {
            pla_or_a_col + 32
        } else {
            pla_or_a_col
        };
        let imux_col = pla_and_a_col + 112;
        let pla_and_b_col = imux_col + dd.imux_width * 2;
        let pla_or_b_col = if dd.bs_layout == BsLayout::Narrow {
            pla_and_b_col + 112
        } else {
            pla_and_b_col
        };
        let mc_b_col = if dd.bs_layout == BsLayout::Narrow {
            pla_or_b_col + 32
        } else {
            pla_and_b_col + 112
        };
        for i in 0..40 {
            for j in 0..dd.imux_width {
                let exp_col = imux_col + (dd.imux_width - 1 - j) * 2 + u32::from(fb_odd);
                let exp_row = match dd.bs_layout {
                    BsLayout::Narrow => fbr * 40 + i,
                    BsLayout::Wide => fbr * 48 + if i < 20 { i } else { i + 8 },
                };
                check_bit(&mut pos, exp_col, exp_row);
            }
        }
        for pt in 0..56 {
            let xpt = match dd.bs_layout {
                BsLayout::Narrow => match pt {
                    0..=7 => pt,
                    8..=31 => 8 + (pt - 8) % 3 + (pt - 8) / 3 * 6,
                    32..=55 => 55 - (pt - 32) % 3 - (pt - 32) / 3 * 6,
                    _ => unreachable!(),
                },
                BsLayout::Wide => pt,
            };
            let exp_col_f = if fb_odd {
                pla_and_b_col + 111 - xpt * 2
            } else {
                pla_and_a_col + xpt * 2
            };
            let exp_col_t = if fb_odd {
                pla_and_b_col + 111 - xpt * 2 - 1
            } else {
                pla_and_a_col + xpt * 2 + 1
            };
            for imux in 0..40 {
                let exp_row = match dd.bs_layout {
                    BsLayout::Narrow => fbr * 40 + imux,
                    BsLayout::Wide => fbr * 48 + if imux < 20 { imux } else { imux + 8 },
                };
                assert_eq!(
                    fpart.bits.fbs[fb].pla_and[PTermId::from_idx(pt as usize)].imux
                        [ImuxId::from_idx(imux as usize)]
                    .0,
                    (pos, false)
                );
                check_bit(&mut pos, exp_col_t, exp_row);
                assert_eq!(
                    fpart.bits.fbs[fb].pla_and[PTermId::from_idx(pt as usize)].imux
                        [ImuxId::from_idx(imux as usize)]
                    .1,
                    (pos, false)
                );
                check_bit(&mut pos, exp_col_f, exp_row);
            }
        }
        for pt in 0..56 {
            for mc in device.fb_mcs() {
                let (exp_col, exp_row) = match dd.bs_layout {
                    BsLayout::Narrow => (
                        if fb_odd {
                            pla_or_b_col + 31
                                - (mc.to_idx() as u32 * 2
                                    + if pt < 32 { pt % 2 } else { 1 - pt % 2 })
                        } else {
                            pla_or_a_col
                                + (mc.to_idx() as u32 * 2
                                    + if pt < 32 { pt % 2 } else { 1 - pt % 2 })
                        },
                        fbr * 40
                            + [
                                17, 19, 22, 20, 0, 1, 3, 4, 5, 7, 8, 11, 12, 13, 15, 16, 23, 24,
                                26, 27, 28, 31, 32, 34, 35, 36, 38, 39,
                            ][pt as usize / 2],
                    ),
                    BsLayout::Wide => (
                        if fb_odd {
                            pla_and_b_col + 111 - (pt * 2 + mc.to_idx() as u32 % 2)
                        } else {
                            pla_and_a_col + (pt * 2 + mc.to_idx() as u32 % 2)
                        },
                        fbr * 48 + 20 + mc.to_idx() as u32 / 2,
                    ),
                };
                assert_eq!(
                    fpart.bits.fbs[fb].mcs[mc].pla_or[PTermId::from_idx(pt as usize)],
                    (pos, false)
                );
                check_bit(&mut pos, exp_col, exp_row);
            }
        }
        for mc in device.fb_mcs() {
            let list = if !device.has_vref {
                JED_MC_BITS_SMALL
            } else if device.io.contains_key(&IoId::Mc((fb, mc))) {
                JED_MC_BITS_LARGE_IOB
            } else {
                JED_MC_BITS_LARGE_BURIED
            };
            for &(name, bit) in list {
                let item = &dd.mc_bits.items[name];
                let coord = item.bits[bit];
                let exp_col = if fb_odd {
                    mc_b_col + dd.mc_width - 1 - coord.column
                } else {
                    mc_a_col + coord.column
                };
                let exp_row = match dd.bs_layout {
                    BsLayout::Narrow => {
                        fbr * 40 + mc.to_idx() as u32 / 2 * 5 + mc.to_idx() as u32 % 2 * 3
                    }
                    BsLayout::Wide => fbr * 48 + mc.to_idx() as u32 * 3,
                } + coord.row;
                check_bit(&mut pos, exp_col, exp_row);
            }
        }
    }
    for (name, bit) in &dd.jed_global_bits {
        let item = &dd.global_bits.items[name];
        let coord = item.bits[*bit];
        check_bit(&mut pos, coord.column, coord.row);
    }
    assert_eq!(pos, fpart.map.main.len());
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DevData {
    bs_cols: u32,
    xfer_cols: Vec<u32>,
    bs_layout: xc2c::BsLayout,
    fb_rows: u32,
    fb_cols: Vec<u32>,
    mc_width: u32,
    imux_width: u32,
    mc_bits: Tile<BitCoord>,
    global_bits: Tile<BitCoord>,
    jed_global_bits: Vec<(String, usize)>,
}

fn convert_io(io: IoId) -> (FbId, FbMcId) {
    let IoId::Mc((fb, mc)) = io else {
        unreachable!();
    };
    (fb, mc)
}

fn bit_to_json(crd: BitCoord) -> serde_json::Value {
    json!([crd.row, crd.column])
}

#[derive(Parser)]
struct Args {
    db: PathBuf,
    fdb: PathBuf,
    sdb: PathBuf,
    out: PathBuf,
    json: PathBuf,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = Database::from_file(args.db)?;
    let fdb = FuzzDb::from_file(args.fdb)?;
    let sdb = SpeedDb::from_file(args.sdb)?;

    let mut dev_data = EntityPartVec::new();
    let mut imux_bits = db
        .devices
        .map_values(|_| core::array::from_fn(|_| BTreeMap::new()));
    let mut bond_idcode = EntityPartVec::new();

    for fpart in &fdb.parts {
        let part = db
            .parts
            .iter()
            .find(|p| p.dev_name == fpart.dev_name && p.pkg_name == fpart.pkg_name)
            .unwrap();
        let device = &db.devices[part.device].device;
        let imux_width = (fpart.bits.fbs[FbId::from_idx(0)].pla_and[PTermId::from_idx(0)].imux
            [ImuxId::from_idx(0)]
        .0
         .0 / 40) as u32;
        let bs_cols = fpart.map.dims.unwrap().0 as u32;
        let bs_rows = fpart.map.dims.unwrap().1 as u32;
        let xfer_cols = fpart.map.transfer.iter().map(|&x| x as u32).collect();
        let bs_layout = match device.fbs {
            2 | 4 | 16 => xc2c::BsLayout::Wide,
            8 | 24 | 32 => xc2c::BsLayout::Narrow,
            _ => unreachable!(),
        };
        let row_height = match bs_layout {
            xc2c::BsLayout::Wide => 48,
            xc2c::BsLayout::Narrow => 40,
        };
        let mc_width = if device.fbs <= 4 {
            9
        } else {
            match bs_layout {
                xc2c::BsLayout::Narrow => 15,
                xc2c::BsLayout::Wide => 10,
            }
        };
        let fb_cols = match device.fbs {
            2 | 4 => 1,
            8 => 2,
            16 | 24 | 32 => 4,
            _ => unreachable!(),
        };
        let fb_rows = (device.fbs / fb_cols / 2) as u32;
        assert_eq!(bs_rows, fb_rows * row_height + 2);
        let fb_cols: Vec<_> = (0..fb_cols)
            .map(|i| {
                let fb = FbId::from_idx(i * (fb_rows as usize) * 2);
                let pt_bit =
                    fpart.bits.fbs[fb].mcs[FbMcId::from_idx(0)].pla_or[PTermId::from_idx(0)].0;
                let pt_col = bs_cols - 1 - (fpart.map.main[pt_bit].1 as u32);
                pt_col - mc_width
            })
            .collect();
        let mut dd = DevData {
            bs_cols,
            xfer_cols,
            bs_layout,
            fb_rows,
            fb_cols,
            mc_width,
            imux_width,
            mc_bits: Tile::new(),
            global_bits: Tile::new(),
            jed_global_bits: vec![],
        };
        extract_mc_bits(device, fpart, &mut dd);
        extract_global_bits(device, fpart, &mut dd);

        extract_imux_bits(&mut imux_bits[part.device], device, fpart, &dd);

        verify_jed(device, fpart, &dd);

        if dev_data.contains_id(part.device) {
            assert_eq!(dev_data[part.device], dd)
        } else {
            dev_data.insert(part.device, dd);
        }

        let idcode = match (&part.dev_name[..], &part.pkg_name[..]) {
            ("xc2c32", "di44") => 0x6c18,
            ("xc2c32", "cp56") => 0x6c1b,
            ("xc2c32", "vq44") => 0x6c1c,
            ("xc2c32", "pc44") => 0x6c1d,

            ("xc2c32a", "di44") => 0x6e18,
            ("xc2c32a", "cv64") => 0x6e1a,
            ("xc2c32a", "cp56") => 0x6e1b,
            ("xc2c32a", "qfg32") => 0x6e1b, // ???
            ("xc2c32a" | "xa2c32a", "vq44") => 0x6e1c,
            ("xc2c32a", "pc44") => 0x6e1d,

            ("xc2c64", "di81") => 0x6c58,
            ("xc2c64", "pc44") => 0x6c5a,
            ("xc2c64", "vq100") => 0x6c5c,
            ("xc2c64", "cp56") => 0x6c5d,
            ("xc2c64", "vq44") => 0x6c5e,

            ("xc2c64a", "di81") => 0x6e58,
            ("xc2c64a", "qfg48") => 0x6e59,
            ("xc2c64a", "pc44") => 0x6e5a,
            ("xc2c64a" | "xa2c64a", "vq100") => 0x6e5c,
            ("xc2c64a", "cv64") => 0x6e5c,
            ("xc2c64a", "cp56") => 0x6e5d,
            ("xc2c64a" | "xa2c64a", "vq44") => 0x6e5e,

            ("xc2c128", "di126") => 0x6d88,
            ("xc2c128" | "xa2c128", "vq100") => 0x6d8a,
            ("xc2c128" | "xa2c128", "cp132") => 0x6d8b,
            ("xc2c128", "tq144") => 0x6d8c,
            ("xc2c128", "cv100") => 0x6d8e,

            ("xc2c256", "di222") => 0x6d48,
            ("xc2c256" | "xa2c256", "vq100") => 0x6d4a,
            ("xc2c256", "cp132") => 0x6d4b,
            ("xc2c256" | "xa2c256", "tq144") => 0x6d4c,
            ("xc2c256", "pq208") => 0x6d4d,
            ("xc2c256", "ft256") => 0x6d4e,

            ("xc2c384", "di288") => 0x6d58,
            ("xc2c384", "fg324") => 0x6d5a,
            ("xc2c384" | "xa2c384", "tq144") => 0x6d5c,
            ("xc2c384", "pq208") => 0x6d5d,
            ("xc2c384", "ft256") => 0x6d5e,

            ("xc2c512", "di324") => 0x6d78,
            ("xc2c512", "pq208") => 0x6d7c,
            ("xc2c512", "fg324") => 0x6d7d,
            ("xc2c512", "ft256") => 0x6d7e,

            _ => panic!("unknown {} {}", part.dev_name, part.pkg_name),
        };
        bond_idcode.insert(part.package, idcode);
    }

    let devices: EntityVec<_, _> = db
        .devices
        .iter()
        .map(|(did, dev)| {
            let device = &dev.device;
            let dd = &dev_data[did];
            let imux_bits = prep_imux_bits(&imux_bits[did], dd);
            let mut io_special = BTreeMap::new();
            io_special.insert("GSR".to_string(), convert_io(device.sr_pad.unwrap()));
            for (i, &io) in &device.clk_pads {
                io_special.insert(format!("GCLK{i}"), convert_io(io));
            }
            for (i, &io) in &device.oe_pads {
                io_special.insert(format!("GOE{i}"), convert_io(io));
            }
            if let Some(io) = device.dge_pad {
                io_special.insert("DGE".to_string(), convert_io(io));
            }
            if let Some(io) = device.cdr_pad {
                io_special.insert("CDR".to_string(), convert_io(io));
            }
            xc2c::Device {
                idcode_part: match (db.devices[did].device.fbs, db.devices[did].device.banks) {
                    (2, 1) => 0x6c18,
                    (2, 2) => 0x6e18,
                    (4, 1) => 0x6c58,
                    (4, 2) => 0x6e58,
                    (8, _) => 0x6d88,
                    (16, _) => 0x6d48,
                    (24, _) => 0x6d58,
                    (32, _) => 0x6d78,
                    _ => unreachable!(),
                },
                ipads: device.ipads,
                banks: device.banks,
                io: device
                    .io
                    .iter()
                    .map(|(&k, v)| {
                        (
                            k,
                            xc2c::Io {
                                bank: xc2c::BankId::from_idx(v.bank.to_idx()),
                                pad_distance: match device.fbs {
                                    2 => (v.pad + 44 - 6) % 44,
                                    4 => (v.pad + 81 - 12) % 81,
                                    8 => (v.pad + 126 - 18) % 126,
                                    16 => (v.pad + 222 - 28) % 222,
                                    24 => (v.pad + 288 - 40) % 288,
                                    32 => (v.pad + 324 - 44) % 324,
                                    _ => unreachable!(),
                                },
                            },
                        )
                    })
                    .collect(),
                io_special,
                has_vref: device.has_vref,
                bs_cols: dd.bs_cols,
                xfer_cols: dd.xfer_cols.clone(),
                imux_width: dd.imux_width,
                mc_width: dd.mc_width,
                bs_layout: dd.bs_layout,
                fb_rows: dd.fb_rows,
                fb_cols: dd.fb_cols.clone(),
                mc_bits: dd.mc_bits.clone(),
                global_bits: dd.global_bits.clone(),
                jed_global_bits: dd.jed_global_bits.clone(),
                imux_bits,
            }
        })
        .collect();

    let mut bonds = EntityVec::new();
    let mut speeds = EntityVec::new();
    let mut parts: Vec<xc2c::Part> = vec![];
    'parts: for spart in &db.parts {
        let package = &db.packages[spart.package];
        let device = xc2c::DeviceId::from_idx(spart.device.to_idx());
        let bond = xc2c::Bond {
            idcode_part: bond_idcode[spart.package],
            pins: package
                .pins
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        match *v {
                            PkgPin::Nc => xc2c::Pad::Nc,
                            PkgPin::Gnd => xc2c::Pad::Gnd,
                            PkgPin::VccInt => xc2c::Pad::VccInt,
                            PkgPin::VccIo(bank) => {
                                xc2c::Pad::VccIo(xc2c::BankId::from_idx(bank.to_idx()))
                            }
                            PkgPin::VccAux => xc2c::Pad::VccAux,
                            PkgPin::Io(IoId::Mc((fb, mc))) => xc2c::Pad::Iob(fb, mc),
                            PkgPin::Io(IoId::Ipad(pad)) => xc2c::Pad::Ipad(pad),
                            PkgPin::Jtag(JtagPin::Tck) => xc2c::Pad::Tck,
                            PkgPin::Jtag(JtagPin::Tms) => xc2c::Pad::Tms,
                            PkgPin::Jtag(JtagPin::Tdi) => xc2c::Pad::Tdi,
                            PkgPin::Jtag(JtagPin::Tdo) => xc2c::Pad::Tdo,
                            _ => unreachable!(),
                        },
                    )
                })
                .collect(),
        };
        let bond = 'bid: {
            for (i, x) in &bonds {
                if *x == bond {
                    break 'bid i;
                }
            }
            bonds.push(bond)
        };

        for dpart in &mut parts {
            if dpart.name == spart.dev_name {
                assert_eq!(dpart.device, device);
                dpart.packages.insert(spart.pkg_name.clone(), bond);
                continue 'parts;
            }
        }
        parts.push(xc2c::Part {
            name: spart.dev_name.clone(),
            device,
            packages: [(spart.pkg_name.clone(), bond)].into_iter().collect(),
            speeds: spart
                .speeds
                .iter()
                .map(|sn| {
                    let speed = xc2c::Speed {
                        timing: sdb
                            .parts
                            .iter()
                            .find(|x| x.dev_name == spart.dev_name && &x.speed_name == sn)
                            .unwrap()
                            .timing
                            .clone(),
                    };
                    let speed = 'sid: {
                        for (i, x) in &speeds {
                            if *x == speed {
                                break 'sid i;
                            }
                        }
                        speeds.push(speed)
                    };
                    (sn.clone(), speed)
                })
                .collect(),
        })
    }

    let database = xc2c::Database {
        devices,
        bonds,
        speeds,
        parts,
        jed_mc_bits_small: JED_MC_BITS_SMALL
            .iter()
            .map(|&(item, bit)| (item.to_string(), bit))
            .collect(),
        jed_mc_bits_large_iob: JED_MC_BITS_LARGE_IOB
            .iter()
            .map(|&(item, bit)| (item.to_string(), bit))
            .collect(),
        jed_mc_bits_large_buried: JED_MC_BITS_LARGE_BURIED
            .iter()
            .map(|&(item, bit)| (item.to_string(), bit))
            .collect(),
    };
    database.to_file(args.out)?;

    let json = json! ({
        "devices": Vec::from_iter(database.devices.values().map(|device| json! ({
            "idcode_part": device.idcode_part,
            "ipads": device.ipads,
            "banks": device.banks,
            "has_vref": device.has_vref,
            "bs_cols": device.bs_cols,
            "xfer_cols": device.xfer_cols,
            "imux_width": device.imux_width,
            "mc_width": device.mc_width,
            "bs_layout": match device.bs_layout {
                xc2c::BsLayout::Narrow => "NARROW",
                xc2c::BsLayout::Wide => "WIDE",
            },
            "fb_rows": device.fb_rows,
            "fb_cols": device.fb_cols,
            "ios": serde_json::Map::from_iter(
                device.io.iter().map(|(&io, bank)| (match io {
                    IoId::Mc((fb, mc)) => format!("IOB_{fb}_{mc}"),
                    IoId::Ipad(ip) => format!("IPAD{ip}"),
                }, json!(bank)))
            ),
            "io_special": device.io_special,
            "mc_bits": device.mc_bits.to_json(bit_to_json),
            "global_bits": device.global_bits.to_json(bit_to_json),
            "jed_global_bits": device.jed_global_bits,
            "imux_bits": device.imux_bits.to_json(bit_to_json),
        }))),
        "bonds": Vec::from_iter(
            database.bonds.values().map(|bond| json!({
                "idcode_part": bond.idcode_part,
                "pins": serde_json::Map::from_iter(
                    bond.pins.iter().map(|(k, v)| {
                        (k.clone(), match v {
                            xc2c::Pad::Nc => "NC".to_string(),
                            xc2c::Pad::Gnd => "GND".to_string(),
                            xc2c::Pad::VccInt => "VCCINT".to_string(),
                            xc2c::Pad::VccIo(bank) => format!("VCCIO{bank}"),
                            xc2c::Pad::VccAux => "VCCAUX".to_string(),
                            xc2c::Pad::Iob(fb, mc) => format!("IOB_{fb}_{mc}"),
                            xc2c::Pad::Ipad(ipad) => format!("IPAD{ipad}"),
                            xc2c::Pad::Tck => "TCK".to_string(),
                            xc2c::Pad::Tms => "TMS".to_string(),
                            xc2c::Pad::Tdi => "TDI".to_string(),
                            xc2c::Pad::Tdo => "TDO".to_string(),
                        }.into())
                    })
                ),
            }))
        ),
        "speeds": &database.speeds,
        "parts": &database.parts,
        "jed_mc_bits_small": &database.jed_mc_bits_small,
        "jed_mc_bits_large_iob": &database.jed_mc_bits_large_iob,
        "jed_mc_bits_large_buried": &database.jed_mc_bits_large_buried,
    });
    std::fs::write(args.json, json.to_string())?;

    Ok(())
}
