.. _virtex2-geometry:

Device geometry
###############

General structure
=================

The Virtex 2 devices are made of a rectangular grid of interconnect tiles.

Interconnect rows come in three kinds:

- bottom IOI row (the bottommost row)
- top IOI row (the topmost row)
- general row (all other rows)

Interconnect columns come in four kinds:

- left IOI column (the leftmost column)

  - contains ``INT.CNR`` and the ``LL`` (lower left) corner tile in the bottom IOI row
  - contains ``INT.CNR`` and the ``UL`` (upper left) corner tile in the top IOI row
  - contains ``INT.IOI`` and an ``IOI`` tile in remaining rows

- right IOI column (the rightmost column)

  - contains ``INT.CNR`` and the ``LR`` (lower right) corner tile in the bottom IOI row
  - contains ``INT.CNR`` and the ``UR`` (upper right) corner tile in the top IOI row
  - contains ``INT.IOI`` and an ``IOI`` tile in remaining rows

- CLB columns (most of the inner columns)

  - contains ``INT.IOI`` and an ``IOI`` IO tile in the bottom and top IOI rows
  - contains ``INT.CLB`` and a ``CLB`` tile in the remaining rows (except for PowerPC holes)

- BRAM columns (some of the inner columns), which further come in three kinds:

  - plain BRAM column

    - contains ``INT.DCM.*`` and a ``DCM`` tile in the bottom and top IOI rows
    - contains ``INT.BRAM`` in the remaining rows, and a ``BRAM`` tile every 4 rows (except for PowerPC holes)

  - ``GT`` column (Virtex 2 Pro)

    - contains ``INT.GT.CLKPAD`` in the bottom and top IOI rows
    - contains ``INT.PPC`` in the 4 rows above bottom IOI row and the 4 rows below top IOI row
    - contains a ``GIGABIT.B`` tile in the bottom and a ``GIGABIT.T`` tile at the top
    - contains ``INT.BRAM`` in the remaining rows, and a ``BRAM`` tile every 4 rows (except for PowerPC holes)

  - ``GT10`` column (Virtex 2 Pro X)

    - contains ``INT.GT.CLKPAD`` in the bottom and top IOI rows
    - contains ``INT.PPC`` in the 8 rows above bottom IOI row and the 8 rows below top IOI row
    - contains a ``GIGABIT10.B`` tile in the bottom and a ``GIGABIT10.T`` tile at the top
    - contains ``INT.BRAM`` in the remaining rows, and a ``BRAM`` tile every 4 rows (except for PowerPC holes)

The bottom and top of every interconnect column has ``TERM.S`` and ``TERM.N`` tiles, respectively. Likewise, the left edge and right edge of every interconnect row has ``TERM.W`` and ``TERM.E`` tiles.

PowerPC holes
-------------

Virtex 2 Pro and Virtex 2 Pro X devices have hard PowerPC cores, which create a hole in the normal pattern.  The hole is 16 rows high and 10 columns across. The 10 columns involved are always the following, in order:

- CLB column
- BRAM column
- 6 CLB columns
- BRAM column
- CLB column

The leftmost and rightmost column of the hole, as well as topmost and bottommost row of the hole contain ``INT.PPC`` interconnect tiles. The middle 8 columns of the hole and middle 14 rows of the hole have no interconnect tiles.

Right above each bottom ``INT.PPC`` tile in the 8 middle columns, there is a ``PPC.N`` tile, and below each top ``INT.PPC`` tile is a ``PPC.S`` tile.

Likewise, each of the 14 middle rows has a ``PPC.E`` tile to the right of the left ``INT.PPC`` tile and a ``PPC.W`` tile to the left of the right ``INT.PPC`` tile.


Clock rows
==========

The device has some amount of clock rows. They exist in between interconnect rows, and are not counted in the coordinate system. Each clock row provides dedicated clock routing to some range of interconnect rows.

On the intersection of every clock row and interconnect column there is a ``GCLKH`` tile, which distributes the clocks vertically to some segment of interconnect tiles.

The horizontal clock lines in the clock row are driven from ``GCLKC.*`` tiles at the intersections with the clock spine.


Clock spine
===========

The device also has a special column, called the clock spine. It is located in between two interconnect columns, somewhere in the middle of the device. It is not counted in the coordinate system.

The clock spine has special tiles dealing with clock interconnect:

- the bottom IOI row has the ``CLKB.*`` tile, with 8 ``BUFGMUX`` primitives
- the top IOI row has the ``CLKT.*`` tile, with 8 ``BUFGMUX`` primitives
- the middle row has the ``CLKC`` tile, which buffers the vertical clock lines
- each clock row has a ``GCLKC.*`` tile, which multiplexes the vertical clock lines onto the horizontal clock lines


