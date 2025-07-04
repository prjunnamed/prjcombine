# General interconnect

The Spartan 3 general interconnect is essentially a scaled-down version of [Virtex 2 interconnect](../../virtex2/interconnect/README.md).

The general interconnect of Virtex 2 is made of several kinds of similar, but not identical interconnect tiles. The tile types include:

- `INT.CLB`, the interconnect tile associated with [configurable logic blocks](../clb.md) and corner tiles
- `INT.BRAM.*`, the interconnect tile associated with [block RAMs](../bram.md)
- `INT.IOI.*`, the interconnect tile associated with [I/O tiles](../io/README.md)
- `INT.DCM.*`, the interconnect tiles associated with [digital clock managers](../dcm-s3.md)

The various tile types have the same backbone, but differ in the types of input multiplexers they have, mostly clock multiplexers.


## Backbone

The core of the interconnect is made of the following wires, each of which is instantiated once per interconnect tile and is driven by a buffered multiplexer:

- `OMUX0` through `OMUX15`, per-tile output multiplexers. The inputs to those include various primitive outputs in the tile. They serve as single-hop interconnect wires — each `OMUX` wire is also visible in one or two of the (immediately or diagonally) adjacent interconnect tiles:

  | Wire     | Direction     | Wire in tile #1 | Wire in tile #2 |
  | -------- | ------------- | --------------- | --------------- |
  | `OMUX0`  | `S`           | `OMUX0.S`       |
  | `OMUX1`  | `W`, then `S` | `OMUX1.W`       | `OMUX1.WS`      |
  | `OMUX2`  | `E` and `S`   | `OMUX2.E`       | `OMUX2.S`       |
  | `OMUX3`  | `S`, then `E` | `OMUX3.S`       | `OMUX3.SE`      |
  | `OMUX4`  | `S`           | `OMUX4.S`       |
  | `OMUX5`  | `S`, then `W` | `OMUX5.S`       | `OMUX5.SW`      |
  | `OMUX6`  | `W`           | `OMUX6.W`       |
  | `OMUX7`  | `E`, then `S` | `OMUX7.E`       | `OMUX7.ES`      |
  | `OMUX8`  | `E`, then `N` | `OMUX8.E`       | `OMUX8.EN`      |
  | `OMUX9`  | `W` and `N`   | `OMUX9.W`       | `OMUX9.N`       |
  | `OMUX10` | `N`, then `W` | `OMUX10.N`      | `OMUX10.NW`     |
  | `OMUX11` | `N`           | `OMUX11.N`      |
  | `OMUX12` | `N`, then `E` | `OMUX12.N`      | `OMUX12.NE`     |
  | `OMUX13` | `E`           | `OMUX13.E`      |
  | `OMUX14` | `W`, then `N` | `OMUX14.W`      | `OMUX12.WN`     |
  | `OMUX15` | `N`           | `OMUX15.N`      |

  <div class="warning">this table is very similar to, but subtly different from the corresponding Virtex 2 table (the differences are in `OMUX9` and `OMUX13`).</div>

- Double lines going in the cardinal directions, 8 per direction, called `DBL.[EWSN][0-7]`. Each of them has three segments, called `DBL.[EWSN][0-7].[0-2]`, where `.0` is located in the source tile and is driven, `.1` is in the next tile in the relevant direction, and `.2` is in the next tile after that. Some of the lines additionally have a fourth segment:

  - `DBL.W[67].3` is to the north of `DBL.W[67].2`
  - `DBL.E[01].3` is to the south of `DBL.E[01].2`
  - `DBL.S[01].3` is to the south of `DBL.S[01].2`
  - `DBL.N[67].3` is to the north of `DBL.N[67].2`

  Only the `.0` segment is driven. The inputs to the multiplexer include:

  - `OMUX` wires
  - `.1`, `.2` and `.3` segments of other `DBL` wires
  - `.3`, `.6` and `.7` segments of `HEX` wires
  - `OUT.FAN` wires

