<script lang="ts">
    import type { Device } from "./types/generated/Device";

    const channels = [1, 2, 3];
    const channelLetters = ["A", "B", "C"];

    let devices: Device[] = [];
    let routeMode: Number = -1;
    let routeChannel: Number = 0;

    async function refresh() {
        const response = await fetch("/api/devices");
        devices = await response.json();
    }

    async function setRouteMode(enabled: boolean, index: Number) {
        if (enabled) {
            routeMode = index;
        } else {
            routeMode = -1;
        }
    }

    refresh();
</script>

<div class="grid h-screen grid-rows-[auto_1fr_auto]">
    <!-- Header -->
    <header class="p-4"><h1>Voxen Intercom - Control Panel</h1></header>
    <!-- Grid Columns -->
    <div class="grid grid-cols-1 md:grid-cols-[auto_1fr]">
        <!-- Left Sidebar. -->
        <aside class="p-4">(sidebar)</aside>

        <!-- Main Content -->
        <div>
            <div class="grid grid-cols-4 gap-4">
                {#each devices as device, index}
                    <article
                        class="card preset-tonal-primary overflow-hidden p-4"
                    >
                        <p><b>{device.id}:</b> {device.name}</p>
                        {#if routeMode == -1}
                            <button
                                class="btn preset-filled-primary-500"
                                onclick={() => setRouteMode(true, index)}
                                >Route
                            </button>
                        {:else if routeMode == index}
                            {#each channels as channel}
                                <button
                                    class="btn preset-outlined-primary-500"
                                    class:preset-filled={routeChannel ==
                                        channel}
                                    onclick={() => (routeChannel = channel)}
                                    >{channelLetters[channel - 1]}
                                </button>
                            {/each}
                            <button
                                class="btn preset-filled-error-500"
                                onclick={() => setRouteMode(false, 0)}>X</button
                            >
                        {:else}
                            <button class="btn preset-outlined-primary-500" class:preset-filled-success-500={false}>Route Here!</button>
                        {/if}
                    </article>
                {/each}
                <nav
                    class="btn-group preset-outlined-surface-200-800 flex-col p-4"
                >
                    <button class="btn preset-filled-primary-500"
                        >Add Desktop Device
                    </button>
                    <button class="btn preset-filled-primary-500"
                        >Add Beltpack Device
                    </button>
                </nav>
            </div>
            <button class="btn preset-filled-primary-500" onclick={refresh}>
                Refresh
            </button>
        </div>
    </div>

    <!-- Footer -->
    <footer class="bg-blue-500 p-4">(footer)</footer>
</div>
