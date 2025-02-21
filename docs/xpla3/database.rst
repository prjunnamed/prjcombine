Database schema
###############

The device database is provided in machine-readable form as a JSON file:

- `xpla3.json <https://raw.githubusercontent.com/prjunnamed/prjcombine/main/databases/xpla3.json>`_, describing all XPLA3 devices


Top level
=========

The top level of the database is an object, with the following fields:

- ``chips`` (list of object): list of :ref:`chip <xpla3-db-chip>`
- ``bonds`` (list of object): list of :ref:`bond <xpla3-db-bond>`
- ``speeds`` (list of object): list of :ref:`speed <xpla3-db-speed>`
- ``parts`` (list of object): list of :ref:`part <xpla3-db-part>`
- ``mc_bits`` (object): a :ref:`tile <xpla3-db-tile>` describing per-MC bits
- ``fb_bits`` (object): a :ref:`tile <xpla3-db-tile>` describing per-FB bits
- ``jed_fb_bits`` (array): a :ref:`jed bits list <xpla3-db-jed-bits>` describing per-FB bits
- ``jed_mc_bits_iob`` (array) a :ref:`jed bits list <xpla3-db-jed-bits>` describing per-MC bits for MCs with IOBs
- ``jed_mc_bits_buried`` (array) a :ref:`jed bits list <xpla3-db-jed-bits>` describing per-MC bits for MCs without IOBs


.. _xpla3-db-chip:

Chip
====

A chip is a structure describing a particular XPLA3 die.  A chip is referenced
from a :ref:`part <xpla3-db-part>`.  A chip is an object with the following fields:

- ``idcode_part`` (number): bit 12-27 of the JTAG IDCODE of the chip, with low 3 bits (package type) masked to 0
- ``bs_cols`` (number): bitstream width in columns
- ``imux_width`` (number): width of the IMUX bitstream area in columns (and also the size of a single IMUX selector in bits)
- ``fb_rows`` (number): the number of FB rows
- ``fb_cols`` (array of object): list of FB columns; each item is an object with the following fields:

  - ``pt_col`` (number): first column of the PT area in bitstream
  - ``imux_col`` (number): first column of the IMUX area in bitstream
  - ``mc_col`` (number): first column of the MC/FB area in bitstream

- ``io_mcs`` (array of number): list of MC ids that have IOBs; the list is the same across all FBs, and the list stores only MC indices
- ``io_special`` (map from string to pair of numbers): describes the I/O pads with special functions on the chip.
  The keys can be:

  - ``"TCK"``
  - ``"TMS"``
  - ``"TDI"``
  - ``"TDO"``

  The values are of the form ``"IOB_{fb_idx}_{mc_idx}"``.

- ``imux_bits`` (object) : a :ref:`tile <xpla3-db-tile>` describing per-FB bits corresponding to IMUX
- ``global_bits`` (object) : a :ref:`tile <xpla3-db-tile>` describing global bits
- ``jed_global_bits`` (array): a :ref:`jed bits list <xpla3-db-jed-bits>` describing global bits


.. _xpla3-db-bond:

Bond
====

A bond is a structure describing the mapping of chip pads to package pins.
Bonds are referenced from :ref:`part <xpla3-db-part>` packages.  A bond is an object
with the following fields:

- ``idcode_part`` (number): bit 12-27 of the JTAG IDCODE of the chip
- ``pins`` (map from string to string): the pins of the package; they keys are package pin names, and the values are:

  - ``NC``: unconnected pin
  - ``GND``, ``VCC``: power and ground pins
  - ``IOB_{fb}_{mc}``: an I/O pin
  - ``GCLK{i}``: dedicated clock input pins
  - ``PORT_EN``: JTAG enable pin


.. _xpla3-db-speed:

Speed
=====

A speed is a structure describing the timings of a device.  They are referenced from
:ref:`part <xpla3-db-part>` speed grades.  A speed is an object with one field:

- ``timing`` (map of string to number): a map from timing parameter name to timing data.
  All timing data is given in picoseconds, and is always an integer number.


.. _xpla3-db-part:

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



.. _xpla3-db-tile:

Tile
====

A tile is a structure describing a set of chip fuses.

A tile is an object where the keys are fuse set names, and the values are objects
with the following keys:

- ``bits`` (list of coordinate): the list of fuse coordinates in this fuse set; a single coordinate is an array of three numbers, in order:

  - fuse plane
  - fuse row
  - fuse column

  Depending on the tile, the coordinates may be either absolute or relative to some base.

- one of:

  - ``values`` (map of string to list of bool): used for an enumerated fuse set;
    the list of possible values for this fuse set; the value is a list of fuse values, corresponding one
    to one to the coordinates in ``bits``
  - ``invert`` (bool): used for a plain bool / bitvec fuse set; if true, it means
    that the value of this bitvec or bool is stored inverted in the datastream
    (0 means true, 1 means false); if false, the value is stored directly without
    inversion


.. _xpla3-db-jed-bits:

JED bits
========

A JED bits list is a structure describing a part of the JED file layout.
It is represented as an array, with each item describing one fuse, in JED file order.
Each item of the array is itself an array of two items:

- item 0 (string): fuse set name, referencing an item in the associated tile
- item 1 (number): fuse index within fuse set
