<script lang="ts">
  export let value: number = 0;
  export let max: number = 100;
  export let color: string = '#4ade80';
  export let warningAt: number = 70;
  export let dangerAt: number = 90;

  $: percent = Math.min((value / max) * 100, 100);
  $: barColor = percent >= dangerAt ? '#ef4444' : percent >= warningAt ? '#f59e0b' : color;
</script>

<div class="track">
  <div class="fill" style="width: {percent}%; background: {barColor};"></div>
</div>

<style>
  .track {
    height: 6px;
    background: #2a2a4a;
    border-radius: 3px;
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.5s ease, background 0.3s ease;
  }
</style>
