use super::parts::VivadoPart;
use indicatif::ProgressBar;
use prjcombine_rawdump::{
    Coord, Part, PkgPin, Source, TileKindId, TkPipDirection, TkPipInversion, TkSite, TkSitePin,
    TkSitePinDir, TkSiteSlot,
};
use prjcombine_rdbuild::{PartBuilder, PbPip, PbSitePin};
use prjcombine_toolchain::{Toolchain, ToolchainReader};
use rayon::prelude::*;
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use std::fmt::Write as FmtWrite;
use std::io::{BufRead, Write};
use std::sync::Mutex;
use unnamed_entity::{entity_id, EntitySet};

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
        _ => panic!("weird bool {s}"),
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
                    _ => panic!("unk pip dirbuf {name} {dir} {buf0} {buf1}"),
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
                            _ => panic!("unk inversion {invfix} {invcan}"),
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
                                    let suffix = format!("_{pin}");
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
    let mut kill_wires = Vec::new();
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
                } else {
                    kill_wires.push(name.to_string());
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
                        println!("UMMMM NO SI {sl:?}");
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
                for name in kill_wires {
                    mctx.rd.kill_wire(tile, &name);
                }
                ctx.bar.inc(1);
                return;
            }
            _ => panic!("unknown line {}", sl[0]),
        }
    }
}

const STEAL_SITES_TCL: &str = r#"
set ifd [open "sites.list" r]
set fd [open "sites.fifo" w]
link_design -part [lindex $argv 0]
foreach x [get_speed_models] {
    set idx [get_property SPEED_INDEX $x]
    puts $fd "SPEED #$idx #$x"
}
while { [gets $ifd sname] >= 0 } {
    set site [get_sites $sname]
    set type [get_property SITE_TYPE $site]
    set tile [get_tiles -of $site]
    puts $fd "SITE $sname #$site $type #$tile"
    foreach y [get_site_pins -of $site] {
        set node [get_nodes -of $y]
        set dir [get_property DIRECTION $y]
        set si [get_property SPEED_INDEX $y]
        puts $fd "SITEPIN #$y #$dir #$si #$node"
        foreach w [get_wires -of $node] {
            puts $fd "WIRE #$w"
        }
        puts $fd "ENDSITEPIN"
    }
    puts $fd "ENDSITE"
}
puts $fd "END"
"#;

fn steal_sites(
    tc: &Toolchain,
    part: &mut Part,
    device: &str,
    slots: HashMap<&str, (TileKindId, TkSiteSlot)>,
) {
    let mut slist: Vec<u8> = Vec::new();
    for site in slots.keys() {
        writeln!(slist, "{site}").unwrap();
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
            device,
        ],
        &[],
        "sites.fifo",
        &[
            ("script.tcl", STEAL_SITES_TCL.as_bytes()),
            ("sites.list", &slist),
        ],
    )
    .unwrap();
    let mut lines = tr.lines();
    let mut speed_models = HashMap::new();
    loop {
        let l = lines.next().unwrap();
        let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
        match sl[0] {
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
            "SITE" => {
                let name = sl[1];
                assert_eq!(name, &sl[2][1..]);
                let kind = sl[3].to_string();
                let tile = &sl[4][1..];
                let (tki, slot) = slots[name];
                let mut site = TkSite {
                    kind,
                    pins: HashMap::new(),
                };
                let spref = format!("{name}/");
                let tpref = format!("{tile}/");
                loop {
                    let l = lines.next().unwrap();
                    let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
                    match sl[0] {
                        "SITEPIN" => {
                            let pin = sl[1][1..].strip_prefix(&spref).unwrap().to_string();
                            let dir = match &sl[2][1..] {
                                "IN" => TkSitePinDir::Input,
                                "OUT" => TkSitePinDir::Output,
                                "INOUT" => TkSitePinDir::Bidir,
                                _ => panic!("weird pin dir {}", &sl[2][1..]),
                            };
                            let si = &sl[3][1..];
                            let node = &sl[4][1..];
                            let speed = if si.is_empty() {
                                None
                            } else {
                                Some(
                                    part.speeds
                                        .get_or_insert(&speed_models[&si.parse::<u32>().unwrap()]),
                                )
                            };

                            let mut wires = Vec::new();
                            loop {
                                let l = lines.next().unwrap();
                                let sl: Vec<_> = l.as_ref().unwrap().split_whitespace().collect();
                                match sl[0] {
                                    "WIRE" => {
                                        if let Some(wname) = sl[1][1..].strip_prefix(&tpref) {
                                            wires.push(wname.to_string());
                                        }
                                    }
                                    "ENDSITEPIN" => break,
                                    _ => panic!("unknown line {}", sl[0]),
                                }
                            }
                            let mut nwires = wires.clone();
                            if nwires.len() > 1 {
                                nwires.retain(|x| x.ends_with(&pin));
                            }
                            if nwires.len() > 1 {
                                panic!(
                                    "AMBIG PIN {name} {pin} {dir:?} {speed:?} {node:?} {wires:?}"
                                );
                            }
                            let wire = if nwires.is_empty() {
                                None
                            } else {
                                Some(part.wires.get_or_insert(&nwires[0]))
                            };
                            site.pins.insert(pin, TkSitePin { dir, wire, speed });
                        }
                        "ENDSITE" => break,
                        _ => panic!("unknown line {}", sl[0]),
                    }
                }
                let tk = &mut part.tile_kinds[tki];
                tk.sites.insert(slot, site);
            }
            "END" => break,
            _ => panic!("unknown line {}", sl[0]),
        }
    }
}

