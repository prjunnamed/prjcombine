Devices
#######

The following devices implement the Digilent Adept protocol:

- Digilent Basys 2 board (Spartan 3E FPGA), AT90USB162-based

  - product name: ``Digilent Basys2-100`` (xc3s100e)
  - product id: ``0x008``
  - variant id: ``0x001`` (xc3s100e)
  - firmware id: ``0x22`` (unclear if applies to all boards)
  - DMGT config reset and query DONE capabilities
  - one DJTG port

    - JTAG chain consists of the FPGA and the Xilinx serial flash device
    - supports set speed; supported frequencies are:

      - 4MHz
      - 2MHz
      - 1MHz
      - 500kHz
      - 250kHz
      - 125kHz
      - 62.5kHz

  - one DEPP port

- SiliconBlue iCEblink40-HX1K board (iCE40 FPGA), AT90USB162-based

  - product name: ``SiliconBlue iCE40 Eval Board``
  - product id: ``0xf04``
  - variant id: ``0x001`` (iCE40HX1K)
  - firmware id: ``0x2e`` (unclear if applies to all boards)
  - DMGT power control capability
  - one DPIO port

    - pin 0 (bidirectional): power supply enable (controls power supply to the rest of the board, has weak pullup)
    - pin 1 (bidirectional): ``CDONE`` FPGA pin

  - one DEPP port
  - one DSPI port

    - can be used to program the SPI flash
    - cannot be used to directly program the FPGA
    - supports set speed; supported frequencies are:

      - 4MHz
      - 2MHz
      - 1MHz
      - 500kHz
      - 250kHz
      - 125kHz
      - 62.5kHz

    - supports all SPI modes
    - supports inter-byte delay
    - supports both shift directions

- Coolrunner II CPLD Starter Board, AT90USB162-based

  - product name: ``CoolRunner 2 Starter 2``
  - product id: ``0x009``
  - variant id: ``0x001`` (XC2C256)
  - firmware id: ``0x26``
  - DMGT power control capability
  - one DJTG port

    - JTAG chain consists of the CPLD only
    - supports set speed; supported frequencies are:

      - 4MHz
      - 2MHz
      - 1MHz
      - 500kHz
      - 250kHz
      - 125kHz
      - 62.5kHz

  - one DEPP port
  - one DSPI port (the port is not, in fact, connected to anything)

.. todo:: list very incomplete
