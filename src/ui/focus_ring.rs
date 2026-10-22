//! Subtree focus with host-controlled proposals and screen-position ordering.
//!
//! This ring does not use ink-rs focus. Mixing it with `Scope::use_focus`
//! handles Tab twice. The host enables one ring or its external prompt at a time.

use std::{cell::RefCell, rc::Rc};

use ink_rs::{Callback, Element, NodeRef, Props, Scope, State, measure_element};

/// Ownership of the ring's focused key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RingFocus {
    /// Applies focus proposals inside the ring.
    #[default]
    Managed,
    /// Holds this key until the host supplies another value.
    Key(String),
    /// Holds no key until the host supplies another value.
    None,
}

/// The keys a focused member uses for itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Claims {
    /// Leaves arrows and text available to the ring.
    #[default]
    None,
    /// Keeps arrows inside the member.
    Arrows,
    /// Keeps arrows and suppresses other members' text hotkeys.
    Text,
}

/// A stop in the nearest ring. Keys must be unique within that ring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RingItem {
    /// The host's key, or a generated per-mount key with the reserved `eink:ring:` prefix.
    pub key: Option<String>,
    /// The outer View's committed layout.
    pub node: NodeRef,
    /// Requests focus when an active ring has no focused key.
    pub auto_focus: bool,
    /// Keys retained by this member while focused.
    pub claims: Claims,
}

/// A subtree whose members share a focus ring.
///
/// Mixing this ring with ink-rs `use_focus` handles Tab twice.
#[derive(Clone, PartialEq, Props)]
#[props(component = focus_ring)]
pub struct FocusRing {
    /// The subtree containing the ring's members.
    pub children: Vec<Element>,
    /// Focus ownership. Defaults to managed inside the ring.
    pub focused: RingFocus,
    /// Receives every proposed focus move, including removal and auto-focus.
    pub on_focus: Option<Callback<Option<String>>>,
    /// Runs when Escape is pressed while the ring holds the keys.
    pub on_escape: Option<Callback>,
    /// Whether this ring holds the keys. Defaults to true.
    pub is_active: bool,
    /// Whether unclaimed arrows walk the ring. Defaults to false.
    pub arrows: bool,
}

impl Default for FocusRing {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            focused: RingFocus::Managed,
            on_focus: None,
            on_escape: None,
            is_active: true,
            arrows: false,
        }
    }
}

#[derive(Clone)]
struct Member {
    item: RingItem,
    key: String,
    position: (i32, i32),
}

#[derive(Default)]
struct Members {
    entries: Vec<Member>,
    removed: Vec<Member>,
    next_key: u64,
    auto_focus_pending: bool,
    was_active: bool,
}

impl Members {
    fn measure(&mut self) {
        for member in &mut self.entries {
            let metrics = measure_element(member.item.node);
            member.position = (metrics.y, metrics.x);
        }
        self.entries.sort_by_key(|member| member.position);
    }

    fn claims(&self, key: Option<&str>) -> Claims {
        self.entries
            .iter()
            .find(|member| Some(member.key.as_str()) == key)
            .map_or(Claims::None, |member| member.item.claims)
    }
}

#[derive(Clone)]
struct Registry {
    members: Rc<RefCell<Members>>,
    revision: State<u64>,
}

impl PartialEq for Registry {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.members, &other.members)
    }
}

#[derive(Clone, PartialEq)]
struct RingContext {
    registry: Registry,
    focused: Option<String>,
    is_active: bool,
    claims: Claims,
}

struct Registration {
    registry: Registry,
    node: NodeRef,
}

impl Drop for Registration {
    fn drop(&mut self) {
        let mut members = self.registry.members.borrow_mut();
        if let Some(index) = members
            .entries
            .iter()
            .position(|m| m.item.node == self.node)
        {
            let removed = members.entries.remove(index);
            members.removed.push(removed);
        }
        self.registry.revision.update(|revision| *revision += 1);
    }
}

/// Joins the nearest ring and returns whether this member holds the keys.
/// Outside a ring it returns true. The caller gates disabled input itself.
///
/// Host components can register stops without using a built-in control:
/// ```
/// use eink_rs::ui::{Claims, RingItem, use_ring_focus};
/// use ink_rs::{Element, Scope, Text, View};
/// fn close_mark(cx: &mut Scope, _: &()) -> Element {
///     let node = cx.use_node_ref();
///     let focused = use_ring_focus(cx, RingItem {
///         key: Some("close".into()), node, auto_focus: false, claims: Claims::None,
///     });
///     let app = cx.use_app();
///     cx.use_input(focused, move |_, key| {
///         if key.enter { app.exit(); }
///     });
///     View::new().node_ref(node).child(Text::new("x").inverse(focused)).into()
/// }
/// ```
pub fn use_ring_focus(cx: &mut Scope, item: RingItem) -> bool {
    let context = cx.use_context::<RingContext>();
    let generated = cx.use_state(|| {
        context.as_ref().map(|context| {
            let mut members = context.registry.members.borrow_mut();
            let key = format!("eink:ring:{}", members.next_key);
            members.next_key += 1;
            key
        })
    });
    let key = item.key.clone().or_else(|| generated.get());
    let registry = context.as_ref().map(|context| context.registry.clone());
    let registration_key = key.clone();
    cx.use_layout_effect((registry.clone(), item.clone()), move || {
        registry.map(|registry| {
            let mut members = registry.members.borrow_mut();
            let metrics = measure_element(item.node);
            members.auto_focus_pending |= item.auto_focus;
            members.entries.push(Member {
                key: registration_key.expect("ring member key"),
                position: (metrics.y, metrics.x),
                item: item.clone(),
            });
            registry.revision.update(|revision| *revision += 1);
            drop(members);
            Registration {
                registry,
                node: item.node,
            }
        })
    });
    context.is_none_or(|context| context.is_active && context.focused == key)
}

