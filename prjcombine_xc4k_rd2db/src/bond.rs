use prjcombine_rawdump::PkgPin;
use prjcombine_xc4k::bond::{Bond, BondPin, CfgPin};
use prjcombine_xc4k::grid::Grid;
use std::collections::{BTreeMap, HashMap};

pub fn make_bond(grid: &Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                BondPin::Io(io)
            } else {
                match &pad[..] {
                    "TDO" => BondPin::Cfg(CfgPin::Tdo),
                    "MD0" => BondPin::Cfg(CfgPin::M0),
                    "MD1" => BondPin::Cfg(CfgPin::M1),
                    "MD2" => BondPin::Cfg(CfgPin::M2),
                    _ => {
                        println!("UNK PAD {}", pad);
                        continue;
                    }
                }
            }
        } else {
            match &pin.func[..] {
                "NC" | "N.C." => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCC" => BondPin::VccO,
                "VCCINT" => BondPin::VccInt,
                "CCLK" => BondPin::Cfg(CfgPin::Cclk),
                "DONE" => BondPin::Cfg(CfgPin::Done),
                "/PROG" | "/PROGRAM" => BondPin::Cfg(CfgPin::ProgB),
                "MODE" | "M0" => BondPin::Cfg(CfgPin::M0),
                "M1" => BondPin::Cfg(CfgPin::M1),
                "M2" => BondPin::Cfg(CfgPin::M2),
                "M2_OPT" => BondPin::Cfg(CfgPin::M2),
                "/PWRDOWN" | "LPWRB" => BondPin::Cfg(CfgPin::PwrdwnB),
                _ => {
                    println!("UNK FUNC {}", pin.func);
                    continue;
                }
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond { pins: bond_pins }
}
