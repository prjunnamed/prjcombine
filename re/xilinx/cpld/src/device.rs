use std::collections::HashMap;

use bincode::{Decode, Encode};
use prjcombine_types::cpld::{
    BlockId, ClusterId, IoCoord, IpadId, MacrocellCoord, MacrocellId, ProductTermId,
};
use unnamed_entity::{EntityId, EntityIds, EntityVec};

use crate::types::{BankId, ClkPadId, ExportDir, FbGroupId, FbnId, ImuxId, OePadId};

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Device {
    pub kind: DeviceKind,
    pub fbs: usize,
    pub ipads: usize,
    pub has_fbk: bool,
    pub has_vref: bool,
    pub io: HashMap<IoCoord, Io>,
    pub banks: usize,
    pub fb_groups: usize,
    pub fb_group: EntityVec<BlockId, FbGroupId>,
    pub clk_pads: EntityVec<ClkPadId, IoCoord>,
    pub oe_pads: EntityVec<OePadId, IoCoord>,
    pub sr_pad: Option<IoCoord>,
    pub dge_pad: Option<IoCoord>,
    pub cdr_pad: Option<IoCoord>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode)]
pub enum DeviceKind {
    Xc9500,
    Xc9500Xl,
    Xc9500Xv,
    Xpla3,
    Coolrunner2,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Io {
    pub pad: u32,
    pub bank: BankId,
    pub jtag: Option<JtagPin>,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
pub struct Package {
    pub pins: HashMap<String, PkgPin>,
    pub banks: EntityVec<BankId, Option<u32>>,
    pub spec_remap: HashMap<IoCoord, IoCoord>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Encode, Decode)]
pub enum PkgPin {
    Nc,
    Gnd,
    VccInt,
    VccIo(BankId),
    VccAux,
    Jtag(JtagPin),
    PortEn,
    Io(IoCoord),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Encode, Decode)]
pub enum JtagPin {
    Tdi,
    Tdo,
    Tck,
    Tms,
}

impl Device {
    pub fn fbs(&self) -> EntityIds<BlockId> {
        EntityIds::new(self.fbs)
    }

    pub fn ipads(&self) -> EntityIds<IpadId> {
        EntityIds::new(self.ipads)
    }

    pub fn banks(&self) -> EntityIds<BankId> {
        EntityIds::new(self.banks)
    }

    pub fn fb_mcs(&self) -> EntityIds<MacrocellId> {
        EntityIds::new(self.kind.mcs_per_fb())
    }

    pub fn fb_imuxes(&self) -> EntityIds<ImuxId> {
        EntityIds::new(self.kind.imux_per_fb())
    }

    pub fn fb_pterms(&self) -> EntityIds<ProductTermId> {
        EntityIds::new(self.kind.pterms_per_fb())
    }

    pub fn fb_fbns(&self) -> EntityIds<FbnId> {
        assert_eq!(self.kind, DeviceKind::Xpla3);
        EntityIds::new(8)
    }

    pub fn mcs(&self) -> impl Iterator<Item = MacrocellCoord> + '_ {
        self.fbs().flat_map(|fb| {
            self.fb_mcs().map(move |mc| MacrocellCoord {
                cluster: ClusterId::from_idx(0),
                block: fb,
                macrocell: mc,
            })
        })
    }

    pub fn export_target(&self, mcid: MacrocellCoord, dir: ExportDir) -> MacrocellCoord {
        self.export_source(mcid, !dir)
    }

    pub fn export_source(&self, mcid: MacrocellCoord, dir: ExportDir) -> MacrocellCoord {
        match dir {
            ExportDir::Up => MacrocellCoord {
                macrocell: if mcid.macrocell.to_idx() == 0 {
                    MacrocellId::from_idx(self.kind.mcs_per_fb() - 1)
                } else {
                    MacrocellId::from_idx(mcid.macrocell.to_idx() - 1)
                },
                ..mcid
            },
            ExportDir::Down => MacrocellCoord {
                macrocell: if mcid.macrocell.to_idx() == self.kind.mcs_per_fb() - 1 {
                    MacrocellId::from_idx(0)
                } else {
                    MacrocellId::from_idx(mcid.macrocell.to_idx() + 1)
                },
                ..mcid
            },
        }
    }
}

impl DeviceKind {
    pub fn is_xc9500(self) -> bool {
        matches!(self, Self::Xc9500 | Self::Xc9500Xl | Self::Xc9500Xv)
    }

    pub fn is_xc9500x(self) -> bool {
        matches!(self, Self::Xc9500Xl | Self::Xc9500Xv)
    }

    pub fn mcs_per_fb(self) -> usize {
        if self.is_xc9500() { 18 } else { 16 }
    }

    pub fn imux_per_fb(self) -> usize {
        match self {
            DeviceKind::Xc9500 => 36,
            DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => 54,
            DeviceKind::Xpla3 | DeviceKind::Coolrunner2 => 40,
        }
    }

    pub fn pterms_per_fb(self) -> usize {
        match self {
            DeviceKind::Xpla3 => 48,
            DeviceKind::Coolrunner2 => 56,
            _ => panic!("no pterms on this device"),
        }
    }
}
