Introduction, general protocol
##############################

Digilent made a family of devboards using a common custom USB protocol.
For lack of a better name, we call it the Digilent Adept protocol.

The protocol is designed to be extensible: there is a set of control requests
that can be used to query information about the board and the capabilities,
and then there is a common multiplexing and framing protocol used to
communicate with one of the device's "subsystems", such as JTAG, SPI, GPIO,
or board management.

There ar two kinds of devices implementing this protocol:

- AT90USB162-based, which have pre-flashed firmware and can be used immediately
- FX2-based, which may require firmware upload first

.. todo:: there are also FX3-based devices, but it is unclear whether they reuse
   the protocol or not

Note that Digilent has phased out this protocol — newer boards are FTDI based instead.
On FTDI-based boards, the Adept runtime libraries will emulate this protocol in software, even
assembling command structures in one part of the stack, then destructuring them in the FTDI-specific
code.  The code doing this (a normal host ``.so`` or ``.dll``) is, hilariously, called "firmware".

The protocol is implemented on devices with USB ID of ``0x1443:0x0007``.

.. todo:: ``0x1443:0x0003`` and ``0x1443:0x0005`` allegedly also exist, on
   the oldest FX2-based devices

The device exposes a single configuration and single interface.  The class,
subclass, and protocol are all-0, as would be expected.  The device has
the following endpoints:

- control endpoint 0: in addition to the usual core USB stuff, used to send
  various custom control requests
- command endpoint (bulk OUT): used to send subsystem commands
- response endpoint (bulk IN): used to receive subsystem command responses
- data out endpoint (bulk OUT): used to send large data payloads for subsystem commands
- data in endpoint (bulk IN): used to receive large data payloads for subsystem commands

The actual endpoint numbers (and their max packet size) involved depend on the device kind:

======== ====================== ==================
Endpoint FX2                    AT90USB
======== ====================== ==================
command  EP1 OUT (64 bytes)     EP1 OUT (16 bytes)
response EP1 OUT (64 bytes)     EP2 IN (16 bytes)
data out EP2 OUT (64/512 bytes) EP3 OUT (64 bytes)
data in  EP6 IN (64/512 bytes)  EP4 IN (64 bytes)
======== ====================== ==================

All numbers in the protocol are encoded as little-endian unless stated otherwise.


Identification
==============

All Adept devices will present useless data (something akin to ``"Digilent Adept USB"``) in their
USB identifier strings.  To identify the actual devboard, the "product id" should be obtained
instead (via ``GET_PRODUCT_ID`` control request, or by reading from user EEPROM area on FTDI
devices).  The product id is a 32-bit word with the following structure:

- bits 0-7: firmware id
- bits 8-19: variant id
- bits 20-31: board id

The "board id" identifies a particular devboard model.  The "variant id" identifies a variant of
that devboard (eg. what FPGA size is fitted).  The "firmware id" identifies which firmware should
be loaded for this board.

The "firmware id" can be thought of as identifying an "equivalence class" of the board from
the firmware's port of view:

- two completely different boards that both use the same set of subsystems (eg. both use DJTG+DEPP)
  and have the exact same control connections to the FX2/AT90USB are equivalent, and will have
  the same firmware id
- two minor revisions of the same board will get distinct firmware id if the connections to
  the AT90USB/FX2 were changed (eg. hooked up a "link activity" LED on the newer revision)

The known firmware ids are:

- ``0x00-0x1f``: FX2-based; ``0x01`` and ``0x02`` are based on the original FX2, all others are
  based on FX2LP
- ``0x01``: DJTG or DSPI; old JTAG-USB cable based on the original FX2, requires firmware swapping
  to switch between DJTG and DSPI
- ``0x02``: DJTG or DEPP: old USB2 module based on the original FX2, requires firmware swapping
  to switch between DJTG and DEPP
