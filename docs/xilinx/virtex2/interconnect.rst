.. _virtex2-interconnect:

General interconnect
####################

The general interconnect of Virtex 2 is made by several kinds of similar, but not identical interconnect tiles. The tile types include:

- ``INT.CLB``, the interconnect tile associated with :ref:`configurable logic blocks <virtex2-clb>`
- ``INT.BRAM``, the interconnect tile associated with :ref:`block RAMs <virtex2-bram>`
- ``INT.{IOI|IOI.CLK_B|IOI.CLK_T}``, the interconnect tile associated with :ref:`I/O tiles <virtex2-io>`
- ``INT.DCM.{V2|V2P}``, the interconnect tiles associated with :ref:`digital clock managers <virtex2-dcm>`
- ``INT.CNR``, the interconnect tile associated with :ref:`corner tiles <virtex2-corner>`
- ``INT.PPC``, the interconnect tile associated with :ref:`PowerPC cores <virtex2-ppc>` and multi-gigabit transceivers
- ``INT.GT.CLKPAD``, the interconnect tile associated with multi-gigabit transceivers in the I/O row

The various tile types have the same backbone, but differ in the types of input multiplexers they have, and the primitive outputs they accept.


Backbone
========

The core of the interconnect is made of the following wires, each of which is instantiated once per interconnect tile and is driven by a buffered multiplexer:

- ``OMUX0`` through ``OMUX15``, per-tile output multiplexers. The inputs to those include various primitive outputs in the tile. They serve as single-hop interconnect wires — each ``OMUX`` wire is also visible in one or two of the (immediately or diagonally) adjacent interconnect tiles:

  ========== ================= =============== ===============
  Wire       Direction         Wire in tile #1 Wire in tile #2
  ========== ================= =============== ===============
  ``OMUX0``  ``S``             ``OMUX0.S``
  ``OMUX1``  ``W``, then ``S`` ``OMUX1.W``     ``OMUX1.WS``
  ``OMUX2``  ``E`` and ``S``   ``OMUX2.E``     ``OMUX2.S``
  ``OMUX3``  ``S``, then ``E`` ``OMUX3.S``     ``OMUX3.SE``
  ``OMUX4``  ``S``             ``OMUX4.S``
  ``OMUX5``  ``S``, then ``W`` ``OMUX5.S``     ``OMUX5.SW``
  ``OMUX6``  ``W``             ``OMUX6.W``
  ``OMUX7``  ``E``, then ``S`` ``OMUX7.E``     ``OMUX7.ES``
  ``OMUX8``  ``E``, then ``N`` ``OMUX8.E``     ``OMUX8.EN``
  ``OMUX9``  ``W``             ``OMUX9.W``
  ``OMUX10`` ``N``, then ``W`` ``OMUX10.N``    ``OMUX10.NW``
  ``OMUX11`` ``N``             ``OMUX11.N``
  ``OMUX12`` ``N``, then ``E`` ``OMUX12.N``    ``OMUX12.NE``
  ``OMUX13`` ``E`` and ``N``   ``OMUX13.E``    ``OMUX13.N``
  ``OMUX14`` ``W``, then ``N`` ``OMUX14.W``    ``OMUX12.WN``
  ``OMUX15`` ``N``             ``OMUX15.N``
  ========== ================= =============== ===============

- Double lines going in the cardinal directions, 10 per direction, called ``DBL.[EWSN][0-9]``. Each of them has three segments, called ``DBL.[EWSN][0-9].[0-2]``, where ``.0`` is located in the source tile and is driven, ``.1`` is in the next tile in the relevant direction, and ``.2`` is in the next tile after that. Some of the lines additionally have a fourth segment:

  - ``DBL.W[89].3`` is to the north of ``DBL.W[89].2``
  - ``DBL.E[01].3`` is to the south of ``DBL.E[01].2``
  - ``DBL.S[01].3`` is to the south of ``DBL.S[01].2``
  - ``DBL.N[89].3`` is to the north of ``DBL.N[89].2``

  Only the ``.0`` segment is driven. The inputs to the multiplexer include:

  - ``OMUX`` wires
  - ``.1``, ``.2`` and ``.3`` segments of other ``DBL`` wires
  - ``.3``, ``.6`` and ``.7`` segments of ``HEX`` wires
  - ``OUT.FAN`` wires

