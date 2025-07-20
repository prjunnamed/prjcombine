use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_interconnect::{
    dir::DirH,
    grid::{ColId, DieId, RowId, TileIobId},
};
use std::collections::BTreeSet;
use unnamed_entity::{
    EntityId, EntityIds, EntityVec,
    id::{EntityIdU8, EntityTag, EntityTagArith},
};

pub struct RegTag;
impl EntityTag for RegTag {
    const PREFIX: &'static str = "REG";
}
impl EntityTagArith for RegTag {}
pub type RegId = EntityIdU8<RegTag>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ChipKind {
    Ultrascale,
    UltrascalePlus,
}

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Ultrascale => write!(f, "ultrascale"),
            ChipKind::UltrascalePlus => write!(f, "ultrascaleplus"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Interposer {
    pub primary: DieId,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_fsr_gap: BTreeSet<ColId>,
    pub cols_hard: Vec<HardColumn>,
    pub cols_io: Vec<IoColumn>,
    pub regs: usize,
    pub ps: Option<Ps>,
    pub has_hbm: bool,
    pub config_kind: ConfigKind,
    pub is_dmc: bool,
    pub is_alt_cfg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ConfigKind {
    Config,
    Csec,
    CsecV2,
}

impl ConfigKind {
    pub fn is_csec(self) -> bool {
        matches!(self, Self::Csec | Self::CsecV2)
    }
}

impl std::fmt::Display for ConfigKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigKind::Config => write!(f, "CONFIG"),
            ConfigKind::Csec => write!(f, "CSEC"),
            ConfigKind::CsecV2 => write!(f, "CSEC_V2"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ColumnKind {
    // both W and E; W can only be plain
    CleL(CleLKind),
    // W only
    CleM(CleMKind),
    // W only
    Bram(BramKind),
    // E only
    Dsp(DspKind),
    // E only; creates a ContUram on the next W
    Uram,
    // E only; creates a ContHard on the next W (unless this is the final column)
    Hard(HardKind, usize),
    // both W and E
    Io(usize),
    // both W and E
    Gt(usize),
    // W only
    Sdfec,
    // E only
    DfeB,
    // E only; creates a ContHard on the next W
    DfeC,
    // E only; creates a ContHard on the next W
    DfeDF,
    // E only; creates a ContHard on the next W
    DfeE,
    // E only; creates a ContHard on the next W
    HdioS,
    // URAM continuation, W only
    ContUram,
    // hard or DFE continuation, W only
    ContHard,
}

impl std::fmt::Display for ColumnKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnKind::CleL(CleLKind::Plain) => write!(f, "CLEL"),
            ColumnKind::CleL(CleLKind::Dcg10) => write!(f, "CLEL:DCG10"),
            ColumnKind::CleM(CleMKind::Plain) => write!(f, "CLEM"),
            ColumnKind::CleM(CleMKind::ClkBuf) => write!(f, "CLEM:CLKBUF"),
            ColumnKind::CleM(CleMKind::Laguna) => write!(f, "CLEM:LAGUNA"),
            ColumnKind::Bram(BramKind::Plain) => write!(f, "BRAM"),
            ColumnKind::Bram(BramKind::Td) => write!(f, "BRAM:TD"),
            ColumnKind::Bram(BramKind::AuxClmp) => write!(f, "BRAM:AUXCLMP"),
            ColumnKind::Bram(BramKind::AuxClmpMaybe) => write!(f, "BRAM:AUXCLMP_MAYBE"),
            ColumnKind::Bram(BramKind::BramClmp) => write!(f, "BRAM:BRAMCLMP"),
            ColumnKind::Bram(BramKind::BramClmpMaybe) => write!(f, "BRAM:BRAMCLMP_MAYBE"),
            ColumnKind::Dsp(DspKind::Plain) => write!(f, "DSP"),
            ColumnKind::Dsp(DspKind::ClkBuf) => write!(f, "DSP:CLKBUF"),
            ColumnKind::Uram => write!(f, "URAM"),
            ColumnKind::Hard(HardKind::Clk, i) => write!(f, "HARD:CLK:{i}"),
            ColumnKind::Hard(HardKind::NonClk, i) => write!(f, "HARD:NON_CLK:{i}"),
            ColumnKind::Hard(HardKind::Term, i) => write!(f, "HARD:TERM:{i}"),
            ColumnKind::Io(i) => write!(f, "IO:{i}"),
            ColumnKind::Gt(i) => write!(f, "GT:{i}"),
            ColumnKind::Sdfec => write!(f, "SDFEC"),
            ColumnKind::DfeB => write!(f, "DFE_B"),
            ColumnKind::DfeC => write!(f, "DFE_C"),
            ColumnKind::DfeDF => write!(f, "DFE_DF"),
            ColumnKind::DfeE => write!(f, "DFE_E"),
            ColumnKind::HdioS => write!(f, "HDIOS"),
            ColumnKind::ContUram => write!(f, "CONT_URAM"),
            ColumnKind::ContHard => write!(f, "CONT_HARD"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum CleMKind {
    Plain,
    ClkBuf,
    Laguna,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum BramKind {
    Plain,
    AuxClmp,
    BramClmp,
    AuxClmpMaybe,
    BramClmpMaybe,
    Td,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum CleLKind {
    Plain,
    Dcg10,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum HardKind {
    Clk,
    NonClk,
    Term,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum DspKind {
    Plain,
    ClkBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Column {
    pub kind: ColumnKind,
    pub clk: [Option<u8>; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum HardRowKind {
    None,
    Cfg,
    Ams,
    Hdio,
    HdioAms,
    HdioL,
    Pcie,
    Pcie4C,
    Pcie4CE,
    Cmac,
    Ilkn,
    DfeA,
    DfeG,
}

impl std::fmt::Display for HardRowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardRowKind::None => write!(f, "NONE"),
            HardRowKind::Cfg => write!(f, "CFG"),
            HardRowKind::Ams => write!(f, "AMS"),
            HardRowKind::Pcie => write!(f, "PCIE"),
            HardRowKind::Pcie4C => write!(f, "PCIE4C"),
            HardRowKind::Pcie4CE => write!(f, "PCIE4CE"),
            HardRowKind::Cmac => write!(f, "CMAC"),
            HardRowKind::Ilkn => write!(f, "ILKN"),
            HardRowKind::DfeA => write!(f, "DFE_A"),
            HardRowKind::DfeG => write!(f, "DFE_G"),
            HardRowKind::Hdio => write!(f, "HDIO"),
            HardRowKind::HdioAms => write!(f, "HDIO:AMS"),
            HardRowKind::HdioL => write!(f, "HDIOL"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum IoRowKind {
    None,
    Hpio,
    Hrio,
    HdioL,
    Xp5io,
    Gth,
    Gty,
    Gtm,
    Gtf,
    HsAdc,
    HsDac,
    RfAdc,
    RfDac,
}

impl std::fmt::Display for IoRowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoRowKind::None => write!(f, "NONE"),
            IoRowKind::Hpio => write!(f, "HPIO"),
            IoRowKind::Hrio => write!(f, "HRIO"),
            IoRowKind::HdioL => write!(f, "HDIOL"),
            IoRowKind::Xp5io => write!(f, "XP5IO"),
            IoRowKind::Gth => write!(f, "GTH"),
            IoRowKind::Gty => write!(f, "GTY"),
            IoRowKind::Gtm => write!(f, "GTM"),
            IoRowKind::Gtf => write!(f, "GTF"),
            IoRowKind::HsAdc => write!(f, "HSADC"),
            IoRowKind::HsDac => write!(f, "HSDAC"),
            IoRowKind::RfAdc => write!(f, "RFADC"),
            IoRowKind::RfDac => write!(f, "RFDAC"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct IoColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, IoRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Ps {
    pub col: ColId,
    pub has_vcu: bool,
    pub intf_kind: PsIntfKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum PsIntfKind {
    Alto,
    Da6,
    Da7,
    Da8,
    Dc12,
    Mx8,
}

impl Ps {
    pub fn height(self) -> usize {
        if self.has_vcu { 240 } else { 180 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub enum DisabledPart {
    Region(DieId, RegId),
    TopRow(DieId, RegId),
    HardIp(DieId, ColId, RegId),
    Gt(DieId, ColId, RegId),
    GtBufs(DieId, ColId, RegId),
    GtmSpareBufs(DieId, ColId, RegId),
    HdioIob(DieId, ColId, RegId, TileIobId),
    HpioIob(DieId, ColId, RegId, TileIobId),
    HpioDci(DieId, ColId, RegId),
    Dfe,
    Sdfec,
    Ps,
    Vcu,
    HbmLeft,
}

impl std::fmt::Display for DisabledPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisabledPart::Region(die, reg) => write!(f, "REGION:{die}:{reg}"),
            DisabledPart::TopRow(die, reg) => write!(f, "TOP_ROW:{die}:{reg}"),
            DisabledPart::HardIp(die, col, reg) => write!(f, "HARD_IP:{die}:{col}:{reg}"),
            DisabledPart::Gt(die, col, reg) => write!(f, "GT:{die}:{col}:{reg}"),
            DisabledPart::GtBufs(die, col, reg) => write!(f, "GT_BUFS:{die}:{col}:{reg}"),
            DisabledPart::GtmSpareBufs(die, col, reg) => {
                write!(f, "GTM_SPARE_BUFS:{die}:{col}:{reg}")
            }
            DisabledPart::HdioIob(die, col, reg, iob) => write!(f, "HDIO:{die}:{col}:{reg}:{iob}"),
            DisabledPart::HpioIob(die, col, reg, iob) => write!(f, "HPIO:{die}:{col}:{reg}:{iob}"),
            DisabledPart::HpioDci(die, col, reg) => write!(f, "HPIO_DCI:{die}:{col}:{reg}"),
            DisabledPart::Dfe => write!(f, "DFE"),
            DisabledPart::Sdfec => write!(f, "SDFEC"),
            DisabledPart::Ps => write!(f, "PS"),
            DisabledPart::Vcu => write!(f, "VCU"),
            DisabledPart::HbmLeft => write!(f, "HBM_LEFT"),
        }
    }
}

impl Chip {
    pub const ROWS_PER_REG: usize = 60;

    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / Chip::ROWS_PER_REG)
    }

    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * Chip::ROWS_PER_REG)
    }

    pub fn row_reg_rclk(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * Chip::ROWS_PER_REG + Chip::ROWS_PER_REG / 2)
    }

    pub fn row_rclk(&self, row: RowId) -> RowId {
        self.row_reg_rclk(self.row_to_reg(row))
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.regs * Chip::ROWS_PER_REG)
    }

    pub fn is_laguna_row(&self, row: RowId) -> bool {
        let reg = self.row_to_reg(row);
        (reg.to_idx() == 0 && !self.has_hbm) || reg.to_idx() == self.regs - 1
    }

    pub fn col_cfg(&self) -> ColId {
        self.cols_hard
            .iter()
            .find(|hc| hc.regs.values().any(|&x| x == HardRowKind::Cfg))
            .unwrap()
            .col
    }

    pub fn row_ams(&self) -> RowId {
        for hc in &self.cols_hard {
            for (reg, &kind) in &hc.regs {
                if kind == HardRowKind::Ams {
                    return self.row_reg_rclk(reg);
                }
            }
        }
        unreachable!()
    }

    pub fn reg_cfg(&self) -> RegId {
        for hc in &self.cols_hard {
            for (reg, &kind) in &hc.regs {
                if kind == HardRowKind::Cfg {
                    return reg;
                }
            }
        }
        unreachable!()
    }

    pub fn is_dc12(&self) -> bool {
        if let Some(ps) = self.ps {
            matches!(ps.intf_kind, PsIntfKind::Dc12 | PsIntfKind::Mx8)
        } else {
            false
        }
    }

    pub fn is_nocfg(&self) -> bool {
        let reg_cfg = self.reg_cfg();
        !self
            .cols_io
            .iter()
            .any(|x| matches!(x.regs[reg_cfg], IoRowKind::Hpio | IoRowKind::Hrio))
    }

    pub fn col_side(&self, col: ColId) -> DirH {
        if col.to_idx().is_multiple_of(2) {
            DirH::W
        } else {
            DirH::E
        }
    }

    pub fn in_int_hole(&self, col: ColId, row: RowId) -> bool {
        if let Some(ps) = self.ps
            && row.to_idx() < ps.height()
            && col < ps.col
        {
            return true;
        }
        false
    }

    pub fn in_site_hole(&self, col: ColId, row: RowId) -> bool {
        if let Some(ps) = self.ps
            && row.to_idx() < ps.height()
            && col <= ps.col
        {
            return true;
        }
        if self.has_hbm && matches!(self.columns[col].kind, ColumnKind::Dsp(_)) && row.to_idx() < 15
        {
            return true;
        }
        false
    }
}

impl From<&HardColumn> for JsonValue {
    fn from(hcol: &HardColumn) -> Self {
        jzon::object! {
            col: hcol.col.to_idx(),
            regs: Vec::from_iter(hcol.regs.values().map(|&kind| if kind == HardRowKind::None {
                JsonValue::Null
            } else {
                kind.to_string().into()
            })),
        }
    }
}

impl From<&IoColumn> for JsonValue {
    fn from(iocol: &IoColumn) -> Self {
        jzon::object! {
            col: iocol.col.to_idx(),
            regs: Vec::from_iter(iocol.regs.values().map(|&kind| if kind == IoRowKind::None {
                JsonValue::Null
            } else {
                kind.to_string().into()
            })),
        }
    }
}

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> Self {
        jzon::object! {
            kind: chip.kind.to_string(),
            columns: Vec::from_iter(chip.columns.values().map(|column| jzon::object! {
                kind: column.kind.to_string(),
                clk: column.clk.to_vec(),
            })),
            cols_vbrk: Vec::from_iter(chip.cols_vbrk.iter().map(|col| col.to_idx())),
            cols_fsr_gap: Vec::from_iter(chip.cols_fsr_gap.iter().map(|col| col.to_idx())),
            cols_hard: Vec::from_iter(chip.cols_hard.iter()),
            cols_io: Vec::from_iter(chip.cols_io.iter()),
            regs: chip.regs,
            ps: match chip.ps {
                None => JsonValue::Null,
                Some(ps) => jzon::object! {
                    col: ps.col.to_idx(),
                    has_vcu: ps.has_vcu,
                },
            },
            has_hbm: chip.has_hbm,
            config_kind: chip.config_kind.to_string(),
            is_alt_cfg: chip.is_alt_cfg,
            is_dmc: chip.is_dmc,
        }
    }
}

impl From<&Interposer> for JsonValue {
    fn from(interp: &Interposer) -> Self {
        jzon::object! {
            primary: interp.primary.to_idx(),
        }
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {k}", k = self.kind)?;
        if let Some(ps) = self.ps {
            write!(f, "\tPS {v:?}", v = ps.intf_kind)?;
            if ps.has_vcu {
                write!(f, " VCU")?;
            }
            writeln!(f)?;
        }
        if self.has_hbm {
            writeln!(f, "\tHAS HBM")?;
        }
        writeln!(f, "\tCONFIG: {}", self.config_kind)?;
        if self.is_dmc {
            writeln!(f, "\tIS DMC")?;
        }
        if self.is_alt_cfg {
            writeln!(f, "\tIS ALT CFG")?;
        }
        writeln!(f, "\tCOLS:")?;
        for (col, cd) in &self.columns {
            if self.cols_vbrk.contains(&col) {
                writeln!(f, "\t\t--- break")?;
            }
            if self.cols_fsr_gap.contains(&col) {
                writeln!(f, "\t\t--- FSR gap")?;
            }
            if matches!(cd.kind, ColumnKind::ContUram | ColumnKind::ContHard) {
                continue;
            }
            if matches!(
                cd.kind,
                ColumnKind::Uram
                    | ColumnKind::Hard(_, _)
                    | ColumnKind::DfeC
                    | ColumnKind::DfeDF
                    | ColumnKind::DfeE
            ) {
                write!(f, "\t\t{col}-{cr}: ", cr = col + 1)?;
            } else {
                write!(f, "\t\t{col}: ")?;
            }
            write!(f, "{}", cd.kind)?;
            if cd.clk.iter().any(|x| x.is_some()) {
                write!(f, " CLK")?;
                for v in cd.clk {
                    if let Some(v) = v {
                        write!(f, " {v}")?;
                    } else {
                        write!(f, " -")?;
                    }
                }
            }
            if let Some(ps) = self.ps
                && ps.col == col
            {
                write!(f, " PS")?;
            }
            writeln!(f,)?;
            if let ColumnKind::Io(idx) | ColumnKind::Gt(idx) = cd.kind {
                let ioc = &self.cols_io[idx];
                for (reg, kind) in &ioc.regs {
                    writeln!(f, "\t\t\t{y}: {kind}", y = self.row_reg_bot(reg))?;
                }
            }
            if let ColumnKind::Hard(_, idx) = cd.kind {
                let hc = &self.cols_hard[idx];
                for (reg, kind) in &hc.regs {
                    writeln!(f, "\t\t\t{y}: {kind}", y = self.row_reg_bot(reg))?;
                }
            }
        }
        writeln!(f, "\tREGS: {r}", r = self.regs)?;

        Ok(())
    }
}

impl std::fmt::Display for Interposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tPRIMARY: D{}", self.primary)?;
        Ok(())
    }
}
