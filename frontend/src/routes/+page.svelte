<script lang="ts">
    import { Button } from '$lib/components/ui/button/index.js';
    import * as Card from '$lib/components/ui/card/index.js';
    import LandingHeader from '$lib/components/landing-header.svelte';
    import { resolve } from '$app/paths';
    import { env } from '$env/dynamic/public';
    import Globe from '@lucide/svelte/icons/globe';
    import Bell from '@lucide/svelte/icons/bell';
    import BarChart3 from '@lucide/svelte/icons/bar-chart-3';
    import Zap from '@lucide/svelte/icons/zap';
    import Check from '@lucide/svelte/icons/check';

    const { data } = $props();

    const features = [
        {
            id: 'distributed',
            icon: Globe,
            title: 'Distributed Checks',
            description:
                'Monitor from multiple regions worldwide to detect regional outages and measure latency from different locations.'
        },
        {
            id: 'alerts',
            icon: Bell,
            title: 'Instant Alerts',
            description:
                'Get notified immediately via email, Slack, or webhooks when your services go down or respond slowly.'
        },
        {
            id: 'analytics',
            icon: BarChart3,
            title: 'Detailed Analytics',
            description:
                'Track response times, uptime percentages, and historical data with beautiful charts and exportable reports.'
        },
        {
            id: 'fast',
            icon: Zap,
            title: 'Sub-minute Checks',
            description:
                'Check your endpoints as frequently as every 5 seconds to catch issues before your users notice.'
        }
    ];

    let selectedFeature = $state(features[0]);

    const plans = [
        {
            name: 'Hobby',
            price: '$0',
            period: 'forever',
            description: 'For side projects',
            features: ['5 monitors', '5-minute checks', 'Email alerts', '24h data retention'],
            cta: 'Get Started',
            highlighted: false
        },
        {
            name: 'Pro',
            price: '$19',
            period: '/month',
            description: 'For growing teams',
            features: [
                '50 monitors',
                '1-minute checks',
                'Slack & webhooks',
                '90-day retention',
                'Status pages'
            ],
            cta: 'Start Trial',
            highlighted: true
        },
        {
            name: 'Enterprise',
            price: '$1M',
            period: '/month',
            description: 'For large organizations',
            features: [
                'Unlimited monitors',
                '30-second checks',
                'Priority support',
                '1-year retention',
                'SSO & audit logs'
            ],
            cta: 'Contact Sales',
            highlighted: false
        }
    ];
</script>

