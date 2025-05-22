# Global interconnect

The SiliconBlue devices have 8 global nets, `GLOBAL.[0-7]`, that can be used to distribute clocks, clock enables, resets, and other high-fanout signals.


## Global net sources

Each global net can be driven by one of two possible sources:

- fabric (driven from an `IMUX.IO.EXTRA` wire in a particular `IO` tile)
- IO, which can resolve to:

  - `HSOSC` (for `GLOBAL.4` on `iCE40R04`)
  - `LSOSC` (for `GLOBAL.5` on `iCE40R04`)
  - `HFOSC` (for `GLOBAL.4` on `iCE40T0*`)
  - `LFOSC` (for `GLOBAL.5` on `iCE40T0*`)
  - PLL output (effectively overrides an IO pad or two when enabled)
  - direct input from IO pad (if none of the above apply)

The rough locations of global net sources for `iCE65*` and `iCE40P0*` are:

| wire       | fabric source               | IO pad                      | PLL output |
| ---------- | --------------------------- | --------------------------- | ---------- |
| `GLOBAL.0` | south edge middle (eastern) | east edge middle (southern) | -          |
| `GLOBAL.1` | north edge middle (eastern) | west edge middle (southern) | -          |
| `GLOBAL.2` | east edge middle (northern) | north edge middle (eastern) | north B    |
| `GLOBAL.3` | west edge middle (northern) | south edge middle (eastern) | south B    |
| `GLOBAL.4` | north edge middle (western) | west edge middle (northern) | -          |
| `GLOBAL.5` | south edge middle (western) | east edge middle (northern) | -          |
| `GLOBAL.6` | west edge middle (southern) | south edge middle (western) | south A    |
| `GLOBAL.7` | east edge middle (southern) | north edge middle (western) | north A    |

And for `iCE40R04` and `iCE40T0*`:

| wire       | fabric source               | IO pad                      | PLL output |
| ---------- | --------------------------- | --------------------------- | ---------- |
| `GLOBAL.0` | south edge middle (eastern) | south edge east quarter     | -          |
| `GLOBAL.1` | north edge middle (eastern) | south edge west quarter     | -          |
| `GLOBAL.2` | north edge east quarter     | north edge middle (eastern) | north B    |
| `GLOBAL.3` | north edge west quarter     | south edge middle (eastern) | south B    |
| `GLOBAL.4` | north edge middle (western) | `HSOSC` / `HFOSC`           | -          |
| `GLOBAL.5` | south edge middle (western) | `LSOSC` / `LFOSC`           | -          |
| `GLOBAL.6` | south edge west quarter     | south edge middle (western) | south A    |
| `GLOBAL.7` | south edge east quarter     | north edge middle (western) | north A    |

The exact locations of fabric and IO pad sources can be obtained from the `GB{i}_FABRIC` and `GB{i}_IO` extra nodes in the database.

The 8 muxes selecting the active global net sources are located in a special `GB_OUT` tile located in the very center of the device.


## Column buffers

On most SiliconBlue devices, the global nets are not connected directly to the `IMUX` and `GOUT` multiplexers within interconnect tiles — they pass through "column buffers" instead.  Such column buffers have to be enabled via bitstream bits when necessary.

Every interconnect column is partitioned into two or three "clock columns".  Each clock column is further partitioned into two "clock sub-columns" around the middle.  The middle row of the clock column contains two sets of column buffers, driving the `GLOBAL` wires in the sourthern and northern sub-columns.

The `rows_colbuf` field of a chip in the database describes how columns are divided into clock columns.  Every clock column is defined by three rows:

- the middle row `M`
- the start row `B`
- the end row `T`

Rows `B..M` make up the southern clock sub-column of the given clock column, and rows `M..E` make up the northern clock sub-column.  The column buffers driving the southern sub-column are located in row `M - 1`, except for BRAM columns on `iCE65L01` and `iCE40P01` where they are located in row `M - 2` instead.  The column buffers driving the northern sub-column are located in row `M`.

The following devices do not have column buffers:

- `iCE65L04`
- `iCE65L08`
- `iCE65P04`
- `iCE40P03`

On these devices, the global nets are permanently connected to all consumers, and the column buffer enable bits don't have to be set.
