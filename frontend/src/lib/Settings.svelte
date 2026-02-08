<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let pollInterval = 60;
  let saved = false;
  let error = '';

  onMount(async () => {
    try {
      const config: any = await invoke('get_config');
      pollInterval = config.poll_interval_secs;
    } catch (e) {
      console.error('Failed to load config:', e);
    }
  });

  async function save() {
    error = '';
    saved = false;
    try {
      await invoke('save_config', {
        config: {
          poll_interval_secs: pollInterval,
        },
      });
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (e: any) {
      error = e.toString();
    }
  }

  async function openLogin() {
    try {
      await invoke('open_claude_login');
    } catch (e) {
      console.error('Failed to open login:', e);
    }
  }

  async function hideScraper() {
    try {
      await invoke('hide_scraper');
    } catch (e) {
      console.error(e);
    }
  }
</script>

<div class="settings">
  <section class="card">
    <h2>🔑 Claude Connection</h2>
    <div class="info">
      Scrapes usage data from <code>claude.ai/settings/usage</code>
    </div>
    <div class="btn-row">
      <button class="action-btn" on:click={openLogin}>Login to Claude</button>
      <button class="action-btn secondary" on:click={hideScraper}>Hide Browser</button>
    </div>
    <div class="info dim">
      Login opens a browser window. After login, it scrapes automatically.
    </div>
  </section>

  <section class="card">
    <h2>⚙️ Preferences</h2>
    <label>
      <span>Poll interval</span>
      <select bind:value={pollInterval}>
        <option value={30}>30s</option>
        <option value={60}>1 min</option>
        <option value={120}>2 min</option>
        <option value={300}>5 min</option>
      </select>
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
    margin: 0 0 8px 0;
    font-weight: 600;
    color: #8a8aaa;
  }

  .info {
    font-size: 12px;
    margin-bottom: 4px;
  }

  .info.dim {
    font-size: 10px;
    color: #6a6a8a;
  }

  code {
    background: #2a2a4a;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
  }

  .btn-row {
    display: flex;
    gap: 8px;
    margin: 8px 0;
  }

  .action-btn {
    flex: 1;
    padding: 8px;
    background: #818cf8;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
  }

  .action-btn:hover {
    background: #6366f1;
  }

  .action-btn.secondary {
    background: #2a2a4a;
    color: #e0e0e0;
  }

  .action-btn.secondary:hover {
    background: #3a3a5a;
  }

  label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
    font-size: 12px;
  }

  select {
    padding: 4px 8px;
    background: #1a1a2e;
    border: 1px solid #3a3a5a;
    border-radius: 5px;
    color: #e0e0e0;
    font-size: 11px;
  }

  select:focus {
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
