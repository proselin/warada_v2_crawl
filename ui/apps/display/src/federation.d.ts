declare module "activity/Activity" {
  import type { ComponentType } from "react";

  const Activity: ComponentType;
  export default Activity;
}

declare module "ui-kit/status-class" {
  export function statusClass(status: unknown): string;
}
