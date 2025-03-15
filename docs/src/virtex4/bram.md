# Block RAM

The `BRAM` tile contains one bel, the block RAM. It corresponds to four vertically stacked interconnect tiles. The `BRAM` bel is a significantly improved version of the Virtex 2 block RAM, introducing:

- optional data output pipeline registers
- hardware ECC encoder and decoder (with error correction)
- per-byte write enable
- cascade capability, for chaining two vertically adjacent BRAMs into one 32768×1 RAM
- independent read and write width for a port
- hardware FIFO controller

The `BRAM` bel can be used in three ways:

- plain RAM mode, corresponding to `RAMB16` library primitive
- ECC RAM mode, corresponding to `RAMB32_S64_ECC` library primitive; this requires a pair of `BRAM` tiles, aligned to a multiple of 8 rows
- FIFO mode, corresponding to `FIFO16` library primitive

The bel has the following inputs connected via general interconnect: 

- `CLK[AB]` (freely invertible through the interconnect): clocks for port A and port B
- `EN[AB]` (freely invertible through the interconnect): enable for port A and port B
- `SSR[AB]` (freely invertible through the interconnect): synchronous set/reset for the read ports
- `REGCE[AB]` (freely invertible through the interconnect): pipeline register clock enable for read ports
- `WE[AB][0-3]` (freely invertible through the bel): per-byte write enable for port A and port B
- `ADDR[AB]0` through `ADDR[AB]14`, the address buses for port A and port B; `ADDR[AB]14` is used only for cascade mode, and must be tied high otherwise
- `DI[AB]0` through `DI[AB]31`: the main data input for port A and port B
- `DIP[AB]0` through `DIP[AB]3`: the parity data input for port A and port B

The bel has the following output pins connected via general interconnect:

- `DO[AB]0` through `DO[AB]31`: the main data output for port A and port B
- `DOP[AB]0` through `DOP[AB]3`: the parity data output for port A and port B

The bel also has special pins routed through dedicated interconnect:

- `CASCADEOUT[AB]`: cascade outputs, used to implement the cascade function
- `CASCADEIN[AB]`: cascade inputs, connected to the `CASCADEOUT[AB]` pins of the BRAM in the tile immediately below

The cascade is not routed across PowerPC holes — cascade functionality cannot be used between BRAMs on two sides of the PowerPC hole.

The attributes of the `BRAM` bel are:

- `MODE`: selects the operating mode of the bel

  - `RAM`: implements a RAM
  - `FIFO`: implements a FIFO

- `WE[AB][0-3]INV`: when set, the corresponding pin is inverted
- `EN_ECC_READ` (4-bit value) and `EN_ECC_WRITE` (4-bit value): must all be set when in ECC mode, exact workings unknown
- `READ_WIDTH_[AB]` selects the read width of the given port; the values are:

  - `1`: `DO[AB]0` is used for data, `ADDR[AB][0-14]` is used for address
  - `2`: `DO[AB][01]` is used for data, `ADDR[AB][1-14]` is used for address
  - `4`: `DO[AB][0-3]` is used for data, `ADDR[AB][2-14]` is used for address
  - `9`: `DO[AB][0-7]` and `DOP[AB]0` are used for data, `ADDR[AB][3-14]` is used for address
  - `18`: `DO[AB][0-15]` and `DOP[AB][01]` are used for data, `ADDR[AB][4-14]` is used for address
  - `36`: `DO[AB][0-31]` and `DOP[AB][0-3]` are used for data, `ADDR[AB][5-14]` is used for address

- `WRITE_WIDTH_[AB]` selects the write width of the given port; the values are:

  - `1`: `DI[AB]0` is used for data, `ADDR[AB][0-14]` is used for address
  - `2`: `DI[AB][01]` is used for data, `ADDR[AB][1-14]` is used for address
  - `4`: `DI[AB][0-3]` is used for data, `ADDR[AB][2-14]` is used for address
  - `9`: `DI[AB][0-7]` and `DIP[AB]0` are used for data, `ADDR[AB][3-14]` is used for address
  - `18`: `DI[AB][0-15]` and `DIP[AB][01]` are used for data, `ADDR[AB][4-14]` is used for address
  - `36`: `DI[AB][0-31]` and `DIP[AB][0-3]` are used for data, `ADDR[AB][5-14]` is used for address

- `FIFO_WIDTH`: selects the data width when in FIFO mode; must be equal to `READ_WIDTH_A` and `WRITE_WIDTH_B`

- `WRITE_MODE_[AB]`: selects behavior of the read port when writing, the values are:

  - `WRITE_FIRST`: data output shows just-written data
  - `READ_FIRST`: data output shows data previously in memory at the given address
  - `NO_CHANGE`: the data output is unchanged from previous cycle

  Both of these attributes must be set to `WRITE_FIRST` in FIFO mode.

- `SRVAL_[AB]` (36-bit value): selects the set/reset value of the read port; the low 32 bits correspond to `DO[AB][0-31]` while the high 4 bits correspond to `DOP[AB][0-3]`
- `INIT_[AB]` (36-bit value): selects the initial value of the read port; the low 32 bits correspond to `DO[AB][0-31]` while the high 4 bits correspond to `DOP[AB][0-3]`
- `DO[AB]_REG` : selects number of pipeline registers for read port

  - `0`: no pipeline registers, just output latches; same behavior as older FPGAs
  - `1`: one additional pipeline register on the read port

  Both of these attributes must be set to `WRITE_FIRST` in FIFO mode.

