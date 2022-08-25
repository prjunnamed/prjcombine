use super::parts::VivadoPart;
use indicatif::ProgressBar;
use prjcombine_entity::{entity_id, EntitySet};
use prjcombine_toolchain::{Toolchain, ToolchainReader};
use prjcombine_xilinx_rawdump::{
    build::{PartBuilder, PbPip, PbSitePin},
    Coord, Part, PkgPin, Source, TkPipDirection, TkPipInversion, TkSitePinDir,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use std::fmt::Write as FmtWrite;
use std::io::{BufRead, Write};
use std::sync::Mutex;

const TILE_BATCH_SIZE: usize = 4000;

const DUMP_TILES_TCL: &str = r#"
set ifd [open "crd.list" r]
set fd [open "tiles.fifo" w]
link_design -part [lindex $argv 0]
while { [gets $ifd ty] >= 0 } {
    foreach tile [get_tiles -filter "GRID_POINT_Y == $ty"] {
        set gx [get_property GRID_POINT_X $tile]
        set gy [get_property GRID_POINT_Y $tile]
        set tt [get_property TYPE $tile]
        puts $fd "TILE $gx $gy $tile $tt"
        foreach x [get_wires -of $tile] {
            set node [get_nodes -of $x]
            set si [get_property SPEED_INDEX $x]
            puts $fd "WIRE $x $si #$node"
        }
        foreach x [get_pips -of $tile] {
            set si [get_property SPEED_INDEX $x]
            puts $fd "PIP #$x #$si"
        }
        foreach x [get_sites -of $tile] {
            set type [get_property SITE_TYPE $x]
            puts $fd "SITE $x $type"
            foreach y [get_site_pins -of $x] {
                set node [get_nodes -of $y]
                set dir [get_property DIRECTION $y]
                set si [get_property SPEED_INDEX $y]
                puts $fd "SITEPIN #$y #$dir #$si #$node"
            }
            puts $fd "ENDSITE"
        }
        puts $fd "ENDTILE"
    }
}
puts $fd "END"
"#;

fn parse_bool(s: &str) -> bool {
    match s {
        "0" => false,
        "1" => true,
        _ => panic!("weird bool {}", s),
    }
}

entity_id! {
    id NameId u32;
}

const LIST_TILES_TCL: &str = r#"
set fd [open "tiles.fifo" w]
link_design -part [lindex $argv 0]
foreach x [get_tiles] {
    set gx [get_property GRID_POINT_X $x]
    set gy [get_property GRID_POINT_Y $x]
    set tt [get_property TYPE $x]
    puts $fd "TILE $gx $gy $tt"
}
foreach x [get_speed_models] {
    set idx [get_property SPEED_INDEX $x]
    puts $fd "SPEED #$idx #$x"
}
puts $fd "END"
"#;

struct TileData {
    tts: HashMap<String, (u16, u16)>,
    width: u16,
    height: u16,
    speed_models: HashMap<u32, String>,
}

fn list_tiles(tc: &Toolchain, part: &VivadoPart) -> TileData {
    let mut tts = HashMap::new();
    let mut tile_cnt = 0;
    let mut width: u16 = 0;
    let mut height: u16 = 0;
    let mut speed_models = HashMap::new();
    {
        let tr = ToolchainReader::new(
            tc,
            "vivado",
            &[
                "-nolog",
                "-nojournal",
                "-mode",
                "batch",
                "-source",
                "script.tcl",
                "-tclargs",
                &part.name,
            ],
            &[],
            "tiles.fifo",
            &[("script.tcl", LIST_TILES_TCL.as_bytes())],
        )
        .unwrap();
        let lines = tr.lines();
        let mut got_end = false;
        for l in lines {
            let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
            match sl[0] {
                "END" => {
                    got_end = true;
                    break;
                }
                "TILE" => {
                    if sl.len() != 4 {
                        println!("UMMMM {} {:?}", part.name, sl);
                    }
                    let gx: u16 = sl[1].parse().unwrap();
                    let gy: u16 = sl[2].parse().unwrap();
                    tile_cnt += 1;
                    if !tts.contains_key(sl[3]) {
                        tts.insert(sl[3].to_string(), (gx, gy));
                    }
                    if gy >= height {
                        height = gy + 1;
                    }
                    if gx >= width {
                        width = gx + 1;
                    }
                }
                "SPEED" => {
                    let idx: u32 = sl[1][1..].parse().unwrap();
                    let name = &sl[2][1..];
                    if idx == 65535 {
                        continue;
                    }
                    if speed_models.contains_key(&idx) {
                        panic!(
                            "double speed model {}: {} {}",
                            idx, name, speed_models[&idx]
                        );
                    }
                    speed_models.insert(idx, name.to_string());
                }
                _ => panic!("unknown line {}", sl[0]),
            }
        }
        if !got_end {
            panic!("missing END in tiles");
        }
        assert_eq!((width as usize) * (height as usize), tile_cnt);
    }
    TileData {
        tts,
        width,
        height,
        speed_models,
    }
}

struct TtPip {
    wire_from: String,
    wire_to: String,
    is_bidi: bool,
    is_buf: bool,
    is_excluded: bool,
    is_test: bool,
    is_pseudo: bool,
    inv: TkPipInversion,
}

const DUMP_TTS_TCL: &str = r#"
set ifd [open "tts.list" r]
set fd [open "tts.fifo" w]
link_design -part [lindex $argv 0]
while { [gets $ifd gx] >= 0 } {
    gets $ifd gy
    set tile [get_tiles -filter "GRID_POINT_X == $gx && GRID_POINT_Y == $gy"]
    set tt [get_property TYPE $tile]
    puts $fd "TILE $tt $tile"
    foreach x [get_pips -of $tile] {
        set wf [get_wires -uphill -of $x]
        set wt [get_wires -downhill -of $x]
        set dir [get_property IS_DIRECTIONAL $x]
        set buf0 [get_property IS_BUFFERED_2_0 $x]
        set buf1 [get_property IS_BUFFERED_2_1 $x]
        set excl [get_property IS_EXCLUDED_PIP $x]
        set test [get_property IS_TEST_PIP $x]
        set pseudo [get_property IS_PSEUDO $x]
        set invfix [get_property IS_FIXED_INVERSION $x]
        set invcan [get_property CAN_INVERT $x]
        puts $fd "PIP $x $wf $wt $dir $buf0 $buf1 $excl $test $pseudo $invfix $invcan"
    }
}
puts $fd "END"
"#;

fn dump_tts(
    tc: &Toolchain,
    part: &VivadoPart,
    tts: &HashMap<String, (u16, u16)>,
) -> HashMap<String, HashMap<String, TtPip>> {
    let mut res = HashMap::new();
    let mut tlist: Vec<u8> = Vec::new();
    for (gx, gy) in tts.values() {
        writeln!(tlist, "{gx}").unwrap();
        writeln!(tlist, "{gy}").unwrap();
    }
    let tr = ToolchainReader::new(
        tc,
        "vivado",
        &[
            "-nolog",
            "-nojournal",
            "-mode",
            "batch",
            "-source",
            "script.tcl",
            "-tclargs",
            &part.name,
        ],
        &[],
        "tts.fifo",
        &[
            ("script.tcl", DUMP_TTS_TCL.as_bytes()),
            ("tts.list", &tlist),
        ],
    )
    .unwrap();
    let lines = tr.lines();
    let mut got_end = false;
    let mut tile = "".to_string();
    let mut tt = "".to_string();
    let mut pips: Option<&mut HashMap<String, TtPip>> = None;
    for l in lines {
        let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
        match sl[0] {
            "END" => {
                got_end = true;
                break;
            }
            "TILE" => {
                if sl.len() != 3 {
                    println!("UMMMM {} {:?}", part.name, sl);
                }
                tile = sl[2].to_string();
                tt = sl[1].to_string();
                res.insert(sl[1].to_string(), HashMap::new());
                pips = Some(res.get_mut(sl[1]).unwrap());
            }
            "PIP" => {
                let prefix = tile.clone() + "/";
                let pprefix = tile.clone() + "/" + &tt + ".";
                let name = sl[1].strip_prefix(&pprefix).unwrap();
                let wf = sl[2].strip_prefix(&prefix).unwrap();
                let wt = sl[3].strip_prefix(&prefix).unwrap();
                let dir = parse_bool(sl[4]);
                let buf0 = parse_bool(sl[5]);
                let buf1 = parse_bool(sl[6]);
                let excl = parse_bool(sl[7]);
                let test = parse_bool(sl[8]);
                let pseudo = parse_bool(sl[9]);
                let invfix = parse_bool(sl[10]);
                let invcan = parse_bool(sl[11]);
                let sep = match (dir, buf0, buf1) {
                    (true, false, false) => "->",
                    (false, false, false) => "<->",
                    (true, false, true) => "->>",
                    (false, true, true) => "<<->>",
                    _ => panic!("unk pip dirbuf {} {} {} {}", name, dir, buf0, buf1),
                };
                assert_eq!(name, wf.to_string() + sep + wt);
                pips.as_mut().unwrap().insert(
                    name.to_string(),
                    TtPip {
                        wire_from: wf.to_string(),
                        wire_to: wt.to_string(),
                        is_bidi: !dir,
                        is_buf: buf1,
                        is_excluded: excl,
                        is_test: test,
                        is_pseudo: pseudo,
                        inv: match (invfix, invcan) {
                            (false, false) => TkPipInversion::Never,
                            (true, false) => TkPipInversion::Always,
                            (false, true) => TkPipInversion::Prog,
                            _ => panic!("unk inversion {} {}", invfix, invcan),
                        },
                    },
                );
            }
            _ => panic!("unknown line {}", sl[0]),
        }
    }
    assert_eq!(res.len(), tts.len());
    if !got_end {
        panic!("missing END in TTs");
    }
    res
}

const DUMP_PKGPINS_TCL: &str = r#"
set fd [open "pkgpins.fifo" w]
link_design -part [lindex $argv 0]
foreach x [get_package_pins] {
    set bank [get_property BANK $x]
    set mind [get_property MIN_DELAY $x]
    set maxd [get_property MAX_DELAY $x]
    set func [get_property PIN_FUNC $x]
    set site [get_sites -of $x]
    puts $fd "PKGPIN $x #$site #$bank #$func #$mind #$maxd"
}
puts $fd "END"
"#;

fn dump_pkgpins(tc: &Toolchain, part: &VivadoPart) -> Vec<PkgPin> {
    let mut pins: Vec<PkgPin> = Vec::new();
    let tr = ToolchainReader::new(
        tc,
        "vivado",
        &[
            "-nolog",
            "-nojournal",
            "-mode",
            "batch",
            "-source",
            "script.tcl",
            "-tclargs",
            &part.name,
        ],
        &[],
        "pkgpins.fifo",
        &[("script.tcl", DUMP_PKGPINS_TCL.as_bytes())],
    )
    .unwrap();
    let lines = tr.lines();
    let mut got_end = false;
    for l in lines {
        let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
        match sl[0] {
            "END" => {
                got_end = true;
                break;
            }
            "PKGPIN" => {
                let pin = sl[1];
                let site = &sl[2][1..];
                let bank = &sl[3][1..];
                let func = &sl[4][1..];
                let mind = &sl[5][1..];
                let maxd = &sl[6][1..];
                pins.push(PkgPin {
                    pad: if site.is_empty() {
                        None
                    } else {
                        Some(site.to_string())
                    },
                    pin: pin.to_string(),
                    vref_bank: if bank.is_empty() {
                        None
                    } else {
                        Some(bank.parse().unwrap())
                    },
                    vcco_bank: if bank.is_empty() {
                        None
                    } else {
                        Some(bank.parse().unwrap())
                    },
                    func: func.to_string(),
                    tracelen_um: None,
                    delay_min_fs: if mind.is_empty() {
                        None
                    } else {
                        Some(mind.parse().unwrap())
                    },
                    delay_max_fs: if maxd.is_empty() {
                        None
                    } else {
                        Some(maxd.parse().unwrap())
                    },
                });
            }
            _ => panic!("unknown line {}", sl[0]),
        }
    }
    if !got_end {
        panic!("missing END in tiles");
    }
    pins
}

struct Context {
    tt_pips: HashMap<String, HashMap<String, TtPip>>,
    speed_models: HashMap<u32, String>,
    mctx: Mutex<MutContext>,
    bar: ProgressBar,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
struct NodeId {
    tile: NameId,
    wire: NameId,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
struct NodeWire {
    tile: NameId,
    wire: NameId,
    speed: u32,
}

struct MutContext {
    names: EntitySet<NameId, String>,
    nodes: HashMap<NodeId, Vec<NodeWire>>,
    rd: PartBuilder,
}

pub struct VSitePin<'a> {
    name: String,
    dir: TkSitePinDir,
    wire: Option<String>,
    speed: Option<&'a str>,
}

fn dump_site<'a>(
    lines: &mut std::io::Lines<impl BufRead>,
    name: &str,
    kind: &str,
    ctx: &'a Context,
    tile_n2w: &HashMap<String, Vec<String>>,
) -> Vec<VSitePin<'a>> {
    let spref = format!("{name}/");
    let mut site_pins = Vec::new();
    loop {
        let l = lines.next().unwrap();
        let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
        match sl[0] {
            "SITEPIN" => {
                let pin = sl[1][1..].strip_prefix(&spref).unwrap();
                let dir = match &sl[2][1..] {
                    "IN" => TkSitePinDir::Input,
                    "OUT" => TkSitePinDir::Output,
                    "INOUT" => TkSitePinDir::Bidir,
                    _ => panic!("weird pin dir {}", &sl[2][1..]),
                };
                let si = &sl[3][1..];
                let node = &sl[4][1..];
                let speed: Option<&str> = if si.is_empty() {
                    None
                } else {
                    Some(&ctx.speed_models[&si.parse::<u32>().unwrap()])
                };
                let wire: Option<String> = match tile_n2w.get(node) {
                    None => None,
                    Some(v) => {
                        let mut v = v.clone();
                        if v.len() > 1 {
                            match pin {
                                "DOUT"
                                | "SYSREF_OUT_NORTH_P"
                                | "SYSREF_OUT_SOUTH_P"
                                | "CLK_DISTR_OUT_NORTH"
                                | "CLK_DISTR_OUT_SOUTH"
                                | "T1_ALLOWED_SOUTH"
                                | "CLK_IN" => {
                                    let suffix = format!("_{}", pin);
                                    v.retain(|n| n.ends_with(&suffix));
                                }
                                _ => (),
                            }
                        }
                        if v.len() == 1 {
                            Some(v[0].to_string())
                        } else {
                            panic!("SITE PIN WIRE AMBIGUOUS {kind} {pin} {sl:?} {v:?}");
                        }
                    }
                };
                site_pins.push(VSitePin {
                    name: pin.to_string(),
                    dir,
                    wire,
                    speed,
                });
            }
            "ENDSITE" => return site_pins,
            _ => panic!("unknown line {}", sl[0]),
        }
    }
}

