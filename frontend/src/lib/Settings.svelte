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
    } catch {}
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
  <div class="grid">
    <section class="card">
      <h2>🔑 API</h2>
      <label>
        <span>API Key</span>
        <input type="password" bind:value={apiKey} placeholder="sk-ant-..." />
      </label>
      <label>
        <span>Org ID</span>
        <input type="text" bind:value={orgId} placeholder="org-..." />
      </label>
    </section>

    <section class="card">
      <h2>⚙️ Preferences</h2>
      <label>
        <span>Poll (sec)</span>
        <input type="number" bind:value={pollInterval} min="60" max="3600" step="60" />
      </label>
      <label>
        <span>Budget ($)</span>
        <input type="number" bind:value={monthlyLimit} min="1" step="5" />
      </label>
    </section>

    <section class="card">
      <h2>🔔 Alerts</h2>
      <label>
        <span>Session (%)</span>
        <input type="number" bind:value={sessionThreshold} min="50" max="100" />
      </label>
      <label>
        <span>Monthly (%)</span>
        <input type="number" bind:value={monthlyThreshold} min="50" max="100" />
      </label>
    </section>

    <section class="card">
      <h2>🖥️ Display</h2>
      <label>
        <span>Mode</span>
        <select bind:value={displayMode}>
          <option value="all">All metrics</option>
          <option value="critical">Critical only</option>
        </select>
      </label>
    </section>
  </div>

  <button class="save-btn" on:click={save}>
    {saved ? '✅ Saved!' : 'Save'}
  </button>

  {#if error}
    <div class="error">{error}</div>
  {/if}
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .card {
    background: #16162a;
    border: 1px solid #2a2a4a;
    border-radius: 10px;
    padding: 10px;
  }

  h2 {
    font-size: 11px;
    margin: 0 0 8px 0;
    font-weight: 600;
    color: #8a8aaa;
  }

  label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
    font-size: 11px;
  }

  label span {
    flex-shrink: 0;
    margin-right: 6px;
  }

  input, select {
    width: 100px;
    padding: 4px 6px;
    background: #1a1a2e;
    border: 1px solid #3a3a5a;
    border-radius: 5px;
    color: #e0e0e0;
    font-size: 11px;
  }

  input:focus, select:focus {
    outline: none;
    border-color: #818cf8;
  }

  .save-btn {
    width: 100%;
    padding: 8px;
    background: #818cf8;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }

  .save-btn:hover {
    background: #6366f1;
  }

  .error {
    color: #ef4444;
    font-size: 11px;
    text-align: center;
  }
</style>
