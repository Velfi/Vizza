<SimulationLayout
    simulationName="Slime Mold"
    {running}
    {loading}
    {showUI}
    {currentFps}
    {controlsVisible}
    {menuPosition}
    on:back={returnToMenu}
    on:toggleUI={toggleBackendGui}
    on:pause={stopSimulation}
    on:resume={resumeSimulation}
    on:userInteraction={() => autoHideManager.handleUserInteraction()}
    on:mouseEvent={handleMouseEvent}
>
    {#if settings && state}
        <form on:submit|preventDefault>
            <!-- About this simulation -->
            <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
                <p>
                    Slime Mold simulates the fascinating behavior of Physarum polycephalum, a
                    single-celled organism that exhibits collective intelligence. Thousands of
                    agents move through space, depositing pheromone trails that attract other
                    agents, creating efficient networks.
                </p>
                <p>
                    The simulation models how slime molds solve complex problems like finding
                    optimal paths through mazes and connecting food sources. Agents sense pheromone
                    gradients and adjust their movement accordingly, while the pheromone trails
                    decay and diffuse over time.
                </p>
                <p>
                    Watch as simple rules for movement and pheromone interaction lead to the
                    emergence of sophisticated transportation networks, branching patterns, and
                    adaptive pathfinding - all without central coordination or planning.
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
                    <label for="colorSchemeSelector">Color Scheme</label>
                    <ColorSchemeSelector
                        bind:available_color_schemes={available_luts}
                        current_color_scheme={state?.current_color_scheme}
                        reversed={state?.color_scheme_reversed}
                        on:select={({ detail }) => updateLutName(detail.name)}
                        on:reverse={() => updateLutReversed()}
                    />
                </div>
            </fieldset>

            <!-- Post Processing -->
            <PostProcessingMenu simulationType="slime_mold" />

            <!-- Controls -->
            <ControlsPanel
                mouseInteractionText="🖱️ Left click: Attract agents | Right click: Repel agents"
                cursorSize={state.cursor_size}
                cursorStrength={state.cursor_strength}
                sizeMin={10}
                sizeMax={500}
                sizeStep={5}
                strengthMin={0}
                strengthMax={50}
                strengthStep={0.5}
                sizePrecision={0}
                strengthPrecision={1}
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
                        <Button
                            variant="warning"
                            type="button"
                            on:click={async () => {
                                try {
                                    await invoke('reset_trails');
                                    console.log('Trails reset successfully');
                                } catch (e) {
                                    console.error('Failed to reset trails:', e);
                                }
                            }}>🧹 Clear Trails</Button
                        >
                    </div>
                    <div class="control-group">
                        <label for="positionGenerator" class="visually-hidden"
                            >Agent Position Generator</label
                        >
                        <ButtonSelect
                            value={state.position_generator}
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
                                { value: 'Image', label: 'Image', buttonAction: 'randomize' },
                            ]}
                            buttonText="Reset Agents"
                            placeholder="Select position generator..."
                            on:change={async (e) => {
                                if (state) {
                                    state.position_generator = e.detail.value;
                                    try {
                                        await invoke('update_simulation_state', {
                                            stateName: 'position_generator',
                                            value: e.detail.value,
                                        });
                                    } catch (err) {
                                        console.error('Failed to update position generator:', err);
                                    }
                                }
                            }}
                            on:buttonclick={async () => {
                                try {
                                    await invoke('reset_agents');
                                    await invoke('reset_trails');
                                    console.log('Agents randomized via ButtonSelect');
                                } catch (err) {
                                    console.error('Failed to randomize agents:', err);
                                }
                            }}
                        />
                    </div>

                    <!-- Image Position Generator Controls -->
                    {#if state.position_generator === 'Image'}
                        <div class="control-group">
                            <ImageSelector
                                fitMode={settings.position_image_fit_mode}
                                loadCommand="load_slime_mold_position_image"
                                showFitMode={true}
                                showLoadButton={true}
                                onFitModeChange={async (
                                    value: 'Center' | 'Stretch' | 'FitH' | 'FitV'
                                ) => {
                                    if (settings) {
                                        settings.position_image_fit_mode = value;
                                        try {
                                            await invoke('update_simulation_setting', {
                                                settingName: 'position_image_fit_mode',
                                                value: value,
                                            });
                                        } catch (err) {
                                            console.error(
                                                'Failed to update position image fit mode:',
                                                err
                                            );
                                        }
                                    }
                                }}
                            />
                        </div>
                    {/if}
                </div>

                <!-- Pheromone Settings -->
                <div class="settings-section">
                    <h3 class="section-header">Pheromone</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="sm-decay-rate">Decay Rate:</label>
                            <NumberDragBox
                                id="sm-decay-rate"
                                bind:value={settings.pheromone_decay_rate}
                                min={0}
                                max={10000}
                                step={1}
                                precision={2}
                                unit="%"
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'pheromone_decay_rate',
                                            value: e.detail,
                                        });
                                    } catch (err) {
                                        console.error(
                                            'Failed to update pheromone decay rate:',
                                            err
                                        );
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="sm-deposition-rate"
                                >Deposition Rate:</label
                            >
                            <NumberDragBox
                                id="sm-deposition-rate"
                                bind:value={settings.pheromone_deposition_rate}
                                min={0}
                                max={100}
                                step={1}
                                precision={2}
                                unit="%"
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'pheromone_deposition_rate',
                                            value: e.detail,
                                        });
                                    } catch (err) {
                                        console.error(
                                            'Failed to update pheromone deposition rate:',
                                            err
                                        );
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="sm-diffusion-rate"
                                >Diffusion Rate:</label
                            >
                            <NumberDragBox
                                id="sm-diffusion-rate"
                                bind:value={settings.pheromone_diffusion_rate}
                                min={0}
                                max={100}
                                step={1}
                                precision={2}
                                unit="%"
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'pheromone_diffusion_rate',
                                            value: e.detail,
                                        });
                                    } catch (err) {
                                        console.error(
                                            'Failed to update pheromone diffusion rate:',
                                            err
                                        );
                                    }
                                }}
                            />
                        </div>
                    </div>
                </div>

                <!-- Agent Settings -->
                <div class="settings-section">
                    <h3 class="section-header">Agent</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="sm-agent-count"
                                >Agent Count (millions):</label
                            >
                            <AgentCountInput
                                value={agent_count_millions}
                                min={0}
                                max={100}
                                on:update={async (e) => {
                                    try {
                                        await updateAgentCount(e.detail);
                                        console.log(`Agent count updated to ${e.detail} million`);
                                    } catch (err) {
                                        console.error('Failed to update agent count:', err);
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="sm-min-speed">Min Speed:</label>
                            <NumberDragBox
                                id="sm-min-speed"
                                bind:value={settings.agent_speed_min}
                                min={0}
                                max={500}
                                step={10}
                                precision={1}
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'agent_speed_min',
                                            value: e.detail,
                                        });
                                        await syncSettingsFromBackend();
                                    } catch (err) {
                                        console.error('Failed to update min speed:', err);
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="sm-max-speed">Max Speed:</label>
                            <NumberDragBox
                                id="sm-max-speed"
                                bind:value={settings.agent_speed_max}
                                min={0}
                                max={500}
                                step={10}
                                precision={1}
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'agent_speed_max',
                                            value: e.detail,
                                        });
                                        await syncSettingsFromBackend();
                                    } catch (err) {
                                        console.error('Failed to update max speed:', err);
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="sm-turn-rate">Turn Rate:</label>
                            <NumberDragBox
                                id="sm-turn-rate"
                                value={(settings.agent_turn_rate * 180) / Math.PI}
                                on:change={({ detail }) =>
                                    (settings!.agent_turn_rate = (detail * Math.PI) / 180)}
                                min={0}
                                max={360}
                                step={1}
                                precision={0}
                                unit="°"
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'agent_turn_rate',
                                            value: (e.detail * Math.PI) / 180, // Convert degrees to radians
                                        });
                                        await syncSettingsFromBackend();
                                    } catch (err) {
                                        console.error('Failed to update turn rate:', err);
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="sm-jitter">Jitter:</label>
                            <NumberDragBox
                                id="sm-jitter"
                                bind:value={settings.agent_jitter}
                                min={0.0}
                                max={10.0}
                                step={0.01}
                                precision={2}
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'agent_jitter',
                                            value: e.detail,
                                        });
                                        await syncSettingsFromBackend();
                                    } catch (err) {
                                        console.error('Failed to update agent jitter:', err);
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="sm-sensor-angle">Sensor Angle:</label>
                            <NumberDragBox
                                id="sm-sensor-angle"
                                value={(settings.agent_sensor_angle * 180) / Math.PI}
                                on:change={({ detail }) =>
                                    (settings!.agent_sensor_angle = (detail * Math.PI) / 180)}
                                min={0}
                                max={180}
                                step={1}
                                precision={0}
                                unit="°"
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'agent_sensor_angle',
                                            value: (e.detail * Math.PI) / 180, // Convert degrees to radians
                                        });
                                        await syncSettingsFromBackend();
                                    } catch (err) {
                                        console.error('Failed to update sensor angle:', err);
                                    }
                                }}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="sm-sensor-distance"
                                >Sensor Distance:</label
                            >
                            <NumberDragBox
                                id="sm-sensor-distance"
                                bind:value={settings.agent_sensor_distance}
                                min={0}
                                max={500}
                                step={1}
                                precision={0}
                                on:change={async (e) => {
                                    try {
                                        await invoke('update_simulation_setting', {
                                            settingName: 'agent_sensor_distance',
                                            value: e.detail,
                                        });
                                        await syncSettingsFromBackend();
                                    } catch (err) {
                                        console.error('Failed to update sensor distance:', err);
                                    }
                                }}
                            />
                        </div>
                    </div>
                </div>

                <!-- Mask Settings -->
                <div class="settings-section">
                    <h3 class="section-header">Mask</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="sm-mask-pattern">Mask Pattern:</label>
                            <Selector
                                id="sm-mask-pattern"
                                options={[
                                    'Disabled',
                                    'Checkerboard',
                                    'Diagonal Gradient',
                                    'Radial Gradient',
                                    'Vertical Stripes',
                                    'Horizontal Stripes',
                                    'Wave Function',
                                    'Cosine Grid',
                                    'Image',
                                ]}
                                value={state.mask_pattern}
                                on:change={handleMaskPattern}
                            />
                        </div>
                        {#if state.mask_pattern !== 'Disabled'}
                            <div class="setting-item">
                                <label class="setting-label" for="sm-mask-target"
                                    >Mask Target:</label
                                >
                                <Selector
                                    id="sm-mask-target"
                                    options={[
                                        'Pheromone Deposition',
                                        'Pheromone Decay',
                                        'Pheromone Diffusion',
                                        'Agent Speed',
                                        'Agent Turn Rate',
                                        'Agent Sensor Distance',
                                        'Trail Map',
                                    ]}
                                    value={state.mask_target}
                                    on:change={handleMaskTarget}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="setting-label" for="sm-mask-strength">Strength:</label
                                >
                                <NumberDragBox
                                    id="sm-mask-strength"
                                    value={state.mask_strength}
                                    min={0.0}
                                    max={1.0}
                                    step={0.01}
                                    on:change={async (e) => {
                                        if (state) {
                                            state.mask_strength = e.detail;
                                            try {
                                                await invoke('update_simulation_state', {
                                                    stateName: 'mask_strength',
                                                    value: e.detail,
                                                });
                                            } catch (err) {
                                                console.error(
                                                    'Failed to update mask strength:',
                                                    err
                                                );
                                            }
                                        }
                                    }}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="setting-label" for="sm-mask-curve">Mask Curve:</label>
                                <NumberDragBox
                                    id="sm-mask-curve"
                                    value={state.mask_curve}
                                    min={0.2}
                                    max={5.0}
                                    step={0.05}
                                    on:change={async (e) => {
                                        if (state) {
                                            state.mask_curve = e.detail;
                                            try {
                                                await invoke('update_simulation_state', {
                                                    stateName: 'mask_curve',
                                                    value: e.detail,
                                                });
                                            } catch (err) {
                                                console.error('Failed to update mask curve:', err);
                                            }
                                        }
                                    }}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="checkbox">
                                    <input
                                        type="checkbox"
                                        checked={state.mask_mirror_horizontal}
                                        on:change={(e) =>
                                            handleMaskMirrorHorizontal(
                                                (e.target as HTMLInputElement).checked
                                            )}
                                    />
                                    Mirror horizontal
                                </label>
                            </div>
                            <div class="setting-item">
                                <label class="checkbox">
                                    <input
                                        type="checkbox"
                                        checked={state.mask_mirror_vertical || false}
                                        on:change={(e) =>
                                            handleMaskMirrorVertical(
                                                (e.target as HTMLInputElement).checked
                                            )}
                                    />
                                    Mirror vertical
                                </label>
                            </div>
                            <div class="setting-item">
                                <label class="checkbox">
                                    <input
                                        type="checkbox"
                                        checked={state.mask_invert_tone || false}
                                        on:change={(e) =>
                                            handleMaskInvertTone(
                                                (e.target as HTMLInputElement).checked
                                            )}
                                    />
                                    Invert tone
                                </label>
                            </div>
                            {#if state.mask_pattern === 'Image'}
                                <ImageSelector
                                    fitMode={state.mask_image_fit_mode}
                                    loadCommand="load_slime_mold_mask_image"
                                    onFitModeChange={async (value) => {
                                        if (state) {
                                            state.mask_image_fit_mode = value as
                                                | 'Stretch'
                                                | 'Center'
                                                | 'FitH'
                                                | 'FitV';
                                            try {
                                                await invoke('update_simulation_state', {
                                                    stateName: 'mask_image_fit_mode',
                                                    value: value,
                                                });
                                            } catch (err) {
                                                console.error('Failed to update fit mode:', err);
                                            }
                                        }
                                    }}
                                />
                                <WebcamControls
                                    {webcamDevices}
                                    {webcamActive}
                                    onStartWebcam={startWebcamCapture}
                                    onStopWebcam={stopWebcamCapture}
                                />
                            {/if}
                        {/if}
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
    import SimulationLayout from './components/shared/SimulationLayout.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
    import PresetFieldset from './components/shared/PresetFieldset.svelte';
    import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
    import ControlsPanel from './components/shared/ControlsPanel.svelte';
    import ButtonSelect from './components/inputs/ButtonSelect.svelte';
    import Button from './components/shared/Button.svelte';
    import AgentCountInput from './components/slime-mold/AgentCountInput.svelte';
    import NumberDragBox from './components/inputs/NumberDragBox.svelte';
    import Selector from './components/inputs/Selector.svelte';
    import ImageSelector from './components/shared/ImageSelector.svelte';
    import WebcamControls from './components/shared/WebcamControls.svelte';
    import { AutoHideManager, createAutoHideEventListeners } from './utils/autoHide';
    import './shared-theme.css';
    import ColorSchemeSelector from './components/shared/ColorSchemeSelector.svelte';

    const dispatch = createEventDispatcher();

    export let menuPosition: string = 'middle';
    export let autoHideDelay: number = 3000;

    // Settings type (matches src-tauri/src/simulations/slime_mold/settings.rs)
    type Settings = {
        // Agent parameters
        agent_jitter: number;
        agent_possible_starting_headings: [number, number];
        agent_sensor_angle: number; // radians
        agent_sensor_distance: number;
        agent_speed_max: number;
        agent_speed_min: number;
        agent_turn_rate: number; // radians per second

        // Pheromone parameters
        pheromone_decay_rate: number;
        pheromone_deposition_rate: number;
        pheromone_diffusion_rate: number;

        // Position image fit mode
        position_image_fit_mode: 'Stretch' | 'Center' | 'FitH' | 'FitV';

        // Update frequencies and randomness
        diffusion_frequency: number;
        decay_frequency: number;
        random_seed: number;

        // Background mode
        background_mode: 'black' | 'white';
    };

    // State type (matches src-tauri/src/simulations/slime_mold/state.rs)
    type State = {
        // Mask system state
        mask_pattern:
            | 'Disabled'
            | 'Checkerboard'
            | 'Diagonal Gradient'
            | 'Radial Gradient'
            | 'Vertical Stripes'
            | 'Horizontal Stripes'
            | 'Wave Function'
            | 'Cosine Grid'
            | 'Image';
        mask_target:
            | 'Pheromone Deposition'
            | 'Pheromone Decay'
            | 'Pheromone Diffusion'
            | 'Agent Speed'
            | 'Agent Turn Rate'
            | 'Agent Sensor Distance'
            | 'Trail Map';
        mask_strength: number;
        mask_curve: number;
        mask_reversed: boolean;
        mask_image_fit_mode: 'Stretch' | 'Center' | 'FitH' | 'FitV';
        mask_mirror_horizontal: boolean;
        mask_mirror_vertical: boolean;
        mask_invert_tone: boolean;

        // Current color scheme state (runtime)
        current_color_scheme: string;
        color_scheme_reversed: boolean;

        // Cursor interaction parameters
        cursor_size: number;
        cursor_strength: number;

        // Position generator
        position_generator: string;

        // UI visibility state
        gui_visible: boolean;

        // Camera state (position and zoom)
        camera_position: [number, number];
        camera_zoom: number;

        // Simulation runtime state
        simulation_time: number;
        is_running: boolean;
    };

    // Simulation state
    let settings: Settings | undefined = undefined;
    let state: State | undefined = undefined;

    // Agent count tracked separately (not part of preset settings)
    let currentAgentCount = 1_000_000;

    // Preset and color scheme state
    let current_preset = '';
    let available_presets: string[] = [];
    let available_luts: string[] = [];

    // UI state
    let show_about_section = false;

    // Simulation control state
    let running = false;
    let loading = false;
    let showUI = true;
    let currentFps = 0;
    let controlsVisible = true;

    // Auto-hide manager
    let autoHideManager: AutoHideManager;
    let eventListeners: { add: () => void; remove: () => void };

    // Event listeners
    let unlistenFps: (() => void) | null = null;

    // Camera controls

    let isMousePressed = false;
    let currentMouseButton = 0;

    // Webcam state
    let webcamDevices: number[] = [];
    let webcamActive = false;

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
            // Toggle local state directly instead of relying on backend state
            showUI = !showUI;

            // Update auto-hide manager state and handle UI toggle
            if (autoHideManager) {
                autoHideManager.updateState({ showUI, running });
                autoHideManager.handleUIToggle(showUI);
            }
        } catch (error) {
            console.error('Failed to toggle GUI:', error);
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
        } catch (error) {
            console.error('Failed to stop simulation:', error);
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

    // Helper function to convert agent count to millions
    const toMillions = (count: number) => count / 1_000_000;
    const fromMillions = (millions: number) => millions * 1_000_000;

    // Computed values
    $: agent_count_millions = toMillions(currentAgentCount);

    // Two-way binding handlers
    async function updateAgentCount(value: number) {
        const newCount = fromMillions(value);
        console.log('Updating agent count: input =', value, 'millions, actual count =', newCount);
        try {
            await invoke('update_agent_count', { count: newCount });
            console.log('Backend update completed, syncing from backend...');
            // Sync the actual agent count from backend
            await syncAgentCountFromBackend();
            console.log('Sync completed, currentAgentCount is now:', currentAgentCount);
        } catch (e) {
            console.error('Failed to update agent count:', e);
        }
    }

    async function updateLutReversed() {
        if (state) {
            state.color_scheme_reversed = !state.color_scheme_reversed;
            try {
                await invoke('update_simulation_state', {
                    stateName: 'color_scheme_reversed',
                    value: state.color_scheme_reversed,
                });
            } catch (e) {
                console.error('Failed to toggle color scheme reversed:', e);
            }
        }
    }

    // Cursor configuration handlers
    async function updateCursorSize(size: number) {
        if (state) {
            state.cursor_size = size;
            try {
                await invoke('update_simulation_state', { stateName: 'cursor_size', value: size });
            } catch (e) {
                console.error('Failed to update cursor size:', e);
            }
        }
    }

    async function updateCursorStrength(strength: number) {
        if (state) {
            state.cursor_strength = strength;
            try {
                await invoke('update_simulation_state', {
                    stateName: 'cursor_strength',
                    value: strength,
                });
            } catch (e) {
                console.error('Failed to update cursor strength:', e);
            }
        }
    }

    async function handleMaskPattern(e: CustomEvent) {
        const value = e.detail.value;
        if (state) {
            state.mask_pattern = value;
            try {
                await invoke('update_simulation_state', {
                    stateName: 'mask_pattern',
                    value: state.mask_pattern,
                });
            } catch (err) {
                console.error('Failed to update mask pattern:', err);
            }
        }
    }

    async function handleMaskTarget(e: CustomEvent) {
        const value = e.detail.value;
        if (state) {
            state.mask_target = value;
            try {
                await invoke('update_simulation_state', {
                    stateName: 'mask_target',
                    value: state.mask_target,
                });
            } catch (err) {
                console.error('Failed to update mask target:', err);
            }
        }
    }

    async function handleMaskMirrorHorizontal(checked: boolean) {
        if (state) {
            state.mask_mirror_horizontal = checked;
            try {
                await invoke('update_simulation_state', {
                    stateName: 'mask_mirror_horizontal',
                    value: state.mask_mirror_horizontal,
                });
            } catch (err) {
                console.error('Failed to update mask mirror horizontal:', err);
            }
        }
    }

    async function handleMaskMirrorVertical(checked: boolean) {
        if (state) {
            state.mask_mirror_vertical = checked;
            try {
                await invoke('update_simulation_state', {
                    stateName: 'mask_mirror_vertical',
                    value: state.mask_mirror_vertical,
                });
            } catch (err) {
                console.error('Failed to update mask mirror vertical:', err);
            }
        }
    }

    async function handleMaskInvertTone(checked: boolean) {
        if (state) {
            state.mask_invert_tone = checked;
            try {
                await invoke('update_simulation_state', {
                    stateName: 'mask_invert_tone',
                    value: state.mask_invert_tone,
                });
            } catch (err) {
                console.error('Failed to update mask invert tone:', err);
            }
        }
    }

    async function updatePreset(value: string) {
        current_preset = value;
        try {
            await invoke('apply_preset', { presetName: value });
            await invoke('reset_trails'); // Clear all existing trails
            await syncSettingsFromBackend(); // Sync UI with new settings
            // Reset agents asynchronously to avoid blocking the UI
            invoke('reset_agents').catch((e) => console.error('Failed to reset agents:', e));
            console.log(`Applied preset: ${value}`);
        } catch (e) {
            console.error('Failed to apply preset:', e);
        }
    }

    async function savePreset(presetName: string) {
        try {
            await invoke('save_preset', { presetName: presetName.trim() });
            // Refresh the available presets list
            await loadAvailablePresets();
            // Set the current preset to the newly saved one
            current_preset = presetName.trim();
            console.log(`Saved preset: ${presetName}`);
        } catch (e) {
            console.error('Failed to save preset:', e);
        }
    }

    // Load available presets from backend
    async function loadAvailablePresets() {
        try {
            available_presets = await invoke('get_presets_for_simulation_type', {
                simulationType: 'slime_mold',
            });
            console.log('Available presets loaded:', available_presets);
            if (available_presets.length > 0 && !current_preset) {
                current_preset = available_presets[0];
                console.log('Set initial preset to:', current_preset);
            }
        } catch (e) {
            console.error('Failed to load available presets:', e);
        }
    }

    // Load available color schemes from backend
    async function loadAvailableLuts() {
        try {
            available_luts = await invoke('get_available_color_schemes');
            console.log('Available color schemes loaded:', available_luts.length);
        } catch (e) {
            console.error('Failed to load available color schemes:', e);
        }
    }

    // Sync settings from backend to frontend
    async function syncSettingsFromBackend() {
        try {
            const backendSettings = await invoke('get_current_settings');
            if (backendSettings) {
                settings = backendSettings as Settings;
            }
        } catch (e) {
            console.error('Failed to sync settings from backend:', e);
        }
    }

    // Sync state from backend to frontend
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

    // Sync agent count separately from settings
    async function syncAgentCountFromBackend() {
        try {
            const agentCount = await invoke('get_current_agent_count');
            console.log('Backend returned agent count:', agentCount);
            if (agentCount !== null && agentCount !== undefined) {
                console.log('Updating currentAgentCount from', currentAgentCount, 'to', agentCount);
                currentAgentCount = agentCount as number;
            }
        } catch (e) {
            console.error('Failed to sync agent count from backend:', e);
        }
    }

    // Webcam functions
    async function loadWebcamDevices() {
        try {
            webcamDevices = await invoke('get_available_webcam_devices');
            console.log('Available webcam devices:', webcamDevices);
        } catch (e) {
            console.error('Failed to load webcam devices:', e);
        }
    }

    async function startWebcamCapture() {
        try {
            await invoke('start_slime_mold_webcam_capture');
            webcamActive = true;
            console.log('Webcam capture started');
        } catch (e) {
            console.error('Failed to start webcam capture:', e);
        }
    }

    async function stopWebcamCapture() {
        try {
            await invoke('stop_slime_mold_webcam_capture');
            webcamActive = false;
            console.log('Webcam capture stopped');
        } catch (e) {
            console.error('Failed to stop webcam capture:', e);
        }
    }

    async function startSimulation() {
        if (running || loading) return;

        loading = true;

        try {
            await invoke('start_slime_mold_simulation');
            loading = false;
            running = true;

            // Backend now handles the render loop, we just track state
            currentFps = 0;
        } catch (e) {
            console.error('Failed to start simulation:', e);
            loading = false;
            running = false;
        }
    }

    onMount(async () => {
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

        // Start the simulation first
        await startSimulation();

        // Load available presets and color schemes
        await loadAvailablePresets();
        await loadAvailableLuts();

        // Load webcam devices
        await loadWebcamDevices();

        // Sync settings and state from backend
        await syncSettingsFromBackend();
        await syncStateFromBackend();
        await syncAgentCountFromBackend();

        // Listen for FPS updates
        unlistenFps = await listen('fps-update', (event: { payload: number }) => {
            currentFps = event.payload;
        });
    });

    onDestroy(async () => {
        // Clean up the simulation
        try {
            await invoke('destroy_simulation');
        } catch (error) {
            console.error('Failed to destroy simulation on component destroy:', error);
        }

        if (unlistenFps) {
            unlistenFps();
        }

        // Clean up auto-hide functionality
        if (eventListeners) {
            eventListeners.remove();
        }
        if (autoHideManager) {
            autoHideManager.cleanup();
        }
    });

    async function updateLutName(value: string) {
        if (state) {
            state.current_color_scheme = value;
            try {
                await invoke('update_simulation_state', {
                    stateName: 'current_color_scheme',
                    value: state.current_color_scheme,
                });
            } catch (e) {
                console.error('Failed to update color scheme name:', e);
            }
        }
    }

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
                `Mouse interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), raw: (${mouseEvent.clientX}, ${mouseEvent.clientY})`
            );

            try {
                await invoke('handle_mouse_interaction_screen', {
                    screenX: physicalCursorX,
                    screenY: physicalCursorY,
                    mouseButton: mouseEvent.button,
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
                try {
                    await invoke('handle_mouse_interaction_screen', {
                        screenX: physicalCursorX,
                        screenY: physicalCursorY,
                        mouseButton: currentMouseButton,
                    });
                } catch (e) {
                    console.error('Failed to handle mouse interaction:', e);
                }
            }
        } else if (event.type === 'mouseup') {
            // Only handle mouseup if we were actually tracking a mouse press
            if (isMousePressed) {
                isMousePressed = false;

                // Stop cursor interaction when mouse is released
                try {
                    await invoke('handle_mouse_release', { mouseButton: currentMouseButton });
                } catch (e) {
                    console.error('Failed to stop mouse interaction:', e);
                }
            }
        } else if (event.type === 'contextmenu') {
            // Handle context menu as right-click for simulation interaction
            const mouseEvent = event as MouseEvent;

            // Convert screen coordinates to world coordinates
            const devicePixelRatio = window.devicePixelRatio || 1;
            const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
            const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

            console.log(
                `Slime Mold context menu interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: 2`
            );

            // Track as active right-button press to ensure release is generated later
            isMousePressed = true;
            currentMouseButton = 2;

            try {
                await invoke('handle_mouse_interaction_screen', {
                    screenX: physicalCursorX,
                    screenY: physicalCursorY,
                    mouseButton: 2, // Right mouse button
                });
            } catch (e) {
                console.error('Failed to handle Slime Mold context menu interaction:', e);
            }
        }
    }
</script>

<style>
    /* SlimeMold specific styles */

    fieldset {
        border: 1px solid #ccc;
        border-radius: 4px;
        padding: 0.5rem;
        margin-bottom: 0.5rem;
    }

    legend {
        font-weight: bold;
        padding: 0 0.3rem;
    }

    .control-group {
        margin-bottom: 0.5rem;
    }

    label {
        display: block;
        margin-bottom: 0.25rem;
    }

    /* Key/Value pair settings layout */
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

    .visually-hidden {
        position: absolute;
        width: 1px;
        height: 1px;
        margin: -1px;
        border: 0;
        padding: 0;
        white-space: nowrap;
        clip-path: inset(100%);
        clip: rect(0 0 0 0);
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
</style>
