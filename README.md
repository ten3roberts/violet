## Violet
A retained mode GUI library focused on reactive and composable UI

Violet aims to be a simple library of minimal parts that can be composed to create complex UIs.

State and reactivity is managed locally using async Streams, such as signals or MPSC channels and `map` methods. This
allows composing a declarative reactive UI where data flows naturally from source to destination without re-renders or
useState hooks.

![image](https://github.com/ten3roberts/violet/assets/25723553/e057405d-0acf-4f88-a86d-9106e4e912a5)

## Features
- Declarative Widgets and reactive state
- Flexible layout system for responsive layouts
- Composable widgets
- Async widgets
- Thread local `!Send` + `!Sync` state and futures
- Signal based state management
- Wasm integration
- State decomposition and composition
- Renderer agnostic allowing embedding into other applications
- ECS based widget and property system (with change detection, async subscriptions, hierarchies, and more)

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
