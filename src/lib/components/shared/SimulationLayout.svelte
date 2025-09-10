<div
    class="simulation-container"
    on:mousedown={handleMouseEvent}
    on:mousemove={handleMouseEvent}
    on:mouseup={handleMouseEvent}
    on:wheel={handleMouseEvent}
    on:contextmenu={handleContextMenu}
    role="button"
    tabindex="0"
>
    <SimulationControlBar
        {simulationName}
        {running}
        {loading}
        {showUI}
        {currentFps}
        {controlsVisible}
        {showCenterControls}
        {showRightControls}
        {controlModeButton}
        {showStep}
        on:back={handleBack}
        on:toggleUI={handleToggleUI}
        on:pause={handlePause}
        on:resume={handleResume}
        on:step={handleStep}
        on:reset={handleReset}
        on:randomize={handleRandomize}
        on:userInteraction={handleUserInteraction}
    />

    <SimulationMenuContainer position={menuPosition} {showUI}>
        <slot />
    </SimulationMenuContainer>

    <!-- Loading Screen -->
    {#if loading}
        <div class="loading-overlay">
            <div class="loading-content">
                <div class="loading-spinner"></div>
                <h2>Starting Simulation...</h2>
                <p>Initializing GPU resources</p>
            </div>
        </div>
    {/if}
</div>

<script lang="ts">
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import SimulationControlBar from './SimulationControlBar.svelte';
    import SimulationMenuContainer from './SimulationMenuContainer.svelte';

    const dispatch = createEventDispatcher();

    export let simulationName: string;
    export let running: boolean = false;
    export let loading: boolean = false;
    export let showUI: boolean = true;
    export let currentFps: number = 0;
    export let controlsVisible: boolean = true;
    export let menuPosition: string = 'middle';
    export let showCenterControls: boolean = true;
    export let showRightControls: boolean = true;
    export let controlModeButton: import('svelte').Snippet | undefined = undefined;
    export let showStep: boolean = false;

    let isMousePressed = false;
    let currentMouseButton = 0;

    // Event handlers
    function handleBack() {
        dispatch('back');
    }

    function handleToggleUI() {
        dispatch('toggleUI');
    }

    function handlePause() {
        dispatch('pause');
    }

    function handleResume() {
        dispatch('resume');
    }

    function handleStep() {
        dispatch('step');
    }

    function handleReset() {
        dispatch('reset');
    }

    function handleRandomize() {
        dispatch('randomize');
    }

    function handleUserInteraction() {
        dispatch('userInteraction');
    }

    // Helper function to check if the event occurred directly on the container
    function isDirectTarget(event: Event): boolean {
        return event.target === event.currentTarget;
    }

    // Global mouse event handler to detect mouse release outside simulation area
    function handleGlobalMouseUp(event: MouseEvent) {
        if (isMousePressed) {
            isMousePressed = false;
            // Create a synthetic mouseup event to dispatch
            const syntheticEvent = new MouseEvent('mouseup', {
                button: currentMouseButton,
                clientX: event.clientX,
                clientY: event.clientY,
                bubbles: true,
                cancelable: true,
            });
            dispatch('mouseEvent', syntheticEvent);
        }
    }

    // Handle mouse enter on menu container to trigger release when dragging
    function handleMenuMouseEnter(event: Event) {
        if (isMousePressed) {
            isMousePressed = false;
            // Create a synthetic mouseup event to dispatch
            const mouseEvent = event as MouseEvent;
            const syntheticEvent = new MouseEvent('mouseup', {
                button: currentMouseButton,
                clientX: mouseEvent.clientX,
                clientY: mouseEvent.clientY,
                bubbles: true,
                cancelable: true,
            });
            dispatch('mouseEvent', syntheticEvent);
        }
    }

    // Handle mouse events and forward to parent, but only if not on UI elements
    function handleMouseEvent(event: MouseEvent | WheelEvent) {
        // Allow continuous drag interactions even when cursor passes over UI children
        if (event.type === 'mousemove' && isMousePressed) {
            event.preventDefault();

            dispatch('mouseEvent', event);
            return;
        }

        if (isDirectTarget(event)) {
            event.preventDefault();

            // Track mouse press state for global mouse up detection
            if (event.type === 'mousedown') {
                isMousePressed = true;
                currentMouseButton = (event as MouseEvent).button;
            } else if (event.type === 'mouseup') {
                isMousePressed = false;
            }

            dispatch('mouseEvent', event);
        }
    }

    // Handle context menu - only prevent default when clicking on simulation area
    function handleContextMenu(event: MouseEvent) {
        if (isDirectTarget(event)) {
            event.preventDefault();
            // Treat contextmenu as a right-button press so we reliably get a release later
            isMousePressed = true;
            currentMouseButton = 2;
            dispatch('mouseEvent', event);
        }
    }

    onMount(() => {
        // Add global mouse up listener
        document.addEventListener('mouseup', handleGlobalMouseUp);

        // Add mouse enter listener to menu container using CSS selector
        // Use a small delay to ensure the menu container is rendered
        setTimeout(() => {
            const menuContainer = document.querySelector('.simulation-menu-container');
            if (menuContainer) {
                menuContainer.addEventListener('mouseenter', handleMenuMouseEnter);
            }
        }, 100);
    });

    onDestroy(() => {
        // Remove global mouse up listener
        document.removeEventListener('mouseup', handleGlobalMouseUp);

        // Remove menu mouse enter listener
        const menuContainer = document.querySelector('.simulation-menu-container');
        if (menuContainer) {
            menuContainer.removeEventListener('mouseenter', handleMenuMouseEnter);
        }
    });
</script>

<style>
    .simulation-container {
        display: flex;
        flex-direction: column;
        height: 100vh;
        background: transparent;
        position: relative;
    }

    .loading-overlay {
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.8);
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .loading-content {
        text-align: center;
        color: white;
    }

    .loading-spinner {
        width: 40px;
        height: 40px;
        border: 4px solid rgba(255, 255, 255, 0.3);
        border-top: 4px solid white;
        border-radius: 50%;
        animation: spin 1s linear infinite;
        margin: 0 auto 1rem;
    }

    @keyframes spin {
        0% {
            transform: rotate(0deg);
        }
        100% {
            transform: rotate(360deg);
        }
    }

    .loading-content h2 {
        margin: 0 0 0.5rem 0;
        font-size: 1.5rem;
    }

    .loading-content p {
        margin: 0;
        opacity: 0.8;
    }
</style>
