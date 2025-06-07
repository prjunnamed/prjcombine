use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_spartan6::bond::{Bond, BondPad, CfgPad, GtPad};

use prjcombine_re_xilinx_naming_spartan6::ExpandedNamedDevice;
use prjcombine_re_xilinx_rd2db_grid::split_num;

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
    let mut gt_lookup: HashMap<_, (String, u32, GtPad)> = HashMap::new();
    for gt in endev.get_gts() {
        let bank = gt.bank;
        for (i, &(pp, pn)) in gt.pads_clk.iter().enumerate() {
            gt_lookup.insert(
                pp,
                (format!("MGTREFCLK{i}P_{bank}"), bank, GtPad::ClkP(i as u8)),
            );
            gt_lookup.insert(
                pn,
                (format!("MGTREFCLK{i}N_{bank}"), bank, GtPad::ClkN(i as u8)),
            );
        }
        for (i, (pp, pn)) in gt.pads_rx.iter().enumerate() {
            gt_lookup.insert(pp, (format!("MGTRXP{i}_{bank}"), bank, GtPad::RxP(i as u8)));
            gt_lookup.insert(pn, (format!("MGTRXN{i}_{bank}"), bank, GtPad::RxN(i as u8)));
        }
        for (i, (pp, pn)) in gt.pads_tx.iter().enumerate() {
            gt_lookup.insert(pp, (format!("MGTTXP{i}_{bank}"), bank, GtPad::TxP(i as u8)));
            gt_lookup.insert(pn, (format!("MGTTXN{i}_{bank}"), bank, GtPad::TxN(i as u8)));
        }
    }
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&**pad) {
                //assert_eq!(pin.vref_bank, Some(bank));
                let bank = endev.chip.get_io_bank(io);
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                if pin.func.contains("VREF") {
                    vref.insert(io);
                }
                BondPad::Io(io)
            } else if let Some(&(ref exp_func, bank, gpin)) = gt_lookup.get(&**pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPad::Gt(bank, gpin)
            } else {
                println!("unk iopad {pad} {f}", f = pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPad::Nc,
                "GND" => BondPad::Gnd,
                "VCCINT" => BondPad::VccInt,
                "VCCAUX" => BondPad::VccAux,
                "VBATT" => BondPad::VccBatt,
                "VFS" => BondPad::Vfs,
                "RFUSE" => BondPad::RFuse,
                "TCK" => BondPad::Cfg(CfgPad::Tck),
                "TDI" => BondPad::Cfg(CfgPad::Tdi),
                "TDO" => BondPad::Cfg(CfgPad::Tdo),
                "TMS" => BondPad::Cfg(CfgPad::Tms),
                "CMPCS_B_2" => BondPad::Cfg(CfgPad::CmpCsB),
                "DONE_2" => BondPad::Cfg(CfgPad::Done),
                "PROGRAM_B_2" => BondPad::Cfg(CfgPad::ProgB),
                "SUSPEND" => BondPad::Cfg(CfgPad::Suspend),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPad::VccO(b),
                            "MGTAVCC_" => BondPad::Gt(b, GtPad::AVcc),
                            "MGTAVCCPLL0_" => BondPad::Gt(b, GtPad::AVccPll(0)),
                            "MGTAVCCPLL1_" => BondPad::Gt(b, GtPad::AVccPll(1)),
                            "MGTAVTTRX_" => BondPad::Gt(b, GtPad::VtRx),
                            "MGTAVTTTX_" => BondPad::Gt(b, GtPad::VtTx),
                            "MGTRREF_" => BondPad::Gt(b, GtPad::RRef),
                            "MGTAVTTRCAL_" => BondPad::Gt(b, GtPad::AVttRCal),
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
