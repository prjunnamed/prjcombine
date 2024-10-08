Input/Output
############


I/O banks and special functions
===============================

Virtex 7 devices have a regular I/O bank structure.  There are up to two I/O columns in the device: the left I/O column and the right I/O column.  They contain one I/O bank per region (with the exception of regions that are covered up by the PS or GT holes).

There are two genders of I/O banks:

- HP (high performance) banks, with 1.8V maximum voltage and DCI support
- HR (high range) banks, with 3.3V maximum voltage and no DCI

In both cases, banks are 50 rows high. They have the following structure:

- row 0: contains a ``IO_HP_BOT`` or ``IO_HR_BOT`` tile with a single unpaired IOB
- rows 1-2, 3-4, 5-6, 7-8, ..., 45-46, 47-48: contain ``IO_HP_PAIR`` or ``IO_HR_PAIR`` tiles, which are two rows high and contain two IOBs each, forming a differential pair; ``IOB0`` is located in the bottom (odd) row and is the "complemented" pin of the pair, while ``IOB1`` is in the top (even) row and is the "true" pin of the pair
- row 49: contains another ``IO_HP_TOP`` or ``IO_HR_TOP`` tile
- HCLK row: contains an ``HCLK_IO_HP`` or ``HCLK_IO_HR`` tile with common bank circuitry

The single IOB in row 0 is the VRP pin for DCI. The single IOB in row 49 is VRN pin.

The ``IOB1`` pads in rows 24 and 26 are considered "multi-region clock capable", and have dedicated routing to ``BUFIO`` and ``BUFR`` of this region and the two adjacent ones. The ``IOB1`` pads in rows 22 and 28 are considered "single-region clock capable", and can drive ``BUFIO`` and ``BUFR`` only within their own region.

The ``IOB0`` pads in rows 11 and 37 can be used as VREF.

The ``IOB1`` pads in rows 8, 20, 32, 44 can be used as DQS for byte groups. The byte groups are:

- rows 1-12: byte group with DQS in row 8
- rows 13-24: byte group with DQS in row 20
- rows 25-36: byte group with DQS in row 32
- rows 37-48: byte group with DQS in row 44

The banks are numbered as follows, where ``c`` is the region with the ``CFG`` tile (for multi-die packages, the ``CFG`` tile of the primary device):

- the bank in left column region ``c + i`` is ``14 + i``
- the bank in right column region ``c + i`` is ``34 + i``

In case of multi-die packages, this numbering continues across devices within the package.

In parallel or SPI configuration modes, some I/O pads in banks 14 and 15 are borrowed for configuration use:

- bank 14 row 1: ``A[0]/D[16]``
- bank 14 row 2: ``A[1]/D[17]``
- bank 14 row 3: ``A[2]/D[18]``
- bank 14 row 4: ``A[3]/D[19]``
- bank 14 row 5: ``A[4]/D[20]``
- bank 14 row 6: ``A[5]/D[21]``
- bank 14 row 7: ``A[6]/D[22]``
- bank 14 row 9: ``A[7]/D[23]``
- bank 14 row 10: ``A[8]/D[24]``
- bank 14 row 11: ``A[9]/D[25]``
- bank 14 row 12: ``A[10]/D[26]``
- bank 14 row 13: ``A[11]/D[27]``
- bank 14 row 14: ``A[12]/D[28]``
- bank 14 row 15: ``A[13]/D[29]``
- bank 14 row 16: ``A[14]/D[30]``
- bank 14 row 17: ``A[15]/D[31]``
- bank 14 row 18: ``CSI_B``
- bank 14 row 19: ``DOUT/CSO_B``
- bank 14 row 20: ``RDWR_B``
- bank 14 row 29: ``D[15]``
- bank 14 row 30: ``D[14]``
- bank 14 row 31: ``D[13]``
- bank 14 row 33: ``D[12]``
- bank 14 row 34: ``D[11]``
- bank 14 row 36: ``D[10]``
- bank 14 row 36: ``D[9]``
- bank 14 row 37: ``D[8]``
- bank 14 row 38: ``FCS_B``
- bank 14 row 39: ``D[7]``
- bank 14 row 40: ``D[6]``
- bank 14 row 41: ``D[5]``
- bank 14 row 42: ``D[4]``
- bank 14 row 43: ``EM_CCLK``
- bank 14 row 44: ``PUDC_B``
- bank 14 row 45: ``D[3]``
- bank 14 row 46: ``D[2]``
- bank 14 row 47: ``D[1]/DIN``
- bank 14 row 48: ``D[0]/MOSI``
- bank 15 row 1: ``RS[0]``
- bank 15 row 2: ``RS[1]``
- bank 15 row 3: ``FWE_B``
- bank 15 row 4: ``FOE_B``
- bank 15 row 5: ``A[16]``
- bank 15 row 6: ``A[17]``
- bank 15 row 7: ``A[18]``
- bank 15 row 9: ``A[19]``
- bank 15 row 10: ``A[20]``
- bank 15 row 11: ``A[21]``
- bank 15 row 12: ``A[22]``
- bank 15 row 13: ``A[23]``
- bank 15 row 14: ``A[24]``
- bank 15 row 15: ``A[25]``
- bank 15 row 16: ``A[26]``
- bank 15 row 17: ``A[27]``
- bank 15 row 18: ``A[28]``
- bank 15 row 19: ``ADV_B``

