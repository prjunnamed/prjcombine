# DMGT (board management)

This subsystem is used for general board control.  It is always present and always enabled, but its exact capabilities must be queried.


## `DMGT_GET_CAPABILITIES`

Returns capabilities of the DMGT subsystem.  Always supported.

- subsystem: `0x01` (DMGT)
- command type: `0x02` (short)
- port: N/A, always 0
- command payload: none
- response payload: 32-bit capabilities word
  - bit 0: power control (`DMGT_POWER_ON`, `DMGT_POWER_OFF`)
  - bit 1: config reset (`DMGT_CONFIG_RESET`)
  - bit 2: user reset (`DMGT_USER_RESET`)
  - bit 3: query DONE (`DMGT_QUERY_DONE`)
  - bit 5: query power state (`DMGT_QUERY_POWER_STATE`)
  - bit 6: power supply monitoring (`DMGT_GET_POWER_SUPPLY_*`)


## `DMGT_POWER_ON`

Turns on power to the rest of the board.

- subsystem: `0x01` (DMGT)
- command type: `0x03` (short)
- port: N/A, always 0
- command payload: none
- response payload: none


## `DMGT_POWER_OFF`

Turns off power to the rest of the board.

Note that on some boards (eg. iCEblink40), this command only momentarily switches off power, turning it back on within half a second or so.

- subsystem: `0x01` (DMGT)
- command type: `0x04` (short)
- port: N/A, always 0
- command payload: none
- response payload: none


## `DMGT_CONFIG_RESET`

Sets the state of the FPGA configuration reset pin (eg. `PROG_B` on Xilinx devices).

- subsystem: `0x01` (DMGT)
- command type: `0x06` (short)
- port: N/A, always 0
- command payload: 1 byte
  - 0: deassert reset to the device
  - 1: assert reset to the device
- response payload: none

Note that the reset here is active-high, which is the inverse of the state of the actual `PROG_B` pin.


## `DMGT_USER_RESET`

Sets the state of the user reset pin on the FPGA, if the board has one.

- subsystem: `0x01` (DMGT)
- command type: `0x07` (short)
- port: N/A, always 0
- command payload: 1 byte
  - 0: deassert reset to the device
  - 1: assert reset to the device
- response payload: none


## `DMGT_QUERY_DONE`

Returns the state of the `DONE` pin on the FPGA.

- subsystem: `0x01` (DMGT)
- command type: `0x08` (short)
- port: N/A, always 0
- command payload: none
- response payload: 1 byte (the LSB is the state of the DONE pin)


## `DMGT_QUERY_POWER_STATE`

Returns the state of the board's power supply.

- subsystem: `0x01` (DMGT)
- command type: `0x0c` (short)
- port: N/A, always 0
- command payload: none
- response payload: 1 byte
  - 0: power is off
  - 1: power is on


## `DMGT_GET_POWER_SUPPLY_COUNT`

Returns the number of power supplies on the board that can be monitored.

- subsystem: `0x01` (DMGT)
- command type: `0x0d` (short)
- port: N/A, always 0
- command payload: none
- response payload: 32-bit word (number of power supplies)


## `DMGT_GET_POWER_SUPPLY_DATA`

Returns the current state of the given power supply.

- subsystem: `0x01` (DMGT)
- command type: `0x0d` (short)
- port: N/A, always 0
- command payload: 1 byte (power supply index)
- response payload: 5 32-bit words:
  - word 0: raw voltage value
  - word 1: raw current value
  - word 2: raw power value
  - word 3: raw temperature value
  - word 4: status; status bits for all power supplies on the board (up to 8 of them) are packed into this word, 4 bits for each supply:
    - bit 0: power supply is on
    - bit 1: power supply voltage out of spec
    - bit 2: overcurrent condition
    - bit 3: overtemperature condition

All sensor values returned are unsigned integers which need to have the conversion factor applied to them.


## `DMGT_GET_POWER_SUPPLY_PROPERTIES`

Returns the scaling factors that should be applied to the raw sensor values of the given power supply.

- subsystem: `0x01` (DMGT)
- command type: `0x0f` (short)
- port: N/A, always 0
- command payload: 1 byte (power supply index)
- response payload: 4 32-bit words:
  - word 0: voltage scaling factor (in µV units)
  - word 1: current scaling factor (in µA units)
  - word 2: power scaling factor (in µW units)
  - word 3: temperature scaling factor (in µK units)

The raw values from the sensors should be multiplied by these constant scaling factors to obtain the actual physical values.


## `DMGT_GET_POWER_SUPPLY_LABEL`

Returns the free-form label string describing the given power supply.

- subsystem: `0x01` (DMGT)
- command type: `0x10` (short)
- port: N/A, always 0
- command payload: 1 byte (power supply index)
- response payload: 32-byte string (NUL-terminated)
