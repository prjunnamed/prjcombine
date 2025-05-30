# iCE65 and iCE40

The iCE65 and iCE40 are series of small FPGAs originally designed by SiliconBlue, later acquired by Lattice.

There's a significant amount of variation between devices in these series, so each device is represented as a separate database.

The iCE65 series is based on a 65nm process, and includes the following devices:

- iCE65L04, the original one in the series
- iCE65P04, a variant of L04 with a PLL
- iCE65L08, a larger L04
- iCE65L01, a smaller L04, does not include the special west IO bank functions

The iCE40 series is based on a 40nm process.  The original series (codenamed Los Angeles) includes the following devices:

- iCE40P01, essentially a die shrink of iCE65L01 with some new functions added, sold as
  - iCE40LP1K (low power version)
  - iCE40HX1K (high performance version)
  - iCE40LP640 (low power version, software-limitted to 640 LUTs)
- iCE40P03, a very cut down P01 with no BRAM nor PLLs, sold as
  - iCE40LP384
- iCE40P08, mostly just a larger P01, sold as
  - iCE40LP8K (low power version)
  - iCE40HX8K (high performance version)
  - iCE40LP4K (low power version, software-limitted to 4k LUTs)
  - iCE40HX4K (high performance version, software-limitted to 4k LUTs)

There was an unreleased series named iCE40MX (codename San Francisco) which added hard MIPI D-PHY and TMDS transceivers, larger BRAMs, and DSP units.  While support for it is present in vendor tools, it is incredibly buggy, and so is not (currently) included in the database.  The devices in this series are:

- iCE40M08 (aka iCE40MX8), includes hard MIPI D-PHY transceivers
- iCE40M16 (aka iCE40MX16), includes hard MIPI D-PHY transceivers, TMDS receivers, 16-kbit BRAMs, DSP units

The first series made after Lattice acquisition is iCE40LM (codenamed Lightning). It includes new SPI and I2C controller hard IP and internal oscillators.  There was only one device in this series:

- iCE40R04, sold as
  - iCE40LM4K
  - iCE40LM2K (software-limitted to 2k LUTs)
  - iCE40LM1K (software-limitted to 1k LUTs)

This was followed by the iCE40 Ultra series (codenamed Thunder), which adds new high-current RGB and IR LED drivers, and brings back the (previously unreleased) DSP units from iCE40MX16.  There was only one device in this series:

- iCE40T04, sold as
  - iCE5LP4K (do not ask me why they called it iCE5; I have absolutely no idea beyond "marketing should be doing less drugs, or at least different drugs")
  - iCE5LP2K (software-limitted to 2k LUTs)
  - iCE5LP1K (software-limitted to 1k LUTs)

The next series was iCE40 UltraPlus (codenamed ThunderPlus), with minor improvements in the hard IP, and new 256-kbit single-port RAM blocks.  There was only one device in this series:

- iCE40T05, sold as
  - iCE40UP5K
  - iCE40UP3K (software-limitted to 3k LUTs)

The final iCE40 series was iCE40 UltraLite (codenamed Bolt or ThunderBolt), including a new (different) I2C hard controller IP and a new IR/barcode LED driver.  There was, again, only one device in this series:

- iCE40T01, sold as
  - iCE40UL1K
  - iCE40UL640 (software-limitted to 640 LUTs)