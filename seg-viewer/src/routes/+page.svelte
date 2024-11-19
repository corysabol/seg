<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { open as openFile } from "@tauri-apps/plugin-fs";
  import { appConfigDir } from "@tauri-apps/api/path";

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

  // State ====
  let isDataLoaded: boolean = $state(false);
  let data: GraphData = $state({ nodes: [], links: [] });
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
      (
        invoke("load_data", { filePath: selected[0] }) as Promise<GraphData>
      ).then((parsedData) => {
        data = parsedData;
        isDataLoaded = true;
        console.log(data);
      });
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

<div id="vis-container"></div>

<style>
  .container {
    width: 100%;
    display: flex;
    flex-direction: column;
  }
</style>
