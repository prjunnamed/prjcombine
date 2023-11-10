Introduction
############

XPLA3 is a family of flash-based 3.3V CPLDs manufactured by Xilinx.  It is a derivative of the earlier XPLA architectures
made by Phillips (whose CPLD division has been acquired by Xilinx).


Devices
=======

The following devices exist:

========= =============== =========
Device    Function Blocks FB groups
========= =============== =========
XCR3032XL 2               1
XCR3064XL 4               1
XCR3128XL 8               2
XCR3256XL 16              2
XCR3384XL 24              3
XCR3512XL 32              4
========= =============== =========

Packages
========

The devices come in the following packages:

- PLCC:

  - PC44 (JEDEC MO-047)

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

  - chip-scale BGA (0.8mm pitch):

    - CS48
    - CS144 (JEDEC MO-216-BAG-2)
    - CS280 (JEDEC MO-216-BAL-1)

  - chip-scale BGA (0.5mm pitch):

    - CP56

========= ==== ==== ===== ===== ===== ==== ==== ===== ===== ===== =====
Device    PC44 VQ44 VQ100 TQ144 PQ208 CP56 CS48 CS144 CS280 FT256 FG324
========= ==== ==== ===== ===== ===== ==== ==== ===== ===== ===== =====
XCR3032XL X    X                           X
XCR3064XL X    X    X                 X    X
XCR3128XL           X     X                     X
XCR3256XL                 X     X                     X     X
XCR3384XL                 X     X                           X     X
XCR3512XL                       X                           X     X
========= ==== ==== ===== ===== ===== ==== ==== ===== ===== ===== =====

Pin compatibility is maintained across devices within a single package, with one exception:

- XCR3384XL has the JTAG pins on different package pins in the TQ144 package only


Device pins
===========

The XPLA3 devices have the following pins:

- ``GND``: ground pins
- ``VCC``: power pins (3.3V), used for both core and I/O
- ``GCLK[0-3]``: dedicated input pins; can be used as general-purpose inputs, or drive ``FCLK`` fast clock networks
- ``FB{i}_MC{j}``: general purpose I/O, associated with a macrocell; some of them also have an associated
  special function that can be used instead of their plain I/O function:

  - ``TCK``, ``TMS``, ``TDI``, ``TDO``: JTAG pins

  The JTAG function of dual-purpose pads is enabled when the ``ISP_DISABLE`` fuse is not programmed
  (including a completely unconfigured device), or when the ``PORT_EN`` dedicated input is set to 1.

- ``PORT_EN``: special dedicated input that unconditionally enables the ISP JTAG function of dual-purpose I/O pads when set to 1.