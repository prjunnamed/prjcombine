use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelSlotId, CellSlotId, TileClassId},
    dir::Dir,
    grid::TileIobId,
};

use crate::{
    chip::{ChipKind, ColumnIoKind, RowIoKind},
    defs::{self, bslots},
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum IobDiff {
    None,
    True(BelSlotId),
    Comp(BelSlotId),
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
    pub iob: BelSlotId,
    pub cell: CellSlotId,
    pub iob_id: TileIobId,
    pub ioi: BelSlotId,
    pub diff: IobDiff,
    pub kind: IobKind,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct IobTileData {
    pub tcid: TileClassId,
    pub edge: Dir,
    pub cells: usize,
    pub iobs: Vec<IobData>,
}

fn iob(tile: usize, iob: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Iob,
        diff: IobDiff::None,
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IOI[iob],
    }
}
fn iobt(tile: usize, iob: usize, other: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Iob,
        diff: IobDiff::True(bslots::IOB[other]),
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IOI[iob],
    }
}
fn iobc(tile: usize, iob: usize, other: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Iob,
        diff: IobDiff::Comp(bslots::IOB[other]),
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IOI[iob],
    }
}
fn ibuf(tile: usize, iob: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Ibuf,
        diff: IobDiff::None,
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IOI[iob],
    }
}
fn ibuft(tile: usize, iob: usize, other: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Ibuf,
        diff: IobDiff::True(bslots::IOB[other]),
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IOI[iob],
    }
}
fn ibufc(tile: usize, iob: usize, other: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Ibuf,
        diff: IobDiff::Comp(bslots::IOB[other]),
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IOI[iob],
    }
}
fn fc_ibuf(tile: usize, iob: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Ibuf,
        diff: IobDiff::None,
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IBUF[iob],
    }
}
fn fc_obuf(tile: usize, iob: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Obuf,
        diff: IobDiff::None,
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob + 4),
        ioi: defs::bslots::OBUF[iob],
    }
}
fn clkt(tile: usize, iob: usize, other: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Clk,
        diff: IobDiff::True(bslots::IOB[other]),
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IOI[iob],
    }
}
fn clkc(tile: usize, iob: usize, other: usize) -> IobData {
    IobData {
        iob: bslots::IOB[0],
        kind: IobKind::Clk,
        diff: IobDiff::Comp(bslots::IOB[other]),
        cell: CellSlotId::from_idx(tile),
        iob_id: TileIobId::from_idx(iob),
        ioi: defs::bslots::IOI[iob],
    }
}

pub fn get_iob_data_s(kind: ChipKind, col: ColumnIoKind) -> (IobTileData, CellSlotId) {
    match kind {
        ChipKind::Virtex2 => match col {
            ColumnIoKind::DoubleW(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2_SW2),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::DoubleE(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2_SE2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Virtex2P | ChipKind::Virtex2PX => match col {
            ColumnIoKind::DoubleW(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SW2),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::DoubleE(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SE2),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::DoubleEClk(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SE2_CLK),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::SingleW => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SW1),
                CellSlotId::from_idx(0),
            ),
            ColumnIoKind::SingleWAlt => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SW1_ALT),
                CellSlotId::from_idx(0),
            ),
            ColumnIoKind::SingleE => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SE1),
                CellSlotId::from_idx(0),
            ),
            ColumnIoKind::SingleEAlt => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SE1_ALT),
                CellSlotId::from_idx(0),
            ),
            _ => unreachable!(),
        },
        ChipKind::Spartan3 => match col {
            ColumnIoKind::Double(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3_S2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::FpgaCore => (
            get_iob_data(kind, defs::spartan3::tcls::IOB_FC_S),
            CellSlotId::from_idx(0),
        ),
        ChipKind::Spartan3E => match col {
            ColumnIoKind::Single => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_S1),
                CellSlotId::from_idx(0),
            ),
            ColumnIoKind::Double(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_S2),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::Triple(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_S3),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::Quad(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_S4),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match col {
            ColumnIoKind::Double(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3A_S2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
    }
}

pub fn get_iob_data_n(kind: ChipKind, col: ColumnIoKind) -> (IobTileData, CellSlotId) {
    match kind {
        ChipKind::Virtex2 => match col {
            ColumnIoKind::DoubleW(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2_NW2),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::DoubleE(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2_NE2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Virtex2P | ChipKind::Virtex2PX => match col {
            ColumnIoKind::DoubleW(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NW2),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::DoubleE(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NE2),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::DoubleEClk(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NE2_CLK),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::SingleW => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NW1),
                CellSlotId::from_idx(0),
            ),
            ColumnIoKind::SingleWAlt => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NW1_ALT),
                CellSlotId::from_idx(0),
            ),
            ColumnIoKind::SingleE => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NE1),
                CellSlotId::from_idx(0),
            ),
            ColumnIoKind::SingleEAlt => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NE1_ALT),
                CellSlotId::from_idx(0),
            ),
            _ => unreachable!(),
        },
        ChipKind::Spartan3 => match col {
            ColumnIoKind::Double(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3_N2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::FpgaCore => (
            get_iob_data(kind, defs::spartan3::tcls::IOB_FC_N),
            CellSlotId::from_idx(0),
        ),
        ChipKind::Spartan3E => match col {
            ColumnIoKind::Single => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_N1),
                CellSlotId::from_idx(0),
            ),
            ColumnIoKind::Double(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_N2),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::Triple(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_N3),
                CellSlotId::from_idx(i.into()),
            ),
            ColumnIoKind::Quad(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_N4),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match col {
            ColumnIoKind::Double(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3A_N2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
    }
}

