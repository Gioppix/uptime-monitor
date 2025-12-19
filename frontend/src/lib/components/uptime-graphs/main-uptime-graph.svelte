<script lang="ts">
    import * as Chart from '$lib/components/ui/chart/index.js';
    import * as Card from '$lib/components/ui/card/index.js';
    import { scaleUtc, scaleLinear } from 'd3-scale';
    import { Area, AreaChart, ChartClipPath } from 'layerchart';
    import {
        // curveLinear
        curveLinear as chosenCurve
    } from 'd3-shape';
    import { cubicInOut } from 'svelte/easing';
    import type { GraphData, SingleMetrics } from '$lib/types';
    import { formatMicrosToMs } from '$lib/utils';

    interface Props {
        graphData: GraphData | undefined;
    }

    let { graphData }: Props = $props();

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
        }) ?? []
    );

    const responseYDomain = $derived.by(() => {
        if (data.length === 0) return [0, 100];
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

{#if data.length > 0}
    <div class="space-y-4">
        <Card.Root>
            <Card.Header>
                <Card.Title>Response Time</Card.Title>
                <Card.Description>P99 and average response times</Card.Description>
            </Card.Header>
            <Card.Content class="">
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
            </Card.Content>
        </Card.Root>
    </div>
{/if}
