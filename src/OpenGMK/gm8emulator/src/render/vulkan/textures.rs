//! Bindless texture arrays and descriptor sets (VK_EXT_descriptor_indexing).

use std::collections::HashMap;
use crate::render::atlas::{AtlasRect, AtlasRef};

pub struct VulkanTextureManager {
    pub textures: HashMap<i32, AtlasRect>,
    pub max_texture_dimension: u32,
}

impl VulkanTextureManager {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            max_texture_dimension: 4096,
        }
    }

    pub fn get_rect(&self, atlas_ref: AtlasRef) -> Option<AtlasRect> {
        self.textures.get(&atlas_ref.0).copied()
    }
}
