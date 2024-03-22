use std::collections::BTreeMap;

use anyhow::Context;
use flume::Sender;
use futures::Future;
use heck::ToKebabCase;
use indexmap::IndexMap;
use itertools::Itertools;
use rfd::AsyncFileDialog;
use violet::{
    core::{
        to_owned,
        widget::{centered, label, row, Button},
        Widget,
    },
    futures_signals::signal::Mutable,
    palette::{num::Sqrt, FromColor, IntoColor, Oklch, Srgb},
};

use crate::{local_dir, HexColor, Notification, NotificationKind, PaletteColor, TINTS};

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

pub fn menu_bar(
    items: Mutable<Vec<Mutable<PaletteColor>>>,
    notify_tx: Sender<Notification>,
) -> impl Widget {
    row((
        centered(label("Palette editor")),
        save_button(items.clone(), notify_tx.clone()),
        load_button(items.clone(), notify_tx.clone()),
        export_button(items.clone(), notify_tx.clone()),
    ))
    .with_stretch(true)
}

fn save_items(items: &Vec<Mutable<PaletteColor>>) -> anyhow::Result<String> {
    let data = serde_json::to_string_pretty(items).context("Failed to serialize state")?;
    Ok(data)
}

type PaletteItems = Vec<Mutable<PaletteColor>>;

fn save_button(items: Mutable<PaletteItems>, notify_tx: Sender<Notification>) -> impl Widget {
    Button::label("Save").on_press({
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

                let data = save_items(&items.lock_ref())?;

                file.write(data.as_bytes())
                    .await
                    .context("Failed to write to save file")?;

                Ok(())
            };

            frame.spawn(notify_result(fut, notify_tx, "Saved"));
        }
    })
}

fn load_button(items: Mutable<PaletteItems>, notify_tx: Sender<Notification>) -> impl Widget {
    Button::label("Load").on_press({
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
    })
}

fn export_button(items: Mutable<PaletteItems>, notify_tx: Sender<Notification>) -> impl Widget {
    Button::label("Export Json").on_press({
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

            frame.spawn(notify_result(fut, notify_tx.clone(), "Exported"));
        }
    })
}
