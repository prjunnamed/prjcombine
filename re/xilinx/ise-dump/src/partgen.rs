use prjcombine_re_xilinx_rawdump::PkgPin;
use prjcombine_re_toolchain::Toolchain;
use simple_error::bail;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;
use tempfile;

#[derive(Debug)]
pub struct PartgenPkg {
    pub family: String,
    pub device: String,
    pub package: String,
    pub speedgrades: Vec<String>,
    pub pins: Vec<PkgPin>,
}

fn parse_delay(d: &str) -> Result<Option<u32>, Box<dyn Error>> {
    if d == "N.A." {
        return Ok(None);
    }
    let d = d.parse::<f64>()? * 1000.0;
    Ok(Some(d.round() as u32))
}

fn parse_bank(d: &str) -> Result<Option<u32>, Box<dyn Error>> {
    if d == "-1" {
        return Ok(None);
    }
    Ok(Some(d.parse()?))
}

pub fn parse_pkgfile(f: &mut impl BufRead) -> Result<Vec<PkgPin>, Box<dyn Error>> {
    let mut res: Vec<PkgPin> = Vec::new();
    for l in f.lines() {
        let l = l?;
        if l.starts_with('#') {
            continue;
        }
        let l: Vec<_> = l.split_whitespace().collect();

        match l[..] {
            [typ, pad, pin] => {
                if typ != "pin" && typ != "pkgpin" {
                    continue;
                }
                res.push(PkgPin {
                    pad: if typ == "pin" {
                        Some(pad.to_string())
                    } else {
                        None
                    },
                    pin: pin.to_string(),
                    vref_bank: None,
                    vcco_bank: None,
                    func: "IO".to_string(),
                    tracelen_um: None,
                    delay_min_fs: None,
                    delay_max_fs: None,
                });
            }
            [typ, pad, pin, bank, func, _, _] => {
                if typ != "pin" && typ != "pkgpin" {
                    continue;
                }
                res.push(PkgPin {
                    pad: if typ == "pin" {
                        Some(pad.to_string())
                    } else {
                        None
                    },
                    pin: pin.to_string(),
                    vref_bank: parse_bank(bank)?,
                    vcco_bank: parse_bank(bank)?,
                    func: func.to_string(),
                    tracelen_um: None,
                    delay_min_fs: None,
                    delay_max_fs: None,
                });
            }
            [typ, pad, pin, vref_bank, vcco_bank, func, _, _, tracelen] => {
                if typ != "pin" && typ != "pkgpin" {
                    continue;
                }
                let tracelen: Option<u32> = if tracelen == "N.A." {
                    None
                } else {
                    Some(tracelen.parse()?)
                };
                res.push(PkgPin {
                    pad: if typ == "pin" {
                        Some(pad.to_string())
                    } else {
                        None
                    },
                    pin: pin.to_string(),
                    vref_bank: parse_bank(vref_bank)?,
                    vcco_bank: parse_bank(vcco_bank)?,
                    func: func.to_string(),
                    tracelen_um: tracelen,
                    delay_min_fs: None,
                    delay_max_fs: None,
                });
            }
            [typ, pad, pin, vref_bank, vcco_bank, func, _, _, delay_min, delay_max] => {
                if typ != "pin" && typ != "pkgpin" {
                    continue;
                }
                res.push(PkgPin {
                    pad: if typ == "pin" {
                        Some(pad.to_string())
                    } else {
                        None
                    },
                    pin: pin.to_string(),
                    vref_bank: parse_bank(vref_bank)?,
                    vcco_bank: parse_bank(vcco_bank)?,
                    func: func.to_string(),
                    tracelen_um: None,
                    delay_min_fs: parse_delay(delay_min)?,
                    delay_max_fs: parse_delay(delay_max)?,
                });
            }
            _ => (),
        }
    }
    Ok(res)
}

use regex::Regex;

