<fieldset>
  <legend>
    <button type="button" class="fieldset-toggle" on:click={toggle} aria-expanded={open}>
      {open ? '▼' : '▶'}
      {title}
    </button>
  </legend>

  {#if open}
    <div class="fieldset-content">
      <slot />
    </div>
  {/if}
</fieldset>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let title: string;
  export let open: boolean = false;

  const dispatch = createEventDispatcher();

  function toggle() {
    open = !open;
    dispatch('toggle', { open });
  }
</script>

<style>
  .fieldset-toggle {
    background: none;
    border: none;
    color: inherit;
    font-family: inherit;
    font-size: inherit;
    font-weight: inherit;
    cursor: pointer;
    padding: 0;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .fieldset-toggle:hover {
    color: #51cf66;
  }

  .fieldset-content {
    padding: 1rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
    margin-top: 0.5rem;
  }

  .fieldset-content :global(p) {
    margin: 0 0 1rem 0;
    color: rgba(255, 255, 255, 0.9);
    line-height: 1.5;
  }

  .fieldset-content :global(p:last-child) {
    margin-bottom: 0;
  }
</style>
