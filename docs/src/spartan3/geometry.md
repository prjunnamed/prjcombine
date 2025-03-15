# Device geometry

Spartan 3 geometry is largely the same as [Virtex 2](../virtex2/geometry.md). The changes are:

- in addition to the primary vertical clock spine, there are now two secondary vertical clock spines that distribute clocks to `GCLKH` tiles; there is also the new horizontal clock spine that connects them together
- the `TERM.*` tiles no longer have multiplexers, and thus are no longer present in the bitstream
- when more than 2 BRAM columns are present, only the leftmost and rightmost ones have DCMs; the others have empty DCM stubs
- the `CLKB` and `CLKT` tiles contain 4 `BUFGMUX` primitives each, making for 8 global clocks total; this eliminates muxes from clock distribution
- `PCILOGIC` is removed

Spartan 3E includes more changes:

- BRAM columns no longer take up the whole height of the device; instead, a BRAM column is a 4-CLBs-wide hole in the CLB grid which spans most of the height of the device
- DCMs are no longer located at the top and bottom of BRAM columns; instead, they are located in 4×4 holes in the CLB grid, close to the `CLK[LRTB]` tiles
- in addition to the `CLKB` and `CLKT` tiles, there are new `CLKL` and `CLKR` tiles, with 8 `BUFGMUX` each, located on the ends of the horizontal clock spine; the clocks are usable only in the left and right half of the device, respectively; the `GCLKVM` tiles now contain per-device-quadrant muxes that can select 8 clocks from the set of clocks driven by `CLK[LR]` and `CLK[BT]` tiles
- the `CLK[LR]` tiles also contain the new `PCILOGICSE` primitive that contains a bit of hard logic helping with PCI implementation
- on larger devices, the `LV.*` and `LH.*` long lines are optionally split in the middle; new `LLV.*` and `LLH.*` tiles are added that contain the buffers that can optionally join the two segments together; they are located on the horizontal clock spine and primary vertical clock spine

Spartan 3A includes the following changes to Spartan 3E geometry:

- the BRAM columns now span almost the entire height of the device, with no CLBs above or below, but with IOI tiles still present
- the 4×4 DCM holes are sometimes located within BRAM columns instead of CLB grid; a DCM replaces one BRAM

The `xc3s50a` is a special small Spartan 3A device with unusual geometry:

- there are no secondary clock spines
- there is only one clock row
- the role of `GCLKVM` multiplexer tiles is taken by the new `CLKC_50A` tile in the middle of the device, which directly drives the single clock row

Spartan 3A DSP includes the following changes to Spartan 3A geometry:

- BRAM columns are now 3 IOI tiles wide instead of 4
- a new type of column is introduced, the DSP column; like BRAM, it has IOI tiles on top and bottom, but is only one IOI tile wide; a DSP column is always located immediately to the right of BRAM column
- the DCM holes in BRAM column also extend to the DSP column


## General structure

The Spartan 3 devices are made of a rectangular grid of interconnect tiles.

Interconnect rows come in three kinds:

- bottom IOI row (the bottommost row)
- top IOI row (the topmost row)
- general row (all other rows)

Interconnect columns come in several kinds:

- left IOI column (the leftmost column)
  - contains `INT.CNR` and the `LL` (lower left) corner tile in the bottom IOI row
  - contains `INT.CNR` and the `UL` (upper left) corner tile in the top IOI row
  - contains `INT.IOI` and an `IOI` tile in remaining rows
- right IOI column (the rightmost column)
  - contains `INT.CNR` and the `LR` (lower right) corner tile in the bottom IOI row
  - contains `INT.CNR` and the `UR` (upper right) corner tile in the top IOI row
  - contains `INT.IOI` and an `IOI` tile in remaining rows
- CLB columns (most of the inner columns)
  - contains `INT.IOI` and an `IOI` IO tile in the bottom and top IOI rows
  - contains `INT.CLB` and a `CLB` tile in the remaining rows (except for holes on Spartan 3E and up)
- (plain Spartan 3) BRAM columns (some of the inner columns)
  - contains `INT.DCM.*` and a `DCM` tile in the bottom and top IOI rows
  - contains `INT.BRAM.S3` in the remaining rows, and a `BRAM` tile every 4 rows
