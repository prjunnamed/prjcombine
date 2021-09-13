use std::io::{BufRead, Write};
use std::collections::HashMap;
use crate::toolchain::Toolchain;
use crate::toolreader::ToolchainReader;
use crate::error::Error;
use crate::stringpool::StringPool;
use crate::xilinx::rawdump::{Part, Source, Coord, TkSitePinDir, TkPipInversion, TkPipDirection, PkgPin};
use crate::xilinx::rdbuild::PartBuilder;
use super::parts::VivadoPart;

const TILE_BATCH_SIZE: usize = 4000;

const LIST_TILES_TCL: &str = r#"
link_design -part [lindex $argv 0]
set fd [open "tiles.fifo" w]
foreach x [get_tiles] {
    set gx [get_property GRID_POINT_X $x]
    set gy [get_property GRID_POINT_Y $x]
    set tt [get_property TYPE $x]
    puts $fd "TILE $gx $gy $x $tt"
}
foreach x [get_speed_models] {
    set idx [get_property SPEED_INDEX $x]
    puts $fd "SPEED #$idx #$x"
}
puts $fd "END"
"#;

const DUMP_PKGPINS_TCL: &str = r#"
link_design -part [lindex $argv 0]
set fd [open "pkgpins.fifo" w]
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

const DUMP_TTS_TCL: &str = r#"
link_design -part [lindex $argv 0]
set ifd [open "tts.list" r]
set fd [open "tts.fifo" w]
while { [gets $ifd tname] >= 0 } {
    set tile [get_tiles $tname]
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

const DUMP_TILES_TCL: &str = r#"
link_design -part [lindex $argv 0]
set ifd [open "tiles.list" r]
set fd [open "tiles.fifo" w]
while { [gets $ifd tname] >= 0 } {
    set tile [get_tiles $tname]
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
puts $fd "END"
"#;

fn parse_bool(s: &str) -> bool {
    match s {
        "0" => false,
        "1" => true,
        _ => panic!("weird bool {}", s),
    }
}

pub fn get_rawdump(tc: &Toolchain, parts: &[VivadoPart]) -> Result<Part, Error> {
    let fpart = &parts[0];

    // STEP 1: list tiles, gather list of tile types, get dimensions; also list speed models
    let mut tts: HashMap<String, String> = HashMap::new();
    let mut tile_names: Vec<String> = Vec::new();
    let mut width: u16 = 0;
    let mut height: u16 = 0;
    let mut speed_models: HashMap<u32, String> = HashMap::new();
    {
        let tr = ToolchainReader::new(tc, "vivado", &["-nolog", "-nojournal", "-mode", "batch", "-source", "script.tcl", "-tclargs", &fpart.name], &[], "tiles.fifo", &[("script.tcl", LIST_TILES_TCL.as_bytes())])?;
        let lines = tr.lines();
        let mut got_end = false;
        for l in lines {
            let l = l?;
            let sl: Vec<_> = l.split_whitespace().collect();
            match sl[0] {
                "END" => {
                    got_end = true;
                    break;
                },
                "TILE" => {
                    let gx: u16 = sl[1].parse()?;
                    let gy: u16 = sl[2].parse()?;
                    tile_names.push(sl[3].to_string());
                    if !tts.contains_key(sl[4]) {
                        tts.insert(sl[4].to_string(), sl[3].to_string());
                    }
                    if gy >= height {
                        height = gy + 1;
                    }
                    if gx >= width {
                        width = gx + 1;
                    }
                },
                "SPEED" => {
                    let idx: u32 = sl[1][1..].parse()?;
                    let name = &sl[2][1..];
                    if idx == 65535 {
                        continue;
                    }
                    if speed_models.contains_key(&idx) {
                        panic!("double speed model {}: {} {}", idx, name, speed_models.get(&idx).unwrap());
                    }
                    speed_models.insert(idx, name.to_string());
                },
                _ => panic!("unknown line {}", sl[0]),
            }
        }
        if !got_end {
            return Err(Error::ParseError("missing END in tiles".to_string()));
        }
        assert!((width as usize) * (height as usize) == tile_names.len());
    }
    println!("{}: {}×{} tiles, {} tts, {} SMs", fpart.device, width, height, tts.len(), speed_models.len());

    let mut rd = PartBuilder::new(fpart.device.clone(), fpart.actual_family.clone(), Source::Vivado, width, height);

    // STEP 2: dump TTs [pips]
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
    let mut tt_pips: HashMap<String, HashMap<String, TtPip>> = HashMap::new();
    {
        let mut tlist: Vec<u8> = Vec::new();
        for (_, tn) in tts {
            tlist.write_all(tn.as_bytes())?;
            tlist.write_all(b"\n")?;
        }
        let tr = ToolchainReader::new(tc, "vivado", &["-nolog", "-nojournal", "-mode", "batch", "-source", "script.tcl", "-tclargs", &fpart.name], &[], "tts.fifo", &[("script.tcl", DUMP_TTS_TCL.as_bytes()), ("tts.list", &tlist)])?;
        let lines = tr.lines();
        let mut got_end = false;
        let mut tile = "".to_string();
        let mut tt = "".to_string();
        let mut pips: Option<&mut HashMap<String, TtPip>> = None;
        for l in lines {
            let l = l?;
            let sl: Vec<_> = l.split_whitespace().collect();
            match sl[0] {
                "END" => {
                    got_end = true;
                    break;
                },
                "TILE" => {
                    tile = sl[2].to_string();
                    tt = sl[1].to_string();
                    tt_pips.insert(sl[1].to_string(), HashMap::new());
                    pips = Some(tt_pips.get_mut(sl[1]).unwrap());
                },
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
                    pips.as_mut().unwrap().insert(name.to_string(), TtPip {
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
                        }
                    });
                },
                _ => panic!("unknown line {}", sl[0]),
            }
        }
        if !got_end {
            return Err(Error::ParseError("missing END in TTs".to_string()));
        }
    }

    // STEP 3: dump tiles [sites, pip speed, wires], STREAM THE MOTHERFUCKER, gather nodes
    let mut node_sp = StringPool::new();
    let mut nodes: HashMap<String, Vec<(u32, u32, u32)>> = HashMap::new();
    for batch in tile_names.chunks(TILE_BATCH_SIZE) {
        let mut tlist: Vec<u8> = Vec::new();
        for t in batch {
            tlist.write_all(t.as_bytes())?;
            tlist.write_all(b"\n")?;
        }
        let tr = ToolchainReader::new(tc, "vivado", &["-nolog", "-nojournal", "-mode", "batch", "-source", "script.tcl", "-tclargs", &fpart.name], &[], "tiles.fifo", &[("script.tcl", DUMP_TILES_TCL.as_bytes()), ("tiles.list", &tlist)])?;
        let lines = tr.lines();
        let mut got_end = false;
        let mut tile: Option<String> = None;
        let mut wpref: String = String::new();
        let mut ppref: String = String::new();
        let mut tt: Option<String> = None;
        let mut coord: Option<Coord> = None;
        let mut wires: Vec<(String, u32)> = Vec::new();
        let mut pips: Vec<(&str, &str, bool, bool, bool, TkPipInversion, TkPipDirection, Option<&str>)> = Vec::new();
        let mut ttt_pips: Option<&HashMap<String, TtPip>> = None;
        let mut tile_n2w: HashMap<String, Vec<u32>> = HashMap::new();
        let mut site_pins: Vec<(String, TkSitePinDir, Option<String>, Option<&str>)> = Vec::new();
        let mut site: Option<(String, String)> = None;
        let mut spref: String = String::new();
        let mut sites: Vec<(String, String, Vec<(String, TkSitePinDir, Option<String>, Option<&str>)>)> = Vec::new();
        for l in lines {
            let l = l?;
            let sl: Vec<_> = l.split_whitespace().collect();
            match sl[0] {
                "TILE" => {
                    assert!(tile.is_none());
                    coord = Some(Coord {
                        x: sl[1].parse()?,
                        y: height - 1 - sl[2].parse::<u16>()?,
                    });
                    tile = Some(sl[3].to_string());
                    tt = Some(sl[4].to_string());
                    wpref = sl[3].to_string() + "/";
                    ppref = sl[3].to_string() + "/" + sl[4] + ".";
                    ttt_pips = Some(tt_pips.get(sl[4]).unwrap());
                },
                "SITE" => {
                    assert!(site.is_none());
                    site = Some((sl[1].to_string(), sl[2].to_string()));
                    spref = sl[1].to_string() + "/";
                },
                "SITEPIN" => {
                    assert!(!site.is_none());
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
                        Some(speed_models.get(&si.parse::<u32>().unwrap()).unwrap())
                    };
                    let wire: Option<String> = match tile_n2w.get(node) {
                        None => None,
                        Some(v) => {
                            let mut v = v.clone();
                            if v.len() > 1 && matches!(pin,
                                "DOUT" |
                                "SYSREF_OUT_NORTH_P" |
                                "SYSREF_OUT_SOUTH_P" |
                                "CLK_DISTR_OUT_NORTH" |
                                "CLK_DISTR_OUT_SOUTH" |
                                "T1_ALLOWED_SOUTH"
                            ) {
                                let suffix = format!("_{}", pin);
                                v.retain(|&n| node_sp.get(n).ends_with(&suffix));
                            }
                            if v.len() == 1 {
                                Some(node_sp.get(v[0]).to_string())
                            } else {
                                panic!("SITE PIN WIRE AMBIGUOUS {:?} {:?}", sl, v.iter().map(|&n| node_sp.get(n)).collect::<Vec<_>>());
                            }
                        }
                    };
                    site_pins.push((pin.to_string(), dir, wire, speed));
                },
                "ENDSITE" => {
                    let (sname, skind) = site.unwrap();
                    site = None;
                    sites.push((sname, skind, site_pins));
                    site_pins = Vec::new();
                },
                "WIRE" => {
                    assert!(!tile.is_none());
                    let name = sl[1].strip_prefix(&wpref).unwrap();
                    let si = sl[2].parse::<u32>().unwrap();
                    let node = &sl[3][1..];
                    wires.push((name.to_string(), si));
                    if !node.is_empty() {
                        let nwires = nodes.entry(node.to_string()).or_default();
                        nwires.push((node_sp.put(tile.as_ref().unwrap()), node_sp.put(name), si));
                        let n2w = tile_n2w.entry(node.to_string()).or_default();
                        n2w.push(node_sp.put(name));
                    }
                },
                "PIP" => {
                    assert!(!tile.is_none());
                    let name = sl[1][1..].strip_prefix(&ppref).unwrap();
                    let si = &sl[2][1..];
                    let pip = ttt_pips.unwrap().get(name).unwrap();
                    if pip.is_pseudo {
                        continue;
                    }
                    let speed: Option<&str> = if si.is_empty() {
                        None
                    } else {
                        Some(speed_models.get(&si.parse::<u32>().unwrap()).unwrap())
                    };
                    if pip.is_bidi {
                        pips.push((
                            &pip.wire_from,
                            &pip.wire_to,
                            pip.is_buf,
                            pip.is_excluded,
                            pip.is_test,
                            pip.inv,
                            TkPipDirection::BiFwd,
                            speed,
                        ));
                        pips.push((
                            &pip.wire_to,
                            &pip.wire_from,
                            pip.is_buf,
                            pip.is_excluded,
                            pip.is_test,
                            pip.inv,
                            TkPipDirection::BiBwd,
                            speed,
                        ));
                    } else {
                        pips.push((
                            &pip.wire_from,
                            &pip.wire_to,
                            pip.is_buf,
                            pip.is_excluded,
                            pip.is_test,
                            pip.inv,
                            TkPipDirection::Uni,
                            speed,
                        ));
                    }
                },
                "ENDTILE" => {
                    assert!(site.is_none());
                    assert!(!tile.is_none());
                    rd.add_tile(coord.unwrap(), tile.unwrap(), tt.unwrap(),
                        &sites.iter().map(|(n, t, p)| -> (&str, &str, _) {
                            (&n, &t, p.iter().map(|&(ref n, d, ref w, s)| -> (&str, TkSitePinDir, Option<&str>, Option<&str>) {
                                (n, d, w.as_ref().map(|s| &s[..]), s)
                            }).collect::<Vec<_>>())
                        }).collect::<Vec<_>>(),
                        &wires.iter().map(|(w, s)| -> (&str, Option<&str>) {
                            (w, Some(&speed_models.get(&s).unwrap()[..]))
                        }).collect::<Vec<_>>(),
                        &pips,
                    );
                    coord = None;
                    tile = None;
                    tt = None;
                    wires = Vec::new();
                    pips = Vec::new();
                    sites = Vec::new();
                    tile_n2w = HashMap::new();
                },
                "END" => {
                    assert!(tile.is_none());
                    got_end = true;
                    break;
                },
                _ => panic!("unknown line {}", sl[0]),
            }
        }
        if !got_end {
            return Err(Error::ParseError("missing END in tiles".to_string()));
        }
    }

    // STEP 4: stream nodes
    for (_, v) in nodes {
        rd.add_node(&v.into_iter().map(|(t, w, s)| -> (&str, &str, Option<&str>) {
            (node_sp.get(t), node_sp.get(w), Some(speed_models.get(&s).unwrap()))
        }).collect::<Vec<_>>());
    }

    // STEP 5: dump packages
    for part in parts.iter() {
        if rd.part.packages.contains_key(&part.package) {
            continue;
        }
        let mut pins: Vec<PkgPin> = Vec::new();
        let tr = ToolchainReader::new(tc, "vivado", &["-nolog", "-nojournal", "-mode", "batch", "-source", "script.tcl", "-tclargs", &part.name], &[], "pkgpins.fifo", &[("script.tcl", DUMP_PKGPINS_TCL.as_bytes())])?;
        let lines = tr.lines();
        let mut got_end = false;
        for l in lines {
            let l = l?;
            let sl: Vec<_> = l.split_whitespace().collect();
            match sl[0] {
                "END" => {
                    got_end = true;
                    break;
                },
                "PKGPIN" => {
                    let pin = sl[1];
                    let site = &sl[2][1..];
                    let bank = &sl[3][1..];
                    let func = &sl[4][1..];
                    let mind = &sl[5][1..];
                    let maxd = &sl[6][1..];
                    pins.push(PkgPin {
                        pad: if site.is_empty() { None } else { Some(site.to_string()) },
                        pin: pin.to_string(),
                        vref_bank: if bank.is_empty() { None } else { Some(bank.parse()?) },
                        vcco_bank: if bank.is_empty() { None } else { Some(bank.parse()?) },
                        func: func.to_string(),
                        tracelen_um: None,
                        delay_min_fs: if mind.is_empty() { None } else { Some(mind.parse()?) },
                        delay_max_fs: if maxd.is_empty() { None } else { Some(maxd.parse()?) },
                    });
                },
                _ => panic!("unknown line {}", sl[0]),
            }
        }
        if !got_end {
            return Err(Error::ParseError("missing END in tiles".to_string()));
        }
        rd.add_package(part.package.to_string(), pins);
    }

    for part in parts {
        rd.add_combo(part.name.clone(), part.device.clone(), part.package.clone(), part.speed.clone(), part.temp.clone());
    }

    Ok(rd.finish())
}
