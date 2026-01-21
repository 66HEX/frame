<script>
	let { children } = $props();
	import './layout.css';
</script>

<div
	class="**:focus:ring- -mt-2.5 flex h-screen flex-col overflow-hidden rounded-2xl border-none bg-black/60 select-none **:focus:outline-none"
>
	<div class="relative flex-1">
		{@render children()}
	</div>
</div>
