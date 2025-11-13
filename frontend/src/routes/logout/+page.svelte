<script lang="ts">
	import { api } from '$lib/api/client';
	import { goto, invalidateAll } from '$app/navigation';
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';

	let error = $state('');
	let loading = $state(true);

	onMount(async () => {
		try {
			const result = await api.POST('/users/logout', {});

			if (result.error) {
				error = 'Failed to logout';
				loading = false;
			} else {
				await invalidateAll();
				await goto(resolve('/login'));
			}
		} catch (_) {
			error = 'An error occurred during logout';
			loading = false;
		}
	});
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-100">
	<div class="w-full max-w-md rounded bg-white p-8 text-center shadow-md">
		{#if loading}
			<h1 class="mb-4 text-2xl font-bold">Logging out...</h1>
			<p class="text-gray-600">Please wait</p>
		{:else if error}
			<h1 class="mb-4 text-2xl font-bold text-red-600">Error</h1>
			<p class="mb-4 text-gray-700">{error}</p>
			<a href={resolve('/login')} class="text-blue-500 hover:underline">Go to login</a>
		{/if}
	</div>
</div>
