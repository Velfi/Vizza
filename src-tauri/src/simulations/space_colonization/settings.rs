use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// The maximum distance at which attractors can influence nodes
    /// 
    /// Large values create smoother curves but cost performance.
    /// Defaults to 50.0.
    pub attraction_distance: f32,
    
    /// The distance at which attractors are removed when nodes get close
    /// 
    /// Smaller values create denser branching. Defaults to 15.0.
    pub kill_distance: f32,
    
    /// The distance between nodes as the network grows
    /// 
    /// Larger values improve performance but create choppier curves.
    /// Defaults to 5.0.
    pub segment_length: f32,
    
    /// Maximum number of attractors to simulate
    /// 
    /// Defaults to 1000.
    pub max_attractors: u32,
    
    /// Maximum number of nodes in the growing network
    /// 
    /// Defaults to 10000.
    pub max_nodes: u32,
    
    /// Growth mode: true for open venation (trees), false for closed (leaves)
    /// 
    /// Open venation creates tree-like structures without loops.
    /// Closed venation creates more realistic leaf-like patterns with potential loops.
    /// Defaults to true (open).
    pub open_venation: bool,
    
    /// Enable or disable vein thickening based on branch depth
    /// 
    /// Makes branches thicker as they accumulate more descendant nodes.
    /// Defaults to true.
    pub enable_vein_thickening: bool,
    
    /// Minimum thickness for branch rendering
    /// 
    /// Base thickness for all branches. Defaults to 1.0.
    pub min_thickness: f32,
    
    /// Maximum thickness for branch rendering
    /// 
    /// Cap on how thick branches can become. Defaults to 8.0.
    pub max_thickness: f32,
    
    /// Enable opacity blending based on thickness
    /// 
    /// Creates depth illusion by varying opacity with thickness.
    /// Defaults to true.
    pub enable_opacity_blending: bool,
    
    /// Minimum opacity for branches
    /// 
    /// Base opacity for thin branches. Defaults to 0.3.
    pub min_opacity: f32,
    
    /// Maximum opacity for branches  
    /// 
    /// Opacity for thick branches. Defaults to 1.0.
    pub max_opacity: f32,
    
    /// Random seed for reproducible generation
    /// 
    /// Defaults to 0.
    pub random_seed: u32,
    
    /// Attractor placement pattern
    /// 
    /// Defaults to Random.
    pub attractor_pattern: AttractorPattern,
    
    /// Growth speed multiplier
    /// 
    /// Controls how many new nodes are added per frame.
    /// Defaults to 1.0.
    pub growth_speed: f32,
    
    /// Bounding shape for constraining growth
    /// 
    /// Defaults to None (unlimited).
    pub bounding_shape: BoundingShape,
    
    /// Enable interactive mouse attractors
    /// 
    /// Allows placing attractors with mouse clicks.
    /// Defaults to true.
    pub interactive_attractors: bool,
    
    /// Size of mouse-placed attractor clusters
    /// 
    /// Radius of attractor clouds placed by mouse.
    /// Defaults to 30.0.
    pub mouse_attractor_size: f32,
    
    /// Density of mouse-placed attractor clusters
    /// 
    /// Number of attractors per cluster. Defaults to 20.
    pub mouse_attractor_density: u32,
    
    /// Controls the tightness of curves (0.0 = straight lines, 1.0 = tight curves)
    /// 
    /// Higher values create more dramatic curves. Defaults to 0.3.
    pub curve_tension: f32,
    
    /// Number of segments to subdivide curves into for rendering
    /// 
    /// Higher values create smoother curves but cost performance.
    /// Defaults to 8.
    pub curve_segments: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AttractorPattern {
    /// Random distribution across the canvas
    Random,
    /// Clustered around specific points
    Clustered,
    /// Grid-based placement
    Grid,
    /// Circular arrangement
    Circular,
    /// Image-based placement (stippling)
    ImageBased,
    /// Along boundaries only (for marginal growth)
    Boundary,
    /// Leaf-specific pattern for beautiful venation
    Leaf,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BoundingShape {
    /// No constraints
    None,
    /// Rectangular boundary
    Rectangle,
    /// Circular boundary
    Circle,
    /// Elliptical boundary
    Ellipse,
    /// Custom polygon (for future implementation)
    Polygon,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            attraction_distance: 100.0,
            kill_distance: 8.0,
            segment_length: 3.0,
            max_attractors: 4000,
            max_nodes: 25000,
            open_venation: true,
            enable_vein_thickening: true,
            min_thickness: 1.5,
            max_thickness: 10.0,
            enable_opacity_blending: true,
            min_opacity: 0.4,
            max_opacity: 1.0,
            random_seed: 0,
            attractor_pattern: AttractorPattern::Leaf,
            growth_speed: 2.5,
            bounding_shape: BoundingShape::None,
            interactive_attractors: true,
            mouse_attractor_size: 30.0,
            mouse_attractor_density: 20,
            curve_tension: 0.3,
            curve_segments: 8,
        }
    }
}

impl Settings {
    /// Randomize all settings within reasonable bounds
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        self.attraction_distance = rng.random::<f32>() * 100.0 + 20.0; // 20-120
        self.kill_distance = rng.random::<f32>() * 30.0 + 5.0; // 5-35
        self.segment_length = rng.random::<f32>() * 10.0 + 2.0; // 2-12
        self.max_attractors = (rng.random::<f32>() * 1500.0 + 500.0) as u32; // 500-2000
        self.max_nodes = (rng.random::<f32>() * 15000.0 + 5000.0) as u32; // 5000-20000
        self.open_venation = rng.random::<bool>();
        self.enable_vein_thickening = rng.random::<bool>();
        self.min_thickness = rng.random::<f32>() * 2.0 + 0.5; // 0.5-2.5
        self.max_thickness = self.min_thickness + rng.random::<f32>() * 10.0; // min_thickness + 0-10
        self.enable_opacity_blending = rng.random::<bool>();
        self.min_opacity = rng.random::<f32>() * 0.5 + 0.1; // 0.1-0.6
        self.max_opacity = self.min_opacity + rng.random::<f32>() * (1.0 - self.min_opacity); // min_opacity to 1.0
        self.random_seed = rng.random();
        self.attractor_pattern = match rng.random::<u8>() % 5 {
            0 => AttractorPattern::Random,
            1 => AttractorPattern::Clustered,
            2 => AttractorPattern::Grid,
            3 => AttractorPattern::Circular,
            _ => AttractorPattern::Leaf,
        };
        self.growth_speed = rng.random::<f32>() * 3.0 + 0.5; // 0.5-3.5
        self.bounding_shape = match rng.random::<u8>() % 4 {
            0 => BoundingShape::None,
            1 => BoundingShape::Rectangle,
            2 => BoundingShape::Circle,
            _ => BoundingShape::Ellipse,
        };
        self.mouse_attractor_size = rng.random::<f32>() * 50.0 + 10.0; // 10-60
        self.mouse_attractor_density = (rng.random::<f32>() * 40.0 + 10.0) as u32; // 10-50
        self.curve_tension = rng.random::<f32>() * 1.0; // 0.0-1.0
        self.curve_segments = (rng.random::<f32>() * 16.0 + 4.0) as u32; // 4-20
    }
} 