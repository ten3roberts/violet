#[macro_export]
macro_rules! to_owned {
    ($($ident: ident),*) => (
        $(let $ident = $ident.to_owned();)*
    )
}
