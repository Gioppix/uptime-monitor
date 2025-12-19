<script lang="ts">
    import * as Select from '$lib/components/ui/select/index.js';
    import * as Chart from '$lib/components/ui/chart/index.js';
    import * as Card from '$lib/components/ui/card/index.js';
    import { Skeleton } from '$lib/components/ui/skeleton';
    import { scaleUtc, scaleLinear } from 'd3-scale';
    import { Area, AreaChart, ChartClipPath } from 'layerchart';
    import {
        // curveLinear
        curveLinear as chosenCurve
    } from 'd3-shape';
    import { cubicInOut } from 'svelte/easing';
    import type { GraphData, GraphGranularity, SingleMetrics } from '$lib/types';
    import { formatMicrosToMs, getGraphTimes } from '$lib/utils';
    import { api } from '$lib/api/client';
    import { TriangleAlert } from '@lucide/svelte';

    const RANGES = {
        '24h': {
            label: '24 hours',
            granularity: 'Hourly' as GraphGranularity,
            days_ago: 1
        },
        '72h': {
            label: '72 hours',
            granularity: 'Hourly' as GraphGranularity,
            days_ago: 3
        },
        '7d': {
            label: '7 days',
            granularity: 'Daily' as GraphGranularity,
            days_ago: 7
        },
        '30d': {
            label: '30 days',
            granularity: 'Daily' as GraphGranularity,
            days_ago: 30
        },
        '90d': {
            label: '90 days',
            granularity: 'Daily' as GraphGranularity,
            days_ago: 90
        }
    } as const;

    type RangeId = keyof typeof RANGES;

    interface Props {
        defaultGraphData: GraphData | undefined;
        checkId: string;
    }

    let { defaultGraphData, checkId }: Props = $props();

    let newData = $state<
        { selectedRange: RangeId } & (
            | { state: 'not_loaded' }
            | { state: 'loading'; currentlyLoading: number }
            | { state: 'error' }
            | { state: 'success'; data: GraphData }
        )
    >({ state: 'not_loaded', selectedRange: '24h' });

    const updateData = async (selectedRange: RangeId) => {
        const currentlyLoading = Math.random();

        newData = { state: 'loading', selectedRange, currentlyLoading };

        const { from, to } = getGraphTimes({
            days: RANGES[selectedRange].days_ago,
            granularity: RANGES[selectedRange].granularity
        });

        const data = await api.GET('/checks/{check_id}/metrics/graph', {
            params: {
                path: { check_id: checkId },
                query: { from, to, granularity: RANGES[selectedRange].granularity }
            },
            fetch
        });

        // Fresh loading taking course
        if (newData.currentlyLoading != currentlyLoading) return;

        if (data.data) {
            newData = { state: 'success', data: data.data, selectedRange };
        } else {
            newData = { state: 'error', selectedRange };
        }
    };

    const graphData = $derived(
        newData.state == 'not_loaded'
            ? defaultGraphData
            : newData.state == 'success'
              ? newData.data
              : undefined
    );

    const data = $derived(
        graphData?.map((g) => {
            const regions = Object.entries(g.by_region);
            const getMax = (fn: (m: SingleMetrics) => number) =>
                regions
                    .map(([r, m]) => ({ region: r, value: fn(m) }))
                    .reduce((max, curr) => (curr.value > max.value ? curr : max));

            const p99 = getMax((m) => m.p99_response_time_micros);
            const avg = getMax((m) => m.avg_response_time_micros);
            const downtime = getMax((m) => 100 - m.uptime_percent);

            return {
                date: new Date(g.date),
                p99: formatMicrosToMs(p99.value),
                avg: formatMicrosToMs(avg.value),
                downtime: downtime.value,
                p99Region: p99.region,
                avgRegion: avg.region,
                downtimeRegion: downtime.region
            };
        })
    );

    const responseYDomain = $derived.by(() => {
        if (!data || data.length === 0) return [0, 100];
        const maxValue = Math.max(...data.map((d) => Math.max(d.p99, d.avg)));
        return [0, maxValue * 1.1]; // Add 10% padding at the top
    });

    const responseConfig = {
        p99: { label: 'P99', color: 'var(--chart-1)' },
        avg: { label: 'Average', color: 'var(--chart-2)' }
    } satisfies Chart.ChartConfig;

    const downtimeConfig = {
        downtime: { label: 'Downtime', color: '#ef4444' }
    } satisfies Chart.ChartConfig;

    const formatDateTime = (v: Date) =>
        v.toLocaleString('en-US', {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
            hour12: false
        });

    const createGradient = (id: string, color: string) => `
        <linearGradient id="${id}" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stop-color="${color}" stop-opacity="0.8"/>
            <stop offset="95%" stop-color="${color}" stop-opacity="0.1"/>
        </linearGradient>
    `;
