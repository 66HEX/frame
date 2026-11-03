use gpui::MAX_FILTER_GROUP_DEPTH;
use smallvec::SmallVec;

/// Active render target and the parents visible to backdrop filters.
pub struct TargetStack<Target> {
    current: Target,
    parents: SmallVec<[(Target, bool); MAX_FILTER_GROUP_DEPTH]>,
}

impl<Target> TargetStack<Target> {
    /// Starts rendering into the root target.
    pub fn new(root: Target) -> Self {
        Self {
            current: root,
            parents: SmallVec::new(),
        }
    }

    /// Returns the active render target.
    pub fn current(&self) -> &Target {
        &self.current
    }

    /// Enters an isolated group, optionally including its parent in backdrop sampling.
    pub fn enter(&mut self, next: Target, inherits_backdrop: bool) {
        self.parents.push((
            std::mem::replace(&mut self.current, next),
            inherits_backdrop,
        ));
    }

    /// Returns whether the active group samples its parent's backdrop.
    pub fn inherits_backdrop(&self) -> bool {
        self.parents.last().is_some_and(|(_, inherits)| *inherits)
    }

    /// Returns backdrop layers in paint order, stopping at a content-filter group.
    pub fn backdrop_layers(&self) -> impl Iterator<Item = &Target> {
        let start = self
            .parents
            .iter()
            .rposition(|(_, inherits)| !inherits)
            .map_or(0, |idx| idx + 1);

        self.parents[start..]
            .iter()
            .map(|(target, _)| target)
            .chain(std::iter::once(&self.current))
    }

    /// Leaves the current group and returns its target and restored parent.
    pub fn exit(&mut self) -> (Target, &Target) {
        let (parent, _) = self
            .parents
            .pop()
            .expect("render plan ended an isolated filter without beginning one");
        let filtered = std::mem::replace(&mut self.current, parent);

        (filtered, &self.current)
    }

    /// Checks that every isolated group has ended.
    pub fn assert_balanced(&self) {
        assert!(
            self.parents.is_empty(),
            "render plan left an isolated filter group open"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_groups_inherit_backdrops_until_a_content_filter_and_restore_parents() {
        let mut targets = TargetStack::new("root");
        targets.enter("opacity", true);
        assert!(targets.inherits_backdrop());
        assert_eq!(
            targets.backdrop_layers().copied().collect::<Vec<_>>(),
            ["root", "opacity"]
        );

        targets.enter("content", false);
        assert!(!targets.inherits_backdrop());
        assert_eq!(
            targets.backdrop_layers().copied().collect::<Vec<_>>(),
            ["content"]
        );

        targets.enter("nested", true);
        assert!(targets.inherits_backdrop());
        assert_eq!(
            targets.backdrop_layers().copied().collect::<Vec<_>>(),
            ["content", "nested"]
        );

        assert_eq!(targets.exit(), ("nested", &"content"));
        assert_eq!(targets.exit(), ("content", &"opacity"));
        assert!(targets.inherits_backdrop());
        assert_eq!(
            targets.backdrop_layers().copied().collect::<Vec<_>>(),
            ["root", "opacity"]
        );

        assert_eq!(targets.exit(), ("opacity", &"root"));
        assert!(!targets.inherits_backdrop());
        assert_eq!(
            targets.backdrop_layers().copied().collect::<Vec<_>>(),
            ["root"]
        );
        targets.assert_balanced();
    }
}