pub fn get_iob_data_w(kind: ChipKind, row: RowIoKind) -> (IobTileData, CellSlotId) {
    match kind {
        ChipKind::Virtex2 => match row {
            RowIoKind::DoubleS(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2_WS2),
                CellSlotId::from_idx(i.into()),
            ),
            RowIoKind::DoubleN(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2_WN2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Virtex2P | ChipKind::Virtex2PX => match row {
            RowIoKind::DoubleS(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_WS2),
                CellSlotId::from_idx(i.into()),
            ),
            RowIoKind::DoubleN(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_WN2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Spartan3 => match row {
            RowIoKind::Single => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3_W1),
                CellSlotId::from_idx(0),
            ),
            _ => unreachable!(),
        },
        ChipKind::FpgaCore => (
            get_iob_data(kind, defs::spartan3::tcls::IOB_FC_W),
            CellSlotId::from_idx(0),
        ),
        ChipKind::Spartan3E => match row {
            RowIoKind::Single => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_W1),
                CellSlotId::from_idx(0),
            ),
            RowIoKind::Double(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_W2),
                CellSlotId::from_idx(i.into()),
            ),
            RowIoKind::Triple(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_W3),
                CellSlotId::from_idx(i.into()),
            ),
            RowIoKind::Quad(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_W4),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match row {
            RowIoKind::Quad(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3A_W4),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
    }
}

pub fn get_iob_data_e(kind: ChipKind, row: RowIoKind) -> (IobTileData, CellSlotId) {
    match kind {
        ChipKind::Virtex2 => match row {
            RowIoKind::DoubleS(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2_ES2),
                CellSlotId::from_idx(i.into()),
            ),
            RowIoKind::DoubleN(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2_EN2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Virtex2P | ChipKind::Virtex2PX => match row {
            RowIoKind::DoubleS(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_ES2),
                CellSlotId::from_idx(i.into()),
            ),
            RowIoKind::DoubleN(i) => (
                get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_EN2),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Spartan3 => match row {
            RowIoKind::Single => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3_E1),
                CellSlotId::from_idx(0),
            ),
            _ => unreachable!(),
        },
        ChipKind::FpgaCore => (
            get_iob_data(kind, defs::spartan3::tcls::IOB_FC_E),
            CellSlotId::from_idx(0),
        ),
        ChipKind::Spartan3E => match row {
            RowIoKind::Single => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_E1),
                CellSlotId::from_idx(0),
            ),
            RowIoKind::Double(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_E2),
                CellSlotId::from_idx(i.into()),
            ),
            RowIoKind::Triple(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_E3),
                CellSlotId::from_idx(i.into()),
            ),
            RowIoKind::Quad(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_E4),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => match row {
            RowIoKind::Quad(i) => (
                get_iob_data(kind, defs::spartan3::tcls::IOB_S3A_E4),
                CellSlotId::from_idx(i.into()),
            ),
            _ => unreachable!(),
        },
    }
}

