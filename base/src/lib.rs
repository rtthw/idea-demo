//! # Demo Library

mod color;
mod flex;
mod math;
mod object_tree;

pub use {color::*, flex::*, math::*, object_tree::*};

use std::any::{Any, TypeId};


pub const OBJECT_TYPE_ID: TypeId = TypeId::of::<dyn Object>();

/// A visible element within the [`ObjectTree`].
///
/// ## Instantiation
///
/// *TODO: Document the instantiation process.*
#[allow(unused)]
pub trait Object: Any {
    fn children_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {}

    fn render(&self, pass: &mut RenderPass<'_>, renderer: &mut dyn Renderer) {}

    fn render_overlay(&self, pass: &mut RenderPass<'_>, renderer: &mut dyn Renderer) {}

    fn layout(&mut self, pass: &mut LayoutPass<'_>);

    fn measure(
        &mut self,
        context: &mut MeasurePass<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32;
}

/// The current state of the [object](Object).
pub struct ObjectState {
    id: u64,

    layout_position: Point,
    layout_size: Size,
    layout_baseline_offset: f32,

    /// Whether the object needs to be re-laid out.
    needs_layout: bool,
    /// Whether the object's has had any child added or removed since the last
    /// update pass.
    children_changed: bool,
}

impl ObjectState {
    const fn new(id: u64) -> Self {
        Self {
            id,

            layout_position: Point::ZERO,
            layout_size: Size::ZERO,
            layout_baseline_offset: 0.0,

            needs_layout: true,
            children_changed: true,
        }
    }

    #[inline]
    pub const fn id(&self) -> u64 {
        self.id
    }

    fn merge_with_child(&mut self, child_state: &Self) {
        self.needs_layout |= child_state.needs_layout;
        self.children_changed |= child_state.children_changed;
    }
}



pub struct UpdatePass<'tree> {
    state: &'tree mut ObjectState,
    children: ObjectChildrenMut<'tree>,
}

impl UpdatePass<'_> {
    /// See [`Object::update_children`].
    pub fn update_child(&mut self, child: &mut ChildObject) {
        let Some(ObjectBuilder { id, object }) = child.take_inner() else {
            return;
        };

        let state = ObjectState::new(id);

        self.children.push(id, object, state);
    }
}

pub fn update_pass(tree: &mut ObjectTree) {
    let node = tree.root_node_mut();
    update_element_tree(node);
}

fn update_element_tree(mut node: ObjectNodeMut<'_>) {
    let mut children = node.children;
    let object = &mut **node.object;
    let state = &mut node.state;

    if !state.children_changed {
        return;
    }

    state.children_changed = false;

    object.update_children(&mut UpdatePass {
        state,
        children: children.reborrow_mut(),
    });

    // if state.newly_added {
    //     state.newly_added = false;
    //     object.on_build(&mut UpdatePass {
    //         state,
    //         children: children.reborrow_mut(),
    //     });
    // }

    let parent_state = &mut *state;
    for_each_child_object(object, children, |mut node| {
        update_element_tree(node.reborrow_mut());
        parent_state.merge_with_child(&node.state);
    });
}



pub fn render_pass(tree: &mut ObjectTree, renderer: &mut dyn Renderer) {
    let node = tree.root_node_mut();
    render_object(node, renderer);
}

fn render_object(mut node: ObjectNodeMut<'_>, renderer: &mut dyn Renderer) {
    let object = &**node.object;
    let state = &mut node.state;
    let children = node.children;

    object.render(&mut RenderPass { state }, renderer);

    let parent_state = &mut *state;
    for_each_child_object(object, children, |mut node| {
        render_object(node.reborrow_mut(), renderer);
        parent_state.merge_with_child(&node.state);
    });

    object.render_overlay(&mut RenderPass { state }, renderer);
}

pub struct RenderPass<'tree> {
    state: &'tree mut ObjectState,
}

pub trait Renderer {
    fn text(&mut self, content: &str, position: Point, font_size: f32, color: Rgba);
}



pub fn layout_pass(tree: &mut ObjectTree, measure_context: &mut dyn MeasureContext) {
    let size = tree.size();
    let node = tree.root_node_mut();
    layout_object(measure_context, node, size);
}

fn layout_object(
    measure_context: &mut dyn MeasureContext,
    mut node: ObjectNodeMut<'_>,
    size: Size,
) {
    let object = &mut **node.object;
    let state = &mut node.state;
    let children = node.children;

    if !state.needs_layout {
        return;
    }
    state.needs_layout = false;

    object.layout(&mut LayoutPass {
        state,
        children,
        size,
        context: measure_context,
    });
}

fn place_object(state: &mut ObjectState, position: Point) {
    // if position != state.layout_position {
    //     state.transformed = true;
    // }

    state.layout_position = position;
}

pub struct LayoutPass<'tree> {
    state: &'tree mut ObjectState,
    children: ObjectChildrenMut<'tree>,
    context: &'tree mut dyn MeasureContext,
    size: Size,
}

