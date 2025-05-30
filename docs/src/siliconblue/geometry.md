# Device geometry

The SiliconBlue devices up to iCE40R04 largely follow the classic column-based FPGA structure, with IO surrounding the device:

- every cell in the westernmost column (except for corners) contains an `IOI_W` tile and an `IOB_W` tile
- every cell in the easternmost column (except for corners) contains an `IOI_E` tile and an `IOB_E` tile
- every cell in the southmost row (except for corners) contains an `IOI_S` tile and an `IOB_S` tile
- every cell in the northmost row (except for corners) contains an `IOI_N` tile and an `IOB_N` tile
- each of the `IOI_*` tiles contains two IO bels (with I/O registers); each of the `IOB_*` tiles contains two corresponding IOB bels
- there are 5 IO banks in the device:
  - bank 0 (`IOB_N` tiles)
  - bank 1 (`IOB_E` tiles)
  - bank 2 (`IOB_S` tiles except the ones included in SPI bank)
  - bank 3 (`IOB_W` tiles)
  - SPI bank (known as bank 4 in Project Combine), containing just the 2 `IOB_S` tiles involved in the SPI configuration interface
- the corner cells are empty
- two columns of the device are designated as BRAM columns
  - they are located at around ¼ and ¾ of the device
  - every cell of the column (excluding the southmost and northmost ones) contains an `INT_BRAM` tile
  - there is a `BRAM` cell for every two `INT_BRAM` cells, with a single block RAM primitive
  - the iCE40P03 device is an exception; it does not have any BRAM columns
- all remaining cells contain one `PLB` tile
  - each `PLB` tile contains 8 "logic cells"
  - a "logic cell" consists of a LUT4, a register, and a carry primitive
- PLLs and other "miscellaneous" bels have irregular placement, are largely outside of the tile structure
  - input interconnect is borrowed from IO tiles, which have one extra input multiplexer each
  - output interconnect is connected at the corners, which otherwise have no interconnect at all

The new iCE40R04 hard IP was added in a rather cursed way: instead of integrating it into the device structure in any ordinary way, it was added at the west and east edges of the device, replacing the I/O buffers (`IOB_W` and `IOB_E` tiles), while reusing the IO tiles as interconnect.  The IO logic part of the tiles is still present (including the IO registers), and has to be configured in "bypass" combinational mode to actually make use of the hard IP.  IO banks 1 and 3 are thus entirely gone, replaced by the hard IP.

The iCE40T0\* series does different, but similar crimes:

- the `IOI_W` and `IOI_E` tiles are gone, replaced by more `PLB` tiles
- the `PLB` cells in the westernmost and easternmost columns are, however, special
  - the input multiplexers to the LUTs are used as inputs to hard IP blocks (thus likely making them unusable as actual LUT inputs)
  - the "cascade" inputs to the LUTs are not connected to the previous LUT as usual; instead, they are connected to the outputs of the hard IP blocks
  - to use the hard IP block outputs, the relevant LUTs should thus be configured to a "bypass" configuration from the cascade inputs
  - on the `iCE40T01` device, only the three southmost and three northmost `PLB` tiles in these columns are special; the rest have normal cascade connections and no associated hard IP
- the corner cells are still empty
- there are 3 IO banks in the device:
  - bank 0 (`IOB_N` tiles)
  - bank 1 (eastern `IOB_S` tiles; includes the SPI configuration interface)
  - bank 2 (western `IOB_S` tiles)


## Bitstream geometry

The bitstream is split into two areas: the BRAM area, containing BRAM initialization data, and the main area, containing everything else.

Both areas are split into 4 banks (unrelated to the IO banks):

- bank 0: southwest quadrant of the device
- bank 1: northwest quadrant
- bank 2: southeast quadrant
- bank 3: northeast quadrant

The boundary between west and east quadrants of the device goes through the exact middle of the device.  The boundary between south and north quadrants varies, and can be quite far from the geometric center.  It is stored in the database as the `row_mid` field.

Every bank is a 2D array of configuration bits:

- each bank is divided into many "frames" of equal lengths
- for the main area:
  - each row corresponds to exactly 16 frames
  - the rows and frames are enumerated starting from the outside and going towards the middle
  - the frame numbers in the database are always counted from the south
    - for the southern quadrants, the numbering matches frame order within the bitstream
    - for the northern quadrants, the numbering is reversed wrt the bitstream
  - each column corresponds to some amount of bits within a frame:
    - an `IOI_W` or `IOI_E` column corresponds to 18 bits
    - a BRAM column corresponds to 42 bits
    - a PLB column corresponds to 54 bits
  - the columns and bits within columns are likewise enumerated starting from the outside and going towards the middle
  - the bit numbers in the database are always counted from the west
    - for the western quadrants, the numbering matches bit order within the frame
    - for the eastern quadrants, the numbering is reversed wrt the bitstream order
  - in addition to all the ordinary columns, there are two extra bits at the end of every frame, containing configuration for the global clocks and the iCE65 PLL
    - the two 2×16 bittiles at the northeast corner of bank 0 and southeast corner of bank 1 configure global clocks; they are represented as the `GB_ROOT` tile in the database
    - on the iCE65P04 only, the two 2×16 bittiles at southeast corner of bank 0 and southwest corner of bank 2 configure the PLL
- for the BRAM area:
  - there are exactly `0x100` frames
  - each frame consists of 16 bits for each `BRAM` tile within the quadrant, starting from the south
  - the bits in the database are always in bitstream order
