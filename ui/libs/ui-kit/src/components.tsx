import { createElement, type ButtonHTMLAttributes, type HTMLAttributes } from "react";
import "./styles.css";

export type ButtonVariant =
  | "default"
  | "destructive"
  | "outline"
  | "secondary"
  | "ghost";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
};

export function Button({
  className = "",
  variant = "default",
  ...props
}: ButtonProps) {
  return createElement("button", {
    ...props,
    "data-slot": "button",
    className: ["ui-button", `ui-button--${variant}`, className].filter(Boolean).join(" "),
  });
}

export type BadgeVariant =
  | "default"
  | "secondary"
  | "destructive"
  | "outline"
  | "queued"
  | "running"
  | "finished"
  | "completed"
  | "failed";

type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  variant?: BadgeVariant;
};

export function Badge({
  className = "",
  variant = "default",
  ...props
}: BadgeProps) {
  return createElement("span", {
    ...props,
    "data-slot": "badge",
    className: ["ui-badge", `ui-badge--${variant}`, className].filter(Boolean).join(" "),
  });
}

type CardProps = HTMLAttributes<HTMLElement> & {
  as?: "article" | "div";
};

export function Card({ as = "div", className = "", ...props }: CardProps) {
  return createElement(as, {
    ...props,
    "data-slot": "card",
    className: ["ui-card", className].filter(Boolean).join(" "),
  });
}
