# FPGA grids and interconnect

This document describes the Project Combine model for FPGA tile grid structure and interconnect

The provided databases include an *interconnect database* for each target, which describes tile types, wire types, wire connections, and other objects that can be found in the devices.  An *expanded grid* structure describes how they are instantiated combined together within a given *device*.  Both *interconnect database* and *expanded grid* are target-independent structures.

Project Combine provides per-target Rust crates that construct an *expanded device* given the *device*.  The *expanded device* is a target-dependent structure that contains the *expanded grid* as one of its fields.

TODO: the currently implemented interconnect model differs from the one described here in multiple places (mostly in naming, but there are more complex mismatches as well).  This document describes the *intended* model — the plan is to change the code to reflect this document.


## General notes and scope

The generic FPGA grid and interconnect system described here is designed for the objects that would be subject to generic place and route.  Thus, it is only used to describe general interconnect and the actual FPGA grid.  In particular, the following features are considered out of scope and are (usually) handled by target-specific means instead:

1. Dedicated interconnect (connections between predefined bel pins that are not routable to/from the general interconnect) and any bel pins using only dedicated interconnect.  This includes:

   - carry or cascade chains of all sorts
     - if the carry chain can be sourced from the general interconnect at the start, or sunk into the general interconnect at the end, such entry / exit points are represented as special bels
   - direct connections to and from dedicated or semi-dedicated I/O pads, such as:
     - dedicated clock input pads
     - dedicated PLL input or output pads
     - designated I/O pads for a hard logic block, such as a memory controller, SPI or I²C controller, PCI logic, and so on
   - fixed connections between bels that can be "combined together" and used as a larger bel, such as:
     - wide mux connections between SLICEs
     - SLICEs that can be combined together to form a large LUT-RAM
     - two 18kbit blockRAMs that can be combined to form a 36kbit blockRAM

   From a placer's point of view, dedicated interconnect behaves like a placement constraint (as opposed to general interconnect, which imposes no placement constraints other than those implied by timing requirements).  From a router's point of view, dedicated interconnect should be handled by dedicated router, as the routing resources involved are independent from the bulk of work involved in general interconnect routing, and routing failures tend to result from violations of relatively simple constraints that can be communicated to the user.  Thus, there is no need to represent it in the same data structures.

2. Global / clock interconnect, except for the "last mile" where it enters the general interconnect muxes.

   Clock interconnect tends to have special properties:

   - is usually closely tied to clock enable / divider / multiplexer bels
   - has low-skew properties that make the usual "Euclidean distance" approximate metric inapplicable for timing
   - is scarce and should be allocated intentionally

   These properties usually make it inconvenient to handle clock interconnect by the same means as general interconnect.

3. Any sort of circuitry that's outside of the interconnect grid, except as needed to represent its connections to the general interconnect.

   This includes things such as:

   - the configuration logic, on targets where it doesn't have any routable pins
   - on hard SoC devices such as Zynq, any sort of CPU cores, peripherials, or dedicated I/O pads
   - the Virtex 7 GTZ transceivers
   - on Versal:
     - the NoC horizontal rows on the north and south edges of the device
     - XPIO banks and their associated logic
     - DDR and HBM memory controllers
     - the AI engine grid

   This sort of circuitry is exposed via target-specific means, as applicable.

Further, grid columns and rows are intended to correspond 1-to-1 with columns and rows of interconnect tiles (tiles containing general interconnect muxes).  This means:

- if the tile grid, as understood by vendor tools, has separate columns for interconnect and actual logic associated with that interconnect (such as later Xilinx FPGAs), we disregard this separation and treat both of those together as a single column
  - for the special case of Ultrascale and Versal, which have independent logic columns on both sides of an interconnect column, we treat each vendor interconnect column as *two* grid columns (so that bels on both sides gets separate X coordinates)
- any sort of special columns and rows (clock distribution, edge terminators, I/O buffers, ...) in between "normal" columns and rows or at the edges are not assigned their own grid coordinates; any sort of logic and wiring within is considered to be part of a nearby interconnect column or row:
  - for a special column in between two interconnect columns, the column to the east is used
  - for a special row in between two interconnect rows, the row to the north is used
  - for a special column or row at the edge, the outermost column or row in the given direction is used

The above rules aren't completely hard, and are mostly based on vibes and on how hard it would be to squash a given thing to match the grid model.


## The grid

