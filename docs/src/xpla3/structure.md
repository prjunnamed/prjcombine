# Device structure

## Overview

An XPLA3 device is made of:

- the ZIA (Zero-power Interconnect Array), which routes various signals to FB inputs; the routable signals include:
  - MC outputs
  - IOB outputs (ie. input buffers from general purpose I/O)
  - GCLK input buffers
  - a special POR (power-on reset) signal that is pulsed at device startup
- 2-32 FBs (function blocks), each of which has:
  - two `FCLK[i]` fast clock networks, routable from `GCLK` pads
  - 40 routable inputs from ZIA
  - 48 PTs (product terms) shared between all MCs, each of them having one special function that can be used
    instead of (or in addition to) being included in MC sum terms:
    - PT0-7: directly (with programmable inversion) drive LCTs (local control terms), which can be routed to control
      inputs of all MCs within this FB; the possible uses of each LCT include:
      - LCT0-2: RST/SET, OE
      - LCT3: RST/SET
      - LCT4: CLK, RST/SET, CE
      - LCT5: CLK, RST/SET
      - LCT6: CLK, OE, UCTs (on XCR3032XL only)
      - LCT7: CLK, UCTs
    - PT8-38 (even): fast data input to MC0-15, respectively
    - PT9-39 (off): dedicated CLK or CE input to MC0-15, respectively
    - PT40-47: foldback NANDs, the PT outputs are inverted and fed back as possible inputs to all PTs in this FB
  - 16 MCs (macrocells), each of which has:
    - a sum term, including an arbitrary subset of this FB's PTs
    - a LUT2 implementing an arbitrary function of the sum term and the fast data input PT
    - a register, with:
      - D input tied to either the LUT2 output, or this MC's input buffer (sidestepping the ZIA)
      - configurable mode, one of: DFF, TFF, D latch, DFF with clock enable
      - clock (or latch gate), freely invertible and routable from LCT4-7, FCLK0-1, dedicated per-MC PT, or UCT0
      - async set and reset, both routable from LCT0-5, UCT2 (set), UCT3 (reset), or const 0
      - clock enable, routable from LCT4 or dedicated per-MC PT
      - always 0 initial value
    - output to ZIA, routed either from the LUT2 (combinatorial) or the register's Q output (registered)
    - IOB (input/output buffer) (on larger devices, not all MCs have an IOB), with:
      - input buffer (routed to ZIA and this MC's register D input)
      - output to ZIA, routed either from the input buffer or from the MC's register Q output
      - either combinatorial (from LUT2) or registered (from register's Q) output to the output buffer, selectable
        independently from ZIA output
      - output enable, routable from LCT0-2, LCT6, UCT1, const 0, const 1
      - optional pull-up (can only be used when OE is tied to 0)
      - configurable slew rate (fast or slow)
- global signals
  - the 4 GCLK dedicated inputs, routable to ZIA and per-FB `FCLK` networks
  - the 4 UCTs (Universal Control Terms), routable from LCT7 of all FBs (and, on XCR3032XL, also LCT6 of all FBs);
    UCTs can be used to drive control signals throughout the whole device:
    - UCT0: CLK
    - UCT1: OE
    - UCT2: RST
    - UCT3: SET
- special global configuration bits
  - JTAG pins disable bit (if not programmed, the IOBs corresponding to JTAG pins are connected to the TAP instead of their MCs)
  - user electronic signature bits (free-form field, size varies with device)
  - read protection enable

## FB columns and rows

The FBs in the device are organized in a 2D structure of columns and rows.

Each FB column has its own branch of the ZIA, and actually contains two FBs per row:
one on each side of the ZIA.  The FBs are nubered as follows:

- Even-numbered FBs are on one side of the ZIA branch of their column, and odd-numbered are on
  the other
- When increasing FB number from odd to even, move to the next FB row
- If already at the last FB row, move to the next FB column and first FB row

For example, the xcr3384xl device is organized as follows:

```
+-----+-----------+-----------+-----------+
| Row | Column 0  | Column 1  | Column 2  |
+=====+=====+=====+=====+=====+=====+=====+
|  0  |  0  |  1  |  8  |  9  |  16 |  17 |
+-----+-----+-----+-----+-----+-----+-----+
|  1  |  2  |  3  |  10 |  11 |  18 |  19 |
+-----+-----+-----+-----+-----+-----+-----+
|  2  |  4  |  5  |  12 |  13 |  20 |  21 |
+-----+-----+-----+-----+-----+-----+-----+
|  3  |  6  |  7  |  14 |  15 |  22 |  23 |
+-----+-----+-----+-----+-----+-----+-----+
```

## ZIA and FB inputs

The core interconnect structure in XPLA3 devices is called ZIA, the Zero-power Interconnect Array.
It is a classic CPLD design.

Each FB has 40 inputs from ZIA, which we call `FB[i].IM[j]`.  Each FB input is controlled
by a set of fuses:

- mux fuses (`FB[i].IM[j].MUX`) select what is routed to the input.  The combinations include:

  - `VCC`: the input is const 1 (for unused inputs)
  - `GND`: the input is const 0 (not very useful)
  - `MC_{k}_{l}`: the input is routed from the macrocell output `FB[k].MC[l].MC_ZIA_OUT`
  - `IOB_{k}_{l}`: the input is routed from the macrocell output `FB[k].MC[l].IOB_ZIA_OUT`
  - `GCLK{i}`: the input is routed from the input buffer of dedicated `GCLK{i}` pin;
    for this choice to work, the relevant `FB_COL[*].ZIA_GCLK{i}_ENABLE` fuse has to be enabled
  - `STARTUP`: the input is routed to the special `STARTUP` network

  The allowable combinations differ between inputs within a single FB, but don't differ across
  FBs within a single device.  In other words, the set of allowed values for these fuses
  depends only on the `j` coordinate, but not on `i`.

The ZIA lines corresponding to `GCLK*` inputs are gated, for the purpose for power saving.
The gating is per-FB column, and is controlled by the following fuses:

- `FB_COL[i].ZIA_GCLK{j}_ENABLE`: if programmed, `GCLK{i}` can be routed through ZIA to FBs
  within this column.  Otherwise, the `GCLK{i}` option above is non-functional and will result
  in XXX.  Routing `GCLK*` to `FCLK*` is always possible, regardless of this fuse.

TODO: while ISE sets the fuse as if the above is true, disabling the fuse doesn't actually seem to make ZIA GCLK routing not work. what's going on?

The `STARTUP` net is a special device-wide net that is pulsed to `1` for a short time at device
initialization, then remains at `0` afterwards.  It can be connected as the `SET` input
to a register to effectively obtain a `1`-initialized register.


## FCLK networks

Every FB has two fast clock nets, `FCLK[0-1]`.  They can be routed to the clock input
of every MC within the FB.  They can be routed from `GCLK*` input pads in many combinations.

The `FCLK*` routing is selected by a single fuse set per FB that selects both inputs
at once:

- `FB[i].FCLK_MUX`: selects `FCLK*` routing for this FB.  The values are:

  - `GCLK{j}_GCLK{k}`: routes `FCLK0` to `GCLK{j}` and `FCLK1` to `GCLK{k}`
  - `GCLK{j}_NONE`: routes `FCLK0` to `GCLK{j}` and `FCLK1` to const 0
  - `NONE_GCLK{j}`: routes `FCLK0` to const 0 and `FCLK1` to `GCLK{j}`
  - `NONE`: routes `FCLK[0-1]` to const 0

  See the database for the exact set of allowed values.


## Product terms

As the name suggests, XPLA3 has a PLA-like structure.  Each FB has 48 product terms, all of
which are routable to sum terms of all MCs within that FB.  We call them `FB[i].PT[j]`. In addition, each product term
also has a single dedicated function it can be used for:

- PTs 0-7 are used to derive Local Control Terms (LCTs), which can be routed to special inputs
  of all MCs within the FB
- PTs 8-38 (even) can be used as fast data input (D1) of MCs 0-15, respectively.  This data input
  can be combined with the slower sum term input via a programmable LUT2.
- PTs 9-39 (odd) can be used as dedicated clock or CE input to registers of MCs 0-15, respectively.
- PTs 40-47 can be used for foldback NAND: the output of these product term is inverted and can
  be further used as input to other product terms within the same FB.

The inputs to all product terms within the FB are the same and include:

- the 40 `FB[i].IM[*]` signals, both true and negated
- the 8 foldback NAND signals from the same FB, `FB[i].PT[40-47]`, negated only

The fuses controlling a product term are:

- `FB[i].PT[j].IM[k].P`: if programmed (set to 0), `FB[i].IM[k]` is included in the product term (true polarity)
- `FB[i].PT[j].IM[k].N`: if programmed (set to 0), `~FB[i].IM[k]` is included in the product term (inverted polarity)
- `FB[i].PT[j].FBN[k]`: if programmed (set to 0), `~FB[i].PT[40+k]` is included in the product term (foldback NAND, inverted polarity)


### Local control terms

Every FB has 8 LCTs (local control terms), which are derived from PT 0-7 respectively, via an optional inversion::

  FB[i].LCT{j} = FB[i].PT[j] ^ FB[i].LCT{j}_INV

The fuses controlling LCTs are:

- `FB[i].LCT{j}_INV`: if programmed, LCT j is inverse of PT j; otherwise LCT j is equal to PT jed_to_jtag

The LCTs can be used for the following purposes:

| LCT | CLK | RST | SET | CE  | OE  | UCT |
| --- | --- | --- | --- | --- | --- | --- |
| 0   | ❌   | ✅   | ✅   | ❌   | ✅   | ❌   |
| 1   | ❌   | ✅   | ✅   | ❌   | ✅   | ❌   |
| 2   | ❌   | ✅   | ✅   | ❌   | ✅   | ❌   |
| 3   | ❌   | ✅   | ✅   | ❌   | ❌   | ❌   |
| 4   | ✅   | ✅   | ✅   | ✅   | ❌   | ❌   |
| 5   | ✅   | ✅   | ✅   | ❌   | ❌   | ❌   |
| 6   | ✅   | ❌   | ❌   | ❌   | ✅   | \*  |
| 7   | ✅   | ❌   | ❌   | ❌   | ❌   | ✅   |

\*: on XCR3032XL only


### Universal control terms

The device has 4 UCTs (universal control terms).  On the XCR3032XL, all UCTs are routable
from LCT6 and LCT7 of all FBs.  On other devices, all the UCTs are routaeble from only LCT7 off
all FBs.

The UCTs can be routed to special inputs of all macrocells on the device.  They have fixed functions:

- UCT 0: can be routed to OE of all MCs
- UCT 1: can be routed to RST of all MCs
- UCT 2: can be routed to SET of all MCs
- UCT 3: can be routed to CLK of all MCs

The "universal" name is a slight misnomer.  On all devices other than XCR3512XL, the UCTs are truly
universal and cover the entire device.  However, on XCR3512XL, the FBs of the device are
divided into two FB groups, and each group has their own UCT muxes.  The vendor toolchain doesn'T
make use of this functionality and always mirrors UCT mux setting between the two groups.

The relevant fuse sets are:

- `FB_GROUP[i].UCT{j}`: selects the signal routed to `UCT{j}` of FB group `i` (all devices other than XCR3512XL have only FB group 0)

  - `FB{k}_LCT{l}`: the UCT is driven by the given LCT of the given FB
  - `NONE`: the UCT is unused and will be const-0

TODO: exact FB assignment to groups on XCR3512XL


## Sum term, LUT2

Every MC in the device has a sum term, `FB[i].MC[j].SUM`, which can be constructed from all
product terms within the same FB.  Is is controlled by the following fuses:

- `FB[i].MC[j].SUM.PT[k]`: if programmed (set to 0), `FB[i].PT[k]` is included in the sum term for `FB[i].MC[j]`

The sum term, together with the fast input product term, are fed into a programmable LUT2 which
determines the given macrocell's combinatorial output and can also be routed to the register's
data input.  The LUT2 can be used for many purposes:

- select between the sum term and the fast product term
- serve as a XOR or AND gate between the sum term and fast product term
- perform inversion

The LUT2 is controlled by a fuse set:

- `FB[i].MC[j].LUT`: a 4-bit fuse set configuring the LUT2

The LUT2 works as follows:

```
select = FB[i].MC[j].SUM | FB[i].PT[8 + j * 2] << 1
FB[i].MC[j].LUT_OUT = FB[i].MC[j].LUT[select]
```


## Register

Each macrocell has a register.  It has:

- four modes of operation:
  - DFF
  - TFF
  - D latch
  - DFF with clock enable
- D or T input routable from one of:
  - LUT output
  - pad input buffer (so-called fast input register)
  - the Q output of the previous register in the FB (wrapping from 0 to 15) (so-called fast shift register)
  - the Q output of the next register in the FB (wrapping from 15 to 0) (so-called fast shift register)
- clock or gate input routable from one of:
  - `FCLK*`
  - `LCT[4-7]`
  - `UCT3`
  - dedicated per-MC product term
- configurable inversion on the clock or gate input
- (in DFF with clock enable mode only) clock enable input routable to one of:
  - dedicated per-MC product term
  - `LCT4`
- asynchronous reset, routable from:
  - `LCT[0-5]`
  - `UCT1`
  - const-0
- asynchronous set, routable from:
  - `LCT[0-5]`
  - `UCT2`
  - const-0
- initial state of 0

The fuses involved are:

- `FB[i].MC[j].CLK_MUX`: selects CLK input

  - `PT`: dedicated product term `9 + j * 2`
  - `FCLK[0-1]`: per-FB `FCLK[0-1]` network
  - `LCTx`: per-FB local control term
  - `UCT3`: universal control term

- `FB[i].MC[j].CLK_INV`: if programmed, the CLK input is inverted (ie. clock is negedge)

- `FB[i].MC[j].RST_MUX`: selects RST input

  - `LCTx`: per-FB local control term
  - `UCT1`: universal control term
  - `GND`: const 0

- `FB[i].MC[j].SET_MUX`: selects SET input


  - `LCTx`: per-FB local control term
  - `UCT1`: universal control term
  - `GND`: const 0

- `FB[i].MC[j].CE_MUX`: selects CE input (only relevant in `DFFCE` mode)

  - `PT`: dedicated product term `9 + j * 2`
  - `LCTx`: per-FB local control term

- `FB[i].MC[j].REG_MODE`: selects FF mode

  - `DFF`
  - `TFF`
  - `LATCH`
  - `DFFCE`

- `FB[i].MC[j].REG_D_IREG`: if programmed, and the `REG_D_SHIFT` fuse is not programmed, the register D input is connected to `FB[i].MC[j].IOB.I`; if neither is programmed, the register D input is connected to `FB[i].MC[j].LUT_OUT`
- `FB[i].MC[j].REG_D_SHIFT`: if programmed, the register D input is connected to the previous or next MC's register Q output; otherwise, the connection is determined by `REG_D_IREG` fuse
- `FB[i].MC[j].REG_D_SHIFT_DIR`: when the previous fuse is programmed, determines which MC's register Q output is connected to this register's D input

  - `UP`: D input connected to `FB[i].MC[(j - 1) % 16].REG`
  - `DOWN`: D input connected to `FB[i].MC[(j + 1) % 16].REG`

The register works as follows:

```
case(FB[i].MC[j].CLK_MUX)
    PT: FB[i].MC[j].CLK = FB[i].PT[9 + j * 2]. ^ FB[i].MC[j].CLK_INV;
    LCTx: FB[i].MC[j].CLK = FB[i].LCTx ^ FB[i].MC[j].CLK_INV;
    UCT3: FB[i].MC[j].CLK = FB_GROUP[fb_to_fb_group(i)].UCT3 ^ FB[i].MC[j].CLK_INV;
    FCLKx: FB[i].MC[j].CLK = FB[i].FCLKx ^ FB[i].MC[j].CLK_INV;
endcase

case(FB[i].MC[j].RST_MUX)
    LCTx: FB[i].MC[j].RST = FB[i].LCTx;
    UCT1: FB[i].MC[j].RST = FB_GROUP[fb_to_fb_group(i)].UCT1;
    GND: FB[i].MC[j].RST = 0;
endcase

case(FB[i].MC[j].SET_MUX)
    LCTx: FB[i].MC[j].SET = FB[i].LCTx;
    UCT2: FB[i].MC[j].SET = FB_GROUP[fb_to_fb_group(i)].UCT2;
    GND: FB[i].MC[j].SET = 0;
endcase

case(FB[i].MC[j].CE_MUX)
    PT: FB[i].MC[j].CE = FB[i].PT[9 + j * 2];
    LCT4: FB[i].MC[j].CE = FB[i].LCT4;
endcase

if (FB[i].MC[j].REG_D_SHIFT)
    case(FB[i].MC[j].REG_D_SHIFT_DIR)
    UP: FB[i].MC[j].REG_D = FB[i].MC[(j - 1) % 16].REG;
    DOWN: FB[i].MC[j].REG_D = FB[i].MC[(j + 1) % 16].REG;
    endcase
else if (FB[i].MC[j].REG_D_IREG)
    FB[i].MC[j].REG_D = FB[i].MC[j].IOB.I;
else
    FB[i].MC[j].REG_D = FB[i].MC[j].LUT_OUT;

initial FB[i].MC[j].REG = 0;

case(FB[i].MC[j].REG_MODE)
    // Pretend the usual synth/sim mismatch doesn't happen.
    DFF:
        always @(posedge FB[i].MC[j].CLK, posedge FB[i].MC[j].RST, posedge FB[i].MC[j].SET)
            if (FB[i].MC[j].RST)
                FB[i].MC[j].REG = 0;
            else if (FB[i].MC[j].SET)
                FB[i].MC[j].REG = 1;
            else
                FB[i].MC[j].REG = FB[i].MC[j].REG_D;
    TFF:
        always @(posedge FB[i].MC[j].CLK, posedge FB[i].MC[j].RST, posedge FB[i].MC[j].SET)
            if (FB[i].MC[j].RST)
                FB[i].MC[j].REG = 0;
            else if (FB[i].MC[j].SET)
                FB[i].MC[j].REG = 1;
            else
                FB[i].MC[j].REG ^= FB[i].MC[j].REG_D;
    LATCH:
        always @*
            if (FB[i].MC[j].RST)
                FB[i].MC[j].REG = 0;
            else if (FB[i].MC[j].SET)
                FB[i].MC[j].REG = 1;
            else if (FB[i].MC[j].CLK)
                FB[i].MC[j].REG = FB[i].MC[j].REG_D;
    DFFCE:
        always @(posedge FB[i].MC[j].CLK, posedge FB[i].MC[j].RST, posedge FB[i].MC[j].SET)
            if (FB[i].MC[j].RST)
                FB[i].MC[j].REG = 0;
            else if (FB[i].MC[j].SET)
                FB[i].MC[j].REG = 1;
            else if (FB[i].MC[j].CE)
                FB[i].MC[j].REG = FB[i].MC[j].REG_D;
endcase
```

## Macrocell and IOB outputs

Macrocells come in two variants: ones with IOBs and buried macrocells (without IOBs).

A macrocell with IOB has two outputs to ZIA:

- `FB[i].MC[j].MC_ZIA_OUT`: routable from the LUT2 or register output
- `FB[i].MC[j].IOB_ZIA_OUT`: routable from the IOB's input buffer or register output

A macrocell with IOB also has an output to the IOB:

- `FB[i].MC[j].MC_IOB_OUT`: routable from the LUT2 or register output

The muxes for all three of the above signals are independent, allowing any two of
IBUF, LUT2, and register output to be routed to the ZIA, and also independently connect
either LUT2 or register output to the output buffer.

A buried macrocell has only one output to ZIA, `FB[i].MC[j].MC_ZIA_OUT`.  It is thus
not possible to use both its combinatorial and registered output at the same time.

The fuses involved are:

- `FB[i].MC[j].MC_ZIA_MUX`: controls routing to `FB[i].MC[j].MC_ZIA_OUT`

  - `REG`: routes from `FB[i].MC[j].REG`
  - `LUT`: routes from `FB[i].MC[j].LUT_OUT`

- `FB[i].MC[j].MC_IOB_MUX`: controls routing to `FB[i].MC[j].MC_IOB_OUT`

  - `REG`: routes from `FB[i].MC[j].REG`
  - `LUT`: routes from `FB[i].MC[j].LUT_OUT`

- `FB[i].MC[j].IOB_ZIA_MUX`: controls routing to `FB[i].MC[j].IOB_ZIA_OUT`

  - `REG`: routes from `FB[i].MC[j].REG`
  - `IBUF`: routes from `FB[i].MC[j].IOB.I`


## Input/output buffer

All I/O buffers (except `GCLK*` pins which are input-only) are associated with a macrocell.
Not all MCs have associated IOBs.

The output buffer is controlled by the `FB[i].MC[j].MC_IOB_OUT` and `FB[i].MC[j].OE` signals of the macrocell.

The OE signal is routable from:

- `GND`: const-0 (input-only pin)
- `VCC`: const-1 (output-only pin)
- `LCT[0126]`: local control terms
- `UCT0`: universal control term
- `PULLUP`: special option; like `GND`, but also enables a weak pull-up on the pin

The output slew rate is programmable between two settings, "fast" and "slow".

Each I/O buffer has the following fuses:

- `FB[i].MC[j].OE_MUX`: selects output enable source

  - `GND`: const-0
  - `VCC`: const-1
  - `LCTx`: per-FB local control term
  - `UCT1`: universal control term
  - `PULLUP`: const-0 and enable weak pull-up resistor

- `FB[i].MC[j].IOB_SLEW`: selects slew rate, one of:

  - `SLOW`
  - `FAST`


## Misc configuration

The XPLA3 devices also have some global configuration fuses that affect the whole device:

- `READ_PROT`: if programmed, the device is read-protected, and the bitstream cannot be read back (except for UES area)
- `ISP_DISABLE`: if programmed, and not overriden by `PORT_EN`, the special function of the JTAG pins is disabled, and the pins are controlled by the macrocells as normal I/O; if not programmed, the JTAG pins retain their special functions and the relevant IOBs are disconnected from macrocell control
- `UES` (user electronic signature): this scratchpad multi-bit field can be used for any user-defined purpose; it is exempt from read protection; the exact size of this field varies with device

When used by ISE, the UES field stores 8-bit ASCII data, with MSB-first bit numbering (ie. bit 0 of `UES` is the MSB of first character).
