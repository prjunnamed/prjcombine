# Project Combine

Project Combine aims to provide comprehensive documentation of CPLD and FPGA internals, particularly in the areas where vendor documentation is lacking, such as routing structure, bitstream format, and internal timings.  This includes both prose (such as this book) and machine-readable databases that would be useful for toolchain development.

Project Combine covers multiple *targets* (ie. supported device families).  The targets vary widely in their completeness levels.  While Project Combine aims to share data structures and code paths between targets as much as possible, this is not always practical due to the sheer variety of supported devices.

Project Combine consists of:

1. The database files (`databases/` directory), one or more per target.  Some targets have a single database file covering all devices, while others have per-device databases, or something in between.

   Each individual database file is provided in two forms:

   - zstd-compressed bincode-serialized Rust structures, suitable for use with the provided Rust crates
   - text format dump

   The database format is target-dependent, though there are broad similarities.

   One of the project goals is providing highly compact databases, via aggressive deduplication.  For instance, the databases do not contain the full list of primitives and nets for a given FPGA — instead, they contain just enough information to allow recreating the full geometry via target-specific means.  You can use the provided Rust code to expand and navigate this information.

2. The mdBook documentation (`docs/` directory), which you are reading right now.  Large parts of the documentation are generated from the database files (via an mdBook plugin implemented in `docgen/`).  The goal is to include all data within the database as part of the documentation, but this is far from done.

3. The public interface Rust crates (`public/` directory).  They provide:

   - data structures for deserializing the binary database
   - functions for recreating the full tile grid of a given device from the target-specific description
   - all sort of target-specific queries about a device (eg. list all IO pads in BSCAN order)
   - parsing and emitting bitstreams

   The public crates include:

   - `prjcombine-types`: common data types used across multiple targets
     - `bsdata`: bitstream format description (for the databases)
     - `speed`: raw speed data description (for the databases)
     - `units`: newtypes over `f64` associated with physical units (used for speed data)
     - `bscan`: boundary scan chain description
   - `prjcombine-interconnect`: implements data structures for FPGA-like tile grids and general interconnect; this is the crate that FPGA targets are generally based around
   - `prjcombine-jed`: implements JESD3 bitstream format, also known as the `.jed` file format
   - `prjcombine-xilinx-bitstream`: the Xilinx bitstream format (common across Xilinx FPGA targets)
   - per-target crates (listed below)

4. The reverse engineering tools (`re/` directory).

   These crates contain code that has been used to create the database files.  Because of their characteristics (essentially, this is the "only has to work once" kind of code), little to no documentation is provided for them, or their usage.  More often than not, they require very specific environment to work in that is hard to recreate (such as heavily patched versions of vendor toolchains).


## Supported targets

The currently supported top-level FPGA targets and their corresponding crates are:

- `prjcombine-siliconblue`: SiliconBlue / Lattice iCE40 and iCE65 FPGAs; has one database:

  - `siliconblue`: iCE65, iCE40

  Status:

  - geometry and bitstream data: ✅ complete
  - speed data: 👷🏼‍♀️ mostly complete (missing DSP timings)
  - documentation: 👷🏼‍♀️ in progress
  - verification: ❌ not started

- `prjcombine-xc2000`: Earliest Xilinx devices; has several databases, corresponding to the subtargets:

  - `xc2000`: XC2000, XC2000L
  - `xc3000`: XC3000, XC3100
  - `xc3000a`: XC3000A, XC3000L, XC3100A, XC3100L
  - `xc4000`: XC4000, XC4000D
  - `xc4000a`: XC4000A
  - `xc4000h`: XC4000H
  - `xc4000e`: XC4000E, XC4000L, Spartan
  - `xc4000ex`: XC4000EX, XC4000XL
  - `xc4000xla`: XC4000XLA
  - `xc4000xv`: XC4000XV
  - `spartanxl`: Spartan XL
  - `xc5200`: XC5200, XC5200L

  Status:

  - geometry and bitstream data: ✅ complete
  - speed data, documentation, verification: ❌ not started

- `prjcombine-virtex`: Xilinx Virtex and related devices; has one database:

  - `virtex`: Virtex, Virtex-E, Spartan-II, Spartan-IIE

  Status:

  - geometry and bitstream data: ✅ complete
  - speed data, documentation, verification: ❌ not started

- `prjcombine-virtex2`: Xilinx Virtex-II and Spartan-3; has two databases:

  - `virtex2`:
    - Virtex-II
    - Virtex-II Pro
  - `spartan3`:
    - Spartan-3
    - FPGAcore eFPGA
    - Spartan-3E
    - Spartan-3A and Spartan-3AN
    - Spartan-3A DSP

  Status:

  - geometry and bitstream data: ✅ complete
  - documentation: 👷🏼‍♀️ in progress
  - speed data, verification: ❌ not started

- `prjcombine-spartan6`: Xilinx Spartan-6; has one database:

  - `spartan6`: Spartan-6 LX, LXT

  Status:

  - geometry and bitstream data: ✅ complete
  - speed data, documentation, verification: ❌ not started

- `prjcombine-virtex4`: Xilinx Virtex-4, -5, -6, -7 devices; has four databases:

  - `virtex4`: Virtex-4 LX, SX, FX
  - `virtex5`: Virtex-5 LX, LXT, SXT, FXT, TXT
  - `virtex6`: Virtex-6 LX, LXT, SXT, HXT, CXT
  - `virtex7`:
    - Spartan-7
    - Artix-7
    - Kintex-7
    - Virtex-7
    - Zynq-7000

  Status:

  - geometry and bitstream data: ✅ complete
  - documentation: 👷🏼‍♀️ in progress
  - speed data, verification: ❌ not started

- `prjcombine-ultrascale`: Xilinx/AMD UltraScale devices; has two databases:

  - `ultrascale`:
    - Kintex UltraScale
    - Virtex UltraScale
  - `ultrascaleplus`:
    - Spartan UltraScale+
    - Artix UltraScale+
    - Kintex UltraScale+
    - Virtex UltraScale+
    - Zynq UltraScale+ MPSoC
    - Zynq UltraScale+ RFSoC

  Status:

  - geometry data: ✅ complete
  - bitstream and speed data, documentation, verification: ❌ not started

- `prjcombine-versal`: Xilinx/AMD Versal devices; will have one database:

  - `versal`:
    - Versal Prime
    - Versal Prime Gen 2
    - Versal AI Core
    - Versal AI Edge
    - Versal AI Edge Gen 2
    - Versal RF
    - Versal Network
    - Versal Premium
    - Versal Premium Gen 2
    - Versal HBM

  Status:

  - geometry data: 👷🏼‍♀️ in progress
  - bitstream and speed data, documentation, verification: ❌ not started

The currently supported top-level CPLD targets and their corresponding crates are:

- `prjcombine-xc9500`: Xilinx XC9500 and variants; has three databases:

  - `xc9500`: XC9500
  - `xc9500xl`: XC9500XL
  - `xc9500xv`: XC9500XV

  Status: ✅ complete.

- `prjcombine-xpla3`: Philips/Xilinx CoolRunner XPLA3 devices; has one database:

  - `xpla3`: CoolRunner XPLA3

  Status:

  - geometry, bitstream, speed data, and verification: ✅ complete
  - documentation: 👷🏼‍♀️ mostly complete (missing JTAG commands)

- `prjcombine-coolrunner2`: Xilinx CoolRunner-II devices; has one database:

  - `coolrunner2`: CoolRunner-II

  Status:

  - geometry, bitstream, speed data, and verification: ✅ complete
  - documentation: 👷🏼‍♀️ in progress