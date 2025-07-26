<SimulationLayout
  simulationName="Flow Field"
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
          Flow Field creates beautiful patterns by moving particles through a vector field generated
          from noise functions. Particles follow the direction of nearby flow vectors, creating
          organic, flowing animations.
        </p>
        <p>
          The simulation uses various noise algorithms to generate the underlying vector field,
          including Perlin noise, FBM, Billow, and others. Each noise type produces different flow
          patterns and behaviors.
        </p>
        <p>
          Experiment with different noise types, adjust particle parameters, and watch as simple
          vector fields create complex, mesmerizing particle flows reminiscent of natural phenomena
          like wind, water currents, and magnetic fields.
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
          <label for="background-select">Background</label>
          <Selector
            options={['Black', 'White']}
            bind:value={backgroundValue}
            on:change={(e) => updateBackground(e.detail.value)}
          />
        </div>

        <div class="control-group">
          <label for="display-mode-select">Display Mode</label>
          <Selector
            options={['Age', 'Random', 'Direction']}
            bind:value={settings.display_mode}
            on:change={(e) => updateDisplayMode(e.detail.value)}
          />
          <div class="setting-description">
            <small>
              <strong>Age:</strong> Particles are colored based on their age (younger = brighter).<br
              />
              <strong>Random:</strong> Particles have a random color chosen at creation that doesn't
              change.<br />
              <strong>Direction:</strong> Particles are colored based on their velocity direction.
            </small>
          </div>
        </div>

        <LutSelector
          {available_luts}
          bind:current_lut={currentLutValue}
          bind:reversed={lutReversedValue}
          on:select={({ detail }) => updateLut(detail.name)}
          on:reverse={() => updateLutReversed()}
        />
      </fieldset>

      <!-- Controls -->
      <fieldset>
        <legend>Controls</legend>
        <div class="interaction-controls-grid">
          <div class="interaction-help">
            <div class="control-group">
              <span>🖱️ Left click: Spawn particles | Right click: Destroy particles</span>
            </div>
            <div class="control-group">
              <Button variant="default" on:click={() => dispatch('navigate', 'how-to-play')}>
                📖 Camera Controls
              </Button>
            </div>
            <div class="control-group">
              <span
                >Camera controls not working? Click the control bar at the top of the screen.</span
              >
            </div>
            <div class="control-group">
              <Button variant="danger" on:click={killAllParticles}>💀 Kill All Particles</Button>
            </div>
          </div>
          <div class="cursor-settings">
            <div class="cursor-settings-header">
              <span>🎯 Cursor Settings</span>
            </div>
            <CursorConfig
              {cursorSize}
              {cursorStrength}
              sizeMin={10}
              sizeMax={65}
              sizeStep={5}
              strengthMin={0}
              strengthMax={1}
              strengthStep={0.01}
              sizePrecision={0}
              strengthPrecision={2}
              on:sizechange={(e) => updateCursorSize(e.detail)}
              on:strengthchange={(e) => updateCursorStrength(e.detail)}
            />
          </div>
        </div>
      </fieldset>

      <!-- Combined Settings -->
      <fieldset>
        <legend>Settings</legend>

        <!-- Flow Field Settings -->
        <div class="settings-section">
          <h3 class="section-header">Flow Field</h3>
          <div class="settings-grid">
            <div class="setting-item">
              <span class="setting-label">Noise Type:</span>
              <Selector
                options={[
                  'Perlin',
                  'Simplex',
                  'OpenSimplex',
                  'Worley',
                  'Value',
                  'FBM',
                  'FBMBillow',
                  'FBMClouds',
                  'FBMRidged',
                  'Billow',
                  'RidgedMulti',
                  'Cylinders',
                  'Checkerboard',
                ]}
                bind:value={settings.noise_type}
                on:change={(e) => updateNoiseType(e.detail.value)}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Noise Seed:</span>
              <NumberDragBox
                value={settings.noise_seed}
                on:change={({ detail }) => updateNoiseSeed(detail)}
                min={0}
                max={100000}
                step={1}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Noise Scale:</span>
              <NumberDragBox
                value={settings.noise_scale}
                on:change={({ detail }) => updateNoiseScale(detail)}
                min={0.01}
                max={10.0}
                step={0.1}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Vector Magnitude:</span>
              <NumberDragBox
                value={settings.vector_magnitude}
                on:change={({ detail }) => updateVectorMagnitude(detail)}
                min={0.001}
                max={5.0}
                step={0.1}
              />
            </div>
          </div>
        </div>

        <!-- Particle Settings -->
        <div class="settings-section">
          <h3 class="section-header">Particles</h3>
          <div class="settings-grid">
            <div class="setting-item">
              <span class="setting-label">Autospawn Limit:</span>
              <NumberDragBox
                value={settings.autospawn_limit}
                on:change={({ detail }) => updateAutospawnLimit(detail)}
                min={100}
                max={50000}
                step={100}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Particle Lifetime:</span>
              <NumberDragBox
                value={settings.particle_lifetime}
                on:change={({ detail }) => updateParticleLifetime(detail)}
                min={0.1}
                max={10.0}
                step={0.1}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Particle Speed:</span>
              <NumberDragBox
                value={settings.particle_speed}
                on:change={({ detail }) => updateParticleSpeed(detail)}
                min={0.001}
                max={0.2}
                step={0.001}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Particle Size (pixels):</span>
              <NumberDragBox
                value={settings.particle_size}
                on:change={({ detail }) => updateParticleSize(detail)}
                min={1}
                max={50}
                step={1}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Particle Shape:</span>
              <Selector
                options={['Circle', 'Square', 'Triangle', 'Flower', 'Diamond']}
                bind:value={settings.particle_shape}
                on:change={(e) => updateParticleShape(e.detail.value)}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">
                <input
                  type="checkbox"
                  checked={settings.particle_autospawn}
                  on:change={(e) => updateParticleAutospawn((e.target as HTMLInputElement).checked)}
                />
                Auto-spawn Particles
              </span>
            </div>
            <div class="setting-item">
              <span class="setting-label">
                <input
                  type="checkbox"
                  checked={settings.show_particles}
                  on:change={(e) => updateShowParticles((e.target as HTMLInputElement).checked)}
                />
                Show Particles
              </span>
            </div>
            <div class="setting-item">
              <span class="setting-label">Particle Spawn Rate:</span>
              <NumberDragBox
                value={settings.particle_spawn_rate}
                on:change={({ detail }) => updateParticleSpawnRate(detail)}
                min={0.0}
                max={1.0}
                step={0.01}
              />
            </div>
          </div>
        </div>

        <!-- Trail Settings -->
        <div class="settings-section">
          <h3 class="section-header">Trails</h3>
          <div class="settings-grid">
            <div class="setting-item">
              <span class="setting-label">Trail Decay Rate:</span>
              <NumberDragBox
                value={settings.trail_decay_rate}
                on:change={({ detail }) => updateTrailDecayRate(detail)}
                min={0.0}
                max={1.0}
                step={0.01}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Trail Deposition Rate:</span>
              <NumberDragBox
                value={settings.trail_deposition_rate}
                on:change={({ detail }) => updateTrailDepositionRate(detail)}
                min={0.0}
                max={1.0}
                step={0.01}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Trail Diffusion Rate:</span>
              <NumberDragBox
                value={settings.trail_diffusion_rate}
                on:change={({ detail }) => updateTrailDiffusionRate(detail)}
                min={0.0}
                max={1.0}
                step={0.01}
              />
            </div>
            <div class="setting-item">
              <span class="setting-label">Trail Wash Out Rate:</span>
              <NumberDragBox
                value={settings.trail_wash_out_rate}
                on:change={({ detail }) => updateTrailWashOutRate(detail)}
                min={0.0}
                max={1.0}
                step={0.01}
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
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import Button from './components/shared/Button.svelte';
  import NumberDragBox from './components/inputs/NumberDragBox.svelte';
  import LutSelector from './components/shared/LutSelector.svelte';
  import Selector from './components/inputs/Selector.svelte';
  import SimulationLayout from './components/shared/SimulationLayout.svelte';
  import CameraControls from './components/shared/CameraControls.svelte';
  import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
  import PresetFieldset from './components/shared/PresetFieldset.svelte';
  import CursorConfig from './components/shared/CursorConfig.svelte';
  import './shared-theme.css';

  const dispatch = createEventDispatcher();

  export let menuPosition: string = 'middle';

  // Simulation state
  type Settings = {
    // Flow field parameters
    noise_type: string;
    noise_seed: number;
    noise_scale: number;
    vector_magnitude: number;

    // Particle parameters
    particle_limit: number;
    autospawn_limit: number;
    particle_lifetime: number;
    particle_speed: number;
    particle_size: number;
    particle_shape: string;
    particle_autospawn: boolean;
    particle_spawn_rate: number;

    // Trail parameters
    trail_decay_rate: number;
    trail_deposition_rate: number;
    trail_diffusion_rate: number;
    trail_wash_out_rate: number;

    // Visual parameters
    background: string;
    current_lut: string;
    lut_reversed: boolean;
    show_particles: boolean;
    display_mode: string;
  };

  let settings: Settings | undefined = undefined;

  // Local variables for binding
  let backgroundValue = 'Black';
  let currentLutValue = 'MATPLOTLIB_viridis';
  let lutReversedValue = false;

  // Preset and LUT state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];

  // UI state
  let show_about_section = false;

  // Cursor state
  let cursorSize = 100;
  let cursorStrength = 0.3;

  // Simulation control state
  let running = false;
  let loading = false;
  let showUI = true;
  let currentFps = 0;
  let controlsVisible = true;

  // Auto-hide functionality for controls when UI is hidden
  let hideTimeout: number | null = null;

  // Cursor hiding functionality
  let cursorHidden = false;
  let cursorHideTimeout: number | null = null;

  // Event listeners
  let unlistenFps: (() => void) | null = null;
  let unlistenSimulationInitialized: (() => void) | null = null;

  async function returnToMenu() {
    try {
      await invoke('destroy_simulation');
      dispatch('back');
    } catch (error) {
      console.error('Failed to return to menu:', error);
    }
  }

  async function toggleBackendGui() {
    try {
      await invoke('toggle_gui');
      const visible = (await invoke('get_gui_state')) as boolean;
      showUI = visible;

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
  }

  async function stopSimulation() {
    try {
      await invoke('pause_simulation');
      running = false;
    } catch (error) {
      console.error('Failed to stop simulation:', error);
    }
  }

  async function resumeSimulation() {
    try {
      await invoke('resume_simulation');
      running = true;
    } catch (error) {
      console.error('Failed to resume simulation:', error);
    }
  }

  async function togglePause() {
    if (running) {
      await stopSimulation();
    } else {
      await resumeSimulation();
    }
  }

  // Auto-hide functionality
  function startAutoHideTimer() {
    stopAutoHideTimer();
    hideTimeout = setTimeout(() => {
      controlsVisible = false;
      // Also hide cursor when controls are hidden
      if (!showUI) {
        hideCursor();
      }
    }, 3000);
  }

  function stopAutoHideTimer() {
    if (hideTimeout !== null) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }
  }

  function showControls() {
    controlsVisible = true;
    startAutoHideTimer();
  }

  // Cursor hiding functionality
  function startCursorHideTimer() {
    stopCursorHideTimer();
    cursorHideTimeout = setTimeout(() => {
      if (!showUI && !controlsVisible) {
        hideCursor();
      }
    }, 2000);
  }

  function stopCursorHideTimer() {
    if (cursorHideTimeout !== null) {
      clearTimeout(cursorHideTimeout);
      cursorHideTimeout = null;
    }
  }

  function showCursor() {
    if (cursorHidden) {
      document.body.style.cursor = '';
      cursorHidden = false;
    }
  }

  function hideCursor() {
    if (!cursorHidden) {
      document.body.style.cursor = 'none';
      cursorHidden = true;
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

  let isMousePressed = false;
  let currentMouseButton = 0;

  async function handleMouseEvent(e: CustomEvent) {
    const event = e.detail as MouseEvent | WheelEvent;

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
    } else if (event.type === 'mousedown') {
      const mouseEvent = event as MouseEvent;
      mouseEvent.preventDefault();

      isMousePressed = true;
      currentMouseButton = mouseEvent.button;

      // Convert screen coordinates to world coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
      const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

      console.log(
        `Flow mouse interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: ${mouseEvent.button}`
      );

      try {
        await invoke('handle_mouse_interaction_screen', {
          screenX: physicalCursorX,
          screenY: physicalCursorY,
          mouseButton: mouseEvent.button,
        });
      } catch (e) {
        console.error('Failed to handle Flow mouse interaction:', e);
      }
    } else if (event.type === 'mousemove') {
      if (isMousePressed) {
        const mouseEvent = event as MouseEvent;
        mouseEvent.preventDefault();

        // Convert screen coordinates to world coordinates
        const devicePixelRatio = window.devicePixelRatio || 1;
        const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
        const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

        // Use the same button state as when mouse was first pressed
        try {
          await invoke('handle_mouse_interaction_screen', {
            screenX: physicalCursorX,
            screenY: physicalCursorY,
            mouseButton: currentMouseButton,
          });
        } catch (e) {
          console.error('Failed to handle Flow mouse interaction:', e);
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
        console.error('Failed to stop Flow mouse interaction:', e);
      }
    } else if (event.type === 'contextmenu') {
      // Handle context menu as right-click for simulation interaction
      const mouseEvent = event as MouseEvent;

      // Convert screen coordinates to physical coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
      const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

      console.log(
        `Flow context menu interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: 2`
      );

      try {
        await invoke('handle_mouse_interaction_screen', {
          screenX: physicalCursorX,
          screenY: physicalCursorY,
          mouseButton: 2, // Right mouse button
        });
      } catch (e) {
        console.error('Failed to handle Flow context menu interaction:', e);
      }
    }

    // Always handle user interaction for UI visibility
    handleUserInteraction();
  }

  async function startSimulation() {
    if (running || loading) return;

    loading = true;

    try {
      await invoke('start_simulation', { simulationType: 'flow' });
      currentFps = 0;
    } catch (e) {
      console.error('Failed to start simulation:', e);
      loading = false;
      running = false;
    }
  }

  // Setting update functions
  async function updateNoiseSeed(value: number) {
    // Validate the value before using it
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid noise seed value:', value);
      return;
    }

    settings!.noise_seed = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'noiseSeed',
        value,
      });
    } catch (e) {
      console.error('Failed to update noise seed:', e);
    }
  }

  async function updateNoiseScale(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid noise scale value:', value);
      return;
    }

    settings!.noise_scale = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'noiseScale',
        value,
      });
    } catch (e) {
      console.error('Failed to update noise scale:', e);
    }
  }

  async function updateNoiseType(value: string) {
    settings!.noise_type = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'noiseType',
        value,
      });
    } catch (e) {
      console.error('Failed to update noise type:', e);
    }
  }

  async function updateVectorMagnitude(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid vector magnitude value:', value);
      return;
    }

    settings!.vector_magnitude = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'vectorMagnitude',
        value,
      });
    } catch (e) {
      console.error('Failed to update vector magnitude:', e);
    }
  }

  async function updateAutospawnLimit(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid autospawn limit value:', value);
      return;
    }

    settings!.autospawn_limit = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'autospawnLimit',
        value,
      });
    } catch (e) {
      console.error('Failed to update autospawn limit:', e);
    }
  }

  async function updateParticleLifetime(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid particle lifetime value:', value);
      return;
    }

    settings!.particle_lifetime = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'particleLifetime',
        value,
      });
    } catch (e) {
      console.error('Failed to update particle lifetime:', e);
    }
  }

  async function updateParticleSpeed(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid particle speed value:', value);
      return;
    }

    settings!.particle_speed = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'particleSpeed',
        value,
      });
    } catch (e) {
      console.error('Failed to update particle speed:', e);
    }
  }

  async function updateParticleSize(value: number) {
    // Validate the value before using it
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid particle size value:', value);
      return;
    }

    // Ensure particle size is an integer
    const intValue = Math.round(value);
    settings!.particle_size = intValue;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'particleSize',
        value: intValue,
      });
    } catch (e) {
      console.error('Failed to update particle size:', e);
    }
  }

  async function updateParticleShape(value: string) {
    settings!.particle_shape = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'particleShape',
        value,
      });
    } catch (e) {
      console.error('Failed to update particle shape:', e);
    }
  }

  async function updateParticleAutospawn(value: boolean) {
    settings!.particle_autospawn = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'particleAutospawn',
        value,
      });
    } catch (e) {
      console.error('Failed to update particle autospawn:', e);
    }
  }

  async function updateShowParticles(value: boolean) {
    settings!.show_particles = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'showParticles',
        value,
      });
    } catch (e) {
      console.error('Failed to update show particles:', e);
    }
  }

  async function updateParticleSpawnRate(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid particle spawn rate value:', value);
      return;
    }

    settings!.particle_spawn_rate = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'particleSpawnRate',
        value,
      });
    } catch (e) {
      console.error('Failed to update particle spawn rate:', e);
    }
  }

  async function killAllParticles() {
    try {
      await invoke('kill_all_particles');
      console.log('All particles killed successfully');
    } catch (e) {
      console.error('Failed to kill all particles:', e);
    }
  }

  async function updateTrailDecayRate(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid trail decay rate value:', value);
      return;
    }

    settings!.trail_decay_rate = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'trailDecayRate',
        value,
      });
    } catch (e) {
      console.error('Failed to update trail decay rate:', e);
    }
  }

  async function updateTrailDepositionRate(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid trail deposition rate value:', value);
      return;
    }

    settings!.trail_deposition_rate = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'trailDepositionRate',
        value,
      });
    } catch (e) {
      console.error('Failed to update trail deposition rate:', e);
    }
  }

  async function updateTrailDiffusionRate(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid trail diffusion rate value:', value);
      return;
    }

    settings!.trail_diffusion_rate = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'trailDiffusionRate',
        value,
      });
    } catch (e) {
      console.error('Failed to update trail diffusion rate:', e);
    }
  }

  async function updateTrailWashOutRate(value: number) {
    if (typeof value !== 'number' || isNaN(value)) {
      console.error('Invalid trail wash out rate value:', value);
      return;
    }

    settings!.trail_wash_out_rate = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'trailWashOutRate',
        value,
      });
    } catch (e) {
      console.error('Failed to update trail wash out rate:', e);
    }
  }

  async function updateBackground(value: string) {
    settings!.background = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'background',
        value,
      });
    } catch (e) {
      console.error('Failed to update background:', e);
    }
  }

  async function updateLut(lutName: string) {
    settings!.current_lut = lutName;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'currentLut',
        value: lutName,
      });
    } catch (e) {
      console.error('Failed to update LUT:', e);
    }
  }

  async function updateLutReversed() {
    settings!.lut_reversed = !settings!.lut_reversed;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'lutReversed',
        value: settings!.lut_reversed,
      });
    } catch (e) {
      console.error('Failed to update LUT reversed:', e);
    }
  }

  async function updateCursorSize(value: number) {
    cursorSize = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'cursorSize',
        value,
      });
    } catch (e) {
      console.error('Failed to update cursor size:', e);
    }
  }

  async function updateCursorStrength(value: number) {
    cursorStrength = value;
    try {
      await invoke('update_simulation_setting', {
        settingName: 'cursorStrength',
        value,
      });
    } catch (e) {
      console.error('Failed to update cursor strength:', e);
    }
  }

  async function updateDisplayMode(value: string) {
    try {
      await invoke('update_simulation_setting', {
        settingName: 'displayMode',
        value,
      });
      await syncSettingsFromBackend();
    } catch (e) {
      console.error('Failed to update display mode:', e);
    }
  }

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

  async function savePreset(presetName: string) {
    try {
      await invoke('save_preset', { presetName: presetName.trim() });
      await loadAvailablePresets();
      current_preset = presetName.trim();
      console.log(`Saved preset: ${presetName}`);
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  async function loadAvailablePresets() {
    try {
      available_presets = await invoke('get_presets_for_simulation_type', {
        simulationType: 'flow',
      });
      if (available_presets.length > 0 && !current_preset) {
        current_preset = available_presets[0];
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
      const backendSettings = await invoke('get_current_settings');
      const backendState = await invoke('get_current_state');

      if (backendSettings) {
        // Use backend settings directly
        settings = backendSettings as Settings;

        // Update local binding variables with proper defaults
        backgroundValue = settings.background || 'Black';
        currentLutValue = settings.current_lut || 'MATPLOTLIB_viridis';
        lutReversedValue = settings.lut_reversed || false;

        console.log('Flow settings synced:', {
          background: backgroundValue,
          currentLut: currentLutValue,
          lutReversed: lutReversedValue,
        });
      }

      if (backendState) {
        // Update cursor state from backend
        const state = backendState as any;
        if (state.cursorSize !== undefined) {
          cursorSize = state.cursorSize;
        }
        if (state.cursorStrength !== undefined) {
          cursorStrength = state.cursorStrength;
        }
      }
    } catch (e) {
      console.error('Failed to sync settings from backend:', e);
    }
  }

  onMount(async () => {
    // Add event listeners for auto-hide functionality (excluding keydown to avoid conflicts with CameraControls)
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.addEventListener(event, handleUserInteraction, { passive: true });
    });

    // Listen for simulation-initialized event
    unlistenSimulationInitialized = await listen('simulation-initialized', async () => {
      running = true;
      loading = false;

      // Now that simulation is initialized, sync settings and load data
      await syncSettingsFromBackend();
      await loadAvailablePresets();
      await loadAvailableLuts();
    });

    // Start simulation and keep loading until we get settings
    await startSimulation();

    unlistenFps = await listen('fps-update', (event: { payload: number }) => {
      currentFps = event.payload;
    });
  });

  onDestroy(async () => {
    try {
      await invoke('destroy_simulation');
    } catch (error) {
      console.error('Failed to destroy simulation on component destroy:', error);
    }

    if (unlistenFps) {
      unlistenFps();
    }
    if (unlistenSimulationInitialized) {
      unlistenSimulationInitialized();
    }

    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];
    events.forEach((event) => {
      window.removeEventListener(event, handleUserInteraction);
    });

    stopAutoHideTimer();

    // Stop cursor hide timer and restore cursor
    stopCursorHideTimer();
    showCursor();
  });
</script>

<style>
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
    font-weight: 600;
    margin-bottom: 0.5rem;
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .control-group span {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }
</style>
