.. _virtex6-geometry:

Device geometry
###############


General structure
=================

Virtex 6 devices follow the same general structure as Virtex 4 and Virtex 5 devices, with minor changes.

Virtex 6 devices are divided into "regions". A region is exactly 40 interconnect tiles high. In addition to 40 interconnect rows, each region has a special HCLK row running through the middle (between rows 19 and 20), which is not counted in the row numbering.

The exact sequence of columns varies with the device. The main available column types are:

- CLBLL column: contains ``CLBLL`` tiles, one for every interconnect tile.
- CLBLM column: contains ``CLBLM`` tiles, one for every interconnect tile.
- BRAM column: contains ``BRAM`` tiles, one for every 5 interconnect tiles (ie. 8 ``BRAM`` tiles per region)
- hard logic column: a variant of BRAM column; contains a mixture of:

  - ``BRAM`` tiles
  - ``EMAC`` tiles (10 interconnect tiles high, replaces two ``BRAM`` tiles)
  - ``PCIE`` tiles (20 interconnect tiles high and 4 columns wide, replaces 4 ``BRAM`` tiles and 3×20 CLBs in the three CLB columns to the left)

- DSP column: contains ``DSP`` tiles, one for every 5 interconnect tiles (ie. 8 ``DSP`` tiles per region)
- IO column: contains ``IO`` tiles, one for every 2 interconnect tiles; also contains special ``HCLK_IOI`` tiles in the HCLK rows
- the center column: there is exactly one of those per device; it contains a mixture of ``CFG``, ``CMT``, ``CMT_BUFG_*``, and clock distribution tiles
- GT column: for each region, contains either four ``GTX`` tiles (each of them 10 rows high), or one ``GTH`` tile (40 rows high)

Each of the above types of columns is colocated with a single interconnect column. The interconnect column consists of:

- ``INT`` tiles, one per interconnect row (16 per region)
- ``INTF`` or ``INTF.DELAY`` tiles, one for every ``INT`` tile, except for ``INT`` tiles associated with ``CLB*`` tiles

  - ``INTF`` is associated with ``BRAM``, ``DSP``, ``IO``, ``CMT``, and ``CFG`` tiles
  - ``INTF.DELAY`` is associated with ``GTX``, ``GTH``, ``PCIE``, and ``EMAC`` tiles

- ``HCLK`` tiles, one per region (located in the HCLK row)

While there is still a clock spine column, it is no longer considered meaningfully separate from the center column.

There can be up to four IO columns per device:

- outer left column (present on all except ``hxt`` devices, always the leftmost column of the device when present)
- inner left column (always present, relatively close to the center column)
- inner right column (always present, relatively close to the center column)
- outer right column (present on some devices; is the rightmost column of the device if the device has no MGT columns)

There can be up to two GT columns on the device, depending on the device type:

- ``xc6vlx760`` has no GT columns at all; the leftmost and rightmost columns are the outer IO columns
- ``lxt``, ``cxt``, and ``sxt`` devices have one GT column, which is the rightmost column of the device; it contains ``GTX`` tiles
- ``hxt`` devices have two GT columns, the leftmost and the rightmost column of the device; they contain ``GTX`` tiles and, on all devices except ``xc6vhx250t``, additionally some ``GTH`` tiles

If the device contains MGT columns, it also has a hard logic column, which is the second rightmost column of the device, right next to the right MGT column. ``lx`` devices have no hard logic column.

The other kinds of columns (CLB, BRAM, DSP) come in varying numbers, locations, and proportions, depending on device size and kind.

The grid is mostly regular, but can have holes of two kinds:

