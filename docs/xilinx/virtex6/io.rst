Input/Output
############


I/O banks and special functions
===============================

Virtex 6 devices have a very regular I/O bank structure. There are up to four I/O columns in the device:

- outer left (sometimes present)
- inner left (always present)
- inner right (always present)
- outer right (sometimes present)

These columns consist entirely of ``IO`` tiles, with one tile per two interconnect rows. Every tile contains two I/O pads: ``IOB0`` and ``IOB1``.  ``IOB0`` is located in the bottom row of the tile, while ``IOB1`` is located in the top row.  Every I/O bank consists of exactly one region, or 40 I/O pads.  The banks are numbered as follows:

- the bank in region ``c + i`` of outer left column (where ``c`` is the region containing the top half of the ``CFG`` tile) has number ``15 + i``
- the bank in region ``c + i`` of inner left column has number ``25 + i``
- the bank in region ``c + i`` of inner right column has number ``35 + i``
- the bank in region ``c + i`` of outer right column has number ``45 + i``

All IOBs in the device are grouped into differential pairs, one pair per IO tile.  ``IOB1`` is the "true" pin of the pair, while ``IOB0`` is the "complemented" pin.  Differential input and true differential output is supported on all pins of the device.

``IOB1`` pads in the 8 rows surrounding the HCLK row (that is, rows 17, 19, 21, 23) are considered "clock-capable".  They can drive ``BUFIODQS`` buffers via dedicated connections.  The ones in rows 19 and 21 can drive ``BUFR`` buffers in this and two surrounding regions, and are considered "multi-region clock capable", while the ones in rows 17 and 23 are considered "single-region clock capable". While Xilinx documentation also considers corresponding ``IOB0`` pads clock-capable, this only means that they can be used together with ``IOB1`` as a differential pair.

There are 8 ``IOB1``\ s that are considered "global clock-capable" and can drive ``BUFGCTRL`` global buffers via dedicated interconnect.  They are:

- bank 24 rows 37, 39
- bank 25 rows 1, 3
- bank 34 rows 37, 39
- bank 35 rows 1, 3

The ``IOB0`` in rows 10 and 30 of every region is capable of being used as a VREF pad.

Each bank has two IOBs that can be used for reference resistors in DCI operation. They are both located in the same I/O tile, with VRP located on ``IOB0`` and VRN located on ``IOB1``. The relevant tile is located as follows:

- bank 24: rows 4-5
- bank 34: rows 0-1
- banks 15, 25, 35: rows 6-7
- all other banks: rows 14-15

In parallel or SPI configuration modes, some I/O pads in banks 24 and 34 are borrowed for configuration use:

- bank 24 row 6: ``CSO_B``
- bank 24 row 7: ``RS[0]``
- bank 24 row 8: ``RS[1]``
- bank 24 row 9: ``FWE_B``
- bank 24 row 10: ``FOE_B/MOSI``
- bank 24 row 11: ``FCS_B``
- bank 24 row 12: ``D[0]/FS[0]``
- bank 24 row 13: ``D[1]/FS[1]``
- bank 24 row 14: ``D[2]/FS[2]``
- bank 24 row 15: ``D[3]``
- bank 24 row 24: ``D[4]``
- bank 24 row 25: ``D[5]``
- bank 24 row 26: ``D[6]``
- bank 24 row 27: ``D[7]``
- bank 24 row 28: ``D[8]``
- bank 24 row 29: ``D[9]``
- bank 24 row 30: ``D[10]``
- bank 24 row 31: ``D[11]``
- bank 24 row 32: ``D[12]``
- bank 24 row 33: ``D[13]``
- bank 24 row 34: ``D[14]``
- bank 24 row 35: ``D[15]``
- bank 34 row 2: ``A[16]``
- bank 34 row 3: ``A[17]``
- bank 34 row 4: ``A[18]``
- bank 34 row 5: ``A[19]``
- bank 34 row 6: ``A[20]``
- bank 34 row 7: ``A[21]``
- bank 34 row 8: ``A[22]``
- bank 34 row 9: ``A[23]``
- bank 34 row 10: ``A[24]``
- bank 34 row 11: ``A[25]``
- bank 34 row 12: ``D[16]/A[0]``
- bank 34 row 13: ``D[17]/A[1]``
- bank 34 row 14: ``D[18]/A[2]``
- bank 34 row 15: ``D[19]/A[3]``
- bank 34 row 24: ``D[20]/A[4]``
- bank 34 row 25: ``D[21]/A[5]``
- bank 34 row 26: ``D[22]/A[6]``
- bank 34 row 27: ``D[23]/A[7]``
- bank 34 row 28: ``D[24]/A[8]``
- bank 34 row 29: ``D[25]/A[9]``
- bank 34 row 30: ``D[26]/A[10]``
- bank 34 row 31: ``D[27]/A[11]``
- bank 34 row 32: ``D[28]/A[12]``
- bank 34 row 33: ``D[29]/A[13]``
- bank 34 row 34: ``D[30]/A[14]``
- bank 34 row 35: ``D[31]/A[15]``

