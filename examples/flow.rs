use futures_signals::signal::{Mutable, SignalExt};
use glam::{vec2, Vec2};
use palette::{Hsva, IntoColor, Srgba};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet::{
    components::{anchor, aspect_ratio, offset, rect, screen_position, Edges},
    input::{
        focus_sticky, focusable, on_char_typed, on_cursor_move, on_keyboard_input, on_mouse_input,
    },
    layout::{CrossAlign, Direction},
    style::StyleExt,
    text::{FontFamily, TextSegment},
    unit::Unit,
    widget::{ContainerExt, List, Rectangle, Signal, Stack, Text},
    App, Frame, Widget,
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
        let value = Mutable::new(0.5);

        List::new((
            List::new((Text::new("Input: "), TextInput::new(content))),
            Stack::new((
                Movable::new(
                    Rectangle::new(EMERALD)
                        .with_size(Unit::px(vec2(15.0, 15.0)))
                        .with_component(aspect_ratio(), 1.0),
                )
                .on_move(|_, v| v.clamp(vec2(0.0, 0.0), vec2(300.0, 0.0))),
                Rectangle::new(TEAL).with_size(Unit::px(vec2(10.0, 300.0))),
                Rectangle::new(TEAL).with_size(Unit::px(vec2(300.0, 10.0))),
            )),
            ItemList,
            List::new((
                Slider::new(
                    value.clone(),
                    0.0,
                    1.0,
                    Rectangle::new(EMERALD * Srgba::new(1.0, 1.0, 1.0, 0.2))
                        .with_size(Unit::px(vec2(20.0, 20.0))),
                ),
                Signal(
                    value
                        .signal_cloned()
                        .map(|v| Text::new(format!("{:.2}", v))),
                ),
            )),
        ))
        .with_direction(Direction::Vertical)
        // .with_cross_align(CrossAlign::Center)
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
        // Rectangle::new(self.color)
        //     .with_component(
        //         size_resolver(),
        // Box::new(FixedAreaConstraint {
        //     area: self.area,
        //     unit_size: 10.0,
        // }),
        // )
        // .with_margin(MARGIN)
        // .mount(scope)
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

pub struct Movable<W> {
    content: W,
    on_move: Box<dyn Send + Sync + FnMut(&Frame, Vec2) -> Vec2>,
}

impl<W> Movable<W> {
    pub fn new(content: W) -> Self {
        Self {
            content,
            on_move: Box::new(|_, v| v),
        }
    }

    pub fn on_move(
        mut self,
        on_move: impl 'static + Send + Sync + FnMut(&Frame, Vec2) -> Vec2,
    ) -> Self {
        self.on_move = Box::new(on_move);
        self
    }
}

impl<W: Widget> Widget for Movable<W> {
    fn mount(mut self, scope: &mut violet::Scope<'_>) {
        let start_offset = Mutable::new(Vec2::ZERO);

        Stack::new(self.content)
            .with_component(focusable(), ())
            .with_component(offset(), Unit::default())
            .with_component(
                on_mouse_input(),
                Box::new({
                    let start_offset = start_offset.clone();
                    move |_, _, _, input| {
                        if input.state == ElementState::Pressed {
                            let cursor_pos = input.cursor.local_pos;
                            *start_offset.lock_mut() = cursor_pos;
                        }
                    }
                }),
            )
            .with_component(
                on_cursor_move(),
                Box::new({
                    move |frame, entity, _, input| {
                        let rect = entity.get_copy(rect()).unwrap();
                        let anchor = entity
                            .get_copy(anchor())
                            .unwrap_or_default()
                            .resolve(rect.size());

                        let cursor_pos = input.local_pos + rect.min;

                        let new_offset = cursor_pos - start_offset.get() + anchor;
                        let new_offset = (self.on_move)(frame, new_offset);
                        entity.update_dedup(offset(), Unit::px(new_offset));
                    }
                }),
            )
            .mount(scope)
    }
}

pub struct Slider<W> {
    value: Mutable<f32>,
    min: f32,
    max: f32,
    handle: W,
}

impl<W> Slider<W> {
    pub fn new(value: Mutable<f32>, min: f32, max: f32, handle: W) -> Self {
        Self {
            value,
            min,
            max,
            handle,
        }
    }
}

impl<W: Widget> Widget for Slider<W> {
    fn mount(self, scope: &mut violet::Scope<'_>) {
        Stack::new(SliderTrack {
            value: self.value,
            min: self.min,
            max: self.max,
            handle: self.handle,
        })
        .mount(scope)
    }
}

struct SliderTrack<W> {
    value: Mutable<f32>,

    min: f32,
    max: f32,
    handle: W,
}

impl<W: Widget> Widget for SliderTrack<W> {
    fn mount(self, scope: &mut violet::Scope<'_>) {
        let id = scope.attach(Rectangle::new(EERIE_BLACK).with_size(Unit::px(vec2(200.0, 10.0))));

        let on_move = move |frame: &Frame, v: Vec2| {
            let parent_rect = frame.world.get(id, rect()).unwrap();

            let pos = v.clamp(parent_rect.min * Vec2::X, parent_rect.max * Vec2::X);

            let value = (pos.x - parent_rect.min.x) / parent_rect.size().x * (self.max - self.min)
                + self.min;

            self.value.set(value);

            pos
        };

        scope.attach(
            Movable::new(self.handle)
                .on_move(on_move)
                .with_component(anchor(), Unit::rel(vec2(0.5, 0.0))),
        );

        Stack::new(())
            .with_vertical_alignment(CrossAlign::Center)
            .with_size(Unit::rel(vec2(0.0, 0.0)) + Unit::px(vec2(100.0, 0.0)))
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
            Box::new(move |_, _, _, char| {
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
