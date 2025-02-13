Introduction, general protocol
##############################

Digilent made a family of devboards using a common custom USB protocol.
For lack of a better name, we call it the Digilent Adept protocol.

The protocol is designed to be extensible: there is a set of control requests
that can be used to query information about the board and the capabilities,
and then there is a common multiplexing and framing protocol used to
communicate with one of the device's "subsystems", such as JTAG, SPI, GPIO,
or board management.

The protocol is implemented by a variety of devices on the board side — some
of them are ATmega based, and come up directly implementing the protocol,
with upgradeability provided only by programming pins on the board.
Others are FX2 based, requiring firmware upload from the host before they
start speaking the protocol.

Note that Digilent has phased out this protocol — newer boards are FTDI
based instead.

The protocol is implemented on devices with USB ID of ``0x1443:0x0007``.

The device exposes a single configuration and single interface.  The class,
subclass, and protocol are all-0, as would be expected.  The device has
the following endpoints:

- control endpoint 0: in addition to the usual core USB stuff, used to send
  various custom control requests
- endpoint 1 OUT (bulk, 16 bytes max packet size): used to send subsystem commands
- endpoint 2 IN (bulk, 16 bytes max packet size): used to receive subsystem command responses
- endpoint 3 OUT (bulk, 64 bytes max packet size): used to send large data payloads for subsystem commands
- endpoint 4 IN (bulk, 64 bytes max packet size): used to receive large data payloads for subsystem commands

All numbers in the protocol are encoded as little-endian unless stated otherwise.


Control requests
================

These control requests can be sent at any time, without any particular
preparation.

A few of the requests here deal with strings.  The strings are stored
on the board in fixed-length storage (different for every string),
and are NUL-terminated, except for the (legal) case of a string that takes
up the entire length of the storage.  The requests will return the entire
contents of the string storage, possibly including garbage beyond the NUL
terminator (which is usually either all-NUL or all-``0xff``).  This garbage
should be trimmed off.


``GET_PRODUCT_NAME``
--------------------

Returns a product name describing the board.  The storage for that string
is 28 bytes long.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xe1``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 28


``GET_USER_NAME``
-----------------

Returns a "user name" describing the board.  It is a 16-byte string field.
Allegedly it can be set by the user.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xe2``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 16


``SET_USER_NAME``
-----------------

Sets the user name of this board.  It can be up to 16 bytes long.
Doesn't seem to be actually supported on all boards?

- ``bmRequestType``: ``0x40``
- ``bRequest``: ``0xe3``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 16


``GET_SERIAL_NUMBER``
---------------------

Returns the serial number of this board.  It is 12 bytes long.  It can be
reprogrammed by the user.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xe4``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 12


``SET_SERIAL_NUMBER``
---------------------

Sets the serial number of this board.  It can be up to 12 bytes long.

- ``bmRequestType``: ``0x40``
- ``bRequest``: ``0xe5``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 12


``GET_FIRMWARE_VERSION``
------------------------

Returns the firmware version of this board.  It is a 16-bit word.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xe6``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 2


``GET_CAPS``
------------

Returns the capabilities of this board.  It is a 32-bit word.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xe7``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 4

The capabilities are a bitfield describing the available subsystems:

- bit 0: DJTG
- bit 1: DPIO
- bit 2: DEPP
- bit 3: DSTM
- bit 4: DSPI
- bit 5: DTWI
- bit 6: DACI
- bit 7: DAIO
- bit 8: DEMC
- bit 9: DDCI
- bit 10: DGIO


``SET_SECRET_HANDSHAKE``
------------------------

Sets a 16-bit number used in the secret handshake.

- ``bmRequestType``: ``0x40``
- ``bRequest``: ``0xe8``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 2


``GET_PRODUCT_ID``
------------------

Returns the binary product ID of this board.  It is a little-endian 32-bit word.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xe9``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 4

This word has the following bitfields:

- bits 0-7: firmware identifier
- bits 8-19: variant identifier
- bits 20-31: product identifier

The product identifier describes the particular board.  The variant identifier
describes its variant, such as what FPGA size has been fitted to it.


``GET_SECRET_HANDSHAKE``
------------------------

Gets a secret 32-bit handshake number from the board.  This is a strong
cryptographic protocol used to verify the board as a genuine Digilent product.
A secret 16-bit nonce must first be set via the ``SET_SECRET_HANDSHAKE``
request, then this request must be used to get a 32-bit MAC from the device.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xe9``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 4

To verify the device as a genuine Digilent board, check the MAC is correct
as follows::

    fn correct_mac(nonce: u16, mac: u32) -> bool {
        let byte_nonce = (((nonce >> 8) ^ nonce) & 0xff) as u32;
        const DIGILENT_PUBLIC_KEY: u32 = 0x69676944;
        mac == (DIGILENT_PUBLIC_KEY ^ (byte_nonce | byte_nonce << 8 | byte_nonce << 16 | byte_nonce << 24))
    }


Subsystem commands
==================

All commands other than the above control requests are handled via a uniform
protocol over endpoints 1-4.  Commands are targetted to particular "subsystem"
and a particular "port" (or, in other words, instance) of that subsystem.
The following subsystems can exist:

