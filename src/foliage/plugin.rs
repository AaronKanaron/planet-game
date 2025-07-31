use crate::foliage::{Foliage, FoliageGrowing, assets::FoliageAssets};
use bevy::{prelude::*, time::common_conditions::on_timer};
use std::time::Duration;

pub struct FoliagePlugin;

impl FoliagePlugin {
    fn setup_foliage_assets(
        mut foliage_assets: ResMut<FoliageAssets>,
        asset_server: Res<AssetServer>,
    ) {
        *foliage_assets = FoliageAssets::load_from_directory(&asset_server);
    }

    /// Periodically grows foliage
    fn foliage_growth(
        mut commands: Commands,
        mut foliage_query: Query<(Entity, &mut Foliage, &mut Sprite), With<FoliageGrowing>>,
        foliage_assets: Res<FoliageAssets>,
    ) {
        for (entity, mut foliage, mut sprite) in foliage_query.iter_mut() {
            let should_grow = rand::random::<f32>() < 0.02; // growth_chance
            if !should_grow {
                continue;
            }

            if foliage.grow() {
                if let Some(texture_handle) = foliage.get_image_handle(&foliage_assets) {
                    sprite.image = texture_handle;
                }
            } else {
                // Remove growing component when fully grown
                commands.entity(entity).remove::<FoliageGrowing>();
            }
        }
    }
}

impl Plugin for FoliagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FoliageAssets>()
            .add_systems(Startup, Self::setup_foliage_assets)
            .add_systems(
                Update,
                Self::foliage_growth.run_if(on_timer(Duration::from_secs(1))),
            );
    }
}
