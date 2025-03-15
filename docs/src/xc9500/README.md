# Xilinx XC9500, XC9500XL, XC9500XV CPLDs

XC9500 is a family of flash-based CPLDs manufactured by Xilinx.  It is a derivative of the earlier XC7200 and XC7300
EPLD families.  It comes in three variants:

1. XC9500: 5V core logic, 3.3V or 5V I/O, original version.
2. XC9500XL: 3.3V core logic, 2.5V or 3.3V I/O (5V tolerant), with some functional changes from XC9500:
   - UIM no longer has wired-AND functionality
   - FFs have clock enable function
   - 54 (instead of 36) UIM inputs per function block
   - FF has configurable clock polarity
   - output enable has configurable polarity
   - FOE/FCLK multiplexers and inversion have been removed
   - optional weak bus keeper can be configured for all pins
3. XC9500XV: 2.5V core logic, 1.8V, 2.5V, or 3.3V I/O, with minor functional changes from XC9500XL:
   - larger devices have two or four I/O banks with separate VCCIO
   - the bitstream now contains a `DONE` bit tied to `ISC_DONE`, preventing problems with partially configured devices


## Devices

The following devices exist:

{{devlist xc9500}}

The parts starting with XA are automotive versions.  They are functionally completely identical to corresponding XC versions.


## Packages

The devices come in the following packages:

- PLCC:
  - PC44 (JEDEC MO-047)
  - PC84 (JEDEC MO-047)
- QFP:
  - very thin QFP (1mm thickness):
    - VQ44 (0.8mm pitch; JEDEC MS-026-ACB)
    - VQ64 (0.5mm pitch; JEDEC MS-026-ACD)
  - thin QFP (1.4mm thickness):
    - TQ100 (0.5mm pitch; JEDEC MS-026-BED)
    - TQ144 (0.5mm pitch; JEDEC MS-026-BFB)
  - plastic QFP:
    - PQ100 (2.7mm thickness, non-square, 30×20 pins, 0.65mm pitch; JEDEC MS-022-GC1)
    - PQ160 (3.4mm thickness, 0.65mm pitch; JEDEC MS-022-DD1)
    - PQ208 (3.4mm thickness, 0.5mm pitch; JEDEC MS-029-FA-1)
  - platic QFP with heat sink:
    - HQ208 (same footprint as PQ208)
- BGA:
  - standard BGA (1.27mm pitch):
    - BG256 (JEDEC MS-034-BAL-2)
    - BG352
  - fine-pitch BGA (1mm pitch):
    - FG256
  - chip-scale BGA (0.8mm pitch):
    - CS48
    - CS144 (JEDEC MO-216-BAG-2)
    - CS280 (JEDEC MO-216-BAL-1)

{{devpkg xc9500}}

Pin compatibility is maintained across all 3 variants of the XC9500 family within a single package.
Additionally, devices in PQ208 and HQ208 packages are pin compatible with each other.


## Device pins

The XC9500 family devices have the following pads:

- `GND`: ground pins
- `VCCINT`: core power, has to be:

  - 5V on XC9500
  - 3.3V on XC9500XL
  - 2.5V on XC9500XV

- `VCCIO` or `VCCIO{bank}`: I/O power, has to be:

  - 3.3V or 5V on XC9500
  - 2.5V or 3.3V on XC9500XL
  - 1.8V, 2.5V, or 3.3V on XC9500XV

- `IOB_{i}_{j}`: general purpose I/O, associated with a macrocell (FB `i`, MC `j`); some of them also have an associated
  special function that can be used in addition to or instead of their plain I/O function:

  - `GCLK[0-2]`: capable of driving `FCLK*` fast clock networks
  - `GSR`: capable of driving `FSR` fast set/reset network
  - `GOE[0-1]` (smaller devices) or `GOE[0-3]` (larger devices): capable of driving `FOE*` fast output enable networks

  Curiously, the `GOE` specials mapping to device pads varies with packaging on XC9572* devices.
  How exactly that works is unknown.

  The output drivers are powered by the `VCCIO` rails, and the output voltage
  is determined by that rail.

  For input, the following voltages are supported on all pins, regardless of VCCIO voltage:

  - XC9500: 3.3V or 5V
  - XC9500XL: 2.5V or 3.3V
  - XC9500XV: 2.5V or 3.3V (notably, 1.8V is not supported)

- `TCK`, `TMS`, `TDI`, `TDO`: dedicated JTAG pins; the `TDO` pin output driver
  is powered by one of the `VCCIO` rails
