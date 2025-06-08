# Logic block

The PLB, or programmable logic block, consists of:

- 3 shared control inputs
  - `CLK`, the clock
  - `CE`, the clock enable
  - `RST`, the reset signal
- 8 LCs (logic cells), each consisting of:
  - a four-input LUT
  - carry logic
  - a flip-flop
- a carry chain
- a LUT cascade chain (iCE40 only) or hard logic input (`iCE40T0*` only)

## LUTs and LUT inputs

Each LC contains a 4-input LUT.  The 4 inputs to the LUT are called `LC[0-7].I[0-3]`.  The output of the LUT is called `LC[0-7].LTOUT`.

The LUT inputs usually come directly from general interconnect.  However, two of the inputs are special:

- `I3` can be driven by the LC's carry input, `LC[0-7].CIN`.  This is encoded in the bitstream as a special `CI` value for the relevant general interconnect `MUX` field.
- on iCE40 only, `I2` can be driven from the special `LC[0-7].LTIN` input.  This is encoded in the bitstream as a separate `LC{i}:MUX.I2` field, selecting between general interconnect and `LTIN`.

For most PLBs, the `LTIN` input is used to implement LUT cascading: `LTIN` is connected directly to `LTOUT` of the previous LC in the PLB, or to `LC7.LTOUT` of the PLB to the south for `LC0.LTIN`.

However, on `iCE40T0*`, some `PLB` tiles are designated as "IP connect" tiles, and their `LTIN` is instead connected to the output of some hard logic block.  To route the hard logic output to the fabric, the LC should be configured as combinational, with `0xf0f0` as the LUT table, and with `LTIN` input selected.  The LUT cascading cannot be used in those tiles.

The `PLB` tiles designed as "IP connect" are:

- `iCE40T04` and `iCE40T05`: all `PLB`s within the westernmost and easternmost columns
- `iCE40T01`: the southernmost three and northernmost three `PLB`s within the westernmost and easternmost columns


## Carry chain and carry logic

In addition to the LUT, each LC also has:

- a carry input, `LC[0-7].CIN`
- a carry output, `LC[0-7].COUT`
- a carry logic primitive, implementing a fixed majority function over the LC's `CIN`, `I1`, `I2` inputs:

  ```
  LC[i].COUT = MAJ(LC[i].I1, LC[i].I2, LC[i].CIN) = (
    (LC[i].CIN & LC[i].I1) |
    (LC[i].CIN & LC[i].I2) |
    (LC[i].I1 & LC[i].I2)
  )
  ```

The carry logic primitive is only functional when the `LC{i}:CARRY_ENABLE` bit is set in the bitstream.  Without that, the carry output is indeterminate.

The `CIN` inputs of LCs 1-7 are connected directly to `COUT` outputs of LCs 0-6 of the same PLB.

The `CIN` input of LC 0 can be selected (via `LC0:MUX.CI`) from one of:

- const `0`
- const `1`
- `CHAIN`: `LC7.COUT` of the `PLB` to the south


## Flip-flops

Every LC includes a flip-flop that may be optionally enabled.  If the flip-flop is enabled, the combinational output of the LUT is not available outside the LC, except for the LUT cascade function.

The flip-flop always has an initial value of 0.  The reset signal can be synchronous or asynchronous, and the reset value can be configured via the bitstream.  The clock enable has priority over the synchronous reset.

The flip-flop implements the following logic:

```
if (!LC[i].FF_ENABLE) begin

    assign LC[i].OUT = LC[i].LTOUT;

end else if (LC[i].FF_SR_ASYNC) begin

    initial LC[i].OUT = 0;

    always @(posedge CLK, posedge RST)
        if (RST)
            LC[i].OUT <= LC[i].FF_SR_VALUE;
        else if (CE)
            LC[i].OUT <= LC[i].LTOUT;

end else begin

    initial LC[i].OUT = 0;

    always @(posedge CLK)
        if (CE) begin
            if (RST)
                LC[i].OUT <= LC[i].FF_SR_VALUE;
            else
                LC[i].OUT <= LC[i].LTOUT;
        end

end
```

{{ tile siliconblue PLB_L04 }}

{{ tile siliconblue PLB_L08 }}

{{ tile siliconblue PLB_P01 }}
