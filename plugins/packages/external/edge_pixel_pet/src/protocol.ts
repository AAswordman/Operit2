/** Core-side wrapper. Protocol failures are distinct from authorization/Link failures. */
export async function sceneCall(node: string, action: string, fields: Record<string, any> = {}): Promise<any> {
    const args = { v: 1, ...fields };
    // All schema fields/chunks are ASCII in this example. JSON bytes <= 1024.
    if (JSON.stringify(args).length > 1024)
        throw new Error('Scene request exceeds 1024 bytes');
    const { data } = await tools.edge.execute(node, { pluginId: 'display.scene', action }, args);
    if (data?.v !== 1 || typeof data.ok !== 'boolean')
        throw new Error('Invalid scene protocol reply');
    if (!data.ok) {
        const error = new Error(`${data.error.code}: ${data.error.message}`);
        (error as any).code = data.error.code;
        throw error;
    }
    return data.result;
}
