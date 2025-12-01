<script lang="ts">
    import * as Card from '$lib/components/ui/card';
    import { Badge } from '$lib/components/ui/badge';
    import UptimeIndicator from './uptime-indicator.svelte';
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
            <!-- Overall Metrics -->
            <div>
                <h3 class="mb-3 text-sm font-medium text-muted-foreground">Overall</h3>
                <div class="grid gap-4 md:grid-cols-4">
                    <div>
                        <p class="text-xs text-muted-foreground">Uptime</p>
                        <div class="mt-1">
                            <UptimeIndicator uptime={metrics.uptime_percent} />
                        </div>
                    </div>
                    <div>
                        <p class="text-xs text-muted-foreground">Avg Response</p>
                        <p class="mt-1 text-lg font-semibold">
                            {metrics.avg_response_time_ms.toFixed(0)}ms
                        </p>
                    </div>
                    <div>
                        <p class="text-xs text-muted-foreground">P95 Response</p>
                        <p class="mt-1 text-lg font-semibold">
                            {metrics.p95_response_time_ms.toFixed(0)}ms
                        </p>
                    </div>
                    <div>
                        <p class="text-xs text-muted-foreground">P99 Response</p>
                        <p class="mt-1 text-lg font-semibold">
                            {metrics.p99_response_time_ms.toFixed(0)}ms
                        </p>
                    </div>
                </div>
            </div>

            <!-- Per-Region Metrics -->
            {#if expectedRegions.length > 0}
                <div>
                    <h3 class="mb-3 text-sm font-medium text-muted-foreground">By Region</h3>
                    <div class="space-y-4">
                        {#each expectedRegions as region (region)}
                            {@const regionMetrics = metrics.by_region?.find(
                                (r) => r.region === region
                            )}
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
                                    <div class="grid gap-4 md:grid-cols-4">
                                        <div>
                                            <p class="text-xs text-muted-foreground">Uptime</p>
                                            <div class="mt-1">
                                                <UptimeIndicator
                                                    uptime={regionMetrics.uptime_percent}
                                                />
                                            </div>
                                        </div>
                                        <div>
                                            <p class="text-xs text-muted-foreground">
                                                Avg Response
                                            </p>
                                            <p class="mt-1 text-sm font-semibold">
                                                {regionMetrics.avg_response_time_ms.toFixed(0)}ms
                                            </p>
                                        </div>
                                        <div>
                                            <p class="text-xs text-muted-foreground">
                                                P95 Response
                                            </p>
                                            <p class="mt-1 text-sm font-semibold">
                                                {regionMetrics.p95_response_time_ms.toFixed(0)}ms
                                            </p>
                                        </div>
                                        <div>
                                            <p class="text-xs text-muted-foreground">
                                                P99 Response
                                            </p>
                                            <p class="mt-1 text-sm font-semibold">
                                                {regionMetrics.p99_response_time_ms.toFixed(0)}ms
                                            </p>
                                        </div>
                                    </div>
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
