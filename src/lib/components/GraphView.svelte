<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import * as d3 from "d3";
  import { notesApi } from "$lib/api/notes";
  import { appState, selectNote } from "$lib/stores/app.store";
  import { closeGraphView } from "$lib/stores/graphView.store";
  import type { GraphData, GraphEdge } from "$lib/types";

  let svgEl: SVGSVGElement;
  let loading = $state(true);
  let nodeCount = $state(0);

  type SimNode = d3.SimulationNodeDatum & {
    id: string;
    title: string;
    folderId: string | null;
    degree: number;
  };
  type SimLink = d3.SimulationLinkDatum<SimNode>;

  function idOf(x: string | SimNode): string {
    return typeof x === "object" ? x.id : x;
  }

  async function loadData(): Promise<GraphData> {
    try {
      return await notesApi.getGraphData();
    } catch {
      return clientGraph();
    }
  }

  /** Fallback bez Tauri — buduje graf z notatek z załadowaną treścią. */
  function clientGraph(): GraphData {
    const notes = get(appState).notes;
    const titleToId = new Map<string, string>();
    for (const n of notes) titleToId.set(n.title.trim().toLowerCase(), n.id);
    const nodes = notes.map((n) => ({ id: n.id, title: n.title, folderId: n.folderId }));
    const edges: GraphEdge[] = [];
    const seen = new Set<string>();
    for (const n of notes) {
      if (!n.content) continue;
      for (const m of n.content.matchAll(/\[\[([^\]]+)\]\]/g)) {
        const tid = titleToId.get(m[1].trim().toLowerCase());
        if (tid && tid !== n.id) {
          const k = n.id + ">" + tid;
          if (!seen.has(k)) {
            seen.add(k);
            edges.push({ source: n.id, target: tid });
          }
        }
      }
    }
    return { nodes, edges };
  }

  onMount(() => {
    let simulation: d3.Simulation<SimNode, SimLink> | undefined;
    let destroyed = false;

    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") closeGraphView();
    }
    window.addEventListener("keydown", onKey);

    function draw(data: GraphData) {
      // Mierz z faktycznego SVG (100vw/100vh), z rozsądną podłogą — niektóre
      // osadzone konteksty raportują znikomą szerokość zanim layout się ustali.
      const rect = svgEl.getBoundingClientRect();
      const width = Math.max(rect.width, window.innerWidth, 600);
      const height = Math.max(rect.height, window.innerHeight, 400);

      const degree = new Map<string, number>();
      for (const e of data.edges) {
        degree.set(e.source, (degree.get(e.source) ?? 0) + 1);
        degree.set(e.target, (degree.get(e.target) ?? 0) + 1);
      }

      const nodes: SimNode[] = data.nodes.map((n) => ({
        ...n,
        degree: degree.get(n.id) ?? 0,
      }));
      const links: SimLink[] = data.edges.map((e) => ({
        source: e.source,
        target: e.target,
      }));

      // Sąsiedztwo do podświetlania przy hover.
      const adj = new Map<string, Set<string>>();
      nodes.forEach((n) => adj.set(n.id, new Set()));
      data.edges.forEach((e) => {
        adj.get(e.source)?.add(e.target);
        adj.get(e.target)?.add(e.source);
      });

      const radius = (d: SimNode) => Math.min(6 + d.degree * 1.5, 20);

      const svg = d3.select(svgEl).attr("viewBox", `0 0 ${width} ${height}`);
      svg.selectAll("*").remove();
      const root = svg.append("g");

      const link = root
        .append("g")
        .attr("stroke", "rgba(255,255,255,0.15)")
        .attr("stroke-width", 1)
        .selectAll<SVGLineElement, SimLink>("line")
        .data(links)
        .join("line");

      const nodeG = root
        .append("g")
        .selectAll<SVGGElement, SimNode>("g")
        .data(nodes)
        .join("g")
        .style("cursor", "pointer");

      nodeG
        .append("circle")
        .attr("r", radius)
        .attr("fill", (d) => (d.degree > 0 ? "#5c4ee8" : "rgba(255,255,255,0.3)"));

      const label = nodeG
        .append("text")
        .text((d) => d.title)
        .attr("x", (d) => radius(d) + 4)
        .attr("y", 4)
        .attr("fill", "rgba(255,255,255,0.75)")
        .attr("font-size", "11px")
        .attr("pointer-events", "none")
        .style("opacity", (d) => (d.degree >= 3 ? 1 : 0));

      nodeG
        .on("mouseenter", (_e, d) => {
          const nb = adj.get(d.id) ?? new Set<string>();
          nodeG.style("opacity", (o) => (o.id === d.id || nb.has(o.id) ? 1 : 0.2));
          link
            .style("opacity", (l) =>
              idOf(l.source as string | SimNode) === d.id ||
              idOf(l.target as string | SimNode) === d.id
                ? 1
                : 0.05,
            )
            .attr("stroke-width", (l) =>
              idOf(l.source as string | SimNode) === d.id ||
              idOf(l.target as string | SimNode) === d.id
                ? 1.8
                : 1,
            );
          label.style("opacity", (o) => (o.id === d.id || nb.has(o.id) ? 1 : 0));
        })
        .on("mouseleave", () => {
          nodeG.style("opacity", 1);
          link.style("opacity", 1).attr("stroke-width", 1);
          label.style("opacity", (o) => (o.degree >= 3 ? 1 : 0));
        })
        .on("click", (_e, d) => {
          selectNote(d.id);
          closeGraphView();
        });

      nodeG.call(
        d3
          .drag<SVGGElement, SimNode>()
          .on("start", (event, d) => {
            if (!event.active) simulation?.alphaTarget(0.3).restart();
            d.fx = d.x;
            d.fy = d.y;
          })
          .on("drag", (event, d) => {
            d.fx = event.x;
            d.fy = event.y;
          })
          .on("end", (event, d) => {
            if (!event.active) simulation?.alphaTarget(0);
            d.fx = null;
            d.fy = null;
          }),
      );

      svg.call(
        d3
          .zoom<SVGSVGElement, unknown>()
          .scaleExtent([0.2, 4])
          .on("zoom", (e) => root.attr("transform", e.transform.toString())),
      );

      const ticked = () => {
        link
          .attr("x1", (l) => (l.source as SimNode).x ?? 0)
          .attr("y1", (l) => (l.source as SimNode).y ?? 0)
          .attr("x2", (l) => (l.target as SimNode).x ?? 0)
          .attr("y2", (l) => (l.target as SimNode).y ?? 0);
        nodeG.attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
      };

      simulation = d3
        .forceSimulation<SimNode>(nodes)
        .force("charge", d3.forceManyBody().strength(-160))
        .force(
          "link",
          d3.forceLink<SimNode, SimLink>(links).id((d) => d.id).distance(70),
        )
        .force("center", d3.forceCenter(width / 2, height / 2))
        .force("collide", d3.forceCollide<SimNode>().radius((d) => radius(d) + 6))
        .on("tick", ticked);

      // Rozgrzewka synchroniczna: rozkłada węzły od razu, niezależnie od tego
      // czy środowisko pozwala animować przez requestAnimationFrame. Żywa
      // symulacja (drag, dalsze klatki) działa dalej tam, gdzie rAF jest aktywny.
      simulation.tick(150);
      ticked();
    }

    void (async () => {
      const data = await loadData();
      if (destroyed) return;
      loading = false;
      nodeCount = data.nodes.length;
      draw(data);
    })();

    return () => {
      destroyed = true;
      window.removeEventListener("keydown", onKey);
      simulation?.stop();
    };
  });
