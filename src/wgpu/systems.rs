use flax::{entity_ids, BoxedSystem, CommandBuffer, FetchExt, Query, QueryBorrow, System};

use crate::{assets::AssetCache, components::font_family};

use super::{
    components::{self},
    font_map::FontMap,
};

pub fn load_fonts_system(font_map: FontMap) -> BoxedSystem {
    System::builder()
        .with_cmd_mut()
        .with_query(Query::new((entity_ids(), font_family().modified())))
        .build(
            move |cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
                for (id, font) in &mut query {
                    let font = match font_map.get(font) {
                        Ok(v) => v,
                        Err(err) => {
                            tracing::error!("Error loading font: {:?}", err);
                            continue;
                        }
                    };

                    cmd.set(id, components::font_handle(), font);
                }
            },
        )
        .boxed()
}
