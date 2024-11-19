<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { open as openFile } from "@tauri-apps/plugin-fs";
  import { appConfigDir } from "@tauri-apps/api/path";
  import { VisSingleContainer, VisGraph } from "@unovis/svelte";
  import {
    Graph,
    GraphLayoutType,
    GraphNodeShape,
    SingleContainer,
    type GraphLinkLabel,
  } from "@unovis/ts";

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
  // =========

  const layoutType = GraphLayoutType.Dagre;
  const nodeLabel = (n: NodeDatum) => n.label;
  const nodeShape = (n: NodeDatum) => n.shape as GraphNodeShape;
  const nodeStroke = (l: LinkDatum) => l.color;
  const linkLabel = (l: LinkDatum) => {
    /*
    export type GraphCircleLabel = {
      text: string;
      textColor?: string | null;
      color?: string | null;
      cursor?: string | null;
      fontSize?: string | null;
      radius?: number;
    }
    */
    return {
      text: l.label,
    } as GraphLinkLabel;
  };
  const linkFlow = (l: LinkDatum) => l.active;
  const linkStroke = (l: LinkDatum) => l.color;

  // State ====
  let isDataLoaded: boolean = $state(false);
  let data: GraphData = $state({ nodes: [], links: [] });
  // ==========

  data = {
    nodes: [
      {
        id: "172.19.254.181:scanner",
        label: "foo:172.19.254.181:scanner",
        shape: "hexagon",
        color: "#35D068",
      },
      {
        id: "91.189.91.157:scanner",
        label: "foo:91.189.91.157:scanner",
        shape: "hexagon",
        color: "#35D068",
      },
      {
        id: "172.19.247.124:listener",
        label: "foo:172.19.247.124:listener",
        shape: "square",
        color: "#35D068",
      },
    ],
    links: [
      {
        id: "172.19.254.181:172.19.247.124:24151",
        label: "53686 -> 24151",
        source: "172.19.254.181:scanner",
        target: "172.19.247.124:listener",
        active: true,
        color: "#35D068",
      },
      {
        id: "172.19.254.181:172.19.247.124:50563",
        label: "50356 -> 50563",
        source: "172.19.254.181:scanner",
        target: "172.19.247.124:listener",
        active: true,
        color: "#35D068",
      },
      {
        id: "172.19.254.181:172.19.247.124:41534",
        label: "41954 -> 41534",
        source: "172.19.254.181:scanner",
        target: "172.19.247.124:listener",
        active: true,
        color: "#35D068",
      },
      {
        id: "172.19.254.181:172.19.247.124:63750",
        label: "38230 -> 63750",
        source: "172.19.254.181:scanner",
        target: "172.19.247.124:listener",
        active: true,
        color: "#35D068",
      },
      {
        id: "172.19.254.181:172.19.247.124:52937",
        label: "56160 -> 52937",
        source: "172.19.254.181:scanner",
        target: "172.19.247.124:listener",
        active: true,
        color: "#35D068",
      },
      {
        id: "91.189.91.157:172.19.247.124:58989",
        label: "123 -> 58989",
        source: "91.189.91.157:scanner",
        target: "172.19.247.124:listener",
        active: true,
        color: "#35D068",
      },
      {
        id: "172.19.254.181:172.19.247.124:60771",
        label: "62758 -> 60771",
        source: "172.19.254.181:scanner",
        target: "172.19.247.124:listener",
        active: true,
        color: "#35D068",
      },
      {
        id: "172.19.254.181:172.19.247.124:46798",
        label: "62758 -> 46798",
        source: "172.19.254.181:scanner",
        target: "172.19.247.124:listener",
        active: true,
        color: "#35D068",
      },
    ],
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
      (
        invoke("load_data", { filePath: selected[0] }) as Promise<GraphData>
      ).then((parsedData) => {
        data = parsedData;
        isDataLoaded = true;
        console.log(data);
      });

      const container = document.getElementById("vis-container") as HTMLElement;
      const chart = new SingleContainer(
        container,
        {
          component: new Graph<NodeDatum, LinkDatum>({
            layoutType: layoutType,
            nodeLabel: nodeLabel,
            nodeShape: nodeShape,
            nodeStroke: (l) => l.color,
            linkLabel: linkLabel,
            linkFlow: linkFlow,
            linkStroke: linkStroke,
          }),
          height: 600,
        },
        data,
      );
    } else if (selected === null) {
      // User cancelled the selection
    } else {
      // user selected a single directory
    }
  };
</script>

<div class="container">
  <button onclick={handle_file_open}>Import data</button>
</div>

<div id="vis-container">
  <!--<VisSingleContainer {data} height={600}>
    <VisGraph
      {layoutType}
      {nodeLabel}
      {nodeShape}
      {nodeStroke}
      {linkLabel}
      {linkFlow}
      {linkStroke}
      onLayoutCalculated={() => console.log("Layout complete")}
    />
  </VisSingleContainer>-->
</div>

<style>
  .container {
    width: 100%;
    display: flex;
    flex-direction: column;
  }
</style>
