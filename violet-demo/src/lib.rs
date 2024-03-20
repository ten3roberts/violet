use std::sync::Arc;

use anyhow::Context;
use flume::Sender;
use futures::{Future, Stream, StreamExt};
use glam::Vec2;
use heck::ToKebabCase;
use indexmap::IndexMap;
use itertools::Itertools;
use rfd::AsyncFileDialog;
use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use violet::{
    core::{
        declare_atom,
        layout::Alignment,
        state::{DynStateDuplex, State, StateMut, StateStream, StateStreamRef},
        style::{
            danger_item, primary_background, success_item, warning_item, Background, SizeExt,
            ValueOrRef,
        },
        time::interval,
        to_owned,
        unit::Unit,
        utils::zip_latest_ref,
        widget::{
            card, centered, column, label, row, Button, Radio, Rectangle, SliderWithLabel, Stack,
            StreamWidget, Text, TextInput, WidgetExt,
        },
        Edges, Scope, Widget,
    },
    futures_signals::signal::Mutable,
    palette::{FromColor, IntoColor, OklabHue, Oklch, Srgb},
    web_time::Duration,
    wgpu::{app::App, renderer::RendererConfig},
};
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
fn setup() {
    use tracing_subscriber::{filter::LevelFilter, fmt::format::Pretty, Layer};
    use tracing_web::{performance_layer, MakeWebConsoleWriter};

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(MakeWebConsoleWriter::new())
        .with_filter(LevelFilter::INFO);

    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();

    console_error_panic_hook::set_once();
}

#[cfg(not(target_arch = "wasm32"))]
fn setup() {
    tracing_subscriber::registry()
        .with(
            tracing_tree::HierarchicalLayer::default()
                .with_deferred_spans(true)
                .with_span_retrace(true)
                .with_indent_lines(true)
                .with_indent_amount(4),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[wasm_bindgen]
pub fn run() {
    setup();

    App::builder()
        .with_title("Palette Editor")
        .with_renderer_config(RendererConfig { debug_mode: false })
        .run(MainApp)
        .unwrap();
}

struct MainApp;

const DEFAULT_FALLOFF: f32 = 15.0;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let palette_item = Mutable::new(
            (0..8)
                .map(|i| {
                    Mutable::new(PaletteColor {
                        color: Oklch::new(0.5, 0.27, (i as f32 * 60.0) % 360.0),
                        falloff: DEFAULT_FALLOFF,
                        name: format!("Color {i}"),
                    })
                })
                .collect(),
        );

        let (notify_tx, notify_rx) = flume::unbounded();

        scope.frame_mut().set_atom(crate::notify_tx(), notify_tx);

        Stack::new((
            Palettes::new(palette_item),
            Stack::new(Notifications {
                items: notify_rx.into_stream(),
            })
            .with_maximize(Vec2::ONE)
            .with_horizontal_alignment(Alignment::End),
        ))
        .with_size(Unit::rel2(1.0, 1.0))
        .with_background(Background::new(primary_background()))
        .mount(scope);
    }
}

fn tints(color: impl StateStream<Item = PaletteColor>) -> impl Widget {
    puffin::profile_function!();
    row(TINTS
        .iter()
        .map(move |&i| {
            let color = color.stream().map(move |v| {
                let f = (i as f32) / 1000.0;
                let color = v.tint(f);

                Rectangle::new(ValueOrRef::value(color.into_color()))
                    .with_size(Unit::px2(120.0, 80.0))
            });

            Stack::new(column(StreamWidget(color)))
                .with_margin(Edges::even(4.0))
                .with_name("Tint")
        })
        .collect_vec())
}

pub fn color_hex(color: impl IntoColor<Srgb>) -> String {
    let hex: Srgb<u8> = color.into_color().into_format();
    format!("#{:0>2x}{:0>2x}{:0>2x}", hex.red, hex.green, hex.blue)
}

pub struct Palettes {
    items: Mutable<Vec<Mutable<PaletteColor>>>,
}

