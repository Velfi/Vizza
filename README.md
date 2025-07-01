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
