//! # Object Tree
//!
//! See [`ObjectTree`] for details.

use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{CursorIcon, MeasureContext, Object, ObjectState, Point, Size};



/// A tree of [objects](Object) representing a user interface.
pub struct ObjectTree {
    root: u64,
    nodes: HashMap<u64, Box<UnsafeCell<ObjectNode>>>,
    parents: HashMap<u64, Option<u64>>,
    size: Size,

    pub(super) pointer_position: Option<Point>,
    pub(super) pointer_capture_target: Option<u64>,
    pub(super) hovered_path: Vec<u64>,
    pub(super) cursor_icon: CursorIcon,
}

impl ObjectTree {
    /// Create a new object tree with a single node containing the provided root
    /// [object](Object).
    pub fn new(root_object: Box<dyn Object>) -> Self {
        let root_id = 0;

        let mut nodes = HashMap::new();
        let mut parents = HashMap::new();

        parents.insert(root_id, None);
        nodes.insert(
            root_id,
            Box::new(UnsafeCell::new(ObjectNode {
                object: root_object,
                state: ObjectState::new(root_id),
                children: Vec::new(),
            })),
        );

        let mut this = Self {
            root: root_id,
            nodes,
            parents,
            size: Size::ZERO,
            pointer_position: None,
            pointer_capture_target: None,
            hovered_path: Vec::new(),
            cursor_icon: CursorIcon::Default,
        };

        crate::update_pass(&mut this);

        this
    }

    /// Get a shared (immutable) reference to this tree's root [object](Object)
    /// instance.
    pub fn root_node(&self) -> ObjectNodeRef<'_> {
        let node = unsafe {
            self.nodes
                .get(&self.root)
                .expect("root exists")
                .get()
                .as_ref()
                .expect("never null")
        };

        ObjectNodeRef {
            parent_id: None,
            object: &node.object,
            state: &node.state,
            children: ObjectChildrenRef {
                parent_id: Some(self.root),
                all_nodes: &self.nodes,
                all_parents: &self.parents,
            },
        }
    }

    /// Get an exclusive (mutable) reference to this tree's root
    /// [object](Object) instance.
    pub fn root_node_mut(&mut self) -> ObjectNodeMut<'_> {
        let node = unsafe {
            self.nodes
                .get(&self.root)
                .expect("root exists")
                .get()
                .as_mut()
                .expect("never null")
        };

        ObjectNodeMut {
            parent_id: None,
            object: &mut node.object,
            state: &mut node.state,
            children: ObjectChildrenMut {
                parent_id: Some(self.root),
                children: &mut node.children,
                all_nodes: &mut self.nodes,
                all_parents: &mut self.parents,
            },
        }
    }

    /// Get the current [cursor icon](CursorIcon) indicated by the
    /// [objects](Object) in the tree.
    #[inline]
    pub const fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    /// Get the current [visible size](Size) of the tree.
    #[inline]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Resize the tree to the provided [size](Size).
    pub fn resize(&mut self, size: Size, measure_context: &mut dyn MeasureContext) {
        if size == self.size {
            return;
        }
        self.size = size;
        crate::layout_pass(self, measure_context);
        crate::update_pass(self);
    }

    /// Get a shared (immutable) reference to the [object](Object) instance with
    /// the provided ID, if it exists.
    pub fn find(&self, id: u64) -> Option<ObjectNodeRef<'_>> {
        let parent_id = *self.parents.get(&id)?;
        let node = unsafe { self.nodes.get(&id)?.get().as_ref()? };

        Some(ObjectNodeRef {
            parent_id,
            object: &node.object,
            state: &node.state,
            children: ObjectChildrenRef {
                parent_id: Some(id),
                all_nodes: &self.nodes,
                all_parents: &self.parents,
            },
        })
    }

    /// Get an exclusive (mutable) reference to the [object](Object) instance
    /// with the provided ID, if it exists.
    pub fn find_mut(&mut self, id: u64) -> Option<ObjectNodeMut<'_>> {
        let parent_id = *self.parents.get(&id)?;
        let node = unsafe { self.nodes.get(&id)?.get().as_mut()? };

        Some(ObjectNodeMut {
            parent_id,
            object: &mut node.object,
            state: &mut node.state,
            children: ObjectChildrenMut {
                parent_id: Some(id),
                children: &mut node.children,
                all_nodes: &mut self.nodes,
                all_parents: &mut self.parents,
            },
        })
    }

    /// Get the path of object IDs from `id` to `start_id` (or the root ID if
    /// `start_id` is `None`).
    ///
    /// The returned path will be in bottom-up order.
    pub fn get_id_path(&self, id: u64, start_id: Option<u64>) -> Vec<u64> {
        let mut path = Vec::new();

        if !self.parents.contains_key(&id) {
            return path;
        }

        let mut current_id = Some(id);
        while let Some(current) = current_id {
            path.push(current);
            current_id = *self.parents.get(&current).unwrap();
            if current_id == start_id {
                break;
            }
        }

        if current_id != start_id {
            path.clear();
        }

        path
    }

    pub fn handle_pointer_move(
        &mut self,
        position: Option<Point>,
        measure_context: &mut dyn MeasureContext,
    ) {
        if position == self.pointer_position {
            return;
        }
        self.pointer_position = position;
        crate::update_pointer_pass(self);
        crate::layout_pass(self, measure_context);
        crate::update_pass(self);
    }
}

