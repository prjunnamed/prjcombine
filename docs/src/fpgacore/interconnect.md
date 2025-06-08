# General interconnect

FPGAcore interconnect is identical to [Spartan 3](../spartan3/interconnect/README.md) with one exception: there are 12 long lines for each orientation instead of 24.

{{int-basics fpgacore}}

## Bitstream — interconnect tiles

The interconnect tiles are 19×64 bits. The space on the left is unused by the interconnect tile, and contains data for whatever primitive is associated with the interconnect tile.

{{tile fpgacore INT.CLB}}
{{tile fpgacore INT.IOI.FC}}
