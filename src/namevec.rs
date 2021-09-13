use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::slice::{Iter, IterMut};
use std::ops::{Index, IndexMut};

pub trait Named {
    fn get_name(&self) -> &str;
}

impl Named for String {
    fn get_name(&self) -> &str {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameVec<T: Named> {
    vec: Vec<T>,
    index: HashMap<String, usize>,
}

impl<T: Named> NameVec<T> {
    pub fn new() -> Self {
        NameVec {
            vec: Vec::new(),
            index: HashMap::new(),
        }
    }
    pub fn push(&mut self, value: T) -> usize {
        let name = value.get_name();
        let idx = self.vec.len();
        if *self.index.entry(name.to_string()).or_insert(idx) != idx {
            panic!("name {} redefined", name);
        }
        self.vec.push(value);
        idx
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }
    pub fn idx(&self, name: &str) -> usize {
        self.index[name]
    }
    pub fn get_idx(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }
    pub fn iter(&self) -> Iter<T> {
        self.into_iter()
    }
    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.into_iter()
    }
}

impl<T: Named> Default for NameVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: Named> IntoIterator for &'a NameVec<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.vec.iter()
    }
}

impl<'a, T: Named> IntoIterator for &'a mut NameVec<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> IterMut<'a, T> {
        self.vec.iter_mut()
    }
}

impl<T: Named> Index<usize> for NameVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.vec[index]
    }
}

impl<T: Named> IndexMut<usize> for NameVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.vec[index]
    }
}

impl<T: Named> Index<&str> for NameVec<T> {
    type Output = T;
    fn index(&self, index: &str) -> &T {
        &self.vec[self.index[index]]
    }
}

impl<T: Named> IndexMut<&str> for NameVec<T> {
    fn index_mut(&mut self, index: &str) -> &mut T {
        &mut self.vec[self.index[index]]
    }
}
