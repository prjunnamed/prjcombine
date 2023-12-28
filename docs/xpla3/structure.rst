Device structure
################

Overview
========

An XPLA3 device is made of:

- the ZIA (Zero-power Interconnect Array), which routes various signals to FB inputs; the routable signals include:

  - MC outputs
  - IOB outputs (ie. input buffers from general purpose I/O)
  - GCLK input buffers
  - a special POR (power-on reset) signal that is pulsed at device startup

- 2-32 FBs (function blocks), each of which has:

  - two ``FCLK[i]`` fast clock networks, routable from ``GCLK`` pads
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

  - the 4 GCLK dedicated inputs, routable to ZIA and per-FB ``FCLK`` networks
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


ZIA and FB inputs
=================

.. todo:: write me


FCLK networks
=============

.. todo:: write me


Product terms
=============

.. todo:: write me


Local control terms
-------------------

.. todo:: write me


Universal control terms
-----------------------

.. todo:: write me


Sum term, LUT2
==============

.. todo:: write me


Register
========

.. todo:: write me


Macrocell and IOB outputs
=========================

.. todo:: write me


Input/output buffer
===================

.. todo:: write me


Misc configuration
==================

.. todo:: write me
