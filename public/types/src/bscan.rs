use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BScanPin {
    Input(usize),
    Output(usize),
    OutputTristate(usize, usize),
    OutputEnable(usize, usize),
    InputOutputTristate(usize, usize, usize),
    InputOutputEnable(usize, usize, usize),
}

#[derive(Debug, Default)]
pub struct BScanBuilder {
    pub bits: usize,
}

impl BScanBuilder {
    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn get_toi(&mut self) -> BScanPin {
        let res = BScanPin::InputOutputTristate(self.bits + 2, self.bits + 1, self.bits);
        self.bits += 3;
        res
    }

    pub fn get_to(&mut self) -> BScanPin {
        let res = BScanPin::OutputTristate(self.bits + 1, self.bits);
        self.bits += 2;
        res
    }

    pub fn get_o(&mut self) -> BScanPin {
        let res = BScanPin::Output(self.bits);
        self.bits += 1;
        res
    }

    pub fn get_i(&mut self) -> BScanPin {
        let res = BScanPin::Input(self.bits);
        self.bits += 1;
        res
    }
}
