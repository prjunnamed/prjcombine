use prjcombine_siliconblue::grid::GridKind;

pub struct Part {
    pub kind: GridKind,
    pub name: &'static str,
    pub packages: &'static [&'static str],
    pub speeds: &'static [&'static str],
    pub temps: &'static [&'static str],
}

pub const PARTS: &[Part] = &[
    // ICE1
    Part {
        kind: GridKind::Ice65L01,
        name: "iCE65L01",
        packages: &["CB81", "CB121", "CB132", "CS36", "QFN84", "VQ100", "DI"],
        speeds: &["L", "T"],
        temps: &["C", "I"],
    },
    // ICE5
    Part {
        kind: GridKind::Ice65L04,
        name: "iCE65L04",
        // XXX CB132R which is LP8K?
        packages: &["CB284", "CB196", "CB132", "CB121", "CS63", "VQ100", "DI"],
        speeds: &["L", "T"],
        temps: &["C", "I"],
    },
    // ICE8
    Part {
        kind: GridKind::Ice65L08,
        name: "iCE65L08",
        packages: &["CB284", "CB196", "CB132", "CS110", "CC72", "DI"],
        speeds: &["L", "T"],
        temps: &["C", "I"],
    },
    // ICE4P
    Part {
        kind: GridKind::Ice65P04,
        name: "iCE65P04",
        packages: &["CB284", "CB196", "CB121", "CB132", "VQ100", "CS63", "DI"],
        speeds: &["T"],
        temps: &["C", "I"],
    },
    // ICE40P01 aka ICE40P05
    Part {
        kind: GridKind::Ice40P01,
        name: "iCE40LP1K",
        packages: &[
            "CM121", "CM81", "CM49", "CM49A", "CY36", "CM36", "QN84", "CB81", "CB121", "CM36A",
            "SWG16TR", "CX36", "DI",
        ],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40P01,
        name: "iCE40HX1K",
        packages: &["CB132", "VQ100", "TQ144", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40P01,
        name: "iCE40LP640",
        packages: &["CM81", "CM49", "CM36", "CM36A", "SWG16TR"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40P01,
        name: "iCE40HX640",
        packages: &["VQ100"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40P08 aka ICE40P04
    Part {
        kind: GridKind::Ice40P08,
        name: "iCE40LP8K",
        packages: &["CM121", "CM225", "CM81", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40P08,
        name: "iCE40HX8K",
        packages: &["CM225", "CT256", "CB132", "BG121", "TQ144", "CB132R", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40P08,
        name: "iCE40LP4K",
        packages: &["CM121", "CM225", "CM81"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40P08,
        name: "iCE40HX4K",
        packages: &["TQ144", "CB132", "BG121", "CB132R"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40P03
    Part {
        kind: GridKind::Ice40P03,
        name: "iCE40LP384",
        packages: &["CM36", "CM49", "CM81", "QN32"],
        speeds: &[""],
        temps: &[""],
    },
    // // ICE40M08
    // Part {
    //     kind: GridKind::Ice40MX,
    //     name: "iCE40MX8K",
    //     packages: &["CM225", "CT256"],
    //     speeds: &[""],
    //     temps: &[""],
    // },
    // // ICE40M16
    // Part {
    //     kind: GridKind::Ice40MX,
    //     name: "iCE40MX16K",
    //     packages: &["CM323", "DI"],
    //     speeds: &[""],
    //     temps: &[""],
    // },
    // ICE40R04
    Part {
        kind: GridKind::Ice40R04,
        name: "iCE40LM4K",
        packages: &["UMG225", "SWG25TR", "CM36", "CM49", "FC36"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40R04,
        name: "iCE40LM2K",
        packages: &["SWG25TR", "CM36", "CM49"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40R04,
        name: "iCE40LM1K",
        packages: &["SWG25TR", "CM36", "CM49"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40T04
    Part {
        kind: GridKind::Ice40T04,
        name: "iCE5LP4K",
        packages: &["SWG30", "SWG36", "CM225", "CM36", "SG48", "UWG20", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40T04,
        name: "iCE5LP2K",
        packages: &["SWG30", "SWG36", "CM36", "SG48", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40T04,
        name: "iCE5LP1K",
        packages: &["SWG30", "SWG36", "CM36", "SG48", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40T01
    Part {
        kind: GridKind::Ice40T01,
        name: "iCE40UL1K",
        packages: &["CM225", "CM36", "CM36A", "SWG16", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40T01,
        name: "iCE40UL640",
        packages: &["CM36A", "SWG16", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    // ICE40T05
    Part {
        kind: GridKind::Ice40T05,
        name: "iCE40UP5K",
        packages: &["CM225", "UWG30", "SG48", "FWG49", "DI"],
        speeds: &[""],
        temps: &[""],
    },
    Part {
        kind: GridKind::Ice40T05,
        name: "iCE40UP3K",
        packages: &["UWG30", "SG48", "FWG49", "DI"],
        speeds: &[""],
        temps: &[""],
    },
];
