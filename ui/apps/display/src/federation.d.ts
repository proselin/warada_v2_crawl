declare module "activity/Activity" {
  import type { ComponentType } from "react";

  const Activity: ComponentType;
  export default Activity;
}

declare module "ui-kit/status-class" {
  export type StatusClass = "queued" | "running" | "finished" | "completed" | "failed" | "";
  export function statusClass(status: unknown): StatusClass;
}

declare module "ui-kit/components" {
  import type { ButtonHTMLAttributes, ComponentType, HTMLAttributes } from "react";

  export const Button: ComponentType<ButtonHTMLAttributes<HTMLButtonElement> & {
    variant?: "default" | "destructive" | "outline" | "secondary" | "ghost";
  }>;
  export const Badge: ComponentType<HTMLAttributes<HTMLSpanElement> & {
    variant?:
      | "default"
      | "secondary"
      | "destructive"
      | "outline"
      | "queued"
      | "running"
      | "finished"
      | "completed"
      | "failed";
  }>;
  export const Card: ComponentType<HTMLAttributes<HTMLElement> & {
    as?: "article" | "div";
  }>;
}
