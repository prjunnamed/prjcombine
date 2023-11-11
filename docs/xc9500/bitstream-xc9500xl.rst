Bitstream structure — XC9500XL/XV
#################################

The main differences from XC9500 are:

1. The UIM wire-AND area is completely gone, only the main areas exist.
2. The main area has 108 rows per FB instead of 72.
3. Unprogrammed fuse state is ``0``, programmed fuse state is ``1``.
   Thus, the sense of every bitstream bit is inverted from the XC9500 version.
4. While in XC9500 all areas are loaded sequentially, in XC9500XL/XV the areas
   are loaded in parallel.  Thus, the JTAG unit is not a byte, but a word of
   size ``8 * num_fbs``.  Likewise, the bytes for each FB are interleaved
   in the JED format.

On a high level, the whole bitstream is split into "areas".  Each FB
of the device corresponds to one area.

Each area is made of 108 "rows".  Each row is made of 15 "columns".
Each column is made of 6 or 8 bits: columns 0-8 are made of 8 bits, while
columns 9-14 are made of 6 bits.

The low 6 bits of every column are used to store product term masks, and
the high 2 bits of columns 0-8 are used to store everything else.

When programmed or read via JTAG, the bitstream is transmitted as words.
Each word is 8 bits per FB.  Each word of the bitstream has its address.
Not all addresses are valid, and valid addresses are not contiguous.
Address is 12 bits long, and is split to several fields:

- bits 5-11: row
- bits 3-4: column / 5
- bits 0-2: column % 5

The unprogrammed state of a bit on XC9500XL/XV is ``0``.
The programmed state is ``1``.  Thus, whenever a boolean fuse is mentioned
in the documentation, the "true" value is actually represented as ``1``
in the bitstream.


JED format mapping
==================

