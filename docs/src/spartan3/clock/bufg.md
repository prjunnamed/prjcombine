# Primary global buffers

TODO: document


## Bitstream — bottom tiles

The `CLKB.*` tiles use two bitstream tiles:

- tile 0: 1×64 (Spartan 3, 3E) or 2×64 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the bottom interconnect row
- tile 1: 1×16 (Spartan 3, 3E) or 2×16 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the low special area (used for bottom `IOB` tiles and clock rows in normal columns)

On Spartan 3A devices that have long line splitters, bitstream tile 0 is shared with the `LLH.CLKB.S3A` tile.


### `CLKB.S3`

This tile is used on Spartan 3.

{{tile spartan3 CLKB.S3}}


### `CLKB.S3E`

This tile is used on Spartan 3E.

{{tile spartan3 CLKB.S3E}}


### `CLKB.S3A`

This tile is used on Spartan 3A and 3A DSP.

{{tile spartan3 CLKB.S3A}}


## Bitstream — top tiles

The `CLKT.*` tiles use two bitstream tiles:

- tile 0: 1×64 (Spartan 3, 3E) or 2×64 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the top interconnect row
- tile 1: 1×16 (Spartan 3, 3E) or 2×16 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the high special area (used for top `IOB` tiles and clock rows in normal columns)

On Spartan 3A devices that have long line splitters, bitstream tile 0 is shared with the `LLH.CLKT.S3A` tile.


### `CLKT.S3`

This tile is used on Spartan 3.

{{tile spartan3 CLKT.S3}}


### `CLKT.S3E`

This tile is used on Spartan 3E.

{{tile spartan3 CLKT.S3E}}


### `CLKT.S3A`

This tile is used on Spartan 3A and 3A DSP.

{{tile spartan3 CLKT.S3A}}