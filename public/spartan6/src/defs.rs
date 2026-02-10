use prjcombine_tablegen::target_defs;

target_defs! {
    enum SLICE_MUX_ADI1 { ALT, AX }
    enum SLICE_MUX_BDI1 { ALT, BX }
    enum SLICE_MUX_CDI1 { ALT, CX }
    enum SLICE_MUX_WE { WE, CE }
    enum SLICE_RAMMODE { NONE, RAM64, RAM32, SRL32, SRL16 }
    enum SLICE_CYINIT { PRECYINIT, CIN }
    enum SLICE_PRECYINIT { CONST_0, CONST_1, AX }
    enum SLICE_MUX_ACY0 { AX, O5 }
    enum SLICE_MUX_BCY0 { BX, O5 }
    enum SLICE_MUX_CCY0 { CX, O5 }
    enum SLICE_MUX_DCY0 { DX, O5 }
    enum SLICE_MUX_AFF { AX, O6, O5, XOR, CY, F7 }
    enum SLICE_MUX_BFF { BX, O6, O5, XOR, CY, F8 }
    enum SLICE_MUX_CFF { CX, O6, O5, XOR, CY, F7 }
    enum SLICE_MUX_DFF { DX, O6, O5, XOR, CY, MC31 }
    enum SLICE_MUX_AOUT { NONE, A5Q, O6, O5, XOR, CY, F7 }
    enum SLICE_MUX_BOUT { NONE, B5Q, O6, O5, XOR, CY, F8 }
    enum SLICE_MUX_COUT { NONE, C5Q, O6, O5, XOR, CY, F7 }
    enum SLICE_MUX_DOUT { NONE, D5Q, O6, O5, XOR, CY, MC31 }
    bel_class SLICE {
        input A1, A2, A3, A4, A5, A6;
        input B1, B2, B3, B4, B5, B6;
        input C1, C2, C3, C4, C5, C6;
        input D1, D2, D3, D4, D5, D6;
        input AX, BX, CX, DX;
        input AI, BI, CI, DI;
        input CLK, SR, CE, WE;
        output A, B, C, D;
        output AQ, BQ, CQ, DQ;
        output AMUX, BMUX, CMUX, DMUX;

        attribute A6LUT, B6LUT, C6LUT, D6LUT: bitvec[64];

        // SLICEM only
        attribute MUX_ADI1: SLICE_MUX_ADI1;
        attribute MUX_BDI1: SLICE_MUX_BDI1;
        attribute MUX_CDI1: SLICE_MUX_CDI1;
        attribute MUX_WE: SLICE_MUX_WE;

        // SLICEM only
        attribute ARAMMODE: SLICE_RAMMODE;
        attribute BRAMMODE: SLICE_RAMMODE;
        attribute CRAMMODE: SLICE_RAMMODE;
        attribute DRAMMODE: SLICE_RAMMODE;
        attribute WA7USED: bool;
        attribute WA8USED: bool;

        // SLICEL/SLICEM only
        attribute PRECYINIT: SLICE_PRECYINIT;
        attribute CYINIT: SLICE_CYINIT;
        attribute MUX_ACY0: SLICE_MUX_ACY0;
        attribute MUX_BCY0: SLICE_MUX_BCY0;
        attribute MUX_CCY0: SLICE_MUX_CCY0;
        attribute MUX_DCY0: SLICE_MUX_DCY0;

        // restricted to [ABCD]X, O6 on SLICEX
        attribute MUX_AFF: SLICE_MUX_AFF;
        attribute MUX_BFF: SLICE_MUX_BFF;
        attribute MUX_CFF: SLICE_MUX_CFF;
        attribute MUX_DFF: SLICE_MUX_DFF;

        attribute FF_LATCH: bool;
        attribute FF_SR_ENABLE: bool;
        attribute FF_SR_SYNC: bool;
        attribute FF_CE_ENABLE: bool;

        attribute AFFSRINIT, BFFSRINIT, CFFSRINIT, DFFSRINIT: bitvec[1];
        attribute A5FFSRINIT, B5FFSRINIT, C5FFSRINIT, D5FFSRINIT: bitvec[1];

        // restricted to [ABCD]5Q, O5 on SLICEX
        attribute MUX_AOUT: SLICE_MUX_AOUT;
        attribute MUX_BOUT: SLICE_MUX_BOUT;
        attribute MUX_COUT: SLICE_MUX_COUT;
        attribute MUX_DOUT: SLICE_MUX_DOUT;
    }

    enum BRAM_RAM_MODE { SP, TDP, SDP }
    enum BRAM_DATA_WIDTH { _0, _1, _2, _4, _9, _18, _36 }
    enum BRAM_WRITE_MODE { WRITE_FIRST, READ_FIRST, NO_CHANGE }
    enum BRAM_RSTTYPE { SYNC, ASYNC }
    enum BRAM_RST_PRIORITY { SR, CE }
    // The 9kbit blockram.  Two of them in the same tile can be put together to
    // make a 18kbit blockram (the two halves are much more independent than
    // on the virtex series, and so will be treated as separate bels).
    bel_class BRAM {
        input CLKA, CLKB, ENA, ENB, RSTA, RSTB, REGCEA, REGCEB;
        input ADDRA[13], ADDRB[13];
        input WEA[2], WEB[2];
        input DIA[16], DIB[16];
        input DIPA[2], DIPB[2];
        output DOA[16], DOB[16];
        output DOPA[2], DOPB[2];

        // for SDP mode:
        // - for control signals, port A is write, port B is read (opposite from virtex!);
        // - DIA/DIPA/WEA become low bits, DIB/DIPB/WEB become the high bits of write port
        // - DOA/DOPA become low bits, DOB/DOPB become the high bits of read port

        // when combined together to a 18-kbit blockram:
        // - for data and WE: BRAM[0] provides low bits and BRAM[1] provides high bits of the combined bus
        // - for CLK, EN, RST, REGCE, and ADDR: BRAM[0] inputs are used
        // - for all data widths other than 36, BRAM[1].ADDR[0] provides the extra address bit used
        //   to select between the two BRAMs (0 being BRAM[0], 1 being BRAM[1])
        // - thus, BRAM[1].ADDR[0] can be simply stuffed in between BRAM[0].ADDR[3] and BRAM[0].ADDR[4]
        //   as address bit 4 (appropriately swizzling DATA and DATAP of course)
        // - ISE, in its infinite wisdom, does something more batshit (avoiding the swizzle unless
        //   strictly necessary, ie. when any port has width of 36)

        // NOTE: those are not represented via bel pin inversions, because in RAMB16 mode they need
        // to be set on *both* bels, including the one that doesn't have control inputs connected
        attribute CLKA_INV, CLKB_INV: bool;
        attribute ENA_INV, ENB_INV: bool;
        attribute RSTA_INV, RSTB_INV: bool;
        attribute REGCEA_INV, REGCEB_INV: bool;

        attribute COMBINE: bool;
        attribute RAM_MODE: BRAM_RAM_MODE;
        attribute DATA_WIDTH_A, DATA_WIDTH_B: BRAM_DATA_WIDTH;
        attribute WRITE_MODE_A, WRITE_MODE_B: BRAM_WRITE_MODE;
        attribute DOA_REG, DOB_REG: bool;
        attribute EN_RSTRAM_A, EN_RSTRAM_B: bool;
        attribute RSTTYPE_A, RSTTYPE_B: BRAM_RSTTYPE;
        attribute RST_PRIORITY_A, RST_PRIORITY_B: BRAM_RST_PRIORITY;
        attribute INIT_A, INIT_B: bitvec[18];
        attribute SRVAL_A, SRVAL_B: bitvec[18];

        // ???
        attribute BW_EN_A, BW_EN_B: bool;
        attribute DDEL_A, DDEL_B: bitvec[3];
        attribute WDEL_A, WDEL_B: bitvec[3];
        attribute EN_WEAK_WRITE_A, EN_WEAK_WRITE_B: bool;
        attribute WEAK_WRITE_VAL_A, WEAK_WRITE_VAL_B: bitvec[1];

        attribute DATA: bitvec[0x2000];
        attribute DATAP: bitvec[0x400];
    }

    enum DSP_B_INPUT { DIRECT, CASCADE }
    enum DSP_CARRYINSEL { CARRYIN, OPMODE5 }
    bel_class DSP {
        input A[18];
        input B[18];
        input C[48];
        input D[18];
        input OPMODE[8];
        input CLK;
        input CEA, CEB, CEC, CED, CEOPMODE, CECARRYIN, CEM, CEP;
        input RSTA, RSTB, RSTC, RSTD, RSTOPMODE, RSTCARRYIN, RSTM, RSTP;
        output M[36];
        output P[48];
        output CARRYOUTF;

        attribute B_INPUT: DSP_B_INPUT;
        attribute CARRYINSEL: DSP_CARRYINSEL;
        attribute A0REG: bool;
        attribute A1REG: bool;
        attribute B0REG: bool;
        attribute B1REG: bool;
        attribute CREG: bool;
        attribute DREG: bool;
        attribute MREG: bool;
        attribute PREG: bool;
        attribute OPMODEREG: bool;
        attribute CARRYINREG: bool;
        attribute CARRYOUTREG: bool;
        attribute RSTTYPE: BRAM_RSTTYPE;
    }

    enum ILOGIC_MUX_D { IOB_I, OTHER_IOB_I }
    enum ILOGIC_MUX_Q { NETWORKING, NETWORKING_PIPELINED, RETIMED, SHIFT_REGISTER }
    enum ILOGIC_MUX_SR { INT, OLOGIC_SR }
    enum ILOGIC_DATA_WIDTH { _1, _2, _3, _4, _5, _6, _7, _8 }
    enum ILOGIC_MUX_TSBYPASS { GND, T }
    bel_class ILOGIC {
        output FABRICOUT;
        input CLK, IOCE;
        input CLKDIV;
        input SR, REV;
        input CE0;
        input BITSLIP;
        output Q1, Q2, Q3, Q4;
        output INCDEC;
        output VALID;

        // special output to BUFIO2 and BUFIO2FB only
        output DFB;
        // special outputs to BUFIO2FB only
        output CFB0;
        output CFB1;

        // TODO: attrs
        attribute ENABLE: bool;
        attribute DDR: bool;
        attribute IOCE_ENABLE: bool;

        attribute FFI_INIT: bitvec[1];
        attribute FFI_SRVAL: bitvec[1];
        attribute FFI_LATCH: bool;
        attribute FFI_SR_ENABLE: bool;
        attribute FFI_SR_SYNC: bool;
        attribute FFI_REV_ENABLE: bool;
        attribute FFI_CE_ENABLE: bool;

        attribute FFI_DELAY_ENABLE: bool;
        attribute I_DELAY_ENABLE: bool;

        attribute MUX_TSBYPASS: ILOGIC_MUX_TSBYPASS;
        attribute MUX_D: ILOGIC_MUX_D;
        attribute MUX_SR: ILOGIC_MUX_SR;

        attribute DATA_WIDTH_START: ILOGIC_DATA_WIDTH;
        attribute DATA_WIDTH_RELOAD: ILOGIC_DATA_WIDTH;
        attribute BITSLIP_ENABLE: bool;
        attribute CASCADE_ENABLE: bool;
        attribute MUX_Q1: ILOGIC_MUX_Q;
        attribute MUX_Q2: ILOGIC_MUX_Q;
        attribute MUX_Q3: ILOGIC_MUX_Q;
        attribute MUX_Q4: ILOGIC_MUX_Q;
        attribute ROW1_CLK_ENABLE: bool;
        attribute ROW2_CLK_ENABLE: bool;
        attribute ROW3_CLK_ENABLE: bool;
        attribute ROW4_CLK_ENABLE: bool;
    }

    enum OLOGIC_MUX_IN { INT, MCB }
    enum OLOGIC_MUX_OCE { INT, PCI_CE }
    enum OLOGIC_MUX_SR { GND, INT }
    enum OLOGIC_MUX_REV { GND, INT }
    enum OLOGIC_MUX_TRAIN { GND, INT, MCB }
    enum OLOGIC_MUX_O { D1, FFO }
    enum OLOGIC_MUX_T { T1, FFT }
    enum OLOGIC_OUTPUT_MODE { SINGLE_ENDED, DIFFERENTIAL }
    bel_class OLOGIC {
        input CLK, IOCE;
        input CLKDIV;
        input SR, REV;
        input OCE, TCE;
        input D1, D2, D3, D4;
        input T1, T2, T3, T4;
        input TRAIN;

        attribute ENABLE: bool;
        attribute IOCE_ENABLE: bool;
        attribute DDR_OPPOSITE_EDGE: bool;

        attribute FFO_INIT: bitvec[1];
        attribute FFO_SRVAL: bitvec[1];
        attribute FFO_LATCH: bool;
        attribute FFO_RANK1_BYPASS: bool;
        attribute FFO_RANK1_CLK_ENABLE: bool;
        attribute FFO_RANK2_CLK_ENABLE: bool;
        attribute FFO_SR_SYNC: bool;
        attribute FFO_SR_ENABLE: bool;
        attribute FFO_REV_ENABLE: bool;
        attribute FFO_CE_ENABLE: bool;
        attribute FFO_CE_OR_DDR: bool;

        attribute FFT_INIT: bitvec[1];
        attribute FFT_SRVAL: bitvec[1];
        attribute FFT_LATCH: bool;
        attribute FFT_RANK1_BYPASS: bool;
        attribute FFT_RANK1_CLK_ENABLE: bool;
        attribute FFT_RANK2_CLK_ENABLE: bool;
        attribute FFT_SR_SYNC: bool;
        attribute FFT_SR_ENABLE: bool;
        attribute FFT_REV_ENABLE: bool;
        attribute FFT_CE_ENABLE: bool;
        attribute FFT_CE_OR_DDR: bool;

        attribute MUX_IN_O: OLOGIC_MUX_IN;
        attribute MUX_IN_T: OLOGIC_MUX_IN;
        attribute MUX_OCE: OLOGIC_MUX_OCE;
        attribute MUX_SR: OLOGIC_MUX_SR;
        attribute MUX_REV: OLOGIC_MUX_REV;
        attribute MUX_TRAIN: OLOGIC_MUX_TRAIN;

        attribute MUX_O: OLOGIC_MUX_O;
        attribute MUX_T: OLOGIC_MUX_T;

        attribute CASCADE_ENABLE: bool;
        attribute OUTPUT_MODE: OLOGIC_OUTPUT_MODE;
        attribute TRAIN_PATTERN: bitvec[4];

        attribute MISR_ENABLE_CLK: bool;
        attribute MISR_ENABLE_DATA: bool;
        attribute MISR_RESET: bool;
    }

    enum IOLOGIC_COUNTER_WRAPAROUND { WRAPAROUND, STAY_AT_LIMIT }
    enum IODELAY_DELAY_SRC { IO, ODATAIN, IDATAIN }
    enum IODELAY_IDELAY_MODE { NORMAL, PCI }
    enum IODELAY_CHANGE { CHANGE_ON_CLOCK, CHANGE_ON_DATA }
    enum IODELAY_MODE { IODRP2, IODELAY2, IODRP2_MCB }
    bel_class IODELAY {
        input IOCLK;
        input RST;
        input CAL;
        input CE;
        input CIN;
        input CLK;
        input INC;
        output BUSY;
        output LOAD;
        output RCLK;

        // special outputs to BUFIO2 only
        output DQSOUTP, DQSOUTN;

        attribute MODE: IODELAY_MODE;
        attribute COUNTER_WRAPAROUND: IOLOGIC_COUNTER_WRAPAROUND;
        attribute DELAYCHAIN_OSC: bool;
        attribute DELAYCHAIN_OSC_OR_ODATAIN_LP_OR_IDRP2_MCB: bitvec[2];
        attribute DELAY_SRC: IODELAY_DELAY_SRC;
        attribute DIFF_PHASE_DETECTOR: bool;
        attribute CIN_ENABLE: bitvec[3];
        attribute ODATAIN_ENABLE: bool;
        attribute IDELAY_FIXED: bool;
        attribute IDELAY_FROM_HALF_MAX: bool;
        attribute IDELAY_MODE: IODELAY_IDELAY_MODE;
        attribute IODELAY_CHANGE: IODELAY_CHANGE;
        attribute LUMPED_DELAY: bool;
        attribute LUMPED_DELAY_SELECT: bool;
        attribute PLUS1: bool;
        attribute TEST_GLITCH_FILTER: bool;
        attribute TEST_PCOUNTER: bool;
        attribute TEST_NCOUNTER: bool;
        attribute EVENT_SEL: bitvec[2];

        attribute CAL_DELAY_MAX: bitvec[8];
        attribute IDELAY_VALUE_P: bitvec[8];
        attribute IDELAY_VALUE_N: bitvec[8];
        attribute ODELAY_VALUE_P: bitvec[8];
        attribute ODELAY_VALUE_N: bitvec[8];

        attribute DRP_ADDR: bitvec[5];
        attribute DRP06: bitvec[8];
        attribute DRP07: bitvec[8];
    }

    enum IOI_DDR_ALIGNMENT { NONE, CLK0, CLK1 }
    bel_class IOI_DDR {
        input CLK0, CLK1;
        output CLK, IOCE;
        attribute ENABLE: bitvec[2];
        attribute ALIGNMENT: IOI_DDR_ALIGNMENT;
    }

    bel_class MISC_IOI {
        attribute MEM_PLL_DIV_EN: bool;
        attribute MEM_PLL_POL_SEL: MCB_MEM_PLL_POL_SEL;
        attribute DRP_ENABLE: bool;
        attribute DRP_FROM_MCB: bool;
        attribute ENFFSCAN_DRP: bitvec[2];
        attribute DRP_MCB_ADDRESS: bitvec[4];
        // ????? I hate this FPGA
        attribute DIFF_PHASE_DETECTOR: bool;
    }

    enum IOB_DIFF_MODE { NONE, LVDS, TMDS }
    enum IOB_PULL { NONE, PULLUP, PULLDOWN, KEEPER }
    enum IOB_SUSPEND {
        _3STATE,
        DRIVE_LAST_VALUE,
        _3STATE_PULLDOWN,
        _3STATE_PULLUP,
        _3STATE_KEEPER,
        _3STATE_OCT_ON,
    }
    enum IOB_IBUF_MODE { NONE, LOOPBACK_T, LOOPBACK_O, CMOS_VCCINT, CMOS_VCCO, VREF, DIFF, CMOS_VCCAUX}
    bel_class IOB {
        // normally nonroutable (bolted straight to ILOGIC), but exposed for the dedicated clock
        // pads that need routing to BUFIO and clock spines.
        output I;

        pad PAD: inout;

        attribute PDRIVE: bitvec[6];
        attribute PTERM: bitvec[6];
        attribute NDRIVE: bitvec[7];
        attribute NTERM: bitvec[7];
        attribute TML: bool;
        attribute PSLEW: bitvec[4];
        attribute NSLEW: bitvec[4];
        attribute DIFF_TERM: bool;
        attribute DIFF_OUTPUT_ENABLE: bool;
        attribute LVDS_GROUP: bitvec[1];
        attribute DIFF_MODE: IOB_DIFF_MODE;
        attribute PRE_EMPHASIS: bool;
        attribute OUTPUT_LOW_VOLTAGE: bool;
        attribute PCI_CLAMP: bool;
        attribute PULL: IOB_PULL;
        attribute SUSPEND: IOB_SUSPEND;
        attribute IBUF_MODE: IOB_IBUF_MODE;
        attribute VREF_HV: bool;
        attribute PCI_INPUT: bool;
        attribute I_INV: bool;
        attribute VREF: bool;
        attribute OUTPUT_ENABLE: bool;
    }

    table IOB_DATA {
        field PDRIVE: bitvec[6];
        field NDRIVE_2V5: bitvec[7];
        field NDRIVE_3V3: bitvec[7];
        field PSLEW: bitvec[4];
        field NSLEW: bitvec[4];

        row OFF;
        row IN_TERM;
        row SLEW_SLOW, SLEW_FAST, SLEW_QUIETIO;

        row LVCMOS12_2, LVCMOS12_4, LVCMOS12_6, LVCMOS12_8, LVCMOS12_12;
        row LVCMOS15_2, LVCMOS15_4, LVCMOS15_6, LVCMOS15_8, LVCMOS15_12, LVCMOS15_16;
        row LVCMOS18_2, LVCMOS18_4, LVCMOS18_6, LVCMOS18_8, LVCMOS18_12, LVCMOS18_16, LVCMOS18_24;
        row LVCMOS25_2, LVCMOS25_4, LVCMOS25_6, LVCMOS25_8, LVCMOS25_12, LVCMOS25_16, LVCMOS25_24;
        row LVCMOS33_2, LVCMOS33_4, LVCMOS33_6, LVCMOS33_8, LVCMOS33_12, LVCMOS33_16, LVCMOS33_24;
        row LVTTL_2, LVTTL_4, LVTTL_6, LVTTL_8, LVTTL_12, LVTTL_16, LVTTL_24;
        row MOBILE_DDR;
        row SDIO;
        row I2C, SMBUS;
        row PCI33_3, PCI66_3;

        row DIFF_MOBILE_DDR;
        row BLVDS_25;
        row DISPLAY_PORT;
        row TML_33;

        row HSTL_I, HSTL_II, HSTL_III;
        row HSTL_I_18, HSTL_II_18, HSTL_III_18;
        row SSTL15_II;
        row SSTL18_I, SSTL18_II;
        row SSTL2_I, SSTL2_II;
        row SSTL3_I, SSTL3_II;

        row UNTUNED_25_1V2;
        row UNTUNED_25_1V5;
        row UNTUNED_25_1V8;
        row UNTUNED_25_2V5;
        row UNTUNED_25_3V3;
        row UNTUNED_50_1V2;
        row UNTUNED_50_1V5;
        row UNTUNED_50_1V8;
        row UNTUNED_50_2V5;
        row UNTUNED_50_3V3;
        row UNTUNED_75_1V2;
        row UNTUNED_75_1V5;
        row UNTUNED_75_1V8;
        row UNTUNED_75_2V5;
        row UNTUNED_75_3V3;
    }

    table LVDSBIAS {
        field LVDSBIAS: bitvec[12];
        row OFF;
        row LVDS_25, LVDS_33;
        row MINI_LVDS_25, MINI_LVDS_33;
        row RSDS_25, RSDS_33;
        row PPDS_25, PPDS_33;
        row TMDS_33, TML_33;
    }

    table IOB_TERM {
        field PTERM_2V5: bitvec[6];
        field PTERM_3V3: bitvec[6];
        field NTERM_2V5: bitvec[7];
        field NTERM_3V3: bitvec[7];

        row OFF;
        row TML_33;
        row UNTUNED_SPLIT_25_1V2;
        row UNTUNED_SPLIT_25_1V5;
        row UNTUNED_SPLIT_25_1V8;
        row UNTUNED_SPLIT_25_2V5;
        row UNTUNED_SPLIT_25_3V3;
        row UNTUNED_SPLIT_50_1V2;
        row UNTUNED_SPLIT_50_1V5;
        row UNTUNED_SPLIT_50_1V8;
        row UNTUNED_SPLIT_50_2V5;
        row UNTUNED_SPLIT_50_3V3;
        row UNTUNED_SPLIT_75_1V2;
        row UNTUNED_SPLIT_75_1V5;
        row UNTUNED_SPLIT_75_1V8;
        row UNTUNED_SPLIT_75_2V5;
        row UNTUNED_SPLIT_75_3V3;
    }

    enum DCM_MODE { DCM, DCM_CLKGEN }
    enum DCM_CLKDV_MODE { HALF, INT }
    enum DCM_FREQUENCY_MODE { LOW, HIGH }
    enum DCM_CLKOUT_PHASE_SHIFT { MISSING, NONE, FIXED, VARIABLE }
    enum DCM_CLKFXDV_DIVIDE { NONE, _32, _16, _8, _4, _2 }
    enum DCM_SPREAD_SPECTRUM { MISSING, NONE, DCM, CENTER_HIGH_SPREAD, CENTER_LOW_SPREAD, VIDEO_LINK_M0, VIDEO_LINK_M1, VIDEO_LINK_M2 }
    bel_class DCM {
        input CLKIN, CLKFB, RST;
        input PSCLK, PSEN, PSINCDEC;
        input STSADRS[5];
        input FREEZEDLL, FREEZEDFS;
        input CTLMODE, CTLGO, CTLOSC1, CTLOSC2, CTLSEL[3];
        input SKEWCLKIN1, SKEWCLKIN2;
        input SKEWIN, SKEWRST;
        output CLK0, CLK90, CLK180, CLK270;
        output CLK2X, CLK2X180, CLKDV;
        output CLKFX, CLKFX180, CONCUR;
        output LOCKED, PSDONE;
        output STATUS[8];
        output SKEWOUT, SCANOUT;

        attribute REG_DLL_C: bitvec[32];
        attribute REG_DLL_S: bitvec[32];
        attribute REG_DFS_C: bitvec[3];
        attribute REG_DFS_S: bitvec[87];
        attribute REG_INTERFACE: bitvec[40];
        attribute REG_OPT_INV: bitvec[3];

        attribute MODE: DCM_MODE;

        attribute OUT_CLK0_ENABLE: bool;
        attribute OUT_CLK90_ENABLE: bool;
        attribute OUT_CLK180_ENABLE: bool;
        attribute OUT_CLK270_ENABLE: bool;
        attribute OUT_CLK2X_ENABLE: bool;
        attribute OUT_CLK2X180_ENABLE: bool;
        attribute OUT_CLKDV_ENABLE: bool;
        attribute OUT_CLKFX_ENABLE: bool;
        attribute OUT_CONCUR_ENABLE: bool;

        attribute CLKIN_CLKFB_ENABLE: bool;

        attribute CLKDV_COUNT_MAX: bitvec[4];
        attribute CLKDV_COUNT_FALL: bitvec[4];
        attribute CLKDV_COUNT_FALL_2: bitvec[4];
        attribute CLKDV_PHASE_RISE: bitvec[2];
        attribute CLKDV_PHASE_FALL: bitvec[2];
        attribute CLKDV_MODE: DCM_CLKDV_MODE;

        attribute DESKEW_ADJUST: bitvec[4];
        attribute CLKIN_IOB: bool;
        attribute CLKFB_IOB: bool;
        attribute CLKIN_DIVIDE_BY_2: bool;
        attribute CLK_FEEDBACK_2X: bool;
        attribute CLK_FEEDBACK_DISABLE: bool;

        attribute DLL_ENABLE: bool;
        attribute DLL_FREQUENCY_MODE: DCM_FREQUENCY_MODE;

        attribute DFS_ENABLE: bool;
        attribute DFS_FEEDBACK: bool;
        attribute DFS_FREQUENCY_MODE: DCM_FREQUENCY_MODE;

        attribute CLKOUT_PHASE_SHIFT: DCM_CLKOUT_PHASE_SHIFT;
        attribute PHASE_SHIFT: bitvec[8];
        attribute PHASE_SHIFT_NEGATIVE: bool;

        attribute STARTUP_WAIT: bool;

        attribute CLKFX_MULTIPLY: bitvec[8];
        attribute CLKFX_DIVIDE: bitvec[8];
        attribute CLKFXDV_DIVIDE: DCM_CLKFXDV_DIVIDE;
        attribute DUTY_CYCLE_CORRECTION: bool;

        attribute PROG_ENABLE: bool;
        attribute SPREAD_SPECTRUM: DCM_SPREAD_SPECTRUM;
    }

    bel_class PLL {
        input CLKIN1, CLKIN2;
        input CLKINSEL;
        input CLKFBIN;
        output TEST_CLKIN;
        output CLKFBOUT, CLKFBDCM;
        output CLKOUT[6];
        output CLKOUTDCM[6];
        input RST;
        output LOCKED;

        input DCLK;
        input DEN;
        input DWE;
        input DADDR[5];
        input DI[16];
        output DO[16];
        output DRDY;

        input CLKBRST;
        input ENOUTSYNC;
        input MANPDLF, MANPULF;
        input SKEWCLKIN1, SKEWCLKIN2;
        input SKEWRST, SKEWSTB;
        output TEST[29];

        attribute DRP: bitvec[16][32];

        attribute ENABLE: bool;
        attribute CLKINSEL_STATIC_VAL: bitvec[1];
        attribute CLKINSEL_MODE_DYNAMIC: bool;
        attribute REL_INV: bool;

        attribute PLL_ADD_LEAKAGE: bitvec[2];
        attribute PLL_AVDD_COMP_SET: bitvec[2];
        attribute PLL_CLAMP_BYPASS: bool;
        attribute PLL_CLAMP_REF_SEL: bitvec[3];
        attribute PLL_CLKCNTRL: bitvec[1]; // set when CLKIN/CLKFB connected
        attribute PLL_CLK0MX: bitvec[2];
        attribute PLL_CLK1MX: bitvec[2];
        attribute PLL_CLK2MX: bitvec[2];
        attribute PLL_CLK3MX: bitvec[2];
        attribute PLL_CLK4MX: bitvec[2];
        attribute PLL_CLK5MX: bitvec[2];
        attribute PLL_CLKBURST_CNT: bitvec[3];
        attribute PLL_CLKBURST_ENABLE: bool;
        attribute PLL_CLKFBMX: bitvec[2];
        attribute PLL_CLKFBOUT2_DT: bitvec[6];
        attribute PLL_CLKFBOUT2_EDGE: bool;
        attribute PLL_CLKFBOUT2_HT: bitvec[6];
        attribute PLL_CLKFBOUT2_LT: bitvec[6];
        attribute PLL_CLKFBOUT2_NOCOUNT: bool;
        attribute PLL_CLKFBOUT_DT: bitvec[6];
        attribute PLL_CLKFBOUT_EDGE: bool;
        attribute PLL_CLKFBOUT_EN: bool;
        attribute PLL_CLKFBOUT_HT: bitvec[6];
        attribute PLL_CLKFBOUT_LT: bitvec[6];
        attribute PLL_CLKFBOUT_NOCOUNT: bool;
        attribute PLL_CLKFBOUT_PM: bitvec[3];
        attribute PLL_CLKOUT0_DT: bitvec[6];
        attribute PLL_CLKOUT0_EDGE: bool;
        attribute PLL_CLKOUT0_EN: bool;
        attribute PLL_CLKOUT0_HT: bitvec[6];
        attribute PLL_CLKOUT0_LT: bitvec[6];
        attribute PLL_CLKOUT0_NOCOUNT: bool;
        attribute PLL_CLKOUT0_PM: bitvec[3];
        attribute PLL_CLKOUT1_DT: bitvec[6];
        attribute PLL_CLKOUT1_EDGE: bool;
        attribute PLL_CLKOUT1_EN: bool;
        attribute PLL_CLKOUT1_HT: bitvec[6];
        attribute PLL_CLKOUT1_LT: bitvec[6];
        attribute PLL_CLKOUT1_NOCOUNT: bool;
        attribute PLL_CLKOUT1_PM: bitvec[3];
        attribute PLL_CLKOUT2_DT: bitvec[6];
        attribute PLL_CLKOUT2_EDGE: bool;
        attribute PLL_CLKOUT2_EN: bool;
        attribute PLL_CLKOUT2_HT: bitvec[6];
        attribute PLL_CLKOUT2_LT: bitvec[6];
        attribute PLL_CLKOUT2_NOCOUNT: bool;
        attribute PLL_CLKOUT2_PM: bitvec[3];
        attribute PLL_CLKOUT3_DT: bitvec[6];
        attribute PLL_CLKOUT3_EDGE: bool;
        attribute PLL_CLKOUT3_EN: bool;
        attribute PLL_CLKOUT3_HT: bitvec[6];
        attribute PLL_CLKOUT3_LT: bitvec[6];
        attribute PLL_CLKOUT3_NOCOUNT: bool;
        attribute PLL_CLKOUT3_PM: bitvec[3];
        attribute PLL_CLKOUT4_DT: bitvec[6];
        attribute PLL_CLKOUT4_EDGE: bool;
        attribute PLL_CLKOUT4_EN: bool;
        attribute PLL_CLKOUT4_HT: bitvec[6];
        attribute PLL_CLKOUT4_LT: bitvec[6];
        attribute PLL_CLKOUT4_NOCOUNT: bool;
        attribute PLL_CLKOUT4_PM: bitvec[3];
        attribute PLL_CLKOUT5_DT: bitvec[6];
        attribute PLL_CLKOUT5_EDGE: bool;
        attribute PLL_CLKOUT5_EN: bool;
        attribute PLL_CLKOUT5_HT: bitvec[6];
        attribute PLL_CLKOUT5_LT: bitvec[6];
        attribute PLL_CLKOUT5_NOCOUNT: bool;
        attribute PLL_CLKOUT5_PM: bitvec[3];
        attribute PLL_CLK_LOST_DETECT: bool;
        attribute PLL_CP: bitvec[4];
        attribute PLL_CP_BIAS_TRIP_SHIFT: bool;
        attribute PLL_CP_REPL: bitvec[4];
        attribute PLL_CP_RES: bitvec[2];
        attribute PLL_DIRECT_PATH_CNTRL: bool;
        attribute PLL_DIVCLK_EDGE: bool;
        attribute PLL_DIVCLK_EN: bool;
        attribute PLL_DIVCLK_HT: bitvec[6];
        attribute PLL_DIVCLK_LT: bitvec[6];
        attribute PLL_DIVCLK_NOCOUNT: bool;
        attribute PLL_DVDD_COMP_SET: bitvec[2];
        attribute PLL_EN: bool;
        attribute PLL_EN_CNTRL: bitvec[85];
        attribute PLL_EN_DLY: bool;
        attribute PLL_EN_LEAKAGE: bitvec[2];
        attribute PLL_EN_TCLK0: bool;
        attribute PLL_EN_TCLK1: bool;
        attribute PLL_EN_TCLK2: bool;
        attribute PLL_EN_TCLK3: bool;
        attribute PLL_EN_VCO0: bool;
        attribute PLL_EN_VCO1: bool;
        attribute PLL_EN_VCO2: bool;
        attribute PLL_EN_VCO3: bool;
        attribute PLL_EN_VCO4: bool;
        attribute PLL_EN_VCO5: bool;
        attribute PLL_EN_VCO6: bool;
        attribute PLL_EN_VCO7: bool;
        attribute PLL_EN_VCO_DIV1: bool;
        attribute PLL_EN_VCO_DIV6: bool;
        attribute PLL_INTFB: bitvec[2];
        attribute PLL_IN_DLY_MX_SEL: bitvec[5];
        attribute PLL_IN_DLY_SET: bitvec[9];
        attribute PLL_LFHF: bitvec[2];
        attribute PLL_LOCK_CNT: bitvec[10];
        attribute PLL_LOCK_FB_DLY: bitvec[5];
        attribute PLL_LOCK_REF_DLY: bitvec[5];
        attribute PLL_LOCK_SAT_HIGH: bitvec[10];
        attribute PLL_MAN_LF_EN: bool;
        attribute PLL_NBTI_EN: bool;
        attribute PLL_PFD_CNTRL: bitvec[4];
        attribute PLL_PFD_DLY: bitvec[2];
        attribute PLL_PWRD_CFG: bool;
        attribute PLL_REG_INPUT: bool;
        attribute PLL_RES: bitvec[4];
        attribute PLL_SEL_SLIPD: bool;
        attribute PLL_TEST_IN_WINDOW: bool;
        attribute PLL_UNLOCK_CNT: bitvec[10];
        attribute PLL_VDD_SEL: bitvec[2];
        attribute PLL_VLFHIGH_DIS: bool;
    }

    table PLL_MULT {
        field PLL_CP_LOW, PLL_CP_HIGH: bitvec[4];
        field PLL_CP_REPL_LOW, PLL_CP_REPL_HIGH: bitvec[4];
        field PLL_LFHF_LOW, PLL_LFHF_HIGH: bitvec[2];
        field PLL_RES_LOW, PLL_RES_HIGH: bitvec[4];
        field PLL_LOCK_CNT: bitvec[10];
        field PLL_LOCK_FB_DLY: bitvec[5];
        field PLL_LOCK_REF_DLY: bitvec[5];
        field PLL_LOCK_SAT_HIGH: bitvec[10];
        field PLL_UNLOCK_CNT: bitvec[10];

        row _1, _2, _3, _4, _5, _6, _7, _8, _9;
        row _10, _11, _12, _13, _14, _15, _16, _17, _18, _19;
        row _20, _21, _22, _23, _24, _25, _26, _27, _28, _29;
        row _30, _31, _32, _33, _34, _35, _36, _37, _38, _39;
        row _40, _41, _42, _43, _44, _45, _46, _47, _48, _49;
        row _50, _51, _52, _53, _54, _55, _56, _57, _58, _59;
        row _60, _61, _62, _63, _64;
    }

    bel_class CMT_VREG {
        attribute REG_REG: bitvec[9];
        attribute REG_BG: bitvec[11];
    }

    enum BUFGMUX_CLK_SEL_TYPE { SYNC, ASYNC }
    bel_class BUFGMUX {
        input I0, I1, S;
        output O;

        attribute INIT_OUT: bitvec[1];
        attribute CLK_SEL_TYPE: BUFGMUX_CLK_SEL_TYPE;
    }

    enum BUFIO2_DIVIDE { _1, _2, _3, _4, _5, _6, _7, _8 }
    bel_class BUFIO2 {
        input I, IB;
        output DIVCLK;
        // same as DIVCLK, but needs to be explicitly enabled?
        output DIVCLK_CMT;
        output IOCLK;
        // aka IOCE
        output SERDESSTROBE;

        attribute ENABLE: bool;
        attribute ENABLE_2CLK: bool;
        attribute CMT_ENABLE: bool;
        attribute IOCLK_ENABLE: bool;
        attribute DIVIDE: BUFIO2_DIVIDE;
        attribute DIVIDE_BYPASS: bool;
        attribute R_EDGE: bool;
        // TODO: WHAT IN FUCKS NAME ARE THESE ATTRIBUTES DEAR MEOWING GODS THIS IS THE SINGLE
        // WORST PRIMITIVE IN ALL OF PRJCOMBINE I HAVE DEALT WITH
        attribute POS_EDGE: bitvec[3];
        attribute NEG_EDGE: bitvec[2];
        // setting DIVIDE sets *_EDGE bits as follows:
        // DIVIDE   POS_EDGE    NEG_EDGE
        // 1        000         00
        // 2        001         01
        // 3        010         00
        // 4        011         00
        // 5        100         01
        // 6        101         01
        // 7        110         10
        // 8        111         10
        // setting actual POS_EDGE sets POS_EDGE bits as follows:
        // 1        000
        // 2        001
        // 3        000
        // 4        011
        // 5        000
        // 6        101
        // 7        110
        // 8        111
        // setting actual NEG_EDGE sets NEG_EDGE bits as follows:
        // 1        00
        // 2        01
        // 3        10
        // 4        00
        // 5        00
        // 6        00
        // 7        00
        // 8        00
    }

    bel_class BUFIO2FB {
        input I;
        // NOTE: CMT_ENABLE in corresponding BUFIO2 needs to be set for this to work?!?
        output O;

        attribute ENABLE: bool;
        // TODO: likely has a DIVIDE attribute just like BUFIO2
        attribute DIVIDE_BYPASS: bitvec[4];
    }

    enum BUFPLL_DATA_RATE { SDR, DDR }
    enum BUFPLL_MUX_PLLIN { CMT, GCLK }
    enum BUFPLL_LOCK_SRC { NONE, LOCK_TO_0, LOCK_TO_1 }
    bel_class BUFPLL {
        input GCLK[2];
        input PLLIN_CMT[2];
        // only on W/E edges
        input PLLIN_GCLK[2];
        input LOCKED[2];

        output PLLCLK[2];
        output PLLCE[2];
        output LOCK[2];

        attribute ENABLE: bool;
        // only on W/E edges (only CMT input exists on S/N)
        attribute MUX_PLLIN: BUFPLL_MUX_PLLIN;
        attribute LOCK_SRC: BUFPLL_LOCK_SRC;
        attribute DATA_RATE0: BUFPLL_DATA_RATE;
        attribute DATA_RATE1: BUFPLL_DATA_RATE;
        attribute DIVIDE0: BUFIO2_DIVIDE;
        attribute DIVIDE1: BUFIO2_DIVIDE;
        // ????????????
        attribute ENABLE_BOTH_SYNC0: bitvec[3];
        attribute ENABLE_BOTH_SYNC1: bitvec[3];
        attribute ENABLE_NONE_SYNC0: bitvec[2];
        attribute ENABLE_NONE_SYNC1: bitvec[2];
        attribute ENABLE_SYNC0: bool;
        attribute ENABLE_SYNC1: bool;
    }

    enum PCILOGICSE_PCI_CE_DELAY {
        TAP2,
        TAP3,
        TAP4,
        TAP5,
        TAP6,
        TAP7,
        TAP8,
        TAP9,
        TAP10,
        TAP11,
        TAP12,
        TAP13,
        TAP14,
        TAP15,
        TAP16,
        TAP17,
        TAP18,
        TAP19,
        TAP20,
        TAP21,
        TAP22,
        TAP23,
        TAP24,
        TAP25,
        TAP26,
        TAP27,
        TAP28,
        TAP29,
        TAP30,
        TAP31,
    }
    bel_class PCILOGICSE {
        input I1, I2, I3;

        attribute ENABLE: bool;
        attribute PCI_CE_DELAY: PCILOGICSE_PCI_CE_DELAY;
    }
    device_data PCILOGICSE_PCI_CE_DELAY: PCILOGICSE_PCI_CE_DELAY;

    enum MCB_MEM_PLL_POL_SEL { INVERTED, NOTINVERTED }
    enum MCB_MEM_TYPE { DDR3, DDR2, DDR, MDDR }
    enum MCB_MEM_WIDTH { NONE, _4, _8, _16 }
    enum MCB_MEM_ADDR_ORDER { BANK_ROW_COLUMN, ROW_BANK_COLUMN }
    enum MCB_MEM_BURST_LEN { NONE, _4, _8 }
    enum MCB_MEM_CA_SIZE { _9, _10, _11, _12 }
    enum MCB_MEM_BA_SIZE { _2, _3 }
    enum MCB_MEM_RA_SIZE { _12, _13, _14, _15 }
    enum MCB_PORT_CONFIG { B32_B32_X32_X32_X32_X32, B32_B32_B32_B32, B64_B32_B32, B64_B64, B128 }
    enum MCB_ARB_NUM_TIME_SLOTS { _10, _12 }
    enum MCB_CAL_CALIBRATION_MODE { CALIBRATION, NOCALIBRATION }
    enum MCB_CAL_CLK_DIV { _1, _2, _4, _8 }
    enum MCB_CAL_DELAY { QUARTER, HALF, THREEQUARTER, FULL }
    enum MCB_MEM_CAS_LATENCY { NONE, _1, _2, _3, _4, _5, _6 }
    enum MCB_MEM_DDR3_CAS_LATENCY { NONE, _5, _6, _7, _8, _9, _10 }
    enum MCB_MEM_DDR2_WRT_RECOVERY { NONE, _2, _3, _4, _5, _6 }
    enum MCB_MEM_DDR3_WRT_RECOVERY { NONE, _5, _6, _7, _8, _10, _12 }
    enum MCB_MEM_DDR1_2_ODS { FULL, REDUCED }
    enum MCB_MEM_DDR2_ADD_LATENCY { _0, _1, _2, _3, _4, _5 }
    enum MCB_MEM_DDR2_DIFF_DQS_EN { NO, YES }
    enum MCB_MEM_DDR2_RTT { NONE, _75OHMS, _150OHMS, _50OHMS }
    enum MCB_MEM_DDR3_ADD_LATENCY { NONE, CL1, CL2 }
    enum MCB_MEM_DDR3_ODS { DIV6, DIV7 }
    enum MCB_MEM_DDR3_RTT { NONE, DIV2, DIV4, DIV6, DIV8, DIV12 }
    enum MCB_MEM_MDDR_ODS { FULL, HALF, QUARTER, THREEQUARTERS }
    enum MCB_MEM_MOBILE_PA_SR { FULL, HALF }
    enum MCB_MEM_MOBILE_TC_SR { _0, _1, _2, _3 }
    enum MCB_MEM_DDR2_3_PA_SR { FULL, HALF1, QUARTER1, EIGHTH1, THREEQUARTER, HALF2, QUARTER2, EIGHTH2 }
    enum MCB_MEM_DDR3_CAS_WR_LATENCY { _5, _6, _7, _8 }
    enum MCB_MEM_DDR3_AUTO_SR { MANUAL, ENABLED }
    enum MCB_MEM_DDR2_3_HIGH_TEMP_SR { NORMAL, EXTENDED }
    enum MCB_MEM_DDR3_DYN_WRT_ODT { NONE, DIV2, DIV4 }
    enum MCB_MUI_PORT_CONFIG { READ, WRITE }

    bel_class MCB {
        // directly wired to BUFPLL
        input PLLCLK[2];
        input PLLCE[2];

        // the command queues
        for i in 0..6 {
            input "P{i}ARBEN";
            input "P{i}CMDCLK", "P{i}CMDEN";
            input "P{i}CMDBA"[3];
            input "P{i}CMDBL"[6];
            input "P{i}CMDCA"[12];
            input "P{i}CMDRA"[15];
            input "P{i}CMDINSTR"[3];
            output "P{i}CMDEMPTY", "P{i}CMDFULL";
        }

        // the MUIs
        for i in 0..2 {
            input "P{i}RDCLK", "P{i}RDEN";
            output "P{i}RDCOUNT"[7];
            output "P{i}RDDATA"[32];
            output "P{i}RDEMPTY", "P{i}RDFULL", "P{i}RDOVERFLOW", "P{i}RDERROR";
            input "P{i}RTSTENB", "P{i}RTSTPINENB";
            input "P{i}RTSTMODEB"[4];
            output "P{i}RTSTUNDERRUN";
            input "P{i}RTSTWRDATA"[32];
            input "P{i}RTSTWRMASK"[4];
        }

        for i in 0..2 {
            input "P{i}WRCLK", "P{i}WREN";
            output "P{i}WRCOUNT"[7];
            input "P{i}WRDATA"[32];
            input "P{i}RWRMASK"[4];
            output "P{i}WREMPTY", "P{i}WRFULL", "P{i}WRUNDERRUN", "P{i}WRERROR";
            input "P{i}WTSTENB", "P{i}WTSTPINENB";
            input "P{i}WTSTMODEB"[4];
            output "P{i}WTSTDATA"[32];
            output "P{i}WTSTOVERFLOW";
        }

        for i in 2..6 {
            input "P{i}CLK", "P{i}EN";
            output "P{i}COUNT"[7];
            output "P{i}RDDATA"[32];
            input "P{i}WRDATA"[32];
            input "P{i}WRMASK"[4];
            output "P{i}EMPTY", "P{i}FULL", "P{i}ERROR", "P{i}RDOVERFLOW", "P{i}WRUNDERRUN";
            input "P{i}TSTENB", "P{i}TSTPINENB";
            input "P{i}TSTMODEB"[4];
        }

        output P0RTSTUDMP, P0WTSTLDMP;
        output P1RTSTUDMN, P1WTSTLDMN;
        output P2TSTUDMP, P3TSTLDMP, P4TSTUDMN, P5TSTLDMN;

        // misc
        input SYSRST;
        input PLLLOCK;
        input RECAL;
        input SELFREFRESHENTER;
        output SELFREFRESHMODE;
        output STATUS[32];
        output TSTCMDOUT[39];
        input TSTCMDTESTENB;
        input TSTINB[16];
        input TSTSCANCLK;
        input TSTSCANENB;
        input TSTSCANIN;
        input TSTSCANMODE;
        output TSTSCANOUT;
        input TSTSCANRST;
        input TSTSCANSET;
        input TSTSEL[8];

        // DRP etc
        input UIADD;
        input UIADDR[5];
        input UIBROADCAST;
        input UICLK;
        input UICMD;
        input UICMDEN;
        input UICMDIN;
        input UICS;
        input UIDONECAL;
        input UIDQCOUNT[4];
        input UIDQLOWERDEC;
        input UIDQLOWERINC;
        input UIDQUPPERDEC;
        input UIDQUPPERINC;
        input UIDRPUPDATE;
        input UILDQSDEC;
        input UILDQSINC;
        input UIREAD;
        input UISDI;
        input UIUDQSDEC;
        input UIUDQSINC;
        output UOCALSTART;
        output UOCMDREADYIN;
        output UODATA[8];
        output UODATAVALID;
        output UODONECAL;
        output UOREFRSHFLAG;
        output UOSDO;

        // main configuration
        attribute MEM_PLL_DIV_EN: bool;
        attribute MEM_PLL_POL_SEL: MCB_MEM_PLL_POL_SEL;
        attribute MEM_TYPE: MCB_MEM_TYPE;
        attribute MEM_WIDTH: MCB_MEM_WIDTH;
        attribute MEM_ADDR_ORDER: MCB_MEM_ADDR_ORDER;
        attribute MEM_BURST_LEN: MCB_MEM_BURST_LEN;
        attribute MEM_CA_SIZE: MCB_MEM_CA_SIZE;
        attribute MEM_BA_SIZE: MCB_MEM_BA_SIZE;
        attribute MEM_RA_SIZE: MCB_MEM_RA_SIZE;

        attribute PORT_CONFIG: MCB_PORT_CONFIG;
        attribute ARB_NUM_TIME_SLOTS: MCB_ARB_NUM_TIME_SLOTS;
        attribute ARB_TIME_SLOT: bitvec[18][12];

        attribute CAL_CA: bitvec[12];
        attribute CAL_BA: bitvec[3];
        attribute CAL_RA: bitvec[15];
        attribute CAL_BYPASS: bool;
        attribute CAL_CALIBRATION_MODE: MCB_CAL_CALIBRATION_MODE;
        attribute CAL_CLK_DIV: MCB_CAL_CLK_DIV;
        attribute CAL_DELAY: MCB_CAL_DELAY;

        attribute MEM_RAS_VAL: bitvec[5];
        attribute MEM_RCD_VAL: bitvec[3];
        attribute MEM_REFI_VAL: bitvec[12];
        attribute MEM_RFC_VAL: bitvec[8];
        attribute MEM_RP_VAL: bitvec[4];
        attribute MEM_RTP_VAL: bitvec[3];
        attribute MEM_WR_VAL: bitvec[3];
        attribute MEM_WTR_VAL: bitvec[3];

        // MRs and their fields
        attribute MR: bitvec[14];
        attribute EMR1: bitvec[14];
        attribute EMR2: bitvec[14];
        attribute EMR3: bitvec[14];
        // MR
        attribute MEM_DDR_DDR2_MDDR_BURST_LEN: MCB_MEM_BURST_LEN;
        attribute MEM_CAS_LATENCY: MCB_MEM_CAS_LATENCY;
        attribute MEM_DDR3_CAS_LATENCY: MCB_MEM_DDR3_CAS_LATENCY;
        attribute MEM_DDR2_WRT_RECOVERY: MCB_MEM_DDR2_WRT_RECOVERY;
        attribute MEM_DDR3_WRT_RECOVERY: MCB_MEM_DDR3_WRT_RECOVERY;
        // EMR1
        attribute MEM_DDR1_2_ODS: MCB_MEM_DDR1_2_ODS;
        attribute MEM_DDR2_ADD_LATENCY: MCB_MEM_DDR2_ADD_LATENCY;
        attribute MEM_DDR2_DIFF_DQS_EN: MCB_MEM_DDR2_DIFF_DQS_EN;
        attribute MEM_DDR2_RTT: MCB_MEM_DDR2_RTT;
        attribute MEM_DDR3_ADD_LATENCY: MCB_MEM_DDR3_ADD_LATENCY;
        attribute MEM_DDR3_ODS: MCB_MEM_DDR3_ODS;
        attribute MEM_DDR3_RTT: MCB_MEM_DDR3_RTT;
        attribute MEM_MDDR_ODS: MCB_MEM_MDDR_ODS;
        attribute MEM_MOBILE_PA_SR: MCB_MEM_MOBILE_PA_SR;
        attribute MEM_MOBILE_TC_SR: MCB_MEM_MOBILE_TC_SR;
        // EMR2
        attribute MEM_DDR2_3_PA_SR: MCB_MEM_DDR2_3_PA_SR;
        attribute MEM_DDR3_CAS_WR_LATENCY: MCB_MEM_DDR3_CAS_WR_LATENCY;
        attribute MEM_DDR3_AUTO_SR: MCB_MEM_DDR3_AUTO_SR;
        attribute MEM_DDR2_3_HIGH_TEMP_SR: MCB_MEM_DDR2_3_HIGH_TEMP_SR;
        attribute MEM_DDR3_DYN_WRT_ODT: MCB_MEM_DDR3_DYN_WRT_ODT;

        // MUI configuration
        for i in 0..2 {
            attribute "MUI{i}R_MEM_PLL_DIV_EN": bool;
            attribute "MUI{i}R_MEM_PLL_POL_SEL": MCB_MEM_PLL_POL_SEL;
            attribute "MUI{i}R_MEM_WIDTH": MCB_MEM_WIDTH;
            attribute "MUI{i}R_PORT_CONFIG": MCB_MUI_PORT_CONFIG;

            attribute "MUI{i}W_MEM_PLL_DIV_EN": bool;
            attribute "MUI{i}W_MEM_PLL_POL_SEL": MCB_MEM_PLL_POL_SEL;
            attribute "MUI{i}W_MEM_WIDTH": MCB_MEM_WIDTH;
            attribute "MUI{i}W_PORT_CONFIG": MCB_MUI_PORT_CONFIG;
        }
        for i in 2..6 {
            attribute "MUI{i}_MEM_PLL_DIV_EN": bool;
            attribute "MUI{i}_MEM_PLL_POL_SEL": MCB_MEM_PLL_POL_SEL;
            attribute "MUI{i}_MEM_WIDTH": MCB_MEM_WIDTH;
            attribute "MUI{i}_PORT_CONFIG": MCB_MUI_PORT_CONFIG;
        }
    }

    bel_class PCIE {
        input MGTCLK;
        input USERCLK;
        input SYSRESETN;
        output USERRSTN;
        output RECEIVEDHOTRESET;
        input CLOCKLOCKED;

        output TRNLNKUPN;
        input TRNFCSEL[3];
        output TRNFCPH[8];
        output TRNFCPD[12];
        output TRNFCNPH[8];
        output TRNFCNPD[12];
        output TRNFCCPLH[8];
        output TRNFCCPLD[12];

        input TRNTSOFN;
        input TRNTEOFN;
        input TRNTD[32];
        input TRNTSRCRDYN;
        output TRNTDSTRDYN;
        input TRNTSRCDSCN;
        output TRNTBUFAV[6];
        output TRNTERRDROPN;
        input TRNTSTRN;
        output TRNTCFGREQN;
        input TRNTCFGGNTN;
        input TRNTERRFWDN;

        output TRNRSOFN;
        output TRNREOFN;
        output TRNRD[32];
        output TRNRERRFWDN;
        output TRNRSRCDSCN;
        input TRNRDSTRDYN;
        output TRNRSRCRDYN;
        input TRNRNPOKN;
        output TRNRBARHITN[7];

        output CFGDO[32];
        output CFGRDWRDONEN;
        input CFGDWADDR[10];
        input CFGRDENN;

        input CFGINTERRUPTN;
        output CFGINTERRUPTRDYN;
        input CFGINTERRUPTASSERTN;
        input CFGINTERRUPTDI[8];
        output CFGINTERRUPTDO[8];
        output CFGINTERRUPTMMENABLE[3];
        output CFGINTERRUPTMSIENABLE;

        output CFGBUSNUMBER[8];
        output CFGDEVICENUMBER[5];
        output CFGFUNCTIONNUMBER[3];

        input CFGVENID[16];
        input CFGDEVID[16];
        input CFGREVID[8];
        input CFGSUBSYSID[16];
        input CFGSUBSYSVENID[16];

        output CFGCOMMANDIOENABLE;
        output CFGCOMMANDMEMENABLE;
        output CFGCOMMANDBUSMASTERENABLE;
        output CFGCOMMANDSERREN;
        output CFGCOMMANDINTERRUPTDISABLE;

        output CFGDEVSTATUSCORRERRDETECTED;
        output CFGDEVSTATUSNONFATALERRDETECTED;
        output CFGDEVSTATUSFATALERRDETECTED;
        output CFGDEVSTATUSURDETECTED;

        output CFGDEVCONTROLCORRERRREPORTINGEN;
        output CFGDEVCONTROLNONFATALREPORTINGEN;
        output CFGDEVCONTROLFATALERRREPORTINGEN;
        output CFGDEVCONTROLURERRREPORTINGEN;
        output CFGDEVCONTROLENABLERO;
        output CFGDEVCONTROLMAXPAYLOAD[3];
        output CFGDEVCONTROLEXTTAGEN;
        output CFGDEVCONTROLPHANTOMEN;
        output CFGDEVCONTROLAUXPOWEREN;
        output CFGDEVCONTROLNOSNOOPEN;
        output CFGDEVCONTROLMAXREADREQ[3];

        output CFGLINKCONTROLASPMCONTROL[2];
        output CFGLINKCONTOLRCB;
        output CFGLINKCONTROLCOMMONCLOCK;
        output CFGLINKCONTROLEXTENDEDSYNC;

        output CFGTOTURNOFFN;
        input CFGTURNOFFOKN;
        input CFGPMWAKEN;
        output CFGPCIELINKSTATEN[3];
        input CFGTRNPENDINGN;
        input CFGDSN[64];
        output CFGLTSSMSTATE[5];

        input CFGERRECRCN;
        input CFGERRURN;
        input CFGERRCPLTIMEOUTN;
        input CFGERRCPLABORTN;
        input CFGERRPOSTEDN;
        input CFGERRCORN;
        input CFGERRTLPCPLHEADER[48];
        output CFGERRCPLRDYN;
        input CFGERRLOCKEDN;

        output DBGBADDLLPSTATUS;
        output DBGBADTLPLCRC;
        output DBGBADTLPSEQNUM;
        output DBGBADTLPSTATUS;
        output DBGDLPROTOCOLSTATUS;
        output DBGFCPROTOCOLERRSTATUS;
        output DBGMLFRMDLENGTH;
        output DBGMLFRMDMPS;
        output DBGMLFRMDTCVC;
        output DBGMLFRMDTLPSTATUS;
        output DBGMLFRMDUNRECTYPE;
        output DBGPOISTLPSTATUS;
        output DBGRCVROVERFLOWSTATUS;
        output DBGREGDETECTEDCORRECTABLE;
        output DBGREGDETECTEDFATAL;
        output DBGREGDETECTEDNONFATAL;
        output DBGREGDETECTEDUNSUPPORTED;
        output DBGRPLYROLLOVERSTATUS;
        output DBGRPLYTIMEOUTSTATUS;
        output DBGURNOBARHIT;
        output DBGURPOISCFGWR;
        output DBGURSTATUS;
        output DBGURUNSUPMSG;

        output MIMRXREN;
        output MIMRXRADDR[12];
        input MIMRXRDATA[35];
        output MIMRXWEN;
        output MIMRXWADDR[12];
        output MIMRXWDATA[35];

        output MIMTXREN;
        output MIMTXRADDR[12];
        input MIMTXRDATA[36];
        output MIMTXWEN;
        output MIMTXWADDR[12];
        output MIMTXWDATA[36];

        output PIPEGTPOWERDOWNA[2];
        output PIPEGTPOWERDOWNB[2];
        input PIPEGTRESETDONEA;
        input PIPEGTRESETDONEB;
        output PIPEGTTXELECIDLEA;
        output PIPEGTTXELECIDLEB;
        input PIPEPHYSTATUSA;
        input PIPEPHYSTATUSB;
        input PIPERXCHARISKA[2];
        input PIPERXCHARISKB[2];
        input PIPERXDATAA[16];
        input PIPERXDATAB[16];
        input PIPERXENTERELECIDLEA;
        input PIPERXENTERELECIDLEB;
        output PIPERXPOLARITYA;
        output PIPERXPOLARITYB;
        output PIPERXRESETA;
        output PIPERXRESETB;
        input PIPERXSTATUSA[3];
        input PIPERXSTATUSB[3];
        output PIPETXCHARDISPMODEA[2];
        output PIPETXCHARDISPMODEB[2];
        output PIPETXCHARDISPVALA[2];
        output PIPETXCHARDISPVALB[2];
        output PIPETXCHARISKA[2];
        output PIPETXCHARISKB[2];
        output PIPETXDATAA[16];
        output PIPETXDATAB[16];
        output PIPETXRCVRDETA;
        output PIPETXRCVRDETB;

        input SCANEN;
        input SCANIN[5];
        output SCANOUT[5];
        input SCANRESETMASK;

        attribute BAR0: bitvec[32];
        attribute BAR1: bitvec[32];
        attribute BAR2: bitvec[32];
        attribute BAR3: bitvec[32];
        attribute BAR4: bitvec[32];
        attribute BAR5: bitvec[32];
        attribute CARDBUS_CIS_POINTER: bitvec[32];
        attribute CLASS_CODE: bitvec[24];
        attribute DEV_CAP_ENDPOINT_L0S_LATENCY: bitvec[3];
        attribute DEV_CAP_ENDPOINT_L1_LATENCY: bitvec[3];
        attribute DEV_CAP_EXT_TAG_SUPPORTED: bool;
        attribute DEV_CAP_MAX_PAYLOAD_SUPPORTED: bitvec[3];
        attribute DEV_CAP_PHANTOM_FUNCTIONS_SUPPORT: bitvec[2];
        attribute DEV_CAP_ROLE_BASED_ERROR: bool;
        attribute DISABLE_BAR_FILTERING: bool;
        attribute DISABLE_ID_CHECK: bool;
        attribute DISABLE_SCRAMBLING: bool;
        attribute ENABLE_RX_TD_ECRC_TRIM: bool;
        attribute EXPANSION_ROM: bitvec[22];
        attribute FAST_TRAIN: bool;
        attribute GTP_SEL: bitvec[1];
        attribute LINK_CAP_ASPM_SUPPORT: bitvec[2];
        attribute LINK_CAP_L0S_EXIT_LATENCY: bitvec[3];
        attribute LINK_CAP_L1_EXIT_LATENCY: bitvec[3];
        attribute LINK_STATUS_SLOT_CLOCK_CONFIG: bool;
        attribute LL_ACK_TIMEOUT: bitvec[15];
        attribute LL_ACK_TIMEOUT_EN: bool;
        attribute LL_REPLAY_TIMEOUT: bitvec[15];
        attribute LL_REPLAY_TIMEOUT_EN: bool;
        attribute MSI_CAP_MULTIMSGCAP: bitvec[3];
        attribute MSI_CAP_MULTIMSG_EXTENSION: bitvec[1];
        attribute PCIE_CAP_CAPABILITY_VERSION: bitvec[4];
        attribute PCIE_CAP_DEVICE_PORT_TYPE: bitvec[4];
        attribute PCIE_CAP_INT_MSG_NUM: bitvec[5];
        attribute PCIE_CAP_SLOT_IMPLEMENTED: bool;
        attribute PCIE_GENERIC: bitvec[12];
        attribute PLM_AUTO_CONFIG: bool;
        attribute PM_CAP_AUXCURRENT: bitvec[3];
        attribute PM_CAP_D1SUPPORT: bool;
        attribute PM_CAP_D2SUPPORT: bool;
        attribute PM_CAP_DSI: bool;
        attribute PM_CAP_PMESUPPORT: bitvec[5];
        attribute PM_CAP_PME_CLOCK: bool;
        attribute PM_CAP_VERSION: bitvec[3];
        attribute PM_DATA0: bitvec[8];
        attribute PM_DATA1: bitvec[8];
        attribute PM_DATA2: bitvec[8];
        attribute PM_DATA3: bitvec[8];
        attribute PM_DATA4: bitvec[8];
        attribute PM_DATA5: bitvec[8];
        attribute PM_DATA6: bitvec[8];
        attribute PM_DATA7: bitvec[8];
        attribute PM_DATA_SCALE0: bitvec[2];
        attribute PM_DATA_SCALE1: bitvec[2];
        attribute PM_DATA_SCALE2: bitvec[2];
        attribute PM_DATA_SCALE3: bitvec[2];
        attribute PM_DATA_SCALE4: bitvec[2];
        attribute PM_DATA_SCALE5: bitvec[2];
        attribute PM_DATA_SCALE6: bitvec[2];
        attribute PM_DATA_SCALE7: bitvec[2];
        attribute SLOT_CAP_ATT_BUTTON_PRESENT: bool;
        attribute SLOT_CAP_ATT_INDICATOR_PRESENT: bool;
        attribute SLOT_CAP_POWER_INDICATOR_PRESENT: bool;
        attribute TL_RX_RAM_RADDR_LATENCY: bitvec[1];
        attribute TL_RX_RAM_RDATA_LATENCY: bitvec[2];
        attribute TL_RX_RAM_WRITE_LATENCY: bitvec[1];
        attribute TL_TFC_DISABLE: bool;
        attribute TL_TX_CHECKS_DISABLE: bool;
        attribute TL_TX_RAM_RADDR_LATENCY: bitvec[1];
        attribute TL_TX_RAM_RDATA_LATENCY: bitvec[2];
        attribute USR_CFG: bool;
        attribute USR_EXT_CFG: bool;
        attribute VC0_CPL_INFINITE: bool;
        attribute VC0_RX_RAM_LIMIT: bitvec[12];
        attribute VC0_TOTAL_CREDITS_CD: bitvec[11];
        attribute VC0_TOTAL_CREDITS_CH: bitvec[7];
        attribute VC0_TOTAL_CREDITS_NPH: bitvec[7];
        attribute VC0_TOTAL_CREDITS_PD: bitvec[11];
        attribute VC0_TOTAL_CREDITS_PH: bitvec[7];
        attribute VC0_TX_LASTPACKET: bitvec[5];
    }

    enum GTP_MUX_CLKOUT { REFCLKPLL0, REFCLKPLL1 }
    enum GTP_MUX_REFSELPLL { CLK0, GCLK0, PLLCLK0, CLKINEAST, CLK1, GCLK1, PLLCLK1, CLKINWEST }
    enum GTP_ALIGN_COMMA_WORD { _1, _2 }
    enum GTP_CHAN_BOND_SEQ_LEN { _1, _2, _3, _4 }
    enum GTP_CLK_COR_ADJ_LEN { _1, _2, _3, _4 }
    enum GTP_CLK_COR_DET_LEN { _1, _2, _3, _4 }
    enum GTP_CLK25_DIVIDER { _1, _2, _3, _4, _5, _6, _10, _12 }
    enum GTP_OOB_CLK_DIVIDER { _1, _2, _4, _6, _8, _10, _12, _14 }
    enum GTP_PLL_DIVSEL_FB { _1, _2, _3, _4, _5, _8, _10 }
    enum GTP_PLL_DIVSEL_REF { _1, _2, _3, _4, _5, _6, _8, _10, _12, _16, _20 }
    enum GTP_PLL_DIVSEL_OUT { _1, _2, _4 }
    enum GTP_PLL_SOURCE { PLL0, PLL1 }
    enum GTP_RX_LOS_INVALID_INCR { _1, _2, _4, _8, _16, _32, _64, _128 }
    enum GTP_RX_LOS_THRESHOLD { _4, _8, _16, _32, _64, _128, _256, _512 }
    enum GTP_RX_SLIDE_MODE { PCS, PMA }
    enum GTP_RX_STATUS_FMT { PCIE, SATA }
    enum GTP_RX_XCLK_SEL { RXREC, RXUSR }
    enum GTP_TX_XCLK_SEL { TXUSR, TXOUT }
    enum GTP_CLK_OUT_GTP_SEL_0 { TXOUTCLK0, REFCLKPLL0 }
    enum GTP_CLK_OUT_GTP_SEL_1 { TXOUTCLK1, REFCLKPLL1 }

    bel_class GTP {
        input DADDR[8];
        input DCLK;
        input DEN;
        input DWE;
        input DI[16];
        output DRDY;
        output DRPDO[16];

        input SCANCLK;
        input SCANENB;
        input SCANMODEB;
        input SCANIN[5];
        input SCANINPMA;
        output SCANOUT[5];
        output SCANOUTPMA;

        input GTPCLKFBSEL0EAST[2];
        input GTPCLKFBSEL0WEST[2];
        input GTPCLKFBSEL1EAST[2];
        input GTPCLKFBSEL1WEST[2];
        output GTPCLKFBEAST[2];
        output GTPCLKFBWEST[2];

        pad AVCC: power;
        pad AVTTRX: power;
        pad AVTTTX: power;
        pad AVTTRCAL: power;
        pad RREF: analog;

        for i in 0..2 {
            pad "CLKP{i}": input;
            pad "CLKN{i}": input;
        }

        for ch in 0..2 {
            pad "TXP{ch}": output;
            pad "TXN{ch}": output;
            pad "RXP{ch}": input;
            pad "RXN{ch}": input;

            pad "AVCCPLL{ch}": power;

            input "GCLK0{ch}";
            input "GCLK1{ch}";
            input "PLLCLK0{ch}";
            input "PLLCLK1{ch}";
            input "CLKTESTSIG0{ch}";
            input "CLKTESTSIG1{ch}";
            output "REFCLKOUT{ch}";
            input "REFCLKPWRDNB{ch}";
            input "REFSELDYPLL{ch}"[3];
            output "GTPCLKOUT{ch}"[2];

            input "GTPRESET{ch}";
            input "GTPTEST{ch}"[8];
            input "INTDATAWIDTH{ch}";
            output "RESETDONE{ch}";

            output "PLLLKDET{ch}";
            input "PLLLKDETEN{ch}";
            input "PLLPOWERDOWN{ch}";

            input "RXUSRCLK{ch}";
            input "RXUSRCLK2{ch}";
            output "RXRECCLK{ch}";
            input "RXRESET{ch}";
            input "RXPOWERDOWN{ch}"[2];

            output "RXCHARISCOMMA{ch}"[4];
            output "RXCHARISK{ch}"[4];
            input "RXDATAWIDTH{ch}"[2];
            output "RXDATA{ch}"[32];
            input "RXDEC8B10BUSE{ch}";
            output "RXDISPERR{ch}"[4];
            output "RXNOTINTABLE{ch}"[4];
            output "RXRUNDISP{ch}"[4];

            output "RXBYTEISALIGNED{ch}";
            output "RXBYTEREALIGN{ch}";
            output "RXCOMMADET{ch}";
            input "RXCOMMADETUSE{ch}";
            input "RXENMCOMMAALIGN{ch}";
            input "RXENPCOMMAALIGN{ch}";
            input "RXSLIDE{ch}";
            output "RXLOSSOFSYNC{ch}"[2];

            input "RXBUFRESET{ch}";
            output "RXBUFSTATUS{ch}"[3];
            output "RXCLKCORCNT{ch}"[3];

            output "RXCHANBONDSEQ{ch}";
            output "RXCHANISALIGNED{ch}";
            output "RXCHANREALIGN{ch}";
            input "RXCHBONDMASTER{ch}";
            input "RXCHBONDSLAVE{ch}";
            input "RXENCHANSYNC{ch}";

            input "PRBSCNTRESET{ch}";
            input "RXENPRBSTST{ch}"[3];
            output "RXPRBSERR{ch}";

            input "RXENPMAPHASEALIGN{ch}";
            input "RXPMASETPHASE{ch}";

            input "RXCDRRESET{ch}";
            output "RXELECIDLE{ch}";
            output "RXSTATUS{ch}"[3];
            output "RXVALID{ch}";
            input "RXPOLARITY{ch}";
            input "RXEQMIX{ch}"[2];

            input "TXUSRCLK{ch}";
            input "TXUSRCLK2{ch}";
            output "TXOUTCLK{ch}";
            input "TXRESET{ch}";
            input "TXPOWERDOWN{ch}"[2];
            input "TXPDOWNASYNCH{ch}";

            input "TXCHARDISPMODE{ch}"[4];
            input "TXCHARDISPVAL{ch}"[4];
            input "TXCHARISK{ch}"[4];
            input "TXBYPASS8B10B{ch}"[4];
            input "TXDATA{ch}"[32];
            input "TXDATAWIDTH{ch}"[2];
            input "TXENC8B10BUSE{ch}";
            output "TXKERR{ch}"[4];
            output "TXRUNDISP{ch}"[4];

            output "TXBUFSTATUS{ch}"[2];
            input "TXENPMAPHASEALIGN{ch}";
            input "TXPMASETPHASE{ch}";

            input "TXENPRBSTST{ch}"[3];
            input "TXPRBSFORCEERR{ch}";

            input "TXPOLARITY{ch}";

            input "TXBUFDIFFCTRL{ch}"[3];
            input "TXDIFFCTRL{ch}"[4];
            input "TXELECIDLE{ch}";
            input "TXINHIBIT{ch}";
            input "TXPREEMPHASIS{ch}"[3];
            input "TXCOMSTART{ch}";
            input "TXCOMTYPE{ch}";
            input "TXDETECTRX{ch}";

            output "PHYSTATUS{ch}";
            input "LOOPBACK{ch}"[3];
            input "GATERXELECIDLE{ch}";
            input "IGNORESIGDET{ch}";
            input "USRCODEERR{ch}";

            input "TSTCLK{ch}";
            input "TSTIN{ch}"[12];
            output "TSTOUT{ch}"[5];
            input "TSTPWRDN{ch}"[5];
            input "TSTPWRDNOVRD{ch}";
        }

        attribute DRP: bitvec[16][0x80];

        attribute PMA_COM_CFG_EAST: bitvec[36];
        attribute PMA_COM_CFG_WEST: bitvec[36];

        attribute MUX_CLKOUT_EAST: GTP_MUX_CLKOUT;
        attribute MUX_CLKOUT_WEST: GTP_MUX_CLKOUT;
        attribute REFSELPLL0_STATIC_VAL: GTP_MUX_REFSELPLL;
        attribute REFSELPLL1_STATIC_VAL: GTP_MUX_REFSELPLL;
        attribute REFSELPLL0_STATIC_ENABLE: bool;
        attribute REFSELPLL1_STATIC_ENABLE: bool;

        for ch in 0..2 {
            attribute "AC_CAP_DIS_{ch}": bool;
            attribute "CHAN_BOND_KEEP_ALIGN_{ch}": bool;
            attribute "CHAN_BOND_SEQ_2_USE_{ch}": bool;
            attribute "CLK_COR_INSERT_IDLE_FLAG_{ch}": bool;
            attribute "CLK_COR_KEEP_IDLE_{ch}": bool;
            attribute "CLK_COR_PRECEDENCE_{ch}": bool;
            attribute "CLK_CORRECT_USE_{ch}": bool;
            attribute "CLK_COR_SEQ_2_USE_{ch}": bool;
            attribute "CLKINDC_B_{ch}": bool;
            attribute "CLKRCV_TRST_{ch}": bool;
            attribute "DEC_MCOMMA_DETECT_{ch}": bool;
            attribute "DEC_PCOMMA_DETECT_{ch}": bool;
            attribute "DEC_VALID_COMMA_ONLY_{ch}": bool;
            attribute "GTP_CFG_PWRUP_{ch}": bool;
            attribute "LOOPBACK_DRP_EN_{ch}": bool;
            attribute "MASTER_DRP_EN_{ch}": bool;
            attribute "MCOMMA_DETECT_{ch}": bool;
            attribute "PCI_EXPRESS_MODE_{ch}": bool;
            attribute "PCOMMA_DETECT_{ch}": bool;
            attribute "PDELIDLE_DRP_EN_{ch}": bool;
            attribute "PHASEALIGN_DRP_EN_{ch}": bool;
            attribute "PLL_DRP_EN_{ch}": bool;
            attribute "PLL_SATA_{ch}": bool;
            attribute "PLL_STARTUP_EN_{ch}": bool;
            attribute "POLARITY_DRP_EN_{ch}": bool;
            attribute "PRBS_DRP_EN_{ch}": bool;
            attribute "RCV_TERM_GND_{ch}": bool;
            attribute "RCV_TERM_VTTRX_{ch}": bool;
            attribute "RESET_DRP_EN_{ch}": bool;
            attribute "RX_BUFFER_USE_{ch}": bool;
            attribute "RX_CDR_FORCE_ROTATE_{ch}": bool;
            attribute "RX_DECODE_SEQ_MATCH_{ch}": bool;
            attribute "RX_EN_IDLE_HOLD_CDR_{ch}": bool;
            attribute "RX_EN_IDLE_RESET_BUF_{ch}": bool;
            attribute "RX_EN_IDLE_RESET_FR_{ch}": bool;
            attribute "RX_EN_IDLE_RESET_PH_{ch}": bool;
            attribute "RX_EN_MODE_RESET_BUF_{ch}": bool;
            attribute "RXEQ_DRP_EN_{ch}": bool;
            attribute "RX_LOSS_OF_SYNC_FSM_{ch}": bool;
            attribute "TERMINATION_OVRD_{ch}": bool;
            attribute "TX_BUFFER_USE_{ch}": bool;
            attribute "TXDRIVE_DRP_EN_{ch}": bool;

            attribute "A_GTPRESET_{ch}": bitvec[1];
            attribute "A_LOOPBACK_{ch}": bitvec[3];
            attribute "A_PLLLKDETEN_{ch}": bitvec[1];
            attribute "A_PLLPOWERDOWN_{ch}": bitvec[1];
            attribute "A_PRBSCNTRESET_{ch}": bitvec[1];
            attribute "A_RXBUFRESET_{ch}": bitvec[1];
            attribute "A_RXCDRFREQRESET_{ch}": bitvec[1];
            attribute "A_RXCDRHOLD_{ch}": bitvec[1];
            attribute "A_RXCDRPHASERESET_{ch}": bitvec[1];
            attribute "A_RXCDRRESET_{ch}": bitvec[1];
            attribute "A_RXENPMAPHASEALIGN_{ch}": bitvec[1];
            attribute "A_RXENPRBSTST_{ch}": bitvec[3];
            attribute "A_RXEQMIX_{ch}": bitvec[2];
            attribute "A_RXPMASETPHASE_{ch}": bitvec[1];
            attribute "A_RXPOLARITY_{ch}": bitvec[1];
            attribute "A_RXPOWERDOWN_{ch}": bitvec[2];
            attribute "A_RXRESET_{ch}": bitvec[1];
            attribute "A_TXBUFDIFFCTRL_{ch}": bitvec[3];
            attribute "A_TXDIFFCTRL_{ch}": bitvec[4];
            attribute "A_TXELECIDLE_{ch}": bitvec[1];
            attribute "A_TXENPMAPHASEALIGN_{ch}": bitvec[1];
            attribute "A_TXENPRBSTST_{ch}": bitvec[3];
            attribute "A_TXPMASETPHASE_{ch}": bitvec[1];
            attribute "A_TXPOLARITY_{ch}": bitvec[1];
            attribute "A_TXPOWERDOWN_{ch}": bitvec[2];
            attribute "A_TXPRBSFORCEERR_{ch}": bitvec[1];
            attribute "A_TXPREEMPHASIS_{ch}": bitvec[3];
            attribute "A_TXRESET_{ch}": bitvec[1];
            attribute "CDR_PH_ADJ_TIME_{ch}": bitvec[5];
            attribute "CHAN_BOND_SEQ_1_1_{ch}": bitvec[10];
            attribute "CHAN_BOND_SEQ_1_2_{ch}": bitvec[10];
            attribute "CHAN_BOND_SEQ_1_3_{ch}": bitvec[10];
            attribute "CHAN_BOND_SEQ_1_4_{ch}": bitvec[10];
            attribute "CHAN_BOND_SEQ_1_ENABLE_{ch}": bitvec[4];
            attribute "CHAN_BOND_SEQ_2_1_{ch}": bitvec[10];
            attribute "CHAN_BOND_SEQ_2_2_{ch}": bitvec[10];
            attribute "CHAN_BOND_SEQ_2_3_{ch}": bitvec[10];
            attribute "CHAN_BOND_SEQ_2_4_{ch}": bitvec[10];
            attribute "CHAN_BOND_SEQ_2_ENABLE_{ch}": bitvec[4];
            attribute "CLK_COR_SEQ_1_1_{ch}": bitvec[10];
            attribute "CLK_COR_SEQ_1_2_{ch}": bitvec[10];
            attribute "CLK_COR_SEQ_1_3_{ch}": bitvec[10];
            attribute "CLK_COR_SEQ_1_4_{ch}": bitvec[10];
            attribute "CLK_COR_SEQ_1_ENABLE_{ch}": bitvec[4];
            attribute "CLK_COR_SEQ_2_1_{ch}": bitvec[10];
            attribute "CLK_COR_SEQ_2_2_{ch}": bitvec[10];
            attribute "CLK_COR_SEQ_2_3_{ch}": bitvec[10];
            attribute "CLK_COR_SEQ_2_4_{ch}": bitvec[10];
            attribute "CLK_COR_SEQ_2_ENABLE_{ch}": bitvec[4];
            attribute "CM_TRIM_{ch}": bitvec[2];
            attribute "COMMA_10B_ENABLE_{ch}": bitvec[10];
            attribute "COM_BURST_VAL_{ch}": bitvec[4];
            attribute "MCOMMA_10B_VALUE_{ch}": bitvec[10];
            attribute "OOBDETECT_THRESHOLD_{ch}": bitvec[3];
            attribute "PCOMMA_10B_VALUE_{ch}": bitvec[10];
            attribute "PLLLKDET_CFG_{ch}": bitvec[3];
            attribute "RXEQ_CFG_{ch}": bitvec[8];
            attribute "RXPRBSERR_LOOPBACK_{ch}": bitvec[1];
            attribute "RX_IDLE_HI_CNT_{ch}": bitvec[4];
            attribute "RX_IDLE_LO_CNT_{ch}": bitvec[4];
            attribute "SATA_BURST_VAL_{ch}": bitvec[3];
            attribute "SATA_IDLE_VAL_{ch}": bitvec[3];
            attribute "TERMINATION_CTRL_{ch}": bitvec[5];
            attribute "TEST_CLK_OUT_GTP_{ch}": bitvec[2];
            attribute "TXRX_INVERT_{ch}": bitvec[3];
            attribute "TX_IDLE_DELAY_{ch}": bitvec[3];
            attribute "TX_TDCC_CFG_{ch}": bitvec[2];
            attribute "USR_CODE_ERR_CLR_{ch}": bitvec[1];

            attribute "CB2_INH_CC_PERIOD_{ch}": bitvec[4];
            attribute "CLK_COR_REPEAT_WAIT_{ch}": bitvec[5];

            attribute "PLL_COM_CFG_{ch}": bitvec[24];
            attribute "PLL_CP_CFG_{ch}": bitvec[8];
            attribute "PMA_CDR_SCAN_{ch}": bitvec[27];
            attribute "PMA_RXSYNC_CFG_{ch}": bitvec[7];
            attribute "PMA_RX_CFG_{ch}": bitvec[25];
            attribute "PMA_TX_CFG_{ch}": bitvec[20];
            attribute "TRANS_TIME_FROM_P2_{ch}": bitvec[12];
            attribute "TRANS_TIME_NON_P2_{ch}": bitvec[8];
            attribute "TRANS_TIME_TO_P2_{ch}": bitvec[10];
            attribute "TST_ATTR_{ch}": bitvec[32];
            attribute "TX_DETECT_RX_CFG_{ch}": bitvec[14];

            attribute "CHAN_BOND_1_MAX_SKEW_{ch}": bitvec[4];
            attribute "CHAN_BOND_2_MAX_SKEW_{ch}": bitvec[4];
            attribute "CLK_COR_MAX_LAT_{ch}": bitvec[6];
            attribute "CLK_COR_MIN_LAT_{ch}": bitvec[6];
            attribute "SATA_MAX_BURST_{ch}": bitvec[6];
            attribute "SATA_MAX_INIT_{ch}": bitvec[6];
            attribute "SATA_MAX_WAKE_{ch}": bitvec[6];
            attribute "SATA_MIN_BURST_{ch}": bitvec[6];
            attribute "SATA_MIN_INIT_{ch}": bitvec[6];
            attribute "SATA_MIN_WAKE_{ch}": bitvec[6];

            attribute "ALIGN_COMMA_WORD_{ch}": GTP_ALIGN_COMMA_WORD;
            attribute "CHAN_BOND_SEQ_LEN_{ch}": GTP_CHAN_BOND_SEQ_LEN;
            attribute "CLK_COR_ADJ_LEN_{ch}": GTP_CLK_COR_ADJ_LEN;
            attribute "CLK_COR_DET_LEN_{ch}": GTP_CLK_COR_DET_LEN;
            attribute "CLK25_DIVIDER_{ch}": GTP_CLK25_DIVIDER;
            attribute "OOB_CLK_DIVIDER_{ch}": GTP_OOB_CLK_DIVIDER;
            attribute "PLL_DIVSEL_FB_{ch}": GTP_PLL_DIVSEL_FB;
            attribute "PLL_DIVSEL_REF_{ch}": GTP_PLL_DIVSEL_REF;
            attribute "PLL_RXDIVSEL_OUT_{ch}": GTP_PLL_DIVSEL_OUT;
            attribute "PLL_TXDIVSEL_OUT_{ch}": GTP_PLL_DIVSEL_OUT;
            attribute "PLL_SOURCE_{ch}": GTP_PLL_SOURCE;
            attribute "RX_LOS_INVALID_INCR_{ch}": GTP_RX_LOS_INVALID_INCR;
            attribute "RX_LOS_THRESHOLD_{ch}": GTP_RX_LOS_THRESHOLD;
            attribute "RX_SLIDE_MODE_{ch}": GTP_RX_SLIDE_MODE;
            attribute "RX_STATUS_FMT_{ch}": GTP_RX_STATUS_FMT;
            attribute "RX_XCLK_SEL_{ch}": GTP_RX_XCLK_SEL;
            attribute "TX_XCLK_SEL_{ch}": GTP_TX_XCLK_SEL;
            attribute "CLK_OUT_GTP_SEL_{ch}": "GTP_CLK_OUT_GTP_SEL_{ch}";
        }
    }

    bel_class GLUTMASK_HCLK {
        attribute FRAME21: bool;
        attribute FRAME22: bool;
        attribute FRAME23: bool;
        attribute FRAME24: bool;
        attribute FRAME25: bool;
        attribute FRAME26: bool;
        attribute FRAME27: bool;
        attribute FRAME28: bool;
        attribute FRAME29: bool;
        attribute FRAME30: bool;
    }

    enum OCT_CAL_VREF_VALUE { NONE, _0P25, _0P5, _0P75 }
    enum OCT_CAL_ACCESS_MODE { STATIC, USER }
    bel_class OCT_CAL {
        input S[2];
        attribute VREF_VALUE: OCT_CAL_VREF_VALUE;
        attribute ACCESS_MODE: OCT_CAL_ACCESS_MODE;
    }

    bel_class BANK {
        attribute LVDSBIAS: bitvec[12][2];
    }

    bel_class MISR {
        attribute ENABLE: bool;
        attribute RESET: bool;
    }

    bel_class PMV {
        input SELECTB[6];
        input ENABLEB;
        output OUT, OUT_DIV2, OUT_DIV4;

        attribute PSLEW, NSLEW: bitvec[4];
    }

    enum DNA_PORT_OPTIONS { READ, PROGRAM, ANALOG_READ }
    bel_class DNA_PORT {
        input CLK, READ, SHIFT, TEST, DIN;
        output DOUT;
        attribute ENABLE: bool;
        attribute OPTIONS: DNA_PORT_OPTIONS;
    }

    bel_class ICAP {
        input CLK, CE, WRITE;
        input I[16];
        output BUSY;
        output O[16];
        attribute ENABLE: bool;
    }

    bel_class SPI_ACCESS {
        input CLK, CSB, MOSI;
        output MISO;
        attribute ENABLE: bool;
    }

    bel_class SUSPEND_SYNC {
        input CLK, SACK;
        output SREQ;
        attribute ENABLE: bool;
    }

    bel_class POST_CRC_INTERNAL {
        output CRCERROR;
    }

    bel_class STARTUP {
        input CLK, GSR, GTS, KEYCLEARB;
        output EOS, CFGCLK, CFGMCLK;

        attribute USER_GTS_GSR_ENABLE: bool;
        attribute GTS_SYNC: bool;
        attribute GSR_SYNC: bool;
        attribute CFGCLK_ENABLE: bool;
        attribute CFGMCLK_ENABLE: bool;
        attribute KEYCLEARB_ENABLE: bool;
    }

    bel_class SLAVE_SPI {
        input CMPMISO;
        output CMPACTIVEB, CMPCLK, CMPCSB, CMPMOSI;
    }

    bel_class BSCAN {
        input TDO;
        output TCK, TMS, TDI;
        output DRCK, SEL, RESET, RUNTEST, CAPTURE, SHIFT, UPDATE;
        attribute ENABLE: bool;
    }

    bel_class MISC_SW {
        pad PROG_B: input;
        pad MISO2: input;

        attribute PROG_PULL: IOB_PULL;
        attribute MISO2_PULL: IOB_PULL;

        attribute LEAKER_GAIN_OPTIONS: bitvec[4];
        attribute LEAKER_SLOPE_OPTIONS: bitvec[4];
        attribute VBG_SLOPE_OPTIONS: bitvec[4];
        attribute VGG_SLOPE_OPTIONS: bitvec[4];
        attribute VGG_TEST_OPTIONS: bitvec[3];
        attribute VGG_COMP_OPTION: bitvec[1];
    }

    bel_class MISC_SE {
        pad DONE: inout;
        pad SUSPEND: input;
        pad CMP_CS_B: input;
        pad CCLK2: output;
        pad MOSI2: output;

        attribute DONE_PULL: IOB_PULL;
        attribute CMP_CS_PULL: IOB_PULL;
        attribute CCLK2_PULL: IOB_PULL;
        attribute MOSI2_PULL: IOB_PULL;

        attribute GLUTMASK_IOB: bool;
    }

    bel_class MISC_NW {
        // ????
        pad M2: input;
        pad SELECTHS: input;

        attribute M2_PULL: IOB_PULL;
        attribute SELECTHS_PULL: IOB_PULL;
        attribute VREF_LV: bitvec[2];
    }

    bel_class MISC_NE {
        pad TCK, TMS, TDI: input;
        pad TDO: output;
        pad CSO2: output;

        attribute CSO2_PULL: IOB_PULL;
        attribute TCK_PULL: IOB_PULL;
        attribute TMS_PULL: IOB_PULL;
        attribute TDI_PULL: IOB_PULL;
        attribute TDO_PULL: IOB_PULL;
        attribute JTAG_TEST: bool;
        attribute USERCODE: bitvec[32];
    }

    enum STARTUP_CYCLE { _1, _2, _3, _4, _5, _6, DONE, KEEP, NOWAIT }
    enum STARTUP_CLOCK { CCLK, USERCLK, JTAGCLK }
    enum SECURITY { NONE, LEVEL1, LEVEL2, LEVEL3 }
    enum ENCRYPT_KEY_SELECT { BBRAM, EFUSE }
    enum SW_CLK { INTERNALCLK, STARTUPCLK }
    enum SPI_BUSWIDTH { _1, _2, _4 }
    bel_class GLOBAL {
        // COR
        attribute GWE_CYCLE: STARTUP_CYCLE;
        attribute GTS_CYCLE: STARTUP_CYCLE;
        attribute LOCK_CYCLE: STARTUP_CYCLE;
        attribute DONE_CYCLE: STARTUP_CYCLE;
        attribute BPI_DIV8: bool;
        attribute BPI_DIV16: bool;
        attribute RESET_ON_ERR: bool;
        attribute DISABLE_VRD_REG: bool;
        attribute DRIVE_DONE: bool;
        attribute DONE_PIPE: bool;
        attribute DRIVE_AWAKE: bool;
        attribute CRC_ENABLE: bool;
        attribute VRDSEL: bitvec[3];
        attribute SEND_VGG: bitvec[4];
        attribute VGG_ENABLE_OFFCHIP: bool;
        attribute VGG_SENDMAX: bool;
        attribute STARTUP_CLOCK: STARTUP_CLOCK;

        // CTL
        attribute GTS_USR_B: bool;
        attribute POST_CRC_INIT_FLAG: bool;
        attribute MULTIBOOT_ENABLE: bool;
        attribute SECURITY: SECURITY;
        attribute PERSIST: bool;
        attribute ENCRYPT: bool;
        attribute ENCRYPT_KEY_SELECT: ENCRYPT_KEY_SELECT;

        // CCLK_FREQ
        // CONFIG_RATE = 400 / (CONFIG_RATE_DIV + 1)
        attribute CONFIG_RATE_DIV: bitvec[10];
        attribute CCLK_DLY: bitvec[2];
        attribute CCLK_SEP: bitvec[2];
        attribute EXT_CCLK_ENABLE: bool;

        // HC_OPT
        attribute HC_CYCLE: bitvec[4];
        attribute TWO_ROUND: bool;
        attribute BRAM_SKIP: bool;
        attribute INIT_SKIP: bool;

        // POWERDOWN
        attribute SW_CLK: SW_CLK;
        attribute EN_SUSPEND: bool;
        attribute EN_SW_GSR: bool;
        attribute SUSPEND_FILTER: bool;
        attribute MULTIPIN_WAKEUP: bool;
        attribute WAKE_DELAY1: bitvec[3];
        attribute WAKE_DELAY2: bitvec[5];
        attribute SW_GWE_CYCLE: bitvec[10];
        attribute SW_GTS_CYCLE: bitvec[10];
        attribute WAKEUP_MASK: bitvec[8];

        // MODE
        attribute NEXT_CONFIG_BOOT_MODE: bitvec[3];
        attribute NEXT_CONFIG_NEW_MODE: bool;
        attribute SPI_BUSWIDTH: SPI_BUSWIDTH;

        attribute NEXT_CONFIG_ADDR: bitvec[32];
        attribute GOLDEN_CONFIG_ADDR: bitvec[32];
        attribute FAILSAFE_USER: bitvec[16];
        attribute TIMER_CFG: bitvec[16];

        // SEU_OPT
        attribute POST_CRC_EN: bool;
        attribute GLUTMASK: bool;
        attribute POST_CRC_KEEP: bool;
        attribute POST_CRC_ONESHOT: bool;
        attribute POST_CRC_SEL: bool;
        // FREQ = 400 / (POST_CRC_FREQ + 1)
        attribute POST_CRC_FREQ_DIV: bitvec[10];

        // TESTMODE
        attribute VGG_TEST: bool;
        attribute ICAP_BYPASS: bool;
        attribute TESTMODE_EN: bool;
    }

    device_data IDCODE: bitvec[32];

    region_slot GLOBAL;
    region_slot PLLCLK;
    region_slot IOCLK;
    region_slot DIVCLK_CMT;
    region_slot HROW;
    region_slot LEAF;

    wire PULLUP: pullup;
    wire TIE_0: tie 0;
    wire TIE_1: tie 1;
    wire HCLK[16]: regional LEAF;

    wire SNG_W0[8]: mux;
    wire SNG_W1[8]: branch E;
    wire SNG_W1_N3: branch S;
    wire SNG_W1_S4: branch N;
    wire SNG_E0[8]: mux;
    wire SNG_E1[8]: branch W;
    wire SNG_E1_S0: branch N;
    wire SNG_E1_N7: branch S;
    wire SNG_S0[8]: mux;
    wire SNG_S1[8]: branch N;
    wire SNG_S1_N7: branch S;
    wire SNG_N0[8]: mux;
    wire SNG_N1[8]: branch S;
    wire SNG_N1_S0: branch N;

    wire DBL_WW0[4]: mux;
    wire DBL_WW1[4]: branch E;
    wire DBL_WW2[4]: branch E;
    wire DBL_WW2_N3: branch S;
    wire DBL_EE0[4]: mux;
    wire DBL_EE1[4]: branch W;
    wire DBL_EE2[4]: branch W;
    wire DBL_SS0[4]: mux;
    wire DBL_SS1[4]: branch N;
    wire DBL_SS2[4]: branch N;
    wire DBL_SS2_N3: branch S;
    wire DBL_SW0[4]: mux;
    wire DBL_SW1[4]: branch N;
    wire DBL_SW2[4]: branch E;
    wire DBL_SW2_N3: branch S;
    wire DBL_SE0[4]: mux;
    wire DBL_SE1[4]: branch N;
    wire DBL_SE2[4]: branch W;
    wire DBL_NN0[4]: mux;
    wire DBL_NN1[4]: branch S;
    wire DBL_NN2[4]: branch S;
    wire DBL_NN2_S0: branch N;
    wire DBL_NW0[4]: mux;
    wire DBL_NW1[4]: branch S;
    wire DBL_NW2[4]: branch E;
    wire DBL_NW2_S0: branch N;
    wire DBL_NE0[4]: mux;
    wire DBL_NE1[4]: branch S;
    wire DBL_NE2[4]: branch W;
    wire DBL_NE2_S0: branch N;

    wire QUAD_WW0[4]: mux;
    wire QUAD_WW1[4]: branch E;
    wire QUAD_WW2[4]: branch E;
    wire QUAD_WW3[4]: branch E;
    wire QUAD_WW4[4]: branch E;
    wire QUAD_WW4_S0: branch N;
    wire QUAD_EE0[4]: mux;
    wire QUAD_EE1[4]: branch W;
    wire QUAD_EE2[4]: branch W;
    wire QUAD_EE3[4]: branch W;
    wire QUAD_EE4[4]: branch W;
    wire QUAD_SS0[4]: mux;
    wire QUAD_SS1[4]: branch N;
    wire QUAD_SS2[4]: branch N;
    wire QUAD_SS3[4]: branch N;
    wire QUAD_SS4[4]: branch N;
    wire QUAD_SS4_N3: branch S;
    wire QUAD_SW0[4]: mux;
    wire QUAD_SW1[4]: branch N;
    wire QUAD_SW2[4]: branch N;
    wire QUAD_SW3[4]: branch E;
    wire QUAD_SW4[4]: branch E;
    wire QUAD_SW4_N3: branch S;
    wire QUAD_SE0[4]: mux;
    wire QUAD_SE1[4]: branch N;
    wire QUAD_SE2[4]: branch N;
    wire QUAD_SE3[4]: branch W;
    wire QUAD_SE4[4]: branch W;
    wire QUAD_NN0[4]: mux;
    wire QUAD_NN1[4]: branch S;
    wire QUAD_NN2[4]: branch S;
    wire QUAD_NN3[4]: branch S;
    wire QUAD_NN4[4]: branch S;
    wire QUAD_NW0[4]: mux;
    wire QUAD_NW1[4]: branch S;
    wire QUAD_NW2[4]: branch S;
    wire QUAD_NW3[4]: branch E;
    wire QUAD_NW4[4]: branch E;
    wire QUAD_NW4_S0: branch N;
    wire QUAD_NE0[4]: mux;
    wire QUAD_NE1[4]: branch S;
    wire QUAD_NE2[4]: branch S;
    wire QUAD_NE3[4]: branch W;
    wire QUAD_NE4[4]: branch W;

    wire IMUX_GFAN[2]: mux;
    wire IMUX_CLK[2]: mux;
    wire IMUX_SR[2]: mux;
    wire IMUX_LOGICIN[64]: mux;
    wire IMUX_LOGICIN20_BOUNCE: mux;
    wire IMUX_LOGICIN36_BOUNCE: mux;
    wire IMUX_LOGICIN44_BOUNCE: mux;
    wire IMUX_LOGICIN62_BOUNCE: mux;
    wire IMUX_LOGICIN21_BOUNCE: mux;
    wire IMUX_LOGICIN28_BOUNCE: mux;
    wire IMUX_LOGICIN52_BOUNCE: mux;
    wire IMUX_LOGICIN60_BOUNCE: mux;
    wire IMUX_LOGICIN20_S: branch N;
    wire IMUX_LOGICIN36_S: branch N;
    wire IMUX_LOGICIN44_S: branch N;
    wire IMUX_LOGICIN62_S: branch N;
    wire IMUX_LOGICIN21_N: branch S;
    wire IMUX_LOGICIN28_N: branch S;
    wire IMUX_LOGICIN52_N: branch S;
    wire IMUX_LOGICIN60_N: branch S;
    wire OUT[24]: bel;
    wire OUT_BEL[24]: bel;
    wire OUT_TEST[24]: test;
    wire IMUX_CLK_GCLK[2]: mux;

    wire IOCLK[4]: regional IOCLK;
    wire IOCE[4]: regional IOCLK;
    wire PLLCLK[2]: regional PLLCLK;
    wire PLLCE[2]: regional PLLCLK;
    wire GTPCLK[4]: regional IOCLK;
    wire GTPFB[4]: regional IOCLK;

    wire IMUX_BUFIO2_I[4]: mux;
    wire IMUX_BUFIO2_IB[4]: mux;
    wire IMUX_BUFIO2FB[4]: mux;

    wire OUT_DIVCLK[4]: bel;
    wire DIVCLK_CLKC[8]: mux;
    wire DIVCLK_CMT_W[4]: regional DIVCLK_CMT;
    wire DIVCLK_CMT_E[4]: regional DIVCLK_CMT;
    wire DIVCLK_CMT_V[8]: regional DIVCLK_CMT;
    wire IOFBCLK_CMT_W[4]: regional DIVCLK_CMT;
    wire IOFBCLK_CMT_E[4]: regional DIVCLK_CMT;
    wire IOFBCLK_CMT_V[8]: regional DIVCLK_CMT;

    wire OUT_CLKPAD_I[2]: bel;
    wire OUT_CLKPAD_DFB[2]: bel;
    wire OUT_CLKPAD_CFB0[2]: bel;
    wire OUT_CLKPAD_CFB1[2]: bel;
    wire OUT_CLKPAD_DQSP: bel;
    wire OUT_CLKPAD_DQSN: bel;

    wire IMUX_BUFG[16]: mux;
    wire GCLK[16]: regional GLOBAL;

    wire HCLK_ROW[16]: regional HROW;
    wire CMT_OUT[16]: mux;
    wire CMT_CLKC_O[16]: mux;
    wire CMT_CLKC_I[16]: branch CMT_PREV;

    wire CMT_BUFPLL_H_CLKOUT[2]: mux;
    wire CMT_BUFPLL_H_LOCKED[2]: mux;

    wire CMT_BUFPLL_V_CLKOUT_S[6]: multi_root;
    wire CMT_BUFPLL_V_CLKOUT_N[6]: multi_branch CMT_N;
    wire CMT_BUFPLL_V_LOCKED_S[3]: multi_root;
    wire CMT_BUFPLL_V_LOCKED_N[3]: multi_branch CMT_N;

    wire IMUX_BUFPLL_PLLIN[2]: mux;
    wire IMUX_BUFPLL_LOCKED[2]: mux;

    wire IMUX_DCM_CLKIN[2]: mux;
    wire IMUX_DCM_CLKFB[2]: mux;
    wire OMUX_DCM_SKEWCLKIN1[2]: mux;
    wire OMUX_DCM_SKEWCLKIN2[2]: mux;
    wire OUT_DCM_CLK0[2]: bel;
    wire OUT_DCM_CLK90[2]: bel;
    wire OUT_DCM_CLK180[2]: bel;
    wire OUT_DCM_CLK270[2]: bel;
    wire OUT_DCM_CLK2X[2]: bel;
    wire OUT_DCM_CLK2X180[2]: bel;
    wire OUT_DCM_CLKDV[2]: bel;
    wire OUT_DCM_CLKFX[2]: bel;
    wire OUT_DCM_CLKFX180[2]: bel;
    wire OUT_DCM_CONCUR[2]: bel;

    wire IMUX_PLL_CLKIN1: mux;
    wire IMUX_PLL_CLKIN2: mux;
    wire IMUX_PLL_CLKFB: mux;
    wire TEST_PLL_CLKIN: bel;
    wire OMUX_PLL_SKEWCLKIN1: mux;
    wire OMUX_PLL_SKEWCLKIN2: mux;
    wire OUT_PLL_CLKOUT[6]: bel;
    wire OUT_PLL_CLKOUTDCM[6]: bel;
    wire OUT_PLL_CLKFBOUT: bel;
    wire OUT_PLL_CLKFBDCM: bel;
    wire OUT_PLL_LOCKED: bel;

    wire OMUX_PLL_SKEWCLKIN1_BUF: mux;
    wire OMUX_PLL_SKEWCLKIN2_BUF: mux;
    wire CMT_TEST_CLK: mux;

    wire IOI_IOCLK[6]: mux;
    wire IOI_IOCLK_OPTINV[6]: mux;
    wire IOI_IOCE[4]: mux;
    wire OUT_DDR_IOCLK[2]: bel;
    wire OUT_DDR_IOCE[2]: bel;
    wire IOI_ICLK[2]: mux;
    wire IOI_OCLK[2]: mux;
    wire IMUX_ILOGIC_CLK[2]: mux;
    wire IMUX_OLOGIC_CLK[2]: mux;
    wire IMUX_IODELAY_IOCLK[2]: mux;
    wire IMUX_ILOGIC_IOCE[2]: mux;
    wire IMUX_OLOGIC_IOCE[2]: mux;

    bitrect INT = vertical (22, rev 64);
    bitrect CLEXL = vertical (30, rev 64);
    bitrect CLEXM = vertical (31, rev 64);
    bitrect CLE_CLK = vertical (31, rev 64);
    bitrect BRAM = vertical (25, rev 64);
    bitrect DSP = vertical (24, rev 64);
    bitrect CLK_V = vertical (4, rev 64);
    bitrect HCLK = vertical (22, rev 16);
    bitrect BRAM_DATA = vertical (1, 18720);
    bitrect IOB = vertical (1, 128);
    bitrect CLK = vertical (1, 384);

    bitrect REG16 = horizontal (1, rev 16);

    tile_slot INT {
        bel_slot INT: routing;

        tile_class INT, INT_IOI {
            cell CELL;
            bitrect MAIN: INT;
        }
    }

    tile_slot INTF {
        bel_slot INTF_INT: routing;
        bel_slot INTF_TESTMUX: routing;

        tile_class INTF, INTF_IOI, INTF_CMT, INTF_CMT_IOI {
            cell CELL;
            if tile_class [INTF, INTF_IOI] {
                bitrect MAIN: INT;
            } else {
                bitrect MAIN: CLE_CLK;
            }
        }
    }

    tile_slot BEL {
        bel_slot SLICE[2]: SLICE;
        tile_class CLEXL {
            cell CELL;
            bitrect MAIN: CLEXL;
        }
        tile_class CLEXM {
            cell CELL;
            bitrect MAIN: CLEXM;
        }

        bel_slot BRAM[2]: BRAM;
        tile_class BRAM {
            cell CELL[4];
            bitrect MAIN[4]: BRAM;
            bitrect DATA: BRAM_DATA;
        }

        bel_slot DSP: DSP;
        tile_class DSP {
            cell CELL[4];
            bitrect MAIN[4]: DSP;
        }

        bel_slot IOI_INT: routing;
        bel_slot IOI_DDR[2]: IOI_DDR;
        bel_slot ILOGIC[2]: ILOGIC;
        bel_slot OLOGIC[2]: OLOGIC;
        bel_slot IODELAY[2]: IODELAY;
        bel_slot MISC_IOI: MISC_IOI;

        tile_class IOI_WE, IOI_SN {
            cell CELL;
            bitrect MAIN: CLEXL;

            switchbox IOI_INT {
                mux IOI_IOCLK[0] = IMUX_GFAN[1] | IMUX_CLK[1] | IOCLK[0] | IOCLK[2] | PLLCLK[0];
                mux IOI_IOCLK[1] = IMUX_GFAN[1] | IMUX_CLK[1] | IOCLK[1] | IOCLK[3] | PLLCLK[1];
                mux IOI_IOCLK[2] = PLLCLK[0] | PLLCLK[1];
                mux IOI_IOCLK[3] = IMUX_GFAN[0] | IMUX_CLK[0] | IOCLK[0] | IOCLK[2] | PLLCLK[0];
                mux IOI_IOCLK[4] = IMUX_GFAN[0] | IMUX_CLK[0] | IOCLK[1] | IOCLK[3] | PLLCLK[1];
                mux IOI_IOCLK[5] = PLLCLK[0] | PLLCLK[1];
                proginv IOI_IOCLK_OPTINV[0] = IOI_IOCLK[0];
                proginv IOI_IOCLK_OPTINV[1] = IOI_IOCLK[1];
                proginv IOI_IOCLK_OPTINV[2] = IOI_IOCLK[2];
                proginv IOI_IOCLK_OPTINV[3] = IOI_IOCLK[3];
                proginv IOI_IOCLK_OPTINV[4] = IOI_IOCLK[4];
                proginv IOI_IOCLK_OPTINV[5] = IOI_IOCLK[5];
                mux IOI_IOCE[0] = IOCE[0] | IOCE[2] | PLLCE[0];
                mux IOI_IOCE[1] = IOCE[1] | IOCE[3] | PLLCE[1];
                mux IOI_IOCE[2] = IOCE[0] | IOCE[2] | PLLCE[0];
                mux IOI_IOCE[3] = IOCE[1] | IOCE[3] | PLLCE[1];

                mux IOI_ICLK[0] = IOI_IOCLK[0] | IOI_IOCLK[1] | IOI_IOCLK[2] | OUT_DDR_IOCLK[0];
                mux IOI_ICLK[1] = IOI_IOCLK[3] | IOI_IOCLK[4] | IOI_IOCLK[5] | OUT_DDR_IOCLK[1];
                mux IOI_OCLK[0] = IOI_IOCLK[0] | IOI_IOCLK[1] | IOI_IOCLK[2] | OUT_DDR_IOCLK[0];
                mux IOI_OCLK[1] = IOI_IOCLK[3] | IOI_IOCLK[4] | IOI_IOCLK[5] | OUT_DDR_IOCLK[1];

                mux IMUX_ILOGIC_IOCE[0] = IOI_IOCE[0] | IOI_IOCE[1] | OUT_DDR_IOCE[0];
                mux IMUX_ILOGIC_IOCE[1] = IOI_IOCE[2] | IOI_IOCE[3] | OUT_DDR_IOCE[1];
                mux IMUX_OLOGIC_IOCE[0] = IOI_IOCE[0] | IOI_IOCE[1] | OUT_DDR_IOCE[0];
                mux IMUX_OLOGIC_IOCE[1] = IOI_IOCE[2] | IOI_IOCE[3] | OUT_DDR_IOCE[1];

                mux IMUX_ILOGIC_CLK[0] = IOI_ICLK[0] | IOI_ICLK[1];
                mux IMUX_ILOGIC_CLK[1] = IOI_ICLK[0] | IOI_ICLK[1];
                mux IMUX_OLOGIC_CLK[0] = IOI_OCLK[0] | IOI_OCLK[1];
                mux IMUX_OLOGIC_CLK[1] = IOI_OCLK[0] | IOI_OCLK[1];
                mux IMUX_IODELAY_IOCLK[0] = IMUX_ILOGIC_CLK[0] | IMUX_OLOGIC_CLK[0];
                mux IMUX_IODELAY_IOCLK[1] = IMUX_ILOGIC_CLK[1] | IMUX_OLOGIC_CLK[1];
            }

            bel IOI_DDR[0] {
                input CLK0 = IOI_IOCLK[0];
                input CLK1 = IOI_IOCLK[1];
                output CLK = OUT_DDR_IOCLK[0];
                output IOCE = OUT_DDR_IOCE[0];
            }

            bel IOI_DDR[1] {
                input CLK0 = IOI_IOCLK[3];
                input CLK1 = IOI_IOCLK[4];
                output CLK = OUT_DDR_IOCLK[1];
                output IOCE = OUT_DDR_IOCE[1];
            }

        }

        bel_slot DCM[2]: DCM;
        bel_slot CMT_VREG: CMT_VREG;
        bel_slot PLL: PLL;
        bel_slot CMT_INT: routing;
        tile_class CMT_DCM {
            cell CELL[2];
            cell CELL_PLL;
            bitrect MAIN[16]: CLE_CLK;
            bitrect CLK[16]: CLK_V;
        }
        tile_class CMT_PLL {
            cell CELL[2];
            cell CELL_DCM;
            bitrect MAIN[16]: CLE_CLK;
            bitrect CLK[16]: CLK_V;
        }

        bel_slot MCB: MCB;
        tile_class MCB {
            cell CELL[12];
            cell CELL_MUI[16];
            bitrect MAIN[12]: CLEXL;
            bitrect MAIN_MUI[16]: CLEXL;
        }

        bel_slot PCIE: PCIE;
        tile_class PCIE {
            cell W[16];
            cell E[16];
            bitrect MAIN_W[16]: CLEXM;
            bitrect MAIN_E[16]: CLEXL;
        }

        bel_slot GTP: GTP;
        tile_class GTP {
            cell W[8];
            cell E[8];
            bitrect MAIN_W[8]: CLEXL;
            bitrect MAIN_E[8]: CLEXM;
        }

        bel_slot PCILOGICSE: PCILOGICSE;
        tile_class PCILOGICSE {
            cell CELL;
            bitrect MAIN: CLEXL;
        }

        bel_slot OCT_CAL[6]: OCT_CAL;
        bel_slot PMV: PMV;
        bel_slot DNA_PORT: DNA_PORT;
        bel_slot ICAP: ICAP;
        bel_slot SPI_ACCESS: SPI_ACCESS;
        bel_slot SUSPEND_SYNC: SUSPEND_SYNC;
        bel_slot POST_CRC_INTERNAL: POST_CRC_INTERNAL;
        bel_slot STARTUP: STARTUP;
        bel_slot SLAVE_SPI: SLAVE_SPI;
        bel_slot BSCAN[4]: BSCAN;
        bel_slot MISR_CNR_H: MISR;
        bel_slot MISR_CNR_V: MISR;
        bel_slot BANK[6]: BANK;
        bel_slot MISC_SW: MISC_SW;
        bel_slot MISC_SE: MISC_SE;
        bel_slot MISC_NW: MISC_NW;
        bel_slot MISC_NE: MISC_NE;
        tile_class CNR_SW, CNR_NW {
            cell CELL;
            bitrect MAIN: CLEXL;
        }
        tile_class CNR_SE, CNR_NE {
            cell CELL[2];
            bitrect MAIN[2]: CLEXL;
        }

        bel_slot BUFGMUX[16]: BUFGMUX;
        bel_slot CLKC_INT: routing;
        tile_class CLKC {
            cell S, N, EDGE_W, EDGE_E, EDGE_S, EDGE_N;
            bitrect MAIN: CLE_CLK;
        }
    }

    tile_slot IOB {
        bel_slot IOB[2]: IOB;
        tile_class IOB {
            cell CELL;
            bitrect MAIN: IOB;
        }
    }

    tile_slot HCLK {
        bel_slot HCLK: routing;
        tile_class HCLK {
            cell S, N;
            bitrect MAIN: HCLK;
        }
    }

    tile_slot HCLK_BEL {
        bel_slot GLUTMASK_HCLK: GLUTMASK_HCLK;
        // used in CLEXL (incl. spine) and DSP columns; also used on PCIE sides and left GTP side
        tile_class HCLK_CLEXL {
            bitrect MAIN: HCLK;
            bel GLUTMASK_HCLK {
                attribute FRAME21 @MAIN[16][0];
                attribute FRAME22 @MAIN[17][0];
                attribute FRAME23 @MAIN[18][0];
                attribute FRAME24 @MAIN[19][0];
                attribute FRAME26 @MAIN[16][1];
                attribute FRAME27 @MAIN[17][1];
                attribute FRAME28 @MAIN[18][1];
                attribute FRAME29 @MAIN[19][1];
            }
        }
        // used in CLEXM columns
        tile_class HCLK_CLEXM {
            bitrect MAIN: HCLK;
            bel GLUTMASK_HCLK {
                attribute FRAME21 @MAIN[16][0];
                attribute FRAME22 @MAIN[17][0];
                attribute FRAME24 @MAIN[18][0];
                attribute FRAME25 @MAIN[19][0];
                attribute FRAME27 @MAIN[16][1];
                attribute FRAME28 @MAIN[17][1];
                attribute FRAME29 @MAIN[18][1];
                attribute FRAME30 @MAIN[19][1];
            }
        }
        // used in IOI columns
        tile_class HCLK_IOI {
            bitrect MAIN: HCLK;
            bel GLUTMASK_HCLK {
                attribute FRAME25 @MAIN[16][0];
                attribute FRAME23 @MAIN[18][0];
                attribute FRAME24 @MAIN[19][0];
                attribute FRAME21 @MAIN[16][1];
                attribute FRAME27 @MAIN[17][1];
                attribute FRAME28 @MAIN[18][1];
                attribute FRAME29 @MAIN[19][1];
            }
        }
        // used on right GTP side
        tile_class HCLK_GTP {
            bitrect MAIN: HCLK;
            bel GLUTMASK_HCLK {
                attribute FRAME25 @MAIN[16][0];
                attribute FRAME22 @MAIN[17][0];
                attribute FRAME23 @MAIN[18][0];
                attribute FRAME24 @MAIN[19][0];
            }
        }
    }

    tile_slot HCLK_ROW {
        bel_slot HCLK_ROW: routing;
        tile_class HCLK_ROW {
            cell W, E;
            bitrect MAIN: CLK_V;
        }
    }

    tile_slot CLK {
        bel_slot CLK_INT: routing;
        bel_slot BUFIO2[8]: BUFIO2;
        bel_slot BUFIO2FB[8]: BUFIO2FB;
        bel_slot BUFPLL: BUFPLL;
        bel_slot MISR_CLK: MISR;

        tile_class CLK_W, CLK_E {
            cell CELL[6];
            bitrect MAIN: CLK;
        }
        tile_class CLK_S, CLK_N {
            cell CELL[4];
            bitrect MAIN: CLK;
        }
    }

    tile_slot CMT_BUF {
        bel_slot CMT_BUF: routing;

        tile_class
            DCM_BUFPLL_BUF_S,
            DCM_BUFPLL_BUF_S_MID,
            DCM_BUFPLL_BUF_N,
            DCM_BUFPLL_BUF_N_MID
        {
            cell CELL;
            bitrect MAIN: CLK_V;
        }

        tile_class
            PLL_BUFPLL_OUT0_S,
            PLL_BUFPLL_OUT0_N,
            PLL_BUFPLL_OUT1_S,
            PLL_BUFPLL_OUT1_N,
            PLL_BUFPLL_S,
            PLL_BUFPLL_N
        {
            cell CELL;
            bitrect MAIN: CLK_V;
        }
    }

    tile_slot GLOBAL {
        bel_slot GLOBAL: GLOBAL;
        tile_class GLOBAL {
            bitrect COR1: REG16;
            bitrect COR2: REG16;
            bitrect CTL: REG16;
            bitrect CCLK_FREQ: REG16;
            bitrect HC_OPT: REG16;
            bitrect POWERDOWN: REG16;
            bitrect PU_GWE: REG16;
            bitrect PU_GTS: REG16;
            bitrect MODE: REG16;
            bitrect GENERAL1: REG16;
            bitrect GENERAL2: REG16;
            bitrect GENERAL3: REG16;
            bitrect GENERAL4: REG16;
            bitrect GENERAL5: REG16;
            bitrect SEU_OPT: REG16;
            bitrect EYE_MASK: REG16;
            bitrect TIMER: REG16;
            bitrect TESTMODE: REG16;
            bel GLOBAL;
        }
    }

    connector_slot W {
        opposite E;

        connector_class PASS_W {
            pass SNG_E1 = SNG_E0;
            pass DBL_EE1 = DBL_EE0;
            pass DBL_EE2 = DBL_EE1;
            pass DBL_SE2 = DBL_SE1;
            pass DBL_NE2 = DBL_NE1;
            pass QUAD_EE1 = QUAD_EE0;
            pass QUAD_EE2 = QUAD_EE1;
            pass QUAD_EE3 = QUAD_EE2;
            pass QUAD_EE4 = QUAD_EE3;
            pass QUAD_SE3 = QUAD_SE2;
            pass QUAD_SE4 = QUAD_SE3;
            pass QUAD_NE3 = QUAD_NE2;
            pass QUAD_NE4 = QUAD_NE3;

        }
        connector_class TERM_W;
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            pass SNG_W1 = SNG_W0;
            pass DBL_WW1 = DBL_WW0;
            pass DBL_WW2 = DBL_WW1;
            pass DBL_SW2 = DBL_SW1;
            pass DBL_NW2 = DBL_NW1;
            pass QUAD_WW1 = QUAD_WW0;
            pass QUAD_WW2 = QUAD_WW1;
            pass QUAD_WW3 = QUAD_WW2;
            pass QUAD_WW4 = QUAD_WW3;
            pass QUAD_SW3 = QUAD_SW2;
            pass QUAD_SW4 = QUAD_SW3;
            pass QUAD_NW3 = QUAD_NW2;
            pass QUAD_NW4 = QUAD_NW3;
        }
        connector_class TERM_E;
    }

    connector_slot S {
        opposite N;

        connector_class PASS_S {
            pass SNG_N1 = SNG_N0;
            pass SNG_W1_N3 = SNG_W1[3];
            pass SNG_E1_N7 = SNG_E1[7];
            pass SNG_S1_N7 = SNG_S1[7];
            pass DBL_NN1 = DBL_NN0;
            pass DBL_NN2 = DBL_NN1;
            pass DBL_NW1 = DBL_NW0;
            pass DBL_NE1 = DBL_NE0;
            pass DBL_WW2_N3 = DBL_WW2[3];
            pass DBL_SS2_N3 = DBL_SS2[3];
            pass DBL_SW2_N3 = DBL_SW2[3];
            pass QUAD_NN1 = QUAD_NN0;
            pass QUAD_NN2 = QUAD_NN1;
            pass QUAD_NN3 = QUAD_NN2;
            pass QUAD_NN4 = QUAD_NN3;
            pass QUAD_NW1 = QUAD_NW0;
            pass QUAD_NW2 = QUAD_NW1;
            pass QUAD_NE1 = QUAD_NE0;
            pass QUAD_NE2 = QUAD_NE1;
            pass QUAD_SS4_N3 = QUAD_SS4[3];
            pass QUAD_SW4_N3 = QUAD_SW4[3];
            pass IMUX_LOGICIN21_N = IMUX_LOGICIN21_BOUNCE;
            pass IMUX_LOGICIN28_N = IMUX_LOGICIN28_BOUNCE;
            pass IMUX_LOGICIN52_N = IMUX_LOGICIN52_BOUNCE;
            pass IMUX_LOGICIN60_N = IMUX_LOGICIN60_BOUNCE;
        }
        connector_class TERM_S;
    }

    connector_slot N {
        opposite S;

        connector_class PASS_N {
            pass SNG_S1 = SNG_S0;
            pass SNG_W1_S4 = SNG_W1[4];
            pass SNG_E1_S0 = SNG_E1[0];
            pass SNG_N1_S0 = SNG_N1[0];
            pass DBL_SS1 = DBL_SS0;
            pass DBL_SS2 = DBL_SS1;
            pass DBL_SW1 = DBL_SW0;
            pass DBL_SE1 = DBL_SE0;
            pass DBL_NN2_S0 = DBL_NN2[0];
            pass DBL_NW2_S0 = DBL_NW2[0];
            pass DBL_NE2_S0 = DBL_NE2[0];
            pass QUAD_SS1 = QUAD_SS0;
            pass QUAD_SS2 = QUAD_SS1;
            pass QUAD_SS3 = QUAD_SS2;
            pass QUAD_SS4 = QUAD_SS3;
            pass QUAD_SW1 = QUAD_SW0;
            pass QUAD_SW2 = QUAD_SW1;
            pass QUAD_SE1 = QUAD_SE0;
            pass QUAD_SE2 = QUAD_SE1;
            pass QUAD_WW4_S0 = QUAD_WW4[0];
            pass QUAD_NW4_S0 = QUAD_NW4[0];
            pass IMUX_LOGICIN20_S = IMUX_LOGICIN20_BOUNCE;
            pass IMUX_LOGICIN36_S = IMUX_LOGICIN36_BOUNCE;
            pass IMUX_LOGICIN44_S = IMUX_LOGICIN44_BOUNCE;
            pass IMUX_LOGICIN62_S = IMUX_LOGICIN62_BOUNCE;
        }
        connector_class TERM_N;
    }

    connector_slot CMT_PREV {
        opposite CMT_NEXT;
        connector_class CMT_PREV {
            pass CMT_CLKC_I = CMT_CLKC_O;
        }
    }
    connector_slot CMT_NEXT {
        opposite CMT_PREV;
        connector_class CMT_NEXT {
        }
    }

    connector_slot CMT_S {
        opposite CMT_N;
        connector_class CMT_S {
        }
    }
    connector_slot CMT_N {
        opposite CMT_S;
        connector_class CMT_N {
            pass CMT_BUFPLL_V_CLKOUT_N = CMT_BUFPLL_V_CLKOUT_S;
            pass CMT_BUFPLL_V_LOCKED_N = CMT_BUFPLL_V_LOCKED_S;
        }
    }
}
