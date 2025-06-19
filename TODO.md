# TODO - Rust Codebase Refactoring

## Definitions

### State vs Settings

- **Settings**: Data that can and should be saved when saving a preset. These are user-configurable parameters that define how the simulation behaves.
  - Examples: agent count, speed, turn rate, diffusion rate, LUT selection, camera position, etc.
  - Settings are serializable and persistable
  - Settings can be modified by the user through the UI
  - Settings are part of the simulation's configuration

- **State**: Everything else that represents the current runtime condition of the simulation.
  - Examples: current agent positions, trail map data, simulation time, render loop status, GUI visibility, etc.
  - State is typically not saved with presets
  - State may be transient or computed
  - State represents the simulation's current execution state

This distinction is crucial for:
- Preset management (only save settings, not state)
- State restoration after simulation restart
- UI state management
- Performance optimization (state can be recomputed, settings should be cached)

## High Priority - Immediate Improvements

### 1. ✅ Extract Command Handlers from main.rs - COMPLETED
- [x] Create `src-tauri/src/commands/` directory
- [x] Create `commands/mod.rs` to export all command modules
- [x] Extract simulation commands to `commands/simulation.rs`
  - [x] `start_slime_mold_simulation`
  - [x] `start_gray_scott_simulation`
  - [x] `pause_simulation`
  - [x] `resume_simulation`
  - [x] `destroy_simulation`
  - [x] `get_simulation_status`
- [x] Extract rendering commands to `commands/rendering.rs`
  - [x] `render_frame`
  - [x] `render_single_frame`
  - [x] `handle_window_resize`
- [x] Extract preset commands to `commands/presets.rs`
  - [x] `get_available_presets`
  - [x] `get_slime_mold_presets`
  - [x] `get_gray_scott_presets`
  - [x] `apply_preset`
  - [x] `save_preset`
  - [x] `delete_preset`
- [x] Extract LUT commands to `commands/luts.rs`
  - [x] `apply_lut_by_name`
  - [x] `apply_lut`
  - [x] `toggle_lut_reversed`
  - [x] `apply_custom_lut`
  - [x] `save_custom_lut`
  - [x] `update_gradient_preview`
  - [x] `get_available_luts`
- [x] Extract camera commands to `commands/camera.rs`
  - [x] `pan_camera`
  - [x] `zoom_camera`
  - [x] `zoom_camera_to_cursor`
  - [x] `reset_camera`
  - [x] `get_camera_state`
  - [x] `stop_camera_pan`
- [x] Extract settings commands to `commands/settings.rs`
  - [x] `update_simulation_setting`
  - [x] `get_current_settings`
  - [x] `get_current_state`
  - [x] `randomize_settings`
- [x] Extract interaction commands to `commands/interaction.rs`
  - [x] `handle_mouse_interaction`
  - [x] `handle_mouse_interaction_screen`
  - [x] `update_cursor_position_screen`
  - [x] `seed_random_noise`
- [x] Extract utility commands to `commands/utility.rs`
  - [x] `check_gpu_context_ready`
  - [x] `toggle_gui`
  - [x] `get_gui_state`
  - [x] `set_fps_limit`
- [x] Extract reset commands to `commands/reset.rs`
  - [x] `reset_trails`
  - [x] `reset_agents`
  - [x] `reset_simulation`
- [x] Update `main.rs` to use the new command modules

**Implementation Notes:**
- All command handlers have been successfully extracted into focused modules
- Each module contains related commands with proper error handling and logging
- The main.rs file is now much cleaner and focused on application setup
- All commands maintain the same API as before, ensuring backward compatibility
- Proper imports and dependencies have been set up

### 2. ✅ Create Simulation Trait/Interface - COMPLETED
- [x] Create `src-tauri/src/simulations/traits.rs`
- [x] Define `Simulation` trait with common methods:
  - [x] `render_frame()`
  - [x] `resize()`
  - [x] `update_setting()` - For modifying user-configurable settings
  - [x] `get_settings()` - Returns serializable settings (for presets)
  - [x] `get_state()` - Returns current runtime state (not for presets)
  - [x] `handle_mouse_interaction()`
  - [x] `pan_camera()` - Updates camera settings
  - [x] `zoom_camera()` - Updates camera settings
  - [x] `reset_camera()` - Resets camera to default settings
  - [x] `get_camera_state()` - Returns current camera settings
  - [x] `save_preset()` - Saves only settings, not state
  - [x] `load_preset()` - Loads settings and resets state
