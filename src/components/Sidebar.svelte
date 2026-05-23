<script lang="ts">
    import {
        duration,
        receivePill,
        SectionLabel,
        sendPill,
        sidebarSlide,
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
        IconSettings,
        IconInfoCircle,
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

    const navLinks = [
        { href: '/', label: 'Home', icon: IconHome, match: (p: string) => p === '/' },
        {
            href: '/convert',
            label: 'Convert',
            icon: IconFileSignal,
            match: (p: string) => p.startsWith('/convert'),
        },
        {
            href: '/settings',
            label: 'Settings',
            icon: IconSettings,
            match: (p: string) => p.startsWith('/settings'),
        },
    ];

    function handleStepClick(id: string, status: string) {
        if (status === 'done') {
            document.dispatchEvent(new CustomEvent('wizard:goto', { detail: id }));
        }
    }

    let locked = $derived(wizard.converting);

    function preventIfLocked(e: Event) {
        if (locked) e.preventDefault();
    }
</script>

<!-- Sidebar container: tab always visible, panel slides in beside it -->
<aside class="z-50 flex h-full flex-shrink-0">
    {#if sidebar.isOpen}
        <!-- Panel -->
        <nav
            class="flex w-52 flex-col overflow-hidden border-r border-anasthasia-border bg-anasthasia-surface"
            transition:sidebarSlide
        >
            <!-- Wordmark -->
            <a
                href="/"
                onclick={preventIfLocked}
                aria-disabled={locked}
                tabindex={locked ? -1 : 0}
                class="group flex flex-shrink-0 items-center gap-2.5 border-b border-anasthasia-border px-4 pt-5 pb-4 transition-colors duration-150 hover:bg-anasthasia-panel
                       {locked ? 'pointer-events-none opacity-40' : ''}"
            >
                <img
                    src={pfpUrl}
                    alt=""
                    class="h-8 w-8 flex-shrink-0 rounded-lg border border-anasthasia-border bg-anasthasia-panel object-cover transition-transform duration-150 group-hover:scale-105"
                    aria-hidden="true"
                />
                <div class="min-w-0">
                    <div
                        class="text-sm font-bold tracking-widest text-anasthasia-accent uppercase transition-opacity duration-150 group-hover:opacity-80"
                    >
                        Thasia
                    </div>
                    <div class="mt-0.5 text-[10px] tracking-wider text-anasthasia-muted uppercase">
                        Conversion Engine
                    </div>
                </div>
            </a>

            <!-- Content — keyed so it transitions on mode change -->
            <div class="flex flex-1 flex-col overflow-y-auto">
                {#key sidebar.mode}
                    <div
                        class="flex flex-1 flex-col gap-0.5 p-3"
                        in:slideUp={{ duration: duration.base }}
                    >
                        {#if sidebar.mode === 'nav'}
                            {#each navLinks as link (link.href)}
                                {@const active = link.match(page.url.pathname)}
                                {@const Icon = link.icon}
                                <a
                                    href={link.href}
                                    onclick={preventIfLocked}
                                    aria-disabled={locked}
                                    tabindex={locked ? -1 : 0}
                                    class="relative flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-colors duration-150
                                           {active
                                        ? 'text-anasthasia-text'
                                        : 'text-anasthasia-muted hover:bg-anasthasia-panel hover:text-anasthasia-text'}
                                           {locked && !active
                                        ? 'pointer-events-none opacity-40'
                                        : ''}"
                                >
                                    {#if active}
                                        <span
                                            class="absolute inset-0 rounded-lg border border-anasthasia-accent/25 bg-anasthasia-accent/8"
                                            in:receivePill={{ key: 'nav-active' }}
                                            out:sendPill={{ key: 'nav-active' }}
                                        ></span>
                                    {/if}
                                    <span class="relative {active ? 'text-anasthasia-accent' : ''}">
                                        <Icon size={15} />
                                    </span>
                                    <span class="relative">{link.label}</span>
                                </a>
                            {/each}
                        {:else}
                            <!-- Exit wizard -->
                            <a
                                href="/"
                                onclick={preventIfLocked}
                                aria-disabled={locked}
                                tabindex={locked ? -1 : 0}
                                class="mb-2 flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm
                                       text-anasthasia-muted transition-colors duration-150 hover:bg-anasthasia-panel hover:text-anasthasia-text
                                       {locked ? 'pointer-events-none opacity-40' : ''}"
                                title={locked ? 'Conversion in progress…' : ''}
                            >
                                <IconHome size={15} />
                                Home
                            </a>

                            <div class="px-3 pb-2">
                                <SectionLabel>Wizard</SectionLabel>
                            </div>

                            {#each sidebarSteps as step (step.id)}
                                <button
                                    onclick={() => handleStepClick(step.id, step.status)}
                                    disabled={locked ||
                                        step.status === 'locked' ||
                                        step.status === 'conditional'}
                                    class="relative flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-left text-sm
                                           transition-colors duration-150
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

                                    <!-- Step indicator -->
                                    <span
                                        class="relative flex h-4 w-4 flex-shrink-0 items-center justify-center"
                                    >
                                        {#if step.status === 'done'}
                                            <span
                                                class="flex h-4 w-4 items-center justify-center rounded-full bg-anasthasia-accent"
                                                in:fade={{ duration: duration.fast }}
                                            >
                                                <IconCheck
                                                    size={10}
                                                    class="text-anasthasia-text"
                                                    stroke={3}
                                                />
                                            </span>
                                        {:else if step.status === 'active'}
                                            <span
                                                class="relative flex h-4 w-4 items-center justify-center"
                                            >
                                                <span
                                                    class="absolute h-4 w-4 animate-ping rounded-full bg-anasthasia-accent/30"
                                                ></span>
                                                <span
                                                    class="h-2 w-2 rounded-full border-2 border-anasthasia-accent"
                                                ></span>
                                            </span>
                                        {:else if step.status === 'conditional'}
                                            <span
                                                class="h-3 w-3 rounded-full border border-dashed border-anasthasia-border"
                                            ></span>
                                        {:else}
                                            <span
                                                class="h-3 w-3 rounded-full border border-anasthasia-border"
                                            ></span>
                                        {/if}
                                    </span>

                                    <span
                                        class="relative truncate {step.status === 'conditional'
                                            ? 'italic'
                                            : ''}">{step.label}</span
                                    >
                                </button>
                            {/each}
                        {/if}
                    </div>
                {/key}
            </div>

            <!-- Bottom: theme toggle -->
            <div class="flex-shrink-0 border-t border-anasthasia-border p-3">
                <button
                    onclick={() => theme.toggle()}
                    class="flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-sm text-anasthasia-muted
                           transition-colors duration-150 hover:bg-anasthasia-panel hover:text-anasthasia-text"
                >
                    {#if theme.dark}
                        <IconSun size={15} />
                        <span>Light mode</span>
                    {:else}
                        <IconMoon size={15} />
                        <span>Dark mode</span>
                    {/if}
                </button>
            </div>
        </nav>
    {/if}

    <!-- Tab — glued to right edge of panel, always rendered -->
    <button
        onclick={() => sidebar.toggle()}
        aria-label="Toggle sidebar"
        class="flex flex-shrink-0 items-center justify-center self-stretch border-r border-anasthasia-border
               bg-anasthasia-surface text-anasthasia-muted transition-all duration-150
               hover:bg-anasthasia-panel hover:text-anasthasia-accent
               {sidebar.isOpen ? 'w-4' : 'w-6'}"
    >
        {#if sidebar.isOpen}
            <IconChevronLeft size={13} />
        {:else}
            <IconChevronRight size={13} />
        {/if}
    </button>
</aside>
