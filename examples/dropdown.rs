use futures_signals::signal::Mutable;
use palette::Srgba;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::base_colors::*,
    widget::{card, icon, interactive::dropdown::Dropdown, label, row},
    StateExt, Widget,
};
use violet_lucide::LucideIcon;
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    let selection = Mutable::new(None);

    #[derive(Clone)]
    struct Item {
        icon: LucideIcon,
        color: Srgba,
        label: &'static str,
    }

    impl Widget for Item {
        fn mount(self, scope: &mut violet_core::Scope<'_>) {
            row((icon(self.icon).with_color(self.color), label(self.label))).mount(scope);
        }
    }

    card(Dropdown::new(
        selection.lower_option(),
        [
            Item {
                icon: LucideIcon::Box,
                color: SAPPHIRE_400,
                label: "Box",
            },
            Item {
                icon: LucideIcon::Droplets,
                color: AMETHYST_400,
                label: "Liquid",
            },
            Item {
                icon: LucideIcon::Hammer,
                color: AMBER_400,
                label: "Tools",
            },
            Item {
                icon: LucideIcon::Backpack,
                color: RUBY_400,
                label: "Items",
            },
            Item {
                icon: LucideIcon::Headphones,
                color: EMERALD_400,
                label: "Music",
            },
            Item {
                icon: LucideIcon::BriefcaseBusiness,
                color: AMBER_400,
                label: "Business",
            },
            Item {
                icon: LucideIcon::Wrench,
                color: SAPPHIRE_400,
                label: "Settings",
            },
            Item {
                icon: LucideIcon::Leaf,
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
        .with_icon_pack(violet_lucide::pack())
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}
