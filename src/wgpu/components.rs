use flax::{component, Debuggable};

use crate::{
    assets::Handle,
    wgpu::{
        font::{Font, FontFromFile},
        graphics::texture::Texture,
    },
};

component! {
    /// The gpu texture to use for rendering
    pub(crate) texture: Handle<Texture>,

    pub(crate) font: Handle<Font>,

    pub font_from_file: FontFromFile => [ Debuggable ],
    pub text_mesh: Handle<Mesh>,
}
