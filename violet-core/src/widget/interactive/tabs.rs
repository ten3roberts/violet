use std::sync::Arc;

use futures::StreamExt;

use crate::{
    state::{StateDuplex, StateExt, StateStream},
    widget::{col, StreamWidget},
    Scope, Widget,
};

use super::SegmentedControl;

type TabContent = Arc<dyn Send + Sync + Fn() -> Box<dyn Send + Widget>>;

/// One tab of [`Tabs`]: the value that selects it, its label, and its content
pub struct Tab<T> {
    value: T,
    label: Box<dyn Send + Sync + Widget>,
    content: TabContent,
}

impl<T> Tab<T> {
    /// A tab whose content `content` builds each time the tab is shown
    pub fn new<L, W>(value: T, label: L, content: impl 'static + Send + Sync + Fn() -> W) -> Self
    where
        L: 'static + Send + Sync + Widget,
        W: 'static + Send + Widget,
    {
        Self {
            value,
            label: Box::new(label),
            content: Arc::new(move || Box::new(content()) as Box<dyn Send + Widget>),
        }
    }
}

/// A [`SegmentedControl`] of tab labels over the selected tab's content.
///
/// Only the selected tab's content is mounted; switching tabs builds it anew.
pub struct Tabs<T> {
    selection: Arc<dyn Send + Sync + StateDuplex<Item = T>>,
    tabs: Vec<Tab<T>>,
}

impl<T: 'static + Send + Sync + Clone + PartialEq> Tabs<T> {
    pub fn new(
        selection: impl 'static + Send + Sync + StateDuplex<Item = T>,
        tabs: impl IntoIterator<Item = Tab<T>>,
    ) -> Self {
        Self {
            selection: Arc::new(selection),
            tabs: tabs.into_iter().collect(),
        }
    }
}

impl<T: 'static + Send + Sync + Clone + PartialEq> Widget for Tabs<T> {
    fn mount(self, scope: &mut Scope<'_>) {
        let (labels, contents): (Vec<_>, Vec<_>) = self
            .tabs
            .into_iter()
            .map(|tab| ((tab.value.clone(), tab.label), (tab.value, tab.content)))
            .unzip();

        let content = self.selection.clone().dedup().stream().map(move |value| {
            contents
                .iter()
                .find(|(v, _)| *v == value)
                .map(|(_, content)| content())
        });

        col((
            SegmentedControl::new(self.selection, labels),
            StreamWidget::new(content),
        ))
        .with_stretch(true)
        .mount(scope);
    }
}