fn dump_tile(
    lines: &mut std::io::Lines<impl BufRead>,
    crd: Coord,
    tile: &str,
    tt: &str,
    ctx: &Context,
) {
    let wpref = format!("{tile}/");
    let ppref = format!("{tile}/{tt}.");
    let tt_pips = &ctx.tt_pips[tt];
    let mut wires: Vec<(String, u32)> = Vec::new();
    let mut pips = Vec::new();
    let mut sites: Vec<(String, String, Vec<VSitePin<'_>>)> = Vec::new();
    let mut tile_n2w: HashMap<String, Vec<String>> = HashMap::new();
    let mut node_wires = Vec::new();
    loop {
        let l = lines.next().unwrap();
        let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
        match sl[0] {
            "SITE" => {
                let site_pins = dump_site(lines, sl[1], sl[2], ctx, &tile_n2w);
                sites.push((sl[1].to_string(), sl[2].to_string(), site_pins));
            }
            "WIRE" => {
                let name = sl[1].strip_prefix(&wpref).unwrap();
                let si = sl[2].parse::<u32>().unwrap();
                let node = &sl[3][1..];
                wires.push((name.to_string(), si));
                if !node.is_empty() {
                    node_wires.push((name.to_string(), node.to_string(), si));
                    tile_n2w
                        .entry(node.to_string())
                        .or_default()
                        .push(name.to_string());
                }
            }
            "PIP" => {
                let name = sl[1][1..].strip_prefix(&ppref).unwrap();
                let si = &sl[2][1..];
                let pip = &tt_pips[name];
                if pip.is_pseudo {
                    continue;
                }
                let speed: Option<&str> = if si.is_empty() {
                    None
                } else {
                    let si = si.parse::<u32>().unwrap();
                    let s = ctx.speed_models.get(&si);
                    if s.is_none() && si != 65535 {
                        println!("UMMMM NO SI {:?}", sl);
                    }
                    s.map(|x| &x[..])
                };
                if pip.is_bidi {
                    pips.push(PbPip {
                        wire_from: &pip.wire_from,
                        wire_to: &pip.wire_to,
                        is_buf: pip.is_buf,
                        is_excluded: pip.is_excluded,
                        is_test: pip.is_test,
                        inv: pip.inv,
                        dir: TkPipDirection::BiFwd,
                        speed,
                    });
                    pips.push(PbPip {
                        wire_from: &pip.wire_to,
                        wire_to: &pip.wire_from,
                        is_buf: pip.is_buf,
                        is_excluded: pip.is_excluded,
                        is_test: pip.is_test,
                        inv: pip.inv,
                        dir: TkPipDirection::BiBwd,
                        speed,
                    });
                } else {
                    pips.push(PbPip {
                        wire_from: &pip.wire_from,
                        wire_to: &pip.wire_to,
                        is_buf: pip.is_buf,
                        is_excluded: pip.is_excluded,
                        is_test: pip.is_test,
                        inv: pip.inv,
                        dir: TkPipDirection::Uni,
                        speed,
                    });
                }
            }
            "ENDTILE" => {
                let mut mctx_l = ctx.mctx.lock().unwrap();
                let mctx: &mut MutContext = &mut mctx_l;
                mctx.rd.add_tile(
                    crd,
                    tile.to_string(),
                    tt.to_string(),
                    &sites
                        .iter()
                        .map(|(n, t, p)| -> (&str, &str, _) {
                            (
                                n,
                                t,
                                p.iter()
                                    .map(|sp| PbSitePin {
                                        name: &sp.name,
                                        dir: sp.dir,
                                        wire: sp.wire.as_ref().map(|s| &s[..]),
                                        speed: sp.speed,
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .collect::<Vec<_>>(),
                    &wires
                        .iter()
                        .map(|(w, s)| -> (&str, Option<&str>) {
                            (w, Some(&ctx.speed_models[s][..]))
                        })
                        .collect::<Vec<_>>(),
                    &pips,
                );
                for (name, node, si) in node_wires {
                    let pos = node.find('/').unwrap();
                    let node = NodeId {
                        tile: mctx.names.get_or_insert(&node[..pos]),
                        wire: mctx.names.get_or_insert(&node[pos + 1..]),
                    };
                    let nwires = mctx.nodes.entry(node).or_default();
                    nwires.push(NodeWire {
                        tile: mctx.names.get_or_insert(tile),
                        wire: mctx.names.get_or_insert(&name),
                        speed: si,
                    });
                }
                ctx.bar.inc(1);
                return;
            }
            _ => panic!("unknown line {}", sl[0]),
        }
    }
}

pub fn get_rawdump(tc: &Toolchain, parts: &[VivadoPart]) -> Result<Part, Box<dyn Error>> {
    let fpart = &parts[0];

    // STEP 1: list tiles, gather list of tile types, get dimensions; also list speed models
    let td = list_tiles(tc, fpart);
    println!(
        "{}: {}×{} tiles, {} tts, {} SMs",
        fpart.device,
        td.width,
        td.height,
        td.tts.len(),
        td.speed_models.len()
    );

    // STEP 2: dump TTs [pips]
    let tt_pips = dump_tts(tc, fpart, &td.tts);

    // STEP 3: dump tiles [sites, pip speed, wires], STREAM THE MOTHERFUCKER, gather nodes
    let ctx = Context {
        tt_pips,
        speed_models: td.speed_models,
        mctx: Mutex::new(MutContext {
            names: EntitySet::new(),
            nodes: HashMap::new(),
            rd: PartBuilder::new(
                fpart.device.clone(),
                fpart.actual_family.clone(),
                Source::Vivado,
                td.width,
                td.height,
            ),
        }),
        bar: ProgressBar::new((td.width as u64) * (td.height as u64)),
    };
    let gys: Vec<_> = (0..td.height).collect();
    let chunks: Vec<_> = gys.chunks(TILE_BATCH_SIZE / (td.width as usize)).collect();
    chunks.into_par_iter().for_each(|batch| {
        let mut crdlist = String::new();
        for y in batch {
            writeln!(crdlist, "{y}").unwrap();
        }
        let tr = ToolchainReader::new(
            tc,
            "vivado",
            &[
                "-nolog",
                "-nojournal",
                "-mode",
                "batch",
                "-source",
                "script.tcl",
                "-tclargs",
                &fpart.name,
            ],
            &[],
            "tiles.fifo",
            &[
                ("script.tcl", DUMP_TILES_TCL.as_bytes()),
                ("crd.list", crdlist.as_bytes()),
            ],
        )
        .unwrap();
        let mut lines = tr.lines();
        loop {
            let l = lines.next().unwrap();
            let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
            match sl[0] {
                "TILE" => {
                    if sl.len() != 5 {
                        println!("UMMMM {} {:?}", fpart.name, sl);
                    }
                    let coord = Coord {
                        x: sl[1].parse().unwrap(),
                        y: td.height - 1 - sl[2].parse::<u16>().unwrap(),
                    };
                    dump_tile(&mut lines, coord, sl[3], sl[4], &ctx);
                }
                "END" => {
                    break;
                }
                _ => panic!("unknown line {}", sl[0]),
            }
        }
    });

    ctx.bar.finish();
    let mut mctx = ctx.mctx.into_inner().unwrap();

    // STEP 4: stream nodes
    for v in mctx.nodes.into_values() {
        mctx.rd.add_node(
            &v.into_iter()
                .map(|w| -> (&str, &str, Option<&str>) {
                    (
                        &mctx.names[w.tile],
                        &mctx.names[w.wire],
                        Some(&ctx.speed_models[&w.speed]),
                    )
                })
                .collect::<Vec<_>>(),
        );
    }

    // STEP 5: dump packages
    let mut packages = HashMap::new();
    for part in parts.iter() {
        if !packages.contains_key(&part.package) {
            packages.insert(part.package.clone(), part);
        }
    }
    let pkg_pins: Vec<_> = packages
        .into_par_iter()
        .map(|(pkg, part)| (pkg, dump_pkgpins(tc, part)))
        .collect();
    for (pkg, pins) in pkg_pins {
        mctx.rd.add_package(pkg, pins);
    }

    for part in parts {
        mctx.rd.add_combo(
            part.name.clone(),
            part.device.clone(),
            part.package.clone(),
            part.speed.clone(),
            part.temp.clone(),
        );
    }

    Ok(mctx.rd.finish())
}
