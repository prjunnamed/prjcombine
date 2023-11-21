Bitstream structure — XC9500
############################

On a high level, the whole bitstream is split into "areas".  Each FB
of the device corresponds two areas, one of which contains the UIM wire-AND
configuration, and the other (main area) contains everything else.

The main area of a FB is made of 72 "rows".  Each row is made of 15 "columns".
Each column is made of 6 or 8 bits: columns 0-8 are made of 8 bits, while
columns 9-14 are made of 6 bits.

The low 6 bits of every column are used to store product term masks, and
the high 2 bits of columns 0-8 are used to store everything else.

The UIM wire-AND area of a FB is, in turn, made of "subareas", one for each
FB of the device.  Each subarea is in turn made of 18 rows.  Each row
is made of 5 columns.  Column 0 is made of 8 bits, while columns 1-4 are made
of 7 bits, making for 36 total bits per row.

When programmed or read via JTAG, the bitstream is transmitted as bytes,
which are 6-8 bits long.  Each byte of the bitstream has its address.
Not all addresses are valid, and valid addresses are not contiguous.
Address is 17 bits long, and is split to several fields:

- bits 13-16: FB index
- bit 12: area kind

  - 0: main FB config
  - 1: UIM wire-AND config

- for main FB config area:

  - bits 5-11: row
  - bits 3-4: column / 5
  - bits 0-2: column % 5

- for UIM wire-AND config area:

  - bits 8-11: subarea (source FB index)
  - bits 3-7: row
  - bits 0-2: column

The unprogrammed state of a bit on XC9500 is ``1``.
The programmed state is ``0``.  Thus, whenever a boolean fuse is mentioned
in the documentation, the "true" value is actually represented as ``0``
in the bitstream.  This includes the USERCODE bits.


JED format mapping
==================