- ``0x00``: SYS (manages other subsystems, always present, always 1 "port")
- ``0x01``: DMGT (general board management, always present, always 1 "port")
- ``0x02``: DJTG (JTAG, bit 0 in ``GET_CAPS``)
- ``0x03``: DPIO (simple GPIO, bit 1 in ``GET_CAPS``)
- ``0x04``: DEPP (EPP-like parallel port, bit 2 in ``GET_CAPS``)
- ``0x06``: DSPI (SPI, bit 4 in ``GET_CAPS``)

.. todo:: list incomplete

Commands are sent on endpoint 1, and have the following general format:

- byte 0: command length in bytes, minus one
- byte 1: target subsystem
- byte 2:

  - bits 0-6: command type
  - bit 7:

    - ``0``: this is a short command, or the start of a long command
    - ``1``: this is the end of a long command

- byte 3: target port (if not applicable, set to 0)
- bytes 4 and up (if any): short payload, determined by subsystem and command type

Responses to commands are received on endpoint 2, and have the following general format:

- byte 0: response length in bytes, minus one
- byte 1:

  - bits 0-5: status code

    - ``0x00``: success, payload determined by command type
    - ``0x01``: command not supported (no payload)

    - ``0x03``: resource in use (attempt to enable a port that is already enabled, or that uses resources shared with another enabled port), no payload
    - ``0x04``: port disabled error (attempt to send a non-enable command to disabled port), no payload
    - ``0x05``: DEPP address timeout (no payload)
    - ``0x06``: DEPP data timeout

      - payload: 32-bit number (unknown semantics)

    - ``0x0d``: command parameter out of range (no payload)

    - ``0x31``: unknown subsystem (no payload)
    - ``0x32``: unknown command (no payload)

  - bit 6: if set, a "received byte count" field is present in the reply
  - bit 7: if set, a "transmitted byte count" field is present in the reply

- bytes 2 and up (if any): several packed fields, in order:

  - if status code is non-0: error payload specific to status code
  - if bit 7 of byte 1 set: a 32-bit word containing "transmitted byte count" (the number of bytes sent over OUT EP3 for a long command)
  - if bit 6 of byte 1 set: a 32-bit word containing "received byte count" (the number of bytes sent over IN EP4 for a long command)
  - if status code is 0: short response payload, determined by subsystem and command type

Commands and responses are short and fit in one USB packet, which can be at most 16 bytes for the relevant endpoints.

Commands come in two kinds: short and long.  Whether a command is short
or long depends only on its subsystem and command type.  A short command
simply consists of two USB transfers:

- OUT EP1: command to device
- IN EP2: response from device

A long command is one that possibly takes a long time and can be aborted (via the ``SYS_ABORT`` command).
Long commands can also involve large data transfers over endpoints 3 and 4.  A long consists of the following transfers:

- OUT EP1: start command to device (bit 7 of byte 2 set to 0; short payload contains command arguments, if any)
- IN EP2: response from device (if not successful, the command is aborted now)
- OUT EP3 and/or IN EP4 (if needed): large payload to/from device (if both are needed, the two transfers may have to be overlapped)
- OUT EP1: end command to device (bit 7 of byte 2 set to 1; no short payload present)
- IN EP2: response from device (contains actual transmitted and received byte counts, as appropriate)


System management commands
==========================

The "subsystem" 0 ("SYS") is special, always present, and always enabled.


``SYS_ABORT`` command
---------------------

This command can be sent in the middle of a long command to abort the transfer.

- subsystem: ``0x00`` (SYS)
- command type: ``0x02`` (short)
- port: N/A, always 0
- command payload: none
- response payload: none


``SYS_RESET`` command
---------------------

This command can be sent at any time to reset the current state of the device.
This involves disabling all ports.

- subsystem: ``0x00`` (SYS)
- command type: ``0x03`` (short)
- port: N/A, always 0
- command payload: 32-bit word
- response payload: 32-bit word

For unknown reasons, this command takes a 32-bit word payload, and returns as the response payload another 32-bit word, which is equal to ``0x7a - command_payload``.


General subsystem commands
==========================

These commands apply to all supported subsystems except ``SYS`` and ``DMGT``.


``ENABLE`` command
------------------

This command enables a subsystem port, making it ready for use.  The only
commands that can be sent to a disabled port are ``ENABLE`` and
``GET_CAPABILITIES``.  All ports start out as disabled.  A port will fail
to enable if it is already enabled, or if another port using the same hardware
resources is currently enabled.

- subsystem: any except SYS and DMGT
- command type: ``0x00`` (short)
- port: port index
- command payload: none
- response payload: none


``DISABLE`` command
-------------------

Disables a subsystem port, undoing the ``ENABLE`` command.

- subsystem: any except SYS and DMGT
- command type: ``0x01`` (short)
- port: port index
- command payload: none
- response payload: none


``GET_PORT_PROPERTIES`` command
-------------------------------

Returns the properties of a given port of a subsystem, and also the available port count.

- subsystem: any except SYS and DMGT
- command type: ``0x02`` (short)
- port: port index; call with port 0 to obtain number of available ports
- command payload: 1 byte: requested data byte count (can be 1 or 5)
- response payload:

  - byte 0: port count
  - bytes 1-4 (if requested): 32-bit word, port properties; exact meaning depends on subsystem