Some 

The devices with Processing System are not configured by normal means, so the above list is inapplicable.  Furthermore, they do not have banks 14 and 15 at all — the place they would occupy is taken up by the PS itself.  They do, however, have a special pin in bank 34 instead:

- bank 34 row 44: ``PUDC_B``

.. todo:: really, Wanda, how surprised would you be if it turned out that they *are* configurable by normal means by just substituting banks 34+35 and poking at the reserved mode pins that definitely aren't ``M0/M1/M2``?

The ``XADC``, if present on the device, can use up to 16 IOB pairs as auxiliary analog differential inputs. The ``VPx`` input corresponds to ``IOB1`` and ``VNx`` corresponds to ``IOB0`` within the same tile.  Depending on device banks present on the device, there are three different arrangements possible:

- variant LR, used for devices that have both bank 15 and 35
- variant L, used for devices without bank 35
- variant R, used for devices without bank 15 (that is, devices with Processing System)

The IOBs for variant LR are:

- ``VP0/VN0``: bank 15 rows 47-48
- ``VP1/VN1``: bank 15 rows 43-44
- ``VP2/VN2``: bank 15 rows 35-36
- ``VP3/VN3``: bank 15 rows 31-32
- ``VP4/VN4``: bank 35 rows 47-48
- ``VP5/VN5``: bank 35 rows 43-44
- ``VP6/VN6``: bank 35 rows 35-31
- ``VP7/VN7``: bank 35 rows 31-32
- ``VP8/VN8``: bank 15 rows 45-46
- ``VP9/VN9``: bank 15 rows 39-40
- ``VP10/VN10``: bank 15 rows 33-34
- ``VP11/VN11``: bank 15 rows 29-30
- ``VP12/VN12``: bank 35 rows 45-46
- ``VP13/VN13``: bank 35 rows 39-40
- ``VP14/VN14``: bank 35 rows 33-34
- ``VP15/VN15``: bank 35 rows 29-30

The IOBs for variant L are:

- ``VP0/VN0``: bank 15 rows 47-48
- ``VP1/VN1``: bank 15 rows 43-44
- ``VP2/VN2``: bank 15 rows 39-40
- ``VP3/VN3``: bank 15 rows 33-34
- ``VP4/VN4``: bank 15 rows 29-30
- ``VP5/VN5``: bank 15 rows 25-26
- ``VP6/VN6``: unconnected
- ``VP7/VN7``: unconnected
- ``VP8/VN8``: bank 15 rows 45-46
- ``VP9/VN9``: bank 15 rows 41-42
- ``VP10/VN10``: bank 15 rows 35-36
- ``VP11/VN11``: bank 15 rows 31-32
- ``VP12/VN12``: bank 15 rows 27-28
- ``VP13/VN13``: unconnected
- ``VP14/VN14``: unconnected
- ``VP15/VN15``: unconnected

