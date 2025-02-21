use std::{collections::BTreeMap, path::Path};

use regex::Regex;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PartKind {
    Xc2000,
    Xc3000,
    Xc4000,
    Xc5200,
    Xc7000,
}

#[derive(Debug)]
pub struct Part {
    pub kind: PartKind,
    pub name: String,
    pub package: String,
    pub die_file: String,
    pub pkg_file: String,
    pub spd_file: String,
    pub kv: BTreeMap<String, Vec<String>>,
}

pub fn get_parts(xact: &Path) -> Vec<Part> {
    let path = xact.join("xact/data/partlist.xct");
    let data = std::fs::read_to_string(path).unwrap();
    let mut lines = data.lines();
    let mut res = vec![];
    let np_re = Regex::new(r"^([0-9]+[ahldq]?)([a-z][a-z][0-9]+)$").unwrap();
    while let Some(line) = lines.next() {
        let mut line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut cont = false;
        if let Some(r) = line.strip_suffix(" \\") {
            line = r;
            cont = true;
        }
        let mut words = line.split_ascii_whitespace();
        let kw = words.next().unwrap();
        let kind = match kw {
            "revision" | "alias" => {
                assert!(!cont);
                continue;
            }
            "part2000" => PartKind::Xc2000,
            "part3000" | "part3100" => PartKind::Xc3000,
            "part4000" => PartKind::Xc4000,
            "part5200" => PartKind::Xc5200,
            "part7000" => PartKind::Xc7000,
            _ => panic!("umm {kw}"),
        };
        let np = words.next().unwrap().to_ascii_lowercase();
        let cap = np_re.captures(&np).unwrap();
        let name = cap.get(1).unwrap().as_str().to_string();
        let package = cap.get(2).unwrap().as_str().to_string();
        let die_file = words
            .next()
            .unwrap()
            .strip_suffix(".die")
            .unwrap()
            .to_string();
        let pkg_file = words
            .next()
            .unwrap()
            .strip_suffix(".pkg")
            .unwrap()
            .to_string();
        let spd_file = words
            .next()
            .unwrap()
            .strip_prefix("SPEEDFILE=")
            .unwrap()
            .strip_suffix(".spd")
            .unwrap()
            .to_string();
        let mut kv: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for word in words {
            let (k, v) = word.split_once('=').unwrap();
            kv.entry(k.to_string()).or_default().push(v.to_string());
        }
        while cont {
            let mut line = lines.next().unwrap().trim();
            cont = false;
            if let Some(r) = line.strip_suffix(" \\") {
                line = r;
                cont = true;
            }
            for word in line.split_ascii_whitespace() {
                let (k, v) = word.split_once('=').unwrap();
                kv.entry(k.to_string()).or_default().push(v.to_string());
            }
        }
        res.push(Part {
            kind,
            name,
            package,
            die_file,
            pkg_file,
            spd_file,
            kv,
        })
    }
    res
}
