use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_virtex2::bond::{Bond, BondPad, CfgPad, GtPad};

use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rd2db_grid::split_num;
use prjcombine_virtex2::chip::ChipKind;

pub fn make_bond(endev: &ExpandedNamedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let mut vref = BTreeSet::new();
    let io_lookup: HashMap<_, _> = endev
        .chip
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io), io))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if pad.starts_with("PAD") || pad.starts_with("IPAD") || pad.starts_with("CLK") {
                let io = io_lookup[&**pad];
                let info = endev.chip.get_io_info(io);
                if endev.chip.kind != ChipKind::FpgaCore {
                    assert_eq!(pin.vref_bank, Some(info.bank));
                    let old = io_banks.insert(info.bank, pin.vcco_bank.unwrap());
                    assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                    if pin.func.contains("VREF_") {
                        vref.insert(io);
                    }
                } else {
                    assert_eq!(pin.vref_bank, None);
                    assert_eq!(pin.vcco_bank, None);
                }
                BondPad::Io(io)
            } else if let Some((n, b)) = split_num(pad) {
                let pk = match n {
                    "RXPPAD" => GtPad::RxP,
                    "RXNPAD" => GtPad::RxN,
                    "TXPPAD" => GtPad::TxP,
                    "TXNPAD" => GtPad::TxN,
                    _ => panic!("FUNNY PAD {pad}"),
                };
                BondPad::Gt(b, pk)
            } else {
                panic!("FUNNY PAD {pad}");
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPad::Nc,
                "RSVD" => BondPad::Rsvd, // virtex2: likely DXP/DXN
                "GND" => BondPad::Gnd,
                "VCCINT" => BondPad::VccInt,
                "VCCAUX" => BondPad::VccAux,
                "VCCO" => BondPad::VccO(0),
                "VBATT" => BondPad::VccBatt,
                "TCK" => BondPad::Cfg(CfgPad::Tck),
                "TDI" => BondPad::Cfg(CfgPad::Tdi),
                "TDO" => BondPad::Cfg(CfgPad::Tdo),
                "TMS" => BondPad::Cfg(CfgPad::Tms),
                "CCLK" => BondPad::Cfg(CfgPad::Cclk),
                "DONE" => BondPad::Cfg(CfgPad::Done),
                "PROG_B" => BondPad::Cfg(CfgPad::ProgB),
                "M0" => BondPad::Cfg(CfgPad::M0),
                "M1" => BondPad::Cfg(CfgPad::M1),
                "M2" => BondPad::Cfg(CfgPad::M2),
                "HSWAP_EN" => BondPad::Cfg(CfgPad::HswapEn),
                "PWRDWN_B" => BondPad::Cfg(CfgPad::PwrdwnB),
                "SUSPEND" => BondPad::Cfg(CfgPad::Suspend),
                "DXN" => BondPad::Dxn,
                "DXP" => BondPad::Dxp,
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPad::VccO(b),
                            "GNDA" => BondPad::Gt(b, GtPad::GndA),
                            "VTRXPAD" => BondPad::Gt(b, GtPad::VtRx),
                            "VTTXPAD" => BondPad::Gt(b, GtPad::VtTx),
                            "AVCCAUXRX" => BondPad::Gt(b, GtPad::AVccAuxRx),
                            "AVCCAUXTX" => BondPad::Gt(b, GtPad::AVccAuxTx),
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
        vref,
    }
}
