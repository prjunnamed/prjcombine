use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{
    EntityId, EntityRange, EntityVec,
    id::{EntityIdU8, EntityTag, EntityTagArith},
};
use prjcombine_interconnect::{
    dir::{DirH, DirHV},
    grid::{BelCoord, CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId},
};
use std::collections::BTreeMap;

use crate::defs;

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
    pub io_s: ColumnIoKind,
    pub io_n: ColumnIoKind,
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
    pub io_w: bool,
    pub io_e: bool,
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

impl std::fmt::Display for McbIo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{row}.{iob}", row = self.row, iob = self.iob)
    }
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

    pub fn row_s(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_s_inner(&self) -> RowId {
        RowId::from_idx(1)
    }

    pub fn row_n(&self) -> RowId {
        RowId::from_idx(self.rows.len() - 1)
    }

    pub fn row_n_inner(&self) -> RowId {
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
    pub fn row_reg_s(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 16)
    }

    #[inline]
    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        self.row_reg_s(reg) + 8
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
                (self.row_s() + 8, DcmKind::Bot),
                (self.row_n() - 23, DcmKind::Top),
            ],
            128 => vec![
                (self.row_s() + 8, DcmKind::Bot),
                (self.row_s() + 40, DcmKind::BotMid),
                (self.row_n() - 55, DcmKind::Top),
                (self.row_n() - 23, DcmKind::TopMid),
            ],
            192 => vec![
                (self.row_s() + 8, DcmKind::Bot),
                (self.row_s() + 40, DcmKind::BotMid),
                (self.row_s() + 72, DcmKind::BotMid),
                (self.row_n() - 87, DcmKind::Top),
                (self.row_n() - 55, DcmKind::TopMid),
                (self.row_n() - 23, DcmKind::TopMid),
            ],
            _ => unreachable!(),
        }
    }

    pub fn get_plls(&self) -> Vec<(RowId, PllKind)> {
        match self.rows.len() {
            64 | 80 => vec![
                (self.row_s() + 24, PllKind::BotOut1),
                (self.row_n() - 7, PllKind::TopOut1),
            ],
            128 => vec![
                (self.row_s() + 24, PllKind::BotOut1),
                (self.row_s() + 56, PllKind::BotOut0),
                (self.row_n() - 39, PllKind::TopOut0),
                (self.row_n() - 7, PllKind::TopOut1),
            ],
            192 => vec![
                (self.row_s() + 24, PllKind::BotOut1),
                (self.row_s() + 56, PllKind::BotNoOut),
                (self.row_s() + 88, PllKind::BotOut0),
                (self.row_n() - 71, PllKind::TopOut0),
                (self.row_n() - 39, PllKind::TopNoOut),
                (self.row_n() - 7, PllKind::TopOut1),
            ],
            _ => unreachable!(),
        }
    }

    pub fn get_io_crd(&self, bel: BelCoord) -> EdgeIoCoord {
        let iob = defs::bslots::IOB.index_of(bel.slot).unwrap();
        if bel.col == self.col_w() {
            EdgeIoCoord::W(bel.row, TileIobId::from_idx(iob))
        } else if bel.col == self.col_e() {
            EdgeIoCoord::E(bel.row, TileIobId::from_idx(iob))
        } else if bel.row == self.row_s_inner() {
            EdgeIoCoord::S(bel.col, TileIobId::from_idx(iob))
        } else if bel.row == self.row_s() {
            EdgeIoCoord::S(bel.col, TileIobId::from_idx(iob + 2))
        } else if bel.row == self.row_n_inner() {
            EdgeIoCoord::N(bel.col, TileIobId::from_idx(iob))
        } else if bel.row == self.row_n() {
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
                    CellCoord::new(die, col, self.row_n_inner())
                        .bel(defs::bslots::IOB[iob.to_idx()])
                } else {
                    CellCoord::new(die, col, self.row_n()).bel(defs::bslots::IOB[iob.to_idx() - 2])
                }
            }
            EdgeIoCoord::E(row, iob) => {
                CellCoord::new(die, self.col_e(), row).bel(defs::bslots::IOB[iob.to_idx()])
            }
            EdgeIoCoord::S(col, iob) => {
                if iob.to_idx() < 2 {
                    CellCoord::new(die, col, self.row_s_inner())
                        .bel(defs::bslots::IOB[iob.to_idx()])
                } else {
                    CellCoord::new(die, col, self.row_s()).bel(defs::bslots::IOB[iob.to_idx() - 2])
                }
            }
            EdgeIoCoord::W(row, iob) => {
                CellCoord::new(die, self.col_w(), row).bel(defs::bslots::IOB[iob.to_idx()])
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
            if cd.io_n == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // outer
                (3, cd.io_n == ColumnIoKind::Inner),
                (2, cd.io_n == ColumnIoKind::Inner),
                // inner
                (1, cd.io_n == ColumnIoKind::Outer),
                (0, cd.io_n == ColumnIoKind::Outer),
            ] {
                if !unused {
                    res.push(EdgeIoCoord::N(col, TileIobId::from_idx(iob)));
                }
            }
        }
        // RIO
        for (row, &rd) in self.rows.iter().rev() {
            if rd.io_e {
                for iob in [1, 0] {
                    res.push(EdgeIoCoord::E(row, TileIobId::from_idx(iob)));
                }
            }
        }
        // BIO
        for (col, &cd) in self.columns.iter().rev() {
            if cd.io_s == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // outer
                (3, cd.io_s == ColumnIoKind::Inner),
                (2, cd.io_s == ColumnIoKind::Inner),
                // inner
                (1, cd.io_s == ColumnIoKind::Outer),
                (0, cd.io_s == ColumnIoKind::Outer),
            ] {
                if !unused {
                    res.push(EdgeIoCoord::S(col, TileIobId::from_idx(iob)));
                }
            }
        }
        // LIO
        for (row, &rd) in &self.rows {
            if rd.io_w {
                for iob in [1, 0] {
                    res.push(EdgeIoCoord::W(row, TileIobId::from_idx(iob)));
                }
            }
        }
        res
    }

    pub fn bel_pcilogicse(&self, edge: DirH) -> BelCoord {
        CellCoord::new(DieId::from_idx(0), self.col_edge(edge), self.row_clk())
            .bel(defs::bslots::PCILOGICSE)
    }

    pub fn bel_gtp(&self, side: DirHV) -> Option<BelCoord> {
        match (self.gts, side) {
            (Gts::Single(col) | Gts::Double(col, _) | Gts::Quad(col, _), DirHV::NW) => {
                Some(CellCoord::new(DieId::from_idx(0), col, self.row_n()).bel(defs::bslots::GTP))
            }
            (Gts::Double(_, col) | Gts::Quad(_, col), DirHV::NE) => {
                Some(CellCoord::new(DieId::from_idx(0), col, self.row_n()).bel(defs::bslots::GTP))
            }
            (Gts::Quad(col, _), DirHV::SW) => {
                Some(CellCoord::new(DieId::from_idx(0), col, self.row_s()).bel(defs::bslots::GTP))
            }
            (Gts::Quad(_, col), DirHV::SE) => {
                Some(CellCoord::new(DieId::from_idx(0), col, self.row_s()).bel(defs::bslots::GTP))
            }
            _ => None,
        }
    }

    pub fn bel_pcie(&self) -> Option<BelCoord> {
        match self.gts {
            Gts::Single(col) | Gts::Double(col, _) | Gts::Quad(col, _) => Some(
                CellCoord::new(DieId::from_idx(0), col - 2, self.row_n() - 31)
                    .bel(defs::bslots::PCIE),
            ),
            Gts::None => None,
        }
    }
}