- the configuration center (``CFG`` tile) occupies a 6×80 area of what would otherwise be ``CLBLL`` tiles to the left of the center column; this area has no interconnect tiles; horizontal interconnect lines pass through the hole, skipping over the 6 columns, while vertical interconnect lines cannot cross the hole, and are turned around at the bottom and top edge of the hole. The inputs/outputs for the primitives in the ``CFG`` tile are borrowed from the ``INT`` tiles in the center column.
- the ``PCIE`` tile occupies a 4×20 area of interconnect tiles; the columns it occupies are, in order:

  - a CLBLL column; the ``CLBLL`` tiles in this area are skipped, but the ``INT`` tiles remain and serve as interconnect to the ``PCIE`` tile
  - a CLBLM column; the ``CLBLM`` tiles in this area are skipped, but the ``INT`` tiles remain and serve as interconnect to the ``PCIE`` tile
  - a CLBLL column; both the ``CLBLL`` and ``INT`` tiles in this area are skipped
  - the hard logic column; there are no ``INT`` tiles in this area

  As with the ``CFG`` tile, horizontal interconnect lines jump across the skipped ``INT`` tiles, but vertical lines are turned around at the bottom and top edge.


Center column
-------------

The center column has a triple role as the configuration center, CMT column, and clock distrubution column. It has the following contents:

- there is one ``CMT`` tile (consisting of two ``MMCM``\ s) per region
- two consecutive regions in the middle additionally host the ``CFG`` tile; the boundary between this two regions is considered to be the "center point" of the device, regardless of how far it is from actual geometrical center (the devices can be quite unbalanced in that respect); we will call the regions hosting ``CFG`` tile ``c - 1`` and ``c``
- the top 3 rows of region ``c - 1`` contain the ``CMT_BUFG_BOT`` tile, which contains 16 global clock buffers (``BUGCTRL``)
- the bottom 3 rows of region ``c`` contain the ``CMT_BUFG_TOP`` tile, which contains another set of 16 global clock buffers (``BUGCTRL``)
- the top rows of regions ``c - 2`` and below contain the ``GCLK_BUF`` tile, which rebuffers the global clock signals
- likewise, the bottom rows of regions ``c + 1`` and above also contain the ``GCLK_BUF`` tile
- the bottom two rows of regions ``c - 2`` and below contain the ``PMVIOB`` tile, the purpose of which is unknown
- the top two rows of regions ``c`` and above likewise contain the ``PMVIOB`` tile


Bitstream geometry
==================

The bitstream is made of frames, which come in two types:

- 0: main area
- 1: BRAM data area

The bitstream is split by region — each frame covers 40 interconnect rows plus the HCLK row, and the frame size is independent of device size.

Frames are identified by their type, region, major and minor numbers. The major number identifies a column (interconnect column or the clock spine), and the minor number identifies a frame within a column. The major numbers are counted separately for each type of frame.

For bitstream purposes, the regions are counted using the ``CFG`` tile as the origin. Top region 0 is considered to be the region that contains the upper half of the ``CFG`` tile, top region 1 is the region above that, and so on. Bottom region 0 is considered to be the region that contains the lower half of the ``CFG`` tile, bottom region 1 is the region below that, and so on.

The main area contains all interconnect columns, with major numbers assigned sequentially from 0 on the left, one for each column. The columns have the following widths:

- CLBLL and CLBLM columns: 36 frames
- BRAM or hard logic column: 28 frames
- DSP column: 28 frames
- IO column: 44 frames
- center column: 38 frames
- MGT column: 30 frames

The BRAM data area contains 128 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

Each frame is exactly 2592 bits long and has the following structure:

- bits 0-1279: interconnect rows 0 to 19 of the region, 64 bits per row
- bits 1280-1292: ECC
- bits 1293-1311: HCLK row
- bits 1312-2591: interconnect rows 20 to 39 of the region, 64 bits per row

Every interconnect tile thus corresponds to a bitstream tile that is 28×64 to 44×64 bits. The actual interconnect tile is 26×64 bits, occupying the first 26 frames of the column. If ``INTF`` or ``INTF.DELAY`` tile is present in the tile, it occupies leftover space in frames 24 and 25.  The remaining frames, as well as unused space in frames 24-25, are used for configuring the associated primitive tile.

The HCLK row has smaller bitstream tiles, 28×19 to 44×19 bits in size.

The BRAM data tiles are 128×320 bits in size (covering the height of 5 interconnect rows). The area at intersection with HCLK rows is unused.


ECC
---

.. todo:: reverse, document
