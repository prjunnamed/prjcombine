# Logic block

> [!NOTE]
> This document describes Spartan 3 and Virtex 4 CLBs, since they are very similar.

The main logic resource in Spartan 3 and Virtex 4 devices is the CLB (Configurable Logic Block). It is based on the [Virtex 2 CLB](../virtex2/clb.md), but has significant changes, particularly to the LUT RAM structures.

A CLB corresponds one-to-one with the `INT.CLB` interconnect tile (on Spartan 3), or to an `INT` interconnect tile (on Virtex 4). Every CLB has four `SLICE`s. The `SLICE`s come in two kinds:

- `SLICEM`: the full-featured version of `SLICE`, with LUT RAM capability
- `SLICEL`: logic-only `SLICE`, without LUT RAM capability; it is a strict subset of `SLICEM`

The `SLICE`s within a CLB are organized as follows (this is **different** from Virtex 2):

- `SLICE0`: `SLICEM`, on the bottom left of the CLB
- `SLICE1`: `SLICEL`, to the right of `SLICE0`
- `SLICE2`: `SLICEM`, above `SLICE0`
- `SLICE3`: `SLICEL`, to the right of `SLICE2` and above `SLICE1`

Every slice has:

- two 4-input LUTs, named `F` and `G`
  - each of them has four inputs, named `F[1-4]` and `G[1-4]`
  - in `SLICEM`s, each LUT can be used as LUT RAM or shift register
- two "bypass inputs" used for various purposes
  - `BX`, associated with the `F` LUT
  - `BY`, associated with the `G` LUT
- two wide multiplexers
  - `F5`, associated with the `F` LUT, multiplexing `F` and `G`
  - `FX`, associated with the `G` LUT, multiplexing `F5` and `FX` outputs of this and other `SLICE`s
- carry logic with a carry chain, going vertically upwards through the CLB column
- two main combinational outputs
  - `X`, associated with the `F` LUT
  - `Y`, associated with the `G` LUT
- (Virtex 4 only) two secondary combinational outputs
  - `XMUX`, associated with the `F` LUT
  - `YMUX`, associated with the `G` LUT
- two "bypass" combinational outputs, used for long shift registers and carry chains
  - `XB`, associated with the `F` LUT
  - `YB`, associated with the `G` LUT
- two registers and their outputs
  - `FFX` and `XQ`, associated with the `F` LUT
  - `FFY` and `YQ`, associated with the `G` LUT
- shared control inputs:
  - `CLK`, the clock input
  - `SR`, the set/reset input (also used as LUT RAM write enable in `SLICEM`)
  - `CE`, the clock enable input

In summary, a single `SLICE` has the following pins:

- `F[1-4]` and `G[1-4]`: general interconnect inputs, used as LUT inputs and LUT RAM write address
- `BX` and `BY`: general interconnect freely-invertible inputs, used for various purposes
- `CLK`, `SR`, `CE`: general interconnect freely-invertible inputs
- `X`, `Y`, `XQ`, `YQ`, `XB`, `YB`: general interconnect outputs
- (Virtex 4 only) `XMUX`, `YMUX`: general interconnect outputs
- `COUT`: dedicated output (carry output)
- `CIN`: dedicated input (carry input), routed from `COUT` of the slice below
- `SHIFTOUT`: dedicated output (shift register output)
- `SHIFTIN`: dedicated input (shift register input), routed from `SHIFTOUT` of the previous slice in sequence
- `F5` and `FX`: dedicated outputs (wide multiplexer outputs)
- `FXINA` and `FXINB`: dedicated inputs (wide multiplexer inputs), routed from `F5` and `FX` of neighbouring slices
- `DIG`: dedicated output (`SLICEM` only)
- `ALTDIG`: dedicated input (`SLICEM` only)

Additionally, some pins and circuitry are shared between `SLICEM`s within the same CLB.

Note that on Virtex 4, the CLB tile is interconnect-limitted: only up to 16 out of the `[XY]Q`, `[XY]MUX`, and `[XY]B` outputs within a single CLB can be used at a time due to the `OMUX` bottleneck. The main `[XY]` outputs don't count towards that limit, since they can use other interconnect resources.

