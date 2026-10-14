<script>
    // @ts-nocheck
    // Courbes SVG légères : series = [{ values: number[], color: string }]
    export let series = [];
    export let max = null; // null = auto
    export let points = 60;
    export let height = 120;
    export let grid = false;

    $: peak = max ?? Math.max(1, ...series.flatMap(s => s.values)) * 1.15;
    const uid = Math.random().toString(36).slice(2);

    function path(values, close) {
        if (values.length < 2) return '';
        const step = 100 / (points - 1);
        const offset = (points - values.length) * step;
        const pts = values.map((v, i) => `${(offset + i * step).toFixed(2)},${(height - (Math.min(v, peak) / peak) * height).toFixed(2)}`);
        const line = 'M' + pts.join(' L');
        return close ? `${line} L100,${height} L${offset.toFixed(2)},${height} Z` : line;
    }
</script>

<svg viewBox="0 0 100 {height}" preserveAspectRatio="none" class="w-full block" style="height: {height}px" aria-hidden="true">
    <defs>
        {#each series as s, i}
            <linearGradient id="g{uid}{i}" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stop-color={s.color} stop-opacity="0.35" />
                <stop offset="100%" stop-color={s.color} stop-opacity="0" />
            </linearGradient>
        {/each}
    </defs>
    {#if grid}
        {#each [0.25, 0.5, 0.75] as g}
            <line x1="0" x2="100" y1={height * g} y2={height * g} stroke="currentColor" stroke-opacity="0.07" vector-effect="non-scaling-stroke" stroke-dasharray="4 4" />
        {/each}
    {/if}
    {#each series as s, i}
        <path d={path(s.values, true)} fill="url(#g{uid}{i})" />
        <path d={path(s.values, false)} fill="none" stroke={s.color} stroke-width="2" vector-effect="non-scaling-stroke" stroke-linejoin="round" stroke-linecap="round" />
    {/each}
</svg>
