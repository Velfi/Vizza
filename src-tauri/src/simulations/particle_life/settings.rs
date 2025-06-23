use serde::{Deserialize, Serialize};

/// Settings for the Particle Life simulation that can be saved in presets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Number of particle species (2-8)
    pub species_count: u32,

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

    /// Wrap particles around screen edges if true
    pub wrap_edges: bool,

    /// Minimum distance for repulsion calculation
    pub repulsion_min_distance: f32,

    /// Medium distance for repulsion calculation
    pub repulsion_medium_distance: f32,

    /// Extreme repulsion strength for very close particles
    pub repulsion_extreme_strength: f32,

    /// Linear repulsion strength for medium-distance particles
    pub repulsion_linear_strength: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PositionGenerator {
    Random,
    Center,
    UniformCircle,
    CenteredCircle,
    Ring,
    RainbowRing,
    ColorBattle,
    ColorWheel,
    Line,
    Spiral,
    RainbowSpiral,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TypeGenerator {
    Random,
    Randomize10Percent,
    Slices,
    Onion,
    Rotate,
    Flip,
    MoreOfFirst,
    KillStill,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MatrixGenerator {
    Random,
    Symmetry,
    Chains,
    Chains2,
    Chains3,
    Snakes,
    Zero,
    // Biological/Ecological Patterns
    PredatorPrey,
    Symbiosis,
    Territorial,
    // Physical/Chemical Patterns
    Magnetic,
    Crystal,
    Wave,
    // Social/Behavioral Patterns
    Hierarchy,
    Clique,
    AntiClique,
    // Mathematical Patterns
    Fibonacci,
    Prime,
    Fractal,
    // Game Theory Patterns
    RockPaperScissors,
    Cooperation,
    Competition,
}

impl Default for Settings {
    fn default() -> Self {
        // Create default 4-species system with interesting dynamics
        // Using the same default matrix as standalone version
        let mut force_matrix = vec![vec![0.0; 4]; 4];

        // Set up default interactions - these are the exact values from standalone
        force_matrix[0][0] = -0.1;
        force_matrix[0][1] = 0.2;
        force_matrix[0][2] = -0.1;
        force_matrix[0][3] = 0.1;

        force_matrix[1][0] = 0.2;
        force_matrix[1][1] = -0.1;
        force_matrix[1][2] = 0.3;
        force_matrix[1][3] = -0.1;

        force_matrix[2][0] = -0.1;
        force_matrix[2][1] = 0.3;
        force_matrix[2][2] = -0.1;
        force_matrix[2][3] = 0.2;

        force_matrix[3][0] = 0.1;
        force_matrix[3][1] = -0.1;
        force_matrix[3][2] = 0.2;
        force_matrix[3][3] = -0.1;

        Self {
            species_count: 4,
            force_matrix,
            max_force: 1.0,
            min_distance: 0.001,
            max_distance: 0.03,
            friction: 0.85,
            wrap_edges: true,
            repulsion_min_distance: 0.001,
            repulsion_medium_distance: 0.01,
            repulsion_extreme_strength: 1.0,
            repulsion_linear_strength: 0.5,
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
            self.randomize_force_matrix(&MatrixGenerator::Random);
        }
    }

    /// Randomize the interaction force matrix using the specified generator
    pub fn randomize_force_matrix(&mut self, generator: &MatrixGenerator) {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(rand::random::<u64>());

        match generator {
            MatrixGenerator::Random => {
                // Random matrix
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        self.force_matrix[i][j] = rng.random_range(-1.0..1.0);
                    }
                }
            }
            MatrixGenerator::Symmetry => {
                // Symmetric matrix - only iterate over upper triangle
                for i in 0..self.species_count as usize {
                    for j in i..self.species_count as usize {
                        let value = rng.random_range(-1.0..1.0);
                        self.force_matrix[i][j] = value;
                        if i != j {
                            self.force_matrix[j][i] = value; // Copy to lower triangle
                        }
                    }
                }
            }
            MatrixGenerator::Chains => {
                // Chain-like structure
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if (i as i32 - j as i32).abs() == 1 {
                            self.force_matrix[i][j] = 0.5; // Strong attraction to neighbors
                        } else {
                            self.force_matrix[i][j] = 0.0; // No interaction with others
                        }
                    }
                }
            }
            MatrixGenerator::Chains2 => {
                // Alternative chain structure
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.2;
                        } else if (i as i32 - j as i32).abs() == 1 {
                            self.force_matrix[i][j] = 0.3;
                        } else if (i as i32 - j as i32).abs() == 2 {
                            self.force_matrix[i][j] = -0.1;
                        } else {
                            self.force_matrix[i][j] = 0.0;
                        }
                    }
                }
            }
            MatrixGenerator::Chains3 => {
                // Complex chain structure
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1;
                        } else if (i as i32 - j as i32).abs() == 1 {
                            self.force_matrix[i][j] = 0.4;
                        } else if (i as i32 - j as i32).abs() == 2 {
                            self.force_matrix[i][j] = 0.1;
                        } else if (i as i32 - j as i32).abs() == 3 {
                            self.force_matrix[i][j] = -0.05;
                        } else {
                            self.force_matrix[i][j] = 0.0;
                        }
                    }
                }
            }
            MatrixGenerator::Snakes => {
                // Snake-like pattern
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1;
                        } else if i == 0 && j == (self.species_count as usize) - 1 {
                            self.force_matrix[i][j] = 0.3; // Connect ends
                        } else if (i as i32 - j as i32).abs() == 1 {
                            self.force_matrix[i][j] = 0.3;
                        } else {
                            self.force_matrix[i][j] = 0.0;
                        }
                    }
                }
            }
            MatrixGenerator::Zero => {
                // Zero matrix - no interactions
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        self.force_matrix[i][j] = 0.0;
                    }
                }
            }
            // Biological/Ecological Patterns
            MatrixGenerator::PredatorPrey => {
                // Predator-Prey: species i attracts species i+1, species i+1 repels species i
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if j == (i + 1) % (self.species_count as usize) {
                            self.force_matrix[i][j] = 0.4; // Predator attracts prey
                            self.force_matrix[j][i] = -0.3; // Prey repels predator
                        } else {
                            self.force_matrix[i][j] = 0.0; // No interaction
                        }
                    }
                }
            }
            MatrixGenerator::Symbiosis => {
                // Symbiosis: pairs of species have mutual attraction
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if i % 2 == 0 && j == i + 1 && j < self.species_count as usize {
                            // Even-odd pairs have mutual attraction
                            self.force_matrix[i][j] = 0.5;
                            self.force_matrix[j][i] = 0.5;
                        } else {
                            self.force_matrix[i][j] = 0.0; // Neutral to others
                        }
                    }
                }
            }
            MatrixGenerator::Territorial => {
                // Territorial: strong self-repulsion, moderate repulsion from others
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.8; // Strong self-repulsion
                        } else {
                            self.force_matrix[i][j] = -0.3; // Moderate repulsion from others
                        }
                    }
                }
            }
            // Physical/Chemical Patterns
            MatrixGenerator::Magnetic => {
                // Magnetic: alternating attraction/repulsion based on "charge"
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if (i % 2 == 0) == (j % 2 == 0) {
                            // Same "charge" (both even or both odd) - attraction
                            self.force_matrix[i][j] = 0.4;
                        } else {
                            // Different "charge" - repulsion
                            self.force_matrix[i][j] = -0.4;
                        }
                    }
                }
            }
            MatrixGenerator::Crystal => {
                // Crystal: strong attraction to specific neighbors (like crystal lattice)
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.2; // Self-repulsion
                        } else if (i as i32 - j as i32).abs() == 1
                            || (i == 0 && j == (self.species_count as usize) - 1)
                            || (j == 0 && i == (self.species_count as usize) - 1)
                        {
                            // Neighbors in crystal lattice - strong attraction
                            self.force_matrix[i][j] = 0.6;
                        } else {
                            self.force_matrix[i][j] = -0.1; // Weak repulsion from others
                        }
                    }
                }
            }
            MatrixGenerator::Wave => {
                // Wave: sinusoidal force based on species distance
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else {
                            let distance = (i as f32 - j as f32).abs();
                            let wave = (distance * std::f32::consts::PI / 2.0).sin();
                            self.force_matrix[i][j] = wave * 0.5; // Scale to reasonable range
                        }
                    }
                }
            }
            // Social/Behavioral Patterns
            MatrixGenerator::Hierarchy => {
                // Hierarchy: higher species attract lower ones, lower species neutral to higher
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if i < j {
                            // Higher species attracts lower ones
                            self.force_matrix[i][j] = 0.3;
                            self.force_matrix[j][i] = 0.0; // Lower species neutral to higher
                        } else {
                            self.force_matrix[i][j] = 0.0; // Already set above
                        }
                    }
                }
            }
            MatrixGenerator::Clique => {
                // Clique: small groups have strong internal attraction, weak repulsion between groups
                let group_size = 2;
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if (i / group_size) == (j / group_size) {
                            // Same clique - strong attraction
                            self.force_matrix[i][j] = 0.5;
                        } else {
                            // Different clique - weak repulsion
                            self.force_matrix[i][j] = -0.2;
                        }
                    }
                }
            }
            MatrixGenerator::AntiClique => {
                // Anti-Clique: strong repulsion within groups, attraction between groups
                let group_size = 2;
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if (i / group_size) == (j / group_size) {
                            // Same clique - strong repulsion
                            self.force_matrix[i][j] = -0.5;
                        } else {
                            // Different clique - attraction
                            self.force_matrix[i][j] = 0.3;
                        }
                    }
                }
            }
            // Mathematical Patterns
            MatrixGenerator::Fibonacci => {
                // Fibonacci: force strength based on Fibonacci sequence
                let mut fib = vec![1, 1];
                for k in 2..self.species_count as usize {
                    fib.push(fib[k - 1] + fib[k - 2]);
                }

                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else {
                            let fib_value = fib[i.min(j)];
                            let max_fib = *fib.iter().max().unwrap_or(&1) as f32;
                            self.force_matrix[i][j] = (fib_value as f32 / max_fib) * 0.8 - 0.4;
                            // Scale to [-0.4, 0.4]
                        }
                    }
                }
            }
            MatrixGenerator::Prime => {
                // Prime: prime-numbered species have special interactions
                let is_prime = |n: usize| -> bool {
                    if n < 2 {
                        return false;
                    }
                    for i in 2..=((n as f64).sqrt() as usize) {
                        if n % i == 0 {
                            return false;
                        }
                    }
                    true
                };

                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if is_prime(i) && is_prime(j) {
                            // Both prime - strong attraction
                            self.force_matrix[i][j] = 0.6;
                        } else if is_prime(i) || is_prime(j) {
                            // One prime - moderate attraction
                            self.force_matrix[i][j] = 0.2;
                        } else {
                            // Neither prime - weak repulsion
                            self.force_matrix[i][j] = -0.1;
                        }
                    }
                }
            }
            MatrixGenerator::Fractal => {
                // Fractal: recursive force patterns (simplified version)
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else {
                            let distance = (i as f32 - j as f32).abs();
                            let scale = (distance / (self.species_count as f32)).log2().max(0.0);
                            let force = (scale * std::f32::consts::PI).sin() * 0.5;
                            self.force_matrix[i][j] = force;
                        }
                    }
                }
            }
            // Game Theory Patterns
            MatrixGenerator::RockPaperScissors => {
                // Rock-Paper-Scissors: cyclic dominance
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else if j == (i + 1) % (self.species_count as usize) {
                            // i beats j (attracts)
                            self.force_matrix[i][j] = 0.4;
                            self.force_matrix[j][i] = -0.2; // j loses to i (weak repulsion)
                        } else if i == (j + 1) % (self.species_count as usize) {
                            // i loses to j (already set above)
                            self.force_matrix[i][j] = -0.2;
                        } else {
                            // No direct relationship
                            self.force_matrix[i][j] = 0.0;
                        }
                    }
                }
            }
            MatrixGenerator::Cooperation => {
                // Cooperation: all species have weak mutual attraction
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else {
                            self.force_matrix[i][j] = 0.2; // Weak mutual attraction
                        }
                    }
                }
            }
            MatrixGenerator::Competition => {
                // Competition: all species have weak mutual repulsion
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = -0.1; // Self-repulsion
                        } else {
                            self.force_matrix[i][j] = -0.2; // Weak mutual repulsion
                        }
                    }
                }
            }
        }
    }

    /// Randomize only the force matrix, preserving physics settings
    pub fn randomize(&mut self) {
        // Only randomize the force matrix - preserve all physics parameters
        self.randomize_force_matrix(&MatrixGenerator::Random);

        // Note: Physics settings (max_force, distances, friction, wrap_edges)
        // are intentionally NOT randomized.
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