impl Chip {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tcolumns {{")?;
        for (col, cd) in &self.columns {
            write!(o, "\t\t")?;
            match cd.kind {
                ColumnKind::Io => write!(o, "io")?,
                ColumnKind::CleXL => write!(o, "clexl")?,
                ColumnKind::CleXM => write!(o, "clexm")?,
                ColumnKind::CleClk => write!(o, "clexl_clk")?,
                ColumnKind::Bram => write!(o, "bram")?,
                ColumnKind::Dsp => write!(o, "dsp")?,
                ColumnKind::DspPlus => write!(o, "dsp_gt")?,
            }
            match cd.io_s {
                ColumnIoKind::None => (),
                ColumnIoKind::Inner => write!(o, " + io_s_inner")?,
                ColumnIoKind::Outer => write!(o, " + io_s_outer")?,
                ColumnIoKind::Both => write!(o, " + io_s")?,
            }
            match cd.io_n {
                ColumnIoKind::None => (),
                ColumnIoKind::Inner => write!(o, " + io_n_inner")?,
                ColumnIoKind::Outer => write!(o, " + io_n_outer")?,
                ColumnIoKind::Both => write!(o, " + io_n")?,
            }
            write!(o, ", // {col}")?;
            if let Some((cl, cr)) = self.cols_clk_fold
                && (col == cl || col == cr)
            {
                write!(o, " FOLD")?;
            }
            if col == self.cols_reg_buf.0 || col == self.cols_reg_buf.1 {
                write!(o, " REGBUF")?;
            }
            if let Gts::Single(cl) | Gts::Double(cl, _) | Gts::Quad(cl, _) = self.gts
                && col == cl
            {
                write!(o, " LGT")?;
            }
            if let Gts::Double(_, cr) | Gts::Quad(_, cr) = self.gts
                && col == cr
            {
                write!(o, " RGT")?;
            }
            writeln!(o)?;
        }
        writeln!(o, "\t}}")?;
        if let Some((cl, cr)) = self.cols_clk_fold {
            writeln!(o, "\tcols_clk_fold {cl}, {cr};")?;
        }
        writeln!(
            o,
            "\tcols_reg_buf {cl}, {cr};",
            cl = self.cols_reg_buf.0,
            cr = self.cols_reg_buf.1
        )?;
        writeln!(o, "\trows {{")?;
        for (row, rd) in &self.rows {
            if row.to_idx() != 0 && row.to_idx().is_multiple_of(16) {
                writeln!(o, "\t\t// clock break")?;
            }
            if row.to_idx() % 16 == 8 {
                writeln!(o, "\t\t// clock row")?;
            }
            if row == self.row_clk() {
                writeln!(o, "\t\t// spine row")?;
            }
            if let Some((rl, rr)) = self.rows_bank_split {
                if row == rl {
                    writeln!(o, "\t\t// left bank split")?;
                }
                if row == rr {
                    writeln!(o, "\t\t// right bank split")?;
                }
            }
            if Some(row) == self.row_mcb_split {
                writeln!(o, "\t\t// MCB split")?;
            }
            write!(o, "\t\t")?;
            match (rd.io_w, rd.io_e) {
                (true, true) => write!(o, "io_w + io_e")?,
                (true, false) => write!(o, "io_w")?,
                (false, true) => write!(o, "io_e")?,
                (false, false) => write!(o, "null")?,
            }
            write!(o, ", // {row}")?;
            if row == self.rows_midbuf.0 || row == self.rows_midbuf.1 {
                write!(o, " MIDBUF")?;
            }
            if row == self.rows_hclkbuf.0 || row == self.rows_hclkbuf.1 {
                write!(o, " HCLKBUF")?;
            }
            for (i, mcb) in self.mcbs.iter().enumerate() {
                if row == mcb.row_mcb {
                    write!(o, " MCB{i}.MCB")?;
                }
                for (j, &r) in mcb.row_mui.iter().enumerate() {
                    if row == r {
                        write!(o, " MCB{i}.MUI{j}")?;
                    }
                }
                for (j, &r) in mcb.iop_dq.iter().enumerate() {
                    if row == r {
                        write!(o, " MCB{i}.DQ({jj0},{jj1})", jj0 = j * 2, jj1 = j * 2 + 1)?;
                    }
                }
                for (j, &r) in mcb.iop_dqs.iter().enumerate() {
                    if row == r {
                        write!(o, " MCB{i}.DQS{j}")?;
                    }
                }
                if row == mcb.iop_clk {
                    write!(o, " MCB{i}.CLK")?;
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
                        o,
                        " MCB{i}.({p0},{p1})",
                        p0 = pins[0].unwrap(),
                        p1 = pins[1].unwrap()
                    )?;
                }
            }
            writeln!(o)?;
        }
        writeln!(o, "\t}}")?;
        writeln!(
            o,
            "\trows_midbuf {rb}, {rt};",
            rb = self.rows_midbuf.0,
            rt = self.rows_midbuf.1
        )?;
        writeln!(
            o,
            "\trows_hclkbuf {rb}, {rt};",
            rb = self.rows_hclkbuf.0,
            rt = self.rows_hclkbuf.1
        )?;
        writeln!(
            o,
            "\trows_pci_ce_split {rb}, {rt};",
            rb = self.rows_pci_ce_split.0,
            rt = self.rows_pci_ce_split.1
        )?;
        if let Some((rl, rr)) = self.rows_bank_split {
            writeln!(o, "\trows_bank_split {rl}, {rr};")?;
        }
        if let Some(row) = self.row_mcb_split {
            writeln!(o, "\trow_mcb_split {row};")?;
        }

