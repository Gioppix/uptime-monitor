<script lang="ts">
    import * as Sidebar from '$lib/components/ui/sidebar/index.js';
    import UserMenu from '$lib/components/user-menu.svelte';
    import { resolve } from '$app/paths';
    import ListChecks from '@lucide/svelte/icons/list-checks';
    import Home from '@lucide/svelte/icons/home';
    import LogIn from '@lucide/svelte/icons/log-in';
    import UserPlus from '@lucide/svelte/icons/user-plus';
    import ActivityIcon from '@lucide/svelte/icons/activity';

    interface Props {
        user?: { username: string; user_id: string } | null;
    }

    let { user }: Props = $props();

    const items = [
        {
            title: 'Home',
            url: resolve('/'),
            icon: Home
        },
        {
            title: 'Checks',
            url: resolve('/app/checks'),
            icon: ListChecks
        }
    ];
</script>

<Sidebar.Root collapsible="icon">
    <Sidebar.Header>
        <Sidebar.Menu>
            <Sidebar.MenuItem>
                <Sidebar.MenuButton size="lg" class="data-[slot=sidebar-menu-button]:p-1.5!">
                    {#snippet child({ props })}
                        <a {...props} href={resolve('/app')}>
                            <div
                                class="flex aspect-square size-5 items-center justify-center rounded-md bg-sidebar-primary text-sidebar-primary-foreground"
                            >
                                <ActivityIcon class="size-3!" />
                            </div>
                            <span class="text-base font-semibold">Pinger</span>
                        </a>
                    {/snippet}
                </Sidebar.MenuButton>
            </Sidebar.MenuItem>
        </Sidebar.Menu>
    </Sidebar.Header>
    <Sidebar.Content>
        <Sidebar.Group>
            <Sidebar.GroupContent>
                <Sidebar.Menu>
                    {#each items as item (item.title)}
                        <Sidebar.MenuItem>
                            <Sidebar.MenuButton>
                                {#snippet child({ props })}
                                    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
                                    <a href={item.url} {...props}>
                                        <item.icon />
                                        <span>{item.title}</span>
                                    </a>
                                {/snippet}
                            </Sidebar.MenuButton>
                        </Sidebar.MenuItem>
                    {/each}
                </Sidebar.Menu>
            </Sidebar.GroupContent>
        </Sidebar.Group>
    </Sidebar.Content>
    <Sidebar.Footer>
        <Sidebar.Menu>
            {#if user}
                <Sidebar.MenuItem>
                    <UserMenu {user} variant="sidebar" />
                </Sidebar.MenuItem>
            {:else}
                <Sidebar.MenuItem>
                    <Sidebar.MenuButton>
                        {#snippet child({ props })}
                            <a href={resolve('/login')} {...props}>
                                <LogIn />
                                <span>Login</span>
                            </a>
                        {/snippet}
                    </Sidebar.MenuButton>
                </Sidebar.MenuItem>
                <Sidebar.MenuItem>
                    <Sidebar.MenuButton>
                        {#snippet child({ props })}
                            <a href={resolve('/signup')} {...props}>
                                <UserPlus />
                                <span>Sign Up</span>
                            </a>
                        {/snippet}
                    </Sidebar.MenuButton>
                </Sidebar.MenuItem>
            {/if}
        </Sidebar.Menu>
    </Sidebar.Footer>
</Sidebar.Root>
