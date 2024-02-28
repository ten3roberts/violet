use std::time::Duration;

use flax::{fetch::entity_refs, Query};

use crate::{components::on_animation_frame, Frame};

pub fn update_animations(frame: &mut Frame, time: Duration) {
    let mut query = Query::new((entity_refs(), on_animation_frame().as_mut()));

    for (entity, func) in query.borrow(&frame.world).iter() {
        (func)(frame, &entity, time);
    }
}
