//! A wrapper around [`Vec`] for vectors whose length is at most 3 elements.
//!
//! This is used in the implementation of [`frontier::Node`](crate::internal::frontier::Node)s to
//! store the lefthand siblings of the frontier's rightmost child, which must number at most 3
//! (because nodes must have at most 4 children total).

use std::marker::PhantomData;

use serde::{de::Visitor, Deserialize, Serialize};

/// A vector capable of storing at most 3 elements.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Derivative, Serialize)]
#[derivative(Debug = "transparent")]
pub struct Three<T> {
    elems: Vec<T>,
}

impl<T> Three<T> {
    /// Create a new `Three` with no elements, but the capacity for them pre-allocated.
    ///
    /// This technically allocates space for 4 elements, so that when [`Self::push`] overfills, it
    /// does not re-allocate.
    pub fn new() -> Self {
        Self {
            // This has capacity 4 to prevent re-allocating memory when we push to a filled `Three`
            // and thereby generate a [T; 4].
            elems: Vec::with_capacity(4),
        }
    }

    /// Push a new item into this [`Three`], or return exactly four items (including the pushed
    /// item) if the [`Three`] is already full.
    ///
    /// Note that this takes ownership of `self` and returns a new [`Three`] with the pushed item if
    /// successful.
    #[inline]
    pub fn push(mut self, item: T) -> Result<Self, [T; 4]> {
        // Push an element into the internal vec
        self.elems.push(item);
        // In debug mode, check that the size is never above 4
        debug_assert!(self.elems.len() <= 4);
        // If this makes the internal vec >= 4 elements, we're overfull
        match self.elems.try_into() {
            Ok(elems) => Err(elems),
            Err(elems) => Ok(Self { elems }),
        }
    }

    /// Get an enumeration of the elements of this [`Three`] by reference.
    pub fn elems(&self) -> Elems<T> {
        match self.elems.len() {
            0 => Elems::_0([]),
            1 => Elems::_1([&self.elems[0]]),
            2 => Elems::_2([&self.elems[0], &self.elems[1]]),
            3 => Elems::_3([&self.elems[0], &self.elems[1], &self.elems[2]]),
            _ => unreachable!("impossible for `Three` to contain more than 3 elements"),
        }
    }

    /// Get an enumeration of the elements of this [`Three`] by mutable reference.
    pub fn elems_mut(&mut self) -> ElemsMut<T> {
        match self.elems.as_mut_slice() {
            [] => ElemsMut::_0([]),
            [a] => ElemsMut::_1([a]),
            [a, b] => ElemsMut::_2([a, b]),
            [a, b, c] => ElemsMut::_3([a, b, c]),
            _ => unreachable!("impossible for `Three` to contain more than 3 elements"),
        }
    }

    /// Convert this [`Three`] into an enumeration of its elements.
    pub fn into_elems(self) -> IntoElems<T> {
        match self.elems.len() {
            0 => IntoElems::_0([]),
            1 => IntoElems::_1(if let Ok(elems) = self.elems.try_into() {
                elems
            } else {
                unreachable!("already checked the length of elements")
            }),
            2 => IntoElems::_2(if let Ok(elems) = self.elems.try_into() {
                elems
            } else {
                unreachable!("already checked the length of elements")
            }),
            3 => IntoElems::_3(if let Ok(elems) = self.elems.try_into() {
                elems
            } else {
                unreachable!("already checked the length of elements")
            }),
            _ => unreachable!("impossible for `Three` to contain more than 3 elements"),
        }
    }
}

impl<T> Default for Three<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// All the possible cases of the elements in a [`Three`], by reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elems<'a, T> {
    /// Zero elements.
    _0([&'a T; 0]),
    /// One element.
    _1([&'a T; 1]),
    /// Two elements.
    _2([&'a T; 2]),
    /// Three elements.
    _3([&'a T; 3]),
}

/// All the possible cases of the elements in a [`Three`], by mutable reference.
#[derive(Debug, PartialEq, Eq)]
pub enum ElemsMut<'a, T> {
    /// Zero elements.
    _0([&'a mut T; 0]),
    /// One element.
    _1([&'a mut T; 1]),
    /// Two elements.
    _2([&'a mut T; 2]),
    /// Three elements.
    _3([&'a mut T; 3]),
}

/// All the possible cases of the elements in a [`Three`], by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntoElems<T> {
    /// Zero elements.
    _0([T; 0]),
    /// One element.
    _1([T; 1]),
    /// Two elements.
    _2([T; 2]),
    /// Three elements.
    _3([T; 3]),
}

impl<T> From<IntoElems<T>> for Three<T> {
    fn from(elems: IntoElems<T>) -> Self {
        match elems {
            IntoElems::_0(elems) => Self {
                elems: elems.into(),
            },
            IntoElems::_1(elems) => Self {
                elems: elems.into(),
            },
            IntoElems::_2(elems) => Self {
                elems: elems.into(),
            },
            IntoElems::_3(elems) => Self {
                elems: elems.into(),
            },
        }
    }
}

impl<T> From<Three<T>> for IntoElems<T> {
    fn from(three: Three<T>) -> Self {
        three.into_elems()
    }
}

struct ThreeVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for ThreeVisitor<T> {
    type Value = Three<T>;

    fn expecting(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "a vector of at most 3 elements")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut elems = Vec::with_capacity(4);
        for _ in 0..=3 {
            if let Some(elem) = seq.next_element()? {
                elems.push(elem);
            } else {
                break;
            }
        }
        if seq.next_element::<T>()?.is_some() {
            return Err(serde::de::Error::invalid_length(3, &"at most 3 elements"));
        }
        Ok(Three { elems })
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Three<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ThreeVisitor(PhantomData))
    }
}
