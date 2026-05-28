<script lang="ts">
    import {
        duration,
        receivePill,
        SectionLabel,
        sendPill,
        slideUp,
        theme,
    } from 'anasthasia';
    import { sidebar } from '$lib/sidebar/state.svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { STEPS } from '$lib/wizard/steps';
    import { page } from '$app/state';
    import { fade } from 'svelte/transition';
    import {
        IconHome,
        IconFileSignal,
        IconSearch,
        IconSettings,
        IconChevronRight,
        IconChevronLeft,
        IconCheck,
        IconSun,
        IconMoon,
    } from '@tabler/icons-svelte';
    import pfpUrl from '$assets/pfp.avif';

    let sidebarSteps = $derived(
        STEPS.map((s) => ({
            id: s.id,
            label: s.label,
            status: (wizard.completedStepIds.has(s.id)
                ? 'done'
                : s.id === wizard.currentStepId
                  ? 'active'
                  : s.condition && !s.condition(wizard)
                    ? 'conditional'
                    : 'locked') as 'done' | 'active' | 'locked' | 'conditional',
        }))
    );

    let navLinks = $derived(
        [
            { href: '/', label: 'Home', icon: IconHome, match: (p: string) => p === '/' },
            {
                href: '/convert',
                label: 'Convert',
                icon: IconFileSignal,
                match: (p: string) => p.startsWith('/convert'),
            },
            {
                href: '/discover',
                label: 'Discover',
                icon: IconSearch,
                match: (p: string) => p.startsWith('/discover'),
            },
            {
                href: '/settings',
                label: 'Settings',
                icon: IconSettings,
                match: (p: string) => p.startsWith('/settings'),
            },
        ].filter((link) => link !== null)
    );

    function handleStepClick(id: string, status: string) {
        if (status === 'done') {
            document.dispatchEvent(new CustomEvent('wizard:goto', { detail: id }));
        }
    }

    let locked = $derived(wizard.converting);

    function preventIfLocked(e: Event) {
        if (locked) e.preventDefault();
    }

    function handleNavClick(e: Event) {
        if (locked) {
            e.preventDefault();
            return;
        }
        if (sidebar.mode === 'nav') sidebar.close();
    }
</script>

<aside
    class="z-50 flex h-full flex-shrink-0 overflow-hidden border-r border-anasthasia-border bg-anasthasia-surface transition-[width] duration-200 {sidebar.isOpen
        ? 'w-50'
        : 'w-12'}"
