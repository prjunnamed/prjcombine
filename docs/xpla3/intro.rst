Introduction
############

XPLA3 is a family of flash-based 3.3V CPLDs orignally designed by Philips, and later acquired by Xilinx (along with Philips entire CPLD division).  It is a derivative of Philips earlier XPLA and XPLA2 architectures.


Devices
=======

The following devices exist:

.. include:: gen-devices.inc

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

.. include:: gen-devices-pkg.inc

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
