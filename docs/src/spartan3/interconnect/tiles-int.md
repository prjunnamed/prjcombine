# Interconnect tiles

The interconnect tiles are 19×64 bits. The space on the left is unused by the interconnect tile, and contains data for whatever primitive is associated with the interconnect tile.

## `INT_CLB`

Used with `CLB` tiles and the corner tiles.

{{tile spartan3 INT_CLB}}

## `INT_CLB_FC`

Used with `CLB` tiles and the corner tiles — FPGAcore variant.

{{tile spartan3 INT_CLB_FC}}


## `INT_IOI_S3`

Used with `IOI` tiles on Spartan 3.

{{tile spartan3 INT_IOI_S3}}


## `INT_IOI_FC`

Used with `IOI` tiles on FPGAcore.

{{tile spartan3 INT_IOI_FC}}


## `INT_IOI_S3E`

Used with `IOI` tiles on Spartan 3E.

{{tile spartan3 INT_IOI_S3E}}


## `INT_IOI_S3A_WE`

Used with `IOI` tiles on Spartan 3A / 3A DSP that are on the left or right edge of the device.

{{tile spartan3 INT_IOI_S3A_WE}}


## `INT_IOI_S3A_SN`

Used with `IOI` tiles on Spartan 3A / 3A DSP that are on the top or bottom edge of the device.

{{tile spartan3 INT_IOI_S3A_SN}}


## `INT_BRAM_S3`

Used with `BRAM_S3` tiles on Spartan 3.

{{tile spartan3 INT_BRAM_S3}}


## `INT_BRAM_S3E`

Used with `BRAM_S3E` tiles on Spartan 3E.

{{tile spartan3 INT_BRAM_S3E}}


## `INT_BRAM_S3A_03`

Used with `BRAM_S3A` tiles on Spartan 3A. This interconnect tile is used in rows 0 and 3 of the BRAM.

{{tile spartan3 INT_BRAM_S3A_03}}


## `INT_BRAM_S3A_12`

Used with `BRAM_S3A` tiles on Spartan 3A. This interconnect tile is used in rows 1 and 2 of the BRAM.

{{tile spartan3 INT_BRAM_S3A_12}}


## `INT_BRAM_S3ADSP`

Used with `BRAM_S3ADSP` or `DSP` tiles on Spartan 3A DSP.

{{tile spartan3 INT_BRAM_S3ADSP}}


## `INT_DCM`

Used with `DCM_*` tiles.

{{tile spartan3 INT_DCM}}


## `INT_DCM_S3_DUMMY`

Used for the dummy interconnect tile in DCM holes on Spartan 3 devices with more than 2 BRAM columns. Not associated with any primitive.

{{tile spartan3 INT_DCM_S3_DUMMY}}


## `INT_DCM_S3E_DUMMY`

Used for the dummy interconnect tile in DCM holes on Spartan 3E devices with 2 DCMs. Not associated with any primitive.

{{tile spartan3 INT_DCM_S3E_DUMMY}}
