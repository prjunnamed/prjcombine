# Device geometry


## General structure

Virtex 4 devices are the first to use fully columnar architecture — as opposed to earlier devices that had I/O ring surrounding the device, Virtex 4 devices are made almost entirely of fully uniform columns consisting of a single kind of tiles.

Virtex 4 devices are also divided in "regions". A region is always exactly 16 interconnect tiles high. The device always has a height that is a multiple of 32 interconnect tiles, or 2 regions. In addition to the 16 interconnect rows, each region has a special HCLK row running through the middle (between rows 7 and 8), which is not counted in the row numbering.

The exact sequence of columns varies with the device. The main available column types are:

- CLB column: contains `CLB` tiles, one for every interconnect tile.
- BRAM column: contains `BRAM` tiles, one for every 4 interconnect tiles (ie. 4 `BRAM` tiles per region)
- DSP column: contains `DSP` tiles, one for every 4 interconnect tiles (ie. 4 `DSP` tiles per region)
- IO column: contains `IO` tiles, one for every interconnect tile; also contains special `HCLK_IOIS_*` tiles in the HCLK rows
- the center column: there is exactly one of those per device; it contains a mixture of `CFG`, `IO`, `DCM`, `CCM`, `SYSMON` tiles
- MGT column: contains `MGT` tiles, one for every 2 regions (ie. one for every 32 interconnect tiles)

Each of the above types of columns is colocated with a single interconnect column. The interconnect column consists of:

- `INT` tiles, one per interconnect row (16 per region)
- `INTF` tiles, one for every `INT` tile, except for `INT` tiles associated with `CLB` tiles
- `HCLK` tiles, one per region (located in the HCLK row)

Additionally, there are two special kinds of columns that are not counted in the normal column numbering, and exist in between interconnect columns:

- the clock column, which is always immediately to the right of the center column; it contains:
  - the `BUFGCTRL` global clock buffers (located next to the `CFG` tile)
  - `CLK_IOB_*` and `CLK_DCM_*` tiles, which multiplex and feed clock sources into the `BUGCTRL` primitives
  - `CLK_HROW` tiles, which buffer the `BUFGCTRL` outputs onto HCLK rows
- the vbrk columns, the significance of which is unknown; on devices with transceivers, they contain `HCLK_MGT_REPEATER` tiles

There are always exactly two dedicated IO columns per device, and one center column which counts as the third IO column.

If the MGT columns are present on the device, there are exactly two of them, and they are the leftmost and the rightmost column, with the left and right IO columns a few columns away from them.  If the MGT columns are not present, the leftmost and rightmost columns are IO columns.

The other kinds of columns (CLB, BRAM, DSP) come in varying numbers, locations, and proportions, depending on device size and kind.

The leftmost column of the device, whether it is MGT or IO, contains a special `HCLK_TERM_L` tile in every HCLK row. Likewise, the rightmost column of the device contains a special `HCLK_TERM_R` tile in every HCLK row.


### PowerPC holes

Some devices have hard PPC cores, which are the only exceptions to the otherwise regular structure, creating a hole in the interconnect grid.  The hole is 24 rows high and 9 columns across. The 9 columns involved are always the following, in order:

- BRAM column
- 4 CLB columns
- BRAM column
- 3 CLB columns

The hole always starts at row 12 of a region, and ends at row 3 of another region (ie. it takes up 4 rows of one region, all 16 rows of the second region, and 4 more rows of the third region).

The bottom/top rows and leftmost/rightmost columns of the hole contain interconnect tiles as usual, providing inputs/outputs to the PPC core. However, the inner area consisting of 22 rows and 7 columns has no interconnect tiles, and some interconnect lines terminate at this boundary.


### Center column

The center column consists of the following main tiles, in order:

- 0 or 1 lower `SYSMON` tile; a `SYSMON` tile is 8 interconnect tiles high
- 2 to 6 lower `DCM` tiles; a `DCM` tile is 4 interconnect tiles high
- 0 to 2 lower `CCM` tiles; a `CCM` tile is 4 interconnect tiles high
- 16, 32, or 48 lower `IO` tiles, one per interconnect tile; this block of IO tiles always starts and ends at row 8 of a region
- the singular `CFG` tile, which is 16 interconnect tiles high and straddles two regions (top 8 rows of one region, then bottom 8 rows of the next region)
- 16, 32, or 48 upper `IO` tiles
- 0 to 2 upper `CCM` tiles
- 2 to 6 upper `DCM` tiles
- 0 or 1 upper `SYSMON` tile

The `CFG` tile, or rather the midpoint of it, is considered the center point of the device. The center column is mostly symmetric around the `CFG` tile:

- the amount of `IO` tiles below and above `CFG` is equal
- the amount of `CCM` tiles below and above `CFG` is equal
- the total height of `DCM + SYSMON` tiles below and above `CFG` is equal; however, there exist devices that have a `SYSMON` only on the bottom on the device, replacing it with two `DCM` tiles on the top

