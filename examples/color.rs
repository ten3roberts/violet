use futures::StreamExt;
use futures_signals::signal::{Mutable, SignalExt};
use glam::Vec3;
use itertools::Itertools;
use palette::{FromColor, IntoColor, Oklch, Srgb};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    state::{Map, MapRef, StateStream, StateStreamRef},
    style::{SizeExt, ValueOrRef},
    unit::Unit,
    utils::zip_latest,
    widget::{
        card, col, row, Rectangle, SignalWidget, SliderWithLabel, Stack, StreamWidget, Text,
    },
    Edges, Scope, Widget,
};
use violet_wgpu::renderer::RendererConfig;

pub fn main() -> anyhow::Result<()> {
    registry()
        .with(
            HierarchicalLayer::default()
                .with_deferred_spans(true)
                .with_span_retrace(true)
                .with_indent_lines(true)
                .with_indent_amount(4),
        )
        .with(EnvFilter::from_default_env())
        .init();

    violet_wgpu::AppBuilder::new()
        .with_renderer_config(RendererConfig { debug_mode: false })
        .run(MainApp)
}

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let color = Mutable::new(Vec3::new(0.5, 0.27, 153.0));
        let color_oklch = Map::new(
            color.clone(),
            |v| Oklch::new(v.x, v.y, v.z),
            |v| Vec3::new(v.l, v.chroma, v.hue.into_positive_degrees()),
        );

        let lightness = MapRef::new(color.clone(), |v| &v.x, |v| &mut v.x);
        let chroma = MapRef::new(color.clone(), |v| &v.y, |v| &mut v.y);
        let hue = MapRef::new(color.clone(), |v| &v.z, |v| &mut v.z);

        let color_rect = color.signal().map(|v| {
            let color = Oklch::new(v.x, v.y, v.z).into_color();
            Rectangle::new(ValueOrRef::value(color)).with_min_size(Unit::px2(200.0, 100.0))
        });

        let falloff = Mutable::new(50.0);

        card(
            col((
                row((
                    Text::new("Lightness"),
                    SliderWithLabel::new(lightness, 0.0, 1.0)
                        .editable(true)
                        .round(0.01),
                )),
                row((
                    Text::new("Chroma"),
                    SliderWithLabel::new(chroma, 0.0, 0.37)
                        .editable(true)
                        .round(0.005),
                )),
                row((
                    Text::new("Hue"),
                    SliderWithLabel::new(hue, 0.0, 360.0)
                        .editable(true)
                        .round(1.0),
                )),
                StreamWidget(color.stream_ref(|v| {
                    let hex: Srgb<u8> = Srgb::from_color(Oklch::new(v.x, v.y, v.z)).into_format();
                    Text::new(format!(
                        "#{:0>2x}{:0>2x}{:0>2x}",
                        hex.red, hex.green, hex.blue
                    ))
                })),
                SignalWidget(color.signal().map(|v| Text::new(format!("{}", v)))),
                SignalWidget(color_rect),
                row((
                    Text::new("Chroma falloff"),
                    SliderWithLabel::new(falloff.clone(), 0.0, 100.0)
                        .editable(true)
                        .round(1.0),
                )),
                StreamWidget(
                    zip_latest(color_oklch.stream(), falloff.stream())
                        .map(|(color, falloff)| Tints::new(color, falloff)),
                ),
            ))
            .with_stretch(true)
            .with_margin(Edges::even(4.0)),
        )
        .with_size(Unit::rel2(1.0, 1.0))
        .mount(scope);
    }
}

struct Tints {
    base: Oklch,
    falloff: f32,
}

impl Tints {
    fn new(base: Oklch, falloff: f32) -> Self {
        Self { base, falloff }
    }
}

impl Widget for Tints {
    fn mount(self, scope: &mut Scope<'_>) {
        row((1..=9)
            .map(|i| {
                let f = (i as f32) / 10.0;
                let chroma = self.base.chroma * (1.0 / (1.0 + self.falloff * (f - 0.5).powi(2)));

                // let color = self.base.lighten(f);
                let color = Oklch {
                    chroma,
                    l: f,
                    ..self.base
                };

                Stack::new(col((
                    Rectangle::new(ValueOrRef::value(color.into_color()))
                        .with_min_size(Unit::px2(60.0, 60.0)),
                    Text::new(format!("{:.2}", f)),
                )))
                .with_margin(Edges::even(4.0))
            })
            .collect_vec())
        .mount(scope)
    }
}
