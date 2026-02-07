<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let apiKey = '';
  let orgId = '';
  let pollInterval = 300;
  let monthlyLimit = 50;
  let sessionThreshold = 80;
  let monthlyThreshold = 90;
  let displayMode = 'all';
  let saved = false;
  let error = '';

  onMount(async () => {
    try {
      const config: any = await invoke('get_config');
      orgId = config.org_id;
      pollInterval = config.poll_interval_secs;
      monthlyLimit = config.monthly_limit;
      sessionThreshold = config.session_alert_threshold;
      monthlyThreshold = config.monthly_alert_threshold;
      displayMode = config.display_mode;
    } catch (e) {
      console.error('Failed to load config:', e);
    }

    try {
      apiKey = await invoke('load_api_key');
    } catch {
      // No key stored yet
    }
  });

  async function save() {
    error = '';
    saved = false;
    try {
      await invoke('save_config', {
        config: {
          api_key: apiKey,
          org_id: orgId,
          poll_interval_secs: pollInterval,
          monthly_limit: monthlyLimit,
          session_alert_threshold: sessionThreshold,
          monthly_alert_threshold: monthlyThreshold,
          display_mode: displayMode,
        },
      });
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (e: any) {
      error = e.toString();
    }
  }
</script>

<div class="settings">
  <section class="card">
    <h2>🔑 API Configuration</h2>
    <label>
      <span>Anthropic API Key</span>
      <input type="password" bind:value={apiKey} placeholder="sk-ant-..." />
    </label>
    <label>
      <span>Organization ID</span>
      <input type="text" bind:value={orgId} placeholder="org-..." />
    </label>
  </section>

  <section class="card">
    <h2>⚙️ Preferences</h2>
    <label>
      <span>Poll interval (seconds)</span>
      <input type="number" bind:value={pollInterval} min="60" max="3600" step="60" />
    </label>
    <label>
      <span>Monthly budget ($)</span>
      <input type="number" bind:value={monthlyLimit} min="1" step="5" />
    </label>
    <label>
      <span>Display mode</span>
      <select bind:value={displayMode}>
        <option value="all">Show all metrics</option>
        <option value="critical">Show most critical only</option>
      </select>
    </label>
  </section>

  <section class="card">
    <h2>🔔 Alert Thresholds</h2>
    <label>
      <span>Session alert at (%)</span>
      <input type="number" bind:value={sessionThreshold} min="50" max="100" />
    </label>
    <label>
      <span>Monthly alert at (%)</span>
      <input type="number" bind:value={monthlyThreshold} min="50" max="100" />
    </label>
  </section>

  <button class="save-btn" on:click={save}>
    {saved ? '✅ Saved!' : 'Save Settings'}
  </button>

  {#if error}
    <div class="error">{error}</div>
  {/if}
</div>

<style>
  .settings {
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

  label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
    font-size: 13px;
  }

  label span {
    flex: 1;
  }

  input, select {
    width: 160px;
    padding: 6px 10px;
    background: #1a1a2e;
    border: 1px solid #3a3a5a;
    border-radius: 6px;
    color: #e0e0e0;
    font-size: 13px;
  }

  input:focus, select:focus {
    outline: none;
    border-color: #818cf8;
  }

  .save-btn {
    width: 100%;
    padding: 10px;
    background: #818cf8;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .save-btn:hover {
    background: #6366f1;
  }

  .error {
    color: #ef4444;
    font-size: 12px;
    text-align: center;
    margin-top: 8px;
  }
</style>
