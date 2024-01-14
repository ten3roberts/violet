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
