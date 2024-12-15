use super::{base_colors::*, EMERALD_950};
use palette::Srgba;

pub struct ColorPalette {
    surface_primary: Srgba,
    surface_secondary: Srgba,
    surface_tertiary: Srgba,

    element_primary: Srgba,
    element_secondary: Srgba,
    element_tertiary: Srgba,

    // interactive (base)
    element_interactive: Srgba,
    element_interactive_danger: Srgba,
    element_interactive_success: Srgba,
    element_interactive_warning: Srgba,

    surface_interactive: Srgba,
    surface_interactive_danger: Srgba,
    surface_interactive_success: Srgba,
    surface_interactive_warning: Srgba,

    // active
    element_active: Srgba,
    element_active_danger: Srgba,
    element_active_success: Srgba,
    element_active_warning: Srgba,

    surface_active: Srgba,
    surface_active_danger: Srgba,
    surface_active_success: Srgba,
    surface_active_warning: Srgba,

    // hover
    element_hover: Srgba,
    element_hover_danger: Srgba,
    element_hover_success: Srgba,
    element_hover_warning: Srgba,

    surface_hover: Srgba,
    surface_hover_danger: Srgba,
    surface_hover_success: Srgba,
    surface_hover_warning: Srgba,

    // disabled
    element_disabled: Srgba,
    element_disabled_danger: Srgba,
    element_disabled_success: Srgba,
    element_disabled_warning: Srgba,

    surface_disabled: Srgba,
    surface_disabled_danger: Srgba,
    surface_disabled_success: Srgba,
    surface_disabled_warning: Srgba,
}

impl ColorPalette {
    pub fn new() -> Self {
        Self {
            surface_primary: ZINC_900,
            surface_secondary: ZINC_800,
            surface_tertiary: ZINC_700,
            element_primary: PLATINUM_50,
            element_secondary: PLATINUM_100,
            element_tertiary: PLATINUM_200,
            element_interactive: PLATINUM_50,
            element_interactive_danger: CHERRY_950,
            element_interactive_success: EMERALD_950,
            element_interactive_warning: AMBER_950,
            surface_interactive: ZINC_700,
            surface_interactive_danger: CHERRY_400,
            surface_interactive_success: EMERALD_400,
            surface_interactive_warning: AMBER_400,
            element_active: EMERALD_950,
            element_active_danger: CHERRY_950,
            element_active_success: EMERALD_950,
            element_active_warning: AMBER_950,
            surface_active: EMERALD_500,
            surface_active_danger: CHERRY_500,
            surface_active_success: EMERALD_500,
            surface_active_warning: AMBER_500,
            element_hover: PLATINUM_50,
            element_hover_danger: CHERRY_950,
            element_hover_success: EMERALD_950,
            element_hover_warning: AMBER_950,
            surface_hover: ZINC_600,
            surface_hover_danger: CHERRY_600,
            surface_hover_success: EMERALD_600,
            surface_hover_warning: AMBER_600,
            element_disabled: ZINC_800,
            element_disabled_danger: CHERRY_950,
            element_disabled_success: EMERALD_950,
            element_disabled_warning: AMBER_950,
            surface_disabled: ZINC_800,
            surface_disabled_danger: CHERRY_800,
            surface_disabled_success: EMERALD_800,
            surface_disabled_warning: AMBER_800,
        }
    }
}
