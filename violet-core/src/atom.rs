use flax::Component;

pub struct Atom<T>(pub(crate) Component<T>);

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
                use flax::entity::EntityKind;

                static COMPONENT_ID: ::core::sync::atomic::AtomicU32 = ::core::sync::atomic::AtomicU32::new(flax::entity::EntityIndex::MAX);
                static VTABLE: &flax::vtable::ComponentVTable<$ty> = flax::component_vtable!($name: $ty $(=> [$($metadata),*])?);
                $crate::atom::Atom(flax::Component::static_init(&COMPONENT_ID, EntityKind::COMPONENT, VTABLE))
            }

        flax::component!{ $($rest)* }
    };

}
