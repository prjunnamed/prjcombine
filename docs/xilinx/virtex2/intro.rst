.. _virtex2:

Introduction
############

Virtex 2 is a family of SRAM-based FPGAs, the successor to :ref:`Virtex <virtex>`.

There are three kinds of Virtex 2 FPGAs:

- Virtex 2, the base version
- Virtex 2 Pro, an upgraded version that adds multi-gigabit transceivers and hard PowerPC 405 cores
- Virtex 2 Pro X, a version of Virtex 2 Pro with faster multi-gigabit transceivers

The Virtex 2 FPGAs feature:

- a new, fully buffered :ref:`general interconnect structure <virtex2-interconnect>`
- a dedicated :ref:`clock interconnect <virtex2-clock>` with 8 global clocks and ``BUFGMUX`` primitives with clock multiplexing
- new :ref:`configurable logic blocks <virtex2-clb>`, derived from the Virtex ones
- new :ref:`block RAM tiles <virtex2-bram>`, containing:

  - 18-kbit block RAM
  - 18×18 multiplier blocks

- new :ref:`input-output tiles <virtex2-io>`, with DDR register support
- new :ref:`digital clock managers <virtex2-dcm>`, a new version of the :ref:`Virtex DLLs <virtex-dll>` with added frequency synthesis capability
- :ref:`corner tiles <virtex2-corner>`, with various global bits of logic:

  - ``BSCAN`` primitive, allowing access to FPGA fabric via dedicated JTAG instructions
  - ``JTAGPPC`` primitive (Virtex 2 Pro), for connecting the PowerPC cores to JTAG
  - ``STARTUP`` primitive, controlling the startup process
  - ``CAPTURE`` primitive, for user-triggered FF state capture
  - ``ICAP`` primitive, allowing access to the internal configuration port
  - ``PMV`` primitive, an internal oscillator used for configuration
  - per-IO bank:

    - DCI control blocks
    - LVDS bias generators

- a tiny bit of :ref:`hard PCI logic <virtex2-pcilogic>`

The Virtex 2 Pro FPGAs additionally feature:

- :ref:`hard PowerPC 405 cores <virtex2-ppc>`
- :ref:`RocketIO multi-gigabit transceivers <virtex2-gt>` (plain Virtex 2 Pro)
- :ref:`RocketIO X multi-gigabit transceivers <virtex2-gt10>` (Virtex 2 Pro X)


Device table
============

.. todo:: generate this