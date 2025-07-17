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
      <Button variant="success" on:click={handleSave} disabled={presetName.trim() === ''}>
        Save
      </Button>
      <Button variant="default" on:click={() => dispatch('close')}>Cancel</Button>
    </div>
  </div>
</div>

<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import Button from './Button.svelte';

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

<style>
  .dialog-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: rgba(0, 0, 0, 0.9);
    padding: 2rem;
    border-radius: 8px;
    min-width: 300px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
  }

  .dialog h3 {
    margin-top: 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1.5rem;
  }

  .dialog input {
    width: 100%;
    margin: 1rem 0;
    padding: 0.75rem;
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    font-family: inherit;
    font-size: 1rem;
    box-sizing: border-box;
  }

  .dialog input:focus {
    outline: none;
    border-color: #646cff;
    background: rgba(255, 255, 255, 0.15);
  }

  .dialog input::placeholder {
    color: rgba(255, 255, 255, 0.5);
  }

  .dialog .input-group {
    margin: 1rem 0;
  }

  .dialog .input-group label {
    display: block;
    margin-bottom: 0.5rem;
    color: rgba(255, 255, 255, 0.9);
    font-weight: 500;
    font-size: 0.9rem;
  }

  .dialog-buttons {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.5rem;
  }
</style>