- `RAM_EXTENSION_[AB]`: controls the cascade mode for the given port

  - `NONE_UPPER`: the port is not in cascade mode, or it is the upper BRAM in cascade mode

    - when `ADDR[AB]14` is high, data is written to BRAM and/or read from BRAM
    - when `ADDR[AB]14` is low, data is not written to BRAM; read data is forwarded from `CASCADEIN[AB]`

    If the BRAM is not part of cascade, `ADDR[AB]14` should be tied high.

  - `LOWER`: the port is the lower BRAM in cascade mode

    - when `ADDR[AB]14` is high, data is not written to BRAM
    - when `ADDR[AB]14` is low, data is written to BRAM

- `INVERT_CLK_DO[AB]_REG`: if set, the clock controlling the pipeline register is inverted from `CLK[AB]`
- `DATA` (16384-bit value): the initial data for the BRAM main plane; corresponds to concatenation of `INIT_xx` primitive attributes
- `DATAP` (2048-bit value): the initial data for the BRAM parity plane; corresponds to concatenation of `INITP_xx` primitive attributes
- `SAVEDATA` (64-bit value): if set, the BRAM data will not be written during partial reconfiguration; the attribute has one bit per frame of the bitstream BRAM data tile
- `FIRST_WORD_FALL_THROUGH`: like the corresponding `FIFO16` attribute
- `ALMOST_FULL_OFFSET` (12-bit value): like the corresponding `FIFO16` attribute
- `ALMOST_EMPTY_OFFSET` (12-bit value): like the corresponding `FIFO16` attribute, except the bitstream value is:

  - 1 smaller then primitive attribute when not in FWFT mode
  - 2 smaller then primitive attribute when in FWFT mode

- `WW_VALUE`: unknown purpose, must be set to `NONE`

TODO: details of `SAVEDATA`


## FIFO mode

When used in FIFO mode, the pins are repurposed as follows:

- `CLKA`: becomes `RDCLK`
- `CLKB`: becomes `WRCLK`
- `ENA`: becomes `RDEN`
- `ENB`: becomes `WREN`
- `SSRA`: becomes `RST`
- `SSRB`: unused
- `REGCE[AB]`: unused
- `WE[AB]*`: unused
- `ADDR[AB]*`: unused; address generation is done internally by FIFO logic
- `DIB*` and `DIPB*`: used as `DI*` and `DIP*`, the data input
- `DIA*` and `DIPA*`: unused
- `DOA*` and `DOPA*`: used as `DO*` and `DOP*`, the data output
- `DOB[0-3]`: becomes `RDCOUNT[0-3]`
- `DOB4`: unused
- `DOB5`: becomes `RDERR`
- `DOB6`: becomes `ALMOSTEMPTY`
- `DOB7`: becomes `EMPTY`
- `DOB8`: becomes `FULL`
- `DOB9`: becomes `ALMOSTFULL`
- `DOB10`: becomes `WRERR`
- `DOB11`: unused
- `DOB[12-15]`: becomes `RDCOUNT[8-11]`
- `DOB[16-23]`: becomes `WRCOUNT[0-7]`
- `DOB[24-27]`: becomes `RDCOUNT[4-7]`
- `DOB[28-31]`: becomes `WRCOUNT[8-11]`

TODO: details of FIFO mode

## ECC mode

To use the ECC mode an implement a `RAMB32_S64_ECC` primitive, do the following:

- pick a pair of aligned adjacent BRAM tiles (the lower one having bottom row index divisible by 8)
- use the `A` port for reading, `B` port for writing
- connect `CLK[AB]`, `EN[AB]`, `ADDR[AB]` appropriately, mirroring them across both BRAMs
- tie `WEA*` low, `WEB*` high
- tie `REGCEA` high
- tie `SSR[AB]` low
- set `DOA_REG` appropriately
- set `EN_ECC_READ` and `EN_ECC_WRITE` to all-1
- set `READ_WIDTH_[AB]` and `WRITE_WIDTH_[AB]` to `36`
- connect `DI` and `DO` of the primitive as follows:

  - bits 0-13 to lower `D{IB|OA}[0-13]`
  - bit 14 to lower `D{IB|OA}P1`
  - bit 15 to lower `D{IB|OA}P3`
  - bits 16-29 to lower `D{IB|OA}[16-19]`
  - bit 30 to lower `D{IB|OA}P0`
  - bit 31 to lower `D{IB|OA}P2`
  - bits 32-45 to upper `D{IB|OA}[0-13]`
  - bit 46 to upper `D{IB|OA}P1`
  - bit 47 to upper `D{IB|OA}P3`
  - bits 48-61 to upper `D{IB|OA}[16-19]`
  - bit 62 to upper `D{IB|OA}P0`
  - bit 63 to upper `D{IB|OA}P2`

- connect `STATUS[0]` of the primitive to lower `DOA31`
- connect `STATUS[0]` of the primitive to upper `DOA0`

TODO: details of ECC mode


## Bitstream

The data for a BRAM is spread across 5 bitstream tiles:

- tiles 0-3: the 4 bitstream tiles that are shared with the `INT` interconnect tiles (starting from the bottom)
- tile 4: the dedicated BRAM data tile located in the BRAM data area; this tile is 64×320 bits; it contains solely the `DATA` and `DATAP` attributes and the `SAVEDATA` attribute


{{tile virtex4 BRAM}}
