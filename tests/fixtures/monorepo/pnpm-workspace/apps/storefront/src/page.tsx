import debounce from "lodash/debounce";
import React from "react";

export default function Page() {
  debounce(() => undefined, 1);
  return React.createElement("main", null, "storefront");
}
