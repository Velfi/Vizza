<SimulationLayout
  simulationName="Pellets"
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
          Pellets simulates particle collisions with pixel-perfect collision detection. Particles
          bounce off each other using a 3-phase collision system: broad phase, narrow phase, and
          overlap resolution.
        </p>
        <p>
          The simulation uses 4th-order Runge-Kutta integration for stable physics and includes
          collision damping to prevent particles from accelerating indefinitely. Mouse interactions
          allow you to attract particles by left-clicking and dragging.
        </p>
        <p>
          Experiment with different particle counts, collision damping, and initial velocities to
          observe various collision behaviors and particle dynamics.
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
          <label for="lutSelector">Color Scheme</label>
          <LutSelector
            bind:available_luts
            current_lut={state?.current_lut_name || 'MATPLOTLIB_viridis'}
            reversed={state?.lut_reversed || false}
            on:select={({ detail }) => updateLutName(detail.name)}
            on:reverse={() => updateLutReversed()}
          />
        </div>
        <div class="control-group">
          <label for="backgroundType">Background</label>
          <Selector
            id="backgroundType"
            options={['Black', 'White']}
            value={settings?.background_type || 'Black'}
            on:change={({ detail }) => updateSetting('background_type', detail.value)}
          />
        </div>
        <div class="control-group">
          <label for="coloringMode">Particle Coloring Mode</label>
          <Selector
            id="coloringMode"
            options={['Density', 'Velocity', 'Random']}
            value={settings?.coloring_mode || 'Density'}
            on:change={({ detail }) => updateSetting('coloring_mode', detail.value)}
          />
          <div class="setting-description">
            <small>
              <strong>Density:</strong> Particles are colored based on how many other particles are
              nearby.<br />
              <strong>Velocity:</strong> Particles are colored based on their speed (faster =
              brighter).<br />
              <strong>Random:</strong> Each particle gets a unique random color that stays constant.
            </small>
          </div>
        </div>
        <div class="control-group">
          <label for="densityRadius">Density Radius</label>
          <NumberDragBox
            value={settings?.density_radius ?? 0.04}
            min={0.005}
            max={0.1}
            step={0.005}
            precision={3}
            on:change={({ detail }) => updateSetting('density_radius', detail)}
          />
        </div>
        <div class="control-group">
          <label>
            <input
              type="checkbox"
              checked={state?.trails_enabled || false}
              on:change={(e) => updatePelletsTrailsEnabled((e.target as HTMLInputElement).checked)}
            />
            Enable Trails
          </label>
        </div>
        {#if state?.trails_enabled}
          <div class="control-group">
            <label for="trailFade">Trail Fade</label>
            <input
              type="range"
              id="trailFade"
              min="0"
              max="1"
              step="0.01"
              value={state?.trail_fade ?? 0.5}
              on:input={(e) =>
                updatePelletsTrailFade(parseFloat((e.target as HTMLInputElement).value))}
            />
            <span class="range-value">{(state?.trail_fade ?? 0.5).toFixed(2)}</span>
          </div>
        {/if}
      </fieldset>

      <!-- Post Processing -->
      <PostProcessingMenu simulationType="pellets" />

      <!-- Controls -->
      <fieldset>
        <legend>Controls</legend>
        <div class="interaction-controls-grid">
          <div class="interaction-help">
            <div class="control-group">
              <span>🖱️ Left click: Attract particles</span>
            </div>
            <div class="control-group">
              <button type="button" on:click={() => dispatch('navigate', 'how-to-play')}>
                📖 Camera Controls
              </button>
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
            <button type="button" on:click={respawnParticles} class="respawn-button">
              Respawn All Particles
            </button>
          </div>
        </div>

        <!-- Particle Settings -->
        <div class="settings-section">
          <h3 class="section-header">Particle</h3>
          <div class="settings-grid">
            <div class="setting-item">
              <span class="setting-label">Particle Count:</span>
              <NumberDragBox
                value={settings.particle_count}
                min={1}
                max={50000}
                step={1000}
                on:change={({ detail }) => {
                  console.log('NumberDragBox change event triggered:', detail);
                  updateSetting('particle_count', detail);
                }}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Particle Size:</span>
              <NumberDragBox
                value={settings.particle_size}
                min={0.0005}
                max={1.0}
                step={0.0005}
                precision={4}
                on:change={({ detail }) => updateSetting('particle_size', detail)}
              />
            </div>
          </div>
        </div>

        <!-- Physics Settings -->
        <div class="settings-section">
          <h3 class="section-header">Physics</h3>
          <div class="settings-grid">
            <div class="setting-item">
              <span class="setting-label">Gravitational Constant:</span>
              <NumberDragBox
                value={settings?.gravitational_constant ?? 0.0}
                min={0.0}
                max={0.01}
                step={1e-6}
                precision={7}
                on:change={({ detail }) => updateSetting('gravitational_constant', detail)}
              />
            </div>

            <div class="setting-item">
              <span class="setting-label">Energy Lost per Tick (%):</span>
              <NumberDragBox
                value={Number(((1 - (settings?.energy_damping ?? 0.999)) * 100).toFixed(2))}
                min={0.0}
                max={10.0}
                step={0.05}
                precision={2}
                on:change={({ detail }) => updateSetting('energy_damping', 1 - detail / 100)}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Energy Lost on Collision (%):</span>
              <NumberDragBox
                value={Number(((1 - settings.collision_damping) * 100).toFixed(1))}
                min={0.0}
                max={20.0}
                step={0.5}
                precision={1}
                on:change={({ detail }) => updateSetting('collision_damping', 1 - detail / 100)}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Overlap Resolution Strength (%):</span>
              <NumberDragBox
                value={Number(((settings.overlap_resolution_strength ?? 0.02) * 100).toFixed(1))}
                min={0.0}
                max={10.0}
                step={0.5}
                precision={1}
                on:change={({ detail }) =>
                  updateSetting('overlap_resolution_strength', detail / 100)}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Gravity Softening:</span>
              <NumberDragBox
                value={settings?.gravity_softening ?? 0.003}
                min={0.0}
                max={0.02}
                step={0.0005}
                precision={4}
                on:change={({ detail }) => updateSetting('gravity_softening', detail)}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Density-Based Damping:</span>
              <input
                type="checkbox"
                checked={settings?.density_damping_enabled ?? true}
                on:change={(e) =>
                  updateSetting('density_damping_enabled', (e.target as HTMLInputElement).checked)}
              />
            </div>
          </div>
        </div>
      </fieldset>
    </form>
  {/if}
</SimulationLayout>

<!-- Shared camera controls component -->
<CameraControls enabled={true} on:toggleGui={toggleBackendGui} on:togglePause={togglePause} />

<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
  import PresetFieldset from './components/shared/PresetFieldset.svelte';
  import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
  import NumberDragBox from './components/inputs/NumberDragBox.svelte';
  import CameraControls from './components/shared/CameraControls.svelte';
  import LutSelector from './components/shared/LutSelector.svelte';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import Selector from './components/inputs/Selector.svelte';
  import './shared-theme.css';

  export let menuPosition: string;

  const dispatch = createEventDispatcher();

  interface PelletsSettings {
    particle_count: number;
    particle_size: number;
    collision_damping: number;
    initial_velocity_max: number;
    initial_velocity_min: number;
    random_seed: number;
    background_type: string;
    coloring_mode: string;
    // Legacy fields that may still be present in backend but not used in UI
    gravitational_constant?: number;
    energy_damping?: number;
    gravity_softening?: number;
    density_radius?: number;
    density_damping_enabled?: boolean;
    overlap_resolution_strength?: number;
  }

  interface PelletsState {
    current_lut_name: string;
    lut_reversed: boolean;
    gui_visible: boolean;
    mouse_pressed: boolean;
    mouse_mode: number;
    mouse_position: [number, number];
    camera_position: [number, number];
    camera_zoom: number;
    simulation_time: number;
    is_running: boolean;
    cursor_size: number;
    cursor_strength: number;
    trails_enabled?: boolean;
    trail_fade?: number;
  }

  let settings: PelletsSettings | null = null;
  let state: PelletsState | null = null;
  let running = false;
  let currentFps = 0;
  let showUI = true;
  let controlsVisible = true;
  let loading = false;
  let show_about_section = false;
  let available_presets: string[] = [];
  let current_preset = '';
  let available_luts: string[] = [];
  let cursorSize = 0.5;
  let cursorStrength = 0.01;

  let renderLoopId: number | null = null;
  let fpsUpdateUnlisten: (() => void) | null = null;

  // Auto-hide functionality for controls when UI is hidden
  let hideTimeout: number | null = null;

  // Cursor hiding functionality
  let cursorHidden = false;
  let cursorHideTimeout: number | null = null;

  // Mouse press tracking for consistent interaction and release handling
  let isMousePressed = false;
  let currentMouseButton = 0;

  const returnToMenu = () => {
    dispatch('back');
  };

  const handleUserInteraction = () => {
    if (!showUI && !controlsVisible) {
      showControls();
      showCursor();
      startAutoHideTimer();
    } else if (!showUI && controlsVisible) {
      showCursor();
      startAutoHideTimer();
      startCursorHideTimer();
    }
  };

  const handleMouseEvent = async (e: CustomEvent) => {
    const event = e.detail as MouseEvent | WheelEvent;

    // Handle zoom separately
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
      return;
    }

    // Handle mouse interactions for attraction / repulsion
    if (event instanceof MouseEvent) {
      const mouseEvent = event as MouseEvent;

      // Convert to physical screen coords
      const devicePixelRatio = window.devicePixelRatio || 1;
      const screenX = mouseEvent.clientX * devicePixelRatio;
      const screenY = mouseEvent.clientY * devicePixelRatio;

      try {
        if (mouseEvent.type === 'mousedown') {
          isMousePressed = true;
          currentMouseButton = mouseEvent.button;
          await invoke('handle_mouse_interaction_screen', {
            screenX,
            screenY,
            mouseButton: currentMouseButton,
          });
        } else if (mouseEvent.type === 'mousemove') {
          if (isMousePressed) {
            // Continue interaction while button is held
            await invoke('handle_mouse_interaction_screen', {
              screenX,
              screenY,
              mouseButton: currentMouseButton,
            });
          }
        } else if (mouseEvent.type === 'mouseup') {
          if (isMousePressed) {
            isMousePressed = false;
            await invoke('handle_mouse_release', {
              mouseButton: currentMouseButton,
            });
          }
        } else if (mouseEvent.type === 'contextmenu') {
          // Treat context menu as a right-button press
          isMousePressed = true;
          currentMouseButton = 2;
          await invoke('handle_mouse_interaction_screen', {
            screenX,
            screenY,
            mouseButton: 2,
          });
        }
      } catch (err) {
        console.error('Mouse interaction failed:', err);
      }
    }
  };

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

  const updateSetting = async (key: string, value: string | number | boolean) => {
    console.log('updateSetting called with:', key, value);
    if (!settings) {
      console.log('No settings available, returning');
      return;
    }

    // Special handling for particle count changes
    if (key === 'particle_count') {
      console.log('Handling particle_count change');
      if (typeof value === 'number') {
        await updateParticleCount(value);
      }
      return;
    }

    try {
      console.log('Calling update_simulation_setting for:', key, value);
      await invoke('update_simulation_setting', { settingName: key, value });
      // Update local settings
      if (settings) {
        (settings as unknown as Record<string, unknown>)[key] = value;
      }
      console.log('Setting updated successfully');
    } catch (error) {
      console.error('Failed to update setting:', error);
    }
  };

  const updateParticleCount = async (value: number) => {
    if (!settings) return;

    const newCount = Math.max(1, Math.min(50000, Math.round(value)));
    console.log(`updateParticleCount called: current=${settings.particle_count}, new=${newCount}`);

    // Don't check if they're equal since the UI binding might have already updated the local value
    // Just proceed with the update

    settings.particle_count = newCount;

    try {
      console.log(`Sending particle count update to backend: ${newCount}`);
      console.log('Invoking update_simulation_setting with:', {
        settingName: 'particle_count',
        value: newCount,
      });

      const result = await invoke('update_simulation_setting', {
        settingName: 'particle_count',
        value: newCount,
      });
      console.log('Backend response:', result);

      console.log(`Backend update complete, waiting for GPU operations...`);
      // Add a small delay to ensure GPU operations are complete
      await new Promise((resolve) => setTimeout(resolve, 100));

      console.log(`Syncing state from backend...`);
      // Sync state from backend to ensure frontend reflects actual backend state
      await loadSettings();

      console.log(`Particle count update complete: ${newCount}`);
    } catch (error) {
      console.error('Failed to update particle count:', error);
      // Revert state on error
      await loadSettings();
    }
  };

  async function togglePause() {
    if (running) {
      await stopSimulation();
    } else {
      await resumeSimulation();
    }
  }

  const loadSettings = async () => {
    try {
      const response = await invoke('get_current_settings');
      settings = response as PelletsSettings;
    } catch (error) {
      console.error('Failed to load settings:', error);
    }
  };

  const loadState = async () => {
    try {
      const response = await invoke('get_current_state');
      state = response as PelletsState;

      // Sync cursor values from backend state
      if (state && state.cursor_size !== undefined) {
        cursorSize = state.cursor_size;
      }
      if (state && state.cursor_strength !== undefined) {
        cursorStrength = state.cursor_strength;
      }
    } catch (error) {
      console.error('Failed to load state:', error);
    }
  };

  const updatePreset = async (presetName: string) => {
    if (!presetName) return;

    try {
      await invoke('apply_preset', { presetName });
      await loadSettings();
      current_preset = presetName;
    } catch (error) {
      console.error('Failed to load preset:', error);
    }
  };

  const savePreset = async (presetName: string) => {
    if (!presetName) return;

    try {
      await invoke('save_preset', { presetName });
      await loadAvailablePresets();
      current_preset = presetName;
    } catch (error) {
      console.error('Failed to save preset:', error);
    }
  };

  const loadAvailablePresets = async () => {
    try {
      const presets = await invoke('get_available_presets');
      available_presets = presets as string[];

      // Set the default preset if available
      if (available_presets.includes('Default')) {
        current_preset = 'Default';
      }
    } catch (error) {
      console.error('Failed to load presets:', error);
    }
  };

  const loadAvailableLuts = async () => {
    try {
      const luts = await invoke('get_available_luts');
      available_luts = luts as string[];
    } catch (error) {
      console.error('Failed to load LUTs:', error);
    }
  };

  const updateLutName = async (value: string) => {
    try {
      await invoke('apply_lut_by_name', { lutName: value });
      await loadSettings(); // Sync UI with backend state
    } catch (error) {
      console.error('Failed to update LUT name:', error);
    }
  };

  const updateLutReversed = async () => {
    try {
      await invoke('toggle_lut_reversed');
      await loadSettings(); // Sync UI with backend state
    } catch (error) {
      console.error('Failed to update LUT reversed:', error);
    }
  };

  async function updatePelletsTrailsEnabled(enabled: boolean) {
    if (!state) return;
    state.trails_enabled = enabled;
    try {
      await invoke('update_pellets_trails_state', { enabled, fade: state.trail_fade ?? 0.5 });
    } catch (e) {
      console.error('Failed to update pellets trails enabled', e);
    }
  }

  async function updatePelletsTrailFade(fade: number) {
    if (!state) return;
    state.trail_fade = fade;
    try {
      await invoke('update_pellets_trails_state', { enabled: state.trails_enabled ?? false, fade });
    } catch (e) {
      console.error('Failed to update pellets trail fade', e);
    }
  }

  const updateCursorSize = async (value: number) => {
    cursorSize = value;
    try {
      await invoke('update_cursor_size', { size: value });
    } catch (error) {
      console.error('Failed to update cursor size:', error);
    }
  };

  const updateCursorStrength = async (value: number) => {
    cursorStrength = value;
    try {
      await invoke('update_cursor_strength', { strength: value });
    } catch (error) {
      console.error('Failed to update cursor strength:', error);
    }
  };

  const respawnParticles = async () => {
    try {
      await invoke('reset_simulation');
      console.log('All particles respawned');
    } catch (error) {
      console.error('Failed to respawn particles:', error);
    }
  };

  const stopSimulation = async () => {
    try {
      await invoke('pause_simulation');
      running = false;
      if (renderLoopId) {
        cancelAnimationFrame(renderLoopId);
        renderLoopId = null;
      }
    } catch (error) {
      console.error('Failed to stop simulation:', error);
    }
  };

  const resumeSimulation = async () => {
    try {
      await invoke('resume_simulation');
      running = true;
      startRenderLoop();
    } catch (error) {
      console.error('Failed to resume simulation:', error);
    }
  };

  const toggleBackendGui = async () => {
    try {
      await invoke('toggle_gui');

      // Get the current GUI state
      const visible = (await invoke('get_gui_state')) as boolean;
      showUI = visible;

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
    } catch (error) {
      console.error('Failed to toggle GUI:', error);
    }
  };

  const startRenderLoop = () => {
    if (renderLoopId) return;

    const render = async () => {
      if (!running) return;

      try {
        await invoke('render_frame');
        renderLoopId = requestAnimationFrame(render);
      } catch (error) {
        console.error('Render failed:', error);
        running = false;
        renderLoopId = null;
      }
    };

    render();
  };

  const startSimulation = async () => {
    if (running || loading) return;

    loading = true;

    try {
      await invoke('start_pellets_simulation');
      loading = false;
      running = true;

      // Backend now handles the render loop, we just track state
      currentFps = 0;
    } catch (error) {
      console.error('Failed to switch to pellets simulation:', error);
    } finally {
      loading = false;
    }
  };

  onMount(async () => {
    // Add event listeners for auto-hide functionality (excluding keydown to avoid conflicts with CameraControls)
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.addEventListener(event, handleUserInteraction, { passive: true });
    });

    try {
      // Start the simulation first
      await startSimulation();

      // Load available presets, LUTs, settings, and state
      await loadAvailablePresets();
      await loadAvailableLuts();
      await loadSettings();
      await loadState();

      // Set the default preset if available and not already set
      if (available_presets.includes('Default') && !current_preset) {
        current_preset = 'Default';
      }

      // Listen for FPS updates
      listen('fps-update', (event) => {
        currentFps = event.payload as number;
      }).then((unlisten) => {
        fpsUpdateUnlisten = unlisten;
      });
    } catch (error) {
      console.error('Failed to initialize pellets simulation:', error);
    }
  });

  onDestroy(async () => {
    // Clean up the simulation
    try {
      await invoke('destroy_simulation');
    } catch (error) {
      console.error('Failed to destroy simulation on component destroy:', error);
    }

    if (fpsUpdateUnlisten) {
      fpsUpdateUnlisten();
    }

    if (renderLoopId) {
      cancelAnimationFrame(renderLoopId);
    }

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
  });
</script>

<style>
  .respawn-button {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.2s ease;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .respawn-button:hover {
    background: linear-gradient(135deg, #5a6fd8 0%, #6a4190 100%);
    transform: translateY(-1px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
  }

  .respawn-button:active {
    transform: translateY(0);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .setting-description {
    margin-top: 4px;
    color: #888;
    line-height: 1.4;
  }

  .setting-description small {
    font-size: 13px;
    color: #aaa;
  }

  .setting-description strong {
    color: #ccc;
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
