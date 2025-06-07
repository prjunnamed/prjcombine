use prjcombine_interconnect::grid::EdgeIoCoord;
use prjcombine_re_xilinx_naming_xc2000::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_xc2000::{
    bond::{Bond, BondPad, CfgPad},
    chip::{ChipKind, SharedCfgPad},
};
use std::collections::{BTreeMap, HashMap, btree_map};

pub fn make_bond(
    endev: &ExpandedNamedDevice,
    pkg: &str,
    pins: &[PkgPin],
    cfg_io: &mut BTreeMap<SharedCfgPad, EdgeIoCoord>,
) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = endev
        .chip
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io), io))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&**pad) {
                BondPad::Io(io)
            } else {
                match &pad[..] {
                    "TDO" => BondPad::Cfg(CfgPad::Tdo),
                    "MD0" => BondPad::Cfg(CfgPad::M0),
                    "MD1" => BondPad::Cfg(CfgPad::M1),
                    "MD2" => BondPad::Cfg(CfgPad::M2),
                    _ => {
                        println!("UNK PAD {pad}");
                        continue;
                    }
                }
            }
        } else {
            match &pin.func[..] {
                "NC" | "N.C." => BondPad::Nc,
                "GND" => BondPad::Gnd,
                "VCC" => BondPad::Vcc,
                "VCCINT" => BondPad::VccInt,
                "CCLK" => BondPad::Cfg(CfgPad::Cclk),
                "DONE" => BondPad::Cfg(CfgPad::Done),
                "/PROG" | "/PROGRAM" => BondPad::Cfg(CfgPad::ProgB),
                "MODE" | "M0" => BondPad::Cfg(CfgPad::M0),
                "M1" => BondPad::Cfg(CfgPad::M1),
                "M2" => BondPad::Cfg(CfgPad::M2),
                "M2_OPT" => BondPad::Cfg(CfgPad::M2),
                "/PWRDOWN" | "LPWRB" => BondPad::Cfg(CfgPad::PwrdwnB),
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
                ("P3", SharedCfgPad::Addr(8)),
                ("P4", SharedCfgPad::Addr(9)),
                ("P5", SharedCfgPad::Addr(10)),
                ("P6", SharedCfgPad::Addr(11)),
                ("P7", SharedCfgPad::Addr(12)),
                ("P8", SharedCfgPad::Addr(13)),
                ("P9", SharedCfgPad::Addr(14)),
                ("P10", SharedCfgPad::Addr(15)),
                ("P13", SharedCfgPad::Addr(16)),
                ("P14", SharedCfgPad::Addr(17)),
                ("P15", SharedCfgPad::Tdi),
                ("P16", SharedCfgPad::Tck),
                ("P17", SharedCfgPad::Tms),
                ("P36", SharedCfgPad::Hdc),
                ("P37", SharedCfgPad::Ldc),
                ("P41", SharedCfgPad::InitB),
                ("P56", SharedCfgPad::Data(7)),
                ("P58", SharedCfgPad::Data(6)),
                ("P59", SharedCfgPad::Data(5)),
                ("P60", SharedCfgPad::Cs0B),
                ("P61", SharedCfgPad::Data(4)),
                ("P65", SharedCfgPad::Data(3)),
                ("P66", SharedCfgPad::Cs1B),
                ("P67", SharedCfgPad::Data(2)),
                ("P69", SharedCfgPad::Data(1)),
                ("P70", SharedCfgPad::RclkB),
                ("P71", SharedCfgPad::Data(0)),
                ("P72", SharedCfgPad::Dout),
                ("P77", SharedCfgPad::Addr(0)),
                ("P78", SharedCfgPad::Addr(1)),
                ("P79", SharedCfgPad::Addr(2)),
                ("P80", SharedCfgPad::Addr(3)),
                ("P81", SharedCfgPad::Addr(4)),
                ("P82", SharedCfgPad::Addr(5)),
                ("P83", SharedCfgPad::Addr(6)),
                ("P84", SharedCfgPad::Addr(7)),
            ]);
        }
        "vq100" => {
            pkg_cfg_io.extend([
                ("P28", SharedCfgPad::Hdc),
                ("P30", SharedCfgPad::Ldc),
                ("P36", SharedCfgPad::InitB),
                ("P53", SharedCfgPad::Data(7)),
                ("P55", SharedCfgPad::Data(6)),
                ("P57", SharedCfgPad::Data(5)),
                ("P58", SharedCfgPad::Cs0B),
                ("P61", SharedCfgPad::Data(4)),
                ("P65", SharedCfgPad::Data(3)),
                ("P66", SharedCfgPad::Cs1B),
                ("P68", SharedCfgPad::Data(2)),
                ("P70", SharedCfgPad::Data(1)),
                ("P71", SharedCfgPad::RclkB),
                ("P72", SharedCfgPad::Data(0)),
                ("P73", SharedCfgPad::Dout),
                ("P78", SharedCfgPad::Addr(0)),
                ("P79", SharedCfgPad::Addr(1)),
                ("P80", SharedCfgPad::Addr(2)),
                ("P81", SharedCfgPad::Addr(3)),
                ("P82", SharedCfgPad::Addr(4)),
                ("P83", SharedCfgPad::Addr(5)),
                ("P86", SharedCfgPad::Addr(6)),
                ("P87", SharedCfgPad::Addr(7)),
                ("P90", SharedCfgPad::Addr(8)),
                ("P91", SharedCfgPad::Addr(9)),
                ("P94", SharedCfgPad::Addr(10)),
                ("P95", SharedCfgPad::Addr(11)),
                ("P96", SharedCfgPad::Addr(12)),
                ("P97", SharedCfgPad::Addr(13)),
                ("P98", SharedCfgPad::Addr(14)),
                ("P99", SharedCfgPad::Addr(15)),
                ("P2", SharedCfgPad::Addr(16)),
                ("P3", SharedCfgPad::Addr(17)),
            ]);
        }
        "pq160" | "hq160" => {
            pkg_cfg_io.extend([
                ("P143", SharedCfgPad::Addr(8)),
                ("P144", SharedCfgPad::Addr(9)),
                ("P147", SharedCfgPad::Addr(10)),
                ("P148", SharedCfgPad::Addr(11)),
                ("P154", SharedCfgPad::Addr(12)),
                ("P155", SharedCfgPad::Addr(13)),
                ("P158", SharedCfgPad::Addr(14)),
                ("P159", SharedCfgPad::Addr(15)),
                ("P2", SharedCfgPad::Addr(16)),
                ("P3", SharedCfgPad::Addr(17)),
                ("P146", SharedCfgPad::Addr(18)),
                ("P145", SharedCfgPad::Addr(19)),
                ("P138", SharedCfgPad::Addr(20)),
                ("P137", SharedCfgPad::Addr(21)),
                ("P6", SharedCfgPad::Tdi),
                ("P7", SharedCfgPad::Tck),
                ("P13", SharedCfgPad::Tms),
                ("P44", SharedCfgPad::Hdc),
                ("P48", SharedCfgPad::Ldc),
                ("P59", SharedCfgPad::InitB),
                ("P83", SharedCfgPad::Data(7)),
                ("P87", SharedCfgPad::Data(6)),
                ("P94", SharedCfgPad::Data(5)),
                ("P95", SharedCfgPad::Cs0B),
                ("P98", SharedCfgPad::Data(4)),
                ("P102", SharedCfgPad::Data(3)),
                ("P103", SharedCfgPad::Cs1B),
                ("P106", SharedCfgPad::Data(2)),
                ("P113", SharedCfgPad::Data(1)),
                ("P114", SharedCfgPad::RclkB),
                ("P117", SharedCfgPad::Data(0)),
                ("P118", SharedCfgPad::Dout),
                ("P123", SharedCfgPad::Addr(0)),
                ("P124", SharedCfgPad::Addr(1)),
                ("P127", SharedCfgPad::Addr(2)),
                ("P128", SharedCfgPad::Addr(3)),
                ("P134", SharedCfgPad::Addr(4)),
                ("P135", SharedCfgPad::Addr(5)),
                ("P139", SharedCfgPad::Addr(6)),
                ("P140", SharedCfgPad::Addr(7)),
            ]);
        }
        "pq240" | "hq240" => {
            pkg_cfg_io.extend([
                ("P213", SharedCfgPad::Addr(8)),
                ("P214", SharedCfgPad::Addr(9)),
                ("P220", SharedCfgPad::Addr(10)),
                ("P221", SharedCfgPad::Addr(11)),
                ("P232", SharedCfgPad::Addr(12)),
                ("P233", SharedCfgPad::Addr(13)),
                ("P238", SharedCfgPad::Addr(14)),
                ("P239", SharedCfgPad::Addr(15)),
                ("P2", SharedCfgPad::Addr(16)),
                ("P3", SharedCfgPad::Addr(17)),
                ("P216", SharedCfgPad::Addr(18)),
                ("P215", SharedCfgPad::Addr(19)),
                ("P208", SharedCfgPad::Addr(20)),
                ("P207", SharedCfgPad::Addr(21)),
                ("P64", SharedCfgPad::Hdc),
                ("P68", SharedCfgPad::Ldc),
                ("P89", SharedCfgPad::InitB),
                ("P123", SharedCfgPad::Data(7)),
                ("P129", SharedCfgPad::Data(6)),
                ("P141", SharedCfgPad::Data(5)),
                ("P142", SharedCfgPad::Cs0B),
                ("P148", SharedCfgPad::Data(4)),
                ("P152", SharedCfgPad::Data(3)),
                ("P153", SharedCfgPad::Cs1B),
                ("P159", SharedCfgPad::Data(2)),
                ("P173", SharedCfgPad::Data(1)),
                ("P174", SharedCfgPad::RclkB),
                ("P177", SharedCfgPad::Data(0)),
                ("P178", SharedCfgPad::Dout),
                ("P183", SharedCfgPad::Addr(0)),
                ("P184", SharedCfgPad::Addr(1)),
                ("P187", SharedCfgPad::Addr(2)),
                ("P188", SharedCfgPad::Addr(3)),
                ("P202", SharedCfgPad::Addr(4)),
                ("P203", SharedCfgPad::Addr(5)),
                ("P209", SharedCfgPad::Addr(6)),
                ("P210", SharedCfgPad::Addr(7)),
            ]);
        }
        "bg432" => {
            pkg_cfg_io.extend([
                ("D17", SharedCfgPad::Addr(8)),
                ("A17", SharedCfgPad::Addr(9)),
                ("B19", SharedCfgPad::Addr(10)),
                ("C19", SharedCfgPad::Addr(11)),
                ("D24", SharedCfgPad::Addr(12)),
                ("B26", SharedCfgPad::Addr(13)),
                ("C28", SharedCfgPad::Addr(14)),
                ("D28", SharedCfgPad::Addr(15)),
                ("D29", SharedCfgPad::Addr(16)),
                ("C30", SharedCfgPad::Addr(17)),
                ("D18", SharedCfgPad::Addr(18)),
                ("C18", SharedCfgPad::Addr(19)),
                ("B15", SharedCfgPad::Addr(20)),
                ("C15", SharedCfgPad::Addr(21)),
                ("AH27", SharedCfgPad::Hdc),
                ("AH26", SharedCfgPad::Ldc),
                ("AK16", SharedCfgPad::InitB),
                ("AJ2", SharedCfgPad::Data(7)),
                ("AF1", SharedCfgPad::Data(6)),
                ("AA2", SharedCfgPad::Data(5)),
                ("Y2", SharedCfgPad::Cs0B),
                ("T1", SharedCfgPad::Data(4)),
                ("T3", SharedCfgPad::Data(3)),
                ("R1", SharedCfgPad::Cs1B),
                ("L2", SharedCfgPad::Data(2)),
                ("G4", SharedCfgPad::Data(1)),
                ("F2", SharedCfgPad::RclkB),
                ("C2", SharedCfgPad::Data(0)),
                ("D3", SharedCfgPad::Dout),
                ("B3", SharedCfgPad::Addr(0)),
                ("D5", SharedCfgPad::Addr(1)),
                ("A5", SharedCfgPad::Addr(2)),
                ("D7", SharedCfgPad::Addr(3)),
                ("C14", SharedCfgPad::Addr(4)),
                ("A13", SharedCfgPad::Addr(5)),
                ("B16", SharedCfgPad::Addr(6)),
                ("A16", SharedCfgPad::Addr(7)),
            ]);
        }
        _ => (),
    }
    for (pin, io) in pkg_cfg_io {
        let pad = bond_pins[pin];
        let BondPad::Io(crd) = pad else {
            unreachable!()
        };
        if matches!(io, SharedCfgPad::Addr(18..=21))
            && !matches!(
                endev.chip.kind,
                ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv
            )
        {
            continue;
        }
        if matches!(
            io,
            SharedCfgPad::Addr(0 | 1 | 3..)
                | SharedCfgPad::Cs1B
                | SharedCfgPad::RclkB
                | SharedCfgPad::Cs0B
        ) && endev.chip.kind == ChipKind::SpartanXl
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
