## Violet
A retained mode GUI library focused on reactive and composable UI

Violet aims to be a simple library of minimal parts that can be composed to create complex UIs.

State and reactivity is managed locally using async Streams, such as signals or MPSC channels and `map` methods. This
allows composing a declarative reactive UI where data flows naturally from source to destination without re-renders or
useState hooks.

## Features
- Declarative Widgets and reactive state
- Flexible layout system for responsive layouts
- Composable widgets
- First class async and stream based widget reactivity
- Thread local `!Send` + `!Sync` state and futures
- Signal based state management
- Wasm/web compatible
- Wgpu rendering
- State decomposition and composition
- Renderer agnostic allowing embedding into other applications
- ...and more

## [Live Demo](https://ten3roberts.github.io/violet/demo)
![image](https://github.com/user-attachments/assets/af3c1e0b-2720-476e-93ae-317cdf7d0baf)

## Example
![image](https://github.com/ten3roberts/violet/assets/25723553/b9882e28-9e4b-49be-8dcc-9c12d42e12b1)

```rust
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
        SliderWithLabel::new(r, 0.0, 1.0).round(0.01),
        SliderWithLabel::new(g, 0.0, 1.0).round(0.01),
        SliderWithLabel::new(b, 0.0, 1.0).round(0.01),
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
                None => pill(Text::rich([
                    TextSegment::new("×").with_weight(violet::core::text::Weight::BOLD)
                ]))
                .with_background(danger_surface()),
            }
        })),
    ))),
))
```
## State Management

State is primarily managed through [`futures-signals`](https://github.com/Pauan/rust-signals).

State can be decomposed into smaller parts, composed into larger parts, or be mapped to different types using the built-in state morphism (https://docs.rs/violet/0.1.0/violet/state/).

This allows mapping state from a struct to a string field for use in a text input widget, or mapping a larger user state to a different type to render reactively in a stream.

These state morphisms and bidirectional which means it allows mapping to another type of state and back, supporting both
read and write operations.

This makes sliders and text inputs targeting individual fields or even fallible operations such as parsing a string into a number trivial.

## Reactivity

Reactivity goes hand in hand with the state management. 

State can be converted into an async stream of changes using [StateStream](https://docs.rs/violet/0.1.0/violet/state/trait.StateStream.html) and then mapped, filtered and combined using Rust's conventional stream combinators into a widget, such as a text display or color preview.

Most notable is that state and reactivity is managed locally, meaning that each widget can have its own state and reactivity without affecting the rest of the application.

## Layout System

Violet uses a custom layout system that allows for a flexible layouts that respond to different sizes.

Each widgets has a preferred size and a minimum size. The layout system uses these sizes to determine how to distribute
the available space between widgets.

### Layouts
- Flow - Works similar to a flexbox and distributes widgets in a row or column based on the available space and each
  widgets minimum and preferred size.
- Stack - Stacks widgets on top of each other. Can be used to create overlays or centering or aligning widgets.
- Float - Floats widgets on top of each other. Can be used to create tooltips or floating popups.

## Contributing
Contributions are always welcome! Feel free to open an issue or a PR.
