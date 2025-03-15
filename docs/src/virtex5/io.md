# Input/Output


## I/O banks and special functions

Virtex 5 devices have up to three I/O columns:

- the left I/O column, containing only IO tiles; if the device has no transceivers on the left side, it is the leftmost column of the device; otherwise, it is somewhat to the right of the left GT column; it is always present
- the center column, part of which contains IO tiles; the IO tiles in this column come in up to four segments:
  - the lower segment (variable size, not present on all devices), between the bottom edge of the device and the lower CMTs
  - the lower middle segment (always 20 tiles high), between lower CMTs and the configuration center
  - the upper middle segment (always 20 tiles high), between the configuration center and the upper CMTs
  - the upper segment (variable size, not present on all devices), between the device and the upper CMTs and the top edge of the device
- the right I/O column, containing only IO tiles; it is present on all devices except for `xc5vlx20t`; if present, it is somewhat to the left of the device's right edge

Virtex 5 has the following banks:

- bank 0 is the configuration bank; it contains only dedicated configuration I/O pins, as follows:

  - `CCLK`
  - `CS_B`
  - `DONE`
  - `D_OUT_BUSY`
  - `D_IN`
  - `HSWAPEN`
  - `INIT`
  - `M0`
  - `M1`
  - `M2`
  - `PROGRAM_B`
  - `RDWR_B`
  - `TCK`
  - `TDI`
  - `TDO`
  - `TMS`

  bank 0 is not associated with any IO tiles

- banks 1-4: middle segments of the center column; each of them consists of 10 IO tiles; they contain global clock inputs and shared configuration pins

  - bank 1: immediately above configuration center
  - bank 2: immediately below configuration center
  - bank 3: above bank 1, below upper CMTs (not present on `xc5vlx20t`)
  - bank 4: below bank 2, above lower CMTs

- banks 5-10: lower and upper segments of the center column; each of them consists of 20 IO tiles

  - banks 5, 7, 9 are the upper segment, with bank 5 being immediately above upper CMTs; bank number increases upwards
  - banks 6, 8, 10 are the lower segment, with bank 6 being immediately below lower CMTs; bank number increases downwards

- banks 11 and up: left and right column; each of them consists of 20 IO tiles

  - banks 11, 15, 19, 23, ...: left column, above the configuration center; bank number increases upwards, starting from bank 11 immediately above the configuration center row
  - banks 12, 16, 20, 24, ...: right column, above the configuration center; bank number increases upwards, starting from bank 12 immediately above the configuration center row
  - banks 13, 17, 21, 25, ...: left column, below the configuration center; bank number increases downwards, starting from bank 13 immediately below the configuration center row
  - banks 14, 18, 22, 26, ...: right column, below the configuration center; bank number increases downwards, starting from bank 14 immediately below the configuration center row

All IOBs in the device are grouped into differential pairs, one pair per IO tile.  `IOB1` is the "true" pin of the pair, while `IOB0` is the "complemented" pin.  Differential input and true differential output is supported on all pins of the device.

`IOB1` pads in the 4 rows surronding the HCLK row (that is, in rows 8-11 of every clock region) are considered "clock-capable". They can drive `BUFIO` and `BUFR` buffers via dedicated connections. While Xilinx documentation also considers `IOB0` pads clock-capable, this only means that they can be used together with `IOB1` as a differential pair.

The `IOB1` pads in banks 3 and 4 are considered "global clock-capable". They can drive `BUFGCTRL` buffers and CMT primitives via dedicated connections.  Likewise, Xilinx considers `IOB0` pads to be clock-capable, but they can only drive clocks as part of differential pair with `IOB1`.

The `IOB0` in rows 5 and 15 of every region is capable of being used as a VREF pad.

Each bank except for banks 1 and 2 has two IOBs that can be used for reference resistors in DCI operation. They are both located in the same I/O tile, with VRP located on `IOB0` and VRN located on `IOB1`. The relevant tile is located as follows:

- bank 1 and 2: VRP/VRN are not present in this bank (DCI can still be used by cascade from banks 3 and 4)
- bank 3: row 7 of the bank (or row 7 of the region)
- bank 4: row 2 of the bank (or row 12 of the region)
- banks 5 and up: row 7 of the bank (or row 7 of the region)

In parallel configuration modes, some I/O pads in banks 1-4 are borrowed for configuration use, as the parallel data pins:

