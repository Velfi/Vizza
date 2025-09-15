pub mod noise_seed;
pub mod paint_compute;

pub const REACTION_DIFFUSION_SHADER: &str = include_str!("reaction_diffusion.wgsl");
pub const RENDER_INFINITE_SHADER: &str = crate::simulations::shared::INFINITE_RENDER_SHADER;
pub const BACKGROUND_RENDER_SHADER: &str = include_str!("background_render.wgsl");
