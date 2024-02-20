use std::usize;

use futures_signals::{
    map_ref,
    signal::{self, Mutable, SignalExt},
};

use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{Hsva, IntoColor, Srgba};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;

use futures::stream::StreamExt;
use violet::core::{
    components::{self, screen_rect, Edges, Rect},
    editor::{self, EditAction, EditorAction, TextEditor},
    input::{focusable, on_char_typed, on_keyboard_input, on_mouse_input},
    layout::{CrossAlign, Direction},
    style::StyleExt,
    text::{LayoutGlyphs, TextSegment},
    to_owned,
    unit::Unit,
    widget::{List, NoOp, Rectangle, Signal, Stack, Text, WidgetExt},
    Scope, Widget,
};
use violet_core::{
    input::{focus_sticky, ElementState, VirtualKeyCode},
    style::Background,
    widget::{BoxSized, Button, ContainerStyle, Positioned, SliderWithLabel},
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
    Stack::new(widget).with_style(ContainerStyle {
        background: Some(Background::new(EERIE_BLACK_300)),
        padding: MARGIN_SM,
        margin: MARGIN_SM,
    })
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

    violet_wgpu::App::new().run(MainApp)
}

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let content = Mutable::new(
            "This is a multiline text that is wrapped around because it is so long".into(),
        );
        let value = Mutable::new(1.0f32);
        let count = Mutable::new(5);

        let scale = value.signal();

        let item_list = Box::new(map_ref! {scale, let count = count.signal() => ItemList {
            scale: scale.round(),
            count: *count,
        }});

        List::new((
            List::new((Text::new("Input: "), TextInput::new(content))).with_style(ContainerStyle {
                margin: MARGIN_SM,
                padding: MARGIN_SM,
                ..Default::default()
            }),
            Button::new(Text::new("Button")),
            List::new((
                List::new((Text::new("Size"), SliderWithLabel::new(value, 0.0, 20.0))),
                List::new((Text::new("Count"), SliderWithLabel::new(count, 4, 20))),
            ))
            .with_direction(Direction::Vertical)
            .with_style(ContainerStyle {
                padding: MARGIN_SM,
                margin: MARGIN_SM,
                ..Default::default()
            }),
            Signal::new(item_list),
            BoxSized::new(Rectangle::new(EERIE_BLACK_600))
                .with_size(Unit::rel2(1.0, 0.0) + Unit::px2(0.0, 1.0)),
        ))
        .with_direction(Direction::Vertical)
        .with_style(ContainerStyle {
            padding: MARGIN,
            ..Default::default()
        })
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
                    let size = 10.0 + i as f32 * self.scale;
                    BoxSized::new(
                        Stack::new(Text::new(format!("{size}px")))
                            .with_style(ContainerStyle {
                                background: Some(Background::new(
                                    Hsva::new(i as f32 * 30.0, 0.6, 0.7, 1.0).into_color(),
                                )),
                                padding: MARGIN_SM,
                                margin: MARGIN_SM,
                            })
                            .with_vertical_alignment(CrossAlign::Center)
                            .with_horizontal_alignment(CrossAlign::Center),
                    )
                    .with_size(Unit::px2(size, size))
                })
                .collect::<Vec<_>>(),
        )
        .with_cross_align(CrossAlign::Center)
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
    fn mount(self, scope: &mut Scope<'_>) {
        let (tx, rx) = flume::unbounded();

        let content = self.content.clone();

        let mut editor = TextEditor::new();

        let layout_glyphs = Mutable::new(None);
        let text_bounds: Mutable<Option<Rect>> = Mutable::new(None);

        editor.set_text(content.lock_mut().split('\n').map(ToOwned::to_owned));
        editor.set_cursor_at_end();

        let (editor_props_tx, editor_props_rx) = signal::channel(Box::new(NoOp) as Box<dyn Widget>);

        scope.spawn({
            let mut layout_glyphs = layout_glyphs.signal_cloned().to_stream();
            async move {
                let mut rx = rx.into_stream();

                let mut glyphs: LayoutGlyphs;

                let mut cursor_pos = Vec2::ZERO;

                loop {
                    tokio::select! {
                        Some(action) = rx.next() => {

                            editor.apply_action(action);

                            let mut c = content.lock_mut();
                            c.clear();
                            for line in editor.lines() {
                                c.push_str(line.text());
                                c.push('\n');
                            }
                        }
                        Some(Some(new_glyphs)) = layout_glyphs.next() => {
                            glyphs = new_glyphs;
                            tracing::info!("{:?}", glyphs.lines().iter().map(|v| v.glyphs.len()).collect_vec());

                            if let Some(loc) = glyphs.to_glyph_boundary(editor.cursor()) {
                                cursor_pos = loc;
                            } else if editor.past_eol() {
                                cursor_pos = glyphs
                                    .find_lines_indices(editor.cursor().row)
                                    .last()
                                    .map(|(ln, line)| {
                                        vec2(line.bounds.max.x, ln as f32 * glyphs.line_height())
                                    })
                                    .unwrap_or_default();
                            } else {
                                cursor_pos = Vec2::ZERO;
                            }
                        }
                        else => break,
                    }

                    editor_props_tx
                        .send(Box::new(Stack::new(
                                    (
                                        Positioned::new(BoxSized::new(Rectangle::new(EMERALD))
                                        .with_size(Unit::px2(2.0, 18.0)))
                                        .with_offset(Unit::px(cursor_pos)),
                                    )
                        )))
                        .ok();
                }
            }
        });

        scope
            .set(focusable(), ())
            .set(focus_sticky(), ())
            .on_event(on_mouse_input(), {
                to_owned![layout_glyphs, text_bounds, tx];
                move |_, _, input| {
                    let glyphs = layout_glyphs.lock_ref();

                    if let (Some(glyphs), Some(text_bounds)) = (&*glyphs, &*text_bounds.lock_ref())
                    {
                        if input.state == ElementState::Pressed {
                            let text_pos = input.cursor.absolute_pos - text_bounds.min;
                            if let Some(hit) = glyphs.hit(text_pos) {
                                tracing::info!(?hit, "hit");
                                tx.send(EditorAction::CursorMove(editor::CursorMove::SetPosition(
                                    hit,
                                )))
                                .ok();
                            }

                            tracing::info!(?input, "click");
                        }
                    }
                }
            })
            .on_event(on_char_typed(), {
                to_owned![tx];
                move |_, _, char| {
                    if char.is_control() {
                        return;
                    }

                    tx.send(EditorAction::Edit(EditAction::InsertChar(char)))
                        .ok();
                }
            })
            .on_event(on_keyboard_input(), {
                to_owned![tx];
                move |_, _, input| {
                    let ctrl = input.modifiers.ctrl();
                    if input.state == ElementState::Pressed {
                        match input.keycode {
                            VirtualKeyCode::Back if ctrl => {
                                tx.send(EditorAction::Edit(EditAction::DeleteBackwardWord))
                                    .ok();
                            }
                            VirtualKeyCode::Back => {
                                tx.send(EditorAction::Edit(EditAction::DeleteBackwardChar))
                                    .ok();
                            }
                            VirtualKeyCode::Return => {
                                tx.send(EditorAction::Edit(EditAction::InsertLine)).ok();
                            }
                            VirtualKeyCode::Left if ctrl => {
                                tx.send(EditorAction::CursorMove(editor::CursorMove::BackwardWord))
                                    .ok();
                            }
                            VirtualKeyCode::Right if ctrl => {
                                tx.send(EditorAction::CursorMove(editor::CursorMove::ForwardWord))
                                    .ok();
                            }
                            VirtualKeyCode::Left => {
                                tx.send(EditorAction::CursorMove(editor::CursorMove::Left))
                                    .ok();
                            }
                            VirtualKeyCode::Right => {
                                tx.send(EditorAction::CursorMove(editor::CursorMove::Right))
                                    .ok();
                            }
                            VirtualKeyCode::Up => {
                                tx.send(EditorAction::CursorMove(editor::CursorMove::Up))
                                    .ok();
                            }
                            VirtualKeyCode::Down => {
                                tx.send(EditorAction::CursorMove(editor::CursorMove::Down))
                                    .ok();
                            }
                            _ => {}
                        }
                    }
                }
            });

        pill(Stack::new((
            Signal(self.content.signal_cloned().map(move |v| {
                to_owned![text_bounds];
                Text::rich([TextSegment::new(v)])
                    .with_font_size(18.0)
                    .monitor_signal(components::layout_glyphs(), layout_glyphs.clone())
                    .monitor_signal(screen_rect(), text_bounds.clone())
            })),
            Signal(editor_props_rx),
        )))
        .mount(scope)
    }
}
