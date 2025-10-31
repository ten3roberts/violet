use futures_signals::signal::Mutable;
use glam::{BVec2, Vec2};
use winit::event::ElementState;

use crate::{
    components::{anchor, layout, offset, rect, LayoutAlignment},
    input::{interactive, on_cursor_move, on_mouse_input},
    layout::{Align, Direction, FloatLayout, FlowLayout, Layout, StackLayout},
    scope::ScopeRef,
    style::{
        default_corner_radius, default_separation, spacing_small, surface_secondary,
        surface_tertiary, Background, SizeExt, StyleExt, WidgetSizeProps,
    },
    unit::Unit,
    Scope, Widget, WidgetCollection,
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

/// The stack layout
///
/// A stack layout is the Swiss army knife of layouts.
///
/// It can be used to create a stacked arrangement of widgets, aligning widgets in a horizontal or
/// vertical direction, or constraining and offsetting widgets within.
///
/// In short, this layout can works as one of the following:
/// - Stack
/// - Overlaying widgets
/// - Horizontal or vertical alignment
/// - Padding and margin with background colors (widgets don't inherently have a concept of "inner"
///     content, as they are their own content)
/// - Centering widgets (this isn't HTML :P)
/// - Limiting and expanding size of widgets
///
/// Margins:
/// By default, the stack layout will inherit the margins of the inner children
pub struct Stack<W> {
    items: W,

    layout: StackLayout,
    style: ContainerStyle,
    size: WidgetSizeProps,
}

impl<W> Stack<W> {
    pub fn new(items: W) -> Self
    where
        W: WidgetCollection,
    {
        Self {
            items,
            layout: StackLayout::default(),
            style: Default::default(),
            size: Default::default(),
        }
    }

    /// Set the horizontal alignment
    pub fn with_horizontal_alignment(mut self, align: Align) -> Self {
        self.layout.alignment.horizontal = align;
        self
    }

    /// Set the vertical alignment
    pub fn with_vertical_alignment(mut self, align: Align) -> Self {
        self.layout.alignment.vertical = align;
        self
    }

    pub fn with_alignment(mut self, align: LayoutAlignment) -> Self {
        self.layout.alignment = align;
        self
    }

    pub fn with_background(mut self, background: impl Into<Background>) -> Self {
        self.style.background = Some(background.into());
        self
    }

    pub fn with_background_opt(mut self, background: impl Into<Option<Background>>) -> Self {
        self.style.background = background.into();
        self
    }

    pub fn with_clip(mut self, clip: impl Into<BVec2>) -> Self {
        self.layout.clip = clip.into();
        self
    }

    // Preserved minimum size in the given axis, even if clipping is enabled
    pub fn with_preserve_size(mut self, preserve_size: impl Into<BVec2>) -> Self {
        self.layout.preserve_size = preserve_size.into();
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
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
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

#[derive(Clone, Default)]
pub struct List<W> {
    items: W,
    layout: FlowLayout,
    style: ContainerStyle,
    size: WidgetSizeProps,
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

    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.layout.reverse = reverse;
        self
    }

    /// Set the List's cross axis alignment
    pub fn with_cross_align(mut self, cross_align: Align) -> Self {
        self.layout.cross_align = cross_align;
        self
    }

    pub fn with_contain_margins(mut self, enable: bool) -> Self {
        self.layout.contain_margins = enable;
        self
    }

    pub fn with_stretch(mut self, enable: bool) -> Self {
        self.layout.stretch = enable;
        self
    }

    pub fn with_background(mut self, background: impl Into<Background>) -> Self {
        self.style.background = Some(background.into());
        self
    }

    /// Shorthand for `with_cross_align(Align::Center)`
    pub fn center(mut self) -> Self {
        self.layout.cross_align = Align::Center;
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
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
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

type OnMove = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>, Vec2) -> Vec2>;
type OnDrop = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>, Vec2)>;

/// Allows a widget to be dragged around using the mouse.
///
/// Building block for windows and other draggable widgets.
pub struct Movable<W> {
    content: W,
    on_move: OnMove,
    on_drop: OnDrop,
    size: WidgetSizeProps,
}

impl<W> Movable<W> {
    pub fn new(content: W) -> Self {
        Self {
            content,
            on_move: Box::new(|_, v| v),
            on_drop: Box::new(|_, _| {}),
            size: Default::default(),
        }
    }

    pub fn on_move(
        mut self,
        on_move: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>, Vec2) -> Vec2,
    ) -> Self {
        self.on_move = Box::new(on_move);
        self
    }

    pub fn on_drop(
        mut self,
        on_drop: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>, Vec2),
    ) -> Self {
        self.on_drop = Box::new(on_drop);
        self
    }
}

impl<W: Widget> Widget for Movable<W> {
    fn mount(mut self, scope: &mut Scope<'_>) {
        let start_offset = Mutable::new(Vec2::ZERO);

        scope
            .set(interactive(), ())
            .set(offset(), Unit::default())
            .on_event(on_mouse_input(), {
                let start_offset = start_offset.clone();
                move |scope, input| {
                    if input.state == ElementState::Pressed {
                        let cursor_pos = input.cursor.local_pos;
                        tracing::debug!(?cursor_pos, "grab");
                        *start_offset.lock_mut() = cursor_pos;
                    } else {
                        (self.on_drop)(scope, input.cursor.absolute_pos);
                    }

                    None
                }
            })
            .on_event(on_cursor_move(), move |scope, input| {
                let rect = scope.get_copy(rect()).unwrap();
                let anchor = scope
                    .get_copy(anchor())
                    .unwrap_or_default()
                    .resolve(rect.size());

                let cursor_pos = input.local_pos + rect.min;

                let new_offset = cursor_pos - start_offset.get() + anchor;
                let new_offset = (self.on_move)(scope, new_offset);
                scope.update_dedup(offset(), Unit::px(new_offset));

                None
            });

        self.content.mount(scope)
    }
}

impl<W> SizeExt for Movable<W> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}

pub struct Float<W> {
    items: W,
}

impl<W> Float<W> {
    pub fn new(items: W) -> Self {
        Self { items }
    }
}

impl<W> Widget for Float<W>
where
    W: WidgetCollection,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.items.attach(scope);

        scope.set(layout(), Layout::Float(FloatLayout {}));
    }
}

pub fn row<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Horizontal)
}

pub fn col<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Vertical)
}

pub fn centered<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_horizontal_alignment(Align::Center)
        .with_vertical_alignment(Align::Center)
}

pub fn centered_vertical<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget).with_vertical_alignment(Align::Center)
}

pub fn centered_horizontal<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget).with_horizontal_alignment(Align::Center)
}

/// Inset content area with margin
pub fn card<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_background(Background::new(surface_secondary()))
        .with_padding(default_separation())
        .with_margin(default_separation())
        .with_corner_radius(default_corner_radius())
}

pub fn raised_card<W: WidgetCollection>(widget: W) -> Stack<W> {
    card(widget).with_background(surface_tertiary())
}

/// Inset content area
pub fn panel<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_padding(default_separation())
        .with_background(Background::new(surface_secondary()))
}

pub fn maximized<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget).with_maximize(Vec2::ONE)
}

pub fn pill<W: Widget>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_background(Background::new(surface_tertiary()))
        .with_padding(spacing_small())
        .with_margin(spacing_small())
        .with_corner_radius(default_corner_radius())
}
