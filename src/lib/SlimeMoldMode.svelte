<SimulationLayout
    simulationName="Slime Mold"
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
    on:userInteraction={() => autoHideManager?.handleUserInteraction()}
    on:mouseEvent={handleMouseEvent}
>
    {#if settings}
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
                        current_color_scheme={lut_name}
                        reversed={lut_reversed}
                        on:select={({ detail }) => updateLutName(detail.name)}
                        on:reverse={() => updateLutReversed()}
                    />
                </div>
            </fieldset>

            <!-- Post Processing -->
            <PostProcessingMenu simulationType="slime_mold" />

            <!-- Controls -->
            <fieldset>
                <legend>Controls</legend>
                <div class="interaction-controls-grid">
                    <div class="interaction-help">
                        <div class="control-group">
                            <span>🖱️ Left click: Attract agents | Right click: Repel agents</span>
                        </div>
                        <div class="control-group">
                            <Button
                                variant="default"
                                on:click={() => dispatch('navigate', 'how-to-play')}
                            >
                                📖 Camera Controls
                            </Button>
                        </div>
                        <div class="control-group">
                            <span
                                >Camera controls not working? Click the control bar at the top of
                                the screen.</span
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
                            sizeMin={10}
                            sizeMax={500}
                            sizeStep={5}
                            strengthMin={0}
                            strengthMax={50}
                            strengthStep={0.5}
                            sizePrecision={0}
                            strengthPrecision={1}
                            on:sizechange={(e) => updateCursorSize(e.detail)}
                            on:strengthchange={(e) => updateCursorStrength(e.detail)}
                        />
                    </div>
                </div>
            </fieldset>

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
                            bind:value={position_generator}
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
                                try {
                                    await invoke('update_simulation_setting', {
                                        settingName: 'position_generator',
                                        value: e.detail.value,
                                    });
                                    await syncSettingsFromBackend();
                                } catch (err) {
                                    console.error('Failed to update position generator:', err);
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
                    {#if position_generator === 'Image'}
                        <div class="control-group">
                            <ImageSelector
                                fitMode={settings.position_image_fit_mode}
                                loadCommand="load_slime_mold_position_image"
                                showMirrorHorizontal={false}
                                showInvertTone={false}
                                onFitModeChange={async (value) => {
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
                            <span class="setting-label">Decay Rate:</span>
                            <NumberDragBox
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
                            <span class="setting-label">Deposition Rate:</span>
                            <NumberDragBox
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
                            <span class="setting-label">Diffusion Rate:</span>
                            <NumberDragBox
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
                            <span class="setting-label">Agent Count (millions):</span>
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
                            <span class="setting-label">Min Speed:</span>
                            <NumberDragBox
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
                            <span class="setting-label">Max Speed:</span>
                            <NumberDragBox
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
                            <span class="setting-label">Turn Rate:</span>
                            <NumberDragBox
                                value={(settings.agent_turn_rate * 180) / Math.PI}
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
                            <span class="setting-label">Jitter:</span>
                            <NumberDragBox
                                bind:value={settings.agent_jitter}
                                min={0}
                                max={5}
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
                            <span class="setting-label">Sensor Angle:</span>
                            <NumberDragBox
                                value={(settings.agent_sensor_angle * 180) / Math.PI}
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
                            <span class="setting-label">Sensor Distance:</span>
                            <NumberDragBox
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

                <!-- Gradient Settings -->
                <div class="settings-section">
                    <h3 class="section-header">Gradient</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <span class="setting-label">Gradient Type:</span>
                            <Selector
                                options={[
                                    'disabled',
                                    'radial',
                                    'linear',
                                    'ellipse',
                                    'spiral',
                                    'checkerboard',
                                    'image',
                                ]}
                                bind:value={settings.gradient_type}
                                on:change={handleGradientType}
                            />
                        </div>
                        {#if settings.gradient_type !== 'disabled'}
                            <div class="setting-item">
                                <span class="setting-label">Strength:</span>
                                <NumberDragBox
                                    bind:value={settings.gradient_strength}
                                    min={0}
                                    max={2}
                                    step={0.01}
                                    precision={2}
                                    on:change={async (e) => {
                                        try {
                                            await invoke('update_simulation_setting', {
                                                settingName: 'gradient_strength',
                                                value: e.detail,
                                            });
                                        } catch (err) {
                                            console.error(
                                                'Failed to update gradient strength:',
                                                err
                                            );
                                        }
                                    }}
                                />
                            </div>
                            {#if settings.gradient_type !== 'image'}
                                <div class="setting-item">
                                    <span class="setting-label">Center X:</span>
                                    <NumberDragBox
                                        value={gradient_center_x_percent}
                                        min={0}
                                        max={100}
                                        step={1}
                                        precision={0}
                                        on:change={(e) => updateGradientCenterX(e.detail)}
                                    />
                                </div>
                                <div class="setting-item">
                                    <span class="setting-label">Center Y:</span>
                                    <NumberDragBox
                                        value={gradient_center_y_percent}
                                        min={0}
                                        max={100}
                                        step={1}
                                        precision={0}
                                        on:change={(e) => updateGradientCenterY(e.detail)}
                                    />
                                </div>
                                <div class="setting-item">
                                    <span class="setting-label">Size:</span>
                                    <NumberDragBox
                                        bind:value={settings.gradient_size}
                                        min={0.1}
                                        max={2}
                                        step={0.01}
                                        precision={2}
                                        on:change={async (e) => {
                                            try {
                                                await invoke('update_simulation_setting', {
                                                    settingName: 'gradient_size',
                                                    value: e.detail,
                                                });
                                            } catch (err) {
                                                console.error(
                                                    'Failed to update gradient size:',
                                                    err
                                                );
                                            }
                                        }}
                                    />
                                </div>
                                <div class="setting-item">
                                    <span class="setting-label">Angle:</span>
                                    <NumberDragBox
                                        bind:value={settings.gradient_angle}
                                        min={0}
                                        max={360}
                                        step={1}
                                        precision={0}
                                        unit="°"
                                        on:change={async (e) => {
                                            try {
                                                await invoke('update_simulation_setting', {
                                                    settingName: 'gradient_angle',
                                                    value: e.detail,
                                                });
                                            } catch (err) {
                                                console.error(
                                                    'Failed to update gradient angle:',
                                                    err
                                                );
                                            }
                                        }}
                                    />
                                </div>
                            {:else}
                                <ImageSelector
                                    fitMode={settings.gradient_image_fit_mode}
                                    mirrorHorizontal={settings?.gradient_image_mirror_horizontal ||
                                        false}
                                    invertTone={settings?.gradient_image_invert_tone || false}
                                    loadCommand="load_slime_mold_gradient_image"
                                    onFitModeChange={async (value) => {
                                        try {
                                            await invoke('update_simulation_setting', {
                                                settingName: 'gradient_image_fit_mode',
                                                value: value,
                                            });
                                        } catch (err) {
                                            console.error('Failed to update fit mode:', err);
                                        }
                                    }}
                                    onMirrorHorizontalChange={async (value) => {
                                        try {
                                            await invoke('update_simulation_setting', {
                                                settingName: 'gradient_image_mirror_horizontal',
                                                value: value,
                                            });
                                            if (settings)
                                                settings.gradient_image_mirror_horizontal = value;
                                        } catch (err) {
                                            console.error('Failed to update mirror:', err);
                                        }
                                    }}
                                    onInvertToneChange={async (value) => {
                                        try {
                                            await invoke('update_simulation_setting', {
                                                settingName: 'gradient_image_invert_tone',
                                                value: value,
                                            });
                                            if (settings)
                                                settings.gradient_image_invert_tone = value;
                                        } catch (err) {
                                            console.error('Failed to update invert:', err);
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
    import CursorConfig from './components/shared/CursorConfig.svelte';
    import SimulationLayout from './components/shared/SimulationLayout.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
    import PresetFieldset from './components/shared/PresetFieldset.svelte';
    import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
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

    // Simulation state
    let settings: any | undefined = undefined;

    // State (not saved in presets)
    let position_generator = 'Random';

    // LUT state (runtime, not saved in presets)
    let lut_name: string;
    let lut_reversed = true;

    // Agent count tracked separately (not part of preset settings)
    let currentAgentCount = 1_000_000;

    // Cursor interaction state (runtime, not saved in presets)
    let cursorSize: number = 300.0; // Default cursor size (matches backend)
    let cursorStrength: number = 5.0; // Default cursor strength (matches backend)

    // Preset and LUT state
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
    $: gradient_center_x_percent = settings?.gradient_center_x
        ? settings.gradient_center_x * 100
        : 50;
    $: gradient_center_y_percent = settings?.gradient_center_y
        ? settings.gradient_center_y * 100
        : 50;

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

    async function updateGradientCenterX(value: number) {
        if (settings) {
            settings.gradient_center_x = value / 100;
            try {
                await invoke('update_simulation_setting', {
                    settingName: 'gradient_center_x',
                    value: settings.gradient_center_x,
                });
            } catch (e) {
                console.error('Failed to update gradient center X:', e);
            }
        }
    }

    async function updateGradientCenterY(value: number) {
        if (settings) {
            settings.gradient_center_y = value / 100;
            try {
                await invoke('update_simulation_setting', {
                    settingName: 'gradient_center_y',
                    value: settings.gradient_center_y,
                });
            } catch (e) {
                console.error('Failed to update gradient center Y:', e);
            }
        }
    }

    async function updateLutReversed() {
        try {
            await invoke('toggle_color_scheme_reversed');
            await syncSettingsFromBackend(); // Sync UI with backend
        } catch (e) {
            console.error('Failed to toggle LUT reversed:', e);
        }
    }

    // Cursor configuration handlers
    async function updateCursorSize(size: number) {
        cursorSize = size;
        try {
            await invoke('update_simulation_setting', { settingName: 'cursor_size', value: size });
        } catch (e) {
            console.error('Failed to update cursor size:', e);
        }
    }

    async function updateCursorStrength(strength: number) {
        cursorStrength = strength;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'cursor_strength',
                value: strength,
            });
        } catch (e) {
            console.error('Failed to update cursor strength:', e);
        }
    }

    async function handleGradientType(e: CustomEvent) {
        const value = e.detail.value;
        if (settings) {
            settings.gradient_type = value;
            try {
                await invoke('update_simulation_setting', {
                    settingName: 'gradient_type',
                    value: settings.gradient_type,
                });
            } catch (err) {
                console.error('Failed to update gradient type:', err);
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

    // Load available LUTs from backend
    async function loadAvailableLuts() {
        try {
            available_luts = await invoke('get_available_color_schemes');
            console.log('Available LUTs loaded:', available_luts.length);
        } catch (e) {
            console.error('Failed to load available LUTs:', e);
        }
    }

    // Sync settings from backend to frontend
    async function syncSettingsFromBackend() {
        try {
            const backendSettings = await invoke('get_current_settings');
            const backendState = await invoke('get_current_state');

            if (backendSettings) {
                // Use backend settings directly
                settings = backendSettings as Record<string, unknown>;
            }

            if (backendState) {
                // Update LUT-related settings from state
                const state = backendState as {
                    current_lut_name?: string;
                    lut_reversed?: boolean;
                    cursor_size?: number;
                    cursor_strength?: number;
                    position_generator?: string;
                };
                if (state.current_lut_name !== undefined) {
                    lut_name = state.current_lut_name;
                }
                if (state.lut_reversed !== undefined) {
                    lut_reversed = state.lut_reversed;
                }

                // Update cursor configuration from state
                if (state.cursor_size !== undefined) {
                    cursorSize = state.cursor_size;
                }
                if (state.cursor_strength !== undefined) {
                    cursorStrength = state.cursor_strength;
                }

                // Update position generator from state
                if (state.position_generator !== undefined) {
                    position_generator = state.position_generator;
                }
            }
        } catch (e) {
            console.error('Failed to sync settings from backend:', e);
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

        // Set up event listeners BEFORE starting simulation to avoid race conditions

        // Start the simulation first
        await startSimulation();

        // Load available presets and LUTs
        await loadAvailablePresets();
        await loadAvailableLuts();

        // Load webcam devices
        await loadWebcamDevices();

        // Sync settings from backend
        await syncSettingsFromBackend();
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
        try {
            await invoke('apply_color_scheme_by_name', { colorSchemeName: value });
            await syncSettingsFromBackend(); // Sync UI with backend state
        } catch (e) {
            console.error('Failed to update LUT name:', e);
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

    /* Checkbox styling */
    .checkbox {
        display: flex;
        align-items: center;
        gap: 8px;
        cursor: pointer;
    }

    .checkbox input[type='checkbox'] {
        margin: 0;
    }
</style>
