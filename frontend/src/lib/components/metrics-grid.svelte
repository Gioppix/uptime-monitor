<script lang="ts">
    import UptimeIndicator from './uptime-indicator.svelte';
    import { cn, formatMicrosToMs } from '$lib/utils';
    import type { SingleMetrics } from '$lib/types';

    interface Props {
        metrics: SingleMetrics;
        secondary?: boolean;
    }

    let { metrics, secondary = false }: Props = $props();

    const textSize = secondary ? 'text-sm' : 'text-lg';
</script>

<div class={cn('grid gap-4', secondary ? 'grid-cols-1' : 'grid-cols-4')}>
    <!-- <div class={cn('grid gap-4 md:grid-cols-4', textSize)}> -->
    <div>
        <p class="text-xs text-muted-foreground">Uptime</p>
        <div class="mt-1">
            <UptimeIndicator uptime={metrics.uptime_percent} />
        </div>
    </div>
    <div>
        <p class="text-xs text-muted-foreground">Avg Response</p>
        <p class="mt-1 {textSize} font-semibold">
            {formatMicrosToMs(metrics.avg_response_time_micros)}ms
        </p>
    </div>
    <div>
        <p class="text-xs text-muted-foreground">P95 Response</p>
        <p class="mt-1 {textSize} font-semibold">
            {formatMicrosToMs(metrics.p95_response_time_micros)}ms
        </p>
    </div>
    <div>
        <p class="text-xs text-muted-foreground">P99 Response</p>
        <p class="mt-1 {textSize} font-semibold">
            {formatMicrosToMs(metrics.p99_response_time_micros)}ms
        </p>
    </div>
</div>
