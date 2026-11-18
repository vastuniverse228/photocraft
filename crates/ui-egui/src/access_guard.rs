//! Windows closes PhotoCraft when a menu's accessibility node names a popup that this frame's
//! tree does not contain (#15597).
//!
//! `accesskit_windows` asks every node for the nodes it controls while it applies a tree update.
//! `accesskit_consumer`'s `Node::controls` unwraps that lookup, so one id with no node takes the
//! process down. The same consumer panics when the update's focus is not in its node list. The
//! craft UI framework can report a menu popup as open on the frame before the popup's node is
//! emitted (Image, Edit and Layer do this on Windows once UI Automation is connected).
//!
//! [`sanitize`] drops those dangling ids, and points a missing focus at the tree root, before the
//! update leaves egui. A control that does exist is kept, so a menu that opened still exposes its
//! popup.

use std::collections::HashSet;

use egui::accesskit::{NodeId, TreeUpdate};
use egui::{Context, FullOutput};

/// Drop relation targets the update does not contain, and keep the focus inside the node list.
pub fn sanitize(update: &mut TreeUpdate) {
    let present: HashSet<NodeId> = update.nodes.iter().map(|(id, _)| *id).collect();
    if !present.contains(&update.focus) {
        if let Some(root) = update.tree.as_ref().map(|tree| tree.root) {
            update.focus = root;
        } else if let Some((id, _)) = update.nodes.first() {
            update.focus = *id;
        }
    }
    for (_, node) in &mut update.nodes {
        let controls = node.controls().to_vec();
        if controls.iter().all(|id| present.contains(id)) {
            continue;
        }
        let kept: Vec<NodeId> = controls.into_iter().filter(|id| present.contains(id)).collect();
        if kept.is_empty() {
            node.clear_controls();
        } else {
            node.set_controls(kept);
        }
    }
}

/// Runs [`sanitize`] on each frame's accessibility update.
struct AccessGuard;

impl egui::Plugin for AccessGuard {
    fn debug_name(&self) -> &'static str {
        "photocraft.access_guard"
    }

    fn output_hook(&mut self, _ctx: &Context, output: &mut FullOutput) {
        if let Some(update) = output.platform_output.accesskit_update.as_mut() {
            sanitize(update);
        }
    }
}

/// Install the guard on `ctx`. A second call leaves the one already installed.
pub fn install(ctx: &Context) {
    if ctx.plugin_opt::<AccessGuard>().is_none() {
        ctx.add_plugin(AccessGuard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::{Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};

    const ROOT: NodeId = NodeId(1);
    const POPUP: NodeId = NodeId(2);
    const MISSING: NodeId = NodeId(99);

    fn update(controls: &[NodeId], focus: NodeId) -> TreeUpdate {
        let mut nodes = Vec::new();
        let mut children = Vec::new();
        if controls.contains(&POPUP) {
            children.push(POPUP);
            nodes.push((POPUP, Node::new(Role::Menu)));
        }
        let mut root = Node::new(Role::Window);
        root.set_children(children);
        root.set_controls(controls.to_vec());
        nodes.insert(0, (ROOT, root));
        TreeUpdate { nodes, tree: Some(TreeInfo::new(ROOT)), tree_id: TreeId::ROOT, focus }
    }

    /// How many nodes the root controls, as the Windows adapter asks. Panics when one id is not in the tree.
    fn controlled(update: TreeUpdate) -> usize {
        let tree = accesskit_consumer::Tree::new(update, true);
        tree.state().root().controls().count()
    }

    #[test]
    fn a_controls_id_the_tree_does_not_contain_panics_until_it_is_dropped() {
        let raw = update(&[POPUP, MISSING], ROOT);
        let panicked = std::panic::catch_unwind(|| controlled(raw)).is_err();
        assert!(panicked, "the consumer unwraps a controls id that has no node");

        let mut guarded = update(&[POPUP, MISSING], ROOT);
        sanitize(&mut guarded);
        assert_eq!(controlled(guarded), 1, "the popup that is in the tree stays; the missing id does not");
    }

    #[test]
    fn a_focus_the_tree_does_not_contain_moves_to_the_root() {
        let raw = update(&[], MISSING);
        assert!(std::panic::catch_unwind(|| controlled(raw)).is_err(), "a focus outside the node list panics the consumer");

        let mut guarded = update(&[], MISSING);
        sanitize(&mut guarded);
        assert_eq!(guarded.focus, ROOT);
        assert_eq!(controlled(guarded), 0);
    }

    #[test]
    fn a_complete_update_is_left_unchanged() {
        let mut update = update(&[POPUP], POPUP);
        let before = update.clone();
        sanitize(&mut update);
        assert_eq!(update.focus, before.focus);
        assert_eq!(update.nodes[0].1.controls(), before.nodes[0].1.controls());
    }
}
