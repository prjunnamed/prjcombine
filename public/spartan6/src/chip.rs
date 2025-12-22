use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_entity::{
    EntityId, EntityRange, EntityVec,
    id::{EntityIdU8, EntityTag, EntityTagArith},
};
use prjcombine_interconnect::{
    dir::{DirH, DirHV},
    grid::{BelCoord, CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId},
};
use std::collections::BTreeMap;

use crate::bels;

pub struct RegTag;
impl EntityTag for RegTag {
    const PREFIX: &'static str = "REG";
}
impl EntityTagArith for RegTag {}
pub type RegId = EntityIdU8<RegTag>;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Chip {
    pub columns: EntityVec<ColId, Column>,
    pub col_clk: ColId,
    pub cols_clk_fold: Option<(ColId, ColId)>,
    pub cols_reg_buf: (ColId, ColId),
    pub rows: EntityVec<RowId, Row>,
    pub rows_midbuf: (RowId, RowId),
    pub rows_hclkbuf: (RowId, RowId),
    pub rows_pci_ce_split: (RowId, RowId),
    pub rows_bank_split: Option<(RowId, RowId)>,
    pub row_mcb_split: Option<RowId>,
    pub gts: Gts,
    pub mcbs: Vec<Mcb>,
    pub cfg_io: BTreeMap<SharedCfgPad, EdgeIoCoord>,
    pub has_encrypt: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum SharedCfgPad {
    // ×16
    // 0 doubles as DIN, MISO, MISO1
    // 1 doubles as MISO2
    // 2 doubles as MISO3
    Data(u8),
    CsoB,
    InitB,
    RdWrB,
    FcsB,
    FoeB,
    FweB,
    Ldc,
    Hdc,
    Addr(u8),
    Dout, // doubles as BUSY
    Mosi, // doubles as CSI_B, MISO0
    M0,   // doubles as CMPMISO
    M1,
    Cclk,
    UserCclk,
    HswapEn,
    CmpClk,
    CmpMosi,
    Awake,
    Scp(u8), // ×8
}

impl std::fmt::Display for SharedCfgPad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedCfgPad::Data(i) => write!(f, "D{i}"),
            SharedCfgPad::Addr(i) => write!(f, "A{i}"),
            SharedCfgPad::Scp(i) => write!(f, "SCP{i}"),
            SharedCfgPad::CsoB => write!(f, "CSO_B"),
            SharedCfgPad::RdWrB => write!(f, "RDWR_B"),
            SharedCfgPad::Dout => write!(f, "DOUT"),
            SharedCfgPad::InitB => write!(f, "INIT_B"),
            SharedCfgPad::Cclk => write!(f, "CCLK"),
            SharedCfgPad::UserCclk => write!(f, "USER_CCLK"),
            SharedCfgPad::Mosi => write!(f, "MOSI"),
            SharedCfgPad::CmpMosi => write!(f, "CMP_MOSI"),
            SharedCfgPad::CmpClk => write!(f, "CMP_CLK"),
            SharedCfgPad::FcsB => write!(f, "FCS_B"),
            SharedCfgPad::FoeB => write!(f, "FOE_B"),
            SharedCfgPad::FweB => write!(f, "FWE_B"),
            SharedCfgPad::Ldc => write!(f, "LDC"),
            SharedCfgPad::M0 => write!(f, "M0"),
            SharedCfgPad::M1 => write!(f, "M1"),
            SharedCfgPad::Hdc => write!(f, "HDC"),
            SharedCfgPad::HswapEn => write!(f, "HSWAP_EN"),
            SharedCfgPad::Awake => write!(f, "AWAKE"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Column {
    pub kind: ColumnKind,
    pub bio: ColumnIoKind,
    pub tio: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ColumnKind {
    Io,
    CleXL,
    CleXM,
    CleClk,
    Bram,
    Dsp,
    DspPlus,
}

impl std::fmt::Display for ColumnKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnKind::Io => write!(f, "IO"),
            ColumnKind::CleXL => write!(f, "CLEXL"),
            ColumnKind::CleXM => write!(f, "CLEXM"),
            ColumnKind::CleClk => write!(f, "CLEXL_CLK"),
            ColumnKind::Bram => write!(f, "BRAM"),
            ColumnKind::Dsp => write!(f, "DSP"),
            ColumnKind::DspPlus => write!(f, "DSP_PLUS"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ColumnIoKind {
    None,
    Both,
    Inner,
    Outer,
}

impl std::fmt::Display for ColumnIoKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnIoKind::None => write!(f, "NONE"),
            ColumnIoKind::Both => write!(f, "BOTH"),
            ColumnIoKind::Outer => write!(f, "OUTER"),
            ColumnIoKind::Inner => write!(f, "INNER"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Row {
    pub lio: bool,
    pub rio: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum Gts {
    None,
    Single(ColId),
    Double(ColId, ColId),
    Quad(ColId, ColId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct McbIo {
    pub row: RowId,
    pub iob: TileIobId,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Mcb {
    pub row_mcb: RowId,
    pub row_mui: [RowId; 8],
    pub iop_dq: [RowId; 8],
    pub iop_dqs: [RowId; 2],
    pub io_dm: [McbIo; 2],
    pub iop_clk: RowId,
    pub io_addr: [McbIo; 15],
    pub io_ba: [McbIo; 3],
    pub io_ras: McbIo,
    pub io_cas: McbIo,
    pub io_we: McbIo,
    pub io_odt: McbIo,
    pub io_cke: McbIo,
    pub io_reset: McbIo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub enum DisabledPart {
    Gtp,
    Mcb,
    ClbColumn(ColId),
    BramRegion(ColId, RegId),
    DspRegion(ColId, RegId),
}

impl std::fmt::Display for DisabledPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisabledPart::ClbColumn(col) => write!(f, "CLB_COL:{col}"),
            DisabledPart::BramRegion(col, reg) => write!(f, "BRAM_REG:{col}:{reg}"),
            DisabledPart::DspRegion(col, reg) => write!(f, "DSP_REG:{col}:{reg}"),
            DisabledPart::Mcb => write!(f, "MCB"),
            DisabledPart::Gtp => write!(f, "GTP"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum DcmKind {
    Bot,
    BotMid,
    Top,
    TopMid,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub enum PllKind {
    BotOut0,
    BotOut1,
    BotNoOut,
    TopOut0,
    TopOut1,
    TopNoOut,
}

impl Chip {
    pub fn col_w(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_e(&self) -> ColId {
        ColId::from_idx(self.columns.len() - 1)
    }

    pub fn col_edge(&self, edge: DirH) -> ColId {
        match edge {
            DirH::W => self.col_w(),
            DirH::E => self.col_e(),
        }
    }

    pub fn row_bot(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_top(&self) -> RowId {
        self.rows.next_id()
    }

    pub fn row_bio_outer(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_bio_inner(&self) -> RowId {
        RowId::from_idx(1)
    }

    pub fn row_tio_outer(&self) -> RowId {
        RowId::from_idx(self.rows.len() - 1)
    }

    pub fn row_tio_inner(&self) -> RowId {
        RowId::from_idx(self.rows.len() - 2)
    }

    pub fn get_mcb(&self, row: RowId) -> &Mcb {
        for mcb in &self.mcbs {
            if mcb.row_mcb == row {
                return mcb;
            }
        }
        unreachable!()
    }

    pub fn row_clk(&self) -> RowId {
        RowId::from_idx(self.rows.len() / 2)
    }

    #[inline]
    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / 16)
    }

    #[inline]
    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 16)
    }

    #[inline]
    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        self.row_reg_bot(reg) + 8
    }

    pub fn regs(&self) -> EntityRange<RegId> {
        EntityRange::new(0, self.rows.len() / 16)
    }

    #[inline]
    pub fn row_hclk(&self, row: RowId) -> RowId {
        self.row_reg_hclk(self.row_to_reg(row))
    }

    pub fn is_25(&self) -> bool {
        self.rows.len() % 32 == 16
    }

    pub fn get_dcms(&self) -> Vec<(RowId, DcmKind)> {
        match self.rows.len() {
            64 | 80 => vec![
                (self.row_bot() + 8, DcmKind::Bot),
                (self.row_top() - 24, DcmKind::Top),
            ],
            128 => vec![
                (self.row_bot() + 8, DcmKind::Bot),
                (self.row_bot() + 40, DcmKind::BotMid),
                (self.row_top() - 56, DcmKind::Top),
                (self.row_top() - 24, DcmKind::TopMid),
            ],
            192 => vec![
                (self.row_bot() + 8, DcmKind::Bot),
                (self.row_bot() + 40, DcmKind::BotMid),
                (self.row_bot() + 72, DcmKind::BotMid),
                (self.row_top() - 88, DcmKind::Top),
                (self.row_top() - 56, DcmKind::TopMid),
                (self.row_top() - 24, DcmKind::TopMid),
            ],
            _ => unreachable!(),
        }
    }

    pub fn get_plls(&self) -> Vec<(RowId, PllKind)> {
        match self.rows.len() {
            64 | 80 => vec![
                (self.row_bot() + 24, PllKind::BotOut1),
                (self.row_top() - 8, PllKind::TopOut1),
            ],
            128 => vec![
                (self.row_bot() + 24, PllKind::BotOut1),
                (self.row_bot() + 56, PllKind::BotOut0),
                (self.row_top() - 40, PllKind::TopOut0),
                (self.row_top() - 8, PllKind::TopOut1),
            ],
            192 => vec![
                (self.row_bot() + 24, PllKind::BotOut1),
                (self.row_bot() + 56, PllKind::BotNoOut),
                (self.row_bot() + 88, PllKind::BotOut0),
                (self.row_top() - 72, PllKind::TopOut0),
                (self.row_top() - 40, PllKind::TopNoOut),
                (self.row_top() - 8, PllKind::TopOut1),
            ],
            _ => unreachable!(),
        }
    }

    pub fn get_io_crd(&self, bel: BelCoord) -> EdgeIoCoord {
        let iob = bels::IOB.iter().position(|&x| x == bel.slot).unwrap();
        if bel.col == self.col_w() {
            EdgeIoCoord::W(bel.row, TileIobId::from_idx(iob))
        } else if bel.col == self.col_e() {
            EdgeIoCoord::E(bel.row, TileIobId::from_idx(iob))
        } else if bel.row == self.row_bio_inner() {
            EdgeIoCoord::S(bel.col, TileIobId::from_idx(iob))
        } else if bel.row == self.row_bio_outer() {
            EdgeIoCoord::S(bel.col, TileIobId::from_idx(iob + 2))
        } else if bel.row == self.row_tio_inner() {
            EdgeIoCoord::N(bel.col, TileIobId::from_idx(iob))
        } else if bel.row == self.row_tio_outer() {
            EdgeIoCoord::N(bel.col, TileIobId::from_idx(iob + 2))
        } else {
            unreachable!()
        }
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> BelCoord {
        let die = DieId::from_idx(0);
        match io {
            EdgeIoCoord::N(col, iob) => {
                if iob.to_idx() < 2 {
                    CellCoord::new(die, col, self.row_tio_inner()).bel(bels::IOB[iob.to_idx()])
                } else {
                    CellCoord::new(die, col, self.row_tio_outer()).bel(bels::IOB[iob.to_idx() - 2])
                }
            }
            EdgeIoCoord::E(row, iob) => {
                CellCoord::new(die, self.col_e(), row).bel(bels::IOB[iob.to_idx()])
            }
            EdgeIoCoord::S(col, iob) => {
                if iob.to_idx() < 2 {
                    CellCoord::new(die, col, self.row_bio_inner()).bel(bels::IOB[iob.to_idx()])
                } else {
                    CellCoord::new(die, col, self.row_bio_outer()).bel(bels::IOB[iob.to_idx() - 2])
                }
            }
            EdgeIoCoord::W(row, iob) => {
                CellCoord::new(die, self.col_w(), row).bel(bels::IOB[iob.to_idx()])
            }
        }
    }

    pub fn get_io_bank(&self, io: EdgeIoCoord) -> u32 {
        match io {
            EdgeIoCoord::N(_, _) => 0,
            EdgeIoCoord::E(row, _) => {
                if let Some((_, rs)) = self.rows_bank_split {
                    if row < rs { 1 } else { 5 }
                } else {
                    1
                }
            }
            EdgeIoCoord::S(_, _) => 2,
            EdgeIoCoord::W(row, _) => {
                if let Some((rs, _)) = self.rows_bank_split {
                    if row < rs { 3 } else { 4 }
                } else {
                    3
                }
            }
        }
    }

    pub fn get_bonded_ios(&self) -> Vec<EdgeIoCoord> {
        let mut res = vec![];
        // TIO
        for (col, &cd) in &self.columns {
            if cd.tio == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // outer
                (3, cd.tio == ColumnIoKind::Inner),
                (2, cd.tio == ColumnIoKind::Inner),
                // inner
                (1, cd.tio == ColumnIoKind::Outer),
                (0, cd.tio == ColumnIoKind::Outer),
            ] {
                if !unused {
                    res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                }
            }
        }
        // RIO
        for (row, &rd) in self.rows.iter().rev() {
            if rd.rio {
                for iob in [1, 0] {
                    res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                }
            }
        }
        // BIO
        for (col, &cd) in self.columns.iter().rev() {
            if cd.bio == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // outer
                (3, cd.bio == ColumnIoKind::Inner),
                (2, cd.bio == ColumnIoKind::Inner),
                // inner
                (1, cd.bio == ColumnIoKind::Outer),
                (0, cd.bio == ColumnIoKind::Outer),
            ] {
                if !unused {
                    res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                }
            }
        }
        // LIO
        for (row, &rd) in &self.rows {
            if rd.lio {
                for iob in [1, 0] {
                    res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                }
            }
        }
        res
    }

    pub fn bel_pcilogicse(&self, edge: DirH) -> BelCoord {
        CellCoord::new(DieId::from_idx(0), self.col_edge(edge), self.row_clk())
            .bel(bels::PCILOGICSE)
    }

    pub fn bel_gtp(&self, side: DirHV) -> Option<BelCoord> {
        match (self.gts, side) {
            (Gts::Single(col) | Gts::Double(col, _) | Gts::Quad(col, _), DirHV::NW) => {
                Some(CellCoord::new(DieId::from_idx(0), col, self.row_tio_outer()).bel(bels::GTP))
            }
            (Gts::Double(_, col) | Gts::Quad(_, col), DirHV::NE) => {
                Some(CellCoord::new(DieId::from_idx(0), col, self.row_tio_outer()).bel(bels::GTP))
            }
            (Gts::Quad(col, _), DirHV::SW) => {
                Some(CellCoord::new(DieId::from_idx(0), col, self.row_bio_outer()).bel(bels::GTP))
            }
            (Gts::Quad(_, col), DirHV::SE) => {
                Some(CellCoord::new(DieId::from_idx(0), col, self.row_bio_outer()).bel(bels::GTP))
            }
            _ => None,
        }
    }

    pub fn bel_pcie(&self) -> Option<BelCoord> {
        match self.gts {
            Gts::Single(col) | Gts::Double(col, _) | Gts::Quad(col, _) => Some(
                CellCoord::new(DieId::from_idx(0), col - 2, self.row_top() - 32).bel(bels::PCIE),
            ),
            Gts::None => None,
        }
    }
}

impl From<&McbIo> for JsonValue {
    fn from(io: &McbIo) -> Self {
        jzon::object! {
            row: io.row.to_idx(),
            iob: io.iob.to_idx(),
        }
    }
}

impl From<&Mcb> for JsonValue {
    fn from(mcb: &Mcb) -> Self {
        jzon::object! {
            row_mcb: mcb.row_mcb.to_idx(),
            row_mui: Vec::from_iter(mcb.row_mui.iter().map(|row| row.to_idx())),
            iop_dq: Vec::from_iter(mcb.iop_dq.iter().map(|row| row.to_idx())),
            iop_dqs: Vec::from_iter(mcb.iop_dqs.iter().map(|row| row.to_idx())),
            io_dm: Vec::from_iter(mcb.io_dm.iter()),
            iop_clk: mcb.iop_clk.to_idx(),
            io_addr: Vec::from_iter(mcb.io_addr.iter()),
            io_ba: Vec::from_iter(mcb.io_ba.iter()),
            io_ras: &mcb.io_ras,
            io_cas: &mcb.io_cas,
            io_we: &mcb.io_we,
            io_odt: &mcb.io_odt,
            io_cke: &mcb.io_cke,
            io_reset: &mcb.io_reset,
        }
    }
}

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> Self {
        jzon::object! {
            columns: Vec::from_iter(chip.columns.values().map(|column| {
                jzon::object! {
                    kind: column.kind.to_string(),
                    bio: column.bio.to_string(),
                    tio: column.tio.to_string(),
                }
            })),
            col_clk: chip.col_clk.to_idx(),
            cols_clk_fold: chip.cols_clk_fold.map(|(col_l, col_r)| jzon::array![col_l.to_idx(), col_r.to_idx()]),
            cols_reg_buf: jzon::array![chip.cols_reg_buf.0.to_idx(), chip.cols_reg_buf.1.to_idx()],
            rows: Vec::from_iter(chip.rows.values().map(|row| {
                jzon::object! {
                    lio: row.lio,
                    rio: row.rio,
                }
            })),
            rows_midbuf: jzon::array![chip.rows_midbuf.0.to_idx(), chip.rows_midbuf.1.to_idx()],
            rows_hclkbuf: jzon::array![chip.rows_hclkbuf.0.to_idx(), chip.rows_hclkbuf.1.to_idx()],
            rows_pci_ce_split: jzon::array![chip.rows_pci_ce_split.0.to_idx(), chip.rows_pci_ce_split.1.to_idx()],
            rows_bank_split: chip.rows_bank_split.map(|(row_l, row_r)| jzon::array![row_l.to_idx(), row_r.to_idx()]),
            row_mcb_split: chip.row_mcb_split.map(|row| row.to_idx()),
            gts: match chip.gts {
                Gts::None => JsonValue::Null,
                Gts::Single(col_l) => jzon::object! {
                    num: 1,
                    col_l: col_l.to_idx(),
                },
                Gts::Double(col_l, col_r) => jzon::object! {
                    num: 2,
                    col_l: col_l.to_idx(),
                    col_r: col_r.to_idx(),
                },
                Gts::Quad(col_l, col_r) => jzon::object! {
                    num: 4,
                    col_l: col_l.to_idx(),
                    col_r: col_r.to_idx(),
                },
            },
            mcbs: Vec::from_iter(chip.mcbs.iter()),
            cfg_io: jzon::object::Object::from_iter(chip.cfg_io.iter().map(|(k, io)| {
                (k.to_string(), io.to_string())
            })),
            has_encrypt: chip.has_encrypt,
        }
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: Spartan6")?;
        writeln!(f, "\tCOLS:")?;
        for (col, cd) in &self.columns {
            write!(f, "\t\t{col}: ")?;
            match cd.kind {
                ColumnKind::Io => write!(f, "IO")?,
                ColumnKind::CleXL => write!(f, "CLEXL")?,
                ColumnKind::CleXM => write!(f, "CLEXM")?,
                ColumnKind::CleClk => write!(f, "CLEXL+CLK")?,
                ColumnKind::Bram => write!(f, "BRAM")?,
                ColumnKind::Dsp => write!(f, "DSP")?,
                ColumnKind::DspPlus => write!(f, "DSP*")?,
            }
            match cd.bio {
                ColumnIoKind::None => (),
                ColumnIoKind::Inner => write!(f, " BIO: I-")?,
                ColumnIoKind::Outer => write!(f, " BIO: -O")?,
                ColumnIoKind::Both => write!(f, " BIO: IO")?,
            }
            match cd.tio {
                ColumnIoKind::None => (),
                ColumnIoKind::Inner => write!(f, " TIO: I-")?,
                ColumnIoKind::Outer => write!(f, " TIO: -O")?,
                ColumnIoKind::Both => write!(f, " TIO: IO")?,
            }
            if let Some((cl, cr)) = self.cols_clk_fold
                && (col == cl || col == cr)
            {
                write!(f, " FOLD")?;
            }
            if col == self.cols_reg_buf.0 || col == self.cols_reg_buf.1 {
                write!(f, " REGBUF")?;
            }
            if let Gts::Single(cl) | Gts::Double(cl, _) | Gts::Quad(cl, _) = self.gts
                && col == cl
            {
                write!(f, " LGT")?;
            }
            if let Gts::Double(_, cr) | Gts::Quad(_, cr) = self.gts
                && col == cr
            {
                write!(f, " RGT")?;
            }
            writeln!(f,)?;
        }
        writeln!(f, "\tROWS:")?;
        for (row, rd) in &self.rows {
            if row.to_idx() != 0 && row.to_idx().is_multiple_of(16) {
                writeln!(f, "\t\t--- clock break")?;
            }
            if row.to_idx() % 16 == 8 {
                writeln!(f, "\t\t--- clock row")?;
            }
            if row == self.row_clk() {
                writeln!(f, "\t\t--- spine row")?;
            }
            if let Some((rl, rr)) = self.rows_bank_split {
                if row == rl {
                    writeln!(f, "\t\t--- left bank split")?;
                }
                if row == rr {
                    writeln!(f, "\t\t--- right bank split")?;
                }
            }
            if Some(row) == self.row_mcb_split {
                writeln!(f, "\t\t--- MCB split")?;
            }
            write!(f, "\t\t{row}: ")?;
            if rd.lio {
                write!(f, " LIO")?;
            }
            if rd.rio {
                write!(f, " RIO")?;
            }
            if row == self.rows_midbuf.0 || row == self.rows_midbuf.1 {
                write!(f, " MIDBUF")?;
            }
            if row == self.rows_hclkbuf.0 || row == self.rows_hclkbuf.1 {
                write!(f, " HCLKBUF")?;
            }
            for (i, mcb) in self.mcbs.iter().enumerate() {
                if row == mcb.row_mcb {
                    write!(f, " MCB{i}.MCB")?;
                }
                for (j, &r) in mcb.row_mui.iter().enumerate() {
                    if row == r {
                        write!(f, " MCB{i}.MUI{j}")?;
                    }
                }
                for (j, &r) in mcb.iop_dq.iter().enumerate() {
                    if row == r {
                        write!(f, " MCB{i}.DQ({jj0},{jj1})", jj0 = j * 2, jj1 = j * 2 + 1)?;
                    }
                }
                for (j, &r) in mcb.iop_dqs.iter().enumerate() {
                    if row == r {
                        write!(f, " MCB{i}.DQS{j}")?;
                    }
                }
                if row == mcb.iop_clk {
                    write!(f, " MCB{i}.CLK")?;
                }
                let mut pins: [Option<&'static str>; 2] = [None, None];
                for (pin, io) in [
                    ("DM0", mcb.io_dm[0]),
                    ("DM1", mcb.io_dm[1]),
                    ("A0", mcb.io_addr[0]),
                    ("A1", mcb.io_addr[1]),
                    ("A2", mcb.io_addr[2]),
                    ("A3", mcb.io_addr[3]),
                    ("A4", mcb.io_addr[4]),
                    ("A5", mcb.io_addr[5]),
                    ("A6", mcb.io_addr[6]),
                    ("A7", mcb.io_addr[7]),
                    ("A8", mcb.io_addr[8]),
                    ("A9", mcb.io_addr[9]),
                    ("A10", mcb.io_addr[10]),
                    ("A11", mcb.io_addr[11]),
                    ("A12", mcb.io_addr[12]),
                    ("A13", mcb.io_addr[13]),
                    ("A14", mcb.io_addr[14]),
                    ("BA0", mcb.io_ba[0]),
                    ("BA1", mcb.io_ba[1]),
                    ("BA2", mcb.io_ba[2]),
                    ("RAS", mcb.io_ras),
                    ("CAS", mcb.io_cas),
                    ("WE", mcb.io_we),
                    ("ODT", mcb.io_odt),
                    ("CKE", mcb.io_cke),
                    ("RST", mcb.io_reset),
                ] {
                    if row == io.row {
                        pins[io.iob.to_idx()] = Some(pin);
                    }
                }
                if pins.iter().any(|x| x.is_some()) {
                    write!(
                        f,
                        " MCB{i}.({p0},{p1})",
                        p0 = pins[0].unwrap(),
                        p1 = pins[1].unwrap()
                    )?;
                }
            }
            writeln!(f)?;
        }
        match self.gts {
            Gts::None => (),
            Gts::Single(..) => writeln!(f, "\tGTS: SINGLE")?,
            Gts::Double(..) => writeln!(f, "\tGTS: DOUBLE")?,
            Gts::Quad(..) => writeln!(f, "\tGTS: QUAD")?,
        }
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(f, "\t\t{k:?}: {v}")?;
        }
        if self.has_encrypt {
            writeln!(f, "\tHAS ENCRYPT")?;
        }
        Ok(())
    }
}