In the JED format, all fuses of the device are simply concatenated in order,
skipping over invalid addresses.  The bytes are *not* padded to 8 bits, but
have their native size.  Thus, converting from JED fuse index to device
address involves some complex calculations::

    row_bits = (8 * 9 + 6 * 6) * device.num_fbs
    total_bits = row_bits * 108

    def jed_to_jtag(fuse):
        row = fuse // row_bits
        fuse %= row_bits
        if fuse < 8 * 9 * device.num_fbs:
            column = fuse // (8 * device.num_fbs)
            fuse %= (8 * device.num_fbs)
            fb = fuse // 8
            bit = fuse % 8
        else:
            fuse -= 8 * 9 * device.num_fbs
            column = 9 + fuse // (6 * device.num_fbs)
            fuse %= (6 * device.num_fbs)
            fb = fuse // 6
            bit = fuse % 6
        return (
            row << 5 | 
            (column // 5) << 3 |
            (column % 5)
        ), (fb * 8 + bit)

    def jtag_to_jed(addr, bit):
        fb = bit // 8
        bit %= 8
        row = addr >> 5 & 0x7f
        assert row < 108
        col_hi = addr >> 3 & 3
        assert col_hi < 3
        col_lo = addr & 7
        assert col_lo < 5
        column = col_hi * 5 + col_lo
        if column < 9:
            cfuse = column * 8 * device.num_fbs + fb * 8 + bit
        else:
            cfuse = 8 * 8 + (column - 9) * 6 * device.num_fbs + fb * 6 + bit
        return row * row_bits + cfuse


Fuses — product terms
=====================

The product term masks are stored in bits 0-5 of every column and every row of the main area.
The formulas are as follows (unchanged from XC9500, but now with more rows):

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
columns 0-8 of rows 12-49 of the main area.  The formulas are as follows:

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
- 27: ``OE_MUX`` bit 0
- 28: ``OE_MUX`` bit 1
- 29: ``OE_MUX`` bit 2
- 30: ``OE_INV``
- 31: unused
- 32: ``OUT_MUX``
- 33: ``CLK_MUX`` bit 0
- 34: ``CLK_MUX`` bit 1
- 35: ``CLK_INV``
- 36: ``CE_MUX`` bit 0
- 37: ``CE_MUX`` bit 1
- 38: unused
- 39: ``REG_MODE``
- 40: ``RST_MUX``
- 41: ``SET_MUX``
- 42: ``INIT``
- 43: ``GND``
- 44: ``SLEW``
- 45: ``PT[0].HP``
- 46: ``PT[1].HP``
- 47: ``PT[2].HP``
- 48: ``PT[3].HP``
- 49: ``PT[4].HP``

The fuse combination assignments are:

- ``PT[*].ALLOC``:

  - ``00``: ``NONE``
  - ``01``: ``OR_MAIN``
  - ``10``: ``OR_EXPORT``
  - ``11``: ``SPECIAL``

- ``IMPORT_*_ALLOC``:

  - ``0``: ``OR_EXPORT``
  - ``1``: ``OR_MAIN``

- ``EXPORT_DIR``:

  - ``0``: ``UP``
  - ``1``: ``DOWN``

- ``OUT_MUX``:

   - ``0``: ``COMB``
   - ``1``: ``FF``

- ``CLK_MUX``:

  - ``00``: ``FCLK1``
  - ``01``: ``FCLK0``
  - ``10``: ``FCLK2``
  - ``11``: ``PT``

- ``CE_MUX``:

  - ``00``: ``NONE``
  - ``01``: ``PT2``
  - ``10``: ``PT3``

- ``REG_MODE``:

  - ``0``: ``DFF``
  - ``1``: ``TFF``

- ``RST_MUX``, ``SET_MUX``:

  - ``0``: ``PT``
  - ``1``: ``FSR``

- ``SLEW``:

  - ``0``: ``SLOW``
  - ``1``: ``FAST``

- everything else:

  - ``0``: false
  - ``1``: true


Fuses — FB inputs
=================

The FB input mux configuraton is stored in rows 50-76, columns 0-8, bits 6-7.
``FB[i].IM[j].MUX`` has 9 bits and is stored at the following coordinates:

- row: ``50 + j % 27``
- column: mux fuse index (0-8)
- bit: 6 if ``j < 27``, 7 otherwise

The exact bit assignments are irregular and should be obtained from the database.


Fuses — per-FB bits and globals
===============================

Per-FB bits are stored in row 78, columns 0-8, bits 6-7.  The bits are (row, column, bit):

- (78, 0, 6): ``ENABLE``
- (78, 1, 6): ``EXPORT_ENABLE``
- (78, 6, 6): ``PULLUP_DISABLE``

Global bits are stored in rows (2, 6, 7), columns 0-8, bits 6-7 of FB 0.  The bits are (row, column, bit):

- (2, 0, 6): ``FSR_INV``
- (2, 1, 6): ``FCLK[0].EN``
- (2, 2, 6): ``FCLK[1].EN``
- (2, 3, 6): ``FCLK[2].EN``
- (2, 4, 6): ``FOE[0].EN``
- (2, 5, 6): ``FOE[1].EN``
- (2, 6, 6): ``FOE[2].EN``
- (2, 7, 6): ``FOE[3].EN``
- (2, 8, 6): ``KEEPER``
- (6, i, j) for i < 8, j in (6, 7): ``USERCODE`` bit ``16 + (7 - i) * 2 + (j - 6)``
- (7, i, j) for i < 8, j in (6, 7): ``USERCODE`` bit ``(7 - i) * 2 + (j - 6)``

The fuse combination assignments are:

- ``USERCODE``: each bit stored directly

- everything else:

  - ``0``: false
  - ``1``: true


Fuses — protection bits, DONE
=============================

The protection bits are stored as follows (row, column, bit) in every FB:

- (11, 0, 6): ``WRITE_PROT``
- (11, 3, 6): ``READ_PROT``

The ``DONE`` bit is stored only once, in FB 0:

- (11, 6, 6): ``DONE``
