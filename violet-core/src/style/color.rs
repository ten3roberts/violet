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

#[derive(Clone, Copy, Debug)]
pub struct ColorSchemeConfig {
    surface: Neutrals,
    element: Neutrals,
    accent_color: Srgba,
    light: bool,
}

impl Default for ColorSchemeConfig {
    fn default() -> Self {
        Self {
            // TODO: from_temperature and store shades instead
            surface: Neutrals::dark(ColorTemperature::Cool),
            element: Neutrals::light(ColorTemperature::Warm),
            // TODO: primary,secondary,tertiary class
            accent_color: EMERALD_500,
            light: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Neutrals {
    primary: Srgba,
    secondary: Srgba,
    tertiary: Srgba,
}

impl Neutrals {
    pub fn light(temperature: ColorTemperature) -> Self {
        match temperature {
            ColorTemperature::Neutral => Self {
                primary: STONE_50,
                secondary: STONE_100,
                tertiary: STONE_200,
            },
            ColorTemperature::Warm => Self {
                primary: PLATINUM_50,
                secondary: PLATINUM_100,
                tertiary: PLATINUM_200,
            },
            ColorTemperature::Cool => Self {
                primary: ZINC_50,
                secondary: ZINC_100,
                tertiary: ZINC_200,
            },
        }
    }

    fn dark(temperature: ColorTemperature) -> Self {
        match temperature {
            ColorTemperature::Neutral => Self {
                primary: STONE_950,
                secondary: STONE_900,
                tertiary: STONE_800,
            },
            ColorTemperature::Warm => Self {
                primary: PLATINUM_950,
                secondary: PLATINUM_900,
                tertiary: PLATINUM_800,
            },
            ColorTemperature::Cool => Self {
                primary: ZINC_950,
                secondary: ZINC_900,
                tertiary: ZINC_800,
            },
        }
    }

    pub fn new(temperature: ColorTemperature, light: bool) -> Self {
        if light {
            Self::light(temperature)
        } else {
            Self::dark(temperature)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ColorTemperature {
    Neutral,
    Warm,
    Cool,
}

impl ColorPalette {
    pub fn new(config: ColorSchemeConfig) -> Self {
        let surface = config.surface;
        let element = config.element;

        Self {
            surface_accent: EMERALD_800,
            element_accent: config.accent_color,
            surface_primary: surface.primary,
            surface_secondary: surface.secondary,
            surface_tertiary: surface.tertiary,
            element_primary: element.primary,
            element_secondary: element.secondary,
            element_tertiary: element.tertiary,
            //
            surface_interactive: ZINC_700,
            surface_interactive_accent: config.accent_color,
            surface_interactive_danger: RUBY_500,
            surface_interactive_success: EMERALD_500,
            surface_interactive_warning: AMBER_500,
            //
            element_interactive: element.primary,
            element_interactive_accent: element.secondary,
            element_interactive_danger: element.secondary,
            element_interactive_success: element.secondary,
            element_interactive_warning: element.secondary,
            //
            surface_pressed: config.accent_color,
            surface_pressed_accent: EMERALD_700,
            surface_pressed_danger: RUBY_700,
            surface_pressed_success: EMERALD_700,
            surface_pressed_warning: AMBER_700,
            //
            element_pressed: element.secondary,
            element_pressed_accent: element.secondary,
            element_pressed_danger: element.secondary,
            element_pressed_success: element.secondary,
            element_pressed_warning: element.secondary,
            //
            surface_hover: ZINC_600,
            surface_hover_accent: EMERALD_300,
            surface_hover_danger: RUBY_600,
            surface_hover_success: EMERALD_600,
            surface_hover_warning: AMBER_700,
            //
            element_hover: element.primary,
            element_hover_accent: element.secondary,
            element_hover_danger: element.secondary,
            element_hover_success: element.secondary,
            element_hover_warning: element.secondary,
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
