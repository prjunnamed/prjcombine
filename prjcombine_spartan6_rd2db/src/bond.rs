use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_rawdump::PkgPin;
use prjcombine_spartan6::bond::{Bond, BondPin, CfgPin, GtPin};

use prjcombine_rdgrid::split_num;
use prjcombine_spartan6_naming::ExpandedNamedDevice;

pub fn make_bond(endev: &ExpandedNamedDevice, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let mut vref = BTreeSet::new();
    let io_lookup: HashMap<_, _> = endev
        .edev
        .io
        .iter()
        .map(|&io| (endev.get_io_name(io), io))
        .collect();
    let mut gt_lookup: HashMap<_, (String, u32, GtPin)> = HashMap::new();
    for gt in endev.get_gts() {
        let bank = gt.bank;
        for (i, &(pp, pn)) in gt.pads_clk.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (format!("MGTREFCLK{i}P_{bank}"), bank, GtPin::ClkP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("MGTREFCLK{i}N_{bank}"), bank, GtPin::ClkN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_rx.iter().enumerate() {
            gt_lookup.insert(pp, (format!("MGTRXP{i}_{bank}"), bank, GtPin::RxP(i as u8)));
            gt_lookup.insert(pn, (format!("MGTRXN{i}_{bank}"), bank, GtPin::RxN(i as u8)));
        }
        for (i, (pp, pn)) in gt.pads_tx.iter().enumerate() {
            gt_lookup.insert(pp, (format!("MGTTXP{i}_{bank}"), bank, GtPin::TxP(i as u8)));
            gt_lookup.insert(pn, (format!("MGTTXN{i}_{bank}"), bank, GtPin::TxN(i as u8)));
        }
    }
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&**pad) {
                //assert_eq!(pin.vref_bank, Some(bank));
                let bank = endev.edev.get_io_bank(io);
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                if pin.func.contains("VREF") {
                    vref.insert(io);
                }
                BondPin::Io(io)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(&**pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::Gt(bank, gpin)
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
                            "MGTAVCC_" => BondPin::Gt(b, GtPin::AVcc),
                            "MGTAVCCPLL0_" => BondPin::Gt(b, GtPin::AVccPll(0)),
                            "MGTAVCCPLL1_" => BondPin::Gt(b, GtPin::AVccPll(1)),
                            "MGTAVTTRX_" => BondPin::Gt(b, GtPin::VtRx),
                            "MGTAVTTTX_" => BondPin::Gt(b, GtPin::VtTx),
                            "MGTRREF_" => BondPin::Gt(b, GtPin::RRef),
                            "MGTAVTTRCAL_" => BondPin::Gt(b, GtPin::AVttRCal),
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
