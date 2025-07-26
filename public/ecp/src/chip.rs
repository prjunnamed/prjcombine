use std::{collections::BTreeMap, fmt::Display};

use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_interconnect::{
    dir::{Dir, DirH, DirHV, DirV},
    grid::{BelCoord, CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::bels;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum ChipKind {
    Ecp,
    Xp,
    MachXo,
    Ecp2,
    Ecp2M,
    Xp2,
    Ecp3,
    Ecp3A,
}

impl ChipKind {
    pub fn has_x0_branch(self) -> bool {
        matches!(self, ChipKind::Ecp | ChipKind::Xp | ChipKind::MachXo)
    }

    pub fn has_ecp_plc(self) -> bool {
        matches!(self, ChipKind::Ecp | ChipKind::Xp | ChipKind::MachXo)
    }

    pub fn has_ecp2_plc(self) -> bool {
        matches!(
            self,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A
        )
    }

    pub fn has_ecp3_dsp(self) -> bool {
        matches!(self, ChipKind::Ecp3 | ChipKind::Ecp3A)
    }

    pub fn has_x1_bi(self) -> bool {
        matches!(
            self,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A
        )
    }

    pub fn has_distributed_sclk(self) -> bool {
        matches!(
            self,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A
        )
    }

    pub fn has_distributed_sclk_ecp3(self) -> bool {
        matches!(self, ChipKind::Ecp3 | ChipKind::Ecp3A)
    }
}

impl Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Ecp => write!(f, "ecp"),
            ChipKind::Xp => write!(f, "xp"),
            ChipKind::MachXo => write!(f, "machxo"),
            ChipKind::Ecp2 => write!(f, "ecp2"),
            ChipKind::Ecp2M => write!(f, "ecp2m"),
            ChipKind::Xp2 => write!(f, "xp2"),
            ChipKind::Ecp3 => write!(f, "ecp3"),
            ChipKind::Ecp3A => write!(f, "ecp3a"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: EntityVec<ColId, Column>,
    pub rows: EntityVec<RowId, Row>,
    pub col_clk: ColId,
    pub row_clk: RowId,
    pub special_loc: BTreeMap<SpecialLocKey, CellCoord>,
    pub special_io: BTreeMap<SpecialIoKey, EdgeIoCoord>,
    pub io_direct_plc: BTreeMap<EdgeIoCoord, (CellCoord, u8)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum RowKind {
    Plc,
    Fplc,
    Io,
    Ebr,
    Dsp,
}

impl Display for RowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RowKind::Plc => write!(f, "PLC"),
            RowKind::Fplc => write!(f, "FPLC"),
            RowKind::Io => write!(f, "IO"),
            RowKind::Ebr => write!(f, "EBR"),
            RowKind::Dsp => write!(f, "DSP"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum IoGroupKind {
    None,
    Double,
    DoubleA,
    DoubleB,
    DoubleDqs,
    DoubleDummy,
    Quad,
    QuadReverse,
    QuadDqs,
    QuadDqsDummy,
    Hex,
    HexReverse,
    Serdes,
}

impl Display for IoGroupKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoGroupKind::None => write!(f, "NONE"),
            IoGroupKind::Double => write!(f, "DOUBLE"),
            IoGroupKind::DoubleA => write!(f, "DOUBLE_A"),
            IoGroupKind::DoubleB => write!(f, "DOUBLE_B"),
            IoGroupKind::DoubleDummy => write!(f, "DOUBLE_DUMMY"),
            IoGroupKind::DoubleDqs => write!(f, "DOUBLE_DQS"),
            IoGroupKind::Quad => write!(f, "QUAD"),
            IoGroupKind::QuadReverse => write!(f, "QUAD_REVERSE"),
            IoGroupKind::QuadDqs => write!(f, "QUAD_DQS"),
            IoGroupKind::QuadDqsDummy => write!(f, "QUAD_DQS_DUMMY"),
            IoGroupKind::Hex => write!(f, "HEX"),
            IoGroupKind::HexReverse => write!(f, "HEX_REVERSE"),
            IoGroupKind::Serdes => write!(f, "SERDES"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum IoKind {
    Dummy,
    Io,
    IoPll,
    Sio,
    Xsio,
    Dqs,
    SDqs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Column {
    pub io_s: IoGroupKind,
    pub io_n: IoGroupKind,
    pub bank_s: Option<u32>,
    pub bank_n: Option<u32>,
    pub eclk_tap_s: bool,
    pub eclk_tap_n: bool,
    pub pclk_leaf_break: bool,
    pub sdclk_break: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Row {
    pub kind: RowKind,
    pub io_w: IoGroupKind,
    pub io_e: IoGroupKind,
    pub bank_w: Option<u32>,
    pub bank_e: Option<u32>,
    pub sclk_break: bool,
    pub pclk_break: bool,
    pub pclk_drive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct PllLoc {
    pub quad: DirHV,
    pub idx: u8,
}

impl PllLoc {
    pub fn new(quad: DirHV, idx: u8) -> Self {
        Self { quad, idx }
    }

    pub fn new_hv(h: DirH, v: DirV, idx: u8) -> Self {
        Self {
            quad: DirHV::new(h, v),
            idx,
        }
    }
}

impl Display for PllLoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.quad, self.idx)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum PllPad {
    PllIn0,
    PllIn1,
    PllFb,
    DllIn0,
    DllIn1,
    DllFb,
}

impl Display for PllPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PllPad::PllIn0 => write!(f, "PLL_IN0"),
            PllPad::PllIn1 => write!(f, "PLL_IN1"),
            PllPad::PllFb => write!(f, "PLL_FB"),
            PllPad::DllIn0 => write!(f, "DLL_IN0"),
            PllPad::DllIn1 => write!(f, "DLL_IN1"),
            PllPad::DllFb => write!(f, "DLL_FB"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SpecialIoKey {
    Clock(Dir, u8),
    Pll(PllPad, PllLoc),
    Vref1(u32),
    Vref2(u32),
    Gsr,
    TsAll,
    WriteN,
    CsN,
    Cs1N,
    D(u8),
    Dout,
    Di,
    Busy,
    MClk,
    SleepN,
    // XP2 stuff
    InitB,
    SpiSdi,
    SpiSdo,
    Cclk,
    SpiCCsB,
    SpiPCsB,
    M1,
    Done,
    ProgB,
}

impl Display for SpecialIoKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialIoKey::Clock(dir, i) => write!(f, "CLOCK_{dir}{i}"),
            SpecialIoKey::Pll(pad, loc) => write!(f, "{pad}_{loc}"),
            SpecialIoKey::Vref1(bank) => write!(f, "VREF1_{bank}"),
            SpecialIoKey::Vref2(bank) => write!(f, "VREF2_{bank}"),
            SpecialIoKey::Gsr => write!(f, "GSR"),
            SpecialIoKey::TsAll => write!(f, "TSALL"),
            SpecialIoKey::WriteN => write!(f, "WRITE_N"),
            SpecialIoKey::CsN => write!(f, "CS_N"),
            SpecialIoKey::Cs1N => write!(f, "CS1_N"),
            SpecialIoKey::D(i) => write!(f, "D{i}"),
            SpecialIoKey::Dout => write!(f, "DOUT"),
            SpecialIoKey::Di => write!(f, "DI"),
            SpecialIoKey::Busy => write!(f, "BUSY"),
            SpecialIoKey::MClk => write!(f, "MCLK"),
            SpecialIoKey::SleepN => write!(f, "SLEEP_N"),
            SpecialIoKey::InitB => write!(f, "INIT_B"),
            SpecialIoKey::SpiSdi => write!(f, "SPI_SDI"),
            SpecialIoKey::SpiSdo => write!(f, "SPI_SDO"),
            SpecialIoKey::Cclk => write!(f, "CCLK"),
            SpecialIoKey::SpiCCsB => write!(f, "SPI_C_CS_B"),
            SpecialIoKey::SpiPCsB => write!(f, "SPI_P_CS_B"),
            SpecialIoKey::M1 => write!(f, "M1"),
            SpecialIoKey::Done => write!(f, "DONE"),
            SpecialIoKey::ProgB => write!(f, "PROG_B"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SpecialLocKey {
    Pll(PllLoc),
    Ebr(u8),
    PclkIn(Dir, u8),
    PclkInMid(u8),
    SclkIn(Dir, u8),
    Config,
    ConfigBits,
    Osc,
}

impl Display for SpecialLocKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialLocKey::Pll(dir) => write!(f, "PLL_{dir}"),
            SpecialLocKey::Ebr(idx) => write!(f, "EBR{idx}"),
            SpecialLocKey::Config => write!(f, "CONFIG"),
            SpecialLocKey::ConfigBits => write!(f, "CONFIG_BITS"),
            SpecialLocKey::Osc => write!(f, "OSC"),
            SpecialLocKey::PclkIn(dir, idx) => write!(f, "PCLK_IN_{dir}{idx}"),
            SpecialLocKey::PclkInMid(idx) => write!(f, "PCLK_IN_M{idx}"),
            SpecialLocKey::SclkIn(dir, idx) => write!(f, "SCLK_IN_{dir}{idx}"),
        }
    }
}

impl Chip {
    pub fn col_w(&self) -> ColId {
        self.columns.first_id().unwrap()
    }

    pub fn col_e(&self) -> ColId {
        self.columns.last_id().unwrap()
    }

    pub fn col_edge(&self, edge: DirH) -> ColId {
        match edge {
            DirH::W => self.col_w(),
            DirH::E => self.col_e(),
        }
    }

    pub fn row_s(&self) -> RowId {
        self.rows.first_id().unwrap()
    }

    pub fn row_n(&self) -> RowId {
        self.rows.last_id().unwrap()
    }

    pub fn row_edge(&self, edge: DirV) -> RowId {
        match edge {
            DirV::S => self.row_s(),
            DirV::N => self.row_n(),
        }
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> BelCoord {
        if matches!(self.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) && io.iob().to_idx() >= 4 {
            let (col, row, iob) = match io {
                EdgeIoCoord::W(row, iob) => (self.col_w() + 1, row, iob),
                EdgeIoCoord::E(row, iob) => (self.col_e() - 1, row, iob),
                _ => unreachable!(),
            };
            let slot = bels::IO[iob.to_idx() - 4];
            CellCoord::new(DieId::from_idx(0), col, row).bel(slot)
        } else {
            let (col, row, iob) = match io {
                EdgeIoCoord::N(col, iob) => (col, self.row_n(), iob),
                EdgeIoCoord::E(row, iob) => (self.col_e(), row, iob),
                EdgeIoCoord::S(col, iob) => (col, self.row_s(), iob),
                EdgeIoCoord::W(row, iob) => (self.col_w(), row, iob),
            };
            let slot = bels::IO[iob.to_idx()];
            CellCoord::new(DieId::from_idx(0), col, row).bel(slot)
        }
    }

    pub fn get_io_crd(&self, bel: BelCoord) -> EdgeIoCoord {
        let iob = TileIobId::from_idx(bels::IO.iter().position(|&x| x == bel.slot).unwrap());
        if bel.col == self.col_w() {
            EdgeIoCoord::W(bel.row, iob)
        } else if bel.col == self.col_e() {
            EdgeIoCoord::E(bel.row, iob)
        } else if bel.row == self.row_s() {
            EdgeIoCoord::S(bel.col, iob)
        } else if bel.row == self.row_n() {
            EdgeIoCoord::N(bel.col, iob)
        } else if bel.col == self.col_w() + 1 {
            let iob =
                TileIobId::from_idx(bels::IO.iter().position(|&x| x == bel.slot).unwrap() + 4);
            EdgeIoCoord::W(bel.row, iob)
        } else if bel.col == self.col_e() - 1 {
            let iob =
                TileIobId::from_idx(bels::IO.iter().position(|&x| x == bel.slot).unwrap() + 4);
            EdgeIoCoord::E(bel.row, iob)
        } else {
            unreachable!()
        }
    }

    pub fn get_io_bank(&self, io: EdgeIoCoord) -> u32 {
        match io {
            EdgeIoCoord::W(row, _) => self.rows[row].bank_w,
            EdgeIoCoord::E(row, _) => self.rows[row].bank_e,
            EdgeIoCoord::S(col, _) => self.columns[col].bank_s,
            EdgeIoCoord::N(col, _) => self.columns[col].bank_n,
        }
        .unwrap()
    }

    pub fn get_io_kind(&self, io: EdgeIoCoord) -> IoKind {
        match self.kind {
            ChipKind::MachXo => IoKind::Io,
            ChipKind::Ecp | ChipKind::Xp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
                match io {
                    EdgeIoCoord::W(row, iob) => match (self.rows[row].io_w, iob.to_idx()) {
                        (IoGroupKind::DoubleDqs, 0) => IoKind::Dqs,
                        (IoGroupKind::DoubleA, 1) => IoKind::Dummy,
                        (IoGroupKind::DoubleB, 0) => IoKind::Dummy,
                        (IoGroupKind::DoubleDummy, _) => IoKind::Dummy,
                        _ => IoKind::Io,
                    },
                    EdgeIoCoord::E(row, iob) => match (self.rows[row].io_e, iob.to_idx()) {
                        (IoGroupKind::DoubleDqs, 0) => IoKind::Dqs,
                        (IoGroupKind::DoubleA, 1) => IoKind::Dummy,
                        (IoGroupKind::DoubleB, 0) => IoKind::Dummy,
                        (IoGroupKind::DoubleDummy, _) => IoKind::Dummy,
                        _ => IoKind::Io,
                    },
                    EdgeIoCoord::S(col, iob) => match (self.columns[col].io_s, iob.to_idx()) {
                        (IoGroupKind::DoubleDqs, 0) => IoKind::Dqs,
                        (IoGroupKind::DoubleA, 1) => IoKind::Dummy,
                        (IoGroupKind::DoubleB, 0) => IoKind::Dummy,
                        (IoGroupKind::DoubleDummy, _) => IoKind::Dummy,
                        _ => IoKind::Io,
                    },
                    EdgeIoCoord::N(col, iob) => match (self.columns[col].io_n, iob.to_idx()) {
                        (IoGroupKind::DoubleDqs, 0) => IoKind::Dqs,
                        (IoGroupKind::DoubleA, 1) => IoKind::Dummy,
                        (IoGroupKind::DoubleB, 0) => IoKind::Dummy,
                        (IoGroupKind::DoubleDummy, _) => IoKind::Dummy,
                        _ => IoKind::Io,
                    },
                }
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => match io {
                EdgeIoCoord::W(row, iob) => match (self.rows[row].io_w, iob.to_idx()) {
                    (_, 4..8) => IoKind::IoPll,
                    (IoGroupKind::QuadDqs, 2) => IoKind::Dqs,
                    (IoGroupKind::QuadDqs, 3) => IoKind::Xsio,
                    _ => IoKind::Io,
                },
                EdgeIoCoord::E(row, iob) => match (
                    self.rows[row].io_e,
                    self.rows[row].bank_e.unwrap(),
                    iob.to_idx(),
                ) {
                    (_, _, 4..8) => IoKind::IoPll,
                    (_, 8, _) => IoKind::Xsio,
                    (IoGroupKind::QuadDqs, _, 2) => IoKind::Dqs,
                    (IoGroupKind::QuadDqs, _, 3) => IoKind::Xsio,
                    _ => IoKind::Io,
                },
                EdgeIoCoord::S(..) => IoKind::Xsio,
                EdgeIoCoord::N(col, iob) => match (
                    self.columns[col].io_n,
                    self.columns[col].bank_n.unwrap(),
                    iob.to_idx(),
                ) {
                    (_, 8, _) => IoKind::Xsio,
                    (IoGroupKind::QuadDqs, _, 2) => IoKind::SDqs,
                    (IoGroupKind::QuadDqs, _, 3) => IoKind::Xsio,
                    _ => IoKind::Sio,
                },
            },
        }
    }

    pub fn col_sclk_idx(&self, col: ColId) -> usize {
        assert!(self.kind.has_distributed_sclk());
        if self.kind.has_distributed_sclk_ecp3() {
            (col - self.col_clk).rem_euclid(4) as usize
        } else {
            match col.to_idx() % 4 {
                0 => 0,
                1 => 3,
                2 => 2,
                3 => 1,
                _ => unreachable!(),
            }
        }
    }

    pub fn bel_cibtest_sel(&self) -> BelCoord {
        assert_eq!(self.kind, ChipKind::MachXo);
        assert!(self.special_loc.contains_key(&SpecialLocKey::Ebr(0)));
        CellCoord::new(DieId::from_idx(0), self.col_w(), self.row_n() - 1).bel(bels::CIBTEST_SEL)
    }

    pub fn bel_clk_root(&self) -> BelCoord {
        CellCoord::new(DieId::from_idx(0), self.col_clk, self.row_clk).bel(bels::CLK_ROOT)
    }

    pub fn bel_dqsdll(&self, io: CellCoord) -> BelCoord {
        match self.kind {
            ChipKind::Ecp | ChipKind::Xp => self.bel_dqsdll_ecp(if io.row < self.row_clk {
                DirV::S
            } else {
                DirV::N
            }),
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.bel_dqsdll_ecp2(if io.col < self.col_clk {
                    DirH::W
                } else {
                    DirH::E
                })
            }
            ChipKind::MachXo => unreachable!(),
        }
    }

    pub fn bel_dqsdll_ecp(&self, edge: DirV) -> BelCoord {
        assert!(matches!(self.kind, ChipKind::Ecp | ChipKind::Xp));
        match edge {
            DirV::S => {
                CellCoord::new(DieId::from_idx(0), self.col_clk - 2, self.row_s()).bel(bels::DQSDLL)
            }
            DirV::N => {
                CellCoord::new(DieId::from_idx(0), self.col_clk + 1, self.row_n()).bel(bels::DQSDLL)
            }
        }
    }

    pub fn bel_dqsdll_ecp2(&self, edge: DirH) -> BelCoord {
        match self.kind {
            ChipKind::Ecp2 | ChipKind::Ecp2M => self.special_loc
                [&SpecialLocKey::Pll(PllLoc::new_hv(edge, DirV::S, 0))]
                .bel(bels::DQSDLL),
            ChipKind::Xp2 => CellCoord::new(DieId::from_idx(0), self.col_edge(edge), self.row_clk)
                .bel(bels::DQSDLL),
            ChipKind::Ecp3 | ChipKind::Ecp3A => CellCoord::new(
                DieId::from_idx(0),
                match edge {
                    DirH::W => self.col_w() + 1,
                    DirH::E => self.col_e() - 1,
                },
                self.row_clk,
            )
            .bel(bels::DQSDLL),
            _ => unreachable!(),
        }
    }

    pub fn bel_eclk_root(&self, edge: Dir) -> BelCoord {
        assert!(matches!(
            self.kind,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2
        ));
        match edge {
            Dir::W => {
                CellCoord::new(DieId::from_idx(0), self.col_w(), self.row_clk).bel(bels::ECLK_ROOT)
            }
            Dir::E => {
                CellCoord::new(DieId::from_idx(0), self.col_e(), self.row_clk).bel(bels::ECLK_ROOT)
            }
            Dir::S => {
                CellCoord::new(DieId::from_idx(0), self.col_clk, self.row_s()).bel(bels::ECLK_ROOT)
            }
            Dir::N => {
                CellCoord::new(DieId::from_idx(0), self.col_clk, self.row_n()).bel(bels::ECLK_ROOT)
            }
        }
    }

    pub fn bel_eclksync(&self, edge: Dir, idx: usize) -> BelCoord {
        assert!(matches!(self.kind, ChipKind::Ecp3 | ChipKind::Ecp3A));
        match edge {
            Dir::W => CellCoord::new(DieId::from_idx(0), self.col_w(), self.row_clk)
                .bel(bels::ECLKSYNC[idx]),
            Dir::E => CellCoord::new(DieId::from_idx(0), self.col_e(), self.row_clk)
                .bel(bels::ECLKSYNC[idx]),
            Dir::S => unreachable!(),
            Dir::N => CellCoord::new(DieId::from_idx(0), self.col_clk - 1, self.row_n())
                .bel(bels::ECLKSYNC[idx]),
        }
    }

    pub fn bel_serdes(&self, edge: DirV, col: ColId) -> BelCoord {
        match self.kind {
            ChipKind::Ecp2M => {
                let row = match edge {
                    DirV::S => self.row_s() + 7,
                    DirV::N => self.row_n() - 7,
                };
                CellCoord::new(DieId::from_idx(0), col, row).bel(bels::SERDES)
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                assert_eq!(edge, DirV::S);
                CellCoord::new(DieId::from_idx(0), col, self.row_s() + 9).bel(bels::SERDES)
            }
            _ => unreachable!(),
        }
    }
}

impl From<&Column> for JsonValue {
    fn from(value: &Column) -> Self {
        jzon::object! {
            io_s: value.io_s.to_string(),
            io_n: value.io_n.to_string(),
            bank_s: value.bank_s,
            bank_n: value.bank_n,
            eclk_tap_s: value.eclk_tap_s,
            eclk_tap_n: value.eclk_tap_n,
            pclk_leaf_break: value.pclk_leaf_break,
            sdclk_break: value.sdclk_break,
        }
    }
}

impl From<&Row> for JsonValue {
    fn from(value: &Row) -> Self {
        jzon::object! {
            kind: value.kind.to_string(),
            io_w: value.io_w.to_string(),
            io_e: value.io_e.to_string(),
            bank_w: value.bank_w,
            bank_e: value.bank_e,
            sclk_break: value.sclk_break,
            pclk_break: value.pclk_break,
            pclk_drive: value.pclk_drive,
        }
    }
}

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> Self {
        jzon::object! {
            kind: chip.kind.to_string(),
            columns: Vec::from_iter(chip.columns.values()),
            rows: Vec::from_iter(chip.rows.values()),
            col_clk: chip.col_clk.to_idx(),
            row_clk: chip.row_clk.to_idx(),
            special_loc: jzon::object::Object::from_iter(chip.special_loc.iter().map(|(k, v)| (k.to_string(), v.to_string()))),
            special_io: jzon::object::Object::from_iter(chip.special_io.iter().map(|(k, v)| (k.to_string(), v.to_string()))),
            io_direct_plc: jzon::object::Object::from_iter(chip.io_direct_plc.iter().map(|(k, (cell, lut))| (k.to_string(), format!("{cell}_{lut}")))),
        }
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {k}", k = self.kind)?;
        writeln!(f, "\tCOLS:")?;
        for (col, cd) in &self.columns {
            if self.col_clk == col {
                writeln!(f, "\t\t--- clock")?;
            }
            if cd.pclk_leaf_break {
                writeln!(f, "\t\t--- pclk leaf break")?;
            }
            if cd.sdclk_break {
                writeln!(f, "\t\t--- sdclk break")?;
            }
            write!(f, "\t\t{col:>3}:", col = col.to_string())?;
            if cd.io_s == IoGroupKind::None {
                write!(f, "                  ")?;
            } else {
                write!(
                    f,
                    " IO_S:{bank_s}:{io_s:10}",
                    bank_s = cd.bank_s.unwrap(),
                    io_s = cd.io_s.to_string()
                )?;
            }
            if cd.io_n == IoGroupKind::None {
                write!(f, "                  ")?;
            } else {
                write!(
                    f,
                    " IO_N:{bank_n}:{io_n:10}",
                    bank_n = cd.bank_n.unwrap(),
                    io_n = cd.io_n.to_string()
                )?;
            }
            if cd.eclk_tap_s {
                write!(f, " ECLK_TAP_S")?;
            }
            if cd.eclk_tap_n {
                write!(f, " ECLK_TAP_N")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "\tROWS:")?;
        for (row, rd) in &self.rows {
            if rd.pclk_break {
                writeln!(f, "\t\t--- pclk break")?;
            }
            if rd.sclk_break {
                writeln!(f, "\t\t--- sclk break")?;
            }
            if self.row_clk == row {
                writeln!(f, "\t\t--- clock")?;
            }
            write!(
                f,
                "\t\t{row:>3}: {kind:5}",
                row = row.to_string(),
                kind = rd.kind.to_string(),
            )?;
            if rd.io_w == IoGroupKind::None {
                write!(f, "                  ")?;
            } else {
                write!(
                    f,
                    " IO_W:{bank_w}:{io_w:10}",
                    bank_w = rd.bank_w.unwrap(),
                    io_w = rd.io_w.to_string()
                )?;
            }
            if rd.io_e == IoGroupKind::None {
                write!(f, "                  ")?;
            } else {
                write!(
                    f,
                    " IO_E:{bank_e}:{io_e:10}",
                    bank_e = rd.bank_e.unwrap(),
                    io_e = rd.io_e.to_string()
                )?;
            }
            if rd.pclk_drive {
                write!(f, " PCLK_DRIVE")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "\tSPECIAL LOC:")?;
        for (k, v) in &self.special_loc {
            writeln!(f, "\t\t{k}: {v}")?;
        }
        writeln!(f, "\tSPECIAL IO:")?;
        for (k, v) in &self.special_io {
            writeln!(f, "\t\t{k}: {v}")?;
        }
        if self.kind == ChipKind::MachXo {
            writeln!(f, "\tIO DIRECT:")?;
            for (k, (cell, lut)) in &self.io_direct_plc {
                writeln!(f, "\t\t{k}: {cell}_{lut}")?;
            }
        }
        Ok(())
    }
}
