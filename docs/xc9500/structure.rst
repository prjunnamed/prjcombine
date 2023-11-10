Device structure
################

Overview
========

An XC9500 family device is made of:

- the UIM (universal interconnect matrix), which routes MC and IOB outputs to FB inputs
- 2-16 FBs (function blocks), each of which has:

  - 36 (XC9500) or 54 (XC9500XL/XV) routable inputs from UIM
  - 18 MCs (macrocells), each of which has:

    - configurable low power / high performance mode
    - 5 PTs (product terms)
    - PT router, which can route PTs to:

      - the sum term for this MC
      - the sum term for export into neighbouring MCs
      - a special function (OE, RST, SET, CLK, CE, XOR depending on the PT)
    
    - PT export/import logic for borrowing PTs between neighbouring MCs
    - a sum term
    - a dedicated XOR gate
    - optional inverter
    - a flip-flop, with:

      - configurable DFF or TFF function
      - configurable initial value
      - clock (freely invertible on XC9500XL/XV), routable from FCLK or PT
      - async reset, routable from FSR or PT
      - async set, routable from FSR or PT
      - (XC9500XL/XV only) clock enable, routable from PT

    - a single output (selectable from combinatorial or FF output), routed to IOB and UIM
    - (XC9500 only) UIM output enable and inversion
    - IOB (input/output buffer) (on larger devices, not all macrocells have an IOB), with:

      - input buffer (routed to UIM)
      - output enable (freely invertible on XC9500XL/XV), routable from FOE or PT
      - configurable slew rate (fast or slow)
      - programmable ground

- global signals

  - 3 FCLK (fast clock) signals

    - (XC9500) freely invertible and routable from GCLK pins
    - (XC9500XL/XV) hardwired 1-1 to GCLK pins

  - 1 FSR (fast set/reset) signal

    - freely invertible
    - always routed from GSR pin

  - 2-4 FOE (fast output enable) signals

    - (XC9500) freely invertible and routable from GOE pins
    - (XC9500XL/XV) hardwired 1-1 to GOE pins

- global pull-up enable (only meant to be used in unconfigured devices)
- (XC9500XL/XV) global bus keeper enable
- special global configuration bits

  - 32-bit standard JTAG USERCODE
  - read protection enable
  - write protection enable
  - (XC9500XV only) DONE bit


UIM and FB inputs — XC9500
==========================

The core interconnect structure in XC9500 devices is the UIM, Universal Interconnect Matrix.
The name is quite appropriate, at least for internal signals: any FB input can be routed
to any MC output.  More than that, any FB input can be routed to a *wire-AND* of an arbitrary
subset of MC outputs from the entire device.  Together with the UIM OE functionality within
the MC, this can be used for emulated internal tri-state buses.

The name is, however, less appropriate when it comes to external signals: a given FB input
can only be routed to some subset of input signals from IOBs.  The set of routable IOBs
depends on the FB input index and the device.

Additionally, on devices other than XC9536, some FB inputs can be routed to "fast feedback"
paths, which come straight from MC outputs within the same FB.  This is functionally redundant
with the wire-AND path, but much faster.

Each FB has 36 inputs, which we call ``FB[i].IM[j]``.  Each FB input is controlled by two sets of fuses:

- mux fuses (``FB[i].IM[j].MUX``) select what is routed to the input.  The combinations include:

  - ``EMPTY``: the input is a constant ??? (for unused inputs)
  - ``UIM``: the input is a wire-AND of MC outputs (and the second set of fuses is relevant)
  - ``FBK_MC{k}``: the input is routed through fast feedback path from ``FB[i].MC[k].OUT``
  - ``PAD_FB{k}_MC{l}``: the input is routed from an input buffer ``FB[k].MC[l].IOB.I``

  The allowable combinations differ between inputs within a single FB, but don't differ across
  FBs within a single device.  In other words, the set of allowed values for these fuses
  depends only on the ``j`` coordinate, but not on ``i``.

- wire-AND fuses (``FB[i].IM[j].UIM.FB[k].MC[l]``) select which MC outputs participate in the
  wire-AND.  If a given fuse is programmed, it means that ``FB[k].MC[l].OUT_UIM`` is included
  in the product.  These fuses are only relevant when the mux fuse set is set to ``UIM``.

