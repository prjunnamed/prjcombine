pub fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_ascii_digit())?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}
