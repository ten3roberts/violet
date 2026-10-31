use std::sync::Arc;

use crate::{
    state::StateDuplex,
    style::{
        default_corner_radius, inset_shadow_well, surface_primary, Background, SizeExt, StyleExt,
        WidgetSizeProps,
    },
    widget::{row, ButtonStyle, Selectable, Stack},
    Edges, Scope, Widget,
};

/// A row of mutually exclusive options in one well, the selected one raised
pub struct SegmentedControl<T, W> {
    selection: Arc<dyn Send + Sync + StateDuplex<Item = T>>,
    items: Vec<(T, W)>,
    size: WidgetSizeProps,
}

impl<T: 'static + Send + Sync + Clone + PartialEq, W: Widget> SegmentedControl<T, W> {
    /// Shows `items`, each a value and its label, selecting into `selection`
    pub fn new(
        selection: impl 'static + Send + Sync + StateDuplex<Item = T>,
        items: impl IntoIterator<Item = (T, W)>,
    ) -> Self {
        Self {
            selection: Arc::new(selection),
            items: items.into_iter().collect(),
            size: Default::default(),
        }
    }
}

impl<T: 'static + Send + Sync + Clone + PartialEq, W: Widget> Widget for SegmentedControl<T, W> {
    fn mount(self, scope: &mut Scope<'_>) {
        self.size.mount(scope);

        let segments = self
            .items
            .into_iter()
            .map(|(value, label)| {
                Selectable::new_value(label, self.selection.clone(), value)
                    .with_style(ButtonStyle::segment())
            })
            .collect::<Vec<_>>();

        Stack::new(row(segments))
            .with_background(
                Background::new(surface_primary()).with_inset_shadow(inset_shadow_well()),
            )
            .with_padding(Edges::even(2.0))
            .with_corner_radius(default_corner_radius())
            .mount(scope);
    }
}

impl<T, W> SizeExt for SegmentedControl<T, W> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}
