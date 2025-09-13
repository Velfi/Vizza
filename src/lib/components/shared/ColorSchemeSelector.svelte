<div class="color-scheme-selector">
    <div class="color-scheme-controls">
        <Selector
            options={available_color_schemes}
            bind:value={current_color_scheme}
            on:change={handleSelect}
        />
        <button
            type="button"
            class="control-btn reverse-btn"
            class:reversed
            on:click={handleReverse}
            title="Reverse Color Scheme"
        >
            Reverse
        </button>
        <button
            type="button"
            class="control-btn gradient-btn"
            on:click={openGradientEditor}
            title="Create Custom Color Scheme"
        >
            🎨
        </button>
    </div>
</div>

{#if show_gradient_editor}
    <div
        class="gradient-editor-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="gradient-editor-title"
        tabindex="-1"
        on:keydown={(e) => e.key === 'Escape' && closeGradientEditor()}
        use:portalToBody
    >
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
            class="dialog-content gradient-editor-content"
            role="document"
            on:click|stopPropagation
        >
            <h3 id="gradient-editor-title">Color Scheme Editor</h3>

            <!-- Color Scheme Name Input (styled like Gradient Editor sim) -->
            <div class="control-group">
                <label for="customLutName">Name</label>
                <input
                    type="text"
                    id="customColorSchemeName"
                    bind:value={custom_color_scheme_name}
                    placeholder="Color scheme name"
                    class="color-scheme-name-input"
                />
            </div>

            <!-- Preset Selector (compact) -->
            <div class="control-group">
                <label for="presetSelector">Preset</label>
                <Selector
                    id="presetSelector"
                    options={['Custom', 'Rainbow', 'Heat', 'Cool', 'Viridis', 'Plasma', 'Inferno']}
                    bind:value={selectedPreset}
                    on:change={applyPreset}
                />
            </div>

            <!-- Color Space (compact, like Gradient Editor sim) -->
            <div class="control-group">
                <label for="colorSpaceTop">Space</label>
                <Selector
                    id="colorSpaceTop"
                    options={['RGB', 'Lab', 'OkLab', 'Jzazbz', 'HSLuv']}
                    bind:value={selectedColorSpace}
                    on:change={handleColorSpaceChange}
                />
            </div>

            <!-- Interpolation Mode -->
            <div class="control-group">
                <label for="interpolationModeTop">Interpolation</label>
                <Selector
                    id="interpolationModeTop"
                    options={['Smooth', 'Stepped']}
                    bind:value={interpolationMode}
                    on:change={updateGradientPreview}
                />
            </div>

            <!-- Gradient Preview -->
            <div class="gradient-preview-container">
                <div
                    class="gradient-preview"
                    style="background: {gradientBackgroundCss}"
                    role="button"
                    tabindex="0"
                    aria-label="Gradient preview - double-click to add color stops"
                    on:dblclick={(e) => {
                        const rect = e.currentTarget.getBoundingClientRect();
                        const position = (e.clientX - rect.left) / rect.width;
                        addGradientStop(position);
                    }}
                    on:keydown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                            e.preventDefault();
                            // Add a stop at the center if activated with keyboard
                            addGradientStop(0.5);
                        }
                    }}
                >
                    {#each gradientStops as stop, index}
                        <div
                            class="gradient-stop"
                            class:selected={index === selectedStopIndex}
                            class:dragging={isDragging && dragStopIndex === index}
                            class:no-transition={isAddingStop}
                            style="left: {stop.position * 100}%; background-color: {stop.color}"
                            role="button"
                            tabindex="0"
                            aria-label="Color stop {index + 1} at {Math.round(
                                stop.position * 100
                            )}% - click to select"
                            on:mousedown={(e) => handleStopMouseDown(e, index)}
                            on:click|stopPropagation={() => (selectedStopIndex = index)}
                            on:keydown={(e) => {
                                if (e.key === 'Enter' || e.key === ' ') {
                                    e.preventDefault();
                                    selectedStopIndex = index;
                                }
                            }}
                        ></div>
                    {/each}
                </div>
            </div>

            <!-- Selected Stop Controls -->
            <div class="stop-controls">
                {#if selectedStopIndex >= 0 && selectedStopIndex < gradientStops.length}
                    <h4>Color Stop {selectedStopIndex + 1}</h4>

                    <div class="control-row">
                        <div class="control-group">
                            <label for="stopColor">Color</label>
                            <input
                                type="color"
                                id="stopColor"
                                value={gradientStops[selectedStopIndex].color}
                                on:input={(e) => {
                                    const color = (e.target as HTMLInputElement).value;
                                    updateStopColor(selectedStopIndex, color);
                                }}
                                class="color-input"
                            />
                        </div>
                        <div class="control-group">
                            <label for="positionSlider">Position</label>
                            <input
                                type="range"
                                id="positionSlider"
                                min="0"
                                max="1"
                                step="0.01"
                                bind:value={gradientStops[selectedStopIndex].position}
                                on:input={handlePositionInput}
                                class="position-slider"
                            />
                        </div>
                        {#if gradientStops.length > 2}
                            <div class="control-group">
                                <span class="section-label">&nbsp;</span>
                                <button
                                    type="button"
                                    class="delete-stop-btn"
                                    on:click={removeSelectedStop}
                                    title="Delete this color stop"
                                >
                                    🗑️ Delete Stop
                                </button>
                            </div>
                        {/if}
                    </div>
                {:else}
                    <div class="editor-instructions">
                        <p><strong>How to use:</strong></p>
                        <ul>
                            <li>Double-click on the gradient to add new color stops</li>
                            <li>Click on a color stop to select and edit it</li>
                            <li>Drag color stops to reposition them</li>
                            <li>Use the delete button to remove selected stops</li>
                            <li>Choose a color space for interpolation (RGB, Lab, or OkLab)</li>
                        </ul>
                    </div>
                {/if}
            </div>

            <!-- Random Generator (compact) -->
            <div class="random-controls">
                <div class="control-row">
                    <div class="control-group">
                        <label for="randScheme">Random Scheme</label>
                        <Selector
                            id="randScheme"
                            options={[
                                'Basic',
                                'Warm',
                                'Cool',
                                'Pastel',
                                'Neon',
                                'Earth',
                                'Monochrome',
                                'Complementary',
                                'Truly Random',
                            ]}
                            bind:value={selectedRandomScheme}
                        />
                    </div>
                    <div class="control-group">
                        <label for="randPlacement">Stop Placement</label>
                        <Selector
                            id="randPlacement"
                            options={['Random', 'Even']}
                            bind:value={randomStopPlacement}
                        />
                    </div>
                </div>
                <div class="control-row">
                    <div class="control-group">
                        <label for="stopCount">Stops: {randomStopCount}</label>
                        <input
                            id="stopCount"
                            type="range"
                            min="2"
                            max="16"
                            step="1"
                            bind:value={randomStopCount}
                        />
                    </div>
                    <div class="control-group">
                        <span class="section-label">&nbsp;</span>
                        <button
                            type="button"
                            class="primary-button"
                            on:click={() => randomizeGradient(selectedRandomScheme)}
                        >
                            Generate
                        </button>
                    </div>
                </div>
            </div>

            <!-- Dialog Actions -->
            <div class="dialog-actions">
                <button
                    type="button"
                    class="primary-button"
                    on:click={saveCustomColorScheme}
                    disabled={!custom_color_scheme_name.trim()}
                >
                    💾 Save Color Scheme
                </button>
                <button type="button" class="secondary-button" on:click={closeGradientEditor}>
                    Cancel
                </button>
            </div>
        </div>
    </div>
{/if}

<script lang="ts">
    import { createEventDispatcher, tick } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import Selector from '../inputs/Selector.svelte';
    import { interpolate, formatHex } from 'culori';

    // Portal the dialog to document.body so it is not clipped by parent containers
    function portalToBody(node: HTMLElement) {
        if (typeof document !== 'undefined') {
            document.body.appendChild(node);
        }
        return {
            destroy() {
                // Remove from DOM without restoring to original parent to avoid layout flashes
                if (node.parentNode) node.parentNode.removeChild(node);
            },
        };
    }

    export let available_color_schemes: string[] = [];
    export let current_color_scheme: string = '';
    export let reversed: boolean = false;

    const dispatch = createEventDispatcher<{
        select: { name: string };
        reverse: { reversed: boolean };
    }>();

    // Gradient editor state
    let show_gradient_editor = false;
    let custom_color_scheme_name = '';
    let gradientStops = [
        { position: 0, color: '#000000' },
        { position: 1, color: '#ffffff' },
    ];
    let selectedStopIndex = -1;
    let isDragging = false;
    let dragStopIndex = -1;
    let original_color_scheme_name = ''; // Store the original color scheme name to restore on cancel
    let isAddingStop = false; // Flag to track when a new stop is being added
    let selectedColorSpace: 'RGB' | 'Lab' | 'OkLab' | 'Jzazbz' | 'HSLuv' = 'OkLab'; // Default to OkLab for better perceptual uniformity
    let selectedPreset = 'Custom';
    let selectedRandomScheme: string = 'Basic';
    let randomStopPlacement: 'Even' | 'Random' = 'Random';
    let randomStopCount: number = 3;
    let interpolationMode: 'Smooth' | 'Stepped' = 'Smooth';
    let gradientBackgroundCss = '';

    // Reactive statements to handle prop changes
    // Note: Don't auto-select the first LUT when current_lut is empty,
    // let the parent component set the initial LUT from backend state

    function handleSelect({ detail }: { detail: { value: string } }) {
        const selectedName = detail.value;
        console.log(`LutSelector: Selected ${selectedName}, was ${current_color_scheme}`);
        current_color_scheme = selectedName; // Update local state
        dispatch('select', { name: selectedName });
    }

    async function handleReverse() {
        reversed = !reversed;
        console.log(`LutSelector: Reversing to ${reversed}, current LUT: ${current_color_scheme}`);
        dispatch('reverse', { reversed });
    }

    // Function to open gradient editor and apply initial gradient
    async function openGradientEditor() {
        original_color_scheme_name = current_color_scheme; // Store the original color scheme name
        show_gradient_editor = true;

        // Apply the initial gradient preview immediately
        await updateGradientPreview();
    }

    // Function to close gradient editor and restore original color scheme
    async function closeGradientEditor() {
        show_gradient_editor = false;
        custom_color_scheme_name = '';

        try {
            // Restore the original color scheme to ensure it's properly applied
            if (original_color_scheme_name) {
                await invoke('apply_color_scheme_by_name', {
                    colorSchemeName: original_color_scheme_name,
                });
                console.log(`Restored original color scheme: ${original_color_scheme_name}`);
            }
        } catch (e) {
            console.error('Failed to restore original color scheme:', e);
        }
    }

    // Gradient editor functions
    // Function to add a gradient stop without transition
    async function addGradientStop(position: number) {
        // Find the color at this position
        const color = getColorAtPosition(position);

        // Set flag to prevent transition on new stops
        isAddingStop = true;

        gradientStops = [...gradientStops, { position, color }];
        gradientStops.sort((a, b) => a.position - b.position);

        // Reset flag after a short delay to allow rendering
        await tick();
        isAddingStop = false;

        updateGradientPreview();
    }

    async function removeGradientStop(index: number) {
        if (gradientStops.length <= 2) return;

        // Set flag to prevent transition on stop removal
        isAddingStop = true;

        gradientStops = gradientStops.filter((_, i) => i !== index);
        if (selectedStopIndex === index) {
            selectedStopIndex = -1;
        } else if (selectedStopIndex > index) {
            selectedStopIndex = selectedStopIndex - 1;
        }

        // Reset flag after a short delay to allow rendering
        await tick();
        isAddingStop = false;

        updateGradientPreview();
    }

    function updateStopColor(index: number, color: string) {
        gradientStops[index].color = color;
        updateGradientPreview();
    }

    // Handle position slider input: keep selection stable and regenerate preview
    function handlePositionInput() {
        gradientStops = [...gradientStops].sort((a, b) => a.position - b.position);
        selectedStopIndex = gradientStops.findIndex(
            (stop) => Math.abs(stop.position - gradientStops[selectedStopIndex].position) < 0.001
        );
        updateGradientPreview();
    }

    function getColorAtPosition(position: number): string {
        // Find the two stops that bound this position
        let leftStop = gradientStops[0];
        let rightStop = gradientStops[gradientStops.length - 1];

        for (let i = 0; i < gradientStops.length - 1; i++) {
            if (
                gradientStops[i].position <= position &&
                gradientStops[i + 1].position >= position
            ) {
                leftStop = gradientStops[i];
                rightStop = gradientStops[i + 1];
                break;
            }
        }

        // Interpolate or step between the two colors
        const t = (position - leftStop.position) / (rightStop.position - leftStop.position);
        if (interpolationMode === 'Stepped') return leftStop.color;
        return interpolateColor(leftStop.color, rightStop.color, t);
    }

    // Simplified color interpolation using culori
    function interpolateColor(color1: string, color2: string, t: number): string {
        let colorSpace = 'rgb';

        switch (selectedColorSpace) {
            case 'RGB':
                colorSpace = 'rgb';
                break;
            case 'Lab':
                colorSpace = 'lab';
                break;
            case 'OkLab':
                colorSpace = 'oklab';
                break;
            case 'Jzazbz':
                colorSpace = 'jzazbz';
                break;
            case 'HSLuv':
                colorSpace = 'hsluv';
                break;
            default:
                throw new Error(`Unsupported color space: ${selectedColorSpace}`);
        }

        // Create interpolator
        const interpolator = interpolate([color1, color2], colorSpace);
        const result = interpolator(t);

        if (!result) {
            throw new Error(`Failed to interpolate colors in ${colorSpace} color space`);
        }

        // Convert result to hex
        const hexResult = formatHex(result);
        if (!hexResult) {
            throw new Error(`Failed to convert interpolated color to hex format`);
        }

        return hexResult;
    }

    // Reverse entire gradient
    function reverseGradient() {
        gradientStops = gradientStops
            .map((stop) => ({ ...stop, position: 1 - stop.position }))
            .sort((a, b) => a.position - b.position);
        updateGradientPreview();
    }

    // Export color scheme as .lut text file
    function exportColorScheme() {
        const lutData: number[] = [];
        for (let i = 0; i < 256; i++) {
            const t = i / 255;
            const color = getColorAtPosition(t);
            const r = parseInt(color.slice(1, 3), 16);
            const g = parseInt(color.slice(3, 5), 16);
            const b = parseInt(color.slice(5, 7), 16);
            lutData.push(r, g, b);
        }
        const dataStr = lutData.join('\n');
        const dataBlob = new Blob([dataStr], { type: 'text/plain' });
        const url = URL.createObjectURL(dataBlob);
        const link = document.createElement('a');
        link.href = url;
        link.download = `${custom_color_scheme_name || 'custom'}.lut`;
        link.click();
        URL.revokeObjectURL(url);
    }

    function applyPreset() {
        switch (selectedPreset) {
            case 'Rainbow':
                gradientStops = [
                    { position: 0, color: '#ff0000' },
                    { position: 0.17, color: '#ff8000' },
                    { position: 0.33, color: '#ffff00' },
                    { position: 0.5, color: '#00ff00' },
                    { position: 0.67, color: '#0080ff' },
                    { position: 0.83, color: '#8000ff' },
                    { position: 1, color: '#ff0080' },
                ];
                break;
            case 'Heat':
                gradientStops = [
                    { position: 0, color: '#000000' },
                    { position: 0.5, color: '#ff0000' },
                    { position: 1, color: '#ffff00' },
                ];
                break;
            case 'Cool':
                gradientStops = [
                    { position: 0, color: '#0000ff' },
                    { position: 0.5, color: '#00ffff' },
                    { position: 1, color: '#ffffff' },
                ];
                break;
            case 'Viridis':
                gradientStops = [
                    { position: 0, color: '#440154' },
                    { position: 0.25, color: '#31688e' },
                    { position: 0.5, color: '#35b779' },
                    { position: 0.75, color: '#fde725' },
                    { position: 1, color: '#fde725' },
                ];
                break;
            case 'Plasma':
                gradientStops = [
                    { position: 0, color: '#0d0887' },
                    { position: 0.25, color: '#7e03a8' },
                    { position: 0.5, color: '#cc4778' },
                    { position: 0.75, color: '#f89441' },
                    { position: 1, color: '#f0f921' },
                ];
                break;
            case 'Inferno':
                gradientStops = [
                    { position: 0, color: '#000004' },
                    { position: 0.25, color: '#1b0c41' },
                    { position: 0.5, color: '#4a0c6b' },
                    { position: 0.75, color: '#781c6d' },
                    { position: 1, color: '#ed6925' },
                ];
                break;
        }
        selectedStopIndex = 0;
        updateGradientPreview();
    }

    function hslToHex(h: number, s: number, l: number): string {
        s /= 100;
        l /= 100;
        const c = (1 - Math.abs(2 * l - 1)) * s;
        const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
        const m = l - c / 2;
        let r = 0;
        let g = 0;
        let b = 0;
        if (h < 60) {
            r = c;
            g = x;
            b = 0;
        } else if (h < 120) {
            r = x;
            g = c;
            b = 0;
        } else if (h < 180) {
            r = 0;
            g = c;
            b = x;
        } else if (h < 240) {
            r = 0;
            g = x;
            b = c;
        } else if (h < 300) {
            r = x;
            g = 0;
            b = c;
        } else {
            r = c;
            g = 0;
            b = x;
        }
        r = Math.round((r + m) * 255);
        g = Math.round((g + m) * 255);
        b = Math.round((b + m) * 255);
        const hexR = r.toString(16).padStart(2, '0');
        const hexG = g.toString(16).padStart(2, '0');
        const hexB = b.toString(16).padStart(2, '0');
        return `#${hexR}${hexG}${hexB}`;
    }

    function generateRandomColors(scheme: string, desiredCount?: number): string[] {
        let colors: string[] = [];
        switch (scheme) {
            case 'Basic':
                colors = [
                    '#ff0000',
                    '#00ff00',
                    '#0000ff',
                    '#ffff00',
                    '#ff00ff',
                    '#00ffff',
                    '#ff8000',
                    '#8000ff',
                ];
                break;
            case 'Warm':
                colors = [
                    '#ff4500',
                    '#ff6347',
                    '#ffa500',
                    '#ff8c00',
                    '#dc143c',
                    '#b22222',
                    '#cd853f',
                    '#d2691e',
                ];
                break;
            case 'Cool':
                colors = [
                    '#4169e1',
                    '#0000cd',
                    '#1e90ff',
                    '#00bfff',
                    '#87ceeb',
                    '#20b2aa',
                    '#008b8b',
                    '#4682b4',
                ];
                break;
            case 'Pastel':
                colors = [
                    '#ffb3ba',
                    '#ffdfba',
                    '#ffffba',
                    '#baffc9',
                    '#bae1ff',
                    '#e6baff',
                    '#ffc9ba',
                    '#c9baff',
                ];
                break;
            case 'Neon':
                colors = [
                    '#ff073a',
                    '#39ff14',
                    '#00ffff',
                    '#ff00ff',
                    '#ffff00',
                    '#ff4500',
                    '#8a2be2',
                    '#00ff7f',
                ];
                break;
            case 'Earth':
                colors = [
                    '#8b4513',
                    '#a0522d',
                    '#cd853f',
                    '#daa520',
                    '#b8860b',
                    '#9acd32',
                    '#6b8e23',
                    '#556b2f',
                ];
                break;
            case 'Monochrome': {
                const baseHue = Math.floor(Math.random() * 360);
                colors = [];
                for (let i = 0; i < 8; i++) {
                    const saturation = 50 + Math.random() * 50;
                    const lightness = 20 + Math.random() * 60;
                    colors.push(hslToHex(baseHue, saturation, lightness));
                }
                break;
            }
            case 'Complementary': {
                const hue1 = Math.floor(Math.random() * 360);
                const hue2 = (hue1 + 180) % 360;
                colors = [
                    hslToHex(hue1, 70, 50),
                    hslToHex(hue1, 80, 30),
                    hslToHex(hue1, 60, 70),
                    hslToHex(hue2, 70, 50),
                    hslToHex(hue2, 80, 30),
                    hslToHex(hue2, 60, 70),
                ];
                break;
            }
            case 'Truly Random': {
                const target = Math.max(2, desiredCount ?? 8);
                const set = new Set<string>();
                while (set.size < target) {
                    const r = Math.floor(Math.random() * 256);
                    const g = Math.floor(Math.random() * 256);
                    const b = Math.floor(Math.random() * 256);
                    const hex = `#${r.toString(16).padStart(2, '0')}${g
                        .toString(16)
                        .padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
                    set.add(hex);
                }
                colors = Array.from(set);
                break;
            }
        }
        return colors;
    }

    function generateStopPositions(count: number, placement: 'Even' | 'Random'): number[] {
        if (placement === 'Even') {
            const positions: number[] = [];
            for (let i = 0; i < count; i++) positions.push(i / (count - 1));
            return positions;
        } else {
            const positions: number[] = [0, 1];
            for (let i = 2; i < count; i++) positions.push(0.1 + Math.random() * 0.8);
            return positions.sort((a, b) => a - b);
        }
    }

    function randomizeGradient(scheme: string = 'Basic') {
        const stopCount = randomStopCount;
        const colors = generateRandomColors(scheme, stopCount);
        const positions = generateStopPositions(stopCount, randomStopPlacement);
        const palette = [...colors];
        for (let i = palette.length - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1));
            [palette[i], palette[j]] = [palette[j], palette[i]];
        }
        gradientStops = positions.map((position, i) => ({
            position,
            color: palette[i % palette.length],
        }));
        selectedStopIndex = 0;
        updateGradientPreview();
    }

    // Reference functions to satisfy linter when template analysis misses usage
    void reverseGradient;
    void exportColorScheme;

    function handleStopMouseDown(event: MouseEvent, index: number) {
        event.preventDefault();
        event.stopPropagation();
        isDragging = true;
        dragStopIndex = index;
        selectedStopIndex = index;

        // The container is now the gradient preview itself
        const container = (event.currentTarget as HTMLElement)?.parentElement as HTMLElement;

        const handleMouseMove = (e: MouseEvent) => {
            if (!isDragging || !container) return;

            // Use the container reference and recalculate rect if needed
            const rect = container.getBoundingClientRect();
            const position = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));

            // Update the stop position
            gradientStops[dragStopIndex].position = position;

            // Re-sort stops by position and update the array to trigger reactivity
            gradientStops = [...gradientStops].sort((a, b) => a.position - b.position);

            // Update the drag index to match the new position
            dragStopIndex = gradientStops.findIndex(
                (stop) => Math.abs(stop.position - position) < 0.001
            );

            updateGradientPreview();
        };

        const handleMouseUp = () => {
            isDragging = false;
            document.removeEventListener('mousemove', handleMouseMove);
            document.removeEventListener('mouseup', handleMouseUp);
        };

        document.addEventListener('mousemove', handleMouseMove);
        document.addEventListener('mouseup', handleMouseUp);
    }

    async function updateGradientPreview() {
        try {
            // Build color scheme in [r0..r255, g0..g255, b0..b255] format for preview, as integers 0-255
            const rArr: number[] = [];
            const gArr: number[] = [];
            const bArr: number[] = [];
            for (let i = 0; i < 256; i++) {
                const t = i / 255;
                const interpolatedColor = getColorAtPosition(t);

                const r = parseInt(interpolatedColor.slice(1, 3), 16);
                const g = parseInt(interpolatedColor.slice(3, 5), 16);
                const b = parseInt(interpolatedColor.slice(5, 7), 16);
                rArr.push(r);
                gArr.push(g);
                bArr.push(b);
            }
            const lutData = [...rArr, ...gArr, ...bArr];

            await invoke('update_gradient_preview', { colorSchemeData: lutData });
        } catch (e) {
            console.error('Failed to update gradient preview:', e);
        }
    }

    async function saveCustomColorScheme() {
        if (!custom_color_scheme_name.trim()) return;
        try {
            // Build color scheme in [r0..r255, g0..g255, b0..b255] format as integers
            const rArr: number[] = [];
            const gArr: number[] = [];
            const bArr: number[] = [];
            for (let i = 0; i < 256; i++) {
                const t = i / 255;
                const interpolatedColor = getColorAtPosition(t);
                const r = parseInt(interpolatedColor.slice(1, 3), 16);
                const g = parseInt(interpolatedColor.slice(3, 5), 16);
                const b = parseInt(interpolatedColor.slice(5, 7), 16);
                rArr.push(r);
                gArr.push(g);
                bArr.push(b);
            }
            const lutData = [...rArr, ...gArr, ...bArr];
            await invoke('save_custom_color_scheme', {
                name: custom_color_scheme_name,
                colorSchemeData: lutData,
            });

            // Update current color scheme to the newly saved one
            current_color_scheme = custom_color_scheme_name;

            // Notify parent component about the color scheme change
            dispatch('select', { name: custom_color_scheme_name });

            // Close the editor without restoring the original color scheme
            show_gradient_editor = false;
            custom_color_scheme_name = '';

            // Refresh available color schemes to include the new one
            available_color_schemes = await invoke('get_available_color_schemes');
        } catch (e) {
            console.error('Failed to save custom color scheme:', e);
        }
    }

    function handleColorSpaceChange({ detail }: { detail: { value: string } }) {
        selectedColorSpace = detail.value as 'RGB' | 'Lab' | 'OkLab' | 'Jzazbz' | 'HSLuv';
        updateGradientPreview();
    }

    // Build CSS gradient string depending on interpolation mode
    function buildGradientCSS(): string {
        const stops = [...gradientStops].sort((a, b) => a.position - b.position);
        if (stops.length === 0) return 'transparent';
        if (interpolationMode === 'Stepped') {
            const parts: string[] = [];
            parts.push(`${stops[0].color} 0%`);
            for (let i = 0; i < stops.length - 1; i++) {
                const boundary = Math.round(stops[i + 1].position * 100);
                parts.push(`${stops[i].color} ${boundary}%`);
                parts.push(`${stops[i + 1].color} ${boundary}%`);
            }
            parts.push(`${stops[stops.length - 1].color} 100%`);
            return `linear-gradient(to right, ${parts.join(', ')})`;
        } else {
            const parts = stops.map((s) => `${s.color} ${s.position * 100}%`);
            return `linear-gradient(to right, ${parts.join(', ')})`;
        }
    }

    $: gradientBackgroundCss = buildGradientCSS();

    function removeSelectedStop() {
        if (selectedStopIndex >= 0 && selectedStopIndex < gradientStops.length) {
            removeGradientStop(selectedStopIndex);
        }
    }
</script>

<style>
    .color-scheme-selector {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .color-scheme-controls {
        display: flex;
        gap: 0.5rem;
        align-items: center;
    }

    .control-btn {
        padding: 0.5rem;
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 4px;
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.9);
        cursor: pointer;
        font-size: 0.9rem;
        transition: all 0.2s ease;
    }

    .control-btn:hover {
        background: rgba(255, 255, 255, 0.2);
        border-color: rgba(255, 255, 255, 0.4);
        color: rgba(255, 255, 255, 1);
    }

    .control-btn.reverse-btn {
        padding: 0.5rem 1rem;
        font-size: 0.8rem;
    }

    .control-btn.reverse-btn.reversed {
        background: #646cff;
        color: white;
        border-color: #646cff;
    }

    .control-btn.gradient-btn {
        font-size: 1.2rem;
        padding: 0.5rem 0.75rem;
    }

    /* Gradient Editor Dialog Styles */
    /* Dialog container without a fullscreen gray overlay */
    .gradient-editor-dialog {
        position: fixed;
        inset: auto; /* don't stretch over the screen */
        z-index: 2000;
        pointer-events: none; /* allow clicks through empty areas */
    }

    .gradient-editor-content {
        pointer-events: auto; /* re-enable inside dialog */
        background: rgba(0, 0, 0, 0.85);
        color: rgba(255, 255, 255, 0.9);
        padding: 1rem;
        border-radius: 8px;
        min-width: 720px;
        max-width: min(900px, 95vw);
        max-height: 85vh;
        overflow-y: auto;
        border: 1px solid rgba(255, 255, 255, 0.2);
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
        position: fixed;
        top: 10vh;
        left: 50%;
        transform: translateX(-50%);
        backdrop-filter: blur(10px);
        -webkit-backdrop-filter: blur(10px);
    }

    .gradient-editor-content h3 {
        margin: 0 0 1rem 0;
        color: rgba(255, 255, 255, 0.95);
        font-size: 1.25rem;
    }

    /* legacy .text-input styles removed (unused) */

    /* Match compact name input styling from Gradient Editor sim */
    .color-scheme-name-input {
        background: #333;
        border: 1px solid #555;
        color: white;
        padding: 0.25rem 0.4rem;
        border-radius: 3px;
        width: 100%;
        font-size: 0.9rem;
    }

    .color-scheme-name-input:focus {
        border-color: #646cff;
        box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
        outline: none;
    }

    .gradient-preview-container {
        margin: 1.5rem 0;
    }

    .gradient-preview {
        position: relative;
        height: 48px;
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 6px;
        margin-bottom: 12px;
        box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
        cursor: crosshair;
        user-select: none;
        -webkit-user-select: none;
        -moz-user-select: none;
        -ms-user-select: none;
    }

    .gradient-preview:hover {
        border-color: var(--ui-border-focus, rgba(255, 255, 255, 0.5));
    }

    .gradient-stop {
        position: absolute;
        top: 50%;
        transform: translateX(-50%) translateY(-50%);
        width: 24px;
        height: 48px;
        border: 2px solid rgba(255, 255, 255, 0.9);
        border-radius: 6px;
        cursor: grab;
        box-shadow: 0 3px 8px rgba(0, 0, 0, 0.4);
        transition: all 0.2s ease;
        user-select: none;
    }

    .gradient-stop:hover {
        transform: translateX(-50%) translateY(-50%) scale(1.1);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    }

    .gradient-stop.selected {
        border-color: #646cff;
        border-width: 3px;
        box-shadow: 0 4px 16px rgba(100, 108, 255, 0.5);
    }

    .gradient-stop.dragging {
        cursor: grabbing;
        transform: translateX(-50%) translateY(-50%) scale(1.2);
        z-index: 10;
        transition: none;
    }

    .gradient-stop.no-transition {
        transition: none;
    }

    .stop-controls {
        background: rgba(255, 255, 255, 0.05);
        padding: 1rem;
        border-radius: 6px;
        margin: 1rem 0;
        border: 1px solid rgba(255, 255, 255, 0.15);
        min-height: 160px;
        display: flex;
        flex-direction: column;
    }

    .stop-controls h4 {
        margin: 0 0 0.75rem 0;
        color: rgba(255, 255, 255, 0.95);
        font-size: 1rem;
    }

    .control-row {
        display: flex;
        gap: 1rem;
        align-items: end;
        flex: 1;
    }

    .control-row .control-group {
        flex: 1;
    }

    .control-group {
        margin-bottom: 1rem;
    }

    .control-group label {
        display: block;
        margin-bottom: 0.5rem;
        font-weight: 500;
        color: rgba(255, 255, 255, 0.8);
    }

    .color-input {
        width: 100%;
        height: 40px;
        border: 1px solid rgba(255, 255, 255, 0.2);
        background: rgba(255, 255, 255, 0.1);
        color: white;
        border-radius: 4px;
        cursor: pointer;
    }

    .color-input:focus {
        outline: none;
        border-color: #646cff;
        box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
    }

    /* Compact actions styling - keep defined even if minifier misses usage */
    :global(.action-row) {
        margin-top: 1rem;
    }

    :global(.action-buttons) {
        display: flex;
        gap: 0.5rem;
    }

    :global(.action-button) {
        padding: 0.4rem 0.75rem;
        font-size: 0.9rem;
        border-radius: 6px;
        border: 1px solid #555;
        background: #333;
        color: white;
        cursor: pointer;
        transition: background-color 0.2s ease;
    }

    :global(.action-button:hover) {
        background: #444;
    }

    .editor-instructions {
        display: flex;
        flex-direction: column;
    }

    .editor-instructions p {
        margin: 0 0 0.75rem 0;
        color: #333;
        font-weight: 600;
    }

    .editor-instructions ul {
        margin: 0;
        padding-left: 1.5rem;
        flex: 1;
    }

    .editor-instructions li {
        margin: 0.3rem 0;
        color: #333;
        line-height: 1.4;
    }

    .dialog-actions {
        display: flex;
        gap: 0.5rem;
        justify-content: flex-end;
        margin-top: 1rem;
        padding-top: 0.75rem;
        border-top: 1px solid rgba(255, 255, 255, 0.15);
    }

    .primary-button {
        background: #646cff;
        color: white;
        border: 1px solid #646cff;
        padding: 0.75rem 1.5rem;
        border-radius: 6px;
        cursor: pointer;
        font-size: 1rem;
        font-weight: 500;
        transition: all 0.2s ease;
    }

    .primary-button:hover:not(:disabled) {
        background: #535bf2;
        border-color: #535bf2;
        transform: translateY(-1px);
        box-shadow: 0 4px 8px rgba(100, 108, 255, 0.3);
    }

    .primary-button:disabled {
        background: #adb5bd;
        border-color: #adb5bd;
        cursor: not-allowed;
        transform: none;
        box-shadow: none;
    }

    .secondary-button {
        background: rgba(255, 255, 255, 0.15);
        color: white;
        border: 1px solid rgba(255, 255, 255, 0.2);
        padding: 0.6rem 1.2rem;
        border-radius: 6px;
        cursor: pointer;
        font-size: 1rem;
        font-weight: 500;
        transition: all 0.2s ease;
    }

    .secondary-button:hover {
        background: rgba(255, 255, 255, 0.25);
        border-color: rgba(255, 255, 255, 0.35);
        transform: translateY(-1px);
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.3);
    }

    .delete-stop-btn {
        background: #dc3545;
        color: white;
        border: 1px solid #dc3545;
        padding: 0.5rem 1rem;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.9rem;
        font-weight: 500;
        transition: all 0.2s ease;
    }

    .delete-stop-btn:hover {
        background: #c82333;
        border-color: #c82333;
        transform: translateY(-1px);
        box-shadow: 0 4px 8px rgba(220, 53, 69, 0.3);
    }

    /* Removed unused color-space-info styles after compact UI change */
</style>
