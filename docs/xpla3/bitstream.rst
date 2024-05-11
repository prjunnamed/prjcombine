Bitstream structure
###################

The raw XPLA3 bitstream is a three-dimensional array of bits. The bitstream is arranged into:

- ``fb_rows * 52 + 2`` rows
- 2 planes
- ``bs_cols`` columns

The ``fb_rows`` and ``bs_cols`` variables are per-device and can be obtained from the database.

The bitstream is roughly divided into areas as follows:

- the last row is used to store the UES (user electronic signature)
- the second to last row is used to store the ``READ_PROT`` and ``ISP_DISABLE`` bits, which should generally be programmed last in the sequence
- all other rows are used to store (mostly) per-FB data, with 52 bitstream rows per FB row
  - each FB column has three ranges of columns, the start of each range is stored in the database:
    - IMUX bits (includes per-FB input multiplexers and the per-column ``ZIA_GCLK*_ENABLE`` bits)
    - PT bits (includes product term bits and PLA sum term bits)
    - MC bits (includes per-MC config bits and misc per-FB config bits; also includes global bits configuring UCT muxes)


JED structure
=============

XPLA3 bitstreams are commonly stored in JED files. Bits in JED files are stored in a logical order completely unrelated to the physical structure of a bitstream. The order of bits in a JED file is as follows:

- per-FB bits, for every FB ``i`` in order:
  - IMUX bits, for every ``j < 40``:
    - every bit of ``FB[i].IM[j].MUX`` in order
  - for every product term ``j < 48``, in order:
    - for every FB input ``k < 40``, in order:
      - ``FB[i].PT[i].IM[k].P``
      - ``FB[i].PT[i].IM[k].N``
    - for every feedback term ``k < 8``, in order:
      - ``FB[i].PT[i].FBN[k]``
  - for every product term ``j < 48``, in order:
    - for every macrocell ``k < 16``, in order:
      - ``FB[i].MC[k].SUM.PT[j]``
  - misc per-FB bits, in order :ref:`given below <xpla3-fb-bits-jed>`
  - for every macrocell with an IOB, in order (which macrocells have a IOB can be checked in the database ``io_mcs`` field):
    - misc per-MC bits, in order :ref:`given below <xpla3-mc-bits-jed-iob>`
  - for every macrocell without an IOB, in order:
    - misc per-MC bits, in order :ref:`given below <xpla3-mc-bits-jed-buried>`
- all global bits, in order given in the per-device database

Note that the UES bits and read protection bit are not included in the JED file, and must be provided out of band.


Fuses — IMUX bits
=================

The size of every ``FB[i].IM[j].MUX`` fuse set in the device is the same, and is given by the database ``imux_width`` field. The position of bit ``k`` of ``FB[i].IM[j].MUX`` in the fuse array can be computed as follows:

- row is:
  - if ``j < 20``: ``fb_row(i) * 52 + 2 + j``
  - if ``j >= 20``: ``fb_row(i) * 52 + 2 + j + 8``
- plane is: ``(i & 1) ^ 1``
- column is ``fb_cols[fb_col(i)].imux_col + imux_width - 1 - k``


Fuses — product and sum terms
=============================

The position of ``FB[i].PT[j].IM[k].P`` and ``FB[i].PT[j].IM[k].N`` can be computed as follows:

- row is:
  - if ``k < 20``: ``fb_row(i) * 52 + 2 + k``
  - if ``k >= 20``: ``fb_row(i) * 52 + 2 + k + 8``
- plane is ``0`` for ``.P`` bit, ``1`` for ``.N`` bit
- column is:
  - if ``i`` is even: ``fb_cols[fb_col(i)].pt_col + j``
  - if ``i`` is odd: ``fb_cols[fb_col(i)].pt_col + 95 - j``

The position of ``FB[i].PT[j].FBN[k]`` can be computed as follows:

- row is ``fb_row(i) * 52 + dr``, where ``dr`` is given in the table below
- plane is given in the table below
- column is:
  - if ``i`` is even: ``fb_cols[fb_col(i)].pt_col + j``
  - if ``i`` is odd: ``fb_cols[fb_col(i)].pt_col + 95 - j``

===== ====== =========
``k`` ``dr`` ``plane``
===== ====== =========
0     0      1
1     0      0
2     1      1
3     1      0
4     50     0
5     50     1
6     51     0
7     51     1
===== ====== =========

The position of ``FB[i].MC[j].SUM.PT[k]`` can be computed as follows:

- row is ``fb_row(i) * 52 + 22 + (j // 2)``
- plane is ``1 - (j % 2)``
- column is:
  - if ``i`` is even: ``fb_cols[fb_col(i)].pt_col + k``
  - if ``i`` is odd: ``fb_cols[fb_col(i)].pt_col + 95 - k``


Fuses — macrocells
==================

The per-MC bits are listed in the table below. The per-MC coordinates should be translated to global coordinates as follows:

- row is:
  if ``mc < 8``: ``fb_row(fb) * 52 + mc * 3 + table_row``
  if ``mc >= 8``: ``fb_row(fb) * 52 + 4 + mc * 3 + table_row``
- plane is: ``table_plane``
- column is:
  - if ``fb`` is even: ``fb_cols[fb_col(fb)].mc_col + table_column``
  - if ``fb`` is odd: ``fb_cols[fb_col(fb)].mc_col + 9 - table_column``

.. raw:: html
   :file: gen-tile-mc.html


.. _xpla3-mc-bits-jed-iob:

JED mapping — macrocells with IOB
---------------------------------

.. raw:: html
   :file: gen-jed-mc-iob.html


.. _xpla3-mc-bits-jed-buried:

JED mapping — macrocells without IOB
------------------------------------

.. raw:: html
   :file: gen-jed-mc-buried.html


Fuses — FBs
===========

The per-FB bits are listed in the table below. The per-FB coordinates should be translated to global coordinates as follows:

- row is: ``fb_row(fb) * 52 + 24 + table_row``
- plane is: ``table_plane``
- column is:
  - if ``fb`` is even: ``fb_cols[fb_col(fb)].mc_col + table_column``
  - if ``fb`` is odd: ``fb_cols[fb_col(fb)].mc_col + 9 - table_column``

.. raw:: html
   :file: gen-tile-fb.html


.. _xpla3-fb-bits-jed:

JED mapping
-----------

.. raw:: html
   :file: gen-jed-fb.html


Fuses — global bits
===================

The global bits are given by their raw position in the per-device database.