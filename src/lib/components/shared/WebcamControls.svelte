<div class="webcam-controls">
  <div class="setting-item">
    <span class="setting-label">Webcam:</span>
    <div style="display: flex; gap: 8px; align-items: center;">
      {#if !webcamActive}
        <Button
          variant="default"
          on:click={startWebcamCapture}
          disabled={webcamDevices.length === 0}
        >Start</Button>
      {:else}
        <Button
          variant="default"
          on:click={stopWebcamCapture}
        >Stop</Button>
      {/if}
      {#if webcamDevices.length > 0}
        <span style="font-size: 0.8em; color: #666;">
          {webcamDevices.length} camera{webcamDevices.length === 1 ? '' : 's'} available
        </span>
      {/if}
    </div>
  </div>
</div>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Button from './Button.svelte';

  const dispatch = createEventDispatcher();

  // Props
  export let webcamDevices: number[] = [];
  export let webcamActive: boolean = false;

  // Event handlers
  export let onStartWebcam: (() => void) | undefined = undefined;
  export let onStopWebcam: (() => void) | undefined = undefined;

  function startWebcamCapture() {
    onStartWebcam?.();
    dispatch('start');
  }

  function stopWebcamCapture() {
    onStopWebcam?.();
    dispatch('stop');
  }
</script>

<style>
  .webcam-controls {
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