The IOBs for variant R are:

- ``VP0/VN0``: bank 35 rows 47-48
- ``VP1/VN1``: bank 35 rows 43-44
- ``VP2/VN2``: bank 35 rows 35-36
- ``VP3/VN3``: bank 35 rows 31-32
- ``VP4/VN4``: bank 35 rows 21-22
- ``VP5/VN5``: bank 35 rows 15-16
- ``VP6/VN6``: bank 35 rows 9-10
- ``VP7/VN7``: bank 35 rows 5-6
- ``VP8/VN8``: bank 35 rows 45-46
- ``VP9/VN9``: bank 35 rows 39-40
- ``VP10/VN10``: bank 35 rows 33-34
- ``VP11/VN11``: bank 35 rows 29-30
- ``VP12/VN12``: bank 35 rows 19-20
- ``VP13/VN13``: bank 35 rows 13-14
- ``VP14/VN14``: bank 35 rows 7-8
- ``VP15/VN15``: bank 35 rows 1-2

The devices also have dedicated configuration bank 0, which has no user I/O and is located in the ``CFG`` tile. It has the following pins:

- ``CCLK``
- ``CFGBVS``
- ``DONE``
- ``INIT_B``
- ``M0``, ``M1``, ``M2``
- ``PROGRAM_B``
- ``TCK``, ``TDI``, ``TDO``, ``TMS``


Bitstream — ``IO_HP_PAIR``
==========================

.. raw:: html
   :file: ../gen/tile-xc7v-IO_HP_PAIR.html


Bitstream — ``IO_HP_BOT``
=========================

.. raw:: html
   :file: ../gen/tile-xc7v-IO_HP_BOT.html


Bitstream — ``IO_HP_TOP``
=========================

.. raw:: html
   :file: ../gen/tile-xc7v-IO_HP_TOP.html


Bitstream — ``IO_HR_PAIR``
==========================

.. raw:: html
   :file: ../gen/tile-xc7v-IO_HR_PAIR.html


Bitstream — ``IO_HR_BOT``
=========================

.. raw:: html
   :file: ../gen/tile-xc7v-IO_HR_BOT.html


Bitstream — ``IO_HR_TOP``
=========================

.. raw:: html
   :file: ../gen/tile-xc7v-IO_HR_TOP.html


Bitstream — ``HCLK_IOI_HP``
===========================

.. raw:: html
   :file: ../gen/tile-xc7v-HCLK_IOI_HP.html


Bitstream — ``HCLK_IOI_HR``
===========================

.. raw:: html
   :file: ../gen/tile-xc7v-HCLK_IOI_HR.html


Tables — HP IO
==============

.. raw:: html
   :file: ../gen/xc7v-hp-iostd-drive.html

.. raw:: html
   :file: ../gen/xc7v-hp-iostd-slew.html

.. raw:: html
   :file: ../gen/xc7v-hp-iostd-lvds.html

.. raw:: html
   :file: ../gen/xc7v-hp-iostd-lvdsbias.html

.. raw:: html
   :file: ../gen/xc7v-hp-iostd-dci-output.html

.. raw:: html
   :file: ../gen/xc7v-hp-iostd-dci-output-half.html

.. raw:: html
   :file: ../gen/xc7v-hp-iostd-dci-term-split.html


Tables — HR IO
==============

.. raw:: html
   :file: ../gen/xc7v-hr-iostd-drive.html

.. raw:: html
   :file: ../gen/xc7v-hr-iostd-slew.html

.. raw:: html
   :file: ../gen/xc7v-hr-iostd-misc.html

.. raw:: html
   :file: ../gen/xc7v-hr-iostd-lvds.html

.. raw:: html
   :file: ../gen/xc7v-hr-iostd-driverbias.html

.. raw:: html
   :file: ../gen/xc7v-hr-iostd-lvdsbias.html
