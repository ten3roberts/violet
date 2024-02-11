use std::usize;

use flax::{
    events::{EventKind, EventSubscriber},
    Entity, EntityRef,
};
use futures::StreamExt;
use futures_signals::{
    map_ref,
    signal::{self, Mutable, SignalExt},
};
use glam::{vec2, Vec2};
use guillotiere::euclid::num::Round;
use palette::{Hsva, IntoColor, Srgba};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet::{
    components::{self, anchor, offset, rect, Edges},
    editor::{self, EditAction, EditorAction, TextEditor},
    input::{
        focus_sticky, focusable, on_char_typed, on_cursor_move, on_keyboard_input, on_mouse_input,
        CursorMove,
    },
    layout::{CrossAlign, Direction},
    style::StyleExt,
    text::{LayoutGlyphs, TextSegment},
    to_owned,
    unit::Unit,
    widget::{ContainerExt, List, NoOp, Rectangle, Signal, Stack, Text},
    App, Frame, Scope, StreamEffect, Widget,
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
    fn mount(self, scope: &mut Scope<'_>) {
        let content = Mutable::new("Hello, World!".into());
        let value = Mutable::new(1.0);
        let count = Mutable::new(5);

        let scale = value.signal();

        let item_list = Box::new(map_ref! {scale, let count = count.signal() => ItemList {
            scale: scale.round(),
            count: *count,
        }});

        List::new((
            List::new((
                Text::new("Input: ").with_margin(MARGIN_SM),
                TextInput::new(content).with_margin(MARGIN_SM),
            ))
            .with_padding(MARGIN_SM)
            .with_margin(MARGIN_SM),
            List::new((
                List::new((
                    Text::new("Size").with_margin(MARGIN_SM),
                    Text::new("Count").with_margin(MARGIN_SM),
                ))
                .with_direction(Direction::Vertical),
                List::new((
                    SliderWithLabel::new(value, 0.0, 20.0),
                    SliderWithLabel::new(count, 4, 20),
                ))
                .with_direction(Direction::Vertical),
            ))
            .with_padding(MARGIN_SM)
            .with_margin(MARGIN_SM),
            Signal::new(item_list),
            Rectangle::new(EERIE_BLACK_600).with_size(Unit::rel2(1.0, 0.0) + Unit::px2(0.0, 1.0)),
        ))
        .with_direction(Direction::Vertical)
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
    fn mount(self, scope: &mut Scope<'_>) {
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
                    // Rectangle::new(Hsva::new(i as f32 * 10.0, 1.0, 1.0, 1.0).into_color())
                    Stack::new(Text::new(format!("{size}px")))
                        .with_background(Rectangle::new(
                            Hsva::new(i as f32 * 30.0, 0.6, 0.7, 1.0).into_color(),
                        ))
                        .with_vertical_alignment(CrossAlign::Center)
                        .with_horizontal_alignment(CrossAlign::Center)
                        .with_padding(MARGIN_SM)
                        .with_margin(MARGIN_SM)
                        .with_size(Unit::px2(size, size))
                })
                .collect::<Vec<_>>(),
        )
        .with_cross_align(CrossAlign::Center)
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
    fn mount(mut self, scope: &mut Scope<'_>) {
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

pub struct Slider<V> {
    value: Mutable<V>,
    min: V,
    max: V,
}

impl<V> Slider<V> {
    pub fn new(value: Mutable<V>, min: V, max: V) -> Self {
        Self { value, min, max }
    }
}

impl<V: SliderValue> Widget for Slider<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        let track = scope.attach(
            Rectangle::new(EERIE_BLACK_400)
                .with_size(Unit::px2(200.0, 5.0))
                // Accommodate the handle
                .with_margin(Edges::even(10.0))
                .with_component(offset(), Default::default()),
        );

        let min = self.min.to_progress();
        let max = self.max.to_progress();

        fn update<V: SliderValue>(
            entity: &EntityRef,
            input: CursorMove,
            min: f32,
            max: f32,
            dst: &Mutable<V>,
        ) {
            let rect = entity.get_copy(rect()).unwrap();
            let value = (input.local_pos.x / rect.size().x).clamp(0.0, 1.0) * (max - min) + min;
            dst.set(V::from_progress(value));
        }

        Stack::new(SliderHandle {
            value: self.value.clone(),
            min,
            max,
            rect_id: track,
        })
        .with_vertical_alignment(CrossAlign::Center)
        .with_component(focusable(), ())
        // TODO:wrapper for this
        .with_component(
            on_mouse_input(),
            Box::new({
                let value = self.value.clone();
                move |_, entity, _, input| {
                    if input.state == ElementState::Pressed {
                        update(entity, input.cursor, min, max, &value);
                    }
                }
            }),
        )
        .with_component(
            on_cursor_move(),
            Box::new(move |_, entity, _, input| update(entity, input, min, max, &self.value)),
        )
        .mount(scope)
    }
}

