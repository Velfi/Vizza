<fieldset>
    <legend>Controls</legend>
    <div class="controls-grid">
        <div class="interaction-section">
            <div class="interaction-help">
                <div class="control-group">
                    <span>{mouseInteractionText}</span>
                </div>
                <div class="control-group">
                    <Button variant="default" on:click={() => dispatch('navigate', 'how-to-play')}>
                        📖 Camera Controls
                    </Button>
                </div>
                <div class="control-group">
                    <span
                        >Camera controls not working? Click the control bar at the top of the
                        screen.</span
                    >
                </div>
            </div>
        </div>

        <div class="cursor-section">
            <div class="cursor-settings">
                <div class="cursor-settings-header">
                    <span>{cursorSettingsTitle}</span>
                </div>
                <CursorConfig
                    {cursorSize}
                    {cursorStrength}
                    {sizeMin}
                    {sizeMax}
                    {sizeStep}
                    {strengthMin}
                    {strengthMax}
                    {strengthStep}
                    {sizePrecision}
                    {strengthPrecision}
                    on:sizechange={handleCursorSizeChange}
                    on:strengthchange={handleCursorStrengthChange}
                />
            </div>
        </div>
    </div>
</fieldset>

<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import Button from './Button.svelte';
    import CursorConfig from './CursorConfig.svelte';

    const dispatch = createEventDispatcher();

    // Props
    export let mouseInteractionText: string = '🖱️ Left click: Interact | Right click: Interact';
    export let cursorSettingsTitle: string = '🎯 Cursor Settings';
    export let cursorSize: number = 0.2;
    export let cursorStrength: number | undefined = 1.0;
    export let sizeMin: number = 0.01;
    export let sizeMax: number = 1.0;
    export let sizeStep: number = 0.01;
    export let sizePrecision: number = 2.0;
    export let strengthMin: number = 0.01;
    export let strengthMax: number = 1.0;
    export let strengthStep: number = 0.01;
    export let strengthPrecision: number = 2.0;

    // Event handlers
    function handleCursorSizeChange(e: CustomEvent) {
        dispatch('cursorSizeChange', e.detail);
    }

    function handleCursorStrengthChange(e: CustomEvent) {
        dispatch('cursorStrengthChange', e.detail);
    }
</script>

<style>
    .controls-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
        align-items: start;
    }

    .interaction-section {
        display: flex;
        flex-direction: column;
    }

    .interaction-help {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .cursor-section {
        display: flex;
        flex-direction: column;
    }

    .cursor-settings {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .cursor-settings-header {
        font-size: 0.9rem;
        font-weight: 500;
        color: rgba(255, 255, 255, 0.8);
        padding: 0.25rem 0;
    }

    .control-group {
        margin-bottom: 0.5rem;
    }

    /* Mobile responsive design */
    @media (max-width: 768px) {
        .controls-grid {
            grid-template-columns: 1fr;
            gap: 0.75rem;
        }

        .interaction-help {
            gap: 0.4rem;
        }

        .cursor-settings {
            gap: 0.4rem;
        }

        .cursor-settings-header {
            font-size: 0.85rem;
        }
    }
</style>
