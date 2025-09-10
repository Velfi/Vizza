<CollapsibleFieldset title="Post Processing" bind:open={show_post_processing_section}>
    <div class="post-processing-section">
        <h3 class="section-header">Blur Filter</h3>
        <div class="settings-grid">
            <div class="setting-item">
                <span class="setting-label">Enabled:</span>
                <Button
                    variant={postProcessingState.blur_filter.enabled ? 'primary' : 'default'}
                    size="small"
                    on:click={() => updateBlurFilter(!postProcessingState.blur_filter.enabled)}
                >
                    {postProcessingState.blur_filter.enabled ? 'Enabled' : 'Disabled'}
                </Button>
            </div>

            <div class="setting-item">
                <span class="setting-label">Radius:</span>
                <NumberDragBox
                    value={postProcessingState.blur_filter.radius}
                    on:change={({ detail }) => updateBlurFilter(undefined, detail)}
                    min={0.0}
                    max={50.0}
                    step={0.5}
                    precision={1}
                />
            </div>

            <div class="setting-item">
                <span class="setting-label">Sigma:</span>
                <NumberDragBox
                    value={postProcessingState.blur_filter.sigma}
                    on:change={({ detail }) => updateBlurFilter(undefined, undefined, detail)}
                    min={0.1}
                    max={10.0}
                    step={0.1}
                    precision={1}
                />
            </div>
        </div>

        <div class="setting-description">
            <small>
                <strong>Radius:</strong> Controls the size of the blur effect (higher = more blur).<br
                />
                <strong>Sigma:</strong> Controls the intensity of the blur effect (higher = stronger
                blur).
            </small>
        </div>
    </div>
</CollapsibleFieldset>

<script lang="ts">
    import { createEventDispatcher, onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import Button from './Button.svelte';
    import NumberDragBox from '../inputs/NumberDragBox.svelte';
    import CollapsibleFieldset from './CollapsibleFieldset.svelte';

    const dispatch = createEventDispatcher();

    export let simulationType: string = 'flow';
    export let enabled: boolean = true; // Whether the component should load state

    type PostProcessingState = {
        blur_filter: {
            enabled: boolean;
            radius: number;
            sigma: number;
        };
    };

    let postProcessingState: PostProcessingState = {
        blur_filter: {
            enabled: false,
            radius: 5.0,
            sigma: 2.0,
        },
    };

    let show_post_processing_section = false;

    function getCommandName(command: string): string {
        switch (simulationType) {
            case 'particle_life':
                return command === 'get'
                    ? 'get_particle_life_post_processing_state'
                    : 'update_particle_life_post_processing_state';
            case 'gray_scott':
                return command === 'get'
                    ? 'get_gray_scott_post_processing_state'
                    : 'update_gray_scott_post_processing_state';
            case 'slime_mold':
                return command === 'get'
                    ? 'get_slime_mold_post_processing_state'
                    : 'update_slime_mold_post_processing_state';
            case 'pellets':
                return command === 'get'
                    ? 'get_pellets_post_processing_state'
                    : 'update_pellets_post_processing_state';
            case 'voronoi_ca':
                return command === 'get'
                    ? 'get_voronoi_ca_post_processing_state'
                    : 'update_voronoi_ca_post_processing_state';
            case 'flow':
            default:
                return command === 'get'
                    ? 'get_post_processing_state'
                    : 'update_post_processing_state';
        }
    }

    async function loadPostProcessingState() {
        try {
            const command = getCommandName('get');
            const state = await invoke(command);
            postProcessingState = state as PostProcessingState;
        } catch (error) {
            // Only log error if it's not a "simulation not running" error
            if (!String(error).includes('only available for')) {
                console.error('Failed to load post processing state:', error);
            }
        }
    }

    async function updateBlurFilter(enabled?: boolean, radius?: number, sigma?: number) {
        try {
            const params: { radius?: number; sigma?: number } = {};
            if (radius !== undefined) params.radius = radius;
            if (sigma !== undefined) params.sigma = sigma;

            const command = getCommandName('update');
            await invoke(command, {
                effectName: 'blur_filter',
                enabled: enabled ?? postProcessingState.blur_filter.enabled,
                params,
            });

            // Update local state
            if (enabled !== undefined) postProcessingState.blur_filter.enabled = enabled;
            if (radius !== undefined) postProcessingState.blur_filter.radius = radius;
            if (sigma !== undefined) postProcessingState.blur_filter.sigma = sigma;

            dispatch('stateChanged', { state: postProcessingState });
        } catch (error) {
            console.error('Failed to update blur filter:', error);
        }
    }

    onMount(() => {
        if (enabled) {
            loadPostProcessingState();
        }
    });
</script>

<style>
    .post-processing-section {
        margin-bottom: 1rem;
    }

    .section-header {
        font-size: 1rem;
        font-weight: 600;
        margin: 0 0 0.5rem 0;
        color: var(--text-color);
    }

    .settings-grid {
        display: grid;
        grid-template-columns: 1fr;
        gap: 0.5rem;
        margin-bottom: 0.5rem;
    }

    .setting-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .setting-label {
        font-weight: 500;
        min-width: 80px;
        color: var(--text-color);
    }

    .setting-description {
        margin-top: 0.5rem;
        padding: 0.5rem;
        background: var(--bg-secondary);
        border-radius: 4px;
        border-left: 3px solid var(--accent-color);
    }

    .setting-description small {
        color: var(--text-muted);
        line-height: 1.4;
    }
</style>
