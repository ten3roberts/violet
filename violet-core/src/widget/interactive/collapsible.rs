use std::{
    f32::consts::PI,
    sync::atomic::{AtomicBool, Ordering},
};

use futures_signals::signal::Mutable;
use glam::{vec2, BVec2};
use itertools::Either;
use palette::Srgba;
use tween::Tweener;

use crate::{
    components::{max_size, min_size, rect, rotation, transform_origin, translation, visible},
    layout::Align,
    state::StateStream,
    stored::WeakHandle,
    style::{
        icon_chevron, surface_secondary, ResolvableStyle, SizeExt, StyleExt, ValueOrRef,
        WidgetSizeProps,
    },
    tweens::tweens,
    unit::Unit,
    utils::zip_latest,
    widget::{
        col,
        interactive::base::{ClickCallback, InteractiveWidget},
        label, row, ButtonStyle, Stack, Text,
    },
    Edges, Scope, ScopeRef, Widget,
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
pub struct Collapsible<L, W, F = fn() -> W> {
    label: L,
    size: WidgetSizeProps,
    inner: Either<W, F>,
    style: CollapsibleStyle,
    can_collapse: bool,
    on_click: Option<ClickCallback>,
    collapsed: bool,
    indent: bool,
}

impl<W> Collapsible<Text, W> {
    pub fn label(text: impl Into<String>, inner: W) -> Self
    where
        W: 'static + Widget,
    {
        Self::new(label(text.into()), inner)
    }
}

impl<L, W> Collapsible<L, W> {
    pub fn new(label: L, inner: W) -> Self
    where
        L: Widget,
        W: 'static + Widget,
    {
        Self {
            size: Default::default(),
            inner: Either::Left(inner),
            label,
            style: Default::default(),
            can_collapse: true,
            on_click: None,
            collapsed: false,
            indent: false,
        }
    }
}

impl<L, W, F> Collapsible<L, W, F> {
    pub fn on_click<Click>(mut self, on_click: Click) -> Self
    where
        Click: FnMut(&ScopeRef<'_>) + Send + Sync + 'static,
    {
        self.on_click = Some(Box::new(on_click));
        self
    }

    pub fn can_collapse(mut self, can_collapse: bool) -> Self {
        self.can_collapse = can_collapse;
        self
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn indent(mut self, indent: bool) -> Self {
        self.indent = indent;
        self
    }
}

impl<L, W, F> Collapsible<L, W, F>
where
    L: Widget,
    W: 'static + Widget,
    F: 'static + FnOnce() -> W,
{
    /// Defers creating the inner widget until the collapsible is expanded.
    ///
    /// This is useful for very nested widgets, such as file trees
    pub fn deferred(label: L, inner: F) -> Self {
        Self {
            size: Default::default(),
            inner: Either::Right(inner),
            label,
            style: Default::default(),
            can_collapse: true,
            on_click: None,
            collapsed: false,
            indent: false,
        }
    }
}

impl<L: Widget, W: 'static + Widget, F: 'static + FnOnce() -> W> Widget for Collapsible<L, W, F> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let collapsed = scope.store(Mutable::new(self.collapsed));

        col((
            CollapsibleHeader {
                label: self.label,
                collapse: collapsed,
                style: &self.style,
                can_collapse: self.can_collapse,
                on_click: self.on_click,
            },
            CollapsibleContent {
                collapsed,
                inner: self.inner,
                indent: self.indent,
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
    on_click: Option<ClickCallback>,

    label: L,
}

impl<L: Widget> Widget for CollapsibleHeader<'_, L> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let toggle = move |scope: &ScopeRef| {
            let value = &mut *scope.read(self.collapse).lock_mut();
            *value = !*value;
        };

        let (click, double_click) = if let Some(on_click) = self.on_click {
            (on_click, Some(Box::new(toggle) as ClickCallback))
        } else {
            (Box::new(toggle) as ClickCallback, None)
        };

        InteractiveWidget::new(
            row((
                InteractiveWidget::new(CollapsibleChevron {
                    collapse: self.collapse.clone(),
                    style: self.style,
                    can_collapse: self.can_collapse,
                })
                .on_click(move |scope| {
                    let value = &mut *scope.read(self.collapse).lock_mut();
                    *value = !*value;
                }),
                self.label,
            ))
            .with_cross_align(Align::Center),
        )
        // .with_style(self.style.button)
        .on_click(click)
        .on_double_click_opt(double_click)
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

struct CollapsibleContent<W, F> {
    collapsed: WeakHandle<Mutable<bool>>,
    inner: Either<W, F>,
    indent: bool,
}

impl<W: 'static + Widget, F: 'static + FnOnce() -> W> Widget for CollapsibleContent<W, F> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let inner_size = Mutable::new(0.0);
        let mut old_size = None;

        let stream = zip_latest(scope.read(&self.collapsed).stream(), inner_size.stream());
        let mut inner = Some(self.inner);
        let mounted = scope.store(AtomicBool::new(false));
        scope.spawn_stream(stream, move |scope, (collapsed, inner_size)| {
            if inner_size == 0.0 {
                return;
            }

            if !scope.read(&mounted).load(Ordering::SeqCst) {
                return;
            }

            let new_height = if collapsed { 0.0 } else { inner_size };

            let old_size = old_size.replace(new_height).unwrap_or(new_height);
            // scope.set(max_size(), Unit::px2(f32::MAX, new_height));
            // scope.set(min_size(), Unit::px2(0.0, new_height));

            // tracing::info!(
            //     ?collapsed,
            //     ?inner_size,
            //     ?new_height,
            //     "collapsible size change"
            // );

            scope.add_tween(
                max_size(),
                Tweener::back_out(
                    Unit::px2(f32::MAX, old_size),
                    Unit::px2(f32::MAX, new_height),
                    0.2,
                ),
            );
            scope.add_tween(
                min_size(),
                Tweener::back_out(Unit::px2(0.0, old_size), Unit::px2(0.0, new_height), 0.2),
            );
        });

        scope
            .set(min_size(), Unit::px2(0.0, 0.0))
            .set(max_size(), Unit::px2(f32::MAX, f32::MAX))
            .set_default(tweens());

        scope.spawn_stream(
            scope.read(&self.collapsed).stream(),
            move |scope, collapsed| {
                if collapsed {
                    return;
                }

                if let Some(inner) = inner.take() {
                    let inner = match inner {
                        Either::Left(inner) => inner,
                        Either::Right(inner_fn) => inner_fn(),
                    };

                    Stack::new(|scope: &mut Scope<'_>| {
                        let inner_size = inner_size.clone();
                        let name = scope.entity().to_string();
                        scope.monitor(rect(), move |v| {
                            if let Some(v) = v {
                                tracing::info!("Collapsible `{name}` size: {:?}", v.size());
                                inner_size.set(v.size().y);
                            }
                        });

                        inner.mount(scope);
                    })
                    .with_clip(BVec2::new(false, true))
                    .with_padding(Edges::new(16.0 * self.indent as i32 as f32, 0.0, 0.0, 0.0))
                    .mount(scope);

                    scope.read(&mounted).store(true, Ordering::SeqCst);
                }
            },
        );
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
