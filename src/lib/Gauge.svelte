<script>
    // @ts-nocheck
    // Jauge circulaire 0-100 ; le contenu central passe par le slot
    export let value = 0;
    export let color = 'var(--accent-color)';
    export let size = 128;

    const r = 52;
    const circ = 2 * Math.PI * r;
    $: clamped = Math.max(0, Math.min(100, value || 0));
</script>

<div class="relative shrink-0" style="width: {size}px; height: {size}px">
    <svg viewBox="0 0 120 120" class="w-full h-full -rotate-90" aria-hidden="true">
        <circle cx="60" cy="60" r={r} fill="none" stroke="currentColor" stroke-opacity="0.08" stroke-width="10" />
        <circle cx="60" cy="60" r={r} fill="none" stroke={color} stroke-width="10" stroke-linecap="round"
            stroke-dasharray={circ} stroke-dashoffset={circ * (1 - clamped / 100)}
            style="transition: stroke-dashoffset .6s cubic-bezier(.16,1,.3,1), stroke .3s; filter: drop-shadow(0 0 6px {color})" />
    </svg>
    <div class="absolute inset-0 flex flex-col items-center justify-center text-center">
        <slot />
    </div>
</div>
