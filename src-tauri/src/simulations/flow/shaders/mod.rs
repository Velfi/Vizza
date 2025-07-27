pub const PARTICLE_UPDATE_SHADER: &str = include_str!("particle_update.wgsl");
pub const PARTICLE_RENDER_SHADER: &str = include_str!("particle_render.wgsl");
pub const TRAIL_DECAY_DIFFUSION_SHADER: &str = include_str!("trail_decay_diffusion.wgsl");
pub const TRAIL_RENDER_SHADER: &str = include_str!("trail_render.wgsl");
pub const BACKGROUND_RENDER_SHADER: &str = include_str!("background_render.wgsl");
pub const RENDER_INFINITE_SHADER: &str = crate::simulations::shared::INFINITE_RENDER_SHADER;
