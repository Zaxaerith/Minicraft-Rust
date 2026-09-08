//! Java Game.displayQuery and Updater's deferred Display lifecycle.
//!
//! This is deliberately not an eager Vec::push/pop stack. Requested displays
//! become current at the next update boundary. Parents are assigned only on
//! first init; resuming a parent exits the child without initializing it again.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::screen::display::{Display, DisplayId, ParentPolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    Init,
    Exit,
}

#[derive(Debug, Clone)]
struct Node<D> {
    value: D,
    initialized: bool,
    parent: Option<DisplayId>,
}

#[derive(Debug, Clone)]
pub struct DisplayStack<D> {
    nodes: BTreeMap<DisplayId, Node<D>>,
    query: VecDeque<DisplayId>,
    current: Option<DisplayId>,
    next_id: u64,
}

impl<D> Default for DisplayStack<D> {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
            query: VecDeque::new(),
            current: None,
            next_id: 0,
        }
    }
}

impl<D: Display> DisplayStack<D> {
    /// Game.setDisplay(non-null): enqueue, without calling init yet.
    pub fn open(&mut self, value: D) -> DisplayId {
        let id = DisplayId(self.next_id);
        self.next_id = self.next_id.checked_add(1).expect("display ID exhausted");
        self.nodes.insert(
            id,
            Node {
                value,
                initialized: false,
                parent: None,
            },
        );
        self.query.push_back(id);
        id
    }

    /// Game.setDisplay(null). This does not synthesize exitDisplay's craft sound.
    pub fn clear(&mut self) {
        self.query.clear();
    }

    /// Game.exitDisplay(depth). True means Java would request ONE craft sound,
    /// regardless of depth. The caller owns audio delivery (never the stack).
    pub fn exit(&mut self, depth: usize) -> bool {
        if depth == 0 || self.query.is_empty() {
            return false;
        }
        for _ in 0..depth {
            let Some(id) = self.query.back() else { break };
            if let Some(parent) = self.nodes[id].parent {
                self.query.push_back(parent);
            } else {
                self.query.clear();
            }
        }
        true
    }

    pub fn current_id(&self) -> Option<DisplayId> {
        self.current
    }

    /// Game.getDisplay(), which can differ from the display currently rendered.
    pub fn requested_id(&self) -> Option<DisplayId> {
        self.query.back().copied()
    }

    pub fn current(&self) -> Option<&D> {
        self.current.map(|id| &self.nodes[&id].value)
    }

    pub fn current_mut(&mut self) -> Option<&mut D> {
        self.current
            .map(|id| &mut self.nodes.get_mut(&id).unwrap().value)
    }

    pub fn requested(&self) -> Option<&D> {
        self.requested_id().map(|id| &self.nodes[&id].value)
    }

    pub fn requested_mut(&mut self) -> Option<&mut D> {
        self.requested_id()
            .map(|id| &mut self.nodes.get_mut(&id).unwrap().value)
    }

    pub fn parent_id(&self, id: DisplayId) -> Option<DisplayId> {
        self.nodes.get(&id).and_then(|node| node.parent)
    }

    pub fn get(&self, id: DisplayId) -> Option<&D> {
        self.nodes.get(&id).map(|node| &node.value)
    }

    /// Updater.tick's lifecycle prelude, in the original operation order.
    pub fn update(&mut self) {
        self.update_observed(|_, _| {});
    }

    /// Observe actual hooks in order, before unreachable nodes are dropped.
    /// This includes intermediate roots that do not survive in the final tree.
    pub fn update_observed(&mut self, mut observer: impl FnMut(LifecycleEvent, &D)) {
        let first = self.query.front().copied();
        if self.current != first
            && let Some(first) = first
        {
            let previous = self.current;
            self.current = Some(first);
            self.init(first, previous);
            observer(LifecycleEvent::Init, &self.nodes[&first].value);
        }
        if self.current.is_some() && self.query.is_empty() {
            let id = self.current.take().unwrap();
            let node = self.nodes.get_mut(&id).unwrap();
            node.value.on_exit();
            observer(LifecycleEvent::Exit, &node.value);
        }
        while self.query.len() > 1 {
            let previous = self.query.pop_front().unwrap();
            let next = *self.query.front().unwrap();
            self.current = Some(next);
            if self.nodes[&previous].parent == Some(next) {
                self.nodes.get_mut(&previous).unwrap().value.on_exit();
                observer(LifecycleEvent::Exit, &self.nodes[&previous].value);
            } else {
                self.init(next, Some(previous));
                observer(LifecycleEvent::Init, &self.nodes[&next].value);
            }
        }
        // Rust ownership replacement for Java GC. Dropping an unreachable node
        // is NOT onExit: Java does not unwind every ancestor on setDisplay(null).
        let mut reachable = BTreeSet::new();
        let mut pending: Vec<_> = self.query.iter().copied().chain(self.current).collect();
        while let Some(id) = pending.pop() {
            if reachable.insert(id)
                && let Some(parent) = self.nodes[&id].parent
            {
                pending.push(parent);
            }
        }
        self.nodes.retain(|id, _| reachable.contains(id));
    }