.. todo:: check ``EMPTY`` semantics

.. todo:: verify ``FBK`` doesn't go through OE and inversion


UIM and FB inputs — XC9500XL/XV
===============================

The core interconnect structure in XC9500XL/XV devices is the UIM 2.  This version of the UIM
is not really universal, and is much more of a classic CPLD design.

Each FB has 54 inputs, which we call ``FB[i].IM[j]``.  Each FB input is controlled by a set of fuses:

- mux fuses (``FB[i].IM[j].MUX``) select what is routed to the input.  The combinations include:

  - ``EMPTY``: the input is a constant ??? (for unused inputs)
  - ``FB{k}_MC{l}``: the input is routed from the macrocell output ``FB[k].MC[l].OUT``
  - ``PAD_FB{k}_MC{l}``: the input is routed from the input buffer ``FB[k].MC[l].IOB.I``

  The allowable combinations differ between inputs within a single FB, but don't differ across
  FBs within a single device.  In other words, the set of allowed values for these fuses
  depends only on the ``j`` coordinate, but not on ``i``.

.. todo:: check ``EMPTY`` semantics


FB global fuses
===============

Each function block has two fuses controlling the entire FB:

- ``FB[i].ENABLE``: function block enable; needs to be programmed to use this function block
- ``FB[i].EXPORT_ENABLE``: function block PT export enable; needs to be programmed to use PT
  import/export within this function block

.. todo:: determine what these do exactly


Product terms
=============

Each function block has 90 product terms, 5 per macrocell.  We call them ``FB[i].MC[j].PT[k]``.
Each of the 5 PTs has a dedicated function, as follows:

- ``*.PT[0]``: clock
- ``*.PT[1]``: output enable
- ``*.PT[2]``: async reset (or clock enable on XC9500XL/XV)
- ``*.PT[3]``: async set (or clock enable on XC9500XL/XV)
- ``*.PT[4]``: second XOR input

The inputs to all product terms within a FB are the same, and consist of all FB inputs, in both
true and inverted forms.

Each product term can be individually configured as low power or high performance.  This affects propagation time.

Each product term can be routed to at most one of three destinations:

- ``OR_MAIN``: input to the MC's sum term
- ``OR_EXPORT``: input to the export OR gate
- ``SPECIAL``: used for the dedicated function

The fuses controlling a product term are:

- ``FB[i].MC[j].PT[k].IM[l].P``: if programmed, ``FB[i].IM[l]`` is included in the product term (true polarity)
- ``FB[i].MC[j].PT[k].IM[l].N``: if programmed, ``~FB[i].IM[l]`` is included in the product term (inverted polarity)
- ``FB[i].MC[j].PT[k].HP``: if programmed, the product term is in high performance mode; otherwise, it is in low power mode
- ``FB[i].MC[j].PT[k].ALLOC``: has one of four values:

  - ``EMPTY``: product term is unused
  - ``OR_MAIN``: product term is used for MC's sum term; dedicated function wired to 0
  - ``OR_EXPORT`` product term is used for export sum term; dedicated function wired to 0
  - ``SPECIAL``: product term is used for the dedicated function

The product term's corresponding dedicated function is called ``FB[i].MC[j].PT[k].SPECIAL``.
It is equal to ``FB[i].MC[j].PT[k]`` if the dedicated function is enabled in ``ALLOC``, 0 otherwise.

PT import/export
----------------

Product terms can be borrowed between neighbouring MCs within a FB.  To this end, each MC has two outputs,
``FB[i].MC[j].EXPORT_{UP|DOWN}``, and two inputs, ``FB[i].MC[j].IMPORT_{UP|DOWN}``.  The "down" direction
corresponds to exporting PTs toward lower-numbered MCs (with a wraparound from 0 to 17), and the "up"
direction corresponds to exporting PTs towards higher-numbered MCs (with a wraparound from 17 to 0).
Accordingly, we have::

    FB[i].MC[j].IMPORT_DOWN = FB[i].MC[(j + 1) % 18].EXPORT_DOWN;
    FB[i].MC[j].IMPORT_UP = FB[i].MC[(j - 1) % 18].EXPORT_UP;

