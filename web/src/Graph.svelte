<script lang="ts">
 import type { RulesMap, ArtifactsMap, RuleState } from './api';
 import type { Graph } from './graph';

 import { fade } from 'svelte/transition';
 import { tweened } from 'svelte/motion';
 import { writable, derived } from 'svelte/store';
 import { cubicOut } from 'svelte/easing';
 import Progress from './Progress.svelte';

 export let rules: RulesMap;
 export let artifacts: ArtifactsMap;
 export let graph: Graph;

 export interface ProgressBarSettings {
     height: number,
     margin: number,
     location: 'T' | 'B',
 }

 export let progress: ProgressBarSettings = {height: 18, margin: 8, location: 'B'};

 const progress_total = writable(0);
 const progress_processing = tweened(0, {
     duration: 500,
     easing: cubicOut,
 });
 const progress_scheduled = tweened(0, {
     duration: 500,
     easing: cubicOut,
 });
 const progress_processed = derived([progress_total, progress_processing, progress_scheduled], ([total, processing, scheduled]) => total - processing - scheduled);

 $: {
     const {processed, processing, scheduled} = Object.values(rules).reduce((counters, rule) => {
         counters[rule.state]++;
         return counters;
     }, {processed: 0, processing: 0, scheduled: 0});

     progress_total.set(processed + processing + scheduled);
     progress_processing.set(processing);
     progress_scheduled.set(scheduled);
 }

 function node_class(artifacts: ArtifactsMap, name: string): string {
		 const artifact = artifacts[name];
		 return artifact.goal ? 'goal' : artifact.rule ? 'product' : 'source';
 }

 function state_class(artifacts: ArtifactsMap, rules: RulesMap, name: string): string {
		 const artifact = artifacts[name];
		 return artifact.rule ? rules[artifact.rule].state : 'processed';
 }

 function edge_class(artifacts: ArtifactsMap, source: string, sink: string): string {
		 const artifact = artifacts[sink];
		 return artifact.goal ? 'phony' : 'actual';
 }

 function point_coords({x, y}: Point): string {
		 return `${x},${y}`;
 }

 function edge_path(points: Point[]) {
		 if (points.length < 1) return "";
		 let path = `M${point_coords(points[0])}`;
		 for (let i = 1; i < points.length; i++) {
				 path += ` L${point_coords(points[i])}`;
		 }
		 return path;
 }

 function copy_label(label: string) {
     navigator.clipboard.writeText(label).then(() => {
         console.log('label copyed to clipboard');
     }, (error) => {
         console.error(`error when copying label to clipboard: ${error}`);
     });
 }
</script>

<svg viewBox="0 0 {graph.width} {graph.height + progress.height + progress.margin}">
		<defs>
				<marker id="arrow" markerWidth="10" markerHeight="7"
								refX="10" refY="3.5" orient="auto" class="arrow">
						<polygon points="0 0, 10 3.5, 0 7" />
				</marker>
				<pattern id="pattern-scheduled"
								 width="8" height="8"
								 patternUnits="userSpaceOnUse"
								 patternTransform="rotate(45)">
						<rect width="4" height="8" fill="white" fill-opacity="40%"></rect>
        </pattern>
				<pattern id="pattern-processing"
								 width="8" height="8"
								 patternUnits="userSpaceOnUse"
								 patternTransform="rotate(45)">
						<animateTransform attributeType="xml"
															attributeName="patternTransform"
															additive="sum"
															type="translate" from="0" to="8" begin="0"
															dur="1s" repeatCount="indefinite"/>
						<rect width="4" height="8" fill="white" fill-opacity="40%"></rect>
        </pattern>
		</defs>
    <g transform="translate(0, {progress.location == 'T' ? progress.height + progress.margin : 0})">
		    {#each graph.nodes as {x, y, width, height, label, font_size, text_lines}}
				    <rect x={x - width/2 + 1} y={y - height/2 + 1} width={width-2} height={height-2} class="node node-{node_class(artifacts, label)}" on:click={copy_label(label)} />
            {#if state_class(artifacts, rules, label) != "processed"}
				        <rect x={x - width/2 + 1} y={y - height/2 + 1} width={width-2} height={height-2} class="rule rule-{state_class(artifacts, rules, label)}" on:click={copy_label(label)} transition:fade />
            {/if}
				    <text x={x} y={y} class="label" font-size="{font_size}px" on:click={copy_label(label)}>
						    {#each text_lines as line, i}
								    <tspan x={x} dy={font_size * (i > 0 ? 1 : -((text_lines.length - 1) / 2))}>{line}{i < text_lines.length - 1 ? "↩" : ""}</tspan>
						    {/each}
				    </text>
		    {/each}
		    {#each graph.edges as {points, source, sink}}
				    <path class="edge edge-{edge_class(artifacts, source, sink)}" d="{edge_path(points)}" marker-end="url(#arrow)" />
		    {/each}
    </g>
    <g transform="translate(0, {progress.location == 'T' ? 0 : graph.height + progress.margin})">
        <!-- {#if progress_counters.scheduled > 0 || progress_counters.processing > 0} -->
        <Progress width={graph.width} height={progress.height} processed={$progress_processed} processing={$progress_processing} scheduled={$progress_scheduled} />
        <!-- {/if} -->
    </g>
</svg>

<style>
 svg {
		 width: 100%;
		 height: 100%;
 }
 .label {
		 fill: #222;
		 text-anchor: middle;
		 dominant-baseline: central;
 }
 .node {
		 rx: 10;
		 ry: 10;
		 stroke: #555;
		 stroke-width: 0.5;
 }
 .node-source {
		 fill: #7fffd4;
 }
 .node-product {
		 fill: #ffc0cb;
 }
 .node-goal {
		 fill: #ffc125;
 }
 .rule {
		 rx: 10;
		 ry: 10;
 }
 .rule-processed {
		 fill: none;
 }
 .rule-scheduled {
		 fill: url(#pattern-scheduled);
 }
 .rule-processing {
		 fill: url(#pattern-processing);
 }
 .edge {
		 fill: none;
		 stroke: #555;
		 stroke-width: 1;
		 stroke-linecap: round;
 }
 .edge-actual {}
 .edge-phony {
		 stroke-dasharray: 8 4;
 }
 .arrow {
		 fill: #555;
		 stroke: #555;
		 stroke-width: 1;
		 stroke-linecap: round;
 }
</style>
