import type { ReactNode } from "react";
import { type FC, type PropsWithChildren } from "react";
export type { Config } from "@scope/types";
export { type WidgetProps } from "@scope/types";
import { value } from "@scope/runtime/utils";

export const widget = value as unknown as ReactNode | FC<PropsWithChildren>;
