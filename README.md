# Vizzy

Interactive GPU-accelerated simulations for desktop.

## Simulations

### Slime Mold
Agent-based simulation where creatures follow trails to create emergent networks.

![Slime Mold Example](example-slime-mold.png)

### Gray-Scott
Reaction-diffusion simulation modeling chemical reactions that create organic patterns.

### Particle Life
Multi-species particle simulation with attraction/repulsion interactions.

### Flow
Flow field simulation with particle movement patterns.

![Flow Mode Example](example-flow-mode.png)

### Pellets
Particle simulation with gravity and density-based interactions.

## Getting Started

### Prerequisites
- Node.js 18+
- Rust toolchain
- Tauri CLI

### Development
```bash
cargo tauri dev
```

### Build
```bash
cargo tauri build
```

## Tech Stack
- Frontend: Svelte 5 + TypeScript
- Backend: Rust with Tauri
- Graphics: WebGPU
- Build: Vite
