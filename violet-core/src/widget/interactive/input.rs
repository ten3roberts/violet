use core::panic;
use std::{future::Future, str::FromStr, sync::Arc};

use futures::StreamExt;
use futures_signals::signal::{self, Mutable, SignalExt};
use glam::{vec2, BVec2, Mat4, Vec2, Vec3, Vec3Swizzles};
use itertools::Itertools;
use palette::{Srgba, WithAlpha};
use web_time::Duration;
use winit::{
    event::ElementState,
    keyboard::{Key, ModifiersState, NamedKey},
};

use crate::{
    components::{self, screen_transform},
    editor::{CursorMove, EditAction, EditorAction, EditorLine, TextChange, TextEditorCore},
    input::{
        interactive, keep_focus, on_cursor_move, on_focus, on_keyboard_input, on_mouse_input,
        KeyboardInput,
    },
    io::{self, Clipboard},
    layout::Align,
    state::{StateDuplex, StateExt, StateSink, StateStream},
    style::*,
    text::{CursorLocation, LayoutGlyphs},
    time::sleep,
    to_owned,
    unit::Unit,
    utils::throttle,
    widget::{col, Float, Positioned, Rectangle, Stack, StreamWidget, Text, WidgetExt},
    Edges, Rect, Scope, Widget,
};

pub struct TextInputStyle {
    pub cursor_color: ValueOrRef<Srgba>,
    pub selection_color: ValueOrRef<Srgba>,
    pub background: Background,
    pub font_size: f32,
    pub align: Align,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            cursor_color: surface_interactive_accent().into(),
            selection_color: surface_hover_accent().into(),
            background: Background::new(surface_interactive()),
            font_size: 16.0,
            align: Align::Start,
        }
    }
}

pub struct TextOptions {
    allow_newlines: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            allow_newlines: false,
        }
    }
}

/// Text field allowing arbitrary user input
pub struct TextInput {
    style: TextInputStyle,
    content: Arc<dyn Send + Sync + StateDuplex<Item = String>>,
    size: WidgetSizeProps,
    options: TextOptions,
}

impl TextInput {
    pub fn new(content: impl 'static + Send + Sync + StateDuplex<Item = String>) -> Self {
        Self {
            content: Arc::new(content),
            style: Default::default(),
            size: WidgetSizeProps::default()
                .with_min_size(Unit::px2(32.0, 0.0))
                .with_margin(spacing_small())
                .with_padding(spacing_small())
                .with_corner_radius(default_corner_radius()),
            options: Default::default(),
        }
    }

