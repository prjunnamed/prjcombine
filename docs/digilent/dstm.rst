DSTM: FX2 FIFO interface subsystem
###################################

This subsystem implements a high-speed parallel interface using the FX2 FIFO mode.

There are no port properties on DSTM.


Protocol
========

.. todo:: document


``IO``
======

Performs a data transfer.

- subsystem: ``0x05`` (DSTM)
- command type: ``0x03`` (long)
- command payload: 8 bytes:

  - bytes 0-3: number of bytes to send
  - bytes 4-7: number of bytes to receive

- response payload: none
- long data OUT: data to send
- long data IN: received data


``IO_EX``
=========

Performs a high-speed data transfer.

- subsystem: ``0x05`` (DSTM)
- command type: ``0x04`` (long)
- command payload: 8 bytes:

  - bytes 0-3: number of bytes to send
  - bytes 4-7: number of bytes to receive

- response payload: none
- long data OUT: data to send
- long data IN: received data
