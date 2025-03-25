use flax::component;
use futures_signals::signal::Mutable;
use itertools::Itertools;
use palette::{Srgba, WithAlpha};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{base_colors::*, spacing_small, SizeExt},
    unit::Unit,
    widget::{
        card, col, drop_target, row, Draggable, OverlayStack, Rectangle, SignalWidget, Stack,
    },
    Scope, Widget,
};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

component! {
    tile_index: usize,
}

fn draggable_tile(items: Mutable<Vec<Srgba>>, index: usize, color: Srgba) -> impl Widget {
    move |scope: &mut Scope<'_>| {
        scope.set(tile_index(), index).set_default(drop_target());

        Draggable::new(
            Rectangle::new(color)
                .with_margin(spacing_small())
                .with_exact_size(Unit::px2(64.0, 64.0)),
            move || Rectangle::new(color.with_alpha(0.5)).with_exact_size(Unit::px2(64.0, 64.0)),
            move |_, drop_target| {
                if let Some(target_index) = drop_target.and_then(|v| v.get_copy(tile_index()).ok())
                {
                    items.lock_mut().swap(index, target_index);
                }
            },
        )
        .mount(scope);
    }
}

fn app() -> impl Widget {
    let items = Mutable::new(vec![
        EMERALD_50,
        EMERALD_100,
        EMERALD_200,
        EMERALD_300,
        EMERALD_400,
        EMERALD_500,
        EMERALD_600,
        EMERALD_700,
        EMERALD_800,
        EMERALD_900,
        EMERALD_950,
    ]);

    col(card(SignalWidget::new(items.clone().signal_ref(
        move |v| {
            row(v
                .iter()
                .enumerate()
                .map(|(i, &item)| draggable_tile(items.clone(), i, item))
                .collect_vec())
        },
    ))))
    .with_contain_margins(true)
}

pub fn main() -> anyhow::Result<()> {
    registry()
        .with(
            HierarchicalLayer::default()
                .with_deferred_spans(true)
                .with_span_retrace(true)
                .with_indent_lines(true),
        )
        .with(EnvFilter::from_default_env())
        .init();

    AppBuilder::new()
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}
