use std::collections::BTreeMap;

use prjcombine_interconnect::db::PinDir;
use prjcombine_siliconblue::chip::ChipKind;

pub struct Primitive {
    pub pins: BTreeMap<&'static str, PrimPin>,
    pub props: BTreeMap<&'static str, PropKind>,
}

pub struct PrimPin {
    pub dir: PinDir,
    pub is_pad: bool,
    pub len: Option<usize>,
}

#[derive(Copy, Clone, Debug)]
pub enum PropKind {
    String(&'static [&'static str]),
    BitvecHex(usize),
    BitvecBin(usize),
    BitvecBinStr(usize),
}

fn add_prim(
    prims: &mut BTreeMap<&'static str, Primitive>,
    name: &'static str,
    ins: &[&'static str],
    outs: &[&'static str],
    pads: &[(&'static str, PinDir)],
    props: &[(&'static str, PropKind)],
) {
    let mut pins = BTreeMap::new();
    for inp in ins {
        if let Some((name, len)) = inp.split_once(':') {
            let len: usize = len.parse().unwrap();
            pins.insert(
                name,
                PrimPin {
                    dir: PinDir::Input,
                    is_pad: false,
                    len: Some(len),
                },
            );
        } else {
            pins.insert(
                inp,
                PrimPin {
                    dir: PinDir::Input,
                    is_pad: false,
                    len: None,
                },
            );
        }
    }
    for outp in outs {
        if let Some((name, len)) = outp.split_once(':') {
            let len: usize = len.parse().unwrap();
            pins.insert(
                name,
                PrimPin {
                    dir: PinDir::Output,
                    is_pad: false,
                    len: Some(len),
                },
            );
        } else {
            pins.insert(
                outp,
                PrimPin {
                    dir: PinDir::Output,
                    is_pad: false,
                    len: None,
                },
            );
        }
    }
    for &(inout, dir) in pads {
        if let Some((name, len)) = inout.split_once(':') {
            let len: usize = len.parse().unwrap();
            pins.insert(
                name,
                PrimPin {
                    dir,
                    is_pad: true,
                    len: Some(len),
                },
            );
        } else {
            pins.insert(
                inout,
                PrimPin {
                    dir,
                    is_pad: true,
                    len: None,
                },
            );
        }
    }

    prims.insert(
        name,
        Primitive {
            pins,
            props: props.iter().map(|(k, v)| (*k, *v)).collect(),
        },
    );
}

pub fn get_prims(kind: ChipKind) -> BTreeMap<&'static str, Primitive> {
    let mut res = BTreeMap::new();

    add_prim(&mut res, "GND", &[], &["Y"], &[], &[]);
    add_prim(&mut res, "VCC", &[], &["Y"], &[], &[]);

    add_prim(&mut res, "SB_CARRY", &["CI", "I0", "I1"], &["CO"], &[], &[]);
    add_prim(
        &mut res,
        "SB_LUT4",
        &["I0", "I1", "I2", "I3"],
        &["O"],
        &[],
        &[("LUT_INIT", PropKind::BitvecHex(16))],
    );
    add_prim(&mut res, "SB_DFF", &["D", "C"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFR", &["D", "C", "R"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFS", &["D", "C", "S"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFSR", &["D", "C", "R"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFSS", &["D", "C", "S"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFE", &["D", "C", "E"], &["Q"], &[], &[]);
    add_prim(
        &mut res,
        "SB_DFFER",
        &["D", "C", "E", "R"],
        &["Q"],
        &[],
        &[],
    );
    add_prim(
        &mut res,
        "SB_DFFES",
        &["D", "C", "E", "S"],
        &["Q"],
        &[],
        &[],
    );
    add_prim(
        &mut res,
        "SB_DFFESR",
        &["D", "C", "E", "R"],
        &["Q"],
        &[],
        &[],
    );
    add_prim(
        &mut res,
        "SB_DFFESS",
        &["D", "C", "E", "S"],
        &["Q"],
        &[],
        &[],
    );
    add_prim(&mut res, "SB_DFFN", &["D", "C"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFNR", &["D", "C", "R"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFNS", &["D", "C", "S"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFNSR", &["D", "C", "R"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFNSS", &["D", "C", "S"], &["Q"], &[], &[]);
    add_prim(&mut res, "SB_DFFNE", &["D", "C", "E"], &["Q"], &[], &[]);
    add_prim(
        &mut res,
        "SB_DFFNER",
        &["D", "C", "E", "R"],
        &["Q"],
        &[],
        &[],
    );
    add_prim(
        &mut res,
        "SB_DFFNES",
        &["D", "C", "E", "S"],
        &["Q"],
        &[],
        &[],
    );
    add_prim(
        &mut res,
        "SB_DFFNESR",
        &["D", "C", "E", "R"],
        &["Q"],
        &[],
        &[],
    );
    add_prim(
        &mut res,
        "SB_DFFNESS",
        &["D", "C", "E", "S"],
        &["Q"],
        &[],
        &[],
    );

    if kind.is_ice65() {
        for name in ["SB_RAM4K", "SB_RAM4KNR", "SB_RAM4KNW", "SB_RAM4KNRNW"] {
            add_prim(
                &mut res,
                name,
                &[
                    if name.contains("NR") { "RCLKN" } else { "RCLK" },
                    "RCLKE",
                    "RE",
                    "RADDR:8",
                    if name.contains("NW") { "WCLKN" } else { "WCLK" },
                    "WCLKE",
                    "WE",
                    "WADDR:8",
                    "WDATA:16",
                    "MASK:16",
                ],
                &["RDATA:16"],
                &[],
                &[
                    ("INIT_0", PropKind::BitvecHex(256)),
                    ("INIT_1", PropKind::BitvecHex(256)),
                    ("INIT_2", PropKind::BitvecHex(256)),
                    ("INIT_3", PropKind::BitvecHex(256)),
                    ("INIT_4", PropKind::BitvecHex(256)),
                    ("INIT_5", PropKind::BitvecHex(256)),
                    ("INIT_6", PropKind::BitvecHex(256)),
                    ("INIT_7", PropKind::BitvecHex(256)),
                    ("INIT_8", PropKind::BitvecHex(256)),
                    ("INIT_9", PropKind::BitvecHex(256)),
                    ("INIT_A", PropKind::BitvecHex(256)),
                    ("INIT_B", PropKind::BitvecHex(256)),
                    ("INIT_C", PropKind::BitvecHex(256)),
                    ("INIT_D", PropKind::BitvecHex(256)),
                    ("INIT_E", PropKind::BitvecHex(256)),
                    ("INIT_F", PropKind::BitvecHex(256)),
                ],
            );
        }
    } else {
        for name in [
            "SB_RAM40_4K",
            "SB_RAM40_4KNR",
            "SB_RAM40_4KNW",
            "SB_RAM40_4KNRNW",
        ] {
            add_prim(
                &mut res,
                name,
                &[
                    if name.contains("NR") { "RCLKN" } else { "RCLK" },
                    "RCLKE",
                    "RE",
                    "RADDR:11",
                    if name.contains("NW") { "WCLKN" } else { "WCLK" },
                    "WCLKE",
                    "WE",
                    "WADDR:11",
                    "WDATA:16",
                    "MASK:16",
                ],
                &["RDATA:16"],
                &[],
                &[
                    ("READ_MODE", PropKind::String(&["0", "1", "2", "3"])),
                    ("WRITE_MODE", PropKind::String(&["0", "1", "2", "3"])),
                    ("INIT_0", PropKind::BitvecHex(256)),
                    ("INIT_1", PropKind::BitvecHex(256)),
                    ("INIT_2", PropKind::BitvecHex(256)),
                    ("INIT_3", PropKind::BitvecHex(256)),
                    ("INIT_4", PropKind::BitvecHex(256)),
                    ("INIT_5", PropKind::BitvecHex(256)),
                    ("INIT_6", PropKind::BitvecHex(256)),
                    ("INIT_7", PropKind::BitvecHex(256)),
                    ("INIT_8", PropKind::BitvecHex(256)),
                    ("INIT_9", PropKind::BitvecHex(256)),
                    ("INIT_A", PropKind::BitvecHex(256)),
                    ("INIT_B", PropKind::BitvecHex(256)),
                    ("INIT_C", PropKind::BitvecHex(256)),
                    ("INIT_D", PropKind::BitvecHex(256)),
                    ("INIT_E", PropKind::BitvecHex(256)),
                    ("INIT_F", PropKind::BitvecHex(256)),
                ],
            );
        }
    }
    if kind == ChipKind::Ice40M16 {
        for name in [
            "SB_RAM40_16K",
            "SB_RAM40_16KNR",
            "SB_RAM40_16KNW",
            "SB_RAM40_16KNRNW",
        ] {
            add_prim(
                &mut res,
                name,
                &[
                    if name.contains("NR") { "RCLKN" } else { "RCLK" },
                    "RCLKE",
                    "RE",
                    "RADDR:13",
                    if name.contains("NW") { "WCLKN" } else { "WCLK" },
                    "WCLKE",
                    "WE",
                    "WADDR:13",
                    "WDATA:16",
                    "MASK:16",
                ],
                &["RDATA:16"],
                &[],
                &[
                    ("READ_MODE", PropKind::String(&["0", "1", "2", "3"])),
                    ("WRITE_MODE", PropKind::String(&["0", "1", "2", "3"])),
                    ("INIT_0", PropKind::BitvecHex(256)),
                    ("INIT_1", PropKind::BitvecHex(256)),
                    ("INIT_2", PropKind::BitvecHex(256)),
                    ("INIT_3", PropKind::BitvecHex(256)),
                    ("INIT_4", PropKind::BitvecHex(256)),
                    ("INIT_5", PropKind::BitvecHex(256)),
                    ("INIT_6", PropKind::BitvecHex(256)),
                    ("INIT_7", PropKind::BitvecHex(256)),
                    ("INIT_8", PropKind::BitvecHex(256)),
                    ("INIT_9", PropKind::BitvecHex(256)),
                    ("INIT_A", PropKind::BitvecHex(256)),
                    ("INIT_B", PropKind::BitvecHex(256)),
                    ("INIT_C", PropKind::BitvecHex(256)),
                    ("INIT_D", PropKind::BitvecHex(256)),
                    ("INIT_E", PropKind::BitvecHex(256)),
                    ("INIT_F", PropKind::BitvecHex(256)),
                    ("INIT_10", PropKind::BitvecHex(256)),
                    ("INIT_11", PropKind::BitvecHex(256)),
                    ("INIT_12", PropKind::BitvecHex(256)),
                    ("INIT_13", PropKind::BitvecHex(256)),
                    ("INIT_14", PropKind::BitvecHex(256)),
                    ("INIT_15", PropKind::BitvecHex(256)),
                    ("INIT_16", PropKind::BitvecHex(256)),
                    ("INIT_17", PropKind::BitvecHex(256)),
                    ("INIT_18", PropKind::BitvecHex(256)),
                    ("INIT_19", PropKind::BitvecHex(256)),
                    ("INIT_1A", PropKind::BitvecHex(256)),
                    ("INIT_1B", PropKind::BitvecHex(256)),
                    ("INIT_1C", PropKind::BitvecHex(256)),
                    ("INIT_1D", PropKind::BitvecHex(256)),
                    ("INIT_1E", PropKind::BitvecHex(256)),
                    ("INIT_1F", PropKind::BitvecHex(256)),
                    ("INIT_20", PropKind::BitvecHex(256)),
                    ("INIT_21", PropKind::BitvecHex(256)),
                    ("INIT_22", PropKind::BitvecHex(256)),
                    ("INIT_23", PropKind::BitvecHex(256)),
                    ("INIT_24", PropKind::BitvecHex(256)),
                    ("INIT_25", PropKind::BitvecHex(256)),
                    ("INIT_26", PropKind::BitvecHex(256)),
                    ("INIT_27", PropKind::BitvecHex(256)),
                    ("INIT_28", PropKind::BitvecHex(256)),
                    ("INIT_29", PropKind::BitvecHex(256)),
                    ("INIT_2A", PropKind::BitvecHex(256)),
                    ("INIT_2B", PropKind::BitvecHex(256)),
                    ("INIT_2C", PropKind::BitvecHex(256)),
                    ("INIT_2D", PropKind::BitvecHex(256)),
                    ("INIT_2E", PropKind::BitvecHex(256)),
                    ("INIT_2F", PropKind::BitvecHex(256)),
                    ("INIT_30", PropKind::BitvecHex(256)),
                    ("INIT_31", PropKind::BitvecHex(256)),
                    ("INIT_32", PropKind::BitvecHex(256)),
                    ("INIT_33", PropKind::BitvecHex(256)),
                    ("INIT_34", PropKind::BitvecHex(256)),
                    ("INIT_35", PropKind::BitvecHex(256)),
                    ("INIT_36", PropKind::BitvecHex(256)),
                    ("INIT_37", PropKind::BitvecHex(256)),
                    ("INIT_38", PropKind::BitvecHex(256)),
                    ("INIT_39", PropKind::BitvecHex(256)),
                    ("INIT_3A", PropKind::BitvecHex(256)),
                    ("INIT_3B", PropKind::BitvecHex(256)),
                    ("INIT_3C", PropKind::BitvecHex(256)),
                    ("INIT_3D", PropKind::BitvecHex(256)),
                    ("INIT_3E", PropKind::BitvecHex(256)),
                    ("INIT_3F", PropKind::BitvecHex(256)),
                ],
            );
        }
    }

    add_prim(
        &mut res,
        "SB_IO",
        &[
            "D_OUT_0",
            "D_OUT_1",
            "OUTPUT_ENABLE",
            "CLOCK_ENABLE",
            "INPUT_CLK",
            "OUTPUT_CLK",
            "LATCH_INPUT_VALUE",
        ],
        &["D_IN_0", "D_IN_1"],
        &[("PACKAGE_PIN", PinDir::Inout)],
        &[
            ("PIN_TYPE", PropKind::BitvecBin(6)),
            ("PULLUP", PropKind::BitvecBin(1)),
            ("NEG_TRIGGER", PropKind::BitvecBin(1)),
            (
                "IO_STANDARD",
                PropKind::String(&[
                    "SB_LVCMOS",
                    "SB_SSTL2_CLASS_1",
                    "SB_SSTL2_CLASS_2",
                    "SB_SSTL18_FULL",
                    "SB_SSTL18_HALF",
                    "SB_MDDR10",
                    "SB_MDDR8",
                    "SB_MDDR4",
                    "SB_MDDR2",
                ]),
            ),
        ],
    );
    add_prim(
        &mut res,
        "SB_IO_DS",
        &[
            "D_OUT_0",
            "D_OUT_1",
            "OUTPUT_ENABLE",
            "CLOCK_ENABLE",
            "INPUT_CLK",
            "OUTPUT_CLK",
            "LATCH_INPUT_VALUE",
        ],
        &["D_IN_0", "D_IN_1"],
        &[
            ("PACKAGE_PIN", PinDir::Inout),
            ("PACKAGE_PIN_B", PinDir::Inout),
        ],
        &[
            ("PIN_TYPE", PropKind::BitvecBin(6)),
            ("NEG_TRIGGER", PropKind::BitvecBin(1)),
            (
                "IO_STANDARD",
                PropKind::String(&["SB_LVDS_INPUT", "SB_LVDS_OUTPUT", "SB_LVDS_IO"]),
            ),
        ],
    );
    if matches!(kind, ChipKind::Ice40M08 | ChipKind::Ice40M16) {
        add_prim(
            &mut res,
            "SB_IO_DLY",
            &[
                "D_OUT_0",
                "D_OUT_1",
                "OUTPUT_ENABLE",
                "CLOCK_ENABLE",
                "INPUT_CLK",
                "OUTPUT_CLK",
                "LATCH_INPUT_VALUE",
                "SDI",
                "SCLK",
                "C_R_SEL",
            ],
            &["D_IN_0", "D_IN_1", "SDO"],
            &[("PACKAGE_PIN", PinDir::Inout)],
            &[
                ("PIN_TYPE", PropKind::BitvecBin(6)),
                ("PULLUP", PropKind::BitvecBin(1)),
                ("NEG_TRIGGER", PropKind::BitvecBin(1)),
                (
                    "IO_STANDARD",
                    PropKind::String(&[
                        "SB_LVCMOS",
                        "SB_SSTL2_CLASS_1",
                        "SB_SSTL2_CLASS_2",
                        "SB_SSTL18_FULL",
                        "SB_SSTL18_HALF",
                        "SB_MDDR10",
                        "SB_MDDR8",
                        "SB_MDDR4",
                        "SB_MDDR2",
                    ]),
                ),
                ("INDELAY_VAL", PropKind::BitvecBin(6)),
                ("OUTDELAY_VAL", PropKind::BitvecBin(6)),
            ],
        );
    }
    if matches!(
        kind,
        ChipKind::Ice40T04 | ChipKind::Ice40T01 | ChipKind::Ice40T05
    ) {
        add_prim(
            &mut res,
            "SB_IO_OD",
            &[
                "DOUT0",
                "DOUT1",
                "OUTPUTENABLE",
                "CLOCKENABLE",
                "INPUTCLK",
                "OUTPUTCLK",
                "LATCHINPUTVALUE",
            ],
            &["DIN0", "DIN1"],
            &[("PACKAGEPIN", PinDir::Inout)],
            &[
                ("PIN_TYPE", PropKind::BitvecBin(6)),
                ("NEG_TRIGGER", PropKind::BitvecBin(1)),
            ],
        );
    }
    if kind == ChipKind::Ice40T05 {
        add_prim(
            &mut res,
            "SB_IO_I3C",
            &[
                "D_OUT_0",
                "D_OUT_1",
                "OUTPUT_ENABLE",
                "CLOCK_ENABLE",
                "INPUT_CLK",
                "OUTPUT_CLK",
                "LATCH_INPUT_VALUE",
                "PU_ENB",
                "WEAK_PU_ENB",
            ],
            &["D_IN_0", "D_IN_1"],
            &[("PACKAGE_PIN", PinDir::Inout)],
            &[
                ("PIN_TYPE", PropKind::BitvecBin(6)),
                ("PULLUP", PropKind::BitvecBin(1)),
                ("WEAK_PULLUP", PropKind::BitvecBin(1)),
                ("NEG_TRIGGER", PropKind::BitvecBin(1)),
                (
                    "IO_STANDARD",
                    PropKind::String(&[
                        "SB_LVCMOS",
                        "SB_SSTL2_CLASS_1",
                        "SB_SSTL2_CLASS_2",
                        "SB_SSTL18_FULL",
                        "SB_SSTL18_HALF",
                        "SB_MDDR10",
                        "SB_MDDR8",
                        "SB_MDDR4",
                        "SB_MDDR2",
                    ]),
                ),
            ],
        );
    }

    add_prim(
        &mut res,
        "SB_GB_IO",
        &[
            "D_OUT_0",
            "D_OUT_1",
            "OUTPUT_ENABLE",
            "CLOCK_ENABLE",
            "INPUT_CLK",
            "OUTPUT_CLK",
            "LATCH_INPUT_VALUE",
        ],
        &["D_IN_0", "D_IN_1", "GLOBAL_BUFFER_OUTPUT"],
        &[("PACKAGE_PIN", PinDir::Inout)],
        &[
            ("PIN_TYPE", PropKind::BitvecBin(6)),
            ("PULLUP", PropKind::BitvecBin(1)),
            ("NEG_TRIGGER", PropKind::BitvecBin(1)),
            (
                "IO_STANDARD",
                PropKind::String(&[
                    "SB_LVCMOS",
                    "SB_SSTL2_CLASS_1",
                    "SB_SSTL2_CLASS_2",
                    "SB_SSTL18_FULL",
                    "SB_SSTL18_HALF",
                    "SB_MDDR10",
                    "SB_MDDR8",
                    "SB_MDDR4",
                    "SB_MDDR2",
                ]),
            ),
        ],
    );
    add_prim(
        &mut res,
        "SB_GB",
        &["USER_SIGNAL_TO_GLOBAL_BUFFER"],
        &["GLOBAL_BUFFER_OUTPUT"],
        &[],
        &[],
    );

    if kind.is_ice65() {
        add_prim(
            &mut res,
            "SB_PLL_CORE",
            &[
                "REFERENCECLK",
                "EXTFEEDBACK",
                "DYNAMICDELAY:4",
                "BYPASS",
                "RESET",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &["PLLOUTCORE", "PLLOUTGLOBAL", "LOCK", "SDO"],
            &[],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE",
                    PropKind::String(&["DYNAMIC", "FIXED"]),
                ),
                ("FIXED_DELAY_ADJUSTMENT", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_PHASE",
                    PropKind::String(&["NONE", "0deg", "90deg", "180deg", "270deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(6)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
        add_prim(
            &mut res,
            "SB_PLL_PAD",
            &[
                "EXTFEEDBACK",
                "DYNAMICDELAY:4",
                "BYPASS",
                "RESET",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &["PLLOUTCORE", "PLLOUTGLOBAL", "LOCK", "SDO"],
            &[("PACKAGEPIN", PinDir::Inout)],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE",
                    PropKind::String(&["DYNAMIC", "FIXED"]),
                ),
                ("FIXED_DELAY_ADJUSTMENT", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_PHASE",
                    PropKind::String(&["NONE", "0deg", "90deg", "180deg", "270deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(6)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
        add_prim(
            &mut res,
            "SB_PLL_2_PAD",
            &[
                "EXTFEEDBACK",
                "DYNAMICDELAY:4",
                "BYPASS",
                "RESET",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &[
                "PLLOUTCOREA",
                "PLLOUTGLOBALA",
                "PLLOUTCOREB",
                "PLLOUTGLOBALB",
                "LOCK",
                "SDO",
            ],
            &[("PACKAGEPIN", PinDir::Inout)],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE",
                    PropKind::String(&["DYNAMIC", "FIXED"]),
                ),
                ("FIXED_DELAY_ADJUSTMENT", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_PHASE",
                    PropKind::String(&["NONE", "0deg", "90deg", "180deg", "270deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(6)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE_PORTA", PropKind::BitvecBin(1)),
                ("ENABLE_ICEGATE_PORTB", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
    } else {
        add_prim(
            &mut res,
            "SB_PLL40_CORE",
            &[
                "REFERENCECLK",
                "EXTFEEDBACK",
                "DYNAMICDELAY:8",
                "BYPASS",
                "RESETB",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &["PLLOUTCORE", "PLLOUTGLOBAL", "LOCK", "SDO"],
            &[],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_RELATIVE",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                ("SHIFTREG_DIV_MODE", PropKind::BitvecBin(2)),
                ("FDA_FEEDBACK", PropKind::BitvecBin(4)),
                ("FDA_RELATIVE", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_SELECT",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(7)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
        add_prim(
            &mut res,
            "SB_PLL40_PAD",
            &[
                "EXTFEEDBACK",
                "DYNAMICDELAY:8",
                "BYPASS",
                "RESETB",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &["PLLOUTCORE", "PLLOUTGLOBAL", "LOCK", "SDO"],
            &[("PACKAGEPIN", PinDir::Inout)],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_RELATIVE",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                ("SHIFTREG_DIV_MODE", PropKind::BitvecBin(2)),
                ("FDA_FEEDBACK", PropKind::BitvecBin(4)),
                ("FDA_RELATIVE", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_SELECT",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(7)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
        add_prim(
            &mut res,
            "SB_PLL40_PAD_DS",
            &[
                "EXTFEEDBACK",
                "DYNAMICDELAY:8",
                "BYPASS",
                "RESETB",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &["PLLOUTCORE", "PLLOUTGLOBAL", "LOCK", "SDO"],
            &[
                ("PACKAGEPIN", PinDir::Inout),
                ("PACKAGEPINB", PinDir::Inout),
            ],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_RELATIVE",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                ("SHIFTREG_DIV_MODE", PropKind::BitvecBin(2)),
                ("FDA_FEEDBACK", PropKind::BitvecBin(4)),
                ("FDA_RELATIVE", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_SELECT",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(7)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
        add_prim(
            &mut res,
            "SB_PLL40_2_PAD",
            &[
                "EXTFEEDBACK",
                "DYNAMICDELAY:8",
                "BYPASS",
                "RESETB",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &[
                "PLLOUTCOREA",
                "PLLOUTGLOBALA",
                "PLLOUTCOREB",
                "PLLOUTGLOBALB",
                "LOCK",
                "SDO",
            ],
            &[("PACKAGEPIN", PinDir::Inout)],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_RELATIVE",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                ("SHIFTREG_DIV_MODE", PropKind::BitvecBin(2)),
                ("FDA_FEEDBACK", PropKind::BitvecBin(4)),
                ("FDA_RELATIVE", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_SELECT_PORTB",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(7)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE_PORTA", PropKind::BitvecBin(1)),
                ("ENABLE_ICEGATE_PORTB", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
        add_prim(
            &mut res,
            "SB_PLL40_2F_CORE",
            &[
                "REFERENCECLK",
                "EXTFEEDBACK",
                "DYNAMICDELAY:8",
                "BYPASS",
                "RESETB",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &[
                "PLLOUTCOREA",
                "PLLOUTGLOBALA",
                "PLLOUTCOREB",
                "PLLOUTGLOBALB",
                "LOCK",
                "SDO",
            ],
            &[],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_RELATIVE",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                ("SHIFTREG_DIV_MODE", PropKind::BitvecBin(2)),
                ("FDA_FEEDBACK", PropKind::BitvecBin(4)),
                ("FDA_RELATIVE", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_SELECT_PORTA",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                (
                    "PLLOUT_SELECT_PORTB",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(7)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE_PORTA", PropKind::BitvecBin(1)),
                ("ENABLE_ICEGATE_PORTB", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
        add_prim(
            &mut res,
            "SB_PLL40_2F_PAD",
            &[
                "EXTFEEDBACK",
                "DYNAMICDELAY:8",
                "BYPASS",
                "RESETB",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &[
                "PLLOUTCOREA",
                "PLLOUTGLOBALA",
                "PLLOUTCOREB",
                "PLLOUTGLOBALB",
                "LOCK",
                "SDO",
            ],
            &[("PACKAGEPIN", PinDir::Inout)],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_RELATIVE",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                ("SHIFTREG_DIV_MODE", PropKind::BitvecBin(2)),
                ("FDA_FEEDBACK", PropKind::BitvecBin(4)),
                ("FDA_RELATIVE", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_SELECT_PORTA",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                (
                    "PLLOUT_SELECT_PORTB",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(7)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE_PORTA", PropKind::BitvecBin(1)),
                ("ENABLE_ICEGATE_PORTB", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
        add_prim(
            &mut res,
            "SB_PLL40_2F_PAD_DS",
            &[
                "EXTFEEDBACK",
                "DYNAMICDELAY:8",
                "BYPASS",
                "RESETB",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &[
                "PLLOUTCOREA",
                "PLLOUTGLOBALA",
                "PLLOUTCOREB",
                "PLLOUTGLOBALB",
                "LOCK",
                "SDO",
            ],
            &[
                ("PACKAGEPIN", PinDir::Inout),
                ("PACKAGEPINB", PinDir::Inout),
            ],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_RELATIVE",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                ("SHIFTREG_DIV_MODE", PropKind::BitvecBin(2)),
                ("FDA_FEEDBACK", PropKind::BitvecBin(4)),
                ("FDA_RELATIVE", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_SELECT_PORTA",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                (
                    "PLLOUT_SELECT_PORTB",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(7)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE_PORTA", PropKind::BitvecBin(1)),
                ("ENABLE_ICEGATE_PORTB", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
    }

    if matches!(kind, ChipKind::Ice40M08 | ChipKind::Ice40M16) {
        add_prim(
            &mut res,
            "SB_MIPI_RX_2LANE",
            &[
                "ENPDESER",
                "PU",
                "D0RXHSEN",
                "D0DTXLPP",
                "D0DTXLPN",
                "D0TXLPEN",
                "D0RXLPEN",
                "D0CDEN",
                "D0HSDESEREN",
                "D1RXHSEN",
                "D1RXLPEN",
                "D1HSDESEREN",
                "CLKRXHSEN",
                "CLKRXLPEN",
            ],
            &[
                "D0DRXLPP",
                "D0DRXLPN",
                "D0DCDP",
                "D0DCDN",
                "D0HSRXDATA:8",
                "D0HSBYTECLKD",
                "D0SYNC",
                "D0ERRSYNC",
                "D0NOSYNC",
                "D1DRXLPP",
                "D1DRXLPN",
                "D1HSRXDATA:8",
                "D1SYNC",
                "D1ERRSYNC",
                "D1NOSYNC",
                "CLKDRXLPP",
                "CLKDRXLPN",
                "CLKHSBYTE",
            ],
            &[
                ("DP0", PinDir::Input),
                ("DN0", PinDir::Input),
                ("DP1", PinDir::Input),
                ("DN1", PinDir::Input),
                ("CKP", PinDir::Input),
                ("CKN", PinDir::Input),
            ],
            &[],
        );
        add_prim(
            &mut res,
            "SB_MIPI_TX_4LANE",
            &[
                "PU",
                "LBEN",
                "ROUTCAL:2",
                "ENPDESER",
                "PDCKG",
                "D0OPMODE",
                "D0DTXLPP",
                "D0DTXLPN",
                "D0TXLPEN",
                "D0RXLPEN",
                "D0CDEN",
                "D0TXHSPD",
                "D0TXHSEN",
                "D0HSTXDATA:8",
                "D0HSSEREN",
                "D0RXHSEN",
                "D0HSDESEREN",
                "D1DTXLPP",
                "D1DTXLPN",
                "D1TXLPEN",
                "D1RXLPEN",
                "D1CDEN",
                "D1TXHSPD",
                "D1TXHSEN",
                "D1HSTXDATA:8",
                "D1HSSEREN",
                "D1RXHSEN",
                "D1HSDESEREN",
                "D2DTXLPP",
                "D2DTXLPN",
                "D2TXLPEN",
                "D2RXLPEN",
                "D2CDEN",
                "D2TXHSPD",
                "D2TXHSEN",
                "D2HSTXDATA:8",
                "D2HSSEREN",
                "D2RXHSEN",
                "D2HSDESEREN",
                "D3DTXLPP",
                "D3DTXLPN",
                "D3TXLPEN",
                "D3RXLPEN",
                "D3CDEN",
                "D3TXHSPD",
                "D3TXHSEN",
                "D3HSTXDATA:8",
                "D3HSSEREN",
                "D3RXHSEN",
                "D3HSDESEREN",
                "PLLPU",
                "PLLREF",
                "PLLCFGSRDI",
                "PLLCFGSRRESET",
                "PLLCFGSRCLK",
            ],
            &[
                "D0DRXLPP",
                "D0DRXLPN",
                "D0DCDP",
                "D0DCDN",
                "D0HSRXDATA:8",
                "D0HSBYTECLKD",
                "D0SYNC",
                "D0ERRSYNC",
                "D0HSBYTECLKSNOSYNC",
                "D1DRXLPP",
                "D1DRXLPN",
                "D1DCDP",
                "D1DCDN",
                "D1HSRXDATA:8",
                "D1SYNC",
                "D1ERRSYNC",
                "D1NOSYNC",
                "D2DRXLPP",
                "D2DRXLPN",
                "D2DCDP",
                "D2DCDN",
                "D2HSRXDATA:8",
                "D2SYNC",
                "D2ERRSYNC",
                "D2NOSYNC",
                "D3DRXLPP",
                "D3DRXLPN",
                "D3DCDP",
                "D3DCDN",
                "D3HSRXDATA:8",
                "D3SYNC",
                "D3ERRSYNC",
                "D3NOSYNC",
                "PLLLOCK",
                "PLLCFGSRDO",
            ],
            &[
                ("DP0", PinDir::Inout),
                ("DN0", PinDir::Inout),
                ("DP1", PinDir::Inout),
                ("DN1", PinDir::Inout),
                ("DP2", PinDir::Inout),
                ("DN2", PinDir::Inout),
                ("DP3", PinDir::Inout),
                ("DN3", PinDir::Inout),
                ("CKP", PinDir::Inout),
                ("CKN", PinDir::Inout),
            ],
            &[
                ("DIVR", PropKind::BitvecBin(5)),
                ("DIVF", PropKind::BitvecBin(8)),
                ("DIVQ", PropKind::BitvecBin(2)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
                ("TEST_BITS", PropKind::BitvecBin(4)),
            ],
        );
        add_prim(
            &mut res,
            "SB_TMDS_deserializer",
            &[
                "RSTNdeser",
                "RSTNpll",
                "EN",
                "PHASELch0:4",
                "PHASELch1:4",
                "PHASELch2:4",
                "EXTFEEDBACK",
                "DYNAMICDELAY:8",
                "BYPASS",
                "LATCHINPUTVALUE",
                "SDI",
                "SCLK",
            ],
            &[
                "PLLlock",
                "PLLOUTGLOBALclkx1",
                "PLLOUTCOREclkx1",
                "PLLOUTGLOBALclkx5",
                "PLLOUTCOREclkx5",
                "RAWDATAch0:10",
                "RAWDATAch1:10",
                "RAWDATAch2:10",
                "SDO",
            ],
            &[
                ("TMDSch0p", PinDir::Input),
                ("TMDSch0n", PinDir::Input),
                ("TMDSch1p", PinDir::Input),
                ("TMDSch1n", PinDir::Input),
                ("TMDSch2p", PinDir::Input),
                ("TMDSch2n", PinDir::Input),
                ("TMDSclkp", PinDir::Input),
                ("TMDSclkn", PinDir::Input),
            ],
            &[
                (
                    "FEEDBACK_PATH",
                    PropKind::String(&["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                (
                    "DELAY_ADJUSTMENT_MODE_RELATIVE",
                    PropKind::String(&["FIXED", "DYNAMIC"]),
                ),
                ("SHIFTREG_DIV_MODE", PropKind::BitvecBin(2)),
                ("FDA_FEEDBACK", PropKind::BitvecBin(4)),
                ("FDA_RELATIVE", PropKind::BitvecBin(4)),
                (
                    "PLLOUT_SELECT_PORTA",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                (
                    "PLLOUT_SELECT_PORTB",
                    PropKind::String(&["GENCLK", "GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"]),
                ),
                ("DIVR", PropKind::BitvecBin(4)),
                ("DIVF", PropKind::BitvecBin(7)),
                ("DIVQ", PropKind::BitvecBin(3)),
                ("FILTER_RANGE", PropKind::BitvecBin(3)),
                ("ENABLE_ICEGATE_PORTA", PropKind::BitvecBin(1)),
                ("ENABLE_ICEGATE_PORTB", PropKind::BitvecBin(1)),
                ("TEST_MODE", PropKind::BitvecBin(1)),
            ],
        );
    }

    if matches!(
        kind,
        ChipKind::Ice40M16 | ChipKind::Ice40T04 | ChipKind::Ice40T05
    ) {
        add_prim(
            &mut res,
            "SB_MAC16",
            &[
                "A:16",
                "B:16",
                "C:16",
                "D:16",
                "CLK",
                "CE",
                "IRSTTOP",
                "IRSTBOT",
                "ORSTTOP",
                "ORSTBOT",
                "AHOLD",
                "BHOLD",
                "CHOLD",
                "DHOLD",
                "OHOLDTOP",
                "OHOLDBOT",
                "OLOADTOP",
                "OLOADBOT",
                "ADDSUBTOP",
                "ADDSUBBOT",
                "CI",
                "ACCUMCI",
                "SIGNEXTIN",
            ],
            &["O:32", "CO", "ACCUMCO", "SIGNEXTOUT"],
            &[],
            &[
                ("NEG_TRIGGER", PropKind::BitvecBin(1)),
                ("A_REG", PropKind::BitvecBin(1)),
                ("B_REG", PropKind::BitvecBin(1)),
                ("C_REG", PropKind::BitvecBin(1)),
                ("D_REG", PropKind::BitvecBin(1)),
                ("TOP_8x8_MULT_REG", PropKind::BitvecBin(1)),
                ("BOT_8x8_MULT_REG", PropKind::BitvecBin(1)),
                ("PIPELINE_16x16_MULT_REG1", PropKind::BitvecBin(1)),
                ("PIPELINE_16x16_MULT_REG2", PropKind::BitvecBin(1)),
                ("TOPOUTPUT_SELECT", PropKind::BitvecBin(2)),
                ("BOTOUTPUT_SELECT", PropKind::BitvecBin(2)),
                ("TOPADDSUB_LOWERINPUT", PropKind::BitvecBin(2)),
                ("BOTADDSUB_LOWERINPUT", PropKind::BitvecBin(2)),
                ("TOPADDSUB_UPPERINPUT", PropKind::BitvecBin(1)),
                ("BOTADDSUB_UPPERINPUT", PropKind::BitvecBin(1)),
                ("TOPADDSUB_CARRYSELECT", PropKind::BitvecBin(2)),
                ("BOTADDSUB_CARRYSELECT", PropKind::BitvecBin(2)),
                ("MODE_8x8", PropKind::BitvecBin(1)),
                ("A_SIGNED", PropKind::BitvecBin(1)),
                ("B_SIGNED", PropKind::BitvecBin(1)),
            ],
        );
    }
    if kind == ChipKind::Ice40T05 {
        add_prim(
            &mut res,
            "SB_SPRAM256KA",
            &[
                "ADDRESS:14",
                "DATAIN:16",
                "MASKWREN:4",
                "WREN",
                "CHIPSELECT",
                "CLOCK",
                "STANDBY",
                "SLEEP",
                "POWEROFF",
                "RDMARGIN:4",
                "RDMARGINEN",
                "TEST",
            ],
            &["DATAOUT:16"],
            &[],
            &[],
        );
        add_prim(
            &mut res,
            "SB_FILTER_50NS",
            &["FILTERIN"],
            &["FILTEROUT"],
            &[],
            &[],
        );
    }

    add_prim(
        &mut res,
        "SB_WARMBOOT",
        &["BOOT", "S0", "S1"],
        &[],
        &[],
        &[],
    );

    if matches!(
        kind,
        ChipKind::Ice40R04 | ChipKind::Ice40T04 | ChipKind::Ice40T01 | ChipKind::Ice40T05
    ) {
        add_prim(
            &mut res,
            "SB_I2C",
            &[
                "SBCLKI", "SBRWI", "SBSTBI", "SBADRI0", "SBADRI1", "SBADRI2", "SBADRI3", "SBADRI4",
                "SBADRI5", "SBADRI6", "SBADRI7", "SBDATI0", "SBDATI1", "SBDATI2", "SBDATI3",
                "SBDATI4", "SBDATI5", "SBDATI6", "SBDATI7", "SCLI", "SDAI",
            ],
            &[
                "SBDATO0", "SBDATO1", "SBDATO2", "SBDATO3", "SBDATO4", "SBDATO5", "SBDATO6",
                "SBDATO7", "SBACKO", "I2CIRQ", "I2CWKUP", "SCLO", // inout?
                "SCLOE", "SDAO", "SDAOE",
            ],
            &[],
            &[
                ("I2C_SLAVE_INIT_ADDR", PropKind::BitvecBinStr(10)),
                ("BUS_ADDR74", PropKind::BitvecBinStr(4)),
            ],
        );
        add_prim(
            &mut res,
            "SB_SPI",
            &[
                "SBCLKI", "SBRWI", "SBSTBI", "SBADRI0", "SBADRI1", "SBADRI2", "SBADRI3", "SBADRI4",
                "SBADRI5", "SBADRI6", "SBADRI7", "SBDATI0", "SBDATI1", "SBDATI2", "SBDATI3",
                "SBDATI4", "SBDATI5", "SBDATI6", "SBDATI7", "MI", "SI", "SCKI", "SCSNI",
            ],
            &[
                "SBDATO0", "SBDATO1", "SBDATO2", "SBDATO3", "SBDATO4", "SBDATO5", "SBDATO6",
                "SBDATO7", "SBACKO", "SPIIRQ", "SPIWKUP", "SO", "SOE", "MO", "MOE",
                "SCKO", // inout?
                "SCKOE", "MCSNO0", "MCSNO1", "MCSNO2", "MCSNO3", "MCSNOE0", "MCSNOE1", "MCSNOE2",
                "MCSNOE3",
            ],
            &[],
            &[("BUS_ADDR74", PropKind::BitvecBinStr(4))],
        );
    }

    match kind {
        ChipKind::Ice40R04 => {
            add_prim(&mut res, "SB_HSOSC", &["ENACLKM"], &["CLKM"], &[], &[]);
            add_prim(&mut res, "SB_LSOSC", &["ENACLKK"], &["CLKK"], &[], &[]);
        }
        ChipKind::Ice40T04 | ChipKind::Ice40T01 | ChipKind::Ice40T05 => {
            add_prim(
                &mut res,
                "SB_HFOSC",
                &[
                    "CLKHFPU", "CLKHFEN", "TRIM0", "TRIM1", "TRIM2", "TRIM3", "TRIM4", "TRIM5",
                    "TRIM6", "TRIM7", "TRIM8", "TRIM9",
                ],
                &["CLKHF"],
                &[],
                &[("CLKHF_DIV", PropKind::BitvecBinStr(2))],
            );
            add_prim(
                &mut res,
                "SB_LFOSC",
                &[
                    "CLKLFPU", "CLKLFEN", "TRIM0", "TRIM1", "TRIM2", "TRIM3", "TRIM4", "TRIM5",
                    "TRIM6", "TRIM7", "TRIM8", "TRIM9",
                ],
                &["CLKLF"],
                &[],
                &[],
            );
            // add_prim(
            //     &mut res,
            //     "SMCCLK",
            //     &[],
            //     &["CLK"],
            //     &[],
            //     &[],
            // );
        }
        _ => (),
    }

    if kind == ChipKind::Ice40T04 {
        add_prim(
            &mut res,
            "SB_IR_DRV",
            &["IRLEDEN", "IRPWM", "IRPU"],
            &[],
            &[("IRLED", PinDir::Output)],
            &[("IR_CURRENT", PropKind::BitvecBinStr(10))],
        );
        add_prim(
            &mut res,
            "SB_RGB_DRV",
            &["RGBLEDEN", "RGB0PWM", "RGB1PWM", "RGB2PWM", "RGBPU"],
            &[],
            &[
                ("RGB0", PinDir::Output),
                ("RGB1", PinDir::Output),
                ("RGB2", PinDir::Output),
            ],
            &[
                ("RGB0_CURRENT", PropKind::BitvecBinStr(6)),
                ("RGB1_CURRENT", PropKind::BitvecBinStr(6)),
                ("RGB2_CURRENT", PropKind::BitvecBinStr(6)),
            ],
        );
        add_prim(
            &mut res,
            "SB_LED_DRV_CUR",
            &[
                "EN", "TRIM0", "TRIM1", "TRIM2", "TRIM3", "TRIM4", "TRIM5", "TRIM6", "TRIM7",
                "TRIM8", "TRIM9",
            ],
            &["LEDPU"],
            &[],
            &[],
        );
        add_prim(
            &mut res,
            "SB_LEDD_IP",
            &[
                "LEDDCS",
                "LEDDCLK",
                "LEDDDAT0",
                "LEDDDAT1",
                "LEDDDAT2",
                "LEDDDAT3",
                "LEDDDAT4",
                "LEDDDAT5",
                "LEDDDAT6",
                "LEDDDAT7",
                "LEDDADDR0",
                "LEDDADDR1",
                "LEDDADDR2",
                "LEDDADDR3",
                "LEDDDEN",
                "LEDDEXE",
                "LEDDRST",
            ],
            &["PWMOUT0", "PWMOUT1", "PWMOUT2", "LEDDON"],
            &[],
            &[],
        );
    }
    if matches!(kind, ChipKind::Ice40T01 | ChipKind::Ice40T05) {
        add_prim(
            &mut res,
            "SB_BARCODE_DRV",
            &[
                "BARCODEEN",
                "BARCODEPWM",
                "CURREN",
                "TRIM0",
                "TRIM1",
                "TRIM2",
                "TRIM3",
                "TRIM4",
                "TRIM5",
                "TRIM6",
                "TRIM7",
                "TRIM8",
                "TRIM9",
            ],
            &[],
            &[("BARCODE", PinDir::Output)],
            &[
                ("CURRENT_MODE", PropKind::BitvecBinStr(1)),
                ("BARCODE_CURRENT", PropKind::BitvecBinStr(4)),
            ],
        );
        add_prim(
            &mut res,
            "SB_IR400_DRV",
            &[
                "IRLEDEN", "IRPWM", "CURREN", "TRIM0", "TRIM1", "TRIM2", "TRIM3", "TRIM4", "TRIM5",
                "TRIM6", "TRIM7", "TRIM8", "TRIM9",
            ],
            &[],
            &[("IRLED", PinDir::Output)],
            &[
                ("CURRENT_MODE", PropKind::BitvecBinStr(1)),
                ("IR400_CURRENT", PropKind::BitvecBinStr(8)),
            ],
        );
        add_prim(
            &mut res,
            "SB_IR500_DRV",
            &[
                "IRLEDEN", "IRPWM", "CURREN", "TRIM0", "TRIM1", "TRIM2", "TRIM3", "TRIM4", "TRIM5",
                "TRIM6", "TRIM7", "TRIM8", "TRIM9",
            ],
            &[],
            &[("IRLED1", PinDir::Output), ("IRLED2", PinDir::Output)],
            &[
                ("CURRENT_MODE", PropKind::BitvecBinStr(1)),
                ("IR500_CURRENT", PropKind::BitvecBinStr(12)),
            ],
        );
        add_prim(
            &mut res,
            "SB_RGBA_DRV",
            &[
                "RGBLEDEN", "RGB0PWM", "RGB1PWM", "RGB2PWM", "CURREN", "TRIM0", "TRIM1", "TRIM2",
                "TRIM3", "TRIM4", "TRIM5", "TRIM6", "TRIM7", "TRIM8", "TRIM9",
            ],
            &[],
            &[
                ("RGB0", PinDir::Output),
                ("RGB1", PinDir::Output),
                ("RGB2", PinDir::Output),
            ],
            &[
                ("CURRENT_MODE", PropKind::BitvecBinStr(1)),
                ("RGB0_CURRENT", PropKind::BitvecBinStr(6)),
                ("RGB1_CURRENT", PropKind::BitvecBinStr(6)),
                ("RGB2_CURRENT", PropKind::BitvecBinStr(6)),
            ],
        );

        add_prim(
            &mut res,
            "SB_LEDDA_IP",
            &[
                "LEDDCS",
                "LEDDCLK",
                "LEDDDAT0",
                "LEDDDAT1",
                "LEDDDAT2",
                "LEDDDAT3",
                "LEDDDAT4",
                "LEDDDAT5",
                "LEDDDAT6",
                "LEDDDAT7",
                "LEDDADDR0",
                "LEDDADDR1",
                "LEDDADDR2",
                "LEDDADDR3",
                "LEDDDEN",
                "LEDDEXE",
                "LEDDRST",
            ],
            &["PWMOUT0", "PWMOUT1", "PWMOUT2", "LEDDON"],
            &[],
            &[],
        );
        add_prim(
            &mut res,
            "SB_IR_IP",
            &[
                "IRIN", "ADRI0", "ADRI1", "ADRI2", "ADRI3", "CSI", "DENI", "EXE", "LEARN", "RST",
                "WEI", "CLKI", "WDATA0", "WDATA1", "WDATA2", "WDATA3", "WDATA4", "WDATA5",
                "WDATA6", "WDATA7",
            ],
            &[
                "IROUT", "BUSY", "DRDY", "ERR", "RDATA0", "RDATA1", "RDATA2", "RDATA3", "RDATA4",
                "RDATA5", "RDATA6", "RDATA7",
            ],
            &[],
            &[],
        );
        add_prim(
            &mut res,
            "SB_RGB_IP",
            &[
                "CLK",
                "RST",
                "PARAMSOK",
                "RGBCOLOR:4",
                "BRIGHTNESS:4",
                "BREATHRAMP:4",
                "BLINKRATE:4",
            ],
            &["REDPWM", "GREENPWM", "BLUEPWM"],
            &[],
            &[],
        );

        add_prim(
            &mut res,
            "SB_I2C_FIFO",
            &[
                "CLKI", "CSI", "WEI", "STBI", "ADRI0", "ADRI1", "ADRI2", "ADRI3", "DATI0", "DATI1",
                "DATI2", "DATI3", "DATI4", "DATI5", "DATI6", "DATI7", "DATI8", "DATI9", "SCLI",
                "SDAI", "FIFORST",
            ],
            &[
                "DATO0",
                "DATO1",
                "DATO2",
                "DATO3",
                "DATO4",
                "DATO5",
                "DATO6",
                "DATO7",
                "DATO8",
                "DATO9",
                "ACKO",
                "I2CIRQ",
                "I2CWKUP",
                "SCLO", // inout?
                "SCLOE",
                "SDAO",
                "SDAOE",
                "SRWO",
                "TXFIFOAEMPTY",
                "TXFIFOEMPTY",
                "TXFIFOFULL",
                "RXFIFOAFULL",
                "RXFIFOFULL",
                "RXFIFOEMPTY",
                "MRDCMPL",
            ],
            &[],
            &[("I2C_SLAVE_ADDR", PropKind::BitvecBinStr(10))],
        );
    }

    res
}
