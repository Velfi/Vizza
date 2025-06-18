<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import NumberDragBox from './NumberDragBox.svelte';
  import LutSelector from './components/LutSelector.svelte';

  const dispatch = createEventDispatcher();

  interface Settings {
    // Particle settings
    particle_count: number;
    matrix_size: number;
    wrap_boundaries: boolean;
    friction: number;
    force: number;
    particle_size: number;
    trace_fade: number;
    traces_enabled: boolean;
    dt: number;
    cursor_size: number;
    cursor_strength: number;
    matrix: number[][];

    // Position and type settings
    position_setter: string;
    matrix_generator: string;
    type_setter: string;
    current_preset: string;

    // LUT settings
    lut_name: string;
    lut_reversed: boolean;

    // FPS settings
    fps_limit: number;
    fps_limit_enabled: boolean;
  }

  let settings: Settings = {
    particle_count: 1000,
    matrix_size: 3,
    wrap_boundaries: true,
    friction: 0.1,
    force: 0.1,
    particle_size: 2,
    trace_fade: 0.1,
    traces_enabled: true,
    dt: 1,
    cursor_size: 50,
    cursor_strength: 0.1,
    matrix: Array(3).fill(0).map(() => Array(3).fill(0)),
    position_setter: 'random',
    matrix_generator: 'random',
    type_setter: 'random',
    current_preset: 'random',
    lut_name: 'viridis',
    lut_reversed: false,
    fps_limit: 60,
    fps_limit_enabled: false
  };

  // Constants
  const matrixGenerators = [
    "Random",
    "Symmetry",
    "Chains",
    "Chains 2",
    "Chains 3",
    "Snakes",
    "Zero"
  ];

  const positionSetters = [
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

  const typeSetters = [
    "Random",
    "Randomize 10%",
    "Slices",
    "Onion",
    "Rotate",
    "Flip",
    "More of First",
    "Kill Still"
  ];

  // UI state
  let available_presets: string[] = [];
  let available_luts: string[] = [];
  let running = false;
  let loading = false;
  let currentFps = 0;
  let showUI = true;
  let showAboutWindow = false;
  let pressedKeys = new Set<string>();
  let animationFrameId: number | null = null;

  // Store the current LUT colors for each particle type
  let particleTypeColors: string[] = [];

  // Debounce helper
  function debounce<T extends (...args: any[]) => void>(fn: T, delay: number): T {
    let timeout: ReturnType<typeof setTimeout>;
    return ((...args: Parameters<T>) => {
      clearTimeout(timeout);
      timeout = setTimeout(() => fn(...args), delay);
    }) as T;
  }

  // Send all settings to backend
  const sendSettings = debounce(async () => {
    try {
      await invoke('update_settings', { settings });
    } catch (err) {
      console.error('Failed to update settings:', err);
    }
  }, 200);

  // Watch for settings changes
  $: {
    if (settings) {
      sendSettings();
    }
  }

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

  async function loadAvailablePresets() {
    try {
      available_presets = await invoke('get_available_presets');
      if (available_presets.length > 0 && !settings.current_preset) {
        settings.current_preset = available_presets[0];
      }
    } catch (e) {
      console.error('Failed to load available presets:', e);
    }
  }

  async function loadAvailableLuts() {
    try {
      available_luts = await invoke('get_available_luts');
    } catch (e) {
      console.error('Failed to load available LUTs:', e);
    }
  }

  async function syncSettingsFromBackend() {
    try {
      const currentSettings = await invoke('get_settings');
      if (currentSettings) {
        settings = currentSettings;
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  async function handleMatrixSizeChange(newSize: number) {
    if (newSize < 2 || newSize > 8) return;
    settings.matrix_size = newSize;
    settings.matrix = Array(newSize).fill(0).map(() => Array(newSize).fill(0));
    sendSettings();
  }

  onMount(async () => {
    loading = true;
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('keyup', handleKeyup);
    try {
      await invoke('start_particle_life_simulation');
      running = true;
      await loadAvailablePresets();
      await loadAvailableLuts();
      await syncSettingsFromBackend();
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

  // --- Type Distribution State ---
  let typeCounts: number[] = [];
  let totalParticles = 0;

  async function fetchTypeCounts() {
    const n = settings.matrix_size;
    typeCounts = Array(n).fill(0).map(() => Math.floor(Math.random() * 1000 + 1000));
    totalParticles = typeCounts.reduce((a, b) => a + b, 0);
  }
  onMount(fetchTypeCounts);
  $: settings.matrix_size, fetchTypeCounts();

  // --- LUT Preview State ---
  let lutPreviews: string[] = [];
  async function fetchLutPreviews() {
    lutPreviews = available_luts.map((lut, i) => `https://dummyimage.com/80x20/${(i*1234567%0xffffff).toString(16).padStart(6,'0')}/fff&text=${encodeURIComponent(lut)}`);
  }
  $: available_luts, fetchLutPreviews();

  // Keep global CSS variable in sync for other uses if needed
  $: {
    document.documentElement.style.setProperty('--matrix-size', settings.matrix.length.toString());
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
      await invoke('toggle_gui');
      // Sync UI state with backend
      const isVisible = await invoke<boolean>('get_gui_state');
      showUI = isVisible;
    } catch (err) {
      console.error('Failed to toggle backend GUI:', err);
    }
  }

  async function generateMatrix() {
    try {
      await invoke('generate_matrix', { generator: settings.matrix_generator });
      // Get the actual matrix values from the backend
      const matrixValues = await invoke('get_matrix_values') as number[][];
      if (matrixValues && matrixValues.length > 0) {
        settings.matrix = matrixValues;
      }
      sendSettings();
    } catch (err) {
      console.error('Failed to generate matrix:', err);
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

  // Fetch the current LUT colors from the backend
  async function fetchParticleTypeColors() {
    try {
      const colors: number[][] = await invoke('get_particle_type_colors');
      particleTypeColors = colors.map(([r, g, b]) => `rgb(${r},${g},${b})`);
    } catch (err) {
      // fallback to rainbow if backend fails
      const n = settings.matrix_size;
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

  // Update event handlers to handle null cases
  function handleCheckboxChange(event: Event) {
    const target = event.target as HTMLInputElement;
    if (target?.checked !== undefined) {
      settings.lut_reversed = target.checked;
      sendSettings();
    }
  }

  function handleSelectChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    if (target?.value !== undefined) {
      settings.lut_name = target.value;
      sendSettings();
    }
  }
</script>

<div class="particle-life-container">
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
              on:change={() => sendSettings()}
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
                on:change={() => sendSettings()}
              />
            </div>
          {/if}
        </fieldset>

        <!-- 2. Color Scheme (LUT) -->
        <fieldset>
          <legend>Color Scheme</legend>
          <div class="control-group">
            <label for="lutSelector">Current LUT</label>
            <LutSelector
              available_luts={available_luts}
              current_lut={settings.lut_name}
              reversed={settings.lut_reversed}
              on:select={handleSelectChange}
              on:reverse={handleCheckboxChange}
            />
          </div>
        </fieldset>

        <!-- 3. Physics Settings -->
        <fieldset>
          <legend>Physics Settings</legend>
          <div class="control-group">
            <label for="particleCount">Particle Count</label>
            <NumberDragBox 
              bind:value={settings.particle_count}
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
              bind:value={settings.force}
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
              bind:value={settings.friction}
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
              bind:checked={settings.wrap_boundaries}
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
          <div class="matrix-editor-grid" style="--matrix-size: {settings.matrix.length}">
            <!-- Top-left empty cell -->
            <div class="matrix-corner"></div>
            <!-- Top color bar -->
            {#each Array(settings.matrix.length) as _, j}
              <div class="matrix-border-cell" style="background-color: {getParticleColor(j)}">
                <span class="matrix-border-label">{j + 1}</span>
              </div>
            {/each}
            {#each Array(settings.matrix.length) as _, i}
              <!-- Left color bar -->
              <div class="matrix-border-cell" style="background-color: {getParticleColor(i)}">
                <span class="matrix-border-label">{i + 1}</span>
              </div>
              {#each Array(settings.matrix.length) as _, j}
                <div class="matrix-cell">
                  <NumberDragBox 
                    bind:value={settings.matrix[i][j]}
                    min={-1}
                    max={1}
                    step={0.01}
                    precision={2}
                    showButtons={false}
                    on:change={() => sendSettings()}
                  />
                </div>
              {/each}
            {/each}
          </div>

          <div class="matrix-controls">
            <div class="control-group">
              <label for="matrixSize">Matrix Size</label>
              <NumberDragBox 
                bind:value={settings.matrix_size}
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
                    await handleMatrixSizeChange(value);
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
                bind:value={settings.matrix_generator}
                on:change={() => sendSettings()}
              >
                {#each matrixGenerators as generator}
                  <option value={generator}>{generator}</option>
                {/each}
              </select>
            </div>

            <div class="matrix-buttons">
              <button type="button" on:click={() => {/* placeholder for generateMatrix */}}>Generate Matrix</button>
              <button type="button" on:click={() => { settings.matrix = Array(settings.matrix_size).fill(0).map(() => Array(settings.matrix_size).fill(0)); sendSettings(); }}>Zero Matrix</button>
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
              bind:value={settings.position_setter}
              on:change={() => sendSettings()}
            >
              {#each positionSetters as setter}
                <option value={setter}>{setter}</option>
              {/each}
            </select>
            <button type="button" on:click={() => {/* placeholder for resetPositions */}}>Reset Positions</button>
          </div>

          <div class="control-group">
            <label for="typeSetter">Type Mode</label>
            <select 
              id="typeSetter"
              bind:value={settings.type_setter}
              on:change={() => sendSettings()}
            >
              {#each typeSetters as setter}
                <option value={setter}>{setter}</option>
              {/each}
            </select>
            <button type="button" on:click={() => {/* placeholder for resetTypes */}}>Reset Types</button>
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
              bind:value={settings.cursor_size}
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
              bind:value={settings.cursor_strength}
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

    <!-- Type Distribution Bar -->
    {#if typeCounts.length > 0}
      <div class="type-distribution-bar">
        <div class="type-bar-label">Type Distribution</div>
        <div class="type-bar">
          {#each typeCounts as count, idx}
            <div
              class="type-bar-segment"
              style="width: {totalParticles > 0 ? (count / totalParticles) * 100 : 0}%; background: {getParticleColor(idx)}"
              title="Type {idx + 1}: {count} particles ({totalParticles > 0 ? ((count / totalParticles) * 100).toFixed(1) : 0}%)"
            ></div>
          {/each}
        </div>
        <div class="type-bar-total">Total: {totalParticles}</div>
      </div>
    {/if}

    <!-- LUT Previews -->
    {#if lutPreviews.length > 0}
      <div class="lut-preview-bar">
        <div class="lut-preview-label">LUT Previews</div>
        <div class="lut-preview-list">
          {#each lutPreviews as preview, i}
            <div class="lut-preview-item {settings.lut_index === i ? 'selected' : ''}" on:click={() => { settings.lut_index = i; sendSettings(); }}>
              <img src={preview} alt={available_luts[i]} />
              <div class="lut-preview-name">{available_luts[i]}</div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
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

  .type-distribution-bar {
    margin: 1rem 0;
    padding: 0.5rem;
    background: rgba(255,255,255,0.05);
    border-radius: 4px;
  }
  .type-bar-label {
    font-size: 0.95rem;
    margin-bottom: 0.25rem;
    color: #fff;
  }
  .type-bar {
    display: flex;
    height: 18px;
    border-radius: 4px;
    overflow: hidden;
    background: #222;
    margin-bottom: 0.25rem;
  }
  .type-bar-segment {
    height: 100%;
    transition: width 0.3s;
  }
  .type-bar-total {
    font-size: 0.85rem;
    color: #ccc;
  }
  .lut-preview-bar {
    margin: 1rem 0;
    padding: 0.5rem;
    background: rgba(255,255,255,0.05);
    border-radius: 4px;
  }
  .lut-preview-label {
    font-size: 0.95rem;
    margin-bottom: 0.25rem;
    color: #fff;
  }
  .lut-preview-list {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .lut-preview-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    cursor: pointer;
    border: 2px solid transparent;
    border-radius: 4px;
    padding: 0.25rem;
    background: #111;
    transition: border 0.2s;
  }
  .lut-preview-item.selected {
    border: 2px solid #51cf66;
    background: #222;
  }
  .lut-preview-item img {
    width: 80px;
    height: 20px;
    object-fit: cover;
    border-radius: 2px;
  }
  .lut-preview-name {
    font-size: 0.8rem;
    color: #fff;
    margin-top: 0.15rem;
    text-align: center;
  }
</style>
