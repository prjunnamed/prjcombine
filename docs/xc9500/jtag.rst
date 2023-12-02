JTAG interface
##############

Frequency
=========

The max TCK frequency for all XC9500 family devices is 10MHz.


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

Note that the protection and ``DONE`` status is latched when the device is reset and when
``ISPEX`` is executed — when erasing or programming the fuses, the new settings won't take
effect before exiting the ISP mode.


IDCODE
======

The IDCODE for XC9500* devices can be determined as follows:

- bits 0-11: vendor code, ``0x093``
- bits 12-19: number of FBs in the device
- bits 20-27: device kind

  - ``0x95``: XC9500
  - ``0x96``: XC9500XL
  - ``0x97``: XC9500XV

- bits 28-31: device revision (varies)


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


ISP instructions — XC9500
=========================

ISP DR registers — XC9500
-------------------------

The following DR registers exist on XC9500:

1. ``ISPENABLE`` (``num_fbs + 4`` bits): used to power on the flash programming circuits

   - bits 0-``num_fbs - 1``: when set, powers up flash access circuitry for main areas, one bit per FB
   - bit ``num_fbs``: when set, powers up flash access circuitry for the UIM wire-AND area
   - bits ``num_fbs + 1`` - ``num_fbs + 3``: unknown, set to 0

2. ``ISPCONFIGURATION`` (27 bits): used to load and store bitstream bytes

   - bit 0: valid bit
   - bit 1: strobe bit
   - bits 2-9: data byte
   - bits 10-26: address

3. ``ISPDATA`` (10 bits): a subset of ``ISPCONFIGURATION`` used by instructions with autoincrementing address

   - bit 0: valid bit
   - bit 1: strobe bit
   - bits 2-9: data byte

Programming operations are triggered when the ``ISPDATA`` or ``ISPCONFIGURATION`` register
is written with the strobe bit set and the valid bit unset (ie. with the bottom two bits set
to ``0b10``).  Once the operation succeeds, the valid bit will be set by the device, and the
bottom two bits of the register will read as ``0b11``.  This should be checked by the programmer.


Entering and exiting ISP mode — XC9500
--------------------------------------

Before any programming or readout can be done, the device needs to be put into ISP mode.
For that purpose, the ``ISPEN`` instructions can be used.
The instruction uses the ``ISPENABLE`` register, which is described above.

To enter ISP mode:

- shift ``ISPEN`` into IR
- shift a value into DR, setting all bits except the top 3
- go to Run-Test/Idle state for at least 1 clock

All outputs will be put in high-Z with weak pull-ups while ISP mode is active.

To exit ISP mode:

- shift ``ISPEX`` into IR
- go to Run-Test/Idle state for at least 100µs

When ISP mode is exited, the device will initialize itself and start normal operation.

The reset value of ``ISPENABLE`` is:

- bits 0 - ``num_fbs``: set to 1
- bits ``num_fbs + 1`` - ``num_fbs + 3``: set to 0

The step of shifting a value into DR above is thus optional if the default hasn't been modified.

When read, the ``ISPENABLE`` register always returns the reset value, regardless of the actual
value that was last shifted in.


Write protection — XC9500
-------------------------

If the device is write protected (which can be determined by reading bit 2 of IR), it cannot
be written (erased or programmed) without unlocking write protection first.

To unlock write protection:

- enter ISP mode, if not already entered
- shift ``FERASE`` or ``FBULK`` to IR
- shift the following value to DR:

  - bit 0 (valid): 0
  - bit 1 (strobe): 1
  - bits 2-9 (data): don't care
  - bits 10-26 (address): ``0x1aa55``

Once that value is shifted in, the device is unlocked, and bit 2 of IR goes to 0.  However,
this unlock only lasts for the duration of the current ISP mode session — once ``ISPEX`` is
executed (or the device is reset), the write protection status will be reloaded from the flash.


Erasing fuses — XC9500
----------------------

There are two instructions that erase fuses:

- ``FERASE``: erases one area at a time (either a single FB main area, or a single FB UIM
  wire-AND area)
- ``FBULK``: erases either all main areas on the device at once, or all UIM wire-AND areas
  at once

An erase operation is triggered by the following sequence:

1. The DR is written in one of the above opcodes with strobe set to 1 and valid set to 0
2. The Run-Test/Idle state is entered

Once started, the erase operation is self-timed.  The maximum programming time can be obtained
from the database.  When successful, the low two bits of
DR will become ``11``.  Note that shifting DR again before the operation is complete will
abort it and return ``00`` in the low bits.

