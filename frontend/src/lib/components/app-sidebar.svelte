<script lang="ts">
    import * as Sidebar from '$lib/components/ui/sidebar/index.js';
    import * as Avatar from '$lib/components/ui/avatar/index.js';
    import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
    import { resolve } from '$app/paths';
    import { goto, invalidateAll } from '$app/navigation';
    import { api } from '$lib/api/client';
    import ListChecks from '@lucide/svelte/icons/list-checks';
    import LogOut from '@lucide/svelte/icons/log-out';
    import LogIn from '@lucide/svelte/icons/log-in';
    import UserPlus from '@lucide/svelte/icons/user-plus';
    import PanelLeft from '@lucide/svelte/icons/panel-left';
    import ActivityIcon from '@lucide/svelte/icons/activity';
    import EllipsisVertical from '@lucide/svelte/icons/ellipsis-vertical';

    interface Props {
        user?: { username: string; user_id: string } | null;
    }

    let { user }: Props = $props();

    const sidebar = Sidebar.useSidebar();

    const items = [
        {
            title: 'Checks',
            url: resolve('/app/checks'),
            icon: ListChecks
        }
    ];

    function getUserInitial(username: string): string {
        return username.charAt(0).toUpperCase();
    }

    function getUserColor(userId: string): string {
        let hash = 0;
        for (let i = 0; i < userId.length; i++) {
            hash = userId.charCodeAt(i) + ((hash << 5) - hash);
        }

        const hue = Math.abs(hash) % 360;
        return `hsl(${hue}, 70%, 60%)`;
    }

    async function handleLogout() {
        try {
            const result = await api.POST('/users/logout', {});

            if (!result.error) {
                await invalidateAll();
                await goto(resolve('/login'));
            }
        } catch (_) {
            // Handle error silently or show a toast notification
        }
    }
</script>

<Sidebar.Root collapsible="icon">
    <Sidebar.Header>
        <Sidebar.Menu>
            <Sidebar.MenuItem>
                <Sidebar.MenuButton size="lg" class="data-[slot=sidebar-menu-button]:p-1.5!">
                    {#snippet child({ props })}
                        <div {...props}>
                            <div
                                class="flex aspect-square size-5 items-center justify-center rounded-md bg-sidebar-primary text-sidebar-primary-foreground"
                            >
                                <ActivityIcon class="size-3!" />
                            </div>
                            <span class="text-base font-semibold">Pinger</span>
                        </div>
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
                    <DropdownMenu.Root>
                        <DropdownMenu.Trigger>
                            {#snippet child({ props })}
                                <Sidebar.MenuButton
                                    {...props}
                                    size="lg"
                                    class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
                                >
                                    <Avatar.Root class="size-8 rounded-lg">
                                        <Avatar.Fallback
                                            class="rounded-lg"
                                            style="background-color: {getUserColor(
                                                user.user_id
                                            )}; color: white;"
                                            >{getUserInitial(user.username)}</Avatar.Fallback
                                        >
                                    </Avatar.Root>
                                    <div class="grid flex-1 text-left text-sm leading-tight">
                                        <span class="truncate font-medium">{user.username}</span>
                                    </div>
                                    <EllipsisVertical class="ml-auto size-4" />
                                </Sidebar.MenuButton>
                            {/snippet}
                        </DropdownMenu.Trigger>
                        <DropdownMenu.Content
                            class="w-(--bits-dropdown-menu-anchor-width) min-w-56 rounded-lg"
                            side={sidebar.isMobile ? 'bottom' : 'right'}
                            align="end"
                            sideOffset={4}
                        >
                            <DropdownMenu.Item>
                                <button
                                    type="button"
                                    onclick={handleLogout}
                                    class="flex w-full items-center"
                                >
                                    <LogOut class="mr-2 size-4" />
                                    Log out
                                </button>
                            </DropdownMenu.Item>
                        </DropdownMenu.Content>
                    </DropdownMenu.Root>
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
            <!-- <Sidebar.MenuItem>
                <Sidebar.MenuButton onclick={() => sidebar.toggle()}>
                    {#snippet child({ props })}
                        <button type="button" {...props}>
                            <PanelLeft />
                            <span>Collapse</span>
                        </button>
                    {/snippet}
                </Sidebar.MenuButton>
            </Sidebar.MenuItem> -->
        </Sidebar.Menu>
    </Sidebar.Footer>
</Sidebar.Root>
