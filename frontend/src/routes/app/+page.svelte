<script lang="ts">
	let { data } = $props();
</script>

{data.user?.username ?? 'Not logged in'}
