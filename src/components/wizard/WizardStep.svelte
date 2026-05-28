<script lang="ts">
    import type { Snippet } from 'svelte';
    import { onMount, onDestroy } from 'svelte';
    import { Alert, Button, keyboard } from 'anasthasia';
    import { IconArrowLeft, IconArrowRight } from '@tabler/icons-svelte';
    import { mountedHint } from '$lib/keyhint.svelte';

    export type ValidationResult = string | string[] | null;

    interface Props {
        title: string;
        description?: string;
        onNext?: () => void | Promise<void>;
        onBack?: () => void;
        backDisabled?: boolean;
        /** Runs before onNext. Return null to allow, string(s) to block + display. */
        validate?: () => ValidationResult;
        loading?: boolean;
        loadingLabel?: string;
        nextLabel?: string;
        nextVariant?: 'primary' | 'secondary';
        /** Step registers its own shift+arrowright — wrapper won't bind one. */
        selfManagedNext?: boolean;
        /** Extra step-specific keyboard hints to show in the hint bar. */
        extraHints?: [string, string][];
        children: Snippet;
        /** Override the default footer (Back + Next). */
        footer?: Snippet;
        showFooter?: boolean;
    }

    let {
        title,
        description,
        onNext,
        onBack,
        backDisabled = false,
        validate,
        loading = false,
        loadingLabel = 'Working',
        nextLabel = 'Next',
        nextVariant = 'secondary',
        selfManagedNext = false,
        extraHints = [],
        children,
        footer,
        showFooter = true,
    }: Props = $props();

    let errors = $state<string[]>([]);

    async function handleNext() {
        if (loading || !onNext) return;
        if (validate) {
            const result = validate();
            const normalized = result === null ? [] : Array.isArray(result) ? result : [result];
            errors = normalized;
            if (normalized.length > 0) return;
        } else {
            errors = [];
        }
        await onNext();
    }

    function handleBack() {
        if (loading || backDisabled || !onBack) return;
        errors = [];
        onBack();
    }

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        const keys: Array<[string, (e: KeyboardEvent) => boolean]> = [];
        if (!selfManagedNext && onNext) {
            keys.push([
                'shift+arrowright',
                (e) => {
                    e.preventDefault();
                    handleNext();
                    return true;
                },
            ]);
        }
        if (onBack) {
            keys.push([
                'shift+arrowleft',
                (e) => {
                    if (backDisabled) return false;
                    e.preventDefault();
                    handleBack();
                    return true;
                },
            ]);
        }
        if (keys.length > 0) cleanupKb = keyboard.smartRegister(keys);
    });
    onDestroy(() => cleanupKb?.());

    let hints = $derived(
        [
            !selfManagedNext && onNext ? (['shift+arrowright', 'Next step'] as [string, string]) : null,
            onBack && !backDisabled ? (['shift+arrowleft', 'Back'] as [string, string]) : null,
            ...extraHints,
        ].filter((h): h is [string, string] => h !== null)
    );
</script>

<div class="flex h-full flex-col" use:mountedHint={hints}>
    <!-- Header -->
    <div class="flex-shrink-0 border-b border-anasthasia-border px-5 py-3.5">
        <h2 class="text-base font-bold">{title}</h2>
        {#if description}
            <p class="mt-0.5 text-xs text-anasthasia-muted">{description}</p>
        {/if}
    </div>

    <!-- Content -->
    <div class="flex flex-1 flex-col gap-3 overflow-y-auto px-4 py-4 lg:px-5">
        {@render children()}
    </div>

    <!-- Validation errors -->
    {#if errors.length > 0}
        <div class="flex-shrink-0 px-5 pb-3">
            <Alert variant="danger" title="Can't continue yet">
                {#if errors.length === 1}
                    {errors[0]}
                {:else}
                    <ul class="list-disc pl-4">
                        {#each errors as e (e)}
                            <li>{e}</li>
                        {/each}
                    </ul>
                {/if}
            </Alert>
        </div>
    {/if}

    <!-- Footer -->
    {#if showFooter}
        {#if footer}
            {@render footer()}
        {:else}
            <div class="flex flex-shrink-0 gap-2 border-t border-anasthasia-border px-5 py-3.5">
                {#if onBack}
                    <Button onclick={handleBack} disabled={backDisabled || loading}>
                        <IconArrowLeft size={15} /> Back
                    </Button>
                {/if}
                {#if onNext}
                    <Button
                        variant={nextVariant}
                        onclick={handleNext}
                        loading={loading}
                        loadingLabel={loadingLabel}
                        class="ml-auto"
                    >
                        {nextLabel}
                        <IconArrowRight size={15} />
                    </Button>
                {/if}
            </div>
        {/if}
    {/if}
</div>
