.. _spartan3-bram:

Block RAM
#########

See :ref:`Virtex 2 documentation <virtex2-bram>` documentation for functional description.


Bitstream
=========

The data for a BRAM is spread across 5 bitstream tiles:

- tiles 0-3: the 4 bitstream tiles that are shared with the ``INT.BRAM.*`` interconnect tiles (starting from the bottom)
- tile 4: the dedicated BRAM data tile located in the BRAM data area; this tile is 76×256 bits; it contains solely the ``DATA`` and ``DATAP`` attributes


``BRAM.S3``
-----------

This tile is used on Spartan 3 devices.

.. raw:: html
   :file: ../gen/tile-xc3s-BRAM.S3.html


``BRAM.S3E``
------------

This tile is used on Spartan 3E devices.

.. raw:: html
   :file: ../gen/tile-xc3s-BRAM.S3E.html


``BRAM.S3A``
------------

This tile is used on Spartan 3A devices.

.. raw:: html
   :file: ../gen/tile-xc3s-BRAM.S3A.html


``BRAM.S3ADSP``
---------------

This tile is used on Spartan 3A DSP devices.

.. raw:: html
   :file: ../gen/tile-xc3s-BRAM.S3ADSP.html


Default option values
=====================

.. raw:: html
   :file: ../gen/xc3s-bram-opts.html
