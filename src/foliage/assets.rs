use bevy::{platform::collections::HashMap, prelude::*};
use std::fs;

/// All foliage textures are stored here, with their
/// respective stage and type. (e.g., "tree" at stage 0)
#[derive(Resource, Default)]
pub struct FoliageAssets {
    pub images: HashMap<(String, u32), Handle<Image>>,
}

impl FoliageAssets {
    /// Load all foliage assets from the assets/foliage directory
    /// Expects folder structure like: assets/foliage/tree/{00,01,02}.png
    pub fn load_from_directory(asset_server: &AssetServer) -> Self {
        let mut foliage_assets = FoliageAssets::default();

        let foliage_path = "assets/foliage";
        if let Ok(entries) = fs::read_dir(foliage_path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let foliage_type = entry.file_name().to_string_lossy().to_string();
                    let type_path = entry.path();

                    if let Ok(stage_entries) = fs::read_dir(&type_path) {
                        for stage_entry in stage_entries.flatten() {
                            if let Some(filename) = stage_entry.file_name().to_str() {
                                if filename.ends_with(".png") {
                                    // Extract stage number from filename (e.g., "00.png" -> 0)
                                    if let Some(stage_str) = filename.strip_suffix(".png") {
                                        if let Ok(stage) = stage_str.parse::<u32>() {
                                            let asset_path =
                                                format!("foliage/{}/{}", foliage_type, filename);
                                            let handle = asset_server.load(asset_path);
                                            foliage_assets
                                                .images
                                                .insert((foliage_type.clone(), stage), handle);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        foliage_assets
    }
}
