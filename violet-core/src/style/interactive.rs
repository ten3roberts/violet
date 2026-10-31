use std::{cell::RefCell, time::Duration};

use flax::{component::ComponentDesc, Entity, EntityRef};
use glam::Vec2;
use palette::{IntoColor, Oklab, Srgb, Srgba};

use super::{
    transition_duration, Border, BoxShadow, Decoration, ResolvableStyle, ResolvedDecoration,
    ValueOrRef,
};
use crate::{
    components::{color, rotation, translation, widget_border, widget_inset_shadow, widget_shadow},
    stored::WeakHandle,
    tweens::{tweens, DynamicTween},
    Scope,
};

/// Duration of interaction transitions and overlay fades when the stylesheet sets none
pub(crate) const DEFAULT_TRANSITION: Duration = Duration::from_millis(100);

/// Whether the pointer is over a widget and whether it holds the widget pressed
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InteractionState {
    pub hovered: bool,
    pub pressed: bool,
}

/// Look of an interactive widget in one interaction state
#[derive(Debug, Clone, Copy)]
pub struct StateStyle {
    pub surface: ValueOrRef<Srgba>,
    pub element: ValueOrRef<Srgba>,
    pub decoration: Decoration,
    /// Moves the widget without relayout; a pressed raised control drops 1px
    pub translation: Vec2,
}

impl StateStyle {
    pub fn new(
        surface: impl Into<ValueOrRef<Srgba>>,
        element: impl Into<ValueOrRef<Srgba>>,
    ) -> Self {
        Self {
            surface: surface.into(),
            element: element.into(),
            decoration: Decoration::default(),
            translation: Vec2::ZERO,
        }
    }

    pub fn with_decoration(mut self, decoration: Decoration) -> Self {
        self.decoration = decoration;
        self
    }

    pub fn with_translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }
}

/// A [`StateStyle`] resolved against the stylesheet
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedStateStyle {
    pub surface: Srgba,
    pub element: Srgba,
    pub decoration: ResolvedDecoration,
    pub translation: Vec2,
}

impl ResolvableStyle for StateStyle {
    type Value = ResolvedStateStyle;

    fn resolve(&self, stylesheet: EntityRef<'_>) -> ResolvedStateStyle {
        ResolvedStateStyle {
            surface: self.surface.resolve(stylesheet),
            element: self.element.resolve(stylesheet),
            decoration: self.decoration.resolve(stylesheet),
            translation: self.translation,
        }
    }
}

/// Looks of a widget per interaction state
#[derive(Debug, Clone, Copy)]
pub struct StateStyles<T = StateStyle> {
    pub normal: T,
    pub hover: T,
    pub pressed: T,
}

impl<T> StateStyles<T> {
    pub fn new(normal: T, hover: T, pressed: T) -> Self {
        Self {
            normal,
            hover,
            pressed,
        }
    }

    /// The look of `state`: pressed over hover over normal
    pub fn pick(&self, state: InteractionState) -> &T {
        if state.pressed {
            &self.pressed
        } else if state.hovered {
            &self.hover
        } else {
            &self.normal
        }
    }

    /// Applies `f` to the look of each state
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> StateStyles<U> {
        StateStyles {
            normal: f(self.normal),
            hover: f(self.hover),
            pressed: f(self.pressed),
        }
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        [&self.normal, &self.hover, &self.pressed].into_iter()
    }
}

impl<T: Clone> StateStyles<T> {
    /// The same look in every state
    pub fn uniform(style: T) -> Self {
        Self {
            normal: style.clone(),
            hover: style.clone(),
            pressed: style,
        }
    }
}

impl<T: ResolvableStyle> ResolvableStyle for StateStyles<T> {
    type Value = StateStyles<T::Value>;

    fn resolve(&self, stylesheet: EntityRef<'_>) -> Self::Value {
        StateStyles {
            normal: self.normal.resolve(stylesheet),
            hover: self.hover.resolve(stylesheet),
            pressed: self.pressed.resolve(stylesheet),
        }
    }
}

