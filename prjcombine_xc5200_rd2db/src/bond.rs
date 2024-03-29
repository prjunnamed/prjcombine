use prjcombine_rawdump::PkgPin;
use prjcombine_xc5200::bond::{Bond, BondPin};
use prjcombine_xc5200::expanded::ExpandedDevice;
use std::collections::{BTreeMap, HashMap};

pub fn make_bond(edev: &ExpandedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = edev
        .get_bonded_ios()
        .into_iter()
        .map(|io| (io.name.to_string(), io))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                BondPin::Io(io.coord)
            } else {
                println!("UNK PAD {pad}");
                continue;
            }
        } else {
            println!("UNK FUNC {}", pin.func);
            continue;
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond { pins: bond_pins }
}
