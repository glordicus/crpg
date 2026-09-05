//! Runtime-only string handles and their interning tables.
//!
//! [`StatId`] and [`TagId`] must never implement `Serialize`, `Deserialize`, or
//! `Display`. They are load-order-dependent handles whose meaning exists only
//! within the [`Interners`] that issued them. Persist the resolved string
//! instead; see ADR-0006 Decision 4.

use indexmap::IndexSet;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

/// A runtime-only handle into the stat namespace of an [`Interners`].
///
/// This value is meaningless without the interner that issued it and is
/// deliberately not serializable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatId(u32);

impl StatId {
    /// Returns the dense runtime index assigned to this stat.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// A runtime-only handle into the tag namespace of an [`Interners`].
///
/// This value is meaningless without the interner that issued it and is
/// deliberately not serializable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TagId(u32);

impl TagId {
    /// Returns the dense runtime index assigned to this tag.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Interns strings to dense `u32` handles in first-intern order.
///
/// Serialization is an ordered sequence of strings. Numeric handles remain
/// meaningful only against the particular table that issued them.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct Interner {
    strings: IndexSet<String>,
}

impl PartialEq for Interner {
    fn eq(&self, other: &Self) -> bool {
        self.strings.iter().eq(other.strings.iter())
    }
}

impl Eq for Interner {}

impl<'de> Deserialize<'de> for Interner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = Vec::<String>::deserialize(deserializer)?;
        let mut strings = IndexSet::with_capacity(values.len());
        for value in values {
            if !strings.insert(value) {
                return Err(D::Error::custom("duplicate string in interner"));
            }
        }
        Ok(Self { strings })
    }
}

impl Interner {
    /// Creates an empty interner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the existing handle for `s`, or assigns the next dense handle.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(index) = self.strings.get_index_of(s) {
            return handle_from_index(index);
        }

        let handle = handle_from_index(self.strings.len());
        self.strings.insert(s.to_owned());
        handle
    }

    /// Returns the handle assigned to `s`, without modifying the interner.
    #[must_use]
    pub fn get(&self, s: &str) -> Option<u32> {
        self.strings.get_index_of(s).map(handle_from_index)
    }

    /// Resolves a handle to its interned string.
    #[must_use]
    pub fn resolve(&self, id: u32) -> Option<&str> {
        self.strings.get_index(id as usize).map(String::as_str)
    }

    /// Returns the number of distinct interned strings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns whether no strings have been interned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Iterates over handles and strings in ascending handle order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &str)> {
        self.strings
            .iter()
            .enumerate()
            .map(|(index, string)| (handle_from_index(index), string.as_str()))
    }
}

/// Owns independent stat and tag interning namespaces.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interners {
    stats: Interner,
    tags: Interner,
}

impl Interners {
    /// Creates empty stat and tag namespaces.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the stat handle for `s`, interning it when absent.
    pub fn intern_stat(&mut self, s: &str) -> StatId {
        StatId(self.stats.intern(s))
    }

    /// Returns the tag handle for `s`, interning it when absent.
    pub fn intern_tag(&mut self, s: &str) -> TagId {
        TagId(self.tags.intern(s))
    }

    /// Returns the stat handle assigned to `s`.
    #[must_use]
    pub fn stat(&self, s: &str) -> Option<StatId> {
        self.stats.get(s).map(StatId)
    }

    /// Returns the tag handle assigned to `s`.
    #[must_use]
    pub fn tag(&self, s: &str) -> Option<TagId> {
        self.tags.get(s).map(TagId)
    }

    /// Resolves a stat handle to its interned string.
    #[must_use]
    pub fn resolve_stat(&self, id: StatId) -> Option<&str> {
        self.stats.resolve(id.0)
    }

    /// Resolves a tag handle to its interned string.
    #[must_use]
    pub fn resolve_tag(&self, id: TagId) -> Option<&str> {
        self.tags.resolve(id.0)
    }

    /// Returns the stat interner.
    #[must_use]
    pub const fn stats(&self) -> &Interner {
        &self.stats
    }

    /// Returns the tag interner.
    #[must_use]
    pub const fn tags(&self) -> &Interner {
        &self.tags
    }
}

fn handle_from_index(index: usize) -> u32 {
    u32::try_from(index).expect("interner exhausted u32 handles")
}
