<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NumberDragBox from './components/NumberDragBox.svelte';
  import LutSelector from './components/LutSelector.svelte';
  import UiHiddenIndicator from './components/UiHiddenIndicator.svelte';
  import './shared-theme.css';

  const dispatch = createEventDispatcher();

  // Simulation state
  let settings = {
    // Core Algorithm Settings
    attraction_distance: 50.0,
    kill_distance: 15.0,
    segment_length: 5.0,
    max_attractors: 1000,
    max_nodes: 10000,
    open_venation: true,
    
    // Visual Settings
    enable_vein_thickening: true,
    min_thickness: 1.0,
    max_thickness: 8.0,
    enable_opacity_blending: true,
    min_opacity: 0.3,
    max_opacity: 1.0,
    
    // Growth Settings
    growth_speed: 1.0,
    random_seed: 0,
    attractor_pattern: 'Leaf',
    bounding_shape: 'None',
    
    // Interaction Settings
    interactive_attractors: true,
    mouse_attractor_size: 30.0,
    mouse_attractor_density: 20,
  };

  // LUT state (runtime, not saved in presets)
  let lut_name = 'MATPLOTLIB_viridis';
  let lut_reversed = false;

  // Preset state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

  // Dialog state
  let show_save_preset_dialog = false;
  let new_preset_name = '';

  // Camera control state
  let pressedKeys = new Set<string>();
  let animationFrameId: number | null = null;

  let running = false;
  let loading = false;
  
  // FPS tracking (received from backend)
  let currentFps = 0;
  
  // UI visibility toggle
  let showUI = true;

  async function startSimulation() {
    if (running || loading) return;
    
    loading = true;

    try {
      await invoke('start_space_colonization_simulation');
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
      available_presets = await invoke('get_presets_for_simulation_type', { simulationType: 'space_colonization' });
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
      const currentState = await invoke('get_current_state') as { current_lut_name: string; lut_reversed: boolean } | null;
      
      console.log('Syncing settings from backend:', { currentSettings, currentState });
      
      if (currentSettings) {
        // Update the settings object with current backend values
        settings = {
          ...settings,
          ...currentSettings
        };
        
        console.log('Settings synced from backend:', settings);
      }
      
      if (currentState) {
        // Update LUT-related settings from state
        lut_name = currentState.current_lut_name;
        lut_reversed = currentState.lut_reversed;
        
        console.log('State synced from backend:', { 
          lut_name: lut_name, 
          lut_reversed: lut_reversed
        });
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
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
      await invoke('save_preset', { presetName: new_preset_name });
      show_save_preset_dialog = false;
      new_preset_name = '';
      // Refresh the available presets list
      await loadAvailablePresets();
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  async function updateLutReversed() {
    try {
      await invoke('toggle_lut_reversed');
      await syncSettingsFromBackend(); // Sync UI with backend
    } catch (e) {
      console.error('Failed to toggle LUT reversed:', e);
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
      zoomCamera(0.05);
    }
    if (pressedKeys.has('e')) {
      zoomCamera(-0.05);
    }
    if (pressedKeys.has('c')) {
      resetCamera();
    }

    animationFrameId = requestAnimationFrame(updateCamera);
  }

  // Handle mouse interaction
  function handleMouseEvent(event: MouseEvent) {
    if (event.type === 'wheel') {
      event.preventDefault();
      const delta = (event as WheelEvent).deltaY > 0 ? -0.1 : 0.1;
      zoomCameraToCursor(delta, event.clientX, event.clientY);
    } else if (event.buttons === 1 || event.buttons === 2) { // Left or right mouse button
      event.preventDefault();
      const button = event.buttons === 1 ? 0 : 2; // 0 for left, 2 for right
      handleMouseInteractionScreen(event.clientX, event.clientY, button);
    }
  }

  async function handleMouseInteractionScreen(screenX: number, screenY: number, button: number) {
    try {
      await invoke('handle_mouse_interaction_screen', {
        screenX,
        screenY,
        mouseButton: button
      });
    } catch (e) {
      console.error('Failed to handle mouse interaction:', e);
    }
  }

  // Helper function to update a setting
  async function updateSetting(settingName: string, value: any) {
    try {
      await invoke('update_simulation_setting', { settingName, value });
      console.log(`Updated ${settingName} to ${value}`);
    } catch (e) {
      console.error(`Failed to update ${settingName}:`, e);
    }
  }

  onMount(async () => {
    console.log('SpaceColonizationMode mounted');

    // Start event listeners
    simulationInitializedUnlisten = await listen('simulation-initialized', async () => {
      console.log('Simulation initialized event received');
      loading = false;
      running = true;
      
      // Sync settings from backend after simulation is initialized
      await syncSettingsFromBackend();
    });

    simulationResumedUnlisten = await listen('simulation-resumed', () => {
      console.log('Simulation resumed event received');
      running = true;
    });

    fpsUpdateUnlisten = await listen('fps-update', (event) => {
      const fps = event.payload as number;
      currentFps = Math.round(fps);
    });

    // Set up keyboard event listeners
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('keyup', handleKeyup);

    // Start camera update loop
    updateCamera();

    // Load initial data
    await loadAvailablePresets();
    await loadAvailableLuts();

    // Then start simulation
    startSimulation();
  });

  onDestroy(() => {
    console.log('SpaceColonizationMode destroyed');

    // Clean up event listeners
    if (simulationInitializedUnlisten) simulationInitializedUnlisten();
    if (simulationResumedUnlisten) simulationResumedUnlisten();
    if (fpsUpdateUnlisten) fpsUpdateUnlisten();

    // Clean up keyboard event listeners
    window.removeEventListener('keydown', handleKeydown);
    window.removeEventListener('keyup', handleKeyup);

    // Stop camera update loop
    if (animationFrameId) {
      cancelAnimationFrame(animationFrameId);
    }
  });
</script>

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
        <p>Initializing GPU resources and space colonization algorithm</p>
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
        Space Colonization Simulation {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
      </div>
      
      {#if running && !loading}
        <div class="mouse-instructions">
          <span>🖱️ Left click: Place attractors | Right click: Reset | R: Randomize</span>
          <span>📹 WASD/Arrows: Pan | Q/E or Mouse wheel: Zoom | C: Reset camera</span>
        </div>
      {/if}
    </div>

    <!-- Main UI Controls Panel -->
    <div class="simulation-controls">
    <form on:submit|preventDefault>
      <!-- Status -->
      <fieldset>
        <legend>Status</legend>
        <div class="control-group">
          <span>Running at {currentFps} FPS</span>
        </div>
      </fieldset>

      <!-- Algorithm Settings -->
      <fieldset>
        <legend>Algorithm Settings</legend>
        <div class="control-group">
          <label>Attraction Distance</label>
          <NumberDragBox
            bind:value={settings.attraction_distance}
            min={10}
            max={200}
            step={1}
            on:change={(e) => updateSetting('attraction_distance', e.detail)}
          />
        </div>
        <div class="control-group">
          <label>Kill Distance</label>
          <NumberDragBox
            bind:value={settings.kill_distance}
            min={1}
            max={50}
            step={0.5}
            on:change={(e) => updateSetting('kill_distance', e.detail)}
          />
        </div>
        <div class="control-group">
          <label>Segment Length</label>
          <NumberDragBox
            bind:value={settings.segment_length}
            min={1}
            max={20}
            step={0.5}
            on:change={(e) => updateSetting('segment_length', e.detail)}
          />
        </div>
        <div class="control-group">
          <label>Growth Speed</label>
          <NumberDragBox
            bind:value={settings.growth_speed}
            min={0.1}
            max={5.0}
            step={0.1}
            on:change={(e) => updateSetting('growth_speed', e.detail)}
          />
        </div>
        <div class="control-group">
          <label>
            <input 
              type="checkbox" 
              bind:checked={settings.open_venation}
              on:change={(e) => updateSetting('open_venation', (e.target as HTMLInputElement).checked)}
            />
            Open Venation (tree-like)
          </label>
        </div>
      </fieldset>

      <!-- Visual Settings -->
      <fieldset>
        <legend>Visual Settings</legend>
        <div class="control-group">
          <label>
            <input 
              type="checkbox" 
              bind:checked={settings.enable_vein_thickening}
              on:change={(e) => updateSetting('enable_vein_thickening', (e.target as HTMLInputElement).checked)}
            />
            Enable Vein Thickening
          </label>
        </div>
        {#if settings.enable_vein_thickening}
        <div class="control-group">
          <label>Min Thickness</label>
          <NumberDragBox
            bind:value={settings.min_thickness}
            min={0.1}
            max={10}
            step={0.1}
            on:change={(e) => updateSetting('min_thickness', e.detail)}
          />
        </div>
        <div class="control-group">
          <label>Max Thickness</label>
          <NumberDragBox
            bind:value={settings.max_thickness}
            min={1}
            max={20}
            step={0.5}
            on:change={(e) => updateSetting('max_thickness', e.detail)}
          />
        </div>
        {/if}
        <div class="control-group">
          <label>
            <input 
              type="checkbox" 
              bind:checked={settings.enable_opacity_blending}
              on:change={(e) => updateSetting('enable_opacity_blending', (e.target as HTMLInputElement).checked)}
            />
            Enable Opacity Blending
          </label>
        </div>
      </fieldset>

      <!-- Color Scheme -->
      <fieldset>
        <legend>Color Scheme</legend>
        <div class="control-group">
          <LutSelector
            {available_luts}
            bind:current_lut={lut_name}
            bind:reversed={lut_reversed}
            on:select={async (e) => {
              await invoke('apply_lut_by_name', { lutName: e.detail.name });
              await syncSettingsFromBackend(); // Sync UI with backend
            }}
            on:reverse={updateLutReversed}
          />
        </div>
      </fieldset>

      <!-- Presets -->
      <fieldset>
        <legend>Presets</legend>
        <div class="control-group">
          <div class="preset-controls">
            <button type="button" on:click={cyclePresetBack} title="Previous preset">←</button>
            <select bind:value={current_preset} on:change={(e) => updatePreset((e.target as HTMLSelectElement).value)}>
              {#each available_presets as preset}
                <option value={preset}>{preset}</option>
              {/each}
            </select>
            <button type="button" on:click={cyclePresetForward} title="Next preset">→</button>
          </div>
        </div>
        <div class="control-group">
          <button type="button" on:click={() => show_save_preset_dialog = true}>Save Current Settings as Preset</button>
          <button type="button" on:click={randomizeSimulation}>🎲 Randomize</button>
        </div>
      </fieldset>

      <!-- Simulation Controls -->
      <fieldset>
        <legend>Simulation Controls</legend>
        <div class="control-group">
          {#if !running}
            <button type="button" on:click={startSimulation} disabled={loading}>
              {loading ? 'Starting...' : 'Start Simulation'}
            </button>
          {:else}
            <button type="button" on:click={stopSimulation}>Stop Simulation</button>
          {/if}
          <button type="button" on:click={() => updateSetting('random_seed', Math.floor(Math.random() * 1000000))}>
            🌱 New Seed
          </button>
          <button type="button" on:click={async () => {
            try {
              await invoke('seed_space_colonization_noise');
              await syncSettingsFromBackend();
              console.log('Space colonization noise seeded successfully');
            } catch (e) {
              console.error('Failed to seed space colonization noise:', e);
            }
          }}>
            🌿 Seed Noise
          </button>
        </div>
      </fieldset>
    </form>
    </div>
  {:else}
    <!-- UI Hidden Indicator -->
    <UiHiddenIndicator {showUI} on:toggle={toggleBackendGui} />
  {/if}
</div>

<!-- Save Preset Dialog -->
{#if show_save_preset_dialog}
  <div class="dialog-overlay">
    <div class="dialog">
      <h3>Save Preset</h3>
      <input 
        type="text" 
        placeholder="Enter preset name" 
        bind:value={new_preset_name}
        on:keydown={(e) => e.key === 'Enter' && savePreset()}
      />
      <div class="dialog-buttons">
        <button on:click={savePreset} disabled={!new_preset_name.trim()}>Save</button>
        <button on:click={() => show_save_preset_dialog = false}>Cancel</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .loading-overlay {
    position: absolute;
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
    border: 3px solid rgba(255, 255, 255, 0.3);
    border-top: 3px solid white;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: rgba(0, 0, 0, 0.9);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    padding: 2rem;
    min-width: 300px;
    text-align: center;
  }

  .dialog h3 {
    margin-top: 0;
    color: rgba(255, 255, 255, 0.9);
  }

  .dialog input {
    width: 100%;
    padding: 0.5rem;
    margin: 1rem 0;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.5);
    color: rgba(255, 255, 255, 0.9);
  }

  .dialog-buttons {
    display: flex;
    gap: 1rem;
    justify-content: center;
  }

  .dialog-buttons button {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
  }

  .dialog-buttons button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
  }

  .dialog-buttons button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style> 