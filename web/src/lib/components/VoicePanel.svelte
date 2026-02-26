<script lang="ts">
	import { voice } from '$lib/services/voice.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
</script>

{#if voice.connected || voice.connecting}
	<Separator />
	<div class="space-y-2 p-3">
		<!-- Status -->
		<div class="flex items-center justify-between">
			<div>
				<p class="text-xs font-semibold text-green-500">
					{voice.connecting ? 'Connecting...' : 'Voice Connected'}
				</p>
			</div>
			<button
				class="rounded p-1 text-red-400 hover:bg-zinc-800 hover:text-red-300"
				onclick={() => voice.leave()}
				title="Disconnect"
			>
				✕
			</button>
		</div>

		<!-- Participants -->
		<div class="space-y-1">
			{#each voice.participants as participant (participant.identity)}
				<div class="flex items-center gap-2 rounded px-2 py-1 text-sm {participant.isSpeaking ? 'bg-green-500/10' : ''}">
					<div
						class="flex h-6 w-6 items-center justify-center rounded-full text-xs font-medium
						{participant.isSpeaking ? 'bg-green-600 ring-2 ring-green-400' : 'bg-zinc-700'}"
					>
						{participant.name.charAt(0).toUpperCase()}
					</div>
					<span class="flex-1 truncate text-zinc-300">
						{participant.name}
						{#if participant.isLocal}
							<span class="text-xs text-muted-foreground">(you)</span>
						{/if}
					</span>
					{#if participant.isMuted}
						<span class="text-xs text-red-400" title="Muted">🔇</span>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Controls -->
		<div class="flex gap-1">
			<Button
				size="sm"
				variant={voice.isMuted ? 'destructive' : 'outline'}
				class="flex-1"
				onclick={() => voice.toggleMute()}
			>
				{voice.isMuted ? '🔇 Unmute' : '🎤 Mute'}
			</Button>
		</div>
	</div>
{/if}