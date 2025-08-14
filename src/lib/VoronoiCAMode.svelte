<SimulationLayout
  simulationName="Voronoi CA"
  {menuPosition}
  {running}
  {loading}
  {showUI}
  {controlsVisible}
  {currentFps}
  showStep={true}
  on:back={() => dispatch('back')}
  on:toggleUI={toggleBackendGui}
  on:pause={stopSimulation}
  on:resume={resumeSimulation}
  on:step={stepSimulation}
  on:navigate={(e) => dispatch('navigate', e.detail)}
  on:userInteraction={handleUserInteraction}
  on:mouseEvent={handleMouseEvent}
>
  <form on:submit|preventDefault>
    <!-- About this simulation -->
    <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
      <p>
        Voronoi Cellular Automata evolves regions based on nearest-seed influence and local rules.
        Cells belong to Voronoi regions that shift as seeds move via run-and-tumble dynamics and
        drift. Parameters control neighborhood size, activity thresholds, and temporal behavior.
      </p>
      <p>
        Experiment with point count, neighborhood radius, and behavioral timings to explore
        tessellations, flowing boundaries, and emergent patterns.
      </p>
    </CollapsibleFieldset>

    <!-- Preset Controls -->
    <PresetFieldset
      availablePresets={available_presets}
      bind:currentPreset={current_preset}
      placeholder="Select preset..."
      on:presetChange={({ detail }) => handlePresetChange(detail.value)}
      on:presetSave={({ detail }) => handlePresetSave(detail.name)}
    />

    <!-- Display Settings -->
    <fieldset>
      <legend>Display Settings</legend>
      <div class="control-group">
        <label for="vcaLutSelector">Color Scheme</label>
        <LutSelector
          bind:available_luts
          current_lut={currentLut}
          reversed={lutReversed}
          on:select={({ detail }) => applyLut(detail.name)}
          on:reverse={() => toggleLutReversed()}
        />
      </div>
      <div class="control-group">
        <label for="vcaColoringMode">Coloring Mode</label>
        <Selector
          options={['Random', 'Density', 'Age', 'Binary']}
          value={coloringMode}
          on:change={({ detail }) => updateColoringMode(detail.value)}
        />
      </div>
      <div class="control-group">
        <label for="vcaBordersEnabled">Borders</label>
        <Selector
          options={['On', 'Off']}
          value={bordersEnabled ? 'On' : 'Off'}
          on:change={({ detail }) => updateBordersEnabled(detail.value === 'On')}
        />
      </div>
      {#if bordersEnabled}
        <div class="control-group">
          <label for="vcaBorderThreshold">Border Threshold</label>
          <NumberDragBox
            value={borderThreshold}
            min={0.0}
            max={1.0}
            step={0.01}
            precision={2}
            on:change={({ detail }) => {
              updateBorderThreshold(detail);
            }}
          />
        </div>
      {/if}
    </fieldset>

    <!-- Post Processing -->
    <PostProcessingMenu simulationType="voronoi_ca" {enabled} />

    <!-- Controls -->
    <fieldset>
      <legend>Controls</legend>
      <div class="interaction-controls-grid">
        <div class="interaction-help">
          <div class="control-group">
            <Button variant="default" on:click={() => dispatch('navigate', 'how-to-play')}>
              📖 Camera Controls
            </Button>
          </div>
          <div class="control-group">
            <span>Camera controls not working? Click the control bar at the top of the screen.</span
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
            sizeMin={0.01}
            sizeMax={1.0}
            sizeStep={0.01}
            strengthMin={0}
            strengthMax={1.0}
            strengthStep={0.01}
            sizePrecision={2}
            strengthPrecision={2}
            on:sizechange={(e) => updateCursorSize(e.detail)}
            on:strengthchange={(e) => updateCursorStrength(e.detail)}
          />
        </div>
      </div>
    </fieldset>

    <!-- Settings -->
    <fieldset>
      <legend>Settings</legend>

      <!-- General Settings -->
      <div class="settings-section">
        <div class="control-group">
          <Button
            variant="warning"
            type="button"
            on:click={async () => {
              try {
                await invoke('reset_simulation');
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
                await syncFromBackend();
              } catch (e) {
                console.error('Failed to randomize settings:', e);
              }
            }}>🎲 Randomize Settings</Button
          >
        </div>
      </div>

      <!-- Voronoi Parameters -->
      <div class="settings-section">
        <h3 class="section-header">Voronoi Parameters</h3>
        <div class="settings-grid">
          <div class="setting-item">
            <span class="setting-label">Point Count:</span>
            <NumberDragBox
              value={pointCount}
              min={10}
              max={5000}
              step={10}
              precision={0}
              on:change={({ detail }) => updatePointCount(detail)}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Neighbor Radius:</span>
            <NumberDragBox
              value={neighborRadius}
              min={4}
              max={200}
              step={1}
              precision={0}
              on:change={({ detail }) => updateNeighborRadius(detail)}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Alive Threshold:</span>
            <NumberDragBox
              value={aliveThreshold}
              min={0}
              max={1}
              step={0.01}
              precision={2}
              on:change={({ detail }) => updateAliveThreshold(detail)}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Run Speed (px/s):</span>
            <NumberDragBox
              value={runSpeed}
              min={0}
              max={300}
              step={1}
              precision={0}
              on:change={({ detail }) => updateRunSpeed(detail)}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Avg Run Time (s):</span>
            <NumberDragBox
              value={avgRunTime}
              min={0.05}
              max={5}
              step={0.05}
              precision={2}
              on:change={({ detail }) => updateAvgRunTime(detail)}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Tumble Time (s):</span>
            <NumberDragBox
              value={tumbleTime}
              min={0}
              max={2}
              step={0.01}
              precision={2}
              on:change={({ detail }) => updateTumbleTime(detail)}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Drift:</span>
            <NumberDragBox
              value={drift}
              min={0}
              max={2}
              step={0.01}
              precision={2}
              on:change={({ detail }) => updateDrift(detail)}
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">Time Scale:</span>
            <NumberDragBox
              value={timeScale}
              min={0}
              max={5}
              step={0.1}
              precision={2}
              on:change={({ detail }) => updateTimeScale(detail)}
            />
          </div>
        </div>
      </div>
    </fieldset>
  </form>
</SimulationLayout>

<CameraControls
  enabled={true}
  on:toggleGui={toggleBackendGui}
  on:togglePause={async () => (running ? await stopSimulation() : await resumeSimulation())}
/>

<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import NumberDragBox from './components/inputs/NumberDragBox.svelte';
  import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
  import LutSelector from './components/shared/LutSelector.svelte';
  import Selector from './components/inputs/Selector.svelte';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import CameraControls from './components/shared/CameraControls.svelte';
  import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
  import PresetFieldset from './components/shared/PresetFieldset.svelte';
  import Button from './components/shared/Button.svelte';

  const dispatch = createEventDispatcher();
  export let menuPosition: string = 'middle';

  // Control bar / UI state
  let running = false;
  let loading = true;
  let showUI = true;
  let controlsVisible = true;
  let currentFps = 0;
  let enabled = false; // Whether post processing should be enabled

  // Auto-hide functionality for controls when UI is hidden
  let hideTimeout: number | null = null;

  // Cursor hiding functionality
  let cursorHidden = false;
  let cursorHideTimeout: number | null = null;

  // Simple settings
  let drift = 0.5;
  let neighborRadius = 60;
  let aliveThreshold = 0.5;
  let runSpeed = 60;
  let avgRunTime = 1.5;
  let tumbleTime = 0.2;
  let timeScale = 1.0;
  let pointCount = 300;
  let borderThreshold = 0.5;

  // LUT + controls
  let available_luts: string[] = [];
  let currentLut = 'MATPLOTLIB_bone';
  let lutReversed = true;
  let coloringMode = 'Density';
  let bordersEnabled = true;
  let cursorSize = 0.2;
  let cursorStrength = 1.0;

  // Presets + UI
  let available_presets: string[] = [];
  let current_preset = '';
  let show_about_section = false;

  let unlistenInitialized: (() => void) | null = null;
  let unlistenFps: (() => void) | null = null;
  let isMousePressed = false;
  let currentMouseButton = 0;

  async function start() {
    try {
      unlistenInitialized = await listen('simulation-initialized', async () => {
        running = true;
        enabled = true; // Enable post processing when simulation starts
        // sync initial GUI visibility
        try {
          showUI = (await invoke('get_gui_state')) as boolean;
        } catch {
          // Ignore error
        }
        // pull initial settings
        try {
          const settings = (await invoke('get_current_settings')) as Record<string, unknown>;
          if (settings && typeof settings.drift === 'number') drift = settings.drift;
          if (settings && typeof settings.neighborRadius === 'number')
            neighborRadius = settings.neighborRadius;
          if (settings && typeof settings.aliveThreshold === 'number')
            aliveThreshold = settings.aliveThreshold;
          if (settings && typeof settings.runSpeed === 'number') runSpeed = settings.runSpeed;
          if (settings && typeof settings.avgRunTime === 'number') avgRunTime = settings.avgRunTime;
          if (settings && typeof settings.tumbleTime === 'number') tumbleTime = settings.tumbleTime;
          if (settings && typeof settings.timeScale === 'number') timeScale = settings.timeScale;
          if (settings && typeof settings.numPoints === 'number') pointCount = settings.numPoints;
          if (settings && typeof settings.currentLutName === 'string')
            currentLut = settings.currentLutName;
          if (settings && typeof settings.lutReversed === 'boolean')
            lutReversed = settings.lutReversed;
          if (settings && typeof settings.coloringMode === 'string')
            coloringMode = settings.coloringMode;
          if (settings && typeof settings.bordersEnabled === 'boolean')
            bordersEnabled = settings.bordersEnabled;
          if (settings && typeof settings.cursor_size === 'number')
            cursorSize = settings.cursor_size;
          if (settings && typeof settings.cursor_strength === 'number')
            cursorStrength = settings.cursor_strength;
          if (settings && typeof settings.borderThreshold === 'number')
            borderThreshold = settings.borderThreshold;
        } catch {
          // Ignore error
        }
        await loadAvailablePresets();
        loading = false;
      });
      unlistenFps = await listen('fps-update', (e: { payload: number }) => {
        currentFps = e.payload;
      });
      await invoke('start_simulation', { simulationType: 'voronoi_ca' });
      // Ensure the compute step runs (backend starts paused by design)
      await invoke('resume_simulation');
      running = true;
      await loadAvailableLuts();
    } catch (e) {
      console.error('Failed to start Voronoi CA:', e);
    }
  }

  async function loadAvailableLuts() {
    try {
      const luts = await invoke('get_available_luts');
      available_luts = luts as string[];
    } catch (e) {
      console.error('Failed to load LUTs:', e);
    }
  }

  async function applyLut(lutName: string) {
    currentLut = lutName;
    try {
      await invoke('apply_lut_by_name', { lutName });
    } catch (e) {
      console.error('Failed to apply LUT:', e);
    }
  }

  async function toggleLutReversed() {
    lutReversed = !lutReversed;
    try {
      await invoke('toggle_lut_reversed');
    } catch (e) {
      console.error('Failed to reverse LUT:', e);
    }
  }

  async function updateColoringMode(value: string) {
    coloringMode = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'coloringMode', value });
    } catch (e) {
      console.error('Failed to update coloring mode:', e);
    }
  }

  async function updateBordersEnabled(value: boolean) {
    bordersEnabled = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'bordersEnabled', value });
    } catch (e) {
      console.error('Failed to update borders setting:', e);
    }
  }

  async function updateBorderThreshold(value: number) {
    borderThreshold = value;
    try {
      const params = { settingName: 'borderThreshold', value };
      await invoke('update_simulation_setting', params);
    } catch (e) {
      console.error('Failed to update border threshold:', e);
      console.error('Error details:', {
        message: String(e),
        stack: e instanceof Error ? e.stack : undefined,
        params: { settingName: 'borderThreshold', value },
      });
    }
  }

  async function updateCursorSize(value: number) {
    cursorSize = value;
    try {
      await invoke('update_cursor_size', { size: value });
    } catch (e) {
      console.error('Failed to update cursor size:', e);
    }
  }

  async function updateCursorStrength(value: number) {
    cursorStrength = value;
    try {
      await invoke('update_cursor_strength', { strength: value });
    } catch (e) {
      console.error('Failed to update cursor strength:', e);
    }
  }

  // Preset management
  async function loadAvailablePresets() {
    try {
      available_presets = await invoke('get_presets_for_simulation_type', {
        simulationType: 'voronoi_ca',
      });
      if (available_presets.length > 0 && !current_preset) {
        current_preset = available_presets[0];
      }
    } catch (e) {
      console.error('Failed to load Voronoi CA presets:', e);
    }
  }

  async function handlePresetChange(value: string) {
    current_preset = value;
    try {
      await invoke('apply_preset', { presetName: value });
      await syncFromBackend();
    } catch (e) {
      console.error('Failed to apply preset:', e);
    }
  }

  async function handlePresetSave(presetName: string) {
    try {
      await invoke('save_preset', { presetName: presetName.trim() });
      await loadAvailablePresets();
      current_preset = presetName.trim();
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  async function syncFromBackend() {
    try {
      const settings = (await invoke('get_current_settings')) as Record<string, unknown>;
      if (settings) {
        if (typeof settings.drift === 'number') drift = settings.drift;
        if (typeof settings.neighborRadius === 'number') neighborRadius = settings.neighborRadius;
        if (typeof settings.aliveThreshold === 'number') aliveThreshold = settings.aliveThreshold;
        if (typeof settings.runSpeed === 'number') runSpeed = settings.runSpeed;
        if (typeof settings.avgRunTime === 'number') avgRunTime = settings.avgRunTime;
        if (typeof settings.tumbleTime === 'number') tumbleTime = settings.tumbleTime;
        if (typeof settings.timeScale === 'number') timeScale = settings.timeScale;
        if (typeof settings.numPoints === 'number') pointCount = settings.numPoints;
        if (typeof settings.currentLutName === 'string') currentLut = settings.currentLutName;
        if (typeof settings.lutReversed === 'boolean') lutReversed = settings.lutReversed;
        if (typeof settings.coloringMode === 'string') coloringMode = settings.coloringMode;
        if (typeof settings.bordersEnabled === 'boolean') bordersEnabled = settings.bordersEnabled;
        if (typeof settings.cursor_size === 'number') cursorSize = settings.cursor_size;
        if (typeof settings.cursor_strength === 'number') cursorStrength = settings.cursor_strength;
        if (typeof settings.borderThreshold === 'number')
          borderThreshold = settings.borderThreshold;
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  async function stopSimulation() {
    try {
      await invoke('pause_simulation');
      running = false;
    } catch (e) {
      console.error('Failed to pause Voronoi CA:', e);
    }
  }

  async function resumeSimulation() {
    try {
      await invoke('resume_simulation');
      running = true;
    } catch (e) {
      console.error('Failed to resume Voronoi CA:', e);
    }
  }

  async function stepSimulation() {
    try {
      // Ensure we are paused; step is ignored while running
      running = false;
      await invoke('pause_simulation');
      await invoke('step_simulation');
    } catch (e) {
      console.error('Failed to step Voronoi CA:', e);
    }
  }

  async function toggleBackendGui() {
    try {
      await invoke('toggle_gui');
      showUI = (await invoke('get_gui_state')) as boolean;
      // When UI is hidden, show controls briefly and start auto-hide timers
      if (!showUI) {
        showControls();
        showCursor();
        startAutoHideTimer();
        startCursorHideTimer();
      } else {
        // If UI is shown, ensure controls are visible and stop timers
        stopAutoHideTimer();
        stopCursorHideTimer();
        showCursor();
        controlsVisible = true;
      }
    } catch (e) {
      console.error('Failed to toggle GUI:', e);
    }
  }

  async function handleMouseEvent(e: CustomEvent) {
    const event = e.detail as MouseEvent | WheelEvent;
    if (event.type === 'wheel') {
      const wheelEvent = event as WheelEvent;
      wheelEvent.preventDefault();
      const dpr = window.devicePixelRatio || 1;
      const screenX = wheelEvent.clientX * dpr;
      const screenY = wheelEvent.clientY * dpr;
      // Pass zoom to backend camera util
      try {
        await invoke('zoom_camera_to_cursor', {
          delta: -wheelEvent.deltaY * 0.001,
          cursorX: screenX,
          cursorY: screenY,
        });
      } catch {
        // Ignore error
      }
      return;
    }

    if (event instanceof MouseEvent) {
      const dpr = window.devicePixelRatio || 1;
      const screenX = event.clientX * dpr;
      const screenY = event.clientY * dpr;
      try {
        if (event.type === 'mousedown') {
          isMousePressed = true;
          currentMouseButton = event.button;
          await invoke('handle_mouse_interaction_screen', {
            screenX,
            screenY,
            mouseButton: currentMouseButton,
          });
        } else if (event.type === 'mousemove') {
          if (isMousePressed) {
            await invoke('handle_mouse_interaction_screen', {
              screenX,
              screenY,
              mouseButton: currentMouseButton,
            });
          }
        } else if (event.type === 'mouseup') {
          if (isMousePressed) {
            isMousePressed = false;
            await invoke('handle_mouse_release', { mouseButton: currentMouseButton });
          }
        } else if (event.type === 'contextmenu') {
          isMousePressed = true;
          currentMouseButton = 2;
          await invoke('handle_mouse_interaction_screen', { screenX, screenY, mouseButton: 2 });
        }
      } catch (err) {
        console.error('Mouse interaction failed:', err);
      }
    }
  }

  async function updateDrift(value: number) {
    drift = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'drift', value });
    } catch (e) {
      console.error('Failed to update drift:', e);
    }
  }

  async function updateNeighborRadius(value: number) {
    neighborRadius = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'neighborRadius', value });
    } catch (e) {
      console.error('Failed to update neighbor radius:', e);
    }
  }

  async function updateAliveThreshold(value: number) {
    aliveThreshold = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'aliveThreshold', value });
    } catch (e) {
      console.error('Failed to update alive threshold:', e);
    }
  }

  async function updateRunSpeed(value: number) {
    runSpeed = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'runSpeed', value });
    } catch (e) {
      console.error('Failed to update run speed:', e);
    }
  }

  async function updatePointCount(value: number) {
    pointCount = Math.max(1, Math.round(value));
    try {
      await invoke('update_simulation_setting', { settingName: 'numPoints', value: pointCount });
    } catch (e) {
      console.error('Failed to update point count:', e);
    }
  }

  async function updateAvgRunTime(value: number) {
    avgRunTime = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'avgRunTime', value });
    } catch (e) {
      console.error('Failed to update avg run time:', e);
    }
  }

  async function updateTumbleTime(value: number) {
    tumbleTime = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'tumbleTime', value });
    } catch (e) {
      console.error('Failed to update tumble time:', e);
    }
  }

  async function updateTimeScale(value: number) {
    timeScale = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'timeScale', value });
    } catch (e) {
      console.error('Failed to update time scale:', e);
    }
  }

  // Auto-hide functionality
  function startAutoHideTimer() {
    stopAutoHideTimer();
    hideTimeout = window.setTimeout(() => {
      controlsVisible = false;
      // Also hide cursor when controls are hidden and UI is hidden
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
    }, 2000);
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

  onMount(() => {
    // Add event listeners for auto-hide functionality (excluding keydown to avoid conflicts)
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'] as const;
    events.forEach((event) => {
      window.addEventListener(event, handleUserInteraction as (event: Event) => void, {
        passive: true,
      });
    });

    start();
  });

  onDestroy(async () => {
    if (unlistenInitialized) unlistenInitialized();
    if (unlistenFps) unlistenFps();
    try {
      await invoke('destroy_simulation');
    } catch (e) {
      console.error('Failed to destroy Voronoi CA:', e);
    }

    // Remove auto-hide event listeners
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'] as const;
    events.forEach((event) => {
      window.removeEventListener(event, handleUserInteraction as (event: Event) => void);
    });

    // Stop timers and restore cursor
    stopAutoHideTimer();
    stopCursorHideTimer();
    showCursor();
  });
</script>

<style>
  .interaction-controls-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
    align-items: start;
  }

  .interaction-help {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .cursor-settings {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .cursor-settings-header {
    font-size: 0.9rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    padding: 0.15rem 0;
  }

  /* Settings grid for key/value pairs */
  .settings-grid {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.15rem 0.3rem;
    width: 100%;
  }

  .setting-item {
    display: contents;
  }

  .setting-label {
    font-weight: 500;
    color: rgba(255, 255, 255, 0.9);
    padding: 0.5rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .setting-item:last-child .setting-label {
    border-bottom: none;
  }

  /* Settings section styling */
  .settings-section {
    margin-bottom: 1.5rem;
  }

  .settings-section:last-child {
    margin-bottom: 0;
  }

  .section-header {
    font-size: 1rem;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.9);
    margin: 0 0 0.75rem 0;
    padding: 0.25rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.2);
  }

  /* Mobile responsive design */
  @media (max-width: 768px) {
    .interaction-controls-grid {
      grid-template-columns: 1fr;
      gap: 0.4rem;
    }

    .interaction-help {
      gap: 0.2rem;
    }

    .cursor-settings {
      gap: 0.2rem;
    }

    .cursor-settings-header {
      font-size: 0.85rem;
    }
  }
</style>
