<div
  class="dialog-backdrop"
  role="dialog"
  aria-modal="true"
  aria-labelledby="save-preset-title"
  tabindex="-1"
  on:click={() => dispatch('close')}
  on:keydown={(e) => e.key === 'Escape' && dispatch('close')}
>
  <div class="dialog" role="document" on:click|stopPropagation>
    <h3 id="save-preset-title">Save Preset</h3>
    <div class="input-group">
      <label for="preset-name-input">Preset Name</label>
      <input
        id="preset-name-input"
        type="text"
        placeholder="Enter preset name..."
        value={presetName}
        on:input={(e) => (presetName = (e.target as HTMLInputElement).value)}
        on:keydown={(e) => e.key === 'Enter' && handleSave()}
        bind:this={inputElement}
      />
    </div>
    <div class="dialog-buttons">
      <button on:click={handleSave} disabled={presetName.trim() === ''}> Save </button>
      <button on:click={() => dispatch('close')}> Cancel </button>
    </div>
  </div>
</div>

<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';

  export let presetName: string = '';

  const dispatch = createEventDispatcher<{
    save: { name: string };
    close: void;
  }>();

  let inputElement: HTMLInputElement;

  function handleSave() {
    if (presetName.trim() !== '') {
      dispatch('save', { name: presetName.trim() });
    }
  }

  onMount(() => {
    // Auto-focus the input when the component mounts
    if (inputElement) {
      inputElement.focus();
    }
  });
</script> 