use std::{collections::BTreeMap, fmt::Display};

use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_interconnect::{
    dir::{Dir, DirHV, DirV},
    grid::{BelCoord, CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::bels;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum ChipKind {
    Ecp,
    Xp,
    MachXo,
}

impl Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Ecp => write!(f, "ecp"),
            ChipKind::Xp => write!(f, "xp"),
            ChipKind::MachXo => write!(f, "machxo"),
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
pub enum IoKind {
    None,
    Double,
    DoubleA,
    DoubleB,
    DoubleDqs,
    Quad,
    QuadReverse,
    Hex,
    HexReverse,
}

impl Display for IoKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoKind::None => write!(f, "NONE"),
            IoKind::Double => write!(f, "DOUBLE"),
            IoKind::DoubleA => write!(f, "DOUBLE_A"),
            IoKind::DoubleB => write!(f, "DOUBLE_B"),
            IoKind::DoubleDqs => write!(f, "DOUBLE_DQS"),
            IoKind::Quad => write!(f, "QUAD"),
            IoKind::QuadReverse => write!(f, "QUAD_REVERSE"),
            IoKind::Hex => write!(f, "HEX"),
            IoKind::HexReverse => write!(f, "HEX_REVERSE"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Column {
    pub io_s: IoKind,
    pub io_n: IoKind,
    pub bank_s: Option<u32>,
    pub bank_n: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Row {
    pub kind: RowKind,
    pub io_w: IoKind,
    pub io_e: IoKind,
    pub bank_w: Option<u32>,
    pub bank_e: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SpecialIoKey {
    Clock(Dir, u8),
    PllIn(DirHV),
    PllFb(DirHV),
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
    SleepN,
}

impl Display for SpecialIoKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialIoKey::Clock(dir, i) => write!(f, "CLOCK_{dir}{i}"),
            SpecialIoKey::PllIn(dir) => write!(f, "PLL_{dir}_IN"),
            SpecialIoKey::PllFb(dir) => write!(f, "PLL_{dir}_FB"),
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
            SpecialIoKey::SleepN => write!(f, "SLEEP_N"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SpecialLocKey {
    Pll(DirHV),
    Ebr(u8),
    PclkIn(Dir, u8),
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

    pub fn row_s(&self) -> RowId {
        self.rows.first_id().unwrap()
    }

    pub fn row_n(&self) -> RowId {
        self.rows.last_id().unwrap()
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> BelCoord {
        let (col, row, iob) = match io {
            EdgeIoCoord::N(col, iob) => (col, self.row_n(), iob),
            EdgeIoCoord::E(row, iob) => (self.col_e(), row, iob),
            EdgeIoCoord::S(col, iob) => (col, self.row_s(), iob),
            EdgeIoCoord::W(row, iob) => (self.col_w(), row, iob),
        };
        let slot = bels::IO[iob.to_idx()];
        CellCoord::new(DieId::from_idx(0), col, row).bel(slot)
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

    pub fn bel_cibtest_sel(&self) -> BelCoord {
        assert_eq!(self.kind, ChipKind::MachXo);
        assert!(self.special_loc.contains_key(&SpecialLocKey::Ebr(0)));
        CellCoord::new(DieId::from_idx(0), self.col_w(), self.row_n() - 1).bel(bels::CIBTEST_SEL)
    }

    pub fn bel_clk_root(&self) -> BelCoord {
        CellCoord::new(DieId::from_idx(0), self.col_clk, self.row_clk).bel(bels::CLK_ROOT)
    }

    pub fn bel_dqsdll(&self, edge: DirV) -> BelCoord {
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
}

impl From<&Column> for JsonValue {
    fn from(value: &Column) -> Self {
        jzon::object! {
            io_s: value.io_s.to_string(),
            io_n: value.io_n.to_string(),
        }
    }
}

impl From<&Row> for JsonValue {
    fn from(value: &Row) -> Self {
        jzon::object! {
            kind: value.kind.to_string(),
            io_w: value.io_w.to_string(),
            io_e: value.io_e.to_string(),
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
            write!(f, "\t\t{col:>3}:", col = col.to_string())?;
            if cd.io_s == IoKind::None {
                write!(f, "                  ")?;
            } else {
                write!(
                    f,
                    " IO_S:{bank_s}:{io_s:10}",
                    bank_s = cd.bank_s.unwrap(),
                    io_s = cd.io_s.to_string()
                )?;
            }
            if cd.io_n == IoKind::None {
                write!(f, "                  ")?;
            } else {
                write!(
                    f,
                    " IO_N:{bank_n}:{io_n:10}",
                    bank_n = cd.bank_n.unwrap(),
                    io_n = cd.io_n.to_string()
                )?;
            }
            writeln!(f)?;
        }
        writeln!(f, "\tROWS:")?;
        for (row, rd) in &self.rows {
            if self.row_clk == row {
                writeln!(f, "\t\t--- clock")?;
            }
            write!(
                f,
                "\t\t{row:>3}: {kind:5}",
                row = row.to_string(),
                kind = rd.kind.to_string(),
            )?;
            if rd.io_w == IoKind::None {
                write!(f, "                  ")?;
            } else {
                write!(
                    f,
                    " IO_W:{bank_w}:{io_w:10}",
                    bank_w = rd.bank_w.unwrap(),
                    io_w = rd.io_w.to_string()
                )?;
            }
            if rd.io_e == IoKind::None {
                write!(f, "                  ")?;
            } else {
                write!(
                    f,
                    " IO_E:{bank_e}:{io_e:10}",
                    bank_e = rd.bank_e.unwrap(),
                    io_e = rd.io_e.to_string()
                )?;
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
