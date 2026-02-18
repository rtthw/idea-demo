//! # Demo Library

mod color;
mod flex;
mod label;
mod math;
mod object_tree;

pub use {color::*, flex::*, label::*, math::*, object_tree::*};

use std::{
    any::{Any, TypeId},
    collections::HashSet,
};


pub const OBJECT_TYPE_ID: TypeId = TypeId::of::<dyn Object>();

/// A visible element within the [`ObjectTree`].
///
/// ## Instantiation
///
/// *TODO: Document the instantiation process.*
#[allow(unused)]
pub trait Object: Any {
    /// Whether this object can accept pointer events like hovering and
    /// clicking.
    ///
    /// *Defaults to `true`.*
    fn accepts_pointer_events(&self) -> bool {
        true
    }

    fn children_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {}

    fn render(&self, pass: &mut RenderPass<'_>, renderer: &mut dyn Renderer) {}

    fn render_overlay(&self, pass: &mut RenderPass<'_>, renderer: &mut dyn Renderer) {}

    fn layout(&mut self, pass: &mut LayoutPass<'_>) {}

    fn measure(
        &mut self,
        pass: &mut MeasurePass<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32;

    fn compose(&mut self, pass: &mut ComposePass<'_>) {}

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::Default
    }

    fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {}
}

/// The current state of the [object](Object).
pub struct ObjectState {
    id: u64,

    global_area: Area,
    global_transform: Affine,

    layout_area: Area,
    layout_baseline_offset: f32,
    local_transform: Affine,
    scroll_translation: Point,

    /// Whether the object needs to be re-laid out.
    needs_layout: bool,
    /// Whether the object needs to be recomposed onto the screen (i.e. whether
    /// its global position should be updated).
    needs_compose: bool,
    /// Whether the object wants to be recomposed onto the screen (i.e. whether
    /// [`Object::compose`] should be called).
    wants_compose: bool,
    /// Whether the object's has had any child added or removed since the last
    /// update pass.
    children_changed: bool,
    /// Whether this object has been transformed (and therefore needs to ).
    transformed: bool,

    /// Whether this object is hovered by the user's mouse cursor.
    hovered: bool,
}

impl ObjectState {
    const fn new(id: u64) -> Self {
        Self {
            id,

            global_area: Area::ZERO,
            global_transform: Affine::IDENTITY,

            layout_area: Area::ZERO,
            layout_baseline_offset: 0.0,
            local_transform: Affine::IDENTITY,
            scroll_translation: Point::ZERO,

            needs_layout: true,
            needs_compose: true,
            wants_compose: true,
            children_changed: true,
            transformed: true,
            hovered: false,
        }
    }

    #[inline]
    pub const fn id(&self) -> u64 {
        self.id
    }

    #[inline]
    pub const fn area(&self) -> Area {
        self.global_area
    }

    fn merge_with_child(&mut self, child_state: &Self) {
        self.needs_layout |= child_state.needs_layout;
        self.children_changed |= child_state.children_changed;
    }
}



#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CursorIcon {
    #[default]
    Default,
    PointingHand,
    IBeam,
}



pub trait ViewContext {
    fn load_texture(&mut self, path: &str) -> u64;
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
    update_object_tree(node);
}

fn update_object_tree(mut node: ObjectNodeMut<'_>) {
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

    // if state.newly_instantiated {
    //     state.newly_instantiated = false;
    //     object.on_ready(&mut UpdatePass {
    //         state,
    //         children: children.reborrow_mut(),
    //     });
    // }

    let parent_state = &mut *state;
    for_each_child_object(object, children, |mut node| {
        update_object_tree(node.reborrow_mut());
        parent_state.merge_with_child(&node.state);
    });
}



pub struct EventPass<'tree> {
    state: &'tree mut ObjectState,
    children: ObjectChildrenMut<'tree>,
    handled: bool,
}

fn event_pass(
    tree: &mut ObjectTree,
    target: Option<u64>,
    mut callback: impl FnMut(&mut dyn Object, &mut EventPass<'_>),
) {
    let mut target_id = target;
    let mut handled = false;
    while let Some(node_id) = target_id {
        let parent_id = {
            let mut node = tree
                .find_mut(node_id)
                .expect("invalid object ID for event target");

            if !handled {
                let mut pass = EventPass {
                    state: &mut node.state,
                    children: node.children,
                    handled: false,
                };
                callback(&mut **node.object, &mut pass);

                handled = pass.handled;
            }

            node.parent_id
        };

        if let Some(parent_id) = parent_id {
            let mut parent_node = tree.find_mut(parent_id).unwrap();
            let node = parent_node.children.get_mut(node_id).unwrap();

            parent_node.state.merge_with_child(&node.state);
        }

        target_id = parent_id;
    }
}

fn single_event_pass(
    tree: &mut ObjectTree,
    target: Option<u64>,
    mut callback: impl FnMut(&mut dyn Object, &mut EventPass<'_>),
) {
    let Some(target) = target else {
        return;
    };

    let mut node = tree
        .find_mut(target)
        .expect("invalid object ID passed to single_event_pass");

    let mut pass = EventPass {
        state: &mut node.state,
        children: node.children,
        handled: false,
    };
    callback(&mut **node.object, &mut pass);

    let mut current_id = Some(target);
    while let Some(node_id) = current_id {
        let parent_id = tree
            .find_mut(node_id)
            .expect("invalid object ID for pointer target")
            .parent_id;
        if let Some(parent_id) = parent_id {
            let mut parent_node = tree.find_mut(parent_id).unwrap();
            let node = parent_node.children.get_mut(node_id).unwrap();

            parent_node.state.merge_with_child(&node.state);
        }

        current_id = parent_id;
    }
}

fn update_pointer_pass(tree: &mut ObjectTree) {
    let next_hovered_object = tree
        .pointer_position
        .and_then(|pos| find_pointer_target(tree.root_node(), pos))
        .map(|node| node.state.id());
    let next_hovered_path =
        next_hovered_object.map_or(Vec::new(), |node_id| tree.get_id_path(node_id, None));
    let prev_hovered_path = std::mem::take(&mut tree.hovered_path);
    let prev_hovered_object = prev_hovered_path.first().copied();

    if prev_hovered_path != next_hovered_path {
        let mut hovered_set = HashSet::new();
        for node_id in &next_hovered_path {
            hovered_set.insert(*node_id);
        }

        for node_id in prev_hovered_path.iter().copied() {
            if tree
                .find_mut(node_id)
                .map(|node| node.state.hovered != hovered_set.contains(&node_id))
                .unwrap_or(false)
            {
                let hovered = hovered_set.contains(&node_id);
                event_pass(tree, Some(node_id), |_object, pass| {
                    // if pass.state.hovered != hovered {
                    //     object.on_child_hover(pass, hovered);
                    // }
                    pass.state.hovered = hovered;
                });
            }
        }
        for node_id in next_hovered_path.iter().copied() {
            if tree
                .find_mut(node_id)
                .map(|node| node.state.hovered != hovered_set.contains(&node_id))
                .unwrap_or(false)
            {
                let hovered = hovered_set.contains(&node_id);
                event_pass(tree, Some(node_id), |_object, pass| {
                    // if pass.state.hovered != hovered {
                    //     object.on_child_hover(pass, hovered);
                    // }
                    pass.state.hovered = hovered;
                });
            }
        }
    }

    if prev_hovered_object != next_hovered_object {
        single_event_pass(tree, prev_hovered_object, |object, pass| {
            pass.state.hovered = false;
            object.on_hover(pass, false);
        });
        single_event_pass(tree, next_hovered_object, |object, pass| {
            pass.state.hovered = true;
            object.on_hover(pass, true);
        });
    }

    let next_cursor_icon =
        if let Some(node_id) = tree.pointer_capture_target.or(next_hovered_object) {
            let node = tree
                .find_mut(node_id)
                .expect("failed to find the object tree's hover target");

            node.object.cursor_icon()
        } else {
            CursorIcon::Default
        };

    tree.cursor_icon = next_cursor_icon;
    tree.hovered_path = next_hovered_path;
}

fn find_pointer_target<'tree>(
    node: ObjectNodeRef<'tree>,
    position: Point,
) -> Option<ObjectNodeRef<'tree>> {
    if !node.state.area().contains(position) {
        return None;
    }

    for child_id in node.object.children_ids().iter().rev() {
        if let Some(child) = find_pointer_target(
            node.children
                .reborrow()
                .get(*child_id)
                .expect("passed invalid child ID to find_pointer_target"),
            position,
        ) {
            return Some(child);
        }
    }

    if node.object.accepts_pointer_events() {
        Some(node)
    } else {
        None
    }
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
    fn quad(&mut self, position: Point, size: Size, color: Rgba);
    fn image(&mut self, texture_id: u64, position: Point, size: Size);
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
    let size = size.round();
    let object = &mut **node.object;
    let state = &mut node.state;
    let children = node.children;

    if !state.needs_layout {
        return;
    }
    state.needs_layout = false;

    state.layout_area.size = size;

    object.layout(&mut LayoutPass {
        state,
        children,
        size,
        context: measure_context,
    });

    state.needs_compose = true;
    state.wants_compose = true;
}

fn place_object(state: &mut ObjectState, position: Point) {
    let position = position.round();
    if position != state.layout_area.position {
        state.transformed = true;
    }

    state.layout_area.position = position;
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



pub struct ComposePass<'tree> {
    state: &'tree mut ObjectState,
    children: ObjectChildrenMut<'tree>,
}

impl ComposePass<'_> {
    pub fn set_child_scroll(&mut self, child: &mut ChildObject, translation: Point) {
        let translation = translation.round();
        let child_state = &mut self
            .children
            .get_mut(child.id())
            .expect("invalid child passed to `ComposePass::set_child_scroll`")
            .state;
        if translation != child_state.scroll_translation {
            child_state.scroll_translation = translation;
            child_state.transformed = true;
        }
    }
}

pub fn compose_pass(view: &mut ObjectTree) {
    let node = view.root_node_mut();
    compose_object(node, Affine::IDENTITY, false);
}

fn compose_object(
    mut node: ObjectNodeMut<'_>,
    parent_global_transform: Affine,
    parent_transformed: bool,
) {
    let object = &mut **node.object;
    let state = &mut node.state;
    let mut children = node.children;

    let transformed = parent_transformed || state.transformed;

    if !transformed && !state.needs_compose {
        return;
    }

    let local_translation = state.scroll_translation + state.layout_area.position;
    state.global_transform =
        parent_global_transform * state.local_transform.with_translation(local_translation);
    state.global_area = state
        .global_transform
        .transform_area(Area::from_size(state.layout_area.size));

    if state.wants_compose {
        object.compose(&mut ComposePass {
            state,
            children: children.reborrow_mut(),
        });
    }

    state.needs_compose = false;
    state.wants_compose = false;
    state.transformed = false;

    let parent_state = &mut *state;
    for_each_child_object(object, children, |mut node| {
        compose_object(
            node.reborrow_mut(),
            parent_state.global_transform,
            transformed,
        );
        parent_state.merge_with_child(&node.state);
    });
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
    ComposePass<'_>,
    EventPass<'_>,
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
        pub const fn area(&self) -> Area {
            self.state.global_area
        }

        #[inline]
        pub const fn position(&self) -> Point {
            self.state.global_area.position
        }

        #[inline]
        pub fn size(&self) -> Size {
            self.state.global_area.size
        }

        #[inline]
        pub fn request_layout(&mut self) {
            self.state.needs_layout = true;
        }

        #[inline]
        pub fn request_compose(&mut self) {
            self.state.needs_compose = true;
            self.state.wants_compose = true;
        }

    }
}

// Types with a `children: ObjectChildrenMut` field.
multi_impl! {
    ComposePass<'_>,
    EventPass<'_>,
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
