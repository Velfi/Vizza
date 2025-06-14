/// Strongly-typed coordinate system to prevent mixing different coordinate spaces
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenCoords {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldCoords {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NdcCoords {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureCoords {
    pub x: f32,
    pub y: f32,
}

impl ScreenCoords {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    pub fn from_array(coords: [f32; 2]) -> Self {
        Self { x: coords[0], y: coords[1] }
    }
    
    pub fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl WorldCoords {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    pub fn from_array(coords: [f32; 2]) -> Self {
        Self { x: coords[0], y: coords[1] }
    }
    
    pub fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl NdcCoords {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    pub fn from_array(coords: [f32; 2]) -> Self {
        Self { x: coords[0], y: coords[1] }
    }
    
    pub fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl TextureCoords {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    pub fn from_array(coords: [f32; 2]) -> Self {
        Self { x: coords[0], y: coords[1] }
    }
    
    pub fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
    
    /// Clamp texture coordinates to valid range [0.0, 1.0]
    pub fn clamp(self) -> Self {
        Self {
            x: self.x.clamp(0.0, 1.0),
            y: self.y.clamp(0.0, 1.0),
        }
    }
    
    /// Check if texture coordinates are within valid bounds
    pub fn is_valid(self) -> bool {
        self.x >= 0.0 && self.x <= 1.0 && self.y >= 0.0 && self.y <= 1.0
    }
}

// Conversion traits to make coordinate transformations explicit
pub trait CoordinateTransform {
    fn screen_to_world(&self, screen: ScreenCoords) -> WorldCoords;
    fn world_to_screen(&self, world: WorldCoords) -> ScreenCoords;
    fn screen_to_ndc(&self, screen: ScreenCoords) -> NdcCoords;
    fn ndc_to_world(&self, ndc: NdcCoords) -> WorldCoords;
    fn world_to_ndc(&self, world: WorldCoords) -> NdcCoords;
}

/// Utility functions for coordinate conversion
impl WorldCoords {
    /// Convert world coordinates to texture coordinates (for simulation space mapping)
    /// Assumes world space ranges from -1 to 1, maps to texture space 0 to 1
    pub fn to_texture_coords(self) -> TextureCoords {
        TextureCoords {
            x: (self.x + 1.0) * 0.5,
            y: (-self.y + 1.0) * 0.5, // Flip Y axis: world Y increases upward, texture Y increases downward
        }
    }
}

impl TextureCoords {
    /// Convert texture coordinates to world coordinates
    /// Maps texture space 0 to 1 to world space -1 to 1
    pub fn to_world_coords(self) -> WorldCoords {
        WorldCoords {
            x: self.x * 2.0 - 1.0,
            y: -(self.y * 2.0 - 1.0), // Flip Y axis: texture Y increases downward, world Y increases upward
        }
    }
}