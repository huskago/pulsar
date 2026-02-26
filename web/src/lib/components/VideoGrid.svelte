<script lang="ts">
	import { voice, type ParticipantInfo, type TrackInfo } from '$lib/services/voice.svelte';
	import { onDestroy } from 'svelte';

	interface VideoTile {
		participantName: string;
		participantIdentity: string;
		trackSid: string;
		kind: 'video' | 'screen';
		isLocal: boolean;
	}

	let tiles = $derived.by(() => {
		const result: VideoTile[] = [];
		for (const p of voice.participants) {
			for (const t of p.videoTracks) {
				result.push({
					participantName: p.name,
					participantIdentity: p.identity,
					trackSid: t.sid,
					kind: t.kind,
					isLocal: p.isLocal
				});
			}
		}
		return result;
	});

	let gridCols = $derived(
		tiles.length <= 1 ? 'grid-cols-1' :
		tiles.length <= 4 ? 'grid-cols-2' :
		'grid-cols-3'
	);

	function videoAttach(el: HTMLVideoElement, tile: VideoTile) {
		let retries = 0;
		const maxRetries = 10;

		function tryAttach() {
			voice.attachTrack(tile.trackSid, el);
			if (!el.srcObject && retries < maxRetries) {
				retries++;
				setTimeout(tryAttach, 100);
			}
		}

		tryAttach();

		if (tile.isLocal && tile.kind === 'video') {
			el.style.transform = 'scaleX(-1)';
		}

		return {
			update(newTile: VideoTile) {
				voice.detachTrack(tile.trackSid);
				tile = newTile;
				retries = 0;

				tryAttach();

				if (newTile.isLocal && newTile.kind === 'video') {
					el.style.transform = 'scaleX(-1)';
				} else {
					el.style.transform = '';
				}
			},
			destroy() {
				voice.detachTrack(tile.trackSid);
			}
		};
	}
</script>

{#if tiles.length > 0}
	<div class="grid {gridCols} auto-rows-fr gap-2 p-4">
		{#each tiles as tile (tile.trackSid)}
			<div class="relative aspect-video overflow-hidden rounded-lg bg-zinc-900">
				<!-- svelte-ignore element_invalid_self_closing_tag -->
				<video
					class="absolute inset-0 h-full w-full object-contain bg-black"
					autoplay
					playsinline
					muted={tile.isLocal}
					use:videoAttach={tile}
				/>
				<div class="absolute bottom-2 left-2 flex items-center gap-1.5 rounded bg-black/60 px-2 py-1 text-xs text-white">
					{#if tile.kind === 'screen'}
						<span>🖥️</span>
					{/if}
					{tile.participantName}
					{#if tile.isLocal}
						<span class="text-muted-foreground">(you)</span>
					{/if}
				</div>
			</div>
		{/each}
	</div>
{/if}