- [x] Implement `Simulation` trait for `SlimeMoldModel`
- [x] Implement `Simulation` trait for `GrayScottModel`
- [x] Create `SimulationType` enum to replace `Box<dyn Simulation>`
  - [x] Define enum variants for each simulation type
  - [x] Implement `Simulation` trait for the enum
  - [x] Update `SimulationManager` to use enum instead of trait objects
- [x] Ensure clear separation between settings (presettable) and state (runtime)
- [x] Fix compilation errors and import issues
- [x] Remove async trait methods to avoid dyn compatibility issues

**Implementation Notes:**
- Successfully created a unified `Simulation` trait that abstracts common simulation operations
- Implemented the trait for both `SlimeMoldModel` and `GrayScottModel`
- Created `SimulationType` enum to replace trait objects, avoiding async trait compatibility issues
- Updated `SimulationManager` to use the enum-based approach
- Fixed all import statements in command modules
- Code now compiles successfully with only warnings (no errors)

### 3. Refactor SimulationManager
- [x] Start creating `src-tauri/src/simulation/` directory and submodules (manager, render_loop, preset_manager, lut_manager)
- [ ] Incrementally move logic from `simulation_manager.rs` into new submodules, resolving visibility/import issues as needed
- [ ] Update main.rs and command modules to use new submodules once stable
- [ ] Remove duplicate code by using the `Simulation` trait
- [ ] Simplify the manager to delegate to trait implementations

**Current Status:**
- The codebase is currently using the original `simulation_manager.rs` for stability
- The new `simulation/` submodules are scaffolded but not yet integrated
- The next step is to incrementally move logic into the new submodules and ensure compilation after each step
- All simulation trait and enum-based management is complete and compiling
- Ready to proceed with modularizing the SimulationManager

## Medium Priority - Architecture Improvements

### 4. Create Error Types
- [ ] Create `src-tauri/src/error.rs`
- [ ] Define specific error types:
  - [ ] `SimulationError`
  - [ ] `GpuError`
  - [ ] `CommandError`
  - [ ] `PresetError`
  - [ ] `LutError`
- [ ] Replace `Box<dyn std::error::Error>` with specific error types
- [ ] Add proper error context and conversion traits

### 5. Extract GPU Context Management
- [ ] Create `src-tauri/src/gpu/` directory
- [ ] Split `GpuContext` into focused modules:
  - [ ] `gpu/context.rs` - Core GPU context
  - [ ] `gpu/surface.rs` - Surface management
  - [ ] `gpu/renderer.rs` - Main menu renderer
- [ ] Improve surface configuration management
- [ ] Add better error handling for GPU operations

### 6. Create Command Registry Pattern
- [ ] Create `src-tauri/src/commands/registry.rs`
- [ ] Implement `CommandRegistry` struct
- [ ] Create `CommandHandler` trait
- [ ] Auto-generate Tauri command handlers from registry
- [ ] Replace manual command registration in `main.rs`

### 7. Implement Builder Pattern for Simulations
- [ ] Create `src-tauri/src/simulations/builder.rs`
- [ ] Implement `SimulationBuilder` with fluent interface
- [ ] Support configuration through builder methods
- [ ] Validate configuration before building
- [ ] Replace direct simulation instantiation with builders

## Low Priority - Advanced Improvements

### 8. Create State Management System
- [ ] Create `src-tauri/src/state/` directory
- [ ] Implement `AppState` for global application state
- [ ] Implement `SimulationState` for simulation-specific runtime state
- [ ] Implement `SettingsState` for user-configurable settings
- [ ] Add state persistence and restoration
- [ ] Ensure settings are separate from state in all data structures
- [ ] Create clear serialization boundaries between settings and state

