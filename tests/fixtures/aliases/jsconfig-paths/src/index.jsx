import env from "env";
import { FiAlertCircle } from "react-icons/fi";

export function boot() {
  return `${env.mode}-${FiAlertCircle.displayName ?? "icon"}`;
}
