use serde::{Deserialize, Serialize};

/// Settings for the Particle Life simulation that can be saved in presets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Number of particle species (2-8)
    pub species_count: u32,
    
    /// Number of particles in the simulation (1,000-100,000)
    pub particle_count: u32,
    
    /// Force matrix - attraction/repulsion between species
    /// Each entry is in range [-1.0, 1.0] where:
    /// -1.0 = maximum repulsion, 0.0 = neutral, 1.0 = maximum attraction
    pub force_matrix: Vec<Vec<f32>>,
    
    /// Maximum force magnitude applied to particles
    pub max_force: f32,
    
    /// Minimum distance for force calculation (prevents singularities)
    pub min_distance: f32,
    
    /// Maximum distance for force calculation (cutoff radius)
    pub max_distance: f32,
    
    /// Friction/damping factor for particle movement
    pub friction: f32,
    
    /// Time step for simulation integration
    pub time_step: f32,
    
    /// Wrap particles around screen edges if true
    pub wrap_edges: bool,
    
    /// Random seed for particle reset operations
    pub random_seed: u32,
    
    /// Short-range repulsion settings
    /// Minimum distance before extreme repulsion kicks in
    pub repulsion_min_distance: f32,
    
    /// Medium distance where linear repulsion operates
    pub repulsion_medium_distance: f32,
    
    /// Strength of extreme repulsion at very close distances
    pub repulsion_extreme_strength: f32,
    
    /// Strength of linear repulsion at medium distances
    pub repulsion_linear_strength: f32,
}

impl Default for Settings {
    fn default() -> Self {
        // Create default 4-species system with interesting dynamics
        let mut force_matrix = vec![vec![0.0; 4]; 4];
        
        // Set up some interesting default interactions
        force_matrix[0][0] = -0.2;  // Red repels red
        force_matrix[0][1] = 0.3;   // Red attracts green
        force_matrix[0][2] = -0.1;  // Red slightly repels blue
        force_matrix[0][3] = 0.1;   // Red slightly attracts yellow
        
        force_matrix[1][0] = -0.3;  // Green repels red
        force_matrix[1][1] = -0.1;  // Green slightly repels green
        force_matrix[1][2] = 0.4;   // Green attracts blue
        force_matrix[1][3] = -0.2;  // Green repels yellow
        
        force_matrix[2][0] = 0.2;   // Blue attracts red
        force_matrix[2][1] = -0.4;  // Blue repels green
        force_matrix[2][2] = 0.1;   // Blue slightly attracts blue
        force_matrix[2][3] = 0.3;   // Blue attracts yellow
        
        force_matrix[3][0] = -0.1;  // Yellow slightly repels red
        force_matrix[3][1] = 0.2;   // Yellow attracts green
        force_matrix[3][2] = -0.3;  // Yellow repels blue
        force_matrix[3][3] = -0.2;  // Yellow repels yellow
        
        Self {
            species_count: 4,
            particle_count: 20000,
            force_matrix,
            max_force: 100.0,
            min_distance: 5.0,
            max_distance: 100.0,
            friction: 0.98,
            time_step: 0.016, // ~60 FPS
            wrap_edges: true,
            random_seed: 0,
            repulsion_min_distance: 0.1,
            repulsion_medium_distance: 0.5,
            repulsion_extreme_strength: 1000.0,
            repulsion_linear_strength: 200.0,
        }
    }
}

impl Settings {
    /// Create a new settings instance with the specified number of species
    pub fn with_species_count(species_count: u32) -> Self {
        let species_count = species_count.clamp(2, 8);
        let mut settings = Self::default();
        settings.set_species_count(species_count);
        settings
    }
    
    /// Update the number of species and resize the force matrix
    pub fn set_species_count(&mut self, count: u32) {
        let count = count.clamp(2, 8) as usize;
        self.species_count = count as u32;
        
        // Resize force matrix
        self.force_matrix.resize(count, vec![0.0; count]);
        for row in &mut self.force_matrix {
            row.resize(count, 0.0);
        }
        
        // Fill with some default interesting values if expanding
        if count > 2 {
            self.randomize_force_matrix();
        }
    }
    
    /// Randomize the force matrix with biologically-inspired patterns
    pub fn randomize_force_matrix(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();
        
        for i in 0..self.species_count as usize {
            for j in 0..self.species_count as usize {
                if i == j {
                    // Same species - usually slight repulsion to prevent clustering
                    self.force_matrix[i][j] = rng.random_range(-0.3..0.1);
                } else {
                    // Different species - random attraction/repulsion
                    self.force_matrix[i][j] = rng.random_range(-0.5..0.5);
                }
            }
        }
    }
    
    /// Randomize all settings within reasonable bounds
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();
        
        // Keep species count and particle count stable but randomize force matrix
        self.randomize_force_matrix();
        
        self.max_force = rng.random_range(50.0..200.0);
        self.min_distance = rng.random_range(2.0..10.0);
        self.max_distance = rng.random_range(50.0..150.0);
        self.friction = rng.random_range(0.9..0.99);
        self.time_step = rng.random_range(0.01..0.03);
        self.wrap_edges = rng.random_bool(0.7); // Usually wrap edges
        self.random_seed = rng.random();
        
        // Note: particle_count and species_count are intentionally NOT randomized
        // to preserve user's preferred configuration
    }
    
    /// Get the force between two species
    pub fn get_force(&self, species_a: usize, species_b: usize) -> f32 {
        if species_a < self.force_matrix.len() && species_b < self.force_matrix[species_a].len() {
            self.force_matrix[species_a][species_b]
        } else {
            0.0
        }
    }
    
    /// Set the force between two species
    pub fn set_force(&mut self, species_a: usize, species_b: usize, force: f32) {
        if species_a < self.force_matrix.len() && species_b < self.force_matrix[species_a].len() {
            self.force_matrix[species_a][species_b] = force.clamp(-1.0, 1.0);
        }
    }
}