use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    iter::FromIterator,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

pub mod map;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawId(u32);

impl From<RawId> for u32 {
    fn from(raw: RawId) -> u32 {
        raw.0
    }
}

impl From<u32> for RawId {
    fn from(id: u32) -> RawId {
        RawId(id)
    }
}

impl fmt::Debug for RawId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for RawId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Idx<T> {
    raw: RawId,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Idx<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Idx<T> {}

impl<T> PartialEq for Idx<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for Idx<T> {}

impl<T> Hash for Idx<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T> PartialOrd for Idx<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Idx<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<T> fmt::Debug for Idx<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut type_name = std::any::type_name::<T>();
        if let Some(idx) = type_name.rfind(':') {
            type_name = &type_name[idx + 1..];
        }
        write!(f, "Idx::<{}>({})", type_name, self.raw)
    }
}

impl<T> Idx<T> {
    pub fn from_raw(raw: RawId) -> Self {
        Idx {
            raw,
            _ty: PhantomData,
        }
    }
    pub fn into_raw(self) -> RawId {
        self.raw
    }
}

/// An `Arena<T>` holds a collection of `T`s but allocates persistent ID's that
/// are used to refer to an element in the arena. When adding an item to an
/// `Arena` it returns an `Idx<T>` that is only valid for the `Arena` that
/// allocated the `Idx`. Its only possible to add items to an `Arena`.
#[derive(Clone, PartialEq, Eq)]
pub struct Arena<T> {
    data: Vec<T>,
}

impl<T: fmt::Debug> fmt::Debug for Arena<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Arena")
            .field("len", &self.len())
            .field("data", &self.data)
            .finish()
    }
}

impl<T> Arena<T> {
    /// Returns the number of elements in the arena
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the arena does not contain any element
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Stores `value` in the arena and returns the associated Id.
    pub fn alloc(&mut self, value: T) -> Idx<T> {
        let id = RawId(self.data.len() as u32);
        self.data.push(value);
        Idx::from_raw(id)
    }

    /// Iterate over the elements in the arena
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (Idx<T>, &T)> + DoubleEndedIterator {
        self.data
            .iter()
            .enumerate()
            .map(|(idx, value)| (Idx::from_raw(RawId(idx as u32)), value))
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Arena<T> {
        Arena { data: Vec::new() }
    }
}

impl<T> Index<Idx<T>> for Arena<T> {
    type Output = T;
    fn index(&self, idx: Idx<T>) -> &T {
        let idx = idx.into_raw().0 as usize;
        &self.data[idx]
    }
}

impl<T> IndexMut<Idx<T>> for Arena<T> {
    fn index_mut(&mut self, idx: Idx<T>) -> &mut T {
        let idx = idx.into_raw().0 as usize;
        &mut self.data[idx]
    }
}

impl<T> FromIterator<T> for Arena<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Arena {
            data: Vec::from_iter(iter),
        }
    }
}