impl Palettes {
    pub fn new(items: Mutable<Vec<Mutable<PaletteColor>>>) -> Self {
        Self { items }
    }
}

declare_atom! {
    notify_tx: flume::Sender<Notification>,
}

impl Widget for Palettes {
    fn mount(self, scope: &mut Scope<'_>) {
        let notify_tx = scope.frame().get_atom(notify_tx()).unwrap().clone();
        let items = self.items.clone();

        let discard = move |i| {
            let items = items.clone();
            Button::new(Text::new("-"))
                .on_press({
                    move |_, _| {
                        items.lock_mut().remove(i);
                    }
                })
                .danger()
        };

        let current_choice = Mutable::new(Some(0));

        let editor = zip_latest_ref(
            self.items.stream(),
            current_choice.stream(),
            |items, i: &Option<usize>| {
                i.and_then(|i| items.get(i).cloned())
                    .map(PaletteEditor::new)
            },
        );

        let palettes = StreamWidget(self.items.stream_ref({
            to_owned![current_choice];
            move |items| {
                let items = items
                    .iter()
                    .enumerate()
                    .map({
                        to_owned![current_choice];
                        let discard = &discard;
                        move |(i, item)| {
                            puffin::profile_scope!("Update palette item", format!("{i}"));
                            let checkbox = Radio::new(
                                current_choice
                                    .clone()
                                    .map(move |v| v == Some(i), move |state| state.then_some(i)),
                            );

                            card(row((
                                checkbox,
                                discard(i),
                                palette_color_view(item.clone()),
                            )))
                        }
                    })
                    .collect_vec();

                column(items)
            }
        }));

        let items = self.items.clone();

        let new_color = Button::label("+").on_press(move |_, _| {
            items.write_mut(|v| {
                v.push(Mutable::new(PaletteColor {
                    color: Oklch::new(0.5, 0.27, (v.len() as f32 * 60.0) % 360.0),
                    falloff: DEFAULT_FALLOFF,
                    name: format!("Color {}", v.len() + 1),
                }));
                current_choice.set(Some(v.len() - 1));
            })
        });

        let editor_column = column((StreamWidget(editor), palettes, new_color));

        column((
            menu_bar(self.items.clone(), notify_tx),
            row((editor_column, description())),
        ))
        .mount(scope)
    }
}

struct Notification {
    message: String,
    kind: NotificationKind,
}

pub enum NotificationKind {
    Info,
    Warning,
    Error,
}

pub struct Notifications<S> {
    items: S,
}

impl<S> Notifications<S> {
    pub fn new(items: S) -> Self {
        Self { items }
    }
}

impl<S> Widget for Notifications<S>
where
    S: 'static + Stream<Item = Notification>,
{
    fn mount(self, scope: &mut Scope<'_>) {
        let notifications = Mutable::new(Vec::new());

        let notifications_stream = notifications.stream_ref(|v| {
            let items = v
                .iter()
                .map(|(_, v): &(f32, Notification)| {
                    let color = match v.kind {
                        NotificationKind::Info => success_item(),
                        NotificationKind::Warning => warning_item(),
                        NotificationKind::Error => danger_item(),
                    };
                    card(label(v.message.clone())).with_background(Background::new(color))
                })
                .collect_vec();

            column(items)
        });

        scope.spawn(async move {
            let stream = self.items;

            let mut interval = interval(Duration::from_secs(1)).fuse();

            let stream = stream.fuse();
            futures::pin_mut!(stream);

            loop {
                futures::select! {
                    _ = interval.next() =>  {
                        let notifications = &mut *notifications.lock_mut();
                        notifications.retain(|(time, _)| *time > 0.0);
                        for (time, _) in notifications {
                            *time -= 1.0;
                        }
                    },
                    notification = stream.select_next_some() => {
                        notifications.lock_mut().push((5.0, notification));
                    }
                    complete => break,
                }
            }
        });

        StreamWidget(notifications_stream).mount(scope);
    }
}