        match self.gts {
            Gts::None => (),
            Gts::Single(cl) => writeln!(o, "\tgts single {cl};")?,
            Gts::Double(cl, cr) => writeln!(o, "\tgts double {cl}, {cr};")?,
            Gts::Quad(cl, cr) => writeln!(o, "\tgts quad {cl}, {cr};")?,
        }

        for mcb in &self.mcbs {
            writeln!(o, "\tmcb {} {{", mcb.row_mcb)?;
            writeln!(
                o,
                "\t\tmui {};",
                mcb.row_mui.iter().map(|x| x.to_string()).join(", ")
            )?;
            writeln!(
                o,
                "\t\tiop_dq {};",
                mcb.iop_dq.iter().map(|x| x.to_string()).join(", ")
            )?;
            writeln!(
                o,
                "\t\tiop_dqs {};",
                mcb.iop_dqs.iter().map(|x| x.to_string()).join(", ")
            )?;
            writeln!(
                o,
                "\t\tio_dm {};",
                mcb.io_dm.iter().map(|x| x.to_string()).join(", ")
            )?;
            writeln!(o, "\t\tiop_clk {};", mcb.iop_clk,)?;
            writeln!(
                o,
                "\t\tio_addr {};",
                mcb.io_addr.iter().map(|x| x.to_string()).join(", ")
            )?;
            writeln!(
                o,
                "\t\tio_ba {};",
                mcb.io_ba.iter().map(|x| x.to_string()).join(", ")
            )?;
            writeln!(o, "\t\tio_ras {};", mcb.io_ras)?;
            writeln!(o, "\t\tio_cas {};", mcb.io_cas)?;
            writeln!(o, "\t\tio_we {};", mcb.io_we)?;
            writeln!(o, "\t\tio_odt {};", mcb.io_odt)?;
            writeln!(o, "\t\tio_cke {};", mcb.io_cke)?;
            writeln!(o, "\t\tio_reset {};", mcb.io_reset)?;
            writeln!(o, "\t}}")?;
        }

        for (k, v) in &self.cfg_io {
            writeln!(o, "\tcfg_io {k} = {v};")?;
        }
        if self.has_encrypt {
            writeln!(o, "\thas_encrypt;")?;
        }
        Ok(())
    }
}
