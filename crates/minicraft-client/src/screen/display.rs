//! Lifecycle contract corresponding to Java Display.init/getParent/onExit.

/// Stable identity within one DisplayStack; identity is not a screen's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisplayId(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentPolicy {
    Inherit,
    /// TitleDisplay and PauseDisplay call super.init(null).
    Root,
}

/// Platform-independent lifecycle. Menu input and rendering remain separate
/// during migration; these hooks must not play sounds or mutate the stack.
pub trait Display {
    fn parent_policy(&self) -> ParentPolicy {
        ParentPolicy::Inherit
    }

    fn init(&mut self, _parent: Option<DisplayId>) {}

    fn on_exit(&mut self) {}
}
