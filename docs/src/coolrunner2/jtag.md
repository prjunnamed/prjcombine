# JTAG interface

## IR

The IR is 8 bits long.  The following instructions exist:

| IR         | Instruction        | Register   |
| ---------- | ------------------ | ---------- |
| `00000000` | `EXTEST`           | `BOUNDARY` |
| `00000001` | `IDCODE`           | `IDCODE`   |
| `00000010` | `INTEST`           | `BOUNDARY` |
| `00000011` | `SAMPLE`           | `BOUNDARY` |
| `00010001` | `TEST_ENABLE`      | `DATAREG`  |
| `00010010` | `BULKPROG`         | `DATAREG`  |
| `00010011` | `MVERIFY`          | `DATAREG`  |
| `00010100` | `ERASE_ALL`        | `DATAREG`  |
| `00010101` | `TEST_DISABLE`     | `DATAREG`  |
| `00010110` | `STCTEST`          | `STC`      |
| `11000000` | `ISC_DISABLE`      | `DATAREG`  |
| `11100000` | `ISC_NOOP`         | `BYPASS`   |
| `11100100` | `ISC_ENABLE_OTF`   | `DATAREG`  |
| `11100110` | `ISC_SRAM_WRITE`   | `DATAREG`  |
| `11100111` | `ISC_SRAM_READ`    | `DATAREG`  |
| `11101000` | `ISC_ENABLE`       | `DATAREG`  |
| `11101001` | `ISC_ENABLE_CLAMP` | `DATAREG`  |
| `11101010` | `ISC_PROGRAM`      | `DATAREG`  |
| `11101101` | `ISC_ERASE`        | `DATAREG`  |
| `11101110` | `ISC_READ`         | `DATAREG`  |
| `11110000` | `ISC_INIT`         | `DATAREG`  |
| `11111010` | `CLAMP`            | `BYPASS`   |
| `11111100` | `HIGHZ`            | `BYPASS`   |
| `11111101` | `USERCODE`         | `USERCODE` |
| `11111111` | `BYPASS`           | `BYPASS`   |

The IR status is:

- bit 0: const 1
- bit 1: const 0
- bits 2-7: ???

TODO: completely unverified from BSDL; DR assignments are suspect


## Boundary scan register

TODO: write me


## ISP instructions

TODO: write me


## Programming sequence

TODO: write me
