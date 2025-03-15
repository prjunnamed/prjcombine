# DSPI (SPI controller)

This subsystem implements an SPI controller with a single chip-select pin.

The port properties on DSPI are as follows:

- bit 0: supports `SET_SPEED` command
- bit 1: supports MSB-first shift
- bit 2: supports LSB-first shift
- bit 3: supports `SET_DELAY` and `GET_DELAY` commands
- bit 4: supports SPI mode 0
- bit 5: supports SPI mode 1
- bit 6: supports SPI mode 2
- bit 7: supports SPI mode 3


## `SET_SPEED`

Sets the clock frequency.

- subsystem: `0x06` (DSPI)
- command type: `0x03` (short)
- command payload: 32-bit word (requested frequency in Hz)
- response payload: 32-bit word (actual frequency in Hz)

This command returns the actual frequency used, as adjusted by the device
to match hardware capabilities.


## `GET_SPEED`

Gets the clock frequency.

- subsystem: `0x06` (DSPI)
- command type: `0x04` (short)
- command payload: none
- response payload: 32-bit word (frequency in Hz)


## `SET_SPI_MODE`

Sets the SPI mode and shift direction.

- subsystem: `0x06` (DSPI)
- command type: `0x05` (short)
- command payload: 1 byte:
  - bits 0-1: SPI mode
  - bit 2: shift direction
    - 0: MSB first
    - 1: LSB first
- response payload: none


## `SET_SELECT`

Sets the current state of the SPI CS# pin.

- subsystem: `0x06` (DSPI)
- command type: `0x06` (short)
- command payload: 1 byte:
  - 0: drive CS# low (active)
  - 1: drive CS# high (inactive)
- response payload: none


## `PUT`

Sends and optionally receives bytes.

- subsystem: `0x06` (DSPI)
- command type: `0x07` (long)
- command payload: 7 bytes:
  - byte 0: CS# state to set before starting the operation
  - byte 1: CS# state to set after finishing the operation
  - byte 2:
    - 0: just send bytes
    - 1: send and receive bytes
  - bytes 3-6: byte count to send (and, optionally, receive)
- response payload: none
- long data OUT: data to send, count as specified in command payload
- long data IN: if receive enabled, data received from device, count as specified in command payload; otherwise, none


## `GET`

Receives bytes.

- subsystem: `0x06` (DSPI)
- command type: `0x08` (long)
- command payload: 7 bytes:
  - byte 0: CS# state to set before starting the operation
  - byte 1: CS# state to set after finishing the operation
  - byte 2: the value to drive on the COPI pin while receiving bytes
  - bytes 3-6: byte count to receive
- response payload: none
- long data OUT: none
- long data IN: data received from device, count as specified in command payload


## `SET_DELAY`

Sets the inter-byte delay.  This is the amount of time to sleep between
each byte sent/received on the SPI interface.

- subsystem: `0x06` (DSPI)
- command type: `0x09` (short)
- command payload: 32-bit word (inter-byte delay in µs)
- response payload: none

Note that overly large values of delay will be rejected with the "parameter
out of range" error (overly large means ≥256 bytes on iCEblink40).


## `GET_DELAY`

Gets the inter-byte delay.

- subsystem: `0x06` (DSPI)
- command type: `0x0a` (short)
- command payload: none
- response payload: 32-bit word (delay in µs)