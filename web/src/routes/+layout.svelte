<script lang="ts">
	import '../app.css';
	let { children } = $props();
</script>

<div class="dark h-screen bg-background text-foreground">
	{@render children()}
</div>
