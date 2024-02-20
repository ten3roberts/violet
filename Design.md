# Violet Design Document

## Overview

## Terminology

- Widget - A single UI element, such as a button, text box, or image.
- Entity - The internal representation of a widgets declarative state.
- Component - A single piece of data which is attached to an entity.
- Container - A widget which contain children widgets, usually associated to a layout
- ECS - Entity Component System, used to represent the widget tree and some of the app's state

## Renderer

The rendering of widgets is a deterministic and entirely stateless and declarative system.

This leads to the widgets and the entire UI being agnostic to the rendering and backend current being used.

The core of rendering is given by the `draw_shape` component, which dictates what type of shape the widget, is and how
it would like to be drawn, such as a rectangle, text buffer, or path.

The `draw_shape` component is the read by the active renderer and dictates which additional components to read from the
widget, and ultimately how to convert it into a draw call or equivalent.

This system alleviates the ambiguity which can arise with traditional ECS implementations, where the *components*
present on an entity dictate which renderer to use. This often leads to an unpredictable system where it is both
difficult as a user to know *which* components to add to an entity to make it visible, and also difficult to ensure that
ECS systems don't conflict and fight each other. The aforementioned approach also leads to a lot of boilerplate code with
default valued components being added to entities, just for the sake of making them detectable by the intended systems;
which often lead to a concept of *Bundles*, with default implementations, and increased memory usage of identical
default data.

The Violet approach is to have a single component which dictates the shape of the widget, and then have the renderer be 
responsible for reading the additional components it needs to render the widget. This means that the user only needs to
add a few components to make a widget visible, and the renderer or other systems are then responsible for providing sane
defaults, such as color or default fonts unless explicitly specified.

This further means that it is easier to debug or visualize an entity and deduce how it will act, as well as being less
steep to approach for new users.

# Layout

Layout consists of two stages, query and apply.

The query stage determines the minimum and preferred size of a widgets, recursively.

The minimum size is the smallest a widgets can be, and preferred is how large the widget would be if it was allowed to
expand as much as possible.

This is used for the flow layout to ensure each widgets has its minimum size, and then distribute the remaining size
as much as possible for each widget to approach its preferred size.

The querying is done *as if* there was only one widgets in the container, and it does not need to take into account the
other widgets when calculating its preferred size. Think of it as a" "if I had all I needed and didn't have to share
with anyone, how big would I be?".

Parameters:
- content_area: the size of the inner container rect, used for relative sizing
- squeeze: sometimes, a widgets minimum size in one axis leads to a larger size in the other axis compared to the
  preferred size. This is especially relevant for wrapped text, where the minimum size is taller than the preferred
  slimmer size. The squeeze parameter is used to determine which direction we want to optimize for.

The apply stage is where the final dimensions of a widget have been calculated by the parent container. The widgets is
responsible for calculating its final size *within* the given bounds, and then updates its children. It returns a rect
that is its final bounding box.

This extra step allows the widgets to react to their size, update their children (such as nested flow layouts), and
respond to size changes with slight modifications, such as text wrapping or snapping to a size multiple or aspect
ratio.

Parameters:
- content_area: the size of the inner container rect, used for relative sizing, *should be used to limit a widget's max
  size*
- squeeze: when laying out a widget, it is sometimes necessary

## Recursive querying

Due to the squeeze parameter, a single container may be queried multiple times, with different squeeze axes.

Example:

```rust
Row(
    Column(
        Text("a"),
        Text("a"),
    )
)
```

The row will query the column with squeez `[1, 0]`, meaning "what is your minimum size if you optimize for minimum width".

The column will then query the two text objects with squeeze `[1, 0]` to return the total optimization for the
minimum width.

The row will then use this information to determine how to layout the children and what final size to give them.

In this case, there is only one child, so the column will receive all available size (lets say 100px).

It will then apply this limited size to the column.

The column will use these 100px and query the two text objects with `[0, 1]`, to reduce the height of the container as
much as possible, which will in extension force the text objects to use as much width as possible.

# Styling
