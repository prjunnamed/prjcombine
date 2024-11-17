#![allow(clippy::too_many_arguments)]

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
    pub slr_start_x: u16,
    pub slr_end_x: u16,
    pub slr_start_y: u16,
    pub slr_end_y: u16,
    pub mirror_x: bool,
    pub mirror_y: bool,
}

impl IntGrid<'_> {
    pub fn lookup_column(&self, col: i32) -> ColId {
        if self.mirror_x {
            self.cols.binary_search_by_key(&-col, |&x| -x).unwrap()
        } else {
            self.cols.binary_search(&col).unwrap()
        }
    }

    pub fn lookup_column_inter(&self, col: i32) -> ColId {
        if self.mirror_x {
            self.cols.binary_search_by_key(&-col, |&x| -x).unwrap_err()
        } else {
            self.cols.binary_search(&col).unwrap_err()
        }
    }

    pub fn lookup_row(&self, row: i32) -> RowId {
        if self.mirror_y {
            self.rows.binary_search_by_key(&-row, |&y| -y).unwrap()
        } else {
            self.rows.binary_search(&row).unwrap()
        }
    }

    pub fn lookup_row_inter(&self, row: i32) -> RowId {
        if self.mirror_y {
            self.rows.binary_search_by_key(&-row, |&y| -y).unwrap_err()
        } else {
            self.rows.binary_search(&row).unwrap_err()
        }
    }

    pub fn find_tiles(&self, tts: &[&str]) -> BTreeSet<(i32, i32)> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            for &xy in self.rd.tiles_by_kind_name(tt) {
                if (self.slr_start_x..self.slr_end_x).contains(&xy.x)
                    && (self.slr_start_y..self.slr_end_y).contains(&xy.y)
                {
                    res.insert((xy.x as i32, xy.y as i32));
                }
            }
        }
        res
    }

    pub fn find_rows_bset(&self, tts: &[&str]) -> BTreeSet<i32> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            for &xy in self.rd.tiles_by_kind_name(tt) {
                if (self.slr_start_x..self.slr_end_x).contains(&xy.x)
                    && (self.slr_start_y..self.slr_end_y).contains(&xy.y)
                {
                    res.insert(xy.y as i32);
                }
            }
        }
        res
    }

    pub fn find_rows(&self, tts: &[&str]) -> Vec<i32> {
        let rows = self.find_rows_bset(tts);
        if self.mirror_y {
            Vec::from_iter(rows.into_iter().rev())
        } else {
            Vec::from_iter(rows)
        }
    }

    pub fn find_row(&self, tts: &[&str]) -> Option<i32> {
        let res = self.find_rows_bset(tts);
        if res.len() > 1 {
            panic!("more than one row found for {tts:?}");
        }
        res.into_iter().next()
    }

    pub fn find_columns_bset(&self, tts: &[&str]) -> BTreeSet<i32> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            for &xy in self.rd.tiles_by_kind_name(tt) {
                if (self.slr_start_x..self.slr_end_x).contains(&xy.x)
                    && (self.slr_start_y..self.slr_end_y).contains(&xy.y)
                {
                    res.insert(xy.x as i32);
                }
            }
        }
        res
    }

    pub fn find_columns(&self, tts: &[&str]) -> Vec<i32> {
        let cols = self.find_columns_bset(tts);
        if self.mirror_x {
            Vec::from_iter(cols.into_iter().rev())
        } else {
            Vec::from_iter(cols)
        }
    }

    pub fn find_column(&self, tts: &[&str]) -> Option<i32> {
        let res = self.find_columns_bset(tts);
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
    extract_int_slr_column(rd, tts, extra_cols, 0, rd.height)
}

pub fn extract_int_slr_column<'a>(
    rd: &'a Part,
    tts: &[&str],
    extra_cols: &[ExtraCol],
    slr_start: u16,
    slr_end: u16,
) -> IntGrid<'a> {
    extract_int_slr(
        rd, tts, extra_cols, 0, rd.width, slr_start, slr_end, false, false,
    )
}

pub fn extract_int_slr<'a>(
    rd: &'a Part,
    tts: &[&str],
    extra_cols: &[ExtraCol],
    slr_start_x: u16,
    slr_end_x: u16,
    slr_start_y: u16,
    slr_end_y: u16,
    mirror_x: bool,
    mirror_y: bool,
) -> IntGrid<'a> {
    let mut res = IntGrid {
        rd,
        cols: EntityVec::new(),
        rows: EntityVec::new(),
        slr_start_y,
        slr_end_y,
        slr_start_x,
        slr_end_x,
        mirror_x,
        mirror_y,
    };
    let mut cols = res.find_columns_bset(tts);
    for ec in extra_cols {
        for c in res.find_columns(ec.tts) {
            for d in ec.dx {
                cols.insert(c + d);
            }
        }
    }
    if res.mirror_x {
        res.cols = cols.into_iter().rev().collect();
    } else {
        res.cols = cols.into_iter().collect();
    }
    let rows = res.find_rows(tts);
    res.rows = rows.into_iter().collect();
    res
}
