# Device structure

## Overview

A Coolrunner II device is made of:

- the AIM (Advanced Interconnect Matrix), which routes MC outputs and input buffer outputs to FB inputs
- 2-32 FBs (function blocks), each of which has:
  - 40 routable inputs from AIM
  - 56 PTs (product terms) shared between all MCs, most of them having one special function that can be used
    instead of (or in addition to) being included in MC sum terms:
    - `PT4`: also known as CTC (control term clock), routable to all register CLK inputs in this FB
    - `PT5`: also known as CTR (control term reset), routable to all register RST inputs in this FB
    - `PT6`: also known as CTS (control term set), routable to all register SET inputs in this FB
    - `PT7`: also known as CTE (control term enable), routable to all output enables in this FB
    - `PT{8+i*3}`: also known as `MC[i].PTA`, routable to the given MC's register RST and SET inputs
    - `PT{9+i*3}`: also known as `MC[i].PTB`, routable to the given MC's IOB output enable
    - `PT{10+i*3}`: also known as `MC[i].PTC`, routable to the given MC's XOR gate and register CLK and CE inputs
  - 16 MCs (macrocells), each of which has:
    - a sum term, including an arbitrary subset of this FB's PTs
    - a dedicated XOR gate, which can XOR the sum term with `PTC` (or with 0)
    - optional inverter
    - a register, with:
      - D input tied to either the XOR gate output, or this MC's input buffer (sidestepping the ZIA)
      - configurable mode, one of: DFF, TFF, D latch, DFF with clock enable
      - clock (or latch gate), freely invertible and routable from FCLK, PTC, CTC
      - async reset, routable from PTA, FSR, CTR, const 0
      - async set, routable from PTA, FSR, CTS, const 0
      - clock enable, routable from PTC
      - configurable dual-edge mode
      - configurable initial value
    - output to AIM, routed either from the XOR gate (combinatorial) or the register's Q output (registered)
    - IOB (input/output buffer) (on larger devices, not all MCs have an IOB), with:
      - input buffer (routed to AIM and this MC's register D input), configurable in several modes:
        - CMOS Input
        - Schmitt trigger input
        - differential input (with a per-bank voltage reference)
        - used *as* voltage reference for this bank
      - output to AIM, routed either from the input buffer or from the MC's register Q output
      - either combinatorial (from XOR gate) or registered (from register's Q) output to the output buffer, selectable
        independently from AIM output
      - output enable, routable from PTB, FOE, CTE, const 0, const 1
      - some special modes (replacing normal output enable mechanism):
        - programmed ground (always-on const-0 output)
        - open drain output
      - configurable slew rate (fast or slow)
      - termination enable (termination mode is selected globally)
      - (on larger devices) individually enabled data gate latch
- (on larger devices) clock divider
  - if enabled, drives the FCLK2 global net
  - driven by the GCLK2 pad
  - can divide by 2, 4, 6, 8, 10, 12, 14, 16
  - has a reset driven directly by the CDRST pad
  - has optional delay [?]
- global signals
  - 3 FCLK signals, hardwired 1-1 to GCLK pins, except for FCLK2 which can
    optionally go through the clock divider
  - 4 FOE signals, routable from:
    - the corresponding GOE pad, directly
    - the corresponding GOE pad, inverted
    - MC output
  - 1 FSR signal, freely invertible, driven from GSR pads
  - (on larger devices) 1 DGE signal, driven from DGE pad
- per-bank configuration
  - input buffer mode (high voltage or low voltage)
  - output buffer mode (high voltage or low voltage)
- special global configuration bits
  - termination mode
  - global VREF enable
  - global DGE enable
  - 32-bit standard JTAG USERCODE
  - read protection enable
  - DONE bit


## AIM and FB inputs

TODO: write me


## Product terms

TODO: write me


## Sum term, XOR gate

TODO: write me


## Register

TODO: write me


## Macrocell and IOB outputs

TODO: write me


## Input/output buffer

TODO: write me


## Global networks

TODO: write me


## Clock divider

TODO: write me


## Bank configuration

TODO: write me


## Misc configuration

TODO: write me
