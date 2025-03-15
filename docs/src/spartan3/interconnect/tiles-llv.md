# Vertical long line splitter tiles

Used for `LLV` tiles that split vertical long lines. Physically located on the horizontal clock spine.


## `LLV.S3E`

On Spartan 3E, the data for `LLV` tiles is split into two bitstream tiles: a 19×1 tile that lives in the bottom special area of every interconnect column, and a 19×2 tile that lives in the top special area of every interconnect column.

{{tile spartan3 LLV.S3E}}


## `LLV.S3A`

On Spartan 3A and Spartan 3A DSP, the data for `LLV` tiles lives in 19×3 bitstream tiles in the top special area of every interconnect column.

{{tile spartan3 LLV.S3A}}
