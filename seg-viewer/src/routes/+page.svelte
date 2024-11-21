<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
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

  const graph = $state(new Graph());
  onMount(() => {
    // Need to figure out how to make the graph a reactive value
    // So that the graph can be updated with imported data by the user.
    //const graph = new Graph();

    graph.addNode("John", {
      x: 0,
      y: 10,
      size: 15,
      label: "John",
      color: "blue",
    });
    graph.addNode("Mary", {
      x: 10,
      y: 0,
      size: 10,
      label: "Mary",
      color: "green",
    });
    graph.addNode("Thomas", {
      x: 7,
      y: 9,
      size: 20,
      label: "Thomas",
      color: "red",
    });
    graph.addNode("Hannah", {
      x: -7,
      y: -6,
      size: 25,
      label: "Hannah",
      color: "teal",
    });

    graph.addEdge("John", "Mary");
    graph.addEdge("John", "Thomas");
    graph.addEdge("John", "Hannah");
    graph.addEdge("Hannah", "Thomas");
    graph.addEdge("Hannah", "Mary");

    const renderer = new Sigma(graph, container);
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
