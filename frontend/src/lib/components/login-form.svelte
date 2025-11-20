<script lang="ts">
    import { Button } from '$lib/components/ui/button/index.js';
    import * as Card from '$lib/components/ui/card/index.js';
    import { Input } from '$lib/components/ui/input/index.js';
    import { Label } from '$lib/components/ui/label/index.js';
    import { api } from '$lib/api/client';
    import { goto, invalidateAll } from '$app/navigation';
    import { resolve } from '$app/paths';
    import LogIn from '@lucide/svelte/icons/log-in';
    import AlertCircle from '@lucide/svelte/icons/alert-circle';

    let username = $state('');
    let password = $state('');
    let error = $state('');
    let loading = $state(false);

    async function handleLogin() {
        error = '';
        loading = true;

        try {
            const result = await api.POST('/users/login', {
                body: {
                    username,
                    password
                }
            });

            if (result.error) {
                error = 'Invalid username or password';
            } else {
                await invalidateAll();
                await goto(resolve('/'));
            }
        } catch (_) {
            error = 'An error occurred. Please try again.';
        } finally {
            loading = false;
        }
    }
</script>

<Card.Root class="mx-auto w-full max-w-sm">
    <Card.Header>
        <Card.Title class="text-2xl">Login</Card.Title>
        <Card.Description>Enter your credentials to access your account</Card.Description>
    </Card.Header>
    <Card.Content>
        {#if error}
            <div
                class="mb-4 flex items-center gap-2 rounded-lg border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive"
            >
                <AlertCircle class="size-4" />
                <span>{error}</span>
            </div>
        {/if}

        <form
            onsubmit={(e) => {
                e.preventDefault();
                handleLogin();
            }}
        >
            <div class="grid gap-4">
                <div class="grid gap-2">
                    <Label for="username">Username</Label>
                    <Input
                        id="username"
                        type="text"
                        placeholder="Enter your username"
                        bind:value={username}
                        required
                    />
                </div>
                <div class="grid gap-2">
                    <Label for="password">Password</Label>
                    <Input
                        id="password"
                        type="password"
                        placeholder="Enter your password"
                        bind:value={password}
                        required
                    />
                </div>
                <Button type="submit" class="w-full" disabled={loading}>
                    {#if loading}
                        Logging in...
                    {:else}
                        <LogIn class="mr-2 size-4" />
                        Login
                    {/if}
                </Button>
            </div>
        </form>

        <div class="mt-4 text-center text-sm">
            Don't have an account?
            <a href={resolve('/signup')} class="underline"> Sign up </a>
        </div>
    </Card.Content>
</Card.Root>
