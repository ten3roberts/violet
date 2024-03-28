use editor::palette_editor;
use futures::{Stream, StreamExt};
use glam::Vec2;
use itertools::Itertools;
use menu::menu_bar;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use violet::{
    core::{
        declare_atom,
        layout::Alignment,
        state::{State, StateMut, StateStream, StateStreamRef},
        style::{
            danger_element, primary_surface, success_element, warning_element, Background, SizeExt,
            ValueOrRef,
        },
        time::{interval, sleep},
        to_owned,
        unit::Unit,
        utils::{throttle, zip_latest_ref},
        widget::{
            card, col, label, row, Button, Checkbox, Rectangle, ScrollArea, Stack, StreamWidget,
            Text, TextInput, WidgetExt,
        },
        Edges, Scope, Widget,
    },
    futures_signals::signal::Mutable,
    palette::{IntoColor, Oklch, Srgb},
    web_time::Duration,
};

mod editor;
mod menu;

pub struct App;

const DEFAULT_FALLOFF: f32 = 15.0;

impl Widget for App {
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

        scope.frame_mut().set_atom(self::notify_tx(), notify_tx);

        Stack::new((
            Palettes::new(palette_item),
            Stack::new(Notifications {
                items: notify_rx.into_stream(),
            })
            .with_maximize(Vec2::ONE)
            .with_horizontal_alignment(Alignment::End),
        ))
        .with_size(Unit::rel2(1.0, 1.0))
        .with_background(Background::new(primary_surface()))
        .mount(scope);
    }
}

fn tints(color: impl StateStream<Item = PaletteColor>) -> impl Widget {
    puffin::profile_function!();
    row(TINTS
        .iter()
        .map(move |&i| {
            let color =
                throttle(color.stream(), || sleep(Duration::from_millis(200))).map(move |v| {
                    let f = (i as f32) / 1000.0;
                    let color = v.tint(f);

                    Rectangle::new(ValueOrRef::value(color.into_color()))
                        .with_min_size(Unit::px2(80.0, 60.0))
                });

            Stack::new(col(StreamWidget(color)))
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

        let items = self.items.clone();
        let move_up = move |i| {
            let items = items.clone();
            Button::new(Text::new("˰")).on_press({
                move |_, _| {
                    items.write_mut(|v| {
                        if i > 0 {
                            v.swap(i, i - 1);
                        }
                    });
                }
            })
        };

        let items = self.items.clone();
        let move_down = move |i| {
            let items = items.clone();
            Button::new(Text::new("˯")).on_press({
                move |_, _| {
                    items.write_mut(|v| {
                        if i < v.len() - 1 {
                            v.swap(i, i + 1);
                        }
                    });
                }
            })
        };

        let current_choice = Mutable::new(Some(0));

        let editor = zip_latest_ref(
            self.items.stream(),
            current_choice.stream(),
            |items, i: &Option<usize>| i.and_then(|i| items.get(i).cloned()).map(palette_editor),
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
                        let move_up = &move_up;
                        let move_down = &move_down;
                        move |(i, item)| {
                            puffin::profile_scope!("Update palette item", format!("{i}"));
                            let checkbox = Checkbox::new(
                                (),
                                current_choice
                                    .clone()
                                    .map(move |v| v == Some(i), move |state| state.then_some(i)),
                            );

                            card(row((
                                checkbox,
                                move_down(i),
                                move_up(i),
                                discard(i),
                                palette_color_view(item.clone()),
                            )))
                        }
                    })
                    .collect_vec();

                ScrollArea::vertical(col(items))
            }
        }));

        let items = self.items.clone();

        let new_color = Button::label("+").on_press({
            to_owned![items];
            move |_, _| {
                items.write_mut(|v| {
                    v.push(Mutable::new(PaletteColor {
                        color: Oklch::new(0.5, 0.27, (v.len() as f32 * 60.0) % 360.0),
                        falloff: DEFAULT_FALLOFF,
                        name: format!("Color {}", v.len() + 1),
                    }));
                    current_choice.set(Some(v.len() - 1));
                })
            }
        });

        let sort = Button::label("Sort").on_press({
            to_owned![items];
            move |_, _| {
                items.write_mut(|v| {
                    v.sort_by_cached_key(|v| {
                        let v = v.lock_ref();
                        (
                            (v.color.chroma / 0.37 * 5.0) as u32,
                            OrderedFloat(v.color.hue.into_positive_degrees()),
                        )
                    });
                });
            }
        });

        let editor_column = col((StreamWidget(editor), palettes, card(row((new_color, sort)))));

        col((
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

#[allow(dead_code)]
pub enum NotificationKind {
    Info,
    Warning,
    Error,
}

pub struct Notifications<S> {
    items: S,
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
                        NotificationKind::Info => success_element(),
                        NotificationKind::Warning => warning_element(),
                        NotificationKind::Error => danger_element(),
                    };
                    card(label(v.message.clone())).with_background(Background::new(color))
                })
                .collect_vec();

            col(items)
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
        std::path::PathBuf::from(".")
    }
}

fn description() -> impl Widget {
    let content = Mutable::new(
        r#"Create and edit a palette of colors. Each color can be adjusted with a falloff parameter to control how quickly the color fades to white or black.

The colors are displayed in a grid with the tints of the color. The tints are generated by adjusting the chroma and lightness of the color between 50 and 950.

The colors can be save and loaded, and exported in a tailwind style `.json` file.
        "#.to_string(),
    );

    card(TextInput::new(content))
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
            l: (TINT_MAX - TINT_MIN) * (1.0 - tint) + TINT_MIN,
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

/// Going from 0.0 to 1.0 is too dark to be perceptible in the higher ranges
static TINT_MIN: f32 = 0.17;
static TINT_MAX: f32 = 0.97;
