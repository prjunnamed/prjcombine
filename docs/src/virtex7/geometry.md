# Device geometry


## General structure

Virtex 7 devices follow the same general structure as [Virtex 6 devices](../virtex6/geometry.md), with minor changes.

Virtex 7 devices are divided into "regions". A region is exactly 50 interconnect tiles high. In addition to 50 interconnect rows, each region has a special HCLK row running through the middle (between rows 24 and 25), which is not counted in the row numbering.

The exact sequence of columns varies with the device. The main available column types are:

- CLBLL column: contains `CLBLL` tiles, one for every interconnect tile.
- CLBLM column: contains `CLBLM` tiles, one for every interconnect tile.
- BRAM column: contains `BRAM` tiles, one for every 5 interconnect tiles (ie. 10 `BRAM` tiles per region)
- DSP column: contains `DSP` tiles, one for every 5 interconnect tiles (ie. 10 `DSP` tiles per region)
- IO column: contains `IOP.*` and `IOS.*` tiles; also contains special `HCLK_IO.*` tiles in the HCLK rows
- CMT column: contains `CMT` tiles; always paired 1-1 with IO columns
- the center column: there is exactly one of those per device; it contains the `CFG` and `XADC` tiles in the center regions, and no primitives in the other regions
- the clock spine column: there is exactly one of those per device; it contains `CLK_*` tiles that distribute global clocks through the device
- GT column: contains `GTX` or `GTH` tiles

Each of the above types of columns is colocated with a single interconnect column. Virtex 7 introduces a change to the internal geometry of columns: on Virtex 6 and below, the interconnect tile was almost always to the left of the associated primitive. On Virtex 7, it is alternating: in even-numbered columns, the interconnect tile is to the right of the associated primitive, while in odd-numbered columns, it is to the left.  This means that the interconnect columns always come in pairs that are next to each other, and such pairs of columns share the last mile clock interconnect lines. The interconnect column consists of:

- `INT` tiles, one per interconnect row (16 per region)
- `INTF` or `INTF.DELAY` tiles, one for every `INT` tile, except for `INT` tiles associated with `CLB*` tiles
  - `INTF` is associated with `BRAM`, `DSP`, `IO`, `CMT`, and `CFG` tiles
  - `INTF.DELAY` is associated with `GTP`, `GTX`, `GTH`, `PCIE2`, `PCIE3`, and `PS` tiles
- `HCLK` tiles, one per region (located in the HCLK row) per *intereconnect column pair* (ie. the `HCLK` tile now takes two columns horizontally)

The clock spine column is now a normal interconnect column unrelated to the center column — they can be in any horizontal position relative to each other. Likewise, the `CFG` center and the `BUFG` complex can have arbitrary vertical positions.

The clock spine column is always the right column of a pair. The center column is always the left column of a pair. There is always exactly one of each per device.

There can be up to two IO columns in the device:

- the left IO column; it is always the left column in the column pair, and the right column in the same pair is a CMT column
- the right IO column; it is always the right column in the column pair, and the left column in the same pair is a CMT column

There can be up to two GT columns in the device: the left GT column and the right GT column. When present, they are always the leftmost and the rightmost column of the device.

The other kinds of columns (CLB, BRAM, DSP) come in varying numbers, locations, and proportions, depending on device size and kind.

Virtex 7 devices have a significantly more irregular structure that previous Virtex devices. The regular grid can have quite a number of holes:

- the configuration center
- the processing system
- the `PCIE` tile
- the `PCIE2` tile
- `GTP` and `GTX` holes in the right IO column
- the middle `GTP` holes
- top and bottom `BRAM` tiles are missing when a BRAM column is the leftmost column of the device

Some Virtex 7 devices are meant to be used as part of a multi-die package. In such devices, a special variant of `INT` tiles is used in the bottom and top regions, where the `LV9` interconnect line is bonded out, and can be connected through the interposer to the `LV9` line of another tile in another device. On devices with GTZ transceivers, these lines are also used to provide input/output connections to the GTZ die.


