.. _virtex2-bram:

Block RAM — Virtex 2, Spartan 3
###############################

There are several closely related BRAM tiles described here:

- the Virtex 2 ``BRAM`` tile, containing:

  - the ``BRAM`` bel, implementing the ``RAMB16_*`` library primitives
  - the ``MULT`` bel, implementing the ``MULT18X18`` and ``MULT18X18S`` library primitives

- the Spartan 3 ``BRAM.S3`` tile, containing:

  - the ``BRAM`` bel, nearly identical to the Virtex 2 one
  - the ``MULT`` bel, identical to the Virtex 2 one

- the Spartan 3E ``BRAM.S3E`` tile, containing:

  - the ``BRAM`` bel, nearly identical to the Spartan 3 one
  - the upgraded ``MULT`` bel, implementing the ``MULT18X18SIO`` primitive

- the Spartan 3A ``BRAM.S3A`` tile, containing:

  - the upgraded ``BRAM`` bel, implementing the ``RAMB16BWE_*`` library primitives in addition to the old ones
  - the ``MULT`` bel, functionally identical to the Spartan 3E one, but with added input multiplexers allowing bypassing ``BRAM`` data output directly to multiplier inputs

- the Spartan 3A DSP ``BRAM.S3ADSP`` tile, containing:

  - the upgraded ``BRAM`` bel, implementing the ``RAMB16BWER`` library primitive
  - no ``MULT`` bel — it has been functionally replaced by the ``DSP`` tile

The ``BRAM`` tile corresponds to four vertically stacked interconnect tiles. They are:

- Virtex 2: four ``INT.BRAM`` tiles
- Spartan 3: four ``INT.BRAM.S3`` tiles
- Spartan 3E: four ``INT.BRAM.S3E`` tiles
- Spartan 3A:

  - row 0 (bottom) and 3 (top): ``INT.BRAM.S3A.03`` tiles
  - rows 1 and 2: ``INT.BRAM.S3A.12`` tiles

- Spartan 3A DSP: four ``INT.BRAM.S3ADSP`` tiles

The ``BRAM`` tile is interconnect-limitted in two ways:

- inputs ``MULT.B0`` through ``MULT.B15`` are shared with ``BRAM.DIB16`` through ``BRAM.DIB31`` (so it is not really possible to use the multiplier when both BRAM ports are used for writing with 36-bit data width, except when the ``BCIN`` cascade is used, or on Spartan 3A when the bypass path is used)
- each interconnect tile is limitted (through ``OMUX`` bottleneck) to 24 out of 28 output wires; the exact rules are very non-obvious, but effectively the following can be supported:

  - ``BRAM`` with full data output (36-bit on both ports), no ``MULT``
  - ``BRAM`` with full data output (36-bit) on one port, up to 18-bit on the other, ``MULT`` with full (36-bit) output
  - ``BRAM`` with 32-bit data output on both ports  (no parity bits), ``MULT`` with 32-bit output

  If the Spartan 3A ``MULT`` bypass path is used, it doesn't count to the above restrictions.

Since the ``MULT`` bel is gone on Spartan 3A DSP, its ``BRAM`` tile is not interconnect-limitted.


The ``BRAM`` bel
================

The ``BRAM`` bel implements a 18-kbit block RAM, consisting of 16-kbit main plane and 2-kbit parity plane. Its operation is mostly described in the Xilinx documentation. However, the pin mapping of library primitives to the hardware bel is not obvious.

The bel has the following input pins, all of which are connected through general interconnect:

- ``CLK[AB]`` (freely invertible through the interconnect): clocks for port A and port B
- ``EN[AB]`` (freely invertible through the interconnect): enable for port A and port B
- (Virtex 2, Spartan 3, Spartan 3E, Spartan 3A) ``SSR[AB]`` (freely invertible through the interconnect): synchronous set/reset for the read ports
- (Spartan 3A DSP) ``RST[AB]`` (freely invertible through the interconnect): set/reset for the read ports (synchronous or not); replaces ``SSR[AB]``
- (Spartan 3A DSP) ``REGCE[AB]`` (freely invertible through the interconnect): pipeline register clock enable for read ports
- (Virtex 2, Spartan 3, Spartan 3E) ``WE[AB]`` (freely invertible through the interconnect): write enable for port A and port B
- (Spartan 3A, Spartan 3A DSP) ``WE[AB][0-3]`` (freely invertible through the interconnect): per-byte write enable for port A and port B
- ``ADDR[AB]0`` through ``ADDR[AB]13``, the address buses for port A and port B
- ``DI[AB]0`` through ``DI[AB]31``: the main data input for port A and port B
- ``DIP[AB]0`` through ``DIP[AB]3``: the parity data input for port A and port B

The bel has the following output pins, all of which are connected through general interconnect:

- ``DO[AB]0`` through ``DO[AB]31``: the main data output for port A and port B
- ``DOP[AB]0`` through ``DOP[AB]3``: the parity data output for port A and port B