/// Duration of transitions and fades, from the stylesheet
pub(crate) fn transition_secs(stylesheet: &EntityRef<'_>) -> f32 {
    stylesheet
        .get_copy(transition_duration())
        .unwrap_or(DEFAULT_TRANSITION)
        .as_secs_f32()
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Mixes two colours in Oklab, alpha linearly.
///
/// A fully transparent end takes the other end's hue, so fading from transparent does not pass
/// through black.
pub(crate) fn mix_color(mut a: Srgba, mut b: Srgba, t: f32) -> Srgba {
    if a.alpha == 0.0 {
        a.color = b.color;
    }
    if b.alpha == 0.0 {
        b.color = a.color;
    }

    let la: Oklab = a.color.into_color();
    let lb: Oklab = b.color.into_color();
    let mixed = Oklab::new(
        lerp(la.l, lb.l, t),
        lerp(la.a, lb.a, t),
        lerp(la.b, lb.b, t),
    );
    let rgb: Srgb = mixed.into_color();

    Srgba::new(
        rgb.red.clamp(0.0, 1.0),
        rgb.green.clamp(0.0, 1.0),
        rgb.blue.clamp(0.0, 1.0),
        lerp(a.alpha, b.alpha, t),
    )
}

pub(crate) fn mix_border(a: Border, b: Border, t: f32) -> Border {
    Border::new(lerp(a.width, b.width, t), mix_color(a.color, b.color, t))
}

pub(crate) fn mix_shadow(a: BoxShadow, b: BoxShadow, t: f32) -> BoxShadow {
    BoxShadow {
        offset: a.offset.lerp(b.offset, t),
        blur: lerp(a.blur, b.blur, t),
        spread: lerp(a.spread, b.spread, t),
        color: mix_color(a.color, b.color, t),
    }
}

pub(crate) fn mix_state(
    a: &ResolvedStateStyle,
    b: &ResolvedStateStyle,
    t: f32,
) -> ResolvedStateStyle {
    ResolvedStateStyle {
        surface: mix_color(a.surface, b.surface, t),
        element: mix_color(a.element, b.element, t),
        decoration: ResolvedDecoration {
            border: mix_border(a.decoration.border, b.decoration.border, t),
            shadow: mix_shadow(a.decoration.shadow, b.decoration.shadow, t),
            inset_shadow: mix_shadow(a.decoration.inset_shadow, b.decoration.inset_shadow, t),
        },
        translation: a.translation.lerp(b.translation, t),
    }
}

/// Transition of a widget from one look to another.
///
/// Targets [`color`], so stopping the colour tweens of the widget cancels it.
pub(crate) struct StyleTween {
    from: ResolvedStateStyle,
    to: ResolvedStateStyle,
    element_target: Option<Entity>,
    elapsed: f32,
    duration: f32,
}

impl StyleTween {
    /// The look the widget currently shows
    pub(crate) fn current(
        entity: &EntityRef<'_>,
        element_target: Option<Entity>,
    ) -> ResolvedStateStyle {
        let element = element_target
            .and_then(|id| entity.world().entity(id).ok())
            .and_then(|v| v.get_copy(color()).ok())
            .unwrap_or_default();

        ResolvedStateStyle {
            surface: entity.get_copy(color()).unwrap_or_default(),
            element,
            decoration: ResolvedDecoration {
                border: entity.get_copy(widget_border()).unwrap_or(Border::NONE),
                shadow: entity.get_copy(widget_shadow()).unwrap_or(BoxShadow::NONE),
                inset_shadow: entity
                    .get_copy(widget_inset_shadow())
                    .unwrap_or(BoxShadow::NONE),
            },
            translation: entity.get_copy(translation()).unwrap_or(Vec2::ZERO),
        }
    }

    /// Shows `style` on the widget, on the components it has
    fn apply(entity: &EntityRef<'_>, element_target: Option<Entity>, style: &ResolvedStateStyle) {
        fn write<T: flax::component::ComponentValue>(
            entity: &EntityRef<'_>,
            component: flax::Component<T>,
            value: T,
        ) {
            if let Ok(mut v) = entity.get_mut(component) {
                *v = value;
            }
        }

        write(entity, color(), style.surface);
        write(entity, widget_border(), style.decoration.border);
        write(entity, widget_shadow(), style.decoration.shadow);
        write(entity, widget_inset_shadow(), style.decoration.inset_shadow);
        write(entity, translation(), style.translation);

        if let Some(target) = element_target.and_then(|id| entity.world().entity(id).ok()) {
            write(&target, color(), style.element);
        }
    }

    /// Transitions the widget from what it shows to `to` over `duration` seconds
    pub(crate) fn start(
        entity: &EntityRef<'_>,
        to: ResolvedStateStyle,
        element_target: Option<Entity>,
        duration: f32,
    ) {
        let from = Self::current(entity, element_target);

        let Ok(mut tweens) = entity.get_mut(tweens()) else {
            Self::apply(entity, element_target, &to);
            return;
        };

        tweens.stop_tweens(color());

        if duration <= 0.0 {
            drop(tweens);
            Self::apply(entity, element_target, &to);
            return;
        }

        tweens.add_tween(Box::new(StyleTween {
            from,
            to,
            element_target,
            elapsed: 0.0,
            duration,
        }));
    }
}

impl DynamicTween for StyleTween {
    fn update(&mut self, entity: EntityRef, delta: Duration) -> bool {
        self.elapsed += delta.as_secs_f32();
        let t = (self.elapsed / self.duration).min(1.0);
        let eased = 1.0 - (1.0 - t).powi(3);

        Self::apply(
            &entity,
            self.element_target,
            &mix_state(&self.from, &self.to, eased),
        );

        t < 1.0
    }

    fn target(&self) -> ComponentDesc {
        color().desc()
    }

    fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.duration)
    }
}

/// Shows an interactive widget's look for its interaction state and selection, transitioning
/// between them
pub(crate) struct StyleDriver {
    unselected: StateStyles<ResolvedStateStyle>,
    selected: StateStyles<ResolvedStateStyle>,
    element_target: Option<Entity>,
    interaction: InteractionState,
    is_selected: bool,
    duration: f32,
}

