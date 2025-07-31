use crate::tiles::{data::GenericTileData, preview::TilePreview};
use bevy::{color::palettes::tailwind::GREEN_500, prelude::*};

#[derive(Component)]
pub struct DebugTile {
    data: GenericTileData,
}

impl DebugTile {
    pub fn new(data: GenericTileData) -> Self {
        Self { data }
    }

    fn spawn(trigger: Trigger<OnAdd, Self>, query: Query<&Self>, mut commands: Commands) -> () {
        let this = query.get(trigger.target()).unwrap();

        let text_entity = commands
            .spawn((
                Text2d::new(this.data.position_index.to_string()),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ))
            .id();

        let mut this_entity = commands.get_entity(trigger.target()).unwrap();
        this_entity
            .insert((
                Sprite {
                    color: GREEN_500.into(),
                    custom_size: Some(Vec2::splat(40.0)),
                    ..default()
                },
                // planet.get_transform_from_index(this.data.position_index),
            ))
            .add_child(text_entity);

        if this.data.is_preview {
            this_entity.insert(TilePreview::default());
        }
    }
}

pub struct DebugTilePlugin;
impl Plugin for DebugTilePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(DebugTile::spawn);
    }
}
