use core::panic;
use std::{fmt::Display, future::ready, str::FromStr, sync::Arc};

use futures::StreamExt;
use futures_signals::signal::{self, Mutable, SignalExt};
use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{Srgba, WithAlpha};
use web_time::Duration;
use winit::{
    event::ElementState,
    keyboard::{Key, NamedKey},
};

use crate::{
    components::{self, screen_rect},
    editor::{CursorMove, EditAction, EditorAction, TextEditor},
    input::{
        focus_sticky, focusable, on_cursor_move, on_focus, on_keyboard_input, on_mouse_input,
        KeyboardInput,
    },
    io,
    state::{State, StateDuplex, StateSink, StateStream},
    style::{
        interactive_active, interactive_hover, interactive_inactive, interactive_passive,
        spacing_small, Background, SizeExt, StyleExt, ValueOrRef, WidgetSize,
    },
    text::{CursorLocation, LayoutGlyphs, TextSegment},
    time::sleep,
    to_owned,
    unit::Unit,
    utils::throttle,
    widget::{
        row, Float, NoOp, Positioned, Rectangle, SignalWidget, Stack, StreamWidget, Text, WidgetExt,
    },
    Rect, Scope, Widget,
};

pub struct TextInputStyle {
    pub cursor_color: ValueOrRef<Srgba>,
    pub selection_color: ValueOrRef<Srgba>,
    pub background: Background,
    pub font_size: f32,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            cursor_color: interactive_active().into(),
            selection_color: interactive_hover().into(),
            background: Background::new(interactive_passive()),
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
                .with_min_size(Unit::px2(16.0, 16.0))
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
        let selection_color = self
            .style
            .selection_color
            .resolve(stylesheet)
            .with_alpha(0.2);

        let (tx, rx) = flume::unbounded();

        let focused = Mutable::new(false);

        // Internal text to keep track of non-bijective text changes, such as incomplete numeric
        // input
        let text_content = Mutable::new(String::new());
        let mut editor = TextEditor::new();

        let layout_glyphs = Mutable::new(None);
        let text_bounds: Mutable<Option<Rect>> = Mutable::new(None);

        editor.set_cursor_at_end();

        let (editor_props_tx, editor_props_rx) = signal::channel(Box::new(NoOp) as Box<dyn Widget>);
        let content = self.content.prevent_feedback();

        let clipboard = scope
            .frame()
            .get_atom(io::clipboard())
            .expect("Missing clipboard")
            .clone();

        let clipboard = scope.frame().store().get(&clipboard).clone();

        scope.spawn({
            let mut layout_glyphs = layout_glyphs.signal_cloned().to_stream().fuse();
            let mut focused_signal = focused.stream().fuse();
            to_owned![text_content];
            async move {
                let mut rx = rx.into_stream().fuse();

                let mut glyphs: Option<LayoutGlyphs> = None;

                let mut new_text =
                    throttle(content.stream(), || sleep(Duration::from_millis(100))).fuse();
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
                            match action {
                                Action::Editor(editor_action) => editor.apply_action(editor_action),
                                Action::Copy => {
                                    if let Some(sel) = editor.selected_text() {
                                        clipboard.set_text(sel.join("\n")).await;
                                    }
                                }
                                Action::Paste => {
                                    if let Some(text) = clipboard.get_text().await {
                                        editor.edit(EditAction::InsertText(text));
                                    }
                                }
                                Action::Cut => {
                                    if let Some(sel) = editor.selected_text() {
                                        clipboard.set_text(sel.join("\n")).await;
                                        editor.delete_selected_text();
                                    }
                                }
                            }

                            let mut text = text_content.lock_mut();
                            text.clear();
                            #[allow(unstable_name_collisions)]
                            text.extend(editor.lines().iter().map(|v| v.text()).intersperse("\n"));

                            content.send(editor.lines().iter().map(|v| v.text()).join("\n"));
                            // text_content.send(editor.lines().iter().map(|v| v.text()).join("\n"));
                        }
                        new_glyphs = layout_glyphs.select_next_some() => {
                                glyphs = new_glyphs;

                            }
                    }

                    if let Some(glyphs) = &glyphs {
                        let cursor_pos = calculate_position(glyphs, editor.cursor());

                        let selection = if let Some((start, end)) = editor.selection_bounds() {
                            tracing::info!(?start, ?end, "selection");

                            let selected_lines =
                                glyphs.lines().iter().enumerate().filter(|(_, v)| {
                                    tracing::info!(?v.row);
                                    v.row >= start.row && v.row <= end.row
                                });

                            let selection = selected_lines
                                .filter_map(|(ln, v)| {
                                    tracing::info!(?ln, glyphs = v.glyphs.len());

                                    let left = if v.row == start.row {
                                        v.glyphs.iter().find(|v| {
                                            v.start >= start.col
                                                && (start.row != end.row || v.start < end.col)
                                        })
                                    } else {
                                        // None
                                        v.glyphs.first()
                                    }?;
                                    let right = if v.row == end.row {
                                        v.glyphs.iter().rev().find(|v| {
                                            v.end <= end.col
                                                && (start.row != end.row || v.end > start.col)
                                        })
                                    } else {
                                        // None
                                        v.glyphs.last()
                                    }?;

                                    // dbg!(left, right);

                                    let rect = Rect::new(
                                        left.bounds.min - vec2(0.0, 2.0),
                                        right.bounds.max + vec2(0.0, 2.0),
                                    );

                                    Some(
                                        Positioned::new(
                                            Rectangle::new(selection_color)
                                                .with_min_size(Unit::px(rect.size())),
                                        )
                                        .with_offset(Unit::px(rect.pos())),
                                    )
                                })
                                .collect_vec();

                            Some(Stack::new(selection))
                        } else {
                            None
                        };
                        let props = Stack::new((
                            focused.then(|| {
                                Positioned::new(
                                    Rectangle::new(cursor_color)
                                        .with_min_size(Unit::px2(2.0, 16.0)),
                                )
                                .with_offset(Unit::px(cursor_pos))
                            }),
                            selection,
                        ));

                        editor_props_tx.send(Box::new(props)).ok();
                    }
                }
            }
        });

        let dragging = Mutable::new(None);

        scope
            .set(focusable(), ())
            .set(focus_sticky(), ())
            .on_event(on_focus(), move |_, focus| {
                focused.set(focus);
            })
            .on_event(on_mouse_input(), {
                to_owned![layout_glyphs, text_bounds, tx, dragging];
                move |_, input| {
                    let glyphs = layout_glyphs.lock_ref();

                    if let (Some(glyphs), Some(text_bounds)) = (&*glyphs, &*text_bounds.lock_ref())
                    {
                        if input.state == ElementState::Pressed {
                            let text_pos = input.cursor.absolute_pos - text_bounds.min;

                            if let Some(hit) = glyphs.hit(text_pos) {
                                dragging.set(Some(hit));
                                tx.send(Action::Editor(EditorAction::CursorMove(
                                    CursorMove::SetPosition(hit),
                                )))
                                .ok();
                                tx.send(Action::Editor(EditorAction::SelectionClear)).ok();
                            }

                            tracing::info!(?input, "click");
                        } else {
                            dragging.set(None)
                        }
                    }
                }
            })
            .on_event(on_cursor_move(), {
                to_owned![layout_glyphs, tx, dragging];
                move |_, input| {
                    let dragging = dragging.get();

                    if let Some(dragging) = dragging {
                        let glyphs = layout_glyphs.lock_ref();

                        if let Some(glyphs) = &*glyphs {
                            let text_pos = input.local_pos;

                            if let Some(hit) = glyphs.hit(text_pos) {
                                tx.send(Action::Editor(EditorAction::SelectionMove(
                                    CursorMove::SetPosition(dragging),
                                )))
                                .ok();
                                tx.send(Action::Editor(EditorAction::CursorMove(
                                    CursorMove::SetPosition(hit),
                                )))
                                .ok();
                            }
                        }
                    }
                }
            })
            .on_event(on_keyboard_input(), {
                to_owned![tx];
                move |_, input| {
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
            Float::new(SignalWidget(editor_props_rx)),
        ))
        .with_background(self.style.background)
        .with_size_props(self.size)
        .mount(scope)
    }
}

