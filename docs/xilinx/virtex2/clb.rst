.. _virtex2-clb:

Configurable Logic Block
########################

The main logic resource in Virtex 2 devices is the CLB (Configurable Logic Block). It corresponds one-to-one with the ``INT.CLB`` interconnect tile.  Every CLB has:

- four ``SLICE``\ s, numbered ``SLICE0`` through ``SLICE3``
- four horizontal tristate buses, going horizontally through the whole row of CLBs
- two tristate buffers, ``TBUF0`` and ``TBUF1``, driving the tristate buses

The slices are organized as follows:

- ``SLICE0`` is on the bottom left of the CLB
- ``SLICE1`` is above ``SLICE0``
- ``SLICE2`` is to the right of ``SLICE0``
- ``SLICE3`` is above ``SLICE2`` and to the right of ``SLICE1``

Every slice has:

- two 4-input LUTs, named ``F`` and ``G``

  - each of them has four inputs, named ``F[1-4]`` and ``G[1-4]``
  - every LUT can be used as LUT RAM or shift register

- two "bypass inputs" used for various purposes

  - ``BX``, associated with the ``F`` LUT
  - ``BY``, associated with the ``G`` LUT

- two wide multiplexers

  - ``F5``, associated with the ``F`` LUT, multiplexing ``F`` and ``G``
  - ``FX``, associated with the ``G`` LUT, multiplexing ``F5`` and ``FX`` outputs of this and other ``SLICE``\ s

- carry logic with a carry chain, going vertically upwards through the CLB column

- sum of products logic, going horizontally rightwards through the CLB row

- two main combinational outputs

  - ``X``, associated with the ``F`` LUT
  - ``Y``, associated with the ``G`` LUT

- two "bypass" combinational outputs, used for long shift registers and carry chains

  - ``XB``, associated with the ``F`` LUT
  - ``YB``, associated with the ``G`` LUT

- two registers and their outputs

  - ``FFX`` and ``XQ``, associated with the ``F`` LUT
  - ``FFY`` and ``YQ``, associated with the ``G`` LUT

- shared control inputs:

  - ``CLK``, the clock input
  - ``SR``, the set/reset input (also used as LUT RAM write enable)
  - ``CE``, the clock enable input

In summary, a single ``SLICE`` has the following pins:

- ``F[1-4]`` and ``G[1-4]``: general interconnect inputs, used as LUT inputs and LUT RAM write address
- ``BX`` and ``BY``: general interconnect freely-invertible inputs, used for various purposes
- ``CLK``, ``SR``, ``CE``: general interconnect freely-invertible inputs
- ``X``, ``Y``, ``XQ``, ``YQ``, ``XB``, ``YB``: general interconnect outputs
- ``COUT``: dedicated output (carry output)
- ``CIN``: dedicated input (carry input), routed from ``COUT`` of the slice below
- ``SHIFTOUT``: dedicated output (shift register output)
- ``SHIFTIN``: dedicated input (shift register input), routed from ``SHIFTOUT`` of the previous slice in sequence
- ``SOPOUT``: dedicated output (sum of products output)
- ``SOPIN``: dedicated output (sum of products input), routed from ``SOPOUT`` of the slice to the left
- ``F5`` and ``FX``: dedicated outputs (wide multiplexer outputs)
- ``FXINA`` and ``FXINB``: dedicated inputs (wide multiplexer inputs), routed from ``F5`` and ``FX`` of neighbouring slices
- ``DIG``: dedicated output
- ``ALTDIG``: dedicated input

Additionally, some pins and circuitry are shared between ``SLICE``\ s within the same CLB.


LUTs
====

There are two 4-input LUTs in each slice, ``F`` and ``G``. The ``F`` LUT has inputs ``F[1-4]``, with ``F1`` being the LSB and ``F4`` being the MSB. The ``G`` LUT likewise has inputs ``G[1-4]``.

The initial LUT contents are determined by the ``F`` and ``G`` attributes in the bitstream.

The LUT outputs go to:

- the ``FXMUX`` and ``GYMUX`` multiplexers
- the carry logic
- the ``F5`` wide multiplexer


LUT RAM
-------

The ``F_RAM`` and ``G_RAM`` attributes, when set, turn ``F`` and ``G`` (respectively) into LUT RAM mode.