.. note:: The timeout value of 1.3s present in the database is taken directly from ISE SVFs,
   but it appears to be too small for the two devices I (@wanda-phi) personally possess,
   which require a timeout of 2s.  Since these devices came from random ebay listings, this
   may be due to age or mishandling.  Still, you may want to consider using a larger timeout
   in your programming software.

To erase fuses:

- enter ISP mode, if not already entered
- ensure the ``ISPENABLE`` bit corresponding to the area being erased is set
- shift a value into DR:

  - bit 0 (valid): 0
  - bit 1 (strobe): 1
  - bits 2-9 (data): don't care
  - bits 10-27 (address):

    - bits 0-11 (row, column): don't care
    - bit 12: 0 to erase main area, 1 to erase UIM wire-AND area
    - bits 13-16:

      - for ``FERASE``: FB index
      - for ``FBULK``: don't care

- go to Run-Test/Idle state for at least the time specified in the database for this device
- shift a neutral value into DR:

  - bits 0-1 (valid and strobe): 0
  - all other bits: don't care

  Verify that the low 2 bits of the value shifted out are ``11``.  Any other value is an error.
  Value ``00`` could mean that the erase was still in progress.  Value ``10`` could mean that
  device is write protected.  Other values are unknown.


Programming fuses — XC9500
--------------------------

Fuses are programmed a byte at a time in random-access fashion.  The ``FPGM`` and ``FPGMI``
opcodes are used for programming.  A program operation is triggered by the following sequence:

1. The DR is written in one of the above opcodes with strobe set to 1 and valid set to 0
2. The Run-Test/Idle state is entered

Once started, the programming operation is self-timed.  The maximum programming time depends
on the device and can be obtained from the database.  When successful, the low two bits of
DR will become ``11``.  Note that shifting DR again before the operation is complete will
abort it and return ``01`` in the low bits.

As usual with flash devices, a program operation can only change a ``1`` bit to a ``0`` bit.
Bits can be reset back to ``1`` only by an erase operation.  A programming operation
that attempts to set an already-0 bit to 1 will fail, and result in ``01`` in the low DR bits.

To program a single byte:

- enter ISP mode, if not already entered
- ensure the ``ISPENABLE`` bit corresponding to the area being programmed is set
- shift ``FPGM`` into IR
- shift a value into DR:

  - bit 0 (valid): 0
  - bit 1 (strobe): 1
  - bits 2-9 (data): the data to be programmed
  - bits 10-26 (address): the address to program

- go to Run-Test/Idle state for at least the time specified in the database for this device
- shift a neutral value into DR:

  - bits 0-1 (valid and strobe): 0
  - all other bits: don't care

  Verify that the low 2 bits of the value shifted out are ``11``.  Any other value is an error.

When more than one byte is to be read, the DR shifts can be overlapped — instead of shifting
in a neutral DR value when reading back the status, the shift can be used to
trigger the second program:

- prepare ISP mode, shift ``FPGM`` into IR
- shift first address + data + strobe into DR
- go to RTI for programming time
- shift second address + data + strobe into DR, verify low 2 bits of the value shifted out
- go to RTI for programming time
- shift third address + data + strobe into DR, verify low 2 bits of the value shifted out
- ...
- shift neutral value into DR, verify low 2 bits of the value shifted out

When a block of sequential addresses is to be programmed (including programming the whole device),
the ``FPGMI`` auto-incrementing instruction can be used as follows:

- prepare ISP mode, shift ``FPGM`` into IR
- shift first address + data + strobe into DR
- go to RTI for programming time
- shift ``FPGMI`` into IR
- shift second data + strobe into DR, ie. shift the following value into DR:

  - bit 0 (valid): set to 0
  - bit 1 (strobe): set to 1
  - bits 2-9 (data): second data byte

  Verify low two bits of the value shifted out are ``11``
- go to RTI for programming time
- shift third data + strobe into DR, verify status
- go to RTI for programming time
- ...
- shift neutral value (strobe and valid 0) into DR, verify status of the last byte

Since ``FPGMI`` skips over invalid addresses when auto-incrementing, the above sequence
can be used to program the entire device in one go.


Reading fuses — XC9500
----------------------

Fuses are read a byte at a time in random-access fashion.  The ``FVFY`` and ``FVFYI`` opcodes
are used for reading.  A read is triggered by the following sequence:

1. The DR is written in one of the above opcodes with strobe set to 1 and valid set to 0
2. The Run-Test/Idle state is entered

The reads are immediate — they require only one TCK cycle to be spent in the Run-Test/Idle state.

