use super::base_colors::*;
use palette::Srgba;

macro_rules! colorpalette {
    ($vis: vis $name: ident, $($ident: ident: $ty: ty,)+) => {
        flax::component! {
            $($vis $ident: $ty,)+
        }

        $vis struct $name {
            $($ident: $ty,)+
        }

        impl $name {
            $vis fn install(&self, entity: &mut flax::EntityBuilder) {
                $(entity.set($ident(), self.$ident);)+
            }
        }
    }
}

colorpalette! {
    pub ColorPalette,
    surface_accent: Srgba,
    element_accent: Srgba,

    surface_primary: Srgba,
    surface_secondary: Srgba,
    surface_tertiary: Srgba,

    surface_success: Srgba,
    surface_warning: Srgba,
    surface_danger: Srgba,

    element_primary: Srgba,
    element_secondary: Srgba,
    element_tertiary: Srgba,
    element_success: Srgba,
    element_warning: Srgba,
    element_danger: Srgba,

    // interactive (base)
    element_interactive: Srgba,
    element_interactive_accent: Srgba,
    element_interactive_danger: Srgba,
    element_interactive_success: Srgba,
    element_interactive_warning: Srgba,

    surface_interactive: Srgba,
    surface_interactive_accent: Srgba,
    surface_interactive_danger: Srgba,
    surface_interactive_success: Srgba,
    surface_interactive_warning: Srgba,

    // pressed
    element_pressed: Srgba,
    element_pressed_accent: Srgba,
    element_pressed_danger: Srgba,
    element_pressed_success: Srgba,
    element_pressed_warning: Srgba,

    surface_pressed: Srgba,
    surface_pressed_accent: Srgba,
    surface_pressed_danger: Srgba,
    surface_pressed_success: Srgba,
    surface_pressed_warning: Srgba,

    // hover
    element_hover: Srgba,
    element_hover_accent: Srgba,
    element_hover_danger: Srgba,
    element_hover_success: Srgba,
    element_hover_warning: Srgba,

    surface_hover: Srgba,
    surface_hover_accent: Srgba,
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
        let element_primary = PLATINUM_50;
        let element_secondary = PLATINUM_100;
        let element_tertiary = PLATINUM_100;

        let surface_primary = STONE_950;
        let surface_secondary = STONE_900;
        let surface_tertiary = STONE_800;

        Self {
            surface_accent: EMERALD_800,
            element_accent: EMERALD_400,
            surface_primary,
            surface_secondary,
            surface_tertiary,
            element_primary,
            element_secondary,
            element_tertiary,
            //
            surface_interactive: ZINC_700,
            surface_interactive_accent: EMERALD_500,
            surface_interactive_danger: RUBY_500,
            surface_interactive_success: EMERALD_500,
            surface_interactive_warning: AMBER_500,
            //
            element_interactive: element_primary,
            element_interactive_accent: element_secondary,
            element_interactive_danger: element_secondary,
            element_interactive_success: element_secondary,
            element_interactive_warning: element_secondary,
            //
            surface_pressed: EMERALD_500,
            surface_pressed_accent: EMERALD_700,
            surface_pressed_danger: RUBY_700,
            surface_pressed_success: EMERALD_700,
            surface_pressed_warning: AMBER_700,
            //
            element_pressed: element_secondary,
            element_pressed_accent: element_secondary,
            element_pressed_danger: element_secondary,
            element_pressed_success: element_secondary,
            element_pressed_warning: element_secondary,
            //
            surface_hover: ZINC_600,
            surface_hover_accent: EMERALD_300,
            surface_hover_danger: RUBY_600,
            surface_hover_success: EMERALD_600,
            surface_hover_warning: AMBER_700,
            //
            element_hover: element_primary,
            element_hover_accent: element_secondary,
            element_hover_danger: element_secondary,
            element_hover_success: element_secondary,
            element_hover_warning: element_secondary,
            //
            surface_disabled: ZINC_800,
            surface_disabled_danger: RUBY_800,
            surface_disabled_success: EMERALD_800,
            surface_disabled_warning: AMBER_800,
            //
            element_disabled: PLATINUM_600,
            element_disabled_danger: PLATINUM_950,
            element_disabled_success: PLATINUM_950,
            element_disabled_warning: PLATINUM_950,
            //
            surface_success: EMERALD_500,
            surface_warning: AMBER_500,
            surface_danger: RUBY_500,
            element_success: EMERALD_300,
            element_warning: AMBER_300,
            element_danger: RUBY_300,
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::new()
    }
}
