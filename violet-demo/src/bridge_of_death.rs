use futures::StreamExt;
use glam::Vec2;
use violet::{
    core::{
        state::{StateExt, StateStream},
        style::{surface_warning, SizeExt},
        text::TextSegment,
        unit::Unit,
        widget::{
            card, col, label, pill, row, Rectangle, SliderWithLabel, StreamWidget, Text, TextInput,
        },
        Widget,
    },
    futures_signals::signal::Mutable,
    palette::Srgba,
};

pub fn app() -> impl Widget {
    let name = Mutable::new("".to_string());
    let quest = Mutable::new("".to_string());
    let color = Mutable::new(Srgba::new(0.0, 0.61, 0.388, 1.0));

    // Map a `Mutable<Srgba>` into a `StateDuplex<f32>` for each field
    let r = color.clone().project_ref(|v| &v.red, |v| &mut v.red);
    let g = color.clone().project_ref(|v| &v.green, |v| &mut v.green);
    let b = color.clone().project_ref(|v| &v.blue, |v| &mut v.blue);

    let speed = Mutable::new(None as Option<f32>);

    col((
        card(row((label("What is your name?"), TextInput::new(name)))),
        card(row((label("What is your quest?"), TextInput::new(quest)))),
        card(col((
            label("What is your favorite colour?"),
            SliderWithLabel::new(r, 0.0, 1.0).precision(2),
            SliderWithLabel::new(g, 0.0, 1.0).precision(2),
            SliderWithLabel::new(b, 0.0, 1.0).precision(2),
            StreamWidget(color.stream().map(|v| {
                Rectangle::new(v)
                    .with_maximize(Vec2::X)
                    .with_min_size(Unit::px2(100.0, 100.0))
            })),
        ))),
        card(row((
            label("What is the airspeed velocity of an unladen swallow?"),
            // Fallibly parse and fill in the None at the same time using the `State` trait
            // combinators
            TextInput::new(speed.clone().prevent_feedback().filter_map(
                |v| v.map(|v| v.to_string()),
                |v| Some(v.parse::<f32>().ok()),
            )),
            StreamWidget(speed.stream().map(|v| {
                match v {
                    Some(v) => pill(Text::new(format!("{v} m/s"))),
                    None => pill(Text::formatted([
                        TextSegment::new("×").with_weight(violet::core::text::Weight::BOLD)
                    ]))
                    .with_background(surface_warning()),
                }
            })),
        ))),
    ))
    .with_contain_margins(true)
}
