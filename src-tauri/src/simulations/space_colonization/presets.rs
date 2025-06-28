use super::settings::Settings;

/// Predefined presets for space colonization simulation
impl Settings {
    /// Dense tree-like growth with thick trunks
    pub fn tree_preset() -> Self {
        Self {
            attraction_distance: 120.0, // Very large attraction for sustained growth
            kill_distance: 6.0, // Very small kill distance for long growth
            segment_length: 2.5, // Shorter segments for smoother growth
            open_venation: true, // Open venation for tree-like patterns
            enable_vein_thickening: true,
            min_thickness: 2.0, // Thicker branches
            max_thickness: 15.0, // Very thick main trunk
            enable_opacity_blending: true,
            min_opacity: 0.5, // Higher visibility
            max_opacity: 1.0,
            growth_speed: 3.0, // Fast growth for sustained development
            max_attractors: 5000, // Many attractors for sustained growth
            max_nodes: 30000, // Many nodes for complex growth
            attractor_pattern: super::settings::AttractorPattern::Leaf, // Use leaf pattern for trees
            ..Default::default()
        }
    }
    
    /// Fine leaf venation with loops
    pub fn leaf_preset() -> Self {
        Self {
            attraction_distance: 25.0,
            kill_distance: 6.0,
            segment_length: 1.5,
            open_venation: false,
            enable_vein_thickening: true,
            min_thickness: 0.5,
            max_thickness: 6.0,
            enable_opacity_blending: true,
            min_opacity: 0.2,
            max_opacity: 1.0,
            growth_speed: 3.0,
            max_attractors: 4000,
            max_nodes: 25000,
            ..Default::default()
        }
    }
    
    /// Coral or sea fan-like structures
    pub fn coral_preset() -> Self {
        Self {
            attraction_distance: 60.0,
            kill_distance: 18.0,
            segment_length: 6.0,
            open_venation: true,
            enable_vein_thickening: false,
            min_thickness: 2.0,
            max_thickness: 2.0,
            growth_speed: 1.2,
            ..Default::default()
        }
    }
    
    /// Lightning-like branching
    pub fn lightning_preset() -> Self {
        Self {
            attraction_distance: 120.0,
            kill_distance: 40.0,
            segment_length: 8.0,
            open_venation: true,
            enable_vein_thickening: false,
            min_thickness: 1.0,
            max_thickness: 3.0,
            enable_opacity_blending: false,
            growth_speed: 2.0,
            ..Default::default()
        }
    }
    
    /// Beautiful leaf patterns with fine venation
    pub fn beautiful_leaf_preset() -> Self {
        Self {
            attraction_distance: 90.0, // Large attraction for sustained growth
            kill_distance: 5.0, // Very small kill distance for long growth
            segment_length: 2.0, // Short segments for smooth growth
            open_venation: true, // Open venation for tree-like patterns
            enable_vein_thickening: true,
            min_thickness: 1.0, // Moderate thickness
            max_thickness: 8.0, // Good max thickness
            enable_opacity_blending: true,
            min_opacity: 0.3, // Good depth
            max_opacity: 1.0,
            growth_speed: 3.0, // Fast growth for sustained development
            max_attractors: 5000, // Many attractors for sustained growth
            max_nodes: 35000, // Many nodes for complex growth
            random_seed: 42, // Good seed for beautiful patterns
            attractor_pattern: super::settings::AttractorPattern::Leaf, // Use leaf-specific pattern
            ..Default::default()
        }
    }
} 