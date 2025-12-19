<script lang="ts">
    import * as Card from '$lib/components/ui/card';
    import { Badge } from '$lib/components/ui/badge';
    import MetricsGrid from './metrics-grid.svelte';
    import { REGION_LABELS } from '$lib/constants';
    import type { MetricsResponse, Region } from '$lib/types';

    interface Props {
        metrics: MetricsResponse;
        expectedRegions: Region[];
    }

    let { metrics, expectedRegions }: Props = $props();
</script>

<Card.Root>
    <Card.Header>
        <Card.Title>Uptime Metrics (Last 24 Hours)</Card.Title>
        <Card.Description>Overall and per-region performance statistics</Card.Description>
    </Card.Header>
    <Card.Content>
        <div class="space-y-6">
            <div>
                <h3 class="mb-3 text-sm font-medium text-muted-foreground">Overall</h3>
                <MetricsGrid {metrics} />
            </div>

            {#if expectedRegions.length > 0}
                <div>
                    <h3 class="mb-3 text-sm font-medium text-muted-foreground">By Region</h3>
                    <div class="grid grid-cols-[repeat(auto-fit,minmax(0,1fr))] gap-4">
                        {#each expectedRegions as region (region)}
                            {@const regionMetrics = metrics.by_region[region]}
                            <div class="rounded-lg border p-4">
                                <div class="mb-3 flex items-center gap-2">
                                    <Badge variant="secondary">{REGION_LABELS[region]}</Badge>
                                    {#if !regionMetrics}
                                        <Badge variant="outline" class="text-muted-foreground">
                                            No data
                                        </Badge>
                                    {/if}
                                </div>
                                {#if regionMetrics}
                                    <MetricsGrid metrics={regionMetrics} secondary />
                                {:else}
                                    <p class="text-sm text-muted-foreground">
                                        No metrics data available for this region in the last 24
                                        hours.
                                    </p>
                                {/if}
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}
        </div>
    </Card.Content>
</Card.Root>
