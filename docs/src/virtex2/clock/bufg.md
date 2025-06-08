# Global buffers

TODO: document


## Bitstream

The `CLKB.*` tiles use two bitstream tiles:

- tile 0: 4×80 tile located in the clock spine column, in the bits corresponding to the bottom interconnect row
- tile 1: 4×16 tile located in the clock spine column, in the bits corresponding to the low special area (used for bottom `IOB` tiles and clock rows in normal columns)

The `CLKT.*` tiles use two bitstream tiles:

- tile 0: 4×80 tile located in the clock spine column, in the bits corresponding to the top interconnect row
- tile 1: 4×16 tile located in the clock spine column, in the bits corresponding to the high special area (used for top `IOB` tiles and clock rows in normal columns)

Each tile comes in three variants:

- `*.V2`: used on Virtex 2 devices
- `*.V2P`: used on Virtex 2 Pro devices with `GT` transceivers
- `*.V2PX`: used on Virtex 2 Pro X devices with `GT10` transceivers

{{tile virtex2 CLKB.V2}}
{{tile virtex2 CLKB.V2P}}
{{tile virtex2 CLKB.V2PX}}
{{tile virtex2 CLKT.V2}}
{{tile virtex2 CLKT.V2P}}
{{tile virtex2 CLKT.V2PX}}
{{tile virtex2 CLKC}}

