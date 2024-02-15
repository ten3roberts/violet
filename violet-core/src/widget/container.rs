use futures_signals::signal::Mutable;
use glam::Vec2;
use winit::event::ElementState;

use crate::{
    components::{self, anchor, layout, offset, rect, Edges},
    input::{focusable, on_cursor_move, on_mouse_input},
    layout::{CrossAlign, Direction, FlowLayout, Layout, StackLayout},
    style::{StyleExt, WithComponent},
    unit::Unit,
    Frame, Scope, Widget, WidgetCollection,
};

pub struct Stack<W> {
    items: W,
    background: Option<Box<dyn Widget>>,

    horizontal_alignment: CrossAlign,
    vertical_alignment: CrossAlign,
}

impl<W> Stack<W> {
    pub fn new(items: W) -> Self {
        Self {
            items,
            background: None,
            horizontal_alignment: CrossAlign::default(),
            vertical_alignment: CrossAlign::default(),
        }
    }

    /// Set the horizontal alignment
    pub fn with_horizontal_alignment(mut self, align: CrossAlign) -> Self {
        self.horizontal_alignment = align;
        self
    }

    /// Set the vertical alignment
    pub fn with_vertical_alignment(mut self, align: CrossAlign) -> Self {
        self.vertical_alignment = align;
        self
    }
}

impl<W> ContainerExt for Stack<W> {
    fn with_background<B: 'static + Widget>(mut self, background: B) -> Self {
        self.background = Some(Box::new(background));
        self
    }
}

impl<W> Widget for Stack<W>
where
    W: WidgetCollection,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.items.attach(scope);

        if let Some(background) = self.background {
            background.mount(scope);
        }

        scope.set(
            layout(),
            Layout::Stack(StackLayout {
                horizontal_alignment: self.horizontal_alignment,
                vertical_alignment: self.vertical_alignment,
            }),
        );
    }
}

#[derive(Default)]
pub struct List<W> {
    items: W,
    layout: FlowLayout,
    background: Option<Box<dyn Widget>>,
}

impl<W: WidgetCollection> List<W> {
    pub fn new(items: W) -> Self {
        Self {
            items,
            layout: FlowLayout::default(),
            background: None,
        }
    }

    /// Set the List's direction
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.layout.direction = direction;
        self
    }

    /// Set the List's cross axis alignment
    pub fn with_cross_align(mut self, cross_align: CrossAlign) -> Self {
        self.layout.cross_align = cross_align;
        self
    }

    pub fn contain_margins(mut self, enable: bool) -> Self {
        self.layout.contain_margins = enable;
        self
    }

    pub fn with_stretch(mut self, enable: bool) -> Self {
        self.layout.stretch = enable;
        self
    }

    pub fn with_proportional_growth(mut self, proportional_growth: bool) -> Self {
        self.layout.proportional_growth = proportional_growth;
        self
    }
}

impl<W: WidgetCollection> ContainerExt for List<W> {
    fn with_background<B: 'static + Widget>(mut self, background: B) -> Self {
        self.background = Some(Box::new(background));
        self
    }
}

impl<W: WidgetCollection> Widget for List<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        if let Some(background) = self.background {
            background.mount(scope);
        }

        scope.set(layout(), Layout::Flow(self.layout));

        self.items.attach(scope);
    }
}

/// Additional functionality for available containers.
pub trait ContainerExt {
    fn with_padding(self, padding: Edges) -> WithComponent<Self, Edges>
    where
        Self: Sized,
    {
        WithComponent::new(self, components::padding(), padding)
    }

    /// Adds a background to the widget.
    fn with_background<W: 'static + Widget>(self, background: W) -> Self;
}

type OnMove = Box<dyn Send + Sync + FnMut(&Frame, Vec2) -> Vec2>;

/// Allows a widget to be dragged around using the mouse.
///
/// Building block for windows and other draggable widgets.
pub struct Movable<W> {
    content: W,
    on_move: OnMove,
}

impl<W> Movable<W> {
    pub fn new(content: W) -> Self {
        Self {
            content,
            on_move: Box::new(|_, v| v),
        }
    }

    pub fn on_move(
        mut self,
        on_move: impl 'static + Send + Sync + FnMut(&Frame, Vec2) -> Vec2,
    ) -> Self {
        self.on_move = Box::new(on_move);
        self
    }
}

impl<W: Widget> Widget for Movable<W> {
    fn mount(mut self, scope: &mut Scope<'_>) {
        let start_offset = Mutable::new(Vec2::ZERO);

        Stack::new(self.content)
            .with_component(focusable(), ())
            .with_component(offset(), Unit::default())
            .with_component(
                on_mouse_input(),
                Box::new({
                    let start_offset = start_offset.clone();
                    move |_, _, _, input| {
                        if input.state == ElementState::Pressed {
                            let cursor_pos = input.cursor.local_pos;
                            *start_offset.lock_mut() = cursor_pos;
                        }
                    }
                }),
            )
            .with_component(
                on_cursor_move(),
                Box::new({
                    move |frame, entity, _, input| {
                        let rect = entity.get_copy(rect()).unwrap();
                        let anchor = entity
                            .get_copy(anchor())
                            .unwrap_or_default()
                            .resolve(rect.size());

                        let cursor_pos = input.local_pos + rect.min;

                        let new_offset = cursor_pos - start_offset.get() + anchor;
                        let new_offset = (self.on_move)(frame, new_offset);
                        entity.update_dedup(offset(), Unit::px(new_offset));
                    }
                }),
            )
            .mount(scope)
    }
}
