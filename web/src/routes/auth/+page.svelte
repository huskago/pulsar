<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { api } from '$lib/services/api';

	let mode: 'login' | 'register' = $state('login');
	let username = $state('');
	let email = $state('');
	let password = $state('');
	let errorMsg = $state('');

	async function handleSubmit() {
		errorMsg = '';
		try {
			if (mode === 'register') {
				await auth.register(username, email, password);
			} else {
				await auth.login(email, password);
			}

			const pendingInvite = localStorage.getItem('pulsar_pending_invite');
			if (pendingInvite) {
				localStorage.removeItem('pulsar_pending_invite');
				try {
					await api.joinInvite(pendingInvite);
				} catch (e) {
					console.error('Failed to join pending invite', e);
				}
			}

			goto('/channels');
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : 'Something went wrong';
		}
	}
</script>

<div class="flex h-full items-center justify-center bg-zinc-950">
	<Card.Root class="w-full max-w-md p-8">
		<Card.Header class="mb-4 text-center">
			<Card.Title class="text-3xl font-bold tracking-tight">Pulsar</Card.Title>
			<Card.Description>
				{mode === 'login' ? 'Welcome back!' : 'Create your account'}
			</Card.Description>
		</Card.Header>

		<Card.Content>
			<form onsubmit={handleSubmit} class="space-y-4">
				{#if mode === 'register'}
					<div class="grid gap-2">
						<label for="username" class="text-sm font-medium">Username</label>
						<Input
							id="username"
							bind:value={username}
							placeholder="tiago"
							required
							minlength={2}
							maxlength={32}
						/>
					</div>
				{/if}

				<div class="grid gap-2">
					<label for="email" class="text-sm font-medium">Email</label>
					<Input
						id="email"
						type="email"
						bind:value={email}
						placeholder="you@example.com"
						required
					/>
				</div>

				<div class="grid gap-2">
					<label for="password" class="text-sm font-medium">Password</label>
					<Input
						id="password"
						type="password"
						bind:value={password}
						placeholder="••••••••"
						required
						minlength={8}
					/>
				</div>

				{#if errorMsg}
					<p class="text-sm text-red-500">{errorMsg}</p>
				{/if}

				<Button type="submit" class="w-full cursor-pointer" disabled={auth.loading}>
					{auth.loading ? 'Loading...' : mode === 'login' ? 'Login' : 'Create Account'}
				</Button>
			</form>
		</Card.Content>

		<Card.Footer class="justify-center text-sm text-muted-foreground">
			{#if mode === 'login'}
				Don't have an account?
				<button
					class="ml-1 cursor-pointer text-primary underline-offset-4 hover:underline"
					onclick={() => (mode = 'register')}
				>
					Register
				</button>
			{:else}
				Already have an account?
				<button
					class="ml-1 cursor-pointer text-primary underline-offset-4 hover:underline"
					onclick={() => (mode = 'login')}
				>
					Login
				</button>
			{/if}
		</Card.Footer>
	</Card.Root>
</div>
