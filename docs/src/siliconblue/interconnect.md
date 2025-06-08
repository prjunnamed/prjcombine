# General interconnect

The SiliconBlue general interconnect structure involves the following tile types:

- `PLB` and `INT_BRAM`: the "center" tiles of the interconnect, connecting to four neighbouring tiles
- `IOI_W`, `IOI_E`, `IOI_S`, `IOI_N`: the "edge" tiles of the interconnect, connecting to three neighbouring tiles


## `GLOBAL` wires

There are 8 global wires:

- `GLOBAL.[0-7]`: global wires

They are driven by the [global interconnect](global.md).  They can directly drive `IMUX.CLK`, `IMUX.IO.ICLK`, `IMUX.IO.OCLK`, `IMUX.CE`, `IMUX.RST` multiplexers.  In addition to that, every non-`IO` tile has 4 special intermediate wires that can be driven by the `GLOBAL.*` wires and can be used to route them further to the `LOCAL.*` lines:

- `GOUT.[0-3]`: intermediate wires for routing `GLOBAL.*` to `LOCAL.*`; can only be driven by `GLOBAL.*`


## `OUT` wires

Every tile (including the corner tiles, otherwise devoid of interconnect) has 8 output wires, which
are driven by the various bels within the FPGA:

- `OUT.LC[0-7]`: bel output wires; note that, depending on tile type, some of them may actually alias other `OUT` wires, effectively making for fewer than 8 distinct outputs per tile:

  - `PLB`: all 8 wires are distinct; `OUT.LC{i}` corresponds directly to the output of LC `i`
  - `INT_BRAM`: all 8 wires are distinct
  - `IOI_*`: there are 4 distinct wires; `OUT.LC[4-7]` are aliased to `OUT.LC[0-3]`:
    - `OUT.LC[04]` is `IO0.DIN0`
    - `OUT.LC[15]` is `IO0.DIN1`
    - `OUT.LC[26]` is `IO1.DIN0`
    - `OUT.LC[37]` is `IO1.DIN1`
  - corner tiles: there's only one distinct wire (all 8 wires are aliased to each other)

The `OUT` wires are also visible in the 8 directly neighbouring tiles:

- `OUT.LC[0-7].W`: the same as `OUT.LC[0-7]` of tile `(x + 1, y)`
- `OUT.LC[0-7].E`: the same as `OUT.LC[0-7]` of tile `(x - 1, y)`
- `OUT.LC[0-7].S`: the same as `OUT.LC[0-7]` of tile `(x, y + 1)`
- `OUT.LC[0-7].N`: the same as `OUT.LC[0-7]` of tile `(x, y - 1)`
- `OUT.LC[0-7].WS`: the same as `OUT.LC[0-7]` of tile `(x + 1, y + 1)`
- `OUT.LC[0-7].WN`: the same as `OUT.LC[0-7]` of tile `(x + 1, y - 1)`
- `OUT.LC[0-7].ES`: the same as `OUT.LC[0-7]` of tile `(x - 1, y + 1)`
- `OUT.LC[0-7].EN`: the same as `OUT.LC[0-7]` of tile `(x - 1, y - 1)`


## `QUAD` and `LONG` wires

The long-distance backbone of the interconnect consists of the `QUAD` (span-4) and `LONG` (span-12) wires:

- `QUAD.H[0-11].[0-4]`: horizontal length-4 wires

  - `QUAD.Ha.b` in tile `(x, y)` is the same as `QUAD.Ha.(b+1)` in tile `(x + 1, y)`

- `QUAD.V[0-11].[0-4]`, `QUAD.V[0-11].[1-4].W`: vertical length-4 wires

  - `QUAD.Va.b` in tile `(x, y)` is the same as `QUAD.Va.(b+1)` in tile `(x, y + 1)`
  - `QUAD.Va.b.W` in tile `(x, y)` is the same as `QUAD.Va.b` in tile `(x + 1, y)`

