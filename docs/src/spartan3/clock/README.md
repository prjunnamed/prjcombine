# Clock interconnect

The dedicated clock interconnect on Spartan 3 includes:

- three special clock distribution columns, dividing the device into four roughly equally-sized parts
  - primary vertical clock spine, through the middle of the device
  - two secondary vertical clock spines (the west one and east one)
- a horizontal clock spine, through the middle of the device
- two or more horizontal clock rows
- the `CLKB*` tile on the south end of the primary vertical clock spine
  - 4 `BUFGMUX` global buffers
  - 4 nearby I/O pads are considered "dedicated clock inputs" and have direct connections to the `BUFGMUX` primitives and `DCM`s on the same side of the device
  - the `DCM`s on the same side of the device have dedicated connections to `BUFGMUX` inputs
- the `CLKT*` tile on the south end of the primary vertical clock spine (same contents as `CLKB*`)
- 8 global clocks (4 driven by `CLKB`, 4 driven by `CLKT`)
- two `GCLKVM` tiles, at intersections of the secondary vertical spines and the horizontal clock spine
  - programmable buffers for each of the 8 global clocks, separately for the north and south directions (dividing the device into four quarters with independently gated clocks)
- a `GCLKH` tile at the intersection of every interconnect column and clock row
  - programmable buffers for each of the 8 global clocks, separately for the north and south directions
- up to 4 `DCM`s, near the corners of the device at the ends of the outermost BRAM columns
  - `xc3s50` has 2 `DCM`s, at the southwest and northwest corners
  - remaining devices have 4 `DCM`s, one at each corner

Spartan 3E and 3A devices include the following enhancements:

- the `CLKL*` tile on the west end of the horizontal clock spine
  - 8 `BUFGMUX` global buffers, capable of feeding only the west half of the device
  - a `PCILOGICSE` hard PCI logic primitive (unrelated to the clock interconnect, just colocated)
  - 8 nearby I/O pads are considered "dedicated clock inputs" and have direct connections to the `BUFGMUX` primitives and the `DCM`s associated with the `CLKL*` tile (if any)
  - the associated `DCM`s (if any) have direct connections to `BUFGMUX` inputs
- the `CLKR*` tile on the east end of the horizontal clock spine
  - same contents as `CLKL*`; the contained `BUFGMUX` primitives are capable of feeding only the east half of the device
- 24 `BUFGMUX` primitives total (8 in `CLKB*` and `CLKT*` capable of feeding the whole device, 16 in `CLKL*` and `CLKR*` capable of feeding only half of the device)
- the device is divided into four clock regions (southwest, northwest, southeast, northeast) by the horizontal clock spine and the primary vertical clock spine
  - each clock region has 8 global clocks
  - each global clock can be driven from the corresponding clock from either the primary vertical clock spine (ie. `CLKB*` or `CLKT*` `BUFGMUX`) or the `CLKL`/`CLKR` tile (`CLKL` for the west clock regions, `CLKR` for the east clock regions)
- the `GCLKVM` tiles contain the multiplexers feeding the clock regions
- 2-8 `DCM`s, located close to (and associated with) `CLKB`/`CLKT`/`CLKL`/`CLKR` tiles
  - `xc3s100e`:
    - one `DCM` for `CLKB`
    - one `DCM` for `CLKT`
  - `xc3s50a`:
    - two `DCM`s for `CLKT`
  - `xc3s250e`, `xc3s500e`, `xc3s200a`, `xc3s400a`:
    - two `DCM`s for `CLKB`
    - two `DCM`s for `CLKT`
  - remaining (larger) devices:
    - two `DCM`s for `CLKB`
    - two `DCM`s for `CLKT`
    - two `DCM`s for `CLKL`
    - two `DCM`s for `CLKR`

The `xc3s50a` is an extra-small device with an unusual variant of the Spartan 3E clock interconnect structure:

- there are no secondary vertical clock spines
- there is only one clock row, colocated with the hroizontal clock spine
- there are only two clock regions: the west one and the east one
  - the 8 clocks in the west region are multiplexed from the `CLKL` clocks and the `CLKB`+`CLKT` clocks
  - the 8 clocks in the east region are multiplexed from the `CLKR` clocks and the `CLKB`+`CLKT` clocks
- the `GCLKVM` tiles are gone and replaced with a central `CLKC_50A` tile that multiplexes the clocks to both clock regions