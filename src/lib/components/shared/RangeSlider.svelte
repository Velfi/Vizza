<div class="range-slider-container">
    {#if label}
        <div class="range-slider-label">{label}</div>
    {/if}

    <div class="range-slider-wrapper">
        <div class="range-slider-labels">
            <span class="range-slider-low-label">{lowLabel}</span>
            <span class="range-slider-high-label">{highLabel}</span>
        </div>

        <div bind:this={sliderElement} class="range-slider" class:disabled></div>
    </div>
</div>

<script lang="ts">
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import noUiSlider from 'nouislider';
    import 'nouislider/dist/nouislider.css';

    const dispatch = createEventDispatcher();

    export let label: string = '';
    export let min: number = 0;
    export let max: number = 100;
    export let step: number = 1;
    export let lowValue: number = min;
    export let highValue: number = max;
    export let lowLabel: string = 'Low';
    export let highLabel: string = 'High';
    export let allowOverlap: boolean = true;
    export let disabled: boolean = false;

    let sliderElement: HTMLDivElement;
    let slider: any = null;

    // Update slider values when props change
    $: if (slider) {
        slider.set([lowValue, highValue]);
    }

    // Update disabled state
    $: if (slider) {
        if (disabled) {
            slider.disable();
        } else {
            slider.enable();
        }
    }

    onMount(() => {
        if (sliderElement) {
            slider = noUiSlider.create(sliderElement, {
                start: [lowValue, highValue],
                connect: true,
                range: {
                    min: min,
                    max: max,
                },
                step: step,
                margin: allowOverlap ? 0 : step,
                limit: allowOverlap ? undefined : max - min,
                behaviour: 'tap-drag',
                tooltips: [
                    {
                        to: (value) => {
                            return Math.round(value);
                        },
                    },
                    {
                        to: (value) => {
                            return Math.round(value);
                        },
                    },
                ],
                format: {
                    to: (value) => {
                        return Math.round(value);
                    },
                    from: (value) => {
                        return Number(value);
                    },
                },
            });

            // Listen for slider updates
            slider.on('update', (values: string[]) => {
                const newLowValue = Number(values[0]);
                const newHighValue = Number(values[1]);

                // Only dispatch if values actually changed
                if (newLowValue !== lowValue || newHighValue !== highValue) {
                    dispatch('change', {
                        lowValue: newLowValue,
                        highValue: newHighValue,
                    });
                }
            });
        }
    });

    onDestroy(() => {
        if (slider) {
            slider.destroy();
        }
    });
</script>

<style>
    .range-slider-container {
        display: flex;
        flex-direction: column;
        gap: 8px;
        width: 100%;
    }

    .range-slider-label {
        font-size: 12px;
        color: rgba(255, 255, 255, 0.8);
        font-weight: 500;
        margin: 0;
    }

    .range-slider-wrapper {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .range-slider-labels {
        display: flex;
        justify-content: space-between;
        align-items: center;
        font-size: 11px;
        color: rgba(255, 255, 255, 0.6);
        font-family: monospace;
    }

    .range-slider {
        height: 20px;
        margin: 0;
    }

    .range-slider.disabled {
        opacity: 0.5;
        pointer-events: none;
    }

    /* noUiSlider custom styling to match the app theme */
    :global(.range-slider .noUi-target) {
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.3);
        border-radius: 4px;
        box-shadow: none;
    }

    :global(.range-slider .noUi-base) {
        background: transparent;
    }

    :global(.range-slider .noUi-connects) {
        background: transparent;
    }

    :global(.range-slider .noUi-connect) {
        background: linear-gradient(90deg, #646cff, #8a4acc);
    }

    :global(.range-slider .noUi-handle) {
        background: #646cff;
        border: 2px solid #ffffff;
        border-radius: 50%;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
        cursor: pointer;
        transition: all 0.2s ease;
    }

    :global(.range-slider .noUi-handle:hover) {
        background: #8a4acc;
        transform: scale(1.1);
    }

    :global(.range-slider .noUi-handle:focus) {
        outline: none;
        box-shadow: 0 0 0 3px rgba(100, 108, 255, 0.3);
    }

    :global(.range-slider .noUi-handle:after) {
        display: none;
    }

    :global(.range-slider .noUi-handle:before) {
        display: none;
    }

    :global(.range-slider .noUi-tooltip) {
        background: rgba(0, 0, 0, 0.8);
        color: white;
        border: none;
        border-radius: 4px;
        padding: 4px 8px;
        font-size: 11px;
        font-family: monospace;
        font-weight: 500;
    }

    :global(.range-slider .noUi-tooltip:after) {
        border-top-color: rgba(0, 0, 0, 0.8);
    }

    /* Disabled state styling */
    :global(.range-slider.disabled .noUi-handle) {
        background: #666;
        border-color: #999;
        cursor: not-allowed;
    }

    :global(.range-slider.disabled .noUi-connect) {
        background: #666;
    }

    /* Responsive design */
    @media (max-width: 768px) {
        .range-slider-labels {
            font-size: 10px;
        }

        .range-slider {
            height: 18px;
        }
    }
</style>
