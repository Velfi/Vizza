//! # Pellets Shaders Module
//!
//! Contains the GPU programs that drive the Pellets simulation, implementing
//! both the physics calculations and visual rendering. These shaders transform
//! simple mathematical rules into complex emergent behaviors.
//!
//! ## Shader Philosophy
//!
//! The shaders implement the core simulation logic through parallel computation.
//! By processing all particles simultaneously on the GPU, the simulation can
//! handle thousands of interacting particles while maintaining real-time
//! performance and responsive user interaction.
//!
//! ## Computational Approach
//!
//! The simulation uses a multi-stage approach: spatial partitioning for efficient
//! neighbor lookups, compute shaders handle the physics calculations that determine
//! particle behavior, while render shaders create the visual representation.

// Compute shaders
pub const PHYSICS_COMPUTE_SHADER: &str = include_str!("physics_compute.wgsl");
pub const DENSITY_COMPUTE_SHADER: &str = include_str!("density_compute.wgsl");
pub const GRID_CLEAR_SHADER: &str = include_str!("grid_clear.wgsl");
pub const GRID_POPULATE_SHADER: &str = include_str!("grid_populate.wgsl");

// Offscreen rendering shaders
pub const BACKGROUND_RENDER_SHADER: &str = include_str!("background_render.wgsl");
pub const PARTICLE_RENDER_SHADER: &str = include_str!("particle_render.wgsl");
pub const PARTICLE_FRAGMENT_RENDER_SHADER: &str = include_str!("particle_fragment_render.wgsl");
pub const POST_EFFECT_VERTEX_SHADER: &str = include_str!("post_effect_vertex.wgsl");
pub const POST_EFFECT_FRAGMENT_SHADER: &str = include_str!("post_effect_fragment.wgsl");
pub const RENDER_INFINITE_SHADER: &str = crate::simulations::shared::INFINITE_RENDER_SHADER;

// Trail shaders
pub const TRAIL_FADE_VERTEX_SHADER: &str = include_str!("trail_fade_vertex.wgsl");
pub const TRAIL_FADE_FRAGMENT_SHADER: &str = include_str!("trail_fade_fragment.wgsl");
pub const TRAIL_BLIT_SHADER: &str = include_str!("trail_blit.wgsl");
