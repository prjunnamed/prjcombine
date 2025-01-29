# Project Combine

An FPGA reverse engineering project.

The goals of this project are:

- providing a deduplicated geometry database for target devices
  - types and locations of all logical primitives within the device
  - geometry of all interconnect wires
  - interconnection points between the wires
  - package pinouts
- providing a timing database for target devices
  - combinational and sequential timings of logical primitives
  - interconnect parameters and timing formulas
- providing a bitstream format database for target devices
  - the location and semantics of all configuration bits
- making all of the above available both for use by external programs and as human-readable documentation
- providing Rust libraries for loading and operating on the above databases
- documenting the behavior of target devices and their logical primitives

The target devices currently include:

- all SiliconBlue FPGAs
- all Xilinx FPGAs
- all Xilinx CPLDs

More targets are expected to be added in the future.

## Generated documentation

See https://prjunnamed.github.io/prjcombine/

## IRC and Matrix channel

- [#prjcombine on libera.chat](https://web.libera.chat/?channel=#prjcombine)
- [#prjcombine:catircservices.org on matrix](https://matrix.to/#/#prjcombine:catircservices.org) (bridged with IRC)
- [channel logs](https://libera.irclog.whitequark.org/prjcombine/)

## Roadmap

For each of the supported device families:

- phase 1: geometry database extraction
- phase 2: bitstream reverse engineering
- phase 3: timing database extraction
- phase 4: in-hardware test, documentation writing, final database export

## Status

| Target                                              | Geometry | Bitstream | Timing | Test | Final documentation |
| --------------------------------------------------- | -------- | --------- | ------ | ---- | ------------------- |
| iCE65, iCE40                                        | 👷🏼‍♀️        | 👷🏼‍♀️         | ❌      | ❌    | ❌                   |
| XC2000, XC3000, XC4000, Spartan, Spartan XL, XC5200 | ✅        | ✅         | ❌      | ❌    | ❌                   |
| Virtex, Virtex E, Spartan 2, Spartan 2E             | ✅        | ✅         | ❌      | ❌    | ❌                   |
| Virtex 2, Virtex 2 Pro                              | ✅        | ✅         | ❌      | ❌    | ❌                   |
| Spartan 3 (all variants)                            | ✅        | ✅         | ❌      | ❌    | ❌                   |
| Spartan 6                                           | ✅        | ✅         | ❌      | ❌    | ❌                   |
| Virtex 4, 5, 6                                      | ✅        | ✅         | ❌      | ❌    | ❌                   |
| Virtex 7, Kintex 7, Artix 7, Spartan 7, Zynq 7000   | ✅        | ✅         | ❌      | ❌    | ❌                   |
| Ultrascale, Ultrascale+                             | ✅        | ❌         | ❌      | ❌    | ❌                   |
| Versal                                              | 👷🏼‍♀️        | ❌         | ❌      | ❌    | ❌                   |
| XC9500, XC9500XL, XC9500XV                          | ✅        | ✅         | ✅      | ✅    | ✅                   |
| Coolrunner XPLA3                                    | ✅        | ✅         | ✅      | ✅    | ✅                   |
| Coolrunner 2                                        | ✅        | ✅         | ✅      | ✅    | 👷🏼‍♀️                   |

## License

Project Combine is distributed under the terms of the [0-clause BSD license](LICENSE-0BSD.txt) and the [Apache License (Version 2.0)](LICENSE-Apache-2.0.txt).

By submitting your contribution you agree to be bound by all provisions of both of these licenses, including the clause 3 (Grant of Patent License) of the Apache License (Version 2.0).