pub fn get_iob_tiles(kind: ChipKind) -> Vec<IobTileData> {
    match kind {
        ChipKind::Virtex2 => vec![
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2_NW2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2_NE2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2_ES2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2_EN2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2_SW2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2_SE2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2_WS2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2_WN2),
        ],
        ChipKind::Virtex2P | ChipKind::Virtex2PX => vec![
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NW1),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NW1_ALT),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NE1),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NE1_ALT),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NW2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NE2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_NE2_CLK),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_ES2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_EN2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SW1),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SW1_ALT),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SE1),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SE1_ALT),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SW2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SE2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_SE2_CLK),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_WS2),
            get_iob_data(kind, defs::virtex2::tcls::IOB_V2P_WN2),
        ],
        ChipKind::Spartan3 => vec![
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3_N2),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3_E1),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3_S2),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3_W1),
        ],
        ChipKind::FpgaCore => vec![
            get_iob_data(kind, defs::spartan3::tcls::IOB_FC_N),
            get_iob_data(kind, defs::spartan3::tcls::IOB_FC_E),
            get_iob_data(kind, defs::spartan3::tcls::IOB_FC_S),
            get_iob_data(kind, defs::spartan3::tcls::IOB_FC_W),
        ],
        ChipKind::Spartan3E => vec![
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_N1),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_N2),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_N3),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_N4),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_E1),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_E2),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_E3),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_E4),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_S1),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_S2),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_S3),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_S4),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_W1),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_W2),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_W3),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3E_W4),
        ],
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => vec![
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3A_N2),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3A_E4),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3A_S2),
            get_iob_data(kind, defs::spartan3::tcls::IOB_S3A_W4),
        ],
    }
}

