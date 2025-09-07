use std::sync::Arc;

use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{vec2, Vec2, Vec3, Vec3Swizzles};
use palette::{Srgba, WithAlpha};

use crate::{
    components::{offset, opacity, rect, screen_transform},
    layout::Align,
    state::StateDuplex,
    style::{
        base_colors::EMERALD_500, default_corner_radius, icon_ellipsis, spacing_medium,
        surface_interactive, SizeExt, StyleExt,
    },
    to_owned,
    unit::Unit,
    widget::{
        bold, card, col,
        interactive::{
            overlay::{overlay_state, CloseOnDropHandle, Overlay},
            InteractiveWidget,
        },
        label, maximized, row, Button, ButtonStyle, IterWidgetCollection, ScrollArea, Stack,
        StreamWidget,
    },
    Rect, Scope, Widget,
};

pub struct Dropdown<T, I> {
    items: I,
    selection: Arc<dyn Send + Sync + StateDuplex<Item = T>>,
}

impl<T, I> Dropdown<T, I>
where
    I: IntoIterator<Item = T>,
    T: 'static + Send + Sync + Clone + Widget,
    Self: Widget,
{
    /// Create a new dropdown widget.
    ///
    /// `selection` is a state duplex that will be updated with the selected item index.
    pub fn new(selection: impl 'static + Send + Sync + StateDuplex<Item = T>, items: I) -> Self {
        Self {
            items,
            selection: Arc::new(selection),
        }
    }
}

impl<T, I> Widget for Dropdown<T, I>
where
    I: IntoIterator<Item = T>,
    T: 'static + Send + Sync + Clone + Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        let overlays = scope.get_context_cloned(overlay_state());

        let items = Arc::new(self.items.into_iter().collect::<Vec<_>>());
        let current_item = self.selection.stream();

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

        Button::new(row((StreamWidget::new(current_item), label(pick_icon))).center())
            .on_click(move |_| {
                let rect = positioned_rect.get().translate(screen_pos.get());
                let pos = vec2(rect.min.x, rect.max.y + 4.0);
                let token = overlays.open(DropdownListOverlay {
                    position: pos,
                    items: items.clone(),
                    width: rect.size().x,
                    selection: self.selection.clone(),
                });

                current_dropdown.set(Some(CloseOnDropHandle::new(token)));
            })
            .mount(scope)
    }
}

struct DropdownListOverlay<T> {
    position: Vec2,
    width: f32,
    items: Arc<Vec<T>>,
    selection: Arc<dyn Send + Sync + StateDuplex<Item = T>>,
}

impl<T: 'static + Send + Sync + Clone + Widget> Overlay for DropdownListOverlay<T> {
    fn create(self, scope: &mut Scope<'_>, token: super::overlay::OverlayHandle) {
        let selection = scope.store(self.selection);
        let token = scope.store(token);

        let menu = |scope: &mut Scope| {
            scope
                .set(offset(), Unit::px(self.position))
                .set(opacity(), 0.9);

            card(
                ScrollArea::vertical(
                    col(IterWidgetCollection::new(
                        self.items.iter().enumerate().map(|(i, item)| {
                            to_owned!(items = self.items);
                            Button::new(item.clone())
                                .with_style(ButtonStyle::selectable_entry())
                                .on_click(move |scope| {
                                    scope.read(selection).send(items[i].clone());
                                    scope.read(token).close();
                                })
                        }),
                    ))
                    .with_stretch(true),
                )
                .with_max_size(Unit::px2(self.width, 100.0)),
            )
            .with_background(surface_interactive())
            .with_min_size(Unit::px2(self.width, 0.0))
            .mount(scope);
        };

        InteractiveWidget::new(maximized(menu))
            .on_generic_mouse_input(move |scope, input| {
                scope.read(token).close();
                Some(input)
            })
            .mount(scope);
    }
}
