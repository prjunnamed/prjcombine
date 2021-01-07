use std::io::{BufRead, Lines};
use std::str::FromStr;
use crate::xilinx::rawdump::TkSitePinDir;
use crate::error::Error;
use crate::toolreader::ToolchainReader;
use crate::toolchain::Toolchain;

use Error::ParseError;

#[derive(Debug)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub name: String,
    pub kind: String,
    pub prims: Vec<Prim>,
    pub wires: Vec<Wire>,
    pub pips: Vec<Pip>,
}

#[derive(Debug)]
pub enum PrimBonded {
    Bonded,
    Unbonded,
    Internal,
}

#[derive(Debug)]
pub struct Prim {
    pub name: String,
    pub kind: String,
    pub bonded: PrimBonded,
    pub pinwires: Vec<PinWire>,
}

#[derive(Debug)]
pub struct PinWire {
    pub name: String,
    pub dir: TkSitePinDir,
    pub wire: String,
}

#[derive(Debug)]
pub struct Wire {
    pub name: String,
    pub speed: Option<String>,
    pub conns: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum PipKind {
    Uni,
    BiPass,
    BiUniBuf,
    BiBuf,
}

#[derive(Debug)]
pub struct PipRouteThrough {
    pub pin_from: String,
    pub pin_to: String,
    pub prim_kind: String,
}

#[derive(Debug)]
pub struct Pip {
    pub wire_from: String,
    pub wire_to: String,
    pub kind: PipKind,
    pub speed: Option<String>,
    pub route_through: Option<PipRouteThrough>,
}

pub struct Parser {
    version: String,
    part: String,
    family: String,
    width: u32,
    height: u32,
    lines: Lines<Box<dyn BufRead>>,
    tiles_done: bool,
}

pub struct Options {
    pub part: String,
    pub need_pips: bool,
    pub need_conns: bool,
    pub dump_test: bool,
    pub dump_excluded: bool,
}

impl FromStr for PipKind {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        match s {
            "->" => Ok(PipKind::Uni),
            "==" => Ok(PipKind::BiPass),
            "=>" => Ok(PipKind::BiUniBuf),
            "=-" => Ok(PipKind::BiBuf),
            _ => Err(ParseError(format!("invalid pip direction {}", s))),
        }
    }
}

impl Parser {
    pub fn new(file: Box<dyn BufRead>) -> Result<Self, Error> {
        let mut lines = file.lines();
        // Comments.
        let l = loop {
            let l = lines
                .next()
                .ok_or(ParseError(format!("eof before xdl_resource_report")))??;
            if !l.starts_with("#") {
                break l;
            }
        };
        // xdl_resource_report.
        let l: Vec<_> = l
            .strip_prefix("(xdl_resource_report ")
            .ok_or(ParseError(format!("expected xdl_resource_report")))?
            .split(" ")
            .collect();
        let (version, part, family) = match l[..] {
            [v, p, f] => (v.to_string(), p.to_string(), f.to_string()),
            _ => return Err(ParseError(format!("xdl_resource_report wrong arg count"))),
        };
        // More comments.
        let l = loop {
            let l = lines
                .next()
                .ok_or(ParseError(format!("eof before xdl_resource_report")))??;
            if !l.starts_with("#") {
                break l;
            }
        };
        // tiles.
        let l: Vec<_> = l
            .strip_prefix("(tiles ")
            .ok_or(ParseError(format!("expected tiles")))?
            .split(" ")
            .collect();
        let (width, height) = match l[..] {
            [h, w] => (w.parse::<u32>()?, h.parse::<u32>()?),
            _ => return Err(ParseError(format!("tiles wrong arg count"))),
        };
        // Make the actual parser.
        Ok(Parser {
            version,
            part,
            family,
            width,
            height,
            lines,
            tiles_done: false,
        })
    }

