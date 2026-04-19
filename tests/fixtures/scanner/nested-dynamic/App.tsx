export async function loadFeature(flag: boolean) {
  const loaders = {
    chart: async () => (await import("chart.js/auto")).Chart,
    map: () => import("mapbox-gl"),
  };

  return flag ? loaders.chart() : loaders.map();
}
