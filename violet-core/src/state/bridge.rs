/// Bridge two different states together.
///
/// This is a bidirectional bridge, meaning that it can be used to bridge two states together
///
/// State updates from `A` will be sent to `B` and vice versa.
pub struct Bridge {
    a: A,
    b: B,
}