The data is read from the currently set address and placed in the ``ISPCONFIGURATION`` / ``ISPDATA``
data field.  If the ``FVFYI`` opcode was used, the address is then auto-incremented to the next
valid address in the device.

To read a single byte of fuses:

- enter ISP mode, if not already entered
- ensure the ``ISPENABLE`` bit corresponding to the area being read is set
- shift ``FVFY`` into IR
- shift a value into DR:

  - bit 0 (valid): 0
  - bit 1 (strobe): 1
  - bits 2-9 (data): don't care
  - bits 10-26 (address): the address to read

- go to Run-Test/Idle state for at least 1 clock
- shift a neutral value into DR:

  - bits 0-1 (valid and strobe): 0
  - all other bits: don't care

  The data read will be:

  - bit 0 (valid): 1 if operation succeeded (0 if error)
  - bit 1 (strobe): always 1
  - bits 2-9: the data byte read
  - bits 10-26: the address

When more than one byte is to be read, the DR shifts can be overlapped — instead of shifting
in a neutral DR value when reading back the first data byte, the shift can be used to
trigger the second read:

- prepare ISP mode, shift ``FVFY`` into IR
- shift first address + strobe into DR
- go to RTI for one clock
- shift second address + strobe into DR, grab first data from the value shifted out
- go to RTI for one clock
- shift third address + strobe into DR, grab second data from the value shifted out
- ...
- shift neutral value into DR, grab final data from the value shifted out

When a block of sequential addresses is to be read (including reading the whole device),
the ``FVFYI`` auto-incrementing instruction can be used as follows:

- prepare ISP mode, shift ``FVFY`` into IR
- shift first address + strobe into DR
- go to RTI for one clock
- shift ``FVFYI`` into IR
- shift second strobe into DR, ie. shift the following value into DR:

  - bit 0 (valid): set to 0
  - bit 1 (strobe): set to 1
  - bits 2-9 (data): don't care

  Grab first data byte from the value shifted out
- go to RTI for one clock
- shift third strobe into DR, grab second data from the value shifted out
- go to RTI for one clock
- ...
- shift neutral value (strobe and valid 0) into DR, grab final data from the value shifted out

Since ``FVFYI`` skips over invalid addresses when auto-incrementing, the above sequence
can be used to read the entire device in one go.

If any read protection fuse was programmed in the device as of the time of last ``ISPEN``,
the device is considered read protected, and most addresses cannot be read (reads from them will
return all-1 instead of the actual data).  The only readable fuses are:

- main area fuses, with
- row in range 0-7
- bit in range 6-7

This corresponds (roughly) to global configuration bits, including the USERCODE.

Note that read protection remains active until the ISP mode is exited even after the device has
been completely erased — for that reason, one should perform ISP mode exit and re-entry after
erasing the device.


ISP instructions — XC9500XL/XV
==============================

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
   - bits 2-``(num_fbs * 8 + 1)``: data word

4. ``ISPADDRESS`` (18 bits): a subset of ``ISPCONFIGURATION`` used by some instructions:

  - bit 0: valid bit
  - bit 1: strobe bit
  - bits 2-17: address



Entering and exiting ISP mode — XC9500XL/XV
-------------------------------------------

Before any programming or readout can be done, the device needs to be put into ISP mode.
For that purpose, the ``ISPEN`` or ``ISPENC`` instructions can be used.
Both instructions use the ``ISPENABLE`` register, which is 6 bits long.  Its meaning, if any,
is unknown.

To enter ISP mode:

- shift ``ISPEN`` or ``ISPENC`` into IR
- shift 0s into DR
- go to Run-Test/Idle state for at least 1 clock

If the ``ISPEN`` instruction is used, all outputs will be put in high-Z with weak pull-ups while ISP mode is active.
If the ``ISPENC`` ("clamp" mode) instruction is used, all output and output enable signals will be snapshotted
and outputs will continue driving the last value while ISP mode is active.

To exit ISP mode:

- shift ``ISPEX`` into IR
- go to Run-Test/Idle state for at least 100 µs

When ISP mode is exitted, the device will initialize itself and start normal operation.

.. todo:: verify, see if anything can be figured out about the DR


Erasing fuses — XC9500XL/XV
---------------------------

.. todo:: write me


Programming fuses — XC9500XL/XV
-------------------------------

.. todo:: write me


Reading fuses — XC9500XL/XV
---------------------------

.. todo:: write me


Blank check — XC9500XL/XV
-------------------------

.. todo:: write me


Programming sequence
====================

.. todo:: write me