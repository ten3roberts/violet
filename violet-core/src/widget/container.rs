use futures_signals::signal::Mutable;
use glam::Vec2;
use winit::event::ElementState;

use crate::{
    components::{anchor, layout, margin, max_size, min_size, offset, padding, rect},
    input::{focusable, on_cursor_move, on_mouse_input},
    layout::{Alignment, Direction, FlowLayout, Layout, StackLayout},
    style::{Background, StyleExt},
    unit::Unit,
    Edges, Frame, Scope, Widget, WidgetCollection,
};

/// Style for most container type widgets.
///
/// Includes margin, padding, and background color.
///
/// **NOTE**: direction and alignment are not included here, and should be given on a per-widget basis.
#[derive(Default, Debug, Clone)]
pub struct ContainerStyle {
    pub margin: Edges,
    pub padding: Edges,
    pub background: Option<Background>,
}

impl ContainerStyle {
    pub fn mount(self, scope: &mut Scope) {
        if let Some(background) = self.background {
            background.mount(scope);
        }

        scope
            .set(margin(), self.margin)
            .set(padding(), self.padding);
    }
}

pub struct Stack<W> {
    items: W,

    layout: StackLayout,
    style: ContainerStyle,
    min_size: Option<Unit<Vec2>>,
    max_size: Option<Unit<Vec2>>,
}

impl<W> Stack<W> {
    pub fn new(items: W) -> Self {
        Self {
            items,
            layout: StackLayout::default(),
            style: Default::default(),
            min_size: None,
            max_size: None,
        }
    }

    /// Set the horizontal alignment
    pub fn with_horizontal_alignment(mut self, align: Alignment) -> Self {
        self.layout.horizontal_alignment = align;
        self
    }

    /// Set the vertical alignment
    pub fn with_vertical_alignment(mut self, align: Alignment) -> Self {
        self.layout.vertical_alignment = align;
        self
    }

    pub fn with_margin(mut self, margin: Edges) -> Self {
        self.style.margin = margin;
        self
    }

    pub fn with_padding(mut self, padding: Edges) -> Self {
        self.style.padding = padding;
        self
    }

    pub fn with_background(mut self, background: Background) -> Self {
        self.style.background = Some(background);
        self
    }

    /// Set the max size
    pub fn with_max_size(mut self, max_size: Unit<Vec2>) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Set the min size
    pub fn with_min_size(mut self, min_size: Unit<Vec2>) -> Self {
        self.min_size = Some(min_size);
        self
    }
}

impl<W> StyleExt for Stack<W> {
    type Style = ContainerStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl<W> Widget for Stack<W>
where
    W: WidgetCollection,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.items.attach(scope);

        self.style.mount(scope);

        scope
            .set(layout(), Layout::Stack(self.layout))
            .set_opt(min_size(), self.min_size)
            .set_opt(max_size(), self.max_size);
    }
}

#[derive(Default)]
pub struct List<W> {
    items: W,
    layout: FlowLayout,
    style: ContainerStyle,
    min_size: Option<Unit<Vec2>>,
    max_size: Option<Unit<Vec2>>,
}

impl<W: WidgetCollection> List<W> {
    pub fn new(items: W) -> Self {
        Self {
            items,
            layout: FlowLayout::default(),
            style: Default::default(),
            min_size: None,
            max_size: None,
        }
    }

    /// Set the List's direction
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.layout.direction = direction;
        self
    }

    /// Set the List's cross axis alignment
    pub fn with_cross_align(mut self, cross_align: Alignment) -> Self {
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

    pub fn with_margin(mut self, margin: Edges) -> Self {
        self.style.margin = margin;
        self
    }

    pub fn with_padding(mut self, padding: Edges) -> Self {
        self.style.padding = padding;
        self
    }

    pub fn with_background(mut self, background: Background) -> Self {
        self.style.background = Some(background);
        self
    }

    /// Set the max size
    pub fn with_max_size(mut self, max_size: Unit<Vec2>) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Set the min size
    pub fn with_min_size(mut self, min_size: Unit<Vec2>) -> Self {
        self.min_size = Some(min_size);
        self
    }
}

impl<W: WidgetCollection> StyleExt for List<W> {
    type Style = ContainerStyle;
    fn with_style(mut self, style: ContainerStyle) -> Self {
        self.style = style;
        self
    }
}

impl<W: WidgetCollection> Widget for List<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        self.style.mount(scope);

        scope
            .set(layout(), Layout::Flow(self.layout))
            .set_opt(max_size(), self.max_size)
            .set_opt(min_size(), self.min_size);

        self.items.attach(scope);
    }
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

        scope
            .set(focusable(), ())
            .set(offset(), Unit::default())
            .on_event(on_mouse_input(), {
                let start_offset = start_offset.clone();
                move |_, _, input| {
                    if input.state == ElementState::Pressed {
                        let cursor_pos = input.cursor.local_pos;
                        *start_offset.lock_mut() = cursor_pos;
                    }
                }
            })
            .on_event(on_cursor_move(), move |frame, entity, input| {
                let rect = entity.get_copy(rect()).unwrap();
                let anchor = entity
                    .get_copy(anchor())
                    .unwrap_or_default()
                    .resolve(rect.size());

                let cursor_pos = input.local_pos + rect.min;

                let new_offset = cursor_pos - start_offset.get() + anchor;
                let new_offset = (self.on_move)(frame, new_offset);
                entity.update_dedup(offset(), Unit::px(new_offset));
            });

        Stack::new(self.content).mount(scope)
    }
}