- `LONG.H[01].[0-12]`: horizontal length-12 wires

  - `LONG.Ha.b` in tile `(x, y)` is the same as `LONG.Ha.(b+1)` in tile `(x + 1, y)`

- `LONG.V[01].[0-12]`: vertical length-12 wires

  - `LONG.Va.b` in tile `(x, y)` is the same as `LONG.Va.(b+1)` in tile `(x, y + 1)`

The interconnect in `IO` tiles is special:

- in the `IOI_W` and `IOI_E` tiles:
  - there are no vertical `LONG` wires
  - there are only 4 sets of vertical `QUAD` wires (`QUAD.V[0-3].*`)
- in the `IOI_S` and `IOI_N` tiles:
  - there are no horizontal `LONG` wires
  - there are only 4 sets of horizontal `QUAD` wires (`QUAD.H[0-3].*`)

Further, the corner tiles are special: the `QUAD.H*.*` wires of the horizontally adjacent `IOI_[WE]` tile are connected directly to the `QUAD.V*.*` wires of the vertically adjacent `IOI_[SN]` or `PLB` tile.

The `LONG` wires can be driven:

- at many points along the wire: from `OUT` wires
- at endpoints within "center" tiles: from endpoints of other `LONG` wires

The `QUAD` wires can be driven:

- at many points along the wire: from `OUT` wires
- at endpoints within "center" tiles: from endpoints of other `QUAD` wires
- at `.1` (for horizontal wires) or `.3` (for vertical wires): from various segments of `LONG` wires of the same direction
- at `IO` tiles: from other `QUAD` wires


## `LOCAL` wires

Every tile has 32 (`PLB`, `INT_BRAM`) or 16 (`IOI_*`) local wires:

- `LOCAL.[0-3].[0-7]`: local wires

  - `IOI_*` tiles only have `LOCAL.[0-1].[0-7]`

The `LOCAL` wires are an intermediate step between `IMUX.*` wires and other interconnect — every signal routed to `IMUX.*` must first pass through a `LOCAL` wire, except for some multiplexers that can also be directly driven by `GLOBAL` wires.  They can be driven by:

- `QUAD` wires
- `LONG` wires
- `OUT` wires (including ones of the 8 immediately neighbouring tiles)
- `GOUT` wires (not in `IO` tiles)


## `IMUX` wires

`IMUX` wires directly drive bel inputs.  `PLB` and `INT_BRAM` tiles contain the following wires:

- `IMUX.LC[0-7].I[0-3]`: "normal" inputs; in `PLB` tiles, they correspond to LCs in the obvious way
- `IMUX.CLK`: a clock input; freely invertible, can be driven directly by all `GLOBAL` wires
- `IMUX.CE`: a clock enable input; gates the `IMUX.CLK` input, can be driven directly by some `GLOBAL` wires
- `IMUX.RST`: a reset input; can be driven directly by some `GLOBAL` wires

`IOI_*` tiles contain the following wires:

- `IMUX.IO[0-1].DOUT[0-1]`: "normal" inputs, I/O data
- `IMUX.IO[0-1].OE`: "normal" inputs, I/O output enable
- `IMUX.IO.EXTRA`: "normal" input, used for various irregularly placed bels (such as PLLs)
- `IMUX.IO.ICLK` and `IMUX.IO.OCLK`: clock inputs; freely invertible, can be driven directly by all `GLOBAL` wires
- `IMUX.CE`: a clock enable input; gates the `IMUX.IO.ICLK` and `IMUX.IO.OCLK` inputs, can be driven directly by some `GLOBAL` wires

The `IMUX` wires can be driven by `LOCAL` wires.  Some of them can also be driven by `GLOBAL` wires.

If an `IMUX` wire is not driven at all (the `MUX` field in the bitstream is set to `NONE`), it takes on a default value.  The default value is `1` for `IMUX.CE`, `0` for all other `IMUX` wires.


{{int-basics siliconblue}}