An *expanded grid* is made of one or more *die*, which are identified by `DieId`.  The target-specific *chip* structure generally describes a particular model of a *die*.  A *die* is a 2D array of *cells*, organized in *columns* and *rows*.  A *cell* is identified by its `(DieId, ColId, RowId)` coordinates.  There is no requirement that the *die* within the device have the same dimensions.

The directions within a die are:

- *west*: towards smaller column index (usually to the left when displayed on screen)
- *east*: towards larger column index (usually to the right when displayed on screen)
- *south*: towards smaller row index (usually to the bottom when displayed on screen)
- *north*: towards larger row index (usually to the top when displayed on screen)

The directions are always relative to the silicon die, not the package.  Thus, for flip-chip packages, package left tends to be east.  Note that for multi-die devices the orientation of individual die may vary (ie. some of them may be rotated).


## Wires, wire segments, connectors

The general interconnect is made of *wires* and *muxes*, which connect wires together.  A *wire* is a physical net, or a set of nets that are always logically connected together (through always-on buffers or similar means).  A *wire segment* is a part of wire contained in a particular *cell*.

The interconnect database contains a list of wire segments present in each cell, indexed by `WireId`.

A *wire segment* in the device is identified by a tuple of `(DieId, ColId, RowId, WireId)`, which is also known as `WireCoord`.  A *wire* is identified by choosing a *canonical wire segment* to represent it.  If the wire can only be driven in one cell, the single segment that can be driven is considered the canonical one.  Otherwise, the canonical segment is chosen by whatever means are convenient to get the device to fit in the interconnect model — it tends to be either an endpoint or a midpoint of the wire.

For every `WireId`, the database has:

- a name
- the segment's *kind* and kind-dependent information; the segment kind describes, among other things, how to determine how to find the *canonical wire segment* given a *wire segment*

The segment kinds are:

- *tie to 0* and *tie to 1*:
  - permanently driven to a given constant value
  - this segment is the canonical wire segment
  - no other segment can be driven
- *pullup tie*:
  - weakly driven to a const-1 value
  - generally identical to *tie to 1*, except for edge cases involving partial reconfiguration
- *regional*:
  - has an associated `RegionSlotId`
  - every *cell* in the *expanded grid* has a map from `RegionSlotId` to `(DieId, ColId, RowId)`; the canonical wire segment is the same `WireId` at the given coordinates
  - used for clock networks and other cases where a wire is widely distributed in a way that's not easily described by the branch construct
- *mux output*:
  - this segment is the canonical wire segment
  - this segment is driven by interconnect muxes
  - no other segment can be driven
  - other segments of this wire, if any, will be of type *branch*
- *logic output*:
  - this segment is the canonical wire segment
  - this segment is driven by a bel
  - no other segment can be driven
  - other segments of this wire, if any, will be of type *branch*
- *test output*:
  - mostly like *logic output*, with the following differences:
  - the wire segment and any associated bel outputs are considered to be test-only (should not be used except when testing the device itself)
  - the output may not be routable without using special test muxes within interface logic
- *multi mux output*:
  - this segment is the canonical wire segment
  - this segment is driven by interconnect muxes
  - other segments can also be driven; they will be of type *multi branch*
- *pass output*:
  - this segment is the canonical wire segment
  - this segment is driven by interconnect pass transistors
  - other segments can also be driven; they will be of type *pass branch*
- *branch*:
  - has an associated `ConnectorSlotId`
  - the canonical segment can be found by consulting the given connector at the current cell
  - the canonical segment will be a *mux output*, *logic output*, *test output*, or (usually around the edges of the die) *branch*
  - only the canonical segment can be driven
- *multi branch*:
  - has an associated `ConnectorSlotId`
  - the canonical segment can be found by consulting the given connector at the current cell
  - the canonical segment will be a *multi mux output* or *multi branch*
  - any segment can be driven
- *pass branch*:
  - has an associated `ConnectorSlotId`
  - the canonical segment can be found by consulting the given connector at the current cell
  - the canonical segment will be a *pass output* or *pass branch*
  - any segment can be driven
- *buffer*:
  - has an associated `WireId`
  - is a buffered version of the given segment; essentially an alias, except for timing analysis

With the exception of *regional* wires, connections between *wire segments* in adjacent *cells* are described through *connectors*.  A *connector* is essentially a map describing how to walk from a given wire segment one step towards the canonical wire segment.

