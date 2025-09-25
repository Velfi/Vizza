<SimulationLayout
    simulationName="Primordial Particles"
    running={isSimulationRunning}
    loading={isLoading || !settings || !state || !isInitialized}
    {showUI}
    currentFps={fps_display}
    {controlsVisible}
    {menuPosition}
    on:back={() => dispatch('back')}
    on:toggleUI={toggleBackendGui}
    on:pause={pauseSimulation}
    on:resume={resumeSimulation}
    on:userInteraction={() => autoHideManager?.handleUserInteraction()}
    on:mouseEvent={handleMouseEvent}
>
    {#if settings && state}
        <form on:submit|preventDefault>
            <!-- About this simulation -->
            <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
                <p>
                    Primordial Particles is a simulation based on the research paper "How a
                    life-like system emerges from a simplistic particle motion law" published in
                    Nature Scientific Reports. It demonstrates how complex, life-like behaviors can
                    emerge from simple particle interaction rules.
                </p>
                <p>
                    Each particle follows a simple motion law: Δφ = α + β × N, where Δφ is the
                    change in heading, α is a fixed rotation parameter, β is a proportional rotation
                    parameter, and N is the number of neighboring particles within the interaction
                    radius.
                </p>
                <p>
                    This simple rule can lead to the emergence of self-organizing structures,
                    protocells, and even life-like behaviors including growth, reproduction, and
                    complex ecosystem dynamics. The simulation explores how order can spontaneously
                    arise from chaos.
                </p>
            </CollapsibleFieldset>

            <!-- Presets -->
            <PresetFieldset
                availablePresets={available_presets}
                bind:currentPreset={current_preset}
                placeholder="Select Preset..."
                on:presetChange={({ detail }) => updatePreset(detail.value)}
                on:presetSave={({ detail }) => savePreset(detail.name)}
            />

            <!-- Display Settings -->
            <fieldset>
                <legend>Display Settings</legend>
                <div class="control-group">
                    <label for="particleSize">Particle Size</label>
                    <NumberDragBox
                        value={state.particle_size}
                        min={0.001}
                        max={0.01}
                        step={0.0001}
                        precision={4}
                        on:change={({ detail }) => updateParticleSize(detail)}
                    />
                </div>
                <div class="control-group">
                    <label for="colorSchemeSelector">Color Scheme</label>
                    <ColorSchemeSelector
                        bind:available_color_schemes
                        bind:current_color_scheme={state.current_color_scheme}
                        bind:reversed={state.color_scheme_reversed}
                        on:select={(e) => updateColorScheme(e.detail.name)}
                        on:reverse={(e) => updateColorSchemeReversed(e.detail.reversed)}
                    />
                </div>
                <div class="control-group">
                    <label for="backgroundColorMode">Background Color Mode</label>
                    <Selector
                        options={['Black', 'White', 'Gray18', 'Color Scheme']}
                        value={state.background_color_mode || 'Black'}
                        on:change={({ detail }) => updateBackgroundColorMode(detail.value)}
                    />
                </div>
                <div class="control-group">
                    <label for="foregroundColorMode">Particle Color Mode</label>
                    <Selector
                        options={['Heading', 'Density', 'Random', 'Velocity']}
                        value={state.foreground_color_mode}
                        on:change={({ detail }) => updateForegroundColorMode(detail.value)}
                    />
                </div>
                <div class="control-group">
                    <label>
                        <input
                            type="checkbox"
                            checked={state.traces_enabled || false}
                            on:change={(e) =>
                                updateTracesEnabled((e.target as HTMLInputElement).checked)}
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
                            value={state.trace_fade || 0.5}
                            min="0"
                            max="1"
                            step="0.01"
                            on:input={(e) =>
                                updateTraceFade(parseFloat((e.target as HTMLInputElement).value))}
                        />
                        <span class="range-value">{(state.trace_fade || 0.5).toFixed(2)}</span>
                    </div>
                    <div class="control-group">
                        <Button
                            variant="warning"
                            on:click={clearTrails}
                            title="Clear all particle trails"
                        >
                            Clear Trails
                        </Button>
                    </div>
                {/if}
            </fieldset>

            <!-- Post Processing -->
            <PostProcessingMenu simulationType="primordial_particles" />

            <!-- Controls -->
            <ControlsPanel
                mouseInteractionText="🖱️ Mouse: Fling particles | Scroll: Zoom"
                cursorSize={state?.cursor_size}
                cursorStrength={state?.cursor_strength}
                on:cursorSizeChange={(e) => updateCursorSize(e.detail)}
                on:cursorStrengthChange={(e) => updateCursorStrength(e.detail)}
                on:navigate={(e) => dispatch('navigate', e.detail)}
            />

            <!-- Combined Settings -->
            <fieldset>
                <legend>Settings</legend>

                <!-- General Settings -->
                <div class="settings-section">
                    <div class="control-group">
                        <Button variant="primary" on:click={resetSimulation}
                            >Regenerate Particles</Button
                        >
                        <Button
                            variant="warning"
                            type="button"
                            on:click={async () => {
                                try {
                                    await invoke('randomize_settings');
                                    await syncSettingsFromBackend();
                                    console.log('Settings randomized successfully');
                                } catch (e) {
                                    console.error('Failed to randomize settings:', e);
                                }
                            }}>🎲 Randomize Settings</Button
                        >
                    </div>
                </div>

                <!-- Particle Configuration -->
                <div class="settings-section">
                    <h3 class="section-header">Particle Configuration</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="positionGenerator"
                                >Position Generator:</label
                            >
                            <ButtonSelect
                                value={getPositionGeneratorString(selectedPositionGenerator)}
                                options={[
                                    { value: 'Random', label: 'Random', buttonAction: 'randomize' },
                                    { value: 'Center', label: 'Center', buttonAction: 'randomize' },
                                    {
                                        value: 'UniformCircle',
                                        label: 'Uniform Circle',
                                        buttonAction: 'randomize',
                                    },
                                    {
                                        value: 'CenteredCircle',
                                        label: 'Centered Circle',
                                        buttonAction: 'randomize',
                                    },
                                    { value: 'Ring', label: 'Ring', buttonAction: 'randomize' },
                                    { value: 'Line', label: 'Line', buttonAction: 'randomize' },
                                    { value: 'Spiral', label: 'Spiral', buttonAction: 'randomize' },
                                ]}
                                buttonText="Reset Particles"
                                placeholder="Select position generator..."
                                on:change={async (e) => {
                                    // Only update the local selection, don't apply to simulation yet
                                    selectedPositionGenerator = getPositionGeneratorNumber(
                                        e.detail.value
                                    );
                                }}
                                on:buttonclick={async () => {
                                    try {
                                        // Apply the selected position generator to the simulation
                                        await updatePositionGenerator(selectedPositionGenerator);
                                        // Then reset the particles
                                        await invoke('reset_runtime_state');
                                        console.log('Particles reset via ButtonSelect');
                                    } catch (err) {
                                        console.error('Failed to reset particles:', err);
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="particleCount">Particle Count:</label>
                            <NumberDragBox
                                value={state?.particle_count || 10000}
                                min={1000}
                                max={100000}
                                step={1000}
                                precision={0}
                                on:change={({ detail }) => updateParticleCount(detail)}
                            />
                        </div>
                    </div>
                </div>

                <!-- Physics Parameters -->
                <div class="settings-section">
                    <h3 class="section-header">Physics Parameters</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="alpha">Alpha (Fixed Rotation):</label>
                            <NumberDragBox
                                value={settings.alpha}
                                min={-180}
                                max={180}
                                step={1}
                                precision={1}
                                unit="°"
                                on:change={({ detail }) => updateAlpha(detail)}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="beta"
                                >Beta (Proportional Rotation):</label
                            >
                            <NumberDragBox
                                value={settings.beta}
                                min={-60}
                                max={60}
                                step={1}
                                precision={1}
                                on:change={({ detail }) => updateBeta(detail)}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="velocity">Velocity:</label>
                            <NumberDragBox
                                value={settings.velocity}
                                min={0.1}
                                max={2.0}
                                step={0.1}
                                precision={1}
                                on:change={({ detail }) => updateVelocity(detail)}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="radius">Interaction Radius:</label>
                            <NumberDragBox
                                value={settings.radius}
                                min={0.005}
                                max={0.1}
                                step={0.001}
                                precision={3}
                                on:change={({ detail }) => updateRadius(detail)}
                            />
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">
                                <input
                                    type="checkbox"
                                    checked={settings.wrap_edges}
                                    on:change={(e) =>
                                        updateWrapEdges((e.target as HTMLInputElement).checked)}
                                />
                                Wrap Edges
                            </span>
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
    import ButtonSelect from './components/inputs/ButtonSelect.svelte';
    import Selector from './components/inputs/Selector.svelte';
    import ColorSchemeSelector from './components/shared/ColorSchemeSelector.svelte';
    import SimulationLayout from './components/shared/SimulationLayout.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
    import PresetFieldset from './components/shared/PresetFieldset.svelte';
    import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
    import ControlsPanel from './components/shared/ControlsPanel.svelte';
    import { AutoHideManager, createAutoHideEventListeners } from './utils/autoHide';
    import './shared-theme.css';

    const dispatch = createEventDispatcher();

    export let menuPosition: string = 'middle';
    export let autoHideDelay: number = 3000;

    interface Settings {
        alpha: number;
        beta: number;
        velocity: number;
        radius: number;
        wrap_edges: boolean;
    }

    interface State {
        particle_count: number;
        dt: number;
        position_generator: number;
        random_seed: number;
        particle_size: number;
        background_color: [number, number, number];
        color_scheme: number;
        base_color: [number, number, number];
        current_color_scheme: string;
        color_scheme_reversed: boolean;
        background_color_mode: string;
        foreground_color_mode: string;
        traces_enabled: boolean;
        trace_fade: number;
        cursor_size: number;
        cursor_strength: number;
    }

    // Helper functions for position generator conversion
    function getPositionGeneratorString(number: number): string {
        const generators = [
            'Random',
            'Center',
            'UniformCircle',
            'CenteredCircle',
            'Ring',
            'Line',
            'Spiral',
        ];
        return generators[number] || 'Random';
    }

    function getPositionGeneratorNumber(string: string): number {
        const generators = [
            'Random',
            'Center',
            'UniformCircle',
            'CenteredCircle',
            'Ring',
            'Line',
            'Spiral',
        ];
        return generators.indexOf(string);
    }

    // Simulation state
    let settings: Settings | undefined = undefined;
    let state: State | undefined = undefined;

    // Local UI state for position generator selection (not applied until reset)
    let selectedPositionGenerator: number = 0;

    // UI state
    let current_preset = '';
    let available_presets: string[] = [];
    let available_color_schemes: string[] = [];

    let show_about_section = false;
    let fps_display = 0;
    let isSimulationRunning = false;
    let isLoading = true;
    let isInitialized = false;

    // Enhanced UI state
    let showUI = true;
    let controlsVisible = true;

    // Mouse interaction state
    let isMousePressed = false;
    let currentMouseButton = 0;

    // Auto-hide manager
    let autoHideManager: AutoHideManager;
    let eventListeners: { add: () => void; remove: () => void };

    // Event listeners
    let unsubscribeFps: (() => void) | null = null;
    let unsubscribeSimulationInitialized: (() => void) | null = null;
    let unsubscribeSimulationResumed: (() => void) | null = null;

    // Settings update functions
    async function updateParticleCount(value: number) {
        if (!state) return;

        const newCount = Math.max(1000, Math.min(100000, Math.round(value)));
        if (newCount === state.particle_count) return;

        state.particle_count = newCount;

        try {
            await invoke('update_simulation_state', {
                stateName: 'particle_count',
                value: newCount,
            });
            await syncStateFromBackend();
        } catch (e) {
            console.error('Failed to update particle count:', e);
        }
    }

    async function updateAlpha(value: number) {
        if (!settings) return;

        settings.alpha = value;
        try {
            await invoke('update_simulation_setting', { settingName: 'alpha', value });
        } catch (e) {
            console.error('Failed to update alpha:', e);
        }
    }

    async function updateBeta(value: number) {
        if (!settings) return;

        settings.beta = value;
        try {
            await invoke('update_simulation_setting', { settingName: 'beta', value });
        } catch (e) {
            console.error('Failed to update beta:', e);
        }
    }

    async function updateVelocity(value: number) {
        if (!settings) return;

        settings.velocity = value;
        try {
            await invoke('update_simulation_setting', { settingName: 'velocity', value });
        } catch (e) {
            console.error('Failed to update velocity:', e);
        }
    }

    async function updateRadius(value: number) {
        if (!settings) return;

        settings.radius = value;
        try {
            await invoke('update_simulation_setting', { settingName: 'radius', value });
        } catch (e) {
            console.error('Failed to update radius:', e);
        }
    }

    async function updateWrapEdges(value: boolean) {
        if (!settings) return;

        settings.wrap_edges = value;
        try {
            await invoke('update_simulation_setting', { settingName: 'wrap_edges', value });
        } catch (e) {
            console.error('Failed to update wrap edges:', e);
        }
    }

    async function updatePositionGenerator(value: number) {
        if (!state) return;

        state.position_generator = value;
        try {
            await invoke('update_simulation_state', { stateName: 'position_generator', value });
        } catch (e) {
            console.error('Failed to update position generator:', e);
        }
    }

    // State update functions
    async function updateParticleSize(value: number) {
        if (!state) return;

        state.particle_size = value;
        try {
            await invoke('update_simulation_state', { stateName: 'particle_size', value });
        } catch (e) {
            console.error('Failed to update particle size:', e);
        }
    }

    async function updateColorScheme(colorSchemeName: string) {
        if (!state) return;

        try {
            state.current_color_scheme = colorSchemeName;
            await invoke('apply_color_scheme_by_name', { colorSchemeName });
        } catch (e) {
            console.error('Failed to update color scheme:', e);
        }
    }

    async function updateColorSchemeReversed(reversed: boolean) {
        if (!state) return;

        try {
            state.color_scheme_reversed = reversed;
            await invoke('update_simulation_state', {
                stateName: 'color_scheme_reversed',
                value: reversed,
            });
        } catch (e) {
            console.error('Failed to update color scheme reversed:', e);
        }
    }

    async function updateBackgroundColorMode(value: string) {
        if (!state) return;

        try {
            state.background_color_mode = value;
            await invoke('update_simulation_state', {
                stateName: 'background_color_mode',
                value: value,
            });
        } catch (e) {
            console.error('Failed to update background color mode:', e);
        }
    }

    async function updateForegroundColorMode(value: string) {
        if (!state) return;

        try {
            state.foreground_color_mode = value;
            await invoke('update_simulation_state', {
                stateName: 'foreground_color_mode',
                value: value,
            });
        } catch (e) {
            console.error('Failed to update foreground color mode:', e);
        }
    }

    async function updateTracesEnabled(value: boolean) {
        if (!state) return;

        try {
            state.traces_enabled = value;
            await invoke('update_simulation_state', {
                stateName: 'traces_enabled',
                value: value,
            });
        } catch (e) {
            console.error('Failed to update traces enabled:', e);
        }
    }

    async function updateTraceFade(value: number) {
        if (!state) return;

        try {
            state.trace_fade = value;
            await invoke('update_simulation_state', {
                stateName: 'trace_fade',
                value: value,
            });
        } catch (e) {
            console.error('Failed to update trace fade:', e);
        }
    }

    async function clearTrails() {
        try {
            await invoke('clear_trail_texture');
            console.log('Trail texture cleared successfully');
        } catch (e) {
            console.error('Failed to clear trail texture:', e);
        }
    }

    // Mouse interaction controls
    async function updateCursorSize(value: number) {
        if (!state) return;

        state.cursor_size = value;
        try {
            await invoke('update_cursor_size', { size: value });
        } catch (e) {
            console.error('Failed to update cursor size:', e);
        }
    }

    async function updateCursorStrength(value: number) {
        if (!state) return;

        state.cursor_strength = value;
        try {
            await invoke('update_cursor_strength', { strength: value });
        } catch (e) {
            console.error('Failed to update cursor strength:', e);
        }
    }

    // Preset management
    async function updatePreset(value: string) {
        current_preset = value;
        try {
            await invoke('apply_preset', { presetName: value });
            await syncSettingsFromBackend();
        } catch (e) {
            console.error('Failed to apply preset:', e);
        }
    }

    async function savePreset(presetName: string) {
        if (presetName.trim() === '') return;

        try {
            await invoke('save_preset', {
                presetName: presetName.trim(),
            });
            await loadPresets();
            current_preset = presetName.trim();
        } catch (e) {
            console.error('Failed to save preset:', e);
        }
    }

    // Data loading functions
    async function loadPresets() {
        try {
            available_presets = await invoke('get_presets_for_simulation_type', {
                simulationType: 'primordial_particles',
            });

            if (available_presets.includes('Default')) {
                current_preset = 'Default';
            }
        } catch (e) {
            console.error('Failed to load presets:', e);
            available_presets = [];
        }
    }

    async function loadColorSchemes() {
        try {
            available_color_schemes = await invoke('get_available_color_schemes');
        } catch (e) {
            console.error('Failed to load color schemes:', e);
            available_color_schemes = [];
        }
    }

    async function syncSettingsFromBackend() {
        try {
            const backendSettings = await invoke('get_current_settings');
            const backendState = await invoke('get_current_state');

            if (backendSettings) {
                settings = backendSettings as Settings;
            }

            if (backendState) {
                state = backendState as State;
                // Initialize the selected position generator to match current state
                selectedPositionGenerator = state?.position_generator || 0;
            }
        } catch (e) {
            console.error('Failed to sync settings from backend:', e);
        }
    }

    async function syncStateFromBackend() {
        try {
            const backendState = await invoke('get_current_state');
            if (backendState) {
                state = backendState as State;
            }
        } catch (e) {
            console.error('Failed to sync state from backend:', e);
        }
    }

    // Simulation control
    async function startSimulation() {
        try {
            await invoke('start_primordial_particles_simulation');
            console.log('Primordial Particles simulation started');
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

    async function pauseSimulation() {
        try {
            await invoke('pause_simulation');
            isSimulationRunning = false;

            if (autoHideManager) {
                autoHideManager.updateState({ running: isSimulationRunning });
                autoHideManager.handlePause();
            }
            console.log('Simulation paused');
        } catch (e) {
            console.error('Failed to pause simulation:', e);
        }
    }

    async function resumeSimulation() {
        try {
            await invoke('resume_simulation');
            isSimulationRunning = true;

            if (autoHideManager) {
                autoHideManager.updateState({ running: isSimulationRunning });
                autoHideManager.handleResume();
            }
            console.log('Simulation resumed');
        } catch (e) {
            console.error('Failed to resume simulation:', e);
        }
    }

    async function togglePause() {
        if (isSimulationRunning) {
            await pauseSimulation();
        } else {
            await resumeSimulation();
        }
    }

    async function resetSimulation() {
        try {
            await invoke('reset_simulation');
            await new Promise((resolve) => setTimeout(resolve, 100));
            await syncSettingsFromBackend();
            console.log('Simulation reset complete');
        } catch (e) {
            console.error('Failed to reset simulation:', e);
        }
    }

    async function handleMouseEvent(e: CustomEvent) {
        const event = e.detail as MouseEvent | WheelEvent;

        // Handle zoom separately
        if (event.type === 'wheel') {
            const wheelEvent = event as WheelEvent;
            wheelEvent.preventDefault();

            const zoomDelta = -wheelEvent.deltaY * 0.001;
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

        // Handle mouse interactions for particle grabbing/flinging
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

        if (autoHideManager) {
            autoHideManager.handleUserInteraction();
        }
    }

    async function toggleBackendGui() {
        try {
            await invoke('toggle_gui');
            showUI = !showUI;

            if (autoHideManager) {
                autoHideManager.updateState({ showUI, running: isSimulationRunning });
                autoHideManager.handleUIToggle(showUI);
            }
        } catch (err) {
            console.error('Failed to toggle backend GUI:', err);
        }
    }

    // Lifecycle
    onMount(async () => {
        try {
            // Initialize auto-hide manager
            autoHideManager = new AutoHideManager(
                {
                    controlsVisible,
                    cursorHidden: false,
                    showUI,
                    running: isSimulationRunning,
                },
                {
                    onControlsShow: () => {
                        controlsVisible = true;
                    },
                    onControlsHide: () => {
                        controlsVisible = false;
                    },
                    onCursorShow: () => {
                        document.body.style.cursor = '';
                    },
                    onCursorHide: () => {
                        document.body.style.cursor = 'none';
                    },
                },
                {
                    autoHideDelay,
                    cursorHideDelay: 2000,
                }
            );

            // Create event listeners
            eventListeners = createAutoHideEventListeners(() => {
                autoHideManager.handleUserInteraction();
            });
            eventListeners.add();

            // Set up FPS monitoring
            try {
                unsubscribeFps = await listen('fps-update', (event) => {
                    fps_display = event.payload as number;
                });
            } catch (e) {
                console.error('Failed to set up FPS listener:', e);
            }

            // Listen for simulation initialization event
            try {
                unsubscribeSimulationInitialized = await listen(
                    'simulation-initialized',
                    async () => {
                        await syncSettingsFromBackend();
                        isSimulationRunning = true;
                        isInitialized = true;
                    }
                );
            } catch (e) {
                console.error('Failed to set up simulation-initialized listener:', e);
            }

            // Listen for simulation resumed event
            try {
                unsubscribeSimulationResumed = await listen('simulation-resumed', async () => {
                    isSimulationRunning = true;
                });
            } catch (e) {
                console.error('Failed to set up simulation-resumed listener:', e);
            }

            // Start simulation
            await startSimulation();

            // Load initial data
            await Promise.all([loadPresets(), loadColorSchemes()]);

            // Sync settings
            await syncSettingsFromBackend();
        } catch (e) {
            console.error('Failed to initialize simulation:', e);
        } finally {
            isLoading = false;
        }
    });

    onDestroy(async () => {
        await stopSimulation();

        if (unsubscribeFps) {
            unsubscribeFps();
        }
        if (unsubscribeSimulationInitialized) {
            unsubscribeSimulationInitialized();
        }
        if (unsubscribeSimulationResumed) {
            unsubscribeSimulationResumed();
        }

        if (eventListeners) {
            eventListeners.remove();
        }
        if (autoHideManager) {
            autoHideManager.cleanup();
        }
    });
</script>

<style>
    /* Primordial Particles specific styles */
    .range-value {
        color: rgba(255, 255, 255, 0.7);
        font-size: 0.8rem;
        margin-left: 0.5rem;
    }
</style>
