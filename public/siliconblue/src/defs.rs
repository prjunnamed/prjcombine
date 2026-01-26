use prjcombine_entity::id::EntityStaticRange;
use prjcombine_interconnect::db::WireSlotId;
use prjcombine_tablegen::target_defs;

target_defs! {
    // Selects `CI` input routing on `LC0`.
    enum LC_MUX_CI {
        // Constant zero.
        ZERO,
        // Constant one.
        ONE,
        // `CO` of `LC7` in the previous PLB.
        CHAIN,
    }

    // Logic Cell: SB_LUT4 + SB_CARRY + SB_DFF*
    bel_class LC {
        // LUT inputs from interconnect.
        input I0, I1, I2, I3;

        // Bypass LUT input, replacing `I2` if enabled. Only present on iCE40.
        // Almost always non-routable and directly connected to LTOUT of previous LUT.
        // However, on iCE40T04/T01/T05, some PLBs on the west/east edges have it connected
        // to misc hard IP output.  Even though it's a direct connection, we treat it as routable
        // interconnect so that we can store the hard IP pin mapping in the database.
        input LTIN;
        // Bypass LUT output. Always connected directly to the combinational LUT output,
        // even if FF is enabled.
        nonroutable output LTOUT;

        // Carry input. For `LC[1..=7]`, directly connected to the previous LC's `CO`.
        // For `LC0`, selected by `MUX_CI`.
        nonroutable input CI;
        // Carry output.
        nonroutable output CO;
        // Mirror of `CI`.  This is a hack: the hardware has a direct path from `CI` to `I3`,
        // but it is implemented as part of the general interconnect mux for `MUX_LC_I3`.
        output CI_OUT;

        // Control signals, shared with all other LCs in the tile.
        input CE, RST, CLK;

        // The main output.  Connected to FF output if `FF_ENABLE` is set, otherwise to LUT output.
        output O;

        // Must be set to enable the carry chain.
        attribute CARRY_ENABLE: bool;

        // Present on iCE40 only.  Replaces the `I2` input to the LUT with `LTIN`.
        attribute LTIN_ENABLE: bool;

        // If set, `O` is connected to the FF output.  Otherwise, it is connected to the LUT output.
        attribute FF_ENABLE: bool;

        // If set, the FF has an asynchronous reset.  Otherwise, the reset is synchronous.
        attribute FF_SR_ASYNC: bool;

        // The `RST` input sets the FF to this value.  Note that the initial value is independent
        // from this setting, and is always 0.
        attribute FF_SR_VALUE: bitvec[1];

        // The LUT truth table.
        attribute LUT_INIT: bitvec[16];

        // Present only on `LC0`.  Selects what is routed to the `CI` input.
        attribute MUX_CI: LC_MUX_CI;
    }

    // I/O interface: input and output registers.
    // On iCE40R04, also used to connect the hard IP to the interconnect.
    bel_class IOI {
        // Output value and output enable from fabric.
        input DOUT0, DOUT1, OE;
        // Input value to fabric.
        output DIN0, DIN1;

        // Signals to the IOB, or whatever is replacing one.
        // Almost always non-routable and directly connected to the IOB.
        // However, on iCE40R04, IOIs on the west/east edges have it connected
        // to misc hard IP pins.  Even though it's a direct connection, we treat it as routable
        // interconnect so that we can store the hard IP pin mapping in the database.
        input IOB_DIN;
        output IOB_DOUT;
        nonroutable output IOB_OE;

        // Control signals, shared with the other IOI in the tile.
        input CE, ICLK, OCLK;

        // The latch signal, shared with all other IOIs on the same edge.
        input LATCH;

        // The associated pad.  This is a *lie* for convenience reasons, the pad (if present at all)
        // is actually located in the associated IOB.
        pad PAD: inout;

        // TODO: split into individual fields?
        attribute PIN_TYPE: bitvec[6];

        // Only on iCE40T04/T01/T05.  Has to be set to ungate OE.  Purpose unknown.
        // TODO: ???
        attribute OUTPUT_ENABLE: bool;
    }

    // I/O buffer: the analog part of I/O.
    bel_class IOB {
        // Signals to the IOI.
        nonroutable input DOUT, OE;
        nonroutable output DIN;

        // Note: as a convenience lie, the IOB's pad is included in the IOI instead.
        // pad PAD: inout;

        // Only on iCE65L04/L08/P04 west bank.  Determines the output drive strength for
        // the buffer.
        attribute DRIVE: bitvec[2];
        // Only on iCE65L04/L08/P04 west bank.  If set, uses the CMOS input buffer; otherwise, uses
        // the VREF-based input buffer.
        attribute CMOS_INPUT: bool;
        // Only on iCE65L04/L08/P04 west bank.  Purpose unknown.  Unset for LVCMOS12/15/18 and MDDR
        // I/O standards, set for all others.
        attribute IOSTD_MISC: bitvec[1];

        // Enables the input buffer.  Not present on iCE65, except for the special west bank.
        attribute IBUF_ENABLE: bool;

        // Enables the pullup.  Not present on the iCE65 special west bank.
        // TODO: interaction with other pullup attrs on iCE40T01/T05?
        attribute PULLUP: bool;

        // Only on iCE40T01/T05.  Enables the weak (100kΩ) pullup.
        attribute WEAK_PULLUP: bool;
        // Only on iCE40T01/T05.  Selects pullup strength.  One-hot.
        attribute PULLUP_3P3K, PULLUP_6P8K, PULLUP_10K: bool;

        // Only on iCE40T04/T01/T05.  If set, the relevant dedicated I/O input paths to hard IP
        // are connected to general routing; otherwise, they are connected to this IOB.
        // On iCE40R04, stored in `IOB_PAIR` instead.
        attribute HARDIP_FABRIC_IN: bool;
        // Only on iCE40T04/T01/T05.  If set, the `DOUT` and `OE` inputs of this IOB are connected
        // to the relevant hard IP.  Otherwise, they are connected to the IOI.
        // On iCE40R04, stored in `IOB_PAIR` instead.
        attribute HARDIP_DEDICATED_OUT: bool;
    }

    // Circuitry common to both `IOB`s in a tile.
    bel_class IOB_PAIR {
        // Output to the global nets.  Only usable for a few pads.  Mirror of `GLOBAL_IN`, with
        // `LATCH_GLOBAL_OUT` applied as appropriate.  Note that pads associated with a PLL
        // or a PLL stub are connected through the PLL instead, even if it is not in use.
        output GLOBAL_OUT;
        // Input from one of the associated IOB, if `GLOBAL_OUT` is used.
        nonroutable input GLOBAL_IN;
        // The latch signal, shared with all IOIs on the same edge.
        input LATCH;

        // If set, the differential input buffer is enabled, and connected to IOB0's `DIN`.
        attribute LVDS_INPUT: bool;

        // If set, the `GLOBAL_OUT` signal is latched by `LATCH`.
        attribute LATCH_GLOBAL_OUT: bool;

        // Only on iCE40T04/T01/T05, and only on IOB pairs associated with `I2C`'s or `I2C_FIFO`'s
        // dedicated SDA I/O.  Enables the respective delays on the I2C pin.  Applies even if the
        // pin is connected via general routing.
        attribute SDA_INPUT_DELAYED: bool;
        attribute SDA_OUTPUT_DELAYED: bool;

        // Only on iCE40R04.  See the description of corresponding IOB attributes.  Applies to both
        // IOBs in the tile.
        attribute HARDIP_FABRIC_IN: bool;
        attribute HARDIP_DEDICATED_OUT: bool;
    }

    enum BRAM_MODE {
        _0,
        _1,
        _2,
        _3,
    }

    // A block of RAM.
    bel_class BRAM {
        input MASK[16];
        // `WADDR[8..11]` and `RADDR[8..11]` only exist on iCE40.
        input WADDR[11], RADDR[11];
        input WDATA[16];
        output RDATA[16];
        // `WCLKE` directly gates `WCLK` signal and works correctly. `WE` is buggy.
        input WCLK, WCLKE, WE;
        // `RCLKE` directly gates `RCLK` signal and works correctly. `RE` is buggy.
        input RCLK, RCLKE, RE;

        // Only on iCE40. Must be set if the BRAM is used.
        attribute ENABLE: bool;
        // Only on iCE40.
        attribute WRITE_MODE, READ_MODE: BRAM_MODE;
        // Only on iCE40.  TODO: document.
        attribute CASCADE_IN_WADDR, CASCADE_OUT_WADDR, CASCADE_IN_RADDR, CASCADE_OUT_RADDR: bool;

        // The initial RAM data.
        attribute INIT: bitvec[4096];
    }

    // A DSP block.
    bel_class MAC16 {
        input A[16], B[16], C[16], D[16];
        input AHOLD, BHOLD, CHOLD, DHOLD;
        input ADDSUBBOT, ADDSUBTOP;
        input CI;
        output CO;
        input CLK, CE;
        input IRSTBOT, IRSTTOP, ORSTBOT, ORSTTOP;
        input OHOLDBOT, OHOLDTOP;
        input OLOADBOT, OLOADTOP;
        output O[32];

        // Connected from the previous `MAC16` in chain.
        nonroutable input ACCUMCI, SIGNEXTIN;
        nonroutable output ACCUMCO, SIGNEXTOUT;

        attribute A_REG, B_REG, C_REG, D_REG: bool;
        attribute A_SIGNED, B_SIGNED: bool;
        attribute BOT_8X8_MULT_REG, TOP_8X8_MULT_REG: bool;
        attribute MODE_8X8: bool;
        attribute PIPELINE_16X16_MULT_REG1, PIPELINE_16X16_MULT_REG2: bool;
        attribute BOTADDSUB_CARRYSELECT, TOPADDSUB_CARRYSELECT: bitvec[2];
        attribute BOTADDSUB_LOWERINPUT, TOPADDSUB_LOWERINPUT: bitvec[2];
        attribute BOTADDSUB_UPPERINPUT, TOPADDSUB_UPPERINPUT: bitvec[1];
        attribute BOTOUTPUT_SELECT, TOPOUTPUT_SELECT: bitvec[2];
    }

    // A single-port large RAM.
    bel_class SPRAM {
        input ADDRESS[14];
        input DATAIN[16];
        output DATAOUT[16];
        input MASKWREN[4];
        input CLOCK;
        input CHIPSELECT, WREN;
        input POWEROFF, SLEEP, STANDBY;

        // Test-only inputs, shared between the two SPRAMs in a tile.
        input RDMARGIN[4];
        input RDMARGINEN, TEST;

        // Must be set if the SPRAM is used.
        attribute ENABLE: bool;
    }

    enum FEEDBACK_PATH {
        DELAY,
        SIMPLE,
        PHASE_AND_DELAY,
        EXTERNAL,
    }

    enum PLL65_MODE {
        NONE,
        PLL_PAD,
        PLL_CORE,
        PLL_2_PAD,
    }

    enum PLL65_PLLOUT_PHASE {
        NONE,
        _0DEG,
        _90DEG,
        _180DEG,
        _270DEG,
    }

    bel_class PLL65 {
        input BYPASS;
        input DYNAMICDELAY[4];
        input EXTFEEDBACK;
        input REFERENCECLK;
        input RESET;
        output LOCK;

        // Secret dynamic reconfiguration.
        input SCLK, SDI;
        output SDO;

        // Output to fabric. Connected to the relevant IOI's `IOB_DIN` when PLL enabled.
        nonroutable output PLLOUTCOREA, PLLOUTCOREB;

        // Output to global wires.
        output PLLOUTGLOBALA, PLLOUTGLOBALB;

        // Latch control signal, shared with all IOIs on the same edge.
        input LATCHINPUTVALUE;

        // Input from the relevant IOB.
        nonroutable input PACKAGEPIN;

        pad AGND: power;
        pad AVCC: power;

        attribute DIVQ: bitvec[3];
        attribute DIVR: bitvec[4];
        attribute DIVF: bitvec[6];
        // Actually works per-bit.
        attribute DELAY_ADJUSTMENT_MODE_DYNAMIC: bitvec[4];
        attribute FILTER_RANGE: bitvec[3];
        attribute FIXED_DELAY_ADJUSTMENT: bitvec[4];
        attribute FEEDBACK_PATH: FEEDBACK_PATH;
        // Used for global output from IOBs even if the PLL is disabled.
        attribute LATCH_GLOBAL_OUT_A, LATCH_GLOBAL_OUT_B: bool;
        attribute TEST_MODE: bool;
        attribute MODE: PLL65_MODE;
        attribute PLLOUT_PHASE: PLL65_PLLOUT_PHASE;
    }

    enum PLL40_DELAY_ADJUSTMENT_MODE {
        FIXED,
        DYNAMIC,
    }

    enum PLL40_MODE {
        NONE,
        PLL40_PAD,
        PLL40_CORE,
        PLL40_2_PAD,
        PLL40_2F_PAD,
        PLL40_2F_CORE,
    }

    enum PLL40_PLLOUT_SELECT {
        GENCLK,
        GENCLK_HALF,
        SHIFTREG_90DEG,
        SHIFTREG_0DEG,
    }

    bel_class PLL40 {
        input BYPASS;
        input DYNAMICDELAY[8];
        input EXTFEEDBACK;
        input REFERENCECLK;
        input RESETB;
        output LOCK;

        // Secret dynamic reconfiguration.
        input SCLK, SDI;
        output SDO;

        // Output to fabric. Connected to the relevant IOI's `IOB_DIN` when PLL enabled.
        nonroutable output PLLOUTCOREA, PLLOUTCOREB;

        // Output to global wires.
        output PLLOUTGLOBALA, PLLOUTGLOBALB;

        // Latch control signal, shared with all IOIs on the same edge.
        input LATCHINPUTVALUE;

        // Input from the relevant IOBs.
        nonroutable input PACKAGEPIN, PACKAGEPINB;

        pad AGND: power;
        pad AVCC: power;

        attribute DIVQ: bitvec[3];
        attribute DIVR: bitvec[4];
        attribute DIVF: bitvec[7];
        attribute DELAY_ADJUSTMENT_MODE_FEEDBACK: PLL40_DELAY_ADJUSTMENT_MODE;
        attribute DELAY_ADJUSTMENT_MODE_RELATIVE: PLL40_DELAY_ADJUSTMENT_MODE;
        attribute FILTER_RANGE: bitvec[3];
        attribute FDA_FEEDBACK, FDA_RELATIVE: bitvec[4];
        attribute FEEDBACK_PATH: FEEDBACK_PATH;
        // Used for global output from IOBs even if the PLL is disabled.
        attribute LATCH_GLOBAL_OUT_A, LATCH_GLOBAL_OUT_B: bool;
        attribute TEST_MODE: bool;
        attribute SHIFTREG_DIV_MODE: bool;
        attribute MODE: PLL40_MODE;
        attribute PLLOUT_SELECT_PORTA, PLLOUT_SELECT_PORTB: PLL40_PLLOUT_SELECT;
    }

    bel_class WARMBOOT {
        input BOOT, S0, S1;
    }

    bel_class SMCCLK {
        output CLK;
    }

    bel_class SPI {
        input SBADRI[8];
        input SBCLKI;
        input SBDATI[8];
        output SBACKO;
        output SBDATO[8];
        input SBRWI, SBSTBI;
        output SPIIRQ, SPIWKUP;

        // I/O pad control. All except MCSNO[2..4] have dedicated IOBs.
        input SCSNI;
        output MCSNO[4], MCSNOE[4];
        input MI, SI;
        output MO, MOE, SO, SOE;
        input SCKI;
        output SCKO, SCKOE;
    }

    bel_class I2C {
        input SBADRI[8];
        input SBCLKI;
        input SBDATI[8];
        output SBACKO;
        output SBDATO[8];
        input SBRWI, SBSTBI;
        output I2CIRQ, I2CWKUP;

        // I/O pad control.
        input SCLI, SDAI;
        output SCLO, SCLOE, SDAO, SDAOE;

        // SDA_*_DELAY stored in `IOB_PAIR` instead.
    }

    bel_class I2C_FIFO {
        input ADRI[4];
        input CLKI;
        input CSI;
        input DATI[10];
        output ACKO;
        output DATO[10];
        input WEI, STBI;
        input FIFORST;
        output RXFIFOAFULL, RXFIFOEMPTY, RXFIFOFULL;
        output TXFIFOAEMPTY, TXFIFOEMPTY, TXFIFOFULL;
        output SRWO;
        output MRDCMPL;
        output I2CIRQ, I2CWKUP;

        // I/O pad control.
        input SCLI, SDAI;
        output SCLO, SCLOE, SDAO, SDAOE;

        // SDA_*_DELAY stored in `IOB_PAIR` instead.
    }

    // Extra pullup controls for special I3C pads.  Present for two pads on iCE40T05 only.
    // Connected in a completely different part of the chip because why not.
    bel_class IOB_I3C {
        input WEAK_PU_ENB;
        input PU_ENB;
    }

    // 50ns glitch filter for I2C.
    bel_class FILTER {
        input FILTERIN;
        output FILTEROUT;

        // All 3 bits must be set when enabled.
        // TODO: what is this exactly?
        attribute ENABLE: bitvec[3];
    }

    bel_class LSOSC {
        input ENACLKK;
        output CLKK;
        // Copy of CLKK for global routing.
        output CLKK_GLOBAL;
    }

    bel_class HSOSC {
        input ENACLKM;
        output CLKM;
        // Copy of CLKM for global routing.
        output CLKM_GLOBAL;
    }

    bel_class LFOSC {
        input CLKLFEN;
        input CLKLFPU;
        output CLKLF;
        // Copy of CLKLF for global routing.
        output CLKLF_GLOBAL;
        input TRIM[10];

        attribute TRIM_FABRIC: bool;
    }

    bel_class HFOSC {
        input CLKHFEN;
        input CLKHFPU;
        output CLKHF;
        // Copy of CLKHF for global routing.
        output CLKHF_GLOBAL;
        input TRIM[10];

        attribute TRIM_FABRIC: bool;
        attribute CLKHF_DIV: bitvec[2];
    }

    // Present only on iCE40T04/T01/T05.  Despite what you might think, still lives on in T01/T05
    // even though it's been exorcised from the official primitive library.
    bel_class LED_DRV_CUR {
        // Corresponds to `CURREN` on `SB_RGBA_DRV`, `SB_IR400_DRV`, `SB_IR500_DRV`,
        // `SB_BARCODE_DRV`.
        input EN;
        input TRIM[10];

        // not present on iCE40T05 for whatever reason
        pad GND_LED: power;

        attribute TRIM_FABRIC: bool;
        // Only on iCE40T04. Must be set when the bel is used.
        attribute ENABLE: bool;
        // Only on iCE40T01. Must be set when `RGB_DRV` is used. Relation with actual
        // `RGB_DRV`'s enable is unknown.
        attribute RGB_ENABLE: bool;
        // TODO: are the two above bits in fact one and the same?
    }

    // Represents both `SB_RGB_DRV` and `SB_RGBA_DRV`.
    bel_class RGB_DRV {
        input RGB0PWM, RGB1PWM, RGB2PWM;
        input RGBLEDEN;

        // Must be set when the bel is used.
        attribute ENABLE: bool;
        attribute RGB0_CURRENT, RGB1_CURRENT, RGB2_CURRENT: bitvec[6];
        // Only on iCE40T01/T05.
        attribute CURRENT_MODE: bitvec[1];
    }

    bel_class IR_DRV {
        input IRLEDEN;
        input IRPWM;

        // Must be set when the bel is used.
        attribute ENABLE: bool;
        attribute IR_CURRENT: bitvec[10];
    }

    // Composite bel implementing `SB_IR500_DRV`, `SB_IR400_DRV`, `SB_BARCODE_DRV`.
    bel_class IR500_DRV {
        input IRLEDEN;
        input IRPWM;
        input BARCODEEN;
        input BARCODEPWM;

        // Set when in `SB_IR500_DRV` mode (tied control signals).
        attribute IR500_ENABLE: bool;
        // Set when the IR400 part is used.
        attribute IR400_ENABLE: bool;
        // Set when the BARCODE part is used.
        attribute BARCODE_ENABLE: bool;
        attribute CURRENT_MODE: bitvec[1];
        attribute BARCODE_CURRENT: bitvec[4];
        attribute IR400_CURRENT: bitvec[8];
    }

    // Represents both `SB_LEDD_IP` and `SB_LEDDA_IP`.
    // TODO: `SB_LEDD_IP` is presumably unusably buggy.
    bel_class LEDD_IP {
        input LEDDADDR[4];
        input LEDDCLK;
        input LEDDCS;
        input LEDDDAT[8];
        input LEDDDEN;
        input LEDDEXE;
        output LEDDON;
        output PWMOUT0, PWMOUT1, PWMOUT2;
    }

    bel_class IR_IP {
        input ADRI[4];
        input CLKI;
        input CSI;
        input DENI;
        input EXE;
        input LEARN;
        input WDATA[8];
        input WEI;
        output RDATA[8];
        output DRDY, ERR, BUSY;
        input IRIN;
        output IROUT;
    }

    // Connected across the whole device.
    region_slot GLOBAL;

    // Group of cells sharing the column buffer leaf.  For devices without column buffers, same as
    // `GLOBAL`.
    region_slot COLBUF;

    // Connected across all IOIs on the same edge of the device.
    region_slot EDGE;

    wire TIE_0: tie 0;
    wire TIE_1: tie 1;

    // The global wire roots, driven by `GB_ROOT`, and used by `COLBUF`.  Unused for devices without
    // column buffers (`GLOBAL` is used directly instead).
    wire GLOBAL_ROOT[8]: regional GLOBAL;

    // The global wires, as seen by most tiles.  Driven by the column buffers, or directly by
    // `GB_ROOT` if column buffers are not present.
    wire GLOBAL[8]: regional COLBUF;

    // Helper wires used to route `GLOBAL` wires towards `LOCAL` wires.
    wire GLOBAL_OUT[4]: mux;

    // Length-4 interconnect.
    wire QUAD_H0[12]: multi_root;
    for i in 1..=4 {
        wire "QUAD_H{i}"[12]: multi_branch W;
    }
    wire QUAD_V0[12]: multi_root;
    for i in 1..=4 {
        wire "QUAD_V{i}"[12]: multi_branch S;
        wire "QUAD_V{i}_W"[12]: multi_branch E;
    }

    // Length-12 interconnect.
    wire LONG_H0[2]: multi_root;
    for i in 1..=12 {
        wire "LONG_H{i}"[2]: multi_branch W;
    }
    wire LONG_V0[2]: multi_root;
    for i in 1..=12 {
        wire "LONG_V{i}"[2]: multi_branch S;
    }

    // Local interconnect.  All signals going to `IMUX_*` must go through these wires, except
    // for direct `GLOBAL` → `IMUX_CLK`/`IMUX_CE`/`IMUX_RST` paths.
    for i in 0..4 {
        wire "LOCAL_{i}"[8]: mux;
    }

    // General interconnect inputs to LUTs, BRAMs, and iCE40T04/T01/T05 hard IP.
    wire IMUX_LC_I0[8]: mux;
    wire IMUX_LC_I1[8]: mux;
    wire IMUX_LC_I2[8]: mux;
    wire IMUX_LC_I3[8]: mux;

    // Control inputs to LUTs, BRAMs, and iCE40T04/T01/T05 hard IP.
    // `IMUX_CE` is also used for IOIs.
    wire IMUX_CLK: mux;
    wire IMUX_CLK_OPTINV: mux;
    wire IMUX_RST: mux;
    wire IMUX_CE: mux;

    // General interconnect inputs to IOIs.
    wire IMUX_IO_DOUT0[2]: mux;
    wire IMUX_IO_DOUT1[2]: mux;
    wire IMUX_IO_OE[2]: mux;

    // Control inputs to IOIs.  `IMUX_CE` is also used.
    wire IMUX_IO_ICLK: mux;
    wire IMUX_IO_ICLK_OPTINV: mux;
    wire IMUX_IO_OCLK: mux;
    wire IMUX_IO_OCLK_OPTINV: mux;

    // General interconnect input for misc stuff.  Located in IOI tiles.
    wire IMUX_IO_EXTRA: mux;

    // Bel outputs.  For IOI tiles, `OUT_LC[4..8]` are the same as `OUT_LC[0..4]`.  For special
    // PLL outputs in corner cells, all 8 `OUT_LC` wires are the same.
    wire OUT_LC[8]: bel;
    wire OUT_LC_N[8]: branch S;
    wire OUT_LC_S[8]: branch N;
    wire OUT_LC_E[8]: branch W;
    wire OUT_LC_EN[8]: branch S;
    wire OUT_LC_ES[8]: branch N;
    wire OUT_LC_W[8]: branch E;
    wire OUT_LC_WN[8]: branch S;
    wire OUT_LC_WS[8]: branch N;

    // Direct connection for iCE40T04/T01/T05 hard IP output through LC.
    wire LC_LTIN[8]: bel;

    // Direct connection for LC `CI` → `I3` routing.
    wire LC_CI_OUT[8]: bel;

    // Latch input to all IOIs and PLLs on an edge.
    wire IO_LATCH: regional EDGE;

    // Output from IOBs and PLLs to global wires.
    wire IO_GLOBAL: bel;

    // Outputs from *OSC to global wires.
    wire HSOSC_GLOBAL: regional GLOBAL;
    wire LSOSC_GLOBAL: regional GLOBAL;

    // Direct connection for iCE40R04 hard IP connection through IOI.
    wire IOB_DIN[2]: bel;
    wire IOB_DOUT[2]: bel;

    bitrect PLB = horizontal (16, 54);
    bitrect BRAM = horizontal (16, 42);
    bitrect IOI_WE = horizontal (16, 18);
    bitrect CLK = horizontal (16, 2);
    bitrect BRAM_DATA = horizontal (256, 16);

    // Main interconnect, LCs, IOIs.
    tile_slot MAIN {
        // The main interconnect switchbox.
        bel_slot INT: routing;

        bel_slot LC[8]: LC;
        tile_class PLB_L04, PLB_L08, PLB_P01 {
            cell CELL;
            bitrect MAIN: PLB;

            switchbox INT {
                // filled by harvester
            }
            for i in 0..8 {
                bel LC[i] {
                    input CLK = IMUX_CLK_OPTINV;
                    input RST = IMUX_RST;
                    input CE = IMUX_CE;
                    input I0 = IMUX_LC_I0[i];
                    input I1 = IMUX_LC_I1[i];
                    input I2 = IMUX_LC_I2[i];
                    input I3 = IMUX_LC_I3[i];
                    if tile_class PLB_P01 {
                        input LTIN = LC_LTIN[i];
                    }
                    output CI_OUT = LC_CI_OUT[i];
                    output O = OUT_LC[i];
                }
            }
        }

        // Two `INT_BRAM` tiles for every `BRAM` tile.
        tile_class INT_BRAM {
            cell CELL;
            bitrect MAIN: BRAM;

            switchbox INT {
                // filled by harvester
            }
        }

        bel_slot IOI[2]: IOI;
        tile_class IOI_W_L04, IOI_E_L04, IOI_W_L08, IOI_E_L08, IOI_S_L04, IOI_N_L04, IOI_S_L08, IOI_N_L08, IOI_S_T04, IOI_N_T04 {
            cell CELL;
            if tile_class [IOI_W_L04, IOI_E_L04, IOI_W_L08, IOI_E_L08] {
                bitrect MAIN: IOI_WE;
            } else {
                bitrect MAIN: BRAM;
            }

            switchbox INT {
                // filled by harvester
            }
            for i in 0..2 {
                bel IOI[i] {
                    input CE = IMUX_CE;
                    input ICLK = IMUX_IO_ICLK_OPTINV;
                    input OCLK = IMUX_IO_OCLK_OPTINV;
                    input DOUT0 = IMUX_IO_DOUT0[i];
                    input DOUT1 = IMUX_IO_DOUT1[i];
                    input OE = IMUX_IO_OE[i];
                    if bel_slot IOI[0] {
                        output DIN0 = OUT_LC[0], OUT_LC[4];
                        output DIN1 = OUT_LC[1], OUT_LC[5];
                    } else {
                        output DIN0 = OUT_LC[2], OUT_LC[6];
                        output DIN1 = OUT_LC[3], OUT_LC[7];
                    }
                    input LATCH = IO_LATCH;
                    input IOB_DIN = IOB_DIN[i];
                    output IOB_DOUT = IOB_DOUT[i];
                }
            }
        }
    }

    // Global wire column buffers.
    tile_slot COLBUF {
        bel_slot COLBUF: routing;

        tile_class COLBUF_L01, COLBUF_P08, COLBUF_IO_W, COLBUF_IO_E {
            cell CELL;
            if tile_class [COLBUF_IO_W, COLBUF_IO_E] {
                bitrect MAIN: IOI_WE;
            } else {
                bitrect MAIN: BRAM;
            }

            switchbox COLBUF {
                for i in 0..8 {
                    progbuf GLOBAL[i] = GLOBAL_ROOT[i];
                }
            }
        }

        tile_class COLBUF_FIXED {
            cell CELL;

            switchbox COLBUF {
                for i in 0..8 {
                    permabuf GLOBAL[i] = GLOBAL_ROOT[i];
                }
            }
        }
    }

    // Global wire root muxes.
    tile_slot GB_ROOT {
        bel_slot GB_ROOT: routing;
        tile_class GB_ROOT_L04, GB_ROOT_L08, GB_ROOT_R04 {
            cell SE, NE, EN, WN, NW, SW, WS, ES;
            bitrect CLK[2]: CLK;

            switchbox GB_ROOT {
                mux SE.GLOBAL_ROOT[0] = SE.IMUX_IO_EXTRA | ES.IO_GLOBAL;
                mux SE.GLOBAL_ROOT[1] = NE.IMUX_IO_EXTRA | WS.IO_GLOBAL;
                mux SE.GLOBAL_ROOT[2] = EN.IMUX_IO_EXTRA | NE.IO_GLOBAL;
                mux SE.GLOBAL_ROOT[3] = WN.IMUX_IO_EXTRA | SE.IO_GLOBAL;
                if tile_class GB_ROOT_R04 {
                    mux SE.GLOBAL_ROOT[4] = NW.IMUX_IO_EXTRA | SE.HSOSC_GLOBAL;
                    mux SE.GLOBAL_ROOT[5] = SW.IMUX_IO_EXTRA | SE.LSOSC_GLOBAL;
                } else {
                    mux SE.GLOBAL_ROOT[4] = NW.IMUX_IO_EXTRA | WN.IO_GLOBAL;
                    mux SE.GLOBAL_ROOT[5] = SW.IMUX_IO_EXTRA | EN.IO_GLOBAL;
                }
                mux SE.GLOBAL_ROOT[6] = WS.IMUX_IO_EXTRA | SW.IO_GLOBAL;
                mux SE.GLOBAL_ROOT[7] = ES.IMUX_IO_EXTRA | NW.IO_GLOBAL;
            }
        }
    }

    // Used for most bels.
    tile_slot BEL {
        bel_slot IO_LATCH: routing;
        tile_class IO_LATCH {
            cell CELL;
            switchbox IO_LATCH {
                permabuf CELL.IO_LATCH = CELL.IMUX_IO_EXTRA;
            }
        }

        bel_slot BRAM: BRAM;
        tile_class BRAM_L04, BRAM_P01, BRAM_P08 {
            cell CELL[2];
            bitrect MAIN[2]: BRAM;
            bitrect DATA: BRAM_DATA;

            bel BRAM {
                if tile_class [BRAM_L04, BRAM_P01] {
                    input WCLK = CELL[0].IMUX_CLK_OPTINV;
                    input WCLKE = CELL[0].IMUX_CE;
                    input WE = CELL[0].IMUX_RST;
                    for i in 0..8 {
                        input WADDR[i] = CELL[0].IMUX_LC_I0[i];
                    }
                    if tile_class BRAM_P01 {
                        for i in 0..3 {
                            input WADDR[i + 8] = CELL[0].IMUX_LC_I2[i];
                        }
                    }
                    for i in 0..8 {
                        input WDATA[i] = CELL[0].IMUX_LC_I1[i];
                        input WDATA[i + 8] = CELL[1].IMUX_LC_I1[i];
                        input MASK[i] = CELL[0].IMUX_LC_I3[i];
                        input MASK[i + 8] = CELL[1].IMUX_LC_I3[i];
                    }

                    input RCLK = CELL[1].IMUX_CLK_OPTINV;
                    input RCLKE = CELL[1].IMUX_CE;
                    input RE = CELL[1].IMUX_RST;
                    for i in 0..8 {
                        input RADDR[i] = CELL[1].IMUX_LC_I0[i];
                    }
                    if tile_class BRAM_P01 {
                        for i in 0..3 {
                            input RADDR[i + 8] = CELL[1].IMUX_LC_I2[i];
                        }
                    }
                    for i in 0..8 {
                        output RDATA[i] = CELL[0].OUT_LC[i];
                        output RDATA[i + 8] = CELL[1].OUT_LC[i];
                    }
                } else {
                    input WCLK = CELL[1].IMUX_CLK_OPTINV;
                    input WCLKE = CELL[1].IMUX_CE;
                    input WE = CELL[1].IMUX_RST;
                    input WADDR[0] = CELL[1].IMUX_LC_I0[7];
                    input WADDR[1] = CELL[1].IMUX_LC_I0[6];
                    input WADDR[2] = CELL[1].IMUX_LC_I0[5];
                    input WADDR[3] = CELL[1].IMUX_LC_I0[4];
                    input WADDR[4] = CELL[1].IMUX_LC_I0[3];
                    input WADDR[5] = CELL[1].IMUX_LC_I0[2];
                    input WADDR[6] = CELL[1].IMUX_LC_I0[1];
                    input WADDR[7] = CELL[1].IMUX_LC_I0[0];
                    input WADDR[8] = CELL[1].IMUX_LC_I2[7];
                    input WADDR[9] = CELL[1].IMUX_LC_I2[6];
                    input WADDR[10] = CELL[1].IMUX_LC_I2[5];
                    input WDATA[0] = CELL[1].IMUX_LC_I1[7];
                    input WDATA[1] = CELL[1].IMUX_LC_I1[6];
                    input WDATA[2] = CELL[1].IMUX_LC_I1[5];
                    input WDATA[3] = CELL[1].IMUX_LC_I1[4];
                    input WDATA[4] = CELL[1].IMUX_LC_I1[3];
                    input WDATA[5] = CELL[1].IMUX_LC_I1[2];
                    input WDATA[6] = CELL[1].IMUX_LC_I1[1];
                    input WDATA[7] = CELL[1].IMUX_LC_I1[0];
                    input WDATA[8] = CELL[0].IMUX_LC_I1[7];
                    input WDATA[9] = CELL[0].IMUX_LC_I1[6];
                    input WDATA[10] = CELL[0].IMUX_LC_I1[5];
                    input WDATA[11] = CELL[0].IMUX_LC_I1[4];
                    input WDATA[12] = CELL[0].IMUX_LC_I1[3];
                    input WDATA[13] = CELL[0].IMUX_LC_I1[2];
                    input WDATA[14] = CELL[0].IMUX_LC_I1[1];
                    input WDATA[15] = CELL[0].IMUX_LC_I1[0];
                    input MASK[0] = CELL[1].IMUX_LC_I3[7];
                    input MASK[1] = CELL[1].IMUX_LC_I3[6];
                    input MASK[2] = CELL[1].IMUX_LC_I3[5];
                    input MASK[3] = CELL[1].IMUX_LC_I3[4];
                    input MASK[4] = CELL[1].IMUX_LC_I3[3];
                    input MASK[5] = CELL[1].IMUX_LC_I3[2];
                    input MASK[6] = CELL[1].IMUX_LC_I3[1];
                    input MASK[7] = CELL[1].IMUX_LC_I3[0];
                    input MASK[8] = CELL[0].IMUX_LC_I3[7];
                    input MASK[9] = CELL[0].IMUX_LC_I3[6];
                    input MASK[10] = CELL[0].IMUX_LC_I3[5];
                    input MASK[11] = CELL[0].IMUX_LC_I3[4];
                    input MASK[12] = CELL[0].IMUX_LC_I3[3];
                    input MASK[13] = CELL[0].IMUX_LC_I3[2];
                    input MASK[14] = CELL[0].IMUX_LC_I3[1];
                    input MASK[15] = CELL[0].IMUX_LC_I3[0];

                    input RCLK = CELL[0].IMUX_CLK_OPTINV;
                    input RCLKE = CELL[0].IMUX_CE;
                    input RE = CELL[0].IMUX_RST;
                    input RADDR[0] = CELL[0].IMUX_LC_I0[7];
                    input RADDR[1] = CELL[0].IMUX_LC_I0[6];
                    input RADDR[2] = CELL[0].IMUX_LC_I0[5];
                    input RADDR[3] = CELL[0].IMUX_LC_I0[4];
                    input RADDR[4] = CELL[0].IMUX_LC_I0[3];
                    input RADDR[5] = CELL[0].IMUX_LC_I0[2];
                    input RADDR[6] = CELL[0].IMUX_LC_I0[1];
                    input RADDR[7] = CELL[0].IMUX_LC_I0[0];
                    input RADDR[8] = CELL[0].IMUX_LC_I2[7];
                    input RADDR[9] = CELL[0].IMUX_LC_I2[6];
                    input RADDR[10] = CELL[0].IMUX_LC_I2[5];
                    output RDATA[0] = CELL[1].OUT_LC[7];
                    output RDATA[1] = CELL[1].OUT_LC[6];
                    output RDATA[2] = CELL[1].OUT_LC[5];
                    output RDATA[3] = CELL[1].OUT_LC[4];
                    output RDATA[4] = CELL[1].OUT_LC[3];
                    output RDATA[5] = CELL[1].OUT_LC[2];
                    output RDATA[6] = CELL[1].OUT_LC[1];
                    output RDATA[7] = CELL[1].OUT_LC[0];
                    output RDATA[8] = CELL[0].OUT_LC[7];
                    output RDATA[9] = CELL[0].OUT_LC[6];
                    output RDATA[10] = CELL[0].OUT_LC[5];
                    output RDATA[11] = CELL[0].OUT_LC[4];
                    output RDATA[12] = CELL[0].OUT_LC[3];
                    output RDATA[13] = CELL[0].OUT_LC[2];
                    output RDATA[14] = CELL[0].OUT_LC[1];
                    output RDATA[15] = CELL[0].OUT_LC[0];
                }
            }
        }

        bel_slot MAC16: MAC16;
        tile_class MAC16, MAC16_TRIM {
            cell CELL[5];
            bitrect MAIN[5]: PLB;
            bel MAC16 {
                // filled by harvester
            }
        }

        bel_slot SPRAM[2]: SPRAM;
        tile_class SPRAM {
            cell CELL[4];
            bitrect MAIN[4]: PLB;
            for i in 0..2 {
                bel SPRAM[i] {
                    // filled by harvester
                }
            }
        }

        bel_slot PLL65: PLL65;
        tile_class PLL65 {
            cell CELL[13];
            cell CORNER_W;
            cell CORNER_E;
            bitrect CLK[2]: CLK;

            bel PLL65 {
                output PLLOUTGLOBALA = CELL[6].IO_GLOBAL;
                output PLLOUTGLOBALB = CELL[7].IO_GLOBAL;
                input LATCHINPUTVALUE = CELL[6].IO_LATCH;

                // rest filled by harvester
            }
        }

        bel_slot PLL40: PLL40;
        tile_class PLL40_S_P01 {
            cell CELL[7];
            cell CELL_SIDE[14];
            cell CORNER_W;
            cell CORNER_E;
            bitrect MAIN[7]: PLB;
            bitrect MAIN_SIDE[14]: IOI_WE;

            bel PLL40 {
                output PLLOUTGLOBALA = CELL[5].IO_GLOBAL;
                output PLLOUTGLOBALB = CELL[6].IO_GLOBAL;
                input LATCHINPUTVALUE = CELL[5].IO_LATCH;

                // rest filled by harvester
            }
        }
        tile_class PLL40_S_P08, PLL40_N_P08, PLL40_S_R04, PLL40_N_R04, PLL40_S_STUB {
            cell CELL[18];
            cell CORNER_W;
            cell CORNER_E;
            bitrect MAIN[18]: PLB;

            bel PLL40 {
                output PLLOUTGLOBALA = CELL[11].IO_GLOBAL;
                output PLLOUTGLOBALB = CELL[12].IO_GLOBAL;
                input LATCHINPUTVALUE = CELL[11].IO_LATCH;

                // rest filled by harvester
            }
        }
        tile_class PLL40_S_T01 {
            cell CELL[9];
            cell CELL_SIDE;
            cell CORNER_W;
            cell CORNER_E;
            bitrect MAIN[9]: PLB;
            bitrect MAIN_SIDE: PLB;

            bel PLL40 {
                output PLLOUTGLOBALA = CELL[5].IO_GLOBAL;
                output PLLOUTGLOBALB = CELL[6].IO_GLOBAL;
                input LATCHINPUTVALUE = CELL[5].IO_LATCH;

                // rest filled by harvester
            }
        }

        bel_slot SPI: SPI;
        tile_class SPI_R04 {
            cell CELL[10];
            cell CELL_IO[5];
            bel SPI {
                // filled by harvester
            }
        }
        tile_class SPI_T04, SPI_T05 {
            cell CELL[4];
            bel SPI {
                // filled by harvester
            }
        }

        bel_slot I2C: I2C;
        tile_class I2C_R04 {
            cell CELL[10];
            cell CELL_IO[2];
            bel I2C {
                // filled by harvester
            }
        }
        tile_class I2C_T04 {
            cell CELL[2];
            bel I2C {
                // filled by harvester
            }
        }

        bel_slot I2C_FIFO: I2C_FIFO;
        tile_class I2C_FIFO {
            cell CELL[4];
            bel I2C_FIFO {
                // filled by harvester
            }
        }

        bel_slot LSOSC: LSOSC;
        tile_class LSOSC {
            cell CELL_OUT;
            cell CELL_EN;

            bel LSOSC {
                output CLKK_GLOBAL = CELL_OUT.LSOSC_GLOBAL;
                // rest filled by harvester
            }
        }

        bel_slot HSOSC: HSOSC;
        tile_class HSOSC {
            cell CELL_OUT;
            cell CELL_EN;

            bel HSOSC {
                output CLKM_GLOBAL = CELL_OUT.HSOSC_GLOBAL;
                // rest filled by harvester
            }
        }

        bel_slot WARMBOOT: WARMBOOT;
        tile_class WARMBOOT {
            cell CELL[3];
            bel WARMBOOT {
                input BOOT = CELL[0].IMUX_IO_EXTRA;
                input S0 = CELL[1].IMUX_IO_EXTRA;
                input S1 = CELL[2].IMUX_IO_EXTRA;
            }
        }

        bel_slot HFOSC: HFOSC;
        bel_slot LFOSC: LFOSC;
        bel_slot LED_DRV_CUR: LED_DRV_CUR;
        bel_slot RGB_DRV: RGB_DRV;
        bel_slot IR_DRV: IR_DRV;
        bel_slot IR500_DRV: IR500_DRV;
        bel_slot LEDD_IP: LEDD_IP;
        bel_slot IR_IP: IR_IP;
        bel_slot IOB_I3C[2]: IOB_I3C;
        bel_slot FILTER[2]: FILTER;
        bel_slot SMCCLK: SMCCLK;

        tile_class MISC_T04 {
            cell CELL_W[3];
            cell CELL_E[4];
            cell CELL_TRIM;
            cell CELL_SMCCLK;
            bitrect MAIN_W[3]: PLB;
            bitrect MAIN_E[4]: PLB;
            bitrect MAIN_TRIM: PLB;
            bitrect MAIN_SMCCLK: PLB;

            bel LFOSC {
                output CLKLF_GLOBAL = CELL_W[0].LSOSC_GLOBAL;
            }

            bel HFOSC {
                output CLKHF_GLOBAL = CELL_W[0].HSOSC_GLOBAL;
            }

            bel LED_DRV_CUR;
            bel RGB_DRV;
            bel IR_DRV;
            bel LEDD_IP;

            bel SMCCLK {
                output CLK = CELL_SMCCLK.LC_LTIN[5];
            }
        }

        tile_class MISC_T01 {
            cell CELL_W[6];
            cell CELL_E[6];
            bitrect MAIN_W[6]: PLB;
            bitrect MAIN_E[6]: PLB;

            bel LFOSC {
                output CLKLF_GLOBAL = CELL_W[0].LSOSC_GLOBAL;
            }

            bel HFOSC {
                output CLKHF_GLOBAL = CELL_W[0].HSOSC_GLOBAL;
            }

            bel LED_DRV_CUR;
            bel RGB_DRV;
            bel IR500_DRV;
            bel LEDD_IP;
            bel IR_IP;

            bel WARMBOOT;

            bel SMCCLK {
                output CLK = CELL_E[3].LC_LTIN[2];
            }
        }

        tile_class MISC_T05 {
            cell CELL_W[3];
            cell CELL_E[4];
            cell CELL_TRIM;
            cell CELL_SMCCLK;
            bitrect MAIN_W[3]: PLB;
            bitrect MAIN_E[4]: PLB;
            bitrect MAIN_TRIM: PLB;
            bitrect MAIN_SMCCLK: PLB;

            bel LFOSC {
                output CLKLF_GLOBAL = CELL_W[0].LSOSC_GLOBAL;
            }

            bel HFOSC {
                output CLKHF_GLOBAL = CELL_W[0].HSOSC_GLOBAL;
            }

            bel LED_DRV_CUR;
            bel RGB_DRV;
            bel LEDD_IP;

            bel FILTER[0];
            bel FILTER[1];

            bel IOB_I3C[0];
            bel IOB_I3C[1];

            bel SMCCLK {
                output CLK = CELL_SMCCLK.LC_LTIN[1];
            }
        }
    }

    // The I/O buffers.
    tile_slot IOB {
        bel_slot IOB[2]: IOB;
        bel_slot IOB_PAIR: IOB_PAIR;
        tile_class
            IOB_W_L04, IOB_E_L04, IOB_S_L04, IOB_N_L04,
            IOB_W_P04, IOB_E_P04, IOB_S_P04, IOB_N_P04,
            IOB_W_L08, IOB_E_L08, IOB_S_L08, IOB_N_L08,
            IOB_W_L01, IOB_E_L01, IOB_S_L01, IOB_N_L01,
            IOB_W_P01, IOB_E_P01, IOB_S_P01, IOB_N_P01,
            IOB_W_P08, IOB_E_P08, IOB_S_P08, IOB_N_P08,
            IOB_W_P03, IOB_E_P03, IOB_S_P03, IOB_N_P03,
            IOB_S_R04, IOB_N_R04,
            IOB_S_T04, IOB_N_T04,
            IOB_S_T05, IOB_N_T05,
            IOB_S_T01, IOB_N_T01 {
            cell CELL;
            if tile_class ["IOB_W_*", "IOB_E_*"] {
                bitrect MAIN: IOI_WE;
            } else {
                bitrect MAIN: BRAM;
            }

            for i in 0..2 {
                bel IOB[i];
            }
            bel IOB_PAIR {
                output GLOBAL_OUT = IO_GLOBAL;
                input LATCH = IO_LATCH;
            }
        }
    }

    enum CONFIG_SPEED {
        LOW,
        MEDIUM,
        HIGH,
    }

    enum CONFIG_CRC_MODE {
        WRITE,
        READ,
    }

    bel_class GLOBAL_OPTIONS {
        attribute SPEED: CONFIG_SPEED;
        attribute FLASH_POWERDOWN: bool;
        attribute KEEP_SMCCLK: bool;
        attribute DAISY_CHAIN_ENABLE: bool;
        attribute CRC_MODE: CONFIG_CRC_MODE;
        attribute COLDBOOT_ENABLE: bool;
        attribute WARMBOOT_ENABLE: bool;
        attribute READ_PROTECT: bool;
        attribute WRITE_PROTECT: bool;
        attribute PARALLEL_ENABLE: bool;
        attribute WARMBOOT_NVCM_MASK: bitvec[4];
    }

    bel_class POWER {
        pad GND: power;
        pad VCCINT: power;
    }

    bel_class IO_BANK {
        pad VCCIO: power;
        pad VREF: analog;
    }

    bel_class CONFIG {
        pad CRESET_B: input;
        pad CDONE: inout;
        pad TRST_B: input;
        pad TCK: input;
        pad TMS: input;
        pad TDI: input;
        pad TDO: output;
        // ???
        pad POR_TEST: inout;
        pad VPP_2V5: power;
        pad VPP_FAST: power;
    }

    bitrect CONFIG_CREG = horizontal (1, rev 16);
    bitrect CONFIG_SPEED = horizontal (1, rev 2);

    tile_slot GLOBALS {
        bel_slot GLOBAL_OPTIONS: GLOBAL_OPTIONS;
        bel_slot POWER: POWER;
        bel_slot CONFIG: CONFIG;
        bel_slot IO_BANK[4]: IO_BANK;
        bel_slot IO_BANK_SPI: IO_BANK;
        tile_class GLOBALS {
            bitrect CREG: CONFIG_CREG;
            bitrect SPEED: CONFIG_SPEED;

            bel GLOBAL_OPTIONS {
                attribute SPEED @[
                    SPEED[1],
                    SPEED[0],
                ] {
                    LOW = 0b00,
                    MEDIUM = 0b01,
                    HIGH = 0b10,
                }
                attribute FLASH_POWERDOWN @!CREG[0];
                attribute KEEP_SMCCLK @CREG[1];
                attribute DAISY_CHAIN_ENABLE @CREG[2];
                attribute CRC_MODE @CREG[3] {
                    WRITE = 0b0,
                    READ = 0b1,
                }
                attribute COLDBOOT_ENABLE @CREG[4];
                attribute WARMBOOT_ENABLE @CREG[5];
                attribute READ_PROTECT @CREG[6];
                attribute WRITE_PROTECT @CREG[7];
                attribute PARALLEL_ENABLE @CREG[8];
                attribute WARMBOOT_NVCM_MASK @[
                    CREG[12],
                    CREG[11],
                    CREG[10],
                    CREG[9],
                ];
            }

            bel POWER;
            bel CONFIG;
            for i in 0..4 {
                bel IO_BANK[i];
            }
            bel IO_BANK_SPI;
        }
    }

    connector_slot W {
        opposite E;
        connector_class PASS_W {
            for i in 0..4 {
                pass "QUAD_H{i + 1}" = "QUAD_H{i}";
            }
            for i in 0..12 {
                pass "LONG_H{i + 1}" = "LONG_H{i}";
            }
            pass OUT_LC_E = OUT_LC;
        }
    }

    connector_slot E {
        opposite W;
        connector_class PASS_E {
            for i in 1..=4 {
                pass "QUAD_V{i}_W" = "QUAD_V{i}";
            }
            pass OUT_LC_W = OUT_LC;
        }
    }

    connector_slot S {
        opposite N;
        connector_class PASS_S {
            for i in 0..4 {
                pass "QUAD_V{i + 1}" = "QUAD_V{i}";
            }
            for i in 0..12 {
                pass "LONG_V{i + 1}" = "LONG_V{i}";
            }
            pass OUT_LC_N = OUT_LC;
            pass OUT_LC_WN = OUT_LC_W;
            pass OUT_LC_EN = OUT_LC_E;
        }
    }

    connector_slot N {
        opposite S;
        connector_class PASS_N {
            pass OUT_LC_S = OUT_LC;
            pass OUT_LC_WS = OUT_LC_W;
            pass OUT_LC_ES = OUT_LC_E;
        }
    }

    table IOSTD {
        field DRIVE: bitvec[2];
        field IOSTD_MISC: bitvec[1];

        row
            SB_LVCMOS15_2,
            SB_LVCMOS15_4,
            SB_LVCMOS18_2,
            SB_LVCMOS18_4,
            SB_LVCMOS18_8,
            SB_LVCMOS18_10,
            SB_LVCMOS25_4,
            SB_LVCMOS25_8,
            SB_LVCMOS25_12,
            SB_LVCMOS25_16,
            SB_LVCMOS33_8,
            SB_MDDR2,
            SB_MDDR4,
            SB_MDDR8,
            SB_MDDR10,
            SB_SSTL18_FULL,
            SB_SSTL18_HALF,
            SB_SSTL2_CLASS_1,
            SB_SSTL2_CLASS_2,
            SB_LVDS_INPUT,
            SB_SUBLVDS_INPUT
        ;
    }
}

pub const QUAD_H: &[EntityStaticRange<WireSlotId, 12>; 5] = &[
    wires::QUAD_H0,
    wires::QUAD_H1,
    wires::QUAD_H2,
    wires::QUAD_H3,
    wires::QUAD_H4,
];

pub const QUAD_V: &[EntityStaticRange<WireSlotId, 12>; 5] = &[
    wires::QUAD_V0,
    wires::QUAD_V1,
    wires::QUAD_V2,
    wires::QUAD_V3,
    wires::QUAD_V4,
];