- (Spartan 3E) BRAM columns, which actually take up 4 interconnect columns
  - the top and bottom of each of the 4 interconnect columns is identical to a CLB column
  - the bottom BRAM row contains nothing in the leftmost column (vertical and horizontal interconnect lines pass right through it), and `TERM.BRAM.N` in the other 3 columns, terminating the vertical interconnect but passing through horizontal interconnect
  - likewise, the top BRAM row contains nothing in the leftmost column, and `TERM.BRAM.S` in the other 3 columns
  - the rows between the bottom and top BRAM row contain `INT.BRAM.S3E` tiles in each row of the leftmost column, and a `BRAM` tile every 4 rows
- (Spartan 3A) BRAM columns, which actually take up 4 interconnect columns
  - the top IOI and bottom IOI row of each of the 4 interconnect columns contains an `INT.IOI` and `IOI` tiles just like CLB columns
  - the remaining rows contain `INT.BRAM.S3A.*` in every row of the leftmost column, and `BRAM` every 4 rows (except for DCM holes)
- (Spartan 3A DSP) BRAM columns, which actually take up 3 interconnect columns
  - like Spartan 3A, but with only 3 columns instead of 4 and with `INT.BRAM.S3ADSP` tiles
- (Spartan 3A DSP) DSP columns
  - contains `INT.IOI` and an `IOI` IO tile in the bottom and top IOI rows
  - contains `INT.BRAM.S3ADSP` in the remaining rows, and a `DSP` tile every 4 rows

The interconnect contains holes; depending on the type of the hole, some interconnect lines are either bounced back at the edges (via `TERM.*` tiles), or pass through the hole unaffected. Whenever interconnect skips across a hole, the hole is not counted towards the conceptual length of the interconnect lines — eg. a `HEX` line will always span the distance of 6 `INT.*` tiles, even if it takes 9 interconnect columns because of a 3 tile wide hole in the middle. Likewise, `LV.*` and `LH.*` lines are only rotated when crossing an actual `INT.*` tile.

### Spartan 3E BRAM columns

A BRAM column makes a hole in the interconnect structure. The hole works as follows:

- the 3 interconnect columns to the right of an `INT.BRAM.*` tile contain no interconnect tiles; horizontal interconnect passes through them to the next `INT.CLB` tile in the row
- the bottom and top row of the column contain no interconnect tiles
  - horizontal interconnect passes through them (directly between the `INT.CLB` to the left and to the right of the column)
  - vertical interconnect passes through the leftmost tile in the row (directly between `INT.CLB` and `INT.BRAM.*`)
  - the remaining 3 tiles contain `TERM.BRAM.[NS]` terminators that bounce most of the vertical interconnect back; long lines (`LV.*` and `LH.*`) are an exception, and pass the whole height of the BRAM column

### Spartan 3A and 3A DSP BRAM columns

A BRAM column makes a hole in the interconnect structure. The hole works as follows:

- the 3 (Spartan 3A) or 2 (Spartan 3A DSP) columns to the right of an `INT.BRAM.*` tile contain no interconnect tiles; horizontal interconnect passes through them
- vertical interconnect doesn't pass through the 3 or 2 rightmost columns — the `INT.IOI` tiles on the top and bottom of the column aren't connected by vertical interconnect in any way

### Spartan 3E DCMs

Spartan 3E devices can have 2, 4, or 8 DCMs:

- the devices with 2 DCMs have one DCM at the top, close to `CLKT`, and one DCM at the bottom, close to `CLKB`
- the devices with 4 DCMs have two DCMs at the top, close to `CLKT`, and two DCMs at the bottom, close to `CLKB`
- the devices with 8 DCMs have the 4 DCMs above, and also two DCMs on the left, close to `CLKL`, and two DCMs on the right, close to `CLKR`

Spartan 3E DCMs are designed to work in pairs, with some shared circuitry between them. The devices with 2 DCMs instead contain single DCMs. The single DCMs are actually a cut-down version of a DCM pair, and contain a stub DCM tile.

The top and bottom holes work as follows:

- the DCM hole spans the bottom 4 or top 4 general rows (displacing some CLBs but not IOIs)
- on devices with 2 DCMs, the DCM hole spans 5 columns, from the column immediately to the left of the clock spine, to the 4th column right from the primary clock spine
- on devices with 4 or more DCMs, the DCM hole spans 8 columns, from the 4th column to the left of the clock spine, to the 4th column right from the primary clock spine
- the two tiles closest to the `CLKB` / `CLKT` contain `INT.DCM.*` interconnect tiles; the remaining tiles contain no `INT.*` tiles and interconnect passes right through them
- on devices with 2 DCMs, the `INT.DCM.*` tile to the right is associated with a `DCM` tile; the tile to the left is a stub
- on other devices, both `INT.DCM.*` tiles are associated with a DCM

The left and right holes (present only on devices with 8 DCMs) work as follows:

- the hole spans 4 columns and 8 rows
- the hole spans rows from the 4th row below the horizontal clock spine to the 4th row above the horizontal clock spine
- the left hole spans from column 9 to column 12 (there is a space of 8 columns between the DCM and the left IOIs)
- the right hole spans from column `width - 13` to `width - 10` (there is likewise a space of 8 columns between the DCM and the right IOIs)
- the hole contains two `INT.DCM.*` tiles in the column closest to the IOIs, in the two rows closest to the horizontal clock spine; each of them is associated with a `DCM` tile
- remaining tiles of the hole have no `INT.*` tiles and interconnect passes right through them

### Spartan 3A DCMs

Spartan 3A (and 3A DSP) devices have 2, 4, or 8 DCMs:

- the devices with 2 DCMs have two DCMs at the top, close to `CLKT`
- the devices with 4 DCMs have two DCMs at the top, close to `CLKT`, and two DCMs at the bottom, close to `CLKB` (like Spartan 3E)
- the devices with 8 DCMs have the 4 DCMs above, and also two DCMs on the left, close to `CLKL`, and two DCMs on the right, close to `CLKR`

The placement of the top and bottom holes is identical to Spartan 3E, except that devices with 2 DCMs have both DCMs in the top hole (which is 8×4 tiles), and no bottom hole. On the smallest device, the top DCM hole displaces BRAM tiles instead of CLB tiles.

The left and right holes (present only on devices with 8 DCMs) work as follows:

- the hole spans 4 columns and 8 rows
- the hole spans rows from the 4th row below the horizontal clock spine to the 4th row above the horizontal clock spine
- the left hole spans from column 3 to column 6 (there is a space of 2 CLB columns between the DCM and the left IOIs)
- the right hole spans from column `width - 7` to `width - 4` (there is likewise a space of 2 CLB columns between the DCM and the right IOIs)
- the hole is always placed in the middle of a BRAM column (or, for Spartan 3A DSP devices, a BRAM column and a DSP column)
- the hole contains two `INT.DCM.*` tiles in the leftmost column, in the two rows closest to the horizontal clock spine; each of them is associated with a `DCM` tile
- remaining tiles of the hole have no `INT.*` tiles and interconnect passes right through them


## Clock rows

The device has some amount of clock rows. They exist in between interconnect rows, and are not counted in the coordinate system. Each clock row provides dedicated clock routing to some range of interconnect rows.

On the intersection of every clock row and interconnect column there is a `GCLKH.*` tile, which distributes the clocks vertically to some segment of interconnect tiles.

The horizontal clock lines in the clock row are driven from `GCLKVC.*` tiles at the intersections with the secondary vertical clock spines.

The `xc3s50a` device is an exception — on that device, there is only one clock row (colocated with horizontal clock spine), and it is driven from the `CLKC_50A` tile at the intersection with primary vertical clock spine.


## Primary vertical clock spine

The device has a special column, called the primary vertical clock spine. It is located in between two interconnect columns, somewhere in the middle of the device. It is not counted in the coordinate system.

The clock spine has special tiles dealing with clock interconnect:

- the bottom IOI row has the `CLKB.*` tile, with 4 `BUFGMUX` primitives
- the top IOI row has the `CLKT.*` tile, with 4 `BUFGMUX` primitives
- the intersection with horizontal clock spine has the `CLKC` tile, which buffers the vertical clock lines onto horizontal clock lines, going to `GCLKVM` tiles
- on `xc3s50a`, the `CLKC` tile is replaced with the `CLKC_50A` tile, which instead multiplexes the vertical and horizontal clock lines (from the horizontal clock spine) directly onto the clock row


## Horizontal clock spine