The signals used in RAM mode are:

- ``CLK`` is the write clock
- ``SR`` is the write enable
- ``WF[1-4]`` and ``WG[1-4]`` are write address for the ``F`` and ``G`` LUTs, respectively
- ``DIF`` and ``DIG`` are the data input for the ``F`` and ``G`` LUTs, respectively
- ``SLICEWE0``: bit 4 of the write address, when enabled
- ``SLICEWE1``: bit 5 of the write address, when enabled
- ``SLICEWE2``: bit 6 of the write address, when enabled

The write address is routed as follows:

- ``SLICE0.W[FG][1-4]`` is routed from ``SLICE0.[FG][1-4]``
- ``SLICE1.W[FG][1-4]`` is routed from ``SLICE1.[FG][1-4]``
- ``SLICE2.W[FG][1-4]`` is routed from ``SLICE0.[FG][1-4]``
- ``SLICE3.W[FG][1-4]`` is routed from ``SLICE1.[FG][1-4]``

Thus, ``SLICE[01]`` can be used alone to implement single-port RAM, or together with ``SLICE[23]`` to implement dual port or larger RAM.

The ``DIF_MUX`` determines the value of ``DIF``:

- ``BX``: use the ``BX`` pin (used for 16×X RAMs)
- ``ALT``: use the ``DIG`` value (used for 32×X and larger RAMs)

The ``DIG_MUX`` determines the value of ``DIG``:

- ``BY``: use the ``BY`` pin
- ``ALT``: use the ``ALTDIG`` value

``ALTDIG`` is determined as follows:

- ``SLICE0.ALTDIG`` is connected to ``SLICE1.DIG``
- ``SLICE1.ALTDIG`` is connected to ``SLICE3.DIG``
- ``SLICE2.ALTDIG`` is connected to ``SLICE3.DIG``
- ``SLICE3.ALTDIG`` is connected to ``SLICE3.DIG`` of the CLB above

Note that ``DI[FG]_MUX`` attributes are also used in the shift register mode, but with different meaning.

The ``SLICEWE0`` signals are routed as follows:

- ``SLICE0.SLICEWE0 = SLICE0.BX``
- ``SLICE1.SLICEWE0 = SLICE1.BX``
- ``SLICE2.SLICEWE0 = SLICE0.BX``
- ``SLICE3.SLICEWE0 = SLICE1.BX``

When ``SLICEWE0USED`` is set, the ``SLICEWE0`` signal is used within the slice. The ``F`` LUT is written when it is 1, the ``G`` LUT is written when it is 0. Otherwise, the signal is ignored, and both LUTs are written at the same time.

The ``SLICEWE1`` and ``SLICEWE2`` signals are routed as follows:

- ``SLICE0.SLICEWE1 = SLICE0.BY``
- ``SLICE1.SLICEWE1 = !SLICE0.BY``
- ``SLICE2.SLICEWE1 = SLICE0.BY``
- ``SLICE3.SLICEWE1 = !SLICE0.BY``
- ``SLICE0.SLICEWE2 = SLICE1.BY``
- ``SLICE1.SLICEWE2 = SLICE1.BY``
- ``SLICE2.SLICEWE2 = !SLICE1.BY``
- ``SLICE3.SLICEWE2 = !SLICE1.BY``

If ``SLICE0.BYOUTUSED`` is set, all ``SLICE``\ s within the CLB will use their ``SLICEWE1`` signal as a write enable — the LUTs are only written when ``SLICEWE1`` is 1. Otherwise, all ``SLICEWE1`` signals are ignored.

If ``SLICE1.BYOUTUSED`` is set, all ``SLICE``\ s within the CLB will use their ``SLICEWE2`` signal as a write enable — the LUTs are only written when ``SLICEWE2`` is 1. Otherwise, all ``SLICEWE2`` signals are ignored.

.. todo:: ``SLICE2`` and ``SLICE3`` also have ``BYOUTUSED`` bits — what do they do, if anything?


Single-port 16×X RAM
++++++++++++++++++++

Single-port 16×X RAM can be implemented as follows:

