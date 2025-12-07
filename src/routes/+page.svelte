<script>
  // @ts-nocheck
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-shell';
  
  // Imports de toutes les icônes nécessaires
  import { 
      LayoutDashboard, Monitor, Settings, Coffee, Bug, X, Languages, 
      Cpu, MemoryStick, HardDrive, Network, Activity, HeartPulse, 
      ExternalLink, Clock, Layers, Speaker, Usb, Keyboard, Mouse, Camera, History,
      Globe, Terminal, Code, Gamepad2, MessageCircle, AppWindow, Music
  } from 'lucide-svelte';

  // --- ÉTAT GLOBAL ---
  let activeTab = 'dashboard';
  let showSettings = false;
  let settingsTab = 'appearance';
  let staticSpecs = null;
  
  // Données temps réel
  let realtimeData = { 
      cpu_usage: 0, 
      ram_used: 0, 
      ram_total: 1, 
      swap_used: 0, 
      net_down: 0, 
      net_up: 0, 
      uptime: 0, 
      process_count: 0 
  };
  
  // Liste des apps récentes
  let recentApps = [];
  
  let unlisten = null;
  let lang = 'fr'; 
  
  // États locaux pour les outils
  let benchmarkScore = null;
  let isBenchmarking = false;
  let pingResult = null;
  let isPinging = false;
  let recentAppsInterval = null;

  // --- CALCULS ---
  $: ramPercent = (realtimeData.ram_used / realtimeData.ram_total) * 100;
  
  // Calcul de la santé (Note sur 100)
  $: healthScore = Math.max(0, Math.round(100 - ((realtimeData.cpu_usage + ramPercent) / 2)));
  
  // Couleur dynamique de la santé
  $: healthColor = healthScore > 80 ? 'text-green-500' : (healthScore > 50 ? 'text-yellow-500' : 'text-red-500');

  // Filtres Périphériques (Réactifs)
  $: periphs_monitors = staticSpecs?.peripherals.filter(p => p.kind.includes('Monitor')) || [];
  $: periphs_audio = staticSpecs?.peripherals.filter(p => p.kind.includes('Audio')) || [];
  $: periphs_input = staticSpecs?.peripherals.filter(p => p.kind.includes('Keyboard') || p.kind.includes('Pointing')) || [];
  $: periphs_other = staticSpecs?.peripherals.filter(p => !p.kind.includes('Monitor') && !p.kind.includes('Audio') && !p.kind.includes('Keyboard') && !p.kind.includes('Pointing')) || [];

  // Fonction pour formater le temps d'activité
  function formatUptime(seconds) {
      const h = Math.floor(seconds / 3600);
      const m = Math.floor((seconds % 3600) / 60);
      return `${h}h ${m}m`;
  }

  // Fonction pour deviner l'icone de l'app
  function getAppIcon(name) {
      const n = name.toLowerCase();
      if (n.includes('chrome') || n.includes('firefox') || n.includes('edge') || n.includes('brave')) return Globe;
      if (n.includes('code') || n.includes('studio') || n.includes('terminal') || n.includes('powershell')) return Terminal;
      if (n.includes('discord') || n.includes('whatsapp') || n.includes('telegram')) return MessageCircle;
      if (n.includes('spotify') || n.includes('music')) return Music;
      if (n.includes('steam') || n.includes('game') || n.includes('play')) return Gamepad2;
      return AppWindow;
  }

  // --- TRADUCTIONS ---
  const t = {
      fr: {
          title: "Geewer's Control Room",
          dashboard: "Tableau de Bord", 
          specs: "Hardware & Outils", 
          cpu: "Processeur", 
          ram: "Mémoire RAM", 
          swap: "Fichier d'échange", 
          net: "Conso. Réseau",
          settings: "Paramètres", 
          appearance: "Apparence", 
          about: "À Propos", 
          projects: "Autres Projets",
          bug: "Signaler un bug", 
          coffee: "M'offrir un café",
          os: "Système", 
          cores: "Cœurs", 
          hostname: "Nom PC", 
          close: "Fermer",
          welcome: "Bienvenue dans la salle de contrôle.", 
          version: "Version 1.0.4",
          themes: "Thèmes Prédéfinis", 
          custom: "Personnalisation Avancée",
          bench_btn: "Lancer", 
          bench_score: "Score Performance",
          ping_btn: "Ping Google", 
          ping_res: "Latence Réseau",
          disks: "Disques & Stockage", 
          gpu: "Carte Graphique", 
          health: "Santé Globale",
          gamehub_desc: "Le launcher ultime pour centraliser Steam, Epic, GOG et plus encore.",
          uptime: "Temps allumé", 
          procs: "Processus", 
          periphs: "Périphériques Connectés",
          recent_apps: "Dernières Activités",
          cat_monitors: "Écrans", cat_audio: "Audio", cat_input: "Entrées", cat_other: "Autres / USB",
          cust_accent: "Couleur Accent", 
          cust_card: "Couleur Cartes", 
          cust_bg_top: "Fond (Haut)", 
          cust_bg_bot: "Fond (Bas)", 
          cust_text: "Couleur Texte",
          
          // Tooltips
          tt_cpu: "Charge actuelle du processeur.",
          tt_ram: "Mémoire vive utilisée.",
          tt_net: "Débit internet utilisé à l'instant T.",
          tt_health: "Score global (100 = Parfait, 0 = Surchartge).",
          tt_bench: "Test de puissance brute du CPU.",
          tt_ping: "Temps de réponse avec les serveurs Google.",
          tt_swap: "Mémoire virtuelle sur disque (ralentit si utilisé).",
          tt_up: "Temps écoulé depuis le dernier démarrage complet.",
          tt_recent: "Applications avec une fenêtre active détectées récemment."
      },
      en: {
          title: "Geewer's Control Room",
          dashboard: "Dashboard", 
          specs: "Components & Drives", 
          cpu: "Processor", 
          ram: "RAM Memory", 
          swap: "Swap File", 
          net: "Network Usage",
          settings: "Settings", 
          appearance: "Appearance", 
          about: "About", 
          projects: "Other Projects",
          bug: "Report a bug", 
          coffee: "Buy me a coffee",
          os: "System", 
          cores: "Cores", 
          hostname: "PC Name", 
          close: "Close",
          welcome: "Welcome to the control room.", 
          version: "Version 1.0.4",
          themes: "Preset Themes", 
          custom: "Advanced Customization",
          bench_btn: "Run", 
          bench_score: "Performance Score",
          ping_btn: "Ping Google", 
          ping_res: "Network Latency",
          disks: "Disks & Storage", 
          gpu: "Graphics Card", 
          health: "System Health",
          gamehub_desc: "The ultimate launcher to centralize Steam, Epic, GOG and more.",
          uptime: "Uptime", 
          procs: "Processes", 
          periphs: "Connected Peripherals",
          recent_apps: "Recent Activities",
          cat_monitors: "Monitors", cat_audio: "Audio", cat_input: "Inputs", cat_other: "Other / USB",
          cust_accent: "Accent Color", 
          cust_card: "Card Color", 
          cust_bg_top: "Background (Top)", 
          cust_bg_bot: "Background (Bottom)", 
          cust_text: "Text Color",

          tt_cpu: "Current processor load.",
          tt_ram: "System memory usage.",
          tt_net: "Current internet bandwidth usage.",
          tt_health: "Global score (100 = Perfect, 0 = Overload).",
          tt_bench: "Raw CPU power test.",
          tt_ping: "Response time with Google servers.",
          tt_swap: "Virtual memory on disk (slower).",
          tt_up: "Time since last full boot.",
          tt_recent: "Recently detected applications with active windows."
      }
  };

  // --- THEMES ---
  let currentTheme = { 
      accent: '#5865F2', 
      bg_from: '#121212', 
      bg_to: '#0a0a0a', 
      card_bg: '#1e1e1e', 
      text: '#f3f4f6' 
  };
  
  const themes = [
    { name: 'Discord Dark', accent: '#5865F2', bg_from: '#202225', bg_to: '#2f3136', card_bg: '#36393f', text: '#dcddde' },
    { name: 'Midnight Blue', accent: '#3b82f6', bg_from: '#0f172a', bg_to: '#1e293b', card_bg: '#1e293b', text: '#e2e8f0' },
    { name: 'Cyberpunk', accent: '#facc15', bg_from: '#18181b', bg_to: '#27272a', card_bg: '#09090b', text: '#f4f4f5' },
    { name: 'Forest', accent: '#10b981', bg_from: '#064e3b', bg_to: '#022c22', card_bg: '#115e59', text: '#d1fae5' },
    { name: 'Blood', accent: '#ef4444', bg_from: '#450a0a', bg_to: '#7f1d1d', card_bg: '#2c0404', text: '#fee2e2' },
    { name: 'Royal', accent: '#a855f7', bg_from: '#2e1065', bg_to: '#4c1d95', card_bg: '#5b21b6', text: '#ede9fe' },
    { name: 'Sunset', accent: '#f97316', bg_from: '#431407', bg_to: '#7c2d12', card_bg: '#9a3412', text: '#ffedd5' },
    { name: 'Steel', accent: '#9ca3af', bg_from: '#111827', bg_to: '#374151', card_bg: '#1f2937', text: '#f9fafb' },
  ];

  // --- INITIALISATION ---
  onMount(async () => {
    try {
        // Charger les specs
        staticSpecs = await invoke('get_static_specs');
        
        // Écouter le backend
        unlisten = await listen('system-update', (event) => {
            realtimeData = event.payload;
        });

        // Charger apps récentes au démarrage
        refreshRecentApps();
        // Et toutes les 5 secondes
        recentAppsInterval = setInterval(refreshRecentApps, 5000);

        // Charger langue et thème
        const savedLang = localStorage.getItem('hw_lang');
        if(savedLang) lang = savedLang;
        
        const savedTheme = localStorage.getItem('hw_theme');
        if(savedTheme) {
            currentTheme = JSON.parse(savedTheme);
            applyTheme();
        } else {
            applyTheme();
        }
    } catch(e) { 
        console.error("Erreur initialisation:", e); 
    }
  });

  onDestroy(() => { 
      if(unlisten) unlisten(); 
      if(recentAppsInterval) clearInterval(recentAppsInterval);
  });

  async function refreshRecentApps() {
      try {
          const apps = await invoke('get_recent_apps');
          recentApps = apps;
      } catch(e) {
          // Silencieux si échec
      }
  }

  // --- FONCTIONS ---
  function applyTheme() {
      const root = document.documentElement;
      root.style.setProperty('--accent-color', currentTheme.accent);
      root.style.setProperty('--bg-from', currentTheme.bg_from);
      root.style.setProperty('--bg-to', currentTheme.bg_to);
      root.style.setProperty('--card-bg', currentTheme.card_bg);
      root.style.setProperty('--text-primary', currentTheme.text);
  }

  function selectTheme(th) {
      currentTheme = th;
      applyTheme();
      localStorage.setItem('hw_theme', JSON.stringify(th));
  }

  function toggleLang() {
      lang = lang === 'fr' ? 'en' : 'fr';
      localStorage.setItem('hw_lang', lang);
  }

  async function openExternal(url) {
      try { 
          await open(url); 
      } catch (err) { 
          console.error("Erreur lien:", err); 
      }
  }

  async function runBenchmark() {
      if(isBenchmarking) return;
      isBenchmarking = true;
      benchmarkScore = "...";
      try {
          const score = await invoke('run_benchmark');
          benchmarkScore = score + " pts";
      } catch(e) { 
          benchmarkScore = "Err"; 
      }
      isBenchmarking = false;
  }

  async function runPing() {
      if(isPinging) return;
      isPinging = true;
      pingResult = "...";
      try {
          const res = await invoke('test_connection');
          pingResult = res;
      } catch(e) { 
          pingResult = "Err"; 
      }
      isPinging = false;
  }

  // Formatage des octets
  const formatBytes = (bytes) => (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
  const formatSpeed = (bytes) => {
      if(bytes > 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB/s';
      if(bytes > 1024) return (bytes / 1024).toFixed(1) + ' KB/s';
      return bytes + ' B/s';
  }
</script>

<main class="h-screen w-screen flex flex-col text-[var(--text-primary)] font-sans overflow-hidden select-none transition-colors duration-500" 
      style="background: linear-gradient(to bottom, var(--bg-from), var(--bg-to));">
    
    <header class="flex flex-col shrink-0 p-6 z-10 relative pb-2">
        <div class="flex items-center justify-between mb-6">
            <div class="flex items-center">
                <div class="w-16 h-16 bg-gradient-to-br from-[#2a2a2a] to-black rounded-2xl flex items-center justify-center mr-5 shadow-2xl ring-1 ring-white/10 p-2">
                    <img src="/icon.png" alt="Logo" class="w-full h-full object-contain drop-shadow-[0_0_5px_rgba(255,255,255,0.2)]" />      
                </div>
                <div>
                    <h1 class="text-4xl font-black tracking-wider text-[var(--text-primary)] uppercase mb-1">{t[lang].title}</h1>
                    <p class="text-sm text-gray-400 font-medium">{t[lang].welcome}</p>
                </div>
            </div>
            
            <div class="flex gap-3">
                <button on:click={toggleLang} class="flex items-center justify-center gap-2 border border-white/10 px-4 py-2 rounded-lg bg-[var(--card-bg)] hover:bg-[#333] transition-all hover:scale-105 shadow-md active:scale-95">
                    {#if lang === 'fr'}
                        <svg class="w-5 h-5 rounded shadow-sm" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 3 2"><path fill="#EC1920" d="M0 0h3v2H0z"/><path fill="#fff" d="M0 0h2v2H0z"/><path fill="#051440" d="M0 0h1v2H0z"/></svg>
                        <span class="text-xs font-bold text-gray-300">FR</span>
                    {:else}
                        <svg class="w-5 h-5 rounded shadow-sm" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 60 30"><clipPath id="a"><path d="M0 0v30h60V0z"/></clipPath><path d="M0 0v30h60V0z" fill="#012169"/><path d="M0 0l60 30m0-30L0 30" stroke="#fff" stroke-width="6"/><path d="M0 0l60 30m0-30L0 30" clip-path="url(#a)" stroke="#C8102E" stroke-width="4"/><path d="M30 0v30M0 15h60" stroke="#fff" stroke-width="10"/><path d="M30 0v30M0 15h60" stroke="#C8102E" stroke-width="6"/></svg>
                        <span class="text-xs font-bold text-gray-300">EN</span>
                    {/if}
                </button>

                <button on:click={() => showSettings = true} class="group bg-[var(--card-bg)] hover:bg-[var(--bg-from)] text-gray-400 hover:text-[var(--accent-color)] border border-white/10 px-3 py-2 rounded-lg transition-all hover:border-[var(--accent-color)] active:scale-95">
                    <Settings size={24} />
                </button>
            </div>
        </div>

        <div class="flex space-x-3 overflow-x-auto pb-2">
            <button on:click={() => activeTab = 'dashboard'} class={`px-5 py-2 rounded-full text-sm transition-all flex items-center font-bold ${activeTab === 'dashboard' ? 'bg-[var(--text-primary)] text-black shadow-lg scale-105' : 'bg-[var(--card-bg)] text-gray-400 border border-white/5 hover:border-white/20 hover:text-white'}`}>
                <LayoutDashboard size={16} class="mr-2"/> {t[lang].dashboard}
            </button>
            <button on:click={() => activeTab = 'specs'} class={`px-5 py-2 rounded-full text-sm transition-all flex items-center font-bold ${activeTab === 'specs' ? 'bg-[var(--text-primary)] text-black shadow-lg scale-105' : 'bg-[var(--card-bg)] text-gray-400 border border-white/5 hover:border-white/20 hover:text-white'}`}>
                <Monitor size={16} class="mr-2"/> {t[lang].specs}
            </button>
        </div>
    </header>

    <div class="flex-1 overflow-y-auto px-6 pb-6 custom-scroll z-0">
        
        {#if activeTab === 'dashboard'}
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 animate-fade-in mb-4">
                
                <div class="group bg-[var(--card-bg)] p-6 rounded-xl border border-white/5 flex flex-col justify-center items-center hover:bg-white/5 transition-colors relative cursor-help">
                    <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-50 pointer-events-none border border-white/20 w-40 text-center shadow-xl">
                        {t[lang].tt_health}
                    </div>
                    <div class="flex items-center gap-2 mb-2 text-gray-400 font-bold uppercase text-xs tracking-widest">
                        <HeartPulse size={16}/> {t[lang].health}
                    </div>
                    <div class={`text-4xl font-black ${healthColor}`}>{healthScore}/100</div>
                </div>

                <div class="group bg-[var(--card-bg)] p-4 rounded-xl border border-white/5 flex flex-col justify-between hover:bg-white/5 transition-colors relative cursor-help">
                    <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-50 pointer-events-none border border-white/20 w-40 text-center shadow-xl">
                        {t[lang].tt_bench}
                    </div>
                    <div class="flex justify-between items-start">
                        <div class="text-xs text-gray-500 uppercase font-bold">{t[lang].bench_score}</div>
                        <Activity size={16} class="text-yellow-500"/>
                    </div>
                    <div class="flex justify-between items-end mt-2">
                        <div class="text-2xl font-black text-white">{benchmarkScore || '---'}</div>
                        <button on:click={runBenchmark} disabled={isBenchmarking} class="bg-yellow-500 text-black px-3 py-1 rounded text-xs font-bold hover:scale-105 transition-transform disabled:opacity-50">
                            {t[lang].bench_btn}
                        </button>
                    </div>
                </div>

                <div class="group bg-[var(--card-bg)] p-4 rounded-xl border border-white/5 flex flex-col justify-between hover:bg-white/5 transition-colors relative cursor-help">
                    <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-50 pointer-events-none border border-white/20 w-40 text-center shadow-xl">
                        {t[lang].tt_ping}
                    </div>
                    <div class="flex justify-between items-start">
                        <div class="text-xs text-gray-500 uppercase font-bold">{t[lang].ping_res}</div>
                        <Network size={16} class="text-blue-500"/>
                    </div>
                    <div class="flex justify-between items-end mt-2">
                        <div class="text-2xl font-black text-white">{pingResult || '---'}</div>
                        <button on:click={runPing} disabled={isPinging} class="bg-blue-500 text-white px-3 py-1 rounded text-xs font-bold hover:scale-105 transition-transform disabled:opacity-50">
                            {t[lang].ping_btn}
                        </button>
                    </div>
                </div>

                <div class="group bg-[var(--card-bg)] p-4 rounded-xl border border-white/5 flex flex-col justify-between hover:bg-white/5 transition-colors relative cursor-help">
                    <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-50 pointer-events-none border border-white/20 w-40 text-center shadow-xl">
                        {t[lang].tt_up}
                    </div>
                    <div class="flex justify-between items-center border-b border-white/5 pb-2">
                         <div class="text-xs text-gray-500 uppercase font-bold">{t[lang].uptime}</div>
                         <div class="font-bold text-white flex items-center">
                             <Clock size={12} class="mr-1"/> {formatUptime(realtimeData.uptime)}
                         </div>
                    </div>
                    <div class="flex justify-between items-center pt-2">
                        <div class="text-xs text-gray-500 uppercase font-bold">{t[lang].procs}</div>
                        <div class="font-bold text-white flex items-center">
                            <Layers size={12} class="mr-1"/> {realtimeData.process_count}
                        </div>
                   </div>
                </div>
            </div>

            <div class="grid grid-cols-1 gap-4 animate-fade-in mb-4">
                 <div class="group bg-[var(--card-bg)] p-6 rounded-xl border border-white/5 hover:bg-white/5 transition-colors relative">
                    <div class="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-50 pointer-events-none border border-white/20 w-auto max-w-[200px] shadow-xl">
                        {t[lang].tt_recent}
                    </div>
                    <h3 class="font-bold text-white mb-4 flex items-center"><History size={20} class="mr-2"/> {t[lang].recent_apps}</h3>
                    
                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                        {#if recentApps.length > 0}
                            {#each recentApps as app}
                                <div class="bg-black/30 p-3 rounded-lg border border-white/5 flex items-center justify-between hover:bg-[var(--accent-color)]/10 transition-colors group/app">
                                    <div class="flex items-center overflow-hidden">
                                        <div class="text-[var(--accent-color)] mr-3">
                                            <svelte:component this={getAppIcon(app.name)} size={20} />
                                        </div>
                                        <span class="text-sm font-bold text-gray-300 truncate group-hover/app:text-white transition-colors">{app.name}</span>
                                    </div>
                                    <div class="w-2 h-2 rounded-full bg-green-500 shadow-[0_0_5px_lime]"></div>
                                </div>
                            {/each}
                        {:else}
                            <p class="text-sm text-gray-500 italic p-2">Analyse en cours...</p>
                        {/if}
                    </div>
                 </div>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 animate-fade-in">
                <div class="group relative bg-[var(--card-bg)] rounded-xl overflow-hidden p-6 border border-white/5 hover:bg-white/5 transition-colors duration-300">
                    <div class="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-20 pointer-events-none border border-white/20 w-auto max-w-[200px] shadow-xl">
                        {t[lang].tt_cpu}
                    </div>
                    <div class="flex justify-between items-center mb-6">
                        <div class="flex items-center gap-3">
                            <div class="p-2 bg-white/5 rounded-lg text-[var(--accent-color)]"><Cpu size={24} /></div>
                            <h3 class="font-bold text-[var(--text-primary)] text-lg">{t[lang].cpu}</h3>
                        </div>
                        <span class="text-3xl font-black text-[var(--accent-color)]">{realtimeData.cpu_usage.toFixed(1)}%</span>
                    </div>
                    <div class="w-full bg-black/40 h-3 rounded-full overflow-hidden mb-2">
                        <div class="h-full bg-[var(--accent-color)] transition-all duration-500 ease-out shadow-[0_0_10px_var(--accent-color)]" style="width: {realtimeData.cpu_usage}%"></div>
                    </div>
                </div>

                <div class="group relative bg-[var(--card-bg)] rounded-xl overflow-hidden p-6 border border-white/5 hover:bg-white/5 transition-colors duration-300">
                    <div class="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-20 pointer-events-none border border-white/20 w-auto max-w-[200px] shadow-xl">
                        {t[lang].tt_ram}
                    </div>
                    <div class="flex justify-between items-center mb-6">
                        <div class="flex items-center gap-3">
                            <div class="p-2 bg-white/5 rounded-lg text-[#10b981]"><MemoryStick size={24} /></div>
                            <h3 class="font-bold text-[var(--text-primary)] text-lg">{t[lang].ram}</h3>
                        </div>
                        <div class="text-right">
                            <span class="text-2xl font-black text-white">{formatBytes(realtimeData.ram_used)}</span>
                            <p class="text-xs text-gray-500">/ {formatBytes(realtimeData.ram_total)}</p>
                        </div>
                    </div>
                    <div class="w-full bg-black/40 h-3 rounded-full overflow-hidden mb-2">
                        <div class="h-full bg-[#10b981] transition-all duration-500 ease-out shadow-[0_0_10px_#10b981]" style="width: {ramPercent}%"></div>
                    </div>
                </div>

                <div class="group relative bg-[var(--card-bg)] rounded-xl overflow-hidden p-6 border border-white/5 hover:bg-white/5 transition-colors duration-300">
                    <div class="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-20 pointer-events-none border border-white/20 w-auto max-w-[200px] shadow-xl">
                        {t[lang].tt_net}
                    </div>
                    <div class="flex justify-between items-center mb-6">
                        <div class="flex items-center gap-3">
                            <div class="p-2 bg-white/5 rounded-lg text-[#3b82f6]"><Network size={24} /></div>
                            <h3 class="font-bold text-[var(--text-primary)] text-lg">{t[lang].net}</h3>
                        </div>
                    </div>
                    <div class="flex justify-between items-end">
                        <div class="text-center w-1/2 border-r border-white/10">
                            <p class="text-xs text-gray-500 uppercase font-bold">Down</p>
                            <span class="text-xl font-black text-white">{formatSpeed(realtimeData.net_down)}</span>
                        </div>
                        <div class="text-center w-1/2">
                            <p class="text-xs text-gray-500 uppercase font-bold">Up</p>
                            <span class="text-xl font-black text-white">{formatSpeed(realtimeData.net_up)}</span>
                        </div>
                    </div>
                </div>

                 <div class="group relative bg-[var(--card-bg)] rounded-xl overflow-hidden p-6 border border-white/5 hover:bg-white/5 transition-colors duration-300">
                    <div class="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity bg-black/90 text-white text-xs p-2 rounded z-20 pointer-events-none border border-white/20 w-auto max-w-[200px] shadow-xl">
                        {t[lang].tt_swap}
                    </div>
                    <div class="flex justify-between items-center mb-6">
                        <div class="flex items-center gap-3">
                            <div class="p-2 bg-white/5 rounded-lg text-[#f59e0b]"><HardDrive size={24} /></div>
                            <h3 class="font-bold text-[var(--text-primary)] text-lg">{t[lang].swap}</h3>
                        </div>
                        <span class="text-2xl font-black text-white">{formatBytes(realtimeData.swap_used)}</span>
                    </div>
                    <div class="w-full bg-black/40 h-1 rounded-full overflow-hidden">
                        <div class="h-full bg-[#f59e0b] w-full opacity-20"></div>
                    </div>
                </div>
            </div>
        {/if}

        {#if activeTab === 'specs' && staticSpecs}
            <div class="grid gap-4 animate-fade-in">
                <div class="bg-[var(--card-bg)] p-8 rounded-2xl border border-white/5 flex items-center gap-6 shadow-lg relative overflow-hidden">
                    <div class="absolute inset-0 bg-gradient-to-r from-[var(--accent-color)]/10 to-transparent"></div>
                    <div class="relative p-4 bg-black/30 rounded-2xl text-[var(--accent-color)] ring-1 ring-white/10"><Monitor size={48}/></div>
                    <div class="relative">
                        <div class="text-xs text-gray-400 uppercase tracking-widest font-bold mb-1">{t[lang].hostname}</div>
                        <div class="text-4xl font-black text-white tracking-wide">{staticSpecs.host_name}</div>
                    </div>
                </div>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div class="bg-[var(--card-bg)] p-6 rounded-xl border border-white/5 hover:bg-white/5 transition-colors">
                        <div class="text-xs text-gray-500 uppercase font-bold tracking-wider mb-2">CPU</div>
                        <div class="text-xl font-bold text-white mb-1">{staticSpecs.cpu_brand}</div>
                        <span class="bg-[var(--accent-color)]/20 text-[var(--accent-color)] text-xs font-bold px-2 py-1 rounded">{staticSpecs.cpu_cores} {t[lang].cores}</span>
                    </div>
                    <div class="bg-[var(--card-bg)] p-6 rounded-xl border border-white/5 hover:bg-white/5 transition-colors">
                        <div class="text-xs text-gray-500 uppercase font-bold tracking-wider mb-2">{t[lang].gpu}</div>
                        <div class="text-xl font-bold text-white mb-1">{staticSpecs.gpu_name}</div>
                    </div>
                    <div class="bg-[var(--card-bg)] p-6 rounded-xl border border-white/5 hover:bg-white/5 transition-colors">
                        <div class="text-xs text-gray-500 uppercase font-bold tracking-wider mb-2">RAM Total</div>
                        <div class="text-xl font-bold text-white">{formatBytes(staticSpecs.total_memory)}</div>
                    </div>
                    <div class="bg-[var(--card-bg)] p-6 rounded-xl border border-white/5 hover:bg-white/5 transition-colors">
                        <div class="text-xs text-gray-500 uppercase font-bold tracking-wider mb-2">{t[lang].os}</div>
                        <div class="text-xl font-bold text-white">{staticSpecs.os}</div>
                    </div>
                </div>

                <div class="bg-[var(--card-bg)] p-6 rounded-xl border border-white/5">
                    <h3 class="font-bold text-white mb-6 flex items-center text-xl"><Usb size={24} class="mr-3"/> {t[lang].periphs}</h3>
                    {#if staticSpecs.peripherals && staticSpecs.peripherals.length > 0}
                        
                        {#if periphs_monitors.length > 0}
                            <div class="mb-4">
                                <h4 class="text-xs font-bold text-gray-500 uppercase mb-2 tracking-wider border-b border-white/10 pb-1">{t[lang].cat_monitors}</h4>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                                    {#each periphs_monitors as p}
                                        <div class="bg-black/20 p-3 rounded-lg flex items-center gap-3 border border-white/5">
                                            <Monitor size={18} class="text-blue-400 shrink-0"/>
                                            <span class="text-sm font-bold text-gray-300 truncate">{p.name}</span>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {/if}

                        {#if periphs_audio.length > 0}
                            <div class="mb-4">
                                <h4 class="text-xs font-bold text-gray-500 uppercase mb-2 tracking-wider border-b border-white/10 pb-1">{t[lang].cat_audio}</h4>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                                    {#each periphs_audio as p}
                                        <div class="bg-black/20 p-3 rounded-lg flex items-center gap-3 border border-white/5">
                                            <Speaker size={18} class="text-green-400 shrink-0"/>
                                            <span class="text-sm font-bold text-gray-300 truncate">{p.name}</span>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {/if}

                        {#if periphs_input.length > 0}
                            <div class="mb-4">
                                <h4 class="text-xs font-bold text-gray-500 uppercase mb-2 tracking-wider border-b border-white/10 pb-1">{t[lang].cat_input}</h4>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                                    {#each periphs_input as p}
                                        <div class="bg-black/20 p-3 rounded-lg flex items-center gap-3 border border-white/5">
                                            {#if p.kind.includes('Keyboard')}<Keyboard size={18} class="text-yellow-400 shrink-0"/>
                                            {:else}<Mouse size={18} class="text-orange-400 shrink-0"/>{/if}
                                            <span class="text-sm font-bold text-gray-300 truncate">{p.name}</span>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {/if}

                        {#if periphs_other.length > 0}
                            <div class="mb-4">
                                <h4 class="text-xs font-bold text-gray-500 uppercase mb-2 tracking-wider border-b border-white/10 pb-1">{t[lang].cat_other}</h4>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                                    {#each periphs_other as p}
                                        <div class="bg-black/20 p-3 rounded-lg flex items-center gap-3 border border-white/5">
                                            {#if p.kind.includes('Camera')}<Camera size={18} class="text-purple-400 shrink-0"/>
                                            {:else}<Usb size={18} class="text-gray-400 shrink-0"/>{/if}
                                            <span class="text-sm font-bold text-gray-300 truncate">{p.name}</span>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {/if}

                    {:else}
                        <p class="text-sm text-gray-500 italic">Aucun périphérique détecté.</p>
                    {/if}
                </div>

                <div class="bg-[var(--card-bg)] p-6 rounded-xl border border-white/5">
                    <h3 class="font-bold text-white mb-4 flex items-center"><HardDrive size={20} class="mr-2"/> {t[lang].disks}</h3>
                    <div class="space-y-4">
                        {#each staticSpecs.disks as disk}
                            <div>
                                <div class="flex justify-between text-sm mb-1">
                                    <span class="text-gray-300 font-bold">{disk.name} <span class="text-xs text-gray-500">({disk.kind})</span></span>
                                    <span class="text-[var(--accent-color)] font-mono">{formatBytes(disk.available_space)} / {formatBytes(disk.total_space)}</span>
                                </div>
                                <div class="w-full bg-black/40 h-2 rounded-full overflow-hidden">
                                    <div class="h-full bg-[var(--accent-color)]" style="width: {((disk.total_space - disk.available_space) / disk.total_space) * 100}%"></div>
                                </div>
                            </div>
                        {/each}
                    </div>
                </div>
            </div>
        {/if}
    </div>

    {#if showSettings}
        <div class="fixed inset-0 z-[150] flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fade-in">
            <div class="bg-[var(--card-bg)] w-full max-w-4xl h-[70vh] rounded-2xl shadow-[0_0_50px_rgba(0,0,0,0.6)] border border-white/10 flex overflow-hidden">
                
                <div class="w-64 bg-black/20 border-r border-white/5 p-6 flex flex-col gap-2">
                    <h2 class="text-xl font-black text-[var(--text-primary)] mb-6 tracking-wide uppercase px-2">{t[lang].settings}</h2>
                    
                    <button on:click={() => settingsTab = 'appearance'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'appearance' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>
                        {t[lang].appearance}
                    </button>
                    <button on:click={() => settingsTab = 'projects'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'projects' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>
                        {t[lang].projects}
                    </button>
                    <button on:click={() => settingsTab = 'about'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'about' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>
                        {t[lang].about}
                    </button>
                    
                    <div class="flex-1"></div>
                    <button on:click={() => showSettings = false} class="text-left px-4 py-3 rounded-lg font-bold text-gray-500 hover:text-white transition-colors">
                        {t[lang].close}
                    </button>
                </div>

                <div class="flex-1 p-8 overflow-y-auto custom-scroll bg-[var(--card-bg)]">
                    
                    {#if settingsTab === 'appearance'}
                        <div class="space-y-8 animate-fade-in">
                            <div>
                                <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-4">{t[lang].themes}</h3>
                                <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
                                    {#each themes as th}
                                        <button on:click={() => selectTheme(th)} class="group relative overflow-hidden rounded-xl border border-white/10 hover:border-white/40 transition-all text-left h-24 shadow-lg hover:shadow-xl hover:scale-[1.02]">
                                            <div class="absolute inset-0" style={`background: linear-gradient(to bottom right, ${th.bg_from}, ${th.bg_to})`}></div>
                                            <div class="absolute inset-0 p-4 flex flex-col justify-end">
                                                <div class="flex items-center justify-between">
                                                    <span class="font-bold text-white shadow-black drop-shadow-md">{th.name}</span>
                                                    <div class="w-6 h-6 rounded-full border-2 border-white shadow-md" style={`background-color: ${th.accent}`}></div>
                                                </div>
                                            </div>
                                        </button>
                                    {/each}
                                </div>
                            </div>

                            <div class="border-t border-white/10 pt-6">
                                <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-6">{t[lang].custom}</h3>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                    <label class="block space-y-2">
                                        <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{t[lang].cust_accent}</span>
                                        <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                            <input type="color" bind:value={currentTheme.accent} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/><span class="text-gray-300 font-mono text-sm">{currentTheme.accent}</span>
                                        </div>
                                    </label>
                                    <label class="block space-y-2">
                                        <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{t[lang].cust_card}</span>
                                        <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                            <input type="color" bind:value={currentTheme.card_bg} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/><span class="text-gray-300 font-mono text-sm">{currentTheme.card_bg}</span>
                                        </div>
                                    </label>
                                    <label class="block space-y-2">
                                        <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{t[lang].cust_bg_top}</span>
                                        <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                            <input type="color" bind:value={currentTheme.bg_from} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/><span class="text-gray-300 font-mono text-sm">{currentTheme.bg_from}</span>
                                        </div>
                                    </label>
                                    <label class="block space-y-2">
                                        <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{t[lang].cust_bg_bot}</span>
                                        <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                            <input type="color" bind:value={currentTheme.bg_to} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/><span class="text-gray-300 font-mono text-sm">{currentTheme.bg_to}</span>
                                        </div>
                                    </label>
                                    <label class="block space-y-2">
                                        <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{t[lang].cust_text}</span>
                                        <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                            <input type="color" bind:value={currentTheme.text} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/><span class="text-gray-300 font-mono text-sm">{currentTheme.text}</span>
                                        </div>
                                    </label>
                                </div>
                            </div>
                        </div>

                    {:else if settingsTab === 'projects'}
                        <div class="space-y-8 animate-fade-in">
                            <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-4">{t[lang].projects}</h3>
                            <div class="bg-[var(--card-bg)] rounded-xl border border-white/10 overflow-hidden flex flex-col md:flex-row hover:border-[var(--accent-color)] transition-all group">
                                <div class="w-full md:w-48 h-32 bg-black/40 flex items-center justify-center text-[var(--accent-color)] relative">
                                    <img src="/gamehub_banner.png" alt="Game Hub" class="w-full h-full object-cover"/>
                                </div>
                                <div class="p-6 flex-1 flex flex-col justify-center">
                                    <h4 class="text-xl font-bold text-white mb-2">Geewer's Game Hub</h4>
                                    <p class="text-sm text-gray-400 mb-4">{t[lang].gamehub_desc}</p>
                                    <button on:click={() => openExternal('https://apps.microsoft.com/detail/9N9CF4NLLQZ7?hl=fr&gl=FR&ocid=pdpshare')} class="self-start bg-[var(--accent-color)] text-white px-4 py-2 rounded-lg font-bold text-xs flex items-center hover:brightness-110 transition-all">
                                        Voir sur Microsoft Store <ExternalLink size={14} class="ml-2"/>
                                    </button>
                                </div>
                            </div>
                        </div>

                    {:else if settingsTab === 'about'}
                        <div class="flex flex-col items-center justify-center h-full text-center space-y-6 animate-fade-in">
                            <div class="w-24 h-24 bg-gradient-to-br from-[#2a2a2a] to-black rounded-3xl flex items-center justify-center shadow-2xl ring-1 ring-white/10 p-4"><img src="/icon.png" alt="Logo" class="w-full h-full object-contain" /></div>
                            <h3 class="text-3xl font-black text-[var(--text-primary)]">Geewer Monitor</h3>
                            <p class="text-gray-400 mt-2">{t[lang].version}</p>
                            <div class="flex flex-col gap-3 w-full max-w-sm mt-8">
                                <button on:click={() => openExternal('mailto:s99ddhhh9@proton.me?subject=Feedback%20Geewer%20Monitor')} class="bg-[#333] hover:bg-white hover:text-black text-white px-6 py-4 rounded-xl font-bold shadow-lg transition-all flex items-center justify-center"><Bug size={20} class="mr-3"/> {t[lang].bug}</button>
                                <button on:click={() => openExternal('https://buymeacoffee.com/geewerrr')} class="bg-[#FFDD00] text-black font-black px-6 py-4 rounded-xl shadow-lg hover:scale-105 transition-all flex items-center justify-center"><Coffee size={20} class="mr-3"/> {t[lang].coffee}</button>
                            </div>
                        </div>
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</main>

<style>
    .custom-scroll::-webkit-scrollbar { width: 5px; height: 5px; }
    .custom-scroll::-webkit-scrollbar-track { background: rgba(255, 255, 255, 0.05); border-radius: 10px; }
    .custom-scroll::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.2); border-radius: 10px; }
    .custom-scroll::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.4); }
    .animate-fade-in { animation: fadeIn 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards; }
    @keyframes fadeIn { from { opacity: 0; transform: translateY(10px) scale(0.98); } to { opacity: 1; transform: translateY(0) scale(1); } }
</style>