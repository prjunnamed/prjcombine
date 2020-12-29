pub mod xdlrc;
pub mod rawdump;

use regex::Regex;

const PATTERNS: &[(&str, &str, &str)] = &[
    ("x[ca]95[0-9]+(?:xl|xv)?",         "[a-z]{2}[0-9]+",       "xc9500"),
    ("xcr3[0-9]+xl",                    "[a-z]{2}[0-9]+",       "xpla3"),
    ("x[ca]2c[0-9]+a?",                 "[a-z]{2}g?[0-9]+",     "xbr"),

    ("xc40[0-9]+[el]",                  "[a-z]{2}[0-9]+",       "xc4000e"),
    ("xcs[0-9]+",                       "[a-z]{2}[0-9]+",       "xc4000e"),
    ("xc40[0-9]+(?:xl|ex)",             "[a-z]{2}[0-9]+",       "xc4000ex"),
    ("xc40[0-9]+xla",                   "[a-z]{2}[0-9]+",       "xc4000xla"),
    ("xc40[0-9]+xv",                    "[a-z]{2}[0-9]+",       "xc4000xv"),
    ("xcs[0-9]+xl",                     "[a-z]{2}[0-9]+",       "spartanxl"),

    ("x(?:cv|qv|qvr|c2s)[0-9]+",        "[a-z]{2}[0-9]+",       "virtex"),
    ("x(?:cv|qv|c2s|a2s)[0-9]+e",       "[a-z]{2}[0-9]+",       "virtexe"),

    ("x(?:c|q|qr)2v[0-9]+",             "[a-z]{2}[0-9]+",       "virtex2"),
    ("x[cq]2vpx?[0-9]+",                "[a-z]{2}[0-9]+",       "virtex2p"),

    ("xc3s[0-9]+l?",                    "[a-z]{2}[0-9]+",       "spartan3"),
    ("xa3s[0-9]+l?",                    "[a-z]{2}g[0-9]+",      "spartan3"),

    ("xc3s[0-9]+e",                     "[a-z]{2}[0-9]+",       "spartan3e"),
    ("xa3s[0-9]+e",                     "[a-z]{2}g[0-9]+",      "spartan3e"),

    ("xc3s[0-9]+a",                     "[a-z]{2}[0-9]+",       "spartan3a"),
    ("xc3s[0-9]+an",                    "[a-z]{2}g[0-9]+",      "spartan3a"),
    ("xa3s[0-9]+a",                     "[a-z]{2}g[0-9]+",      "spartan3a"),

    ("xc3sd[0-9]+a",                    "[a-z]{2}[0-9]+",       "spartan3adsp"),
    ("xa3sd[0-9]+a",                    "[a-z]{2}g[0-9]+",      "spartan3adsp"),

    ("x[cqa]6slx[0-9](?:[0-9]+t?|)l?",  "[a-z]{2}g?[0-9]+",     "spartan6"),

    ("x(?:c|q|qr)4v[lsf]x[0-9]+",       "[a-z]{2}[0-9]+",       "virtex4"),
    ("x[cq]5v[lsft]x[0-9]+t?",          "[a-z]{2}[0-9]+",       "virtex5"),
    ("x[cq]6v[lshc]x[0-9]+t?l?",        "[a-z]{2}g?[0-9]+",     "virtex6"),

    ("x[ca]7(?:[akvz]|v[xh])[0-9]+[st]?[li]?","[a-z]{2}[gv][0-9]+",  "7series"),
    ("xq7(?:[akv]|v[xh])[0-9]+t?[li]?", "[a-z]{2}[0-9]+",       "7series"),
    ("xq7z[0-9]+",                      "[a-z]{2}g?[0-9]+",     "7series"),

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

#[cfg(test)]
mod tests {
    #[test]
    fn split_partname_test() {
        assert_eq!(super::split_partname("xc6slx9tqg144"), Some(("xc6slx9", "tqg144", "spartan6")));
        assert_eq!(super::split_partname("xq6slx75tcs484"), Some(("xq6slx75t", "cs484", "spartan6")));
    }
}