struct ObjectNode {
    object: Box<dyn Object>,
    state: ObjectState,
    children: Vec<u64>,
}

/// A shared (immutable) reference to an [object](Object) instance.
pub struct ObjectNodeRef<'tree> {
    pub parent_id: Option<u64>,
    pub object: &'tree Box<dyn Object>,
    pub state: &'tree ObjectState,
    pub children: ObjectChildrenRef<'tree>,
}

impl<'tree> ObjectNodeRef<'tree> {
    pub fn reborrow(&self) -> ObjectNodeRef<'tree> {
        ObjectNodeRef {
            parent_id: self.parent_id,
            object: self.object,
            state: self.state,
            children: self.children.reborrow(),
        }
    }
}

// impl<'tree> ObjectNodeRef<'tree> {
//     pub fn reborrow_up(&self) -> ObjectNodeRef<'tree> {
//         ObjectNodeRef {
//             parent_id: self.parent_id,
//             object: self.object,
//             state: self.state,
//             children: self.children.reborrow_up(),
//         }
//     }
// }

/// An exclusive (mutable) reference to an [object](Object) instance.
pub struct ObjectNodeMut<'tree> {
    pub parent_id: Option<u64>,
    pub object: &'tree mut Box<dyn Object>,
    pub state: &'tree mut ObjectState,
    pub children: ObjectChildrenMut<'tree>,
}

impl ObjectNodeMut<'_> {
    pub fn reborrow(&self) -> ObjectNodeRef<'_> {
        ObjectNodeRef {
            parent_id: self.parent_id,
            object: self.object,
            state: self.state,
            children: self.children.reborrow(),
        }
    }

    pub fn reborrow_mut(&mut self) -> ObjectNodeMut<'_> {
        ObjectNodeMut {
            parent_id: self.parent_id,
            object: self.object,
            state: self.state,
            children: self.children.reborrow_mut(),
        }
    }
}

/// A shared (immutable) reference to the children of an [object](Object)
/// instance.
pub struct ObjectChildrenRef<'tree> {
    parent_id: Option<u64>,
    all_nodes: &'tree HashMap<u64, Box<UnsafeCell<ObjectNode>>>,
    all_parents: &'tree HashMap<u64, Option<u64>>,
}

impl<'tree> ObjectChildrenRef<'tree> {
    pub fn has(&self, id: u64) -> bool {
        let child_id = id.into();
        let parent_id = self.parent_id;

        self.all_parents
            .get(&child_id)
            .is_some_and(|parent| *parent == parent_id)
    }

    pub fn get(&self, id: u64) -> Option<ObjectNodeRef<'tree>> {
        if self.has(id) {
            let parent_id = *self.all_parents.get(&id)?;
            let ObjectNode { object, state, .. } =
                unsafe { self.all_nodes.get(&id)?.get().as_ref() }?;

            let children = ObjectChildrenRef {
                parent_id: Some(id),
                all_nodes: self.all_nodes,
                all_parents: self.all_parents,
            };

            Some(ObjectNodeRef {
                parent_id,
                object,
                state,
                children,
            })
        } else {
            None
        }
    }

    pub fn reborrow(&self) -> ObjectChildrenRef<'tree> {
        ObjectChildrenRef {
            parent_id: self.parent_id,
            all_nodes: self.all_nodes,
            all_parents: self.all_parents,
        }
    }
}

