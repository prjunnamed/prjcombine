use std::collections::HashMap;

pub struct StringPool {
    s2i: HashMap<String, u32>,
    i2s: Vec<String>,
}

impl StringPool {
    pub fn new() -> StringPool {
        StringPool {
            s2i: HashMap::new(),
            i2s: Vec::new(),
        }
    }

    pub fn get(&self, idx: u32) -> &str {
        &self.i2s[idx as usize]
    }

    pub fn put(&mut self, s: &str) -> u32 {
        match self.s2i.get(s) {
            None => {
                let i = self.i2s.len();
                assert!(i <= u32::MAX as usize);
                let i = i as u32;
                self.i2s.push(s.to_string());
                self.s2i.insert(s.to_string(), i);
                i
            },
            Some(i) => *i
        }
    }
}
