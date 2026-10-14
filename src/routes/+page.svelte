<script>
  // @ts-nocheck
  import { onMount, onDestroy } from 'svelte';
  import { fade, fly, scale } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-shell';
  import Gauge from '$lib/Gauge.svelte';
  import Chart from '$lib/Chart.svelte';
  import { t, VERSION } from '$lib/i18n.js';

  import {
      LayoutDashboard, ListTree, Monitor, Wrench, Settings, Coffee, Bug, X,
      Cpu, MemoryStick, HardDrive, Network, Activity, HeartPulse, ExternalLink, Clock, Layers,
      Speaker, Usb, Keyboard, Mouse, Camera, Globe, Terminal, Gamepad2, MessageCircle, AppWindow, Music,
      Search, ArrowDown, ArrowUp, Copy, Thermometer, Zap, Fan, CircuitBoard, Bluetooth, Trophy, Play,
      LoaderCircle, TriangleAlert, CircleCheck, Skull, Wifi, Folder, Image
  } from 'lucide-svelte';

  // --- ÉTAT GLOBAL ---
  const HISTORY = 60;
  let activeTab = 'dashboard';
  let showSettings = false;
  let settingsTab = 'appearance';
  let staticSpecs = null;
  let lang = 'fr';

  let realtimeData = {
      cpu_usage: 0, cpu_cores: [], cpu_freq: 0, ram_used: 0, ram_total: 1, swap_used: 0, swap_total: 0,
      net_down: 0, net_up: 0, net_total_down: 0, net_total_up: 0, disk_read: 0, disk_write: 0,
      uptime: 0, process_count: 0, processes: []
  };
  let gpu = null; // stats live NVIDIA, null si indisponible
  let hist = { cpu: [], ram: [], gpu: [], down: [], up: [], read: [], write: [] };
  let recentApps = null;

  // Préférences
  let prefs = { closeToTray: false, thresholds: { cpu: 90, ram: 90, gpuTemp: 85 } };

  // Outils
  let bench = null;
  let benchHistory = [];
  let isBenchmarking = false;
  let pingResults = null;
  let isPinging = false;

  // Processus
  let procSearch = '';
  let procSort = 'cpu';
  let procSortDesc = true;
  let confirmPid = null;
  let confirmTimer = null;

  let toast = null;
  let toastTimer = null;
  let cleanups = [];

  $: L = t[lang];

  const tabs = [
      { id: 'dashboard', icon: LayoutDashboard },
      { id: 'processes', icon: ListTree },
      { id: 'hardware', icon: Monitor },
      { id: 'tools', icon: Wrench },
  ];

  // --- CALCULS ---
  const avg = (arr, n = arr.length) => { const s = arr.slice(-n); return s.length ? s.reduce((a, b) => a + b, 0) / s.length : 0; };
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));

  $: ramPercent = (realtimeData.ram_used / realtimeData.ram_total) * 100;
  $: swapPercent = realtimeData.swap_total ? (realtimeData.swap_used / realtimeData.swap_total) * 100 : 0;
  $: cpuSmooth = avg(hist.cpu, 5);

  // Santé : 100 au repos, pénalités seulement au-delà d'une charge "normale"
  $: healthScore = Math.round(clamp(100
      - Math.max(0, cpuSmooth - 20) * 0.6
      - Math.max(0, ramPercent - 50) * 0.9
      - (gpu && gpu.temp > 70 ? (gpu.temp - 70) * 1.5 : 0), 0, 100));
  $: healthColor = healthScore > 80 ? 'var(--ok)' : healthScore > 50 ? 'var(--warn)' : 'var(--bad)';
  $: healthLabel = healthScore > 80 ? L.health_great : healthScore > 50 ? L.health_ok : L.health_bad;

  $: alerts = [
      cpuSmooth > prefs.thresholds.cpu && hist.cpu.length >= 5 && L.alert_cpu,
      ramPercent > prefs.thresholds.ram && L.alert_ram,
      gpu && gpu.temp > prefs.thresholds.gpuTemp && `${L.alert_gpu} (${gpu.temp}°C)`,
  ].filter(Boolean);

  const level = (v) => v < 60 ? 'var(--ok)' : v < 85 ? 'var(--warn)' : 'var(--bad)';

  // --- MATÉRIEL ---
  const VIRTUAL = /virtual|parsec|citrix|remote|rdp|meta |basic display|mirage|indirect|idd/i;
  const asList = (x) => (x == null ? [] : [].concat(x));
  $: hw = staticSpecs?.hardware || {};
  $: gpus = asList(hw.gpus).filter(g => g?.Name && !VIRTUAL.test(g.Name));
  $: mainGpu = gpus.find(g => /nvidia|amd|radeon|rtx|gtx|arc/i.test(g.Name)) || gpus[0] || null;
  $: gpuName = gpu?.name || mainGpu?.Name || '—';
  $: board = asList(hw.board)[0];
  $: bios = asList(hw.bios)[0];
  $: ramModules = asList(hw.ram);
  $: systemDisk = staticSpecs?.disks.find(d => d.mount.startsWith('C:')) || staticSpecs?.disks[0];

  // Périphériques dédupliqués et groupés
  $: peripherals = (() => {
      const map = new Map();
      for (const p of asList(hw.peripherals)) {
          if (!p?.FriendlyName || /composite|root hub/i.test(p.FriendlyName)) continue;
          const key = p.Class + '|' + p.FriendlyName;
          map.set(key, { name: p.FriendlyName, kind: p.Class, count: (map.get(key)?.count || 0) + 1 });
      }
      return [...map.values()];
  })();
  $: periphGroups = [
      { label: L.cat_monitors, items: peripherals.filter(p => p.kind === 'Monitor') },
      { label: L.cat_audio, items: peripherals.filter(p => p.kind === 'AudioEndpoint') },
      { label: L.cat_input, items: peripherals.filter(p => p.kind === 'Keyboard' || p.kind === 'Mouse') },
      { label: L.cat_other, items: peripherals.filter(p => !['Monitor', 'AudioEndpoint', 'Keyboard', 'Mouse'].includes(p.kind)) },
  ].filter(g => g.items.length);

  const periphIcon = (kind) => ({ Monitor, AudioEndpoint: Speaker, Keyboard, Mouse, Camera, Image: Camera, Bluetooth })[kind] || Usb;

  // --- PROCESSUS ---
  $: procList = realtimeData.processes
      .filter(p => !procSearch || p.name.toLowerCase().includes(procSearch.toLowerCase()) || String(p.pid).includes(procSearch))
      .sort((a, b) => {
          const r = procSort === 'name' ? a.name.localeCompare(b.name) : (a[procSort] - b[procSort]);
          return procSortDesc ? -r : r;
      });

  function sortBy(key) {
      if (procSort === key) procSortDesc = !procSortDesc;
      else { procSort = key; procSortDesc = key !== 'name'; }
  }

  async function killProc(p) {
      if (confirmPid !== p.pid) {
          confirmPid = p.pid;
          clearTimeout(confirmTimer);
          confirmTimer = setTimeout(() => confirmPid = null, 3000);
          return;
      }
      confirmPid = null;
      try {
          await invoke('kill_process', { pid: p.pid });
          showToast(`${L.killed} : ${p.name}`);
      } catch (e) {
          showToast(`${L.kill_failed} : ${e}`, 'bad');
      }
  }

  // --- FORMATAGE ---
  function formatBytes(bytes, digits = 1) {
      const units = ['B', 'KB', 'MB', 'GB', 'TB'];
      let i = 0, v = bytes || 0;
      while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
      return `${v.toFixed(i === 0 ? 0 : digits)} ${units[i]}`;
  }
  const formatSpeed = (b) => formatBytes(b) + '/s';
  function formatUptime(s) {
      const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
      return d ? `${d}${lang === 'fr' ? 'j' : 'd'} ${h}h ${m}m` : `${h}h ${m}m`;
  }
  const ms = (v) => v == null ? '—' : `${v.toFixed(v < 10 ? 1 : 0)} ms`;

  function getAppIcon(name) {
      const n = name.toLowerCase();
      if (/chrome|firefox|edge|brave|opera|vivaldi/.test(n)) return Globe;
      if (/code|studio|terminal|powershell|cmd|idea|cursor/.test(n)) return Terminal;
      if (/discord|whatsapp|telegram|teams|slack|signal/.test(n)) return MessageCircle;
      if (/spotify|music|deezer|vlc/.test(n)) return Music;
      if (/steam|game|play|epic|riot|battle|ubisoft|gog/.test(n)) return Gamepad2;
      if (/explorer/.test(n)) return Folder;
      return AppWindow;
  }

  // --- THEMES ---
  let currentTheme = { accent: '#5865F2', bg_from: '#121212', bg_to: '#0a0a0a', card_bg: '#1e1e1e', text: '#f3f4f6' };

  const themes = [
    { name: 'Discord Dark', accent: '#5865F2', bg_from: '#202225', bg_to: '#2f3136', card_bg: '#36393f', text: '#dcddde' },
    { name: 'Midnight Blue', accent: '#3b82f6', bg_from: '#0f172a', bg_to: '#1e293b', card_bg: '#1e293b', text: '#e2e8f0' },
    { name: 'Cyberpunk', accent: '#facc15', bg_from: '#18181b', bg_to: '#27272a', card_bg: '#09090b', text: '#f4f4f5' },
    { name: 'Forest', accent: '#10b981', bg_from: '#064e3b', bg_to: '#022c22', card_bg: '#115e59', text: '#d1fae5' },
    { name: 'Blood', accent: '#ef4444', bg_from: '#450a0a', bg_to: '#7f1d1d', card_bg: '#2c0404', text: '#fee2e2' },
    { name: 'Royal', accent: '#a855f7', bg_from: '#2e1065', bg_to: '#4c1d95', card_bg: '#5b21b6', text: '#ede9fe' },
    { name: 'Sunset', accent: '#f97316', bg_from: '#431407', bg_to: '#7c2d12', card_bg: '#9a3412', text: '#ffedd5' },
    { name: 'Steel', accent: '#9ca3af', bg_from: '#111827', bg_to: '#374151', card_bg: '#1f2937', text: '#f9fafb' },
    { name: 'Neon Tokyo', accent: '#ec4899', bg_from: '#0b0217', bg_to: '#1a0633', card_bg: '#1e0b3a', text: '#fce7f3' },
    { name: 'Matrix', accent: '#22c55e', bg_from: '#020a04', bg_to: '#04140a', card_bg: '#071d0f', text: '#dcfce7' },
    { name: 'Arctic', accent: '#06b6d4', bg_from: '#0c1a24', bg_to: '#06121a', card_bg: '#10293a', text: '#e0f7fa' },
    { name: 'OLED', accent: '#ffffff', bg_from: '#000000', bg_to: '#000000', card_bg: '#0a0a0a', text: '#fafafa' },
  ];

  function applyTheme() {
      const root = document.documentElement;
      root.style.setProperty('--accent-color', currentTheme.accent);
      root.style.setProperty('--bg-from', currentTheme.bg_from);
      root.style.setProperty('--bg-to', currentTheme.bg_to);
      root.style.setProperty('--card-bg', currentTheme.card_bg);
      root.style.setProperty('--text-primary', currentTheme.text);
  }

  function saveTheme() {
      applyTheme();
      store('hw_theme', currentTheme);
  }

  function selectTheme(th) {
      currentTheme = { ...th };
      saveTheme();
  }

  function toggleLang() {
      lang = lang === 'fr' ? 'en' : 'fr';
      localStorage.setItem('hw_lang', lang);
  }

  // --- STOCKAGE LOCAL ---
  function load(key, fallback) {
      try { const v = localStorage.getItem(key); return v ? JSON.parse(v) : fallback; } catch { return fallback; }
  }
  function store(key, value) {
      try { localStorage.setItem(key, JSON.stringify(value)); } catch {}
  }
  $: store('hw_prefs', prefs);

  function showToast(msg, kind = 'ok') {
      toast = { msg, kind };
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => toast = null, 2800);
  }

  // --- INITIALISATION ---
  const push = (arr, v) => [...arr.slice(-(HISTORY - 1)), v];

  onMount(async () => {
    lang = localStorage.getItem('hw_lang') || 'fr';
    currentTheme = load('hw_theme', currentTheme);
    prefs = { ...prefs, ...load('hw_prefs', {}) };
    benchHistory = load('hw_bench', []);
    applyTheme();

    try {
        cleanups.push(await listen('system-update', ({ payload: d }) => {
            realtimeData = d;
            hist = {
                ...hist,
                cpu: push(hist.cpu, d.cpu_usage),
                ram: push(hist.ram, (d.ram_used / d.ram_total) * 100),
                down: push(hist.down, d.net_down),
                up: push(hist.up, d.net_up),
                read: push(hist.read, d.disk_read),
                write: push(hist.write, d.disk_write),
            };
        }));
        cleanups.push(await listen('gpu-update', ({ payload }) => {
            gpu = payload;
            hist = { ...hist, gpu: push(hist.gpu, payload.usage) };
        }));

        const win = getCurrentWindow();
        cleanups.push(await win.onCloseRequested((e) => {
            if (prefs.closeToTray) { e.preventDefault(); win.hide(); }
        }));
    } catch (e) {
        console.error('Erreur initialisation:', e);
    }

    // Specs en arrière-plan : l'UI temps réel s'affiche sans attendre PowerShell
    invoke('get_static_specs').then(s => staticSpecs = s).catch(e => console.error('Specs:', e));
    refreshRecentApps();
    const iv = setInterval(refreshRecentApps, 5000);
    cleanups.push(() => clearInterval(iv));
  });

  onDestroy(() => cleanups.forEach(fn => fn()));

  async function refreshRecentApps() {
      try { recentApps = await invoke('get_recent_apps'); } catch { recentApps = recentApps || []; }
  }

  async function openExternal(url) {
      try { await open(url); } catch (err) { console.error('Erreur lien:', err); }
  }

  async function runBenchmark() {
      if (isBenchmarking) return;
      isBenchmarking = true;
      try {
          bench = await invoke('run_benchmark');
          benchHistory = [{ ...bench, date: Date.now() }, ...benchHistory].slice(0, 8);
          store('hw_bench', benchHistory);
      } catch (e) {
          showToast(String(e), 'bad');
      }
      isBenchmarking = false;
  }
  $: bestSingle = Math.max(1, ...benchHistory.map(b => b.single));
  $: bestMulti = Math.max(1, ...benchHistory.map(b => b.multi));

  async function runPing() {
      if (isPinging) return;
      isPinging = true;
      try { pingResults = await invoke('test_connection'); } catch (e) { showToast(String(e), 'bad'); }
      isPinging = false;
  }
  $: bestPing = pingResults ? Math.min(...pingResults.filter(r => r.avg != null).map(r => r.avg)) : null;
  $: pingQuality = bestPing == null || !isFinite(bestPing) ? null
      : bestPing < 30 ? { label: L.quality_great, color: 'var(--ok)' }
      : bestPing < 80 ? { label: L.quality_ok, color: 'var(--warn)' }
      : { label: L.quality_bad, color: 'var(--bad)' };

  async function copyReport() {
      const s = staticSpecs;
      if (!s) return;
      const lines = [
          `${L.title} ${VERSION} — ${new Date().toLocaleString()}`,
          `${L.hostname}: ${s.host_name}`,
          `${L.os}: ${s.os} (${s.kernel})`,
          `CPU: ${s.cpu_brand} — ${s.cpu_cores} ${L.cores} / ${s.cpu_threads} ${L.threads} @ ${s.cpu_freq} MHz`,
          `GPU: ${gpuName}${mainGpu?.DriverVersion ? ` (${L.driver} ${mainGpu.DriverVersion})` : ''}`,
          `RAM: ${formatBytes(s.total_memory)}${ramModules.length ? ` — ${ramModules.length}x ${formatBytes(ramModules[0].Capacity, 0)} @ ${ramModules[0].Speed} MHz` : ''}`,
          board ? `${L.board}: ${board.Manufacturer} ${board.Product}` : null,
          bios ? `${L.bios}: ${bios.Manufacturer} ${bios.SMBIOSBIOSVersion}` : null,
          ...s.disks.map(d => `${L.disks}: ${d.mount} ${d.name} (${d.kind}, ${d.fs}) — ${formatBytes(d.available_space)} ${L.free} / ${formatBytes(d.total_space)}`),
          benchHistory.length ? `${L.bench_title}: ${L.single} ${bestSingle} / ${L.multi} ${bestMulti}` : null,
      ].filter(Boolean);
      try {
          await navigator.clipboard.writeText(lines.join('\n'));
          showToast(L.copied);
      } catch (e) {
          showToast(String(e), 'bad');
      }
  }

  function onKey(e) {
      if (e.target.tagName === 'INPUT') return;
      if (e.key === 'Escape') showSettings = false;
      const i = Number(e.key) - 1;
      if (tabs[i] && !showSettings) activeTab = tabs[i].id;
  }
