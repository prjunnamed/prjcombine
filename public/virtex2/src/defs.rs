use prjcombine_tablegen::target_defs;

target_defs! {
    variant virtex2;
    variant spartan3;

    enum SLICE_CYINIT { BX, CIN }
    enum SLICE_CY0F { CONST_0, CONST_1, BX, F1, F2, PROD }
    enum SLICE_CY0G { CONST_0, CONST_1, BY, G1, G2, PROD }
    enum SLICE_CYSELF { CONST_1, F }
    enum SLICE_CYSELG { CONST_1, G }
    enum SLICE_DIF_MUX { ALT, BX }
    enum SLICE_DIG_MUX { ALT, BY }
    enum SLICE_DXMUX { BX, X }
    enum SLICE_DYMUX { BY, Y }
    enum SLICE_FXMUX { F, F5, FXOR }
    // SOPOUT is virtex2 only
    enum SLICE_GYMUX { G, FX, GXOR, SOPOUT }
    enum SLICE_XBMUX { FCY, FMC15 }
    enum SLICE_YBMUX { GCY, GMC15 }
    enum SLICE_SOPEXTSEL { CONST_0, SOPIN }
    bel_class SLICE {
        input F1, F2, F3, F4;
        input G1, G2, G3, G4;
        input BX, BY;
        input CLK, SR, CE;
        output X, Y;
        output XQ, YQ;
        output XB, YB;

        attribute F, G: bitvec[16];

        // SLICEM only
        attribute DIF_MUX: SLICE_DIF_MUX;
        attribute DIG_MUX: SLICE_DIG_MUX;
        attribute F_RAM_ENABLE, G_RAM_ENABLE: bool;
        attribute F_SHIFT_ENABLE, G_SHIFT_ENABLE: bool;
        // SLICEM only
        // TODO should these have better names?
        attribute SLICEWE0USED: bool;
        // spartan3 only, and only in SLICE[0]; SLICE[1] effectively borrows SLICE[0] value
        attribute SLICEWE1USED: bool;
        // virtex2 only
        attribute BYOUTUSED: bool;

        attribute CYINIT: SLICE_CYINIT;
        attribute CY0F: SLICE_CY0F;
        attribute CY0G: SLICE_CY0G;
        attribute CYSELF: SLICE_CYSELF;
        attribute CYSELG: SLICE_CYSELG;

        attribute FFX_INIT, FFY_INIT: bitvec[1];
        attribute FFX_SRVAL, FFY_SRVAL: bitvec[1];
        attribute FF_LATCH: bool;
        attribute FF_REV_ENABLE: bool;
        attribute FF_SR_SYNC: bool;
        // SLICEM only (effectively always enabled on SLICEL)
        attribute FF_SR_ENABLE: bool;

        attribute FXMUX: SLICE_FXMUX;
        attribute GYMUX: SLICE_GYMUX;
        attribute DXMUX: SLICE_DXMUX;
        attribute DYMUX: SLICE_DYMUX;

        // SLICEM only (effectively *CY on SLICEL)
        attribute XBMUX: SLICE_XBMUX;
        attribute YBMUX: SLICE_YBMUX;

        // virtex2 only
        attribute SOPEXTSEL: SLICE_SOPEXTSEL;
    }

    bel_class TBUF {
        input I, T;
        attribute OUT_A, OUT_B: bool;
    }
    bel_class TBUS {
        output OUT;
        attribute JOINER_E: bool;
    }

    // TODO: figure out just what the fuck this is
    enum RANDOR_MODE { AND, OR }
    bel_class RANDOR_INIT {
        attribute MODE: RANDOR_MODE;
    }
    bel_class RANDOR {
        attribute MODE: RANDOR_MODE;
    }
    bel_class RANDOR_OUT {
        output O;
    }

    enum BRAM_DATA_WIDTH { _1, _2, _4, _9, _18, _36 }
    enum BRAM_WRITE_MODE { WRITE_FIRST, READ_FIRST, NO_CHANGE }
    enum BRAM_WW_VALUE { NONE, _0, _1 }
    enum BRAM_RSTTYPE { SYNC, ASYNC }
    bel_class BRAM {
        input CLKA, CLKB;
        input ENA, ENB;
        input RSTA, RSTB;
        // separate byte enables are spartan3a+ only; older devices only have [0] which controls all bits
        input WEA[4], WEB[4];
        // spartan3adsp only
        input REGCEA, REGCEB;
        input ADDRA[14], ADDRB[14];
        input DIA[32], DIB[32];
        input DIPA[4], DIPB[4];
        output DOA[32], DOB[32];
        output DOPA[4], DOPB[4];

        attribute DATA: bitvec[0x4000];
        attribute DATAP: bitvec[0x800];
        // virtex2 only
        attribute SAVEDATA: bitvec[64];
        attribute INIT_A, INIT_B: bitvec[36];
        attribute SRVAL_A, SRVAL_B: bitvec[36];
        attribute DATA_WIDTH_A, DATA_WIDTH_B: BRAM_DATA_WIDTH;
        attribute WRITE_MODE_A, WRITE_MODE_B: BRAM_WRITE_MODE;

        // spartan3+ only
        attribute WDEL_A, WDEL_B: bitvec[3];
        attribute DDEL_A, DDEL_B: bitvec[2];
        attribute WW_VALUE_A, WW_VALUE_B: BRAM_WW_VALUE;

        // spartan3a+ only
        // TODO: what *is* this really?
        attribute ENABLE_A, ENABLE_B: bool;

        // spartan3adsp only
        attribute DOA_REG, DOB_REG: bool;
        attribute RSTTYPE_A, RSTTYPE_B: BRAM_RSTTYPE;
    }
    device_data BRAM_WDEL_A, BRAM_WDEL_B: bitvec[3];
    device_data BRAM_DDEL_A, BRAM_DDEL_B: bitvec[2];

    enum MULT_B_INPUT { DIRECT, CASCADE }
    bel_class MULT {
        input A[18], B[18];
        output P[36];
        input CLK;
        input CEP, RSTP;
        // spartan3e+ only
        input CEA, RSTA;
        input CEB, RSTB;

        attribute PREG: bool;
        // spartan3e+ only
        attribute AREG: bool;
        attribute BREG: bool;
        attribute B_INPUT: MULT_B_INPUT;
        attribute PREG_CLKINVERSION: bool;
    }

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
        output P[48];

        attribute B_INPUT: MULT_B_INPUT;
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
        attribute RSTTYPE: BRAM_RSTTYPE;
    }

    enum IOI_MUX_TSBYPASS { GND, T }
    enum IOI_MUX_FFI { NONE, IBUF, PAIR_IQ1, PAIR_IQ2 }
    enum IOI_MUX_MISR_CLOCK { NONE, OTCLK1, OTCLK2 }
    enum IOI_MUX_O { NONE, O1, O2, FFO1, FFO2, FFODDR }
    enum IOI_MUX_OCE { NONE, OCE, PCI_CE }
    enum IOI_MUX_T { NONE, T1, T2, FFT1, FFT2, FFTDDR }
    enum IOI_MUX_FFO1 { O1, PAIR_FFO2 }
    enum IOI_MUX_FFO2 { O2, PAIR_FFO1 }
    bel_class IOI {
        input ICLK1, ICLK2, ICE;
        input O1, O2, T1, T2;
        input OTCLK1, OTCLK2, OCE, TCE;
        input SR, REV;
        output I, IQ1, IQ2, CLKPAD, T;
        // spartan3a only
        input S1, S2, S3;

        pad PAD: inout;

        // input path
        attribute FFI1_INIT, FFI2_INIT: bitvec[1];
        attribute FFI1_SRVAL, FFI2_SRVAL: bitvec[1];
        attribute FFI_LATCH: bool;
        attribute FFI_SR_SYNC: bool;
        attribute FFI_SR_ENABLE: bool;
        attribute FFI_REV_ENABLE: bool;
        attribute I_DELAY_ENABLE: bool;
        attribute I_TSBYPASS_ENABLE: bool;
        attribute IQ_DELAY_ENABLE: bool;
        attribute IQ_TSBYPASS_ENABLE: bool;
        attribute READBACK_I: bitvec[1];
        attribute MUX_TSBYPASS: IOI_MUX_TSBYPASS;
        // spartan3e+ only
        attribute MUX_FFI: IOI_MUX_FFI;

        // spartan3a W/E edges only (S/N have it in IOB)
        attribute DELAY_VARIABLE: bool;
        attribute DELAY_COMMON: bitvec[1];
        attribute IQ_DELAY: bitvec[2];
        attribute I_DELAY: bitvec[3];

        // output path
        attribute FFO_INIT: bitvec[1];
        attribute FFO1_SRVAL, FFO2_SRVAL: bitvec[1];
        attribute FFO1_LATCH, FFO2_LATCH: bool;
        attribute FFO_SR_SYNC: bool;
        attribute FFO_SR_ENABLE: bool;
        attribute FFO_REV_ENABLE: bool;
        attribute MUX_O: IOI_MUX_O;
        // spartan3e+ only
        attribute MUX_OCE: IOI_MUX_OCE;
        // according to data sheet, doesn't work on spartan3e?
        attribute MUX_FFO1: IOI_MUX_FFO1;
        attribute MUX_FFO2: IOI_MUX_FFO2;

        // T path
        attribute FFT_INIT: bitvec[1];
        attribute FFT1_SRVAL, FFT2_SRVAL: bitvec[1];
        attribute FFT1_LATCH, FFT2_LATCH: bool;
        attribute FFT_SR_SYNC: bool;
        attribute FFT_SR_ENABLE: bool;
        attribute FFT_REV_ENABLE: bool;
        attribute MUX_T: IOI_MUX_T;

        // spartan3e+
        attribute MUX_MISR_CLOCK: IOI_MUX_MISR_CLOCK;
        attribute MISR_ENABLE: bool;
        attribute MISR_RESET: bool;
    }

    bel_class IREG {
        input CLK, SR, REV, CE;
        output I, IQ, CLKPAD;

        pad PAD: input;

        attribute FF_INIT: bitvec[1];
        attribute FF_SRVAL: bitvec[1];
        attribute FF_LATCH: bool;
        attribute FF_SR_SYNC: bool;
        attribute FF_SR_ENABLE: bool;
        attribute FF_REV_ENABLE: bool;
        attribute O2I_ENABLE: bool;
        attribute O2IQ_ENABLE: bool;
        attribute O2I_O2IQ_ENABLE: bool;
        attribute I_DELAY_ENABLE: bool;
        attribute IQ_DELAY_ENABLE: bool;
        // why is this 14 bits? I have no meowing idea.
        attribute DELAY_ENABLE: bitvec[14];
        attribute READBACK_I: bitvec[1];
    }

    enum OREG_MUX_O { NONE, O, OQ }
    bel_class OREG {
        input O, CLK, SR, REV, CE;

        pad PAD: output;

        attribute FF_INIT: bitvec[1];
        attribute FF_SRVAL: bitvec[1];
        attribute FF_LATCH: bool;
        attribute FF_SR_SYNC: bool;
        attribute FF_SR_ENABLE: bool;
        attribute FF_REV_ENABLE: bool;
        attribute MUX_O: OREG_MUX_O;
    }

    enum IOB_PULL { NONE, PULLUP, PULLDOWN, KEEPER }
    enum IOB_IBUF_MODE {
        NONE,
        VREF,
        DIFF,

        // not present on spartan3e and up
        CMOS,

        // spartan3e only; _HV is 2.5V and up
        CMOS_LV,
        CMOS_HV,

        // spartan3a only
        CMOS_VCCINT,
        CMOS_VCCAUX,
        CMOS_VCCO,
        LOOPBACK_O,
        LOOPBACK_T,
    }
    enum IOB_SUSPEND {
        _3STATE,
        _3STATE_PULLUP,
        _3STATE_PULLDOWN,
        _3STATE_KEEPER,
        DRIVE_LAST_VALUE,
    }
    enum IOB_DCI_MODE { NONE, OUTPUT, OUTPUT_HALF, TERM_VCC, TERM_SPLIT }
    bel_class IOB {
        attribute PULL: IOB_PULL;
        attribute VREF: bool;
        // not present on spartan3e and up
        attribute VR: bool;
        // virtex2p only
        attribute BREFCLK: bool;
        // spartan3a only
        attribute PCI_CLAMP: bool;
        attribute PCI_INPUT: bool;
        attribute SUSPEND: IOB_SUSPEND;

        attribute IBUF_MODE: IOB_IBUF_MODE;
        // spartan3e and up only; note that spartan3a W/E has it in the IOI tile instead
        attribute DELAY_COMMON: bitvec[1];
        attribute IQ_DELAY: bitvec[2];
        attribute I_DELAY: bitvec[3];
        // spartan3a only
        attribute DELAY_VARIABLE: bool;

        // ??? spartan3e input-only pads only
        attribute IBUF_ENABLE: bool;

        attribute OUTPUT_ENABLE: bitvec[2];
        attribute DISABLE_GTS: bool;
        // not present on spartan3e and up
        // TODO: this field is currently 4 bits, but looks like it should be 3 bits.
        // Seems there's a bit tied to TERM_VCC mode?
        attribute DCI_MODE: IOB_DCI_MODE;
        // virtex2p, spartan3 only
        attribute DCIUPDATEMODE_ASREQUIRED: bool;
        // spartan3e and up only
        attribute OUTPUT_DIFF_GROUP: bitvec[1];

        // these come from tables

        attribute V2_PDRIVE: bitvec[5];
        attribute V2_NDRIVE: bitvec[5];
        attribute V2_SLEW: bitvec[4];
        attribute V2_OUTPUT_MISC: bitvec[1];
        attribute V2_OUTPUT_DIFF: bitvec[6];

        attribute V2P_PDRIVE: bitvec[4];
        attribute V2P_NDRIVE: bitvec[5];
        attribute V2P_SLEW: bitvec[5];
        attribute V2P_OUTPUT_MISC: bitvec[2];
        attribute V2P_OUTPUT_DIFF: bitvec[4];

        attribute S3_PDRIVE: bitvec[4];
        attribute S3_NDRIVE: bitvec[4];
        attribute S3_SLEW: bitvec[5];
        attribute S3_OUTPUT_MISC: bitvec[2];
        attribute S3_OUTPUT_DIFF: bitvec[3];

        attribute S3E_PDRIVE: bitvec[4];
        attribute S3E_NDRIVE: bitvec[4];
        attribute S3E_SLEW: bitvec[6];
        attribute S3E_OUTPUT_MISC: bitvec[1];
        attribute S3E_OUTPUT_DIFF: bitvec[2];

        attribute S3A_PDRIVE: bitvec[3];
        attribute S3A_NDRIVE: bitvec[3];
        attribute S3A_PSLEW: bitvec[4];
        attribute S3A_NSLEW: bitvec[4];
        attribute S3A_OUTPUT_DIFF: bitvec[4];
    }

    bel_class IBUF {
        attribute ENABLE: bool;
        attribute O2IPAD_ENABLE: bool;
    }

    bel_class OBUF {
        attribute ENABLE: bitvec[2];
        attribute MISR_ENABLE: bool;
    }

    bel_class DCI {
        input DCI_CLK, DCI_RESET;
        input HI_LO_P, HI_LO_N;
        output SCLK;
        output ADDRESS[3];
        output DATA;
        output N_OR_P;
        output UPDATE;
        output IOUPDATE;
        output DCI_DONE;
        attribute ENABLE: bool;
        attribute TEST_ENABLE: bool;
        attribute FORCE_DONE_HIGH: bool;

        attribute V2_PMASK_TERM_SPLIT: bitvec[5];
        attribute V2_NMASK_TERM_SPLIT: bitvec[5];
        attribute V2_PMASK_TERM_VCC: bitvec[5];
        attribute V2_LVDSBIAS: bitvec[9];

        attribute S3_PMASK_TERM_SPLIT: bitvec[4];
        attribute S3_NMASK_TERM_SPLIT: bitvec[4];
        attribute S3_PMASK_TERM_VCC: bitvec[4];
        attribute S3_LVDSBIAS: bitvec[13];

        // virtex2p and spartan3 only
        attribute QUIET: bool;
    }

    bel_class DCIRESET {
        input RST;
        attribute ENABLE: bool;
    }

    bel_class BANK {
        attribute S3E_LVDSBIAS: bitvec[11][2];
        attribute S3A_LVDSBIAS: bitvec[12][2];
    }

    table IOB_DATA {
        field V2_PDRIVE: bitvec[5];
        field V2_NDRIVE: bitvec[5];
        field V2_SLEW: bitvec[4];
        field V2_OUTPUT_MISC: bitvec[1];
        field V2_OUTPUT_DIFF: bitvec[6];
        field V2_PMASK_TERM_SPLIT: bitvec[5];
        field V2_NMASK_TERM_SPLIT: bitvec[5];
        field V2_PMASK_TERM_VCC: bitvec[5];
        field V2_LVDSBIAS: bitvec[9];

        field V2P_PDRIVE: bitvec[4];
        field V2P_NDRIVE: bitvec[5];
        field V2P_SLEW: bitvec[5];
        field V2P_OUTPUT_MISC: bitvec[2];
        field V2P_OUTPUT_DIFF: bitvec[4];
        field V2P_PMASK_TERM_SPLIT: bitvec[5];
        field V2P_NMASK_TERM_SPLIT: bitvec[5];
        field V2P_PMASK_TERM_VCC: bitvec[5];
        field V2P_LVDSBIAS: bitvec[9];

        field S3_PDRIVE: bitvec[4];
        field S3_NDRIVE: bitvec[4];
        field S3_SLEW: bitvec[5];
        field S3_OUTPUT_MISC: bitvec[2];
        field S3_OUTPUT_DIFF: bitvec[3];
        field S3_PMASK_TERM_SPLIT: bitvec[4];
        field S3_NMASK_TERM_SPLIT: bitvec[4];
        field S3_PMASK_TERM_VCC: bitvec[4];
        field S3_LVDSBIAS: bitvec[13];

        field S3E_PDRIVE: bitvec[4];
        field S3E_NDRIVE: bitvec[4];
        field S3E_SLEW: bitvec[6];
        field S3E_OUTPUT_MISC: bitvec[1];
        field S3E_OUTPUT_DIFF: bitvec[2];
        field S3E_LVDSBIAS: bitvec[11];

        field S3A_WE_PDRIVE: bitvec[3];
        field S3A_WE_NDRIVE: bitvec[3];
        field S3A_SN_PDRIVE: bitvec[3];
        field S3A_SN_NDRIVE: bitvec[3];
        field S3A_PSLEW: bitvec[4];
        field S3A_2V5_NSLEW: bitvec[4];
        field S3A_3V3_NSLEW: bitvec[4];
        field S3A_OUTPUT_DIFF: bitvec[4];
        field S3A_LVDSBIAS: bitvec[12];

        // specials
        row OFF;
        row SLEW_SLOW_3V3;
        row SLEW_SLOW_LV;
        row SLEW_FAST;
        row SLEW_QUIETIO;
        row VR;
        row DIFF_TERM;

        // push-pull I/O standards
        row LVCMOS12, LVCMOS12_2, LVCMOS12_4, LVCMOS12_6;
        row LVCMOS15, LVCMOS15_2, LVCMOS15_4, LVCMOS15_6, LVCMOS15_8, LVCMOS15_12, LVCMOS15_16;
        row LVCMOS18, LVCMOS18_2, LVCMOS18_4, LVCMOS18_6, LVCMOS18_8, LVCMOS18_12, LVCMOS18_16;
        row LVCMOS25, LVCMOS25_2, LVCMOS25_4, LVCMOS25_6, LVCMOS25_8, LVCMOS25_12, LVCMOS25_16, LVCMOS25_24;
        row LVCMOS33, LVCMOS33_2, LVCMOS33_4, LVCMOS33_6, LVCMOS33_8, LVCMOS33_12, LVCMOS33_16, LVCMOS33_24;
        row LVTTL, LVTTL_2, LVTTL_4, LVTTL_6, LVTTL_8, LVTTL_12, LVTTL_16, LVTTL_24;
        row PCI33_3, PCI66_3, PCIX;

        // DCI output
        row LVDCI_15, LVDCI_18, LVDCI_25, LVDCI_33;
        row LVDCI_DV2_15, LVDCI_DV2_18, LVDCI_DV2_25, LVDCI_DV2_33;
        // VREF-based with DCI output
        row HSLVDCI_15, HSLVDCI_18, HSLVDCI_25, HSLVDCI_33;

        // VREF-based
        row GTL, GTLP, AGP;
        row SSTL18_I, SSTL18_II;
        row SSTL2_I, SSTL2_II;
        row SSTL3_I, SSTL3_II;
        row HSTL_I, HSTL_II, HSTL_III, HSTL_IV;
        row HSTL_I_18, HSTL_II_18, HSTL_III_18, HSTL_IV_18;
        // with DCI
        row GTL_DCI, GTLP_DCI;
        row SSTL18_I_DCI, SSTL18_II_DCI;
        row SSTL2_I_DCI, SSTL2_II_DCI;
        row SSTL3_I_DCI, SSTL3_II_DCI;
        row HSTL_I_DCI, HSTL_II_DCI, HSTL_III_DCI, HSTL_IV_DCI;
        row HSTL_I_DCI_18, HSTL_II_DCI_18, HSTL_III_DCI_18, HSTL_IV_DCI_18;

        // pseudo-differential
        row DIFF_SSTL18_I, DIFF_SSTL18_II;
        row DIFF_SSTL2_I, DIFF_SSTL2_II;
        row DIFF_SSTL3_I, DIFF_SSTL3_II;
        row DIFF_HSTL_I, DIFF_HSTL_II, DIFF_HSTL_III;
        row DIFF_HSTL_I_18, DIFF_HSTL_II_18, DIFF_HSTL_III_18;
        row LVPECL_25, LVPECL_33;
        row BLVDS_25;
        // with DCI
        row DIFF_SSTL18_II_DCI;
        row DIFF_SSTL2_II_DCI;
        row DIFF_HSTL_II_DCI;
        row DIFF_HSTL_II_DCI_18;

        // true differential
        row LVDS_25, LVDS_33;
        row LVDSEXT_25, LVDSEXT_33;
        row MINI_LVDS_25, MINI_LVDS_33;
        row HT_25;
        row RSDS_25, RSDS_33;
        row PPDS_25, PPDS_33;
        row TMDS_33;
        // with DCI
        row LVDS_25_DCI, LVDS_33_DCI;
        row LVDSEXT_25_DCI, LVDSEXT_33_DCI;
    }

    table IOB_I_DELAY {
        field DELAY_WSN: bitvec[4];
        field DELAY_E: bitvec[4];

        row DLY1;
        row DLY2;
        row DLY3;
        row DLY4;
        row DLY5;
        row DLY6;
        row DLY7;
        row DLY8;
        row DLY9;
        row DLY10;
        row DLY11;
        row DLY12;
        row DLY13;
    }

    table IOB_IQ_DELAY {
        field DELAY_WSN: bitvec[3];
        field DELAY_E: bitvec[3];

        row DLY1;
        row DLY2;
        row DLY3;
        row DLY4;
        row DLY5;
        row DLY6;
        row DLY7;
    }

    enum DCM_CLKDV_MODE { HALF, INT }
    enum DCM_FREQUENCY_MODE { LOW, HIGH }
    enum DCM_DSS_MODE { SPREAD_2, SPREAD_4, SPREAD_6, SPREAD_8 }
    enum DCM_PS_MODE { CLKIN, CLKFB }
    enum DCM_TEST_OSC { _90, _180, _270, _360 }
    bel_class DCM {
        input CLKIN, CLKFB, RST;
        input PSCLK, PSEN, PSINCDEC;
        input STSADRS[5];
        input FREEZEDLL, FREEZEDFS, DSSEN;
        input CTLMODE, CTLGO, CTLOSC1, CTLOSC2, CTLSEL[3];
        output CLK0, CLK90, CLK180, CLK270;
        output CLK2X, CLK2X180, CLKDV;
        output CLKFX, CLKFX180, CONCUR;
        output LOCKED, PSDONE;
        output STATUS[8];

        // common between V2 and S3E

        attribute OUT_CLK0_ENABLE: bool;
        attribute OUT_CLK90_ENABLE: bool;
        attribute OUT_CLK180_ENABLE: bool;
        attribute OUT_CLK270_ENABLE: bool;
        attribute OUT_CLK2X_ENABLE: bool;
        attribute OUT_CLK2X180_ENABLE: bool;
        attribute OUT_CLKDV_ENABLE: bool;
        attribute OUT_CLKFX_ENABLE: bool;
        // not present on s3e (uses OUT_CLKFX_ENABLE for both instead)
        attribute OUT_CLKFX180_ENABLE: bool;
        attribute OUT_CONCUR_ENABLE: bool;

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

        attribute DLL_ENABLE: bool;
        attribute DLL_FREQUENCY_MODE: DCM_FREQUENCY_MODE;

        attribute DFS_ENABLE: bool;
        attribute DFS_FEEDBACK: bool;
        attribute DFS_FREQUENCY_MODE: DCM_FREQUENCY_MODE;

        attribute PHASE_SHIFT: bitvec[8];
        attribute PHASE_SHIFT_NEGATIVE: bool;
        attribute PS_ENABLE: bool;

        attribute STARTUP_WAIT: bool;

        // V2 only

        attribute V2_REG_COM: bitvec[32];
        attribute V2_REG_DFS: bitvec[32];
        attribute V2_REG_DLLC: bitvec[32];
        attribute V2_REG_DLLS: bitvec[32];
        attribute V2_REG_MISC: bitvec[32];
        attribute S3_REG_MISC: bitvec[12];

        attribute V2_CLKFX_MULTIPLY: bitvec[12];
        attribute V2_CLKFX_DIVIDE: bitvec[12];
        attribute V2_DUTY_CYCLE_CORRECTION: bitvec[4];

        attribute DSS_ENABLE: bool;
        attribute DSS_MODE: DCM_DSS_MODE;
        attribute CLKFB_ENABLE: bool;
        attribute STATUS1_ENABLE: bool;
        attribute STATUS7_ENABLE: bool;

        attribute PL_CENTERED: bool;
        attribute PS_CENTERED: bool;
        attribute PS_MODE: DCM_PS_MODE;
        attribute SEL_PL_DLY: bitvec[2];

        attribute COIN_WINDOW: bitvec[2];
        attribute NON_STOP: bool;
        attribute V2_EN_DUMMY_OSC: bitvec[3];
        attribute EN_DUMMY_OSC_OR_NON_STOP: bool;
        attribute EN_OSC_COARSE: bool;
        attribute FACTORY_JF1: bitvec[8];
        attribute FACTORY_JF2: bitvec[8];
        attribute TEST_ENABLE: bool;
        attribute TEST_OSC: DCM_TEST_OSC;
        attribute ZD2_BY1: bool;

        attribute V2_VBG_SEL: bitvec[3];
        attribute V2_VBG_PD: bitvec[2];

        // virtex2p and spartan3 only
        attribute ZD1_BY1: bool;
        attribute RESET_PS_SEL: bool;

        // spartan3 only
        attribute CFG_DLL_LP: bitvec[3];
        attribute CFG_DLL_PS: bitvec[9];
        attribute S3_EN_DUMMY_OSC: bool;
        attribute EN_OLD_OSCCTL: bool;
        attribute EN_PWCTL: bool;
        attribute EN_RELRST_B: bool;
        attribute EXTENDED_FLUSH_TIME: bool;
        attribute EXTENDED_HALT_TIME: bool;
        attribute EXTENDED_RUN_TIME: bool;
        attribute INVERT_ZD1_CUSTOM: bool;
        attribute LPON_B_DFS: bitvec[2];
        attribute M1D1: bool;
        attribute MIS1: bool;
        attribute SEL_HSYNC_B: bitvec[2];
        attribute SPLY_IDC: bitvec[2];
        attribute TRIM_LP_B: bool;
        attribute VREG_PROBE: bitvec[5];

        // S3E only

        attribute PS_VARIABLE: bool;

        attribute S3E_REG_DFS_C: bitvec[3];
        attribute S3E_REG_DFS_S: bitvec[76];
        attribute S3E_REG_DLL_C: bitvec[32];
        attribute S3E_REG_DLL_S: bitvec[32];
        attribute S3E_REG_INTERFACE: bitvec[16];
        attribute S3E_REG_VREG: bitvec[20];

        attribute S3E_CLKFX_MULTIPLY: bitvec[8];
        attribute S3E_CLKFX_DIVIDE: bitvec[8];
        attribute S3E_DUTY_CYCLE_CORRECTION: bool;

        attribute S3E_VBG_SEL: bitvec[4];

        attribute UNK_PERIOD_LF: bitvec[2];
        attribute UNK_PERIOD_NOT_HF: bitvec[1];
    }
    device_data DCM_DESKEW_ADJUST: bitvec[4];
    device_data DCM_V2_VBG_SEL: bitvec[3];
    device_data DCM_V2_VBG_PD: bitvec[2];

    bel_class BUFGMUX {
        input I0, I1;
        input S;
        output O;

        attribute INIT_OUT: bitvec[1];
    }

    bel_class GLOBALSIG_BUFG {
        attribute GWE_ENABLE: bool;
    }

    bel_class GLOBALSIG_HCLK_V2 {
        attribute GWE_GHIGH_S_ENABLE: bool;
        attribute GWE_GHIGH_N_ENABLE: bool;
        attribute GSR_S_ENABLE: bool;
        attribute GSR_N_ENABLE: bool;
    }

    bel_class GLOBALSIG_HCLK_S3 {
        attribute ENABLE: bool;
    }

    bel_class PCILOGIC {
        input FI[4];
        input SI[10];
        output OUT[6];
    }

    enum PCILOGICSE_DELAY { NILL, LOW, MED, HIGH }
    bel_class PCILOGICSE {
        input I1, I2, I3;

        attribute ENABLE: bool;
        attribute DELAY: PCILOGICSE_DELAY;
    }
    device_data PCILOGICSE_DELAY: PCILOGICSE_DELAY;

    bel_class STARTUP {
        input CLK;
        input GSR, GTS;
        // spartan3e only
        input MBT;

        attribute USER_GTS_GSR_ENABLE: bool;
        attribute GTS_SYNC: bool;
        attribute GSR_SYNC: bool;
        // virtex2 only (and probably doesn't really exist)
        attribute GWE_SYNC: bool;
    }

    bel_class CAPTURE {
        input CLK;
        input CAP;
    }

    bel_class ICAP {
        input CLK, CE, WRITE;
        input I[8];
        output BUSY;
        output O[8];

        attribute ENABLE: bool;
    }

    bel_class SPI_ACCESS {
        input CLK, CSB, MOSI;
        output MISO;

        attribute ENABLE: bool;
    }

    bel_class PMV {
        input A[6];
        input EN;
        output O;
    }

    bel_class DNA_PORT {
        input CLK, DIN, READ, SHIFT;
        output DOUT;
    }

    bel_class BSCAN {
        input TDO1, TDO2;
        output DRCK1, DRCK2;
        output SEL1, SEL2;
        output TDI;
        output RESET, CAPTURE, SHIFT, UPDATE;
        // spartan3a only
        output TCK, TMS;

        // presumably one bit per TDO, but unknown which is which
        attribute USER_TDO_ENABLE: bitvec[2];
        attribute USERCODE: bitvec[32];
    }

    bel_class JTAGPPC {
        input TDOPPC, TDOTSPPC;
        output TCK, TMS, TDIPPC;
        attribute ENABLE: bool;
    }

    enum MISC_TEMP_SENSOR { NONE, THERM, PGATE, BG, CGATE }
    bel_class MISC_SW {
        // not present on spartan3e and up (shared I/O instead)
        pad M0, M1, M2: input;
        // spartan3a only
        pad CCLK2: output;
        pad MOSI2: output;

        attribute M0_PULL: IOB_PULL;
        attribute M1_PULL: IOB_PULL;
        attribute M2_PULL: IOB_PULL;
        attribute CCLK2_PULL: IOB_PULL;
        attribute MOSI2_PULL: IOB_PULL;
        attribute DCI_CLK_ENABLE: bool;
        // virtex2 (not virtex2p) only
        attribute DCI_ALTVR: bool;

        // virtex2 misc
        attribute BCLK_N_DIV2: bitvec[5];
        attribute ZCLK_N_DIV2: bitvec[5];
        attribute DISABLE_BANDGAP: bool;
        attribute DISABLE_VGG_GENERATION: bool;
        attribute RAISE_VGG: bitvec[2];

        // spartan3 misc
        attribute DCI_OSC_SEL: bitvec[3];
        attribute GATE_GHIGH: bool;
        attribute SEND_VGG: bitvec[4];
        attribute VGG_ENABLE_OFFCHIP: bool;
        attribute VGG_SENDMAX: bool;
        // spartan3e and up only
        attribute TEMP_SENSOR: MISC_TEMP_SENSOR;
        // spartan3a only
        attribute UNK_ALWAYS_SET: bitvec[4];
    }

    bel_class MISC_SE {
        pad CCLK, DONE: inout;
        // virtex2 only
        pad POWERDOWN_B: input;
        // spartan3a only
        pad SUSPEND: input;

        attribute CCLK_PULL: IOB_PULL;
        attribute DONE_PULL: IOB_PULL;
        attribute POWERDOWN_PULL: IOB_PULL;

        // fpgacore only
        attribute ABUFF: bitvec[4];
    }

    bel_class MISC_NW {
        // HSWAPEN not present on spartan3e and up (shared I/O instead)
        pad HSWAPEN: input;
        pad PROG_B: input;
        pad TDI: input;
        // spartan3a only (is at MISC_NE otherwise)
        pad TMS: input;

        attribute HSWAPEN_PULL: IOB_PULL;
        attribute PROG_PULL: IOB_PULL;
        attribute TDI_PULL: IOB_PULL;
        attribute TMS_PULL: IOB_PULL;
        attribute TEST_LL: bool;
    }

    bel_class MISC_NE {
        // on spartan3a, TMS is at MISC_NW instead
        pad TCK, TMS: input;
        pad TDO: output;

        // spartan3a only
        pad CSO2: output;
        pad MISO2: input;

        attribute TCK_PULL: IOB_PULL;
        attribute TMS_PULL: IOB_PULL;
        attribute TDO_PULL: IOB_PULL;
        attribute CSO2_PULL: IOB_PULL;
        attribute MISO2_PULL: IOB_PULL;
        // virtex2 only
        attribute TEST_LL: bool;
    }

    bel_class MISC_CNR_S3 {
        attribute MUX_DCI_TEST: bitvec[1];
        attribute DCM_ENABLE: bool;
    }

    bel_class MISR_FC {
        input CLK;
        attribute MISR_CLOCK: bool;
        attribute MISR_RESET: bool;
    }

    enum STARTUP_CYCLE { _0, _1, _2, _3, _4, _5, _6, DONE, KEEP, NOWAIT }
    enum STARTUP_CLOCK { CCLK, USERCLK, JTAGCLK }
    enum CONFIG_RATE_V2 { _4, _5, _7, _8, _9, _10, _13, _15, _20, _26, _30, _34, _41, _51, _55, _60, _130 }
    enum CONFIG_RATE_S3 { _3, _6, _12, _25, _50, _100 }
    enum CONFIG_RATE_S3E { _1, _3, _6, _12, _25, _50 }
    enum CONFIG_RATE_S3A { _6, _1, _3, _7, _8, _10, _12, _13, _17, _22, _25, _27, _33, _44, _50, _100 }
    enum BUSCLK_FREQ { _25, _50, _100, _200 }
    enum S3_VRDSEL { _80, _90, _95, _100 }
    enum S3E_VRDSEL { _70, _80, _90 }
    enum SECURITY { NONE, LEVEL1, LEVEL2, LEVEL3 }
    enum SW_CLK { INTERNALCLK, STARTUPCLK }
    bel_class GLOBAL {
        // COR
        attribute GWE_CYCLE: STARTUP_CYCLE;
        attribute GTS_CYCLE: STARTUP_CYCLE;
        attribute LOCK_CYCLE: STARTUP_CYCLE;
        attribute MATCH_CYCLE: STARTUP_CYCLE;
        attribute DONE_CYCLE: STARTUP_CYCLE;
        attribute STARTUP_CLOCK: STARTUP_CLOCK;
        attribute CONFIG_RATE_V2: CONFIG_RATE_V2;
        attribute CONFIG_RATE_S3: CONFIG_RATE_S3;
        attribute CONFIG_RATE_S3E: CONFIG_RATE_S3E;
        attribute CONFIG_RATE_S3A: CONFIG_RATE_S3A;
        attribute CAPTURE_ONESHOT: bool;
        attribute DRIVE_DONE: bool;
        attribute DONE_PIPE: bool;
        attribute DCM_SHUTDOWN: bool;
        attribute POWERDOWN_STATUS: bool;
        attribute CRC_ENABLE: bool;
        // spartan3 and up only
        attribute BUSCLK_FREQ: BUSCLK_FREQ;
        attribute S3_VRDSEL: S3_VRDSEL;
        attribute S3E_VRDSEL: S3E_VRDSEL;
        // spartan3e only
        attribute MULTIBOOT_ENABLE: bool;

        // CTL
        attribute GTS_USR_B: bool;
        attribute VGG_TEST: bool;
        attribute BCLK_TEST: bool;
        attribute SECURITY: SECURITY;
        attribute PERSIST: bool;
        // spartan3a only
        attribute ICAP_ENABLE: bool;

        // spartan3a and up only
        attribute S3A_VRDSEL: bitvec[3];
        attribute SEND_VGG: bitvec[4];
        attribute VGG_ENABLE_OFFCHIP: bool;
        attribute VGG_SENDMAX: bool;
        attribute DRIVE_AWAKE: bool;
        attribute BPI_DIV8: bool;
        attribute ICAP_BYPASS: bool;
        attribute RESET_ON_ERR: bool;
        // CONFIG_RATE = 400 / (CONFIG_RATE_DIV + 1)
        attribute CONFIG_RATE_DIV: bitvec[10];
        attribute CCLK_DLY: bitvec[2];
        attribute CCLK_SEP: bitvec[2];
        attribute CLK_SWITCH_OPT: bitvec[2];

        // HC_OPT
        attribute HC_CYCLE: bitvec[4];
        attribute TWO_ROUND: bool;
        attribute BRAM_SKIP: bool;

        // POWERDOWN
        attribute SW_CLK: SW_CLK;
        attribute EN_PORB: bool;
        attribute EN_SUSPEND: bool;
        attribute EN_SW_GSR: bool;
        attribute SUSPEND_FILTER: bool;
        attribute WAKE_DELAY1: bitvec[3];
        attribute WAKE_DELAY2: bitvec[5];
        attribute SW_GWE_CYCLE: bitvec[10];
        attribute SW_GTS_CYCLE: bitvec[10];

        // MODE
        attribute BOOTVSEL: bitvec[3];
        attribute NEXT_CONFIG_BOOT_MODE: bitvec[3];
        attribute NEXT_CONFIG_NEW_MODE: bool;
        attribute TESTMODE_EN: bool;

        attribute NEXT_CONFIG_ADDR: bitvec[32];

        // SEU_OPT
        attribute POST_CRC_EN: bool;
        attribute GLUTMASK: bool;
        attribute POST_CRC_KEEP: bool;
        // FREQ = 400 / (POST_CRC_FREQ + 1)
        attribute POST_CRC_FREQ_DIV: bitvec[10];
    }

    device_data IDCODE: bitvec[32];
    device_data DOUBLE_GRESTORE: bool;
    device_data FREEZE_DCI_NOPS: u32;

    enum GT_DATA_WIDTH { _1, _2, _4 }
    enum GT_SEQ_LEN { _1, _2, _3, _4 }
    enum GT_CHAN_BOND_MODE { NONE, MASTER, SLAVE_1_HOP, SLAVE_2_HOPS }
    enum GT_CRC_FORMAT { USER_MODE, ETHERNET, INFINIBAND, FIBRE_CHAN }
    enum GT_RX_LOS_INVALID_INCR { _1, _2, _4, _8, _16, _32, _64, _128 }
    enum GT_RX_LOS_THRESHOLD { _4, _8, _16, _32, _64, _128, _256, _512 }
    enum GT_TERMINATION_IMP { _50, _75 }
    enum GT_TX_DIFF_CTRL { _400, _500, _600, _700, _800 }
    bel_class GT {
        input REFCLK;
        input REFCLK2;
        input REFCLKSEL;
        nonroutable input BREFCLK, BREFCLK2;

        input POWERDOWN;
        input LOOPBACK[2];

        input RXUSRCLK;
        input RXUSRCLK2;
        output RXRECCLK;
        input RXRESET;
        input RXPOLARITY;
        output RXDATA[32];
        output RXNOTINTABLE[4];
        output RXDISPERR[4];
        output RXCHARISK[4];
        output RXCHARISCOMMA[4];
        output RXRUNDISP[4];
        output RXCOMMADET;
        output RXREALIGN;
        input ENPCOMMAALIGN;
        input ENMCOMMAALIGN;
        output RXLOSSOFSYNC[2];
        output RXCLKCORCNT[3];
        output RXBUFSTATUS[2];
        output RXCHECKINGCRC;
        output RXCRCERR;

        input TXUSRCLK;
        input TXUSRCLK2;
        input TXRESET;
        input TXPOLARITY;
        input TXINHIBIT;
        input TXDATA[32];
        input TXBYPASS8B10B[4];
        input TXCHARISK[4];
        input TXCHARDISPMODE[4];
        input TXCHARDISPVAL[4];
        input TXFORCECRCERR;
        output TXKERR[4];
        output TXRUNDISP[4];
        output TXBUFERR;

        input CONFIGENABLE;
        input CONFIGIN;
        output CONFIGOUT;

        input ENCHANSYNC;
        input CHBONDI[4];
        output CHBONDO[4];
        output CHBONDDONE;

        pad RXP, RXN: input;
        pad TXP, TXN: output;
        pad GNDA: power;
        pad AVCCAUXRX, AVCCAUXTX: power;
        pad VTRX, VTTX: power;

        attribute ENABLE: bool;
        attribute REF_CLK_V_SEL: bitvec[1];
        attribute SERDES_10B: bool;
        attribute TERMINATION_IMP: GT_TERMINATION_IMP;

        attribute ALIGN_COMMA_MSB: bool;
        attribute PCOMMA_DETECT: bool;
        attribute MCOMMA_DETECT: bool;
        attribute COMMA_10B_MASK: bitvec[10];
        attribute PCOMMA_10B_VALUE: bitvec[10];
        attribute MCOMMA_10B_VALUE: bitvec[10];
        attribute DEC_PCOMMA_DETECT: bool;
        attribute DEC_MCOMMA_DETECT: bool;
        attribute DEC_VALID_COMMA_ONLY: bool;

        attribute RX_DATA_WIDTH: GT_DATA_WIDTH;
        attribute RX_BUFFER_USE: bool;
        attribute RX_BUFFER_LIMIT: bitvec[4];
        attribute RX_DECODE_USE: bool;
        attribute RX_CRC_USE: bool;
        attribute RX_LOS_INVALID_INCR: GT_RX_LOS_INVALID_INCR;
        attribute RX_LOS_THRESHOLD: GT_RX_LOS_THRESHOLD;
        attribute RX_LOSS_OF_SYNC_FSM: bool;

        attribute TX_DATA_WIDTH: GT_DATA_WIDTH;
        attribute TX_BUFFER_USE: bool;
        attribute TX_CRC_USE: bool;
        attribute TX_CRC_FORCE_VALUE: bitvec[8];
        attribute TX_DIFF_CTRL: GT_TX_DIFF_CTRL;
        attribute TX_PREEMPHASIS: bitvec[2];

        attribute CRC_FORMAT: GT_CRC_FORMAT;
        attribute CRC_START_OF_PKT: bitvec[8];
        attribute CRC_END_OF_PKT: bitvec[8];

        attribute CLK_CORRECT_USE: bool;
        attribute CLK_COR_INSERT_IDLE_FLAG: bool;
        attribute CLK_COR_KEEP_IDLE: bool;
        attribute CLK_COR_REPEAT_WAIT: bitvec[5];
        attribute CLK_COR_SEQ_LEN: GT_SEQ_LEN;
        attribute CLK_COR_SEQ_2_USE: bool;
        for i in 1..=4 {
            attribute "CLK_COR_SEQ_1_{i}": bitvec[11];
        }
        for i in 1..=4 {
            attribute "CLK_COR_SEQ_2_{i}": bitvec[11];
        }

        attribute CHAN_BOND_MODE: GT_CHAN_BOND_MODE;
        attribute CHAN_BOND_WAIT: bitvec[4];
        attribute CHAN_BOND_OFFSET: bitvec[4];
        attribute CHAN_BOND_LIMIT: bitvec[5];
        attribute CHAN_BOND_ONE_SHOT: bool;
        attribute CHAN_BOND_SEQ_LEN: GT_SEQ_LEN;
        attribute CHAN_BOND_SEQ_2_USE: bool;
        for i in 1..=4 {
            attribute "CHAN_BOND_SEQ_1_{i}": bitvec[11];
        }
        for i in 1..=4 {
            attribute "CHAN_BOND_SEQ_2_{i}": bitvec[11];
        }

        attribute TEST_MODE_1: bool;
        attribute TEST_MODE_2: bool;
        attribute TEST_MODE_3: bool;
        attribute TEST_MODE_4: bool;
        attribute TEST_MODE_5: bool;
        attribute TEST_MODE_6: bool;
    }

    enum GT10_ALIGN_COMMA_WORD { _1, _2, _4 }
    enum GT10_SEQ_LEN { _1, _2, _3, _4, _8 }
    bel_class GT10 {
        input REFCLK;
        input REFCLK2;
        input REFCLKBSEL;
        input REFCLKSEL;
        nonroutable input BREFCLKPIN, BREFCLKNIN;

        input POWERDOWN;
        input LOOPBACK[2];

        input RXUSRCLK;
        input RXUSRCLK2;
        output RXRECCLK;
        input RXRESET;
        input PMARXLOCKSEL[2];
        output PMARXLOCK;
        input RXPOLARITY;
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
        output RXCLKCORCNT[3];
        output RXBUFSTATUS[2];
        output RXCHECKINGCRC;
        output RXCRCERR;

        input TXUSRCLK;
        input TXUSRCLK2;
        output TXOUTCLK;
        input TXRESET;
        input TXPOLARITY;
        input TXINHIBIT;
        input TXDATAWIDTH[2];
        input TXINTDATAWIDTH[2];
        input TXDATA[64];
        input TXBYPASS8B10B[8];
        input TXCHARISK[8];
        input TXCHARDISPMODE[8];
        input TXCHARDISPVAL[8];
        input TXFORCECRCERR;
        input TXENC8B10BUSE;
        input TXENC64B66BUSE;
        input TXSCRAM64B66BUSE;
        input TXGEARBOX64B66BUSE;
        output TXKERR[8];
        output TXRUNDISP[8];
        output TXBUFERR;

        input ENCHANSYNC;
        input CHBONDI[5];
        output CHBONDO[5];
        output CHBONDDONE;

        input PMAINIT;
        input PMAREGADDR[6];
        input PMAREGDATAIN[8];
        input PMAREGRW;
        input PMAREGSTROBE;

        input SCANEN;
        input SCANMODE;
        input SCANIN;
        output SCANOUT;
        input TESTMEMORY;

        pad RXP, RXN: input;
        pad TXP, TXN: output;
        pad GNDA: power;
        pad AVCCAUXRX, AVCCAUXTX: power;
        pad VTRX, VTTX: power;

        attribute PMA_REG: bitvec[8][16];

        // the following are contained within PMA_REG
        attribute MASTERBIAS: bitvec[2];
        attribute VCODAC: bitvec[6];
        attribute TXDIVRATIO: bitvec[10];
        attribute TXBUSWID: bitvec[1];
        attribute ENDCD: bitvec[1];
        attribute SEL_DAC_TRAN: bitvec[4];
        attribute SEL_DAC_FIX: bitvec[4];
        attribute TXLOOPFILTERC: bitvec[2];
        attribute TXLOOPFILTERR: bitvec[2];
        attribute IBOOST: bitvec[1];
        attribute TXCPI: bitvec[1];
        attribute TXVCODAC: bitvec[1];
        attribute TXVCOGAIN: bitvec[1];
        attribute TXVSEL: bitvec[2];
        attribute TXREG: bitvec[2];
        attribute TXDOWNLEVEL: bitvec[4];
        attribute PRDRVOFF: bitvec[1];
        attribute EMPOFF: bitvec[1];
        attribute SLEW: bitvec[1];
        attribute TXEMPHLEVEL: bitvec[4];
        attribute TXDIGSW: bitvec[1];
        attribute TXANASW: bitvec[1];
        attribute RXDIVRATIO: bitvec[14];
        attribute RXLOOPFILTERC: bitvec[2];
        attribute RXLOOPFILTERR: bitvec[3];
        attribute AFE_FLAT_ENABLE: bitvec[1];
        attribute RXVCOSW: bitvec[1];
        attribute RXCPI: bitvec[2];
        attribute RXVCODAC: bitvec[1];
        attribute RXVCOGAIN: bitvec[1];
        attribute RXVSEL: bitvec[2];
        attribute RXREG: bitvec[2];
        attribute RXFLTCPT: bitvec[5];
        attribute RXVSELCP: bitvec[2];
        attribute VSELAFE: bitvec[2];
        attribute RXFEI: bitvec[2];
        attribute RXFLCPI: bitvec[2];
        attribute RXFER: bitvec[10];
        attribute PMA_REG_0E: bitvec[8];
        attribute BIASEN: bool;
        attribute TXANAEN: bool;
        attribute TXDIGEN: bool;
        attribute RXANAEN: bool;
        attribute PMA_PWR_CNTRL_BIT4: bool;
        attribute TXEN: bool;
        attribute RXEN: bool;
        attribute TXDRVEN: bool;

        attribute RX_BUFFER_USE: bool;
        attribute RX_CRC_USE: bool;
        attribute RX_LOS_INVALID_INCR: GT_RX_LOS_INVALID_INCR;
        attribute RX_LOS_THRESHOLD: GT_RX_LOS_THRESHOLD;
        attribute RX_LOSS_OF_SYNC_FSM: bool;

        attribute TX_BUFFER_USE: bool;
        attribute TX_CRC_FORCE_VALUE: bitvec[8];
        attribute TX_CRC_USE: bool;

        attribute ALIGN_COMMA_WORD: GT10_ALIGN_COMMA_WORD;
        attribute PCOMMA_DETECT: bool;
        attribute MCOMMA_DETECT: bool;
        attribute COMMA_10B_MASK: bitvec[10];
        attribute PCOMMA_10B_VALUE: bitvec[10];
        attribute MCOMMA_10B_VALUE: bitvec[10];
        attribute DEC_PCOMMA_DETECT: bool;
        attribute DEC_MCOMMA_DETECT: bool;
        attribute DEC_VALID_COMMA_ONLY: bool;

        attribute SH_CNT_MAX: bitvec[8];
        attribute SH_INVALID_CNT_MAX: bitvec[8];

        attribute CRC_FORMAT: GT_CRC_FORMAT;
        attribute CRC_START_OF_PKT: bitvec[8];
        attribute CRC_END_OF_PKT: bitvec[8];

        attribute CLK_CORRECT_USE: bool;
        attribute CLK_COR_8B10B_DE: bool;
        attribute CLK_COR_INSERT_IDLE_FLAG: bool;
        attribute CLK_COR_KEEP_IDLE: bool;
        attribute CLK_COR_REPEAT_WAIT: bitvec[5];
        attribute CLK_COR_ADJ_MAX: bitvec[5];
        attribute CLK_COR_MIN_LAT: bitvec[6];
        attribute CLK_COR_MAX_LAT: bitvec[6];
        attribute CLK_COR_SEQ_LEN: GT10_SEQ_LEN;
        attribute CLK_COR_SEQ_2_USE: bool;
        attribute CLK_COR_SEQ_DROP: bool;
        attribute CLK_COR_SEQ_1_MASK: bitvec[4];
        attribute CLK_COR_SEQ_2_MASK: bitvec[4];
        for i in 1..=4 {
            attribute "CLK_COR_SEQ_1_{i}": bitvec[11];
        }
        for i in 1..=4 {
            attribute "CLK_COR_SEQ_2_{i}": bitvec[11];
        }

        attribute CHAN_BOND_MODE: GT_CHAN_BOND_MODE;
        attribute CHAN_BOND_64B66B_SV: bool;
        attribute CHAN_BOND_LIMIT: bitvec[5];
        attribute CHAN_BOND_ONE_SHOT: bool;
        attribute CHAN_BOND_SEQ_LEN: GT10_SEQ_LEN;
        attribute CHAN_BOND_SEQ_2_USE: bool;
        attribute CHAN_BOND_SEQ_1_MASK: bitvec[4];
        attribute CHAN_BOND_SEQ_2_MASK: bitvec[4];
        for i in 1..=4 {
            attribute "CHAN_BOND_SEQ_1_{i}": bitvec[11];
        }
        for i in 1..=4 {
            attribute "CHAN_BOND_SEQ_2_{i}": bitvec[11];
        }

        attribute TEST_MODE_1: bool;
        attribute TEST_MODE_2: bool;
        attribute TEST_MODE_3: bool;
        attribute TEST_MODE_4: bool;
        attribute TEST_MODE_5: bool;
        attribute TEST_MODE_6: bool;
    }

    table GT10_PMA_SPEED {
        field MASTERBIAS: bitvec[2];
        field VCODAC: bitvec[6];
        field TXDIVRATIO: bitvec[10];
        field TXBUSWID: bitvec[1];
        field ENDCD: bitvec[1];
        field SEL_DAC_TRAN: bitvec[4];
        field SEL_DAC_FIX: bitvec[4];
        field TXLOOPFILTERC: bitvec[2];
        field TXLOOPFILTERR: bitvec[2];
        field IBOOST: bitvec[1];
        field TXCPI: bitvec[1];
        field TXVCODAC: bitvec[1];
        field TXVCOGAIN: bitvec[1];
        field TXVSEL: bitvec[2];
        field TXREG: bitvec[2];
        field TXDOWNLEVEL: bitvec[4];
        field PRDRVOFF: bitvec[1];
        field EMPOFF: bitvec[1];
        field SLEW: bitvec[1];
        field TXEMPHLEVEL: bitvec[4];
        field TXDIGSW: bitvec[1];
        field TXANASW: bitvec[1];
        field RXDIVRATIO: bitvec[14];
        field RXLOOPFILTERC: bitvec[2];
        field RXLOOPFILTERR: bitvec[3];
        field AFE_FLAT_ENABLE: bitvec[1];
        field RXVCOSW: bitvec[1];
        field RXCPI: bitvec[2];
        field RXVCODAC: bitvec[1];
        field RXVCOGAIN: bitvec[1];
        field RXVSEL: bitvec[2];
        field RXREG: bitvec[2];
        field RXFLTCPT: bitvec[5];
        field RXVSELCP: bitvec[2];
        field VSELAFE: bitvec[2];
        field RXFEI: bitvec[2];
        field RXFLCPI: bitvec[2];
        field RXFER: bitvec[10];
        field PMA_REG_0E: bitvec[8];

        row _0_32;
        row _0_64;
        row _1_32;
        row _1_64;
        row _2_32;
        row _2_64;
        row _3_32;
        row _3_64;
        row _4_32;
        row _4_64;
        row _5_32;
        row _5_64;
        row _6_32;
        row _6_64;
        row _7_32;
        row _7_64;
        row _8_32;
        row _8_64;
        row _9_32;
        row _9_64;
        row _10_32;
        row _10_64;
        row _11_32;
        row _11_64;
        row _12_40;
        row _12_80;
        row _13_40;
        row _13_80;
        row _14_40;
        row _14_80;
        row _15_32;
        row _15_64;
        row _16_32;
        row _16_64;
        row _17_32;
        row _17_64;
        row _18_40;
        row _18_80;
        row _19_40;
        row _19_80;
        row _20_40;
        row _20_80;
        row _21_40;
        row _21_80;
        row _22_40;
        row _22_80;
        row _23_10;
        row _23_20;
        row _23_40;
        row _24_10;
        row _24_20;
        row _24_40;
        row _25_10;
        row _25_20;
        row _25_40;
        row _26_10;
        row _26_20;
        row _26_40;
        row _27_10;
        row _27_20;
        row _27_40;
        row _28_10;
        row _28_20;
        row _28_40;
        row _29_10;
        row _29_20;
        row _29_40;
        row _30_8;
        row _30_16;
        row _30_32;
        row _31_8;
        row _31_16;
        row _31_32;
    }

    bel_class PPC405 {
        input CPMC405CLOCK;
        input CPMC405CORECLKINACTIVE;
        input CPMC405CPUCLKEN;
        input CPMC405JTAGCLKEN;
        input CPMC405TIMERCLKEN;
        input CPMC405TIMERTICK;
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

        input DCRC405ACK;
        input DCRC405DBUSIN[0:31];
        output C405DCRABUS[0:9];
        output C405DCRDBUSOUT[0:31];
        output C405DCRREAD;
        output C405DCRWRITE;

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
        input BRAMDSOCMRDDACK;
        input BRAMDSOCMRDDBUS[0:31];
        input TIEDSOCMDCRADDR[0:7];
        input DSARCVALUE[0:7];
        input DSCNTLVALUE[0:7];
        output DSOCMBRAMABUS[8:29];
        output DSOCMBRAMBYTEWRITE[0:3];
        output DSOCMBRAMEN;
        output DSOCMBRAMWRDBUS[0:31];
        output DSOCMBUSY;
        output DSOCMRDADDRVALID;
        output C405DSOCMCACHEABLE;
        output C405DSOCMGUARDED;
        output C405DSOCMSTRINGMULTIPLE;
        output C405DSOCMU0ATTR;

        input BRAMISOCMCLK;
        input BRAMISOCMRDDACK;
        input BRAMISOCMRDDBUS[0:63];
        input TIEISOCMDCRADDR[0:7];
        input ISARCVALUE[0:7];
        input ISCNTLVALUE[0:7];
        output ISOCMBRAMEN;
        output ISOCMBRAMEVENWRITEEN;
        output ISOCMBRAMODDWRITEEN;
        output ISOCMBRAMRDABUS[8:28];
        output ISOCMBRAMWRABUS[8:28];
        output ISOCMBRAMWRDBUS[0:31];
        output ISOCMRDADDRVALID;
        output C405ISOCMCACHEABLE;
        output C405ISOCMCONTEXTSYNC;
        output C405ISOCMU0ATTR;

        input APUC405DCDAPUOP;
        input APUC405DCDCREN;
        input APUC405DCDFORCEALGN;
        input APUC405DCDFORCEBESTEERING;
        input APUC405DCDFPUOP;
        input APUC405DCDGPRWRITE;
        input APUC405DCDLDSTBYTE;
        input APUC405DCDLDSTDW;
        input APUC405DCDLDSTHW;
        input APUC405DCDLDSTQW;
        input APUC405DCDLDSTWD;
        input APUC405DCDLOAD;
        input APUC405DCDPRIVOP;
        input APUC405DCDRAEN;
        input APUC405DCDRBEN;
        input APUC405DCDSTORE;
        input APUC405DCDTRAPBE;
        input APUC405DCDTRAPLE;
        input APUC405DCDUPDATE;
        input APUC405DCDVALIDOP;
        input APUC405DCDXERCAEN;
        input APUC405DCDXEROVEN;
        input APUC405EXCEPTION;
        input APUC405EXEBLOCKINGMCO;
        input APUC405EXEBUSY;
        input APUC405EXECR[0:3];
        input APUC405EXECRFIELD[0:2];
        input APUC405EXELDDEPEND;
        input APUC405EXENONBLOCKINGMCO;
        input APUC405EXERESULT[0:31];
        input APUC405EXEXERCA;
        input APUC405EXEXEROV;
        input APUC405FPUEXCEPTION;
        input APUC405LWBLDDEPEND;
        input APUC405SLEEPREQ;
        input APUC405WBLDDEPEND;
        output C405APUDCDFULL;
        output C405APUDCDHOLD;
        output C405APUDCDINSTRUCTION[0:31];
        output C405APUEXEFLUSH;
        output C405APUEXEHOLD;
        output C405APUEXELOADDBUS[0:31];
        output C405APUEXELOADDVALID;
        output C405APUEXERADATA[0:31];
        output C405APUEXERBDATA[0:31];
        output C405APUEXEWDCNT[0:1];
        output C405APUMSRFE[0:1];
        output C405APUWBBYTEEN[0:3];
        output C405APUWBENDIAN;
        output C405APUWBFLUSH;
        output C405APUWBHOLD;
        output C405APUXERCA;

        input LSSDC405ACLK;
        input LSSDC405ARRAYCCLKNEG;
        input LSSDC405BCLK;
        input LSSDC405BISTCCLK;
        input LSSDC405CNTLPOINT;
        input LSSDC405SCANGATE;
        input LSSDC405SCANIN[0:9];
        input LSSDC405TESTEVS;
        input LSSDC405TESTM1;
        input LSSDC405TESTM3;
        output C405LSSDDIAGABISTDONE;
        output C405LSSDDIAGOUT;
        output C405LSSDSCANOUT[0:9];

        input TESTSELI;

        input TIEC405APUDIVEN;
        input TIEC405APUPRESENT;
        input TIEC405DETERMINISTICMULT;
        input TIEC405DISOPERANDFWD;
        input TIEC405MMUEN;
        input TIEC405PVR[0:31];
        input TIERAMTAP1;
        input TIERAMTAP2;
        input TIETAGTAP1;
        input TIETAGTAP2;
        input TIEUTLBTAP1;
        input TIEUTLBTAP2;

        input TSTC405DCRABUSI[0:9];
        input TSTC405DCRDBUSOUTI[0:31];
        input TSTC405DCRREADI;
        input TSTC405DCRWRITEI;

        input TSTCLKINACTI;
        output TSTCLKINACTO;
        input TSTCPUCLKENI;
        output TSTCPUCLKENO;
        input TSTCPUCLKI;
        output TSTCPUCLKO;
        input TSTDCRACKI;
        output TSTDCRACKO;
        input TSTDCRBUSI[0:31];
        output TSTDCRBUSO[0:31];
        input TSTDSOCMABORTOPI;
        output TSTDSOCMABORTOPO;
        input TSTDSOCMABORTREQI;
        output TSTDSOCMABORTREQO;
        input TSTDSOCMABUSI[0:29];
        output TSTDSOCMABUSO[0:29];
        input TSTDSOCMBYTEENI[0:3];
        output TSTDSOCMBYTEENO[0:3];
        input TSTDSOCMCOMPLETEI;
        input TSTDSOCMDBUSI[0:7];
        output TSTDSOCMDBUSO[0:7];
        input TSTDSOCMDCRACKI;
        output TSTDSOCMDCRACKO;
        input TSTDSOCMHOLDI;
        output TSTDSOCMHOLDO;
        input TSTDSOCMLOADREQI;
        output TSTDSOCMLOADREQO;
        input TSTDSOCMSTOREREQI;
        output TSTDSOCMSTOREREQO;
        input TSTDSOCMWAITI;
        output TSTDSOCMWAITO;
        input TSTDSOCMWRDBUSI[0:31];
        output TSTDSOCMWRDBUSO[0:31];
        input TSTDSOCMXLATEVALIDI;
        output TSTDSOCMXLATEVALIDO;
        input TSTISOCMABORTI;
        output TSTISOCMABORTO;
        input TSTISOCMABUSI[0:29];
        output TSTISOCMABUSO[0:29];
        input TSTISOCMHOLDI;
        output TSTISOCMHOLDO;
        input TSTISOCMICUREADYI;
        output TSTISOCMICUREADYO;
        input TSTISOCMRDATAI[0:63];
        output TSTISOCMRDATAO[0:63];
        input TSTISOCMRDDVALIDI[0:1];
        output TSTISOCMRDDVALIDO[0:1];
        input TSTISOCMREQPENDI;
        output TSTISOCMREQPENDO;
        input TSTISOCMXLATEVALIDI;
        output TSTISOCMXLATEVALIDO;
        input TSTISOPFWDI;
        output TSTISOPFWDO;
        input TSTJTAGENI;
        output TSTJTAGENO;
        output TSTOCMCOMPLETEO;
        input TSTPLBSAMPLECYCLEI;
        output TSTPLBSAMPLECYCLEO;
        input TSTRDDBUSI[0:31];
        output TSTRDDBUSO[0:31];
        input TSTRESETCHIPI;
        output TSTRESETCHIPO;
        input TSTRESETCOREI;
        output TSTRESETCOREO;
        input TSTRESETSYSI;
        output TSTRESETSYSO;
        input TSTTIMERENI;
        output TSTTIMERENO;
        input TSTTRSTNEGI;
        output TSTTRSTNEGO;
    }

    region_slot GLOBAL;
    // A set of cells sharing a HCLK row.
    region_slot HCLK;
    // A set of cells sharing HCLK leaf.
    region_slot LEAF;
    region_slot DCM_CLKPAD;
    region_slot DCM_BUS;

    if variant virtex2 {
        wire PULLUP: pullup;

        wire GCLK_S[8]: regional GLOBAL;
        wire GCLK_N[8]: regional GLOBAL;
        wire GCLK_ROW[8]: regional HCLK;
        wire GCLK[8]: regional LEAF;
        wire OUT_CLKPAD[4]: bel;
        wire DCM_CLKPAD[8]: regional DCM_CLKPAD;
        wire DCM_BUS[8]: regional DCM_BUS;

        wire OMUX[16]: mux;
        wire OMUX_S0: branch N;
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

        wire LH[24]: multi_branch W;
        wire LV[24]: multi_branch S;

        wire IMUX_CLK[4]: mux;
        wire IMUX_CLK_OPTINV[4]: mux;
        wire IMUX_DCM_CLK[4]: mux;
        wire IMUX_DCM_CLK_OPTINV[4]: mux;
        wire IMUX_SR[4]: mux;
        wire IMUX_SR_OPTINV[4]: mux;
        wire IMUX_CE[4]: mux;
        wire IMUX_CE_OPTINV[4]: mux;
        wire IMUX_TI[2]: mux;
        wire IMUX_TI_OPTINV[2]: mux;
        wire IMUX_TS[2]: mux;
        wire IMUX_TS_OPTINV[2]: mux;

        for i in 1..=5 {
            wire "IMUX_CLB_F{i}"[4]: mux;
        }
        for i in 1..=5 {
            wire "IMUX_CLB_G{i}"[4]: mux;
        }
        wire IMUX_CLB_BX[4]: mux;
        wire IMUX_CLB_BY[4]: mux;

        for i in 0..4 {
            wire "IMUX_G{i}_FAN"[2]: mux;
            wire "IMUX_G{i}_DATA"[8]: mux;
        }
        wire IMUX_IOI_ICLK[4]: mux;
        wire IMUX_IOI_TS1[4]: mux;
        wire IMUX_IOI_TS2[4]: mux;
        wire IMUX_IOI_ICE[4]: mux;
        wire IMUX_IOI_TCE[4]: mux;

        wire IMUX_BRAM_ADDRA[4]: mux;
        for i in 1..=5 {
            wire "IMUX_BRAM_ADDRA_S{i}"[4]: branch N;
        }
        for i in 1..=5 {
            wire "IMUX_BRAM_ADDRA_N{i}"[4]: branch S;
        }
        wire IMUX_BRAM_ADDRB[4]: mux;
        for i in 1..=5 {
            wire "IMUX_BRAM_ADDRB_S{i}"[4]: branch N;
        }
        for i in 1..=5 {
            wire "IMUX_BRAM_ADDRB_N{i}"[4]: branch S;
        }

        wire OUT_FAN[8]: bel;
        wire OUT_FAN_BEL[8]: bel;
        wire OUT_SEC[24]: bel;
        wire OUT_SEC_BEL[24]: bel;
        wire OUT_HALF0[18]: bel;
        wire OUT_HALF1[18]: bel;
        wire OUT_TEST[16]: bel;
        wire OUT_TBUS: bel;
        wire OUT_PCI[2]: bel;

        wire IMUX_BUFG_CLK_INT[8]: mux;
        wire IMUX_BUFG_CLK[8]: mux;
        wire IMUX_BUFG_SEL[8]: mux;
    } else {
        wire PULLUP: pullup;

        wire GCLK_S[4]: regional GLOBAL;
        wire GCLK_N[4]: regional GLOBAL;
        wire GCLK_WE[8]: regional HCLK;
        wire GCLK_QUAD[8]: regional HCLK;
        wire GCLK[8]: regional LEAF;
        wire OUT_CLKPAD[2]: bel;
        wire DCM_CLKPAD[4]: regional DCM_CLKPAD;
        wire DCM_BUS[4]: regional DCM_BUS;

        wire OMUX[16]: mux;
        wire OMUX_S0: branch N;
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
        wire OMUX_N9: branch S;
        wire OMUX_N10: branch S;
        wire OMUX_NW10: branch E;
        wire OMUX_N11: branch S;
        wire OMUX_N12: branch S;
        wire OMUX_NE12: branch W;
        wire OMUX_E13: branch W;
        wire OMUX_W14: branch E;
        wire OMUX_WN14: branch S;
        wire OMUX_N15: branch S;

        wire DBL_W0[8]: mux;
        wire DBL_W1[8]: branch E;
        wire DBL_W2[8]: branch E;
        wire DBL_W2_N[8]: branch S;
        wire DBL_E0[8]: mux;
        wire DBL_E1[8]: branch W;
        wire DBL_E2[8]: branch W;
        wire DBL_E2_S[8]: branch N;
        wire DBL_S0[8]: mux;
        wire DBL_S1[8]: branch N;
        wire DBL_S2[8]: branch N;
        wire DBL_S3[8]: branch N;
        wire DBL_N0[8]: mux;
        wire DBL_N1[8]: branch S;
        wire DBL_N2[8]: branch S;
        wire DBL_N3[8]: branch S;

        wire HEX_W0[8]: mux;
        for i in 1..=6 {
            wire "HEX_W{i}"[8]: branch E;
        }
        wire HEX_W6_N[8]: branch S;
        wire HEX_E0[8]: mux;
        for i in 1..=6 {
            wire "HEX_E{i}"[8]: branch W;
        }
        wire HEX_E6_S[8]: branch N;
        wire HEX_S0[8]: mux;
        for i in 1..=7 {
            wire "HEX_S{i}"[8]: branch N;
        }
        wire HEX_N0[8]: mux;
        for i in 1..=7 {
            wire "HEX_N{i}"[8]: branch S;
        }

        wire LH[24]: multi_branch W;
        wire LV[24]: multi_branch S;

        wire IMUX_CLK[4]: mux;
        wire IMUX_CLK_OPTINV[4]: mux;
        wire IMUX_SR[4]: mux;
        wire IMUX_SR_OPTINV[4]: mux;
        wire IMUX_CE[4]: mux;
        wire IMUX_CE_OPTINV[4]: mux;
        wire IMUX_IOCLK[8]: mux;

        wire IMUX_FAN_BX[4]: mux;
        wire IMUX_FAN_BY[4]: mux;
        wire IMUX_FAN_BX_BOUNCE[4]: mux;
        wire IMUX_FAN_BY_BOUNCE[4]: mux;
        wire IMUX_DATA[32]: mux;

        wire OUT_FAN[8]: bel;
        wire OUT_FAN_BEL[8]: bel;
        wire OUT_SEC[16]: bel;
        wire OUT_SEC_BEL[16]: bel;
        wire OUT_HALF0[4]: bel;
        wire OUT_HALF1[4]: bel;
        wire OUT_HALF0_BEL[4]: bel;
        wire OUT_HALF1_BEL[4]: bel;

        wire IMUX_BUFG_CLK_INT[4]: mux;
        wire IMUX_BUFG_CLK[4]: mux;
        wire IMUX_BUFG_SEL[4]: mux;

        // spartan3a only
        wire IMUX_MULT_A[18]: mux;
        wire IMUX_MULT_B[18]: mux;
    }

    if variant virtex2 {
        bitrect MAIN = vertical (rev 22, rev 80);
        bitrect CLK = vertical (rev 4, rev 80);
        bitrect CLK_SN = vertical (rev 4, rev 16);
        bitrect HCLK = vertical (rev 22, rev 1);
        bitrect TERM_H = vertical (rev 4, rev 80);
        bitrect TERM_V = vertical (rev 22, rev 12);
        bitrect BRAM_DATA = vertical (rev 64, rev 320);
    } else {
        bitrect MAIN = vertical (rev 19, rev 64);
        bitrect CLK = vertical (rev 1, rev 64);
        bitrect CLK_SN = vertical (rev 1, rev 16);
        bitrect CLK_LL = vertical (rev 2, rev 64);
        bitrect CLK_SN_LL = vertical (rev 2, rev 16);
        bitrect HCLK = vertical (rev 19, rev 1);
        bitrect TERM_H = vertical (rev 2, rev 64);
        bitrect TERM_V_S3 = vertical (rev 19, rev 5);
        bitrect TERM_V_S3A = vertical (rev 19, rev 6);
        bitrect LLV_S = vertical (rev 19, rev 1);
        bitrect LLV_N = vertical (rev 19, rev 2);
        bitrect LLV = vertical (rev 19, rev 3);
        bitrect BRAM_DATA = vertical (rev 76, rev 256);
    }

    bitrect REG32 = horizontal (1, rev 32);
    bitrect REG16 = horizontal (1, rev 16);

    tile_slot INT {
        bel_slot INT: routing;
        bel_slot PTE2OMUX: routing;

        if variant virtex2 {
            tile_class
                INT_CLB,
                INT_IOI,
                INT_IOI_CLK_S, // TODO: merge
                INT_IOI_CLK_N, // TODO: merge
                INT_BRAM,
                INT_DCM_V2,
                INT_DCM_V2P, // TODO: merge (if possible)
                INT_CNR,
                INT_PPC,
                INT_GT_CLKPAD
            {
                cell CELL;
                bitrect MAIN: MAIN;
            }
        } else {
            tile_class
                INT_CLB,
                INT_CLB_FC,
                INT_IOI_S3,
                INT_IOI_FC,
                INT_IOI_S3E, // TODO: merge
                INT_IOI_S3A_WE, // TODO: merge
                INT_IOI_S3A_SN, // TODO: merge
                INT_BRAM_S3,
                INT_BRAM_S3E, // TODO: merge
                INT_BRAM_S3A_03, // do *NOT* merge; evil one without CLK/CE
                INT_BRAM_S3A_12, // TODO: merge
                INT_BRAM_S3ADSP, // TODO: merge
                INT_DCM,
                INT_DCM_S3_DUMMY, // TODO: merge if possible
                INT_DCM_S3E_DUMMY // TODO: merge if possible
            {
                cell CELL;
                bitrect MAIN: MAIN;
            }
        }
    }

    tile_slot INTF {
        bel_slot INTF_TESTMUX: routing;

        if variant virtex2 {
            tile_class
                INTF_GT_S0,
                INTF_GT_S123,
                INTF_GT_S_CLKPAD,
                INTF_GT_N0,
                INTF_GT_N123,
                INTF_GT_N_CLKPAD,
                INTF_PPC
            {
                cell CELL;
                bitrect MAIN: MAIN;
            }
        }
    }

    tile_slot BEL {
        bel_slot SLICE[4]: SLICE;
        bel_slot TBUF[2]: TBUF;
        bel_slot TBUS: TBUS;
        tile_class CLB {
            cell CELL;
            bitrect MAIN: MAIN;
        }

        bel_slot IOI[4]: IOI;
        bel_slot IREG[4]: IREG;
        bel_slot OREG[4]: OREG;
        if variant virtex2 {
            tile_class IOI, IOI_CLK_S, IOI_CLK_N { // TODO: possible to merge?
                cell CELL;
                bitrect MAIN: MAIN;
            }
        } else {
            tile_class IOI_S3, IOI_FC, IOI_S3E, IOI_S3A_WE, IOI_S3A_S, IOI_S3A_N {
                cell CELL;
                bitrect MAIN: MAIN;
            }
        }

        bel_slot BRAM: BRAM;
        bel_slot MULT: MULT;
        bel_slot MULT_INT: routing;
        if variant virtex2 {
            tile_class BRAM {
                cell CELL[4];
                bitrect MAIN[4]: MAIN;
                bitrect DATA: BRAM_DATA;
            }
        } else {
            tile_class BRAM_S3, BRAM_S3E, BRAM_S3A, BRAM_S3ADSP {
                cell CELL[4];
                bitrect MAIN[4]: MAIN;
                bitrect DATA: BRAM_DATA;
            }
        }

        bel_slot DSP: DSP;
        bel_slot DSP_TESTMUX: routing;
        if variant spartan3 {
            tile_class DSP {
                cell CELL[4];
                bitrect MAIN[4]: MAIN;
            }
        }

        bel_slot DCM: DCM;
        bel_slot DCM_INT: routing;
        if variant virtex2 {
            tile_class DCM_V2, DCM_V2P {
                cell CELL;
                bitrect MAIN: MAIN;
                bitrect TERM: TERM_V;
            }
        } else {
            tile_class DCM_S3 {
                cell CELL;
                bitrect MAIN: MAIN;
            }

            tile_class
                DCM_S3E_SW,
                DCM_S3E_SE,
                DCM_S3E_NW,
                DCM_S3E_NE,
                DCM_S3E_WS,
                DCM_S3E_WN,
                DCM_S3E_ES,
                DCM_S3E_EN
            {
                cell CELL;
                bitrect MAIN_C[4]: MAIN;
                bitrect MAIN_S[4]: MAIN;
            }
        }

        bel_slot GT: GT;
        bel_slot GT10: GT10;
        if variant virtex2 {
            tile_class GIGABIT_S, GIGABIT_N {
                cell CELL_IO;
                cell CELL[4];
                bitrect MAIN_IO: MAIN;
                bitrect MAIN[4]: MAIN;
            }
            tile_class GIGABIT10_S, GIGABIT10_N {
                cell CELL_IO;
                cell CELL[8];
                bitrect MAIN_IO: MAIN;
                bitrect MAIN[8]: MAIN;
            }
        }

        bel_slot PPC405: PPC405;
        if variant virtex2 {
            tile_class PPC_W, PPC_E {
                cell CELL_W[16];
                cell CELL_E[16];
                cell CELL_S[8];
                cell CELL_N[8];
            }
        }

        bel_slot DCI[2]: DCI;
        bel_slot DCIRESET[2]: DCIRESET;
        bel_slot STARTUP: STARTUP;
        bel_slot CAPTURE: CAPTURE;
        bel_slot ICAP: ICAP;
        bel_slot SPI_ACCESS: SPI_ACCESS;
        bel_slot PMV: PMV;
        bel_slot DNA_PORT: DNA_PORT;
        bel_slot BSCAN: BSCAN;
        bel_slot JTAGPPC: JTAGPPC;
        bel_slot RANDOR_OUT: RANDOR_OUT;
        bel_slot MISC_CNR_S3: MISC_CNR_S3;
        bel_slot MISR_FC: MISR_FC;
        bel_slot MISC_SW: MISC_SW;
        bel_slot MISC_SE: MISC_SE;
        bel_slot MISC_NW: MISC_NW;
        bel_slot MISC_NE: MISC_NE;
        bel_slot BANK: BANK;
        if variant virtex2 {
            tile_class
                CNR_SW_V2,
                CNR_SW_V2P,
                CNR_SE_V2,
                CNR_SE_V2P,
                CNR_NW_V2,
                CNR_NW_V2P,
                CNR_NE_V2,
                CNR_NE_V2P
            {
                cell CELL;
                bitrect TERM_H: TERM_H;
                bitrect TERM_V: TERM_V;
            }
        } else {
            tile_class
                CNR_SW_S3,
                CNR_SW_FC,
                CNR_SW_S3E,
                CNR_SW_S3A,
                CNR_SE_S3,
                CNR_SE_FC,
                CNR_SE_S3E,
                CNR_SE_S3A,
                CNR_NW_S3,
                CNR_NW_FC,
                CNR_NW_S3E,
                CNR_NW_S3A,
                CNR_NE_S3,
                CNR_NE_FC,
                CNR_NE_S3E,
                CNR_NE_S3A
            {
                cell CELL;
                bitrect TERM_H: TERM_H;
            }
        }
    }

    tile_slot TERM_H {
        bel_slot TERM_W: routing;
        bel_slot TERM_E: routing;
        bel_slot PPC_TERM_W: routing;
        bel_slot PPC_TERM_E: routing;
        bel_slot LLH: routing;

        if variant virtex2 {
            tile_class TERM_W {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class TERM_E {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class PPC_TERM_W {
                cell CELL;
                cell FAR;
                bitrect MAIN: MAIN;
            }
            tile_class PPC_TERM_E {
                cell CELL;
                cell FAR;
                bitrect MAIN: MAIN;
            }
        } else {
            tile_class LLH, LLH_S_S3A, LLH_N_S3A {
                cell W;
                cell E;
                bitrect CLK: CLK_LL;
            }
        }
    }

    tile_slot TERM_V {
        bel_slot TERM_S: routing;
        bel_slot TERM_N: routing;
        bel_slot PPC_TERM_S: routing;
        bel_slot PPC_TERM_N: routing;
        bel_slot LLV: routing;

        if variant virtex2 {
            tile_class TERM_S {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class TERM_N {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class PPC_TERM_S {
                cell CELL;
                cell FAR;
                bitrect MAIN: MAIN;
            }
            tile_class PPC_TERM_N {
                cell CELL;
                cell FAR;
                bitrect MAIN: MAIN;
            }
        } else {
            tile_class LLV_S3E {
                cell S;
                cell N;
                bitrect LLV_S: LLV_S;
                bitrect LLV_N: LLV_N;
            }
            tile_class LLV_S3A {
                cell S;
                cell N;
                bitrect LLV: LLV;
            }
        }
    }

    tile_slot IOB {
        bel_slot IOB[8]: IOB;
        bel_slot IBUF[4]: IBUF;
        bel_slot OBUF[4]: OBUF;
        if variant virtex2 {
            tile_class IOB_V2_SW2, IOB_V2_SE2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V;
            }
            tile_class IOB_V2_NW2, IOB_V2_NE2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V;
            }
            tile_class IOB_V2_WS2, IOB_V2_WN2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
            tile_class IOB_V2_ES2, IOB_V2_EN2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }

            tile_class IOB_V2P_SW2, IOB_V2P_SE2, IOB_V2P_SE2_CLK {
                cell CELL[2];
                bitrect TERM[2]: TERM_V;
            }
            tile_class IOB_V2P_SW1, IOB_V2P_SW1_ALT, IOB_V2P_SE1, IOB_V2P_SE1_ALT {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class IOB_V2P_NW2, IOB_V2P_NE2, IOB_V2P_NE2_CLK {
                cell CELL[2];
                bitrect TERM[2]: TERM_V;
            }
            tile_class IOB_V2P_NW1, IOB_V2P_NW1_ALT, IOB_V2P_NE1, IOB_V2P_NE1_ALT {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class IOB_V2P_WS2, IOB_V2P_WN2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
            tile_class IOB_V2P_ES2, IOB_V2P_EN2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
        } else {
            tile_class IOB_S3_W1 {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_S3_E1 {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_S3_S2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3;
            }
            tile_class IOB_S3_N2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3;
            }

            tile_class IOB_FC_W {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_FC_E {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_FC_S {
                cell CELL;
                bitrect TERM: TERM_V_S3;
            }
            tile_class IOB_FC_N {
                cell CELL;
                bitrect TERM: TERM_V_S3;
            }

            tile_class IOB_S3E_W1 {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_S3E_W2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
            tile_class IOB_S3E_W3 {
                cell CELL[3];
                bitrect TERM[3]: TERM_H;
            }
            tile_class IOB_S3E_W4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_H;
            }
            tile_class IOB_S3E_E1 {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_S3E_E2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
            tile_class IOB_S3E_E3 {
                cell CELL[3];
                bitrect TERM[3]: TERM_H;
            }
            tile_class IOB_S3E_E4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_H;
            }
            tile_class IOB_S3E_S1 {
                cell CELL;
                bitrect TERM: TERM_V_S3;
            }
            tile_class IOB_S3E_S2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3;
            }
            tile_class IOB_S3E_S3 {
                cell CELL[3];
                bitrect TERM[3]: TERM_V_S3;
            }
            tile_class IOB_S3E_S4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_V_S3;
            }
            tile_class IOB_S3E_N1 {
                cell CELL;
                bitrect TERM: TERM_V_S3;
            }
            tile_class IOB_S3E_N2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3;
            }
            tile_class IOB_S3E_N3 {
                cell CELL[3];
                bitrect TERM[3]: TERM_V_S3;
            }
            tile_class IOB_S3E_N4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_V_S3;
            }

            tile_class IOB_S3A_W4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_H;
            }
            tile_class IOB_S3A_E4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_H;
            }
            tile_class IOB_S3A_S2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3A;
            }
            tile_class IOB_S3A_N2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3A;
            }
        }
    }

    tile_slot CLK {
        bel_slot CLK_INT: routing;
        bel_slot BUFGMUX[8]: BUFGMUX;
        bel_slot PCILOGIC: PCILOGIC;
        bel_slot PCILOGICSE: PCILOGICSE;
        bel_slot GLOBALSIG_BUFG[2]: GLOBALSIG_BUFG;
        if variant virtex2 {
            tile_class CLK_S {
                cell CELL[2];
                bitrect MAIN: CLK;
                bitrect TERM: CLK_SN;
            }
            tile_class CLK_N {
                cell CELL[2];
                bitrect MAIN: CLK;
                bitrect TERM: CLK_SN;
            }
        } else {
            tile_class CLK_S_S3, CLK_S_FC, CLK_S_S3E, CLK_S_S3A {
                if tile_class [CLK_S_S3, CLK_S_FC] {
                    cell CELL[2];
                } else {
                    cell CELL[8];
                }
                if tile_class CLK_S_S3A {
                    bitrect MAIN: CLK;
                    bitrect TERM: CLK_SN_LL;
                } else {
                    bitrect MAIN: CLK;
                    bitrect TERM: CLK_SN;
                }
            }
            tile_class CLK_N_S3, CLK_N_FC, CLK_N_S3E, CLK_N_S3A {
                if tile_class [CLK_N_S3, CLK_N_FC] {
                    cell CELL[2];
                } else {
                    cell CELL[8];
                }
                if tile_class CLK_N_S3A {
                    bitrect MAIN: CLK;
                    bitrect TERM: CLK_SN_LL;
                } else {
                    bitrect MAIN: CLK;
                    bitrect TERM: CLK_SN;
                }
            }
            tile_class CLK_W_S3E, CLK_W_S3A {
                cell CELL[8];
                bitrect MAIN[2]: MAIN;
                bitrect TERM[4]: TERM_H;
            }
            tile_class CLK_E_S3E, CLK_E_S3A {
                cell CELL[8];
                bitrect MAIN[2]: MAIN;
                bitrect TERM[4]: TERM_H;
            }
        }

        bel_slot DCMCONN: routing;
        if variant virtex2 {
            tile_class DCMCONN_S, DCMCONN_N {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class PCI_W, PCI_E {
                cell CELL[4];
            }
        } else {
            tile_class DCMCONN_S, DCMCONN_N {
                cell CELL;
            }
        }
    }

    tile_slot HROW {
        bel_slot HROW: routing;
        if variant virtex2 {
            tile_class HROW {
                cell CELL_W;
                cell CELL_E;
                bitrect CLK[4]: CLK;
            }
            tile_class HROW_S {
                cell CELL_W;
                cell CELL_E;
                bitrect CLK_S: CLK_SN;
                bitrect CLK[3]: CLK;
            }
            tile_class HROW_N {
                cell CELL_W;
                cell CELL_E;
                bitrect CLK[3]: CLK;
                bitrect CLK_N: CLK_SN;
            }
        } else {
            tile_class CLKC_50A {
                cell CELL_W, CELL_E;
                bitrect MAIN: CLK_LL;
            }
            tile_class CLKQC_S3, CLKQC_S3E {
                cell CELL_S, CELL_N;
                bitrect CLK_S: CLK;
                bitrect CLK_N: CLK;
            }
        }
    }

    tile_slot HCLK {
        bel_slot HCLK: routing;
        if variant virtex2 {
            bel_slot GLOBALSIG_HCLK: GLOBALSIG_HCLK_V2;
            tile_class HCLK {
                cell S, N;
                bitrect MAIN: HCLK;
            }
        } else {
            bel_slot GLOBALSIG_HCLK: GLOBALSIG_HCLK_S3;
            tile_class HCLK, HCLK_UNI {
                cell S, N;
                bitrect MAIN: HCLK;
            }
        }
    }

    tile_slot RANDOR {
        bel_slot RANDOR: RANDOR;
        bel_slot RANDOR_INIT: RANDOR_INIT;
        if variant spartan3 {
            tile_class RANDOR {
                bitrect MAIN: MAIN;
            }
            tile_class RANDOR_FC {
                bitrect TERM: TERM_V_S3;
            }
            tile_class RANDOR_INIT {
                bitrect MAIN: MAIN;
                bel RANDOR_INIT;
            }
            tile_class RANDOR_INIT_FC {
                bitrect MAIN: MAIN;
                bel RANDOR_INIT;
            }
        }
    }

    tile_slot GLOBAL {
        bel_slot GLOBAL: GLOBAL;
        if variant virtex2 {
            tile_class GLOBAL {
                bitrect COR: REG32;
                bitrect CTL: REG32;
                bel GLOBAL;
            }
        } else {
            tile_class GLOBAL_S3, GLOBAL_FC, GLOBAL_S3E {
                bitrect COR: REG32;
                bitrect CTL: REG32;
                bel GLOBAL;
            }
            tile_class GLOBAL_S3A {
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
                bitrect SEU_OPT: REG16;
                bel GLOBAL;
            }
        }
    }

    connector_slot W {
        opposite E;

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
            pass LH[0] = LH[23];
            for i in 0..23 {
                pass LH[i + 1] = LH[i];
            }
        }
        if variant spartan3 {
            connector_class PASS_W_FC {
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
                pass LH[0] = LH[11];
                for i in 0..11 {
                    pass LH[i + 1] = LH[i];
                }
            }
        }
        connector_class TERM_W;
        if variant virtex2 {
            connector_class PPC_W;
        } else {
            connector_class LLH_W;
            connector_class LLH_DCM_S3ADSP_W;
            connector_class DSPHOLE_W;
            connector_class HDCM_W;
        }
    }

    connector_slot E {
        opposite W;

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
        }
        connector_class TERM_E;
        if variant virtex2 {
            connector_class PPC_E;
        } else {
            connector_class LLH_E;
            connector_class LLH_DCM_S3ADSP_E;
            connector_class DSPHOLE_E;
            connector_class HDCM_E;
        }
    }

    connector_slot S {
        opposite N;
        connector_class PASS_S {
            pass OMUX_EN8 = OMUX_E8;
            pass OMUX_N10 = OMUX[10];
            pass OMUX_N11 = OMUX[11];
            pass OMUX_N12 = OMUX[12];
            if variant virtex2 {
                pass OMUX_N13 = OMUX[13];
            } else {
                pass OMUX_N9 = OMUX[9];
            }
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
            for i in 0..23 {
                pass LV[i] = LV[i + 1];
            }
            pass LV[23] = LV[0];
            if variant virtex2 {
                pass IMUX_BRAM_ADDRA_N1 = IMUX_BRAM_ADDRA;
                pass IMUX_BRAM_ADDRA_N2 = IMUX_BRAM_ADDRA_N1;
                pass IMUX_BRAM_ADDRA_N3 = IMUX_BRAM_ADDRA_N2;
                pass IMUX_BRAM_ADDRA_N4 = IMUX_BRAM_ADDRA_N3;
                pass IMUX_BRAM_ADDRB_N1 = IMUX_BRAM_ADDRB;
                pass IMUX_BRAM_ADDRB_N2 = IMUX_BRAM_ADDRB_N1;
                pass IMUX_BRAM_ADDRB_N3 = IMUX_BRAM_ADDRB_N2;
                pass IMUX_BRAM_ADDRB_N4 = IMUX_BRAM_ADDRB_N3;
            }
        }
        if variant spartan3 {
            connector_class PASS_S_FC {
                pass OMUX_EN8 = OMUX_E8;
                pass OMUX_N9 = OMUX[9];
                pass OMUX_N10 = OMUX[10];
                pass OMUX_N11 = OMUX[11];
                pass OMUX_N12 = OMUX[12];
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
                for i in 0..11 {
                    pass LV[i] = LV[i + 1];
                }
                pass LV[11] = LV[0];
            }
        }
        connector_class TERM_S;
        if variant virtex2 {
            connector_class PPC_S;
        } else {
            connector_class BRKH_S3_S;
            connector_class TERM_BRAM_S;
            connector_class LLV_S;
            connector_class LLV_CLK_WE_S3E_S;
            connector_class CLK_WE_S3E_S;
        }
    }

    connector_slot N {
        opposite S;
        connector_class PASS_N {
            pass OMUX_S0 = OMUX[0];
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
            if variant virtex2 {
                pass IMUX_BRAM_ADDRA_S1 = IMUX_BRAM_ADDRA;
                pass IMUX_BRAM_ADDRA_S2 = IMUX_BRAM_ADDRA_S1;
                pass IMUX_BRAM_ADDRA_S3 = IMUX_BRAM_ADDRA_S2;
                pass IMUX_BRAM_ADDRA_S4 = IMUX_BRAM_ADDRA_S3;
                pass IMUX_BRAM_ADDRB_S1 = IMUX_BRAM_ADDRB;
                pass IMUX_BRAM_ADDRB_S2 = IMUX_BRAM_ADDRB_S1;
                pass IMUX_BRAM_ADDRB_S3 = IMUX_BRAM_ADDRB_S2;
                pass IMUX_BRAM_ADDRB_S4 = IMUX_BRAM_ADDRB_S3;
            }
        }
        connector_class TERM_N;
        if variant virtex2 {
            connector_class PPC_N;
        } else {
            connector_class BRKH_S3_N;
            connector_class TERM_BRAM_N;
            connector_class LLV_N;
            connector_class LLV_CLK_WE_S3E_N;
            connector_class CLK_WE_S3E_N;
        }
    }
}
