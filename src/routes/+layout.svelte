<script>
    let { children } = $props();
    import "./layout.css";
</script>

<div
    class="flex flex-col h-screen overflow-hidden bg-black/50 rounded-3xl border-none -mt-6"
>
    <div class="flex-1 relative">
        {@render children()}
    </div>
</div>
