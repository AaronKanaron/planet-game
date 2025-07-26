use crate::utils::debug::framerate_overlay::FpsOverlayPlugin;
use bevy::prelude::*;

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FpsOverlayPlugin);
    }
}