The device has a special row, called the horizontal clock spine. It is located in between two interconnect rows, exactly in the middle of the device. It is not counted in the coordinate system.

The horizontal clock spine has special tiles dealing with clock interconnect:

- the left IOI column has the `CLKL.*` tile, with 8 `BUFGMUX` primitives and a `PCILOGICSE` primitive
- the right IOI column has the `CLKR.*` tile, with 8 `BUFGMUX` primitives and a `PCILOGICSE` primitive
- the intersection with the primary vertical clock spine has the `CLKC` tile, which buffers the vertical clock lines onto horizontal clock lines
- the intersections with the secondary vertical clock spines have `GCLKVM` tiles, which multiplex between clock signals from `CLKC` and `CLKL` or `CLKR`, and drive vertical clock lines towards `GCLKVC` tiles
- on `xc3s50a`, there are no `GCLKVM` tiles, and the `CLKC` tile is replaced with `CLKC_50A` that multiplexes the clocks and drives the clock row directly


## Secondary vertical clock spines

The device has two special columns, the left and right secondary vertical clock spines. Like the primary clock spine, they are located in between two interconnect columns and don't count towards the coordinate system. The secondary clock spines are located somewhere to the left and to the right of the primary clock spine, and together with the primary clock spine divide the device roughly in 4 equal parts.

The secondary clock spines have special tiles dealing with clock interconnect:

- the intersection with the horizontal clock spine has a `GCLKVM` tile that multiplexes clocks signals and drives the vertical clock lines in the column
- the intersections with the clock rows have `GCLKVC` tiles that drive the horizontal clock signals in the clock row

The `xc3s50a` device is special and doesn't have secondary clock spines.


## IO banks and IO buffers

Spartan 3 devices have 8 IO banks, numbered 0 through 7 in clockwise fashion:

- 0: top IO row, to the left of primary clock spine
- 1: top IO row, to the right of primary clock spine
- 2: right IO column, top half
- 3: right IO column, bottom half
- 4: bottom IO row, to the right of primary clock spine
- 5: bottom IO row, to the left of primary clock spine
- 6: left IO column, bottom half
- 7: left IO column, top half

Spartan 3E and 3A devices have 4 IO banks, numbered 0 through 3 in clockwise fashion:

- 0: top IO row
- 1: right IO column
- 2: bottom IO row
- 3: left IO column

The edges of the device have `IOI.*` tiles, each of which contains 3 or 2 `IOI` (IO interface) primitives that implement IO registers. However, not all `IOI` primitives have an associated IO buffer.

The IO buffers live in special `IOBS.*` tiles, contained in:

- top IOB row, above the top IOI row
- bottom IOB row, below the bottom IOI row
- left IOB column, to the left of the left IOI column
- right IOB column, to the right of the right IOI column

The `IOBS.*` tiles come in multiple variants, and can span one or two `IOI` tiles.

TODO: more details


## Bitstream geometry

The bitstream is made of frames, which come in three types:

- 0: main area
- 1: BRAM data area
- 2: BRAM interconnect area

Frames are identified by their type, major and minor numbers. The major number identifies a column (interconnect column, clock spine, or IOB column), and the minor number identifies a frame within a column. The major numbers are counted separately for each type of frame.

The main area contains the following columns, in order (with major numbers assigned sequentially from 0, one for each column):

- a special column for the vertical clock spines (2, 3, or 4 frames)
  - Spartan 3 devices and Spartan 3E devices without `LL*` tiles have 3 frames
    - frame 0 corresponds to left secondary clock spine
    - frame 1 corresponds to right secondary clock spine
    - frame 2 corresponds to primary clock spine
  - Spartan 3E devices with `LL*` tiles and Spartan 3A devices other than `xc3s50a` have 4 frames
    - frames 0-1 are allocated as above
    - frames 2-3 correspond to primary clock spine
  - the `xc3s50a` device has 2 frames, corresponding to the primary clock spine
- the left IOB column (2 frames)
- the left IOI column (19 frames)
- the CLB and DSP columns, in order (19 frames each)
- the right IOI column (19 frames)
- the right IOB column (2 frames)

The BRAM data area contains 76 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

The BRAM interconnect area contains 19 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

On Spartan 3E and 3A, the BRAM tiles are special in that they span several interconnect columns. The data for interconnect columns is placed as follows:

- the leftmost interconnect column (which has `INT.*` for (almost) the whole height of the device) is placed in the BRAM interconnect area
- the remaining interconnect columns, where they have `INT.*` tiles, are placed in the BRAM data area (which, in the relevant rows, is not used for actual BRAM data):
  - column 1 of the hole is placed in frames 0-18 of the BRAM data area
  - column 2 of the hole is placed in frames 19-37 of the BRAM data area
  - column 3 of the hole (on devices other than Spartan 3A DSP) is placed in frames 38-56 of the BRAM data area

For example, `xc3s100e` has the following frames (`type.major.minor`):

- `0.0.0-2`: clock spines
- `0.1.0-1`: left IOB column
- `0.2.0-18`: left IOI column (interconnect X == 0)
- `0.3.0-18`: CLB column 1 (interconnect X == 1)
- `0.4.0-18`: CLB column 2 (interconnect X == 2)
- `0.5.0-18`: CLB column 3 (interconnect X == 7)
- `0.6.0-18`: CLB column 4 (interconnect X == 8)
- `0.7.0-18`: CLB column 5 (interconnect X == 9)
- `0.8.0-18`: CLB column 6 (interconnect X == 10)
- `0.9.0-18`: CLB column 7 (interconnect X == 11)
- `0.10.0-18`: CLB column 8 (interconnect X == 12)
- `0.11.0-18`: CLB column 9 (interconnect X == 13)
- `0.12.0-18`: CLB column 10 (interconnect X == 14)
- `0.13.0-18`: CLB column 11 (interconnect X == 15)
- `0.14.0-18`: CLB column 12 (interconnect X == 16)
- `0.15.0-18`: right IOI column (interconnect X == 17)
- `0.16.0-1`: right IOB column
- `1.0.0-75`: BRAM column 1 data
  - `1.0.0-18`: data for CLBs in interconnect X == 4
  - `1.0.18-37`: data for CLBs in interconnect X == 5
  - `1.0.38-56`: data for CLBs in interconnect X == 6
- `2.0.0-18`: BRAM column 1 interconnect (interconnect X == 3)

Each bitstream frame within a device has the same size. The size is `32 + num_interconnect_rows * 64` bits. The bits are, in order:

- 16 special bits, assigned as follows:
  - Spartan 3 and Spartan 3E:
    - bits 0-3: clock rows in bottom half of the device (one bit per clock row, in order from bottom), then one bit for `LLV` tiles (if any)
    - bits 7-11: bottom IOB row
  - Spartan 3A (and 3A DSP):
    - bits 0-5: bottom IOB row
    - bits 11-15: clock rows in bottom half of the device (one bit per clock row, in order from bottom)
- 64 bits for every interconnect row, in order
- 16 special bits, assigned as follows:
  - Spartan 3 and Spartan 3E:
    - bits 0-4: bottom IOB row
    - bits 12-15 (devices without `LLV` tiles): clock rows in top half of the device (one bit per clock row, in order from top)
    - bits 11-15 (devices with `LLV` tiles): clock rows in top half of the device (one bit per clock row, in order from top), then two bits for `LLV` tiles
  - Spartan 3A (and 3A DSP):
    - bits 0-5: bottom IOB row
    - bits 8-10: `LLV` tiles
    - bits 11-15: clock rows in top half of the device (one bit per clock row, in order from top)

Thus, every interconnect tile corresponds to a bitstream tile of 19×64 bits. Such bitstream tiles are shared between the interconnect tiles and their associated primitives.

The IOB row tiles are 19×5 or 19×6 bits, and belong to `IOBS.*` tiles. Likewise, IOB column tiles are 2×64 bits and are belong to `IOBS.*`.  The corner tiles also reuse some tiles in IOB area with the same dimensions.

Blockram data tiles are 76×256 bits (corresponding to 4 interconnect rows). The blockram data frame space to non-BRAM rows is repurposed for interconnect tile storage as described above.

The space in the intersection of IOB columns and IOB or clock rows is unused.

On Spartan 3E, the `LLV` tiles are split in two parts, a 19×1 one in the low special area, and a 19×2 one in the high special area. On Spartan 3A, `LLV` tiles are 19×3 bits.

The `GCLKH` tiles have 19×1 bits.