In the JED format, all fuses of the device are simply concatenated in order,
skipping over invalid addresses.  The bytes are *not* padded to 8 bits, but
have their native size.  Thus, converting from JED fuse index to device
address involves some complex calculations::

    main_row_bits = 8 * 9 + 6 * 6
    uim_row_bits = 8 + 7 * 4
    main_area_bits = main_row_bits * 72
    uim_subarea_bits = uim_row_bits * 18
    uim_area_bits = uim_subarea_bits * device.num_fbs
    fb_bits = main_area_bits + uim_area_bits
    total_bits = fb_bits * device.num_fbs

    def jed_to_jtag(fuse):
        fb = fuse // fb_bits
        fuse %= fb_bits
        if fuse < main_area_bits:
            row = fuse // main_row_bits
            fuse %= main_row_bits
            if fuse < 8 * 9:
                column = fuse // 8
                bit = fuse % 8
            else:
                fuse -= 8 * 9
                column = 9 + fuse // 6
                bit = fuse % 6
            return (
                fb << 13 |
                0 << 12 |
                row << 5 | 
                (column // 5) << 3 |
                (column % 5)
            ), bit
        else:
            fuse -= main_area_bits
            subarea = fuse // uim_subarea_bits
            fuse %= uim_subarea_bits
            row = fuse // uim_row_bits
            fuse %= uim_row_bits
            if fuse < 8:
                column = 0
                bit = fuse
            else:
                fuse -= 8
                column = 1 + fuse // 7
                bit = fuse % 7
            return (
                fb << 13 |
                1 << 12 |
                subarea << 8 |
                row << 3 |
                column
            ), bit

    def jtag_to_jed(addr, bit):
        fb = addr >> 13 & 0xf
        assert fb < device.num_fbs
        if addr & (1 << 12):
            row = addr >> 5 & 0x7f
            assert row < 72
            col_hi = addr >> 3 & 3
            assert col_hi < 3
            col_lo = addr & 7
            assert col_lo < 5
            column = col_hi * 5 + col_lo
            if column < 9:
                cfuse = column * 8 + bit
            else:
                cfuse = 8 * 8 + (column - 9) * 6 + bit
            return fb * fb_bits + row * main_row_bits + cfuse
        else:
            subarea = addr >> 8 & 0xf
            assert subarea < device.num_fbs
            row = addr >> 3 & 0x1f
            assert row < 18
            column = addr & 7
            assert column < 5
            if column == 0:
                cfuse = bit
            else:
                cfuse = 8 + (column - 1) * 7 + bit
            return fb * fb_bits + main_area_bits + subarea * uim_subarea_bits + row * uim_row_bits + cfuse


Fuses — product terms
=====================

The product term masks are stored in bits 0-5 of every column and every row of the main area.
The formulas are as follows:

1. ``FB[i].MC[j].PT[k].IM[l].P`` is stored at:

   - row: ``l * 2 + 1``
   - column: ``k + (j % 3) * 5``
   - bit: ``j // 3``

2. ``FB[i].MC[j].PT[k].IM[l].N`` is stored at:

   - row: ``l * 2``
   - column: ``k + (j % 3) * 5``
   - bit: ``j // 3``


Fuses — macrocells
==================

Per-MC config fuses (that are not product term masks) are stored in bits 6-7 of
columns 0-8 of rows 12-54 of the main area.  The formulas are as follows:

- row: corresponds to fuse function
- column: ``mc_idx % 9``
- bit: ``6 + mc_idx // 9``

The row is:

- 12: ``PT[0].ALLOC`` bit 0
- 13: ``PT[0].ALLOC`` bit 1
- 14: ``PT[1].ALLOC`` bit 0
- 15: ``PT[1].ALLOC`` bit 1
- 16: ``PT[2].ALLOC`` bit 0
- 17: ``PT[2].ALLOC`` bit 1
- 18: ``PT[3].ALLOC`` bit 0
- 19: ``PT[3].ALLOC`` bit 1
- 20: ``PT[4].ALLOC`` bit 0
- 21: ``PT[5].ALLOC`` bit 1
- 22: ``INV``
- 23: ``IMPORT_UP_ALLOC``
- 24: ``IMPORT_DOWN_ALLOC``
- 25: ``EXPORT_DIR``
- 26: ``SUM_HP``
- 27: ``IOB_OE_MUX`` bit 0
- 28: ``IOB_OE_MUX`` bit 1
- 29: ``OE_MUX`` bit 0
- 30: ``OE_MUX`` bit 1
- 31: ``OE_MUX`` bit 2
- 32-34: unused?
- 35: ``OUT_MUX``
- 36: ``CLK_MUX`` bit 0
- 37: ``CLK_MUX`` bit 1
- 38-39: unused
- 40: ``REG_MODE``
- 41: ``RST_MUX``
- 42: ``SET_MUX``
- 43: ``INIT``
- 44: ``UIM_OE_MUX`` bit 0
- 45: ``UIM_OE_MUX`` bit 1
- 46: ``UIM_OUT_INV``
- 47: unused
- 48: ``IOB_GND``
- 49: ``IOB_SLEW``
- 50: ``PT[0].HP``
- 51: ``PT[1].HP``
- 52: ``PT[2].HP``
- 53: ``PT[3].HP``
- 54: ``PT[4].HP``

The fuse combination assignments are:

- ``PT[*].ALLOC``:

  - ``11``: ``NONE``
  - ``10``: ``SUM``
  - ``01``: ``EXPORT``
  - ``00``: ``SPECIAL``

- ``IMPORT_*_ALLOC``:

  - ``1``: ``EXPORT``
  - ``0``: ``SUM``

- ``EXPORT_DIR``:

  - ``1``: ``UP``
  - ``0``: ``DOWN``

- ``IOB_OE_MUX``:

  - ``11``: ``GND``
  - ``10``: ``OE_MUX``
  - ``01``: ``VCC``

- ``OUT_MUX``:

   - ``1``: ``COMB``
   - ``0``: ``FF``

- ``OE_MUX``:

   - ``111``: ``PT``
   - ``110``: ``FOE0``
   - ``101``: ``FOE1``
   - ``100``: ``FOE2``
   - ``011``: ``FOE3``

- ``CLK_MUX``:

  - ``11``: ``FCLK1``
  - ``10``: ``FCLK2``
  - ``01``: ``FCLK0``
  - ``00``: ``PT``

- ``REG_MODE``:

  - ``1``: ``DFF``
  - ``0``: ``TFF``

- ``RST_MUX``, ``SET_MUX``:

  - ``1``: ``PT``
  - ``0``: ``FSR``

- ``UIM_OE_MUX``:

  - ``11``: ``OE_MUX``
  - ``10``: ``GND``
  - ``01``: ``VCC``

- ``IOB_SLEW``:

  - ``1``: ``SLOW``
  - ``0``: ``FAST``

- everything else:

  - ``1``: false
  - ``0``: true


Fuses — FB inputs
=================

The FB input mux configuraton is stored in rows 55-66, columns 0-8, bits 6-7.
The exact bit assignments are irregular and should be obtained from the database.


Fuses — per-FB bits and globals
===============================

Per-FB bits are stored in rows 67-68, columns 0-8, bits 6-7.  The bits are (row, column, bit):

- (67, 0, 6): ``ENABLE``
- (67, 1, 6): ``EXPORT_ENABLE``
- (68, 6, 6): ``PULLUP_DISABLE``

Global bits are stored in rows (0, 3, 4, 6, 7), columns 0-8, bits 6-7 of FB 0.  The bits are (row, column, bit):

- (0, 1, 6): ``FSR_INV``
- (0, 2, 6): ``FCLK[0].INV``
- (0, 3, 6): ``FCLK[1].INV``
- (0, 4, 6): ``FCLK[2].INV``
- (0, 5, 6): ``FOE[0].INV``
- (0, 6, 6): ``FOE[1].INV``
- (0, 7, 6): ``FOE[2].INV``
- (0, 8, 6): ``FOE[3].INV``
- (3, 2, 6): ``FCLK[0].MUX`` bit 0
- (3, 3, 6): ``FCLK[1].MUX`` bit 0
- (3, 4, 6): ``FCLK[2].MUX`` bit 0
- (3, 5, 6): ``FOE[0].MUX`` bit 0
- (3, 6, 6): ``FOE[1].MUX`` bit 0
- (3, 7, 6): ``FOE[2].MUX`` bit 0
- (3, 8, 6): ``FOE[3].MUX`` bit 0
- (4, 2, 6): ``FCLK[0].MUX`` bit 1
- (4, 3, 6): ``FCLK[1].MUX`` bit 1
- (4, 4, 6): ``FCLK[2].MUX`` bit 1
- (4, 5, 6): ``FOE[0].MUX`` bit 1
- (4, 6, 6): ``FOE[1].MUX`` bit 1
- (4, 7, 6): ``FOE[2].MUX`` bit 1
- (4, 8, 6): ``FOE[3].MUX`` bit 1
- (6, i, j) for i < 8, j in (6, 7): ``USERCODE`` bit ``16 + (7 - i) * 2 + (j - 6)``
- (7, i, j) for i < 8, j in (6, 7): ``USERCODE`` bit ``(7 - i) * 2 + (j - 6)``

The fuse combination assignments are:

- ``FCLK[0].MUX``:

  - ``11``: ``NONE``
  - ``10``: ``GCLK[1]``
  - ``01``: ``GCLK[0]``

- ``FCLK[1].MUX``:

  - ``11``: ``NONE``
  - ``10``: ``GCLK[2]``
  - ``01``: ``GCLK[1]``

- ``FCLK[2].MUX``:

  - ``11``: ``NONE``
  - ``10``: ``GCLK[0]``
  - ``01``: ``GCLK[2]``

- ``FOE[0].MUX`` (small device):

  - ``11``: ``NONE``
  - ``10``: ``GOE[1]``
  - ``01``: ``GOE[0]``

- ``FOE[1].MUX`` (small device):

  - ``11``: ``NONE``
  - ``10``: ``GOE[0]``
  - ``01``: ``GOE[1]``

- ``FOE[0].MUX`` (large device):

  - ``11``: ``NONE``
  - ``10``: ``GOE[1]``
  - ``01``: ``GOE[0]``

- ``FOE[1].MUX`` (large device):

  - ``11``: ``NONE``
  - ``10``: ``GOE[2]``
  - ``01``: ``GOE[1]``

- ``FOE[2].MUX`` (large device):

  - ``11``: ``NONE``
  - ``10``: ``GOE[3]``
  - ``01``: ``GOE[2]``

- ``FOE[3].MUX`` (large device):

  - ``11``: ``NONE``
  - ``10``: ``GOE[0]``
  - ``01``: ``GOE[3]``

- ``USERCODE``: each bit stored inverted

- everything else:

  - ``1``: false
  - ``0``: true


Fuses — input buffer enable
===========================

On XC95288, the ``IBUF_UIM_ENABLE`` fuses are stored in rows (1, 2, 5, 8, 9),
columns 0-8, bits 6-7 of FBs (0, 1, 14, 15) in an irregular manner.  Each
fuse is duplicated twice: once in FBs (0, 1) and once in FBs (14, 15).
The purpose of this duplication is unknown.  Consult the database for exact
bit assignments.


Fuses — protection bits
=======================

The protection bits are stored as follows (row, column, bit) in every FB:

- (11, 3, 6): ``READ_PROT_A``
- (68, 3, 6): ``READ_PROT_B``
- (68, 0, 6): ``WRITE_PROT``


Fuses — UIM wire-AND
====================

The ``FB[i].IM[j].UIM.FB[k].MC[l]`` fuse is stored at:

  - subarea: ``k``
  - row: ``l``
  - column: ``j % 5``
  - bit: ``j // 5``
