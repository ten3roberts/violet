use std::sync::Arc;

use itertools::Itertools;

use crate::{
    state::{StateDuplex, StateExt},
    style::{SizeExt, StyleExt, WidgetSizeProps},
    widget::{col, ButtonStyle, ScrollArea, Selectable},
    Scope, Widget,
};

pub struct SelectList<I> {
    items: I,
    size: WidgetSizeProps,
    selection: Arc<dyn Send + Sync + StateDuplex<Item = Option<usize>>>,
}

impl<I> SelectList<I> {
    pub fn new(
        selection: impl 'static + Send + Sync + StateDuplex<Item = Option<usize>>,
        items: I,
    ) -> Self
    where
        I: IntoIterator,
        I::Item: Widget,
    {
        Self {
            items,
            selection: Arc::new(selection),
            size: Default::default(),
        }
    }
}

impl<T, I> Widget for SelectList<I>
where
    T: Widget,
    I: IntoIterator<Item = T>,
{
    fn mount(self, scope: &mut Scope<'_>) {
        ScrollArea::vertical(
            col(self
                .items
                .into_iter()
                .enumerate()
                .map(|(i, item)| {
                    Selectable::new_value(
                        item,
                        self.selection.clone().filter_map(|v| v, |v| Some(Some(v))),
                        i,
                    )
                    .with_style(ButtonStyle::selectable_entry())
                })
                .collect_vec())
            .with_stretch(true),
        )
        .with_size_props(self.size)
        .mount(scope);
    }
}

impl<I> SizeExt for SelectList<I> {
    fn size_mut(&mut self) -> &mut crate::style::WidgetSizeProps {
        &mut self.size
    }
}
