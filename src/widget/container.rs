use crate::{
    components::{self, layout, Edges},
    layout::{CrossAlign, Direction, FlowLayout, Layout, StackLayout},
    style::WithComponent,
    Scope, Widget, WidgetCollection,
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
