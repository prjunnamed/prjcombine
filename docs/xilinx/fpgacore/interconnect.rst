.. _fpgacore-interconnect:

General interconnect
####################

FPGAcore interconnect is identical to Spartan 3 with one exception: there are 12 long lines for each orientation instead of 24.


Bitstream — interconnect tiles
==============================

The interconnect tiles are 19×64 bits. The space on the left is unused by the interconnect tile, and contains data for whatever primitive is associated with the interconnect tile.

``INT.CLB``
-----------

Used with ``CLB`` tiles.

.. raw:: html
   :file: ../gen/tile-xcexf-INT.CLB.html


``INT.IOI.FC``
--------------

Used with ``IOI`` tiles.

.. raw:: html
   :file: ../gen/tile-xcexf-INT.IOI.FC.html
