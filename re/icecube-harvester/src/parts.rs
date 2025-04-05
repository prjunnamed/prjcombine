use prjcombine_siliconblue::chip::ChipKind;

pub struct Part {
    pub kind: ChipKind,
    pub name: &'static str,
    pub packages: &'static [&'static str],
    pub speeds: &'static [&'static str],
    pub temps: &'static [&'static str],
}

pub const PARTS: &[Part] = &[
    // ICE1
    Part {
        kind: ChipKind::Ice65L01,
        name: "iCE65L01",
        packages: &["DI", "CB81", "CB121", "CB132", "CS36", "QFN84", "VQ100"],
        speeds: &["L", "T"],
        temps: &["C", "I"],
    },
    // ICE5
    Part {
        kind: ChipKind::Ice65L04,
        name: "iCE65L04",
        // XXX CB132R which is LP8K?
        packages: &["DI", "CB284", "CB196", "CB132", "CB121", "CS63", "VQ100"],
        speeds: &["L", "T"],
        temps: &["C", "I"],
    },
    // ICE8
    Part {
        kind: ChipKind::Ice65L08,
        name: "iCE65L08",
        packages: &["DI", "CB284", "CB196", "CB132", "CS110", "CC72"],
        speeds: &["L", "T"],
        temps: &["C", "I"],
    },
    // ICE4P
    Part {
        kind: ChipKind::Ice65P04,
        name: "iCE65P04",
        packages: &["DI", "CB284", "CB196", "CB121", "CB132", "VQ100", "CS63"],
        speeds: &["T"],
        temps: &["C", "I"],
    },
    // ICE40P01 aka ICE40P05
    Part {
        kind: ChipKind::Ice40P01,
        name: "iCE40LP1K",
        packages: &[
            "DI", "CM121", "CM81", "CM49", "CM49A", "CY36", "CM36", "QN84", "CB81", "CB121",
            "CM36A", "SWG16TR", "CX36",
        ],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40P01,
        name: "iCE40HX1K",
        packages: &["DI", "CB132", "VQ100", "TQ144"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40P01,
        name: "iCE40LP640",
        packages: &["CM81", "CM49", "CM36", "CM36A", "SWG16TR"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40P01,
        name: "iCE40HX640",
        packages: &["VQ100"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40P08 aka ICE40P04
    Part {
        kind: ChipKind::Ice40P08,
        name: "iCE40LP8K",
        packages: &["DI", "CM121", "CM225", "CM81"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40P08,
        name: "iCE40HX8K",
        packages: &["DI", "CM225", "CT256", "CB132", "BG121", "TQ144", "CB132R"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40P08,
        name: "iCE40LP4K",
        packages: &["CM225", "CM121", "CM81"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40P08,
        name: "iCE40HX4K",
        packages: &["TQ144", "CB132", "BG121", "CB132R"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40P03
    Part {
        kind: ChipKind::Ice40P03,
        name: "iCE40LP384",
        packages: &["CM81", "CM49", "CM36", "QN32"],
        speeds: &[""],
        temps: &[""],
    },
    // // ICE40M08
    // Part {
    //     kind: GridKind::Ice40M08,
    //     name: "iCE40MX8K",
    //     packages: &["CM225", "CT256"],
    //     speeds: &[""],
    //     temps: &[""],
    // },
    // // ICE40M16
    // Part {
    //     kind: GridKind::Ice40M16,
    //     name: "iCE40MX16K",
    //     packages: &["DI", "CM323"],
    //     speeds: &[""],
    //     temps: &[""],
    // },
    // ICE40R04
    Part {
        kind: ChipKind::Ice40R04,
        name: "iCE40LM4K",
        packages: &["UMG225", "SWG25TR", "CM36", "CM49", "FC36"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40R04,
        name: "iCE40LM2K",
        packages: &["SWG25TR", "CM36", "CM49"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40R04,
        name: "iCE40LM1K",
        packages: &["SWG25TR", "CM36", "CM49"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40T04
    Part {
        kind: ChipKind::Ice40T04,
        name: "iCE5LP4K",
        packages: &["DI", "SWG30", "SWG36", "CM225", "CM36", "SG48", "UWG20"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40T04,
        name: "iCE5LP2K",
        packages: &["DI", "SWG30", "SWG36", "CM36", "SG48"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40T04,
        name: "iCE5LP1K",
        packages: &["DI", "SWG30", "SWG36", "CM36", "SG48"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40T01
    Part {
        kind: ChipKind::Ice40T01,
        name: "iCE40UL1K",
        packages: &["DI", "CM225", "CM36", "CM36A", "SWG16"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40T01,
        name: "iCE40UL640",
        packages: &["DI", "CM36A", "SWG16"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40T05
    Part {
        kind: ChipKind::Ice40T05,
        name: "iCE40UP5K",
        packages: &["DI", "CM225", "UWG30", "SG48", "FWG49"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: ChipKind::Ice40T05,
        name: "iCE40UP3K",
        packages: &["DI", "UWG30", "SG48", "FWG49"],
        speeds: &[""],
        temps: &[""],
    },
];
