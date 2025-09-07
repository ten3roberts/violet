use std::time::Duration;

use flax::{component, Debuggable, Entity, EntityRef, Exclusive};
use glam::{Mat4, Vec2};
use image::DynamicImage;
use palette::Srgba;

use crate::{
    assets::Asset,
    layout::{Align, Layout, LayoutArgs, SizeResolver},
    stored::UntypedHandle,
    text::{LayoutGlyphs, TextSegment, Wrap},
    unit::Unit,
    Edges, Frame, Rect, Scope,
};

#[derive(Default, Debug, Clone, Copy)]
pub struct LayoutAlignment {
    pub horizontal: Align,
    pub vertical: Align,
}

impl LayoutAlignment {
    pub fn new(horizontal: Align, vertical: Align) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    pub fn left_center() -> Self {
        Self {
            horizontal: Align::Start,
            vertical: Align::Center,
        }
    }

    pub fn center() -> Self {
        Self {
            horizontal: Align::Center,
            vertical: Align::Center,
        }
    }

    pub fn right_center() -> Self {
        Self {
            horizontal: Align::End,
            vertical: Align::Center,
        }
    }

    pub fn top_left() -> Self {
        Self {
            horizontal: Align::Start,
            vertical: Align::Start,
        }
    }

    pub fn top_center() -> Self {
        Self {
            horizontal: Align::Center,
            vertical: Align::Start,
        }
    }

    pub fn top_right() -> Self {
        Self {
            horizontal: Align::End,
            vertical: Align::Start,
        }
    }

    pub fn bottom_left() -> Self {
        Self {
            horizontal: Align::Start,
            vertical: Align::End,
        }
    }

    pub fn bottom_center() -> Self {
        Self {
            horizontal: Align::Center,
            vertical: Align::End,
        }
    }

    pub fn bottom_right() -> Self {
        Self {
            horizontal: Align::End,
            vertical: Align::End,
        }
    }
}

impl LayoutAlignment {
    pub fn align(&self, total_size: Vec2, item_size: Vec2) -> Vec2 {
        let x = self.horizontal.align_offset(total_size.x, item_size.x);
        let y = self.vertical.align_offset(total_size.y, item_size.y);
        Vec2::new(x, y)
    }
}

component! {
    /// Ordered list of children for an entity
    pub children: Vec<Entity> => [ Debuggable ],
    // pub child_of(parent): Entity => [ Debuggable ],

    /// Defines the outer bounds of a widget relative to its position
    pub rect: Rect => [ Debuggable ],
    /// Clips rendering to the bounds of the widget, relative to the widget itself
    pub clip_mask: Rect => [ Debuggable ],

    /// The merged clip mask of the widget and its parents
    pub screen_clip_mask: Rect => [ Debuggable ],

    pub screen_mask_tranform: Vec2,

    /// Position relative to parent for layout position.
    pub local_position: Vec2 => [ Debuggable ],

    /// Offset the widget from its original position.
    ///
    /// This influences the layout bounds and the final position of the widget, and will move other
    /// widgets around in flow layouts.
    pub offset: Unit<Vec2> => [ Debuggable ],

    /// Optional transform of the widget. Applied after layout
    pub transform: Mat4,

    pub rotation: f32 => [ Debuggable ],

    pub transform_origin: Vec2 => [ Debuggable ],

    pub translation: Vec2 => [ Debuggable ],

    pub screen_transform: Mat4,

    pub visible: bool,

    pub computed_visible: bool,

    /// Explicit widget size. This will override the intrinsic size of the widget.
    ///
    /// The final size may be smaller if there is not enough space.
    pub size: Unit<Vec2> => [ Debuggable ],

    /// The minimum allowed size of a widget. A widgets bound will not be made any smaller even if
    /// that implies clipping/overflow.
    pub min_size: Unit<Vec2> => [ Debuggable ],

    /// The maximum allowed size of the widget.
    ///
    /// This is to constrain an upper size for containers or relatively sized widgets
    pub max_size: Unit<Vec2> => [ Debuggable ],

    /// Constrain the aspect ratio of a widget
    pub aspect_ratio: f32 => [ Debuggable ],

    /// Set the origin or anchor point of a widget.
    ///
    /// This determines the center of positioning and rotation
    pub anchor: Unit<Vec2> => [ Debuggable ],


    /// Manages the layout of the children
    pub layout: Layout => [ Debuggable ],

    /// Spacing between a outer and inner bounds
    ///
    /// Only applicable for containers
    pub padding: Edges => [ Debuggable ],

    /// Override alignment of a single item for supported containers.
    ///
    /// Useful for aligning individual items within a [`Stack`](crate::widget::Stack)
    pub item_align: LayoutAlignment => [ Debuggable ],
    /// Spacing between the item outer bounds and another items outer bounds
    ///
    /// Margins will be merged
    ///
    /// A margin is in essence a minimum allowed distance to another items bounds
    pub margin: Edges => [ Debuggable ],

    pub maximize: Vec2 => [ Debuggable ],

    pub text: Vec<TextSegment> => [ ],
    pub text_wrap: Wrap => [ Debuggable ],
    pub font_size: f32 => [ Debuggable ],

    /// To retain consistent text wrapping between size query and the snug fitted rect the bounds
    /// of the size query are stored and used instead of the snug-fitted rect which will cause a
    /// different wrapping, and therefore final size.
    pub layout_bounds: Vec2 => [ Debuggable ],

    pub layout_args: LayoutArgs => [ Debuggable ],

    /// The color of the widget
    pub color: Srgba => [ Debuggable ],

    pub opacity: f32 => [ Debuggable ],
    pub computed_opacity: f32 => [ Debuggable ],

    pub widget_corner_radius: Unit<f32> => [ Debuggable ],

    /// The widget will be rendered as a filled rectange coverings its bounds
    pub image: Asset<DynamicImage> => [ Debuggable ],

    pub draw_shape(variant): () => [ Debuggable, Exclusive ],

    pub size_resolver: Box<dyn SizeResolver>,

    /// If present, contains information about the laid out text
    pub layout_glyphs: LayoutGlyphs,

    pub(crate) atoms,

    pub(crate) context_store(id): (),

    pub on_animation_frame: OnAnimationFrame,

    pub handles: Vec<UntypedHandle>,

    /// Intercept detach events for the entity
    pub handle_detach: Option<Box<dyn FnOnce(&mut Scope<'_>) + Send + Sync>>,
}

component! {
    /// Singleton app instance entity
    pub app_instance,

    pub delta_time: Duration,
}

pub type OnAnimationFrame = Box<dyn FnMut(&Frame, &EntityRef, Duration, Duration) + Send + Sync>;
