use std::{
    collections::BTreeMap,
    io::{self, Write},
};

use prjcombine_xact_naming::grid::PipCoords;

#[derive(Debug)]
pub struct Design {
    pub part: String,
    pub package: String,
    pub speed: String,
    pub nets: Vec<Net>,
    pub blocks: Vec<Block>,
}

#[derive(Debug)]
pub struct Net {
    pub name: String,
    pub pins: Vec<(String, String)>,
    pub pips: Vec<PipCoords>,
}

#[derive(Debug)]
pub struct Block {
    pub loc: String,
    pub base: Option<String>,
    pub cfg: BTreeMap<String, Vec<String>>,
    pub equate: Vec<(String, String)>,
}

impl Design {
    pub fn write(&self, f: &mut dyn Write) -> io::Result<()> {
        writeln!(f, "Version 2")?;
        writeln!(
            f,
            "Design {}{} {} 0",
            self.part[2..].to_ascii_uppercase(),
            self.package.to_ascii_uppercase(),
            match &self.part[2..3] {
                "2" => 8,
                "3" => 10,
                "4" => 4,
                "5" => 6,
                _ => panic!("ummm wtf is {}", self.part),
            }
        )?;
        writeln!(f, "Speed {}", self.speed)?;
        for blk in &self.blocks {
            writeln!(f, "Editblk {}", blk.loc)?;
            if let Some(ref base) = blk.base {
                writeln!(f, "Base {base}")?;
            }
            if !blk.cfg.is_empty() {
                write!(f, "Config")?;
                for (k, vs) in &blk.cfg {
                    write!(f, " {k}")?;
                    for v in vs {
                        write!(f, ":{v}")?;
                    }
                }
                writeln!(f)?;
            }
            for (k, v) in &blk.equate {
                writeln!(f, "Equate {k} = {v}")?;
            }
            writeln!(f, "Endblk")?;
        }
        for net in &self.nets {
            write!(f, "Addnet {}", net.name)?;
            for (blk, pin) in &net.pins {
                write!(f, " {blk}.{pin}")?;
            }
            writeln!(f)?;
            for &crd in &net.pips {
                match crd {
                    PipCoords::Pip((x, y)) => {
                        writeln!(f, "Program {nn} {{{x}G{y}}}", nn = net.name)?;
                    }
                    PipCoords::BoxPip((x0, y0), (x1, y1)) => {
                        writeln!(f, "Program {nn} {{{x0}G{y0}}} {{{x1}G{y1}}}", nn = net.name)?;
                    }
                }
            }
        }
        Ok(())
    }
}
