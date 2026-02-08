<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ProgressBar from './ProgressBar.svelte';

  export let usage: {
    session_percent: number;
    session_reset_minutes: number;
    weekly_all_percent: number;
    weekly_sonnet_percent: number;
    monthly_cost: number;
    monthly_limit: number;
    today_messages: number;
    today_tokens: number;
    opus_tokens: number;
    sonnet_tokens: number;
    web_connected: boolean;
    last_updated: string;
  };

  $: sessionColor = usage.session_percent >= 80 ? '#ef4444' : usage.session_percent >= 50 ? '#f59e0b' : '#4ade80';
  $: weeklyColor = usage.weekly_all_percent >= 80 ? '#ef4444' : usage.weekly_all_percent >= 50 ? '#f59e0b' : '#818cf8';
  $: sonnetColor = usage.weekly_sonnet_percent >= 80 ? '#ef4444' : usage.weekly_sonnet_percent >= 50 ? '#f59e0b' : '#38bdf8';
  $: monthlyPercent = usage.monthly_limit > 0 ? (usage.monthly_cost / usage.monthly_limit) * 100 : 0;
  $: monthlyColor = monthlyPercent >= 80 ? '#ef4444' : monthlyPercent >= 50 ? '#f59e0b' : '#4ade80';

  $: resetDisplay = formatReset(usage.session_reset_minutes);

  function formatReset(minutes: number): string {
    if (minutes <= 0) return '';
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  }

  async function openLogin() {
    try {
      await invoke('open_claude_login');
    } catch (e) {
      console.error('Failed to open login:', e);
    }
  }
</script>

<div class="grid">
  {#if !usage.web_connected}
    <div class="login-banner" on:click={openLogin}>
      🔑 Not connected to Claude — <button class="link-btn" on:click={openLogin}>Login</button>
    </div>
  {:else}
    <div class="connected-banner">
      ✅ Connected {#if usage.last_updated}· Updated {usage.last_updated}{/if}
    </div>
  {/if}

  <!-- Session -->
  <section class="card">
    <h2>⚡ Session</h2>
    {#if usage.web_connected}
      <div class="big-num" style="color: {sessionColor}">
        {usage.session_percent.toFixed(0)}<span class="unit">%</span>
      </div>
      <ProgressBar value={usage.session_percent} color={sessionColor} warningAt={50} dangerAt={80} />
      {#if resetDisplay}
        <div class="stat-row">
          <span>🔄 Reset in {resetDisplay}</span>
        </div>
      {/if}
    {:else}
      <div class="placeholder">Login required</div>
    {/if}
  </section>

  <!-- Weekly -->
  <section class="card">
    <h2>📅 Weekly</h2>
    {#if usage.web_connected}
      <div class="sub-metric">
        <span class="sub-label">All models</span>
        <span class="sub-value" style="color: {weeklyColor}">{usage.weekly_all_percent.toFixed(0)}%</span>
      </div>
      <ProgressBar value={usage.weekly_all_percent} color={weeklyColor} warningAt={50} dangerAt={80} />
      <div class="sub-metric" style="margin-top: 8px;">
        <span class="sub-label">Sonnet</span>
        <span class="sub-value" style="color: {sonnetColor}">{usage.weekly_sonnet_percent.toFixed(0)}%</span>
      </div>
      <ProgressBar value={usage.weekly_sonnet_percent} color={sonnetColor} warningAt={50} dangerAt={80} />
    {:else}
      <div class="placeholder">Login required</div>
    {/if}
  </section>

  <!-- Extra Usage -->
  <section class="card">
    <h2>💰 Extra Usage</h2>
    {#if usage.web_connected && usage.monthly_cost > 0}
      {#if usage.monthly_limit > 0}
        <div class="big-num" style="color: {monthlyColor}">
          ${usage.monthly_cost.toFixed(2)}<span class="unit">/ ${usage.monthly_limit.toFixed(0)}</span>
        </div>
        <ProgressBar value={monthlyPercent} color={monthlyColor} warningAt={50} dangerAt={80} />
      {:else}
        <div class="big-num" style="color: {monthlyColor}">
          ${usage.monthly_cost.toFixed(2)}<span class="unit">used</span>
        </div>
      {/if}
    {:else if usage.web_connected}
      <div class="no-charges">No extra charges</div>
    {:else}
      <div class="placeholder">Login required</div>
    {/if}
  </section>
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .login-banner {
    grid-column: 1 / -1;
    background: #2a1a1a;
    border: 1px solid #4a2a2a;
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 12px;
    color: #f59e0b;
    text-align: center;
    cursor: pointer;
  }

  .login-banner:hover {
    background: #3a2a2a;
  }

  .connected-banner {
    grid-column: 1 / -1;
    background: #1a2a1a;
    border: 1px solid #2a4a2a;
    border-radius: 8px;
    padding: 4px 12px;
    font-size: 10px;
    color: #4ade80;
    text-align: center;
  }

  .link-btn {
    background: none;
    border: none;
    color: #818cf8;
    text-decoration: underline;
    cursor: pointer;
    font-size: 12px;
    padding: 0;
  }

  .card {
    background: #16162a;
    border: 1px solid #2a2a4a;
    border-radius: 10px;
    padding: 12px;
  }

  h2 {
    font-size: 11px;
    margin: 0 0 6px 0;
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

  .sub-metric {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 3px;
  }

  .sub-label {
    font-size: 11px;
    color: #8a8aaa;
  }

  .sub-value {
    font-size: 16px;
    font-weight: 700;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: #6a6a8a;
    margin-top: 4px;
  }

  .no-charges {
    color: #5a5a7a;
    font-size: 13px;
    text-align: center;
    padding: 12px 0;
  }

  .placeholder {
    color: #5a5a7a;
    font-size: 12px;
    text-align: center;
    padding: 16px 0;
  }

</style>
