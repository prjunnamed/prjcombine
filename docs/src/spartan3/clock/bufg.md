# Primary global buffers

TODO: document


## Bitstream — south tiles

The `CLK_S_*` tiles use two bitstream tiles:

- tile 0: 1×64 (Spartan 3, 3E) or 2×64 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the bottom interconnect row
- tile 1: 1×16 (Spartan 3, 3E) or 2×16 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the low special area (used for bottom `IOB` tiles and clock rows in normal columns)

On Spartan 3A devices that have long line splitters, bitstream tile 0 is shared with the `LLH_S_S3A` tile.


## `CLK_S_S3`

This tile is used on Spartan 3.

{{tile spartan3 CLK_S_S3}}


## `CLK_S_FC`

This tile is used on FPGAcore.

{{tile spartan3 CLK_S_FC}}


## `CLK_S_S3E`

This tile is used on Spartan 3E.

{{tile spartan3 CLK_S_S3E}}


## `CLK_S_S3A`

This tile is used on Spartan 3A and 3A DSP.

{{tile spartan3 CLK_S_S3A}}


## Bitstream — north tiles

The `CLK_N_*` tiles use two bitstream tiles:

- tile 0: 1×64 (Spartan 3, 3E) or 2×64 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the top interconnect row
- tile 1: 1×16 (Spartan 3, 3E) or 2×16 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the high special area (used for top `IOB` tiles and clock rows in normal columns)

On Spartan 3A devices that have long line splitters, bitstream tile 0 is shared with the `LLH_N_S3A` tile.


## `CLK_N_S3`

This tile is used on Spartan 3.

{{tile spartan3 CLK_N_S3}}


## `CLK_N_FC`

This tile is used on FPGAcore.

{{tile spartan3 CLK_N_FC}}


## `CLK_N_S3E`

This tile is used on Spartan 3E.

{{tile spartan3 CLK_N_S3E}}


## `CLK_N_S3A`

This tile is used on Spartan 3A and 3A DSP.

{{tile spartan3 CLK_N_S3A}}
