pub mod builder;

use std::fs::File;
use std::path::Path;
use ndarray::Array2;
use serde::{Serialize, Deserialize};
use crate::namevec::{NameVec, Named};
use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeomDb {
    pub name: String,
    pub vert_bus: NameVec<String>,
    pub horiz_bus: NameVec<String>,
    pub wires: NameVec<WireClass>,
    pub port_slots: NameVec<String>,
    pub ports: NameVec<PortClass>,
    pub tile_slots: NameVec<String>,
    pub tiles: NameVec<TileClass>,
    pub grids: NameVec<Grid>,
    pub parts: NameVec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireClass {
    pub name: String,
    pub cls: String,
    pub has_multicell_drive: bool,
    pub conn: WireConn,
}

impl Named for WireClass {
    fn get_name(&self) -> &str { &self.name }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireConn {
    Internal,
    Port {
        // (port slot, port conn idx)
        up: Option<(usize, usize)>,
        down: Vec<(usize, usize)>,
    },
    Tie(TieState),
    VertBus(usize),
    HorizBus(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortClass {
    pub name: String,
    pub slot: usize,
    pub opposite: usize,
    pub conns: Vec<PortConn>,
}

impl Named for PortClass {
    fn get_name(&self) -> &str { &self.name }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortConn {
    Unconnected,
    Local(usize),
    Remote(usize),
    Tie(TieState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileClass {
    pub name: String,
    // dx, dy, slot
    pub cells: Vec<(usize, usize, usize)>,
    pub muxes: Vec<TileMux>,
    pub multimuxes: Vec<TileMultiMux>,
    pub trans: Vec<TileTran>,
    pub sites: Vec<SiteSlot>,
}

impl Named for TileClass {
    fn get_name(&self) -> &str { &self.name }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TCWire {
    pub cell: usize,
    pub wire: usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TileMux {
    pub wire_out: TCWire,
    pub branches: Vec<TileMuxBranch>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TileMuxBranch {
    pub wire_in: TCWire,
    pub inversion: PipInversion,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TileMultiMux {
    pub name: String,
    pub wires_out: Vec<TCWire>,
    pub settings: Vec<MultiMuxSetting>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct MultiMuxSetting {
    pub name: String,
    pub branches_in: Vec<TileMuxBranch>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TileTran {
    pub wire_a: TCWire,
    pub wire_b: TCWire,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TieState {
    S0,
    S1,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SiteSlot {
    pub kind: String,
    pub pins: Vec<SitePin>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SitePin {
    pub name: String,
    pub mode: SitePinMode,
    pub wire: TCWire,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum SitePinMode {
    Input,
    Output,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum PipInversion {
    Never,
    Always,
    Prog,
}

// Grid starts here

/* future ideas:
 *
 * - scan chain
 * - banks
 * - packages
 * - site relations
 *   - counterpoint: just recover from wire connections
 * - SLR boundaries?
 *   - counterpoint: just use a vert_bus.
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub name: String,
    pub grid: Array2<GridCell>,
    pub columns: Vec<String>,
    pub vert_bus: Vec<GridRanges>,
    pub horiz_bus: Vec<GridRanges>,
    pub tiles: Vec<Tile>,
}

impl Grid {
    pub fn width(&self) -> usize { self.grid.dim().0 }
    pub fn height(&self) -> usize { self.grid.dim().1 }
}

impl Named for Grid {
    fn get_name(&self) -> &str { &self.name }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    // tile idx, cell idx
    pub tiles: Vec<Option<(usize, usize)>>,
    pub ports: Vec<Option<Port>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
pub struct GridRanges {
    // first is always 0, last is always width
    pub endpoints: Vec<usize>,
    // always one shorter than endpoints
    pub midpoints: Vec<usize>,
    // range #x means [endpoints[x], endpoints[x+1])
    pub grid2range: Vec<usize>,
}

impl GridRanges {
    pub fn new(endpoints: Vec<usize>, midpoints: Vec<usize>) -> Self {
        assert_eq!(endpoints[0], 0);
        assert_eq!(endpoints.len(), midpoints.len() + 1);
        let mut grid2range = Vec::new();
        for pos in 1..endpoints.len() {
            assert!(endpoints[pos - 1] < endpoints[pos]);
            while grid2range.len() < endpoints[pos] {
                grid2range.push(pos - 1);
            }
        }
        GridRanges { endpoints, midpoints, grid2range }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub cls: usize,
    pub origin: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub cls: usize,
    pub other: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub grid: usize,
}

impl Named for Part {
    fn get_name(&self) -> &str { &self.name }
}

impl GeomDb {
    pub fn from_file<P: AsRef<Path>> (path: P) -> Result<Self, Error> {
        let f = File::open(path)?;
        let cf = zstd::stream::Decoder::new(f)?;
        Ok(bincode::deserialize_from(cf).unwrap())
    }

    pub fn to_file<P: AsRef<Path>> (&self, path: P) -> Result<(), Error> {
        let f = File::create(path)?;
        let mut cf = zstd::stream::Encoder::new(f, 9)?;
        bincode::serialize_into(&mut cf, self).unwrap();
        cf.finish()?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Dir {
    E, W, N, S,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Orient {
    H, V,
}
