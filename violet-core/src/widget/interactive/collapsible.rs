use std::f32::consts::PI;

use futures_signals::signal::Mutable;
use glam::{vec2, BVec2};
use palette::Srgba;
use tween::Tweener;

use crate::{
    components::{max_size, min_size, rect, rotation, transform_origin, translation, visible},
    layout::Align,
    state::StateStream,
    stored::WeakHandle,
    style::{
        icon_chevron, surface_secondary, Background, ResolvableStyle, SizeExt, StyleExt,
        ValueOrRef, WidgetSizeProps,
    },
    tweens::tweens,
    unit::Unit,
    utils::zip_latest,
    widget::{col, label, row, Button, ButtonStyle, Stack, Text},
    Scope, Widget,
};

pub struct CollapsibleStyle {
    button: ButtonStyle,
    content: ValueOrRef<Srgba>,
    chevron: ValueOrRef<String>,
}

impl Default for CollapsibleStyle {
    fn default() -> Self {
        Self {
            button: ButtonStyle::hidden(),
            content: surface_secondary().into(),
            chevron: icon_chevron().into(),
        }
    }
}

impl CollapsibleStyle {
    pub fn with_chevron(mut self, chevron: impl Into<ValueOrRef<String>>) -> Self {
        self.chevron = chevron.into();
        self
    }

    pub fn with_button_style(mut self, style: ButtonStyle) -> Self {
        self.button = style;
        self
    }

    pub fn with_background(mut self, content: impl Into<ValueOrRef<Srgba>>) -> Self {
        self.content = content.into();
        self
    }
}

/// Displays a horizontal header that can be clicked to collapse or expand its subtree.
pub struct Collapsible<L, W> {
    label: L,
    size: WidgetSizeProps,
    inner: W,
    style: CollapsibleStyle,
    can_collapse: bool,
}

impl<W> Collapsible<Text, W> {
    pub fn label(text: impl Into<String>, inner: W) -> Self
    where
        W: Widget,
    {
        Self::new(label(text.into()), inner)
    }
}

impl<L, W> Collapsible<L, W> {
    pub fn new(label: L, inner: W) -> Self
    where
        L: Widget,
        W: Widget,
    {
        Self {
            size: Default::default(),
            inner,
            label,
            style: Default::default(),
            can_collapse: true,
        }
    }

    pub fn can_collapse(mut self, can_collapse: bool) -> Self {
        self.can_collapse = can_collapse;
        self
    }
}

impl<L: Widget, W: Widget> Widget for Collapsible<L, W> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let collapsed = scope.store(Mutable::new(false));

        col((
            CollapsibleHeader {
                label: self.label,
                collapse: collapsed,
                style: &self.style,
                can_collapse: self.can_collapse,
            },
            CollapsibleContent {
                collapsed,
                inner: self.inner,
            },
        ))
        // .with_background(Background::new(self.style.content))
        .with_stretch(true)
        .with_size_props(self.size)
        .mount(scope);
    }
}

struct CollapsibleHeader<'a, L> {
    can_collapse: bool,
    collapse: WeakHandle<Mutable<bool>>,
    style: &'a CollapsibleStyle,

    label: L,
}

impl<L: Widget> Widget for CollapsibleHeader<'_, L> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        Button::new(
            row((
                CollapsibleChevron {
                    collapse: self.collapse.clone(),
                    style: self.style,
                    can_collapse: self.can_collapse,
                },
                self.label,
            ))
            .with_cross_align(Align::Center),
        )
        .with_style(self.style.button)
        .on_click(move |scope| {
            let value = &mut *scope.read(self.collapse).lock_mut();
            *value = !*value;
        })
        .mount(scope);
    }
}

struct CollapsibleChevron<'a> {
    can_collapse: bool,
    collapse: WeakHandle<Mutable<bool>>,
    style: &'a CollapsibleStyle,
}

impl Widget for CollapsibleChevron<'_> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let chevron = self.style.chevron.resolve(scope.stylesheet());
        scope.spawn_stream(scope.read(&self.collapse).stream(), |scope, value| {
            let current_rotation = scope.entity().get_copy(rotation()).unwrap_or_default();

            scope.stop_tweens(rotation());
            scope.add_tween(
                rotation(),
                Tweener::back_out(current_rotation, !value as i32 as f32 * PI / 2.0, 0.2),
            );
        });

        scope
            .set(visible(), self.can_collapse)
            .set(transform_origin(), vec2(0.5, 0.5))
            .set_default(rotation())
            .set_default(translation())
            .set_default(tweens());

        label(chevron).mount(scope);
    }
}

struct CollapsibleContent<W> {
    collapsed: WeakHandle<Mutable<bool>>,
    inner: W,
}

impl<W: Widget> Widget for CollapsibleContent<W> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let inner_size = Mutable::new(None);
        let mut old_size = None;

        let stream = zip_latest(scope.read(&self.collapsed).stream(), inner_size.stream());
        scope.spawn_stream(stream, move |scope, (collapsed, inner_size)| {
            let Some(inner_size) = inner_size else {
                return;
            };
            // let old_size = if collapsed { inner_size } else { 0.0 };

            let new_height = if collapsed { 0.0 } else { inner_size };

            let old_size = old_size.replace(new_height).unwrap_or(new_height);

            scope.add_tween(
                max_size(),
                Tweener::cubic_in_out(
                    Unit::px2(f32::MAX, old_size),
                    Unit::px2(f32::MAX, new_height),
                    0.2,
                ),
            );
            scope.add_tween(
                min_size(),
                Tweener::cubic_in_out(
                    Unit::px2(f32::MAX, old_size),
                    Unit::px2(f32::MAX, new_height),
                    0.2,
                ),
            );
        });

        scope
            .set(max_size(), Unit::px2(f32::MAX, f32::MAX))
            .set_default(tweens());
        Stack::new(|scope: &mut Scope<'_>| {
            scope.monitor(rect(), move |v| {
                if let Some(v) = v {
                    inner_size.set(Some(v.size().y));
                }
            });

            self.inner.mount(scope);
        })
        .with_clip(BVec2::new(false, true))
        .mount(scope);
    }
}

impl<L, W> SizeExt for Collapsible<L, W> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}

impl<L, W> StyleExt for Collapsible<L, W> {
    type Style = CollapsibleStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}
