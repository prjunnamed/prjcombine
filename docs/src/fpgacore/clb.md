# Logic block

The CLB is identical to [Spartan 3](../spartan3/clb.md).


## Bitstream

The data for a CLB is located in the same bitstream tile as the associated `INT.CLB` tile.

{{tile fpgacore CLB}}


## `RESERVED_ANDOR`

TODO: wtf is this even


## `RANDOR`

This tile overlaps `IOI.*`.

{{tile fpgacore RANDOR}}


## `RANDOR_INIT`

This tile overlaps top-left interconnect tile.

{{tile fpgacore RANDOR_INIT}}
