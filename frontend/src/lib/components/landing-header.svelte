<script lang="ts">
    import { Button } from '$lib/components/ui/button/index.js';
    import UserMenu from '$lib/components/user-menu.svelte';
    import { resolve } from '$app/paths';
    import ActivityIcon from '@lucide/svelte/icons/activity';

    interface Props {
        user?: { username: string; user_id: string } | null;
        showBackButton?: boolean;
    }

    let { user, showBackButton = false }: Props = $props();
</script>

<header
    class="sticky top-0 z-50 flex h-16 items-center justify-between border-b bg-background px-6"
>
    <a href={resolve('/')} class="flex items-center gap-2">
        <div
            class="flex size-8 items-center justify-center rounded-md bg-primary text-primary-foreground"
        >
            <ActivityIcon class="size-5" />
        </div>
        <span class="text-xl font-semibold">Pinger</span>
    </a>

    <nav class="flex items-center gap-2">
        {#if showBackButton}
            <Button href={resolve('/')} variant="ghost">Back to Home</Button>
        {/if}
        {#if user}
            <Button href={resolve('/app')} variant="ghost">Dashboard</Button>
            <UserMenu {user} />
        {:else}
            <Button href={resolve('/login')} variant="ghost">Log in</Button>
            <Button href={resolve('/signup')}>Sign up</Button>
        {/if}
    </nav>
</header>
