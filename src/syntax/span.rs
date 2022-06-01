use std::fmt::{self, Debug, Display, Formatter};
use std::num::NonZeroU64;
use std::ops::Range;

use crate::syntax::SourceId;

/// A value with the span it corresponds to in the source code.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Spanned<T> {
    /// The spanned value.
    pub v: T,
    /// The location in source code of the value.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Create a new instance from a value and its span.
    pub fn new(v: T, span: impl Into<Span>) -> Self {
        Self { v, span: span.into() }
    }

    /// Convert from `&Spanned<T>` to `Spanned<&T>`
    pub fn as_ref(&self) -> Spanned<&T> {
        Spanned { v: &self.v, span: self.span }
    }

    /// Map the value using a function keeping the span.
    pub fn map<F, U>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned { v: f(self.v), span: self.span }
    }
}

impl<T: Debug> Debug for Spanned<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.v.fmt(f)
    }
}

/// A unique identifier for a syntax node.
///
/// This is used throughout the compiler to track which source section an error
/// or element stems from. Can be [mapped back](crate::source::SourceStore::range)
/// to a source id + byte range for user facing display.
///
/// Span ids are ordered in the tree to enable quickly finding the node with
/// some id:
/// - The id of a parent is always smaller than the ids of any of its children.
/// - The id of a node is always greater than any id in the subtrees of any left
///   sibling and smaller than any id in the subtrees of any right sibling.
///
/// The internal ids of spans stay mostly stable, even for nodes behind an
/// insertion. This is not true for simple ranges as they shift. Spans can be
/// used as inputs to memoized functions without hurting cache performance when
/// text is inserted somewhere in the document other than the end.
///
/// This type takes 8 bytes and is null-optimized (i.e. `Option<Span>` also
/// takes 8 bytes).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Span(NonZeroU64);

impl Span {
    // Number of bits for and minimum and maximum numbers assignable to spans.
    const BITS: usize = 48;
    const DETACHED: u64 = 1;
    const MIN: u64 = 2;
    const MAX: u64 = (1 << Self::BITS) - 1;

    /// The full range of numbers available to spans.
    pub const FULL: Range<u64> = Self::MIN .. Self::MAX + 1;

    /// Create a new span from a source id and a unique number.
    ///
    /// Panics if the `number` is not contained in `FULL`.
    pub const fn new(id: SourceId, number: u64) -> Self {
        assert!(number >= Self::MIN && number <= Self::MAX);
        let bits = ((id.into_raw() as u64) << Self::BITS) | number;
        Self(to_non_zero(bits))
    }

    /// A span that does not point into any source file.
    pub const fn detached() -> Self {
        Self(to_non_zero(Self::DETACHED))
    }

    /// The id of the source file the span points into.
    pub const fn source(self) -> SourceId {
        SourceId::from_raw((self.0.get() >> Self::BITS) as u16)
    }

    /// The unique number of the span within the source file.
    pub const fn number(self) -> u64 {
        self.0.get() & ((1 << Self::BITS) - 1)
    }
}

/// Convert to a non zero u64.
const fn to_non_zero(v: u64) -> NonZeroU64 {
    match NonZeroU64::new(v) {
        Some(v) => v,
        None => unreachable!(),
    }
}

/// Result of numbering a node within an interval.
pub type NumberingResult = Result<(), Unnumberable>;

/// Indicates that a node cannot be numbered within a given interval.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Unnumberable;

impl Display for Unnumberable {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("cannot number within this interval")
    }
}

impl std::error::Error for Unnumberable {}
