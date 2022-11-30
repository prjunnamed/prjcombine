use std::collections::{BTreeMap, HashMap};

use prjcombine_rawdump::PkgPin;
use prjcombine_virtex2::bond::{Bond, BondPin, CfgPin, GtPin};
use prjcombine_virtex2::expanded::ExpandedDevice;

use prjcombine_rdgrid::split_num;

pub fn make_bond(edev: &ExpandedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = edev
        .get_bonded_ios()
        .into_iter()
        .map(|io| (io.name.to_string(), io))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if pad.starts_with("PAD") || pad.starts_with("IPAD") || pad.starts_with("CLK") {
                let io = io_lookup[pad];
                assert_eq!(pin.vref_bank, Some(io.bank));
                let old = io_banks.insert(io.bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::Io(io.coord)
            } else if let Some((n, b)) = split_num(pad) {
                let pk = match n {
                    "RXPPAD" => GtPin::RxP,
                    "RXNPAD" => GtPin::RxN,
                    "TXPPAD" => GtPin::TxP,
                    "TXNPAD" => GtPin::TxN,
                    _ => panic!("FUNNY PAD {}", pad),
                };
                BondPin::Gt(b, pk)
            } else {
                panic!("FUNNY PAD {}", pad);
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "RSVD" => BondPin::Rsvd, // virtex2: likely DXP/DXN
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VCCO" => BondPin::VccO(0),
                "VBATT" => BondPin::VccBatt,
                "TCK" => BondPin::Cfg(CfgPin::Tck),
                "TDI" => BondPin::Cfg(CfgPin::Tdi),
                "TDO" => BondPin::Cfg(CfgPin::Tdo),
                "TMS" => BondPin::Cfg(CfgPin::Tms),
                "CCLK" => BondPin::Cfg(CfgPin::Cclk),
                "DONE" => BondPin::Cfg(CfgPin::Done),
                "PROG_B" => BondPin::Cfg(CfgPin::ProgB),
                "M0" => BondPin::Cfg(CfgPin::M0),
                "M1" => BondPin::Cfg(CfgPin::M1),
                "M2" => BondPin::Cfg(CfgPin::M2),
                "HSWAP_EN" => BondPin::Cfg(CfgPin::HswapEn),
                "PWRDWN_B" => BondPin::Cfg(CfgPin::PwrdwnB),
                "SUSPEND" => BondPin::Cfg(CfgPin::Suspend),
                "DXN" => BondPin::Dxn,
                "DXP" => BondPin::Dxp,
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPin::VccO(b),
                            "GNDA" => BondPin::Gt(b, GtPin::GndA),
                            "VTRXPAD" => BondPin::Gt(b, GtPin::VtRx),
                            "VTTXPAD" => BondPin::Gt(b, GtPin::VtTx),
                            "AVCCAUXRX" => BondPin::Gt(b, GtPin::AVccAuxRx),
                            "AVCCAUXTX" => BondPin::Gt(b, GtPin::AVccAuxTx),
                            _ => {
                                println!("UNK FUNC {}", pin.func);
                                continue;
                            }
                        }
                    } else {
                        println!("UNK FUNC {}", pin.func);
                        continue;
                    }
                }
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks,
    }
}
