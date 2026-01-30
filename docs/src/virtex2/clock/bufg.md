# Global buffers

TODO: document


## Bitstream

The `CLK_S` tile uses two bitstream tiles:

- tile 0: 4×80 tile located in the clock spine column, in the bits corresponding to the bottom interconnect row
- tile 1: 4×16 tile located in the clock spine column, in the bits corresponding to the low special area (used for bottom `IOB` tiles and clock rows in normal columns)

The `CLK_N` tile uses two bitstream tiles:

- tile 0: 4×80 tile located in the clock spine column, in the bits corresponding to the top interconnect row
- tile 1: 4×16 tile located in the clock spine column, in the bits corresponding to the high special area (used for top `IOB` tiles and clock rows in normal columns)

{{tile virtex2 CLK_S}}
{{tile virtex2 CLK_N}}