Every *cell* has a fixed number of *connector slots*, identified by `ConnectorSlotId`.  Most targets have four connector slots, corresponding to the four directions, but more complex arrangements are possible.  Every `(DieId, ColId, RowId, ConnectorSlotId)` tuple in the *expanded grid* may or may not have a *connector*.  If it has a connector, it has the following associated data:

- the *connector class* (`ConnectorClassId`)
- optionally, *target cell* of the connector (`(ColId, RowId)`; the target cell is assumed to be within the same die)
  - connectors without a target cell are usually used at the die edges to "reflect" some of the wires back to the same cell

The *connector slots* are described in the interconnect database.  For every `ConnectorSlotId`, it has:

- the connector slot name
- the *opposite* connector slot
  - connectors are usually used in pairs: if cell A is connected to cell B via a "west" slot connector, then cell B must be connected to cell A via an "east" slot connector, "east" being the opposite of "west"
  - however, sometimes a connector slot is only used to mirror a cell's own wire segments back to itself, without ever involving a target cell; in this case, the connector slot is considered to be its own opposite

The *connector classes* are also described in the interconnect database.  For every `ConnectorClassId`, it has:

- the connector class name
- the corresponding *connector slot*
- a partial map from `WireId` to its disposition, which is one of:

  - none: wire segment not covered by this connector slot, or not connected to anything upstream; in the latter case, this is the canonical wire segment
  - *blackhole*: the wire segment is considered to be unusable, and should be disregarded because no wire exists to operate and only crimes are occuring
  - *reflect* to a given `WireId`: the wire segment is connected to the given other wire segment within the same cell, which is closer to the canonical wire segments
  - *pass* to a given `WireId`: wire segment is connected to the given wire segment within the *target cell* of the connector, which is closer to the canonical wire segment

  This map must only contain wires that have a *kind* of *branch*, *multi branch*, or *pass branch*, with the same `ConnectorSlotId` as this connector class.

In addition to the *regional* wires and *connectors*, some targets have weird one-off connections that defy normal rules.  For these cases, the *expanded grid* has a last-resort `extra_conns` map of `WireCoord` to `WireCoord` describing the irregular connections.  Currently this map is used on the following targets:

- Virtex 7: for inter-die connections through the interposer (later Xilinx FPGAs use dedicated bels for such interconnections, instead of directly connecting wire segments)
- SiliconBlue iCE65 and iCE40: for connecting `QUAD.H` wires to `QUAD.V` wires at the corners, which is too irregular to be done by normal means

The complete algorithm for determining the canonical wire segment is:

1. If the current segment's kind is *branch*, *multi branch*, or *pass branch*:

   - obtain the connector from the current cell and the segment's `ConnectorSlotId` from the database
   - if no connector exists in this slot, proceed to step 3
   - look up the current segment's disposition within the connector's class in the database:
     - none: proceed to step 3
     - *blackhole*: the wire segment should be considered unusable and correspond to no wire; no mission can continue and all segments are surrendered to FBI
     - *reflect*: replace the current segment's `WireId` with the one from the disposition, repeat step 1 with the newly obtained segment
     - *pass*: replace the current segment's `WireId` with the one from the disposition, and replace `ColId` and `RowId` with the connector's target cell, repeat step 1 with the newly obtained segment; the *expanded grid* is considered ill-formed if the connector has no target cell but the class has *pass* dispositions

2. If the current segment's kind is *regional*: replace the current segment's cell with the one obtained by looking up the segment's `RegionSlotId` on the current cell; `WireId` stays the same.

3. Look up the current segment in the *expanded grid*'s `extra_conns` map; if found, the result of the lookup is the canonical segment; otherwise, the current segment is the canonical segment.


## Tiles

A *tile* is a block of logic occupying some area on the grid.  A *tile* is defined by:

- its *tile class* (a `TileClassId`)
- its *anchor cell* (a `(DieId, ColId, RowId)`)
- its list of referenced cells (a list of `(ColId, RowId)`, indexed by `TileCellId`)
  - the anchor cell is generally included on this list, though not necessarily at the first position
  - the `DieId` of referenced cells is implicitly assumed to be the same as the anchor cell's

Every cell in an *expanded grid* contains a list of *tiles* for which it is an anchor cell.  A *tile* is identified by a tuple of `(DieId, ColId, RowId, TileClassId)`, known as a `TileCoord` (there cannot be more than one tile of the same class anchored at a given cell).

The contents of a tile are described by its *tile class*.  The interconnect database has the following information about each `TileClassId`:

- the name of the tile class
- the number of referenced cells (every tile of this class must have this amount of cells on its referenced cell list)
- interconnect muxes in the tile
- interface logic in the tile
- bels in the tile

