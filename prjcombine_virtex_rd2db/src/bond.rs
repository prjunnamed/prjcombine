use prjcombine_rawdump::PkgPin;
use prjcombine_virtex::bond::{Bond, BondPin, CfgPin};
use prjcombine_virtex_naming::ExpandedNamedDevice;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub fn make_bond(endev: &ExpandedNamedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let mut vref = BTreeSet::new();
    let mut diffp = BTreeSet::new();
    let mut diffn = BTreeSet::new();
    let io_lookup: HashMap<_, _> = endev
        .edev
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io), io))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if pad.starts_with("GCLKPAD") {
                let bank = match &pad[..] {
                    "GCLKPAD0" => 4,
                    "GCLKPAD1" => 5,
                    "GCLKPAD2" => 1,
                    "GCLKPAD3" => 0,
                    _ => panic!("unknown pad {pad}"),
                };
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::Clk(bank)
            } else {
                let io = io_lookup[&**pad];
                let bank = endev.edev.get_io_bank(io);
                assert_eq!(pin.vref_bank, Some(bank));
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                if pin.func.starts_with("IO_VREF_") {
                    vref.insert(io);
                }
                if let Some(pos) = pin.func.find("_L") {
                    let diff = &pin.func[pos..];
                    if diff.contains('P') {
                        diffp.insert(io);
                    }
                    if diff.contains('N') {
                        diffn.insert(io);
                    }
                }
                BondPin::Io(io)
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
        vref,
        diffp,
        diffn,
    }
}