A MC can only export product terms in one direction (up or down).
Product terms can be imported from both directions at once.
Imported terms from a given direction can be used for either the main sum term, or for further export, but not both at once.

PT import/export is controlled by the following per-MC fuses:

- ``FB[i].MC[j].EXPORT_DIR``: one of:

  - ``DOWN``: exports PTs downwards
  - ``UP``: exports PTs upwards

- ``FB[i].MC[j].IMPORT_UP_ALLOC``: one of:

  - ``OR_MAIN``: includes PTs imported upwards in the main sum term
  - ``OR_EXPORT``: includes PTs imported upwards in the export sum term

- ``FB[i].MC[j].IMPORT_DOWN_ALLOC``: one of:

  - ``OR_MAIN``: includes PTs imported downwards in the main sum term
  - ``OR_EXPORT``: includes PTs imported downwards in the export sum term

Additionally, the per-FB ``FB[i].EXPORT_ENABLE`` fuse needs to be set if any term within a FB is exported or imported.

Export works as follows::

    FB[i].MC[j].EXPORT = 
        (FB[i].MC[j].IMPORT_UP_ALLOC == OR_EXPORT ? FB[i].MC[j].IMPORT_UP : 0) |
        (FB[i].MC[j].IMPORT_DOWN_ALLOC == OR_EXPORT ? FB[i].MC[j].IMPORT_DOWN : 0) |
        (FB[i].MC[j].PT[0].ALLOC == OR_EXPORT ? FB[i].MC[j].PT[0] : 0) |
        (FB[i].MC[j].PT[1].ALLOC == OR_EXPORT ? FB[i].MC[j].PT[1] : 0) |
        (FB[i].MC[j].PT[2].ALLOC == OR_EXPORT ? FB[i].MC[j].PT[2] : 0) |
        (FB[i].MC[j].PT[3].ALLOC == OR_EXPORT ? FB[i].MC[j].PT[3] : 0) |
        (FB[i].MC[j].PT[4].ALLOC == OR_EXPORT ? FB[i].MC[j].PT[4] : 0);

    FB[i].MC[j].EXPORT_UP = (FB[i].MC[j].EXPORT_DIR == UP ? FB[i].MC[j].EXPORT : 0);
    FB[i].MC[j].EXPORT_DOWN = (FB[i].MC[j].EXPORT_DIR == DOWN ? FB[i].MC[j].EXPORT : 0);

.. todo:: verify all of the above


Sum term, XOR gate
==================

Each macrocell has a main sum term, which includes all product terms and imports routed towards it::

    FB[i].MC[j].SUM = 
        (FB[i].MC[j].IMPORT_UP_ALLOC == OR_MAIN ? FB[i].MC[j].IMPORT_UP : 0) |
        (FB[i].MC[j].IMPORT_DOWN_ALLOC == OR_MAIN ? FB[i].MC[j].IMPORT_DOWN : 0) |
        (FB[i].MC[j].PT[0].ALLOC == OR_MAIN ? FB[i].MC[j].PT[0] : 0) |
        (FB[i].MC[j].PT[1].ALLOC == OR_MAIN ? FB[i].MC[j].PT[1] : 0) |
        (FB[i].MC[j].PT[2].ALLOC == OR_MAIN ? FB[i].MC[j].PT[2] : 0) |
        (FB[i].MC[j].PT[3].ALLOC == OR_MAIN ? FB[i].MC[j].PT[3] : 0) |
        (FB[i].MC[j].PT[4].ALLOC == OR_MAIN ? FB[i].MC[j].PT[4] : 0);

The sum term then goes through a XOR gate (whose other input is either 0 or a dedicated PT) and a programmable inverter::

    FB[i].MC[j].XOR = FB[i].MC[j].SUM ^ FB[i].MC[j].PT[4].SPECIAL ^ FB[i].MC[j].INV;

The fuses involved are:

- ``FB[i].MC[j].SUM_HP``: if programmed, the sum term is in high performance mode; otherwise, it's in low power mode
- ``FB[i].MC[j].INV``: if programmed, the output of the XOR gate is further inverted

Flip-flop
=========

Each macrocell includes a flip-flop.  It has:

- configurable DFF or TFF function
- D or T input connected to the (potentially inverted) XOR gate output
- configurable initial value
- clock (freely invertible on XC9500XL/XV), routable from FCLK or ``PT[0]``
- async reset, routable from FSR or ``PT[2]``
- async set, routable from FSR or ``PT[3]``
- (XC9500XL/XV only) clock enable, routable from ``PT[2]`` or ``PT[3]``

The fuses involved are:

- ``FB[i].MC[j].CLK_MUX``: selects CLK input

  - ``PT``: product term 0 dedicated function
  - ``FCLK[0-2]``: global ``FCLK[0-2]`` network

- ``FB[i].MC[j].CLK_INV``: if programmed, the CLK input is inverted (ie. clock is negedge) (XC9500XL/XV only)

- ``FB[i].MC[j].RST_MUX``: selects RST input

  - ``PT``: product term 2 dedicated function or 0; if PT 2 is used for clock enable, it will not be routed to RST (0 will be substituted)
  - ``FSR``: global ``FSR`` network

- ``FB[i].MC[j].SET_MUX``: selects SET input

  - ``PT``: product term 3 dedicated function or 0; if PT 3 is used for clock enable, it will not be routed to SET (0 will be substituted)
  - ``FSR``: global ``FSR`` network

- ``FB[i].MC[j].CE_MUX``: selects CE input (XC9500XL/XV only)

  - ``NONE``: const 1
  - ``PT2``: product term 2 dedicated function
  - ``PT3``: product term 3 dedicated function

- ``FB[i].MC[j].INIT``: if programmed, the initial value of the FF is 1; otherwise, it is 0

- ``FB[i].MC[j].REG_MODE``: selects FF mode

  - ``DFF``
  - ``TFF``

On XC9500, the FF works as follows::

    case(FB[i].MC[j].CLK_MUX)
    PT: FB[i].MC[j].CLK = FB[i].MC[j].PT[0].SPECIAL;
    FCLK0: FB[i].MC[j].CLK = FCLK[0];
    FCLK1: FB[i].MC[j].CLK = FCLK[1];
    FCLK2: FB[i].MC[j].CLK = FCLK[2];
    endcase

    case(FB[i].MC[j].RST_MUX)
    PT: FB[i].MC[j].RST = FB[i].MC[j].PT[2].SPECIAL;
    FSR: FB[i].MC[j].RST = FSR;
    endcase

    case(FB[i].MC[j].SET_MUX)
    PT: FB[i].MC[j].SET = FB[i].MC[j].PT[3].SPECIAL;
    FSR: FB[i].MC[j].SET = FSR;
    endcase

    initial FB[i].MC[j].FF = FB[i].MC[j].INIT;

    // Pretend the usual synth/sim mismatch doesn't happen.
    always @(posedge FB[i].MC[j].CLK, posedge FB[i].MC[j].RST_MUX, posedge FB[i].MC[j].SET_MUX)
        if (FB[i].MC[j].RST_MUX)
            FB[i].MC[j].FF = 0;
        else if (FB[i].MC[j].SET_MUX)
            FB[i].MC[j].FF = 1;
        else
            FB[i].MC[j].FF = FB[i].MC[j].XOR;

On XC9500XL/XV, the FF works as follows::

    case(FB[i].MC[j].CLK_MUX)
    PT: FB[i].MC[j].CLK = FB[i].MC[j].PT[0].SPECIAL ^ FB[i].MC[j].CLK_INV;
    FCLK0: FB[i].MC[j].CLK = FCLK[0] ^ FB[i].MC[j].CLK_INV;
    FCLK1: FB[i].MC[j].CLK = FCLK[1] ^ FB[i].MC[j].CLK_INV;
    FCLK2: FB[i].MC[j].CLK = FCLK[2] ^ FB[i].MC[j].CLK_INV;
    endcase

    case(FB[i].MC[j].RST_MUX)
    PT: FB[i].MC[j].RST = (FB[i].MC[j].CE_MUX == PT2 ? 0 : FB[i].MC[j].PT[2].SPECIAL);
    FSR: FB[i].MC[j].RST = FSR;
    endcase

    case(FB[i].MC[j].SET_MUX)
    PT: FB[i].MC[j].SET = (FB[i].MC[j].CE_MUX == PT3 ? 0 : FB[i].MC[j].PT[3].SPECIAL);
    FSR: FB[i].MC[j].SET = FSR;
    endcase

    case(FB[i].MC[j].CE_MUX)
    PT2: FB[i].MC[j].CE = FB[i].MC[j].PT[2].SPECIAL;
    PT3: FB[i].MC[j].CE = FB[i].MC[j].PT[3].SPECIAL;
    NONE: FB[i].MC[j].CE = 1;
    endcase

    initial FB[i].MC[j].FF = FB[i].MC[j].INIT;

    // Pretend the usual synth/sim mismatch doesn't happen.
    always @(posedge FB[i].MC[j].CLK, posedge FB[i].MC[j].RST_MUX, posedge FB[i].MC[j].SET_MUX)
        if (FB[i].MC[j].RST_MUX)
            FB[i].MC[j].FF = 0;
        else if (FB[i].MC[j].SET_MUX)
            FB[i].MC[j].FF = 1;
        else if (FB[i].MC[j].CE)
            FB[i].MC[j].FF = FB[i].MC[j].XOR;

