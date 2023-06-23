use flax::{entity_ids, BoxedSystem, CommandBuffer, Query, QueryBorrow, System};

use crate::assets::AssetCache;

use super::components::{self, font, font_from_file};
