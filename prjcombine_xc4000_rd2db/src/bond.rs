use prjcombine_rawdump::PkgPin;
use prjcombine_xc4000::bond::{Bond, BondPin, CfgPin};
use prjcombine_xc4000::grid::{GridKind, IoCoord, SharedCfgPin};
use prjcombine_xc4000_naming::ExpandedNamedDevice;
use std::collections::{btree_map, BTreeMap, HashMap};

pub fn make_bond(
    endev: &ExpandedNamedDevice,
    pkg: &str,
    pins: &[PkgPin],
    cfg_io: &mut BTreeMap<SharedCfgPin, IoCoord>,
) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = endev
        .edev
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io), io))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&**pad) {
                BondPin::Io(io)
            } else {
                match &pad[..] {
                    "TDO" => BondPin::Cfg(CfgPin::Tdo),
                    "MD0" => BondPin::Cfg(CfgPin::M0),
                    "MD1" => BondPin::Cfg(CfgPin::M1),
                    "MD2" => BondPin::Cfg(CfgPin::M2),
                    _ => {
                        println!("UNK PAD {pad}");
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
        // ???
        let pname = pin.pin.strip_suffix('*').unwrap_or(&pin.pin[..]);
        bond_pins.insert(pname.to_string(), bpin);
    }

    let mut pkg_cfg_io = vec![];
    match pkg {
        "pc84" => {
            pkg_cfg_io.extend([
                ("P3", SharedCfgPin::Addr(8)),
                ("P4", SharedCfgPin::Addr(9)),
                ("P5", SharedCfgPin::Addr(10)),
                ("P6", SharedCfgPin::Addr(11)),
                ("P7", SharedCfgPin::Addr(12)),
                ("P8", SharedCfgPin::Addr(13)),
                ("P9", SharedCfgPin::Addr(14)),
                ("P10", SharedCfgPin::Addr(15)),
                ("P13", SharedCfgPin::Addr(16)),
                ("P14", SharedCfgPin::Addr(17)),
                ("P15", SharedCfgPin::Tdi),
                ("P16", SharedCfgPin::Tck),
                ("P17", SharedCfgPin::Tms),
                ("P36", SharedCfgPin::Hdc),
                ("P37", SharedCfgPin::Ldc),
                ("P41", SharedCfgPin::InitB),
                ("P56", SharedCfgPin::Data(7)),
                ("P58", SharedCfgPin::Data(6)),
                ("P59", SharedCfgPin::Data(5)),
                ("P60", SharedCfgPin::Cs0B),
                ("P61", SharedCfgPin::Data(4)),
                ("P65", SharedCfgPin::Data(3)),
                ("P66", SharedCfgPin::RsB),
                ("P67", SharedCfgPin::Data(2)),
                ("P69", SharedCfgPin::Data(1)),
                ("P70", SharedCfgPin::BusyB),
                ("P71", SharedCfgPin::Data(0)),
                ("P72", SharedCfgPin::Dout),
                ("P77", SharedCfgPin::Addr(0)),
                ("P78", SharedCfgPin::Addr(1)),
                ("P79", SharedCfgPin::Addr(2)),
                ("P80", SharedCfgPin::Addr(3)),
                ("P81", SharedCfgPin::Addr(4)),
                ("P82", SharedCfgPin::Addr(5)),
                ("P83", SharedCfgPin::Addr(6)),
                ("P84", SharedCfgPin::Addr(7)),
            ]);
        }
        "vq100" => {
            pkg_cfg_io.extend([
                ("P28", SharedCfgPin::Hdc),
                ("P30", SharedCfgPin::Ldc),
                ("P36", SharedCfgPin::InitB),
                ("P53", SharedCfgPin::Data(7)),
                ("P55", SharedCfgPin::Data(6)),
                ("P57", SharedCfgPin::Data(5)),
                ("P58", SharedCfgPin::Cs0B),
                ("P61", SharedCfgPin::Data(4)),
                ("P65", SharedCfgPin::Data(3)),
                ("P66", SharedCfgPin::RsB),
                ("P68", SharedCfgPin::Data(2)),
                ("P70", SharedCfgPin::Data(1)),
                ("P71", SharedCfgPin::BusyB),
                ("P72", SharedCfgPin::Data(0)),
                ("P73", SharedCfgPin::Dout),
                ("P78", SharedCfgPin::Addr(0)),
                ("P79", SharedCfgPin::Addr(1)),
                ("P80", SharedCfgPin::Addr(2)),
                ("P81", SharedCfgPin::Addr(3)),
                ("P82", SharedCfgPin::Addr(4)),
                ("P83", SharedCfgPin::Addr(5)),
                ("P86", SharedCfgPin::Addr(6)),
                ("P87", SharedCfgPin::Addr(7)),
                ("P90", SharedCfgPin::Addr(8)),
                ("P91", SharedCfgPin::Addr(9)),
                ("P94", SharedCfgPin::Addr(10)),
                ("P95", SharedCfgPin::Addr(11)),
                ("P96", SharedCfgPin::Addr(12)),
                ("P97", SharedCfgPin::Addr(13)),
                ("P98", SharedCfgPin::Addr(14)),
                ("P99", SharedCfgPin::Addr(15)),
                ("P2", SharedCfgPin::Addr(16)),
                ("P3", SharedCfgPin::Addr(17)),
            ]);
        }
        "pq160" | "hq160" => {
            pkg_cfg_io.extend([
                ("P143", SharedCfgPin::Addr(8)),
                ("P144", SharedCfgPin::Addr(9)),
                ("P147", SharedCfgPin::Addr(10)),
                ("P148", SharedCfgPin::Addr(11)),
                ("P154", SharedCfgPin::Addr(12)),
                ("P155", SharedCfgPin::Addr(13)),
                ("P158", SharedCfgPin::Addr(14)),
                ("P159", SharedCfgPin::Addr(15)),
                ("P2", SharedCfgPin::Addr(16)),
                ("P3", SharedCfgPin::Addr(17)),
                ("P146", SharedCfgPin::Addr(18)),
                ("P145", SharedCfgPin::Addr(19)),
                ("P138", SharedCfgPin::Addr(20)),
                ("P137", SharedCfgPin::Addr(21)),
                ("P6", SharedCfgPin::Tdi),
                ("P7", SharedCfgPin::Tck),
                ("P13", SharedCfgPin::Tms),
                ("P44", SharedCfgPin::Hdc),
                ("P48", SharedCfgPin::Ldc),
                ("P59", SharedCfgPin::InitB),
                ("P83", SharedCfgPin::Data(7)),
                ("P87", SharedCfgPin::Data(6)),
                ("P94", SharedCfgPin::Data(5)),
                ("P95", SharedCfgPin::Cs0B),
                ("P98", SharedCfgPin::Data(4)),
                ("P102", SharedCfgPin::Data(3)),
                ("P103", SharedCfgPin::RsB),
                ("P106", SharedCfgPin::Data(2)),
                ("P113", SharedCfgPin::Data(1)),
                ("P114", SharedCfgPin::BusyB),
                ("P117", SharedCfgPin::Data(0)),
                ("P118", SharedCfgPin::Dout),
                ("P123", SharedCfgPin::Addr(0)),
                ("P124", SharedCfgPin::Addr(1)),
                ("P127", SharedCfgPin::Addr(2)),
                ("P128", SharedCfgPin::Addr(3)),
                ("P134", SharedCfgPin::Addr(4)),
                ("P135", SharedCfgPin::Addr(5)),
                ("P139", SharedCfgPin::Addr(6)),
                ("P140", SharedCfgPin::Addr(7)),
            ]);
        }
        "pq240" | "hq240" => {
            pkg_cfg_io.extend([
                ("P213", SharedCfgPin::Addr(8)),
                ("P214", SharedCfgPin::Addr(9)),
                ("P220", SharedCfgPin::Addr(10)),
                ("P221", SharedCfgPin::Addr(11)),
                ("P232", SharedCfgPin::Addr(12)),
                ("P233", SharedCfgPin::Addr(13)),
                ("P238", SharedCfgPin::Addr(14)),
                ("P239", SharedCfgPin::Addr(15)),
                ("P2", SharedCfgPin::Addr(16)),
                ("P3", SharedCfgPin::Addr(17)),
                ("P216", SharedCfgPin::Addr(18)),
                ("P215", SharedCfgPin::Addr(19)),
                ("P208", SharedCfgPin::Addr(20)),
                ("P207", SharedCfgPin::Addr(21)),
                ("P64", SharedCfgPin::Hdc),
                ("P68", SharedCfgPin::Ldc),
                ("P89", SharedCfgPin::InitB),
                ("P123", SharedCfgPin::Data(7)),
                ("P129", SharedCfgPin::Data(6)),
                ("P141", SharedCfgPin::Data(5)),
                ("P142", SharedCfgPin::Cs0B),
                ("P148", SharedCfgPin::Data(4)),
                ("P152", SharedCfgPin::Data(3)),
                ("P153", SharedCfgPin::RsB),
                ("P159", SharedCfgPin::Data(2)),
                ("P173", SharedCfgPin::Data(1)),
                ("P174", SharedCfgPin::BusyB),
                ("P177", SharedCfgPin::Data(0)),
                ("P178", SharedCfgPin::Dout),
                ("P183", SharedCfgPin::Addr(0)),
                ("P184", SharedCfgPin::Addr(1)),
                ("P187", SharedCfgPin::Addr(2)),
                ("P188", SharedCfgPin::Addr(3)),
                ("P202", SharedCfgPin::Addr(4)),
                ("P203", SharedCfgPin::Addr(5)),
                ("P209", SharedCfgPin::Addr(6)),
                ("P210", SharedCfgPin::Addr(7)),
            ]);
        }
        "bg432" => {
            pkg_cfg_io.extend([
                ("D17", SharedCfgPin::Addr(8)),
                ("A17", SharedCfgPin::Addr(9)),
                ("B19", SharedCfgPin::Addr(10)),
                ("C19", SharedCfgPin::Addr(11)),
                ("D24", SharedCfgPin::Addr(12)),
                ("B26", SharedCfgPin::Addr(13)),
                ("C28", SharedCfgPin::Addr(14)),
                ("D28", SharedCfgPin::Addr(15)),
                ("D29", SharedCfgPin::Addr(16)),
                ("C30", SharedCfgPin::Addr(17)),
                ("D18", SharedCfgPin::Addr(18)),
                ("C18", SharedCfgPin::Addr(19)),
                ("B15", SharedCfgPin::Addr(20)),
                ("C15", SharedCfgPin::Addr(21)),
                ("AH27", SharedCfgPin::Hdc),
                ("AH26", SharedCfgPin::Ldc),
                ("AK16", SharedCfgPin::InitB),
                ("AJ2", SharedCfgPin::Data(7)),
                ("AF1", SharedCfgPin::Data(6)),
                ("AA2", SharedCfgPin::Data(5)),
                ("Y2", SharedCfgPin::Cs0B),
                ("T1", SharedCfgPin::Data(4)),
                ("T3", SharedCfgPin::Data(3)),
                ("R1", SharedCfgPin::RsB),
                ("L2", SharedCfgPin::Data(2)),
                ("G4", SharedCfgPin::Data(1)),
                ("F2", SharedCfgPin::BusyB),
                ("C2", SharedCfgPin::Data(0)),
                ("D3", SharedCfgPin::Dout),
                ("B3", SharedCfgPin::Addr(0)),
                ("D5", SharedCfgPin::Addr(1)),
                ("A5", SharedCfgPin::Addr(2)),
                ("D7", SharedCfgPin::Addr(3)),
                ("C14", SharedCfgPin::Addr(4)),
                ("A13", SharedCfgPin::Addr(5)),
                ("B16", SharedCfgPin::Addr(6)),
                ("A16", SharedCfgPin::Addr(7)),
            ]);
        }
        _ => (),
    }
    for (pin, io) in pkg_cfg_io {
        let pad = bond_pins[pin];
        let BondPin::Io(crd) = pad else {
            unreachable!()
        };
        if matches!(io, SharedCfgPin::Addr(18..=21))
            && !matches!(
                endev.grid.kind,
                GridKind::Xc4000Ex | GridKind::Xc4000Xla | GridKind::Xc4000Xv
            )
        {
            continue;
        }
        if matches!(
            io,
            SharedCfgPin::Addr(0 | 1 | 3..)
                | SharedCfgPin::RsB
                | SharedCfgPin::BusyB
                | SharedCfgPin::Cs0B
        ) && endev.grid.kind == GridKind::SpartanXl
        {
            continue;
        }
        match cfg_io.entry(io) {
            btree_map::Entry::Vacant(e) => {
                e.insert(crd);
            }
            btree_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), crd);
            }
        }
    }

    Bond { pins: bond_pins }
}