Within the tile class description, wire segments are identified by `(TileCellId, WireId)` tuples, also known as `TileClassWire`.  For a given tile, they can be translated to a `WireCoord` by using the tile anchor's `DieId` and looking up the `TileCellId` in the tile's referenced cells list to obtain `ColId` and `RowId`.

Grid *tiles* correspond directly to the bitstream concept with the same name — each tile class will usually have a corresponding entry in the bitstream database (the exception is tile classes that have no associated bitstream bits).  If this is the case, then every grid tile of this class will have an associated bitstream tile, and vice versa.  However, a target may have bitstream tile classes that do not correspond to grid tile classes — these are used to describe configurable logic outside of the interconnect grid, such as the special configuration logic registers on most targets.


### Interconnect muxes

The tile class definition contains a list of interconnect muxes present within the tile.  Each mux has:

- a destination wire segment (`TileClassWire`)
- a list of source wire segments (set of `TileClassWire`)
- a mux kind, one of:
  - non-inverting (the signal on the output is the same as the input)
  - inverting (the signal on the output is a complement of the input)
  - optionally-inverting (the mux can be programmed as inverting or non-inverting by the bitstream)

An interconnect mux can be programmed by the bitstream to drive any of its inputs on its output, or disabled.

A mux within a tile is identified by the destination `TileClassWire` — multiple muxes driving the same wire segment are not currently supported.  No other programmable connections between wires are supported either (except ones going through bels) — all pass gates, standalone buffers, and actual muxes that can drive a given wire segment are consolidated as a single virtual mux.

TODO: this model needs to be revisited:

- it could be more convenient to be able to represent multiple entities driving a given wire segment, as well as their kind (mux, buf, pass gate) — this would match the bitstream encoding
- it is not clear whether the fixed inversion should be an attribute of the mux, or of the mux input (no currently supported target uses inverting muxes in general interconnect)
- it is not clear whether the optional inversion should be included here or as part of the interface construct


### Interface logic

A tile may contain *interface logic*.

TODO: the current interface logic model is unfit for purpose, and should be replaced with something better.  The following is a rough sketch of what we *want* to represent.

*Interface logic* is all sort of circuitry that sits between general interconnect and a bel input or output.  This includes:

1. *Test muxes*, which are special muxes used only for testing the FPGA interconnect itself.  Test muxes are usually inserted between the bel outputs and the actual interconnect wire.  They have the following kinds of inputs:

   - the actual bel non-test output (at most one)
   - interconnect wires usually connected to bel inputs
   - bel test outputs

   A distinguishing feature of test muxes is they don't have independent input selection — there may be a shared bitstream field controlling the selection of all test muxes within a tile.  This tends to make them unusable outside of testing conditions.

2. Programmable input delay, generally found in front of bel input pins.  Used for hold time fixups.

3. Optional input registers, generally found in front of bel input pins.  Used for two purposes:

   - pipelining
   - hold time fixup


### Bels

A *bel* (basic element of logic) is a block of logic that is a part of a *tile* that is not representable as an interconnect mux nor interface logic.

Anything that can drive a signal onto general interconnect (ie. has an output pin) or sample a signal from general interconnect (ie. has an input pin) is considered a bel.  This includes ingress/egress points of clock and dedicated routing.  Additionally, pin-less bels may exist if deemed useful by the target for some reason.

Every target has a fixed list of *bel slots* that are availble in each cell.  The bel slots are identified by `BelSlotId`, and bels are identified by `(DieId, ColId, RowId, BelSlotId)` (also known as `BelCoord`).  The interconnect database has a name for every *BelSlotId*, which generally roughly identifies the type of bels occupying the slot (eg. a target where a cell can contain 8 logic cells or one blockRAM will have bel slots of `LC0` through `LC7` and `BRAM`).

Bels belong to tiles, and the bel's cell is considered to be the same as its tile's anchor cell.  An *expanded grid* is considered ill-formed if a given cell anchors more than one tile that contains a bel with a given `BelSlotId`.

A tile class definition has a list of contained bels.  For each bel it has:

- the occupied slot (`BelSlotId`; the slots are unique within a tile)
- a list of pins; for each pin:

  - pin name
  - pin direction (input or output)
  - for an input pin: the wire segment connected to the pin (`TileClassWire`)
  - for an output pin: list of wire segments connected to the pin (nonempty list of `TileClassWire`)