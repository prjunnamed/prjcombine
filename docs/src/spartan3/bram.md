# Block RAM

See [Virtex 2 documentation](../virtex2/bram.md) documentation for functional description.


## Bitstream

The data for a BRAM is spread across 5 bitstream tiles:

- tiles 0-3: the 4 bitstream tiles that are shared with the `INT.BRAM.*` interconnect tiles (starting from the bottom)
- tile 4: the dedicated BRAM data tile located in the BRAM data area; this tile is 76×256 bits; it contains solely the `DATA` and `DATAP` attributes


## `BRAM.S3`

This tile is used on Spartan 3 devices.

{{tile spartan3 BRAM.S3}}


## `BRAM.S3E`

This tile is used on Spartan 3E devices.

{{tile spartan3 BRAM.S3E}}


## `BRAM.S3A`

This tile is used on Spartan 3A devices.

{{tile spartan3 BRAM.S3A}}


## `BRAM.S3ADSP`

This tile is used on Spartan 3A DSP devices.

{{tile spartan3 BRAM.S3ADSP}}


## Default option values

{{devdata spartan3 bram-opts}}
