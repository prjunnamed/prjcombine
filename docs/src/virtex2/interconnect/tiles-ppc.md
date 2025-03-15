# PowerPC hole tiles

These tiles are located inside the PowerPC holes, and serve a similar function to the terminator tiles.

## `PPC.W`

This tile is located on the right of every interconnect row interrupted by the PowerPC hole. It reuses the bitstream tile of the rightmost `INT.PPC` tile of that row.

The interconnect signals prefixed with `0` refer to signals in the rightmost `INT.PPC` tile of the row.  The interconnect signals prefixed with `1` refer to signals in the leftmost `INT.PPC` tile of the row.

{{tile virtex2 PPC.W}}


## `PPC.E`

This tile is located on the left of every interconnect row interrupted by the PowerPC hole. It reuses the bitstream tile of the leftmost `INT.PPC` tile of that row.

The interconnect signals prefixed with `0` refer to signals in the leftmost `INT.PPC` tile of the row.  The interconnect signals prefixed with `1` refer to signals in the rightmost `INT.PPC` tile of the row.

{{tile virtex2 PPC.E}}


## `PPC.S`

This tile is located on the top of every interconnect column interrupted by the PowerPC hole. It uses the bitstream tile corresponding to the interconnect tile below the topmost `INT.PPC` tile of the column (which is otherwise empty, as it doesn't contain an `INT.*` tile).

The interconnect signals prefixed with `0` refer to signals in the topmost `INT.PPC` tile of the row.  The interconnect signals prefixed with `1` refer to signals in the bottommost `INT.PPC` tile of the row.

{{tile virtex2 PPC.S}}


## `PPC.N`

This tile is located on the bottom of every interconnect column interrupted by the PowerPC hole. It uses the bitstream tile corresponding to the interconnect tile above the bottommost `INT.PPC` tile of the column (which is otherwise empty, as it doesn't contain an `INT.*` tile).

The interconnect signals prefixed with `0` refer to signals in the bottommost `INT.PPC` tile of the row.  The interconnect signals prefixed with `1` refer to signals in the topmost `INT.PPC` tile of the row.

{{tile virtex2 PPC.N}}

