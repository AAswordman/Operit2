/**
 * Workflow type definitions for Assistance Package Tools
 * 
 * This file provides type definitions for workflow-related functionality.
 */

import {
    WorkflowListResultData,
    WorkflowResultData,
    WorkflowDetailResultData,
    WorkflowNode,
    WorkflowNodeConnection,
    ConditionNode,
    LogicNode,
    ExtractNode,
    ConditionOperator,
    LogicOperator,
    ExtractMode,
    ParameterValue,
    TriggerType,
    TriggerNode,
    ExecuteNode,
    NodePosition,
    StringResultData
} from './results';

/**
 * Workflow namespace containing workflow-related types and utilities
 */
export namespace Workflow {
    /**
     * Node position in the workflow canvas
     */
    export type Position = NodePosition;

    /**
     * Trigger node configuration
     */
    export interface Trigger extends TriggerNode { }

    /**
     * Execute node configuration
     */
    export interface Execute extends ExecuteNode { }

    /**
     * Condition node configuration
     */
    export interface Condition extends ConditionNode { }

    /**
     * Logic node configuration
     */
    export interface Logic extends LogicNode { }

    /**
     * Extract node configuration
     */
    export interface Extract extends ExtractNode { }

    /**
     * Workflow node (union type)
     */
    export type Node = WorkflowNode;

    export type ParameterValueInput =
        | string
        | number
        | boolean
        | null
        | {
            value?: string;
            nodeId?: string;
            ref?: string;
            refNodeId?: string;
        };

    export interface NodeInput {
        id?: string;
        type: 'trigger' | 'execute' | 'condition' | 'logic' | 'extract';
        name?: string;
        description?: string;
        position?: { x: number; y: number };

        triggerType?: TriggerType;
        triggerConfig?: Record<string, string>;

        actionType?: string;
        actionConfig?: Record<string, ParameterValueInput>;
        jsCode?: string;

        left?: ParameterValueInput;
        operator?: string;
        right?: ParameterValueInput;

        source?: ParameterValueInput;
        mode?: string;
        expression?: string;
        group?: number;
        defaultValue?: string;

        others?: ParameterValueInput[];
        startIndex?: number;
        length?: number;
        randomMin?: number;
        randomMax?: number;
        randomStringLength?: number;
        randomStringCharset?: string;
        useFixed?: boolean;
        fixedValue?: string;
    }

    /**
     * Workflow connection between nodes
     */
    export interface Connection extends WorkflowNodeConnection { }

    export type ConnectionConditionKeyword =
        | 'true'
        | 'false'
        | 'on_success'
        | 'success'
        | 'ok'
        | 'on_error'
        | 'error'
        | 'failed';

    export type ConnectionCondition = ConnectionConditionKeyword | (string & { __regexConditionBrand?: never });

    export interface ConnectionInput {
        id?: string;
        sourceNodeId?: string;
        targetNodeId?: string;
        condition?: ConnectionCondition | null;
    }

    /**
     * Basic workflow information
     */
    export interface Info extends WorkflowResultData { }

    /**
     * Detailed workflow information with nodes and connections
     */
    export interface Detail extends WorkflowDetailResultData { }

    /**
     * Workflow list response
     */
    export interface List extends WorkflowListResultData { }

    /**
     * Parameters for creating a workflow
     */
    export interface CreateParams {
        /** Workflow name */
        name: string;
        /** Workflow description (optional) */
        description?: string;
        /** Nodes array or JSON string (optional) */
        nodes?: NodeInput[] | string;
        /** Connections array or JSON string (optional) */
        connections?: ConnectionInput[] | string;
        /** Whether the workflow is enabled (optional, default true) */
        enabled?: boolean;
    }

    /**
     * Parameters for getting a workflow
     */
    export interface GetParams {
        /** Workflow ID */
        workflow_id: string;
    }

    /**
     * Parameters for updating a workflow
     */
    export interface UpdateParams {
        /** Workflow ID */
        workflow_id: string;
        /** New workflow name (optional) */
        name?: string;
        /** New workflow description (optional) */
        description?: string;
        /** New nodes array or JSON string (optional) */
        nodes?: NodeInput[] | string;
        /** New connections array or JSON string (optional) */
        connections?: ConnectionInput[] | string;
        /** Whether the workflow is enabled (optional) */
        enabled?: boolean;
    }

    /**
     * Parameters for deleting a workflow
     */
    export interface DeleteParams {
        /** Workflow ID */
        workflow_id: string;
    }

    export interface EnableParams {
        /** Workflow ID */
        workflow_id: string;
    }

    export interface DisableParams {
        /** Workflow ID */
        workflow_id: string;
    }

    /**
     * Parameters for triggering a workflow
     */
    export interface TriggerParams {
        /** Workflow ID */
        workflow_id: string;
    }

    export type PatchOperation = 'add' | 'update' | 'remove';

    export interface NodePatch {
        op: PatchOperation;
        id?: string;
        node?: NodeInput;
    }

    export interface ConnectionPatch {
        op: PatchOperation;
        id?: string;
        connection?: ConnectionInput;
    }

    export interface PatchParams {
        /** Workflow ID */
        workflow_id: string;

        /** New workflow name (optional) */
        name?: string;

        /** New workflow description (optional) */
        description?: string;

        /** Whether the workflow is enabled (optional) */
        enabled?: boolean;

        /** Node patch operations (optional) */
        node_patches?: NodePatch[] | string;

        /** Connection patch operations (optional) */
        connection_patches?: ConnectionPatch[] | string;
    }

    export interface Runtime {
        getAll(): Promise<WorkflowListResultData>;

        create(
            name: string,
            description?: string,
            nodes?: NodeInput[] | string | null,
            connections?: ConnectionInput[] | string | null,
            enabled?: boolean
        ): Promise<WorkflowDetailResultData>;

        get(workflowId: string): Promise<WorkflowDetailResultData>;

        update(
            workflowId: string,
            updates?: Omit<UpdateParams, 'workflow_id'>
        ): Promise<WorkflowDetailResultData>;

        patch(
            workflowId: string,
            patch?: Omit<PatchParams, 'workflow_id'>
        ): Promise<WorkflowDetailResultData>;

        setEnabled(workflowId: string, enabled: boolean): Promise<WorkflowDetailResultData>;

        enable(workflowId: string): Promise<WorkflowDetailResultData>;

        disable(workflowId: string): Promise<WorkflowDetailResultData>;

        'delete'(workflowId: string): Promise<StringResultData>;

        trigger(workflowId: string): Promise<StringResultData>;
    }
}

// Re-export workflow-related types at module level for convenience
export {
    WorkflowListResultData,
    WorkflowResultData,
    WorkflowDetailResultData,
    WorkflowNode,
    WorkflowNodeConnection,
    ConditionNode,
    LogicNode,
    ExtractNode,
    ConditionOperator,
    LogicOperator,
    ExtractMode,
    ParameterValue,
    TriggerNode,
    ExecuteNode,
    NodePosition
};
