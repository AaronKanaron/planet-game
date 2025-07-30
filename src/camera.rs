use bevy::{
    color::palettes::css::GRAY,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
    window::WindowResized,
};

use crate::planet::rendering::culling::ChunkCullingBox;

/// In-game resolution width.
const RES_WIDTH: u32 = 240;

/// In-game resolution height.
const RES_HEIGHT: u32 = 135;

/// Default render layers for pixel-perfect rendering.
/// You can skip adding this component, as this is the default.
const PIXEL_PERFECT_LAYERS: RenderLayers = RenderLayers::layer(0);

/// Render layers for high-resolution rendering.
const HIGH_RES_LAYERS: RenderLayers = RenderLayers::layer(1);

/// Low-resolution texture that contains the pixel-perfect world.
/// Canvas itself is rendered to the high-resolution world.
#[derive(Component)]
struct Canvas;

/// Camera that renders the pixel-perfect world to the [`Canvas`].
#[derive(Component)]
struct InGameCamera;

/// Camera that renders the [`Canvas`] (and other graphics on [`HIGH_RES_LAYERS`]) to the screen.
#[derive(Component)]
struct OuterCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera).add_systems(
            Update,
            (fit_canvas, /* handle_zoom */ follow_culling_center),
        );
    }
}

fn setup_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let canvas_size = Extent3d {
        width: RES_WIDTH,
        height: RES_HEIGHT,
        ..default()
    };

    // This Image serves as a canvas representing the low-resolution game screen
    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // Fill image.data with zeroes
    canvas.resize(canvas_size);

    let image_handle = images.add(canvas);

    // This camera renders whatever is on `PIXEL_PERFECT_LAYERS` to the canvas
    commands.spawn((
        Camera2d,
        Camera {
            // Render before the "main pass" camera
            order: -1,
            target: RenderTarget::Image(image_handle.clone().into()),
            clear_color: ClearColorConfig::Custom(GRAY.into()),
            ..default()
        },
        Msaa::Off,
        InGameCamera,
        PIXEL_PERFECT_LAYERS,
    ));

    // Spawn the canvas
    commands.spawn((Sprite::from_image(image_handle), Canvas, HIGH_RES_LAYERS));

    // The "outer" camera renders whatever is on `HIGH_RES_LAYERS` to the screen.
    // here, the canvas and one of the sample sprites will be rendered by this camera
    commands.spawn((Camera2d, Msaa::Off, OuterCamera, HIGH_RES_LAYERS));
}

/// Scales camera projection to fit the window (integer multiples only).
fn fit_canvas(
    mut resize_events: EventReader<WindowResized>,
    mut projection: Single<&mut Projection, With<OuterCamera>>,
) {
    let Projection::Orthographic(projection) = &mut **projection else {
        return;
    };
    for event in resize_events.read() {
        let h_scale = event.width / RES_WIDTH as f32;
        let v_scale = event.height / RES_HEIGHT as f32;
        projection.scale = 1. / h_scale.min(v_scale).round();
    }
}

/// Makes the camera follow the culling box center
fn follow_culling_center(
    culling_box: Res<ChunkCullingBox>,
    mut camera_transform: Single<&mut Transform, With<InGameCamera>>,
) {
    // Only update if the culling box center has changed
    if culling_box.is_changed() {
        // Set camera position to the culling box center
        camera_transform.translation.x = culling_box.center.x;
        camera_transform.translation.y = culling_box.center.y;
        // Keep the Z coordinate unchanged for proper 2D rendering
    }
}
