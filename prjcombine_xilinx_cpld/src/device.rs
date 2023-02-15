use std::collections::HashMap;

use prjcombine_entity::{EntityId, EntityIds, EntityVec};
use serde::{Deserialize, Serialize};

use crate::types::{
    BankId, ClkPadId, ExportDir, FbGroupId, FbId, FbMcId, FbnId, ImuxId, IoId, IpadId, McId,
    OePadId, PTermId,
};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub kind: DeviceKind,
    pub fbs: usize,
    pub ipads: usize,
    pub has_fbk: bool,
    pub has_vref: bool,
    pub io: HashMap<IoId, Io>,
    pub banks: usize,
    pub fb_groups: usize,
    pub fb_group: EntityVec<FbId, FbGroupId>,
    pub clk_pads: EntityVec<ClkPadId, IoId>,
    pub oe_pads: EntityVec<OePadId, IoId>,
    pub sr_pad: Option<IoId>,
    pub dge_pad: Option<IoId>,
    pub cdr_pad: Option<IoId>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeviceKind {
    Xc9500,
    Xc9500Xl,
    Xc9500Xv,
    Xpla3,
    Coolrunner2,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Io {
    pub pad: u32,
    pub bank: BankId,
    pub jtag: Option<JtagPin>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Package {
    pub pins: HashMap<String, PkgPin>,
    pub banks: EntityVec<BankId, Option<u32>>,
    pub spec_remap: HashMap<IoId, IoId>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum PkgPin {
    Nc,
    Gnd,
    VccInt,
    VccIo(BankId),
    VccAux,
    Jtag(JtagPin),
    PortEn,
    Io(IoId),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum JtagPin {
    Tdi,
    Tdo,
    Tck,
    Tms,
}

impl Device {
    pub fn fbs(&self) -> EntityIds<FbId> {
        EntityIds::new(self.fbs)
    }

    pub fn ipads(&self) -> EntityIds<IpadId> {
        EntityIds::new(self.ipads)
    }

    pub fn banks(&self) -> EntityIds<BankId> {
        EntityIds::new(self.banks)
    }

    pub fn fb_mcs(&self) -> EntityIds<FbMcId> {
        EntityIds::new(self.kind.mcs_per_fb())
    }

    pub fn fb_imuxes(&self) -> EntityIds<ImuxId> {
        EntityIds::new(self.kind.imux_per_fb())
    }

    pub fn fb_pterms(&self) -> EntityIds<PTermId> {
        EntityIds::new(self.kind.pterms_per_fb())
    }

    pub fn fb_fbns(&self) -> EntityIds<FbnId> {
        assert_eq!(self.kind, DeviceKind::Xpla3);
        EntityIds::new(8)
    }

    pub fn mcs(&self) -> impl Iterator<Item = McId> + '_ {
        self.fbs()
            .flat_map(|fb| self.fb_mcs().map(move |mc| (fb, mc)))
    }

    pub fn export_target(&self, mcid: McId, dir: ExportDir) -> McId {
        self.export_source(mcid, !dir)
    }

    pub fn export_source(&self, mcid: McId, dir: ExportDir) -> McId {
        match dir {
            ExportDir::Up => (
                mcid.0,
                if mcid.1.to_idx() == 0 {
                    FbMcId::from_idx(self.kind.mcs_per_fb() - 1)
                } else {
                    FbMcId::from_idx(mcid.1.to_idx() - 1)
                },
            ),
            ExportDir::Down => (
                mcid.0,
                if mcid.1.to_idx() == self.kind.mcs_per_fb() - 1 {
                    FbMcId::from_idx(0)
                } else {
                    FbMcId::from_idx(mcid.1.to_idx() + 1)
                },
            ),
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
        if self.is_xc9500() {
            18
        } else {
            16
        }
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
