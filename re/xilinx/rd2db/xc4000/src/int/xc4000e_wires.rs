use prjcombine_interconnect::db::WireSlotId;
use prjcombine_xc2000::xc4000::wires;

pub fn xc4000e_wires() -> Vec<(&'static str, WireSlotId)> {
    vec![
        ("JB_K", wires::IMUX_CLB_K),
        ("JB_C1", wires::IMUX_CLB_C1),
        ("JB_C2", wires::IMUX_CLB_C2_N),
        ("JB_C3", wires::IMUX_CLB_C3_W),
        ("JB_C4", wires::IMUX_CLB_C4),
        ("JB_F1", wires::IMUX_CLB_F1),
        ("JB_F2", wires::IMUX_CLB_F2_N),
        ("JB_F3", wires::IMUX_CLB_F3_W),
        ("JB_F4", wires::IMUX_CLB_F4),
        ("JB_G1", wires::IMUX_CLB_G1),
        ("JB_G2", wires::IMUX_CLB_G2_N),
        ("JB_G3", wires::IMUX_CLB_G3_W),
        ("JB_G4", wires::IMUX_CLB_G4),
        ("JB_X", wires::OUT_CLB_X),
        ("JB_XQ", wires::OUT_CLB_XQ),
        ("JB_Y", wires::OUT_CLB_Y),
        ("JB_YQ", wires::OUT_CLB_YQ),
        ("TBUF_JB_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_JB_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_JB_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_JB_1_T", wires::IMUX_TBUF_T[1]),
        ("TIE_JB_1_O", wires::TIE_0),
        // CENTER_SEG_0 G4B
        ("CENTER_SEG_1", wires::LONG_H[0]), // HLL1
        // CENTER_SEG_3 F4B
        ("CENTER_SEG_5", wires::SINGLE_V[1]), // V2
        ("CENTER_SEG_6", wires::LONG_H[1]),   // HLL2
        // CENTER_SEG_7 C4B
        ("CENTER_SEG_9", wires::SINGLE_V[2]),   // V3
        ("CENTER_SEG_11", wires::LONG_H[2]),    // HLL3
        ("CENTER_SEG_16", wires::SINGLE_V[6]),  // V7
        ("CENTER_SEG_17", wires::SINGLE_V[3]),  // V4
        ("CENTER_SEG_18", wires::DOUBLE_V1[1]), // DV4
        ("CENTER_SEG_19", wires::SINGLE_V[4]),  // V5
        ("CENTER_SEG_20", wires::SINGLE_V[0]),  // V1
        ("CENTER_SEG_22", wires::LONG_V[5]),    // VLL6
        ("CENTER_SEG_23", wires::SINGLE_V[7]),  // V8
        ("CENTER_SEG_24", wires::LONG_V[0]),    // VLL1
        ("CENTER_SEG_27", wires::DOUBLE_V0[1]), // DV3
        ("CENTER_SEG_28", wires::SINGLE_V[5]),  // V6
        // CENTER_SEG_38 CINB
        ("CENTER_SEG_40", wires::IMUX_CLB_F3),  // F3L
        ("CENTER_SEG_30", wires::GCLK[1]),      // K2
        ("CENTER_SEG_31", wires::LONG_V[4]),    // VLL5
        ("CENTER_SEG_32", wires::LONG_V[3]),    // VLL4
        ("CENTER_SEG_33", wires::DOUBLE_V0[0]), // DV2
        ("CENTER_SEG_34", wires::DOUBLE_V1[0]), // DV1
        ("CENTER_SEG_35", wires::LONG_V[1]),    // VLL2
        ("CENTER_SEG_39", wires::GCLK[0]),      // K1
        ("CENTER_SEG_41", wires::LONG_V[2]),    // VLL3
        ("CENTER_SEG_43", wires::GCLK[3]),      // K4
        ("CENTER_SEG_44", wires::GCLK[2]),      // K3
        ("CENTER_SEG_47", wires::IMUX_CLB_C3),  // C3L
        ("CENTER_SEG_50", wires::IMUX_CLB_G3),  // G3L
        ("CENTER_SEG_52", wires::OUT_CLB_Y_E),
        // CENTER_SEG_56 CINT
        ("CENTER_SEG_57", wires::OUT_CLB_YQ_E),
        ("CENTER_SEG_61", wires::LONG_H[3]),    // HLL4
        ("CENTER_SEG_62", wires::IMUX_CLB_C2),  // C2T
        ("CENTER_SEG_63", wires::IMUX_CLB_G2),  // G2T
        ("CENTER_SEG_64", wires::LONG_H[4]),    // HLL5
        ("CENTER_SEG_65", wires::IMUX_CLB_F2),  // F2T
        ("CENTER_SEG_66", wires::LONG_H[5]),    // HLL6
        ("CENTER_SEG_67", wires::DOUBLE_H1[0]), // DH1
        ("CENTER_SEG_68", wires::OUT_CLB_X_S),
        ("CENTER_SEG_69", wires::DOUBLE_H0[0]), // DH2R
        ("CENTER_SEG_70", wires::OUT_CLB_XQ_S),
        ("CENTER_SEG_71", wires::DOUBLE_H2[0]),  // DH2
        ("CENTER_SEG_72", wires::SINGLE_H[0]),   // H1R
        ("CENTER_SEG_73", wires::SINGLE_H_E[0]), // H1
        ("CENTER_SEG_74", wires::SINGLE_H[1]),   // H2R
        ("CENTER_SEG_75", wires::SINGLE_H_E[1]), // H2
        ("CENTER_SEG_76", wires::SINGLE_H[2]),   // H3R
        ("CENTER_SEG_77", wires::SINGLE_H_E[2]), // H3
        ("CENTER_SEG_78", wires::SINGLE_H[3]),   // H4R
        ("CENTER_SEG_79", wires::SINGLE_H_E[3]), // H4
        ("CENTER_SEG_80", wires::SINGLE_H[4]),   // H5R
        ("CENTER_SEG_81", wires::SINGLE_H_E[4]), // H5
        ("CENTER_SEG_82", wires::SINGLE_H[5]),   // H6R
        ("CENTER_SEG_83", wires::SINGLE_H_E[5]), // H6
        ("CENTER_SEG_84", wires::SINGLE_H[6]),   // H7R
        ("CENTER_SEG_85", wires::SINGLE_H_E[6]), // H7
        ("CENTER_SEG_86", wires::SINGLE_H[7]),   // H8R
        ("CENTER_SEG_87", wires::SINGLE_H_E[7]), // H8
        ("CENTER_SEG_88", wires::DOUBLE_H0[1]),  // DH3R
        ("CENTER_SEG_89", wires::DOUBLE_H2[1]),  // DH3
        ("CENTER_SEG_90", wires::DOUBLE_V2[1]),  // DV3T
        ("CENTER_SEG_91", wires::SINGLE_V_S[7]), // V8T
        ("CENTER_SEG_92", wires::SINGLE_V_S[6]), // V7T
        ("CENTER_SEG_93", wires::SINGLE_V_S[5]), // V6T
        ("CENTER_SEG_94", wires::SINGLE_V_S[4]), // V5T
        ("CENTER_SEG_95", wires::SINGLE_V_S[3]), // V4T
        ("CENTER_SEG_96", wires::SINGLE_V_S[2]), // V3T
        ("CENTER_SEG_97", wires::SINGLE_V_S[1]), // V2T
        ("CENTER_SEG_98", wires::SINGLE_V_S[0]), // V1T
        ("CENTER_SEG_99", wires::DOUBLE_V2[0]),  // DV2T
        ("CENTER_SEG_100", wires::DOUBLE_H1[1]), // DH4
        // BOT
        ("PAD46_I1", wires::OUT_IO_SN_I1[0]),
        ("PAD46_I2", wires::OUT_IO_SN_I2[0]),
        ("PAD46_IK", wires::IMUX_IO_IK[0]),
        ("PAD46_OK", wires::IMUX_IO_OK[0]),
        ("PAD46_T", wires::IMUX_IO_T[0]),
        ("PAD45_I1", wires::OUT_IO_SN_I1[1]),
        ("PAD45_I2", wires::OUT_IO_SN_I2[1]),
        ("PAD45_IK", wires::IMUX_IO_IK[1]),
        ("PAD45_OK", wires::IMUX_IO_OK[1]),
        ("PAD45_T", wires::IMUX_IO_T[1]),
        ("DEC_KC_2_I", wires::IMUX_CLB_C4),
        ("TIE_KC_1_O", wires::TIE_0),
        ("BOT_SEG_1", wires::DOUBLE_IO_S0[0]),  // BDH1
        ("BOT_SEG_3", wires::DBUF_IO_H[0]),     // DMUX_OUTER
        ("BOT_SEG_4", wires::DBUF_IO_H[1]),     // DMUX_INNER
        ("BOT_SEG_5", wires::SINGLE_V[0]),      // V1
        ("BOT_SEG_6", wires::DOUBLE_IO_S2[0]),  // BDH1L
        ("BOT_SEG_7", wires::DOUBLE_V1[0]),     // DV1
        ("BOT_SEG_8", wires::DOUBLE_IO_S1[0]),  // BDH2
        ("BOT_SEG_9", wires::SINGLE_V[1]),      // V2
        ("BOT_SEG_10", wires::DOUBLE_IO_S0[1]), // BDH3
        ("BOT_SEG_11", wires::SINGLE_V[2]),     // V3
        ("BOT_SEG_12", wires::DOUBLE_V0[0]),    // DV2
        ("BOT_SEG_13", wires::DOUBLE_IO_S2[1]), // BDH3L
        ("BOT_SEG_14", wires::DOUBLE_IO_S1[1]), // BDH4
        ("BOT_SEG_15", wires::SINGLE_V[3]),     // V4
        ("BOT_SEG_16", wires::DOUBLE_IO_S0[2]), // BDH5
        ("BOT_SEG_17", wires::DOUBLE_V0[1]),    // DV3
        ("BOT_SEG_18", wires::SINGLE_V[4]),     // V5
        ("BOT_SEG_19", wires::DOUBLE_IO_S2[2]), // BDH5L
        ("BOT_SEG_20", wires::DOUBLE_IO_S1[2]), // BDH6
        ("BOT_SEG_21", wires::SINGLE_V[5]),     // V6
        ("BOT_SEG_22", wires::DOUBLE_IO_S0[3]), // BDH7
        ("BOT_SEG_23", wires::DOUBLE_V1[1]),    // DV4
        ("BOT_SEG_24", wires::SINGLE_V[6]),     // V7
        ("BOT_SEG_25", wires::DOUBLE_IO_S2[3]), // BDH7L
        ("BOT_SEG_26", wires::DOUBLE_IO_S1[3]), // BDH8
        ("BOT_SEG_27", wires::SINGLE_V[7]),     // V8
        ("BOT_SEG_28", wires::LONG_IO_H[0]),    // BHLL1
        ("BOT_SEG_29", wires::LONG_V[0]),       // VLL1
        ("BOT_SEG_30", wires::LONG_IO_H[1]),    // BHLL2
        ("BOT_SEG_31", wires::LONG_V[3]),       // VLL4
        ("BOT_SEG_32", wires::LONG_V[1]),       // VLL2
        ("BOT_SEG_33", wires::LONG_IO_H[2]),    // BHLL3
        ("BOT_SEG_34", wires::LONG_V[4]),       // VLL5
        ("BOT_SEG_35", wires::LONG_V[2]),       // VLL3
        ("BOT_SEG_36", wires::LONG_IO_H[3]),    // BHLL4
        ("BOT_SEG_37", wires::LONG_V[5]),       // VLL6
        ("BOT_SEG_39", wires::DEC_H[0]),        // TX1
        ("BOT_SEG_46", wires::DEC_H[1]),        // TX2
        ("BOT_SEG_50", wires::DEC_H[2]),        // TX3
        ("BOT_SEG_54", wires::DEC_H[3]),        // TX4
        // BOT_SEG_65 OK_2L
        // BOT_SEG_66 IK_2L
        ("BOT_SEG_68", wires::GCLK[0]),         // K1
        ("BOT_SEG_70", wires::GCLK[1]),         // K2
        ("BOT_SEG_71", wires::GCLK[2]),         // K3
        ("BOT_SEG_72", wires::GCLK[3]),         // K4
        ("BOT_SEG_73", wires::OUT_IO_SN_I1_E1), // I1_2L
        // BOT_SEG_78 CE_2L
        ("BOT_SEG_79", wires::OUT_IO_SN_I2_E1), // I2_2L
        ("BOT_SEG_80", wires::LONG_H[3]),       // HLL4
        ("BOT_SEG_81", wires::IMUX_CLB_C2),     // C2T
        ("BOT_SEG_82", wires::IMUX_CLB_G2),     // G2T
        ("BOT_SEG_83", wires::LONG_H[4]),       // HLL5
        ("BOT_SEG_84", wires::IMUX_CLB_F2),     // F2T
        ("BOT_SEG_85", wires::LONG_H[5]),       // HLL6
        ("BOT_SEG_86", wires::DOUBLE_H1[0]),    // DH1
        ("BOT_SEG_87", wires::OUT_CLB_X_S),     // FXT
        ("BOT_SEG_88", wires::DOUBLE_H0[0]),    // DH2R
        ("BOT_SEG_89", wires::OUT_CLB_XQ_S),    // FXQT
        ("BOT_SEG_90", wires::DOUBLE_H2[0]),    // DH2
        ("BOT_SEG_91", wires::SINGLE_H[0]),     // H1R
        ("BOT_SEG_92", wires::SINGLE_H_E[0]),   // H1
        ("BOT_SEG_93", wires::SINGLE_H[1]),     // H2R
        ("BOT_SEG_94", wires::SINGLE_H_E[1]),   // H2
        ("BOT_SEG_95", wires::SINGLE_H[2]),     // H3R
        ("BOT_SEG_96", wires::SINGLE_H_E[2]),   // H3
        ("BOT_SEG_97", wires::SINGLE_H[3]),     // H4R
        ("BOT_SEG_98", wires::SINGLE_H_E[3]),   // H4
        ("BOT_SEG_99", wires::SINGLE_H[4]),     // H5R
        ("BOT_SEG_100", wires::SINGLE_H_E[4]),  // H5
        ("BOT_SEG_101", wires::SINGLE_H[5]),    // H6R
        ("BOT_SEG_102", wires::SINGLE_H_E[5]),  // H6
        ("BOT_SEG_103", wires::SINGLE_H[6]),    // H7R
        ("BOT_SEG_104", wires::SINGLE_H_E[6]),  // H7
        ("BOT_SEG_105", wires::SINGLE_H[7]),    // H8R
        ("BOT_SEG_106", wires::SINGLE_H_E[7]),  // H8
        ("BOT_SEG_107", wires::DOUBLE_H0[1]),   // DH3R
        ("BOT_SEG_108", wires::DOUBLE_H2[1]),   // DH3
        ("BOT_SEG_109", wires::DOUBLE_V2[1]),   // DV3T
        ("BOT_SEG_110", wires::SINGLE_V_S[7]),  // V8T
        ("BOT_SEG_111", wires::SINGLE_V_S[6]),  // V7T
        ("BOT_SEG_112", wires::SINGLE_V_S[5]),  // V6T
        ("BOT_SEG_113", wires::SINGLE_V_S[4]),  // V5T
        ("BOT_SEG_114", wires::SINGLE_V_S[3]),  // V4T
        ("BOT_SEG_115", wires::SINGLE_V_S[2]),  // V3T
        ("BOT_SEG_116", wires::SINGLE_V_S[1]),  // V2T
        ("BOT_SEG_117", wires::SINGLE_V_S[0]),  // V1T
        ("BOT_SEG_118", wires::DOUBLE_V2[0]),   // DV2T
        ("BOT_SEG_119", wires::DOUBLE_H1[1]),   // DH4
        // BOT_SEG_120 CLOCK_3_4
        // BOT_SEG_121 COUT
        // BOT_SEG_122 CINT
        // BOTRR
        ("I_BUFGS_BR_I_BOTRR", wires::OUT_IO_CLKIN),
        ("PAD38_I1", wires::OUT_IO_SN_I1[0]),
        ("PAD38_I2", wires::OUT_IO_SN_I2[0]),
        ("PAD38_IK", wires::IMUX_IO_IK[0]),
        ("PAD38_OK", wires::IMUX_IO_OK[0]),
        ("PAD38_T", wires::IMUX_IO_T[0]),
        ("PAD37_I1", wires::OUT_IO_SN_I1[1]),
        ("PAD37_I2", wires::OUT_IO_SN_I2[1]),
        ("PAD37_IK", wires::IMUX_IO_IK[1]),
        ("PAD37_OK", wires::IMUX_IO_OK[1]),
        ("PAD37_T", wires::IMUX_IO_T[1]),
        ("DEC_KH_2_I", wires::IMUX_CLB_C4),
        ("TIE_KH_1_O", wires::TIE_0),
        ("BOTR_SEG_1", wires::DOUBLE_IO_S0[0]),  // BDH1
        ("BOTR_SEG_3", wires::DBUF_IO_H[0]),     // DMUX_OUTER
        ("BOTR_SEG_4", wires::DBUF_IO_H[1]),     // DMUX_INNER
        ("BOTR_SEG_5", wires::SINGLE_V[0]),      // V1
        ("BOTR_SEG_6", wires::DOUBLE_IO_S2[0]),  // BDH1L
        ("BOTR_SEG_7", wires::DOUBLE_V1[0]),     // DV1
        ("BOTR_SEG_8", wires::DOUBLE_IO_S1[0]),  // BDH2
        ("BOTR_SEG_9", wires::SINGLE_V[1]),      // V2
        ("BOTR_SEG_10", wires::DOUBLE_IO_S0[1]), // BDH3
        ("BOTR_SEG_11", wires::SINGLE_V[2]),     // V3
        ("BOTR_SEG_12", wires::DOUBLE_V0[0]),    // DV2
        ("BOTR_SEG_13", wires::DOUBLE_IO_S2[1]), // BDH3L
        ("BOTR_SEG_14", wires::DOUBLE_IO_S1[1]), // BDH4
        ("BOTR_SEG_15", wires::SINGLE_V[3]),     // V4
        ("BOTR_SEG_16", wires::DOUBLE_IO_S0[2]), // BDH5
        ("BOTR_SEG_17", wires::DOUBLE_V0[1]),    // DV3
        ("BOTR_SEG_18", wires::SINGLE_V[4]),     // V5
        ("BOTR_SEG_19", wires::DOUBLE_IO_S2[2]), // BDH5L
        ("BOTR_SEG_20", wires::DOUBLE_IO_S1[2]), // BDH6
        ("BOTR_SEG_21", wires::SINGLE_V[5]),     // V6
        ("BOTR_SEG_22", wires::DOUBLE_IO_S0[3]), // BDH7
        ("BOTR_SEG_23", wires::DOUBLE_V1[1]),    // DV4
        ("BOTR_SEG_24", wires::SINGLE_V[6]),     // V7
        ("BOTR_SEG_25", wires::DOUBLE_IO_S2[3]), // BDH7L
        ("BOTR_SEG_26", wires::DOUBLE_IO_S1[3]), // BDH8
        ("BOTR_SEG_27", wires::SINGLE_V[7]),     // V8
        ("BOTR_SEG_28", wires::LONG_IO_H[0]),    // BHLL1
        ("BOTR_SEG_29", wires::LONG_V[0]),       // VLL1
        ("BOTR_SEG_30", wires::LONG_IO_H[1]),    // BHLL2
        ("BOTR_SEG_31", wires::LONG_V[3]),       // VLL4
        ("BOTR_SEG_32", wires::LONG_V[1]),       // VLL2
        ("BOTR_SEG_33", wires::LONG_IO_H[2]),    // BHLL3
        ("BOTR_SEG_34", wires::LONG_V[4]),       // VLL5
        ("BOTR_SEG_35", wires::LONG_V[2]),       // VLL3
        ("BOTR_SEG_36", wires::LONG_IO_H[3]),    // BHLL4
        ("BOTR_SEG_37", wires::LONG_V[5]),       // VLL6
        ("BOTR_SEG_39", wires::DEC_H[0]),        // TX1
        ("BOTR_SEG_46", wires::DEC_H[1]),        // TX2
        ("BOTR_SEG_50", wires::DEC_H[2]),        // TX3
        ("BOTR_SEG_54", wires::DEC_H[3]),        // TX4
        // BOTR_SEG_65 OK_2L
        // BOTR_SEG_66 IK_2L
        ("BOTR_SEG_68", wires::GCLK[0]),         // K1
        ("BOTR_SEG_70", wires::GCLK[1]),         // K2
        ("BOTR_SEG_71", wires::GCLK[2]),         // K3
        ("BOTR_SEG_72", wires::GCLK[3]),         // K4
        ("BOTR_SEG_73", wires::OUT_IO_SN_I1_E1), // I1_2L
        // BOTR_SEG_78 CE_2L
        ("BOTR_SEG_79", wires::OUT_IO_SN_I2_E1), // I2_2L
        ("BOTR_SEG_80", wires::LONG_H[3]),       // HLL4
        ("BOTR_SEG_81", wires::IMUX_CLB_C2),     // C2T
        ("BOTR_SEG_82", wires::IMUX_CLB_G2),     // G2T
        ("BOTR_SEG_83", wires::LONG_H[4]),       // HLL5
        ("BOTR_SEG_84", wires::IMUX_CLB_F2),     // F2T
        ("BOTR_SEG_85", wires::LONG_H[5]),       // HLL6
        ("BOTR_SEG_86", wires::DOUBLE_H1[0]),    // DH1
        ("BOTR_SEG_87", wires::OUT_CLB_X_S),     // FXT
        ("BOTR_SEG_88", wires::DOUBLE_H0[0]),    // DH2R
        ("BOTR_SEG_89", wires::OUT_CLB_XQ_S),    // FXQT
        ("BOTR_SEG_90", wires::DOUBLE_H2[0]),    // DH2
        ("BOTR_SEG_91", wires::SINGLE_H[0]),     // H1R
        ("BOTR_SEG_92", wires::SINGLE_H_E[0]),   // H1
        ("BOTR_SEG_93", wires::SINGLE_H[1]),     // H2R
        ("BOTR_SEG_94", wires::SINGLE_H_E[1]),   // H2
        ("BOTR_SEG_95", wires::SINGLE_H[2]),     // H3R
        ("BOTR_SEG_96", wires::SINGLE_H_E[2]),   // H3
        ("BOTR_SEG_97", wires::SINGLE_H[3]),     // H4R
        ("BOTR_SEG_98", wires::SINGLE_H_E[3]),   // H4
        ("BOTR_SEG_99", wires::SINGLE_H[4]),     // H5R
        ("BOTR_SEG_100", wires::SINGLE_H_E[4]),  // H5
        ("BOTR_SEG_101", wires::SINGLE_H[5]),    // H6R
        ("BOTR_SEG_102", wires::SINGLE_H_E[5]),  // H6
        ("BOTR_SEG_103", wires::SINGLE_H[6]),    // H7R
        ("BOTR_SEG_104", wires::SINGLE_H_E[6]),  // H7
        ("BOTR_SEG_105", wires::SINGLE_H[7]),    // H8R
        ("BOTR_SEG_106", wires::SINGLE_H_E[7]),  // H8
        ("BOTR_SEG_107", wires::DOUBLE_H0[1]),   // DH3R
        ("BOTR_SEG_108", wires::DOUBLE_H2[1]),   // DH3
        ("BOTR_SEG_109", wires::DOUBLE_V2[1]),   // DV3T
        ("BOTR_SEG_110", wires::SINGLE_V_S[7]),  // V8T
        ("BOTR_SEG_111", wires::SINGLE_V_S[6]),  // V7T
        ("BOTR_SEG_112", wires::SINGLE_V_S[5]),  // V6T
        ("BOTR_SEG_113", wires::SINGLE_V_S[4]),  // V5T
        ("BOTR_SEG_114", wires::SINGLE_V_S[3]),  // V4T
        ("BOTR_SEG_115", wires::SINGLE_V_S[2]),  // V3T
        ("BOTR_SEG_116", wires::SINGLE_V_S[1]),  // V2T
        ("BOTR_SEG_117", wires::SINGLE_V_S[0]),  // V1T
        ("BOTR_SEG_118", wires::DOUBLE_V2[0]),   // DV2T
        ("BOTR_SEG_119", wires::DOUBLE_H1[1]),   // DH4
        // BOTR_SEG_120 CLOCK_3_4
        // BOTR_SEG_121 COUT
        // BOTR_SEG_122 CINT
        // BOTS
        ("PAD44_I1", wires::OUT_IO_SN_I1[0]),
        ("PAD44_I2", wires::OUT_IO_SN_I2[0]),
        ("PAD44_IK", wires::IMUX_IO_IK[0]),
        ("PAD44_OK", wires::IMUX_IO_OK[0]),
        ("PAD44_T", wires::IMUX_IO_T[0]),
        ("PAD43_I1", wires::OUT_IO_SN_I1[1]),
        ("PAD43_I2", wires::OUT_IO_SN_I2[1]),
        ("PAD43_IK", wires::IMUX_IO_IK[1]),
        ("PAD43_OK", wires::IMUX_IO_OK[1]),
        ("PAD43_T", wires::IMUX_IO_T[1]),
        ("DEC_KD_2_I", wires::IMUX_CLB_C4),
        ("TIE_KD_1_O", wires::TIE_0),
        ("BOTS_SEG_1", wires::DOUBLE_IO_S0[0]),  // BDH1
        ("BOTS_SEG_3", wires::DBUF_IO_H[0]),     // DMUX_OUTER
        ("BOTS_SEG_4", wires::DBUF_IO_H[1]),     // DMUX_INNER
        ("BOTS_SEG_5", wires::SINGLE_V[1]),      // V2
        ("BOTS_SEG_6", wires::DOUBLE_IO_S2[0]),  // BDH1L
        ("BOTS_SEG_7", wires::DOUBLE_V1[0]),     // DV1
        ("BOTS_SEG_8", wires::DOUBLE_IO_S1[0]),  // BDH2
        ("BOTS_SEG_9", wires::SINGLE_V[0]),      // V1
        ("BOTS_SEG_10", wires::DOUBLE_IO_S0[1]), // BDH3
        ("BOTS_SEG_11", wires::SINGLE_V[3]),     // V4
        ("BOTS_SEG_12", wires::DOUBLE_V0[0]),    // DV2
        ("BOTS_SEG_13", wires::DOUBLE_IO_S2[1]), // BDH3L
        ("BOTS_SEG_14", wires::DOUBLE_IO_S1[1]), // BDH4
        ("BOTS_SEG_15", wires::SINGLE_V[2]),     // V3
        ("BOTS_SEG_16", wires::DOUBLE_IO_S0[2]), // BDH5
        ("BOTS_SEG_17", wires::DOUBLE_V0[1]),    // DV3
        ("BOTS_SEG_18", wires::SINGLE_V[5]),     // V6
        ("BOTS_SEG_19", wires::DOUBLE_IO_S2[2]), // BDH5L
        ("BOTS_SEG_20", wires::DOUBLE_IO_S1[2]), // BDH6
        ("BOTS_SEG_21", wires::SINGLE_V[4]),     // V5
        ("BOTS_SEG_22", wires::DOUBLE_IO_S0[3]), // BDH7
        ("BOTS_SEG_23", wires::DOUBLE_V1[1]),    // DV4
        ("BOTS_SEG_24", wires::SINGLE_V[7]),     // V8
        ("BOTS_SEG_25", wires::DOUBLE_IO_S2[3]), // BDH7L
        ("BOTS_SEG_26", wires::DOUBLE_IO_S1[3]), // BDH8
        ("BOTS_SEG_27", wires::SINGLE_V[6]),     // V7
        ("BOTS_SEG_28", wires::LONG_IO_H[0]),    // BHLL1
        ("BOTS_SEG_29", wires::LONG_V[0]),       // VLL1
        ("BOTS_SEG_30", wires::LONG_IO_H[1]),    // BHLL2
        ("BOTS_SEG_31", wires::LONG_V[3]),       // VLL4
        ("BOTS_SEG_32", wires::LONG_V[1]),       // VLL2
        ("BOTS_SEG_33", wires::LONG_IO_H[2]),    // BHLL3
        ("BOTS_SEG_34", wires::LONG_V[4]),       // VLL5
        ("BOTS_SEG_35", wires::LONG_V[2]),       // VLL3
        ("BOTS_SEG_36", wires::LONG_IO_H[3]),    // BHLL4
        ("BOTS_SEG_37", wires::LONG_V[5]),       // VLL6
        ("BOTS_SEG_39", wires::DEC_H[0]),        // TX1
        ("BOTS_SEG_46", wires::DEC_H[1]),        // TX2
        ("BOTS_SEG_50", wires::DEC_H[2]),        // TX3
        ("BOTS_SEG_54", wires::DEC_H[3]),        // TX4
        // BOTS_SEG_65 OK_2L
        // BOTS_SEG_66 IK_2L
        ("BOTS_SEG_68", wires::GCLK[0]),         // K1
        ("BOTS_SEG_70", wires::GCLK[1]),         // K2
        ("BOTS_SEG_71", wires::GCLK[2]),         // K3
        ("BOTS_SEG_72", wires::GCLK[3]),         // K4
        ("BOTS_SEG_73", wires::OUT_IO_SN_I1_E1), // I1_2L
        // BOTS_SEG_78 CE_2L
        ("BOTS_SEG_79", wires::OUT_IO_SN_I2_E1), // I2_2L
        ("BOTS_SEG_80", wires::LONG_H[3]),       // HLL4
        ("BOTS_SEG_81", wires::IMUX_CLB_C2),     // C2T
        ("BOTS_SEG_82", wires::IMUX_CLB_G2),     // G2T
        ("BOTS_SEG_83", wires::LONG_H[4]),       // HLL5
        ("BOTS_SEG_84", wires::IMUX_CLB_F2),     // F2T
        ("BOTS_SEG_85", wires::LONG_H[5]),       // HLL6
        ("BOTS_SEG_86", wires::DOUBLE_H1[0]),    // DH1
        ("BOTS_SEG_87", wires::OUT_CLB_X_S),     // FXT
        ("BOTS_SEG_88", wires::DOUBLE_H0[0]),    // DH2R
        ("BOTS_SEG_89", wires::OUT_CLB_XQ_S),    // FXQT
        ("BOTS_SEG_90", wires::DOUBLE_H2[0]),    // DH2
        ("BOTS_SEG_91", wires::SINGLE_H[0]),     // H1R
        ("BOTS_SEG_92", wires::SINGLE_H_E[0]),   // H1
        ("BOTS_SEG_93", wires::SINGLE_H[1]),     // H2R
        ("BOTS_SEG_94", wires::SINGLE_H_E[1]),   // H2
        ("BOTS_SEG_95", wires::SINGLE_H[2]),     // H3R
        ("BOTS_SEG_96", wires::SINGLE_H_E[2]),   // H3
        ("BOTS_SEG_97", wires::SINGLE_H[3]),     // H4R
        ("BOTS_SEG_98", wires::SINGLE_H_E[3]),   // H4
        ("BOTS_SEG_99", wires::SINGLE_H[4]),     // H5R
        ("BOTS_SEG_100", wires::SINGLE_H_E[4]),  // H5
        ("BOTS_SEG_101", wires::SINGLE_H[5]),    // H6R
        ("BOTS_SEG_102", wires::SINGLE_H_E[5]),  // H6
        ("BOTS_SEG_103", wires::SINGLE_H[6]),    // H7R
        ("BOTS_SEG_104", wires::SINGLE_H_E[6]),  // H7
        ("BOTS_SEG_105", wires::SINGLE_H[7]),    // H8R
        ("BOTS_SEG_106", wires::SINGLE_H_E[7]),  // H8
        ("BOTS_SEG_107", wires::DOUBLE_H0[1]),   // DH3R
        ("BOTS_SEG_108", wires::DOUBLE_H2[1]),   // DH3
        ("BOTS_SEG_109", wires::DOUBLE_V2[1]),   // DV3T
        ("BOTS_SEG_110", wires::SINGLE_V_S[7]),  // V8T
        ("BOTS_SEG_111", wires::SINGLE_V_S[6]),  // V7T
        ("BOTS_SEG_112", wires::SINGLE_V_S[5]),  // V6T
        ("BOTS_SEG_113", wires::SINGLE_V_S[4]),  // V5T
        ("BOTS_SEG_114", wires::SINGLE_V_S[3]),  // V4T
        ("BOTS_SEG_115", wires::SINGLE_V_S[2]),  // V3T
        ("BOTS_SEG_116", wires::SINGLE_V_S[1]),  // V2T
        ("BOTS_SEG_117", wires::SINGLE_V_S[0]),  // V1T
        ("BOTS_SEG_118", wires::DOUBLE_V2[0]),   // DV2T
        ("BOTS_SEG_119", wires::DOUBLE_H1[1]),   // DH4
        // BOTS_SEG_120 CLOCK_3_4
        // BOTS_SEG_121 COUT
        // BOTS_SEG_122 CINT

        // BOTSL
        ("I_BUFGP_BL_I_BOTSL", wires::OUT_IO_CLKIN),
        ("PAD48_I1", wires::OUT_IO_SN_I1[0]),
        ("PAD48_I2", wires::OUT_IO_SN_I2[0]),
        ("PAD48_IK", wires::IMUX_IO_IK[0]),
        ("PAD48_OK", wires::IMUX_IO_OK[0]),
        ("PAD48_T", wires::IMUX_IO_T[0]),
        ("PAD47_I1", wires::OUT_IO_SN_I1[1]),
        ("PAD47_I2", wires::OUT_IO_SN_I2[1]),
        ("PAD47_IK", wires::IMUX_IO_IK[1]),
        ("PAD47_OK", wires::IMUX_IO_OK[1]),
        ("PAD47_T", wires::IMUX_IO_T[1]),
        ("DEC_KB_2_I", wires::IMUX_CLB_C4),
        ("TIE_KB_1_O", wires::TIE_0),
        ("BOTSL_SEG_1", wires::DOUBLE_IO_S0[0]),  // BDH1
        ("BOTSL_SEG_3", wires::DBUF_IO_H[0]),     // DMUX_OUTER
        ("BOTSL_SEG_4", wires::DBUF_IO_H[1]),     // DMUX_INNER
        ("BOTSL_SEG_5", wires::SINGLE_V[1]),      // V2
        ("BOTSL_SEG_6", wires::DOUBLE_IO_S2[0]),  // BDH1L
        ("BOTSL_SEG_7", wires::DOUBLE_V1[0]),     // DV1
        ("BOTSL_SEG_8", wires::DOUBLE_IO_S1[0]),  // BDH2
        ("BOTSL_SEG_9", wires::SINGLE_V[0]),      // V1
        ("BOTSL_SEG_10", wires::DOUBLE_IO_S0[1]), // BDH3
        ("BOTSL_SEG_11", wires::SINGLE_V[3]),     // V4
        ("BOTSL_SEG_12", wires::DOUBLE_V0[0]),    // DV2
        ("BOTSL_SEG_13", wires::DOUBLE_IO_S2[1]), // BDH3L
        ("BOTSL_SEG_14", wires::DOUBLE_IO_S1[1]), // BDH4
        ("BOTSL_SEG_15", wires::SINGLE_V[2]),     // V3
        ("BOTSL_SEG_16", wires::DOUBLE_IO_S0[2]), // BDH5
        ("BOTSL_SEG_17", wires::DOUBLE_V0[1]),    // DV3
        ("BOTSL_SEG_18", wires::SINGLE_V[5]),     // V6
        ("BOTSL_SEG_19", wires::DOUBLE_IO_S2[2]), // BDH5L
        ("BOTSL_SEG_20", wires::DOUBLE_IO_S1[2]), // BDH6
        ("BOTSL_SEG_21", wires::SINGLE_V[4]),     // V5
        ("BOTSL_SEG_22", wires::DOUBLE_IO_S0[3]), // BDH7
        ("BOTSL_SEG_23", wires::DOUBLE_V1[1]),    // DV4
        ("BOTSL_SEG_24", wires::SINGLE_V[7]),     // V8
        ("BOTSL_SEG_25", wires::DOUBLE_IO_S2[3]), // BDH7L
        ("BOTSL_SEG_26", wires::DOUBLE_IO_S1[3]), // BDH8
        ("BOTSL_SEG_27", wires::SINGLE_V[6]),     // V7
        ("BOTSL_SEG_28", wires::LONG_IO_H[0]),    // BHLL1
        ("BOTSL_SEG_29", wires::LONG_V[0]),       // VLL1
        ("BOTSL_SEG_30", wires::LONG_IO_H[1]),    // BHLL2
        ("BOTSL_SEG_31", wires::LONG_V[3]),       // VLL4
        ("BOTSL_SEG_32", wires::LONG_V[1]),       // VLL2
        ("BOTSL_SEG_33", wires::LONG_IO_H[2]),    // BHLL3
        ("BOTSL_SEG_34", wires::LONG_V[4]),       // VLL5
        ("BOTSL_SEG_35", wires::LONG_V[2]),       // VLL3
        ("BOTSL_SEG_36", wires::LONG_IO_H[3]),    // BHLL4
        ("BOTSL_SEG_37", wires::LONG_V[5]),       // VLL6
        ("BOTSL_SEG_39", wires::DEC_H[0]),        // TX1
        ("BOTSL_SEG_46", wires::DEC_H[1]),        // TX2
        ("BOTSL_SEG_50", wires::DEC_H[2]),        // TX3
        ("BOTSL_SEG_54", wires::DEC_H[3]),        // TX4
        // BOTSL_SEG_65 OK_2L
        ("BOTSL_SEG_67", wires::GCLK[0]),         // K1
        ("BOTSL_SEG_69", wires::GCLK[1]),         // K2
        ("BOTSL_SEG_70", wires::GCLK[2]),         // K3
        ("BOTSL_SEG_71", wires::GCLK[3]),         // K4
        ("BOTSL_SEG_72", wires::OUT_IO_SN_I1_E1), // I1_2L
        // BOTSL_SEG_76 CE_2L
        ("BOTSL_SEG_77", wires::OUT_IO_SN_I2_E1), // I2_2L
        ("BOTSL_SEG_78", wires::LONG_H[3]),       // HLL4
        ("BOTSL_SEG_79", wires::IMUX_CLB_C2),     // C2T
        ("BOTSL_SEG_80", wires::IMUX_CLB_G2),     // G2T
        ("BOTSL_SEG_81", wires::LONG_H[4]),       // HLL5
        ("BOTSL_SEG_82", wires::IMUX_CLB_F2),     // F2T
        ("BOTSL_SEG_83", wires::LONG_H[5]),       // HLL6
        ("BOTSL_SEG_84", wires::DOUBLE_H1[0]),    // DH1
        ("BOTSL_SEG_85", wires::OUT_CLB_X_S),     // FXT
        ("BOTSL_SEG_86", wires::DOUBLE_H0[0]),    // DH2R
        ("BOTSL_SEG_87", wires::OUT_CLB_XQ_S),    // FXQT
        ("BOTSL_SEG_88", wires::DOUBLE_H2[0]),    // DH2
        ("BOTSL_SEG_89", wires::SINGLE_H[0]),     // H1R
        ("BOTSL_SEG_90", wires::SINGLE_H_E[0]),   // H1
        ("BOTSL_SEG_91", wires::SINGLE_H[1]),     // H2R
        ("BOTSL_SEG_92", wires::SINGLE_H_E[1]),   // H2
        ("BOTSL_SEG_93", wires::SINGLE_H[2]),     // H3R
        ("BOTSL_SEG_94", wires::SINGLE_H_E[2]),   // H3
        ("BOTSL_SEG_95", wires::SINGLE_H[3]),     // H4R
        ("BOTSL_SEG_96", wires::SINGLE_H_E[3]),   // H4
        ("BOTSL_SEG_97", wires::SINGLE_H[4]),     // H5R
        ("BOTSL_SEG_98", wires::SINGLE_H_E[4]),   // H5
        ("BOTSL_SEG_99", wires::SINGLE_H[5]),     // H6R
        ("BOTSL_SEG_100", wires::SINGLE_H_E[5]),  // H6
        ("BOTSL_SEG_101", wires::SINGLE_H[6]),    // H7R
        ("BOTSL_SEG_102", wires::SINGLE_H_E[6]),  // H7
        ("BOTSL_SEG_103", wires::SINGLE_H[7]),    // H8R
        ("BOTSL_SEG_104", wires::SINGLE_H_E[7]),  // H8
        ("BOTSL_SEG_105", wires::DOUBLE_H0[1]),   // DH3R
        ("BOTSL_SEG_106", wires::DOUBLE_H2[1]),   // DH3
        ("BOTSL_SEG_107", wires::DOUBLE_V2[1]),   // DV3T
        ("BOTSL_SEG_108", wires::SINGLE_V_S[7]),  // V8T
        ("BOTSL_SEG_109", wires::SINGLE_V_S[6]),  // V7T
        ("BOTSL_SEG_110", wires::SINGLE_V_S[5]),  // V6T
        ("BOTSL_SEG_111", wires::SINGLE_V_S[4]),  // V5T
        ("BOTSL_SEG_112", wires::SINGLE_V_S[3]),  // V4T
        ("BOTSL_SEG_113", wires::SINGLE_V_S[2]),  // V3T
        ("BOTSL_SEG_114", wires::SINGLE_V_S[1]),  // V2T
        ("BOTSL_SEG_115", wires::SINGLE_V_S[0]),  // V1T
        ("BOTSL_SEG_116", wires::DOUBLE_V2[0]),   // DV2T
        ("BOTSL_SEG_117", wires::DOUBLE_H1[1]),   // DH4
        // BOTSL_SEG_118 CLOCK_3_4
        // BOTSL_SEG_119 CIN
        // BOTSL_SEG_120 CINT
        // TOP
        ("PAD3_I1", wires::OUT_IO_SN_I1[0]),
        ("PAD3_I2", wires::OUT_IO_SN_I2[0]),
        ("PAD3_IK", wires::IMUX_IO_IK[0]),
        ("PAD3_OK", wires::IMUX_IO_OK[0]),
        ("PAD3_T", wires::IMUX_IO_T[0]),
        ("PAD4_I1", wires::OUT_IO_SN_I1[1]),
        ("PAD4_I2", wires::OUT_IO_SN_I2[1]),
        ("PAD4_IK", wires::IMUX_IO_IK[1]),
        ("PAD4_OK", wires::IMUX_IO_OK[1]),
        ("PAD4_T", wires::IMUX_IO_T[1]),
        ("DEC_AC_2_I", wires::IMUX_CLB_C2_N),
        // TOP_SEG_0 G4B
        ("TOP_SEG_1", wires::LONG_H[0]), // HLL1
        // TOP_SEG_3 F4B
        ("TOP_SEG_5", wires::SINGLE_V[1]), // V2
        ("TOP_SEG_6", wires::LONG_H[1]),   // HLL2
        // TOP_SEG_7 C4B
        ("TOP_SEG_9", wires::SINGLE_V[2]),      // V3
        ("TOP_SEG_10", wires::LONG_H[2]),       // HLL3
        ("TOP_SEG_11", wires::SINGLE_V[3]),     // V4
        ("TOP_SEG_12", wires::DOUBLE_V1[1]),    // DV4
        ("TOP_SEG_13", wires::OUT_IO_SN_I2_E1), // I2_2L
        ("TOP_SEG_14", wires::SINGLE_V[4]),     // V5
        ("TOP_SEG_15", wires::SINGLE_V[0]),     // V1
        ("TOP_SEG_16", wires::LONG_V[2]),       // VLL3
        ("TOP_SEG_17", wires::LONG_V[1]),       // VLL2
        ("TOP_SEG_18", wires::LONG_V[0]),       // VLL1
        ("TOP_SEG_19", wires::LONG_V[5]),       // VLL6
        ("TOP_SEG_21", wires::LONG_V[4]),       // VLL5
        ("TOP_SEG_22", wires::LONG_V[3]),       // VLL4
        ("TOP_SEG_23", wires::DOUBLE_V0[1]),    // DV3
        ("TOP_SEG_24", wires::SINGLE_V[5]),     // V6
        ("TOP_SEG_26", wires::DOUBLE_V1[0]),    // DV1
        ("TOP_SEG_27", wires::DOUBLE_V0[0]),    // DV2
        // TOP_SEG_31 CE_2L
        ("TOP_SEG_40", wires::GCLK[3]),         // K4
        ("TOP_SEG_41", wires::SINGLE_V[7]),     // V8
        ("TOP_SEG_42", wires::OUT_IO_SN_I1_E1), // I1_2L
        ("TOP_SEG_43", wires::GCLK[2]),         // K3
        ("TOP_SEG_44", wires::GCLK[1]),         // K2
        ("TOP_SEG_45", wires::GCLK[0]),         // K1
        ("TOP_SEG_46", wires::SINGLE_V[6]),     // V7
        // TOP_SEG_48 IK_2L
        // TOP_SEG_49 OK_2L
        ("TOP_SEG_63", wires::DEC_H[3]),        // TTX1
        ("TOP_SEG_64", wires::DEC_H[2]),        // TTX2
        ("TOP_SEG_65", wires::DEC_H[1]),        // TTX3
        ("TOP_SEG_66", wires::DEC_H[0]),        // TTX4
        ("TOP_SEG_67", wires::LONG_IO_H[0]),    // THLL1
        ("TOP_SEG_68", wires::LONG_IO_H[1]),    // THLL2
        ("TOP_SEG_69", wires::LONG_IO_H[2]),    // THLL3
        ("TOP_SEG_70", wires::LONG_IO_H[3]),    // THLL4
        ("TOP_SEG_71", wires::DOUBLE_IO_N2[0]), // DH1
        ("TOP_SEG_72", wires::DBUF_IO_H[0]),    // DMUX_OUTER
        ("TOP_SEG_73", wires::DBUF_IO_H[1]),    // DMUX_INNER
        ("TOP_SEG_74", wires::DOUBLE_IO_N0[0]), // DH1L
        ("TOP_SEG_75", wires::DOUBLE_IO_N1[0]), // DH2
        ("TOP_SEG_76", wires::DOUBLE_IO_N2[1]), // DH3
        ("TOP_SEG_77", wires::DOUBLE_IO_N0[1]), // DH3L
        ("TOP_SEG_78", wires::DOUBLE_IO_N1[1]), // DH4
        ("TOP_SEG_79", wires::DOUBLE_IO_N2[2]), // DH5
        ("TOP_SEG_80", wires::DOUBLE_IO_N0[2]), // DH5L
        ("TOP_SEG_81", wires::DOUBLE_IO_N1[2]), // DH6
        ("TOP_SEG_82", wires::DOUBLE_IO_N2[3]), // DH7
        ("TOP_SEG_83", wires::DOUBLE_IO_N0[3]), // DH7L
        ("TOP_SEG_84", wires::DOUBLE_IO_N1[3]), // DH8
        // TOP_SEG_85 CINB
        // TOP_SEG_86 CIN
        // TOP_SEG_87 CLOCK_7_8
        // TOPRR
        ("I_BUFGP_TR_I_TOPRR", wires::OUT_IO_CLKIN),
        ("PAD11_I1", wires::OUT_IO_SN_I1[0]),
        ("PAD11_I2", wires::OUT_IO_SN_I2[0]),
        ("PAD11_IK", wires::IMUX_IO_IK[0]),
        ("PAD11_OK", wires::IMUX_IO_OK[0]),
        ("PAD11_T", wires::IMUX_IO_T[0]),
        ("PAD12_I1", wires::OUT_IO_SN_I1[1]),
        ("PAD12_I2", wires::OUT_IO_SN_I2[1]),
        ("PAD12_IK", wires::IMUX_IO_IK[1]),
        ("PAD12_OK", wires::IMUX_IO_OK[1]),
        ("PAD12_T", wires::IMUX_IO_T[1]),
        ("DEC_AH_2_I", wires::IMUX_CLB_C2_N),
        // TOPR_SEG_0 G4B
        ("TOPR_SEG_1", wires::LONG_H[0]), // HLL1
        // TOPR_SEG_3 F4B
        ("TOPR_SEG_5", wires::SINGLE_V[1]), // V2
        ("TOPR_SEG_6", wires::LONG_H[1]),   // HLL2
        // TOPR_SEG_7 C4B
        ("TOPR_SEG_9", wires::SINGLE_V[2]),      // V3
        ("TOPR_SEG_10", wires::LONG_H[2]),       // HLL3
        ("TOPR_SEG_11", wires::SINGLE_V[3]),     // V4
        ("TOPR_SEG_12", wires::DOUBLE_V1[1]),    // DV4
        ("TOPR_SEG_13", wires::OUT_IO_SN_I2_E1), // I2_2L
        ("TOPR_SEG_14", wires::SINGLE_V[4]),     // V5
        ("TOPR_SEG_15", wires::SINGLE_V[0]),     // V1
        ("TOPR_SEG_16", wires::LONG_V[2]),       // VLL3
        ("TOPR_SEG_17", wires::LONG_V[1]),       // VLL2
        ("TOPR_SEG_18", wires::LONG_V[0]),       // VLL1
        ("TOPR_SEG_19", wires::LONG_V[5]),       // VLL6
        ("TOPR_SEG_21", wires::LONG_V[4]),       // VLL5
        ("TOPR_SEG_22", wires::LONG_V[3]),       // VLL4
        ("TOPR_SEG_23", wires::DOUBLE_V0[1]),    // DV3
        ("TOPR_SEG_24", wires::SINGLE_V[5]),     // V6
        ("TOPR_SEG_26", wires::DOUBLE_V1[0]),    // DV1
        ("TOPR_SEG_27", wires::DOUBLE_V0[0]),    // DV2
        // TOPR_SEG_31 CE_2L
        ("TOPR_SEG_40", wires::GCLK[3]),         // K4
        ("TOPR_SEG_41", wires::SINGLE_V[7]),     // V8
        ("TOPR_SEG_42", wires::OUT_IO_SN_I1_E1), // I1_2L
        ("TOPR_SEG_43", wires::GCLK[2]),         // K3
        ("TOPR_SEG_44", wires::GCLK[1]),         // K2
        ("TOPR_SEG_45", wires::GCLK[0]),         // K1
        ("TOPR_SEG_46", wires::SINGLE_V[6]),     // V7
        // TOPR_SEG_48 IK_2L
        // TOPR_SEG_49 OK_2L
        ("TOPR_SEG_63", wires::DEC_H[3]),        // TTX1
        ("TOPR_SEG_64", wires::DEC_H[2]),        // TTX2
        ("TOPR_SEG_65", wires::DEC_H[1]),        // TTX3
        ("TOPR_SEG_66", wires::DEC_H[0]),        // TTX4
        ("TOPR_SEG_67", wires::LONG_IO_H[0]),    // THLL1
        ("TOPR_SEG_68", wires::LONG_IO_H[1]),    // THLL2
        ("TOPR_SEG_69", wires::LONG_IO_H[2]),    // THLL3
        ("TOPR_SEG_70", wires::LONG_IO_H[3]),    // THLL4
        ("TOPR_SEG_71", wires::DOUBLE_IO_N2[0]), // DH1
        ("TOPR_SEG_72", wires::DBUF_IO_H[0]),    // DMUX_OUTER
        ("TOPR_SEG_73", wires::DBUF_IO_H[1]),    // DMUX_INNER
        ("TOPR_SEG_74", wires::DOUBLE_IO_N0[0]), // DH1L
        ("TOPR_SEG_75", wires::DOUBLE_IO_N1[0]), // DH2
        ("TOPR_SEG_76", wires::DOUBLE_IO_N2[1]), // DH3
        ("TOPR_SEG_77", wires::DOUBLE_IO_N0[1]), // DH3L
        ("TOPR_SEG_78", wires::DOUBLE_IO_N1[1]), // DH4
        ("TOPR_SEG_79", wires::DOUBLE_IO_N2[2]), // DH5
        ("TOPR_SEG_80", wires::DOUBLE_IO_N0[2]), // DH5L
        ("TOPR_SEG_81", wires::DOUBLE_IO_N1[2]), // DH6
        ("TOPR_SEG_82", wires::DOUBLE_IO_N2[3]), // DH7
        ("TOPR_SEG_83", wires::DOUBLE_IO_N0[3]), // DH7L
        ("TOPR_SEG_84", wires::DOUBLE_IO_N1[3]), // DH8
        // TOPR_SEG_85 CINB
        // TOPR_SEG_86 CIN
        // TOPR_SEG_87 CLOCK_7_8
        // TOPS
        ("PAD5_I1", wires::OUT_IO_SN_I1[0]),
        ("PAD5_I2", wires::OUT_IO_SN_I2[0]),
        ("PAD5_IK", wires::IMUX_IO_IK[0]),
        ("PAD5_OK", wires::IMUX_IO_OK[0]),
        ("PAD5_T", wires::IMUX_IO_T[0]),
        ("PAD6_I1", wires::OUT_IO_SN_I1[1]),
        ("PAD6_I2", wires::OUT_IO_SN_I2[1]),
        ("PAD6_IK", wires::IMUX_IO_IK[1]),
        ("PAD6_OK", wires::IMUX_IO_OK[1]),
        ("PAD6_T", wires::IMUX_IO_T[1]),
        ("DEC_AD_2_I", wires::IMUX_CLB_C2_N),
        // TOPS_SEG_0 G4B
        ("TOPS_SEG_1", wires::LONG_H[0]), // HLL1
        // TOPS_SEG_3 F4B
        ("TOPS_SEG_5", wires::SINGLE_V[1]), // V2
        ("TOPS_SEG_6", wires::LONG_H[1]),   // HLL2
        // TOPS_SEG_7 C4B
        ("TOPS_SEG_9", wires::SINGLE_V[2]),      // V3
        ("TOPS_SEG_10", wires::LONG_H[2]),       // HLL3
        ("TOPS_SEG_11", wires::SINGLE_V[3]),     // V4
        ("TOPS_SEG_12", wires::DOUBLE_V1[1]),    // DV4
        ("TOPS_SEG_13", wires::OUT_IO_SN_I2_E1), // I2_2L
        ("TOPS_SEG_14", wires::SINGLE_V[4]),     // V5
        ("TOPS_SEG_15", wires::SINGLE_V[0]),     // V1
        ("TOPS_SEG_16", wires::LONG_V[2]),       // VLL3
        ("TOPS_SEG_17", wires::LONG_V[1]),       // VLL2
        ("TOPS_SEG_18", wires::LONG_V[0]),       // VLL1
        ("TOPS_SEG_19", wires::LONG_V[5]),       // VLL6
        ("TOPS_SEG_21", wires::LONG_V[4]),       // VLL5
        ("TOPS_SEG_22", wires::LONG_V[3]),       // VLL4
        ("TOPS_SEG_23", wires::DOUBLE_V0[1]),    // DV3
        ("TOPS_SEG_24", wires::SINGLE_V[5]),     // V6
        ("TOPS_SEG_26", wires::DOUBLE_V1[0]),    // DV1
        ("TOPS_SEG_27", wires::DOUBLE_V0[0]),    // DV2
        // TOPS_SEG_31 CE_2L
        ("TOPS_SEG_40", wires::GCLK[3]),         // K4
        ("TOPS_SEG_41", wires::SINGLE_V[7]),     // V8
        ("TOPS_SEG_42", wires::OUT_IO_SN_I1_E1), // I1_2L
        ("TOPS_SEG_43", wires::GCLK[2]),         // K3
        ("TOPS_SEG_44", wires::GCLK[1]),         // K2
        ("TOPS_SEG_45", wires::GCLK[0]),         // K1
        ("TOPS_SEG_46", wires::SINGLE_V[6]),     // V7
        // TOPS_SEG_48 IK_2L
        // TOPS_SEG_49 OK_2L
        ("TOPS_SEG_63", wires::DEC_H[3]),        // TTX1
        ("TOPS_SEG_64", wires::DEC_H[2]),        // TTX2
        ("TOPS_SEG_65", wires::DEC_H[1]),        // TTX3
        ("TOPS_SEG_66", wires::DEC_H[0]),        // TTX4
        ("TOPS_SEG_67", wires::LONG_IO_H[0]),    // THLL1
        ("TOPS_SEG_68", wires::LONG_IO_H[1]),    // THLL2
        ("TOPS_SEG_69", wires::LONG_IO_H[2]),    // THLL3
        ("TOPS_SEG_70", wires::LONG_IO_H[3]),    // THLL4
        ("TOPS_SEG_71", wires::DOUBLE_IO_N2[0]), // DH1
        ("TOPS_SEG_72", wires::DBUF_IO_H[0]),    // DMUX_OUTER
        ("TOPS_SEG_73", wires::DBUF_IO_H[1]),    // DMUX_INNER
        ("TOPS_SEG_74", wires::DOUBLE_IO_N0[0]), // DH1L
        ("TOPS_SEG_75", wires::DOUBLE_IO_N1[0]), // DH2
        ("TOPS_SEG_76", wires::DOUBLE_IO_N2[1]), // DH3
        ("TOPS_SEG_77", wires::DOUBLE_IO_N0[1]), // DH3L
        ("TOPS_SEG_78", wires::DOUBLE_IO_N1[1]), // DH4
        ("TOPS_SEG_79", wires::DOUBLE_IO_N2[2]), // DH5
        ("TOPS_SEG_80", wires::DOUBLE_IO_N0[2]), // DH5L
        ("TOPS_SEG_81", wires::DOUBLE_IO_N1[2]), // DH6
        ("TOPS_SEG_82", wires::DOUBLE_IO_N2[3]), // DH7
        ("TOPS_SEG_83", wires::DOUBLE_IO_N0[3]), // DH7L
        ("TOPS_SEG_84", wires::DOUBLE_IO_N1[3]), // DH8
        // TOPS_SEG_85 CINB
        // TOPS_SEG_86 CIN
        // TOPS_SEG_87 CLOCK_7_8
        // TOPSL
        ("I_BUFGS_TL_I_TOPSL", wires::OUT_IO_CLKIN),
        ("PAD1_I1", wires::OUT_IO_SN_I1[0]),
        ("PAD1_I2", wires::OUT_IO_SN_I2[0]),
        ("PAD1_IK", wires::IMUX_IO_IK[0]),
        ("PAD1_OK", wires::IMUX_IO_OK[0]),
        ("PAD1_T", wires::IMUX_IO_T[0]),
        ("PAD2_I1", wires::OUT_IO_SN_I1[1]),
        ("PAD2_I2", wires::OUT_IO_SN_I2[1]),
        ("PAD2_IK", wires::IMUX_IO_IK[1]),
        ("PAD2_OK", wires::IMUX_IO_OK[1]),
        ("PAD2_T", wires::IMUX_IO_T[1]),
        ("DEC_AB_2_I", wires::IMUX_CLB_C2_N),
        // TOPSL_SEG_0 G4B
        ("TOPSL_SEG_1", wires::LONG_H[0]), // HLL1
        // TOPSL_SEG_3 F4B
        ("TOPSL_SEG_5", wires::SINGLE_V[1]), // V2
        ("TOPSL_SEG_6", wires::LONG_H[1]),   // HLL2
        // TOPSL_SEG_7 C4B
        ("TOPSL_SEG_9", wires::SINGLE_V[2]),      // V3
        ("TOPSL_SEG_10", wires::LONG_H[2]),       // HLL3
        ("TOPSL_SEG_11", wires::SINGLE_V[3]),     // V4
        ("TOPSL_SEG_12", wires::DOUBLE_V1[1]),    // DV4
        ("TOPSL_SEG_13", wires::OUT_IO_SN_I2_E1), // I2_2L
        ("TOPSL_SEG_14", wires::SINGLE_V[4]),     // V5
        ("TOPSL_SEG_15", wires::SINGLE_V[0]),     // V1
        ("TOPSL_SEG_16", wires::LONG_V[2]),       // VLL3
        ("TOPSL_SEG_17", wires::LONG_V[1]),       // VLL2
        ("TOPSL_SEG_18", wires::LONG_V[0]),       // VLL1
        ("TOPSL_SEG_19", wires::LONG_V[5]),       // VLL6
        ("TOPSL_SEG_21", wires::LONG_V[4]),       // VLL5
        ("TOPSL_SEG_22", wires::LONG_V[3]),       // VLL4
        ("TOPSL_SEG_23", wires::DOUBLE_V0[1]),    // DV3
        ("TOPSL_SEG_24", wires::SINGLE_V[5]),     // V6
        ("TOPSL_SEG_25", wires::DOUBLE_V0[0]),    // DV2
        // TOPSL_SEG_29 CE_2L
        ("TOPSL_SEG_30", wires::DOUBLE_V1[0]),    // DV1
        ("TOPSL_SEG_39", wires::GCLK[3]),         // K4
        ("TOPSL_SEG_40", wires::SINGLE_V[7]),     // V8
        ("TOPSL_SEG_41", wires::OUT_IO_SN_I1_E1), // I1_2L
        ("TOPSL_SEG_42", wires::GCLK[2]),         // K3
        ("TOPSL_SEG_43", wires::GCLK[1]),         // K2
        ("TOPSL_SEG_44", wires::GCLK[0]),         // K1
        ("TOPSL_SEG_45", wires::SINGLE_V[6]),     // V7
        // TOPSL_SEG_47 IK_2L
        ("TOPSL_SEG_61", wires::DEC_H[3]),        // TTX1
        ("TOPSL_SEG_62", wires::DEC_H[2]),        // TTX2
        ("TOPSL_SEG_63", wires::DEC_H[1]),        // TTX3
        ("TOPSL_SEG_64", wires::DEC_H[0]),        // TTX4
        ("TOPSL_SEG_65", wires::LONG_IO_H[0]),    // THLL1
        ("TOPSL_SEG_66", wires::LONG_IO_H[1]),    // THLL2
        ("TOPSL_SEG_67", wires::LONG_IO_H[2]),    // THLL3
        ("TOPSL_SEG_68", wires::LONG_IO_H[3]),    // THLL4
        ("TOPSL_SEG_69", wires::DOUBLE_IO_N2[0]), // DH1
        ("TOPSL_SEG_70", wires::DBUF_IO_H[0]),    // DMUX_OUTER
        ("TOPSL_SEG_71", wires::DBUF_IO_H[1]),    // DMUX_INNER
        ("TOPSL_SEG_72", wires::DOUBLE_IO_N0[0]), // DH1L
        ("TOPSL_SEG_73", wires::DOUBLE_IO_N1[0]), // DH2
        ("TOPSL_SEG_74", wires::DOUBLE_IO_N2[1]), // DH3
        ("TOPSL_SEG_75", wires::DOUBLE_IO_N0[1]), // DH3L
        ("TOPSL_SEG_76", wires::DOUBLE_IO_N1[1]), // DH4
        ("TOPSL_SEG_77", wires::DOUBLE_IO_N2[2]), // DH5
        ("TOPSL_SEG_78", wires::DOUBLE_IO_N0[2]), // DH5L
        ("TOPSL_SEG_79", wires::DOUBLE_IO_N1[2]), // DH6
        ("TOPSL_SEG_80", wires::DOUBLE_IO_N2[3]), // DH7
        ("TOPSL_SEG_81", wires::DOUBLE_IO_N0[3]), // DH7L
        ("TOPSL_SEG_82", wires::DOUBLE_IO_N1[3]), // DH8
        // TOPSL_SEG_83 CINB
        // TOPSL_SEG_84 CIN
        // TOPSL_SEG_85 CLOCK_7_8

        // RT
        ("PAD21_I1", wires::OUT_IO_WE_I1[0]),
        ("PAD21_I2", wires::OUT_IO_WE_I2[0]),
        ("PAD21_IK", wires::IMUX_IO_IK[0]),
        ("PAD21_OK", wires::IMUX_IO_OK[0]),
        ("PAD21_T", wires::IMUX_IO_T[0]),
        ("PAD22_I1", wires::OUT_IO_WE_I1[1]),
        ("PAD22_I2", wires::OUT_IO_WE_I2[1]),
        ("PAD22_IK", wires::IMUX_IO_IK[1]),
        ("PAD22_OK", wires::IMUX_IO_OK[1]),
        ("PAD22_T", wires::IMUX_IO_T[1]),
        ("DEC_DK_2_I", wires::IMUX_CLB_C1),
        ("TBUF_DK_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_DK_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_DK_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_DK_1_T", wires::IMUX_TBUF_T[1]),
        ("TIE_DK_1_O", wires::TIE_0),
        ("RT_SEG_0", wires::LONG_IO_V[0]),     // RVLL1
        ("RT_SEG_1", wires::LONG_H[0]),        // HLL1
        ("RT_SEG_2", wires::DEC_V[3]),         // RTX1
        ("RT_SEG_6", wires::SINGLE_V[1]),      // V2
        ("RT_SEG_7", wires::LONG_IO_V[1]),     // RVLL2
        ("RT_SEG_8", wires::LONG_H[1]),        // HLL2
        ("RT_SEG_9", wires::DEC_V[2]),         // RTX2
        ("RT_SEG_10", wires::SINGLE_V[2]),     // V3
        ("RT_SEG_12", wires::LONG_H[2]),       // HLL3
        ("RT_SEG_13", wires::DOUBLE_IO_E1[3]), // RDV8
        ("RT_SEG_14", wires::DOUBLE_IO_E2[3]), // RDV7
        ("RT_SEG_15", wires::DOUBLE_IO_E1[2]), // RDV6
        ("RT_SEG_16", wires::DOUBLE_IO_E2[2]), // RDV5
        ("RT_SEG_17", wires::LONG_IO_V[2]),    // RVLL3
        ("RT_SEG_18", wires::DEC_V[1]),        // RTX3
        ("RT_SEG_21", wires::SINGLE_V[3]),     // V4
        ("RT_SEG_22", wires::DEC_V[0]),        // RTX4
        ("RT_SEG_23", wires::DOUBLE_IO_E1[1]), // RDV4
        ("RT_SEG_25", wires::DOUBLE_IO_E2[1]), // RDV3
        ("RT_SEG_26", wires::DOUBLE_IO_E1[0]), // RDV2
        ("RT_SEG_27", wires::DOUBLE_IO_E2[0]), // RDV1
        ("RT_SEG_28", wires::LONG_IO_V[3]),    // RVLL4
        ("RT_SEG_35", wires::DOUBLE_V0[1]),    // DV3
        ("RT_SEG_36", wires::SINGLE_V[5]),     // V6
        ("RT_SEG_38", wires::GCLK[3]),         // K4
        ("RT_SEG_40", wires::GCLK[2]),         // K3
        ("RT_SEG_41", wires::GCLK[1]),         // K2
        ("RT_SEG_42", wires::GCLK[0]),         // K1
        ("RT_SEG_46", wires::SINGLE_V[7]),     // V8
        ("RT_SEG_47", wires::SINGLE_V[0]),     // V1
        ("RT_SEG_49", wires::LONG_V[4]),       // VLL5
        ("RT_SEG_50", wires::LONG_V[3]),       // VLL4
        ("RT_SEG_51", wires::DOUBLE_V1[1]),    // DV4
        ("RT_SEG_52", wires::SINGLE_V[6]),     // V7
        ("RT_SEG_53", wires::SINGLE_V[4]),     // V5
        ("RT_SEG_54", wires::DOUBLE_V0[0]),    // DV2
        ("RT_SEG_55", wires::DOUBLE_V1[0]),    // DV1
        ("RT_SEG_56", wires::LONG_V[1]),       // VLL2
        ("RT_SEG_57", wires::LONG_V[0]),       // VLL1
        ("RT_SEG_59", wires::LONG_V[5]),       // VLL6
        ("RT_SEG_60", wires::IMUX_CLB_F3),     // F3L
        ("RT_SEG_61", wires::LONG_V[2]),       // VLL3
        ("RT_SEG_66", wires::IMUX_CLB_C3),     // C3L
        ("RT_SEG_69", wires::IMUX_CLB_G3),     // G3L
        ("RT_SEG_73", wires::OUT_CLB_Y_E),     // GYL
        ("RT_SEG_79", wires::OUT_CLB_YQ_E),    // GYQL
        ("RT_SEG_84", wires::LONG_H[3]),       // HLL4
        ("RT_SEG_85", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("RT_SEG_86", wires::LONG_H[4]),       // HLL5
        ("RT_SEG_87", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("RT_SEG_88", wires::LONG_H[5]),       // HLL6
        ("RT_SEG_89", wires::DOUBLE_H1[0]),    // DH1
        ("RT_SEG_90", wires::DOUBLE_IO_E0[0]), // RDV1T
        ("RT_SEG_91", wires::OUT_IO_WE_I1_S1), // I1_2T
        ("RT_SEG_92", wires::DOUBLE_IO_E0[3]), // RDV7T
        ("RT_SEG_93", wires::DOUBLE_IO_E0[2]), // RDV5T
        ("RT_SEG_94", wires::DOUBLE_IO_E0[1]), // RDV3T
        ("RT_SEG_95", wires::DOUBLE_H0[0]),    // DH2R
        // RT_SEG_96 CE_2T
        ("RT_SEG_98", wires::DOUBLE_H2[0]),     // DH2
        ("RT_SEG_99", wires::SINGLE_H[0]),      // H1R
        ("RT_SEG_100", wires::OUT_IO_WE_I2_S1), // I2_2T
        ("RT_SEG_101", wires::SINGLE_H_E[0]),   // H1
        ("RT_SEG_102", wires::SINGLE_H[1]),     // H2R
        ("RT_SEG_103", wires::SINGLE_H_E[1]),   // H2
        ("RT_SEG_104", wires::SINGLE_H[2]),     // H3R
        // RT_SEG_105 OK_2T
        // RT_SEG_106 IK_2T
        ("RT_SEG_107", wires::SINGLE_H_E[2]), // H3
        ("RT_SEG_108", wires::SINGLE_H[3]),   // H4R
        ("RT_SEG_109", wires::SINGLE_H_E[3]), // H4
        ("RT_SEG_110", wires::SINGLE_H[4]),   // H5R
        ("RT_SEG_111", wires::SINGLE_H_E[4]), // H5
        ("RT_SEG_112", wires::SINGLE_H[5]),   // H6R
        ("RT_SEG_113", wires::SINGLE_H_E[5]), // H6
        ("RT_SEG_114", wires::SINGLE_H[6]),   // H7R
        ("RT_SEG_115", wires::SINGLE_H_E[6]), // H7
        ("RT_SEG_116", wires::SINGLE_H[7]),   // H8R
        ("RT_SEG_117", wires::SINGLE_H_E[7]), // H8
        ("RT_SEG_118", wires::DOUBLE_H0[1]),  // DH3R
        ("RT_SEG_119", wires::DOUBLE_H2[1]),  // DH3
        ("RT_SEG_120", wires::DOUBLE_H1[1]),  // DH4
        ("RT_SEG_121", wires::DOUBLE_V2[1]),  // DV3T
        ("RT_SEG_122", wires::SINGLE_V_S[7]), // V8T
        ("RT_SEG_123", wires::SINGLE_V_S[6]), // V7T
        ("RT_SEG_124", wires::SINGLE_V_S[5]), // V6T
        ("RT_SEG_125", wires::SINGLE_V_S[4]), // V5T
        ("RT_SEG_126", wires::SINGLE_V_S[3]), // V4T
        ("RT_SEG_127", wires::SINGLE_V_S[2]), // V3T
        ("RT_SEG_128", wires::SINGLE_V_S[1]), // V2T
        ("RT_SEG_129", wires::SINGLE_V_S[0]), // V1T
        ("RT_SEG_130", wires::DOUBLE_V2[0]),  // DV2T
        // RT_SEG_131: CLOCK_5_6

        // RTT
        ("I_BUFGS_TR_I_RTT", wires::OUT_IO_CLKIN),
        ("PAD17_I1", wires::OUT_IO_WE_I1[0]),
        ("PAD17_I2", wires::OUT_IO_WE_I2[0]),
        ("PAD17_IK", wires::IMUX_IO_IK[0]),
        ("PAD17_OK", wires::IMUX_IO_OK[0]),
        ("PAD17_T", wires::IMUX_IO_T[0]),
        ("PAD18_I1", wires::OUT_IO_WE_I1[1]),
        ("PAD18_I2", wires::OUT_IO_WE_I2[1]),
        ("PAD18_IK", wires::IMUX_IO_IK[1]),
        ("PAD18_OK", wires::IMUX_IO_OK[1]),
        ("PAD18_T", wires::IMUX_IO_T[1]),
        ("DEC_BK_2_I", wires::IMUX_CLB_C1),
        ("TBUF_BK_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_BK_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_BK_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_BK_1_T", wires::IMUX_TBUF_T[1]),
        ("TIE_BK_1_O", wires::TIE_0),
        ("RTT_SEG_0", wires::LONG_IO_V[0]),     // RVLL1
        ("RTT_SEG_1", wires::LONG_H[0]),        // HLL1
        ("RTT_SEG_2", wires::DEC_V[3]),         // RTX1
        ("RTT_SEG_6", wires::SINGLE_V[1]),      // V2
        ("RTT_SEG_7", wires::LONG_IO_V[1]),     // RVLL2
        ("RTT_SEG_8", wires::LONG_H[1]),        // HLL2
        ("RTT_SEG_9", wires::DEC_V[2]),         // RTX2
        ("RTT_SEG_10", wires::SINGLE_V[2]),     // V3
        ("RTT_SEG_12", wires::LONG_H[2]),       // HLL3
        ("RTT_SEG_13", wires::DOUBLE_IO_E1[3]), // RDV8
        ("RTT_SEG_14", wires::DOUBLE_IO_E2[3]), // RDV7
        ("RTT_SEG_15", wires::DOUBLE_IO_E1[2]), // RDV6
        ("RTT_SEG_16", wires::DOUBLE_IO_E2[2]), // RDV5
        ("RTT_SEG_17", wires::LONG_IO_V[2]),    // RVLL3
        ("RTT_SEG_18", wires::DEC_V[1]),        // RTX3
        ("RTT_SEG_21", wires::SINGLE_V[3]),     // V4
        ("RTT_SEG_22", wires::DEC_V[0]),        // RTX4
        ("RTT_SEG_23", wires::DOUBLE_IO_E1[1]), // RDV4
        ("RTT_SEG_25", wires::DOUBLE_IO_E2[1]), // RDV3
        ("RTT_SEG_26", wires::DOUBLE_IO_E1[0]), // RDV2
        ("RTT_SEG_27", wires::DOUBLE_IO_E2[0]), // RDV1
        ("RTT_SEG_28", wires::LONG_IO_V[3]),    // RVLL4
        ("RTT_SEG_35", wires::DOUBLE_V0[1]),    // DV3
        ("RTT_SEG_36", wires::SINGLE_V[5]),     // V6
        ("RTT_SEG_38", wires::GCLK[3]),         // K4
        ("RTT_SEG_40", wires::GCLK[2]),         // K3
        ("RTT_SEG_41", wires::GCLK[1]),         // K2
        ("RTT_SEG_42", wires::GCLK[0]),         // K1
        ("RTT_SEG_46", wires::SINGLE_V[7]),     // V8
        ("RTT_SEG_47", wires::SINGLE_V[0]),     // V1
        ("RTT_SEG_49", wires::LONG_V[4]),       // VLL5
        ("RTT_SEG_50", wires::LONG_V[3]),       // VLL4
        ("RTT_SEG_51", wires::DOUBLE_V1[1]),    // DV4
        ("RTT_SEG_52", wires::SINGLE_V[6]),     // V7
        ("RTT_SEG_53", wires::SINGLE_V[4]),     // V5
        ("RTT_SEG_54", wires::DOUBLE_V0[0]),    // DV2
        ("RTT_SEG_55", wires::DOUBLE_V1[0]),    // DV1
        ("RTT_SEG_56", wires::LONG_V[1]),       // VLL2
        ("RTT_SEG_57", wires::LONG_V[0]),       // VLL1
        ("RTT_SEG_59", wires::LONG_V[5]),       // VLL6
        ("RTT_SEG_60", wires::IMUX_CLB_F3),     // F3L
        ("RTT_SEG_61", wires::LONG_V[2]),       // VLL3
        ("RTT_SEG_66", wires::IMUX_CLB_C3),     // C3L
        ("RTT_SEG_69", wires::IMUX_CLB_G3),     // G3L
        ("RTT_SEG_73", wires::OUT_CLB_Y_E),     // GYL
        ("RTT_SEG_79", wires::OUT_CLB_YQ_E),    // GYQL
        ("RTT_SEG_84", wires::LONG_H[3]),       // HLL4
        ("RTT_SEG_85", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("RTT_SEG_86", wires::LONG_H[4]),       // HLL5
        ("RTT_SEG_87", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("RTT_SEG_88", wires::LONG_H[5]),       // HLL6
        ("RTT_SEG_89", wires::DOUBLE_H1[0]),    // DH1
        ("RTT_SEG_90", wires::DOUBLE_IO_E0[0]), // RDV1T
        ("RTT_SEG_91", wires::OUT_IO_WE_I1_S1), // I1_2T
        ("RTT_SEG_92", wires::DOUBLE_IO_E0[3]), // RDV7T
        ("RTT_SEG_93", wires::DOUBLE_IO_E0[2]), // RDV5T
        ("RTT_SEG_94", wires::DOUBLE_IO_E0[1]), // RDV3T
        ("RTT_SEG_95", wires::DOUBLE_H0[0]),    // DH2R
        // RTT_SEG_96 CE_2T
        ("RTT_SEG_97", wires::DOUBLE_H2[0]),    // DH2
        ("RTT_SEG_98", wires::SINGLE_H[0]),     // H1R
        ("RTT_SEG_99", wires::OUT_IO_WE_I2_S1), // I2_2T
        ("RTT_SEG_100", wires::SINGLE_H_E[0]),  // H1
        ("RTT_SEG_101", wires::SINGLE_H[1]),    // H2R
        ("RTT_SEG_102", wires::SINGLE_H_E[1]),  // H2
        ("RTT_SEG_103", wires::SINGLE_H[2]),    // H3R
        // RTT_SEG_104 OK_2T
        ("RTT_SEG_105", wires::SINGLE_H_E[2]), // H3
        ("RTT_SEG_106", wires::SINGLE_H[3]),   // H4R
        ("RTT_SEG_107", wires::SINGLE_H_E[3]), // H4
        ("RTT_SEG_108", wires::SINGLE_H[4]),   // H5R
        ("RTT_SEG_109", wires::SINGLE_H_E[4]), // H5
        ("RTT_SEG_110", wires::SINGLE_H[5]),   // H6R
        ("RTT_SEG_111", wires::SINGLE_H_E[5]), // H6
        ("RTT_SEG_112", wires::SINGLE_H[6]),   // H7R
        ("RTT_SEG_113", wires::SINGLE_H_E[6]), // H7
        ("RTT_SEG_114", wires::SINGLE_H[7]),   // H8R
        ("RTT_SEG_115", wires::SINGLE_H_E[7]), // H8
        ("RTT_SEG_116", wires::DOUBLE_H0[1]),  // DH3R
        ("RTT_SEG_117", wires::DOUBLE_H2[1]),  // DH3
        ("RTT_SEG_118", wires::DOUBLE_H1[1]),  // DH4
        ("RTT_SEG_119", wires::DOUBLE_V2[1]),  // DV3T
        ("RTT_SEG_120", wires::SINGLE_V_S[7]), // V8T
        ("RTT_SEG_121", wires::SINGLE_V_S[6]), // V7T
        ("RTT_SEG_122", wires::SINGLE_V_S[5]), // V6T
        ("RTT_SEG_123", wires::SINGLE_V_S[4]), // V5T
        ("RTT_SEG_124", wires::SINGLE_V_S[3]), // V4T
        ("RTT_SEG_125", wires::SINGLE_V_S[2]), // V3T
        ("RTT_SEG_126", wires::SINGLE_V_S[1]), // V2T
        ("RTT_SEG_127", wires::SINGLE_V_S[0]), // V1T
        ("RTT_SEG_128", wires::DOUBLE_V2[0]),  // DV2T
        // RTT_SEG_129: CLOCK_5_6

        // RTS
        ("PAD27_I1", wires::OUT_IO_WE_I1[0]),
        ("PAD27_I2", wires::OUT_IO_WE_I2[0]),
        ("PAD27_IK", wires::IMUX_IO_IK[0]),
        ("PAD27_OK", wires::IMUX_IO_OK[0]),
        ("PAD27_T", wires::IMUX_IO_T[0]),
        ("PAD28_I1", wires::OUT_IO_WE_I1[1]),
        ("PAD28_I2", wires::OUT_IO_WE_I2[1]),
        ("PAD28_IK", wires::IMUX_IO_IK[1]),
        ("PAD28_OK", wires::IMUX_IO_OK[1]),
        ("PAD28_T", wires::IMUX_IO_T[1]),
        ("DEC_HK_2_I", wires::IMUX_CLB_C1),
        ("TBUF_HK_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_HK_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_HK_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_HK_1_T", wires::IMUX_TBUF_T[1]),
        ("TIE_HK_1_O", wires::TIE_0),
        ("RTS_SEG_0", wires::LONG_IO_V[0]),     // RVLL1
        ("RTS_SEG_1", wires::LONG_H[0]),        // HLL1
        ("RTS_SEG_2", wires::DEC_V[3]),         // RTX1
        ("RTS_SEG_6", wires::SINGLE_V[1]),      // V2
        ("RTS_SEG_7", wires::LONG_IO_V[1]),     // RVLL2
        ("RTS_SEG_8", wires::LONG_H[1]),        // HLL2
        ("RTS_SEG_9", wires::DEC_V[2]),         // RTX2
        ("RTS_SEG_10", wires::SINGLE_V[2]),     // V3
        ("RTS_SEG_12", wires::LONG_H[2]),       // HLL3
        ("RTS_SEG_13", wires::DOUBLE_IO_E1[3]), // RDV8
        ("RTS_SEG_14", wires::DOUBLE_IO_E2[3]), // RDV7
        ("RTS_SEG_15", wires::DOUBLE_IO_E1[2]), // RDV6
        ("RTS_SEG_16", wires::DOUBLE_IO_E2[2]), // RDV5
        ("RTS_SEG_17", wires::LONG_IO_V[2]),    // RVLL3
        ("RTS_SEG_18", wires::DEC_V[1]),        // RTX3
        ("RTS_SEG_21", wires::SINGLE_V[3]),     // V4
        ("RTS_SEG_22", wires::DEC_V[0]),        // RTX4
        ("RTS_SEG_23", wires::DOUBLE_IO_E1[1]), // RDV4
        ("RTS_SEG_25", wires::DOUBLE_IO_E2[1]), // RDV3
        ("RTS_SEG_26", wires::DOUBLE_IO_E1[0]), // RDV2
        ("RTS_SEG_27", wires::DOUBLE_IO_E2[0]), // RDV1
        ("RTS_SEG_28", wires::LONG_IO_V[3]),    // RVLL4
        ("RTS_SEG_35", wires::DOUBLE_V0[1]),    // DV3
        ("RTS_SEG_36", wires::SINGLE_V[5]),     // V6
        ("RTS_SEG_38", wires::GCLK[3]),         // K4
        ("RTS_SEG_40", wires::GCLK[2]),         // K3
        ("RTS_SEG_41", wires::GCLK[1]),         // K2
        ("RTS_SEG_42", wires::GCLK[0]),         // K1
        ("RTS_SEG_46", wires::SINGLE_V[7]),     // V8
        ("RTS_SEG_47", wires::SINGLE_V[0]),     // V1
        ("RTS_SEG_49", wires::LONG_V[4]),       // VLL5
        ("RTS_SEG_50", wires::LONG_V[3]),       // VLL4
        ("RTS_SEG_51", wires::DOUBLE_V1[1]),    // DV4
        ("RTS_SEG_52", wires::SINGLE_V[6]),     // V7
        ("RTS_SEG_53", wires::SINGLE_V[4]),     // V5
        ("RTS_SEG_54", wires::DOUBLE_V0[0]),    // DV2
        ("RTS_SEG_55", wires::DOUBLE_V1[0]),    // DV1
        ("RTS_SEG_56", wires::LONG_V[1]),       // VLL2
        ("RTS_SEG_57", wires::LONG_V[0]),       // VLL1
        ("RTS_SEG_59", wires::LONG_V[5]),       // VLL6
        ("RTS_SEG_60", wires::IMUX_CLB_F3),     // F3L
        ("RTS_SEG_61", wires::LONG_V[2]),       // VLL3
        ("RTS_SEG_66", wires::IMUX_CLB_C3),     // C3L
        ("RTS_SEG_69", wires::IMUX_CLB_G3),     // G3L
        ("RTS_SEG_73", wires::OUT_CLB_Y_E),     // GYL
        ("RTS_SEG_79", wires::OUT_CLB_YQ_E),    // GYQL
        ("RTS_SEG_84", wires::LONG_H[3]),       // HLL4
        ("RTS_SEG_85", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("RTS_SEG_86", wires::LONG_H[4]),       // HLL5
        ("RTS_SEG_87", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("RTS_SEG_88", wires::LONG_H[5]),       // HLL6
        ("RTS_SEG_89", wires::DOUBLE_H1[0]),    // DH1
        ("RTS_SEG_90", wires::DOUBLE_IO_E0[0]), // RDV1T
        ("RTS_SEG_91", wires::OUT_IO_WE_I1_S1), // I1_2T
        ("RTS_SEG_92", wires::DOUBLE_IO_E0[3]), // RDV7T
        ("RTS_SEG_93", wires::DOUBLE_IO_E0[2]), // RDV5T
        ("RTS_SEG_94", wires::DOUBLE_IO_E0[1]), // RDV3T
        ("RTS_SEG_95", wires::DOUBLE_H0[0]),    // DH2R
        // RTS_SEG_96 CE_2T
        ("RTS_SEG_98", wires::DOUBLE_H2[0]),     // DH2
        ("RTS_SEG_99", wires::SINGLE_H[0]),      // H1R
        ("RTS_SEG_100", wires::OUT_IO_WE_I2_S1), // I2_2T
        ("RTS_SEG_101", wires::SINGLE_H_E[0]),   // H1
        ("RTS_SEG_102", wires::SINGLE_H[1]),     // H2R
        ("RTS_SEG_103", wires::SINGLE_H_E[1]),   // H2
        ("RTS_SEG_104", wires::SINGLE_H[2]),     // H3R
        // RTS_SEG_105 OK_2T
        // RTS_SEG_106 IK_2T
        ("RTS_SEG_107", wires::SINGLE_H_E[2]), // H3
        ("RTS_SEG_108", wires::SINGLE_H[3]),   // H4R
        ("RTS_SEG_109", wires::SINGLE_H_E[3]), // H4
        ("RTS_SEG_110", wires::SINGLE_H[4]),   // H5R
        ("RTS_SEG_111", wires::SINGLE_H_E[4]), // H5
        ("RTS_SEG_112", wires::SINGLE_H[5]),   // H6R
        ("RTS_SEG_113", wires::SINGLE_H_E[5]), // H6
        ("RTS_SEG_114", wires::SINGLE_H[6]),   // H7R
        ("RTS_SEG_115", wires::SINGLE_H_E[6]), // H7
        ("RTS_SEG_116", wires::SINGLE_H[7]),   // H8R
        ("RTS_SEG_117", wires::SINGLE_H_E[7]), // H8
        ("RTS_SEG_118", wires::DOUBLE_H0[1]),  // DH3R
        ("RTS_SEG_119", wires::DOUBLE_H2[1]),  // DH3
        ("RTS_SEG_120", wires::DOUBLE_H1[1]),  // DH4
        ("RTS_SEG_121", wires::DOUBLE_V2[1]),  // DV3T
        ("RTS_SEG_122", wires::SINGLE_V_S[7]), // V8T
        ("RTS_SEG_123", wires::SINGLE_V_S[6]), // V7T
        ("RTS_SEG_124", wires::SINGLE_V_S[5]), // V6T
        ("RTS_SEG_125", wires::SINGLE_V_S[4]), // V5T
        ("RTS_SEG_126", wires::SINGLE_V_S[3]), // V4T
        ("RTS_SEG_127", wires::SINGLE_V_S[2]), // V3T
        ("RTS_SEG_128", wires::SINGLE_V_S[1]), // V2T
        ("RTS_SEG_129", wires::SINGLE_V_S[0]), // V1T
        ("RTS_SEG_130", wires::DOUBLE_V2[0]),  // DV2T
        // RTS_SEG_131: CLOCK_5_6

        // RTSB
        ("I_BUFGP_BR_I_RTSB", wires::OUT_IO_CLKIN),
        ("PAD31_I1", wires::OUT_IO_WE_I1[0]),
        ("PAD31_I2", wires::OUT_IO_WE_I2[0]),
        ("PAD31_IK", wires::IMUX_IO_IK[0]),
        ("PAD31_OK", wires::IMUX_IO_OK[0]),
        ("PAD31_T", wires::IMUX_IO_T[0]),
        ("PAD32_I1", wires::OUT_IO_WE_I1[1]),
        ("PAD32_I2", wires::OUT_IO_WE_I2[1]),
        ("PAD32_IK", wires::IMUX_IO_IK[1]),
        ("PAD32_OK", wires::IMUX_IO_OK[1]),
        ("PAD32_T", wires::IMUX_IO_T[1]),
        ("DEC_JK_2_I", wires::IMUX_CLB_C1),
        ("TBUF_JK_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_JK_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_JK_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_JK_1_T", wires::IMUX_TBUF_T[1]),
        ("TIE_JK_1_O", wires::TIE_0),
        ("RTSB_SEG_0", wires::LONG_IO_V[0]),     // RVLL1
        ("RTSB_SEG_1", wires::LONG_H[0]),        // HLL1
        ("RTSB_SEG_2", wires::DEC_V[3]),         // RTX1
        ("RTSB_SEG_6", wires::SINGLE_V[1]),      // V2
        ("RTSB_SEG_7", wires::LONG_IO_V[1]),     // RVLL2
        ("RTSB_SEG_8", wires::LONG_H[1]),        // HLL2
        ("RTSB_SEG_9", wires::DEC_V[2]),         // RTX2
        ("RTSB_SEG_10", wires::SINGLE_V[2]),     // V3
        ("RTSB_SEG_12", wires::LONG_H[2]),       // HLL3
        ("RTSB_SEG_13", wires::DOUBLE_IO_E1[3]), // RDV8
        ("RTSB_SEG_14", wires::DOUBLE_IO_E2[3]), // RDV7
        ("RTSB_SEG_15", wires::DOUBLE_IO_E1[2]), // RDV6
        ("RTSB_SEG_16", wires::DOUBLE_IO_E2[2]), // RDV5
        ("RTSB_SEG_17", wires::LONG_IO_V[2]),    // RVLL3
        ("RTSB_SEG_18", wires::DEC_V[1]),        // RTX3
        ("RTSB_SEG_21", wires::SINGLE_V[3]),     // V4
        ("RTSB_SEG_22", wires::DEC_V[0]),        // RTX4
        ("RTSB_SEG_23", wires::DOUBLE_IO_E1[1]), // RDV4
        ("RTSB_SEG_25", wires::DOUBLE_IO_E2[1]), // RDV3
        ("RTSB_SEG_26", wires::DOUBLE_IO_E1[0]), // RDV2
        ("RTSB_SEG_27", wires::DOUBLE_IO_E2[0]), // RDV1
        ("RTSB_SEG_28", wires::LONG_IO_V[3]),    // RVLL4
        ("RTSB_SEG_35", wires::DOUBLE_V0[1]),    // DV3
        ("RTSB_SEG_36", wires::SINGLE_V[5]),     // V6
        ("RTSB_SEG_38", wires::GCLK[3]),         // K4
        ("RTSB_SEG_40", wires::GCLK[2]),         // K3
        ("RTSB_SEG_41", wires::GCLK[1]),         // K2
        ("RTSB_SEG_42", wires::GCLK[0]),         // K1
        ("RTSB_SEG_46", wires::SINGLE_V[7]),     // V8
        ("RTSB_SEG_47", wires::SINGLE_V[0]),     // V1
        ("RTSB_SEG_49", wires::LONG_V[4]),       // VLL5
        ("RTSB_SEG_50", wires::LONG_V[3]),       // VLL4
        ("RTSB_SEG_51", wires::DOUBLE_V1[1]),    // DV4
        ("RTSB_SEG_52", wires::SINGLE_V[6]),     // V7
        ("RTSB_SEG_53", wires::SINGLE_V[4]),     // V5
        ("RTSB_SEG_54", wires::DOUBLE_V0[0]),    // DV2
        ("RTSB_SEG_55", wires::DOUBLE_V1[0]),    // DV1
        ("RTSB_SEG_56", wires::LONG_V[1]),       // VLL2
        ("RTSB_SEG_57", wires::LONG_V[0]),       // VLL1
        ("RTSB_SEG_59", wires::LONG_V[5]),       // VLL6
        ("RTSB_SEG_60", wires::IMUX_CLB_F3),     // F3L
        ("RTSB_SEG_61", wires::LONG_V[2]),       // VLL3
        ("RTSB_SEG_66", wires::IMUX_CLB_C3),     // C3L
        ("RTSB_SEG_69", wires::IMUX_CLB_G3),     // G3L
        ("RTSB_SEG_73", wires::OUT_CLB_Y_E),     // GYL
        ("RTSB_SEG_79", wires::OUT_CLB_YQ_E),    // GYQL
        ("RTSB_SEG_84", wires::LONG_H[3]),       // HLL4
        ("RTSB_SEG_85", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("RTSB_SEG_86", wires::LONG_H[4]),       // HLL5
        ("RTSB_SEG_87", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("RTSB_SEG_88", wires::LONG_H[5]),       // HLL6
        ("RTSB_SEG_89", wires::DOUBLE_H1[0]),    // DH1
        ("RTSB_SEG_90", wires::DOUBLE_IO_E0[0]), // RDV1T
        ("RTSB_SEG_91", wires::OUT_IO_WE_I1_S1), // I1_2T
        ("RTSB_SEG_92", wires::DOUBLE_IO_E0[3]), // RDV7T
        ("RTSB_SEG_93", wires::DOUBLE_IO_E0[2]), // RDV5T
        ("RTSB_SEG_94", wires::DOUBLE_IO_E0[1]), // RDV3T
        ("RTSB_SEG_95", wires::DOUBLE_H0[0]),    // DH2R
        // RTSB_SEG_96 CE_2T
        ("RTSB_SEG_98", wires::DOUBLE_H2[0]),     // DH2
        ("RTSB_SEG_99", wires::SINGLE_H[0]),      // H1R
        ("RTSB_SEG_100", wires::OUT_IO_WE_I2_S1), // I2_2T
        ("RTSB_SEG_101", wires::SINGLE_H_E[0]),   // H1
        ("RTSB_SEG_102", wires::SINGLE_H[1]),     // H2R
        ("RTSB_SEG_103", wires::SINGLE_H_E[1]),   // H2
        ("RTSB_SEG_104", wires::SINGLE_H[2]),     // H3R
        // RTSB_SEG_105 OK_2T
        // RTSB_SEG_106 IK_2T
        ("RTSB_SEG_107", wires::SINGLE_H_E[2]), // H3
        ("RTSB_SEG_108", wires::SINGLE_H[3]),   // H4R
        ("RTSB_SEG_109", wires::SINGLE_H_E[3]), // H4
        ("RTSB_SEG_110", wires::SINGLE_H[4]),   // H5R
        ("RTSB_SEG_111", wires::SINGLE_H_E[4]), // H5
        ("RTSB_SEG_112", wires::SINGLE_H[5]),   // H6R
        ("RTSB_SEG_113", wires::SINGLE_H_E[5]), // H6
        ("RTSB_SEG_114", wires::SINGLE_H[6]),   // H7R
        ("RTSB_SEG_115", wires::SINGLE_H_E[6]), // H7
        ("RTSB_SEG_116", wires::SINGLE_H[7]),   // H8R
        ("RTSB_SEG_117", wires::SINGLE_H_E[7]), // H8
        ("RTSB_SEG_118", wires::DOUBLE_H0[1]),  // DH3R
        ("RTSB_SEG_119", wires::DOUBLE_H2[1]),  // DH3
        ("RTSB_SEG_120", wires::DOUBLE_H1[1]),  // DH4
        ("RTSB_SEG_121", wires::DOUBLE_V2[1]),  // DV3T
        ("RTSB_SEG_122", wires::SINGLE_V_S[7]), // V8T
        ("RTSB_SEG_123", wires::SINGLE_V_S[6]), // V7T
        ("RTSB_SEG_124", wires::SINGLE_V_S[5]), // V6T
        ("RTSB_SEG_125", wires::SINGLE_V_S[4]), // V5T
        ("RTSB_SEG_126", wires::SINGLE_V_S[3]), // V4T
        ("RTSB_SEG_127", wires::SINGLE_V_S[2]), // V3T
        ("RTSB_SEG_128", wires::SINGLE_V_S[1]), // V2T
        ("RTSB_SEG_129", wires::SINGLE_V_S[0]), // V1T
        ("RTSB_SEG_130", wires::DOUBLE_V2[0]),  // DV2T
        // RTSB_SEG_131: CLOCK_5_6

        // LEFT
        ("PAD60_I1", wires::OUT_IO_WE_I1[0]),
        ("PAD60_I2", wires::OUT_IO_WE_I2[0]),
        ("PAD60_IK", wires::IMUX_IO_IK[0]),
        ("PAD60_OK", wires::IMUX_IO_OK[0]),
        ("PAD60_T", wires::IMUX_IO_T[0]),
        ("PAD59_I1", wires::OUT_IO_WE_I1[1]),
        ("PAD59_I2", wires::OUT_IO_WE_I2[1]),
        ("PAD59_IK", wires::IMUX_IO_IK[1]),
        ("PAD59_OK", wires::IMUX_IO_OK[1]),
        ("PAD59_T", wires::IMUX_IO_T[1]),
        ("DEC_DA_2_I", wires::IMUX_CLB_C3_W),
        ("TBUF_DA_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_DA_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_DA_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_DA_1_T", wires::IMUX_TBUF_T[1]),
        ("LEFT_SEG_1", wires::LONG_H[0]),        // HLL1
        ("LEFT_SEG_4", wires::DEC_V[0]),         // LTX1
        ("LEFT_SEG_5", wires::LONG_IO_V[0]),     // LVLL1
        ("LEFT_SEG_6", wires::LONG_H[1]),        // HLL2
        ("LEFT_SEG_7", wires::DEC_V[1]),         // LTX2
        ("LEFT_SEG_8", wires::LONG_IO_V[1]),     // LVLL2
        ("LEFT_SEG_9", wires::LONG_H[2]),        // HLL3
        ("LEFT_SEG_12", wires::DEC_V[2]),        // LTX3
        ("LEFT_SEG_13", wires::LONG_IO_V[2]),    // LVLL3
        ("LEFT_SEG_14", wires::DOUBLE_IO_W1[3]), // LDV8
        ("LEFT_SEG_15", wires::DOUBLE_IO_W0[3]), // LDV7T
        ("LEFT_SEG_16", wires::DOUBLE_IO_W1[2]), // LDV6
        ("LEFT_SEG_17", wires::DOUBLE_IO_W0[2]), // LDV5T
        ("LEFT_SEG_19", wires::DEC_V[3]),        // LTX4
        ("LEFT_SEG_21", wires::LONG_IO_V[3]),    // LVLL4
        ("LEFT_SEG_22", wires::DOUBLE_IO_W1[1]), // LDV4
        ("LEFT_SEG_23", wires::DOUBLE_IO_W0[1]), // LDV3T
        ("LEFT_SEG_24", wires::DOUBLE_IO_W1[0]), // LDV2
        ("LEFT_SEG_25", wires::DOUBLE_IO_W0[0]), // LDV1T
        ("LEFT_SEG_33", wires::GCLK[3]),         // K4
        ("LEFT_SEG_34", wires::GCLK[2]),         // K3
        ("LEFT_SEG_35", wires::GCLK[1]),         // K2
        ("LEFT_SEG_36", wires::GCLK[0]),         // K1
        ("LEFT_SEG_60", wires::LONG_H[3]),       // HLL4
        ("LEFT_SEG_61", wires::LONG_H[4]),       // HLL5
        ("LEFT_SEG_62", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("LEFT_SEG_63", wires::LONG_H[5]),       // HLL6
        ("LEFT_SEG_65", wires::DOUBLE_H0[0]),    // DH1
        ("LEFT_SEG_64", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("LEFT_SEG_66", wires::OUT_IO_WE_I1_S1), // I1_2T
        ("LEFT_SEG_67", wires::DOUBLE_IO_W2[0]), // LDV1
        ("LEFT_SEG_68", wires::DOUBLE_IO_W2[3]), // LDV7
        ("LEFT_SEG_69", wires::DOUBLE_IO_W2[2]), // LDV5
        ("LEFT_SEG_70", wires::SINGLE_H[1]),     // H2
        // LEFT_SEG_71: CE_2T
        ("LEFT_SEG_72", wires::DOUBLE_H1[0]),    // DH2
        ("LEFT_SEG_74", wires::DOUBLE_IO_W2[1]), // LDV3
        ("LEFT_SEG_75", wires::OUT_IO_WE_I2_S1), // I2_2T
        ("LEFT_SEG_76", wires::SINGLE_H[0]),     // H1
        ("LEFT_SEG_77", wires::SINGLE_H[2]),     // H3
        // LEFT_SEG_78: IK_2T
        // LEFT_SEG_79: OK_2T
        ("LEFT_SEG_80", wires::SINGLE_H[3]),  // H4
        ("LEFT_SEG_81", wires::SINGLE_H[4]),  // H5
        ("LEFT_SEG_82", wires::SINGLE_H[5]),  // H6
        ("LEFT_SEG_83", wires::SINGLE_H[6]),  // H7
        ("LEFT_SEG_84", wires::SINGLE_H[7]),  // H8
        ("LEFT_SEG_85", wires::DOUBLE_H1[1]), // DH3
        ("LEFT_SEG_86", wires::DOUBLE_H0[1]), // DH4
        // LEFT_SEG_87: CLOCK_1_2
        // LEFTT
        ("I_BUFGP_TL_I_LEFTT", wires::OUT_IO_CLKIN),
        ("PAD64_I1", wires::OUT_IO_WE_I1[0]),
        ("PAD64_I2", wires::OUT_IO_WE_I2[0]),
        ("PAD64_IK", wires::IMUX_IO_IK[0]),
        ("PAD64_OK", wires::IMUX_IO_OK[0]),
        ("PAD64_T", wires::IMUX_IO_T[0]),
        ("PAD63_I1", wires::OUT_IO_WE_I1[1]),
        ("PAD63_I2", wires::OUT_IO_WE_I2[1]),
        ("PAD63_IK", wires::IMUX_IO_IK[1]),
        ("PAD63_OK", wires::IMUX_IO_OK[1]),
        ("PAD63_T", wires::IMUX_IO_T[1]),
        ("DEC_BA_2_I", wires::IMUX_CLB_C3_W),
        ("TBUF_BA_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_BA_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_BA_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_BA_1_T", wires::IMUX_TBUF_T[1]),
        ("LEFTT_SEG_1", wires::LONG_H[0]),        // HLL1
        ("LEFTT_SEG_4", wires::DEC_V[0]),         // LTX1
        ("LEFTT_SEG_5", wires::LONG_IO_V[0]),     // LVLL1
        ("LEFTT_SEG_6", wires::LONG_H[1]),        // HLL2
        ("LEFTT_SEG_7", wires::DEC_V[1]),         // LTX2
        ("LEFTT_SEG_8", wires::LONG_IO_V[1]),     // LVLL2
        ("LEFTT_SEG_9", wires::LONG_H[2]),        // HLL3
        ("LEFTT_SEG_12", wires::DEC_V[2]),        // LTX3
        ("LEFTT_SEG_13", wires::LONG_IO_V[2]),    // LVLL3
        ("LEFTT_SEG_14", wires::DOUBLE_IO_W1[3]), // LDV8
        ("LEFTT_SEG_15", wires::DOUBLE_IO_W0[3]), // LDV7T
        ("LEFTT_SEG_16", wires::DOUBLE_IO_W1[2]), // LDV6
        ("LEFTT_SEG_17", wires::DOUBLE_IO_W0[2]), // LDV5T
        ("LEFTT_SEG_19", wires::DEC_V[3]),        // LTX4
        ("LEFTT_SEG_21", wires::LONG_IO_V[3]),    // LVLL4
        ("LEFTT_SEG_22", wires::DOUBLE_IO_W1[1]), // LDV4
        ("LEFTT_SEG_23", wires::DOUBLE_IO_W0[1]), // LDV3T
        ("LEFTT_SEG_24", wires::DOUBLE_IO_W1[0]), // LDV2
        ("LEFTT_SEG_25", wires::DOUBLE_IO_W0[0]), // LDV1T
        ("LEFTT_SEG_33", wires::GCLK[3]),         // K4
        ("LEFTT_SEG_34", wires::GCLK[2]),         // K3
        ("LEFTT_SEG_35", wires::GCLK[1]),         // K2
        ("LEFTT_SEG_36", wires::GCLK[0]),         // K1
        ("LEFTT_SEG_60", wires::LONG_H[3]),       // HLL4
        ("LEFTT_SEG_61", wires::LONG_H[4]),       // HLL5
        ("LEFTT_SEG_62", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("LEFTT_SEG_63", wires::LONG_H[5]),       // HLL6
        ("LEFTT_SEG_65", wires::DOUBLE_H0[0]),    // DH1
        ("LEFTT_SEG_64", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("LEFTT_SEG_66", wires::OUT_IO_WE_I1_S1), // I1_2T
        ("LEFTT_SEG_67", wires::DOUBLE_IO_W2[0]), // LDV1
        ("LEFTT_SEG_68", wires::DOUBLE_IO_W2[3]), // LDV7
        ("LEFTT_SEG_69", wires::DOUBLE_IO_W2[2]), // LDV5
        ("LEFTT_SEG_70", wires::SINGLE_H[1]),     // H2
        // LEFTT_SEG_71: CE_2T
        ("LEFTT_SEG_72", wires::DOUBLE_H1[0]),    // DH2
        ("LEFTT_SEG_73", wires::DOUBLE_IO_W2[1]), // LDV3
        ("LEFTT_SEG_74", wires::OUT_IO_WE_I2_S1), // I2_2T
        ("LEFTT_SEG_75", wires::SINGLE_H[0]),     // H1
        ("LEFTT_SEG_76", wires::SINGLE_H[2]),     // H3
        // LEFTT_SEG_77: IK_2T
        ("LEFTT_SEG_78", wires::SINGLE_H[3]),  // H4
        ("LEFTT_SEG_79", wires::SINGLE_H[4]),  // H5
        ("LEFTT_SEG_80", wires::SINGLE_H[5]),  // H6
        ("LEFTT_SEG_81", wires::SINGLE_H[6]),  // H7
        ("LEFTT_SEG_82", wires::SINGLE_H[7]),  // H8
        ("LEFTT_SEG_83", wires::DOUBLE_H1[1]), // DH3
        ("LEFTT_SEG_84", wires::DOUBLE_H0[1]), // DH4
        // LEFTT_SEG_85: CLOCK_1_2

        // LEFTS
        ("PAD54_I1", wires::OUT_IO_WE_I1[0]),
        ("PAD54_I2", wires::OUT_IO_WE_I2[0]),
        ("PAD54_IK", wires::IMUX_IO_IK[0]),
        ("PAD54_OK", wires::IMUX_IO_OK[0]),
        ("PAD54_T", wires::IMUX_IO_T[0]),
        ("PAD53_I1", wires::OUT_IO_WE_I1[1]),
        ("PAD53_I2", wires::OUT_IO_WE_I2[1]),
        ("PAD53_IK", wires::IMUX_IO_IK[1]),
        ("PAD53_OK", wires::IMUX_IO_OK[1]),
        ("PAD53_T", wires::IMUX_IO_T[1]),
        ("DEC_HA_2_I", wires::IMUX_CLB_C3_W),
        ("TBUF_HA_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_HA_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_HA_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_HA_1_T", wires::IMUX_TBUF_T[1]),
        ("LEFTS_SEG_1", wires::LONG_H[0]),        // HLL1
        ("LEFTS_SEG_4", wires::DEC_V[0]),         // LTX1
        ("LEFTS_SEG_5", wires::LONG_IO_V[0]),     // LVLL1
        ("LEFTS_SEG_6", wires::LONG_H[1]),        // HLL2
        ("LEFTS_SEG_7", wires::DEC_V[1]),         // LTX2
        ("LEFTS_SEG_8", wires::LONG_IO_V[1]),     // LVLL2
        ("LEFTS_SEG_9", wires::LONG_H[2]),        // HLL3
        ("LEFTS_SEG_12", wires::DEC_V[2]),        // LTX3
        ("LEFTS_SEG_13", wires::LONG_IO_V[2]),    // LVLL3
        ("LEFTS_SEG_14", wires::DOUBLE_IO_W1[3]), // LDV8
        ("LEFTS_SEG_15", wires::DOUBLE_IO_W0[3]), // LDV7T
        ("LEFTS_SEG_16", wires::DOUBLE_IO_W1[2]), // LDV6
        ("LEFTS_SEG_17", wires::DOUBLE_IO_W0[2]), // LDV5T
        ("LEFTS_SEG_19", wires::DEC_V[3]),        // LTX4
        ("LEFTS_SEG_21", wires::LONG_IO_V[3]),    // LVLL4
        ("LEFTS_SEG_22", wires::DOUBLE_IO_W1[1]), // LDV4
        ("LEFTS_SEG_23", wires::DOUBLE_IO_W0[1]), // LDV3T
        ("LEFTS_SEG_24", wires::DOUBLE_IO_W1[0]), // LDV2
        ("LEFTS_SEG_25", wires::DOUBLE_IO_W0[0]), // LDV1T
        ("LEFTS_SEG_33", wires::GCLK[3]),         // K4
        ("LEFTS_SEG_34", wires::GCLK[2]),         // K3
        ("LEFTS_SEG_35", wires::GCLK[1]),         // K2
        ("LEFTS_SEG_36", wires::GCLK[0]),         // K1
        ("LEFTS_SEG_60", wires::LONG_H[3]),       // HLL4
        ("LEFTS_SEG_61", wires::LONG_H[4]),       // HLL5
        ("LEFTS_SEG_62", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("LEFTS_SEG_63", wires::LONG_H[5]),       // HLL6
        ("LEFTS_SEG_65", wires::DOUBLE_H0[0]),    // DH1
        ("LEFTS_SEG_64", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("LEFTS_SEG_66", wires::OUT_IO_WE_I1_S1), // I1_2T
        ("LEFTS_SEG_67", wires::DOUBLE_IO_W2[0]), // LDV1
        ("LEFTS_SEG_68", wires::DOUBLE_IO_W2[3]), // LDV7
        ("LEFTS_SEG_69", wires::DOUBLE_IO_W2[2]), // LDV5
        ("LEFTS_SEG_70", wires::DOUBLE_IO_W2[1]), // LDV3
        // LEFTS_SEG_71: CE_2T
        ("LEFTS_SEG_72", wires::DOUBLE_H1[0]),    // DH2
        ("LEFTS_SEG_74", wires::OUT_IO_WE_I2_S1), // I2_2T
        ("LEFTS_SEG_75", wires::SINGLE_H[0]),     // H1
        ("LEFTS_SEG_76", wires::SINGLE_H[1]),     // H2
        ("LEFTS_SEG_77", wires::SINGLE_H[2]),     // H3
        // LEFTS_SEG_78: IK_2T
        // LEFTS_SEG_79: OK_2T
        ("LEFTS_SEG_80", wires::SINGLE_H[3]),  // H4
        ("LEFTS_SEG_81", wires::SINGLE_H[4]),  // H5
        ("LEFTS_SEG_82", wires::SINGLE_H[5]),  // H6
        ("LEFTS_SEG_83", wires::SINGLE_H[6]),  // H7
        ("LEFTS_SEG_84", wires::SINGLE_H[7]),  // H8
        ("LEFTS_SEG_85", wires::DOUBLE_H1[1]), // DH3
        ("LEFTS_SEG_86", wires::DOUBLE_H0[1]), // DH4
        // LEFTS_SEG_87: CLOCK_1_2
        // LEFTSB
        ("I_BUFGS_BL_I_LEFTSB", wires::OUT_IO_CLKIN),
        ("PAD50_I1", wires::OUT_IO_WE_I1[0]),
        ("PAD50_I2", wires::OUT_IO_WE_I2[0]),
        ("PAD50_IK", wires::IMUX_IO_IK[0]),
        ("PAD50_OK", wires::IMUX_IO_OK[0]),
        ("PAD50_T", wires::IMUX_IO_T[0]),
        ("PAD49_I1", wires::OUT_IO_WE_I1[1]),
        ("PAD49_I2", wires::OUT_IO_WE_I2[1]),
        ("PAD49_IK", wires::IMUX_IO_IK[1]),
        ("PAD49_OK", wires::IMUX_IO_OK[1]),
        ("PAD49_T", wires::IMUX_IO_T[1]),
        ("DEC_JA_2_I", wires::IMUX_CLB_C3_W),
        ("TBUF_JA_2_I", wires::IMUX_TBUF_I[0]),
        ("TBUF_JA_2_T", wires::IMUX_TBUF_T[0]),
        ("TBUF_JA_1_I", wires::IMUX_TBUF_I[1]),
        ("TBUF_JA_1_T", wires::IMUX_TBUF_T[1]),
        ("LEFTSB_SEG_1", wires::LONG_H[0]),        // HLL1
        ("LEFTSB_SEG_4", wires::DEC_V[0]),         // LTX1
        ("LEFTSB_SEG_5", wires::LONG_IO_V[0]),     // LVLL1
        ("LEFTSB_SEG_6", wires::LONG_H[1]),        // HLL2
        ("LEFTSB_SEG_7", wires::DEC_V[1]),         // LTX2
        ("LEFTSB_SEG_8", wires::LONG_IO_V[1]),     // LVLL2
        ("LEFTSB_SEG_9", wires::LONG_H[2]),        // HLL3
        ("LEFTSB_SEG_12", wires::DEC_V[2]),        // LTX3
        ("LEFTSB_SEG_13", wires::LONG_IO_V[2]),    // LVLL3
        ("LEFTSB_SEG_14", wires::DOUBLE_IO_W1[3]), // LDV8
        ("LEFTSB_SEG_15", wires::DOUBLE_IO_W0[3]), // LDV7T
        ("LEFTSB_SEG_16", wires::DOUBLE_IO_W1[2]), // LDV6
        ("LEFTSB_SEG_17", wires::DOUBLE_IO_W0[2]), // LDV5T
        ("LEFTSB_SEG_19", wires::DEC_V[3]),        // LTX4
        ("LEFTSB_SEG_21", wires::LONG_IO_V[3]),    // LVLL4
        ("LEFTSB_SEG_22", wires::DOUBLE_IO_W1[1]), // LDV4
        ("LEFTSB_SEG_23", wires::DOUBLE_IO_W0[1]), // LDV3T
        ("LEFTSB_SEG_24", wires::DOUBLE_IO_W1[0]), // LDV2
        ("LEFTSB_SEG_25", wires::DOUBLE_IO_W0[0]), // LDV1T
        ("LEFTSB_SEG_33", wires::GCLK[3]),         // K4
        ("LEFTSB_SEG_34", wires::GCLK[2]),         // K3
        ("LEFTSB_SEG_35", wires::GCLK[1]),         // K2
        ("LEFTSB_SEG_36", wires::GCLK[0]),         // K1
        ("LEFTSB_SEG_60", wires::LONG_H[3]),       // HLL4
        ("LEFTSB_SEG_61", wires::LONG_H[4]),       // HLL5
        ("LEFTSB_SEG_62", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("LEFTSB_SEG_63", wires::LONG_H[5]),       // HLL6
        ("LEFTSB_SEG_65", wires::DOUBLE_H0[0]),    // DH1
        ("LEFTSB_SEG_64", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("LEFTSB_SEG_66", wires::OUT_IO_WE_I1_S1), // I1_2T
        ("LEFTSB_SEG_67", wires::DOUBLE_IO_W2[0]), // LDV1
        ("LEFTSB_SEG_68", wires::DOUBLE_IO_W2[3]), // LDV7
        ("LEFTSB_SEG_69", wires::DOUBLE_IO_W2[2]), // LDV5
        ("LEFTSB_SEG_70", wires::DOUBLE_IO_W2[1]), // LDV3
        // LEFTSB_SEG_71: CE_2T
        ("LEFTSB_SEG_72", wires::DOUBLE_H1[0]),    // DH2
        ("LEFTSB_SEG_74", wires::OUT_IO_WE_I2_S1), // I2_2T
        ("LEFTSB_SEG_75", wires::SINGLE_H[0]),     // H1
        ("LEFTSB_SEG_76", wires::SINGLE_H[1]),     // H2
        ("LEFTSB_SEG_77", wires::SINGLE_H[2]),     // H3
        // LEFTSB_SEG_78: IK_2T
        // LEFTSB_SEG_79: OK_2T
        ("LEFTSB_SEG_80", wires::SINGLE_H[3]),  // H4
        ("LEFTSB_SEG_81", wires::SINGLE_H[4]),  // H5
        ("LEFTSB_SEG_82", wires::SINGLE_H[5]),  // H6
        ("LEFTSB_SEG_83", wires::SINGLE_H[6]),  // H7
        ("LEFTSB_SEG_84", wires::SINGLE_H[7]),  // H8
        ("LEFTSB_SEG_85", wires::DOUBLE_H1[1]), // DH3
        ("LEFTSB_SEG_86", wires::DOUBLE_H0[1]), // DH4
        // LEFTSB_SEG_87: CLOCK_1_2

        // LR
        ("STARTUP_Q1Q4", wires::OUT_STARTUP_Q1Q4),
        ("STARTUP_Q2", wires::OUT_STARTUP_Q2),
        ("STARTUP_Q3", wires::OUT_STARTUP_Q3),
        ("STARTUP_DONEIN", wires::OUT_STARTUP_DONEIN),
        ("STARTUP_CLK", wires::IMUX_STARTUP_CLK),
        ("STARTUP_GSR", wires::IMUX_STARTUP_GSR),
        ("STARTUP_GTS", wires::IMUX_STARTUP_GTS),
        ("RDCLK_I", wires::IMUX_READCLK_I),
        ("BUFGP_BR_I", wires::IMUX_BUFG_V),
        ("BUFGS_BR_I", wires::IMUX_BUFG_H),
        ("BUFGP_BR_O", wires::BUFGLS[4]),
        ("BUFGS_BR_O", wires::BUFGLS[3]),
        ("I_BUFGP_BR_I", wires::OUT_IO_CLKIN_S),
        ("I_BUFGS_BR_I", wires::OUT_IO_CLKIN_E),
        ("TIE_KK_1_O", wires::TIE_0),
        ("LR_SEG_0", wires::DBUF_IO_H[0]),     // BDMUX_OUTER
        ("LR_SEG_1", wires::DOUBLE_IO_E1[0]),  // RDV1
        ("LR_SEG_2", wires::DBUF_IO_H[1]),     // BDMUX_INNER
        ("LR_SEG_3", wires::SINGLE_V[1]),      // V2
        ("LR_SEG_4", wires::DOUBLE_IO_S2[0]),  // BDH1
        ("LR_SEG_5", wires::DOUBLE_V1[0]),     // DV1
        ("LR_SEG_6", wires::SINGLE_V[0]),      // V1
        ("LR_SEG_7", wires::DOUBLE_IO_S1[0]),  // BDH2
        ("LR_SEG_8", wires::DOUBLE_IO_E1[1]),  // RDV3
        ("LR_SEG_9", wires::SINGLE_V[3]),      // V4
        ("LR_SEG_10", wires::DOUBLE_V0[0]),    // DV2
        ("LR_SEG_11", wires::DOUBLE_IO_S2[1]), // BDH3
        ("LR_SEG_12", wires::SINGLE_V[2]),     // V3
        ("LR_SEG_13", wires::DOUBLE_IO_S1[1]), // BDH4
        ("LR_SEG_14", wires::DOUBLE_IO_E1[2]), // RDV5
        ("LR_SEG_15", wires::DOUBLE_V0[1]),    // DV3
        ("LR_SEG_16", wires::SINGLE_V[5]),     // V6
        ("LR_SEG_17", wires::DOUBLE_IO_S2[2]), // BDH5
        ("LR_SEG_18", wires::DOUBLE_IO_S1[2]), // BDH6
        ("LR_SEG_19", wires::SINGLE_V[4]),     // V5
        ("LR_SEG_20", wires::DOUBLE_IO_E1[3]), // RDV7
        ("LR_SEG_21", wires::DOUBLE_V1[1]),    // DV4
        ("LR_SEG_22", wires::SINGLE_V[7]),     // V8
        ("LR_SEG_23", wires::DOUBLE_IO_S2[3]), // BDH7
        ("LR_SEG_24", wires::DOUBLE_IO_S1[3]), // BDH8
        ("LR_SEG_25", wires::SINGLE_V[6]),     // V7
        ("LR_SEG_26", wires::LONG_IO_V[2]),    // RVLL3
        ("LR_SEG_27", wires::LONG_IO_H[0]),    // BHLL1
        ("LR_SEG_28", wires::LONG_IO_V[0]),    // RVLL1
        ("LR_SEG_29", wires::LONG_V[0]),       // VLL1
        ("LR_SEG_30", wires::LONG_IO_V[3]),    // RVLL4
        ("LR_SEG_31", wires::LONG_IO_H[1]),    // BHLL2
        ("LR_SEG_32", wires::LONG_IO_V[1]),    // RVLL2
        ("LR_SEG_33", wires::LONG_V[3]),       // VLL4
        ("LR_SEG_34", wires::LONG_V[1]),       // VLL2
        ("LR_SEG_35", wires::LONG_IO_H[2]),    // BHLL3
        ("LR_SEG_36", wires::LONG_V[4]),       // VLL5
        ("LR_SEG_37", wires::LONG_V[2]),       // VLL3
        ("LR_SEG_38", wires::LONG_IO_H[3]),    // BHLL4
        ("LR_SEG_39", wires::LONG_V[5]),       // VLL6
        ("LR_SEG_41", wires::DEC_H[0]),        // BTX1
        ("LR_SEG_44", wires::DEC_H[1]),        // BTX2
        ("LR_SEG_46", wires::DEC_H[2]),        // BTX3
        ("LR_SEG_50", wires::DEC_H[3]),        // BTX4
        // LR_SEG_58 LOK_2
        // LR_SEG_59 LIK_2
        ("LR_SEG_63", wires::OUT_IO_SN_I1_E1), // LI1_2
        // LR_SEG_69 LO_2 [aka LCE_2]
        ("LR_SEG_76", wires::OUT_IO_SN_I2_E1), // LI2_2
        ("LR_SEG_72", wires::DEC_V[0]),        // RTX4
        ("LR_SEG_73", wires::DEC_V[1]),        // RTX3
        ("LR_SEG_74", wires::DEC_V[2]),        // RTX2
        ("LR_SEG_75", wires::DEC_V[3]),        // RTX1
        ("LR_SEG_77", wires::LONG_H[3]),       // HLL4
        ("LR_SEG_78", wires::DBUF_IO_V[0]),    // RDMUX_OUTER
        ("LR_SEG_79", wires::LONG_H[4]),       // HLL5
        ("LR_SEG_80", wires::DBUF_IO_V[1]),    // RDMUX_INNER
        ("LR_SEG_81", wires::LONG_H[5]),       // HLL6
        ("LR_SEG_82", wires::DOUBLE_H1[0]),    // DH1
        ("LR_SEG_83", wires::DOUBLE_IO_E0[0]), // RDV2
        ("LR_SEG_84", wires::OUT_IO_WE_I1_S1), // TI1_2
        ("LR_SEG_85", wires::DOUBLE_IO_E0[3]), // RDV8
        ("LR_SEG_86", wires::DOUBLE_IO_E0[2]), // RDV6
        ("LR_SEG_87", wires::DOUBLE_IO_E0[1]), // RDV4
        ("LR_SEG_88", wires::DOUBLE_H0[0]),    // DH2R
        // LR_SEG_89 TCE_2
        ("LR_SEG_91", wires::DOUBLE_H2[0]),    // DH2
        ("LR_SEG_92", wires::SINGLE_H[0]),     // H1R
        ("LR_SEG_93", wires::OUT_IO_WE_I2_S1), // TI2_2
        ("LR_SEG_94", wires::SINGLE_H_E[0]),   // H1
        ("LR_SEG_95", wires::SINGLE_H[1]),     // H2R
        ("LR_SEG_96", wires::SINGLE_H_E[1]),   // H2
        ("LR_SEG_97", wires::SINGLE_H[2]),     // H3R
        // LR_SEG_98 TOK_2
        // LR_SEG_99 TIK_2
        ("LR_SEG_100", wires::SINGLE_H_E[2]), // H3
        ("LR_SEG_101", wires::SINGLE_H[3]),   // H4R
        ("LR_SEG_102", wires::SINGLE_H_E[3]), // H4
        ("LR_SEG_103", wires::SINGLE_H[4]),   // H5R
        ("LR_SEG_104", wires::SINGLE_H_E[4]), // H5
        ("LR_SEG_105", wires::SINGLE_H[5]),   // H6R
        ("LR_SEG_106", wires::SINGLE_H_E[5]), // H6
        ("LR_SEG_107", wires::SINGLE_H[6]),   // H7R
        ("LR_SEG_108", wires::SINGLE_H_E[6]), // H7
        ("LR_SEG_109", wires::SINGLE_H[7]),   // H8R
        ("LR_SEG_110", wires::SINGLE_H_E[7]), // H8
        ("LR_SEG_111", wires::DOUBLE_H0[1]),  // DH3R
        ("LR_SEG_112", wires::DOUBLE_H2[1]),  // DH3
        ("LR_SEG_113", wires::DOUBLE_H1[1]),  // DH4
        ("LR_SEG_114", wires::DOUBLE_V2[1]),  // DV3T
        ("LR_SEG_115", wires::SINGLE_V_S[7]), // V8T
        ("LR_SEG_116", wires::SINGLE_V_S[6]), // V7T
        ("LR_SEG_117", wires::SINGLE_V_S[5]), // V6T
        ("LR_SEG_118", wires::SINGLE_V_S[4]), // V5T
        ("LR_SEG_119", wires::SINGLE_V_S[3]), // V4T
        ("LR_SEG_120", wires::SINGLE_V_S[2]), // V3T
        ("LR_SEG_121", wires::SINGLE_V_S[1]), // V2T
        ("LR_SEG_122", wires::SINGLE_V_S[0]), // V1T
        ("LR_SEG_123", wires::DOUBLE_V2[0]),  // DV2T
        // UR
        ("TDO_O", wires::IMUX_TDO_O),
        ("TDO_T", wires::IMUX_TDO_T),
        ("UPDATE_O", wires::OUT_UPDATE_O),
        ("OSC_F8M", wires::OUT_IO_WE_I1[1]),
        ("BUFGP_TR_I", wires::IMUX_BUFG_H),
        ("BUFGS_TR_I", wires::IMUX_BUFG_V),
        ("BUFGP_TR_O", wires::BUFGLS[6]),
        ("BUFGS_TR_O", wires::BUFGLS[5]),
        ("I_BUFGP_TR_I", wires::OUT_IO_CLKIN_E),
        ("I_BUFGS_TR_I", wires::OUT_IO_CLKIN_N),
        ("UR_SEG_0", wires::LONG_IO_V[0]),     // RVLL1
        ("UR_SEG_1", wires::LONG_H[0]),        // HLL1
        ("UR_SEG_2", wires::DEC_V[3]),         // RTX1
        ("UR_SEG_4", wires::OUT_IO_WE_I2[1]),  // OSC_OUT
        ("UR_SEG_5", wires::SINGLE_V[1]),      // V2
        ("UR_SEG_6", wires::LONG_IO_V[1]),     // RVLL2
        ("UR_SEG_7", wires::LONG_H[1]),        // HLL2
        ("UR_SEG_8", wires::DEC_V[2]),         // RTX2
        ("UR_SEG_9", wires::SINGLE_V[2]),      // V3
        ("UR_SEG_10", wires::LONG_IO_V[2]),    // RVLL3
        ("UR_SEG_11", wires::LONG_H[2]),       // HLL3
        ("UR_SEG_12", wires::DEC_V[1]),        // RTX3
        ("UR_SEG_13", wires::SINGLE_V[3]),     // V4
        ("UR_SEG_14", wires::DOUBLE_V1[1]),    // DV4
        ("UR_SEG_15", wires::OUT_IO_SN_I2_E1), // I2_2
        ("UR_SEG_16", wires::SINGLE_V[4]),     // V5
        ("UR_SEG_17", wires::SINGLE_V[0]),     // V1
        ("UR_SEG_18", wires::LONG_V[2]),       // VLL3
        ("UR_SEG_19", wires::LONG_V[1]),       // VLL2
        ("UR_SEG_20", wires::LONG_V[0]),       // VLL1
        ("UR_SEG_22", wires::DEC_V[0]),        // RTX4
        ("UR_SEG_26", wires::LONG_V[5]),       // VLL6
        ("UR_SEG_28", wires::LONG_V[4]),       // VLL5
        ("UR_SEG_29", wires::LONG_V[3]),       // VLL4
        ("UR_SEG_30", wires::DOUBLE_V0[0]),    // DV2
        ("UR_SEG_31", wires::DOUBLE_V0[1]),    // DV3
        ("UR_SEG_33", wires::DOUBLE_V1[0]),    // DV1
        // UR_SEG_35: CE_2
        ("UR_SEG_37", wires::SINGLE_V[6]),     // V7
        ("UR_SEG_42", wires::SINGLE_V[7]),     // V8
        ("UR_SEG_43", wires::OUT_IO_SN_I1_E1), // I1_2
        ("UR_SEG_44", wires::OUT_OSC_MUX1),    // OSC_IN
        ("UR_SEG_45", wires::SINGLE_V[5]),     // V6
        // UR_SEG_47: IK_2
        // UR_SEG_48: OK_2
        ("UR_SEG_54", wires::DEC_H[3]),        // TTX1
        ("UR_SEG_56", wires::DEC_H[2]),        // TTX2
        ("UR_SEG_59", wires::DEC_H[1]),        // TTX3
        ("UR_SEG_60", wires::DOUBLE_IO_E1[3]), // D8
        ("UR_SEG_61", wires::DOUBLE_IO_E2[3]), // D7
        ("UR_SEG_62", wires::DOUBLE_IO_E1[2]), // D6
        ("UR_SEG_63", wires::DOUBLE_IO_E2[2]), // D5
        ("UR_SEG_64", wires::DOUBLE_IO_E1[1]), // D4
        ("UR_SEG_65", wires::DOUBLE_IO_E2[1]), // D3
        ("UR_SEG_66", wires::DOUBLE_IO_E1[0]), // D2
        ("UR_SEG_67", wires::DOUBLE_IO_E2[0]), // D1
        ("UR_SEG_70", wires::DEC_H[0]),        // TTX4
        ("UR_SEG_71", wires::LONG_IO_H[0]),    // THLL1
        ("UR_SEG_72", wires::LONG_IO_V[3]),    // RVLL4
        ("UR_SEG_73", wires::LONG_IO_H[1]),    // THLL2
        ("UR_SEG_74", wires::LONG_IO_H[2]),    // THLL3
        ("UR_SEG_75", wires::LONG_IO_H[3]),    // THLL4
        ("UR_SEG_76", wires::DBUF_IO_H[0]),    // DMUX_OUTER
        ("UR_SEG_77", wires::DBUF_IO_H[1]),    // DMUX_INNER
        ("UR_SEG_78", wires::DOUBLE_IO_N0[0]), // D1L
        ("UR_SEG_79", wires::DOUBLE_IO_N0[1]), // D3L
        ("UR_SEG_80", wires::DOUBLE_IO_N0[2]), // D5L
        ("UR_SEG_81", wires::DOUBLE_IO_N0[3]), // D7L
        // LL
        ("RDBK_TRIG", wires::IMUX_RDBK_TRIG),
        ("RDBK_DATA", wires::OUT_RDBK_DATA),
        ("RDBK_RIP", wires::OUT_IO_SN_I2[1]),
        ("MD0_I", wires::OUT_MD0_I),
        ("MD1_O", wires::IMUX_IO_O1[1]),
        ("MD1_T", wires::IMUX_IO_IK[1]),
        ("MD2_I", wires::OUT_IO_SN_I1[1]),
        ("BUFGP_BL_I", wires::IMUX_BUFG_H),
        ("BUFGS_BL_I", wires::IMUX_BUFG_V),
        ("BUFGP_BL_O", wires::BUFGLS[2]),
        ("BUFGS_BL_O", wires::BUFGLS[1]),
        ("I_BUFGP_BL_I", wires::OUT_IO_CLKIN_W),
        ("I_BUFGS_BL_I", wires::OUT_IO_CLKIN_S),
        ("LL_SEG_0", wires::LONG_IO_V[2]),     // LVLL3
        ("LL_SEG_1", wires::LONG_IO_H[0]),     // BHLL1
        ("LL_SEG_2", wires::LONG_IO_V[0]),     // LVLL1
        ("LL_SEG_3", wires::LONG_IO_V[3]),     // LVLL4
        ("LL_SEG_4", wires::LONG_IO_H[1]),     // BHLL2
        ("LL_SEG_5", wires::LONG_IO_V[1]),     // LVLL2
        ("LL_SEG_6", wires::LONG_IO_H[2]),     // BHLL3
        ("LL_SEG_7", wires::LONG_IO_H[3]),     // BHLL4
        ("LL_SEG_8", wires::DEC_H[0]),         // BTX1
        ("LL_SEG_10", wires::DEC_H[1]),        // BTX2
        ("LL_SEG_12", wires::DEC_H[2]),        // BTX3
        ("LL_SEG_15", wires::DEC_H[3]),        // BTX4
        ("LL_SEG_21", wires::DOUBLE_IO_W1[3]), // D7
        ("LL_SEG_22", wires::DOUBLE_IO_S0[3]), // D8B
        ("LL_SEG_23", wires::DOUBLE_IO_W1[2]), // D5
        ("LL_SEG_24", wires::DOUBLE_IO_S0[2]), // D6B
        ("LL_SEG_25", wires::DOUBLE_IO_W1[1]), // D3
        ("LL_SEG_26", wires::DOUBLE_IO_S0[1]), // D4B
        ("LL_SEG_27", wires::DOUBLE_IO_W1[0]), // D1
        ("LL_SEG_28", wires::DOUBLE_IO_S0[0]), // D2B
        ("LL_SEG_41", wires::DEC_V[3]),        // LTX4
        ("LL_SEG_42", wires::DEC_V[2]),        // LTX3
        ("LL_SEG_43", wires::DEC_V[1]),        // LTX2
        ("LL_SEG_44", wires::DEC_V[0]),        // LTX1
        ("LL_SEG_45", wires::LONG_H[3]),       // HLL4
        ("LL_SEG_46", wires::LONG_H[4]),       // HLL5
        ("LL_SEG_47", wires::DBUF_IO_V[0]),    // DMUX_OUTER
        ("LL_SEG_48", wires::LONG_H[5]),       // HLL6
        ("LL_SEG_49", wires::DBUF_IO_V[1]),    // DMUX_INNER
        ("LL_SEG_50", wires::DOUBLE_H0[0]),    // DH1
        ("LL_SEG_51", wires::OUT_IO_WE_I1_S1), // I1_2
        ("LL_SEG_52", wires::DOUBLE_IO_W2[0]), // D2
        ("LL_SEG_53", wires::DOUBLE_IO_W2[3]), // D8
        ("LL_SEG_54", wires::DOUBLE_IO_W2[2]), // D6
        ("LL_SEG_55", wires::SINGLE_H[1]),     // H2
        // LL_SEG_56 CE_2
        ("LL_SEG_57", wires::DOUBLE_H1[0]),    // DH2
        ("LL_SEG_59", wires::DOUBLE_IO_W2[1]), // D4
        ("LL_SEG_60", wires::OUT_IO_WE_I2_S1), // I2_2
        ("LL_SEG_61", wires::SINGLE_H[0]),     // H1
        // LL_SEG_63 IK_2
        // LL_SEG_64 OK_2
        ("LL_SEG_62", wires::SINGLE_H[2]),  // H3
        ("LL_SEG_65", wires::SINGLE_H[3]),  // H4
        ("LL_SEG_66", wires::SINGLE_H[4]),  // H5
        ("LL_SEG_67", wires::SINGLE_H[5]),  // H6
        ("LL_SEG_68", wires::SINGLE_H[6]),  // H7
        ("LL_SEG_69", wires::SINGLE_H[7]),  // H8
        ("LL_SEG_70", wires::DOUBLE_H1[1]), // DH3
        ("LL_SEG_71", wires::DOUBLE_H0[1]), // DH4
        // UL
        ("BSCAN_SEL2", wires::OUT_IO_SN_I1[1]),
        ("BSCAN_DRCK", wires::OUT_IO_SN_I2[1]),
        ("BSCAN_SEL1", wires::OUT_IO_WE_I1[1]),
        ("BSCAN_IDLE", wires::OUT_IO_WE_I2[1]),
        ("BSCAN_TDO1", wires::IMUX_BSCAN_TDO1),
        ("BSCAN_TDO2", wires::IMUX_BSCAN_TDO2),
        ("BUFGP_TL_I", wires::IMUX_BUFG_V),
        ("BUFGS_TL_I", wires::IMUX_BUFG_H),
        ("BUFGP_TL_O", wires::BUFGLS[0]),
        ("BUFGS_TL_O", wires::BUFGLS[7]),
        ("I_BUFGP_TL_I", wires::OUT_IO_CLKIN_N),
        ("I_BUFGS_TL_I", wires::OUT_IO_CLKIN_W),
        ("UL_SEG_3", wires::LONG_H[0]),        // HLL1
        ("UL_SEG_5", wires::DEC_V[0]),         // LTX1
        ("UL_SEG_6", wires::LONG_IO_V[0]),     // LVLL1
        ("UL_SEG_7", wires::LONG_H[1]),        // HLL2
        ("UL_SEG_8", wires::DEC_V[1]),         // LTX2
        ("UL_SEG_9", wires::LONG_IO_V[1]),     // LVLL2
        ("UL_SEG_10", wires::LONG_H[2]),       // HLL3
        ("UL_SEG_11", wires::DEC_V[2]),        // LTX3
        ("UL_SEG_12", wires::LONG_IO_V[2]),    // LVLL3
        ("UL_SEG_14", wires::DEC_V[3]),        // LTX4
        ("UL_SEG_25", wires::DEC_H[3]),        // TTX1
        ("UL_SEG_29", wires::DOUBLE_IO_N2[3]), // UL_D8
        ("UL_SEG_30", wires::DOUBLE_IO_N1[3]), // UL_D7
        ("UL_SEG_31", wires::DOUBLE_IO_N2[2]), // UL_D6
        ("UL_SEG_32", wires::DOUBLE_IO_N1[2]), // UL_D5
        ("UL_SEG_33", wires::DOUBLE_IO_N2[1]), // UL_D4
        ("UL_SEG_34", wires::DOUBLE_IO_N1[1]), // UL_D3
        ("UL_SEG_35", wires::DOUBLE_IO_N2[0]), // UL_D2
        ("UL_SEG_36", wires::DOUBLE_IO_N1[0]), // UL_D1
        ("UL_SEG_37", wires::DEC_H[2]),        // TTX2
        ("UL_SEG_39", wires::DEC_H[1]),        // TTX3
        ("UL_SEG_41", wires::DEC_H[0]),        // TTX4
        ("UL_SEG_44", wires::LONG_IO_H[0]),    // THLL1
        ("UL_SEG_45", wires::LONG_IO_V[3]),    // LVLL4
        ("UL_SEG_46", wires::LONG_IO_H[1]),    // THLL2
        ("UL_SEG_47", wires::LONG_IO_H[2]),    // THLL3
        ("UL_SEG_48", wires::LONG_IO_H[3]),    // THLL4
        // CLKL
        ("CLKL_SEG_0", wires::GCLK[0]),
        ("CLKL_SEG_1", wires::BUFGLS[0]),
        ("CLKL_SEG_2", wires::GCLK[1]),
        ("CLKL_SEG_3", wires::BUFGLS[2]),
        ("CLKL_SEG_4", wires::GCLK[2]),
        ("CLKL_SEG_5", wires::BUFGLS[4]),
        ("CLKL_SEG_6", wires::GCLK[3]),
        ("CLKL_SEG_7", wires::BUFGLS[6]),
        ("CLKL_SEG_24", wires::BUFGLS[1]),
        ("CLKL_SEG_25", wires::BUFGLS[3]),
        ("CLKL_SEG_26", wires::BUFGLS[5]),
        ("CLKL_SEG_27", wires::BUFGLS[7]),
        // CLKR
        ("CLKR_SEG_0", wires::GCLK[0]),
        ("CLKR_SEG_1", wires::BUFGLS[0]),
        ("CLKR_SEG_2", wires::GCLK[1]),
        ("CLKR_SEG_3", wires::BUFGLS[2]),
        ("CLKR_SEG_4", wires::GCLK[2]),
        ("CLKR_SEG_5", wires::BUFGLS[4]),
        ("CLKR_SEG_6", wires::GCLK[3]),
        ("CLKR_SEG_7", wires::BUFGLS[6]),
        ("CLKR_SEG_36", wires::BUFGLS[1]),
        ("CLKR_SEG_37", wires::BUFGLS[3]),
        ("CLKR_SEG_38", wires::BUFGLS[5]),
        ("CLKR_SEG_39", wires::BUFGLS[7]),
        // CLKH
        ("CLKH_SEG_0", wires::GCLK[0]),
        ("CLKH_SEG_1", wires::BUFGLS[0]),
        ("CLKH_SEG_2", wires::GCLK[1]),
        ("CLKH_SEG_3", wires::BUFGLS[2]),
        ("CLKH_SEG_4", wires::GCLK[2]),
        ("CLKH_SEG_5", wires::BUFGLS[4]),
        ("CLKH_SEG_6", wires::GCLK[3]),
        ("CLKH_SEG_7", wires::BUFGLS[6]),
        ("CLKH_SEG_20", wires::BUFGLS[1]),
        ("CLKH_SEG_21", wires::BUFGLS[3]),
        ("CLKH_SEG_22", wires::BUFGLS[5]),
        ("CLKH_SEG_23", wires::BUFGLS[7]),
    ]
}
