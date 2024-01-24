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
    layout::Direction,
    style::StyleExt,
    text::TextSegment,
    time::interval,
    unit::Unit,
    widget::{ContainerExt, List, Rectangle, Signal, Stack, Text, WidgetExt},
    App, Widget,
};

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
        List::new((List::new((
            List::new((
                Rectangle::new(EMERALD).with_size(Unit::px(vec2(100.0, 100.0))),
                FixedArea::new(EMERALD, 100.0).with_name("a"),
                FixedArea::new(TEAL, 200.0).with_name("b"),
            )),
            FixedArea::new(CHILI_RED, 200.0).with_name("c"),
        ))
        .with_direction(Direction::Vertical),))
        .with_padding(Edges::even(50.0))
        .mount(scope)

        // List::new((
        //     List::new((
        //         Rectangle::new(PLATINUM)
        //             .with_min_size(Unit::px(vec2(100.0, 50.0)))
        //             .with_margin(MARGIN),
        //         List::new((
        //             FixedArea::new(EMERALD, 10000.0),
        //             FixedArea::new(CHILI_RED, 20000.0),
        //         ))
        //         .with_direction(Direction::Vertical),
        //     ))
        //     .with_stretch(true),
        //     Rectangle::new(TEAL)
        //         .with_min_size(Unit::px(vec2(300.0, 50.0)))
        //         .with_margin(MARGIN),
        //     // ItemList,
        // ))
        // .with_direction(Direction::Vertical)
        // .with_padding(Edges::even(50.0))
        // .mount(scope)
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
        List::new(
            (0..10)
                .map(|i| {
                    let size = 100.0 + i as f32 * 10.0;
                    // Rectangle::new(Hsva::new(i as f32 * 10.0, 1.0, 1.0, 1.0).into_color())
                    pill(Text::new(format!("{size}px")).with_size(Unit::px(vec2(size, 20.0))))
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

        self.content.lock_mut().push_str("Lorem ipsum dolor sit amet, qui minim labore adipisicing minim sint cillum sint consectetur cupidatat.");
        let dup = scope.attach(
            Text::new("This is another piece of text that is a bit shorter.").with_margin(MARGIN),
        );

        // let content = self.content.clone();
        // scope.set(
        //     on_char_typed(),
        //     Box::new(move |frame, _, char| {
        //         content.lock_mut().push(char);
        //         frame.world.entity(dup).unwrap().update(text(), |v| {
        //             *v = vec![TextSegment::new(content.lock_mut().to_string())]
        //         });
        //     }),
        // );

        // let content = self.content.clone();
        // scope.set(
        //     on_keyboard_input(),
        //     Box::new(move |frame, _, input| {
        //         match input.virtual_keycode {
        //             Some(winit::event::VirtualKeyCode::Back) => {
        //                 content.lock_mut().pop();
        //             }
        //             Some(winit::event::VirtualKeyCode::Return) => {
        //                 content.lock_mut().push('\n');
        //             }
        //             _ => {}
        //         }

        //         frame.world.entity(dup).unwrap().update(text(), |v| {
        //             *v = vec![TextSegment::new(content.lock_mut().to_string())]
        //         });
        //     }),
        // );

        // let content = self.content.clone();
        // scope.spawn_async(
        //     interval(Duration::from_millis(50))
        //         .zip(stream::iter("Lorem ipsum dolor sit amet, officia excepteur ex fugiat reprehenderit enim labore culpa sint ad nisi Lorem pariatur mollit ex esse exercitation amet. Nisi anim cupidatat excepteur officia. Reprehenderit nostrud nostrud ipsum Lorem est aliquip amet voluptate voluptate dolor minim nulla est proident. Nostrud officia pariatur ut officia. Sit irure elit esse ea nulla sunt ex occaecat reprehenderit commodo officia dolor Lorem duis laboris cupidatat officia voluptate. Culpa proident adipisicing id nulla nisi laboris ex in Lorem sunt duis officia eiusmod. Aliqua reprehenderit commodo ex non excepteur duis sunt velit enim. Voluptate laboris sint cupidatat ullamco ut ea consectetur et est culpa et culpa duis.".chars()))
        //         .for_each(move |(_, c)| {
        //             tracing::info!(?c, "sending char");
        //             content.lock_mut().push(c);
        //             async {}
        //         }),
        // );

        List::new(pill(Signal(
            self.content
                .signal_cloned()
                .map(|v| Text::new(v).with_wrap(Wrap::Word)),
        )))
        .with_direction(Direction::Vertical)
        .mount(scope)
    }
}
