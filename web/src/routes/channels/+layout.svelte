<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { messages } from '$lib/stores/messages.svelte';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Separator } from '$lib/components/ui/separator/index.js';

	let { children } = $props();

	onMount(() => {
		if (!auth.user) {
			goto('/auth');
			return;
		}
		messages.init();
	});

	const channels = [
		{ id: '1', name: 'general', kind: 'text' as const },
		{ id: '2', name: 'random', kind: 'text' as const },
		{ id: '3', name: 'dev', kind: 'text' as const }
	];
</script>

<div class="flex h-full">
	<!-- Sidebar -->
	<aside class="flex w-60 flex-col bg-zinc-900">
		<!-- Guild header -->
		<div class="flex h-12 items-center px-4 font-semibold shadow-md">Pulsar</div>
		<Separator />

		<!-- Channel list -->
		<nav class="flex-1 space-y-0.5 overflow-y-auto p-2">
			<p class="px-2 py-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
				Text Channels
			</p>
			{#each channels as channel (channel.id)}
				<a
					href="/channels/{channel.id}"
					class="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100"
				>
					<span class="text-lg text-muted-foreground">#</span>
					{channel.name}
				</a>
			{/each}
		</nav>

		<Separator />

		<!-- User bar -->
		<div class="flex items-center gap-2 p-3">
			<div
				class="flex h-8 w-8 items-center justify-center rounded-full bg-indigo-600 text-sm font-medium"
			>
				{auth.user?.username.charAt(0).toUpperCase()}
			</div>
			<div class="flex-1 truncate text-sm font-medium">
				{auth.user?.username}
			</div>
			<button
				class="text-xs text-muted-foreground hover:text-zinc-100"
				onclick={() => {
					auth.logout();
					goto('/auth');
				}}
			>
				Logout
			</button>
		</div>
	</aside>

	<!-- Main content -->
	<main class="flex flex-1 flex-col bg-zinc-950">
		{@render children()}
	</main>
</div>
