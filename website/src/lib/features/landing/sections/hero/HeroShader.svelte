<script lang="ts">
	import { onMount } from 'svelte';
	import type { HeroController } from './hero-renderer';

	let host: HTMLDivElement;
	let ready = $state(false);
	let controller: HeroController | undefined;

	onMount(() => {
		let disposed = false;
		void import('./hero-renderer')
			.then(({ mountHeroShader }) => {
				if (disposed) return;
				controller = mountHeroShader(host, (isReady) => {
					ready = isReady;
				});
			})
			.catch((error) => {
				if (import.meta.env.DEV) console.warn('Hero shader could not be loaded.', error);
			});
		return () => {
			disposed = true;
			controller?.destroy();
		};
	});
</script>

<div
	bind:this={host}
	data-hero-shader
	data-ready={ready}
	aria-hidden="true"
	class="hero-shader pointer-events-none absolute inset-0 -z-1 overflow-hidden rounded-[inherit]"
></div>

<style>
	.hero-shader {
		--hero-shader-primary: #fc4f4a;
		--hero-shader-secondary: #234a57;
		--hero-shader-accent: #f5a4a9;
		opacity: 0;
	}
	:global(.dark) .hero-shader {
		--hero-shader-primary: #e31616;
		--hero-shader-accent: #fd837c;
	}
	.hero-shader[data-ready='true'] {
		opacity: 1;
	}
	@media (forced-colors: active) {
		.hero-shader {
			display: none;
		}
	}
</style>
