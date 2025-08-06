use crate::{
    foliage::{Foliage, assets::FoliageAssets},
    utils::debug::plugin::DebugRotate,
};
use bevy::{prelude::*, sprite::Anchor};
use std::f32::consts::FRAC_PI_2;

#[derive(Component)]
pub struct Tree {
    pub transform: Transform,
}

impl Tree {
    const MAX_STAGE: u32 = 3;

    pub fn spawn(
        trigger: Trigger<OnAdd, Self>,
        mut commands: Commands,
        foliage_assets: Res<FoliageAssets>,
        q_self: Query<&Self>,
    ) {
        let this = q_self.get(trigger.target()).unwrap();
        let foliage = Foliage::new("tree", 0, Self::MAX_STAGE);
        let initial_texture = foliage.get_image_handle(&foliage_assets);
        let angle = this.transform.rotation.to_euler(EulerRot::XYZ).2 - FRAC_PI_2;
        commands.entity(trigger.target()).insert((
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

pub struct TreePlugin;
impl Plugin for TreePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Tree::spawn);
    }
}