- Hex lines going in the cardinal directions, 8 per direction, called `HEX.[EWSN][0-7]`. They are analogous to double lines, except they have 7 (or sometimes 8) segments, thus spanning a distance of 6 tiles. The lines with 8th segment include:

  - `HEX.W[67].7` is to the north of `HEX.W[67].6`
  - `HEX.E[01].7` is to the south of `HEX.E[01].6`
  - `HEX.S[01].7` is to the south of `HEX.S[01].6`
  - `HEX.N[67].7` is to the north of `HEX.N[67].6`

  Only the `.0` segment is driven. The inputs to the multiplexer include:

  - `OMUX` wires
  - `.3`, `.6` and `.7` segments of other `HEX` wires
  - `.0`, `.6`, `.12`, `.18` segments of `LV` wires (for vertical lines)
  - `.0`, `.6`, `.12`, `.18` segments of `LH` wires (for horizontal lines)
  - `OUT.FAN` wires

  Some hex lines are associated with various kind of input multiplexers, such that a single hex line can drive all input multiplexers of given type through its `.0` to `.5` segments:

  - `HEX.[SN]0` is associated with `IMUX.SR*`
  - `HEX.[SN]4` is associated with `IMUX.CLK*` and `IMUX.IOCLK*`
  - `HEX.[SN]7` is associated with `IMUX.CE*`

For large-fanout nets or nets that need to span long distances, the interconnect also has long lines that span the whole width or height of the device. There are 24 vertical long lines, `LV`, per an interconnect column, and 24 horizontal long lines, `LH`, per an interconnect row. They are visible as `LV.{0-23}` and `LH.{0-23}` segments in each interconnect tile, in a rotating way: `LH.0` in a given tile is visible as `LH.1` or `LH.23` in the horizontally adjacent tiles.
Only `.0`, `.6`, `.12`, `.18` segments of long wires are actually accessible to a tile — the rest are just passing through.

Each interconnect tile can optionally drive the long line segments accessible to it via buffered multiplexers. The inputs to `LH` driver multiplexers include:

- `OMUX` wires
- `.1` segments of `DBL` wires

The inputs to `LV` driver multiplexers include:

- `OMUX` wires
- `.1` segments of `DBL` wires
- various segments of `HEX.E0` and `HEX.W6` wires

The every-6-tiles nature of long wires combined with existence of hex wires allows for easy distribution of signals everywhere on the FPGA.



## Input multiplexers

Every interconnect tile also contains input multiplexers, which drive the associated primitive inputs. The exact set of available input multiplexers depends on the type of interconnect tile.

The baseline set of input muxes is present in the `INT.CLB` tile:

- `IMUX.CLK[0-3]`: four clock inputs. In CLBs, they correspond to the four `SLICE`\ s. They are multiplexed from:

  - `PULLUP`, a dummy always-1 wire
  - `GCLK0` through `GCLK7`, the [clock interconnect](../clock/README.md) global lines
  - various segments of `DBL` lines
  - any segment of `HEX.S4` and `HEX.N4` lines 

  The `IMUX.CLK` multiplexers have a programmable inverter.

- `IMUX.SR[0-3]`: four set/reset inputs. In CLBs, they correspond to the four `SLICE`\ s. They are multiplexed from:

  - `PULLUP`, a dummy always-1 wire
  - various segments of `DBL` lines
  - any segment of `HEX.S0` and `HEX.N0` lines 

  The `IMUX.SR` multiplexers have a programmable inverter.

- `IMUX.CE[0-3]`: four clock enable inputs. In CLBs, they correspond to the four `SLICE`\ s. They are multiplexed from:

  - `PULLUP`, a dummy always-1 wire
  - various segments of `DBL` lines
  - any segment of `HEX.S7` and `HEX.N7` lines 

  The `IMUX.CE` multiplexers have a programmable inverter.

- `IMUX.FAN.B[XY][0-3]`: 8 fanout inputs, corresponding to `BX` and `BY` in CLBs. They are special in that they can be used both as primitive inputs and as extra routing resources to reach other primitive inputs (specifically, `IMUX.DATA*`). They are multiplexed from:

  - `PULLUP`, a dummy always-1 wire
  - `OMUX` wires
  - various segments of `DBL` lines
  - other `IMUX.FAN.*` wires

