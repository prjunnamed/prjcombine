# Terminator tiles

These tiles are placed at the edges of the device and deal with interconnect lines that go out-of-bounds. The associated bitstream tiles are shared with `IOBS` tiles and primitive data for corner tiles.

## `TERM.W`

Located at the left edge of every interconnect row, this tile is 4×80 bits.

{{tile virtex2 TERM.W}}


## `TERM.E`

Located at the right edge of every interconnect row, this tile is 4×80 bits.

{{tile virtex2 TERM.E}}


## `TERM.S`

Located at the bottom edge of every interconnect column, this tile is 22×12 bits.

{{tile virtex2 TERM.S}}


## `TERM.N`

Located at the top edge of every interconnect column, this tile is 22×12 bits.

{{tile virtex2 TERM.N}}
