use std::{collections::BTreeMap, path::Path};

pub fn get_pkg(xact: &Path, pkg: &str) -> BTreeMap<String, String> {
    let path = xact.join(format!("xact/data/{pkg}.pkg"));
    let data = std::fs::read_to_string(path).unwrap();
    let mut res = BTreeMap::new();
    let mut comment = false;
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("/*") {
            comment = true;
        }
        if comment {
            if line.ends_with("*/") {
                comment = false;
            }
            continue;
        }
        let words = Vec::from_iter(line.split_ascii_whitespace());
        match &words[0].to_ascii_lowercase()[..] {
            "package" => {
                continue;
            }
            "pin" => {
                if words.len() == 2 {
                    continue;
                }
                assert_eq!(words.len(), 3);
                res.insert(words[2].to_string(), words[1].to_string());
            }
            w => panic!("umm {w}?"),
        }
    }
    res
}
