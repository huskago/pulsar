<script lang="ts">
	import { page } from '$app/state';
	import { messages } from '$lib/stores/messages.svelte';
	import { gateway } from '$lib/services/gateway.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { tick } from 'svelte';

	let channelId = $derived(page.params.id ?? '');
	let channelMessages = $derived(messages.getMessages(channelId));
	let typingUsers = $derived(messages.getTypingUsers(channelId));

	let inputValue = $state('');
	let messagesContainer: HTMLDivElement | undefined = $state();

	$effect(() => {
		void channelMessages.length;
		tick().then(() => {
			if (messagesContainer) {
				messagesContainer.scrollTop = messagesContainer.scrollHeight;
			}
		});
	});

	function handleSend() {
		const content = inputValue.trim();
		if (!content) return;

		messages.sendMessage(channelId, content);
		inputValue = '';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSend();
		}
	}

	function formatTime(timestamp: number): string {
		return new Date(timestamp).toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<!-- Channel header -->
<div class="flex h-12 items-center gap-2 px-4 shadow-md">
	<span class="text-lg text-muted-foreground">#</span>
	<span class="font-semibold">Channel {channelId}</span>
	{#if gateway.connected}
		<span class="ml-auto text-xs text-green-500">● Connected</span>
	{:else if gateway.reconnecting}
		<span class="ml-auto text-xs text-yellow-500">● Reconnecting...</span>
	{:else}
		<span class="ml-auto text-xs text-red-500">● Disconnected</span>
	{/if}
</div>

<Separator />

<!-- Messages area -->
<div bind:this={messagesContainer} class="flex-1 space-y-1 overflow-y-auto p-4">
	{#if channelMessages.length === 0}
		<div class="flex h-full items-center justify-center text-muted-foreground">
			<p>No messages yet. Say something!</p>
		</div>
	{:else}
		{#each channelMessages as msg (msg.id)}
			<div class="group flex gap-3 rounded px-2 py-1 hover:bg-zinc-900/50">
				<!-- Avatar -->
				<div
					class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-indigo-600 text-xs font-medium"
				>
					{String(msg.author_id).slice(-2)}
				</div>
				<!-- Content -->
				<div class="min-w-0 flex-1">
					<div class="flex items-baseline gap-2">
						<span class="text-sm font-semibold text-zinc-200">
							User {String(msg.author_id).slice(-4)}
						</span>
						<span class="text-xs text-muted-foreground">
							{formatTime(msg.timestamp)}
						</span>
					</div>
					<p class="text-sm wrap-break-word text-zinc-300">{msg.content}</p>
				</div>
			</div>
		{/each}
	{/if}
</div>

<!-- Typing indicator -->
{#if typingUsers.length > 0}
	<div class="px-4 py-1 text-xs text-muted-foreground">
		{typingUsers.length === 1
			? `User ${typingUsers[0].slice(-4)} is typing...`
			: `${typingUsers.length} people are typing...`}
	</div>
{/if}

<!-- Input area -->
<div class="p-4 pt-0">
	<div class="flex gap-2">
		<Input
			bind:value={inputValue}
			placeholder="Message #Channel {channelId}"
			onkeydown={handleKeydown}
			class="flex-1 border-zinc-700 bg-zinc-900"
		/>
		<Button onclick={handleSend} disabled={!inputValue.trim()}>Send</Button>
	</div>
</div>
