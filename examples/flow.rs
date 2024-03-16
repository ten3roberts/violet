use std::usize;

use futures_signals::{map_ref, signal::Mutable};

use itertools::Itertools;
use palette::{FromColor, Hsva, IntoColor, Oklcha, Srgba};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;

use violet::core::{
    layout::Alignment,
    style::StyleExt,
    unit::Unit,
    widget::{List, Rectangle, SignalWidget, Stack, Text},
    Scope, Widget,
};
use violet_core::{
    style::{
        self,
        colors::{EERIE_BLACK_600, EERIE_BLACK_DEFAULT},
        spacing_small, Background, SizeExt,
    },
    text::Wrap,
    widget::{card, column, label, row, Button, ButtonStyle, SliderWithLabel, TextInput},
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

    violet_wgpu::App::new()
        .with_renderer_config(RendererConfig { debug_mode: false })
        .run(MainApp)
}

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let content = Mutable::new(
            "This is a multiline text that is wrapped around because it is so long".into(),
        );
        let value = Mutable::new(24.0f32);
        let count = Mutable::new(8);

        let scale = value.signal();

        let item_list = Box::new(map_ref! {scale, let count = count.signal() => ItemList {
            scale: scale.round(),
            count: *count,
        }});

        column((
            row((Text::new("Input: "), TextInput::new(content))),
            card(
                column((
                    Button::label("Button"),
                    Button::label("Button").with_style(ButtonStyle {
                        normal_color: style::success_item().into(),
                        ..Default::default()
                    }),
                    Button::label("Warning").with_style(ButtonStyle {
                        normal_color: style::warning_item().into(),
                        ..Default::default()
                    }),
                    Button::label("Error").with_style(ButtonStyle {
                        normal_color: style::danger_item().into(),
                        ..Default::default()
                    }),
                ))
                .with_stretch(true),
            ),
            Rectangle::new(EERIE_BLACK_600).with_size(Unit::rel2(1.0, 0.0) + Unit::px2(0.0, 1.0)),
            card(column((
                column((
                    row((
                        Text::new("Size"),
                        SliderWithLabel::new(value, 20.0, 200.0).editable(true),
                    )),
                    row((
                        Text::new("Count"),
                        SliderWithLabel::new(count, 1, 2).editable(true),
                    )),
                )),
                SignalWidget::new(item_list),
            ))),
            column(
                [
                    // EERIE_BLACK_DEFAULT,
                    // PLATINUM_DEFAULT,
                    // JADE_DEFAULT,
                    // DARK_CYAN_DEFAULT,
                    // ULTRA_VIOLET_DEFAULT,
                    // LION_DEFAULT,
                    // REDWOOD_DEFAULT,
                ]
                .into_iter()
                .map(|color| Tints { color })
                .collect_vec(),
            ),
        ))
        .with_background(Background::new(EERIE_BLACK_DEFAULT))
        .contain_margins(true)
        .mount(scope)
    }
}

struct Tints {
    color: Srgba,
}

impl Widget for Tints {
    fn mount(self, scope: &mut Scope<'_>) {
        row((0..=10)
            .map(|i| {
                let tint = i * 100;
                let color = style::tint(self.color, tint);
                let color_bytes: Srgba<u8> = color.into_format();
                let color_string = format!(
                    "#{:02x}{:02x}{:02x}",
                    color_bytes.red, color_bytes.green, color_bytes.blue
                );

                card(column((
                    Rectangle::new(color).with_size(Unit::px2(100.0, 40.0)),
                    label(format!("{tint}")),
                    label(color_string),
                )))
            })
            .collect_vec())
        .mount(scope)
    }
}

struct ItemList {
    scale: f32,
    count: usize,
}

impl Widget for ItemList {
    fn mount(self, scope: &mut Scope<'_>) {
        List::new(
            (0..self.count)
                .map(|i| {
                    let size = self.scale;
                    let color: Srgba = Hsva::new(i as f32 * 30.0, 0.6, 0.7, 1.0).into_color();
                    let oklch = Oklcha::from_color(color);

                    Stack::new(
                        Text::new(format!(
                            "{},{},{}",
                            oklch.l.round(),
                            oklch.chroma.round(),
                            oklch.hue.into_positive_degrees().round()
                        ))
                        .with_wrap(Wrap::None),
                    )
                    .with_background(Background::new(Srgba::from_color(Hsva::new(
                        i as f32 * 30.0,
                        0.6,
                        0.7,
                        1.0,
                    ))))
                    .with_padding(spacing_small())
                    .with_margin(spacing_small())
                    // .with_cross_align(Alignment::Center)
                    .with_vertical_alignment(Alignment::Center)
                    .with_horizontal_alignment(Alignment::Center)
                    .with_size(Unit::px2(size, size))
                    .with_max_size(Unit::px2(size, size))
                })
                .collect::<Vec<_>>(),
        )
        .with_cross_align(Alignment::Center)
        .mount(scope)
    }
}
