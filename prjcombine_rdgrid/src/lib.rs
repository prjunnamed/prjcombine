use prjcombine_int::grid::{ColId, RowId};
use prjcombine_rawdump::Part;
use std::collections::BTreeSet;
use unnamed_entity::EntityVec;

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

pub fn find_columns(rd: &Part, tts: &[&str]) -> BTreeSet<i32> {
    let mut res = BTreeSet::new();
    for &tt in tts {
        for &xy in rd.tiles_by_kind_name(tt) {
            res.insert(xy.x as i32);
        }
    }
    res
}

pub fn find_column(rd: &Part, tts: &[&str]) -> Option<i32> {
    let res = find_columns(rd, tts);
    if res.len() > 1 {
        panic!("more than one column found for {tts:?}");
    }
    res.into_iter().next()
}

pub fn find_rows(rd: &Part, tts: &[&str]) -> BTreeSet<i32> {
    let mut res = BTreeSet::new();
    for &tt in tts {
        for &xy in rd.tiles_by_kind_name(tt) {
            res.insert(xy.y as i32);
        }
    }
    res
}

pub fn find_row(rd: &Part, tts: &[&str]) -> Option<i32> {
    let res = find_rows(rd, tts);
    if res.len() > 1 {
        panic!("more than one row found for {tts:?}");
    }
    res.into_iter().next()
}

pub fn find_tiles(rd: &Part, tts: &[&str]) -> BTreeSet<(i32, i32)> {
    let mut res = BTreeSet::new();
    for &tt in tts {
        for &xy in rd.tiles_by_kind_name(tt) {
            res.insert((xy.x as i32, xy.y as i32));
        }
    }
    res
}

#[derive(Clone, Debug)]
pub struct IntGrid<'a> {
    pub rd: &'a Part,
    pub cols: EntityVec<ColId, i32>,
    pub rows: EntityVec<RowId, i32>,
    pub slr_start: u16,
    pub slr_end: u16,
}

impl IntGrid<'_> {
    pub fn lookup_column(&self, col: i32) -> ColId {
        self.cols.binary_search(&col).unwrap()
    }

    pub fn lookup_column_inter(&self, col: i32) -> ColId {
        self.cols.binary_search(&col).unwrap_err()
    }

    pub fn lookup_row(&self, row: i32) -> RowId {
        self.rows.binary_search(&row).unwrap()
    }

    pub fn lookup_row_inter(&self, row: i32) -> RowId {
        self.rows.binary_search(&row).unwrap_err()
    }

    pub fn find_tiles(&self, tts: &[&str]) -> BTreeSet<(i32, i32)> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            for &xy in self.rd.tiles_by_kind_name(tt) {
                if (self.slr_start..self.slr_end).contains(&xy.y) {
                    res.insert((xy.x as i32, xy.y as i32));
                }
            }
        }
        res
    }

    pub fn find_rows(&self, tts: &[&str]) -> BTreeSet<i32> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            for &xy in self.rd.tiles_by_kind_name(tt) {
                if (self.slr_start..self.slr_end).contains(&xy.y) {
                    res.insert(xy.y as i32);
                }
            }
        }
        res
    }

    pub fn find_row(&self, tts: &[&str]) -> Option<i32> {
        let res = self.find_rows(tts);
        if res.len() > 1 {
            panic!("more than one row found for {tts:?}");
        }
        res.into_iter().next()
    }

    pub fn find_columns(&self, tts: &[&str]) -> BTreeSet<i32> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            for &xy in self.rd.tiles_by_kind_name(tt) {
                if (self.slr_start..self.slr_end).contains(&xy.y) {
                    res.insert(xy.x as i32);
                }
            }
        }
        res
    }

    pub fn find_column(&self, tts: &[&str]) -> Option<i32> {
        let res = self.find_columns(tts);
        if res.len() > 1 {
            panic!("more than one column found for {tts:?}");
        }
        res.into_iter().next()
    }
}

#[derive(Clone, Debug, Copy)]
pub struct ExtraCol {
    pub tts: &'static [&'static str],
    pub dx: &'static [i32],
}

pub fn extract_int<'a>(rd: &'a Part, tts: &[&str], extra_cols: &[ExtraCol]) -> IntGrid<'a> {
    extract_int_slr(rd, tts, extra_cols, 0, rd.height)
}

pub fn extract_int_slr<'a>(
    rd: &'a Part,
    tts: &[&str],
    extra_cols: &[ExtraCol],
    slr_start: u16,
    slr_end: u16,
) -> IntGrid<'a> {
    let mut res = IntGrid {
        rd,
        cols: EntityVec::new(),
        rows: EntityVec::new(),
        slr_start,
        slr_end,
    };
    let mut cols = res.find_columns(tts);
    let rows = res.find_rows(tts);
    for ec in extra_cols {
        for c in res.find_columns(ec.tts) {
            for d in ec.dx {
                cols.insert(c + d);
            }
        }
    }
    res.cols = cols.into_iter().collect();
    res.rows = rows
        .into_iter()
        .filter(|&x| (slr_start..slr_end).contains(&(x as u16)))
        .collect();
    res
}
