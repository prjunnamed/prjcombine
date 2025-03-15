# Clock interconnect

The dedicated clock interconnect on Virtex 2 includes:

- the vertical clock spine through the middle of the device, including:
  - the `CLKB*` tile on the south edge of the device
    - 8 `BUFGMUX` global buffer primitives (with clock multiplexer functionality)
    - 8 nearby I/O pads are considered "dedicated clock inputs" and have direct connections to the `BUFGMUX` primitives and `DCM`s on the same side of the device
    - the `DCM`s on the same side of the device have direct connections to the `BUFGMUX` inputs
  - the `CLKT*` tile on the north edge of the device, with the same contents as `CLKB*`
  - 2 or more `GCLKC*` clock distribution tiles
    - each `GCLKC*` tile has two associated clock regions: west and east
    - each clock region has 8 global clocks
      - each global clock can be driven from the corresponding `BUFGMUX` in either the `CLKB*` tile or the `CLKT*` tile
- clock distrubution rows, one for each `GCLKC*` tile, each containing:
  - one `GCLKH` tile for each interconnect column, containing:
    - clock buffers for the 8 global clocks, for distribution north/south within the column
    - global control signal control
- `DCMCONN*` tiles along the south and north edges, one for each `DCM` and multi-gigabit transceiver, supplying them with inputs from dedicated clock inputs and multiplexing their outputs onto a bus going to the `CLKB*`/`CLKT*` tiles
- 4-12 `DCM`s, located along the south and north edges, at the ends of the BRAM columns