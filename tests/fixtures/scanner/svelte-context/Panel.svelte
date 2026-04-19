<script context="module">
  export { default as UtcPlugin } from "dayjs/plugin/utc";
</script>

<script>
  const adapter = require("@sveltejs/kit/node");
</script>

<div>{adapter ? "ready" : "pending"}</div>
