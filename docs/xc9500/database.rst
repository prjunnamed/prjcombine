Database schema
###############

The device database is provided in machine-readable form as three JSON files:

- `xc9500.json <https://raw.githubusercontent.com/prjunnamed/prjcombine/main/databases/xc9500.json>`_, describing all XC9500 devices
- `xc9500xl.json <https://raw.githubusercontent.com/prjunnamed/prjcombine/main/databases/xc9500xl.json>`_, describing all XC9500XL devices
- `xc9500xv.json <https://raw.githubusercontent.com/prjunnamed/prjcombine/main/databases/xc9500xv.json>`_, describing all XC9500XV devices

All three files have the same schema.


Top level
=========

The top level of the database is an object, with the following fields:

- ``chips`` (list of object): list of :ref:`chip <xc9500-db-chip>`
- ``bonds`` (list of object): list of :ref:`bond <xc9500-db-bond>`
- ``speeds`` (list of object): list of :ref:`speed <xc9500-db-speed>`
- ``parts`` (list of object): list of :ref:`part <xc9500-db-part>`
- ``mc_bits`` (object): a :ref:`tile <xc9500-db-tile>` describing per-MC bits
- ``fb_bits`` (object): a :ref:`tile <xc9500-db-tile>` describing per-FB bits
- ``global_bits`` (object): a :ref:`tile <xc9500-db-tile>` describing global bits


.. _xc9500-db-chip:

Chip
====

A chip is a structure describing a particular XC9500 die.  A chip is referenced
from a :ref:`part <xc9500-db-part>`.  A chip is an object with the following fields:

- ``kind`` (string): the kind of the chip, one of:

  - ``"xc9500"``
  - ``"xc9500xl"``
  - ``"xc9500xv"``

  All chips within a single file will have the same kind.

- ``idcode`` (number): the JTAG IDCODE of the chip
- ``fbs`` (number): the number of FBs in the chip
- ``ios`` (map from string to number): describes the I/O pads available on the chip.
  The keys are of the form ``"IOB_{fb_idx}_{mc_idx}"`` and identify the MC that owns the IOB.
  The value corresponding to the key is the bank index that the IOB belongs to.
- ``banks`` (number): the number of I/O banks in the chip
- ``tdo_bank`` (number): the I/O bank index that is used to drive the TDO special pin
- ``io_special`` (map from string to pair of numbers): describes the I/O pads with special functions on the chip.
  The keys can be:

  - ``"GCLK[0-2]"``
  - ``"GSR"``
  - ``"GOE[0-3]"``

  The values are two-element lists of numbers.  The first number is FB index, and the second
  is MC index.  Note that sometimes items in this map are overriden by the bond.

- ``imux_bits`` (object) : a :ref:`tile <xc9500-db-tile>` describing per-FB bits corresponding to IMUX
- ``uim_ibuf_bits`` (object or null): for XC95288 chip, a :ref:`tile <xc9500-db-tile>` describing the UIM IBUF bits; for every other chip, ``null``
- ``program_time`` (number): maximum time required by a program operation, in µs
- ``erase_time`` (number): maximum time required by an erase operation, in µs


.. _xc9500-db-bond:

Bond
====

A bond is a structure describing the mapping of chip pads to package pins.
Bonds are referenced from :ref:`part <xc9500-db-part>` packages.  A bond is an object
with the following fields:

- ``io_special_override`` (map from string to pair of numbers): a map like the chip's ``io_special`` map, containing per-bond overrides
  to the defaults (usually empty)
- ``pins`` (map from string to string): the pins of the package; they keys are package pin names, and the values are:

  - ``NC``: unconnected pin
  - ``GND``, ``VCCINT``, ``VCCIO{bank}``: power and ground pins
  - ``IOB_{fb}_{mc}``: an I/O pin
  - ``TMS``, ``TCK``, ``TDI``, ``TDO``: JTAG pins


.. _xc9500-db-speed:

Speed
=====

A speed is a structure describing the timings of a device.  They are referenced from
:ref:`part <xc9500-db-part>` speed grades.  A speed is an object with one field:

- ``timing`` (map of string to number): a map from timing parameter name to timing data.
  All timing data is given in picoseconds, and is always an integer number.


.. _xc9500-db-part:

Part
====

A part is a structure describing a particular commercially available part number.
Several parts may correspond to the same chip.  A part is an object with the following fields:

- ``name`` (string): the base name of the part, in lowercase
- ``chip`` (number): the index of the corresponding chip in the ``chips`` field
  of the database
- ``packages`` (map from string to int): the packages in which this part is available;
  the key is package name, and the value is the index of the corresponding bond in the ``bonds`` database field
- ``speeds`` (map from string to int): the speed grades in which this part is available;
  the key is speed grade name (including the leading ``-``), and the value is the index of the corresponding speed in the ``speeds`` database field


.. _xc9500-db-tile:

Tile
====

A tile is a structure describing a set of chip fuses.  There are multiple kinds
of tiles used to describe the bitstream.  The base structure of a tile is the same
for all of these kinds.

A tile is an object where the keys are fuse set names, and the values are objects
with the following keys:

- ``bits`` (list of coordinate): the list of fuse coordinates in this fuse set
- one of:

  - ``values`` (map of string to list of bool): used for an enumerated fuse set;
    the list of possible values for this fuse set; the value is a list of fuse values, corresponding one
    to one to the coordinates in ``bits``
  - ``invert`` (bool): used for a plain bool / bitvec fuse set; if true, it means
    that the value of this bitvec or bool is stored inverted in the datastream
    (0 means true, 1 means false); if false, the value is stored directly without
    inversion

The type and interpretation of coordinate depends on the tile kind.

The following tile kinds exist:

- per-MC bits tile: identical for all chips in the database, the coordinate
  is a single number and corresponds to the row coordinate of the fuse
- per-FB bits tile: identical for all chips in the database, the coordinate
  is a list of 3 numbers, in order:

  - row
  - bit
  - column

- IMUX bits tile: chip-specific, the coordinate is the same as for per-FB bits tile
- global bits tile: identical for all chips in the database, the coordinate
  is a list of 4 numbers, in order:

  - fb
  - row
  - bit
  - column

- UIM IBUF bits tile: chip-specific, only for XC995288, the coordinate is the same as for global bits tile
