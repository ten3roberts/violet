use futures::Stream;
use glam::{vec2, Vec2};
use palette::Srgba;
use tween::Tweener;

use crate::{
    components::{rotation, translation},
    layout::Align,
    state::{StateDuplex, StateStream, WatchState},
    style::{shadow_knob, transition_secs, ResolvableStyle, SizeExt, StyleExt, WidgetSizeProps},
    tweens::tweens,
    unit::Unit,
    widget::{label, Rectangle, Stack},
    Scope, Widget,
};

use super::{
    base::{InteractiveWidget, TooltipOptions},
    button::{follow_selection, interaction_handler, mount_driver},
    ButtonStyle,
};

/// Diameter of the knob
const KNOB_SIZE: f32 = 20.0;

/// A toggle drawn as a knob sliding along a track, on to the right
pub struct Switch {
    state: Box<dyn Send + Sync + StateDuplex<Item = bool>>,
    style: ButtonStyle,
    tooltip: Option<TooltipOptions>,
}

impl Switch {
    pub fn new(state: impl 'static + Send + Sync + StateDuplex<Item = bool>) -> Self {
        Self {
            state: Box::new(state),
            style: ButtonStyle::switch(),
            tooltip: None,
        }
    }

    pub fn with_tooltip_text(mut self, tooltip: impl Into<String>) -> Self {
        let tooltip = tooltip.into();
        self.tooltip = Some(TooltipOptions::new(move |_| label(&tooltip)));
        self
    }
}

impl StyleExt for Switch {
    type Style = ButtonStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl SizeExt for Switch {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.style.size
    }
}

impl Widget for Switch {
    fn mount(self, scope: &mut Scope<'_>) {
        // The knob travels the track's inner width, which the min size and padding fix
        let stylesheet = scope.stylesheet();
        let width = self.style.size.min_size.map(|v| v.px.x).unwrap_or(0.0);
        let padding = self
            .style
            .size
            .padding
            .map(|v| v.resolve(stylesheet))
            .unwrap_or_default();
        let travel = (width - padding.left - padding.right - KNOB_SIZE).max(0.0);

        let knob = scope.attach(SwitchKnob {
            on: self.state.stream(),
            travel,
        });

        let (driver, background) = mount_driver(scope, &self.style, Some(knob), false);
        follow_selection(scope, driver, self.state.stream());

        let mut last_state = WatchState::new(self.state.stream());

        let inner = Stack::new(())
            .with_background(background)
            .with_horizontal_alignment(self.style.align.horizontal)
            .with_vertical_alignment(Align::Center);

        InteractiveWidget::new(inner)
            .with_size_props(self.style.size)
            .on_pointer_press(move |_, state| {
                if state.is_pressed() {
                    if let Some(state) = last_state.get() {
                        self.state.send(!state)
                    }
                }
            })
            .on_interaction_state(interaction_handler(driver))
            .with_tooltip_opt(self.tooltip)
            .mount(scope);
    }
}

/// The knob of a [`Switch`], sliding `travel` pixels to the right while on
struct SwitchKnob<S> {
    on: S,
    travel: f32,
}

impl<S: 'static + Stream<Item = bool>> Widget for SwitchKnob<S> {
    fn mount(self, scope: &mut Scope<'_>) {
        let duration = transition_secs(&scope.stylesheet());
        let travel = self.travel;

        scope
            .set_default(rotation())
            .set_default(translation())
            .set_default(tweens());

        let mut first = true;
        scope.spawn_stream(self.on, move |scope, on| {
            let target = vec2(if on { travel } else { 0.0 }, 0.0);
            if first {
                first = false;
                scope.set(translation(), target);
                return;
            }

            let current = scope.entity().get_copy(translation()).unwrap_or(Vec2::ZERO);
            scope.stop_tweens(translation());
            scope.add_tween(translation(), Tweener::cubic_out(current, target, duration));
        });

        Rectangle::new(Srgba::default())
            .with_exact_size(Unit::px2(KNOB_SIZE, KNOB_SIZE))
            .with_corner_radius(Unit::rel(1.0))
            .with_shadow(shadow_knob())
            .mount(scope);
    }
}
