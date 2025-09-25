/**
 * Shared auto-hide utility for simulation modes
 * Provides consistent auto-hide behavior for controls and cursor
 */

export interface AutoHideConfig {
    autoHideDelay?: number;
    cursorHideDelay?: number;
}

export interface AutoHideState {
    controlsVisible: boolean;
    cursorHidden: boolean;
    showUI: boolean;
    running: boolean;
}

export interface AutoHideCallbacks {
    onControlsShow: () => void;
    onControlsHide: () => void;
    onCursorShow: () => void;
    onCursorHide: () => void;
}

export class AutoHideManager {
    private hideTimeout: number | null = null;
    private cursorHideTimeout: number | null = null;
    private config: Required<AutoHideConfig>;
    private state: AutoHideState;
    private callbacks: AutoHideCallbacks;

    constructor(
        initialState: AutoHideState,
        callbacks: AutoHideCallbacks,
        config: AutoHideConfig = {}
    ) {
        this.state = initialState;
        this.callbacks = callbacks;
        this.config = {
            autoHideDelay: config.autoHideDelay ?? 3000,
            cursorHideDelay: config.cursorHideDelay ?? 2000,
        };
    }

    /**
     * Update the current state
     */
    updateState(newState: Partial<AutoHideState>) {
        this.state = { ...this.state, ...newState };
    }

    /**
     * Start the auto-hide timer for controls
     */
    startAutoHideTimer() {
        this.stopAutoHideTimer();
        this.hideTimeout = window.setTimeout(() => {
            // Only hide controls if simulation is running and UI is hidden
            if (this.state.running && !this.state.showUI) {
                this.hideControls();
            }
        }, this.config.autoHideDelay);
    }

    /**
     * Stop the auto-hide timer for controls
     */
    stopAutoHideTimer() {
        if (this.hideTimeout) {
            clearTimeout(this.hideTimeout);
            this.hideTimeout = null;
        }
    }

    /**
     * Show controls and restart auto-hide timer
     */
    showControls() {
        this.state.controlsVisible = true;
        this.callbacks.onControlsShow();
        this.startAutoHideTimer();
    }

    /**
     * Hide controls
     */
    hideControls() {
        this.state.controlsVisible = false;
        this.callbacks.onControlsHide();
        // Also hide cursor when controls are hidden
        this.hideCursor();
    }

    /**
     * Start the cursor hide timer
     */
    startCursorHideTimer() {
        this.stopCursorHideTimer();
        this.cursorHideTimeout = window.setTimeout(() => {
            if (!this.state.showUI && !this.state.controlsVisible) {
                this.hideCursor();
            }
        }, this.config.cursorHideDelay);
    }

    /**
     * Stop the cursor hide timer
     */
    stopCursorHideTimer() {
        if (this.cursorHideTimeout) {
            clearTimeout(this.cursorHideTimeout);
            this.cursorHideTimeout = null;
        }
    }

    /**
     * Show cursor
     */
    showCursor() {
        if (this.state.cursorHidden) {
            this.state.cursorHidden = false;
            this.callbacks.onCursorShow();
        }
    }

    /**
     * Hide cursor
     */
    hideCursor() {
        if (!this.state.cursorHidden) {
            this.state.cursorHidden = true;
            this.callbacks.onCursorHide();
        }
    }

    /**
     * Handle user interaction - show controls and restart timers
     */
    handleUserInteraction() {
        if (!this.state.showUI && !this.state.controlsVisible) {
            this.showControls();
            this.showCursor();
        } else if (!this.state.showUI && this.state.controlsVisible) {
            this.showCursor();
            this.startAutoHideTimer();
            this.startCursorHideTimer();
        }
    }

    /**
     * Handle UI toggle - start/stop auto-hide based on UI visibility
     */
    handleUIToggle(showUI: boolean) {
        this.state.showUI = showUI;

        if (!showUI) {
            this.showControls();
            this.showCursor();
            this.startAutoHideTimer();
            this.startCursorHideTimer();
        } else {
            this.stopAutoHideTimer();
            this.stopCursorHideTimer();
            this.showCursor();
            this.state.controlsVisible = true;
        }
    }

    /**
     * Handle simulation pause - stop auto-hide and show controls
     */
    handlePause() {
        if (!this.state.showUI) {
            this.showControls();
            this.stopAutoHideTimer(); // Stop auto-hide when paused
        }
    }

    /**
     * Handle simulation resume - restart auto-hide if UI is hidden
     */
    handleResume() {
        if (!this.state.showUI) {
            this.startAutoHideTimer();
        }
    }

    /**
     * Clean up all timers and restore cursor
     */
    cleanup() {
        this.stopAutoHideTimer();
        this.stopCursorHideTimer();
        this.showCursor();
    }

    /**
     * Get current state
     */
    getState(): AutoHideState {
        return { ...this.state };
    }
}

/**
 * Create event listeners for auto-hide functionality
 */
export function createAutoHideEventListeners(handleUserInteraction: () => void): {
    add: () => void;
    remove: () => void;
} {
    const events = ['mousedown', 'mousemove', 'wheel', 'touchstart'];

    return {
        add: () => {
            events.forEach((event) => {
                window.addEventListener(event, handleUserInteraction, { passive: true });
            });
        },
        remove: () => {
            events.forEach((event) => {
                window.removeEventListener(event, handleUserInteraction);
            });
        },
    };
}
