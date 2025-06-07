use prjcombine_re_xilinx_naming_virtex::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_virtex::bond::{Bond, BondPad, CfgPad};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub fn make_bond(endev: &ExpandedNamedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let mut vref = BTreeSet::new();
    let mut diffp = BTreeSet::new();
    let mut diffn = BTreeSet::new();
    let io_lookup: HashMap<_, _> = endev
        .grid
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
                BondPad::Clk(bank)
            } else {
                let io = io_lookup[&**pad];
                let bank = endev.grid.get_io_bank(io);
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
                BondPad::Io(io)
            }
        } else if pin.func.starts_with("VCCO_") {
            let bank = pin.func[5..].parse().unwrap();
            BondPad::VccO(bank)
        } else {
            match &pin.func[..] {
                "NC" => BondPad::Nc,
                "GND" => BondPad::Gnd,
                "VCCINT" => BondPad::VccInt,
                "VCCAUX" => BondPad::VccAux,
                "VCCO" => BondPad::VccO(0),
                "TCK" => BondPad::Cfg(CfgPad::Tck),
                "TDI" => BondPad::Cfg(CfgPad::Tdi),
                "TDO" => BondPad::Cfg(CfgPad::Tdo),
                "TMS" => BondPad::Cfg(CfgPad::Tms),
                "CCLK" => BondPad::Cfg(CfgPad::Cclk),
                "DONE" => BondPad::Cfg(CfgPad::Done),
                "PROGRAM" => BondPad::Cfg(CfgPad::ProgB),
                "M0" => BondPad::Cfg(CfgPad::M0),
                "M1" => BondPad::Cfg(CfgPad::M1),
                "M2" => BondPad::Cfg(CfgPad::M2),
                "DXN" => BondPad::Dxn,
                "DXP" => BondPad::Dxp,
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
