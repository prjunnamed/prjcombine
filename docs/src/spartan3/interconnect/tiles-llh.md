# Horizontal long line splitter tiles

Used for `LLH` tiles that split horizontal long lines, physically located on the primary vertical clock spine. Such tiles are located on the intersection of frames 2-3 of the clock spines bitstream column and interconnect rows, making them 2×64 in size.

## `LLH`

This type of tile is used for all horizontal splitters on Spartan 3E, and for horizontal splitters in general rows on Spartan 3A and Spartan 3A DSP. On Spartan 3A and 3A DSP, the horizontal splitters in IO rows have special tile types.

On Spartan 3E, the bitstream area is also used by `CLKB.S3E` and `CLKT.S3E` in the bottom and top rows.

{{tile spartan3 LLH}}


## `LLH.CLKB.S3A`

This type of tile is used for horizontal splitters in the bottom IO row on Spartan 3A and Spartan 3A DSP. The same bitstream area is also used for `CLKB.S3A`.

{{tile spartan3 LLH.CLKB.S3A}}


## `LLH.CLKT.S3A`

This type of tile is used for horizontal splitters in the top IO row on Spartan 3A and Spartan 3A DSP. The same bitstream area is also used for `CLKT.S3A`.

{{tile spartan3 LLH.CLKT.S3A}}