use futures_signals::signal::Mutable;
use glam::Vec2;
use winit::event::ElementState;

use crate::{
    components::{anchor, layout, offset, rect},
    input::{focusable, on_cursor_move, on_mouse_input},
    layout::{Alignment, Direction, FlowLayout, Layout, StackLayout},
    style::{
        primary_background, secondary_background, spacing_medium, spacing_small, Background,
        SizeExt, StyleExt, WidgetSize,
    },
    unit::Unit,
    Frame, Scope, Widget, WidgetCollection,
};

/// Style for most container type widgets.
///
/// Includes margin, padding, and background color.
///
/// **NOTE**: direction and alignment are not included here, and should be given on a per-widget basis.
#[derive(Default, Debug, Clone)]
pub struct ContainerStyle {
    pub background: Option<Background>,
}

impl ContainerStyle {
    pub fn mount(self, scope: &mut Scope) {
        if let Some(background) = self.background {
            background.mount(scope);
        }
    }
}

pub struct Stack<W> {
    items: W,

    layout: StackLayout,
    style: ContainerStyle,
    size: WidgetSize,
}

impl<W> Stack<W> {
    pub fn new(items: W) -> Self {
        Self {
            items,
            layout: StackLayout::default(),
            style: Default::default(),
            size: Default::default(),
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

    pub fn with_background(mut self, background: Background) -> Self {
        self.style.background = Some(background);
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

impl<W> SizeExt for Stack<W> {
    fn size_mut(&mut self) -> &mut WidgetSize {
        &mut self.size
    }
}

impl<W> Widget for Stack<W>
where
    W: WidgetCollection,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.items.attach(scope);

        self.style.mount(scope);
        self.size.mount(scope);

        scope.set(layout(), Layout::Stack(self.layout));
    }
}

#[derive(Default)]
pub struct List<W> {
    items: W,
    layout: FlowLayout,
    style: ContainerStyle,
    size: WidgetSize,
}

impl<W: WidgetCollection> List<W> {
    pub fn new(items: W) -> Self {
        Self {
            items,
            layout: FlowLayout::default(),
            style: Default::default(),
            size: Default::default(),
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

    pub fn with_background(mut self, background: Background) -> Self {
        self.style.background = Some(background);
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

impl<W: WidgetCollection> SizeExt for List<W> {
    fn size_mut(&mut self) -> &mut WidgetSize {
        &mut self.size
    }
}

impl<W: WidgetCollection> Widget for List<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        self.items.attach(scope);

        self.style.mount(scope);
        self.size.mount(scope);

        scope.set(layout(), Layout::Flow(self.layout));
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

pub fn row<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Horizontal)
}

pub fn column<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Vertical)
}

pub fn centered<W>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_horizontal_alignment(Alignment::Center)
        .with_vertical_alignment(Alignment::Center)
}

pub fn card<W>(widget: W) -> Stack<W> {
    Stack::new(widget)
        // TODO: semantic color and sizing increment
        .with_background(Background::new(secondary_background()))
        .with_padding(spacing_medium())
        .with_margin(spacing_medium())
}

pub fn pill<W>(widget: W) -> Stack<W> {
    Stack::new(widget)
        // TODO: semantic color and sizing increment
        .with_background(Background::new(primary_background()))
        .with_padding(spacing_small())
        .with_margin(spacing_small())
}