### IO and transceiver placement

Virtex 7 devices come in several general floorplans:

1. There are two HR IO columns in the device, and they are the leftmost and rightmost column. There are no transceivers.

   This floorplan applies to: `xc7s*` devices that are not just rebadged `xc7a*` devices.

2. Like floorplan 1, but there are `GTP` transceivers in holes in the top right and maybe bottom right corners of the device.

   This floorplan applies to: all `xc7a*` devices except `xc7a200t`.

3. Like floorplan 1, but there are `GTP` transceivers in holes along the bottom and top border of the device, in the middle of the bottom and top regions.

   This floorplan applies to: `xc7a200t`.

4. Like floorplan 1, but the right IO column contains HP IO, and the top right corner has a hole containing `GTX` transceivers.

   This floorplan applies to: `xc7k70t`, `xc7k160t`, `xc7k325t`, `xc7k410t`.

5. The leftmost column is an HR IO column. The rightmost column is a GT column and contains `GTX` transceivers.

   This floorplan applies to: `xc7k355t`, `xc7k480t`.

6. Like floorplan 1, but there is a processing system in the top left corner.

   This floorplan applies to: `xc7z010` and `xc7z020`.

7. Like floorplan 6, but there's a `GTP` transceiver hole in the bottom right corner.

   This floorplan applies to: `xc7z015`.

8. Like floorplan 6, but the right column is HP IO column, and there's a `GTX` transceiver hole in the bottom right corner.

   This floorplan applies to: `xc7z030`, `xc7z045`, `xc7z100`.

9. The leftmost column is the left IO column, and the right IO column is somewhere in the middle. The rightmost column is the right GT column, containing `GTX` or `GTH` transceivers.

   The IO columns contain mostly HP IO, except for some HR IO at the bottom of the left IO column.

   This floorplan applies to: `xc7v585t` (with `GTX`), `xc7vx330t` (with `GTH`)

10. There are two GT columns on the device, and they are the leftmost and rightmost columns. There are also two HP IO columns in the middle.

   This floorplan applies to: `xc7vx485t` (with `GTX`); `xc7vx415t`, `xc7vx690t`, `xc7vx980t` (with `GTH`)

11. The leftmost column is a special `BRAM` column, where the bottom `BRAM` tile and the topmost `BRAM` tile are missing (due to die corner cutting). The rightmost column is a `GTX` GT column. There are two HP IO columns in the middle. The device has special interconnect tiles in the bottom and top regions, for cross-die connections in multi-die devices.

   This floorplan applies to: the `xc7v2000t` die.

12. Like floorplan 10, but there are special interconnect tiles in the bottom and top regions, for cross-die connections in multi-die devices.

   This floorplan applies to: the `xc7vh580t` / `xc7vh870t` / `xc7vx1140t` die.


### Center column and configuration center

The center column on Virtex 7 has no responsibilities other than hosting the configuration center complex.  The full configuration center complex is a hole 6×100 interconnect tiles in size, occupying 6 CLBLL columns to the left of the center column. Very small devices consisting of only one region have a minimal configuration center complex instead, which is a 6×50 hole.

The 6×100 (or 6×50) area where the configuration center is located has no `CLBLL` or `INT` tiles. The `INT` tiles in the center column are used to provide inputs/outputs to the configuration center.

The configuration center has the following layout:

- the bottom 6×50 area contains the `CFG` tile
- the 6×25 area above that contains the bitstream encryption logic; single-region devices don't have this area, and thus don't support bitstream encryption
- the top 6×25 area contains the `XADC` tile; this tile is not present in single-region devices

On devices that are 1 or 2 regions wide, this hole will effectively result in 6 CLBLL columns that have no `CLBLL` tiles at all. While effectively useless, these columns are still counted for bitstream column numbering (and have corresponding bitstream data in uncompressed bitstreams).

