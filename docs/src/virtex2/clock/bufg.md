# Global buffers

TODO: document


## Bitstream — bottom tiles

The `CLKB.*` tiles use two bitstream tiles:

- tile 0: 4×80 tile located in the clock spine column, in the bits corresponding to the bottom interconnect row
- tile 1: 4×16 tile located in the clock spine column, in the bits corresponding to the low special area (used for bottom `IOB` tiles and clock rows in normal columns)


### `CLKB.V2`

This tile is used on Virtex 2 devices.

{{tile virtex2 CLKB.V2}}

### `CLKB.V2P`

This tile is used on Virtex 2 Pro devices.

{{tile virtex2 CLKB.V2P}}

### `CLKB.V2PX`

This tile is used on Virtex 2 Pro X devices.

{{tile virtex2 CLKB.V2PX}}


## Bitstream — top tiles

The `CLKT.*` tiles use two bitstream tiles:

- tile 0: 4×80 tile located in the clock spine column, in the bits corresponding to the top interconnect row
- tile 1: 4×16 tile located in the clock spine column, in the bits corresponding to the high special area (used for top `IOB` tiles and clock rows in normal columns)


### `CLKT.V2`

This tile is used on Virtex 2 devices.

{{tile virtex2 CLKT.V2}}


### `CLKT.V2P`

This tile is used on Virtex 2 Pro devices.

{{tile virtex2 CLKT.V2P}}


### `CLKT.V2PX`

This tile is used on Virtex 2 Pro X devices.

{{tile virtex2 CLKT.V2PX}}

