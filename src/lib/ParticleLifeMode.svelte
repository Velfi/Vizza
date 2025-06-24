<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NumberDragBox from './components/NumberDragBox.svelte';
  import LutSelector from './components/LutSelector.svelte';

  const dispatch = createEventDispatcher();

  interface Settings {
    species_count: number;
    force_matrix: number[][];
    max_force: number;
    min_distance: number;
    max_distance: number;
    friction: number;
    wrap_edges: boolean;
    force_beta: number;
  }

  interface State {
    particle_count: number;
    random_seed: number;
    dt: number;
    cursor_size: number;
    cursor_strength: number;
    traces_enabled: boolean;
    trace_fade: number;
    edge_fade_strength: number;
    position_generator: string;
    type_generator: string;
    matrix_generator: string;
    current_lut: string;
    lut_reversed: boolean;
  }

  // Simulation state
  let settings: Settings = {
    species_count: 4,
    force_matrix: [
      [-0.1,  0.2, -0.1,  0.1],
      [ 0.2, -0.1,  0.3, -0.1],
      [-0.1,  0.3, -0.1,  0.2],
      [ 0.1, -0.1,  0.2, -0.1]
    ],
    max_force: 1.0,
    min_distance: 0.01,
    max_distance: 0.03,
    friction: 0.85,
    wrap_edges: true,
    force_beta: 0.3,
  };

  // Runtime state
  let state: State = {
    particle_count: 15000,
    random_seed: 0,
    dt: 0.01,
    cursor_size: 0.2,
    cursor_strength: 5.0,
    traces_enabled: false,
    trace_fade: 0.95,
    edge_fade_strength: 0.0,
    position_generator: 'Random',
    type_generator: 'Random',
    matrix_generator: 'Random',
    current_lut: '',
    lut_reversed: false
  };

  // UI state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];
  let show_save_preset_dialog = false;
  let new_preset_name = '';
  let fps_display = 0;
  let physics_time_avg = 0;
  let isSimulationRunning = false;
  
  // Enhanced UI state
  let showUI = true;
  let showControls = true;
  let showMatrixEditor = true;
  let showParticleSetup = true;
  let showRenderingSettings = true;
  let showMouseInteraction = true;
  let showPhysicsSettings = true;
  let showTypeDistribution = true;
  let showGenerators = true;
  
  // Type distribution data
  let typeCounts: number[] = [];
  let totalParticles = 0;

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
  let unsubscribePhysicsTime: (() => void) | null = null;
  let unsubscribeTypeCounts: (() => void) | null = null;

  // Reactive statement to ensure force matrix is always properly initialized
  $: {
    if (settings.species_count && (!settings.force_matrix || !Array.isArray(settings.force_matrix) || settings.force_matrix.length !== settings.species_count)) {
      // Initialize or resize force matrix to match species count
      const currentMatrix = settings.force_matrix || [];
      const newMatrix: number[][] = [];
      
      for (let i = 0; i < settings.species_count; i++) {
        newMatrix[i] = [];
        for (let j = 0; j < settings.species_count; j++) {
          if (i < currentMatrix.length && currentMatrix[i] && j < currentMatrix[i].length && currentMatrix[i][j] !== undefined) {
            newMatrix[i][j] = currentMatrix[i][j];
          } else {
            // Random values for new entries
            newMatrix[i][j] = (Math.random() - 0.5) * 0.6;
          }
        }
      }
      
      settings.force_matrix = newMatrix;
    }
  }

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
    
    // Update both settings atomically to prevent race conditions
    settings.species_count = newCount;
    settings.force_matrix = newMatrix;
    
    // Force a reactive update by triggering a change
    settings = { ...settings };
    
    try {
      // First update the species count
      await invoke('update_simulation_setting', { 
        settingName: 'species_count', 
        value: newCount 
      });
      
      // Then update the force matrix with the new size
      await invoke('update_simulation_setting', { 
        settingName: 'force_matrix', 
        value: newMatrix 
      });
      
      console.log(`Species count updated to ${newCount}, particles respawned`);
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
    const newCount = Math.max(1000, Math.min(50000, Math.round(value)));
    if (newCount === state.particle_count) return;
    
    console.log(`updateParticleCount called: ${state.particle_count} -> ${newCount}`);

    state.particle_count = newCount;
    
    try {
      console.log(`Sending particle count update to backend: ${newCount}`);
      // Use the new dynamic particle count update
      await invoke('update_simulation_setting', { settingName: 'particle_count', value: newCount });
      
      console.log(`Backend update complete, waiting for GPU operations...`);
      // Add a small delay to ensure GPU operations are complete
      await new Promise(resolve => setTimeout(resolve, 100));
      
      console.log(`Syncing state from backend...`);
      // Sync state from backend to ensure frontend reflects actual backend state
      await syncSettingsFromBackend();
      
      console.log(`Particle count update complete: ${newCount}`);
    } catch (e) {
      console.error('Failed to update particle count:', e);
      // Revert state on error
      await syncSettingsFromBackend();
    }
  }

  // Mouse interaction controls
  async function updateCursorSize(value: number) {
    state.cursor_size = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'cursor_size', value });
    } catch (e) {
      console.error('Failed to update cursor size:', e);
    }
  }

  async function updateCursorStrength(value: number) {
    state.cursor_strength = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'cursor_strength', value });
    } catch (e) {
      console.error('Failed to update cursor strength:', e);
    }
  }

  // Rendering controls
  async function updateTracesEnabled(value: boolean) {
    state.traces_enabled = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'traces_enabled', value });
    } catch (e) {
      console.error('Failed to update traces enabled:', e);
    }
  }

  async function updateTraceFade(value: number) {
    state.trace_fade = value;
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'trace_fade', 
        value: value 
      });
    } catch (e) {
      console.error('Failed to update trace fade:', e);
    }
  }

  async function updateEdgeFadeStrength(value: number) {
    state.edge_fade_strength = value;
    try {
      await invoke('update_simulation_setting', { settingName: 'edge_fade_strength', value });
    } catch (e) {
      console.error('Failed to update edge fade strength:', e);
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
        const oldParticleCount = state.particle_count;
        state = { ...state, ...backendState };
        
        // Extract type distribution data
        if (backendState && typeof backendState === 'object' && 'type_counts' in backendState && Array.isArray(backendState.type_counts)) {
          typeCounts = backendState.type_counts;
          totalParticles = (backendState as any).particle_count || 0;
        }
        
        // Ensure particle_count is properly set from state
        if (backendState && typeof backendState === 'object' && 'particle_count' in backendState) {
          state.particle_count = (backendState as any).particle_count || 15000;
        }
        
        // Sync LUT state from backend
        if (backendState && typeof backendState === 'object') {
          if ('current_lut' in backendState) {
            const backendLut = (backendState as any).current_lut || '';
            // Only update if we don't have a current LUT, or if the backend LUT is different and valid
            if (!state.current_lut || (backendLut && backendLut !== state.current_lut)) {
              // Check if this is a default fallback (bone_reversed) when we had a different LUT
              if (state.current_lut && backendLut.includes('bone') && !state.current_lut.includes('bone')) {
                console.log(`Backend tried to reset LUT from ${state.current_lut} to ${backendLut}, ignoring`);
              } else {
                state.current_lut = backendLut;
                console.log(`Synced LUT from backend: ${state.current_lut}`);
              }
            }
          }
          if ('lut_reversed' in backendState) {
            const newReversed = (backendState as any).lut_reversed || false;
            if (newReversed !== state.lut_reversed) {
              state.lut_reversed = newReversed;
              console.log(`Synced LUT reversed from backend: ${state.lut_reversed}`);
            }
          }
        }
        
        // Log particle count changes for debugging
        if (oldParticleCount !== state.particle_count) {
          console.log(`Frontend particle count updated: ${oldParticleCount} -> ${state.particle_count}`);
        }
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
      console.log('Resetting simulation...');
      await invoke('reset_simulation');
      
      console.log('Reset complete, waiting for GPU operations...');
      // Add a small delay to ensure GPU operations are complete
      await new Promise(resolve => setTimeout(resolve, 100));
      
      console.log('Syncing state from backend...');
      // Sync state from backend to ensure frontend reflects actual backend state
      await syncSettingsFromBackend();
      
      console.log('Simulation reset complete');
    } catch (e) {
      console.error('Failed to reset simulation:', e);
      // Sync state on error to ensure consistency
      await syncSettingsFromBackend();
    }
  }

  async function randomizeMatrix() {
    try {
      // First update the matrix generator setting
      await invoke('update_simulation_setting', { settingName: 'matrix_generator', value: state.matrix_generator });
      
      // Then randomize the matrix using the current generator
      await invoke('randomize_settings');
      await syncSettingsFromBackend();
      
      console.log(`Matrix randomized using ${state.matrix_generator} generator`);
    } catch (e) {
      console.error('Failed to randomize matrix:', e);
    }
  }

  // Camera controls
  let pressedKeys = new Set<string>();
  let animationFrameId: number | null = null;
  
  // Camera movement batching for smoother panning
  let accumulatedDeltaX = 0;
  let accumulatedDeltaY = 0;
  let lastCameraUpdate = 0;
  const CAMERA_UPDATE_INTERVAL = 16; // ~60 FPS

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === '/') {
      event.preventDefault();
      toggleBackendGui();
      return;
    }
    
    // Handle camera controls
    const cameraKeys = ['w', 'a', 's', 'd', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright', 'q', 'e', 'c'];
    if (cameraKeys.includes(event.key.toLowerCase())) {
      event.preventDefault();
      pressedKeys.add(event.key.toLowerCase());
      
      if (animationFrameId === null) {
        animationFrameId = requestAnimationFrame(processCameraMovement);
      }
    }
  }

  function handleKeyUp(event: KeyboardEvent) {
    const cameraKeys = ['w', 'a', 's', 'd', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright', 'q', 'e', 'c'];
    if (cameraKeys.includes(event.key.toLowerCase())) {
      pressedKeys.delete(event.key.toLowerCase());
      
      if (pressedKeys.size === 0 && animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
        
        // Send any remaining accumulated movement
        if (accumulatedDeltaX !== 0 || accumulatedDeltaY !== 0) {
          sendCameraMovement(accumulatedDeltaX, accumulatedDeltaY);
          accumulatedDeltaX = 0;
          accumulatedDeltaY = 0;
        }
        
        invoke('stop_camera_pan').catch(e => 
          console.error('Failed to stop camera pan:', e)
        );
      }
    }
  }

  async function sendCameraMovement(deltaX: number, deltaY: number) {
    try {
      await invoke('pan_camera', { deltaX: deltaX * 0.1, deltaY: -deltaY * 0.1 });
    } catch (e) {
      console.error('Failed to pan camera:', e);
    }
  }

  async function resetCamera() {
    try {
      await invoke('reset_camera');
    } catch (e) {
      console.error('Failed to reset camera:', e);
    }
  }

  async function processCameraMovement(timestamp: number) {
    let deltaX = 0;
    let deltaY = 0;
    let zoomDelta = 0;
    
    if (pressedKeys.has('arrowleft') || pressedKeys.has('a')) deltaX -= 1;
    if (pressedKeys.has('arrowright') || pressedKeys.has('d')) deltaX += 1;
    if (pressedKeys.has('arrowup') || pressedKeys.has('w')) deltaY -= 1;
    if (pressedKeys.has('arrowdown') || pressedKeys.has('s')) deltaY += 1;
    
    // Q/E for zoom in/out
    if (pressedKeys.has('q')) zoomDelta -= 1;
    if (pressedKeys.has('e')) zoomDelta += 1;
    
    // C for camera reset
    if (pressedKeys.has('c')) {
      resetCamera();
    }
    
    // Accumulate camera movement
    if (deltaX !== 0 || deltaY !== 0) {
      accumulatedDeltaX += deltaX;
      accumulatedDeltaY += deltaY;
      
      // Send accumulated movement at regular intervals
      if (timestamp - lastCameraUpdate >= CAMERA_UPDATE_INTERVAL) {
        if (accumulatedDeltaX !== 0 || accumulatedDeltaY !== 0) {
          await sendCameraMovement(accumulatedDeltaX, accumulatedDeltaY);
          accumulatedDeltaX = 0;
          accumulatedDeltaY = 0;
          lastCameraUpdate = timestamp;
        }
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

  let isMousePressed = false;
  let currentMouseButton = 0;

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
    } else if (event.type === 'mousedown') {
      const mouseEvent = event as MouseEvent;
      mouseEvent.preventDefault();
      
      isMousePressed = true;
      currentMouseButton = mouseEvent.button;
      
      // Convert screen coordinates to world coordinates
      const devicePixelRatio = window.devicePixelRatio || 1;
      const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
      const physicalCursorY = mouseEvent.clientY * devicePixelRatio;
      
      // Determine if it's left click (attract) or right click (repel)
      const isAttract = mouseEvent.button === 0; // Left click = attract, right click = repel
      
      console.log(`Mouse ${isAttract ? 'attract' : 'repel'} at screen coords: (${physicalCursorX}, ${physicalCursorY}), raw: (${mouseEvent.clientX}, ${mouseEvent.clientY})`);
      
      invoke('handle_mouse_interaction_screen', {
        screenX: physicalCursorX,
        screenY: physicalCursorY,
        isAttract: isAttract
      }).catch(e => console.error('Failed to handle mouse interaction:', e));
    } else if (event.type === 'mousemove') {
      if (isMousePressed) {
        const mouseEvent = event as MouseEvent;
        mouseEvent.preventDefault();
        
        // Convert screen coordinates to world coordinates
        const devicePixelRatio = window.devicePixelRatio || 1;
        const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
        const physicalCursorY = mouseEvent.clientY * devicePixelRatio;
        
        // Use the same button state as when mouse was first pressed
        const isAttract = currentMouseButton === 0;
        
        invoke('handle_mouse_interaction_screen', {
          screenX: physicalCursorX,
          screenY: physicalCursorY,
          isAttract: isAttract
        }).catch(e => console.error('Failed to handle mouse interaction:', e));
      }
    } else if (event.type === 'mouseup') {
      const mouseEvent = event as MouseEvent;
      mouseEvent.preventDefault();
      
      isMousePressed = false;
      
      // Stop cursor interaction when mouse is released
      // Send special coordinates to indicate mouse release
      invoke('handle_mouse_interaction_screen', {
        screenX: -9999.0,
        screenY: -9999.0,
        isAttract: false
      }).catch(e => console.error('Failed to stop mouse interaction:', e));
    }
  }

  function handleContextMenu(event: MouseEvent) {
    // Prevent right-click context menu from appearing
    event.preventDefault();
  }

  // Generator trigger functions
  async function triggerPositionGenerator() {
    try {
      await invoke('update_simulation_setting', { settingName: 'position_generator', value: state.position_generator });
      console.log(`Triggered position generator: ${state.position_generator}`);
    } catch (e) {
      console.error('Failed to trigger position generator:', e);
    }
  }

  async function triggerTypeGenerator() {
    try {
      await invoke('update_simulation_setting', { settingName: 'type_generator', value: state.type_generator });
      console.log(`Triggered type generator: ${state.type_generator}`);
    } catch (e) {
      console.error('Failed to trigger type generator:', e);
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

  // Lifecycle
  onMount(async () => {
    // Start simulation automatically
    await startSimulation();
    
    // Load initial data
    await Promise.all([
      loadPresets(),
      loadLuts()
    ]);
    
    // Sync settings after LUTs are loaded
    await syncSettingsFromBackend();
    
    // Set up FPS monitoring
    try {
      unsubscribeFps = await listen('fps-update', (event) => {
        fps_display = event.payload as number;
      });
    } catch (e) {
      console.error('Failed to set up FPS listener:', e);
    }
    
    // Set up physics time monitoring
    try {
      unsubscribePhysicsTime = await listen('physics-time-update', (event) => {
        physics_time_avg = event.payload as number;
      });
    } catch (e) {
      console.error('Failed to set up physics time listener:', e);
    }
    
    // Set up type counts monitoring
    try {
      unsubscribeTypeCounts = await listen('type-counts-update', (event) => {
        const data = event.payload as { counts: number[], total: number };
        typeCounts = data.counts;
        totalParticles = data.total;
      });
    } catch (e) {
      console.error('Failed to set up type counts listener:', e);
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
    if (unsubscribePhysicsTime) {
      unsubscribePhysicsTime();
    }
    if (unsubscribeTypeCounts) {
      unsubscribeTypeCounts();
    }
    
    document.removeEventListener('keydown', handleKeyDown);
    document.removeEventListener('keyup', handleKeyUp);
    
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
    }
  });

  $: typePercentages = typeCounts.map(count => totalParticles > 0 ? (count / totalParticles) * 100 : 0);

  async function updateLut(lutName: string) {
    try {
      console.log(`Updating LUT to: ${lutName}`);
      state.current_lut = lutName;
      await invoke('update_simulation_setting', { 
        settingName: 'lut_name', 
        value: lutName 
      });
    } catch (e) {
      console.error('Failed to update LUT:', e);
    }
  }

  async function updateLutReversed(reversed: boolean) {
    try {
      console.log(`Updating LUT reversed to: ${reversed}, current LUT: ${state.current_lut}`);
      const originalLut = state.current_lut;
      state.lut_reversed = reversed;
      
      await invoke('update_simulation_setting', { 
        settingName: 'lut_reversed', 
        value: reversed 
      });
      
      // Ensure the LUT name doesn't get reset - if it changed, restore it
      if (state.current_lut !== originalLut && originalLut) {
        console.log(`LUT was reset from ${originalLut} to ${state.current_lut}, restoring...`);
        state.current_lut = originalLut;
        await invoke('update_simulation_setting', { 
          settingName: 'lut_name', 
          value: originalLut 
        });
      }
    } catch (e) {
      console.error('Failed to update LUT reversed:', e);
    }
  }

  async function updateBackgroundType(colorMode: string) {
    try {
      await invoke('update_simulation_setting', { 
        settingName: 'color_mode', 
        value: colorMode 
      });
    } catch (e) {
      console.error('Failed to update background type:', e);
    }
  }

  async function scaleMatrix(scaleFactor: number) {
    // Ensure force matrix exists and has proper dimensions
    if (!settings.force_matrix || !settings.force_matrix[0] || settings.force_matrix[0].length !== settings.species_count) {
      console.warn('Force matrix not properly initialized, skipping scaling');
      return;
    }
    
    const newMatrix: number[][] = [];
    for (let i = 0; i < settings.species_count; i++) {
      newMatrix[i] = [];
      for (let j = 0; j < settings.species_count; j++) {
        if (i === j) {
          newMatrix[i][j] = settings.force_matrix[i][j]; // Keep diagonal values unchanged
        } else {
          newMatrix[i][j] = Math.max(-1, Math.min(1, settings.force_matrix[i][j] * scaleFactor));
        }
      }
    }
    
    settings.force_matrix = newMatrix;
    
    try {
      await invoke('scale_force_matrix', { scaleFactor });
      console.log(`Matrix scaled by factor: ${scaleFactor}`);
    } catch (e) {
      console.error('Failed to scale force matrix:', e);
    }
  }
</script>

<div class="particle-life-container">
  {#if isSimulationRunning}
    {#if showUI}
    <div class="controls">
      <button class="back-button" on:click={() => dispatch('back')}>
        ← Back to Menu
      </button>
      
      <div class="status">
        <span class="status-indicator running"></span>
        Particle Life Simulation Running
      </div>
      
      <div class="mouse-instructions">
        <span>🖱️ Left click: Attract | Right click: Repel</span>
        <span>📹 WASD/Arrows: Pan | Q/E or Mouse wheel: Zoom</span>
      </div>
    </div>
    {/if}

    <!-- Main UI Controls Panel -->
    {#if showUI}
    <div class="simulation-controls">
    <form on:submit|preventDefault>
      <!-- FPS Display & Status -->
      <fieldset>
        <legend>Status</legend>
        <div class="control-group">
          <span>Running at {fps_display.toFixed(0)} FPS | Physics: {physics_time_avg.toFixed(2)}ms</span>
        </div>
      </fieldset>

      <!-- Quick Controls -->
      <fieldset>
        <legend>Controls</legend>
        <div class="control-group">
          <button type="button" on:click={resetSimulation}>🔄 Reset</button>
          <button type="button" on:click={async () => {
            state.matrix_generator = 'Random';
            await randomizeMatrix();
          }}>🎲 Randomize Matrix</button>
        </div>
      </fieldset>
      <!-- Presets -->
      <fieldset>
        <legend>Presets</legend>
        <div class="control-group">
          <div class="preset-controls">
            <button type="button" on:click={async () => {
              const currentIndex = available_presets.indexOf(current_preset);
              const newIndex = currentIndex > 0 ? currentIndex - 1 : available_presets.length - 1;
              if (available_presets[newIndex]) await updatePreset(available_presets[newIndex]);
            }}>◀</button>
            <select 
              bind:value={current_preset}
              on:change={(e) => updatePreset((e.target as HTMLSelectElement).value)}
            >
              <option value="">Select Preset...</option>
              {#each available_presets as preset}
                <option value={preset}>{preset}</option>
              {/each}
            </select>
            <button type="button" on:click={async () => {
              const currentIndex = available_presets.indexOf(current_preset);
              const newIndex = currentIndex < available_presets.length - 1 ? currentIndex + 1 : 0;
              if (available_presets[newIndex]) await updatePreset(available_presets[newIndex]);
            }}>▶</button>
          </div>
        </div>
        <div class="preset-actions">
          <button type="button" on:click={() => show_save_preset_dialog = true}>Save Current Settings</button>
        </div>
      </fieldset>

      <!-- Interaction Matrix and Particle Setup -->
      <fieldset>
        <legend>Interaction Matrix & Particle Setup</legend>
        <div class="matrix-info">
          <p>Click and drag to edit values. Purple = Repulsion, Blue = Weak, Green = Moderate, Yellow = Strong Attraction</p>
        </div>
        
        <div class="matrix-and-setup-container">
          <div class="matrix-section">
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
                    {@const matrixValue = settings.force_matrix && settings.force_matrix[i] && settings.force_matrix[i][j] !== undefined ? settings.force_matrix[i][j] : 0}
                    <div class="matrix-cell" class:repulsion={matrixValue < -0.3} class:weak={matrixValue >= -0.3 && matrixValue < 0} class:moderate={matrixValue >= 0 && matrixValue < 0.5} class:strong={matrixValue >= 0.5}>
                      {#if settings.force_matrix && settings.force_matrix[i] && settings.force_matrix[i][j] !== undefined}
                        <NumberDragBox
                          value={settings.force_matrix[i][j]}
                          min={-1}
                          max={1}
                          step={0.1}
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
            
            <!-- Matrix Scaling Controls -->
            <div class="matrix-scaling-controls">
              <div class="scaling-buttons">
                <button 
                  type="button" 
                  class="scale-btn scale-down" 
                  on:click={() => scaleMatrix(0.8)}
                  title="Scale down matrix values by 20%"
                >
                  ⬇️ Scale Down
                </button>
                <button 
                  type="button" 
                  class="scale-btn scale-up" 
                  on:click={() => scaleMatrix(1.2)}
                  title="Scale up matrix values by 20%"
                >
                  ⬆️ Scale Up
                </button>
              </div>
              <div class="scaling-info">
                <small>Scaling affects all non-diagonal matrix values</small>
              </div>
            </div>
          </div>

          <div class="setup-section">
            <h4>Particle Setup</h4>
            <div class="control-group">
              <label for="speciesCount">Species Count</label>
              <NumberDragBox
                value={settings.species_count}
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
                value={state.particle_count}
                min={1}
                max={50000}
                step={1000}
                precision={0}
                on:change={(e) => updateParticleCount(e.detail)}
              />
              <small class="warning-text">
                ⚠️ Performance drops significantly above 25,000 particles
              </small>
            </div>
          </div>
        </div>
      </fieldset>


      <!-- Advanced Physics Settings -->
      <fieldset>
        <legend>Physics Settings</legend>
        <div class="physics-controls-grid">
          <div class="control-group">
            <label for="maxForce">Max Force</label>
            <NumberDragBox
              value={settings.max_force}
              min={0.1}
              max={10}
              step={0.01}
              precision={2}
              on:change={(e) => updateSetting('max_force', e.detail)}
            />
          </div>
          <div class="control-group">
            <label for="minDistance">Min Distance</label>
            <NumberDragBox
              value={settings.min_distance}
              min={0.0001}
              max={0.01}
              step={0.0001}
              precision={4}
              on:change={(e) => updateSetting('min_distance', e.detail)}
            />
          </div>
          <div class="control-group">
            <label for="maxDistance">Max Distance</label>
            <NumberDragBox
              value={settings.max_distance}
              min={0.01}
              max={0.2}
              step={0.001}
              precision={3}
              on:change={(e) => updateSetting('max_distance', e.detail)}
            />
          </div>
          <div class="control-group">
            <label for="friction">Friction</label>
            <NumberDragBox
              value={settings.friction}
              min={0.5}
              max={1.0}
              step={0.01}
              precision={2}
              on:change={(e) => updateSetting('friction', e.detail)}
            />
          </div>
          <div class="control-group">
            <label for="forceBeta">Force Beta</label>
            <NumberDragBox
              value={settings.force_beta}
              min={0.1}
              max={0.9}
              step={0.01}
              precision={2}
              on:change={(e) => updateSetting('force_beta', e.detail)}
            />
          </div>
        </div>
      </fieldset>

      <!-- Type Distribution and Generators -->
      <fieldset>
        <legend>Type Distribution & Generators</legend>
        <div class="distribution-generators-container">
          <div class="distribution-section">
            <h4>Type Distribution</h4>
            {#if typeCounts.length > 0}
              {#each typeCounts as count, i}
                <div class="type-distribution-item">
                  <div class="type-info">
                    <span class="type-color" style="background-color: {speciesColors[i]}"></span>
                    <span class="type-label">Type {i + 1}</span>
                    <span class="type-count">{count.toLocaleString()}</span>
                    <span class="type-percentage">({typePercentages[i].toFixed(1)}%)</span>
                  </div>
                  <div class="type-progress">
                    <div class="progress-bar" style="width: {typePercentages[i]}%"></div>
                  </div>
                </div>
              {/each}
            {:else}
              <p class="no-data">No type distribution data available</p>
            {/if}
          </div>

          <div class="generators-section">
            <h4>Generators</h4>
            <div class="control-group">
              <label for="positionGenerator">Position Generator</label>
              <div class="generator-control">
                <select 
                  id="positionGenerator"
                  value={state.position_generator}
                  on:change={(e) => {
                    state.position_generator = (e.target as HTMLSelectElement).value;
                    triggerPositionGenerator();
                  }}
                >
                  <option value="Random">Random</option>
                  <option value="Center">Center</option>
                  <option value="UniformCircle">Uniform Circle</option>
                  <option value="CenteredCircle">Centered Circle</option>
                  <option value="Ring">Ring</option>
                  <option value="RainbowRing">Rainbow Ring</option>
                  <option value="ColorBattle">Color Battle</option>
                  <option value="ColorWheel">Color Wheel</option>
                  <option value="Line">Line</option>
                  <option value="Spiral">Spiral</option>
                  <option value="RainbowSpiral">Rainbow Spiral</option>
                </select>
                <button type="button" on:click={triggerPositionGenerator} class="generator-btn" title="Generate new positions">🔄</button>
              </div>
            </div>
            
            <div class="control-group">
              <label for="typeGenerator">Type Generator</label>
              <div class="generator-control">
                <select 
                  id="typeGenerator"
                  value={state.type_generator}
                  on:change={(e) => {
                    state.type_generator = (e.target as HTMLSelectElement).value;
                    triggerTypeGenerator();
                  }}
                >
                  <option value="Random">Random</option>
                  <option value="Randomize10Percent">Randomize 10%</option>
                  <option value="Slices">Slices</option>
                  <option value="Onion">Onion</option>
                  <option value="Rotate">Rotate</option>
                  <option value="Flip">Flip</option>
                  <option value="MoreOfFirst">More of First</option>
                  <option value="KillStill">Kill Still</option>
                </select>
                <button type="button" on:click={triggerTypeGenerator} class="generator-btn" title="Generate new types">🔄</button>
              </div>
            </div>
          </div>
        </div>
      </fieldset>

      <!-- Mouse Interaction -->
      <fieldset>
        <legend>Mouse Interaction</legend>
        <div class="control-group">
          <label for="cursorSize">Cursor Size</label>
          <input 
            type="range" 
            id="cursorSize"
            value={state.cursor_size}
            min="0.05" 
            max="1.0" 
            step="0.05"
            on:input={(e) => updateCursorSize(parseFloat((e.target as HTMLInputElement).value))}
          />
          <span class="range-value">{state.cursor_size.toFixed(2)}</span>
        </div>
        
        <div class="control-group">
          <label for="cursorStrength">Cursor Strength</label>
          <input 
            type="range" 
            id="cursorStrength"
            value={state.cursor_strength}
            min="0" 
            max="20" 
            step="0.5"
            on:input={(e) => updateCursorStrength(parseFloat((e.target as HTMLInputElement).value))}
          />
          <span class="range-value">{state.cursor_strength.toFixed(1)}</span>
        </div>
      </fieldset>

      <!-- Display Settings -->
      <fieldset>
        <legend>Display Settings</legend>
        <div class="display-controls-grid">
          <div class="control-group">
            <label for="lutSelector">Color Scheme</label>
            <LutSelector 
              bind:available_luts 
              bind:current_lut={state.current_lut}
              bind:reversed={state.lut_reversed}
              on:select={(e) => updateLut(e.detail.name)}
              on:reverse={(e) => updateLutReversed(e.detail.reversed)}
            />
          </div>
          <div class="control-group">
            <label>
              <input 
                type="checkbox" 
                checked={state.traces_enabled}
                on:change={(e) => updateTracesEnabled((e.target as HTMLInputElement).checked)}
              />
              Enable Particle Traces
            </label>
          </div>
          {#if state.traces_enabled}
            <div class="control-group">
              <label for="traceFade">Trace Fade</label>
              <input 
                type="range" 
                id="traceFade"
                value={state.trace_fade}
                min="0" 
                max="1" 
                step="0.01"
                on:input={(e) => updateTraceFade(parseFloat((e.target as HTMLInputElement).value))}
              />
              <span class="range-value">{state.trace_fade.toFixed(2)}</span>
            </div>
          {/if}
          
          <div class="control-group">
            <label for="edgeFade">Edge Fade</label>
            <input 
              type="range" 
              id="edgeFade"
              value={state.edge_fade_strength}
              min="0" 
              max="1" 
              step="0.05"
              on:input={(e) => updateEdgeFadeStrength(parseFloat((e.target as HTMLInputElement).value))}
            />
            <span class="range-value">{state.edge_fade_strength.toFixed(2)}</span>
          </div>
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
          placeholder="Enter preset name..."
          value={new_preset_name}
          on:input={(e) => new_preset_name = (e.target as HTMLInputElement).value}
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
  {/if}
  
  <!-- Mouse overlay for camera interaction (only when simulation is running) -->
  <div 
    class="mouse-overlay"
    on:mousedown={handleMouseEvent}
    on:mouseup={handleMouseEvent}
    on:mousemove={handleMouseEvent}
    on:wheel={handleMouseEvent}
    on:contextmenu={handleContextMenu}
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

  .mouse-instructions {
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.8rem;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
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

  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    background: #f8f9fa;
    color: #333;
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

  .warning-text {
    color: #ff6b6b;
    margin-top: 4px;
    display: block;
    font-size: 0.75rem;
  }


  button {
    padding: 0.5rem 1rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    background: #f8f9fa;
    color: #333;
    cursor: pointer;
    height: 35px;
  }

  button:hover {
    background: #e9ecef;
    color: #222;
  }

  .matrix-info {
    margin-bottom: 1rem;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
  }

  .matrix-info p {
    margin: 0;
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.8rem;
  }

  .matrix-and-setup-container {
    display: flex;
    gap: 2rem;
    align-items: flex-start;
  }

  .matrix-section {
    flex: 1;
  }

  .setup-section {
    flex: 0 0 280px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 1rem;
  }

  .setup-section h4 {
    margin: 0 0 1rem 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1rem;
    font-weight: 600;
  }

  .distribution-generators-container {
    display: flex;
    gap: 2rem;
    align-items: flex-start;
  }

  .distribution-section {
    flex: 1;
  }

  .generators-section {
    flex: 0 0 320px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 1rem;
  }

  .distribution-section h4,
  .generators-section h4 {
    margin: 0 0 1rem 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1rem;
    font-weight: 600;
  }

  .physics-controls-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 1rem;
  }

  .physics-controls-grid .control-group {
    margin-bottom: 0;
  }

  .display-controls-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 1rem;
  }

  .display-controls-grid .control-group {
    margin-bottom: 0;
  }

  .force-matrix {
    display: block;
    max-width: 100%;
    overflow-x: auto;
    margin-bottom: 1rem;
  }

  .matrix-labels {
    display: grid;
    grid-template-columns: 40px repeat(var(--species-count, 4), 55px);
    gap: 2px;
    margin-bottom: 2px;
  }

  .matrix-row {
    display: grid;
    grid-template-columns: 40px repeat(var(--species-count, 4), 55px);
    gap: 2px;
    margin-bottom: 2px;
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
    min-width: 55px;
    max-width: 55px;
    height: 55px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: all 0.2s ease;
  }

  .matrix-cell.repulsion {
    background: rgba(255, 100, 100, 0.2);
    border: 1px solid rgba(255, 100, 100, 0.4);
  }

  .matrix-cell.weak {
    background: rgba(100, 150, 255, 0.2);
    border: 1px solid rgba(100, 150, 255, 0.4);
  }

  .matrix-cell.moderate {
    background: rgba(100, 255, 100, 0.2);
    border: 1px solid rgba(100, 255, 100, 0.4);
  }

  .matrix-cell.strong {
    background: rgba(255, 255, 100, 0.2);
    border: 1px solid rgba(255, 255, 100, 0.4);
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
    min-width: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: transparent;
    border: none;
  }
  
  .matrix-placeholder {
    padding: 8px;
    text-align: center;
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.8rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
  }

  .matrix-legend {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    margin-top: 0.5rem;
  }

  .matrix-legend span {
    padding: 2px 6px;
    border-radius: 3px;
  }

  .matrix-legend .negative {
    background: rgba(255, 100, 100, 0.2);
    color: rgba(255, 100, 100, 0.9);
  }

  .matrix-legend .neutral {
    background: rgba(100, 150, 255, 0.2);
    color: rgba(100, 150, 255, 0.9);
  }

  .matrix-legend .positive {
    background: rgba(100, 255, 100, 0.2);
    color: rgba(100, 255, 100, 0.9);
  }

  /* Matrix Scaling Controls */
  .matrix-scaling-controls {
    margin-top: 1rem;
    padding: 0.75rem;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
  }

  .scaling-buttons {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .scale-btn {
    flex: 1;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: white;
    padding: 0.5rem;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
  }

  .scale-btn:hover {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.3);
  }

  .scale-btn.scale-down {
    background: rgba(255, 100, 100, 0.2);
    border-color: rgba(255, 100, 100, 0.4);
  }

  .scale-btn.scale-down:hover {
    background: rgba(255, 100, 100, 0.3);
    border-color: rgba(255, 100, 100, 0.5);
  }

  .scale-btn.scale-up {
    background: rgba(100, 255, 100, 0.2);
    border-color: rgba(100, 255, 100, 0.4);
  }

  .scale-btn.scale-up:hover {
    background: rgba(100, 255, 100, 0.3);
    border-color: rgba(100, 255, 100, 0.5);
  }

  .scaling-info {
    text-align: center;
    color: rgba(255, 255, 255, 0.6);
    font-size: 0.75rem;
  }

  /* Generator Controls */
  .generator-control {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .generator-control select {
    flex: 1;
  }

  .generator-btn {
    background: rgba(255, 255, 255, 0.15);
    border: 1px solid rgba(255, 255, 255, 0.3);
    color: white;
    padding: 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 1rem;
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .generator-btn:hover {
    background: rgba(255, 255, 255, 0.25);
    border-color: rgba(255, 255, 255, 0.4);
  }

  input[type="range"] {
    width: 100%;
    margin: 0.5rem 0;
  }

  .range-value {
    display: inline-block;
    margin-left: 0.5rem;
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.8rem;
    font-family: monospace;
  }

  /* Type Distribution */
  .type-distribution-item {
    margin-bottom: 0.75rem;
  }

  .type-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .type-color {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.3);
  }

  .type-label {
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.85rem;
    font-weight: 500;
    min-width: 50px;
  }

  .type-count {
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.8rem;
    font-family: monospace;
  }

  .type-percentage {
    color: rgba(255, 255, 255, 0.6);
    font-size: 0.75rem;
  }

  .type-progress {
    height: 6px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background: linear-gradient(90deg, #51cf66, #74c0fc);
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .no-data {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.8rem;
    text-align: center;
    font-style: italic;
  }

  /* Dialog Styles */
  .dialog-backdrop {
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

  .dialog-buttons {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
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