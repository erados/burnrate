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
  $: monthlyPercent = usage.monthly_limit > 0 ? (usage.monthly_cost / usage.monthly_limit) * 100 : 0;
  $: daysInMonth = new Date(new Date().getFullYear(), new Date().getMonth() + 1, 0).getDate();
  $: daysRemaining = daysInMonth - new Date().getDate();
</script>

<div class="grid">
  <!-- Session -->
  <section class="card">
    <h2>⚡ Session</h2>
    <div class="big-num">{usage.session_percent.toFixed(0)}%</div>
    <ProgressBar value={usage.session_percent} color="#818cf8" warningAt={70} dangerAt={85} />
    <div class="meta">Resets in {sessionResetDisplay}</div>
  </section>

  <!-- Weekly All -->
  <section class="card">
    <h2>📅 Weekly</h2>
    <div class="big-num">{usage.weekly_all_percent.toFixed(0)}%</div>
    <ProgressBar value={usage.weekly_all_percent} color="#38bdf8" warningAt={60} dangerAt={85} />
    <div class="meta">Resets {weeklyResetDisplay} (Sat)</div>
  </section>

  <!-- Sonnet -->
  <section class="card">
    <h2>🎵 Sonnet</h2>
    <div class="big-num">{usage.weekly_sonnet_percent.toFixed(0)}%</div>
    <ProgressBar value={usage.weekly_sonnet_percent} color="#34d399" warningAt={60} dangerAt={85} />
    <div class="meta">Weekly limit</div>
  </section>

  <!-- Monthly -->
  <section class="card">
    <h2>💰 Monthly</h2>
    <div class="big-num">${usage.monthly_cost.toFixed(0)}<span class="limit">/${usage.monthly_limit}</span></div>
    <ProgressBar value={monthlyPercent} color="#fb923c" warningAt={70} dangerAt={90} />
    <div class="meta">${monthlyRemaining.toFixed(0)} left · {daysRemaining}d</div>
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

  .big-num .limit {
    font-size: 13px;
    font-weight: 400;
    color: #6a6a8a;
  }

  .meta {
    font-size: 10px;
    color: #6a6a8a;
    margin-top: 4px;
  }
</style>
