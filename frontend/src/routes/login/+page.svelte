<script lang="ts">
	import { api } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

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
				goto(resolve('/'));
			}
		} catch (_) {
			error = 'An error occurred. Please try again.';
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-100">
	<div class="w-full max-w-md rounded bg-white p-8 shadow-md">
		<h1 class="mb-6 text-2xl font-bold">Login</h1>

		{#if error}
			<div class="mb-4 rounded border border-red-400 bg-red-100 px-4 py-3 text-red-700">
				{error}
			</div>
		{/if}

		<form
			onsubmit={(e) => {
				e.preventDefault();
				handleLogin();
			}}
		>
			<div class="mb-4">
				<label for="username" class="mb-2 block text-gray-700">Username</label>
				<input
					id="username"
					type="text"
					bind:value={username}
					class="w-full rounded border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none"
					required
				/>
			</div>

			<div class="mb-6">
				<label for="password" class="mb-2 block text-gray-700">Password</label>
				<input
					id="password"
					type="password"
					bind:value={password}
					class="w-full rounded border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none"
					required
				/>
			</div>

			<button
				type="submit"
				disabled={loading}
				class="w-full rounded bg-blue-500 py-2 text-white hover:bg-blue-600 disabled:bg-gray-400"
			>
				{loading ? 'Logging in...' : 'Login'}
			</button>
		</form>

		<p class="mt-4 text-center text-gray-600">
			Don't have an account? <a href={resolve('/signup')} class="text-blue-500 hover:underline">
				Sign up
			</a>
		</p>
	</div>
</div>
