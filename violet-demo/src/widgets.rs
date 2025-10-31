use glam::{BVec2, Vec2};
use itertools::Itertools;
use violet::{
    core::{
        layout::Align,
        style::{
            base_colors::*, default_corner_radius, spacing_small, surface_danger, surface_primary,
            surface_warning, text_medium, SizeExt,
        },
        unit::Unit,
        widget::{
            bold, card, col,
            interactive::{
                colorpicker::RgbColorPicker, dropdown::Dropdown, select_list::SelectList,
            },
            label, pill, row, subtitle, title, Button, Checkbox, Collapsible, LabeledSlider, Radio,
            Rectangle, ScrollArea, SignalWidget, Text, TextInput,
        },
        Edges, StateExt, Widget,
    },
    futures_signals::signal::{Mutable, SignalExt},
    lucide::icons::*,
    palette::Srgba,
};

use crate::drag;

fn dialog(name: impl Into<String>, content: impl 'static + Widget) -> impl Widget {
    card(Collapsible::new(title(name.into()), content))
}

pub fn main_app() -> impl Widget {
    ScrollArea::new(
        BVec2::TRUE,
        col((
            row((texts(), buttons(), inputs(), dropdown(), list(), colors())),
            row((icons(), drag_and_drop())),
        ))
        .with_contain_margins(true)
        .with_maximize(Vec2::ONE),
    )
    .with_background(surface_primary())
}

fn buttons() -> impl Widget {
    dialog(
        "Buttons",
        col((
            Button::label("Default"),
            Button::label("Success").success(),
            Button::label("Warning").warning(),
            Button::label("Danger").danger(),
            Button::label("Accent").accent(),
        ))
        .with_stretch(true)
        .with_cross_align(Align::Center),
    )
}

fn dropdown() -> impl Widget {
    let selection = Mutable::new(None);
    dialog(
        "Dropdown",
        Dropdown::new(
            selection.lower_option(),
            [
                row((bold(LUCIDE_BOX).with_color(SAPPHIRE_500), label("Box"))),
                row((
                    bold(LUCIDE_DROPLETS).with_color(AMETHYST_400),
                    label("Liquid"),
                )),
                row((bold(LUCIDE_HAMMER).with_color(AMBER_400), label("Tools"))),
                row((bold(LUCIDE_BACKPACK).with_color(RUBY_400), label("Items"))),
                row((
                    bold(LUCIDE_HEADPHONES).with_color(EMERALD_400),
                    label("Music"),
                )),
                row((
                    bold(LUCIDE_BRIEFCASE_BUSINESS).with_color(AMBER_500),
                    label("Business"),
                )),
                row((
                    bold(LUCIDE_WRENCH).with_color(SAPPHIRE_500),
                    label("Settings"),
                )),
                row((bold(LUCIDE_LEAF).with_color(FOREST_400), label("Nature"))),
            ],
        ),
    )
}
fn texts() -> impl Widget {
    dialog(
        "Text",
        col((
            title("Title"),
            subtitle("Subtitle"),
            Text::new("Lorem ipsum dolor sit amet"),
            pill(label("Pill")),
            Text::italic("Italic"),
            Text::bold("Bold"),
            Text::medium("Medium"),
            Text::new("Regular"),
            Text::light("Light"),
            Text::extra_light("Extra Light"),
        ))
        .with_stretch(true)
        .with_cross_align(Align::Start),
    )
}

fn colors() -> impl Widget {
    fn color(color: Srgba) -> Rectangle {
        Rectangle::new(color)
            .with_exact_size(Unit::px2(60.0, 40.0))
            .with_margin(spacing_small())
            .with_corner_radius(default_corner_radius())
    }

    dialog(
        "Colors",
        col((
            row((color(RUBY_400), color(RUBY_500), color(RUBY_600))),
            row((color(AMBER_400), color(AMBER_500), color(AMBER_600))),
            row((color(FOREST_400), color(FOREST_500), color(FOREST_600))),
            row((color(EMERALD_400), color(EMERALD_500), color(EMERALD_600))),
            row((color(TEAL_400), color(TEAL_500), color(TEAL_600))),
            row((
                color(SAPPHIRE_400),
                color(SAPPHIRE_500),
                color(SAPPHIRE_600),
            )),
            row((
                color(AMETHYST_400),
                color(AMETHYST_500),
                color(AMETHYST_600),
            )),
        ))
        .with_stretch(true)
        .with_cross_align(Align::Center),
    )
}

fn list() -> impl Widget {
    let selection = Mutable::new(None);

    dialog(
        "Selection",
        col((SelectList::new(
            selection,
            ('A'..='Z')
                .map(|v| label(format!("Item {v}")).with_min_size(Unit::px2(140.0, 0.0)))
                .collect_vec(),
        )
        .with_min_size(Unit::px2(140.0, 400.0))
        .with_max_size(Unit::px2(f32::MAX, 400.0)),))
        .with_cross_align(Align::Center),
    )
}

fn drag_and_drop() -> impl Widget {
    dialog("Drag and Drop", drag::app())
}

fn icons() -> impl Widget {
    fn icon_large(s: impl Into<String>) -> Text {
        label(s).with_font_size(text_medium())
    }

    fn icon_button<W: Widget>(w: W) -> Button<W> {
        Button::new(w).with_corner_radius(Unit::rel(1.0))
    }

    dialog(
        "Icons",
        col((row((
            icon_button(icon_large(LUCIDE_COG).with_color(AMBER_300)).with_tooltip_text("Settings"),
            icon_button(icon_large(LUCIDE_CHECK).with_color(EMERALD_500))
                .with_tooltip_text("Success"),
            icon_button(icon_large(LUCIDE_LIGHTBULB).with_color(TEAL_500))
                .with_tooltip_text("Hint"),
            icon_button(icon_large(LUCIDE_TRIANGLE_ALERT).with_color(surface_warning()))
                .with_tooltip_text("Warning"),
            icon_button(icon_large(LUCIDE_CIRCLE_X).with_color(surface_danger()))
                .with_tooltip_text("Error"),
            icon_button(
                row((
                    TextInput::new(Mutable::new("Text".to_string())).with_padding(Edges::ZERO),
                    icon_large(LUCIDE_PENCIL).with_color(SAPPHIRE_500),
                ))
                .with_cross_align(Align::Center),
            )
            .with_tooltip_text("Write some text"),
        ))
        .with_cross_align(Align::Center),)),
    )
}

fn inputs() -> impl Widget {
    let selection = Mutable::new(0);

    dialog(
        "Input",
        col((
            LabeledSlider::new(Mutable::new(50), 0, 100).editable(true),
            LabeledSlider::new(Mutable::new(10.0), 0.1, 100.0).editable(true).logarithmic().precision(2),
            TextInput::new(Mutable::new("Text Input".to_string())),
            row((
                row((0..10)
                    .map(|i| Radio::new_value(selection.clone(), i))
                    .collect_vec()),
                SignalWidget::new(selection.signal().map(|v| label(format!("Selected: {v}")))),
            )),
            row((
                bold("Checkbox: "),
                Checkbox::new(Mutable::new(false))
                    .with_tooltip_text("This is a checkbox")
            )),
            Collapsible::label(
                "Options",
                col((
                    RgbColorPicker::new(Mutable::new(FOREST_400)),
                    TextInput::new(Mutable::new("This is an editable textbox.\n\nIt supports multiple lines of text, selections, copy/pasting and more".to_string())),
                )),
            )
            .on_click(|_| {
                eprintln!("Clicked collapsible header");
            }),
        ))
        .with_stretch(true)
        .with_cross_align(Align::Center),
    )
}
