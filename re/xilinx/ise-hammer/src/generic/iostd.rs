#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DiffKind {
    None,
    Pseudo,
    True,
    TrueTerm,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DciKind {
    None,
    Output,
    OutputHalf,
    InputVcc,
    InputSplit,
    BiVcc,
    BiSplit,
    BiSplitT,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Iostd {
    pub name: &'static str,
    pub vcco: Option<u16>,
    pub vref: Option<u16>,
    pub diff: DiffKind,
    pub dci: DciKind,
    pub drive: &'static [u8],
    pub input_only: bool,
}

impl Iostd {
    pub const fn cmos(name: &'static str, vcco: u16, drive: &'static [u8]) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::None,
            dci: DciKind::None,
            drive,
            input_only: false,
        }
    }

    pub const fn cmos_od(name: &'static str) -> Iostd {
        Iostd {
            name,
            vcco: None,
            vref: None,
            diff: DiffKind::None,
            dci: DciKind::None,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn odci(name: &'static str, vcco: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::None,
            dci: DciKind::Output,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn odci_half(name: &'static str, vcco: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::None,
            dci: DciKind::OutputHalf,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn odci_vref(name: &'static str, vcco: u16, vref: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: Some(vref),
            diff: DiffKind::None,
            dci: DciKind::Output,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn vref_od(name: &'static str, vref: u16) -> Iostd {
        Iostd {
            name,
            vcco: None,
            vref: Some(vref),
            diff: DiffKind::None,
            dci: DciKind::None,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn vref(name: &'static str, vcco: u16, vref: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: Some(vref),
            diff: DiffKind::None,
            dci: DciKind::None,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn vref_input(name: &'static str, vcco: u16, vref: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: Some(vref),
            diff: DiffKind::None,
            dci: DciKind::None,
            drive: &[],
            input_only: true,
        }
    }

    pub const fn vref_dci_od(name: &'static str, vcco: u16, vref: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: Some(vref),
            diff: DiffKind::None,
            dci: DciKind::BiVcc,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn vref_dci(name: &'static str, vcco: u16, vref: u16, dci: DciKind) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: Some(vref),
            diff: DiffKind::None,
            dci,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn pseudo_diff(name: &'static str, vcco: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::Pseudo,
            dci: DciKind::None,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn pseudo_diff_dci(name: &'static str, vcco: u16, dci: DciKind) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::Pseudo,
            dci,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn true_diff(name: &'static str, vcco: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::True,
            dci: DciKind::None,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn true_diff_dci(name: &'static str, vcco: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::True,
            dci: DciKind::InputSplit,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn true_diff_term(name: &'static str, vcco: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::TrueTerm,
            dci: DciKind::None,
            drive: &[],
            input_only: false,
        }
    }

    pub const fn diff_input(name: &'static str, vcco: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::Pseudo,
            dci: DciKind::None,
            drive: &[],
            input_only: true,
        }
    }

    pub const fn true_diff_input(name: &'static str, vcco: u16) -> Iostd {
        Iostd {
            name,
            vcco: Some(vcco),
            vref: None,
            diff: DiffKind::True,
            dci: DciKind::None,
            drive: &[],
            input_only: true,
        }
    }
}
