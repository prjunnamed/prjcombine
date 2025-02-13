DJTG: JTAG controller subsystem
###############################

This subsystem implements a JTAG controller.

The port properties on DJTG are as follows:

- bit 0: supports ``SET_SPEED`` command
- bit 1: supports ``SET_TMS_TDI_TCK`` command


``SET_SPEED``
=============

Sets the clock frequency.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x03`` (short)
- command payload: 32-bit word (requested frequency in Hz)
- response payload: 32-bit word (actual frequency in Hz)

This command returns the actual frequency used, as adjusted by the device
to match hardware capabilities.


``GET_SPEED``
=============

Gets the clock frequency.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x04`` (short)
- command payload: none
- response payload: 32-bit word (frequency in Hz)


``SET_TMS_TDI_TCK``
===================

Sets the current state of the pins.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x05`` (short)
- command payload: 3 bytes:

  - byte 0: TMS state
  - byte 1: TDI state
  - byte 2: TCK state

- response payload: none


``GET_TMS_TDI_TDO_TCK``
=======================

Gets the current state of the pins.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x06`` (short)
- command payload: none
- response payload: 4 bytes

  - byte 0: TMS state
  - byte 1: TDI state
  - byte 2: TDO state
  - byte 3: TCK state


``CLOCK_TCK``
=============

Clocks TCK for the specified number of cycles, transferring no data.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x07`` (long)
- command payload: 6 bytes

  - byte 0: TMS state
  - byte 1: TDI state
  - byte 2-5 (32-bit word): cycle count

- response payload: none
- long data OUT: none
- long data IN: none


``PUT_TDI_BITS``
================

Shifts out data sent through OUT EP3 on the TDI pin.  Optionally also shifts in data from TDO
to IN EP4.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x08`` (long)
- command payload: 6 bytes

  - byte 0:

    - 0: only shift out data on TDI
    - 1: shift out data on TDI and shift in data from TDO

  - byte 1: TMS state
  - byte 2-5 (32-bit word): bit count

- response payload: none
- long data OUT: data to shift out on TDI, ``bit_count.ceil_div(8)`` bytes long, packed LSB-first
- long data IN: if shifting in enabled, data shifted in from TDO, ``bit_count.ceil_div(8)`` bytes long, packed LSB-first; otherwise, none


``GET_TDO_BITS``
================

Shifts in data from TDO to IN EP4.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x09`` (long)
- command payload: 6 bytes

  - byte 0: TMS state
  - byte 1: TDI state
  - byte 2-5 (32-bit word): bit count

- response payload: none
- long data IN: data shifted in from TDO, ``bit_count.ceil_div(8)`` bytes long, packed LSB-first


``PUT_TMS_TDI_BITS``
====================

Shifts out data sent through OUT EP3 on the TMS and TDI pins.  Optionally also shifts in data from TDO
to IN EP4.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x0a`` (long)
- command payload: 5 bytes

  - byte 0:

    - 0: only shift out data on TDI+TMS (OUT EP4)
    - 1: shift out data on TDI+TMS and shift in data from TDO (OUT EP4 + IN EP4)

  - byte 1-4 (32-bit word): bit count

- response payload: none
- long data OUT: data to shift out on TDI+TMS, ``bit_count.ceil_div(4)`` bytes long
- long data IN: if shifting in enabled, data shifted in from TDO, ``bit_count.ceil_div(8)`` bytes long, packed LSB-first; otherwise, none

For shifting out, TDI and TMS are interleaved LSB-first as follows:

- bit 0: TDI for cycle 0
- bit 1: TMS for cycle 0
- bit 2: TDI for cycle 1
- bit 3: TMS for cycle 1
- ...


``PUT_TMS_BITS``
================

Shifts out data sent through OUT EP3 on the TMS pin.  Optionally also shifts in data from TDO
to IN EP4.

- subsystem: ``0x02`` (DJTG)
- command type: ``0x0b`` (long)
- command payload: 6 bytes

  - byte 0:

    - 0: only shift out data on TMS
    - 1: shift out data on TMS and shift in data from TDO

  - byte 1: TDI state
  - byte 2-5 (32-bit word): bit count

- response payload: none
- long data OUT: data to shift out on TMS, ``bit_count.ceil_div(8)`` bytes long, packed LSB-first
- long data IN: if shifting in enabled, data shifted in from TDO, ``bit_count.ceil_div(8)`` bytes long, packed LSB-first; otherwise, none