.. todo:: verify (in particular, CE vs RST/SET shenanigans)


Macrocell output — XC9500
=========================

The output of the macrocell can be selected from combinatorial (XOR gate output) or registered (FF output)::

    FB[i].MC[j].OUT = (FB[i].MC[j].OUT_MUX == COMB ? FB[i].MC[j].XOR : FB[i].MC[j].FF);

The macrocell also has an output enable, which can be from a product term, or a global network.
It can be used for the IOB output buffer, for UIM output, or both::

    case(FB[i].MC[j].OE_MUX)
    PT: FB[i].MC[j].OE = FB[i].MC[j].PT[1].SPECIAL;
    FOE0: FB[i].MC[j].OE = FOE[0];
    FOE1: FB[i].MC[j].OE = FOE[1];
    FOE2: FB[i].MC[j].OE = FOE[2];
    FOE3: FB[i].MC[j].OE = FOE[3];
    endcase

    case(FB[i].MC[j].UIM_OE_MUX)
    GND: FB[i].MC[j].UIM_OE = 0;
    VCC: FB[i].MC[j].UIM_OE = 1;
    OE_MUX: FB[i].MC[j].UIM_OE = FB[i].MC[j].OE;
    endcase

    case(FB[i].MC[j].IOB_OE_MUX)
    GND: FB[i].MC[j].IOB_OE = 0;
    VCC: FB[i].MC[j].IOB_OE = 1;
    OE_MUX: FB[i].MC[j].IOB_OE = FB[i].MC[j].OE;
    endcase

The output is routed to up to three places:

- this MC's IOB (``FB[i].MC[j].OUT``)
- this FB's inputs via the feedback path (``FB[i].MC[j].OUT``)
- the UIM (``FB[i].MC[j].OUT_UIM``)

The output to UIM additionally goes through (emulated) output enable logic, which can be used
to implement emulated on-chip tristate buses in conjunction with the UIM wire-AND logic::

    FB[i].MC[j].OUT_UIM = FB[i].MC[j].UIM_OE ? FB[i].MC[j].OUT ^ FB[i].MC[j].UIM_OUT_INV : 1;

The fuses involved are:

- ``FB[i].MC[j].OUT_MUX``: selects output mode

  - ``COMB``: combinatorial, the output is connected to XOR gate output
  - ``FF``: registered, the output is connected to FF output

- ``FB[i].MC[j].OE_MUX``: selects output enable source

  - ``PT``: product term 1 dedicated function or 0
  - ``FOE[0-3]``: global ``FOE[0-3]`` network

- ``FB[i].MC[j].UIM_OE_MUX``: selects output enable for UIM output

  - ``GND``: const 0
  - ``VCC``: const 1
  - ``OE_MUX``: the input selected by the ``OE_MUX`` fuses

- ``FB[i].MC[j].IOB_OE_MUX``: selects output enable for IOB output

  - ``GND``: const 0
  - ``VCC``: const 1
  - ``OE_MUX``: the input selected by the ``OE_MUX`` fuses

- ``FB[i].MC[j].UIM_OUT_INV``: if programmed, the output to UIM is inverted


.. todo:: verify all of the above (in particular, feedback path vs OE and UIM out)