    pub fn new_parsed<T>(content: impl 'static + Send + Sync + StateDuplex<Item = T>) -> Self
    where
        T: 'static + Send + Sync + ToString + FromStr,
    {
        let content = content
            .filter_map(|v| Some(v.to_string()), |v| v.parse().ok())
            .prevent_feedback();

        let mut this = Self::new(content);
        this.style.align = Align::End;
        this
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
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
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

        let (tx, actions_rx) = flume::unbounded();

        let focused = Mutable::new(false);

        let (dirty_tx, dirty_rx) = flume::unbounded();

        let on_change = move |text: &[EditorLine], change| match change {
            TextChange::Insert(start, end) => {
                for (row, text) in text.iter().enumerate().take(end.row + 1).skip(start.row) {
                    dirty_tx.send((row, Some(text.as_str().to_string()))).ok();
                }
            }
            TextChange::Delete(start, end) => {
                for (row, text) in text.iter().enumerate().take(end.row + 1).skip(start.row) {
                    dirty_tx.send((row, Some(text.as_str().to_string()))).ok();
                }
            }
            TextChange::DeleteLine(row) => {
                dirty_tx.send((row, None)).ok();
            }
        };

        let mut editor = TextEditorCore::new(on_change);

        let layout_glyphs = Mutable::new(Default::default());
        let text_bounds: Mutable<Option<Mat4>> = Mutable::new(None);

        editor.set_cursor_at_end();

        let visual_cursor = Mutable::new(VisualCursor {
            lines: vec![],
            cursor: Default::default(),
        });

        let clipboard = scope
            .get_atom(io::clipboard())
            .expect("Missing clipboard")
            .clone();

        let clipboard = scope.frame().store().get(&clipboard).clone();

        scope.spawn(process_edit_commands(
            focused.clone(),
            layout_glyphs.clone(),
            actions_rx,
            editor,
            clipboard,
            self.content.clone(),
            visual_cursor.clone(),
        ));

        let dragging = Mutable::new(None);

        scope
            .set(interactive(), ())
            .set(keep_focus(), ())
            .on_event(on_focus(), {
                to_owned![tx];
                move |_, focus| {
                    focused.set(focus);

                    if !focus {
                        tx.send(Action::Editor(EditorAction::SelectionClear)).ok();
                    }

                    None
                }
            })
            .on_event(on_mouse_input(), {
                to_owned![layout_glyphs, text_bounds, tx, dragging];
                move |_, input| {
                    if let Some(text_bounds) = &*text_bounds.lock_ref() {
                        let glyphs = layout_glyphs.lock_ref();

                        if input.state == ElementState::Pressed {
                            let text_pos = input.cursor.absolute_pos
                                - text_bounds.transform_point3(Vec3::ZERO).xy();

                            // If shift-clicking, start selecting the region between the current
                            // cursor and the new clicked position
                            if input.modifiers.shift_key() {
                                tx.send(Action::Editor(EditorAction::SelectionStart)).ok();
                            } else {
                                tx.send(Action::Editor(EditorAction::SelectionClear)).ok();
                            }

                            if let Some(hit) = glyphs.hit(text_pos) {
                                dragging.set(Some(input.cursor.local_pos));
                                tx.send(Action::Editor(EditorAction::CursorMove(
                                    CursorMove::SetPosition(hit),
                                )))
                                .ok();
                            }
                        } else {
                            dragging.set(None)
                        }
                    }

                    None
                }
            })
            .on_event(on_cursor_move(), {
                to_owned![layout_glyphs, tx, dragging];
                move |_, input| {
                    let dragging = dragging.get();

                    let drag_start = dragging?;

                    if input.local_pos.distance(drag_start) < 5.0 {
                        return None;
                    }

                    let glyphs = layout_glyphs.lock_ref();

                    let text_pos = input.local_pos;

                    if let Some(hit) = glyphs.hit(text_pos) {
                        tx.send(Action::Editor(EditorAction::SelectionStart)).ok();
                        tx.send(Action::Editor(EditorAction::CursorMove(
                            CursorMove::SetPosition(hit),
                        )))
                        .ok();
                    }

                    None
                }
            })
            .on_event(on_keyboard_input(), {
                to_owned![tx];
                move |_, input| {
                    if input.state == ElementState::Pressed {
                        handle_input(input, |v| {
                            tx.send(v).ok();
                        })
                    }

                    None
                }
            });

        Stack::new(Stack::new((
            TextContent {
                rx: dirty_rx,
                font_size: self.style.font_size,
                text_bounds: text_bounds.clone(),
                layout_glyphs: layout_glyphs.clone(),
            },
            Float::new(StreamWidget(
                visual_cursor
                    .signal_ref(move |v| {
                        let lines = v
                            .lines
                            .iter()
                            .map(move |line| {
                                Positioned::new(
                                    Rectangle::new(selection_color)
                                        .with_exact_size(Unit::px(line.size())),
                                )
                                .with_offset(Unit::px(line.pos()))
                            })
                            .chain([Positioned::new(
                                Rectangle::new(cursor_color)
                                    .with_exact_size(Unit::px(v.cursor.size())),
                            )
                            .with_offset(Unit::px(v.cursor.pos()))]);

                        Stack::new(lines.collect_vec())
                    })
                    .to_stream(),
            )),
        )))
        .with_size_props(self.size)
        .with_horizontal_alignment(self.style.align)
        .with_background(self.style.background)
        .with_clip(BVec2::TRUE)
        .mount(scope)
    }
}

struct VisualCursor {
    lines: Vec<Rect>,
    cursor: Rect,
}

fn process_edit_commands(
    focused: Mutable<bool>,
    layout_glyphs: Mutable<LayoutGlyphs>,
    rx: flume::Receiver<Action>,
    mut editor: TextEditorCore,
    clipboard: Arc<Clipboard>,
    source_content: Arc<dyn Send + Sync + StateDuplex<Item = String>>,
    cursor: Mutable<VisualCursor>,
) -> impl Future<Output = ()> {
    let mut layout_glyphs = layout_glyphs.signal_cloned().to_stream().fuse();
    let mut focused_signal = focused.stream().fuse();
    async move {
        let mut rx = rx.into_stream().fuse();

        let mut glyphs: LayoutGlyphs = LayoutGlyphs::default();

        let source_content = source_content.dedup().prevent_feedback();
        let mut new_text = throttle(source_content.stream(), || {
            sleep(Duration::from_millis(100))
        })
        .fuse();

        let mut focused = false;

        loop {
            futures::select! {
                focus = focused_signal.select_next_some() => {
                    focused = focus;
                }
                new_text = new_text.select_next_some() => {
                    editor.set_text(new_text.split('\n'));
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

                    source_content.send(editor.lines().iter().map(|v| v.text()).join("\n"));
                }
                new_glyphs = layout_glyphs.select_next_some() => {
                    glyphs = new_glyphs;
                }
            }

            let cursor_pos = calculate_position(&glyphs, editor.cursor());

            let mut cursor = cursor.lock_mut();
            if let Some((start, end)) = editor.selection_bounds() {
                let selected_lines = glyphs
                    .lines()
                    .enumerate()
                    .filter(|(_, v)| v.row >= start.row && v.row <= end.row);

                let lines = selected_lines.filter_map(|(ln, v)| {
                    let left = if v.row == start.row {
                        v.glyphs.iter().find(|v| {
                            v.start >= start.col && (start.row != end.row || v.start < end.col)
                        })
                    } else {
                        // None
                        v.glyphs.first()
                    }?;
                    let right = if v.row == end.row {
                        v.glyphs.iter().rev().find(|v| {
                            v.end <= end.col && (start.row != end.row || v.end > start.col)
                        })
                    } else {
                        // None
                        v.glyphs.last()
                    }?;

                    let rect = Rect::new(
                        left.bounds.min + vec2(0.0, ln as f32 * glyphs.line_height),
                        right.bounds.max + vec2(0.0, ln as f32 * glyphs.line_height),
                    );

                    Some(
                        rect, // Positioned::new(
                             // Rectangle::new(selection_color)
                             //     .with_min_size(Unit::px(rect.size())),
                             // )
                             // .with_offset(Unit::px(rect.pos())),
                    )
                });

                cursor.lines.clear();
                cursor.lines.extend(lines);
            } else if focused {
                cursor.lines.clear();

                cursor.cursor = Rect::from_size_pos(vec2(2.0, 16.0), cursor_pos);
            } else {
                cursor.cursor = Rect::default();
                cursor.lines.clear();
            };
        }
    }
}

struct TextContent {
    rx: flume::Receiver<(usize, Option<String>)>,
    font_size: f32,
    text_bounds: Mutable<Option<Mat4>>,
    layout_glyphs: Mutable<LayoutGlyphs>,
}

impl Widget for TextContent {
    fn mount(self, scope: &mut Scope<'_>) {
        let create_row = move |row, text| {
            let layout_glyphs = self.layout_glyphs.clone();
            Text::new(text)
                .with_margin(Edges::ZERO)
                .with_font_size(self.font_size)
                .monitor(
                    components::layout_glyphs(),
                    Box::new(move |glyphs| {
                        if let Some(new) = glyphs {
                            let glyphs = &mut *layout_glyphs.lock_mut();

                            glyphs.set_row(row, new.rows[0].clone());
                            glyphs.line_height = new.line_height;
                        }
                    }),
                )
        };

        let mut text_items = vec![];

        scope.spawn_stream(self.rx.into_stream(), move |scope, (row, text)| {
            if let Some(text) = text {
                if let Some(&id) = text_items.get(row) {
                    // Access and update the text widget
                    let scope = scope.frame_mut().scoped(id).unwrap();

                    scope.entity().get_mut(components::text()).unwrap()[0].text = text;
                } else {
                    let id = scope.attach(create_row(row, text));

                    text_items.push(id);
                }
            } else {
                // Lines were deleted
                let id = text_items.remove(row);
                scope.detach(id);
            }
        });

        col(())
            .monitor_signal(screen_transform(), self.text_bounds.clone())
            .mount(scope);
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
        tracing::debug!("not on a glyph boundary");
        glyphs
            .find_lines_indices(cursor.row)
            .last()
            .map(|(ln, line)| vec2(line.bounds.max.x, ln as f32 * glyphs.line_height))
            .unwrap_or_default()
    }
}

fn handle_cursor_move(key: NamedKey, mods: ModifiersState) -> Option<CursorMove> {
    let ctrl = mods.control_key();
    match key {
        NamedKey::ArrowLeft if ctrl => Some(CursorMove::BackwardWord),
        NamedKey::ArrowRight if ctrl => Some(CursorMove::ForwardWord),
        NamedKey::ArrowLeft => Some(CursorMove::Left),
        NamedKey::ArrowRight => Some(CursorMove::Right),
        NamedKey::ArrowUp => Some(CursorMove::Up),
        NamedKey::ArrowDown => Some(CursorMove::Down),
        _ => None,
    }
}

fn handle_input(input: KeyboardInput, send: impl Fn(Action)) {
    let ctrl = input.modifiers.control_key();
    if let Key::Named(key) = input.key {
        if let Some(m) = handle_cursor_move(key, input.modifiers) {
            if input.modifiers.shift_key() {
                send(Action::Editor(EditorAction::SelectionStart));
            } else {
                send(Action::Editor(EditorAction::SelectionClear));
            }

            return send(Action::Editor(EditorAction::CursorMove(m)));
        }

        match key {
            NamedKey::Backspace if ctrl => {
                return send(Action::Editor(EditorAction::Edit(
                    EditAction::DeleteBackwardWord,
                )))
            }
            NamedKey::Backspace => {
                return send(Action::Editor(EditorAction::Edit(
                    EditAction::DeleteBackwardChar,
                )))
            }
            NamedKey::Enter => {
                return send(Action::Editor(EditorAction::Edit(EditAction::InsertLine)))
            }
            _ => {}
        }
    } else if let Key::Character(c) = input.key {
        match &*c {
            "c" if ctrl => return send(Action::Copy),
            "v" if ctrl => return send(Action::Paste),
            "x" if ctrl => return send(Action::Cut),
            _ => {}
        }
    }

    if let Some(text) = input.text {
        send(Action::Editor(EditorAction::Edit(EditAction::InsertText(
            text.into(),
        ))));
    }
}
