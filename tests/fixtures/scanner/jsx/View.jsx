import * as Icons from "lucide-react";

export function View() {
  return (
    <div>
      import should stay text
      <span>require should stay text too</span>
      <strong>{Icons.Activity ? "ok" : "nope"}</strong>
    </div>
  );
}
