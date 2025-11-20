<script lang="ts">
    import { Button } from '$lib/components/ui/button/index.js';
    import * as Card from '$lib/components/ui/card/index.js';
    import { Input } from '$lib/components/ui/input/index.js';
    import { Label } from '$lib/components/ui/label/index.js';
    import { api } from '$lib/api/client';
    import { goto, invalidateAll } from '$app/navigation';
    import { resolve } from '$app/paths';
    import UserPlus from '@lucide/svelte/icons/user-plus';
    import AlertCircle from '@lucide/svelte/icons/alert-circle';

    let username = $state('');
    let password = $state('');
    let confirmPassword = $state('');
    let error = $state('');
    let loading = $state(false);

    async function handleSignup() {
        error = '';

        if (password !== confirmPassword) {
            error = 'Passwords do not match';
            return;
        }

        if (password.length < 6) {
            error = 'Password must be at least 6 characters';
            return;
        }

        loading = true;

        try {
            const result = await api.POST('/users/new', {
                body: {
                    username,
                    password
                }
            });

            if (result.error) {
                error = 'Failed to create account. Username may already exist.';
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
        <Card.Title class="text-2xl">Sign Up</Card.Title>
        <Card.Description>Create a new account to get started</Card.Description>
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
                handleSignup();
            }}
        >
            <div class="grid gap-4">
                <div class="grid gap-2">
                    <Label for="username">Username</Label>
                    <Input
                        id="username"
                        type="text"
                        placeholder="Choose a username"
                        bind:value={username}
                        required
                    />
                </div>
                <div class="grid gap-2">
                    <Label for="password">Password</Label>
                    <Input
                        id="password"
                        type="password"
                        placeholder="Create a password"
                        bind:value={password}
                        required
                    />
                </div>
                <div class="grid gap-2">
                    <Label for="confirmPassword">Confirm Password</Label>
                    <Input
                        id="confirmPassword"
                        type="password"
                        placeholder="Confirm your password"
                        bind:value={confirmPassword}
                        required
                    />
                </div>
                <Button type="submit" class="w-full" disabled={loading}>
                    {#if loading}
                        Creating account...
                    {:else}
                        <UserPlus class="mr-2 size-4" />
                        Sign Up
                    {/if}
                </Button>
            </div>
        </form>

        <div class="mt-4 text-center text-sm">
            Already have an account?
            <a href={resolve('/login')} class="underline"> Login </a>
        </div>
    </Card.Content>
</Card.Root>
