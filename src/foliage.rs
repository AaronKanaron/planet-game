pub mod assets;
pub mod plugin;
pub mod variants;

use crate::foliage::assets::FoliageAssets;
use bevy::prelude::*;

#[derive(Component)]
pub struct Foliage {
    /// Important: This should be lowercased, and should match the
    /// directory of the texture in the assets folder. E.g "tree"
    name: String,
    stage: u32,
    max_stage: u32,

    /// Each foliage update tick (e.g every second) has a chance to grow the foliage.
    growth_chance: f32,
}

/// Removed from foliage bundles when the foliage is fully grown
/// to make queries more efficient
#[derive(Component)]
pub struct FoliageGrowing;

impl Foliage {
    pub fn new(name: &str, stage: u32, max_stage: u32) -> Self {
        Foliage {
            stage,
            max_stage,
            name: Self::parse_name(name),
            growth_chance: 0.02,
        }
    }

    /// Create a new foliage that doesn't grow
    pub fn new_static(name: &str) -> Self {
        Foliage {
            stage: 0,
            max_stage: 0,
            name: Self::parse_name(name),
            growth_chance: 0.0,
        }
    }

    /// Won't exceed `max_stage`. Returns `true` if the foliage has grown.
    pub fn grow(&mut self) -> bool {
        if self.stage < self.max_stage {
            self.stage += 1;
            return true;
        }

        false
    }

    pub fn stage(&self) -> u32 {
        self.stage
    }

    pub fn max_stage(&self) -> u32 {
        self.max_stage
    }

    /// Returns an image handle for the current stage of foliage
    pub fn get_image_handle(&self, foliage_assets: &FoliageAssets) -> Option<Handle<Image>> {
        // Okay to clone here because name is often short like "tree"
        foliage_assets
            .images
            .get(&(self.name.clone(), self.stage))
            .cloned()
    }

    // Via assertions
    fn parse_name(name: &str) -> String {
        assert!(!name.is_empty(), "Foliage name cannot be empty");
        assert!(
            name.chars().all(|c| c.is_lowercase()),
            "Foliage name must be lowercase: {}",
            name
        );
        assert!(
            name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "Foliage name can only contain alphanumeric characters and underscores: {}",
            name
        );

        name.to_string()
    }
}