impl LayoutPass<'_> {
    pub fn do_layout(&mut self, child: &mut ChildObject, size: Size) {
        let mut node = self
            .children
            .get_mut(child.id())
            .expect("invalid child passed to LayoutPass::do_layout");
        layout_object(self.context, node.reborrow_mut(), size);
        self.state.merge_with_child(&node.state);
    }

    pub fn place_child(&mut self, child: &mut ChildObject, position: Point) {
        place_object(
            &mut self
                .children
                .get_mut(child.id())
                .expect("invalid child passed to LayoutPass::place_child")
                .state,
            position,
        );
    }
}



pub struct MeasurePass<'tree> {
    state: &'tree mut ObjectState,
    children: ObjectChildrenMut<'tree>,
    context: &'tree mut dyn MeasureContext,
}

fn resolve_axis_measurement(
    pass: &mut MeasurePass<'_>,
    object: &mut dyn Object,
    axis: Axis,
    length: Length,
    cross_length: Option<f32>,
) -> f32 {
    let length_request = match length {
        Length::MaxContent => LengthRequest::MaxContent,
        Length::MinContent => LengthRequest::MinContent,
        Length::FitContent(max_size) => LengthRequest::FitContent(max_size),
        Length::Exact(amount) => return amount,
    };
    object.measure(pass, axis, length_request, cross_length)
}

pub trait MeasureContext {
    fn text_size(&mut self, content: &str, font_size: f32) -> Size;
}

impl MeasureContext for () {
    fn text_size(&mut self, content: &str, font_size: f32) -> Size {
        let width = content.len() as f32 * (font_size / 2.0);
        let height = font_size;

        Size { width, height }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum Length {
    MaxContent,
    MinContent,
    FitContent(f32),
    Exact(f32),
}

impl Length {
    pub const fn exact(&self) -> Option<f32> {
        if let Self::Exact(amount) = *self {
            Some(amount)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum LengthRequest {
    MaxContent,
    MinContent,
    FitContent(f32),
}

impl Into<Length> for LengthRequest {
    fn into(self) -> Length {
        match self {
            LengthRequest::MaxContent => Length::MaxContent,
            LengthRequest::MinContent => Length::MinContent,
            LengthRequest::FitContent(max_size) => Length::FitContent(max_size),
        }
    }
}



fn for_each_child_object(
    object: &dyn Object,
    mut children: ObjectChildrenMut<'_>,
    mut callback: impl FnMut(ObjectNodeMut<'_>),
) {
    for child_id in object.children_ids() {
        callback(
            children
                .get_mut(child_id)
                .expect("Object::children_ids produced an invalid child ID"),
        );
    }
}



macro_rules! multi_impl {
    ($ty:ty, { $($item:item)+ }) => {
        impl $ty { $($item)+ }
    };
    ($ty:ty, $($others:ty),+, { $($item:item)+ }) => {
        multi_impl!($ty, { $($item)+ });
        multi_impl!($($others),+, { $($item)+ });
    };
}

// Types with a `state: &mut ObjectState` field.
multi_impl! {
    LayoutPass<'_>,
    MeasurePass<'_>,
    RenderPass<'_>,
    UpdatePass<'_>,
    {
        #[inline]
        pub fn id(&self) -> u64 {
            self.state.id()
        }

        #[inline]
        pub fn position(&self) -> Point {
            self.state.layout_position
        }

        pub fn request_layout(&mut self) {
            self.state.needs_layout = true;
        }
    }
}

// Types with a `children: ObjectChildrenMut` field.
multi_impl! {
    LayoutPass<'_>,
    MeasurePass<'_>,
    UpdatePass<'_>,
    {
        pub fn child(&self, id: u64) -> Option<ObjectNodeRef<'_>> {
            self.children.get(id)
        }

        pub fn expect_child(&self, id: u64) -> ObjectNodeRef<'_> {
            self.children.get(id).expect("invalid ID passed to `expect_child`")
        }
    }
}

// Types with a `context: &mut dyn MeasureContext` field.
multi_impl! {
    LayoutPass<'_>,
    MeasurePass<'_>,
    {
        pub fn resolve_length(
            &mut self,
            child_id: u64,
            axis: Axis,
            fallback_length: Length,
            cross_length: Option<f32>,
        ) -> f32 {
            let mut child = self
                .children
                .get_mut(child_id)
                .expect("invalid child ID provided to resolve_length");
            let object = &mut **child.object;
            let state = &mut child.state;
            let children = child.children;

            let mut pass = MeasurePass {
                state,
                children,
                context: self.context,
            };

            fallback_length.exact().unwrap_or_else(|| {
                resolve_axis_measurement(&mut pass, object, axis, fallback_length, cross_length)
            })
        }

        #[inline]
        pub fn measure_context(&mut self) -> &mut dyn MeasureContext {
            self.context
        }
    }
}
