use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Ultrascale,
    UltrascalePlus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ColSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_fsr_gap: BTreeSet<ColId>,
    pub col_cfg: HardColumn,
    pub col_hard: Option<HardColumn>,
    pub cols_io: Vec<IoColumn>,
    pub regs: usize,
    pub ps: Option<Ps>,
    pub has_hbm: bool,
    pub is_dmc: bool,
    pub is_alt_cfg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKindLeft {
    CleL,
    CleM(CleMKind),
    Bram(BramKind),
    Uram,
    Hard,
    Io,
    Gt,
    Sdfec,
    DfeC,
    DfeDF,
    DfeE,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CleMKind {
    Plain,
    ClkBuf,
    Laguna,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BramKind {
    Plain,
    AuxClmp,
    BramClmp,
    AuxClmpMaybe,
    BramClmpMaybe,
    Td,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKindRight {
    CleL(CleLKind),
    Dsp(DspKind),
    Uram,
    Hard,
    Io,
    Gt,
    DfeB,
    DfeC,
    DfeDF,
    DfeE,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CleLKind {
    Plain,
    Dcg10,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DspKind {
    Plain,
    ClkBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub l: ColumnKindLeft,
    pub r: ColumnKindRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HardRowKind {
    None,
    Cfg,
    Ams,
    Hdio,
    HdioAms,
    Pcie,
    PciePlus,
    Cmac,
    Ilkn,
    DfeA,
    DfeG,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: Vec<HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum IoRowKind {
    None,
    Hpio,
    Hrio,
    Gth,
    Gty,
    Gtm,
    Gtf,
    HsAdc,
    HsDac,
    RfAdc,
    RfDac,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub side: ColSide,
    pub regs: Vec<IoRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ps {
    pub col: ColId,
    pub has_vcu: bool,
}

impl Ps {
    pub fn height(self) -> usize {
        if self.has_vcu {
            240
        } else {
            180
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    ProgB,
    Done,
    M0,
    M1,
    M2,
    Cclk,
    HswapEn,
    InitB,
    RdWrB,
    Data(u8),
    CfgBvs,
    PorOverride,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    RxP(u8),
    RxN(u8),
    TxP(u8),
    TxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVcc,
    AVtt,
    RRef,
    AVttRCal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegion {
    All,
    L,
    R,
    LS,
    RS,
    LLC,
    RLC,
    LC,
    RC,
    LUC,
    RUC,
    LN,
    RN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtRegionPin {
    AVtt,
    AVcc,
    VccAux,
    VccInt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SysMonPin {
    VP,
    VN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PsPin {
    Mio(u32),
    Clk,
    PorB,
    SrstB,
    DdrDq(u32),
    DdrDm(u32),
    DdrDqsP(u32),
    DdrDqsN(u32),
    DdrA(u32),
    DdrBa(u32),
    DdrCkP(u32),
    DdrCkN(u32),
    DdrCke(u32),
    DdrOdt(u32),
    DdrDrstB,
    DdrCsB(u32),
    ErrorOut,
    ErrorStatus,
    Done,
    InitB,
    ProgB,
    JtagTck,
    JtagTdi,
    JtagTdo,
    JtagTms,
    Mode(u32),
    PadI,
    PadO,
    DdrActN,
    DdrAlertN,
    DdrBg(u32),
    DdrParity,
    DdrZq,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HbmPin {
    Vcc,
    VccIo,
    VccAux,
    Rsvd,
    RsvdGnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RfDacPin {
    VOutP(u8),
    VOutN(u8),
    ClkP,
    ClkN,
    RExt,
    SysRefP,
    SysRefN,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RfAdcPin {
    VInP(u8),
    VInN(u8),
    VInPairP(u8),
    VInPairN(u8),
    ClkP,
    ClkN,
    VCm(u8),
    RExt,
    PllTestOutP,
    PllTestOutN,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    // bank, bel idx
    Io(u32, u32),
    IoVref(u32),
    // bank, type
    Gt(u32, GtPin),
    GtRegion(GtRegion, GtRegionPin),
    // bank, type
    SysMon(DieId, SysMonPin),
    SysMonVRefP,
    SysMonVRefN,
    SysMonGnd,
    SysMonVcc,
    PsSysMonGnd,
    PsSysMonVcc,
    Cfg(CfgPin),
    Gnd,
    VccInt,
    VccAux,
    VccBram,
    VccAuxHpio,
    VccAuxHdio,
    VccAuxIo,
    VccIntIo,
    VccO(u32),
    VccBatt,
    Nc,
    Rsvd,
    RsvdGnd,
    Dxp,
    Dxn,
    VccPsAux,
    VccPsPll,
    VccPsIntLp,
    VccPsIntFp,
    VccPsIntFpDdr,
    VccPsBatt,
    VccPsDdrPll,
    VccIntVcu,
    // xqrku060 special
    GndSense,
    VccIntSense,
    IoPs(u32, PsPin),
    Hbm(u32, HbmPin),
    // RFSoC
    VccIntAms,
    VccSdfec,
    RfDacGnd,
    RfDacSubGnd,
    RfDacAVcc,
    RfDacAVccAux,
    RfDacAVtt,
    RfAdcGnd,
    RfAdcSubGnd,
    RfAdcAVcc,
    RfAdcAVccAux,
    RfDac(u32, RfDacPin),
    RfAdc(u32, RfAdcPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Region(DieId, u32),
    TopRow(DieId, u32),
    Ps,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoKind {
    Hpio,
    Hrio,
    Hdio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub col: ColId,
    pub side: Option<ColSide>,
    pub reg: u32,
    pub bel: u32,
    pub iox: u32,
    pub ioy: u32,
    pub bank: u32,
    pub kind: IoKind,
    pub grid_kind: GridKind,
    pub is_hdio_ams: bool,
    pub is_alt_cfg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×32 total, but 0-3 are dedicated; high 16 bits are also low 16 bits of Addr
    Data(u8),
    Addr(u8), // ×29 total, but 0-15 are represented as Data(16-31)
    CsiB,     // doubles as ADV_B
    Dout,     // doubles as CSO_B
    EmCclk,
    PudcB,
    Rs(u8), // ×2
    FweB,   // doubles as FCS2_B
    FoeB,
    I2cSclk,
    I2cSda,   // on Ultrascale+, doubles as PERSTN1
    SmbAlert, // Ultrascale+ only
    PerstN0,
    PerstN1, // Ultrascale only (shared with I2C_SDA on Ultrascale+)
}

impl Io {
    pub fn iob_name(&self) -> String {
        let x = self.iox;
        let y = self.ioy;
        format!("IOB_X{x}Y{y}")
    }
    pub fn is_vrp(&self) -> bool {
        self.kind == IoKind::Hpio && self.bel == 12
    }
    pub fn is_dbc(&self) -> bool {
        self.kind != IoKind::Hdio && matches!(self.bel, 0 | 1 | 6 | 7 | 39 | 40 | 45 | 46)
    }
    pub fn is_qbc(&self) -> bool {
        self.kind != IoKind::Hdio && matches!(self.bel, 13 | 14 | 19 | 20 | 26 | 27 | 32 | 33)
    }
    pub fn is_gc(&self) -> bool {
        if self.kind == IoKind::Hdio {
            matches!(self.bel, 8..=15)
        } else {
            matches!(self.bel, 21 | 22 | 23 | 24 | 26 | 27 | 28 | 29)
        }
    }
    pub fn sm_pair(&self) -> Option<u32> {
        if self.kind == IoKind::Hdio {
            if self.is_hdio_ams {
                match self.bel {
                    0 | 1 => Some(11),
                    2 | 3 => Some(10),
                    4 | 5 => Some(9),
                    6 | 7 => Some(8),
                    8 | 9 => Some(7),
                    10 | 11 => Some(6),
                    12 | 13 => Some(5),
                    14 | 15 => Some(4),
                    16 | 17 => Some(3),
                    18 | 19 => Some(2),
                    20 | 21 => Some(1),
                    22 | 23 => Some(0),
                    _ => None,
                }
            } else {
                match self.bel {
                    0 | 1 => Some(15),
                    2 | 3 => Some(14),
                    4 | 5 => Some(13),
                    6 | 7 => Some(12),
                    16 | 17 => Some(11),
                    18 | 19 => Some(10),
                    20 | 21 => Some(9),
                    22 | 23 => Some(8),
                    _ => None,
                }
            }
        } else {
            match self.bel {
                4 | 5 => Some(15),
                6 | 7 => Some(7),
                8 | 9 => Some(14),
                10 | 11 => Some(6),
                13 | 14 => Some(13),
                15 | 16 => Some(5),
                17 | 18 => Some(12),
                19 | 20 => Some(4),
                30 | 31 => Some(11),
                32 | 33 => Some(3),
                34 | 35 => Some(10),
                36 | 37 => Some(2),
                39 | 40 => Some(9),
                41 | 42 => Some(1),
                43 | 44 => Some(8),
                45 | 46 => Some(0),
                _ => None,
            }
        }
    }
    pub fn get_cfg(&self) -> Option<SharedCfgPin> {
        if !self.is_alt_cfg {
            match (self.bank, self.bel) {
                (65, 0) => Some(SharedCfgPin::Rs(0)),
                (65, 1) => Some(SharedCfgPin::Rs(1)),
                (65, 2) => Some(SharedCfgPin::FoeB),
                (65, 3) => Some(SharedCfgPin::FweB),
                (65, 4) => Some(SharedCfgPin::Addr(26)),
                (65, 5) => Some(SharedCfgPin::Addr(27)),
                (65, 6) => Some(SharedCfgPin::Addr(24)),
                (65, 7) => Some(SharedCfgPin::Addr(25)),
                (65, 8) => Some(SharedCfgPin::Addr(22)),
                (65, 9) => Some(SharedCfgPin::Addr(23)),
                (65, 10) => Some(SharedCfgPin::Addr(20)),
                (65, 11) => Some(SharedCfgPin::Addr(21)),
                (65, 12) => Some(SharedCfgPin::Addr(28)),
                (65, 13) => Some(SharedCfgPin::Addr(18)),
                (65, 14) => Some(SharedCfgPin::Addr(19)),
                (65, 15) => Some(SharedCfgPin::Addr(16)),
                (65, 16) => Some(SharedCfgPin::Addr(17)),
                (65, 17) => Some(SharedCfgPin::Data(30)),
                (65, 18) => Some(SharedCfgPin::Data(31)),
                (65, 19) => Some(SharedCfgPin::Data(28)),
                (65, 20) => Some(SharedCfgPin::Data(29)),
                (65, 21) => Some(SharedCfgPin::Data(26)),
                (65, 22) => Some(SharedCfgPin::Data(27)),
                (65, 23) => Some(SharedCfgPin::Data(24)),
                (65, 24) => Some(SharedCfgPin::Data(25)),
                (65, 25) => Some(if self.grid_kind == GridKind::Ultrascale {
                    SharedCfgPin::PerstN1
                } else {
                    SharedCfgPin::SmbAlert
                }),
                (65, 26) => Some(SharedCfgPin::Data(22)),
                (65, 27) => Some(SharedCfgPin::Data(23)),
                (65, 28) => Some(SharedCfgPin::Data(20)),
                (65, 29) => Some(SharedCfgPin::Data(21)),
                (65, 30) => Some(SharedCfgPin::Data(18)),
                (65, 31) => Some(SharedCfgPin::Data(19)),
                (65, 32) => Some(SharedCfgPin::Data(16)),
                (65, 33) => Some(SharedCfgPin::Data(17)),
                (65, 34) => Some(SharedCfgPin::Data(14)),
                (65, 35) => Some(SharedCfgPin::Data(15)),
                (65, 36) => Some(SharedCfgPin::Data(12)),
                (65, 37) => Some(SharedCfgPin::Data(13)),
                (65, 38) => Some(SharedCfgPin::CsiB),
                (65, 39) => Some(SharedCfgPin::Data(10)),
                (65, 40) => Some(SharedCfgPin::Data(11)),
                (65, 41) => Some(SharedCfgPin::Data(8)),
                (65, 42) => Some(SharedCfgPin::Data(9)),
                (65, 43) => Some(SharedCfgPin::Data(6)),
                (65, 44) => Some(SharedCfgPin::Data(7)),
                (65, 45) => Some(SharedCfgPin::Data(4)),
                (65, 46) => Some(SharedCfgPin::Data(5)),
                (65, 47) => Some(SharedCfgPin::I2cSclk),
                (65, 48) => Some(SharedCfgPin::I2cSda),
                (65, 49) => Some(SharedCfgPin::EmCclk),
                (65, 50) => Some(SharedCfgPin::Dout),
                (65, 51) => Some(SharedCfgPin::PerstN0),
                _ => None,
            }
        } else {
            match (self.bank, self.bel) {
                (65, 0) => Some(SharedCfgPin::Rs(1)),
                (65, 1) => Some(SharedCfgPin::FweB),
                (65, 2) => Some(SharedCfgPin::Rs(0)),
                (65, 3) => Some(SharedCfgPin::FoeB),
                (65, 4) => Some(SharedCfgPin::Addr(28)),
                (65, 5) => Some(SharedCfgPin::Addr(26)),
                (65, 6) => Some(SharedCfgPin::SmbAlert),
                (65, 7) => Some(SharedCfgPin::Addr(27)),
                (65, 8) => Some(SharedCfgPin::Addr(24)),
                (65, 9) => Some(SharedCfgPin::Addr(22)),
                (65, 10) => Some(SharedCfgPin::Addr(25)),
                (65, 11) => Some(SharedCfgPin::Addr(23)),
                (65, 12) => Some(SharedCfgPin::Addr(20)),
                (65, 13) => Some(SharedCfgPin::Addr(18)),
                (65, 14) => Some(SharedCfgPin::Addr(16)),
                (65, 15) => Some(SharedCfgPin::Addr(19)),
                (65, 16) => Some(SharedCfgPin::Addr(17)),
                (65, 17) => Some(SharedCfgPin::Data(30)),
                (65, 18) => Some(SharedCfgPin::Data(28)),
                (65, 19) => Some(SharedCfgPin::Data(31)),
                (65, 20) => Some(SharedCfgPin::Data(29)),
                (65, 21) => Some(SharedCfgPin::Data(26)),
                (65, 22) => Some(SharedCfgPin::Data(24)),
                (65, 23) => Some(SharedCfgPin::Data(27)),
                (65, 24) => Some(SharedCfgPin::Data(25)),
                (65, 25) => Some(SharedCfgPin::Addr(21)),
                (65, 26) => Some(SharedCfgPin::CsiB),
                (65, 27) => Some(SharedCfgPin::Data(22)),
                (65, 28) => Some(SharedCfgPin::EmCclk),
                (65, 29) => Some(SharedCfgPin::Data(23)),
                (65, 30) => Some(SharedCfgPin::Data(20)),
                (65, 31) => Some(SharedCfgPin::Data(18)),
                (65, 32) => Some(SharedCfgPin::Data(21)),
                (65, 33) => Some(SharedCfgPin::Data(19)),
                (65, 34) => Some(SharedCfgPin::Data(16)),
                (65, 35) => Some(SharedCfgPin::Data(14)),
                (65, 36) => Some(SharedCfgPin::Data(17)),
                (65, 37) => Some(SharedCfgPin::Data(15)),
                (65, 38) => Some(SharedCfgPin::Data(12)),
                (65, 39) => Some(SharedCfgPin::Data(10)),
                (65, 40) => Some(SharedCfgPin::Data(8)),
                (65, 41) => Some(SharedCfgPin::Data(11)),
                (65, 42) => Some(SharedCfgPin::Data(9)),
                (65, 43) => Some(SharedCfgPin::Data(6)),
                (65, 44) => Some(SharedCfgPin::Data(4)),
                (65, 45) => Some(SharedCfgPin::Data(7)),
                (65, 46) => Some(SharedCfgPin::Data(5)),
                (65, 47) => Some(SharedCfgPin::I2cSclk),
                (65, 48) => Some(SharedCfgPin::Dout),
                (65, 49) => Some(SharedCfgPin::I2cSda),
                (65, 50) => Some(SharedCfgPin::PerstN0),
                (65, 51) => Some(SharedCfgPin::Data(13)),
                _ => None,
            }
        }
    }
}

pub fn get_io(
    grids: &EntityVec<DieId, Grid>,
    grid_master: DieId,
    disabled: &BTreeSet<DisabledPart>,
) -> Vec<Io> {
    let mut res = Vec::new();
    let mut io_has_io: Vec<_> = grids[grid_master].cols_io.iter().map(|_| false).collect();
    let mut hard_has_io = false;
    let mut cfg_has_io = false;
    let mut reg_has_hprio = Vec::new();
    let mut reg_has_hdio = Vec::new();
    let mut reg_base = 0;
    let mut reg_cfg = None;
    for (gi, grid) in grids {
        for _ in 0..grid.regs {
            reg_has_hdio.push(false);
            reg_has_hprio.push(false);
        }
        for (i, c) in grid.cols_io.iter().enumerate() {
            for (j, &kind) in c.regs.iter().enumerate() {
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue;
                }
                if matches!(kind, IoRowKind::Hpio | IoRowKind::Hrio) {
                    io_has_io[i] = true;
                    reg_has_hprio[reg_base + j] = true;
                }
            }
        }
        if let Some(ref c) = grid.col_hard {
            for (j, &kind) in c.regs.iter().enumerate() {
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue;
                }
                if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                    hard_has_io = true;
                    reg_has_hdio[reg_base + j] = true;
                }
            }
        }
        for (j, &kind) in grid.col_cfg.regs.iter().enumerate() {
            if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                continue;
            }
            if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                cfg_has_io = true;
                reg_has_hdio[reg_base + j] = true;
            }
            if gi == grid_master && kind == HardRowKind::Cfg {
                reg_cfg = Some(reg_base + j);
            }
        }
        reg_base += grid.regs;
    }
    let reg_cfg = reg_cfg.unwrap();
    let mut ioy: u32 = 0;
    let mut reg_ioy = Vec::new();
    for (has_hprio, has_hdio) in reg_has_hprio.into_iter().zip(reg_has_hdio.into_iter()) {
        if has_hprio {
            reg_ioy.push((ioy, ioy + 26));
            ioy += 52;
        } else if has_hdio {
            reg_ioy.push((ioy, ioy + 12));
            ioy += 24;
        } else {
            reg_ioy.push((ioy, ioy));
        }
    }
    let mut iox_io = Vec::new();
    let mut iox_hard = 0;
    let mut iox_cfg = 0;
    let mut iox = 0;
    let mut prev_col = grids[grid_master].columns.first_id().unwrap();
    for (i, &has_io) in io_has_io.iter().enumerate() {
        if hard_has_io
            && grids[grid_master].col_hard.as_ref().unwrap().col > prev_col
            && grids[grid_master].col_hard.as_ref().unwrap().col < grids[grid_master].cols_io[i].col
        {
            iox_hard = iox;
            iox += 1;
        }
        if cfg_has_io
            && grids[grid_master].col_cfg.col > prev_col
            && grids[grid_master].col_cfg.col < grids[grid_master].cols_io[i].col
        {
            iox_cfg = iox;
            iox += 1;
        }
        iox_io.push(iox);
        if has_io {
            iox += 1;
        }
        prev_col = grids[grid_master].cols_io[i].col;
    }
    let iox_spec = if iox_io.len() > 1 && io_has_io[iox_io.len() - 2] {
        iox_io[iox_io.len() - 2]
    } else {
        assert!(io_has_io[iox_io.len() - 1]);
        iox_io[iox_io.len() - 1]
    };
    reg_base = 0;
    for (gi, grid) in grids {
        // HPIO/HRIO
        for (i, c) in grid.cols_io.iter().enumerate() {
            for (j, &kind) in c.regs.iter().enumerate() {
                let reg = reg_base + j;
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue;
                }
                if matches!(kind, IoRowKind::Hpio | IoRowKind::Hrio) {
                    for bel in 0..52 {
                        let mut bank = (65 + reg - reg_cfg) as u32 + iox_io[i] * 20 - iox_spec * 20;
                        if bank == 64 && kind == IoRowKind::Hrio {
                            if bel < 26 {
                                bank = 94;
                            } else {
                                bank = 84;
                            }
                        }
                        if i == 0
                            && iox_io[i] != iox_spec
                            && grids[grid_master].kind == GridKind::UltrascalePlus
                            && !hard_has_io
                        {
                            bank -= 20;
                        }
                        res.push(Io {
                            col: c.col,
                            side: Some(c.side),
                            reg: reg as u32,
                            bel,
                            iox: iox_io[i],
                            ioy: reg_ioy[reg].0 + bel,
                            bank,
                            kind: match kind {
                                IoRowKind::Hpio => IoKind::Hpio,
                                IoRowKind::Hrio => IoKind::Hrio,
                                _ => unreachable!(),
                            },
                            grid_kind: grid.kind,
                            is_hdio_ams: false,
                            is_alt_cfg: grid.is_alt_cfg,
                        });
                    }
                }
            }
        }
        // HDIO
        if let Some(ref c) = grid.col_hard {
            for (j, &kind) in c.regs.iter().enumerate() {
                let reg = reg_base + j;
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue;
                }
                if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                    let bank = (65 + reg - reg_cfg) as u32 + iox_hard * 20 - iox_spec * 20;
                    for bel in 0..24 {
                        res.push(Io {
                            col: c.col,
                            side: None,
                            reg: reg as u32,
                            bel,
                            iox: iox_hard,
                            ioy: if bel < 12 {
                                reg_ioy[reg].0 + bel
                            } else {
                                reg_ioy[reg].1 + bel - 12
                            },
                            bank,
                            kind: IoKind::Hdio,
                            grid_kind: grid.kind,
                            is_hdio_ams: kind == HardRowKind::HdioAms,
                            is_alt_cfg: grid.is_alt_cfg,
                        });
                    }
                }
            }
        }
        for (j, &kind) in grid.col_cfg.regs.iter().enumerate() {
            let reg = reg_base + j;
            if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                continue;
            }
            if matches!(kind, HardRowKind::Hdio | HardRowKind::HdioAms) {
                let bank = (65 + reg - reg_cfg) as u32 + iox_cfg * 20 - iox_spec * 20;
                for bel in 0..24 {
                    res.push(Io {
                        col: grid.col_cfg.col,
                        side: None,
                        reg: reg as u32,
                        bel,
                        iox: iox_cfg,
                        ioy: if bel < 12 {
                            reg_ioy[reg].0 + bel
                        } else {
                            reg_ioy[reg].1 + bel - 12
                        },
                        bank,
                        kind: IoKind::Hdio,
                        grid_kind: grid.kind,
                        is_hdio_ams: kind == HardRowKind::HdioAms,
                        is_alt_cfg: grid.is_alt_cfg,
                    });
                }
            }
        }
        reg_base += grid.regs;
    }
    res
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Gt {
    pub col: ColId,
    pub side: ColSide,
    pub reg: u32,
    pub gx: u32,
    pub gy: u32,
    pub bank: u32,
    pub kind: IoRowKind,
}

pub fn get_gt(
    grids: &EntityVec<DieId, Grid>,
    grid_master: DieId,
    disabled: &BTreeSet<DisabledPart>,
) -> Vec<Gt> {
    let mut res = Vec::new();
    for kind in [
        IoRowKind::Gth,
        IoRowKind::Gty,
        IoRowKind::Gtm,
        IoRowKind::Gtf,
        IoRowKind::HsAdc,
        IoRowKind::HsDac,
        IoRowKind::RfAdc,
        IoRowKind::RfDac,
    ] {
        let mut col_has_gt: Vec<_> = grids[grid_master].cols_io.iter().map(|_| false).collect();
        let mut reg_has_gt = Vec::new();
        let mut reg_base = 0;
        let mut reg_cfg = None;
        for (gi, grid) in grids {
            #[allow(clippy::same_item_push)]
            for _ in 0..grid.regs {
                reg_has_gt.push(false);
            }
            for (i, c) in grid.cols_io.iter().enumerate() {
                for (j, &rkind) in c.regs.iter().enumerate() {
                    let reg = reg_base + j;
                    if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                        continue;
                    }
                    if kind == rkind {
                        col_has_gt[i] = true;
                        reg_has_gt[reg] = true;
                    }
                }
            }
            for (j, &rkind) in grid.col_cfg.regs.iter().enumerate() {
                let reg = reg_base + j;
                if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                    continue;
                }
                if gi == grid_master && rkind == HardRowKind::Cfg {
                    reg_cfg = Some(reg);
                }
            }
            reg_base += grid.regs;
        }
        let reg_cfg = reg_cfg.unwrap();
        let mut gy: u32 = 0;
        let mut reg_gy = Vec::new();
        for has_gt in reg_has_gt {
            reg_gy.push(gy);
            if has_gt {
                gy += 1;
            }
        }
        let mut col_gx = Vec::new();
        let mut gx = 0;
        for has_gt in col_has_gt {
            col_gx.push(gx);
            if has_gt {
                gx += 1;
            }
        }
        reg_base = 0;
        for (gi, grid) in grids {
            for (i, c) in grid.cols_io.iter().enumerate() {
                for (j, &rkind) in c.regs.iter().enumerate() {
                    let reg = reg_base + j;
                    if disabled.contains(&DisabledPart::Region(gi, j as u32)) {
                        continue;
                    }
                    if kind != rkind {
                        continue;
                    }
                    let mut bank = (125 + reg - reg_cfg) as u32;
                    if i != 0 {
                        bank += 100;
                    }
                    res.push(Gt {
                        col: c.col,
                        side: c.side,
                        reg: reg as u32,
                        gx: col_gx[i],
                        gy: reg_gy[reg],
                        bank,
                        kind,
                    });
                }
            }
            reg_base += grid.regs;
        }
    }
    res
}

pub fn expand_grid<'a>(
    grids: &EntityVec<DieId, &Grid>,
    _grid_master: DieId,
    disabled: &BTreeSet<DisabledPart>,
    db: &'a IntDb,
) -> ExpandedGrid<'a> {
    let mut egrid = ExpandedGrid::new(db);
    let mut yb = 0;
    let mut syb = 0;
    let has_hbm = grids.first().unwrap().has_hbm;
    for (dieid, grid) in grids {
        let mut reg_skip_bot = 0;
        let mut reg_skip_top = 0;
        for i in 0..grid.regs {
            if disabled.contains(&DisabledPart::Region(dieid, i as u32)) {
                reg_skip_bot += 1;
            } else {
                break;
            }
        }
        for i in (0..grid.regs).rev() {
            if disabled.contains(&DisabledPart::Region(dieid, i as u32)) {
                reg_skip_top += 1;
            } else {
                break;
            }
        }
        if grid.kind == GridKind::Ultrascale && reg_skip_bot != 0 {
            yb += 1;
        }
        let has_laguna = grid
            .columns
            .values()
            .any(|cd| cd.l == ColumnKindLeft::CleM(CleMKind::Laguna));
        let row_skip = reg_skip_bot * 60;
        let (_, mut die) = egrid.add_die(grid.columns.len(), grid.regs * 60);
        for (col, &cd) in &grid.columns {
            let x = col.to_idx();
            for row in die.rows() {
                let y = if row.to_idx() < row_skip {
                    0
                } else {
                    yb + row.to_idx() - row_skip
                };
                die.fill_tile((col, row), "INT", "INT", format!("INT_X{x}Y{y}"));
                if row.to_idx() % 60 == 30 {
                    let lr = if col < grid.col_cfg.col { 'L' } else { 'R' };
                    let name = format!("RCLK_INT_{lr}_X{x}Y{yy}", yy = y - 1);
                    die[(col, row)].add_xnode(
                        db.get_node("RCLK"),
                        &[&name],
                        db.get_node_naming("RCLK"),
                        &[(col, row)],
                    );
                }
                match cd.l {
                    ColumnKindLeft::CleL | ColumnKindLeft::CleM(_) => (),
                    ColumnKindLeft::Bram(_) | ColumnKindLeft::Uram => {
                        let kind = if grid.kind == GridKind::Ultrascale {
                            "INT_INTERFACE_L"
                        } else {
                            "INT_INTF_L"
                        };
                        die[(col, row)].add_intf(
                            db.get_intf("INTF.W"),
                            format!("{kind}_X{x}Y{y}"),
                            db.get_intf_naming("INTF.W"),
                        );
                    }
                    ColumnKindLeft::Gt | ColumnKindLeft::Io => {
                        let cio = grid
                            .cols_io
                            .iter()
                            .find(|x| x.col == col && x.side == ColSide::Left)
                            .unwrap();
                        let rk = cio.regs[row.to_idx() / 60];
                        match (grid.kind, rk) {
                            (_, IoRowKind::None) => (),
                            (GridKind::Ultrascale, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = "INT_INT_INTERFACE_XIPHY_FT";
                                die[(col, row)].add_intf(
                                    db.get_intf("INTF.W.DELAY"),
                                    format!("{kind}_X{x}Y{y}"),
                                    db.get_intf_naming("INTF.W.IO"),
                                );
                            }
                            (GridKind::UltrascalePlus, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = if col.to_idx() == 0 {
                                    "INT_INTF_LEFT_TERM_IO_FT"
                                } else if matches!(row.to_idx() % 15, 0 | 1 | 13 | 14) {
                                    "INT_INTF_L_CMT"
                                } else {
                                    "INT_INTF_L_IO"
                                };
                                die[(col, row)].add_intf(
                                    db.get_intf("INTF.W.IO"),
                                    format!("{kind}_X{x}Y{y}"),
                                    db.get_intf_naming("INTF.W.IO"),
                                );
                            }
                            _ => {
                                let kind = if grid.kind == GridKind::Ultrascale {
                                    "INT_INT_INTERFACE_GT_LEFT_FT"
                                } else {
                                    "INT_INTF_L_TERM_GT"
                                };
                                die[(col, row)].add_intf(
                                    db.get_intf("INTF.W.DELAY"),
                                    format!("{kind}_X{x}Y{y}"),
                                    db.get_intf_naming("INTF.W.GT"),
                                );
                            }
                        }
                    }
                    ColumnKindLeft::Hard
                    | ColumnKindLeft::Sdfec
                    | ColumnKindLeft::DfeC
                    | ColumnKindLeft::DfeDF
                    | ColumnKindLeft::DfeE => {
                        let kind = if grid.kind == GridKind::Ultrascale {
                            "INT_INTERFACE_PCIE_L"
                        } else {
                            "INT_INTF_L_PCIE4"
                        };
                        die[(col, row)].add_intf(
                            db.get_intf("INTF.W.DELAY"),
                            format!("{kind}_X{x}Y{y}"),
                            db.get_intf_naming("INTF.W.PCIE"),
                        );
                    }
                }
                match cd.r {
                    ColumnKindRight::CleL(_) => (),
                    ColumnKindRight::Dsp(_) | ColumnKindRight::Uram => {
                        let kind = if grid.kind == GridKind::Ultrascale {
                            "INT_INTERFACE_R"
                        } else {
                            "INT_INTF_R"
                        };
                        die[(col, row)].add_intf(
                            db.get_intf("INTF.E"),
                            format!("{kind}_X{x}Y{y}"),
                            db.get_intf_naming("INTF.E"),
                        );
                    }
                    ColumnKindRight::Gt | ColumnKindRight::Io => {
                        let cio = grid
                            .cols_io
                            .iter()
                            .find(|x| x.col == col && x.side == ColSide::Right)
                            .unwrap();
                        let rk = cio.regs[row.to_idx() / 60];
                        match (grid.kind, rk) {
                            (_, IoRowKind::None) => (),
                            (GridKind::Ultrascale, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                unreachable!()
                            }
                            (GridKind::UltrascalePlus, IoRowKind::Hpio | IoRowKind::Hrio) => {
                                let kind = "INT_INTF_RIGHT_TERM_IO";
                                die[(col, row)].add_intf(
                                    db.get_intf("INTF.E.IO"),
                                    format!("{kind}_X{x}Y{y}"),
                                    db.get_intf_naming("INTF.E.IO"),
                                );
                            }
                            _ => {
                                let kind = if grid.kind == GridKind::Ultrascale {
                                    "INT_INTERFACE_GT_R"
                                } else {
                                    "INT_INTF_R_TERM_GT"
                                };
                                die[(col, row)].add_intf(
                                    db.get_intf("INTF.E.DELAY"),
                                    format!("{kind}_X{x}Y{y}"),
                                    db.get_intf_naming("INTF.E.GT"),
                                );
                            }
                        }
                    }
                    ColumnKindRight::Hard
                    | ColumnKindRight::DfeB
                    | ColumnKindRight::DfeC
                    | ColumnKindRight::DfeDF
                    | ColumnKindRight::DfeE => {
                        let kind = if grid.kind == GridKind::Ultrascale {
                            "INT_INTERFACE_PCIE_R"
                        } else {
                            "INT_INTF_R_PCIE4"
                        };
                        die[(col, row)].add_intf(
                            db.get_intf("INTF.E.DELAY"),
                            format!("{kind}_X{x}Y{y}"),
                            db.get_intf_naming("INTF.E.PCIE"),
                        );
                    }
                }
            }
        }

        if grid.kind == GridKind::UltrascalePlus {
            for (col, &cd) in &grid.columns {
                if cd.l == ColumnKindLeft::Io && col.to_idx() != 0 {
                    let term_e = db.get_term("IO.E");
                    let term_w = db.get_term("IO.W");
                    for row in die.rows() {
                        die.fill_term_pair_anon((col - 1, row), (col, row), term_e, term_w);
                    }
                }
            }
        }

        if let Some(ps) = grid.ps {
            let height = ps.height();
            let width = ps.col.to_idx();
            die.nuke_rect(ColId::from_idx(0), RowId::from_idx(0), width, height);
            if height != grid.regs * 60 {
                let row_t = RowId::from_idx(height);
                for dx in 0..width {
                    let col = ColId::from_idx(dx);
                    die.fill_term_anon((col, row_t), "TERM.S");
                }
            }
            let x = ps.col.to_idx();
            for dy in 0..height {
                let row = RowId::from_idx(dy);
                let y = if row.to_idx() < row_skip {
                    0
                } else {
                    yb + row.to_idx() - row_skip
                };
                die.fill_term_anon((ps.col, row), "TERM.W");
                die[(ps.col, row)].insert_intf(
                    0,
                    db.get_intf("INTF.W.IO"),
                    format!("INT_INTF_LEFT_TERM_PSS_X{x}Y{y}"),
                    db.get_intf_naming("INTF.PSS"),
                );
            }
        }

        die.nuke_rect(
            ColId::from_idx(0),
            RowId::from_idx(0),
            grid.columns.len(),
            reg_skip_bot * 60,
        );
        die.nuke_rect(
            ColId::from_idx(0),
            RowId::from_idx((grid.regs - reg_skip_top) * 60),
            grid.columns.len(),
            reg_skip_top * 60,
        );

        let col_l = die.cols().next().unwrap();
        let col_r = die.cols().next_back().unwrap();
        let row_b = die.rows().next().unwrap();
        let row_t = die.rows().next_back().unwrap();
        for col in die.cols() {
            if !die[(col, row_b)].nodes.is_empty() {
                die.fill_term_anon((col, row_b), "TERM.S");
            }
            if !die[(col, row_t)].nodes.is_empty() {
                die.fill_term_anon((col, row_t), "TERM.N");
            }
        }
        for row in die.rows() {
            if !die[(col_l, row)].nodes.is_empty() {
                die.fill_term_anon((col_l, row), "TERM.W");
            }
            if !die[(col_r, row)].nodes.is_empty() {
                die.fill_term_anon((col_r, row), "TERM.E");
            }
        }

        die.fill_main_passes();

        let mut sx = 0;
        for (col, &cd) in &grid.columns {
            let mut found = false;
            if let Some((kind, tk)) = match cd.l {
                ColumnKindLeft::CleL => Some(("CLEL_L", "CLEL_L")),
                ColumnKindLeft::CleM(_) => Some((
                    "CLEM",
                    match (grid.kind, col < grid.col_cfg.col) {
                        (GridKind::Ultrascale, true) => "CLE_M",
                        (GridKind::Ultrascale, false) => "CLE_M_R",
                        (GridKind::UltrascalePlus, true) => "CLEM",
                        (GridKind::UltrascalePlus, false) => "CLEM_R",
                    },
                )),
                _ => None,
            } {
                for row in die.rows() {
                    if cd.l == ColumnKindLeft::CleM(CleMKind::Laguna) {
                        if row.to_idx() / 60 == 0 {
                            continue;
                        }
                        if row.to_idx() / 60 == grid.regs - 1 {
                            continue;
                        }
                    }
                    let tile = &mut die[(col, row)];
                    if let Some(ps) = grid.ps {
                        if col == ps.col && row.to_idx() < ps.height() {
                            continue;
                        }
                    }
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let x = col.to_idx();
                    let y = yb + row.to_idx() - row_skip;
                    let name = format!("{tk}_X{x}Y{y}");
                    let node = tile.add_xnode(
                        db.get_node(kind),
                        &[&name],
                        db.get_node_naming(kind),
                        &[(col, row)],
                    );
                    if row.to_idx() % 60 == 59
                        && disabled.contains(&DisabledPart::TopRow(dieid, row.to_idx() as u32 / 60))
                    {
                        continue;
                    }
                    let sy = syb + row.to_idx() - row_skip;
                    node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                    found = true;
                }
            }
            if found {
                sx += 1;
            }
            let mut found = false;
            if matches!(cd.r, ColumnKindRight::CleL(_)) {
                for row in die.rows() {
                    let tile = &mut die[(col, row)];
                    if tile.nodes.is_empty() {
                        continue;
                    }
                    let x = col.to_idx();
                    let y = yb + row.to_idx() - row_skip;
                    let name = format!("CLEL_R_X{x}Y{y}");
                    let node = tile.add_xnode(
                        db.get_node("CLEL_R"),
                        &[&name],
                        db.get_node_naming("CLEL_R"),
                        &[(col, row)],
                    );
                    if row.to_idx() % 60 == 59
                        && disabled.contains(&DisabledPart::TopRow(dieid, row.to_idx() as u32 / 60))
                    {
                        continue;
                    }
                    let sy = syb + row.to_idx() - row_skip;
                    node.add_bel(0, format!("SLICE_X{sx}Y{sy}"));
                    found = true;
                }
            }
            if found {
                sx += 1;
            }
        }

        let mut bx = 0;
        for (col, &cd) in &grid.columns {
            if !matches!(cd.l, ColumnKindLeft::Bram(_)) {
                continue;
            }
            for row in die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                let tile = &mut die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = col.to_idx();
                let y = yb + row.to_idx() - row_skip;
                let name = format!("BRAM_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node("BRAM"),
                    &[&name],
                    db.get_node_naming("BRAM"),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                let sy = syb + row.to_idx() - row_skip;
                node.add_bel(0, format!("RAMB36_X{bx}Y{y}", y = sy / 5));
                node.add_bel(1, format!("RAMB18_X{bx}Y{y}", y = sy / 5 * 2));
                node.add_bel(2, format!("RAMB18_X{bx}Y{y}", y = sy / 5 * 2 + 1));
                if row.to_idx() % 60 == 30 {
                    let in_laguna = has_laguna
                        && (row.to_idx() / 60 == 0 || row.to_idx() / 60 == grid.regs - 1);
                    let tk = match (grid.kind, cd.l, col < grid.col_cfg.col) {
                        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::Plain), true) => {
                            "RCLK_BRAM_L"
                        }
                        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::Plain), false) => {
                            "RCLK_BRAM_R"
                        }
                        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::BramClmp), true) => {
                            "RCLK_RCLK_BRAM_L_BRAMCLMP_FT"
                        }
                        (GridKind::Ultrascale, ColumnKindLeft::Bram(BramKind::AuxClmp), true) => {
                            "RCLK_RCLK_BRAM_L_AUXCLMP_FT"
                        }
                        (
                            GridKind::Ultrascale,
                            ColumnKindLeft::Bram(BramKind::BramClmpMaybe),
                            true,
                        ) => {
                            if in_laguna {
                                "RCLK_BRAM_L"
                            } else {
                                "RCLK_RCLK_BRAM_L_BRAMCLMP_FT"
                            }
                        }
                        (
                            GridKind::Ultrascale,
                            ColumnKindLeft::Bram(BramKind::AuxClmpMaybe),
                            true,
                        ) => {
                            if in_laguna {
                                "RCLK_BRAM_L"
                            } else {
                                "RCLK_RCLK_BRAM_L_AUXCLMP_FT"
                            }
                        }
                        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Plain), true) => {
                            "RCLK_BRAM_INTF_L"
                        }
                        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Td), true) => {
                            "RCLK_BRAM_INTF_TD_L"
                        }
                        (GridKind::UltrascalePlus, ColumnKindLeft::Bram(BramKind::Td), false) => {
                            "RCLK_BRAM_INTF_TD_R"
                        }
                        _ => unreachable!(),
                    };
                    let name_h = format!("{tk}_X{x}Y{y}", y = y - 1);
                    let node = die[(col, row)].add_xnode(
                        db.get_node("HARD_SYNC"),
                        &[&name_h],
                        db.get_node_naming("HARD_SYNC"),
                        &[(col, row)],
                    );
                    let sy = syb + row.to_idx() - row_skip;
                    node.add_bel(
                        0,
                        format!("HARD_SYNC_X{sx}Y{sy}", sx = bx * 2, sy = sy / 60 * 2),
                    );
                    node.add_bel(
                        1,
                        format!("HARD_SYNC_X{sx}Y{sy}", sx = bx * 2, sy = sy / 60 * 2 + 1),
                    );
                    node.add_bel(
                        2,
                        format!("HARD_SYNC_X{sx}Y{sy}", sx = bx * 2 + 1, sy = sy / 60 * 2),
                    );
                    node.add_bel(
                        3,
                        format!(
                            "HARD_SYNC_X{sx}Y{sy}",
                            sx = bx * 2 + 1,
                            sy = sy / 60 * 2 + 1
                        ),
                    );
                }
            }
            bx += 1;
        }

        let mut dx = 0;
        for (col, &cd) in &grid.columns {
            if !matches!(cd.r, ColumnKindRight::Dsp(_)) {
                continue;
            }
            for row in die.rows() {
                if row.to_idx() % 5 != 0 {
                    continue;
                }
                if grid.has_hbm && row.to_idx() < 15 {
                    continue;
                }
                let tile = &mut die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = col.to_idx();
                let y = yb + row.to_idx() - row_skip;
                let name = format!("DSP_X{x}Y{y}");
                let node = tile.add_xnode(
                    db.get_node("DSP"),
                    &[&name],
                    db.get_node_naming("DSP"),
                    &[
                        (col, row),
                        (col, row + 1),
                        (col, row + 2),
                        (col, row + 3),
                        (col, row + 4),
                    ],
                );
                let sy = syb + row.to_idx() - row_skip;
                let dy = if has_hbm { sy / 5 - 3 } else { sy / 5 };
                node.add_bel(0, format!("DSP48E2_X{dx}Y{y}", y = dy * 2));
                if row.to_idx() % 60 == 55
                    && disabled.contains(&DisabledPart::TopRow(dieid, row.to_idx() as u32 / 60))
                {
                    continue;
                }
                node.add_bel(1, format!("DSP48E2_X{dx}Y{y}", y = dy * 2 + 1));
            }
            dx += 1;
        }

        let mut uyb = 0;
        if let Some(ps) = grid.ps {
            uyb = ps.height();
            for (col, &cd) in &grid.columns {
                if cd.r == ColumnKindRight::Uram && col >= ps.col {
                    uyb = 0;
                }
            }
        }
        let mut ux = 0;
        for (col, &cd) in &grid.columns {
            if cd.r != ColumnKindRight::Uram {
                continue;
            }
            for row in die.rows() {
                if row.to_idx() % 15 != 0 {
                    continue;
                }
                let tile = &mut die[(col, row)];
                if tile.nodes.is_empty() {
                    continue;
                }
                let x = col.to_idx();
                let y = yb + row.to_idx() - row_skip;
                let tk = if row.to_idx() % 60 == 45 {
                    "URAM_URAM_DELAY_FT"
                } else {
                    "URAM_URAM_FT"
                };
                let name = format!("{tk}_X{x}Y{y}");
                let mut crds = vec![];
                for dy in 0..15 {
                    crds.push((col, row + dy));
                }
                for dy in 0..15 {
                    crds.push((col + 1, row + dy));
                }
                let node = tile.add_xnode(
                    db.get_node("URAM"),
                    &[&name],
                    db.get_node_naming("URAM"),
                    &crds,
                );
                let sy = syb + row.to_idx() - row_skip - uyb;
                node.add_bel(0, format!("URAM288_X{ux}Y{y}", y = sy / 15 * 4));
                node.add_bel(1, format!("URAM288_X{ux}Y{y}", y = sy / 15 * 4 + 1));
                node.add_bel(2, format!("URAM288_X{ux}Y{y}", y = sy / 15 * 4 + 2));
                node.add_bel(3, format!("URAM288_X{ux}Y{y}", y = sy / 15 * 4 + 3));
            }
            ux += 1;
        }

        for col in die.cols() {
            for row in die.rows() {
                let crow = RowId::from_idx(if row.to_idx() % 60 < 30 {
                    row.to_idx() / 60 * 60 + 29
                } else {
                    row.to_idx() / 60 * 60 + 30
                });
                die[(col, row)].clkroot = (col, crow);
            }
        }

        yb += die.rows().len() - reg_skip_bot * 60 - reg_skip_top * 60;
        syb += die.rows().len() - reg_skip_bot * 60 - reg_skip_top * 60;
    }

    egrid
}
