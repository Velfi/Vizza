<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NumberDragBox from './NumberDragBox.svelte';
  import AgentCountInput from './AgentCountInput.svelte';
  import LutSelector from './components/LutSelector.svelte';

  const dispatch = createEventDispatcher();

  // Simulation state
  let settings = {
    // Pheromone Settings
    pheromone_decay_rate: 1.0,
    pheromone_deposition_rate: 0.5,
    pheromone_diffusion_rate: 0.5,
    decay_frequency: 1,
    diffusion_frequency: 1,

    // Agent Settings  
    agent_speed_min: 100,
    agent_speed_max: 200,
    agent_turn_rate: 180, // degrees
    agent_jitter: 0.1,
    agent_sensor_angle: 45, // degrees
    agent_sensor_distance: 50,

    // Gradient Settings
    gradient_type: 'disabled',
    gradient_strength: 50,
    gradient_center_x: 0.5,
    gradient_center_y: 0.5,
    gradient_size: 1.0,
    gradient_angle: 0,

    // Display Settings
    fps_limit: 60,
    fps_limit_enabled: false,
    lut_name: 'MATPLOTLIB_bone', // Match backend default
    lut_reversed: true
  };

  // Agent count tracked separately (not part of preset settings)
  let currentAgentCount = 1_000_000;

  // Preset and LUT state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

  // Dialog state
  let show_save_preset_dialog = false;
  let new_preset_name = '';

  // Camera control state
  let pressedKeys = new Set<string>();
  let animationFrameId: number | null = null;

  // Helper function to convert agent count to millions
  const toMillions = (count: number) => count / 1_000_000;
  const fromMillions = (millions: number) => millions * 1_000_000;
  
  // Helper function to format numbers with commas
  const formatNumber = (num: number) => num.toLocaleString();

  // Computed values
  $: agent_count_millions = toMillions(currentAgentCount);
  $: gradient_center_x_percent = settings.gradient_center_x * 100;
  $: gradient_center_y_percent = settings.gradient_center_y * 100;

  // Two-way binding handlers
  async function updateAgentCount(value: number) {
    const newCount = fromMillions(value);
    console.log('Updating agent count: input =', value, 'millions, actual count =', newCount);
    try {
      await invoke('update_agent_count', { count: newCount });
      console.log('Backend update completed, syncing from backend...');
      // Sync the actual agent count from backend
      await syncAgentCountFromBackend();
      console.log('Sync completed, currentAgentCount is now:', currentAgentCount);
    } catch (e) {
      console.error('Failed to update agent count:', e);
    }
  }

  async function updateGradientCenterX(value: number) {
    settings.gradient_center_x = value / 100;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_center_x', 
        value: settings.gradient_center_x 
      });
    } catch (e) {
      console.error('Failed to update gradient center X:', e);
    }
  }

  async function updateGradientCenterY(value: number) {
    settings.gradient_center_y = value / 100;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_center_y', 
        value: settings.gradient_center_y 
      });
    } catch (e) {
      console.error('Failed to update gradient center Y:', e);
    }
  }

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

  async function handleGradientType(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    settings.gradient_type = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'gradient_type', 
        value: settings.gradient_type 
      });
    } catch (err) {
      console.error('Failed to update gradient type:', err);
    }
  }

  async function updatePreset(value: string) {
    current_preset = value;
    try {
      await invoke('apply_preset', { presetName: value });
      await invoke('reset_trails'); // Clear all existing trails
      await syncSettingsFromBackend(); // Sync UI with new settings
      // Reset agents asynchronously to avoid blocking the UI
      invoke('reset_agents').catch(e => console.error('Failed to reset agents:', e));
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
      await invoke('save_preset', { presetName: new_preset_name });
      show_save_preset_dialog = false;
      new_preset_name = '';
      // Refresh the available presets list
      await loadAvailablePresets();
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  let running = false;
  let loading = false;
  
  // FPS tracking (now received from backend)
  let currentFps = 0;
  
  // UI visibility toggle
  let showUI = true;

  async function startSimulation() {
    if (running || loading) return;
    
    loading = true;

    try {
      await invoke('start_slime_mold_simulation');
      loading = false;
      running = true;
      
      // Backend now handles the render loop, we just track state
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
      available_presets = await invoke('get_slime_mold_presets');
      console.log('Available presets loaded:', available_presets);
      if (available_presets.length > 0 && !current_preset) {
        current_preset = available_presets[0];
        console.log('Set initial preset to:', current_preset);
      }
    } catch (e) {
      console.error('Failed to load available presets:', e);
    }
  }

  // Load available LUTs from backend
  async function loadAvailableLuts() {
    try {
      available_luts = await invoke('get_available_luts');
      console.log('Available LUTs loaded:', available_luts.length);
    } catch (e) {
      console.error('Failed to load available LUTs:', e);
    }
  }

  // Sync settings from backend to frontend
  async function syncSettingsFromBackend() {
    try {
      const currentSettings = await invoke('get_current_settings') as any;
      const currentState = await invoke('get_current_state') as { lut_name: string; lut_reversed: boolean } | null;
      
      console.log('Syncing settings from backend:', { currentSettings, currentState });
      
      if (currentSettings) {
        // Handle gradient type conversion from enum to lowercase string
        if (currentSettings.gradient_type) {
          currentSettings.gradient_type = currentSettings.gradient_type.toLowerCase();
        }
        
        // Convert radians to degrees for frontend display
        if (currentSettings.agent_turn_rate !== undefined) {
          currentSettings.agent_turn_rate = (currentSettings.agent_turn_rate * 180) / Math.PI;
        }
        if (currentSettings.agent_sensor_angle !== undefined) {
          currentSettings.agent_sensor_angle = (currentSettings.agent_sensor_angle * 180) / Math.PI;
        }
        
        // Update the settings object with current backend values
        settings = {
          ...settings,
          ...currentSettings
        };
        
        // Update computed values
        gradient_center_x_percent = settings.gradient_center_x * 100;
        gradient_center_y_percent = settings.gradient_center_y * 100;
        
        console.log('Settings synced from backend:', settings);
      }
      
      if (currentState) {
        // Update LUT-related settings from state
        settings.lut_name = currentState.lut_name;
        settings.lut_reversed = currentState.lut_reversed;
        console.log('LUT state synced from backend:', { lut_name: settings.lut_name, lut_reversed: settings.lut_reversed });
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  // Sync agent count separately from settings
  async function syncAgentCountFromBackend() {
    try {
      const agentCount = await invoke('get_current_agent_count');
      console.log('Backend returned agent count:', agentCount);
      if (agentCount !== null && agentCount !== undefined) {
        console.log('Updating currentAgentCount from', currentAgentCount, 'to', agentCount);
        currentAgentCount = agentCount as number;
      }
    } catch (e) {
      console.error('Failed to sync agent count from backend:', e);
    }
  }

  let simulationInitializedUnlisten: (() => void) | null = null;
  let simulationResumedUnlisten: (() => void) | null = null;
  let fpsUpdateUnlisten: (() => void) | null = null;

  // Keyboard event handler
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === '/') {
      event.preventDefault();
      toggleBackendGui();
    } else if (event.key === 'r' || event.key === 'R') {
      event.preventDefault();
      randomizeSimulation();
    } else {
      // Camera controls
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

  async function randomizeSimulation() {
    try {
      await invoke('randomize_settings');
      await syncSettingsFromBackend();
      console.log('Settings randomized via keyboard shortcut');
    } catch (e) {
      console.error('Failed to randomize settings:', e);
    }
  }

  // Camera control functions
  async function panCamera(deltaX: number, deltaY: number) {
    try {
      await invoke('pan_camera', { deltaX, deltaY });
    } catch (e) {
      console.error('Failed to pan camera:', e);
    }
  }

  async function zoomCamera(delta: number) {
    try {
      await invoke('zoom_camera', { delta });
    } catch (e) {
      console.error('Failed to zoom camera:', e);
    }
  }

  async function zoomCameraToCursor(delta: number, cursorX: number, cursorY: number) {
    try {
      await invoke('zoom_camera_to_cursor', { delta, cursorX, cursorY });
    } catch (e) {
      console.error('Failed to zoom camera to cursor:', e);
    }
  }

  async function resetCamera() {
    try {
      await invoke('reset_camera');
    } catch (e) {
      console.error('Failed to reset camera:', e);
    }
  }

  // Camera update loop for smooth movement
  function updateCamera() {
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

    if (moved) {
      panCamera(deltaX, deltaY);
    }

    if (pressedKeys.has('q')) {
      zoomCamera(-0.2);
    }
    if (pressedKeys.has('e')) {
      zoomCamera(0.2);
    }
    if (pressedKeys.has('c')) {
      resetCamera();
    }

    animationFrameId = requestAnimationFrame(updateCamera);
  }

  // Mouse event handler for camera controls and UI interaction
  async function handleMouseEvent(event: MouseEvent | WheelEvent) {
    const isWheelEvent = event instanceof WheelEvent;
    const isMouseEvent = event instanceof MouseEvent;
    
    // Prevent default for all events
    event.preventDefault();
    
    // Get cursor position
    const cursorX = event.clientX;
    const cursorY = event.clientY;
    
    // Convert CSS pixels to physical pixels for backend
    const devicePixelRatio = window.devicePixelRatio || 1;
    const physicalCursorX = cursorX * devicePixelRatio;
    const physicalCursorY = cursorY * devicePixelRatio;
    
    // Handle wheel events (zoom) - allow even when paused
    if (isWheelEvent) {
      const wheelEvent = event as WheelEvent;
      
      // Normalize wheel delta for smoother zooming
      const normalizedDelta = wheelEvent.deltaY * 0.01;
      
      // Zoom towards cursor position
      await zoomCameraToCursor(normalizedDelta, physicalCursorX, physicalCursorY);
    }
    
    // Handle mouse events (currently just for camera, could be extended for slime mold interaction)
    if (isMouseEvent) {
      const mouseEvent = event as MouseEvent;
      
      // Handle mouse down (start of drag) - could be used for slime mold interaction
      if (mouseEvent.type === 'mousedown') {
        // Future: Add slime mold interaction here if needed
        console.log('Mouse down at:', cursorX, cursorY);
      }
      
      // Handle mouse up (end of drag) - always handle
      if (mouseEvent.type === 'mouseup') {
        console.log('Mouse up');
      }
      
      // Handle mouse leave (end of drag if mouse leaves window) - always handle
      if (mouseEvent.type === 'mouseleave') {
        console.log('Mouse leave');
      }
      
      // Handle context menu (right click) - always prevent
      if (mouseEvent.type === 'contextmenu') {
        console.log('Context menu prevented');
      }
    }
  }

  onMount(async () => {
    // Load presets and LUTs first
    await loadAvailablePresets();
    await loadAvailableLuts();
    
    // Add keyboard event listener
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('keyup', handleKeyup);
    
    // Start camera update loop
    animationFrameId = requestAnimationFrame(updateCamera);
    
    // Listen for simulation initialization event
    simulationInitializedUnlisten = await listen('simulation-initialized', async () => {
      console.log('Simulation initialized, syncing settings and agent count...');
      await syncSettingsFromBackend();
      await syncAgentCountFromBackend();
      
      // Apply the initial preset after simulation is initialized
      if (available_presets.length > 0 && current_preset) {
        try {
          await invoke('apply_preset', { presetName: current_preset });
          await syncSettingsFromBackend(); // Sync UI with preset settings
          console.log(`Applied initial preset: ${current_preset}`);
        } catch (e) {
          console.error('Failed to apply initial preset:', e);
        }
      }
    });

    // Listen for simulation resumed event
    simulationResumedUnlisten = await listen('simulation-resumed', async () => {
      console.log('Simulation resumed');
      running = true;
      currentFps = 0;
    });

    // Listen for FPS updates from backend
    fpsUpdateUnlisten = await listen('fps-update', (event) => {
      currentFps = event.payload as number;
    });
    
    // Then start simulation
    startSimulation();
  });

  onDestroy(() => {
    // Remove keyboard event listener
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

  async function updateLutName(value: string) {
    settings.lut_name = value;
    try {
      await invoke('apply_lut_by_name', { lutName: value });
    } catch (e) {
      console.error('Failed to update LUT name:', e);
    }
  }
</script>

<div class="slime-mold-container">
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
        <p>Initializing GPU resources and agents</p>
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
        Slime Mold Simulation {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
      </div>
    </div>

    <!-- Simulation Controls -->
    <div class="simulation-controls">
    <form on:submit|preventDefault>
      <!-- 1. FPS Display & Limiter -->
      <fieldset>
        <legend>FPS & Display</legend>
        <div class="control-group">
          <span>{formatNumber(currentAgentCount)} agents at {currentFps} FPS</span>
        </div>
        <div class="control-group">
          <label for="fpsLimitEnabled">Enable FPS Limit</label>
          <input 
            type="checkbox" 
            id="fpsLimitEnabled"
            bind:checked={settings.fps_limit_enabled}
            on:change={(e: Event) => {
              const value = (e.target as HTMLInputElement).checked;
              updateFpsLimitEnabled(value);
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
          <label for="presetSelector">Current Preset</label>
          <div class="preset-controls">
            <button 
              type="button"
              on:click={cyclePresetBack}
            >
              ◀
            </button>
            <select 
              id="presetSelector"
              bind:value={current_preset}
              on:change={(e: Event) => {
                const value = (e.target as HTMLSelectElement).value;
                updatePreset(value);
              }}
            >
              {#each available_presets as preset}
                <option value={preset}>{preset}</option>
              {/each}
            </select>
            <button 
              type="button"
              on:click={cyclePresetForward}
            >
              ▶
            </button>
          </div>
        </div>
        <div class="control-group preset-actions">
          <button 
            type="button"
            on:click={() => show_save_preset_dialog = true}
          >
            💾 Save Current
          </button>
          <!-- TODO: Implement preset deletion -->
          <!-- <button 
            type="button"
            on:click={deletePreset}
          >
            🗑 Delete
          </button> -->
        </div>
        {#if show_save_preset_dialog}
          <div class="save-preset-dialog">
            <div class="dialog-content">
              <h3>Save Preset</h3>
              <div class="control-group">
                <label for="newPresetName">Preset Name</label>
                <input 
                  type="text" 
                  id="newPresetName"
                  bind:value={new_preset_name}
                />
              </div>
              <div class="dialog-actions">
                <button 
                  type="button"
                  on:click={savePreset}
                >
                  Save
                </button>
                <button 
                  type="button"
                  on:click={() => {
                    show_save_preset_dialog = false;
                    new_preset_name = '';
                  }}
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        {/if}
      </fieldset>

      <!-- Display Settings -->
      <fieldset>
        <legend>Display Settings</legend>
        <div class="control-group">
          <label for="lutSelector">Color Scheme</label>
          <LutSelector
            {available_luts}
            current_lut={settings.lut_name}
            reversed={settings.lut_reversed}
            on:select={({ detail }) => updateLutName(detail.name)}
            on:reverse={({ detail }) => updateLutReversed(detail.reversed)}
          />
        </div>
      </fieldset>

      <!-- Camera Controls -->
      <fieldset>
        <legend>Camera Controls</legend>
        <div class="control-group">
          <span>📹 WASD/Arrows: Pan | Q/E or Mouse wheel: Zoom | C: Reset camera</span>
        </div>
        <div class="control-group">
          <button type="button" on:click={resetCamera}>Reset Camera</button>
        </div>
      </fieldset>

      <!-- 4. Controls (Pause/Resume, Reset Trails, Reset Agents, Randomize) -->
      <fieldset>
        <legend>Controls</legend>
        <div class="control-group">
          <button type="button" on:click={resumeSimulation} disabled={running}>▶ Resume</button>
          <button type="button" on:click={stopSimulation} disabled={!running}>⏸ Pause</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('reset_trails');
              console.log('Trails reset successfully');
            } catch (e) {
              console.error('Failed to reset trails:', e);
            }
          }}>Reset Trails</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('reset_agents');
              await invoke('reset_trails'); // Also reset trails to make agent redistribution visible
              console.log('Agents reset successfully');
            } catch (e) {
              console.error('Failed to reset agents:', e);
            }
          }}>Reset Agents</button>
          <button type="button" on:click={async () => {
            try {
              await invoke('randomize_settings');
              await syncSettingsFromBackend(); // Sync UI with new random settings
              console.log('Settings randomized successfully');
            } catch (e) {
              console.error('Failed to randomize settings:', e);
            }
          }}>🎲 Randomize Settings</button>
        </div>
      </fieldset>

      <!-- 5. Pheromone Settings -->
      <fieldset>
        <legend>Pheromone Settings</legend>
        <div class="control-group">
          <label for="decayRate">Decay Rate (%)</label>
          <NumberDragBox 
            bind:value={settings.pheromone_decay_rate}
            min={0}
            max={10000}
            step={1}
            precision={2}
            unit="%"
            on:change={async (e) => {
              // The bind:value should have already updated the local state
              // Now we just need to call the backend
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'pheromone_decay_rate', 
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update pheromone decay rate:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="depositionRate">Deposition Rate (%)</label>
          <NumberDragBox 
            bind:value={settings.pheromone_deposition_rate}
            min={0}
            max={100}
            step={1}
            precision={2}
            unit="%"
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'pheromone_deposition_rate', 
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update pheromone deposition rate:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="diffusionRate">Diffusion Rate (%)</label>
          <NumberDragBox 
            bind:value={settings.pheromone_diffusion_rate}
            min={0}
            max={100}
            step={1}
            precision={2}
            unit="%"
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'pheromone_diffusion_rate', 
                  value: e.detail 
                });
              } catch (err) {
                console.error('Failed to update pheromone diffusion rate:', err);
              }
            }}
          />
        </div>
      </fieldset>

      <!-- 6. Agent Settings -->
      <fieldset>
        <legend>Agent Settings</legend>
        <div class="control-group">
          <label for="agentCount">Agent Count (millions)</label>
          <AgentCountInput 
            value={agent_count_millions}
            min={0}
            max={100}
            on:update={async (e) => {
              try {
                await updateAgentCount(e.detail);
                console.log(`Agent count updated to ${e.detail} million`);
              } catch (err) {
                console.error('Failed to update agent count:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="minSpeed">Min Speed</label>
          <NumberDragBox 
            bind:value={settings.agent_speed_min}
            min={0}
            max={500}
            step={10}
            precision={1}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'agent_speed_min', 
                  value: e.detail 
                });
                await syncSettingsFromBackend();
              } catch (err) {
                console.error('Failed to update min speed:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="maxSpeed">Max Speed</label>
          <NumberDragBox 
            bind:value={settings.agent_speed_max}
            min={0}
            max={500}
            step={10}
            precision={1}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'agent_speed_max', 
                  value: e.detail 
                });
                await syncSettingsFromBackend();
              } catch (err) {
                console.error('Failed to update max speed:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="turnRate">Turn Rate (degrees)</label>
          <NumberDragBox 
            bind:value={settings.agent_turn_rate}
            min={0}
            max={360}
            step={1}
            precision={0}
            unit="°"
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'agent_turn_rate', 
                  value: (e.detail * Math.PI) / 180 // Convert degrees to radians
                });
                await syncSettingsFromBackend();
              } catch (err) {
                console.error('Failed to update turn rate:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="jitter">Jitter</label>
          <NumberDragBox 
            bind:value={settings.agent_jitter}
            min={0}
            max={5}
            step={0.01}
            precision={2}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'agent_jitter', 
                  value: e.detail 
                });
                await syncSettingsFromBackend();
              } catch (err) {
                console.error('Failed to update agent jitter:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="sensorAngle">Sensor Angle (degrees)</label>
          <NumberDragBox 
            bind:value={settings.agent_sensor_angle}
            min={0}
            max={180}
            step={1}
            precision={0}
            unit="°"
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'agent_sensor_angle', 
                  value: (e.detail * Math.PI) / 180 // Convert degrees to radians
                });
                await syncSettingsFromBackend();
              } catch (err) {
                console.error('Failed to update sensor angle:', err);
              }
            }}
          />
        </div>
        <div class="control-group">
          <label for="sensorDistance">Sensor Distance</label>
          <NumberDragBox 
            bind:value={settings.agent_sensor_distance}
            min={0}
            max={500}
            step={1}
            precision={0}
            on:change={async (e) => {
              try {
                await invoke('update_simulation_setting', { 
                  settingName: 'agent_sensor_distance', 
                  value: e.detail 
                });
                await syncSettingsFromBackend();
              } catch (err) {
                console.error('Failed to update sensor distance:', err);
              }
            }}
          />
        </div>
      </fieldset>

      <!-- 7. Gradient Settings -->
      <fieldset>
        <legend>Gradient Settings</legend>
        <div class="control-group">
          <label for="gradientType">Gradient Type</label>
          <select 
            id="gradientType"
            bind:value={settings.gradient_type}
            on:change={handleGradientType}
          >
            <option value="disabled">Disabled</option>
            <option value="radial">Radial</option>
            <option value="linear">Linear</option>
            <option value="spiral">Spiral</option>
          </select>
        </div>
        {#if settings.gradient_type !== 'disabled'}
          <div class="control-group">
            <label for="gradientCenterX">Center X (%)</label>
            <input 
              type="number" 
              id="gradientCenterX" 
              min="0" 
              max="100" 
              step="1" 
              bind:value={gradient_center_x_percent}
              on:change={(e: Event) => {
                const value = parseFloat((e.target as HTMLInputElement).value);
                updateGradientCenterX(value);
              }}
            />
          </div>
          <div class="control-group">
            <label for="gradientCenterY">Center Y (%)</label>
            <input 
              type="number" 
              id="gradientCenterY" 
              min="0" 
              max="100" 
              step="1" 
              bind:value={gradient_center_y_percent}
              on:change={(e: Event) => {
                const value = parseFloat((e.target as HTMLInputElement).value);
                updateGradientCenterY(value);
              }}
            />
          </div>
          <div class="control-group">
            <label for="gradientSize">Size</label>
            <NumberDragBox 
              bind:value={settings.gradient_size}
              min={0.1}
              max={2}
              step={0.01}
              precision={2}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', { 
                    settingName: 'gradient_size', 
                    value: e.detail 
                  });
                } catch (err) {
                  console.error('Failed to update gradient size:', err);
                }
              }}
            />
          </div>
          <div class="control-group">
            <label for="gradientAngle">Angle (degrees)</label>
            <NumberDragBox 
              bind:value={settings.gradient_angle}
              min={0}
              max={360}
              step={1}
              precision={0}
              unit="°"
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', { 
                    settingName: 'gradient_angle', 
                    value: e.detail 
                  });
                } catch (err) {
                  console.error('Failed to update gradient angle:', err);
                }
              }}
            />
          </div>
        {/if}
      </fieldset>
    </form>
    </div>
  {/if}
</div>

<style>
  .slime-mold-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: transparent;
    position: relative;
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

  input[type="number"],
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

  .save-preset-dialog {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .dialog-content {
    background: white;
    padding: 1rem;
    border-radius: 4px;
    min-width: 300px;
  }

  .dialog-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1rem;
  }

  button {
    padding: 0.5rem 1rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    background: #f8f9fa;
    cursor: pointer;
    height: 35px;
  }

  button:hover {
    background: #e9ecef;
  }

  /* Loading Screen Styles */
  .loading-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: black;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .loading-content {
    text-align: center;
    color: white;
    padding: 2rem;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
  }

  .loading-content h2 {
    margin: 1rem 0 0.5rem 0;
    font-size: 1.5rem;
    color: rgba(255, 255, 255, 0.9);
  }

  .loading-content p {
    margin: 0;
    color: rgba(255, 255, 255, 0.7);
    font-size: 1rem;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid #646cff;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
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
</style>