# Input/Output


## I/O banks and special functions

Virtex 4 devices have exactly three I/O columns:

- the left I/O column, containing only IO tiles; if the device has no transceivers, it is the leftmost column of the device; otherwise, it is somewhat to the right of the left GT column
- the center column, part of which contains IO tiles; the IO tiles in this column come in two segments:
  - the lower segment, between lower DCMs/CCMs and the configuration center
  - the upper segment, between the configuration center and the upper DCMs/CCMs
- the right I/O column, containing only IO tiles; if the device has no transceivers, it is the rightmost column of the device; otherwise, it is somewhat to the left of the right GT column

Virtex 4 has the following banks:

- bank 0 is the configuration bank; it contains only dedicated configuration I/O pins, as follows:

  - `CCLK`
  - `CS_B`
  - `DONE`
  - `DOUT_BUSY`
  - `D_IN`
  - `HSWAP_EN`
  - `INIT`
  - `M0`
  - `M1`
  - `M2`
  - `PROGRAM_B`
  - `PWRDWN_B`
  - `RDWR_B`
  - `TCK`
  - `TDI`
  - `TDO`
  - `TMS`

  bank 0 is not associated with any IO tiles

- banks 1-4 are central column banks, with no support for true differential output; they are:
  - bank 1: right above configuration center; has 8, 24, or 40 I/O tiles
  - bank 2: right below configuration center; has 8, 24, or 40 I/O tiles
  - bank 3: above bank 1, below top DCMs/CCMs; always has 8 I/O tiles
  - bank 4: below bank 2, above bottom DCMs/CCMs; always has 8 I/O tiles

- banks 5-16: left and right column banks; the number of present banks in these column varies between devices, but each bank has a constant size of 32 I/O tiles (ie. is two regions high); the HCLK tile in bottom region of the bank contains DCI control circuitry, while the HCLK tile in top region of the bank contains contains LVDS output circuitry

  - odd-numbered banks belong to the left column; the banks, in order from the bottom, will be numbered as follows depending on height of the device:
    - 4 regions: 7, 5
    - 6 regions: 7, 9, 5
    - 8 regions: 7, 11, 9, 5
    - 10 regions: 7, 11, 13, 9, 5
    - 12 regions: 7, 11, 15, 13, 9, 5
  - even-numbered banks belong to the right column; the banks, in order from the bottom, will be numbered as follows depending on height of the device:
    - 4 regions: 8, 6
    - 6 regions: 8, 10, 6
    - 8 regions: 8, 12, 10, 6
    - 10 regions: 8, 12, 14, 10, 6
    - 12 regions: 8, 12, 16, 14, 10, 6

All IOBs in the device are grouped into differential pairs, one pair per IO tile.  `IOB1` is the "true" pin of the pair, while `IOB0` is the "complemented" pin.  Differential input is supported on all pins of the device.  True differential output is supported only in the left and right columns, in all tiles except for rows 7 and 8 of every region (ie. except the "clock-capable" pads).

`IOB1` pads next to the HCLK row (that is, in row 7 and 8 of every clock region) are considered "clock-capable". They can drive `BUFIO` and `BUFR` buffers via dedicated connections. While Xilinx documentation also considers `IOB0` pads clock-capable, this only means that they can be used together with `IOB1` as a differential pair.

The 16 bottommost `IOB1` pads and 16 topmost `IOB1` pads in the central column are considered "global clock-capable". They can drive `BUFGCTRL` buffers and `DCM` primitives via dedicated connections.  Likewise, Xilinx considers `IOB0` pads to be clock-capable, but they can only drive clocks as part of differential pair with `IOB1`.

The `IOB0` in rows 4 and 12 of every region is capable of being used as a VREF pad.

Each bank, with some exceptions on the smaller devices, has two IOBs that can be used for reference resistors in DCI operation. They are both located in the same I/O tile, with VRP located on `IOB0` and VRN located on `IOB1`. The relevant tile is located as follows:

- bank 1, if the bank has 8 I/O tiles: DCI is not supported in this bank
- bank 1, if the bank has 24 I/O tiles: row 14 of the bank (row 6 of the topmost region of the bank)
- bank 1, if the bank has 40 I/O tiles: row 30 of the bank (row 6 of the topmost region of the bank)
- bank 2, if the bank has 8 I/O tiles: DCI is not supported in this bank
- bank 2, if the bank has 24 I/O tiles: row 9 of the bank (row 9 of the bottom region of the bank)
- bank 2, if the bank has 40 I/O tiles: row 9 of the bank (row 9 of the bottom region of the bank)
- bank 3: row 6 of the bank (row 6 of the region)
- bank 4: row 1 of the bank (row 9 of the region)
- banks 5-16: row 9 of the bank (row 9 of the bottom region of the bank)

In parallel configuration modes, some I/O pads in banks 1 and 2 are borrowed for configuration use, as the parallel data pins:

- `D[i]`, `i % 2 == 0`, `0 <= i < 16`: `IOB0` of row `i / 2` of topmost region of bank 2
- `D[i]`, `i % 2 == 1`, `0 <= i < 16`: `IOB1` of row `(i - 1) / 2` of topmost region of bank 2
- `D[i]`, `i % 2 == 0`, `16 <= i < 32`: `IOB0` of row `i / 2` of bottom region of bank 1 (or, row `(i - 16) / 2` of the bank)
- `D[i]`, `i % 2 == 1`, `16 <= i < 32`: `IOB1` of row `(i - 1) / 2` of bottom region of bank 1 (or, row `(i - 17) / 2` of the bank)

Every `SYSMON` present on the device can use up to seven IOB pairs from the left I/O column as auxiliary analog differential inputs. The `VPx` input corresponds to `IOB1` and `VNx` corresponds to `IOB0` within the same tile. The IOBs are in the following tiles, where `r` is the bottom row of the `SYSMON`:

- `VP1/VN1`: left I/O column, row `r`
- `VP2/VN2`: left I/O column, row `r + 1`
- `VP3/VN3`: left I/O column, row `r + 2`
- `VP4/VN4`: left I/O column, row `r + 3`
- `VP5/VN5`: left I/O column, row `r + 5`
- `VP6/VN6`: left I/O column, row `r + 6`
- `VP7/VN7`: left I/O column, row `r + 7`

Row `r + 4` is not used as `SYSMON` input — the "analog function" of that pin is considered to be VREF instead (they are controlled by the same bit).


{{tile virtex4 IO}}


## Tables

{{misc virtex4 iostd-drive}}
{{misc virtex4 iostd-slew}}
{{misc virtex4 iostd-misc}}
{{misc virtex4 iostd-lvds}}
{{misc virtex4 iostd-lvdsbias}}
{{misc virtex4 iostd-dci-lvdiv2}}
{{misc virtex4 iostd-dci-mask-term-vcc}}
{{misc virtex4 iostd-dci-mask-term-split}}
