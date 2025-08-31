<SimulationLayout
  simulationName="Gray-Scott"
  {running}
  loading={loading || !settings}
  {showUI}
  {currentFps}
  {controlsVisible}
  {menuPosition}
  on:back={returnToMenu}
  on:toggleUI={toggleBackendGui}
  on:pause={stopSimulation}
  on:resume={resumeSimulation}
  on:userInteraction={handleUserInteraction}
  on:mouseEvent={handleMouseEvent}
>
  {#if settings}
    <form on:submit|preventDefault>
      <!-- About this simulation -->
      <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
        <p>
          The Gray-Scott simulation demonstrates reaction-diffusion patterns that occur in chemical
          and biological systems. Two virtual chemicals, U and V, interact and diffuse through
          space, creating complex, self-organizing patterns.
        </p>
        <p>
          The simulation is governed by reaction-diffusion equations with feed and kill rates that
          determine how the chemicals interact. Different parameter combinations produce
          dramatically different patterns - from spots and stripes to spirals and labyrinthine
          structures.
        </p>
        <p>
          Click to seed reactions, adjust the parameters to explore different behaviors, and watch
          as simple chemical rules generate intricate, ever-changing patterns reminiscent of natural
          phenomena like coral growth, bacterial colonies, and animal coat patterns.
        </p>
      </CollapsibleFieldset>

      <!-- Preset Controls -->
      <PresetFieldset
        availablePresets={available_presets}
        bind:currentPreset={current_preset}
        placeholder="Select preset..."
        on:presetChange={({ detail }) => updatePreset(detail.value)}
        on:presetSave={({ detail }) => savePreset(detail.name)}
      />

      <!-- Display Settings -->
      <fieldset>
        <legend>Display Settings</legend>
        <div class="control-group">
          <LutSelector
            bind:available_color_schemes={available_luts}
            current_color_scheme={lut_name}
            reversed={lut_reversed}
            on:select={({ detail }) => updateLut(detail.name)}
            on:reverse={() => updateLutReversed()}
          />
        </div>
      </fieldset>

      <!-- Post Processing -->
      <PostProcessingMenu simulationType="gray_scott" />

      <!-- Controls -->
      <fieldset>
        <legend>Controls</legend>
        <div class="interaction-controls-grid">
          <div class="interaction-help">
            <div class="control-group">
              <span>🖱️ Left click: Seed reaction | Right click: Erase</span>
            </div>
            <div class="control-group">
              <Button variant="default" on:click={() => dispatch('navigate', 'how-to-play')}>
                📖 Camera Controls
              </Button>
            </div>
            <div class="control-group">
              <span
                >Camera controls not working? Click the control bar at the top of the screen.</span
              >
            </div>
          </div>
          <div class="cursor-settings">
            <div class="cursor-settings-header">
              <span>🎯 Cursor Settings</span>
            </div>
            <CursorConfig
              {cursorSize}
              {cursorStrength}
              sizeMin={5}
              sizeMax={50}
              sizeStep={1}
              strengthMin={0.1}
              strengthMax={2.0}
              strengthStep={0.1}
              sizePrecision={0}
              strengthPrecision={1}
              on:sizechange={async (e) => {
                try {
                  cursorSize = e.detail;
                  await invoke('update_simulation_setting', {
                    settingName: 'cursor_size',
                    value: e.detail,
                  });
                } catch (err) {
                  console.error('Failed to update cursor size:', err);
                }
              }}
              on:strengthchange={async (e) => {
                try {
                  cursorStrength = e.detail;
                  await invoke('update_simulation_setting', {
                    settingName: 'cursor_strength',
                    value: e.detail,
                  });
                } catch (err) {
                  console.error('Failed to update cursor strength:', err);
                }
              }}
            />
          </div>
        </div>
      </fieldset>

      <!-- Settings -->
      <fieldset>
        <legend>Settings</legend>
        <div class="control-group">
          <Button
            variant="warning"
            type="button"
            on:click={async () => {
              try {
                await invoke('reset_simulation');
                console.log('Simulation reset successfully');
              } catch (e) {
                console.error('Failed to reset simulation:', e);
              }
            }}>🔄 Reset Simulation</Button
          >
          <Button
            variant="warning"
            type="button"
            on:click={async () => {
              try {
                await invoke('randomize_settings');
                await syncSettingsFromBackend();
                console.log('Settings randomized successfully');
              } catch (e) {
                console.error('Failed to randomize settings:', e);
              }
            }}>🎲 Randomize Settings</Button
          >
          <Button
            variant="primary"
            type="button"
            on:click={async () => {
              try {
                await invoke('seed_random_noise');
                console.log('Random noise seeded successfully');
              } catch (e) {
                console.error('Failed to seed random noise:', e);
              }
            }}>🌱 Seed Noise</Button
          >
        </div>
      </fieldset>

      <!-- Reaction-Diffusion -->
      <fieldset>
        <legend>Reaction-Diffusion</legend>
        <GrayScottDiagram
          feedRate={settings.feed_rate}
          killRate={settings.kill_rate}
          diffusionRateU={settings.diffusion_rate_u}
          diffusionRateV={settings.diffusion_rate_v}
          timestep={settings.timestep}
          on:update={async (e) => {
            console.log('GrayScottDiagram update event:', e.detail);
            try {
              const settingName = e.detail.setting as keyof Settings;
              const value = e.detail.value as number;
              if (!settings) return;

              const updated: Settings = { ...settings };
              if (settingName === 'feed_rate') updated.feed_rate = value;
              else if (settingName === 'kill_rate') updated.kill_rate = value;
              else if (settingName === 'diffusion_rate_u') updated.diffusion_rate_u = value;
              else if (settingName === 'diffusion_rate_v') updated.diffusion_rate_v = value;
              else if (settingName === 'timestep') updated.timestep = value;
              settings = updated;

              await invoke('update_simulation_setting', {
                settingName,
                value,
              });
            } catch (err) {
              console.error('Failed to update setting:', err);
            }
          }}
        />

        <!-- Nutrient Gradients (merged) -->
        <div class="control-group" style="margin-top: 0.5rem;">
          <Selector
            label="Nutrient Pattern"
            options={nutrient_pattern_options}
            value={settings?.nutrient_pattern || 'Uniform'}
            placeholder="Select pattern..."
            on:change={({ detail }) => updateNutrientPattern(detail.value)}
          />
        </div>
        <div class="control-group">
          <label class="checkbox">
            <input
              type="checkbox"
              checked={settings?.nutrient_pattern_reversed || false}
              on:change={(e) =>
                updateNutrientPatternReversed((e.target as HTMLInputElement).checked)}
            />
            Reverse nutrient pattern
          </label>
        </div>
      </fieldset>
    </form>
  {/if}
</SimulationLayout>

<!-- Shared camera controls component -->
<CameraControls enabled={true} on:toggleGui={toggleBackendGui} on:togglePause={togglePause} />

<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import LutSelector from './components/shared/ColorSchemeSelector.svelte';
  import GrayScottDiagram from './components/gray-scott/GrayScottDiagram.svelte';
  import CameraControls from './components/shared/CameraControls.svelte';
  import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
  import PresetFieldset from './components/shared/PresetFieldset.svelte';
  import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
  import Button from './components/shared/Button.svelte';
  import Selector from './components/inputs/Selector.svelte';
  import './shared-theme.css';

  const dispatch = createEventDispatcher();

  export let menuPosition: string = 'middle';
  export let autoHideDelay: number = 3000;

  interface Settings {
    feed_rate: number;
    kill_rate: number;
    diffusion_rate_u: number;
    diffusion_rate_v: number;
    timestep: number;
    nutrient_pattern: string;
    nutrient_pattern_reversed: boolean;
  }

  // Simulation state
  let settings: Settings | undefined = undefined;

  // Cursor state (not saved in presets)
  let cursorSize = 10.0;
  let cursorStrength = 0.5;

  // Preset and LUT state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

  // Nutrient gradient options (display names match backend serialization)
  const nutrient_pattern_options: string[] = [
    'Uniform',
    'Checkerboard',
    'Diagonal Gradient',
    'Radial Gradient',
    'Vertical Stripes',
    'Horizontal Stripes',
    'Enhanced Noise',
    'Wave Function',
    'Cosine Grid',
  ];

  async function updateNutrientPattern(value: string) {
    if (!settings) return;
    try {
      settings = { ...settings, nutrient_pattern: value };
      await invoke('update_simulation_setting', {
        settingName: 'nutrient_pattern',
        value,
      });
    } catch (err) {
      console.error('Failed to set nutrient pattern:', err);
    }
  }

  async function updateNutrientPatternReversed(checked: boolean) {
    if (!settings) return;
    try {
      settings = { ...settings, nutrient_pattern_reversed: checked };
      await invoke('update_simulation_setting', {
        settingName: 'nutrient_pattern_reversed',
        value: checked,
      });
    } catch (err) {
      console.error('Failed to toggle nutrient reversal:', err);
    }
  }

  // UI state
  let show_about_section = false;

  // LUT state (runtime, not saved in presets)
  let lut_name = '';
  let lut_reversed = false;

  // Auto-hide functionality for controls when UI is hidden
  let controlsVisible = true;
  let hideTimeout: number | null = null;

  // Cursor hiding functionality
  let cursorHidden = false;
  let cursorHideTimeout: number | null = null;

  async function updateLutReversed() {
    try {
      await invoke('toggle_color_scheme_reversed');
      await syncSettingsFromBackend(); // Sync UI with backend state
    } catch (e) {
      console.error('Failed to toggle LUT reversed:', e);
    }
  }

  async function updatePreset(value: string) {
    current_preset = value;
    try {
      await invoke('apply_preset', { presetName: value });
      await syncSettingsFromBackend(); // Sync UI with new settings
      console.log(`Applied preset: ${value}`);
    } catch (e) {
      console.error('Failed to apply preset:', e);
    }
  }

  async function savePreset(presetName: string) {
    try {
      await invoke('save_preset', { presetName: presetName.trim() });
      // Refresh the available presets list
      await loadAvailablePresets();
      // Set the current preset to the newly saved one
      current_preset = presetName.trim();
      console.log(`Saved preset: ${presetName}`);
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  // Simulation state
  let running = false;
  let loading = false;
  let currentFps = 0;
  let showUI = true;

  async function startSimulation() {
    if (running || loading) return;

    loading = true;

    try {
      await invoke('start_gray_scott_simulation');
      // Don't set running = true here - wait for simulation-initialized event
      // The simulation-initialized event will set running = true when everything is ready
      currentFps = 0;
    } catch (e) {
      console.error('Failed to start simulation:', e);
      loading = false;
      running = false;
    }
  }

  async function resumeSimulation() {
    if (running || loading) return;

    try {
      // Just restart the render loop without recreating the simulation
      await invoke('resume_simulation');
      running = true;
      currentFps = 0;
    } catch (e) {
      console.error('Failed to resume simulation:', e);
    }
  }

  async function stopSimulation() {
    running = false;

    try {
      // Just pause the render loop, don't destroy simulation
      await invoke('pause_simulation');

      // Reset FPS
      currentFps = 0;

      // Immediately render a frame to show the triangle instead of last simulation frame
      await invoke('render_frame');
    } catch (e) {
      console.error('Failed to stop simulation:', e);
    }
  }

  async function togglePause() {
    if (running) {
      await stopSimulation();
    } else {
      await resumeSimulation();
    }
  }

  async function destroySimulation() {
    running = false;

    try {
      // Actually destroy the simulation completely
      await invoke('destroy_simulation');

      // Reset FPS
      currentFps = 0;

      // Render a frame to show the triangle
      await invoke('render_frame');
    } catch (e) {
      console.error('Failed to destroy simulation:', e);
    }
  }

  async function returnToMenu() {
    await destroySimulation();
    dispatch('back');
  }

  // Load available presets from backend
  let initialPresetApplied = false;
  async function loadAvailablePresets() {
    try {
      available_presets = await invoke('get_presets_for_simulation_type', {
        simulationType: 'gray_scott',
      });
      // Only apply initial preset once on first load, not on subsequent calls
      if (available_presets.length > 0 && !current_preset && !initialPresetApplied) {
        current_preset = available_presets.includes('Undulating')
          ? 'Undulating'
          : available_presets[0];
        // Apply the initial preset to the simulation
        await invoke('apply_preset', { presetName: current_preset });
        initialPresetApplied = true;
        console.log(`Applied initial preset: ${current_preset}`);
      }
    } catch (e) {
      console.error('Failed to load available presets:', e);
    }
  }

  // Load available LUTs from backend
  async function loadAvailableLuts() {
    try {
      available_luts = await invoke('get_available_color_schemes');
    } catch (e) {
      console.error('Failed to load available LUTs:', e);
    }
  }

  // Sync settings from backend to frontend
  async function syncSettingsFromBackend() {
    try {
      const backendSettings = await invoke('get_current_settings');
      const backendState = await invoke('get_current_state');

      if (backendSettings) {
        // Use backend settings directly
        settings = backendSettings as Settings;
      }

      if (backendState) {
        // Update LUT-related settings from state
        const state = backendState as {
          current_lut_name?: string;
          lut_reversed?: boolean;
          cursor_size?: number;
          cursor_strength?: number;
        };
        if (state.current_lut_name !== undefined) {
          lut_name = state.current_lut_name;
        }
        if (state.lut_reversed !== undefined) {
          lut_reversed = state.lut_reversed;
        }

        // Update cursor configuration from state
        if (state.cursor_size !== undefined) {
          cursorSize = state.cursor_size;
        }
        if (state.cursor_strength !== undefined) {
          cursorStrength = state.cursor_strength;
        }
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  let simulationInitializedUnlisten: (() => void) | null = null;
  let simulationResumedUnlisten: (() => void) | null = null;
  let fpsUpdateUnlisten: (() => void) | null = null;

  // Add a function to fetch the latest camera state from the backend
  async function fetchCameraState() {
    try {
      const cam = (await invoke('get_camera_state')) as {
        position: number[];
        zoom: number;
        viewport_width: number;
        viewport_height: number;
        aspect_ratio: number;
      };
      if (cam) {
        console.log('Camera state fetched:', cam);
      }
    } catch (e) {
      console.error('Failed to fetch camera state:', e);
    }
  }

  async function sendCursorToBackend(screenX: number, screenY: number) {
    try {
      await invoke('update_cursor_position_screen', {
        screenX,
        screenY,
      });
    } catch (err) {
      console.error('Failed to update cursor position:', err);
    }
  }

  // Mouse state tracking for dragging support
  let isMousePressed = false;
  let currentMouseButton = 0;

  // Mouse event handling for camera controls and simulation interaction
  async function handleMouseEvent(e: CustomEvent) {
    const event = e.detail as MouseEvent | WheelEvent;

    if (event.type === 'wheel') {
      const wheelEvent = event as WheelEvent;
      wheelEvent.preventDefault();

      const zoomDelta = -wheelEvent.deltaY * 0.001;

      // Convert screen coordinates to physical coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = wheelEvent.clientX * devicePixelRatio;
      const physicalCursorY = wheelEvent.clientY * devicePixelRatio;

      try {
        await invoke('zoom_camera_to_cursor', {
          delta: zoomDelta,
          cursorX: physicalCursorX,
          cursorY: physicalCursorY,
        });
      } catch (e) {
        console.error('Failed to zoom camera to cursor:', e);
      }
    } else if (event.type === 'mousedown') {
      const mouseEvent = event as MouseEvent;
      mouseEvent.preventDefault();

      isMousePressed = true;
      currentMouseButton = mouseEvent.button;

      // Convert screen coordinates to physical coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
      const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

      console.log(
        `Gray-Scott mouse interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: ${mouseEvent.button}`
      );

      try {
        await invoke('handle_mouse_interaction_screen', {
          screenX: physicalCursorX,
          screenY: physicalCursorY,
          mouseButton: mouseEvent.button,
        });
      } catch (e) {
        console.error('Failed to handle Gray-Scott mouse interaction:', e);
      }
    } else if (event.type === 'mousemove') {
      const mouseEvent = event as MouseEvent;

      // Convert screen coordinates to physical coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
      const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

      if (isMousePressed) {
        // Continue interaction while dragging
        mouseEvent.preventDefault();

        try {
          await invoke('handle_mouse_interaction_screen', {
            screenX: physicalCursorX,
            screenY: physicalCursorY,
            mouseButton: currentMouseButton,
          });
        } catch (e) {
          console.error('Failed to handle Gray-Scott mouse drag:', e);
        }
      } else {
        // Just update cursor position for visual feedback when not dragging
        try {
          await sendCursorToBackend(physicalCursorX, physicalCursorY);
        } catch (e) {
          console.error('Failed to update cursor position:', e);
        }
      }
    } else if (event.type === 'mouseup') {
      const mouseEvent = event as MouseEvent;
      mouseEvent.preventDefault();

      // Only handle mouseup if we were actually tracking a mouse press
      if (isMousePressed) {
        isMousePressed = false;

        // Stop cursor interaction when mouse is released
        try {
          await invoke('handle_mouse_release', { mouseButton: currentMouseButton });
        } catch (e) {
          console.error('Failed to stop Gray-Scott mouse interaction:', e);
        }
      }
    } else if (event.type === 'contextmenu') {
      // Handle context menu as right-click for simulation interaction
      const mouseEvent = event as MouseEvent;

      // Convert screen coordinates to physical coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
      const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

      console.log(
        `Gray-Scott context menu interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: 2`
      );

      // Track as active right-button press to ensure release is generated later
      isMousePressed = true;
      currentMouseButton = 2;

      try {
        await invoke('handle_mouse_interaction_screen', {
          screenX: physicalCursorX,
          screenY: physicalCursorY,
          mouseButton: 2, // Right mouse button
        });
      } catch (e) {
        console.error('Failed to handle Gray-Scott context menu interaction:', e);
      }
    }
  }

  // Initialize camera state with proper type
  // Note: camera_state is now fetched from backend when needed

  async function toggleBackendGui() {
    try {
      await invoke('toggle_gui');
      // Sync UI state with backend
      const isVisible = await invoke<boolean>('get_gui_state');
      showUI = isVisible;

      // Handle auto-hide when UI is hidden
      if (!showUI) {
        showControls();
        showCursor();
        startAutoHideTimer();
        startCursorHideTimer();
      } else {
        stopAutoHideTimer();
        stopCursorHideTimer();
        showCursor();
        controlsVisible = true;
      }
    } catch (err) {
      console.error('Failed to toggle backend GUI:', err);
    }
  }

  // Auto-hide functionality
  function startAutoHideTimer() {
    stopAutoHideTimer();
    hideTimeout = window.setTimeout(() => {
      controlsVisible = false;
      // Also hide cursor when controls are hidden
      if (!showUI) {
        hideCursor();
      }
    }, autoHideDelay);
  }

  function stopAutoHideTimer() {
    if (hideTimeout) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }
  }

  function showControls() {
    controlsVisible = true;
  }

  // Cursor hiding functionality
  function hideCursor() {
    if (!cursorHidden) {
      document.body.style.cursor = 'none';
      cursorHidden = true;
    }
  }

  function showCursor() {
    if (cursorHidden) {
      document.body.style.cursor = '';
      cursorHidden = false;
    }
  }

  function startCursorHideTimer() {
    stopCursorHideTimer();
    cursorHideTimeout = window.setTimeout(() => {
      if (!showUI && !controlsVisible) {
        hideCursor();
      }
    }, 2000); // Hide cursor 2 seconds after last interaction
  }

  function stopCursorHideTimer() {
    if (cursorHideTimeout) {
      clearTimeout(cursorHideTimeout);
      cursorHideTimeout = null;
    }
  }

  function handleUserInteraction() {
    if (!showUI && !controlsVisible) {
      showControls();
      showCursor();
      startAutoHideTimer();
    } else if (!showUI && controlsVisible) {
      showCursor();
      startAutoHideTimer();
      startCursorHideTimer();
    }
  }

  async function updateLut(name: string) {
    try {
      await invoke('apply_color_scheme_by_name', { colorSchemeName: name });
      await syncSettingsFromBackend(); // Sync UI with backend state
    } catch (e) {
      console.error('Failed to update LUT:', e);
    }
  }

  onMount(() => {
    // Add event listeners for auto-hide functionality (excluding keydown to avoid conflicts with CameraControls)
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.addEventListener(event, handleUserInteraction, { passive: true });
    });

    // Listen for simulation initialization event
    listen('simulation-initialized', async () => {
      console.log('Simulation initialized, syncing settings...');
      // Load presets and LUTs after simulation is initialized
      await loadAvailablePresets();
      await loadAvailableLuts();
      await syncSettingsFromBackend();

      // Fetch initial camera state to get correct viewport dimensions
      await fetchCameraState();

      // Initialize cursor position to center of screen so golden crosshair is visible
      const centerX = window.innerWidth / 2;
      const centerY = window.innerHeight / 2;
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCenterX = centerX * devicePixelRatio;
      const physicalCenterY = centerY * devicePixelRatio;
      sendCursorToBackend(physicalCenterX, physicalCenterY);

      // Seed random noise to start with interesting patterns
      try {
        await invoke('seed_random_noise');
        console.log('Initial random noise seeded successfully');
      } catch (e) {
        console.error('Failed to seed initial random noise:', e);
      }

      // Now that simulation is fully initialized, set running to true
      loading = false;
      running = true;
      console.log('Simulation is now running and ready for mouse interaction');
    }).then((unlisten) => {
      simulationInitializedUnlisten = unlisten;
    });

    // Listen for simulation resumed event
    listen('simulation-resumed', async () => {
      console.log('Simulation resumed');
      running = true;
      currentFps = 0;
    }).then((unlisten) => {
      simulationResumedUnlisten = unlisten;
    });

    // Listen for FPS updates from backend
    listen('fps-update', (event) => {
      currentFps = event.payload as number;
    }).then((unlisten) => {
      fpsUpdateUnlisten = unlisten;
    });

    // Then start simulation
    startSimulation();

    return () => {
      stopSimulation();
    };
  });

  onDestroy(() => {
    // Remove keyboard event listeners

    // Remove auto-hide event listeners
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.removeEventListener(event, handleUserInteraction);
    });

    // Stop auto-hide timer
    stopAutoHideTimer();

    // Stop cursor hide timer and restore cursor
    stopCursorHideTimer();
    showCursor();

    // Cancel animation frame

    if (simulationInitializedUnlisten) {
      simulationInitializedUnlisten();
    }
    if (simulationResumedUnlisten) {
      simulationResumedUnlisten();
    }
    if (fpsUpdateUnlisten) {
      fpsUpdateUnlisten();
    }
  });
</script>

<style>
  .interaction-controls-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    align-items: start;
  }

  .interaction-help {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .cursor-settings {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .cursor-settings-header {
    font-size: 0.9rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    padding: 0.25rem 0;
  }

  /* Mobile responsive design */
  @media (max-width: 768px) {
    .interaction-controls-grid {
      grid-template-columns: 1fr;
      gap: 0.75rem;
    }

    .interaction-help {
      gap: 0.4rem;
    }

    .cursor-settings {
      gap: 0.4rem;
    }

    .cursor-settings-header {
      font-size: 0.85rem;
    }
  }
</style>
