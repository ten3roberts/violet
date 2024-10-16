use flax::components::name;
use futures_signals::signal::{Mutable, SignalExt};

use glam::{vec2, Vec2};
use palette::Srgba;
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;

use violet::core::{
    components,
    state::MapRef,
    style::{Background, SizeExt},
    text::Wrap,
    unit::Unit,
    widget::{card, centered, col, label, row, Rectangle, SignalWidget, Slider, Text, WidgetExt},
    Scope, Widget,
};
use violet_core::style::{
    colors::{AMBER_500, EMERALD_500, EMERALD_800, REDWOOD_500, TEAL_500},
    primary_surface,
};
use violet_wgpu::renderer::MainRendererConfig;

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
        .with_renderer_config(MainRendererConfig { debug_mode: true })
        .run(MainApp)
}

struct Vec2Editor {
    value: Mutable<Vec2>,
    x_label: String,
    y_label: String,
}

impl Vec2Editor {
    fn new(value: Mutable<Vec2>, x_label: impl Into<String>, y_label: impl Into<String>) -> Self {
        Self {
            value,
            x_label: x_label.into(),
            y_label: y_label.into(),
        }
    }
}

impl Widget for Vec2Editor {
    fn mount(self, scope: &mut Scope<'_>) {
        let value = self.value;

        let x = MapRef::new(value.clone(), |v| &v.x, |v| &mut v.x);
        let y = MapRef::new(value.clone(), |v| &v.y, |v| &mut v.y);

        col((
            row((label(self.x_label), Slider::new(x, 0.0, 200.0))),
            row((label(self.y_label), Slider::new(y, 0.0, 200.0))),
        ))
        .mount(scope)
    }
}
struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let size = Mutable::new(vec2(100.0, 100.0));

        col((
            card(col((
                Vec2Editor::new(size.clone(), "width", "height"),
                SignalWidget::new(size.signal().map(|size| label(format!("Rectangle size: {size}")))),
            ))),
            row((label("This is a row of longer text that is wrapped. When the text wraps it will take up more vertical space in the layout, and will as such increase the overall height. Another sentence for good measure to force the text to wrap"), card(Text::new(":P").with_wrap(Wrap::None)))),
            SignalWidget::new(size.signal().map(|size| FlowSizing { size })),
            // AnimatedSize,
        ))
        .contain_margins(true)
        .with_background(Background::new(primary_surface()))
        .mount(scope)
    }
}

struct FlowSizing {
    size: Vec2,
}

impl Widget for FlowSizing {
    fn mount(self, scope: &mut Scope<'_>) {
        let bg = Background::new(EMERALD_800);

        let content = (
            SizedBox::new(EMERALD_500, Unit::px(self.size)).with_name("EMERALD"),
            SizedBox::new(REDWOOD_500, Unit::px2(50.0, 40.0)).with_name("REDWOOD"),
            SizedBox::new(TEAL_500, Unit::rel2(0.0, 0.0) + Unit::px2(10.0, 50.0))
                .with_name("DARK_CYAN"),
            AnimatedSize,
        );

        col((
            row((
                card(col((
                    label("Unconstrained list"),
                    row(content.clone()).with_background(bg),
                ))),
                card(col((
                    label("Constrained list with min size"),
                    row(content.clone())
                        .with_background(bg)
                        .with_min_size(Unit::px2(100.0, 100.0)),
                ))),
                card(col((
                    label("Constrained list with max size"),
                    row(content.clone())
                        .with_background(bg)
                        .with_max_size(Unit::px2(100.0, 100.0)),
                ))),
                card(col((
                    label("Constrained list with max size"),
                    row(content.clone())
                        .with_background(bg)
                        .with_min_size(Unit::px2(100.0, 100.0))
                        .with_max_size(Unit::px2(100.0, 100.0)),
                ))),
            )),
            row((
                card(col((
                    label("Unconstrained stack"),
                    centered(content.clone()).with_background(bg),
                ))),
                card(col((
                    label("Constrained stack with min size"),
                    centered(content.clone())
                        .with_background(bg)
                        .with_min_size(Unit::px2(100.0, 100.0)),
                ))),
                card(col((
                    label("Constrained stack with max size"),
                    centered(content.clone())
                        .with_background(bg)
                        .with_max_size(Unit::px2(100.0, 100.0)),
                ))),
                card(col((
                    label("Constrained stack with max size"),
                    centered(content.clone())
                        .with_background(bg)
                        .with_min_size(Unit::px2(100.0, 100.0))
                        .with_max_size(Unit::px2(100.0, 100.0)),
                ))),
            )),
        ))
        .mount(scope)
    }
}

#[derive(Debug, Clone)]
struct SizedBox {
    color: Srgba,
    size: Unit<Vec2>,
}

impl SizedBox {
    fn new(color: Srgba, size: Unit<Vec2>) -> Self {
        Self { color, size }
    }
}

impl Widget for SizedBox {
    fn mount(self, scope: &mut Scope<'_>) {
        // Stack::new((
        Rectangle::new(self.color)
            .with_size(self.size)
            //     column((
            //         Text::new(format!("{}", self.size.px)),
            //         Text::new(format!("{}", self.size.rel)),
            //     )),
            // ))
            .mount(scope)
    }
}

#[derive(Debug, Clone)]
pub struct AnimatedSize;

impl Widget for AnimatedSize {
    fn mount(self, scope: &mut Scope<'_>) {
        scope.set(name(), "AnimatedBox".into());
        scope.set(
            components::on_animation_frame(),
            Box::new(move |_, entity, t| {
                let t = t.as_secs_f32();

                let size = vec2(t.sin() * 50.0, (t * 2.5).cos() * 50.0) + vec2(100.0, 100.0);
                entity.update_dedup(components::size(), Unit::px(size));
            }),
        );

        Rectangle::new(AMBER_500)
            .with_size(Default::default())
            .mount(scope)
    }
}
