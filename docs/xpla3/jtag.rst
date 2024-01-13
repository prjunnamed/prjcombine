JTAG interface
##############

IR
==

The IR is 5 bits long.  The following instructions exist:

========= =============== =============
IR        Instruction     Register
========= =============== =============
``00000`` ``EXTEST``      ``BOUNDARY``
``00001`` ``IDCODE``      ``IDCODE``
``00010`` ``SAMPLE``      ``BOUNDARY``
``00011`` ``INTEST``      ``BOUNDARY``
``00100`` ``STRTEST``     ``BOUNDARY``
``00101`` ``HIGHZ``       ``BYPASS``
``00110`` ``CLAMP``       ``BYPASS``
``00111`` ``ISP_WRITE``   ``MISR``
``01000`` ``ISP_EOTF``    ``MISR``
``01001`` ``ISP_ENABLE``  ``MISR``
``01010`` ``ISP_ERASE``   ``MISR``
``01011`` ``ISP_PROGRAM`` ``MISR``
``01100`` ``ISP_VERIFY``  ``MISR``
``01101`` ``ISP_INIT``    ``BYPASS``
``01110`` ``ISP_READ``    ``MISR``
``10000`` ``ISP_DISABLE`` ``MISR``
``10001`` ``TEST_MODE``   ``MISR``
``11111`` ``BYPASS``      ``BYPASS``
========= =============== =============

The IR status is:

- bit 0: const 1
- bits 1-4: const 0 [?]

.. todo:: completely unverified from BSDL


Boundary scan register
======================

.. todo:: write me


ISP instructions
================

.. todo:: write me


Programming sequence
====================

.. todo:: write me
