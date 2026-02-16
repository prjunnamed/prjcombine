use prjcombine_tablegen::target_defs;

target_defs! {
    variant virtex4;
    variant virtex5;
    variant virtex6;
    variant virtex7;

    enum SLICE_V4_CYINIT { BX, CIN }
    enum SLICE_V4_CY0F { CONST_0, CONST_1, BX, F3, F2, PROD }
    enum SLICE_V4_CY0G { CONST_0, CONST_1, BY, G3, G2, PROD }
    enum SLICE_V4_DIF_MUX { ALT, BX }
    enum SLICE_V4_DIG_MUX { ALT, BY }
    enum SLICE_V4_DXMUX { X, BX, F5, FXOR, XB }
    enum SLICE_V4_DYMUX { Y, BY, FX, GXOR, YB }
    enum SLICE_V4_FXMUX { F5, FXOR }
    enum SLICE_V4_GYMUX { FX, GXOR }
    enum SLICE_V4_XBMUX { FCY, FMC15 }
    enum SLICE_V4_YBMUX { GCY, GMC15 }
    bel_class SLICE_V4 {
        input F1, F2, F3, F4;
        input G1, G2, G3, G4;
        input BX, BY;
        input CLK, SR, CE;
        output X, Y;
        output XQ, YQ;
        output XB, YB;
        output XMUX, YMUX;

        attribute F, G: bitvec[16];

        // SLICEM only
        attribute DIF_MUX: SLICE_V4_DIF_MUX;
        attribute DIG_MUX: SLICE_V4_DIG_MUX;
        attribute F_RAM_ENABLE, G_RAM_ENABLE: bool;
        attribute F_SHIFT_ENABLE, G_SHIFT_ENABLE: bool;
        // SLICEM only
        attribute F_SLICEWE0USED, G_SLICEWE0USED: bool;
        attribute F_SLICEWE1USED, G_SLICEWE1USED: bool;

        attribute CYINIT: SLICE_V4_CYINIT;
        attribute CY0F: SLICE_V4_CY0F;
        attribute CY0G: SLICE_V4_CY0G;

        attribute FFX_INIT, FFY_INIT: bitvec[1];
        attribute FFX_SRVAL, FFY_SRVAL: bitvec[1];
        attribute FF_LATCH: bool;
        attribute FF_REV_ENABLE: bool;
        attribute FF_SR_SYNC: bool;
        // SLICEM only (effectively always enabled on SLICEL)
        attribute FF_SR_ENABLE: bool;

        attribute FXMUX: SLICE_V4_FXMUX;
        attribute GYMUX: SLICE_V4_GYMUX;
        attribute DXMUX: SLICE_V4_DXMUX;
        attribute DYMUX: SLICE_V4_DYMUX;

        // SLICEM only (effectively *CY on SLICEL)
        attribute XBMUX: SLICE_V4_XBMUX;
        attribute YBMUX: SLICE_V4_YBMUX;
    }

    enum BRAM_WRITE_MODE { WRITE_FIRST, READ_FIRST, NO_CHANGE }
    enum BRAM_WW_VALUE { NONE, _0, _1 }
    enum BRAM_V4_DATA_WIDTH { _1, _2, _4, _9, _18, _36 }
    enum BRAM_V4_FIFO_WIDTH { _4, _9, _18, _36 }
    bel_class BRAM_V4 {
        input CLKA, CLKB;
        input ENA, ENB;
        input SSRA, SSRB;
        input WEA[4], WEB[4];
        input REGCEA, REGCEB;
        input ADDRA[15], ADDRB[15];
        input DIA[32], DIB[32];
        input DIPA[4], DIPB[4];
        output DOA[32], DOB[32];
        output DOPA[4], DOPB[4];

        // FIFO mode pin assignments:
        // CLKA/ENA: RDCLK/RDEN
        // CLKB/ENB: WRCLK/WREN
        // SSRA: RST
        // DIB/DIPB: DI/DIP
        // DOA/DOPA: DO/DOP
        // DOB0-3: RDCOUNT0-3
        // DOB5: RDERR
        // DOB6: ALMOSTEMPTY
        // DOB7: EMPTY
        // DOB8: FULL
        // DOB9: ALMOSTFULL
        // DOB10: WRERR
        // DOB12-15: RDCOUNT8-11
        // DOB16-19: WRCOUNT0-3
        // DOB20-23: WRCOUNT4-7
        // DOB24-27: RDCOUNT4-7
        // DOB28-31: WRCOUNT8-11

        attribute DATA: bitvec[0x4000];
        attribute DATAP: bitvec[0x800];
        attribute SAVEDATA: bitvec[64];
        attribute INIT_A, INIT_B: bitvec[36];
        attribute SRVAL_A, SRVAL_B: bitvec[36];
        attribute READ_WIDTH_A, READ_WIDTH_B: BRAM_V4_DATA_WIDTH;
        attribute WRITE_WIDTH_A, WRITE_WIDTH_B: BRAM_V4_DATA_WIDTH;
        attribute WRITE_MODE_A, WRITE_MODE_B: BRAM_WRITE_MODE;

        attribute WW_VALUE_A, WW_VALUE_B: BRAM_WW_VALUE;

        attribute DOA_REG, DOB_REG: bool;
        attribute INVERT_CLK_DOA_REG: bool;
        attribute INVERT_CLK_DOB_REG: bool;

        attribute RAM_EXTENSION_A_LOWER: bool;
        attribute RAM_EXTENSION_B_LOWER: bool;

        attribute EN_ECC_READ: bitvec[4];
        attribute EN_ECC_WRITE: bitvec[4];

        attribute FIFO_ENABLE: bool;
        attribute FIFO_WIDTH: BRAM_V4_FIFO_WIDTH;
        attribute FIRST_WORD_FALL_THROUGH: bool;
        attribute ALMOST_EMPTY_OFFSET: bitvec[12];
        attribute ALMOST_FULL_OFFSET: bitvec[12];
    }

    bel_class BRAM_V5 {
        // TODO
    }

    bel_class PMVBRAM {
        input DISABLE0;
        input DISABLE1;
        output O;
        output ODIV2;
        output ODIV4;
    }

    enum DSP_B_INPUT { DIRECT, CASCADE }
    enum DSP_REG2 {_0, _1, _2 }
    bel_class DSP_V4 {
        input A[18];
        input B[18];
        input CARRYIN;
        input CARRYINSEL[2];
        input OPMODE[7];
        input SUBTRACT;
        input CLK;
        input CEA, CEB, CEM, CEP, CECARRYIN, CECINSUB, CECTRL;
        input RSTA, RSTB, RSTM, RSTP, RSTCARRYIN, RSTCTRL;
        output P[48];

        attribute AREG: DSP_REG2;
        attribute BREG: DSP_REG2;
        attribute MREG: bool;
        attribute PREG: bool;
        attribute OPMODEREG: bool;
        attribute SUBTRACTREG: bool;
        attribute CARRYINREG: bool;
        attribute CARRYINSELREG: bool;
        attribute B_INPUT: DSP_B_INPUT;
        attribute UNK_ENABLE: bool;
    }

    bel_class DSP_C {
        input C[48];
        input CEC;
        input RSTC;

        attribute MUX_CLK: bitvec[1];
        attribute CREG: bool;
    }

    bel_class DSP_V5 {
        // TODO
    }

    // TODO: enums, bel slots

    enum IO_DATA_RATE { SDR, DDR }
    enum IO_DATA_WIDTH { NONE, _2, _3, _4, _5, _6, _7, _8, _10 }
    enum IO_SERDES_MODE { MASTER, SLAVE }
    enum ILOGIC_MUX_TSBYPASS { GND, T }
    enum ILOGIC_INTERFACE_TYPE { MEMORY, NETWORKING }
    enum ILOGIC_DDR_CLK_EDGE { SAME_EDGE_PIPELINED, SAME_EDGE, OPPOSITE_EDGE }
    enum ILOGIC_IDELAYMUX { NONE, D, OFB }
    enum ILOGIC_IOBDELAY_TYPE { DEFAULT, FIXED, VARIABLE }
    enum ILOGIC_NUM_CE { _1, _2 }
    bel_class ILOGIC_V4 {
        input CLK, CLKDIV;
        input SR, REV;
        input CE1, CE2;
        input BITSLIP;
        input DLYCE, DLYINC, DLYRST;
        output O;
        output Q1, Q2, Q3, Q4, Q5, Q6;
        output CLKPAD;

        // ???
        attribute CLK_INV: bitvec[3];
        attribute OCLK1_INV: bool;
        attribute OCLK2_INV: bool;

        attribute FFI1_INIT: bitvec[1];
        attribute FFI2_INIT: bitvec[1];
        attribute FFI3_INIT: bitvec[1];
        attribute FFI4_INIT: bitvec[1];
        attribute FFI1_SRVAL: bitvec[1];
        attribute FFI2_SRVAL: bitvec[1];
        attribute FFI3_SRVAL: bitvec[1];
        attribute FFI4_SRVAL: bitvec[1];
        attribute FFI_ENABLE: bool;
        attribute FFI_LATCH: bool;
        // umm. shouldn't there be like. SR and REV enables, or something?
        attribute FFI_SR_SYNC: bool;

        attribute INIT_BITSLIPCNT: bitvec[4];
        attribute INIT_CE: bitvec[2];
        attribute INIT_RANK1_PARTIAL: bitvec[5];
        attribute INIT_RANK2: bitvec[6];
        attribute INIT_RANK3: bitvec[6];

        attribute I_DELAY_ENABLE: bool;
        attribute I_DELAY_DEFAULT: bool;
        attribute I_TSBYPASS_ENABLE: bool;
        attribute FFI_DELAY_ENABLE: bool;
        attribute FFI_TSBYPASS_ENABLE: bool;
        attribute MUX_TSBYPASS: ILOGIC_MUX_TSBYPASS;

        attribute SERDES: bool;
        attribute SERDES_MODE: IO_SERDES_MODE;
        attribute DATA_RATE: IO_DATA_RATE;
        attribute DATA_WIDTH: IO_DATA_WIDTH;
        attribute INTERFACE_TYPE: ILOGIC_INTERFACE_TYPE;
        attribute NUM_CE: ILOGIC_NUM_CE;
        // ???
        attribute BITSLIP_ENABLE: bitvec[7];
        attribute BITSLIP_SYNC: bool;
        attribute DDR_CLK_EDGE: ILOGIC_DDR_CLK_EDGE;

        attribute IDELAYMUX: ILOGIC_IDELAYMUX;
        attribute IOBDELAY_TYPE: ILOGIC_IOBDELAY_TYPE;
        attribute IOBDELAY_VALUE_CUR: bitvec[6];
        attribute IOBDELAY_VALUE_INIT: bitvec[6];

        attribute READBACK_I: bitvec[1];
    }

    enum OLOGIC_TRISTATE_WIDTH { _1, _2, _4 }
    enum OLOGIC_MUX_O { NONE, D1, FFO1, FFODDR }
    enum OLOGIC_MUX_T { NONE, T1, FFT1, FFTDDR }
    bel_class OLOGIC_V4 {
        input CLK, CLKDIV;
        input SR, REV;
        input OCE, TCE;
        input D1, D2, D3, D4, D5, D6;
        input T1, T2, T3, T4;
        output TQ;

        attribute CLK1_INV: bool;
        attribute CLK2_INV: bool;

        // ??? what
        attribute FFO_INIT: bitvec[4];
        attribute FFO_INIT_SERDES: bitvec[3];
        attribute FFO_SRVAL: bitvec[3];
        attribute FFO_SERDES: bitvec[4];
        attribute FFO_LATCH: bool;
        attribute FFO_SR_SYNC: bitvec[4];
        attribute FFO_SR_ENABLE: bool;
        attribute FFO_REV_ENABLE: bool;
        attribute MUX_O: OLOGIC_MUX_O;

        attribute FFT_INIT: bitvec[5];
        attribute FFT1_SRVAL: bitvec[1];
        attribute FFT2_SRVAL: bitvec[1];
        attribute FFT3_SRVAL: bitvec[1];
        attribute FFT_LATCH: bool;
        attribute FFT_SR_SYNC: bitvec[2];
        attribute FFT_SR_ENABLE: bool;
        attribute FFT_REV_ENABLE: bool;
        attribute MUX_T: OLOGIC_MUX_T;

        attribute INIT_LOADCNT: bitvec[4];

        attribute SERDES: bool;
        attribute SERDES_MODE: IO_SERDES_MODE;
        attribute DATA_WIDTH: IO_DATA_WIDTH;
        attribute TRISTATE_WIDTH: OLOGIC_TRISTATE_WIDTH;
    }

    enum IOB_PULL { NONE, PULLUP, PULLDOWN, KEEPER }
    enum IOB_IBUF_MODE { NONE, VREF, DIFF, CMOS }
    enum IOB_DCI_MODE { NONE, OUTPUT, OUTPUT_HALF, TERM_VCC, TERM_SPLIT }

    bel_class IOB_V4 {
        pad PAD: inout;

        attribute PULL: IOB_PULL;
        attribute VREF_SYSMON: bool;
        attribute VR: bool;

        attribute IBUF_MODE: IOB_IBUF_MODE;

        attribute OUTPUT_ENABLE: bitvec[2];
        attribute DCI_MODE: IOB_DCI_MODE;
        attribute DCI_MISC: bitvec[2];
        attribute DCI_T: bool;
        attribute DCIUPDATEMODE_ASREQUIRED: bool;

        attribute PDRIVE: bitvec[5];
        attribute NDRIVE: bitvec[5];
        attribute PSLEW: bitvec[4];
        attribute NSLEW: bitvec[4];
        attribute OUTPUT_MISC: bitvec[2];
        attribute LVDS: bitvec[4];
    }

    if variant virtex4 {
        table IOB_DATA {
            field PDRIVE: bitvec[5];
            field NDRIVE: bitvec[5];
            field OUTPUT_MISC: bitvec[2];
            field PSLEW_FAST: bitvec[4];
            field NSLEW_FAST: bitvec[4];
            field PSLEW_SLOW: bitvec[4];
            field NSLEW_SLOW: bitvec[4];
            field PMASK_TERM_VCC: bitvec[5];
            field PMASK_TERM_SPLIT: bitvec[5];
            field NMASK_TERM_SPLIT: bitvec[5];
            field LVDIV2: bitvec[2];

            row OFF, VREF, VR;

            // push-pull I/O standards
            row LVCMOS15_2, LVCMOS15_4, LVCMOS15_6, LVCMOS15_8, LVCMOS15_12, LVCMOS15_16;
            row LVCMOS18_2, LVCMOS18_4, LVCMOS18_6, LVCMOS18_8, LVCMOS18_12, LVCMOS18_16;
            row LVCMOS25_2, LVCMOS25_4, LVCMOS25_6, LVCMOS25_8, LVCMOS25_12, LVCMOS25_16, LVCMOS25_24;
            row LVCMOS33_2, LVCMOS33_4, LVCMOS33_6, LVCMOS33_8, LVCMOS33_12, LVCMOS33_16, LVCMOS33_24;
            row LVTTL_2, LVTTL_4, LVTTL_6, LVTTL_8, LVTTL_12, LVTTL_16, LVTTL_24;
            row PCI33_3, PCI66_3, PCIX;

            // DCI output
            row LVDCI_15, LVDCI_18, LVDCI_25, LVDCI_33;
            row LVDCI_DV2_15, LVDCI_DV2_18, LVDCI_DV2_25;
            // VREF-based with DCI output
            row HSLVDCI_15, HSLVDCI_18, HSLVDCI_25, HSLVDCI_33;

            // VREF-based
            row GTL, GTLP;
            row SSTL18_I, SSTL18_II;
            row SSTL2_I, SSTL2_II;
            row HSTL_I_12;
            row HSTL_I, HSTL_II, HSTL_III, HSTL_IV;
            row HSTL_I_18, HSTL_II_18, HSTL_III_18, HSTL_IV_18;
            // with DCI
            row GTL_DCI, GTLP_DCI;
            row SSTL18_I_DCI, SSTL18_II_DCI, SSTL18_II_T_DCI;
            row SSTL2_I_DCI, SSTL2_II_DCI, SSTL2_II_T_DCI;
            row HSTL_I_DCI, HSTL_II_DCI, HSTL_III_DCI, HSTL_II_T_DCI, HSTL_IV_DCI;
            row HSTL_I_DCI_18, HSTL_II_DCI_18, HSTL_II_T_DCI_18, HSTL_IV_DCI_18, HSTL_III_DCI_18;

            // pseudo-differential
            row BLVDS_25;
            row LVPECL_25;

            // DCI term for true differential
            row LVDS_25_DCI, LVDSEXT_25_DCI;
        }
        table LVDS_DATA {
            field OUTPUT_T: bitvec[4];
            field OUTPUT_C: bitvec[4];
            field TERM_T: bitvec[4];
            field TERM_C: bitvec[4];
            field LVDSBIAS: bitvec[10];

            row OFF;
            row LVDS_25;
            row LVDSEXT_25;
            row MINI_LVDS_25;
            row RSDS_25;
            row HT_25;
            row LVDS_25_DCI;
            row LVDSEXT_25_DCI;
        }
    }

    bel_class GLOBALSIG {
    }

    bel_class HCLK_CMT_DRP {
        attribute DRP_MASK: bool;
    }

    bel_class BUFGCTRL {
        input I0, I1;
        input S0, S1;
        input CE0, CE1;
        input IGNORE0, IGNORE1;
        output O;

        attribute CREATE_EDGE: bool;
        attribute INIT_OUT: bitvec[1];
        attribute PRESELECT_I0, PRESELECT_I1: bool;
    }

    bel_class BUFIO {
        input I;
        // virtex6 only
        input DQSMASK;
        output O;

        attribute ENABLE: bool;
        // virtex6 only
        attribute DQSMASK_ENABLE: bool;
        // virtex6 and up only
        attribute DELAY_ENABLE: bool;
    }

    enum BUFR_DIVIDE { BYPASS, _1, _2, _3, _4, _5, _6, _7, _8 }
    bel_class BUFR {
        input I, CE, CLR;
        output O;

        attribute ENABLE: bool;
        attribute DIVIDE: BUFR_DIVIDE;
    }

    enum IDELAYCTRL_RESET_STYLE { V4, V5 }
    bel_class IDELAYCTRL {
        input REFCLK, RST;
        output RDY;
        output DNPULSEOUT, UPPULSEOUT;
        output OUTN1, OUTN65;

        // set when calibrated delay used in bank (REFCLK connected)
        attribute DLL_ENABLE: bool;

        // virtex5 and up only; set when any delay used in bank, including uncalibrated
        attribute DELAY_ENABLE: bool;

        // virtex5 and up only
        // virtex5 settings:
        // - 00: no delay used
        // - 01: calibrated delay used
        // - 11: no calibrated delay used, uncalibrated delay used
        // virtex6 settings:
        // - 00: no delay used or calibrated delay used
        // - 10 uncalibrated delay used
        // virtex7 settings: always 00
        attribute VCTL_SEL: bitvec[2];

        // virtex6 only
        attribute RESET_STYLE: IDELAYCTRL_RESET_STYLE;

        // virtex6 and up only
        attribute HIGH_PERFORMANCE_MODE: bool;
        // for calibrated delay only; the BIAS_MODE setting of "2" is stored the same as "0" here.
        attribute BIAS_MODE: bitvec[0];
    }

    // used for virtex4 and virtex5
    bel_class DCI_V4 {
        input TSTCLK, TSTRST;
        input TSTHLP, TSTHLN;
        output DCISCLK;
        output DCIADDRESS[3];
        output DCIDATA;
        output DCIIOUPDATE;
        output DCIREFIOUPDATE;
        output DCIDONE;

        attribute ENABLE: bool;
        attribute QUIET: bool;
        attribute V4_LVDIV2: bitvec[2];
        attribute V5_LVDIV2: bitvec[3];
        attribute PMASK_TERM_VCC: bitvec[5];
        attribute PMASK_TERM_SPLIT: bitvec[5];
        attribute NMASK_TERM_SPLIT: bitvec[5];
        attribute NREF: bitvec[2];
        attribute PREF: bitvec[4];
        attribute TEST_ENABLE: bitvec[2];
        attribute CASCADE_FROM_ABOVE: bool;
        attribute CASCADE_FROM_BELOW: bool;
    }

    bel_class LVDS_V4 {
        attribute LVDSBIAS: bitvec[10];
    }

    enum INTERNAL_VREF {
        OFF,
        // virtex6 and up only
        _600,
        // virtex7 only
        _675,
        _750,
        _900,
        // virtex5 only
        _1080,
        // virtex6 and up only
        _1100,
        _1250,
    }
    bel_class BANK {
        // virtex5 and up; virtex4 has this on a separate bel
        attribute V5_LVDSVIAS: bitvec[12];
        // virtex5 and up
        attribute INTERNAL_VREF: INTERNAL_VREF;
    }

    enum DCM_CLKDV_MODE { HALF, INT }
    enum DCM_CLK_FEEDBACK { _1X, _2X, NONE }
    enum DCM_PS_MODE { CLKIN, CLKFB }
    enum DCM_PERFORMANCE_MODE { MAX_RANGE, MAX_SPEED }
    enum DCM_VREF_SOURCE { VDD_VBG, BGM_SNAP, BGM_ABS_SNAP, BGM_ABS_REF }
    enum DCM_DLL_CONTROL_CLOCK_SPEED { HALF, QUARTER }
    enum DCM_DLL_FREQUENCY_MODE { LOW, HIGH_SER, HIGH }
    enum DCM_DLL_PHASE_DETECTOR_MODE { LEVEL, ENHANCED }
    enum DCM_DLL_PHASE_SHIFT_CALIBRATION { AUTO_DPS, CONFIG, MASK, AUTO_ZD2 }
    enum DCM_DFS_AVE_FREQ_GAIN { NONE, _0P5, _0P25, _0P125, _1P0, _2P0, _4P0, _8P0 }
    enum DCM_DFS_SEL { LEVEL, LEGACY }
    enum DCM_DFS_FREQUENCY_MODE { LOW, HIGH }
    enum DCM_DFS_OSCILLATOR_MODE { PHASE_FREQ_LOCK, FREQ_LOCK, AVE_FREQ_LOCK }
    enum DCM_BGM_CONFIG_REF_SEL { DCLK, CLKIN }
    enum DCM_BGM_MODE { BG_SNAPSHOT, ABS_FREQ_SNAPSHOT, ABS_FREQ_REF }
    bel_class DCM_V4 {
        input CLKIN, CLKFB;
        output CLK0, CLK90, CLK180, CLK270;
        output CLK2X, CLK2X180;
        output CLKDV;
        output CLKFX, CLKFX180, CONCUR;

        input RST;
        output LOCKED;

        input PSCLK, PSEN, PSINCDEC;
        output PSDONE;

        input DCLK;
        input DEN;
        input DWE;
        input DADDR[7];
        input DI[16];
        output DRDY;
        output DO[16];

        input FREEZE_DLL, FREEZE_DFS;
        input CTLMODE, CTLGO, CTLOSC1, CTLOSC2, CTLSEL[3];

        // address 0x40 and up
        attribute DRP: bitvec[16][32];
        attribute DRP_MASK: bitvec[32];

        attribute OUT_CLK0_ENABLE: bool;
        attribute OUT_CLK90_ENABLE: bool;
        attribute OUT_CLK180_ENABLE: bool;
        attribute OUT_CLK270_ENABLE: bool;
        attribute OUT_CLK2X_ENABLE: bool;
        attribute OUT_CLK2X180_ENABLE: bool;
        attribute OUT_CLKDV_ENABLE: bool;
        attribute OUT_CLKFX_ENABLE: bool;
        attribute OUT_CLKFX180_ENABLE: bool;
        attribute OUT_CONCUR_ENABLE: bool;

        attribute CLKDV_COUNT_MAX: bitvec[4];
        attribute CLKDV_COUNT_FALL: bitvec[4];
        attribute CLKDV_COUNT_FALL_2: bitvec[4];
        attribute CLKDV_PHASE_RISE: bitvec[2];
        attribute CLKDV_PHASE_FALL: bitvec[2];
        attribute CLKDV_MODE: DCM_CLKDV_MODE;

        attribute STARTUP_WAIT: bool;
        attribute UNK_ALWAYS_SET: bool;
        attribute DESKEW_ADJUST: bitvec[5];
        attribute CLKIN_ENABLE: bool;
        attribute CLKIN_IOB: bool;
        attribute CLKFB_ENABLE: bool;
        attribute CLKFB_IOB: bool;
        attribute CLKFB_FEEDBACK: bool;
        attribute CLKIN_DIVIDE_BY_2: bool;
        attribute CLK_FEEDBACK: DCM_CLK_FEEDBACK;

        attribute CLKFX_MULTIPLY: bitvec[5];
        attribute CLKFX_DIVIDE: bitvec[5];

        attribute DUTY_CYCLE_CORRECTION: bitvec[4];
        attribute FACTORY_JF: bitvec[16];
        attribute PHASE_SHIFT: bitvec[10];
        attribute PHASE_SHIFT_NEGATIVE: bool;
        attribute PMCD_SYNC: bool;
        attribute PS_CENTERED: bool;
        attribute PS_DIRECT: bool;
        attribute PS_ENABLE: bool;
        attribute PS_MODE: DCM_PS_MODE;

        attribute DCM_CLKDV_CLKFX_ALIGNMENT: bool;
        attribute DCM_EXT_FB_EN: bool;
        attribute DCM_LOCK_HIGH: bool;
        attribute DCM_PERFORMANCE_MODE: DCM_PERFORMANCE_MODE;
        attribute DCM_PULSE_WIDTH_CORRECTION_LOW: bitvec[5];
        attribute DCM_PULSE_WIDTH_CORRECTION_HIGH: bitvec[5];
        attribute DCM_UNUSED_TAPS_POWERDOWN: bool;

        attribute DCM_VREG_ENABLE: bool;
        attribute DCM_VBG_PD: bitvec[2];
        attribute DCM_VBG_SEL: bitvec[4];
        attribute DCM_VREF_SOURCE: DCM_VREF_SOURCE;
        attribute DCM_VREG_PHASE_MARGIN: bitvec[3];

        attribute DLL_CONTROL_CLOCK_SPEED: DCM_DLL_CONTROL_CLOCK_SPEED;
        attribute DLL_CTL_SEL_CLKIN_DIV2: bool;
        attribute DLL_DEAD_TIME: bitvec[8];
        attribute DLL_DESKEW_LOCK_BY1: bool;
        attribute DLL_DESKEW_MAXTAP: bitvec[8];
        attribute DLL_DESKEW_MINTAP: bitvec[8];
        attribute DLL_FREQUENCY_MODE: DCM_DLL_FREQUENCY_MODE;
        attribute DLL_LIVE_TIME: bitvec[8];
        attribute DLL_PD_DLY_SEL: bitvec[3];
        attribute DLL_PERIOD_LOCK_BY1: bool;
        attribute DLL_PHASE_DETECTOR_AUTO_RESET: bool;
        attribute DLL_PHASE_DETECTOR_MODE: DCM_DLL_PHASE_DETECTOR_MODE;
        attribute DLL_PHASE_SHIFT_CALIBRATION: DCM_DLL_PHASE_SHIFT_CALIBRATION;
        attribute DLL_PHASE_SHIFT_HFC: bitvec[8];
        attribute DLL_PHASE_SHIFT_LFC: bitvec[8];
        attribute DLL_PHASE_SHIFT_LOCK_BY1: bool;
        attribute DLL_SETTLE_TIME: bitvec[8];
        attribute DLL_TEST_MUX_SEL: bitvec[2];
        attribute DLL_ZD2_EN: bool;
        attribute DLL_SPARE: bitvec[16];

        attribute DFS_AVE_FREQ_ADJ_INTERVAL: bitvec[4];
        attribute DFS_AVE_FREQ_GAIN: DCM_DFS_AVE_FREQ_GAIN;
        attribute DFS_AVE_FREQ_SAMPLE_INTERVAL: bitvec[3];
        attribute DFS_COARSE_SEL: DCM_DFS_SEL;
        attribute DFS_COIN_WINDOW: bitvec[2];
        attribute DFS_EARLY_LOCK: bool;
        attribute DFS_ENABLE: bool;
        attribute DFS_EN_RELRST: bool;
        attribute DFS_EXTEND_FLUSH_TIME: bool;
        attribute DFS_EXTEND_HALT_TIME: bool;
        attribute DFS_EXTEND_RUN_TIME: bool;
        attribute DFS_FEEDBACK: bool;
        attribute DFS_FINE_SEL: DCM_DFS_SEL;
        attribute DFS_FREQUENCY_MODE: DCM_DFS_FREQUENCY_MODE;
        attribute DFS_HARDSYNC: bitvec[2];
        attribute DFS_NON_STOP: bool;
        attribute DFS_OSCILLATOR_MODE: DCM_DFS_OSCILLATOR_MODE;
        attribute DFS_SKIP_FINE: bool;
        attribute DFS_SPARE: bitvec[16];
        attribute DFS_TP_SEL: DCM_DFS_SEL;
        attribute DFS_TRACKMODE: bool;

        attribute BGM_CONFIG_REF_SEL: DCM_BGM_CONFIG_REF_SEL;
        attribute BGM_LDLY: bitvec[3];
        attribute BGM_MODE: DCM_BGM_MODE;
        attribute BGM_MULTIPLY: bitvec[6];
        attribute BGM_DIVIDE: bitvec[6];
        attribute BGM_SAMPLE_LEN: bitvec[3];
        attribute BGM_SDLY: bitvec[3];
        attribute BGM_VADJ: bitvec[4];
        attribute BGM_VLDLY: bitvec[3];
        attribute BGM_VSDLY: bitvec[3];
    }

    enum PMCD_RST_DEASSERT_CLK { CLKA, CLKB, CLKC, CLKD }
    bel_class PMCD {
        input CLKA, CLKB, CLKC, CLKD;
        input REL;
        input RST;
        output CLKA1, CLKA1D2, CLKA1D4, CLKA1D8;
        output CLKB1, CLKC1, CLKD1;

        attribute CLKA_ENABLE: bitvec[4]; // TODO: actually per-output?
        attribute CLKB_ENABLE: bool;
        attribute CLKC_ENABLE: bool;
        attribute CLKD_ENABLE: bool;
        attribute EN_REL: bool;
        attribute RST_DEASSERT_CLK: PMCD_RST_DEASSERT_CLK;
    }

    bel_class DPM {
        input REFCLK;
        input TESTCLK1, TESTCLK2;
        input RST;
        input SELSKEW;
        input ENOSC[3];
        input FREEZE;
        input HFSEL[3];
        input OUTSEL[3];
        output REFCLKOUT;
        output OSCOUT1, OSCOUT2;
        output CENTER;
        output DOUT[8];
        output VALID;
    }

    bel_class CCM {
        attribute VREG_ENABLE: bool;
        attribute VBG_SEL: bitvec[4];
        attribute VBG_PD: bitvec[2];
        attribute VREG_PHASE_MARGIN: bitvec[3];
    }

    enum SYSMON_MONITOR_MODE { MONITOR, ADC, TEST }
    bel_class SYSMON_V4 {
        input CONVST;
        input RST;
        output ALARM[7];
        output BUSY;
        output CHANNEL[5];
        output DB[12];
        output EOC;
        output EOS;
        output OT;

        input DCLK;
        input DEN;
        input DWE;
        input DADDR[7];
        input DI[16];
        output DRDY;
        output DO[16];

        input ROMTESTENABLE;
        input ROMTESTADDR[16];
        output ROMTESTDATA[16];
        input SCANMEMCLK;
        input SCANMEMWE;
        input SCANTESTENA;
        input SCANTESTENB;
        input SCLKA;
        input SCLKB;
        input SEA;
        input SEB;
        input SDIA;
        input SDIB;
        output SDOA;
        output SDOB;

        // address 0x40 and up
        attribute INIT: bitvec[16][48];
        attribute MONITOR_MODE: SYSMON_MONITOR_MODE;
        attribute BLOCK_ENABLE: bitvec[5];
        attribute DCLK_DIVID_2: bitvec[1];
        attribute LW_DIVID_2_4: bitvec[1];
        attribute DCLK_MISSING: bitvec[10];
        attribute FEATURE_ENABLE: bitvec[8];
        attribute MCCLK_DIVID: bitvec[8];
        attribute OVER_TEMPERATURE: bitvec[10];
        attribute OVER_TEMPERATURE_DELAY: bitvec[8];
        attribute OVER_TEMPERATURE_OFF: bitvec[1];
        attribute PROM_DATA: bitvec[8];
    }

    bel_class SYSMON_V5 {
        input CONVST;
        input CONVSTCLK;
        input RESET;
        output ALM[3];
        output BUSY;
        output CHANNEL[5];
        output EOC;
        output EOS;
        output OT;

        input DCLK;
        input DEN;
        input DWE;
        input DADDR[7];
        input DI[16];
        output DRDY;
        output DO[16];

        output JTAGBUSY;
        output JTAGLOCKED;
        output JTAGMODIFIED;

        input TESTADCCLK[4];
        input TESTADCIN[20];
        output TESTADCOUT[20];
        output TESTDB[16];

        input TESTSCANCLKA;
        input TESTSCANCLKB;
        input TESTSCANCLKC;
        input TESTSCANCLKD;
        input TESTSCANCLKE;
        input TESTSCANMODEA;
        input TESTSCANMODEB;
        input TESTSCANMODEC;
        input TESTSCANMODED;
        input TESTSCANMODEE;
        input TESTSCANRESET;
        input TESTSEA;
        input TESTSEB;
        input TESTSEC;
        input TESTSED;
        input TESTSEE;
        input TESTSEL;
        input TESTSIA;
        input TESTSIB;
        input TESTSIC;
        input TESTSID;
        input TESTSIE;
        output TESTSOA;
        output TESTSOB;
        output TESTSOC;
        output TESTSOD;
        output TESTSOE;

        input TESTENJTAG;
        input TESTDRCK;
        input TESTRST;
        input TESTSHIFT;
        input TESTUPDATE;
        input TESTCAPTURE;
        input TESTTDI;
        output TESTTDO;

        // address 0x40..0x58
        attribute INIT: bitvec[16][24];

        attribute SYSMON_TEST_A: bitvec[16];
        attribute SYSMON_TEST_B: bitvec[16];
        attribute SYSMON_TEST_C: bitvec[16];
        attribute SYSMON_TEST_D: bitvec[16];
        attribute SYSMON_TEST_E: bitvec[16];
    }

    bel_class STARTUP {
        input CLK;
        input GTS, GSR;
        input USRCCLKO, USRCCLKTS;
        input USRDONEO, USRDONETS;
        output EOS;

        // virtex5 only
        output CFGCLK, CFGMCLK, DINSPI, TCKSPI;

        attribute USER_GTS_GSR_ENABLE: bool;
        attribute GTS_SYNC: bool;
        attribute GSR_SYNC: bool;
        // virtex4 only
        attribute GWE_SYNC: bool;
        attribute USRCCLK_ENABLE: bool;
    }

    bel_class CAPTURE {
        input CLK;
        input CAP;
    }

    bel_class ICAP {
        input CLK;
        input CE;
        input WRITE;
        input I[32];
        output BUSY;
        output O[32];

        attribute ENABLE: bool;
    }

    bel_class BSCAN {
        input TDO;
        output DRCK;
        output SEL;
        output TDI;
        output RESET, CAPTURE, SHIFT, UPDATE;

        attribute ENABLE: bool;
    }

    enum JTAGPPC_NUM_PPC { _0, _1, _2, _3, _4 }
    bel_class JTAGPPC {
        input TDOPPC;
        output TCK;
        output TMS;
        output TDIPPC;

        // virtex4 only
        attribute ENABLE: bool;

        // virtex5 only
        attribute NUM_PPC: JTAGPPC_NUM_PPC;
    }

    bel_class PMV {
        input EN;
        input A[6];
        output O, ODIV2, ODIV4;
    }

    bel_class DCIRESET {
        input RST;
        output LOCKED;

        attribute ENABLE: bool;
    }

    bel_class FRAME_ECC {
        output ERROR;
        output SYNDROMEVALID;
        output SYNDROME[12];

        // virtex5 and up only
        output CRCERROR, ECCERROR;
    }

    bel_class USR_ACCESS {
        output DATAVALID;
        output DATA[32];

        // virtex5 and up only
        output CFGCLK;
    }

    bel_class KEY_CLEAR {
        input KEYCLEARB;
    }

    bel_class EFUSE_USR {
        output EFUSEUSR[32];
    }

    // X16 only supported on virtex5 and up
    enum ICAP_WIDTH { X8, X16, X32 }
    enum PROBESEL { NONE, _0, _1, _2, _3 }
    bel_class MISC_CFG {
        pad HSWAPEN: input;
        pad PROG_B: input;
        // virtex4 only
        pad POWERDOWN_B: input;
        pad DONE: inout;
        pad M0, M1, M2: input;
        pad CCLK: inout;
        pad INIT_B: inout;
        pad DIN: input;
        pad CS_B: input;
        pad RDWR_B: input;
        pad BUSY: output;
        pad TCK, TMS, TDI: input;
        pad TDO: output;

        attribute USERCODE: bitvec[32];
        attribute ICAP_WIDTH: ICAP_WIDTH;
        attribute DCI_CLK_ENABLE: bitvec[2];
        attribute PROBESEL: PROBESEL;

        attribute HSWAPEN_PULL: IOB_PULL;
        attribute PROG_PULL: IOB_PULL;
        attribute POWERDOWN_PULL: IOB_PULL;
        attribute DONE_PULL: IOB_PULL;
        attribute M0_PULL: IOB_PULL;
        attribute M1_PULL: IOB_PULL;
        attribute M2_PULL: IOB_PULL;
        attribute CCLK_PULL: IOB_PULL;
        attribute INIT_PULL: IOB_PULL;
        attribute DIN_PULL: IOB_PULL;
        attribute CS_PULL: IOB_PULL;
        attribute RDWR_PULL: IOB_PULL;
        attribute BUSY_PULL: IOB_PULL;
        attribute TCK_PULL: IOB_PULL;
        attribute TMS_PULL: IOB_PULL;
        attribute TDI_PULL: IOB_PULL;
        attribute TDO_PULL: IOB_PULL;
    }

    enum STARTUP_CYCLE { _0, _1, _2, _3, _4, _5, _6, DONE, KEEP, NOWAIT }
    enum STARTUP_CLOCK { CCLK, USERCLK, JTAGCLK }
    enum CONFIG_RATE_V4 { _4, _5, _7, _8, _9, _10, _13, _15, _20, _26, _30, _34, _41, _51, _55, _60, _130 }
    enum CONFIG_RATE_V5 { _2, _6, _9, _13, _17, _20, _24, _27, _31, _35, _38, _42, _46, _49, _53, _56, _60 }
    enum SECURITY { NONE, LEVEL1, LEVEL2 }
    enum ICAP_SELECT { BOTTOM, TOP }
    enum BPI_PAGE_SIZE { _1, _4, _8 }
    enum BPI_1ST_READ_CYCLE { _1, _2, _3, _4 }
    enum ENCRYPT_KEY_SELECT { BBRAM, EFUSE }
    bel_class GLOBAL {
        // COR
        attribute GWE_CYCLE: STARTUP_CYCLE;
        attribute GTS_CYCLE: STARTUP_CYCLE;
        attribute LOCK_CYCLE: STARTUP_CYCLE;
        attribute MATCH_CYCLE: STARTUP_CYCLE;
        attribute DONE_CYCLE: STARTUP_CYCLE;
        attribute STARTUP_CLOCK: STARTUP_CLOCK;
        attribute CONFIG_RATE_V4: CONFIG_RATE_V4;
        attribute CONFIG_RATE_V5: CONFIG_RATE_V5;
        attribute CAPTURE_ONESHOT: bool;
        attribute DRIVE_DONE: bool;
        attribute DONE_PIPE: bool;
        attribute DCM_SHUTDOWN: bool;
        attribute POWERDOWN_STATUS: bool;
        attribute CRC_ENABLE: bool;

        // COR1 (virtex5 and up only)
        attribute BPI_PAGE_SIZE: BPI_PAGE_SIZE;
        attribute BPI_1ST_READ_CYCLE: BPI_1ST_READ_CYCLE;
        attribute POST_CRC_EN: bool;
        attribute POST_CRC_NO_PIN: bool;
        attribute POST_CRC_RECONFIG: bool;
        attribute RETAIN_CONFIG_STATUS: bool;
        attribute POST_CRC_SEL: bitvec[1];
        attribute PERSIST_DEASSERT_AT_DESYNCH: bool;

        // CTL
        attribute GTS_USR_B: bool;
        // virtex4 only
        attribute EN_VTEST: bool;
        // virtex4 only
        attribute VGG_TEST: bool;
        attribute PERSIST: bool;
        attribute SECURITY: SECURITY;
        attribute ENCRYPT: bool;
        attribute GLUTMASK: bool;
        attribute ICAP_SELECT: ICAP_SELECT;
        // the following are virtex5 and up only
        attribute CONFIG_FALLBACK: bool;
        attribute ENCRYPT_KEY_SELECT: ENCRYPT_KEY_SELECT;
        attribute OVERTEMP_POWERDOWN: bool;
        attribute SELECTMAP_ABORT: bool;
        attribute VGG_SEL: bitvec[5];
        attribute VBG_DLL_SEL: bitvec[5];
        attribute VBG_SEL: bitvec[5];

        // TIMER (virtex5 and up only)
        attribute TIMER: bitvec[24];
        attribute TIMER_CFG: bool;
        attribute TIMER_USR: bool;

        // WBSTAR (virtex5 and up only)
        attribute V5_NEXT_CONFIG_ADDR: bitvec[26];
        attribute V7_NEXT_CONFIG_ADDR: bitvec[29];
        attribute REVISION_SELECT_TRISTATE: bool;
        attribute REVISION_SELECT: bitvec[2];

        // TESTMODE (virtex5 and up only)
        attribute DD_OVERRIDE: bool;

    }

    bel_class PPC405 {
        input CPMC405CLOCK;
        input CPMC405CLOCKFBENABLE;
        input CPMC405CORECLKINACTIVE;
        input CPMC405CPUCLKEN;
        input CPMC405JTAGCLKEN;
        input CPMC405PLBSAMPLECYCLE;
        input CPMC405PLBSAMPLECYCLEALT;
        input CPMC405PLBSYNCCLOCK;
        input CPMC405SYNCBYPASS;
        input CPMC405TIMERCLKEN;
        input CPMC405TIMERTICK;
        input CPMDCRCLK;
        input CPMFCMCLK;
        output C405CPMCLOCKFB;
        output C405CPMCORESLEEPREQ;
        output C405CPMMSRCE;
        output C405CPMMSREE;
        output C405CPMTIMERIRQ;
        output C405CPMTIMERRESETREQ;

        input RSTC405RESETCHIP;
        input RSTC405RESETCORE;
        input RSTC405RESETSYS;
        input MCBCPUCLKEN;
        input MCBJTAGEN;
        input MCBTIMEREN;
        input MCPPCRST;
        output C405RSTCHIPRESETREQ;
        output C405RSTCORERESETREQ;
        output C405RSTSYSRESETREQ;

        input PLBCLK;

        input PLBC405DCUADDRACK;
        input PLBC405DCUBUSY;
        input PLBC405DCUERR;
        input PLBC405DCURDDACK;
        input PLBC405DCURDDBUS[0:63];
        input PLBC405DCURDWDADDR[1:3];
        input PLBC405DCUSSIZE1;
        input PLBC405DCUWRDACK;
        output C405PLBDCUABORT;
        output C405PLBDCUABUS[0:31];
        output C405PLBDCUBE[0:7];
        output C405PLBDCUCACHEABLE;
        output C405PLBDCUGUARDED;
        output C405PLBDCUPRIORITY[0:1];
        output C405PLBDCUREQUEST;
        output C405PLBDCURNW;
        output C405PLBDCUSIZE2;
        output C405PLBDCUU0ATTR;
        output C405PLBDCUWRDBUS[0:63];
        output C405PLBDCUWRITETHRU;

        input PLBC405ICUADDRACK;
        input PLBC405ICUBUSY;
        input PLBC405ICUERR;
        input PLBC405ICURDDACK;
        input PLBC405ICURDDBUS[0:63];
        input PLBC405ICURDWDADDR[1:3];
        input PLBC405ICUSSIZE1;
        output C405PLBICUABORT;
        output C405PLBICUABUS[0:29];
        output C405PLBICUCACHEABLE;
        output C405PLBICUPRIORITY[0:1];
        output C405PLBICUREQUEST;
        output C405PLBICUSIZE[2:3];
        output C405PLBICUU0ATTR;

        output DCREMACENABLER;
        input EXTDCRACK;
        input EXTDCRDBUSIN[0:31];
        output EXTDCRABUS[0:9];
        output EXTDCRDBUSOUT[0:31];
        output EXTDCRREAD;
        output EXTDCRWRITE;
        input TIEDCRADDR[0:5];
        input TSTC405DCRABUSI[0:9];
        input TSTC405DCRDBUSOUTI[0:31];
        input TSTC405DCRREADI;
        input TSTC405DCRWRITEI;
        output TSTDCRC405ACKO;
        output TSTDCRC405DBUSINO[0:31];

        input EICC405CRITINPUTIRQ;
        input EICC405EXTINPUTIRQ;

        input DBGC405DEBUGHALT;
        input DBGC405EXTBUSHOLDACK;
        input DBGC405UNCONDDEBUGEVENT;
        output C405DBGLOADDATAONAPUDBUS;
        output C405DBGMSRWE;
        output C405DBGSTOPACK;
        output C405DBGWBCOMPLETE;
        output C405DBGWBFULL;
        output C405DBGWBIAR[0:29];

        input JTGC405BNDSCANTDO;
        input JTGC405TCK;
        input JTGC405TDI;
        input JTGC405TMS;
        input JTGC405TRSTNEG;
        output C405JTGCAPTUREDR;
        output C405JTGEXTEST;
        output C405JTGPGMOUT;
        output C405JTGSHIFTDR;
        output C405JTGTDO;
        output C405JTGTDOEN;
        output C405JTGUPDATEDR;

        input TRCC405TRACEDISABLE;
        input TRCC405TRIGGEREVENTIN;
        output C405TRCCYCLE;
        output C405TRCEVENEXECUTIONSTATUS[0:1];
        output C405TRCODDEXECUTIONSTATUS[0:1];
        output C405TRCTRACESTATUS[0:3];
        output C405TRCTRIGGEREVENTOUT;
        output C405TRCTRIGGEREVENTTYPE[0:10];

        output C405XXXMACHINECHECK;

        input BRAMDSOCMCLK;
        input BRAMDSOCMRDDBUS[0:31];
        input DSARCVALUE[0:7];
        input DSCNTLVALUE[0:7];
        output DSOCMBRAMABUS[8:29];
        output DSOCMBRAMBYTEWRITE[0:3];
        output DSOCMBRAMEN;
        output DSOCMBRAMWRDBUS[0:31];
        output DSOCMBUSY;
        output DSOCMRDADDRVALID;
        input DSOCMRWCOMPLETE;
        output DSOCMWRADDRVALID;
        output C405DSOCMCACHEABLE;
        output C405DSOCMGUARDED;
        output C405DSOCMSTRINGMULTIPLE;
        output C405DSOCMU0ATTR;
        input TSTC405DSOCMABORTOPI;
        output TSTC405DSOCMABORTOPO;
        input TSTC405DSOCMABORTREQI;
        output TSTC405DSOCMABORTREQO;
        input TSTC405DSOCMABUSI[0:29];
        output TSTC405DSOCMABUSO[0:29];
        input TSTC405DSOCMBYTEENI[0:3];
        output TSTC405DSOCMBYTEENO[0:3];
        input TSTC405DSOCMLOADREQI;
        output TSTC405DSOCMLOADREQO;
        input TSTC405DSOCMSTOREREQI;
        output TSTC405DSOCMSTOREREQO;
        input TSTC405DSOCMWAITI;
        output TSTC405DSOCMWAITO;
        input TSTC405DSOCMWRDBUSI[0:31];
        output TSTC405DSOCMWRDBUSO[0:31];
        input TSTC405DSOCMXLTVALIDI;
        output TSTC405DSOCMXLTVALIDO;
        input TSTDSOCMC405COMPLETEI;
        output TSTDSOCMC405COMPLETEO;
        input TSTDSOCMC405DISOPERANDFWDI;
        output TSTDSOCMC405DISOPERANDFWDO;
        input TSTDSOCMC405HOLDI;
        output TSTDSOCMC405HOLDO;
        input TSTDSOCMC405RDDBUSI[0:31];
        output TSTDSOCMC405RDDBUSO[0:31];

        input BRAMISOCMCLK;
        input BRAMISOCMDCRRDDBUS[0:31];
        input BRAMISOCMRDDBUS[0:63];
        input ISARCVALUE[0:7];
        input ISCNTLVALUE[0:7];
        output ISOCMBRAMEN;
        output ISOCMBRAMEVENWRITEEN;
        output ISOCMBRAMODDWRITEEN;
        output ISOCMBRAMRDABUS[8:28];
        output ISOCMBRAMWRABUS[8:28];
        output ISOCMBRAMWRDBUS[0:31];
        output ISOCMDCRBRAMEVENEN;
        output ISOCMDCRBRAMODDEN;
        output ISOCMDCRBRAMRDSELECT;
        output C405ISOCMCACHEABLE;
        output C405ISOCMCONTEXTSYNC;
        output C405ISOCMU0ATTR;
        input TSTC405ISOCMABORTI;
        output TSTC405ISOCMABORTO;
        input TSTC405ISOCMABUSI[0:29];
        output TSTC405ISOCMABUSO[0:29];
        input TSTC405ISOCMICUREADYI;
        output TSTC405ISOCMICUREADYO;
        input TSTC405ISOCMREQPENDINGI;
        output TSTC405ISOCMREQPENDINGO;
        input TSTC405ISOCMXLTVALIDI;
        output TSTC405ISOCMXLTVALIDO;
        input TSTISOCMC405HOLDI;
        output TSTISOCMC405HOLDO;
        input TSTISOCMC405RDDVALIDI[0:1];
        output TSTISOCMC405RDDVALIDO[0:1];
        input TSTISOCMC405READDATAOUTI[0:63];
        output TSTISOCMC405READDATAOUTO[0:63];

        input FCMAPUCR[0:3];
        input FCMAPUDCDCREN;
        input FCMAPUDCDFORCEALIGN;
        input FCMAPUDCDFORCEBESTEERING;
        input FCMAPUDCDFPUOP;
        input FCMAPUDCDGPRWRITE;
        input FCMAPUDCDLDSTBYTE;
        input FCMAPUDCDLDSTDW;
        input FCMAPUDCDLDSTHW;
        input FCMAPUDCDLDSTQW;
        input FCMAPUDCDLDSTWD;
        input FCMAPUDCDLOAD;
        input FCMAPUDCDPRIVOP;
        input FCMAPUDCDRAEN;
        input FCMAPUDCDRBEN;
        input FCMAPUDCDSTORE;
        input FCMAPUDCDTRAPBE;
        input FCMAPUDCDTRAPLE;
        input FCMAPUDCDUPDATE;
        input FCMAPUDCDXERCAEN;
        input FCMAPUDCDXEROVEN;
        input FCMAPUDECODEBUSY;
        input FCMAPUDONE;
        input FCMAPUEXCEPTION;
        input FCMAPUEXEBLOCKINGMCO;
        input FCMAPUEXECRFIELD[0:2];
        input FCMAPUEXENONBLOCKINGMCO;
        input FCMAPUINSTRACK;
        input FCMAPULOADWAIT;
        input FCMAPURESULT[0:31];
        input FCMAPURESULTVALID;
        input FCMAPUSLEEPNOTREADY;
        input FCMAPUXERCA;
        input FCMAPUXEROV;
        output APUFCMDECODED;
        output APUFCMDECUDI[0:2];
        output APUFCMDECUDIVALID;
        output APUFCMENDIAN;
        output APUFCMFLUSH;
        output APUFCMINSTRUCTION[0:31];
        output APUFCMINSTRVALID;
        output APUFCMLOADBYTEEN[0:3];
        output APUFCMLOADDATA[0:31];
        output APUFCMLOADDVALID;
        output APUFCMOPERANDVALID;
        output APUFCMRADATA[0:31];
        output APUFCMRBDATA[0:31];
        output APUFCMWRITEBACKOK;
        output APUFCMXERCA;
        input TIEAPUCONTROL[0:15];
        input TIEAPUUDI1[0:23];
        input TIEAPUUDI2[0:23];
        input TIEAPUUDI3[0:23];
        input TIEAPUUDI4[0:23];
        input TIEAPUUDI5[0:23];
        input TIEAPUUDI6[0:23];
        input TIEAPUUDI7[0:23];
        input TIEAPUUDI8[0:23];
        input TSTAPUC405APUDIVENI;
        output TSTAPUC405APUDIVENO;
        input TSTAPUC405APUPRESENTI;
        output TSTAPUC405APUPRESENTO;
        input TSTAPUC405DCDAPUOPI;
        output TSTAPUC405DCDAPUOPO;
        input TSTAPUC405DCDCRENI;
        output TSTAPUC405DCDCRENO;
        input TSTAPUC405DCDFORCEALIGNI;
        output TSTAPUC405DCDFORCEALIGNO;
        input TSTAPUC405DCDFORCEBESTEERINGI;
        output TSTAPUC405DCDFORCEBESTEERINGO;
        input TSTAPUC405DCDFPUOPI;
        output TSTAPUC405DCDFPUOPO;
        input TSTAPUC405DCDGPRWRITEI;
        output TSTAPUC405DCDGPRWRITEO;
        input TSTAPUC405DCDLDSTBYTEI;
        output TSTAPUC405DCDLDSTBYTEO;
        input TSTAPUC405DCDLDSTDWI;
        output TSTAPUC405DCDLDSTDWO;
        input TSTAPUC405DCDLDSTHWI;
        output TSTAPUC405DCDLDSTHWO;
        input TSTAPUC405DCDLDSTQWI;
        output TSTAPUC405DCDLDSTQWO;
        input TSTAPUC405DCDLDSTWDI;
        output TSTAPUC405DCDLDSTWDO;
        input TSTAPUC405DCDLOADI;
        output TSTAPUC405DCDLOADO;
        input TSTAPUC405DCDPRIVOPI;
        output TSTAPUC405DCDPRIVOPO;
        input TSTAPUC405DCDRAENI;
        output TSTAPUC405DCDRAENO;
        input TSTAPUC405DCDRBENI;
        output TSTAPUC405DCDRBENO;
        input TSTAPUC405DCDSTOREI;
        output TSTAPUC405DCDSTOREO;
        input TSTAPUC405DCDTRAPBEI;
        output TSTAPUC405DCDTRAPBEO;
        input TSTAPUC405DCDTRAPLEI;
        output TSTAPUC405DCDTRAPLEO;
        input TSTAPUC405DCDUPDATEI;
        output TSTAPUC405DCDUPDATEO;
        input TSTAPUC405DCDVALIDOPI;
        output TSTAPUC405DCDVALIDOPO;
        input TSTAPUC405DCDXERCAENI;
        output TSTAPUC405DCDXERCAENO;
        input TSTAPUC405DCDXEROVENI;
        output TSTAPUC405DCDXEROVENO;
        input TSTAPUC405EXCEPTIONI;
        output TSTAPUC405EXCEPTIONO;
        input TSTAPUC405EXEBLOCKINGMCOI;
        output TSTAPUC405EXEBLOCKINGMCOO;
        input TSTAPUC405EXEBUSYI;
        output TSTAPUC405EXEBUSYO;
        input TSTAPUC405EXECRFIELDI[0:2];
        output TSTAPUC405EXECRFIELDO[0:2];
        input TSTAPUC405EXECRI[0:3];
        output TSTAPUC405EXECRO[0:3];
        input TSTAPUC405EXELDDEPENDI;
        output TSTAPUC405EXELDDEPENDO;
        input TSTAPUC405EXENONBLOCKINGMCOI;
        output TSTAPUC405EXENONBLOCKINGMCOO;
        input TSTAPUC405EXERESULTI[0:31];
        output TSTAPUC405EXERESULTO[0:31];
        input TSTAPUC405EXEXERCAI;
        output TSTAPUC405EXEXERCAO;
        input TSTAPUC405EXEXEROVI;
        output TSTAPUC405EXEXEROVO;
        input TSTAPUC405FPUEXCEPTIONI;
        output TSTAPUC405FPUEXCEPTIONO;
        input TSTAPUC405LWBLDDEPENDI;
        output TSTAPUC405LWBLDDEPENDO;
        input TSTAPUC405SLEEPREQI;
        output TSTAPUC405SLEEPREQO;
        input TSTAPUC405WBLDDEPENDI;
        output TSTAPUC405WBLDDEPENDO;
        input TSTC405APUDCDFULLI;
        output TSTC405APUDCDFULLO;
        input TSTC405APUDCDHOLDI;
        output TSTC405APUDCDHOLDO;
        input TSTC405APUDCDINSTRUCTIONI[0:31];
        output TSTC405APUDCDINSTRUCTIONO[0:31];
        input TSTC405APUEXEFLUSHI;
        output TSTC405APUEXEFLUSHO;
        input TSTC405APUEXEHOLDI;
        output TSTC405APUEXEHOLDO;
        input TSTC405APUEXELOADDBUSI[0:31];
        output TSTC405APUEXELOADDBUSO[0:31];
        input TSTC405APUEXELOADDVALIDI;
        output TSTC405APUEXELOADDVALIDO;
        input TSTC405APUEXERADATAI[0:31];
        output TSTC405APUEXERADATAO[0:31];
        input TSTC405APUEXERBDATAI[0:31];
        output TSTC405APUEXERBDATAO[0:31];
        input TSTC405APUEXEWDCNTI[0:1];
        output TSTC405APUEXEWDCNTO[0:1];
        input TSTC405APUMSRFE0I;
        output TSTC405APUMSRFE0O;
        input TSTC405APUMSRFE1I;
        output TSTC405APUMSRFE1O;
        input TSTC405APUWBBYTEENI[0:3];
        output TSTC405APUWBBYTEENO[0:3];
        input TSTC405APUWBENDIANI;
        output TSTC405APUWBENDIANO;
        input TSTC405APUWBFLUSHI;
        output TSTC405APUWBFLUSHO;
        input TSTC405APUWBHOLDI;
        output TSTC405APUWBHOLDO;
        input TSTC405APUXERCAI;
        output TSTC405APUXERCAO;

        input LSSDCE0A;
        input LSSDCE0CNTLPOINT;
        input LSSDCE0SCAN;
        input LSSDCE0TESTM3;
        input LSSDCE1B;
        input LSSDCE1C1;
        input LSSDCE1C3BIST;
        input LSSDCE1CA1;
        input LSSDCE1CRAM;
        input LSSDSCANIN[0:15];
        output LSSDSCANOUT[0:15];

        input TESTSELI;
        output DIAGOUT;

        input TIEC405CLOCKENABLE;
        input TIEC405CLOCKSELECTS[0:1];
        input TIEC405DCUMARGIN;
        input TIEC405DETERMINISTICMULT;
        input TIEC405DISOPERANDFWD;
        input TIEC405DUTYENABLE;
        input TIEC405ICUMARGIN;
        input TIEC405MMUEN;
        input TIEC405TAGMARGIN;
        input TIEC405TLBMARGIN;
        input TIEPVRBIT[0:31];

        input TSTCLKINACTI;
        output TSTCLKINACTO;
        input TSTCPUCLKENI;
        output TSTCPUCLKENO;
        input TSTJTAGENI;
        output TSTJTAGENO;
        output TSTPLBSAMPLECYCLEO;
        input TSTRESETCHIPI;
        output TSTRESETCHIPO;
        input TSTRESETCOREI;
        output TSTRESETCOREO;
        input TSTRESETSYSI;
        output TSTRESETSYSO;
        input TSTSEPPCEMACI;
        input TSTSIGASKETI[0:1];
        output TSTSOGASKETO[0:1];
        input TSTTIMERENI;
        output TSTTIMERENO;
        input TSTTRSTNEGI;
        output TSTTRSTNEGO;
        input TSTUSECPMCLKSELI;

        input BISTCE0CONTINUE;
        input BISTCE0DIAGSHIFTSEL;
        input BISTCE0LOADIN;
        input BISTCE0LOADOPCODE;
        input BISTCE0TESTM1;

        input C405TESTRESERVE1;
        input C405TESTRESERVE2;
    }

    bel_class EMAC_V4 {
        input RESET;

        // all except DCREMACENABLE are non-routable on virtex4 (bolted directly to PPC)
        input DCREMACCLK;
        input DCREMACENABLE;
        input DCREMACREAD;
        input DCREMACWRITE;
        input DCREMACABUS[10];
        input DCREMACDBUS[32];
        output EMACDCRACK;
        output EMACDCRDBUS[32];

        output DCRHOSTDONEIR;

        input HOSTCLK;
        input HOSTREQ;
        input HOSTOPCODE[2];
        input HOSTEMAC1SEL;
        input HOSTMIIMSEL;
        output HOSTMIIMRDY;
        input HOSTADDR[10];
        input HOSTWRDATA[32];
        output HOSTRDDATA[32];

        for i in 0..2 {
            input "CLIENTEMAC{i}DCMLOCKED";
            input "CLIENTEMAC{i}PAUSEREQ";
            input "CLIENTEMAC{i}PAUSEVAL"[16];
            input "CLIENTEMAC{i}RXCLIENTCLKIN";
            input "CLIENTEMAC{i}TXCLIENTCLKIN";
            input "CLIENTEMAC{i}TXD"[16];
            input "CLIENTEMAC{i}TXDVLD";
            input "CLIENTEMAC{i}TXDVLDMSW";
            input "CLIENTEMAC{i}TXFIRSTBYTE";
            input "CLIENTEMAC{i}TXGMIIMIICLKIN";
            input "CLIENTEMAC{i}TXIFGDELAY"[8];
            input "CLIENTEMAC{i}TXUNDERRUN";

            output "EMAC{i}CLIENTANINTERRUPT";
            output "EMAC{i}CLIENTRXBADFRAME";
            output "EMAC{i}CLIENTRXCLIENTCLKOUT";
            output "EMAC{i}CLIENTRXD"[16];
            output "EMAC{i}CLIENTRXDVLD";
            output "EMAC{i}CLIENTRXDVLDMSW";
            output "EMAC{i}CLIENTRXDVREG6";
            output "EMAC{i}CLIENTRXFRAMEDROP";
            output "EMAC{i}CLIENTRXGOODFRAME";
            output "EMAC{i}CLIENTRXSTATS"[7];
            output "EMAC{i}CLIENTRXSTATSBYTEVLD";
            output "EMAC{i}CLIENTRXSTATSVLD";
            output "EMAC{i}CLIENTTXACK";
            output "EMAC{i}CLIENTTXCLIENTCLKOUT";
            output "EMAC{i}CLIENTTXCOLLISION";
            output "EMAC{i}CLIENTTXRETRANSMIT";
            output "EMAC{i}CLIENTTXSTATS";
            output "EMAC{i}CLIENTTXSTATSBYTEVLD";
            output "EMAC{i}CLIENTTXSTATSVLD";
            // virtex4 only
            output "EMAC{i}CLIENTTXGMIIMIICLKOUT";

            input "EMAC{i}TIBUS"[5];
            // virtex5 only
            output "EMAC{i}TOBUS"[5];

            input "PHYEMAC{i}COL";
            input "PHYEMAC{i}CRS";
            input "PHYEMAC{i}GTXCLK";
            input "PHYEMAC{i}MCLKIN";
            input "PHYEMAC{i}MDIN";
            input "PHYEMAC{i}MIITXCLK";
            input "PHYEMAC{i}PHYAD"[5];
            input "PHYEMAC{i}RXBUFERR";
            input "PHYEMAC{i}RXBUFSTATUS"[2];
            input "PHYEMAC{i}RXCHARISCOMMA";
            input "PHYEMAC{i}RXCHARISK";
            input "PHYEMAC{i}RXCHECKINGCRC";
            input "PHYEMAC{i}RXCLK";
            input "PHYEMAC{i}RXCLKCORCNT"[3];
            input "PHYEMAC{i}RXCOMMADET";
            input "PHYEMAC{i}RXD"[8];
            input "PHYEMAC{i}RXDISPERR";
            input "PHYEMAC{i}RXDV";
            input "PHYEMAC{i}RXER";
            input "PHYEMAC{i}RXLOSSOFSYNC"[2];
            input "PHYEMAC{i}RXNOTINTABLE";
            input "PHYEMAC{i}RXRUNDISP";
            input "PHYEMAC{i}SIGNALDET";
            input "PHYEMAC{i}TXBUFERR";
            // virtex5 only
            input "PHYEMAC{i}TXGMIIMIICLKIN";

            output "EMAC{i}PHYENCOMMAALIGN";
            output "EMAC{i}PHYLOOPBACKMSB";
            output "EMAC{i}PHYMCLKOUT";
            output "EMAC{i}PHYMDOUT";
            output "EMAC{i}PHYMDTRI";
            output "EMAC{i}PHYMGTRXRESET";
            output "EMAC{i}PHYMGTTXRESET";
            output "EMAC{i}PHYPOWERDOWN";
            output "EMAC{i}PHYSYNCACQSTATUS";
            output "EMAC{i}PHYTXCHARDISPMODE";
            output "EMAC{i}PHYTXCHARDISPVAL";
            output "EMAC{i}PHYTXCHARISK";
            output "EMAC{i}PHYTXCLK";
            output "EMAC{i}PHYTXD"[8];
            output "EMAC{i}PHYTXEN";
            output "EMAC{i}PHYTXER";
            // virtex5 only
            output "EMAC{i}PHYTXGMIIMIICLKOUT";
            output "EMAC{i}SPEEDIS10100";

            // virtex4 only
            input "TIEEMAC{i}CONFIGVEC"[80];
            input "TIEEMAC{i}UNICASTADDR"[48];

            // virtex5 only
            attribute "EMAC{i}_1000BASEX_ENABLE": bool;
            attribute "EMAC{i}_ADDRFILTER_ENABLE": bool;
            attribute "EMAC{i}_BYTEPHY": bool;
            attribute "EMAC{i}_CONFIGVEC_79": bool;
            attribute "EMAC{i}_DCRBASEADDR": bitvec[8];
            attribute "EMAC{i}_FUNCTION": bitvec[3];
            attribute "EMAC{i}_GTLOOPBACK": bool;
            attribute "EMAC{i}_HOST_ENABLE": bool;
            attribute "EMAC{i}_LINKTIMERVAL": bitvec[9];
            attribute "EMAC{i}_LTCHECK_DISABLE": bool;
            attribute "EMAC{i}_MDIO_ENABLE": bool;
            attribute "EMAC{i}_PAUSEADDR": bitvec[48];
            attribute "EMAC{i}_PHYINITAUTONEG_ENABLE": bool;
            attribute "EMAC{i}_PHYISOLATE": bool;
            attribute "EMAC{i}_PHYLOOPBACKMSB": bool;
            attribute "EMAC{i}_PHYPOWERDOWN": bool;
            attribute "EMAC{i}_PHYRESET": bool;
            attribute "EMAC{i}_RGMII_ENABLE": bool;
            attribute "EMAC{i}_RX16BITCLIENT_ENABLE": bool;
            attribute "EMAC{i}_RXFLOWCTRL_ENABLE": bool;
            attribute "EMAC{i}_RXHALFDUPLEX": bool;
            attribute "EMAC{i}_RXINBANDFCS_ENABLE": bool;
            attribute "EMAC{i}_RXJUMBOFRAME_ENABLE": bool;
            attribute "EMAC{i}_RXRESET": bool;
            attribute "EMAC{i}_RXVLAN_ENABLE": bool;
            attribute "EMAC{i}_RX_ENABLE": bool;
            attribute "EMAC{i}_SGMII_ENABLE": bool;
            attribute "EMAC{i}_SPEED_LSB": bool;
            attribute "EMAC{i}_SPEED_MSB": bool;
            attribute "EMAC{i}_TX16BITCLIENT_ENABLE": bool;
            attribute "EMAC{i}_TXFLOWCTRL_ENABLE": bool;
            attribute "EMAC{i}_TXHALFDUPLEX": bool;
            attribute "EMAC{i}_TXIFGADJUST_ENABLE": bool;
            attribute "EMAC{i}_TXINBANDFCS_ENABLE": bool;
            attribute "EMAC{i}_TXJUMBOFRAME_ENABLE": bool;
            attribute "EMAC{i}_TXRESET": bool;
            attribute "EMAC{i}_TXVLAN_ENABLE": bool;
            attribute "EMAC{i}_TX_ENABLE": bool;
            attribute "EMAC{i}_UNICASTADDR": bitvec[48];
            attribute "EMAC{i}_UNIDIRECTION_ENABLE": bool;
            attribute "EMAC{i}_USECLKEN": bool;
        }

        // virtex5 only
        input TESTSELI;
        input TSTSEEMACI;

        input TSTSIEMACI[7];
        output TSTSOEMACO[7];
    }

    enum GT11_ALIGN_COMMA_WORD { _1, _2, _4 }
    enum GT11_CHAN_BOND_MODE { NONE, MASTER, SLAVE_1_HOP, SLAVE_2_HOPS }
    enum GT11_CHAN_BOND_SEQ_LEN { _1, _2, _3, _4, _8 }
    enum GT11_CLK_COR_SEQ_LEN { _1, _2, _3, _4, _8 }
    enum GT11_FDCAL_CLOCK_DIVIDE { TWO, NONE, FOUR }
    enum GT11_RX_LOS_INVALID_INCR { _1, _2, _4, _8, _16, _32, _64, _128 }
    enum GT11_RX_LOS_THRESHOLD { _4, _8, _16, _32, _64, _128, _256, _512 }
    enum GT11_RXTXOUTDIV2SEL { _1, _2, _4, _8, _16, _32 }
    enum GT11_PLLNDIVSEL { _8, _10, _16, _20, _32, _40 }
    enum GT11_PMACLKSEL { REFCLK1, REFCLK2, GREFCLK }
    enum GT11_RXUSRDIVISOR { _1, _2, _4, _8, _16 }

    bel_class GT11 {
        input REFCLK1, REFCLK2;
        input GREFCLK;
        output RXPCSHCLKOUT;
        output TXPCSHCLKOUT;

        input POWERDOWN;
        input LOOPBACK[2];

        input DCLK;
        input DEN;
        input DWE;
        input DADDR[8];
        input DI[16];
        output DRDY;
        output DO[16];

        input RXUSRCLK;
        input RXUSRCLK2;
        output RXRECCLK1;
        output RXRECCLK2;
        input RXRESET;

        input RXPMARESET;
        output RXCALFAIL;
        output RXENABLECAL;
        output RXCOARSEST;
        output RXFCALSTATE[3];
        output RXFDETSTATE[3];
        input RXCLKSTABLE;
        output RXCYCLELIMIT;
        output RXLOCK;
        output RXLOCKUPDATE;
        input RXPOLARITY;
        output RXSIGDET;
        input RXSYNC;
        input RXUSRLOCK;
        input RXUSRVCOCAL;
        input RXUSRVCODAC[10];
        output RXVCOHIGH;
        output RXADCN;
        output RXADCP;
        output CDRSTATUS[18];

        input RXDATAWIDTH[2];
        input RXINTDATAWIDTH[2];
        output RXDATA[64];
        output RXNOTINTABLE[8];
        output RXDISPERR[8];
        output RXCHARISK[8];
        output RXCHARISCOMMA[8];
        output RXRUNDISP[8];
        input RXDEC8B10BUSE;
        input RXDEC64B66BUSE;
        input RXBLOCKSYNC64B66BUSE;
        input RXDESCRAM64B66BUSE;
        input RXCOMMADETUSE;
        input RXIGNOREBTF;
        output RXCOMMADET;
        output RXREALIGN;
        input RXSLIDE;
        input ENMCOMMAALIGN;
        input ENPCOMMAALIGN;
        output RXLOSSOFSYNC[2];
        output RXSTATUS[6];
        output RXBUFERR;
        input ENCHANSYNC;
        input CHBONDI[5];
        output CHBONDO[5];

        input MGTADCSEL[5];

        input TXUSRCLK;
        input TXUSRCLK2;
        output TXOUTCLK1;
        output TXOUTCLK2;
        input TXRESET;

        input TXPMARESET;
        input TXPOLARITY;
        input TXINHIBIT;
        output TXCALFAIL;
        input TXCLKSTABLE;
        output TXCYCLELIMIT;
        output TXCOARSEST;
        output TXENABLECAL;
        input TXENOOB;
        output TXFCALSTATE[3];
        output TXFDETSTATE[3];
        output TXLOCK;
        output TXLOCKUPDATE;
        input TXSYNC;
        input TXUSRLOCK;
        input TXUSRVCOCAL;
        input TXUSRVCODAC[10];
        output TXVCOHIGH;
        output TXADCN;
        output TXADCP;

        input TXDATAWIDTH[2];
        input TXINTDATAWIDTH[2];
        input TXDATA[64];
        input TXBYPASS8B10B[8];
        input TXCHARISK[8];
        input TXCHARDISPMODE[8];
        input TXCHARDISPVAL[8];
        input TXENC8B10BUSE;
        input TXENC64B66BUSE;
        input TXSCRAM64B66BUSE;
        input TXGEARBOX64B66BUSE;
        output TXKERR[8];
        output TXRUNDISP[8];
        output TXBUFERR;

        input RXCRCCLK;
        input RXCRCINTCLK;
        input RXCRCRESET;
        input RXCRCPD;
        input RXCRCDATAVALID;
        input RXCRCDATAWIDTH[3];
        input RXCRCIN[64];
        input RXCRCINIT;
        output RXCRCOUT[32];

        input TXCRCCLK;
        input TXCRCINTCLK;
        input TXCRCRESET;
        input TXCRCPD;
        input TXCRCDATAVALID;
        input TXCRCDATAWIDTH[3];
        input TXCRCIN[64];
        input TXCRCINIT;
        output TXCRCOUT[32];

        input SCANEN[3];
        input SCANIN[3];
        input SCANMODE[3];
        output SCANOUT[3];
        input TESTMEMORY;

        pad RXP, RXN: input;
        pad TXP, TXN: output;
        pad AVCCAUXRX: power;
        pad VTRX, VTTX: power;

        // address 0x40 and up
        attribute DRP: bitvec[16][64];
        attribute DRP_MASK: bitvec[64];

        attribute AUTO_CAL: bool;
        attribute BYPASS_CAL: bool;
        attribute BYPASS_FDET: bool;
        attribute CCCB_ARBITRATOR_DISABLE: bool;
        attribute CHAN_BOND_ONE_SHOT: bool;
        attribute CHAN_BOND_SEQ_2_USE: bool;
        attribute CLK_COR_8B10B_DE: bool;
        attribute CLK_CORRECT_USE: bool;
        attribute CLK_COR_SEQ_2_USE: bool;
        attribute CLK_COR_SEQ_DROP: bool;
        attribute COMMA32: bool;
        attribute DEC_MCOMMA_DETECT: bool;
        attribute DEC_PCOMMA_DETECT: bool;
        attribute DEC_VALID_COMMA_ONLY: bool;
        attribute DIGRX_SYNC_MODE: bool;
        attribute ENABLE_DCDR: bool;
        attribute MCOMMA_DETECT: bool;
        attribute OPPOSITE_SELECT: bool;
        attribute PCOMMA_DETECT: bool;
        attribute PCS_BIT_SLIP: bool;
        attribute PMA_BIT_SLIP: bool;
        attribute POWER_ENABLE: bool;
        attribute REPEATER: bool;
        attribute RESERVED_CB1: bool;
        attribute RESERVED_CCA: bool;
        attribute RESERVED_CCB: bool;
        attribute RESERVED_M2: bool;
        attribute RXACTST: bool;
        attribute RXADCADJPD: bool;
        attribute RXAFEPD: bool;
        attribute RXAFETST: bool;
        attribute RXAPD: bool;
        attribute RXAPTST: bool;
        attribute RXAUTO_CAL: bool;
        attribute RXBIASPD: bool;
        attribute RX_BUFFER_USE: bool;
        attribute RXBY_32: bool;
        attribute RXBYPASS_CAL: bool;
        attribute RXBYPASS_FDET: bool;
        attribute RXCLK0_FORCE_PMACLK: bool;
        attribute RXCLK0_INVERT_PMALEAF: bool;
        attribute RXCMFPD: bool;
        attribute RXCMFTST: bool;
        attribute RXCPSEL: bool;
        attribute RXCPTST: bool;
        attribute RXCRCCLOCKDOUBLE: bool;
        attribute RXCRCENABLE: bool;
        attribute RXCRCINVERTGEN: bool;
        attribute RXCRCSAMECLOCK: bool;
        attribute RXDACSEL: bool;
        attribute RXDACTST: bool;
        attribute RXDCCOUPLE: bool;
        attribute RXDIGRESET: bool;
        attribute RXDIGRX: bool;
        attribute RXDIVBUFPD: bool;
        attribute RXDIVBUFTST: bool;
        attribute RXDIVPD: bool;
        attribute RXDIVTST: bool;
        attribute RXFILTTST: bool;
        attribute RXLB: bool;
        attribute RXLKAPD: bool;
        attribute RXPDDTST: bool;
        attribute RXPD: bool;
        attribute RXPFDTST: bool;
        attribute RXPFDTX: bool;
        attribute RXQPPD: bool;
        attribute RXRCPPD: bool;
        attribute RXRECCLK1_USE_SYNC: bool;
        attribute RXRPDPD: bool;
        attribute RXRSDPD: bool;
        attribute RXSLOSEL: bool;
        attribute RXTADJ: bool;
        attribute RXVCOBUFPD: bool;
        attribute RXVCOBUFTST: bool;
        attribute RXVCO_CTRL_ENABLE: bool;
        attribute RXVCOPD: bool;
        attribute RXVCOTST: bool;
        attribute SAMPLE_8X: bool;
        attribute TEST_MODE_1: bool;
        attribute TEST_MODE_2: bool;
        attribute TEST_MODE_3: bool;
        attribute TXAREFBIASSEL: bool;
        attribute TX_BUFFER_USE: bool;
        attribute TXCFGENABLE: bool;
        attribute TXCLK0_FORCE_PMACLK: bool;
        attribute TXCLK0_INVERT_PMALEAF: bool;
        attribute TXCRCCLOCKDOUBLE: bool;
        attribute TXCRCENABLE: bool;
        attribute TXCRCINVERTGEN: bool;
        attribute TXCRCSAMECLOCK: bool;
        attribute TXDIGPD: bool;
        attribute TXHIGHSIGNALEN: bool;
        attribute TXLVLSHFTPD: bool;
        attribute TXOUTCLK1_USE_SYNC: bool;
        attribute TXPD: bool;
        attribute TXPHASESEL: bool;
        attribute TXPOST_TAP_PD: bool;
        attribute TXPRE_TAP_PD: bool;
        attribute TXSLEWRATE: bool;
        attribute VCO_CTRL_ENABLE: bool;

        attribute CLK_COR_SEQ_1_1: bitvec[11];
        attribute CLK_COR_SEQ_1_2: bitvec[11];
        attribute CLK_COR_SEQ_1_3: bitvec[11];
        attribute CLK_COR_SEQ_1_4: bitvec[11];
        attribute CLK_COR_SEQ_2_1: bitvec[11];
        attribute CLK_COR_SEQ_2_2: bitvec[11];
        attribute CLK_COR_SEQ_2_3: bitvec[11];
        attribute CLK_COR_SEQ_2_4: bitvec[11];
        attribute CHAN_BOND_SEQ_1_1: bitvec[11];
        attribute CHAN_BOND_SEQ_1_2: bitvec[11];
        attribute CHAN_BOND_SEQ_1_3: bitvec[11];
        attribute CHAN_BOND_SEQ_1_4: bitvec[11];
        attribute CHAN_BOND_SEQ_2_1: bitvec[11];
        attribute CHAN_BOND_SEQ_2_2: bitvec[11];
        attribute CHAN_BOND_SEQ_2_3: bitvec[11];
        attribute CHAN_BOND_SEQ_2_4: bitvec[11];
        attribute CLK_COR_SEQ_1_MASK: bitvec[4];
        attribute CLK_COR_SEQ_2_MASK: bitvec[4];
        attribute CHAN_BOND_SEQ_1_MASK: bitvec[4];
        attribute CHAN_BOND_SEQ_2_MASK: bitvec[4];
        attribute CHAN_BOND_TUNE: bitvec[8];
        attribute CYCLE_LIMIT_SEL: bitvec[2];
        attribute RXCYCLE_LIMIT_SEL: bitvec[2];
        attribute DCDR_FILTER: bitvec[3];
        attribute DIGRX_FWDCLK: bitvec[2];
        attribute FDET_HYS_CAL: bitvec[3];
        attribute FDET_HYS_SEL: bitvec[3];
        attribute FDET_LCK_CAL: bitvec[3];
        attribute FDET_LCK_SEL: bitvec[3];
        attribute LOOPCAL_WAIT: bitvec[2];
        attribute RXAFEEQ: bitvec[9];
        attribute RXASYNCDIVIDE: bitvec[2];
        attribute RXCDRLOS: bitvec[6];
        attribute RXCLKMODE: bitvec[6];
        attribute RXCLMODE: bitvec[2];
        attribute RXCMADJ: bitvec[2];
        attribute RXDATA_SEL: bitvec[2];
        attribute RXFDET_HYS_CAL: bitvec[3];
        attribute RXFDET_HYS_SEL: bitvec[3];
        attribute RXFDET_LCK_CAL: bitvec[3];
        attribute RXFDET_LCK_SEL: bitvec[3];
        attribute RXFECONTROL1: bitvec[2];
        attribute RXFECONTROL2: bitvec[3];
        attribute RXFETUNE: bitvec[2];
        attribute RXLKADJ: bitvec[5];
        attribute RXLOOPCAL_WAIT: bitvec[2];
        attribute RXLOOPFILT: bitvec[4];
        attribute RXMODE: bitvec[6];
        attribute RXRCPADJ: bitvec[3];
        attribute RXRIBADJ: bitvec[2];
        attribute RXSLOWDOWN_CAL: bitvec[2];
        attribute RXVCODAC_INIT: bitvec[10];
        attribute RX_CLOCK_DIVIDER: bitvec[2];
        attribute SLOWDOWN_CAL: bitvec[2];
        attribute TXASYNCDIVIDE: bitvec[2];
        attribute TXCLKMODE: bitvec[4];
        attribute TXDATA_SEL: bitvec[2];
        attribute TXDAT_PRDRV_DAC: bitvec[3];
        attribute TXDAT_TAP_DAC: bitvec[5];
        attribute TXLNDR_TST1: bitvec[4];
        attribute TXLNDR_TST2: bitvec[2];
        attribute TXPOST_PRDRV_DAC: bitvec[3];
        attribute TXPOST_TAP_DAC: bitvec[5];
        attribute TXPRE_PRDRV_DAC: bitvec[3];
        attribute TXPRE_TAP_DAC: bitvec[5];
        attribute TXTERMTRIM: bitvec[4];
        attribute TX_CLOCK_DIVIDER: bitvec[2];
        attribute VCODAC_INIT: bitvec[10];
        attribute COMMA_10B_MASK: bitvec[10];
        attribute RESERVED_CM: bitvec[24];
        attribute RESERVED_CM2: bitvec[22];
        attribute RXCRCINITVAL: bitvec[32];
        attribute RXCTRL1: bitvec[10];
        attribute RXEQ: bitvec[64];
        attribute RXTUNE: bitvec[13];
        attribute TXCRCINITVAL: bitvec[32];
        attribute TXLNDR_TST3: bitvec[15];
        attribute CHAN_BOND_LIMIT: bitvec[5];
        attribute CLK_COR_MIN_LAT: bitvec[6];
        attribute CLK_COR_MAX_LAT: bitvec[6];
        attribute SH_INVALID_CNT_MAX: bitvec[8];
        attribute SH_CNT_MAX: bitvec[8];
        attribute MCOMMA_VALUE: bitvec[32];
        attribute PCOMMA_VALUE: bitvec[32];

        // TODO: RXOUTDIV2SEL split
        // TODO: intify RXUSRDIVISOR, RX_LOS_INVALID_INCR, RX_LOS_THRESHOLD (div4!)
        attribute ALIGN_COMMA_WORD: GT11_ALIGN_COMMA_WORD;
        attribute CHAN_BOND_MODE: GT11_CHAN_BOND_MODE;
        attribute CHAN_BOND_SEQ_LEN: GT11_CHAN_BOND_SEQ_LEN;
        attribute CLK_COR_SEQ_LEN: GT11_CLK_COR_SEQ_LEN;
        attribute RXFDCAL_CLOCK_DIVIDE: GT11_FDCAL_CLOCK_DIVIDE;
        attribute RX_LOS_INVALID_INCR: GT11_RX_LOS_INVALID_INCR;
        attribute RX_LOS_THRESHOLD: GT11_RX_LOS_THRESHOLD;
        attribute RXOUTDIV2SEL: GT11_RXTXOUTDIV2SEL;
        attribute RXPLLNDIVSEL: GT11_PLLNDIVSEL;
        attribute RXPMACLKSEL: GT11_PMACLKSEL;
        attribute RXUSRDIVISOR: GT11_RXUSRDIVISOR;
        attribute TXFDCAL_CLOCK_DIVIDE: GT11_FDCAL_CLOCK_DIVIDE;
        attribute TXOUTDIV2SEL: GT11_RXTXOUTDIV2SEL;
    }

    enum GT11_REFCLKSEL { SYNCLK1IN, SYNCLK2IN, RXBCLK, REFCLK, MGTCLK }
    enum GT11_SYNCLK_DRIVE { NONE, BUF_UP, BUF_DOWN, DRIVE_UP, DRIVE_DOWN, DRIVE_BOTH }
    bel_class GT11CLK {
        input REFCLK;
        output SYNCLK1, SYNCLK2;

        pad CLKP, CLKN: input;
        pad GNDA: power;
        pad AVCCAUXMGT: power;
        pad AVCCAUXTX: power;

        // these attributes are specified on GT11, but actually apply to both GT11 in tile
        attribute TXADCADJPD: bool;
        attribute TXAPTST: bool;
        attribute TXAPD: bool;
        attribute TXBIASPD: bool;
        attribute TXCMFPD: bool;
        attribute TXCMFTST: bool;
        attribute TXCPSEL: bool;
        attribute TXDIVPD: bool;
        attribute TXDIVTST: bool;
        attribute TXDIVBUFPD: bool;
        attribute TXDIVBUFTST: bool;
        attribute TXDIGRX: bool;
        attribute TXDACTST: bool;
        attribute TXDACSEL: bool;
        attribute TXFILTTST: bool;
        attribute TXPFDTST: bool;
        attribute TXPFDTX: bool;
        attribute TXQPPD: bool;
        attribute TXSLOSEL: bool;
        attribute TXVCOBUFPD: bool;
        attribute TXVCOBUFTST: bool;
        attribute TXVCOPD: bool;
        attribute TXVCOTST: bool;
        attribute NATBENABLE: bool;
        attribute ATBENABLE: bool;
        attribute ATBBUMPEN: bool;
        attribute BIASRESSEL: bool;
        attribute PMATUNE: bool;
        attribute PMABIASPD: bool;
        attribute PMACOREPWRENABLE: bool;
        attribute PMACTRL: bool;
        attribute VREFSELECT: bool;
        attribute BANDGAPSEL: bool;
        attribute IREFBIASMODE: bitvec[2];
        attribute PMAIREFTRIM: bitvec[4];
        attribute PMAVBGCTRL: bitvec[5];
        attribute PMAVREFTRIM: bitvec[4];
        attribute RXAREGCTRL: bitvec[5];
        attribute TXCLMODE: bitvec[2];
        attribute TXLOOPFILT: bitvec[4];
        attribute TXREGCTRL: bitvec[5];
        attribute VREFBIASMODE: bitvec[2];
        attribute ATBSEL: bitvec[18];
        attribute PMACFG2SPARE: bitvec[46];
        attribute TXCTRL1: bitvec[10];
        attribute TXTUNE: bitvec[13];
        attribute TXABPMACLKSEL: GT11_PMACLKSEL;
        attribute TXPLLNDIVSEL: GT11_PLLNDIVSEL;

        attribute REFCLKSEL: GT11_REFCLKSEL;
        attribute SYNCLK1_DRIVE: GT11_SYNCLK_DRIVE;
        attribute SYNCLK2_DRIVE: GT11_SYNCLK_DRIVE;
        attribute SYNCLK_DRIVE_ENABLE: bool;
        attribute SYNCLK_ENABLE: bool;
    }

    if variant virtex4 {
        region_slot GLOBAL;
        region_slot GIOB;
        region_slot HROW;
        region_slot LEAF;
        region_slot LEAF_DCM;

        wire PULLUP: pullup;
        wire TIE_0: tie 0;
        wire TIE_1: tie 1;

        wire HCLK[8]: regional LEAF;
        wire RCLK[2]: regional LEAF;

        wire OMUX[16]: mux;
        wire OMUX_S0: branch N;
        wire OMUX_S0_ALT: branch N;
        wire OMUX_W1: branch E;
        wire OMUX_WS1: branch N;
        wire OMUX_E2: branch W;
        wire OMUX_S2: branch N;
        wire OMUX_S3: branch N;
        wire OMUX_SE3: branch W;
        wire OMUX_S4: branch N;
        wire OMUX_S5: branch N;
        wire OMUX_SW5: branch E;
        wire OMUX_W6: branch E;
        wire OMUX_E7: branch W;
        wire OMUX_ES7: branch N;
        wire OMUX_E8: branch W;
        wire OMUX_EN8: branch S;
        wire OMUX_W9: branch E;
        wire OMUX_N10: branch S;
        wire OMUX_NW10: branch E;
        wire OMUX_N11: branch S;
        wire OMUX_N12: branch S;
        wire OMUX_NE12: branch W;
        wire OMUX_E13: branch W;
        wire OMUX_N13: branch S;
        wire OMUX_W14: branch E;
        wire OMUX_WN14: branch S;
        wire OMUX_N15: branch S;

        wire DBL_W0[10]: mux;
        wire DBL_W1[10]: branch E;
        wire DBL_W2[10]: branch E;
        wire DBL_W2_N[10]: branch S;
        wire DBL_E0[10]: mux;
        wire DBL_E1[10]: branch W;
        wire DBL_E2[10]: branch W;
        wire DBL_E2_S[10]: branch N;
        wire DBL_S0[10]: mux;
        wire DBL_S1[10]: branch N;
        wire DBL_S2[10]: branch N;
        wire DBL_S3[10]: branch N;
        wire DBL_N0[10]: mux;
        wire DBL_N1[10]: branch S;
        wire DBL_N2[10]: branch S;
        wire DBL_N3[10]: branch S;

        wire HEX_W0[10]: mux;
        for i in 1..=6 {
            wire "HEX_W{i}"[10]: branch E;
        }
        wire HEX_W6_N[10]: branch S;
        wire HEX_E0[10]: mux;
        for i in 1..=6 {
            wire "HEX_E{i}"[10]: branch W;
        }
        wire HEX_E6_S[10]: branch N;
        wire HEX_S0[10]: mux;
        for i in 1..=7 {
            wire "HEX_S{i}"[10]: branch N;
        }
        wire HEX_N0[10]: mux;
        for i in 1..=7 {
            wire "HEX_N{i}"[10]: branch S;
        }

        wire LH[25] {
            0..12 => multi_branch W,
            12 => multi_root,
            13..25 => multi_branch E,
        }
        wire LV[25] {
            0..12 => multi_branch S,
            12 => multi_root,
            13..25 => multi_branch N,
        }

        wire IMUX_SR[4]: mux;
        wire IMUX_SR_OPTINV[4]: mux;
        wire IMUX_BOUNCE[4]: mux;
        wire IMUX_CLK[4]: mux;
        wire IMUX_CLK_OPTINV[4]: mux;
        wire IMUX_CE[4]: mux;
        wire IMUX_CE_OPTINV[4]: mux;
        wire IMUX_BYP[8]: mux;
        wire IMUX_BYP_BOUNCE[8]: mux;
        wire IMUX_IMUX[32]: mux;

        wire OUT_BEST[8]: bel;
        wire OUT_BEST_TMIN[8]: bel;
        wire OUT_SEC[8]: bel;
        wire OUT_SEC_TMIN[8]: bel;
        wire OUT_HALF0[8]: bel;
        wire OUT_HALF0_BEL[8]: bel;
        wire OUT_HALF0_TEST[8]: test;
        wire OUT_HALF1[8]: bel;
        wire OUT_HALF1_BEL[8]: bel;
        wire OUT_HALF1_TEST[8]: test;

        wire IMUX_SPEC[4]: test;

        wire HCLK_ROW[8]: regional HROW;
        wire RCLK_ROW[2]: regional HROW;
        wire MGT_ROW[2]: regional HROW;

        wire OUT_BUFG[32]: bel;
        wire GCLK[32]: regional GLOBAL;
        wire GCLK_BUF[32]: mux;
        wire GIOB[16]: regional GIOB;
        wire IMUX_BUFG_O[32]: mux;
        wire IMUX_BUFG_I[32]: branch CLK_PREV;

        wire OUT_CLKPAD: bel;

        wire HCLK_IO[8]: regional LEAF;
        wire RCLK_IO[2]: regional LEAF;
        wire IMUX_IDELAYCTRL_REFCLK: mux;
        wire IMUX_BUFR[2]: mux;
        wire IOCLK[2]: regional LEAF;
        wire IOCLK_S[2]: branch IO_N;
        wire IOCLK_N[2]: branch IO_S;
        wire IOCLK_S_IO[2]: regional LEAF;
        wire IOCLK_N_IO[2]: regional LEAF;
        wire VRCLK[2]: mux;
        wire VRCLK_S[2]: branch IO_N;
        wire VRCLK_N[2]: branch IO_S;

        wire HCLK_DCM[8]: regional LEAF_DCM;
        wire GIOB_DCM[16]: regional LEAF_DCM;
        wire MGT_DCM[4]: regional LEAF_DCM;
        wire OUT_DCM[12]: mux;

        wire IMUX_CCM_REL[2]: mux;
        wire OUT_CCM_CLKA1[2]: bel;
        wire OUT_CCM_CLKA1D2[2]: bel;
        wire OUT_CCM_CLKA1D4[2]: bel;
        wire OUT_CCM_CLKA1D8[2]: bel;
        wire OUT_CCM_CLKB1[2]: bel;
        wire OUT_CCM_CLKC1[2]: bel;
        wire OUT_CCM_CLKD1[2]: bel;
        wire OUT_CCM_REFCLKOUT: bel;
        wire OUT_CCM_OSCOUT1: bel;
        wire OUT_CCM_OSCOUT2: bel;
        wire OUT_DCM_LOCKED: bel;

        wire DCM_DCM_O[24]: mux;
        wire DCM_DCM_I[24]: branch CMT_PREV;

        wire HCLK_MGT[8]: mux;
        wire MGT_CLK_OUT[2]: mux;
        wire MGT_CLK_OUT_SYNCLK: mux;
        wire MGT_CLK_OUT_FWDCLK[2]: mux;
        wire MGT_FWDCLK_S[4]: multi_branch MGT_S;
        wire MGT_FWDCLK_N[4]: multi_root;

        wire IMUX_MGT_GREFCLK: mux;
        wire IMUX_MGT_REFCLK: mux;
        wire IMUX_MGT_GREFCLK_PRE[2]: mux;
        wire IMUX_MGT_REFCLK_PRE[2]: mux;
        wire OUT_MGT_SYNCLK[2]: bel;
        wire OUT_MGT_RXPCSHCLKOUT[2]: bel;
        wire OUT_MGT_TXPCSHCLKOUT[2]: bel;
    }

    if variant virtex5 {
        region_slot GLOBAL;
        region_slot GIOB;
        region_slot HROW;
        region_slot LEAF;

        wire PULLUP: pullup;
        wire TIE_0: tie 0;
        wire TIE_1: tie 1;

        wire HCLK[10]: regional LEAF;
        wire RCLK[4]: regional LEAF;

        wire DBL_WW0_S0: mux;
        wire DBL_WW0_N5: mux;
        wire DBL_WW0[6] {
            0 => branch S,
            1..5 => mux,
            5 => branch N,
        }
        wire DBL_WW1[6]: branch E;
        wire DBL_WW2[6]: branch E;
        wire DBL_WS0[3]: mux;
        wire DBL_WS1[3]: branch E;
        wire DBL_WS2[3]: branch N;
        wire DBL_WS1_BUF0: mux;
        wire DBL_WS1_S0: branch N;
        wire DBL_WN0[3]: mux;
        wire DBL_WN1[3]: branch E;
        wire DBL_WN2[3]: branch S;
        wire DBL_WN2_S0: branch N;
        wire DBL_EE0_S3: mux;
        wire DBL_EE0[6] {
            0..3 => mux,
            3 => branch S,
            4..6 => mux,
        }
        wire DBL_EE1[6]: branch W;
        wire DBL_EE2[6]: branch W;
        wire DBL_ES0[3]: mux;
        wire DBL_ES1[3]: branch W;
        wire DBL_ES2[3]: branch N;
        wire DBL_EN0[3]: mux;
        wire DBL_EN1[3]: branch W;
        wire DBL_EN2[3]: branch S;
        wire DBL_NN0_S0: mux;
        wire DBL_NN0_N5: mux;
        wire DBL_NN0[6] {
            0 => branch S,
            1..5 => mux,
            5 => branch N,
        }
        wire DBL_NN1[6]: branch S;
        wire DBL_NN2[6]: branch S;
        wire DBL_NW0[3]: mux;
        wire DBL_NW1[3]: branch S;
        wire DBL_NW2[3]: branch E;
        wire DBL_NW2_N2: branch S;
        wire DBL_NE0[3]: mux;
        wire DBL_NE1[3]: branch S;
        wire DBL_NE2[3]: branch W;
        wire DBL_NE1_BUF2: mux;
        wire DBL_NE1_N2: branch S;
        wire DBL_SS0_N2: mux;
        wire DBL_SS0[6] {
            0..2 => mux,
            2 => branch N,
            3..6 => mux,
        }
        wire DBL_SS1[6]: branch N;
        wire DBL_SS2[6]: branch N;
        wire DBL_SW0[3]: mux;
        wire DBL_SW1[3]: branch N;
        wire DBL_SW2[3]: branch E;
        wire DBL_SE0[3]: mux;
        wire DBL_SE1[3]: branch N;
        wire DBL_SE2[3]: branch W;

        wire PENT_WW0_S0: mux;
        wire PENT_WW0[6] {
            0 => branch S,
            1..6 => mux,
        }
        wire PENT_WW1[6]: branch E;
        wire PENT_WW2[6]: branch E;
        wire PENT_WW3[6]: branch E;
        wire PENT_WW4[6]: branch E;
        wire PENT_WW5[6]: branch E;
        wire PENT_WS0[3]: mux;
        wire PENT_WS1[3]: branch E;
        wire PENT_WS2[3]: branch E;
        wire PENT_WS3[3]: branch E;
        wire PENT_WS4[3]: branch N;
        wire PENT_WS5[3]: branch N;
        wire PENT_WS3_BUF0: mux;
        wire PENT_WS3_S0: branch N;
        wire PENT_WN0[3]: mux;
        wire PENT_WN1[3]: branch E;
        wire PENT_WN2[3]: branch E;
        wire PENT_WN3[3]: branch E;
        wire PENT_WN4[3]: branch S;
        wire PENT_WN5[3]: branch S;
        wire PENT_WN5_S0: branch N;
        wire PENT_EE0[6]: mux;
        wire PENT_EE1[6]: branch W;
        wire PENT_EE2[6]: branch W;
        wire PENT_EE3[6]: branch W;
        wire PENT_EE4[6]: branch W;
        wire PENT_EE5[6]: branch W;
        wire PENT_ES0[3]: mux;
        wire PENT_ES1[3]: branch W;
        wire PENT_ES2[3]: branch W;
        wire PENT_ES3[3]: branch W;
        wire PENT_ES4[3]: branch N;
        wire PENT_ES5[3]: branch N;
        wire PENT_EN0[3]: mux;
        wire PENT_EN1[3]: branch W;
        wire PENT_EN2[3]: branch W;
        wire PENT_EN3[3]: branch W;
        wire PENT_EN4[3]: branch S;
        wire PENT_EN5[3]: branch S;
        wire PENT_SS0[6]: mux;
        wire PENT_SS1[6]: branch N;
        wire PENT_SS2[6]: branch N;
        wire PENT_SS3[6]: branch N;
        wire PENT_SS4[6]: branch N;
        wire PENT_SS5[6]: branch N;
        wire PENT_SW0[3]: mux;
        wire PENT_SW1[3]: branch N;
        wire PENT_SW2[3]: branch N;
        wire PENT_SW3[3]: branch N;
        wire PENT_SW4[3]: branch E;
        wire PENT_SW5[3]: branch E;
        wire PENT_SE0[3]: mux;
        wire PENT_SE1[3]: branch N;
        wire PENT_SE2[3]: branch N;
        wire PENT_SE3[3]: branch N;
        wire PENT_SE4[3]: branch W;
        wire PENT_SE5[3]: branch W;
        wire PENT_NN0_N5: mux;
        wire PENT_NN0[6] {
            0..5 => mux,
            5 => branch N,
        }
        wire PENT_NN1[6]: branch S;
        wire PENT_NN2[6]: branch S;
        wire PENT_NN3[6]: branch S;
        wire PENT_NN4[6]: branch S;
        wire PENT_NN5[6]: branch S;
        wire PENT_NW0[3]: mux;
        wire PENT_NW1[3]: branch S;
        wire PENT_NW2[3]: branch S;
        wire PENT_NW3[3]: branch S;
        wire PENT_NW4[3]: branch E;
        wire PENT_NW5[3]: branch E;
        wire PENT_NW5_N2: branch S;
        wire PENT_NE0[3]: mux;
        wire PENT_NE1[3]: branch S;
        wire PENT_NE2[3]: branch S;
        wire PENT_NE3[3]: branch S;
        wire PENT_NE4[3]: branch W;
        wire PENT_NE5[3]: branch W;
        wire PENT_NE3_BUF2: mux;
        wire PENT_NE3_N2: branch S;

        wire LH[19] {
            0..9 => multi_branch W,
            9 => multi_root,
            10..19 => multi_branch E,
        }
        wire LV[19] {
            0..9 => multi_branch N,
            9 => multi_root,
            10..19 => multi_branch S,
        }

        wire IMUX_GFAN[2]: mux;
        wire IMUX_CLK[2]: mux;

        wire IMUX_CTRL[4]: mux;
        wire IMUX_CTRL_SITE[4]: mux;
        wire IMUX_CTRL_BOUNCE[4]: mux;
        wire IMUX_CTRL_BOUNCE_S0: branch N;
        wire IMUX_CTRL_BOUNCE_N3: branch S;

        wire IMUX_BYP[8]: mux;
        wire IMUX_BYP_SITE[8]: mux;
        wire IMUX_BYP_BOUNCE[8]: mux;
        wire IMUX_BYP_BOUNCE_S0: branch N;
        wire IMUX_BYP_BOUNCE_N3: branch S;
        wire IMUX_BYP_BOUNCE_S4: branch N;
        wire IMUX_BYP_BOUNCE_N7: branch S;

        wire IMUX_FAN[8]: mux;
        wire IMUX_FAN_SITE[8]: mux;
        wire IMUX_FAN_BOUNCE[8]: mux;
        wire IMUX_FAN_BOUNCE_S0: branch N;
        wire IMUX_FAN_BOUNCE_N7: branch S;

        wire IMUX_IMUX[48]: mux;
        wire IMUX_IMUX_DELAY[48]: mux;

        wire OUT[24]: bel;
        wire OUT_BEL[24]: bel;
        wire OUT_TEST[24]: test;
        wire OUT_S12_DBL: branch N;
        wire OUT_N15_DBL: branch S;
        wire OUT_N17_DBL: branch S;
        wire OUT_S18_DBL: branch N;
        wire OUT_S12_PENT: branch N;
        wire OUT_N15_PENT: branch S;
        wire OUT_N17_PENT: branch S;
        wire OUT_S18_PENT: branch N;

        wire IMUX_SPEC[4]: test;

        wire HCLK_ROW[10]: regional HROW;
        wire RCLK_ROW[4]: regional HROW;
        wire MGT_ROW_I[5]: branch MGT_ROW_PREV;
        wire MGT_ROW_O[5]: mux;

        wire OUT_BUFG[32]: bel;
        wire GCLK[32]: regional GLOBAL;
        wire GCLK_BUF[32]: mux;
        wire GIOB[10]: regional GIOB;
        wire IMUX_BUFG_O[32]: mux;
        wire IMUX_BUFG_I[32]: branch CLK_PREV;
            wire MGT_BUF[10]: mux;

        wire OUT_CLKPAD: bel;

        wire HCLK_IO[10]: regional LEAF;
        wire RCLK_IO[4]: regional LEAF;
        wire IMUX_IDELAYCTRL_REFCLK: mux;
        wire IMUX_BUFR[2]: mux;
        wire IOCLK[4]: regional LEAF;
        wire VRCLK[2]: mux;
        wire VRCLK_S[2]: branch IO_N;
        wire VRCLK_N[2]: branch IO_S;

        wire IMUX_IO_ICLK[2]: mux;
        // ?!?!??!?? why three bits for inversion
        wire IMUX_IO_ICLK_OPTINV[2]: mux;
        wire IMUX_ILOGIC_CLK[2]: mux;
        wire IMUX_ILOGIC_CLKB[2]: mux;

        wire HCLK_CMT[10]: regional LEAF;
        wire GIOB_CMT[10]: regional LEAF;

        wire OUT_CMT[28]: bel;
            wire IMUX_DCM_CLKIN[2]: mux;
            wire IMUX_DCM_CLKFB[2]: mux;
            wire OMUX_DCM_SKEWCLKIN1[2]: mux;
            wire OMUX_DCM_SKEWCLKIN2[2]: mux;
            wire IMUX_PLL_CLKIN1: mux;
            wire IMUX_PLL_CLKIN2: mux;
            wire IMUX_PLL_CLKFB: mux;
            wire TEST_PLL_CLKIN: bel;
            wire OMUX_PLL_SKEWCLKIN1: mux;
            wire OMUX_PLL_SKEWCLKIN2: mux;
            wire OUT_PLL_CLKOUTDCM[6]: bel;
            wire OUT_PLL_CLKFBDCM: bel;
    }

    if variant virtex6 {
        region_slot GLOBAL;
        region_slot GIOB;
        region_slot HROW;
        region_slot LEAF;

        wire TIE_0: tie 0;
        wire TIE_1: tie 1;

        wire LCLK[8]: regional LEAF;

        wire SNG_W0_N3: mux;
        wire SNG_W0_S4: mux;
        wire SNG_W0[8] {
            0..3 => mux,
            3 => branch N,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_W1[8]: branch E;
        wire SNG_W1_S4: branch N;
        wire SNG_W1_N3: branch S;
        wire SNG_E0_N3: mux;
        wire SNG_E0_S4: mux;
        wire SNG_E0[8] {
            0..3 => mux,
            3 => branch N,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_E1[8]: branch W;
        wire SNG_E1_S0: branch N;
        wire SNG_E1_N7: branch S;
        wire SNG_S0_S4: mux;
        wire SNG_S0[8] {
            0..4 => mux,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_S1[8]: branch N;
        wire SNG_S1_N7: branch S;
        wire SNG_N0_N3: mux;
        wire SNG_N0[8] {
            0..3 => mux,
            3 => branch N,
            4..8 => mux,
        }
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
        wire QUAD_SW1[4]: branch E;
        wire QUAD_SW2[4]: branch N;
        wire QUAD_SW3[4]: branch N;
        wire QUAD_SW4[4]: branch E;
        wire QUAD_SW4_N3: branch S;
        wire QUAD_SE0[4]: mux;
        wire QUAD_SE1[4]: branch W;
        wire QUAD_SE2[4]: branch N;
        wire QUAD_SE3[4]: branch N;
        wire QUAD_SE4[4]: branch W;
        wire QUAD_NN0[4]: mux;
        wire QUAD_NN1[4]: branch S;
        wire QUAD_NN2[4]: branch S;
        wire QUAD_NN3[4]: branch S;
        wire QUAD_NN4[4]: branch S;
        wire QUAD_NN4_S0: branch N;
        wire QUAD_NW0[4]: mux;
        wire QUAD_NW1[4]: branch E;
        wire QUAD_NW2[4]: branch S;
        wire QUAD_NW3[4]: branch S;
        wire QUAD_NW4[4]: branch E;
        wire QUAD_NW4_S0: branch N;
        wire QUAD_NE0[4]: mux;
        wire QUAD_NE1[4]: branch W;
        wire QUAD_NE2[4]: branch S;
        wire QUAD_NE3[4]: branch S;
        wire QUAD_NE4[4]: branch W;

        wire LH[17] {
            0..8 => multi_branch W,
            8 => multi_root,
            9..17 => multi_branch E,
        }
        wire LV[17] {
            0..8 => multi_branch N,
            8 => multi_root,
            9..17 => multi_branch S,
        }

        wire IMUX_GFAN[2]: mux;
        wire IMUX_CLK[2]: mux;
        wire IMUX_CTRL[2]: mux;

        wire IMUX_BYP[8]: mux;
        wire IMUX_BYP_SITE[8]: mux;
        wire IMUX_BYP_BOUNCE[8]: mux;
        wire IMUX_BYP_BOUNCE_N[8]: branch S;

        wire IMUX_FAN[8]: mux;
        wire IMUX_FAN_SITE[8]: mux;
        wire IMUX_FAN_BOUNCE[8]: mux;
        wire IMUX_FAN_BOUNCE_S[8]: branch N;

        wire IMUX_IMUX[48]: mux;
        wire IMUX_IMUX_DELAY[48]: mux;

        wire OUT[24]: bel;
        wire OUT_BEL[24]: bel;
        wire OUT_TEST[24]: test;

        wire TEST[4]: test;
    }

    if variant virtex7 {
        region_slot GLOBAL;
        region_slot HROW;
        region_slot LEAF;

        wire TIE_0: tie 0;
        wire TIE_1: tie 1;

        wire LCLK[12]: bel;

        wire SNG_W0_N3: mux;
        wire SNG_W0_S4: mux;
        wire SNG_W0[8] {
            0..3 => mux,
            3 => branch N,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_W1[8]: branch E;
        wire SNG_W1_S4: branch N;
        wire SNG_W1_N3: branch S;
        wire SNG_E0_N3: mux;
        wire SNG_E0_S4: mux;
        wire SNG_E0[8] {
            0..3 => mux,
            3 => branch N,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_E1[8]: branch W;
        wire SNG_E1_S0: branch N;
        wire SNG_E1_N7: branch S;
        wire SNG_S0_S4: mux;
        wire SNG_S0[8] {
            0..4 => mux,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_S1[8]: branch N;
        wire SNG_S1_N7: branch S;
        wire SNG_N0_N3: mux;
        wire SNG_N0[8] {
            0..3 => mux,
            3 => branch N,
            4..8 => mux,
        }
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

        wire HEX_SS0[4]: mux;
        wire HEX_SS1[4]: branch N;
        wire HEX_SS2[4]: branch N;
        wire HEX_SS3[4]: branch N;
        wire HEX_SS4[4]: branch N;
        wire HEX_SS5[4]: branch N;
        wire HEX_SS6[4]: branch N;
        wire HEX_SS6_N3: branch S;
        wire HEX_SW0[4]: mux;
        wire HEX_SW1[4]: branch E;
        wire HEX_SW2[4]: branch N;
        wire HEX_SW3[4]: branch N;
        wire HEX_SW4[4]: branch N;
        wire HEX_SW5[4]: branch N;
        wire HEX_SW6[4]: branch E;
        wire HEX_SW6_N3: branch S;
        wire HEX_SE0[4]: mux;
        wire HEX_SE1[4]: branch W;
        wire HEX_SE2[4]: branch N;
        wire HEX_SE3[4]: branch N;
        wire HEX_SE4[4]: branch N;
        wire HEX_SE5[4]: branch N;
        wire HEX_SE6[4]: branch W;
        wire HEX_NN0[4]: mux;
        wire HEX_NN1[4]: branch S;
        wire HEX_NN2[4]: branch S;
        wire HEX_NN3[4]: branch S;
        wire HEX_NN4[4]: branch S;
        wire HEX_NN5[4]: branch S;
        wire HEX_NN6[4]: branch S;
        wire HEX_NN6_S0: branch N;
        wire HEX_NW0[4]: mux;
        wire HEX_NW1[4]: branch E;
        wire HEX_NW2[4]: branch S;
        wire HEX_NW3[4]: branch S;
        wire HEX_NW4[4]: branch S;
        wire HEX_NW5[4]: branch S;
        wire HEX_NW6[4]: branch E;
        wire HEX_NW6_S0: branch N;
        wire HEX_NE0[4]: mux;
        wire HEX_NE1[4]: branch W;
        wire HEX_NE2[4]: branch S;
        wire HEX_NE3[4]: branch S;
        wire HEX_NE4[4]: branch S;
        wire HEX_NE5[4]: branch S;
        wire HEX_NE6[4]: branch W;

        wire LH[13] {
            0..6 => multi_branch W,
            6 => multi_root,
            7..13 => multi_branch E,
        }
        wire LV[19] {
            0..9 => multi_branch N,
            9 => multi_root,
            10..19 => multi_branch S,
        }
        wire LVB[13] {
            0..6 => multi_branch N,
            6 => multi_root,
            7..13 => multi_branch S,
        }

        wire IMUX_GFAN[2]: mux;
        wire IMUX_CLK[2]: mux;
        wire IMUX_CTRL[2]: mux;

        wire IMUX_BYP[8]: mux;
        wire IMUX_BYP_SITE[8]: mux;
        wire IMUX_BYP_BOUNCE[8]: mux;
        wire IMUX_BYP_BOUNCE_N[8]: branch S;

        wire IMUX_FAN[8]: mux;
        wire IMUX_FAN_SITE[8]: mux;
        wire IMUX_FAN_BOUNCE[8]: mux;
        wire IMUX_FAN_BOUNCE_S[8]: branch N;

        wire IMUX_IMUX[48]: mux;
        wire IMUX_IMUX_DELAY[48]: mux;
        wire IMUX_BRAM[48]: test;

        wire OUT[24]: bel;
        wire OUT_BEL[24]: bel;
        wire OUT_TEST[24]: test;

        wire TEST[4]: test;
    }

    bitrect REG32 = horizontal (1, rev 32);
    if variant virtex4 {
        bitrect INT = vertical (19, rev 80);
        bitrect CLB = vertical (22, rev 80);
        bitrect BRAM = vertical (20, rev 80);
        bitrect DSP = vertical (21, rev 80);
        bitrect IO = vertical (30, rev 80);
        bitrect GT = vertical (20, rev 80);
        bitrect CLK = vertical (3, rev 80);
        bitrect HCLK = vertical (20, rev 32);
        bitrect HCLK_IO = vertical (30, rev 32);
        bitrect HCLK_CLK = vertical (3, rev 32);
        bitrect BRAM_DATA = vertical (64, rev 320);
    }
    if variant virtex5 {
        bitrect INT = vertical (28, rev 64);
        bitrect CLB = vertical (36, rev 64);
        bitrect BRAM = vertical (30, rev 64);
        bitrect DSP = vertical (28, rev 64);
        bitrect IO = vertical (54, rev 64);
        bitrect GT = vertical (32, rev 64);
        bitrect CLK = vertical (4, rev 64);
        bitrect HCLK = vertical (28, rev 32);
        bitrect HCLK_GT = vertical (32, rev 32);
        bitrect HCLK_IO = vertical (54, rev 32);
        bitrect HCLK_CLK = vertical (4, rev 32);
        bitrect BRAM_DATA = vertical (128, rev 320);
    }
    if variant virtex6 {
        bitrect INT = vertical (28, rev 64);
        bitrect CLB = vertical (36, rev 64);
        bitrect BRAM = vertical (28, rev 64);
        bitrect DSP = vertical (28, rev 64);
        bitrect IO = vertical (44, rev 64);
        bitrect CMT = vertical (38, rev 64);
        bitrect GT = vertical (30, rev 64);
        bitrect HCLK = vertical (28, rev 32);
        bitrect HCLK_CMT = vertical (38, rev 32);
        bitrect HCLK_IO = vertical (44, rev 32);
        bitrect BRAM_DATA = vertical (128, rev 320);
    }
    if variant virtex7 {
        bitrect INT = vertical (28, rev 64);
        bitrect CLB = vertical (36, rev 64);
        bitrect BRAM = vertical (28, rev 64);
        bitrect DSP = vertical (28, rev 64);
        bitrect IO = vertical (42, rev 64);
        bitrect CMT = vertical (30, rev 64);
        bitrect CFG = vertical (30, rev 64);
        bitrect CLK = vertical (30, rev 64);
        bitrect GT = vertical (32, rev 64);
        bitrect HCLK = vertical (28, rev 32);
        bitrect HCLK_CMT = vertical (30, rev 32);
        bitrect HCLK_CLK = vertical (30, rev 32);
        bitrect HCLK_IO = vertical (42, rev 32);
        bitrect BRAM_DATA = vertical (128, rev 320);
    }

    tile_slot INT {
        bel_slot INT: routing;
        tile_class INT {
            cell CELL;
            bitrect MAIN: INT;
        }
    }

    tile_slot INTF {
        bel_slot INTF_INT: routing;
        bel_slot INTF_TESTMUX: routing;
        tile_class INTF {
            cell CELL;
            bitrect MAIN: INT;
        }
        if variant [virtex5, virtex6, virtex7] {
            tile_class INTF_DELAY {
                cell CELL;
                bitrect MAIN: INT;
            }
        }
        if variant virtex7 {
            tile_class INTF_BRAM {
                cell CELL;
                bitrect MAIN: INT;
            }
        }
    }

    tile_slot BEL {
        bel_slot SPEC_INT: routing;

        if variant virtex4 {
            bel_slot SLICE[4]: SLICE_V4;
        } else {
            bel_slot SLICE[4]: legacy;
        }
        if variant virtex4 {
            tile_class CLB {
                cell CELL;
                bitrect MAIN: CLB;
            }
        } else {
            tile_class CLBLL, CLBLM {
                cell CELL;
                bitrect MAIN: CLB;
            }
        }

        if variant virtex4 {
            bel_slot BRAM: BRAM_V4;
        } else {
            bel_slot BRAM: legacy;
        }

        bel_slot BRAM_F: legacy;
        bel_slot BRAM_H[2]: legacy;
        bel_slot BRAM_ADDR: legacy;

        if variant virtex4 {
            tile_class BRAM {
                cell CELL[4];
                bitrect MAIN[4]: BRAM;
                bitrect DATA: BRAM_DATA;
            }
        } else {
            tile_class BRAM {
                cell CELL[5];
                bitrect MAIN[5]: BRAM;
                bitrect DATA: BRAM_DATA;
            }
        }

        if variant virtex4 {
            bel_slot DSP[2]: DSP_V4;
        } else {
            bel_slot DSP[2]: legacy;
        }
        bel_slot DSP_C: DSP_C;
        bel_slot TIEOFF_DSP: legacy;
        if variant virtex4 {
            tile_class DSP {
                cell CELL[4];
                bitrect MAIN[4]: DSP;
            }
        } else {
            tile_class DSP {
                cell CELL[5];
                bitrect MAIN[5]: DSP;
            }
        }

        if variant virtex4 {
            bel_slot ILOGIC[2]: ILOGIC_V4;
            bel_slot OLOGIC[2]: OLOGIC_V4;
            bel_slot IODELAY[2]: legacy;
            bel_slot IDELAY[2]: legacy;
            bel_slot ODELAY[2]: legacy;
            bel_slot IOB[2]: IOB_V4;
        } else {
            bel_slot ILOGIC[2]: legacy;
            bel_slot OLOGIC[2]: legacy;
            bel_slot IODELAY[2]: legacy;
            bel_slot IDELAY[2]: legacy;
            bel_slot ODELAY[2]: legacy;
            bel_slot IOB[2]: legacy;
        }
        bel_slot IOI: legacy;
        if variant virtex4 {
            tile_class IO {
                cell CELL;
                bitrect MAIN: IO;
            }
        }
        if variant virtex5 {
            tile_class IO {
                cell CELL;
                bitrect MAIN: IO;
            }
        }
        if variant virtex6 {
            tile_class IO {
                cell CELL[2];
                bitrect MAIN[2]: IO;
            }
        }
        if variant virtex7 {
            tile_class IO_HP_S, IO_HP_N, IO_HR_S, IO_HR_N {
                cell CELL;
                bitrect MAIN: IO;
            }
            tile_class IO_HP_PAIR, IO_HR_PAIR {
                cell CELL[2];
                bitrect MAIN[2]: IO;
            }
        }

        if variant virtex4 {
            bel_slot DCM[2]: DCM_V4;
        } else {
            bel_slot DCM[2]: legacy;
        }
        bel_slot PLL: legacy;
        bel_slot MMCM[2]: legacy;
        bel_slot CMT: legacy;
        bel_slot CMT_A: legacy;
        bel_slot CMT_B: legacy;
        bel_slot CMT_C: legacy;
        bel_slot CMT_D: legacy;
        bel_slot HCLK_CMT: legacy;
        bel_slot PPR_FRAME: legacy;
        bel_slot PHASER_IN[4]: legacy;
        bel_slot PHASER_OUT[4]: legacy;
        bel_slot PHASER_REF: legacy;
        bel_slot PHY_CONTROL: legacy;
        bel_slot BUFMRCE[2]: legacy;
        if variant virtex4 {
            tile_class DCM {
                cell CELL[4];
                bitrect MAIN[4]: IO;
            }
        }
        if variant virtex5 {
            tile_class CMT {
                cell CELL[10];
                bitrect MAIN[10]: IO;
            }
        }
        if variant virtex6 {
            tile_class CMT {
                cell CELL[40];
                bitrect MAIN[40]: CMT;
                bitrect HCLK: HCLK_CMT;
            }
        }
        if variant virtex7 {
            tile_class CMT {
                cell CELL[50];
                bitrect MAIN[50]: CMT;
                bitrect HCLK: HCLK_CMT;
            }
        }

        bel_slot CCM: CCM;
        bel_slot PMCD[2]: PMCD;
        bel_slot DPM: DPM;
        if variant virtex4 {
            tile_class CCM {
                cell CELL[4];
                bitrect MAIN[4]: IO;
            }
        }

        bel_slot BUFHCE_W[12]: legacy;
        bel_slot BUFHCE_E[12]: legacy;
        bel_slot CLK_HROW_V7: legacy;
        bel_slot GCLK_TEST_BUF_HROW_GCLK[32]: legacy;
        bel_slot GCLK_TEST_BUF_HROW_BUFH_W: legacy;
        bel_slot GCLK_TEST_BUF_HROW_BUFH_E: legacy;
        if variant virtex7 {
            tile_class CLK_HROW {
                cell CELL[2];
                bitrect MAIN[8]: CLK;
                bitrect HCLK: HCLK_CLK;
            }
        }

        bel_slot PMV_CLK: legacy;
        bel_slot PMVIOB_CLK: legacy;
        bel_slot PMV2: legacy;
        bel_slot PMV2_SVT: legacy;
        bel_slot MTBF2: legacy;
        if variant virtex6 {
            tile_class PMVIOB {
                cell CELL[2];
                bitrect MAIN[2]: CMT;
            }
        }
        if variant virtex7 {
            tile_class CLK_PMV {
                cell CELL;
            }
            tile_class CLK_PMVIOB {
                cell CELL;
            }
            tile_class CLK_PMV2_SVT {
                cell CELL;
            }
            tile_class CLK_PMV2 {
                cell CELL;
            }
            tile_class CLK_MTBF2 {
                cell CELL;
            }
        }

        if variant virtex4 {
            bel_slot PPC: PPC405;
        } else {
            bel_slot PPC: legacy;
        }
        if variant virtex4 {
            tile_class PPC {
                cell CELL_W[24];
                cell CELL_E[24];
                cell CELL_S[7];
                cell CELL_N[7];
            }
        }
        if variant virtex5 {
            tile_class PPC {
                cell CELL_W[40];
                cell CELL_E[40];
                bitrect MAIN_W[40]: CLB;
                bitrect MAIN_E[40]: CLB;
            }
        }

        if variant [virtex4, virtex5] {
            bel_slot EMAC: EMAC_V4;
        } else {
            bel_slot EMAC: legacy;
        }
        if variant [virtex5, virtex6] {
            tile_class EMAC {
                cell CELL[10];
                bitrect MAIN[10]: BRAM;
            }
        }

        bel_slot PCIE: legacy;
        if variant virtex5 {
            tile_class PCIE {
                cell CELL[40];
                bitrect MAIN[40]: BRAM;
            }
        }
        if variant virtex6 {
            tile_class PCIE {
                cell CELL_W[20];
                cell CELL_E[20];
                bitrect MAIN[20]: BRAM;
            }
        }
        if variant virtex7 {
            tile_class PCIE {
                cell CELL_A[25];
                cell CELL_B[25];
                bitrect MAIN[25]: CLB;
            }
        }

        bel_slot PCIE3: legacy;
        if variant virtex7 {
            tile_class PCIE3 {
                cell CELL_W[50];
                cell CELL_E[50];
                bitrect MAIN[50]: CLB;
            }
        }

        bel_slot GT11[2]: GT11;
        bel_slot GT11CLK: GT11CLK;
        if variant virtex4 {
            tile_class MGT {
                cell CELL[32];
                bitrect MAIN[32]: GT;
            }
        }

        bel_slot GTP_DUAL: legacy;
        if variant virtex5 {
            tile_class GTP {
                cell CELL[20];
                bitrect MAIN[20]: GT;
                bitrect HCLK: HCLK_GT;
            }
        }

        bel_slot GTX_DUAL: legacy;
        if variant virtex5 {
            tile_class GTX {
                cell CELL[20];
                bitrect MAIN[20]: GT;
                bitrect HCLK: HCLK_GT;
            }
        }

        bel_slot GTX[4]: legacy;
        if variant virtex6 {
            tile_class GTX {
                cell CELL[40];
                bitrect MAIN[40]: GT;
            }
        }

        bel_slot GTH_QUAD: legacy;
        if variant virtex6 {
            tile_class GTH {
                cell CELL[40];
                bitrect MAIN[40]: GT;
            }
        }

        bel_slot GTP_COMMON: legacy;
        bel_slot GTX_COMMON: legacy;
        bel_slot GTH_COMMON: legacy;

        if variant virtex7 {
            tile_class GTP_COMMON {
                cell CELL[6];
                bitrect MAIN[6]: GT;
            }
            tile_class GTP_COMMON_MID {
                cell CELL[6];
                bitrect MAIN[6]: INT;
                bitrect HCLK: HCLK;
            }
            tile_class GTX_COMMON {
                cell CELL[6];
                bitrect MAIN[6]: GT;
            }
            tile_class GTH_COMMON {
                cell CELL[6];
                bitrect MAIN[6]: GT;
            }
        }

        bel_slot GTP_CHANNEL: legacy;
        bel_slot GTX_CHANNEL: legacy;
        bel_slot GTH_CHANNEL: legacy;

        if variant virtex7 {
            tile_class GTP_CHANNEL {
                cell CELL[11];
                bitrect MAIN[11]: GT;
            }
            tile_class GTP_CHANNEL_MID {
                cell CELL[11];
                bitrect MAIN[11]: INT;
            }
            tile_class GTX_CHANNEL {
                cell CELL[11];
                bitrect MAIN[11]: GT;
            }
            tile_class GTH_CHANNEL {
                cell CELL[11];
                bitrect MAIN[11]: GT;
            }
        }

        bel_slot BUFDS[2]: legacy;
        bel_slot CRC32[4]: legacy;
        bel_slot CRC64[2]: legacy;

        bel_slot IPAD_CLKP[2]: legacy;
        bel_slot IPAD_CLKN[2]: legacy;
        bel_slot IPAD_RXP[4]: legacy;
        bel_slot IPAD_RXN[4]: legacy;
        bel_slot OPAD_TXP[4]: legacy;
        bel_slot OPAD_TXN[4]: legacy;

        if variant [virtex4, virtex5] {
            bel_slot BUFGCTRL[32]: BUFGCTRL;
        } else {
            bel_slot BUFGCTRL[32]: legacy;
        }
        bel_slot GIO_S: legacy;
        bel_slot GIO_N: legacy;
        if variant virtex4 {
            tile_class CLK_BUFG {
                cell CELL[16];
                cell CELL_E0;
                cell CELL_E8;
                bitrect MAIN[16]: CLK;
            }
        }
        if variant virtex5 {
            tile_class CLK_BUFG {
                cell CELL[20];
                cell CELL_E0;
                cell CELL_E10;
                bitrect MAIN[20]: CLK;
            }
        }
        if variant virtex6 {
            tile_class CMT_BUFG_S, CMT_BUFG_N {
                cell CELL[3];
                bitrect MAIN[2]: CMT;
            }
        }
        if variant virtex7 {
            tile_class CLK_BUFG {
                cell CELL[4];
                bitrect MAIN[4]: CLK;
            }
        }

        bel_slot GCLK_BUF: legacy;
        if variant virtex6 {
            tile_class GCLK_BUF {
            }
        }

        bel_slot HCLK_GTX: legacy;
        bel_slot HCLK_GTH: legacy;

        bel_slot CLK_REBUF: legacy;
        bel_slot GCLK_TEST_BUF_REBUF_S[16]: legacy;
        bel_slot GCLK_TEST_BUF_REBUF_N[16]: legacy;
        if variant virtex7 {
            tile_class CLK_BUFG_REBUF {
                cell CELL[2];
                bitrect MAIN[2]: CLK;
            }
            tile_class CLK_BALI_REBUF {
                cell CELL[16];
                bitrect MAIN[16]: CLK;
            }
        }

        bel_slot PS: legacy;
        bel_slot HCLK_PS_S: legacy;
        bel_slot HCLK_PS_N: legacy;
        bel_slot IOPAD_DDRWEB: legacy;
        bel_slot IOPAD_DDRVRN: legacy;
        bel_slot IOPAD_DDRVRP: legacy;
        bel_slot IOPAD_DDRA[15]: legacy;
        bel_slot IOPAD_DDRBA[3]: legacy;
        bel_slot IOPAD_DDRCASB: legacy;
        bel_slot IOPAD_DDRCKE: legacy;
        bel_slot IOPAD_DDRCKN: legacy;
        bel_slot IOPAD_DDRCKP: legacy;
        bel_slot IOPAD_PSCLK: legacy;
        bel_slot IOPAD_DDRCSB: legacy;
        bel_slot IOPAD_DDRDM[4]: legacy;
        bel_slot IOPAD_DDRDQ[32]: legacy;
        bel_slot IOPAD_DDRDQSN[4]: legacy;
        bel_slot IOPAD_DDRDQSP[4]: legacy;
        bel_slot IOPAD_DDRDRSTB: legacy;
        bel_slot IOPAD_MIO[54]: legacy;
        bel_slot IOPAD_DDRODT: legacy;
        bel_slot IOPAD_PSPORB: legacy;
        bel_slot IOPAD_DDRRASB: legacy;
        bel_slot IOPAD_PSSRSTB: legacy;
        if variant virtex7 {
            tile_class PS {
                cell CELL[100];
            }
        }
    }

    tile_slot CMT_FIFO {
        bel_slot IN_FIFO: legacy;
        bel_slot OUT_FIFO: legacy;
        if variant virtex7 {
            tile_class CMT_FIFO {
                cell CELL[12];
                bitrect MAIN[12]: CMT;
            }
        }
    }

    tile_slot CFG {
        bel_slot SYSMON_INT: routing;
        if variant [virtex4, virtex5] {
            bel_slot BSCAN[4]: BSCAN;
            bel_slot ICAP[2]: ICAP;
            bel_slot STARTUP: STARTUP;
            bel_slot CAPTURE: CAPTURE;
            bel_slot JTAGPPC: JTAGPPC;
            bel_slot PMV_CFG[2]: PMV;
            bel_slot DCIRESET: DCIRESET;
            bel_slot FRAME_ECC: FRAME_ECC;
            bel_slot USR_ACCESS: USR_ACCESS;
            bel_slot DNA_PORT: legacy;
            bel_slot KEY_CLEAR: KEY_CLEAR;
            bel_slot EFUSE_USR: EFUSE_USR;
            bel_slot CFG_IO_ACCESS: legacy;
            bel_slot PMVIOB_CFG: legacy;
        } else {
            bel_slot BSCAN[4]: legacy;
            bel_slot ICAP[2]: legacy;
            bel_slot STARTUP: legacy;
            bel_slot CAPTURE: legacy;
            bel_slot JTAGPPC: legacy;
            bel_slot PMV_CFG[2]: legacy;
            bel_slot DCIRESET: legacy;
            bel_slot FRAME_ECC: legacy;
            bel_slot USR_ACCESS: legacy;
            bel_slot DNA_PORT: legacy;
            bel_slot KEY_CLEAR: legacy;
            bel_slot EFUSE_USR: legacy;
            bel_slot CFG_IO_ACCESS: legacy;
            bel_slot PMVIOB_CFG: legacy;
        }
        bel_slot MISC_CFG: MISC_CFG;
        if variant virtex4 {
            bel_slot SYSMON: SYSMON_V4;
        } else if variant virtex5 {
            bel_slot SYSMON: SYSMON_V5;
        } else {
            bel_slot SYSMON: legacy;
        }
        bel_slot IPAD_VP: legacy;
        bel_slot IPAD_VN: legacy;

        if variant virtex4 {
            tile_class CFG {
                cell CELL[16];
                bitrect MAIN[16]: IO;
            }
            tile_class SYSMON {
                cell CELL[8];
                bitrect MAIN[8]: IO;
            }
        }
        if variant virtex5 {
            tile_class CFG {
                cell CELL[20];
                bitrect MAIN[20]: IO;
            }
        }
        if variant virtex6 {
            tile_class CFG {
                cell CELL[80];
                bitrect MAIN[80]: CMT;
            }
        }
        if variant virtex7 {
            tile_class CFG {
                cell CELL[50];
                bitrect MAIN[50]: CFG;
            }
            tile_class SYSMON {
                cell CELL[25];
                bitrect MAIN[25]: CFG;
            }
        }
    }

    tile_slot CLK {
        bel_slot CLK_INT: routing;
        if variant virtex4 {
            tile_class CLK_DCM_S, CLK_DCM_N {
                cell CELL[8];
                bitrect MAIN[8]: CLK;
            }
            tile_class CLK_IOB_S, CLK_IOB_N {
                cell CELL[16];
                bitrect MAIN[16]: CLK;
            }
        }
        if variant virtex5 {
            tile_class CLK_CMT_S, CLK_CMT_N {
                cell CELL[10];
                cell CELL_E;
                bitrect MAIN[10]: CLK;
            }
            tile_class CLK_IOB_S, CLK_IOB_N {
                cell CELL[10];
                cell CELL_E;
                bitrect MAIN[10]: CLK;
            }
            tile_class CLK_MGT_S, CLK_MGT_N {
                cell CELL[10];
                cell CELL_E;
                bitrect MAIN[10]: CLK;
            }
        }

        bel_slot HCLK_MGT_BUF: legacy;
        if variant [virtex4, virtex5, virtex6] {
            tile_class HCLK_MGT_BUF {
                if variant [virtex4, virtex5] {
                    cell CELL;
                }
                bitrect MAIN: HCLK;
            }
        }

        bel_slot INT_LCLK_W: legacy;
        bel_slot INT_LCLK_E: legacy;
        if variant virtex7 {
            tile_class INT_LCLK {
                cell W, E;
            }
        }
    }

    tile_slot HROW {
        bel_slot HROW_INT: routing;
        if variant [virtex4, virtex5] {
            tile_class CLK_HROW {
                cell W, E;
                bitrect MAIN[2]: CLK;
                bitrect HCLK: HCLK_CLK;
            }
        }

        bel_slot HCLK_QBUF: legacy;
        if variant virtex6 {
            tile_class HCLK_QBUF {
            }
        }

        if variant virtex4 {
            tile_class CLK_TERM {
                cell CELL;
                bitrect MAIN: CLK;
            }

            tile_class HCLK_TERM {
                cell CELL;
                bitrect MAIN: HCLK;
            }
        }
    }

    tile_slot HCLK {
        if variant [virtex4, virtex5] {
            bel_slot HCLK: routing;
        } else {
            bel_slot HCLK: legacy;
        }
        bel_slot HCLK_W: legacy;
        bel_slot HCLK_E: legacy;
        if variant [virtex4, virtex5] {
            bel_slot GLOBALSIG: GLOBALSIG;
        } else {
            bel_slot GLOBALSIG: legacy;
        }

        if variant [virtex4, virtex5] {
            tile_class HCLK {
                cell CELL;
                bitrect MAIN: HCLK;
            }
        }
        if variant virtex6 {
            tile_class HCLK {
                cell S, N;
                bitrect MAIN: HCLK;
            }
        }
        if variant virtex7 {
            tile_class HCLK {
                bitrect MAIN[2]: HCLK;
            }
        }
    }

    tile_slot HCLK_BEL {
        if variant virtex5 {
            bel_slot PMVBRAM: PMVBRAM;
            bel_slot PMVBRAM_NC: PMVBRAM;
        } else {
            bel_slot PMVBRAM: legacy;
            bel_slot PMVBRAM_NC: legacy;
        }

        if variant [virtex5] {
            tile_class PMVBRAM {
                cell CELL[5];
            }
        }
        if variant [virtex6, virtex7] {
            tile_class PMVBRAM {
                cell CELL[15];
            }
        }
        if variant virtex7 {
            tile_class PMVBRAM_NC {
            }
        }

        bel_slot HCLK_IO_INT: routing;
        bel_slot HCLK_IO: legacy;
        bel_slot HCLK_CMT_DRP: HCLK_CMT_DRP;
        if variant [virtex4, virtex5] {
            bel_slot BUFR[4]: BUFR;
            bel_slot BUFIO[4]: BUFIO;
            bel_slot BUFO[2]: legacy;
            bel_slot IDELAYCTRL: IDELAYCTRL;
            bel_slot DCI: DCI_V4;
        } else {
            bel_slot BUFR[4]: legacy;
            bel_slot BUFIO[4]: legacy;
            bel_slot BUFO[2]: legacy;
            bel_slot IDELAYCTRL: legacy;
            bel_slot DCI: legacy;
        }
        bel_slot BANK: BANK;
        bel_slot LVDS: LVDS_V4;

        if variant virtex4 {
            tile_class HCLK_IO_DCI, HCLK_IO_LVDS, HCLK_IO_CENTER, HCLK_IO_CFG_N, HCLK_IO_DCM_S, HCLK_IO_DCM_N {
                cell CELL[4];
                if tile_class [HCLK_IO_DCM_S, HCLK_IO_DCM_N] {
                    cell CELL_E;
                }
                bitrect MAIN: HCLK_IO;

                bel BUFIO[0] {
                    input I = CELL[2].OUT_CLKPAD;
                    output O = CELL[2].IOCLK[0];
                }

                bel BUFIO[1] {
                    input I = CELL[1].OUT_CLKPAD;
                    output O = CELL[2].IOCLK[1];
                }
            }
            tile_class HCLK_DCM {
                cell CELL[4];
                cell CELL_E;
                bitrect MAIN: HCLK_IO;
            }
        }
        if variant virtex5 {
            tile_class HCLK_IO, HCLK_IO_CENTER, HCLK_IO_CFG_S, HCLK_IO_CMT_S, HCLK_IO_CFG_N, HCLK_IO_CMT_N, HCLK_CMT {
                cell CELL[4];
                bitrect MAIN: HCLK_IO;
            }
        }
        if variant virtex6 {
            tile_class HCLK_IO {
                cell CELL[2];
                bitrect MAIN: HCLK_IO;
            }
        }
        if variant virtex7 {
            tile_class HCLK_IO_HP, HCLK_IO_HR {
                cell CELL[8];
                bitrect MAIN: HCLK_IO;
            }
        }

        bel_slot BRKH_GTX: legacy;

        if variant virtex7 {
            tile_class BRKH_GTX {
            }
        }

        if variant virtex4 {
            tile_class HCLK_MGT {
                cell CELL;
                bitrect MAIN: HCLK;
                switchbox HCLK_IO_INT {
                    for i in 0..8 {
                        progbuf HCLK_MGT[i] = HCLK_ROW[i];
                    }
                    for i in 0..2 {
                        progbuf MGT_ROW[i] = MGT_CLK_OUT[i];
                    }
                }
            }
        }
    }

    tile_slot GLOBAL {
        bel_slot GLOBAL: GLOBAL;
        if variant virtex4 {
            tile_class GLOBAL {
                bitrect COR: REG32;
                bitrect CTL: REG32;
                bel GLOBAL;
            }
        }
        if variant virtex5 {
            tile_class GLOBAL {
                bitrect COR0: REG32;
                bitrect COR1: REG32;
                bitrect CTL0: REG32;
                bitrect CTL1: REG32;
                bitrect TIMER: REG32;
                bitrect WBSTAR: REG32;
                bitrect TESTMODE: REG32;
                bel GLOBAL;
            }
        }
    }

    connector_slot W {
        opposite E;

        if variant virtex4 {
            connector_class PASS_W {
                pass OMUX_E2 = OMUX[2];
                pass OMUX_SE3 = OMUX_S3;
                pass OMUX_E7 = OMUX[7];
                pass OMUX_E8 = OMUX[8];
                pass OMUX_NE12 = OMUX_N12;
                pass OMUX_E13 = OMUX[13];
                pass DBL_E1 = DBL_E0;
                pass DBL_E2 = DBL_E1;
                pass HEX_E1 = HEX_E0;
                pass HEX_E2 = HEX_E1;
                pass HEX_E3 = HEX_E2;
                pass HEX_E4 = HEX_E3;
                pass HEX_E5 = HEX_E4;
                pass HEX_E6 = HEX_E5;
                for i in 0..12 {
                    pass LH[i] = LH[i + 1];
                }
            }
            connector_class TERM_W;
            connector_class CLB_BUFFER_W;
            connector_class PPC_W;
        }
        if variant virtex5 {
            connector_class PASS_W {
                pass DBL_EE1 = DBL_EE0;
                pass DBL_EE2 = DBL_EE1;
                pass DBL_ES1 = DBL_ES0;
                pass DBL_EN1 = DBL_EN0;
                pass DBL_SE2 = DBL_SE1;
                pass DBL_NE2 = DBL_NE1;

                pass PENT_EE1 = PENT_EE0;
                pass PENT_EE2 = PENT_EE1;
                pass PENT_EE3 = PENT_EE2;
                pass PENT_EE4 = PENT_EE3;
                pass PENT_EE5 = PENT_EE4;
                pass PENT_ES1 = PENT_ES0;
                pass PENT_ES2 = PENT_ES1;
                pass PENT_ES3 = PENT_ES2;
                pass PENT_EN1 = PENT_EN0;
                pass PENT_EN2 = PENT_EN1;
                pass PENT_EN3 = PENT_EN2;
                pass PENT_SE4 = PENT_SE3;
                pass PENT_SE5 = PENT_SE4;
                pass PENT_NE4 = PENT_NE3;
                pass PENT_NE5 = PENT_NE4;

                for i in 0..9 {
                    pass LH[i] = LH[i + 1];
                }
            }
            connector_class TERM_W;
            connector_class INT_BUFS_W;
            connector_class PPC_W;
        }
        if variant virtex6 {
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
                pass QUAD_SE1 = QUAD_SE0;
                pass QUAD_SE4 = QUAD_SE3;
                pass QUAD_NE1 = QUAD_NE0;
                pass QUAD_NE4 = QUAD_NE3;

                for i in 0..8 {
                    pass LH[i] = LH[i + 1];
                }
            }
            connector_class TERM_W;
        }
        if variant virtex7 {
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
                pass HEX_SE1 = HEX_SE0;
                pass HEX_SE6 = HEX_SE5;
                pass HEX_NE1 = HEX_NE0;
                pass HEX_NE6 = HEX_NE5;

                for i in 0..6 {
                    pass LH[i] = LH[i + 1];
                }
            }
            connector_class TERM_W;
        }
    }

    connector_slot E {
        opposite W;

        if variant virtex4 {
            connector_class PASS_E {
                pass OMUX_W1 = OMUX[1];
                pass OMUX_SW5 = OMUX_S5;
                pass OMUX_W6 = OMUX[6];
                pass OMUX_W9 = OMUX[9];
                pass OMUX_NW10 = OMUX_N10;
                pass OMUX_W14 = OMUX[14];
                pass DBL_W1 = DBL_W0;
                pass DBL_W2 = DBL_W1;
                pass HEX_W1 = HEX_W0;
                pass HEX_W2 = HEX_W1;
                pass HEX_W3 = HEX_W2;
                pass HEX_W4 = HEX_W3;
                pass HEX_W5 = HEX_W4;
                pass HEX_W6 = HEX_W5;
                for i in 13..25 {
                    pass LH[i] = LH[i - 1];
                }
            }
            connector_class TERM_E;
            connector_class CLB_BUFFER_E;
            connector_class PPC_E;
        }
        if variant virtex5 {
            connector_class PASS_E {
                pass DBL_WW1 = DBL_WW0;
                pass DBL_WW2 = DBL_WW1;
                pass DBL_WS1 = DBL_WS0;
                pass DBL_WN1 = DBL_WN0;
                pass DBL_SW2 = DBL_SW1;
                pass DBL_NW2 = DBL_NW1;

                pass PENT_WW1 = PENT_WW0;
                pass PENT_WW2 = PENT_WW1;
                pass PENT_WW3 = PENT_WW2;
                pass PENT_WW4 = PENT_WW3;
                pass PENT_WW5 = PENT_WW4;
                pass PENT_WS1 = PENT_WS0;
                pass PENT_WS2 = PENT_WS1;
                pass PENT_WS3 = PENT_WS2;
                pass PENT_WN1 = PENT_WN0;
                pass PENT_WN2 = PENT_WN1;
                pass PENT_WN3 = PENT_WN2;
                pass PENT_SW4 = PENT_SW3;
                pass PENT_SW5 = PENT_SW4;
                pass PENT_NW4 = PENT_NW3;
                pass PENT_NW5 = PENT_NW4;

                for i in 10..19 {
                    pass LH[i] = LH[i - 1];
                }
            }
            connector_class TERM_E;
            connector_class TERM_E_HOLE {
                for i in 10..19 {
                    blackhole LH[i];
                }
            }
            connector_class INT_BUFS_E;
            connector_class PPC_E;
        }
        if variant virtex6 {
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
                pass QUAD_SW1 = QUAD_SW0;
                pass QUAD_SW4 = QUAD_SW3;
                pass QUAD_NW1 = QUAD_NW0;
                pass QUAD_NW4 = QUAD_NW3;

                for i in 9..17 {
                    pass LH[i] = LH[i - 1];
                }
            }
            connector_class TERM_E;
        }
        if variant virtex7 {
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
                pass HEX_SW1 = HEX_SW0;
                pass HEX_SW6 = HEX_SW5;
                pass HEX_NW1 = HEX_NW0;
                pass HEX_NW6 = HEX_NW5;

                for i in 7..13 {
                    pass LH[i] = LH[i - 1];
                }
            }
            connector_class TERM_E;
        }
    }

    connector_slot S {
        opposite N;

        if variant virtex4 {
            connector_class PASS_S {
                pass OMUX_EN8 = OMUX_E8;
                pass OMUX_N10 = OMUX[10];
                pass OMUX_N11 = OMUX[11];
                pass OMUX_N12 = OMUX[12];
                pass OMUX_N13 = OMUX[13];
                pass OMUX_WN14 = OMUX_W14;
                pass OMUX_N15 = OMUX[15];
                pass DBL_W2_N = DBL_W2;
                pass DBL_N1 = DBL_N0;
                pass DBL_N2 = DBL_N1;
                pass DBL_N3 = DBL_N2;
                pass HEX_W6_N = HEX_W6;
                pass HEX_N1 = HEX_N0;
                pass HEX_N2 = HEX_N1;
                pass HEX_N3 = HEX_N2;
                pass HEX_N4 = HEX_N3;
                pass HEX_N5 = HEX_N4;
                pass HEX_N6 = HEX_N5;
                pass HEX_N7 = HEX_N6;
                for i in 0..12 {
                    pass LV[i] = LV[i + 1];
                }
            }
            connector_class TERM_S;
            connector_class BRKH_S;
            connector_class PPC_A_S;
            connector_class PPC_B_S;
        }
        if variant virtex5 {
            connector_class PASS_S {
                pass DBL_NN1 = DBL_NN0;
                pass DBL_NN2 = DBL_NN1;
                pass DBL_NW1 = DBL_NW0;
                pass DBL_NE1 = DBL_NE0;
                pass DBL_WN2 = DBL_WN1;
                pass DBL_EN2 = DBL_EN1;
                pass DBL_WW0[0] = DBL_WW0_S0;
                pass DBL_EE0[3] = DBL_EE0_S3;
                pass DBL_NN0[0] = DBL_NN0_S0;
                pass DBL_NW2_N2 = DBL_NW2[2];
                pass DBL_NE1_N2 = DBL_NE1_BUF2;

                pass PENT_NN1 = PENT_NN0;
                pass PENT_NN2 = PENT_NN1;
                pass PENT_NN3 = PENT_NN2;
                pass PENT_NN4 = PENT_NN3;
                pass PENT_NN5 = PENT_NN4;
                pass PENT_NW1 = PENT_NW0;
                pass PENT_NW2 = PENT_NW1;
                pass PENT_NW3 = PENT_NW2;
                pass PENT_NE1 = PENT_NE0;
                pass PENT_NE2 = PENT_NE1;
                pass PENT_NE3 = PENT_NE2;
                pass PENT_WN4 = PENT_WN3;
                pass PENT_WN5 = PENT_WN4;
                pass PENT_EN4 = PENT_EN3;
                pass PENT_EN5 = PENT_EN4;
                pass PENT_WW0[0] = PENT_WW0_S0;
                pass PENT_NW5_N2 = PENT_NW5[2];
                pass PENT_NE3_N2 = PENT_NE3_BUF2;

                for i in 10..19 {
                    pass LV[i] = LV[i - 1];
                }

                pass IMUX_CTRL_BOUNCE_N3 = IMUX_CTRL_BOUNCE[3];
                pass IMUX_BYP_BOUNCE_N3 = IMUX_BYP_BOUNCE[3];
                pass IMUX_BYP_BOUNCE_N7 = IMUX_BYP_BOUNCE[7];
                pass IMUX_FAN_BOUNCE_N7 = IMUX_FAN_BOUNCE[7];
                pass OUT_N15_DBL = OUT[15];
                pass OUT_N17_DBL = OUT[17];
                pass OUT_N15_PENT = OUT[15];
                pass OUT_N17_PENT = OUT[17];
            }
            connector_class TERM_S_HOLE {
                for i in 10..19 {
                    blackhole LV[i];
                }
            }
            connector_class TERM_S_PPC;
        }
        if variant virtex6 {
            connector_class PASS_S {
                pass SNG_N1 = SNG_N0;
                pass SNG_W0[4] = SNG_W0_S4;
                pass SNG_E0[4] = SNG_E0_S4;
                pass SNG_S0[4] = SNG_S0_S4;
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
                pass QUAD_NW2 = QUAD_NW1;
                pass QUAD_NW3 = QUAD_NW2;
                pass QUAD_NE2 = QUAD_NE1;
                pass QUAD_NE3 = QUAD_NE2;
                pass QUAD_SS4_N3 = QUAD_SS4[3];
                pass QUAD_SW4_N3 = QUAD_SW4[3];

                for i in 9..17 {
                    pass LV[i] = LV[i - 1];
                }

                pass IMUX_BYP_BOUNCE_N = IMUX_BYP_BOUNCE;
            }
            connector_class TERM_S;
            connector_class TERM_S_HOLE {
                for i in 9..17 {
                    blackhole LV[i];
                }
            }
        }
        if variant virtex7 {
            connector_class PASS_S {
                pass SNG_N1 = SNG_N0;
                pass SNG_W0[4] = SNG_W0_S4;
                pass SNG_E0[4] = SNG_E0_S4;
                pass SNG_S0[4] = SNG_S0_S4;
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

                pass HEX_NN1 = HEX_NN0;
                pass HEX_NN2 = HEX_NN1;
                pass HEX_NN3 = HEX_NN2;
                pass HEX_NN4 = HEX_NN3;
                pass HEX_NN5 = HEX_NN4;
                pass HEX_NN6 = HEX_NN5;
                pass HEX_NW2 = HEX_NW1;
                pass HEX_NW3 = HEX_NW2;
                pass HEX_NW4 = HEX_NW3;
                pass HEX_NW5 = HEX_NW4;
                pass HEX_NE2 = HEX_NE1;
                pass HEX_NE3 = HEX_NE2;
                pass HEX_NE4 = HEX_NE3;
                pass HEX_NE5 = HEX_NE4;
                pass HEX_SS6_N3 = HEX_SS6[3];
                pass HEX_SW6_N3 = HEX_SW6[3];

                for i in 10..19 {
                    pass LV[i] = LV[i - 1];
                }
                for i in 7..13 {
                    pass LVB[i] = LVB[i - 1];
                }

                pass IMUX_BYP_BOUNCE_N = IMUX_BYP_BOUNCE;
            }
            connector_class TERM_S;
            connector_class TERM_S_HOLE {
                for i in 10..19 {
                    blackhole LV[i];
                }
                for i in 7..13 {
                    blackhole LVB[i];
                }
            }
            connector_class BRKH_S;
        }
    }

    connector_slot N {
        opposite S;

        if variant virtex4 {
            connector_class PASS_N {
                pass OMUX_S0 = OMUX[0];
                pass OMUX_S0_ALT = OMUX[0];
                pass OMUX_WS1 = OMUX_W1;
                pass OMUX_S2 = OMUX[2];
                pass OMUX_S3 = OMUX[3];
                pass OMUX_S4 = OMUX[4];
                pass OMUX_S5 = OMUX[5];
                pass OMUX_ES7 = OMUX_E7;
                pass DBL_E2_S = DBL_E2;
                pass DBL_S1 = DBL_S0;
                pass DBL_S2 = DBL_S1;
                pass DBL_S3 = DBL_S2;
                pass HEX_E6_S = HEX_E6;
                pass HEX_S1 = HEX_S0;
                pass HEX_S2 = HEX_S1;
                pass HEX_S3 = HEX_S2;
                pass HEX_S4 = HEX_S3;
                pass HEX_S5 = HEX_S4;
                pass HEX_S6 = HEX_S5;
                pass HEX_S7 = HEX_S6;
                for i in 13..25 {
                    pass LV[i] = LV[i - 1];
                }
            }
            connector_class TERM_N;
            connector_class BRKH_N;
            connector_class PPC_A_N;
            connector_class PPC_B_N;
        }
        if variant virtex5 {
            connector_class PASS_N, PASS_NHOLE_N {
                pass DBL_SS1 = DBL_SS0;
                pass DBL_SS2 = DBL_SS1;
                pass DBL_SW1 = DBL_SW0;
                pass DBL_SE1 = DBL_SE0;
                pass DBL_WS2 = DBL_WS1;
                pass DBL_ES2 = DBL_ES1;
                pass DBL_WW0[5] = DBL_WW0_N5;
                pass DBL_NN0[5] = DBL_NN0_N5;
                pass DBL_SS0[2] = DBL_SS0_N2;
                pass DBL_WS1_S0 = DBL_WS1_BUF0;
                pass DBL_WN2_S0 = DBL_WN2[0];

                pass PENT_SS1 = PENT_SS0;
                pass PENT_SS2 = PENT_SS1;
                pass PENT_SS3 = PENT_SS2;
                pass PENT_SS4 = PENT_SS3;
                pass PENT_SS5 = PENT_SS4;
                pass PENT_SW1 = PENT_SW0;
                pass PENT_SW2 = PENT_SW1;
                pass PENT_SW3 = PENT_SW2;
                pass PENT_SE1 = PENT_SE0;
                pass PENT_SE2 = PENT_SE1;
                pass PENT_SE3 = PENT_SE2;
                pass PENT_WS4 = PENT_WS3;
                pass PENT_WS5 = PENT_WS4;
                pass PENT_ES4 = PENT_ES3;
                pass PENT_ES5 = PENT_ES4;
                pass PENT_NN0[5] = PENT_NN0_N5;
                pass PENT_WS3_S0 = PENT_WS3_BUF0;
                pass PENT_WN5_S0 = PENT_WN5[0];

                if connector_class PASS_N {
                    for i in 0..9 {
                        pass LV[i] = LV[i + 1];
                    }
                } else {
                    for i in 0..9 {
                        blackhole LV[i];
                    }
                }

                pass IMUX_CTRL_BOUNCE_S0 = IMUX_CTRL_BOUNCE[0];
                pass IMUX_BYP_BOUNCE_S0 = IMUX_BYP_BOUNCE[0];
                pass IMUX_BYP_BOUNCE_S4 = IMUX_BYP_BOUNCE[4];
                pass IMUX_FAN_BOUNCE_S0 = IMUX_FAN_BOUNCE[0];
                pass OUT_S12_DBL = OUT[12];
                pass OUT_S18_DBL = OUT[18];
                pass OUT_S12_PENT = OUT[12];
                pass OUT_S18_PENT = OUT[18];
            }
            connector_class TERM_N_HOLE {
                for i in 0..9 {
                    blackhole LV[i];
                }
            }
            connector_class TERM_N_PPC;
        }
        if variant virtex6 {
            connector_class PASS_N {
                pass SNG_S1 = SNG_S0;
                pass SNG_W0[3] = SNG_W0_N3;
                pass SNG_E0[3] = SNG_E0_N3;
                pass SNG_N0[3] = SNG_N0_N3;
                pass SNG_W1_S4 = SNG_W1[4];
                pass SNG_E1_S0 = SNG_E1[0];
                pass SNG_N1_S0 = SNG_N1[0];

                pass DBL_SS1 = DBL_SS0;
                pass DBL_SS2 = DBL_SS1;
                pass DBL_SW1 = DBL_SW0;
                pass DBL_SE1 = DBL_SE0;
                pass DBL_NN2_S0 = DBL_NN2[0];
                pass DBL_NE2_S0 = DBL_NE2[0];
                pass DBL_NW2_S0 = DBL_NW2[0];

                pass QUAD_SS1 = QUAD_SS0;
                pass QUAD_SS2 = QUAD_SS1;
                pass QUAD_SS3 = QUAD_SS2;
                pass QUAD_SS4 = QUAD_SS3;
                pass QUAD_SW2 = QUAD_SW1;
                pass QUAD_SW3 = QUAD_SW2;
                pass QUAD_SE2 = QUAD_SE1;
                pass QUAD_SE3 = QUAD_SE2;
                pass QUAD_WW4_S0 = QUAD_WW4[0];
                pass QUAD_NN4_S0 = QUAD_NN4[0];
                pass QUAD_NW4_S0 = QUAD_NW4[0];

                for i in 0..8 {
                    pass LV[i] = LV[i + 1];
                }

                pass IMUX_FAN_BOUNCE_S = IMUX_FAN_BOUNCE;
            }
            connector_class TERM_N;
            connector_class TERM_N_HOLE {
                for i in 0..8 {
                    blackhole LV[i];
                }
            }
        }
        if variant virtex7 {
            connector_class PASS_N {
                pass SNG_S1 = SNG_S0;
                pass SNG_W0[3] = SNG_W0_N3;
                pass SNG_E0[3] = SNG_E0_N3;
                pass SNG_N0[3] = SNG_N0_N3;
                pass SNG_W1_S4 = SNG_W1[4];
                pass SNG_E1_S0 = SNG_E1[0];
                pass SNG_N1_S0 = SNG_N1[0];

                pass DBL_SS1 = DBL_SS0;
                pass DBL_SS2 = DBL_SS1;
                pass DBL_SW1 = DBL_SW0;
                pass DBL_SE1 = DBL_SE0;
                pass DBL_NN2_S0 = DBL_NN2[0];
                pass DBL_NE2_S0 = DBL_NE2[0];
                pass DBL_NW2_S0 = DBL_NW2[0];

                pass QUAD_WW4_S0 = QUAD_WW4[0];

                pass HEX_SS1 = HEX_SS0;
                pass HEX_SS2 = HEX_SS1;
                pass HEX_SS3 = HEX_SS2;
                pass HEX_SS4 = HEX_SS3;
                pass HEX_SS5 = HEX_SS4;
                pass HEX_SS6 = HEX_SS5;
                pass HEX_SW2 = HEX_SW1;
                pass HEX_SW3 = HEX_SW2;
                pass HEX_SW4 = HEX_SW3;
                pass HEX_SW5 = HEX_SW4;
                pass HEX_SE2 = HEX_SE1;
                pass HEX_SE3 = HEX_SE2;
                pass HEX_SE4 = HEX_SE3;
                pass HEX_SE5 = HEX_SE4;
                pass HEX_NN6_S0 = HEX_NN6[0];
                pass HEX_NW6_S0 = HEX_NW6[0];

                for i in 0..9 {
                    pass LV[i] = LV[i + 1];
                }
                for i in 0..6 {
                    pass LVB[i] = LVB[i + 1];
                }

                pass IMUX_FAN_BOUNCE_S = IMUX_FAN_BOUNCE;
            }
            connector_class TERM_N;
            connector_class TERM_N_HOLE {
                for i in 0..9 {
                    blackhole LV[i];
                }
                for i in 0..6 {
                    blackhole LVB[i];
                }
            }
            connector_class BRKH_N;
        }
    }

    connector_slot IO_S {
        opposite IO_N;
        if variant [virtex4, virtex5] {
            connector_class IO_S {
                if variant virtex4 {
                    pass IOCLK_N = IOCLK;
                }
                pass VRCLK_N = VRCLK;
            }
        }
    }
    connector_slot IO_N {
        opposite IO_S;
        if variant [virtex4, virtex5] {
            connector_class IO_N {
                if variant virtex4 {
                    pass IOCLK_S = IOCLK;
                }
                pass VRCLK_S = VRCLK;
            }
        }
    }

    connector_slot CLK_PREV {
        opposite CLK_NEXT;
        if variant [virtex4, virtex5] {
            connector_class CLK_PREV {
                pass IMUX_BUFG_I = IMUX_BUFG_O;
            }
        }
    }
    connector_slot CLK_NEXT {
        opposite CLK_PREV;
        if variant [virtex4, virtex5] {
            connector_class CLK_NEXT {
            }
        }
    }

    connector_slot MGT_S {
        opposite MGT_N;
        if variant virtex4 {
            connector_class MGT_S {
                pass MGT_FWDCLK_S = MGT_FWDCLK_N;
            }
        }
    }
    connector_slot MGT_N {
        opposite MGT_S;
        if variant virtex4 {
            connector_class MGT_N {
            }
        }
    }

    connector_slot CMT_PREV {
        opposite CMT_NEXT;
        if variant virtex4 {
            connector_class CMT_PREV {
                pass DCM_DCM_I = DCM_DCM_O;
            }
            connector_class CMT_PREV_CCM {
                pass DCM_DCM_I = DCM_DCM_I;
            }
        }
    }

    connector_slot CMT_NEXT {
        opposite CMT_PREV;
        if variant virtex4 {
            connector_class CMT_NEXT;
        }
    }

    connector_slot MGT_ROW_PREV {
        opposite MGT_ROW_NEXT;
        if variant virtex5 {
            connector_class MGT_ROW_PREV {
                pass MGT_ROW_I = MGT_ROW_O;
            }
            connector_class MGT_ROW_PREV_PASS {
                pass MGT_ROW_I = MGT_ROW_I;
            }
        }
    }

    connector_slot MGT_ROW_NEXT {
        opposite MGT_ROW_PREV;
        if variant virtex5 {
            connector_class MGT_ROW_NEXT;
        }
    }
}
