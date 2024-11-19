<script lang="ts">
	import { open } from "@tauri-apps/plugin-dialog";
	import { appConfigDir } from "@tauri-apps/api/path";
	import SigmaGraph from "./SigmaGraph.svelte";

	// Graph Data Structure
	type NodeDatum = {
		id: string;
		label: string;
		color: string;
};

	type LinkDatum = {
		id: string;
		source: string;
		target: string;
		color: string;
		active: boolean;
	};

	type GraphData = {
		nodes: NodeDatum[];
		links: LinkDatum[];
	};

	let isDataLoaded = false;
	let data: GraphData = { nodes: [], links: [] };

	const handleFileOpen = async () => {
		const selected = await open({
			multiple: false,
			defaultPath: await appConfigDir(),
			filters: [{ name: "jsonl", extensions: ["jsonl"] }],
		});

		if (selected) {
			console.log("Selected file:", selected);

			// Example JSONL parsing
			const fileContent = await fetch(selected).then((res) => res.text());
			const parsedData: GraphData = JSON.parse(
				`[${fileContent.replace(/\n/g, ",")}]`,
			);

			// Ensure data validity
			if (parsedData?.nodes && parsedData?.links) {
				data = parsedData;
				isDataLoaded = true;
				console.log("Loaded Graph Data:", data);
			} else {
				console.error("Invalid data format");
			}
		}
	};

	let sigmaGraphRef;

	const filterNodesByColor = (color) => {
		sigmaGraphRef?.filterNodes((node) => node.color === color);
	};

	const resetGraphFilters = () => {
		sigmaGraphRef?.resetFilters();
	};
</script>

<div class="container">
	<button on:click={handleFileOpen}>Import Data</button>
	<button on:click={() => filterNodesByColor("#35D068")}
		>Filter by Green Nodes</button
	>
	<button on:click={resetGraphFilters}>Reset Filters</button>
</div>

<div class="graph-container">
	{#if isDataLoaded}
		<SigmaGraph bind:this={sigmaGraphRef} {data} />
	{/if}
</div>

<style>
	.container {
		display: flex;
		justify-content: center;
		gap: 10px;
		margin: 20px 0;
	}

	.graph-container {
		width: 100%;
		height: 600px;
	}

	button {
		padding: 10px 20px;
		font-size: 16px;
		cursor: pointer;
	}
</style>
