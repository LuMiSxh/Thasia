<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { sidebar } from '$lib/sidebar/state.svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { activeSteps } from '$lib/wizard/steps';
    import { keyboard } from 'anasthasia';
    import { mountedHint } from '$lib/keyhint.svelte';

    onMount(() => {
        sidebar.enterWizard();
        if (!wizard.currentStepId) wizard.currentStepId = 'source';

        const raw = localStorage.getItem('thasia:settings');
        if (raw && !wizard.sourcePath) {
            try {
                const d = JSON.parse(raw);
                if (d.imageFormat) wizard.imageFormat = d.imageFormat;
                if (d.container) wizard.container = d.container;
                if (d.direction) wizard.direction = d.direction;
                if (d.bundle) wizard.bundle = d.bundle;
                if (d.volumeSeparator !== undefined) wizard.volumeSeparator = d.volumeSeparator;
                if (d.hideSingleVolume !== undefined) wizard.hideSingleVolume = d.hideSingleVolume;
                if (d.createDirectory !== undefined) wizard.createDirectory = d.createDirectory;
                if (d.maxWidth !== undefined) wizard.maxWidth = d.maxWidth;
            } catch {}
        }

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

    // Steps that manage their own shift+arrowright (async/complex handleNext, or own arrow key usage)
    const selfManagedNext = new Set(['source', 'volume-review', 'page-editor']);

    let canGoBack = $derived(currentIndex > 0 && currentStep?.id !== 'convert');
    let canGoNext = $derived(
        !!currentStep &&
            !selfManagedNext.has(currentStep.id) &&
            currentStep.id !== 'convert' &&
            (currentStep.id !== 'destination' || (!!wizard.outputDir && !!wizard.outputName))
    );

    let nextDisabled = $derived(
        currentStep?.id === 'destination' ? !(wizard.outputDir && wizard.outputName) : false
    );

    let navHints = $derived([
        ...(canGoNext ? [['shift+arrowright', 'Next step'] as [string, string]] : []),
        ...(canGoBack ? [['shift+arrowleft', 'Back'] as [string, string]] : []),
    ]);

    function goNext() {
        if (!currentStep) return;
        wizard.markComplete(currentStep.id);

        // Flatten mode skips volume-review — merge all scan volumes into one output volume
        // so the page editor and convert both see a single unified volume.
        if (currentStep.id === 'bundling' && wizard.bundle === 'flatten') {
            wizard.pageEdits = [
                {
                    volumeNum: 1,
                    pages: wizard.pageEdits.flatMap((ve) => ve.pages),
                },
            ];
        }

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
            <currentStep.component
                onNext={goNext}
                onBack={goBack}
                {nextDisabled}
                backDisabled={!canGoBack}
            />
        {/key}
    {/if}
</div>
