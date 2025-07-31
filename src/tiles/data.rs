/// This struct contains some generic tile data so that we don't
/// have to update every single tile when adding "global" data.
pub struct GenericTileData {
    pub position_index: usize,

    /// If this is true, we have to turn off some logic for many
    /// tiles to prevent users from abusing logic whilst just using the preview
    pub is_preview: bool,
}

impl GenericTileData {
    pub fn new(position_index: usize, is_preview: bool) -> Self {
        Self {
            position_index,
            is_preview,
        }
    }
}

impl Default for GenericTileData {
    fn default() -> Self {
        Self {
            position_index: 0,
            is_preview: false,
        }
    }
}