>
    <nav
        class="flex min-w-0 flex-1 flex-col overflow-hidden py-3"
        aria-label="Primary"
    >
        <div class="flex h-11 flex-shrink-0 items-center px-2">
            <a
                href="/"
                onclick={handleNavClick}
                aria-disabled={locked}
                tabindex={locked ? -1 : 0}
                class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-panel transition-colors duration-150 hover:border-anasthasia-accent/40 {locked
                    ? 'pointer-events-none opacity-40'
                    : ''}"
                title="Home"
            >
                <img src={pfpUrl} alt="" class="h-6 w-6 rounded-md object-cover" aria-hidden="true" />
            </a>
            {#if sidebar.isOpen}
            <div class="ml-2 min-w-0">
                <div class="truncate text-xs font-bold tracking-widest text-anasthasia-accent uppercase">
                    Thasia
                </div>
                <div class="truncate text-[9px] tracking-wider text-anasthasia-muted uppercase">
                    {sidebar.mode === 'wizard' ? 'Steps' : 'Workspace'}
                </div>
            </div>
            {/if}
        </div>

        <div class="mt-3 flex flex-1 flex-col overflow-y-auto px-2">
            {#key sidebar.mode}
                <div class="flex flex-col gap-1" in:slideUp={{ duration: duration.base }}>
                    {#if sidebar.mode === 'nav'}
                        {#each navLinks as link (link.href)}
                            {@const active = link.match(page.url.pathname)}
                            {@const Icon = link.icon}
                            <a
                                href={link.href}
                                onclick={handleNavClick}
                                aria-label={link.label}
                                aria-current={active ? 'page' : undefined}
                                aria-disabled={locked}
                                tabindex={locked ? -1 : 0}
                                title={link.label}
                                class="relative flex h-8 rounded-lg transition-colors duration-150
                                    {sidebar.isOpen
                                    ? 'w-full items-center gap-2 px-2'
                                    : 'w-8 items-center justify-center px-0'}
                                    {active
                                    ? 'text-anasthasia-text'
                                    : 'text-anasthasia-muted hover:bg-anasthasia-panel hover:text-anasthasia-text'}
                                    {locked && !active ? 'pointer-events-none opacity-40' : ''}"
                            >
                                {#if active}
                                    <span
                                        class="absolute inset-0 rounded-lg border border-anasthasia-accent/25 bg-anasthasia-accent/8"
                                        in:receivePill={{ key: 'nav-active' }}
                                        out:sendPill={{ key: 'nav-active' }}
                                    ></span>
                                {/if}
                                <Icon
                                    size={15}
                                    class="relative flex-shrink-0 {active ? 'text-anasthasia-accent' : ''}"
                                />
                                {#if sidebar.isOpen}
                                    <span class="relative min-w-0 truncate text-sm">
                                        {link.label}
                                    </span>
                                {/if}
                            </a>
                        {/each}
                    {:else}
                        <div
                            class="px-2 pb-2 transition-opacity duration-150 {sidebar.isOpen
                                ? 'opacity-100'
                                : 'pointer-events-none opacity-0'}"
                        >
                            <SectionLabel>Wizard</SectionLabel>
                        </div>

                        {#each sidebarSteps as step (step.id)}
                            <button
                                onclick={() => handleStepClick(step.id, step.status)}
                                disabled={locked ||
                                    step.status === 'locked' ||
                                    step.status === 'conditional'}
                                title={step.label}
                                class="relative flex h-8 rounded-lg text-left transition-colors duration-150
                                    {sidebar.isOpen
                                    ? 'w-full items-center gap-2 px-2'
                                    : 'w-8 items-center justify-center px-0'}
                                    {step.status === 'active'
                                    ? 'text-anasthasia-text'
                                    : step.status === 'done'
                                      ? 'cursor-pointer text-anasthasia-muted hover:bg-anasthasia-panel hover:text-anasthasia-text'
                                      : 'cursor-default text-anasthasia-muted opacity-40'}"
                            >
                                {#if step.status === 'active'}
                                    <span
                                        class="absolute inset-0 rounded-lg border border-anasthasia-accent/25 bg-anasthasia-accent/8"
                                        in:receivePill={{ key: 'step-active' }}
                                        out:sendPill={{ key: 'step-active' }}
                                    ></span>
                                {/if}

                                <span class="relative flex h-4 w-4 flex-shrink-0 items-center justify-center">
                                    {#if step.status === 'done'}
                                        <span
                                            class="flex h-4 w-4 items-center justify-center rounded-full bg-anasthasia-accent"
                                            in:fade={{ duration: duration.fast }}
                                        >
                                            <IconCheck size={10} class="text-anasthasia-text" stroke={3} />
                                        </span>
                                    {:else if step.status === 'active'}
                                        <span class="h-2 w-2 rounded-full border-2 border-anasthasia-accent"></span>
                                    {:else if step.status === 'conditional'}
                                        <span class="h-3 w-3 rounded-full border border-dashed border-anasthasia-border"></span>
                                    {:else}
                                        <span class="h-3 w-3 rounded-full border border-anasthasia-border"></span>
                                    {/if}
                                </span>

                                {#if sidebar.isOpen}
                                    <span
                                        class="relative min-w-0 truncate text-sm {step.status ===
                                        'conditional'
                                            ? 'italic'
                                            : ''}"
                                    >
                                        {step.label}
                                    </span>
                                {/if}
                            </button>
                        {/each}
                    {/if}
                </div>
            {/key}
        </div>

        <div class="flex-shrink-0 px-2">
            <button
                onclick={() => theme.toggle()}
                class="mb-1 flex h-8 rounded-lg text-anasthasia-muted transition-colors duration-150 hover:bg-anasthasia-panel hover:text-anasthasia-text {sidebar.isOpen
                    ? 'w-full items-center gap-2 px-2'
                    : 'w-8 items-center justify-center px-0'}"
                title={theme.dark ? 'Light mode' : 'Dark mode'}
                aria-label={theme.dark ? 'Light mode' : 'Dark mode'}
            >
                {#if theme.dark}
                    <IconSun size={15} class="flex-shrink-0" />
                    {#if sidebar.isOpen}
                        <span class="truncate text-sm">Light mode</span>
                    {/if}
                {:else}
                    <IconMoon size={15} class="flex-shrink-0" />
                    {#if sidebar.isOpen}
                        <span class="truncate text-sm">Dark mode</span>
                    {/if}
                {/if}
            </button>

            <button
                onclick={() => sidebar.toggle()}
                aria-label="Toggle sidebar"
                title="Toggle sidebar"
                class="flex h-8 rounded-lg text-anasthasia-muted transition-colors duration-150 hover:bg-anasthasia-panel hover:text-anasthasia-accent {sidebar.isOpen
                    ? 'w-full items-center gap-2 px-2'
                    : 'w-8 items-center justify-center px-0'}"
            >
                {#if sidebar.isOpen}
                    <IconChevronLeft size={15} class="flex-shrink-0" />
                    <span class="truncate text-sm">Collapse</span>
                {:else}
                    <IconChevronRight size={15} />
                {/if}
            </button>
        </div>
    </nav>
</aside>
