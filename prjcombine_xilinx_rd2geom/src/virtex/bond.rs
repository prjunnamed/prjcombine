use prjcombine_rawdump::PkgPin;
use prjcombine_xilinx_geom::pkg::{Bond, BondPin, CfgPin};
use prjcombine_xilinx_geom::virtex::Grid;
use std::collections::{BTreeMap, HashMap};

pub fn make_bond(grid: &Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, (io.coord, io.bank)))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if pad.starts_with("GCLKPAD") {
                let bank = match &pad[..] {
                    "GCLKPAD0" => 4,
                    "GCLKPAD1" => 5,
                    "GCLKPAD2" => 1,
                    "GCLKPAD3" => 0,
                    _ => panic!("unknown pad {}", pad),
                };
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByBank(bank, 0)
            } else {
                let (coord, bank) = io_lookup[pad];
                assert_eq!(pin.vref_bank, Some(bank));
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByCoord(coord)
            }
        } else if pin.func.starts_with("VCCO_") {
            let bank = pin.func[5..].parse().unwrap();
            BondPin::VccO(bank)
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VCCO" => BondPin::VccO(0),
                "TCK" => BondPin::Cfg(CfgPin::Tck),
                "TDI" => BondPin::Cfg(CfgPin::Tdi),
                "TDO" => BondPin::Cfg(CfgPin::Tdo),
                "TMS" => BondPin::Cfg(CfgPin::Tms),
                "CCLK" => BondPin::Cfg(CfgPin::Cclk),
                "DONE" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM" => BondPin::Cfg(CfgPin::ProgB),
                "M0" => BondPin::Cfg(CfgPin::M0),
                "M1" => BondPin::Cfg(CfgPin::M1),
                "M2" => BondPin::Cfg(CfgPin::M2),
                "DXN" => BondPin::Dxn,
                "DXP" => BondPin::Dxp,
                _ => panic!("UNK FUNC {}", pin.func),
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks,
    }
}