- pick a slice

  - ``SLICE0`` and ``SLICE1`` can always be used
  - ``SLICE2`` can be used if ``SLICE0`` is also used with the same address
  - ``SLICE3`` can be used if ``SLICE1`` is also used with the same address

- connect ``CLK`` to write clock
- connect ``SR`` to write enable
- for the 16×1 slice in ``F`` LUT:

  - connect ``F[1-4]`` to the read/write address
  - connect ``BX`` to write data
  - set ``DIF_MUX`` to ``BX``
  - use ``F`` output as read data

- for the 16×1 slice in ``G`` LUT:

  - connect ``G[1-4]`` to the read/write address
  - connect ``BY`` to write data
  - set ``DIG_MUX`` to ``BY``
  - use ``G`` output as read data


Dual-port 16×X RAM
++++++++++++++++++

Dual-port 16×X RAM can be implemented as follows:

- pick a pair of slices: either ``SLICE0`` and ``SLICE2`` or ``SLICE1`` and ``SLICE3``
- connect ``CLK`` to write clock
- connect ``SR`` to write enable
- for the 16×1 slice in ``F`` LUTs:

  - connect ``F[1-4]`` on ``SLICE[01]`` to the write address
  - connect ``F[1-4]`` on ``SLICE[23]`` to the read address
  - connect ``BX`` of both slices to write data
  - set ``DIF_MUX`` to ``BX``
  - use ``F`` outputs as read data

- for the 16×1 slice in ``G`` LUTs:

  - connect ``G[1-4]`` on ``SLICE[01]`` to the write address
  - connect ``G[1-4]`` on ``SLICE[23]`` to the read address
  - connect ``BY`` of both slices to write data
  - set ``DIG_MUX`` to ``BY``
  - use ``G`` outputs as read data


Single-port 32×X RAM
++++++++++++++++++++

Single-port 32×X RAM can be implemented as follows:

- pick a slice

  - ``SLICE0`` and ``SLICE1`` can always be used
  - ``SLICE2`` can be used if ``SLICE0`` is also used with the same address
  - ``SLICE3`` can be used if ``SLICE1`` is also used with the same address

- connect ``CLK`` to write clock
- connect ``SR`` to write enable
- ``F`` LUT corresponds to addresses ``0x1X``
- ``G`` LUT corresponds to addresses ``0x0X``
- connect ``F[1-4]`` and ``G[1-4]`` to low 4 bits of the read/write address
- connect ``BX`` to bit 4 of read/write address
- set ``SLICEWE0USED``
- connect ``BY`` to write data
- set ``DIF_MUX`` to ``ALT``
- set ``DIG_MUX`` to ``BY``
- use ``F5`` output as read data


Dual-port 32×X RAM
++++++++++++++++++

Dual-port 32×X RAM can be implemented as follows:

- pick a pair of slices: either ``SLICE0+SLICE2`` or ``SLICE1+SLICE3``
- connect ``CLK`` to write clock
- connect ``SR`` to write enable
- ``F`` LUTs correspond to addresses ``0x1X``
- ``G`` LUTs correspond to addresses ``0x0X``
- connect ``F[1-4]`` and ``G[1-4]`` of ``SLICE[01]`` to low 4 bits of the write address
- connect ``F[1-4]`` and ``G[1-4]`` of ``SLICE[23]`` to low 4 bits of the read address
- connect ``SLICE[01].BX`` to bit 4 of write address
- connect ``SLICE[23].BX`` to bit 4 of read address
- set ``SLICEWE0USED``
- connect ``BY`` of both slices to write data
- set ``DIF_MUX`` to ``ALT``
- set ``DIG_MUX`` to ``BY``
- use ``F5`` outputs as read data


Single-port 64×X RAM
++++++++++++++++++++

- pick a pair of slices

  - ``SLICE0+SLICE1`` can always be used
  - ``SLICE2+SLICE3`` can also be used if ``SLICE0+SLICE1`` is used with the same address

