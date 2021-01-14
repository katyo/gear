<script lang="ts">
 export let x: number = 0;
 export let y: number = 0;
 export let width: number = 0;
 export let height: number = 0;

 export let processed: number = 0;
 export let processing: number = 0;
 export let scheduled: number = 0;

 $: font_size = `${height/2}px`;

 $: total = Math.max(processed + processing + scheduled, Number.EPSILON);
 $: total_count = Math.round(total);

 $: processed_percent = Math.round(processed / total * 100);
 $: processing_percent = Math.round(processing / total * 100);
 $: scheduled_percent = Math.round(scheduled / total * 100);

 $: processed_count = Math.round(processed);
 $: processing_count = Math.round(processing);
 $: scheduled_count = Math.round(scheduled);

 $: processed_size = (processed / total) * width;
 $: processing_size = (processing / total) * width;
 $: scheduled_offset = processed_size + processing_size;
 $: scheduled_size = width - scheduled_offset;
</script>

<rect {x} {y} width={processed_size} {height} class="progress-processed" />
<text x={x + processed_size/2} y={y + height/2} width={processed_size} {height} font-size={font_size} class="label">{#if processed_count > 0}{processed_percent}% ({processed_count}/{total_count}){/if}</text>
<rect x={x + processed_size} {y} width={processing_size} {height} class="progress-processing" />
<rect x={x + processed_size} {y} width={processing_size} {height} class="overlay-processing" />
<text x={x + processed_size + processing_size/2} y={y + height/2} width={processing_size} {height} font-size={font_size} class="label">{#if processing_count > 0}{processing_percent}% ({processing_count}/{total_count}){/if}</text>
<rect x={x + scheduled_offset} {y} width={scheduled_size} {height} class="progress-scheduled" />
<rect x={x + scheduled_offset} {y} width={scheduled_size} {height} class="overlay-scheduled" />
<text x={x + scheduled_offset + scheduled_size/2} y={y + height/2} width={scheduled_size} {height} font-size={font_size} class="label">{#if scheduled_count > 0}{scheduled_percent}% ({scheduled_count}/{total_count}){/if}</text>

<style>
 .label {
     fill: #333;
		 text-anchor: middle;
		 dominant-baseline: central;
 }
 .progress-processed {
		 fill: #7fffd4;
 }
 .progress-processing {
		 fill: #ffc0cb;
 }
 .progress-scheduled {
		 fill: #ffc125;
 }
 .overlay-scheduled {
		 fill: url(#pattern-scheduled);
 }
 .overlay-processing {
		 fill: url(#pattern-processing);
 }
</style>
