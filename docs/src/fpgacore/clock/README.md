# Clock interconnect

The FPGAcore clock interconnect is a simplified version of the [Spartan 3 clock interconnect](../../spartan3/clock/README.md), with the following differences:

- the `BUFGMUX` primitives are replaced with `BUFG` primitives with no clock multiplexer support
- there are no `DCM`s