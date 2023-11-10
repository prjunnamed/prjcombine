JTAG interface
##############

IR
==

The IR is 8 bits long.  The following instructions exist:

============ ============ ==================== =======
IR           Instruction  Register             Notes
============ ============ ==================== =======
``00000000`` ``EXTEST``   ``BOUNDARY``
``00000001`` ``SAMPLE``   ``BOUNDARY``
``00000010`` ``INTEST``   ``BOUNDARY``
``11100101`` ``FBLANK``   ``ISPADDRESS``       XC9500XL/XV only
``11101000`` ``ISPEN``    ``ISPENABLE``
``11101001`` ``ISPENC``   ``ISPENABLE``        XC9500XL/XV only
``11101010`` ``FPGM``     ``ISPCONFIGURATION``
``11101011`` ``FPGMI``    ``ISPDATA``
``11101100`` ``FERASE``   ``ISPCONFIGURATION`` XC9500 only
``11101100`` ``FERASE``   ``ISPADDRESS``       XC9500XL/XV only
``11101101`` ``FBULK``    ``ISPCONFIGURATION`` XC9500 only
``11101101`` ``FBULK``    ``ISPADDRESS``       XC9500XL/XV only
``11101110`` ``FVFY``     ``ISPCONFIGURATION``
``11101111`` ``FVFYI``    ``ISPDATA``
``11110000`` ``ISPEX``    ``BYPASS``
``11111010`` ``CLAMP``    ``BYPASS``           XC9500XL/XV only
``11111100`` ``HIGHZ``    ``BYPASS``
``11111101`` ``USERCODE`` ``USERCODE``
``11111110`` ``IDCODE``   ``IDCODE``
``11111111`` ``BYPASS``   ``BYPASS``
============ ============ ==================== =======

The IR status is:

- bit 0: const 1
- bit 1: const 0
- bit 2: ``WRITE_PROT`` status
- bit 3: ``READ_PROT`` status
- bit 4: ISP mode enabled
- bit 5: ``DONE`` status (XC9500XV only, const 0 on other devices)
- bits 6-7: const 0

.. todo:: verify


Boundary scan register
======================

The boundary scan register is ``3 * 18 * num_fbs`` bits long, and consists of 3 bits for every MC
in the device: input, output, and output enable.  Such bits are included even for MCs that do not
have a corresponding IOB.

The boundary register bit indices for ``FB[i].MC[j]`` are:

- input: ``(num_fbs - 1 - i) * 18 * 3 + (17 - j) * 3 + 2``
- output: ``(num_fbs - 1 - i) * 18 * 3 + (17 - j) * 3 + 1``
- output enable: ``(num_fbs - 1 - i) * 18 * 3 + (17 - j) * 3 + 0``

All bits of the register are ``BC_1`` type cells.

.. todo:: details on the cell connection, EXTEST, INTEST semantics


ISP instructions
================

ISP DR registers — XC9500
-------------------------

The following DR registers exist on XC9500:

1. ``ISPENABLE`` (16 bits): function and contents unclear
2. ``ISPCONFIGURATION`` (27 bits): used to load and store bitstream bytes

   - bit 0: valid bit
   - bit 1: strobe bit
   - bits 2-9: data byte
   - bits 10-26: address

3. ``ISPDATA`` (10 bits): a subset of ``ISPCONFIGURATION`` used by instructions with autoincrementing address

   - bit 0: valid bit
   - bit 1: strobe bit
   - bits 2-9: data byte

.. todo:: ISPENABLE, describe valid & strobe


ISP DR registers — XC9500XL/XV
------------------------------

The following DR registers exist on XC9500XL/XV:

1. ``ISPENABLE`` (6 bits): function and contents unclear
2. ``ISPCONFIGURATION`` (``18 + num_fbs * 8`` bits): used to load and store bitstream words

   - bit 0: valid bit
   - bit 1: strobe bit
   - bits 2-``(num_sbs * 8 + 1)``: data word
   - bits ``(num_fbs * 8 + 2)``-``(num_fbs * 8 + 17)``: address

3. ``ISPDATA`` (``2 + num_fbs * 8`` bits): a subset of ``ISPCONFIGURATION`` used by instructions with autoincrementing address

   - bit 0: valid bit
   - bit 1: strobe bit
   - bits 2-``(num_sbs * 8 + 1)``: data word

4. ``ISPADDRESS`` (18 bits): a subset of ``ISPCONFIGURATION`` used by some instructions:

  - bit 0: valid bit
  - bit 1: strobe bit
  - bits 2-17: address


Entering and exiting ISP mode
-----------------------------

Before any programming or readout can be done, the device needs to be put into ISP mode.
For that purpose, the ``ISPEN`` or ``ISPENC`` (XC9500XL/XV only) instructions can be used.
Both instructions use the ``ISPENABLE`` register, which is 16 bits on XC9500
and 6 bits on XC9500XL/XV.  Its meaning, if any, is unknown.

To enter ISP mode:

- shift ``ISPEN`` or ``ISPENC`` into IR
- shift 0s into DR
- go to Run-Test/Idle state for at least 1 clock

If the ``ISPEN`` instruction is used, all outputs will be put in high-Z with weak pull-ups while ISP mode is active.
If the ``ISPENC`` ("clamp" mode) instruction is used, all output and output enable signals will be snapshotted
and outputs will continue driving the last value while ISP mode is active.

To exit ISP mode:

- shift ``ISPEX`` into IR
- go to Run-Test/Idle state for at least 1 clock

When ISP mode is exitted, the device will initialize itself and start normal operation.

.. todo:: verify, see if anything can be figured out about the DR


Erasing fuses
-------------

.. todo:: write me


Programming fuses
-----------------

.. todo:: write me


Reading fuses
-------------

.. todo:: write me


Blank check
-----------

.. todo:: write me


Programming sequence
====================

.. todo:: write me