- Hex lines going in the cardinal directions, 10 per direction, called ``HEX.[EWSN][0-9]``. They are analogous to double lines, except they have 7 (or sometimes 8) segments, thus spanning a distance of 6 tiles. The lines with 8th segment include:

  - ``HEX.W[89].7`` is to the north of ``HEX.W[89].6``
  - ``HEX.E[01].7`` is to the south of ``HEX.E[01].6``
  - ``HEX.S[01].7`` is to the south of ``HEX.S[01].6``
  - ``HEX.N[89].7`` is to the north of ``HEX.N[89].6``

  Only the ``.0`` segment is driven. The inputs to the multiplexer include:

  - ``OMUX`` wires
  - ``.3``, ``.6`` and ``.7`` segments of other ``HEX`` wires
  - ``.0``, ``.6``, ``.12``, ``.18`` segments of ``LV`` wires (for vertical lines)
  - ``.0``, ``.6``, ``.12``, ``.18`` segments of ``LH`` wires (for horizontal lines)
  - ``OUT.FAN`` wires

  Some hex lines are associated with various kind of input multiplexers, such that a single hex line can drive all input multiplexers of given type through its ``.0`` to ``.5`` segments:

  - ``HEX.[SN]0`` is associated with ``IMUX.SR*``
  - ``HEX.[SN]1`` is associated with ``IMUX.IOI.TS1*``
  - ``HEX.[SN]3`` is associated with ``IMUX.TI*``, ``IMUX.TS*``, and ``IMUX.IOI.ICLK*``
  - ``HEX.[SN]4`` is associated with ``IMUX.IOI.TS2*``
  - ``HEX.[SN]5`` is associated with ``IMUX.IOI.ICE*``
  - ``HEX.[SN]6`` is associated with ``IMUX.CLK*`` and ``IMUX.DCMCLK*``
  - ``HEX.[SN]8`` is associated with ``IMUX.IOI.TCE*``
  - ``HEX.[SN]9`` is associated with ``IMUX.CE*``

For large-fanout nets or nets that need to span long distances, the interconnect also has long lines that span the whole width or height of the device. There are 24 vertical long lines, ``LV``, per an interconnect column, and 24 horizontal long lines, ``LH``, per an interconnect row. They are visible as ``LV.{0-23}`` and ``LH.{0-23}`` segments in each interconnect tile, in a rotating way: ``LH.0`` in a given tile is visible as ``LH.1`` or ``LH.23`` in the horizontally adjacent tiles.
Only ``.0``, ``.6``, ``.12``, ``.18`` segments of long wires are actually accessible to a tile — the rest are just passing through.

Each interconnect tile can optionally drive the long line segments accessible to it via buffered multiplexers. The inputs to ``LH`` driver multiplexers include:

- ``OMUX`` wires
- ``.1`` segments of ``DBL`` wires

The inputs to ``LV`` driver multiplexers include:

- ``OMUX`` wires
- ``.1`` segments of ``DBL`` wires
- various segments of ``HEX.E1`` and ``HEX.W7`` wires

The every-6-tiles nature of long wires combined with existence of hex wires allows for easy distribution of signals everywhere on the FPGA.


Input multiplexers
==================

Every interconnect tile also contains input multiplexers, which drive the associated primitive inputs. The exact set of available input multiplexers depends on the type of interconnect tile.

The baseline set of input muxes is present in the ``INT.CLB`` tile:

- ``IMUX.CLK[0-3]``: four clock inputs. In CLBs, they correspond to the four ``SLICE``\ s. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - ``GCLK0`` through ``GCLK7``, the :ref:`clock interconnect <virtex2-clock>` global lines
  - various segments of ``DBL`` lines
  - any segment of ``HEX.S6`` and ``HEX.N6`` lines 

  The ``IMUX.CLK`` multiplexers have a programmable inverter.

- ``IMUX.SR[0-3]``: four set/reset inputs. In CLBs, they correspond to the four ``SLICE``\ s. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - various segments of ``DBL`` lines
  - any segment of ``HEX.S0`` and ``HEX.N0`` lines 

  The ``IMUX.SR`` multiplexers have a programmable inverter.

- ``IMUX.CE[0-3]``: four clock enable inputs. In CLBs, they correspond to the four ``SLICE``\ s. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - various segments of ``DBL`` lines
  - any segment of ``HEX.S9`` and ``HEX.N9`` lines 

  The ``IMUX.CE`` multiplexers have a programmable inverter.