The address buses are always in terms of (main plane) bits. When port widths wider than one bit are used, the low bits of the address are unused. When port widths smaller than 36 bits are used, the high bits of the data input/output ports are unused.

On Spartan 3A and 3A DSP, the write enables are per byte. The ``WE[AB][0-3]`` ports should be connected as follows:

- for port width of 9 or smaller, the same signal (the single write enable) should be connected to all 4 bits of the write enable
- for port width of 18:

  - write enable signal 0 (for lower byte) should be connected to both ``WE[AB]0`` and ``WE[AB]2``
  - write enable signal 1 (for upper byte) should be connected to both ``WE[AB]1`` and ``WE[AB]3``

The bel has the following bitstream attributes:

- ``DATA_WIDTH_[AB]`` selects the width of the given port; the values are:

  - ``1``: ``D[IO][AB]0`` is used for data, ``ADDR[AB][0-13]`` is used for address
  - ``2``: ``D[IO][AB][01]`` is used for data, ``ADDR[AB][1-13]`` is used for address
  - ``4``: ``D[IO][AB][0-3]`` is used for data, ``ADDR[AB][2-13]`` is used for address
  - ``9``: ``D[IO][AB][0-7]`` and ``D[IO]P[AB]0`` are used for data, ``ADDR[AB][3-13]`` is used for address
  - ``18``: ``D[IO][AB][0-15]`` and ``D[IO]P[AB][01]`` are used for data, ``ADDR[AB][4-13]`` is used for address
  - ``36``: ``D[IO][AB][0-31]`` and ``D[IO]P[AB][0-3]`` are used for data, ``ADDR[AB][5-13]`` is used for address

- ``WRITE_MODE_[AB]``: selects behavior of the read port when writing, the values are:

  - ``WRITE_FIRST``: data output shows just-written data
  - ``READ_FIRST``: data output shows data previously in memory at the given address
  - ``NO_CHANGE``: the data output is unchanged from previous cycle

- ``SRVAL_[AB]`` (36-bit value): selects the set/reset value of the read port; the low 32 bits correspond to ``DO[AB][0-31]`` while the high 4 bits correspond to ``DOP[AB][0-3]``
- ``INIT_[AB]`` (36-bit value): selects the initial value of the read port; the low 32 bits correspond to ``DO[AB][0-31]`` while the high 4 bits correspond to ``DOP[AB][0-3]``
- (Spartan 3A DSP) ``RSTTYPE`` : selects reset mode, either ``SYNC`` or ``ASYNC``
- (Spartan 3A DSP) ``DO[AB]_REG`` : selects number of pipeline registers for read port

  - ``0``: no pipeline registers, just output latches; same behavior as older FPGAs
  - ``1``: one additional pipeline register on the read port

- ``DATA`` (16384-bit value): the initial data for the BRAM main plane; corresponds to concatenation of ``INIT_xx`` primitive attributes
- ``DATAP`` (2048-bit value): the initial data for the BRAM parity plane; corresponds to concatenation of ``INITP_xx`` primitive attributes
- (Virtex 2) ``SAVEDATA`` (64-bit value): if set, the BRAM data will not be written during partial reconfiguration; the attribute has one bit per frame of the bitstream BRAM data tile
- (Spartan 3 and up) ``WW_VALUE_[AB]``: unknown purpose, must be set to ``NONE``
- (Spartan 3 and up) ``WDEL_A`` (3-bit value): unknown purpose, must be set to device-dependent value from the table below
- (Spartan 3 and up) ``DDEL_A`` (2-bit value): unknown purpose, must be set to device-dependent value from the table below
- (Spartan 3E and up) ``WDEL_B`` (3-bit value): unknown purpose, must be set to device-dependent value from the table below
- (Spartan 3E and up) ``DDEL_B`` (2-bit value): unknown purpose, must be set to device-dependent value from the table below
- (Spartan 3A, Spartan 3A DSP) ``UNK_PRESENT`` (2-bit value): unknown purpose, must be set to all-1

.. todo:: the ``RSTTYPE`` attribute takes two bits in the bitstream, there's a good chance it's actually per-port

.. todo:: ``UNK_PRESENT``

.. todo:: exact semantics of ``SAVEDATA`` are unclear


The ``MULT`` bel — Virtex 2 and Spartan 3
=========================================

The ``MULT`` bel implements a 18×18 signed multiplier with an optional register.
It corresponds to the ``MULT18X18`` and ``MULT18X18S`` library primitives.

The bel has the following general interconnect inputs:

- ``CLK`` (freely invertible via interconnect): the clock
- ``CE`` (freely invertible via interconnect): the clock enable
- ``RST`` (freely invertible via interconnect): the synchronous reset signal (sets the register to all-0)
- ``A[0-17]``: first input
- ``B[0-17]``: second input

The bel has a single general interconnect output:

