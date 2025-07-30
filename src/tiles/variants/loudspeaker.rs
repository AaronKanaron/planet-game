use crate::tiles::{data::GenericTileData, preview::TilePreview};
use bevy::{prelude::*, sprite::Anchor};

#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

#[derive(Component)]
pub struct Loudspeaker {
    data: GenericTileData,
}

impl Loudspeaker {
    pub fn new(data: GenericTileData) -> Self {
        Self { data }
    }

    fn spawn(
        trigger: Trigger<OnAdd, Self>,
        query: Query<&Self>,
        asset_server: Res<AssetServer>,
        mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
        mut commands: Commands,
    ) -> () {
        let this = query.get(trigger.target()).unwrap();
        let texture = asset_server.load("tiles/loudspeaker/loudspeaker.png");
        let layout = TextureAtlasLayout::from_grid(UVec2::new(32, 48), 15, 1, None, None);
        let texture_atlas_layout = texture_atlas_layouts.add(layout);
        let animation_indices = AnimationIndices { first: 0, last: 14 };

        let mut component = commands.spawn((
            Sprite {
                anchor: Anchor::BottomCenter,
                image: texture,
                texture_atlas: Some(TextureAtlas {
                    layout: texture_atlas_layout,
                    index: animation_indices.first,
                }),
                ..default()
            },
            AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
            animation_indices,
        ));

        if this.data.is_preview {
            component.insert(TilePreview::default());
        }
    }

    fn animate_sprite(
        time: Res<Time>,
        mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
    ) {
        for (indices, mut timer, mut sprite) in &mut query {
            timer.tick(time.delta());

            if timer.just_finished() {
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = if atlas.index == indices.last {
                        indices.first
                    } else {
                        atlas.index + 1
                    };
                }
            }
        }
    }
}

pub struct LoudspeakerPlugin;
impl Plugin for LoudspeakerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Loudspeaker::spawn).add_systems(
            Update,
            Loudspeaker::animate_sprite.after(Loudspeaker::spawn),
        );
    }
}
