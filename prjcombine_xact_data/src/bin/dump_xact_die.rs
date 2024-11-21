use std::path::PathBuf;

use clap::Parser;
use prjcombine_xact_data::die::Die;

#[derive(Parser)]
struct Args {
    xact: PathBuf,
    part: String,
}

fn main() {
    let args = Args::parse();
    let die = Die::parse(&args.xact, &args.part);
    let matrix = die.make_unified_matrix();
    for row in (0..matrix.dim().1).rev() {
        for col in 0..matrix.dim().0 {
            let c = matrix[(col, row)];
            let (g, c) = match c & 0xff {
                _ if (c & 0x3f) == 9 => ('M', 'Y'),
                0x31 => ('O', 'Y'),
                0x35 => ('I', 'Y'),
                0x39 => (' ', ' '),
                0x11 => ('─', 'P'),
                0x15 => ('│', 'P'),
                0x18 => ('│', 'W'),
                0x58 => ('─', 'W'),
                0x10 => ('┬', 'W'),
                0x50 => ('┤', 'W'),
                0x90 => ('┴', 'W'),
                0xd0 => ('├', 'W'),
                0x14 => ('└', 'W'),
                0x54 => ('┌', 'W'),
                0x94 => ('┐', 'W'),
                0xd4 => ('┘', 'W'),
                0x0c => ('┼', 'W'),

                0x28 => ('│', 'B'),
                0x68 => ('─', 'B'),
                0x2b => ('V', 'R'),
                0x2a => ('^', 'R'),
                0x6b => ('<', 'G'),
                0x6a => ('>', 'G'),

                0x20 => ('┬', 'B'),
                0x60 => ('┤', 'B'),
                0xa0 => ('┴', 'B'),
                0xe0 => ('├', 'B'),
                0x22 => ('┬', 'R'),
                0x62 => ('┤', 'G'),
                0xa2 => ('┴', 'R'),
                0xe2 => ('├', 'G'),
                0x23 => ('┬', 'G'),
                0x63 => ('┤', 'R'),
                0xa3 => ('┴', 'G'),
                0xe3 => ('├', 'R'),

                0x24 => ('└', 'B'),
                0x64 => ('┌', 'B'),
                0xa4 => ('┐', 'B'),
                0xe4 => ('┘', 'B'),
                0x26 => ('└', 'R'),
                0x66 => ('┌', 'G'),
                0xa6 => ('┐', 'R'),
                0xe6 => ('┘', 'G'),
                0x27 => ('└', 'G'),
                0x67 => ('┌', 'R'),
                0xa7 => ('┐', 'G'),
                0xe7 => ('┘', 'R'),

                0x2c => ('┼', 'B'),
                0x2e => ('┼', 'R'),
                0x2f => ('┼', 'G'),

                _ => panic!("umm {c}"),
            };
            match c {
                ' ' => (),
                'W' => print!("\x1b[1m"),
                'R' => print!("\x1b[31;1m"),
                'G' => print!("\x1b[32;1m"),
                'B' => print!("\x1b[34;1m"),
                'P' => print!("\x1b[35;1m"),
                'Y' => print!("\x1b[33;1m"),
                _ => unreachable!(),
            }
            print!("{g}");
            if c != ' ' {
                print!("\x1b[0m");
            }
        }
        println!();
    }
}