pub fn get_iob_data(kind: ChipKind, tcid: TileClassId) -> IobTileData {
    let mut data = if kind.is_virtex2() {
        match tcid {
            // Virtex 2
            defs::virtex2::tcls::IOB_V2_NW2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2_NW2,
                edge: Dir::N,
                cells: 2,
                iobs: vec![
                    iobc(0, 3, 1),
                    iobt(0, 2, 0),
                    iobc(0, 1, 3),
                    iobt(0, 0, 2),
                    iobc(1, 1, 5),
                    iobt(1, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2_NE2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2_NE2,
                edge: Dir::N,
                cells: 2,
                iobs: vec![
                    iobc(0, 3, 1),
                    iobt(0, 2, 0),
                    iobc(1, 3, 3),
                    iobt(1, 2, 2),
                    iobc(1, 1, 5),
                    iobt(1, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2_EN2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2_EN2,
                edge: Dir::E,
                cells: 2,
                iobs: vec![
                    iobc(1, 3, 1),
                    iobt(1, 2, 0),
                    iobc(1, 1, 3),
                    iobt(1, 0, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2_ES2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2_ES2,
                edge: Dir::E,
                cells: 2,
                iobs: vec![
                    iobc(1, 3, 1),
                    iobt(1, 2, 0),
                    iobc(0, 3, 3),
                    iobt(0, 2, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2_SE2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2_SE2,
                edge: Dir::S,
                cells: 2,
                iobs: vec![
                    iobc(1, 3, 1),
                    iobt(1, 2, 0),
                    iobc(1, 1, 3),
                    iobt(1, 0, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2_SW2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2_SW2,
                edge: Dir::S,
                cells: 2,
                iobs: vec![
                    iobc(1, 3, 1),
                    iobt(1, 2, 0),
                    iobc(0, 3, 3),
                    iobt(0, 2, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2_WS2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2_WS2,
                edge: Dir::W,
                cells: 2,
                iobs: vec![
                    iobt(0, 0, 1),
                    iobc(0, 1, 0),
                    iobt(0, 2, 3),
                    iobc(0, 3, 2),
                    iobt(1, 2, 5),
                    iobc(1, 3, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2_WN2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2_WN2,
                edge: Dir::W,
                cells: 2,
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
            defs::virtex2::tcls::IOB_V2P_NW1 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_NW1,
                edge: Dir::N,
                cells: 1,
                iobs: vec![iob(0, 2), iobc(0, 1, 2), iobt(0, 0, 1)],
            },
            defs::virtex2::tcls::IOB_V2P_NW1_ALT => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_NW1_ALT,
                edge: Dir::N,
                cells: 1,
                iobs: vec![iobc(0, 2, 1), iobt(0, 1, 0), iob(0, 0)],
            },
            defs::virtex2::tcls::IOB_V2P_NE1 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_NE1,
                edge: Dir::N,
                cells: 1,
                iobs: vec![iobc(0, 3, 1), iobt(0, 2, 0), iob(0, 1)],
            },
            defs::virtex2::tcls::IOB_V2P_NE1_ALT => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_NE1_ALT,
                edge: Dir::N,
                cells: 1,
                iobs: vec![iob(0, 3), iobc(0, 2, 2), iobt(0, 1, 1)],
            },
            defs::virtex2::tcls::IOB_V2P_NW2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_NW2,
                edge: Dir::N,
                cells: 2,
                iobs: vec![
                    iobc(0, 3, 1),
                    iobt(0, 2, 0),
                    iobc(0, 1, 3),
                    iobt(0, 0, 2),
                    iobc(1, 1, 5),
                    iobt(1, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_NE2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_NE2,
                edge: Dir::N,
                cells: 2,
                iobs: vec![
                    iobc(0, 3, 1),
                    iobt(0, 2, 0),
                    iobc(1, 3, 3),
                    iobt(1, 2, 2),
                    iobc(1, 1, 5),
                    iobt(1, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_NE2_CLK => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_NE2_CLK,
                edge: Dir::N,
                cells: 2,
                iobs: vec![
                    iobc(0, 3, 1),
                    iobt(0, 2, 0),
                    iobc(1, 3, 3),
                    iobt(1, 2, 2),
                    clkc(1, 1, 5),
                    clkt(1, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_EN2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_EN2,
                edge: Dir::E,
                cells: 2,
                iobs: vec![
                    iobc(1, 3, 1),
                    iobt(1, 2, 0),
                    iobc(1, 1, 3),
                    iobt(1, 0, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_ES2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_ES2,
                edge: Dir::E,
                cells: 2,
                iobs: vec![
                    iobc(1, 3, 1),
                    iobt(1, 2, 0),
                    iobc(0, 3, 3),
                    iobt(0, 2, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_SE1 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_SE1,
                edge: Dir::S,
                cells: 1,
                iobs: vec![iob(0, 2), iobc(0, 1, 2), iobt(0, 0, 1)],
            },
            defs::virtex2::tcls::IOB_V2P_SE1_ALT => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_SE1_ALT,
                edge: Dir::S,
                cells: 1,
                iobs: vec![iobc(0, 2, 1), iobt(0, 1, 0), iob(0, 0)],
            },
            defs::virtex2::tcls::IOB_V2P_SW1 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_SW1,
                edge: Dir::S,
                cells: 1,
                iobs: vec![iobc(0, 3, 1), iobt(0, 2, 0), iob(0, 1)],
            },
            defs::virtex2::tcls::IOB_V2P_SW1_ALT => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_SW1_ALT,
                edge: Dir::S,
                cells: 1,
                iobs: vec![iob(0, 3), iobc(0, 2, 2), iobt(0, 1, 1)],
            },
            defs::virtex2::tcls::IOB_V2P_SE2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_SE2,
                edge: Dir::S,
                cells: 2,
                iobs: vec![
                    iobc(1, 3, 1),
                    iobt(1, 2, 0),
                    iobc(1, 1, 3),
                    iobt(1, 0, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_SW2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_SW2,
                edge: Dir::S,
                cells: 2,
                iobs: vec![
                    iobc(1, 3, 1),
                    iobt(1, 2, 0),
                    iobc(0, 3, 3),
                    iobt(0, 2, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_SE2_CLK => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_SE2_CLK,
                edge: Dir::S,
                cells: 2,
                iobs: vec![
                    clkc(1, 3, 1),
                    clkt(1, 2, 0),
                    iobc(1, 1, 3),
                    iobt(1, 0, 2),
                    iobc(0, 1, 5),
                    iobt(0, 0, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_WS2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_WS2,
                edge: Dir::W,
                cells: 2,
                iobs: vec![
                    iobt(0, 0, 1),
                    iobc(0, 1, 0),
                    iobt(0, 2, 3),
                    iobc(0, 3, 2),
                    iobt(1, 2, 5),
                    iobc(1, 3, 4),
                ],
            },
            defs::virtex2::tcls::IOB_V2P_WN2 => IobTileData {
                tcid: defs::virtex2::tcls::IOB_V2P_WN2,
                edge: Dir::W,
                cells: 2,
                iobs: vec![
                    iobt(0, 0, 1),
                    iobc(0, 1, 0),
                    iobt(1, 0, 3),
                    iobc(1, 1, 2),
                    iobt(1, 2, 5),
                    iobc(1, 3, 4),
                ],
            },
            _ => unreachable!(),
        }
    } else {
        match tcid {
            // Spartan 3
            defs::spartan3::tcls::IOB_S3_N2 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3_N2,
                edge: Dir::N,
                cells: 2,
                iobs: vec![
                    iob(0, 2),
                    iobc(0, 1, 2),
                    iobt(0, 0, 1),
                    iobc(1, 1, 4),
                    iobt(1, 0, 3),
                ],
            },
            defs::spartan3::tcls::IOB_S3_E1 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3_E1,
                edge: Dir::E,
                cells: 1,
                iobs: vec![iobc(0, 1, 1), iobt(0, 0, 0)],
            },
            defs::spartan3::tcls::IOB_S3_S2 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3_S2,
                edge: Dir::S,
                cells: 2,
                iobs: vec![
                    iob(1, 2),
                    iobc(1, 1, 2),
                    iobt(1, 0, 1),
                    iobc(0, 1, 4),
                    iobt(0, 0, 3),
                ],
            },
            defs::spartan3::tcls::IOB_S3_W1 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3_W1,
                edge: Dir::W,
                cells: 1,
                iobs: vec![iobc(0, 0, 1), iobt(0, 1, 0)],
            },

            // FPGA core
            defs::spartan3::tcls::IOB_FC_N => IobTileData {
                tcid: defs::spartan3::tcls::IOB_FC_N,
                edge: Dir::N,
                cells: 1,
                iobs: vec![
                    fc_ibuf(0, 3),
                    fc_obuf(0, 3),
                    fc_ibuf(0, 2),
                    fc_obuf(0, 2),
                    fc_ibuf(0, 1),
                    fc_obuf(0, 1),
                    fc_ibuf(0, 0),
                    fc_obuf(0, 0),
                ],
            },
            defs::spartan3::tcls::IOB_FC_E => IobTileData {
                tcid: defs::spartan3::tcls::IOB_FC_E,
                edge: Dir::E,
                cells: 1,
                iobs: vec![
                    fc_ibuf(0, 3),
                    fc_obuf(0, 3),
                    fc_ibuf(0, 2),
                    fc_obuf(0, 2),
                    fc_ibuf(0, 1),
                    fc_obuf(0, 1),
                    fc_ibuf(0, 0),
                    fc_obuf(0, 0),
                ],
            },
            defs::spartan3::tcls::IOB_FC_S => IobTileData {
                tcid: defs::spartan3::tcls::IOB_FC_S,
                edge: Dir::S,
                cells: 1,
                iobs: vec![
                    fc_ibuf(0, 3),
                    fc_obuf(0, 3),
                    fc_ibuf(0, 2),
                    fc_obuf(0, 2),
                    fc_ibuf(0, 1),
                    fc_obuf(0, 1),
                    fc_ibuf(0, 0),
                    fc_obuf(0, 0),
                ],
            },
            defs::spartan3::tcls::IOB_FC_W => IobTileData {
                tcid: defs::spartan3::tcls::IOB_FC_W,
                edge: Dir::W,
                cells: 1,
                iobs: vec![
                    fc_ibuf(0, 0),
                    fc_obuf(0, 0),
                    fc_ibuf(0, 1),
                    fc_obuf(0, 1),
                    fc_ibuf(0, 2),
                    fc_obuf(0, 2),
                    fc_ibuf(0, 3),
                    fc_obuf(0, 3),
                ],
            },

            // Spartan 3E
            defs::spartan3::tcls::IOB_S3E_N1 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_N1,
                edge: Dir::N,
                cells: 1,
                iobs: vec![iob(0, 2)],
            },
            defs::spartan3::tcls::IOB_S3E_N2 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_N2,
                edge: Dir::N,
                cells: 2,
                iobs: vec![iobc(0, 1, 1), iobt(0, 0, 0), ibuf(1, 2)],
            },
            defs::spartan3::tcls::IOB_S3E_N3 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_N3,
                edge: Dir::N,
                cells: 3,
                iobs: vec![
                    iobc(0, 1, 1),
                    iobt(0, 0, 0),
                    ibuf(1, 2),
                    iobc(2, 1, 4),
                    iobt(2, 0, 3),
                ],
            },
            defs::spartan3::tcls::IOB_S3E_N4 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_N4,
                edge: Dir::N,
                cells: 4,
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
            defs::spartan3::tcls::IOB_S3E_E1 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_E1,
                edge: Dir::E,
                cells: 1,
                iobs: vec![iob(0, 2)],
            },
            defs::spartan3::tcls::IOB_S3E_E2 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_E2,
                edge: Dir::E,
                cells: 2,
                iobs: vec![iobc(0, 1, 1), iobt(0, 0, 0)],
            },
            defs::spartan3::tcls::IOB_S3E_E3 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_E3,
                edge: Dir::E,
                cells: 3,
                iobs: vec![ibuf(2, 2), iob(1, 2), iobc(0, 1, 3), iobt(0, 0, 2)],
            },
            defs::spartan3::tcls::IOB_S3E_E4 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_E4,
                edge: Dir::E,
                cells: 4,
                iobs: vec![
                    ibuf(3, 2),
                    iobc(2, 1, 2),
                    iobt(2, 0, 1),
                    iobc(0, 1, 4),
                    iobt(0, 0, 3),
                ],
            },
            defs::spartan3::tcls::IOB_S3E_S1 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_S1,
                edge: Dir::S,
                cells: 1,
                iobs: vec![iob(0, 2)],
            },
            defs::spartan3::tcls::IOB_S3E_S2 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_S2,
                edge: Dir::S,
                cells: 2,
                iobs: vec![iobc(1, 1, 1), iobt(1, 0, 0), ibuf(0, 2)],
            },
            defs::spartan3::tcls::IOB_S3E_S3 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_S3,
                edge: Dir::S,
                cells: 3,
                iobs: vec![
                    iobc(2, 1, 1),
                    iobt(2, 0, 0),
                    ibuf(1, 2),
                    iobc(0, 1, 4),
                    iobt(0, 0, 3),
                ],
            },
            defs::spartan3::tcls::IOB_S3E_S4 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_S4,
                edge: Dir::S,
                cells: 4,
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
            defs::spartan3::tcls::IOB_S3E_W1 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_W1,
                edge: Dir::W,
                cells: 1,
                iobs: vec![iob(0, 2)],
            },
            defs::spartan3::tcls::IOB_S3E_W2 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_W2,
                edge: Dir::W,
                cells: 2,
                iobs: vec![iobc(1, 1, 1), iobt(1, 0, 0)],
            },
            defs::spartan3::tcls::IOB_S3E_W3 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_W3,
                edge: Dir::W,
                cells: 3,
                iobs: vec![ibuf(0, 2), iob(1, 2), iobc(2, 1, 3), iobt(2, 0, 2)],
            },
            defs::spartan3::tcls::IOB_S3E_W4 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3E_W4,
                edge: Dir::W,
                cells: 4,
                iobs: vec![
                    ibuf(0, 2),
                    iobc(1, 1, 2),
                    iobt(1, 0, 1),
                    iobc(3, 1, 4),
                    iobt(3, 0, 3),
                ],
            },

            // Spartan 3A
            defs::spartan3::tcls::IOB_S3A_N2 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3A_N2,
                edge: Dir::N,
                cells: 2,
                iobs: vec![
                    iobc(0, 0, 1),
                    iobt(0, 1, 0),
                    ibuf(0, 2),
                    iobc(1, 0, 4),
                    iobt(1, 1, 3),
                ],
            },
            defs::spartan3::tcls::IOB_S3A_E4 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3A_E4,
                edge: Dir::E,
                cells: 4,
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
            defs::spartan3::tcls::IOB_S3A_S2 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3A_S2,
                edge: Dir::S,
                cells: 2,
                iobs: vec![
                    iobc(1, 1, 1),
                    iobt(1, 0, 0),
                    ibuf(0, 2),
                    iobc(0, 1, 4),
                    iobt(0, 0, 3),
                ],
            },
            defs::spartan3::tcls::IOB_S3A_W4 => IobTileData {
                tcid: defs::spartan3::tcls::IOB_S3A_W4,
                edge: Dir::W,
                cells: 4,
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
    };
    for (i, iob) in data.iobs.iter_mut().enumerate() {
        if let Some(idx) = bslots::IREG.index_of(iob.ioi) {
            iob.iob = bslots::IBUF[idx];
        } else if let Some(idx) = bslots::OREG.index_of(iob.ioi) {
            iob.iob = bslots::OBUF[idx];
        } else {
            iob.iob = bslots::IOB[i];
        }
    }
    data
}