### 9. Implement Event System
- [ ] Create `src-tauri/src/events/` directory
- [ ] Define event types:
  - [ ] `SimulationEvents`
  - [ ] `RenderEvents`
  - [ ] `UiEvents`
- [ ] Implement event dispatcher
- [ ] Replace direct function calls with events where appropriate

### 10. Add Configuration Management
- [ ] Create `src-tauri/src/config/` directory
- [ ] Implement `AppConfig` for application configuration
- [ ] Implement `SimulationConfig` for simulation-specific configs
- [ ] Implement `GpuConfig` for GPU-related configuration
- [ ] Add configuration file loading/saving

### 11. Improve Documentation
- [ ] Add comprehensive doc comments to all public APIs
- [ ] Create architecture documentation
- [ ] Add examples for common use cases
- [ ] Document the simulation trait system

### 12. Add Testing
- [ ] Create unit tests for command handlers
- [ ] Create integration tests for simulation lifecycle
- [ ] Create tests for error handling
- [ ] Add benchmarks for performance-critical code

## Code Quality Improvements

### 13. Clippy and Linting
- [ ] Fix all clippy warnings
- [ ] Add custom lint rules where appropriate
- [ ] Ensure consistent code style
- [ ] Add pre-commit hooks for linting

### 14. Performance Optimizations
- [ ] Profile render loop performance
- [ ] Optimize GPU resource management
- [ ] Reduce memory allocations in hot paths
- [ ] Add performance monitoring

### 15. Security and Safety
- [ ] Audit error handling for panics
- [ ] Validate all user inputs
- [ ] Add bounds checking where needed
- [ ] Review unsafe code usage

## Migration Strategy

### ✅ Phase 1: Extract Commands (Week 1) - COMPLETED
1. ✅ Create command modules
2. ✅ Move command handlers
3. ✅ Update main.rs
4. ✅ Test all commands still work

### ✅ Phase 2: Create Simulation Trait (Week 2) - COMPLETED
1. ✅ Define trait interface
2. ✅ Implement for existing simulations
3. ✅ Update SimulationManager
4. ✅ Remove duplicate code
5. ✅ Fix compilation issues

### Phase 3: Refactor SimulationManager (Week 3)
1. Split into focused modules
2. Improve error handling
3. Add proper abstractions
4. Test simulation lifecycle

### Phase 4: Advanced Improvements (Week 4+)
1. Add command registry
2. Implement builder pattern
3. Add state management
4. Improve documentation

## Notes

- Each phase should be completed and tested before moving to the next
- Maintain backward compatibility during refactoring
- Add tests for new abstractions
- Document breaking changes
- Consider creating a migration guide for future contributors
- Remember the State vs Settings distinction throughout all refactoring work

## Recent Progress

**Phase 1 Completed (2024-12-19):**
- Successfully extracted all command handlers from main.rs into focused modules
- Created 8 command modules: simulation, rendering, presets, luts, camera, settings, interaction, utility, and reset
- Maintained backward compatibility - all existing frontend calls continue to work
- Improved code organization and maintainability
- Reduced main.rs from ~1200 lines to ~230 lines
- All commands compile successfully with proper error handling and logging

**Phase 2 Completed (2024-12-19):**
- Successfully created and implemented the `Simulation` trait system
- Implemented the trait for both `SlimeMoldModel` and `GrayScottModel`
- Created `SimulationType` enum to replace trait objects, avoiding async trait compatibility issues
- Updated `SimulationManager` to use the enum-based approach
- Fixed all import statements in command modules
- Code now compiles successfully with only warnings (no errors)
- Maintained clear separation between settings (presettable) and state (runtime)

**Main Menu Fix (2024-12-19):**
- Fixed the `render_frame` and `render_single_frame` commands to properly handle the main menu visualization
- When no simulation is running, the commands now render the animated main menu background
- Restored the original behavior where the main menu shows an animated plasma effect
- Both commands now check `sim_manager.is_running()` and render the appropriate content

**Current Status:**
- All compilation errors have been resolved
- The simulation trait system is fully implemented and working
- Ready to proceed with Phase 3: Refactoring SimulationManager into focused modules