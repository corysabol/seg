<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import ForceSupervisor from "graphology-layout-force/worker";
  import { open as openFile } from "@tauri-apps/plugin-fs";
  import { appConfigDir } from "@tauri-apps/api/path";

  import Sigma from "sigma";
  import Graph from "graphology";
  import { onMount } from "svelte";

  // Data ====
  type NodeDatum = {
    id: string;
    label: string;
    shape: string;
    color: string;
  };

  type LinkDatum = {
    id: string;
    label: string;
    source: string;
    target: string;
    active: boolean;
    color: string;
  };

  type GraphData = {
    nodes: NodeDatum[];
    links: LinkDatum[];
  };

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
      (invoke("load_data", { filePath: selected[0] }) as Promise<string>).then(
        (rawData) => {
          let parsedData = JSON.parse(rawData);
          let graphData: GraphData = parsedData as GraphData;

          console.log(graphData);
          console.log(graphData.nodes);
          processDataToGraph(graphData);
          console.log(graph);
        },
      );
    } else if (selected === null) {
      // User cancelled the selection
    } else {
      // user selected a single directory
    }
  };

  const processDataToGraph = (data: GraphData) => {
    // TODO: Process nodes
    console.log(data.nodes);

    data.nodes.map((n: NodeDatum) => {
      graph.addNode(n.id, {
        x: 0,
        y: 0,
        size: 25,
        label: n.label,
        color: n.color,
      });
    });
    // TODO: Process and add edges
    data.links.map((l: LinkDatum) => {
      graph.addEdge(l.source, l.target);
    });
  };

  let container: HTMLElement;

  const graph = $state(new Graph({ multi: true }));
  const layout = new ForceSupervisor(graph, {
    isNodeFixed: (_, attr) => attr.highlighted,
  });
  const draggedNode: string | null = null;
  let isDragging = false;

  onMount(() => {
    layout.start();

    const renderer = new Sigma(graph, container);

    renderer.on("downNode", (e) => {});
    renderer.on("moveBody", (e) => {});
    renderer.on("clickStage", (e) => {});
    renderer.on("clickNode", (e) => {});
    renderer.on("clickEdge", (e) => {});
  });
</script>

<div class="container">
  <button onclick={handle_file_open}>Import data</button>
</div>

<div bind:this={container} id="vis-container" class="vis-container"></div>

<style>
  .container {
    width: 100%;
    display: flex;
    flex-direction: column;
  }

  .vis-container {
    width: 800px;
    height: 600px;
  }
</style>
