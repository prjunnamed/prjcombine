# Virtex 2

Virtex 2 is a family of SRAM-based FPGAs, the successor to [Virtex](../virtex/README.md).

There are three kinds of Virtex 2 FPGAs:

- Virtex 2, the base version
- Virtex 2 Pro, an upgraded version that adds multi-gigabit transceivers and hard PowerPC 405 cores
- Virtex 2 Pro X, a version of Virtex 2 Pro with faster multi-gigabit transceivers

The Virtex 2 FPGAs feature:

- a new, fully buffered [general interconnect structure](interconnect/README.md)
- a dedicated [clock interconnect](clock/README.md) with 8 global clocks and `BUFGMUX` primitives with clock multiplexing
- new [configurable logic blocks](clb.md), derived from the Virtex ones
- new [block RAM tiles](bram.md), containing:
  - 18-kbit block RAM
  - 18×18 multiplier blocks
- new [input-output tiles](io/README.md), with DDR register and DCI (digitally controlled impedance) support
- new [digital clock managers](dcm.md), a new version of the [Virtex DLLs](../virtex/dll.md) with added frequency synthesis capability
- [corner tiles](corner.md), with various global bits of logic:
  - `BSCAN` primitive, allowing access to FPGA fabric via dedicated JTAG instructions
  - `JTAGPPC` primitive (Virtex 2 Pro), for connecting the PowerPC cores to JTAG
  - `STARTUP` primitive, controlling the startup process
  - `CAPTURE` primitive, for user-triggered FF state capture
  - `ICAP` primitive, allowing access to the internal configuration port
  - `PMV` primitive, an internal oscillator used for configuration
  - per-IO bank:
    - DCI control blocks
    - LVDS bias generators
- a tiny bit of [hard PCI logic](pcilogic.md)

The Virtex 2 Pro FPGAs additionally feature:

- [hard PowerPC 405 cores](ppc.md)
- [RocketIO multi-gigabit transceivers](gt.md) (plain Virtex 2 Pro)
- [RocketIO X multi-gigabit transceivers](gt10.md) (Virtex 2 Pro X)


## Device table

TODO: generate this