<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NumberDragBox from './components/NumberDragBox.svelte';
  import LutSelector from './components/LutSelector.svelte';
  import InteractivePhysicsDiagram from './components/InteractivePhysicsDiagram.svelte';

  import './particle_life_mode.css';

  const dispatch = createEventDispatcher();

  interface Settings {
    species_count: number;
    force_matrix: number[][];
    max_force: number;
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
    color_mode: string;
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
    lut_reversed: false,
    color_mode: 'Lut',
  };

  // UI state
  let current_preset = '';
  let available_presets: string[] = [];
  let available_luts: string[] = [];
  let show_save_preset_dialog = false;
  let new_preset_name = '';
  let show_physics_diagram = false; // Toggle for expandable physics diagram section
  let fps_display = 0;
  let physics_time_avg = 0;
  let isSimulationRunning = false;
  let isLoading = true;
  
  // Enhanced UI state
  let showUI = true;
  
  // Type distribution data
  let typeCounts: number[] = [];
  let totalParticles = 0;

  // Species colors for UI visualization - will be populated from backend
  let speciesColors: string[] = [];

  // Function to update species colors from backend
  async function updateSpeciesColors() {
    try {
      console.log('Requesting species colors from backend...');
      const colors = await invoke<[number, number, number, number][]>('get_species_colors');
      
      if (colors && colors.length > 0) {
        // Convert from linear RGB to sRGB for proper display in UI
        const linearToSrgb = (linear: number): number => {
          if (linear <= 0.0031308) {
            return linear * 12.92;
          } else {
            return 1.055 * Math.pow(linear, 1.0 / 2.4) - 0.055;
          }
        };

        // In LUT mode, the first color is the background color, so we skip it
        // In non-LUT mode, all colors are species colors
        const isLutMode = state.color_mode === 'Lut';
        const endIndex = isLutMode ? settings.species_count : colors.length;
        const colorsToProcess = colors.slice(0, endIndex);

        speciesColors = colorsToProcess.map(([r, g, b, a]) => {
          const r_srgb = Math.round(linearToSrgb(r) * 255);
          const g_srgb = Math.round(linearToSrgb(g) * 255);
          const b_srgb = Math.round(linearToSrgb(b) * 255);
          return `rgba(${r_srgb}, ${g_srgb}, ${b_srgb}, ${a})`;
        });
        console.log(`Updated species colors from backend (${isLutMode ? 'LUT mode' : 'non-LUT mode'}):`, speciesColors);
      } else {
        console.warn('Received empty species colors, using fallback colors');
        useFallbackColors();
      }
    } catch (e) {
      console.error('Failed to get species colors, using fallback colors:', e);
      useFallbackColors();
    }
  }

  // Function to use fallback colors when species colors can't be retrieved
  function useFallbackColors() {
    speciesColors = [
      'rgb(255,51,51)', // Red
      'rgb(51,255,51)', // Green  
      'rgb(51,51,255)', // Blue
      'rgb(255,255,51)', // Yellow
      'rgb(255,51,255)', // Magenta
      'rgb(51,255,255)', // Cyan
      'rgb(255,153,51)', // Orange
      'rgb(153,51,255)'  // Purple
    ];
  }

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
    
    // Reset type distribution data to prevent stale data display
    typeCounts = [];
    totalParticles = 0;
    
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
      
      // Update species colors after species count change
      await updateSpeciesColors();
      
      // Wait a bit for the backend to process the changes and respawn particles
      await new Promise(resolve => setTimeout(resolve, 200));
      
      // Sync state from backend to get updated type distribution
      await syncSettingsFromBackend();
      
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
      // Update local state first for immediate UI feedback
      switch (settingName) {
        case 'max_force':
          settings.max_force = value;
          break;
        case 'max_distance':
          settings.max_distance = value;
          break;
        case 'force_beta':
          settings.force_beta = value;
          break;
        case 'friction':
          settings.friction = value;
          break;
        case 'wrap_edges':
          settings.wrap_edges = value;
          break;
      }
      
      // Then update backend
      await invoke('update_simulation_setting', { settingName, value });
    } catch (e) {
      console.error(`Failed to update ${settingName}:`, e);
      // On error, sync from backend to restore correct state
      await syncSettingsFromBackend();
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
    try {
      await invoke('update_simulation_setting', { settingName: 'edge_fade_strength', value });
      console.log(`Updated edge fade strength: ${value}`);
    } catch (e) {
      console.error('Failed to update edge fade strength:', e);
    }
  }

  async function updateMatrixGenerator(value: string) {
    try {
      await invoke('update_simulation_setting', { settingName: 'matrix_generator', value });
      console.log(`Updated matrix generator: ${value}`);
    } catch (e) {
      console.error('Failed to update matrix generator:', e);
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

  // Data loading functions
  async function loadPresets() {
    try {
      available_presets = await invoke('get_presets_for_simulation_type', { 
        simulationType: 'particle_life' 
      });
      
      // Set the default preset if available
      if (available_presets.includes('Default')) {
        current_preset = 'Default';
      }
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
          const backendTypeCounts = backendState.type_counts;
          const backendSpeciesCount = (backendState as any).species_count || settings.species_count;
          
          // Validate that type counts array matches species count
          if (backendTypeCounts.length === backendSpeciesCount) {
            typeCounts = backendTypeCounts;
            totalParticles = (backendState as any).particle_count || 0;
            console.log(`Synced type distribution: ${typeCounts.length} types, ${totalParticles} total particles`);
          } else {
            console.warn(`Type counts array length (${backendTypeCounts.length}) doesn't match species count (${backendSpeciesCount}), ignoring`);
            typeCounts = [];
            totalParticles = 0;
          }
        } else {
          // No type counts data available
          typeCounts = [];
          totalParticles = 0;
        }
        
        // Ensure particle_count is properly set from state
        if (backendState && typeof backendState === 'object' && 'particle_count' in backendState) {
          state.particle_count = (backendState as any).particle_count || 15000;
        }
        
        // Sync LUT state from backend
        if (backendState && typeof backendState === 'object') {
          if ('current_lut' in backendState) {
            const backendLut = (backendState as any).current_lut || '';
            // Always sync LUT from backend to ensure consistency
            if (backendLut !== state.current_lut) {
                state.current_lut = backendLut;
                console.log(`Synced LUT from backend: ${state.current_lut}`);
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
        
        // Update species colors after syncing settings from backend
        if (isSimulationRunning) {
          await updateSpeciesColors();
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
      
      // Reset type distribution data to prevent stale data display
      typeCounts = [];
      totalParticles = 0;
      
      // Apply current generator settings before reset
      await invoke('update_simulation_setting', { settingName: 'position_generator', value: state.position_generator });
      await invoke('update_simulation_setting', { settingName: 'type_generator', value: state.type_generator });
      
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
      // Reset type distribution data to prevent stale data display
      typeCounts = [];
      totalParticles = 0;
      
      // First update the matrix generator setting
      await invoke('update_simulation_setting', { settingName: 'matrix_generator', value: state.matrix_generator });
      
      // Then randomize the matrix using the current generator
      await invoke('randomize_settings');
      await syncSettingsFromBackend();
      
      // Update species colors after matrix randomization
      await updateSpeciesColors();
      
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

  async function handleMouseEvent(event: MouseEvent | WheelEvent) {
    if (event.type === 'wheel') {
      const wheelEvent = event as WheelEvent;
      wheelEvent.preventDefault();
      
      const zoomDelta = -wheelEvent.deltaY * 0.001;
      
      try {
        await invoke('zoom_camera_to_cursor', {
          delta: zoomDelta,
          cursorX: wheelEvent.clientX,
          cursorY: wheelEvent.clientY
        });
      } catch (e) {
        console.error('Failed to zoom camera:', e);
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
      
      // Determine if it's left click (attract) or right click (repel)
      const isAttract = mouseEvent.button === 0; // Left click = attract, right click = repel
      
      console.log(`Mouse ${isAttract ? 'attract' : 'repel'} at screen coords: (${physicalCursorX}, ${physicalCursorY}), raw: (${mouseEvent.clientX}, ${mouseEvent.clientY})`);
      
      try {
        await invoke('handle_mouse_interaction_screen', {
          screenX: physicalCursorX,
          screenY: physicalCursorY,
          isAttract: isAttract
        });
      } catch (e) {
        console.error('Failed to handle mouse interaction:', e);
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
        const isAttract = currentMouseButton === 0;
        
        try {
          await invoke('handle_mouse_interaction_screen', {
            screenX: physicalCursorX,
            screenY: physicalCursorY,
            isAttract: isAttract
          });
        } catch (e) {
          console.error('Failed to handle mouse interaction:', e);
        }
      }
    } else if (event.type === 'mouseup') {
      const mouseEvent = event as MouseEvent;
      mouseEvent.preventDefault();
      
      isMousePressed = false;
      
      // Stop cursor interaction when mouse is released
      // Send special coordinates to indicate mouse release
      try {
        await invoke('handle_mouse_interaction_screen', {
          screenX: -9999.0,
          screenY: -9999.0,
          isAttract: false
        });
      } catch (e) {
        console.error('Failed to stop mouse interaction:', e);
      }
    }
  }

  function handleContextMenu(event: MouseEvent) {
    // Prevent right-click context menu from appearing
    event.preventDefault();
  }

  // Generator update functions (local state only)
  function updatePositionGenerator(value: string) {
    state.position_generator = value;
    console.log(`Position generator set to: ${value} (will apply on next reset)`);
  }

  function updateTypeGenerator(value: string) {
    state.type_generator = value;
    console.log(`Type generator set to: ${value} (will apply on next reset)`);
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
    try {
      // Start simulation automatically
      await startSimulation();
      
      // Load initial data
      await Promise.all([
        loadPresets(),
        loadLuts()
      ]);
      
      // Set the default preset if available and not already set
      if (available_presets.includes('Default') && !current_preset) {
        current_preset = 'Default';
      }
      
      // Sync settings after LUTs are loaded
      await syncSettingsFromBackend();
      
      // Only update species colors after simulation is running
      if (isSimulationRunning) {
        await updateSpeciesColors();
      }
      
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
    } catch (e) {
      console.error('Failed to initialize simulation:', e);
    } finally {
      isLoading = false;
    }
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

  // Reactive statement to ensure typeCounts array matches current species count
  $: if (settings.species_count && typeCounts.length !== settings.species_count) {
    // If typeCounts array doesn't match species count, reset it
    if (typeCounts.length > 0) {
      console.log(`Type counts array length (${typeCounts.length}) doesn't match species count (${settings.species_count}), resetting`);
      typeCounts = [];
      totalParticles = 0;
    }
  }

  // Reactive statement to update species colors when LUT changes
  $: if (isSimulationRunning && (state.current_lut || state.lut_reversed !== undefined)) {
    // Add a small delay to ensure backend has processed the LUT change
    setTimeout(() => {
      updateSpeciesColors();
    }, 50);
  }

  // Reactive statement to update species colors when species count changes
  $: if (isSimulationRunning && settings.species_count) {
    // Add a small delay to ensure backend has processed the species count change
    setTimeout(() => {
      updateSpeciesColors();
    }, 100);
  }

  // Reactive statement to update species colors when simulation starts
  $: if (isSimulationRunning && !isLoading) {
    // Update colors when simulation starts
    setTimeout(() => {
      updateSpeciesColors();
    }, 200);
  }

  async function updateLut(lutName: string) {
    try {
      console.log(`Updating LUT to: ${lutName}`);
      state.current_lut = lutName;
      await invoke('update_simulation_setting', { 
        settingName: 'lut_name', 
        value: lutName 
      });
      
      // Immediately update species colors after LUT change
      await updateSpeciesColors();
    } catch (e) {
      console.error('Failed to update LUT:', e);
    }
  }

  async function updateLutReversed(reversed: boolean) {
    try {
      console.log(`Updating LUT reversed to: ${reversed}, current LUT: ${state.current_lut}`);
      state.lut_reversed = reversed;
      
      await invoke('update_simulation_setting', { 
        settingName: 'lut_reversed', 
        value: reversed 
      });
      
      // Immediately update species colors after LUT change
      await updateSpeciesColors();
    } catch (e) {
      console.error('Failed to update LUT reversed:', e);
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

  async function flipMatrixHorizontal() {
    try {
      await invoke('flip_force_matrix_horizontal');
      // Sync settings from backend to update the UI
      await syncSettingsFromBackend();
      console.log('Matrix flipped horizontally');
    } catch (e) {
      console.error('Failed to flip force matrix horizontally:', e);
    }
  }

  async function flipMatrixVertical() {
    try {
      await invoke('flip_force_matrix_vertical');
      // Sync settings from backend to update the UI
      await syncSettingsFromBackend();
      console.log('Matrix flipped vertically');
    } catch (e) {
      console.error('Failed to flip force matrix vertically:', e);
    }
  }

  async function rotateMatrixClockwise() {
    try {
      await invoke('rotate_force_matrix_clockwise');
      // Sync settings from backend to update the UI
      await syncSettingsFromBackend();
      console.log('Matrix rotated clockwise');
    } catch (e) {
      console.error('Failed to rotate force matrix clockwise:', e);
    }
  }

  async function rotateMatrixCounterclockwise() {
    try {
      await invoke('rotate_force_matrix_counterclockwise');
      // Sync settings from backend to update the UI
      await syncSettingsFromBackend();
      console.log('Matrix rotated counterclockwise');
    } catch (e) {
      console.error('Failed to rotate force matrix counterclockwise:', e);
    }
  }

  async function shiftMatrixLeft() {
    try {
      await invoke('shift_force_matrix_left');
      // Sync settings from backend to update the UI
      await syncSettingsFromBackend();
      console.log('Matrix shifted left');
    } catch (e) {
      console.error('Failed to shift force matrix left:', e);
    }
  }

  async function shiftMatrixRight() {
    try {
      await invoke('shift_force_matrix_right');
      // Sync settings from backend to update the UI
      await syncSettingsFromBackend();
      console.log('Matrix shifted right');
    } catch (e) {
      console.error('Failed to shift force matrix right:', e);
    }
  }

  async function shiftMatrixUp() {
    try {
      await invoke('shift_force_matrix_up');
      // Sync settings from backend to update the UI
      await syncSettingsFromBackend();
      console.log('Matrix shifted up');
    } catch (e) {
      console.error('Failed to shift force matrix up:', e);
    }
  }

  async function shiftMatrixDown() {
    try {
      await invoke('shift_force_matrix_down');
      // Sync settings from backend to update the UI
      await syncSettingsFromBackend();
      console.log('Matrix shifted down');
    } catch (e) {
      console.error('Failed to shift force matrix down:', e);
    }
  }

  function matrixValueIsNeutral(value: number) {
    return Math.abs(value) <= 0.1;
  }

  function matrixValueIsWeak(value: number) {
    return (value > 0.1 && value <= 0.3) || (value < -0.1 && value >= -0.3);
  }

  function matrixValueIsModerate(value: number) {
    return (value > 0.3 && value <= 0.5) || (value < -0.3 && value >= -0.5);
  }

  function matrixValueIsStrong(value: number) {
    return value > 0.5 || value < -0.5;
  }

  async function updateColorMode(value: string) {
    try {
      console.log(`Updating color mode to: ${value}`);
      state.color_mode = value;
      await invoke('update_simulation_setting', { 
        settingName: 'color_mode', 
        value: value 
      });
      
      // Immediately update species colors after color mode change
      await updateSpeciesColors();
    } catch (e) {
      console.error('Failed to update color mode:', e);
    }
  }
</script>

<div class="particle-life-container">
  {#if isLoading}
    <div class="loading-screen">
      <div class="loading-content">
        <div class="loading-spinner"></div>
        <h2>Loading Particle Life Simulation...</h2>
        <p>Initializing GPU resources and starting simulation</p>
      </div>
    </div>
  {:else if isSimulationRunning}
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
          <button type="button" on:click={resetSimulation}>🔄 Reset Particles</button>
          <div class="matrix-controls">
            <button type="button" on:click={async () => {
              await randomizeMatrix();
            }}>🎲 Randomize Matrix</button>
            <select 
              id="matrix-generator"
              bind:value={state.matrix_generator}
              on:change={(e) => updateMatrixGenerator((e.target as HTMLSelectElement).value)}
            >
              <option value="Random">Random</option>
              <option value="Symmetry">Symmetry</option>
              <option value="Chains">Chains</option>
              <option value="Chains2">Chains2</option>
              <option value="Chains3">Chains3</option>
              <option value="Snakes">Snakes</option>
              <option value="Zero">Zero</option>
              <option value="PredatorPrey">PredatorPrey</option>
              <option value="Symbiosis">Symbiosis</option>
              <option value="Territorial">Territorial</option>
              <option value="Magnetic">Magnetic</option>
              <option value="Crystal">Crystal</option>
              <option value="Wave">Wave</option>
              <option value="Hierarchy">Hierarchy</option>
              <option value="Clique">Clique</option>
              <option value="AntiClique">AntiClique</option>
              <option value="Fibonacci">Fibonacci</option>
              <option value="Prime">Prime</option>
              <option value="Fractal">Fractal</option>
              <option value="RockPaperScissors">RockPaperScissors</option>
              <option value="Cooperation">Cooperation</option>
              <option value="Competition">Competition</option>
            </select>
          </div>
          <div class="generator-controls">
            <div class="generator-control">
              <label for="position-generator">Position:</label>
              <select 
                id="position-generator"
                value={state.position_generator}
                on:change={(e) => {
                  updatePositionGenerator((e.target as HTMLSelectElement).value);
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
            </div>
            <div class="generator-control">
              <label for="type-generator">Type:</label>
              <select 
                id="type-generator"
                value={state.type_generator}
                on:change={(e) => {
                  updateTypeGenerator((e.target as HTMLSelectElement).value);
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
            </div>
          </div>
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

        
        <div class="control-group">
          <div class="matrix-info">
            <p>Click and drag to edit values.</p>
          </div>

          <label for="particleCount">Particle Count</label>
          <NumberDragBox
            value={state.particle_count}
            min={1}
            max={50000}
            step={1000}
            precision={0}
            on:change={(e) => updateParticleCount(e.detail)}
          />

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
                    <div class="matrix-cell"
                    class:repulsion={matrixValue < 0.0}
                    class:neutral={matrixValueIsNeutral(matrixValue)}
                    class:weak={matrixValueIsWeak(matrixValue)}
                    class:moderate={matrixValueIsModerate(matrixValue)}
                    class:strong={matrixValueIsStrong(matrixValue)}
                    >
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
            
            <!-- Matrix Transformation Controls -->
            <div class="matrix-scaling-controls">
              <div class="icon-button-pair">
                <button 
                  type="button" 
                  class="icon-btn scale-down" 
                  on:click={() => scaleMatrix(0.8)}
                  title="Scale down matrix values by 20%"
                >
                  ⬇↓
                </button>
                <button 
                  type="button" 
                  class="icon-btn scale-up" 
                  on:click={() => scaleMatrix(1.2)}
                  title="Scale up matrix values by 20%"
                >
                  ↑⬆
                </button>
              </div>
              
              <!-- Transformation Controls -->
              <div class="icon-transformation-grid">
                <!-- Rotation Pair -->
                <div class="icon-button-pair">
                  <button 
                    type="button" 
                    class="icon-btn" 
                    on:click={rotateMatrixCounterclockwise}
                    title="Rotate matrix anticlockwise"
                  >
                    ↺
                  </button>
                  <button 
                    type="button" 
                    class="icon-btn" 
                    on:click={rotateMatrixClockwise}
                    title="Rotate matrix clockwise"
                  >
                    ↻
                  </button>
                </div>
                
                <!-- Flip Pair -->
                <div class="icon-button-pair">
                  <button 
                    type="button" 
                    class="icon-btn" 
                    on:click={flipMatrixHorizontal}
                    title="Flip matrix horizontally"
                  >
                    ↔
                  </button>
                  <button 
                    type="button" 
                    class="icon-btn" 
                    on:click={flipMatrixVertical}
                    title="Flip matrix vertically"
                  >
                    ↕
                  </button>
                </div>
                
                <!-- Horizontal Shift Pair -->
                <div class="icon-button-pair">
                  <button 
                    type="button" 
                    class="icon-btn" 
                    on:click={shiftMatrixLeft}
                    title="Shift matrix left"
                  >
                    ←
                  </button>
                  <button 
                    type="button" 
                    class="icon-btn" 
                    on:click={shiftMatrixRight}
                    title="Shift matrix right"
                  >
                    →
                  </button>
                </div>
                
                <!-- Vertical Shift Pair -->
                <div class="icon-button-pair">
                  <button 
                    type="button" 
                    class="icon-btn" 
                    on:click={shiftMatrixUp}
                    title="Shift matrix up"
                  >
                    ↑
                  </button>
                  <button 
                    type="button" 
                    class="icon-btn" 
                    on:click={shiftMatrixDown}
                    title="Shift matrix down"
                  >
                    ↓
                  </button>
                </div>
              </div>
              
              <div class="scaling-info">
                <small>Transformations preserve diagonal (self-repulsion) values</small>
              </div>
            </div>
          </div>
        </div>
      </fieldset>


      <!-- Physics Equation Visualization -->
      <fieldset>
        <legend>
          <button 
            type="button" 
            class="fieldset-toggle"
            on:click={() => show_physics_diagram = !show_physics_diagram}
          >
            {show_physics_diagram ? '▼' : '▶'} Physics
          </button>
        </legend>
        
        {#if show_physics_diagram}
          <div class="diagram-content">
            <InteractivePhysicsDiagram 
              maxForce={settings.max_force}
              maxDistance={settings.max_distance}
              forceBeta={settings.force_beta}
              friction={settings.friction}
              on:update={(e) => updateSetting(e.detail.setting, e.detail.value)}
            />
          </div>
        {/if}
      </fieldset>

      <!-- Type Distribution -->
      <fieldset>
        <legend>Type Distribution</legend>
        <div class="distribution-section">
          {#if typeCounts.length > 0 && typeCounts.length === settings.species_count}
            {#each typeCounts as count, i}
              <div class="type-distribution-item">
                <div class="type-info">
                  <span class="type-color" style="background-color: {speciesColors[i] || '#ffffff'}"></span>
                  <span class="type-label">Type {i + 1}</span>
                  <span class="type-count">{count.toLocaleString()}</span>
                  <span class="type-percentage">({typePercentages[i].toFixed(1)}%)</span>
                </div>
                <div class="type-progress">
                  <div class="progress-bar" style="width: {typePercentages[i]}%; background-color: {speciesColors[i] || '#74c0fc'}"></div>
                </div>
              </div>
            {/each}
          {:else if typeCounts.length > 0 && typeCounts.length !== settings.species_count}
            <p class="no-data">Type distribution data mismatch: got {typeCounts.length} types for {settings.species_count} species</p>
          {:else}
            <p class="no-data">No type distribution data available</p>
          {/if}
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
            <label for="colorModeSelector">Background Mode</label>
            <select id="colorModeSelector" bind:value={state.color_mode} on:change={(e) => updateColorMode((e.target as HTMLSelectElement).value)}>
              <option value="Lut">Color Scheme (LUT)</option>
              <option value="Gray18">Gray 18%</option>
              <option value="White">White</option>
              <option value="Black">Black</option>
            </select>
          </div>
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

  /* Loading Screen Styles */
  .loading-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: linear-gradient(135deg, #1a1a2e, #16213e, #0f3460);
    color: white;
  }

  .loading-content {
    text-align: center;
    max-width: 400px;
    padding: 2rem;
  }

  .loading-spinner {
    width: 60px;
    height: 60px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid #51cf66;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 2rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .loading-content h2 {
    margin: 0 0 1rem 0;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .loading-content p {
    margin: 0;
    color: rgba(255, 255, 255, 0.7);
    font-size: 1rem;
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
    border: 1px solid rgba(255, 255, 255, 0.2);
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
    display: flex;
    gap: 1rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .matrix-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .matrix-controls select {
    flex: 1;
    min-width: 150px;
  }

  .matrix-controls button {
    white-space: nowrap;
  }

  .generator-controls {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  .generator-control {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .generator-control label {
    margin: 0;
    white-space: nowrap;
    font-size: 0.9rem;
    color: rgba(255, 255, 255, 0.8);
  }

  .generator-control select {
    flex: 1;
    min-width: 120px;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
    color: rgba(255, 255, 255, 0.8);
  }

  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.5);
    color: rgba(255, 255, 255, 0.9);
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

  button {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    height: 35px;
  }

  button:hover {
    background: rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 1);
  }

  .matrix-info {
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

  .distribution-section {
    flex: 1;
  }

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
  }

  .progress-bar {
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .no-data {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.8rem;
    text-align: center;
    font-style: italic;
  }

  /* Force Matrix Styles */
  .force-matrix {
    display: grid;
    grid-template-columns: 40px repeat(var(--species-count), 60px);
    grid-template-rows: 40px repeat(var(--species-count), 60px);
    gap: 2px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    padding: 8px;
    margin-bottom: 1rem;
    max-width: 100%;
    overflow-x: auto;
  }

  .matrix-labels {
    display: contents;
  }

  .corner {
    grid-column: 1;
    grid-row: 1;
  }

  .col-label {
    grid-row: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.9);
    padding: 4px;
  }

  .row-label {
    grid-column: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.9);
    padding: 4px;
  }

  .matrix-row {
    display: contents;
  }

  .matrix-cell {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    transition: all 0.2s ease;
    width: 60px;
    height: 60px;
  }

 .matrix-cell.repulsion.weak {
    background: rgb(59, 130, 246);
    border-color: rgb(59, 130, 246);
  }

  .matrix-cell.repulsion.moderate {
    background: rgb(37, 99, 235);
    border-color: rgb(37, 99, 235);
  }

  .matrix-cell.repulsion.strong {
    background: rgb(29, 78, 216);
    border-color: rgb(29, 78, 216);
  }

  .matrix-cell.weak {
    background: rgb(239, 68, 68);
    border-color: rgb(239, 68, 68);
  }

  .matrix-cell.moderate {
    background: rgb(220, 38, 38);
    border-color: rgb(220, 38, 38);
  }

  .matrix-cell.strong {
    background: rgb(185, 28, 28);
    border-color: rgb(185, 28, 28);
  }

  .matrix-cell.neutral {
    background: rgb(138, 138, 138);
    border-color: rgb(138, 138, 138);
  }

  .matrix-placeholder {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.8rem;
    font-family: monospace;
  }

  .matrix-legend {
    display: flex;
    gap: 1rem;
    justify-content: center;
    margin-bottom: 1rem;
    font-size: 0.8rem;
    flex-wrap: wrap;
  }

  .matrix-legend span {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .matrix-legend .negative {
    color: #ea3333;
  }

  .matrix-legend .neutral {
    color: #a4a4a4;
  }

  .matrix-legend .positive {
    color: #22c55e;
  }

  .matrix-scaling-controls {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: center;
  }

  .icon-btn.scale-down {
    background: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.4);
  }

  .icon-btn.scale-down:hover {
    background: rgba(239, 68, 68, 0.4);
    border-color: rgba(239, 68, 68, 0.6);
  }

  .icon-btn.scale-up {
    background: rgba(34, 197, 94, 0.2);
    border-color: rgba(34, 197, 94, 0.4);
  }

  .icon-btn.scale-up:hover {
    background: rgba(34, 197, 94, 0.4);
    border-color: rgba(34, 197, 94, 0.6);
  }

  .icon-transformation-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .icon-button-pair {
    display: flex;
    gap: 2px;
    justify-content: center;
    align-items: center;
  }

  .icon-btn {
    width: 32px;
    height: 32px;
    padding: 0;
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.2);
    color: white;
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  .icon-btn:hover {
    background: rgba(59, 130, 246, 0.4);
    border-color: rgba(59, 130, 246, 0.6);
    transform: scale(1.1);
  }

  .icon-btn:active {
    transform: scale(0.95);
  }

  .scaling-info {
    text-align: center;
  }

  .scaling-info small {
    color: rgba(255, 255, 255, 0.6);
    font-size: 0.75rem;
  }

  .physics-controls-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  /* Responsive matrix sizing for larger species counts */
  @media (max-width: 1200px) {
    .force-matrix {
      grid-template-columns: 35px repeat(var(--species-count), 50px);
      grid-template-rows: 35px repeat(var(--species-count), 50px);
    }
    
    .matrix-cell {
      width: 50px;
      height: 50px;
    }
    
    .col-label, .row-label {
      font-size: 0.75rem;
    }
  }

  @media (max-width: 900px) {
    .force-matrix {
      grid-template-columns: 30px repeat(var(--species-count), 40px);
      grid-template-rows: 30px repeat(var(--species-count), 40px);
    }
    
    .matrix-cell {
      width: 40px;
      height: 40px;
    }
    
    .col-label, .row-label {
      font-size: 0.7rem;
    }
  }

  @media (max-width: 600px) {
    .matrix-and-setup-container {
      flex-direction: column;
      gap: 1rem;
    }

    .force-matrix {
      grid-template-columns: 25px repeat(var(--species-count), 35px);
      grid-template-rows: 25px repeat(var(--species-count), 35px);
    }
    
    .matrix-cell {
      width: 35px;
      height: 35px;
    }
    
    .col-label, .row-label {
      font-size: 0.65rem;
    }
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