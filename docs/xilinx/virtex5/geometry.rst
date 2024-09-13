.. _virtex5-geometry:

Device geometry
###############


General structure
=================

Virtex 5 devices follow the same general structure as Virtex 4 devices, with minor changes.

Virtex 5 devices are divided into "regions". A region is exactly 20 interconnect tiles high. In addition to 20 interconnect rows, each region has a special HCLK row running through the middle (between rows 9 and 10), which is not counted in the row numbering.

The exact sequence of columns varies with the device. The main available column types are:

- CLBLL column: contains ``CLBLL`` tiles, one for every interconnect tile.
- CLBLM column: contains ``CLBLM`` tiles, one for every interconnect tile.
- BRAM column: contains ``BRAM`` tiles, one for every 5 interconnect tiles (ie. 4 ``BRAM`` tiles per region)
- hard logic column: a variant of BRAM column; contains a mixture of:

  - ``BRAM`` tiles
  - ``EMAC`` tiles (10 interconnect tiles high, replaces two ``BRAM`` tiles)
  - ``PCIE`` tiles (40 interconnect tiles high, replaces 8 ``BRAM`` tiles)

- DSP column: contains ``DSP`` tiles, one for every 5 interconnect tiles (ie. 4 ``DSP`` tiles per region)
- IO column: contains ``IO`` tiles, one for every interconnect tile; also contains special ``HCLK_IOI`` tiles in the HCLK rows
- the center column: there is exactly one of those per device; it contains a mixture of ``CFG``, ``IO``, ``CMT`` tiles
- GT column: contains ``GTP`` or ``GTX`` tiles, one for every region (ie. one for every 20 interconnect tiles)

Each of the above types of columns is colocated with a single interconnect column. The interconnect column consists of:

- ``INT`` tiles, one per interconnect row (16 per region)
- ``INTF`` or ``INTF.DELAY`` tiles, one for every ``INT`` tile, except for ``INT`` tiles associated with ``CLB*`` tiles

  - ``INTF`` is associated with ``BRAM``, ``DSP``, ``IO``, ``CMT``, and ``CFG`` tiles
  - ``INTF.DELAY`` is associated with ``GTP``, ``GTX``, ``PCIE``, ``EMAC``, and ``PPC`` tiles

- ``HCLK`` tiles, one per region (located in the HCLK row)

Additionally, there are two special kinds of columns that are not counted in the normal column numbering, and exist in between interconnect columns:

- the clock column, which is always immediately to the right of the center column; it contains:

  - the ``BUFGCTRL`` global clock buffers (located next to the ``CFG`` tile)
  - ``CLK_IOB_*``, ``CLK_CMT_*``, and ``CLK_MGT_*`` tiles, which multiplex and feed clock sources into the ``BUGCTRL`` primitives
  - ``CLK_HROW`` tiles, which buffer the ``BUFGCTRL`` outputs onto HCLK rows

- the vbrk columns, the significance of which is unknown

There can be up to two dedicated IO columns per device, the left and right IO column. The center column counts as the third IO column.  The left IO column is the leftmost column of the device, except for devices that have a left MGT column, where it is a few columns away from the left edge.  The right IO column is present on all devices except ``xc5vlx20t``, and is located a few columns away from the right edge.

There can be up to two MGT columns on the device, depending on the device type:

- the ``lxt`` and ``sxt`` devices have exactly one MGT column, which contains ``GTP`` tiles and is the rightmost column of the device
- the ``lx`` devices have no MGT columns; however, they are suspiciously identical to ``lxt`` devices with the rightmost two columns cut off
- the ``fxt`` devices have exactly one MGT column, which contains ``GTX`` tiles and is the rightmost column of the device
- the ``txt`` devices have exactly two MGT columns, which contain ``GTX`` tiles and are the leftmost and rightmost columns of the device

If the device contains MGT columns, it also has a hard logic column, which is the second rightmost column of the device, right next to the right MGT column. ``lx`` devices have no hard logic column.

The other kinds of columns (CLB, BRAM, DSP) come in varying numbers, locations, and proportions, depending on device size and kind.


PowerPC holes
-------------

Some devices have hard PPC cores, which are the only exceptions to the otherwise regular structure, creating a hole in the interconnect grid.  The hole is 2 regions (40 rows) high and 14 columns across. The 14 columns involved are always the following, in order:

- CLBLM
- CLBLL
- BRAM
- CLBLM
- CLBLL
- CLBLM
- CLBLL
- CLBLM
- CLBLL
- BRAM
- CLBLM
- CLBLL
- CLBLM
- CLBLL

The hole always starts at row 0 of a region, and ends at row 19 of another region (ie. it takes up 2 whole regions).

The leftmost/rightmost columns of the hole contain interconnect tiles as usual, providing inputs/outputs to the PPC core. However, the inner 12 columns have no interconnect tiles, and some interconnect lines terminate at the boundaries.


Center column
-------------

The full-featured center column contains the following main tiles, in order:

