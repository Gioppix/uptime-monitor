<script lang="ts">
	import { api } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

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
		<h1 class="mb-6 text-2xl font-bold">Sign Up</h1>

		{#if error}
			<div class="mb-4 rounded border border-red-400 bg-red-100 px-4 py-3 text-red-700">
				{error}
			</div>
		{/if}

		<form
			onsubmit={(e) => {
				e.preventDefault();
				handleSignup();
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

			<div class="mb-4">
				<label for="password" class="mb-2 block text-gray-700">Password</label>
				<input
					id="password"
					type="password"
					bind:value={password}
					class="w-full rounded border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none"
					required
				/>
			</div>

			<div class="mb-6">
				<label for="confirmPassword" class="mb-2 block text-gray-700">Confirm Password</label>
				<input
					id="confirmPassword"
					type="password"
					bind:value={confirmPassword}
					class="w-full rounded border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none"
					required
				/>
			</div>

			<button
				type="submit"
				disabled={loading}
				class="w-full rounded bg-blue-500 py-2 text-white hover:bg-blue-600 disabled:bg-gray-400"
			>
				{loading ? 'Creating account...' : 'Sign Up'}
			</button>
		</form>

		<p class="mt-4 text-center text-gray-600">
			Already have an account? <a href={resolve('/login')} class="text-blue-500 hover:underline">
				Login
			</a>
		</p>
	</div>
</div>