- `IMUX.DATA[0-31]`: 32 general data inputs, corresponding to LUT inputs in CLBs. They are multiplexed from the same sources as `IMUX.FAN.*` lines.

The input multiplexers in `INT.BRAM` tiles are essentially subsets of `INT.CLB`. The `INT.BRAM.S3A.03` tiles are specifically missing `IMUX.CLK` and `IMUX.CE` because their bitstream tile space is repurposed for hard multiplier input selection.

The `INT.DCM.*` tiles are a variant of `INT.CLB` with one difference: the `IMUX.CLK*` inputs can be additionally multiplexed from `DCM.CLKPAD[0-3]`, which are direct inputs from clock I/O pads (see [clock interconnect](../clock/README.md)).

The `INT.IOI*` tiles are a variant of `INT.CLB` with the following differences:

- `IMUX.CLK*` are not present
- the `IMUX.IOCLK[0-7]` inputs are added, which are multiplexed from:

  - `PULLUP`, a dummy always-1 wire
  - `GCLK0` through `GCLK7`, the [clock interconnect](../clock/README.md) global lines
  - various segments of `DBL` lines
  - any segment of `HEX.S4` and `HEX.N4` lines 

  The `IMUX.IOCLK*` lines are not invertible on interconnect level, but they have inverters in the `IOI` primitives.


## Primitive outputs

Primitive outputs are wires that go from the various primitives into the general interconnect. The set of available primitive outputs depends on the type of the interconnect tile. The `OUT.FAN*` outputs can be used as inputs to many interconnect multiplexers, while other outputs can only be routed via `OMUX` multiplexers.

The `INT.CLB`, `INT.DCM.*`, and `INT.IOI.*` tiles have the following primitive outputs:

- `OUT.FAN[0-7]`: the main combinational LUT outputs (`X` and `Y`); they have access to many more routing resources than other outputs
- `OUT.SEC[0-15]`: the remaining SLICE outputs (`XQ`, `YQ`, `XB`, `YB`)

Every `OMUX` multiplexer can mux from all `OUT.FAN` and `OUT.SEC` wires.

The `INT.BRAM.*` tiles have the following primitive outputs:

- `OUT.FAN[0-7]`: as above
- `OUT.SEC[4-15]`: as above
- `OUT.HALF[0-3].[0-1]`: they take the place of `OUT.SEC[0-3]`; `.0` outputs are routed only to `OMUX[0-7]` while `.1` outputs are routed only to `OMUX[8-15]`, creating a possible bottleneck


## Terminators

The edges of the device contain special `TERM.[EWSN]` tiles that handle interconnect lines going out-of-bounds:

- `DBL` lines get reflected — eg. northbound lines "bounce off" the top edge and become southbound lines
- `HEX` lines are likewise reflected
- long lines begin and end at `TERM` tiles
- `OMUX` lines going out of bounds are unconnected

The Spartan 3E devices contain special `TERM.BRAM.[SN]` tiles that handle vertical interconnect lines hitting the bottom and top edge of the BRAM column. They work like `TERM.[EWSN]`, but they pass long lines through to the other end of the BRAM column.

The top and bottom of Spartan 3A BRAM columns don't have a terminator — the out-of-bounds interconnect lines just go nowhere and aren't usable.


## Long line splitters

On larger Spartan 3E and Spartan 3A devices, the horizontal clock spine contains `LLV` tiles, which contain programmable buffers that can optionally join the top and bottom segments of `LV` lines (they can buffer each line top-to-bottom, bottom-to-top, or the two segments can be disconnected and used for different signals). Likewise, the primary vertical clock spine contains `LLH` tiles, which can optionally join the left and right segments of `LH` lines.

The area of horizontal clock spine located within the inside of BRAM column doesn't contain `LLV` tiles. This is moot anyway, as the columns don't have enough interconnect tiles for the long lines to actually be useful (each long line only has at most one tap, making them useless).

{{int-basics spartan3}}
