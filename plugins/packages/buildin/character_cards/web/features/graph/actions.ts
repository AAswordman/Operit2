import type { EditorContext } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { graphData, graphViewport, selectedMemory } from "../graph/data";
import { fitMemoryGraph, memoryGraphBounds, zoomMemoryGraph } from "../graph/layout";
/** Binds graph actions operations to one editor context. */
export function createGraphActionsFeature(context: EditorContext) {
  const { topDialog, pushDialog, request, renderDialogs, updateGraphSurface } = context;


  /** Opens a graph with data strictly scoped to the selected card's current binding. */
  async function openGraph(ownerKey: string, name: string): Promise<void> {
    const dialog = pushDialog({ type: "graph", ownerKey, name, loading: true, data: null, selected: null, selectedEdge: null, linkMode: false, linkSource: null, viewport: null, layoutSignature: null, sizes: new Map(), query: "", appliedQuery: "", searchIds: null, folder: "*", positions: new Map(), camera: { x: 0, y: 0, scale: 1 } });
    try { dialog.data = await request({ action: "graph", ownerKey }); dialog.loading = false; renderDialogs(); }
    catch (error) { dialog.loading = false; dialog.error = String(error); renderDialogs(); }
  }

  /** Executes only this feature's registered actions without choosing another transport. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "owner-graph": await openGraph(dataValue(element, "owner"), dataValue(element, "name")); return;

      case "refresh-graph": { const dialog = topDialog("graph"); dialog.data = await request({ action: "graph", ownerKey: dialog.ownerKey }); renderDialogs(); return; }

      case "search-graph": {
        const graph = topDialog("graph"), query = graph.query.trim();
        if (query === "") graph.searchIds = null;
        else {
          const items = await request({ action: "searchMemory", ownerKey: graph.ownerKey, query });
          graph.searchIds = new Set(); for (const item of items) graph.searchIds.add(item.uuid);
        }
        graph.appliedQuery = query; graph.selected = null; updateGraphSurface(graph); return;
      }

      case "clear-search": { const graph = topDialog("graph"); graph.query = ""; graph.appliedQuery = ""; graph.searchIds = null; renderDialogs(); return; }

      case "select-edge": {
        const graph = topDialog("graph"), linkId = dataValue(element, "link");
        const edge = graphData(graph).graph.edges.find(
          /** Preserves the lossless string identity from the plugin's graph contract. */
          item => item.id === linkId,
        );
        if (edge === undefined) throw new Error("记忆关系不存在");
        graph.selected = null; graph.selectedEdge = edge.id; updateGraphSurface(graph); return;
      }

      case "select-memory": {
        const graph = topDialog("graph"); graph.selected = dataValue(element, "id"); graph.selectedEdge = null;
        if (graph.linkMode) {
          if (graph.linkSource === null) graph.linkSource = graph.selected;
          else if (graph.linkSource !== graph.selected) {
            const source = graphData(graph).items.find(
              /** Resolves the selected source node to the exact memory UUID. */
              item => item.uuid === graph.linkSource,
            );
            const target = selectedMemory(graph);
            if (source === undefined || target === null) throw new Error("无法定位记忆节点");
            pushDialog({ type: "link", ownerKey: graph.ownerKey, items: graphData(graph).items, draft: { sourceTitle: source.title, targetTitle: target.title, linkType: "related", weight: 1, description: "" } }); return;
          }
        }
        updateGraphSurface(graph); return;
      }

      case "zoom-in":
      case "zoom-out": {
        const graph = topDialog("graph"); graph.camera = zoomMemoryGraph(graph.camera, { x: graphViewport(graph).width / 2, y: graphViewport(graph).height / 2 }, action === "zoom-in" ? 1.25 : 1 / 1.25); updateGraphSurface(graph); return;
      }

      case "fit-graph": { const graph = topDialog("graph"); graph.camera = fitMemoryGraph(memoryGraphBounds(graph.positions, graph.sizes), graphViewport(graph).width, graphViewport(graph).height); updateGraphSurface(graph); return; }

      case "export-graph": throw new Error("插件记忆备份工作流尚未接入");
      default: throw new Error(`Unknown graph/actions action: ${action}`);
    }
  }
  return { names: ["owner-graph", "refresh-graph", "search-graph", "clear-search", "select-edge", "select-memory", "zoom-in", "zoom-out", "fit-graph", "export-graph"], handleAction, openGraph };
}
