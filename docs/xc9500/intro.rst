Introduction
############

XC9500 is a family of flash-based CPLDs manufactured by Xilinx.  It is a derivative of the earlier XC7200 and XC7300
EPLD families.  It comes in three variants:

1. XC9500: 5V core logic, 3.3V or 5V I/O, original version.
2. XC9500XL: 3.3V core logic, 2.5V or 3.3V I/O (5V tolerant), with some functional changes from XC9500:

   - UIM no longer has wired-AND functionality
   - FFs have clock enable function
   - 54 (instead of 36) UIM inputs per function block
   - FF has configurable clock polarity
   - FOE/FCLK multiplexers and inversion have been removed
   - optional weak bus keeper can be configured for all pins

3. XC9500XV: 2.5V core logic, 1.8V, 2.5V, or 3.3V I/O, with minor functional changes from XC9500XL:

   - larger devices have two I/O banks with separate VCCIO
   - the bitstream now contains a ``DONE`` bit tied to ``ISC_DONE``, preventing problems with partially configured devices


Devices
=======

The following devices exist:

========= ======== ===============
Device    Variant  Function Blocks
========= ======== ===============
XC9536    XC9500   2
XC9572    XC9500   4
XC95108   XC9500   6
XC95144   XC9500   8
XC95216   XC9500   12
XC95288   XC9500   16
XC9536XL  XC9500XL 2
XC9572XL  XC9500XL 4
XC95144XL XC9500XL 8
XC95288XL XC9500XL 16
XA9536XL  XC9500XL 2
XA9572XL  XC9500XL 4
XA95144XL XC9500XL 8
XC9536XV  XC9500XV 2
XC9572XV  XC9500XV 4
XC95144XV XC9500XV 8
XC95288XV XC9500XV 16
========= ======== ===============

The parts starting with XA are automotive versions.  They are functionally completely identical to corresponding XC versions.


Packages
========

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


========= ==== ==== ==== ==== ===== ===== ===== ===== ===== ===== ===== ===== ===== ==== ===== =====
Device    PC44 PC84 VQ44 VQ64 TQ100 TQ144 PQ100 PQ160 PQ208 HQ208 BG256 BG352 FG256 CS48 CS144 CS280
========= ==== ==== ==== ==== ===== ===== ===== ===== ===== ===== ===== ===== ===== ==== ===== =====
XC9536    X         X                                                               X
XC9572    X    X              X           X
XC95108        X              X           X     X
XC95144                       X           X     X
XC95216                                         X           X           X
XC95288                                                     X           X
XC9536XL  X         X    X                                                          X
XC9572XL  X         X    X    X                                                     X
XC95144XL                     X     X                                                    X
XC95288XL                           X                 X           X           X                X
XA9536XL            X
XA9572XL            X    X    X
XA95144XL                                                                                X
XC9536XV  X         X                                                               X
XC9572XV  X         X         X                                                     X
XC95144XV                     X     X                                                    X
XC95288XV                           X                 X                       X                X
========= ==== ==== ==== ==== ===== ===== ===== ===== ===== ===== ===== ===== ===== ==== ===== =====

Pin compatibility is maintained across all 3 variants of the XC9500 family within a single package.
Additionally, devices in PQ208 and HQ208 packages are pin compatible with each other.