</script>

<svelte:window on:keydown={onKey} />

<main class="h-screen w-screen flex text-[var(--text-primary)] overflow-hidden select-none relative"
      style="background: linear-gradient(160deg, var(--bg-from), var(--bg-to));">

    <!-- Lueurs d'ambiance -->
    <div class="pointer-events-none absolute -top-40 -left-40 w-[520px] h-[520px] rounded-full opacity-25 blur-[120px]" style="background: var(--accent-color)"></div>
    <div class="pointer-events-none absolute -bottom-48 right-0 w-[460px] h-[460px] rounded-full opacity-[0.12] blur-[120px]" style="background: var(--accent-color)"></div>

    <!-- SIDEBAR -->
    <aside class="w-64 shrink-0 flex flex-col p-4 gap-1 z-10 border-r border-[var(--border)]" style="background: color-mix(in srgb, var(--bg-to) 55%, transparent); backdrop-filter: blur(20px)">
        <div class="flex items-center gap-3 px-2 py-3 mb-4">
            <div class="w-11 h-11 rounded-xl flex items-center justify-center p-1.5 ring-1 ring-white/10 shadow-lg" style="background: linear-gradient(135deg, var(--accent-soft), #000)">
                <img src="/icon.png" alt="" class="w-full h-full object-contain" />
            </div>
            <div class="leading-tight">
                <div class="font-black tracking-wide text-[15px]">GEEWER'S</div>
                <div class="text-[11px] font-bold tracking-[0.25em] accent">CONTROL ROOM</div>
            </div>
        </div>

        <nav class="flex flex-col gap-1" aria-label="Navigation">
            {#each tabs as tab, i}
                <button on:click={() => activeTab = tab.id}
                    class="group relative flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-semibold transition-all text-left
                        {activeTab === tab.id ? '' : 'dim hover:text-[var(--text-primary)] hover:bg-white/5'}"
                    style={activeTab === tab.id ? 'background: var(--accent-soft); color: var(--text-primary)' : ''}
                    aria-current={activeTab === tab.id ? 'page' : undefined}>
                    {#if activeTab === tab.id}
                        <span class="absolute left-0 top-2 bottom-2 w-1 rounded-full" style="background: var(--accent-color); box-shadow: 0 0 10px var(--accent-color)"></span>
                    {/if}
                    <svelte:component this={tab.icon} size={18} class={activeTab === tab.id ? 'accent' : ''} />
                    <span class="flex-1">{L[tab.id]}</span>
                    {#if tab.id === 'processes'}
                        <span class="text-[10px] mono dim">{realtimeData.process_count}</span>
                    {:else}
                        <kbd class="text-[10px] mono dim opacity-0 group-hover:opacity-100 transition-opacity">{i + 1}</kbd>
                    {/if}
                </button>
            {/each}
        </nav>

        <div class="flex-1"></div>

        <!-- Mini stats -->
        <div class="rounded-xl p-3 mb-2 space-y-3 border border-[var(--border)]" style="background: color-mix(in srgb, var(--card-bg) 50%, transparent)">
            {#each [
                { label: 'CPU', v: realtimeData.cpu_usage },
                { label: 'RAM', v: ramPercent },
                ...(gpu ? [{ label: 'GPU', v: gpu.usage }] : []),
            ] as s}
                <div>
                    <div class="flex justify-between text-[11px] font-bold mb-1"><span class="dim">{s.label}</span><span class="num">{s.v.toFixed(0)}%</span></div>
                    <div class="bar !h-1.5"><div style="width: {s.v}%; background: {level(s.v)}"></div></div>
                </div>
            {/each}
        </div>

        <div class="flex gap-2">
            <button on:click={toggleLang} class="btn-ghost flex-1" title={L.language}>
                {#if lang === 'fr'}
                    <svg class="w-5 h-3.5 rounded-sm" viewBox="0 0 3 2"><path fill="#EC1920" d="M0 0h3v2H0z"/><path fill="#fff" d="M0 0h2v2H0z"/><path fill="#051440" d="M0 0h1v2H0z"/></svg> FR
                {:else}
                    <svg class="w-5 h-3.5 rounded-sm" viewBox="0 0 60 30"><clipPath id="uk"><path d="M0 0v30h60V0z"/></clipPath><path d="M0 0v30h60V0z" fill="#012169"/><path d="M0 0l60 30m0-30L0 30" stroke="#fff" stroke-width="6"/><path d="M0 0l60 30m0-30L0 30" clip-path="url(#uk)" stroke="#C8102E" stroke-width="4"/><path d="M30 0v30M0 15h60" stroke="#fff" stroke-width="10"/><path d="M30 0v30M0 15h60" stroke="#C8102E" stroke-width="6"/></svg> EN
                {/if}
            </button>
            <button on:click={() => showSettings = true} class="btn-ghost" title={L.settings} aria-label={L.settings}>
                <Settings size={18} />
            </button>
        </div>
    </aside>

    <!-- CONTENU -->
    <section class="flex-1 flex flex-col min-w-0 z-10">
        <header class="flex items-center justify-between px-8 pt-7 pb-5 shrink-0 gap-4">
            <div class="min-w-0">
                <h1 class="text-3xl font-black tracking-tight">{L[activeTab]}</h1>
                <p class="text-sm dim mt-0.5 truncate">{activeTab === 'dashboard' ? L.welcome : staticSpecs?.host_name || L.title}</p>
            </div>
            <div class="flex items-center gap-2 shrink-0">
                <div class="hidden lg:flex items-center gap-2 px-3 py-2 rounded-xl text-xs font-bold border border-[var(--border)] dim">
                    <Clock size={14} /> {L.uptime} <span class="num text-[var(--text-primary)]">{formatUptime(realtimeData.uptime)}</span>
                </div>
                {#if alerts.length}
                    <div class="flex items-center gap-2 px-3 py-2 rounded-xl text-xs font-bold animate-pulse" style="background: color-mix(in srgb, var(--bad) 18%, transparent); color: var(--bad); border: 1px solid color-mix(in srgb, var(--bad) 40%, transparent)" role="status">
                        <TriangleAlert size={14} /> {alerts.join(' · ')}
                    </div>
                {:else}
                    <div class="flex items-center gap-2 px-3 py-2 rounded-xl text-xs font-bold" style="background: color-mix(in srgb, var(--ok) 14%, transparent); color: var(--ok)" role="status">
                        <CircleCheck size={14} /> {L.all_good}
                    </div>
                {/if}
            </div>
        </header>

        <div class="flex-1 overflow-y-auto px-8 pb-8">
            {#key activeTab}
            <div in:fly={{ y: 12, duration: 350 }}>

            <!-- ============ DASHBOARD ============ -->
            {#if activeTab === 'dashboard'}
                <div class="grid grid-cols-2 xl:grid-cols-4 gap-4 mb-4">
                    <!-- CPU -->
                    <div class="card flex items-center gap-5">
                        <Gauge value={realtimeData.cpu_usage} size={108}>
                            <span class="text-2xl font-black num">{realtimeData.cpu_usage.toFixed(0)}<span class="text-sm">%</span></span>
                        </Gauge>
                        <div class="min-w-0">
                            <div class="label flex items-center gap-1.5 mb-1"><Cpu size={13} /> {L.cpu}</div>
                            <div class="font-bold truncate text-sm" title={staticSpecs?.cpu_brand}>{staticSpecs?.cpu_brand || '…'}</div>
                            <div class="text-xs dim mt-2 num">{(realtimeData.cpu_freq / 1000).toFixed(2)} GHz</div>
                            <div class="text-xs dim num">{realtimeData.cpu_cores.length} {L.threads}</div>
                        </div>
                    </div>

                    <!-- RAM -->
                    <div class="card flex items-center gap-5">
                        <Gauge value={ramPercent} color="#10b981" size={108}>
                            <span class="text-2xl font-black num">{ramPercent.toFixed(0)}<span class="text-sm">%</span></span>
                        </Gauge>
                        <div class="min-w-0">
                            <div class="label flex items-center gap-1.5 mb-1"><MemoryStick size={13} /> {L.ram}</div>
                            <div class="font-black text-lg num">{formatBytes(realtimeData.ram_used)}</div>
                            <div class="text-xs dim num">/ {formatBytes(realtimeData.ram_total)}</div>
                            <div class="text-xs dim mt-1 num">{L.swap}: {formatBytes(realtimeData.swap_used)}</div>
                        </div>
                    </div>

                    <!-- GPU -->
                    <div class="card flex items-center gap-5">
                        <Gauge value={gpu?.usage ?? 0} color="#a855f7" size={108}>
                            {#if gpu}
                                <span class="text-2xl font-black num">{gpu.usage.toFixed(0)}<span class="text-sm">%</span></span>
                            {:else}
                                <Monitor size={28} class="dim" />
                            {/if}
                        </Gauge>
                        <div class="min-w-0">
                            <div class="label flex items-center gap-1.5 mb-1"><Monitor size={13} /> GPU</div>
                            <div class="font-bold truncate text-sm" title={gpuName}>{gpuName}</div>
                            {#if gpu}
                                <div class="flex gap-3 text-xs mt-2 num">
                                    <span class="flex items-center gap-1" style="color: {level(gpu.temp)}"><Thermometer size={12} />{gpu.temp}°C</span>
                                    {#if gpu.power >= 0}<span class="flex items-center gap-1 dim"><Zap size={12} />{gpu.power.toFixed(0)} W</span>{/if}
                                </div>
                                <div class="text-xs dim num">{L.vram} {formatBytes(gpu.mem_used)} / {formatBytes(gpu.mem_total, 0)}</div>
                            {:else}
                                <div class="text-[11px] dim mt-2">{L.no_gpu_stats}</div>
                            {/if}
                        </div>
                    </div>

                    <!-- SANTÉ -->
                    <div class="card flex items-center gap-5">
                        <Gauge value={healthScore} color={healthColor} size={108}>
                            <span class="text-3xl font-black num" style="color: {healthColor}">{healthScore}</span>
                        </Gauge>
                        <div class="min-w-0">
                            <div class="label flex items-center gap-1.5 mb-1"><HeartPulse size={13} /> {L.health}</div>
                            <div class="font-black text-lg" style="color: {healthColor}">{healthLabel}</div>
                            <div class="text-xs dim mt-1 flex items-center gap-1 num"><Layers size={12} /> {realtimeData.process_count} {L.procs.toLowerCase()}</div>
                        </div>
                    </div>
                </div>

                <div class="grid grid-cols-1 xl:grid-cols-4 gap-4 mb-4">
                    <!-- Historique -->
                    <div class="card xl:col-span-3">
                        <div class="flex items-center justify-between mb-4">
                            <div class="label flex items-center gap-1.5"><Activity size={13} /> {L.history}</div>
                            <div class="flex gap-4 text-xs font-bold">
                                <span class="flex items-center gap-1.5"><i class="w-2.5 h-2.5 rounded-full" style="background: var(--accent-color)"></i>CPU</span>
                                <span class="flex items-center gap-1.5"><i class="w-2.5 h-2.5 rounded-full bg-[#10b981]"></i>RAM</span>
                                {#if gpu}<span class="flex items-center gap-1.5"><i class="w-2.5 h-2.5 rounded-full bg-[#a855f7]"></i>GPU</span>{/if}
                            </div>
                        </div>
                        <Chart grid max={100} height={190} series={[
                            { values: hist.ram, color: '#10b981' },
                            ...(gpu ? [{ values: hist.gpu, color: '#a855f7' }] : []),
                            { values: hist.cpu, color: currentTheme.accent },
                        ]} />
                    </div>

                    <!-- Réseau -->
                    <div class="card flex flex-col">
                        <div class="label flex items-center gap-1.5 mb-4"><Network size={13} /> {L.net}</div>
                        <div class="grid grid-cols-2 gap-3 mb-3">
                            <div>
                                <div class="text-[11px] dim flex items-center gap-1"><ArrowDown size={12} class="text-sky-400" />{L.down}</div>
                                <div class="font-black text-lg num">{formatSpeed(realtimeData.net_down)}</div>
                            </div>
                            <div>
                                <div class="text-[11px] dim flex items-center gap-1"><ArrowUp size={12} class="text-pink-400" />{L.up}</div>
                                <div class="font-black text-lg num">{formatSpeed(realtimeData.net_up)}</div>
                            </div>
                        </div>
                        <div class="flex-1 flex items-end">
                            <Chart height={90} series={[{ values: hist.down, color: '#38bdf8' }, { values: hist.up, color: '#f472b6' }]} />
                        </div>
                        <div class="text-[11px] dim mt-2 num">{L.session_total}: ↓ {formatBytes(realtimeData.net_total_down)} · ↑ {formatBytes(realtimeData.net_total_up)}</div>
                    </div>
                </div>

                <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-4 gap-4 mb-4">
                    <!-- Charge par cœur -->
                    <div class="card xl:col-span-2">
                        <div class="label flex items-center gap-1.5 mb-4"><Cpu size={13} /> {L.cores_load}</div>
                        <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax(52px, 1fr))">
                            {#each realtimeData.cpu_cores as load, i}
                                <div class="rounded-lg h-14 flex flex-col items-center justify-center border border-[var(--border)] transition-colors duration-500"
                                     style="background: color-mix(in srgb, var(--accent-color) {Math.max(4, load)}%, transparent)" title="Thread {i}: {load.toFixed(0)}%">
                                    <span class="text-[10px] dim mono">#{i}</span>
                                    <span class="text-xs font-black num">{load.toFixed(0)}%</span>
                                </div>
                            {/each}
                        </div>
                    </div>

                    <!-- Disque -->
                    <div class="card flex flex-col">
                        <div class="label flex items-center gap-1.5 mb-4"><HardDrive size={13} /> {L.disk_io}</div>
                        <div class="grid grid-cols-2 gap-3 mb-2">
                            <div><div class="text-[11px] dim">{L.read}</div><div class="font-black num">{formatSpeed(realtimeData.disk_read)}</div></div>
                            <div><div class="text-[11px] dim">{L.write}</div><div class="font-black num">{formatSpeed(realtimeData.disk_write)}</div></div>
                        </div>
                        <div class="flex-1 flex items-end">
                            <Chart height={56} series={[{ values: hist.read, color: '#f59e0b' }, { values: hist.write, color: '#ef4444' }]} />
                        </div>
                        {#if systemDisk}
                            <div class="mt-3">
                                <div class="flex justify-between text-[11px] mb-1"><span class="dim">{systemDisk.mount}</span><span class="num">{formatBytes(systemDisk.available_space)} {L.free}</span></div>
                                <div class="bar"><div style="width: {(1 - systemDisk.available_space / systemDisk.total_space) * 100}%; background: #f59e0b"></div></div>
                            </div>
                        {/if}
                    </div>

                    <!-- Top processus -->
                    <div class="card flex flex-col">
                        <div class="flex items-center justify-between mb-3">
                            <div class="label flex items-center gap-1.5"><ListTree size={13} /> {L.top_procs}</div>
                            <button class="text-[11px] font-bold accent hover:underline" on:click={() => activeTab = 'processes'}>{L.see_all} →</button>
                        </div>
                        <div class="space-y-2">
                            {#each realtimeData.processes.slice(0, 6) as p (p.pid)}
                                <div class="flex items-center gap-2 text-xs">
                                    <span class="truncate flex-1 font-semibold" title={p.name}>{p.name}</span>
                                    <div class="bar !w-14 !h-1.5"><div style="width: {Math.min(100, p.cpu * 2)}%; background: var(--accent-color)"></div></div>
                                    <span class="num w-11 text-right font-bold">{p.cpu.toFixed(1)}%</span>
                                </div>
                            {/each}
                        </div>
                    </div>
                </div>

                <!-- Fenêtres actives -->
                <div class="card">
                    <div class="label flex items-center gap-1.5 mb-4"><AppWindow size={13} /> {L.recent_apps}</div>
                    {#if recentApps === null}
                        <p class="text-sm dim italic flex items-center gap-2"><LoaderCircle size={14} class="animate-spin" /> {L.analyzing}</p>
                    {:else}
                        <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-2">
                            {#each recentApps as app}
                                <div class="flex items-center gap-3 p-3 rounded-xl border border-[var(--border)] hover:bg-white/5 transition-colors">
                                    <div class="p-2 rounded-lg accent" style="background: var(--accent-soft)"><svelte:component this={getAppIcon(app.process + ' ' + app.name)} size={16} /></div>
                                    <div class="min-w-0 flex-1">
                                        <div class="text-sm font-bold truncate" title={app.name}>{app.name}</div>
                                        <div class="text-[11px] dim truncate mono">{app.process}.exe · {formatBytes(app.memory)}</div>
                                    </div>
                                    <span class="w-2 h-2 rounded-full shrink-0" style="background: var(--ok); box-shadow: 0 0 6px var(--ok)"></span>
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>

            <!-- ============ PROCESSUS ============ -->
            {:else if activeTab === 'processes'}
                <div class="card !p-0 flex flex-col">
                    <div class="flex items-center gap-3 p-4 border-b border-[var(--border)]">
                        <div class="relative flex-1 max-w-md">
                            <Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 dim" />
                            <input bind:value={procSearch} placeholder={L.search} aria-label={L.search}
                                class="w-full pl-9 pr-3 py-2 rounded-xl text-sm outline-none border border-[var(--border)] focus:border-[var(--accent-color)] transition-colors"
                                style="background: color-mix(in srgb, var(--bg-to) 60%, transparent)" />
                        </div>
                        <span class="text-xs dim">{L.showing_top.replace('{n}', realtimeData.processes.length)} · {realtimeData.process_count} {L.procs.toLowerCase()}</span>
                    </div>
                    <table class="w-full text-sm">
                        <thead>
                            <tr class="label text-left">
                                {#each [['name', L.name], ['pid', 'PID'], ['cpu', 'CPU'], ['memory', L.memory]] as [key, label]}
                                    <th class="px-4 py-3 font-bold {key !== 'name' ? 'text-right' : ''}">
                                        <button class="hover:text-[var(--text-primary)] uppercase tracking-[0.14em] {procSort === key ? 'accent' : ''}" on:click={() => sortBy(key)}>
                                            {label} {procSort === key ? (procSortDesc ? '↓' : '↑') : ''}
                                        </button>
                                    </th>
                                {/each}
                                <th class="px-4 py-3 w-32"></th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each procList as p (p.pid)}
                                <tr class="border-t border-[var(--border)] hover:bg-white/[0.03] transition-colors">
                                    <td class="px-4 py-2.5">
                                        <div class="flex items-center gap-2.5 font-semibold">
                                            <svelte:component this={getAppIcon(p.name)} size={15} class="accent shrink-0" />
                                            <span class="truncate max-w-[360px]">{p.name}</span>
                                        </div>
                                    </td>
                                    <td class="px-4 py-2.5 text-right mono dim">{p.pid}</td>
                                    <td class="px-4 py-2.5">
                                        <div class="flex items-center justify-end gap-2">
                                            <div class="bar !w-20 !h-1.5"><div style="width: {Math.min(100, p.cpu)}%; background: {level(p.cpu * 2)}"></div></div>
                                            <span class="num font-bold w-12 text-right">{p.cpu.toFixed(1)}%</span>
                                        </div>
                                    </td>
                                    <td class="px-4 py-2.5 text-right num font-semibold">{formatBytes(p.memory)}</td>
                                    <td class="px-4 py-2.5 text-right">
                                        <button on:click={() => killProc(p)}
                                            class="inline-flex items-center gap-1 text-xs font-bold px-2.5 py-1 rounded-lg transition-all
                                                {confirmPid === p.pid ? 'bg-red-500 text-white' : 'dim hover:text-red-400 hover:bg-red-500/10'}">
                                            <Skull size={12} /> {confirmPid === p.pid ? L.confirm : L.kill}
                                        </button>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>

            <!-- ============ MATÉRIEL ============ -->
            {:else if activeTab === 'hardware'}
                {#if !staticSpecs}
                    <div class="card flex items-center gap-3 dim"><LoaderCircle size={18} class="animate-spin" /> {L.analyzing}</div>
                {:else}
                    <div class="card !p-7 mb-4 flex items-center gap-6">
                        <div class="absolute inset-0 pointer-events-none" style="background: linear-gradient(90deg, var(--accent-soft), transparent 60%)"></div>
                        <div class="relative p-4 rounded-2xl ring-1 ring-white/10 accent" style="background: rgb(0 0 0 / .3)"><Monitor size={44} /></div>
                        <div class="relative flex-1 min-w-0">
                            <div class="label mb-1">{L.hostname}</div>
                            <div class="text-4xl font-black tracking-wide truncate">{staticSpecs.host_name}</div>
                            <div class="text-sm dim mt-1">{staticSpecs.os} · {L.kernel} {staticSpecs.kernel}</div>
                        </div>
                        <button class="btn relative" on:click={copyReport}><Copy size={16} /> {L.copy_report}</button>
                    </div>

                    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-4">
                        <div class="card">
                            <div class="label flex items-center gap-1.5 mb-2"><Cpu size={13} /> CPU</div>
                            <div class="text-xl font-bold mb-3">{staticSpecs.cpu_brand}</div>
                            <div class="flex flex-wrap gap-2">
                                <span class="chip">{staticSpecs.cpu_cores} {L.cores}</span>
                                <span class="chip">{staticSpecs.cpu_threads} {L.threads}</span>
                                <span class="chip">{(staticSpecs.cpu_freq / 1000).toFixed(2)} GHz</span>
                                {#if staticSpecs.cpu_arch}<span class="chip">{staticSpecs.cpu_arch}</span>{/if}
                            </div>
                        </div>

                        <div class="card">
                            <div class="label flex items-center gap-1.5 mb-2"><Monitor size={13} /> {L.gpu}</div>
                            <div class="text-xl font-bold mb-3">{gpuName}</div>
                            <div class="flex flex-wrap gap-2">
                                {#if gpu}<span class="chip">{L.vram} {formatBytes(gpu.mem_total, 0)}</span>
                                    <span class="chip">{gpu.temp}°C</span>
                                    {#if gpu.fan >= 0}<span class="chip flex items-center gap-1"><Fan size={11} />{gpu.fan}%</span>{/if}
                                {/if}
                                {#if mainGpu?.DriverVersion}<span class="chip">{L.driver} {mainGpu.DriverVersion}</span>{/if}
                                {#if mainGpu?.VideoModeDescription}<span class="chip">{mainGpu.VideoModeDescription.split(' x ').slice(0, 2).join(' × ')}</span>{/if}
                            </div>
                        </div>

                        <div class="card">
                            <div class="label flex items-center gap-1.5 mb-2"><MemoryStick size={13} /> RAM</div>
                            <div class="text-xl font-bold mb-3">{formatBytes(staticSpecs.total_memory)}</div>
                            <div class="space-y-1.5">
                                {#each ramModules as m, i}
                                    <div class="flex justify-between text-xs p-2 rounded-lg border border-[var(--border)]">
                                        <span class="dim mono">DIMM {i + 1} · {m.PartNumber?.trim() || m.Manufacturer}</span>
                                        <span class="font-bold num">{formatBytes(m.Capacity, 0)} @ {m.Speed} MHz</span>
                                    </div>
                                {/each}
                            </div>
                        </div>

                        <div class="card">
                            <div class="label flex items-center gap-1.5 mb-2"><CircuitBoard size={13} /> {L.board}</div>
                            <div class="text-xl font-bold mb-1">{board ? `${board.Manufacturer} ${board.Product}` : '—'}</div>
                            {#if bios}<div class="text-sm dim">{L.bios} : {bios.Manufacturer} · {bios.SMBIOSBIOSVersion}</div>{/if}
                        </div>
                    </div>

                    <div class="card mb-4">
                        <div class="label flex items-center gap-1.5 mb-4"><HardDrive size={13} /> {L.disks}</div>
                        <div class="grid grid-cols-1 lg:grid-cols-2 gap-x-8 gap-y-5">
                            {#each staticSpecs.disks as disk}
                                {@const used = (1 - disk.available_space / disk.total_space) * 100}
                                <div>
                                    <div class="flex justify-between items-end text-sm mb-1.5 gap-2">
                                        <span class="font-bold truncate">{disk.mount} <span class="dim font-normal">{disk.name}</span></span>
                                        <span class="flex gap-1.5 shrink-0"><span class="chip">{disk.kind}</span><span class="chip">{disk.fs}</span></span>
                                    </div>
                                    <div class="bar !h-2.5"><div style="width: {used}%; background: {used > 90 ? 'var(--bad)' : 'var(--accent-color)'}"></div></div>
                                    <div class="text-xs dim mt-1 num">{formatBytes(disk.available_space)} {L.free} / {formatBytes(disk.total_space)}</div>
                                </div>
                            {/each}
                        </div>
                    </div>

                    <div class="card">
                        <div class="label flex items-center gap-1.5 mb-4"><Usb size={13} /> {L.periphs}</div>
                        {#each periphGroups as group}
                            <div class="mb-4 last:mb-0">
                                <h4 class="text-xs font-bold dim mb-2">{group.label}</h4>
                                <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-2">
                                    {#each group.items as p}
                                        <div class="p-2.5 rounded-xl flex items-center gap-3 border border-[var(--border)]">
                                            <svelte:component this={periphIcon(p.kind)} size={16} class="accent shrink-0" />
                                            <span class="text-sm font-semibold truncate flex-1" title={p.name}>{p.name}</span>
                                            {#if p.count > 1}<span class="chip">×{p.count}</span>{/if}
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {:else}
                            <p class="text-sm dim italic">{L.no_periphs}</p>
                        {/each}
                    </div>
                {/if}

            <!-- ============ OUTILS ============ -->
            {:else if activeTab === 'tools'}
                <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
                    <div class="card">
                        <div class="flex items-start justify-between gap-4 mb-6">
                            <div>
                                <h3 class="text-xl font-black flex items-center gap-2"><Activity size={20} class="accent" /> {L.bench_title}</h3>
                                <p class="text-sm dim mt-1">{L.bench_desc}</p>
                            </div>
                            <button class="btn shrink-0" on:click={runBenchmark} disabled={isBenchmarking}>
                                {#if isBenchmarking}<LoaderCircle size={16} class="animate-spin" /> {L.bench_running}{:else}<Play size={16} /> {L.bench_btn}{/if}
                            </button>
                        </div>

                        {#each [['single', L.single, bestSingle], ['multi', L.multi, bestMulti]] as [key, label, best]}
                            <div class="mb-5">
                                <div class="flex justify-between items-end mb-1.5">
                                    <span class="label">{label}{key === 'multi' && bench ? ` · ${bench.threads} ${L.threads}` : ''}</span>
                                    <span class="text-3xl font-black num">{bench ? bench[key].toLocaleString() : '—'} <span class="text-sm dim">pts</span></span>
                                </div>
                                <div class="bar !h-3"><div style="width: {bench ? (bench[key] / best) * 100 : 0}%; background: var(--accent-color); box-shadow: 0 0 12px var(--accent-color)"></div></div>
                                <div class="text-[11px] dim mt-1 flex items-center gap-1 num"><Trophy size={11} /> {L.best}: {benchHistory.length ? best.toLocaleString() : '—'}</div>
                            </div>
                        {/each}

                        {#if benchHistory.length}
                            <div class="label mb-2">{L.last_runs}</div>
                            <div class="space-y-1">
                                {#each benchHistory as b}
                                    <div class="flex justify-between text-xs p-2 rounded-lg border border-[var(--border)] num">
                                        <span class="dim">{new Date(b.date).toLocaleString(lang)}</span>
                                        <span><b>{b.single.toLocaleString()}</b> <span class="dim">/</span> <b>{b.multi.toLocaleString()}</b></span>
                                    </div>
                                {/each}
                            </div>
                        {/if}
                    </div>

                    <div class="card">
                        <div class="flex items-start justify-between gap-4 mb-6">
                            <div>
                                <h3 class="text-xl font-black flex items-center gap-2"><Wifi size={20} class="accent" /> {L.net_title}</h3>
                                <p class="text-sm dim mt-1">{L.net_desc}</p>
                            </div>
                            <button class="btn shrink-0" on:click={runPing} disabled={isPinging}>
                                {#if isPinging}<LoaderCircle size={16} class="animate-spin" />{:else}<Play size={16} />{/if} {L.net_btn}
                            </button>
                        </div>

                        {#if pingQuality}
                            <div class="flex items-center gap-4 p-4 rounded-xl mb-4" style="background: color-mix(in srgb, {pingQuality.color} 12%, transparent)">
                                <span class="text-4xl font-black num" style="color: {pingQuality.color}">{bestPing.toFixed(0)}<span class="text-lg"> ms</span></span>
                                <span class="font-bold" style="color: {pingQuality.color}">{pingQuality.label}</span>
                            </div>
                        {/if}

                        <table class="w-full text-sm">
                            <thead>
                                <tr class="label text-right">
                                    <th class="text-left py-2 font-bold">{L.server}</th><th class="font-bold">{L.avg}</th><th class="font-bold">{L.min}</th><th class="font-bold">{L.max}</th><th class="font-bold">{L.jitter}</th><th class="font-bold">{L.loss}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each pingResults || [{ target: 'Google', host: '8.8.8.8' }, { target: 'Cloudflare', host: '1.1.1.1' }, { target: 'Quad9', host: '9.9.9.9' }] as r}
                                    <tr class="border-t border-[var(--border)] text-right num">
                                        <td class="text-left py-3"><div class="font-bold">{r.target}</div><div class="text-[11px] dim mono">{r.host.split(':')[0]}</div></td>
                                        {#if r.avg === null}
                                            <td colspan="4" class="font-bold" style="color: var(--bad)">{L.offline}</td>
                                        {:else}
                                            <td class="font-black" style={r.avg != null ? `color: ${r.avg < 30 ? 'var(--ok)' : r.avg < 80 ? 'var(--warn)' : 'var(--bad)'}` : ''}>{ms(r.avg)}</td>
                                            <td class="dim">{ms(r.min)}</td><td class="dim">{ms(r.max)}</td><td class="dim">{ms(r.jitter)}</td>
                                        {/if}
                                        <td style={r.loss ? 'color: var(--bad)' : ''}>{r.loss != null ? r.loss + '%' : '—'}</td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>
                </div>
            {/if}
            </div>
            {/key}
        </div>
    </section>

    <!-- TOAST -->
    {#if toast}
        <div transition:fly={{ y: 20, duration: 250 }} role="status"
             class="fixed bottom-6 left-1/2 -translate-x-1/2 z-[200] px-4 py-3 rounded-xl text-sm font-bold shadow-2xl flex items-center gap-2 border"
             style="background: var(--card-bg); border-color: {toast.kind === 'bad' ? 'var(--bad)' : 'var(--ok)'}">
            {#if toast.kind === 'bad'}<TriangleAlert size={16} style="color: var(--bad)" />{:else}<CircleCheck size={16} style="color: var(--ok)" />{/if}
            {toast.msg}
        </div>
    {/if}

    <!-- PARAMÈTRES -->
    {#if showSettings}
        <div class="fixed inset-0 z-[150] flex items-center justify-center p-6 bg-black/70 backdrop-blur-md" transition:fade={{ duration: 150 }}
             on:click|self={() => showSettings = false} role="presentation">
            <div class="w-full max-w-4xl h-[78vh] rounded-3xl shadow-[0_0_60px_rgba(0,0,0,0.6)] border border-[var(--border)] flex overflow-hidden"
                 style="background: var(--card-bg)" transition:scale={{ start: 0.96, duration: 200 }} role="dialog" aria-modal="true" aria-label={L.settings}>

                <div class="w-60 border-r border-[var(--border)] p-5 flex flex-col gap-1" style="background: rgb(0 0 0 / .2)">
                    <h2 class="text-lg font-black mb-5 tracking-wide uppercase px-3">{L.settings}</h2>
                    {#each ['appearance', 'behavior', 'projects', 'about'] as id}
                        <button on:click={() => settingsTab = id}
                            class="text-left px-3 py-2.5 rounded-xl text-sm font-bold transition-all {settingsTab === id ? 'text-white shadow-lg' : 'dim hover:bg-white/5'}"
                            style={settingsTab === id ? 'background: var(--accent-color)' : ''}>
                            {L[id]}
                        </button>
                    {/each}
                    <div class="flex-1"></div>
                    <button on:click={() => showSettings = false} class="btn-ghost justify-start"><X size={16} /> {L.close}</button>
                </div>

                <div class="flex-1 p-8 overflow-y-auto">
                    {#if settingsTab === 'appearance'}
                        <h3 class="text-2xl font-black mb-4">{L.themes}</h3>
                        <div class="grid grid-cols-2 md:grid-cols-3 gap-3 mb-8">
                            {#each themes as th}
                                {@const active = th.accent === currentTheme.accent && th.bg_from === currentTheme.bg_from}
                                <button on:click={() => selectTheme(th)}
                                    class="group relative overflow-hidden rounded-xl border-2 transition-all text-left h-24 hover:scale-[1.03]"
                                    style="border-color: {active ? th.accent : 'transparent'}">
                                    <div class="absolute inset-0" style="background: linear-gradient(135deg, {th.bg_from}, {th.bg_to})"></div>
                                    <div class="absolute right-3 top-3 w-16 h-8 rounded-md" style="background: {th.card_bg}; box-shadow: inset 3px 0 0 {th.accent}"></div>
                                    <div class="absolute inset-0 p-3 flex items-end justify-between">
                                        <span class="font-bold text-sm drop-shadow" style="color: {th.text}">{th.name}</span>
                                        <span class="w-5 h-5 rounded-full border-2 border-white/80" style="background: {th.accent}; box-shadow: 0 0 10px {th.accent}"></span>
                                    </div>
                                </button>
                            {/each}
                        </div>

                        <h3 class="text-2xl font-black mb-4">{L.custom}</h3>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                            {#each [['accent', L.cust_accent], ['card_bg', L.cust_card], ['bg_from', L.cust_bg_top], ['bg_to', L.cust_bg_bot], ['text', L.cust_text]] as [key, label]}
                                <label class="flex items-center gap-3 p-2.5 rounded-xl border border-[var(--border)] cursor-pointer hover:border-[var(--accent-color)] transition-colors">
                                    <input type="color" bind:value={currentTheme[key]} on:input={saveTheme} class="w-10 h-10 rounded-lg cursor-pointer bg-transparent border-none" />
                                    <span class="flex-1">
                                        <span class="block label">{label}</span>
                                        <span class="mono text-sm">{currentTheme[key]}</span>
                                    </span>
                                </label>
                            {/each}
                        </div>

                    {:else if settingsTab === 'behavior'}
                        <h3 class="text-2xl font-black mb-6">{L.behavior}</h3>
                        <label class="flex items-start gap-4 p-4 rounded-xl border border-[var(--border)] cursor-pointer mb-6">
                            <input type="checkbox" bind:checked={prefs.closeToTray} class="sr-only peer" />
                            <span class="mt-0.5 w-11 h-6 rounded-full relative shrink-0 transition-colors peer-focus-visible:ring-2 ring-[var(--accent-color)]"
                                  style="background: {prefs.closeToTray ? 'var(--accent-color)' : 'rgb(255 255 255 / .15)'}">
                                <span class="absolute top-0.5 w-5 h-5 rounded-full bg-white transition-all shadow" style="left: {prefs.closeToTray ? '22px' : '2px'}"></span>
                            </span>
                            <span>
                                <span class="font-bold block">{L.close_to_tray}</span>
                                <span class="text-sm dim">{L.close_to_tray_desc}</span>
                            </span>
                        </label>

                        <h4 class="label mb-3">{L.thresholds}</h4>
                        <div class="space-y-4 mb-6">
                            {#each [['cpu', 'CPU', '%', 50, 100], ['ram', 'RAM', '%', 50, 100], ['gpuTemp', 'GPU', '°C', 60, 100]] as [key, label, unit, lo, hi]}
                                <label class="flex items-center gap-4">
                                    <span class="w-12 font-bold text-sm">{label}</span>
                                    <input type="range" min={lo} max={hi} bind:value={prefs.thresholds[key]} class="flex-1 accent-[var(--accent-color)]" />
                                    <span class="w-14 text-right num font-bold">{prefs.thresholds[key]}{unit}</span>
                                </label>
                            {/each}
                        </div>
                        <p class="text-sm dim flex items-center gap-2"><Keyboard size={14} /> {L.shortcuts}</p>

                    {:else if settingsTab === 'projects'}
                        <h3 class="text-2xl font-black mb-4">{L.projects}</h3>
                        <div class="rounded-2xl border border-[var(--border)] overflow-hidden flex flex-col md:flex-row hover:border-[var(--accent-color)] transition-all">
                            <div class="w-full md:w-48 h-32 bg-black/40">
                                <img src="/gamehub_banner.png" alt="Game Hub" class="w-full h-full object-cover" />
                            </div>
                            <div class="p-6 flex-1 flex flex-col justify-center">
                                <h4 class="text-xl font-bold mb-2">Geewer's Game Hub</h4>
                                <p class="text-sm dim mb-4">{L.gamehub_desc}</p>
                                <button on:click={() => openExternal('https://apps.microsoft.com/detail/9N9CF4NLLQZ7?hl=fr&gl=FR&ocid=pdpshare')} class="btn self-start !text-xs">
                                    {L.store} <ExternalLink size={14} />
                                </button>
                            </div>
                        </div>

                    {:else if settingsTab === 'about'}
                        <div class="flex flex-col items-center justify-center h-full text-center">
                            <div class="w-24 h-24 rounded-3xl flex items-center justify-center shadow-2xl ring-1 ring-white/10 p-4 mb-5" style="background: linear-gradient(135deg, var(--accent-soft), #000)">
                                <img src="/icon.png" alt="" class="w-full h-full object-contain" />
                            </div>
                            <h3 class="text-3xl font-black">{L.title}</h3>
                            <p class="dim mt-2">{L.version} {VERSION}</p>
                            <div class="flex flex-col gap-3 w-full max-w-sm mt-8">
                                <button on:click={() => openExternal('mailto:s99ddhhh9@proton.me?subject=Feedback%20Geewer%20Control%20Room')} class="btn-ghost !py-3.5"><Bug size={18} /> {L.bug}</button>
                                <button on:click={() => openExternal('https://buymeacoffee.com/geewerrr')} class="bg-[#FFDD00] text-black font-black px-6 py-3.5 rounded-xl shadow-lg hover:scale-[1.03] transition-all flex items-center justify-center gap-2"><Coffee size={18} /> {L.coffee}</button>
                            </div>
                        </div>
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</main>