The ``SYSMON`` present on the device can use up to 16 IOB pairs as auxiliary analog differential inputs. The ``VPx`` input corresponds to ``IOB1`` and ``VNx`` corresponds to ``IOB0`` within the same tile. If the device has a outer left IO column, the IOBs are located in banks 15 and 35; otherwise, they are located in banks 25 and 35. The IOBs are in the following tiles:

- ``VP0/VN0``: bank 35 rows 34-35
- ``VP1/VN1``: bank 35 rows 32-33
- ``VP2/VN2``: bank 35 rows 28-29
- ``VP3/VN3``: bank 35 rows 26-27
- ``VP4/VN4``: bank 35 rows 24-25
- ``VP5/VN5``: bank 35 rows 14-15
- ``VP6/VN6``: bank 35 rows 12-13
- ``VP7/VN7``: bank 35 rows 8-9
- ``VP8/VN8``: bank 15/25 rows 34-35
- ``VP9/VN9``: bank 15/25 rows 32-33
- ``VP10/VN10``: bank 15/25 rows 28-29
- ``VP11/VN11``: bank 15/25 rows 26-27
- ``VP12/VN12``: bank 15/25 rows 24-25
- ``VP13/VN13``: bank 15/25 rows 14-15
- ``VP14/VN14``: bank 15/25 rows 12-13
- ``VP15/VN15``: bank 15/25 rows 8-9

The devices also have dedicated configuration bank 0, which has no user I/O and is located in the ``CFG`` tile. It has the following pins:

- ``CCLK``
- ``CSI_B``
- ``DIN``
- ``DONE``
- ``DOUT_BUSY``
- ``HSWAPEN``
- ``INIT_B``
- ``M0``, ``M1``, ``M2``
- ``PROGRAM_B``
- ``RDWR_B``
- ``TCK``, ``TDI``, ``TDO``, ``TMS``


Bitstream — ``IO``
==================

.. raw:: html
   :file: ../gen/tile-xc6v-IO.html


Bitstream — ``HCLK_IOI``
========================

.. raw:: html
   :file: ../gen/tile-xc6v-HCLK_IOI.html


Tables
======

.. raw:: html
   :file: ../gen/xc6v-iostd-drive.html

.. raw:: html
   :file: ../gen/xc6v-iostd-slew.html

.. raw:: html
   :file: ../gen/xc6v-iostd-misc.html

.. raw:: html
   :file: ../gen/xc6v-iostd-lvds.html

.. raw:: html
   :file: ../gen/xc6v-iostd-lvdsbias.html

.. raw:: html
   :file: ../gen/xc6v-iostd-dci-output.html

.. raw:: html
   :file: ../gen/xc6v-iostd-dci-output-half.html

.. raw:: html
   :file: ../gen/xc6v-iostd-dci-term-vcc.html

.. raw:: html
   :file: ../gen/xc6v-iostd-dci-term-split.html

.. raw:: html
   :file: ../gen/xc6v-iodelay-default-idelay-value.html
