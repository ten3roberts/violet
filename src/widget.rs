use flax::Entity;

use crate::Scope;

/// Represents a widget in the UI tree which can mount itself into the frame.
///
/// Is inert before mounting
pub trait Widget {
    /// Mount the widget into the world, returning a handle to refer to it
    fn mount(self, scope: &mut Scope<'_>);
}
