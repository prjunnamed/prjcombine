use bincode::{Decode, Encode};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum DirH {
    W,
    E,
}

impl core::ops::Not for DirH {
    type Output = DirH;
    fn not(self) -> DirH {
        match self {
            DirH::W => DirH::E,
            DirH::E => DirH::W,
        }
    }
}

impl std::fmt::Display for DirH {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DirH::W => "W",
                DirH::E => "E",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum DirV {
    S,
    N,
}

impl core::ops::Not for DirV {
    type Output = DirV;
    fn not(self) -> DirV {
        match self {
            DirV::S => DirV::N,
            DirV::N => DirV::S,
        }
    }
}

impl std::fmt::Display for DirV {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DirV::S => "S",
                DirV::N => "N",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub enum Dir {
    H(DirH),
    V(DirV),
}

impl Dir {
    pub const DIRS: [Dir; 4] = [Dir::W, Dir::E, Dir::S, Dir::N];

    pub const W: Dir = Dir::H(DirH::W);
    pub const E: Dir = Dir::H(DirH::E);
    pub const S: Dir = Dir::V(DirV::S);
    pub const N: Dir = Dir::V(DirV::N);
}

impl core::ops::Not for Dir {
    type Output = Dir;
    fn not(self) -> Dir {
        match self {
            Dir::W => Dir::E,
            Dir::E => Dir::W,
            Dir::S => Dir::N,
            Dir::N => Dir::S,
        }
    }
}

impl std::fmt::Display for Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Dir::W => "W",
                Dir::E => "E",
                Dir::S => "S",
                Dir::N => "N",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct DirHV {
    pub h: DirH,
    pub v: DirV,
}

impl DirHV {
    pub const DIRS: [DirHV; 4] = [DirHV::SW, DirHV::SE, DirHV::NW, DirHV::NE];

    pub const SW: DirHV = DirHV {
        h: DirH::W,
        v: DirV::S,
    };
    pub const SE: DirHV = DirHV {
        h: DirH::E,
        v: DirV::S,
    };
    pub const NW: DirHV = DirHV {
        h: DirH::W,
        v: DirV::N,
    };
    pub const NE: DirHV = DirHV {
        h: DirH::E,
        v: DirV::N,
    };
}

impl std::fmt::Display for DirHV {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                DirHV::SW => "SW",
                DirHV::SE => "SE",
                DirHV::NW => "NW",
                DirHV::NE => "NE",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode, Default)]
pub struct DirMap<T> {
    w: T,
    e: T,
    s: T,
    n: T,
}

impl<T> DirMap<T> {
    pub fn from_fn(mut f: impl FnMut(Dir) -> T) -> Self {
        Self {
            w: f(Dir::W),
            e: f(Dir::E),
            s: f(Dir::S),
            n: f(Dir::N),
        }
    }

    pub fn iter(&self) -> DirMapIter<&T> {
        DirMapIter {
            inner: DirPartMap {
                inner: DirMap::from_fn(|dir| Some(&self[dir])),
            },
        }
    }

    pub fn iter_mut(&mut self) -> DirMapIter<&mut T> {
        DirMapIter {
            inner: DirPartMap {
                inner: DirMap {
                    w: Some(&mut self.w),
                    e: Some(&mut self.e),
                    s: Some(&mut self.s),
                    n: Some(&mut self.n),
                },
            },
        }
    }

    pub fn values(&self) -> DirMapValues<&T> {
        DirMapValues {
            inner: DirPartMap {
                inner: DirMap::from_fn(|dir| Some(&self[dir])),
            },
        }
    }

    pub fn values_mut(&mut self) -> DirMapValues<&mut T> {
        DirMapValues {
            inner: self.iter_mut().inner,
        }
    }
}

impl<T> std::ops::Index<Dir> for DirMap<T> {
    type Output = T;

    fn index(&self, index: Dir) -> &Self::Output {
        match index {
            Dir::W => &self.w,
            Dir::E => &self.e,
            Dir::S => &self.s,
            Dir::N => &self.n,
        }
    }
}

impl<T> std::ops::IndexMut<Dir> for DirMap<T> {
    fn index_mut(&mut self, index: Dir) -> &mut Self::Output {
        match index {
            Dir::W => &mut self.w,
            Dir::E => &mut self.e,
            Dir::S => &mut self.s,
            Dir::N => &mut self.n,
        }
    }
}

impl<T> IntoIterator for DirMap<T> {
    type Item = (Dir, T);

    type IntoIter = DirMapIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        DirMapIter {
            inner: DirPartMap {
                inner: DirMap {
                    w: Some(self.w),
                    e: Some(self.e),
                    s: Some(self.s),
                    n: Some(self.n),
                },
            },
        }
    }
}

impl<'a, T> IntoIterator for &'a DirMap<T> {
    type Item = (Dir, &'a T);

    type IntoIter = DirMapIter<&'a T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct DirPartMap<T> {
    inner: DirMap<Option<T>>,
}

impl<T> DirPartMap<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: Dir) -> Option<&T> {
        self.inner[key].as_ref()
    }

    pub fn get_mut(&mut self, key: Dir) -> Option<&mut T> {
        self.inner[key].as_mut()
    }

    pub fn contains_key(&self, key: Dir) -> bool {
        self.inner[key].is_some()
    }

    pub fn insert(&mut self, key: Dir, value: T) -> Option<T> {
        self.inner[key].replace(value)
    }

    pub fn remove(&mut self, key: Dir) -> Option<T> {
        self.inner[key].take()
    }

    pub fn iter(&self) -> DirMapIter<&T> {
        DirMapIter {
            inner: DirPartMap {
                inner: DirMap::from_fn(|dir| self.get(dir)),
            },
        }
    }

    pub fn values(&self) -> DirMapValues<&T> {
        DirMapValues {
            inner: DirPartMap {
                inner: DirMap::from_fn(|dir| self.get(dir)),
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.values().all(|x| x.is_none())
    }
}

impl<T> Default for DirPartMap<T> {
    fn default() -> Self {
        Self {
            inner: DirMap::from_fn(|_| None),
        }
    }
}

impl<T> std::ops::Index<Dir> for DirPartMap<T> {
    type Output = T;

    fn index(&self, index: Dir) -> &Self::Output {
        self.inner[index].as_ref().unwrap()
    }
}

impl<T> IntoIterator for DirPartMap<T> {
    type Item = (Dir, T);

    type IntoIter = DirMapIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        DirMapIter { inner: self }
    }
}

impl<'a, T> IntoIterator for &'a DirPartMap<T> {
    type Item = (Dir, &'a T);

    type IntoIter = DirMapIter<&'a T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct DirMapIter<T> {
    inner: DirPartMap<T>,
}

impl<T> Iterator for DirMapIter<T> {
    type Item = (Dir, T);

    fn next(&mut self) -> Option<Self::Item> {
        for key in Dir::DIRS {
            if let Some(val) = self.inner.remove(key) {
                return Some((key, val));
            }
        }
        None
    }
}

pub struct DirMapValues<T> {
    inner: DirPartMap<T>,
}

impl<T> Iterator for DirMapValues<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        for key in Dir::DIRS {
            if let Some(val) = self.inner.remove(key) {
                return Some(val);
            }
        }
        None
    }
}
