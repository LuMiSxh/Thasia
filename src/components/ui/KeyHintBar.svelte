<script lang="ts">
    import { keyHint } from '$lib/keyhint.svelte';
    import { uiPrefs } from '$lib/ui-prefs.svelte';
    import { cubicOut } from 'svelte/easing';
    import KeyComboDisplay from './KeyComboDisplay.svelte';

    function glassChip(node: Element, { duration = 200 }: { duration?: number } = {}) {
        const style = getComputedStyle(node);
        const w = parseFloat(style.width);
        const mr = parseFloat(style.marginRight);
        return {
            duration,
            css: (t: number) => {
                const e = cubicOut(t);
                return `
                    width:${w * e}px;
                    margin-right:${mr * e}px;
                    opacity:${e};
                    transform:translateX(${(1 - e) * -16}px) scale(${0.9 + 0.1 * e});
                    filter:blur(${(1 - e) * 4}px);
                    overflow:hidden;
                    white-space:nowrap;
                `;
            },
        };
    }

    let hints = $derived(keyHint.get());
</script>

{#if uiPrefs.showKeyHints && hints.length > 0}
    <div
        class="flex h-8 flex-shrink-0 items-center overflow-hidden border-t border-thasia-border bg-thasia-surface px-4"
    >
        {#each hints as [combo, label] (combo)}
            <div class="mr-5 flex items-center gap-2" transition:glassChip>
                <KeyComboDisplay {combo} />
                <span class="text-xs whitespace-nowrap text-thasia-muted">{label}</span>
            </div>
        {/each}
    </div>
{/if}
