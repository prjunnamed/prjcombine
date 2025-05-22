# Project Combine

Project Combine aims to provide comprehensive documentation of CPLD and FPGA internals, particularly in the areas where vendor documentation is lacking, such as routing structure, bitstream format, and internal timings.  This includes both prose (such as this book) and machine-readable databases that would be useful for toolchain development.

Project Combine covers multiple *targets* (ie. supported device families).  The targets vary widely in their completeness levels.  While Project Combine aims to share data structures and code paths between targets as much as possible, this is not always practical due to the sheer variety of supported devices.

Project Combine consists of:

1. The database files (`databases/` directory), one or more per target.  Some targets have a single database file covering all devices, while others have per-device databases, or something in between.

   Each individual database file is provided in two forms:

   - zstd-compressed bincode-serialized Rust structures, suitable for use with the provided Rust crates
   - JSON

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
     - `tiledb`: bitstream format description (for the databases)
     - `speed`: raw speed data description (for the databases)
     - `units`: newtypes over `f64` associated with physical units (used for speed data)
     - `bscan`: boundary scan chain description
   - `prjcombine-interconnect`: implements data structures for FPGA-like tile grids and general interconnect; this is the crate that FPGA targets are generally based around
   - `prjcombine-jed`: implements JESD3 bitstream format, also known as the `.jed` file format
   - `prjcombine-xilinx-bitstream`: the Xilinx bitstream format (common across Xilinx FPGA targets)
   - per-target crates (FPGA):
     - `prjcombine-siliconblue`: SiliconBlue / Lattice iCE40 and iCE65
     - `prjcombine-xc2000`: Xilinx XC2000, XC3000, XC4000 (including all its variants, such as Spartan and Spartan XL), XC5200
     - `prjcombine-virtex`: Xilinx Virtex, Virtex-E, Spartan-II, Spartan-IIE
     - `prjcombine-virtex2`: Xilinx Virtex II, Virtex II Pro, Spartan 3, Spartan 3E, Spartan 3A, Spartan 3A DSP
     - `prjcombine-virtex4`: Xilinx Virtex 4, Virtex 5, Virtex 6, Virtex 7 (including its variants: Spartan 7, Artix 7, Kintex 7, Zynq 7000)
     - `prjcombine-spartan6`: Xilinx Spartan 6
     - `prjcombine-ultrascale`: Xilinx UltraScale and UltraScale+
     - `prjcombine-versal`: Xilinx / AMD Versal
   - per-target crates (CPLD):
     - `prjcombine-xc9500`: Xilinx XC9500, XC9500XL, XC9500XV
     - `prjcombine-xpla3`: Phillips / Xilinx Coolrunner XPLA3
     - `prjcombine-coolrunner2`: Xilinx Coolrunner 2

4. The reverse engineering tools (`re/` directory).

   These crates contain code that has been used to create the database files.  Because of their characteristics (essentially, this is the "only has to work once" kind of code), little to no documentation is provided for them, or their usage.  More often than not, they require very specific environment to work in that is hard to recreate (such as heavily patched versions of vendor toolchains).
