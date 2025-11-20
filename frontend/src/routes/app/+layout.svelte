<script lang="ts">
    import * as Sidebar from '$lib/components/ui/sidebar/index.js';
    import AppSidebar from '$lib/components/app-sidebar.svelte';

    let { children, data } = $props();
</script>

<Sidebar.Provider>
    <AppSidebar user={data.user} />
    <main>
        {@render children()}
    </main>
</Sidebar.Provider>
