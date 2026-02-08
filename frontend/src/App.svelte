<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import Dashboard from './lib/Dashboard.svelte';
  import Settings from './lib/Settings.svelte';

  interface UsageData {
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
  }

  let usage: UsageData = {
    session_percent: 0,
    session_reset_minutes: 0,
    weekly_all_percent: 0,
    weekly_sonnet_percent: 0,
    monthly_cost: 0,
    monthly_limit: 0,
    today_messages: 0,
    today_tokens: 0,
    opus_tokens: 0,
    sonnet_tokens: 0,
    web_connected: false,
    last_updated: '',
  };

  let showSettings = false;
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    try {
      usage = await invoke('get_usage');
    } catch (e) {
      console.error('Failed to get usage:', e);
    }

    unlisten = await listen<UsageData>('usage-updated', (event) => {
      usage = event.payload;
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  function toggleSettings() {
    showSettings = !showSettings;
  }
</script>

<main>
  <header>
    <h1>🔥 BurnRate</h1>
    <button class="settings-btn" on:click={toggleSettings}>
      {showSettings ? '← Back' : '⚙️'}
    </button>
  </header>

  {#if showSettings}
    <Settings />
  {:else}
    <Dashboard {usage} />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background: #1a1a2e;
    color: #e0e0e0;
    overflow-y: auto;
  }

  main {
    padding: 16px;
    max-width: 420px;
    margin: 0 auto;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid #2a2a4a;
  }

  h1 {
    font-size: 18px;
    margin: 0;
    font-weight: 600;
  }

  .settings-btn {
    background: none;
    border: 1px solid #3a3a5a;
    color: #e0e0e0;
    padding: 4px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    transition: background 0.2s;
  }

  .settings-btn:hover {
    background: #2a2a4a;
  }
</style>