const PATTERNS: &[(&str, &str, &str)] = &[
    ("x[ca]95[0-9]+", "[a-z]{2}[0-9]+", "xc9500"),
    ("x[ca]95[0-9]+xl", "[a-z]{2}[0-9]+", "xc9500xl"),
    ("x[ca]95[0-9]+xv?", "[a-z]{2}[0-9]+", "xc9500xv"),
    ("xcr3[0-9]+xl", "[a-z]{2}[0-9]+", "xpla3"),
    ("x[ca]2c[0-9]+a?", "[a-z]{2}g?[0-9]+", "xbr"),
    ("xc3[01][0-9]+[al]", "[a-z]{2}[0-9]+", "xc3000a"),
    ("xc40[0-9]+[el]", "[a-z]{2}[0-9]+", "xc4000e"),
    ("xcs[0-9]+", "[a-z]{2}[0-9]+", "xc4000e"),
    ("xc40[0-9]+(?:xl|ex)", "[a-z]{2}[0-9]+", "xc4000ex"),
    ("xc40[0-9]+xla", "[a-z]{2}[0-9]+", "xc4000xla"),
    ("xc40[0-9]+xv", "[a-z]{2}[0-9]+", "xc4000xv"),
    ("xcs[0-9]+xl", "[a-z]{2}[0-9]+", "spartanxl"),
    ("xc52[0-9]+", "[a-z]{2}[0-9]+", "xc5200"),
    ("x(?:cv|qv|qvr|c2s)[0-9]+", "[a-z]{2}[0-9]+", "virtex"),
    ("x(?:cv|qv|c2s|a2s)[0-9]+e", "[a-z]{2}[0-9]+", "virtexe"),
    ("x(?:c|q|qr)2v[0-9]+", "[a-z]{2}[0-9]+", "virtex2"),
    ("x[cq]2vpx?[0-9]+", "[a-z]{2}[0-9]+", "virtex2p"),
    ("xc3s[0-9]+l?", "[a-z]{2}[0-9]+", "spartan3"),
    ("xa3s[0-9]+l?", "[a-z]{2}g[0-9]+", "spartan3"),
    ("xcexf[0-9]+", "die", "fpgacore"),
    ("xc3s[0-9]+e", "[a-z]{2}[0-9]+", "spartan3e"),
    ("xa3s[0-9]+e", "[a-z]{2}g[0-9]+", "spartan3e"),
    ("xc3s[0-9]+a", "[a-z]{2}[0-9]+", "spartan3a"),
    ("xc3s[0-9]+an", "[a-z]{2}g[0-9]+", "spartan3a"),
    ("xa3s[0-9]+a", "[a-z]{2}g[0-9]+", "spartan3a"),
    ("xc3sd[0-9]+a", "[a-z]{2}[0-9]+", "spartan3adsp"),
    ("xa3sd[0-9]+a", "[a-z]{2}g[0-9]+", "spartan3adsp"),
    (
        "x[cqa]6slx[0-9](?:[0-9]+t?|)l?",
        "[a-z]{2}g?[0-9]+",
        "spartan6",
    ),
    ("x(?:c|q|qr)4v[lsf]x[0-9]+", "[a-z]{2}[0-9]+", "virtex4"),
    ("x[cq]5v[lsft]x[0-9]+t?", "[a-z]{2}[0-9]+", "virtex5"),
    ("x[cq]6v[lshc]x[0-9]+t?l?", "[a-z]{2}g?[0-9]+", "virtex6"),
    (
        "x[ca]7(?:[akvz]|v[xh])[0-9]+[st]?[li]?",
        "[a-z]{2}[gv][0-9]+",
        "virtex7",
    ),
    (
        "xq7(?:[akv]|v[xh])[0-9]+t?[li]?",
        "[a-z]{2}[0-9]+",
        "virtex7",
    ),
    ("x[ca]7s[0-9]+", "[a-z]{2}[gv][a-z][0-9]+", "virtex7"),
    ("xq7z[0-9]+", "[a-z]{2}g?[0-9]+", "virtex7"),
];

pub fn split_partname(s: &str) -> Option<(&str, &str, &str)> {
    for (dpat, ppat, fam) in PATTERNS {
        let re = Regex::new(&("^(".to_string() + dpat + ")(" + ppat + ")$")).unwrap();
        if let Some(cap) = re.captures(s) {
            let dev = cap.get(1).unwrap();
            let pkg = cap.get(2).unwrap();
            assert!(dev.start() == 0);
            assert!(dev.end() == pkg.start());
            assert!(pkg.end() == s.len());
            let m = dev.end();
            return Some((&s[..m], &s[m..], fam));
        }
    }
    None
}

