<script lang="ts">
    import * as Avatar from '$lib/components/ui/avatar/index.js';
    import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
    import { resolve } from '$app/paths';
    import { goto, invalidateAll } from '$app/navigation';
    import { api } from '$lib/api/client';
    import { getUserInitial, getUserColor } from '$lib/utils';
    import LogOut from '@lucide/svelte/icons/log-out';

    interface Props {
        user: { username: string; user_id: string };
        showName?: boolean;
        variant?: 'default' | 'sidebar';
    }

    let { user, showName = true, variant = 'default' }: Props = $props();

    async function handleLogout() {
        const result = await api.POST('/users/logout', {});
        if (!result.error) {
            await invalidateAll();
            await goto(resolve('/login'));
        }
    }
</script>

<DropdownMenu.Root>
    <DropdownMenu.Trigger class={variant === 'sidebar' ? 'w-full' : ''}>
        <button
            type="button"
            class="flex items-center gap-2 rounded-md hover:bg-accent {variant === 'sidebar'
                ? 'h-12 w-full px-3'
                : 'h-9 px-2'}"
        >
            <Avatar.Root class={variant === 'sidebar' ? 'size-8 rounded-lg' : 'size-6 rounded-md'}>
                <Avatar.Fallback
                    class={variant === 'sidebar' ? 'rounded-lg' : 'rounded-md text-xs'}
                    style="background-color: {getUserColor(user.user_id)}; color: white;"
                >
                    {getUserInitial(user.username)}
                </Avatar.Fallback>
            </Avatar.Root>
            {#if showName}
                <span class="text-sm font-medium">{user.username}</span>
            {/if}
        </button>
    </DropdownMenu.Trigger>
    <DropdownMenu.Content align="end">
        <DropdownMenu.Item onclick={handleLogout}>
            <LogOut class="mr-2 size-4" />
            Log out
        </DropdownMenu.Item>
    </DropdownMenu.Content>
</DropdownMenu.Root>
