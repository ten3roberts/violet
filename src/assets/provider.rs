/// Plugin source for assets
pub trait AssetProvider: Send + Sync {}

pub struct FsProvider {}

impl AssetProvider for FsProvider {}
