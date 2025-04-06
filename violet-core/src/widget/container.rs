use futures_signals::signal::Mutable;
use glam::{BVec2, Vec2, Vec3, Vec3Swizzles};
use winit::event::ElementState;

use crate::{
    components::{anchor, layout, offset, rect, transform},
    input::{interactive, on_cursor_move, on_mouse_input},
    layout::{Align, Direction, FloatLayout, FlowLayout, Layout, StackLayout},
    scope::ScopeRef,
    style::{
        default_corner_radius, spacing_medium, surface_secondary, surface_tertiary, Background,
        SizeExt, StyleExt, WidgetSize,
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

pub struct Stack<W> {
    items: W,

    layout: StackLayout,
    style: ContainerStyle,
    size: WidgetSize,
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
        self.layout.horizontal_alignment = align;
        self
    }

    /// Set the vertical alignment
    pub fn with_vertical_alignment(mut self, align: Align) -> Self {
        self.layout.vertical_alignment = align;
        self
    }

    pub fn with_alignment(mut self, horizontal: Align, vertical: Align) -> Self {
        self.layout.horizontal_alignment = horizontal;
        self.layout.vertical_alignment = vertical;
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

type OnMove = Box<dyn Send + Sync + FnMut(&ScopeRef, Vec2) -> Vec2>;
type OnDrop = Box<dyn Send + Sync + FnMut(&ScopeRef, Vec2)>;

/// Allows a widget to be dragged around using the mouse.
///
/// Building block for windows and other draggable widgets.
pub struct Movable<W> {
    content: W,
    on_move: OnMove,
    on_drop: OnDrop,
    size: WidgetSize,
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
        on_move: impl 'static + Send + Sync + FnMut(&ScopeRef, Vec2) -> Vec2,
    ) -> Self {
        self.on_move = Box::new(on_move);
        self
    }

    pub fn on_drop(mut self, on_drop: impl 'static + Send + Sync + FnMut(&ScopeRef, Vec2)) -> Self {
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
                    let transform = scope
                        .get_copy(transform())
                        .unwrap_or_default()
                        .transform_point3(Vec3::ZERO)
                        .xy();

                    if input.state == ElementState::Pressed {
                        tracing::info!(%input.cursor.local_pos);
                        let cursor_pos = input.cursor.absolute_pos - transform;
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

                let cursor_pos = input.absolute_pos;

                let new_offset = cursor_pos - start_offset.get() + anchor;
                let new_offset = (self.on_move)(scope, new_offset);
                scope.update_dedup(offset(), Unit::px(new_offset));

                None
            });

        self.content.mount(scope)
    }
}

impl<W> SizeExt for Movable<W> {
    fn size_mut(&mut self) -> &mut WidgetSize {
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

pub fn card<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_background(Background::new(surface_secondary()))
        .with_padding(spacing_medium())
        .with_margin(spacing_medium())
        .with_corner_radius(default_corner_radius())
}

pub fn panel<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget).with_background(Background::new(surface_secondary()))
}

pub fn maximized<W: WidgetCollection>(widget: W) -> Stack<W> {
    Stack::new(widget).with_maximize(Vec2::ONE)
}

pub fn pill<W: Widget>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_background(Background::new(surface_tertiary()))
        .with_padding(spacing_medium())
        .with_margin(spacing_medium())
        .with_corner_radius(default_corner_radius())
}