- ``P[0-35]``: output

The bel has a single attribute:

- ``REG``:

  - if set, the multiplier is in synchronous mode, implementing the ``MULT18X18S`` primitive
  - if unset, the multiplier is in combinational mode, implementing the ``MULT18X18`` primitive; the ``CLK``, ``RST`` and ``CE`` inputs are unused

The semantics of the primitive is pretty simple: it computes ``A * B``, either combinationally or with an output register.


The ``MULT`` bel — Spartan 3E and Spartan 3A
============================================

The ``MULT`` bel implements a 18×18 signed multiplier with optional registers on inputs and output. 
It corresponds to the ``MULT18X18SIO`` library primitive (and can also support the older primitives in an obvious way).

The bel has the following general interconnect inputs:

- ``CLK`` (freely invertible via interconnect): the clock
- ``CE[ABP]`` (freely invertible via interconnect): the clock enables
- ``RST[ABP]`` (freely invertible via interconnect): the synchronous reset signals (sets the corresponding register to all-0)
- ``A[0-17]``: first input
- ``B[0-17]``: second input

On Spartan 3A, the ``A`` and ``B`` inputs can be switched (per-bit) between general interconnect and bypass from the corresponding ``DOA`` or ``DOB`` output from the colocated ``BRAM`` bel.

The bel has a single general interconnect output:

- ``P[0-35]``: output

The bel also has special pins connected via dedicated interconnect:

- ``BCOUT[0-17]``: cascade output; mirrors the ``B`` or ``BCIN`` input (if ``BREG == 0``), or the ``B`` register output (if ``BREG == 1``)
- ``BCIN[0-17]``: cascade input; routed from ``BCOUT`` of the ``MULT`` in the tile immediately below; can be used instead of the ``B`` input to save up on routing resources

When the BRAM column has a hole (for a DCM), the cascade chain jumps over the hole.

The bel has the following attributes:

- ``AREG``: selects the number of pipeline registers on the ``A`` input; either ``0`` or ``1``
- ``BREG``: selects the number of pipeline registers on the ``B`` or ``BCIN`` input; either ``0`` or ``1``
- ``PREG``: selects the number of pipeline registers on the ``P`` output; either ``0`` or ``1``
- ``B_INPUT``: selects the second input to the multiplier

  - ``DIRECT``: uses the general interconnect ``B`` input
  - ``CASCADE``: uses the dedicated ``BCIN`` input

- ``PREG_CLKINVERSION``: if set, the clock to the ``P`` register is inverted from ``CLK``

- (Spartan 3A) ``[AB][0-17]MUX``: selects where the corresponding input pin comes from:

  - ``INT``: general interconnect
  - ``BRAM``: bypass from ``BRAM`` outputs

    - ``[AB][0-15]`` are bypassed from ``DO[AB][0-15]``
    - ``[AB][16-17]`` are bypassed from ``DOP[AB][0-1]``

The semantics of the primitive are as follows:

- ``A_Q = (AREG == 1 ? ff(A, CLK, CEA, RSTA) : A)``
- ``B_MUX = (B_INPUT == "CASCADE" ? BCIN : B)``
- ``B_Q = BCOUT = (BREG == 1 ? ff(B_MUX, CLK, CEB, RSTB) : B_MUX)``
- ``P_D = A_Q * B_Q``
- ``P = (PREG == 1 ? ff(P_D, CLK ^ PREG_CLKINVERSION, CEP, RSTP) : P_D)``
- ``ff(d, clk, ce, rst)`` signifies a flip-flop with synchronous reset and reset-over-CE priority

.. todo:: ``PREG_CLKINVERSION`` is not documented by Xilinx nor exposed via library primitive; test it


Bitstream
=========

The data for a BRAM is spread across 5 bitstream tiles:

- tiles 0-3: the 4 bitstream tiles that are shared with the ``INT.BRAM.*`` interconnect tiles (starting from the bottom)
- tile 4: the dedicated BRAM data tile located in the BRAM data area; this tile is 64×320 bits on Virtex 2 devices and 76×256 bits on Spartan 3 devices; it contains solely the ``DATA`` and ``DATAP`` attributes and, on Virtex 2, the ``SAVEDATA`` attribute


``BRAM`` (Virtex 2)
-------------------

This tile is used on Virtex 2 devices (all kinds).

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-BRAM.html


``BRAM.S3``
-----------

This tile is used on Spartan 3 devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-BRAM.S3.html


``BRAM.S3E``
------------

This tile is used on Spartan 3E devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-BRAM.S3E.html


``BRAM.S3A``
------------

This tile is used on Spartan 3A devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-BRAM.S3A.html


``BRAM.S3ADSP``
---------------

This tile is used on Spartan 3A DSP devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-BRAM.S3ADSP.html


Default option values
=====================

.. raw:: html
   :file: ../gen-xilinx-xc3s-bram-opts.html
