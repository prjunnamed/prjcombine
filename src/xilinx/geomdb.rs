use ndarray::Array2;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeomDb {
    pub name: String,
    pub vert_bus: Vec<String>,
    pub horiz_bus: Vec<String>,
    pub wires: Vec<WireClass>,
    pub port_slots: Vec<String>,
    pub ports: Vec<PortClass>,
    pub tile_slots: Vec<String>,
    pub tiles: Vec<TileClass>,
    pub grids: Vec<Grid>,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireClass {
    pub name: String,
    pub cls: String,
    pub has_multicell_drive: bool,
    pub is_permabuf_alias: bool,
    pub conn: WireConn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireConn {
    Internal,
    Port {
        // (port slot, port conn idx)
        up: Option<(usize, usize)>,
        down: Vec<(usize, usize)>,
    },
    VertBus(usize),
    HorizBus(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortClass {
    pub name: String,
    pub slot: usize,
    pub raw_variants: Vec<String>,
    pub opposite: usize,
    pub conns: Vec<PortConn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortConn {
    Unconnected,
    Local(usize, Vec<RawPip>),
    Remote(usize, Vec<RawPip>),
    Tie(TieState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileClass {
    pub name: String,
    pub slot: usize,
    pub raw_variants: Vec<String>,
    // dx, dy, slot
    pub cells: Vec<(usize, usize, usize)>,
    pub muxes: Vec<TileMux>,
    pub trans: Vec<TileTran>,
    pub ties: Vec<TileTie>,
    pub sites: Vec<SiteSlot>,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct TCWire {
    pub cell: usize,
    pub wire: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMux {
    pub wire_out: TCWire,
    pub branches: Vec<TileMuxBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMuxBranch {
    pub wire_in: TCWire,
    pub is_excl: bool,
    pub is_test: bool,
    pub inversion: PipInversion,
    pub raw: Vec<RawPip>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileTiedMux {
    pub name: String,
    pub wires_out: Vec<TCWire>,
    pub settings: Vec<TiedMuxSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiedMuxSetting {
    pub name: String,
    pub branches_in: Vec<TileMuxBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileTran {
    pub wire_a: TCWire,
    pub wire_b: TCWire,
    pub is_excl: bool,
    pub is_test: bool,
    pub raw: RawPip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileTie {
    pub wire: TCWire,
    pub state: TieState,
    pub raw_site_pin: Option<(usize, usize, String)>,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TieState {
    S0,
    S1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteSlot {
    pub kind: String,
    pub subkind: String,
    pub raw: Option<(usize, usize)>,
    pub pins: Vec<SitePin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SitePin {
    pub name: String,
    pub mode: SitePinMode,
    pub wire: TCWire,
    pub raw_pip: Vec<RawPip>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPip {
    pub variants: Vec<RawPipVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPipVariant {
    pub tile: usize,
    pub wire_out: String,
    pub wire_in: String,
    pub direction: PipDirection,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum PipDirection {
    Uni,
    BiFwd,
    BiBwd,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    // tile idx, cell idx
    pub tiles: Vec<Option<(usize, usize)>>,
    pub ports: Vec<Option<Port>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridRanges {
    // first is always 0, last is always width
    pub endpoints: Vec<usize>,
    // range #x means [endpoints[x], endpoints[x+1])
    pub grid2range: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub cls: usize,
    pub origin: (usize, usize),
    pub raw_tiles: Vec<String>,
    pub raw_sites: Vec<Option<String>>,
    pub raw_variant: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub cls: usize,
    pub other: (usize, usize),
    pub raw_tiles: Vec<String>,
    pub raw_variant: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub grid: usize,
}
