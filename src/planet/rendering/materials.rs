use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin},
};

#[derive(AsBindGroup, TypePath, Asset, Debug, Clone)]
pub struct RockMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub mesh_size: Vec3,
}

impl Material2d for RockMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/rock_material.wgsl".into()
    }
}

impl Default for RockMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::rgb(0.4, 0.4, 0.4),
            mesh_size: Vec3::ONE,
        }
    }
}

#[derive(AsBindGroup, TypePath, Asset, Debug, Clone)]
pub struct DirtMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub mesh_size: Vec3,
}

impl Material2d for DirtMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/dirt_material.wgsl".into()
    }
}

impl Default for DirtMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::rgb(0.4, 0.2, 0.0), // Brighter gray for more visible shader effects
            mesh_size: Vec3::ONE,
        }
    }
}

pub struct PlanetMaterialPlugin;

impl Plugin for PlanetMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<RockMaterial>::default())
            .add_plugins(Material2dPlugin::<DirtMaterial>::default());
    }
}