impl StyleDriver {
    /// Resolves the styles and shows the current look at once.
    ///
    /// Sets the surface colour, decoration and translation of the widget being mounted, and the
    /// colour of `element_target`.
    pub(crate) fn mount(
        scope: &mut Scope<'_>,
        unselected: &StateStyles,
        selected: &StateStyles,
        element_target: Option<Entity>,
        is_selected: bool,
    ) -> WeakHandle<RefCell<StyleDriver>> {
        let stylesheet = scope.stylesheet();
        let driver = StyleDriver {
            unselected: unselected.resolve(stylesheet),
            selected: selected.resolve(stylesheet),
            element_target,
            interaction: InteractionState::default(),
            is_selected,
            duration: transition_secs(&stylesheet),
        };

        let moves = unselected
            .iter()
            .chain(selected.iter())
            .any(|v| v.translation != Vec2::ZERO);

        let target = driver.target();

        scope.set_default(tweens());
        if moves {
            scope
                .set_default(rotation())
                .set(translation(), target.translation);
        }

        scope.set(color(), target.surface);
        target.decoration.mount(scope);

        if let Some(id) = element_target {
            if let Ok(entity) = scope.world().entity(id) {
                entity.update_dedup(color(), target.element);
            }
        }

        scope.store(RefCell::new(driver))
    }

    /// The look of the current state
    pub(crate) fn target(&self) -> ResolvedStateStyle {
        let styles = if self.is_selected {
            &self.selected
        } else {
            &self.unselected
        };

        *styles.pick(self.interaction)
    }

    /// Transitions to the look of `state`
    pub(crate) fn set_interaction(&mut self, entity: &EntityRef<'_>, state: InteractionState) {
        if self.interaction == state {
            return;
        }

        self.interaction = state;
        StyleTween::start(entity, self.target(), self.element_target, self.duration);
    }

    /// Shows the selected or unselected look, with a transition if `animate`
    pub(crate) fn set_selected(&mut self, entity: &EntityRef<'_>, selected: bool, animate: bool) {
        if self.is_selected == selected {
            return;
        }

        self.is_selected = selected;
        let duration = if animate { self.duration } else { 0.0 };
        StyleTween::start(entity, self.target(), self.element_target, duration);
    }
}

#[cfg(test)]
mod tests {
    use flax::{Entity, World};

    use super::*;
    use crate::style::base_colors::{EMERALD_500, ZINC_600};

    fn assert_close(a: Srgba, b: Srgba) {
        let diff = [
            a.red - b.red,
            a.green - b.green,
            a.blue - b.blue,
            a.alpha - b.alpha,
        ];
        assert!(
            diff.iter().all(|v| v.abs() < 1e-3),
            "{a:?} is not close to {b:?}"
        );
    }

    #[test]
    fn mix_color_endpoints() {
        assert_close(mix_color(EMERALD_500, ZINC_600, 0.0), EMERALD_500);
        assert_close(mix_color(EMERALD_500, ZINC_600, 1.0), ZINC_600);
    }

    #[test]
    fn mix_color_from_transparent() {
        let mixed = mix_color(Srgba::new(0.0, 0.0, 0.0, 0.0), ZINC_600, 0.5);
        assert_close(
            mixed,
            Srgba::new(ZINC_600.red, ZINC_600.green, ZINC_600.blue, 0.5),
        );
    }

    #[test]
    fn pick_priority() {
        let styles = StateStyles::new(1, 2, 3);
        let state = |hovered, pressed| InteractionState { hovered, pressed };

        assert_eq!(*styles.pick(state(false, false)), 1);
        assert_eq!(*styles.pick(state(true, false)), 2);
        assert_eq!(*styles.pick(state(false, true)), 3);
        assert_eq!(*styles.pick(state(true, true)), 3);
    }

    #[test]
    fn style_tween_finishes() {
        let style = |surface| ResolvedStateStyle {
            surface,
            element: Srgba::default(),
            decoration: ResolvedDecoration {
                border: Border::NONE,
                shadow: BoxShadow::NONE,
                inset_shadow: BoxShadow::NONE,
            },
            translation: Vec2::ZERO,
        };
        let from = Srgba::new(0.0, 0.0, 0.0, 1.0);
        let to = Srgba::new(1.0, 1.0, 1.0, 1.0);

        let mut world = World::new();
        let id = Entity::builder().set(color(), from).spawn(&mut world);
        let entity = world.entity(id).unwrap();

        let mut tween = StyleTween {
            from: style(from),
            to: style(to),
            element_target: None,
            elapsed: 0.0,
            duration: 0.1,
        };

        assert!(tween.update(entity, Duration::from_millis(50)));
        let halfway = entity.get_copy(color()).unwrap();
        assert!(halfway.red > 0.0 && halfway.red < 1.0, "{halfway:?}");

        assert!(!tween.update(entity, Duration::from_millis(100)));
        assert_close(entity.get_copy(color()).unwrap(), to);
    }
}
