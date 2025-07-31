use crate::foliage::{Foliage, assets::FoliageAssets};
use bevy::{prelude::*, sprite::Anchor};
use std::f32::consts::FRAC_PI_2;

#[derive(Component)]
pub struct Tree {
    pub transform: Transform,
}

impl Tree {
    const MAX_STAGE: u32 = 3;
    const ANGLE_DEVIATION: f32 = 0.125;

    pub fn spawn(
        trigger: Trigger<OnAdd, Self>,
        mut commands: Commands,
        foliage_assets: Res<FoliageAssets>,
        q_self: Query<&Self>,
    ) {
        let this = q_self.get(trigger.target()).unwrap();
        let foliage = Foliage::new("tree", 0, Self::MAX_STAGE);
        let initial_texture = foliage.get_image_handle(&foliage_assets);
        let angle = this.transform.rotation.z - FRAC_PI_2;

        commands.spawn((
            this.transform.with_rotation(Quat::from_rotation_z(angle)),
            foliage,
            Sprite {
                image: initial_texture.unwrap_or_default(),
                anchor: Anchor::BottomCenter,
                flip_x: rand::random_bool(0.5),
                ..default()
            },
        ));
    }
}