    fn init(&mut self, id: DisplayId, previous: Option<DisplayId>) {
        let node = self.nodes.get_mut(&id).unwrap();
        let parent = match node.value.parent_policy() {
            ParentPolicy::Inherit => previous,
            ParentPolicy::Root => None,
        };
        if !node.initialized {
            node.initialized = true;
            node.parent = parent;
        }
        node.value.init(parent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone)]
    struct Probe {
        name: &'static str,
        root: bool,
        log: Rc<RefCell<Vec<String>>>,
    }

    impl Display for Probe {
        fn parent_policy(&self) -> ParentPolicy {
            if self.root {
                ParentPolicy::Root
            } else {
                ParentPolicy::Inherit
            }
        }
        fn init(&mut self, parent: Option<DisplayId>) {
            self.log
                .borrow_mut()
                .push(format!("init:{}:{parent:?}", self.name));
        }
        fn on_exit(&mut self) {
            self.log.borrow_mut().push(format!("exit:{}", self.name));
        }
    }

    fn probe(name: &'static str, root: bool, log: &Rc<RefCell<Vec<String>>>) -> Probe {
        Probe {
            name,
            root,
            log: log.clone(),
        }
    }

    #[test]
    fn observer_sees_discarded_roots_and_exit_in_hook_order() {
        let log = Rc::default();
        let mut stack = DisplayStack::default();
        let first = stack.open(probe("first", true, &log));
        stack.open(probe("middle", true, &log));
        stack.open(probe("last", true, &log));
        let mut events = Vec::new();
        stack.update_observed(|event, value| events.push((event, value.name)));
        assert_eq!(
            events,
            vec![
                (LifecycleEvent::Init, "first"),
                (LifecycleEvent::Init, "middle"),
                (LifecycleEvent::Init, "last"),
            ]
        );
        assert!(stack.get(first).is_none());
        stack.clear();
        stack.update_observed(|event, value| events.push((event, value.name)));
        assert_eq!(events.last(), Some(&(LifecycleEvent::Exit, "last")));
        assert_eq!(events.len(), 4);
        stack.update_observed(|event, value| events.push((event, value.name)));
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn deferred_open_and_parent_resume_keep_identity_without_reinit() {
        let log = Rc::default();
        let mut stack = DisplayStack::default();
        let title = stack.open(probe("title", true, &log));
        assert!(stack.current().is_none());
        stack.update();
        let options = stack.open(probe("options", false, &log));
        assert_eq!(stack.current_id(), Some(title));
        assert_eq!(stack.requested_id(), Some(options));
        stack.update();
        assert_eq!(stack.parent_id(options), Some(title));
        assert!(stack.exit(1));
        assert_eq!(stack.current_id(), Some(options));
        stack.update();
        assert_eq!(stack.current_id(), Some(title));
        assert_eq!(log.borrow().len(), 3);
        assert_eq!(log.borrow()[2], "exit:options");
        assert!(stack.get(options).is_none());
    }

    #[test]
    fn queued_children_initialize_in_order_and_multilevel_exit_is_deferred() {
        let log = Rc::default();
        let mut stack = DisplayStack::default();
        let title = stack.open(probe("title", true, &log));
        stack.open(probe("options", false, &log));
        stack.open(probe("language", false, &log));
        stack.update();
        assert_eq!(log.borrow().len(), 3);
        assert!(stack.exit(2));
        assert_eq!(stack.requested_id(), Some(title));
        stack.update();
        assert_eq!(&log.borrow()[3..], &["exit:language", "exit:options"]);
        assert_eq!(stack.nodes.len(), 1);
    }

    #[test]
    fn title_and_pause_ignore_previous_display_as_parent() {
        for root in ["title", "pause"] {
            let log = Rc::default();
            let mut stack = DisplayStack::default();
            let old = stack.open(probe("old", false, &log));
            stack.update();
            let id = stack.open(probe(root, true, &log));
            stack.update();
            assert_eq!(stack.parent_id(id), None);
            assert!(stack.get(old).is_none());
            assert!(stack.exit(1));
            stack.update();
            assert!(stack.current().is_none());
            assert_eq!(log.borrow().len(), 3);
        }
    }

    #[test]
    fn clear_exits_only_current_not_every_ancestor_or_uninitialized_child() {
        let log = Rc::default();
        let mut stack = DisplayStack::default();
        stack.open(probe("pause", true, &log));
        stack.update();
        stack.open(probe("popup", false, &log));
        stack.clear();
        assert!(stack.requested().is_none());
        assert!(stack.current().is_some());
        stack.update();
        assert_eq!(&*log.borrow(), &["init:pause:None", "exit:pause"]);
        assert!(stack.nodes.is_empty());
        assert!(!stack.exit(1));
    }

    #[test]
    fn exiting_uninitialized_child_clears_query_like_java_and_zero_is_noop() {
        let log = Rc::default();
        let mut stack = DisplayStack::default();
        let title = stack.open(probe("title", true, &log));
        stack.update();
        assert!(!stack.exit(0));
        assert_eq!(stack.requested_id(), Some(title));
        stack.open(probe("options", false, &log));
        assert!(stack.exit(1));
        stack.update();
        assert!(stack.current().is_none());
        assert_eq!(&*log.borrow(), &["init:title:None", "exit:title"]);
    }
}
