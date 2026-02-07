<script lang="ts">
  import ProgressBar from './ProgressBar.svelte';

  export let usage: {
    session_percent: number;
    session_reset_minutes: number;
    weekly_all_percent: number;
    weekly_sonnet_percent: number;
    weekly_reset_hours: number;
    monthly_cost: number;
    monthly_limit: number;
  };

  $: sessionResetDisplay = usage.session_reset_minutes >= 60
    ? `${Math.floor(usage.session_reset_minutes / 60)}h ${usage.session_reset_minutes % 60}m`
    : `${usage.session_reset_minutes}m`;

  $: weeklyResetDisplay = usage.weekly_reset_hours >= 24
    ? `${Math.floor(usage.weekly_reset_hours / 24)}d ${usage.weekly_reset_hours % 24}h`
    : `${usage.weekly_reset_hours}h`;

  $: monthlyRemaining = Math.max(usage.monthly_limit - usage.monthly_cost, 0);
  $: daysInMonth = new Date(new Date().getFullYear(), new Date().getMonth() + 1, 0).getDate();
  $: daysRemaining = daysInMonth - new Date().getDate();
</script>

<div class="dashboard">
  <!-- Session -->
  <section class="card">
    <h2>⚡ Session</h2>
    <ProgressBar
      label="Current session usage"
      value={usage.session_percent}
      suffix="%"
      color="#818cf8"
      detail="Resets in {sessionResetDisplay}"
      warningAt={70}
      dangerAt={85}
    />
  </section>

  <!-- Weekly -->
  <section class="card">
    <h2>📅 Weekly</h2>
    <ProgressBar
      label="All models"
      value={usage.weekly_all_percent}
      suffix="%"
      color="#38bdf8"
      warningAt={60}
      dangerAt={85}
    />
    <ProgressBar
      label="Sonnet"
      value={usage.weekly_sonnet_percent}
      suffix="%"
      color="#34d399"
      warningAt={60}
      dangerAt={85}
    />
    <div class="meta">Resets in {weeklyResetDisplay} (Saturday)</div>
  </section>

  <!-- Monthly -->
  <section class="card">
    <h2>💰 Monthly</h2>
    <ProgressBar
      label="API cost"
      value={usage.monthly_cost}
      max={usage.monthly_limit}
      suffix="$"
      color="#fb923c"
      warningAt={70}
      dangerAt={90}
      detail="Remaining ${monthlyRemaining.toFixed(2)} · {daysRemaining} days left"
    />
  </section>
</div>

<style>
  .dashboard {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card {
    background: #16162a;
    border: 1px solid #2a2a4a;
    border-radius: 10px;
    padding: 14px;
  }

  h2 {
    font-size: 14px;
    margin: 0 0 12px 0;
    font-weight: 600;
  }

  .meta {
    font-size: 11px;
    color: #6a6a8a;
    text-align: right;
  }
</style>
