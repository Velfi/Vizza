use serde::{Deserialize, Serialize};
use super::matrix_operations;

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

    /// Friction/damping factor for particle movement
    pub friction: f32,

    /// Wrap particles around screen edges if true
    pub wrap_edges: bool,

    /// Beta parameter for force calculation (0.0-1.0)
    /// Controls the transition point between repulsion and attraction zones
    /// Lower values = more repulsion, higher values = more attraction
    pub force_beta: f32,

    /// Repulsion strength multiplier for close-range interactions
    /// Multiplies the base repulsion force in the close range zone
    pub repulsion_strength: f32,

    /// Minimum distance for force calculation (prevents singularities)
    pub min_distance: f32,

    /// Maximum distance for force calculation (cutoff radius)
    pub max_distance: f32,

    /// Brownian motion strength (0.0-1.0)
    /// Controls the amount of random thermal motion applied to particles
    /// Higher values create more chaotic, jittery movement
    pub brownian_motion: f32,

    /// Enable 3x3 grid view to visualize wrap-around behavior
    /// Always enabled to show 9 instances of the simulation in a grid with faded outer cells
    pub show_wrap_grid: bool,
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
            max_force: 0.5,
            friction: 0.5,
            wrap_edges: true,
            force_beta: 0.3,
            repulsion_strength: 1.0,
            min_distance: 0.001,
            max_distance: 0.01,
            brownian_motion: 0.5,
            show_wrap_grid: true,
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
                // Random matrix with more varied ranges
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        self.force_matrix[i][j] = rng.random_range(-1.0..1.0);
                    }
                }
            }
            MatrixGenerator::Symmetry => {
                // Symmetric matrix with random strength variations
                let base_strength = rng.random_range(0.3..0.8);
                let variation = rng.random_range(0.1..0.4);
                
                for i in 0..self.species_count as usize {
                    for j in i..self.species_count as usize {
                        let value: f32 = if i == j {
                            rng.random_range(-0.3..-0.05) // Self-repulsion varies
                        } else {
                            let sign = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                            sign * rng.random_range(0.2..base_strength) + rng.random_range(-variation..variation)
                        };
                        self.force_matrix[i][j] = value.clamp(-1.0, 1.0);
                        if i != j {
                            self.force_matrix[j][i] = value; // Copy to lower triangle
                        }
                    }
                }
            }
            MatrixGenerator::Chains => {
                // Chain-like structure with random strengths
                let chain_strength = rng.random_range(0.3..0.7);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                let background_strength = rng.random_range(-0.2..0.1);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if (i as i32 - j as i32).abs() == 1 {
                            self.force_matrix[i][j] = chain_strength + rng.random_range(-0.1..0.1);
                        } else {
                            self.force_matrix[i][j] = background_strength + rng.random_range(-0.05..0.05);
                        }
                    }
                }
            }
            MatrixGenerator::Chains2 => {
                // Alternative chain structure with more complexity
                let near_strength = rng.random_range(0.2..0.6);
                let far_strength = rng.random_range(-0.3..0.1);
                let self_repulsion = rng.random_range(-0.4..-0.1);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if (i as i32 - j as i32).abs() == 1 {
                            self.force_matrix[i][j] = near_strength + rng.random_range(-0.15..0.15);
                        } else if (i as i32 - j as i32).abs() == 2 {
                            self.force_matrix[i][j] = far_strength + rng.random_range(-0.1..0.1);
                        } else {
                            self.force_matrix[i][j] = rng.random_range(-0.1..0.05);
                        }
                    }
                }
            }
            MatrixGenerator::Chains3 => {
                // Complex chain structure with decaying interactions
                let decay_rate: f32 = rng.random_range(0.6..0.9);
                let base_strength = rng.random_range(0.3..0.6);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else {
                            let distance = (i as i32 - j as i32).abs() as f32;
                            let strength: f32 = base_strength * decay_rate.powf(distance - 1.0);
                            let variation = rng.random_range(-0.1..0.1);
                            self.force_matrix[i][j] = (strength + variation).clamp(-0.8, 0.8);
                        }
                    }
                }
            }
            MatrixGenerator::Snakes => {
                // Snake-like pattern with random variations
                let snake_strength = rng.random_range(0.2..0.5);
                let end_connection_strength = rng.random_range(0.1..0.4);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                let background_strength = rng.random_range(-0.1..0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if i == 0 && j == (self.species_count as usize) - 1 {
                            self.force_matrix[i][j] = end_connection_strength + rng.random_range(-0.1..0.1);
                        } else if (i as i32 - j as i32).abs() == 1 {
                            self.force_matrix[i][j] = snake_strength + rng.random_range(-0.1..0.1);
                        } else {
                            self.force_matrix[i][j] = background_strength + rng.random_range(-0.05..0.05);
                        }
                    }
                }
            }
            MatrixGenerator::Zero => {
                // Zero matrix with tiny random noise
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        self.force_matrix[i][j] = rng.random_range(-0.01..0.01);
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
                // Symbiosis: pairs of species have mutual attraction with random variations
                let symbiosis_strength = rng.random_range(0.3..0.7);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                let background_strength = rng.random_range(-0.1..0.1);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if i % 2 == 0 && j == i + 1 && j < self.species_count as usize {
                            // Even-odd pairs have mutual attraction with variation
                            let strength = symbiosis_strength + rng.random_range(-0.1..0.1);
                            self.force_matrix[i][j] = strength;
                            self.force_matrix[j][i] = strength;
                        } else {
                            self.force_matrix[i][j] = background_strength + rng.random_range(-0.05..0.05);
                        }
                    }
                }
            }
            MatrixGenerator::Territorial => {
                // Territorial: strong self-repulsion, varied repulsion from others
                let self_repulsion = rng.random_range(-0.9..-0.5);
                let other_repulsion_base = rng.random_range(-0.5..-0.1);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else {
                            self.force_matrix[i][j] = other_repulsion_base + rng.random_range(-0.2..0.2);
                        }
                    }
                }
            }
            // Physical/Chemical Patterns
            MatrixGenerator::Magnetic => {
                // Magnetic: alternating attraction/repulsion based on "charge" with random strengths
                let attraction_strength = rng.random_range(0.2..0.6);
                let repulsion_strength = rng.random_range(-0.6..-0.2);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if (i % 2 == 0) == (j % 2 == 0) {
                            // Same "charge" (both even or both odd) - attraction with variation
                            self.force_matrix[i][j] = attraction_strength + rng.random_range(-0.1..0.1);
                        } else {
                            // Different "charge" - repulsion with variation
                            self.force_matrix[i][j] = repulsion_strength + rng.random_range(-0.1..0.1);
                        }
                    }
                }
            }
            MatrixGenerator::Crystal => {
                // Crystal: strong attraction to specific neighbors with random lattice variations
                let lattice_strength = rng.random_range(0.4..0.8);
                let self_repulsion = rng.random_range(-0.4..-0.1);
                let background_strength = rng.random_range(-0.2..0.05);
                let lattice_variation = rng.random_range(0.05..0.2);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if (i as i32 - j as i32).abs() == 1
                            || (i == 0 && j == (self.species_count as usize) - 1)
                            || (j == 0 && i == (self.species_count as usize) - 1)
                        {
                            // Neighbors in crystal lattice - strong attraction with variation
                            self.force_matrix[i][j] = lattice_strength + rng.random_range(-lattice_variation..lattice_variation);
                        } else {
                            self.force_matrix[i][j] = background_strength + rng.random_range(-0.1..0.1);
                        }
                    }
                }
            }
            MatrixGenerator::Wave => {
                // Wave: sinusoidal force based on species distance with random amplitude and phase
                let amplitude = rng.random_range(0.3..0.7);
                let frequency = rng.random_range(0.5..2.0);
                let phase = rng.random_range(0.0..std::f32::consts::PI * 2.0);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else {
                            let distance = (i as f32 - j as f32).abs();
                            let wave = (distance * frequency + phase).sin();
                            let variation = rng.random_range(-0.1..0.1);
                            self.force_matrix[i][j] = wave * amplitude + variation;
                        }
                    }
                }
            }
            // Social/Behavioral Patterns
            MatrixGenerator::Hierarchy => {
                // Hierarchy: higher species attract lower ones with random strength variations
                let hierarchy_strength = rng.random_range(0.2..0.5);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                let background_strength = rng.random_range(-0.05..0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if i < j {
                            // Higher species attracts lower ones with variation
                            self.force_matrix[i][j] = hierarchy_strength + rng.random_range(-0.1..0.1);
                            self.force_matrix[j][i] = background_strength + rng.random_range(-0.05..0.05);
                        } else {
                            self.force_matrix[i][j] = background_strength + rng.random_range(-0.05..0.05);
                        }
                    }
                }
            }
            MatrixGenerator::Clique => {
                // Clique: small groups have strong internal attraction, weak repulsion between groups
                let group_size = rng.random_range(2..=4).min(self.species_count as usize / 2);
                let clique_strength = rng.random_range(0.3..0.7);
                let between_clique_strength = rng.random_range(-0.4..-0.1);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if (i / group_size) == (j / group_size) {
                            // Same clique - strong attraction with variation
                            self.force_matrix[i][j] = clique_strength + rng.random_range(-0.1..0.1);
                        } else {
                            // Different clique - repulsion with variation
                            self.force_matrix[i][j] = between_clique_strength + rng.random_range(-0.1..0.1);
                        }
                    }
                }
            }
            MatrixGenerator::AntiClique => {
                // Anti-Clique: strong repulsion within groups, attraction between groups
                let group_size = rng.random_range(2..=4).min(self.species_count as usize / 2);
                let within_clique_strength = rng.random_range(-0.7..-0.3);
                let between_clique_strength = rng.random_range(0.2..0.5);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if (i / group_size) == (j / group_size) {
                            // Same clique - strong repulsion with variation
                            self.force_matrix[i][j] = within_clique_strength + rng.random_range(-0.1..0.1);
                        } else {
                            // Different clique - attraction with variation
                            self.force_matrix[i][j] = between_clique_strength + rng.random_range(-0.1..0.1);
                        }
                    }
                }
            }
            // Mathematical Patterns
            MatrixGenerator::Fibonacci => {
                // Fibonacci: force strength based on Fibonacci sequence with random scaling
                let mut fib = vec![1, 1];
                for k in 2..self.species_count as usize {
                    fib.push(fib[k - 1] + fib[k - 2]);
                }

                let scale_factor = rng.random_range(0.5..1.5);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                let base_offset = rng.random_range(-0.2..0.2);

                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else {
                            let fib_value = fib[i.min(j)];
                            let max_fib = *fib.iter().max().unwrap_or(&1) as f32;
                            let base_force = (fib_value as f32 / max_fib) * scale_factor + base_offset;
                            let variation = rng.random_range(-0.1..0.1);
                            self.force_matrix[i][j] = (base_force + variation).clamp(-0.8, 0.8);
                        }
                    }
                }
            }
            MatrixGenerator::Prime => {
                // Prime: prime-numbered species have special interactions with random strengths
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

                let prime_attraction = rng.random_range(0.4..0.8);
                let mixed_attraction = rng.random_range(0.1..0.4);
                let non_prime_repulsion = rng.random_range(-0.2..-0.05);
                let self_repulsion = rng.random_range(-0.3..-0.05);

                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else if is_prime(i) && is_prime(j) {
                            // Both prime - strong attraction with variation
                            self.force_matrix[i][j] = prime_attraction + rng.random_range(-0.1..0.1);
                        } else if is_prime(i) || is_prime(j) {
                            // One prime - moderate attraction with variation
                            self.force_matrix[i][j] = mixed_attraction + rng.random_range(-0.1..0.1);
                        } else {
                            // Neither prime - weak repulsion with variation
                            self.force_matrix[i][j] = non_prime_repulsion + rng.random_range(-0.05..0.05);
                        }
                    }
                }
            }
            MatrixGenerator::Fractal => {
                // Fractal: recursive force patterns with random parameters
                let scale_factor = rng.random_range(0.3..0.7);
                let frequency = rng.random_range(1.0..3.0);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                let base_offset = rng.random_range(-0.1..0.1);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else {
                            let distance = (i as f32 - j as f32).abs();
                            let normalized_distance = distance / (self.species_count as f32);
                            let scale = (normalized_distance * frequency).log2().max(0.0);
                            let force = (scale * std::f32::consts::PI).sin() * scale_factor + base_offset;
                            let variation = rng.random_range(-0.1..0.1);
                            self.force_matrix[i][j] = (force + variation).clamp(-0.8, 0.8);
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
                // Cooperation: all species have weak mutual attraction with random variations
                let cooperation_strength = rng.random_range(0.1..0.4);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else {
                            self.force_matrix[i][j] = cooperation_strength + rng.random_range(-0.1..0.1);
                        }
                    }
                }
            }
            MatrixGenerator::Competition => {
                // Competition: all species have weak mutual repulsion with random variations
                let competition_strength = rng.random_range(-0.4..-0.1);
                let self_repulsion = rng.random_range(-0.3..-0.05);
                
                for i in 0..self.species_count as usize {
                    for j in 0..self.species_count as usize {
                        if i == j {
                            self.force_matrix[i][j] = self_repulsion;
                        } else {
                            self.force_matrix[i][j] = competition_strength + rng.random_range(-0.1..0.1);
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

    /// Scale all force matrix values by a given factor
    pub fn scale_force_matrix(&mut self, scale_factor: f32) {
        super::matrix_operations::scale_force_matrix(&mut self.force_matrix, scale_factor);
    }

    /// Flip the force matrix horizontally
    pub fn flip_horizontal(&mut self) {
        matrix_operations::flip_horizontal(&mut self.force_matrix);
    }

    /// Flip the force matrix vertically
    pub fn flip_vertical(&mut self) {
        matrix_operations::flip_vertical(&mut self.force_matrix);
    }

    /// Rotate the force matrix clockwise (90 degrees)
    pub fn rotate_clockwise(&mut self) {
        matrix_operations::rotate_clockwise(&mut self.force_matrix);
    }

    /// Rotate the force matrix counterclockwise (90 degrees)
    pub fn rotate_counterclockwise(&mut self) {
        matrix_operations::rotate_counterclockwise(&mut self.force_matrix);
    }

    /// Shift the force matrix left (circular shift of columns)
    pub fn shift_left(&mut self) {
        matrix_operations::shift_left(&mut self.force_matrix);
    }

    /// Shift the force matrix right (circular shift of columns)
    pub fn shift_right(&mut self) {
        matrix_operations::shift_right(&mut self.force_matrix);
    }

    /// Shift the force matrix up (circular shift of rows)
    pub fn shift_up(&mut self) {
        matrix_operations::shift_up(&mut self.force_matrix);
    }

    /// Shift the force matrix down (circular shift of rows)
    pub fn shift_down(&mut self) {
        matrix_operations::shift_down(&mut self.force_matrix);
    }

    /// Set all force matrix values to zero
    pub fn zero_matrix(&mut self) {
        matrix_operations::zero_matrix(&mut self.force_matrix);
    }

    /// Flip the sign of all force matrix values (multiply by -1)
    pub fn flip_sign(&mut self) {
        matrix_operations::flip_sign(&mut self.force_matrix);
    }
}
