.. _spartan3-clock:

Clock interconnect
##################

.. todo:: document


Clock source — spine bottom and top
===================================

.. todo:: document


Bitstream — bottom tiles
------------------------

The ``CLKB.*`` tiles use two bitstream tiles:

- tile 0: 1×64 (Spartan 3, 3E) or 2×64 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the bottom interconnect row
- tile 1: 1×16 (Spartan 3, 3E) or 2×16 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the low special area (used for bottom ``IOB`` tiles and clock rows in normal columns)

On Spartan 3A devices that have long line splitters, bitstream tile 0 is shared with the ``LLH.CLKB.S3A`` tile.


``CLKB.S3``
+++++++++++

This tile is used on Spartan 3.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-CLKB.S3.html


``CLKB.S3E``
++++++++++++

This tile is used on Spartan 3E.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-CLKB.S3E.html


``CLKB.S3A``
++++++++++++

This tile is used on Spartan 3A and 3A DSP.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-CLKB.S3A.html


Bitstream — top tiles
---------------------

The ``CLKT.*`` tiles use two bitstream tiles:

- tile 0: 1×64 (Spartan 3, 3E) or 2×64 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the top interconnect row
- tile 1: 1×16 (Spartan 3, 3E) or 2×16 (Spartan 3A) tile located in the primary clock spine column, in the bits corresponding to the high special area (used for top ``IOB`` tiles and clock rows in normal columns)

On Spartan 3A devices that have long line splitters, bitstream tile 0 is shared with the ``LLH.CLKT.S3A`` tile.


``CLKT.S3``
+++++++++++

This tile is used on Spartan 3.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-CLKT.S3.html


``CLKT.S3E``
++++++++++++

This tile is used on Spartan 3E.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-CLKT.S3E.html


``CLKT.S3A``
++++++++++++

This tile is used on Spartan 3A and 3A DSP.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-CLKT.S3A.html


Clock source — left and right
=============================

.. todo:: document


The ``CLKC`` clock center tile
==============================

The ``CLKC`` tile is located in the center of the FPGA (intersection of primary vertical and horizontal clock spines) of all devices except ``xc3s50a``. It has permanent buffers forwarding the clock signals from ``CLKB`` and ``CLKT`` to ``GCLKVM``. It has no configuration.

.. todo:: describe exact forwarding


The ``GCLKVM`` secondary clock center tiles
===========================================

The ``GCLKVM`` tiles are located on the intersection of secondary vertical clock spines and the horizontal clock spine.

The Spartan 3 version has permanent buffers forwarding the clock signals from ``CLKC`` to ``GCLKVC`` tiles. It has no configuration.

The Spartan 3E and 3A versions multiplex clock signals from ``CLK[LR]`` and ``CLKC`` tiles.

.. todo:: document Spartan 3E/3A version


The ``GCLKVC`` clock spine distribution tiles
=============================================

.. todo:: document


The ``GCLKH`` clock row distribution tiles
==========================================

.. todo:: document