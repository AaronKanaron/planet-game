use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin as FpsOverlayBevy},
    prelude::*,
    text::FontSmoothing,
};

pub struct FpsOverlayPlugin;
impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FpsOverlayBevy {
            config: FpsOverlayConfig {
                text_config: TextFont {
                    font_size: 20.0,
                    font: default(),
                    font_smoothing: FontSmoothing::default(),
                    ..default()
                },
                text_color: Color::WHITE,
                refresh_interval: core::time::Duration::from_millis(20),
                enabled: true,
            },
        });
    }
}
