use palette::Srgba;

pub struct ColorPalette {
    // Main color for elements
    primary_surface: Srgba,
    // Color for text/elements on a primary surface
    primary_element: Srgba,

    secondary_surface: Srgba,
    secondary_element: Srgba,

    interactive_surface: Srgba,
    interactive_element: Srgba,

    active_surface: Srgba,
    active_element: Srgba,

    disabled_surface: Srgba,
    disabled_element: Srgba,
}