enum Action {
    Editor(EditorAction),
    Copy,
    Paste,
    Cut,
}

pub fn calculate_position(glyphs: &LayoutGlyphs, cursor: CursorLocation) -> Vec2 {
    if let Some(loc) = glyphs.to_glyph_boundary(cursor) {
        loc
    } else {
        glyphs
            .find_lines_indices(cursor.row)
            .last()
            .map(|(ln, line)| vec2(line.bounds.max.x, ln as f32 * glyphs.line_height()))
            .unwrap_or_default()
    }
}

fn handle_input(input: KeyboardInput) -> Option<Action> {
    let ctrl = input.modifiers.control_key();
    if let Key::Named(key) = input.event.logical_key {
        match key {
            NamedKey::Backspace if ctrl => {
                return Some(Action::Editor(EditorAction::Edit(
                    EditAction::DeleteBackwardWord,
                )))
            }
            NamedKey::Backspace => {
                return Some(Action::Editor(EditorAction::Edit(
                    EditAction::DeleteBackwardChar,
                )))
            }
            NamedKey::Enter => {
                return Some(Action::Editor(EditorAction::Edit(EditAction::InsertLine)))
            }
            NamedKey::ArrowLeft if ctrl => {
                return Some(Action::Editor(EditorAction::CursorMove(
                    CursorMove::BackwardWord,
                )))
            }
            NamedKey::ArrowRight if ctrl => {
                return Some(Action::Editor(EditorAction::CursorMove(
                    CursorMove::ForwardWord,
                )))
            }
            NamedKey::ArrowLeft => {
                return Some(Action::Editor(EditorAction::CursorMove(CursorMove::Left)))
            }
            NamedKey::ArrowRight => {
                return Some(Action::Editor(EditorAction::CursorMove(CursorMove::Right)))
            }
            NamedKey::ArrowUp => {
                return Some(Action::Editor(EditorAction::CursorMove(CursorMove::Up)))
            }
            NamedKey::ArrowDown => {
                return Some(Action::Editor(EditorAction::CursorMove(CursorMove::Down)))
            }
            _ => {}
        }
    } else if let Key::Character(c) = input.event.logical_key {
        match &*c {
            "c" if ctrl => return Some(Action::Copy),
            "v" if ctrl => return Some(Action::Paste),
            "x" if ctrl => return Some(Action::Cut),
            _ => {}
        }
    }

    if let Some(text) = input.event.text {
        return Some(Action::Editor(EditorAction::Edit(EditAction::InsertText(
            text.into(),
        ))));
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
