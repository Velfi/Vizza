<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NumberDragBox from './NumberDragBox.svelte';
  import LutSelector from './components/LutSelector.svelte';

  const dispatch = createEventDispatcher();

  interface Settings {
    feed_rate: number;
    kill_rate: number;
    diffusion_rate_u: number;
    diffusion_rate_v: number;
    timestep: number;
    nutrient_pattern: string;
    nutrient_pattern_reversed: boolean;
    fps_limit: number;
    fps_limit_enabled: boolean;
    lut_name: string;
    lut_reversed: boolean;
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

    // Display Settings
    fps_limit: 60,
    fps_limit_enabled: false,
    lut_name: '',
    lut_reversed: false
  };

  // Preset and LUT state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

  // Dialog state
  let show_save_preset_dialog = false;
  let new_preset_name = '';

  // Two-way binding handlers
  async function updateFpsLimitEnabled(value: boolean) {
    settings.fps_limit_enabled = value;
    try {
      await invoke('set_fps_limit', { 
        enabled: settings.fps_limit_enabled, 
        limit: settings.fps_limit 
      });
      console.log(`FPS limiting ${value ? 'enabled' : 'disabled'}`);
    } catch (e) {
      console.error('Failed to update FPS limit enabled:', e);
    }
  }

  async function updateLutReversed(value: boolean) {
    settings.lut_reversed = value;
    try {
      await invoke('toggle_lut_reversed');
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

  async function cyclePresetBack() {
    const currentIndex = available_presets.indexOf(current_preset);
    const newIndex = currentIndex > 0 ? currentIndex - 1 : available_presets.length - 1;
    const newPreset = available_presets[newIndex];
    await updatePreset(newPreset);
  }

  async function cyclePresetForward() {
    const currentIndex = available_presets.indexOf(current_preset);
    const newIndex = currentIndex < available_presets.length - 1 ? currentIndex + 1 : 0;
    const newPreset = available_presets[newIndex];
    await updatePreset(newPreset);
  }

  async function savePreset() {
    try {
      await invoke('save_preset', { preset_name: new_preset_name });
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
  async function loadAvailablePresets() {
    try {
      available_presets = await invoke('get_available_presets');
      if (available_presets.length > 0 && !current_preset) {
        current_preset = available_presets.includes('Undulating') ? 'Undulating' : available_presets[0];
        // Apply the initial preset to the simulation
        await invoke('apply_preset', { presetName: current_preset });
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
      const currentState = await invoke('get_current_state') as { lut_name: string; lut_reversed: boolean } | null;
      
      if (currentSettings) {
        // Update the settings object with current backend values
        settings = {
          ...settings,
          ...currentSettings
        };
      }
      
      if (currentState) {
        // Update LUT-related settings from state
        settings.lut_name = currentState.lut_name;
        settings.lut_reversed = currentState.lut_reversed;
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  let simulationInitializedUnlisten: (() => void) | null = null;
  let simulationResumedUnlisten: (() => void) | null = null;
  let fpsUpdateUnlisten: (() => void) | null = null;

  // Simple panning state
  let pressedKeys = new Set<string>();

  // Mouse drag state
  let isMouseDown = false;
  let lastMouseButton = -1; // Track which button was pressed for drag consistency

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === '/') {
      event.preventDefault();
      toggleBackendGui();
    } else if (event.key === 'r' || event.key === 'R') {
      event.preventDefault();
      randomizeSimulation();
    } else if (event.key === 'n' || event.key === 'N') {
      event.preventDefault();
      seedRandomNoise();
    } else {
      // Allow camera controls even when simulation is paused
      const cameraKeys = ['w', 'a', 's', 'd', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright', 'q', 'e', 'c'];
      if (cameraKeys.includes(event.key.toLowerCase())) {
        event.preventDefault();
        pressedKeys.add(event.key.toLowerCase());
      }
    }
  }

  function handleKeyup(event: KeyboardEvent) {
    const cameraKeys = ['w', 'a', 's', 'd', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright', 'q', 'e', 'c'];
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
    if (pressedKeys.has('c')) {
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

  async function seedRandomNoise() {
    try {
      await invoke('seed_random_noise');
      console.log('Random noise seeded via keyboard shortcut');
    } catch (e) {
      console.error('Failed to seed random noise:', e);
    }
  }

  // Add a function to fetch the latest camera state from the backend
  async function fetchCameraState() {
    try {
      const cam = await invoke('get_camera_state') as {
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
        value: value 
      });
    } catch (err) {
      console.error('Failed to update nutrient pattern:', err);
    }
  }

  async function updateNutrientPatternReversed(value: boolean) {
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'nutrient_pattern_reversed', 
        value: value 
      });
    } catch (err) {
      console.error('Failed to update nutrient pattern reversed:', err);
    }
  }

  async function sendCursorToBackend(screenX: number, screenY: number) {
    try {
      await invoke('update_cursor_position_screen', { 
        screenX, 
        screenY 
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

    console.log(`Mouse event: ${event.type}, running: ${running}, cursor: (${cursorX}, ${cursorY}), physical: (${physicalCursorX}, ${physicalCursorY})`);

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
        
        // Start seeding or erasing based on mouse button
        const isSeeding = mouseEvent.button === 0; // Left click = seeding, right click = erasing
        
        try {
          await invoke('handle_mouse_interaction_screen', {
            screenX: physicalCursorX,
            screenY: physicalCursorY,
            isSeeding: isSeeding
          });
          console.log(`Mouse interaction: ${isSeeding ? 'seeding' : 'erasing'} at (${physicalCursorX}, ${physicalCursorY})`);
        } catch (err) {
          console.error('Failed to handle mouse interaction:', err);
        }
      }
      
      // Handle mouse move during drag - only when running
      if (mouseEvent.type === 'mousemove' && isMouseDown && running) {
        const isSeeding = lastMouseButton === 0; // Use the button that started the drag
        
        try {
          await invoke('handle_mouse_interaction_screen', {
            screenX: physicalCursorX,
            screenY: physicalCursorY,
            isSeeding: isSeeding
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
    } catch (err) {
      console.error('Failed to toggle backend GUI:', err);
    }
  }

  function updateLut(name: string) {
    settings.lut_name = name;
    invoke('apply_lut_by_name', { lutName: name });
  }

  onMount(() => {
    // Add keyboard event listeners
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('keyup', handleKeyup);
    
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
      
      // Now that simulation is fully initialized, set running to true
      loading = false;
      running = true;
      console.log('Simulation is now running and ready for mouse interaction');
    }).then(unlisten => {
      simulationInitializedUnlisten = unlisten;
    });

    // Listen for simulation resumed event
    listen('simulation-resumed', async () => {
      console.log('Simulation resumed');
      running = true;
      currentFps = 0;
    }).then(unlisten => {
      simulationResumedUnlisten = unlisten;
    });

    // Listen for FPS updates from backend
    listen('fps-update', (event) => {
      currentFps = event.payload as number;
    }).then(unlisten => {
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

<div class="gray-scott-container">
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

  {#if showUI}
    <div class="controls">
      <button class="back-button" on:click={returnToMenu}>
        ← Back to Menu
      </button>
      
      <div class="status">
        <span class="status-indicator" class:running></span>
        Gray-Scott Simulation {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
      </div>
      
      {#if running && !loading}
        <div class="mouse-instructions">
          <span>🖱️ Left click: Seed reaction | Right click: Erase | Press N: Seed noise</span>
          <span>📹 WASD/Arrows: Pan | Q/E or Mouse wheel: Zoom | C: Reset camera</span>
        </div>
      {/if}
    </div>

    <!-- Simulation Controls -->
    <div class="simulation-controls">
    <form on:submit|preventDefault>
      <!-- 1. FPS Display & Limiter -->
      <fieldset>
        <legend>FPS & Display</legend>
        <div class="control-group">
          <span>Running at {currentFps} FPS</span>
        </div>
        <div class="control-group">
          <label for="fpsLimitEnabled">Enable FPS Limit</label>
          <input 
            type="checkbox" 
            id="fpsLimitEnabled"
            bind:checked={settings.fps_limit_enabled}
            on:change={(e: Event) => {
              const target = e.target as HTMLInputElement;
              if (target) {
                updateFpsLimitEnabled(target.checked);
              }
            }}
          />
        </div>
        {#if settings.fps_limit_enabled}
          <div class="control-group">
            <label for="fpsLimit">FPS Limit</label>
            <NumberDragBox 
              bind:value={settings.fps_limit}
              min={1}
              max={1200}
              step={1}
              precision={0}
              on:change={async (e) => {
                try {
                  await invoke('set_fps_limit', { 
                    enabled: settings.fps_limit_enabled, 
                    limit: e.detail 
                  });
                  console.log(`FPS limit set to: ${e.detail}`);
                } catch (err) {
                  console.error('Failed to update FPS limit:', err);
                }
              }}
            />
          </div>
        {/if}
      </fieldset>

      <!-- 2. Preset Controls -->
      <fieldset>
        <legend>Presets</legend>
        <div class="control-group">
          <div class="preset-controls">
            <button type="button" on:click={cyclePresetBack}>◀</button>
            <select 
              bind:value={current_preset}
              on:change={(e: Event) => updatePreset((e.target as HTMLSelectElement).value)}
            >
              {#each available_presets as preset}
                <option value={preset}>{preset}</option>
              {/each}
            </select>
            <button type="button" on:click={cyclePresetForward}>▶</button>
          </div>
        </div>
        <div class="preset-actions">
          <button type="button" on:click={() => show_save_preset_dialog = true}>
            Save Current Settings
          </button>
        </div>
      </fieldset>

      <!-- 3. LUT Controls -->
      <fieldset>
        <legend>Color Map</legend>
        <div class="control-group">
          <LutSelector
            {available_luts}
            current_lut={settings.lut_name}
            reversed={settings.lut_reversed}
            on:select={({ detail }) => updateLut(detail.name)}
            on:reverse={({ detail }) => updateLutReversed(detail.reversed)}
          />
        </div>
      </fieldset>

      <!-- 4. Simulation Controls -->
      <fieldset>
        <legend>Controls</legend>
        <div class="control-group">
          <button type="button" on:click={resumeSimulation} disabled={running}>▶ Resume</button>
          <button type="button" on:click={stopSimulation} disabled={!running}>⏸ Pause</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('reset_simulation');
              console.log('Simulation reset successfully');
            } catch (e) {
              console.error('Failed to reset simulation:', e);
            }
          }}>Reset Simulation</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('randomize_settings');
              await syncSettingsFromBackend(); // Sync UI with new random settings
              console.log('Settings randomized successfully');
            } catch (e) {
              console.error('Failed to randomize settings:', e);
            }
          }}>🎲 Randomize Settings</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('seed_random_noise');
              console.log('Random noise seeded successfully');
            } catch (e) {
              console.error('Failed to seed random noise:', e);
            }
          }}>🌱 Seed Noise</button>
        </div>
      </fieldset>

      <!-- 5. Reaction-Diffusion Settings -->
      <fieldset>
        <legend>Reaction-Diffusion Settings</legend>
        <div class="control-group">
          <label for="feedRate">Feed Rate</label>
          <NumberDragBox 
            bind:value={settings.feed_rate}
            min={0}
            max={0.1}
            step={0.001}
            precision={3}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'feed_rate', 
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update feed rate:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="killRate">Kill Rate</label>
          <NumberDragBox 
            bind:value={settings.kill_rate}
            min={0}
            max={0.1}
            step={0.001}
            precision={3}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'kill_rate', 
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update kill rate:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="diffusionRateU">Diffusion Rate U</label>
          <NumberDragBox 
            bind:value={settings.diffusion_rate_u}
            min={0}
            max={1}
            step={0.01}
            precision={2}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'diffusion_rate_u', 
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update diffusion rate U:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="diffusionRateV">Diffusion Rate V</label>
          <NumberDragBox 
            bind:value={settings.diffusion_rate_v}
            min={0}
            max={1}
            step={0.01}
            precision={2}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'diffusion_rate_v', 
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update diffusion rate V:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="timestep">Timestep</label>
          <NumberDragBox 
            bind:value={settings.timestep}
            min={0.1}
            max={10}
            step={0.1}
            precision={1}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'timestep', 
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update timestep:', err);
              }
            }}
          />
        </div>
      </fieldset>

      <!-- 6. Nutrient Pattern Settings -->
      <fieldset>
        <legend>Nutrient Pattern</legend>
        <div class="control-group">
          <label for="nutrientPattern">Pattern Type</label>
          <select 
            id="nutrientPattern"
            bind:value={settings.nutrient_pattern}
            on:change={(e: Event) => updateNutrientPattern((e.target as HTMLSelectElement).value)}
          >
            <option value="Uniform">Uniform</option>
            <option value="Checkerboard">Checkerboard</option>
            <option value="Diagonal Gradient">Diagonal Gradient</option>
            <option value="Radial Gradient">Radial Gradient</option>
            <option value="Vertical Stripes">Vertical Stripes</option>
            <option value="Horizontal Stripes">Horizontal Stripes</option>
            <option value="Enhanced Noise">Enhanced Noise</option>
            <option value="Wave Function">Wave Function</option>
            <option value="Cosine Grid">Cosine Grid</option>
          </select>
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
    </div>
  {/if}

  <!-- Save Preset Dialog -->
  {#if show_save_preset_dialog}
    <div class="dialog-overlay">
      <div class="dialog">
        <h3>Save Current Settings</h3>
        <input 
          type="text" 
          bind:value={new_preset_name}
          placeholder="Enter preset name"
        />
        <div class="dialog-actions">
          <button type="button" on:click={savePreset}>Save</button>
          <button type="button" on:click={() => show_save_preset_dialog = false}>Cancel</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .gray-scott-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: transparent;
    position: relative;
  }

  .mouse-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 10;
    pointer-events: auto;
  }

  .controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(10px);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    position: relative;
    z-index: 20;
  }

  .back-button {
    padding: 0.5rem 1rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    font-family: inherit;
    transition: all 0.3s ease;
  }

  .back-button:hover {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.4);
  }

  .status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9rem;
  }

  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ff6b6b;
    transition: background-color 0.3s ease;
  }

  .status-indicator.running {
    background: #51cf66;
  }

  .mouse-instructions {
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.8rem;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .simulation-controls {
    padding: 1rem;
    max-width: 800px;
    margin: 0 auto;
    background: rgba(0, 0, 0, 1.0);
    position: relative;
    z-index: 20;
  }

  fieldset {
    border: 1px solid #ccc;
    border-radius: 4px;
    padding: 1rem;
    margin-bottom: 1rem;
  }

  legend {
    font-weight: bold;
    padding: 0 0.5rem;
  }

  .control-group {
    margin-bottom: 1rem;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
  }

  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
  }

  input[type="checkbox"] {
    margin-right: 0.5rem;
  }

  .preset-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .preset-controls select {
    flex: 1;
  }

  .preset-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: white;
    padding: 2rem;
    border-radius: 8px;
    min-width: 300px;
  }

  .dialog h3 {
    margin-top: 0;
  }

  .dialog input {
    width: 100%;
    margin: 1rem 0;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .loading-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .loading-content {
    text-align: center;
    color: white;
  }

  .loading-spinner {
    width: 50px;
    height: 50px;
    border: 5px solid #f3f3f3;
    border-top: 5px solid #3498db;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style> 