- connect ``CLK`` to write clock
- connect ``SR`` to write enable
- ``SLICE[13].G`` corresponds to addresses ``0x0X``
- ``SLICE[13].F`` corresponds to addresses ``0x1X``
- ``SLICE[02].G`` corresponds to addresses ``0x2X``
- ``SLICE[02].F`` corresponds to addresses ``0x3X``
- connect ``F[1-4]`` and ``G[1-4]`` to low 4 bits of the address
- connect ``BX`` to bit 4 of the address
- set ``SLICEWE0USED``
- connect ``SLICE[02].BY`` to bit 5 of the address
- set ``SLICE0.BYOUTUSED``
- connect ``SLICE[13].BY`` to write data
- set ``DIF_MUX`` to ``ALT``
- set ``SLICE[02].DIG_MUX`` to ``ALT``
- set ``SLICE[13].DIG_MUX`` to ``BY``
- use ``SLICE[02].FX`` as read data


Dual-port 64×1 RAM
++++++++++++++++++

- use the whole CLB
- connect ``CLK`` to write clock
- connect ``SR`` to write enable
- ``SLICE[13].G`` corresponds to addresses ``0x0X``
- ``SLICE[13].F`` corresponds to addresses ``0x1X``
- ``SLICE[02].G`` corresponds to addresses ``0x2X``
- ``SLICE[02].F`` corresponds to addresses ``0x3X``
- connect ``SLICE[01].F[1-4]`` and ``SLICE[01].G[1-4]`` to low 4 bits of the write address
- connect ``SLICE[23].F[1-4]`` and ``SLICE[23].G[1-4]`` to low 4 bits of the read address
- connect ``SLICE[01].BX`` to bit 4 of the write address
- connect ``SLICE[23].BX`` to bit 4 of the read address
- set ``SLICEWE0USED``
- connect ``SLICE0.BY`` to bit 5 of the write address
- connect ``SLICE2.BY`` to bit 5 of the read address
- set ``SLICE0.BYOUTUSED``
- connect ``SLICE[13].BY`` to write data
- set ``DIF_MUX`` to ``ALT``
- set ``SLICE[02].DIG_MUX`` to ``ALT``
- set ``SLICE[13].DIG_MUX`` to ``BY``
- use ``SLICE[02].FX`` as read data


Single-port 128×1 RAM
+++++++++++++++++++++

- use the whole CLB
- connect ``CLK`` to write clock
- connect ``SR`` to write enable
- ``SLICE3.G`` corresponds to addresses ``0x0X``
- ``SLICE3.F`` corresponds to addresses ``0x1X``
- ``SLICE2.G`` corresponds to addresses ``0x2X``
- ``SLICE2.F`` corresponds to addresses ``0x3X``
- ``SLICE1.G`` corresponds to addresses ``0x4X``
- ``SLICE1.F`` corresponds to addresses ``0x5X``
- ``SLICE0.G`` corresponds to addresses ``0x6X``
- ``SLICE0.F`` corresponds to addresses ``0x7X``
- connect ``F[1-4]`` and ``G[1-4]`` to low 4 bits of the address
- connect ``BX`` to bit 4 of the address
- set ``SLICEWE0USED``
- connect ``SLICE[02].BY`` to bit 5 of the address
- set ``SLICE0.BYOUTUSED``
- connect ``SLICE1.BY`` to bit 6 of the address
- set ``SLICE1.BYOUTUSED``
- connect ``SLICE3.BY`` to write data
- set ``DIF_MUX`` to ``ALT``
- set ``SLICE[012].DIG_MUX`` to ``ALT``
- set ``SLICE3.DIG_MUX`` to ``BY``
- use ``SLICE1.FX`` as read data


Shift registers
---------------

The ``F_SHIFT`` and ``G_SHIFT`` attributes, when set, turn ``F`` and ``G`` (respectively) into shift register mode.

The signals used in shift register mode are:

- ``CLK`` is the write clock
- ``SR`` is the write enable
- ``DIF`` and ``DIG`` are the data input for the ``F`` and ``G`` LUTs, respectively

The LUTs in shift register mode have shift-out outputs, ``FMC15`` and ``GMC15``, which are the next bit to be shifted out. They can be connected to another LUT's data input to assemble larger shift registers.

The ``DIF_MUX`` determines the value of ``DIF``:

- ``BX``: use the ``BX`` pin
- ``ALT``: use the ``GMC15`` value

The ``DIG_MUX`` determines the value of ``DIG``:

- ``BY``: use the ``BY`` pin
- ``ALT``: use the ``SHIFTIN`` pin

