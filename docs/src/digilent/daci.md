# DACI (UART)

This subsystem implements an UART interface.  DACI stands for "asynchronous communication
interface".

The port properties for DACI are as follows:

- bit 0: port implements a DTE device
- bit 1: port implements a DCE device
- bit 2: port implements RTS/CTS hardware handshaking
- bit 3: port implements XON/XOFF hardware handshaking
- bit 4: port supports the `SET_BAUD` command
- bit 5: port supports setting of number of stop bits
- bit 6: port supports setting of number of data bits
- bit 7: port supports "none" parity
- bit 8: port supports "odd" parity
- bit 9: port supports "even" parity
- bit 10: port supports "mark" parity
- bit 11: port supports "space" parity


# `PUT`

Transmits bytes.

- subsystem: `0x08` (DACI)
- command type: `0x03` (long)
- command payload: 4 bytes:
  - bytes 0-3: byte count to send
- response payload: none
- long data OUT: data to send
- long data IN: none


# `GET`

Receives bytes.

- subsystem: `0x08` (DACI)
- command type: `0x04` (long)
- command payload: 4 bytes:
  - bytes 0-3: max byte count to receive
- response payload: none
- long data OUT: none
- long data IN: received data (up to the byte count specified in the command)

This command may receive fewer bytes than asked for: the long response should
be checked for the actual received byte count.


# `GET_MODE`

Gets current UART mode.

- subsystem: `0x08` (DACI)
- command type: `0x05` (short)
- command payload: none
- response payload: 3 bytes:
  - byte 0: number of data bits (5 to 8)
  - byte 1: encoded number of stop bits:
    - 1: 1 stop bit
    - 2: 1.5 stop bits
    - 3: 2 stop bits
  - byte 2: parity mode:
    - 0: none
    - 1: odd
    - 2: even
    - 3: mark
    - 4: space


# `SET_MODE`

Sets current UART mode.

- subsystem: `0x08` (DACI)
- command type: `0x06` (short)
- command payload: 3 bytes:
  - byte 0: number of data bits (5 to 8)
  - byte 1: encoded number of stop bits:
    - 1: 1 stop bit
    - 2: 1.5 stop bits
    - 3: 2 stop bits
  - byte 2: parity mode:
    - 0: none
    - 1: odd
    - 2: even
    - 3: mark
    - 4: space
- response payload: none

Note that not all ports support the full range of possible values.  Check with
`GET_MODE` to see what mode has been actually set.


# `SET_BAUD`

Sets the baud rate.

- subsystem: `0x08` (DACI)
- command type: `0x07` (short)
- command payload: 32-bit word (requested baud rate)
- response payload: 32-bit word (actual baud rate)

This command returns the actual baud rate used, as adjusted by the device
to match hardware capabilities.

TODO: driver seems to expect 8 bytes returned, but this looks like a bug


# `GET_BAUD`

Gets the baud rate.

- subsystem: `0x08` (DACI)
- command type: `0x08` (short)
- command payload: none
- response payload: 32-bit word (baud rate)


# `QUERY_STATUS`

Gets the current status of the port.

- subsystem: `0x08` (DACI)
- command type: `0x09` (short)
- command payload: none
- response payload: 8 bytes:
  - bytes 0-1: number of bytes currently in the transmit buffer
  - bytes 2-3: number of bytes currently in the receive buffer
  - bytes 4-7: flags:
    - bit 0: transmit buffer is halted
    - bit 1: receive buffer is in blocking mode
    - bit 2: transmit stalled due to flow control
    - bit 3: receive stalled due to flow control
    - bit 4: transmit flow control enabled
    - bit 5: receive flow control enabled


# `GET_BUFFER_SIZE`

Gets the buffer sizes on the device.

- subsystem: `0x08` (DACI)
- command type: `0x0a` (short)
- command payload: none
- response payload: 4 bytes:
  - bytes 0-1: transmit buffer size in bytes
  - bytes 2-3: receive buffer size in bytes


# `PURGE_BUFFER`

Purges (forcefully empties, discarding contents) transmit and/or receive buffers
on the device.

- subsystem: `0x08` (DACI)
- command type: `0x0b` (short)
- command payload: 2 bytes:
  - byte 0:
    - 0: don't purge transmit buffer
    - 1: purge transmit buffer
  - byte 1:
    - 0: don't purge receive buffer
    - 1: purge receive buffer
- response payload: none


# `HALT_TX`

Halts or unhalts the transmit buffer.  While the transmit buffer is halted,
no bytes will be sent over the line, and `PUT` commands that would normally
block waiting for buffer space will return an error instead.

- subsystem: `0x08` (DACI)
- command type: `0x0c` (short)
- command payload: 1 byte:
  - 0: unhalt transmit buffer
  - 1: halt transmit buffer
- response payload: none


# `SET_RX_BLOCK`

Sets the receive buffer to blocking or non-blocking mode.  When in non-blocking
mode, `GET` command will always return immediately, returning whatever is available
in the buffer.  When in blocking mode, it will wait for the full number of requested
bytes to be available.

- subsystem: `0x08` (DACI)
- command type: `0x0d` (short)
- command payload: 1 byte:
  - 0: set non-blocking mode
  - 1: set blocking mode
- response payload: none


# `SET_RTS_CTS_ENABLE`

Enables or disables hardware RTS/CTS control flow.

- subsystem: `0x08` (DACI)
- command type: `0x0e` (short)
- command payload: 1 byte:
  - 0: disable RTS/CTS control flow
  - 1: enable RTS/CTS control flow
- response payload: none


# `SET_XON_XOFF_ENABLE`

Enables or disables hardware XON/XOFF control flow.

- subsystem: `0x08` (DACI)
- command type: `0x0f` (short)
- command payload: 1 byte:
  - 0: disable XON/XOFF control flow
  - 1: enable XON/XOFF control flow
- response payload: none
