# DPIO (GPIO)

This subsystem is used for simple GPIO control.  A single "port" of this subsystem can support
up to 32 GPIO pins.

The port properties on DPIO are as follows:

- bit 0: supports inter-byte delay on stream commands (`GET_STREAM_TIMING` and `SET_STREAM_TIMING`)
- bit 1: supports data streaming command (`STREAM_STATE`)


## `GET_PIN_MASK`

Returns available pins and their allowed directions.

- subsystem: `0x03` (DPIO)
- command type: `0x03` (short)
- command payload: none
- response payload: two 32-bit words:
  - word 0: bitmask of output-capable pins
  - word 1: bitmask of input-capable pins

Pins can be input-only, output-only, or bidirectional.  Bidirectional pins will have bits
set in both masks, while input-only/output-only pins will have a bit set only in only one
of the masks.


## `SET_PIN_DIR`

Sets pin directions.

- subsystem: `0x03` (DPIO)
- command type: `0x04` (short)
- command payload: 32-bit word: bitmask of pins that should be set to output
- response payload: 32-bit word: bitmask of pins currently set to output

This command returns the actual mask that has been set.  Note that, as is customary
for garbage GPIO controllers, the pins that have been newly set as outputs will immediately
start to drive a 0 value, and sending a `SET_PIN_STATE` beforehand will not change this.


## `GET_PIN_DIR`

Returns current pin directions.

- subsystem: `0x03` (DPIO)
- command type: `0x05` (short)
- command payload: none
- response payload: 32-bit word: bitmask of pins currently set to output


## `SET_PIN_STATE`

Sets state of output pins.

- subsystem: `0x03` (DPIO)
- command type: `0x06` (short)
- command payload: 32-bit word: bitmask of ouptut state for all pins (bits corresponding to input pins are ignored)
- response payload: none


## `GET_PIN_STATE`

Returns the current state of all pins.

- subsystem: `0x03` (DPIO)
- command type: `0x07` (short)
- command payload: none
- response payload: 32-bit word: bitmask of current state of all pins


## `SET_STREAM_TIMING`

Sets requested timing for the `STREAM_STATE` command.

- subsystem: `0x03` (DPIO)
- command type: `0x08` (short)
- command payload: two 32-bit words:
   - word 0: delay in nanoseconds from input sampling to output update
   - word 1: delay in nanoseconds from output update to input sampling
- response payload: two 32-bit words: same format as command payload

This command returns the actual timings that will be used (which are adjusted from the requested ones as required by hardware capabilities).


## `GET_STREAM_TIMING`

Gets the timing for the `STREAM_STATE` command.

- subsystem: `0x03` (DPIO)
- command type: `0x09` (short)
- command payload: none
- response payload: two 32-bit words: same format as `SET_STREAM_TIMING`


## `STREAM_STATE`

Performs streaming I/O.  Can access only the first 8 pins.  Pins will be sampled and updated
in a tight loop, and transferred to/from the host as long command payload.

- subsystem: `0x03` (DPIO)
- command type: `0x0a` (long)
- command short payload: 6 bytes
  - byte 0:
    - 0: no output performed
    - 1: stream output data
  - byte 1:
    - 0: no input performed
    - 1: stream input data
  - bytes 2-5: 32-bit word, number of bytes to be transferred
- response payload (end): 1 byte; 0 if all data was transferred at requested rate,
  1 if a hang has occured during the transfer (no buffer space was available and
  the transfer was temporarily paused)
- long data OUT: if output requested, the output data, byte count as specified in the command payload; otherwise, none
- long data IN: if input requested, the input data, byte count as specified in the command payload; otherwise, none

Each sample will be transferred as one byte over the long payload endpoints.  Note that
the documentation doesn't say whether, for a given sample, output or input is performed first.