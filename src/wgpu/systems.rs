use flax::{entity_ids, BoxedSystem, CommandBuffer, FetchExt, Query, QueryBorrow, System};
use fontdue::layout::{Layout, TextStyle};
use glam::{vec2, Vec2};

use crate::{
    assets::AssetCache,
    components::{font_size, intrinsic_size, text},
};

use super::components::{self, font, font_from_file};

pub fn load_fonts_system(assets: AssetCache) -> BoxedSystem {
    System::builder()
        .with_cmd_mut()
        .with_query(Query::new((entity_ids(), font_from_file().modified())))
        .build(
            move |cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
                for (id, key) in &mut query {
                    let font = assets.load(key);
                    tracing::info!(?id, "Set font {key:?}");
                    cmd.set(id, components::font(), font);
                }
            },
        )
        .boxed()
}

pub fn update_text_heuristics() -> BoxedSystem {
    System::builder()
        .with_query(Query::new((
            (font(), text(), font_size().opt_or(11.0)).modified(),
            intrinsic_size().as_mut(),
        )))
        .for_each(|((font, text, font_size), instrinsic_size)| {
            // Update intrinsic sizes

            let mut layout = Layout::<()>::new(fontdue::layout::CoordinateSystem::PositiveYDown);

            layout.reset(&fontdue::layout::LayoutSettings {
                x: 0.0,
                y: 0.0,
                max_width: None,
                max_height: None,
                horizontal_align: fontdue::layout::HorizontalAlign::Left,
                vertical_align: fontdue::layout::VerticalAlign::Top,
                line_height: 1.0,
                wrap_style: fontdue::layout::WrapStyle::Word,
                wrap_hard_breaks: true,
            });

            layout.append(
                &[&**font],
                &TextStyle {
                    text,
                    px: *font_size,
                    font_index: 0,
                    user_data: (),
                },
            );

            let max = layout
                .glyphs()
                .iter()
                .map(|v| vec2(v.x + v.width as f32, v.y + v.height as f32))
                .fold(Vec2::ZERO, |acc, v| acc.max(v));

            let size = max;

            *instrinsic_size = size;
        })
        .boxed()
}
