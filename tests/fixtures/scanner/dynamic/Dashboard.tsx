export async function loadDashboard() {
  const maps = await import("mapbox-gl");
  const local = await import("./local");
  const fsModule = await import("node:fs");

  return import.meta.env ? maps : local ?? fsModule;
}
