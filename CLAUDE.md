# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Don't be so obsequious and sycophantic.

## Development Commands

**Frontend (Svelte + Vite):**
- `npm run dev` - Start development server
- `npm run build` - Build frontend (runs TypeScript compilation + Vite build)
- `npm run preview` - Preview production build

**Tauri (Rust backend):**
- `npm run tauri dev` - Start Tauri development mode (runs both frontend and backend)
- `npm run tauri build` - Build production Tauri application

**Testing:**
No test commands are currently configured in package.json.

## Architecture Overview

This is a **Tauri v2 + WebGPU** application for running GPU-accelerated visual simulations. The architecture splits between:

### Frontend (TypeScript/Svelte)
- Located in `src/` directory
- Uses Svelte 5 with Vite as build tool
- Main entry point: `src/App.svelte`
- UI components in `src/lib/`

### Backend (Rust/Tauri)
- Located in `src-tauri/` directory  
- Uses `wgpu` (WebGPU) for GPU compute and rendering
- Main entry: `src-tauri/src/main.rs`

### Key Backend Architecture

**Core Components:**
- `GpuContext` (`main.rs:18-157`) - Unified GPU context with WGPU device, queue, and surface management
- `SimulationManager` (`simulation/manager.rs`) - Central orchestrator for all simulations
- `Simulation` trait (`simulations/traits.rs`) - Common interface for all simulation types

**Simulation Types:**
- **SlimeMold** - Particle-based slime mold simulation
- **GrayScott** - Reaction-diffusion system simulation  
- **MainMenu** - Background visual for main menu

**Modular Design:**
- Each simulation type implements the `Simulation` trait
- `SimulationType` enum provides type-safe simulation handling without trait objects
- Shared components in `simulations/shared/` (camera, LUTs, etc.)
- Commands are organized by functionality in `commands/` module

**GPU Resource Management:**
- All simulations share the same GPU context (device, queue, surface)
- WGPU limits increased for large buffer support (2GB max buffer size)
- Surface resizing handled centrally through `GpuContext::resize_surface`

**Frontend-Backend Communication:**
- Tauri commands expose backend functionality to frontend
- Over 40 commands registered for simulation control, rendering, camera, presets, etc.
- Commands are organized in separate modules by functionality

**Key Patterns:**
- Settings vs State separation: Settings are user-configurable and saveable in presets, State is runtime data
- Async GPU operations managed through Tokio runtime
- Render loop runs on separate tokio task with FPS limiting
- Camera system supports pan/zoom with world-to-screen coordinate conversion