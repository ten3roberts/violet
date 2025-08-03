use std::sync::Arc;

use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{vec2, Vec2, Vec3, Vec3Swizzles};
use tracing::info;

use crate::{
    components::{offset, opacity, rect, screen_transform},
    layout::Align,
    state::StateDuplex,
    style::{
        default_corner_radius, icon_chevron, icon_ellipsis, spacing_medium, surface_interactive,
        SizeExt, StyleExt,
    },
    to_owned,
    unit::Unit,
    widget::{
        card, col,
        interactive::overlay::{overlay_state, CloseOnDropHandle, Overlay},
        row, Button, ButtonStyle, IterWidgetCollection, ScrollArea, Stack, StreamWidget,
    },
    Rect, Scope, Widget,
};

pub struct Dropdown<I> {
    items: I,
    selection: Arc<dyn Send + Sync + StateDuplex<Item = Option<usize>>>,
}

impl<I> Dropdown<I>
where
    I: IntoIterator,
    I::Item: 'static + Send + Sync + Clone + Widget,
{
    /// Create a new dropdown widget.
    ///
    /// `selection` is a state duplex that will be updated with the selected item index.
    pub fn new(
        selection: impl 'static + Send + Sync + StateDuplex<Item = Option<usize>>,
        items: I,
    ) -> Self
    where
        I: IntoIterator,
        I::Item: 'static + Send + Sync + Clone + Widget,
    {
        Self {
            items,
            selection: Arc::new(selection),
        }
    }
}

impl<I> Widget for Dropdown<I>
where
    I: IntoIterator,
    I::Item: 'static + Send + Sync + Clone + Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        let overlays = scope.get_context_cloned(overlay_state());

        let items = Arc::new(self.items.into_iter().collect::<Vec<_>>());
        let current_item = self.selection.stream().map({
            to_owned!(items);
            move |v| {
                v.map(|i| {
                    Stack::new(items.get(i).cloned())
                        .with_margin(spacing_medium())
                        .with_padding(spacing_medium())
                })
            }
        });

        let current_dropdown = Mutable::new(None);

        let pick_icon = scope
            .stylesheet()
            .get_clone(icon_ellipsis())
            .unwrap_or_else(|_| "⋯".to_string());

        let screen_pos = Mutable::new(Vec2::ZERO);

        scope.monitor(screen_transform(), {
            to_owned!(screen_pos);
            move |transform| {
                if let Some(transform) = transform {
                    screen_pos.set(transform.transform_point3(Vec3::ZERO).xy());
                }
            }
        });

        let positioned_rect = Mutable::new(Rect::ZERO);
        scope.monitor(rect(), {
            to_owned!(positioned_rect);
            move |rect| {
                if let Some(&rect) = rect {
                    positioned_rect.set(rect);
                }
            }
        });

        row((
            StreamWidget::new(current_item),
            Button::label(pick_icon).on_click(move |_| {
                let rect = positioned_rect.get().translate(screen_pos.get());
                let pos = vec2(rect.min.x, rect.max.y + 4.0);
                let token = overlays.open(DropdownListOverlay {
                    position: pos,
                    items: items.clone(),
                    width: rect.size().x,
                    selection: self.selection.clone(),
                });

                current_dropdown.set(Some(CloseOnDropHandle::new(token)));
            }),
        ))
        .with_corner_radius(default_corner_radius())
        .with_background(surface_interactive())
        .with_cross_align(Align::Center)
        .mount(scope)
    }
}

struct DropdownListOverlay<T> {
    position: Vec2,
    width: f32,
    items: Arc<Vec<T>>,
    selection: Arc<dyn Send + Sync + StateDuplex<Item = Option<usize>>>,
}

impl<T: 'static + Send + Sync + Clone + Widget> Overlay for DropdownListOverlay<T> {
    fn create(self, scope: &mut Scope<'_>, token: super::overlay::OverlayHandle) {
        let selection = scope.store(self.selection);
        let token = scope.store(token);
        scope
            .set(offset(), Unit::px(self.position))
            .set(opacity(), 0.9);

        card(
            ScrollArea::vertical(
                col(IterWidgetCollection::new(
                    self.items.iter().enumerate().map(|(i, item)| {
                        Button::new(item.clone())
                            .with_style(ButtonStyle::selectable_entry())
                            .on_click(move |scope| {
                                scope.read(selection).send(Some(i));
                                scope.read(token).close();
                            })
                    }),
                ))
                .with_stretch(true),
            )
            .with_max_size(Unit::px2(self.width, 200.0)),
        )
        .with_background(surface_interactive())
        .with_min_size(Unit::px2(self.width, 0.0))
        .mount(scope);
    }
}