pub fn get_pkgs(tc: &Toolchain, query: &str) -> Result<Vec<PartgenPkg>, Box<dyn Error>> {
    let dir = tempfile::Builder::new()
        .prefix("prjcombine_ise_dump_partgen")
        .tempdir()?;
    let mut cmd = tc.command("partgen");
    cmd.current_dir(dir.path().as_os_str());
    cmd.stdin(Stdio::null());
    cmd.arg("-v");
    if !query.is_empty() {
        cmd.arg(query);
    }
    let status = cmd.output()?;
    if !status.status.success() {
        let _ = std::io::stderr().write_all(&status.stdout);
        let _ = std::io::stderr().write_all(&status.stderr);
        bail!("non-zero partgen exit status");
    }
    let file = File::open(dir.path().join("partlist.xct"))?;
    let bufread = BufReader::new(file);
    let mut lines = bufread.lines();
    let mut res: Vec<PartgenPkg> = Vec::new();
    let mut fix_speed = false;
    loop {
        let l = match lines.next() {
            None => break,
            Some(l) => l?,
        };
        let mut sl: &str = &l;
        let mut cont = false;
        if let Some(x) = l.strip_suffix('\\') {
            cont = true;
            sl = x;
        }
        let words = sl.split_whitespace().collect::<Vec<_>>();
        if words.len() < 4 {
            bail!("first line too short: {}", l);
        }
        if !words[0].starts_with("part") {
            bail!("does not start with part: {}", l);
        }
        let mut part = words[1].to_lowercase();
        if !part.starts_with('x') {
            part = format!("xc{part}");
        }
        let (device, package, family) = match split_partname(&part) {
            None => bail!("cannot parse part name: {}", part),
            Some((d, p, f)) => (d.to_string(), p.to_string(), f.to_string()),
        };
        let mut speedgrades: Vec<String> = Vec::new();
        while cont {
            let l = match lines.next() {
                None => bail!("part definition cut off"),
                Some(l) => l?,
            };
            let mut sl: &str = &l;
            cont = false;
            if let Some(x) = l.strip_suffix('\\') {
                cont = true;
                sl = x;
            }
            if let Some(x) = sl.strip_prefix("\tSPEEDGRADE=") {
                let x: Vec<_> = x.split_whitespace().collect();
                if x.is_empty() {
                    bail!("empty speedgrade".to_string());
                }
                speedgrades.push(x[0].to_string());
            }
        }
        if speedgrades.is_empty() {
            fix_speed = true;
        }
        let pfn = if matches!(words[3], "9500.pkg" | "XBR.pkg" | "XPLA3.pkg") {
            format!("{part}.pkg")
        } else {
            words[3].to_string()
        };
        let pfile = File::open(dir.path().join(pfn))?;
        let mut pbufread = BufReader::new(pfile);
        let pins = parse_pkgfile(&mut pbufread)?;
        res.push(PartgenPkg {
            family,
            device,
            package,
            speedgrades,
            pins,
        });
    }
    if fix_speed {
        let mut cmd = tc.command("partgen");
        cmd.current_dir(dir.path().as_os_str());
        cmd.stdin(Stdio::null());
        cmd.arg("-arch");
        cmd.arg(query);
        let status = cmd.output()?;
        if !status.status.success() {
            let _ = std::io::stderr().write_all(&status.stdout);
            let _ = std::io::stderr().write_all(&status.stderr);
            bail!("non-zero partgen exit status");
        }
        let mut parts = HashMap::new();
        for l in std::str::from_utf8(&status.stdout)?.lines() {
            if l.contains("SPEEDS:") {
                let l: Vec<_> = l.split_ascii_whitespace().collect();
                if l[1] != "SPEEDS:" {
                    bail!("weird speed list");
                }
                parts.insert(l[0], l[2..].to_vec());
            }
        }
        for pkg in &mut res {
            if pkg.speedgrades.is_empty() {
                pkg.speedgrades = parts[&*pkg.device].iter().map(|x| x.to_string()).collect();
            }
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    #[test]
    fn split_partname_test() {
        assert_eq!(
            super::split_partname("xc6slx9tqg144"),
            Some(("xc6slx9", "tqg144", "spartan6"))
        );
        assert_eq!(
            super::split_partname("xq6slx75tcs484"),
            Some(("xq6slx75t", "cs484", "spartan6"))
        );
    }
}
