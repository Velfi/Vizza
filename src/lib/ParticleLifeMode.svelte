<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NumberDragBox from './components/NumberDragBox.svelte';
  import LutSelector from './components/LutSelector.svelte';

  const dispatch = createEventDispatcher();

  interface Settings {
    species_count: number;
    particle_count: number;
    force_matrix: number[][];
    max_force: number;
    min_distance: number;
    max_distance: number;
    friction: number;
    time_step: number;
    wrap_edges: boolean;
    random_seed: number;
    repulsion_min_distance: number;
    repulsion_medium_distance: number;
    repulsion_extreme_strength: number;
    repulsion_linear_strength: number;
  }

  // Simulation state
  let settings: Settings = {
    species_count: 4,
    particle_count: 20000,
    force_matrix: [
      [-0.2,  0.3, -0.1,  0.1],
      [-0.3, -0.1,  0.4, -0.2],
      [ 0.2, -0.4,  0.1,  0.3],
      [-0.1,  0.2, -0.3, -0.2]
    ],
    max_force: 100.0,
    min_distance: 5.0,
    max_distance: 100.0,
    friction: 0.95,
    time_step: 0.016,
    wrap_edges: true,
    random_seed: 0,
    repulsion_min_distance: 0.1,
    repulsion_medium_distance: 0.5,
    repulsion_extreme_strength: 1000.0,
    repulsion_linear_strength: 200.0
  };

  // Runtime state
  let state = {};

  // UI state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];
  let show_save_preset_dialog = false;
  let new_preset_name = '';
  let fps_display = 0;
  let isSimulationRunning = false;
  let showForceMatrix = false;

  // Species colors for UI visualization
  const speciesColors = [
    '#ff3333', // Red
    '#33ff33', // Green  
    '#3333ff', // Blue
    '#ffff33', // Yellow
    '#ff33ff', // Magenta
    '#33ffff', // Cyan
    '#ff9933', // Orange
    '#9933ff'  // Purple
  ];

  // Event listeners
  let unsubscribeFps: (() => void) | null = null;

  // Two-way binding handlers
  async function updateSpeciesCount(value: number) {
    const newCount = Math.max(2, Math.min(8, Math.round(value)));
    if (newCount === settings.species_count) return;
    
    // Ensure force matrix exists
    if (!settings.force_matrix || !Array.isArray(settings.force_matrix)) {
      settings.force_matrix = Array(settings.species_count || 4).fill(null).map(() => Array(settings.species_count || 4).fill(0.0));
    }
    
    // Resize force matrix to match new species count
    const oldMatrix = settings.force_matrix;
    const newMatrix: number[][] = [];
    
    for (let i = 0; i < newCount; i++) {
      newMatrix[i] = [];
      for (let j = 0; j < newCount; j++) {
        if (i < oldMatrix.length && oldMatrix[i] && j < oldMatrix[i].length && oldMatrix[i][j] !== undefined) {
          newMatrix[i][j] = oldMatrix[i][j];
        } else {
          // Random values for new entries
          newMatrix[i][j] = (Math.random() - 0.5) * 0.6;
        }
      }
    }
    
    settings.species_count = newCount;
    settings.force_matrix = newMatrix;
    
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'species_count', 
        value: newCount 
      });
    } catch (e) {
      console.error('Failed to update species count:', e);
    }
  }

  async function updateForceMatrix(speciesA: number, speciesB: number, value: number) {
    // Ensure force matrix exists and has proper dimensions
    if (!settings.force_matrix || !settings.force_matrix[speciesA] || settings.force_matrix[speciesA][speciesB] === undefined) {
      console.warn('Force matrix not properly initialized, skipping update');
      return;
    }
    
    settings.force_matrix[speciesA][speciesB] = Math.max(-1, Math.min(1, value));
    
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'force_matrix', 
        value: settings.force_matrix 
      });
    } catch (e) {
      console.error('Failed to update force matrix:', e);
    }
  }

  async function updateSetting(settingName: string, value: any) {
    try {
      await invoke('update_simulation_setting', { settingName, value });
    } catch (e) {
      console.error(`Failed to update ${settingName}:`, e);
    }
  }

  async function updateParticleCount(value: number) {
    const newCount = Math.max(1000, Math.min(100000, Math.round(value)));
    if (newCount === settings.particle_count) return;
    
    settings.particle_count = newCount;
    
    try {
      // Use the new dynamic particle count update
      await invoke('update_simulation_setting', { settingName: 'particle_count', value: newCount });
      console.log(`Particle count updated to ${newCount}`);
    } catch (e) {
      console.error('Failed to update particle count:', e);
    }
  }

  // Preset management
  async function updatePreset(value: string) {
    current_preset = value;
    try {
      await invoke('apply_preset', { presetName: value });
      await syncSettingsFromBackend();
      console.log(`Applied preset: ${value}`);
    } catch (e) {
      console.error('Failed to apply preset:', e);
    }
  }

  async function savePreset() {
    if (new_preset_name.trim() === '') return;
    
    try {
      await invoke('save_preset', {
        presetName: new_preset_name.trim(),
        settings: settings
      });
      
      // Refresh presets list
      await loadPresets();
      
      // Clear dialog
      new_preset_name = '';
      show_save_preset_dialog = false;
      
      console.log(`Saved preset: ${new_preset_name}`);
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  async function deletePreset() {
    if (current_preset === '') return;
    
    try {
      await invoke('delete_preset', { presetName: current_preset });
      
      // Refresh presets list
      await loadPresets();
      current_preset = '';
      
      console.log(`Deleted preset`);
    } catch (e) {
      console.error('Failed to delete preset:', e);
    }
  }

  // Data loading functions
  async function loadPresets() {
    try {
      available_presets = await invoke('get_presets_for_simulation_type', { 
        simulationType: 'particle_life' 
      });
    } catch (e) {
      console.error('Failed to load presets:', e);
      available_presets = [];
    }
  }

  async function loadLuts() {
    try {
      available_luts = await invoke('get_available_luts');
    } catch (e) {
      console.error('Failed to load LUTs:', e);
      available_luts = [];
    }
  }

  async function syncSettingsFromBackend() {
    try {
      const backendSettings = await invoke('get_current_settings');
      if (backendSettings) {
        settings = { ...settings, ...backendSettings };
        
        // Ensure force matrix is properly initialized
        if (!settings.force_matrix || !Array.isArray(settings.force_matrix)) {
          // Initialize with default matrix if missing
          const count = settings.species_count || 4;
          settings.force_matrix = Array(count).fill(null).map(() => Array(count).fill(0.0));
        }
        
        // Ensure matrix dimensions match species count
        const currentSize = settings.force_matrix.length;
        const targetSize = settings.species_count || 4;
        
        if (currentSize !== targetSize) {
          // Resize matrix to match species count
          const newMatrix = Array(targetSize).fill(null).map((_, i) => 
            Array(targetSize).fill(null).map((_, j) => {
              if (i < currentSize && j < currentSize && settings.force_matrix[i] && settings.force_matrix[i][j] !== undefined) {
                return settings.force_matrix[i][j];
              }
              return (Math.random() - 0.5) * 0.6; // Random default value
            })
          );
          settings.force_matrix = newMatrix;
        }
      }
      
      const backendState = await invoke('get_current_state');
      if (backendState) {
        state = { ...state, ...backendState };
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  // Simulation control
  async function startSimulation() {
    try {
      await invoke('start_particle_life_simulation');
      isSimulationRunning = true;
      console.log('Particle Life simulation started');
    } catch (e) {
      console.error('Failed to start simulation:', e);
    }
  }

  async function stopSimulation() {
    try {
      await invoke('destroy_simulation');
      isSimulationRunning = false;
      console.log('Simulation stopped');
    } catch (e) {
      console.error('Failed to stop simulation:', e);
    }
  }

  async function resetSimulation() {
    try {
      await invoke('reset_simulation');
      console.log('Simulation reset');
    } catch (e) {
      console.error('Failed to reset simulation:', e);
    }
  }

  async function randomizeSettings() {
    try {
      // Store current species count and particle count before randomizing
      const currentSpeciesCount = settings.species_count;
      const currentParticleCount = settings.particle_count;
      
      await invoke('randomize_settings');
      await syncSettingsFromBackend();
      
      // Ensure species count and particle count are preserved after sync
      if (settings.species_count !== currentSpeciesCount) {
        settings.species_count = currentSpeciesCount;
        await updateSetting('species_count', currentSpeciesCount);
      }
      
      if (settings.particle_count !== currentParticleCount) {
        settings.particle_count = currentParticleCount;
        await updateSetting('particle_count', currentParticleCount);
      }
      
      console.log('Settings randomized');
    } catch (e) {
      console.error('Failed to randomize settings:', e);
    }
  }

  // Camera controls
  let pressedKeys = new Set<string>();
  let animationFrameId: number | null = null;

  function handleKeyDown(event: KeyboardEvent) {
    pressedKeys.add(event.key);
    
    if (animationFrameId === null) {
      animationFrameId = requestAnimationFrame(processCameraMovement);
    }
  }

  function handleKeyUp(event: KeyboardEvent) {
    pressedKeys.delete(event.key);
    
    if (pressedKeys.size === 0 && animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
      
      invoke('stop_camera_pan').catch(e => 
        console.error('Failed to stop camera pan:', e)
      );
    }
  }

  async function processCameraMovement() {
    let deltaX = 0;
    let deltaY = 0;
    let zoomDelta = 0;
    
    if (pressedKeys.has('ArrowLeft') || pressedKeys.has('a') || pressedKeys.has('A')) deltaX -= 1;
    if (pressedKeys.has('ArrowRight') || pressedKeys.has('d') || pressedKeys.has('D')) deltaX += 1;
    if (pressedKeys.has('ArrowUp') || pressedKeys.has('w') || pressedKeys.has('W')) deltaY -= 1;
    if (pressedKeys.has('ArrowDown') || pressedKeys.has('s') || pressedKeys.has('S')) deltaY += 1;
    
    // Q/E for zoom in/out
    if (pressedKeys.has('q') || pressedKeys.has('Q')) zoomDelta -= 1;
    if (pressedKeys.has('e') || pressedKeys.has('E')) zoomDelta += 1;
    
    if (deltaX !== 0 || deltaY !== 0) {
      try {
        // Increase pan speed and fix y-inversion
        await invoke('pan_camera', { deltaX: deltaX * 0.1, deltaY: -deltaY * 0.1 });
      } catch (e) {
        console.error('Failed to pan camera:', e);
      }
    }
    
    if (zoomDelta !== 0) {
      try {
        await invoke('zoom_camera', { delta: zoomDelta * 0.05 });
      } catch (e) {
        console.error('Failed to zoom camera:', e);
      }
    }
    
    if (pressedKeys.size > 0) {
      animationFrameId = requestAnimationFrame(processCameraMovement);
    } else {
      animationFrameId = null;
    }
  }

  function handleMouseEvent(event: MouseEvent | WheelEvent) {
    if (event.type === 'wheel') {
      const wheelEvent = event as WheelEvent;
      wheelEvent.preventDefault();
      
      const zoomDelta = -wheelEvent.deltaY * 0.001;
      
      invoke('zoom_camera_to_cursor', {
        delta: zoomDelta,
        cursorX: wheelEvent.clientX,
        cursorY: wheelEvent.clientY
      }).catch(e => console.error('Failed to zoom camera:', e));
    }
  }

  async function resetCamera() {
    try {
      await invoke('reset_camera');
    } catch (e) {
      console.error('Failed to reset camera:', e);
    }
  }

  // Lifecycle
  onMount(async () => {
    // Start simulation automatically
    await startSimulation();
    
    // Load initial data
    await Promise.all([
      loadPresets(),
      loadLuts(),
      syncSettingsFromBackend()
    ]);
    
    // Set up FPS monitoring
    try {
      unsubscribeFps = await listen('fps-update', (event) => {
        fps_display = event.payload as number;
      });
    } catch (e) {
      console.error('Failed to set up FPS listener:', e);
    }
    
    // Set up keyboard listeners for camera control
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('keyup', handleKeyUp);
  });

  onDestroy(async () => {
    // Stop simulation
    await stopSimulation();
    
    // Clean up listeners
    if (unsubscribeFps) {
      unsubscribeFps();
    }
    
    document.removeEventListener('keydown', handleKeyDown);
    document.removeEventListener('keyup', handleKeyUp);
    
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
    }
  });

  // Computed values
  $: particleCountFormatted = settings.particle_count.toLocaleString();
</script>

<div class="particle-life-container">
  {#if isSimulationRunning}
    <div class="controls">
      <button class="back-button" on:click={() => dispatch('back')}>
        ← Back to Menu
      </button>
      
      <div class="status">
        <span class="status-indicator running"></span>
        Particle Life Simulation Running
      </div>
    </div>

    <!-- Simulation Controls -->
    <div class="simulation-controls">
      <form on:submit|preventDefault>
        
        <!-- FPS Display & Info -->
        <fieldset>
          <legend>FPS & Info</legend>
          <div class="control-group">
            <span>{particleCountFormatted} particles with {settings.species_count} species at {fps_display} FPS</span>
          </div>
        </fieldset>

        <!-- Presets -->
        <fieldset>
          <legend>Presets</legend>
          <div class="control-group">
            <label for="presetSelector">Current Preset</label>
            <select 
              id="presetSelector"
              bind:value={current_preset} 
              on:change={(e) => updatePreset(e.target.value)}
            >
              <option value="">Select Preset...</option>
              {#each available_presets as preset}
                <option value={preset}>{preset}</option>
              {/each}
            </select>
          </div>
          <div class="control-group preset-actions">
            <button type="button" on:click={() => show_save_preset_dialog = true}>💾 Save Current</button>
            <button type="button" on:click={deletePreset} disabled={current_preset === ''}>🗑 Delete</button>
          </div>
        </fieldset>

        <!-- Display Settings -->
        <fieldset>
          <legend>Display Settings</legend>
          <div class="control-group">
            <label for="lutSelector">Color Scheme</label>
            <LutSelector bind:available_luts />
          </div>
        </fieldset>

        <!-- Camera Controls -->
        <fieldset>
          <legend>Camera Controls</legend>
          <div class="control-group">
            <span>📹 WASD/Arrows: Pan | Q/E or Mouse wheel: Zoom</span>
          </div>
          <div class="control-group">
            <button type="button" on:click={resetCamera}>Reset Camera</button>
          </div>
        </fieldset>

        <!-- Controls -->
        <fieldset>
          <legend>Controls</legend>
          <div class="control-group">
            <button type="button" on:click={resetSimulation}>🔄 Reset</button>
            <button type="button" on:click={randomizeSettings}>🎲 Randomize</button>
          </div>
        </fieldset>

        <!-- Species Settings -->
        <fieldset>
          <legend>Species</legend>
          <div class="control-group">
            <label for="speciesCount">Species Count</label>
            <NumberDragBox
              bind:value={settings.species_count}
              min={2}
              max={8}
              step={1}
              precision={0}
              on:change={(e) => updateSpeciesCount(e.detail)}
            />
          </div>
          <div class="control-group">
            <label for="particleCount">Particle Count</label>
            <NumberDragBox
              bind:value={settings.particle_count}
              min={1000}
              max={100000}
              step={1000}
              precision={0}
              on:change={(e) => updateParticleCount(e.detail)}
            />
          </div>
          <div class="control-group">
            <button 
              type="button"
              class="toggle-button" 
              class:active={showForceMatrix}
              on:click={() => showForceMatrix = !showForceMatrix}
            >
              {showForceMatrix ? 'Hide' : 'Show'} Force Matrix
            </button>
          </div>
        </fieldset>

        <!-- Force Matrix Editor -->
        {#if showForceMatrix}
          <fieldset class="force-matrix-section">
            <legend>Force Matrix</legend>
            <div class="force-matrix" style="--species-count: {settings.species_count}">
              <div class="matrix-labels">
                <div class="corner"></div>
                {#each Array(settings.species_count) as _, j}
                  <div class="col-label" style="color: {speciesColors[j]}">
                    S{j + 1}
                  </div>
                {/each}
              </div>
              
              {#each Array(settings.species_count) as _, i}
                <div class="matrix-row">
                  <div class="row-label" style="color: {speciesColors[i]}">
                    S{i + 1}
                  </div>
                  {#each Array(settings.species_count) as _, j}
                    <div class="matrix-cell">
                      {#if settings.force_matrix && settings.force_matrix[i] && settings.force_matrix[i][j] !== undefined}
                        <NumberDragBox
                          bind:value={settings.force_matrix[i][j]}
                          min={-1}
                          max={1}
                          step={0.01}
                          precision={2}
                          showButtons={false}
                          on:change={(e) => updateForceMatrix(i, j, e.detail)}
                        />
                      {:else}
                        <div class="matrix-placeholder">0.00</div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/each}
            </div>
            <div class="matrix-legend">
              <span class="negative">-1.0 = Repulsion</span>
              <span class="neutral">0.0 = Neutral</span>
              <span class="positive">+1.0 = Attraction</span>
            </div>
          </fieldset>
        {/if}

        <!-- Physics Settings -->
        <fieldset>
          <legend>Physics</legend>
          <div class="control-group">
            <label for="maxForce">Max Force</label>
            <NumberDragBox
              bind:value={settings.max_force}
              min={10}
              max={500}
              step={1}
              precision={1}
              on:change={(e) => updateSetting('max_force', e.detail)}
            />
          </div>
          
          <div class="control-group">
            <label for="minDistance">Min Distance</label>
            <NumberDragBox
              bind:value={settings.min_distance}
              min={1}
              max={20}
              step={0.1}
              precision={1}
              on:change={(e) => updateSetting('min_distance', e.detail)}
            />
          </div>
          
          <div class="control-group">
            <label for="maxDistance">Max Distance</label>
            <NumberDragBox
              bind:value={settings.max_distance}
              min={20}
              max={200}
              step={1}
              precision={1}
              on:change={(e) => updateSetting('max_distance', e.detail)}
            />
          </div>
          
          <div class="control-group">
            <label for="friction">Friction</label>
            <NumberDragBox
              bind:value={settings.friction}
              min={0.8}
              max={0.999}
              step={0.001}
              precision={3}
              on:change={(e) => updateSetting('friction', e.detail)}
            />
          </div>
          
          <div class="control-group">
            <label for="timeStep">Time Step</label>
            <NumberDragBox
              bind:value={settings.time_step}
              min={0.005}
              max={0.05}
              step={0.001}
              precision={3}
              on:change={(e) => updateSetting('time_step', e.detail)}
            />
          </div>
          
          <div class="control-group">
            <label>
              <input 
                type="checkbox" 
                bind:checked={settings.wrap_edges}
                on:change={(e) => updateSetting('wrap_edges', e.target.checked)}
              />
              Wrap Edges
            </label>
          </div>
        </fieldset>

        <!-- Repulsion Settings -->
        <fieldset>
          <legend>Particle Repulsion</legend>
          <div class="control-group">
            <label for="repulsionMinDistance">Min Distance</label>
            <NumberDragBox
              bind:value={settings.repulsion_min_distance}
              min={0.01}
              max={1.0}
              step={0.01}
              precision={2}
              on:change={(e) => updateSetting('repulsion_min_distance', e.detail)}
            />
          </div>
          
          <div class="control-group">
            <label for="repulsionMediumDistance">Medium Distance</label>
            <NumberDragBox
              bind:value={settings.repulsion_medium_distance}
              min={0.1}
              max={2.0}
              step={0.01}
              precision={2}
              on:change={(e) => updateSetting('repulsion_medium_distance', e.detail)}
            />
          </div>
          
          <div class="control-group">
            <label for="repulsionExtremeStrength">Extreme Repulsion</label>
            <NumberDragBox
              bind:value={settings.repulsion_extreme_strength}
              min={100}
              max={5000}
              step={50}
              precision={0}
              on:change={(e) => updateSetting('repulsion_extreme_strength', e.detail)}
            />
          </div>
          
          <div class="control-group">
            <label for="repulsionLinearStrength">Linear Repulsion</label>
            <NumberDragBox
              bind:value={settings.repulsion_linear_strength}
              min={50}
              max={1000}
              step={10}
              precision={0}
              on:change={(e) => updateSetting('repulsion_linear_strength', e.detail)}
            />
          </div>
        </fieldset>

      </form>
    </div>
  {/if}

  <!-- Save Preset Dialog -->
  {#if show_save_preset_dialog}
    <div class="dialog-backdrop" on:click={() => show_save_preset_dialog = false}>
      <div class="dialog" on:click|stopPropagation>
        <h3>Save Preset</h3>
        <input
          type="text"
          bind:value={new_preset_name}
          placeholder="Enter preset name..."
          on:keydown={(e) => e.key === 'Enter' && savePreset()}
        />
        <div class="dialog-buttons">
          <button on:click={savePreset} disabled={new_preset_name.trim() === ''}>
            Save
          </button>
          <button on:click={() => show_save_preset_dialog = false}>
            Cancel
          </button>
        </div>
      </div>
    </div>
    
    <!-- Mouse overlay for camera interaction (only when simulation is running) -->
    <div 
      class="mouse-overlay"
      on:wheel={handleMouseEvent}
      role="button"
      tabindex="0"
    ></div>
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
    color: rgba(255, 255, 255, 0.9);
  }

  .control-group {
    margin-bottom: 1rem;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9rem;
  }

  input[type="number"],
  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    color: white;
  }

  select option {
    background: #333;
    color: white;
  }

  input[type="checkbox"] {
    margin-right: 0.5rem;
  }

  button {
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: white;
    padding: 8px 12px;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.2s;
    font-size: 0.9rem;
  }

  button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toggle-button.active {
    background: rgba(100, 200, 255, 0.3);
    border-color: rgba(100, 200, 255, 0.5);
  }

  .preset-actions {
    display: flex;
    gap: 0.5rem;
  }

  .preset-actions button {
    flex: 1;
  }

  /* Force Matrix Styles */
  .force-matrix-section {
    border-color: rgba(100, 200, 255, 0.3);
  }

  .force-matrix {
    display: grid;
    gap: 2px;
    max-width: 100%;
    overflow-x: auto;
  }

  .matrix-labels {
    display: grid;
    grid-template-columns: 40px repeat(var(--species-count, 4), minmax(50px, 1fr));
    gap: 2px;
    margin-bottom: 2px;
    max-width: 100%;
    overflow-x: auto;
  }

  .matrix-row {
    display: grid;
    grid-template-columns: 40px repeat(var(--species-count, 4), minmax(50px, 1fr));
    gap: 2px;
    max-width: 100%;
  }

  .corner {
    width: 40px;
    height: 30px;
  }

  .col-label, .row-label {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.8rem;
    font-weight: bold;
    min-width: 40px;
    height: 30px;
  }

  .matrix-cell {
    min-width: 50px;
    max-width: 80px;
  }
  
  .matrix-placeholder {
    padding: 8px;
    text-align: center;
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.8rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
  }
  
  /* Responsive matrix sizing for higher species counts */
  .force-matrix[style*="--species-count: 5"],
  .force-matrix[style*="--species-count: 6"],
  .force-matrix[style*="--species-count: 7"],
  .force-matrix[style*="--species-count: 8"] {
    font-size: 0.8rem;
  }
  
  .force-matrix[style*="--species-count: 5"] .matrix-cell,
  .force-matrix[style*="--species-count: 6"] .matrix-cell,
  .force-matrix[style*="--species-count: 7"] .matrix-cell,
  .force-matrix[style*="--species-count: 8"] .matrix-cell {
    min-width: 45px;
    max-width: 60px;
  }
  
  .force-matrix[style*="--species-count: 7"] .matrix-cell,
  .force-matrix[style*="--species-count: 8"] .matrix-cell {
    min-width: 40px;
    max-width: 50px;
  }

  .matrix-legend {
    display: flex;
    justify-content: space-between;
    margin-top: 10px;
    font-size: 0.8rem;
  }

  .negative { color: #ff6666; }
  .neutral { color: #cccccc; }
  .positive { color: #66ff66; }

  /* Dialog Styles */
  .dialog-backdrop {
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
    padding: 20px;
    min-width: 300px;
  }

  .dialog h3 {
    margin: 0 0 15px 0;
    color: white;
  }

  .dialog input {
    width: 100%;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: white;
    padding: 8px;
    border-radius: 4px;
    margin-bottom: 15px;
  }

  .dialog input::placeholder {
    color: rgba(255, 255, 255, 0.5);
  }

  .dialog-buttons {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }

  /* Dynamic CSS for force matrix grid */
  .force-matrix {
    --species-count: var(--species-count, 4);
  }

  /* Mouse overlay for camera interaction */
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