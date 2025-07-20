# Vizzy

A desktop application featuring interactive simulations built with Tauri, Svelte, and WebGPU.

## What it is

Vizzy is a collection of real-time simulations that run on your desktop. Each simulation uses GPU-accelerated rendering for smooth performance and includes interactive controls for experimentation.

## Simulations

### Slime Mold

Agent-based simulation where simple creatures follow trails and create emergent patterns. Watch as individual agents work together to form complex networks and structures.

### Gray-Scott

Reaction-diffusion simulation that models chemical reactions. Creates organic-looking patterns that evolve over time, similar to patterns found in nature.

### Particle Life

Multi-species particle simulation where different types of particles interact based on attraction and repulsion rules. Observe how simple rules create complex emergent behaviors.

### Ecosystem

Artificial life simulation featuring agents with neural networks, chemical sensing, and evolutionary behavior. Watch as digital creatures evolve, learn, and interact through chemotaxis in a dynamic environment.

## Development

Built with:

- **Frontend**: Svelte 5 + TypeScript
- **Backend**: Rust with Tauri
- **Graphics**: WebGPU for GPU-accelerated rendering
- **Build**: Vite

## Getting Started

### Prerequisites

- Node.js 18+
- Rust toolchain

### Development

```bash
cargo tauri dev
```

### Build

```bash
cargo tauri build
```

## Performance Optimizations

Vizzy includes several performance optimizations for smooth simulation performance:

### SIMD Acceleration

- **Rapier Physics**: Enabled with `simd-stable`, `simd-nightly`, and `wasm-simd` features
- **Compiler Optimizations**: Native CPU targeting with AVX2, FMA, and BMI2 instructions
- **Link-time Optimization**: Fat LTO for maximum performance in release builds

### Algorithm Optimizations

- **Spatial Hashing**: O(n) particle density calculation instead of O(n²)
- **Batch Processing**: Optimized particle synchronization with cache-friendly memory access
- **Reduced Physics Iterations**: Balanced performance and stability for real-time simulation

### Build Configuration

The project uses optimized build settings in `.cargo/config.toml`:

- Native CPU targeting for maximum SIMD utilization
- Link-time optimization for cross-module optimization
- Reduced debug information in release builds
- Optimized code generation units for better parallel compilation
