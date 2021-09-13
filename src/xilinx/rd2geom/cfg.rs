use std::collections::{HashMap, BTreeSet};
use crate::xilinx::geomdb::{Dir, Orient, TieState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawSiteSlot {
    Xy(String, u8, u8),
    Indexed(String, u8),
    Single(String),
}

#[derive(Debug, Copy, Clone)]
pub struct IntTileInfo {
    pub raw_tile: &'static str,
    pub name: &'static str,
}

impl IntTileInfo {
    pub fn int(raw_tile: &'static str, name: &'static str) -> Self {
        IntTileInfo { raw_tile, name }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntTermInfo {
    pub dir: Dir,
    pub raw_tile: &'static str,
    // size, pos
    pub span: Option<(usize, usize)>,
    pub name: &'static str,
    pub needs_tile: bool,
}

impl IntTermInfo {
    pub fn term(dir: Dir, raw: &'static str, name: &'static str) -> Self {
        IntTermInfo {
            dir,
            raw_tile: raw,
            span: None,
            name,
            needs_tile: false,
        }
    }
    pub fn l(raw: &'static str, name: &'static str) -> Self {
        Self::term(Dir::W, raw, name)
    }
    pub fn r(raw: &'static str, name: &'static str) -> Self {
        Self::term(Dir::E, raw, name)
    }
    pub fn t(raw: &'static str, name: &'static str) -> Self {
        Self::term(Dir::N, raw, name)
    }
    pub fn b(raw: &'static str, name: &'static str) -> Self {
        Self::term(Dir::S, raw, name)
    }
    pub fn term_fat(dir: Dir, raw: &'static str, name: &'static str) -> Self {
        IntTermInfo {
            dir,
            raw_tile: raw,
            span: None,
            name,
            needs_tile: true,
        }
    }
    pub fn l_fat(raw: &'static str, name: &'static str) -> Self {
        Self::term_fat(Dir::W, raw, name)
    }
    pub fn r_fat(raw: &'static str, name: &'static str) -> Self {
        Self::term_fat(Dir::E, raw, name)
    }
    pub fn t_fat(raw: &'static str, name: &'static str) -> Self {
        Self::term_fat(Dir::N, raw, name)
    }
    pub fn b_fat(raw: &'static str, name: &'static str) -> Self {
        Self::term_fat(Dir::S, raw, name)
    }
    pub fn term_multi(dir: Dir, raw: &'static str, span: (usize, usize), name: &'static str) -> Self {
        IntTermInfo {
            dir,
            raw_tile: raw,
            span: Some(span),
            name,
            needs_tile: false,
        }
    }
    pub fn l_multi(raw: &'static str, span: (usize, usize), name: &'static str) -> Self {
        Self::term_multi(Dir::W, raw, span, name)
    }
    pub fn r_multi(raw: &'static str, span: (usize, usize), name: &'static str) -> Self {
        Self::term_multi(Dir::E, raw, span, name)
    }
    #[allow(dead_code)]
    pub fn t_multi(raw: &'static str, span: (usize, usize), name: &'static str) -> Self {
        Self::term_multi(Dir::N, raw, span, name)
    }
    pub fn b_multi(raw: &'static str, span: (usize, usize), name: &'static str) -> Self {
        Self::term_multi(Dir::S, raw, span, name)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntPassInfo {
    pub orient: Orient,
    pub raw_tile: &'static str,
    // size, pos
    pub span: Option<(usize, usize)>,
    pub name: &'static str,
    pub empty: bool,
}

impl IntPassInfo {
    pub fn pass(orient: Orient, raw: &'static str, name: &'static str) -> Self {
        IntPassInfo {
            orient,
            raw_tile: raw,
            span: None,
            name,
            empty: false,
        }
    }
    pub fn h(raw: &'static str, name: &'static str) -> Self {
        Self::pass(Orient::H, raw, name)
    }
    pub fn v(raw: &'static str, name: &'static str) -> Self {
        Self::pass(Orient::V, raw, name)
    }
    pub fn pass_empty(orient: Orient, raw: &'static str) -> Self {
        IntPassInfo {
            orient,
            raw_tile: raw,
            span: None,
            name: "EMPTY",
            empty: true,
        }
    }
    #[allow(dead_code)]
    pub fn h_empty(raw: &'static str) -> Self {
        Self::pass_empty(Orient::H, raw)
    }
    pub fn v_empty(raw: &'static str) -> Self {
        Self::pass_empty(Orient::V, raw)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntBufInfo {
    pub orient: Orient,
    pub raw_tile: &'static str,
    // size, pos
    pub span: Option<(usize, usize)>,
    pub name: &'static str,
    pub needs_tile: bool,
}

impl IntBufInfo {
    pub fn buf(orient: Orient, raw: &'static str, name: &'static str) -> Self {
        IntBufInfo {
            orient,
            raw_tile: raw,
            span: None,
            name,
            needs_tile: false,
        }
    }
    pub fn h(raw: &'static str, name: &'static str) -> Self {
        Self::buf(Orient::H, raw, name)
    }
    #[allow(dead_code)]
    pub fn v(raw: &'static str, name: &'static str) -> Self {
        Self::buf(Orient::V, raw, name)
    }
    pub fn buf_fat(orient: Orient, raw: &'static str, name: &'static str) -> Self {
        IntBufInfo {
            orient,
            raw_tile: raw,
            span: None,
            name,
            needs_tile: true,
        }
    }
    pub fn h_fat(raw: &'static str, name: &'static str) -> Self {
        Self::buf_fat(Orient::H, raw, name)
    }
    pub fn v_fat(raw: &'static str, name: &'static str) -> Self {
        Self::buf_fat(Orient::V, raw, name)
    }
    pub fn buf_multi(orient: Orient, raw: &'static str, span: (usize, usize), name: &'static str) -> Self {
        IntBufInfo {
            orient,
            raw_tile: raw,
            span: Some(span),
            name,
            needs_tile: false,
        }
    }
    pub fn h_multi(raw: &'static str, span: (usize, usize), name: &'static str) -> Self {
        Self::buf_multi(Orient::H, raw, span, name)
    }
    #[allow(dead_code)]
    pub fn v_multi(raw: &'static str, span: (usize, usize), name: &'static str) -> Self {
        Self::buf_multi(Orient::V, raw, span, name)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IntDoubleBufInfo {
    pub orient: Orient,
    pub raw_tile_a: &'static str,
    pub raw_tile_b: &'static str,
    // size, pos
    pub span: Option<(usize, usize)>,
    pub name: &'static str,
    pub needs_tile: bool,
}

impl IntDoubleBufInfo {
    pub fn dbuf(orient: Orient, raw_a: &'static str, raw_b: &'static str, name: &'static str) -> Self {
        IntDoubleBufInfo {
            orient,
            raw_tile_a: raw_a,
            raw_tile_b: raw_b,
            span: None,
            name,
            needs_tile: false,
        }
    }
    pub fn h(raw_a: &'static str, raw_b: &'static str, name: &'static str) -> Self {
        Self::dbuf(Orient::H, raw_a, raw_b, name)
    }
    pub fn v(raw_a: &'static str, raw_b: &'static str, name: &'static str) -> Self {
        Self::dbuf(Orient::V, raw_a, raw_b, name)
    }
    pub fn dbuf_fat(orient: Orient, raw_a: &'static str, raw_b: &'static str, name: &'static str) -> Self {
        IntDoubleBufInfo {
            orient,
            raw_tile_a: raw_a,
            raw_tile_b: raw_b,
            span: None,
            name,
            needs_tile: true,
        }
    }
    pub fn h_fat(raw_a: &'static str, raw_b: &'static str, name: &'static str) -> Self {
        Self::dbuf_fat(Orient::H, raw_a, raw_b, name)
    }
    pub fn v_fat(raw_a: &'static str, raw_b: &'static str, name: &'static str) -> Self {
        Self::dbuf_fat(Orient::V, raw_a, raw_b, name)
    }
    pub fn dbuf_multi(orient: Orient, raw_a: &'static str, raw_b: &'static str, span: (usize, usize), name: &'static str) -> Self {
        IntDoubleBufInfo {
            orient,
            raw_tile_a: raw_a,
            raw_tile_b: raw_b,
            span: Some(span),
            name,
            needs_tile: false,
        }
    }
    #[allow(dead_code)]
    pub fn h_multi(raw_a: &'static str, raw_b: &'static str, span: (usize, usize), name: &'static str) -> Self {
        Self::dbuf_multi(Orient::H, raw_a, raw_b, span, name)
    }
    pub fn v_multi(raw_a: &'static str, raw_b: &'static str, span: (usize, usize), name: &'static str) -> Self {
        Self::dbuf_multi(Orient::V, raw_a, raw_b, span, name)
    }
}

pub struct TileAnchor {
    pub raw_tiles: &'static [&'static str],
    pub raw_delta: (isize, isize),
    pub grid_snap: (GridSnap, GridSnap),
    pub grid_delta: (isize, isize),
    pub extra_raw: Vec<(isize, isize, &'static [&'static str])>,
}

impl TileAnchor {
    pub fn int(raw_tiles: &'static [&'static str]) -> Self {
        TileAnchor {
            raw_tiles,
            raw_delta: (0, 0),
            grid_snap: (GridSnap::None, GridSnap::None),
            grid_delta: (0, 0),
            extra_raw: vec![],
        }
    }
    pub fn int_extra(raw_tiles: &'static [&'static str], extra_raw: Vec<(isize, isize, &'static [&'static str])>) -> Self {
        TileAnchor {
            raw_tiles,
            raw_delta: (0, 0),
            grid_snap: (GridSnap::None, GridSnap::None),
            grid_delta: (0, 0),
            extra_raw,
        }
    }
    pub fn snap_n(raw_tiles: &'static [&'static str]) -> Self {
        TileAnchor {
            raw_tiles,
            raw_delta: (0, 0),
            grid_snap: (GridSnap::None, GridSnap::Up),
            grid_delta: (0, 0),
            extra_raw: vec![],
        }
    }
    pub fn snap_s(raw_tiles: &'static [&'static str]) -> Self {
        TileAnchor {
            raw_tiles,
            raw_delta: (0, 0),
            grid_snap: (GridSnap::None, GridSnap::Down),
            grid_delta: (0, 0),
            extra_raw: vec![],
        }
    }
    pub fn snap_ne(raw_tiles: &'static [&'static str]) -> Self {
        TileAnchor {
            raw_tiles,
            raw_delta: (0, 0),
            grid_snap: (GridSnap::Up, GridSnap::Up),
            grid_delta: (0, 0),
            extra_raw: vec![],
        }
    }
}

pub struct TieSiteInfo {
    pub kind: &'static str,
    pub pins: &'static [(&'static str, TieState)],
}

