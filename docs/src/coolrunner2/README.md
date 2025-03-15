# Coolrunner II

Coolrunner II is a family of flash-based 1.8V CPLDs manufactured by Xilinx.
It is a derivative of both the XC9500 family and the XPLA3 family.


## Devices

The following devices exist:

{{devlist coolrunner2}}

The parts starting with XA are automotive versions.  They are functionally completely identical to corresponding XC versions.

The parts with names ending with A are revisions of the original design, and are
fully backwards compatible with them (including bitstream backwards compatibility).
Thus, eg. XC2C32A can always be used in place of an XC2C32.


## Packages

The devices come in the following packages:

- PLCC:
  - PC44 (JEDEC MO-047)
- QFN:
  - QF32
  - QF48
- QFP:
  - very thin QFP (1mm thickness):
    - VQ44 (0.8mm pitch; JEDEC MS-026-ACB)
    - VQ100 (0.5mm pitch; JEDEC MS-026-AED)
  - thin QFP (1.4mm thickness):
    - TQ144 (0.5mm pitch; JEDEC MS-026-BFB)
  - plastic QFP:
    - PQ208 (3.4mm thickness, 0.5mm pitch; JEDEC MS-029-FA-1)
- BGA:
  - fine-pitch BGA (1mm pitch):
    - FG324
  - thin fine-pitch BGA (1mm pitch):
    - FT256
  - chip-scale BGA (0.5mm pitch):
    - CP56
    - CP132
  - ???
    - CV64
    - CV100
- bare die (DI*)

{{devpkg coolrunner2}}

Pin compatibility is maintained across devices within a single package.


## Device pins

The Coolrunner-II devices have the following pins:

- `GND`: ground pins
- `VCCINT`: core power, has to be 1.8V
- `VCCIO{bank}`: I/O power, has to be 1.5V, 1.8.V, 2.5V, or 3.3V
- `VCCAUX`: I/O power for dedicated JTAG pins

- `FB{i}_MC{j}`: general purpose I/O, associated with a macrocell; some of them also have an associated
  special function that can be used in addition to or instead of their plain I/O function:

  - `GCLK[0-2]`: capable of driving `FCLK*` fast clock networks
  - `GSR`: capable of driving `FSR` fast set/reset network
  - `GOE[0-3]`: capable of driving `FOE*` fast output enable networks
  - `DGE` (on larger devices): capable of driving the `DGE` global network,
    controlling optional latched on all input buffers in the device
  - `CDRST` (on larger devices): used as clock divider reset when the clock divider is in use

  The output drivers are powered by the `VCCIO` rails, and the output voltage
  is determined by that rail.

  On larger devices, all I/O pads are also configurable as VREF pins for use with VREF-based
  I/O standards.  Curiously, VREF pins are not preassigned, but can be mostly-arbitrarily
  designated by the designer, subject to some constraints.  The VREF rails are per-bank.

- `IPAD0`: general purpose input, without an associated macrocell; present only on
  the XC2C32 and its variants (in the quantity of one)

- `TCK`, `TMS`, `TDI`, `TDO`: dedicated JTAG pins; the `TDO` pin output driver
  is powered by the `VCCAUX` rail

TODO: determine input banking stuff
