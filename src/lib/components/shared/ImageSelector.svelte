<div class="image-selector">
  {#if showFitMode}
    <div class="setting-item">
      <span class="setting-label">Fit Mode:</span>
      <Selector
        options={['Stretch', 'Center', 'Fit H', 'Fit V']}
        value={fitMode}
        on:change={(e) => onFitModeChange?.(e.detail.value)}
      />
    </div>
  {/if}

  {#if showMirrorHorizontal}
    <div class="setting-item">
      <span class="setting-label">Mirror Horizontal:</span>
      <input
        type="checkbox"
        checked={mirrorHorizontal}
        on:change={(e) => onMirrorHorizontalChange?.((e.target as HTMLInputElement).checked)}
      />
    </div>
  {/if}

  {#if showInvertTone}
    <div class="setting-item">
      <span class="setting-label">Invert Tone:</span>
      <input
        type="checkbox"
        checked={invertTone}
        on:change={(e) => onInvertToneChange?.((e.target as HTMLInputElement).checked)}
      />
    </div>
  {/if}

  {#if showLoadButton}
    <div class="setting-item">
      <span class="setting-label">Load Image:</span>
      <Button
        variant="default"
        on:click={handleLoadImage}
      >
        Choose File…
      </Button>
    </div>
  {/if}

</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import Selector from '../inputs/Selector.svelte';
  import Button from './Button.svelte';

  const dispatch = createEventDispatcher();

  // Props
  export let fitMode: string = 'Stretch';
  export let mirrorHorizontal: boolean = false;
  export let invertTone: boolean = false;
  export let loadCommand: string = '';
  export let showFitMode: boolean = true;
  export let showMirrorHorizontal: boolean = true;
  export let showInvertTone: boolean = true;
  export let showLoadButton: boolean = true;

  // Event handlers
  export let onFitModeChange: ((value: string) => void) | undefined = undefined;
  export let onMirrorHorizontalChange: ((value: boolean) => void) | undefined = undefined;
  export let onInvertToneChange: ((value: boolean) => void) | undefined = undefined;

  async function handleLoadImage() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Images',
            extensions: ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp', 'tiff']
          }
        ]
      });
      
      if (selected && loadCommand) {
        await invoke(loadCommand, { imagePath: selected });
        dispatch('imageLoaded', { imagePath: selected });
      }
    } catch (err) {
      console.error('Failed to load image:', err);
      dispatch('error', { error: err });
    }
  }
</script>

<style>
  .image-selector {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .setting-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .setting-label {
    font-weight: 500;
    min-width: 120px;
    color: var(--text-color);
  }
</style>
