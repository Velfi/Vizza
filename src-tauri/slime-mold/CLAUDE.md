# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Don't be so obsequious. Be curt, matter of fact. You don't embellish or narrativize. Keep it simple, stupid.

## Project Overview

This is a GPU-accelerated slime mold (Physarum) simulation written in Rust using WGPU compute shaders. The simulation models emergent behavior of millions of agents following simple rules, creating complex organic patterns.

## Common Development Commands

### Building and Running
- **Native build**: `cargo run --release`
- **Development build**: `cargo run` (debug mode, slower)
- **WebAssembly build**: `./build_wasm.sh` (requires wasm-pack)
- **Serve WASM locally**: After building, use `python3 -m http.server` to serve the files

### Key Controls
- `R`: Randomize all simulation settings
- `/`: Toggle sidebar UI visibility

## Architecture Overview

### Core Application Structure
- **App** (`src/app.rs`): Main application state and event loop using winit/ApplicationHandler
- **Simulation** (`src/simulation.rs`): Defines uniform buffer structure for GPU communication
- **Settings** (`src/settings.rs`): Configuration and parameter management
- **Presets** (`src/presets.rs`): Predefined simulation configurations

### Rendering Pipeline
The rendering system uses a multi-stage GPU compute pipeline:

1. **Gradient Generation** (`shaders/gradient.wgsl`): Creates directional force fields
2. **Agent Movement** (`shaders/compute.wgsl`): Updates agent positions and sensor behavior
3. **Trail Decay** (`shaders/compute.wgsl`): Reduces pheromone intensity over time
4. **Trail Diffusion** (`shaders/compute.wgsl`): Spreads pheromones to neighboring cells
5. **Display Generation** (`shaders/display.wgsl`): Applies color lookup tables (LUTs)
6. **Screen Rendering** (`shaders/quad.wgsl`): Final display to screen

### Key Components
- **Render Module** (`src/render/`): Manages GPU pipelines, shaders, and bind groups
- **LUT Manager** (`src/lut_manager.rs`): Handles color scheme loading from `.lut` files
- **EguiRenderer** (`src/egui_tools.rs`): UI rendering with egui

### GPU Buffer Management
- **Agent Buffer**: Stores position, heading, and speed for each agent (vec4<f32> per agent)
- **Trail Map Buffer**: 2D pheromone concentration grid
- **Gradient Buffer**: Optional directional force field
- **Uniform Buffer**: Simulation parameters (SimSizeUniform struct)

### Settings System
The simulation uses a hierarchical settings system:
- Built-in presets (embedded in code)
- User presets (saved to filesystem)
- Real-time parameter adjustment via UI
- All settings sync to GPU via uniform buffer

## Development Notes

### Performance Considerations
- Use `--release` builds for performance testing
- Agent count directly impacts GPU workload
- Texture size limited by GPU capabilities
- Frame limiting available for power management

### WebAssembly Deployment
- Targets web platform via wasm-bindgen
- Uses WebGPU for browser acceleration
- Error handling displays in browser console
- Build script handles wasm-pack compilation

### Shader Development
- All shaders in `src/shaders/` directory
- Uses WGSL (WebGPU Shading Language)
- Compute shaders handle simulation logic
- Render shaders handle visualization

### LUT System
- Color schemes stored in `src/LUTs/` directory
- Supports both custom (KTZ_*) and matplotlib colormaps
- Runtime loading and caching
- Reversible color mappings