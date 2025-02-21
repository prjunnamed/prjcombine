use indexmap::IndexMap;

pub mod parse;

#[derive(Debug, Default)]
pub struct Sdf {
    pub sdfversion: Option<String>,
    pub design: Option<String>,
    pub date: Option<String>,
    pub vendor: Option<String>,
    pub program: Option<String>,
    pub version: Option<String>,
    pub timescale: Option<u32>, // log10 of timescale in units of fs
    pub cells: IndexMap<String, Cell>,
}

#[derive(Debug)]
pub struct Cell {
    pub typ: String,
    pub iopath: Vec<IoPath>,
    pub ports: Vec<Port>,
    pub setuphold: Vec<SetupHold>,
    pub period: Vec<Period>,
    pub width: Vec<Width>,
}

#[derive(Debug)]
pub struct IoPath {
    pub port_from: String,
    pub port_to: String,
    pub del_rise: Delay,
    pub del_fall: Delay,
}

#[derive(Debug)]
pub struct Port {
    pub port: String,
    pub del_rise: Delay,
    pub del_fall: Delay,
}

#[derive(Debug)]
pub struct SetupHold {
    pub edge_d: Edge,
    pub edge_c: Edge,
    pub setup: Delay,
    pub hold: Delay,
}

#[derive(Debug)]
pub struct Period {
    pub edge: Edge,
    pub val: Delay,
}

#[derive(Debug)]
pub struct Width {
    pub edge: Edge,
    pub val: Delay,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Delay {
    pub min: i64,
    pub typ: i64,
    pub max: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Edge {
    Plain(String),
    Posedge(String),
    Negedge(String),
}
