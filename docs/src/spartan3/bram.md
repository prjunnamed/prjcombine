# Block RAM

See [Virtex 2 documentation](../virtex2/bram.md) documentation for functional description.


## Bitstream

The data for a BRAM is spread across 5 bitstream tiles:

- tiles 0-3: the 4 bitstream tiles that are shared with the `INT_BRAM_*` interconnect tiles (starting from the bottom)
- tile 4: the dedicated BRAM data tile located in the BRAM data area; this tile is 76×256 bits; it contains solely the `DATA` and `DATAP` attributes


## `BRAM_S3`

This tile is used on Spartan 3 devices.

{{tile spartan3 BRAM_S3}}


## `BRAM_S3E`

This tile is used on Spartan 3E devices.

{{tile spartan3 BRAM_S3E}}


## `BRAM_S3A`

This tile is used on Spartan 3A devices.

{{tile spartan3 BRAM_S3A}}


## `BRAM_S3ADSP`

This tile is used on Spartan 3A DSP devices.

{{tile spartan3 BRAM_S3ADSP}}


## Default option values

{{devdata spartan3 bram-opts}}
