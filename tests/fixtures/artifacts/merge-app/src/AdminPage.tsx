import { Chart } from "chart.js";
import React from "react";
import { FiSettings } from "react-icons/fi";

export function AdminPage() {
  return (
    <div
      data-chart={typeof Chart}
      data-icon={typeof FiSettings}
      data-react={typeof React}
    >
      Admin
    </div>
  );
}
