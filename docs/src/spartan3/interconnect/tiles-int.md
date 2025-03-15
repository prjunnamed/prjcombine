# Interconnect tiles

The interconnect tiles are 19×64 bits. The space on the left is unused by the interconnect tile, and contains data for whatever primitive is associated with the interconnect tile.

## `INT.CLB`

Used with `CLB` tiles and the corner tiles.

{{tile spartan3 INT.CLB}}


## `INT.IOI.S3`

Used with `IOI` tiles on Spartan 3.

{{tile spartan3 INT.IOI.S3}}


## `INT.IOI.S3E`

Used with `IOI` tiles on Spartan 3E.

{{tile spartan3 INT.IOI.S3E}}


## `INT.IOI.S3A.LR`

Used with `IOI` tiles on Spartan 3A / 3A DSP that are on the left or right edge of the device.

{{tile spartan3 INT.IOI.S3A.LR}}


## `INT.IOI.S3A.TB`

Used with `IOI` tiles on Spartan 3A / 3A DSP that are on the top or bottom edge of the device.

{{tile spartan3 INT.IOI.S3A.TB}}


## `INT.BRAM.S3`

Used with `BRAM.S3` tiles on Spartan 3.

{{tile spartan3 INT.BRAM.S3}}


## `INT.BRAM.S3E`

Used with `BRAM.S3E` tiles on Spartan 3E.

{{tile spartan3 INT.BRAM.S3E}}


## `INT.BRAM.S3A.03`

Used with `BRAM.S3A` tiles on Spartan 3A. This interconnect tile is used in rows 0 and 3 of the BRAM.

{{tile spartan3 INT.BRAM.S3A.03}}


## `INT.BRAM.S3A.12`

Used with `BRAM.S3A` tiles on Spartan 3A. This interconnect tile is used in rows 1 and 2 of the BRAM.

{{tile spartan3 INT.BRAM.S3A.12}}


## `INT.BRAM.S3ADSP`

Used with `BRAM.S3ADSP` or `DSP` tiles on Spartan 3A DSP.

{{tile spartan3 INT.BRAM.S3ADSP}}


## `INT.DCM`

Used with `DCM.*` tiles.

{{tile spartan3 INT.DCM}}


## `INT.DCM.S3.DUMMY`

Used for the dummy interconnect tile in DCM holes on Spartan 3 devices with more than 2 BRAM columns. Not associated with any primitive.

{{tile spartan3 INT.DCM.S3.DUMMY}}


## `INT.DCM.S3E.DUMMY`

Used for the dummy interconnect tile in DCM holes on Spartan 3E devices with 2 DCMs. Not associated with any primitive.

{{tile spartan3 INT.DCM.S3E.DUMMY}}
