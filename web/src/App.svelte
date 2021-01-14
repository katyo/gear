<script lang="ts">
 export type { RulesMap, GoalsList } from './graph';
 import Graph from './Graph.svelte';
 import { rules, artifacts } from './api';
 import type { Point, Size, LayoutSettings } from './graph';
 import { Layout } from './graph';
 import { LocalStore } from './storage';

 const layout_settings = LocalStore<LayoutSettings>('layout_settings', {
		 node: {
				 width: 120,
				 height: 60,
				 padding: {
						 width: 30,
						 height: 20,
				 },
				 font_size: 20,
		 },
		 graph: {
				 rankdir: 'LR',
		 },
 });

 const layout = Layout(rules, artifacts, layout_settings);
</script>

<main>
		{#if $rules}
				<Graph rules={$rules} artifacts={$artifacts} graph={$layout} />
		{:else}
				Disconnected
		{/if}
</main>

<style>
 main {
		 text-align: center;
		 padding: 1em;
		 max-width: 240px;
		 margin: 0 auto;
		 position: relative;
		 width: 100%;
		 height: 100%;
 }

 /*h1 {
		 color: #ff3e00;
		 text-transform: uppercase;
		 font-size: 4em;
		 font-weight: 100;
 }*/

 @media (min-width: 640px) {
		 main {
				 max-width: none;
		 }
 }
</style>
