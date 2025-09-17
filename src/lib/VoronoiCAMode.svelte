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
    on:userInteraction={() => autoHideManager?.handleUserInteraction()}
    on:mouseEvent={handleMouseEvent}
>
    <form on:submit|preventDefault>
        <!-- About this simulation -->
        <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
            <p>
                Voronoi Cellular Automata evolves regions based on nearest-seed influence and local
                rules. Cells belong to Voronoi regions that shift as seeds move via run-and-tumble
                dynamics and drift. Parameters control neighborhood size, activity thresholds, and
                temporal behavior.
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
                    bind:available_color_schemes={available_luts}
                    current_color_scheme={currentLut}
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
                    <label for="vcaBorderWidth">Border Width</label>
                    <NumberDragBox
                        value={borderWidth}
                        min={0.0}
                        max={1000.0}
                        step={1}
                        precision={1}
                        on:change={({ detail }) => {
                            updateBorderWidth(detail);
                        }}
                    />
                    <small style="color: rgba(255, 255, 255, 0.6); font-size: 0.8rem;">
                        Note: Large values use strategic sampling for performance
                    </small>
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
                        <Button
                            variant="default"
                            on:click={() => dispatch('navigate', 'how-to-play')}
                        >
                            📖 Camera Controls
                        </Button>
                    </div>
                    <div class="control-group">
                        <span class="camera-help-text"
                            >Camera controls not working? Click the control bar at the top of the
                            screen.</span
                        >
                    </div>
                </div>
                <div class="cursor-settings">
                    <div class="cursor-settings-header">
                        <span>{!running ? '🎨 Painting Settings' : '🎯 Cursor Settings'}</span>
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
                    <div class="setting-item rule-item">
                        <span class="setting-label">Cellular Automaton Rule:</span>
                        <Selector
                            options={rulestringOptions.map((opt) => opt.label)}
                            value={getCurrentRuleLabel()}
                            on:change={({ detail }) => updateRulestringFromLabel(detail.value)}
                        />
                        <small class="rulestring-help">
                            {getRulestringDescription(rulestring)} ({rulestring})
                        </small>
                    </div>
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
                        <span class="setting-label">Brownian Speed (px/s):</span>
                        <NumberDragBox
                            value={brownianSpeed}
                            min={0}
                            max={300}
                            step={1}
                            precision={0}
                            on:change={({ detail }) => updateBrownianSpeed(detail)}
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
    import LutSelector from './components/shared/ColorSchemeSelector.svelte';
    import Selector from './components/inputs/Selector.svelte';
    import CursorConfig from './components/shared/CursorConfig.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
    import PresetFieldset from './components/shared/PresetFieldset.svelte';
    import Button from './components/shared/Button.svelte';
    import { AutoHideManager, createAutoHideEventListeners } from './utils/autoHide';

    const dispatch = createEventDispatcher();
    export let menuPosition: string = 'middle';
    export let autoHideDelay: number = 3000;

    // Control bar / UI state
    let running = false;
    let loading = true;
    let showUI = true;
    let controlsVisible = true;
    let currentFps = 0;
    let enabled = false; // Whether post processing should be enabled

    // Auto-hide manager
    let autoHideManager: AutoHideManager;
    let eventListeners: { add: () => void; remove: () => void };

    // Simple settings
    let rulestring = 'B3/S23';
    let drift = 0.5;

    // Rulestring options with human-readable names
    const rulestringOptions = [
        { value: 'B3/S23', label: "Conway's Game of Life" },
        { value: 'B36/S23', label: 'High Life' },
        { value: 'B2/S', label: 'Seeds' },
        { value: 'B1357/S1357', label: 'Replicator' },
        { value: 'B3/S012345678', label: 'Life without Death' },
        { value: 'B3/S1234', label: 'Maze' },
        { value: 'B3/S12345', label: 'Mazectric' },
        { value: 'B34/S34', label: '34 Life' },
        { value: 'B35678/S5678', label: 'Diamoeba' },
        { value: 'B36/S125', label: '2x2' },
        { value: 'B368/S245', label: 'Day & Night' },
        { value: 'B4678/S35678', label: 'Anneal' },
        { value: 'B5678/S45678', label: 'Vote' },
        { value: 'B6/S16', label: 'Coral' },
        { value: 'B6/S1', label: 'Long Life' },
        { value: 'B6/S12', label: 'Stains' },
        { value: 'B6/S123', label: 'Assimilation' },
        { value: 'B6/S15', label: 'Pseudo Life' },
        { value: 'B6/S2', label: 'Long Life (variant)' },
        { value: 'B25/S4', label: 'Self-Replicating' },
        { value: 'B7/S', label: 'Seeds (7 neighbors)' },
        { value: 'B8/S', label: 'Seeds (8 neighbors)' },
        { value: 'B9/S', label: 'Seeds (9 neighbors)' },
    ];
    let brownianSpeed = 60;
    let timeScale = 1.0;
    let pointCount = 300;
    let borderWidth = 1.0;

    // LUT + controls
    let available_luts: string[] = [];
    let currentLut = 'MATPLOTLIB_bone';
    let lutReversed = true;
    let coloringMode = 'Density';
    let bordersEnabled = true;
    let cursorSize = 0.15; // Larger cursor to actually reach cells
    let cursorStrength = 1.0;

    // Presets + UI
    let available_presets: string[] = [];
    let current_preset = '';
    let show_about_section = false;

    let unlistenInitialized: (() => void) | null = null;
    let unlistenFps: (() => void) | null = null;
    let isMousePressed = false;
    let currentMouseButton = 0;

    // Mouse event throttling
    let mouseEventThrottleTimeout: number | null = null;
    let pendingMouseEvent: { screenX: number; screenY: number; mouseButton: number } | null = null;

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
                    const settings = (await invoke('get_current_settings')) as Record<
                        string,
                        unknown
                    >;
                    if (settings && typeof settings.rulestring === 'string')
                        rulestring = settings.rulestring;
                    if (settings && typeof settings.drift === 'number') drift = settings.drift;
                    if (settings && typeof settings.brownianSpeed === 'number')
                        brownianSpeed = settings.brownianSpeed;
                    if (settings && typeof settings.timeScale === 'number')
                        timeScale = settings.timeScale;
                    if (settings && typeof settings.numPoints === 'number')
                        pointCount = settings.numPoints;
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
                    if (settings && typeof settings.borderWidth === 'number')
                        borderWidth = settings.borderWidth;
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
            // Simulation-initialized event will set running state; don't override here
            await loadAvailableLuts();
        } catch (e) {
            console.error('Failed to start Voronoi CA:', e);
        }
    }

    async function loadAvailableLuts() {
        try {
            const luts = await invoke('get_available_color_schemes');
            available_luts = luts as string[];
        } catch (e) {
            console.error('Failed to load LUTs:', e);
        }
    }

    async function applyLut(lutName: string) {
        currentLut = lutName;
        try {
            await invoke('apply_color_scheme_by_name', { colorSchemeName: lutName });
        } catch (e) {
            console.error('Failed to apply LUT:', e);
        }
    }

    async function toggleLutReversed() {
        lutReversed = !lutReversed;
        try {
            await invoke('toggle_color_scheme_reversed');
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

    async function updateBorderWidth(value: number) {
        borderWidth = value;
        try {
            const params = { settingName: 'borderWidth', value };
            await invoke('update_simulation_setting', params);
        } catch (e) {
            console.error('Failed to update border width:', e);
            console.error('Error details:', {
                message: String(e),
                stack: e instanceof Error ? e.stack : undefined,
                params: { settingName: 'borderWidth', value },
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
                if (typeof settings.brownianSpeed === 'number')
                    brownianSpeed = settings.brownianSpeed;
                if (typeof settings.timeScale === 'number') timeScale = settings.timeScale;
                if (typeof settings.numPoints === 'number') pointCount = settings.numPoints;
                if (typeof settings.currentLutName === 'string')
                    currentLut = settings.currentLutName;
                if (typeof settings.lutReversed === 'boolean') lutReversed = settings.lutReversed;
                if (typeof settings.coloringMode === 'string') coloringMode = settings.coloringMode;
                if (typeof settings.bordersEnabled === 'boolean')
                    bordersEnabled = settings.bordersEnabled;
                if (typeof settings.cursor_size === 'number') cursorSize = settings.cursor_size;
                if (typeof settings.cursor_strength === 'number')
                    cursorStrength = settings.cursor_strength;
                if (typeof settings.borderWidth === 'number') borderWidth = settings.borderWidth;
            }
        } catch (e) {
            console.error('Failed to sync settings from backend:', e);
        }
    }

    async function stopSimulation() {
        try {
            await invoke('pause_simulation');
            running = false;

            // Update auto-hide manager state and handle pause
            if (autoHideManager) {
                autoHideManager.updateState({ running });
                autoHideManager.handlePause();
            }
        } catch (e) {
            console.error('Failed to pause Voronoi CA:', e);
        }
    }

    async function resumeSimulation() {
        try {
            await invoke('resume_simulation');
            running = true;

            // Update auto-hide manager state and handle resume
            if (autoHideManager) {
                autoHideManager.updateState({ running });
                autoHideManager.handleResume();
            }
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
            // Toggle local state directly instead of relying on backend state
            showUI = !showUI;

            // Update auto-hide manager state and handle UI toggle
            if (autoHideManager) {
                autoHideManager.updateState({ showUI, running });
                autoHideManager.handleUIToggle(showUI);
            }
        } catch (e) {
            console.error('Failed to toggle GUI:', e);
        }
    }

    // Throttled mouse event processing
    async function processPendingMouseEvent() {
        if (pendingMouseEvent) {
            try {
                await invoke('handle_mouse_interaction_screen', pendingMouseEvent);
                pendingMouseEvent = null;
            } catch (err) {
                console.error('Mouse interaction failed:', err);
            }
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
                    // Clear any pending throttled events and process immediately
                    if (mouseEventThrottleTimeout) {
                        clearTimeout(mouseEventThrottleTimeout);
                        mouseEventThrottleTimeout = null;
                    }
                    await invoke('handle_mouse_interaction_screen', {
                        screenX,
                        screenY,
                        mouseButton: currentMouseButton,
                    });
                } else if (event.type === 'mousemove') {
                    if (isMousePressed) {
                        // Store the latest mouse position for throttled processing
                        pendingMouseEvent = {
                            screenX,
                            screenY,
                            mouseButton: currentMouseButton,
                        };

                        // Clear existing timeout and set a new one
                        if (mouseEventThrottleTimeout) {
                            clearTimeout(mouseEventThrottleTimeout);
                        }

                        // Throttle mouse move events to 60fps (16.67ms)
                        mouseEventThrottleTimeout = window.setTimeout(() => {
                            processPendingMouseEvent();
                            mouseEventThrottleTimeout = null;
                        }, 16);
                    }
                } else if (event.type === 'mouseup') {
                    if (isMousePressed) {
                        isMousePressed = false;
                        // Clear any pending throttled events
                        if (mouseEventThrottleTimeout) {
                            clearTimeout(mouseEventThrottleTimeout);
                            mouseEventThrottleTimeout = null;
                        }
                        // Process any pending mouse event immediately
                        if (pendingMouseEvent) {
                            await processPendingMouseEvent();
                        }
                        await invoke('handle_mouse_release', { mouseButton: currentMouseButton });
                    }
                } else if (event.type === 'contextmenu') {
                    isMousePressed = true;
                    currentMouseButton = 2;
                    // Clear any pending throttled events and process immediately
                    if (mouseEventThrottleTimeout) {
                        clearTimeout(mouseEventThrottleTimeout);
                        mouseEventThrottleTimeout = null;
                    }
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
    }

    async function updateDrift(value: number) {
        drift = value;
        try {
            await invoke('update_simulation_setting', { settingName: 'drift', value });
        } catch (e) {
            console.error('Failed to update drift:', e);
        }
    }

    async function updateRulestring(value: string) {
        rulestring = value;
        try {
            await invoke('update_simulation_setting', { settingName: 'rulestring', value });
        } catch (e) {
            console.error('Failed to update rulestring:', e);
        }
    }

    function getCurrentRuleLabel(): string {
        const option = rulestringOptions.find((opt) => opt.value === rulestring);
        return option ? option.label : "Conway's Game of Life";
    }

    async function updateRulestringFromLabel(label: string) {
        const option = rulestringOptions.find((opt) => opt.label === label);
        if (option) {
            await updateRulestring(option.value);
        }
    }

    function getRulestringDescription(rule: string): string {
        const descriptions: Record<string, string> = {
            'B1357/S1357':
                'Replicator - Every pattern is eventually replaced by multiple copies of itself',
            'B2/S': 'Seeds - All patterns are phoenixes, explosive chaotic growth',
            'B25/S4': 'Small self-replicating pattern with glider bouncing',
            'B3/S012345678': 'Life without Death - Cells never die once alive',
            'B3/S23': "Conway's Game of Life - Highly complex behavior",
            'B3/S1234': 'Maze - Creates maze-like patterns',
            'B3/S12345': 'Mazectric - Maze with diagonal connections',
            'B34/S34': '34 Life - Stable patterns with interesting dynamics',
            'B35678/S5678': 'Diamoeba - Creates diamond-like chaotic patterns',
            'B36/S125': '2x2 - Supports 2x2 blocks and complex patterns',
            'B36/S23': 'High Life - Like Life but also births with 6 neighbors',
            'B368/S245': 'Day & Night - Symmetric rule with interesting patterns',
            'B4678/S35678': 'Anneal - Creates annealing-like patterns',
            'B5678/S45678': 'Vote - Majority rule cellular automaton',
            'B6/S16': 'Coral - Creates coral-like growth patterns',
            'B6/S1': 'Long Life - Long-lived patterns',
            'B6/S12': 'Stains - Creates stain-like patterns',
            'B6/S123': 'Assimilation - Patterns assimilate neighbors',
            'B6/S15': 'Pseudo Life - Life-like but different dynamics',
            'B6/S2': 'Long Life - Another long-lived variant',
            'B7/S': 'Seeds variant - Explosive growth',
            'B8/S': 'Seeds variant - Explosive growth',
            'B9/S': 'Seeds variant - Explosive growth',
        };
        return descriptions[rule] || 'Custom rule - Birth/Survival conditions';
    }

    async function updateBrownianSpeed(value: number) {
        brownianSpeed = value;
        try {
            await invoke('update_simulation_setting', { settingName: 'brownianSpeed', value });
        } catch (e) {
            console.error('Failed to update brownian speed:', e);
        }
    }

    async function updatePointCount(value: number) {
        pointCount = Math.max(1, Math.round(value));
        try {
            await invoke('update_simulation_setting', {
                settingName: 'numPoints',
                value: pointCount,
            });
        } catch (e) {
            console.error('Failed to update point count:', e);
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

    onMount(() => {
        // Initialize auto-hide manager
        autoHideManager = new AutoHideManager(
            {
                controlsVisible,
                cursorHidden: false,
                showUI,
                running,
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

        // Clean up auto-hide functionality
        if (eventListeners) {
            eventListeners.remove();
        }
        if (autoHideManager) {
            autoHideManager.cleanup();
        }

        // Clean up mouse event throttling
        if (mouseEventThrottleTimeout) {
            clearTimeout(mouseEventThrottleTimeout);
            mouseEventThrottleTimeout = null;
        }
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
        word-wrap: break-word;
        overflow-wrap: break-word;
        hyphens: auto;
    }

    .camera-help-text {
        word-wrap: break-word;
        overflow-wrap: break-word;
        hyphens: auto;
        line-height: 1.4;
    }

    /* Settings grid for key/value pairs */
    .settings-grid {
        display: grid;
        grid-template-columns: 1fr auto;
        gap: 0.15rem 0.3rem;
        width: 100%;
    }

    /* Diagram section styles */

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

    .rule-item .rulestring-help {
        grid-column: 1 / -1;
    }

    .rulestring-help {
        display: block;
        color: rgba(255, 255, 255, 0.6);
        font-size: 0.8rem;
        margin-top: 0.25rem;
        font-style: italic;
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
