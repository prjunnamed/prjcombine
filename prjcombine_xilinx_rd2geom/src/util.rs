pub fn split_num(s: &str) -> Option<(&str, u32)> {
    let mut pos = None;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() {
            if pos.is_none() {
                pos = Some(i);
            }
        } else {
            pos = None;
        }
    }
    let pos = pos?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

#[test]
fn test_split_num() {
    assert_eq!(split_num("MEOW"), None);
    assert_eq!(split_num("MEOW3"), Some(("MEOW", 3)));
    assert_eq!(split_num("MEOW3B"), None);
    assert_eq!(split_num("MEOW3B2"), Some(("MEOW3B", 2)));
}