struct SliderHandle<V> {
    value: Mutable<V>,
    min: f32,
    max: f32,
    rect_id: Entity,
}

impl<V: SliderValue> Widget for SliderHandle<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        let (tx, rx) = flume::unbounded();

        let last_known = Mutable::new(None);

        scope
            .frame_mut()
            .world
            .subscribe(tx.filter(move |kind, data| {
                data.ids.contains(&self.rect_id)
                    && matches!(kind, EventKind::Modified | EventKind::Added)
            }));

        let update = scope.store({
            let rect_id = self.rect_id;
            let max = self.max;
            let min = self.min;

            move |scope: &Scope<'_>, value: f32| {
                let parent_rect = scope
                    .frame()
                    .world
                    .get(rect_id, rect())
                    .map(|v| *v)
                    .unwrap_or_default();

                let pos = (value - min) * parent_rect.size().x / (max - min);

                scope.entity().update_dedup(offset(), Unit::px2(pos, 0.0));
            }
        });

        scope.spawn_effect(StreamEffect::new(
            self.value.signal_ref(|v| v.to_progress()).to_stream(),
            {
                to_owned![update, last_known];
                move |scope: &mut Scope<'_>, v: f32| {
                    last_known.set(Some(v));
                    scope.read(&update)(scope, v);
                }
            },
        ));

        to_owned![update, last_known];
        scope.spawn_effect(StreamEffect::new(
            rx.into_stream(),
            move |scope: &mut Scope<'_>, _v| {
                if let Some(last_known) = last_known.get() {
                    scope.read(&update)(scope, last_known);
                }
            },
        ));

        Rectangle::new(EMERALD)
            .with_size(Unit::px2(5.0, 20.0))
            .with_component(anchor(), Unit::rel2(0.5, 0.0))
            .with_component(offset(), Default::default())
            .with_margin(MARGIN_SM)
            .mount(scope)
    }
}

pub trait SliderValue: 'static + Send + Sync + Copy + std::fmt::Display {
    fn from_progress(v: f32) -> Self;
    fn to_progress(&self) -> f32;
}

impl SliderValue for f32 {
    fn from_progress(v: f32) -> Self {
        v
    }

    fn to_progress(&self) -> f32 {
        *self
    }
}

macro_rules! num_impl {
    ($ty: ty) => {
        impl SliderValue for $ty {
            fn from_progress(v: f32) -> Self {
                v.round() as $ty
            }

            fn to_progress(&self) -> f32 {
                *self as f32
            }
        }
    };
}

num_impl!(i8);
num_impl!(u8);
num_impl!(i16);
num_impl!(u16);
num_impl!(i32);
num_impl!(u32);
num_impl!(i64);
num_impl!(u64);
num_impl!(isize);
num_impl!(usize);

pub struct SliderWithLabel<V> {
    slider: Slider<V>,
}

