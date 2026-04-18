<template>
  <div>
    import from "fake-package"
  </div>
</template>

<script setup lang="ts">
import { reactive } from "vue";
import { FaBeer } from "react-icons";

const state = reactive({ loaded: false });
const chartLoader = () => import("chart.js/auto");
</script>

<script>
export { default as ChartHelper } from "chart.js/helpers";
</script>
