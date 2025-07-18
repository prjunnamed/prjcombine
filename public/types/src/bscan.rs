use bincode::{Decode, Encode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub enum BScanPad {
    Input(usize),
    Output(usize),
    OutputTristate(usize, usize),
    OutputEnable(usize, usize),
    InputOutputTristate(usize, usize, usize),
    InputOutputEnable(usize, usize, usize),
    BiTristate(usize, usize),
}

#[derive(Debug, Default)]
pub struct BScanBuilder {
    pub bits: usize,
}

impl BScanBuilder {
    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn get_toi(&mut self) -> BScanPad {
        let res = BScanPad::InputOutputTristate(self.bits + 2, self.bits + 1, self.bits);
        self.bits += 3;
        res
    }

    pub fn get_to(&mut self) -> BScanPad {
        let res = BScanPad::OutputTristate(self.bits + 1, self.bits);
        self.bits += 2;
        res
    }

    pub fn get_o(&mut self) -> BScanPad {
        let res = BScanPad::Output(self.bits);
        self.bits += 1;
        res
    }

    pub fn get_i(&mut self) -> BScanPad {
        let res = BScanPad::Input(self.bits);
        self.bits += 1;
        res
    }

    pub fn get_tb(&mut self) -> BScanPad {
        let res = BScanPad::BiTristate(self.bits + 1, self.bits);
        self.bits += 2;
        res
    }
}