Horizontal interconnect lines jump over the skipped `INT` tiles.  Vertical interconnect lines cannot cross the hole, and are bounced back at the top and bottom boundaries.


### Processing system

Some Virtex 7 devices are associated with a processing system, consisting of an ARM-based system-on-chip. They are commonly known as Zynq 7000 devices.

When present, such processing system is located in the top left corner of the device.  It is effectively a 19×100 tile hole. The 18×100 area on the left is completely devoid of any interconnect, while the rightmost column retains its `INT` tiles and is used to connect inputs/outputs to the PS. The columns covered by the PS are always the following, in sequence:

- IO column
- CMT column
- 4×CLBLM column
- BRAM column
- 2× CLBLM column
- DSP column
- 4× CLBLM column
- DSP column
- 2× CLBLM column
- BRAM column
- CLBLL column (the `INT` tiles here are retained and used for connecting PS instead of `CLBLL`)

On the smallest devices, the processing system complex can occupy the entirety of these columns, making them effectively almost nonexistent.  However, these columns still count for bitstream geometry purposes, and contain (dummy) data in uncompressed bitstreams.

No interconnect lines can cross this hole (there is nothing on the other side anyway) — they are bounced back at the bottom and right boundaries.

If the processing system exists, the configuration center always occupies the same two regions as the processing system.


### PCI Express holes

The PCI Express hard logic on Virtex 7 devices doesn't have a dedicated column — it is instead put into a special hole in the middle of the fabric, somewhere close to transceivers.  It comes in two versions:

- the `PCIE_L` / `PCIE_R` tiles, which require a 4×25 hole; the bottom of the hole is aligned to the bottom of a clock region
- the `PCIE3` tile, which requires a 6×50 hole; the bottom (and top) of the hole are aligned to HCLK rows of adjacent regions

The leftmost and rightmost columns of the hole retain their `INT` tiles, while the internal 2×25 or 4×50 area has no interconnect at all. Like with the configuration center holes, horizontal interconnect lines jump across this area, while vertical lines are turned away at top and bottom borders.

The holes cover the following column types:

- `PCIE_L`:
  - BRAM, right column of a pair (has `INT` tiles)
  - CLBLL
  - CLBLM
  - CLBLM (has `INT` tiles)
- `PCIE_R`:
  - CLBLM, right column of a pair (has `INT` tiles)
  - CLBLL
  - CLBLM
  - BRAM (has `INT` tiles)
- `PCIE3`:
  - CLBLM, right column of a pair (has `INT` tiles)
  - CLBLL
  - CLBLM
  - CLBLL
  - CLBLM
  - BRAM (has `INT` tiles)

The `PCIE_L` and `PCIE_R` tiles are effectively mirrored versions of one another. `PCIE_L` and `PCIE_R` appear on devices with `GTP` or `GTX` transceivers, while `PCIE` tiles appear on devices with `GTH` transceivers.


### `GTP` and `GTX` right column holes

On some devices, the right edge of the device is shared by plain IO and `GTX` or `GTP` transceivers. In such case, since transceivers are larger than plain IO, the transceivers effectively become a hole in the grid.

The hole is 7 columns across. They are always:

- BRAM (has `INT` tiles that feed the `GTP` / `GTX` tile)
- CLBLL
- CLBLM
- CLBLL
- CLBLM
- CMT
- IO

The height of the hole is a multiple of 50 tiles (ie. a region). It is always located at either the bottom or the top of the right IO column.  Like with processing system, the interconnect lines are bounced back at the edges, having nowhere to go anyway.

This type of hole has a unique property of affecting bitstream layout: for bitstream geometry purposes, the replaced columns don't exist in the bitstream at all for the affected regions, and the BRAM column is replaced with a GT column in these regions. This does not apply to any other hole type.


### Middle `GTP` holes

