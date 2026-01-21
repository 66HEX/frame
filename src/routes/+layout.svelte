<script>
	let { children } = $props();
	import './layout.css';
</script>

<div
	class="flex flex-col h-screen overflow-hidden bg-black/60 rounded-2xl border-none -mt-2.5 select-none"
>
	<div class="flex-1 relative">
		{@render children()}
	</div>
</div>
