<script lang="ts">
    import { Button } from '$lib/components/ui/button';
    import Plus from '@lucide/svelte/icons/plus';
    import LoginRequiredAlert from '$lib/components/login-required-alert.svelte';
    import DashboardContent from '$lib/components/dashboard-content.svelte';
    import type { PageData } from './$types';

    let { data }: { data: PageData } = $props();
</script>

<div class="flex flex-col gap-6 p-6">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-3xl font-bold">Dashboard</h1>
            <p class="text-muted-foreground">Overview of your uptime monitoring checks</p>
        </div>
        {#if data.user}
            <Button href="/app/checks">
                <Plus class="mr-2 h-4 w-4" />
                New Check
            </Button>
        {/if}
    </div>

    {#if !data.user}
        <LoginRequiredAlert />
    {:else if data.checks}
        <DashboardContent checks={data.checks} />
    {/if}
</div>