The `CLK`, `SR`, and `CE` inputs are invertible on the interconnect level.

The `BX` and `BY` inputs are invertible within the CLB. The `BXINV` attribute, if set, inverts the `BX` signal from the interconnect. Likewise, `BYINV` inverts the `BY` signal.


## LUTs

There are two 4-input LUTs in each slice, `F` and `G`. The `F` LUT has inputs `F[1-4]`, with `F1` being the LSB and `F4` being the MSB. The `G` LUT likewise has inputs `G[1-4]`.

The initial LUT contents are determined by the `F` and `G` attributes in the bitstream.

The LUT outputs go to:

- (Spartan 3) the `FXMUX` and `GYMUX` multiplexers
- (Virtex 4) the `F` output goes directly to the `X` output; the `G` output goes directly to the `Y` output
- (Virtex 4) the `FFX` and `FFY` registers, via `DXMUX` and `DYMUX` multiplexers
- the carry logic
- the `F5` wide multiplexer


### LUT RAM

This section is only applicable to `SLICEM`. `SLICEL`s don't have LUT RAM capability.

The `F_RAM` and `G_RAM` attributes, when set, turn `F` and `G` (respectively) into LUT RAM mode.

The signals used in RAM mode are:

- `CLK` is the write clock
- `SR` is the write enable
- `G[1-4]` are write address for both the `F` and `G` LUTs
- `DIF` and `DIG` are the data input for the `F` and `G` LUTs, respectively
- `BX`: bit 4 of the write address, when enabled
- `SLICEWE1`: bit 5 of the write address, when enabled

The `DIF_MUX` determines the value of `DIF`:

- `BX`: use the `BX` pin (used for 16×X single-port RAMs)
- `ALT`: use the `DIG` value (used for dual-port RAMs, 32×X RAMs, or 64×1 RAMs)

The `DIG_MUX` determines the value of `DIG`:

- `BY`: use the `BY` pin (used for 16×X and 32×X RAMs and `SLICE2` in 64×X RAMs)
- `ALT`: use the `ALTDIG` value (used for `SLICE0` in 64×1 RAMs)

`ALTDIG` is determined as follows:

- `SLICE0.ALTDIG` is connected to `SLICE2.DIG`
- `SLICE2.ALTDIG` is indeterminate (and should not be used)

Note that `DI[FG]_MUX` attributes are also used in the shift register mode, but with different meaning.

On Spartan 3, when `SLICEWE0USED` is set, the `BX` signal is used as bit 4 of write address. The `F` LUT is written when it is 1, the `G` LUT is written when it is 0. Otherwise, the signal is ignored, and both LUTs are written at the same time.

On Virtex 4, the attribute is replaced with `F_SLICEWE0USED` and `G_SLICEWE0USED`, which are per-LUT.

The `SLICEWE1` signal is routed as follows:

- `SLICE0.SLICEWE1 = SLICE0.BY`
- `SLICE2.SLICEWE1 = !SLICE0.BY`

On Spartan 3, if `SLICE0.SLICEWE1USED` is set, both `SLICEM`s within the CLB will use their `SLICEWE1` signal as a write enable — the LUTs are only written when `SLICEWE1` is 1. Otherwise, all `SLICEWE1` signals are ignored.

Note that `SLICE2` doesn't have a `SLICEWE1USED` bit — it is controlled by the same configuration bit as `SLICE0`.

On Virtex 4, the attribute is replaced with `F_SLICEWE1USED` and `G_SLICEWE1USED`, which are per-LUT, and appear in both slices.


#### Single-port 16×X RAM

Single-port 16×X RAM can be implemented as follows:

- pick a `SLICEM`
- pick a LUT within the slice for each 16×1 subblock
  - `G` can always be used
  - `F` can be used if `G` is also used with the same address
- connect `CLK` to write clock
- connect `SR` to write enable
- for the 16×1 slice in `F` LUT:
  - connect `F[1-4]` to the read/write address
  - connect `BX` to write data
  - set `DIF_MUX` to `BX`
  - use `F` output as read data
