# Spartan 3

Spartan 3 is a family of SRAM-based FPGAs, based on a cut-down version of [Virtex 2](../virtex2/README.md).

There are several kinds of Spartan 3 FPGAs:

- Spartan 3, the base version
- Spartan 3E, an updated version of Spartan 3, with higher logic-to-IO ratio
- Spartan 3A, an updated version of Spartan 3E
- Spartan 3AN, non-volatile version of Spartan 3A; it is literally Spartan 3A and stock SPI flash in a trenchcoat, with the FPGA die being exactly identical to Spartan 3A
- Spartan 3A DSP, an updated version of Spartan 3A with added hard DSP blocks

The base Spartan 3 FPGAs feature:

- a [general interconnect structure](interconnect/README.md) derived from Virtex 2
- a dedicated [clock interconnect](clock/README.md) with 8 global clocks and ``BUFGMUX`` primitives with clock multiplexing
- [configurable logic blocks](clb.md), derived from the Virtex 2 ones
- [block RAM tiles](bram.md), essentially identical to Virtex 2, containing:
  - 18-kbit block RAM
  - 18×18 multiplier blocks
- [input-output tiles](io/README.md), similar to Virtex 2
- [digital clock managers](dcm-s3.md), essentially identical to Virtex 2
- [corner tiles](corners/README.md), with various global bits of logic:
  - ``BSCAN`` primitive, allowing access to FPGA fabric via dedicated JTAG instructions
  - ``STARTUP`` primitive, controlling the startup process
  - ``CAPTURE`` primitive, for user-triggered FF state capture
  - ``ICAP`` primitive, allowing access to the internal configuration port
  - ``PMV`` primitive, an internal oscillator used for configuration
  - per-IO bank:
    - DCI control blocks
    - LVDS bias generators

Spartan 3E brings the following changes:

- improved clock interconnect, with 24 global clock buffers that can be multiplexed to 8 clocks per region
- improved hard multiplier blocks, with pipeline registers
- [a new version of DCM](dcm-s3e.md)
- IO tile changes
  - new set of IO standards supported
  - improved DDR registers
  - removed DCI support
  - 4 banks per device, instead of 8
- a new bit of [hard PCI logic](pcilogicse.md)
- support for SPI and BPI configuration modes

Spartan 3A brings the following changes:

- improved block RAM, with per-byte write enables
- IO tile changes
  - new set of IO standards supported
  - improved DDR registers
  - the IO banks are now specialized (top and bottom banks have differential termination support, left and right banks have higher drive strength)
- more singleton special primitives in the corners:
  - ``DNA_PORT`` allows access to unique per-device identifier
  - ``SPI_ACCESS`` allows access to the SPI flash included in-package on Spartan 3AN devices

Spartan 3A DSP brings the following changes:

- improved block RAM, with pipeline registers and asynchronous reset
- the hard multiplier blocks are removed and replaced with [a new DSP block](dsp.md)


## Device table

TODO: generate this
