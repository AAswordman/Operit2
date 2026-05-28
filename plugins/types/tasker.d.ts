/**
 * Tasker integration type definitions for Assistance Package Tools
 * 
 * This module provides integration with Tasker automation app,
 * allowing you to trigger Tasker events from your assistance tools.
 */

/**
 * Tasker operations namespace
 */
export namespace Tasker {
    /**
     * Parameters for triggering a Tasker event.
     * 
     * Allows passing up to 5 string arguments (arg1-arg5) and/or
     * a JSON string for complex data structures.
     */
    export interface TriggerTaskerEventParams {
        /** 
         * Event type identifier - used to identify which Tasker task to trigger
         * This should match the event name configured in Tasker
         */
        task_type: string;

        /** Optional argument 1 - general purpose string parameter */
        arg1?: string;

        /** Optional argument 2 - general purpose string parameter */
        arg2?: string;

        /** Optional argument 3 - general purpose string parameter */
        arg3?: string;

        /** Optional argument 4 - general purpose string parameter */
        arg4?: string;

        /** Optional argument 5 - general purpose string parameter */
        arg5?: string;

        /** 
         * JSON string for passing arbitrary complex data structures
         * Use this when you need to pass more than 5 arguments or complex objects
         */
        args_json?: string;
    }

    /**
     * Runtime Tasker API exposed under Tools.Tasker
     */
    export interface Runtime {
        /**
         * Trigger a Tasker event
         * 
         * Sends an event to Tasker with the specified parameters.
         * The event can be caught and processed by Tasker tasks.
         * 
         * @param params - Event parameters including task_type and optional arguments
         * @returns Promise resolving to a status message from the native layer
         * 
         * @example
         * ```typescript
         * // Trigger a simple event
         * await Tools.Tasker.triggerEvent({
         *   task_type: "MyTaskEvent",
         *   arg1: "param1",
         *   arg2: "param2"
         * });
         * 
         * // Trigger with JSON data
         * await Tools.Tasker.triggerEvent({
         *   task_type: "ComplexEvent",
         *   args_json: JSON.stringify({
         *     data: { key: "value" },
         *     timestamp: Date.now()
         *   })
         * });
         * ```
         */
        triggerEvent(params: TriggerTaskerEventParams): Promise<string>;
    }
}