- for the 16×1 slice in `G` LUT:
  - connect `G[1-4]` to the read/write address
  - connect `BY` to write data
  - set `DIG_MUX` to `BY`
  - use `G` output as read data


#### Dual-port 16×X RAM

Dual-port 16×X RAM can be implemented as follows:

- pick a `SLICEM`
- connect `CLK` to write clock
- connect `SR` to write enable
- connect `G[1-4]` to the write address
- connect `F[1-4]` to the read address
- connect `BY` to write data
- set `DIF_MUX` to `ALT`
- set `DIG_MUX` to `BY`
- use `F` and `G` outputs as read data


#### Single-port 32×X RAM

Single-port 32×X RAM can be implemented as follows:

- pick a `SLICEM`
- connect `CLK` to write clock
- connect `SR` to write enable
- `F` LUT corresponds to addresses `0x0X`
- `G` LUT corresponds to addresses `0x1X`
- connect `F[1-4]` and `G[1-4]` to low 4 bits of the read/write address
- connect `BX` to bit 4 of read/write address
- set `SLICEWE0USED`
- connect `BY` to write data
- set `DIF_MUX` to `ALT`
- set `DIG_MUX` to `BY`
- use `F5` output as read data

#### Single-port 64×1 RAM

Single-port 64×1 RAM can be implemented as follows:

- use both `SLICE0` and `SLICE2`
- connect `CLK` to write clock
- connect `SR` to write enable
- `SLICE0.G` LUT corresponds to addresses `0x0X`
- `SLICE0.F` LUT corresponds to addresses `0x1X`
- connect `F[1-4]` and `G[1-4]` to low 4 bits of the read/write address
- connect both `BX` to bit 4 of read/write address
- set `SLICEWE0USED`
- connect `SLICE0.BY` to bit 5 of read/write address
- set `SLICE0.SLICEWE1USED`
- connect `SLICE2.BY` to write data
- set `DIF_MUX` to `ALT`
- set `SLICE2.DIG_MUX` to `BY`
- set `SLICE0.DIG_MUX` to `ALT`
- use `SLICE0.FX` output as read data


### Shift registers

This section is only applicable to `SLICEM`. `SLICEL`s don't have LUT RAM capability.

The `F_SHIFT` and `G_SHIFT` attributes, when set, turn `F` and `G` (respectively) into shift register mode.

The signals used in shift register mode are:

- `CLK` is the write clock
- `SR` is the write enable
- `DIF` and `DIG` are the data input for the `F` and `G` LUTs, respectively

The LUTs in shift register mode have shift-out outputs, `FMC15` and `GMC15`, which are the next bit to be shifted out. They can be connected to another LUT's data input to assemble larger shift registers.

The `DIF_MUX` determines the value of `DIF`:

- `BX`: use the `BX` pin
- `ALT`: use the `GMC15` value

The `DIG_MUX` determines the value of `DIG`:

- `BY`: use the `BY` pin
- `ALT`: use the `SHIFTIN` pin

`SHIFTIN` is routed as follows:

- `SLICE0.SHIFTIN = SLICE2.SHIFTOUT = SLICE2.FMC15` 
- `SLICE2.SHIFTIN` is indeterminate.

Note that `DI[FG]_MUX` attributes are also used in the LUT RAM mode, but with different meaning.

The external write data is written to bit 0 of the LUT. Bit 15 is shifted out.

TODO: do LUT RAM and shift register modes interfere within a `SLICE`?


## Wide multiplexers

Every `SLICE` has two wide multiplexers: `F5` and `FX`, used to combine smaller LUTs into larger LUTs. Their function is hardwired:

- `F5 = BX ? F : G`
- `FX = BY ? FXINA : FXINB`

The `F5` output goes to the `FXMUX` multiplexer, and further wide multiplexers. The `FX` output goes to the `GYMUX` multiplexer, and further wide multiplexers.

The `FXINA` and `FXINB` inputs are routed as follows:

