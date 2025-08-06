use crate::utils::debug::framerate_overlay::FpsOverlayPlugin;
use bevy::prelude::*;

/// Just a simple debug rotate thing for checking entities
#[derive(Component)]
pub struct DebugRotate;
impl DebugRotate {
    pub fn update(mut query: Query<(&mut Transform, &DebugRotate)>, time: Res<Time>) {
        for (mut transform, _) in query.iter_mut() {
            transform.rotate(Quat::from_rotation_y(time.delta_secs()));
        }
    }
}

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FpsOverlayPlugin)
            .add_systems(Update, DebugRotate::update);
    }
}
