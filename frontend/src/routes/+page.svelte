<script lang="ts">
	import { api } from '$lib/api/client';

	const { data } = $props();

	async function handle() {
		const result = await api.GET('/users/info/{user_id}', {
			params: { path: { user_id: '22222222-2222-2222-2222-222222222222' } }
		});
		console.log(result.data);
	}
</script>

<h1>Welcome to SvelteKit {data.user?.username ?? ''}</h1>
<p>Visit <a href="https://svelte.dev/docs/kit">svelte.dev/docs/kit</a> to read the documentation</p>
<button onclick={handle}>Example</button>