</script>

<div class="space-y-4">
    <Card.Root>
        <Card.Header>
            <div class="grid flex-1 gap-1 text-center sm:text-start">
                <Card.Title>Response Time</Card.Title>
                <Card.Description>P99 and average response times</Card.Description>
            </div>
            <Select.Root type="single" onValueChange={(v) => updateData(v as RangeId)}>
                <Select.Trigger class="w-40 rounded-lg sm:ms-auto" aria-label="Select a value">
                    {RANGES[newData.selectedRange].label}
                </Select.Trigger>
                <Select.Content class="rounded-xl">
                    {#each Object.entries(RANGES) as [key, range] (key)}
                        <Select.Item value={key} class="rounded-lg">{range.label}</Select.Item>
                    {/each}
                </Select.Content>
            </Select.Root>
        </Card.Header>
        <Card.Content class="">
            {#if data}
                <Chart.Container
                    config={responseConfig}
                    class="flex aspect-auto h-120 w-full flex-col gap-8"
                >
                    <AreaChart
                        {data}
                        x="date"
                        xScale={scaleUtc()}
                        yScale={scaleLinear()}
                        yDomain={responseYDomain}
                        yBaseline={0}
                        series={[
                            { key: 'p99', label: 'P99', color: responseConfig.p99.color },
                            { key: 'avg', label: 'Average', color: responseConfig.avg.color }
                        ]}
                        props={{
                            area: {
                                curve: chosenCurve,
                                'fill-opacity': 0.4,
                                line: { class: 'stroke-1' }
                            },
                            xAxis: {
                                ticks: 0
                            }
                        }}
                    >
                        {#snippet marks({ series, getAreaProps })}
                            <defs>
                                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                                {@html createGradient('fillP99', 'var(--chart-1)')}
                                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                                {@html createGradient('fillAvg', 'var(--chart-2)')}
                            </defs>
                            <ChartClipPath
                                initialWidth={0}
                                motion={{
                                    width: { type: 'tween', duration: 1000, easing: cubicInOut }
                                }}
                            >
                                {#each series as s, i (s.key)}
                                    <Area
                                        {...getAreaProps(s, i)}
                                        fill="url(#fill{s.key === 'p99' ? 'P99' : 'Avg'})"
                                    />
                                {/each}
                            </ChartClipPath>
                        {/snippet}
                        {#snippet tooltip()}
                            <Chart.Tooltip labelFormatter={(value) => formatDateTime(value)} />
                        {/snippet}
                    </AreaChart>
                    <AreaChart
                        {data}
                        x="date"
                        xScale={scaleUtc()}
                        yScale={scaleLinear()}
                        yBaseline={0}
                        series={[
                            {
                                key: 'downtime',
                                label: 'Downtime',
                                color: downtimeConfig.downtime.color
                            }
                        ]}
                        props={{
                            area: {
                                curve: chosenCurve,
                                'fill-opacity': 0.4,
                                line: { class: 'stroke-1' }
                            },
                            xAxis: {
                                ticks: 0
                            }
                        }}
                    >
                        {#snippet marks({ series, getAreaProps })}
                            <defs>
                                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                                {@html createGradient('fillDowntime', '#ef4444')}
                            </defs>
                            <ChartClipPath
                                initialWidth={0}
                                motion={{
                                    width: { type: 'tween', duration: 1000, easing: cubicInOut }
                                }}
                            >
                                <Area {...getAreaProps(series[0], 0)} fill="url(#fillDowntime)" />
                            </ChartClipPath>
                        {/snippet}
                        {#snippet tooltip()}
                            <Chart.Tooltip labelFormatter={(value) => formatDateTime(value)} />
                        {/snippet}
                    </AreaChart>
                </Chart.Container>
            {:else if newData.state === 'loading'}
                <div class="flex flex-col space-y-3 py-8">
                    <Skeleton class="aspect-auto h-120 w-full rounded-xl" />
                </div>
            {:else}
                <div class="flex items-center justify-center py-8">
                    <TriangleAlert class="text-muted-foreground" />
                </div>
            {/if}
        </Card.Content>
    </Card.Root>
</div>