``SHIFTIN`` is routed as follows:

- ``SLICE0.SHIFTIN = SLICE1.SHIFTOUT = SLICE1.FMC15`` 
- ``SLICE1.SHIFTIN = SLICE2.SHIFTOUT = SLICE2.FMC15`` 
- ``SLICE2.SHIFTIN = SLICE3.SHIFTOUT = SLICE3.FMC15`` 

``SLICE3.SHIFTIN`` is indeterminate.

Note that ``DI[FG]_MUX`` attributes are also used in the LUT RAM mode, but with different meaning.

The external write data is written to bit 0 of the LUT. Bit 15 is shifted out.

.. todo:: do LUT RAM and shift register modes interfere within a ``SLICE``?


Wide multiplexers
=================

Every ``SLICE`` has two wide multiplexers: ``F5`` and ``FX``, used to combine smaller LUTs into larger LUTs. Their function is hardwired:

- ``F5 = BX ? F : G``
- ``FX = BY ? FXINA : FXINB``

The ``F5`` output goes to the ``FXMUX`` multiplexer, and further wide multiplexers. The ``FX`` output goes to the ``GYMUX`` multiplexer, and further wide multiplexers.

The ``FXINA`` and ``FXINB`` inputs are routed as follows:

========== ============= ============================= ===================
``SLICE``  ``FXINA``     ``FXINB``                     effective primitive
========== ============= ============================= ===================
``SLICE0`` ``SLICE0.F5`` ``SLICE1.F5``                 ``MUXF6``
``SLICE1`` ``SLICE0.FX`` ``SLICE2.FX``                 ``MUXF7``
``SLICE2`` ``SLICE2.F5`` ``SLICE3.F5``                 ``MUXF6``
``SLICE3`` ``SLICE1.FX`` ``SLICE1.FX``, from CLB above ``MUXF8``
========== ============= ============================= ===================

The ``FX`` output isn't connected across the PowerPC hole — a ``MUXF8`` cannot be made of two CLBs separated by a PowerPC.


Carry logic
===========

The carry logic implements the ``MUXCY`` and ``XORCY`` primitives described in Xilinx documentation. There are several bitstream attributes controlling carry logic operation.

The ``CYINIT`` mux determines the start of the carry chain in the slice:

- ``CIN``: connected from ``COUT`` of the ``SLICE`` below
- ``BX``

The ``CYSELF`` mux determines the "propagate" (or select) input of the lower ``MUXCY``:

- ``F``: propagate is connected to ``F`` LUT output
- ``1``: propagate is connected to const-1 (ie. the ``MUXCY`` is effectively skipped from the chain)

The ``CY0F`` mux determines the "generate" input of the lower ``MUXCY``:

- ``0`` (constant)
- ``1`` (constant)
- ``F1``
- ``F2``
- ``BX``
- ``PROD``: equal to ``F1 & F2``, implementing the ``MULT_AND`` primitive

The ``CYSELG`` mux determines the "propagate" (or select) input of the upper ``MUXCY``:

- ``G``: propagate is connected to ``G`` LUT output
- ``1``: propagate is connected to const-1 (ie. the ``MUXCY`` is effectively skipped from the chain)

The ``CY0G`` mux determines the "generate" input of the upper ``MUXCY``:

- ``0`` (constant)
- ``1`` (constant)
- ``G1``
- ``G2``
- ``BY``
- ``PROD``: equal to ``G1 & G2``, implementing the ``MULT_AND`` primitive

The hardwired logic implemented is:

- ``FCY = CYSELF ? CY0F : CIN`` (lower ``MUXCY``)
- ``COUT = GCY = CYSELG ? CY0G : FCY`` (upper ``MUXCY``)
- ``FXOR = F ^ CIN`` (lower ``XORCY``)
- ``GXOR = G ^ FCY`` (upper ``XORCY``)

The dedicated ``CIN`` input is routed from:

- ``SLICE0.CIN``: from ``SLICE1.COUT`` of CLB below
- ``SLICE1.CIN``: from ``SLICE0.COUT``
- ``SLICE2.CIN``: from ``SLICE3.COUT`` of CLB below
- ``SLICE3.CIN``: from ``SLICE2.COUT``

