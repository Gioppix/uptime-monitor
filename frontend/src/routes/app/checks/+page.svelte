<script lang="ts">
    import { Button } from '$lib/components/ui/button';
    import CheckDialog from '$lib/components/check-dialog.svelte';
    import ChecksTable from '$lib/components/checks-table.svelte';
    import LoginRequiredAlert from '$lib/components/login-required-alert.svelte';
    import Plus from '@lucide/svelte/icons/plus';
    import type { components } from '$lib/api/schema';
    import type { PageData } from './$types';

    type Check = components['schemas']['CheckWithAccess'];

    let { data }: { data: PageData } = $props();

    let showDialog = $state(false);
    let editingCheck = $state<Check | null>(null);

    function openCreateDialog() {
        editingCheck = null;
        showDialog = true;
    }

    function openEditDialog(check: Check) {
        editingCheck = check;
        showDialog = true;
    }
</script>

<div class="flex flex-col gap-6 p-6">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-3xl font-bold">Checks</h1>
            <p class="text-muted-foreground">Manage your uptime monitoring checks</p>
        </div>
        {#if data.user}
            <Button onclick={openCreateDialog}>
                <Plus class="mr-2 h-4 w-4" />
                New Check
            </Button>
        {/if}
    </div>

    {#if !data.user}
        <LoginRequiredAlert />
    {:else}
        <ChecksTable checks={data.checks} onEdit={openEditDialog} />
    {/if}
</div>

{#if data.user}
    <CheckDialog bind:open={showDialog} check={editingCheck} />
{/if}
