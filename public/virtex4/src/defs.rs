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

    // virtex5 has ALT and [ABC]X; virtex[67] has ALT and [ABC]I
    enum SLICE_MUX_ADI1 { ALT, AX, AI }
    enum SLICE_MUX_BDI1 { ALT, BX, BI }
    enum SLICE_MUX_CDI1 { ALT, CX, CI }
    enum SLICE_MUX_WE { WE, CE }
    enum SLICE_RAMMODE { NONE, RAM64, RAM32, SRL32, SRL16 }
    enum SLICE_CYINIT { PRECYINIT, CIN }
    enum SLICE_PRECYINIT { CONST_0, CONST_1, AX }
    enum SLICE_MUX_ACY0 { AX, O5 }
    enum SLICE_MUX_BCY0 { BX, O5 }
    enum SLICE_MUX_CCY0 { CX, O5 }
    enum SLICE_MUX_DCY0 { DX, O5 }
    enum SLICE_MUX_FFA { NONE, AX, O6, O5, XOR, CY, F7 }
    enum SLICE_MUX_FFB { NONE, BX, O6, O5, XOR, CY, F8 }
    enum SLICE_MUX_FFC { NONE, CX, O6, O5, XOR, CY, F7 }
    enum SLICE_MUX_FFD { NONE, DX, O6, O5, XOR, CY, MC31 }
    // [ABCD]5Q are virtex6 and up only
    enum SLICE_MUX_AOUT { NONE, A5Q, O6, O5, XOR, CY, F7 }
    enum SLICE_MUX_BOUT { NONE, B5Q, O6, O5, XOR, CY, F8 }
    enum SLICE_MUX_COUT { NONE, C5Q, O6, O5, XOR, CY, F7 }
    enum SLICE_MUX_DOUT { NONE, D5Q, O6, O5, XOR, CY, MC31 }
    enum SLICE_MUX_FFA5 { NONE, AX, O5 }
    enum SLICE_MUX_FFB5 { NONE, BX, O5 }
    enum SLICE_MUX_FFC5 { NONE, CX, O5 }
    enum SLICE_MUX_FFD5 { NONE, DX, O5 }
    bel_class SLICE_V5 {
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

        attribute PRECYINIT: SLICE_PRECYINIT;
        attribute CYINIT: SLICE_CYINIT;
        attribute MUX_ACY0: SLICE_MUX_ACY0;
        attribute MUX_BCY0: SLICE_MUX_BCY0;
        attribute MUX_CCY0: SLICE_MUX_CCY0;
        attribute MUX_DCY0: SLICE_MUX_DCY0;

        attribute MUX_FFA: SLICE_MUX_FFA;
        attribute MUX_FFB: SLICE_MUX_FFB;
        attribute MUX_FFC: SLICE_MUX_FFC;
        attribute MUX_FFD: SLICE_MUX_FFD;

        // virtex6 and up only
        attribute MUX_FFA5: SLICE_MUX_FFA5;
        attribute MUX_FFB5: SLICE_MUX_FFB5;
        attribute MUX_FFC5: SLICE_MUX_FFC5;
        attribute MUX_FFD5: SLICE_MUX_FFD5;

        // virtex5 and virtex7 only (virtex6 has per-FF attributes instead)
        attribute FF_LATCH: bool;
        attribute FF_SR_ENABLE: bool;
        attribute FF_SR_SYNC: bool;
        attribute FF_CE_ENABLE: bool;
        // virtex5 only
        attribute FF_REV_ENABLE: bool;

        // virtex6 only
        attribute FFA_LATCH, FFB_LATCH, FFC_LATCH, FFD_LATCH: bool;

        attribute FFA_INIT, FFB_INIT, FFC_INIT, FFD_INIT: bitvec[1];
        attribute FFA_SRVAL, FFB_SRVAL, FFC_SRVAL, FFD_SRVAL: bitvec[1];
        // virtex6 and up only
        attribute FFA5_INIT, FFB5_INIT, FFC5_INIT, FFD5_INIT: bitvec[1];
        attribute FFA5_SRVAL, FFB5_SRVAL, FFC5_SRVAL, FFD5_SRVAL: bitvec[1];

        attribute MUX_AOUT: SLICE_MUX_AOUT;
        attribute MUX_BOUT: SLICE_MUX_BOUT;
        attribute MUX_COUT: SLICE_MUX_COUT;
        attribute MUX_DOUT: SLICE_MUX_DOUT;
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

    enum BRAM_V5_DATA_WIDTH { _1, _2, _4, _9, _18 }
    enum BRAM_V5_FIFO_WIDTH { _2, _4, _9, _18, _36 }
    bel_class BRAM_V5 {
        input CLKAL, CLKAU, CLKBL, CLKBU;
        input ENAL, ENAU, ENBU, ENBL;
        input SSRAL, SSRAU, SSRBL, SSRBU;
        input WEAL[4], WEAU[4], WEBL[8], WEBU[8];
        input REGCEAL, REGCEAU, REGCEBL, REGCEBU;
        input REGCLKAL, REGCLKAU, REGCLKBL, REGCLKBU;
        input ADDRAL[16], ADDRAU[15], ADDRBL[16], ADDRBU[15];
        input DIAL[16], DIAU[16], DIBL[16], DIBU[16];
        input DIPAL[2], DIPAU[2], DIPBL[2], DIPBU[2];
        output DOAL[16], DOAU[16], DOBL[16], DOBU[16];
        output DOPAL[2], DOPAU[2], DOPBL[2], DOPBU[2];
        output ECCPARITY[8];
        output SBITERR;
        output DBITERR;
        output EMPTY;
        output FULL;
        output ALMOSTEMPTY;
        output ALMOSTFULL;
        output RDCOUNT[13];
        output RDERR;
        output WRCOUNT[13];
        output WRERR;

        input TSTFLAGIN;
        input TSTOFF;
        input TSTRDCNTOFF;
        input TSTWRCNTOFF;
        input TSTCNT[13];
        input TSTRDOS[13];
        input TSTWROS[13];

        attribute DATA_L, DATA_U: bitvec[0x4000];
        attribute DATAP_L, DATAP_U: bitvec[0x800];
        attribute SAVEDATA: bitvec[128];

        attribute INIT_A_L, INIT_A_U, INIT_B_L, INIT_B_U: bitvec[18];
        attribute SRVAL_A_L, SRVAL_A_U, SRVAL_B_L, SRVAL_B_U: bitvec[18];
        attribute READ_WIDTH_A_L, READ_WIDTH_A_U, READ_WIDTH_B_L, READ_WIDTH_B_U: BRAM_V5_DATA_WIDTH;
        attribute READ_MUX_UL_A, READ_MUX_UL_B: bool;
        attribute READ_SDP_L, READ_SDP_U: bool;
        attribute WRITE_WIDTH_A_L, WRITE_WIDTH_A_U, WRITE_WIDTH_B_L, WRITE_WIDTH_B_U: BRAM_V5_DATA_WIDTH;
        attribute WRITE_MUX_UL_A, WRITE_MUX_UL_B: bool;
        attribute WRITE_SDP_L, WRITE_SDP_U: bool;
        attribute WRITE_MODE_A_L, WRITE_MODE_A_U, WRITE_MODE_B_L, WRITE_MODE_B_U: BRAM_WRITE_MODE;

        attribute WW_VALUE: BRAM_WW_VALUE;

        attribute DOA_REG_L, DOA_REG_U, DOB_REG_L, DOB_REG_U: bool;

        attribute RAM_EXTENSION_A_LOWER: bool;
        attribute RAM_EXTENSION_B_LOWER: bool;

        attribute EN_ECC_READ: bool;
        attribute EN_ECC_SCRUB: bool;
        attribute EN_ECC_WRITE: bool;
        attribute EN_ECC_WRITE_NO_READ: bool;

        attribute FIFO_ENABLE_L, FIFO_ENABLE_U: bool;
        attribute FIFO_WIDTH: BRAM_V5_FIFO_WIDTH;
        attribute FIRST_WORD_FALL_THROUGH: bool;
        attribute EN_SYN: bool;
        attribute ALMOST_EMPTY_OFFSET: bitvec[13];
        attribute ALMOST_FULL_OFFSET: bitvec[13];

        attribute BYPASS_RSR: bool;
        attribute SWAP_CFGPORT: bool;
        attribute TRD_DLY_L, TRD_DLY_U: bitvec[3];
        attribute TSCRUB_DLY_L, TSCRUB_DLY_U: bitvec[1];
        attribute TWR_DLY_L, TWR_DLY_U: bitvec[4];

        attribute TEST_FIFO_CNT: bool;
        attribute TEST_FIFO_FLAG: bool;
        attribute TEST_FIFO_OFFSET: bool;
    }

    // TODO BRAM_V6

    bel_class PMVBRAM_V5 {
        input DISABLE0;
        input DISABLE1;
        output O;
        output ODIV2;
        output ODIV4;
    }

    bel_class PMVBRAM_V6 {
        input SELECT1, SELECT2, SELECT3, SELECT4;
        output O;
        output ODIV2;
        output ODIV4;
    }

    enum DSP_AB_INPUT { DIRECT, CASCADE }
    enum DSP_REG2 { _0, _1, _2 }
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
        attribute B_INPUT: DSP_AB_INPUT;
        attribute UNK_ENABLE: bool;
    }

    bel_class DSP_C {
        input C[48];
        input CEC;
        input RSTC;

        attribute MUX_CLK: bitvec[1];
        attribute CREG: bool;
    }

    enum DSP_REG2_CASC { NONE, _0, _1, _2, DIRECT_2_CASC_1 }
    enum DSP_USE_MULT { NONE, MULT, MULT_S }
    // TODO: pretty obviously the three bits correspond to the three places the carry chain can be broken; nail them down
    enum DSP_USE_SIMD { ONE48, TWO24, FOUR12 }
    enum DSP_SEL_PATTERN { PATTERN, C }
    enum DSP_SEL_MASK { MASK, C }
    enum DSP_SEL_ROUNDING_MASK { SEL_MASK, MODE1, MODE2 }
    bel_class DSP_V5 {
        input CLK;

        input A[30];
        input RSTA, CEA1, CEA2;
        attribute AREG: DSP_REG2_CASC;
        attribute A_INPUT: DSP_AB_INPUT;

        input B[18];
        input RSTB, CEB1, CEB2;
        attribute BREG: DSP_REG2_CASC;
        attribute B_INPUT: DSP_AB_INPUT;

        input C[48];
        input RSTC, CEC;
        attribute CREG: bool;

        input RSTM, CEM;
        attribute MREG: bool;
        attribute CLOCK_INVERT_M: bool;
        attribute USE_MULT: DSP_USE_MULT;

        input OPMODE[7];
        input CARRYINSEL[3];
        input RSTCTRL, CECTRL;
        attribute OPMODEREG: bool;
        attribute CARRYINSELREG: bool;

        input CARRYIN;
        input RSTALLCARRYIN, CECARRYIN, CEMULTCARRYIN;
           attribute CARRYINREG: bool;
        attribute MULTCARRYINREG: bool;

        input RSTALUMODE, CEALUMODE;
        input ALUMODE[4];
        attribute ALUMODEREG: bool;
        attribute USE_SIMD: DSP_USE_SIMD;

        input RSTP, CEP;
        output P[48];
        output CARRYOUT[4];
        attribute PREG: bool;
        attribute CLOCK_INVERT_P: bool;

        attribute USE_PATTERN_DETECT: bool;
        attribute PATTERN: bitvec[48];
        attribute SEL_PATTERN: DSP_SEL_PATTERN;
        attribute MASK: bitvec[48];
        attribute SEL_MASK: DSP_SEL_MASK;
        attribute SEL_ROUNDING_MASK: DSP_SEL_ROUNDING_MASK;
        attribute ROUNDING_LSB_MASK: bitvec[1];
        output PATTERNDETECT, PATTERNBDETECT;
        output OVERFLOW, UNDERFLOW;

        attribute AUTORESET_OVER_UNDER_FLOW: bool;
        attribute AUTORESET_PATTERN_DETECT: bool;
        attribute AUTORESET_PATTERN_DETECT_OPTINV: bool;

        // TODO: is this SET stuff like a INT | CONST_0 | CONST_1 mux?
        input LFSREN;
        attribute LFSR_EN_SET: bool;
        attribute LFSR_EN_SETVAL: bitvec[1];
        input TESTM, TESTP;
        attribute TEST_SET_M: bool;
        attribute TEST_SET_P: bool;
        attribute TEST_SETVAL_M: bitvec[1];
        attribute TEST_SETVAL_P: bitvec[1];
        input SCANINM, SCANINP;
        attribute SCAN_IN_SET_M: bool;
        attribute SCAN_IN_SET_P: bool;
        attribute SCAN_IN_SETVAL_M: bitvec[1];
        attribute SCAN_IN_SETVAL_P: bitvec[1];
        output SCANOUTM, SCANOUTP;
    }

    // TODO: DSP_V6

    enum IO_DATA_RATE { SDR, DDR }
    // 14 is virtex7 only
    enum IO_DATA_WIDTH { NONE, _2, _3, _4, _5, _6, _7, _8, _10, _14 }
    enum IO_SERDES_MODE { MASTER, SLAVE }
    enum ILOGIC_MUX_TSBYPASS { GND, T }
    enum ILOGIC_INTERFACE_TYPE {
        MEMORY,
        NETWORKING,
        // virtex6 and up only
        OVERSAMPLE,
        MEMORY_DDR3_V6,
        // virtex7 only
        MEMORY_DDR3_V7,
    }
    enum ILOGIC_DDR_CLK_EDGE { SAME_EDGE_PIPELINED, SAME_EDGE, OPPOSITE_EDGE }
    enum ILOGIC_IDELAYMUX { NONE, D, OFB }
    enum ILOGIC_IOBDELAY_TYPE { DEFAULT, FIXED, VARIABLE }
    enum ILOGIC_NUM_CE { _1, _2 }
    bel_class ILOGIC {
        // CLKB is virtex5 and up only
        input CLK, CLKB, CLKDIV;
        input SR, REV;
        input CE1, CE2;
        input BITSLIP;
        // these three not present on virtex5 and up (moved into the IODELAY bel)
        input DLYCE, DLYINC, DLYRST;
        output O;
        // Q7 and Q8 are vitex7 only
        output Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8;
        output CLKPAD;

        // virtex6 and up only
        input DYNCLKSEL, DYNCLKDIVSEL;
        // virtex6 only
        input DYNOCLKSEL;
        // virtex7 only
        input DYNCLKDIVPSEL;

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
        attribute FFI_SR_SYNC: bool;
        // umm. those are not present at all on virtex4. shouldn't there like. be some way to enable them?
        // or disable, as the case may be?
        attribute FFI_SR_ENABLE: bool;
        attribute FFI_REV_ENABLE: bool;

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

        // these four not present on virtex5 and up (moved into the IODELAY bel)
        attribute IDELAYMUX: ILOGIC_IDELAYMUX;
        attribute IOBDELAY_TYPE: ILOGIC_IOBDELAY_TYPE;
        attribute IOBDELAY_VALUE_CUR: bitvec[6];
        attribute IOBDELAY_VALUE_INIT: bitvec[6];

        attribute READBACK_I: bitvec[1];
    }

    enum IODELAY_V5_DELAY_SRC { NONE, I, IO, O, DATAIN }
    enum IODELAY_V5_IDELAY_TYPE { FIXED, VARIABLE, DEFAULT }
    bel_class IODELAY_V5 {
        // C is tied to ILOGIC.CLKDIV
        input CE, DATAIN, INC, RST;

        // ??? why? and why inverted?
        // good start, isn't it
        attribute ENABLE: bitvec[4];
        attribute DELAY_SRC: IODELAY_V5_DELAY_SRC;
        attribute DELAYCHAIN_OSC: bool;
        attribute HIGH_PERFORMANCE_MODE: bool;
        attribute LEGIDELAY: bool;
        attribute IDELAY_TYPE: IODELAY_V5_IDELAY_TYPE;
        attribute IDELAY_VALUE_CUR: bitvec[6];
        attribute IDELAY_VALUE_INIT: bitvec[6];
        attribute ODELAY_VALUE: bitvec[6];
    }
    device_data IODELAY_V5_IDELAY_DEFAULT: bitvec[6];

    bel_class IODELAY_V6 {
        // C is still tied to ILOGIC.CLKDIV, but now with separate inversion, so...
        input C, CINVCTRL;
        input CE, DATAIN, INC, RST;
        input CNTVALUEIN[5];
        output CNTVALUEOUT[5];
    }
    device_data IODELAY_V6_IDELAY_DEFAULT: bitvec[5];

    // 2 no longer supported on virtex5
    enum OLOGIC_TRISTATE_WIDTH { _1, _2, _4 }
    enum OLOGIC_V4_MUX_O { NONE, D1, FFO1, FFODDR }
    enum OLOGIC_V4_MUX_T { NONE, T1, FFT1, FFTDDR }
    enum OLOGIC_V5_MUX_O { NONE, D1, SERDES_SDR, SERDES_DDR, LATCH, FF, DDR }
    enum OLOGIC_V5_MUX_T { NONE, T1, SERDES_SDR, SERDES_DDR, LATCH, FF, DDR }
    enum OLOGIC_MISR_CLK_SELECT { NONE, CLK1, CLK2 }
    bel_class OLOGIC {
        // CLKB, CLKDIVB are virtex6 and up only
        // CLKPERF is virtex6 only
        input CLK, CLKB, CLKDIV, CLKDIVB, CLKPERF;
        input SR, REV;
        input OCE, TCE;
        input D1, D2, D3, D4, D5, D6;
        input T1, T2, T3, T4;
        output TQ;

        // virtex6 and up
        output TFB;
        output IOCLKGLITCH;
        // virtex6 only
        input ODV, WC;
        output OCBEXTEND;

        attribute CLK1_INV: bool;
        attribute CLK2_INV: bool;

        // ??? what
        attribute FFO_INIT: bitvec[4];
        attribute FFO_INIT_SERDES: bitvec[3];
        attribute FFO_SRVAL: bitvec[3];
        attribute FFO_SERDES: bitvec[4];
        // merged into MUX_O on virtex5
        attribute FFO_LATCH: bool;
        attribute FFO_SR_SYNC: bitvec[4];
        attribute FFO_SR_ENABLE: bool;
        attribute FFO_REV_ENABLE: bool;
        attribute V4_MUX_O: OLOGIC_V4_MUX_O;
        attribute V5_MUX_O: OLOGIC_V5_MUX_O;

        attribute FFT_INIT: bitvec[5];
        attribute FFT1_SRVAL: bitvec[1];
        attribute FFT2_SRVAL: bitvec[1];
        attribute FFT3_SRVAL: bitvec[1];
        // merged into MUX_T on virtex5
        attribute FFT_LATCH: bool;
        attribute FFT_SR_SYNC: bitvec[2];
        attribute FFT_SR_ENABLE: bool;
        attribute FFT_REV_ENABLE: bool;
        attribute V4_MUX_T: OLOGIC_V4_MUX_T;
        attribute V5_MUX_T: OLOGIC_V5_MUX_T;

        attribute INIT_LOADCNT: bitvec[4];

        attribute SERDES: bool;
        attribute SERDES_MODE: IO_SERDES_MODE;
        attribute DATA_WIDTH: IO_DATA_WIDTH;
        attribute TRISTATE_WIDTH: OLOGIC_TRISTATE_WIDTH;

        // virtex5 only
        attribute MISR_ENABLE: bool;
        attribute MISR_ENABLE_FDBK: bool;
        attribute MISR_RESET: bool;
        attribute MISR_CLK_SELECT: OLOGIC_MISR_CLK_SELECT;
    }

    enum IOB_PULL { NONE, PULLUP, PULLDOWN, KEEPER }
    enum IOB_IBUF_MODE { NONE, VREF, DIFF, CMOS }
    enum IOB_DCI_MODE { NONE, OUTPUT, OUTPUT_HALF, TERM_VCC, TERM_SPLIT }

    bel_class IOB {
        // virtex6 and up only
        input PD_INT_EN, PU_INT_EN, KEEPER_INT_EN;
        input DIFF_TERM_INT_EN;
        // virtex7 only
        input IBUFDISABLE;
        input DCITERMDISABLE;

        pad PAD: inout;

        attribute PULL: IOB_PULL;
        attribute VREF_SYSMON: bool;
        attribute VR: bool;

        attribute IBUF_MODE: IOB_IBUF_MODE;
        // virtex5 only
        attribute I_INV: bool;

        attribute OUTPUT_ENABLE: bitvec[2];
        // virtex5 and up only
        attribute OUTPUT_DELAY: bool;
        attribute DCI_MODE: IOB_DCI_MODE;
        attribute DCI_MISC: bitvec[2];
        attribute DCI_T: bool;
        attribute DCIUPDATEMODE_ASREQUIRED: bool;

        attribute V4_PDRIVE: bitvec[5];
        attribute V4_NDRIVE: bitvec[5];
        attribute V4_PSLEW: bitvec[4];
        attribute V4_NSLEW: bitvec[4];
        attribute V4_OUTPUT_MISC: bitvec[2];
        attribute V4_LVDS: bitvec[4];

        // reuse same-size V4_*DRIVE
        attribute V5_PSLEW: bitvec[6];
        attribute V5_NSLEW: bitvec[6];
        attribute V5_OUTPUT_MISC: bitvec[6];
        attribute V5_LVDS: bitvec[9];
    }

    if variant [virtex4, virtex5] {
        table IOB_DATA {
            field PDRIVE: bitvec[5];
            field NDRIVE: bitvec[5];
            if variant virtex4  {
                field OUTPUT_MISC: bitvec[2];
                field PSLEW_FAST: bitvec[4];
                field NSLEW_FAST: bitvec[4];
                field PSLEW_SLOW: bitvec[4];
                field NSLEW_SLOW: bitvec[4];
            } else {
                field OUTPUT_MISC: bitvec[6];
                field PSLEW_FAST: bitvec[6];
                field NSLEW_FAST: bitvec[6];
                field PSLEW_SLOW: bitvec[6];
                field NSLEW_SLOW: bitvec[6];
            }
            field PMASK_TERM_VCC: bitvec[5];
            field PMASK_TERM_SPLIT: bitvec[5];
            field NMASK_TERM_SPLIT: bitvec[5];
            if variant virtex4 {
                field LVDIV2: bitvec[2];
            } else {
                field LVDIV2: bitvec[3];
            }

            row OFF, VREF, VR;

            // push-pull I/O standards
            if variant virtex5 {
                row LVCMOS12_2, LVCMOS12_4, LVCMOS12_6, LVCMOS12_8;
            }
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
            row HSTL_I_DCI, HSTL_II_DCI, HSTL_II_T_DCI, HSTL_III_DCI, HSTL_IV_DCI;
            row HSTL_I_DCI_18, HSTL_II_DCI_18, HSTL_II_T_DCI_18, HSTL_III_DCI_18, HSTL_IV_DCI_18;

            // pseudo-differential
            row BLVDS_25;
            row LVPECL_25;

            if variant virtex4 {
                // DCI term for true differential
                row LVDS_25_DCI, LVDSEXT_25_DCI;
            }
        }
    }
    if variant virtex4 {
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
    } else if variant virtex5 {
        table LVDS_DATA {
            field OUTPUT_T: bitvec[9];
            field OUTPUT_C: bitvec[9];
            field TERM_T: bitvec[9];
            field TERM_C: bitvec[9];
            field LVDSBIAS: bitvec[12];

            row OFF;
            row LVDS_25;
            row LVDSEXT_25;
            row RSDS_25;
            row HT_25;
        }
    }

    bel_class GLOBALSIG {
    }

    bel_class HCLK_CMT_DRP {
        attribute DRP_MASK: bool;
    }

    bel_class HCLK_DRP_V6 {
        attribute DRP_MASK_S: bool;
        attribute DRP_MASK_N: bool;
        attribute DRP_MASK_SYSMON: bool;
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

    bel_class BUFHCE {
        input I;
        input CE;
        output O;

        attribute ENABLE: bool;
        attribute INIT_OUT: bitvec[1];
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
        attribute BIAS_MODE: bitvec[1];
    }

    // used for virtex4 and virtex5
    bel_class DCI {
        input TSTCLK, TSTRST;
        input TSTHLP, TSTHLN;
        // ??? the following outputs (except DCIDONE) exist on virtex6 and up, but are not connected to anything?
        output DCISCLK;
        output DCIADDRESS[3];
        output DCIDATA;
        output DCIIOUPDATE;
        output DCIREFIOUPDATE;
        output DCIDONE;
        // virtex6 and up only
        input INT_DCI_EN;

        attribute ENABLE: bool;
        attribute QUIET: bool;

        attribute V4_LVDIV2: bitvec[2];
        attribute V5_LVDIV2: bitvec[3];
        attribute V4_PMASK_TERM_VCC: bitvec[5];
        attribute V4_PMASK_TERM_SPLIT: bitvec[5];
        attribute V4_NMASK_TERM_SPLIT: bitvec[5];
        attribute V6_PMASK_TERM_VCC: bitvec[6];
        attribute V6_PMASK_TERM_SPLIT: bitvec[6];
        attribute V6_NMASK_TERM_SPLIT: bitvec[6];

        // not present on virtex6 and up (replaced by other attrs?)
        attribute NREF: bitvec[2];
        attribute PREF: bitvec[4];

        attribute TEST_ENABLE: bitvec[2];
        attribute CASCADE_FROM_ABOVE: bool;
        attribute CASCADE_FROM_BELOW: bool;

        // virtex6 and up only from now on
        attribute DYNAMIC_ENABLE: bool;
        attribute NREF_OUTPUT: bitvec[2];
        attribute NREF_OUTPUT_HALF: bitvec[3];
        attribute NREF_TERM_SPLIT: bitvec[3];
        attribute PREF_OUTPUT: bitvec[2];
        attribute PREF_OUTPUT_HALF: bitvec[3];
        attribute PREF_TERM_VCC: bitvec[2];
        attribute PREF_TERM_SPLIT: bitvec[3];
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
        attribute V5_LVDSBIAS: bitvec[12];
        attribute V6_LVDSBIAS: bitvec[17];
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

    enum DCM_IODLY_MUX { PASS, DELAY_LINE }
    enum DCM_DLL_SYNTH_CLOCK_SPEED { NORMAL, HALF, QUARTER, VDD }
    bel_class DCM_V5 {
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

        input FREEZEDLL, FREEZEDFS;
        input CTLMODE, CTLGO, CTLOSC1, CTLOSC2, CTLSEL[3];

        input SKEWCLKIN1, SKEWCLKIN2;
        input SKEWIN;
        input SKEWRST;
        output SKEWOUT;

        input SCANIN[5];
        output SCANOUT[2];

        // address 0x40 and up
        attribute DRP: bitvec[16][24];

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
        attribute DESKEW_ADJUST: bitvec[5];
        attribute CLKIN_DIVIDE_BY_2: bool;
        attribute CLKIN_CLKFB_ENABLE: bool;

        attribute CLKFX_MULTIPLY: bitvec[8];
        attribute CLKFX_DIVIDE: bitvec[8];

        attribute FACTORY_JF: bitvec[16];
        attribute PHASE_SHIFT: bitvec[10];
        attribute PHASE_SHIFT_NEGATIVE: bool;
        attribute PS_CENTERED: bool;
        attribute PS_DIRECT: bool;
        attribute PS_ENABLE: bool;
        attribute PS_MODE: DCM_PS_MODE;

        attribute DCM_CLKDV_CLKFX_ALIGNMENT: bool;
        attribute DCM_CLKFB_IODLY_MUXINSEL: DCM_IODLY_MUX;
        attribute DCM_CLKFB_IODLY_MUXOUT_SEL: DCM_IODLY_MUX;
        attribute DCM_CLKIN_IODLY_MUXINSEL: DCM_IODLY_MUX;
        attribute DCM_CLKIN_IODLY_MUXOUT_SEL: DCM_IODLY_MUX;
        attribute DCM_CLKLOST_EN: bool;
        attribute DCM_COMMON_MSB_SEL: bitvec[2];
        attribute DCM_COM_PWC_FB_EN: bool;
        attribute DCM_COM_PWC_FB_TAP: bitvec[3];
        attribute DCM_COM_PWC_REF_EN: bool;
        attribute DCM_COM_PWC_REF_TAP: bitvec[3];
        attribute DCM_EXT_FB_EN: bool;
        attribute DCM_LOCK_HIGH_B: bool;
        attribute DCM_PLL_RST_DCM: bool;
        attribute DCM_POWERDOWN_COMMON_EN_B: bool;
        attribute DCM_REG_PWRD_CFG: bool;
        attribute DCM_SCANMODE: bool;
        attribute DCM_TRIM_CAL: bitvec[3];
        attribute DCM_UNUSED_TAPS_POWERDOWN: bitvec[4];
        attribute DCM_USE_REG_READY: bool;
        attribute DCM_VREG_ENABLE: bool;
        attribute DCM_VBG_PD: bitvec[2];
        attribute DCM_VBG_SEL: bitvec[4];
        attribute DCM_VSPLY_VALID_ACC: bitvec[2];
        attribute DCM_WAIT_PLL: bool;

        attribute DLL_CLKFB_STOPPED_PWRD_EN_B: bool;
        attribute DLL_CLKIN_STOPPED_PWRD_EN_B: bool;
        attribute DLL_DEAD_TIME: bitvec[8];
        attribute DLL_DESKEW_LOCK_BY1: bool;
        attribute DLL_DESKEW_MAXTAP: bitvec[8];
        attribute DLL_DESKEW_MINTAP: bitvec[8];
        attribute DLL_ETPP_HOLD: bool;
        attribute DLL_FDBKLOST_EN: bool;
        attribute DLL_FREQUENCY_MODE: DCM_DLL_FREQUENCY_MODE;
        attribute DLL_LIVE_TIME: bitvec[8];
        attribute DLL_PERIOD_LOCK_BY1: bool;
        attribute DLL_PHASE_SHIFT_CALIBRATION: DCM_DLL_PHASE_SHIFT_CALIBRATION;
        attribute DLL_PHASE_SHIFT_LFC: bitvec[9];
        attribute DLL_PHASE_SHIFT_LOCK_BY1: bool;
        attribute DLL_PWRD_STICKY_B: bool;
        attribute DLL_PWRD_ON_SCANMODE_B: bool;
        attribute DLL_SETTLE_TIME: bitvec[8];
        attribute DLL_SYNTH_CLOCK_SPEED: DCM_DLL_SYNTH_CLOCK_SPEED;
        attribute DLL_TAPINIT_CTL: bitvec[3];
        attribute DLL_TEST_MUX_SEL: bitvec[2];
        attribute DLL_ZD1_EN: bool;
        attribute DLL_ZD1_JF_OVERFLOW_HOLD: bool;
        attribute DLL_ZD1_PHASE_SEL_INIT: bitvec[2];
        attribute DLL_ZD1_PWC_EN: bool;
        attribute DLL_ZD1_PWC_TAP: bitvec[3];
        attribute DLL_ZD1_TAP_INIT: bitvec[8];
        attribute DLL_ZD2_EN: bool;
        attribute DLL_ZD2_JF_OVERFLOW_HOLD: bool;
        attribute DLL_ZD2_PWC_EN: bool;
        attribute DLL_ZD2_PWC_TAP: bitvec[3];
        attribute DLL_ZD2_TAP_INIT: bitvec[7];

        attribute DFS_AVE_FREQ_GAIN: DCM_DFS_AVE_FREQ_GAIN;
        attribute DFS_AVE_FREQ_SAMPLE_INTERVAL: bitvec[3];
        attribute DFS_CFG_BYPASS: bool;
        attribute DFS_CUSTOM_FAST_SYNC: bitvec[4];
        attribute DFS_EARLY_LOCK: bool;
        attribute DFS_EN: bool;
        attribute DFS_EN_RELRST_B: bool;
        attribute DFS_FAST_UPDATE: bool;
        attribute DFS_FREQUENCY_MODE: DCM_DFS_FREQUENCY_MODE;
        attribute DFS_HARDSYNC_B: bitvec[2];
        attribute DFS_HF_TRIM_CAL: bitvec[3];
        attribute DFS_JF_LOWER_LIMIT: bitvec[4];
        attribute DFS_MPW_LOW: bool;
        attribute DFS_MPW_HIGH: bool;
        attribute DFS_OSC_ON_FX: bool;
        attribute DFS_OSCILLATOR_MODE: DCM_DFS_OSCILLATOR_MODE;
        attribute DFS_OUTPUT_PSDLY_ON_CONCUR: bool;
        attribute DFS_PWRD_CLKIN_STOP_B: bool;
        attribute DFS_PWRD_CLKIN_STOP_STICKY_B: bool;
        attribute DFS_PWRD_REPLY_TIMES_OUT_B: bool;
        attribute DFS_REF_ON_FX: bool;
        attribute DFS_SYNC_TO_DLL: bool;
        attribute DFS_SYNTH_CLOCK_SPEED: bitvec[3];
        attribute DFS_SYNTH_FAST_SYNCH: bitvec[2];
        attribute DFS_TAPTRIM: bitvec[11];
        attribute DFS_TWEAK: bitvec[8];
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

    bel_class PLL_V5 {
        input CLKIN1, CLKIN2;
        input CLKINSEL;
        output TEST_CLKIN;
        input CLKFBIN;

        output CLKOUT0, CLKOUT1, CLKOUT2, CLKOUT3, CLKOUT4, CLKOUT5, CLKFBOUT;
        output CLKOUTDCM0, CLKOUTDCM1, CLKOUTDCM2, CLKOUTDCM3, CLKOUTDCM4, CLKOUTDCM5, CLKFBDCM;

        input RST;
        input REL;
        output LOCKED;

        input DCLK;
        input DEN;
        input DWE;
        input DADDR[5];
        input DI[16];
        output DRDY;
        output DO[16];

        input CLKBRST;
        input ENOUTSYNC;
        input MANPDLF, MANPULF;

        input SKEWRST;
        input SKEWSTB;
        input SKEWCLKIN1, SKEWCLKIN2;

        output TEST[35];

        // address 0x00 and up
        attribute DRP: bitvec[16][32];

        attribute CLKINSEL_STATIC_VAL: bitvec[1];
        attribute CLKINSEL_MODE_DYNAMIC: bool;

        attribute CLKOUT0_DESKEW_ADJUST: bitvec[5];
        attribute CLKOUT1_DESKEW_ADJUST: bitvec[5];
        attribute CLKOUT2_DESKEW_ADJUST: bitvec[5];
        attribute CLKOUT3_DESKEW_ADJUST: bitvec[5];
        attribute CLKOUT4_DESKEW_ADJUST: bitvec[5];
        attribute CLKOUT5_DESKEW_ADJUST: bitvec[5];
        attribute CLKFBOUT_DESKEW_ADJUST: bitvec[5];

        attribute PLL_AVDD_COMP_SET: bitvec[2];
        attribute PLL_AVDD_VBG_PD: bitvec[2];
        attribute PLL_AVDD_VBG_SEL: bitvec[4];
        attribute PLL_CLKCNTRL: bitvec[1];
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
        attribute PLL_CP: bitvec[4];
        attribute PLL_CP_BIAS_TRIP_SHIFT: bool;
        attribute PLL_CP_RES: bitvec[2];
        attribute PLL_DIRECT_PATH_CNTRL: bool;
        attribute PLL_DIVCLK_DT: bitvec[6];
        attribute PLL_DIVCLK_EDGE: bool;
        attribute PLL_DIVCLK_EN: bool;
        attribute PLL_DIVCLK_HT: bitvec[6];
        attribute PLL_DIVCLK_LT: bitvec[6];
        attribute PLL_DIVCLK_NOCOUNT: bool;
        attribute PLL_DVDD_COMP_SET: bitvec[2];
        attribute PLL_DVDD_VBG_PD: bitvec[2];
        attribute PLL_DVDD_VBG_SEL: bitvec[4];
        attribute PLL_EN: bool;
        attribute PLL_EN_CNTRL: bitvec[78];
        attribute PLL_EN_DLY: bool;
        attribute PLL_EN_TCLK0: bool;
        attribute PLL_EN_TCLK1: bool;
        attribute PLL_EN_TCLK2: bool;
        attribute PLL_EN_TCLK3: bool;
        attribute PLL_EN_TCLK4: bool;
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
        attribute PLL_FLOCK: bitvec[6];
        attribute PLL_INC_FLOCK: bool;
        attribute PLL_INC_SLOCK: bool;
        attribute PLL_INTFB: bitvec[2];
        attribute PLL_IN_DLY_MX_SEL: bitvec[5];
        attribute PLL_IN_DLY_SET: bitvec[9];
        attribute PLL_LFHF: bitvec[2];
        attribute PLL_LF_NEN: bitvec[2];
        attribute PLL_LF_PEN: bitvec[2];
        attribute PLL_LOCK_CNT: bitvec[6];
        attribute PLL_LOCK_CNT_RST_FAST: bool;
        attribute PLL_LOCK_FB_P1: bitvec[5];
        attribute PLL_LOCK_FB_P2: bitvec[5];
        attribute PLL_LOCK_REF_P1: bitvec[5];
        attribute PLL_LOCK_REF_P2: bitvec[5];
        attribute PLL_MAN_LF_EN: bool;
        attribute PLL_MISC: bitvec[4];
        attribute PLL_NBTI_EN: bool;
        attribute PLL_PFD_CNTRL: bitvec[4];
        attribute PLL_PFD_DLY: bitvec[2];
        attribute PLL_PMCD_MODE: bool;
        attribute PLL_PWRD_CFG: bool;
        attribute PLL_RES: bitvec[4];
        attribute PLL_SEL_SLIPD: bool;
        attribute PLL_TCK4_SEL: bitvec[1];
        attribute PLL_UNLOCK_CNT: bitvec[4];
        attribute PLL_UNLOCK_CNT_RST_FAST: bool;
        attribute PLL_VLFHIGH_DIS: bool;
    }
    device_data PLL_IN_DLY_SET: bitvec[9];
    if variant virtex5 {
        table PLL_MULT {
            field PLL_CP_LOW, PLL_CP_HIGH: bitvec[4];
            field PLL_RES_LOW, PLL_RES_HIGH: bitvec[4];
            field PLL_LFHF_LOW, PLL_LFHF_HIGH: bitvec[2];

            for i in 1..=64 {
                row "_{i}";
            }
        }
    }

    bel_class PLL_V6 {
        input CLKIN1, CLKIN2;
        input CLKINSEL;
        input CLKFBIN;
        input CLKIN_CASC, CLKFB_CASC;

        output
            CLKOUT0, CLKOUT0B, CLKOUT1, CLKOUT1B,
            CLKOUT2, CLKOUT2B, CLKOUT3, CLKOUT3B,
            CLKOUT4, CLKOUT5, CLKOUT6, CLKFBOUT, CLKFBOUTB, TMUXOUT;

        input RST;
        input PWRDWN;
        output LOCKED;

        input DCLK;
        input DEN;
        input DWE;
        input DADDR[7];
        input DI[16];
        output DRDY;
        output DO[16];

        input PSCLK;
        output PSDONE;
        input PSEN;
        input PSINCDEC;

        output CLKINSTOPPED, CLKFBSTOPPED;
        input TESTIN[32];
        output TESTOUT[64];

        // 0x00..0x80
        attribute DRP: bitvec[16][128];

        // TODO: attributes
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

        pad VP, VN: input;
        pad VREFP, VREFN: analog;
        pad AVSS, AVDD: power;

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

        pad VP, VN: input;
        pad VREFP, VREFN: analog;
        pad AVSS, AVDD: power;

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

        // virtex5 and up only
        output CFGCLK, CFGMCLK, DINSPI, TCKSPI;

        // virtex6 and up only
        output PREQ;
        input PACK;
        input KEYCLEARB;

        attribute USER_GTS_GSR_ENABLE: bool;
        attribute GTS_SYNC: bool;
        attribute GSR_SYNC: bool;
        // virtex4 only
        attribute GWE_SYNC: bool;
        attribute USRCCLK_ENABLE: bool;

        // virtex6, virtex7
        attribute USER_GTS_GSR_ENABLE_TR: bitvec[3];
        attribute KEY_CLEAR_ENABLE_TR: bitvec[3];
        attribute PROG_USR_TR: bitvec[3];
        attribute USRCCLK_ENABLE_TR: bitvec[3];
    }

    bel_class CAPTURE {
        input CLK;
        input CAP;
    }

    bel_class ICAP_V4 {
        input CLK;
        input CE;
        input WRITE;
        input I[32];
        output BUSY;
        output O[32];

        attribute ENABLE: bool;
    }

    bel_class ICAP_V6 {
        input CLK;
        input CSB;
        input RDWRB;
        input I[32];
        output BUSY;
        output O[32];

        attribute ENABLE_TR: bitvec[3];
    }

    bel_class BSCAN {
        input TDO;
        output DRCK;
        output SEL;
        output TDI;
        output RESET, CAPTURE, SHIFT, UPDATE;
        // virtex6 and up only
        output RUNTEST;
        output TCK, TMS;

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

    bel_class PMVIOB {
        input EN;
        input A[2];
        output O, ODIV2, ODIV4;

        // virtex6 only
        attribute PSLEW4_IN: bool;
        attribute HSLEW4_IN: bool;
        attribute HYS_IN: bool;
    }

    bel_class PMV2 {
        input EN;
        input A[3];
        output O, ODIV2, ODIV4;
    }

    bel_class MTBF2 {
        input CLK, EN, RESET, DIN;
        output Q0B, Q1B, Q2B, Q3B, Q4B, Q5B, Q6B, Q7B;
    }

    bel_class DCIRESET {
        input RST;
        output LOCKED;

        // virtex4, virtex5
        attribute ENABLE: bool;
        // virtex6, virtex7
        attribute ENABLE_TR: bitvec[3];
    }

    bel_class FRAME_ECC_V4 {
        output ERROR;
        output SYNDROMEVALID;
        output SYNDROME[12];

        // virtex5 only
        output CRCERROR, ECCERROR;
    }

    bel_class FRAME_ECC_V6 {
        output ERROR, CRCERROR, ECCERROR, ECCERRORSINGLE;
        output SYNDROMEVALID;
        output SYNDROME[13];
        output SYNBIT[5];
        output SYNWORD[7];
        // virtex6 uses only 24 bits; virtex7 uses 26
        output FAR[26];
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

    bel_class DNA_PORT {
        input CLK, READ, SHIFT, DIN;
        output DOUT;
        attribute ENABLE_TR: bitvec[3];
    }

    bel_class CFG_IO_ACCESS_V6 {
        input DOUT, TDO;
        output HSWAPEN, RDWRB;
        output MODE0, MODE1, MODE2;
        output VGGCOMPOUT;
        attribute ENABLE_TR: bitvec[3];
    }
    bel_class CFG_IO_ACCESS_V7 {
        input TDO;
        input INITBO;
        output INITBI;
        output CFGDATA[32];
        output PUDCB, CCLK, RDWRB, MASTER;
        output MODE0, MODE1, MODE2;
        output VGGCOMPOUT;
        attribute ENABLE_TR: bitvec[3];
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
        attribute DCI_CLK_ENABLE_TR: bitvec[6];
        attribute PROBESEL: PROBESEL;
        // virtex6 and up only
        attribute DISABLE_JTAG_TR: bitvec[3];

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

    bel_class PPR_FRAME {
        input CLK;
        input CTLB, ENB, SHIFTB, UPDATEB;
        input DA[80], DB[80], DH[2];
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

    bel_class PPC440 {
        input CPMPPCMPLBCLK;
        input PLBPPCMADDRACK;
        input PLBPPCMMBUSY;
        input PLBPPCMMIRQ;
        input PLBPPCMMRDERR;
        input PLBPPCMMWRERR;
        input PLBPPCMRDBTERM;
        input PLBPPCMRDDACK;
        input PLBPPCMRDDBUS[0:127];
        input PLBPPCMRDPENDPRI[0:1];
        input PLBPPCMRDPENDREQ;
        input PLBPPCMRDWDADDR[0:3];
        input PLBPPCMREARBITRATE;
        input PLBPPCMREQPRI[0:1];
        input PLBPPCMSSIZE[0:1];
        input PLBPPCMTIMEOUT;
        input PLBPPCMWRBTERM;
        input PLBPPCMWRDACK;
        input PLBPPCMWRPENDPRI[0:1];
        input PLBPPCMWRPENDREQ;
        output PPCMPLBABORT;
        output PPCMPLBABUS[0:31];
        output PPCMPLBBE[0:15];
        output PPCMPLBBUSLOCK;
        output PPCMPLBLOCKERR;
        output PPCMPLBPRIORITY[0:1];
        output PPCMPLBRDBURST;
        output PPCMPLBREQUEST;
        output PPCMPLBRNW;
        output PPCMPLBSIZE[0:3];
        output PPCMPLBTATTRIBUTE[0:15];
        output PPCMPLBTYPE[0:2];
        output PPCMPLBUABUS[28:31];
        output PPCMPLBWRBURST;
        output PPCMPLBWRDBUS[0:127];

        for i in 0..2 {
            input "CPMPPCS{i}PLBCLK";
            input "PLBPPCS{i}ABORT";
            input "PLBPPCS{i}ABUS"[0:31];
            input "PLBPPCS{i}BE"[0:15];
            input "PLBPPCS{i}BUSLOCK";
            input "PLBPPCS{i}LOCKERR";
            input "PLBPPCS{i}MASTERID"[0:1];
            input "PLBPPCS{i}MSIZE"[0:1];
            input "PLBPPCS{i}PAVALID";
            input "PLBPPCS{i}RDBURST";
            input "PLBPPCS{i}RDPENDPRI"[0:1];
            input "PLBPPCS{i}RDPENDREQ";
            input "PLBPPCS{i}RDPRIM";
            input "PLBPPCS{i}REQPRI"[0:1];
            input "PLBPPCS{i}RNW";
            input "PLBPPCS{i}SAVALID";
            input "PLBPPCS{i}SIZE"[0:3];
            input "PLBPPCS{i}TATTRIBUTE"[0:15];
            input "PLBPPCS{i}TYPE"[0:2];
            input "PLBPPCS{i}UABUS"[28:31];
            input "PLBPPCS{i}WRBURST";
            input "PLBPPCS{i}WRDBUS"[0:127];
            input "PLBPPCS{i}WRPENDPRI"[0:1];
            input "PLBPPCS{i}WRPENDREQ";
            input "PLBPPCS{i}WRPRIM";
            output "PPCS{i}PLBADDRACK";
            output "PPCS{i}PLBMBUSY"[0:3];
            output "PPCS{i}PLBMIRQ"[0:3];
            output "PPCS{i}PLBMRDERR"[0:3];
            output "PPCS{i}PLBMWRERR"[0:3];
            output "PPCS{i}PLBRDBTERM";
            output "PPCS{i}PLBRDCOMP";
            output "PPCS{i}PLBRDDACK";
            output "PPCS{i}PLBRDDBUS"[0:127];
            output "PPCS{i}PLBRDWDADDR"[0:3];
            output "PPCS{i}PLBREARBITRATE";
            output "PPCS{i}PLBSSIZE"[0:1];
            output "PPCS{i}PLBWAIT";
            output "PPCS{i}PLBWRBTERM";
            output "PPCS{i}PLBWRCOMP";
            output "PPCS{i}PLBWRDACK";
        }

        input CPMMCCLK;
        input MCMIADDRREADYTOACCEPT;
        input MCMIREADDATA[0:127];
        input MCMIREADDATAERR;
        input MCMIREADDATAVALID;
        output MIMCADDRESS[0:35];
        output MIMCADDRESSVALID;
        output MIMCBANKCONFLICT;
        output MIMCBYTEENABLE[0:15];
        output MIMCREADNOTWRITE;
        output MIMCROWCONFLICT;
        output MIMCWRITEDATA[0:127];
        output MIMCWRITEDATAVALID;

        input CPMC440CLK;
        input CPMC440CLKEN;
        input CPMC440CORECLOCKINACTIVE;
        input CPMC440TIMERCLOCK;
        input CPMINTERCONNECTCLK;
        input CPMINTERCONNECTCLKEN;
        input CPMINTERCONNECTCLKNTO1;
        input RSTC440RESETCHIP;
        input RSTC440RESETCORE;
        input RSTC440RESETSYSTEM;
        output C440RSTCHIPRESETREQ;
        output C440RSTCORERESETREQ;
        output C440RSTSYSTEMRESETREQ;
        output PPCCPMINTERCONNECTBUSY;
        output C440CPMCLOCKDCURDFB;
        output C440CPMCLOCKFB;
        output C440CPMCORESLEEPREQ;
        output C440CPMDECIRPTREQ;
        output C440CPMFITIRPTREQ;
        output C440CPMMSRCE;
        output C440CPMMSREE;
        output C440CPMTIMERRESETREQ;
        output C440CPMWDIRPTREQ;
        output C440MACHINECHECK;

        input CPMDCRCLK;
        input DCRPPCDMACK;
        input DCRPPCDMDBUSIN[0:31];
        input DCRPPCDMTIMEOUTWAIT;
        output PPCDMDCRABUS[0:9];
        output PPCDMDCRDBUSOUT[0:31];
        output PPCDMDCRREAD;
        output PPCDMDCRUABUS[20:21];
        output PPCDMDCRWRITE;
        input DCRPPCDSABUS[0:9];
        input DCRPPCDSDBUSOUT[0:31];
        input DCRPPCDSREAD;
        input DCRPPCDSWRITE;
        output PPCDSDCRACK;
        output PPCDSDCRDBUSIN[0:31];
        output PPCDSDCRTIMEOUTWAIT;

        input EICC440CRITIRQ;
        input EICC440EXTIRQ;
        output PPCEICINTERCONNECTIRQ;

        input JTGC440TCK;
        input JTGC440TDI;
        input JTGC440TMS;
        input JTGC440TRSTNEG;
        output C440JTGTDO;
        output C440JTGTDOEN;

        input DBGC440DEBUGHALT;
        input DBGC440SYSTEMSTATUS[0:4];
        input DBGC440UNCONDDEBUGEVENT;
        output C440DBGSYSTEMCONTROL[0:7];

        input TRCC440TRACEDISABLE;
        input TRCC440TRIGGEREVENTIN;
        output C440TRCBRANCHSTATUS[0:2];
        output C440TRCCYCLE;
        output C440TRCEXECUTIONSTATUS[0:4];
        output C440TRCTRACESTATUS[0:6];
        output C440TRCTRIGGEREVENTOUT;
        output C440TRCTRIGGEREVENTTYPE[0:13];

        input CPMFCMCLK;
        input FCMAPUCONFIRMINSTR;
        input FCMAPUCR[0:3];
        input FCMAPUDONE;
        input FCMAPUEXCEPTION;
        input FCMAPUFPSCRFEX;
        input FCMAPURESULT[0:31];
        input FCMAPURESULTVALID;
        input FCMAPUSLEEPNOTREADY;
        input FCMAPUSTOREDATA[0:127];
        output APUFCMDECFPUOP;
        output APUFCMDECLDSTXFERSIZE[0:2];
        output APUFCMDECLOAD;
        output APUFCMDECNONAUTON;
        output APUFCMDECSTORE;
        output APUFCMDECUDI[0:3];
        output APUFCMDECUDIVALID;
        output APUFCMENDIAN;
        output APUFCMFLUSH;
        output APUFCMINSTRUCTION[0:31];
        output APUFCMINSTRVALID;
        output APUFCMLOADBYTEADDR[0:3];
        output APUFCMLOADDATA[0:127];
        output APUFCMLOADDVALID;
        output APUFCMMSRFE[0:1];
        output APUFCMNEXTINSTRREADY;
        output APUFCMOPERANDVALID;
        output APUFCMRADATA[0:31];
        output APUFCMRBDATA[0:31];
        output APUFCMWRITEBACKOK;

        for i in 0..4 {
            input "CPMDMA{i}LLCLK";
            input "LLDMA{i}RSTENGINEREQ";
            input "LLDMA{i}RXD"[0:31];
            input "LLDMA{i}RXEOFN";
            input "LLDMA{i}RXEOPN";
            input "LLDMA{i}RXREM"[0:3];
            input "LLDMA{i}RXSOFN";
            input "LLDMA{i}RXSOPN";
            input "LLDMA{i}RXSRCRDYN";
            input "LLDMA{i}TXDSTRDYN";
            output "DMA{i}LLRSTENGINEACK";
            output "DMA{i}LLRXDSTRDYN";
            output "DMA{i}LLTXD"[0:31];
            output "DMA{i}LLTXEOFN";
            output "DMA{i}LLTXEOPN";
            output "DMA{i}LLTXREM"[0:3];
            output "DMA{i}LLTXSOFN";
            output "DMA{i}LLTXSOPN";
            output "DMA{i}LLTXSRCRDYN";
            output "DMA{i}RXIRQ";
            output "DMA{i}TXIRQ";
        }

        input TIEC440DCURDLDCACHEPLBPRIO[0:1];
        input TIEC440DCURDNONCACHEPLBPRIO[0:1];
        input TIEC440DCURDTOUCHPLBPRIO[0:1];
        input TIEC440DCURDURGENTPLBPRIO[0:1];
        input TIEC440DCUWRFLUSHPLBPRIO[0:1];
        input TIEC440DCUWRSTOREPLBPRIO[0:1];
        input TIEC440DCUWRURGENTPLBPRIO[0:1];
        input TIEC440ENDIANRESET;
        input TIEC440ERPNRESET[0:3];
        input TIEC440ICURDFETCHPLBPRIO[0:1];
        input TIEC440ICURDSPECPLBPRIO[0:1];
        input TIEC440ICURDTOUCHPLBPRIO[0:1];
        input TIEC440PIR[28:31];
        input TIEC440PVR[28:31];
        input TIEC440PVRTEST[0:27];
        input TIEC440USERRESET[0:3];
        input TIEDCRBASEADDR[0:1];
        input TIEPPCOPENLATCHN;
        input TIEPPCTESTENABLEN;

        input BISTC440ARRAYISOLATEN;
        input BISTC440BISTCLKENABLEN;
        input BISTC440BISTCLOCK;
        input BISTC440BISTMODE;
        input BISTC440BISTRESTARTN;
        input BISTC440BISTSTARTN;
        input BISTC440IRACCCLK;
        input BISTC440IRACCRST;
        input BISTC440IRACCSTARTN;
        input BISTC440LEAKAGETESTN;
        input BISTC440LRACCCLK;
        input BISTC440LRACCRST;
        input BISTC440LRACCSTARTN;
        output C440BISTDONE;
        output C440BISTFAILDCABOT;
        output C440BISTFAILDCATOP;
        output C440BISTFAILICABOT;
        output C440BISTFAILICATOP;
        output C440BISTFAILMMU;
        output C440BISTFAILSRAM;
        output C440BISTIRACCDONE;
        output C440BISTIRACCFAIL;
        output C440BISTLRACCDONE;
        output C440BISTLRACCFAIL;
        output C440BISTREALTIMEFAIL;

        input MBISTC440CLK;
        input MBISTC440RST;
        input MBISTC440STARTN;
        output C440MBISTDONE;
        output C440MBISTFAIL;

        output PPCDIAGPORTA[0:43];
        output PPCDIAGPORTB[0:135];
        output PPCDIAGPORTC[0:19];

        input TSTC440SCANENABLEN;
        input TSTC440TESTCNTLPOINTN;
        input TSTC440TESTMODEN;
        input TSTPPCSCANENABLEN;
        input TSTPPCSCANIN[0:15];
        output PPCTSTSCANOUT[0:15];

        attribute DCR_AUTOLOCK_ENABLE: bool;
        attribute PPCDM_ASYNCMODE: bool;
        attribute PPCDS_ASYNCMODE: bool;
        attribute PPCS0_WIDTH_128N64: bool;
        attribute PPCS1_WIDTH_128N64: bool;

        attribute APU_CONTROL: bitvec[17];
        attribute APU_UDI0: bitvec[24];
        attribute APU_UDI1: bitvec[24];
        attribute APU_UDI2: bitvec[24];
        attribute APU_UDI3: bitvec[24];
        attribute APU_UDI4: bitvec[24];
        attribute APU_UDI5: bitvec[24];
        attribute APU_UDI6: bitvec[24];
        attribute APU_UDI7: bitvec[24];
        attribute APU_UDI8: bitvec[24];
        attribute APU_UDI9: bitvec[24];
        attribute APU_UDI10: bitvec[24];
        attribute APU_UDI11: bitvec[24];
        attribute APU_UDI12: bitvec[24];
        attribute APU_UDI13: bitvec[24];
        attribute APU_UDI14: bitvec[24];
        attribute APU_UDI15: bitvec[24];
        attribute DMA0_CONTROL: bitvec[8];
        attribute DMA0_RXCHANNELCTRL: bitvec[32];
        attribute DMA0_TXCHANNELCTRL: bitvec[32];
        attribute DMA0_RXIRQTIMER: bitvec[10];
        attribute DMA0_TXIRQTIMER: bitvec[10];
        attribute DMA1_CONTROL: bitvec[8];
        attribute DMA1_RXCHANNELCTRL: bitvec[32];
        attribute DMA1_TXCHANNELCTRL: bitvec[32];
        attribute DMA1_RXIRQTIMER: bitvec[10];
        attribute DMA1_TXIRQTIMER: bitvec[10];
        attribute DMA2_CONTROL: bitvec[8];
        attribute DMA2_RXCHANNELCTRL: bitvec[32];
        attribute DMA2_TXCHANNELCTRL: bitvec[32];
        attribute DMA2_RXIRQTIMER: bitvec[10];
        attribute DMA2_TXIRQTIMER: bitvec[10];
        attribute DMA3_CONTROL: bitvec[8];
        attribute DMA3_RXCHANNELCTRL: bitvec[32];
        attribute DMA3_TXCHANNELCTRL: bitvec[32];
        attribute DMA3_RXIRQTIMER: bitvec[10];
        attribute DMA3_TXIRQTIMER: bitvec[10];
        attribute INTERCONNECT_IMASK: bitvec[32];
        attribute INTERCONNECT_TMPL_SEL: bitvec[32];
        attribute MI_ARBCONFIG: bitvec[32];
        attribute MI_BANKCONFLICT_MASK: bitvec[32];
        attribute MI_CONTROL: bitvec[32];
        attribute MI_ROWCONFLICT_MASK: bitvec[32];
        attribute PPCM_ARBCONFIG: bitvec[32];
        attribute PPCM_CONTROL: bitvec[32];
        attribute PPCM_COUNTER: bitvec[32];
        attribute PPCS0_CONTROL: bitvec[32];
        attribute PPCS1_CONTROL: bitvec[32];
        attribute PPCS0_ADDRMAP_TMPL0: bitvec[32];
        attribute PPCS1_ADDRMAP_TMPL0: bitvec[32];
        attribute XBAR_ADDRMAP_TMPL0: bitvec[32];
        attribute PPCS0_ADDRMAP_TMPL1: bitvec[32];
        attribute PPCS1_ADDRMAP_TMPL1: bitvec[32];
        attribute XBAR_ADDRMAP_TMPL1: bitvec[32];
        attribute PPCS0_ADDRMAP_TMPL2: bitvec[32];
        attribute PPCS1_ADDRMAP_TMPL2: bitvec[32];
        attribute XBAR_ADDRMAP_TMPL2: bitvec[32];
        attribute PPCS0_ADDRMAP_TMPL3: bitvec[32];
        attribute PPCS1_ADDRMAP_TMPL3: bitvec[32];
        attribute XBAR_ADDRMAP_TMPL3: bitvec[32];
        attribute APU_TEST: bitvec[3];
        attribute DCR_TEST: bitvec[3];
        attribute DMA_TEST: bitvec[3];
        attribute MIB_TEST: bitvec[3];
        attribute PLB_TEST: bitvec[4];

        attribute CLOCK_DELAY: bitvec[5];
    }
    device_data PPC440_CLOCK_DELAY: bitvec[5];

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

    bel_class EMAC_V6 {
        input RESET;

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
        input HOSTMIIMSEL;
        output HOSTMIIMRDY;
        input HOSTADDR[10];
        input HOSTWRDATA[32];
        output HOSTRDDATA[32];

        input CLIENTEMACDCMLOCKED;
        input CLIENTEMACPAUSEREQ;
        input CLIENTEMACPAUSEVAL[16];
        input CLIENTEMACRXCLIENTCLKIN;
        input CLIENTEMACTXCLIENTCLKIN;
        input CLIENTEMACTXD[16];
        input CLIENTEMACTXDVLD;
        input CLIENTEMACTXDVLDMSW;
        input CLIENTEMACTXFIRSTBYTE;
        input CLIENTEMACTXIFGDELAY[8];
        input CLIENTEMACTXUNDERRUN;

        output EMACCLIENTANINTERRUPT;
        output EMACCLIENTRXBADFRAME;
        output EMACCLIENTRXCLIENTCLKOUT;
        output EMACCLIENTRXD[16];
        output EMACCLIENTRXDVLD;
        output EMACCLIENTRXDVLDMSW;
        output EMACCLIENTRXDVREG6;
        output EMACCLIENTRXFRAMEDROP;
        output EMACCLIENTRXGOODFRAME;
        output EMACCLIENTRXSTATS[7];
        output EMACCLIENTRXSTATSBYTEVLD;
        output EMACCLIENTRXSTATSVLD;
        output EMACCLIENTTXACK;
        output EMACCLIENTTXCLIENTCLKOUT;
        output EMACCLIENTTXCOLLISION;
        output EMACCLIENTTXRETRANSMIT;
        output EMACCLIENTTXSTATS;
        output EMACCLIENTTXSTATSBYTEVLD;
        output EMACCLIENTTXSTATSVLD;

        input EMACTIBUS[5];
        output EMACTOBUS[5];

        input PHYEMACCOL;
        input PHYEMACCRS;
        input PHYEMACGTXCLK;
        input PHYEMACMCLKIN;
        input PHYEMACMDIN;
        input PHYEMACMIITXCLK;
        input PHYEMACPHYAD[5];
        // PHYEMACRXBUFERR gone
        input PHYEMACRXBUFSTATUS[2];
        input PHYEMACRXCHARISCOMMA;
        input PHYEMACRXCHARISK;
        // PHYEMACRXCHECKINGCRC gone
        input PHYEMACRXCLK;
        input PHYEMACRXCLKCORCNT[3];
        // PHYEMACRXCOMMADET gone
        input PHYEMACRXD[8];
        input PHYEMACRXDISPERR;
        input PHYEMACRXDV;
        input PHYEMACRXER;
        // PHYEMACRXLOSSOFSYNC gone
        input PHYEMACRXNOTINTABLE;
        input PHYEMACRXRUNDISP;
        input PHYEMACSIGNALDET;
        input PHYEMACTXBUFERR;
        input PHYEMACTXGMIIMIICLKIN;

        output EMACPHYENCOMMAALIGN;
        output EMACPHYLOOPBACKMSB;
        output EMACPHYMCLKOUT;
        output EMACPHYMDOUT;
        output EMACPHYMDTRI;
        output EMACPHYMGTRXRESET;
        output EMACPHYMGTTXRESET;
        output EMACPHYPOWERDOWN;
        output EMACPHYSYNCACQSTATUS;
        output EMACPHYTXCHARDISPMODE;
        output EMACPHYTXCHARDISPVAL;
        output EMACPHYTXCHARISK;
        output EMACPHYTXCLK;
        output EMACPHYTXD[8];
        output EMACPHYTXEN;
        output EMACPHYTXER;
        output EMACPHYTXGMIIMIICLKOUT;
        output EMACSPEEDIS10100;

        input TESTSELI;
        input TSTSEEMACI;
        input TSTSIEMACI[7];
        output TSTSOEMACO[7];

        attribute EMAC_1000BASEX_ENABLE: bool;
        attribute EMAC_ADDRFILTER_ENABLE: bool;
        attribute EMAC_BYTEPHY: bool;
        attribute EMAC_CONFIGVEC_79: bool;
        // new
        attribute EMAC_CTRLLENCHECK_DISABLE: bool;
        attribute EMAC_DCRBASEADDR: bitvec[8];
        attribute EMAC_FUNCTION: bitvec[3];
        attribute EMAC_GTLOOPBACK: bool;
        attribute EMAC_HOST_ENABLE: bool;
        attribute EMAC_LINKTIMERVAL: bitvec[9];
        attribute EMAC_LTCHECK_DISABLE: bool;
        attribute EMAC_MDIO_ENABLE: bool;
        attribute EMAC_PAUSEADDR: bitvec[48];
        attribute EMAC_PHYINITAUTONEG_ENABLE: bool;
        attribute EMAC_PHYISOLATE: bool;
        attribute EMAC_PHYLOOPBACKMSB: bool;
        attribute EMAC_PHYPOWERDOWN: bool;
        attribute EMAC_PHYRESET: bool;
        attribute EMAC_RGMII_ENABLE: bool;
        attribute EMAC_RX16BITCLIENT_ENABLE: bool;
        attribute EMAC_RXFLOWCTRL_ENABLE: bool;
        attribute EMAC_RXHALFDUPLEX: bool;
        attribute EMAC_RXINBANDFCS_ENABLE: bool;
        attribute EMAC_RXJUMBOFRAME_ENABLE: bool;
        attribute EMAC_RXRESET: bool;
        attribute EMAC_RXVLAN_ENABLE: bool;
        attribute EMAC_RX_ENABLE: bool;
        attribute EMAC_SGMII_ENABLE: bool;
        attribute EMAC_SPEED_LSB: bool;
        attribute EMAC_SPEED_MSB: bool;
        attribute EMAC_TX16BITCLIENT_ENABLE: bool;
        attribute EMAC_TXFLOWCTRL_ENABLE: bool;
        attribute EMAC_TXHALFDUPLEX: bool;
        attribute EMAC_TXIFGADJUST_ENABLE: bool;
        attribute EMAC_TXINBANDFCS_ENABLE: bool;
        attribute EMAC_TXJUMBOFRAME_ENABLE: bool;
        attribute EMAC_TXRESET: bool;
        attribute EMAC_TXVLAN_ENABLE: bool;
        attribute EMAC_TX_ENABLE: bool;
        attribute EMAC_UNICASTADDR: bitvec[48];
        attribute EMAC_UNIDIRECTION_ENABLE: bool;
        attribute EMAC_USECLKEN: bool;
    }

    bel_class PCIE_V5 {
        input CRMCORECLK;
        input CRMCORECLKDLO;
        input CRMCORECLKRXO;
        input CRMCORECLKTXO;
        input CRMUSERCLK;
        input CRMUSERCLKRXO;
        input CRMUSERCLKTXO;
        input CRMURSTN;
        input CRMNVRSTN;
        input CRMMGMTRSTN;
        input CRMUSERCFGRSTN;
        input CRMMACRSTN;
        input CRMLINKRSTN;
        input CRMCFGBRIDGEHOTRESET;
        output CRMDOHOTRESETN;
        output CRMPWRSOFTRESETN;
        input CRMTXHOTRESETN;
        output CRMRXHOTRESETN;

        output LLKTCSTATUS[8];
        input LLKTXDATA[64];
        input LLKTXSRCRDYN;
        output LLKTXDSTRDYN;
        input LLKTXSRCDSCN;
        output LLKTXCHANSPACE[10];
        input LLKTXSOFN;
        input LLKTXEOFN;
        input LLKTXSOPN;
        input LLKTXEOPN;
        input LLKTXENABLEN[2];
        input LLKTXCHTC[3];
        input LLKTXCHFIFO[2];
        output LLKTXCHPOSTEDREADYN[8];
        output LLKTXCHNONPOSTEDREADYN[8];
        output LLKTXCHCOMPLETIONREADYN[8];
        output LLKTXCONFIGREADYN;
        input LLKTX4DWHEADERN;
        input LLKTXCOMPLETEN;
        input LLKTXCREATEECRCN;
        output LLKRXDATA[64];
        output LLKRXSRCRDYN;
        input LLKRXDSTREQN;
        output LLKRXSRCLASTREQN;
        output LLKRXSRCDSCN;
        input LLKRXDSTCONTREQN;
        output LLKRXSOFN;
        output LLKRXEOFN;
        output LLKRXSOPN;
        output LLKRXEOPN;
        output LLKRXVALIDN[2];
        input LLKRXCHTC[3];
        input LLKRXCHFIFO[2];
        output LLKRXCHPOSTEDAVAILABLEN[8];
        output LLKRXCHPOSTEDPARTIALN[8];
        output LLKRXCHNONPOSTEDAVAILABLEN[8];
        output LLKRXCHNONPOSTEDPARTIALN[8];
        output LLKRXCHCOMPLETIONAVAILABLEN[8];
        output LLKRXCHCOMPLETIONPARTIALN[8];
        output LLKRXCHCONFIGAVAILABLEN;
        output LLKRXCHCONFIGPARTIALN;
        output LLKRXPREFERREDTYPE[16];
        output LLKRX4DWHEADERN;
        output LLKRXECRCBADN;

        output MGMTRDATA[32];
        input MGMTWDATA[32];
        input MGMTBWREN[4];
        input MGMTRDEN;
        input MGMTWREN;
        input MGMTADDR[11];
        output MGMTSTATSCREDIT[12];
        input MGMTSTATSCREDITSEL[7];
        output MGMTPSO[17];

        output MIMTXBWDATA[64];
        output MIMTXBWADD[13];
        output MIMTXBRADD[13];
        output MIMTXBWEN;
        input MIMTXBRDATA[64];
        output MIMTXBREN;

        output MIMRXBWDATA[64];
        output MIMRXBWADD[13];
        output MIMRXBRADD[13];
        output MIMRXBWEN;
        input MIMRXBRDATA[64];
        output MIMRXBREN;

        output MIMDLLBWDATA[64];
        output MIMDLLBWADD[12];
        output MIMDLLBRADD[12];
        output MIMDLLBWEN;
        input MIMDLLBRDATA[64];
        output MIMDLLBREN;

        for i in 0..8 {
            input "PIPERXELECIDLEL{i}";
            input "PIPERXSTATUSL{i}"[3];
            input "PIPERXDATAL{i}"[8];
            input "PIPERXDATAKL{i}";
            input "PIPEPHYSTATUSL{i}";
            input "PIPERXVALIDL{i}";
            input "PIPERXCHANISALIGNEDL{i}";
            output "PIPERXPOLARITYL{i}";
            output "PIPETXDATAL{i}"[8];
            output "PIPETXDATAKL{i}";
            output "PIPETXELECIDLEL{i}";
            output "PIPETXDETECTRXLOOPBACKL{i}";
            output "PIPETXCOMPLIANCEL{i}";
            output "PIPEPOWERDOWNL{i}"[2];
            output "PIPEDESKEWLANESL{i}";
            output "PIPERESETL{i}";
        }

        input L0ACKNAKTIMERADJUSTMENT[12];
        input L0ALLDOWNPORTSINL1;
        input L0ALLDOWNRXPORTSINL0S;
        output L0ASAUTONOMOUSINITCOMPLETED;
        input L0ASE;
        input L0ASPORTCOUNT[8];
        input L0ASTURNPOOLBITSCONSUMED[3];
        input L0ATTENTIONBUTTONPRESSED;
        output L0ATTENTIONINDICATORCONTROL[2];
        input L0CFGASSPANTREEOWNEDSTATE;
        input L0CFGASSTATECHANGECMD[4];
        input L0CFGDISABLESCRAMBLE;
        input L0CFGEXTENDEDSYNC;
        input L0CFGL0SENTRYENABLE;
        input L0CFGL0SENTRYSUP;
        input L0CFGL0SEXITLAT[3];
        input L0CFGLINKDISABLE;
        output L0CFGLOOPBACKACK;
        input L0CFGLOOPBACKMASTER;
        input L0CFGNEGOTIATEDMAXP[3];
        input L0CFGVCENABLE[8];
        input L0CFGVCID[24];
        output L0COMPLETERID[13];
        output L0CORRERRMSGRCVD;
        output L0DLLASRXSTATE0;
        output L0DLLASRXSTATE1;
        output L0DLLASTXSTATE;
        output L0DLLERRORVECTOR[7];
        input L0DLLHOLDLINKUP;
        output L0DLLRXACKOUTSTANDING;
        output L0DLLTXNONFCOUTSTANDING;
        output L0DLLTXOUTSTANDING;
        output L0DLLVCSTATUS[8];
        output L0DLUPDOWN[8];
        input L0ELECTROMECHANICALINTERLOCKENGAGED;
        output L0ERRMSGREQID[16];
        output L0FATALERRMSGRCVD;
        output L0FIRSTCFGWRITEOCCURRED;
        input L0FWDASSERTINTALEGACYINT;
        input L0FWDASSERTINTBLEGACYINT;
        input L0FWDASSERTINTCLEGACYINT;
        input L0FWDASSERTINTDLEGACYINT;
        input L0FWDCORRERRIN;
        output L0FWDCORRERROUT;
        input L0FWDDEASSERTINTALEGACYINT;
        input L0FWDDEASSERTINTBLEGACYINT;
        input L0FWDDEASSERTINTCLEGACYINT;
        input L0FWDDEASSERTINTDLEGACYINT;
        input L0FWDFATALERRIN;
        output L0FWDFATALERROUT;
        input L0FWDNONFATALERRIN;
        output L0FWDNONFATALERROUT;
        input L0LEGACYINTFUNCT0;
        output L0LTSSMSTATE[4];
        output L0MACENTEREDL0;
        output L0MACLINKTRAINING;
        output L0MACLINKUP;
        output L0MACNEGOTIATEDLINKWIDTH[4];
        output L0MACNEWSTATEACK;
        output L0MACRXL0SSTATE;
        output L0MACUPSTREAMDOWNSTREAM;
        output L0MCFOUND[3];
        input L0MRLSENSORCLOSEDN;
        output L0MSIENABLE0;
        input L0MSIREQUEST0[4];
        output L0MULTIMSGEN0[3];
        output L0NONFATALERRMSGRCVD;
        input L0PACKETHEADERFROMUSER[128];
        output L0PMEACK;
        output L0PMEEN;
        input L0PMEREQIN;
        output L0PMEREQOUT;
        input L0PORTNUMBER[8];
        output L0POWERCONTROLLERCONTROL;
        input L0POWERFAULTDETECTED;
        output L0POWERINDICATORCONTROL[2];
        input L0PRESENCEDETECTSLOTEMPTYN;
        output L0PWRSTATE0[2];
        output L0PWRL1STATE;
        output L0PWRL23READYDEVICE;
        output L0PWRL23READYSTATE;
        output L0PWRTXL0SSTATE;
        output L0PWRTURNOFFREQ;
        output L0PWRINHIBITTRANSFERS;
        input L0PWRNEWSTATEREQ;
        input L0PWRNEXTLINKSTATE[2];
        output L0RECEIVEDASSERTINTALEGACYINT;
        output L0RECEIVEDASSERTINTBLEGACYINT;
        output L0RECEIVEDASSERTINTCLEGACYINT;
        output L0RECEIVEDASSERTINTDLEGACYINT;
        output L0RECEIVEDDEASSERTINTALEGACYINT;
        output L0RECEIVEDDEASSERTINTBLEGACYINT;
        output L0RECEIVEDDEASSERTINTCLEGACYINT;
        output L0RECEIVEDDEASSERTINTDLEGACYINT;
        input L0REPLAYTIMERADJUSTMENT[12];
        input L0ROOTTURNOFFREQ;
        output L0RXBEACON;
        output L0RXDLLFCCMPLMCCRED[24];
        output L0RXDLLFCCMPLMCUPDATE[8];
        output L0RXDLLFCNPOSTBYPCRED[20];
        output L0RXDLLFCNPOSTBYPUPDATE[8];
        output L0RXDLLFCPOSTORDCRED[24];
        output L0RXDLLFCPOSTORDUPDATE[8];
        output L0RXDLLPM;
        output L0RXDLLPMTYPE[3];
        output L0RXDLLSBFCDATA[19];
        output L0RXDLLSBFCUPDATE;
        output L0RXDLLTLPECRCOK;
        output L0RXDLLTLPEND[2];
        output L0RXMACLINKERROR0;
        output L0RXMACLINKERROR1;
        input L0RXTLTLPNONINITIALIZEDVC[8];
        input L0SENDUNLOCKMESSAGE;
        input L0SETCOMPLETERABORTERROR;
        input L0SETCOMPLETIONTIMEOUTCORRERROR;
        input L0SETCOMPLETIONTIMEOUTUNCORRERROR;
        input L0SETDETECTEDCORRERROR;
        input L0SETDETECTEDFATALERROR;
        input L0SETDETECTEDNONFATALERROR;
        input L0SETLINKDETECTEDPARITYERROR;
        input L0SETLINKMASTERDATAPARITY;
        input L0SETLINKRECEIVEDMASTERABORT;
        input L0SETLINKRECEIVEDTARGETABORT;
        input L0SETLINKSIGNALLEDTARGETABORT;
        input L0SETLINKSYSTEMERROR;
        input L0SETUNEXPECTEDCOMPLETIONCORRERROR;
        input L0SETUNEXPECTEDCOMPLETIONUNCORRERROR;
        input L0SETUNSUPPORTEDREQUESTNONPOSTEDERROR;
        input L0SETUNSUPPORTEDREQUESTOTHERERROR;
        input L0SETUSERDETECTEDPARITYERROR;
        input L0SETUSERMASTERDATAPARITY;
        input L0SETUSERRECEIVEDMASTERABORT;
        input L0SETUSERRECEIVEDTARGETABORT;
        input L0SETUSERSIGNALLEDTARGETABORT;
        input L0SETUSERSYSTEMERROR;
        output L0STATSCFGOTHERRECEIVED;
        output L0STATSCFGOTHERTRANSMITTED;
        output L0STATSCFGRECEIVED;
        output L0STATSCFGTRANSMITTED;
        output L0STATSDLLPRECEIVED;
        output L0STATSDLLPTRANSMITTED;
        output L0STATSOSRECEIVED;
        output L0STATSOSTRANSMITTED;
        output L0STATSTLPRECEIVED;
        output L0STATSTLPTRANSMITTED;
        input L0TLASFCCREDSTARVATION;
        input L0TLLINKRETRAIN;
        output L0TOGGLEELECTROMECHANICALINTERLOCK;
        input L0TRANSACTIONSPENDING;
        output L0TRANSFORMEDVC[3];
        input L0TXBEACON;
        input L0TXCFGPM;
        input L0TXCFGPMTYPE[3];
        output L0TXDLLFCCMPLMCUPDATED[8];
        output L0TXDLLFCNPOSTBYPUPDATED[8];
        output L0TXDLLFCPOSTORDUPDATED[8];
        output L0TXDLLPMUPDATED;
        output L0TXDLLSBFCUPDATED;
        input L0TXTLFCCMPLMCCRED[160];
        input L0TXTLFCCMPLMCUPDATE[16];
        input L0TXTLFCNPOSTBYPCRED[192];
        input L0TXTLFCNPOSTBYPUPDATE[16];
        input L0TXTLFCPOSTORDCRED[160];
        input L0TXTLFCPOSTORDUPDATE[16];
        input L0TXTLSBFCDATA[19];
        input L0TXTLSBFCUPDATE;
        input L0TXTLTLPDATA[64];
        input L0TXTLTLPEDB;
        input L0TXTLTLPENABLE[2];
        input L0TXTLTLPEND0;
        input L0TXTLTLPEND1;
        input L0TXTLTLPLATENCY[4];
        input L0TXTLTLPREQ;
        input L0TXTLTLPREQEND;
        input L0TXTLTLPWIDTH;
        output L0UCBYPFOUND[4];
        output L0UCORDFOUND[4];
        output L0UNLOCKRECEIVED;
        input L0UPSTREAMRXPORTINL0S;
        input L0VC0PREVIEWEXPAND;
        input L0WAKEN;

        input COMPLIANCEAVOID;
        output IOSPACEENABLE;
        output MEMSPACEENABLE;
        output MAXPAYLOADSIZE[3];
        output MAXREADREQUESTSIZE[3];
        output BUSMASTERENABLE;
        output PARITYERRORRESPONSE;
        output SERRENABLE;
        output INTERRUPTDISABLE;
        output URREPORTINGENABLE;
        input AUXPOWER;
        output DLLTXPMDLLPOUTSTANDING;
        input CFGNEGOTIATEDLINKWIDTH[6];
        input CROSSLINKSEED;
        input MAINPOWER;

        input SCANENABLEN;
        input SCANIN[8];
        input SCANMODEN;

        for i in 0..2 {
            attribute "VC{i}TXFIFOBASEP": bitvec[13];
            attribute "VC{i}TXFIFOBASENP": bitvec[13];
            attribute "VC{i}TXFIFOBASEC": bitvec[13];
            attribute "VC{i}TXFIFOLIMITP": bitvec[13];
            attribute "VC{i}TXFIFOLIMITNP": bitvec[13];
            attribute "VC{i}TXFIFOLIMITC": bitvec[13];
            attribute "VC{i}TOTALCREDITSPH": bitvec[7];
            attribute "VC{i}TOTALCREDITSNPH": bitvec[7];
            attribute "VC{i}TOTALCREDITSCH": bitvec[7];
            attribute "VC{i}TOTALCREDITSPD": bitvec[11];
            attribute "VC{i}TOTALCREDITSCD": bitvec[11];
            attribute "VC{i}RXFIFOBASEP": bitvec[13];
            attribute "VC{i}RXFIFOBASENP": bitvec[13];
            attribute "VC{i}RXFIFOBASEC": bitvec[13];
            attribute "VC{i}RXFIFOLIMITP": bitvec[13];
            attribute "VC{i}RXFIFOLIMITNP": bitvec[13];
            attribute "VC{i}RXFIFOLIMITC": bitvec[13];
        }
        attribute ACTIVELANESIN: bitvec[8];
        attribute TXTSNFTS: bitvec[8];
        attribute TXTSNFTSCOMCLK: bitvec[8];
        attribute RETRYRAMREADLATENCY: bitvec[3];
        attribute RETRYRAMWRITELATENCY: bitvec[3];
        attribute RETRYRAMWIDTH: bitvec[1];
        attribute RETRYRAMSIZE: bitvec[12];
        attribute RETRYWRITEPIPE: bool;
        attribute RETRYREADADDRPIPE: bool;
        attribute RETRYREADDATAPIPE: bool;
        attribute XLINKSUPPORTED: bool;
        attribute INFINITECOMPLETIONS: bool;
        attribute TLRAMREADLATENCY: bitvec[3];
        attribute TLRAMWRITELATENCY: bitvec[3];
        attribute TLRAMWIDTH: bitvec[1];
        attribute RAMSHARETXRX: bool;
        attribute L0SEXITLATENCY: bitvec[3];
        attribute L0SEXITLATENCYCOMCLK: bitvec[3];
        attribute L1EXITLATENCY: bitvec[3];
        attribute L1EXITLATENCYCOMCLK: bitvec[3];
        attribute DUALCORESLAVE: bool;
        attribute DUALCOREENABLE: bool;
        attribute DUALROLECFGCNTRLROOTEPN: bitvec[1];
        attribute RXREADADDRPIPE: bool;
        attribute RXREADDATAPIPE: bool;
        attribute TXWRITEPIPE: bool;
        attribute TXREADADDRPIPE: bool;
        attribute TXREADDATAPIPE: bool;
        attribute RXWRITEPIPE: bool;
        attribute LLKBYPASS: bool;
        attribute PCIEREVISION: bitvec[1];
        attribute SELECTDLLIF: bool;
        attribute SELECTASMODE: bool;
        attribute ISSWITCH: bool;
        attribute UPSTREAMFACING: bool;
        attribute SLOTIMPLEMENTED: bool;
        attribute EXTCFGCAPPTR: bitvec[8];
        attribute EXTCFGXPCAPPTR: bitvec[12];
        attribute BAR0EXIST: bool;
        attribute BAR1EXIST: bool;
        attribute BAR2EXIST: bool;
        attribute BAR3EXIST: bool;
        attribute BAR4EXIST: bool;
        attribute BAR5EXIST: bool;
        attribute BAR0ADDRWIDTH: bitvec[1];
        attribute BAR1ADDRWIDTH: bitvec[1];
        attribute BAR2ADDRWIDTH: bitvec[1];
        attribute BAR3ADDRWIDTH: bitvec[1];
        attribute BAR4ADDRWIDTH: bitvec[1];
        attribute BAR5ADDRWIDTH: bitvec[1];
        attribute BAR0PREFETCHABLE: bool;
        attribute BAR1PREFETCHABLE: bool;
        attribute BAR2PREFETCHABLE: bool;
        attribute BAR3PREFETCHABLE: bool;
        attribute BAR4PREFETCHABLE: bool;
        attribute BAR5PREFETCHABLE: bool;
        attribute BAR0IOMEMN: bitvec[1];
        attribute BAR1IOMEMN: bitvec[1];
        attribute BAR2IOMEMN: bitvec[1];
        attribute BAR3IOMEMN: bitvec[1];
        attribute BAR4IOMEMN: bitvec[1];
        attribute BAR5IOMEMN: bitvec[1];
        attribute BAR0MASKWIDTH: bitvec[6];
        attribute BAR1MASKWIDTH: bitvec[6];
        attribute BAR2MASKWIDTH: bitvec[6];
        attribute BAR3MASKWIDTH: bitvec[6];
        attribute BAR4MASKWIDTH: bitvec[6];
        attribute BAR5MASKWIDTH: bitvec[6];
        attribute CONFIGROUTING: bitvec[3];
        attribute XPDEVICEPORTTYPE: bitvec[4];
        attribute HEADERTYPE: bitvec[8];
        attribute XPMAXPAYLOAD: bitvec[3];
        attribute XPRCBCONTROL: bitvec[1];
        attribute LOWPRIORITYVCCOUNT: bitvec[3];
        attribute VENDORID: bitvec[16];
        attribute DEVICEID: bitvec[16];
        attribute REVISIONID: bitvec[8];
        attribute CLASSCODE: bitvec[24];
        attribute CARDBUSCISPOINTER: bitvec[32];
        attribute SUBSYSTEMVENDORID: bitvec[16];
        attribute SUBSYSTEMID: bitvec[16];
        attribute CAPABILITIESPOINTER: bitvec[8];
        attribute INTERRUPTPIN: bitvec[8];
        attribute PMCAPABILITYNEXTPTR: bitvec[8];
        attribute PMCAPABILITYDSI: bool;
        attribute PMCAPABILITYAUXCURRENT: bitvec[3];
        attribute PMCAPABILITYD1SUPPORT: bool;
        attribute PMCAPABILITYD2SUPPORT: bool;
        attribute PMCAPABILITYPMESUPPORT: bitvec[5];
        attribute PMSTATUSCONTROLDATASCALE: bitvec[2];
        attribute PMDATA0: bitvec[8];
        attribute PMDATA1: bitvec[8];
        attribute PMDATA2: bitvec[8];
        attribute PMDATA3: bitvec[8];
        attribute PMDATA4: bitvec[8];
        attribute PMDATA5: bitvec[8];
        attribute PMDATA6: bitvec[8];
        attribute PMDATA7: bitvec[8];
        attribute PMDATA8: bitvec[8];
        attribute PMDATASCALE0: bitvec[2];
        attribute PMDATASCALE1: bitvec[2];
        attribute PMDATASCALE2: bitvec[2];
        attribute PMDATASCALE3: bitvec[2];
        attribute PMDATASCALE4: bitvec[2];
        attribute PMDATASCALE5: bitvec[2];
        attribute PMDATASCALE6: bitvec[2];
        attribute PMDATASCALE7: bitvec[2];
        attribute PMDATASCALE8: bitvec[2];
        attribute MSICAPABILITYNEXTPTR: bitvec[8];
        attribute MSICAPABILITYMULTIMSGCAP: bitvec[3];
        attribute PCIECAPABILITYNEXTPTR: bitvec[8];
        attribute PCIECAPABILITYSLOTIMPL: bool;
        attribute PCIECAPABILITYINTMSGNUM: bitvec[5];
        attribute DEVICECAPABILITYENDPOINTL0SLATENCY: bitvec[3];
        attribute DEVICECAPABILITYENDPOINTL1LATENCY: bitvec[3];
        attribute LINKCAPABILITYMAXLINKWIDTH: bitvec[6];
        attribute LINKCAPABILITYASPMSUPPORT: bitvec[2];
        attribute LINKSTATUSSLOTCLOCKCONFIG: bool;
        attribute SLOTCAPABILITYATTBUTTONPRESENT: bool;
        attribute SLOTCAPABILITYPOWERCONTROLLERPRESENT: bool;
        attribute SLOTCAPABILITYMSLSENSORPRESENT: bool;
        attribute SLOTCAPABILITYATTINDICATORPRESENT: bool;
        attribute SLOTCAPABILITYPOWERINDICATORPRESENT: bool;
        attribute SLOTCAPABILITYHOTPLUGSURPRISE: bool;
        attribute SLOTCAPABILITYHOTPLUGCAPABLE: bool;
        attribute SLOTCAPABILITYSLOTPOWERLIMITVALUE: bitvec[8];
        attribute SLOTCAPABILITYSLOTPOWERLIMITSCALE: bitvec[2];
        attribute SLOTCAPABILITYPHYSICALSLOTNUM: bitvec[13];
        attribute AERCAPABILITYNEXTPTR: bitvec[12];
        attribute AERCAPABILITYECRCGENCAPABLE: bool;
        attribute AERCAPABILITYECRCCHECKCAPABLE: bool;
        attribute VCCAPABILITYNEXTPTR: bitvec[12];
        attribute PORTVCCAPABILITYEXTENDEDVCCOUNT: bitvec[3];
        attribute PORTVCCAPABILITYVCARBCAP: bitvec[8];
        attribute PORTVCCAPABILITYVCARBTABLEOFFSET: bitvec[8];
        attribute DSNCAPABILITYNEXTPTR: bitvec[12];
        attribute DEVICESERIALNUMBER: bitvec[64];
        attribute PBCAPABILITYNEXTPTR: bitvec[12];
        for i in 0..4 {
            attribute "PBCAPABILITYDW{i}BASEPOWER": bitvec[8];
            attribute "PBCAPABILITYDW{i}DATASCALE": bitvec[2];
            attribute "PBCAPABILITYDW{i}PMSUBSTATE": bitvec[3];
            attribute "PBCAPABILITYDW{i}PMSTATE": bitvec[2];
            attribute "PBCAPABILITYDW{i}TYPE": bitvec[3];
            attribute "PBCAPABILITYDW{i}POWERRAIL": bitvec[3];
        }
        attribute PBCAPABILITYSYSTEMALLOCATED: bool;
        attribute RESETMODE: bool;
        attribute AERBASEPTR: bitvec[12];
        attribute DSNBASEPTR: bitvec[12];
        attribute MSIBASEPTR: bitvec[12];
        attribute PBBASEPTR: bitvec[12];
        attribute PMBASEPTR: bitvec[12];
        attribute VCBASEPTR: bitvec[12];
        attribute XPBASEPTR: bitvec[8];
    }

    // Xilinx transceiver quick guide:
    //
    // - virtex2:
    //   - virtex2: no transceivers
    //   - virtex2p: `GT` aka RocketIO
    //     - sprinkled along the south and north IO banks
    //     - one channel per tile
    //     - replaces (occupies the space of) one BRAM and one DCM
    //     - no dedicated clock inputs; uses two shared IOB pairs per bank with a dedicated fast path (`BREFCLK`)
    //   - virtex2px: `GT10` aka RocketIO X (wider, bigger, faster)
    //     - sprinkled along the south and north IO banks
    //     - one channel per tile
    //     - replaces (occupies the space of) two BRAMs and one DCM
    //     - one dedicated differential clock input per bank (confusingly also called `BREFCLK`)
    //       - the differential clock input replaces two IOBs; unclear exactly how much of the IOBs is really dumied out
    // - spartan6:
    //   - 6slx9, 6slx16: no transceivers (the larger lx die are just lxt with transceivers disabled)
    //   - 6slx*t: `GTPA1_DUAL`
    //     - sprinkled along the south and north IO banks (up to two per bank)
    //     - two channels and two dedicated clock inputs per tile
    //     - the entire complex is represented by a single `GTPA1_DUAL` bel
    //     - there's one `PCIE` bel on the entire device, associated with the NW GTP tile
    // - virtex4:
    //   - lx, sx: no transceivers
    //   - fx: `GT11` aka RocketIO
    //     - occupies dedicated columns at the west and east edges of the device
    //     - one tile spans two clock regions (same as one IO bank)
    //     - two channels and one differential clock input per tile
    //     - each channel is represented by a `GT11` bel; the clock input is represented as a `GT11CLK` bel
    //     - the two `GT11`s have a lot of circuitry in common; we put the shared attributes on the `GT11CLK` bel
    // - virtex5:
    //   - lx: no transceivers (quite possibly is just lxt with the right column cut off)
    //   - lxt, sxt: `GTP_DUAL` aka RocketIO GTP
    //     - occupies dedicated column at the east edge of the device
    //     - has a column with `PCIE` and `EMAC` blocks next to it
    //     - one tile spans a single clock region (same as one IO bank)
    //     - two channels and one differential clock input per tile
    //     - the entire complex is represented by a `GTP_DUAL` bel
    //     - bundled with 4× `CRC32` bels per tile (can be combined into 2× CRC64)
    //   - fxt, txt: `GTX_DUAL` aka RocketIO GTX
    //     - seems to be a minor variant of GTP with double data path width and fancier DFE; even the pin and bit locations match
    //     - all of above applies unchanged, except the bel is `GTX_DUAL`
    //     - txt has a second column at the west edge of the device
    // - virtex6:
    //   - lx: no transceivers
    //   - lxt, sxt, hxt, cxt: `GTX`
    //     - occupies dedicated column at the east edge of the device
    //       - on hxt, has another column at the west edge
    //     - has a column with `PCIE` and `EMAC` blocks next to it
    //     - one tile spans a single clock region (same as one IO bank)
    //     - four channels and two differential clock inputs per tile
    //     - each channel is represented by a `GTX` bel
    //     - the clock routing (including the clock inputs) of the tile is represented by a `HCLK_GTX` bel
    //   - hxt (some devices): `GTH_QUAD` (in addition to also having `GTX` on the same device)
    //     - shares the west and east columns with `GTX` (ie. some clock regions just have `GTH` instead of `GTX`)
    //     - four channels and one differential clock input per tile
    //     - the entire complex is represented by a single `GTH_QUAD` bel
    //  - virtex7:
    //    - 7a*, 7z015: GTP
    //      - one "quad" occupies one clock region vertically, and punches a hole of 10-18 or so columns
    //      - on most devices, the quads are located in SE and/or NE corners (up to two quads per device)
    //      - 7a200t has four quads, located in the middle of S and N edges
    //      - a quad has four channels (represented by `GTP_CHANNEL` tile and bel) and two differential clock inputs
    //      - the two clock inputs together with a bunch of common circuitry are represented by a `GTP_COMMON` tile and bel
    //      - has one accompanying `PCIE` tile per device (does PCIE 2.0)
    //    - 7k*, most 7z*, 7v*: GTX
    //      - a GTX quad has the same dimensions as a GTP quad; likewise, four channels and two clock inputs
    //      - GTX transceivers are organized in a column at the east edge of the device
    //        - on 7z* and some 7k*, the column doesn't span the whole height of the device; the rest is occupied by normal IO
    //      - on some 7v*, there's additionally another column at the west edge of the device
    //      - the bels are `GTX_CHANNEL` and `GTX_COMMON`, with the same organization as GTP
    //      - likewise associated with `PCIE` bels; 7v* have several `PCIE` per device
    //    - some 7v*: GTH
    //      - as opposed to virtex6, does *not* appear together with GTX on the same device
    //      - essentially a minor upgrade of GTX; the bels are `GTH_CHANNEL` and `GTH_COMMON`
    //      - associated with `PCIE3` bels instead of `PCIE`
    //    - 7vh*: GTZ
    //      - is located *on a separate die*, communicates with the FPGA via the cross-SLR channels
    //      - is provided *in addition* to the GTH quads present on the main FGPA die
    //      - there is one or two GTZ git per device (located at the north and maybe south edges of the SLR stackup)
    //      - eight channels and two differential clock inputs per die
    //      - configured via a comically complex process (needs its configuration uploaded through the cross-SLR links instead of using the normal configuration write mechanism)
    //      - heavily NDA'd
    // - ultrascale:
    //   - GTH: same general structure as virtex7; four channels, two clock inputs, `GTH_COMMON` and `GTH_CHANNEL` bels
    //   - GTY: likewise; `GTY_COMMON` and `GTY_CHANNEL`
    // - ultrascaleplus:
    //   - GTH: similar to ultrascale
    //   - GTY: similar to ultrascale
    //   - GTF: same general structure as GTH/GTY; a secret, third transceiver type used on xcvu2p
    //   - GTM:
    //     - organized in duals, as opposed to quads; a GTM dual takes up the same space as a GTH quad
    //     - generally coexists with GTY on the same device
    //     - a dual has two channels and one differential clock input
    //     - the entire complex is represented by a single `GTM_DUAL` bel
    //   - GTR: part of the PS complex and dedicated for its purposes; not represented in the database

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

    bel_class CRC32 {
        input CRCCLK, CRCRESET;
        input CRCDATAVALID;
        input CRCDATAWIDTH[3];
        input CRCIN[32];
        output CRCOUT[32];

        attribute CRC_INIT: bitvec[32];
        // only present on [0] and [2]; when set, this bel becomes a CRC64, its own CRCIN becomes
        // the high bits of the wide CRCIN input, and the companion's CRCIN becomes the low bits.
        // all other inputs and outputs remain unchanged.
        attribute ENABLE64: bool;
    }

    enum GTP_CLK25_DIVIDER { _1, _2, _3, _4, _5, _6, _10, _12 }
    enum GTP_OOB_CLK_DIVIDER { _1, _2, _4, _6, _8, _10, _12, _14 }
    enum GTP_PLL_DIVSEL_FB { _1, _2, _3, _4, _5, _8, _10 }
    enum GTP_PLL_DIVSEL_REF { _1, _2, _3, _4, _5, _6, _8, _10, _12, _16, _20 }
    enum GTP_PLL_DIVSEL_OUT { _1, _2, _4 }
    enum GTP_ALIGN_COMMA_WORD { _1, _2 }
    enum GTP_CHAN_BOND_MODE { NONE, MASTER, SLAVE }
    enum GTP_SEQ_LEN { _1, _2, _3, _4 }
    enum GTP_RX_LOS_INVALID_INCR { _1, _2, _4, _8, _16, _32, _64, _128 }
    enum GTP_RX_LOS_THRESHOLD { _4, _8, _16, _32, _64, _128, _256, _512 }
    enum GTP_RX_SLIDE_MODE { PCS, PMA }
    enum GTP_RX_STATUS_FMT { PCIE, SATA }
    enum GTP_RX_XCLK_SEL { RXUSR, RXREC }
    enum GTP_TX_XCLK_SEL { TXUSR, TXOUT }
    enum GTP_TERMINATION_IMP { _50, _75 }
    enum GTP_MUX_CLKIN { CLKPN, GREFCLK, CLKOUT_NORTH_S, CLKOUT_SOUTH_N }
    enum GTP_MUX_CLKOUT_NORTH { CLKPN, CLKOUT_NORTH_S }
    enum GTP_MUX_CLKOUT_SOUTH { CLKPN, CLKOUT_SOUTH_N }
    bel_class GTP_DUAL {
        input DCLK;
        input DEN;
        input DWE;
        input DADDR[7];
        input DI[16];
        output DRDY;
        output DO[16];

        input GREFCLK;
        input GTPRESET;
        input GTPTEST[4];
        input INTDATAWIDTH;
        output PLLLKDET;
        input PLLLKDETEN;
        input PLLPOWERDOWN;
        output REFCLKOUT;
        input REFCLKPWRDNB;

        input PMAAMUX[3];
        output PMATSTCLK;
        input PMATSTCLKSEL[3];
        input RXENELECIDLERESETB;
        input TXENPMAPHASEALIGN;
        input TXPMASETPHASE;

        for ch in 0..2 {
            input "LOOPBACK{ch}"[3];
            output "PHYSTATUS{ch}";
            input "PRBSCNTRESET{ch}";
            output "RESETDONE{ch}";

            input "RXBUFRESET{ch}";
            output "RXBUFSTATUS{ch}"[3];
            output "RXBYTEISALIGNED{ch}";
            output "RXBYTEREALIGN{ch}";
            input "RXCDRRESET{ch}";
            output "RXCHANBONDSEQ{ch}";
            output "RXCHANISALIGNED{ch}";
            output "RXCHANREALIGN{ch}";
            output "RXCHARISCOMMA{ch}"[2];
            output "RXCHARISK{ch}"[2];
            input "RXCHBONDI{ch}"[3];
            output "RXCHBONDO{ch}"[3];
            output "RXCLKCORCNT{ch}"[3];
            output "RXCOMMADET{ch}";
            input "RXCOMMADETUSE{ch}";
            output "RXDATA{ch}"[16];
            input "RXDATAWIDTH{ch}";
            input "RXDEC8B10BUSE{ch}";
            output "RXDISPERR{ch}"[2];
            output "RXELECIDLE{ch}";
            input "RXELECIDLERESET{ch}";
            input "RXENCHANSYNC{ch}";
            input "RXENEQB{ch}";
            input "RXENMCOMMAALIGN{ch}";
            input "RXENPCOMMAALIGN{ch}";
            input "RXENPRBSTST{ch}"[2];
            input "RXENSAMPLEALIGN{ch}";
            input "RXEQMIX{ch}"[2];
            input "RXEQPOLE{ch}"[4];
            output "RXLOSSOFSYNC{ch}"[2];
            output "RXNOTINTABLE{ch}"[2];
            output "RXOVERSAMPLEERR{ch}";
            input "RXPMASETPHASE{ch}";
            input "RXPOLARITY{ch}";
            input "RXPOWERDOWN{ch}"[2];
            output "RXPRBSERR{ch}";
            output "RXRECCLK{ch}";
            input "RXRESET{ch}";
            output "RXRUNDISP{ch}"[2];
            input "RXSLIDE{ch}";
            output "RXSTATUS{ch}"[3];
            input "RXUSRCLK{ch}";
            input "RXUSRCLK2{ch}";
            output "RXVALID{ch}";

            input "TXBUFDIFFCTRL{ch}"[3];
            output "TXBUFSTATUS{ch}"[2];
            input "TXBYPASS8B10B{ch}"[2];
            input "TXCHARDISPMODE{ch}"[2];
            input "TXCHARDISPVAL{ch}"[2];
            input "TXCHARISK{ch}"[2];
            input "TXCOMSTART{ch}";
            input "TXCOMTYPE{ch}";
            input "TXDATA{ch}"[16];
            input "TXDATAWIDTH{ch}";
            input "TXDETECTRX{ch}";
            input "TXDIFFCTRL{ch}"[3];
            input "TXELECIDLE{ch}";
            input "TXENC8B10BUSE{ch}";
            input "TXENPRBSTST{ch}"[2];
            input "TXINHIBIT{ch}";
            output "TXKERR{ch}"[2];
            output "TXOUTCLK{ch}";
            input "TXPOLARITY{ch}";
            input "TXPOWERDOWN{ch}"[2];
            input "TXPREEMPHASIS{ch}"[3];
            input "TXRESET{ch}";
            output "TXRUNDISP{ch}"[2];
            input "TXUSRCLK{ch}";
            input "TXUSRCLK2{ch}";

            input "TSTPWRDN{ch}"[5];
            input "TSTPWRDNOVRD{ch}";
        }

        input SCANEN;
        input SCANIN;
        input SCANMODE;
        output SCANOUT;

        pad REFCLKP: input;
        pad REFCLKN: input;
        pad RXP[2]: input;
        pad RXN[2]: input;
        pad TXP[2]: output;
        pad TXN[2]: output;
        pad AVTTRX, AVTTTX: power;
        pad AVCC, AVCCPLL: power;
        pad RREF: analog;
        pad AVTTRXC: power;

        // start address 0x000
        attribute DRP: bitvec[16][80];
        attribute DRP_MASK: bool;

        attribute MUX_CLKIN: GTP_MUX_CLKIN;
        attribute MUX_CLKOUT_NORTH: GTP_MUX_CLKOUT_NORTH;
        attribute MUX_CLKOUT_SOUTH: GTP_MUX_CLKOUT_SOUTH;

        attribute CLKINDC_B: bool;
        attribute OVERSAMPLE_MODE: bool;
        attribute PLL_STARTUP_EN: bool;
        attribute SYS_CLK_EN: bool;
        attribute TERMINATION_OVRD: bool;
        attribute CLK25_DIVIDER: GTP_CLK25_DIVIDER;
        attribute OOB_CLK_DIVIDER: GTP_OOB_CLK_DIVIDER;
        attribute PLL_DIVSEL_FB: GTP_PLL_DIVSEL_FB;
        attribute PLL_DIVSEL_REF: GTP_PLL_DIVSEL_REF;
        attribute PLL_TXDIVSEL_COMM_OUT: GTP_PLL_DIVSEL_OUT;
        attribute TX_SYNC_FILTERB: bitvec[1];
        attribute PLLLKDET_CFG: bitvec[3];
        attribute TERMINATION_CTRL: bitvec[5];
        attribute PCS_COM_CFG: bitvec[28];
        attribute PMA_COM_CFG: bitvec[90];

        for ch in 0..2 {
            attribute "USRCLK_ENABLE_{ch}": bool;

            attribute "AC_CAP_DIS_{ch}": bool;
            attribute "CHAN_BOND_SEQ_2_USE_{ch}": bool;
            attribute "CLK_CORRECT_USE_{ch}": bool;
            attribute "CLK_COR_KEEP_IDLE_{ch}": bool;
            attribute "CLK_COR_INSERT_IDLE_FLAG_{ch}": bool;
            attribute "CLK_COR_PRECEDENCE_{ch}": bool;
            attribute "CLK_COR_SEQ_2_USE_{ch}": bool;
            attribute "COMMA_DOUBLE_{ch}": bool;
            attribute "DEC_MCOMMA_DETECT_{ch}": bool;
            attribute "DEC_PCOMMA_DETECT_{ch}": bool;
            attribute "DEC_VALID_COMMA_ONLY_{ch}": bool;
            attribute "MCOMMA_DETECT_{ch}": bool;
            attribute "PCOMMA_DETECT_{ch}": bool;
            attribute "PCI_EXPRESS_MODE_{ch}": bool;
            attribute "PLL_SATA_{ch}": bool;
            attribute "RCV_TERM_GND_{ch}": bool;
            attribute "RCV_TERM_MID_{ch}": bool;
            attribute "RCV_TERM_VTTRX_{ch}": bool;
            attribute "RX_BUFFER_USE_{ch}": bool;
            attribute "RX_CDR_FORCE_ROTATE_{ch}": bool;
            attribute "RX_DECODE_SEQ_MATCH_{ch}": bool;
            attribute "RX_LOSS_OF_SYNC_FSM_{ch}": bool;
            attribute "TX_BUFFER_USE_{ch}": bool;
            attribute "TX_DIFF_BOOST_{ch}": bool;
            attribute "ALIGN_COMMA_WORD_{ch}": GTP_ALIGN_COMMA_WORD;
            attribute "CHAN_BOND_MODE_{ch}": GTP_CHAN_BOND_MODE;
            attribute "CHAN_BOND_SEQ_LEN_{ch}": GTP_SEQ_LEN;
            attribute "CLK_COR_ADJ_LEN_{ch}": GTP_SEQ_LEN;
            attribute "CLK_COR_DET_LEN_{ch}": GTP_SEQ_LEN;
            attribute "PLL_RXDIVSEL_OUT_{ch}": GTP_PLL_DIVSEL_OUT;
            attribute "PLL_TXDIVSEL_OUT_{ch}": GTP_PLL_DIVSEL_OUT;
            attribute "RX_LOS_INVALID_INCR_{ch}": GTP_RX_LOS_INVALID_INCR;
            attribute "RX_LOS_THRESHOLD_{ch}": GTP_RX_LOS_THRESHOLD;
            attribute "RX_SLIDE_MODE_{ch}": GTP_RX_SLIDE_MODE;
            attribute "RX_STATUS_FMT_{ch}": GTP_RX_STATUS_FMT;
            attribute "RX_XCLK_SEL_{ch}": GTP_RX_XCLK_SEL;
            attribute "TX_XCLK_SEL_{ch}": GTP_TX_XCLK_SEL;
            attribute "TERMINATION_IMP_{ch}": GTP_TERMINATION_IMP;
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
            attribute "CHAN_BOND_LEVEL_{ch}": bitvec[3];
            attribute "CLK_COR_REPEAT_WAIT_{ch}": bitvec[5];
            attribute "TXOUTCLK_SEL_{ch}": bitvec[1];
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
            attribute "COMMA_10B_ENABLE_{ch}": bitvec[10];
            attribute "COM_BURST_VAL_{ch}": bitvec[4];
            attribute "MCOMMA_10B_VALUE_{ch}": bitvec[10];
            attribute "OOBDETECT_THRESHOLD_{ch}": bitvec[3];
            attribute "PCOMMA_10B_VALUE_{ch}": bitvec[10];
            attribute "SATA_BURST_VAL_{ch}": bitvec[3];
            attribute "SATA_IDLE_VAL_{ch}": bitvec[3];
            attribute "TXRX_INVERT_{ch}": bitvec[5];
            attribute "PMA_CDR_SCAN_{ch}": bitvec[27];
            attribute "PMA_RX_CFG_{ch}": bitvec[25];
            attribute "PRBS_ERR_THRESHOLD_{ch}": bitvec[32];
            attribute "TRANS_TIME_FROM_P2_{ch}": bitvec[16];
            attribute "TRANS_TIME_NON_P2_{ch}": bitvec[16];
            attribute "TRANS_TIME_TO_P2_{ch}": bitvec[16];
            attribute "TX_DETECT_RX_CFG_{ch}": bitvec[14];
        }
    }

    bel_class GTX_DUAL {
        input DCLK;
        input DEN;
        input DWE;
        input DADDR[7];
        input DI[16];
        output DRDY;
        output DO[16];


        input GREFCLK;
        input GTXRESET;
        input GTXTEST[14];
        input INTDATAWIDTH;
        output PLLLKDET;
        input PLLLKDETEN;
        input PLLPOWERDOWN;
        output REFCLKOUT;
        input REFCLKPWRDNB;

        input PMAAMUX[3];
        output PMATSTCLK;
        input PMATSTCLKSEL[3];

        for ch in 0..2 {
            input "DFECLKDLYADJ{ch}"[6];
            output "DFECLKDLYADJMONITOR{ch}"[6];
            output "DFEEYEDACMONITOR{ch}"[5];
            output "DFESENSCAL{ch}"[3];
            input "DFETAP1{ch}"[5];
            output "DFETAP1MONITOR{ch}"[5];
            input "DFETAP2{ch}"[5];
            output "DFETAP2MONITOR{ch}"[5];
            input "DFETAP3{ch}"[4];
            output "DFETAP3MONITOR{ch}"[4];
            input "DFETAP4{ch}"[4];
            output "DFETAP4MONITOR{ch}"[4];

            input "LOOPBACK{ch}"[3];
            output "PHYSTATUS{ch}";
            input "PRBSCNTRESET{ch}";
            output "RESETDONE{ch}";

            input "RXBUFRESET{ch}";
            output "RXBUFSTATUS{ch}"[3];
            output "RXBYTEISALIGNED{ch}";
            output "RXBYTEREALIGN{ch}";
            input "RXCDRRESET{ch}";
            output "RXCHANBONDSEQ{ch}";
            output "RXCHANISALIGNED{ch}";
            output "RXCHANREALIGN{ch}";
            output "RXCHARISCOMMA{ch}"[4];
            output "RXCHARISK{ch}"[4];
            input "RXCHBONDI{ch}"[4];
            output "RXCHBONDO{ch}"[4];
            output "RXCLKCORCNT{ch}"[3];
            output "RXCOMMADET{ch}";
            input "RXCOMMADETUSE{ch}";
            output "RXDATA{ch}"[32];
            output "RXDATAVALID{ch}";
            input "RXDATAWIDTH{ch}"[2];
            input "RXDEC8B10BUSE{ch}";
            output "RXDISPERR{ch}"[4];
            output "RXELECIDLE{ch}";
            input "RXENCHANSYNC{ch}";
            input "RXENEQB{ch}";
            input "RXENMCOMMAALIGN{ch}";
            input "RXENPCOMMAALIGN{ch}";
            input "RXENPMAPHASEALIGN{ch}";
            input "RXENPRBSTST{ch}"[2];
            input "RXENSAMPLEALIGN{ch}";
            input "RXEQMIX{ch}"[2];
            input "RXEQPOLE{ch}"[4];
            input "RXGEARBOXSLIP{ch}";
            output "RXHEADER{ch}"[3];
            output "RXHEADERVALID{ch}";
            output "RXLOSSOFSYNC{ch}"[2];
            output "RXNOTINTABLE{ch}"[4];
            output "RXOVERSAMPLEERR{ch}";
            input "RXPMASETPHASE{ch}";
            input "RXPOLARITY{ch}";
            input "RXPOWERDOWN{ch}"[2];
            output "RXPRBSERR{ch}";
            output "RXRECCLK{ch}";
            input "RXRESET{ch}";
            output "RXRUNDISP{ch}"[4];
            input "RXSLIDE{ch}";
            output "RXSTARTOFSEQ{ch}";
            output "RXSTATUS{ch}"[3];
            input "RXUSRCLK{ch}";
            input "RXUSRCLK2{ch}";
            output "RXVALID{ch}";
            input "TSTPWRDN{ch}"[5];
            input "TSTPWRDNOVRD{ch}";
            input "TXBUFDIFFCTRL{ch}"[3];
            output "TXBUFSTATUS{ch}"[2];
            input "TXBYPASS8B10B{ch}"[4];
            input "TXCHARDISPMODE{ch}"[4];
            input "TXCHARDISPVAL{ch}"[4];
            input "TXCHARISK{ch}"[4];
            input "TXCOMSTART{ch}";
            input "TXCOMTYPE{ch}";
            input "TXDATA{ch}"[32];
            input "TXDATAWIDTH{ch}"[2];
            input "TXDETECTRX{ch}";
            input "TXDIFFCTRL{ch}"[3];
            input "TXELECIDLE{ch}";
            input "TXENC8B10BUSE{ch}";
            input "TXENPMAPHASEALIGN{ch}";
            input "TXENPRBSTST{ch}"[2];
            output "TXGEARBOXREADY{ch}";
            input "TXHEADER{ch}"[3];
            input "TXINHIBIT{ch}";
            output "TXKERR{ch}"[4];
            output "TXOUTCLK{ch}";
            input "TXPMASETPHASE{ch}";
            input "TXPOLARITY{ch}";
            input "TXPOWERDOWN{ch}"[2];
            input "TXPREEMPHASIS{ch}"[4];
            input "TXRESET{ch}";
            output "TXRUNDISP{ch}"[4];
            input "TXSEQUENCE{ch}"[7];
            input "TXSTARTSEQ{ch}";
            input "TXUSRCLK{ch}";
            input "TXUSRCLK2{ch}";
        }

        input SCANEN;
        input SCANINPCS0;
        input SCANINPCS1;
        input SCANINPCSCOMMON;
        input SCANMODE;
        output SCANOUTPCS0;
        output SCANOUTPCS1;
        output SCANOUTPCSCOMMON;

        pad REFCLKP: input;
        pad REFCLKN: input;
        pad RXP[2]: input;
        pad RXN[2]: input;
        pad TXP[2]: output;
        pad TXN[2]: output;
        pad AVTTRX, AVTTTX: power;
        pad AVCC, AVCCPLL: power;
        pad RREF: analog;
        pad AVTTRXC: power;

        // start address 0x000
        attribute DRP: bitvec[16][80];
        attribute DRP_MASK: bool;

        attribute MUX_CLKIN: GTP_MUX_CLKIN;
        attribute MUX_CLKOUT_NORTH: GTP_MUX_CLKOUT_NORTH;
        attribute MUX_CLKOUT_SOUTH: GTP_MUX_CLKOUT_SOUTH;

        attribute CLKINDC_B: bool;
        attribute CLKRCV_TRST: bool;
        attribute OVERSAMPLE_MODE: bool;
        attribute PLL_FB_DCCEN: bool;
        attribute PLL_STARTUP_EN: bool;
        attribute RX_EN_IDLE_HOLD_CDR: bool;
        attribute RX_EN_IDLE_RESET_FR: bool;
        attribute RX_EN_IDLE_RESET_PH: bool;
        attribute TERMINATION_OVRD: bool;
        attribute CLK25_DIVIDER: GTP_CLK25_DIVIDER;
        attribute OOB_CLK_DIVIDER: GTP_OOB_CLK_DIVIDER;
        attribute PLL_DIVSEL_FB: GTP_PLL_DIVSEL_FB;
        attribute PLL_DIVSEL_REF: GTP_PLL_DIVSEL_REF;
        attribute CDR_PH_ADJ_TIME: bitvec[5];
        attribute DFE_CAL_TIME: bitvec[5];
        attribute TERMINATION_CTRL: bitvec[5];
        attribute PLL_LKDET_CFG: bitvec[3];
        attribute PLL_COM_CFG: bitvec[24];
        attribute PLL_CP_CFG: bitvec[8];
        attribute PLL_TDCC_CFG: bitvec[3];
        attribute PMA_COM_CFG: bitvec[69];

        for ch in 0..2 {
            attribute "USRCLK_ENABLE_{ch}": bool;

            attribute "AC_CAP_DIS_{ch}": bool;
            attribute "CHAN_BOND_KEEP_ALIGN_{ch}": bool;
            attribute "CHAN_BOND_SEQ_2_USE_{ch}": bool;
            attribute "CLK_COR_INSERT_IDLE_FLAG_{ch}": bool;
            attribute "CLK_COR_KEEP_IDLE_{ch}": bool;
            attribute "CLK_COR_PRECEDENCE_{ch}": bool;
            attribute "CLK_CORRECT_USE_{ch}": bool;
            attribute "CLK_COR_SEQ_2_USE_{ch}": bool;
            attribute "COMMA_DOUBLE_{ch}": bool;
            attribute "DEC_MCOMMA_DETECT_{ch}": bool;
            attribute "DEC_PCOMMA_DETECT_{ch}": bool;
            attribute "DEC_VALID_COMMA_ONLY_{ch}": bool;
            attribute "MCOMMA_DETECT_{ch}": bool;
            attribute "PCI_EXPRESS_MODE_{ch}": bool;
            attribute "PCOMMA_DETECT_{ch}": bool;
            attribute "PLL_SATA_{ch}": bool;
            attribute "RCV_TERM_GND_{ch}": bool;
            attribute "RCV_TERM_VTTRX_{ch}": bool;
            attribute "RX_BUFFER_USE_{ch}": bool;
            attribute "RX_CDR_FORCE_ROTATE_{ch}": bool;
            attribute "RX_DECODE_SEQ_MATCH_{ch}": bool;
            attribute "RX_EN_IDLE_HOLD_DFE_{ch}": bool;
            attribute "RX_EN_IDLE_RESET_BUF_{ch}": bool;
            attribute "RXGEARBOX_USE_{ch}": bool;
            attribute "RX_LOSS_OF_SYNC_FSM_{ch}": bool;
            attribute "TX_BUFFER_USE_{ch}": bool;
            attribute "TXGEARBOX_USE_{ch}": bool;
            attribute "ALIGN_COMMA_WORD_{ch}": GTP_ALIGN_COMMA_WORD;
            attribute "CHAN_BOND_MODE_{ch}": GTP_CHAN_BOND_MODE;
            attribute "CHAN_BOND_SEQ_LEN_{ch}": GTP_SEQ_LEN;
            attribute "CLK_COR_ADJ_LEN_{ch}": GTP_SEQ_LEN;
            attribute "CLK_COR_DET_LEN_{ch}": GTP_SEQ_LEN;
            attribute "PLL_RXDIVSEL_OUT_{ch}": GTP_PLL_DIVSEL_OUT;
            attribute "PLL_TXDIVSEL_OUT_{ch}": GTP_PLL_DIVSEL_OUT;
            attribute "RX_LOS_INVALID_INCR_{ch}": GTP_RX_LOS_INVALID_INCR;
            attribute "RX_LOS_THRESHOLD_{ch}": GTP_RX_LOS_THRESHOLD;
            attribute "RX_SLIDE_MODE_{ch}": GTP_RX_SLIDE_MODE;
            attribute "RX_STATUS_FMT_{ch}": GTP_RX_STATUS_FMT;
            attribute "RX_XCLK_SEL_{ch}": GTP_RX_XCLK_SEL;
            attribute "TX_XCLK_SEL_{ch}": GTP_TX_XCLK_SEL;
            attribute "TERMINATION_IMP_{ch}": GTP_TERMINATION_IMP;
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
            attribute "CHAN_BOND_LEVEL_{ch}": bitvec[3];
            attribute "CB2_INH_CC_PERIOD_{ch}": bitvec[4];
            attribute "CLK_COR_REPEAT_WAIT_{ch}": bitvec[5];
            attribute "TXOUTCLK_SEL_{ch}": bitvec[1];
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
            attribute "DFE_CFG_{ch}": bitvec[10];
            attribute "GEARBOX_ENDEC_{ch}": bitvec[3];
            attribute "MCOMMA_10B_VALUE_{ch}": bitvec[10];
            attribute "OOBDETECT_THRESHOLD_{ch}": bitvec[3];
            attribute "PCOMMA_10B_VALUE_{ch}": bitvec[10];
            attribute "RX_IDLE_HI_CNT_{ch}": bitvec[4];
            attribute "RX_IDLE_LO_CNT_{ch}": bitvec[4];
            attribute "SATA_BURST_VAL_{ch}": bitvec[3];
            attribute "SATA_IDLE_VAL_{ch}": bitvec[3];
            attribute "TXRX_INVERT_{ch}": bitvec[3];
            attribute "TX_IDLE_DELAY_{ch}": bitvec[3];
            attribute "PMA_CDR_SCAN_{ch}": bitvec[27];
            attribute "PMA_RXSYNC_CFG_{ch}": bitvec[7];
            attribute "PMA_RX_CFG_{ch}": bitvec[25];
            attribute "PMA_TX_CFG_{ch}": bitvec[20];
            attribute "PRBS_ERR_THRESHOLD_{ch}": bitvec[32];
            attribute "TRANS_TIME_FROM_P2_{ch}": bitvec[12];
            attribute "TRANS_TIME_NON_P2_{ch}": bitvec[8];
            attribute "TRANS_TIME_TO_P2_{ch}": bitvec[10];
            attribute "TX_DETECT_RX_CFG_{ch}": bitvec[14];
        }
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
        wire MGT_ROW_I[5]: branch HCLK_ROW_PREV;
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
        region_slot HROW;
        region_slot LEAF;
        region_slot LEAF_IO;

        wire PULLUP: pullup;
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

        wire IMUX_SPEC[4]: test;

        wire HCLK_ROW[12]: regional HROW;
        wire RCLK_ROW[6]: regional HROW;
        wire MGT_ROW[10]: regional HROW;
        wire HCLK_BUF[12]: mux;
        wire RCLK_BUF[6]: mux;
        wire PERF_ROW[4]: branch HCLK_ROW_NEXT;
        wire PERF_ROW_OUTER[4]: mux;

        wire PERF_BUF[4]: mux;
        wire HCLK_IO[12]: regional LEAF_IO;
        wire RCLK_IO[6]: regional LEAF_IO;
        wire IMUX_IDELAYCTRL_REFCLK: mux;
        wire IMUX_BUFIO[4]: mux;
        wire IMUX_BUFR[2]: mux;
        wire VRCLK[2]: bel;
        wire VRCLK_S[2]: branch IO_N;
        wire VRCLK_N[2]: branch IO_S;
        wire SIOCLK[2]: bel;
        wire VIOCLK[2]: bel;
        wire VIOCLK_S[2]: branch IO_N;
        wire VIOCLK_N[2]: branch IO_S;
        wire VIOCLK_S_BUF[2]: mux;
        wire VIOCLK_N_BUF[2]: mux;
        wire IOCLK[8]: regional LEAF_IO;
        wire VOCLK[2]: mux;
        wire VOCLK_S[2]: branch IO_N;
        wire VOCLK_N[2]: branch IO_S;
        wire OCLK[2]: regional LEAF_IO;

        wire OUT_CLKPAD: bel;
        wire IMUX_IOI_ICLK[2]: mux;
        wire IMUX_IOI_OCLK[2]: mux;
        wire IMUX_IOI_OCLKDIV[2]: mux;
        wire IMUX_IOI_OCLKPERF: mux;

        wire IMUX_GTX_PERFCLK: mux;

        wire GCLK[32]: regional GLOBAL;
        wire GIOB[8]: regional GLOBAL;
        wire OUT_BUFG[16]: bel;
        wire OUT_BUFG_GFB[16]: mux;
        wire IMUX_BUFG_O[32]: mux;
        wire IMUX_BUFG_I[32]: branch CLK_PREV;

        // buffered for BUFH inputs
        wire GCLK_CMT[32]: mux;
        wire BUFH_INT_W[2]: mux;
        wire BUFH_INT_E[2]: mux;
        wire GCLK_TEST[32]: mux;
        wire GCLK_TEST_IN[32]: mux;
        wire BUFH_TEST_W: mux;
        wire BUFH_TEST_E: mux;
        wire BUFH_TEST_W_IN: mux;
        wire BUFH_TEST_E_IN: mux;
        wire IMUX_BUFHCE_W[12]: mux;
        wire IMUX_BUFHCE_E[12]: mux;
        // buffered for CMT ins
        wire CCIO_CMT_W[4]: mux;
        wire CCIO_CMT_E[4]: mux;
        wire MGT_CMT_W[10]: mux;
        wire MGT_CMT_E[10]: mux;
        wire HCLK_CMT_W[12]: mux;
        wire HCLK_CMT_E[12]: mux;
        wire RCLK_CMT_W[12]: mux;
        wire RCLK_CMT_E[12]: mux;
        wire GIOB_CMT[8]: mux;

        wire IMUX_MMCM_CLKIN1_HCLK_W[2]: mux;
        wire IMUX_MMCM_CLKIN2_HCLK_W[2]: mux;
        wire IMUX_MMCM_CLKFB_HCLK_W[2]: mux;
        wire IMUX_MMCM_CLKIN1_HCLK_E[2]: mux;
        wire IMUX_MMCM_CLKIN2_HCLK_E[2]: mux;
        wire IMUX_MMCM_CLKFB_HCLK_E[2]: mux;
        wire IMUX_MMCM_CLKIN1_HCLK[2]: mux;
        wire IMUX_MMCM_CLKIN2_HCLK[2]: mux;
        wire IMUX_MMCM_CLKFB_HCLK[2]: mux;
        wire IMUX_MMCM_CLKIN1_IO[2]: mux;
        wire IMUX_MMCM_CLKIN2_IO[2]: mux;
        wire IMUX_MMCM_CLKFB_IO[2]: mux;
        wire IMUX_MMCM_CLKIN1_MGT[2]: mux;
        wire IMUX_MMCM_CLKIN2_MGT[2]: mux;
        wire IMUX_MMCM_CLKIN1[2]: mux;
        wire IMUX_MMCM_CLKIN2[2]: mux;
        wire IMUX_MMCM_CLKFB[2]: mux;
        wire OUT_MMCM_S[14]: bel;
        wire OUT_MMCM_N[14]: bel;
        wire OMUX_MMCM_MMCM[2]: bel;
        wire OMUX_MMCM_PERF_S[4]: bel;
        wire OMUX_MMCM_PERF_N[4]: bel;
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
            bel_slot SLICE[4]: SLICE_V5;
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
        } else if variant virtex5 {
            bel_slot BRAM: BRAM_V5;
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
        } else if variant virtex5 {
            bel_slot DSP[2]: DSP_V5;
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

        if variant [virtex4, virtex5] {
            bel_slot ILOGIC[2]: ILOGIC;
            bel_slot OLOGIC[2]: OLOGIC;
            bel_slot IODELAY[2]: IODELAY_V5;
            bel_slot IDELAY[2]: legacy;
            bel_slot ODELAY[2]: legacy;
            bel_slot IOB[2]: IOB;
        } else if variant virtex6 {
            bel_slot ILOGIC[2]: ILOGIC;
            bel_slot OLOGIC[2]: OLOGIC;
            bel_slot IODELAY[2]: IODELAY_V6;
            bel_slot IDELAY[2]: legacy;
            bel_slot ODELAY[2]: legacy;
            bel_slot IOB[2]: IOB;
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
            bel_slot DCM[2]: DCM_V5;
        }
        if variant [virtex4, virtex5] {
            bel_slot PLL[2]: PLL_V5;
        } else if variant virtex6 {
            bel_slot PLL[2]: PLL_V6;
        } else {
            bel_slot PLL[2]: legacy;
        }
        bel_slot CMT_A: legacy;
        bel_slot CMT_B: legacy;
        bel_slot CMT_C: legacy;
        bel_slot CMT_D: legacy;
        bel_slot HCLK_CMT: legacy;
        bel_slot PPR_FRAME: PPR_FRAME;
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
                cell IO_W[8];
                cell IO_E[8];
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

        if variant virtex6 {
            bel_slot BUFHCE_W[12]: BUFHCE;
            bel_slot BUFHCE_E[12]: BUFHCE;
        } else {
            bel_slot BUFHCE_W[12]: legacy;
            bel_slot BUFHCE_E[12]: legacy;
        }
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

        bel_slot PMV_CLK: PMV;
        bel_slot PMVIOB_CLK: PMVIOB;
        bel_slot PMV2: PMV2;
        bel_slot PMV2_SVT: PMV2;
        bel_slot MTBF2: MTBF2;
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
            bel_slot PPC: PPC440;
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
            bel_slot EMAC: EMAC_V6;
        }
        if variant [virtex5, virtex6] {
            tile_class EMAC {
                cell CELL[10];
                bitrect MAIN[10]: BRAM;
            }
        }

        if variant virtex5 {
            bel_slot PCIE: PCIE_V5;
        } else if variant virtex6 {
            bel_slot PCIE: legacy;
        } else {
            bel_slot PCIE: legacy;
        }
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

        bel_slot GTP_DUAL: GTP_DUAL;
        if variant virtex5 {
            tile_class GTP {
                cell CELL[20];
                bitrect MAIN[20]: GT;
                bitrect HCLK: HCLK_GT;
            }
        }

        bel_slot GTX_DUAL: GTX_DUAL;
        if variant virtex5 {
            tile_class GTX {
                cell CELL[20];
                bitrect MAIN[20]: GT;
                bitrect HCLK: HCLK_GT;
            }
        }

        bel_slot HCLK_GTX: legacy;
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
        bel_slot CRC32[4]: CRC32;

        bel_slot IPAD_CLKP[2]: legacy;
        bel_slot IPAD_CLKN[2]: legacy;
        bel_slot IPAD_RXP[4]: legacy;
        bel_slot IPAD_RXN[4]: legacy;
        bel_slot OPAD_TXP[4]: legacy;
        bel_slot OPAD_TXN[4]: legacy;

        if variant [virtex4, virtex5, virtex6] {
            bel_slot BUFGCTRL[32]: BUFGCTRL;
        } else {
            bel_slot BUFGCTRL[32]: legacy;
        }
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
                cell IO_W[2];
                cell IO_E[2];
                bitrect MAIN[2]: CMT;
            }
        }
        if variant virtex7 {
            tile_class CLK_BUFG {
                cell CELL[4];
                bitrect MAIN[4]: CLK;
            }
        }

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
        bel_slot BSCAN[4]: BSCAN;
        if variant [virtex4, virtex5] {
            bel_slot ICAP[2]: ICAP_V4;
        } else {
            bel_slot ICAP[2]: ICAP_V6;
        }
        bel_slot STARTUP: STARTUP;
        bel_slot CAPTURE: CAPTURE;
        bel_slot JTAGPPC: JTAGPPC;
        bel_slot PMV_CFG[2]: PMV;
        bel_slot DCIRESET: DCIRESET;
        if variant [virtex4, virtex5] {
            bel_slot FRAME_ECC: FRAME_ECC_V4;
        } else {
            bel_slot FRAME_ECC: FRAME_ECC_V6;
        }
        bel_slot USR_ACCESS: USR_ACCESS;
        bel_slot KEY_CLEAR: KEY_CLEAR;
        bel_slot EFUSE_USR: EFUSE_USR;
        bel_slot DNA_PORT: DNA_PORT;
        if variant [virtex4, virtex5, virtex6] {
            bel_slot CFG_IO_ACCESS: CFG_IO_ACCESS_V6;
        } else {
            bel_slot CFG_IO_ACCESS: CFG_IO_ACCESS_V7;
        }
        bel_slot PMVIOB_CFG: PMVIOB;
        bel_slot MISC_CFG: MISC_CFG;
        if variant virtex4 {
            bel_slot SYSMON: SYSMON_V4;
        } else if variant [virtex5, virtex6] {
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

        if variant [virtex4, virtex5] {
            tile_class HCLK_MGT_BUF {
                cell CELL;
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
        if variant [virtex4, virtex5, virtex6] {
            bel_slot HCLK: routing;
        } else {
            bel_slot HCLK: legacy;
        }
        bel_slot HCLK_W: legacy;
        bel_slot HCLK_E: legacy;
        bel_slot GLOBALSIG: GLOBALSIG;
        if variant [virtex4, virtex5, virtex6] {
            bel_slot HCLK_DRP: HCLK_DRP_V6;
        } else {
            // TODO: v7 variant
            bel_slot HCLK_DRP: legacy;
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
        if variant [virtex4, virtex5] {
            bel_slot PMVBRAM: PMVBRAM_V5;
        } else {
            bel_slot PMVBRAM: PMVBRAM_V6;
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
        if variant [virtex4, virtex5, virtex6] {
            bel_slot BUFR[4]: BUFR;
            bel_slot BUFIO[4]: BUFIO;
            bel_slot IDELAYCTRL: IDELAYCTRL;
            bel_slot DCI: DCI;
        } else {
            bel_slot BUFR[4]: legacy;
            bel_slot BUFIO[4]: legacy;
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
                cell CELL[8];
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
        if variant virtex6 {
            tile_class GLOBAL {
                bitrect COR0: REG32;
                bitrect COR1: REG32;
                bitrect CTL0: REG32;
                bitrect CTL1: REG32;
                bitrect TIMER: REG32;
                bitrect WBSTAR: REG32;
                bitrect TESTMODE: REG32;
                bitrect TRIM: REG32;
                bitrect UNK1C: REG32;
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
        if variant [virtex4, virtex5, virtex6] {
            connector_class IO_S {
                if variant virtex4 {
                    pass IOCLK_N = IOCLK;
                } else if variant virtex6 {
                    pass VIOCLK_N = VIOCLK;
                    pass VOCLK_N = VOCLK;
                }
                pass VRCLK_N = VRCLK;
            }
        }
    }
    connector_slot IO_N {
        opposite IO_S;
        if variant [virtex4, virtex5, virtex6] {
            connector_class IO_N {
                if variant virtex4 {
                    pass IOCLK_S = IOCLK;
                } else if variant virtex6 {
                    pass VIOCLK_S = VIOCLK;
                    pass VOCLK_S = VOCLK;
                }
                pass VRCLK_S = VRCLK;
            }
        }
    }

    connector_slot CLK_PREV {
        opposite CLK_NEXT;
        if variant [virtex4, virtex5, virtex6] {
            connector_class CLK_PREV {
                pass IMUX_BUFG_I = IMUX_BUFG_O;
            }
        }
    }
    connector_slot CLK_NEXT {
        opposite CLK_PREV;
        if variant [virtex4, virtex5, virtex6] {
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

    connector_slot HCLK_ROW_PREV {
        opposite HCLK_ROW_NEXT;
        if variant virtex5 {
            connector_class HCLK_ROW_PREV {
                pass MGT_ROW_I = MGT_ROW_O;
            }
            connector_class HCLK_ROW_PREV_PASS {
                pass MGT_ROW_I = MGT_ROW_I;
            }
        }
        if variant virtex6 {
            connector_class HCLK_ROW_PREV;
        }
    }

    connector_slot HCLK_ROW_NEXT {
        opposite HCLK_ROW_PREV;
        if variant virtex5 {
            connector_class HCLK_ROW_NEXT;
        }
        if variant virtex6 {
            connector_class HCLK_ROW_NEXT {
                pass PERF_ROW = PERF_ROW_OUTER;
            }
            connector_class HCLK_ROW_NEXT_PASS {
                pass PERF_ROW = PERF_ROW;
            }
        }
    }
}
