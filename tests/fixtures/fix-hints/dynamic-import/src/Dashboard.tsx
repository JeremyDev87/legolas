export async function loadDashboardChart() {
  const chart = await import("chart.js");

  return chart.Chart;
}