Macrocell output — XC9500XL/XV
==============================

The output of the macrocell can be selected from combinatorial (XOR gate output) or registered (FF output)::

    FB[i].MC[j].OUT = (FB[i].MC[j].OUT_MUX == COMB ? FB[i].MC[j].XOR : FB[i].MC[j].FF);

The output is routed to the UIM and this MC's IOB.

The macrocell also has an output enable, which can be from a product term, or a global network, and can
be freely inverted.  It is used for the IOB output buffer::

    case(FB[i].MC[j].OE_MUX)
    PT: FB[i].MC[j].IOB_OE = FB[i].MC[j].PT[1].SPECIAL ^ FB[i].MC[j].OE_INV;
    FOE0: FB[i].MC[j].IOB_OE = FOE[0] ^ FB[i].MC[j].OE_INV;
    FOE1: FB[i].MC[j].IOB_OE = FOE[1] ^ FB[i].MC[j].OE_INV;
    FOE2: FB[i].MC[j].IOB_OE = FOE[2] ^ FB[i].MC[j].OE_INV;
    FOE3: FB[i].MC[j].IOB_OE = FOE[3] ^ FB[i].MC[j].OE_INV;
    endcase

The fuses involved are:

- ``FB[i].MC[j].OUT_MUX``: selects output mode

  - ``COMB``: combinatorial, the output is connected to XOR gate output
  - ``FF``: registered, the output is connected to FF output

- ``FB[i].MC[j].OE_MUX``: selects output enable source

  - ``PT``: product term 1 dedicated function or 0
  - ``FOE[0-3]``: global ``FOE[0-3]`` network

- ``FB[i].MC[j].OE_INV``: if programmed, the output enable is inverted


Input/output buffer
===================

All I/O buffers (except dedicated JTAG pins) are associated with a macrocell.  Not all MCs
have associated IOBs.

The output buffer is controlled by the ``FB[i].MC[j].OUT`` and ``FB[i].MC[j].IOB_OE`` signals of the macrocell.
If the pad is supposed to be input only, the OE signal should be programmed to be always 0:

- on XC9500, ``IOB_OE_MUX`` should be set to ``GND``
- on XC9500XL/XV, ``OE_MUX`` should be set to ``PT``, ``OE_INV`` should be unset, and ``PT[1]`` should not be allocated to its dedicated function

Likewise, if the pad is supposed to be always-on output, the OE signal should be programmed to be always 1:

- on XC9500, ``IOB_OE_MUX`` should be set to ``VCC``
- on XC9500XL/XV, ``OE_MUX`` should be set to ``PT``, ``OE_INV`` should be set, and ``PT[1]`` should not be allocated to its dedicated function

The output slew rate is programmable between two settings, "fast" and "slow".

Instead of being connected to MC output, an IOB can also be a "programmed ground", ie be set to always output a const 0
regardless of the ``IOB_OE`` and ``OUT`` signals.  In this case, ``IOB_OE`` should be 0.

The input buffer is connected to the ``FB[i].MC[j].IOB.I`` network.  On all devices other than XC95288 (non-XL/XV),
it is directly connected to the UIM.  On XC95288, there are additional enable fuses for this connection.

Each I/O buffer has the following fuses:

- ``FB[i].MC[j].GND``: if programmed, this pin is "programmed ground" and will always output a const 0
- ``FB[i].MC[j].SLEW``: selects slew rate, one of:

  - ``SLOW``
  - ``FAST``

- ``FB[i].MC[j].IBUF_ENABLE`` (XC95288 only): when programmed, the input buffer is active and connected to UIM

.. todo:: figure out the IBUF_ENABLE thing


Configuration pull-ups
======================

Before the device is configured, all IOBs are configured with a very weak pull-up
resistor (XC9500) or a bus keeper (XC9500XL/XV).  To disable this pull-up, a per-FB fuse
is used which is set in the bitstream:

- ``FB[i].PULLUP_DISABLE``: if programmed, disables the pre-configuration pull-ups / bus keepers for all IOBs in this FB


XC9500XL/XV bus keeper
======================

The XC9500XL/XV have a weak bus keeper in each IOB.  The keeper functionality can only be enabled
globally for all pads on the device, or not at all, via a global fuse:

