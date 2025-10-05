import { invoke } from '@tauri-apps/api/core';

/**
 * Creates standardized sync functions for a simulation mode.
 * The backend is the single source of truth.
 * 
 * @template TSettings - Type definition for settings
 * @template TState - Type definition for state
 */
export function createSyncManager<TSettings, TState>() {
    return {
        /**
         * Sync settings from backend (source of truth)
         */
        async syncSettings(): Promise<TSettings | null> {
            try {
                const data = await invoke('get_current_settings');
                return data as TSettings;
            } catch (e) {
                console.error('Failed to sync settings from backend:', e);
                return null;
            }
        },

        /**
         * Sync state from backend (source of truth)
         */
        async syncState(): Promise<TState | null> {
            try {
                const data = await invoke('get_current_state');
                return data as TState;
            } catch (e) {
                console.error('Failed to sync state from backend:', e);
                return null;
            }
        },

        /**
         * Update a single setting in the backend
         * Use optimistic updates in the UI for immediate feedback, then call this
         */
        async updateSetting(settingName: string, value: any): Promise<boolean> {
            try {
                await invoke('update_simulation_setting', {
                    settingName,
                    value,
                });
                return true;
            } catch (e) {
                console.error(`Failed to update setting '${settingName}':`, e);
                return false;
            }
        },

        /**
         * Update a single state value in the backend
         * Use optimistic updates in the UI for immediate feedback, then call this
         */
        async updateState(stateName: string, value: any): Promise<boolean> {
            // stateName isn't snake case, log an error
            if (stateName !== stateName.toLowerCase()) {
                console.error(`State name '${stateName}' is not snake case`);
                return false;
            }

            try {
                await invoke('update_simulation_state', {
                    stateName,
                    value,
                });
                return true;
            } catch (e) {
                console.error(`Failed to update state '${stateName}':`, e);
                return false;
            }
        },

        /**
         * Helper for optimistic setting updates with automatic rollback on error
         * @param settings - Current settings object (will be mutated)
         * @param settingName - Name of the setting to update
         * @param newValue - New value for the setting
         * @param shouldSync - Whether to sync from backend after update (default: false)
         * @returns Updated settings or null/undefined on error
         */
    async updateSettingOptimistic(
        settings: TSettings | null | undefined,
        settingName: keyof TSettings,
        newValue: any,
        shouldSync: boolean = false
    ): Promise<TSettings | null | undefined> {
        if (!settings) return settings;

        const oldValue = settings[settingName];

        // Optimistic update
        (settings as any)[settingName] = newValue;

        try {
            await invoke('update_simulation_setting', {
                settingName: String(settingName),
                value: newValue,
            });

            // Optionally sync from backend to ensure consistency
            if (shouldSync) {
                const synced = await this.syncSettings();
                return synced || settings;
            }

            return settings;
        } catch (e) {
            console.error(`Failed to update setting '${String(settingName)}':`, e);
            // Rollback on error
            (settings as any)[settingName] = oldValue;
            return settings;
        }
    },

        /**
         * Helper for optimistic state updates with automatic rollback on error
         * @param state - Current state object (will be mutated)
         * @param stateName - Name of the state to update
         * @param newValue - New value for the state
         * @param shouldSync - Whether to sync from backend after update (default: false)
         * @returns Updated state or null/undefined on error
         */
    async updateStateOptimistic(
        state: TState | null | undefined,
        stateName: keyof TState,
        newValue: any,
        shouldSync: boolean = false
    ): Promise<TState | null | undefined> {
        if (!state) return state;

        const oldValue = state[stateName];

        // Optimistic update
        (state as any)[stateName] = newValue;

        try {
            await invoke('update_simulation_state', {
                stateName: String(stateName),
                value: newValue,
            });

            // Optionally sync from backend to ensure consistency
            if (shouldSync) {
                const synced = await this.syncState();
                return synced || state;
            }

            return state;
        } catch (e) {
            console.error(`Failed to update state '${String(stateName)}':`, e);
            // Rollback on error
            (state as any)[stateName] = oldValue;
            return state;
        }
    },

        /**
         * Sync both settings and state from backend
         * Use after operations that affect multiple values (presets, resets, etc.)
         */
        async syncAll(): Promise<{
            settings: TSettings | null | undefined;
            state: TState | null | undefined;
        }> {
            const [settings, state] = await Promise.all([
                this.syncSettings(),
                this.syncState(),
            ]);
            return { settings, state };
        },
    };
}

/**
 * Type-safe wrapper for setting updates with validation
 */
export function createSettingUpdater<T>(
    getValue: () => T | null | undefined,
    updateFn: (name: string, value: any) => Promise<boolean>
) {
    return async (settingName: keyof T, value: any): Promise<boolean> => {
        const current = getValue();
        if (!current) {
            console.warn('Cannot update setting: no current value available');
            return false;
        }

        return await updateFn(String(settingName), value);
    };
}