/// An exclusive (mutable) reference to the children of an [object](Object)
/// instance.
pub struct ObjectChildrenMut<'tree> {
    parent_id: Option<u64>,
    children: &'tree mut Vec<u64>,
    all_nodes: &'tree mut HashMap<u64, Box<UnsafeCell<ObjectNode>>>,
    all_parents: &'tree mut HashMap<u64, Option<u64>>,
}

impl ObjectChildrenMut<'_> {
    pub fn has(&self, id: u64) -> bool {
        let child_id = id.into();
        let parent_id = self.parent_id;

        self.all_parents
            .get(&child_id)
            .is_some_and(|parent| *parent == parent_id)
    }

    pub fn get(&self, id: u64) -> Option<ObjectNodeRef<'_>> {
        if self.has(id) {
            let parent_id = *self.all_parents.get(&id)?;
            let ObjectNode { object, state, .. } =
                unsafe { self.all_nodes.get(&id)?.get().as_ref() }?;

            let children = ObjectChildrenRef {
                parent_id: Some(id),
                all_nodes: self.all_nodes,
                all_parents: self.all_parents,
            };

            Some(ObjectNodeRef {
                parent_id,
                object,
                state,
                children,
            })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, id: u64) -> Option<ObjectNodeMut<'_>> {
        if self.has(id) {
            let parent_id = *self.all_parents.get(&id)?;
            let ObjectNode {
                object,
                state,
                children,
            } = unsafe { self.all_nodes.get(&id)?.get().as_mut() }?;

            let children = ObjectChildrenMut {
                parent_id: Some(id),
                children,
                all_nodes: self.all_nodes,
                all_parents: self.all_parents,
            };

            Some(ObjectNodeMut {
                parent_id,
                object,
                state,
                children,
            })
        } else {
            None
        }
    }

    pub fn push(&mut self, id: u64, object: Box<dyn Object>, state: ObjectState) {
        self.all_parents.insert(id, self.parent_id);

        self.children.push(id);

        let node = ObjectNode {
            object,
            state,
            children: Vec::new(),
        };

        self.all_nodes.insert(id, Box::new(UnsafeCell::new(node)));
    }

    pub fn reborrow(&self) -> ObjectChildrenRef<'_> {
        ObjectChildrenRef {
            parent_id: self.parent_id,
            all_nodes: self.all_nodes,
            all_parents: self.all_parents,
        }
    }

    pub fn reborrow_mut(&mut self) -> ObjectChildrenMut<'_> {
        ObjectChildrenMut {
            parent_id: self.parent_id,
            children: self.children,
            all_nodes: self.all_nodes,
            all_parents: self.all_parents,
        }
    }
}

/// A handle to a potentially uninstantiated child [object](Object) instance.
///
/// This object will be instantiated during the next [update
/// pass](crate::update_pass) as long as the parent object calls
/// [`update_child`](crate::UpdatePass::update_child) during its
/// [`update_children`](Object::update_children) method.
pub struct ChildObject {
    id: u64,
    inner: ChildObjectInner,
}

impl ChildObject {
    /// A unique identifier for this [object](Object).
    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Whether this [object](Object) has been instantiated into the
    /// [`ObjectTree`].
    pub fn exists(&self) -> bool {
        matches!(self.inner, ChildObjectInner::Existing)
    }

    pub(super) fn take_inner(&mut self) -> Option<ObjectBuilder> {
        match std::mem::replace(&mut self.inner, ChildObjectInner::Existing) {
            ChildObjectInner::New(builder) => Some(builder),
            ChildObjectInner::Existing => None,
        }
    }
}

enum ChildObjectInner {
    Existing,
    New(ObjectBuilder),
}

/// A tool for building [objects](Object) to be placed into the [`ObjectTree`].
pub struct ObjectBuilder {
    pub(super) id: u64,
    pub(super) object: Box<dyn Object>,
}

impl ObjectBuilder {
    /// Create a new [object](Object) builder.
    pub fn new<E: Object + 'static>(object: E) -> Self {
        static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed);

        Self {
            id,
            object: Box::new(object),
        }
    }

    /// Convert this object builder into an uninstantiated [`ChildObject`],
    /// which will be instantiated during the next [update
    /// pass](crate::update_pass).
    pub fn into_child(self) -> ChildObject {
        ChildObject {
            id: self.id,
            inner: ChildObjectInner::New(self),
        }
    }
}
