<template>
  <section>widget</section>
</template>

<script setup lang="ts">
import { reactive } from "vue";

const state = reactive({ ready: true });
</script>

<script>
export { value as runtimeValue } from "@scope/runtime/utils";
</script>
