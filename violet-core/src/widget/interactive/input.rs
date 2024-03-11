use core::panic;
use std::{fmt::Display, future::ready, str::FromStr, sync::Arc};

use futures::{FutureExt, StreamExt};
use futures_signals::signal::{self, Mutable, SignalExt};
use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::Srgba;
use winit::{
    event::ElementState,
    keyboard::{Key, NamedKey},
};

use crate::{
    components::{self, screen_rect},
    editor::{CursorMove, EditAction, EditorAction, TextEditor},
    input::{focus_sticky, focusable, on_focus, on_keyboard_input, on_mouse_input, KeyboardInput},
    state::{State, StateDuplex, StateSink, StateStream},
    style::{
        colors::EERIE_BLACK_300, interactive_active, spacing_small, Background, SizeExt, StyleExt,
        ValueOrRef, WidgetSize,
    },
    text::{LayoutGlyphs, TextSegment},
    to_owned,
    unit::Unit,
    widget::{
        row, NoOp, Positioned, Rectangle, SignalWidget, Stack, StreamWidget, Text, WidgetExt,
    },
    Rect, Scope, Widget,
};

pub struct TextInputStyle {
    pub cursor_color: ValueOrRef<Srgba>,
    pub background: Background,
    pub font_size: f32,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            cursor_color: interactive_active().into(),
            background: Background::new(EERIE_BLACK_300),
            font_size: 16.0,
        }
    }
}

/// Text field allowing arbitrary user input
pub struct TextInput {
    style: TextInputStyle,
    content: Arc<dyn Send + Sync + StateDuplex<Item = String>>,
    size: WidgetSize,
}

impl TextInput {
    pub fn new(content: impl 'static + Send + Sync + StateDuplex<Item = String>) -> Self {
        Self {
            content: Arc::new(content),
            style: Default::default(),
            size: WidgetSize::default()
                .with_margin(spacing_small())
                .with_padding(spacing_small()),
        }
    }
}

impl StyleExt for TextInput {
    type Style = TextInputStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl SizeExt for TextInput {
    fn size_mut(&mut self) -> &mut WidgetSize {
        &mut self.size
    }
}

impl Widget for TextInput {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let cursor_color = self.style.cursor_color.resolve(stylesheet);

        let (tx, rx) = flume::unbounded();

        let focused = Mutable::new(false);

        // Internal text to keep track of non-bijective text changes, such as incomplete numeric
        // input
        let text_content = Mutable::new(String::new());
        let mut editor = TextEditor::new();

        let layout_glyphs = Mutable::new(None);
        let text_bounds: Mutable<Option<Rect>> = Mutable::new(None);

        // editor.set_text(content.lock_mut().split('\n'));
        editor.set_cursor_at_end();

        let (editor_props_tx, editor_props_rx) = signal::channel(Box::new(NoOp) as Box<dyn Widget>);
        let content = self.content;