IO banks and IO buffers
=======================

The devices have 8 IO banks, numbered 0 through 7 in clockwise fashion:

- 0: top IO row, to the left of clock spine
- 1: top IO row, to the right of clock spine
- 2: right IO column, top half
- 3: right IO column, bottom half
- 4: bottom IO row, to the right of clock spine
- 5: bottom IO row, to the left of clock spine
- 6: left IO column, bottom half
- 7: left IO column, top half

The edges of the device have ``IOI`` tiles, each of which contains 4 ``IOI`` (IO interface) primitives that implement IO registers. However, not all ``IOI`` primitives have an associated IO buffer.

The IO buffers live in special ``IOBS.*`` tiles, contained in:

- top IOB row, above the top IOI row
- bottom IOB row, below the bottom IOI row
- left IOB column, to the left of the left IOI column
- right IOB column, to the right of the right IOI column

The ``IOBS.*`` tiles come in multiple variants, and can span one or two ``IOI`` tiles.

.. todo:: more details


PCI logic
=========

A Virtex 2 device contains two instances of ``PCILOGIC``. They are located somewhere near the midpoint of each IOI column.


Bitstream geometry
==================

The bitstream is made of frames, which come in three types:

- 0: main area
- 1: BRAM data area
- 2: BRAM interconnect area

Frames are identified by their type, major and minor numbers. The major number identifies a column (interconnect column, clock spine, or IOB column), and the minor number identifies a frame within a column. The major numbers are counted separately for each type of frame.

The main area contains the following columns, in order (with major numbers assigned sequentially from 0, one for each column):

- the clock spine (4 frames)
- the left IOB column (4 frames)
- the left IOI column (22 frames)
- the CLB columns, in order (22 frames each)
- the right IOI column (22 frames)
- the right IOB column (4 frames)

The BRAM data area contains 64 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

The BRAM interconnect area contains 22 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

For example, ``xc2v40`` has the following frames (``type.major.minor``):

- ``0.0.0-3``: clock spine (between interconnect X 5 and 6)
- ``0.1.0-3``: left IOB column
- ``0.2.0-21``: left IOI column (interconnect X == 0)
- ``0.3.0-21``: CLB column 1 (interconnect X == 1)
- ``0.4.0-21``: CLB column 2 (interconnect X == 2)
- ``0.5.0-21``: CLB column 3 (interconnect X == 4)
- ``0.6.0-21``: CLB column 4 (interconnect X == 5)
- ``0.7.0-21``: CLB column 5 (interconnect X == 6)
- ``0.8.0-21``: CLB column 6 (interconnect X == 7)
- ``0.9.0-21``: CLB column 7 (interconnect X == 9)
- ``0.10.0-21``: CLB column 8 (interconnect X == 10)
- ``0.11.0-21``: right IOI column (interconnect X == 11)
- ``0.12.0-3``: right IOB column
- ``1.0.0-63``: BRAM column 1 data
- ``1.1.0-63``: BRAM column 2 data
- ``2.0.0-63``: BRAM column 1 interconnect (interconnect X == 3)
- ``2.1.0-63``: BRAM column 2 interconnect (interconnect X == 8)

Each bitstream frame within a device has the same size. The size is ``32 + num_interconnect_rows * 80`` bits. The bits are, in order:

- 4 bits for clock rows in bottom half of the device
- 12 bits for bottom IOB row
- 80 bits for every interconnect row, in order
- 12 bits for top IOB row
- 4 bits for clock rows in top half of the device

Thus, every interconnect tile corresponds to a bitstream tile of 22×80 bits. Such bitstream tiles are shared between the interconnect tiles and their associated primitives.

The IOB row tiles are 22×12 bits, and are shared between ``IOBS.*`` tiles and ``TERM.[SN]`` tiles. Likewise, IOB column tiles are 4×80 bits and are shared between ``IOBS.*`` and ``TERM.[EW]``.

Blockram data tiles are 64×320 bits (corresponding to 4 interconnect rows). The blockram data frame space corresponding to IOI and IOB rows is unused.

The space in the intersection of IOB columns and IOB or clock rows is unused.

The clock spine is made mostly of 4×80 tiles, most of which go unused. Additionally, there are two 4×12 tiles at the intersection with IOB rows, which are used for clock spine top/bottom configuration.

Every clock row in the bottom half of the device is assigned one bit from the bottom 4-bit area, in order starting from bit 0 at the bottom of the device. Every clock row in the top half of the device is assigned one bit from the top 4-bit area, in order starting from bit 0 at the *top* of the device. The remaining space, if any, is unused. The ``GCLKH`` tiles thus have 22×1 bits.
