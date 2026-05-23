<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { sidebar } from '$lib/sidebar/state.svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { activeSteps } from '$lib/wizard/steps';
    import { applyToWizard, loadSettings } from '$lib/settings';
    import { keyboard } from 'anasthasia';
    import { mountedHint } from '$lib/keyhint.svelte';

    onMount(() => {
        sidebar.enterWizard();
        if (!wizard.currentStepId) wizard.currentStepId = 'source';
        applyToWizard(loadSettings());

        document.addEventListener('wizard:goto', handleGoto);

        const cleanup = keyboard.smartRegister([
            [
                'shift+arrowright',
                (e) => {
                    if (!canGoNext) return false;
                    e.preventDefault();
                    goNext();
                    return true;
                },
            ],
            [
                'shift+arrowleft',
                (e) => {
                    if (!canGoBack) return false;
                    e.preventDefault();
                    goBack();
                    return true;
                },
            ],
        ]);

        return () => {
            cleanup();
            document.removeEventListener('wizard:goto', handleGoto);
        };
    });

    onDestroy(() => {
        sidebar.exitWizard();
    });

    function handleGoto(e: Event) {
        const id = (e as CustomEvent<string>).detail;
        if (wizard.completedStepIds.has(id)) wizard.currentStepId = id;
    }

    let active = $derived(activeSteps(wizard));
    let currentStep = $derived(active.find((s) => s.id === wizard.currentStepId) ?? active[0]);
    let currentIndex = $derived(active.findIndex((s) => s.id === wizard.currentStepId));

    let canGoBack = $derived(currentIndex > 0 && currentStep?.id !== 'convert');
    let canGoNext = $derived(
        !!currentStep && !currentStep.selfManagedNext && currentStep.id !== 'convert'
    );

    let navHints = $derived([
        ...(canGoNext ? [['shift+arrowright', 'Next step'] as [string, string]] : []),
        ...(canGoBack ? [['shift+arrowleft', 'Back'] as [string, string]] : []),
    ]);

    function goNext() {
        if (!currentStep) return;
        wizard.markComplete(currentStep.id);

        const nextIndex = currentIndex + 1;
        if (nextIndex < active.length) wizard.currentStepId = active[nextIndex].id;
        if (active[nextIndex]?.id === 'convert') sidebar.collapseForConversion();
    }

    function goBack() {
        if (currentIndex > 0) wizard.currentStepId = active[currentIndex - 1].id;
    }
</script>

<div class="flex h-full flex-col" use:mountedHint={navHints}>
    {#if currentStep}
        {#key currentStep.id}
            <currentStep.component onNext={goNext} onBack={goBack} backDisabled={!canGoBack} />
        {/key}
    {/if}
</div>