| `SLICE`  | `FXINA`     | `FXINB`                     | effective primitive |
| -------- | ----------- | --------------------------- | ------------------- |
| `SLICE0` | `SLICE0.F5` | `SLICE2.F5`                 | `MUXF6`             |
| `SLICE1` | `SLICE1.F5` | `SLICE3.F5`                 | `MUXF6`             |
| `SLICE2` | `SLICE0.FX` | `SLICE1.FX`                 | `MUXF7`             |
| `SLICE3` | `SLICE2.FX` | `SLICE2.FX`, from CLB above | `MUXF8`             |

> [!NOTE]
> The routing is different from Virtex 2.

The `FX` output isn't connected across any interconnect holes — a `MUXF8` cannot be made of two CLBs separated by a hole.


## Carry logic

The carry logic implements the `MUXCY` and `XORCY` primitives described in Xilinx documentation. There are several bitstream attributes controlling carry logic operation.

The `CYINIT` mux determines the start of the carry chain in the slice:

- `CIN`: connected from `COUT` of the `SLICE` below
- `BX`

On Spartan 3, the `CYSELF` mux determines the "propagate" (or select) input of the lower `MUXCY`:

- `F`: propagate is connected to `F` LUT output
- `1`: propagate is connected to const-1 (ie. the `MUXCY` is effectively skipped from the chain)

On Virtex 4, the `CYSELF` mux doesn't exist, and the propagate signal is hardwired to `F` output.

The `CY0F` mux determines the "generate" input of the lower `MUXCY`:

- `0` (constant)
- `1` (constant)
- (Spartan 3) `F1`
- `F2`
- (Virtex 4) `F3`
- `BX`
- (Spartan 3) `PROD`: equal to `F1 & F2`, implementing the `MULT_AND` primitive
- (Virtex 4) `PROD`: equal to `F2 & F3`, implementing the `MULT_AND` primitive

On Spartan 3, the `CYSELG` mux determines the "propagate" (or select) input of the upper `MUXCY`:

- `G`: propagate is connected to `G` LUT output
- `1`: propagate is connected to const-1 (ie. the `MUXCY` is effectively skipped from the chain)

On Virtex 4, the `CYSELG` mux doesn't exist, and the propagate signal is hardwired to `F` output.

The `CY0G` mux determines the "generate" input of the upper `MUXCY`:

- `0` (constant)
- `1` (constant)
- (Spartan 3) `G1`
- `G2`
- (Virtex 4) `G3`
- `BY`
- (Spartan 3) `PROD`: equal to `G1 & G2`, implementing the `MULT_AND` primitive
- (Virtex 4) `PROD`: equal to `G2 & G3`, implementing the `MULT_AND` primitive

The hardwired logic implemented is:

- (Spartan 3) `FCY = CYSELF ? CY0F : CIN` (lower `MUXCY`)
- (Spartan 3) `COUT = GCY = CYSELG ? CY0G : FCY` (upper `MUXCY`)
- (Virtex 4) `FCY = F ? CY0F : CIN` (lower `MUXCY`)
- (Virtex 4) `COUT = GCY = G ? CY0G : FCY` (upper `MUXCY`)
- `FXOR = F ^ CIN` (lower `XORCY`)
- `GXOR = G ^ FCY` (upper `XORCY`)

The dedicated `CIN` input is routed from:

- `SLICE0.CIN`: from `SLICE2.COUT` of CLB below
- `SLICE1.CIN`: from `SLICE3.COUT` of CLB below
- `SLICE2.CIN`: from `SLICE0.COUT`
- `SLICE3.CIN`: from `SLICE1.COUT`

The carry chains are not connected over interconnect holes. The `SLICE[01].CIN` inputs in the row above bottom IOI or any kind of interconnect hole are indeterminate.

The sum-of-products feature of Virtex 2 no longer exists on Spartan 3 and Virtex 4.


## Output multiplexers — Spartan 3

The Spartan 3 output multiplexers are unchanged from Virtex 2, except for `SOPOUT` removal.

