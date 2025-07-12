<SimulationLayout
  simulationName="Gray-Scott"
  {running}
  {loading}
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


  <form on:submit|preventDefault>
    <!-- Preset Controls -->
    <fieldset>
      <legend>Presets</legend>
      <div class="control-group">
        <Selector
          options={available_presets}
          bind:value={current_preset}
          placeholder="Select preset..."
          on:change={({ detail }) => updatePreset(detail.value)}
        />
      </div>
      <div class="preset-actions">
        <button type="button" on:click={() => (show_save_preset_dialog = true)}>
          Save Current Settings
        </button>
      </div>
    </fieldset>

    <!-- Display Settings -->
    <fieldset>
      <legend>Display Settings</legend>
      <div class="control-group">
        <LutSelector
          bind:available_luts
          current_lut={lut_name}
          reversed={lut_reversed}
          on:select={({ detail }) => updateLut(detail.name)}
          on:reverse={() => updateLutReversed()}
        />
      </div>
    </fieldset>

    <!-- Controls -->
    <fieldset>
      <legend>Controls</legend>
      <div class="interaction-controls-grid">
        <div class="interaction-help">
          <div class="control-group">
            <span>🖱️ Left click: Seed reaction | Right click: Erase</span>
          </div>
          <div class="control-group">
            <button type="button" on:click={() => dispatch('navigate', 'how-to-play')}>
              📖 Camera Controls
            </button>
          </div>
          <div class="control-group">
            <span>Camera controls not working? Click the control bar at the top of the screen.</span>
          </div>
        </div>
        <div class="cursor-settings">
          <div class="cursor-settings-header">
            <span>🎯 Cursor Settings</span>
          </div>
          <CursorConfig
            cursorSize={settings.cursor_size}
            cursorStrength={settings.cursor_strength}
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
        <button
          type="button"
          on:click={async () => {
            try {
              await invoke('reset_simulation');
              console.log('Simulation reset successfully');
            } catch (e) {
              console.error('Failed to reset simulation:', e);
            }
          }}>🔄 Reset Simulation</button
        >
        <button
          type="button"
          on:click={async () => {
            try {
              await invoke('randomize_settings');
              await syncSettingsFromBackend(); // Sync UI with new random settings
              console.log('Settings randomized successfully');
            } catch (e) {
              console.error('Failed to randomize settings:', e);
            }
          }}>🎲 Randomize Settings</button
        >
        <button
          type="button"
          on:click={async () => {
            try {
              await invoke('seed_random_noise');
              console.log('Random noise seeded successfully');
            } catch (e) {
              console.error('Failed to seed random noise:', e);
            }
          }}>🌱 Seed Noise</button
        >
      </div>
    </fieldset>

    <!-- Reaction-Diffusion -->
    <fieldset>
      <legend>
        <button
          type="button"
          class="toggle-button"
          on:click={() => (show_physics_diagram = !show_physics_diagram)}
        >
          {show_physics_diagram ? '▼' : '▶'} Reaction-Diffusion
        </button>
      </legend>

      {#if show_physics_diagram}
        <GrayScottDiagram
          feedRate={settings.feed_rate}
          killRate={settings.kill_rate}
          diffusionRateU={settings.diffusion_rate_u}
          diffusionRateV={settings.diffusion_rate_v}
          timestep={settings.timestep}
          on:update={async (e) => {
            console.log('GrayScottDiagram update event:', e.detail);
            try {
              // Update local settings first for immediate UI feedback
              const settingName = e.detail.setting;
              const value = e.detail.value;

              // Update the local settings object to match the backend
              if (settingName in settings) {
                settings = { ...settings, [settingName]: value };
              }

              // Send the update to the backend
              await invoke('update_simulation_setting', {
                settingName: settingName,
                value: value,
              });

              console.log(`Updated ${settingName} to ${value}`);
            } catch (err) {
              console.error('Failed to update setting:', err);
            }
          }}
        />
      {/if}
    </fieldset>
  </form>
</SimulationLayout>

    <!-- Save Preset Dialog -->
  {#if show_save_preset_dialog}
    <SavePresetDialog
      bind:presetName={new_preset_name}
      on:save={({ detail }) => savePreset(detail.name)}
      on:close={() => (show_save_preset_dialog = false)}
    />
  {/if}

  <!-- Shared camera controls component -->
  <CameraControls 
    enabled={true} 
    on:toggleGui={toggleBackendGui}
  />

<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import Selector from './components/inputs/Selector.svelte';
  import LutSelector from './components/shared/LutSelector.svelte';
  import GrayScottDiagram from './components/gray-scott/GrayScottDiagram.svelte';
  import SavePresetDialog from './components/shared/SavePresetDialog.svelte';
  import CameraControls from './components/shared/CameraControls.svelte';
  import './shared-theme.css';

  const dispatch = createEventDispatcher();

  export let menuPosition: string = 'middle';

  interface Settings {
    feed_rate: number;
    kill_rate: number;
    diffusion_rate_u: number;
    diffusion_rate_v: number;
    timestep: number;
    nutrient_pattern: string;
    nutrient_pattern_reversed: boolean;
    cursor_size: number;
    cursor_strength: number;
  }

  // Simulation state
  let settings: Settings = {
    // Reaction-Diffusion Settings
    feed_rate: 0.055,
    kill_rate: 0.062,
    diffusion_rate_u: 0.1,
    diffusion_rate_v: 0.05,
    timestep: 1.0,

    // Nutrient Pattern Settings
    nutrient_pattern: 'Uniform',
    nutrient_pattern_reversed: false,

    // Cursor Settings
    cursor_size: 10,
    cursor_strength: 0.5,
  };

  // Preset and LUT state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

  // Dialog state
  let show_save_preset_dialog = false;
  let new_preset_name = '';

  // UI state
  let show_physics_diagram = false;

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
      await invoke('toggle_lut_reversed');
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

  async function savePreset(presetName?: string) {
    const nameToSave = presetName || new_preset_name;
    try {
      await invoke('save_preset', { preset_name: nameToSave });
      show_save_preset_dialog = false;
      new_preset_name = '';
      // Refresh the available presets list
      await loadAvailablePresets();
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
      available_luts = await invoke('get_available_luts');
    } catch (e) {
      console.error('Failed to load available LUTs:', e);
    }
  }

  // Sync settings from backend to frontend
  async function syncSettingsFromBackend() {
    try {
      const currentSettings = await invoke('get_current_settings');
      const currentState = (await invoke('get_current_state')) as {
        current_lut_name: string;
        lut_reversed: boolean;
      } | null;

      if (currentSettings) {
        // Log the sync for debugging
        console.log('Syncing settings from backend:', {
          before: { ...settings },
          backend: currentSettings,
          currentPreset: current_preset,
        });

        // Update the settings object with current backend values
        settings = {
          ...settings,
          ...currentSettings,
        };

        console.log('Settings after sync:', { ...settings });
      }

      if (currentState) {
        // Update LUT-related settings from state
        lut_name = currentState.current_lut_name;
        lut_reversed = currentState.lut_reversed;
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  let simulationInitializedUnlisten: (() => void) | null = null;
  let simulationResumedUnlisten: (() => void) | null = null;
  let fpsUpdateUnlisten: (() => void) | null = null;



  async function randomizeSimulation() {
    try {
      await invoke('randomize_settings');
      await syncSettingsFromBackend();
      console.log('Settings randomized via keyboard shortcut');
    } catch (e) {
      console.error('Failed to randomize settings:', e);
    }
  }

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

  // Update camera control functions to fetch camera state after movement
  async function panCamera(deltaX: number, deltaY: number) {
    try {
      await invoke('pan_camera', { deltaX, deltaY });
      await fetchCameraState();
    } catch (e) {
      console.error('Failed to pan camera:', e);
    }
  }

  async function zoomCamera(delta: number) {
    try {
      await invoke('zoom_camera', { delta });
      await fetchCameraState();
    } catch (e) {
      console.error('Failed to zoom camera:', e);
    }
  }

  async function resetCamera() {
    try {
      await invoke('reset_camera');
      console.log('Camera reset successfully');
    } catch (e) {
      console.error('Failed to reset camera:', e);
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

      try {
        await invoke('zoom_camera_to_cursor', {
          delta: zoomDelta,
          cursorX: wheelEvent.clientX,
          cursorY: wheelEvent.clientY,
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

      isMousePressed = false;

      // Stop cursor interaction when mouse is released
      try {
        await invoke('handle_mouse_release', { mouseButton: currentMouseButton });
      } catch (e) {
        console.error('Failed to stop Gray-Scott mouse interaction:', e);
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
    }, 3000);
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
      await invoke('apply_lut_by_name', { lutName: name });
      await syncSettingsFromBackend(); // Sync UI with backend state
    } catch (e) {
      console.error('Failed to update LUT:', e);
    }
  }

  onMount(() => {


    // Add event listeners for auto-hide functionality
    const events = ['mousedown', 'mousemove', 'keydown', 'wheel', 'touchstart'];
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
    const events = ['mousedown', 'mousemove', 'keydown', 'wheel', 'touchstart'];
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
  /* Gray-Scott specific styles (if any) */

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

  .toggle-button {
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    padding: 0;
    margin: 0;
    text-align: left;
    width: 100%;
  }

  .toggle-button:hover {
    color: #60a5fa;
  }
</style>
