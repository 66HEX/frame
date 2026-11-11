<script lang="ts">
	import type { PageData } from './$types';

	const { data }: { data: PageData } = $props();
	const Component = $derived(data.component);
	const slug = $derived(data.slug);
</script>

{#key slug}
	<Component />
{/key}
