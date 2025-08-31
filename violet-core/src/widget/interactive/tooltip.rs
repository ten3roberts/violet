use std::time::Duration;

use glam::Vec2;

use crate::{
    components::{offset, opacity},
    unit::Unit,
    widget::Stack,
    Scope, ScopeRef, Widget,
};

use super::{
    base::{InteractiveWidget, TooltipOptions},
    overlay::Overlay,
};

/// Show a tooltip on hover
pub struct Tooltip<T> {
    widget: T,
    options: TooltipOptions,
}

impl<T> Tooltip<T> {
    pub fn new<P>(widget: T, tooltip: impl 'static + Send + Sync + Fn(&ScopeRef<'_>) -> P) -> Self
    where
        P: 'static + Send + Widget,
    {
        Self {
            widget,
            options: TooltipOptions::new(tooltip),
        }
    }

    pub fn label(widget: T, label: impl Into<String>) -> Self {
        Self {
            widget,
            options: TooltipOptions::label(label),
        }
    }

    /// Set the tooltip offset
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.options.offset = offset;
        self
    }

    /// Set the delay
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.options.delay = delay;
        self
    }
}

impl<T: Widget> Widget for Tooltip<T> {
    fn mount(self, scope: &mut Scope<'_>) {
        InteractiveWidget::new(self.widget)
            .with_tooltip(self.options)
            .mount(scope);
    }
}

pub(crate) struct TooltipOverlay {
    pos: Vec2,
    widget: Box<dyn Send + Widget>,
}

impl TooltipOverlay {
    pub(crate) fn new(pos: Vec2, widget: impl 'static + Send + Widget) -> Self {
        Self {
            pos,
            widget: Box::new(widget),
        }
    }
}

impl Overlay for TooltipOverlay {
    fn create(self, scope: &mut Scope<'_>, _: super::overlay::OverlayHandle) {
        scope.set(offset(), Unit::px(self.pos)).set(opacity(), 0.9);
        self.widget.mount_boxed(scope);
    }
}
