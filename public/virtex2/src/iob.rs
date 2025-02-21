use prjcombine_interconnect::db::{BelId, Dir};
use unnamed_entity::EntityId;

use crate::chip::{ChipKind, ColumnIoKind, RowIoKind};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum IobDiff {
    None,
    True(usize),
    Comp(usize),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum IobKind {
    Iob,
    Ibuf,
    Obuf,
    Clk,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct IobData {
    pub tile: usize,
    pub bel: BelId,
    pub diff: IobDiff,
    pub kind: IobKind,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct IobTileData {
    pub node: &'static str,
    pub edge: Dir,
    pub tiles: usize,
    pub iobs: Vec<IobData>,
}

fn iob(tile: usize, bel: usize) -> IobData {
    IobData {
        kind: IobKind::Iob,
        diff: IobDiff::None,
        tile,
        bel: BelId::from_idx(bel),
    }
}
fn iobt(tile: usize, bel: usize, other: usize) -> IobData {
    IobData {
        kind: IobKind::Iob,
        diff: IobDiff::True(other),
        tile,
        bel: BelId::from_idx(bel),
    }
}
fn iobc(tile: usize, bel: usize, other: usize) -> IobData {
    IobData {
        kind: IobKind::Iob,
        diff: IobDiff::Comp(other),
        tile,
        bel: BelId::from_idx(bel),
    }
}
fn ibuf(tile: usize, bel: usize) -> IobData {
    IobData {
        kind: IobKind::Ibuf,
        diff: IobDiff::None,
        tile,
        bel: BelId::from_idx(bel),
    }
}
fn ibuft(tile: usize, bel: usize, other: usize) -> IobData {
    IobData {
        kind: IobKind::Ibuf,
        diff: IobDiff::True(other),
        tile,
        bel: BelId::from_idx(bel),
    }
}
fn ibufc(tile: usize, bel: usize, other: usize) -> IobData {
    IobData {
        kind: IobKind::Ibuf,
        diff: IobDiff::Comp(other),
        tile,
        bel: BelId::from_idx(bel),
    }
}
fn obuf(tile: usize, bel: usize) -> IobData {
    IobData {
        kind: IobKind::Obuf,
        diff: IobDiff::None,
        tile,
        bel: BelId::from_idx(bel),
    }
}
fn clkt(tile: usize, bel: usize, other: usize) -> IobData {
    IobData {
        kind: IobKind::Clk,
        diff: IobDiff::True(other),
        tile,
        bel: BelId::from_idx(bel),
    }
}
fn clkc(tile: usize, bel: usize, other: usize) -> IobData {
    IobData {
        kind: IobKind::Clk,
        diff: IobDiff::Comp(other),
        tile,
        bel: BelId::from_idx(bel),
    }
}

pub fn get_iob_data_b(kind: ChipKind, col: ColumnIoKind) -> (IobTileData, usize) {
    match kind {
        ChipKind::Virtex2 => match col {
            ColumnIoKind::DoubleLeft(i) => (get_iob_data("IOBS.V2.B.L2"), i.into()),
            ColumnIoKind::DoubleRight(i) => (get_iob_data("IOBS.V2.B.R2"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Virtex2P | ChipKind::Virtex2PX => match col {
            ColumnIoKind::DoubleLeft(i) => (get_iob_data("IOBS.V2P.B.L2"), i.into()),
            ColumnIoKind::DoubleRight(i) => (get_iob_data("IOBS.V2P.B.R2"), i.into()),
            ColumnIoKind::DoubleRightClk(i) => (get_iob_data("IOBS.V2P.B.R2.CLK"), i.into()),
            ColumnIoKind::SingleLeft => (get_iob_data("IOBS.V2P.B.L1"), 0),
            ColumnIoKind::SingleLeftAlt => (get_iob_data("IOBS.V2P.B.L1.ALT"), 0),
            ColumnIoKind::SingleRight => (get_iob_data("IOBS.V2P.B.R1"), 0),
            ColumnIoKind::SingleRightAlt => (get_iob_data("IOBS.V2P.B.R1.ALT"), 0),
            _ => unreachable!(),
        },
        ChipKind::Spartan3 => match col {
            ColumnIoKind::Double(i) => (get_iob_data("IOBS.S3.B2"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::FpgaCore => (get_iob_data("IOBS.FC.B"), 0),
        ChipKind::Spartan3E => match col {
            ColumnIoKind::Single => (get_iob_data("IOBS.S3E.B1"), 0),
            ColumnIoKind::Double(i) => (get_iob_data("IOBS.S3E.B2"), i.into()),
            ColumnIoKind::Triple(i) => (get_iob_data("IOBS.S3E.B3"), i.into()),
            ColumnIoKind::Quad(i) => (get_iob_data("IOBS.S3E.B4"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match col {
            ColumnIoKind::Double(i) => (get_iob_data("IOBS.S3A.B2"), i.into()),
            _ => unreachable!(),
        },
    }
}

pub fn get_iob_data_t(kind: ChipKind, col: ColumnIoKind) -> (IobTileData, usize) {
    match kind {
        ChipKind::Virtex2 => match col {
            ColumnIoKind::DoubleLeft(i) => (get_iob_data("IOBS.V2.T.L2"), i.into()),
            ColumnIoKind::DoubleRight(i) => (get_iob_data("IOBS.V2.T.R2"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Virtex2P | ChipKind::Virtex2PX => match col {
            ColumnIoKind::DoubleLeft(i) => (get_iob_data("IOBS.V2P.T.L2"), i.into()),
            ColumnIoKind::DoubleRight(i) => (get_iob_data("IOBS.V2P.T.R2"), i.into()),
            ColumnIoKind::DoubleRightClk(i) => (get_iob_data("IOBS.V2P.T.R2.CLK"), i.into()),
            ColumnIoKind::SingleLeft => (get_iob_data("IOBS.V2P.T.L1"), 0),
            ColumnIoKind::SingleLeftAlt => (get_iob_data("IOBS.V2P.T.L1.ALT"), 0),
            ColumnIoKind::SingleRight => (get_iob_data("IOBS.V2P.T.R1"), 0),
            ColumnIoKind::SingleRightAlt => (get_iob_data("IOBS.V2P.T.R1.ALT"), 0),
            _ => unreachable!(),
        },
        ChipKind::Spartan3 => match col {
            ColumnIoKind::Double(i) => (get_iob_data("IOBS.S3.T2"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::FpgaCore => (get_iob_data("IOBS.FC.T"), 0),
        ChipKind::Spartan3E => match col {
            ColumnIoKind::Single => (get_iob_data("IOBS.S3E.T1"), 0),
            ColumnIoKind::Double(i) => (get_iob_data("IOBS.S3E.T2"), i.into()),
            ColumnIoKind::Triple(i) => (get_iob_data("IOBS.S3E.T3"), i.into()),
            ColumnIoKind::Quad(i) => (get_iob_data("IOBS.S3E.T4"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match col {
            ColumnIoKind::Double(i) => (get_iob_data("IOBS.S3A.T2"), i.into()),
            _ => unreachable!(),
        },
    }
}

pub fn get_iob_data_l(kind: ChipKind, row: RowIoKind) -> (IobTileData, usize) {
    match kind {
        ChipKind::Virtex2 => match row {
            RowIoKind::DoubleBot(i) => (get_iob_data("IOBS.V2.L.B2"), i.into()),
            RowIoKind::DoubleTop(i) => (get_iob_data("IOBS.V2.L.T2"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Virtex2P | ChipKind::Virtex2PX => match row {
            RowIoKind::DoubleBot(i) => (get_iob_data("IOBS.V2P.L.B2"), i.into()),
            RowIoKind::DoubleTop(i) => (get_iob_data("IOBS.V2P.L.T2"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Spartan3 => match row {
            RowIoKind::Single => (get_iob_data("IOBS.S3.L1"), 0),
            _ => unreachable!(),
        },
        ChipKind::FpgaCore => (get_iob_data("IOBS.FC.L"), 0),
        ChipKind::Spartan3E => match row {
            RowIoKind::Single => (get_iob_data("IOBS.S3E.L1"), 0),
            RowIoKind::Double(i) => (get_iob_data("IOBS.S3E.L2"), i.into()),
            RowIoKind::Triple(i) => (get_iob_data("IOBS.S3E.L3"), i.into()),
            RowIoKind::Quad(i) => (get_iob_data("IOBS.S3E.L4"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match row {
            RowIoKind::Quad(i) => (get_iob_data("IOBS.S3A.L4"), i.into()),
            _ => unreachable!(),
        },
    }
}

pub fn get_iob_data_r(kind: ChipKind, row: RowIoKind) -> (IobTileData, usize) {
    match kind {
        ChipKind::Virtex2 => match row {
            RowIoKind::DoubleBot(i) => (get_iob_data("IOBS.V2.R.B2"), i.into()),
            RowIoKind::DoubleTop(i) => (get_iob_data("IOBS.V2.R.T2"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Virtex2P | ChipKind::Virtex2PX => match row {
            RowIoKind::DoubleBot(i) => (get_iob_data("IOBS.V2P.R.B2"), i.into()),
            RowIoKind::DoubleTop(i) => (get_iob_data("IOBS.V2P.R.T2"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Spartan3 => match row {
            RowIoKind::Single => (get_iob_data("IOBS.S3.R1"), 0),
            _ => unreachable!(),
        },
        ChipKind::FpgaCore => (get_iob_data("IOBS.FC.R"), 0),
        ChipKind::Spartan3E => match row {
            RowIoKind::Single => (get_iob_data("IOBS.S3E.R1"), 0),
            RowIoKind::Double(i) => (get_iob_data("IOBS.S3E.R2"), i.into()),
            RowIoKind::Triple(i) => (get_iob_data("IOBS.S3E.R3"), i.into()),
            RowIoKind::Quad(i) => (get_iob_data("IOBS.S3E.R4"), i.into()),
            _ => unreachable!(),
        },
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match row {
            RowIoKind::Quad(i) => (get_iob_data("IOBS.S3A.R4"), i.into()),
            _ => unreachable!(),
        },
    }
}

pub fn get_iob_tiles(kind: ChipKind) -> Vec<IobTileData> {
    match kind {
        ChipKind::Virtex2 => vec![
            get_iob_data("IOBS.V2.T.L2"),
            get_iob_data("IOBS.V2.T.R2"),
            get_iob_data("IOBS.V2.R.B2"),
            get_iob_data("IOBS.V2.R.T2"),
            get_iob_data("IOBS.V2.B.L2"),
            get_iob_data("IOBS.V2.B.R2"),
            get_iob_data("IOBS.V2.L.B2"),
            get_iob_data("IOBS.V2.L.T2"),
        ],
        ChipKind::Virtex2P | ChipKind::Virtex2PX => vec![
            get_iob_data("IOBS.V2P.T.L1"),
            get_iob_data("IOBS.V2P.T.L1.ALT"),
            get_iob_data("IOBS.V2P.T.R1"),
            get_iob_data("IOBS.V2P.T.R1.ALT"),
            get_iob_data("IOBS.V2P.T.L2"),
            get_iob_data("IOBS.V2P.T.R2"),
            get_iob_data("IOBS.V2P.T.R2.CLK"),
            get_iob_data("IOBS.V2P.R.B2"),
            get_iob_data("IOBS.V2P.R.T2"),
            get_iob_data("IOBS.V2P.B.L1"),
            get_iob_data("IOBS.V2P.B.L1.ALT"),
            get_iob_data("IOBS.V2P.B.R1"),
            get_iob_data("IOBS.V2P.B.R1.ALT"),
            get_iob_data("IOBS.V2P.B.L2"),
            get_iob_data("IOBS.V2P.B.R2"),
            get_iob_data("IOBS.V2P.B.R2.CLK"),
            get_iob_data("IOBS.V2P.L.B2"),
            get_iob_data("IOBS.V2P.L.T2"),
        ],
        ChipKind::Spartan3 => vec![
            get_iob_data("IOBS.S3.T2"),
            get_iob_data("IOBS.S3.R1"),
            get_iob_data("IOBS.S3.B2"),
            get_iob_data("IOBS.S3.L1"),
        ],
        ChipKind::FpgaCore => vec![
            get_iob_data("IOBS.FC.T"),
            get_iob_data("IOBS.FC.R"),
            get_iob_data("IOBS.FC.B"),
            get_iob_data("IOBS.FC.L"),
        ],
        ChipKind::Spartan3E => vec![
            get_iob_data("IOBS.S3E.T1"),
            get_iob_data("IOBS.S3E.T2"),
            get_iob_data("IOBS.S3E.T3"),
            get_iob_data("IOBS.S3E.T4"),
            get_iob_data("IOBS.S3E.R1"),
            get_iob_data("IOBS.S3E.R2"),
            get_iob_data("IOBS.S3E.R3"),
            get_iob_data("IOBS.S3E.R4"),
            get_iob_data("IOBS.S3E.B1"),
            get_iob_data("IOBS.S3E.B2"),
            get_iob_data("IOBS.S3E.B3"),
            get_iob_data("IOBS.S3E.B4"),
            get_iob_data("IOBS.S3E.L1"),
            get_iob_data("IOBS.S3E.L2"),
            get_iob_data("IOBS.S3E.L3"),
            get_iob_data("IOBS.S3E.L4"),
        ],
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => vec![
            get_iob_data("IOBS.S3A.T2"),
            get_iob_data("IOBS.S3A.R4"),
            get_iob_data("IOBS.S3A.B2"),
            get_iob_data("IOBS.S3A.L4"),
        ],
    }
}

pub fn get_iob_data(node: &str) -> IobTileData {
    match node {
        // Virtex 2
        "IOBS.V2.T.L2" => IobTileData {
            node: "IOBS.V2.T.L2",
            edge: Dir::N,
            tiles: 2,
            iobs: vec![
                iobc(0, 3, 1),
                iobt(0, 2, 0),
                iobc(0, 1, 3),
                iobt(0, 0, 2),
                iobc(1, 1, 5),
                iobt(1, 0, 4),
            ],
        },
        "IOBS.V2.T.R2" => IobTileData {
            node: "IOBS.V2.T.R2",
            edge: Dir::N,
            tiles: 2,
            iobs: vec![
                iobc(0, 3, 1),
                iobt(0, 2, 0),
                iobc(1, 3, 3),
                iobt(1, 2, 2),
                iobc(1, 1, 5),
                iobt(1, 0, 4),
            ],
        },
        "IOBS.V2.R.T2" => IobTileData {
            node: "IOBS.V2.R.T2",
            edge: Dir::E,
            tiles: 2,
            iobs: vec![
                iobc(1, 3, 1),
                iobt(1, 2, 0),
                iobc(1, 1, 3),
                iobt(1, 0, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2.R.B2" => IobTileData {
            node: "IOBS.V2.R.B2",
            edge: Dir::E,
            tiles: 2,
            iobs: vec![
                iobc(1, 3, 1),
                iobt(1, 2, 0),
                iobc(0, 3, 3),
                iobt(0, 2, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2.B.R2" => IobTileData {
            node: "IOBS.V2.B.R2",
            edge: Dir::S,
            tiles: 2,
            iobs: vec![
                iobc(1, 3, 1),
                iobt(1, 2, 0),
                iobc(1, 1, 3),
                iobt(1, 0, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2.B.L2" => IobTileData {
            node: "IOBS.V2.B.L2",
            edge: Dir::S,
            tiles: 2,
            iobs: vec![
                iobc(1, 3, 1),
                iobt(1, 2, 0),
                iobc(0, 3, 3),
                iobt(0, 2, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2.L.B2" => IobTileData {
            node: "IOBS.V2.L.B2",
            edge: Dir::W,
            tiles: 2,
            iobs: vec![
                iobt(0, 0, 1),
                iobc(0, 1, 0),
                iobt(0, 2, 3),
                iobc(0, 3, 2),
                iobt(1, 2, 5),
                iobc(1, 3, 4),
            ],
        },
        "IOBS.V2.L.T2" => IobTileData {
            node: "IOBS.V2.L.T2",
            edge: Dir::W,
            tiles: 2,
            iobs: vec![
                iobt(0, 0, 1),
                iobc(0, 1, 0),
                iobt(1, 0, 3),
                iobc(1, 1, 2),
                iobt(1, 2, 5),
                iobc(1, 3, 4),
            ],
        },

        // Virtex 2 Pro
        "IOBS.V2P.T.L1" => IobTileData {
            node: "IOBS.V2P.T.L1",
            edge: Dir::N,
            tiles: 1,
            iobs: vec![iob(0, 2), iobc(0, 1, 2), iobt(0, 0, 1)],
        },
        "IOBS.V2P.T.L1.ALT" => IobTileData {
            node: "IOBS.V2P.T.L1.ALT",
            edge: Dir::N,
            tiles: 1,
            iobs: vec![iobc(0, 2, 1), iobt(0, 1, 0), iob(0, 0)],
        },
        "IOBS.V2P.T.R1" => IobTileData {
            node: "IOBS.V2P.T.R1",
            edge: Dir::N,
            tiles: 1,
            iobs: vec![iobc(0, 3, 1), iobt(0, 2, 0), iob(0, 1)],
        },
        "IOBS.V2P.T.R1.ALT" => IobTileData {
            node: "IOBS.V2P.T.R1.ALT",
            edge: Dir::N,
            tiles: 1,
            iobs: vec![iob(0, 3), iobc(0, 2, 2), iobt(0, 1, 1)],
        },
        "IOBS.V2P.T.L2" => IobTileData {
            node: "IOBS.V2P.T.L2",
            edge: Dir::N,
            tiles: 2,
            iobs: vec![
                iobc(0, 3, 1),
                iobt(0, 2, 0),
                iobc(0, 1, 3),
                iobt(0, 0, 2),
                iobc(1, 1, 5),
                iobt(1, 0, 4),
            ],
        },
        "IOBS.V2P.T.R2" => IobTileData {
            node: "IOBS.V2P.T.R2",
            edge: Dir::N,
            tiles: 2,
            iobs: vec![
                iobc(0, 3, 1),
                iobt(0, 2, 0),
                iobc(1, 3, 3),
                iobt(1, 2, 2),
                iobc(1, 1, 5),
                iobt(1, 0, 4),
            ],
        },
        "IOBS.V2P.T.R2.CLK" => IobTileData {
            node: "IOBS.V2P.T.R2.CLK",
            edge: Dir::N,
            tiles: 2,
            iobs: vec![
                iobc(0, 3, 1),
                iobt(0, 2, 0),
                iobc(1, 3, 3),
                iobt(1, 2, 2),
                clkc(1, 1, 5),
                clkt(1, 0, 4),
            ],
        },
        "IOBS.V2P.R.T2" => IobTileData {
            node: "IOBS.V2P.R.T2",
            edge: Dir::E,
            tiles: 2,
            iobs: vec![
                iobc(1, 3, 1),
                iobt(1, 2, 0),
                iobc(1, 1, 3),
                iobt(1, 0, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2P.R.B2" => IobTileData {
            node: "IOBS.V2P.R.B2",
            edge: Dir::E,
            tiles: 2,
            iobs: vec![
                iobc(1, 3, 1),
                iobt(1, 2, 0),
                iobc(0, 3, 3),
                iobt(0, 2, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2P.B.R1" => IobTileData {
            node: "IOBS.V2P.B.R1",
            edge: Dir::S,
            tiles: 1,
            iobs: vec![iob(0, 2), iobc(0, 1, 2), iobt(0, 0, 1)],
        },
        "IOBS.V2P.B.R1.ALT" => IobTileData {
            node: "IOBS.V2P.B.R1.ALT",
            edge: Dir::S,
            tiles: 1,
            iobs: vec![iobc(0, 2, 1), iobt(0, 1, 0), iob(0, 0)],
        },
        "IOBS.V2P.B.L1" => IobTileData {
            node: "IOBS.V2P.B.L1",
            edge: Dir::S,
            tiles: 1,
            iobs: vec![iobc(0, 3, 1), iobt(0, 2, 0), iob(0, 1)],
        },
        "IOBS.V2P.B.L1.ALT" => IobTileData {
            node: "IOBS.V2P.B.L1.ALT",
            edge: Dir::S,
            tiles: 1,
            iobs: vec![iob(0, 3), iobc(0, 2, 2), iobt(0, 1, 1)],
        },
        "IOBS.V2P.B.R2" => IobTileData {
            node: "IOBS.V2P.B.R2",
            edge: Dir::S,
            tiles: 2,
            iobs: vec![
                iobc(1, 3, 1),
                iobt(1, 2, 0),
                iobc(1, 1, 3),
                iobt(1, 0, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2P.B.L2" => IobTileData {
            node: "IOBS.V2P.B.L2",
            edge: Dir::S,
            tiles: 2,
            iobs: vec![
                iobc(1, 3, 1),
                iobt(1, 2, 0),
                iobc(0, 3, 3),
                iobt(0, 2, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2P.B.R2.CLK" => IobTileData {
            node: "IOBS.V2P.B.R2.CLK",
            edge: Dir::S,
            tiles: 2,
            iobs: vec![
                clkc(1, 3, 1),
                clkt(1, 2, 0),
                iobc(1, 1, 3),
                iobt(1, 0, 2),
                iobc(0, 1, 5),
                iobt(0, 0, 4),
            ],
        },
        "IOBS.V2P.L.B2" => IobTileData {
            node: "IOBS.V2P.L.B2",
            edge: Dir::W,
            tiles: 2,
            iobs: vec![
                iobt(0, 0, 1),
                iobc(0, 1, 0),
                iobt(0, 2, 3),
                iobc(0, 3, 2),
                iobt(1, 2, 5),
                iobc(1, 3, 4),
            ],
        },
        "IOBS.V2P.L.T2" => IobTileData {
            node: "IOBS.V2P.L.T2",
            edge: Dir::W,
            tiles: 2,
            iobs: vec![
                iobt(0, 0, 1),
                iobc(0, 1, 0),
                iobt(1, 0, 3),
                iobc(1, 1, 2),
                iobt(1, 2, 5),
                iobc(1, 3, 4),
            ],
        },

        // Spartan 3
        "IOBS.S3.T2" => IobTileData {
            node: "IOBS.S3.T2",
            edge: Dir::N,
            tiles: 2,
            iobs: vec![
                iob(0, 2),
                iobc(0, 1, 2),
                iobt(0, 0, 1),
                iobc(1, 1, 4),
                iobt(1, 0, 3),
            ],
        },
        "IOBS.S3.R1" => IobTileData {
            node: "IOBS.S3.R1",
            edge: Dir::E,
            tiles: 1,
            iobs: vec![iobc(0, 1, 1), iobt(0, 0, 0)],
        },
        "IOBS.S3.B2" => IobTileData {
            node: "IOBS.S3.B2",
            edge: Dir::S,
            tiles: 2,
            iobs: vec![
                iob(1, 2),
                iobc(1, 1, 2),
                iobt(1, 0, 1),
                iobc(0, 1, 4),
                iobt(0, 0, 3),
            ],
        },
        "IOBS.S3.L1" => IobTileData {
            node: "IOBS.S3.L1",
            edge: Dir::W,
            tiles: 1,
            iobs: vec![iobc(0, 0, 1), iobt(0, 1, 0)],
        },

        // FPGA core
        "IOBS.FC.T" => IobTileData {
            node: "IOBS.FC.T",
            edge: Dir::N,
            tiles: 1,
            iobs: vec![
                ibuf(0, 3),
                obuf(0, 7),
                ibuf(0, 2),
                obuf(0, 6),
                ibuf(0, 1),
                obuf(0, 5),
                ibuf(0, 0),
                obuf(0, 4),
            ],
        },
        "IOBS.FC.R" => IobTileData {
            node: "IOBS.FC.R",
            edge: Dir::E,
            tiles: 1,
            iobs: vec![
                ibuf(0, 3),
                obuf(0, 7),
                ibuf(0, 2),
                obuf(0, 6),
                ibuf(0, 1),
                obuf(0, 5),
                ibuf(0, 0),
                obuf(0, 4),
            ],
        },
        "IOBS.FC.B" => IobTileData {
            node: "IOBS.FC.B",
            edge: Dir::S,
            tiles: 1,
            iobs: vec![
                ibuf(0, 3),
                obuf(0, 7),
                ibuf(0, 2),
                obuf(0, 6),
                ibuf(0, 1),
                obuf(0, 5),
                ibuf(0, 0),
                obuf(0, 4),
            ],
        },
        "IOBS.FC.L" => IobTileData {
            node: "IOBS.FC.L",
            edge: Dir::W,
            tiles: 1,
            iobs: vec![
                ibuf(0, 0),
                obuf(0, 4),
                ibuf(0, 1),
                obuf(0, 5),
                ibuf(0, 2),
                obuf(0, 6),
                ibuf(0, 3),
                obuf(0, 7),
            ],
        },

        // Spartan 3E
        "IOBS.S3E.T1" => IobTileData {
            node: "IOBS.S3E.T1",
            edge: Dir::N,
            tiles: 1,
            iobs: vec![iob(0, 2)],
        },
        "IOBS.S3E.T2" => IobTileData {
            node: "IOBS.S3E.T2",
            edge: Dir::N,
            tiles: 2,
            iobs: vec![iobc(0, 1, 1), iobt(0, 0, 0), ibuf(1, 2)],
        },
        "IOBS.S3E.T3" => IobTileData {
            node: "IOBS.S3E.T3",
            edge: Dir::N,
            tiles: 3,
            iobs: vec![
                iobc(0, 1, 1),
                iobt(0, 0, 0),
                ibuf(1, 2),
                iobc(2, 1, 4),
                iobt(2, 0, 3),
            ],
        },
        "IOBS.S3E.T4" => IobTileData {
            node: "IOBS.S3E.T4",
            edge: Dir::N,
            tiles: 4,
            iobs: vec![
                iobc(0, 1, 1),
                iobt(0, 0, 0),
                iob(1, 2),
                iobc(2, 1, 4),
                iobt(2, 0, 3),
                ibufc(3, 1, 6),
                ibuft(3, 0, 5),
            ],
        },
        "IOBS.S3E.R1" => IobTileData {
            node: "IOBS.S3E.R1",
            edge: Dir::E,
            tiles: 1,
            iobs: vec![iob(0, 2)],
        },
        "IOBS.S3E.R2" => IobTileData {
            node: "IOBS.S3E.R2",
            edge: Dir::E,
            tiles: 2,
            iobs: vec![iobc(0, 1, 1), iobt(0, 0, 0)],
        },
        "IOBS.S3E.R3" => IobTileData {
            node: "IOBS.S3E.R3",
            edge: Dir::E,
            tiles: 3,
            iobs: vec![ibuf(2, 2), iob(1, 2), iobc(0, 1, 3), iobt(0, 0, 2)],
        },
        "IOBS.S3E.R4" => IobTileData {
            node: "IOBS.S3E.R4",
            edge: Dir::E,
            tiles: 4,
            iobs: vec![
                ibuf(3, 2),
                iobc(2, 1, 2),
                iobt(2, 0, 1),
                iobc(0, 1, 4),
                iobt(0, 0, 3),
            ],
        },
        "IOBS.S3E.B1" => IobTileData {
            node: "IOBS.S3E.B1",
            edge: Dir::S,
            tiles: 1,
            iobs: vec![iob(0, 2)],
        },
        "IOBS.S3E.B2" => IobTileData {
            node: "IOBS.S3E.B2",
            edge: Dir::S,
            tiles: 2,
            iobs: vec![iobc(1, 1, 1), iobt(1, 0, 0), ibuf(0, 2)],
        },
        "IOBS.S3E.B3" => IobTileData {
            node: "IOBS.S3E.B3",
            edge: Dir::S,
            tiles: 3,
            iobs: vec![
                iobc(2, 1, 1),
                iobt(2, 0, 0),
                ibuf(1, 2),
                iobc(0, 1, 4),
                iobt(0, 0, 3),
            ],
        },
        "IOBS.S3E.B4" => IobTileData {
            node: "IOBS.S3E.B4",
            edge: Dir::S,
            tiles: 4,
            iobs: vec![
                iobc(3, 1, 1),
                iobt(3, 0, 0),
                iob(2, 2),
                iobc(1, 1, 4),
                iobt(1, 0, 3),
                ibufc(0, 1, 6),
                ibuft(0, 0, 5),
            ],
        },
        "IOBS.S3E.L1" => IobTileData {
            node: "IOBS.S3E.L1",
            edge: Dir::W,
            tiles: 1,
            iobs: vec![iob(0, 2)],
        },
        "IOBS.S3E.L2" => IobTileData {
            node: "IOBS.S3E.L2",
            edge: Dir::W,
            tiles: 2,
            iobs: vec![iobc(1, 1, 1), iobt(1, 0, 0)],
        },
        "IOBS.S3E.L3" => IobTileData {
            node: "IOBS.S3E.L3",
            edge: Dir::W,
            tiles: 3,
            iobs: vec![ibuf(0, 2), iob(1, 2), iobc(2, 1, 3), iobt(2, 0, 2)],
        },
        "IOBS.S3E.L4" => IobTileData {
            node: "IOBS.S3E.L4",
            edge: Dir::W,
            tiles: 4,
            iobs: vec![
                ibuf(0, 2),
                iobc(1, 1, 2),
                iobt(1, 0, 1),
                iobc(3, 1, 4),
                iobt(3, 0, 3),
            ],
        },

        // Spartan 3A
        "IOBS.S3A.T2" => IobTileData {
            node: "IOBS.S3A.T2",
            edge: Dir::N,
            tiles: 2,
            iobs: vec![
                iobc(0, 0, 1),
                iobt(0, 1, 0),
                ibuf(0, 2),
                iobc(1, 0, 4),
                iobt(1, 1, 3),
            ],
        },
        "IOBS.S3A.R4" => IobTileData {
            node: "IOBS.S3A.R4",
            edge: Dir::E,
            tiles: 4,
            iobs: vec![
                ibufc(3, 1, 1),
                ibuft(3, 0, 0),
                iobc(2, 1, 3),
                iobt(2, 0, 2),
                iobc(1, 1, 5),
                iobt(1, 0, 4),
                iobc(0, 1, 7),
                iobt(0, 0, 6),
            ],
        },
        "IOBS.S3A.B2" => IobTileData {
            node: "IOBS.S3A.B2",
            edge: Dir::S,
            tiles: 2,
            iobs: vec![
                iobc(1, 1, 1),
                iobt(1, 0, 0),
                ibuf(0, 2),
                iobc(0, 1, 4),
                iobt(0, 0, 3),
            ],
        },
        "IOBS.S3A.L4" => IobTileData {
            node: "IOBS.S3A.L4",
            edge: Dir::W,
            tiles: 4,
            iobs: vec![
                ibufc(0, 0, 1),
                ibuft(0, 1, 0),
                iobc(1, 0, 3),
                iobt(1, 1, 2),
                iobc(2, 0, 5),
                iobt(2, 1, 4),
                iobc(3, 0, 7),
                iobt(3, 1, 6),
            ],
        },

        _ => unreachable!(),
    }
}