- ``KEEPER``: if programmed, the bus keeper is enabled on all IOBs


Global networks — XC9500
========================

The device has several global networks for fast control signals.  The networks are always driven by special pads
on the device (which can also be used as normal I/O).  The networks are:

- ``FCLK[0-2]``: clock
- ``FSR``: async set or reset
- ``FOE[0-1]`` (XC9536, XC9572, XC95108) or ``FOE[0-3]`` (XC95144, XC95216, XC95288): output enable

The special pads are:

- ``GCLK[0-2]``: clock
- ``GSR``: async set or reset
- ``GOE[0-1]`` (XC9536, XC9572, XC95108) or ``GOE[0-3]`` (XC95144, XC95216, XC95288): output enable

The mapping of ``G*`` special pads to MCs depends on the device and, in at least one case, the package (!).

The allowed mappings are:

- ``FCLK0``: ``GCLK0``, ``GCLK1``
- ``FCLK1``: ``GCLK1``, ``GCLK2``
- ``FCLK2``: ``GCLK2``, ``GCLK0``
- ``FSR``: ``GSR``
- ``FOE0`` (XC9536, XC9572, XC95108): ``GOE0``, ``GOE1``
- ``FOE1`` (XC9536, XC9572, XC95108): ``GOE0``, ``GOE1``
- ``FOE0`` (XC95144, XC95216, XC95288): ``GOE0``, ``GOE1``
- ``FOE1`` (XC95144, XC95216, XC95288): ``GOE1``, ``GOE2``
- ``FOE2`` (XC95144, XC95216, XC95288): ``GOE2``, ``GOE3``
- ``FOE3`` (XC95144, XC95216, XC95288): ``GOE3``, ``GOE0``

Additionally, all networks can be inverted from their source pins.

The fuses involved are:

- ``FCLK[i].MUX``: selects the input routed to ``FCLK[i]``

  - ``NONE``: const ???
  - ``GCLK[i]``: the corresponding ``GCLK`` pad

- ``FOE[i].MUX``: selects the input routed to ``FOE[i]``

  - ``NONE``: const ???
  - ``FOE[i]``: the corresponding ``FOE`` pad

- ``FCLK[i].INV``: if programmed, the ``GCLK`` pad is inverted before driving ``FCLK[i]`` network
- ``FSR.INV``: if programmed, the ``GSR`` pad is inverted before driving ``FSR`` network
- ``FOE[i].INV``: if programmed, the ``GOE`` pad is inverted before driving ``FOE[i]`` network

.. todo:: consts

.. todo:: no, really, what is it with the XC9572 GOE mapping varying between packages


Global networks — XC9500XL/XV
=============================

Global networks on XC9500XL/XV work similarly, except there are no muxes and no inversion (except for ``FSR``).
Thus the ``GCLK[i]`` and ``GOE[i]`` pads are mapped 1-1 directly to ``FCLK[i]`` and ``FOE[i]`` networks, with only
an enable fuse.

The fuses involved are:

- ``FCLK[i].EN``: if programmed, the given ``FCLK`` network is active and connected to the ``GCLK[i]`` pad
- ``FOE[i].EN``: if programmed, the given ``FOE`` network is active and connected to the ``GOE[i]`` pad
- ``FSR.INV``: if programmed, the ``GSR`` pad is inverted before driving ``FSR`` network

The ``FSR`` network is always enabled.

.. todo:: what does disabled network do


Misc configuration
==================

The devices also include the following misc fuses:

- ``USERCODE``: 32-bit fuse set, readable via the JTAG USERCODE instruction

- ``FB[i].READ_PROT_{A|B}`` (XC9500): if programmed, the device is read protected
- ``FB[i].READ_PROT`` (XC9500XL/XV): if programmed, the device is read protected
- ``FB[i].WRITE_PROT``: if programmed, the device is write protected (needs a special JTAG instruction sequence
  to program/erase)
- ``DONE`` (XC9500XV only): used to mark a fully programmed device; if programmed, the device will complete its
  boot sequence and activate I/O buffers; otherwise, all output buffers will be disabled

Due to their special semantics, the protection fuses and the ``DONE`` fuse should be programmed last, after all other fuses in the bitstream.

.. todo:: exact semantics of protection fuses