- bank 4 row 6 `IOB0`: `D[8]`
- bank 4 row 6 `IOB1`: `D[9]`
- bank 4 row 7 `IOB0`: `D[10]`
- bank 4 row 7 `IOB1`: `D[11]`
- bank 4 row 8 `IOB0`: `D[12]`
- bank 4 row 8 `IOB1`: `D[13]`
- bank 4 row 9 `IOB0`: `D[14]`
- bank 4 row 9 `IOB1`: `D[15]`
- bank 2 row 0 `IOB0`: `D[0]/FS[0]`
- bank 2 row 0 `IOB1`: `D[1]/FS[1]`
- bank 2 row 1 `IOB0`: `D[2]/FS[2]`
- bank 2 row 1 `IOB1`: `D[3]`
- bank 2 row 2 `IOB0`: `D[4]`
- bank 2 row 2 `IOB1`: `D[5]`
- bank 2 row 3 `IOB0`: `D[6]`
- bank 2 row 3 `IOB1`: `D[7]`
- bank 2 row 4 `IOB0`: `CSO_B`
- bank 2 row 4 `IOB1`: `FWE_B`
- bank 2 row 5 `IOB0`: `FOE_B/MOSI`
- bank 2 row 5 `IOB1`: `FCS_B`
- bank 2 row 6 `IOB0`: `A[20]`
- bank 2 row 6 `IOB1`: `A[21]`
- bank 2 row 7 `IOB0`: `A[22]`
- bank 2 row 7 `IOB1`: `A[23]`
- bank 2 row 8 `IOB0`: `A[24]`
- bank 2 row 8 `IOB1`: `A[25]`
- bank 2 row 9 `IOB0`: `RS[0]`
- bank 2 row 9 `IOB1`: `RS[1]`
- bank 1 row 0 `IOB0`: `D[16]/A[0]`
- bank 1 row 0 `IOB1`: `D[17]/A[1]`
- bank 1 row 1 `IOB0`: `D[18]/A[2]`
- bank 1 row 1 `IOB1`: `D[19]/A[3]`
- bank 1 row 2 `IOB0`: `D[20]/A[4]`
- bank 1 row 2 `IOB1`: `D[21]/A[5]`
- bank 1 row 3 `IOB0`: `D[22]/A[6]`
- bank 1 row 3 `IOB1`: `D[23]/A[7]`
- bank 1 row 4 `IOB0`: `D[24]/A[8]`
- bank 1 row 4 `IOB1`: `D[25]/A[9]`
- bank 1 row 5 `IOB0`: `D[26]/A[10]`
- bank 1 row 5 `IOB1`: `D[27]/A[11]`
- bank 1 row 6 `IOB0`: `D[28]/A[12]`
- bank 1 row 6 `IOB1`: `D[29]/A[13]`
- bank 1 row 7 `IOB0`: `D[30]/A[14]`
- bank 1 row 7 `IOB1`: `D[31]/A[15]`
- bank 1 row 8 `IOB0`: `A[16]`
- bank 1 row 8 `IOB1`: `A[17]`
- bank 1 row 9 `IOB0`: `A[18]`
- bank 1 row 9 `IOB1`: `A[19]`

The `SYSMON` present on the device can use up to 16 IOB pairs from the left I/O column as auxiliary analog differential inputs. The `VPx` input corresponds to `IOB1` and `VNx` corresponds to `IOB0` within the same tile. The IOBs are in the following tiles, where `r` is the configuration center row:

- `VP0/VN0`: left I/O column, row `r - 10`
- `VP1/VN1`: left I/O column, row `r - 9`
- `VP2/VN2`: left I/O column, row `r - 8`
- `VP3/VN3`: left I/O column, row `r - 7`
- `VP4/VN4`: left I/O column, row `r - 6`
- `VP5/VN5`: left I/O column, row `r - 4`
- `VP6/VN6`: left I/O column, row `r - 3`
- `VP7/VN7`: left I/O column, row `r - 2`
- `VP8/VN8`: left I/O column, row `r - 1`
- `VP9/VN9`: left I/O column, row `r`
- `VP10/VN10`: left I/O column, row `r + 1`
- `VP11/VN11`: left I/O column, row `r + 2`
- `VP12/VN12`: left I/O column, row `r + 3`
- `VP13/VN13`: left I/O column, row `r + 4`
- `VP14/VN14`: left I/O column, row `r + 8`
- `VP15/VN15`: left I/O column, row `r + 9`


## Bitstream

{{tile virtex5 IO}}


## Tables

{{devdata virtex5 iodelay-default}}

{{misc virtex5 iostd-drive}}

{{misc virtex5 iostd-slew}}

{{misc virtex5 iostd-misc}}

{{misc virtex5 iostd-lvds}}

{{misc virtex5 iostd-lvdsbias}}

{{misc virtex5 iostd-dci-lvdiv2}}

{{misc virtex5 iostd-dci-mask-term-vcc}}

{{misc virtex5 iostd-dci-mask-term-split}}
