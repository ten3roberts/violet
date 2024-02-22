use std::usize;

use futures_signals::{
    map_ref,
    signal::{self, Mutable, SignalExt},
    signal_map::MutableSignalMap,
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
    layout::{Alignment, Direction},
    style::StyleExt,
    text::{LayoutGlyphs, TextSegment},
    to_owned,
    unit::Unit,
    widget::{List, NoOp, Rectangle, Signal, Stack, Text, WidgetExt},
    Scope, Widget,
};
use violet_core::{
    input::{focus_sticky, ElementState, VirtualKeyCode},
    style::{
        self,
        colors::{
            DARK_CYAN_DEFAULT, EERIE_BLACK_300, EERIE_BLACK_400, EERIE_BLACK_600,
            EERIE_BLACK_DEFAULT, JADE_DEFAULT, LION_DEFAULT, PLATINUM_DEFAULT, REDWOOD_DEFAULT,
            ULTRA_VIOLET_DEFAULT,
        },
        Background,
    },
    widget::{BoxSized, Button, ButtonStyle, ContainerStyle, Positioned, SliderWithLabel},
    WidgetCollection,
};

const MARGIN: Edges = Edges::even(8.0);
const MARGIN_SM: Edges = Edges::even(4.0);

fn label(text: impl Into<String>) -> Stack<Text> {
    Stack::new(Text::new(text.into()))
        .with_padding(MARGIN_SM)
        .with_margin(MARGIN_SM)
}

fn row<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Horizontal)
}

fn column<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Vertical)
}

fn card<W>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_background(Background::new(EERIE_BLACK_400))
        .with_padding(MARGIN)
        .with_margin(MARGIN)
}

fn pill(widget: impl Widget) -> impl Widget {
    Stack::new(widget).with_style(ContainerStyle {
        background: Some(Background::new(EERIE_BLACK_300)),
        padding: MARGIN,
        margin: MARGIN,
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

        column((
            row((Text::new("Input: "), TextInput::new(content))).with_style(ContainerStyle {
                margin: MARGIN_SM,
                padding: MARGIN_SM,
                ..Default::default()
            }),
            card(
                column((
                    BoxSized::new(Button::with_label("Button"))
                        .with_size(Unit::rel2(0.5, 0.0) + Unit::px2(0.0, 10.0)),
                    Button::with_label("Warning").with_style(ButtonStyle {
                        normal_color: style::warning_element(),
                        ..Default::default()
                    }),
                    Button::with_label("Error").with_style(ButtonStyle {
                        normal_color: style::error_element(),
                        ..Default::default()
                    }),
                ))
                .with_stretch(false),
            ),
            BoxSized::new(Rectangle::new(EERIE_BLACK_600))
                .with_size(Unit::rel2(1.0, 0.0) + Unit::px2(0.0, 1.0)),
            card(column((
                column((
                    row((Text::new("Size"), SliderWithLabel::new(value, 0.0, 20.0))),
                    row((Text::new("Count"), SliderWithLabel::new(count, 4, 20))),
                ))
                .with_direction(Direction::Vertical),
                Signal::new(item_list),
            ))),
            column(
                [
                    EERIE_BLACK_DEFAULT,
                    PLATINUM_DEFAULT,
                    JADE_DEFAULT,
                    DARK_CYAN_DEFAULT,
                    ULTRA_VIOLET_DEFAULT,
                    LION_DEFAULT,
                    REDWOOD_DEFAULT,
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
                    BoxSized::new(Rectangle::new(color)).with_size(Unit::px2(100.0, 40.0)),
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
                    let size = 10.0 + i as f32 * self.scale;
                    BoxSized::new(
                        Stack::new(Text::new(format!("{size}px")))
                            .with_background(Background::new(
                                Hsva::new(i as f32 * 30.0, 0.6, 0.7, 1.0).into_color(),
                            ))
                            .with_padding(MARGIN_SM)
                            .with_margin(MARGIN_SM)
                            .with_vertical_alignment(Alignment::Center)
                            .with_horizontal_alignment(Alignment::Center),
                    )
                    .with_size(Unit::px2(size, size))
                })
                .collect::<Vec<_>>(),
        )
        .with_cross_align(Alignment::Center)
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
                                        Positioned::new(BoxSized::new(Rectangle::new(JADE_DEFAULT))
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