- region ``c - 6`` through ``c - 4``: 20 IO tiles per region, one per row, belong to banks 10, 8, 6 (in order from the bottom)
- region ``c - 3`` and bottom half of region ``c - 2``: three ``CMT`` tiles, each of them 10 interconnect rows high
- top half of region ``c - 2``: 10 IO tiles, belonging to bank 4
- bottom half of region ``c - 1``: 10 IO tiles, belonging to bank 2
- top half of region ``c - 1`` and bottom half of region ``c``: the ``CFG`` tile, which is 20 interconnect rows high
- top half of region ``c``: 10 IO tiles, belonging to bank 1
- bottom half of region ``c + 1``: 10 IO tiles, belonging to bank 3
- top half of region ``c + 1`` and all of region ``c + 2``: three ``CMT`` tiles, each of them 10 interconnect rows high
- region ``c + 3`` through ``c + 5``: 20 IO tiles per region, belonging to banks 5, 7, 9 (in order from the bottom)

In devices that are less than 12 regions high, this full-featured column will be truncated at the top and the bottom — first trimming off banks 9 and 10, then 7 and 8, then 5 and 6, then CMTs, then finally trimming bank 3.  The center column on the smallest device, ``xc5vlx20t``, contains only one CMT at the bottom, the ``CFG`` tile, and banks 1, 2, 4.

In addition to the main tiles, the center column also has special tiles in HCLK rows:

- ``HCLK_IOI_CMT`` in the HCLK row above lower ``CMT`` segments (region ``c - 2``)
- ``HCLK_IOI_BOTCEN`` in the HCLK row right below ``CFG`` (region ``c - 1``)
- ``HCLK_IOI_TOPCEN`` in the HCLK row right above ``CFG`` (region ``c``)
- ``HCLK_CMT_IOI`` in the HCLK row below upper ``CMT`` segments (region ``c + 1``)
- ``HCLK_CMT`` in the HCLK rows between two ``CMT`` tiles (regions ``c - 3`` and ``c + 2``)
- ``HCLK_IOI_CENTER`` in the remaining HCLK rows


Spine column
------------

The spine column is responsible for global clock routing.  It has no corresponding interconnect column, borrowing interconnect from the center column where necessary.  It has the following tiles:

- the ``CFG`` tile occupies both the center column and the spine column (specifically, the ``BUFGCTRL`` buffers and their multiplexers are in the spine column)
- at every HCLK row: a ``CLK_HROW`` tile
- next to bank 4: a ``CLK_IOB_B`` tile (10 rows high)
- next to bank 2: a ``CLK_IOB_T`` tile (10 rows high)
- next to every ``CMT`` tile: a ``CLK_CMT_B`` (below ``CFG``) or ``CLK_CMT_T`` (above ``CFG``) tile (10 rows high)
- next to banks 5-10: a ``CLK_MGT_B`` (below ``CFG``) or ``CLK_MGT_T`` (above ``CFG``) tile, one per bank (or per region); the tile is 10 rows high and occupies the top half of the region (below ``CFG``), or the bottom half of the region (above ``CFG``)


Bitstream geometry
==================

The bitstream is made of frames, which come in two types:

- 0: main area
- 1: BRAM data area

The bitstream is split by region — each frame covers 20 interconnect rows plus the HCLK row, and the frame size is independent of device size.

Frames are identified by their type, region, major and minor numbers. The major number identifies a column (interconnect column or the clock spine), and the minor number identifies a frame within a column. The major numbers are counted separately for each type of frame.

For bitstream purposes, the regions are counted using the ``CFG`` tile as the origin. Top region 0 is considered to be the region that contains the upper half of the ``CFG`` tile, top region 1 is the region above that, and so on. Bottom region 0 is considered to be the region that contains the lower half of the ``CFG`` tile, bottom region 1 is the region below that, and so on.

The main area contains all interconnect columns and the clock spine, with major numbers assigned sequentially from 0 on the left, one for each column. The clock spine is included right after the center column, with a separate major number. The columns have the following widths:

- CLBLL and CLBLM columns: 36 frames
- BRAM or hard logic column: 30 frames
- DSP column: 28 frames
- center or IO column: 54 frames
- MGT column: 32 frames
- spine column: 4 frames

The BRAM data area contains 128 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

Each frame is exactly 1312 bits long and has the following structure:

- bits 0-639: interconnect rows 0 to 9 of the region, 64 bits per row
- bits 640-651: ECC
- bits 652-655: HCLK row
- bits 656-671: unused
- bits 672-1311: interconnect rows 10 to 19 of the region, 64 bits per row

Every interconnect tile thus corresponds to a bitstream tile that is 28×64 to 54×64 bits. The actual interconnect tile is 26×64 bits, occupying the first 26 frames of the column. If ``INTF`` or ``INTF.DELAY`` tile is present in the tile, it occupies frames 26-27.  The remaining frames, as well as unused space in frames 26-27, are used for configuring the associated primitive tile.

The HCLK row has smaller bitstream tiles, 28×4 to 54×4 bits in size.

The spine column also has smaller bitstream tiles, 4×64 in size, as well as the extra-small 4×4 tiles on intersections with HCLK rows.

The BRAM data tiles are 128×320 bits in size (covering the height of 5 interconnect rows). The area at intersection with HCLK rows is unused.


ECC
---

.. todo:: reverse, document