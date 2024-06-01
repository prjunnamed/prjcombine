.. _virtex2-clock:

Clock interconnect
##################

.. todo:: describe this madness


Clock source — spine bottom and top
===================================

.. todo:: document


Bitstream — bottom tiles
------------------------

The ``CLKB.*`` tiles use two bitstream tiles:

- tile 0: 4×80 tile located in the clock spine column, in the bits corresponding to the bottom interconnect row
- tile 1: 4×16 tile located in the clock spine column, in the bits corresponding to the low special area (used for bottom ``IOB`` tiles and clock rows in normal columns)


``CLKB.V2``
+++++++++++

This tile is used on Virtex 2 devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-CLKB.V2.html

``CLKB.V2P``
++++++++++++

This tile is used on Virtex 2 Pro devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-CLKB.V2P.html

``CLKB.V2PX``
+++++++++++++

This tile is used on Virtex 2 Pro X devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-CLKB.V2PX.html


Bitstream — top tiles
---------------------

The ``CLKT.*`` tiles use two bitstream tiles:

- tile 0: 4×80 tile located in the clock spine column, in the bits corresponding to the top interconnect row
- tile 1: 4×16 tile located in the clock spine column, in the bits corresponding to the high special area (used for top ``IOB`` tiles and clock rows in normal columns)


``CLKT.V2``
+++++++++++

This tile is used on Virtex 2 devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-CLKT.V2.html


``CLKT.V2P``
++++++++++++

This tile is used on Virtex 2 Pro devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-CLKT.V2P.html


``CLKT.V2PX``
+++++++++++++

This tile is used on Virtex 2 Pro X devices.

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-CLKT.V2PX.html


The ``GCLKC`` clock spine distribution tiles
============================================

.. todo:: document


``GCLKC``
---------

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-GCLKC.html


``GCLKC.B``
-----------

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-GCLKC.B.html


``GCLKC.T``
-----------

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-GCLKC.T.html


The clock row distribution tiles
================================

.. todo:: document

``GCLKH``
---------

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-GCLKH.html


DCM output bus
==============

.. todo:: document


``DCMCONN.BOT``
---------------

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-DCMCONN.BOT.html


``DCMCONN.TOP``
---------------

.. raw:: html
   :file: ../gen-xilinx-tile-xc2v-DCMCONN.TOP.html
