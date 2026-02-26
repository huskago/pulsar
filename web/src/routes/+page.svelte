<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => {
		auth.loadFromStorage();
		if (auth.user) {
			goto('/channels');
		} else {
			goto('/auth');
		}
	});
</script>

<div class="flex h-full items-center justify-center">
	<p class="text-muted-foreground">Loading...</p>
</div>