        scope.spawn({
            let mut layout_glyphs = layout_glyphs.signal_cloned().to_stream();
            let mut focused_signal = focused.stream().fuse();
            to_owned![text_content];
            async move {
                let mut rx = rx.into_stream().fuse();

                let mut glyphs: LayoutGlyphs;

                let mut cursor_pos = Vec2::ZERO;

                let mut new_text = content.stream().fuse();
                let mut focused = false;

                loop {
                    futures::select! {
                        focus = focused_signal.select_next_some() => {
                            focused = focus;
                        }
                        new_text = new_text.select_next_some() => {
                            editor.set_text(new_text.split('\n'));
                            text_content.send(new_text);
                        }
                        action = rx.select_next_some() => {
                            editor.apply_action(action);

                            let mut text = text_content.lock_mut();
                            text.clear();
                            #[allow(unstable_name_collisions)]
                            text.extend(editor.lines().iter().map(|v| v.text()).intersperse("\n"));

                            content.send(editor.lines().iter().map(|v| v.text()).join("\n"));
                            // text_content.send(editor.lines().iter().map(|v| v.text()).join("\n"));
                        }
                        new_glyphs = layout_glyphs.next().fuse() => {
                            if let Some(Some(new_glyphs)) = new_glyphs {
                                glyphs = new_glyphs;

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
                        }
                    }

                                      editor_props_tx
                        .send(Box::new(Stack::new(
                                    (
                                        focused.then(|| Positioned::new(Rectangle::new(cursor_color)
                                            .with_min_size(Unit::px2(2.0, 16.0)))
                                        .with_offset(Unit::px(cursor_pos))),
                                    )
                        )))
                        .ok();
                    }
            }
        });

        scope
            .set(focusable(), ())
            .set(focus_sticky(), ())
            .on_event(on_focus(), move |_, _, focus| {
                focused.set(focus);
            })
            .on_event(on_mouse_input(), {
                to_owned![layout_glyphs, text_bounds, tx];
                move |_, _, input| {
                    let glyphs = layout_glyphs.lock_ref();

                    if let (Some(glyphs), Some(text_bounds)) = (&*glyphs, &*text_bounds.lock_ref())
                    {
                        if input.state == ElementState::Pressed {
                            let text_pos = input.cursor.absolute_pos - text_bounds.min;
                            if let Some(hit) = glyphs.hit(text_pos) {
                                tx.send(EditorAction::CursorMove(CursorMove::SetPosition(hit)))
                                    .ok();
                            }

                            tracing::info!(?input, "click");
                        }
                    }
                }
            })
            .on_event(on_keyboard_input(), {
                to_owned![tx];
                move |_, _, input| {
                    if input.event.state == ElementState::Pressed {
                        if let Some(action) = handle_input(input) {
                            tx.send(action).ok();
                        }
                    }
                }
            });

        Stack::new((
            StreamWidget(text_content.stream().map(move |v| {
                to_owned![text_bounds];
                Text::rich([TextSegment::new(v)])
                    .with_font_size(self.style.font_size)
                    .monitor_signal(components::layout_glyphs(), layout_glyphs.clone())
                    .monitor_signal(screen_rect(), text_bounds.clone())
            })),
            SignalWidget(editor_props_rx),
        ))
        .with_background(self.style.background)
        .with_size_props(self.size)
        .mount(scope)
    }
}

fn handle_input(input: KeyboardInput) -> Option<EditorAction> {
    let ctrl = input.modifiers.control_key();
    if let Key::Named(key) = input.event.logical_key {
        match key {
            NamedKey::Backspace if ctrl => {
                return Some(EditorAction::Edit(EditAction::DeleteBackwardWord))
            }
            NamedKey::Backspace => return Some(EditorAction::Edit(EditAction::DeleteBackwardChar)),
            NamedKey::Enter => return Some(EditorAction::Edit(EditAction::InsertLine)),
            NamedKey::ArrowLeft if ctrl => {
                return Some(EditorAction::CursorMove(CursorMove::BackwardWord))
            }
            NamedKey::ArrowRight if ctrl => {
                return Some(EditorAction::CursorMove(CursorMove::ForwardWord))
            }
            NamedKey::ArrowLeft => return Some(EditorAction::CursorMove(CursorMove::Left)),
            NamedKey::ArrowRight => return Some(EditorAction::CursorMove(CursorMove::Right)),
            NamedKey::ArrowUp => return Some(EditorAction::CursorMove(CursorMove::Up)),
            NamedKey::ArrowDown => return Some(EditorAction::CursorMove(CursorMove::Down)),
            _ => {}
        }
    }

    if let Some(text) = input.event.text {
        return Some(EditorAction::Edit(EditAction::InsertText(text.into())));
    }

    None
}

pub struct InputField<V> {
    label: String,
    value: Arc<dyn StateDuplex<Item = V>>,
}

impl<V> InputField<V> {
    pub fn new(
        label: impl Into<String>,
        value: impl 'static + Send + Sync + StateDuplex<Item = V>,
    ) -> Self {
        Self {
            label: label.into(),
            value: Arc::new(value),
        }
    }
}

impl<V: 'static + Display + FromStr> Widget for InputField<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        let text_value = Mutable::new(String::new());
        let value = self.value.clone();

        scope.spawn(
            text_value
                .signal_cloned()
                .dedupe_cloned()
                .to_stream()
                .filter_map(|v| {
                    tracing::info!(?v, "Parsing");
                    ready(v.trim().parse().ok())
                })
                .for_each(move |v| {
                    tracing::info!("Parsed: {}", v);
                    value.send(v);
                    async {}
                }),
        );

        scope.spawn(self.value.stream().map(|v| v.to_string()).for_each({
            to_owned![text_value];
            move |v| {
                text_value.set(v);
                async {}
            }
        }));

        let editor = TextInput::new(text_value);

        row((Text::new(self.label), editor)).mount(scope);
    }
}
