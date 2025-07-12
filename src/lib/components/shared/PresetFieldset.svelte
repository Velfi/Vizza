<fieldset>
  <legend>Presets</legend>
  <div class="control-group">
    <Selector
      options={availablePresets}
      bind:value={currentPreset}
      {placeholder}
      on:change={handlePresetChange}
    />
  </div>
  <div class="preset-actions">
    <button type="button" on:click={handleSaveClick}> Save Current Settings </button>
  </div>
</fieldset>

{#if showSaveDialog}
  <SavePresetDialog bind:presetName={newPresetName} on:save={handleSave} on:close={handleClose} />
{/if}

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Selector from '../inputs/Selector.svelte';
  import SavePresetDialog from './SavePresetDialog.svelte';

  export let availablePresets: string[] = [];
  export let currentPreset: string = '';
  export let placeholder: string = 'Select preset...';

  const dispatch = createEventDispatcher<{
    presetChange: { value: string };
    presetSave: { name: string };
  }>();

  let showSaveDialog = false;
  let newPresetName = '';

  function handlePresetChange(event: CustomEvent<{ value: string }>) {
    dispatch('presetChange', { value: event.detail.value });
  }

  function handleSaveClick() {
    showSaveDialog = true;
    newPresetName = '';
  }

  function handleSave(event: CustomEvent<{ name: string }>) {
    dispatch('presetSave', { name: event.detail.name });
    showSaveDialog = false;
    newPresetName = '';
  }

  function handleClose() {
    showSaveDialog = false;
    newPresetName = '';
  }
</script>

<style>
  .control-group {
    margin-bottom: 1rem;
  }

  .preset-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .control-group:last-child {
    margin-bottom: 0;
  }
</style>
