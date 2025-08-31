use itertools::Itertools;
use violet::core::style::{base_colors::*, default_corner_radius};
use violet::core::{
    style::{spacing_small, SizeExt},
    unit::Unit,
    widget::{
        drop_target, interactive::tooltip::Tooltip, label, row, Draggable, Rectangle, SignalWidget,
    },
    Scope, Widget,
};
use violet::flax::component;
use violet::futures_signals::signal::Mutable;
use violet::palette::{Srgba, WithAlpha};

component! {
    tile_index: usize,
}

fn draggable_tile(
    items: Mutable<Vec<(Srgba, String)>>,
    index: usize,
    color: Srgba,
    name: String,
) -> impl Widget {
    move |scope: &mut Scope<'_>| {
        scope.set(tile_index(), index).set_default(drop_target());

        Tooltip::new(
            Draggable::new(
                Rectangle::new(color)
                    .with_margin(spacing_small())
                    .with_exact_size(Unit::px2(48.0, 48.0))
                    .with_corner_radius(default_corner_radius()),
                move || {
                    Rectangle::new(color.with_alpha(0.5))
                        .with_exact_size(Unit::px2(48.0, 48.0))
                        .with_corner_radius(default_corner_radius())
                },
                move |_, drop_target| {
                    if let Some(target_index) =
                        drop_target.and_then(|v| v.0.get_copy(tile_index()).ok())
                    {
                        items.lock_mut().swap(index, target_index);
                    }
                },
            ),
            move |_| label(&name),
        )
        .mount(scope);
    }
}

pub fn app() -> impl Widget {
    let items = Mutable::new(vec![
        (EMERALD_50, "Emerald 50".to_string()),
        (EMERALD_100, "Emerald 100".to_string()),
        (EMERALD_200, "Emerald 200".to_string()),
        (EMERALD_300, "Emerald 300".to_string()),
        (EMERALD_400, "Emerald 400".to_string()),
        (EMERALD_500, "Emerald 500".to_string()),
        (EMERALD_600, "Emerald 600".to_string()),
        (EMERALD_700, "Emerald 700".to_string()),
        (EMERALD_800, "Emerald 800".to_string()),
        (EMERALD_900, "Emerald 900".to_string()),
        (EMERALD_950, "Emerald 950".to_string()),
    ]);

    SignalWidget::new(items.clone().signal_ref(move |v| {
        row(v
            .iter()
            .enumerate()
            .map(|(i, item)| draggable_tile(items.clone(), i, item.0, item.1.clone()))
            .collect_vec())
    }))
}
