<script lang="ts">
    import { Button } from '$lib/components/ui/button';
    import * as Card from '$lib/components/ui/card';
    import { Badge } from '$lib/components/ui/badge';
    import Plus from '@lucide/svelte/icons/plus';
    import Activity from '@lucide/svelte/icons/activity';
    import Clock from '@lucide/svelte/icons/clock';
    import Globe from '@lucide/svelte/icons/globe';
    import UptimeIndicator from './uptime-indicator.svelte';
    import { REGION_LABELS } from '$lib/constants';
    import { resolve } from '$app/paths';
    import type { CheckWithMetrics } from '$lib/types';

    interface Props {
        checks: CheckWithMetrics[];
    }

    let { checks }: Props = $props();

    const stats = $derived({
        total: checks.length,
        enabled: checks.filter((c) => c.is_enabled).length,
        disabled: checks.filter((c) => !c.is_enabled).length,
        regions: new Set(checks.flatMap((c) => c.regions)).size
    });
</script>

<div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
    <Card.Root>
        <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
            <Card.Title class="text-sm font-medium">Total Checks</Card.Title>
            <Activity class="h-4 w-4 text-muted-foreground" />
        </Card.Header>
        <Card.Content>
            <div class="text-2xl font-bold">{stats.total}</div>
            <p class="text-xs text-muted-foreground">
                {stats.enabled} enabled, {stats.disabled} disabled
            </p>
        </Card.Content>
    </Card.Root>

    <Card.Root>
        <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
            <Card.Title class="text-sm font-medium">Active Checks</Card.Title>
            <Clock class="h-4 w-4 text-muted-foreground" />
        </Card.Header>
        <Card.Content>
            <div class="text-2xl font-bold">{stats.enabled}</div>
            <p class="text-xs text-muted-foreground">Currently monitoring</p>
        </Card.Content>
    </Card.Root>

    <Card.Root>
        <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
            <Card.Title class="text-sm font-medium">Disabled Checks</Card.Title>
            <Clock class="h-4 w-4 text-muted-foreground" />
        </Card.Header>
        <Card.Content>
            <div class="text-2xl font-bold">{stats.disabled}</div>
            <p class="text-xs text-muted-foreground">Not monitoring</p>
        </Card.Content>
    </Card.Root>

    <Card.Root>
        <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
            <Card.Title class="text-sm font-medium">Regions</Card.Title>
            <Globe class="h-4 w-4 text-muted-foreground" />
        </Card.Header>
        <Card.Content>
            <div class="text-2xl font-bold">{stats.regions}</div>
            <p class="text-xs text-muted-foreground">Active regions</p>
        </Card.Content>
    </Card.Root>
</div>

<Card.Root>
    <Card.Header>
        <Card.Title>Recent Checks</Card.Title>
        <Card.Description>Your most recently created checks</Card.Description>
    </Card.Header>
    <Card.Content>
        {#if checks.length === 0}
            <div class="flex flex-col items-center justify-center py-12 text-center">
                <Activity class="mb-4 h-12 w-12 text-muted-foreground" />
                <h3 class="mb-2 text-lg font-semibold">No checks yet</h3>
                <p class="mb-4 text-sm text-muted-foreground">
                    Get started by creating your first uptime monitoring check
                </p>
                <Button href="/app/checks">
                    <Plus class="mr-2 h-4 w-4" />
                    Create Check
                </Button>
            </div>
        {:else}
            <div class="space-y-4">
                {#each checks.slice(0, 5) as check (check.check_id)}
                    <div class="flex items-center justify-between border-b pb-4 last:border-0">
                        <div class="flex-1">
                            <div class="flex items-center gap-3">
                                {#if check.metrics}
                                    <UptimeIndicator uptime={check.metrics.uptime_percent} />
                                {/if}
                                <a
                                    href={resolve('/app/checks/[id]', { id: check.check_id })}
                                    class="font-medium hover:underline"
                                >
                                    {check.check_name}
                                </a>
                                {#if check.is_enabled}
                                    <Badge variant="default" class="text-xs">Enabled</Badge>
                                {:else}
                                    <Badge variant="secondary" class="text-xs">Disabled</Badge>
                                {/if}
                            </div>
                            <div class="mt-1 flex items-center gap-4 text-sm text-muted-foreground">
                                <span class="max-w-md truncate">{check.url}</span>
                                <span>•</span>
                                <Badge variant="outline" class="text-xs">
                                    {check.http_method}
                                </Badge>
                                <span>•</span>
                                <span>{check.check_frequency_seconds}s interval</span>
                            </div>
                            <div class="mt-2 flex flex-wrap gap-1">
                                {#each check.regions as region (region)}
                                    <Badge variant="secondary" class="text-xs">
                                        {REGION_LABELS[region]}
                                    </Badge>
                                {/each}
                            </div>
                        </div>
                        <Button variant="ghost" size="sm" href="/app/checks/{check.check_id}">
                            View
                        </Button>
                    </div>
                {/each}
            </div>
            {#if checks.length > 5}
                <div class="mt-4 text-center">
                    <Button variant="outline" href="/app/checks">View All Checks</Button>
                </div>
            {/if}
        {/if}
    </Card.Content>
</Card.Root>