On the `xc7a200t` device, `GTP` tiles are, uniquely, inserted in the middle of the bottom and top edges. These are called the middle `GTP` holes. They are 19×50 tiles in size, and contain a single `GTP` tile.

The left middle `GTP` hole covers the following columns:

- CLBLM, the right column of the pair (has `INT` tiles that feed the `GTP` tile)
- CLBLL
- CLBLM
- CLBLL
- CLBLM
- BRAM
- 2×CLBLM
- DSP
- 4×CLBLM
- DSP
- 2×CLBLM
- BRAM
- CLBLL
- CLBLM

The leftmost column retains its `INT` tiles, while the remaining columns have no interconnect.

The right middle `GTP` hole covers the following columns:

- CLBLM (the left column of the pair)
- CLBLL
- BRAM
- 2×CLBLM
- DSP
- 4×CLBLM
- DSP
- 2×CLBLM
- BRAM
- CLBLM
- CLBLL
- CLBLM
- CLBLL
- CLBLM (has `INT` tiles)

The rightmost column retains its `INT` tiles, while the remaining columns have no interconnect.

Despite being in the middle of a region, there are no interconnect lines passing across these holes — horizontal interconnect is bounced back at both left and right edges of the hole. Vertical interconnect is likewise bounced back, but there wouldn't be anywhere for it to go anyway.


## Bitstream geometry

The bitstream is made of frames, which come in two types:

- 0: main area
- 1: BRAM data area

The bitstream is split by region — each frame covers 50 interconnect rows plus the HCLK row, and the frame size is independent of device size.

Frames are identified by their type, region, major and minor numbers. The major number identifies a column (interconnect column or the clock spine), and the minor number identifies a frame within a column. The major numbers are counted separately for each type of frame.

For bitstream purposes, the regions are counted using the top of the `CFG` tile as the origin. Top region 0, if present, is considered to be the region that contains the `XADC` tile, top region 1 is the region above that, and so on. Bottom region 0 is considered to be the region that contains the `CFG` tile, bottom region 1 is the region below that, and so on.

The main area contains all interconnect columns, with major numbers assigned sequentially from 0 on the left, one for each column. If there is a right-edge GT transceiver hole in a given region, it is treated specially: the last BRAM column is instead treated as if it was a GT column within this region, and subsequent columns are entirely omitted. The columns have the following widths:

- CLBLL and CLBLM columns: 36 frames
- BRAM column: 28 frames (unless it is the last BRAM column in a region with right-edge GT hole)
- DSP column: 28 frames
- IO column: 42 frames
- CMT column: 30 frames
- center column: 30 frames
- clock spine column: 30 frames
- GT column: 32 frames (also applies to the last BRAM column in a region with right-edge GT hole)

The BRAM data area contains 128 frames for each BRAM column, in order from left. The major numbers are assigned sequentially for each BRAM column starting from 0.

Each frame is exactly 3232 bits long and has the following structure:

- bits 0-1599: interconnect rows 0 to 24 of the region, 64 bits per row
- bits 1600-1612: ECC
- bits 1613-1631: HCLK row
- bits 1632-3231: interconnect rows 25 to 49 of the region, 64 bits per row

Every interconnect tile thus corresponds to a bitstream tile that is 28×64 to 42×64 bits. The actual interconnect tile is 26×64 bits, occupying the first 26 frames of the column. If `INTF` is present in the tile, it occupies leftover space in frames 0-3. If `INTF.DELAY` tile is present in the tile, it occupies leftover space in frames 0-3, as well as frames 26-27.  The remaining frames, as well as unused space in frames 0-3 and 26-27 where applicable, are used for configuring the associated primitive tile.

The HCLK row has smaller bitstream tiles, 28×19 to 42×19 bits in size.

The BRAM data tiles are 128×320 bits in size (covering the height of 5 interconnect rows). The area at intersection with HCLK rows is unused.


### ECC

TODO: reverse, document