pub enum GridSnap {
    Down,
    None,
    Up,
}

pub struct TileInfo {
    pub slot: &'static str,
    pub name: &'static str,
    pub anchor: TileAnchor,
    pub cells: Vec<(usize, usize)>,
    // XXX extra_grid
}

impl TileInfo {
    pub fn hclk(name: &'static str, raw_tiles: &'static [&'static str]) -> Self {
        TileInfo {
            slot: "HCLK",
            name,
            anchor: TileAnchor::snap_s(raw_tiles),
            cells: vec![(0, 0), (0, 1)],
        }
    }
    pub fn site_int(name: &'static str, raw_tiles: &'static [&'static str]) -> Self {
        TileInfo {
            slot: "SITE",
            name,
            anchor: TileAnchor::int(raw_tiles),
            cells: vec![(0, 0)],
        }
    }
    pub fn site_int_extra(name: &'static str, raw_tiles: &'static [&'static str], extra_raw: Vec<(isize, isize, &'static [&'static str])>) -> Self {
        TileInfo {
            slot: "SITE",
            name,
            anchor: TileAnchor::int_extra(raw_tiles, extra_raw),
            cells: vec![(0, 0)],
        }
    }
    pub fn site_vert_r(name: &'static str, raw_tiles: &'static [&'static str], v: (usize, usize)) -> Self {
        TileInfo {
            slot: "SITE",
            name,
            anchor: TileAnchor {
                raw_tiles,
                raw_delta: (0, -(v.1 as isize)),
                grid_snap: (GridSnap::Down, GridSnap::None),
                grid_delta: (0, 0),
                extra_raw: vec![],
            },
            cells: (0..v.0).map(|i| (0, i)).collect(),
        }
    }
    pub fn site_rect(name: &'static str, raw_tiles: &'static [&'static str], dim: (usize, usize), raw_delta: (isize, isize)) -> Self {
        let mut cells = Vec::new();
        for x in 0..dim.0 {
            for y in 0..dim.1 {
                if x == 0 || x == dim.0-1 || y == 0 || y == dim.1-1 {
                    cells.push((x, y));
                }
            }
        }
        TileInfo {
            slot: "SITE",
            name,
            anchor: TileAnchor {
                raw_tiles,
                raw_delta,
                grid_snap: (GridSnap::None, GridSnap::None),
                grid_delta: (0, 0),
                extra_raw: vec![],
            },
            cells,
        }
    }
    pub fn hclk_site_l(name: &'static str, raw_tiles: &'static [&'static str], v: (usize, usize)) -> Self {
        TileInfo {
            slot: "HCLK_SITE",
            name,
            anchor: TileAnchor {
                raw_tiles,
                raw_delta: (0, 0),
                grid_snap: (GridSnap::Up, GridSnap::Up),
                grid_delta: (0, -(v.1 as isize)),
                extra_raw: vec![],
            },
            cells: (0..v.0).map(|i| (0, i)).collect(),
        }
    }
    pub fn hclk_site_r(name: &'static str, raw_tiles: &'static [&'static str], v: (usize, usize)) -> Self {
        TileInfo {
            slot: "HCLK_SITE",
            name,
            anchor: TileAnchor {
                raw_tiles,
                raw_delta: (0, 0),
                grid_snap: (GridSnap::Down, GridSnap::Up),
                grid_delta: (0, -(v.1 as isize)),
                extra_raw: vec![],
            },
            cells: (0..v.0).map(|i| (0, i)).collect(),
        }
    }
}

pub struct GeomBuilderConfig {
    pub int_tiles: Vec<IntTileInfo>,
    pub int_terms: Vec<IntTermInfo>,
    pub int_bufs: Vec<IntBufInfo>,
    pub int_dbufs: Vec<IntDoubleBufInfo>,
    pub int_passes: Vec<IntPassInfo>,
    pub int_pass_combine: HashMap<BTreeSet<&'static str>, &'static str>,
    pub tie_sites: Vec<TieSiteInfo>,
    pub tiles: Vec<TileInfo>,
    // rd tile kind -> list of X coord deltas that should be marked as grid columns
    pub extra_cell_col_injectors: HashMap<&'static str, &'static [isize]>,
}
