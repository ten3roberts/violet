use std::{thread::Scope, time::Duration};

use cosmic_text::Wrap;
use futures::{stream, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};
use glam::vec2;
use itertools::Itertools;
use palette::{Hsla, Hsva, IntoColor, Srgba};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet::{
    components::{size_resolver, text, Edges},
    constraints::FixedAreaConstraint,
    input::{focus_sticky, focusable, on_char_typed, on_keyboard_input},
    layout::{CrossAlign, Direction},
    style::StyleExt,
    text::{FontFamily, TextSegment},
    time::interval,
    unit::Unit,
    widget::{ContainerExt, List, Rectangle, Signal, Stack, Text, WidgetExt},
    App, Widget,
};
use winit::event::{ElementState, VirtualKeyCode};

macro_rules! srgba {
    ($color:literal) => {{
        let [r, g, b] = color_hex::color_from_hex!($color);

        Srgba::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }};
}

const MARGIN: Edges = Edges::even(10.0);
const MARGIN_SM: Edges = Edges::even(5.0);

pub const EERIE_BLACK: Srgba = srgba!("#222525");
pub const EERIE_BLACK_300: Srgba = srgba!("#151616");
pub const EERIE_BLACK_400: Srgba = srgba!("#1b1e1e");
pub const EERIE_BLACK_600: Srgba = srgba!("#4c5353");
pub const PLATINUM: Srgba = srgba!("#dddddf");
pub const VIOLET: Srgba = srgba!("#8000ff");
pub const TEAL: Srgba = srgba!("#247b7b");
pub const EMERALD: Srgba = srgba!("#50c878");
pub const BRONZE: Srgba = srgba!("#cd7f32");
pub const CHILI_RED: Srgba = srgba!("#d34131");

fn pill(widget: impl Widget) -> impl Widget {
    Stack::new(widget)
        .with_background(Rectangle::new(EERIE_BLACK_300))
        .with_padding(MARGIN_SM)
        .with_margin(MARGIN_SM)
}

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

    App::new().run(MainApp)
}

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut violet::Scope<'_>) {
        let content = Mutable::new(String::new());
        List::new((
            List::new((Text::new("Input: "), TextInput::new(content))),
            ItemList,
        ))
        .with_direction(Direction::Vertical)
        .with_cross_align(CrossAlign::Center)
        .with_padding(MARGIN)
        .mount(scope)
    }
}

struct FixedArea {
    color: Srgba,
    area: f32,
}

impl FixedArea {
    fn new(color: Srgba, area: f32) -> Self {
        Self { color, area }
    }
}

impl Widget for FixedArea {
    fn mount(self, scope: &mut violet::Scope<'_>) {
        Rectangle::new(self.color)
            .with_component(
                size_resolver(),
                Box::new(FixedAreaConstraint {
                    area: self.area,
                    unit_size: 10.0,
                }),
            )
            .with_margin(MARGIN)
            .mount(scope)
    }
}

struct ItemList;

impl Widget for ItemList {
    fn mount(self, scope: &mut violet::Scope<'_>) {
        let count = 10;
        List::new(
            (0..count)
                .map(|i| {
                    let size = 100.0 + i as f32 * 10.0;
                    // Rectangle::new(Hsva::new(i as f32 * 10.0, 1.0, 1.0, 1.0).into_color())
                    Stack::new(Text::new(format!("{size}px")).with_size(Unit::px(vec2(size, 20.0))))
                        .with_background(Rectangle::new(
                            Hsva::new(i as f32 * 30.0, 0.6, 0.7, 1.0).into_color(),
                        ))
                        .with_padding(MARGIN_SM)
                        .with_margin(MARGIN_SM)
                })
                .collect::<Vec<_>>(),
        )
        .mount(scope)
    }
}

struct TextInput {
    content: Mutable<String>,
}

impl TextInput {
    fn new(content: Mutable<String>) -> Self {
        Self { content }
    }
}

impl Widget for TextInput {
    fn mount(self, scope: &mut violet::Scope<'_>) {
        scope.set(focusable(), ()).set(focus_sticky(), ());

        let content = self.content.clone();
        scope.set(
            on_char_typed(),
            Box::new(move |_, _, char| {
                if char.is_control() {
                    return;
                }

                content.lock_mut().push(char);
            }),
        );

        let content = self.content.clone();
        scope.set(
            on_keyboard_input(),
            Box::new(move |_, _, mods, input| {
                let Some(virtual_keycode) = input.virtual_keycode else {
                    return;
                };

                if input.state == ElementState::Pressed {
                    match virtual_keycode {
                        VirtualKeyCode::Back if mods.ctrl() => {
                            let mut content = content.lock_mut();
                            if let Some(last_word) =
                                content.split_inclusive([' ', '\n']).next_back()
                            {
                                let n = last_word.chars().count();
                                for _ in 0..n {
                                    content.pop();
                                }
                            }
                        }
                        VirtualKeyCode::Back => {
                            content.lock_mut().pop();
                        }
                        VirtualKeyCode::Return => {
                            content.lock_mut().push('\n');
                        }
                        _ => {}
                    }
                }
            }),
        );

        pill(Signal(self.content.signal_cloned().map(|v| {
            Text::rich([TextSegment::new(v).with_family(FontFamily::named("Inter"))])
                .with_font_size(18.0)
        })))
        .mount(scope)
    }
}
