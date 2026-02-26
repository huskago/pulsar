<script lang="ts">
	import '../app.css';
	let { children } = $props();
</script>

<div class="h-screen bg-background text-foreground dark">
	{@render children()}
</div>