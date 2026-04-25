<script lang="ts">
    import Kbd from './Kbd.svelte';

    let { combo }: { combo: string } = $props();

    const isMac = typeof navigator !== 'undefined' && navigator.platform.startsWith('Mac');

    const macSymbols: Record<string, string> = {
        meta: '⌘',
        alt: '⌥',
        ctrl: '⌃',
        shift: '⇧',
        arrowright: '→',
        arrowleft: '←',
        arrowup: '↑',
        arrowdown: '↓',
        enter: '↩',
        escape: 'Esc',
        backspace: '⌫',
        tab: '⇥',
        space: '␣',
    };

    const winSymbols: Record<string, string> = {
        meta: 'Win',
        alt: 'Alt',
        ctrl: 'Ctrl',
        shift: '⇧',
        arrowright: '→',
        arrowleft: '←',
        arrowup: '↑',
        arrowdown: '↓',
        enter: '↵',
        escape: 'Esc',
        backspace: '⌫',
        tab: '⇥',
        space: '␣',
    };

    function formatPart(part: string): string {
        const symbols = isMac ? macSymbols : winSymbols;
        if (symbols[part]) return symbols[part];
        if (part.startsWith('key')) return part.slice(3).toUpperCase();
        if (part.startsWith('digit')) return part.slice(5);
        return part.toUpperCase();
    }

    let parts = $derived(combo.toLowerCase().split('+').map(formatPart));
</script>

<span class="inline-flex items-center gap-0.5">
    {#each parts as part, i (i)}
        <Kbd>{part}</Kbd>
    {/each}
</span>