fn local_dir() -> std::path::PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::current_dir().unwrap()
    }
    #[cfg(target_arch = "wasm32")]
    {
        PathBuf::from(".")
    }
}

fn description() -> impl Widget {
    let content = Mutable::new(
        r#"This is a palette editor. You can add, remove and select the colors in the list. Edit the color by selecting them and using the sliders or typing in the slider labels
You can then export the various generated tints of the colors to a tailwind style `.json`

This text is also editable, give it a try :)"#.to_string(),
    );

    card(TextInput::new(content))
}

fn menu_bar(
    items: Mutable<Vec<Mutable<PaletteColor>>>,
    notify_tx: Sender<Notification>,
) -> impl Widget {
    async fn notify_result(
        fut: impl Future<Output = anyhow::Result<()>>,
        notify_tx: Sender<Notification>,
        on_success: &str,
    ) {
        match fut.await {
            Ok(()) => {
                notify_tx
                    .send(Notification {
                        message: on_success.into(),
                        kind: NotificationKind::Info,
                    })
                    .unwrap();
            }
            Err(e) => {
                notify_tx
                    .send(Notification {
                        message: format!("{e:?}"),
                        kind: NotificationKind::Error,
                    })
                    .unwrap();
            }
        }
    }

    let export = Button::label("Export Json").on_press({
        to_owned![items, notify_tx];
        move |frame, _| {
            let data = items
                .lock_ref()
                .iter()
                .map(|item| {
                    let item = item.lock_ref();
                    let tints = TINTS
                        .iter()
                        .map(|&i| {
                            let color = item.tint(i as f32 / 1000.0);
                            (
                                format!("{}", i),
                                HexColor(Srgb::from_color(color).into_format()),
                            )
                        })
                        .collect::<IndexMap<String, _>>();

                    (item.name.to_kebab_case(), tints)
                })
                .collect::<IndexMap<_, _>>();

            let json = serde_json::to_string_pretty(&data).unwrap();

            let fut = async move {
                let Some(file) = AsyncFileDialog::new()
                    .set_directory(local_dir())
                    .set_file_name("colors.json")
                    .save_file()
                    .await
                else {
                    return anyhow::Ok(());
                };

                file.write(json.as_bytes())
                    .await
                    .context("Failed to write to save file")?;

                Ok(())
            };

            frame.spawn(notify_result(fut, notify_tx.clone(), "Saves"));
        }
    });

    let save = Button::label("Save").on_press({
        to_owned![items, notify_tx];
        move |frame, _| {
            to_owned![items, notify_tx];
            let fut = async move {
                let Some(file) = AsyncFileDialog::new()
                    .set_directory(local_dir())
                    .set_file_name("colors.save.json")
                    .save_file()
                    .await
                else {
                    return anyhow::Ok(());
                };

                let items = items.lock_ref();
                let data =
                    serde_json::to_string_pretty(&*items).context("Failed to serialize state")?;

                file.write(data.as_bytes())
                    .await
                    .context("Failed to write to save file")?;

                Ok(())
            };

            frame.spawn(notify_result(fut, notify_tx, "Saves"));
        }
    });

    let load = Button::label("Load").on_press({
        to_owned![items, notify_tx];
        move |frame, _| {
            to_owned![items, notify_tx];
            let fut = async move {
                let Some(file) = AsyncFileDialog::new()
                    .set_directory(local_dir())
                    .pick_file()
                    .await
                else {
                    return anyhow::Ok(());
                };

                let data = file.read().await;

                let data = serde_json::from_slice(&data).context("Failed to deserialize state")?;

                items.set(data);

                Ok(())
            };

            frame.spawn(notify_result(fut, notify_tx, "Loaded"));
        }
    });

    let test_notification = Button::label("Test Notification").on_press({
        to_owned![notify_tx];
        move |_, _| {
            notify_tx
                .send(Notification {
                    message: "Test notification".to_string(),
                    kind: NotificationKind::Info,
                })
                .unwrap();
        }
    });

    row((
        centered(label("Palette editor")),
        save,
        load,
        export,
        test_notification,
    ))
    .with_stretch(true)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PaletteColor {
    color: Oklch,
    falloff: f32,
    name: String,
}

impl PaletteColor {
    pub fn tint(&self, tint: f32) -> Oklch {
        let chroma = self.color.chroma * (1.0 / (1.0 + self.falloff * (tint - 0.5).powi(2)));
        // let color = self.base.lighten(f);
        Oklch {
            chroma,
            l: tint,
            ..self.color
        }
    }
}

fn palette_color_view(color: Mutable<PaletteColor>) -> impl Widget {
    puffin::profile_function!();
    // let label = color.stream().map(|v| label(color_hex(v.color)));
    let label = color.clone().map_ref(|v| &v.name, |v| &mut v.name);

    let label = TextInput::new(label);
    Stack::new((row((tints(color),)), label))
        .with_vertical_alignment(Alignment::End)
        .with_horizontal_alignment(Alignment::Center)
}

pub struct PaletteEditor {
    color: Mutable<PaletteColor>,
}

impl PaletteEditor {
    pub fn new(color: Mutable<PaletteColor>) -> Self {
        Self { color }
    }
}

impl Widget for PaletteEditor {
    fn mount(self, scope: &mut Scope<'_>) {
        let color = Arc::new(self.color.clone().map_ref(|v| &v.color, |v| &mut v.color));
        let falloff = self.color.map_ref(|v| &v.falloff, |v| &mut v.falloff);

        let lightness = color.clone().map_ref(|v| &v.l, |v| &mut v.l);
        let chroma = color.clone().map_ref(|v| &v.chroma, |v| &mut v.chroma);
        let hue = color
            .clone()
            .map_ref(|v| &v.hue, |v| &mut v.hue)
            .map(|v| v.into_positive_degrees(), OklabHue::new);

        let color_rect = color.stream().map(|v| {
            Rectangle::new(ValueOrRef::value(v.into_color()))
                .with_min_size(Unit::px2(100.0, 100.0))
                .with_maximize(Vec2::X)
                // .with_min_size(Unit::new(vec2(0.0, 100.0), vec2(1.0, 0.0)))
                .with_name("ColorPreview")
        });

        card(column((
            row((
                Text::new("Lightness"),
                SliderWithLabel::new(lightness, 0.0, 1.0)
                    .editable(true)
                    .round(0.01),
            )),
            row((
                Text::new("Chroma"),
                SliderWithLabel::new(chroma, 0.0, 0.37)
                    .editable(true)
                    .round(0.005),
            )),
            row((
                Text::new("Hue"),
                SliderWithLabel::new(hue, 0.0, 360.0)
                    .editable(true)
                    .round(1.0),
            )),
            ColorHexEditor {
                color: Box::new(color.clone()),
            },
            StreamWidget(color_rect),
            row((
                Text::new("Chroma falloff"),
                SliderWithLabel::new(falloff, 0.0, 100.0)
                    .editable(true)
                    .round(1.0),
            )),
        )))
        .with_name("PaletteEditor")
        .mount(scope)
    }
}

pub struct ColorHexEditor {
    color: DynStateDuplex<Oklch>,
}

impl Widget for ColorHexEditor {
    fn mount(self, scope: &mut Scope<'_>) {
        let value = self.color.prevent_feedback().filter_map(
            |v| Some(color_hex(v)),
            |v| {
                let v: Srgb<u8> = v.trim().parse().ok()?;

                let v = Oklch::from_color(v.into_format());
                Some(v)
            },
        );

        TextInput::new(value).mount(scope)
    }
}

pub struct HexColor(Srgb<u8>);

impl Serialize for HexColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = format!(
            "#{:0>2x}{:0>2x}{:0>2x}",
            self.0.red, self.0.green, self.0.blue
        );

        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<HexColor, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let color: Srgb<u8> = s.trim().parse().map_err(serde::de::Error::custom)?;
        Ok(HexColor(color))
    }
}

static TINTS: &[i32] = &[50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];
