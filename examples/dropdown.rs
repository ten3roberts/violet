
use futures_signals::signal::Mutable;
use palette::Srgba;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{base_colors::*, StylesheetOptions},
    widget::{
        card, interactive::dropdown::Dropdown, label, row,
    },
    StateExt, Widget,
};
use violet_lucide::icons::{
    LUCIDE_BACKPACK, LUCIDE_BOX, LUCIDE_BRIEFCASE_BUSINESS, LUCIDE_DROPLETS, LUCIDE_HAMMER,
    LUCIDE_HEADPHONES, LUCIDE_LEAF, LUCIDE_WRENCH,
};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    let selection = Mutable::new(None);

    #[derive(Clone)]
    struct Item {
        icon: &'static str,
        color: Srgba,
        label: &'static str,
    }

    impl Widget for Item {
        fn mount(self, scope: &mut violet_core::Scope<'_>) {
            row((label(self.icon).with_color(self.color), label(self.label))).mount(scope);
        }
    }

    card(Dropdown::new(
        selection.lower_option(),
        [
            Item {
                icon: LUCIDE_BOX,
                color: SAPPHIRE_400,
                label: "Box",
            },
            Item {
                icon: LUCIDE_DROPLETS,
                color: AMETHYST_400,
                label: "Liquid",
            },
            Item {
                icon: LUCIDE_HAMMER,
                color: AMBER_400,
                label: "Tools",
            },
            Item {
                icon: LUCIDE_BACKPACK,
                color: RUBY_400,
                label: "Items",
            },
            Item {
                icon: LUCIDE_HEADPHONES,
                color: EMERALD_400,
                label: "Music",
            },
            Item {
                icon: LUCIDE_BRIEFCASE_BUSINESS,
                color: AMBER_400,
                label: "Business",
            },
            Item {
                icon: LUCIDE_WRENCH,
                color: SAPPHIRE_400,
                label: "Settings",
            },
            Item {
                icon: LUCIDE_LEAF,
                color: FOREST_400,
                label: "Nature",
            },
        ],
    ))
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
        .with_font(violet_lucide::font_source())
        .with_stylesheet(
            StylesheetOptions::new()
                .with_icons(violet_lucide::icon_set())
                .build(),
        )
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}
