DEPP: parallel port subsystem
#############################

This subsystem implements a simple parallel interface based on the EPP parallel
printer port.

There are no port properties on DEPP.


Protocol
========

The protocol is asymmetric, and involves two devices: the controller and the peripherial.
The Digilent USB device has the role of the controller, while the FPGA (or whatever other
"main" device on the devboard) is the peripherial.

The hardware protocol allows 4 core operations to be performed:

- write address (8 bits to peripherial)
- read address (8 bits from peripherial)
- write data (8 bits to peripherial)
- read data (8 bits from peripherial)

All operations are initiated by the controller, although the peripherial can do flow control
by indefinitely delaying its acknowledgement strobes.

Conceptually, the peripherial exposes up to 256 8-bit registers, which can be read or written:
the "write address" operation is used to select the register, while the "read/write data" operations
are used to access the selected register.

The DEPP subsystem doesn't directly expose the raw hardware operations. Instead, it provides
the higher-level primitives:

- read register (input: 8-bit address; output: 8-bit data)
- write register (input: 8-bit address, 8-bit data; no output)


Low-level protocol
------------------

The protocol involves the following signals:

- ``data`` (data bus, also known as ``DB``): 8-bit, bidirectional
- ``astb#`` (active-low address strobe): 1-bit, controller to peripherial
- ``dstb#`` (active-low data strobe): 1-bit, controller to peripherial
- ``wr#`` (active-low write enable): 1-bit, controller to peripherial
- ``wait#`` (active-low busy signal): 1-bit, peripherial to controller

In the idle state, ``astb#`` and ``dstb#`` are both 1.  The peripherial is tristating the bus, and driving
0 on the ``wait#`` line.

Protocol operations are done as follows:

1. Controller waits for ``wait#`` to be 0.
2. Controller sets the ``wr#`` signal to 0 for a write operation, to 1 for a read operation.
3. For a read operation, controller tristates the data bus; for a write operation, controller drives the data on the data bus.
4. Controller waits a bit for the signals to settle.
5. Controller drives either ``astb#`` (for address operation) or ``dstb#`` (for data operation) to 0.
6. Peripherial notices the strobe signal, and performs the requested operation.
7. For a read, peripherial drives the read data onto the data bus, and waits a bit.
8. Peripherial drives 1 on ``wait#``, informing the controller that operation is complete (and, in case of read, that data is available).
9. Controller notices the ``wait#`` signal and considers the operation to be complete.
10. For a read, controller latches the data from the data bus.
11. Controller returns both strobe lines to the idle state of 1.
12. Peripherial notices the strobe line going back up.
13. Peripherial tristates the data bus.
14. Peripherial returns ``wait#`` to the idle state of 0.

The ``wr#`` signal must be held stable by the controller while the ``astb#`` or ``dstb#``
signal is low.  For a write operation, the same applies to the data bus.

The read data must be held stable by the peripherial from the moment it drives a 1 on ``wait#``
until the controller releases the strobe signal.

The protocol is not all that well defined, particularly when timing is concerned.  Sorry.


``SET_TIMEOUT``
===============

Sets the transaction timeout.

- subsystem: ``0x04`` (DEPP)
- command type: ``0x03`` (short)
- command payload: 32-bit word (requested timeout in ns)
- response payload: 32-bit word (actual timeout in ns)

This command returns the actual timeout used, as adjusted by the device
to match hardware capabilities.

On Basys 2 and iCEblink40, the valid timeout range is 5120ns-261120ns, with a step of 1024ns.


``PUT_REG_REPEAT``
==================

Performs multiple writes, all to the same register address.

- subsystem: ``0x04`` (DEPP)
- command type: ``0x04`` (long)
- command payload: 5 bytes:

  - byte 0: register address
  - bytes 1-4: data byte count

- response payload: none
- long data OUT: the data bytes to write to the given register, count as specified in the command
- long data IN: none


``GET_REG_REPEAT``
==================

Performs multiple reads, all from the same register address.

- subsystem: ``0x04`` (DEPP)
- command type: ``0x05`` (long)
- command payload: 5 bytes:

  - byte 0: register address
  - bytes 1-4: data byte count

- response payload: none
- long data OUT: none
- long data IN: the data bytes read from the given register, count as specified in the command


``PUT_REGSET``
==============

Performs multiple writes, to varying register addresses.

- subsystem: ``0x04`` (DEPP)
- command type: ``0x06`` (long)
- command payload: 4 bytes:

  - bytes 0-3: number of writes to perform

- response payload: none
- long data OUT: register addresses and data values, ``2 * numer_of_writes`` bytes total, interleaved as follows:

  - byte 0: register address for write #0
  - byte 1: data for write #0
  - byte 2: register address for write #1
  - byte 3: data for write #1
  - ...

- long data IN: none


``GET_REGSET``
==============

Performs multiple reads, from varying register addresses.

- subsystem: ``0x04`` (DEPP)
- command type: ``0x07`` (long)
- command payload: 4 bytes:

  - bytes 0-3: number of reads to perform

- response payload: none
- long data OUT: register addresses to read, one byte per requested read
- long data IN: the data bytes read from the given registers, byte count as specified in the command