struct FixupSiteSlot {
    slot_name: &'static str,
    slot_x: u8,
    slot_y: u8,
    source_site: &'static str,
}

struct FixupTileKind {
    family: &'static str,
    tile_kind: &'static str,
    slots: &'static [FixupSiteSlot],
    source_device: &'static str,
}

const FIXUP_TILE_KINDS: &[FixupTileKind] = &[
    FixupTileKind {
        family: "virtex7",
        tile_kind: "GTP_CHANNEL_0",
        slots: &[
            FixupSiteSlot {
                slot_name: "GTPE2_CHANNEL",
                slot_x: 0,
                slot_y: 0,
                source_site: "GTPE2_CHANNEL_X0Y0",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "IPAD_X1Y0",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "IPAD_X1Y1",
            },
            FixupSiteSlot {
                slot_name: "OPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "OPAD_X0Y0",
            },
            FixupSiteSlot {
                slot_name: "OPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "OPAD_X0Y1",
            },
        ],
        source_device: "xc7a25t",
    },
    FixupTileKind {
        family: "virtex7",
        tile_kind: "GTP_CHANNEL_1",
        slots: &[
            FixupSiteSlot {
                slot_name: "GTPE2_CHANNEL",
                slot_x: 0,
                slot_y: 0,
                source_site: "GTPE2_CHANNEL_X0Y1",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "IPAD_X1Y6",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "IPAD_X1Y7",
            },
            FixupSiteSlot {
                slot_name: "OPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "OPAD_X0Y2",
            },
            FixupSiteSlot {
                slot_name: "OPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "OPAD_X0Y3",
            },
        ],
        source_device: "xc7a25t",
    },
    FixupTileKind {
        family: "virtex7",
        tile_kind: "GTP_CHANNEL_2",
        slots: &[
            FixupSiteSlot {
                slot_name: "GTPE2_CHANNEL",
                slot_x: 0,
                slot_y: 0,
                source_site: "GTPE2_CHANNEL_X0Y2",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "IPAD_X1Y24",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "IPAD_X1Y25",
            },
            FixupSiteSlot {
                slot_name: "OPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "OPAD_X0Y4",
            },
            FixupSiteSlot {
                slot_name: "OPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "OPAD_X0Y5",
            },
        ],
        source_device: "xc7a25t",
    },
    FixupTileKind {
        family: "virtex7",
        tile_kind: "GTP_CHANNEL_3",
        slots: &[
            FixupSiteSlot {
                slot_name: "GTPE2_CHANNEL",
                slot_x: 0,
                slot_y: 0,
                source_site: "GTPE2_CHANNEL_X0Y3",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "IPAD_X1Y30",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "IPAD_X1Y31",
            },
            FixupSiteSlot {
                slot_name: "OPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "OPAD_X0Y6",
            },
            FixupSiteSlot {
                slot_name: "OPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "OPAD_X0Y7",
            },
        ],
        source_device: "xc7a25t",
    },
    FixupTileKind {
        family: "virtex7",
        tile_kind: "GTP_COMMON",
        slots: &[
            FixupSiteSlot {
                slot_name: "GTPE2_COMMON",
                slot_x: 0,
                slot_y: 0,
                source_site: "GTPE2_COMMON_X0Y0",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 0,
                source_site: "IPAD_X1Y8",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 1,
                source_site: "IPAD_X1Y9",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 2,
                source_site: "IPAD_X1Y10",
            },
            FixupSiteSlot {
                slot_name: "IPAD",
                slot_x: 0,
                slot_y: 3,
                source_site: "IPAD_X1Y11",
            },
            FixupSiteSlot {
                slot_name: "IBUFDS_GTE2",
                slot_x: 0,
                slot_y: 0,
                source_site: "IBUFDS_GTE2_X0Y0",
            },
            FixupSiteSlot {
                slot_name: "IBUFDS_GTE2",
                slot_x: 0,
                slot_y: 1,
                source_site: "IBUFDS_GTE2_X0Y1",
            },
        ],
        source_device: "xc7a25t",
    },
    FixupTileKind {
        family: "virtex7",
        tile_kind: "PCIE_BOT",
        slots: &[FixupSiteSlot {
            slot_name: "PCIE",
            slot_x: 0,
            slot_y: 0,
            source_site: "PCIE_X0Y0",
        }],
        source_device: "xc7a25t",
    },
    FixupTileKind {
        family: "ultrascale",
        tile_kind: "LAGUNA_TILE",
        slots: &[
            FixupSiteSlot {
                slot_name: "LAGUNA",
                slot_x: 0,
                slot_y: 0,
                source_site: "LAGUNA_X0Y240",
            },
            FixupSiteSlot {
                slot_name: "LAGUNA",
                slot_x: 1,
                slot_y: 0,
                source_site: "LAGUNA_X1Y240",
            },
            FixupSiteSlot {
                slot_name: "LAGUNA",
                slot_x: 0,
                slot_y: 1,
                source_site: "LAGUNA_X0Y241",
            },
            FixupSiteSlot {
                slot_name: "LAGUNA",
                slot_x: 1,
                slot_y: 1,
                source_site: "LAGUNA_X1Y241",
            },
        ],
        source_device: "xcku115-flva1517-1-c",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "CMAC",
        slots: &[FixupSiteSlot {
            slot_name: "CMACE4",
            slot_x: 0,
            slot_y: 0,
            source_site: "CMACE4_X0Y0",
        }],
        source_device: "xcku5p-ffva676-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "LAG_LAG",
        slots: &[
            FixupSiteSlot {
                slot_name: "LAGUNA",
                slot_x: 0,
                slot_y: 0,
                source_site: "LAGUNA_X0Y240",
            },
            FixupSiteSlot {
                slot_name: "LAGUNA",
                slot_x: 1,
                slot_y: 0,
                source_site: "LAGUNA_X1Y240",
            },
            FixupSiteSlot {
                slot_name: "LAGUNA",
                slot_x: 0,
                slot_y: 1,
                source_site: "LAGUNA_X0Y241",
            },
            FixupSiteSlot {
                slot_name: "LAGUNA",
                slot_x: 1,
                slot_y: 1,
                source_site: "LAGUNA_X1Y241",
            },
        ],
        source_device: "xcvu7p-flva2104-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "GTM_DUAL_RIGHT_FT",
        slots: &[
            FixupSiteSlot {
                slot_name: "BUFG_GT_SYNC",
                slot_x: 0,
                slot_y: 6,
                source_site: "BUFG_GT_SYNC_X1Y6",
            },
            FixupSiteSlot {
                slot_name: "BUFG_GT_SYNC",
                slot_x: 0,
                slot_y: 13,
                source_site: "BUFG_GT_SYNC_X1Y13",
            },
            FixupSiteSlot {
                slot_name: "ABUS_SWITCH",
                slot_x: 0,
                slot_y: 0,
                source_site: "ABUS_SWITCH_X10Y1",
            },
            FixupSiteSlot {
                slot_name: "ABUS_SWITCH",
                slot_x: 0,
                slot_y: 1,
                source_site: "ABUS_SWITCH_X10Y2",
            },
            FixupSiteSlot {
                slot_name: "ABUS_SWITCH",
                slot_x: 0,
                slot_y: 2,
                source_site: "ABUS_SWITCH_X10Y3",
            },
            FixupSiteSlot {
                slot_name: "ABUS_SWITCH",
                slot_x: 0,
                slot_y: 3,
                source_site: "ABUS_SWITCH_X10Y4",
            },
            FixupSiteSlot {
                slot_name: "ABUS_SWITCH",
                slot_x: 0,
                slot_y: 4,
                source_site: "ABUS_SWITCH_X10Y5",
            },
            FixupSiteSlot {
                slot_name: "GTM_DUAL",
                slot_x: 0,
                slot_y: 0,
                source_site: "GTM_DUAL_X1Y0",
            },
            FixupSiteSlot {
                slot_name: "GTM_REFCLK",
                slot_x: 0,
                slot_y: 0,
                source_site: "GTM_REFCLK_X1Y0",
            },
        ],
        source_device: "xcvu27p-figd2104-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "GTM_DUAL_LEFT_FT",
        slots: &[
            FixupSiteSlot {
                slot_name: "BUFG_GT_SYNC",
                slot_x: 0,
                slot_y: 6,
                source_site: "BUFG_GT_SYNC_X0Y6",
            },
            FixupSiteSlot {
                slot_name: "BUFG_GT_SYNC",
                slot_x: 0,
                slot_y: 13,
                source_site: "BUFG_GT_SYNC_X0Y13",
            },
        ],
        source_device: "xcvu27p-figd2104-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "HSADC_HSADC_RIGHT_FT",
        slots: &[FixupSiteSlot {
            slot_name: "HSADC",
            slot_x: 0,
            slot_y: 0,
            source_site: "HSADC_X0Y0",
        }],
        source_device: "xczu28dr-ffve1156-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "HSDAC_HSDAC_RIGHT_FT",
        slots: &[FixupSiteSlot {
            slot_name: "HSDAC",
            slot_x: 0,
            slot_y: 0,
            source_site: "HSDAC_X0Y0",
        }],
        source_device: "xczu28dr-ffve1156-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "PSS_ALTO",
        slots: &[FixupSiteSlot {
            slot_name: "PS8",
            slot_x: 0,
            slot_y: 0,
            source_site: "PS8_X0Y0",
        }],
        source_device: "xczu9eg-ffvb1156-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "VCU_VCU_FT",
        slots: &[FixupSiteSlot {
            slot_name: "VCU",
            slot_x: 0,
            slot_y: 0,
            source_site: "VCU_X0Y0",
        }],
        source_device: "xczu5ev-fbvb900-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "FE_FE_FT",
        slots: &[FixupSiteSlot {
            slot_name: "FE",
            slot_x: 0,
            slot_y: 0,
            source_site: "FE_X0Y0",
        }],
        source_device: "xczu28dr-ffve1156-1-e",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "DFE_DFE_TILEA_FT",
        slots: &[FixupSiteSlot {
            slot_name: "DFE_A",
            slot_x: 0,
            slot_y: 0,
            source_site: "DFE_A_X0Y0",
        }],
        source_device: "xczu67dr-ffve1156-1-i",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "DFE_DFE_TILEB_FT",
        slots: &[FixupSiteSlot {
            slot_name: "DFE_B",
            slot_x: 0,
            slot_y: 0,
            source_site: "DFE_B_X0Y0",
        }],
        source_device: "xczu67dr-ffve1156-1-i",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "DFE_DFE_TILEC_FT",
        slots: &[FixupSiteSlot {
            slot_name: "DFE_C",
            slot_x: 0,
            slot_y: 0,
            source_site: "DFE_C_X0Y0",
        }],
        source_device: "xczu67dr-ffve1156-1-i",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "DFE_DFE_TILED_FT",
        slots: &[FixupSiteSlot {
            slot_name: "DFE_D",
            slot_x: 0,
            slot_y: 0,
            source_site: "DFE_D_X0Y0",
        }],
        source_device: "xczu67dr-ffve1156-1-i",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "DFE_DFE_TILEE_FT",
        slots: &[FixupSiteSlot {
            slot_name: "DFE_E",
            slot_x: 0,
            slot_y: 0,
            source_site: "DFE_E_X0Y0",
        }],
        source_device: "xczu67dr-ffve1156-1-i",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "DFE_DFE_TILEF_FT",
        slots: &[FixupSiteSlot {
            slot_name: "DFE_F",
            slot_x: 0,
            slot_y: 0,
            source_site: "DFE_F_X0Y0",
        }],
        source_device: "xczu67dr-ffve1156-1-i",
    },
    FixupTileKind {
        family: "ultrascaleplus",
        tile_kind: "DFE_DFE_TILEG_FT",
        slots: &[FixupSiteSlot {
            slot_name: "DFE_G",
            slot_x: 0,
            slot_y: 0,
            source_site: "DFE_G_X0Y0",
        }],
        source_device: "xczu67dr-ffve1156-1-i",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "PCIEB_BOT_TILE",
        slots: &[FixupSiteSlot {
            slot_name: "PCIE40",
            slot_x: 0,
            slot_y: 0,
            source_site: "PCIE40_X0Y1",
        }],
        source_device: "xcvc1902-viva1596-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "PCIEB_TOP_TILE",
        slots: &[FixupSiteSlot {
            slot_name: "PCIE40",
            slot_x: 0,
            slot_y: 0,
            source_site: "PCIE40_X0Y2",
        }],
        source_device: "xcvc1902-viva1596-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "MRMAC_BOT_TILE",
        slots: &[FixupSiteSlot {
            slot_name: "MRMAC",
            slot_x: 0,
            slot_y: 0,
            source_site: "MRMAC_X0Y0",
        }],
        source_device: "xcvc1902-viva1596-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "PCIEB5_TOP_TILE",
        slots: &[FixupSiteSlot {
            slot_name: "PCIE50",
            slot_x: 0,
            slot_y: 0,
            source_site: "PCIE50_X0Y1",
        }],
        source_device: "xcvp1202-vsva2785-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "DCMAC_TILE",
        slots: &[FixupSiteSlot {
            slot_name: "DCMAC",
            slot_x: 0,
            slot_y: 0,
            source_site: "DCMAC_X0Y0",
        }],
        source_device: "xcvp1202-vsva2785-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "HSC_TILE",
        slots: &[FixupSiteSlot {
            slot_name: "HSC",
            slot_x: 0,
            slot_y: 0,
            source_site: "HSC_X0Y0",
        }],
        source_device: "xcvp1202-vsva2785-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "GTYP_QUAD_SINGLE",
        slots: &[FixupSiteSlot {
            slot_name: "GTYP_QUAD",
            slot_x: 0,
            slot_y: 0,
            source_site: "GTYP_QUAD_X1Y0",
        }],
        source_device: "xcvp1202-vsva2785-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "GTYP_QUAD_SINGLE",
        slots: &[FixupSiteSlot {
            slot_name: "GTYP_REFCLK",
            slot_x: 0,
            slot_y: 0,
            source_site: "GTYP_REFCLK_X1Y0",
        }],
        source_device: "xcvp1202-vsva2785-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "GTYP_QUAD_SINGLE",
        slots: &[FixupSiteSlot {
            slot_name: "GTYP_REFCLK",
            slot_x: 0,
            slot_y: 1,
            source_site: "GTYP_REFCLK_X1Y1",
        }],
        source_device: "xcvp1202-vsva2785-1LHP-i-L",
    },
    FixupTileKind {
        family: "versal",
        tile_kind: "HDIO_TILE",
        slots: &[FixupSiteSlot {
            slot_name: "DPLL",
            slot_x: 0,
            slot_y: 0,
            source_site: "DPLL_X3Y7",
        }],
        source_device: "xcvc1702-nsvg1369-1LHP-i-L",
    },
];

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

    // ... apparently some tiles are now just straight up missing. sigh.
    for x in 0..mctx.rd.part.width {
        for y in 0..mctx.rd.part.height {
            let crd = Coord { x, y };
            if !mctx.rd.part.tiles.contains_key(&crd) {
                mctx.rd
                    .add_tile(crd, format!("__{x}_{y}__"), "__EMPTY__".into(), &[], &[], &[]);
            }
        }
    }

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

    // STEP 5: add missing site slots to some tiles, by cheating and pulling them from other
    // devices
    let mut fixup_slots: HashMap<&str, HashMap<_, _>> = HashMap::new();
    for ft in FIXUP_TILE_KINDS {
        if ft.family != mctx.rd.part.family {
            continue;
        }
        let Some((tki, tk)) = mctx.rd.part.tile_kinds.get(ft.tile_kind) else {
            continue;
        };
        for fs in ft.slots {
            let sk = mctx.rd.part.slot_kinds.get_or_insert(fs.slot_name);
            let slot = TkSiteSlot::Xy(sk, fs.slot_x, fs.slot_y);
            if !tk.sites.contains_key(&slot) {
                fixup_slots
                    .entry(ft.source_device)
                    .or_default()
                    .insert(fs.source_site, (tki, slot));
            }
        }
        if fixup_slots.is_empty() {
            continue;
        }
        println!(
            "FIXUP {} {} from {}",
            mctx.rd.part.part, ft.tile_kind, ft.source_device
        );
    }
    for (k, v) in fixup_slots {
        steal_sites(tc, &mut mctx.rd.part, k, v);
    }

    // STEP 6: dump packages
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
