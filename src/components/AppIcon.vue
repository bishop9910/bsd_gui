<script setup lang="ts">
withDefaults(
  defineProps<{
    name: string;
    size?: number;
    stroke?: number;
  }>(),
  { size: 18, stroke: 1.8 },
);

const paths: Record<string, string> = {
  download:
    "M12 3v11m0 0 4-4m-4 4-4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2",
  list: "M4 6h16M4 12h16M4 18h10",
  settings: "M4 7h8m4 0h4M4 17h4m4 0h8M12 4v6M8 14v6",
  logs: "M4 5h16v14H4zM7.5 9l2.5 2.5L7.5 14M12.5 14H17",
  info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 7.6h.01",
  file: "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5",
  folder:
    "M3 7.5A2.5 2.5 0 0 1 5.5 5h3.2l2 2H18.5A2.5 2.5 0 0 1 21 9.5v8A2.5 2.5 0 0 1 18.5 20h-13A2.5 2.5 0 0 1 3 17.5z",
  stop: "M7.5 7.5l9 9M16.5 7.5l-9 9",
  retry: "M20 12a8 8 0 1 1-2.4-5.7M20 4v4h-4",
  trash: "M4 7h16M10 11v6M14 11v6M6.5 7l.9 12.2h9.2L17.5 7M9.5 7V4.5h5V7",
  alert: "M12 3.2 2.6 19.6h18.8L12 3.2zM12 9v4.6M12 17h.01",
  check: "M4.5 12.5 9.5 17.5 19.5 6.5",
  plus: "M12 5v14M5 12h14",
  paste:
    "M9 4.5h6v2.6H9zM7 4.5h1.2v2.6h7.6V4.5H17a1.8 1.8 0 0 1 1.8 1.8v13.2A1.8 1.8 0 0 1 17 21.3H7a1.8 1.8 0 0 1-1.8-1.8V6.3A1.8 1.8 0 0 1 7 4.5z",
  external: "M14 4h6v6M20 4l-8.5 8.5M18 14v5.2a1.8 1.8 0 0 1-1.8 1.8H5.8A1.8 1.8 0 0 1 4 19.2V8.8A1.8 1.8 0 0 1 5.8 7H11",
  search: "M11 18.2a7.2 7.2 0 1 0 0-14.4 7.2 7.2 0 0 0 0 14.4zM20.4 20.4l-4.2-4.2",
  zap: "M13.2 2.4 4.6 13.6h5.6l-.9 8 8.7-11.2h-5.7z",
  shield: "M12 3 20 6v6.2c0 4.9-3.4 8.2-8 8.8-4.6-.6-8-3.9-8-8.8V6z",
  globe:
    "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM3.5 12h17M12 3c2.4 2.4 3.8 5.6 3.8 9s-1.4 6.6-3.8 9c-2.4-2.4-3.8-5.6-3.8-9S9.6 5.4 12 3z",
  layers: "M12 3 20.5 7.6 12 12.2 3.5 7.6zM3.5 12.4 12 17l8.5-4.6M3.5 16.6 12 21.2l8.5-4.6",
  clock: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5.2l3.2 1.9",
  chevron: "M6.5 9.5 12 15l5.5-5.5",
  copy: "M9.5 9.5h9.2v9.2H9.5zM5.3 15V5.3H15",
  cube: "M12 3 20 7.5v9L12 21l-8-4.5v-9zM4 7.5l8 4.5 8-4.5M12 12v9",
  wifi: "M2.5 8.5a15 15 0 0 1 19 0M6 12a10.5 10.5 0 0 1 12 0M9.2 15.4a5.6 5.6 0 0 1 5.6 0M12 19h.01",
  refresh: "M20 11a8 8 0 0 0-14.4-3.4M4 5v4h4M4 13a8 8 0 0 0 14.4 3.4M20 19v-4h-4",
  play: "M8 5.5 18.5 12 8 18.5z",
  sun: "M12 16.6a4.6 4.6 0 1 0 0-9.2 4.6 4.6 0 0 0 0 9.2zM12 2.6v2M12 19.4v2M4.6 12h-2M21.4 12h-2M6.3 6.3 4.9 4.9M19.1 19.1l-1.4-1.4M17.7 6.3l1.4-1.4M4.9 19.1l1.4-1.4",
  moon: "M20.4 14.2A8.6 8.6 0 0 1 9.8 3.6a8.6 8.6 0 1 0 10.6 10.6z",
};
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="stroke"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    <path :d="paths[name] ?? paths.info" />
  </svg>
</template>
