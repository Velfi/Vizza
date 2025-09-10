<button
    class="ui-button"
    class:primary={variant === 'primary'}
    class:success={variant === 'success'}
    class:danger={variant === 'danger'}
    class:warning={variant === 'warning'}
    class:small={size === 'small'}
    class:large={size === 'large'}
    {disabled}
    {type}
    on:click
    {...$$restProps}
>
    <slot />
</button>

<script lang="ts">
    export let variant: 'default' | 'primary' | 'success' | 'danger' | 'warning' = 'default';
    export let size: 'small' | 'medium' | 'large' = 'medium';
    export let disabled: boolean = false;
    export let type: 'button' | 'submit' | 'reset' = 'button';
</script>

<style>
    .ui-button {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border: 2px solid var(--ui-border);
        border-radius: var(--radius-sm);
        background: rgba(30, 32, 40, 0.92); /* More opaque, dark neutral */
        color: var(--ui-text);
        cursor: pointer;
        font-family: inherit;
        font-size: 0.95rem;
        font-weight: 600;
        transition:
            var(--transition-fast),
            box-shadow 0.2s;
        user-select: none;
        white-space: nowrap;
        box-sizing: border-box;
        padding: 0.2rem 1rem;
        height: var(--button-height);
        min-width: 90px;
        box-shadow:
            0 4px 16px 0 rgba(0, 0, 0, 0.28),
            0 0 0 2px rgba(100, 108, 255, 0.08);
        letter-spacing: 0.01em;
        outline: none;
    }
    .ui-button.small,
    .ui-button.large {
        padding: 0.2rem 1rem;
    }
    .ui-button:hover:not(:disabled),
    .ui-button:focus-visible {
        background: rgba(40, 44, 60, 0.98);
        border-color: var(--ui-border-focus);
        box-shadow:
            0 6px 24px 0 rgba(100, 108, 255, 0.18),
            0 0 0 3px rgba(100, 108, 255, 0.22);
        z-index: 2;
    }
    .ui-button:focus {
        outline: none;
        border-color: var(--ui-border-focus);
        box-shadow: 0 0 0 3px rgba(100, 108, 255, 0.22);
    }
    .ui-button:disabled {
        opacity: 0.5;
        cursor: not-allowed;
        filter: grayscale(0.2);
    }
    .ui-button.primary {
        background: linear-gradient(90deg, #646cff 90%, #535bf2 100%);
        border-color: #646cff;
        color: #fff;
        font-weight: 700;
        text-shadow: 0 1px 4px rgba(0, 0, 0, 0.18);
    }
    .ui-button.primary:hover:not(:disabled),
    .ui-button.primary:focus-visible {
        background: linear-gradient(90deg, #535bf2 80%, #646cff 100%);
        border-color: #535bf2;
        box-shadow:
            0 6px 24px 0 rgba(100, 108, 255, 0.28),
            0 0 0 3px #535bf2;
    }
    .ui-button.success {
        background: linear-gradient(90deg, #10b981 90%, #059669 100%);
        border-color: #10b981;
        color: #fff;
        font-weight: 700;
        text-shadow: 0 1px 4px rgba(0, 0, 0, 0.18);
    }
    .ui-button.success:hover:not(:disabled),
    .ui-button.success:focus-visible {
        background: linear-gradient(90deg, #059669 80%, #10b981 100%);
        border-color: #059669;
        box-shadow:
            0 6px 24px 0 rgba(16, 185, 129, 0.28),
            0 0 0 3px #059669;
    }
    .ui-button.danger {
        background: linear-gradient(90deg, #ef4444 90%, #dc2626 100%);
        border-color: #ef4444;
        color: #fff;
        font-weight: 700;
        text-shadow: 0 1px 4px rgba(0, 0, 0, 0.18);
    }
    .ui-button.danger:hover:not(:disabled),
    .ui-button.danger:focus-visible {
        background: linear-gradient(90deg, #dc2626 80%, #ef4444 100%);
        border-color: #dc2626;
        box-shadow:
            0 6px 24px 0 rgba(239, 68, 68, 0.28),
            0 0 0 3px #dc2626;
    }
    .ui-button.warning {
        background: linear-gradient(90deg, #f59e0b 90%, #d97706 100%);
        border-color: #f59e0b;
        color: #1a1a1a;
        font-weight: 700;
        text-shadow: 0 1px 4px rgba(0, 0, 0, 0.1);
    }
    .ui-button.warning:hover:not(:disabled),
    .ui-button.warning:focus-visible {
        background: linear-gradient(90deg, #d97706 80%, #f59e0b 100%);
        border-color: #d97706;
        box-shadow:
            0 6px 24px 0 rgba(245, 158, 11, 0.28),
            0 0 0 3px #d97706;
    }
</style>
