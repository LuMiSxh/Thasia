<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { page } from '$app/state';
    import { sidebar } from '$lib/sidebar/state.svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { activeSteps } from '$lib/wizard/steps';
    import { applyToWizard, loadSettings } from '$lib/settings';
    import { commands } from '$types/bindings';
    import { Alert, keyboard } from 'anasthasia';
    import { mountedHint } from '$lib/keyhint.svelte';

    let discoveryScanLoading = $state(false);
    let discoveryScanError = $state('');

    onMount(() => {
        sidebar.enterWizard();
        if (page.url.searchParams.get('source') === 'discovery') {
            void applyDiscoverySource();
        } else if (!wizard.currentStepId) {
            wizard.currentStepId = 'source';
        }
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

    async function applyDiscoverySource() {
        discoveryScanLoading = true;
        discoveryScanError = '';
        wizard.reset();
        applyToWizard(loadSettings(), { force: true });
        wizard.sourcePath = 'Downloaded from Discover';
        wizard.container = 'cbz';
        const name = page.url.searchParams.get('name');
        if (name) wizard.outputName = name;

        const result = await commands.scanCurrentSource();
        discoveryScanLoading = false;

        if (result.status === 'error') {
            discoveryScanError = result.error;
            wizard.currentStepId = 'source';
            return;
        }

        wizard.scanResult = result.data;
        wizard.pageEdits = result.data.map((vol) => ({
            volumeNum: vol.volume_num,
            pages: vol.pages.map((_, i) => ({
                originalPageIndex: i,
                sourceVolumeNum: vol.volume_num,
                customPath: null,
                excluded: false,
            })),
        }));
        wizard.currentStepId = 'source';
    }
</script>

<div class="flex h-full flex-col" use:mountedHint={navHints}>
    {#if discoveryScanLoading}
        <div class="flex flex-1 items-center justify-center text-sm text-anasthasia-muted">
            Preparing downloaded chapters…
        </div>
    {:else if discoveryScanError}
        <div class="mx-auto flex w-full max-w-2xl flex-1 flex-col justify-center px-6">
            <Alert variant="danger" title="Could not prepare download for conversion"
                >{discoveryScanError}</Alert
            >
        </div>
    {:else if currentStep}
        {#key currentStep.id}
            <currentStep.component onNext={goNext} onBack={goBack} backDisabled={!canGoBack} />
        {/key}
    {/if}
</div>
