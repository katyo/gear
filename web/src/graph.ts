import { graphlib, layout } from './graphre/lib/index';
import type {DaGraph, GraphLabel as GraphOptions} from './graphre/lib/index';
import type {RulesMap, ArtifactsMap} from './api';
export type {GraphOptions};
import {Readable, derived} from 'svelte/store';

export interface Graph {
    width: number,
    height: number,
    nodes: Node[],
    edges: Edge[],
}

export interface Node extends Label {
    label: string,
    x: number,
    y: number,
    width: number,
    height: number,
}

export interface Edge {
    source: string,
    sink: string,
    points: Point[],
}

export interface Point {
    x: number,
    y: number,
}

export interface Size {
    width: number,
    height: number,
}

export interface TextBoxSettings extends Size {
    font_size: number,
    padding: Size,
}

export interface LayoutSettings {
    node: TextBoxSettings,
    graph: GraphOptions,
}

function build_layout(rules: RulesMap, artifacts: ArtifactsMap, settings: LayoutSettings): Graph {
    const g: DaGraph = new graphlib.Graph();

    g.setGraph(settings.graph);
    g.setDefaultEdgeLabel(() => ({}));

    const labels = build_labels(Object.keys(artifacts), settings.node);

    for (let artifact_name in artifacts) {
        g.setNode(artifact_name, { label: artifact_name, ...settings.node, ...labels[artifact_name] });
    }

    for (let rule_id in rules) {
        let rule = rules[rule_id];
        for (let output_name of rule.outputs) {
            for (let input_name of rule.inputs) {
                g.setEdge(input_name, output_name);
            }
        }
    }

    if (g.nodeCount() > 0) {
        layout(g);
    }

    return {
        width: g.graph().width,
        height: g.graph().height,
        nodes: g.nodes().map(v => g.node(v) as Node),
        edges: g.edges().map(e => ({
            source: e.v,
            sink: e.w,
            points: g.edge(e).points
        })),
    };
}

export interface Label {
    font_size: number,
    text_lines: string[],
}

export interface LabelsMap {
    [name: string]: Label,
}

function build_labels(texts: string[], { width, height, padding, font_size: base_font_size }: TextBoxSettings): LabelsMap {
    if (padding) {
        width -= padding.width;
        height -= padding.height;
    }

		const p = document.createElement("p");
		const s = `width:${width}px;word-wrap:break-word;visibility:visible;border:solid 1px black`;
		p.setAttribute("style", s);
		document.body.appendChild(p);

    const labels: LabelsMap = {};

    for (let text of texts) {
		    let font_size = base_font_size;
		    while (font_size > 0) {
				    p.style.fontSize = `${font_size}px`;
				    p.textContent = text;
				    if (p.offsetHeight > height) {
						    font_size -= 1;
				    } else {
						    break;
				    }
		    }
		    const text_height = p.offsetHeight;
		    p.textContent = "A";
		    const line_height = p.offsetHeight;

        const count_lines = Math.ceil(text_height / line_height);
		    const line_length = Math.ceil(text.length / count_lines);

        const label = labels[text] = {font_size, text_lines: []};
        while (text.length > line_length) {
            label.text_lines.push(text.substr(0, line_length));
            text = text.substr(line_length);
        }
        label.text_lines.push(text);
    }

		document.body.removeChild(p);

    return labels;
}

export function Layout($rules: Readable<RulesMap>, $artifacts: Readable<ArtifactsMap>, $settings: Readable<LayoutSettings>) {
    let _rules;
    let _settings;
    return derived([$rules, $artifacts, $settings], ([rules, artifacts, settings], set) => {
        if (rules && (rules !== _rules || _settings !== settings)) {
            _rules = rules;
            _settings = settings;
            set(build_layout(rules, artifacts, settings));
        }
    });
}