- ``0x03``: DJTG + DSPI; new FX2LP-based JTAG-USB cable
- ``0x04``: DJTG + DEPP; new FX2LP-based USB2 module
- ``0x05``: DJTG+DEPP+DSTM (used on Nexys 2)
- ``0x06``: DJTG+DEPP+DSTM+DSPI (X-board?)
- ``0x08``: DJTG+DEPP+DSTM
- ``0x09``: DJTG+DEPP+DSTM
- ``0x0a``: DJTG+DEPP+DSTM+DSPI (X-board?)
- ``0x0b``: ~DJTG+DEPP+DSTM
- ``0x0c``: DJTG+DEPP+DSTM (used on Atlys)
- ``0x0d``: DJTG+DEPP+DSTM (used on Nexys 3, FMC Carrier S6)
- ``0x0e``: DJTG+DEPP+DSTM
- ``0x0f``: ~DJTG+DEPP+DSTM
- ``0x20-0x3f``: AT90USB-based
- ``0x21``: DJTG? (JTAG-USB-FS cable)
- ``0x22``: DJTG+DEPP + prog + done (used on Basys 2)
- ``0x23``: DJTG+DEPP + ??? (used on other Basys 2 revisions?)
- ``0x26``: DJTG+DEPP+DSPI (used on Coolrunner 2 starter board)
- ``0x29``: DPIO+DSPI+DTWI+DACI+DAIO+DEMC+DGIO (IO explorer)
- ``0x2d``: DJTG+???
- ``0x2e``: DPIO+DEPP+DSPI; power control shared with DPIO (used on iCE40blink)
- ``0x50-0x6f``: FTDI-based
- ``0x50``: FT2232H DJTG+DSPI (JTAG-HS1)
- ``0x51``: FT2232H DJTG (used on JTAG-SMT1; Basys 3, Arty, USB104-A7, USRP onboard)
- ``0x52``: FT232H DJTG+DSPI (JTAG-HS2)
- ``0x53``: DJTG+SRST (JTAG-HS3; ZEDBOARD)
- ``0x54``: FT232H DJTG+DSPI+DPIO (JTAG-SMT2)
- ``0x55``: DJTG+DSPI+DPTI
- ``0x56``: DJTG+DSPI+DPTI
- ``0x57``: DJTG (ZYBO-Z7; TE07* onboard)
- ``0x58``: DJTG+DSPI
- ``0x59``: DPTI
- ``0x60``: ~DJTG+DPTI (analog discovery)
- ``0x61``: DJTG
- ``0x62``: DSPI+DPTI
- ``0x80-0x8f``: FX3-based
- ``0x80``: ???
- ``0x81``: ???

In addition to the binary product id, the boards also have a product name, which is hopefully in
some correspondence with the product id.  However, it is probably more reliable to recognize
the board by the binary id.


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

Returns the binary product ID of this board.  It is a little-endian 32-bit word, explained above.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xe9``
- ``wValue``: 0
- ``wIndex``: 0
- ``wLength``: 4


``GET_SECRET_HANDSHAKE``
------------------------

Gets a secret 32-bit handshake number from the board.  This is a strong
cryptographic protocol used to verify the board as a genuine Digilent product.
A secret 16-bit nonce must first be set via the ``SET_SECRET_HANDSHAKE``
request, then this request must be used to get a 32-bit MAC from the device.

- ``bmRequestType``: ``0xc0``
- ``bRequest``: ``0xec``
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
- ``0x05``: DSTM (FX2 FIFO interface, bit 3 in ``GET_CAPS``)
- ``0x06``: DSPI (SPI, bit 4 in ``GET_CAPS``)
- ``0x07``: DTWI (I²C / SMBus, bit 5 in ``GET_CAPS``)
- ``0x08``: DACI (UART, bit 6 in ``GET_CAPS``)
- ``0x09``: DAIO (analog I/O, bit 7 in ``GET_CAPS``)
- ``0x0a``: DEMC (electro-mechanical control, bit 8 in ``GET_CAPS``)
- ``0x0c``: DGIO (general sensor and user I/O, bit 10 in ``GET_CAPS``)

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
  - if bit 7 of byte 1 set: a 32-bit word containing "transmitted byte count" (the number of bytes sent over data out endpoint for a long command)
  - if bit 6 of byte 1 set: a 32-bit word containing "received byte count" (the number of bytes sent over data in endpoint for a long command)
  - if status code is 0: short response payload, determined by subsystem and command type

Commands and responses are short and fit in one USB packet, which can be at most 16 bytes for the relevant endpoints.

Commands come in two kinds: short and long.  Whether a command is short
or long depends only on its subsystem and command type.  A short command
simply consists of two USB transfers:

- command endpoint: command to device
- response endpoint: response from device

A long command is one that possibly takes a long time and can be aborted (via the ``SYS_ABORT`` command).
Long commands can also involve large data transfers over endpoints 3 and 4.  A long consists of the following transfers:

- command endpoint: start command to device (bit 7 of byte 2 set to 0; short payload contains command arguments, if any)
- response endpoint: response from device (if not successful, the command is aborted now)
- data out endpoint and/or data in endpoint (if needed): large payload to/from device (if both are needed, the two transfers may have to be overlapped)
- command endpoint: end command to device (bit 7 of byte 2 set to 1; no short payload present)
- response endpoint: response from device (contains actual transmitted and received byte counts, as appropriate)


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
