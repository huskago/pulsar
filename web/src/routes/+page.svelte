<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	let status = $state<'loading' | 'redirecting'>('loading');

	onMount(async () => {
		auth.loadFromStorage();

		if (!auth.token) {
			status = 'redirecting';
			goto('/auth');
			return;
		}

		const valid = await auth.validateAndConnect();
		status = 'redirecting';

		if (valid) {
			goto('/channels');
		} else {
			goto('/auth');
		}
	});
</script>

<div class="flex h-full flex-col items-center justify-center gap-3">
	{#if status === 'loading'}
		<div class="h-8 w-8 animate-spin rounded-full border-2 border-zinc-600 border-t-indigo-500"></div>
		<p class="text-sm text-muted-foreground">Connecting to Pulsar...</p>
	{:else}
		<p class="text-sm text-muted-foreground">Redirecting...</p>
	{/if}
</div>