- ``IMUX.TI[0-1]``: two tristate buffer data inputs. In CLBs, they correspond to the two ``TBUF``\ s. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - ``OMUX`` wires
  - various segments of ``DBL`` lines
  - any segment of ``HEX.S3`` and ``HEX.N3`` lines 

  The ``IMUX.TI`` multiplexers have a programmable inverter.

- ``IMUX.TS[0-1]``: two tristate buffer enable inputs. In CLBs, they correspond to the two ``TBUF``\ s. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - various segments of ``DBL`` lines
  - any segment of ``HEX.S3`` and ``HEX.N3`` lines

  The ``IMUX.TS`` multiplexers have a programmable inverter.

- ``IMUX.S[0-3].B[XY]``: 8 bypass inputs, specific to CLBs. They are special in that they can be used both as primitive inputs and as extra routing resources to reach other primitive inputs. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - ``OMUX`` wires
  - various segments of ``DBL`` lines
  - other ``IMUX.S[0-3].B[XY]`` wires

  These inputs have a programmable inverter. However, the inverter only affects the ``SLICE`` input — it doesn't affect the value seen when this wire is used as input to another ``IMUX``.

  .. todo:: verify the inverter behavior, just in case

- ``IMUX.S[0-3].[FG][0-3]``: 32 LUT inputs, specific to CLBs. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - ``OMUX`` wires
  - various segments of ``DBL`` lines
  - ``IMUX.S[0-3].B[XY]`` wires
  - ``OUT.FAN`` wires

The ``INT.CNR`` and ``INT.PPC`` tiles have a similar set of input multiplexers, with some differences:

- the ``IMUX.S[0-3].*`` wires are not present
- ``IMUX.G[0-3].FAN[0-1]`` wires replace ``IMUX.S[0-3].B[XY]``. They are similar, but do not include the programmable inverter. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - ``OMUX`` wires
  - various segments of ``DBL`` lines
  - other ``IMUX.G[0-3].FAN[0-1]`` wires

- ``IMUX.G[0-3].DATA[0-7]`` wires replace ``IMUX.S[0-3].[FG][0-3]``. They are multplexed from the same sources as ``IMUX.G[0-3].FAN[0-1]`` wires (thus removing the ``OUT.FAN`` sources present in CLBs).

The ``INT.BRAM`` tile is a variant of ``INT.CNR`` with the following differences:

- ``IMUX.G[0-3].DATA[0-1]`` wires are not present
- ``IMUX.BRAM_ADDR[AB][0-3]`` replace the above. They are used for blockram address inputs. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - ``OMUX`` wires
  - various segments of ``DBL`` lines
  - ``IMUX.G[0-3].FAN[0-1]`` wires
  - ``IMUX.BRAM_ADDR[AB][0-3]`` lines of the interconnect tile 4 tiles to the south or to the north of this tile (with matching ``[0-3]`` index, but any ``[AB]`` letter)

  The extra cascading input allows for distribution of identical addresses across a whole column of BRAMs without using general routing resources.

  The PowerPC cores are special: when a blockram is adjacent to a PowerPC core, the multiplexer inputs that would normally source from the adjacent blockram's ``IMUX.BRAM_ADDR*`` are instead connected to the PowerPC core's primitive outputs that drive OCM addresses. This once again allows routing resource savings.

The ``INT.DCM.*`` and ``INT.GT.CLKPAD`` tiles are a variant of ``INT.CNR`` with the following differences:

- ``IMUX.CE[0-1]``, ``IMUX.CLK[0-3]`` and ``IMUX.TS[0-1]`` are not present
- ``IMUX.DCMCLK[0-3]`` wires replace ``IOMUX.CLK[0-3]``. They are multiplexed from:

  - ``PULLUP``, a dummy always-1 wire
  - ``GCLK0`` through ``GCLK7``
  - ``DCM.CLKPAD[0-7]``, direct inputs from clock I/O pads (see :ref:`clock interconnect <virtex2-clock>`)
  - various segments of ``DBL`` lines
  - any segment of ``HEX.S6`` and ``HEX.N6`` lines 

  The ``IMUX.DCMCLK`` multiplexers have a programmable inverter.

The ``INT.IOI*`` tiles are a variant of ``INT.CNR`` with the following differences:

- ``IMUX.TI[0-1]`` and ``IMUX.TS[0-1]`` are not present
- ``IMUX.G[0-3].DATA[0-4]`` are not present
- new multiplexers are added:

  - ``IMUX.IOI.ICLK[0-3]``: four more clock inputs (used for I/O input clocks, while ``IMUX.CLK`` are used for output clocks). Multiplexed from:

    - ``PULLUP``, a dummy always-1 wire
    - ``GCLK0`` through ``GCLK7``, the :ref:`clock interconnect <virtex2-clock>` global lines
    - various segments of ``DBL`` lines
    - any segment of ``HEX.S3`` and ``HEX.N3`` lines 

  - ``IMUX.IOI.TS[12][0-3]``: tristate inputs. Multiplexed from:

    - ``PULLUP``, a dummy always-1 wire
    - various segments of ``DBL`` lines
    - ``TS1``: any segment of ``HEX.S1`` and ``HEX.N1`` lines 
    - ``TS2``: any segment of ``HEX.S4`` and ``HEX.N4`` lines 
    - ``IMUX.G[0-3].FAN[0-1]`` wires

  - ``IMUX.IOI.[IT]CE[0-3]``: clock enable inputs. Multiplexed from:

    - ``PULLUP``, a dummy always-1 wire
    - various segments of ``DBL`` lines
    - ``ICE``: any segment of ``HEX.S5`` and ``HEX.N5`` lines 
    - ``TCE``: any segment of ``HEX.S8`` and ``HEX.N8`` lines 
    - ``IMUX.G[0-3].FAN[0-1]`` wires


Primitive outputs
=================

Primitive outputs are wires that go from the various primitives into the general interconnect. The set of available primitive outputs depends on the type of the interconnect tile. The ``OUT.FAN*`` outputs can be used as inputs to many interconnect multiplexers, while other outputs can only be routed via ``OMUX`` multiplexers.

The ``INT.CLB`` tile has the following primitive outputs:

- ``OUT.FAN[0-7]``: the main combinational LUT outputs (``X`` and ``Y``); they have access to many more routing resources than other outputs
- ``OUT.SEC[8-23]``: the remaining SLICE outputs (``XQ``, ``YQ``, ``XB``, ``YB``)
- ``OUT.TBUS``: a tap of one of the tristate lines passing through the CLB

Every ``OMUX`` multiplexer can mux from all ``OUT.FAN`` wires, and all but one of the remaining wires (in a rotating manner).

Note that the bandwidth limitation of 16 ``OMUX`` wires per tile means that it is not possible to use all primitive outputs in a CLB simultanously (there is one output too many).

The ``INT.IOI*`` tiles have the following primitive outputs:

- ``OUT.FAN[0-7]``: as above
- ``OUT.SEC[8-23]``: routed to all ``OMUX`` wires

There is no ``OMUX`` bottleneck in these tiles.

The ``INT.CNR`` tile has the following primitive outputs:

- ``OUT.FAN[0-7]``: as above
- ``OUT.HALF[8-17].[01]``: various outputs; ``.0`` outputs are routed only to ``OMUX[0-7]`` while ``.1`` outputs are routed only to ``OMUX[8-15]``, creating a possible bottleneck

The ``OMUX`` bottleneck is worse in this kind of tile. However, it only matters when accessing test primitives (specifically, the ``DCI`` primitives).

The ``INT.BRAM`` tile has the following primitive outputs:

- ``OUT.FAN[0-7]``
- ``OUT.SEC[12-23]``: routed to all ``OMUX`` wires
- ``OUT.HALF[8-11].[01]``: as above

The ``OMUX`` bottleneck in this tile type means it's not possible to instantiate the multiplier together with the blockram with maximum port width. However, this is in practice also prevented by IMUX resource sharing.

The ``INT.DCM*`` tiles have the following primitive outputs:

- ``OUT.SEC[2-13]``: routed to all ``OMUX`` wires
- ``OUT.HALF[14-17].[01]```: as above

``OUT.SEC[2-11]`` correspond to the DCM's clock outputs, which are also routable via dedicated clock routing, and would usually not use the general interconnect.

The ``OMUX`` bottleneck in this tile type means that it's not possible to access all the outputs via general interconnect at once. However, this usually doesn't matter, as the clock outputs are generally used via dedicated clock routing, this doesn't matter in practice.

This tile type contains U-turns: some ``IMUX`` lines can be routed directly to ``OMUX`` for test purposes. They are:

- ``IMUX.SR[0-3]``
- ``IMUX.TI[0-1]``
- ``IMUX.DCMCLK[0-3]``
- ``IMUX.CE[2-3]``
- ``IMUX.G[0-3].DATA[0-7]``


The ``INT.PPC`` and ``INT.GT.CLKPAD`` tiles have the following primitive outputs:

- ``OUT.FAN[0-7]``
- ``OUT.SEC[8-15]``, routed to all ``OMUX`` wires
- ``OUT.TEST[0-15]``, used for test outputs, and routed to 2 ``OMUX`` wires each

The ``OMUX`` bottleneck means that it's not possible to access all outputs at once when the ``OUT.TEST`` outputs are used.

This tile type contains U-turns: some ``IMUX`` lines can be routed directly to ``OMUX`` for test purposes. They are:

- ``IMUX.SR[0-3]``
- ``IMUX.TI[0-1]``
- ``IMUX.TS[0-1]``
- ``IMUX.CLK[0-3]`` (``INT.PPC``)
- ``IMUX.DCMCLK[0-3]`` (``INT.GT.CLKPAD``)
- ``IMUX.CE[0-3]``
- ``IMUX.G[0-3].DATA[0-7]``

Additionally, this tile type has associated ``INTF.*`` interface tiles that contain testing U-turn logic that can rewire ``OUT.FAN*`` and ``OUT.SEC*`` to be mirrors of some ``IMUX`` pins instead of primitive outputs. There are four types of ``INTF.*`` tiles which have identical functionality, but differ in bitstream layout.


Terminators
===========

The edges of the device contain special ``TERM.[EWSN]`` tiles that handle interconnect lines going out-of-bounds:

- ``DBL`` lines get reflected — eg. northbound lines "bounce off" the top edge and become southbound lines
- ``HEX`` lines outgoing from the ``TERM`` tile have special multiplexers, with the following choices per each line:

  - reflection: another, incoming ``HEX`` line is reflected onto this line
  - long line (two different taps): one of the ``LH.*`` or ``LV.*`` segments is driven onto this line
  - (some lines only) ``OUT.PCI[01]``: one of the ``PCILOGIC`` outputs is driven onto this line

The :ref:`PCI logic <virtex2-pcilogic>` primitives effectively live outside of normal interconnect tiles, and use ``TERM.[EW]`` as their interconnect tiles. They reuse ``DBL`` and ``OMUX`` interconnect lines for their inputs, and have special ``OUT.PCI[0-1]`` primitive outputs that can be connected to outgoing ``HEX`` lines in the terminator tiles.


PowerPC holes
=============

The PowerPC cores create holes in the interconnect structure — the area they occupy has no interconnect tiles. Not all interconnect lines can cross the gap across the PPC core. There are four special tile types around the core:

- ``PPC.N``: present on the bottom edge of the core
- ``PPC.S``: present on the top edge of the core
- ``PPC.E``: present on the left edge of the core
- ``PPC.W``: present on the right edge of the core

The long lines pass through the core undisturbed. The ``DBL`` lines are reflected as in the ``TERM`` tiles. The ``OMUX`` lines are likewise reflected. The ``HEX`` lines contain a multiplexer with the following choices per each line:

- passthrough: the line passes through the core and connects to the corresponding line on the other side
- reflection: another, incoming ``HEX`` line is reflected onto this line
- long line (two different taps): one of the ``LH.*`` or ``LV.*`` segments is driven onto this line


Clock spine top and bottom
==========================

The clock spine top and bottom (containing ``BUFGMUX`` primitives) live horizontally in between two normal interconnect tiles. They have their own input multiplexers:

- ``CLK.IMUX.SEL[0-7]``: used for the ``BUFGMUX`` ``S`` input, multiplexed from:

  - ``PULLUP``
  - various ``DBL`` horizontal segments

- ``CLK.IMUX.CLK[0-7]``: used for the ``BUFGMUX`` ``I[0-1]`` inputs when not using dedicated clock interconnect, multiplexed from:

  - ``PULLUP``
  - various ``DBL`` horizontal segments

They also have their own primitive outputs:

- ``CLK.OUT.[0-7]``: special ``BUFGMUX`` primitive outputs

They have eight ``OMUX`` lines, multiplexed from the 8 primitive outputs above. The ``OMUX`` wires are connected to the neighbouring general interconnect tiles as-if they came from the otherwise out-of-bounds tiles above or below them.


Bitstream — interconnect tiles
==============================

The interconnect tiles are 22×80 bits. The space on the left is unused by the interconnect tile, and contains data for whatever primitive is associated with the interconnect tile.


``INT.CLB``
-----------

Used with ``CLB`` tiles.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.CLB.html


``INT.BRAM``
------------

Used with ``BRAM`` tiles.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.BRAM.html


``INT.IOI``
-----------

Used with ``IOI`` tiles.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.IOI.html


``INT.IOI.CLK_B``
-----------------

Used with the ``IOI.CLK_B`` tile (Virtex 2 Pro X special IOI tile used for the dedicated reference clock).

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.IOI.CLK_B.html


``INT.IOI.CLK_T``
-----------------

Used with the ``IOI.CLK_T`` tile (Virtex 2 Pro X special IOI tile used for the dedicated reference clock).


.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.IOI.CLK_T.html


``INT.DCM.V2``
--------------

Used with the Virtex 2 ``DCM.V2`` tile.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.DCM.V2.html


``INT.DCM.V2P``
---------------

Used with the Virtex 2 Pro ``DCM.V2P`` tile.


.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.DCM.V2P.html


``INT.CNR``
-----------

Used with the corner tiles.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.CNR.html


``INT.PPC``
-----------

Used with the ``PPC`` tiles, and also for most of the interconnect for the multi-gigabit transceiver tiles.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.PPC.html


``INT.GT.CLKPAD``
-----------------

Used for the IO row of multi-gigabit transceiver tiles.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-INT.GT.CLKPAD.html


Bitstream — terminator tiles
============================

These tiles are placed at the edges of the device and deal with interconnect lines that go out-of-bounds. The associated bitstream tiles are shared with ``IOBS`` tiles and primitive data for corner tiles.

``TERM.W``
----------

Located at the left edge of every interconnect row, this tile is 4×80 bits.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-TERM.W.html


``TERM.E``
----------

Located at the right edge of every interconnect row, this tile is 4×80 bits.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-TERM.E.html


``TERM.S``
----------

Located at the bottom edge of every interconnect column, this tile is 22×12 bits.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-TERM.S.html


``TERM.N``
----------

Located at the top edge of every interconnect column, this tile is 22×12 bits.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-TERM.N.html


Bitstream — PowerPC hole tiles
==============================

These tiles are located inside the PowerPC holes, and serve a similar function to the terminator tiles.

``PPC.W``
---------

This tile is located on the right of every interconnect row interrupted by the PowerPC hole. It reuses the bitstream tile of the rightmost ``INT.PPC`` tile of that row.

The interconnect signals prefixed with ``0`` refer to signals in the rightmost ``INT.PPC`` tile of the row.  The interconnect signals prefixed with ``1`` refer to signals in the leftmost ``INT.PPC`` tile of the row.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-PPC.W.html


``PPC.E``
---------

This tile is located on the left of every interconnect row interrupted by the PowerPC hole. It reuses the bitstream tile of the leftmost ``INT.PPC`` tile of that row.

The interconnect signals prefixed with ``0`` refer to signals in the leftmost ``INT.PPC`` tile of the row.  The interconnect signals prefixed with ``1`` refer to signals in the rightmost ``INT.PPC`` tile of the row.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-PPC.E.html


``PPC.S``
---------

This tile is located on the top of every interconnect column interrupted by the PowerPC hole. It uses the bitstream tile corresponding to the interconnect tile below the topmost ``INT.PPC`` tile of the column (which is otherwise empty, as it doesn't contain an ``INT.*`` tile).

The interconnect signals prefixed with ``0`` refer to signals in the topmost ``INT.PPC`` tile of the row.  The interconnect signals prefixed with ``1`` refer to signals in the bottommost ``INT.PPC`` tile of the row.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-PPC.S.html


``PPC.N``
---------

This tile is located on the bottom of every interconnect column interrupted by the PowerPC hole. It uses the bitstream tile corresponding to the interconnect tile above the bottommost ``INT.PPC`` tile of the column (which is otherwise empty, as it doesn't contain an ``INT.*`` tile).

The interconnect signals prefixed with ``0`` refer to signals in the bottommost ``INT.PPC`` tile of the row.  The interconnect signals prefixed with ``1`` refer to signals in the topmost ``INT.PPC`` tile of the row.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-PPC.N.html