</script>

<div class="graph-overlay">
  <div class="graph-toolbar">
    <span class="graph-title">Graph view</span>
    <button class="graph-close" type="button" title="Close (Esc)" aria-label="Close graph view" onclick={closeGraphView}>×</button>
  </div>

  {#if loading}
    <div class="graph-msg">Loading graph…</div>
  {:else if nodeCount === 0}
    <div class="graph-msg">No notes to graph yet.</div>
  {/if}

  <svg bind:this={svgEl} class="graph-svg" aria-label="Notes graph"></svg>
</div>

<style>
  .graph-overlay {
    position: fixed;
    inset: 0;
    z-index: 5000;
    background: rgba(24, 24, 24, 0.97);
  }
  .graph-svg {
    width: 100vw;
    height: 100vh;
    display: block;
  }
  .graph-toolbar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 5001;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    pointer-events: none;
  }
  .graph-title {
    font-size: 12px;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    color: var(--fg-30);
    pointer-events: none;
  }
  .graph-close {
    pointer-events: auto;
    width: 30px;
    height: 30px;
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: var(--bg-nav);
    color: var(--fg-40);
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
  }
  .graph-close:hover {
    color: var(--text-primary);
    border-color: var(--accent-border);
  }
  .graph-msg {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    color: var(--fg-30);
    font-size: 13px;
    z-index: 5001;
  }
</style>
