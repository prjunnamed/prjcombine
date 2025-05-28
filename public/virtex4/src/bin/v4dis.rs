use std::path::PathBuf;
use std::{collections::HashSet, error::Error};

use bitvec::prelude::*;

use clap::{Arg, Command, value_parser};
use prjcombine_interconnect::grid::TileIobId;
use prjcombine_types::tiledb::TileItemKind;
use prjcombine_virtex4::bond::BondPin;
use prjcombine_virtex4::db::Database;
use prjcombine_virtex4::expanded::IoCoord;
use prjcombine_xilinx_bitstream::{KeyData, Reg};
use unnamed_entity::EntityId;

fn main() -> Result<(), Box<dyn Error>> {
    let m = Command::new("v2dis")
        .arg(
            Arg::new("db")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("part")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("bitfile")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();
    let arg_db = m.get_one::<PathBuf>("db").unwrap();
    let arg_part = m.get_one::<String>("part").unwrap();
    let arg_bitfile = m.get_one::<PathBuf>("bitfile").unwrap();
    let db = Database::from_file(arg_db)?;
    let bitdata = std::fs::read(arg_bitfile)?;
    let (device, bond) = 'a: {
        for device in &db.parts {
            for (_, name, &bond) in &device.bonds {
                let curpart = format!("{}{}", device.name, name);
                if *arg_part == curpart {
                    break 'a (device, bond);
                }
            }
        }
        panic!("umm unknown device {arg_part}?");
    };
    let edev = db.expand_grid(device);
    let bond = &db.bonds[bond];
    let ebond = bond.expand();
    let bitstream =
        prjcombine_xilinx_bitstream::parse(&edev.bs_geom, &bitdata[0x100..], &KeyData::None);
    for (die, dbs) in &bitstream.die {
        if let Some(&val) = dbs.regs.get(&Reg::Idcode) {
            println!("DIE {die} IDCODE {val:08x}");
        }
        let edie = edev.egrid.die(die);
        let mut handled_bits = HashSet::new();
        for col in edie.cols() {
            for row in edie.rows() {
                let cell = &edie[(col, row)];
                for (layer, node) in &cell.nodes {
                    let nloc = (die, col, row, layer);
                    let kind = db.int.nodes.key(node.kind);
                    let tname = format!("D{die}X{col}Y{row}_{kind}");
                    if let Some(btc) = db.tiles.tiles.get(kind) {
                        let mut got_bits = false;
                        let btiles = edev.node_bits(nloc);
                        for (name, item) in &btc.items {
                            let bits: BitVec = item
                                .bits
                                .iter()
                                .map(|&crd| {
                                    let btile = btiles[crd.tile];
                                    let bcrd = btile.xlat_pos_fwd((crd.frame, crd.bit));
                                    let val = bitstream.get_bit(bcrd);
                                    if val {
                                        handled_bits.insert(bcrd);
                                    }
                                    val
                                })
                                .collect();
                            if !bits.any() {
                                continue;
                            }
                            if !got_bits {
                                println!("TILE {tname}:");
                                got_bits = true;
                            }
                            match &item.kind {
                                TileItemKind::Enum { values } => {
                                    print!("    {name}=");
                                    let mut found = false;
                                    for (vn, val) in values {
                                        if val == &bits {
                                            print!("{vn}");
                                            found = true;
                                            break;
                                        }
                                    }
                                    if !found {
                                        for bit in bits.iter().rev() {
                                            print!("{}", u8::from(*bit));
                                        }
                                    }
                                }
                                TileItemKind::BitVec { invert } => {
                                    let bits: BitVec = bits
                                        .iter()
                                        .zip(invert.iter())
                                        .map(|(bit, inv)| *bit ^ *inv)
                                        .collect();
                                    if item.bits.len() == 1 {
                                        if bits[0] {
                                            print!("    {name}");
                                        } else {
                                            print!("    !{name}");
                                        }
                                    } else {
                                        print!("    {name}=");
                                        for bit in bits.iter().rev() {
                                            print!("{}", u8::from(*bit));
                                        }
                                    }
                                }
                            }
                            println!();
                        }
                        if got_bits {
                            let ios = match kind.as_str() {
                                "IO_HR_BOT" | "IO_HR_TOP" | "IO_HP_BOT" | "IO_HP_TOP" => 1,
                                "IO_HR_PAIR" | "IO_HP_PAIR" => 2,
                                _ => 0,
                            };
                            for iob in 0..ios {
                                let iob = TileIobId::from_idx(iob);
                                let io = edev.get_io_info(IoCoord { die, col, row, iob });
                                if let Some(pin) = ebond.ios.get(&(io.bank, io.biob)) {
                                    println!("    IOB{iob} is at {pin}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
