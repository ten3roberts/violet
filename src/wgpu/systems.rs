use flax::{entity_ids, BoxedSystem, CommandBuffer, Query, QueryBorrow, System};

use crate::assets::AssetCache;

use super::components::{self, font_from_file};

pub fn load_fonts_system(assets: AssetCache) -> BoxedSystem {
    System::builder()
        .write::<CommandBuffer>()
        .with(Query::new((entity_ids(), font_from_file().modified())))
        .build(
            move |cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
                for (id, key) in &mut query {
                    let font = assets.load(key);
                    cmd.set(id, components::font(), font);
                }
            },
        )
        .boxed()
}
