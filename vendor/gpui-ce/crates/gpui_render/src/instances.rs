//! Batch addressing shared by every GPU backend.

use std::ops::Range;

/// A contiguous run of instances inside one pipeline's whole-frame instance buffer.
///
/// The render plan hands out `Range<usize>` slices of the scene arrays; backends draw them
/// with 32-bit GPU indices. Building this once at the boundary keeps every draw call
/// working from the same validated base and count, and makes the base impossible to lose
/// between "which instances" and "which draw arguments".
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstanceRange {
    first: u32,
    count: u32,
}

impl InstanceRange {
    /// A single instance at index zero, for draws that pull all their data per vertex.
    pub const SINGLE: Self = Self { first: 0, count: 1 };

    /// Converts a scene slice into GPU draw arguments.
    ///
    /// Returns `None` when the range is descending or does not fit a 32-bit instance index.
    pub fn new(range: Range<usize>) -> Option<Self> {
        let first = u32::try_from(range.start).ok()?;
        let end = u32::try_from(range.end).ok()?;
        let count = end.checked_sub(first)?;
        Some(Self { first, count })
    }

    /// The first instances of a freshly uploaded batch.
    pub fn from_start(count: usize) -> Option<Self> {
        Self::new(0..count)
    }

    /// Index of the first instance, relative to the bound buffer.
    pub fn first(self) -> u32 {
        self.first
    }

    /// Number of instances to draw.
    pub fn count(self) -> u32 {
        self.count
    }

    /// One past the last instance; never overflows because construction checked it.
    pub fn end(self) -> u32 {
        self.first + self.count
    }

    /// Whether the draw would touch no instances at all.
    pub fn is_empty(self) -> bool {
        self.count == 0
    }

    /// The equivalent `first..end` range for APIs that take one directly.
    pub fn as_range(self) -> Range<u32> {
        self.first..self.end()
    }
}

#[cfg(test)]
mod tests {
    use super::InstanceRange;

    #[test]
    fn converts_scene_slices_into_draw_arguments() {
        let range = InstanceRange::new(12..20).expect("small ranges convert");
        assert_eq!(range.first(), 12);
        assert_eq!(range.count(), 8);
        assert_eq!(range.end(), 20);
        assert_eq!(range.as_range(), 12..20);
        assert!(!range.is_empty());
        assert!(
            InstanceRange::new(3..3)
                .expect("empty ranges convert")
                .is_empty()
        );
    }

    #[test]
    fn rejects_ranges_the_gpu_cannot_address() {
        let (start, end) = (5, 4);
        assert!(InstanceRange::new(start..end).is_none());
        assert!(InstanceRange::new(0..u32::MAX as usize + 1).is_none());
    }
}
