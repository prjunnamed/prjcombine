# General terminology

This document describes general terminology used across Project Combine.


## Target

A *target* is a group of "similar enough" devices that are handled by common code paths.  What exactly constitutes a "target" varies with context — for example, depending on the code path, Spartan 3 and Spartan 3E may be considered distinct targets, or variants of the same target.


## Devices, chips, packages, speed grades

![Example device](device-light.svg#light-only)
![Example device](device-dark.svg#dark-only)

A *chip* structure describes a single kind of piece of silicon with programmable logic.

A *device* structure describes a single arrangement of programmable logic distributed by a vendor (eg. `iCE40UP5K`).  It generally corresponds to a vendor designation.

In the simplest case, a device corresponds to a single chip.  However, reality is more complicated than that:

1. A given chip can be customized at the factory by burning fuses or other means.  This results
   in multiple versions of a chip, with minor variations (such as disabling some blocks of the device).
   We generally consider such variations to be distinct devices.

2. The same base chip can be made in multiple versions (high-speed vs low-power, automotive qualified, etc).  We generally consider them to be distinct devices if the vendor does so.

3. Some targets involve multi-chip devices, where several logic die are connected together with inter-chip wires through an interposer die.  In this case:

  - a *die* is an individual *chip* instance within the device; they are identified by the `DieId` type
  - a given device may consist of multiple instances of the same *chip* or be built from heterogenous *chip*s
  - an *interposer* structure describes the interconnections between *die*, as well as any included extra logic that is not considered a *chip* (such as the HBM memory stacks included on the package, or GTZ transceivers for Virtex 7 devices)
    - multiple devices can reuse the same interposer structure
  - the *device* describes the *chip*s and *interposer* involved in the whole assembly, as well as their particular configuration

4. In cases like eFPGAs where programmable logic is embedded in a larger device whose main function is not being a PLD, a Project Combine *device* and *chip* will be just a part of the physical silicon chip.  In this case, we may describe only the programmable logic "macro" and ignore the surroundings.

Further, a given *device* can come in multiple variants:

1. A *device* can come in one or more *packages*, which correspond to actual physical packages the device is offered in.

   - a *pad* is an electrical net exposed by a *device*; it usually corresponds to a physical pad on the silicon die
     - a *pad* may be the IO signal associated with a given IO block, a dedicated configuration signal, a ground or power net, or something else entirely; it is identified by a device-specific type (which can be roundtripped as a string)
   - a *pin* is an electrical contact exposed by a *package*; it may be a QFP lead, a BGA ball, or something similar; it is identified by a string
     - for QFP packages and similar, pins are usually identified by a number; a 144-TQFP package would have pins named `P1` through `P144`
     - for BGA packages and similar, pins are usually identified by grid coordinates, such as `A1` for the ball in the corner, or `D5` for ball further inside the grid
   - a *package* is associated with a *bond*, which is a structure that describes how a device is embedded within the package
   - the main contents of the *bond* structure is the mapping from *pin*s to *pad*s
   - the *bond* structure may additionally contain other relevant data (such us which power rails are connected together in-package)
   - note that there may be multiple *pin*s mapped to a given *pad* (this is usually the case for ground and power pads)
   - in rare cases, there may also be multiple pads mapped to a given *pin*; however, this is usually represented by a special *pad* value that actually describes multiple pads
   - some kinds of devices may not have package data at all (such as eFPGAs, where the concept is meaningless)

2. A *device* can come in one or more *speed grades*, which determine how fast the device is able to run.

   - a *speed grade* is associated with a *speed* structure, which contains the raw speed data;
   - in some cases, some device functionality can be unavailable in a given speed grade (such as PCI Express blocks in a substrate that is too slow to actually run on PCI Express speeds)

3. A *device* can come in one or more *temperature grades*, which determine how hot or cold the device can run.

Generally, a (device, package, speed grade, temperature grade) tuple is used to fully describe the intended device configuration.


## FPGA vs CPLD

We classify a device into one of two categories based solely on the kind of interconnect structure:

- an *FPGA* is a programmable device with a grid-based regular interconnect structure
  - FPGA targets use the `prjcombine-interconnect` crate and its data structures
- a *CPLD* is a programmable device with a hiererchical, not grid-based interconnect structure
  - CPLD targets don't currently have a common crate; they use the CPLD data types defined in the `prjcombine-types` crate
  - SPLD targets are considered to be a special case of CPLDs with unusually simple internal structure

This sometimes conflicts with the vendor designation for a device (eg. Altera considers Max II to be a CPLD because of its non-volatile configuration memory; we consider it to be an FPGA).  We consider this to be one of many ways in which vendors are lying, and ignore their classification entirely.


## CPLD structure

A *macrocell* is the basic logic unit of CPLDs. It generally involves a sum term (made of product terms) outputting a single signal, and a register that optionally registers the signal.  It may or may not have an output buffer and a I/O pad associated.  A macrocell without an associated pad is called a *buried macrocell*.

A *product term* is a wide AND gate that has a number (usually a few dozen) of possible inputs.  An arbitrary subset of them can be enabled when configuring the device.  Depending on target, a product term may be permanently associated with a particular macroblock, or be freely assignable.

A *block* is a group of macrocells, product terms, and other associated logic.  Generally, all product terms within a block have a common set of inputs.  SPLDs are considered to have only one block, covering the entire device.  On non-SPLDs, the block inputs tend to be selected by multiplexers from global interconnect signals (for non-clustered CPLDs) or from cluster interconnect signals (for clustered CPLDs).

Some CPLDs are *clustered devices*.  On such devices, blocks are grouped into *clusters*, and there are two kinds of interconnect lines: global interconnect (covering the whole chip) and cluster interconnect (covering an individual cluster).  Non-clustered devices are considered to have only one cluster, and there's no distinction between global and cluster interconnect.

A macrocell is identified by a `(ClusterId, BlockId, MacrocellId)` tuple.  An I/O pad is identified by either:

- a `(ClusterId, BlockId, MacrocellId)` tuple identifying the associated macrocell that can drive the pad, or
- an `IpadId` identifying an input-only pad

A product term is identified by a `(ClusterId, BlockId, ProductTermId)` tuple.

A block input is identified by a `(ClusterId, BlockId, BlockInputId)` tuple.  This doesn't apply to SPLDs, where block inputs are instead directly identified by the associated I/O pad.
