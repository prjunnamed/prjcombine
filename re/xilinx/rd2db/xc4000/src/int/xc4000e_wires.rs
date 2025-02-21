pub const XC4000E_WIRES: &[(&str, &str)] = &[
    ("JB_K", "IMUX.CLB.K"),
    ("JB_C1", "IMUX.CLB.C1"),
    ("JB_C2", "IMUX.CLB.C2.N"),
    ("JB_C3", "IMUX.CLB.C3.W"),
    ("JB_C4", "IMUX.CLB.C4"),
    ("JB_F1", "IMUX.CLB.F1"),
    ("JB_F2", "IMUX.CLB.F2.N"),
    ("JB_F3", "IMUX.CLB.F3.W"),
    ("JB_F4", "IMUX.CLB.F4"),
    ("JB_G1", "IMUX.CLB.G1"),
    ("JB_G2", "IMUX.CLB.G2.N"),
    ("JB_G3", "IMUX.CLB.G3.W"),
    ("JB_G4", "IMUX.CLB.G4"),
    ("JB_X", "OUT.CLB.FX"),
    ("JB_XQ", "OUT.CLB.FXQ"),
    ("JB_Y", "OUT.CLB.GY"),
    ("JB_YQ", "OUT.CLB.GYQ"),
    ("TBUF_JB_2_I", "IMUX.TBUF0.I"),
    ("TBUF_JB_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_JB_1_I", "IMUX.TBUF1.I"),
    ("TBUF_JB_1_T", "IMUX.TBUF1.TS"),
    ("TIE_JB_1_O", "GND"),
    // CENTER_SEG_0 G4B
    ("CENTER_SEG_1", "LONG.H0"), // HLL1
    // CENTER_SEG_3 F4B
    ("CENTER_SEG_5", "SINGLE.V1"), // V2
    ("CENTER_SEG_6", "LONG.H1"),   // HLL2
    // CENTER_SEG_7 C4B
    ("CENTER_SEG_9", "SINGLE.V2"),    // V3
    ("CENTER_SEG_11", "LONG.H2"),     // HLL3
    ("CENTER_SEG_16", "SINGLE.V6"),   // V7
    ("CENTER_SEG_17", "SINGLE.V3"),   // V4
    ("CENTER_SEG_18", "DOUBLE.V1.1"), // DV4
    ("CENTER_SEG_19", "SINGLE.V4"),   // V5
    ("CENTER_SEG_20", "SINGLE.V0"),   // V1
    ("CENTER_SEG_22", "LONG.V5"),     // VLL6
    ("CENTER_SEG_23", "SINGLE.V7"),   // V8
    ("CENTER_SEG_24", "LONG.V0"),     // VLL1
    ("CENTER_SEG_27", "DOUBLE.V1.0"), // DV3
    ("CENTER_SEG_28", "SINGLE.V5"),   // V6
    // CENTER_SEG_38 CINB
    ("CENTER_SEG_40", "IMUX.CLB.F3"), // F3L
    ("CENTER_SEG_30", "GCLK1"),       // K2
    ("CENTER_SEG_31", "LONG.V4"),     // VLL5
    ("CENTER_SEG_32", "LONG.V3"),     // VLL4
    ("CENTER_SEG_33", "DOUBLE.V0.0"), // DV2
    ("CENTER_SEG_34", "DOUBLE.V0.1"), // DV1
    ("CENTER_SEG_35", "LONG.V1"),     // VLL2
    ("CENTER_SEG_39", "GCLK0"),       // K1
    ("CENTER_SEG_41", "LONG.V2"),     // VLL3
    ("CENTER_SEG_43", "GCLK3"),       // K4
    ("CENTER_SEG_44", "GCLK2"),       // K3
    ("CENTER_SEG_47", "IMUX.CLB.C3"), // C3L
    ("CENTER_SEG_50", "IMUX.CLB.G3"), // G3L
    ("CENTER_SEG_52", "OUT.CLB.GY.E"),
    // CENTER_SEG_56 CINT
    ("CENTER_SEG_57", "OUT.CLB.GYQ.E"),
    ("CENTER_SEG_61", "LONG.H3"),     // HLL4
    ("CENTER_SEG_62", "IMUX.CLB.C2"), // C2T
    ("CENTER_SEG_63", "IMUX.CLB.G2"), // G2T
    ("CENTER_SEG_64", "LONG.H4"),     // HLL5
    ("CENTER_SEG_65", "IMUX.CLB.F2"), // F2T
    ("CENTER_SEG_66", "LONG.H5"),     // HLL6
    ("CENTER_SEG_67", "DOUBLE.H0.1"), // DH1
    ("CENTER_SEG_68", "OUT.CLB.FX.S"),
    ("CENTER_SEG_69", "DOUBLE.H0.0"), // DH2R
    ("CENTER_SEG_70", "OUT.CLB.FXQ.S"),
    ("CENTER_SEG_71", "DOUBLE.H0.2"),  // DH2
    ("CENTER_SEG_72", "SINGLE.H0"),    // H1R
    ("CENTER_SEG_73", "SINGLE.H0.E"),  // H1
    ("CENTER_SEG_74", "SINGLE.H1"),    // H2R
    ("CENTER_SEG_75", "SINGLE.H1.E"),  // H2
    ("CENTER_SEG_76", "SINGLE.H2"),    // H3R
    ("CENTER_SEG_77", "SINGLE.H2.E"),  // H3
    ("CENTER_SEG_78", "SINGLE.H3"),    // H4R
    ("CENTER_SEG_79", "SINGLE.H3.E"),  // H4
    ("CENTER_SEG_80", "SINGLE.H4"),    // H5R
    ("CENTER_SEG_81", "SINGLE.H4.E"),  // H5
    ("CENTER_SEG_82", "SINGLE.H5"),    // H6R
    ("CENTER_SEG_83", "SINGLE.H5.E"),  // H6
    ("CENTER_SEG_84", "SINGLE.H6"),    // H7R
    ("CENTER_SEG_85", "SINGLE.H6.E"),  // H7
    ("CENTER_SEG_86", "SINGLE.H7"),    // H8R
    ("CENTER_SEG_87", "SINGLE.H7.E"),  // H8
    ("CENTER_SEG_88", "DOUBLE.H1.0"),  // DH3R
    ("CENTER_SEG_89", "DOUBLE.H1.2"),  // DH3
    ("CENTER_SEG_90", "DOUBLE.V1.2"),  // DV3T
    ("CENTER_SEG_91", "SINGLE.V7.S"),  // V8T
    ("CENTER_SEG_92", "SINGLE.V6.S"),  // V7T
    ("CENTER_SEG_93", "SINGLE.V5.S"),  // V6T
    ("CENTER_SEG_94", "SINGLE.V4.S"),  // V5T
    ("CENTER_SEG_95", "SINGLE.V3.S"),  // V4T
    ("CENTER_SEG_96", "SINGLE.V2.S"),  // V3T
    ("CENTER_SEG_97", "SINGLE.V1.S"),  // V2T
    ("CENTER_SEG_98", "SINGLE.V0.S"),  // V1T
    ("CENTER_SEG_99", "DOUBLE.V0.2"),  // DV2T
    ("CENTER_SEG_100", "DOUBLE.H1.1"), // DH4
    // BOT
    ("PAD46_I1", "OUT.BT.IOB0.I1"),
    ("PAD46_I2", "OUT.BT.IOB0.I2"),
    ("PAD46_IK", "IMUX.IOB0.IK"),
    ("PAD46_OK", "IMUX.IOB0.OK"),
    ("PAD46_T", "IMUX.IOB0.TS"),
    ("PAD45_I1", "OUT.BT.IOB1.I1"),
    ("PAD45_I2", "OUT.BT.IOB1.I2"),
    ("PAD45_IK", "IMUX.IOB1.IK"),
    ("PAD45_OK", "IMUX.IOB1.OK"),
    ("PAD45_T", "IMUX.IOB1.TS"),
    ("DEC_KC_2_I", "IMUX.CLB.C4"),
    ("TIE_KC_1_O", "GND"),
    ("BOT_SEG_1", "IO.DOUBLE.0.S.0"),  // BDH1
    ("BOT_SEG_3", "IO.DBUF.H0"),       // DMUX_OUTER
    ("BOT_SEG_4", "IO.DBUF.H1"),       // DMUX_INNER
    ("BOT_SEG_5", "SINGLE.V0"),        // V1
    ("BOT_SEG_6", "IO.DOUBLE.0.S.2"),  // BDH1L
    ("BOT_SEG_7", "DOUBLE.V0.1"),      // DV1
    ("BOT_SEG_8", "IO.DOUBLE.0.S.1"),  // BDH2
    ("BOT_SEG_9", "SINGLE.V1"),        // V2
    ("BOT_SEG_10", "IO.DOUBLE.1.S.0"), // BDH3
    ("BOT_SEG_11", "SINGLE.V2"),       // V3
    ("BOT_SEG_12", "DOUBLE.V0.0"),     // DV2
    ("BOT_SEG_13", "IO.DOUBLE.1.S.2"), // BDH3L
    ("BOT_SEG_14", "IO.DOUBLE.1.S.1"), // BDH4
    ("BOT_SEG_15", "SINGLE.V3"),       // V4
    ("BOT_SEG_16", "IO.DOUBLE.2.S.0"), // BDH5
    ("BOT_SEG_17", "DOUBLE.V1.0"),     // DV3
    ("BOT_SEG_18", "SINGLE.V4"),       // V5
    ("BOT_SEG_19", "IO.DOUBLE.2.S.2"), // BDH5L
    ("BOT_SEG_20", "IO.DOUBLE.2.S.1"), // BDH6
    ("BOT_SEG_21", "SINGLE.V5"),       // V6
    ("BOT_SEG_22", "IO.DOUBLE.3.S.0"), // BDH7
    ("BOT_SEG_23", "DOUBLE.V1.1"),     // DV4
    ("BOT_SEG_24", "SINGLE.V6"),       // V7
    ("BOT_SEG_25", "IO.DOUBLE.3.S.2"), // BDH7L
    ("BOT_SEG_26", "IO.DOUBLE.3.S.1"), // BDH8
    ("BOT_SEG_27", "SINGLE.V7"),       // V8
    ("BOT_SEG_28", "LONG.IO.H0"),      // BHLL1
    ("BOT_SEG_29", "LONG.V0"),         // VLL1
    ("BOT_SEG_30", "LONG.IO.H1"),      // BHLL2
    ("BOT_SEG_31", "LONG.V3"),         // VLL4
    ("BOT_SEG_32", "LONG.V1"),         // VLL2
    ("BOT_SEG_33", "LONG.IO.H2"),      // BHLL3
    ("BOT_SEG_34", "LONG.V4"),         // VLL5
    ("BOT_SEG_35", "LONG.V2"),         // VLL3
    ("BOT_SEG_36", "LONG.IO.H3"),      // BHLL4
    ("BOT_SEG_37", "LONG.V5"),         // VLL6
    ("BOT_SEG_39", "DEC.H0"),          // TX1
    ("BOT_SEG_46", "DEC.H1"),          // TX2
    ("BOT_SEG_50", "DEC.H2"),          // TX3
    ("BOT_SEG_54", "DEC.H3"),          // TX4
    // BOT_SEG_65 OK_2L
    // BOT_SEG_66 IK_2L
    ("BOT_SEG_68", "GCLK0"),            // K1
    ("BOT_SEG_70", "GCLK1"),            // K2
    ("BOT_SEG_71", "GCLK2"),            // K3
    ("BOT_SEG_72", "GCLK3"),            // K4
    ("BOT_SEG_73", "OUT.BT.IOB1.I1.E"), // I1_2L
    // BOT_SEG_78 CE_2L
    ("BOT_SEG_79", "OUT.BT.IOB1.I2.E"), // I2_2L
    ("BOT_SEG_80", "LONG.H3"),          // HLL4
    ("BOT_SEG_81", "IMUX.CLB.C2"),      // C2T
    ("BOT_SEG_82", "IMUX.CLB.G2"),      // G2T
    ("BOT_SEG_83", "LONG.H4"),          // HLL5
    ("BOT_SEG_84", "IMUX.CLB.F2"),      // F2T
    ("BOT_SEG_85", "LONG.H5"),          // HLL6
    ("BOT_SEG_86", "DOUBLE.H0.1"),      // DH1
    ("BOT_SEG_87", "OUT.CLB.FX.S"),     // FXT
    ("BOT_SEG_88", "DOUBLE.H0.0"),      // DH2R
    ("BOT_SEG_89", "OUT.CLB.FXQ.S"),    // FXQT
    ("BOT_SEG_90", "DOUBLE.H0.2"),      // DH2
    ("BOT_SEG_91", "SINGLE.H0"),        // H1R
    ("BOT_SEG_92", "SINGLE.H0.E"),      // H1
    ("BOT_SEG_93", "SINGLE.H1"),        // H2R
    ("BOT_SEG_94", "SINGLE.H1.E"),      // H2
    ("BOT_SEG_95", "SINGLE.H2"),        // H3R
    ("BOT_SEG_96", "SINGLE.H2.E"),      // H3
    ("BOT_SEG_97", "SINGLE.H3"),        // H4R
    ("BOT_SEG_98", "SINGLE.H3.E"),      // H4
    ("BOT_SEG_99", "SINGLE.H4"),        // H5R
    ("BOT_SEG_100", "SINGLE.H4.E"),     // H5
    ("BOT_SEG_101", "SINGLE.H5"),       // H6R
    ("BOT_SEG_102", "SINGLE.H5.E"),     // H6
    ("BOT_SEG_103", "SINGLE.H6"),       // H7R
    ("BOT_SEG_104", "SINGLE.H6.E"),     // H7
    ("BOT_SEG_105", "SINGLE.H7"),       // H8R
    ("BOT_SEG_106", "SINGLE.H7.E"),     // H8
    ("BOT_SEG_107", "DOUBLE.H1.0"),     // DH3R
    ("BOT_SEG_108", "DOUBLE.H1.2"),     // DH3
    ("BOT_SEG_109", "DOUBLE.V1.2"),     // DV3T
    ("BOT_SEG_110", "SINGLE.V7.S"),     // V8T
    ("BOT_SEG_111", "SINGLE.V6.S"),     // V7T
    ("BOT_SEG_112", "SINGLE.V5.S"),     // V6T
    ("BOT_SEG_113", "SINGLE.V4.S"),     // V5T
    ("BOT_SEG_114", "SINGLE.V3.S"),     // V4T
    ("BOT_SEG_115", "SINGLE.V2.S"),     // V3T
    ("BOT_SEG_116", "SINGLE.V1.S"),     // V2T
    ("BOT_SEG_117", "SINGLE.V0.S"),     // V1T
    ("BOT_SEG_118", "DOUBLE.V0.2"),     // DV2T
    ("BOT_SEG_119", "DOUBLE.H1.1"),     // DH4
    // BOT_SEG_120 CLOCK_3_4
    // BOT_SEG_121 COUT
    // BOT_SEG_122 CINT
    // BOTRR
    ("I_BUFGS_BR_I_BOTRR", "OUT.IOB.CLKIN"),
    ("PAD38_I1", "OUT.BT.IOB0.I1"),
    ("PAD38_I2", "OUT.BT.IOB0.I2"),
    ("PAD38_IK", "IMUX.IOB0.IK"),
    ("PAD38_OK", "IMUX.IOB0.OK"),
    ("PAD38_T", "IMUX.IOB0.TS"),
    ("PAD37_I1", "OUT.BT.IOB1.I1"),
    ("PAD37_I2", "OUT.BT.IOB1.I2"),
    ("PAD37_IK", "IMUX.IOB1.IK"),
    ("PAD37_OK", "IMUX.IOB1.OK"),
    ("PAD37_T", "IMUX.IOB1.TS"),
    ("DEC_KH_2_I", "IMUX.CLB.C4"),
    ("TIE_KH_1_O", "GND"),
    ("BOTR_SEG_1", "IO.DOUBLE.0.S.0"),  // BDH1
    ("BOTR_SEG_3", "IO.DBUF.H0"),       // DMUX_OUTER
    ("BOTR_SEG_4", "IO.DBUF.H1"),       // DMUX_INNER
    ("BOTR_SEG_5", "SINGLE.V0"),        // V1
    ("BOTR_SEG_6", "IO.DOUBLE.0.S.2"),  // BDH1L
    ("BOTR_SEG_7", "DOUBLE.V0.1"),      // DV1
    ("BOTR_SEG_8", "IO.DOUBLE.0.S.1"),  // BDH2
    ("BOTR_SEG_9", "SINGLE.V1"),        // V2
    ("BOTR_SEG_10", "IO.DOUBLE.1.S.0"), // BDH3
    ("BOTR_SEG_11", "SINGLE.V2"),       // V3
    ("BOTR_SEG_12", "DOUBLE.V0.0"),     // DV2
    ("BOTR_SEG_13", "IO.DOUBLE.1.S.2"), // BDH3L
    ("BOTR_SEG_14", "IO.DOUBLE.1.S.1"), // BDH4
    ("BOTR_SEG_15", "SINGLE.V3"),       // V4
    ("BOTR_SEG_16", "IO.DOUBLE.2.S.0"), // BDH5
    ("BOTR_SEG_17", "DOUBLE.V1.0"),     // DV3
    ("BOTR_SEG_18", "SINGLE.V4"),       // V5
    ("BOTR_SEG_19", "IO.DOUBLE.2.S.2"), // BDH5L
    ("BOTR_SEG_20", "IO.DOUBLE.2.S.1"), // BDH6
    ("BOTR_SEG_21", "SINGLE.V5"),       // V6
    ("BOTR_SEG_22", "IO.DOUBLE.3.S.0"), // BDH7
    ("BOTR_SEG_23", "DOUBLE.V1.1"),     // DV4
    ("BOTR_SEG_24", "SINGLE.V6"),       // V7
    ("BOTR_SEG_25", "IO.DOUBLE.3.S.2"), // BDH7L
    ("BOTR_SEG_26", "IO.DOUBLE.3.S.1"), // BDH8
    ("BOTR_SEG_27", "SINGLE.V7"),       // V8
    ("BOTR_SEG_28", "LONG.IO.H0"),      // BHLL1
    ("BOTR_SEG_29", "LONG.V0"),         // VLL1
    ("BOTR_SEG_30", "LONG.IO.H1"),      // BHLL2
    ("BOTR_SEG_31", "LONG.V3"),         // VLL4
    ("BOTR_SEG_32", "LONG.V1"),         // VLL2
    ("BOTR_SEG_33", "LONG.IO.H2"),      // BHLL3
    ("BOTR_SEG_34", "LONG.V4"),         // VLL5
    ("BOTR_SEG_35", "LONG.V2"),         // VLL3
    ("BOTR_SEG_36", "LONG.IO.H3"),      // BHLL4
    ("BOTR_SEG_37", "LONG.V5"),         // VLL6
    ("BOTR_SEG_39", "DEC.H0"),          // TX1
    ("BOTR_SEG_46", "DEC.H1"),          // TX2
    ("BOTR_SEG_50", "DEC.H2"),          // TX3
    ("BOTR_SEG_54", "DEC.H3"),          // TX4
    // BOTR_SEG_65 OK_2L
    // BOTR_SEG_66 IK_2L
    ("BOTR_SEG_68", "GCLK0"),            // K1
    ("BOTR_SEG_70", "GCLK1"),            // K2
    ("BOTR_SEG_71", "GCLK2"),            // K3
    ("BOTR_SEG_72", "GCLK3"),            // K4
    ("BOTR_SEG_73", "OUT.BT.IOB1.I1.E"), // I1_2L
    // BOTR_SEG_78 CE_2L
    ("BOTR_SEG_79", "OUT.BT.IOB1.I2.E"), // I2_2L
    ("BOTR_SEG_80", "LONG.H3"),          // HLL4
    ("BOTR_SEG_81", "IMUX.CLB.C2"),      // C2T
    ("BOTR_SEG_82", "IMUX.CLB.G2"),      // G2T
    ("BOTR_SEG_83", "LONG.H4"),          // HLL5
    ("BOTR_SEG_84", "IMUX.CLB.F2"),      // F2T
    ("BOTR_SEG_85", "LONG.H5"),          // HLL6
    ("BOTR_SEG_86", "DOUBLE.H0.1"),      // DH1
    ("BOTR_SEG_87", "OUT.CLB.FX.S"),     // FXT
    ("BOTR_SEG_88", "DOUBLE.H0.0"),      // DH2R
    ("BOTR_SEG_89", "OUT.CLB.FXQ.S"),    // FXQT
    ("BOTR_SEG_90", "DOUBLE.H0.2"),      // DH2
    ("BOTR_SEG_91", "SINGLE.H0"),        // H1R
    ("BOTR_SEG_92", "SINGLE.H0.E"),      // H1
    ("BOTR_SEG_93", "SINGLE.H1"),        // H2R
    ("BOTR_SEG_94", "SINGLE.H1.E"),      // H2
    ("BOTR_SEG_95", "SINGLE.H2"),        // H3R
    ("BOTR_SEG_96", "SINGLE.H2.E"),      // H3
    ("BOTR_SEG_97", "SINGLE.H3"),        // H4R
    ("BOTR_SEG_98", "SINGLE.H3.E"),      // H4
    ("BOTR_SEG_99", "SINGLE.H4"),        // H5R
    ("BOTR_SEG_100", "SINGLE.H4.E"),     // H5
    ("BOTR_SEG_101", "SINGLE.H5"),       // H6R
    ("BOTR_SEG_102", "SINGLE.H5.E"),     // H6
    ("BOTR_SEG_103", "SINGLE.H6"),       // H7R
    ("BOTR_SEG_104", "SINGLE.H6.E"),     // H7
    ("BOTR_SEG_105", "SINGLE.H7"),       // H8R
    ("BOTR_SEG_106", "SINGLE.H7.E"),     // H8
    ("BOTR_SEG_107", "DOUBLE.H1.0"),     // DH3R
    ("BOTR_SEG_108", "DOUBLE.H1.2"),     // DH3
    ("BOTR_SEG_109", "DOUBLE.V1.2"),     // DV3T
    ("BOTR_SEG_110", "SINGLE.V7.S"),     // V8T
    ("BOTR_SEG_111", "SINGLE.V6.S"),     // V7T
    ("BOTR_SEG_112", "SINGLE.V5.S"),     // V6T
    ("BOTR_SEG_113", "SINGLE.V4.S"),     // V5T
    ("BOTR_SEG_114", "SINGLE.V3.S"),     // V4T
    ("BOTR_SEG_115", "SINGLE.V2.S"),     // V3T
    ("BOTR_SEG_116", "SINGLE.V1.S"),     // V2T
    ("BOTR_SEG_117", "SINGLE.V0.S"),     // V1T
    ("BOTR_SEG_118", "DOUBLE.V0.2"),     // DV2T
    ("BOTR_SEG_119", "DOUBLE.H1.1"),     // DH4
    // BOTR_SEG_120 CLOCK_3_4
    // BOTR_SEG_121 COUT
    // BOTR_SEG_122 CINT
    // BOTS
    ("PAD44_I1", "OUT.BT.IOB0.I1"),
    ("PAD44_I2", "OUT.BT.IOB0.I2"),
    ("PAD44_IK", "IMUX.IOB0.IK"),
    ("PAD44_OK", "IMUX.IOB0.OK"),
    ("PAD44_T", "IMUX.IOB0.TS"),
    ("PAD43_I1", "OUT.BT.IOB1.I1"),
    ("PAD43_I2", "OUT.BT.IOB1.I2"),
    ("PAD43_IK", "IMUX.IOB1.IK"),
    ("PAD43_OK", "IMUX.IOB1.OK"),
    ("PAD43_T", "IMUX.IOB1.TS"),
    ("DEC_KD_2_I", "IMUX.CLB.C4"),
    ("TIE_KD_1_O", "GND"),
    ("BOTS_SEG_1", "IO.DOUBLE.0.S.0"),  // BDH1
    ("BOTS_SEG_3", "IO.DBUF.H0"),       // DMUX_OUTER
    ("BOTS_SEG_4", "IO.DBUF.H1"),       // DMUX_INNER
    ("BOTS_SEG_5", "SINGLE.V1"),        // V2
    ("BOTS_SEG_6", "IO.DOUBLE.0.S.2"),  // BDH1L
    ("BOTS_SEG_7", "DOUBLE.V0.1"),      // DV1
    ("BOTS_SEG_8", "IO.DOUBLE.0.S.1"),  // BDH2
    ("BOTS_SEG_9", "SINGLE.V0"),        // V1
    ("BOTS_SEG_10", "IO.DOUBLE.1.S.0"), // BDH3
    ("BOTS_SEG_11", "SINGLE.V3"),       // V4
    ("BOTS_SEG_12", "DOUBLE.V0.0"),     // DV2
    ("BOTS_SEG_13", "IO.DOUBLE.1.S.2"), // BDH3L
    ("BOTS_SEG_14", "IO.DOUBLE.1.S.1"), // BDH4
    ("BOTS_SEG_15", "SINGLE.V2"),       // V3
    ("BOTS_SEG_16", "IO.DOUBLE.2.S.0"), // BDH5
    ("BOTS_SEG_17", "DOUBLE.V1.0"),     // DV3
    ("BOTS_SEG_18", "SINGLE.V5"),       // V6
    ("BOTS_SEG_19", "IO.DOUBLE.2.S.2"), // BDH5L
    ("BOTS_SEG_20", "IO.DOUBLE.2.S.1"), // BDH6
    ("BOTS_SEG_21", "SINGLE.V4"),       // V5
    ("BOTS_SEG_22", "IO.DOUBLE.3.S.0"), // BDH7
    ("BOTS_SEG_23", "DOUBLE.V1.1"),     // DV4
    ("BOTS_SEG_24", "SINGLE.V7"),       // V8
    ("BOTS_SEG_25", "IO.DOUBLE.3.S.2"), // BDH7L
    ("BOTS_SEG_26", "IO.DOUBLE.3.S.1"), // BDH8
    ("BOTS_SEG_27", "SINGLE.V6"),       // V7
    ("BOTS_SEG_28", "LONG.IO.H0"),      // BHLL1
    ("BOTS_SEG_29", "LONG.V0"),         // VLL1
    ("BOTS_SEG_30", "LONG.IO.H1"),      // BHLL2
    ("BOTS_SEG_31", "LONG.V3"),         // VLL4
    ("BOTS_SEG_32", "LONG.V1"),         // VLL2
    ("BOTS_SEG_33", "LONG.IO.H2"),      // BHLL3
    ("BOTS_SEG_34", "LONG.V4"),         // VLL5
    ("BOTS_SEG_35", "LONG.V2"),         // VLL3
    ("BOTS_SEG_36", "LONG.IO.H3"),      // BHLL4
    ("BOTS_SEG_37", "LONG.V5"),         // VLL6
    ("BOTS_SEG_39", "DEC.H0"),          // TX1
    ("BOTS_SEG_46", "DEC.H1"),          // TX2
    ("BOTS_SEG_50", "DEC.H2"),          // TX3
    ("BOTS_SEG_54", "DEC.H3"),          // TX4
    // BOTS_SEG_65 OK_2L
    // BOTS_SEG_66 IK_2L
    ("BOTS_SEG_68", "GCLK0"),            // K1
    ("BOTS_SEG_70", "GCLK1"),            // K2
    ("BOTS_SEG_71", "GCLK2"),            // K3
    ("BOTS_SEG_72", "GCLK3"),            // K4
    ("BOTS_SEG_73", "OUT.BT.IOB1.I1.E"), // I1_2L
    // BOTS_SEG_78 CE_2L
    ("BOTS_SEG_79", "OUT.BT.IOB1.I2.E"), // I2_2L
    ("BOTS_SEG_80", "LONG.H3"),          // HLL4
    ("BOTS_SEG_81", "IMUX.CLB.C2"),      // C2T
    ("BOTS_SEG_82", "IMUX.CLB.G2"),      // G2T
    ("BOTS_SEG_83", "LONG.H4"),          // HLL5
    ("BOTS_SEG_84", "IMUX.CLB.F2"),      // F2T
    ("BOTS_SEG_85", "LONG.H5"),          // HLL6
    ("BOTS_SEG_86", "DOUBLE.H0.1"),      // DH1
    ("BOTS_SEG_87", "OUT.CLB.FX.S"),     // FXT
    ("BOTS_SEG_88", "DOUBLE.H0.0"),      // DH2R
    ("BOTS_SEG_89", "OUT.CLB.FXQ.S"),    // FXQT
    ("BOTS_SEG_90", "DOUBLE.H0.2"),      // DH2
    ("BOTS_SEG_91", "SINGLE.H0"),        // H1R
    ("BOTS_SEG_92", "SINGLE.H0.E"),      // H1
    ("BOTS_SEG_93", "SINGLE.H1"),        // H2R
    ("BOTS_SEG_94", "SINGLE.H1.E"),      // H2
    ("BOTS_SEG_95", "SINGLE.H2"),        // H3R
    ("BOTS_SEG_96", "SINGLE.H2.E"),      // H3
    ("BOTS_SEG_97", "SINGLE.H3"),        // H4R
    ("BOTS_SEG_98", "SINGLE.H3.E"),      // H4
    ("BOTS_SEG_99", "SINGLE.H4"),        // H5R
    ("BOTS_SEG_100", "SINGLE.H4.E"),     // H5
    ("BOTS_SEG_101", "SINGLE.H5"),       // H6R
    ("BOTS_SEG_102", "SINGLE.H5.E"),     // H6
    ("BOTS_SEG_103", "SINGLE.H6"),       // H7R
    ("BOTS_SEG_104", "SINGLE.H6.E"),     // H7
    ("BOTS_SEG_105", "SINGLE.H7"),       // H8R
    ("BOTS_SEG_106", "SINGLE.H7.E"),     // H8
    ("BOTS_SEG_107", "DOUBLE.H1.0"),     // DH3R
    ("BOTS_SEG_108", "DOUBLE.H1.2"),     // DH3
    ("BOTS_SEG_109", "DOUBLE.V1.2"),     // DV3T
    ("BOTS_SEG_110", "SINGLE.V7.S"),     // V8T
    ("BOTS_SEG_111", "SINGLE.V6.S"),     // V7T
    ("BOTS_SEG_112", "SINGLE.V5.S"),     // V6T
    ("BOTS_SEG_113", "SINGLE.V4.S"),     // V5T
    ("BOTS_SEG_114", "SINGLE.V3.S"),     // V4T
    ("BOTS_SEG_115", "SINGLE.V2.S"),     // V3T
    ("BOTS_SEG_116", "SINGLE.V1.S"),     // V2T
    ("BOTS_SEG_117", "SINGLE.V0.S"),     // V1T
    ("BOTS_SEG_118", "DOUBLE.V0.2"),     // DV2T
    ("BOTS_SEG_119", "DOUBLE.H1.1"),     // DH4
    // BOTS_SEG_120 CLOCK_3_4
    // BOTS_SEG_121 COUT
    // BOTS_SEG_122 CINT

    // BOTSL
    ("I_BUFGP_BL_I_BOTSL", "OUT.IOB.CLKIN"),
    ("PAD48_I1", "OUT.BT.IOB0.I1"),
    ("PAD48_I2", "OUT.BT.IOB0.I2"),
    ("PAD48_IK", "IMUX.IOB0.IK"),
    ("PAD48_OK", "IMUX.IOB0.OK"),
    ("PAD48_T", "IMUX.IOB0.TS"),
    ("PAD47_I1", "OUT.BT.IOB1.I1"),
    ("PAD47_I2", "OUT.BT.IOB1.I2"),
    ("PAD47_IK", "IMUX.IOB1.IK"),
    ("PAD47_OK", "IMUX.IOB1.OK"),
    ("PAD47_T", "IMUX.IOB1.TS"),
    ("DEC_KB_2_I", "IMUX.CLB.C4"),
    ("TIE_KB_1_O", "GND"),
    ("BOTSL_SEG_1", "IO.DOUBLE.0.S.0"),  // BDH1
    ("BOTSL_SEG_3", "IO.DBUF.H0"),       // DMUX_OUTER
    ("BOTSL_SEG_4", "IO.DBUF.H1"),       // DMUX_INNER
    ("BOTSL_SEG_5", "SINGLE.V1"),        // V2
    ("BOTSL_SEG_6", "IO.DOUBLE.0.S.2"),  // BDH1L
    ("BOTSL_SEG_7", "DOUBLE.V0.1"),      // DV1
    ("BOTSL_SEG_8", "IO.DOUBLE.0.S.1"),  // BDH2
    ("BOTSL_SEG_9", "SINGLE.V0"),        // V1
    ("BOTSL_SEG_10", "IO.DOUBLE.1.S.0"), // BDH3
    ("BOTSL_SEG_11", "SINGLE.V3"),       // V4
    ("BOTSL_SEG_12", "DOUBLE.V0.0"),     // DV2
    ("BOTSL_SEG_13", "IO.DOUBLE.1.S.2"), // BDH3L
    ("BOTSL_SEG_14", "IO.DOUBLE.1.S.1"), // BDH4
    ("BOTSL_SEG_15", "SINGLE.V2"),       // V3
    ("BOTSL_SEG_16", "IO.DOUBLE.2.S.0"), // BDH5
    ("BOTSL_SEG_17", "DOUBLE.V1.0"),     // DV3
    ("BOTSL_SEG_18", "SINGLE.V5"),       // V6
    ("BOTSL_SEG_19", "IO.DOUBLE.2.S.2"), // BDH5L
    ("BOTSL_SEG_20", "IO.DOUBLE.2.S.1"), // BDH6
    ("BOTSL_SEG_21", "SINGLE.V4"),       // V5
    ("BOTSL_SEG_22", "IO.DOUBLE.3.S.0"), // BDH7
    ("BOTSL_SEG_23", "DOUBLE.V1.1"),     // DV4
    ("BOTSL_SEG_24", "SINGLE.V7"),       // V8
    ("BOTSL_SEG_25", "IO.DOUBLE.3.S.2"), // BDH7L
    ("BOTSL_SEG_26", "IO.DOUBLE.3.S.1"), // BDH8
    ("BOTSL_SEG_27", "SINGLE.V6"),       // V7
    ("BOTSL_SEG_28", "LONG.IO.H0"),      // BHLL1
    ("BOTSL_SEG_29", "LONG.V0"),         // VLL1
    ("BOTSL_SEG_30", "LONG.IO.H1"),      // BHLL2
    ("BOTSL_SEG_31", "LONG.V3"),         // VLL4
    ("BOTSL_SEG_32", "LONG.V1"),         // VLL2
    ("BOTSL_SEG_33", "LONG.IO.H2"),      // BHLL3
    ("BOTSL_SEG_34", "LONG.V4"),         // VLL5
    ("BOTSL_SEG_35", "LONG.V2"),         // VLL3
    ("BOTSL_SEG_36", "LONG.IO.H3"),      // BHLL4
    ("BOTSL_SEG_37", "LONG.V5"),         // VLL6
    ("BOTSL_SEG_39", "DEC.H0"),          // TX1
    ("BOTSL_SEG_46", "DEC.H1"),          // TX2
    ("BOTSL_SEG_50", "DEC.H2"),          // TX3
    ("BOTSL_SEG_54", "DEC.H3"),          // TX4
    // BOTSL_SEG_65 OK_2L
    ("BOTSL_SEG_67", "GCLK0"),            // K1
    ("BOTSL_SEG_69", "GCLK1"),            // K2
    ("BOTSL_SEG_70", "GCLK2"),            // K3
    ("BOTSL_SEG_71", "GCLK3"),            // K4
    ("BOTSL_SEG_72", "OUT.BT.IOB1.I1.E"), // I1_2L
    // BOTSL_SEG_76 CE_2L
    ("BOTSL_SEG_77", "OUT.BT.IOB1.I2.E"), // I2_2L
    ("BOTSL_SEG_78", "LONG.H3"),          // HLL4
    ("BOTSL_SEG_79", "IMUX.CLB.C2"),      // C2T
    ("BOTSL_SEG_80", "IMUX.CLB.G2"),      // G2T
    ("BOTSL_SEG_81", "LONG.H4"),          // HLL5
    ("BOTSL_SEG_82", "IMUX.CLB.F2"),      // F2T
    ("BOTSL_SEG_83", "LONG.H5"),          // HLL6
    ("BOTSL_SEG_84", "DOUBLE.H0.1"),      // DH1
    ("BOTSL_SEG_85", "OUT.CLB.FX.S"),     // FXT
    ("BOTSL_SEG_86", "DOUBLE.H0.0"),      // DH2R
    ("BOTSL_SEG_87", "OUT.CLB.FXQ.S"),    // FXQT
    ("BOTSL_SEG_88", "DOUBLE.H0.2"),      // DH2
    ("BOTSL_SEG_89", "SINGLE.H0"),        // H1R
    ("BOTSL_SEG_90", "SINGLE.H0.E"),      // H1
    ("BOTSL_SEG_91", "SINGLE.H1"),        // H2R
    ("BOTSL_SEG_92", "SINGLE.H1.E"),      // H2
    ("BOTSL_SEG_93", "SINGLE.H2"),        // H3R
    ("BOTSL_SEG_94", "SINGLE.H2.E"),      // H3
    ("BOTSL_SEG_95", "SINGLE.H3"),        // H4R
    ("BOTSL_SEG_96", "SINGLE.H3.E"),      // H4
    ("BOTSL_SEG_97", "SINGLE.H4"),        // H5R
    ("BOTSL_SEG_98", "SINGLE.H4.E"),      // H5
    ("BOTSL_SEG_99", "SINGLE.H5"),        // H6R
    ("BOTSL_SEG_100", "SINGLE.H5.E"),     // H6
    ("BOTSL_SEG_101", "SINGLE.H6"),       // H7R
    ("BOTSL_SEG_102", "SINGLE.H6.E"),     // H7
    ("BOTSL_SEG_103", "SINGLE.H7"),       // H8R
    ("BOTSL_SEG_104", "SINGLE.H7.E"),     // H8
    ("BOTSL_SEG_105", "DOUBLE.H1.0"),     // DH3R
    ("BOTSL_SEG_106", "DOUBLE.H1.2"),     // DH3
    ("BOTSL_SEG_107", "DOUBLE.V1.2"),     // DV3T
    ("BOTSL_SEG_108", "SINGLE.V7.S"),     // V8T
    ("BOTSL_SEG_109", "SINGLE.V6.S"),     // V7T
    ("BOTSL_SEG_110", "SINGLE.V5.S"),     // V6T
    ("BOTSL_SEG_111", "SINGLE.V4.S"),     // V5T
    ("BOTSL_SEG_112", "SINGLE.V3.S"),     // V4T
    ("BOTSL_SEG_113", "SINGLE.V2.S"),     // V3T
    ("BOTSL_SEG_114", "SINGLE.V1.S"),     // V2T
    ("BOTSL_SEG_115", "SINGLE.V0.S"),     // V1T
    ("BOTSL_SEG_116", "DOUBLE.V0.2"),     // DV2T
    ("BOTSL_SEG_117", "DOUBLE.H1.1"),     // DH4
    // BOTSL_SEG_118 CLOCK_3_4
    // BOTSL_SEG_119 CIN
    // BOTSL_SEG_120 CINT
    // TOP
    ("PAD3_I1", "OUT.BT.IOB0.I1"),
    ("PAD3_I2", "OUT.BT.IOB0.I2"),
    ("PAD3_IK", "IMUX.IOB0.IK"),
    ("PAD3_OK", "IMUX.IOB0.OK"),
    ("PAD3_T", "IMUX.IOB0.TS"),
    ("PAD4_I1", "OUT.BT.IOB1.I1"),
    ("PAD4_I2", "OUT.BT.IOB1.I2"),
    ("PAD4_IK", "IMUX.IOB1.IK"),
    ("PAD4_OK", "IMUX.IOB1.OK"),
    ("PAD4_T", "IMUX.IOB1.TS"),
    ("DEC_AC_2_I", "IMUX.CLB.C2.N"),
    // TOP_SEG_0 G4B
    ("TOP_SEG_1", "LONG.H0"), // HLL1
    // TOP_SEG_3 F4B
    ("TOP_SEG_5", "SINGLE.V1"), // V2
    ("TOP_SEG_6", "LONG.H1"),   // HLL2
    // TOP_SEG_7 C4B
    ("TOP_SEG_9", "SINGLE.V2"),         // V3
    ("TOP_SEG_10", "LONG.H2"),          // HLL3
    ("TOP_SEG_11", "SINGLE.V3"),        // V4
    ("TOP_SEG_12", "DOUBLE.V1.1"),      // DV4
    ("TOP_SEG_13", "OUT.BT.IOB1.I2.E"), // I2_2L
    ("TOP_SEG_14", "SINGLE.V4"),        // V5
    ("TOP_SEG_15", "SINGLE.V0"),        // V1
    ("TOP_SEG_16", "LONG.V2"),          // VLL3
    ("TOP_SEG_17", "LONG.V1"),          // VLL2
    ("TOP_SEG_18", "LONG.V0"),          // VLL1
    ("TOP_SEG_19", "LONG.V5"),          // VLL6
    ("TOP_SEG_21", "LONG.V4"),          // VLL5
    ("TOP_SEG_22", "LONG.V3"),          // VLL4
    ("TOP_SEG_23", "DOUBLE.V1.0"),      // DV3
    ("TOP_SEG_24", "SINGLE.V5"),        // V6
    ("TOP_SEG_26", "DOUBLE.V0.1"),      // DV1
    ("TOP_SEG_27", "DOUBLE.V0.0"),      // DV2
    // TOP_SEG_31 CE_2L
    ("TOP_SEG_40", "GCLK3"),            // K4
    ("TOP_SEG_41", "SINGLE.V7"),        // V8
    ("TOP_SEG_42", "OUT.BT.IOB1.I1.E"), // I1_2L
    ("TOP_SEG_43", "GCLK2"),            // K3
    ("TOP_SEG_44", "GCLK1"),            // K2
    ("TOP_SEG_45", "GCLK0"),            // K1
    ("TOP_SEG_46", "SINGLE.V6"),        // V7
    // TOP_SEG_48 IK_2L
    // TOP_SEG_49 OK_2L
    ("TOP_SEG_63", "DEC.H3"),          // TTX1
    ("TOP_SEG_64", "DEC.H2"),          // TTX2
    ("TOP_SEG_65", "DEC.H1"),          // TTX3
    ("TOP_SEG_66", "DEC.H0"),          // TTX4
    ("TOP_SEG_67", "LONG.IO.H0"),      // THLL1
    ("TOP_SEG_68", "LONG.IO.H1"),      // THLL2
    ("TOP_SEG_69", "LONG.IO.H2"),      // THLL3
    ("TOP_SEG_70", "LONG.IO.H3"),      // THLL4
    ("TOP_SEG_71", "IO.DOUBLE.0.N.2"), // DH1
    ("TOP_SEG_72", "IO.DBUF.H0"),      // DMUX_OUTER
    ("TOP_SEG_73", "IO.DBUF.H1"),      // DMUX_INNER
    ("TOP_SEG_74", "IO.DOUBLE.0.N.0"), // DH1L
    ("TOP_SEG_75", "IO.DOUBLE.0.N.1"), // DH2
    ("TOP_SEG_76", "IO.DOUBLE.1.N.2"), // DH3
    ("TOP_SEG_77", "IO.DOUBLE.1.N.0"), // DH3L
    ("TOP_SEG_78", "IO.DOUBLE.1.N.1"), // DH4
    ("TOP_SEG_79", "IO.DOUBLE.2.N.2"), // DH5
    ("TOP_SEG_80", "IO.DOUBLE.2.N.0"), // DH5L
    ("TOP_SEG_81", "IO.DOUBLE.2.N.1"), // DH6
    ("TOP_SEG_82", "IO.DOUBLE.3.N.2"), // DH7
    ("TOP_SEG_83", "IO.DOUBLE.3.N.0"), // DH7L
    ("TOP_SEG_84", "IO.DOUBLE.3.N.1"), // DH8
    // TOP_SEG_85 CINB
    // TOP_SEG_86 CIN
    // TOP_SEG_87 CLOCK_7_8
    // TOPRR
    ("I_BUFGP_TR_I_TOPRR", "OUT.IOB.CLKIN"),
    ("PAD11_I1", "OUT.BT.IOB0.I1"),
    ("PAD11_I2", "OUT.BT.IOB0.I2"),
    ("PAD11_IK", "IMUX.IOB0.IK"),
    ("PAD11_OK", "IMUX.IOB0.OK"),
    ("PAD11_T", "IMUX.IOB0.TS"),
    ("PAD12_I1", "OUT.BT.IOB1.I1"),
    ("PAD12_I2", "OUT.BT.IOB1.I2"),
    ("PAD12_IK", "IMUX.IOB1.IK"),
    ("PAD12_OK", "IMUX.IOB1.OK"),
    ("PAD12_T", "IMUX.IOB1.TS"),
    ("DEC_AH_2_I", "IMUX.CLB.C2.N"),
    // TOPR_SEG_0 G4B
    ("TOPR_SEG_1", "LONG.H0"), // HLL1
    // TOPR_SEG_3 F4B
    ("TOPR_SEG_5", "SINGLE.V1"), // V2
    ("TOPR_SEG_6", "LONG.H1"),   // HLL2
    // TOPR_SEG_7 C4B
    ("TOPR_SEG_9", "SINGLE.V2"),         // V3
    ("TOPR_SEG_10", "LONG.H2"),          // HLL3
    ("TOPR_SEG_11", "SINGLE.V3"),        // V4
    ("TOPR_SEG_12", "DOUBLE.V1.1"),      // DV4
    ("TOPR_SEG_13", "OUT.BT.IOB1.I2.E"), // I2_2L
    ("TOPR_SEG_14", "SINGLE.V4"),        // V5
    ("TOPR_SEG_15", "SINGLE.V0"),        // V1
    ("TOPR_SEG_16", "LONG.V2"),          // VLL3
    ("TOPR_SEG_17", "LONG.V1"),          // VLL2
    ("TOPR_SEG_18", "LONG.V0"),          // VLL1
    ("TOPR_SEG_19", "LONG.V5"),          // VLL6
    ("TOPR_SEG_21", "LONG.V4"),          // VLL5
    ("TOPR_SEG_22", "LONG.V3"),          // VLL4
    ("TOPR_SEG_23", "DOUBLE.V1.0"),      // DV3
    ("TOPR_SEG_24", "SINGLE.V5"),        // V6
    ("TOPR_SEG_26", "DOUBLE.V0.1"),      // DV1
    ("TOPR_SEG_27", "DOUBLE.V0.0"),      // DV2
    // TOPR_SEG_31 CE_2L
    ("TOPR_SEG_40", "GCLK3"),            // K4
    ("TOPR_SEG_41", "SINGLE.V7"),        // V8
    ("TOPR_SEG_42", "OUT.BT.IOB1.I1.E"), // I1_2L
    ("TOPR_SEG_43", "GCLK2"),            // K3
    ("TOPR_SEG_44", "GCLK1"),            // K2
    ("TOPR_SEG_45", "GCLK0"),            // K1
    ("TOPR_SEG_46", "SINGLE.V6"),        // V7
    // TOPR_SEG_48 IK_2L
    // TOPR_SEG_49 OK_2L
    ("TOPR_SEG_63", "DEC.H3"),          // TTX1
    ("TOPR_SEG_64", "DEC.H2"),          // TTX2
    ("TOPR_SEG_65", "DEC.H1"),          // TTX3
    ("TOPR_SEG_66", "DEC.H0"),          // TTX4
    ("TOPR_SEG_67", "LONG.IO.H0"),      // THLL1
    ("TOPR_SEG_68", "LONG.IO.H1"),      // THLL2
    ("TOPR_SEG_69", "LONG.IO.H2"),      // THLL3
    ("TOPR_SEG_70", "LONG.IO.H3"),      // THLL4
    ("TOPR_SEG_71", "IO.DOUBLE.0.N.2"), // DH1
    ("TOPR_SEG_72", "IO.DBUF.H0"),      // DMUX_OUTER
    ("TOPR_SEG_73", "IO.DBUF.H1"),      // DMUX_INNER
    ("TOPR_SEG_74", "IO.DOUBLE.0.N.0"), // DH1L
    ("TOPR_SEG_75", "IO.DOUBLE.0.N.1"), // DH2
    ("TOPR_SEG_76", "IO.DOUBLE.1.N.2"), // DH3
    ("TOPR_SEG_77", "IO.DOUBLE.1.N.0"), // DH3L
    ("TOPR_SEG_78", "IO.DOUBLE.1.N.1"), // DH4
    ("TOPR_SEG_79", "IO.DOUBLE.2.N.2"), // DH5
    ("TOPR_SEG_80", "IO.DOUBLE.2.N.0"), // DH5L
    ("TOPR_SEG_81", "IO.DOUBLE.2.N.1"), // DH6
    ("TOPR_SEG_82", "IO.DOUBLE.3.N.2"), // DH7
    ("TOPR_SEG_83", "IO.DOUBLE.3.N.0"), // DH7L
    ("TOPR_SEG_84", "IO.DOUBLE.3.N.1"), // DH8
    // TOPR_SEG_85 CINB
    // TOPR_SEG_86 CIN
    // TOPR_SEG_87 CLOCK_7_8
    // TOPS
    ("PAD5_I1", "OUT.BT.IOB0.I1"),
    ("PAD5_I2", "OUT.BT.IOB0.I2"),
    ("PAD5_IK", "IMUX.IOB0.IK"),
    ("PAD5_OK", "IMUX.IOB0.OK"),
    ("PAD5_T", "IMUX.IOB0.TS"),
    ("PAD6_I1", "OUT.BT.IOB1.I1"),
    ("PAD6_I2", "OUT.BT.IOB1.I2"),
    ("PAD6_IK", "IMUX.IOB1.IK"),
    ("PAD6_OK", "IMUX.IOB1.OK"),
    ("PAD6_T", "IMUX.IOB1.TS"),
    ("DEC_AD_2_I", "IMUX.CLB.C2.N"),
    // TOPS_SEG_0 G4B
    ("TOPS_SEG_1", "LONG.H0"), // HLL1
    // TOPS_SEG_3 F4B
    ("TOPS_SEG_5", "SINGLE.V1"), // V2
    ("TOPS_SEG_6", "LONG.H1"),   // HLL2
    // TOPS_SEG_7 C4B
    ("TOPS_SEG_9", "SINGLE.V2"),         // V3
    ("TOPS_SEG_10", "LONG.H2"),          // HLL3
    ("TOPS_SEG_11", "SINGLE.V3"),        // V4
    ("TOPS_SEG_12", "DOUBLE.V1.1"),      // DV4
    ("TOPS_SEG_13", "OUT.BT.IOB1.I2.E"), // I2_2L
    ("TOPS_SEG_14", "SINGLE.V4"),        // V5
    ("TOPS_SEG_15", "SINGLE.V0"),        // V1
    ("TOPS_SEG_16", "LONG.V2"),          // VLL3
    ("TOPS_SEG_17", "LONG.V1"),          // VLL2
    ("TOPS_SEG_18", "LONG.V0"),          // VLL1
    ("TOPS_SEG_19", "LONG.V5"),          // VLL6
    ("TOPS_SEG_21", "LONG.V4"),          // VLL5
    ("TOPS_SEG_22", "LONG.V3"),          // VLL4
    ("TOPS_SEG_23", "DOUBLE.V1.0"),      // DV3
    ("TOPS_SEG_24", "SINGLE.V5"),        // V6
    ("TOPS_SEG_26", "DOUBLE.V0.1"),      // DV1
    ("TOPS_SEG_27", "DOUBLE.V0.0"),      // DV2
    // TOPS_SEG_31 CE_2L
    ("TOPS_SEG_40", "GCLK3"),            // K4
    ("TOPS_SEG_41", "SINGLE.V7"),        // V8
    ("TOPS_SEG_42", "OUT.BT.IOB1.I1.E"), // I1_2L
    ("TOPS_SEG_43", "GCLK2"),            // K3
    ("TOPS_SEG_44", "GCLK1"),            // K2
    ("TOPS_SEG_45", "GCLK0"),            // K1
    ("TOPS_SEG_46", "SINGLE.V6"),        // V7
    // TOPS_SEG_48 IK_2L
    // TOPS_SEG_49 OK_2L
    ("TOPS_SEG_63", "DEC.H3"),          // TTX1
    ("TOPS_SEG_64", "DEC.H2"),          // TTX2
    ("TOPS_SEG_65", "DEC.H1"),          // TTX3
    ("TOPS_SEG_66", "DEC.H0"),          // TTX4
    ("TOPS_SEG_67", "LONG.IO.H0"),      // THLL1
    ("TOPS_SEG_68", "LONG.IO.H1"),      // THLL2
    ("TOPS_SEG_69", "LONG.IO.H2"),      // THLL3
    ("TOPS_SEG_70", "LONG.IO.H3"),      // THLL4
    ("TOPS_SEG_71", "IO.DOUBLE.0.N.2"), // DH1
    ("TOPS_SEG_72", "IO.DBUF.H0"),      // DMUX_OUTER
    ("TOPS_SEG_73", "IO.DBUF.H1"),      // DMUX_INNER
    ("TOPS_SEG_74", "IO.DOUBLE.0.N.0"), // DH1L
    ("TOPS_SEG_75", "IO.DOUBLE.0.N.1"), // DH2
    ("TOPS_SEG_76", "IO.DOUBLE.1.N.2"), // DH3
    ("TOPS_SEG_77", "IO.DOUBLE.1.N.0"), // DH3L
    ("TOPS_SEG_78", "IO.DOUBLE.1.N.1"), // DH4
    ("TOPS_SEG_79", "IO.DOUBLE.2.N.2"), // DH5
    ("TOPS_SEG_80", "IO.DOUBLE.2.N.0"), // DH5L
    ("TOPS_SEG_81", "IO.DOUBLE.2.N.1"), // DH6
    ("TOPS_SEG_82", "IO.DOUBLE.3.N.2"), // DH7
    ("TOPS_SEG_83", "IO.DOUBLE.3.N.0"), // DH7L
    ("TOPS_SEG_84", "IO.DOUBLE.3.N.1"), // DH8
    // TOPS_SEG_85 CINB
    // TOPS_SEG_86 CIN
    // TOPS_SEG_87 CLOCK_7_8
    // TOPSL
    ("I_BUFGS_TL_I_TOPSL", "OUT.IOB.CLKIN"),
    ("PAD1_I1", "OUT.BT.IOB0.I1"),
    ("PAD1_I2", "OUT.BT.IOB0.I2"),
    ("PAD1_IK", "IMUX.IOB0.IK"),
    ("PAD1_OK", "IMUX.IOB0.OK"),
    ("PAD1_T", "IMUX.IOB0.TS"),
    ("PAD2_I1", "OUT.BT.IOB1.I1"),
    ("PAD2_I2", "OUT.BT.IOB1.I2"),
    ("PAD2_IK", "IMUX.IOB1.IK"),
    ("PAD2_OK", "IMUX.IOB1.OK"),
    ("PAD2_T", "IMUX.IOB1.TS"),
    ("DEC_AB_2_I", "IMUX.CLB.C2.N"),
    // TOPSL_SEG_0 G4B
    ("TOPSL_SEG_1", "LONG.H0"), // HLL1
    // TOPSL_SEG_3 F4B
    ("TOPSL_SEG_5", "SINGLE.V1"), // V2
    ("TOPSL_SEG_6", "LONG.H1"),   // HLL2
    // TOPSL_SEG_7 C4B
    ("TOPSL_SEG_9", "SINGLE.V2"),         // V3
    ("TOPSL_SEG_10", "LONG.H2"),          // HLL3
    ("TOPSL_SEG_11", "SINGLE.V3"),        // V4
    ("TOPSL_SEG_12", "DOUBLE.V1.1"),      // DV4
    ("TOPSL_SEG_13", "OUT.BT.IOB1.I2.E"), // I2_2L
    ("TOPSL_SEG_14", "SINGLE.V4"),        // V5
    ("TOPSL_SEG_15", "SINGLE.V0"),        // V1
    ("TOPSL_SEG_16", "LONG.V2"),          // VLL3
    ("TOPSL_SEG_17", "LONG.V1"),          // VLL2
    ("TOPSL_SEG_18", "LONG.V0"),          // VLL1
    ("TOPSL_SEG_19", "LONG.V5"),          // VLL6
    ("TOPSL_SEG_21", "LONG.V4"),          // VLL5
    ("TOPSL_SEG_22", "LONG.V3"),          // VLL4
    ("TOPSL_SEG_23", "DOUBLE.V1.0"),      // DV3
    ("TOPSL_SEG_24", "SINGLE.V5"),        // V6
    ("TOPSL_SEG_25", "DOUBLE.V0.0"),      // DV2
    // TOPSL_SEG_29 CE_2L
    ("TOPSL_SEG_30", "DOUBLE.V0.1"),      // DV1
    ("TOPSL_SEG_39", "GCLK3"),            // K4
    ("TOPSL_SEG_40", "SINGLE.V7"),        // V8
    ("TOPSL_SEG_41", "OUT.BT.IOB1.I1.E"), // I1_2L
    ("TOPSL_SEG_42", "GCLK2"),            // K3
    ("TOPSL_SEG_43", "GCLK1"),            // K2
    ("TOPSL_SEG_44", "GCLK0"),            // K1
    ("TOPSL_SEG_45", "SINGLE.V6"),        // V7
    // TOPSL_SEG_47 IK_2L
    ("TOPSL_SEG_61", "DEC.H3"),          // TTX1
    ("TOPSL_SEG_62", "DEC.H2"),          // TTX2
    ("TOPSL_SEG_63", "DEC.H1"),          // TTX3
    ("TOPSL_SEG_64", "DEC.H0"),          // TTX4
    ("TOPSL_SEG_65", "LONG.IO.H0"),      // THLL1
    ("TOPSL_SEG_66", "LONG.IO.H1"),      // THLL2
    ("TOPSL_SEG_67", "LONG.IO.H2"),      // THLL3
    ("TOPSL_SEG_68", "LONG.IO.H3"),      // THLL4
    ("TOPSL_SEG_69", "IO.DOUBLE.0.N.2"), // DH1
    ("TOPSL_SEG_70", "IO.DBUF.H0"),      // DMUX_OUTER
    ("TOPSL_SEG_71", "IO.DBUF.H1"),      // DMUX_INNER
    ("TOPSL_SEG_72", "IO.DOUBLE.0.N.0"), // DH1L
    ("TOPSL_SEG_73", "IO.DOUBLE.0.N.1"), // DH2
    ("TOPSL_SEG_74", "IO.DOUBLE.1.N.2"), // DH3
    ("TOPSL_SEG_75", "IO.DOUBLE.1.N.0"), // DH3L
    ("TOPSL_SEG_76", "IO.DOUBLE.1.N.1"), // DH4
    ("TOPSL_SEG_77", "IO.DOUBLE.2.N.2"), // DH5
    ("TOPSL_SEG_78", "IO.DOUBLE.2.N.0"), // DH5L
    ("TOPSL_SEG_79", "IO.DOUBLE.2.N.1"), // DH6
    ("TOPSL_SEG_80", "IO.DOUBLE.3.N.2"), // DH7
    ("TOPSL_SEG_81", "IO.DOUBLE.3.N.0"), // DH7L
    ("TOPSL_SEG_82", "IO.DOUBLE.3.N.1"), // DH8
    // TOPSL_SEG_83 CINB
    // TOPSL_SEG_84 CIN
    // TOPSL_SEG_85 CLOCK_7_8

    // RT
    ("PAD21_I1", "OUT.LR.IOB0.I1"),
    ("PAD21_I2", "OUT.LR.IOB0.I2"),
    ("PAD21_IK", "IMUX.IOB0.IK"),
    ("PAD21_OK", "IMUX.IOB0.OK"),
    ("PAD21_T", "IMUX.IOB0.TS"),
    ("PAD22_I1", "OUT.LR.IOB1.I1"),
    ("PAD22_I2", "OUT.LR.IOB1.I2"),
    ("PAD22_IK", "IMUX.IOB1.IK"),
    ("PAD22_OK", "IMUX.IOB1.OK"),
    ("PAD22_T", "IMUX.IOB1.TS"),
    ("DEC_DK_2_I", "IMUX.CLB.C1"),
    ("TBUF_DK_2_I", "IMUX.TBUF0.I"),
    ("TBUF_DK_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_DK_1_I", "IMUX.TBUF1.I"),
    ("TBUF_DK_1_T", "IMUX.TBUF1.TS"),
    ("TIE_DK_1_O", "GND"),
    ("RT_SEG_0", "LONG.IO.V0"),        // RVLL1
    ("RT_SEG_1", "LONG.H0"),           // HLL1
    ("RT_SEG_2", "DEC.V3"),            // RTX1
    ("RT_SEG_6", "SINGLE.V1"),         // V2
    ("RT_SEG_7", "LONG.IO.V1"),        // RVLL2
    ("RT_SEG_8", "LONG.H1"),           // HLL2
    ("RT_SEG_9", "DEC.V2"),            // RTX2
    ("RT_SEG_10", "SINGLE.V2"),        // V3
    ("RT_SEG_12", "LONG.H2"),          // HLL3
    ("RT_SEG_13", "IO.DOUBLE.3.E.1"),  // RDV8
    ("RT_SEG_14", "IO.DOUBLE.3.E.2"),  // RDV7
    ("RT_SEG_15", "IO.DOUBLE.2.E.1"),  // RDV6
    ("RT_SEG_16", "IO.DOUBLE.2.E.2"),  // RDV5
    ("RT_SEG_17", "LONG.IO.V2"),       // RVLL3
    ("RT_SEG_18", "DEC.V1"),           // RTX3
    ("RT_SEG_21", "SINGLE.V3"),        // V4
    ("RT_SEG_22", "DEC.V0"),           // RTX4
    ("RT_SEG_23", "IO.DOUBLE.1.E.1"),  // RDV4
    ("RT_SEG_25", "IO.DOUBLE.1.E.2"),  // RDV3
    ("RT_SEG_26", "IO.DOUBLE.0.E.1"),  // RDV2
    ("RT_SEG_27", "IO.DOUBLE.0.E.2"),  // RDV1
    ("RT_SEG_28", "LONG.IO.V3"),       // RVLL4
    ("RT_SEG_35", "DOUBLE.V1.0"),      // DV3
    ("RT_SEG_36", "SINGLE.V5"),        // V6
    ("RT_SEG_38", "GCLK3"),            // K4
    ("RT_SEG_40", "GCLK2"),            // K3
    ("RT_SEG_41", "GCLK1"),            // K2
    ("RT_SEG_42", "GCLK0"),            // K1
    ("RT_SEG_46", "SINGLE.V7"),        // V8
    ("RT_SEG_47", "SINGLE.V0"),        // V1
    ("RT_SEG_49", "LONG.V4"),          // VLL5
    ("RT_SEG_50", "LONG.V3"),          // VLL4
    ("RT_SEG_51", "DOUBLE.V1.1"),      // DV4
    ("RT_SEG_52", "SINGLE.V6"),        // V7
    ("RT_SEG_53", "SINGLE.V4"),        // V5
    ("RT_SEG_54", "DOUBLE.V0.0"),      // DV2
    ("RT_SEG_55", "DOUBLE.V0.1"),      // DV1
    ("RT_SEG_56", "LONG.V1"),          // VLL2
    ("RT_SEG_57", "LONG.V0"),          // VLL1
    ("RT_SEG_59", "LONG.V5"),          // VLL6
    ("RT_SEG_60", "IMUX.CLB.F3"),      // F3L
    ("RT_SEG_61", "LONG.V2"),          // VLL3
    ("RT_SEG_66", "IMUX.CLB.C3"),      // C3L
    ("RT_SEG_69", "IMUX.CLB.G3"),      // G3L
    ("RT_SEG_73", "OUT.CLB.GY.E"),     // GYL
    ("RT_SEG_79", "OUT.CLB.GYQ.E"),    // GYQL
    ("RT_SEG_84", "LONG.H3"),          // HLL4
    ("RT_SEG_85", "IO.DBUF.V0"),       // DMUX_OUTER
    ("RT_SEG_86", "LONG.H4"),          // HLL5
    ("RT_SEG_87", "IO.DBUF.V1"),       // DMUX_INNER
    ("RT_SEG_88", "LONG.H5"),          // HLL6
    ("RT_SEG_89", "DOUBLE.H0.1"),      // DH1
    ("RT_SEG_90", "IO.DOUBLE.0.E.0"),  // RDV1T
    ("RT_SEG_91", "OUT.LR.IOB1.I1.S"), // I1_2T
    ("RT_SEG_92", "IO.DOUBLE.3.E.0"),  // RDV7T
    ("RT_SEG_93", "IO.DOUBLE.2.E.0"),  // RDV5T
    ("RT_SEG_94", "IO.DOUBLE.1.E.0"),  // RDV3T
    ("RT_SEG_95", "DOUBLE.H0.0"),      // DH2R
    // RT_SEG_96 CE_2T
    ("RT_SEG_98", "DOUBLE.H0.2"),       // DH2
    ("RT_SEG_99", "SINGLE.H0"),         // H1R
    ("RT_SEG_100", "OUT.LR.IOB1.I2.S"), // I2_2T
    ("RT_SEG_101", "SINGLE.H0.E"),      // H1
    ("RT_SEG_102", "SINGLE.H1"),        // H2R
    ("RT_SEG_103", "SINGLE.H1.E"),      // H2
    ("RT_SEG_104", "SINGLE.H2"),        // H3R
    // RT_SEG_105 OK_2T
    // RT_SEG_106 IK_2T
    ("RT_SEG_107", "SINGLE.H2.E"), // H3
    ("RT_SEG_108", "SINGLE.H3"),   // H4R
    ("RT_SEG_109", "SINGLE.H3.E"), // H4
    ("RT_SEG_110", "SINGLE.H4"),   // H5R
    ("RT_SEG_111", "SINGLE.H4.E"), // H5
    ("RT_SEG_112", "SINGLE.H5"),   // H6R
    ("RT_SEG_113", "SINGLE.H5.E"), // H6
    ("RT_SEG_114", "SINGLE.H6"),   // H7R
    ("RT_SEG_115", "SINGLE.H6.E"), // H7
    ("RT_SEG_116", "SINGLE.H7"),   // H8R
    ("RT_SEG_117", "SINGLE.H7.E"), // H8
    ("RT_SEG_118", "DOUBLE.H1.0"), // DH3R
    ("RT_SEG_119", "DOUBLE.H1.2"), // DH3
    ("RT_SEG_120", "DOUBLE.H1.1"), // DH4
    ("RT_SEG_121", "DOUBLE.V1.2"), // DV3T
    ("RT_SEG_122", "SINGLE.V7.S"), // V8T
    ("RT_SEG_123", "SINGLE.V6.S"), // V7T
    ("RT_SEG_124", "SINGLE.V5.S"), // V6T
    ("RT_SEG_125", "SINGLE.V4.S"), // V5T
    ("RT_SEG_126", "SINGLE.V3.S"), // V4T
    ("RT_SEG_127", "SINGLE.V2.S"), // V3T
    ("RT_SEG_128", "SINGLE.V1.S"), // V2T
    ("RT_SEG_129", "SINGLE.V0.S"), // V1T
    ("RT_SEG_130", "DOUBLE.V0.2"), // DV2T
    // RT_SEG_131: CLOCK_5_6

    // RTT
    ("I_BUFGS_TR_I_RTT", "OUT.IOB.CLKIN"),
    ("PAD17_I1", "OUT.LR.IOB0.I1"),
    ("PAD17_I2", "OUT.LR.IOB0.I2"),
    ("PAD17_IK", "IMUX.IOB0.IK"),
    ("PAD17_OK", "IMUX.IOB0.OK"),
    ("PAD17_T", "IMUX.IOB0.TS"),
    ("PAD18_I1", "OUT.LR.IOB1.I1"),
    ("PAD18_I2", "OUT.LR.IOB1.I2"),
    ("PAD18_IK", "IMUX.IOB1.IK"),
    ("PAD18_OK", "IMUX.IOB1.OK"),
    ("PAD18_T", "IMUX.IOB1.TS"),
    ("DEC_BK_2_I", "IMUX.CLB.C1"),
    ("TBUF_BK_2_I", "IMUX.TBUF0.I"),
    ("TBUF_BK_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_BK_1_I", "IMUX.TBUF1.I"),
    ("TBUF_BK_1_T", "IMUX.TBUF1.TS"),
    ("TIE_BK_1_O", "GND"),
    ("RTT_SEG_0", "LONG.IO.V0"),        // RVLL1
    ("RTT_SEG_1", "LONG.H0"),           // HLL1
    ("RTT_SEG_2", "DEC.V3"),            // RTX1
    ("RTT_SEG_6", "SINGLE.V1"),         // V2
    ("RTT_SEG_7", "LONG.IO.V1"),        // RVLL2
    ("RTT_SEG_8", "LONG.H1"),           // HLL2
    ("RTT_SEG_9", "DEC.V2"),            // RTX2
    ("RTT_SEG_10", "SINGLE.V2"),        // V3
    ("RTT_SEG_12", "LONG.H2"),          // HLL3
    ("RTT_SEG_13", "IO.DOUBLE.3.E.1"),  // RDV8
    ("RTT_SEG_14", "IO.DOUBLE.3.E.2"),  // RDV7
    ("RTT_SEG_15", "IO.DOUBLE.2.E.1"),  // RDV6
    ("RTT_SEG_16", "IO.DOUBLE.2.E.2"),  // RDV5
    ("RTT_SEG_17", "LONG.IO.V2"),       // RVLL3
    ("RTT_SEG_18", "DEC.V1"),           // RTX3
    ("RTT_SEG_21", "SINGLE.V3"),        // V4
    ("RTT_SEG_22", "DEC.V0"),           // RTX4
    ("RTT_SEG_23", "IO.DOUBLE.1.E.1"),  // RDV4
    ("RTT_SEG_25", "IO.DOUBLE.1.E.2"),  // RDV3
    ("RTT_SEG_26", "IO.DOUBLE.0.E.1"),  // RDV2
    ("RTT_SEG_27", "IO.DOUBLE.0.E.2"),  // RDV1
    ("RTT_SEG_28", "LONG.IO.V3"),       // RVLL4
    ("RTT_SEG_35", "DOUBLE.V1.0"),      // DV3
    ("RTT_SEG_36", "SINGLE.V5"),        // V6
    ("RTT_SEG_38", "GCLK3"),            // K4
    ("RTT_SEG_40", "GCLK2"),            // K3
    ("RTT_SEG_41", "GCLK1"),            // K2
    ("RTT_SEG_42", "GCLK0"),            // K1
    ("RTT_SEG_46", "SINGLE.V7"),        // V8
    ("RTT_SEG_47", "SINGLE.V0"),        // V1
    ("RTT_SEG_49", "LONG.V4"),          // VLL5
    ("RTT_SEG_50", "LONG.V3"),          // VLL4
    ("RTT_SEG_51", "DOUBLE.V1.1"),      // DV4
    ("RTT_SEG_52", "SINGLE.V6"),        // V7
    ("RTT_SEG_53", "SINGLE.V4"),        // V5
    ("RTT_SEG_54", "DOUBLE.V0.0"),      // DV2
    ("RTT_SEG_55", "DOUBLE.V0.1"),      // DV1
    ("RTT_SEG_56", "LONG.V1"),          // VLL2
    ("RTT_SEG_57", "LONG.V0"),          // VLL1
    ("RTT_SEG_59", "LONG.V5"),          // VLL6
    ("RTT_SEG_60", "IMUX.CLB.F3"),      // F3L
    ("RTT_SEG_61", "LONG.V2"),          // VLL3
    ("RTT_SEG_66", "IMUX.CLB.C3"),      // C3L
    ("RTT_SEG_69", "IMUX.CLB.G3"),      // G3L
    ("RTT_SEG_73", "OUT.CLB.GY.E"),     // GYL
    ("RTT_SEG_79", "OUT.CLB.GYQ.E"),    // GYQL
    ("RTT_SEG_84", "LONG.H3"),          // HLL4
    ("RTT_SEG_85", "IO.DBUF.V0"),       // DMUX_OUTER
    ("RTT_SEG_86", "LONG.H4"),          // HLL5
    ("RTT_SEG_87", "IO.DBUF.V1"),       // DMUX_INNER
    ("RTT_SEG_88", "LONG.H5"),          // HLL6
    ("RTT_SEG_89", "DOUBLE.H0.1"),      // DH1
    ("RTT_SEG_90", "IO.DOUBLE.0.E.0"),  // RDV1T
    ("RTT_SEG_91", "OUT.LR.IOB1.I1.S"), // I1_2T
    ("RTT_SEG_92", "IO.DOUBLE.3.E.0"),  // RDV7T
    ("RTT_SEG_93", "IO.DOUBLE.2.E.0"),  // RDV5T
    ("RTT_SEG_94", "IO.DOUBLE.1.E.0"),  // RDV3T
    ("RTT_SEG_95", "DOUBLE.H0.0"),      // DH2R
    // RTT_SEG_96 CE_2T
    ("RTT_SEG_97", "DOUBLE.H0.2"),      // DH2
    ("RTT_SEG_98", "SINGLE.H0"),        // H1R
    ("RTT_SEG_99", "OUT.LR.IOB1.I2.S"), // I2_2T
    ("RTT_SEG_100", "SINGLE.H0.E"),     // H1
    ("RTT_SEG_101", "SINGLE.H1"),       // H2R
    ("RTT_SEG_102", "SINGLE.H1.E"),     // H2
    ("RTT_SEG_103", "SINGLE.H2"),       // H3R
    // RTT_SEG_104 OK_2T
    ("RTT_SEG_105", "SINGLE.H2.E"), // H3
    ("RTT_SEG_106", "SINGLE.H3"),   // H4R
    ("RTT_SEG_107", "SINGLE.H3.E"), // H4
    ("RTT_SEG_108", "SINGLE.H4"),   // H5R
    ("RTT_SEG_109", "SINGLE.H4.E"), // H5
    ("RTT_SEG_110", "SINGLE.H5"),   // H6R
    ("RTT_SEG_111", "SINGLE.H5.E"), // H6
    ("RTT_SEG_112", "SINGLE.H6"),   // H7R
    ("RTT_SEG_113", "SINGLE.H6.E"), // H7
    ("RTT_SEG_114", "SINGLE.H7"),   // H8R
    ("RTT_SEG_115", "SINGLE.H7.E"), // H8
    ("RTT_SEG_116", "DOUBLE.H1.0"), // DH3R
    ("RTT_SEG_117", "DOUBLE.H1.2"), // DH3
    ("RTT_SEG_118", "DOUBLE.H1.1"), // DH4
    ("RTT_SEG_119", "DOUBLE.V1.2"), // DV3T
    ("RTT_SEG_120", "SINGLE.V7.S"), // V8T
    ("RTT_SEG_121", "SINGLE.V6.S"), // V7T
    ("RTT_SEG_122", "SINGLE.V5.S"), // V6T
    ("RTT_SEG_123", "SINGLE.V4.S"), // V5T
    ("RTT_SEG_124", "SINGLE.V3.S"), // V4T
    ("RTT_SEG_125", "SINGLE.V2.S"), // V3T
    ("RTT_SEG_126", "SINGLE.V1.S"), // V2T
    ("RTT_SEG_127", "SINGLE.V0.S"), // V1T
    ("RTT_SEG_128", "DOUBLE.V0.2"), // DV2T
    // RTT_SEG_129: CLOCK_5_6

    // RTS
    ("PAD27_I1", "OUT.LR.IOB0.I1"),
    ("PAD27_I2", "OUT.LR.IOB0.I2"),
    ("PAD27_IK", "IMUX.IOB0.IK"),
    ("PAD27_OK", "IMUX.IOB0.OK"),
    ("PAD27_T", "IMUX.IOB0.TS"),
    ("PAD28_I1", "OUT.LR.IOB1.I1"),
    ("PAD28_I2", "OUT.LR.IOB1.I2"),
    ("PAD28_IK", "IMUX.IOB1.IK"),
    ("PAD28_OK", "IMUX.IOB1.OK"),
    ("PAD28_T", "IMUX.IOB1.TS"),
    ("DEC_HK_2_I", "IMUX.CLB.C1"),
    ("TBUF_HK_2_I", "IMUX.TBUF0.I"),
    ("TBUF_HK_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_HK_1_I", "IMUX.TBUF1.I"),
    ("TBUF_HK_1_T", "IMUX.TBUF1.TS"),
    ("TIE_HK_1_O", "GND"),
    ("RTS_SEG_0", "LONG.IO.V0"),        // RVLL1
    ("RTS_SEG_1", "LONG.H0"),           // HLL1
    ("RTS_SEG_2", "DEC.V3"),            // RTX1
    ("RTS_SEG_6", "SINGLE.V1"),         // V2
    ("RTS_SEG_7", "LONG.IO.V1"),        // RVLL2
    ("RTS_SEG_8", "LONG.H1"),           // HLL2
    ("RTS_SEG_9", "DEC.V2"),            // RTX2
    ("RTS_SEG_10", "SINGLE.V2"),        // V3
    ("RTS_SEG_12", "LONG.H2"),          // HLL3
    ("RTS_SEG_13", "IO.DOUBLE.3.E.1"),  // RDV8
    ("RTS_SEG_14", "IO.DOUBLE.3.E.2"),  // RDV7
    ("RTS_SEG_15", "IO.DOUBLE.2.E.1"),  // RDV6
    ("RTS_SEG_16", "IO.DOUBLE.2.E.2"),  // RDV5
    ("RTS_SEG_17", "LONG.IO.V2"),       // RVLL3
    ("RTS_SEG_18", "DEC.V1"),           // RTX3
    ("RTS_SEG_21", "SINGLE.V3"),        // V4
    ("RTS_SEG_22", "DEC.V0"),           // RTX4
    ("RTS_SEG_23", "IO.DOUBLE.1.E.1"),  // RDV4
    ("RTS_SEG_25", "IO.DOUBLE.1.E.2"),  // RDV3
    ("RTS_SEG_26", "IO.DOUBLE.0.E.1"),  // RDV2
    ("RTS_SEG_27", "IO.DOUBLE.0.E.2"),  // RDV1
    ("RTS_SEG_28", "LONG.IO.V3"),       // RVLL4
    ("RTS_SEG_35", "DOUBLE.V1.0"),      // DV3
    ("RTS_SEG_36", "SINGLE.V5"),        // V6
    ("RTS_SEG_38", "GCLK3"),            // K4
    ("RTS_SEG_40", "GCLK2"),            // K3
    ("RTS_SEG_41", "GCLK1"),            // K2
    ("RTS_SEG_42", "GCLK0"),            // K1
    ("RTS_SEG_46", "SINGLE.V7"),        // V8
    ("RTS_SEG_47", "SINGLE.V0"),        // V1
    ("RTS_SEG_49", "LONG.V4"),          // VLL5
    ("RTS_SEG_50", "LONG.V3"),          // VLL4
    ("RTS_SEG_51", "DOUBLE.V1.1"),      // DV4
    ("RTS_SEG_52", "SINGLE.V6"),        // V7
    ("RTS_SEG_53", "SINGLE.V4"),        // V5
    ("RTS_SEG_54", "DOUBLE.V0.0"),      // DV2
    ("RTS_SEG_55", "DOUBLE.V0.1"),      // DV1
    ("RTS_SEG_56", "LONG.V1"),          // VLL2
    ("RTS_SEG_57", "LONG.V0"),          // VLL1
    ("RTS_SEG_59", "LONG.V5"),          // VLL6
    ("RTS_SEG_60", "IMUX.CLB.F3"),      // F3L
    ("RTS_SEG_61", "LONG.V2"),          // VLL3
    ("RTS_SEG_66", "IMUX.CLB.C3"),      // C3L
    ("RTS_SEG_69", "IMUX.CLB.G3"),      // G3L
    ("RTS_SEG_73", "OUT.CLB.GY.E"),     // GYL
    ("RTS_SEG_79", "OUT.CLB.GYQ.E"),    // GYQL
    ("RTS_SEG_84", "LONG.H3"),          // HLL4
    ("RTS_SEG_85", "IO.DBUF.V0"),       // DMUX_OUTER
    ("RTS_SEG_86", "LONG.H4"),          // HLL5
    ("RTS_SEG_87", "IO.DBUF.V1"),       // DMUX_INNER
    ("RTS_SEG_88", "LONG.H5"),          // HLL6
    ("RTS_SEG_89", "DOUBLE.H0.1"),      // DH1
    ("RTS_SEG_90", "IO.DOUBLE.0.E.0"),  // RDV1T
    ("RTS_SEG_91", "OUT.LR.IOB1.I1.S"), // I1_2T
    ("RTS_SEG_92", "IO.DOUBLE.3.E.0"),  // RDV7T
    ("RTS_SEG_93", "IO.DOUBLE.2.E.0"),  // RDV5T
    ("RTS_SEG_94", "IO.DOUBLE.1.E.0"),  // RDV3T
    ("RTS_SEG_95", "DOUBLE.H0.0"),      // DH2R
    // RTS_SEG_96 CE_2T
    ("RTS_SEG_98", "DOUBLE.H0.2"),       // DH2
    ("RTS_SEG_99", "SINGLE.H0"),         // H1R
    ("RTS_SEG_100", "OUT.LR.IOB1.I2.S"), // I2_2T
    ("RTS_SEG_101", "SINGLE.H0.E"),      // H1
    ("RTS_SEG_102", "SINGLE.H1"),        // H2R
    ("RTS_SEG_103", "SINGLE.H1.E"),      // H2
    ("RTS_SEG_104", "SINGLE.H2"),        // H3R
    // RTS_SEG_105 OK_2T
    // RTS_SEG_106 IK_2T
    ("RTS_SEG_107", "SINGLE.H2.E"), // H3
    ("RTS_SEG_108", "SINGLE.H3"),   // H4R
    ("RTS_SEG_109", "SINGLE.H3.E"), // H4
    ("RTS_SEG_110", "SINGLE.H4"),   // H5R
    ("RTS_SEG_111", "SINGLE.H4.E"), // H5
    ("RTS_SEG_112", "SINGLE.H5"),   // H6R
    ("RTS_SEG_113", "SINGLE.H5.E"), // H6
    ("RTS_SEG_114", "SINGLE.H6"),   // H7R
    ("RTS_SEG_115", "SINGLE.H6.E"), // H7
    ("RTS_SEG_116", "SINGLE.H7"),   // H8R
    ("RTS_SEG_117", "SINGLE.H7.E"), // H8
    ("RTS_SEG_118", "DOUBLE.H1.0"), // DH3R
    ("RTS_SEG_119", "DOUBLE.H1.2"), // DH3
    ("RTS_SEG_120", "DOUBLE.H1.1"), // DH4
    ("RTS_SEG_121", "DOUBLE.V1.2"), // DV3T
    ("RTS_SEG_122", "SINGLE.V7.S"), // V8T
    ("RTS_SEG_123", "SINGLE.V6.S"), // V7T
    ("RTS_SEG_124", "SINGLE.V5.S"), // V6T
    ("RTS_SEG_125", "SINGLE.V4.S"), // V5T
    ("RTS_SEG_126", "SINGLE.V3.S"), // V4T
    ("RTS_SEG_127", "SINGLE.V2.S"), // V3T
    ("RTS_SEG_128", "SINGLE.V1.S"), // V2T
    ("RTS_SEG_129", "SINGLE.V0.S"), // V1T
    ("RTS_SEG_130", "DOUBLE.V0.2"), // DV2T
    // RTS_SEG_131: CLOCK_5_6

    // RTSB
    ("I_BUFGP_BR_I_RTSB", "OUT.IOB.CLKIN"),
    ("PAD31_I1", "OUT.LR.IOB0.I1"),
    ("PAD31_I2", "OUT.LR.IOB0.I2"),
    ("PAD31_IK", "IMUX.IOB0.IK"),
    ("PAD31_OK", "IMUX.IOB0.OK"),
    ("PAD31_T", "IMUX.IOB0.TS"),
    ("PAD32_I1", "OUT.LR.IOB1.I1"),
    ("PAD32_I2", "OUT.LR.IOB1.I2"),
    ("PAD32_IK", "IMUX.IOB1.IK"),
    ("PAD32_OK", "IMUX.IOB1.OK"),
    ("PAD32_T", "IMUX.IOB1.TS"),
    ("DEC_JK_2_I", "IMUX.CLB.C1"),
    ("TBUF_JK_2_I", "IMUX.TBUF0.I"),
    ("TBUF_JK_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_JK_1_I", "IMUX.TBUF1.I"),
    ("TBUF_JK_1_T", "IMUX.TBUF1.TS"),
    ("TIE_JK_1_O", "GND"),
    ("RTSB_SEG_0", "LONG.IO.V0"),        // RVLL1
    ("RTSB_SEG_1", "LONG.H0"),           // HLL1
    ("RTSB_SEG_2", "DEC.V3"),            // RTX1
    ("RTSB_SEG_6", "SINGLE.V1"),         // V2
    ("RTSB_SEG_7", "LONG.IO.V1"),        // RVLL2
    ("RTSB_SEG_8", "LONG.H1"),           // HLL2
    ("RTSB_SEG_9", "DEC.V2"),            // RTX2
    ("RTSB_SEG_10", "SINGLE.V2"),        // V3
    ("RTSB_SEG_12", "LONG.H2"),          // HLL3
    ("RTSB_SEG_13", "IO.DOUBLE.3.E.1"),  // RDV8
    ("RTSB_SEG_14", "IO.DOUBLE.3.E.2"),  // RDV7
    ("RTSB_SEG_15", "IO.DOUBLE.2.E.1"),  // RDV6
    ("RTSB_SEG_16", "IO.DOUBLE.2.E.2"),  // RDV5
    ("RTSB_SEG_17", "LONG.IO.V2"),       // RVLL3
    ("RTSB_SEG_18", "DEC.V1"),           // RTX3
    ("RTSB_SEG_21", "SINGLE.V3"),        // V4
    ("RTSB_SEG_22", "DEC.V0"),           // RTX4
    ("RTSB_SEG_23", "IO.DOUBLE.1.E.1"),  // RDV4
    ("RTSB_SEG_25", "IO.DOUBLE.1.E.2"),  // RDV3
    ("RTSB_SEG_26", "IO.DOUBLE.0.E.1"),  // RDV2
    ("RTSB_SEG_27", "IO.DOUBLE.0.E.2"),  // RDV1
    ("RTSB_SEG_28", "LONG.IO.V3"),       // RVLL4
    ("RTSB_SEG_35", "DOUBLE.V1.0"),      // DV3
    ("RTSB_SEG_36", "SINGLE.V5"),        // V6
    ("RTSB_SEG_38", "GCLK3"),            // K4
    ("RTSB_SEG_40", "GCLK2"),            // K3
    ("RTSB_SEG_41", "GCLK1"),            // K2
    ("RTSB_SEG_42", "GCLK0"),            // K1
    ("RTSB_SEG_46", "SINGLE.V7"),        // V8
    ("RTSB_SEG_47", "SINGLE.V0"),        // V1
    ("RTSB_SEG_49", "LONG.V4"),          // VLL5
    ("RTSB_SEG_50", "LONG.V3"),          // VLL4
    ("RTSB_SEG_51", "DOUBLE.V1.1"),      // DV4
    ("RTSB_SEG_52", "SINGLE.V6"),        // V7
    ("RTSB_SEG_53", "SINGLE.V4"),        // V5
    ("RTSB_SEG_54", "DOUBLE.V0.0"),      // DV2
    ("RTSB_SEG_55", "DOUBLE.V0.1"),      // DV1
    ("RTSB_SEG_56", "LONG.V1"),          // VLL2
    ("RTSB_SEG_57", "LONG.V0"),          // VLL1
    ("RTSB_SEG_59", "LONG.V5"),          // VLL6
    ("RTSB_SEG_60", "IMUX.CLB.F3"),      // F3L
    ("RTSB_SEG_61", "LONG.V2"),          // VLL3
    ("RTSB_SEG_66", "IMUX.CLB.C3"),      // C3L
    ("RTSB_SEG_69", "IMUX.CLB.G3"),      // G3L
    ("RTSB_SEG_73", "OUT.CLB.GY.E"),     // GYL
    ("RTSB_SEG_79", "OUT.CLB.GYQ.E"),    // GYQL
    ("RTSB_SEG_84", "LONG.H3"),          // HLL4
    ("RTSB_SEG_85", "IO.DBUF.V0"),       // DMUX_OUTER
    ("RTSB_SEG_86", "LONG.H4"),          // HLL5
    ("RTSB_SEG_87", "IO.DBUF.V1"),       // DMUX_INNER
    ("RTSB_SEG_88", "LONG.H5"),          // HLL6
    ("RTSB_SEG_89", "DOUBLE.H0.1"),      // DH1
    ("RTSB_SEG_90", "IO.DOUBLE.0.E.0"),  // RDV1T
    ("RTSB_SEG_91", "OUT.LR.IOB1.I1.S"), // I1_2T
    ("RTSB_SEG_92", "IO.DOUBLE.3.E.0"),  // RDV7T
    ("RTSB_SEG_93", "IO.DOUBLE.2.E.0"),  // RDV5T
    ("RTSB_SEG_94", "IO.DOUBLE.1.E.0"),  // RDV3T
    ("RTSB_SEG_95", "DOUBLE.H0.0"),      // DH2R
    // RTSB_SEG_96 CE_2T
    ("RTSB_SEG_98", "DOUBLE.H0.2"),       // DH2
    ("RTSB_SEG_99", "SINGLE.H0"),         // H1R
    ("RTSB_SEG_100", "OUT.LR.IOB1.I2.S"), // I2_2T
    ("RTSB_SEG_101", "SINGLE.H0.E"),      // H1
    ("RTSB_SEG_102", "SINGLE.H1"),        // H2R
    ("RTSB_SEG_103", "SINGLE.H1.E"),      // H2
    ("RTSB_SEG_104", "SINGLE.H2"),        // H3R
    // RTSB_SEG_105 OK_2T
    // RTSB_SEG_106 IK_2T
    ("RTSB_SEG_107", "SINGLE.H2.E"), // H3
    ("RTSB_SEG_108", "SINGLE.H3"),   // H4R
    ("RTSB_SEG_109", "SINGLE.H3.E"), // H4
    ("RTSB_SEG_110", "SINGLE.H4"),   // H5R
    ("RTSB_SEG_111", "SINGLE.H4.E"), // H5
    ("RTSB_SEG_112", "SINGLE.H5"),   // H6R
    ("RTSB_SEG_113", "SINGLE.H5.E"), // H6
    ("RTSB_SEG_114", "SINGLE.H6"),   // H7R
    ("RTSB_SEG_115", "SINGLE.H6.E"), // H7
    ("RTSB_SEG_116", "SINGLE.H7"),   // H8R
    ("RTSB_SEG_117", "SINGLE.H7.E"), // H8
    ("RTSB_SEG_118", "DOUBLE.H1.0"), // DH3R
    ("RTSB_SEG_119", "DOUBLE.H1.2"), // DH3
    ("RTSB_SEG_120", "DOUBLE.H1.1"), // DH4
    ("RTSB_SEG_121", "DOUBLE.V1.2"), // DV3T
    ("RTSB_SEG_122", "SINGLE.V7.S"), // V8T
    ("RTSB_SEG_123", "SINGLE.V6.S"), // V7T
    ("RTSB_SEG_124", "SINGLE.V5.S"), // V6T
    ("RTSB_SEG_125", "SINGLE.V4.S"), // V5T
    ("RTSB_SEG_126", "SINGLE.V3.S"), // V4T
    ("RTSB_SEG_127", "SINGLE.V2.S"), // V3T
    ("RTSB_SEG_128", "SINGLE.V1.S"), // V2T
    ("RTSB_SEG_129", "SINGLE.V0.S"), // V1T
    ("RTSB_SEG_130", "DOUBLE.V0.2"), // DV2T
    // RTSB_SEG_131: CLOCK_5_6

    // LEFT
    ("PAD60_I1", "OUT.LR.IOB0.I1"),
    ("PAD60_I2", "OUT.LR.IOB0.I2"),
    ("PAD60_IK", "IMUX.IOB0.IK"),
    ("PAD60_OK", "IMUX.IOB0.OK"),
    ("PAD60_T", "IMUX.IOB0.TS"),
    ("PAD59_I1", "OUT.LR.IOB1.I1"),
    ("PAD59_I2", "OUT.LR.IOB1.I2"),
    ("PAD59_IK", "IMUX.IOB1.IK"),
    ("PAD59_OK", "IMUX.IOB1.OK"),
    ("PAD59_T", "IMUX.IOB1.TS"),
    ("DEC_DA_2_I", "IMUX.CLB.C3.W"),
    ("TBUF_DA_2_I", "IMUX.TBUF0.I"),
    ("TBUF_DA_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_DA_1_I", "IMUX.TBUF1.I"),
    ("TBUF_DA_1_T", "IMUX.TBUF1.TS"),
    ("LEFT_SEG_1", "LONG.H0"),           // HLL1
    ("LEFT_SEG_4", "DEC.V0"),            // LTX1
    ("LEFT_SEG_5", "LONG.IO.V0"),        // LVLL1
    ("LEFT_SEG_6", "LONG.H1"),           // HLL2
    ("LEFT_SEG_7", "DEC.V1"),            // LTX2
    ("LEFT_SEG_8", "LONG.IO.V1"),        // LVLL2
    ("LEFT_SEG_9", "LONG.H2"),           // HLL3
    ("LEFT_SEG_12", "DEC.V2"),           // LTX3
    ("LEFT_SEG_13", "LONG.IO.V2"),       // LVLL3
    ("LEFT_SEG_14", "IO.DOUBLE.3.W.1"),  // LDV8
    ("LEFT_SEG_15", "IO.DOUBLE.3.W.0"),  // LDV7T
    ("LEFT_SEG_16", "IO.DOUBLE.2.W.1"),  // LDV6
    ("LEFT_SEG_17", "IO.DOUBLE.2.W.0"),  // LDV5T
    ("LEFT_SEG_19", "DEC.V3"),           // LTX4
    ("LEFT_SEG_21", "LONG.IO.V3"),       // LVLL4
    ("LEFT_SEG_22", "IO.DOUBLE.1.W.1"),  // LDV4
    ("LEFT_SEG_23", "IO.DOUBLE.1.W.0"),  // LDV3T
    ("LEFT_SEG_24", "IO.DOUBLE.0.W.1"),  // LDV2
    ("LEFT_SEG_25", "IO.DOUBLE.0.W.0"),  // LDV1T
    ("LEFT_SEG_33", "GCLK3"),            // K4
    ("LEFT_SEG_34", "GCLK2"),            // K3
    ("LEFT_SEG_35", "GCLK1"),            // K2
    ("LEFT_SEG_36", "GCLK0"),            // K1
    ("LEFT_SEG_60", "LONG.H3"),          // HLL4
    ("LEFT_SEG_61", "LONG.H4"),          // HLL5
    ("LEFT_SEG_62", "IO.DBUF.V0"),       // DMUX_OUTER
    ("LEFT_SEG_63", "LONG.H5"),          // HLL6
    ("LEFT_SEG_65", "DOUBLE.H0.0"),      // DH1
    ("LEFT_SEG_64", "IO.DBUF.V1"),       // DMUX_INNER
    ("LEFT_SEG_66", "OUT.LR.IOB1.I1.S"), // I1_2T
    ("LEFT_SEG_67", "IO.DOUBLE.0.W.2"),  // LDV1
    ("LEFT_SEG_68", "IO.DOUBLE.3.W.2"),  // LDV7
    ("LEFT_SEG_69", "IO.DOUBLE.2.W.2"),  // LDV5
    ("LEFT_SEG_70", "SINGLE.H1"),        // H2
    // LEFT_SEG_71: CE_2T
    ("LEFT_SEG_72", "DOUBLE.H0.1"),      // DH2
    ("LEFT_SEG_74", "IO.DOUBLE.1.W.2"),  // LDV3
    ("LEFT_SEG_75", "OUT.LR.IOB1.I2.S"), // I2_2T
    ("LEFT_SEG_76", "SINGLE.H0"),        // H1
    ("LEFT_SEG_77", "SINGLE.H2"),        // H3
    // LEFT_SEG_78: IK_2T
    // LEFT_SEG_79: OK_2T
    ("LEFT_SEG_80", "SINGLE.H3"),   // H4
    ("LEFT_SEG_81", "SINGLE.H4"),   // H5
    ("LEFT_SEG_82", "SINGLE.H5"),   // H6
    ("LEFT_SEG_83", "SINGLE.H6"),   // H7
    ("LEFT_SEG_84", "SINGLE.H7"),   // H8
    ("LEFT_SEG_85", "DOUBLE.H1.1"), // DH3
    ("LEFT_SEG_86", "DOUBLE.H1.0"), // DH4
    // LEFT_SEG_87: CLOCK_1_2
    // LEFTT
    ("I_BUFGP_TL_I_LEFTT", "OUT.IOB.CLKIN"),
    ("PAD64_I1", "OUT.LR.IOB0.I1"),
    ("PAD64_I2", "OUT.LR.IOB0.I2"),
    ("PAD64_IK", "IMUX.IOB0.IK"),
    ("PAD64_OK", "IMUX.IOB0.OK"),
    ("PAD64_T", "IMUX.IOB0.TS"),
    ("PAD63_I1", "OUT.LR.IOB1.I1"),
    ("PAD63_I2", "OUT.LR.IOB1.I2"),
    ("PAD63_IK", "IMUX.IOB1.IK"),
    ("PAD63_OK", "IMUX.IOB1.OK"),
    ("PAD63_T", "IMUX.IOB1.TS"),
    ("DEC_BA_2_I", "IMUX.CLB.C3.W"),
    ("TBUF_BA_2_I", "IMUX.TBUF0.I"),
    ("TBUF_BA_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_BA_1_I", "IMUX.TBUF1.I"),
    ("TBUF_BA_1_T", "IMUX.TBUF1.TS"),
    ("LEFTT_SEG_1", "LONG.H0"),           // HLL1
    ("LEFTT_SEG_4", "DEC.V0"),            // LTX1
    ("LEFTT_SEG_5", "LONG.IO.V0"),        // LVLL1
    ("LEFTT_SEG_6", "LONG.H1"),           // HLL2
    ("LEFTT_SEG_7", "DEC.V1"),            // LTX2
    ("LEFTT_SEG_8", "LONG.IO.V1"),        // LVLL2
    ("LEFTT_SEG_9", "LONG.H2"),           // HLL3
    ("LEFTT_SEG_12", "DEC.V2"),           // LTX3
    ("LEFTT_SEG_13", "LONG.IO.V2"),       // LVLL3
    ("LEFTT_SEG_14", "IO.DOUBLE.3.W.1"),  // LDV8
    ("LEFTT_SEG_15", "IO.DOUBLE.3.W.0"),  // LDV7T
    ("LEFTT_SEG_16", "IO.DOUBLE.2.W.1"),  // LDV6
    ("LEFTT_SEG_17", "IO.DOUBLE.2.W.0"),  // LDV5T
    ("LEFTT_SEG_19", "DEC.V3"),           // LTX4
    ("LEFTT_SEG_21", "LONG.IO.V3"),       // LVLL4
    ("LEFTT_SEG_22", "IO.DOUBLE.1.W.1"),  // LDV4
    ("LEFTT_SEG_23", "IO.DOUBLE.1.W.0"),  // LDV3T
    ("LEFTT_SEG_24", "IO.DOUBLE.0.W.1"),  // LDV2
    ("LEFTT_SEG_25", "IO.DOUBLE.0.W.0"),  // LDV1T
    ("LEFTT_SEG_33", "GCLK3"),            // K4
    ("LEFTT_SEG_34", "GCLK2"),            // K3
    ("LEFTT_SEG_35", "GCLK1"),            // K2
    ("LEFTT_SEG_36", "GCLK0"),            // K1
    ("LEFTT_SEG_60", "LONG.H3"),          // HLL4
    ("LEFTT_SEG_61", "LONG.H4"),          // HLL5
    ("LEFTT_SEG_62", "IO.DBUF.V0"),       // DMUX_OUTER
    ("LEFTT_SEG_63", "LONG.H5"),          // HLL6
    ("LEFTT_SEG_65", "DOUBLE.H0.0"),      // DH1
    ("LEFTT_SEG_64", "IO.DBUF.V1"),       // DMUX_INNER
    ("LEFTT_SEG_66", "OUT.LR.IOB1.I1.S"), // I1_2T
    ("LEFTT_SEG_67", "IO.DOUBLE.0.W.2"),  // LDV1
    ("LEFTT_SEG_68", "IO.DOUBLE.3.W.2"),  // LDV7
    ("LEFTT_SEG_69", "IO.DOUBLE.2.W.2"),  // LDV5
    ("LEFTT_SEG_70", "SINGLE.H1"),        // H2
    // LEFTT_SEG_71: CE_2T
    ("LEFTT_SEG_72", "DOUBLE.H0.1"),      // DH2
    ("LEFTT_SEG_73", "IO.DOUBLE.1.W.2"),  // LDV3
    ("LEFTT_SEG_74", "OUT.LR.IOB1.I2.S"), // I2_2T
    ("LEFTT_SEG_75", "SINGLE.H0"),        // H1
    ("LEFTT_SEG_76", "SINGLE.H2"),        // H3
    // LEFTT_SEG_77: IK_2T
    ("LEFTT_SEG_78", "SINGLE.H3"),   // H4
    ("LEFTT_SEG_79", "SINGLE.H4"),   // H5
    ("LEFTT_SEG_80", "SINGLE.H5"),   // H6
    ("LEFTT_SEG_81", "SINGLE.H6"),   // H7
    ("LEFTT_SEG_82", "SINGLE.H7"),   // H8
    ("LEFTT_SEG_83", "DOUBLE.H1.1"), // DH3
    ("LEFTT_SEG_84", "DOUBLE.H1.0"), // DH4
    // LEFTT_SEG_85: CLOCK_1_2

    // LEFTS
    ("PAD54_I1", "OUT.LR.IOB0.I1"),
    ("PAD54_I2", "OUT.LR.IOB0.I2"),
    ("PAD54_IK", "IMUX.IOB0.IK"),
    ("PAD54_OK", "IMUX.IOB0.OK"),
    ("PAD54_T", "IMUX.IOB0.TS"),
    ("PAD53_I1", "OUT.LR.IOB1.I1"),
    ("PAD53_I2", "OUT.LR.IOB1.I2"),
    ("PAD53_IK", "IMUX.IOB1.IK"),
    ("PAD53_OK", "IMUX.IOB1.OK"),
    ("PAD53_T", "IMUX.IOB1.TS"),
    ("DEC_HA_2_I", "IMUX.CLB.C3.W"),
    ("TBUF_HA_2_I", "IMUX.TBUF0.I"),
    ("TBUF_HA_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_HA_1_I", "IMUX.TBUF1.I"),
    ("TBUF_HA_1_T", "IMUX.TBUF1.TS"),
    ("LEFTS_SEG_1", "LONG.H0"),           // HLL1
    ("LEFTS_SEG_4", "DEC.V0"),            // LTX1
    ("LEFTS_SEG_5", "LONG.IO.V0"),        // LVLL1
    ("LEFTS_SEG_6", "LONG.H1"),           // HLL2
    ("LEFTS_SEG_7", "DEC.V1"),            // LTX2
    ("LEFTS_SEG_8", "LONG.IO.V1"),        // LVLL2
    ("LEFTS_SEG_9", "LONG.H2"),           // HLL3
    ("LEFTS_SEG_12", "DEC.V2"),           // LTX3
    ("LEFTS_SEG_13", "LONG.IO.V2"),       // LVLL3
    ("LEFTS_SEG_14", "IO.DOUBLE.3.W.1"),  // LDV8
    ("LEFTS_SEG_15", "IO.DOUBLE.3.W.0"),  // LDV7T
    ("LEFTS_SEG_16", "IO.DOUBLE.2.W.1"),  // LDV6
    ("LEFTS_SEG_17", "IO.DOUBLE.2.W.0"),  // LDV5T
    ("LEFTS_SEG_19", "DEC.V3"),           // LTX4
    ("LEFTS_SEG_21", "LONG.IO.V3"),       // LVLL4
    ("LEFTS_SEG_22", "IO.DOUBLE.1.W.1"),  // LDV4
    ("LEFTS_SEG_23", "IO.DOUBLE.1.W.0"),  // LDV3T
    ("LEFTS_SEG_24", "IO.DOUBLE.0.W.1"),  // LDV2
    ("LEFTS_SEG_25", "IO.DOUBLE.0.W.0"),  // LDV1T
    ("LEFTS_SEG_33", "GCLK3"),            // K4
    ("LEFTS_SEG_34", "GCLK2"),            // K3
    ("LEFTS_SEG_35", "GCLK1"),            // K2
    ("LEFTS_SEG_36", "GCLK0"),            // K1
    ("LEFTS_SEG_60", "LONG.H3"),          // HLL4
    ("LEFTS_SEG_61", "LONG.H4"),          // HLL5
    ("LEFTS_SEG_62", "IO.DBUF.V0"),       // DMUX_OUTER
    ("LEFTS_SEG_63", "LONG.H5"),          // HLL6
    ("LEFTS_SEG_65", "DOUBLE.H0.0"),      // DH1
    ("LEFTS_SEG_64", "IO.DBUF.V1"),       // DMUX_INNER
    ("LEFTS_SEG_66", "OUT.LR.IOB1.I1.S"), // I1_2T
    ("LEFTS_SEG_67", "IO.DOUBLE.0.W.2"),  // LDV1
    ("LEFTS_SEG_68", "IO.DOUBLE.3.W.2"),  // LDV7
    ("LEFTS_SEG_69", "IO.DOUBLE.2.W.2"),  // LDV5
    ("LEFTS_SEG_70", "IO.DOUBLE.1.W.2"),  // LDV3
    // LEFTS_SEG_71: CE_2T
    ("LEFTS_SEG_72", "DOUBLE.H0.1"),      // DH2
    ("LEFTS_SEG_74", "OUT.LR.IOB1.I2.S"), // I2_2T
    ("LEFTS_SEG_75", "SINGLE.H0"),        // H1
    ("LEFTS_SEG_76", "SINGLE.H1"),        // H2
    ("LEFTS_SEG_77", "SINGLE.H2"),        // H3
    // LEFTS_SEG_78: IK_2T
    // LEFTS_SEG_79: OK_2T
    ("LEFTS_SEG_80", "SINGLE.H3"),   // H4
    ("LEFTS_SEG_81", "SINGLE.H4"),   // H5
    ("LEFTS_SEG_82", "SINGLE.H5"),   // H6
    ("LEFTS_SEG_83", "SINGLE.H6"),   // H7
    ("LEFTS_SEG_84", "SINGLE.H7"),   // H8
    ("LEFTS_SEG_85", "DOUBLE.H1.1"), // DH3
    ("LEFTS_SEG_86", "DOUBLE.H1.0"), // DH4
    // LEFTS_SEG_87: CLOCK_1_2
    // LEFTSB
    ("I_BUFGS_BL_I_LEFTSB", "OUT.IOB.CLKIN"),
    ("PAD50_I1", "OUT.LR.IOB0.I1"),
    ("PAD50_I2", "OUT.LR.IOB0.I2"),
    ("PAD50_IK", "IMUX.IOB0.IK"),
    ("PAD50_OK", "IMUX.IOB0.OK"),
    ("PAD50_T", "IMUX.IOB0.TS"),
    ("PAD49_I1", "OUT.LR.IOB1.I1"),
    ("PAD49_I2", "OUT.LR.IOB1.I2"),
    ("PAD49_IK", "IMUX.IOB1.IK"),
    ("PAD49_OK", "IMUX.IOB1.OK"),
    ("PAD49_T", "IMUX.IOB1.TS"),
    ("DEC_JA_2_I", "IMUX.CLB.C3.W"),
    ("TBUF_JA_2_I", "IMUX.TBUF0.I"),
    ("TBUF_JA_2_T", "IMUX.TBUF0.TS"),
    ("TBUF_JA_1_I", "IMUX.TBUF1.I"),
    ("TBUF_JA_1_T", "IMUX.TBUF1.TS"),
    ("LEFTSB_SEG_1", "LONG.H0"),           // HLL1
    ("LEFTSB_SEG_4", "DEC.V0"),            // LTX1
    ("LEFTSB_SEG_5", "LONG.IO.V0"),        // LVLL1
    ("LEFTSB_SEG_6", "LONG.H1"),           // HLL2
    ("LEFTSB_SEG_7", "DEC.V1"),            // LTX2
    ("LEFTSB_SEG_8", "LONG.IO.V1"),        // LVLL2
    ("LEFTSB_SEG_9", "LONG.H2"),           // HLL3
    ("LEFTSB_SEG_12", "DEC.V2"),           // LTX3
    ("LEFTSB_SEG_13", "LONG.IO.V2"),       // LVLL3
    ("LEFTSB_SEG_14", "IO.DOUBLE.3.W.1"),  // LDV8
    ("LEFTSB_SEG_15", "IO.DOUBLE.3.W.0"),  // LDV7T
    ("LEFTSB_SEG_16", "IO.DOUBLE.2.W.1"),  // LDV6
    ("LEFTSB_SEG_17", "IO.DOUBLE.2.W.0"),  // LDV5T
    ("LEFTSB_SEG_19", "DEC.V3"),           // LTX4
    ("LEFTSB_SEG_21", "LONG.IO.V3"),       // LVLL4
    ("LEFTSB_SEG_22", "IO.DOUBLE.1.W.1"),  // LDV4
    ("LEFTSB_SEG_23", "IO.DOUBLE.1.W.0"),  // LDV3T
    ("LEFTSB_SEG_24", "IO.DOUBLE.0.W.1"),  // LDV2
    ("LEFTSB_SEG_25", "IO.DOUBLE.0.W.0"),  // LDV1T
    ("LEFTSB_SEG_33", "GCLK3"),            // K4
    ("LEFTSB_SEG_34", "GCLK2"),            // K3
    ("LEFTSB_SEG_35", "GCLK1"),            // K2
    ("LEFTSB_SEG_36", "GCLK0"),            // K1
    ("LEFTSB_SEG_60", "LONG.H3"),          // HLL4
    ("LEFTSB_SEG_61", "LONG.H4"),          // HLL5
    ("LEFTSB_SEG_62", "IO.DBUF.V0"),       // DMUX_OUTER
    ("LEFTSB_SEG_63", "LONG.H5"),          // HLL6
    ("LEFTSB_SEG_65", "DOUBLE.H0.0"),      // DH1
    ("LEFTSB_SEG_64", "IO.DBUF.V1"),       // DMUX_INNER
    ("LEFTSB_SEG_66", "OUT.LR.IOB1.I1.S"), // I1_2T
    ("LEFTSB_SEG_67", "IO.DOUBLE.0.W.2"),  // LDV1
    ("LEFTSB_SEG_68", "IO.DOUBLE.3.W.2"),  // LDV7
    ("LEFTSB_SEG_69", "IO.DOUBLE.2.W.2"),  // LDV5
    ("LEFTSB_SEG_70", "IO.DOUBLE.1.W.2"),  // LDV3
    // LEFTSB_SEG_71: CE_2T
    ("LEFTSB_SEG_72", "DOUBLE.H0.1"),      // DH2
    ("LEFTSB_SEG_74", "OUT.LR.IOB1.I2.S"), // I2_2T
    ("LEFTSB_SEG_75", "SINGLE.H0"),        // H1
    ("LEFTSB_SEG_76", "SINGLE.H1"),        // H2
    ("LEFTSB_SEG_77", "SINGLE.H2"),        // H3
    // LEFTSB_SEG_78: IK_2T
    // LEFTSB_SEG_79: OK_2T
    ("LEFTSB_SEG_80", "SINGLE.H3"),   // H4
    ("LEFTSB_SEG_81", "SINGLE.H4"),   // H5
    ("LEFTSB_SEG_82", "SINGLE.H5"),   // H6
    ("LEFTSB_SEG_83", "SINGLE.H6"),   // H7
    ("LEFTSB_SEG_84", "SINGLE.H7"),   // H8
    ("LEFTSB_SEG_85", "DOUBLE.H1.1"), // DH3
    ("LEFTSB_SEG_86", "DOUBLE.H1.0"), // DH4
    // LEFTSB_SEG_87: CLOCK_1_2

    // LR
    ("STARTUP_Q1Q4", "OUT.STARTUP.Q1Q4"),
    ("STARTUP_Q2", "OUT.STARTUP.Q2"),
    ("STARTUP_Q3", "OUT.STARTUP.Q3"),
    ("STARTUP_DONEIN", "OUT.STARTUP.DONEIN"),
    ("STARTUP_CLK", "IMUX.STARTUP.CLK"),
    ("STARTUP_GSR", "IMUX.STARTUP.GSR"),
    ("STARTUP_GTS", "IMUX.STARTUP.GTS"),
    ("RDCLK_I", "IMUX.READCLK.I"),
    ("BUFGP_BR_I", "IMUX.BUFG.V"),
    ("BUFGS_BR_I", "IMUX.BUFG.H"),
    ("I_BUFGP_BR_I", "OUT.IOB.CLKIN.S"),
    ("I_BUFGS_BR_I", "OUT.IOB.CLKIN.E"),
    ("TIE_KK_1_O", "GND"),
    ("LR_SEG_0", "IO.DBUF.H0"),       // BDMUX_OUTER
    ("LR_SEG_1", "IO.DOUBLE.0.E.1"),  // RDV1
    ("LR_SEG_2", "IO.DBUF.H1"),       // BDMUX_INNER
    ("LR_SEG_3", "SINGLE.V1"),        // V2
    ("LR_SEG_4", "IO.DOUBLE.0.S.2"),  // BDH1
    ("LR_SEG_5", "DOUBLE.V0.1"),      // DV1
    ("LR_SEG_6", "SINGLE.V0"),        // V1
    ("LR_SEG_7", "IO.DOUBLE.0.S.1"),  // BDH2
    ("LR_SEG_8", "IO.DOUBLE.1.E.1"),  // RDV3
    ("LR_SEG_9", "SINGLE.V3"),        // V4
    ("LR_SEG_10", "DOUBLE.V0.0"),     // DV2
    ("LR_SEG_11", "IO.DOUBLE.1.S.2"), // BDH3
    ("LR_SEG_12", "SINGLE.V2"),       // V3
    ("LR_SEG_13", "IO.DOUBLE.1.S.1"), // BDH4
    ("LR_SEG_14", "IO.DOUBLE.2.E.1"), // RDV5
    ("LR_SEG_15", "DOUBLE.V1.0"),     // DV3
    ("LR_SEG_16", "SINGLE.V5"),       // V6
    ("LR_SEG_17", "IO.DOUBLE.2.S.2"), // BDH5
    ("LR_SEG_18", "IO.DOUBLE.2.S.1"), // BDH6
    ("LR_SEG_19", "SINGLE.V4"),       // V5
    ("LR_SEG_20", "IO.DOUBLE.3.E.1"), // RDV7
    ("LR_SEG_21", "DOUBLE.V1.1"),     // DV4
    ("LR_SEG_22", "SINGLE.V7"),       // V8
    ("LR_SEG_23", "IO.DOUBLE.3.S.2"), // BDH7
    ("LR_SEG_24", "IO.DOUBLE.3.S.1"), // BDH8
    ("LR_SEG_25", "SINGLE.V6"),       // V7
    ("LR_SEG_26", "LONG.IO.V2"),      // RVLL3
    ("LR_SEG_27", "LONG.IO.H0"),      // BHLL1
    ("LR_SEG_28", "LONG.IO.V0"),      // RVLL1
    ("LR_SEG_29", "LONG.V0"),         // VLL1
    ("LR_SEG_30", "LONG.IO.V3"),      // RVLL4
    ("LR_SEG_31", "LONG.IO.H1"),      // BHLL2
    ("LR_SEG_32", "LONG.IO.V1"),      // RVLL2
    ("LR_SEG_33", "LONG.V3"),         // VLL4
    ("LR_SEG_34", "LONG.V1"),         // VLL2
    ("LR_SEG_35", "LONG.IO.H2"),      // BHLL3
    ("LR_SEG_36", "LONG.V4"),         // VLL5
    ("LR_SEG_37", "LONG.V2"),         // VLL3
    ("LR_SEG_38", "LONG.IO.H3"),      // BHLL4
    ("LR_SEG_39", "LONG.V5"),         // VLL6
    ("LR_SEG_41", "DEC.H0"),          // BTX1
    ("LR_SEG_44", "DEC.H1"),          // BTX2
    ("LR_SEG_46", "DEC.H2"),          // BTX3
    ("LR_SEG_50", "DEC.H3"),          // BTX4
    // LR_SEG_58 LOK_2
    // LR_SEG_59 LIK_2
    ("LR_SEG_63", "OUT.BT.IOB1.I1.E"), // LI1_2
    // LR_SEG_69 LO_2 [aka LCE_2]
    ("LR_SEG_76", "OUT.BT.IOB1.I2.E"), // LI2_2
    ("LR_SEG_72", "DEC.V0"),           // RTX4
    ("LR_SEG_73", "DEC.V1"),           // RTX3
    ("LR_SEG_74", "DEC.V2"),           // RTX2
    ("LR_SEG_75", "DEC.V3"),           // RTX1
    ("LR_SEG_77", "LONG.H3"),          // HLL4
    ("LR_SEG_78", "IO.DBUF.V0"),       // RDMUX_OUTER
    ("LR_SEG_79", "LONG.H4"),          // HLL5
    ("LR_SEG_80", "IO.DBUF.V1"),       // RDMUX_INNER
    ("LR_SEG_81", "LONG.H5"),          // HLL6
    ("LR_SEG_82", "DOUBLE.H0.1"),      // DH1
    ("LR_SEG_83", "IO.DOUBLE.0.E.0"),  // RDV2
    ("LR_SEG_84", "OUT.LR.IOB1.I1.S"), // TI1_2
    ("LR_SEG_85", "IO.DOUBLE.3.E.0"),  // RDV8
    ("LR_SEG_86", "IO.DOUBLE.2.E.0"),  // RDV6
    ("LR_SEG_87", "IO.DOUBLE.1.E.0"),  // RDV4
    ("LR_SEG_88", "DOUBLE.H0.0"),      // DH2R
    // LR_SEG_89 TCE_2
    ("LR_SEG_91", "DOUBLE.H0.2"),      // DH2
    ("LR_SEG_92", "SINGLE.H0"),        // H1R
    ("LR_SEG_93", "OUT.LR.IOB1.I2.S"), // TI2_2
    ("LR_SEG_94", "SINGLE.H0.E"),      // H1
    ("LR_SEG_95", "SINGLE.H1"),        // H2R
    ("LR_SEG_96", "SINGLE.H1.E"),      // H2
    ("LR_SEG_97", "SINGLE.H2"),        // H3R
    // LR_SEG_98 TOK_2
    // LR_SEG_99 TIK_2
    ("LR_SEG_100", "SINGLE.H2.E"), // H3
    ("LR_SEG_101", "SINGLE.H3"),   // H4R
    ("LR_SEG_102", "SINGLE.H3.E"), // H4
    ("LR_SEG_103", "SINGLE.H4"),   // H5R
    ("LR_SEG_104", "SINGLE.H4.E"), // H5
    ("LR_SEG_105", "SINGLE.H5"),   // H6R
    ("LR_SEG_106", "SINGLE.H5.E"), // H6
    ("LR_SEG_107", "SINGLE.H6"),   // H7R
    ("LR_SEG_108", "SINGLE.H6.E"), // H7
    ("LR_SEG_109", "SINGLE.H7"),   // H8R
    ("LR_SEG_110", "SINGLE.H7.E"), // H8
    ("LR_SEG_111", "DOUBLE.H1.0"), // DH3R
    ("LR_SEG_112", "DOUBLE.H1.2"), // DH3
    ("LR_SEG_113", "DOUBLE.H1.1"), // DH4
    ("LR_SEG_114", "DOUBLE.V1.2"), // DV3T
    ("LR_SEG_115", "SINGLE.V7.S"), // V8T
    ("LR_SEG_116", "SINGLE.V6.S"), // V7T
    ("LR_SEG_117", "SINGLE.V5.S"), // V6T
    ("LR_SEG_118", "SINGLE.V4.S"), // V5T
    ("LR_SEG_119", "SINGLE.V3.S"), // V4T
    ("LR_SEG_120", "SINGLE.V2.S"), // V3T
    ("LR_SEG_121", "SINGLE.V1.S"), // V2T
    ("LR_SEG_122", "SINGLE.V0.S"), // V1T
    ("LR_SEG_123", "DOUBLE.V0.2"), // DV2T
    // UR
    ("TDO_O", "IMUX.TDO.O"),
    ("TDO_T", "IMUX.TDO.T"),
    ("UPDATE_O", "OUT.UPDATE.O"),
    ("OSC_F8M", "OUT.LR.IOB1.I1"),
    ("BUFGP_TR_I", "IMUX.BUFG.H"),
    ("BUFGS_TR_I", "IMUX.BUFG.V"),
    ("I_BUFGP_TR_I", "OUT.IOB.CLKIN.E"),
    ("I_BUFGS_TR_I", "OUT.IOB.CLKIN.N"),
    ("UR_SEG_0", "LONG.IO.V0"),        // RVLL1
    ("UR_SEG_1", "LONG.H0"),           // HLL1
    ("UR_SEG_2", "DEC.V3"),            // RTX1
    ("UR_SEG_4", "OUT.LR.IOB1.I2"),    // OSC_OUT
    ("UR_SEG_5", "SINGLE.V1"),         // V2
    ("UR_SEG_6", "LONG.IO.V1"),        // RVLL2
    ("UR_SEG_7", "LONG.H1"),           // HLL2
    ("UR_SEG_8", "DEC.V2"),            // RTX2
    ("UR_SEG_9", "SINGLE.V2"),         // V3
    ("UR_SEG_10", "LONG.IO.V2"),       // RVLL3
    ("UR_SEG_11", "LONG.H2"),          // HLL3
    ("UR_SEG_12", "DEC.V1"),           // RTX3
    ("UR_SEG_13", "SINGLE.V3"),        // V4
    ("UR_SEG_14", "DOUBLE.V1.1"),      // DV4
    ("UR_SEG_15", "OUT.BT.IOB1.I2.E"), // I2_2
    ("UR_SEG_16", "SINGLE.V4"),        // V5
    ("UR_SEG_17", "SINGLE.V0"),        // V1
    ("UR_SEG_18", "LONG.V2"),          // VLL3
    ("UR_SEG_19", "LONG.V1"),          // VLL2
    ("UR_SEG_20", "LONG.V0"),          // VLL1
    ("UR_SEG_22", "DEC.V0"),           // RTX4
    ("UR_SEG_26", "LONG.V5"),          // VLL6
    ("UR_SEG_28", "LONG.V4"),          // VLL5
    ("UR_SEG_29", "LONG.V3"),          // VLL4
    ("UR_SEG_30", "DOUBLE.V0.0"),      // DV2
    ("UR_SEG_31", "DOUBLE.V1.0"),      // DV3
    ("UR_SEG_33", "DOUBLE.V0.1"),      // DV1
    // UR_SEG_35: CE_2
    ("UR_SEG_37", "SINGLE.V6"),        // V7
    ("UR_SEG_42", "SINGLE.V7"),        // V8
    ("UR_SEG_43", "OUT.BT.IOB1.I1.E"), // I1_2
    ("UR_SEG_44", "OUT.OSC.MUX1"),     // OSC_IN
    ("UR_SEG_45", "SINGLE.V5"),        // V6
    // UR_SEG_47: IK_2
    // UR_SEG_48: OK_2
    ("UR_SEG_54", "DEC.H3"),          // TTX1
    ("UR_SEG_56", "DEC.H2"),          // TTX2
    ("UR_SEG_59", "DEC.H1"),          // TTX3
    ("UR_SEG_60", "IO.DOUBLE.3.E.1"), // D8
    ("UR_SEG_61", "IO.DOUBLE.3.E.2"), // D7
    ("UR_SEG_62", "IO.DOUBLE.2.E.1"), // D6
    ("UR_SEG_63", "IO.DOUBLE.2.E.2"), // D5
    ("UR_SEG_64", "IO.DOUBLE.1.E.1"), // D4
    ("UR_SEG_65", "IO.DOUBLE.1.E.2"), // D3
    ("UR_SEG_66", "IO.DOUBLE.0.E.1"), // D2
    ("UR_SEG_67", "IO.DOUBLE.0.E.2"), // D1
    ("UR_SEG_70", "DEC.H0"),          // TTX4
    ("UR_SEG_71", "LONG.IO.H0"),      // THLL1
    ("UR_SEG_72", "LONG.IO.V3"),      // RVLL4
    ("UR_SEG_73", "LONG.IO.H1"),      // THLL2
    ("UR_SEG_74", "LONG.IO.H2"),      // THLL3
    ("UR_SEG_75", "LONG.IO.H3"),      // THLL4
    ("UR_SEG_76", "IO.DBUF.H0"),      // DMUX_OUTER
    ("UR_SEG_77", "IO.DBUF.H1"),      // DMUX_INNER
    ("UR_SEG_78", "IO.DOUBLE.0.N.0"), // D1L
    ("UR_SEG_79", "IO.DOUBLE.1.N.0"), // D3L
    ("UR_SEG_80", "IO.DOUBLE.2.N.0"), // D5L
    ("UR_SEG_81", "IO.DOUBLE.3.N.0"), // D7L
    // LL
    ("RDBK_TRIG", "IMUX.RDBK.TRIG"),
    ("RDBK_DATA", "OUT.RDBK.DATA"),
    ("RDBK_RIP", "OUT.BT.IOB1.I2"),
    ("MD0_I", "OUT.MD0.I"),
    ("MD1_O", "IMUX.IOB1.O1"),
    ("MD1_T", "IMUX.IOB1.IK"),
    ("MD2_I", "OUT.BT.IOB1.I1"),
    ("BUFGP_BL_I", "IMUX.BUFG.H"),
    ("BUFGS_BL_I", "IMUX.BUFG.V"),
    ("I_BUFGP_BL_I", "OUT.IOB.CLKIN.W"),
    ("I_BUFGS_BL_I", "OUT.IOB.CLKIN.S"),
    ("LL_SEG_0", "LONG.IO.V2"),        // LVLL3
    ("LL_SEG_1", "LONG.IO.H0"),        // BHLL1
    ("LL_SEG_2", "LONG.IO.V0"),        // LVLL1
    ("LL_SEG_3", "LONG.IO.V3"),        // LVLL4
    ("LL_SEG_4", "LONG.IO.H1"),        // BHLL2
    ("LL_SEG_5", "LONG.IO.V1"),        // LVLL2
    ("LL_SEG_6", "LONG.IO.H2"),        // BHLL3
    ("LL_SEG_7", "LONG.IO.H3"),        // BHLL4
    ("LL_SEG_8", "DEC.H0"),            // BTX1
    ("LL_SEG_10", "DEC.H1"),           // BTX2
    ("LL_SEG_12", "DEC.H2"),           // BTX3
    ("LL_SEG_15", "DEC.H3"),           // BTX4
    ("LL_SEG_21", "IO.DOUBLE.3.W.1"),  // D7
    ("LL_SEG_22", "IO.DOUBLE.3.S.0"),  // D8B
    ("LL_SEG_23", "IO.DOUBLE.2.W.1"),  // D5
    ("LL_SEG_24", "IO.DOUBLE.2.S.0"),  // D6B
    ("LL_SEG_25", "IO.DOUBLE.1.W.1"),  // D3
    ("LL_SEG_26", "IO.DOUBLE.1.S.0"),  // D4B
    ("LL_SEG_27", "IO.DOUBLE.0.W.1"),  // D1
    ("LL_SEG_28", "IO.DOUBLE.0.S.0"),  // D2B
    ("LL_SEG_41", "DEC.V3"),           // LTX4
    ("LL_SEG_42", "DEC.V2"),           // LTX3
    ("LL_SEG_43", "DEC.V1"),           // LTX2
    ("LL_SEG_44", "DEC.V0"),           // LTX1
    ("LL_SEG_45", "LONG.H3"),          // HLL4
    ("LL_SEG_46", "LONG.H4"),          // HLL5
    ("LL_SEG_47", "IO.DBUF.V0"),       // DMUX_OUTER
    ("LL_SEG_48", "LONG.H5"),          // HLL6
    ("LL_SEG_49", "IO.DBUF.V1"),       // DMUX_INNER
    ("LL_SEG_50", "DOUBLE.H0.0"),      // DH1
    ("LL_SEG_51", "OUT.LR.IOB1.I1.S"), // I1_2
    ("LL_SEG_52", "IO.DOUBLE.0.W.2"),  // D2
    ("LL_SEG_53", "IO.DOUBLE.3.W.2"),  // D8
    ("LL_SEG_54", "IO.DOUBLE.2.W.2"),  // D6
    ("LL_SEG_55", "SINGLE.H1"),        // H2
    // LL_SEG_56 CE_2
    ("LL_SEG_57", "DOUBLE.H0.1"),      // DH2
    ("LL_SEG_59", "IO.DOUBLE.1.W.2"),  // D4
    ("LL_SEG_60", "OUT.LR.IOB1.I2.S"), // I2_2
    ("LL_SEG_61", "SINGLE.H0"),        // H1
    // LL_SEG_63 IK_2
    // LL_SEG_64 OK_2
    ("LL_SEG_62", "SINGLE.H2"),   // H3
    ("LL_SEG_65", "SINGLE.H3"),   // H4
    ("LL_SEG_66", "SINGLE.H4"),   // H5
    ("LL_SEG_67", "SINGLE.H5"),   // H6
    ("LL_SEG_68", "SINGLE.H6"),   // H7
    ("LL_SEG_69", "SINGLE.H7"),   // H8
    ("LL_SEG_70", "DOUBLE.H1.1"), // DH3
    ("LL_SEG_71", "DOUBLE.H1.0"), // DH4
    // UL
    ("BSCAN_SEL2", "OUT.BT.IOB1.I1"),
    ("BSCAN_DRCK", "OUT.BT.IOB1.I2"),
    ("BSCAN_SEL1", "OUT.LR.IOB1.I1"),
    ("BSCAN_IDLE", "OUT.LR.IOB1.I2"),
    ("BSCAN_TDO1", "IMUX.BSCAN.TDO1"),
    ("BSCAN_TDO2", "IMUX.BSCAN.TDO2"),
    ("BUFGP_TL_I", "IMUX.BUFG.V"),
    ("BUFGS_TL_I", "IMUX.BUFG.H"),
    ("I_BUFGP_TL_I", "OUT.IOB.CLKIN.N"),
    ("I_BUFGS_TL_I", "OUT.IOB.CLKIN.W"),
    ("UL_SEG_3", "LONG.H0"),          // HLL1
    ("UL_SEG_5", "DEC.V0"),           // LTX1
    ("UL_SEG_6", "LONG.IO.V0"),       // LVLL1
    ("UL_SEG_7", "LONG.H1"),          // HLL2
    ("UL_SEG_8", "DEC.V1"),           // LTX2
    ("UL_SEG_9", "LONG.IO.V1"),       // LVLL2
    ("UL_SEG_10", "LONG.H2"),         // HLL3
    ("UL_SEG_11", "DEC.V2"),          // LTX3
    ("UL_SEG_12", "LONG.IO.V2"),      // LVLL3
    ("UL_SEG_14", "DEC.V3"),          // LTX4
    ("UL_SEG_25", "DEC.H3"),          // TTX1
    ("UL_SEG_29", "IO.DOUBLE.3.N.2"), // UL_D8
    ("UL_SEG_30", "IO.DOUBLE.3.N.1"), // UL_D7
    ("UL_SEG_31", "IO.DOUBLE.2.N.2"), // UL_D6
    ("UL_SEG_32", "IO.DOUBLE.2.N.1"), // UL_D5
    ("UL_SEG_33", "IO.DOUBLE.1.N.2"), // UL_D4
    ("UL_SEG_34", "IO.DOUBLE.1.N.1"), // UL_D3
    ("UL_SEG_35", "IO.DOUBLE.0.N.2"), // UL_D2
    ("UL_SEG_36", "IO.DOUBLE.0.N.1"), // UL_D1
    ("UL_SEG_37", "DEC.H2"),          // TTX2
    ("UL_SEG_39", "DEC.H1"),          // TTX3
    ("UL_SEG_41", "DEC.H0"),          // TTX4
    ("UL_SEG_44", "LONG.IO.H0"),      // THLL1
    ("UL_SEG_45", "LONG.IO.V3"),      // LVLL4
    ("UL_SEG_46", "LONG.IO.H1"),      // THLL2
    ("UL_SEG_47", "LONG.IO.H2"),      // THLL3
    ("UL_SEG_48", "LONG.IO.H3"),      // THLL4
];
