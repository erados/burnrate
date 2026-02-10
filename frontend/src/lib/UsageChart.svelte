<script lang="ts">
  export let history: Array<{
    timestamp: string;
    session_percent: number;
    weekly_all_percent: number;
    weekly_sonnet_percent: number;
  }> = [];

  const W = 380;
  const H = 180;
  const PAD = { top: 10, right: 10, bottom: 30, left: 35 };
  const cw = W - PAD.left - PAD.right;
  const ch = H - PAD.top - PAD.bottom;

  const lines = [
    { key: 'session_percent', color: '#4ade80', label: 'Session' },
    { key: 'weekly_all_percent', color: '#818cf8', label: 'Weekly All' },
    { key: 'weekly_sonnet_percent', color: '#38bdf8', label: 'Weekly Sonnet' },
  ] as const;

  type Entry = typeof history[0];

  let tooltip: { x: number; y: number; entry: Entry } | null = null;

  $: sorted = [...history].sort((a, b) => a.timestamp.localeCompare(b.timestamp));
  $: times = sorted.map(e => new Date(e.timestamp).getTime());
  $: tMin = times.length ? Math.min(...times) : 0;
  $: tMax = times.length ? Math.max(...times) : 1;
  $: tRange = tMax - tMin || 1;

  function x(t: number) { return PAD.left + ((t - tMin) / tRange) * cw; }
  function y(v: number) { return PAD.top + ch - (v / 100) * ch; }

  function pathFor(key: string) {
    if (sorted.length < 2) return '';
    return sorted.map((e, i) => {
      const px = x(times[i]);
      const py = y((e as any)[key]);
      return `${i === 0 ? 'M' : 'L'}${px.toFixed(1)},${py.toFixed(1)}`;
    }).join(' ');
  }

  function handleMouse(e: MouseEvent) {
    if (!sorted.length) { tooltip = null; return; }
    const svg = (e.currentTarget as SVGSVGElement).getBoundingClientRect();
    const mx = e.clientX - svg.left;
    // find closest point
    let best = 0, bestDist = Infinity;
    for (let i = 0; i < sorted.length; i++) {
      const d = Math.abs(x(times[i]) - mx);
      if (d < bestDist) { bestDist = d; best = i; }
    }
    if (bestDist < 30) {
      tooltip = { x: x(times[best]), y: 20, entry: sorted[best] };
    } else {
      tooltip = null;
    }
  }

  $: yTicks = [0, 25, 50, 75, 100];

  $: xLabels = (() => {
    if (sorted.length < 2) return [];
    const count = Math.min(5, sorted.length);
    const step = Math.max(1, Math.floor((sorted.length - 1) / (count - 1)));
    const labels: Array<{ x: number; text: string }> = [];
    for (let i = 0; i < sorted.length; i += step) {
      const d = new Date(sorted[i].timestamp);
      const text = `${d.getMonth()+1}/${d.getDate()} ${d.getHours().toString().padStart(2,'0')}:${d.getMinutes().toString().padStart(2,'0')}`;
      labels.push({ x: x(times[i]), text });
    }
    return labels;
  })();
</script>

{#if sorted.length >= 2}
  <div class="legend">
    {#each lines as line}
      <span class="legend-item">
        <span class="dot" style="background:{line.color}"></span>
        {line.label}
      </span>
    {/each}
  </div>

  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <svg width="100%" viewBox="0 0 {W} {H}" on:mousemove={handleMouse} on:mouseleave={() => tooltip = null}>
    <!-- Grid -->
    {#each yTicks as tick}
      <line x1={PAD.left} y1={y(tick)} x2={W - PAD.right} y2={y(tick)} stroke="#2a2a4a" stroke-width="0.5" />
      <text x={PAD.left - 4} y={y(tick) + 3} text-anchor="end" fill="#6a6a8a" font-size="9">{tick}%</text>
    {/each}

    <!-- X labels -->
    {#each xLabels as label}
      <text x={label.x} y={H - 4} text-anchor="middle" fill="#6a6a8a" font-size="8">{label.text}</text>
    {/each}

    <!-- Lines -->
    {#each lines as line}
      <path d={pathFor(line.key)} fill="none" stroke={line.color} stroke-width="1.5" stroke-linejoin="round" />
    {/each}

    <!-- Tooltip -->
    {#if tooltip}
      <line x1={tooltip.x} y1={PAD.top} x2={tooltip.x} y2={PAD.top + ch} stroke="#4a4a6a" stroke-width="0.5" stroke-dasharray="3,3" />
      <foreignObject x={Math.min(tooltip.x + 5, W - 120)} y={tooltip.y} width="115" height="60">
        <div class="tip" xmlns="http://www.w3.org/1999/xhtml">
          <div style="font-size:8px;color:#6a6a8a;margin-bottom:2px">{new Date(tooltip.entry.timestamp).toLocaleTimeString()}</div>
          <div><span class="dot" style="background:#4ade80"></span> {tooltip.entry.session_percent.toFixed(0)}%</div>
          <div><span class="dot" style="background:#818cf8"></span> {tooltip.entry.weekly_all_percent.toFixed(0)}%</div>
          <div><span class="dot" style="background:#38bdf8"></span> {tooltip.entry.weekly_sonnet_percent.toFixed(0)}%</div>
        </div>
      </foreignObject>
    {/if}
  </svg>
{:else}
  <div class="empty">Not enough data yet — chart appears after 2+ data points</div>
{/if}

<style>
  .legend {
    display: flex;
    gap: 12px;
    margin-bottom: 6px;
    justify-content: center;
  }
  .legend-item {
    font-size: 10px;
    color: #8a8aaa;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  svg {
    display: block;
  }
  .tip {
    background: #1a1a2e;
    border: 1px solid #2a2a4a;
    border-radius: 4px;
    padding: 4px 6px;
    font-size: 9px;
    color: #ccc;
    line-height: 1.4;
  }
  .empty {
    color: #5a5a7a;
    font-size: 11px;
    text-align: center;
    padding: 20px 0;
  }
</style>