The carry chains are not connected over PowerPC holes. The ``SLICE[02].CIN`` inputs in the row above bottom IOI and in tiles directly above PowerPC are indeterminate.


Sum of products
===============

The carry logic can be used to implement fast wide AND gates (ie. products). Each ``SLICE`` also contains a dedicated ``ORCY`` primitive that allows combining multiple carry chains into a sum-of-products function.

The ``SOPEXTSEL`` mux determines the starting point of the ``ORCY`` chain:

- ``0``: const 0 (this is the first ``SLICE`` in the chain)
- ``SOPIN`` (this is not the first ``SLICE``)

The dedicated ``ORCY`` primitive implements simple hardwired logic:

- ``SOPOUT = SOPEXTSEL | COUT``

The ``SOPIN`` pin is routed as follows:

- ``SLICE0.SOPIN``: ``SLICE2.SOPOUT`` of the CLB to the left
- ``SLICE1.SOPIN``: ``SLICE3.SOPOUT`` of the CLB to the left
- ``SLICE2.SOPIN``: ``SLICE0.SOPOUT``
- ``SLICE3.SOPIN``: ``SLICE1.SOPOUT``

The ``SOPOUT`` chain is connected across BRAM columns, but is not connected over PowerPC holes.


Output multiplexers
===================

The ``FXMUX`` multiplexer controls the ``X`` output. It has three inputs:

- ``F`` (the LUT output)
- ``F5``
- ``FXOR``

The ``GYMUX`` multiplexer controls the ``Y`` output. It has four inputs:

- ``G`` (the LUT output)
- ``FX``
- ``GXOR``
- ``SOPOUT``

The ``XBMUX`` multiplexer controls the ``XB`` output. It has two inputs:

- ``FCY``
- ``FMC15``: shift register output of ``F``

The ``YBMUX`` multiplexer controls the ``YB`` output. It has two inputs:

- ``GCY`` (equal to ``COUT``)
- ``GMC15``: shift register output of ``G``


The ``DXMUX`` mulitplexer controls the ``FFX`` data input. It has two inputs:

- ``X`` (the ``FXMUX`` output)
- ``BX``

The ``DYMUX`` mulitplexer controls the ``FFY`` data input. It has two inputs:

- ``Y`` (the ``GYMUX`` output)
- ``BY``


Registers
=========

A ``SLICE`` contains two registers:

- ``FFX``, with input determined by ``DXMUX`` and output connected to ``XQ``
- ``FFY``, with input determined by ``DYMUX`` and output connected to ``YQ``

Both registers share the same control signals:

- ``CLK``: posedge-triggered clock in FF mode or **active-low** gate in latch mode
- ``CE``: active-high clock or gate enable
- ``SR``: if ``FF_SR_EN``, the set/reset signal
- ``BY``: if ``FF_REV_EN``, the alternate set/reset signal

The following attributes determine register function:

- ``FF_LATCH``: if set, the registers are latches and ``CLK`` behaves as **active-low** gate; otherwise, the registers are flip-flops and ``CLK`` is a posedge-triggered clock
- ``FF_SYNC``: if set, the ``SR`` and ``BY`` (if enabled) implement synchronous set/reset (with priority over ``CE``); otherwise, they implement asynchronous set/reset; should not be set together with ``FF_LATCH``
- ``FF[XY]_INIT``: determines the initial or captured value of given register

  - when the global ``GSR`` signal is pulsed (for example, as part of the configuration process), the register is set to the value of this bit
  - when the global ``GCAP`` signal is pulsed (for example, by the ``CAPTURE`` primitive), this bit captures the current state of the register

- ``FF[XY]_SRVAL``: determines the set/reset value of given register
- ``FF_SR_EN``: if set, ``SR`` is used as the set/reset signal for both registers, setting them to their ``FF[XY]_SRVAL``
- ``FF_REV_EN``: if set, ``BY`` behaves as secondary set/reset signal for both registers, setting them to the **opposite** of their ``FF[XY]_SRVAL``


Tristate buses and ``TBUF``\ s
==============================

.. todo:: document this insanity


Bitstream
=========

The data for a CLB is located in the same bitstream tile as the associated ``INT.CLB`` tile.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-CLB.html
