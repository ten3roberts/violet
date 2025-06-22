use futures::StreamExt;
use glam::{BVec2, Vec2};
use itertools::Itertools;
use violet::{
    core::{
        layout::Align,
        state::{StateExt, StateStream},
        style::{
            base_colors::*, default_corner_radius, spacing_small, surface_danger, surface_primary,
            surface_warning, text_medium, SizeExt,
        },
        unit::Unit,
        widget::{
            card, col, interactive::select_list::SelectList, label, pill, row, subtitle, title,
            Button, Collapsible, Radio, Rectangle, ScrollArea, SignalWidget, SliderWithLabel,
            StreamWidget, Text, TextInput,
        },
        Edges, Widget,
    },
    futures_signals::signal::{Mutable, SignalExt},
    lucide::icons::*,
    palette::Srgba,
};
use web_sys::console::info;

use crate::drag;

fn dialog(name: impl Into<String>, content: impl Widget) -> impl Widget {
    card(Collapsible::new(title(name), content))
}

pub fn main_app() -> impl Widget {
    ScrollArea::new(
        BVec2::TRUE,
        col((
            row((texts(), buttons(), inputs(), list(), colors())),
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
        .with_cross_align(Align::Center),
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
            row((color(CHERRY_400), color(CHERRY_500), color(CHERRY_600))),
            row((color(AMBER_400), color(AMBER_500), color(AMBER_600))),
            row((color(CITRUS_400), color(CITRUS_500), color(CITRUS_600))),
            row((color(FOREST_400), color(FOREST_500), color(FOREST_600))),
            row((color(EMERALD_400), color(EMERALD_500), color(EMERALD_600))),
            row((color(TEAL_400), color(TEAL_500), color(TEAL_600))),
            row((color(OCEAN_400), color(OCEAN_500), color(OCEAN_600))),
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
                    icon_large(LUCIDE_PENCIL).with_color(OCEAN_400),
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

    let value = Mutable::new(0.5);
    dialog(
        "Input",
        col((
            SliderWithLabel::new(Mutable::new(50), 0, 100),
            SliderWithLabel::new(Mutable::new(50), 0, 100).editable(true),
            TextInput::new(Mutable::new("Text Input".to_string())),
            row((
                row((0..10)
                    .map(|i| Radio::new_value(selection.clone(), i))
                    .collect_vec()),
                SignalWidget::new(selection.signal().map(|v| label(format!("Selected: {v}")))),
            )),
            Collapsible::label(
                "Options",
                col((
                    SliderWithLabel::new(value.clone(), 0.0, 10.0)
                        .editable(true)
                        .precision(2),
                    TextInput::new(Mutable::new("This is an editable textbox.\n\nIt supports multiple lines of text, selections, copy/pasting and more".to_string())),
                )),
            )
            .on_click(|scope| {
                eprintln!("Clicked collapsible header");
            }),
        ))
        .with_stretch(true)
        .with_cross_align(Align::Center),
    )
}
