<script lang="ts">
  export let label: string;
  export let value: number;
  export let max: number = 100;
  export let suffix: string = '%';
  export let detail: string = '';
  export let color: string = '#4ade80';
  export let warningAt: number = 70;
  export let dangerAt: number = 90;

  $: percent = Math.min((value / max) * 100, 100);
  $: barColor = percent >= dangerAt ? '#ef4444' : percent >= warningAt ? '#f59e0b' : color;
  $: displayValue = suffix === '$' ? `$${value.toFixed(2)}` : `${Math.round(value)}${suffix}`;
</script>

<div class="progress-container">
  <div class="progress-header">
    <span class="label">{label}</span>
    <span class="value">{displayValue}{suffix === '%' ? '' : ` / $${max}`}</span>
  </div>
  <div class="progress-track">
    <div
      class="progress-fill"
      style="width: {percent}%; background: {barColor};"
    ></div>
  </div>
  {#if detail}
    <div class="detail">{detail}</div>
  {/if}
</div>

<style>
  .progress-container {
    margin-bottom: 16px;
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 6px;
    font-size: 13px;
  }

  .label {
    font-weight: 500;
  }

  .value {
    font-variant-numeric: tabular-nums;
    color: #a0a0c0;
  }

  .progress-track {
    height: 8px;
    background: #2a2a4a;
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    border-radius: 4px;
    transition: width 0.5s ease, background 0.3s ease;
  }

  .detail {
    font-size: 11px;
    color: #6a6a8a;
    margin-top: 4px;
  }
</style>
