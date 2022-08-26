use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_rawdump::PkgPin;
use prjcombine_xilinx_geom::pkg::{Bond, BondPin, CfgPin, GtPin};
use prjcombine_xilinx_geom::spartan6::Grid;
use prjcombine_xilinx_geom::DisabledPart;

use crate::util::split_num;

pub fn make_bond(grid: &Grid, disabled: &BTreeSet<DisabledPart>, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, (io.coord, io.bank)))
        .collect();
    let gt_lookup: HashMap<_, _> = grid
        .get_gt(disabled)
        .into_iter()
        .flat_map(|gt| {
            gt.get_pads()
                .into_iter()
                .map(move |(name, func, pin, idx)| (name, (func, gt.bank, pin, idx)))
        })
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&(coord, bank)) = io_lookup.get(pad) {
                //assert_eq!(pin.vref_bank, Some(bank));
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByCoord(coord)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
            } else {
                println!("unk iopad {pad} {f}", f = pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VBATT" => BondPin::VccBatt,
                "VFS" => BondPin::Vfs,
                "RFUSE" => BondPin::RFuse,
                "TCK" => BondPin::Cfg(CfgPin::Tck),
                "TDI" => BondPin::Cfg(CfgPin::Tdi),
                "TDO" => BondPin::Cfg(CfgPin::Tdo),
                "TMS" => BondPin::Cfg(CfgPin::Tms),
                "CMPCS_B_2" => BondPin::Cfg(CfgPin::CmpCsB),
                "DONE_2" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM_B_2" => BondPin::Cfg(CfgPin::ProgB),
                "SUSPEND" => BondPin::Cfg(CfgPin::Suspend),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPin::VccO(b),
                            "MGTAVCC_" => BondPin::GtByBank(b, GtPin::AVcc, 0),
                            "MGTAVCCPLL0_" => BondPin::GtByBank(b, GtPin::AVccPll, 0),
                            "MGTAVCCPLL1_" => BondPin::GtByBank(b, GtPin::AVccPll, 1),
                            "MGTAVTTRX_" => BondPin::GtByBank(b, GtPin::VtRx, 0),
                            "MGTAVTTTX_" => BondPin::GtByBank(b, GtPin::VtTx, 0),
                            "MGTRREF_" => BondPin::GtByBank(b, GtPin::RRef, 0),
                            "MGTAVTTRCAL_" => BondPin::GtByBank(b, GtPin::AVttRCal, 0),
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
