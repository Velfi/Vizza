<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import NumberDragBox from './NumberDragBox.svelte';

  const dispatch = createEventDispatcher();

  // Simulation state
  let settings = {
    // Display Settings
    fps_limit: 60,
    fps_limit_enabled: false,
    lut_index: 0,
    lut_reversed: false,
    particle_size: 0.5,
    trace_fade: 0.95,
    tile_fade_strength: 0.7,
    traces_enabled: false,
  };

  // Physics settings
  let physicsSettings = {
    particle_count: 20000,
    matrix_size: 4,
    wrap_boundaries: true,
    friction: 0.85,
    force: 1.0,
    dt: 0.016,
    cursor_size: 0.2,
    cursor_strength: 5.0,
  };

  // Matrix settings
  let matrix = Array(4).fill(0).map(() => Array(4).fill(0));
  let matrixGenerators = [
    "Random",
    "Symmetry",
    "Chains",
    "Chains 2",
    "Chains 3",
    "Snakes",
    "Zero"
  ];
  let currentMatrixGenerator = "Random";

  // Position and type setters
  let positionSetters = [
    "Default",
    "Random",
    "Center",
    "Centered Circle",
    "Uniform Circle",
    "Color Battle",
    "Color Wheel",
    "Line",
    "Spiral",
    "Rainbow Ring",
    "Rainbow Spiral"
  ];
  let currentPositionSetter = "Default";

  let typeSetters = [
    "Random",
    "Randomize 10%",
    "Slices",
    "Onion",
    "Rotate",
    "Flip",
    "More of First",
    "Kill Still"
  ];
  let currentTypeSetter = "Random";

  // Preset and LUT state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

  // Dialog state
  let show_save_preset_dialog = false;
  let new_preset_name = '';

  // Simulation state
  let running = false;
  let loading = false;
  let currentFps = 0;
  let showUI = true;
  let showAboutWindow = false;

  // Mouse interaction state
  let mouseButton: string | null = null;
  let pressedKeys = new Set<string>();
  let animationFrameId: number | null = null;

  // Debounce helper
  function debounce(fn, delay) {
    let timeout;
    return (...args) => {
      clearTimeout(timeout);
      timeout = setTimeout(() => fn(...args), delay);
    };
  }

  // Send the whole matrix to the backend
  const sendWholeMatrix = debounce(async () => {
    try {
      await invoke('update_interaction_matrix', { matrix });
    } catch (err) {
      console.error('Failed to update whole matrix:', err);
    }
  }, 200);

  async function destroySimulation() {
    running = false;
    stopRenderLoop();
    
    try {
      await invoke('destroy_simulation');
      currentFps = 0;
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
        current_preset = available_presets[0];
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
      if (currentSettings) {
        settings = {
          ...settings,
          ...currentSettings
        };
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  // Update FPS limit enabled state
  async function updateFpsLimitEnabled(enabled: boolean) {
    try {
      await invoke('set_fps_limit', { 
        enabled, 
        limit: settings.fps_limit 
      });
      settings.fps_limit_enabled = enabled;
    } catch (err) {
      console.error('Failed to update FPS limit enabled state:', err);
    }
  }

  // Update LUT index
  async function updateLutIndex(index: number) {
    try {
      await invoke('apply_lut_by_index', { lutIndex: index });
      settings.lut_index = index;
    } catch (err) {
      console.error('Failed to update LUT index:', err);
    }
  }

  // Update LUT reversed state
  async function updateLutReversed(reversed: boolean) {
    try {
      await invoke('toggle_lut_reversed');
      settings.lut_reversed = reversed;
    } catch (err) {
      console.error('Failed to update LUT reversed state:', err);
    }
  }

  // Update preset
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

  async function cycleLutBack() {
    const newIndex = settings.lut_index > 0 ? settings.lut_index - 1 : available_luts.length - 1;
    await updateLutIndex(newIndex);
  }

  async function cycleLutForward() {
    const newIndex = settings.lut_index < available_luts.length - 1 ? settings.lut_index + 1 : 0;
    await updateLutIndex(newIndex);
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

  // Matrix interaction handlers
  async function updateMatrixValue(i: number, j: number, value: number) {
    try {
      await invoke('update_simulation_setting', {
        settingName: `matrix_${i}_${j}`,
        value: value
      });
      matrix[i][j] = value;
    } catch (err) {
      console.error('Failed to update matrix value:', err);
    }
  }

  async function updateMatrixGenerator(generator: string) {
    try {
      await invoke('update_simulation_setting', {
        settingName: 'matrix_generator',
        value: generator
      });
      currentMatrixGenerator = generator;
    } catch (err) {
      console.error('Failed to update matrix generator:', err);
    }
  }

  async function generateMatrix() {
    try {
      await invoke('generate_matrix', { generator: currentMatrixGenerator });
      // Get the actual matrix values from the backend
      const matrixValues = await invoke('get_matrix_values') as number[][];
      if (matrixValues && matrixValues.length > 0) {
        matrix = matrixValues;
      }
      await syncSettingsFromBackend();
    } catch (err) {
      console.error('Failed to generate matrix:', err);
    }
  }

  async function zeroMatrix() {
    try {
      await invoke('zero_matrix');
      matrix = Array(physicsSettings.matrix_size).fill(0).map(() => Array(physicsSettings.matrix_size).fill(0));
    } catch (err) {
      console.error('Failed to zero matrix:', err);
    }
  }

  // Sync matrix values from backend
  async function syncMatrixFromBackend() {
    try {
      const matrixValues = await invoke('get_matrix_values') as number[][];
      if (matrixValues && matrixValues.length > 0) {
        matrix = matrixValues;
      }
    } catch (err) {
      console.error('Failed to sync matrix from backend:', err);
    }
  }

  // Position and type setter handlers
  async function updatePositionSetter(setter: string) {
    try {
      await invoke('update_simulation_setting', {
        settingName: 'position_setter',
        value: setter
      });
      currentPositionSetter = setter;
    } catch (err) {
      console.error('Failed to update position setter:', err);
    }
  }

  async function updateTypeSetter(setter: string) {
    try {
      await invoke('update_simulation_setting', {
        settingName: 'type_setter',
        value: setter
      });
      currentTypeSetter = setter;
    } catch (err) {
      console.error('Failed to update type setter:', err);
    }
  }

  async function resetPositions() {
    try {
      await invoke('reset_positions');
    } catch (err) {
      console.error('Failed to reset positions:', err);
    }
  }

  async function resetTypes() {
    try {
      await invoke('reset_types');
    } catch (err) {
      console.error('Failed to reset types:', err);
    }
  }

  // Mouse interaction handlers
  function handleMouseDown(event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    // Determine which button was pressed
    let buttonName = '';
    if (event.button === 0) buttonName = 'left';
    else if (event.button === 1) buttonName = 'middle';
    else if (event.button === 2) buttonName = 'right';
    
    // Call the mouse press handler
    invoke('handle_mouse_press', { x, y, button: buttonName });
  }

  function handleMouseMove(event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    // Always update mouse position for cursor tracking
    invoke('handle_mouse_move', { x, y });
  }

  function handleMouseUp(event: MouseEvent) {
    // Determine which button was released
    let buttonName = '';
    if (event.button === 0) buttonName = 'left';
    else if (event.button === 1) buttonName = 'middle';
    else if (event.button === 2) buttonName = 'right';
    
    // Call the mouse release handler
    invoke('handle_mouse_release', { button: buttonName });
  }

  function handleMouseLeave() {
    // Release all mouse buttons when leaving the area
    invoke('handle_mouse_release', { button: 'left' });
    invoke('handle_mouse_release', { button: 'right' });
    invoke('handle_mouse_release', { button: 'middle' });
  }

  function handleContextMenu(event: MouseEvent) {
    // Prevent context menu to allow right-click interaction
    event.preventDefault();
  }

  function handleWheel(event: WheelEvent) {
    event.preventDefault();
    const delta = event.deltaY > 0 ? -0.1 : 0.1;
    invoke('zoom_camera_to_cursor', {
      delta,
      cursorX: event.clientX,
      cursorY: event.clientY
    });
  }

  // Keyboard event handler
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === '/') {
      event.preventDefault();
      toggleBackendGui();
    } else if (event.key === 'r' || event.key === 'R') {
      event.preventDefault();
      generateMatrix();
    } else if (event.key === 'p' || event.key === 'P') {
      event.preventDefault();
      resetPositions();
    } else if (event.key === 'c' || event.key === 'C') {
      event.preventDefault();
      resetTypes();
    } else if (event.key === 'z' || event.key === 'Z') {
      event.preventDefault();
      if (event.shiftKey) {
        invoke('reset_camera', { fit_to_window: true });
      } else {
        invoke('reset_camera', { fit_to_window: false });
      }
    } else {
      // Add movement keys to pressed set
      const key = event.key.toLowerCase();
      if (['w', 'a', 's', 'd', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright'].includes(key)) {
        event.preventDefault();
        pressedKeys.add(key);
      }
    }
  }

  // Add key up handler
  function handleKeyup(event: KeyboardEvent) {
    pressedKeys.delete(event.key.toLowerCase());
  }

  // Add camera update function
  function updateCamera() {
    if (!running) return;

    const panAmount = 0.2;
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
      invoke('pan_camera', { deltaX, deltaY });
    } else if (pressedKeys.size === 0) {
      // Stop camera movement when no keys are pressed
      invoke('stop_camera_pan');
    }

    animationFrameId = requestAnimationFrame(updateCamera);
  }

  async function toggleBackendGui() {
    try {
      await invoke('toggle_particle_life_gui');
      // Sync UI state with backend
      const isVisible = await invoke<boolean>('get_particle_life_gui_state');
      showUI = isVisible;
    } catch (err) {
      console.error('Failed to toggle backend GUI:', err);
    }
  }

  // Initialize and start simulation
  onMount(async () => {
    loading = true;
    
    // Add keyboard event listeners
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('keyup', handleKeyup);
    
    try {
      // Start the simulation
      await invoke('start_particle_life_simulation');
      running = true;
      
      // Load presets and settings
      await loadAvailablePresets();
      await loadAvailableLuts();
      await syncSettingsFromBackend();
      await syncMatrixFromBackend(); // Load initial matrix values
      
      // Sync UI state with backend
      const isVisible = await invoke<boolean>('get_particle_life_gui_state');
      showUI = isVisible;
      
      // Start render loop and camera update loop
      startRenderLoop();
      if (animationFrameId === null) {
        animationFrameId = requestAnimationFrame(updateCamera);
      }
    } catch (e) {
      console.error('Failed to start particle life simulation:', e);
      running = false;
    } finally {
      loading = false;
    }
  });

  // Render loop for the simulation
  let renderLoopId: number | null = null;

  function startRenderLoop() {
    if (renderLoopId !== null) return; // Already running
    
    async function renderFrame() {
      if (!running || renderLoopId === null) return;
      
      try {
        await invoke('render_frame');
        // Update FPS counter (simplified)
        currentFps = 60; // Placeholder - you could implement actual FPS calculation
      } catch (e) {
        console.error('Render frame failed:', e);
      }
      
      if (running && renderLoopId !== null) {
        renderLoopId = requestAnimationFrame(renderFrame);
      }
    }
    
    renderLoopId = requestAnimationFrame(renderFrame);
  }

  function stopRenderLoop() {
    if (renderLoopId !== null) {
      cancelAnimationFrame(renderLoopId);
      renderLoopId = null;
    }
  }

  // Cleanup
  onDestroy(async () => {
    // Remove keyboard event listeners
    window.removeEventListener('keydown', handleKeydown);
    window.removeEventListener('keyup', handleKeyup);
    
    // Cancel animation frame
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
    
    await destroySimulation();
  });

  // Store the current LUT colors for each particle type
  let particleTypeColors: string[] = [];

  // Fetch the current LUT colors from the backend
  async function fetchParticleTypeColors() {
    try {
      const colors: number[][] = await invoke('get_particle_type_colors');
      particleTypeColors = colors.map(([r, g, b]) => `rgb(${r},${g},${b})`);
    } catch (err) {
      // fallback to rainbow if backend fails
      const n = physicsSettings.matrix_size;
      particleTypeColors = Array(n).fill(0).map((_, i) => {
        const hue = (i / Math.max(1, n - 1)) * 360;
        return `hsl(${hue}, 100%, 50%)`;
      });
    }
  }

  // Get the color for a given type index
  function getParticleColor(index: number): string {
    return particleTypeColors[index] || '#888';
  }

  // Refetch colors when LUT changes or matrix size changes
  $: settings.lut_index, fetchParticleTypeColors();
  $: settings.lut_reversed, fetchParticleTypeColors();
  $: physicsSettings.matrix_size, fetchParticleTypeColors();

  // Keep global CSS variable in sync for other uses if needed
  $: {
    document.documentElement.style.setProperty('--matrix-size', matrix.length.toString());
  }
</script>

<div class="particle-life-container">
  <!-- Mouse interaction overlay -->
  <div 
    class="mouse-overlay"
    on:mousedown={handleMouseDown}
    on:mousemove={handleMouseMove}
    on:mouseup={handleMouseUp}
    on:mouseleave={handleMouseLeave}
    on:wheel={handleWheel}
    on:contextmenu={handleContextMenu}
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
        Particle Life Simulation {loading ? 'Loading...' : running ? 'Running' : 'Stopped'}
      </div>
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
              on:change={(e) => {
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
                  } catch (err) {
                    console.error('Failed to update FPS limit:', err);
                  }
                }}
              />
            </div>
          {/if}
        </fieldset>

        <!-- 2. Color Scheme (LUT) -->
        <fieldset>
          <legend>Color Scheme</legend>
          <div class="control-group">
            <label for="lutSelector">Current LUT</label>
            <div class="lut-controls">
              <button 
                type="button"
                on:click={cycleLutBack}
              >
                ◀
              </button>
              <select 
                id="lutSelector"
                bind:value={settings.lut_index}
                on:change={(e) => {
                  const target = e.target as HTMLSelectElement;
                  if (target) {
                    const value = parseInt(target.value);
                    updateLutIndex(value);
                  }
                }}
              >
                {#each available_luts as lut, i}
                  <option value={i}>{lut}</option>
                {/each}
              </select>
              <button 
                type="button"
                on:click={cycleLutForward}
              >
                ▶
              </button>
            </div>
          </div>
          <div class="control-group">
            <label for="lutReversed">Reverse Colors</label>
            <input 
              type="checkbox" 
              id="lutReversed"
              bind:checked={settings.lut_reversed}
              on:change={(e) => {
                const target = e.target as HTMLInputElement;
                if (target) {
                  updateLutReversed(target.checked);
                }
              }}
            />
          </div>
        </fieldset>

        <!-- 3. Physics Settings -->
        <fieldset>
          <legend>Physics Settings</legend>
          <div class="control-group">
            <label for="particleCount">Particle Count</label>
            <NumberDragBox 
              bind:value={physicsSettings.particle_count}
              min={1000}
              max={200000}
              step={1000}
              precision={0}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', { 
                    settingName: 'particle_count', 
                    value: e.detail 
                  });
                } catch (err) {
                  console.error('Failed to update particle count:', err);
                }
              }}
            />
          </div>

          <div class="control-group">
            <label for="force">Force</label>
            <input 
              type="range" 
              id="force"
              bind:value={physicsSettings.force}
              min={0}
              max={5}
              step={0.1}
              on:change={async (e) => {
                const target = e.target as HTMLInputElement;
                if (target) {
                  try {
                    await invoke('update_simulation_setting', { 
                      settingName: 'rmax', 
                      value: parseFloat(target.value)
                    });
                  } catch (err) {
                    console.error('Failed to update force:', err);
                  }
                }
              }}
            />
          </div>

          <div class="control-group">
            <label for="friction">Friction</label>
            <input 
              type="range" 
              id="friction"
              bind:value={physicsSettings.friction}
              min={0}
              max={1}
              step={0.01}
              on:change={async (e) => {
                const target = e.target as HTMLInputElement;
                if (target) {
                  try {
                    await invoke('update_simulation_setting', { 
                      settingName: 'damping', 
                      value: parseFloat(target.value)
                    });
                  } catch (err) {
                    console.error('Failed to update friction:', err);
                  }
                }
              }}
            />
          </div>

          <div class="control-group">
            <label for="wrapBoundaries">Wrap Boundaries</label>
            <input 
              type="checkbox" 
              id="wrapBoundaries"
              bind:checked={physicsSettings.wrap_boundaries}
              on:change={async (e) => {
                const target = e.target as HTMLInputElement;
                if (target) {
                  try {
                    await invoke('update_simulation_setting', { 
                      settingName: 'wrap_boundaries', 
                      value: target.checked
                    });
                  } catch (err) {
                    console.error('Failed to update wrap boundaries:', err);
                  }
                }
              }}
            />
          </div>
        </fieldset>

        <!-- 4. Matrix Editor -->
        <fieldset>
          <legend>Interaction Matrix</legend>
          <div class="matrix-editor-grid" style="--matrix-size: {matrix.length}">
            <!-- Top-left empty cell -->
            <div class="matrix-corner"></div>
            <!-- Top color bar -->
            {#each Array(matrix.length) as _, j}
              <div class="matrix-border-cell" style="background-color: {getParticleColor(j)}">
                <span class="matrix-border-label">{j + 1}</span>
              </div>
            {/each}
            {#each Array(matrix.length) as _, i}
              <!-- Left color bar -->
              <div class="matrix-border-cell" style="background-color: {getParticleColor(i)}">
                <span class="matrix-border-label">{i + 1}</span>
              </div>
              {#each Array(matrix.length) as _, j}
                <div class="matrix-cell">
                  <NumberDragBox 
                    bind:value={matrix[i][j]}
                    min={-1}
                    max={1}
                    step={0.01}
                    precision={2}
                    showButtons={false}
                    on:change={() => sendWholeMatrix()}
                  />
                </div>
              {/each}
            {/each}
          </div>

          <div class="matrix-controls">
            <div class="control-group">
              <label for="matrixSize">Matrix Size</label>
              <NumberDragBox 
                bind:value={physicsSettings.matrix_size}
                min={2}
                max={8}
                step={1}
                precision={0}
                on:change={async (e) => {
                  try {
                    let value = Number(e.detail);
                    if (!Number.isInteger(value) || value < 2 || value > 8) {
                      throw new Error('Matrix size must be an integer between 2 and 8');
                    }
                    await invoke('update_simulation_setting', { 
                      settingName: 'matrix_size', 
                      value 
                    });
                    matrix = Array(value).fill(0).map(() => Array(value).fill(0));
                    await syncMatrixFromBackend(); // Sync actual values from backend
                  } catch (err) {
                    console.error('Failed to update matrix size:', err);
                  }
                }}
              />
            </div>

            <div class="control-group">
              <label for="matrixGenerator">Matrix Generator</label>
              <select 
                id="matrixGenerator"
                bind:value={currentMatrixGenerator}
                on:change={(e) => {
                  const target = e.target as HTMLSelectElement;
                  if (target) {
                    updateMatrixGenerator(target.value);
                  }
                }}
              >
                {#each matrixGenerators as generator}
                  <option value={generator}>{generator}</option>
                {/each}
              </select>
            </div>

            <div class="matrix-buttons">
              <button type="button" on:click={generateMatrix}>Generate Matrix</button>
              <button type="button" on:click={zeroMatrix}>Zero Matrix</button>
            </div>
          </div>
        </fieldset>

        <!-- 5. Particle Setup -->
        <fieldset>
          <legend>Particle Setup</legend>
          <div class="control-group">
            <label for="positionSetter">Position Mode</label>
            <select 
              id="positionSetter"
              bind:value={currentPositionSetter}
              on:change={(e) => {
                const target = e.target as HTMLSelectElement;
                if (target) {
                  updatePositionSetter(target.value);
                }
              }}
            >
              {#each positionSetters as setter}
                <option value={setter}>{setter}</option>
              {/each}
            </select>
            <button type="button" on:click={resetPositions}>Reset Positions</button>
          </div>

          <div class="control-group">
            <label for="typeSetter">Type Mode</label>
            <select 
              id="typeSetter"
              bind:value={currentTypeSetter}
              on:change={(e) => {
                const target = e.target as HTMLSelectElement;
                if (target) {
                  updateTypeSetter(target.value);
                }
              }}
            >
              {#each typeSetters as setter}
                <option value={setter}>{setter}</option>
              {/each}
            </select>
            <button type="button" on:click={resetTypes}>Reset Types</button>
          </div>
        </fieldset>

        <!-- 6. Rendering Settings -->
        <fieldset>
          <legend>Rendering Settings</legend>
          <div class="control-group">
            <label for="particleSize">Particle Size</label>
            <input 
              type="range" 
              id="particleSize"
              bind:value={settings.particle_size}
              min={0.1}
              max={1}
              step={0.01}
              on:change={async (e) => {
                const target = e.target as HTMLInputElement;
                if (target) {
                  try {
                    await invoke('update_simulation_setting', { 
                      settingName: 'particle_size', 
                      value: target.value 
                    });
                  } catch (err) {
                    console.error('Failed to update particle size:', err);
                  }
                }
              }}
            />
          </div>

          <div class="control-group">
            <label for="tracesEnabled">Particle Traces</label>
            <input 
              type="checkbox" 
              id="tracesEnabled"
              bind:checked={settings.traces_enabled}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', { 
                    settingName: 'traces_enabled', 
                    value: e.target.checked 
                  });
                } catch (err) {
                  console.error('Failed to update traces:', err);
                }
              }}
            />
          </div>

          {#if settings.traces_enabled}
            <div class="control-group">
              <label for="traceFade">Trace Fade</label>
              <input 
                type="range" 
                id="traceFade"
                bind:value={settings.trace_fade}
                min={0}
                max={1}
                step={0.01}
                on:change={async (e) => {
                  try {
                    await invoke('update_simulation_setting', { 
                      settingName: 'trace_fade', 
                      value: e.target.value 
                    });
                  } catch (err) {
                    console.error('Failed to update trace fade:', err);
                  }
                }}
              />
            </div>
          {/if}

          <div class="control-group">
            <label for="edgeFade">Edge Fade</label>
            <input 
              type="range" 
              id="edgeFade"
              bind:value={settings.tile_fade_strength}
              min={0}
              max={1}
              step={0.05}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', { 
                    settingName: 'tile_fade_strength', 
                    value: e.target.value 
                  });
                } catch (err) {
                  console.error('Failed to update edge fade:', err);
                }
              }}
            />
          </div>
        </fieldset>

        <!-- 7. Mouse Interaction -->
        <fieldset>
          <legend>Mouse Interaction</legend>
          <div class="control-group">
            <label for="cursorSize">Cursor Size</label>
            <input 
              type="range" 
              id="cursorSize"
              bind:value={physicsSettings.cursor_size}
              min={0.05}
              max={1}
              step={0.05}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', { 
                    settingName: 'cursor_size', 
                    value: e.target.value 
                  });
                } catch (err) {
                  console.error('Failed to update cursor size:', err);
                }
              }}
            />
          </div>

          <div class="control-group">
            <label for="cursorStrength">Cursor Strength</label>
            <input 
              type="range" 
              id="cursorStrength"
              bind:value={physicsSettings.cursor_strength}
              min={0}
              max={20}
              step={0.5}
              on:change={async (e) => {
                try {
                  await invoke('update_simulation_setting', { 
                    settingName: 'cursor_strength', 
                    value: e.target.value 
                  });
                } catch (err) {
                  console.error('Failed to update cursor strength:', err);
                }
              }}
            />
          </div>

          <div class="mouse-instructions">
            <p>Left Click: Repel particles</p>
            <p>Right Click: Attract particles</p>
          </div>
        </fieldset>
      </form>
    </div>
  {/if}

  <!-- About Window -->
  {#if showAboutWindow}
    <div class="dialog-overlay">
      <div class="dialog">
        <h3>About Particle Life Simulator</h3>
        <p>A high-performance particle simulation converted from Java to Rust</p>
        
        <h4>Technology Stack:</h4>
        <ul>
          <li>Rust programming language</li>
          <li>egui for immediate mode GUI</li>
          <li>wgpu for GPU-accelerated rendering</li>
          <li>nalgebra for vector mathematics</li>
        </ul>

        <p>Originally inspired by the Java Particle Life project</p>
        <p>Converted with Claude Code</p>

        <div class="dialog-actions">
          <button type="button" on:click={() => showAboutWindow = false}>Close</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .particle-life-container {
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
    cursor: crosshair;
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

  .preset-controls,
  .lut-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .preset-controls select,
  .lut-controls select {
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

  .matrix-editor-grid {
    display: grid;
    grid-template-columns: repeat(calc(var(--matrix-size) + 1), 56px);
    grid-template-rows: repeat(calc(var(--matrix-size) + 1), 56px);
    gap: 1px;
    background-color: var(--border-color);
    padding: 1px;
    border-radius: 4px;
    justify-content: start;
    align-items: start;
    width: fit-content;
    margin: 0 auto;
  }

  .matrix-corner {
    grid-row: 1;
    grid-column: 1;
    width: 56px;
    height: 56px;
    background: transparent;
  }

  .matrix-border-cell {
    aspect-ratio: 1;
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--bg-color);
    border-radius: 2px;
  }

  .matrix-border-label {
    color: white;
    font-weight: bold;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.5);
  }

  .matrix-cell {
    aspect-ratio: 1;
    width: 56px;
    height: 56px;
    background-color: var(--bg-color);
    border-radius: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .matrix-cell :global(.number-drag-container) {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .matrix-cell :global(.number-drag-box) {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .matrix-controls {
    margin-top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .control-group {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .matrix-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .mouse-instructions {
    margin-top: 1rem;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  .mouse-instructions p {
    margin: 0.25rem 0;
    color: rgba(255, 255, 255, 0.8);
  }

  /* Add more styles as needed */
</style>
