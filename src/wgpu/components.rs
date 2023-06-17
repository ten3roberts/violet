use flax::{component, Debuggable};

use crate::{assets::Handle, wgpu::graphics::texture::Texture};

component! {
    /// The gpu texture to use for rendering
    texture: Handle<Texture>,
}
