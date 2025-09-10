<div class="button-select">
    <button type="button" class="action-button" on:click={handleButtonClick}>
        {buttonText}
    </button>
    <select class="select-dropdown" {value} on:change={handleSelectChange}>
        <option value="" disabled>{placeholder}</option>
        {#each options as option}
            <option value={option.value}>{option.label}</option>
        {/each}
    </select>
</div>

<script lang="ts">
    import { createEventDispatcher } from 'svelte';

    export let value: string = '';
    export let options: { value: string; label: string; buttonAction?: string }[] = [];
    export let buttonText: string = 'Quick Action';
    export let placeholder: string = 'Select an option...';

    const dispatch = createEventDispatcher();

    function handleButtonClick() {
        // Find the current option and trigger its button action if it exists
        const currentOption = options.find((opt) => opt.value === value);
        if (currentOption?.buttonAction) {
            dispatch('buttonclick', {
                action: currentOption.buttonAction,
                value: currentOption.value,
            });
        } else {
            // Default action - just dispatch the current value
            dispatch('buttonclick', { action: 'default', value: currentOption?.value || value });
        }
    }

    function handleSelectChange(event: Event) {
        const newValue = (event.target as HTMLSelectElement).value;
        dispatch('change', { value: newValue });
    }
</script>

<style>
    .button-select {
        display: flex;
        align-items: center;
        position: relative;
    }

    .action-button {
        padding: 0.5rem 1rem;
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 4px 0 0 4px;
        color: rgba(255, 255, 255, 0.9);
        cursor: pointer;
        font-family: inherit;
        font-size: 0.875rem;
        transition: all 0.3s ease;
        white-space: nowrap;
        border-right: none;
    }

    .action-button:hover {
        background: rgba(255, 255, 255, 0.2);
        border-color: rgba(255, 255, 255, 0.4);
    }

    .select-dropdown {
        flex: 1;
        padding: 0.5rem;
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 0 4px 4px 0;
        color: rgba(255, 255, 255, 0.9);
        font-family: inherit;
        font-size: 0.875rem;
        cursor: pointer;
        border-left: none;
        appearance: none;
        -webkit-appearance: none;
        -moz-appearance: none;
        background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='rgba(255,255,255,0.6)' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6,9 12,15 18,9'%3e%3c/polyline%3e%3c/svg%3e");
        background-repeat: no-repeat;
        background-position: right 0.5rem center;
        background-size: 1rem;
        padding-right: 2rem;
    }

    .select-dropdown:focus {
        outline: none;
        border-color: rgba(255, 255, 255, 0.5);
        background-color: rgba(255, 255, 255, 0.15);
    }

    .select-dropdown:hover {
        background-color: rgba(255, 255, 255, 0.15);
    }

    .select-dropdown option {
        background: #2a2a2a;
        color: rgba(255, 255, 255, 0.9);
    }

    /* Ensure the button and select have the same height */
    .action-button,
    .select-dropdown {
        height: 2.5rem;
        box-sizing: border-box;
    }
</style>