<div class="min-h-screen">
    <LandingHeader user={data.user} />

    <main>
        <!-- Hero -->
        <section class="flex min-h-[70vh] items-center justify-center px-6">
            <div class="mx-auto max-w-3xl text-center">
                <h1 class="text-5xl font-bold tracking-tight md:text-6xl">
                    Uptime monitoring<br />made simple
                </h1>
                <p class="mx-auto mt-8 max-w-lg text-lg text-muted-foreground">
                    Monitor your websites and APIs with distributed checks. Get notified instantly
                    when something goes down.
                </p>
                <div class="mt-10">
                    {#if data.user}
                        <Button href={resolve('/app')} size="lg">Dashboard</Button>
                    {:else}
                        <Button href={resolve('/signup')} size="lg">Get Started</Button>
                    {/if}
                </div>
            </div>
        </section>

        <!-- Features -->
        <section class="border-t px-6 py-24">
            <div class="mx-auto max-w-4xl">
                <h2 class="text-center text-2xl font-bold">Features</h2>
                <div class="mt-12 grid gap-8 md:grid-cols-2">
                    <div class="space-y-2">
                        {#each features as feature (feature.id)}
                            <Button
                                variant={selectedFeature.id === feature.id ? 'outline' : 'ghost'}
                                class="w-full justify-start"
                                onclick={() => (selectedFeature = feature)}
                            >
                                <feature.icon class="size-4" />
                                {feature.title}
                            </Button>
                        {/each}
                    </div>
                    <div class="rounded-lg border bg-background p-6">
                        <div class="flex items-center gap-3">
                            <selectedFeature.icon class="size-6" />
                            <h3 class="text-lg font-semibold">{selectedFeature.title}</h3>
                        </div>
                        <p class="mt-4 text-muted-foreground">{selectedFeature.description}</p>
                    </div>
                </div>
            </div>
        </section>

        <!-- Regions -->
        <section class="border-t bg-muted/30 px-6 py-24">
            <div class="mx-auto max-w-2xl text-center">
                <h2 class="text-2xl font-bold">Global coverage, local insights</h2>
                <p class="mt-4 text-muted-foreground">
                    Your users are everywhere. Our monitoring infrastructure runs across 6 regions,
                    checking your endpoints from the same locations as your actual users. Catch
                    regional outages before they become global problems.
                </p>
                <div class="mt-8 flex flex-wrap justify-center gap-3">
                    {#each ['Falkenstein', 'Helsinki', 'Nuremberg', 'Ormelle', 'Moon'] as region (region)}
                        <span class="rounded-full border bg-background px-4 py-1.5 text-sm"
                            >{region}</span
                        >
                    {/each}
                </div>
            </div>
        </section>

        <!-- Pricing -->
        <section class="border-t px-6 py-24">
            <div class="mx-auto max-w-5xl">
                <h2 class="text-center text-2xl font-bold">Pricing</h2>
                <p class="mt-2 text-center text-muted-foreground">
                    Simple pricing for teams of all sizes
                </p>
                <div class="mt-12 grid gap-6 md:grid-cols-3">
                    {#each plans as plan (plan.name)}
                        <Card.Root
                            class="flex flex-col {plan.highlighted
                                ? 'border-primary ring-1 ring-primary'
                                : ''}"
                        >
                            <Card.Header>
                                <Card.Title>{plan.name}</Card.Title>
                                <Card.Description>{plan.description}</Card.Description>
                            </Card.Header>
                            <Card.Content class="flex-1">
                                <div class="mb-6">
                                    <span class="text-3xl font-bold">{plan.price}</span>
                                    <span class="text-muted-foreground">{plan.period}</span>
                                </div>
                                <ul class="space-y-2">
                                    {#each plan.features as feature (feature)}
                                        <li class="flex items-center gap-2 text-sm">
                                            <Check class="size-4 text-green-500" />
                                            {feature}
                                        </li>
                                    {/each}
                                </ul>
                            </Card.Content>
                            <Card.Footer>
                                <Button
                                    href={resolve('/signup')}
                                    class="w-full"
                                    variant={plan.highlighted ? 'default' : 'outline'}
                                >
                                    {plan.cta}
                                </Button>
                            </Card.Footer>
                        </Card.Root>
                    {/each}
                </div>
            </div>
        </section>
    </main>

    <footer class="border-t bg-muted/30 px-6 py-12">
        <div class="mx-auto grid max-w-5xl gap-8 md:grid-cols-4">
            <div>
                <h4 class="font-semibold">Product</h4>
                <ul class="mt-4 space-y-2 text-sm text-muted-foreground">
                    <li><span class="hover:underline">Features</span></li>
                    <li><span class="hover:underline">Pricing</span></li>
                    <li><span class="hover:underline">Integrations</span></li>
                    <li><span class="hover:underline">Changelog</span></li>
                </ul>
            </div>
            <div>
                <h4 class="font-semibold">Resources</h4>
                <ul class="mt-4 space-y-2 text-sm text-muted-foreground">
                    <li><span class="hover:underline">Documentation</span></li>
                    <li>
                        <a
                            href="{env.PUBLIC_BACKEND_URL}/swagger-ui/"
                            target="_blank"
                            class="underline hover:no-underline">API Reference</a
                        >
                    </li>
                    <li><span class="hover:underline">Status Page</span></li>
                    <li><span class="hover:underline">Blog</span></li>
                </ul>
            </div>
            <div>
                <h4 class="font-semibold">Company</h4>
                <ul class="mt-4 space-y-2 text-sm text-muted-foreground">
                    <li><span class="hover:underline">About</span></li>
                    <li><span class="hover:underline">Careers</span></li>
                    <li><span class="hover:underline">Contact</span></li>
                    <li><span class="hover:underline">Press Kit</span></li>
                </ul>
            </div>
            <div>
                <h4 class="font-semibold">Legal</h4>
                <ul class="mt-4 space-y-2 text-sm text-muted-foreground">
                    <li><span class="hover:underline">Privacy Policy</span></li>
                    <li><span class="hover:underline">Terms of Service</span></li>
                    <li><span class="hover:underline">Cookie Policy</span></li>
                    <li>
                        <a href={resolve('/gdpr')} class="underline hover:no-underline">GDPR</a>
                    </li>
                </ul>
            </div>
        </div>
        <div class="mx-auto mt-8 max-w-5xl border-t pt-8 text-center text-sm text-muted-foreground">
            © 2025 Pinger. All rights reserved. Made with questionable decisions.
        </div>
    </footer>
</div>
