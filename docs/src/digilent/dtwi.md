# DTWI (I2C controller)

This subsystem implements an I²C / SMBus controller and/or peripherial.  The name stands
for "two-wire interface", lest Phillips lawyers get unhappy.

The port properties on DTWI are as follows:

- bit 0: supports operating as a controller
- bit 1: supports operating as a peripherial
- bit 2: supports operating as a controller in multi-controller environment
- bit 3: supports the `CONTROLLER_BATCH` command
- bit 4: supports the `SET_SPEED` command
- bit 5: supports SMB alert pin
- bit 6: supports SMB suspend pin
- bit 7: supports SMB packet error checking

The subsystem supports only 7-bit addresses.  The addresses used in the protocol are specified
as numbers in the range of 0-127.

When operating in peripherial mode, the device functions as a simple bidirectional FIFO with
a predetermined buffer size in each direction.  The buffer in the USB host → device direction
is called the Tx buffer, while the buffer in the device → USB host direction is called the Rx
buffer.

Except, it appears the peripherial mode is a lie.  The functionality is entirely nopped
out in the driver.


## `SET_SPEED`

Sets the clock frequency (for controller operation).

- subsystem: `0x07` (DTWI)
- command type: `0x03` (short)
- command payload: 32-bit word (requested frequency in Hz)
- response payload: 32-bit word (actual frequency in Hz)

This command returns the actual frequency used, as adjusted by the device
to match hardware capabilities.


## `GET_SPEED`

Gets the clock frequency (for controller operation).

- subsystem: `0x07` (DTWI)
- command type: `0x04` (short)
- command payload: none
- response payload: 32-bit word (frequency in Hz)


## `CONTROLLER_PUT`

Performs a "put" operation as a controller.  This consists of:

- a START condition
- sending an address byte with the specified address and a write operation selected
- sending the specified payload bytes
- a STOP condition

The command is:

- subsystem: `0x07` (DTWI)
- command type: `0x05` (long)
- command payload: 3 bytes:
  - byte 0: peripherial address
  - bytes 1-2: number of bytes to send
- response payload: none
- long data OUT: the bytes to send
- long data IN: none


## `CONTROLLER_GET`

Performs a "get" operation as a controller.  This consists of:

- a START condition
- sending an address byte with the specified address and a read operation selected
- receiving the specified number of bytes; each byte aside of the last one will be ACKed by the controller
- a STOP condition

The command is:

- subsystem: `0x07` (DTWI)
- command type: `0x06` (long)
- command payload: 3 bytes:
  - byte 0: peripherial address
  - bytes 1-2: number of bytes to receive
- response payload: none
- long data OUT: none
- long data IN: received bytes


## `CONTROLLER_PUT_GET`

Performs a "put+get" operation as a controller.  This consists of:

- a START condition
- sending an address byte with the specified address and a write operation selected
- sending the specified payload bytes
- optionally, waiting a specified amount of time (0-65535µs)
- a repeated START condition
- sending an address byte with the specified address and a read operation selected
- receiving the specified number of bytes; each byte aside of the last one will be ACKed by the controller
- a STOP condition

The command is:

- subsystem: `0x07` (DTWI)
- command type: `0x07` (long)
- command payload: 7 bytes:
  - byte 0: peripherial address
  - bytes 1-2: number of bytes to send
  - bytes 3-4: time to wait between send and receive, in µs
  - bytes 5-6: number of bytes to receive
- response payload: none
- long data OUT: the bytes to send
- long data IN: received bytes


## `CONTROLLER_BATCH`

Performs a batch operation as a controller.

- subsystem: `0x07` (DTWI)
- command type: `0x08` (long)
- command payload: 5 bytes:
  - bytes 0-1: total number of bytes to send (the batch command stream)
  - bytes 2-3: total number of bytes to receive
  - byte 4: ????? always 0, possible driver bug
- response payload: none
- long data OUT: the bytes to send (batch command stream)
- long data IN: received bytes

The data sent as payload to this command is the batch command stream, with interleaved
batch commands and data to transmit.  The data received is the concatenation of received
data for all batch commands submitted.

The batch commands are variable-length and start with a single-byte opcode.  The low nibble
of the opcode is always equal to the length of the "header" of the batch command, while
the high nibble is the command type.  For the `BATCH_PUT` command, the data to transmit
immediately follows the header.  All other commands consist of just the header.  The batch
commands are:

- `STOP` (sends a STOP condition)
  - byte 0: opcode (`0x11`)
- `START_WRITE` (sends a START condition, then sends an address byte with write direction)
  - byte 0: opcode (`0x22`)
  - byte 1: peripherial address
- `START_READ` (sends a START condition, then sends an address byte with read direction)
  - byte 0: opcode (`0x32`)
  - byte 1: peripherial address
- `REP_START_WRITE` (sends a repeated START condition, then sends an address byte with write direction)
  - byte 0: opcode (`0x42`)
  - byte 1: peripherial address
- `REP_START_READ` (sends a repeated START condition, then sends an address byte with read direction)
  - byte 0: opcode (`0x52`)
  - byte 1: peripherial address
- `PUT` (sends data bytes)
  - byte 0: opcode (`0x63`)
  - bytes 1-2: number of bytes to send
  - bytes 3 and up: the bytes to send
- `GET` (receives data bytes)
  - byte 0: opcode (`0x73`)
  - bytes 1-2: number of bytes to receive
- `WAIT` (waits a specified amount of time)
  - byte 0: opcode (`0x83`)
  - bytes 1-2: amount of time to wait, in µs


## `SMB_QUERY_ALERT`

Queries the state of the SMBus alert pin.

- subsystem: `0x07` (DTWI)
- command type: `0x09` (short)
- command payload: none
- response payload: 1 byte, the *inverted* alert pin state:
  - 0: alert pin inactive (set to 1)
  - 1: alert pin active (set to 0)


## `SMB_SET_SUSPEND`

Sets the state of the SMBus suspend pin.

- subsystem: `0x07` (DTWI)
- command type: `0x0a` (short)
- command payload: 1 byte, the *inverted* suspend pin state to set:
  - 0: suspend pin inactive (set to 1)
  - 1: suspend pin active (set to 0)
- response payload: none


## `SMB_PEC_ENABLE`

Enables SMBus PEC checking.

- subsystem: `0x07` (DTWI)
- command type: `0x0b` (short)
- command payload: none
- response payload: none


## `SMB_PEC_DISABLE`

Disables SMBus PEC checking.

- subsystem: `0x07` (DTWI)
- command type: `0x0c` (short)
- command payload: none
- response payload: none
