<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { open } from "@tauri-apps/api/dialog";
  import { appDir } from "@tauri-apps/api/path";

  const handle_file_open = async () => {
    const selected = await open({
      multiple: true,
      defaultPath: await appDir(),
      filters: [
        {
          name: "network-data",
          extensions: ["jsonl"],
        },
      ],
    });

    if (Array.isArray(selected)) {
      // User selected multiple directories
      console.log(selected);

      // Now we need to open the file
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
