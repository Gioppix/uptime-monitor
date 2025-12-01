<script lang="ts">
    import { api } from '$lib/api/client';
    import { invalidateAll } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Button } from '$lib/components/ui/button';
    import * as Table from '$lib/components/ui/table';
    import { Badge } from '$lib/components/ui/badge';
    import UptimeIndicator from './uptime-indicator.svelte';
    import Pencil from '@lucide/svelte/icons/pencil';
    import Trash2 from '@lucide/svelte/icons/trash-2';
    import { REGION_LABELS } from '$lib/constants';
    import type { CheckWithMetrics } from '$lib/types';

    interface Props {
        checks: CheckWithMetrics[];
        onEdit?: (check: CheckWithMetrics) => void;
    }

    let { checks, onEdit }: Props = $props();

    async function handleDelete(checkId: string) {
        if (!confirm('Are you sure you want to delete this check?')) return;

        await api.DELETE('/checks/{check_id}', {
            params: { path: { check_id: checkId } }
        });

        await invalidateAll();
    }
</script>

<Table.Root>
    <Table.Header>
        <Table.Row>
            <Table.Head>Name</Table.Head>
            <Table.Head>URL</Table.Head>
            <Table.Head>Method</Table.Head>
            <Table.Head>Frequency</Table.Head>
            <Table.Head>Regions</Table.Head>
            <Table.Head>Uptime</Table.Head>
            <Table.Head>Status</Table.Head>
            <Table.Head class="text-right">Actions</Table.Head>
        </Table.Row>
    </Table.Header>
    <Table.Body>
        {#each checks as check (check.check_id)}
            <Table.Row>
                <Table.Cell class="font-medium">
                    <a
                        href={resolve('/app/checks/[id]', { id: check.check_id })}
                        class="hover:underline"
                    >
                        {check.check_name}
                    </a>
                </Table.Cell>
                <Table.Cell class="max-w-xs truncate">{check.url}</Table.Cell>
                <Table.Cell>
                    <Badge variant="outline">{check.http_method}</Badge>
                </Table.Cell>
                <Table.Cell>{check.check_frequency_seconds}s</Table.Cell>
                <Table.Cell>
                    <div class="flex flex-wrap gap-1">
                        {#each check.regions as region (region)}
                            <Badge variant="secondary" class="text-xs">
                                {REGION_LABELS[region]}
                            </Badge>
                        {/each}
                    </div>
                </Table.Cell>
                <Table.Cell>
                    {#if check.metrics}
                        <UptimeIndicator uptime={check.metrics.uptime_percent} />
                    {:else}
                        <span class="text-sm text-muted-foreground">-</span>
                    {/if}
                </Table.Cell>
                <Table.Cell>
                    {#if check.is_enabled}
                        <Badge variant="default">Enabled</Badge>
                    {:else}
                        <Badge variant="secondary">Disabled</Badge>
                    {/if}
                </Table.Cell>
                <Table.Cell class="text-right">
                    <div class="flex justify-end gap-2">
                        {#if check.can_edit}
                            <Button variant="ghost" size="icon" onclick={() => onEdit?.(check)}>
                                <Pencil class="h-4 w-4" />
                            </Button>
                            <Button
                                variant="ghost"
                                size="icon"
                                onclick={() => handleDelete(check.check_id)}
                            >
                                <Trash2 class="h-4 w-4" />
                            </Button>
                        {/if}
                    </div>
                </Table.Cell>
            </Table.Row>
        {:else}
            <Table.Row>
                <Table.Cell colspan={8} class="text-center text-muted-foreground">
                    No checks found. Create your first check to get started.
                </Table.Cell>
            </Table.Row>
        {/each}
    </Table.Body>
</Table.Root>
