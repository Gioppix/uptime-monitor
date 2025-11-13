<script lang="ts">
	import { api } from '$lib/api/client';

	async function handle() {
		const result = await api.GET('/health', {});
		console.log(result.data);
	}
</script>

<h1>Welcome to SvelteKit</h1>
<p>Visit <a href="https://svelte.dev/docs/kit">svelte.dev/docs/kit</a> to read the documentation</p>
<button onclick={handle}>Example</button>
