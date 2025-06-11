# Changelog

## [v2.0.0-beta.2]

### Added
- Added FPS limiting with configurable target
- Added frame pacing system for smoother performance
- Added high/low range toggle for decay rate control
- Added fine control buttons for decay rate and deposition rate
- Added window title with FPS display
- Added support for up to 50 million agents

### Changed
- Optimized GPU buffer management and texture handling
- Updated default settings for better performance
- Enhanced UI controls for simulation parameters
- Improved shader organization and performance

### Fixed
- Fixed agent count handling to prevent buffer overruns
- Fixed decay rate calculation and display
- Fixed texture dimension handling for large displays
- Fixed agent movement and wrapping behavior

## [v2.0.0-beta.1]

### Added
- Moved LUTs into src directory
- Added GitHub release workflow for automated releases
- Added gradient system with multiple types (radial, linear, etc.)
- Added LUT preview in color scheme selector
- Added fine control buttons for all numeric settings
- Added keyboard shortcut (/) to toggle UI
- Added FPS display
- Added randomize settings button

### Changed
- Refactored gradient system to use enum-based type selection
- Improved UI layout and responsiveness
- Removed custom presets and simplified preset management
- Updated default settings and presets
- Cleaned up code formatting and structure
- Removed unused font file

### Fixed
- Fixed various UI bugs and edge cases
- Fixed buffer binding sizing

## [2025-05-16]

### Added
- Added example images
- Added license field
- Added text overlay with simulation stats and help text
- Added font rendering support and toggle functionality
- Added new manager modules (bind_group, pipeline, shader)
- Added enhanced settings and preset system

### Changed
- Updated main.rs and settings.rs with latest changes
- Swapped R and F hotkeys (R now controls deposition amount, F toggles LUT reversal)
- Updated simulation display and presets for improved visualization
- Made agent count controllable
- Improved code organization

### Fixed
- Fixed F key LUT reversal to only trigger on key down event

## [2025-05-15]

### Added
- Added GPU-accelerated box blur
- Added static gradient support
- Added support for 50 million agents

### Changed
- Rewritten simulation for GPU
- Made agents GPU-compatible
- Refactored shader organization
- Implemented GPU-based display texture conversion
- Refactored Pheromones to use GPU compute shaders
- Optimized pheromone grid rendering with row-major iteration

## [2025-05-14]

### Added
- Added static gradient generator
- Added support for image-based static gradients
- Added clamping movement for agents
- Added parallel agent iteration
- Added Mac and Linux support for file watching

### Changed
- Refactored to use f32 for pheromone grid
- Removed image crate and used Vec<u8> for pheromone grid
- Changed f64 to f32 for performance
- Corrected window height mismatch between settings and pheromone grid
- Updated core simulation logic and MIDI interface

## [2021-05-28]

### Added
- Added cross-compilation support for Raspberry Pi/BeagleBone
- Added support for new wgpu backends

## [2021-04-27]

### Added
- Added support for image-based static gradients
- Added linear static gradient generator

### Changed
- Updated template defaults
- Improved agent movement

## [2021-04-25]

### Added
- Added MIDI control support
- Added hot reload for simulation settings
- Added B&W vs color toggle

### Changed
- Parallelized various operations
- Removed fastrand
- Fixed agents listing counterclockwise
- Updated agent speed calculation

## [2021-04-23]

### Added
- Added MIDI support

## [2021-04-21]

### Added
- Added more agent controls
- Added gradient toggle
- Added slow gradient colorizer

### Changed
- Set circle field as default

## [2021-04-20]

### Added
- Added swapper data structure
- Reintroduced static pheromone gradient

### Changed
- Simplified diffuser
- Updated pheromone reading

### Fixed
- Fixed broken diffusion
