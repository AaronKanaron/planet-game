pub mod data;
pub mod preview;
pub mod variants;

use crate::tiles::{preview::TilePreviewPlugin, variants::debug::DebugTilePlugin};
use bevy::prelude::*;

pub struct TilePlugin;
impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilePreviewPlugin)
            .add_plugins((DebugTilePlugin,));
    }
}