The `FXMUX` multiplexer controls the `X` output. It has three inputs:

- `F` (the LUT output)
- `F5`
- `FXOR`

The `GYMUX` multiplexer controls the `Y` output. It has three inputs:

- `G` (the LUT output)
- `FX`
- `GXOR`

The `XBMUX` multiplexer controls the `XB` output. It has two inputs:

- `FCY`
- `FMC15`: shift register output of `F`

The `YBMUX` multiplexer controls the `YB` output. It has two inputs:

- `GCY` (equal to `COUT`)
- `GMC15`: shift register output of `G`


The `DXMUX` mulitplexer controls the `FFX` data input. It has two inputs:

- `X` (the `FXMUX` output)
- `BX`

The `DYMUX` mulitplexer controls the `FFY` data input. It has two inputs:

- `Y` (the `GYMUX` output)
- `BY`


## Output multiplexers — Virtex 4

The `FXMUX` multiplexer controls the `XMUX` output. It has two inputs:

- `F5`
- `FXOR`

The `GYMUX` multiplexer controls the `YMUX` output. It has two inputs:

- `FX`
- `GXOR`

The `X` output is directly connected to `F` output and doesn't have a mux. Likewise, `Y` output is directly connected to `G` output.

The `XBMUX` multiplexer controls the `XB` output. It has two inputs:

- `FCY`
- `FMC15`: shift register output of `F`

The `YBMUX` multiplexer controls the `YB` output. It has two inputs:

- `GCY` (equal to `COUT`)
- `GMC15`: shift register output of `G`

The `DXMUX` mulitplexer controls the `FFX` data input. It has five inputs:

- `X` (the `F` output)
- `F5`
- `FXOR`
- `XB`
- `BX`

The `DYMUX` mulitplexer controls the `FFY` data input. It has five inputs:

- `Y` (the `G` output)
- `FX`
- `GXOR`
- `YB`
- `BY`


## Registers

The registers are unchanged from Virtex 2.

A `SLICE` contains two registers:

- `FFX`, with input determined by `DXMUX` and output connected to `XQ`
- `FFY`, with input determined by `DYMUX` and output connected to `YQ`

Both registers share the same control signals:

- `CLK`: posedge-triggered clock in FF mode or **active-low** gate in latch mode
- `CE`: active-high clock or gate enable
- `SR`: if `FF_SR_EN`, the set/reset signal
- `BY`: if `FF_REV_EN`, the alternate set/reset signal

The following attributes determine register function:

- `FF_LATCH`: if set, the registers are latches and `CLK` behaves as **active-low** gate; otherwise, the registers are flip-flops and `CLK` is a posedge-triggered clock
- `FF_SYNC`: if set, the `SR` and `BY` (if enabled) implement synchronous set/reset (with priority over `CE`); otherwise, they implement asynchronous set/reset; should not be set together with `FF_LATCH`
- `FF[XY]_INIT`: determines the initial or captured value of given register
  - when the global `GSR` signal is pulsed (for example, as part of the configuration process), the register is set to the value of this bit
  - when the global `GCAP` signal is pulsed (for example, by the `CAPTURE` primitive), this bit captures the current state of the register
- `FF[XY]_SRVAL`: determines the set/reset value of given register
- `FF_SR_EN`: if set, `SR` is used as the set/reset signal for both registers, setting them to their `FF[XY]_SRVAL`
- `FF_REV_EN`: if set, `BY` behaves as secondary set/reset signal for both registers, setting them to the **opposite** of their `FF[XY]_SRVAL`


## Bitstream

The data for a CLB is located in the same bitstream tile as the associated `INT_CLB` tile.

{{tile spartan3 CLB}}


## `RESERVED_ANDOR`

TODO: wtf is this even


## `RANDOR`

This tile overlaps `IOI_*`.

{{tile spartan3 RANDOR}}
{{tile spartan3 RANDOR_FC}}


## `RANDOR_INIT`

This tile overlaps top-left interconnect tile.

{{tile spartan3 RANDOR_INIT}}
{{tile spartan3 RANDOR_INIT_FC}}
