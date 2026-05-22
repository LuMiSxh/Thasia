<script lang="ts">
    import { openUrl } from '@tauri-apps/plugin-opener';
    import {
        IconFolderOpen,
        IconListSearch,
        IconWand,
        IconStack2,
        IconPackage,
        IconBrandGithub,
        IconExternalLink,
    } from '@tabler/icons-svelte';

    const pipeline = [
        {
            icon: IconFolderOpen,
            label: 'Source',
            desc: 'Point to a folder, ZIP, or CBZ file containing your manga images.',
        },
        {
            icon: IconListSearch,
            label: 'Scan',
            desc: 'Images are discovered and sorted into volumes and chapters automatically.',
        },
        {
            icon: IconWand,
            label: 'Encode',
            desc: 'Each page is re-encoded — AVIF, WebP, or kept as-is — with optional downscaling.',
        },
        {
            icon: IconStack2,
            label: 'Bundle',
            desc: 'Pages are grouped into output volumes, either by detected number or flattened into one.',
        },
        {
            icon: IconPackage,
            label: 'Package',
            desc: 'Everything is wrapped into a CBZ, EPUB, or raw image folder — ready to read.',
        },
    ];
</script>

<div class="mx-auto flex max-w-2xl flex-col gap-14 px-8 py-12">
    <!-- Hero -->
    <div class="flex flex-col gap-4">
        <div class="flex items-baseline gap-3">
            <h1 class="text-4xl font-bold tracking-tight text-anasthasia-text">Thasia</h1>
            <span
                class="rounded-md border border-anasthasia-accent/30 bg-anasthasia-accent/8 px-2 py-0.5 text-xs font-bold tracking-wider text-anasthasia-accent uppercase"
            >
                Manga Processing Engine
            </span>
        </div>
        <p class="text-base leading-relaxed text-anasthasia-muted">
            From <span class="italic">Anastasia</span> — meaning
            <span class="font-medium text-anasthasia-text">"resurrection"</span> or
            <span class="font-medium text-anasthasia-text">"rise again"</span>. A name that fits: Thasia
            is a complete rebuild of something that came before.
        </p>
    </div>

    <!-- Divider -->
    <div
        class="h-px w-full bg-gradient-to-r from-transparent via-anasthasia-accent/30 to-transparent"
    ></div>

    <!-- Origin story -->
    <div class="flex flex-col gap-4">
        <h2 class="text-sm font-bold tracking-wider text-anasthasia-muted uppercase">The Story</h2>
        <div class="flex flex-col gap-3 text-sm leading-relaxed text-anasthasia-muted">
            <p>
                Thasia grew out of
                <button
                    onclick={() => openUrl('https://github.com/lumisxh/palaxy').catch(() => {})}
                    class="inline-flex cursor-pointer items-center gap-1 font-medium text-anasthasia-accent underline-offset-3 transition-opacity duration-150 hover:opacity-70"
                >
                    Palaxy <IconExternalLink size={11} class="relative top-px" />
                </button>
                — an older manga converter that worked but was overly ambitious, accumulated rough code,
                and had grown hard to maintain or extend.
            </p>
            <p>
                Rather than keep patching it, the whole thing was thrown out and rebuilt from
                scratch. The result is faster, cleaner, and designed to grow — including a planned
                download feature that will eventually make tools like
                <span class="font-medium text-anasthasia-text">Hakuneko</span> unnecessary.
            </p>
            <p>Same problem, solved properly the second time.</p>
        </div>
    </div>

    <!-- How it works -->
    <div class="flex flex-col gap-5">
        <h2 class="text-sm font-bold tracking-wider text-anasthasia-muted uppercase">How it works</h2>
        <p class="text-sm text-anasthasia-muted">
            Under the hood, Thasia runs a sequential processing pipeline. Each step hands off to the
            next:
        </p>

        <!-- Pipeline -->
        <div class="flex flex-col gap-2">
            {#each pipeline as step, i (step.label)}
                {@const Icon = step.icon}
                <div class="flex items-start gap-4">
                    <!-- Step -->
                    <div
                        class="flex flex-1 items-start gap-3 rounded-xl border border-anasthasia-border bg-anasthasia-surface p-4 transition-colors duration-150 hover:border-anasthasia-accent/25 hover:bg-anasthasia-panel"
                    >
                        <div
                            class="mt-0.5 flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg border border-anasthasia-accent/20 bg-anasthasia-accent/8"
                        >
                            <Icon size={16} class="text-anasthasia-accent" />
                        </div>
                        <div class="min-w-0">
                            <div class="mb-0.5 text-sm font-bold text-anasthasia-text">
                                {step.label}
                            </div>
                            <div class="text-xs leading-relaxed text-anasthasia-muted">{step.desc}</div>
                        </div>
                    </div>

                    <!-- Connector -->
                    {#if i < pipeline.length - 1}
                        <div
                            class="flex w-4 flex-shrink-0 translate-y-6 items-center justify-center text-anasthasia-border"
                        >
                            <!-- vertical connector on narrow view -->
                        </div>
                    {/if}
                </div>

                {#if i < pipeline.length - 1}
                    <div class="ml-6 flex items-center gap-1 text-anasthasia-border">
                        <div class="ml-3.5 h-4 w-px bg-anasthasia-border"></div>
                    </div>
                {/if}
            {/each}
        </div>
    </div>

    <!-- Divider -->
    <div
        class="h-px w-full bg-gradient-to-r from-transparent via-anasthasia-accent/30 to-transparent"
    ></div>

    <!-- Author -->
    <div class="flex flex-col gap-3">
        <h2 class="text-sm font-bold tracking-wider text-anasthasia-muted uppercase">Made by</h2>
        <button
            onclick={() => openUrl('https://github.com/LuMisxh').catch(() => {})}
            class="group inline-flex w-fit cursor-pointer items-center gap-3 rounded-xl border border-anasthasia-border bg-anasthasia-surface px-4 py-3
                   transition-all duration-150 hover:border-anasthasia-accent/30 hover:bg-anasthasia-panel"
        >
            <div
                class="flex h-8 w-8 items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-panel transition-colors duration-150 group-hover:border-anasthasia-accent/20 group-hover:bg-anasthasia-accent/8"
            >
                <IconBrandGithub
                    size={16}
                    class="text-anasthasia-muted transition-colors duration-150 group-hover:text-anasthasia-accent"
                />
            </div>
            <div>
                <div class="text-sm font-bold text-anasthasia-text">LuMisxh</div>
                <div class="text-xs text-anasthasia-muted">github.com/LuMisxh</div>
            </div>
            <IconExternalLink
                size={13}
                class="ml-1 text-anasthasia-border transition-colors duration-150 group-hover:text-anasthasia-accent"
            />
        </button>
    </div>
</div>
