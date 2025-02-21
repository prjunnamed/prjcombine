use prjcombine_interconnect::grid::EdgeIoCoord;
use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_xc2000::{
    bond::{Bond, BondPin, CfgPin},
    grid::SharedCfgPin,
};
use prjcombine_re_xilinx_naming_xc2000::ExpandedNamedDevice;
use std::collections::{btree_map, BTreeMap, HashMap};

pub fn make_bond(
    endev: &ExpandedNamedDevice,
    pkg: &str,
    pins: &[PkgPin],
    cfg_io: &mut BTreeMap<SharedCfgPin, EdgeIoCoord>,
) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = endev
        .grid
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io), io))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(&pad[..]) {
                BondPin::Io(io)
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
    let (gnd, vcc, done, prog, cclk) = match pkg {
        "pc84" => (
            &["P1", "P12", "P21", "P31", "P43", "P52", "P64", "P76"][..],
            &["P2", "P11", "P22", "P33", "P42", "P54", "P63", "P74"][..],
            "P53",
            "P55",
            "P73",
        ),
        "pq100" => (
            &["P4", "P14", "P26", "P41", "P52", "P67", "P80", "P91"][..],
            &["P3", "P15", "P28", "P40", "P54", "P66", "P78", "P92"][..],
            "P53",
            "P55",
            "P77",
        ),
        "pq160" => (
            &[
                "P1", "P10", "P19", "P29", "P39", "P51", "P61", "P70", "P79", "P91", "P101",
                "P110", "P122", "P131", "P141", "P151",
            ][..],
            &["P20", "P41", "P60", "P81", "P100", "P120", "P142", "P160"][..],
            "P80",
            "P82",
            "P119",
        ),
        "pq208" | "hq208" => (
            &[
                "P2", "P14", "P25", "P37", "P49", "P67", "P79", "P90", "P101", "P119", "P131",
                "P142", "P160", "P171", "P182", "P194",
            ][..],
            &["P26", "P55", "P78", "P106", "P130", "P154", "P183", "P205"][..],
            "P103",
            "P108",
            "P153",
        ),
        "pq240" | "hq240" => (
            &[
                "P1", "P14", "P29", "P45", "P59", "P75", "P91", "P106", "P119", "P135", "P151",
                "P166", "P182", "P196", "P211", "P227",
            ][..],
            &[
                "P19", "P30", "P40", "P61", "P80", "P90", "P101", "P121", "P140", "P150", "P161",
                "P180", "P201", "P212", "P222", "P240",
            ][..],
            "P120",
            "P122",
            "P179",
        ),
        "hq304" => (
            &[
                "P19", "P39", "P58", "P75", "P95", "P114", "P134", "P154", "P171", "P190", "P210",
                "P230", "P248", "P268", "P287", "P304",
            ][..],
            &[
                "P1", "P25", "P38", "P52", "P77", "P101", "P115", "P129", "P152", "P177", "P191",
                "P204", "P228", "P253", "P267", "P282",
            ][..],
            "P153",
            "P151",
            "P78",
        ),
        "tq144" => (
            &[
                "P1", "P8", "P17", "P27", "P35", "P45", "P55", "P64", "P71", "P73", "P81", "P91",
                "P100", "P110", "P118", "P127", "P137",
            ][..],
            &["P18", "P37", "P54", "P90", "P108", "P128", "P144"][..],
            "P72",
            "P74",
            "P107",
        ),
        "tq176" => (
            &[
                "P1", "P10", "P22", "P33", "P43", "P55", "P67", "P78", "P87", "P99", "P111",
                "P122", "P134", "P143", "P154", "P166",
            ][..],
            &["P21", "P45", "P66", "P89", "P110", "P132", "P155", "P176"][..],
            "P88",
            "P90",
            "P131",
        ),
        "vq64" => (
            &["P8", "P25", "P41", "P56"][..],
            &["P9", "P24", "P33", "P40", "P64"][..],
            "P32",
            "P34",
            "P48",
        ),
        "vq100" => (
            &["P1", "P11", "P23", "P38", "P49", "P64", "P77", "P88"][..],
            &["P12", "P25", "P37", "P51", "P63", "P75", "P89", "P100"][..],
            "P50",
            "P52",
            "P74",
        ),
        "pg156" => (
            &[
                "F3", "C4", "C6", "C8", "C11", "C13", "F14", "J14", "L14", "P14", "P11", "P8",
                "P6", "N3", "L3", "H2",
            ][..],
            &["H3", "C3", "B8", "C14", "H14", "P13", "R8", "P3"][..],
            "R15",
            "R14",
            "R2",
        ),
        "pg191" => (
            &[
                "G3", "D4", "C7", "D9", "C12", "D15", "G16", "K15", "M16", "R16", "T12", "R9",
                "T7", "R3", "M3", "K4",
            ][..],
            &["J4", "D3", "D10", "D16", "J15", "R15", "R10", "R4"][..],
            "U17",
            "V18",
            "V1",
        ),
        "pg223" => (
            &[
                "G3", "D4", "C7", "D9", "C12", "D15", "G16", "K15", "M16", "R16", "T12", "R9",
                "T7", "R3", "M3", "K4",
            ][..],
            &["J4", "D3", "D10", "D16", "J15", "R15", "R10", "R4"][..],
            "U17",
            "V18",
            "V1",
        ),
        "pg299" => (
            &[
                "F1", "B1", "A5", "A10", "A15", "A19", "E20", "K20", "R20", "W20", "X16", "X11",
                "X6", "X2", "T1", "L1",
            ][..],
            &[
                "K1", "E1", "A2", "A6", "A11", "A16", "B20", "F20", "L20", "T20", "X19", "X15",
                "X10", "X5", "W1", "R1",
            ][..],
            "V18",
            "U17",
            "V3",
        ),
        "bg225" => (
            &[
                "A1", "D12", "G7", "G9", "H6", "H8", "H10", "J8", "K8", "A8", "F8", "G8", "H2",
                "H7", "H9", "J7", "J9", "M8",
            ][..],
            &["B2", "D8", "H15", "R8", "B14", "R1", "H1", "R15"][..],
            "P14",
            "M12",
            "C13",
        ),
        "bg352" => (
            &[
                "A1", "A2", "A5", "A8", "A14", "A19", "A22", "A25", "A26", "B1", "B26", "E1",
                "E26", "H1", "H26", "N1", "P26", "W1", "W26", "AB1", "AB26", "AE1", "AE26", "AF1",
                "AF13", "AF19", "AF2", "AF22", "AF25", "AF26", "AF5", "AF8",
            ][..],
            &[
                "A10", "A17", "B2", "B25", "D13", "D19", "D7", "G23", "H4", "K1", "K26", "N23",
                "P4", "U1", "U26", "W23", "Y4", "AC14", "AC20", "AC8", "AE2", "AE25", "AF10",
                "AF17",
            ][..],
            "AD3",
            "AC4",
            "C3",
        ),
        _ => {
            println!("UNK PKG {pkg}");
            (&[][..], &[][..], "DONE1", "PROG1", "CCLK1")
        }
    };
    for &pin in gnd {
        // println!("INSERT {pkg} {pin} GND");
        assert_eq!(bond_pins.insert(pin.to_string(), BondPin::Gnd), None);
    }
    for &pin in vcc {
        // println!("INSERT {pkg} {pin} VCC");
        assert_eq!(bond_pins.insert(pin.to_string(), BondPin::Vcc), None);
    }
    assert_eq!(
        bond_pins.insert(done.to_string(), BondPin::Cfg(CfgPin::Done)),
        None
    );
    assert_eq!(
        bond_pins.insert(prog.to_string(), BondPin::Cfg(CfgPin::ProgB)),
        None
    );
    assert_eq!(
        bond_pins.insert(cclk.to_string(), BondPin::Cfg(CfgPin::Cclk)),
        None
    );

    let len1d = match pkg {
        "pc84" => Some(84),
        "pq100" => Some(100),
        "pq160" => Some(160),
        "pq208" | "hq208" => Some(208),
        "pq240" | "hq240" => Some(240),
        "hq304" => Some(304),
        "tq144" => Some(144),
        "tq176" => Some(176),
        "vq100" => Some(100),
        "vq64" => Some(64),
        _ => None,
    };
    if let Some(len1d) = len1d {
        for i in 1..=len1d {
            bond_pins.entry(format!("P{i}")).or_insert(BondPin::Nc);
        }
        assert_eq!(bond_pins.len(), len1d);
    }
    match pkg {
        "bg225" => {
            assert_eq!(bond_pins.len(), 225);
        }
        "bg352" => {
            for a in ["A", "B", "C", "D", "AC", "AD", "AE", "AF"] {
                for i in 1..=26 {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in [
                "E", "F", "G", "H", "J", "K", "L", "M", "N", "P", "R", "T", "U", "V", "W", "Y",
                "AA", "AB",
            ] {
                for i in (1..=4).chain(23..=26) {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond_pins.len(), 352);
        }
        "pg156" => {
            for a in ["A", "B", "C", "P", "R", "T"] {
                for i in 1..=16 {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K", "L", "M", "N"] {
                for i in (1..=3).chain(14..=16) {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond_pins.len(), 156);
        }
        "pg191" => {
            for i in 2..=18 {
                bond_pins.entry(format!("A{i}")).or_insert(BondPin::Nc);
            }
            for a in ["B", "C", "T", "U", "V"] {
                for i in 1..=18 {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K", "L", "M", "N", "P", "R"] {
                for i in (1..=3).chain(16..=18) {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["D", "R"] {
                for i in [4, 9, 10, 15] {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["J", "K"] {
                for i in [4, 15] {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond_pins.len(), 191);
        }
        "pg223" => {
            for i in 2..=18 {
                bond_pins.entry(format!("A{i}")).or_insert(BondPin::Nc);
            }
            for a in ["B", "C", "D", "R", "T", "U", "V"] {
                for i in 1..=18 {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["E", "F", "G", "H", "J", "K", "L", "M", "N", "P"] {
                for i in (1..=4).chain(15..=18) {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond_pins.len(), 223);
        }
        "pg299" => {
            for i in 2..=20 {
                bond_pins.entry(format!("A{i}")).or_insert(BondPin::Nc);
            }
            for a in ["B", "C", "D", "E", "T", "U", "V", "W", "X"] {
                for i in 1..=20 {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["F", "G", "H", "J", "K", "L", "M", "N", "P", "R"] {
                for i in (1..=5).chain(16..=20) {
                    bond_pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond_pins.len(), 299);
        }
        _ => (),
    }
    let pkg_cfg_io = match pkg {
        "pc84" => &[
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
            ("P30", SharedCfgPin::M1),
            ("P32", SharedCfgPin::M0),
            ("P34", SharedCfgPin::M2),
            ("P36", SharedCfgPin::Hdc),
            ("P37", SharedCfgPin::Ldc),
            ("P41", SharedCfgPin::InitB),
            ("P56", SharedCfgPin::Data(7)),
            ("P58", SharedCfgPin::Data(6)),
            ("P59", SharedCfgPin::Data(5)),
            ("P60", SharedCfgPin::Cs0B),
            ("P61", SharedCfgPin::Data(4)),
            ("P65", SharedCfgPin::Data(3)),
            ("P66", SharedCfgPin::Cs1B),
            ("P67", SharedCfgPin::Data(2)),
            ("P69", SharedCfgPin::Data(1)),
            ("P70", SharedCfgPin::RclkB),
            ("P71", SharedCfgPin::Data(0)),
            ("P72", SharedCfgPin::Dout),
            ("P75", SharedCfgPin::Tdo),
            ("P77", SharedCfgPin::Addr(0)),
            ("P78", SharedCfgPin::Addr(1)),
            ("P79", SharedCfgPin::Addr(2)),
            ("P80", SharedCfgPin::Addr(3)),
            ("P81", SharedCfgPin::Addr(4)),
            ("P82", SharedCfgPin::Addr(5)),
            ("P83", SharedCfgPin::Addr(6)),
            ("P84", SharedCfgPin::Addr(7)),
        ][..],
        "pq160" => &[
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
            ("P6", SharedCfgPin::Tdi),
            ("P7", SharedCfgPin::Tck),
            ("P13", SharedCfgPin::Tms),
            ("P38", SharedCfgPin::M1),
            ("P40", SharedCfgPin::M0),
            ("P42", SharedCfgPin::M2),
            ("P44", SharedCfgPin::Hdc),
            ("P48", SharedCfgPin::Ldc),
            ("P59", SharedCfgPin::InitB),
            ("P83", SharedCfgPin::Data(7)),
            ("P87", SharedCfgPin::Data(6)),
            ("P94", SharedCfgPin::Data(5)),
            ("P95", SharedCfgPin::Cs0B),
            ("P98", SharedCfgPin::Data(4)),
            ("P102", SharedCfgPin::Data(3)),
            ("P103", SharedCfgPin::Cs1B),
            ("P106", SharedCfgPin::Data(2)),
            ("P113", SharedCfgPin::Data(1)),
            ("P114", SharedCfgPin::RclkB),
            ("P117", SharedCfgPin::Data(0)),
            ("P118", SharedCfgPin::Dout),
            ("P121", SharedCfgPin::Tdo),
            ("P123", SharedCfgPin::Addr(0)),
            ("P124", SharedCfgPin::Addr(1)),
            ("P127", SharedCfgPin::Addr(2)),
            ("P128", SharedCfgPin::Addr(3)),
            ("P134", SharedCfgPin::Addr(4)),
            ("P135", SharedCfgPin::Addr(5)),
            ("P139", SharedCfgPin::Addr(6)),
            ("P140", SharedCfgPin::Addr(7)),
        ][..],
        _ => &[][..],
    };
    for &(pin, io) in pkg_cfg_io {
        let pad = bond_pins[pin];
        let BondPin::Io(crd) = pad else {
            unreachable!()
        };
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