In addition to the main tiles, the center column also has special tiles in HCLK rows:

- every HCLK row completely within one of the `SYSMON + DCM + CCM` segments has a `HCLK_DCM` tile, routing clocks to the `DCM` and `CCM` tiles
- every HCLK row completely within one of the IO segments has a `HCLK_CENTER` tile, responsible for IO clocking and shared IO bank functionality
- the HCLK row on the boundary between lower `IO` tiles and `CFG` tile likewise has a `HCLK_CENTER` tile
- the HCLK row on the boundary between `CFG` tile and upper `IO` tiles has a `HCLK_CENTER_ABOVE_CFG` tile, which is a variant of the `HCLK_CENTER` tile
- the HCLK row on the boundary between lower `DCM/CCM` tiles and lower `IO` tiles has a `HCLK_DCMIOB` tile, combining the responsibilities of `HCLK_CENTER` and `HCLK_DCM` tiles
- the HCLK row on the boundary between upper `IO` tiles and upper `DCM/CCM` tiles likewise has a similar `HCLK_IOBDCM` tile


### Spine column

The spine column is responsible for global clock routing.  It has no corresponding interconnect column, borrowing interconnect from the center column where necessary.  It has the following tiles:

- the `CFG` tile occupies both the center column and the spine column (specifically, the `BUFGCTRL` buffers and their multiplexers are in the spine column)
- at the bottom: a single `CLK_TERM_B` tile
- at the top: a single `CLK_TERM_T` tile
- at every HCLK row: a `CLK_HROW` tile
- for every pair of `DCM` or `CCM` tiles: one `CLK_DCM_B` or `CLK_DCM_T` tile (which is 8 rows high)
- immediately above `HCLK_DCMIOB`: one `CLK_IOB_B` tile (which is 16 rows high)
- immediately below `HCLK_IOBDCM`: one `CLK_IOB_T` tile (which is 16 rows high)


## Bitstream geometry

The bitstream is made of frames, which come in three types:

- 0: main area
- 1: BRAM data area
- 2: BRAM interconnect area

The bitstream is now split by region — each frame covers 16 interconnect rows plus the HCLK row, and the frame size is independent of device size.

Frames are identified by their type, region, major and minor numbers. The major number identifies a column (interconnect column or the clock spine), and the minor number identifies a frame within a column. The major numbers are counted separately for each type of frame.

For bitstream purposes, the regions are counted using the `CFG` tile as the origin. Top region 0 is considered to be the region that contains the upper half of the `CFG` tile, top region 1 is the region above that, and so on. Bottom region 0 is considered to be the region that contains the lower half of the `CFG` tile, bottom region 1 is the region below that, and so on.

The main area contains all columns except BRAM columns but including the clock spine, with major numbers assigned sequentially from 0 on the left, one for each column. The clock spine is included right after the center column, with a separate major number. The columns have the following widths:

- CLB column: 22 frames
- DSP column: 21 frames
- center or IO column: 30 frames
- MGT column: 20 frames
- spine column: 3 frames

The BRAM data area contains 64 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

The BRAM interconnect area contains 20 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

Each frame is exactly 1312 bits long. There are:

- 80 bits per interconnect row
- 4 bits for HCLK row
- 12 bits for ECC
- 16 unused bits

The exact structure of the frame varies between the bottom and top halves of the device. The frames in the top half of the device have the following structure:

- bits 0-639: interconnect rows 0 to 7 of the region, 80 bits per row
- bits 640-651: ECC
- bits 652-655: HCLK row
- bits 656-671: unused
- bits 672-1311: interconnect rows 8 to 15 of the region, 80 bits per row

The frames in the bottom half of the device are almost but not entirely flipped — the bits corresponding to the interconnect rows are completely mirrored upside-down, but the bits corresponding to the ECC and HCLK row stay in the same place:

- bits 0-639: interconnect rows 15 to 8 of the region, 80 bits per row, with all bits in reverse order
- bits 640-651: ECC
- bits 652-655: HCLK row
- bits 656-671: unused
- bits 672-1311: interconnect rows 7 to 0 of the region, 80 bits per row, with all bits in reverse order

Every interconnect tile thus corresponds to a bitstream tile that is 20×80 to 30×80 bits. The actual interconnect tile is 19×80 bits, occupying the first 19 frames of the column. The remaining frames, as well as unused space in frame 19, are used for configuring the associated primitive tile.

The HCLK row has smaller bitstream tiles, 20×4 to 30×4 bits in size.

The spine column also has smaller bitstream tiles, 3×80 in size, as well as the extra-small 3×4 tiles on intersections with HCLK rows.

The BRAM data tiles are 64×320 bits in size (covering the height of 4 interconnect rows). The area at intersection with HCLK rows is unused.


### ECC

TODO: reverse, document