pub(super) fn use_ring_hotkeys(cx: &mut Scope) -> bool {
    cx.use_context::<RingContext>()
        .is_none_or(|context| context.is_active && context.claims != Claims::Text)
}

fn propose(
    managed: State<Option<String>>,
    control: &RingFocus,
    callback: &Option<Callback<Option<String>>>,
    target: Option<String>,
) {
    if *control == RingFocus::Managed {
        managed.set(target.clone());
    }
    if let Some(callback) = callback {
        callback.call(target);
    }
}

fn focus_ring(cx: &mut Scope, props: &FocusRing) -> Element {
    let managed = cx.use_state(|| None::<String>);
    let revision = cx.use_state(|| 0_u64);
    let members = cx
        .use_state(|| Rc::new(RefCell::new(Members::default())))
        .get();
    // Capture the preceding layout before reconciliation detaches removed refs.
    members.borrow_mut().measure();
    let focused = match &props.focused {
        RingFocus::Managed => managed.get(),
        RingFocus::Key(key) => Some(key.clone()),
        RingFocus::None => None,
    };
    let context = RingContext {
        registry: Registry {
            members: members.clone(),
            revision,
        },
        claims: members.borrow().claims(focused.as_deref()),
        focused: focused.clone(),
        is_active: props.is_active,
    };
    cx.provide(context.clone());
    let effect_props = props.clone();
    let effect_members = members.clone();
    cx.use_layout_effect(
        (props.clone(), revision.get(), focused.clone()),
        move || {
            let mut members = effect_members.borrow_mut();
            members.measure();
            let activation = effect_props.is_active && !members.was_active;
            members.was_active = effect_props.is_active;
            let auto_focus = std::mem::take(&mut members.auto_focus_pending);
            let removed = std::mem::take(&mut members.removed);
            let missing = removed
                .iter()
                .find(|member| Some(&member.key) == focused.as_ref())
                .filter(|_| {
                    !members
                        .entries
                        .iter()
                        .any(|member| Some(&member.key) == focused.as_ref())
                });
            let target = if let Some(removed) = missing {
                Some(
                    members
                        .entries
                        .iter()
                        .find(|member| member.position >= removed.position)
                        .or_else(|| members.entries.last())
                        .map(|member| member.key.clone()),
                )
            } else if effect_props.is_active && focused.is_none() && (activation || auto_focus) {
                members
                    .entries
                    .iter()
                    .find(|member| member.item.auto_focus)
                    .map(|member| Some(member.key.clone()))
            } else {
                None
            };
            drop(members);
            if let Some(target) = target {
                propose(
                    managed,
                    &effect_props.focused,
                    &effect_props.on_focus,
                    target,
                );
            }
        },
    );
    let props_snapshot = props.clone();
    cx.use_input(props.is_active, move |_, key| {
        if key.ctrl || key.meta || key.super_key || key.hyper {
            return;
        }
        if key.escape {
            if let Some(callback) = &props_snapshot.on_escape {
                callback.call(());
            }
            return;
        }
        let arrows = props_snapshot.arrows && context.claims == Claims::None;
        if !key.tab
            && !(arrows && (key.up_arrow || key.down_arrow || key.left_arrow || key.right_arrow))
        {
            return;
        }
        let previous = (key.tab && key.shift) || key.up_arrow || key.left_arrow;
        let focused = if props_snapshot.focused == RingFocus::Managed {
            managed.get()
        } else {
            context.focused.clone()
        };
        let mut members = members.borrow_mut();
        members.measure();
        let count = members.entries.len();
        let index = members
            .entries
            .iter()
            .position(|member| Some(&member.key) == focused.as_ref());
        let target = if count == 0 {
            None
        } else {
            let index = match (index, previous) {
                (Some(index), true) => (index + count - 1) % count,
                (Some(index), false) => (index + 1) % count,
                (None, true) => count - 1,
                (None, false) => 0,
            };
            Some(members.entries[index].key.clone())
        };
        drop(members);
        propose(
            managed,
            &props_snapshot.focused,
            &props_snapshot.on_focus,
            target,
        );
    });
    Element::fragment(props.children.clone())
}
