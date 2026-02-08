<script lang="ts">
  import ProgressBar from './ProgressBar.svelte';

  export let usage: {
    today_messages: number;
    today_tool_calls: number;
    today_sessions: number;
    today_tokens: number;
    opus_tokens: number;
    sonnet_tokens: number;
    weekly_daily: number[];
    weekly_messages: number;
    usage_percent: number;
    last5h_tokens: number;
  };

  $: tokensK = (usage.today_tokens / 1000).toFixed(1);
  $: opusK = (usage.opus_tokens / 1000).toFixed(1);
  $: sonnetK = (usage.sonnet_tokens / 1000).toFixed(1);
  $: last5hK = (usage.last5h_tokens / 1000).toFixed(1);
  $: weeklyTotalK = (usage.weekly_daily.reduce((a, b) => a + b, 0) / 1000).toFixed(0);
  $: sparkMax = Math.max(...usage.weekly_daily, 1);
  $: usageColor = usage.usage_percent >= 80 ? '#ef4444' : usage.usage_percent >= 50 ? '#f59e0b' : '#4ade80';

  const dayLabels = ['M', 'T', 'W', 'T', 'F', 'S', 'S'];
</script>

<div class="grid">
  <!-- Today's Activity -->
  <section class="card">
    <h2>⚡ Today's Activity</h2>
    <div class="big-num">{usage.today_messages}<span class="unit">msg</span></div>
    <div class="stat-row">
      <span>🔧 {usage.today_tool_calls} tools</span>
      <span>📂 {usage.today_sessions} sessions</span>
    </div>
  </section>

  <!-- Tokens Today -->
  <section class="card">
    <h2>📊 Tokens Today</h2>
    <div class="big-num">{tokensK}<span class="unit">k</span></div>
    <div class="model-bar">
      {#if usage.opus_tokens > 0}
        <div class="bar-segment opus" style="flex: {usage.opus_tokens}" title="Opus: {opusK}k"></div>
      {/if}
      {#if usage.sonnet_tokens > 0}
        <div class="bar-segment sonnet" style="flex: {usage.sonnet_tokens}" title="Sonnet: {sonnetK}k"></div>
      {/if}
      {#if usage.opus_tokens === 0 && usage.sonnet_tokens === 0}
        <div class="bar-segment empty"></div>
      {/if}
    </div>
    <div class="stat-row legend">
      <span><i class="dot opus"></i>Opus {opusK}k</span>
      <span><i class="dot sonnet"></i>Sonnet {sonnetK}k</span>
    </div>
  </section>

  <!-- Weekly Trend -->
  <section class="card">
    <h2>📅 Weekly Trend</h2>
    <div class="sparkline">
      {#each usage.weekly_daily as val, i}
        <div class="spark-col">
          <div class="spark-bar" style="height: {Math.max((val / sparkMax) * 40, 2)}px"></div>
          <span class="spark-label">{dayLabels[i]}</span>
        </div>
      {/each}
    </div>
    <div class="stat-row">
      <span>{weeklyTotalK}k tokens</span>
      <span>{usage.weekly_messages} msg</span>
    </div>
  </section>

  <!-- Estimated Usage -->
  <section class="card">
    <h2>💰 Estimated Usage</h2>
    <div class="big-num" style="color: {usageColor}">{usage.usage_percent.toFixed(0)}<span class="unit">%</span></div>
    <ProgressBar value={usage.usage_percent} color={usageColor} warningAt={50} dangerAt={80} />
    <div class="stat-row">
      <span>{last5hK}k / 5h window</span>
      <span class="estimated-label">⚠️ Estimated</span>
    </div>
  </section>
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .card {
    background: #16162a;
    border: 1px solid #2a2a4a;
    border-radius: 10px;
    padding: 12px;
  }

  h2 {
    font-size: 11px;
    margin: 0 0 4px 0;
    font-weight: 600;
    color: #8a8aaa;
  }

  .big-num {
    font-size: 24px;
    font-weight: 700;
    margin-bottom: 6px;
  }

  .big-num .unit {
    font-size: 13px;
    font-weight: 400;
    color: #6a6a8a;
    margin-left: 2px;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: #6a6a8a;
    margin-top: 4px;
  }

  .model-bar {
    display: flex;
    height: 6px;
    border-radius: 3px;
    overflow: hidden;
    background: #2a2a4a;
    margin-bottom: 4px;
  }

  .bar-segment {
    min-width: 2px;
    transition: flex 0.3s;
  }

  .bar-segment.opus { background: #818cf8; }
  .bar-segment.sonnet { background: #38bdf8; }
  .bar-segment.empty { flex: 1; }

  .legend {
    font-size: 9px;
  }

  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    margin-right: 3px;
    vertical-align: middle;
  }

  .dot.opus { background: #818cf8; }
  .dot.sonnet { background: #38bdf8; }

  .sparkline {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    height: 50px;
    margin-bottom: 4px;
  }

  .spark-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
  }

  .spark-bar {
    width: 100%;
    background: #818cf8;
    border-radius: 2px;
    min-height: 2px;
    transition: height 0.3s;
  }

  .spark-label {
    font-size: 8px;
    color: #5a5a7a;
    margin-top: 2px;
  }

  .estimated-label {
    color: #f59e0b;
    font-size: 9px;
  }
</style>
