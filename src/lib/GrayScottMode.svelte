<div class="simulation-container">
  <!-- Mouse interaction overlay -->
  <div
    class="mouse-overlay"
    on:mousedown={handleMouseEvent}
    on:mousemove={handleMouseEvent}
    on:mouseup={handleMouseEvent}
    on:mouseleave={handleMouseEvent}
    on:contextmenu={handleMouseEvent}
    on:wheel={handleMouseEvent}
    role="button"
    tabindex="0"
  ></div>

  <!-- Loading Screen -->
  {#if loading}
    <div class="loading-overlay">
      <div class="loading-content">
        <div class="loading-spinner"></div>
        <h2>Starting Simulation...</h2>
        <p>Initializing GPU resources</p>
      </div>
    </div>
  {/if}

  <SimulationControlBar
    {running}
    {loading}
    {showUI}
    {currentFps}
    simulationName="Gray-Scott"
    {controlsVisible}
    on:back={returnToMenu}
    on:toggleUI={toggleBackendGui}
    on:pause={stopSimulation}
    on:resume={resumeSimulation}
    on:userInteraction={handleUserInteraction}
  />

  <SimulationMenuContainer position={menuPosition} {showUI}>
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
            {available_luts}
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

                await invoke('update_simulation_setting', {
                  settingName: settingName,
                  value: value,
                });
                console.log('Backend invoked for', settingName, value);
              } catch (err) {
                console.error(`Failed to update ${e.detail.setting}:`, err);
                // On error, sync from backend to restore correct state
                await syncSettingsFromBackend();
              }
            }}
          />
        {/if}
      </fieldset>

      <!-- Nutrient Pattern Settings -->
      <fieldset>
        <legend>Nutrient Pattern</legend>
        <div class="control-group">
          <Selector
            options={[
              'Uniform',
              'Checkerboard',
              'Diagonal Gradient',
              'Radial Gradient',
              'Vertical Stripes',
              'Horizontal Stripes',
              'Enhanced Noise',
              'Wave Function',
              'Cosine Grid',
            ]}
            bind:value={settings.nutrient_pattern}
            label="Pattern Type"
            on:change={({ detail }) => updateNutrientPattern(detail.value)}
          />
        </div>
        <div class="control-group">
          <label for="nutrientPatternReversed">Reverse Pattern</label>
          <input
            type="checkbox"
            id="nutrientPatternReversed"
            bind:checked={settings.nutrient_pattern_reversed}
            on:change={(e: Event) => {
              const target = e.target as HTMLInputElement;
              if (target) {
                updateNutrientPatternReversed(target.checked);
              }
            }}
          />
        </div>
      </fieldset>
    </form>
  </SimulationMenuContainer>

    <!-- Save Preset Dialog -->
  {#if show_save_preset_dialog}
    <SavePresetDialog
      bind:presetName={new_preset_name}
      on:save={({ detail }) => savePreset(detail.name)}
      on:close={() => (show_save_preset_dialog = false)}
    />
  {/if}
</div>

<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import LutSelector from './components/shared/LutSelector.svelte';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import GrayScottDiagram from './components/gray-scott/GrayScottDiagram.svelte';
  import SimulationControlBar from './components/shared/SimulationControlBar.svelte';
  import SimulationMenuContainer from './components/shared/SimulationMenuContainer.svelte';
  import Selector from './components/inputs/Selector.svelte';
  import SavePresetDialog from './components/shared/SavePresetDialog.svelte';
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

  // Simple panning state
  const pressedKeys = new Set<string>();

  // Mouse drag state
  let isMouseDown = false;
  let lastMouseButton = -1; // Track which button was pressed for drag consistency

  function handleKeydown(event: KeyboardEvent) {
    // Check if the focused element is an input field
    const activeElement = document.activeElement;
    const isInputFocused = activeElement && (
      activeElement.tagName === 'INPUT' ||
      activeElement.tagName === 'TEXTAREA' ||
      activeElement.tagName === 'SELECT' ||
      (activeElement as HTMLElement).contentEditable === 'true'
    );

    if (event.key === '/') {
      event.preventDefault();
      toggleBackendGui();
    } else if (event.key === 'r' || event.key === 'R') {
      event.preventDefault();
      randomizeSimulation();
    } else {
      // Allow camera controls even when simulation is paused
      // But don't handle camera controls when an input is focused
      const cameraKeys = [
        'w',
        'a',
        's',
        'd',
        'arrowup',
        'arrowdown',
        'arrowleft',
        'arrowright',
        'q',
        'e',
        'c',
      ];
      if (cameraKeys.includes(event.key.toLowerCase()) && !isInputFocused) {
        event.preventDefault();
        pressedKeys.add(event.key.toLowerCase());
      }
    }
  }

  function handleKeyup(event: KeyboardEvent) {
    const cameraKeys = [
      'w',
      'a',
      's',
      'd',
      'arrowup',
      'arrowdown',
      'arrowleft',
      'arrowright',
      'q',
      'e',
      'c',
    ];
    if (cameraKeys.includes(event.key.toLowerCase())) {
      pressedKeys.delete(event.key.toLowerCase());
    }
  }

  // Add animation frame loop for smooth camera movement
  let animationFrameId: number | null = null;

  function updateCamera() {
    // Allow camera movement even when simulation is paused
    const panAmount = 0.1;
    let moved = false;
    let deltaX = 0;
    let deltaY = 0;

    if (pressedKeys.has('w') || pressedKeys.has('arrowup')) {
      deltaY += panAmount;
      moved = true;
    }
    if (pressedKeys.has('s') || pressedKeys.has('arrowdown')) {
      deltaY -= panAmount;
      moved = true;
    }
    if (pressedKeys.has('a') || pressedKeys.has('arrowleft')) {
      deltaX -= panAmount;
      moved = true;
    }
    if (pressedKeys.has('d') || pressedKeys.has('arrowright')) {
      deltaX += panAmount;
      moved = true;
    }

    // Apply combined movement if any keys are pressed
    if (moved) {
      console.log('Panning camera:', { deltaX, deltaY });
      panCamera(deltaX, deltaY);
    }

    if (pressedKeys.has('q')) {
      console.log('Zooming out');
      zoomCamera(-0.2);
      moved = true;
    }
    if (pressedKeys.has('e')) {
      console.log('Zooming in');
      zoomCamera(0.2);
      moved = true;
    }
    if (pressedKeys.has('c') || pressedKeys.has('C')) {
      console.log('Resetting camera');
      resetCamera();
      moved = true;
    }

    // Always schedule the next frame to keep the loop running
    animationFrameId = requestAnimationFrame(updateCamera);
  }

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

  async function zoomCameraToCursor(delta: number, cursorX: number, cursorY: number) {
    try {
      await invoke('zoom_camera_to_cursor', { delta, cursorX, cursorY });
      await fetchCameraState();
    } catch (e) {
      console.error('Failed to zoom camera to cursor:', e);
    }
  }

  async function resetCamera() {
    try {
      await invoke('reset_camera');
      await fetchCameraState();
    } catch (e) {
      console.error('Failed to reset camera:', e);
    }
  }

  // Add missing functions for nutrient pattern updates
  async function updateNutrientPattern(value: string) {
    try {
      await invoke('update_simulation_setting', {
        settingName: 'nutrient_pattern',
        value: value,
      });
    } catch (err) {
      console.error('Failed to update nutrient pattern:', err);
    }
  }

  async function updateNutrientPatternReversed(value: boolean) {
    try {
      await invoke('update_simulation_setting', {
        settingName: 'nutrient_pattern_reversed',
        value: value,
      });
    } catch (err) {
      console.error('Failed to update nutrient pattern reversed:', err);
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

  // Unified mouse event handler
  async function handleMouseEvent(event: MouseEvent | WheelEvent) {
    const isWheelEvent = event instanceof WheelEvent;
    const isMouseEvent = event instanceof MouseEvent;

    // Early return if simulation is not running (except for wheel events which can still zoom)
    if (!running && !isWheelEvent) {
      console.log('Mouse event ignored - simulation not running');
      return;
    }

    // Prevent default for all events
    event.preventDefault();

    // Get cursor position
    const cursorX = event.clientX;
    const cursorY = event.clientY;

    // Convert CSS pixels to physical pixels for backend (camera expects physical pixels)
    const devicePixelRatio = window.devicePixelRatio || 1;
    const physicalCursorX = cursorX * devicePixelRatio;
    const physicalCursorY = cursorY * devicePixelRatio;

    console.log(
      `Mouse event: ${event.type}, running: ${running}, cursor: (${cursorX}, ${cursorY}), physical: (${physicalCursorX}, ${physicalCursorY})`
    );

    // Handle wheel events (zoom) - allow even when paused
    if (isWheelEvent) {
      const wheelEvent = event as WheelEvent;

      // Send cursor position to backend (use physical pixels)
      await sendCursorToBackend(physicalCursorX, physicalCursorY);

      // Normalize wheel delta (make it smaller for smoother zooming)
      const normalizedDelta = wheelEvent.deltaY * 0.01;

      // Zoom towards cursor position
      await zoomCameraToCursor(normalizedDelta, physicalCursorX, physicalCursorY);
    }

    // Handle mouse events
    if (isMouseEvent) {
      const mouseEvent = event as MouseEvent;

      // Send cursor position to backend (use physical pixels)
      await sendCursorToBackend(physicalCursorX, physicalCursorY);

      // Handle mouse down (start of drag) - only when running
      if (mouseEvent.type === 'mousedown' && running) {
        isMouseDown = true;
        lastMouseButton = mouseEvent.button;

        try {
          await invoke('handle_mouse_interaction_screen', {
            screenX: physicalCursorX,
            screenY: physicalCursorY,
            mouseButton: mouseEvent.button,
          });
          console.log(
            `Mouse interaction: button ${mouseEvent.button} at (${physicalCursorX}, ${physicalCursorY})`
          );
        } catch (err) {
          console.error('Failed to handle mouse interaction:', err);
        }
      }

      // Handle mouse move during drag - only when running
      if (mouseEvent.type === 'mousemove' && isMouseDown && running) {
        try {
          await invoke('handle_mouse_interaction_screen', {
            screenX: physicalCursorX,
            screenY: physicalCursorY,
            mouseButton: lastMouseButton,
          });
        } catch (err) {
          console.error('Failed to handle mouse drag interaction:', err);
        }
      }

      // Handle mouse up (end of drag) - always handle
      if (mouseEvent.type === 'mouseup') {
        console.log('Mouse up');
        isMouseDown = false;
        lastMouseButton = -1;
      }

      // Handle mouse leave (end of drag if mouse leaves window) - always handle
      if (mouseEvent.type === 'mouseleave') {
        console.log('Mouse leave');
        isMouseDown = false;
        lastMouseButton = -1;
      }

      // Handle context menu (right click) - always prevent
      if (mouseEvent.type === 'contextmenu') {
        console.log('Context menu prevented');
        // Context menu is already prevented by preventDefault() above
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
    // Add keyboard event listeners
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('keyup', handleKeyup);

    // Add event listeners for auto-hide functionality
    const events = ['mousedown', 'mousemove', 'keydown', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.addEventListener(event, handleUserInteraction, { passive: true });
    });

    // Start camera update loop immediately so camera controls work even when paused
    if (animationFrameId === null) {
      animationFrameId = requestAnimationFrame(updateCamera);
    }

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
    window.removeEventListener('keydown', handleKeydown);
    window.removeEventListener('keyup', handleKeyup);

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
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }

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
