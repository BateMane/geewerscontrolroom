<script>
  // @ts-nocheck
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-shell';
  
  // Imports des icônes
  import { LayoutDashboard, Monitor, Settings, Coffee, Bug, X, Languages } from 'lucide-svelte';

  // --- ÉTAT GLOBAL (Votre code d'origine) ---
  let activeTab = 'dashboard';
  let showSettings = false;
  let settingsTab = 'appearance'; // Ajout pour la navigation dans les settings
  let staticSpecs = null;
  let realtimeData = { cpu_usage: 0, ram_used: 0, ram_total: 1, swap_used: 0 };
  let unlisten = null;
  let lang = 'fr'; 

  // Calcul réactif
  $: ramPercent = (realtimeData.ram_used / realtimeData.ram_total) * 100;

  // --- VOS TRADUCTIONS ---
  const t = {
      fr: {
          dashboard: "Tableau de Bord", specs: "Mon PC", cpu: "Charge CPU", ram: "RAM Utilisée", swap: "Mémoire Swap",
          settings: "Paramètres", appearance: "Apparence", about: "À Propos", 
          bug: "Signaler un bug", coffee: "M'offrir un café",
          os: "Système", cores: "Cœurs", hostname: "Nom PC", close: "Fermer",
          welcome: "Bienvenue, système opérationnel.", version: "Version 1.0.0",
          themes: "Thèmes Prédéfinis", custom: "Personnalisation Avancée"
      },
      en: {
          dashboard: "Dashboard", specs: "My PC", cpu: "CPU Load", ram: "RAM Usage", swap: "Swap Memory",
          settings: "Settings", appearance: "Appearance", about: "About",
          bug: "Report a bug", coffee: "Buy me a coffee",
          os: "System", cores: "Cores", hostname: "PC Name", close: "Close",
          welcome: "Welcome, system operational.", version: "Version 1.0.0",
          themes: "Preset Themes", custom: "Advanced Customization"
      }
  };

  // --- VOS THEMES ---
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
  ];

  // --- INITIALISATION (Votre code) ---
  onMount(async () => {
    try {
        staticSpecs = await invoke('get_static_specs');
        unlisten = await listen('system-update', (event) => {
            realtimeData = event.payload;
        });

        const savedLang = localStorage.getItem('hw_lang');
        if(savedLang) lang = savedLang;
        
        const savedTheme = localStorage.getItem('hw_theme');
        if(savedTheme) {
            currentTheme = JSON.parse(savedTheme);
            applyTheme(); // Applique le thème chargé
        } else {
            applyTheme(); // Applique le thème par défaut
        }
    } catch(e) {
        console.error(e);
    }
  });

  onDestroy(() => {
      if(unlisten) unlisten();
  });

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

  function openExternal(url) {
      open(url);
  }

  const formatBytes = (bytes) => (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
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
                    <h1 class="text-4xl font-black tracking-wider text-[var(--text-primary)] uppercase mb-1">Geewer Monitor</h1>
                    <p class="text-sm text-gray-400 font-medium">
                        {t[lang].welcome}
                    </p>
                </div>
            </div>
            
            <div class="flex gap-3">
                <button on:click={toggleLang} class="flex items-center justify-center gap-2 border border-white/10 px-3 py-2 rounded-lg bg-[var(--card-bg)] hover:bg-[#333] transition-all hover:scale-105 shadow-md active:scale-95">
                    {#if lang === 'fr'}
                        <span class="text-xs font-bold text-gray-300">FR</span>
                    {:else}
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
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 animate-fade-in">
                
                <div class="group relative bg-[var(--card-bg)] rounded-xl overflow-hidden p-6 border border-white/5 hover:border-[var(--accent-color)] transition-all duration-300 hover:shadow-lg hover:-translate-y-1">
                    <div class="flex justify-between items-center mb-6">
                        <div class="flex items-center gap-3">
                            <div class="p-2 bg-white/5 rounded-lg text-[var(--accent-color)]"><Monitor size={24} /></div>
                            <h3 class="font-bold text-[var(--text-primary)] text-lg">{t[lang].cpu}</h3>
                        </div>
                        <span class="text-3xl font-black text-[var(--accent-color)]">{realtimeData.cpu_usage.toFixed(1)}%</span>
                    </div>
                    <div class="w-full bg-black/40 h-3 rounded-full overflow-hidden mb-2">
                        <div class="h-full bg-[var(--accent-color)] transition-all duration-500 ease-out shadow-[0_0_10px_var(--accent-color)]" style="width: {realtimeData.cpu_usage}%"></div>
                    </div>
                </div>

                <div class="group relative bg-[var(--card-bg)] rounded-xl overflow-hidden p-6 border border-white/5 hover:border-[#10b981] transition-all duration-300 hover:shadow-lg hover:-translate-y-1">
                    <div class="flex justify-between items-center mb-6">
                        <div class="flex items-center gap-3">
                            <div class="p-2 bg-white/5 rounded-lg text-[#10b981]"><Monitor size={24} /></div>
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

                <div class="group relative bg-[var(--card-bg)] rounded-xl overflow-hidden p-6 border border-white/5 hover:border-[#f59e0b] transition-all duration-300 hover:shadow-lg hover:-translate-y-1">
                    <div class="flex justify-between items-center mb-6">
                        <div class="flex items-center gap-3">
                            <div class="p-2 bg-white/5 rounded-lg text-[#f59e0b]"><Monitor size={24} /></div>
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
                        <div class="flex items-center gap-2">
                            <span class="bg-[var(--accent-color)]/20 text-[var(--accent-color)] text-xs font-bold px-2 py-1 rounded">{staticSpecs.cpu_cores} {t[lang].cores}</span>
                        </div>
                    </div>
                    <div class="bg-[var(--card-bg)] p-6 rounded-xl border border-white/5 hover:bg-white/5 transition-colors">
                        <div class="text-xs text-gray-500 uppercase font-bold tracking-wider mb-2">RAM Total</div>
                        <div class="text-xl font-bold text-white">{formatBytes(staticSpecs.total_memory)}</div>
                        <div class="text-sm text-gray-500 mt-1">DDR4 / DDR5</div>
                    </div>
                    <div class="bg-[var(--card-bg)] p-6 rounded-xl border border-white/5 hover:bg-white/5 transition-colors">
                        <div class="text-xs text-gray-500 uppercase font-bold tracking-wider mb-2">{t[lang].os}</div>
                        <div class="text-xl font-bold text-white">{staticSpecs.os}</div>
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
                                        <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">Accent</span>
                                        <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                            <input type="color" bind:value={currentTheme.accent} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                            <span class="text-gray-300 font-mono text-sm">{currentTheme.accent}</span>
                                        </div>
                                    </label>
                                    </div>
                            </div>
                        </div>

                    {:else if settingsTab === 'about'}
                        <div class="flex flex-col items-center justify-center h-full text-center space-y-6 animate-fade-in">
                            <div class="w-24 h-24 bg-gradient-to-br from-[#2a2a2a] to-black rounded-3xl flex items-center justify-center shadow-2xl ring-1 ring-white/10 p-4">
                                <img src="/icon.png" alt="Logo" class="w-full h-full object-contain" />      
                            </div>
                            <div>
                                <h3 class="text-3xl font-black text-[var(--text-primary)]">Geewer Monitor</h3>
                                <p class="text-gray-400 mt-2">{t[lang].version}</p>
                            </div>
                            
                            <div class="flex flex-col gap-3 w-full max-w-sm mt-8">
                                <button on:click={() => openExternal('mailto:s99ddhhh9@proton.me?subject=Feedback%20Geewer%20Monitor')} class="bg-[#333] hover:bg-white hover:text-black text-white px-6 py-4 rounded-xl font-bold shadow-lg transition-all flex items-center justify-center">
                                    <Bug size={20} class="mr-3"/> {t[lang].bug}
                                </button>
                                <button on:click={() => openExternal('https://buymeacoffee.com/geewerrr')} class="bg-[#FFDD00] text-black font-black px-6 py-4 rounded-xl shadow-lg hover:scale-105 transition-all flex items-center justify-center">
                                    <Coffee size={20} class="mr-3"/> {t[lang].coffee}
                                </button>
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