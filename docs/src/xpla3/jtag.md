# JTAG interface

## IR

The IR is 5 bits long.  The following instructions exist:

| IR      | Instruction   | Register   |
| ------- | ------------- | ---------- |
| `00000` | `EXTEST`      | `BOUNDARY` |
| `00001` | `IDCODE`      | `IDCODE`   |
| `00010` | `SAMPLE`      | `BOUNDARY` |
| `00011` | `INTEST`      | `BOUNDARY` |
| `00100` | `STRTEST`     | `BOUNDARY` |
| `00101` | `HIGHZ`       | `BYPASS`   |
| `00110` | `CLAMP`       | `BYPASS`   |
| `00111` | `ISP_WRITE`   | `MISR`     |
| `01000` | `ISP_EOTF`    | `MISR`     |
| `01001` | `ISP_ENABLE`  | `MISR`     |
| `01010` | `ISP_ERASE`   | `MISR`     |
| `01011` | `ISP_PROGRAM` | `MISR`     |
| `01100` | `ISP_VERIFY`  | `MISR`     |
| `01101` | `ISP_INIT`    | `BYPASS`   |
| `01110` | `ISP_READ`    | `MISR`     |
| `10000` | `ISP_DISABLE` | `MISR`     |
| `10001` | `TEST_MODE`   | `MISR`     |
| `11111` | `BYPASS`      | `BYPASS`   |

The IR status is:

- bit 0: const 1
- bit 1: const 0
- bits 2-4: const 0 [?]


## IDCODE

The product ID part of idcode is given in the database in the per-package information. The low 3 bits of the product ID are the package, so if package is immaterial, only the high 13 bits should be used for matching the device. The vendor ID in the IDCODE can be either Philips (`0x02b` in the low 12 bits) or Xilinx (`0x93` in the low 12 bits), depending on when the device was manufactured.


## Boundary scan register

The boundary scan register contains the following bits, in order from **MSB**:

- for every FB column, in order:
  - for every even-numbered FB in the column in order, and then for every odd-numbered FB in order:
    - for every MC, in order:
      - one unknown-purpose bit
      - if the MC has an associated IOB (see `io_mcs` field in the database):
        - the input bit for the IOB
        - the output bit for the IOB
        - the active-high output-enable bit for the IOB
- for every GCLK pin, in order:
  - the input bit for the pin

All bits of the register are `BC_1` type cells.

<div class="warning">The GCLK cells can reliably capture pin state in EXTEST mode, but only partially override internal connections in INTEST mode: connections through ZIA are overriden by the boundary register value, but connections through per-FB `FCLK` lines are not.</div>

TODO: details on the cell connection, EXTEST, INTEST semantics


## ISP instructions

TODO: write me


## Programming sequence

TODO: write me
