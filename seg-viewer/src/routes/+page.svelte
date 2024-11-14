<script lang="ts">
  import { type PacketInfo, PacketInfoSchema } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { open as openFile } from "@tauri-apps/plugin-fs";
  import { appConfigDir } from "@tauri-apps/api/path";
  import { VisSingleContainer, VisGraph } from "@unovis/svelte";
  import { GraphLayoutType, GraphNodeShape } from "@unovis/ts";
  import { writable } from "svelte/store";
    import type { tooltip } from "@unovis/ts/components/tooltip/style";

  // Data ====
  type NodeDatum = {
    id: string;
    label: string;
    shape: string;
    color: string;
  }

  type LinkDatum = {
    id: string;
    source: string;
    target: string;
    active: boolean;
    color: string;
  }

  type GraphData = {
    nodes: NodeDatum[];
    links: LinkDatum[];
  }
  // =========

  const layoutType = GraphLayoutType.Dagre
  const nodeLabel = (n: NodeDatum) => n.label;
  const nodeShape = (n: NodeDatum) => n.shape as GraphNodeShape;
  const nodeStroke = (l: LinkDatum) => l.color;
  const linkFlow = (l: LinkDatum) => l.active;
  const linkStroke = (l: LinkDatum) => l.color;

  // State ====
  let isDataLoaded: boolean = false;
  let data: GraphData;
  // ==========

  const handle_file_open = async () => {
    const selected = await open({
      multiple: true,
      defaultPath: await appConfigDir(),
      filters: [
        {
          name: "jsonl",
          extensions: ["jsonl"],
        },
      ],
    });

    if (Array.isArray(selected)) {
      // User selected multiple directories
      console.log(selected);

      // Now we need to open the file
      (invoke('load_data', { filePath: selected[0] }) as Promise<GraphData>).then(
        (parsedData) => {
          data = parsedData;
          isDataLoaded = true;
          console.log(data);
        }
      );

    } else if (selected === null) {
      // User cancelled the selection
    } else {
      // user selected a single directory
    }
  };
 
</script>

<div class="container">
  <button on:click={handle_file_open}>Import data</button>
</div>

<div class="container">
  {#if isDataLoaded}
  <VisSingleContainer {data} height={600}>
    <VisGraph
      {layoutType}
      {nodeLabel}
      {nodeShape}
      {nodeStroke}
      {linkFlow}
      {linkStroke}
    />
  </VisSingleContainer>
  {/if}
</div>

<style>
  .container {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    padding: 1rem;
    border-bottom: 1px solid #ccc;
  }
  .graph-view {
    flex: 1;
    overflow: hidden;
  }
</style>
