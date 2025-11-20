<script lang="ts">
    import { api } from '$lib/api/client';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Button } from '$lib/components/ui/button';
    import CheckDialog from '$lib/components/check-dialog.svelte';
    import CheckDetailCards from '$lib/components/check-detail-cards.svelte';
    import ArrowLeft from '@lucide/svelte/icons/arrow-left';
    import Pencil from '@lucide/svelte/icons/pencil';
    import Trash2 from '@lucide/svelte/icons/trash-2';
    import type { PageData } from './$types';

    let { data }: { data: PageData } = $props();

    let showEditDialog = $state(false);

    function openEditDialog() {
        showEditDialog = true;
    }

    async function handleDelete() {
        if (!confirm('Are you sure you want to delete this check?')) return;

        await api.DELETE('/checks/{check_id}', {
            params: { path: { check_id: data.check.check_id } }
        });

        await goto(resolve('/app/checks'));
    }
</script>

<div class="flex flex-col gap-6 p-6">
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
            <Button variant="ghost" size="icon" onclick={() => goto(resolve('/app/checks'))}>
                <ArrowLeft class="h-4 w-4" />
            </Button>
            <div>
                <h1 class="text-3xl font-bold">{data.check.check_name}</h1>
                <p class="text-muted-foreground">Check details and configuration</p>
            </div>
        </div>
        {#if data.check.can_edit}
            <div class="flex gap-2">
                <Button variant="outline" onclick={openEditDialog}>
                    <Pencil class="mr-2 h-4 w-4" />
                    Edit
                </Button>
                <Button variant="destructive" onclick={handleDelete}>
                    <Trash2 class="mr-2 h-4 w-4" />
                    Delete
                </Button>
            </div>
        {/if}
    </div>

    <CheckDetailCards check={data.check} />
</div>

<CheckDialog bind:open={showEditDialog} check={data.check} />
