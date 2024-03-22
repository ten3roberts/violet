#[doc(hidden)]
pub use flax;
use flax::Component;

pub struct Atom<T>(pub(crate) Component<T>);

impl<T> Atom<T> {
    #[doc(hidden)]
    pub fn from_component(component: Component<T>) -> Self {
        Self(component)
    }
}

impl<T> Clone for Atom<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Atom<T> {}

impl<T> PartialEq for Atom<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Atom<T> {}

#[macro_export]
macro_rules! declare_atom {
    ($(#[$outer:meta])* $vis: vis $name: ident: $ty: ty $(=> [$($metadata: ty),*])?, $($rest:tt)*) => {
        $(#[$outer])*
            $vis fn $name() -> $crate::atom::Atom<$ty> {
                use $crate::atom::flax::entity::EntityKind;

                static COMPONENT_ID: ::core::sync::atomic::AtomicU32 = ::core::sync::atomic::AtomicU32::new($crate::atom::flax::entity::EntityIndex::MAX);
                static VTABLE: &$crate::atom::flax::vtable::ComponentVTable<$ty> = $crate::atom::flax::component_vtable!($name: $ty $(=> [$($metadata),*])?);
                $crate::atom::Atom::from_component($crate::atom::flax::Component::static_init(&COMPONENT_ID, EntityKind::COMPONENT, VTABLE))
            }

        $crate::atom::flax::component!{ $($rest)* }
    };

}