impl<V> SliderWithLabel<V> {
    pub fn new(value: Mutable<V>, min: V, max: V) -> Self {
        Self {
            slider: Slider::new(value, min, max),
        }
    }
}

impl<V: SliderValue> Widget for SliderWithLabel<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        let label = Signal(
            self.slider
                .value
                .signal()
                .map(|v| Text::rich([TextSegment::new(format!("{:>4.2}", v))])),
        );

        List::new((self.slider, label)).mount(scope)
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
        scope.set(focusable(), ()).set(focus_sticky(), ());

        let (tx, rx) = flume::unbounded();

        let content = self.content.clone();

        let mut editor = TextEditor::new();

        let layout_glyphs = Mutable::new(LayoutGlyphs::default());

        editor.set_text(content.lock_mut().split('\n').map(ToOwned::to_owned));
        editor.set_cursor_at_end();

        let (editor_props_tx, editor_props_rx) = signal::channel(Box::new(NoOp) as Box<dyn Widget>);

        scope.spawn({
            let mut layout_glyphs = layout_glyphs.signal_cloned().to_stream();
            async move {
                let mut rx = rx.into_stream();

                let mut glyphs;

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
                        Some(new_glyphs) = layout_glyphs.next() => {
                            glyphs = new_glyphs;
                            if let Some(loc) = glyphs.to_glyph_position(editor.cursor()) {
                                cursor_pos = vec2(
                                    glyphs[loc].bounds.min.x,
                                    loc.line_index as f32 * glyphs.line_height(),
                                );
                            } else if editor.past_eol() {
                                cursor_pos = glyphs
                                    .lines_indices(editor.cursor().row)
                                    .last()
                                    .map(|(ln, line)| {
                                        vec2(line.bounds.max.x, ln as f32 * glyphs.line_height())
                                    })
                                    .unwrap_or_default();
                            }
                        }
                        else => break,
                    }
                    editor_props_tx
                        .send(Box::new(Stack::new(
                            Rectangle::new(EMERALD)
                                .with_size(Unit::px2(2.0, 18.0))
                                .with_component(offset(), Unit::px(cursor_pos)),
                        )))
                        .ok();
                }
            }
        });

        scope.set(
            on_char_typed(),
            Box::new({
                to_owned![tx];
                move |_, _, _, char| {
                    if char.is_control() {
                        return;
                    }

                    tx.send(EditorAction::Edit(EditAction::InsertChar(char)))
                        .ok();
                }
            }),
        );

        scope.set(
            on_keyboard_input(),
            Box::new(move |_, _, mods, input| {
                let Some(virtual_keycode) = input.virtual_keycode else {
                    return;
                };

                if input.state == ElementState::Pressed {
                    match virtual_keycode {
                        VirtualKeyCode::Back if mods.ctrl() => {
                            tx.send(EditorAction::Edit(EditAction::DeleteBackwardWord))
                                .ok();
                        }
                        VirtualKeyCode::Back => {
                            tx.send(EditorAction::Edit(EditAction::DeleteBackwardChar))
                                .ok();
                            // content.lock_mut().pop();
                        }
                        VirtualKeyCode::Return => {
                            tx.send(EditorAction::Edit(EditAction::InsertLine)).ok();
                            // content.lock_mut().push('\n');
                        }
                        VirtualKeyCode::Left if mods.ctrl() => {
                            tx.send(EditorAction::CursorMove(editor::CursorMove::BackwardWord))
                                .ok();
                        }
                        VirtualKeyCode::Right if mods.ctrl() => {
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
            }),
        );

        pill(Stack::new((
            Signal(self.content.signal_cloned().map(move |v| {
                Text::rich([TextSegment::new(v)])
                    .with_font_size(18.0)
                    .with_component(components::layout_glyphs(), layout_glyphs.clone())
            })),
            Signal(editor_props_rx),
        )))
        .mount(scope)
    }
}
