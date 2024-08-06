.. _fpgacore-clb:

Configurable Logic Block
########################

The CLB is identical to Spartan 3.


Bitstream
=========

The data for a CLB is located in the same bitstream tile as the associated ``INT.CLB`` tile.

.. raw:: html
   :file: ../gen/tile-xcexf-CLB.html


``RESERVED_ANDOR``
==================

TODO: wtf is this even


``RANDOR``
----------

This tile overlaps ``IOI.*``.

.. raw:: html
   :file: ../gen/tile-xcexf-RANDOR.html


``RANDOR_INIT``
---------------

This tile overlaps top-left interconnect tile.

.. raw:: html
   :file: ../gen/tile-xcexf-RANDOR_INIT.html