    pub fn from_toolchain(tc: &Toolchain, opt: Options) -> Result<Self, Error> {
        let mut args = vec!["-report"];
        let mut env: Vec<(&'static str, &'static str)> = Vec::new();
        if opt.need_pips {
            args.push("-pips");
        }
        if opt.need_conns && !tc.use_wine {
            args.push("-all_conns");
            args.push("-speed");
        }
        if opt.dump_test {
            env.push(("XIL_TEST_ARCS", "1"));
        }
        if opt.dump_excluded {
            env.push(("XIL_DRM_EXCLUDE_ARCS", "1"));
        }
        args.push(&opt.part);
        args.push("fifo.xdlrc");
        Parser::new(Box::new(ToolchainReader::new(tc, "xdl", &args, &env, "fifo.xdlrc")?))
    }

    pub fn get_tile(&mut self) -> Result<Option<Tile>, Error> {
        if self.tiles_done {
            return Ok(None);
        }
        let l = self
            .lines
            .next()
            .ok_or(ParseError(format!("eof in tiles")))??;
        if let Some(l) = l.strip_prefix("\t(tile ") {
            // Parse tile.
            let l: Vec<_> = l.split(" ").collect();
            let (x, y, name, kind) = match l[..] {
                [y, x, name, kind, _] => (
                    x.parse::<u32>()?,
                    y.parse::<u32>()?,
                    name.to_string(),
                    kind.to_string(),
                ),
                _ => return Err(ParseError(format!("tile wrong arg count"))),
            };

            let mut prims: Vec<Prim> = Vec::new();
            let mut wires: Vec<Wire> = Vec::new();
            let mut pips: Vec<Pip> = Vec::new();
            // Parse things.
            loop {
                let l = self
                    .lines
                    .next()
                    .ok_or(ParseError(format!("eof in tile")))??;
                if l == "\t)" {
                    break;
                } else if let Some(l) = l.strip_prefix("\t\t(primitive_site ") {
                    let (l, has_body) = match l.strip_suffix(")") {
                        Some(sl) => (sl, false),
                        None => (l, true),
                    };
                    let mut pinwires: Vec<PinWire> = Vec::new();
                    if has_body {
                        loop {
                            let l = self
                                .lines
                                .next()
                                .ok_or(ParseError(format!("eof in primitive_site")))??;
                            if l == "\t\t)" {
                                break;
                            } else if let Some(l) = l.strip_prefix("\t\t\t(pinwire ") {
                                let l = l
                                    .strip_suffix(")")
                                    .ok_or(ParseError(format!("missing ) on pinwire")))?;
                                let l: Vec<_> = l.split(" ").collect();
                                match l[..] {
                                    [n, k, w] => pinwires.push(PinWire {
                                        name: n.to_string(),
                                        dir: match k {
                                            "input" => TkSitePinDir::Input,
                                            "output" => TkSitePinDir::Output,
                                            "bidir" => TkSitePinDir::Bidir,
                                            _ => {
                                                return Err(ParseError(format!(
                                                    "unknown pinwire kind {}",
                                                    k
                                                )))
                                            }
                                        },
                                        wire: w.to_string(),
                                    }),
                                    _ => {
                                        return Err(ParseError(format!("pinwire wrong arg count")))
                                    }
                                }
                            } else {
                                return Err(ParseError(format!(
                                    "expected primitive_site item: {}",
                                    l
                                )));
                            }
                        }
                    }
                    let l: Vec<_> = l.split(" ").collect();
                    let (name, kind, bonded) = match l[..] {
                        [name, kind, bonded, _] => (
                            name.to_string(),
                            kind.to_string(),
                            match bonded {
                                "bonded" => PrimBonded::Bonded,
                                "unbonded" => PrimBonded::Unbonded,
                                "internal" => PrimBonded::Internal,
                                _ => {
                                    return Err(ParseError(format!("unknown bonding: {}", bonded)))
                                }
                            },
                        ),
                        [name, kind, bonded, _, _] => (
                            name.to_string(),
                            kind.to_string(),
                            match bonded {
                                "bonded" => PrimBonded::Bonded,
                                "unbonded" => PrimBonded::Unbonded,
                                "internal" => PrimBonded::Internal,
                                _ => {
                                    return Err(ParseError(format!("unknown bonding: {}", bonded)))
                                }
                            },
                        ),
                        _ => return Err(ParseError(format!("primitive_site wrong arg count"))),
                    };
                    prims.push(Prim {
                        name,
                        kind,
                        bonded,
                        pinwires,
                    });
                } else if let Some(l) = l.strip_prefix("\t\t(wire ") {
                    let (l, has_body) = match l.strip_suffix(")") {
                        Some(sl) => (sl, false),
                        None => (l, true),
                    };
                    let mut conns: Vec<(String, String)> = Vec::new();
                    if has_body {
                        loop {
                            let l = self
                                .lines
                                .next()
                                .ok_or(ParseError(format!("eof in wire")))??;
                            if l == "\t\t)" {
                                break;
                            } else if let Some(l) = l.strip_prefix("\t\t\t(conn ") {
                                let l = l
                                    .strip_suffix(")")
                                    .ok_or(ParseError(format!("missing ) on conn")))?;
                                let l: Vec<_> = l.split(" ").collect();
                                match l[..] {
                                    [tile, wire] => {
                                        conns.push((tile.to_string(), wire.to_string()))
                                    }
                                    _ => return Err(ParseError(format!("conn wrong arg count"))),
                                }
                            } else {
                                return Err(ParseError(format!("expected wire item: {}", l)));
                            }
                        }
                    }
                    let l: Vec<_> = l.split(" ").collect();
                    let (name, speed) = match l[..] {
                        [name, _] => (name.to_string(), None),
                        [name, _, speed] => (name.to_string(), Some(speed.to_string())),
                        _ => return Err(ParseError(format!("wire wrong arg count"))),
                    };
                    wires.push(Wire { name, speed, conns });
                } else if let Some(l) = l.strip_prefix("\t\t(pip ") {
                    let l = l
                        .strip_suffix(")")
                        .ok_or(ParseError(format!("missing ) on pip")))?;
                    let (l, rt) = match l.strip_suffix(")") {
                        Some(l) => {
                            let sl: Vec<_> = l.split(" (_ROUTETHROUGH-").collect();
                            if sl.len() != 2 {
                                return Err(ParseError(format!("not routethru pip: {:?}", l)));
                            }
                            let sl1: Vec<_> = sl[1].split(" ").collect();
                            if sl1.len() != 2 {
                                return Err(ParseError(format!("not routethru pip: {:?}", l)));
                            }
                            let sl10: Vec<_> = sl1[0].split("-").collect();
                            if sl10.len() != 2 {
                                return Err(ParseError(format!("not routethru pip: {:?}", l)));
                            }
                            (
                                sl[0],
                                Some(PipRouteThrough {
                                    pin_from: sl10[0].to_string(),
                                    pin_to: sl10[1].to_string(),
                                    prim_kind: sl1[1].to_string(),
                                }),
                            )
                        }
                        None => (l, None),
                    };
                    let l: Vec<_> = l.split(" ").collect();
                    match l[..] {
                        [_, w1, kind, w2, speed] => pips.push(Pip {
                            wire_from: w1.to_string(),
                            wire_to: w2.to_string(),
                            kind: kind.parse()?,
                            speed: Some(speed.to_string()),
                            route_through: rt,
                        }),
                        [_, w1, kind, w2] => pips.push(Pip {
                            wire_from: w1.to_string(),
                            wire_to: w2.to_string(),
                            kind: kind.parse()?,
                            speed: None,
                            route_through: rt,
                        }),
                        _ => return Err(ParseError(format!("pip wrong arg count: {:?}", l))),
                    }
                } else if l.starts_with("\t\t(tile_summary") && l.ends_with(")") {
                    // eh.
                } else {
                    return Err(ParseError(format!("expected tile item: {}", l)));
                }
            }

            // Done.
            Ok(Some(Tile {
                x,
                y,
                name,
                kind,
                prims,
                wires,
                pips,
            }))
        } else if l == ")" {
            // XXX parse rest of the fucking owl
            self.tiles_done = true;
            Ok(None)
        } else {
            Err(ParseError(format!("expected tile: {}", l)))
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn part(&self) -> &str {
        &self.part
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
}
