.. _spartan3-interconnect:

General interconnect
####################


Bitstream — interconnect tiles
==============================

The interconnect tiles are 19×64 bits. The space on the left is unused by the interconnect tile, and contains data for whatever primitive is associated with the interconnect tile.

``INT.CLB``
-----------

Used with ``CLB`` tiles and the corner tiles.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.CLB.html


``INT.IOI.S3``
--------------

Used with ``IOI`` tiles on Spartan 3.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.IOI.S3.html


``INT.IOI.S3E``
---------------

Used with ``IOI`` tiles on Spartan 3E.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.IOI.S3E.html


``INT.IOI.S3A.LR``
------------------

Used with ``IOI`` tiles on Spartan 3A / 3A DSP that are on the left or right edge of the device.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.IOI.S3A.LR.html


``INT.IOI.S3A.TB``
------------------

Used with ``IOI`` tiles on Spartan 3A / 3A DSP that are on the top or bottom edge of the device.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.IOI.S3A.TB.html


``INT.BRAM.S3``
---------------

Used with ``BRAM.S3`` tiles on Spartan 3.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.BRAM.S3.html


``INT.BRAM.S3E``
----------------

Used with ``BRAM.S3E`` tiles on Spartan 3E.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.BRAM.S3E.html


``INT.BRAM.S3A.03``
-------------------

Used with ``BRAM.S3A`` tiles on Spartan 3A. This interconnect tile is used in rows 0 and 3 of the BRAM.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.BRAM.S3A.03.html


``INT.BRAM.S3A.12``
-------------------

Used with ``BRAM.S3A`` tiles on Spartan 3A. This interconnect tile is used in rows 1 and 2 of the BRAM.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.BRAM.S3A.12.html


``INT.BRAM.S3ADSP``
-------------------

Used with ``BRAM.S3ADSP`` or ``DSP`` tiles on Spartan 3A DSP.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.BRAM.S3ADSP.html


``INT.DCM``
-----------

Used with ``DCM.*`` tiles.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.DCM.html


``INT.DCM.S3E.DUMMY``
---------------------

Used for the dummy interconnect tile in DCM holes on Spartan 3E devices with 2 DCMs. Not associated with any primitive.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-INT.DCM.S3E.DUMMY.html


Bitstream — ``LLH`` tiles
=========================

Used for ``LLH`` tiles that split horizontal long lines, physically located on the primary vertical clock spine. Such tiles are located on the intersection of frames 2-3 of the clock spines bitstream column and interconnect rows, making them 2×64 in size.

``LLH``
-------

This type of tile is used for all horizontal splitters on Spartan 3E, and for horizontal splitters in general rows on Spartan 3A and Spartan 3A DSP. On Spartan 3A and 3A DSP, the horizontal splitters in IO rows have special tile types.

On Spartan 3E, the bitstream area is also used by ``CLKB.S3E`` and ``CLKT.S3E`` in the bottom and top rows.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-LLH.html


``LLH.CLKB.S3A``
----------------

This type of tile is used for horizontal splitters in the bottom IO row on Spartan 3A and Spartan 3A DSP. The same bitstream area is also used for ``CLKB.S3A``.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-LLH.CLKB.S3A.html


``LLH.CLKT.S3A``
----------------

This type of tile is used for horizontal splitters in the top IO row on Spartan 3A and Spartan 3A DSP. The same bitstream area is also used for ``CLKT.S3A``.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-LLH.CLKT.S3A.html


Bitstream — ``LLV`` tiles
=========================

Used for ``LLV`` tiles that split vertical long lines. Physically located on the horizontal clock spine.

Note that the columns inside BRAM interconnect hole


``LLV.S3E``
-----------

On Spartan 3E, the data for ``LLV`` tiles is split into two bitstream tiles: a 19×1 tile that lives in the bottom special area of every interconnect column, and a 19×2 tile that lives in the top special area of every interconnect column.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-LLV.S3E.html


``LLV.S3A``
-----------

On Spartan 3A and Spartan 3A DSP, the data for ``LLV`` tiles lives in 19×3 bitstream tiles in the top special area of every interconnect column.

.. raw:: html
   :file: ../gen-xilinx-tile-xc3s-LLV.S3A.html
