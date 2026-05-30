<script lang="ts">
    import { Button } from 'anasthasia';
    import { IconSearch } from '@tabler/icons-svelte';

    let {
        query,
        disabled = false,
        searching = false,
        onQuery,
        onSearch,
    }: {
        query: string;
        disabled?: boolean;
        searching?: boolean;
        onQuery: (value: string) => void;
        onSearch: () => void;
    } = $props();
</script>

<form
    class="flex gap-2"
    onsubmit={(event) => {
        event.preventDefault();
        onSearch();
    }}
>
    <input
        class="h-10 min-w-0 flex-1 rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 text-sm text-anasthasia-text transition-colors duration-150 outline-none placeholder:text-anasthasia-muted hover:border-anasthasia-accent/40 focus:border-anasthasia-accent/60"
        placeholder="Search series"
        value={query}
        {disabled}
        oninput={(event) => onQuery(event.currentTarget.value)}
    />
    <Button
        variant="primary"
        loading={searching}
        loadingLabel="Searching…"
        disabled={disabled || !query.trim()}
    >
        <IconSearch size={15} /> Search
    </Button>
</form>
